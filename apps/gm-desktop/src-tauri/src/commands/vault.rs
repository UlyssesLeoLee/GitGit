//! Credential Vault commands.
//!
//! Backed by `gitgit::server::vault::FileVault` and
//! `gitgit::server::vault_versioned::VersionedVault`. The UI uses the
//! version-management surface (list_versions / diff / restore) per
//! ADR-0022 §2.
//!
//! Migration note: the upstream `vault_versioned` types are
//! `VaultVersionSummary` / `VaultVersionDiff` (this file's prior draft
//! called them `VersionEntry` / `VersionDiff`; the rename landed in the
//! gitgit server crate in commit d3d184e and gm-desktop had not been
//! updated since 2f8b9fc). We bridge here with `VersionEntryDto` /
//! `VersionDiffDto` so the on-the-wire JSON contract with the Svelte
//! frontend is unchanged.

use gitgit::server::vault::Vault;
use gitgit::server::vault_versioned::{VaultVersionDiff, VaultVersionSummary, VersionedVault};
use serde::Serialize;
use tauri::State;

use crate::error::{AppError, AppResult};
use crate::state::DesktopState;

/// Truncated view of a single version entry — friendly to the UI.
#[derive(Debug, Clone, Serialize)]
pub struct VersionEntryDto {
    pub version: i32,
    pub bytes_sha256: String,
    pub byte_len: u64,
    /// Forwarded as `i64` from the upstream `VaultVersionSummary`; the
    /// Svelte side reads it as a JS number so the signed/unsigned
    /// difference does not affect UI behaviour for any plausible
    /// 2026-2030 timestamp.
    pub created_at_unix_ms: i64,
    pub change_note: Option<String>,
}

impl From<VaultVersionSummary> for VersionEntryDto {
    fn from(e: VaultVersionSummary) -> Self {
        Self {
            version: e.version,
            bytes_sha256: e.bytes_sha256,
            byte_len: e.byte_len,
            created_at_unix_ms: e.created_at_unix_ms,
            change_note: e.change_note,
        }
    }
}

/// Diff payload between two versions.
#[derive(Debug, Clone, Serialize)]
pub struct VersionDiffDto {
    pub key: String,
    pub base_version: i32,
    pub head_version: i32,
    pub object_changed: bool,
    pub file_size_delta: i64,
}

impl VersionDiffDto {
    fn from_parts(key: String, d: VaultVersionDiff) -> Self {
        Self {
            key,
            base_version: d.base.version,
            head_version: d.head.version,
            object_changed: d.object_changed,
            file_size_delta: d.file_size_delta,
        }
    }
}

#[tauri::command]
pub async fn vault_list(state: State<'_, DesktopState>) -> AppResult<Vec<String>> {
    let vault = state.vault.clone();
    let keys = vault.list().await?;
    Ok(keys)
}

#[tauri::command]
pub async fn vault_get(
    state: State<'_, DesktopState>,
    key: String,
) -> AppResult<Option<String>> {
    let vault = state.vault.clone();
    let value = vault.get(&key).await?;
    Ok(value)
}

#[tauri::command]
pub async fn vault_set(
    state: State<'_, DesktopState>,
    key: String,
    value: String,
) -> AppResult<i32> {
    let vault = state.vault.clone();
    let new_version = vault.set_with_version(&key, &value).await?;
    Ok(new_version)
}

#[tauri::command]
pub async fn vault_rotate(
    state: State<'_, DesktopState>,
    key: String,
) -> AppResult<()> {
    let vault = state.vault.clone();
    vault.rotate(&key).await?;
    Ok(())
}

#[tauri::command]
pub async fn vault_versions(
    state: State<'_, DesktopState>,
    key: String,
) -> AppResult<Vec<VersionEntryDto>> {
    let vault = state.vault.clone();
    let entries = vault.list_versions(&key).await?;
    Ok(entries.into_iter().map(VersionEntryDto::from).collect())
}

#[tauri::command]
pub async fn vault_diff(
    state: State<'_, DesktopState>,
    key: String,
    base: i32,
    head: i32,
) -> AppResult<VersionDiffDto> {
    let vault = state.vault.clone();
    let diff = vault.diff_versions(&key, base, head).await?;
    Ok(VersionDiffDto::from_parts(key, diff))
}

#[tauri::command]
pub async fn vault_restore(
    state: State<'_, DesktopState>,
    key: String,
    target_version: i32,
) -> AppResult<i32> {
    let vault = state.vault.clone();
    let new_version = vault.restore_to_version(&key, target_version).await?;
    Ok(new_version)
}

#[tauri::command]
pub async fn vault_delete(
    state: State<'_, DesktopState>,
    key: String,
) -> AppResult<()> {
    let vault = state.vault.clone();
    vault.delete(&key).await?;
    Ok(())
}

#[allow(dead_code)]
fn validate_key(key: &str) -> AppResult<()> {
    if key.is_empty() {
        return Err(AppError::InvalidRepoName(key.to_string()));
    }
    if key.contains("..") {
        return Err(AppError::InvalidRepoName(key.to_string()));
    }
    Ok(())
}
