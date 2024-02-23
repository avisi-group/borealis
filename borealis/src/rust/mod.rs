//! Rust module generation

use {
    crate::{
        boom::{self, control_flow::ControlFlowBlock, Ast, Statement},
        genc::codegen::{
            format::process_decode_function_clause, instruction::get_instruction_entrypoint_fns,
        },
        passes::{
            self, builtin_fns::AddBuiltinFns, cycle_finder::CycleFinder,
            destruct_structs::DestructStructs, fold_unconditionals::FoldUnconditionals,
            remove_const_branch::RemoveConstBranch, remove_exception::RemoveExceptions,
            remove_redundant_assigns::RemoveRedundantAssigns,
            resolve_bitvectors::ResolveBitvectors, resolve_return_assigns::ResolveReturns,
        },
        rudder,
        rust::codegen::codegen,
    },
    common::intern::InternedString,
    itertools::Itertools,
    log::info,
    proc_macro2::TokenStream,
    quote::quote,
    sail::{jib_ast::Definition, sail_ast},
    std::{cell::RefCell, collections::LinkedList, io::stdout, rc::Rc},
};

mod codegen;
mod decode;

/// Compiles a Sail model to a Brig module
pub fn sail_to_brig(sail_ast: &sail_ast::Ast, jib_ast: &LinkedList<Definition>) -> TokenStream {
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
            ResolveBitvectors::new_boxed(),
            AddBuiltinFns::new_boxed(),
            RemoveRedundantAssigns::new_boxed(),
            CycleFinder::new_boxed(),
        ],
    );

    info!("Generating Rust");

    // let reg_fields =
    // TokenStream::from_iter(ast.borrow().registers.iter().map(|(name, typ)| {
    //     let typ_str = Ident::new(&typ.emit_string(), Span::call_site());
    //     quote! {
    //         #name: #typ_str,
    //     }
    // }));
    let reg_fields = quote! {
        pc: u64,
        sp: u64,
        x: [u64; 31],
    };

    let decode_fn = decode::generate_fn(sail_ast);

    let execute_fns = generate_fns(ast, sail_ast);

    quote! {
        //! BOREALIS GENERATED FILE DO NOT MODIFY

        use super::{CoreState, ExecutionEngine};

        pub struct AArch64Interpreter;

        pub struct AArch64CoreState {
            #reg_fields
        }

        impl CoreState for AArch64CoreState {
            fn pc(&self) -> usize {
                self.pc as usize
            }

            fn new(pc: usize) -> Self {
                Self { pc: pc as u64, sp: 0, x: [0; 31] }
            }
        }

        impl ExecutionEngine<AArch64CoreState> for AArch64Interpreter {
            fn step(amount: super::StepAmount, state: &mut AArch64CoreState) -> super::StepResult {
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

        fn fetch(pc: usize) -> u32 {
            unsafe { *(pc as *const u32) }
        }

        enum ExecuteResult {
            Ok,
            EndOfBlock,
            UndefinedInstruction
        }

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
        .map(|(k, mut def)| {
            // if it's not an allowlisted function, delete the body
            if ![
                "integer_arithmetic_addsub_immediate_decode",
                // "integer_arithmetic_addsub_immediate",
                // "u__id",
                // "AddWithCarry",
                // "IsZero",
                // "u__GetSlice_int",
                // "integer_logical_shiftedreg_decode",
                // "DecodeShift",
                // "integer_logical_shiftedreg",
                // "ShiftReg",
                // "branch_conditional_cond_decode",
                // "branch_conditional_cond",
                // "integer_insext_insert_movewide_decode",
                // "integer_insext_insert_movewide",
                // "integer_arithmetic_addsub_shiftedreg_decode",
                // "DecodeShift",
                // "integer_arithmetic_addsub_shiftedreg",
                // "IsZeroBit",
                // "ConditionHolds",
                // "BranchTo",
                // "branch_unconditional_immediate_decode",
                // "branch_unconditional_immediate",
                // "memory_single_general_immediate_unsigned_memory_single_general_immediate_signed_postidx__decode",
                // "memory_single_general_immediate_signed_postidx",
                // "ConstrainUnpredictable",
                // "system_hints_decode",
                // "integer_arithmetic_address_pcrel_decode",
                // "integer_arithmetic_address_pcrel",
                // "memory_pair_general_preidx_memory_pair_general_postidx__decode",
                // "memory_pair_general_postidx",
                // "memory_pair_general_offset_memory_pair_general_postidx__decode",
                // "integer_arithmetic_addsub_extendedreg_decode",
                // "DecodeRegExtend",
                // "integer_arithmetic_addsub_extendedreg",
                // "ExtendReg",
                // "memory_single_general_immediate_signed_postidx_memory_single_general_immediate_signed_postidx__decode",
                // "branch_conditional_compare_decode",
                // "branch_conditional_compare",
                // "memory_single_general_immediate_signed_preidx_memory_single_general_immediate_signed_postidx__decode",
                // "integer_conditional_select_decode",
                // "integer_conditional_select",
                // "integer_logical_immediate_decode",
                // "DecodeBitMasks",
                // "HighestSetBit",
                // "integer_logical_immediate",
                // "memory_pair_general_postidx_memory_pair_general_postidx__decode",
                // "branch_unconditional_register_decode",
                // "branch_unconditional_register",
                // "system_exceptions_runtime_svc_decode",
                // "system_exceptions_runtime_svc",
                // "integer_conditional_compare_immediate_decode",
                // "integer_conditional_compare_immediate",
                // "memory_single_general_register_memory_single_general_register__decode",
                // "memory_single_general_register",
                // "integer_conditional_compare_register_decode",
                // "integer_conditional_compare_register",
                // "memory_single_general_immediate_signed_offset_normal_memory_single_general_immediate_signed_offset_normal__decode",
                // "memory_single_general_immediate_signed_offset_normal",
                // "integer_bitfield_decode",
                // "integer_bitfield",
                // "branch_conditional_test_decode",
                // "branch_conditional_test",
                // "system_register_system_decode",
                // "AArch64_CheckSystemAccess",
                // "system_register_system",
                // "u__IMPDEF_boolean",
                // "u__IMPDEF_boolean_map",
                // "vector_arithmetic_binary_uniform_mul_int_doubling_sisd_decode",
                // "memory_literal_general_decode",
                // "memory_literal_general"

                // // CHECKPOINT

                //"AArch64_SysRegRead",
                // "AArch64_SysRegWrite",
                // "HaveBTIExt",
                // "HasArchVersion",
                // "BranchTargetCheck",
                // "AArch64_ExecutingBROrBLROrRetInstr",
                // "AArch64_ExecutingBTIInstr",
                // "ThisInstr",
                // "HaveNVExt"
            ]
            .contains(&k.as_ref())
            {
                def.entry_block = ControlFlowBlock::new();
                def.entry_block.set_statements(vec![Rc::new(RefCell::new(
                    Statement::FunctionCall {
                        expression: None,
                        name: "trap".into(),
                        arguments: vec![],
                    },
                ))])
            }

            (k, def)
        })
        .collect();
    ast.borrow_mut().functions = funcs;
}

/// Generates all BOOM functions, passing state struct to each
///
/// TODO: make this work on all functions? or work recursively like in genc
pub fn generate_fns(boom: Rc<RefCell<boom::Ast>>, sail: &sail_ast::Ast) -> TokenStream {
    let rudder_ctx = rudder::build::from_boom(&*(*boom).borrow());
    codegen(rudder_ctx)
}
