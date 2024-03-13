//! Rust module generation

use {
    crate::{
        boom::Ast,
        passes::{
            self, builtin_fns::AddBuiltinFns, cycle_finder::CycleFinder,
            fold_unconditionals::FoldUnconditionals, make_exception_bool::MakeExceptionBool,
            remove_const_branch::RemoveConstBranch,
            remove_redundant_assigns::RemoveRedundantAssigns,
            resolve_bitvectors::ResolveBitvectors, resolve_return_assigns::ResolveReturns,
        },
        rudder,
        rust::codegen::codegen,
    },
    log::info,
    proc_macro2::TokenStream,
    quote::quote,
    sail::{jib_ast::Definition, sail_ast},
    std::{cell::RefCell, collections::LinkedList, rc::Rc},
};

mod codegen;
mod decode;

/// Compiles a Sail model to a Brig module
pub fn sail_to_brig(
    sail_ast: &sail_ast::Ast,
    jib_ast: &LinkedList<Definition>,
    standalone: bool,
) -> TokenStream {
    info!("Converting JIB to BOOM");
    let ast = Ast::from_jib(jib_ast);

    // only run on a subset of the model, for now
    apply_function_denylist(ast.clone());

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
            RemoveRedundantAssigns::new_boxed(),
            CycleFinder::new_boxed(),
        ],
    );

    // // useful for debugging
    // crate::boom::pretty_print::print_ast(&mut std::io::stdout(), ast.clone());
    // panic!();

    info!("Building rudder");

    let rudder = rudder::build::from_boom(&ast.borrow());

    info!("Generating Rust");

    let max = rudder
        .get_registers()
        .into_iter()
        .max_by_key(|(typ, offset)| offset + typ.width_bytes())
        .unwrap();

    let len = max.0.width_bytes() + max.1;

    let state = quote! {
        // todo check this is necessary
        #[repr(align(8))]
        pub struct State {
            // todo PC here
            data: [u8; #len],
        }

        impl Default for State {
            fn default() -> Self {
                Self {
                    data: [0; #len],
                }
            }
        }
    };

    let decode_fn = decode::generate_fn(sail_ast, &rudder);

    let execute_fns = crate::rust::codegen(rudder);

    let prelude = if standalone {
        quote! {
            fn main() {
                let arg = std::env::args().skip(1).next().unwrap();
                let instruction = u32::from_str_radix(arg.trim_start_matches("0x"), 16).unwrap();
                dbg!(decode_execute(instruction, &mut State::default()));
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

        //! BOREALIS GENERATED FILE DO NOT MODIFY

        #state

        #[derive(Debug)]
        enum ExecuteResult {
            Ok,
            EndOfBlock,
            UndefinedInstruction
        }

        #prelude

        #decode_fn

        #execute_fns
    }
}

fn apply_function_denylist(ast: Rc<RefCell<Ast>>) {
    // filter out non addsub functions
    let funcs = ast
        .borrow()
        .functions
        .clone()
        .into_iter()
        .filter(|(k, _)| {
            [
                "integer_arithmetic_addsub_immediate_decode",
                "u__id",
                "ReservedValue",
                "u__PostDecode",
                "integer_arithmetic_addsub_immediate",
                "aget_X",
                "AddWithCarry",
                "u__GetSlice_int",
                "IsZero",
                "aset_X",
            ]
            .contains(&k.as_ref())
        })
        .collect();
    ast.borrow_mut().functions = funcs;
}
