/// Python string type, wrapping a Rust `String`.
///
/// This type provides Python string semantics. Currently supports basic
/// operations like length and equality comparison.
use std::borrow::Cow;

use crate::heap::{Heap, HeapData, HeapId};
use crate::resource::ResourceTracker;
use crate::value::Value;
use crate::values::PyTrait;

/// Python string value stored on the heap.
///
/// Wraps a Rust `String` and provides Python-compatible operations.
/// `len()` returns the number of Unicode codepoints (characters), matching Python semantics.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Str(String);

impl Str {
    /// Creates a new Str from a Rust String.
    #[must_use]
    pub fn new(s: String) -> Self {
        Self(s)
    }

    /// Returns a reference to the inner string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns a mutable reference to the inner string.
    pub fn as_string_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

impl From<String> for Str {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for Str {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<Str> for String {
    fn from(value: Str) -> Self {
        value.0
    }
}

impl std::ops::Deref for Str {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'c, 'e> PyTrait<'c, 'e> for Str {
    fn py_type<T: ResourceTracker>(&self, _heap: Option<&Heap<'c, 'e, T>>) -> &'static str {
        "str"
    }

    fn py_estimate_size(&self) -> usize {
        std::mem::size_of::<Self>() + self.0.len()
    }

    fn py_len<T: ResourceTracker>(&self, _heap: &Heap<'c, 'e, T>) -> Option<usize> {
        // Count Unicode characters, not bytes, to match Python semantics
        Some(self.0.chars().count())
    }

    fn py_eq<T: ResourceTracker>(&self, other: &Self, _heap: &mut Heap<'c, 'e, T>) -> bool {
        self.0 == other.0
    }

    /// Strings don't contain nested heap references.
    fn py_dec_ref_ids(&mut self, _stack: &mut Vec<HeapId>) {
        // No-op: strings don't hold Value references
    }

    fn py_bool<T: ResourceTracker>(&self, _heap: &Heap<'c, 'e, T>) -> bool {
        !self.0.is_empty()
    }

    fn py_repr<'a, T: ResourceTracker>(&'a self, _heap: &'a Heap<'c, 'e, T>) -> Cow<'a, str> {
        Cow::Owned(string_repr(&self.0))
    }

    fn py_str<'a, T: ResourceTracker>(&'a self, _heap: &'a Heap<'c, 'e, T>) -> Cow<'a, str> {
        self.0.as_str().into()
    }

    fn py_add<T: ResourceTracker>(
        &self,
        other: &Self,
        heap: &mut Heap<'c, 'e, T>,
    ) -> Result<Option<Value<'c, 'e>>, crate::resource::ResourceError> {
        let result = format!("{}{}", self.0, other.0);
        let id = heap.allocate(HeapData::Str(result.into()))?;
        Ok(Some(Value::Ref(id)))
    }

    fn py_iadd<T: ResourceTracker>(
        &mut self,
        other: Value<'c, 'e>,
        heap: &mut Heap<'c, 'e, T>,
        self_id: Option<HeapId>,
    ) -> Result<bool, crate::resource::ResourceError> {
        match other {
            Value::Ref(other_id) => {
                if Some(other_id) == self_id {
                    let rhs = self.0.clone();
                    self.0.push_str(&rhs);
                    Ok(true)
                } else if let HeapData::Str(rhs) = heap.get(other_id) {
                    self.0.push_str(rhs.as_str());
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            _ => Ok(false),
        }
    }
    // py_call_attr uses default implementation which returns AttributeError
}

/// Macro for common string escape replacements used in repr formatting.
///
/// Replaces backslash, newline, tab, and carriage return with their escaped forms.
macro_rules! string_replace_common {
    ($s:expr) => {
        $s.replace('\\', "\\\\")
            .replace('\n', "\\n")
            .replace('\t', "\\t")
            .replace('\r', "\\r")
    };
}

/// Returns a Python repr() string for a given string slice.
///
/// Chooses between single and double quotes based on the string content:
/// - Uses double quotes if the string contains single quotes but not double quotes
/// - Uses single quotes by default, escaping any contained single quotes
///
/// Common escape sequences (backslash, newline, tab, carriage return) are always escaped.
pub fn string_repr(s: &str) -> String {
    // Check if the string contains single quotes but not double quotes
    if s.contains('\'') && !s.contains('"') {
        // Use double quotes if string contains only single quotes
        format!("\"{}\"", string_replace_common!(s))
    } else {
        // Use single quotes by default, escape any single quotes in the string
        format!("'{}'", string_replace_common!(s.replace('\'', "\\'")))
    }
}
