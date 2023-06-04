use std::fmt;

use crate::exceptions::exc_err;
use crate::parse_error::{ParseError, ParseResult};

// this is a temporary hack
#[derive(Debug, Clone)]
pub(crate) enum Builtins {
    Print,
    Range,
    Len,
}

impl fmt::Display for Builtins {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Print => write!(f, "print"),
            Self::Range => write!(f, "range"),
            Self::Len => write!(f, "len"),
        }
    }
}

impl Builtins {
    pub fn find(name: &str) -> ParseResult<'static, Self> {
        match name {
            "print" => Ok(Self::Print),
            "range" => Ok(Self::Range),
            "len" => Ok(Self::Len),
            _ => exc_err!(ParseError::Internal; "unknown builtin: {name}"),
        }
    }

    /// whether the function has side effects
    pub fn side_effects(&self) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match self {
            Self::Print => true,
            _ => false,
        }
    }
}
