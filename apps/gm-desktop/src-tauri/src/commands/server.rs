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
use crate::state::{resolve_bind_with_default, DesktopState, ServerStatus};

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
    let bind = resolve_bind_with_default(bind).await;

    // Construct a fresh router + listener. Each (re)start owns its own
    // `axum::serve` task; the previous instance, if any, is aborted in
    // `stop_server`.
    let vault = state.vault.clone();
    let repos_dir = state.repos_dir.clone();
    let bind_for_spawn = bind.clone();

    state
        .server
        .start_with(bind, move |b| async move {
            spawn_embedded_server(b, vault, repos_dir, bind_for_spawn)
                .await
                .map(|(handle, _addr)| handle)
        })
        .await
}

/// Stop the embedded server. Returns the prior status snapshot.
///
/// Async because `ServerManager::stop` awaits the cancelled task, so the
/// port is genuinely free by the time this resolves.
#[tauri::command]
pub async fn stop_server(state: State<'_, DesktopState>) -> AppResult<ServerStatus> {
    state.server.stop().await
}

/// Return up to `limit` recent log lines (oldest-first).
#[tauri::command]
pub fn server_logs(state: State<'_, DesktopState>, limit: Option<usize>) -> AppResult<Vec<String>> {
    Ok(state.logs.snapshot(limit.unwrap_or(200)))
}

/// Wipe the log ring buffer.
#[tauri::command]
pub fn clear_logs(state: State<'_, DesktopState>) -> AppResult<()> {
    state.logs.clear();
    Ok(())
}

/// Spawn the axum router on `bind`. Returns a tokio `JoinHandle`; the
/// caller (ServerManager) holds the handle and aborts on stop, plus the
/// address the listener actually got.
///
/// `[FACT]` The address is returned because a caller that binds port 0 —
/// the right thing for a test, which must never take the product's own
/// port — otherwise has no way to know which port the OS handed out.
///
/// `pub(crate)` because the test helper goes through this function: the
/// production path and the test path have to be the same path, or a
/// test of the wiring proves nothing about the wiring.
pub(crate) async fn spawn_embedded_server(
    bind: String,
    vault: Arc<dyn gitgit::server::vault_versioned::VersionedVault>,
    repos_dir: std::path::PathBuf,
    bind_for_spawn: String,
) -> AppResult<(tokio::task::JoinHandle<()>, std::net::SocketAddr)> {
    // Build the gitgit router via `gitgit::server::http` — same code
    // path the CLI uses, zero duplication.
    use gitgit::config::Config;
    use gitgit::server::auth::{
        enforce_exposure_policy, resolve_admin_credential, AdminCredentialStore,
    };
    use gitgit::server::http::{build_router, AppState};

    // Resolve the admin credential from the **desktop** vault — the one
    // Settings writes `gitgit.password` into. It is the same vault the
    // server's state gets below, so the password the user typed and the
    // keys the API serves come from one place.
    let resolved = resolve_admin_credential(vault.as_ref()).await?;
    // Same policy as the CLI: a non-loopback bind with a password nobody
    // chose is refused rather than served.
    enforce_exposure_policy(&bind, resolved.source).map_err(|e| AppError::Bind(e.to_string()))?;
    if resolved.source == gitgit::server::auth::CredentialSource::Generated {
        // No password is configured. The embedded server binds loopback
        // by default, so an unlearnable random password is safe — but
        // unlike the CLI there is no console to print it to, so say
        // plainly that one exists and where to change it.
        tracing::info!(
            "no admin password configured; generated a random one for this server. \
             Set one in Settings, or export {}, before exposing the bind address.",
            gitgit::config::ADMIN_PASS_ENV
        );
    } else {
        tracing::info!(
            source = resolved.source.as_str(),
            "admin password loaded for the embedded server"
        );
    }
    let admin = Arc::new(AdminCredentialStore::from_shared(resolved.credential));

    let cfg = Config::new(bind.clone(), repos_dir, std::path::PathBuf::from("."));
    let _bind_for_spawn = bind_for_spawn;
    let cfg_arc = std::sync::Arc::new(cfg.clone());
    // The state's vault is **the desktop's vault** — the same instance the
    // admin password was resolved from.
    //
    // It used to be a fresh `FileVault` rooted at the config's vault root,
    // which was the process working directory: the Settings screen wrote
    // keys into the app-data vault while `/api/vault/*` read and wrote a
    // different one, and an API write landed in the working directory.
    let state = AppState::with_admin(cfg.clone(), vault, admin);

    let router = build_router(state);
    let listener = tokio::net::TcpListener::bind(&cfg_arc.bind)
        .await
        .map_err(|e| AppError::Bind(format!("bind {bind}: {e}")))?;
    let addr = listener
        .local_addr()
        .map_err(|e| AppError::Bind(format!("local_addr {bind}: {e}")))?;
    tracing::info!(bind = %cfg_arc.bind, "embedded gitgit server listening");

    let handle = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, router).await {
            tracing::error!(error = %e, "embedded server exited");
        }
    });
    Ok((handle, addr))
}
