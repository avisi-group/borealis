//! Destructures structs into their fields as local variables to work around the
//! lack of struct support in GenSim
//!
//!
//! ## Local structs
//!
//! Replaced with multiple definitions, one for each field. Assignments to local
//! structs are transformed into assingments to these local variables.
//!
//! ## Structs as arguments
//!
//! Function parameter split into multiple parameters, every callsite similarly
//! modified.
//!
//! ## Structs in return types
//!
//! New return type is void, references to each field added as parameters.
//! Callsite modified: new local vars for fields inserted, references passed to
//! call.
//!
//!
//! ## Notes
//!
//! To avoid ambiguity, the return struct fields are placed at the beginning of
//! the function parameters, and any parameters that are structs are replaced in
//! their original location
//!
//! fn (foo, bar, baz) becomes fn(return_a, return_b, foo, bar_a, bar_b, bar_c,
//! baz)

use {
    crate::boom::{
        control_flow::Terminator, passes::Pass, visitor::Visitor, Ast, Expression,
        FunctionDefinition, FunctionSignature, NamedType, NamedValue, Parameter, Statement, Type,
        Value,
    },
    common::{intern::InternedString, HashMap},
    std::{cell::RefCell, rc::Rc},
};

/// Destructures structs into their fields as local variables to work around the
/// lack of struct support in GenSim
#[derive(Debug, Default)]
pub struct DestructStructs;

impl DestructStructs {
    /// Create a new Pass object
    pub fn new_boxed() -> Box<dyn Pass> {
        Box::<Self>::default()
    }
}

impl Pass for DestructStructs {
    fn name(&self) -> &'static str {
        "DestructStructs"
    }

    fn reset(&mut self) {}

    fn run(&mut self, ast: Rc<RefCell<Ast>>) -> bool {
        handle_registers(&mut ast.borrow_mut().registers);

        ast.borrow()
            .functions
            .iter()
            .for_each(|(_, def)| destruct_structs(def));

        false
    }
}

fn destruct_structs(fn_def: &FunctionDefinition) {
    fix_params(&fn_def.signature);
    let return_fields = fix_return(fn_def);
    destruct_locals(fn_def, return_fields);
}

fn fix_params(fn_signature: &FunctionSignature) {
    let mut parameters = fn_signature.parameters.borrow_mut();
    *parameters = parameters
        .iter()
        .flat_map(|parameter| {
            if let Type::Struct {
                name: struct_name,
                fields,
            } = &*parameter.typ.borrow()
            {
                fields
                    .iter()
                    .map(
                        |NamedType {
                             name: field_name,
                             typ,
                         }| Parameter {
                            name: destructed_ident(*struct_name, *field_name),
                            typ: typ.clone(),
                            is_ref: false,
                        },
                    )
                    .collect()
            } else {
                vec![parameter.clone()]
            }
        })
        .collect();
}

fn fix_return(fn_def: &FunctionDefinition) -> Option<Vec<NamedType>> {
    // find function with struct as the return type
    let fields = {
        let Type::Struct { fields, .. } = &*fn_def.signature.return_type.borrow() else {
            return None;
        };
        fields.clone()
    };

    // replace with void return type
    fn_def.signature.return_type.replace(Type::Unit);

    // add struct fields to parameters
    fn_def.signature.parameters.borrow_mut().splice(
        0..0,
        fields
            .iter()
            .cloned()
            .map(|NamedType { name, typ }| Parameter {
                name: destructed_ident("return_value".into(), name),
                typ,
                is_ref: true,
            }),
    );

    // return void
    fn_def.entry_block.iter().for_each(|block| {
        if let Terminator::Return(_) = block.terminator() {
            block.set_terminator(Terminator::Return(None));
        }
    });

    // modification of copies into return value fields occurs in "destruct_locals"

    Some(fields)
}

/// split locally declared structs into individual variables
fn destruct_locals(fn_def: &FunctionDefinition, return_fields: Option<Vec<NamedType>>) {
    let mut structs = HashMap::default();

    if let Some(fields) = return_fields {
        structs.insert("return_value".into(), fields);
    }

    // go through each statement in the function
    // if the statement is a struct type declaration, remove it and replace with
    // type decls for each field of the struct
    fn_def.entry_block.iter().for_each(|block| {
        let statements = block.statements();

        let destructed = statements
            .into_iter()
            .flat_map(|statement| {
                let clone = statement.clone();

                match &*statement.borrow() {
                    // if a struct local var is declared, replace it with declarations for all its
                    // fields
                    Statement::TypeDeclaration {
                        name: variable_name,
                        typ,
                    } => {
                        let Type::Struct { fields, .. } = &*typ.borrow() else {
                            return vec![clone];
                        };

                        structs.insert(*variable_name, fields.clone());

                        fields
                            .iter()
                            .map(
                                |NamedType {
                                     name: field_name,
                                     typ,
                                 }| {
                                    Statement::TypeDeclaration {
                                        name: destructed_ident(*variable_name, *field_name),
                                        typ: typ.clone(),
                                    }
                                    .into()
                                },
                            )
                            .collect()
                    }
                    // if a struct is copied into a local variable, replace with several copies into
                    // each field
                    Statement::Copy { expression, value } => {
                        // if we are assigning to a field, replace with a copy where expression is
                        // destructed
                        if let Expression::Field { expression, field } = expression {
                            let Expression::Identifier(struc) = **expression else {
                                panic!();
                            };

                            return vec![Rc::new(RefCell::new(Statement::Copy {
                                expression: Expression::Identifier(destructed_ident(struc, *field)),
                                value: value.clone(),
                            }))];
                        }

                        // if we are copying *from* a field
                        {
                            let value_mut = &mut *value.borrow_mut();

                            if let Value::Field {
                                value: struc_val,
                                field_name,
                            } = value_mut
                            {
                                let Value::Identifier(struc) = *(struc_val.clone().borrow()) else {
                                    panic!();
                                };

                                *value_mut =
                                    Value::Identifier(destructed_ident(struc, *field_name));
                            }
                        }

                        // otherwise assigning to whole struct
                        let Expression::Identifier(dest) = expression else {
                            return vec![clone];
                        };

                        let Some(fields) = structs.get(dest) else {
                            return vec![clone];
                        };

                        // names of the fields to be copied into
                        let local_fields = fields
                            .iter()
                            .map(|NamedType { name, .. }| {
                                Expression::Identifier(destructed_ident(*dest, *name))
                            })
                            .collect::<Vec<_>>();

                        let values = match &*value.borrow() {
                            // if the value is an identifier, look up fields in structs map, and get
                            // list of values from that
                            Value::Identifier(ident) => {
                                let fields = structs
                                    .get(ident)
                                    .expect("attempting to assign non struct value identifier");

                                fields
                                    .iter()
                                    .map(|NamedType { name, .. }| {
                                        Value::Identifier(destructed_ident(*ident, *name))
                                    })
                                    .map(RefCell::new)
                                    .map(Rc::new)
                                    .collect::<Vec<_>>()
                            }
                            // if the value is a struct, use those fields
                            Value::Struct { fields, .. } => fields
                                .iter()
                                .map(|NamedValue { value, .. }| value)
                                .cloned()
                                .collect::<Vec<_>>(),
                            _ => todo!(),
                        };

                        local_fields
                            .into_iter()
                            .zip(values)
                            .map(|(expression, value)| Statement::Copy { expression, value })
                            .map(RefCell::new)
                            .map(Rc::new)
                            .collect()
                    }

                    // if we pass a struct into a function, instead pass a reference
                    Statement::FunctionCall {
                        expression,
                        name,
                        arguments,
                    } => {
                        let mut expression = expression.clone();
                        let mut arguments = arguments.clone();

                        // if return expression is in `structs`, remove it and add *reference*
                        // fields to arguments
                        if let Some(Expression::Identifier(dest)) = expression {
                            if let Some(fields) = structs.get(&dest) {
                                expression = None;

                                arguments = fields
                                    .iter()
                                    .map(|NamedType { name, .. }| destructed_ident(dest, *name))
                                    .map(|name| Rc::new(RefCell::new(Value::Identifier(name))))
                                    .chain(arguments)
                                    .collect::<Vec<_>>()
                            }
                        }

                        // if any arguments are in `structs`, replace with
                        // mangled field names
                        arguments = arguments
                            .iter()
                            .flat_map(|arg| {
                                let Value::Identifier(ident) = &*arg.borrow() else {
                                    return vec![arg.clone()];
                                };

                                let Some(fields) = structs.get(ident) else {
                                    return vec![arg.clone()];
                                };

                                fields
                                    .iter()
                                    .map(|NamedType { name, .. }| destructed_ident(*ident, *name))
                                    .map(|name| Rc::new(RefCell::new(Value::Identifier(name))))
                                    .collect()
                            })
                            .collect();

                        vec![Statement::FunctionCall {
                            expression,
                            name: *name,
                            arguments,
                        }
                        .into()]
                    }

                    _ => vec![clone],
                }
            })
            .collect();

        block.set_statements(destructed);
    });

    // transform all field values into local identifier values
    FieldVisitor.visit_function_definition(fn_def);

    // split struct copies into multiple field copies
}

struct FieldVisitor;

impl Visitor for FieldVisitor {
    fn visit_value(&mut self, node: Rc<RefCell<Value>>) {
        // if value is a field...
        let (ident, field) = {
            let Value::Field { value, field_name } = &*node.borrow() else {
                return;
            };
            let Value::Identifier(ident) = &*value.borrow() else {
                panic!("field access to non identifier")
            };

            if ident.as_ref() == "PSTATE" {
                return;
            }

            (*ident, *field_name)
        };

        //...replace it with the identifier of the corresponding local variable
        *node.borrow_mut() = Value::Identifier(destructed_ident(ident, field))
    }
}

fn destructed_ident(
    local_variable_name: InternedString,
    field_name: InternedString,
) -> InternedString {
    format!("{local_variable_name}_{field_name}").into()
}

fn handle_registers(registers: &mut HashMap<InternedString, Rc<RefCell<Type>>>) {
    let mut to_remove = vec![];
    let mut to_add = vec![];

    registers.iter().for_each(|(name, typ)| {
        if let Type::Struct { fields, .. } = &*typ.borrow() {
            to_remove.push(*name);
            to_add.extend(fields.iter().map(
                |NamedType {
                     name: field_name,
                     typ,
                 }| (destructed_ident(*name, *field_name), typ.clone()),
            ));
        }
    });

    for name in to_remove {
        registers.remove(&name).unwrap();
    }
    registers.extend(to_add);
}
