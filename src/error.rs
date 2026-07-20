//! Error types for APO.

use std::path::PathBuf;

use thiserror::Error;

/// Top-level error for APO operations.
#[derive(Debug, Error)]
pub enum Error {
    /// The given path is not a directory.
    #[error("path is not a directory: {0}")]
    NotADirectory(PathBuf),

    /// The given path is not a Git repository.
    #[error("not a git repository: {0}")]
    NotAGitRepository(PathBuf),

    /// I/O failure.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization failure.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// Git failure (library or `git` CLI).
    #[error("git error: {0}")]
    Git(String),

    /// Configuration failure.
    #[error("config error: {0}")]
    Config(String),
}

/// Convenient result alias.
pub type Result<T> = std::result::Result<T, Error>;
