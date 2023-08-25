use {
    crate::{
        boom::{
            Ast, Expression, FunctionDefinition, Literal, Operation, OperationKind, Size,
            Statement, Type, Value,
        },
        passes::builtin_fns::HandlerFunction,
    },
    common::{identifiable::unique_id, intern::InternedString, HashMap},
    itertools::Itertools,
    once_cell::sync::Lazy,
    std::{cell::RefCell, rc::Rc},
};

pub static HANDLERS: Lazy<HashMap<InternedString, HandlerFunction>> = Lazy::new(|| {
    let mappings = [
        // this is a GenC builtin function so can be left as a noop here (technically not needed
        // but just being explicit)
        ("trap", noop as HandlerFunction),
        // requires bitvector conversion logic
        ("UInt", replace_with_copy),
        // we represent integers as u64s so these can be removed
        ("pcnt_i___pcnt_i64", replace_with_copy),
        ("pcnt_i64___pcnt_i", replace_with_copy),
        // replace with equality test
        ("eq_int", |ast, f, s| {
            replace_with_op(ast, f, s, OperationKind::Equal)
        }),
        ("lt_int", |ast, f, s| {
            replace_with_op(ast, f, s, OperationKind::LessThan)
        }),
        ("gteq_int", |ast, f, s| {
            replace_with_op(ast, f, s, OperationKind::GreaterThanOrEqual)
        }),
        ("shl_int", |ast, f, s| {
            replace_with_op(ast, f, s, OperationKind::LeftShift)
        }),
        ("shl8", |ast, f, s| {
            replace_with_op(ast, f, s, OperationKind::LeftShift)
        }),
        ("eq_vec", |ast, f, s| {
            replace_with_op(ast, f, s, OperationKind::Equal)
        }),
        ("not_vec", |ast, f, s| {
            replace_with_op(ast, f, s, OperationKind::Complement)
        }),
        ("not_bool", |ast, f, s| {
            replace_with_op(ast, f, s, OperationKind::Not)
        }),
        ("add_atom", |ast, f, s| {
            replace_with_op(ast, f, s, OperationKind::Add)
        }),
        ("sub_atom", |ast, f, s| {
            replace_with_op(ast, f, s, OperationKind::Subtract)
        }),
        ("xor_vec", |ast, f, s| {
            replace_with_op(ast, f, s, OperationKind::Xor)
        }),
        ("or_vec", |ast, f, s| {
            replace_with_op(ast, f, s, OperationKind::Or)
        }),
        ("and_vec", |ast, f, s| {
            replace_with_op(ast, f, s, OperationKind::And)
        }),
        ("add_vec", |ast, f, s| {
            replace_with_op(ast, f, s, OperationKind::Add)
        }),
        ("add_vec_int", |ast, f, s| {
            replace_with_op(ast, f, s, OperationKind::Add)
        }),
        ("neq_vec", |ast, f, s| {
            replace_with_op(ast, f, s, OperationKind::NotEqual)
        }),
        ("ROR", |ast, f, s| {
            replace_with_op(ast, f, s, OperationKind::RotateRight)
        }),
        ("ASR", |ast, f, s| {
            replace_with_op(ast, f, s, OperationKind::RightShift)
        }),
        ("LSR", |ast, f, s| {
            replace_with_op(ast, f, s, OperationKind::RightShift)
        }),
        ("LSL", |ast, f, s| {
            replace_with_op(ast, f, s, OperationKind::LeftShift)
        }),
        ("SInt", replace_with_copy), // probably need to take another look at this
        ("bitvector_length", bv_length_handler),
        ("bitvector_access_B", bv_access_handler),
        ("bitvector_access_A", bv_access_handler),
        ("raw_GetSlice_int", noop),
        ("ZeroExtend__0", zero_extend_handler),
        ("ZeroExtend__1", zero_extend_handler),
        ("SignExtend__0", sign_extend_handler),
        ("eq_anything_EArchVersion_pcnt__", |ast, f, s| {
            replace_with_op(ast, f, s, OperationKind::Equal)
        }),
        ("eq_anything_EMoveWideOp_pcnt__", |ast, f, s| {
            replace_with_op(ast, f, s, OperationKind::Equal)
        }),
        ("eq_anything_EBranchType_pcnt__", |ast, f, s| {
            replace_with_op(ast, f, s, OperationKind::Equal)
        }),
        ("slice", noop),
        ("Zeros", noop),
        ("undefined_bitvector", noop),
        ("bitvector_concat", noop),
        ("aget_PC", |ast, f, s| rename(ast, f, s, "read_pc".into())),
        ("sail_assert", assert_handler),
        // delete undefined initializers
        ("undefined_bool", delete),
        ("undefined_LogicalOp", delete),
        ("undefined_int", delete),
        ("undefined_MoveWideOp", delete),
        ("replicate_bits", replicate_bits_handler),
        ("SetSlice_int", noop),
        ("SetSlice_bits", set_slice_handler),
        //
        ("undefined_real", noop),
        ("undefined_string", noop),
        ("undefined_unit", noop),
        ("ediv_int", noop),
        ("lteq_int", noop),
        ("eq_bool", noop),
        ("negate_atom", noop),
        ("mult_atom", noop),
        ("DecStr", noop),
        ("asl_prerr_string", noop),
        ("Error_Implementation_Defined", noop),
        ("Error_ExceptionTaken", noop),
        ("append_str", noop),
        ("GetVerbosity", noop),
        ("WriteRAM_bool", noop),
        ("cons", noop),
        ("Error_See", noop),
        ("update_fbits", noop),
        ("vector_access_A_B32_", noop),
        ("Error_Undefined", noop),
        ("eq_anything_EFPRounding_pcnt__", noop),
        ("eq_real", noop),
        ("pcnt_string___pcnt_real", noop),
        ("quotient_real", noop),
        ("sub_real", noop),
        ("abs_real", noop),
        ("vector_update_B_B64_", noop),
        ("eq_anything_ECountOp_pcnt__", noop),
        ("vector_access_B_B32_", noop),
        ("sub_vec", noop),
        ("real_power", noop),
        ("internal_pick_EFPConvOp_pcnt__", noop),
        ("undefined_vector_RTLBLine_", noop),
        ("read_register_B32_", noop),
        ("Sqrt", noop),
        ("eq_anything_EConstraint_pcnt__", noop),
        ("eq_anything_EDeviceType_pcnt__", noop),
        ("eq_anything_EMBReqDomain_pcnt__", noop),
        ("Real", noop),
        ("internal_pick_EMemBarrierOp_pcnt__", noop),
        ("internal_pick_EMoveWideOp_pcnt__", noop),
        ("print_bits", noop),
        ("sail_zeros", noop),
        ("undefined_vector_B16_", noop),
        ("vector_subrange_A", noop),
        ("internal_pick_EInstrSet_pcnt__", noop),
        ("load_raw", noop),
        ("sub_vec_int", noop),
        ("emod_int", noop),
        ("eq_anything_EGTEParamType_pcnt__", noop),
        ("eq_anything_EMemOp_pcnt__", noop),
        ("vector_update_B_B128_", noop),
        ("internal_pick_EFPType_pcnt__", noop),
        ("internal_pick_EGTEParamType_pcnt__", noop),
        ("RoundUp", noop),
        ("internal_pick_EMBReqDomain_pcnt__", noop),
        ("internal_pick_EMemAtomicOp_pcnt__", noop),
        ("min_int", noop),
        ("internal_pick_ESystemHintOp_pcnt__", noop),
        ("internal_pick_EDeviceType_pcnt__", noop),
        ("pow2_atom", noop),
        ("gt_real", noop),
        ("undefined_vector_o_", noop),
        ("internal_pick_EBranchType_pcnt__", noop),
        ("vector_access_A_B128_", noop),
        ("eq_bit", noop),
        ("shl1", noop),
        ("get_cycle_count", noop),
        ("zeros", noop),
        ("internal_pick_EFPMaxMinOp_pcnt__", noop),
        ("vector_access_A_RTLBLine_", noop),
        ("internal_pick_ECompareOp_pcnt__", noop),
        ("max_int", noop),
        ("vector_access_B_i_", noop),
        ("eq_anything_Esignal_pcnt__", noop),
        ("vector_update_B_B32_", noop),
        ("eq_anything_EInstrSet_pcnt__", noop),
        ("print", noop),
        ("ReadRAM", noop),
        ("gteq_real", noop),
        ("shr32", noop),
        ("vector_update_B_o_", noop),
        ("eq_anything_ESRType_pcnt__", noop),
        ("internal_pick_EMBReqTypes_pcnt__", noop),
        ("internal_pick_EImmediateOp_pcnt__", noop),
        ("undefined_vector_B64_", noop),
        ("undefined_vector_B128_", noop),
        ("undefined_vector_i_", noop),
        ("vector_update_B_B16_", noop),
        ("check_cycle_count", noop),
        ("shr_int", noop),
        ("eq_string", noop),
        ("shl32", noop),
        ("WakeupRequest", noop),
        ("vector_update_subrange", noop),
        ("internal_pick_Esignal_pcnt__", noop),
        ("abs_int_atom", noop),
        ("concat_str", noop),
        ("vector_update_B_i_", noop),
        ("internal_pick_EConstraint_pcnt__", noop),
        ("internal_pick_EMemType_pcnt__", noop),
        ("vector_update_B_RTLBLine_", noop),
        ("internal_pick_EFPUnaryOp_pcnt__", noop),
        ("Error_Unpredictable", noop),
        ("Error_ReservedEncoding", noop),
        ("gt_int", noop),
        ("SleepRequest", noop),
        ("vector_access_B_o_", noop),
        ("add_real", noop),
        ("eq_anything_EFault_pcnt__", noop),
        ("eq_anything_EMemType_pcnt__", noop),
        ("prerr_bits", noop),
        ("internal_pick_ELogicalOp_pcnt__", noop),
        ("eq_anything_EException_pcnt__", noop),
        ("internal_pick_EException_pcnt__", noop),
        ("RoundDown", noop),
        ("prerr_int", noop),
        ("zero_extend", noop),
        ("HexStr", noop),
        ("internal_pick_ESRType_pcnt__", noop),
        ("lt_real", noop),
        ("pcnt_string___pcnt_i", noop),
        ("vector_access_A__ref_B32_", noop),
        ("eq_anything_EAccType_pcnt__", noop),
        ("eq_anything_r_", noop),
        ("internal_pick_EAccType_pcnt__", noop),
        ("putchar", noop),
        ("mult_real", noop),
        ("vector_access_B_B16_", noop),
        ("internal_pick_EFault_pcnt__", noop),
        ("Error_SError", noop),
        ("internal_pick_EMemOp_pcnt__", noop),
        ("vector_access_B_B64_", noop),
        ("vector_subrange_B", noop),
        ("vector_access_A_B64_", noop),
        ("negate_real", noop),
        ("undefined_vector_B32_", noop),
        ("vector_access_B_RTLBLine_", noop),
        ("eq_anything_EFPType_pcnt__", noop),
        ("vector_access_A_o_", noop),
        ("internal_pick_EPSTATEField_pcnt__", noop),
        ("internal_pick_EPrefetchHint_pcnt__", noop),
        ("internal_pick_EVBitOp_pcnt__", noop),
        ("internal_pick_EFPRounding_pcnt__", noop),
    ]
    .into_iter()
    .map(|(s, f)| (InternedString::from_static(s), f));

    assert!(
        mappings.clone().map(|(n, _)| n).all_unique(),
        "non-unique function name found in mappings to handler!"
    );

    HashMap::from_iter(mappings)
});

pub fn noop(
    _ast: Rc<RefCell<Ast>>,
    _function: FunctionDefinition,
    _statement: Rc<RefCell<Statement>>,
) {
}

pub fn delete(
    _ast: Rc<RefCell<Ast>>,
    function: FunctionDefinition,
    statement: Rc<RefCell<Statement>>,
) {
    let (block, index) = function.entry_block.find_statement(statement).unwrap();
    let mut statements = block.statements();
    statements.remove(index);
    block.set_statements(statements);
}

/// Blindly replace function call with assignment
pub fn replace_with_copy(
    _ast: Rc<RefCell<Ast>>,
    _function: FunctionDefinition,
    statement: Rc<RefCell<Statement>>,
) {
    // get function arguments, and expression the function output is being assigned
    // to
    let (expression, arguments) = {
        let Statement::FunctionCall {
            expression,
            arguments,
            ..
        } = &*statement.borrow()
        else {
            panic!("these should all be function call statements")
        };

        (expression.clone(), arguments.clone())
    };

    // expression should be Some
    let Some(expression) = expression else {
        panic!("no lhs");
    };

    // // should *probably* be an identifier, but we could allow any expression kind
    // // here
    // let Expression::Identifier(_) = expression else {
    //     panic!("non ident expression");
    // };

    // can only do this with a single argument
    assert_eq!(arguments.len(), 1);

    // turn it into a copy
    *statement.borrow_mut() = Statement::Copy {
        expression: expression.clone(),
        value: arguments[0].clone(),
    };
}

///
pub fn replace_with_op(
    _: Rc<RefCell<Ast>>,
    _: FunctionDefinition,
    statement: Rc<RefCell<Statement>>,
    operator: OperationKind,
) {
    // assert there are two arguments, lhs and rhs
    // replace function call with copy where the value is an equals operation

    let (expression, args) = {
        let Statement::FunctionCall {
            expression: Some(expression),
            arguments,
            ..
        } = &*statement.borrow()
        else {
            panic!();
        };

        (expression.clone(), arguments.clone())
    };

    let operation = match operator {
        OperationKind::Not => Operation::Not(args[0].clone()),
        OperationKind::Complement => Operation::Complement(args[0].clone()),
        OperationKind::Equal => Operation::Equal(args[0].clone(), args[1].clone()),
        OperationKind::NotEqual => Operation::NotEqual(args[0].clone(), args[1].clone()),
        OperationKind::LessThan => Operation::LessThan(args[0].clone(), args[1].clone()),
        OperationKind::GreaterThan => Operation::GreaterThan(args[0].clone(), args[1].clone()),
        OperationKind::LessThanOrEqual => {
            Operation::LessThanOrEqual(args[0].clone(), args[1].clone())
        }
        OperationKind::GreaterThanOrEqual => {
            Operation::GreaterThanOrEqual(args[0].clone(), args[1].clone())
        }
        OperationKind::Subtract => Operation::Subtract(args[0].clone(), args[1].clone()),
        OperationKind::Add => Operation::Add(args[0].clone(), args[1].clone()),
        OperationKind::Or => Operation::Or(args[0].clone(), args[1].clone()),
        OperationKind::And => Operation::And(args[0].clone(), args[1].clone()),
        OperationKind::LeftShift => Operation::LeftShift(args[0].clone(), args[1].clone()),
        OperationKind::RightShift => Operation::RightShift(args[0].clone(), args[1].clone()),
        OperationKind::Xor => Operation::Xor(args[0].clone(), args[1].clone()),
        OperationKind::RotateLeft => Operation::RotateLeft(args[0].clone(), args[1].clone()),
        OperationKind::RotateRight => Operation::RotateRight(args[0].clone(), args[1].clone()),
        OperationKind::Cast => panic!(),
    };

    *statement.borrow_mut() = Statement::Copy {
        expression,
        value: operation.into(),
    };
}

///
pub fn bv_length_handler(
    _: Rc<RefCell<Ast>>,
    fn_def: FunctionDefinition,
    statement: Rc<RefCell<Statement>>,
) {
    // get ident of argument from statement, use that to lookup the type, which
    // should have a size

    let Statement::FunctionCall {
        expression,
        arguments,
        ..
    } = statement.borrow().clone()
    else {
        panic!();
    };

    assert_eq!(arguments.len(), 1);

    let Value::Identifier(bv_ident) = &*arguments[0].borrow() else {
        panic!();
    };

    let size = match fn_def.get_ident_type(*bv_ident).unwrap() {
        Type::Int {
            size: Size::Static(size),
            ..
        } => Literal::Int(size.into()).into(),
        Type::Int {
            size: Size::Runtime(ident),
            ..
        } => Rc::new(RefCell::new(Value::Identifier(ident))),
        Type::Int {
            size: Size::Unknown,
            ..
        } => Literal::Int(64.into()).into(),
        _ => panic!("not a bv"),
    };

    *statement.borrow_mut() = Statement::Copy {
        expression: expression.as_ref().unwrap().clone(),
        value: size,
    };

    // replace statement with a copy of that size as a literal to the original
    // destination
}

pub fn bv_access_handler(
    _: Rc<RefCell<Ast>>,
    _: FunctionDefinition,
    statement: Rc<RefCell<Statement>>,
) {
    // access a bit of a bitvector by generating the appropriate shifting logic
    // dest = bitvector_access_B(input, index);
    //
    // dest = (input >> index) & 1

    let Statement::FunctionCall {
        expression,
        arguments,
        ..
    } = statement.borrow().clone()
    else {
        panic!();
    };

    *statement.borrow_mut() = Statement::Copy {
        expression: expression.unwrap(),
        value: Operation::And(
            Operation::RightShift(arguments[0].clone(), arguments[1].clone()).into(),
            Literal::Int(1.into()).into(),
        )
        .into(),
    }
}

pub fn zero_extend_handler(
    _ast: Rc<RefCell<Ast>>,
    function: FunctionDefinition,
    statement: Rc<RefCell<Statement>>,
) {
    let Statement::FunctionCall {
        expression,
        arguments,
        ..
    } = statement.borrow().clone()
    else {
        panic!();
    };

    // x1 = ZeroExtend__0(x, n)
    //
    //

    let Value::Identifier(x_ident) = &*arguments[0].borrow() else {
        panic!();
    };

    let x_len = match function.get_ident_type(*x_ident).unwrap() {
        Type::Int {
            size: Size::Static(len),
            ..
        } => len as isize,
        _ => return,
    };

    // (x << (64 - x_len)) >> (64 - x_len)

    *statement.borrow_mut() = Statement::Copy {
        expression: expression.unwrap(),
        value: Operation::RightShift(
            Operation::LeftShift(
                Operation::Cast(
                    arguments[0].clone(),
                    Rc::new(RefCell::new(Type::Int {
                        signed: false,
                        size: Size::Static(64),
                    })),
                )
                .into(),
                Operation::Subtract(arguments[1].clone(), Literal::Int(x_len.into()).into()).into(),
            )
            .into(),
            Operation::Subtract(arguments[1].clone(), Literal::Int(x_len.into()).into()).into(),
        )
        .into(),
    };
}

pub fn sign_extend_handler(
    _ast: Rc<RefCell<Ast>>,
    function: FunctionDefinition,
    statement: Rc<RefCell<Statement>>,
) {
    let Statement::FunctionCall {
        expression,
        arguments,
        ..
    } = statement.borrow().clone()
    else {
        panic!();
    };

    // x1 = ZeroExtend__0(x, n)
    //
    //

    let Value::Identifier(x_ident) = &*arguments[0].borrow() else {
        panic!();
    };

    let len_value: Rc<RefCell<Value>> =
        if let Type::Int { size, .. } = function.get_ident_type(*x_ident).unwrap() {
            match size {
                Size::Static(size) => Literal::Int(size.into()).into(),
                Size::Runtime(ident) => Rc::new(RefCell::new(Value::Identifier(ident))),
                Size::Unknown => panic!("unknown size"),
            }
        } else {
            panic!(
                "could not get type of {x_ident:?} in {}",
                function.signature.name
            );

            // function.entry_block.get_assignment(ident)
            // Rc::new(RefCell::new(Value::Literal(Rc::new(RefCell::new(
            //     Literal::Int(x_len.into()),
            // )))))
        };

    // ((sint64)x << (64 - x_len)) >> (64 - x_len)

    *statement.borrow_mut() = Statement::Copy {
        expression: expression.unwrap(),
        value: Operation::RightShift(
            Operation::Cast(
                Operation::LeftShift(
                    Operation::Cast(
                        arguments[0].clone(),
                        Rc::new(RefCell::new(Type::Int {
                            signed: true,
                            size: Size::Static(64),
                        })),
                    )
                    .into(),
                    Operation::Subtract(arguments[1].clone(), len_value.clone()).into(),
                )
                .into(),
                Rc::new(RefCell::new(Type::Int {
                    signed: true,
                    size: Size::Static(64),
                })),
            )
            .into(),
            Operation::Subtract(arguments[1].clone(), len_value).into(),
        )
        .into(),
    };
}

fn rename(
    _ast: Rc<RefCell<Ast>>,
    _function: FunctionDefinition,
    statement: Rc<RefCell<Statement>>,
    new_name: InternedString,
) {
    let Statement::FunctionCall { name, .. } = &mut *statement.borrow_mut() else {
        panic!();
    };

    *name = new_name;
}

pub fn assert_handler(
    _ast: Rc<RefCell<Ast>>,
    _function: FunctionDefinition,
    statement: Rc<RefCell<Statement>>,
) {
    let Statement::FunctionCall {
        expression,
        arguments,
        ..
    } = &mut *statement.borrow_mut()
    else {
        panic!();
    };

    *expression = None;
    arguments.pop();
}

pub fn replicate_bits_handler(
    _ast: Rc<RefCell<Ast>>,
    function: FunctionDefinition,
    statement: Rc<RefCell<Statement>>,
) {
    let (expression, name, length, count) = {
        let Statement::FunctionCall {
            expression,
            arguments,
            ..
        } = &*statement.borrow()
        else {
            panic!();
        };

        let Value::Identifier(ident) = &*arguments[1].borrow() else {
            panic!();
        };

        let Some(value) = function.entry_block.get_assignment(*ident) else {
            return;
        };

        let Value::Literal(literal) = &*value.borrow() else {
            panic!();
        };

        let Literal::Int(count) = &*literal.borrow() else {
            panic!();
        };

        let Value::Identifier(ident) = &*arguments[0].borrow() else {
            panic!();
        };

        let Some(Type::Int {
            size: Size::Static(length),
            ..
        }) = function.get_ident_type(*ident)
        else {
            panic!();
        };

        (
            expression.clone().unwrap(),
            *ident,
            length,
            usize::try_from(count).unwrap(),
        )
    };

    let (block, idx) = function
        .entry_block
        .find_statement(statement.clone())
        .unwrap();

    let mut statements = block.statements();

    statements.remove(idx);

    let rep_statements = {
        let mut buf = vec![];

        let id = unique_id();

        // start sequence by copying in input ident
        let mut prev_ident = format!("rep_{id}_0").into();
        buf.push(
            Statement::TypeDeclaration {
                name: prev_ident,
                typ: Rc::new(RefCell::new(Type::Int {
                    signed: false,
                    size: Size::Static(64),
                })),
            }
            .into(),
        );
        buf.push(
            Statement::Copy {
                expression: Expression::Identifier(prev_ident),
                value: Rc::new(RefCell::new(Value::Identifier(name))),
            }
            .into(),
        );

        // for each intermediate shift and or create a new var
        for i in 1..count {
            let this_ident = format!("rep_{id}_{i}").into();
            buf.push(
                Statement::TypeDeclaration {
                    name: this_ident,
                    typ: Rc::new(RefCell::new(Type::Int {
                        signed: false,
                        size: Size::Static(64),
                    })),
                }
                .into(),
            );
            buf.push(
                Statement::Copy {
                    expression: Expression::Identifier(this_ident),
                    value: Operation::Or(
                        Operation::LeftShift(
                            Rc::new(RefCell::new(Value::Identifier(name))),
                            Literal::Int((length * i).into()).into(),
                        )
                        .into(),
                        Rc::new(RefCell::new(Value::Identifier(prev_ident))),
                    )
                    .into(),
                }
                .into(),
            );
            prev_ident = this_ident;
        }

        // copy out result
        buf.push(
            Statement::Copy {
                expression,
                value: Rc::new(RefCell::new(Value::Identifier(prev_ident))),
            }
            .into(),
        );

        buf
    };

    statements.splice(idx..idx, rep_statements);
    block.set_statements(statements);
}

pub fn set_slice_handler(
    _ast: Rc<RefCell<Ast>>,
    _function: FunctionDefinition,
    statement: Rc<RefCell<Statement>>,
) {
    // result = 0;
    // // RefCell { value: LargeInt }
    // uint64 gs_12779;
    // // Identifier("gs_12779") RefCell { value: Literal(RefCell { value: Int(16)
    // }) } gs_12779 = 16;
    // // RefCell { value: LargeInt }
    // uint64 gs_155056;
    // // Identifier("gs_155056") RefCell { value: Identifier("datasize") }
    // gs_155056 = datasize;
    // // RefCell { value: FixedBits(16, false) }
    // uint16 gs_155057;
    // // Identifier("gs_155057") RefCell { value: Identifier("imm") }
    // gs_155057 = imm;
    // result = SetSlice_bits(gs_155056, gs_12779, result, pos, gs_155057);

    // void set_slice(lbits *rop,
    //     const sail_int len_mpz,
    //     const sail_int slen_mpz,
    //     const lbits op,
    //     const sail_int start_mpz,
    //     const lbits slice)

    // result = (result & (~(((1 << slice_len) - 1) << pos))) | (slice << pos)
    let (expression, slice_len, input, pos, slice) = {
        let Statement::FunctionCall {
            expression,

            arguments,
            ..
        } = &*statement.borrow()
        else {
            panic!();
        };

        (
            expression.clone().unwrap(),
            arguments[1].clone(),
            arguments[2].clone(),
            arguments[3].clone(),
            arguments[4].clone(),
        )
    };

    *statement.borrow_mut() = Statement::Copy {
        expression,
        value: Operation::Or(
            Operation::And(
                input,
                Operation::Complement(
                    Operation::LeftShift(
                        Operation::Subtract(
                            Operation::LeftShift(Literal::Int(1.into()).into(), slice_len).into(),
                            Literal::Int(1.into()).into(),
                        )
                        .into(),
                        pos.clone(),
                    )
                    .into(),
                )
                .into(),
            )
            .into(),
            Operation::LeftShift(slice, pos).into(),
        )
        .into(),
    };
}
