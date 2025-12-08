/// Python tuple type, wrapping a `Vec<Value<'c, 'e>>`.
///
/// This type provides Python tuple semantics. Tuples are immutable sequences
/// that can contain any Python object. Like lists, tuples properly handle
/// reference counting for heap-allocated values.
use std::borrow::Cow;

use crate::exceptions::ExcType;
use crate::heap::{Heap, HeapId};
use crate::resource::ResourceTracker;
use crate::run::RunResult;
use crate::value::Value;
use crate::values::list::repr_sequence;
use crate::values::PyTrait;

/// Python tuple value stored on the heap.
///
/// Wraps a `Vec<Value<'c, 'e>>` and provides Python-compatible tuple operations.
/// Unlike lists, tuples are conceptually immutable (though this is not
/// enforced at the type level for internal operations).
///
/// # Reference Counting
/// When a tuple is freed, all contained heap references have their refcounts
/// decremented via `push_stack_ids`.
#[derive(Debug, Default)]
pub struct Tuple<'c, 'e>(Vec<Value<'c, 'e>>);

impl<'c, 'e> Tuple<'c, 'e> {
    /// Creates a new tuple from a vector of values.
    ///
    /// Note: This does NOT increment reference counts - the caller must
    /// ensure refcounts are properly managed.
    #[must_use]
    pub fn from_vec(vec: Vec<Value<'c, 'e>>) -> Self {
        Self(vec)
    }

    /// Returns a reference to the underlying vector.
    #[must_use]
    pub fn as_vec(&self) -> &Vec<Value<'c, 'e>> {
        &self.0
    }

    /// Creates a deep clone of this tuple with proper reference counting.
    ///
    /// All heap-allocated values in the tuple have their reference counts
    /// incremented. This should be used instead of `.clone()` which would
    /// bypass reference counting.
    #[must_use]
    pub fn clone_with_heap<T: ResourceTracker>(&self, heap: &mut Heap<'c, 'e, T>) -> Self {
        let cloned: Vec<Value<'c, 'e>> = self.0.iter().map(|obj| obj.clone_with_heap(heap)).collect();
        Self(cloned)
    }
}

impl<'c, 'e> From<Vec<Value<'c, 'e>>> for Tuple<'c, 'e> {
    fn from(vec: Vec<Value<'c, 'e>>) -> Self {
        Self(vec)
    }
}

impl<'c, 'e> std::iter::FromIterator<Value<'c, 'e>> for Tuple<'c, 'e> {
    fn from_iter<I: IntoIterator<Item = Value<'c, 'e>>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<'c, 'e> From<Tuple<'c, 'e>> for Vec<Value<'c, 'e>> {
    fn from(tuple: Tuple<'c, 'e>) -> Self {
        tuple.0
    }
}

impl<'c, 'e> PyTrait<'c, 'e> for Tuple<'c, 'e> {
    fn py_type<T: ResourceTracker>(&self, _heap: Option<&Heap<'c, 'e, T>>) -> &'static str {
        "tuple"
    }

    fn py_estimate_size(&self) -> usize {
        std::mem::size_of::<Self>() + self.0.len() * std::mem::size_of::<Value>()
    }

    fn py_len<T: ResourceTracker>(&self, _heap: &Heap<'c, 'e, T>) -> Option<usize> {
        Some(self.0.len())
    }

    fn py_getitem<T: ResourceTracker>(
        &self,
        key: &Value<'c, 'e>,
        heap: &mut Heap<'c, 'e, T>,
    ) -> RunResult<'static, Value<'c, 'e>> {
        // Extract integer index from key, returning TypeError if not an int
        let index = match key {
            Value::Int(i) => *i,
            _ => return Err(ExcType::type_error_indices("tuple", key.py_type(Some(heap)))),
        };

        // Convert to usize, handling negative indices (Python-style: -1 = last element)
        let len = self.0.len() as i64;
        let normalized_index = if index < 0 { index + len } else { index };

        // Bounds check
        if normalized_index < 0 || normalized_index >= len {
            return Err(ExcType::tuple_index_error());
        }

        // Return clone of the item with proper refcount increment
        Ok(self.0[normalized_index as usize].clone_with_heap(heap))
    }

    fn py_eq<T: ResourceTracker>(&self, other: &Self, heap: &mut Heap<'c, 'e, T>) -> bool {
        if self.0.len() != other.0.len() {
            return false;
        }
        for (i1, i2) in self.0.iter().zip(&other.0) {
            if !i1.py_eq(i2, heap) {
                return false;
            }
        }
        true
    }

    /// Pushes all heap IDs contained in this tuple onto the stack.
    ///
    /// Called during garbage collection to decrement refcounts of nested values.
    /// When `dec-ref-check` is enabled, also marks all Values as Dereferenced.
    fn py_dec_ref_ids(&mut self, stack: &mut Vec<HeapId>) {
        for obj in &mut self.0 {
            if let Value::Ref(id) = obj {
                stack.push(*id);
                #[cfg(feature = "dec-ref-check")]
                obj.dec_ref_forget();
            }
        }
    }

    // py_call_attr uses default implementation which returns AttributeError

    fn py_bool<T: ResourceTracker>(&self, _heap: &Heap<'c, 'e, T>) -> bool {
        !self.0.is_empty()
    }

    fn py_repr<'a, T: ResourceTracker>(&'a self, heap: &'a Heap<'c, 'e, T>) -> Cow<'a, str> {
        Cow::Owned(repr_sequence('(', ')', &self.0, heap))
    }
}
