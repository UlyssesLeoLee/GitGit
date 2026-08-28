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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Build a fresh, unique temp directory under the OS temp dir. Each
    /// call returns a path that no other concurrently-running test should
    /// touch, because we mix in the process id and an atomic counter.
    ///
    /// We intentionally do not depend on the `tempfile` crate (per the
    /// 0-new-external-deps rule extended from 8/27 D.5+): the test cleanup
    /// contract is "the OS may recycle `temp_dir()` later, but the unique
    /// subdirectory is ours alone until the process exits".
    fn unique_temp(label: &str) -> std::path::PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!(
            "gitgit-test-{label}-{pid}-{n}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Lay down a minimal bare-repo layout under `dir`: just a `HEAD` file
    /// pointing at `refs/heads/main`. Refs are populated by the test.
    fn lay_bare_with_head(dir: &Path, head_target: &str) {
        std::fs::write(dir.join("HEAD"), format!("ref: {head_target}\n")).unwrap();
    }

    #[test]
    fn read_head_returns_symbolic_target() {
        let dir = unique_temp("refs-head");
        lay_bare_with_head(&dir, "refs/heads/main");
        let got = read_head(&dir).unwrap();
        assert_eq!(got, "refs/heads/main");
    }

    #[test]
    fn read_head_rejects_detached() {
        let dir = unique_temp("refs-detached");
        std::fs::write(dir.join("HEAD"), b"0123456789abcdef0123456789abcdef01234567\n").unwrap();
        let err = read_head(&dir).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("HEAD is detached"), "got: {msg}");
    }

    #[test]
    fn read_ref_resolves_short_name() {
        let dir = unique_temp("refs-short");
        lay_bare_with_head(&dir, "refs/heads/main");
        let sha = "1111111111111111111111111111111111111111";
        write_ref(&dir, "refs/heads/main", sha).unwrap();
        let got = read_ref(&dir, "main").unwrap();
        assert_eq!(got.as_deref(), Some(sha));
    }

    #[test]
    fn read_ref_passes_through_full_name() {
        let dir = unique_temp("refs-full");
        lay_bare_with_head(&dir, "refs/heads/main");
        let sha = "2222222222222222222222222222222222222222";
        write_ref(&dir, "refs/heads/feature/x", sha).unwrap();
        let got = read_ref(&dir, "refs/heads/feature/x").unwrap();
        assert_eq!(got.as_deref(), Some(sha));
    }

    #[test]
    fn read_ref_missing_returns_none() {
        let dir = unique_temp("refs-missing");
        lay_bare_with_head(&dir, "refs/heads/main");
        let got = read_ref(&dir, "nope").unwrap();
        assert!(got.is_none());
    }

    #[test]
    fn write_ref_rejects_non_refs_prefix() {
        let dir = unique_temp("refs-bad");
        lay_bare_with_head(&dir, "refs/heads/main");
        let err = write_ref(&dir, "notrefs/x", "abc").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("not a ref"), "got: {msg}");
    }

    #[test]
    fn write_ref_creates_nested_dirs() {
        let dir = unique_temp("refs-nested");
        lay_bare_with_head(&dir, "refs/heads/main");
        let sha = "3333333333333333333333333333333333333333";
        write_ref(&dir, "refs/heads/feature/deep/nested", sha).unwrap();
        // File must now be readable on disk via read_ref.
        let got = read_ref(&dir, "refs/heads/feature/deep/nested").unwrap();
        assert_eq!(got.as_deref(), Some(sha));
    }
}
