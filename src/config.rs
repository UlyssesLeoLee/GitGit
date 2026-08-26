//! Minimal runtime configuration for the `gitgit` binary.

use std::path::PathBuf;

use crate::error::{GitGitError, Result};

/// Bind address the server listens on.
pub const DEFAULT_BIND: &str = "0.0.0.0:8080";

/// Subdirectory under the current working directory where bare repos live.
pub const DEFAULT_REPOS_DIR: &str = "repos";

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
}

impl Config {
    /// Build a config from explicit values, defaulting the bind address.
    pub fn new(bind: impl Into<String>, repos_dir: impl Into<PathBuf>) -> Self {
        Self {
            bind: bind.into(),
            repos_dir: repos_dir.into(),
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

    /// Resolve the on-disk path for a repo name: `<repos_dir>/<name>.git`.
    pub fn repo_path(&self, name: &str) -> Result<PathBuf> {
        validate_repo_name(name)?;
        Ok(self.repos_dir.join(format!("{name}.git")))
    }
}

/// Validate that a repository name is safe to embed in a path.
///
/// Rejects empty names, names with `..`, names with path separators, and names
/// containing characters that would be rejected by `git init` itself on most
/// platforms.
pub fn validate_repo_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(GitGitError::InvalidRepoName(name.to_string()));
    }
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err(GitGitError::InvalidRepoName(name.to_string()));
    }
    if name.starts_with('.') {
        return Err(GitGitError::InvalidRepoName(name.to_string()));
    }
    // Reject whitespace and control characters.
    if name.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(GitGitError::InvalidRepoName(name.to_string()));
    }
    Ok(())
}
