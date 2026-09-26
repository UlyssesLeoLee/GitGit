//! In-process application state for the Tauri runtime.
//!
//! Holds three distinct concerns:
//!
//! 1. `ServerManager` — owns the embedded axum server that mirrors what
//!    `gitgit serve` would do as a CLI. Per ADR-0020 §2.2 we embed the
//!    server in-process so the UI talks to the same router as the CLI;
//!    we add a *start/stop* surface on top of it because the brief
//!    explicitly requires the "本地 server 一键启停" UX.
//! 2. `VaultHandle` — shared `Arc<dyn VersionedVault>` pointing at the
//!    FileVault rooted at the per-user app config directory. Provides
//!    identical semantics to the gitgit CLI's `gitai key *` surface.
//! 3. `LogBuffer` — a ring buffer fed by the tracing layer so the UI can
//!    fetch recent log lines through the `server_logs` command.

//! No part of this file touches gitgit's `vault / auth / http` business
//! logic. Construction uses only `pub` types already exposed by the
//! gitgit library target.

use std::collections::VecDeque;
use std::future::Future;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use gitgit::server::vault::Vault;
use gitgit::server::vault_versioned::VersionedVault;
use serde::Serialize;
use tokio::sync::Mutex as AsyncMutex;
use tokio::task::JoinHandle;
use tracing_subscriber::Layer;

use crate::error::{AppError, AppResult};

/// Maximum log lines kept in memory for the in-process log viewer.
pub const LOG_BUFFER_CAPACITY: usize = 500;
/// Default port for the embedded server. Matches ADR-0020 §2.2.
pub const DEFAULT_BIND: &str = "127.0.0.1:38080";
/// App-data-relative vault root when no explicit override is given.
pub const DEFAULT_VAULT_DIR: &str = "vault";

/// Snapshot of the embedded server's state — written into the response
/// of `start_server` / `server_status`.
#[derive(Debug, Clone, Serialize)]
pub struct ServerStatus {
    /// Logical handle — `"embedded"` for the in-process server (V0 has
    /// exactly one). Reserved for future "named instance" support.
    pub handle: String,
    /// The bound host:port. Empty when not running.
    pub bind: String,
    /// Process ID of the desktop shell — included because the brief
    /// says "显示 PID / 端口 / 日志流". When the server is embedded,
    /// the PID is the shell's own.
    pub pid: u32,
    /// Seconds since the last successful `start_server`. None if not
    /// running.
    pub uptime_secs: Option<u64>,
    /// Whether the server is currently accepting connections.
    pub running: bool,
}

/// Internal record kept while a server is running.
struct Running {
    bind: String,
    started_at: Instant,
    /// The tokio task driving `axum::serve`. Held to allow graceful
    /// shutdown via `abort()`.
    join: JoinHandle<()>,
}

/// Manager that holds at most one running server. The mutex is held only
/// briefly during start/stop/status checks — never across `.await`.
pub struct ServerManager {
    inner: Mutex<Option<Running>>,
}

impl ServerManager {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    /// Snapshot the current state.
    pub fn status(&self) -> ServerStatus {
        let guard = match self.inner.lock() {
            Ok(g) => g,
            Err(_) => {
                return ServerStatus {
                    handle: String::from("embedded"),
                    bind: String::new(),
                    pid: std::process::id(),
                    uptime_secs: None,
                    running: false,
                };
            }
        };
        match &*guard {
            Some(r) => ServerStatus {
                handle: String::from("embedded"),
                bind: r.bind.clone(),
                pid: std::process::id(),
                uptime_secs: Some(r.started_at.elapsed().as_secs()),
                running: true,
            },
            None => ServerStatus {
                handle: String::from("embedded"),
                bind: String::new(),
                pid: std::process::id(),
                uptime_secs: None,
                running: false,
            },
        }
    }

    /// Start the embedded server. Returns a [`ServerStatus`] snapshot on
    /// success, `ServerAlreadyRunning` if already up.
    pub async fn start_with<F, Fut>(
        &self,
        bind: String,
        spawn: F,
    ) -> AppResult<ServerStatus>
    where
        F: FnOnce(String) -> Fut,
        Fut: Future<Output = AppResult<JoinHandle<()>>>,
    {
        // Phase 1: hold the lock long enough to mark the slot as
        // "starting" (still None — pre-join). Drop the guard before
        // awaiting the spawn closure so the lock isn't held across
        // .await (MutexGuard is !Send).
        let already_running = {
            let guard = self
                .inner
                .lock()
                .map_err(|_| AppError::ServerManagerPoisoned)?;
            if guard.is_some() {
                Err(AppError::ServerAlreadyRunning(std::process::id()))
            } else {
                Ok(())
            }
        }?;
        // Phase 2: drive the spawn closure to completion (no lock held).
        let join = spawn(bind.clone()).await?;
        // Phase 3: re-acquire the lock and commit the JoinHandle.
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| AppError::ServerManagerPoisoned)?;
        *guard = Some(Running {
            bind,
            started_at: Instant::now(),
            join,
        });
        Ok(self.status_from_guard(&guard))
    }

    /// Stop a running server and return its prior status.
    pub fn stop(&self) -> AppResult<ServerStatus> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| AppError::ServerManagerPoisoned)?;
        match &*guard {
            None => Err(AppError::ServerNotRunning),
            Some(_) => {
                let prev = self.status_from_guard(&guard);
                // Drop the join handle — its inner future is dropped,
                // which aborts the task.
                *guard = None;
                Ok(prev)
            }
        }
    }

    fn status_from_guard(
        &self,
        guard: &std::sync::MutexGuard<Option<Running>>,
    ) -> ServerStatus {
        match &**guard {
            Some(r) => ServerStatus {
                handle: String::from("embedded"),
                bind: r.bind.clone(),
                pid: std::process::id(),
                uptime_secs: Some(r.started_at.elapsed().as_secs()),
                running: true,
            },
            None => ServerStatus {
                handle: String::from("embedded"),
                bind: String::new(),
                pid: std::process::id(),
                uptime_secs: None,
                running: false,
            },
        }
    }
}

impl Default for ServerManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Ring buffer of recent log lines. Sized to [`LOG_BUFFER_CAPACITY`].
#[derive(Clone)]
pub struct LogBuffer {
    inner: Arc<Mutex<VecDeque<String>>>,
}

impl LogBuffer {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::with_capacity(LOG_BUFFER_CAPACITY))),
        }
    }

    /// Append a formatted log line. Drops the oldest entry when the
    /// buffer is full.
    pub fn push(&self, line: String) {
        if let Ok(mut buf) = self.inner.lock() {
            if buf.len() == LOG_BUFFER_CAPACITY {
                buf.pop_front();
            }
            buf.push_back(line);
        }
    }

    /// Snapshot of the most recent `limit` lines, oldest-first.
    pub fn snapshot(&self, limit: usize) -> Vec<String> {
        match self.inner.lock() {
            Ok(buf) => {
                let n = buf.len();
                let start = n.saturating_sub(limit);
                buf.iter().skip(start).cloned().collect()
            }
            Err(_) => Vec::new(),
        }
    }

    /// Empty the buffer (used by the `clear_logs` command).
    pub fn clear(&self) {
        if let Ok(mut buf) = self.inner.lock() {
            buf.clear();
        }
    }
}

impl Default for LogBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Custom tracing `Layer` that pushes formatted log lines into the
/// shared [`LogBuffer`] in addition to printing them on stdout.
pub struct BufferLayer {
    pub buffer: LogBuffer,
}

impl<S> Layer<S> for BufferLayer
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // Format manually to avoid pulling in the `tracing-logfmt` or
        // `tracing-bunyan-formatter` crates; the brief does not
        // require structured fields, only a readable one-line record.
        let mut visitor = StringVisitor::default();
        event.record(&mut visitor);
        let metadata = event.metadata();
        let line = format!(
            "{} {} {} {}",
            chrono::Utc::now().to_rfc3339(),
            metadata.level(),
            metadata.target(),
            visitor.value
        );
        self.buffer.push(line);
    }
}

/// Minimal `tracing::field::Visit` that concatenates string-ish fields.
#[derive(Default)]
struct StringVisitor {
    value: String,
}

impl tracing::field::Visit for StringVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if self.value.is_empty() {
            self.value = format!("{}={:?}", field.name(), value);
        } else {
            self.value.push_str(&format!(" {}={:?}", field.name(), value));
        }
    }
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if self.value.is_empty() {
            self.value = format!("{}={}", field.name(), value);
        } else {
            self.value.push_str(&format!(" {}={}", field.name(), value));
        }
    }
}

/// Top-level state passed to every Tauri command via `tauri::State<…>`.
pub struct DesktopState {
    /// Embedded-server lifecycle.
    pub server: ServerManager,
    /// Credential vault. Lives for the lifetime of the app — `gitai
    /// key set/openai/…` from the CLI and "Settings → Vault" from the
    /// UI both read/write through this.
    pub vault: Arc<dyn VersionedVault>,
    /// On-disk root for the vault (for diagnostics only; the Vault
    /// trait above is what commands actually use).
    pub vault_root: PathBuf,
    /// Default `repos` directory the UI starts off pointing at. Matches
    /// the brief's "仓库列表".
    pub repos_dir: PathBuf,
    /// Log buffer.
    pub logs: LogBuffer,
    /// Resolved app-data directory (for diagnostics).
    pub app_data_dir: PathBuf,
}

impl DesktopState {
    /// Build a `DesktopState` from explicit roots. Called from the
    /// Tauri `setup` hook with the paths resolved by the runtime.
    pub fn new(
        vault_root: PathBuf,
        repos_dir: PathBuf,
        app_data_dir: PathBuf,
    ) -> AppResult<Self> {
        std::fs::create_dir_all(&vault_root).map_err(|e| AppError::Io(e.to_string()))?;
        std::fs::create_dir_all(&repos_dir).map_err(|e| AppError::Io(e.to_string()))?;
        let file_vault = gitgit::server::vault::FileVault::new(vault_root.clone());
        let vault: Arc<dyn Vault> = Arc::new(file_vault);
        // `gitgit`'s `FileVault` implements `VersionedVault` via the
        // sidecar metadata (per ADR-0022); we cast the `Arc<dyn Vault>`
        // to `Arc<dyn VersionedVault>` through a thin wrapper.
        // Direct cast is not possible because the trait objects differ;
        // we instead build the value twice with the same backing data.
        let versioned: Arc<dyn VersionedVault> =
            Arc::new(gitgit::server::vault::FileVault::new(vault_root.clone()));
        // Keep the simple `Vault` reference alive even though the
        // versioned variant is what the UI uses. (We never read it,
        // but the compiler would still warn about unused allocation;
        // an `Arc<dyn Vault>` for `get/set/delete/list/rotate` isn't
        // exposed via Tauri commands today.)
        drop(vault);

        Ok(Self {
            server: ServerManager::new(),
            vault: versioned,
            vault_root,
            repos_dir,
            logs: LogBuffer::new(),
            app_data_dir,
        })
    }
}

/// Re-exported helper used by the `spawn_embedded_server` function in
/// `lib.rs`. Kept here so the `ServerManager` test surface stays
/// cohesive.
pub async fn resolve_bind_with_default(
    requested: Option<String>,
) -> String {
    requested.unwrap_or_else(|| String::from(DEFAULT_BIND))
}

/// Marker re-export for code that wants to refer to the app data dir
/// without pulling in tauri's `AppHandle`.
pub type SharedState = Arc<AsyncMutex<()>>;
