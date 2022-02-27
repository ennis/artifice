use crate::backend;
use std::{error, fmt};

/// Errors emitted.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Error originating from the application backend.
    #[error("backend error")]
    Backend(#[from] backend::Error),
}
