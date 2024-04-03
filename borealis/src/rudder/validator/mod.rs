use {
    super::{Block, Function, Statement},
    crate::rudder::{ConstantValue, Context, PrimitiveTypeClass, StatementKind, Type},
    std::fmt::Display,
};

pub enum Severity {
    Error,
    Warning,
    Note,
}

pub enum Scope {
    FunctionLevel(Function),
    BlockLevel(Function, Block),
    StatementLevel(Function, Block, Statement),
}

pub struct ValidationMessage(Severity, Scope, String);

impl Display for ValidationMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let severity = match self.0 {
            Severity::Error => "ERROR",
            Severity::Warning => "WARNING",
            Severity::Note => "NOTE",
        };

        let scope = match &self.1 {
            Scope::FunctionLevel(f) => format!("{}", f.name()),
            Scope::BlockLevel(f, b) => format!("{} {}", f.name(), b.name()),
            Scope::StatementLevel(f, b, s) => format!("{} {} {}", f.name(), b.name(), s.name()),
        };

        write!(f, "{severity}: {scope}: {}", self.2)
    }
}

impl ValidationMessage {
    pub fn stmt_msg<T: ToString>(
        f: &Function,
        b: &Block,
        s: &Statement,
        v: Severity,
        m: T,
    ) -> Self {
        Self(
            v,
            Scope::StatementLevel(f.clone(), b.clone(), s.clone()),
            m.to_string(),
        )
    }

    pub fn stmt_warn<T: ToString>(f: &Function, b: &Block, s: &Statement, m: T) -> Self {
        Self::stmt_msg(f, b, s, Severity::Warning, m)
    }

    pub fn stmt_err<T: ToString>(f: &Function, b: &Block, s: &Statement, m: T) -> Self {
        Self::stmt_msg(f, b, s, Severity::Error, m)
    }
}

pub fn validate(ctx: &Context) -> Vec<ValidationMessage> {
    let messages = [check_constant_value_types(ctx), check_operand_types(ctx)];

    messages.into_iter().flatten().collect()
}

fn check_constant_value_types(ctx: &Context) -> Vec<ValidationMessage> {
    let mut messages = Vec::new();

    for (_, f) in ctx.get_functions() {
        for block in f.entry_block().iter() {
            for stmt in block.statements() {
                if let StatementKind::Constant { typ, value } = stmt.kind() {
                    let msg = match value {
                        ConstantValue::UnsignedInteger(_) => match &*typ {
                            Type::Primitive(p) => match p.tc {
                                PrimitiveTypeClass::Void => Some(ValidationMessage::stmt_warn(&f, &block, &stmt, "cannot use void type class for unsigned integer constant")),
                                PrimitiveTypeClass::Unit => Some(ValidationMessage::stmt_warn(&f, &block, &stmt, "cannot use unit type class for unsigned integer constant")),
                                PrimitiveTypeClass::UnsignedInteger => None,
                                PrimitiveTypeClass::SignedInteger => Some(ValidationMessage::stmt_warn(&f, &block, &stmt, "cannot use signed integer type class for unsigned integer constant")),
                                PrimitiveTypeClass::FloatingPoint => Some(ValidationMessage::stmt_warn(&f, &block, &stmt, "cannot use floating point type class for unsigned integer constant"))
                            },
                            Type::Composite(_) => {
                                Some(ValidationMessage::stmt_warn(&f, &block, &stmt, "cannot use composite type for unsigned integer constant"))
                            }
                            Type::Vector {
                               ..
                            } => Some(ValidationMessage::stmt_warn(&f, &block, &stmt, "cannot use vector type for unsigned integer constant")),
                            Type::Bundled {.. } => {
                                Some(ValidationMessage::stmt_warn(&f, &block, &stmt, "cannot use bundled type for unsigned integer constant"))
                            }
                        },
                        ConstantValue::SignedInteger(_) => match &*typ {
                            Type::Primitive(p) => match p.tc {
                                PrimitiveTypeClass::Void => Some(ValidationMessage::stmt_warn(&f, &block, &stmt, "cannot use void type class for signed integer constant")),
                                PrimitiveTypeClass::Unit => Some(ValidationMessage::stmt_warn(&f, &block, &stmt, "cannot use unit type class for signed integer constant")),
                                PrimitiveTypeClass::UnsignedInteger => Some(ValidationMessage::stmt_warn(&f, &block, &stmt, "cannot use unsigned integer type class for signed integer constant")),
                                PrimitiveTypeClass::SignedInteger => None,
                                PrimitiveTypeClass::FloatingPoint => Some(ValidationMessage::stmt_warn(&f, &block, &stmt, "cannot use floating point type class for signed integer constant"))
                            },
                            Type::Composite(_) => {
                                Some(ValidationMessage::stmt_warn(&f, &block, &stmt, "cannot use composite type for signed integer constant"))
                            }
                            Type::Vector {
                                ..
                            } => Some(ValidationMessage::stmt_warn(&f, &block, &stmt, "cannot use vector type for signed integer constant")),
                            Type::Bundled { .. } => {
                                Some(ValidationMessage::stmt_warn(&f, &block, &stmt, "cannot use bundled type for signed integer constant"))
                            }
                        }
                        ConstantValue::FloatingPoint(_) => match &*typ {
                            Type::Primitive(p) => match p.tc {
                                PrimitiveTypeClass::Void => Some(ValidationMessage::stmt_warn(&f, &block, &stmt, "cannot use void type class for floating point constant")),
                                PrimitiveTypeClass::Unit => Some(ValidationMessage::stmt_warn(&f, &block, &stmt, "cannot use unit type class for floating point constant")),
                                PrimitiveTypeClass::UnsignedInteger => Some(ValidationMessage::stmt_warn(&f, &block, &stmt, "cannot use unsigned integer type class for floating point constant")),
                                PrimitiveTypeClass::SignedInteger => Some(ValidationMessage::stmt_warn(&f, &block, &stmt, "cannot use signed integer type class for floating point constant")),
                                PrimitiveTypeClass::FloatingPoint => None
                            },
                            Type::Composite(_) => {
                                Some(ValidationMessage::stmt_warn(&f, &block, &stmt, "cannot use composite type for floating point constant"))
                            }
                            Type::Vector {
                             ..
                            } => Some(ValidationMessage::stmt_warn(&f, &block, &stmt, "cannot use vector type for floating point constant")),
                            Type::Bundled { .. } => {
                                Some(ValidationMessage::stmt_warn(&f, &block, &stmt, "cannot use bundled type for floating point constant"))
                            }
                        }
                        ConstantValue::Unit => match &*typ {
                            Type::Primitive(p) => match p.tc {
                                PrimitiveTypeClass::Void => Some(ValidationMessage::stmt_warn(&f, &block, &stmt, "cannot use void type class for unit constant")),
                                PrimitiveTypeClass::Unit => None,
                                PrimitiveTypeClass::UnsignedInteger => Some(ValidationMessage::stmt_warn(&f, &block, &stmt, "cannot use unsigned integer type class for unit constant")),
                                PrimitiveTypeClass::SignedInteger => Some(ValidationMessage::stmt_warn(&f, &block, &stmt, "cannot use signed integer type class for unit constant")),
                                PrimitiveTypeClass::FloatingPoint => Some(ValidationMessage::stmt_warn(&f, &block, &stmt, "cannot use floating point type class for unit constant")),
                            },
                            Type::Composite(_) => {
                                Some(ValidationMessage::stmt_warn(&f, &block, &stmt, "cannot use composite type for unit constant"))
                            }
                            Type::Vector {
                              ..
                            } => Some(ValidationMessage::stmt_warn(&f, &block, &stmt, "cannot use vector type for unit constant")),
                            Type::Bundled {..} => {
                                Some(ValidationMessage::stmt_warn(&f, &block, &stmt, "cannot use bundled type for unit constant"))
                            }
                        }
                    };

                    if let Some(msg) = msg {
                        messages.push(msg);
                    }
                }
            }
        }
    }

    messages
}

fn check_operand_types(ctx: &Context) -> Vec<ValidationMessage> {
    let mut messages = Vec::new();

    for (_, f) in ctx.get_functions() {
        for b in f.entry_block().iter() {
            for s in b.statements() {
                match s.kind() {
                    StatementKind::BinaryOperation { lhs, rhs, .. } => {
                        if !lhs.typ().is_compatible_with(&*rhs.typ()) {
                            messages.push(ValidationMessage::stmt_err(
                                &f,
                                &b,
                                &s,
                                "incompatible operand types in binary operation",
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    messages
}
