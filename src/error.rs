//! Error types for the gitgit MVP.

use std::path::PathBuf;

use thiserror::Error;

/// Top-level error type for the `gitgit` binary.
#[derive(Debug, Error)]
pub enum GitGitError {
    /// A bare repository at the given path could not be created.
    #[error("failed to create bare repo at {path}: {source}")]
    InitRepo {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// The configured repos directory does not exist or is not a directory.
    #[error("repos directory {0} is missing or not a directory")]
    InvalidReposDir(PathBuf),

    /// A repository name was malformed (empty, contains path separators, etc.).
    #[error("invalid repo name: {0}")]
    InvalidRepoName(String),

    /// The `git` CLI subprocess failed.
    #[error("git subprocess `{cmd}` failed: {source}")]
    GitSubprocess {
        cmd: String,
        #[source]
        source: std::io::Error,
    },

    /// The `git` CLI subprocess returned a non-zero status.
    #[error("git `{cmd}` exited with {status}: {stderr}")]
    GitExit {
        cmd: String,
        status: i32,
        stderr: String,
    },

    /// HTTP-level failure (used by the axum layer).
    #[error("http error: {0}")]
    Http(String),

    /// Authentication failure (HTTP 401 path).
    #[error("authentication required")]
    Unauthenticated,

    /// I/O error not covered by a more specific variant.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Convenience alias used throughout the codebase.
pub type Result<T> = std::result::Result<T, GitGitError>;
