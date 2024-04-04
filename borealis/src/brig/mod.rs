//! Rust module generation

use {
    crate::{
        boom::{
            passes::{
                self, cycle_finder::CycleFinder, fold_unconditionals::FoldUnconditionals,
                make_exception_bool::MakeExceptionBool, remove_const_branch::RemoveConstBranch,
                resolve_return_assigns::ResolveReturns,
            },
            Ast,
        },
        brig::{
            allowlist::apply_fn_allowlist, bundle::codegen_bundle,
            functions_interpreter::codegen_functions, state::codegen_state, workspace_diff::write_workspace,
        },
        rudder::{
            self, analysis::cfg::FunctionCallGraphAnalysis, Context, PrimitiveTypeClass, Type,
        },
    },
    common::{create_file, intern::InternedString, HashMap, HashSet},
    log::{info, warn},
    once_cell::sync::Lazy,
    proc_macro2::{Span, TokenStream},
    quote::{format_ident, quote},
    regex::Regex,
    sailrs::jib_ast,
    std::{
        hash::{DefaultHasher, Hash, Hasher},
        io::Write,
        path::Path,
        rc::Rc,
    },
    syn::Ident,
};

mod allowlist;
mod bundle;
mod functions_dbt;
mod functions_interpreter;
mod state;
mod workspace_diff;

const ENTRYPOINT: &str = "__DecodeA64";

/// Compiles a Sail model to a Brig module
pub fn sail_to_brig<I: Iterator<Item = jib_ast::Definition>, P: AsRef<Path>>(
    jib_ast: I,
    path: P,
    _standalone: bool,
) {
    info!("Converting JIB to BOOM");
    let ast = Ast::from_jib(apply_fn_allowlist(jib_ast));

    // useful for debugging
    crate::boom::pretty_print::print_ast(
        &mut create_file("target/ast_raw.boom").unwrap(),
        ast.clone(),
    );

    info!("Running passes on BOOM");
    passes::run_fixed_point(
        ast.clone(),
        &mut [
            FoldUnconditionals::new_boxed(),
            RemoveConstBranch::new_boxed(),
            ResolveReturns::new_boxed(),
            MakeExceptionBool::new_boxed(),
            CycleFinder::new_boxed(),
        ],
    );

    // useful for debugging
    crate::boom::pretty_print::print_ast(
        &mut create_file("target/ast_processed.boom").unwrap(),
        ast.clone(),
    );

    info!("Building rudder");

    let mut rudder = rudder::build::from_boom(&ast.borrow());

    writeln!(&mut create_file("target/ast.rudder").unwrap(), "{rudder}").unwrap();

    info!("Validating rudder");
    let msgs = rudder.validate();
    for msg in msgs {
        warn!("{msg}");
    }

    info!("Optimising rudder");
    rudder.optimise(rudder::opt::OptLevel::Level3);

    writeln!(
        &mut create_file("target/ast.opt.rudder").unwrap(),
        "{rudder}"
    )
    .unwrap();

    info!("Validating rudder again");
    let msgs = rudder.validate();
    for msg in msgs {
        warn!("{msg}");
    }

    info!("Generating Rust");

    write_workspace(&codegen_workspace(&rudder), path);
}

fn promote_width(width: usize) -> usize {
    match width {
        0 => 0,
        1..=8 => 8,
        9..=16 => 16,
        17..=32 => 32,
        33..=64 => 64,
        65..=128 => 128,
        width => {
            warn!("unsupported width: {width}");
            64
        }
    }
}

pub fn codegen_type(typ: Rc<Type>) -> TokenStream {
    match &*typ {
        Type::Primitive(typ) => {
            if typ.type_class() == PrimitiveTypeClass::UnsignedInteger && typ.width() == 1 {
                return quote!(bool);
            }

            let width = promote_width(typ.width());

            let rust_type = match typ.type_class() {
                PrimitiveTypeClass::Void => return quote!(()),
                PrimitiveTypeClass::Unit => return quote!(()),
                PrimitiveTypeClass::UnsignedInteger => {
                    format_ident!("u{}", width)
                }
                PrimitiveTypeClass::SignedInteger => {
                    format_ident!("i{}", width)
                }
                PrimitiveTypeClass::FloatingPoint => {
                    format_ident!("f{}", width)
                }
            };

            quote!(#rust_type)
        }
        Type::Composite(t) => {
            let mut hasher = DefaultHasher::new();
            t.hash(&mut hasher);
            let hashed = format_ident!("CompositeType{:x}", hasher.finish());
            quote! { #hashed }
        }
        Type::Vector {
            element_count,
            element_type,
        } => {
            let element_type = codegen_type(element_type.clone());

            if *element_count == 0 {
                quote!(alloc::vec::Vec<#element_type>)
            } else {
                let count = quote!(#element_count);
                quote!([#element_type; #count])
            }
        }
        Type::Bundled { value, len } => {
            let value_type = codegen_type(value.clone());
            let len_type = codegen_type(len.clone());
            quote!(Bundle<#value_type, #len_type>)
        }
    }
}

fn codegen_member(idx: usize) -> Ident {
    Ident::new(&format!("_{idx}"), Span::call_site())
}

pub fn codegen_ident(input: InternedString) -> Ident {
    static VALIDATOR: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-zA-Z][a-zA-Z0-9_]*$").unwrap());

    let s = input.as_ref();

    if s == "main" {
        return Ident::new("model_main", Span::call_site());
    }

    let mut buf = String::with_capacity(s.len());

    for ch in s.chars() {
        match ch {
            '%' => buf.push_str("_pcnt_"),
            '&' => buf.push_str("_ref_"),
            '?' => buf.push_str("_unknown_"),
            '-' | '<' | '>' | '#' | ' ' | '(' | ')' | ',' | '\'' => buf.push('_'),
            _ => buf.push(ch),
        }
    }

    if buf.starts_with('_') {
        buf = "u".to_owned() + &buf;
    }

    if !VALIDATOR.is_match(&buf) {
        panic!("identifier {buf:?} not normalized even after normalizing");
    }

    Ident::new(&buf, Span::call_site())
}

fn codegen_types(rudder: &Context) -> TokenStream {
    let structs: TokenStream = rudder
        .get_structs()
        .into_iter()
        .map(|typ| {
            let ident = codegen_type(typ.clone());

            let Type::Composite(fields) = &*typ else {
                panic!("struct must be composite type");
            };

            let fields: TokenStream = fields
                .iter()
                .enumerate()
                .map(|(i, typ)| {
                    let name = codegen_member(i);
                    let typ = codegen_type(typ.clone());
                    quote!(pub #name: #typ,)
                })
                .collect();

            quote! {
                #[derive(Default, Debug, Clone, Copy)]
                pub struct #ident {
                    #fields
                }
            }
        })
        .collect();

    let unions: TokenStream = rudder
        .get_unions()
        .into_iter()
        .map(|typ| {
            let ident = codegen_type(typ.clone());

            let Type::Composite(fields) = &*typ else {
                panic!("union must be composite type");
            };

            let variants: TokenStream = fields
                .iter()
                .enumerate()
                .map(|(i, typ)| {
                    let name = codegen_member(i);
                    let typ = codegen_type(typ.clone());
                    quote!(#name(#typ),)
                })
                .collect();

            quote! {
                #[derive(Debug, Clone, Copy)]
                pub enum #ident {
                    #variants
                }

                impl Default for #ident {
                    fn default() -> Self {
                        Self::_0(Default::default())
                    }
                }
            }
        })
        .collect();

    quote! {
        #structs

        #unions
    }
}

fn codegen_workspace(rudder: &Context) -> Workspace {
    // header for each file
    let header = quote! {
        #![no_std]
        #![allow(non_snake_case)]
        #![allow(unused_assignments)]
        #![allow(unused_mut)]
        #![allow(unused_parens)]
        #![allow(unused_variables)]
        #![allow(dead_code)]
        #![allow(unreachable_code)]
        #![allow(unused_doc_comments)]
        #![allow(non_upper_case_globals)]

        //! BOREALIS GENERATED FILE DO NOT MODIFY
    };

    // common crate depended on by all containing bundle, tracer, state, and structs/enums/unions
    let common = {
        let types = codegen_types(rudder);

        let state = codegen_state(rudder);

        let bundle = codegen_bundle();

        (
            "common".into(),
            Crate {
                dependencies: HashSet::default(),
                contents: quote! {
                    #state

                    #bundle

                    #types

                    pub trait Tracer {
                        fn begin(&self, pc: u64);
                        fn end(&self);
                        fn read_register<T: core::fmt::Debug>(&self, offset: usize, value: T);
                        fn write_register<T: core::fmt::Debug>(&self, offset: usize, value: T);
                    }

                    #[derive(Debug)]
                    pub enum ExecuteResult {
                        Ok,
                        EndOfBlock,
                        UndefinedInstruction
                    }
                },
            },
        )
    };

    // one top-level crate containing prelude
    let arch = {
        let entrypoint_ident = codegen_ident(ENTRYPOINT.into());
        (
            "arch".into(),
            Crate {
                dependencies: ["common".into(), entrypoint_ident.to_string().into()]
                    .into_iter()
                    .collect(),
                contents: quote! {
                        pub use common::*;

                        use #entrypoint_ident::#entrypoint_ident;

                        pub fn decode_execute<T: Tracer>(value: u32, state: &mut State, tracer: &mut T) -> ExecuteResult {
                            // reset SEE
                            state.write_register(REG_SEE, 0u64);

                            #entrypoint_ident(state, tracer, Bundle { value: state.read_register(REG_U_PC), length: 64 }, value);

                            // increment PC if no branch was taken
                            if !state.read_register::<bool>(REG_U__BRANCHTAKEN) {
                                let pc = state.read_register::<u64>(REG_U_PC);
                                state.write_register(REG_U_PC, pc + 4);
                            }

                            state.write_register(REG_U__BRANCHTAKEN, false);

                            ExecuteResult::Ok
                        }

                },
            },
        )
    };

    let cfg = FunctionCallGraphAnalysis::new(rudder);

    let functions = codegen_functions(rudder, ENTRYPOINT.into())
        .into_iter()
        .map(|(name, contents)| {
            let mut dependencies = cfg.fns.get(&name).unwrap().clone();
            dependencies.insert("common".into());

            let imports: TokenStream = dependencies
                .iter()
                .map(|krate| {
                    let krate = codegen_ident(*krate);
                    quote!(use #krate::*;)
                })
                .collect();

            let dependencies = dependencies
                .into_iter()
                .map(|name| codegen_ident(name).to_string().into())
                .collect();

            (
                codegen_ident(name).to_string().into(),
                Crate {
                    dependencies,
                    contents: quote! {
                        extern crate alloc;
                        #imports

                        #contents
                    },
                },
            )
        });

    // create workspace, adding header to each crate contents
    Workspace {
        crates: [common, arch]
            .into_iter()
            .chain(functions)
            .map(
                |(
                    name,
                    Crate {
                        dependencies,
                        contents,
                    },
                )| {
                    (
                        name,
                        Crate {
                            dependencies,
                            contents: quote! {
                                #header

                                #contents
                            },
                        },
                    )
                },
            )
            .collect(),
    }
}

struct Workspace {
    crates: HashMap<InternedString, Crate>,
}

struct Crate {
    dependencies: HashSet<InternedString>,
    contents: TokenStream,
}

pub fn tokens_to_string(tokens: &TokenStream) -> String {
    let syntax_tree = syn::parse_file(&tokens.to_string()).unwrap();
    let formatted = prettyplease::unparse(&syntax_tree);
    // fix comments
    formatted.replace("///", "//")
}
