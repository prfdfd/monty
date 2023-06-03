use crate::exceptions::{ExceptionRaise, InternalRunError, RunError};
use std::borrow::Cow;
use std::fmt;

#[derive(Debug, Clone)]
pub enum ParseError<'c> {
    Todo(&'c str),
    Parsing(String),
    Internal(Cow<'c, str>),
    PreEvalExc(ExceptionRaise<'c>),
    PreEvalInternal(InternalRunError),
}

impl<'c> fmt::Display for ParseError<'c> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Todo(s) => write!(f, "TODO: {s}"),
            Self::Internal(s) => write!(f, "Internal parsing error: {s}"),
            Self::Parsing(s) => write!(f, "Error parsing AST: {s}"),
            Self::PreEvalExc(s) => write!(f, "Pre eval exception: {s}"),
            Self::PreEvalInternal(s) => write!(f, "Pre eval internal error: {s}"),
        }
    }
}

// TODO change to From
impl<'c> ParseError<'c> {
    pub(crate) fn pre_eval(run_error: RunError<'c>) -> Self {
        match run_error {
            RunError::Exc(e) => Self::PreEvalExc(e),
            RunError::Internal(e) => Self::PreEvalInternal(e),
        }
    }
}

pub type ParseResult<'c, T> = Result<T, ParseError<'c>>;

impl<'c> From<InternalRunError> for ParseError<'c> {
    fn from(internal_run_error: InternalRunError) -> Self {
        Self::PreEvalInternal(internal_run_error)
    }
}
