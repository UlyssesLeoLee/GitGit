//! Ref read/write helpers for a bare repo.
//!
//! Currently a stub. The smart-HTTP handlers do **not** read refs directly:
//! the `git upload-pack --advertise-refs` and `git receive-pack --advertise-refs`
//! subprocesses produce the canonical ref list. The helpers below are kept
//! for future debug / diagnostics endpoints and are intentionally a no-op
//! for the MVP to avoid surface area.

use std::path::Path;

use crate::error::{GitGitError, Result};

/// Read the symbolic target of `HEAD` in a bare repo (e.g. `refs/heads/main`).
#[allow(dead_code)]
pub fn read_head(repo: &Path) -> Result<String> {
    let head = repo.join("HEAD");
    let raw = std::fs::read_to_string(&head)?;
    let trimmed = raw.trim_end();
    let after = trimmed
        .strip_prefix("ref:")
        .ok_or_else(|| GitGitError::Http(format!("HEAD is detached: {trimmed}")))?;
    Ok(after.trim().to_string())
}

/// Resolve a ref like `main` or `refs/heads/main` to its full ref name and
/// return the resolved value if it exists.
#[allow(dead_code)]
pub fn read_ref(repo: &Path, name: &str) -> Result<Option<String>> {
    let full = if name.starts_with("refs/") {
        name.to_string()
    } else {
        format!("refs/heads/{name}")
    };
    let path = repo.join(&full);
    if !path.is_file() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path)?;
    Ok(Some(raw.trim().to_string()))
}

/// Write a ref to a bare repo. Used by tests / diagnostics; not on the
/// receive-pack hot path (that goes through `git receive-pack`).
#[allow(dead_code)]
pub fn write_ref(repo: &Path, full_ref: &str, sha: &str) -> Result<()> {
    if !full_ref.starts_with("refs/") {
        return Err(GitGitError::Http(format!("not a ref: {full_ref}")));
    }
    let path = repo.join(full_ref);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, format!("{sha}\n"))?;
    Ok(())
}
