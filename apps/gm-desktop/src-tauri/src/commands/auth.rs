//! Auth-related Tauri commands.
//!
//! V0.1 only exposes admin password (read/write).
//!
//! # This password is now load-bearing
//!
//! `[FACT]` This file used to carry the note that "the HTTP basic-auth
//! check in `gitgit::server::auth` continues to use the hard-coded
//! default (`admin` / `admin`); the UI here mirrors the same vault key
//! (`gitgit.password`) so the next iteration of `auth.rs` can read it
//! without any contract change." That iteration is this one:
//! `gitgit::server::auth::resolve_admin_credential` reads
//! [`ADMIN_PASSWORD_VAULT_KEY`] at server startup, so a password saved
//! here is the password the server checks.
//!
//! # What is still true: a restart is required
//!
//! The embedded server resolves the credential once, when it starts, and
//! caches it (see `gitgit::server::auth` for why). Changing the password
//! while the server is running therefore updates the vault immediately
//! but the *server* picks it up on its next start. The Settings page
//! says so out loud rather than showing a success toast and leaving the
//! user to discover otherwise.

use serde::Serialize;
use tauri::State;

use crate::error::AppResult;
use crate::state::DesktopState;

/// Vault key under which the admin password is stored.
///
/// Re-exported from the library rather than re-declared: the desktop
/// writes it and the server reads it, and two constants that are
/// supposed to be the same string is exactly the kind of drift that
/// makes a password look configured when it is not.
pub use gitgit::config::ADMIN_PASSWORD_VAULT_KEY;

#[derive(Debug, Clone, Serialize)]
pub struct AdminPasswordStatus {
    pub is_set: bool,
    /// Length of the stored value, never the value itself. The brief
    /// explicitly forbids leaking credentials to the UI; we keep this
    /// here so the Settings page can show whether one is present.
    pub length: usize,
}

#[tauri::command]
pub async fn get_admin_password_status(
    state: State<'_, DesktopState>,
) -> AppResult<AdminPasswordStatus> {
    let vault = state.vault.clone();
    let value = vault.get(ADMIN_PASSWORD_VAULT_KEY).await?;
    match value {
        Some(v) => Ok(AdminPasswordStatus {
            is_set: true,
            length: v.len(),
        }),
        None => Ok(AdminPasswordStatus {
            is_set: false,
            length: 0,
        }),
    }
}

#[tauri::command]
pub async fn set_admin_password(
    state: State<'_, DesktopState>,
    password: String,
) -> AppResult<i32> {
    if password.is_empty() {
        return Ok(0);
    }
    let vault = state.vault.clone();
    let new_version = vault
        .set_with_version(ADMIN_PASSWORD_VAULT_KEY, &password)
        .await?;
    Ok(new_version)
}

#[tauri::command]
pub async fn clear_admin_password(state: State<'_, DesktopState>) -> AppResult<()> {
    let vault = state.vault.clone();
    vault.delete(ADMIN_PASSWORD_VAULT_KEY).await?;
    Ok(())
}
