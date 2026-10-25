//! Cross-cutting utility commands: clipboard, app data dir, version.

use serde::Serialize;
use tauri::State;

use crate::error::{AppError, AppResult};
use crate::state::DesktopState;

/// Information about the desktop shell itself — surfaced via the
/// Settings page footer.
#[derive(Debug, Clone, Serialize)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub data_dir: String,
    pub config_dir: String,
    pub repos_dir: String,
    pub vault_dir: String,
    pub current_locale: String,
}

/// Convenience for the i18n bundle / Settings page header.
#[tauri::command]
pub fn app_info(state: State<'_, DesktopState>) -> AppResult<AppInfo> {
    let name = String::from(env!("CARGO_PKG_NAME"));
    let version = String::from(env!("CARGO_PKG_VERSION"));
    Ok(AppInfo {
        name,
        version,
        data_dir: state.app_data_dir.to_string_lossy().into_owned(),
        config_dir: state.app_data_dir.to_string_lossy().into_owned(),
        repos_dir: state.repos_dir.to_string_lossy().into_owned(),
        vault_dir: state.vault_root.to_string_lossy().into_owned(),
        current_locale: current_locale_str(),
    })
}

/// Validate a clipboard payload.
///
/// The Svelte layer copies text with the Web Clipboard API
/// (`navigator.clipboard.writeText`, `src/lib/utils/clipboard.ts`), not
/// with a Tauri plugin, and this command does not touch a plugin either.
/// It exists so that the payload contract — a non-empty string — has a
/// testable home.
///
/// The desktop app does not register `tauri-plugin-clipboard-manager` and
/// grants it no capability; `tests/unit/capability-surface-gate.test.ts`
/// asserts that.
#[tauri::command]
pub fn set_clipboard_text(text: String) -> AppResult<()> {
    if text.is_empty() {
        return Err(AppError::Bridge("empty clipboard payload".into()));
    }
    Ok(())
}

/// Quickly check whether the embedded vault is reachable without
/// surfacing its contents.
#[tauri::command]
pub async fn vault_diagnostics(state: State<'_, DesktopState>) -> AppResult<VaultDiagnostic> {
    let vault = state.vault.clone();
    let keys = vault.list().await?;
    Ok(VaultDiagnostic {
        reachable: true,
        key_count: keys.len(),
        backend: String::from("FileVault"),
        root: state.vault_root.to_string_lossy().into_owned(),
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct VaultDiagnostic {
    pub reachable: bool,
    pub key_count: usize,
    pub backend: String,
    pub root: String,
}

fn current_locale_str() -> String {
    sys_locale::get_locale()
        .unwrap_or_else(|| String::from("zh-CN"))
        .replace('-', "_")
}

/// Minimal locale detection that prefers system-locale over hardcoded
/// `zh-CN`. We keep it in a separate module-like scope here so tests
/// can mock if needed.
mod sys_locale {
    pub fn get_locale() -> Option<String> {
        std::env::var("LC_ALL")
            .ok()
            .or_else(|| std::env::var("LANG").ok())
            .map(|s| {
                let mut out = s.clone();
                if let Some(idx) = out.find('.') {
                    out.truncate(idx);
                }
                out
            })
            .filter(|s| !s.is_empty())
    }
}
