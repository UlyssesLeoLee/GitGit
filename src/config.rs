//! Minimal runtime configuration for the `gitgit` binary.

use std::path::PathBuf;

use crate::error::{GitGitError, Result};

/// Bind address the server listens on.
pub const DEFAULT_BIND: &str = "0.0.0.0:8080";

/// Subdirectory under the current working directory where bare repos live.
pub const DEFAULT_REPOS_DIR: &str = "repos";

/// Default on-disk root for the V0 Credential Vault (per ADR-0021).
///
/// Per ADR-0021 §1.2 the MVP is a single-crate binary with zero
/// platform-specific code; the credential backend is therefore the
/// `FileVault` rooted here. Operators can override via
/// `GITGIT_VAULT_FILE_ROOT` (or `--vault-file-root` on the CLI). The
/// commit that exposes a `MinioVault` runtime toggled by
/// `GITGIT_VAULT_BACKEND=minio` follows in V0 close-out.
pub const DEFAULT_VAULT_FILE_ROOT: &str = ".gitgit/vault";

/// Hard-coded credentials for the MVP.
///
/// The brief is explicit: single hard-coded user. We never read these from
/// disk or the environment because the MVP does not need it.
pub const ADMIN_USER: &str = "admin";
pub const ADMIN_PASS: &str = "admin";

/// Resolved, validated configuration for one server invocation.
#[derive(Debug, Clone)]
pub struct Config {
    /// Address to bind (e.g. `0.0.0.0:8080`).
    pub bind: String,
    /// Directory under which bare repos are stored (`<dir>/<name>.git/`).
    pub repos_dir: PathBuf,
    /// V0 Credential Vault root directory (per ADR-0021 §1.2 default).
    /// Currently only used by `FileVault`; reserved for a minIO backend
    /// in V0 close-out.
    pub vault_file_root: PathBuf,
}

impl Config {
    /// Build a config from explicit values, defaulting the bind address
    /// and the vault file root.
    pub fn new(
        bind: impl Into<String>,
        repos_dir: impl Into<PathBuf>,
        vault_file_root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            bind: bind.into(),
            repos_dir: repos_dir.into(),
            vault_file_root: vault_file_root.into(),
        }
    }

    /// Ensure that `repos_dir` exists and is a directory. Creates it if missing.
    pub fn ensure_repos_dir(&self) -> Result<()> {
        if !self.repos_dir.exists() {
            std::fs::create_dir_all(&self.repos_dir)?;
            return Ok(());
        }
        if !self.repos_dir.is_dir() {
            return Err(GitGitError::InvalidReposDir(self.repos_dir.clone()));
        }
        Ok(())
    }

    /// Ensure that `vault_file_root` exists and is a directory. Creates
    /// it if missing. Mirrors `ensure_repos_dir`'s semantics.
    pub fn ensure_vault_root(&self) -> Result<()> {
        if !self.vault_file_root.exists() {
            std::fs::create_dir_all(&self.vault_file_root)?;
            return Ok(());
        }
        if !self.vault_file_root.is_dir() {
            return Err(GitGitError::InvalidReposDir(
                self.vault_file_root.clone(),
            ));
        }
        Ok(())
    }

    /// Resolve the on-disk path for a repo name: `<repos_dir>/<name>.git`.
    ///
    /// Git clients address bare repos with the conventional `.git` suffix
    /// (e.g. `demo.git`), so we accept both `demo` and `demo.git` and
    /// normalize to a single canonical form (`demo.git`) on disk.
    pub fn repo_path(&self, name: &str) -> Result<PathBuf> {
        validate_repo_name(name)?;
        let stem = name.strip_suffix(".git").unwrap_or(name);
        // Re-validate after stripping the suffix to make sure a name
        // like `..git` doesn't smuggle a path traversal.
        validate_repo_name(stem)?;
        Ok(self.repos_dir.join(format!("{stem}.git")))
    }
}

/// Validate that a repository name is safe to embed in a path.
///
/// Rejects empty names, names with `..`, names with path separators, and
/// control characters. The name may contain `.` (e.g. `demo.git`) since
/// that is the conventional bare-repo name suffix.
pub fn validate_repo_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(GitGitError::InvalidRepoName(name.to_string()));
    }
    if name.contains('/') || name.contains('\\') {
        return Err(GitGitError::InvalidRepoName(name.to_string()));
    }
    // Reject `..` as a whole segment (path traversal). We disallow any
    // occurrence of `..` to be safe even within a name like `foo..bar`,
    // since `foo..bar` could be ambiguous.
    if name == ".." || name.contains("..") {
        return Err(GitGitError::InvalidRepoName(name.to_string()));
    }
    // Reject names that start with a dot (hidden / traversal-like).
    if name.starts_with('.') {
        return Err(GitGitError::InvalidRepoName(name.to_string()));
    }
    // Reject whitespace and control characters.
    if name.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(GitGitError::InvalidRepoName(name.to_string()));
    }
    Ok(())
}
