use std::borrow::Cow;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::object::{Attr, Object};
use crate::run::RunResult;
// Import AbstractValue trait for enum_dispatch to work
use crate::values::PyValue;
use crate::values::{Bytes, Dict, List, Str, Tuple};

/// Unique identifier for objects stored inside the heap arena.
pub type ObjectId = usize;

/// HeapData captures every runtime object that must live in the arena.
///
/// Each variant wraps a type that implements `AbstractValue`, providing
/// Python-compatible operations. The trait is manually implemented to dispatch
/// to the appropriate variant's implementation.
///
/// Note: The `Object` variant is special - it wraps boxed immediate values
/// that need heap identity (e.g., when `id()` is called on an int).
#[derive(Debug)]
pub enum HeapData {
    /// Boxed object used when id() is called on some values
    /// to provide them with a unique identity.
    Object(Box<Object>),
    Str(Str),
    Bytes(Bytes),
    List(List),
    Tuple(Tuple),
    Dict(Dict),
    // TODO: support arbitrary classes
}

impl HeapData {
    /// Computes hash for immutable heap types that can be used as dict keys.
    ///
    /// Returns Some(hash) for immutable types (Str, Bytes, Tuple of hashables).
    /// Returns None for mutable types (List, Object) which cannot be dict keys.
    ///
    /// This is called during heap allocation to precompute and cache hashes,
    /// avoiding the need to access the heap during hash operations.
    fn compute_hash_if_immutable(&self, heap: &Heap) -> Option<u64> {
        match self {
            Self::Str(s) => {
                let mut hasher = DefaultHasher::new();
                s.as_str().hash(&mut hasher);
                Some(hasher.finish())
            }
            Self::Bytes(b) => {
                let mut hasher = DefaultHasher::new();
                b.as_slice().hash(&mut hasher);
                Some(hasher.finish())
            }
            Self::Tuple(t) => {
                // Tuple is hashable only if all elements are hashable
                let mut hasher = DefaultHasher::new();
                for obj in t.as_vec() {
                    match obj.py_hash_u64(heap) {
                        Some(h) => h.hash(&mut hasher),
                        None => return None, // Contains unhashable element
                    }
                }
                Some(hasher.finish())
            }
            // Mutable types cannot be hashed
            Self::List(_) | Self::Dict(_) | Self::Object(_) => None,
        }
    }
}

/// Manual implementation of AbstractValue dispatch for HeapData.
///
/// This provides efficient dispatch without boxing overhead by matching on
/// the enum variant and delegating to the inner type's implementation.
impl PyValue for HeapData {
    fn py_type(&self, heap: &Heap) -> &'static str {
        match self {
            Self::Object(obj) => obj.py_type(heap),
            Self::Str(s) => s.py_type(heap),
            Self::Bytes(b) => b.py_type(heap),
            Self::List(l) => l.py_type(heap),
            Self::Tuple(t) => t.py_type(heap),
            Self::Dict(d) => d.py_type(heap),
        }
    }

    fn py_len(&self, heap: &Heap) -> Option<usize> {
        match self {
            Self::Object(obj) => PyValue::py_len(obj, heap),
            Self::Str(s) => PyValue::py_len(s, heap),
            Self::Bytes(b) => PyValue::py_len(b, heap),
            Self::List(l) => PyValue::py_len(l, heap),
            Self::Tuple(t) => PyValue::py_len(t, heap),
            Self::Dict(d) => PyValue::py_len(d, heap),
        }
    }

    fn py_eq(&self, other: &Self, heap: &Heap) -> bool {
        match (self, other) {
            (Self::Object(a), Self::Object(b)) => a.py_eq(b, heap),
            (Self::Str(a), Self::Str(b)) => a.py_eq(b, heap),
            (Self::Bytes(a), Self::Bytes(b)) => a.py_eq(b, heap),
            (Self::List(a), Self::List(b)) => a.py_eq(b, heap),
            (Self::Tuple(a), Self::Tuple(b)) => a.py_eq(b, heap),
            (Self::Dict(a), Self::Dict(b)) => a.py_eq(b, heap),
            _ => false, // Different types are never equal
        }
    }

    fn py_dec_ref_ids(&self, stack: &mut Vec<ObjectId>) {
        match self {
            Self::Object(obj) => obj.py_dec_ref_ids(stack),
            Self::Str(s) => s.py_dec_ref_ids(stack),
            Self::Bytes(b) => b.py_dec_ref_ids(stack),
            Self::List(l) => l.py_dec_ref_ids(stack),
            Self::Tuple(t) => t.py_dec_ref_ids(stack),
            Self::Dict(d) => d.py_dec_ref_ids(stack),
        }
    }

    fn py_bool(&self, heap: &Heap) -> bool {
        match self {
            Self::Object(obj) => obj.py_bool(heap),
            Self::Str(s) => s.py_bool(heap),
            Self::Bytes(b) => b.py_bool(heap),
            Self::List(l) => l.py_bool(heap),
            Self::Tuple(t) => t.py_bool(heap),
            Self::Dict(d) => d.py_bool(heap),
        }
    }

    fn py_repr<'h>(&'h self, heap: &'h Heap) -> Cow<'h, str> {
        match self {
            Self::Object(obj) => obj.py_repr(heap),
            Self::Str(s) => s.py_repr(heap),
            Self::Bytes(b) => b.py_repr(heap),
            Self::List(l) => l.py_repr(heap),
            Self::Tuple(t) => t.py_repr(heap),
            Self::Dict(d) => d.py_repr(heap),
        }
    }

    fn py_str<'h>(&'h self, heap: &'h Heap) -> Cow<'h, str> {
        match self {
            Self::Object(obj) => obj.py_str(heap),
            Self::Str(s) => s.py_str(heap),
            Self::Bytes(b) => b.py_str(heap),
            Self::List(l) => l.py_str(heap),
            Self::Tuple(t) => t.py_str(heap),
            Self::Dict(d) => d.py_str(heap),
        }
    }

    fn py_add(&self, other: &Self, heap: &mut Heap) -> Option<Object> {
        match (self, other) {
            (Self::Object(a), Self::Object(b)) => a.py_add(b, heap),
            (Self::Str(a), Self::Str(b)) => a.py_add(b, heap),
            (Self::Bytes(a), Self::Bytes(b)) => a.py_add(b, heap),
            (Self::List(a), Self::List(b)) => a.py_add(b, heap),
            (Self::Tuple(a), Self::Tuple(b)) => a.py_add(b, heap),
            (Self::Dict(a), Self::Dict(b)) => a.py_add(b, heap),
            _ => None,
        }
    }

    fn py_sub(&self, other: &Self, heap: &mut Heap) -> Option<Object> {
        match (self, other) {
            (Self::Object(a), Self::Object(b)) => a.py_sub(b, heap),
            (Self::Str(a), Self::Str(b)) => a.py_sub(b, heap),
            (Self::Bytes(a), Self::Bytes(b)) => a.py_sub(b, heap),
            (Self::List(a), Self::List(b)) => a.py_sub(b, heap),
            (Self::Tuple(a), Self::Tuple(b)) => a.py_sub(b, heap),
            (Self::Dict(a), Self::Dict(b)) => a.py_sub(b, heap),
            _ => None,
        }
    }

    fn py_mod(&self, other: &Self) -> Option<Object> {
        match (self, other) {
            (Self::Object(a), Self::Object(b)) => a.py_mod(b),
            (Self::Str(a), Self::Str(b)) => a.py_mod(b),
            (Self::Bytes(a), Self::Bytes(b)) => a.py_mod(b),
            (Self::List(a), Self::List(b)) => a.py_mod(b),
            (Self::Tuple(a), Self::Tuple(b)) => a.py_mod(b),
            (Self::Dict(a), Self::Dict(b)) => a.py_mod(b),
            _ => None,
        }
    }

    fn py_mod_eq(&self, other: &Self, right_value: i64) -> Option<bool> {
        match (self, other) {
            (Self::Object(a), Self::Object(b)) => a.py_mod_eq(b, right_value),
            (Self::Str(a), Self::Str(b)) => a.py_mod_eq(b, right_value),
            (Self::Bytes(a), Self::Bytes(b)) => a.py_mod_eq(b, right_value),
            (Self::List(a), Self::List(b)) => a.py_mod_eq(b, right_value),
            (Self::Tuple(a), Self::Tuple(b)) => a.py_mod_eq(b, right_value),
            (Self::Dict(a), Self::Dict(b)) => a.py_mod_eq(b, right_value),
            _ => None,
        }
    }

    fn py_iadd(&mut self, other: Object, heap: &mut Heap, self_id: Option<ObjectId>) -> Result<(), Object> {
        match self {
            Self::Object(obj) => obj.py_iadd(other, heap, self_id),
            Self::Str(s) => s.py_iadd(other, heap, self_id),
            Self::Bytes(b) => b.py_iadd(other, heap, self_id),
            Self::List(l) => l.py_iadd(other, heap, self_id),
            Self::Tuple(t) => t.py_iadd(other, heap, self_id),
            Self::Dict(d) => d.py_iadd(other, heap, self_id),
        }
    }

    fn py_call_attr<'c>(&mut self, heap: &mut Heap, attr: &Attr, args: Vec<Object>) -> RunResult<'c, Object> {
        match self {
            Self::Object(obj) => obj.py_call_attr(heap, attr, args),
            Self::Str(s) => s.py_call_attr(heap, attr, args),
            Self::Bytes(b) => b.py_call_attr(heap, attr, args),
            Self::List(l) => l.py_call_attr(heap, attr, args),
            Self::Tuple(t) => t.py_call_attr(heap, attr, args),
            Self::Dict(d) => d.py_call_attr(heap, attr, args),
        }
    }

    fn py_getitem(&self, key: &Object, heap: &Heap) -> RunResult<'static, Object> {
        match self {
            Self::Object(obj) => obj.py_getitem(key, heap),
            Self::Str(s) => s.py_getitem(key, heap),
            Self::Bytes(b) => b.py_getitem(key, heap),
            Self::List(l) => l.py_getitem(key, heap),
            Self::Tuple(t) => t.py_getitem(key, heap),
            Self::Dict(d) => d.py_getitem(key, heap),
        }
    }

    fn py_setitem(&mut self, key: Object, value: Object, heap: &mut Heap) -> RunResult<'static, ()> {
        match self {
            Self::Object(obj) => obj.py_setitem(key, value, heap),
            Self::Str(s) => s.py_setitem(key, value, heap),
            Self::Bytes(b) => b.py_setitem(key, value, heap),
            Self::List(l) => l.py_setitem(key, value, heap),
            Self::Tuple(t) => t.py_setitem(key, value, heap),
            Self::Dict(d) => d.py_setitem(key, value, heap),
        }
    }
}

/// A single entry inside the heap arena, storing refcount and payload.
///
/// The `cached_hash` field is used to store precomputed hashes for immutable types
/// (Str, Bytes, Tuple) to allow them to be used as dict keys. Mutable types (List, Dict)
/// have `cached_hash = None` and will raise TypeError if used as dict keys.
#[derive(Debug)]
struct HeapObject {
    refcount: usize,
    data: HeapData,
    /// Cached hash value for immutable types, None for mutable types
    cached_hash: Option<u64>,
}

/// Reference-counted arena that backs all heap-only runtime objects.
///
/// The heap never reuses IDs during a single execution; instead it appends new
/// entries and relies on `clear()` between runs.  This keeps identity checks
/// simple and avoids the need for generation counters while we're still
/// building out semantics.
#[derive(Debug, Default)]
pub struct Heap {
    objects: Vec<Option<HeapObject>>,
}

impl Heap {
    /// Allocates a new heap object, returning the fresh identifier.
    ///
    /// For immutable types (Str, Bytes, Tuple), this precomputes and caches
    /// the hash value so they can be used as dict keys. Mutable types (List)
    /// get cached_hash = None.
    pub fn allocate(&mut self, data: HeapData) -> ObjectId {
        let cached_hash = data.compute_hash_if_immutable(self);
        let id = self.objects.len();
        self.objects.push(Some(HeapObject {
            refcount: 1,
            data,
            cached_hash,
        }));
        id
    }

    /// Increments the reference count for an existing heap object.
    ///
    /// # Panics
    /// Panics if the object ID is invalid or the object has already been freed.
    pub fn inc_ref(&mut self, id: ObjectId) {
        let object = self
            .objects
            .get_mut(id)
            .expect("Heap::inc_ref: slot missing")
            .as_mut()
            .expect("Heap::inc_ref: object already freed");
        object.refcount += 1;
    }

    /// Decrements the reference count and frees the object (plus children) once it hits zero.
    ///
    /// # Panics
    /// Panics if the object ID is invalid or the object has already been freed.
    pub fn dec_ref(&mut self, id: ObjectId) {
        let mut stack = vec![id];
        while let Some(current) = stack.pop() {
            let slot = self.objects.get_mut(current).expect("Heap::dec_ref: slot missing");
            let entry = slot.as_mut().expect("Heap::dec_ref: object already freed");
            if entry.refcount > 1 {
                entry.refcount -= 1;
                continue;
            }

            if let Some(object) = slot.take() {
                enqueue_children(&object.data, &mut stack);
            }
        }
    }

    /// Returns an immutable reference to the heap data stored at the given ID.
    ///
    /// # Panics
    /// Panics if the object ID is invalid or the object has already been freed.
    #[must_use]
    pub fn get(&self, id: ObjectId) -> &HeapData {
        &self
            .objects
            .get(id)
            .expect("Heap::get: slot missing")
            .as_ref()
            .expect("Heap::get: object already freed")
            .data
    }

    /// Returns a mutable reference to the heap data stored at the given ID.
    ///
    /// # Panics
    /// Panics if the object ID is invalid or the object has already been freed.
    pub fn get_mut(&mut self, id: ObjectId) -> &mut HeapData {
        &mut self
            .objects
            .get_mut(id)
            .expect("Heap::get_mut: slot missing")
            .as_mut()
            .expect("Heap::get_mut: object already freed")
            .data
    }

    /// Returns the cached hash for the heap object at the given ID.
    ///
    /// Returns Some(hash) for immutable types, None for mutable types.
    ///
    /// # Panics
    /// Panics if the object ID is invalid or the object has already been freed.
    #[must_use]
    pub fn get_cached_hash(&self, id: ObjectId) -> Option<u64> {
        self.objects
            .get(id)
            .expect("Heap::get_cached_hash: slot missing")
            .as_ref()
            .expect("Heap::get_cached_hash: object already freed")
            .cached_hash
    }

    /// Calls an attribute on the heap object at `id` while temporarily taking ownership
    /// of its payload so we can borrow the heap again inside the call. This avoids the
    /// borrow checker conflict that arises when attribute implementations also need
    /// mutable access to the heap (e.g. for refcounting).
    pub fn call_attr<'c>(&mut self, id: ObjectId, attr: &Attr, args: Vec<Object>) -> RunResult<'c, Object> {
        let mut entry = {
            let slot = self.objects.get_mut(id).expect("Heap::call_attr: slot missing");
            slot.take().expect("Heap::call_attr: object already freed")
        };
        let result = entry.data.py_call_attr(self, attr, args);
        let slot = self.objects.get_mut(id).expect("Heap::call_attr: slot missing");
        *slot = Some(entry);
        result
    }

    /// Gives mutable access to a heap entry while allowing reentrant heap usage
    /// inside the closure (e.g. to read other objects or allocate results).
    ///
    /// The entry is temporarily removed from the heap, so the closure can safely
    /// mutate both the entry data and the heap (e.g. to allocate new objects).
    /// The entry is automatically restored after the closure completes.
    pub fn with_entry_mut<F, R>(&mut self, id: ObjectId, f: F) -> R
    where
        F: FnOnce(&mut Heap, &mut HeapData) -> R,
    {
        let mut entry = self.take_entry(id);
        let result = f(self, &mut entry.data);
        self.restore_entry(id, entry);
        result
    }

    /// Temporarily takes ownership of two heap entries so their data can be borrowed
    /// simultaneously while still permitting mutable access to the heap (e.g. to
    /// allocate results). Automatically restores both entries after the closure
    /// finishes executing.
    pub fn with_two<F, R>(&mut self, left: ObjectId, right: ObjectId, f: F) -> R
    where
        F: FnOnce(&mut Heap, &HeapData, &HeapData) -> R,
    {
        if left == right {
            let entry = self.take_entry(left);
            let data = &entry.data;
            let result = f(self, data, data);
            self.restore_entry(left, entry);
            result
        } else {
            let left_entry = self.take_entry(left);
            let right_entry = self.take_entry(right);
            let result = f(self, &left_entry.data, &right_entry.data);
            self.restore_entry(right, right_entry);
            self.restore_entry(left, left_entry);
            result
        }
    }

    fn take_entry(&mut self, id: ObjectId) -> HeapObject {
        let slot = self.objects.get_mut(id).expect("Heap::take_entry: slot missing");
        slot.take().expect("Heap::take_entry: object already freed")
    }

    fn restore_entry(&mut self, id: ObjectId, entry: HeapObject) {
        let slot = self.objects.get_mut(id).expect("Heap::restore_entry: slot missing");
        debug_assert!(slot.is_none(), "Heap slot should be empty before restore");
        *slot = Some(entry);
    }

    /// Removes all objects and resets the ID counter, used between executor runs.
    pub fn clear(&mut self) {
        self.objects.clear();
    }

    /// Returns the reference count for the heap object at the given ID.
    ///
    /// This is primarily used for testing reference counting behavior.
    ///
    /// # Panics
    /// Panics if the object ID is invalid or the object has already been freed.
    #[must_use]
    pub fn get_refcount(&self, id: ObjectId) -> usize {
        self.objects
            .get(id)
            .expect("Heap::get_refcount: slot missing")
            .as_ref()
            .expect("Heap::get_refcount: object already freed")
            .refcount
    }

    /// Returns the number of live (non-freed) objects on the heap.
    ///
    /// This is primarily used for testing to verify that all heap objects
    /// are accounted for in reference count tests.
    #[must_use]
    pub fn object_count(&self) -> usize {
        self.objects.iter().filter(|o| o.is_some()).count()
    }
}

/// Pushes any child object IDs referenced by `data` onto the provided stack so
/// `dec_ref` can recursively drop entire object graphs without recursion.
///
/// Uses the `AbstractValue::push_stack_ids` trait method via enum_dispatch.
fn enqueue_children(data: &HeapData, stack: &mut Vec<ObjectId>) {
    data.py_dec_ref_ids(stack);
}
