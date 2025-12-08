use crate::exceptions::{ExcType, SimpleException};
use crate::expressions::{Identifier, NameScope};
use crate::heap::{Heap, HeapId};
use crate::resource::ResourceTracker;
use crate::run::RunResult;
use crate::value::Value;

/// Index for the global (module-level) namespace in Namespaces.
/// At module level, local_idx == GLOBAL_NS_IDX (same namespace).
pub const GLOBAL_NS_IDX: usize = 0;

/// Storage for all namespaces during execution.
///
/// This struct owns all namespace data, allowing safe mutable access through indices.
/// Index 0 is always the global (module-level) namespace.
///
/// # Design Rationale
///
/// Instead of using raw pointers to share namespace access between frames,
/// we use indices into this central namespaces. Since variable scope (Local vs Global)
/// is known at compile time, we only ever need one mutable reference at a time.
///
/// # Closure Support
///
/// Variables captured by closures are stored in cells on the heap, not in namespaces.
/// The `get_var_value` method handles both namespace-based and cell-based variable access.
#[derive(Debug)]
pub struct Namespaces<'c, 'e> {
    namespaces: Vec<Vec<Value<'c, 'e>>>,
}

impl<'c, 'e> Namespaces<'c, 'e> {
    /// Creates namespaces with the global namespace initialized.
    ///
    /// The global namespace is always at index 0.
    pub fn new(namespace: Vec<Value<'c, 'e>>) -> Self {
        Self {
            namespaces: vec![namespace],
        }
    }

    /// Gets an immutable slice reference to a namespace by index.
    ///
    /// Used for reading from the enclosing namespace when defining closures,
    /// without requiring mutable access.
    ///
    /// # Panics
    /// Panics if `idx` is out of bounds.
    pub fn get(&self, idx: usize) -> &[Value<'c, 'e>] {
        self.namespaces[idx].as_slice()
    }

    /// Gets a mutable slice reference to a namespace by index.
    ///
    /// # Panics
    /// Panics if `idx` is out of bounds.
    pub fn get_mut(&mut self, idx: usize) -> &mut [Value<'c, 'e>] {
        self.namespaces[idx].as_mut_slice()
    }

    /// Creates a new namespace for a function call, returns its index.
    ///
    /// The new namespace is initialized with `Object::Undefined` values.
    /// Call `pop_with_heap()` when the function returns to clean up.
    pub fn push(&mut self, namespace: Vec<Value<'c, 'e>>) -> usize {
        let idx = self.namespaces.len();
        self.namespaces.push(namespace);
        idx
    }

    /// Removes the most recently added namespace (after function returns),
    /// properly cleaning up any heap-allocated values.
    ///
    /// This method decrements reference counts for any `Value::Ref` entries
    /// in the namespace before removing it.
    ///
    /// # Panics
    /// Panics if attempting to pop the global namespace (index 0).
    pub fn pop_with_heap<T: ResourceTracker>(&mut self, heap: &mut Heap<'c, 'e, T>) {
        debug_assert!(self.namespaces.len() > 1, "cannot pop global namespace");
        if let Some(namespace) = self.namespaces.pop() {
            for value in namespace {
                value.drop_with_heap(heap);
            }
        }
    }

    /// Cleans up the global namespace by dropping all values with proper ref counting.
    ///
    /// Call this before the namespaces is dropped to properly decrement reference counts
    /// for any `Value::Ref` entries in the global namespace.
    ///
    /// Only needed when `dec-ref-check` is enabled, since the Drop impl panics on unfreed Refs.
    #[cfg(feature = "dec-ref-check")]
    pub fn drop_global_with_heap<T: ResourceTracker>(&mut self, heap: &mut Heap<'c, 'e, T>) {
        let global = self.get_mut(GLOBAL_NS_IDX);
        for value in global.iter_mut() {
            let v = std::mem::replace(value, Value::Undefined);
            v.drop_with_heap(heap);
        }
    }

    /// Looks up a variable by name in the appropriate namespace based on the scope index.
    ///
    /// # Arguments
    /// * `local_idx` - Index of the local namespace in namespaces
    /// * `ident` - The identifier to look up (contains heap_id and scope)
    ///
    /// # Returns
    /// A mutable reference to the Value at the identifier's location, or NameError if undefined.
    pub fn get_var_mut(&mut self, local_idx: usize, ident: &Identifier<'c>) -> RunResult<'c, &mut Value<'c, 'e>> {
        let ns_idx = match ident.scope {
            NameScope::Local => local_idx,
            NameScope::Global => GLOBAL_NS_IDX,
            NameScope::Cell => {
                // Cell access should use get_var_value which handles cell dereferencing
                panic!("Cell access should use get_var_value, not get_var_mut");
            }
        };
        let namespace = self.get_mut(ns_idx);

        if let Some(value) = namespace.get_mut(ident.heap_id()) {
            if !matches!(value, Value::Undefined) {
                return Ok(value);
            }
        }
        Err(SimpleException::new(ExcType::NameError, Some(ident.name.into()))
            .with_position(ident.position)
            .into())
    }

    /// Gets a variable's value, handling Local, Global, and Cell scopes.
    ///
    /// This is the primary method for reading variable values during expression evaluation.
    /// It handles all scope types:
    /// - `Local` - reads directly from the local namespace
    /// - `Global` - reads directly from the global namespace (index 0)
    /// - `Cell` - namespace slot contains `Value::Ref(cell_id)`, reads through the cell
    ///
    /// # Arguments
    /// * `local_idx` - Index of the local namespace in namespaces
    /// * `heap` - The heap for cell access and cloning ref-counted values
    /// * `ident` - The identifier to look up (contains heap_id and scope)
    ///
    /// # Returns
    /// A cloned copy of the value (with refcount incremented for Ref values), or NameError if undefined.
    pub fn get_var_value<T: ResourceTracker>(
        &mut self,
        local_idx: usize,
        heap: &mut Heap<'c, 'e, T>,
        ident: &Identifier<'c>,
    ) -> RunResult<'c, Value<'c, 'e>> {
        // Determine which namespace to use
        let ns_idx = match ident.scope {
            NameScope::Global => GLOBAL_NS_IDX,
            _ => local_idx, // Local and Cell both use local namespace
        };

        match ident.scope {
            NameScope::Cell => {
                // Cell access - namespace slot contains Value::Ref(cell_id)
                let namespace = &self.namespaces[ns_idx];
                if let Value::Ref(cell_id) = namespace[ident.heap_id()] {
                    let value = heap.get_cell_value(cell_id);
                    // Cell may be undefined if accessed before assignment in enclosing scope
                    if matches!(value, Value::Undefined) {
                        Err(ExcType::name_error_free_variable(ident.name).into())
                    } else {
                        Ok(value)
                    }
                } else {
                    panic!("Cell variable slot doesn't contain a cell reference - prepare-time bug");
                }
            }
            _ => {
                // Local or Global scope - direct namespace access
                self.get_var_mut(ns_idx, ident)
                    .map(|object| object.clone_with_heap(heap))
            }
        }
    }

    /// Returns the global namespace for final inspection (e.g., ref-count testing).
    ///
    /// Consumes the namespaces since the namespace Vec is moved out.
    ///
    /// Only available when the `ref-counting` feature is enabled.
    #[cfg(feature = "ref-counting")]
    pub fn into_global(mut self) -> Vec<Value<'c, 'e>> {
        self.namespaces.swap_remove(GLOBAL_NS_IDX)
    }

    /// Returns an iterator over all HeapIds referenced by values in all namespaces.
    ///
    /// This is used by garbage collection to find all root references. Any heap
    /// object reachable from these roots should not be collected.
    pub fn iter_heap_ids(&self) -> impl Iterator<Item = HeapId> + '_ {
        self.namespaces.iter().flat_map(|namespace| {
            namespace
                .iter()
                .filter_map(|value| if let Value::Ref(id) = value { Some(*id) } else { None })
        })
    }
}
