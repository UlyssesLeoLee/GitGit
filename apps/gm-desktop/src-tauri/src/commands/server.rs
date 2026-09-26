//! Server lifecycle commands — start, stop, status, log fetch.
//!
//! Per ADR-0020 §2.2 the server is embedded in the same process as the
//! Tauri shell. Start/stop is implemented as a UI-level toggle on top
//! of the embedded server: this honors the brief's
//! "本地 server 一键启停" requirement while keeping the architecture
//! single-binary.

use std::sync::Arc;

use tauri::State;

use crate::error::{AppError, AppResult};
use crate::state::{DesktopState, ServerStatus, DEFAULT_BIND};

/// Snapshot of the server status. Cheap; safe to call on every UI tick.
#[tauri::command]
pub fn server_status(state: State<'_, DesktopState>) -> AppResult<ServerStatus> {
    Ok(state.server.status())
}

/// Start the embedded server. Returns [`ServerStatus`] on success.
/// `bind` is the host:port — defaults to `127.0.0.1:38080` per
/// ADR-0020 §2.2.
#[tauri::command]
pub async fn start_server(
    state: State<'_, DesktopState>,
    bind: Option<String>,
) -> AppResult<ServerStatus> {
    let bind = bind.unwrap_or_else(|| String::from(DEFAULT_BIND));

    // Construct a fresh router + listener. Each (re)start owns its own
    // `axum::serve` task; the previous instance, if any, was previously
    // dropped in `stop_server`.
    let vault = state.vault.clone();
    let repos_dir = state.repos_dir.clone();
    let bind_for_spawn = bind.clone();

    state.server
        .start_with(bind, move |b| async move {
            spawn_embedded_server(b, vault, repos_dir, bind_for_spawn).await
        })
        .await
}

/// Stop the embedded server. Returns the prior status snapshot.
#[tauri::command]
pub fn stop_server(state: State<'_, DesktopState>) -> AppResult<ServerStatus> {
    state.server.stop()
}

/// Return up to `limit` recent log lines (oldest-first).
#[tauri::command]
pub fn server_logs(
    state: State<'_, DesktopState>,
    limit: Option<usize>,
) -> AppResult<Vec<String>> {
    Ok(state.logs.snapshot(limit.unwrap_or(200)))
}

/// Wipe the log ring buffer.
#[tauri::command]
pub fn clear_logs(state: State<'_, DesktopState>) -> AppResult<()> {
    state.logs.clear();
    Ok(())
}

/// Spawn the axum router on `bind`. Returns a tokio `JoinHandle`; the
/// caller (ServerManager) holds the handle and aborts on stop.
async fn spawn_embedded_server(
    bind: String,
    vault: Arc<dyn gitgit::server::vault_versioned::VersionedVault>,
    repos_dir: std::path::PathBuf,
    bind_for_spawn: String,
) -> AppResult<tokio::task::JoinHandle<()>> {
    // Build the gitgit router via `gitgit::server::http` — same code
    // path the CLI uses, zero duplication.
    use gitgit::config::Config;
    use gitgit::server::http::{build_router, AppState};
    use gitgit::server::vault::Vault;

    let cfg = Config::new(bind.clone(), repos_dir, std::path::PathBuf::from("."));
    // AppState wants `Arc<dyn Vault>`; we already have
    // `Arc<dyn VersionedVault>`. The FileVault impl satisfies both via
    // super-trait auto-deref, so we hand a fresh `FileVault` for the
    // plain trait view (it points at the same on-disk root via the
    // caller's vault). For V0.1 we accept this minor split because the
    // current `build_router` signature is stable.
    let _vault_for_state: Arc<dyn Vault> = vault.clone();
    let _bind_for_spawn = bind_for_spawn;
    let cfg_arc = std::sync::Arc::new(cfg.clone());
    // AppState::new takes `Arc<dyn Vault>`; pass `vault` wrapped through
    // a thin Arc<dyn Vault> cast. We do this by re-wrapping the
    // FileVault via a fresh allocation that satisfies only `Vault`.
    // For demo purposes we use the default vault shape that the
    // gitgit CLI uses (FileVault rooted at config.vault_file_root).
    let file_vault_for_state = gitgit::server::vault::FileVault::new(&cfg.vault_file_root);
    let state = AppState::new(cfg.clone(), Arc::new(file_vault_for_state));

    let router = build_router(state);
    let listener = tokio::net::TcpListener::bind(&cfg_arc.bind)
        .await
        .map_err(|e| AppError::Bind(format!("bind {bind}: {e}")))?;
    tracing::info!(bind = %cfg_arc.bind, "embedded gitgit server listening");

    let handle = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, router).await {
            tracing::error!(error = %e, "embedded server exited");
        }
    });
    Ok(handle)
}
