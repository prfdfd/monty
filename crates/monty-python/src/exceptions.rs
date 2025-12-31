//! Exception mapping between Monty and Python.
//!
//! Converts Monty's `PythonException` and `ExcType` to PyO3's `PyErr`
//! so that Python code sees native Python exceptions.

use ::monty::{ExcType, PythonException};
use pyo3::exceptions;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;

/// Converts Monty's `PythonException` to a Python exception.
///
/// Creates an appropriate Python exception type with the message.
/// The traceback information is included in the exception message
/// since PyO3 doesn't provide direct traceback manipulation.
pub fn monty_exception_to_py(exc: PythonException) -> PyErr {
    // Include traceback in the message if available
    let msg = if exc.traceback.is_empty() {
        exc.message.unwrap_or_default()
    } else {
        // Use the Display impl of PythonException which formats the full traceback
        exc.to_string()
    };

    create_py_exception(exc.exc_type, Some(msg))
}

/// Creates a Python exception from Monty's exception type and message.
///
/// Maps all `ExcType` variants to their corresponding PyO3 exception types.
pub fn create_py_exception(exc_type: ExcType, arg: Option<String>) -> PyErr {
    let msg = arg.unwrap_or_default();

    match exc_type {
        ExcType::Exception => exceptions::PyException::new_err(msg),
        ExcType::ArithmeticError => exceptions::PyArithmeticError::new_err(msg),
        ExcType::OverflowError => exceptions::PyOverflowError::new_err(msg),
        ExcType::ZeroDivisionError => exceptions::PyZeroDivisionError::new_err(msg),
        ExcType::LookupError => exceptions::PyLookupError::new_err(msg),
        ExcType::IndexError => exceptions::PyIndexError::new_err(msg),
        ExcType::KeyError => exceptions::PyKeyError::new_err(msg),
        ExcType::RuntimeError => exceptions::PyRuntimeError::new_err(msg),
        ExcType::NotImplementedError => exceptions::PyNotImplementedError::new_err(msg),
        ExcType::RecursionError => exceptions::PyRecursionError::new_err(msg),
        ExcType::AssertionError => exceptions::PyAssertionError::new_err(msg),
        ExcType::AttributeError => exceptions::PyAttributeError::new_err(msg),
        ExcType::MemoryError => exceptions::PyMemoryError::new_err(msg),
        ExcType::NameError => exceptions::PyNameError::new_err(msg),
        ExcType::SyntaxError => exceptions::PySyntaxError::new_err(msg),
        ExcType::TimeoutError => exceptions::PyTimeoutError::new_err(msg),
        ExcType::TypeError => exceptions::PyTypeError::new_err(msg),
        ExcType::ValueError => exceptions::PyValueError::new_err(msg),
    }
}

/// Converts a Python exception to Monty's `PyObject::Exception`.
pub fn exc_to_monty_object(exc: &Bound<'_, PyException>) -> ::monty::PyObject {
    let exc_type = py_err_to_exc_type(exc);
    let arg = exc.str().ok().map(|s| s.to_string_lossy().into_owned());

    ::monty::PyObject::Exception { exc_type, arg }
}

/// Maps a Python exception type to Monty's `ExcType` enum.
///
/// NOTE: order matters here!
fn py_err_to_exc_type(exc: &Bound<'_, PyException>) -> ExcType {
    if exc.cast::<exceptions::PyArithmeticError>().is_ok() {
        ExcType::ZeroDivisionError
    } else if exc.cast::<exceptions::PyAssertionError>().is_ok() {
        ExcType::AssertionError
    } else if exc.cast::<exceptions::PyAttributeError>().is_ok() {
        ExcType::AttributeError
    } else if exc.cast::<exceptions::PyMemoryError>().is_ok() {
        ExcType::MemoryError
    } else if exc.cast::<exceptions::PyNameError>().is_ok() {
        ExcType::NameError
    } else if exc.cast::<exceptions::PySyntaxError>().is_ok() {
        ExcType::SyntaxError
    } else if exc.cast::<exceptions::PyTimeoutError>().is_ok() {
        ExcType::TimeoutError
    } else if exc.cast::<exceptions::PyTypeError>().is_ok() {
        ExcType::TypeError
    } else if exc.cast::<exceptions::PyValueError>().is_ok() {
        ExcType::ValueError
    } else if exc.cast::<exceptions::PyRuntimeError>().is_ok() {
        ExcType::RuntimeError
    } else {
        ExcType::Exception
    }
}
