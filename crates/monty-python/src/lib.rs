//! Python bindings for the Monty sandboxed Python interpreter.
//!
//! This module provides a Python interface to Monty, allowing execution of
//! sandboxed Python code with configurable resource limits and external
//! function callbacks.

mod convert;
mod exceptions;
mod external;
mod limits;
mod monty_cls;

// Use `::monty` to refer to the external crate (not the pymodule)
use pyo3::prelude::*;

pub use limits::PyResourceLimits;
pub use monty_cls::{PyMonty, PyMontyComplete, PyMontySnapshot};

/// Monty - A sandboxed Python interpreter written in Rust.
#[pymodule]
mod monty {
    #[pymodule_export]
    use super::PyMonty as Monty;

    #[pymodule_export]
    use super::PyMontySnapshot as MontySnapshot;

    #[pymodule_export]
    use super::PyMontyComplete as MontyComplete;

    #[pymodule_export]
    use super::PyResourceLimits as ResourceLimits;
}
