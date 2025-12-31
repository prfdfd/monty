//! Python wrapper for Monty's `ResourceLimits`.
//!
//! Provides a Python class to configure resource limits for code execution,
//! including time limits, memory limits, and recursion depth.

use pyo3::prelude::*;
use std::time::Duration;

/// Configuration for resource limits during code execution.
///
/// All limits are optional. Set to `None` to disable a specific limit.
///
/// # Example
/// ```python
/// import monty
///
/// # Create limits with a 5 second timeout and 1MB memory limit
/// limits = monty.ResourceLimits(
///     max_duration_secs=5.0,
///     max_memory=1024 * 1024,
/// )
/// result = monty.run("...", limits=limits)
/// ```
#[pyclass(name = "ResourceLimits")]
#[derive(Debug, Clone, Default)]
pub struct PyResourceLimits {
    /// Maximum number of heap allocations allowed.
    #[pyo3(get, set)]
    pub max_allocations: Option<usize>,

    /// Maximum execution time in seconds.
    #[pyo3(get, set)]
    pub max_duration_secs: Option<f64>,

    /// Maximum heap memory in bytes.
    #[pyo3(get, set)]
    pub max_memory: Option<usize>,

    /// Run garbage collection every N allocations.
    #[pyo3(get, set)]
    pub gc_interval: Option<usize>,

    /// Maximum function call stack depth (default: 1000).
    #[pyo3(get, set)]
    pub max_recursion_depth: Option<usize>,
}

#[pymethods]
impl PyResourceLimits {
    /// Creates a new `ResourceLimits` configuration.
    ///
    /// # Arguments
    /// * `max_allocations` - Maximum number of heap allocations
    /// * `max_duration_secs` - Maximum execution time in seconds
    /// * `max_memory` - Maximum heap memory in bytes
    /// * `gc_interval` - Run garbage collection every N allocations
    /// * `max_recursion_depth` - Maximum function call depth (default: 1000)
    #[new]
    #[pyo3(signature = (
        *,
        max_allocations=None,
        max_duration_secs=None,
        max_memory=None,
        gc_interval=None,
        max_recursion_depth=Some(1000)
    ))]
    fn new(
        max_allocations: Option<usize>,
        max_duration_secs: Option<f64>,
        max_memory: Option<usize>,
        gc_interval: Option<usize>,
        max_recursion_depth: Option<usize>,
    ) -> Self {
        Self {
            max_allocations,
            max_duration_secs,
            max_memory,
            gc_interval,
            max_recursion_depth,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "ResourceLimits(max_allocations={}, max_duration_secs={}, max_memory={}, gc_interval={}, max_recursion_depth={})",
            format_option(self.max_allocations),
            format_option_f64(self.max_duration_secs),
            format_option(self.max_memory),
            format_option(self.gc_interval),
            format_option(self.max_recursion_depth),
        )
    }
}

impl PyResourceLimits {
    /// Converts to Monty's `ResourceLimits` type.
    #[must_use]
    pub fn to_monty_limits(&self) -> monty::ResourceLimits {
        let mut limits = monty::ResourceLimits::new().max_recursion_depth(self.max_recursion_depth);

        if let Some(max) = self.max_allocations {
            limits = limits.max_allocations(max);
        }
        if let Some(secs) = self.max_duration_secs {
            limits = limits.max_duration(Duration::from_secs_f64(secs));
        }
        if let Some(max) = self.max_memory {
            limits = limits.max_memory(max);
        }
        if let Some(interval) = self.gc_interval {
            limits = limits.gc_interval(interval);
        }
        limits
    }
}

/// Formats an Option<usize> for Python repr.
fn format_option(opt: Option<usize>) -> String {
    match opt {
        Some(v) => v.to_string(),
        None => "None".to_string(),
    }
}

/// Formats an Option<f64> for Python repr.
fn format_option_f64(opt: Option<f64>) -> String {
    match opt {
        Some(v) => v.to_string(),
        None => "None".to_string(),
    }
}
