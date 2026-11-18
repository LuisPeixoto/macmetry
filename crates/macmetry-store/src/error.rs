//! Errors of the history database, written as sentences a person can act on.

use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("There is no history yet at {0}. Open macmetry for a minute and try again.")]
    NoHistory(PathBuf),
    #[error("The history database could not be used: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("The history folder could not be prepared: {0}")]
    Folder(#[from] std::io::Error),
    #[error("{0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, StoreError>;
