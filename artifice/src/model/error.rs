use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("no object at the given path")]
    NoObjectAtPath,
    #[error("object already exists")]
    AlreadyExists,
    #[error("mismatched attribute types")]
    MismatchedTypes,
    #[error("file error")]
    FileError(#[from] anyhow::Error),
    #[error("invalid path syntax")]
    PathSyntax,
}
