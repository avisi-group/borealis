//! GenC function generation from BOOM

use {
    crate::{
        boom::{
            control_flow::{ControlFlowBlock, Terminator},
            Ast,
        },
        codegen::emit::Emit,
        genc_model::HelperFunction,
    },
    common::intern::InternedString,
    itertools::Itertools,
    std::{cell::RefCell, collections::HashMap, rc::Rc},
};

/// Generates GenC helper functions from all functions in a BOOM AST
pub fn generate_fns(
    ast: Rc<RefCell<Ast>>,
    initial_fns: Vec<InternedString>,
) -> Vec<HelperFunction> {
    let mut remaining_fns = initial_fns;
    let mut generated_fns = HashMap::new();

    while let Some(ident) = remaining_fns.pop() {
        // skip if already generated
        if generated_fns.contains_key(&ident) {
            continue;
        }

        let ast = ast.borrow();
        let definition = ast
            .functions
            .get(&ident)
            .unwrap_or_else(|| panic!("cannot generate GenC for unknown function {ident:?}"));

        #[allow(unstable_name_collisions)]
        let generated = HelperFunction {
            name: ident.to_string(),

            parameters: definition
                .signature
                .parameters
                .iter()
                .map(Emit::emit_string)
                .join(", "),

            return_type: definition.signature.return_type.emit_string(),

            body: generate_fn_body(definition.entry_block.clone()),
        };

        generated_fns.insert(ident, generated);

        remaining_fns.extend(definition.entry_block.get_functions());
    }

    generated_fns.into_values().collect()
}

#[derive(Debug)]
struct Indent {
    buf: String,
    num: usize,
    whitespace: &'static str,
}

impl Indent {
    pub fn new(whitespace: &'static str) -> Self {
        Self {
            buf: whitespace.to_owned(),
            num: 1,
            whitespace,
        }
    }

    pub fn inc(&mut self) {
        self.num += 1;

        while self.buf.len() < self.num * self.whitespace.len() {
            self.buf += self.whitespace;
            assert!(self.buf.len() == self.num * self.whitespace.len());
        }
    }

    pub fn dec(&mut self) {
        self.num -= 1;
    }

    pub fn get(&self) -> &str {
        &self.buf[..self.num * self.whitespace.len()]
    }
}

fn generate_fn_body(entry_block: ControlFlowBlock) -> String {
    enum StackItem {
        Block(ControlFlowBlock),
        Else,
        EndElse,
    }

    let mut buf = String::new();
    let mut stack = vec![StackItem::Block(entry_block)];
    let mut indent = Indent::new("    ");

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
            StackItem::EndElse => {
                indent.dec();
                buf += indent.get();
                buf += "}\n";
                continue;
            }
        };

        //TODO: write current block statements to buf here
        block.statements().iter().for_each(|stmt| {
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
                    value.emit(&mut buf).unwrap();
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
                condition.emit(&mut buf).unwrap();
                buf += ") {\n";
                indent.inc();

                // set up stack for processing the rest of the if statement

                stack.extend([
                    StackItem::EndElse,
                    StackItem::Block(fallthrough),
                    StackItem::Else,
                    StackItem::Block(target),
                ]);
            }
        }
    }

    buf
}
