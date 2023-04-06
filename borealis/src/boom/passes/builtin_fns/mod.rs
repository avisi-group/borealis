//! Pass for implementing builtin (as in, provided by the Sail compiler) functions in BOOM

use {
    crate::boom::{
        passes::Pass,
        visitor::{Visitor, Walkable},
        Ast, Statement,
    },
    regex::Regex,
    std::{cell::RefCell, rc::Rc},
};

pub mod functions;
pub mod generic_functions;

pub struct AddBuiltinFns {
    ast: Rc<RefCell<Ast>>,
    generic_fn_regex: Regex,
}

impl AddBuiltinFns {
    pub fn new_boxed(ast: Rc<RefCell<Ast>>) -> Box<dyn Pass> {
        Box::new(Self {
            ast,
            generic_fn_regex: Regex::new("([a-z_]+)<(.+)>").unwrap(),
        })
    }
}

impl Pass for AddBuiltinFns {
    fn run(&mut self, ast: Rc<RefCell<Ast>>) {
        // walk AST, inspecting each function call
        // if the function call references an already-defined function, ignore
        // otherwise, lookup function in functions and execute behaviour (either in place modification or inserting new function definition)

        ast.borrow()
            .functions
            .values()
            .for_each(|def| self.visit_function_definition(def));
    }
}

impl Visitor for AddBuiltinFns {
    fn visit_statement(&mut self, node: Rc<RefCell<Statement>>) {
        // ignore statements that are not function calls
        let Statement::FunctionCall { name, .. } = *node.borrow() else {
            return;
        };

        // ignore if the function definition is already in the AST,
        if self.ast.borrow().functions.contains_key(&name) {
            return;
        }

        let name = name.to_string();

        match self.generic_fn_regex.captures(&name) {
            Some(captures) => {
                let name = captures.get(1).unwrap().as_str();
                let typ = captures.get(2).unwrap().as_str();

                // found generic function
                generic_functions::HANDLERS.get(name).unwrap_or_else(|| panic!(
                    "Generic function call \'{}<{}>\' found without definition or builtin function behaviour",
                    name, typ
                ))(&self.ast.borrow(), &*node.borrow(), typ)
            }
            None => {
                // found non-generic function
                functions::HANDLERS.get(&name).unwrap_or_else(|| {
                    panic!(
                        "Function call {:?} found without definition or builtin function behaviour",
                        name
                    )
                })(&self.ast.borrow(), &*node.borrow())
            }
        }

        node.borrow().walk(self);
    }
}
