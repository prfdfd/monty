use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};

use crate::exceptions::{exc_err_fmt, ExcType, SimpleException};
use crate::heap::HeapData;
use crate::heap::{Heap, ObjectId};
use crate::run::RunResult;
use crate::values::PyValue;

/// Primary value type representing Python objects at runtime.
///
/// This enum uses a hybrid design: small immediate values (Int, Bool, None) are stored
/// inline, while heap-allocated objects (List, Str, Dict, etc.) are stored in the arena
/// and referenced via `Ref(ObjectId)`.
///
/// NOTE: We intentionally keep `Clone` and `PartialEq` derives temporarily during
/// migration, but these will be removed once all code uses `clone_with_heap()` and
/// heap-aware comparisons. Direct cloning bypasses reference counting and will cause leaks.
#[derive(Debug, Clone, PartialEq)]
pub enum Object {
    // Immediate values (stored inline, no heap allocation)
    Undefined,
    Ellipsis,
    None,
    Bool(bool),
    Int(i64),
    Float(f64),
    Range(i64),
    Exc(SimpleException),

    // Heap-allocated values (stored in arena)
    Ref(ObjectId),
}

impl PartialOrd for Object {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Self::Int(s), Self::Int(o)) => s.partial_cmp(o),
            (Self::Float(s), Self::Float(o)) => s.partial_cmp(o),
            (Self::Int(s), Self::Float(o)) => (*s as f64).partial_cmp(o),
            (Self::Float(s), Self::Int(o)) => s.partial_cmp(&(*o as f64)),
            (Self::Bool(s), _) => Self::Int(i64::from(*s)).partial_cmp(other),
            (_, Self::Bool(s)) => self.partial_cmp(&Self::Int(i64::from(*s))),
            // Ref comparison requires heap context, not supported in PartialOrd
            (Self::Ref(_), Self::Ref(_)) => None,
            _ => None,
        }
    }
}

impl From<bool> for Object {
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}

impl PyValue for Object {
    fn py_type(&self, heap: &Heap) -> &'static str {
        match self {
            Self::Undefined => "undefined",
            Self::Ellipsis => "ellipsis",
            Self::None => "NoneType",
            Self::Bool(_) => "bool",
            Self::Int(_) => "int",
            Self::Float(_) => "float",
            Self::Range(_) => "range",
            Self::Exc(e) => e.type_str(),
            Self::Ref(id) => heap.get(*id).py_type(heap),
        }
    }

    fn py_len(&self, heap: &Heap) -> Option<usize> {
        match self {
            Self::Ref(id) => heap.get(*id).py_len(heap),
            _ => None,
        }
    }

    fn py_eq(&self, other: &Self, heap: &Heap) -> bool {
        match (self, other) {
            (Self::Undefined, _) => false,
            (_, Self::Undefined) => false,
            (Self::Int(v1), Self::Int(v2)) => v1 == v2,
            (Self::Range(v1), Self::Range(v2)) => v1 == v2,
            (Self::Bool(v1), Self::Bool(v2)) => v1 == v2,
            (Self::Bool(v1), Self::Int(v2)) => i64::from(*v1) == *v2,
            (Self::Int(v1), Self::Bool(v2)) => *v1 == i64::from(*v2),
            (Self::None, Self::None) => true,
            (Self::Ref(id1), Self::Ref(id2)) => (*id1 == *id2) || heap.get(*id1).py_eq(heap.get(*id2), heap),
            _ => false,
        }
    }

    fn py_dec_ref_ids(&self, stack: &mut Vec<ObjectId>) {
        if let Object::Ref(id) = self {
            stack.push(*id);
        }
    }

    fn py_bool(&self, heap: &Heap) -> bool {
        match self {
            Self::Undefined => false,
            Self::Ellipsis => true,
            Self::None => false,
            Self::Bool(b) => *b,
            Self::Int(v) => *v != 0,
            Self::Float(f) => *f != 0.0,
            Self::Range(v) => *v != 0,
            Self::Exc(_) => true,
            Self::Ref(id) => heap.get(*id).py_bool(heap),
        }
    }

    fn py_repr<'h>(&'h self, heap: &'h Heap) -> Cow<'h, str> {
        match self {
            Self::Ref(id) => heap.get(*id).py_repr(heap),
            _ => self.cow_str(),
        }
    }

    fn py_str<'h>(&'h self, heap: &'h Heap) -> Cow<'h, str> {
        match self {
            Self::Ref(id) => heap.get(*id).py_str(heap),
            _ => self.cow_str(),
        }
    }

    fn py_add(&self, other: &Self, heap: &mut Heap) -> Option<Self> {
        match (self, other) {
            (Self::Int(v1), Self::Int(v2)) => Some(Self::Int(v1 + v2)),
            (Self::Float(v1), Self::Float(v2)) => Some(Self::Float(v1 + v2)),
            (Self::Ref(id1), Self::Ref(id2)) => heap.with_two(*id1, *id2, |heap, left, right| left.py_add(right, heap)),
            _ => None,
        }
    }

    fn py_sub(&self, other: &Self, _heap: &mut Heap) -> Option<Self> {
        match (self, other) {
            (Self::Int(v1), Self::Int(v2)) => Some(Self::Int(v1 - v2)),
            _ => None,
        }
    }

    fn py_mod(&self, other: &Self) -> Option<Self> {
        match (self, other) {
            (Self::Int(v1), Self::Int(v2)) => Some(Self::Int(v1 % v2)),
            (Self::Float(v1), Self::Float(v2)) => Some(Self::Float(v1 % v2)),
            (Self::Float(v1), Self::Int(v2)) => Some(Self::Float(v1 % (*v2 as f64))),
            (Self::Int(v1), Self::Float(v2)) => Some(Self::Float((*v1 as f64) % v2)),
            _ => None,
        }
    }

    fn py_mod_eq(&self, other: &Self, right_value: i64) -> Option<bool> {
        match (self, other) {
            (Self::Int(v1), Self::Int(v2)) => Some(v1 % v2 == right_value),
            (Self::Float(v1), Self::Float(v2)) => Some(v1 % v2 == right_value as f64),
            (Self::Float(v1), Self::Int(v2)) => Some(v1 % (*v2 as f64) == right_value as f64),
            (Self::Int(v1), Self::Float(v2)) => Some((*v1 as f64) % v2 == right_value as f64),
            _ => None,
        }
    }

    fn py_iadd(&mut self, other: Self, heap: &mut Heap, _self_id: Option<ObjectId>) -> Result<(), Self> {
        match self {
            Self::Int(v1) => {
                if let Object::Int(v2) = other {
                    *v1 += v2;
                    Ok(())
                } else {
                    Err(other)
                }
            }
            Self::Ref(id) => {
                let id = *id;
                heap.with_entry_mut(id, |heap, data| data.py_iadd(other, heap, Some(id)))
            }
            _ => Err(other),
        }
    }

    fn py_getitem(&self, key: &Self, heap: &Heap) -> RunResult<'static, Self> {
        match self {
            Object::Ref(id) => heap.get(*id).py_getitem(key, heap),
            _ => Err(ExcType::type_error_not_sub(self.py_type(heap))),
        }
    }
}

/// Implementation of AbstractValue for boxed Objects.
///
/// This is used for the `HeapData::Object` variant, which wraps immediate values
/// that have been boxed to give them a stable heap identity (e.g., when `id()` is
/// called on an int).
impl PyValue for Box<Object> {
    fn py_type(&self, heap: &Heap) -> &'static str {
        self.as_ref().py_type(heap)
    }

    fn py_len(&self, heap: &Heap) -> Option<usize> {
        // Boxed Objects don't have len - they're immediate values like Int, Bool
        self.as_ref().py_len(heap)
    }

    fn py_eq(&self, other: &Self, heap: &Heap) -> bool {
        self.as_ref().py_eq(other, heap)
    }

    fn py_dec_ref_ids(&self, stack: &mut Vec<ObjectId>) {
        self.as_ref().py_dec_ref_ids(stack);
    }

    fn py_bool(&self, heap: &Heap) -> bool {
        self.as_ref().py_bool(heap)
    }

    fn py_repr<'h>(&'h self, heap: &'h Heap) -> Cow<'h, str> {
        self.as_ref().py_repr(heap)
    }

    fn py_str<'h>(&'h self, heap: &'h Heap) -> Cow<'h, str> {
        self.as_ref().py_str(heap)
    }

    fn py_add(&self, other: &Self, heap: &mut Heap) -> Option<Object> {
        self.as_ref().py_add(other, heap)
    }

    fn py_sub(&self, other: &Self, heap: &mut Heap) -> Option<Object> {
        self.as_ref().py_sub(other, heap)
    }

    fn py_mod(&self, other: &Self) -> Option<Object> {
        self.as_ref().py_mod(other)
    }

    fn py_mod_eq(&self, other: &Self, right_value: i64) -> Option<bool> {
        self.as_ref().py_mod_eq(other, right_value)
    }

    fn py_iadd(&mut self, other: Object, heap: &mut Heap, self_id: Option<ObjectId>) -> Result<(), Object> {
        PyValue::py_iadd(self.as_mut(), other, heap, self_id)
    }

    fn py_call_attr<'c>(&mut self, heap: &mut Heap, attr: &Attr, args: Vec<Object>) -> RunResult<'c, Object> {
        self.as_mut().py_call_attr(heap, attr, args)
    }
}

impl Object {
    /// Performs Python `__iadd__`, mutating the left operand when possible.
    pub fn py_iadd(&mut self, other: Self, heap: &mut Heap) -> Result<(), Self> {
        PyValue::py_iadd(self, other, heap, None)
    }

    /// Returns a stable, unique identifier for this object, boxing it to the heap if necessary.
    ///
    /// Should match Python's `id()` function.
    ///
    /// For inline values (Int, Float, Range), this method allocates them to the heap on first call
    /// and replaces `self` with an `Object::Ref` pointing to the boxed value. This ensures that
    /// subsequent calls to `id()` return the same stable heap address.
    ///
    /// Singletons (None, True, False, etc.) return constant IDs without heap allocation.
    /// Already heap-allocated objects (Ref) return their existing ObjectId.
    pub fn id(&mut self, heap: &mut Heap) -> usize {
        match self {
            // should not be used in practice
            Self::Undefined => 0,
            // Singletons have constant IDs
            Self::Ellipsis => 1,
            Self::None => 2,
            Self::Bool(b) => usize::from(*b) + 3,
            // Already heap-allocated, return id plus 5
            Self::Ref(id) => *id + 5,
            // Everything else (Int, Float, Range, Exc) needs to be boxed
            _ => {
                // Clone the current value before replacing it
                let boxed = Box::new(self.clone());
                let new_id = heap.allocate(HeapData::Object(boxed));
                // Replace self with a Ref to the newly allocated heap object
                *self = Self::Ref(new_id);
                // again return id plus 5
                new_id
            }
        }
    }

    /// Equivalent of Python's `is` method.
    pub fn is(&mut self, heap: &mut Heap, other: &mut Self) -> bool {
        self.id(heap) == other.id(heap)
    }

    /// Computes the hash value for this object, used for dict keys.
    ///
    /// Returns Some(hash) for hashable types (immediate values and immutable heap types).
    /// Returns None for unhashable types (list, dict).
    ///
    /// For heap-allocated objects (Ref variant), this retrieves the precomputed
    /// cached hash. The hash is computed once during allocation for immutable types.
    pub fn py_hash_u64(&self, heap: &Heap) -> Option<u64> {
        match self {
            // Immediate values can be hashed directly
            Self::Undefined => Some(0),
            Self::Ellipsis => Some(1),
            Self::None => Some(2),
            Self::Bool(b) => {
                let mut hasher = DefaultHasher::new();
                b.hash(&mut hasher);
                Some(hasher.finish())
            }
            Self::Int(i) => {
                let mut hasher = DefaultHasher::new();
                i.hash(&mut hasher);
                Some(hasher.finish())
            }
            Self::Float(f) => {
                let mut hasher = DefaultHasher::new();
                // Hash the bit representation of float for consistency
                f.to_bits().hash(&mut hasher);
                Some(hasher.finish())
            }
            Self::Range(r) => {
                let mut hasher = DefaultHasher::new();
                r.hash(&mut hasher);
                Some(hasher.finish())
            }
            Self::Exc(e) => {
                // Exceptions are rarely used as dict keys, but we provide a hash
                // based on the exception type and argument for proper distribution
                Some(e.py_hash())
            }
            // For heap-allocated objects, use the cached hash
            // No cached hash means it's a mutable type (list, dict)
            Self::Ref(id) => heap.get_cached_hash(*id),
        }
    }

    /// TODO maybe replace with TryFrom
    pub fn as_int(&self) -> RunResult<'static, i64> {
        match self {
            Self::Int(i) => Ok(*i),
            // TODO use self.type
            _ => exc_err_fmt!(ExcType::TypeError; "'{self:?}' object cannot be interpreted as an integer"),
        }
    }

    /// Calls an attribute method on this object (e.g., list.append()).
    ///
    /// This method requires heap access to work with heap-allocated objects and
    /// to generate accurate error messages.
    pub fn call_attr<'c>(&mut self, heap: &mut Heap, attr: &Attr, args: Vec<Self>) -> RunResult<'c, Object> {
        if let Self::Ref(id) = self {
            heap.call_attr(*id, attr, args)
        } else {
            Err(ExcType::attribute_error(self.py_type(heap), attr))
        }
    }

    /// Clones an object with proper heap reference counting.
    ///
    /// For immediate values (Int, Bool, None, etc.), this performs a simple copy.
    /// For heap-allocated objects (Ref variant), this increments the reference count
    /// and returns a new reference to the same heap object.
    ///
    /// # Important
    /// This method MUST be used instead of the derived `Clone` implementation to ensure
    /// proper reference counting. Using `.clone()` directly will bypass reference counting
    /// and cause memory leaks or double-frees.
    #[must_use]
    pub fn clone_with_heap(&self, heap: &mut Heap) -> Self {
        match self {
            Self::Ref(id) => {
                heap.inc_ref(*id);
                Self::Ref(*id)
            }
            // Immediate values can be copied without heap interaction
            other => other.clone_immediate(),
        }
    }

    /// Drops an object, decrementing its heap reference count if applicable.
    ///
    /// For immediate values, this is a no-op. For heap-allocated objects (Ref variant),
    /// this decrements the reference count and frees the object (and any children) when
    /// the count reaches zero.
    ///
    /// # Important
    /// This method MUST be called before overwriting a namespace slot or discarding
    /// a value to prevent memory leaks.
    pub fn drop_with_heap(self, heap: &mut Heap) {
        if let Self::Ref(id) = self {
            heap.dec_ref(id);
        }
    }

    /// Internal helper for copying immediate values without heap interaction.
    ///
    /// This method should only be called by `clone_with_heap()` for immediate values.
    /// Attempting to clone a Ref variant will panic.
    fn clone_immediate(&self) -> Self {
        match self {
            Self::Undefined => Self::Undefined,
            Self::Ellipsis => Self::Ellipsis,
            Self::None => Self::None,
            Self::Bool(b) => Self::Bool(*b),
            Self::Int(v) => Self::Int(*v),
            Self::Float(v) => Self::Float(*v),
            Self::Range(v) => Self::Range(*v),
            Self::Exc(e) => Self::Exc(e.clone()),
            Self::Ref(_) => unreachable!("Ref clones must go through clone_with_heap to maintain refcounts"),
        }
    }

    fn cow_str(&self) -> Cow<'static, str> {
        match self {
            Self::Undefined => "Undefined".into(),
            Self::Ellipsis => "Ellipsis".into(),
            Self::None => "None".into(),
            Self::Bool(true) => "True".into(),
            Self::Bool(false) => "False".into(),
            Self::Int(v) => format!("{v}").into(),
            Self::Float(v) => {
                let s = v.to_string();
                if s.contains('.') {
                    s.into()
                } else {
                    format!("{s}.0").into()
                }
            }
            Self::Range(size) => format!("0:{size}").into(),
            Self::Exc(exc) => format!("{exc}").into(),
            Self::Ref(id) => format!("<Ref({id})>").into(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Attr {
    Append,
    Insert,
    Get,
    Keys,
    Values,
    Items,
    Pop,
    Other(String),
}

impl fmt::Display for Attr {
    // TODO replace with a strum
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Append => write!(f, "append"),
            Self::Insert => write!(f, "insert"),
            Self::Get => write!(f, "get"),
            Self::Keys => write!(f, "keys"),
            Self::Values => write!(f, "values"),
            Self::Items => write!(f, "items"),
            Self::Pop => write!(f, "pop"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

impl From<String> for Attr {
    fn from(name: String) -> Self {
        match name.as_str() {
            "append" => Self::Append,
            "insert" => Self::Insert,
            "get" => Self::Get,
            "keys" => Self::Keys,
            "values" => Self::Values,
            "items" => Self::Items,
            "pop" => Self::Pop,
            _ => Self::Other(name),
        }
    }
}
