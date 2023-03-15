//! String interning

use {
    deepsize::DeepSizeOf,
    fxhash::FxBuildHasher,
    lasso::{Spur, ThreadedRodeo},
    ocaml::{FromValue, ToValue, Value},
    once_cell::sync::Lazy,
};

/// String interner instance
pub static INTERNER: Lazy<ThreadedRodeo<Spur, FxBuildHasher>> =
    Lazy::new(|| ThreadedRodeo::with_hasher(FxBuildHasher::default()));

/// Key for an interned string
#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Hash, Ord)]
pub struct InternedString(Spur);

impl InternedString {
    /// Create a new interned string
    pub fn new<A: AsRef<str>>(str: A) -> Self {
        Self(INTERNER.get_or_intern(str))
    }

    /// Create a new interned string from a static str
    pub fn from_static(key: &'static str) -> Self {
        Self(INTERNER.get_or_intern_static(key))
    }
}

impl AsRef<str> for InternedString {
    fn as_ref(&self) -> &str {
        INTERNER.resolve(&self.0)
    }
}

impl From<Spur> for InternedString {
    fn from(spur: Spur) -> Self {
        Self(spur)
    }
}

impl From<String> for InternedString {
    fn from(string: String) -> Self {
        Self::new(string)
    }
}

impl From<&'_ str> for InternedString {
    fn from(string: &str) -> Self {
        Self::new(string)
    }
}

impl std::fmt::Debug for InternedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        INTERNER.resolve(&self.0).fmt(f)
    }
}

impl std::fmt::Display for InternedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        INTERNER.resolve(&self.0).fmt(f)
    }
}

impl<'de> serde::Deserialize<'de> for InternedString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer).map(Self::new)
    }
}

impl serde::Serialize for InternedString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

unsafe impl FromValue for InternedString {
    fn from_value(v: Value) -> Self {
        Self::new(String::from_value(v))
    }
}

unsafe impl ToValue for InternedString {
    fn to_value(&self, rt: &ocaml::Runtime) -> Value {
        self.to_string().to_value(rt)
    }
}

impl DeepSizeOf for InternedString {
    fn deep_size_of_children(&self, _context: &mut deepsize::Context) -> usize {
        0
    }

    fn deep_size_of(&self) -> usize {
        std::mem::size_of_val(self)
    }
}
