//! GenC function generation from BOOM

use {
    crate::{
        boom::{
            control_flow::{util::find_common_block, ControlFlowBlock, Terminator},
            visitor::{Visitor, Walkable},
            Ast, Definition, Statement,
        },
        genc::{self, codegen::emit::Emit, HelperFunction},
        Indent,
    },
    common::{intern::InternedString, HashMap, HashSet},
    itertools::Itertools,
    once_cell::sync::Lazy,
    std::{cell::RefCell, fmt::Write, rc::Rc},
};

/// GenC builtin functions that do not need to be generated
static BUILTIN_FNS: Lazy<HashSet<InternedString>> = Lazy::new(|| {
    let names = ["trap"];
    HashSet::from_iter(names.into_iter().map(InternedString::from_static))
});

/// Pre-generated GenC functions in text form to be inserted
static PREGENERATED_FNS: Lazy<HashMap<InternedString, HelperFunction>> = Lazy::new(|| {
    let fns = [
        HelperFunction {
            name: "aset_X".into(),
            parameters: "uint8 n, uint64 value".into(),
            return_type: "void".into(),
            body: r#"
                if (n != 31) {
                    write_register_bank(reg_RB, n, value);
                }
                return;
            "#
            .into(),
        },
        HelperFunction {
            name: "aget_X".into(),
            parameters: "uint8 width, uint8 n".into(),
            return_type: "uint64".into(),
            body: r#"
                if (n == 31) {
                    return 0;
                }

                uint64 value = read_register_bank(reg_RB, n);

                switch (width) {
                    case 64: {
                        return value;
                    }

                    case 32: {
                        return (uint32)value;
                    }

                    default: {
                        trap();
                        return 0;
                    }
                }
            "#
            .into(),
        },
        HelperFunction {
            name: "aset_SP".into(),
            parameters: "uint64 value".into(),
            return_type: "void".into(),
            body: r#"
                write_register(reg_SP, value);
                return;
            "#
            .into(),
        },
        HelperFunction {
            name: "aget_SP".into(),
            parameters: "uint8 width".into(),
            return_type: "uint64".into(),
            body: r#"
                uint64 value = read_register(reg_SP);

                switch (width) {
                    case 64: {
                        return value;
                    }

                    case 32: {
                        return (uint32)value;
                    }

                    default: {
                        trap();
                        return 0;
                    }
                }
            "#
            .into(),
        },
        HelperFunction {
            name: "aset_Mem".into(),
            parameters: "uint64 address, uint64 size, uint32 acctype, uint64 value_name__arg"
                .into(),
            return_type: "void".into(),
            body: r#"
                switch (size) {
                case 1: {
                    mem_write_8(Data, address, (uint8)value_name__arg);
                    break;
                }

                case 2: {
                    mem_write_16(Data, address, (uint16)value_name__arg);
                    break;
                }

                case 4: {
                    mem_write_32(Data, address, (uint32)value_name__arg);
                    break;
                }

                case 8: {
                    mem_write_64(Data, address, value_name__arg);
                    break;
                }

                default: {
                    trap();
                    break;
                }
                }

                return;
            "#
            .into(),
        },
        HelperFunction {
            name: "aget_Mem".into(),
            parameters: "uint64 address, uint64 size, uint32 acctype".into(),
            return_type: "uint64".into(),
            body: r#"
                uint64 read_data;

                switch (size) {
                case 1: {
                    uint8 data8;
                    mem_read_8(Data, address, data8);
                    read_data = data8;
                    break;
                }

                case 2: {
                    uint16 data16;
                    mem_read_16(Data, address, data16);
                    read_data = data16;
                    break;
                }

                case 4: {
                    uint32 data32;
                    mem_read_32(Data, address, data32);
                    read_data = data32;
                    break;
                }

                case 8: {
                    mem_read_64(Data, address, read_data);
                    break;
                }

                default: {
                    trap();
                    break;
                }
                }

                return read_data;
            "#
            .into(),
        },
        HelperFunction {
            name: "vector_subrange_A".into(),
            parameters: "uint64 value, uint8 start, uint8 end".into(),
            return_type: "uint64".into(),
            body: r#"
                return (value >> start) & ((1 << (end - start + 1)) - 1);
            "#
            .into(),
        },
        HelperFunction {
            name: "u__raw_GetSlice_int".into(),
            parameters: "uint64 len, uint64 n, uint64 start".into(),
            return_type: "uint64".into(),
            body: r#"
                if (len == 64) {
                    return n;
                } else {
                    return (n >> start) & ((1 << len) - 1);
                }
            "#
            .into(),
        },
        HelperFunction {
            name: "slice".into(),
            parameters: "uint64 n, uint64 start, uint64 len".into(),
            return_type: "uint64".into(),
            body: r#"
                return u__raw_GetSlice_int(len, n, start);
            "#
            .into(),
        },
        HelperFunction {
            name: "UsingAArch32".into(),
            parameters: "".into(),
            return_type: "uint8".into(),
            body: r#"
                return 0;
            "#
            .into(),
        },
        HelperFunction {
            name: "AArch64_BranchAddr".into(),
            parameters: "uint64 vaddress".into(),
            return_type: "uint64".into(),
            body: r#"
                return vaddress;
            "#
            .into(),
        },
        HelperFunction {
            name: "Hint_Branch".into(),
            parameters: "uint32 typ".into(),
            return_type: "void".into(),
            body: r#"
                return;
            "#
            .into(),
        },
        HelperFunction {
            name: "u__PostDecode".into(),
            parameters: "".into(),
            return_type: "void".into(),
            body: r#"
                return;
            "#
            .into(),
        },
        HelperFunction {
            name: "HaveMTEExt".into(),
            parameters: "".into(),
            return_type: "uint8".into(),
            body: r#"
                return 0;
            "#
            .into(),
        },
        HelperFunction {
            name: "Prefetch".into(),
            parameters: "uint64 address, uint8 prfop".into(),
            return_type: "void".into(),
            body: r#"
                return;
            "#
            .into(),
        },
        HelperFunction {
            name: "EndOfInstruction".into(),
            parameters: "".into(),
            return_type: "void".into(),
            body: r#"
                trap();
                return;
            "#
            .into(),
        },
        HelperFunction {
            name: "CheckSPAlignment".into(),
            parameters: "".into(),
            return_type: "void".into(),
            body: r#"
                return;
            "#
            .into(),
        },
        HelperFunction {
            name: "AArch64_SetLSInstructionSyndrome".into(),
            parameters:
                "uint64 size, uint8 sign_extend, uint64 Rt, uint8 sixty_four, uint8 acq_rel".into(),
            return_type: "void".into(),
            body: r#"
                return;
            "#
            .into(),
        },
        HelperFunction {
            name: "system_hints".into(),
            parameters: "uint32 op".into(),
            return_type: "void".into(),
            body: r#"
                return;
            "#
            .into(),
        },
        HelperFunction {
            name: "replicate_bits".into(),
            parameters: "uint64 value, uint64 size, uint64 count".into(),
            return_type: "uint64".into(),
            body: "return 0;".into(),
        },
        HelperFunction {
            name: "HighestSetBit".into(),
            parameters: "uint64 value".into(),
            return_type: "sint64".into(),
            body: r#"
                for (sint64 i = 63; i >= 0; i--) {
                    uint64 bit = (value >> i) & 1;
                    if (bit == 1)
                    {
                        return i;
                    }
                }

                return -1;
            "#
            .into(),
        },
        HelperFunction {
            name: "Ones".into(),
            parameters: "uint64 n".into(),
            return_type: "uint64".into(),
            body: r#"
                uint64 result = 0;

                for (uint8 i = 0; i < n; i++) {
                    result = result << 1 | 1;
                }

                return result;
            "#
            .into(),
        },
        HelperFunction {
            name: "AArch64_CallSupervisor".into(),
            parameters: "uint16 imm".into(),
            return_type: "void".into(),
            body: r#"
                take_exception(3, imm);
		        //write_register(PC_target, read_pc() + 4);
                return;
            "#
            .into(),
        },
        HelperFunction {
            name: "BranchTargetCheck".into(),
            parameters: "".into(),
            return_type: "void".into(),
            body: r#"
                return;
            "#
            .into(),
        },
        HelperFunction {
            name: "EL2Enabled".into(),
            parameters: "".into(),
            return_type: "uint8".into(),
            body: r#"
                return 0;
            "#
            .into(),
        },
    ];

    HashMap::from_iter(
        fns.into_iter()
            .map(|f| (InternedString::from(f.name.clone()), f)),
    )
});

/// Generates GenC helper functions from all functions in a BOOM AST
pub fn generate_fns(
    ast: Rc<RefCell<Ast>>,
    initial_fns: Vec<InternedString>,
) -> Vec<HelperFunction> {
    let mut remaining_fns = initial_fns;
    let mut generated_fns = PREGENERATED_FNS.clone();

    while let Some(ident) = remaining_fns.pop() {
        // skip if already generated
        if generated_fns.contains_key(&ident) {
            continue;
        }

        let ast = ast.borrow();
        let Some(definition) = ast.functions.get(&ident) else {
            log::trace!("cannot generate GenC for unknown function {ident:?}");
            continue;
        };

        log::trace!("generating {ident}");

        #[allow(unstable_name_collisions)]
        let generated = HelperFunction {
            name: ident.to_string(),

            parameters: definition
                .signature
                .parameters
                .borrow()
                .iter()
                .map(Emit::emit_string)
                .join(", "),

            return_type: definition.signature.return_type.emit_string(),

            body: generate_fn_body(definition.entry_block.clone()),
        };

        generated_fns.insert(ident, generated);

        remaining_fns.extend(
            definition
                .entry_block
                .get_functions()
                // ignore builtin functions
                .difference(&BUILTIN_FNS),
        );
    }

    generated_fns.into_values().collect()
}

/// Generates constants from all enum variants
pub fn generate_enums(ast: Rc<RefCell<Ast>>) -> HashMap<InternedString, (genc::Typ, u64)> {
    ast.borrow()
        .definitions
        .iter()
        .filter_map(|def| {
            if let Definition::Enum { name, variants } = def {
                Some((name, variants))
            } else {
                None
            }
        })
        .flat_map(|(name, variants)| variants.iter().map(|variant| enum_mangle(*name, *variant)))
        .zip(0..)
        .map(|(k, v)| (k, (genc::Typ::Uint32, v)))
        .collect()
}

/// Mangled enum variant constant name generator
pub fn enum_mangle(_name: InternedString, variant: InternedString) -> InternedString {
    format!("{variant}").into()
}

fn generate_fn_body(entry_block: ControlFlowBlock) -> String {
    enum StackItem {
        Block(ControlFlowBlock),
        Else,
        End,
    }

    let mut stack = vec![StackItem::Block(entry_block)];

    let mut buf = String::new();
    let mut indent = Indent::new("    ");

    let mut sink_block_stack = vec![];

    // if a block is unconditional, emit the statements and go to the next block
    // if a block is conditional, emit an if, else branch, where the if and else
    // blocks are indented one more

    while let Some(item) = stack.pop() {
        let block = match item {
            StackItem::Block(block) => block,
            StackItem::Else => {
                indent.dec();
                buf += indent.get();
                buf += "} else {\n";
                indent.inc();
                continue;
            }
            StackItem::End { .. } => {
                indent.dec();
                buf += indent.get();
                buf += "}\n";
                continue;
            }
        };

        log::trace!("at block {block}");

        if let Some((sink, has_else)) = sink_block_stack.last().cloned() {
            if block == sink {
                log::trace!("is sink block, has_else: {has_else}");
                sink_block_stack.pop();

                if has_else {
                    sink_block_stack.push((sink, false));
                    stack.push(StackItem::Else);
                    continue;
                } else {
                    stack.push(StackItem::Block(sink));
                    stack.push(StackItem::End);
                    continue;
                }
            }
        }

        // write current block statements to buf here
        block.statements().iter().for_each(|stmt| {
            if let Statement::TypeDeclaration { typ, .. } = &*stmt.borrow() {
                buf += indent.get();
                writeln!(buf, "// {typ:?}").unwrap();
            }

            if let Statement::Copy { expression, value } = &*stmt.borrow() {
                buf += indent.get();
                writeln!(buf, "// {expression:?} {value:?}").unwrap();
            }

            buf += indent.get();
            stmt.emit(&mut buf).unwrap();
            buf += "\n";
        });

        match block.terminator() {
            Terminator::Return(value) => {
                buf += indent.get();
                buf += "return";

                if let Some(value) = value {
                    buf += " ";
                    Rc::new(RefCell::new(value)).emit(&mut buf).unwrap();
                }

                buf += ";\n";
            }
            Terminator::Unconditional { target } => {
                stack.push(StackItem::Block(target));
            }
            Terminator::Conditional {
                condition,
                target,
                fallthrough,
            } => {
                buf += indent.get();
                buf += "if (";
                Rc::new(RefCell::new(condition)).emit(&mut buf).unwrap();
                buf += ") {\n";
                indent.inc();

                // set up stack for processing the rest of the if statement

                // find all children of target and fallthrough
                // find closest child of both target and fallthrough (that all paths converge
                // to?) set child as checkpoint, emit both up to that point
                // emit child (and its children) as normal

                let block = find_common_block(target.clone(), fallthrough.clone())
                    .expect("should always find a common block after processing BOOM");
                log::trace!("found common block {block}");

                if target == block {
                    todo!()
                    // emit nothing in if-branch, and then emit fallthrough in
                    // else-branch, then continue
                } else if fallthrough == block {
                    sink_block_stack.push((block, false));
                    stack.extend([StackItem::Block(target)]);
                } else {
                    sink_block_stack.push((block, true));
                    stack.extend([StackItem::Block(fallthrough), StackItem::Block(target)]);
                }
            }
        }
    }

    buf
}

/// Determines whether the supplied function is a branch instruction
pub fn contains_write_pc(ast: Rc<RefCell<Ast>>, function_name: InternedString) -> bool {
    let borrow = ast.borrow();
    let Some(fn_def) = borrow.functions.get(&function_name) else {
        return false;
    };

    struct PcWritefinder {
        ast: Rc<RefCell<Ast>>,
        writes_pc: bool,
    }

    impl Visitor for PcWritefinder {
        fn visit_statement(&mut self, node: Rc<RefCell<Statement>>) {
            {
                if let Statement::FunctionCall { name, .. } = &*node.borrow() {
                    // TODO remove this special casing
                    if name.as_ref() == "AArch64_CallSupervisor"
                        || contains_write_pc(self.ast.clone(), *name)
                    {
                        self.writes_pc = true;
                    }
                }
            }

            node.borrow().walk(self);
        }

        fn visit_value(&mut self, node: Rc<RefCell<crate::boom::Value>>) {
            if let crate::boom::Value::Identifier(ident) = &*node.borrow() {
                if ident.as_ref() == "reg_PC_target" {
                    self.writes_pc = true;
                }
            }
        }
    }

    let mut finder = PcWritefinder {
        ast: ast.clone(),
        writes_pc: false,
    };

    finder.visit_function_definition(fn_def);

    finder.writes_pc
}
