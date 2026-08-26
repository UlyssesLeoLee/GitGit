//! Filesystem layout for bare repos.
//!
//! The MVP stores one bare repo per directory at `<repos_dir>/<name>.git/`.
//! All operations on the repo go through `git` via
//! [`crate::server::subprocess`]; this module just owns the directory and
//! the discovery / list logic.

use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::error::{GitGitError, Result};

/// Result of scanning the repos directory: a sorted list of names.
pub fn list_repos(repos_dir: &Path) -> Result<Vec<String>> {
    if !repos_dir.exists() {
        return Ok(Vec::new());
    }
    if !repos_dir.is_dir() {
        return Err(GitGitError::InvalidReposDir(repos_dir.to_path_buf()));
    }

    let mut out: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(repos_dir)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        if let Some(stripped) = name.strip_suffix(".git") {
            // A bare repo has a HEAD inside.
            if entry.path().join("HEAD").is_file() {
                out.push(stripped.to_string());
            }
        }
    }
    out.sort();
    Ok(out)
}

/// Returns true if a bare repo with the given name exists under `repos_dir`.
pub fn repo_exists(repos_dir: &Path, name: &str) -> bool {
    let path = repos_dir.join(format!("{name}.git"));
    path.is_dir() && path.join("HEAD").is_file()
}

/// Create a new bare repo at `<repos_dir>/<name>.git/`.
pub async fn create_bare_repo(config: &Config, name: &str) -> Result<PathBuf> {
    config.ensure_repos_dir()?;
    let path = config.repo_path(name)?;
    if repo_exists(&config.repos_dir, name) {
        return Ok(path);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| GitGitError::InitRepo {
            path: path.clone(),
            source,
        })?;
    }
    crate::server::subprocess::git_init_bare(&path).await?;
    Ok(path)
}
