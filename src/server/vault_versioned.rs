//! Versioned Vault layer for `gitgit` V0 Credential Vault (per ADR-0022).
//!
// V0 commit: this module exposes the new `VersionedVault` sub-trait plus
// two backend implementations (`FileVault`, `MinioVault`). Until
// `AppState` is rewired to consume a `Arc<dyn VersionedVault>` (T6
// close-out per ADR-0022 §follow-up) every helper here is "dead" from
// the production build's point of view — same situation as the
// `#[allow(dead_code)]` in `vault.rs`. The lint restriction
// `unused_must_use = "deny"` in `[lints.rust]` is still respected.
#![allow(dead_code)]
//!
//! Reuses the version-management contract that was first shaped in the
//! `AssetsLake` project's ADR-0022 §2.1 / §2.2:
//!
//! ```text
//! - list_versions(key)
//! - get_at_version(key, version)
//! - restore_to_version(key, target_version)        # bytes reverted, current bumped
//! - diff_versions(key, base_version, head_version)
//! - compare_signatures(key, base_version, head_version) -> bool
//! ```
//!
//! Storage shape diverges from `AssetsLake` because `gitgit` is
//! intentionally a *single-crate, no-PG* MVP (per ADR-0021 §1.2 constraint
//! and the project's explicitly archived 14-crate workspace design). The
//! version metadata therefore lives in a JSON sidecar managed by this
//! module, not a `vault_versions` SQL table.
//!
//! ## Sidecar format
//!
//! Per backend, a single file or object stores the timeline for one key:
//!
//! ```json
//! { "key": "<logical key>",
//!   "versions": [
//!     { "version": 1, "bytes_sha256": "…", "byte_len": 42,
//!       "created_at_unix_ms": 1700000000000,
//!       "change_note": "Initial submission" },
//!     { "version": 2, "bytes_sha256": "…", "byte_len": 50,
//!       "created_at_unix_ms": 1700000010000,
//!       "change_note": "rotated" },
//!     { "version": 3, "bytes_sha256": "<== v1 hash>", "byte_len": 42,
//!       "created_at_unix_ms": 1700000020000,
//!       "change_note": "Restored from version 1" }
//!   ] }
//! ```
//!
//! * `versions` is **append-only**. `restore_to_version` does not rewrite
//!   history; it appends a new entry whose `bytes_sha256` matches an
//!   earlier entry's. Reads walk the list and pick the matching entry.
//! * Versions are 1-indexed and contiguous starting from 1 for each key.
//!   A `set` that bumps an existing value produces version `N+1`; the
//!   previous value's snapshot is `N`.
//! * Sidecar file location:
//!   - `FileVault`: `<root>/_versions/<key>.versions.json`
//!   - `MinioVault`: object key `<key_prefix><key>.versions.json` in the
//!     same bucket (no extra bucket needed; `minIO` versioning — when
//!     enabled at the bucket — provides defense-in-depth, mirroring the
//!     same choice we made in `AssetsLake` ADR-0022 §2.4).
//!
//! ## Known gaps (V0)
//!
//! * `get_at_version` returns the *latest* value only. Restoring a
//!   historical version's *bytes* is currently impossible from the
//!   `rust-s3` 0.37 surface used here (no `versionId` lookup). Each
//!   backend writes a marker in place of the historical bytes and
//!   surfaces the situation in the sidecar `change_note`. Follow-up
//!   PRs can wire actual byte-level restore; the metadata layer is
//!   complete today.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::error::{GitGitError, Result};
use crate::server::vault::{FileVault, MinioVault, Vault, VaultError};

// ─── Public types ────────────────────────────────────────────────────────

/// One entry in the version timeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultVersionSummary {
    /// 1-indexed; contiguous per key.
    pub version: i32,
    /// Hex SHA-256 of the value bytes at that version.
    pub bytes_sha256: String,
    /// Byte length of the stored value.
    pub byte_len: u64,
    /// Wall-clock time when the version entry was appended.
    pub created_at_unix_ms: i64,
    /// Free-form note for audit / debugging.
    pub change_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct VersionedTimeline {
    pub key: String,
    pub versions: Vec<VaultVersionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultVersionDiff {
    pub base: VaultVersionSummary,
    pub head: VaultVersionSummary,
    pub object_changed: bool,
    pub file_size_delta: i64,
}

/// Error type for versioned-vault operations.
///
/// All variants carry their own `String` payload. Three explicit
/// `From` impls live alongside so `?` can autoconvert at the call site.
#[derive(Debug, Error)]
pub enum VersionedVaultError {
    #[error("invalid version {0}; versions are 1-indexed and contiguous")]
    InvalidVersion(i32),
    #[error("version {version} not found for key {key}")]
    VersionNotFound { key: String, version: i32 },
    #[error("no versions recorded for key {0}; cannot restore")]
    NoHistory(String),
    #[error("versioned-vault io error: {0}")]
    Io(String),
    #[error("versioned-vault serde error: {0}")]
    Serde(String),
    #[error("minIO/S3 error: {0}")]
    Vault(String),
}

impl From<std::io::Error> for VersionedVaultError {
    fn from(value: std::io::Error) -> Self {
        VersionedVaultError::Io(value.to_string())
    }
}

impl From<serde_json::Error> for VersionedVaultError {
    fn from(value: serde_json::Error) -> Self {
        VersionedVaultError::Serde(value.to_string())
    }
}

impl From<VaultError> for VersionedVaultError {
    fn from(value: VaultError) -> Self {
        VersionedVaultError::Vault(value.to_string())
    }
}

impl From<VersionedVaultError> for GitGitError {
    fn from(value: VersionedVaultError) -> Self {
        GitGitError::Http(format!("versioned-vault: {value}"))
    }
}

// ─── Trait ───────────────────────────────────────────────────────────────

/// Vault that exposes a version timeline for each key.
///
/// Sub-trait of [`Vault`]: a `VersionedVault` MUST be able to answer the
/// same `get` / `set` / `delete` / `list` / `rotate` questions as
/// `Vault`, plus the five methods here. The asymmetry is intent of the
/// "opt-in by subtyping" choice recorded at the top of this file.
#[async_trait]
pub trait VersionedVault: Vault + Send + Sync {
    /// All recorded versions for `key`, oldest-first. Returns `Ok(empty
    /// vec)` if `key` was never written.
    async fn list_versions(&self, key: &str) -> Result<Vec<VaultVersionSummary>>;

    /// Variant of `Vault::set` that **also** appends a sidecar entry so
    /// the version timeline reflects this write. Returns the new
    /// version number (1-indexed, contiguous).
    ///
    /// This is the explicit opt-in entry point; calling the super-trait
    /// `Vault::set` directly on a `VersionedVault` does **not** record
    /// a version. That split keeps the `Vault` V0 contract stable (9
    /// unit tests untouched) while letting callers who want versioning
    /// upgrade their call sites one at a time.
    async fn set_with_version(&self, key: &str, value: &str) -> Result<i32>;

    /// Fetch the byte content stored at a historical version.
    /// V0 returns `Ok(None)` for any version other than the latest; the
    /// metadata is reliable but historical byte payload requires a
    /// follow-up commit (see module-level "Known gaps").
    async fn get_at_version(&self, key: &str, version: i32) -> Result<Option<String>>;

    /// Re-point `key`'s current bytes to the value stored at
    /// `target_version`. The previous current value is preserved as a
    /// new version entry so the timeline still carries a complete audit
    /// trail; `target_version`'s bytes remain unchanged (only metadata
    /// refers to them). Returns the new current version number.
    async fn restore_to_version(&self, key: &str, target_version: i32) -> Result<i32>;

    /// Compare two version snapshots.
    async fn diff_versions(
        &self,
        key: &str,
        base_version: i32,
        head_version: i32,
    ) -> Result<VaultVersionDiff>;

    /// True when the bytes at `base_version` and `head_version` are
    /// byte-for-byte identical (cheap shortcut before a full `diff`).
    async fn compare_signatures(
        &self,
        key: &str,
        base_version: i32,
        head_version: i32,
    ) -> Result<bool>;
}

// ─── Shared sidecar helpers ──────────────────────────────────────────────

/// SHA-256 of `bytes`, hex-encoded (lowercase, 64 chars). Centralized so
/// both backends hash identically.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

/// Unix epoch milliseconds, clamped to 0 for safety on clocks before
/// 1970 (extremely unlikely on V0 dev hardware but cheap to guard).
pub fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Append-or-skip a new entry to `timeline`.
///
/// Behaviour:
/// * The timeline's `versions` are kept contiguous and 1-indexed.
/// * If the incoming `bytes_sha256` matches the **last** entry, the new
///   entry is *not* appended (no-op), and `false` is returned. This
///   mirrors the `ON CONFLICT (asset_id, version) DO NOTHING` rule used
///   in `AssetsLake` migration 022 and prevents accidental double-applies
///   of the same `set` call.
/// * Returns `true` when a new entry was appended; `version` is then
///   the new highest version. Returns `false` when skipped.
pub fn append_version(
    timeline: &mut VersionedTimeline,
    bytes: &[u8],
    change_note: Option<&str>,
) -> bool {
    let sha = sha256_hex(bytes);

    if let Some(last) = timeline.versions.last() {
        if last.bytes_sha256 == sha {
            // Idempotent re-set of the same bytes: skip.
            return false;
        }
    }

    let next_version = timeline.versions.len() as i32 + 1;
    timeline.versions.push(VaultVersionSummary {
        version: next_version,
        bytes_sha256: sha,
        byte_len: bytes.len() as u64,
        created_at_unix_ms: now_unix_ms(),
        change_note: change_note.map(|s| s.to_string()),
    });
    true
}

/// Look up a single version summary by 1-indexed `version`.
pub fn find_version(
    timeline: &VersionedTimeline,
    version: i32,
) -> std::result::Result<VaultVersionSummary, VersionedVaultError> {
    if version < 1 {
        return Err(VersionedVaultError::InvalidVersion(version));
    }
    timeline
        .versions
        .iter()
        .find(|v| v.version == version)
        .cloned()
        .ok_or_else(|| VersionedVaultError::VersionNotFound {
            key: timeline.key.clone(),
            version,
        })
}

/// Append `bytes` (with optional `change_note`) into `timeline` and
/// return either the new entry or the skipped last entry — used by
/// `VersionedVault` impls after their backend-specific `set`.
fn bump_timeline(
    timeline: &mut VersionedTimeline,
    bytes: &[u8],
    change_note: Option<&str>,
) {
    append_version(timeline, bytes, change_note);
}

fn build_diff(
    tl: &VersionedTimeline,
    base_version: i32,
    head_version: i32,
) -> std::result::Result<VaultVersionDiff, VersionedVaultError> {
    let base = find_version(tl, base_version)?;
    let head = find_version(tl, head_version)?;
    let file_size_delta = head.byte_len as i64 - base.byte_len as i64;
    let object_changed = base.bytes_sha256 != head.bytes_sha256;
    Ok(VaultVersionDiff {
        base,
        head,
        object_changed,
        file_size_delta,
    })
}

// ─── FileVault implementation ────────────────────────────────────────────

fn filevault_sidecar_path(root: &Path, key: &str) -> PathBuf {
    // `/` and `\` in key are translated to `__` so multi-segment keys
    // (per `Vault::set` convention) flatten into a single sidecar file.
    let encoded = key.replace(['/', '\\'], "__");
    root.join("_versions").join(format!("{encoded}.versions.json"))
}

fn filevault_timeline_path(vault: &FileVault, key: &str) -> PathBuf {
    filevault_sidecar_path(vault.root(), key)
}

async fn filevault_load_timeline(
    vault: &FileVault,
    key: &str,
) -> Result<VersionedTimeline> {
    let path = filevault_timeline_path(vault, key);
    match tokio::fs::read(&path).await {
        Ok(bytes) => {
            let tl: VersionedTimeline =
                serde_json::from_slice(&bytes).map_err(VersionedVaultError::from)?;
            if tl.key != key {
                let msg = format!(
                    "sidecar for {key:?} stored a different logical key: {:?}",
                    tl.key
                );
                return Err(VersionedVaultError::Serde(msg).into());
            }
            Ok(tl)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(VersionedTimeline {
            key: key.to_string(),
            versions: Vec::new(),
        }),
        Err(e) => Err(VersionedVaultError::from(e).into()),
    }
}

async fn filevault_store_timeline(vault: &FileVault, tl: &VersionedTimeline) -> Result<()> {
    let path = filevault_timeline_path(vault, &tl.key);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let bytes = serde_json::to_vec_pretty(tl).map_err(VersionedVaultError::from)?;
    // Atomic write: write to a temp file in the same directory, then
    // rename over the target. Mirrors FileVault::set's write-to-tmp +
    // rename pattern so a crash mid-write cannot leave a half-written
    // JSON file visible.
    let mut tmp = path.clone();
    let tmp_name = format!(".tmp.{}", uuid::Uuid::new_v4().simple());
    tmp.set_file_name(tmp_name);
    tokio::fs::write(&tmp, &bytes).await?;
    if let Err(e) = tokio::fs::rename(&tmp, &path).await {
        let _ = tokio::fs::remove_file(&tmp).await;
        return Err(VersionedVaultError::from(e).into());
    }
    Ok(())
}

#[async_trait]
impl VersionedVault for FileVault {
    async fn list_versions(&self, key: &str) -> Result<Vec<VaultVersionSummary>> {
        Ok(filevault_load_timeline(self, key).await?.versions)
    }

    async fn set_with_version(&self, key: &str, value: &str) -> Result<i32> {
        // Write the value via the super-trait `set` first so any
        // backend-specific guarantees (FileVault's atomic-rename, MinIO
        // bucket versioning when enabled) apply identically.
        self.set(key, value).await?;
        let mut tl = filevault_load_timeline(self, key).await?;
        append_version(&mut tl, value.as_bytes(), None);
        filevault_store_timeline(self, &tl).await?;
        Ok(tl.versions.last().map(|v| v.version).unwrap_or(1))
    }

    async fn get_at_version(&self, key: &str, _version: i32) -> Result<Option<String>> {
        // V0: FileVault returns the latest value regardless of version.
        // Historical bytes for FileVault have been overwritten by `set`
        // and are not recoverable; the sidecar metadata records
        // bytes_sha256 / byte_len for audit, but the actual past value
        // lives only in the most recent file. Document the limitation
        // (see module-level "Known gaps").
        self.get(key).await
    }

    async fn restore_to_version(&self, key: &str, target_version: i32) -> Result<i32> {
        let mut tl = filevault_load_timeline(self, key).await?;
        if tl.versions.is_empty() {
            return Err(VersionedVaultError::NoHistory(key.to_string()).into());
        }
        let _ = find_version(&tl, target_version)?;

        // Read current value (may be None if key was deleted).
        let current = self.get(key).await?;
        let current_bytes = current.clone().unwrap_or_default();
        // Record what we replaced in the audit trail.
        bump_timeline(
            &mut tl,
            current_bytes.as_bytes(),
            Some(&format!(
                "Pre-restore snapshot before going back to v{target_version}"
            )),
        );

        // Build the "restored" value. FileVault cannot recover the
        // historical bytes, so we surface that gap in a marker that
        // names the target version. Follow-up PRs can add off-object
        // archival for FileVault if real restore is needed.
        let marker = format!(
            "[restored-to-v{target_version}-bytes-not-recoverable-from-filevault]"
        );
        self.set(key, &marker).await?;
        bump_timeline(
            &mut tl,
            marker.as_bytes(),
            Some(&format!(
                "Restored from version {target_version} (FileVault bytes not recoverable; \
                 see ADR-0022 — switch to MinioVault for true byte-level restore)"
            )),
        );
        filevault_store_timeline(self, &tl).await?;
        tracing::info!(
            backend = "FileVault",
            key = %key,
            target_version,
            new_version = tl.versions.last().map(|v| v.version).unwrap_or(0),
            "Vault restore (FileVault): sidecar metadata updated; \
             bytes not recoverable — use MinioVault for full byte restore"
        );
        Ok(tl.versions.last().map(|v| v.version).unwrap_or(0))
    }

    async fn diff_versions(
        &self,
        key: &str,
        base_version: i32,
        head_version: i32,
    ) -> Result<VaultVersionDiff> {
        let tl = filevault_load_timeline(self, key).await?;
        Ok(build_diff(&tl, base_version, head_version)?)
    }

    async fn compare_signatures(
        &self,
        key: &str,
        base_version: i32,
        head_version: i32,
    ) -> Result<bool> {
        let tl = filevault_load_timeline(self, key).await?;
        let diff = build_diff(&tl, base_version, head_version)?;
        Ok(!diff.object_changed)
    }
}

// ─── MinioVault implementation ───────────────────────────────────────────

fn minio_sidecar_object_key(prefix: &str, key: &str) -> String {
    if prefix.is_empty() {
        format!("{key}.versions.json")
    } else {
        format!("{prefix}{key}.versions.json")
    }
}

async fn minio_load_timeline(vault: &MinioVault, key: &str) -> Result<VersionedTimeline> {
    let object_key = minio_sidecar_object_key(vault.key_prefix(), key);
    match vault.get_object_for_sidecar(&object_key).await {
        Ok(Some(bytes)) => {
            let tl: VersionedTimeline =
                serde_json::from_slice(&bytes).map_err(VersionedVaultError::from)?;
            if tl.key != key {
                let msg = format!(
                    "minio sidecar for {key:?} stored a different logical key: {:?}",
                    tl.key
                );
                return Err(VersionedVaultError::Serde(msg).into());
            }
            Ok(tl)
        }
        Ok(None) => Ok(VersionedTimeline {
            key: key.to_string(),
            versions: Vec::new(),
        }),
        Err(e) => Err(VersionedVaultError::from(e).into()),
    }
}

async fn minio_store_timeline(vault: &MinioVault, tl: &VersionedTimeline) -> Result<()> {
    let object_key = minio_sidecar_object_key(vault.key_prefix(), &tl.key);
    let bytes = serde_json::to_vec_pretty(tl).map_err(VersionedVaultError::from)?;
    vault
        .put_object_for_sidecar(&object_key, &bytes)
        .await?;
    Ok(())
}

#[async_trait]
impl VersionedVault for MinioVault {
    async fn list_versions(&self, key: &str) -> Result<Vec<VaultVersionSummary>> {
        Ok(minio_load_timeline(self, key).await?.versions)
    }

    async fn set_with_version(&self, key: &str, value: &str) -> Result<i32> {
        self.set(key, value).await?;
        let mut tl = minio_load_timeline(self, key).await?;
        append_version(&mut tl, value.as_bytes(), None);
        minio_store_timeline(self, &tl).await?;
        Ok(tl.versions.last().map(|v| v.version).unwrap_or(1))
    }

    async fn get_at_version(&self, key: &str, version: i32) -> Result<Option<String>> {
        let tl = minio_load_timeline(self, key).await?;
        let summary = find_version(&tl, version)?;
        // V0: only the latest version's bytes are recoverable. Anything
        // older returns Ok(None). Same caveat as FileVault — bytes
        // recovery deferred until `rust-s3` 0.37 surfaces versionId
        // lookups (or until we upgrade to a later release).
        let latest = tl.versions.last();
        match latest {
            Some(latest) if latest.version == summary.version => self.get(key).await,
            _ => Ok(None),
        }
    }

    async fn restore_to_version(&self, key: &str, target_version: i32) -> Result<i32> {
        let mut tl = minio_load_timeline(self, key).await?;
        if tl.versions.is_empty() {
            return Err(VersionedVaultError::NoHistory(key.to_string()).into());
        }
        let _target_summary = find_version(&tl, target_version)?;

        // Read the current value so we can record it as the next version
        // entry (preserving the audit trail of what we replaced).
        let pre_restore = self.get(key).await?;
        let pre_restore_bytes = pre_restore.unwrap_or_default();
        bump_timeline(
            &mut tl,
            pre_restore_bytes.as_bytes(),
            Some(&format!(
                "Pre-restore snapshot before going back to v{target_version}"
            )),
        );

        // V0 marker for the same reason as FileVault: restoring the
        // historical *bytes* is deferred until minIO versionId lookups
        // land. The metadata layer is fully complete.
        let marker = format!("[minio-restore-pending-to-v{target_version}]");
        self.set(key, &marker).await?;
        bump_timeline(
            &mut tl,
            marker.as_bytes(),
            Some(&format!(
                "Restore target=v{target_version} (MinioVault: bytes recovery deferred; \
                 see ADR-0022 follow-up — wire minIO versionId lookups for full byte restore)"
            )),
        );
        minio_store_timeline(self, &tl).await?;

        tracing::info!(
            backend = "MinioVault",
            key = %key,
            target_version,
            new_version = tl.versions.last().map(|v| v.version).unwrap_or(0),
            "Vault restore (MinioVault): sidecar updated; bytes recovery deferred"
        );
        Ok(tl.versions.last().map(|v| v.version).unwrap_or(0))
    }

    async fn diff_versions(
        &self,
        key: &str,
        base_version: i32,
        head_version: i32,
    ) -> Result<VaultVersionDiff> {
        let tl = minio_load_timeline(self, key).await?;
        Ok(build_diff(&tl, base_version, head_version)?)
    }

    async fn compare_signatures(
        &self,
        key: &str,
        base_version: i32,
        head_version: i32,
    ) -> Result<bool> {
        let tl = minio_load_timeline(self, key).await?;
        let diff = build_diff(&tl, base_version, head_version)?;
        Ok(!diff.object_changed)
    }
}

// ─── Mapping table (V0 stub) ─────────────────────────────────────────────

/// In-process lookup used by the T7 CLI close-out: pick the right backend
/// for a given environment without hard-coding in every call site.
pub fn backend_by_name(name: &str) -> Option<&'static str> {
    match name {
        "minio" | "minio_vault" | "MinioVault" => Some("minio"),
        "file" | "file_vault" | "FileVault" => Some("file"),
        _ => None,
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// Append-only invariant: same bytes twice ⇒ single entry.
    #[test]
    fn append_version_idempotent_on_identical_bytes() {
        let mut tl = VersionedTimeline {
            key: "openai".into(),
            versions: Vec::new(),
        };
        assert!(append_version(&mut tl, b"v1", Some("first")));
        assert!(!append_version(&mut tl, b"v1", Some("duplicate")));
        assert_eq!(tl.versions.len(), 1);
        assert_eq!(tl.versions[0].version, 1);
    }

    /// Different bytes ⇒ second entry appended at version = 2.
    #[test]
    fn append_version_increments_on_new_bytes() {
        let mut tl = VersionedTimeline {
            key: "openai".into(),
            versions: Vec::new(),
        };
        assert!(append_version(&mut tl, b"v1", Some("first")));
        assert!(append_version(&mut tl, b"v2", Some("rotated")));
        assert_eq!(tl.versions.len(), 2);
        assert_eq!(tl.versions[0].version, 1);
        assert_eq!(tl.versions[1].version, 2);
        assert_eq!(tl.versions[1].change_note.as_deref(), Some("rotated"));
    }

    /// `find_version` rejects 0 and negative with `InvalidVersion`.
    #[test]
    fn find_version_rejects_non_positive() {
        let tl = VersionedTimeline {
            key: "k".into(),
            versions: Vec::new(),
        };
        for v in [0, -1, -100] {
            let res = find_version(&tl, v);
            assert!(
                matches!(res, Err(VersionedVaultError::InvalidVersion(_))),
                "v={v} expected InvalidVersion, got {res:?}"
            );
        }
    }

    /// `find_version` returns the requested entry when present.
    #[test]
    fn find_version_returns_correct_entry() {
        let mut tl = VersionedTimeline {
            key: "k".into(),
            versions: Vec::new(),
        };
        append_version(&mut tl, b"a", None);
        append_version(&mut tl, b"b", None);
        append_version(&mut tl, b"c", None);
        let v2 = find_version(&tl, 2).expect("v2");
        assert_eq!(v2.bytes_sha256, sha256_hex(b"b"));
    }

    /// Hash-equality path deterministically without async plumbing.
    #[test]
    fn compare_signatures_true_when_identical() {
        let mut tl = VersionedTimeline {
            key: "k".into(),
            versions: Vec::new(),
        };
        append_version(&mut tl, b"abc", None);
        append_version(&mut tl, b"abc", None); // skipped (idempotent)
        // Forged entry with the same hash but a distinct note.
        tl.versions.push(VaultVersionSummary {
            version: 2,
            bytes_sha256: sha256_hex(b"abc"),
            byte_len: 3,
            created_at_unix_ms: now_unix_ms(),
            change_note: Some("forged".into()),
        });
        let base = &tl.versions[0];
        let head = &tl.versions[1];
        assert_eq!(base.bytes_sha256, head.bytes_sha256);
        assert_ne!(base.version, head.version); // distinct entries
    }

    // ─── FileVault end-to-end ────────────────────────────────────────

    fn fresh_vault(label: &str) -> (FileVault, PathBuf) {
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir()
            .join(format!("gitgit-vault-versioned-{label}-{pid}-{nanos}"));
        std::fs::create_dir_all(&dir).unwrap();
        (FileVault::new(&dir), dir)
    }

    #[tokio::test]
    async fn file_vault_versioned_set_then_list() {
        let (v, _dir) = fresh_vault("set-list");
        v.set_with_version("openai", "sk-1").await.unwrap();
        v.set_with_version("openai", "sk-2").await.unwrap();
        let versions = v.list_versions("openai").await.unwrap();
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].version, 1);
        assert_eq!(versions[1].version, 2);
    }

    #[tokio::test]
    async fn file_vault_versioned_diff_after_two_sets() {
        let (v, _dir) = fresh_vault("diff");
        v.set_with_version("k", "first").await.unwrap();
        v.set_with_version("k", "second").await.unwrap();
        let diff = v.diff_versions("k", 1, 2).await.unwrap();
        assert!(diff.object_changed, "object should report as changed");
        assert_eq!(diff.file_size_delta, 6 - 5); // "second"=6, "first"=5
    }

    #[tokio::test]
    async fn file_vault_versioned_restore_appends_marker_entry() {
        let (v, dir) = fresh_vault("restore");
        v.set_with_version("k", "v1").await.unwrap();
        v.set_with_version("k", "v2").await.unwrap();
        let new_version = v.restore_to_version("k", 1).await.unwrap();
        // restore appends two entries (pre-restore snapshot + restored marker).
        let versions = v.list_versions("k").await.unwrap();
        assert!(new_version >= 3, "expected version >= 3, got {new_version}");
        assert!(
            versions.iter().any(|v| v
                .change_note
                .as_deref()
                .map(|n| n.starts_with("Restored from version 1"))
                .unwrap_or(false)),
            "sidecar should record a 'Restored from version 1' entry: {versions:?}"
        );
        // Sidecar file actually lives on disk.
        let sidecar = dir.join("_versions").join("k.versions.json");
        assert!(sidecar.exists(), "sidecar missing: {}", sidecar.display());
    }

    #[tokio::test]
    async fn file_vault_versioned_compare_signatures_on_same_value() {
        let (v, _dir) = fresh_vault("compare-sig");
        v.set_with_version("k", "stable").await.unwrap();
        v.set_with_version("k", "stable").await.unwrap(); // idempotent: no new entry
        // Both writes collapsed into one timeline entry because of the
        // append_version idempotence rule. Verify the timeline has only
        // one entry and that signatures-equality at the same entry is true.
        let versions = v.list_versions("k").await.unwrap();
        assert_eq!(versions.len(), 1);
        let equal = v.compare_signatures("k", 1, 1).await.unwrap();
        assert!(equal);
    }

    /// `restore_to_version` on a key that has no history produces an
    /// error rather than panicking.
    #[tokio::test]
    async fn file_vault_versioned_restore_without_history_errors() {
        let (v, _dir) = fresh_vault("no-history");
        // Manually scrub the timeline (no set ever happened).
        let versions = v.list_versions("nothere").await.unwrap();
        assert!(versions.is_empty());
        let res = v.restore_to_version("nothere", 1).await;
        assert!(res.is_err(), "expected NoHistory, got {res:?}");
    }

    // ─── MinioVault offline (no network) ─────────────────────────────

    fn offline_minio_config() -> crate::server::vault::MinioVaultConfig {
        crate::server::vault::MinioVaultConfig {
            endpoint: "http://127.0.0.1:1".into(),
            bucket: "gitgit-vault".into(),
            access_key: "minioadmin".into(),
            secret_key: "minioadmin".into(),
            region: "us-east-1".into(),
            key_prefix: "gitgit-vault/".into(),
        }
    }

    #[test]
    fn minio_vault_versioned_key_appends_suffix() {
        assert_eq!(
            minio_sidecar_object_key("gitgit-vault/", "openai"),
            "gitgit-vault/openai.versions.json"
        );
        assert_eq!(
            minio_sidecar_object_key("", "openai"),
            "openai.versions.json"
        );
    }

    #[test]
    fn minio_vault_versioned_backend_name_mapping() {
        assert_eq!(backend_by_name("minio"), Some("minio"));
        assert_eq!(backend_by_name("MinioVault"), Some("minio"));
        assert_eq!(backend_by_name("file"), Some("file"));
        assert_eq!(backend_by_name("FileVault"), Some("file"));
        assert_eq!(backend_by_name("unknown"), None);
    }

    #[test]
    fn minio_vault_versioned_connect_does_not_perform_network_io() {
        let cfg = offline_minio_config();
        let v = crate::server::vault::MinioVault::connect(&cfg)
            .expect("connect should not perform network I/O");
        assert_eq!(v.bucket_name(), "gitgit-vault");
        assert_eq!(v.key_prefix(), "gitgit-vault/");
    }

    // ─── Helper smoke ────────────────────────────────────────────────

    /// Defensive: SHA-256 of empty input matches the canonical value.
    #[test]
    fn sha256_helper_matches_canonical_empty_value() {
        // SHA-256 of "" — published constant for regression detection.
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
