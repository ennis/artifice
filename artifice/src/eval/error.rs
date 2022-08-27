use crate::{
    eval::TaskError,
    model::{Path, TryFromValueError},
};
use kyute_common::Atom;
use std::{
    fmt,
    fmt::{write, Formatter},
    io,
    io::Error,
    sync::Arc,
};
use thiserror::Error;

/// The different kinds of evaluation error.
#[derive(Clone, Debug)]
pub enum EvalError {
    /// Unspecified error.
    Unspecified,
    /// The resulting value wasn't of the expected type.
    ValueConversionError,
    /// A mandatory input was unconnected.
    MandatoryInputUnconnected { input_name: Atom },
    /// A path to a model object was not found in the document being evaluated.
    PathNotFound,
    /// A node was expected to have an associated operator but none was found.
    NoOperator,
    /// An unknown operator was encountered during the evaluation process.
    UnknownOperator,
    /// Task-related error.
    TaskError(TaskError),
    /// Shader syntax error.
    SyntaxError(String),
    /// General error with a message.
    General(String),
    /// General I/O error.
    ///
    // io::Error is not clone
    Io(Arc<io::Error>),
    /// An error with additional context.
    Context { msg: String, source: Box<EvalError> },
}

impl From<io::Error> for EvalError {
    fn from(err: Error) -> Self {
        EvalError::Io(Arc::new(err))
    }
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EvalError::Unspecified => {
                write!(f, "unspecified evaluation error")
            }
            EvalError::ValueConversionError => {
                write!(f, "value conversion error")
            }
            EvalError::MandatoryInputUnconnected { input_name } => {
                write!(f, "mandatory input unconnected: `{input_name}`")
            }
            EvalError::PathNotFound => {
                write!(f, "path not found")
            }
            EvalError::NoOperator => {
                write!(f, "node has no operator")
            }
            EvalError::UnknownOperator => {
                write!(f, "unknown operator")
            }
            EvalError::TaskError(err) => {
                write!(f, "task error")
            }
            EvalError::SyntaxError(err) => {
                write!(f, "{}", err)
            }
            EvalError::General(err) => {
                write!(f, "{}", err)
            }
            EvalError::Io(err) => {
                write!(f, "I/O error: {}", err)
            }
            EvalError::Context { source, msg } => write!(f, "{source}\n  context: {msg}"),
        }
    }
}

impl EvalError {
    pub fn general(msg: impl Into<String>) -> EvalError {
        EvalError::General(msg.into())
    }

    pub fn context(self, info: impl Into<String>) -> EvalError {
        EvalError::Context {
            source: Box::new(self),
            msg: info.into(),
        }
    }
}

pub trait EvalErrorContextExt {
    fn context(self, info: impl Into<String>) -> Self;
}

impl<T> EvalErrorContextExt for Result<T, EvalError> {
    fn context(self, info: impl Into<String>) -> Result<T, EvalError> {
        match self {
            Ok(_) => self,
            Err(e) => Err(e.context(info)),
        }
    }
}

/*impl From<TaskError> for EvalError {
    fn from(err: TaskError) -> Self {
        EvalError::TaskError(err)
    }
}*/
