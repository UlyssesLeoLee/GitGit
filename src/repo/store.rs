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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Build a unique temp directory. See `repo::refs::tests::unique_temp`
    /// for the rationale: we cannot pull in the `tempfile` crate per the
    /// 0-new-external-deps rule (8/27 D.5+), so we synthesize uniqueness
    /// from the process id + an atomic counter + a nanosecond timestamp.
    fn unique_temp(label: &str) -> std::path::PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!(
            "gitgit-test-store-{label}-{pid}-{n}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Lay down a fake bare-repo directory under `parent`: a directory
    /// named `<name>.git/` with a `HEAD` file inside. The content of HEAD
    /// does not need to be a valid symbolic ref for the listing/exists
    /// helpers — they only check that HEAD is a file.
    fn lay_fake_bare(parent: &Path, name: &str) -> PathBuf {
        let dir = parent.join(format!("{name}.git"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("HEAD"), b"ref: refs/heads/main\n").unwrap();
        dir
    }

    #[test]
    fn list_repos_on_missing_dir_is_empty() {
        let dir = unique_temp("missing");
        // Do NOT create `dir`; list_repos must return an empty Vec, not
        // an error. This matches the documented "auto-discover" semantics.
        let listed = list_repos(&dir).unwrap();
        assert!(listed.is_empty(), "got: {listed:?}");
    }

    #[test]
    fn list_repos_skips_dirs_without_head_and_returns_sorted() {
        let root = unique_temp("sorted");
        // Two real bare repos...
        lay_fake_bare(&root, "alpha");
        lay_fake_bare(&root, "beta");
        // ...and a stray directory that does NOT have a HEAD file. It
        // must be filtered out (we use HEAD-presence as the "is a bare
        // repo" signal in `list_repos`).
        std::fs::create_dir_all(root.join("not-a-repo.git")).unwrap();

        let listed = list_repos(&root).unwrap();
        assert_eq!(listed, vec!["alpha".to_string(), "beta".to_string()]);
    }

    #[test]
    fn repo_exists_detects_real_and_rejects_missing() {
        let root = unique_temp("exists");
        lay_fake_bare(&root, "present");
        assert!(repo_exists(&root, "present"));
        assert!(!repo_exists(&root, "absent"));
    }

    #[test]
    fn repo_exists_rejects_dir_without_head() {
        // Stray directory without HEAD must be treated as "not a repo".
        let root = unique_temp("nohead");
        std::fs::create_dir_all(root.join("naked.git")).unwrap();
        assert!(!repo_exists(&root, "naked"));
    }

    #[tokio::test]
    async fn create_bare_repo_actually_invokes_git_init() {
        // This test requires the real `git` binary on PATH. On the
        // developer's machine and CI it is always present; on a minimal
        // container without git the test would fail with a subprocess
        // error which is also acceptable evidence.
        let repos = unique_temp("create");
        let vault_root = unique_temp("create-vault");
        let cfg = Config::new("127.0.0.1:0", repos.clone(), vault_root);
        let path = create_bare_repo(&cfg, "demo").await.unwrap();

        // Path must be `<repos>/demo.git`
        assert_eq!(path, repos.join("demo.git"));
        // HEAD must now exist (real `git init --bare` writes it).
        assert!(path.join("HEAD").is_file(), "HEAD missing in {path:?}");
        // Object/info and refs/heads subdirs are part of a real bare repo.
        assert!(path.join("objects").is_dir());
        assert!(path.join("refs").is_dir());

        // Idempotent: calling again must succeed and not error.
        let again = create_bare_repo(&cfg, "demo").await.unwrap();
        assert_eq!(again, path);
    }

    #[tokio::test]
    async fn create_bare_repo_rejects_invalid_name() {
        // Path traversal / empty name must be rejected before we touch
        // the filesystem. `Config::repo_path` is the gatekeeper.
        let repos = unique_temp("reject");
        let vault_root = unique_temp("reject-vault");
        let cfg = Config::new("127.0.0.1:0", repos, vault_root);
        let err = create_bare_repo(&cfg, "../escape").await.unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("invalid repo name"),
            "expected invalid repo name, got: {msg}"
        );
    }
}
