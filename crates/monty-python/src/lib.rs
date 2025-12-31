//! Python bindings for the Monty sandboxed Python interpreter.
//!
//! This module provides a Python interface to Monty, allowing execution of
//! sandboxed Python code with configurable resource limits and external
//! function callbacks.
use std::borrow::Cow;
use std::fmt::Write;

mod convert;
mod exceptions;
mod external;
mod limits;

// Use `::monty` to refer to the external crate (not the pymodule)
use ::monty::{LimitedTracker, NoLimitTracker, PrintWriter, ResourceTracker, RunProgress, RunSnapshot, StdPrint};
use pyo3::exceptions::{PyKeyError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use convert::{monty_to_py, py_to_monty};
use exceptions::monty_exception_to_py;
use external::ExternalFunctionRegistry;
pub use limits::PyResourceLimits;

/// Monty - A sandboxed Python interpreter written in Rust.
///
/// This module provides a fast, safe way to execute Python code
/// with configurable resource limits and external function callbacks.
#[pymodule]
mod monty {
    #[pymodule_export]
    use super::PyMonty as Monty;

    #[pymodule_export]
    use super::PyResourceLimits as ResourceLimits;
}

/// A sandboxed Python interpreter instance.
///
/// Parses and compiles Python code on initialization, then can be run
/// multiple times with different input values. This separates the parsing
/// cost from execution, making repeated runs more efficient.
#[pyclass(name = "Monty")]
pub struct PyMonty {
    /// The compiled code snapshot, ready to execute.
    runner: RunSnapshot,
    /// The artificial name of the python code "file"
    file_name: String,
    /// Names of input variables expected by the code.
    input_names: Vec<String>,
    /// Names of external functions the code can call.
    external_function_names: Vec<String>,
}

#[pymethods]
impl PyMonty {
    /// Creates a new Monty interpreter by parsing the given code.
    ///
    /// # Arguments
    /// * `code` - Python code to execute
    /// * `inputs` - List of input variable names available in the code
    /// * `external_functions` - List of external function names the code can call
    ///
    /// # Raises
    /// `SyntaxError` if the code cannot be parsed
    #[new]
    #[pyo3(signature = (code, *, file_name="main.py", inputs=None, external_functions=None))]
    fn new(
        code: String,
        file_name: &str,
        inputs: Option<&Bound<'_, PyList>>,
        external_functions: Option<&Bound<'_, PyList>>,
    ) -> PyResult<Self> {
        let input_names = list_str(inputs, "inputs")?;
        let external_function_names = list_str(external_functions, "external_functions")?;

        // Create the snapshot (parses the code)
        let runner = RunSnapshot::new(code, file_name, input_names.clone(), external_function_names.clone())
            .map_err(monty_exception_to_py)?;

        Ok(Self {
            runner,
            file_name: file_name.to_string(),
            input_names,
            external_function_names,
        })
    }

    /// Executes the code and returns the result.
    ///
    /// # Arguments
    /// * `inputs` - Dict of input variable values (must match names from `__init__`)
    /// * `limits` - Optional `ResourceLimits` configuration
    /// * `external_functions` - Dict of external function callbacks (must match names from `__init__`)
    ///
    /// # Returns
    /// The result of the last expression in the code
    ///
    /// # Raises
    /// Various Python exceptions matching what the code would raise
    #[pyo3(signature = (*, inputs=None, limits=None, external_functions=None, print_callback=None))]
    fn run(
        &self,
        py: Python<'_>,
        inputs: Option<&Bound<'_, PyDict>>,
        limits: Option<&PyResourceLimits>,
        external_functions: Option<&Bound<'_, PyDict>>,
        print_callback: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Py<PyAny>> {
        // Extract input values in the order they were declared
        let input_values = self.extract_input_values(inputs)?;

        /// if there are no external functions, run the code without a snapshotting for better performance
        macro_rules! run_code {
            ($resource_tracker:expr, $print_output:expr) => {{
                if self.external_function_names.is_empty() {
                    match self
                        .runner
                        .run_no_snapshot(input_values, $resource_tracker, &mut $print_output)
                    {
                        Ok(v) => monty_to_py(py, v),
                        Err(err) => Err(monty_exception_to_py(err)),
                    }
                } else {
                    // Clone the snapshot since run_snapshot methods consume it - allows reuse of the parsed code
                    let progress = self
                        .runner
                        .clone()
                        .run_snapshot(input_values, $resource_tracker, &mut $print_output)
                        .map_err(monty_exception_to_py)?;
                    execute_progress(py, progress, external_functions, &mut $print_output)?
                }
            }};
        }

        // separate code paths due to generics
        match (limits, print_callback) {
            (Some(limits), Some(callback)) => {
                run_code!(
                    LimitedTracker::new(limits.to_monty_limits()),
                    CallbackStringPrint(callback)
                )
            }
            (Some(limits), None) => {
                run_code!(LimitedTracker::new(limits.to_monty_limits()), StdPrint)
            }
            (None, Some(callback)) => {
                run_code!(NoLimitTracker::default(), CallbackStringPrint(callback))
            }
            (None, None) => {
                run_code!(NoLimitTracker::default(), StdPrint)
            }
        }
    }

    fn __repr__(&self) -> String {
        let lines = self.runner.code().lines().count();
        let mut s = format!(
            "Monty(<{} line{} of code>, file_name='{}'",
            lines,
            if lines == 1 { "" } else { "s" },
            self.file_name
        );
        if !self.input_names.is_empty() {
            write!(s, ", inputs={:?}", self.input_names).unwrap();
        }
        if !self.external_function_names.is_empty() {
            write!(s, ", external_functions={:?}", self.external_function_names).unwrap();
        }
        s.push(')');
        s
    }
}

impl PyMonty {
    /// Extracts input values from the dict in the order they were declared.
    ///
    /// Validates that all required inputs are provided and no extra inputs are given.
    fn extract_input_values(&self, inputs: Option<&Bound<'_, PyDict>>) -> PyResult<Vec<::monty::PyObject>> {
        if self.input_names.is_empty() {
            if inputs.is_some() {
                return Err(PyTypeError::new_err(
                    "No input variables declared but inputs dict was provided",
                ));
            }
            return Ok(vec![]);
        }

        let Some(inputs) = inputs else {
            return Err(PyTypeError::new_err(format!(
                "Missing required inputs: {:?}",
                self.input_names
            )));
        };

        // Extract values in declaration order
        self.input_names
            .iter()
            .map(|name| {
                let value = inputs
                    .get_item(name)?
                    .ok_or_else(|| PyKeyError::new_err(format!("Missing required input: '{name}'")))?;
                py_to_monty(&value)
            })
            .collect::<PyResult<_>>()
    }
}

/// Executes the `RunProgress` loop, handling external function calls.
///
/// Uses a generic type to handle both `NoLimitTracker` and `LimitedTracker`.
fn execute_progress<T: ResourceTracker>(
    py: Python<'_>,
    mut progress: RunProgress<T>,
    external_functions: Option<&Bound<'_, PyDict>>,
    print_output: &mut impl PrintWriter,
) -> PyResult<PyResult<Py<PyAny>>> {
    loop {
        match progress {
            RunProgress::Complete(result) => {
                return Ok(monty_to_py(py, result));
            }
            RunProgress::FunctionCall {
                function_name,
                args,
                kwargs,
                state,
            } => {
                let registry = external_functions
                    .map(|d| ExternalFunctionRegistry::new(py, d))
                    .ok_or_else(|| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "External function '{function_name}' called but no external_functions provided"
                        ))
                    })?;

                let return_value = registry.call(&function_name, args, kwargs);

                progress = state.run(return_value, print_output).map_err(monty_exception_to_py)?;
            }
        }
    }
}

fn list_str(arg: Option<&Bound<'_, PyList>>, name: &str) -> PyResult<Vec<String>> {
    if let Some(names) = arg {
        names
            .iter()
            .map(|item| item.extract::<String>())
            .collect::<PyResult<Vec<_>>>()
            .map_err(|e| PyTypeError::new_err(format!("{name}: {e}")))
    } else {
        Ok(vec![])
    }
}

#[derive(Debug)]
pub struct CallbackStringPrint<'py>(&'py Bound<'py, PyAny>);

impl PrintWriter for CallbackStringPrint<'_> {
    fn stdout_write(&mut self, output: Cow<'_, str>) {
        // TODO PrintWriter needs to return a RunResult
        let s = output.into_pyobject(self.0.py()).unwrap();
        self.0.call1(("stdout", s)).unwrap();
    }

    fn stdout_push(&mut self, end: char) {
        let s = end.into_pyobject(self.0.py()).unwrap();
        self.0.call1(("stdout", s)).unwrap();
    }
}
