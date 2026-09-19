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

    /// Credential vault failure. Wraps the per-backend [`crate::server::vault::VaultError`]
    /// (which in turn wraps `S3Error` or `std::io::Error`) so callers do
    /// not have to thread a new error type through `Result<T>`.
    ///
    /// Added in V0 T6 (per [ADR-0021](../docs/adr/0021-v0-minio-credential-vault.md)).
    /// Backed by the `minIO` / `FileVault` impl in
    /// [`crate::server::vault`].
    #[error("vault error: {0}")]
    Vault(String),
}

/// Convenience alias used throughout the codebase.
pub type Result<T> = std::result::Result<T, GitGitError>;

/// Adapter for the `s3` crate's `S3Error`. The `?` operator inside
/// `MinioVault` method bodies needs `From<S3Error> for GitGitError`;
/// rather than scattering `.map_err(...)` calls we route through
/// `VaultError::Vault` so the message is consistent with the rest of
/// the vault layer.
///
/// Note: this lives in `error.rs` (not `vault.rs`) to keep `vault.rs`
/// decoupled from `crate::error` in the `From` direction. The inverse
/// conversion (`From<VaultError> for GitGitError`) is defined in
/// `vault.rs` because that side knows the `VaultError` shape.
impl From<::s3::error::S3Error> for GitGitError {
    fn from(value: ::s3::error::S3Error) -> Self {
        // The full error text (including status code and request id) is
        // preserved so log output stays debuggable.
        GitGitError::Vault(format!("minIO/S3: {value}"))
    }
}
