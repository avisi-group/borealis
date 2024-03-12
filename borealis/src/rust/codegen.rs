//! Rust code generation
//!
//! Watch out for `return quote!(...)` in these functions when they build up
//! quotes

use {
    crate::rudder::{Block, Context, Function, PrimitiveTypeClass, Statement, Symbol, Type},
    proc_macro2::{Ident, Literal, Span, TokenStream},
    quote::{format_ident, quote, ToTokens},
    std::{
        borrow::Borrow,
        hash::{DefaultHasher, Hash, Hasher},
        rc::Rc,
    },
};

pub fn codegen(rudder: Context) -> TokenStream {
    rudder.update_names();

    let fns: TokenStream = rudder
        .get_functions()
        .into_iter()
        .map(|(name, function)| {
            let (return_type, parameters) = function.signature();
            let return_type = codegen_type(return_type);
            let parameters = codegen_parameters(parameters);
            let body = codegen_body(function);

            quote! {
                fn #name(#parameters) -> #return_type {
                    #body
                }
            }
        })
        .collect();

    let structs: TokenStream = rudder
        .get_structs()
        .into_iter()
        .map(|typ| {
            let ident = codegen_type(typ.clone());

            let Type::Composite(fields) = typ.borrow() else {
                panic!("struct must be composite type");
            };

            let fields: TokenStream = fields
                .iter()
                .enumerate()
                .map(|(i, typ)| {
                    let name = codegen_member(i);
                    let typ = codegen_type(typ.clone());
                    quote!(#name: #typ,)
                })
                .collect();

            quote! {
                #[derive(Default)]
                struct #ident {
                    #fields
                }
            }
        })
        .collect();

    quote! {
        #structs

        #fns
    }
}

fn codegen_parameters(parameters: Vec<Symbol>) -> TokenStream {
    let parameters = [quote!(state: &mut State)]
        .into_iter()
        .chain(parameters.iter().map(|sym| {
            let name = sym.name();
            let typ = codegen_type(sym.typ());
            quote!(#name: #typ)
        }));

    quote! {
        #(#parameters),*
    }
}

fn promote_width(width: usize) -> usize {
    match width {
        0 => 0,
        1..=8 => 8,
        9..=16 => 16,
        17..=32 => 32,
        33..=64 => 64,
        width => panic!("width = {width}"),
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
            quote! {#hashed}
        }
        Type::Vector {
            element_count,
            element_type,
        } => {
            let element_type = codegen_type(element_type.clone());
            quote!([#element_type; #element_count])
        }
    }
}

fn codegen_body(function: Function) -> TokenStream {
    // local variables
    // block indicies
    // current block

    let block_defs = function
        .entry_block()
        .iter()
        .enumerate()
        .map(|(idx, block)| {
            let block_name = format_ident!("BLOCK_{}", block.name().to_string());
            let idx = u32::try_from(idx).unwrap();

            quote! {
                const #block_name: u32 = #idx;
            }
        })
        .collect::<TokenStream>();

    let local_vars = function
        .local_variables()
        .iter()
        .map(|symbol| {
            let name = format_ident!("{}", symbol.name().to_string());
            let typ = codegen_type(symbol.typ());

            if symbol.typ().is_unit() {
                quote! { let mut #name: () = (); }
            } else {
                quote! {
                    let mut #name: #typ = #typ::default();
                }
            }
        })
        .collect::<TokenStream>();

    let entry_block_name = format_ident!("BLOCK_{}", function.entry_block().name().to_string());

    let block_arms = function
        .entry_block()
        .iter()
        .map(|block| {
            let block_name = format_ident!("BLOCK_{}", block.name().to_string());
            let block_impl = codegen_block(block);

            quote! { #block_name => { #block_impl } }
        })
        .collect::<TokenStream>();

    quote! {
        #block_defs

        #local_vars

        let mut current_block = #entry_block_name;

        loop {
            match current_block {
                #block_arms
                u => panic!("undefined block {u}")
            }
        }
    }
}

fn codegen_block(block: Block) -> TokenStream {
    block
        .statements()
        .iter()
        .cloned()
        .map(codegen_stmt)
        .collect()
}

fn get_ident(stmt: Statement) -> TokenStream {
    format_ident!("{}", stmt.name().to_string()).to_token_stream()
}

fn codegen_stmt(stmt: Statement) -> TokenStream {
    let stmt_name = format_ident!("{}", stmt.name().to_string());

    let value = match stmt.kind() {
        crate::rudder::StatementKind::Constant { value, typ } => {
            match value {
                // todo: do not emit type info here
                crate::rudder::ConstantValue::UnsignedInteger(v) => {
                    if *typ == Type::u1() {
                        let b = v != 0;
                        quote!(#b)
                    } else {
                        let v = Literal::usize_unsuffixed(v);
                        quote!(#v)
                    }
                }
                crate::rudder::ConstantValue::SignedInteger(v) => {
                    let v = Literal::isize_unsuffixed(v);
                    quote!(#v)
                }
                crate::rudder::ConstantValue::FloatingPoint(v) => {
                    let v = Literal::f64_unsuffixed(v);
                    quote!(#v)
                }

                crate::rudder::ConstantValue::Unit => quote!(()),
            }
        }
        crate::rudder::StatementKind::ReadVariable { symbol } => {
            let var = format_ident!("{}", symbol.name().to_string());
            quote! {#var}
        }
        crate::rudder::StatementKind::WriteVariable { symbol, value } => {
            let var = format_ident!("{}", symbol.name().to_string());
            let value = get_ident(value);
            quote! {#var = #value}
        }
        crate::rudder::StatementKind::ReadRegister { .. } => quote!(todo!("read-reg")),
        crate::rudder::StatementKind::WriteRegister { .. } => quote!(todo!("write-reg")),
        crate::rudder::StatementKind::ReadMemory { .. } => quote!(todo!("read-mem")),
        crate::rudder::StatementKind::WriteMemory { .. } => quote!(todo!("write-mem")),
        crate::rudder::StatementKind::ReadPc => quote!(todo!("read-pc")),
        crate::rudder::StatementKind::WritePc { .. } => quote!(todo!("write-pc")),
        crate::rudder::StatementKind::BinaryOperation { kind, lhs, rhs } => {
            let left = get_ident(lhs.clone());
            let right = get_ident(rhs.clone());

            let op = match kind {
                crate::rudder::BinaryOperationKind::CmpEq => quote! { (#left) == (#right) },
                crate::rudder::BinaryOperationKind::Add => quote! { (#left) + (#right) },
                crate::rudder::BinaryOperationKind::Sub => quote! { (#left) - (#right) },
                crate::rudder::BinaryOperationKind::Multiply => quote! { (#left) * (#right) },
                crate::rudder::BinaryOperationKind::Divide => quote! { (#left) / (#right) },
                crate::rudder::BinaryOperationKind::Modulo => quote! { (#left) % (#right) },
                crate::rudder::BinaryOperationKind::And => quote! { (#left) & (#right) },
                crate::rudder::BinaryOperationKind::Or => quote! { (#left) | (#right) },
                crate::rudder::BinaryOperationKind::Xor => quote! { (#left) ^ (#right) },
                crate::rudder::BinaryOperationKind::CmpNe => quote! { (#left) != (#right) },
                crate::rudder::BinaryOperationKind::CmpLt => quote! { (#left) < (#right) },
                crate::rudder::BinaryOperationKind::CmpLe => quote! { (#left) <= (#right) },
                crate::rudder::BinaryOperationKind::CmpGt => quote! { (#left) > (#right) },
                crate::rudder::BinaryOperationKind::CmpGe => quote! { (#left) >= (#right) },
            };

            quote! { (#op) }
        }
        crate::rudder::StatementKind::UnaryOperation { kind, value } => {
            let value = get_ident(value);

            match kind {
                crate::rudder::UnaryOperationKind::Not => quote! {!#value},
                crate::rudder::UnaryOperationKind::Negate => quote! {-#value},
                crate::rudder::UnaryOperationKind::Complement => quote! {!#value},
            }
        }
        crate::rudder::StatementKind::ShiftOperation {
            kind,
            value,
            amount,
        } => {
            let value = get_ident(value);
            let amount = get_ident(amount);

            match kind {
                crate::rudder::ShiftOperationKind::LogicalShiftLeft => quote! {#value << #amount},
                crate::rudder::ShiftOperationKind::LogicalShiftRight => quote! {#value >> #amount},
                crate::rudder::ShiftOperationKind::ArithmeticShiftRight => {
                    quote! {#value >> #amount}
                }
                crate::rudder::ShiftOperationKind::RotateRight => todo!(),
                crate::rudder::ShiftOperationKind::RotateLeft => todo!(),
            }
        }
        crate::rudder::StatementKind::Call { target, args } => {
            let ident = format_ident!("{}", target.name().to_string());
            let args = args.iter().map(|arg| {
                let arg = get_ident(arg.clone());
                quote! {#arg}
            });

            // todo: remove this
            if target.name().as_ref().starts_with("unimplemented_") {
                let msg = target.name().to_string();
                quote! { unimplemented!(#msg) }
            } else {
                quote! {
                    #ident(state, #(#args),*)
                }
            }
        }
        crate::rudder::StatementKind::Cast { typ, value, .. } => {
            let source = value.typ();
            let target = typ;
            let ident = get_ident(value);

            if source == target {
                quote! {
                    ((#ident))
                }
            } else if target == Rc::new(Type::u1()) {
                // todo: is special casing booleans like this necessary?
                // todo: this is duplicated in decode
                quote! {
                    ((#ident) != 0)
                }
            } else {
                let target = codegen_type(target);

                quote! {
                    ((#ident) as #target)
                }
            }
        }
        crate::rudder::StatementKind::Jump { target } => {
            let target = format_ident!("BLOCK_{}", target.name().to_string());
            quote! {
                current_block = #target;
            }
        }
        crate::rudder::StatementKind::Branch {
            condition,
            true_target,
            false_target,
        } => {
            let condition = get_ident(condition);
            let true_target = format_ident!("BLOCK_{}", true_target.name().to_string());
            let false_target = format_ident!("BLOCK_{}", false_target.name().to_string());

            quote! {
                current_block = if #condition { #true_target } else { #false_target };
            }
        }
        crate::rudder::StatementKind::PhiNode { .. } => quote!(todo!("phi")),
        crate::rudder::StatementKind::Return { value } => match value {
            Some(value) => {
                let name = value.name();
                quote! {return #name;}
            }
            None => {
                quote! {return return_value;}
            }
        },
        crate::rudder::StatementKind::Select { .. } => quote!(todo!("select")),
        crate::rudder::StatementKind::BitExtract { .. } => quote!(todo!("bitex")),
        crate::rudder::StatementKind::BitInsert { .. } => quote!(todo!("bitins")),
        crate::rudder::StatementKind::Trap => quote!(panic!("it's a trap")),
        crate::rudder::StatementKind::ReadField { value, field } => {
            let value = get_ident(value);
            let field = format_ident!("_{field}");
            quote!(#value.#field)
        }
        crate::rudder::StatementKind::MutateField {
            composite,
            field,
            value,
        } => {
            let composite = get_ident(composite);
            let field = format_ident!("_{field}");
            let value = get_ident(value);
            quote! {
                {
                    #composite.#field = #value;
                    composite
                }
            }
        }
        crate::rudder::StatementKind::ReadElement { value, index } => {
            let value = get_ident(value);
            let index = get_ident(index);
            quote!(#value[#index])
        }
        crate::rudder::StatementKind::MutateElement {
            vector,
            value,
            index,
        } => {
            let vector = get_ident(vector);
            let index = get_ident(index);
            let value = get_ident(value);
            quote! {
                {
                    #vector[#index] = #value;
                    vector
                }
            }
        }
    };

    let msg = stmt.to_string();
    if stmt.has_value() {
        let typ = codegen_type(stmt.typ());
        quote! {
            let _ = #msg;
            let #stmt_name: #typ = #value;
        }
    } else {
        quote! {
            let _ = #msg;
            #value;
        }
    }
}

fn codegen_member(idx: usize) -> Ident {
    Ident::new(&format!("_{idx}"), Span::call_site())
}
