use {
    common::intern::InternedString,
    proc_macro2::TokenStream,
    quote::ToTokens,
    std::{
        cell::RefCell,
        collections::{HashMap, HashSet, LinkedList},
        fmt::Display,
        hash::{Hash, Hasher},
        rc::Rc,
    },
};

pub mod build;

#[derive(Clone)]
pub enum PrimitiveTypeClass {
    Void,
    UnsignedInteger,
    SignedInteger,
    FloatingPoint,
}

#[derive(Clone)]
pub struct PrimitiveType {
    tc: PrimitiveTypeClass,
    element_width_in_bits: usize,
    element_count: usize,
}

#[derive(Clone)]
pub enum Type {
    Primitive(PrimitiveType),
    Composite(Vec<Rc<Type>>),
}

macro_rules! type_def_helper {
    ($name: ident, $cls: ident, $width: expr) => {
        pub fn $name() -> Self {
            Self::new_primitive(PrimitiveTypeClass::$cls, $width)
        }
    };
}

impl Type {
    pub fn new_primitive(tc: PrimitiveTypeClass, element_width: usize) -> Self {
        Self::Primitive(PrimitiveType {
            tc,
            element_width_in_bits: element_width,
            element_count: 1,
        })
    }

    pub fn new_composite(fields: Vec<Rc<Type>>) -> Self {
        Self::Composite(fields)
    }

    pub fn void() -> Self {
        Self::Primitive(PrimitiveType {
            tc: PrimitiveTypeClass::Void,
            element_width_in_bits: 0,
            element_count: 0,
        })
    }

    type_def_helper!(u1, UnsignedInteger, 1);
    type_def_helper!(u8, UnsignedInteger, 8);
    type_def_helper!(u16, UnsignedInteger, 16);
    type_def_helper!(u32, UnsignedInteger, 32);
    type_def_helper!(u64, UnsignedInteger, 64);
    type_def_helper!(s8, SignedInteger, 8);
    type_def_helper!(s16, SignedInteger, 16);
    type_def_helper!(s32, SignedInteger, 32);
    type_def_helper!(s64, SignedInteger, 64);
    type_def_helper!(f32, FloatingPoint, 32);
    type_def_helper!(f64, FloatingPoint, 64);

    pub fn vectorize(self, element_count: usize) -> Result<Self, ()> {
        match self {
            Type::Primitive(p) => {
                if p.element_count > 1 {
                    Ok(Self::Primitive(PrimitiveType {
                        tc: p.tc,
                        element_width_in_bits: p.element_width_in_bits,
                        element_count,
                    }))
                } else {
                    Err(())
                }
            }
            Type::Composite(_) => Err(()),
        }
    }

    /*pub fn is_void(&self) -> bool {
        matches!(self.tc, TypeClass::Void)
    }

    pub fn is_integer(&self) -> bool {
        matches!(
            self.tc,
            TypeClass::UnsignedInteger | TypeClass::SignedInteger
        )
    }

    pub fn is_signed(&self) -> Option<bool> {
        match self.tc {
            TypeClass::SignedInteger => Some(true),
            TypeClass::UnsignedInteger => Some(false),
            _ => None,
        }
    }

    pub fn is_unsigned(&self) -> Option<bool> {
        match self.tc {
            TypeClass::SignedInteger => Some(false),
            TypeClass::UnsignedInteger => Some(true),
            _ => None,
        }
    }

    pub fn is_float(&self) -> bool {
        matches!(self.tc, TypeClass::FloatingPoint)
    }

    pub fn is_vector(&self) -> bool {
        self.element_count > 1
    }*/
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Type::Primitive(p) => match &p.tc {
                PrimitiveTypeClass::Void => write!(f, "void"),
                tc => {
                    if p.element_count > 1 {
                        write!(f, "v{}", p.element_count)?;
                    }

                    let prefix = match tc {
                        PrimitiveTypeClass::Void => unreachable!(),
                        PrimitiveTypeClass::UnsignedInteger => "u",
                        PrimitiveTypeClass::SignedInteger => "s",
                        PrimitiveTypeClass::FloatingPoint => "f",
                    };

                    write!(f, "{}{}", prefix, p.element_width_in_bits)
                }
            },
            Type::Composite(_) => write!(f, "struct"),
        }
    }
}

#[derive(Clone)]
pub enum ConstantValue {
    UnsignedInteger(usize),
    SignedInteger(isize),
    FloatingPoint(f64),
}

impl Display for ConstantValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConstantValue::UnsignedInteger(v) => write!(f, "{}", v),
            ConstantValue::SignedInteger(v) => write!(f, "{}", v),
            ConstantValue::FloatingPoint(v) => write!(f, "{}", v),
        }
    }
}

#[derive(Clone)]
pub struct Symbol {
    inner: Rc<RefCell<SymbolInner>>,
}

pub struct SymbolInner {
    tag: usize,
}

impl Symbol {
    pub fn name(&self) -> InternedString {
        "xxx".into()
    }
}

#[derive(Clone)]
pub enum BinaryOperationKind {
    Add,
    Sub,
    Multiply,
    Divide,
    Modulo,
}

#[derive(Clone)]
pub enum UnaryOperationKind {
    Complement,
}

#[derive(Clone)]
pub enum CastOperationKind {
    ZeroExtend,
    SignExtend,
    Truncate,
    Reinterpret,
    Convert,
    Broadcast,
}

#[derive(Clone)]
pub enum StatementKind {
    Constant {
        value: ConstantValue,
    },
    ReadVariable {
        symbol: Symbol,
    },
    WriteVariable {
        symbol: Symbol,
        value: Value,
    },
    ReadRegister {
        typ: Type,
        offset: Value,
    },
    WriteRegister {
        offset: Value,
        value: Value,
    },
    ReadMemory {
        typ: Type,
        offset: Value,
    },
    WriteMemory {
        offset: Value,
        value: Value,
    },
    BinaryOperation {
        kind: BinaryOperationKind,
        lhs: Value,
        rhs: Value,
    },
    UnaryOperation {
        kind: UnaryOperationKind,
        value: Value,
    },
    Call {
        target: Function,
        args: Vec<Value>,
    },
    Cast {
        kind: CastOperationKind,
        typ: Type,
        value: Value,
    },
    Jump {
        target: Block,
    },
    Branch {
        condition: Value,
        true_target: Block,
        false_target: Block,
    },
    PhiNode {
        members: Vec<(Block, Value)>,
    },
    Return {
        value: Option<Value>,
    },
    Select {
        condition: Value,
        true_value: Value,
        false_value: Value,
    },
}

impl Display for StatementKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            StatementKind::Constant { value } => write!(f, "const #{}", value),
            StatementKind::ReadVariable { symbol } => write!(f, "read-var {}", symbol.name()),
            StatementKind::WriteVariable { symbol, value } => {
                write!(f, "write-var {} {}", symbol.name(), value)
            }
            StatementKind::ReadRegister { typ, offset } => {
                write!(f, "read-reg {}:{}", offset, typ)
            }
            StatementKind::WriteRegister { offset, value } => {
                write!(f, "write-reg {} {}", offset, value)
            }
            StatementKind::ReadMemory { typ, offset } => write!(f, "read-mem {}:{}", offset, typ),
            StatementKind::WriteMemory { offset, value } => {
                write!(f, "write-mem {} {}", offset, value)
            }
            StatementKind::BinaryOperation { kind, lhs, rhs } => {
                let op = match kind {
                    BinaryOperationKind::Add => "add",
                    BinaryOperationKind::Sub => "sub",
                    BinaryOperationKind::Multiply => "mul",
                    BinaryOperationKind::Divide => "div",
                    BinaryOperationKind::Modulo => "mod",
                };

                write!(f, "{} {} {}", op, lhs, rhs)
            }
            StatementKind::UnaryOperation { kind, value } => {
                let op = match kind {
                    UnaryOperationKind::Complement => "cmpl",
                };

                write!(f, "{} {}", op, value)
            }
            StatementKind::Call { target, args } => write!(f, "call FUNCNAME"),
            StatementKind::Cast { kind, typ, value } => {
                let op = match kind {
                    CastOperationKind::ZeroExtend => "zx",
                    CastOperationKind::SignExtend => "sx",
                    CastOperationKind::Truncate => "trunc",
                    CastOperationKind::Reinterpret => "reint",
                    CastOperationKind::Convert => "cvt",
                    CastOperationKind::Broadcast => "bcast",
                };

                write!(f, "cast {} {}:{}", op, value, typ)
            }
            StatementKind::Jump { target } => write!(f, "jump {}", target.name()),
            StatementKind::Branch {
                condition,
                true_target,
                false_target,
            } => {
                write!(
                    f,
                    "branch {} {} {}",
                    condition,
                    true_target.name(),
                    false_target.name()
                )
            }
            StatementKind::PhiNode { members } => {
                write!(f, "phi ")?;

                for member in members {
                    write!(f, "(BLOCK, {}) ", member.1)?;
                }

                Ok(())
            }
            StatementKind::Return { value: None } => {
                write!(f, "return")
            }
            StatementKind::Return { value: Some(value) } => {
                write!(f, "return {value}")
            }
            StatementKind::Select {
                condition,
                true_value,
                false_value,
            } => {
                write!(f, "select {} {} {}", condition, true_value, false_value)
            }
        }
    }
}

#[derive(Clone)]
pub enum ValueKind {
    Statement(Statement),
}

#[derive(Clone)]
pub struct Value {
    kind: ValueKind,
}

impl Value {
    pub fn name(&self) -> InternedString {
        match &self.kind {
            ValueKind::Statement(s) => s.name(),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

pub enum ValueClassification {
    Fixed,
    Dynamic,
}

impl Value {
    pub fn classify(&self) -> ValueClassification {
        match &self.kind {
            ValueKind::Statement(stmt) => {
                todo!()
            }
        }
    }

    pub fn typ(&self) -> Type {
        todo!()
    }
}

#[derive(Clone)]
pub struct Statement {
    inner: Rc<RefCell<StatementInner>>,
}

pub struct StatementInner {
    name: InternedString,
    kind: StatementKind,
}

impl Statement {
    pub fn kind(&self) -> StatementKind {
        (*self.inner).borrow().kind.clone()
    }

    pub fn name(&self) -> InternedString {
        (*self.inner).borrow().name.clone()
    }

    pub fn update_names(&self, name: InternedString) {
        (*self.inner).borrow_mut().update_names(name);
    }

    /*pub fn has_value(&self) -> bool {
        match self.kind() {
            StatementKind::Constant { value } => todo!(),
            StatementKind::ReadVariable { symbol } => todo!(),
            StatementKind::WriteVariable { symbol, value } => todo!(),
            StatementKind::ReadRegister { typ, offset } => todo!(),
            StatementKind::WriteRegister { offset, value } => todo!(),
            StatementKind::ReadMemory { typ, offset } => todo!(),
            StatementKind::WriteMemory { offset, value } => todo!(),
            StatementKind::BinaryOperation { kind, lhs, rhs } => todo!(),
            StatementKind::UnaryOperation { kind, value } => todo!(),
            StatementKind::Call { target, args } => todo!(),
            StatementKind::Cast { kind, typ, value } => todo!(),
            StatementKind::Jump { target } => todo!(),
            StatementKind::Branch { condition, true_target, false_target } => todo!(),
            StatementKind::PhiNode { members } => todo!(),
            StatementKind::Return => todo!(),
            StatementKind::Select { condition, true_value, false_value } => todo!(),
        }
    }*/
}

impl Display for Statement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}: {}", self.name(), self.kind())
    }
}

impl StatementInner {
    pub fn update_names(&mut self, name: InternedString) {
        self.name = name;
    }
}

#[derive(Clone)]
pub struct Block {
    inner: Rc<RefCell<BlockInner>>,
}

impl Block {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(BlockInner {
                name: "???".into(),
                statements: LinkedList::new(),
            })),
        }
    }

    pub fn name(&self) -> InternedString {
        (*self.inner).borrow().name.clone()
    }

    pub fn update_names(&self, name: InternedString) {
        (*self.inner).borrow_mut().update_names(name);
    }

    pub fn statements(&self) -> LinkedList<Statement> {
        self.inner.borrow().statements.clone()
    }

    pub fn set_statements<I: Iterator<Item = Statement>>(&self, statements: I) {
        self.inner.borrow_mut().statements = statements.collect();
    }

    pub fn add_statement(&self, kind: StatementKind) -> Statement {
        let statement = Statement {
            inner: Rc::new(RefCell::new(StatementInner {
                name: "???".into(),
                kind,
            })),
        };
        self.inner
            .borrow_mut()
            .statements
            .push_back(statement.clone());
        statement
    }

    pub fn iter(&self) -> BlockIterator {
        BlockIterator::new(self.clone())
    }
}

impl Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "block {}:", self.name())?;

        for stmt in &(*self.inner).borrow().statements {
            writeln!(f, "{}", stmt)?;
        }

        Ok(())
    }
}

impl Hash for Block {
    fn hash<H: Hasher>(&self, state: &mut H) {
        core::ptr::hash(self.inner.as_ptr(), state)
    }
}

impl PartialEq for Block {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for Block {}

pub struct BlockInner {
    name: InternedString,
    statements: LinkedList<Statement>,
}

impl BlockInner {
    pub fn update_names(&mut self, name: InternedString) {
        self.name = name.clone();

        let mut idx = 0;
        for stmt in &self.statements {
            stmt.update_names(format!("s_{}_{}", name.clone(), idx).into());
            idx += 1;
        }
    }
}

pub struct BlockIterator {
    visited: HashSet<Block>,
    remaining: Vec<Block>,
}

impl BlockIterator {
    fn new(start_block: Block) -> Self {
        Self {
            visited: HashSet::default(),
            remaining: vec![start_block],
        }
    }
}

impl Iterator for BlockIterator {
    type Item = Block;

    fn next(&mut self) -> Option<Self::Item> {
        let current = loop {
            let Some(current) = self.remaining.pop() else {
                // if remaining is empty, all blocks have been visited
                return None;
            };

            // if we've already visited the node, get the next one
            if self.visited.contains(&current) {
                continue;
            } else {
                // otherwise return it
                break current;
            }
        };

        // mark current node as processed
        self.visited.insert(current.clone());

        // push children to visit
        if let Some(last) = current.statements().back() {
            self.remaining.extend(match &last.inner.borrow().kind {
                StatementKind::Jump { target } => vec![target.clone()],
                StatementKind::Branch {
                    true_target,
                    false_target,
                    ..
                } => vec![true_target.clone(), false_target.clone()],
                StatementKind::Return { .. } => vec![],
                _ => panic!("block missing terminator"),
            })
        }

        Some(current)
    }
}

#[derive(Clone)]
pub struct Function {
    inner: Rc<RefCell<FunctionInner>>,
}

impl ToTokens for Function {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        todo!()
    }
}

#[derive(Clone)]
pub struct FunctionInner {
    return_type: Rc<Type>,
    parameters: Vec<Rc<Type>>,
    entry_block: Block,
}

impl Function {
    pub fn new(return_type: Rc<Type>, parameters: Vec<Rc<Type>>) -> Self {
        Self {
            inner: Rc::new(RefCell::new(FunctionInner {
                return_type,
                parameters,
                entry_block: Block::new(),
            })),
        }
    }

    pub fn update_names(&self) {
        let mut idx = 0;
        for b in self.inner.borrow().entry_block.iter() {
            b.update_names(format!("b_{}", idx).into());
            idx += 1;
        }
    }
}

impl Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner
            .borrow()
            .entry_block
            .iter()
            .map(|block| write!(f, "{}", block))
            .collect()
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum FunctionKind {
    Execute,
    Other,
}

pub struct Context {
    fns: HashMap<InternedString, (FunctionKind, Function)>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            fns: HashMap::default(),
        }
    }

    pub fn add_function(&mut self, name: InternedString, kind: FunctionKind, func: Function) {
        self.fns.insert(name, (kind, func));
    }

    pub fn update_names(&self) {
        for (_, func) in self.fns.values() {
            func.update_names();
        }
    }

    pub fn optimise(&mut self) {
        todo!()
    }

    pub fn get_execute_functions(&self) -> Vec<Function> {
        self.fns
            .iter()
            .filter_map(|(name, (kind, func))| {
                if kind == &FunctionKind::Execute {
                    Some(func.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Display for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.update_names();

        writeln!(f, "rudder context:")?;

        for (name, (kind, func)) in self.fns.iter() {
            writeln!(
                f,
                "function {} ({}):",
                name,
                if matches!(kind, FunctionKind::Execute) {
                    "execute"
                } else {
                    "other"
                }
            )?;

            write!(f, "{}", func);
            writeln!(f, "");
        }

        Ok(())
    }
}
