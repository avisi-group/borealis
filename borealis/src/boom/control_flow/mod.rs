//! Generating control flow graph from BOOM
//!
//! Needs to be restructured due to two main areas of complexity, both caused by block targets being unresolved during building (IE. visiting a jump before the label it references is created):
//!
//! 1. Two sets of types, internal maybe-unresolved and public resolved to enforce resolution at type level.
//! 2. Recursive resolution to convert maybe-unresolved to resolved blocks.

use {
    crate::boom::{control_flow::builder::ControlFlowGraphBuilder, Statement, Value},
    common::intern::InternedString,
    std::{
        cell::RefCell,
        collections::{hash_map::DefaultHasher, HashSet},
        fmt::Display,
        fmt::{self, Formatter},
        hash::{Hash, Hasher},
        io::{self, Write},
        ptr,
        rc::{Rc, Weak},
    },
};

mod builder;
mod dot;

/// Control flow graph of a BOOM function
#[derive(Debug, Clone)]
pub struct ControlFlowGraph {
    /// Function entrypoint
    pub entry_block: ControlFlowBlock,
}

impl ControlFlowGraph {
    /// Creates a control flow graph from a slice of statements.
    ///
    /// This should be called per BOOM function.
    pub fn from_statements(statements: &[Rc<RefCell<Statement>>]) -> Self {
        let entry_block = ControlFlowGraphBuilder::from_statements(statements);

        Self { entry_block }
    }

    /// Renders a `ControlFlowGraph` to DOT syntax.
    pub fn as_dot<W: Write>(&self, w: &mut W) -> io::Result<()> {
        dot::render(w, &self.entry_block)
    }
}

/// Node in a control flow graph, contains a basic block of statements and a terminator
#[derive(Debug, Clone)]
pub struct ControlFlowBlock {
    inner: Rc<RefCell<ControlFlowBlockInner>>,
}

#[derive(Debug)]
struct ControlFlowBlockInner {
    /// Optional block label, otherwise the `SharedKey` `Display` format should be used
    label: Option<InternedString>,
    /// Parents of the current node
    parents: HashSet<ControlFlowBlockWeak>,
    /// Sequence of statements within the block
    statements: Vec<Rc<RefCell<Statement>>>,
    /// Block terminator
    terminator: Terminator,
}

impl Hash for ControlFlowBlock {
    fn hash<H: Hasher>(&self, state: &mut H) {
        ptr::hash(self.inner.as_ptr(), state)
    }
}

impl PartialEq for ControlFlowBlock {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for ControlFlowBlock {}

impl Display for ControlFlowBlock {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.inner.borrow().label {
            Some(label) => write!(f, "{label}"),
            None => {
                let mut state = DefaultHasher::new();
                self.hash(&mut state);
                write!(f, "{:016X}", state.finish())
            }
        }
    }
}

impl ControlFlowBlock {
    fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(ControlFlowBlockInner {
                label: None,
                parents: HashSet::new(),
                statements: vec![],
                terminator: Terminator::Return,
            })),
        }
    }

    pub fn downgrade(&self) -> ControlFlowBlockWeak {
        ControlFlowBlockWeak(Rc::downgrade(&self.inner))
    }

    pub fn label(&self) -> Option<InternedString> {
        self.inner.borrow().label
    }

    pub fn set_label(&self, label: Option<InternedString>) {
        self.inner.borrow_mut().label = label;
    }

    /// Gets the parents of this control flow block
    pub fn parents(&self) -> Vec<ControlFlowBlock> {
        self.inner
            .borrow()
            .parents
            .iter()
            .filter_map(ControlFlowBlockWeak::upgrade)
            .collect()
    }

    /// Adds a parent to this control flow block
    pub fn add_parent(&self, parent: &Self) {
        let parents = &mut self.inner.borrow_mut().parents;

        // remove dropped weak pointers
        parents.retain(|weak| weak.upgrade().is_some());

        // add new weak pointer
        parents.insert(parent.downgrade());
    }

    /// Gets the terminator of this control flow block
    pub fn terminator(&self) -> Terminator {
        self.inner.borrow().terminator.clone()
    }

    /// Sets the terminator of this control flow block (and also the weak parental references of any children)
    pub fn set_terminator(&self, terminator: Terminator) {
        match &terminator {
            Terminator::Return | Terminator::Undefined => (),
            Terminator::Conditional {
                target,
                fallthrough,
                ..
            } => {
                target.add_parent(self);
                fallthrough.add_parent(self);
            }
            Terminator::Unconditional { target } => target.add_parent(self),
        }

        self.inner.borrow_mut().terminator = terminator;
    }

    pub fn statements(&self) -> Vec<Rc<RefCell<Statement>>> {
        self.inner.borrow().statements.clone()
    }

    pub fn set_statements(&self, statements: Vec<Rc<RefCell<Statement>>>) {
        self.inner.borrow_mut().statements = statements;
    }

    /// Renders a `ControlFlowBlock` to DOT syntax.
    pub fn as_dot<W: Write>(&self, w: &mut W) -> io::Result<()> {
        dot::render(w, self)
    }
}

#[derive(Debug)]
pub struct ControlFlowBlockWeak(Weak<RefCell<ControlFlowBlockInner>>);

impl ControlFlowBlockWeak {
    pub fn upgrade(&self) -> Option<ControlFlowBlock> {
        self.0.upgrade().map(|inner| ControlFlowBlock { inner })
    }
}

impl Hash for ControlFlowBlockWeak {
    fn hash<H: Hasher>(&self, state: &mut H) {
        ptr::hash(self.0.as_ptr(), state)
    }
}

impl PartialEq for ControlFlowBlockWeak {
    fn eq(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for ControlFlowBlockWeak {}

/// Block terminator
///
/// Describes how one block conditionally or unconditionally jumps to the next
#[derive(Debug, Clone)]
pub enum Terminator {
    /// Function return
    Return,
    /// If condition evaluates to true, then jump to target, otherwise jump to fallthrough
    Conditional {
        condition: Value,
        target: ControlFlowBlock,
        fallthrough: ControlFlowBlock,
    },
    /// Unconditionally jump to target
    Unconditional {
        target: ControlFlowBlock,
    },
    Undefined,
}

impl Terminator {
    pub fn targets(&self) -> Vec<ControlFlowBlock> {
        match self {
            Terminator::Return | Terminator::Undefined => vec![],
            Terminator::Conditional {
                target,
                fallthrough,
                ..
            } => vec![target.clone(), fallthrough.clone()],
            Terminator::Unconditional { target } => vec![target.clone()],
        }
    }
}
