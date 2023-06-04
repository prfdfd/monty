use std::fmt;

use crate::exceptions::ExceptionRaise;

use crate::builtins::Builtins;
use crate::object::Object;
use crate::operators::{CmpOperator, Operator};
use crate::parse::CodeRange;

#[derive(Debug, Clone)]
pub(crate) struct Identifier<'c> {
    pub position: CodeRange<'c>,
    pub name: String, // TODO could this a `&'c str` or cow?
    pub id: usize,
}

impl<'c> Identifier<'c> {
    pub fn from_name(name: String, position: CodeRange<'c>) -> Self {
        Self { name, position, id: 0 }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Kwarg<'c> {
    pub key: Identifier<'c>,
    pub value: ExprLoc<'c>,
}

#[derive(Debug, Clone)]
pub(crate) enum Function<'c> {
    Builtin(Builtins),
    Ident(Identifier<'c>),
}

impl<'c> fmt::Display for Function<'c> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Builtin(b) => write!(f, "{b}"),
            Self::Ident(i) => write!(f, "{}", i.name),
        }
    }
}

impl<'c> Function<'c> {
    /// whether the function has side effects
    pub fn side_effects(&self) -> bool {
        match self {
            Self::Builtin(b) => b.side_effects(),
            _ => true,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Expr<'c> {
    Constant(Object),
    Name(Identifier<'c>),
    Call {
        func: Function<'c>,
        args: Vec<ExprLoc<'c>>,
        kwargs: Vec<Kwarg<'c>>,
    },
    Op {
        left: Box<ExprLoc<'c>>,
        op: Operator,
        right: Box<ExprLoc<'c>>,
    },
    CmpOp {
        left: Box<ExprLoc<'c>>,
        op: CmpOperator,
        right: Box<ExprLoc<'c>>,
    },
    #[allow(dead_code)]
    List(Vec<ExprLoc<'c>>),
}

impl<'c> fmt::Display for Expr<'c> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Constant(object) => write!(f, "{}", object.repr()),
            Self::Name(identifier) => write!(f, "{}", identifier.name),
            Self::Call { func, args, kwargs } => {
                write!(f, "{func}(")?;
                for arg in args.iter() {
                    write!(f, "{arg}, ")?;
                }
                for kwarg in kwargs.iter() {
                    write!(f, "{}={}, ", kwarg.key.name, kwarg.value)?;
                }
                write!(f, ")")
            }
            Self::Op { left, op, right } => write!(f, "{left} {op} {right}"),
            Self::CmpOp { left, op, right } => write!(f, "{left} {op} {right}"),
            Self::List(list) => {
                write!(f, "[")?;
                for item in list.iter() {
                    write!(f, "{item}, ")?;
                }
                write!(f, "]")
            }
        }
    }
}

impl<'c> Expr<'c> {
    pub fn is_const(&self) -> bool {
        matches!(self, Self::Constant(_))
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Self::Constant(Object::None))
    }

    pub fn into_object(self) -> Object {
        match self {
            Self::Constant(object) => object,
            _ => panic!("into_const can only be called on Constant expression."),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ExprLoc<'c> {
    pub position: CodeRange<'c>,
    pub expr: Expr<'c>,
}

impl<'c> fmt::Display for ExprLoc<'c> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // don't show position as that should be displayed separately
        write!(f, "{}", self.expr)
    }
}

impl<'c> ExprLoc<'c> {
    pub fn new(position: CodeRange<'c>, expr: Expr<'c>) -> Self {
        Self { position, expr }
    }
}

// TODO need a new AssignTo (enum of identifier, tuple) type used for "Assign" and "For"

#[derive(Debug, Clone)]
pub(crate) enum Node<'c> {
    Pass,
    Expr(ExprLoc<'c>),
    Return(ExprLoc<'c>),
    ReturnNone,
    Assign {
        target: Identifier<'c>,
        object: ExprLoc<'c>,
    },
    OpAssign {
        target: Identifier<'c>,
        op: Operator,
        object: ExprLoc<'c>,
    },
    For {
        target: Identifier<'c>,
        iter: ExprLoc<'c>,
        body: Vec<Node<'c>>,
        or_else: Vec<Node<'c>>,
    },
    If {
        test: ExprLoc<'c>,
        body: Vec<Node<'c>>,
        or_else: Vec<Node<'c>>,
    },
}

#[derive(Debug)]
pub enum Exit<'c> {
    ReturnNone,
    Return(Object),
    // Yield(Object),
    Raise(ExceptionRaise<'c>),
}

impl<'c> fmt::Display for Exit<'c> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReturnNone => write!(f, "None"),
            Self::Return(v) => write!(f, "{v}"),
            Self::Raise(exc) => write!(f, "{exc}"),
        }
    }
}
