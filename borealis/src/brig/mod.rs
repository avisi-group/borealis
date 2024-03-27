//! Rust module generation

use {
    crate::{
        boom::{
            passes::{
                self, builtin_fns::AddBuiltinFns, cycle_finder::CycleFinder,
                fold_unconditionals::FoldUnconditionals, make_exception_bool::MakeExceptionBool,
                remove_const_branch::RemoveConstBranch, resolve_bitvectors::ResolveBitvectors,
                resolve_return_assigns::ResolveReturns,
            },
            Ast,
        },
        brig::codegen::codegen,
        rudder,
    },
    common::create_file,
    log::info,
    proc_macro2::TokenStream,
    quote::quote,
    sailrs::{
        jib_ast::{Definition, Instruction},
        sail_ast::{Identifier, IdentifierAux, Location},
        types::ListVec,
    },
};

const FN_ALLOWLIST: &[&str] = &[
    "__DecodeA64",
    "__DecodeA64_DataProcImm",
    "decode_add_addsub_imm_aarch64_instrs_integer_arithmetic_add_sub_immediate",
    "execute_aarch64_instrs_integer_arithmetic_add_sub_immediate",
    "place_slice",
];

mod codegen;

/// Compiles a Sail model to a Brig module
pub fn sail_to_brig<I: Iterator<Item = Definition>>(jib_ast: I, standalone: bool) -> TokenStream {
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
            ResolveBitvectors::new_boxed(),
            AddBuiltinFns::new_boxed(),
            CycleFinder::new_boxed(),
        ],
    );

    // useful for debugging
    crate::boom::pretty_print::print_ast(
        &mut create_file("target/ast_processed.boom").unwrap(),
        ast.clone(),
    );

    info!("Building rudder");

    let rudder = rudder::build::from_boom(&ast.borrow());

    info!("Generating Rust");

    let registers_len = {
        let (last_register, last_offset) = rudder
            .get_registers()
            .into_iter()
            .max_by_key(|(typ, offset)| offset + typ.width_bytes())
            .unwrap();

        last_offset + last_register.width_bytes()
    };

    let state = quote! {
        // todo check this is necessary
        #[repr(align(8))]
        pub struct State {
            pc: u64,
            data: [u8; #registers_len],
        }

        impl Default for State {
            fn default() -> Self {
                Self {
                    pc: 0,
                    data: [0; #registers_len],
                }
            }
        }
    };

    let body = codegen(rudder);

    let prelude = if standalone {
        quote! {
            struct ConsoleTracer;

            impl Tracer for ConsoleTracer {
                fn begin(&self, pc: u64) {
                    println!("begin @ {pc:x}");
                }

                fn end(&self) {
                    println!("end");
                }

                fn read_register<T: core::fmt::Debug>(&self, offset: usize, value: T) {
                    println!("read-register {offset:x} = {value:?}");
                }
                fn write_register<T: core::fmt::Debug>(&self, offset: usize, value: T) {
                    println!("write-register {offset:x} = {value:?}");
                }
            }

            fn main() {
                let arg = std::env::args().skip(1).next().unwrap();
                let instruction = u32::from_str_radix(arg.trim_start_matches("0x"), 16).unwrap();
                dbg!(decode_execute(instruction, &mut State::default(), &mut ConsoleTracer));
            }
        }
    } else {
        quote! {
            use super::{CoreState, ExecutionEngine};

            pub struct AArch64Interpreter;

            impl CoreState for State {
                fn pc(&self) -> usize {
                    self.pc as usize
                }

                fn new(pc: usize) -> Self {
                    Self { pc: pc as u64, sp: 0, x: [0; 31] }
                }
            }

            fn fetch(pc: usize) -> u32 {
                unsafe { *(pc as *const u32) }
            }

            impl ExecutionEngine<State> for AArch64Interpreter {
                fn step(amount: super::StepAmount, state: &mut State) -> super::StepResult {
                    let insn_data = fetch(state.pc());
                    log::trace!("fetch @ {} = {:08x}", state.pc(), insn_data);

                    match decode_execute(insn_data, state) {
                        ExecuteResult::Ok => {
                            state.pc += 4;
                            super::StepResult::Ok

                        },
                        ExecuteResult::EndOfBlock => {
                            super::StepResult::Ok
                        },
                        ExecuteResult::UndefinedInstruction => {
                            panic!("undefined instruction {:08x}", insn_data)
                        }
                    }
                }
            }
        }
    };

    quote! {
        #![allow(non_snake_case)]
        #![allow(unused_assignments)]
        #![allow(unused_mut)]
        #![allow(unused_parens)]
        #![allow(unused_variables)]
        #![allow(dead_code)]
        #![allow(unreachable_code)]

        //! BOREALIS GENERATED FILE DO NOT MODIFY

        #state

        trait Tracer {
            fn begin(&self, pc: u64);
            fn end(&self);
            fn read_register<T: core::fmt::Debug>(&self, offset: usize, value: T);
            fn write_register<T: core::fmt::Debug>(&self, offset: usize, value: T);
        }

        #[derive(Debug)]
        enum ExecuteResult {
            Ok,
            EndOfBlock,
            UndefinedInstruction
        }

        #prelude

        #body
    }
}

fn apply_fn_allowlist<I: Iterator<Item = Definition>>(iter: I) -> impl Iterator<Item = Definition> {
    // stub out functions not in the allowlist

    iter.map(|def| {
        if let Definition::Fundef(name, idk, arguments, body) = def {
            let body = if FN_ALLOWLIST.contains(&name.as_interned().as_ref()) {
                body
            } else {
                ListVec::from(vec![
                    Instruction {
                        inner: sailrs::jib_ast::InstructionAux::Funcall(
                            sailrs::jib_ast::Expression::Void,
                            false,
                            (
                                Identifier {
                                    inner: IdentifierAux::Identifier("trap".into()),
                                    location: Location::Unknown,
                                },
                                ListVec::new(),
                            ),
                            ListVec::new(),
                        ),
                        annot: (0, Location::Unknown),
                    },
                    Instruction {
                        inner: sailrs::jib_ast::InstructionAux::Undefined(
                            sailrs::jib_ast::Type::Unit,
                        ),
                        annot: (0, Location::Unknown),
                    },
                ])
            };

            Definition::Fundef(name, idk, arguments, body)
        } else {
            def
        }
    })
}
