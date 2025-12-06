use std::borrow::Cow;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::args::ArgObjects;
use crate::object::{Attr, Object};
use crate::run::RunResult;
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
pub enum HeapData<'c, 'e> {
    /// Object in the heap is used when calculating the id of an object to provide it with a unique identity.
    Object(Object<'c, 'e>),
    Str(Str),
    Bytes(Bytes),
    List(List<'c, 'e>),
    Tuple(Tuple<'c, 'e>),
    Dict(Dict<'c, 'e>),
    // TODO: support arbitrary classes
}

impl<'c, 'e> HeapData<'c, 'e> {
    /// Computes hash for immutable heap types that can be used as dict keys.
    ///
    /// Returns Some(hash) for immutable types (Str, Bytes, Tuple of hashables).
    /// Returns None for mutable types (List, Object) which cannot be dict keys.
    ///
    /// This is called lazily when the object is first used as a dict key,
    /// avoiding unnecessary hash computation for objects that are never used as keys.
    fn compute_hash_if_immutable(&self, heap: &mut Heap<'c, 'e>) -> Option<u64> {
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
impl<'c, 'e> PyValue<'c, 'e> for HeapData<'c, 'e> {
    fn py_type(&self, heap: &Heap<'c, 'e>) -> &'static str {
        match self {
            Self::Object(obj) => obj.py_type(heap),
            Self::Str(s) => s.py_type(heap),
            Self::Bytes(b) => b.py_type(heap),
            Self::List(l) => l.py_type(heap),
            Self::Tuple(t) => t.py_type(heap),
            Self::Dict(d) => d.py_type(heap),
        }
    }

    fn py_len(&self, heap: &Heap<'c, 'e>) -> Option<usize> {
        match self {
            Self::Object(obj) => PyValue::py_len(obj, heap),
            Self::Str(s) => PyValue::py_len(s, heap),
            Self::Bytes(b) => PyValue::py_len(b, heap),
            Self::List(l) => PyValue::py_len(l, heap),
            Self::Tuple(t) => PyValue::py_len(t, heap),
            Self::Dict(d) => PyValue::py_len(d, heap),
        }
    }

    fn py_eq(&self, other: &Self, heap: &mut Heap<'c, 'e>) -> bool {
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

    fn py_dec_ref_ids(&mut self, stack: &mut Vec<ObjectId>) {
        match self {
            Self::Object(obj) => obj.py_dec_ref_ids(stack),
            Self::Str(s) => s.py_dec_ref_ids(stack),
            Self::Bytes(b) => b.py_dec_ref_ids(stack),
            Self::List(l) => l.py_dec_ref_ids(stack),
            Self::Tuple(t) => t.py_dec_ref_ids(stack),
            Self::Dict(d) => d.py_dec_ref_ids(stack),
        }
    }

    fn py_bool(&self, heap: &Heap<'c, 'e>) -> bool {
        match self {
            Self::Object(obj) => obj.py_bool(heap),
            Self::Str(s) => s.py_bool(heap),
            Self::Bytes(b) => b.py_bool(heap),
            Self::List(l) => l.py_bool(heap),
            Self::Tuple(t) => t.py_bool(heap),
            Self::Dict(d) => d.py_bool(heap),
        }
    }

    fn py_repr<'a>(&'a self, heap: &'a Heap<'c, 'e>) -> Cow<'a, str> {
        match self {
            Self::Object(obj) => obj.py_repr(heap),
            Self::Str(s) => s.py_repr(heap),
            Self::Bytes(b) => b.py_repr(heap),
            Self::List(l) => l.py_repr(heap),
            Self::Tuple(t) => t.py_repr(heap),
            Self::Dict(d) => d.py_repr(heap),
        }
    }

    fn py_str<'a>(&'a self, heap: &'a Heap<'c, 'e>) -> Cow<'a, str> {
        match self {
            Self::Object(obj) => obj.py_str(heap),
            Self::Str(s) => s.py_str(heap),
            Self::Bytes(b) => b.py_str(heap),
            Self::List(l) => l.py_str(heap),
            Self::Tuple(t) => t.py_str(heap),
            Self::Dict(d) => d.py_str(heap),
        }
    }

    fn py_add(&self, other: &Self, heap: &mut Heap<'c, 'e>) -> Option<Object<'c, 'e>> {
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

    fn py_sub(&self, other: &Self, heap: &mut Heap<'c, 'e>) -> Option<Object<'c, 'e>> {
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

    fn py_mod(&self, other: &Self) -> Option<Object<'c, 'e>> {
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

    fn py_iadd(&mut self, other: Object<'c, 'e>, heap: &mut Heap<'c, 'e>, self_id: Option<ObjectId>) -> bool {
        match self {
            Self::Object(obj) => obj.py_iadd(other, heap, self_id),
            Self::Str(s) => s.py_iadd(other, heap, self_id),
            Self::Bytes(b) => b.py_iadd(other, heap, self_id),
            Self::List(l) => l.py_iadd(other, heap, self_id),
            Self::Tuple(t) => t.py_iadd(other, heap, self_id),
            Self::Dict(d) => d.py_iadd(other, heap, self_id),
        }
    }

    fn py_call_attr(
        &mut self,
        heap: &mut Heap<'c, 'e>,
        attr: &Attr,
        args: ArgObjects<'c, 'e>,
    ) -> RunResult<'c, Object<'c, 'e>> {
        match self {
            Self::Object(obj) => obj.py_call_attr(heap, attr, args),
            Self::Str(s) => s.py_call_attr(heap, attr, args),
            Self::Bytes(b) => b.py_call_attr(heap, attr, args),
            Self::List(l) => l.py_call_attr(heap, attr, args),
            Self::Tuple(t) => t.py_call_attr(heap, attr, args),
            Self::Dict(d) => d.py_call_attr(heap, attr, args),
        }
    }

    fn py_getitem(&self, key: &Object<'c, 'e>, heap: &mut Heap<'c, 'e>) -> RunResult<'c, Object<'c, 'e>> {
        match self {
            Self::Object(obj) => obj.py_getitem(key, heap),
            Self::Str(s) => s.py_getitem(key, heap),
            Self::Bytes(b) => b.py_getitem(key, heap),
            Self::List(l) => l.py_getitem(key, heap),
            Self::Tuple(t) => t.py_getitem(key, heap),
            Self::Dict(d) => d.py_getitem(key, heap),
        }
    }

    fn py_setitem(&mut self, key: Object<'c, 'e>, value: Object<'c, 'e>, heap: &mut Heap<'c, 'e>) -> RunResult<'c, ()> {
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

/// Hash caching state stored alongside each heap entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HashState {
    /// Hash has not yet been computed but the object might be hashable.
    Unknown,
    /// Cached hash value for immutable types that have been hashed at least once.
    Cached(u64),
    /// Object is unhashable (mutable types or tuples containing unhashables).
    Unhashable,
}

impl HashState {
    fn for_data(data: &HeapData<'_, '_>) -> Self {
        match data {
            HeapData::Str(_) | HeapData::Bytes(_) | HeapData::Tuple(_) => Self::Unknown,
            _ => Self::Unhashable,
        }
    }
}

/// A single entry inside the heap arena, storing refcount, payload, and hash metadata.
///
/// The `hash_state` field tracks whether the heap entry is hashable and, if so,
/// caches the computed hash. Mutable types (List, Dict) start as `Unhashable` and
/// will raise TypeError if used as dict keys.
///
/// The `data` field is an Option to support temporary borrowing: when methods like
/// `with_entry_mut` or `call_attr` need mutable access to both the data and the heap,
/// they can `.take()` the data out (leaving `None`), pass `&mut Heap` to user code,
/// then restore the data. This avoids unsafe code while keeping `refcount` accessible
/// for `inc_ref`/`dec_ref` during the borrow.
#[derive(Debug)]
struct HeapObject<'c, 'e> {
    refcount: usize,
    /// The payload data. Temporarily `None` while borrowed via `with_entry_mut`/`call_attr`.
    data: Option<HeapData<'c, 'e>>,
    /// Current hashing status / cached hash value
    hash_state: HashState,
}

/// Reference-counted arena that backs all heap-only runtime objects.
///
/// Uses a free list to reuse slots from freed objects, keeping memory usage
/// constant for long-running loops that repeatedly allocate and free objects.
/// When an object is freed via `dec_ref`, its slot ID is added to the free list.
/// New allocations pop from the free list when available, otherwise append.
#[derive(Debug, Default)]
pub struct Heap<'c, 'e> {
    objects: Vec<Option<HeapObject<'c, 'e>>>,
    /// IDs of freed slots available for reuse. Populated by `dec_ref`, consumed by `allocate`.
    free_list: Vec<ObjectId>,
}

macro_rules! take_data {
    ($self:ident, $id:expr, $func_name:literal) => {
        $self
            .objects
            .get_mut($id)
            .expect(concat!("Heap::", $func_name, ": slot missing"))
            .as_mut()
            .expect(concat!("Heap::", $func_name, ": object already freed"))
            .data
            .take()
            .expect(concat!("Heap::", $func_name, ": data already borrowed"))
    };
}

macro_rules! restore_data {
    ($self:ident, $id:expr, $new_data:expr, $func_name:literal) => {{
        let entry = $self
            .objects
            .get_mut($id)
            .expect(concat!("Heap::", $func_name, ": slot missing"))
            .as_mut()
            .expect(concat!("Heap::", $func_name, ": object already freed"));
        entry.data = Some($new_data);
    }};
}

impl<'c, 'e> Heap<'c, 'e> {
    /// Allocates a new heap object, returning the identifier.
    ///
    /// Reuses freed slots from the free list when available, otherwise appends
    /// a new slot. This keeps memory usage constant for long-running loops.
    ///
    /// Hash computation is deferred until the object is used as a dict key
    /// (via `get_or_compute_hash`). This avoids computing hashes for objects
    /// that are never used as dict keys, improving allocation performance.
    pub fn allocate(&mut self, data: HeapData<'c, 'e>) -> ObjectId {
        let hash_state = HashState::for_data(&data);
        let new_object = HeapObject {
            refcount: 1,
            data: Some(data),
            hash_state,
        };

        if let Some(id) = self.free_list.pop() {
            // Reuse a freed slot
            self.objects[id] = Some(new_object);
            id
        } else {
            // No free slots, append new entry
            let id = self.objects.len();
            self.objects.push(Some(new_object));
            id
        }
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
    /// When an object is freed, its slot ID is added to the free list for reuse by
    /// future allocations. Uses recursion for child cleanup - avoiding repeated Vec
    /// allocations and benefiting from call stack locality.
    ///
    /// # Panics
    /// Panics if the object ID is invalid or the object has already been freed.
    pub fn dec_ref(&mut self, id: ObjectId) {
        let slot = self.objects.get_mut(id).expect("Heap::dec_ref: slot missing");
        let entry = slot.as_mut().expect("Heap::dec_ref: object already freed");
        if entry.refcount > 1 {
            entry.refcount -= 1;
        } else if let Some(object) = slot.take() {
            // refcount == 1, free the object and add slot to free list for reuse
            self.free_list.push(id);
            // Collect child IDs and mark Objects as Dereferenced (when dec-ref-check enabled)
            if let Some(mut data) = object.data {
                let mut child_ids = Vec::new();
                data.py_dec_ref_ids(&mut child_ids);
                drop(data);
                // Recursively decrement children
                for child_id in child_ids {
                    self.dec_ref(child_id);
                }
            }
        }
    }

    /// Returns an immutable reference to the heap data stored at the given ID.
    ///
    /// # Panics
    /// Panics if the object ID is invalid, the object has already been freed,
    /// or the data is currently borrowed via `with_entry_mut`/`call_attr`.
    #[must_use]
    pub fn get(&self, id: ObjectId) -> &HeapData<'c, 'e> {
        self.objects
            .get(id)
            .expect("Heap::get: slot missing")
            .as_ref()
            .expect("Heap::get: object already freed")
            .data
            .as_ref()
            .expect("Heap::get: data currently borrowed")
    }

    /// Returns a mutable reference to the heap data stored at the given ID.
    ///
    /// # Panics
    /// Panics if the object ID is invalid, the object has already been freed,
    /// or the data is currently borrowed via `with_entry_mut`/`call_attr`.
    pub fn get_mut(&mut self, id: ObjectId) -> &mut HeapData<'c, 'e> {
        self.objects
            .get_mut(id)
            .expect("Heap::get_mut: slot missing")
            .as_mut()
            .expect("Heap::get_mut: object already freed")
            .data
            .as_mut()
            .expect("Heap::get_mut: data currently borrowed")
    }

    /// Returns or computes the hash for the heap object at the given ID.
    ///
    /// Hashes are computed lazily on first use and then cached. Returns
    /// Some(hash) for immutable types (Str, Bytes, hashable Tuple), None
    /// for mutable types (List, Dict).
    ///
    /// # Panics
    /// Panics if the object ID is invalid or the object has already been freed.
    pub fn get_or_compute_hash(&mut self, id: ObjectId) -> Option<u64> {
        let entry = self
            .objects
            .get_mut(id)
            .expect("Heap::get_or_compute_hash: slot missing")
            .as_mut()
            .expect("Heap::get_or_compute_hash: object already freed");

        match entry.hash_state {
            HashState::Unhashable => return None,
            HashState::Cached(hash) => return Some(hash),
            HashState::Unknown => {}
        }

        // Compute hash lazily - need to temporarily take data to avoid borrow conflict
        let data = entry.data.take().expect("Heap::get_or_compute_hash: data borrowed");
        let hash = data.compute_hash_if_immutable(self);

        // Restore data and cache the hash if computed
        let entry = self
            .objects
            .get_mut(id)
            .expect("Heap::get_or_compute_hash: slot missing after compute")
            .as_mut()
            .expect("Heap::get_or_compute_hash: object freed during compute");
        entry.data = Some(data);
        entry.hash_state = match hash {
            Some(value) => HashState::Cached(value),
            None => HashState::Unhashable,
        };
        hash
    }

    /// Calls an attribute on the heap object at `id` while temporarily taking ownership
    /// of its payload so we can borrow the heap again inside the call. This avoids the
    /// borrow checker conflict that arises when attribute implementations also need
    /// mutable access to the heap (e.g. for refcounting).
    pub fn call_attr(&mut self, id: ObjectId, attr: &Attr, args: ArgObjects<'c, 'e>) -> RunResult<'c, Object<'c, 'e>> {
        // Take data out in a block so the borrow of self.objects ends
        let mut data = take_data!(self, id, "call_attr");

        let result = data.py_call_attr(self, attr, args);

        // Restore data
        let entry = self
            .objects
            .get_mut(id)
            .expect("Heap::call_attr: slot missing")
            .as_mut()
            .expect("Heap::call_attr: object already freed");
        entry.data = Some(data);
        result
    }

    /// Gives mutable access to a heap entry while allowing reentrant heap usage
    /// inside the closure (e.g. to read other objects or allocate results).
    ///
    /// The data is temporarily taken from the heap entry, so the closure can safely
    /// mutate both the entry data and the heap (e.g. to allocate new objects).
    /// The data is automatically restored after the closure completes.
    pub fn with_entry_mut<F, R>(&mut self, id: ObjectId, f: F) -> R
    where
        F: FnOnce(&mut Heap<'c, 'e>, &mut HeapData<'c, 'e>) -> R,
    {
        // Take data out in a block so the borrow of self.objects ends
        let mut data = take_data!(self, id, "with_entry_mut");

        let result = f(self, &mut data);

        // Restore data
        restore_data!(self, id, data, "with_entry_mut");
        result
    }

    /// Temporarily takes ownership of two heap entries so their data can be borrowed
    /// simultaneously while still permitting mutable access to the heap (e.g. to
    /// allocate results). Automatically restores both entries after the closure
    /// finishes executing.
    pub fn with_two<F, R>(&mut self, left: ObjectId, right: ObjectId, f: F) -> R
    where
        F: FnOnce(&mut Heap<'c, 'e>, &HeapData<'c, 'e>, &HeapData<'c, 'e>) -> R,
    {
        if left == right {
            // Same object - take data once and pass it twice
            let data = take_data!(self, left, "with_two");

            let result = f(self, &data, &data);

            restore_data!(self, left, data, "with_two");
            result
        } else {
            // Different objects - take both
            let left_data = take_data!(self, left, "with_two (left)");
            let right_data = take_data!(self, right, "with_two (right)");

            let result = f(self, &left_data, &right_data);

            // Restore in reverse order
            restore_data!(self, right, right_data, "with_two (right)");
            restore_data!(self, left, left_data, "with_two (left)");
            result
        }
    }

    /// Removes all objects and resets the ID counter, used between executor runs.
    pub fn clear(&mut self) {
        // When dec-ref-check is enabled, mark all contained Objects as Dereferenced
        // before clearing to prevent Drop panics. We use py_dec_ref_ids for this
        // since it handles the marking (we ignore the collected IDs since we're
        // clearing everything anyway).
        #[cfg(feature = "dec-ref-check")]
        {
            let mut dummy_stack = Vec::new();
            for object in self.objects.iter_mut().flatten() {
                if let Some(data) = &mut object.data {
                    data.py_dec_ref_ids(&mut dummy_stack);
                }
            }
        }
        self.objects.clear();
        self.free_list.clear();
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

    /// Helper for List in-place add: extends the destination vec with items from a heap list.
    ///
    /// This method exists to work around borrow checker limitations when List::py_iadd
    /// needs to read from one heap object while extending another. By keeping both
    /// the read and the refcount increments within Heap's impl block, we can use the
    /// take/restore pattern to avoid the lifetime propagation issues.
    ///
    /// Returns `true` if successful, `false` if the source ID is not a List.
    pub fn iadd_extend_list(&mut self, source_id: ObjectId, dest: &mut Vec<Object<'c, 'e>>) -> bool {
        // Take the source data temporarily
        let source_data = take_data!(self, source_id, "iadd_extend_list");

        let success = if let HeapData::List(list) = &source_data {
            // Copy items and track which refs need incrementing
            let items: Vec<Object<'c, 'e>> = list.as_vec().iter().map(Object::copy_for_extend).collect();
            let ref_ids: Vec<ObjectId> = items
                .iter()
                .filter_map(|obj| if let Object::Ref(id) = obj { Some(*id) } else { None })
                .collect();

            // Restore source data before mutating heap (inc_ref needs it)
            restore_data!(self, source_id, source_data, "iadd_extend_list");

            // Now increment refcounts
            for id in ref_ids {
                self.inc_ref(id);
            }

            // Extend destination
            dest.extend(items);
            true
        } else {
            // Not a list, restore and return false
            restore_data!(self, source_id, source_data, "iadd_extend_list");
            false
        };

        success
    }
}

/// Drop implementation for Heap that marks all contained Objects as Dereferenced
/// before dropping to prevent panics when the `dec-ref-check` feature is enabled.
#[cfg(feature = "dec-ref-check")]
impl Drop for Heap<'_, '_> {
    fn drop(&mut self) {
        // Mark all contained Objects as Dereferenced before dropping.
        // We use py_dec_ref_ids for this since it handles the marking
        // (we ignore the collected IDs since we're dropping everything anyway).
        let mut dummy_stack = Vec::new();
        for object in self.objects.iter_mut().flatten() {
            if let Some(data) = &mut object.data {
                data.py_dec_ref_ids(&mut dummy_stack);
            }
        }
    }
}
