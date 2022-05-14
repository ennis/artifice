use crate::{eval::TaskError, model::Path};
use kyute_common::Atom;
use std::io;
use thiserror::Error;

/// Error produced during evaluation of an attribute.
#[derive(Clone, Debug, Error)]
pub enum EvalError {
    /// Unspecified error.
    #[error("unspecified evaluation error")]
    Unspecified,
    /// The resulting value wasn't of the expected type.
    #[error("type mismatch")]
    TypeMismatch,
    /// A mandatory input was unconnected.
    #[error("mandatory input unconnected: `{input_name}`")]
    MandatoryInputUnconnected { input_name: Atom },
    /// A path to a model object was not found in the document being evaluated.
    #[error("path not found: `{0:?}`")]
    PathNotFound(Path),
    /// An unknown operator was encountered during the evaluation process.
    #[error("unknown operator: `{0:?}`")]
    UnknownOperator(Atom),
    /// Task-related error.
    #[error("task error")]
    TaskError(#[from] TaskError),
    /// I/O-related error.
    #[error("I/O error: {0}")]
    Io(String),
    /// General error with a message.
    #[error("{0}")]
    General(String),
}

impl EvalError {
    pub fn general(msg: impl Into<String>) -> EvalError {
        EvalError::General(msg.into())
    }
}

/*impl From<TaskError> for EvalError {
    fn from(err: TaskError) -> Self {
        EvalError::TaskError(err)
    }
}*/
