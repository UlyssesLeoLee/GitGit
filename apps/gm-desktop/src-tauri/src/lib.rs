//! Tauri builder, tray icon, plugin wiring, and command registration.
//!
//! This is the integration point where the Svelte frontend meets the
//! embedded gitgit library. Per ADR-0020 §2.2 we embed `axum` in the
//! same process; the start/stop semantics are provided by
//! `crate::state::ServerManager`.

use std::path::PathBuf;
use std::sync::Arc;

use gitgit::config::Config;
use gitgit::server::http::{build_router, AppState};
use gitgit::server::vault::FileVault;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager, RunEvent, WindowEvent,
};

use crate::commands;
use crate::error::{AppError, AppResult};
use crate::state::{BufferLayer, DesktopState, LogBuffer, DEFAULT_BIND};

/// Build the `tauri::Builder` and start the runtime. `main.rs` wraps
/// this in a single `gm_desktop_lib::run()` call so the integration
/// tests can mount a bare command surface without spawning a window.
pub fn run() {
    init_tracing();
    tauri::Builder::default()
        .plugin(tauri_plugin_log::init_with_config(
            tauri_plugin_log::Config::default(),
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| setup_app(app))
        .invoke_handler(tauri::generate_handler![
            commands::server::server_status,
            commands::server::start_server,
            commands::server::stop_server,
            commands::server::server_logs,
            commands::server::clear_logs,
            commands::repos::list_repos,
            commands::repos::repo_detail,
            commands::repos::clone_url,
            commands::repos::open_repo_in_shell,
            commands::vault::vault_list,
            commands::vault::vault_get,
            commands::vault::vault_set,
            commands::vault::vault_rotate,
            commands::vault::vault_versions,
            commands::vault::vault_diff,
            commands::vault::vault_restore,
            commands::vault::vault_delete,
            commands::auth::get_admin_password_status,
            commands::auth::set_admin_password,
            commands::auth::clear_admin_password,
            commands::system::app_info,
            commands::system::set_clipboard_text,
            commands::system::vault_diagnostics,
        ])
        .on_window_event(handle_window_event)
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(handle_run_event);
}

/// Build a `DesktopState` for unit testing — returned to integration
/// tests so they can invoke commands against an in-memory router.
#[cfg(any(test, feature = "test-support"))]
pub async fn build_test_state(
    app_data_dir: PathBuf,
    repos_dir: PathBuf,
) -> AppResult<DesktopState> {
    DesktopState::new(app_data_dir.join("vault"), repos_dir, app_data_dir)
}

fn setup_app(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let app_handle = app.handle();
    let app_data_dir: PathBuf = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("gitgit-desktop"));
    let vault_dir = app_data_dir.join("vault");
    let repos_dir = app_data_dir.join("repos");

    let desktop_state = DesktopState::new(vault_dir, repos_dir, app_data_dir.clone())?;

    // Mirror the desktop state's log buffer into the global tracing
    // subscriber so the UI's log-fetch endpoint sees all events,
    // including those raised by the embedded axum server task.
    let buffer_layer = BufferLayer {
        buffer: desktop_state.logs.clone(),
    };
    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")))
        .with(buffer_layer)
        .try_init();

    // Tray icon. The tray menu has three actions: Show Window / Hide
    // Window / Quit. Clicking "Show" brings the main window to the
    // front; "Hide" hides the window without quitting the app.
    let tray_menu = build_tray_menu(app_handle)?;
    let tray_icon_bytes = include_bytes!("../icons/tray.png");
    let tray_image = Image::from_bytes(tray_icon_bytes)?;

    let _tray = TrayIconBuilder::with_id("main-tray")
        .tooltip("gitgit Desktop")
        .icon(tray_image)
        .menu(&tray_menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
            "hide" => {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.hide();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    app.manage(desktop_state);
    Ok(())
}

fn handle_window_event(window: &tauri::Window, event: &WindowEvent) {
    match event {
        WindowEvent::CloseRequested { api, .. } => {
            // Brief requirement: "关闭按钮最小化到托盘". Prevent
            // the default close and hide the window instead.
            api.prevent_close();
            let _ = window.hide();
        }
        _ => {}
    }
}

fn handle_run_event(app: &AppHandle, event: RunEvent) {
    if let RunEvent::ExitRequested { api, .. } = event {
        // We never auto-exit; the user must explicitly choose Quit
        // from the tray menu.
        api.prevent_exit();
        let _ = app;
    }
}

fn build_tray_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let show = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "hide", "Hide Window", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    Menu::with_items(app, &[&show, &hide, &quit])
}

/// Wire the public tracing subscriber to stdout + the in-memory log
/// buffer so the Svelte UI can stream recent log records.
fn init_tracing() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(std::io::stdout))
        .try_init();
}

/// Public re-export so integration tests can build a `gitgit`
/// `Config` / `FileVault` / `AppState` directly without depending on
/// the internal layout of the gitgit library.
pub use gitgit as gitgit_lib;

/// Convenience: spawn the embedded axum server. Used by integration
/// tests; the production `start_server` command shells through the
/// [`crate::state::ServerManager`].
pub async fn run_embedded_router(
    bind: String,
    vault_root: PathBuf,
    repos_dir: PathBuf,
) -> AppResult<tokio::task::JoinHandle<()>> {
    let cfg = Config::new(bind.clone(), repos_dir, vault_root.clone());
    let file_vault = FileVault::new(&cfg.vault_file_root);
    let state = AppState::new(cfg.clone(), Arc::new(file_vault));
    let router = build_router(state);
    let listener = tokio::net::TcpListener::bind(&cfg.bind)
        .await
        .map_err(|e| AppError::Bind(format!("bind {bind}: {e}")))?;
    let handle = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, router).await {
            tracing::error!(error = %e, "embedded router exited");
        }
    });
    Ok(handle)
}

#[allow(unused_imports)]
use tracing_subscriber::prelude::*;

// Avoid unused import warnings for items used only on some targets.
#[allow(dead_code)]
const DEFAULT_BIND_DUP: &str = DEFAULT_BIND;

// Add LogBuffer re-export for tests.
pub use crate::state::LogBuffer as LogBufferPublic;
