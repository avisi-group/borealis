use {
    crate::boom::{self, NamedType},
    common::intern::InternedString,
    sail::{jib_ast, sail_ast},
    std::{
        borrow::Borrow,
        collections::{HashMap, HashSet, LinkedList},
    },
};

#[derive(Debug)]
pub struct BoomEmitter {
    generated_ast: boom::Ast,
    // stored temporarily as spec and function definitions are separate
    function_types: HashMap<InternedString, (Vec<boom::Type>, boom::Type)>,
}

impl BoomEmitter {
    pub fn new() -> Self {
        Self {
            generated_ast: boom::Ast {
                definitions: vec![],
                labels: HashSet::new(),
            },
            function_types: HashMap::new(),
        }
    }

    pub fn process<'a, I: IntoIterator<Item = &'a jib_ast::Definition>>(&mut self, definitions: I) {
        definitions
            .into_iter()
            .for_each(|def| self.process_definition(def));
    }

    pub fn finish(self) -> boom::Ast {
        todo!()
    }

    fn process_definition(&mut self, definition: &jib_ast::Definition) {
        match definition {
            jib_ast::Definition::RegDec(ident, typ, _) => {
                let def = boom::Definition::Register {
                    name: ident.as_interned(),
                    typ: convert_type(typ),
                };
                self.generated_ast.definitions.push(def);
            }
            jib_ast::Definition::Type(type_def) => {
                let def = match type_def {
                    jib_ast::TypeDefinition::Enum(name, variants) => boom::Definition::Enum {
                        name: name.as_interned(),
                        variants: convert_variants(variants),
                    },
                    jib_ast::TypeDefinition::Struct(name, fields) => boom::Definition::Struct {
                        name: name.as_interned(),
                        fields: convert_fields(fields),
                    },
                    jib_ast::TypeDefinition::Variant(name, fields) => boom::Definition::Union {
                        name: name.as_interned(),
                        fields: convert_fields(fields),
                    },
                };
                self.generated_ast.definitions.push(def);
            }
            jib_ast::Definition::Let(_, bindings, body) => {
                self.generated_ast.definitions.push(boom::Definition::Let {
                    bindings: bindings
                        .iter()
                        .map(|(ident, typ)| NamedType {
                            name: ident.as_interned(),
                            typ: convert_type(typ),
                        })
                        .collect(),
                    body: convert_body(body),
                });
            }
            jib_ast::Definition::Spec(id, _, parameters, out) => {
                self.function_types.insert(
                    id.as_interned(),
                    (
                        parameters.iter().map(convert_type).collect(),
                        convert_type(out),
                    ),
                );
            }
            jib_ast::Definition::Fundef(name, _, arguments, body) => {
                let (parameter_types, return_type) =
                    self.function_types.remove(&name.as_interned()).unwrap();

                let parameters = arguments
                    .iter()
                    .map(sail_ast::Identifier::as_interned)
                    .zip(parameter_types)
                    .map(|(name, typ)| NamedType { name, typ })
                    .collect();

                self.generated_ast
                    .definitions
                    .push(boom::Definition::Function {
                        name: name.as_interned(),
                        parameters,
                        return_type,
                        body: convert_body(body),
                    });
            }
            jib_ast::Definition::Startup(_, _) => todo!(),
            jib_ast::Definition::Finish(_, _) => todo!(),
            &jib_ast::Definition::Pragma(key, value) => self
                .generated_ast
                .definitions
                .push(boom::Definition::Pragma { key, value }),
        };
    }
}

fn convert_type<T: Borrow<jib_ast::Type>>(typ: T) -> boom::Type {
    match typ.borrow() {
        jib_ast::Type::Lint => boom::Type::Lint,
        jib_ast::Type::Fint(i) => boom::Type::Fint(*i),
        jib_ast::Type::Constant(_) => todo!(),
        jib_ast::Type::Lbits(b) => boom::Type::Lbits(*b),
        jib_ast::Type::Sbits(_, _) => todo!(),
        jib_ast::Type::Fbits(i, b) => boom::Type::Fbits(*i, *b),
        jib_ast::Type::Unit => boom::Type::Unit,
        jib_ast::Type::Bool => boom::Type::Bool,
        jib_ast::Type::Bit => boom::Type::Bit,
        jib_ast::Type::String => boom::Type::String,
        jib_ast::Type::Real => boom::Type::Real,
        jib_ast::Type::Float(_) => todo!(),
        jib_ast::Type::RoundingMode => todo!(),
        jib_ast::Type::Tup(_) => todo!(),
        jib_ast::Type::Enum(name, variants) => boom::Type::Enum {
            name: name.as_interned(),
            variants: convert_variants(&variants),
        },
        jib_ast::Type::Struct(name, fields) => boom::Type::Struct {
            name: name.as_interned(),
            fields: convert_fields(fields),
        },
        jib_ast::Type::Variant(name, fields) => boom::Type::Union {
            name: name.as_interned(),
            fields: convert_fields(fields),
        },
        jib_ast::Type::Fvector(length, _, typ) => boom::Type::FVector {
            length: *length,
            element_type: Box::new(convert_type(&**typ)),
        },
        jib_ast::Type::Vector(_, typ) => boom::Type::Vector {
            element_type: Box::new(convert_type(&**typ)),
        },
        jib_ast::Type::List(typ) => boom::Type::List {
            element_type: Box::new(convert_type(&**typ)),
        },
        jib_ast::Type::Ref(typ) => boom::Type::Reference(Box::new(convert_type(&**typ))),
        jib_ast::Type::Poly(_) => todo!(),
    }
}

fn convert_body(instructions: &LinkedList<jib_ast::Instruction>) -> Vec<boom::Statement> {
    instructions
        .iter()
        .map(|instr| convert_statement(&instr.inner))
        .collect()
}

fn convert_statement(statement: &jib_ast::InstructionAux) -> boom::Statement {
    match statement {
        jib_ast::InstructionAux::Decl(typ, name) => boom::Statement::TypeDeclaration {
            name: convert_name(name),
            typ: convert_type(typ),
        },
        jib_ast::InstructionAux::Init(_, _, _) => todo!(),
        jib_ast::InstructionAux::Jump(_, _) => todo!(),
        jib_ast::InstructionAux::Goto(_) => todo!(),
        jib_ast::InstructionAux::Label(s) => boom::Statement::Label(*s),
        jib_ast::InstructionAux::Funcall(expression, _, (name, _), args) => {
            boom::Statement::FunctionCall {
                expression: convert_expression(expression),
                name: name.as_interned(),
                arguments: args.iter().map(convert_value).collect(),
            }
        }
        jib_ast::InstructionAux::Copy(expression, value) => boom::Statement::Copy {
            expression: convert_expression(expression),
            value: convert_value(value),
        },
        jib_ast::InstructionAux::Clear(_, name) => boom::Statement::Clear {
            identifier: convert_name(name),
        },
        jib_ast::InstructionAux::Undefined(_) => boom::Statement::Undefined,
        jib_ast::InstructionAux::Exit(_) => todo!(),
        jib_ast::InstructionAux::End(name) => boom::Statement::End(convert_name(name)),
        jib_ast::InstructionAux::If(condition, if_body, else_body, _) => boom::Statement::If {
            condition: convert_value(condition),
            if_body: convert_body(if_body),
            else_body: convert_body(else_body),
        },
        jib_ast::InstructionAux::Block(instrs) => boom::Statement::Block {
            body: convert_body(instrs),
        },
        jib_ast::InstructionAux::TryBlock(_) => todo!(),
        jib_ast::InstructionAux::Throw(_) => todo!(),
        jib_ast::InstructionAux::Comment(_) => todo!(),
        jib_ast::InstructionAux::Raw(_) => todo!(),
        jib_ast::InstructionAux::Return(_) => todo!(),
        jib_ast::InstructionAux::Reset(_, _) => todo!(),
        jib_ast::InstructionAux::Reinit(_, _, _) => todo!(),
    }
}

fn convert_name(name: &jib_ast::Name) -> InternedString {
    match name {
        jib_ast::Name::Name(ident, _) => ident.as_interned(),
        jib_ast::Name::Global(_, _) => todo!(),
        jib_ast::Name::HaveException(_) => todo!(),
        jib_ast::Name::CurrentException(_) => todo!(),
        jib_ast::Name::ThrowLocation(_) => todo!(),
        jib_ast::Name::Return(_) => InternedString::from_static("return"),
    }
}

fn convert_expression(expression: &jib_ast::Expression) -> boom::Expression {
    match expression {
        jib_ast::Expression::Id(name, _) => boom::Expression::Identifier(convert_name(name)),
        jib_ast::Expression::Rmw(_, _, _) => todo!(),
        jib_ast::Expression::Field(_, _) => todo!(),
        jib_ast::Expression::Addr(_) => todo!(),
        jib_ast::Expression::Tuple(_, _) => todo!(),
        jib_ast::Expression::Void => todo!(),
    }
}

fn convert_value(value: &jib_ast::Value) -> boom::Value {
    match value {
        jib_ast::Value::Id(name, _) => boom::Value::Identifier(convert_name(name)),
        jib_ast::Value::Lit(vl, _) => boom::Value::Literal(convert_literal(vl)),
        jib_ast::Value::Tuple(_, _) => todo!(),
        jib_ast::Value::Struct(_, _) => todo!(),
        jib_ast::Value::CtorKind(_, _, _, _) => todo!(),
        jib_ast::Value::CtorUnwrap(_, _, _) => todo!(),
        jib_ast::Value::TupleMember(_, _, _) => todo!(),
        jib_ast::Value::Call(_, _) => todo!(),
        jib_ast::Value::Field(_, _) => todo!(),
    }
}

fn convert_literal(literal: &jib_ast::Vl) -> boom::Literal {
    match literal {
        jib_ast::Vl::Bits(_, _) => todo!(),
        jib_ast::Vl::Bit(_) => todo!(),
        jib_ast::Vl::Bool(_) => todo!(),
        jib_ast::Vl::Unit => todo!(),
        jib_ast::Vl::Int(bigint) => boom::Literal::Int(bigint.0.clone()),
        jib_ast::Vl::String(_) => todo!(),
        jib_ast::Vl::Real(_) => todo!(),
        jib_ast::Vl::EmptyList => todo!(),
        jib_ast::Vl::Enum(_) => todo!(),
        jib_ast::Vl::Ref(_) => todo!(),
        jib_ast::Vl::Undefined => todo!(),
    }
}

fn convert_op(op: &jib_ast::Op) -> boom::Op {
    match op {
        jib_ast::Op::Bnot => todo!(),
        jib_ast::Op::Bor => todo!(),
        jib_ast::Op::Band => todo!(),
        jib_ast::Op::ListHead => todo!(),
        jib_ast::Op::ListTail => todo!(),
        jib_ast::Op::Eq => todo!(),
        jib_ast::Op::Neq => todo!(),
        jib_ast::Op::Ilt => todo!(),
        jib_ast::Op::Ilteq => todo!(),
        jib_ast::Op::Igt => todo!(),
        jib_ast::Op::Igteq => todo!(),
        jib_ast::Op::Iadd => todo!(),
        jib_ast::Op::Isub => todo!(),
        jib_ast::Op::Unsigned(_) => todo!(),
        jib_ast::Op::Signed(_) => todo!(),
        jib_ast::Op::Bvnot => todo!(),
        jib_ast::Op::Bvor => todo!(),
        jib_ast::Op::Bvand => todo!(),
        jib_ast::Op::Bvxor => todo!(),
        jib_ast::Op::Bvadd => todo!(),
        jib_ast::Op::Bvsub => todo!(),
        jib_ast::Op::Bvaccess => todo!(),
        jib_ast::Op::Concat => todo!(),
        jib_ast::Op::ZeroExtend(_) => todo!(),
        jib_ast::Op::SignExtend(_) => todo!(),
        jib_ast::Op::Slice(_) => todo!(),
        jib_ast::Op::Sslice(_) => todo!(),
        jib_ast::Op::SetSlice => todo!(),
        jib_ast::Op::Replicate(_) => todo!(),
    }
}

/// Converts variants of an enum from JIB to BOOM
fn convert_variants(variants: &LinkedList<sail_ast::Identifier>) -> Vec<InternedString> {
    variants
        .iter()
        .map(sail_ast::Identifier::as_interned)
        .collect()
}

/// Converts fields of a struct or union from JIB to BOOM
///
/// Generics are required to be able to convert from `LinkedList<((Identifier, LinkedList<Type>), Box<Type>)>` *and* `LinkedList<((Identifier, LinkedList<Type>), Type)>`.
fn convert_fields<
    'a,
    TYPE: Borrow<jib_ast::Type> + 'a,
    ITER: IntoIterator<Item = &'a ((sail_ast::Identifier, LinkedList<jib_ast::Type>), TYPE)>,
>(
    fields: ITER,
) -> Vec<NamedType> {
    fields
        .into_iter()
        .map(|((name, _), typ)| NamedType {
            name: name.as_interned(),
            typ: convert_type(typ.borrow()),
        })
        .collect()
}
