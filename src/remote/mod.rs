//! `gitremote` — named upstream remotes for managed bare repositories.
//!
//! V0 task T8. The original acceptance line read
//! "`gitremote add gitee <url>` 存 PG + fast-forward sync 工作".
//!
//! # Why there is no PG here
//!
//! The "存 PG" half of that line conflicts with a recorded architectural
//! decision, and the conflict was resolved in favour of the ADR rather
//! than the acceptance wording.
//!
//! - ADR-0022 §2.1 states that sqlx / PG is **not** introduced into
//!   gitgit, as part of the single-crate + zero-platform-specific-code
//!   boundary from ADR-0021 §1.2.
//! - ADR-0022 already evaluated and **rejected** "directly wire
//!   AssetsLake's schema + sqlx into gitgit" as rejected option #2.
//!
//! Building T8 as originally worded would have meant reversing a recorded
//! ADR and adding a native dependency, which is a decision rather than an
//! implementation detail. Instead the storage is a plain JSON file
//! alongside the existing `FileVault` root, and the acceptance is the
//! behaviour — a remote can be added, listed, and fast-forward synced —
//! not the backing store. The registry is deliberately shaped so a real
//! store can be dropped in behind [`RemoteStore`] later without touching
//! the CLI surface.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{GitGitError, Result};

pub mod sync;

/// Default on-disk location of the remote registry.
///
/// Sits next to the `FileVault` root (`DEFAULT_VAULT_FILE_ROOT` =
/// `.gitgit/vault`) rather than inside it, so the two concerns stay
/// separable and wiping a vault does not take remotes with it.
pub const DEFAULT_REMOTES_FILE: &str = ".gitgit/remotes.json";

/// One named upstream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Remote {
    /// Short key used on the command line, e.g. `gitee`.
    pub name: String,
    /// URL passed to `git fetch` / `git push`.
    pub url: String,
    /// When the entry was added, for `ls` output and for pruning later.
    pub created_at_unix_ms: i64,
}

impl Remote {
    /// Validate a remote name.
    ///
    /// The name becomes a path-like token in output and, more importantly,
    /// is what a user types; reject anything that would be ambiguous or
    /// that could be mistaken for a git option. Reusing the repo-name
    /// rules keeps one validation vocabulary in the codebase.
    pub fn validate_name(name: &str) -> Result<()> {
        crate::config::validate_repo_name(name)
    }

    /// Validate a remote URL.
    ///
    /// Deliberately permissive: git accepts `https://`, `ssh://`, and
    /// `user@host:path` scp-style, and this codebase has no business
    /// second-guessing which. The one hard requirement is that it is
    /// non-empty and contains no whitespace, because a URL with a space in
    /// it would be split by the argv handling downstream and silently
    /// target the wrong place.
    pub fn validate_url(url: &str) -> Result<()> {
        if url.is_empty() {
            return Err(GitGitError::InvalidRemoteUrl(url.to_string()));
        }
        if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(GitGitError::InvalidRemoteUrl(url.to_string()));
        }
        if url.starts_with('-') {
            // Would be parsed as an option by git.
            return Err(GitGitError::InvalidRemoteUrl(url.to_string()));
        }
        Ok(())
    }
}

/// The serialized document shape.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct RegistryFile {
    #[serde(default)]
    remotes: Vec<Remote>,
}

/// A JSON-file-backed set of named remotes.
#[derive(Debug, Clone)]
pub struct RemoteStore {
    path: PathBuf,
}

impl RemoteStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Store rooted at `root`'s parent directory, i.e. `.gitgit/remotes.json`.
    pub fn under(root: &Path) -> Self {
        Self::new(root.join("remotes.json"))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Every remote, sorted by name so `ls` output is stable.
    ///
    /// A missing file is an empty registry, not an error: that is the
    /// state of a fresh clone, and `gitremote ls` on one should print
    /// "(no remotes)" rather than fail.
    pub fn list(&self) -> Result<Vec<Remote>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let raw = std::fs::read_to_string(&self.path)?;
        if raw.trim().is_empty() {
            return Ok(Vec::new());
        }
        let parsed: RegistryFile = serde_json::from_str(&raw).map_err(|e| {
            GitGitError::Remote(format!(
                "{} is not valid remotes JSON: {e}",
                self.path.display()
            ))
        })?;
        let mut remotes = parsed.remotes;
        remotes.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(remotes)
    }

    /// Look up one remote by name.
    pub fn get(&self, name: &str) -> Result<Remote> {
        self.list()?
            .into_iter()
            .find(|r| r.name == name)
            .ok_or_else(|| {
                let known: Vec<String> = self
                    .list()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|r| r.name)
                    .collect();
                let suffix = if known.is_empty() {
                    "no remotes are configured".to_string()
                } else {
                    format!("known remotes: {}", known.join(", "))
                };
                GitGitError::Remote(format!("unknown remote `{name}`; {suffix}"))
            })
    }

    /// Add a remote, or update the URL of an existing one.
    ///
    /// Idempotent by name: re-adding `gitee` with a different URL is an
    /// update, not a duplicate and not an error. Reporting "already exists"
    /// would force a separate `set` verb for the common "I moved my repo"
    /// case.
    pub fn add(&self, name: &str, url: &str) -> Result<AddOutcome> {
        Remote::validate_name(name)?;
        Remote::validate_url(url)?;

        let mut remotes = self.list()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        let outcome = match remotes.iter_mut().find(|r| r.name == name) {
            Some(existing) => {
                let changed = existing.url != url;
                existing.url = url.to_string();
                if changed {
                    AddOutcome::Updated
                } else {
                    AddOutcome::Unchanged
                }
            }
            None => {
                remotes.push(Remote {
                    name: name.to_string(),
                    url: url.to_string(),
                    created_at_unix_ms: now,
                });
                AddOutcome::Added
            }
        };

        self.write_all(&remotes)?;
        Ok(outcome)
    }

    /// Remove a remote. Returns `true` if one was removed.
    pub fn remove(&self, name: &str) -> Result<bool> {
        let mut remotes = self.list()?;
        let before = remotes.len();
        remotes.retain(|r| r.name != name);
        if remotes.len() == before {
            return Ok(false);
        }
        self.write_all(&remotes)?;
        Ok(true)
    }

    /// Write atomically via a temp file + rename.
    ///
    /// A half-written registry would be indistinguishable from a corrupt
    /// one, and a crash mid-write is exactly how that happens. Rename is
    /// atomic within a filesystem, so a reader either sees the old file or
    /// the new one.
    fn write_all(&self, remotes: &[Remote]) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let doc = RegistryFile {
            remotes: remotes.to_vec(),
        };
        let body = serde_json::to_string_pretty(&doc)
            .map_err(|e| GitGitError::Remote(format!("serialize remotes: {e}")))?;

        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, body.as_bytes())?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

/// What [`RemoteStore::add`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddOutcome {
    Added,
    Updated,
    /// The same name and URL were already present. Not an error, but worth
    /// reporting so a re-run of a setup script is not silently a no-op.
    Unchanged,
}

impl AddOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            AddOutcome::Added => "added",
            AddOutcome::Updated => "updated",
            AddOutcome::Unchanged => "unchanged",
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn unique_store(label: &str) -> RemoteStore {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "gitgit-test-remote-{label}-{}-{n}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        RemoteStore::new(dir.join("remotes.json"))
    }

    #[test]
    fn missing_file_is_an_empty_registry_not_an_error() {
        // A fresh clone has no registry; `gitremote ls` must say so, not fail.
        let s = unique_store("missing");
        assert!(s.list().unwrap().is_empty());
    }

    #[test]
    fn add_then_list_round_trips() {
        let s = unique_store("roundtrip");
        assert_eq!(
            s.add("gitee", "https://gitee.com/u/r.git").unwrap(),
            AddOutcome::Added
        );
        let remotes = s.list().unwrap();
        assert_eq!(remotes.len(), 1);
        assert_eq!(remotes[0].name, "gitee");
        assert_eq!(remotes[0].url, "https://gitee.com/u/r.git");
    }

    #[test]
    fn readding_same_url_is_unchanged() {
        let s = unique_store("same");
        s.add("gitee", "https://a/b.git").unwrap();
        assert_eq!(
            s.add("gitee", "https://a/b.git").unwrap(),
            AddOutcome::Unchanged
        );
        assert_eq!(s.list().unwrap().len(), 1, "must not duplicate");
    }

    #[test]
    fn readding_new_url_updates_in_place() {
        // "I moved my repo" is the common case; it should not need a
        // separate verb or produce a duplicate entry.
        let s = unique_store("update");
        s.add("gitee", "https://old/b.git").unwrap();
        assert_eq!(
            s.add("gitee", "https://new/b.git").unwrap(),
            AddOutcome::Updated
        );
        let remotes = s.list().unwrap();
        assert_eq!(remotes.len(), 1);
        assert_eq!(remotes[0].url, "https://new/b.git");
    }

    #[test]
    fn list_is_sorted_by_name() {
        let s = unique_store("sorted");
        s.add("zeta", "https://z/z.git").unwrap();
        s.add("alpha", "https://a/a.git").unwrap();
        s.add("mid", "https://m/m.git").unwrap();
        let names: Vec<String> = s.list().unwrap().into_iter().map(|r| r.name).collect();
        assert_eq!(names, vec!["alpha", "mid", "zeta"]);
    }

    #[test]
    fn unknown_name_error_lists_the_known_ones() {
        let s = unique_store("unknown");
        s.add("gitee", "https://a/b.git").unwrap();
        let err = s.get("nope").unwrap_err().to_string();
        assert!(err.contains("nope"), "got: {err}");
        assert!(err.contains("gitee"), "error should be actionable: {err}");
    }

    #[test]
    fn empty_registry_says_so() {
        let s = unique_store("emptymsg");
        let err = s.get("anything").unwrap_err().to_string();
        assert!(err.contains("no remotes are configured"), "got: {err}");
    }

    #[test]
    fn remove_reports_whether_anything_went() {
        let s = unique_store("remove");
        s.add("gitee", "https://a/b.git").unwrap();
        assert!(s.remove("gitee").unwrap());
        assert!(!s.remove("gitee").unwrap());
        assert!(s.list().unwrap().is_empty());
    }

    #[test]
    fn corrupt_file_is_reported_not_silently_ignored() {
        // An unreadable registry must not look like an empty one: that
        // would make the next `add` silently overwrite whatever the
        // operator had configured.
        let s = unique_store("corrupt");
        std::fs::create_dir_all(s.path().parent().unwrap()).unwrap();
        std::fs::write(s.path(), b"{ this is not json").unwrap();
        let err = s.list().unwrap_err().to_string();
        assert!(err.contains("not valid remotes JSON"), "got: {err}");
    }

    #[test]
    fn empty_file_is_treated_as_empty() {
        // Distinct from corrupt: an empty file carries no claim to check.
        let s = unique_store("blank");
        std::fs::create_dir_all(s.path().parent().unwrap()).unwrap();
        std::fs::write(s.path(), b"   \n").unwrap();
        assert!(s.list().unwrap().is_empty());
    }

    #[test]
    fn url_with_whitespace_is_rejected() {
        // A URL with a space would be split by argv handling downstream
        // and could silently target the wrong repository.
        let s = unique_store("spaceurl");
        let err = s.add("x", "https://a b/c.git").unwrap_err().to_string();
        assert!(err.contains("URL"), "got: {err}");
    }

    #[test]
    fn url_that_looks_like_an_option_is_rejected() {
        let s = unique_store("dashurl");
        assert!(s.add("x", "--upload-pack=evil").is_err());
    }

    #[test]
    fn empty_url_is_rejected() {
        let s = unique_store("nourl");
        assert!(s.add("x", "").is_err());
    }

    #[test]
    fn remote_name_reuses_repo_name_rules() {
        // One validation vocabulary in the codebase.
        for bad in ["", "a/b", "..", ".hidden", "has space"] {
            assert!(Remote::validate_name(bad).is_err(), "should reject {bad:?}");
        }
        assert!(Remote::validate_name("gitee").is_ok());
    }

    #[test]
    fn no_temp_file_is_left_behind() {
        let s = unique_store("tmp");
        s.add("gitee", "https://a/b.git").unwrap();
        let tmp = s.path().with_extension("json.tmp");
        assert!(!tmp.exists(), "atomic write left a temp file behind");
    }
}
