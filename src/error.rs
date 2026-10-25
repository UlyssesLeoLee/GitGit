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

    /// AI provider or `gitai` pipeline failure (V0 T7).
    ///
    /// A string payload rather than a structured variant: the failures
    /// that reach here are provider-specific (HTTP status plus the
    /// provider's own error text), and a fixed set of variants would
    /// either lose that detail or need a catch-all anyway.
    ///
    /// Callers must never place an API key into this message. It is
    /// printed to the terminal and captured by CI logs.
    #[error("ai error: {0}")]
    Ai(String),

    /// The requested credential-vault backend is not one of the supported
    /// backends.
    ///
    /// `[FACT]` This is an error rather than a silent fallback to the
    /// local file vault: an operator who asked for object storage and
    /// got the file vault would keep writing credentials to disk while
    /// believing they were in S3. The message names the accepted values
    /// so the mistake is fixable without reading the source.
    #[error("unknown vault backend `{0}`: expected one of `file`, `s3`")]
    InvalidVaultBackend(String),

    /// The `s3` credential-vault backend was selected but one of its
    /// credentials was not supplied.
    ///
    /// `[FACT]` Credentials have no default on purpose. A made-up key
    /// would fail later with a 403 that reads like a permissions
    /// problem, and the operator would have no way to tell that the key
    /// they configured was never used.
    ///
    /// `[FACT]` The message names the environment variable only. There
    /// is deliberately no command-line flag for a credential: a flag
    /// would put the secret in the process arguments and in the shell
    /// history.
    #[error("the `s3` vault backend requires {0}: set {1}")]
    MissingVaultCredential(&'static str, &'static str),

    /// The `s3` credential-vault backend was selected without a key
    /// prefix.
    ///
    /// `[FACT]` The prefix is the isolation: `src/server/vault.rs`
    /// documents it as the way to keep several gitgit instances from
    /// sharing a bucket without collision. A shared default prefix
    /// would put two instances on the same object keys and the same
    /// version sidecars, where either could read or overwrite the
    /// other's credentials.
    #[error("the `s3` vault backend requires a key prefix: set {0} or pass `--{1}` (the prefix isolates two instances that share a bucket, so a shared default would let them overwrite each other's credentials)")]
    MissingVaultKeyPrefix(&'static str, &'static str),

    /// The `s3` credential-vault backend was given a key prefix that is
    /// not a path prefix.
    ///
    /// `[FACT]` A prefix without a trailing separator is a *string*
    /// prefix, and string prefixes overlap: prefix `a` with key `bc`
    /// and prefix `ab` with key `c` are the same object key, so two
    /// instances could land on each other's credentials without either
    /// of them noticing. A prefix that ends with a separator is a path
    /// prefix, which is what the vault documents the prefix as -- and
    /// what every prefix in this repository already is.
    #[error("the `s3` vault backend requires a key prefix that ends with `/`: set {0} or pass `--{1}` with a trailing separator (a prefix without the separator is a string prefix, and two instances can overlap on the same object key); got `{2}`")]
    InvalidVaultKeyPrefix(&'static str, &'static str, String),

    /// The `s3` credential-vault backend was given an empty bucket
    /// name.
    ///
    /// `[FACT]` The bucket is the other half of where a credential
    /// lives. An empty bucket puts the objects at the endpoint root --
    /// a shared, unnamed namespace, and the same hazard the empty key
    /// prefix was.
    #[error("the `s3` vault backend requires a bucket name: set {0} or pass `--{1}` (an empty bucket puts the objects at the endpoint root, which is a shared namespace)")]
    InvalidVaultBucket(&'static str, &'static str),

    /// A remote name or URL failed validation (V0 T8 `gitremote`).
    #[error("invalid remote URL: {0:?}")]
    InvalidRemoteUrl(String),

    /// Remote registry or `gitremote` operation failure (V0 T8).
    #[error("remote error: {0}")]
    Remote(String),
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
