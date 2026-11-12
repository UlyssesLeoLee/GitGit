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

use gitgit::server::vault_versioned::{VaultVersionDiff, VaultVersionSummary};
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

/// Fetch a stored credential.
///
/// `[PROPOSAL]` ADR-0024 §3.9 item 3: the `ai/` namespace is excluded from
/// the generic read path, so a provider key can never travel to the
/// webview. The key is written by `ai_key_set` and read by the AI call
/// itself, on the Rust side; there is deliberately no `ai_key_get`.
///
/// `[INFERENCE]` The *name* of an AI key still shows up in `vault_list`,
/// because ADR-0024 §3.9's landing requirement is "list the name, do not
/// fetch the value" — the settings page can say "a key is configured for
/// openai" without ever seeing it.
#[tauri::command]
pub async fn vault_get(state: State<'_, DesktopState>, key: String) -> AppResult<Option<String>> {
    vault_get_inner(state.vault.as_ref(), &key).await
}

/// The read path without the Tauri state, so it can be exercised directly.
pub async fn vault_get_inner<V: gitgit::server::vault::Vault + ?Sized>(
    vault: &V,
    key: &str,
) -> AppResult<Option<String>> {
    if gitgit::ai::keys::is_ai_key(key) {
        return Err(AppError::Vault(String::from(
            "the ai/ namespace is not readable from the ui; a provider key never leaves the backend",
        )));
    }
    let value = vault.get(key).await?;
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
pub async fn vault_rotate(state: State<'_, DesktopState>, key: String) -> AppResult<()> {
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
pub async fn vault_delete(state: State<'_, DesktopState>, key: String) -> AppResult<()> {
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

#[cfg(test)]
mod tests {
    // The crate lints deny `unwrap_used` / `expect_used` / `panic`, which
    // are the normal vocabulary of test assertions. They are re-enabled
    // for this module only, which is compiled exclusively under
    // `cfg(test)` and never reaches the shipped binary.
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use gitgit::server::vault::{FileVault, Vault};

    use super::*;

    /// A file-backed vault in a fresh temporary directory.
    fn temp_vault(label: &str) -> FileVault {
        let dir = std::env::temp_dir().join(format!(
            "gitgit-desktop-vault-{label}-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        FileVault::new(dir)
    }

    #[tokio::test]
    async fn the_ai_namespace_is_not_readable_through_the_generic_path() {
        // ADR-0024 §3.9 item 3: the `ai/` namespace is excluded from the
        // generic read path, so a provider key can never reach the
        // webview.
        let vault = temp_vault("ai-namespace");
        vault
            .set("ai/openai", "sk-secret-value")
            .await
            .expect("store");
        let err = vault_get_inner(&vault, "ai/openai")
            .await
            .expect_err("the ai/ namespace is refused");
        assert_eq!(err.kind(), "Vault");
        assert!(
            !err.to_string().contains("sk-secret-value"),
            "the key must not be echoed: {err}"
        );
    }

    #[tokio::test]
    async fn an_ordinary_key_still_reads() {
        // The exclusion is narrow: only the `ai/` namespace is refused.
        let vault = temp_vault("ordinary");
        vault.set("openai", "sk-other").await.expect("store");
        assert_eq!(
            vault_get_inner(&vault, "openai").await.expect("read"),
            Some(String::from("sk-other"))
        );
    }
}
