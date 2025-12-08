use std::borrow::Cow;
use std::fmt;

use crate::exceptions::{ExceptionRaise, InternalRunError, RunError};
use crate::resource::ResourceError;

#[derive(Debug, Clone)]
pub enum ParseError<'c> {
    Todo(&'c str),
    Parsing(String),
    Internal(Cow<'static, str>),
    PreEvalExc(ExceptionRaise<'c>),
    PreEvalInternal(InternalRunError),
    PreEvalResource(ResourceError),
}

impl fmt::Display for ParseError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Todo(s) => write!(f, "TODO: {s}"),
            Self::Internal(s) => write!(f, "Internal parsing error: {s}"),
            Self::Parsing(s) => write!(f, "Error parsing AST: {s}"),
            Self::PreEvalExc(s) => write!(f, "Pre eval exception: {s}"),
            Self::PreEvalInternal(s) => write!(f, "Pre eval internal error: {s}"),
            Self::PreEvalResource(s) => write!(f, "Pre eval resource error: {s}"),
        }
    }
}

impl<'c> From<RunError<'c>> for ParseError<'c> {
    fn from(run_error: RunError<'c>) -> Self {
        match run_error {
            RunError::Exc(e) => Self::PreEvalExc(e),
            RunError::Internal(e) => Self::PreEvalInternal(e),
            RunError::Resource(e) => Self::PreEvalResource(e),
        }
    }
}

impl<'c> From<ExceptionRaise<'c>> for ParseError<'c> {
    fn from(exc: ExceptionRaise<'c>) -> Self {
        Self::PreEvalExc(exc)
    }
}

impl From<InternalRunError> for ParseError<'_> {
    fn from(internal_run_error: InternalRunError) -> Self {
        Self::PreEvalInternal(internal_run_error)
    }
}

impl ParseError<'_> {
    #[must_use]
    pub fn summary(&self) -> String {
        match self {
            Self::Todo(s) => format!("TODO: {s}"),
            Self::Internal(s) => format!("Internal: {s}"),
            Self::Parsing(s) => format!("AST: {s}"),
            Self::PreEvalExc(s) => format!("Exc: {}", s.summary()),
            Self::PreEvalInternal(s) => format!("Eval Internal: {s}"),
            Self::PreEvalResource(s) => format!("Resource: {s}"),
        }
    }
}
