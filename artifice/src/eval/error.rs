use crate::model::ModelPath;
use kyute_common::Atom;
use std::io;
use thiserror::Error;

#[derive(Clone, Debug, thiserror::Error)]
pub enum EvalError {
    #[error("unspecified general evaluation error")]
    General,
    /// Value type mismatch.
    #[error("type mismatch")]
    TypeMismatch,
    /// Path not found.
    #[error("path not found: `{0:?}`")]
    PathNotFound(ModelPath),
    /// Unknown operator.
    #[error("unknown operator: `{0:?}`")]
    UnknownOperator(Atom),
    /// Task join error
    #[error("Task error")]
    Join(#[from] tokio::task::JoinError),
    /// I/O error.
    #[error("I/O error")]
    Io(#[from] io::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
