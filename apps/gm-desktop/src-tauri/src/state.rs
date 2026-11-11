//! In-process application state for the Tauri runtime.
//!
//! No part of this file touches gitgit's `vault / auth / http` business
//! logic. Construction uses only `pub` types already exposed by the
//! gitgit library target.
//!
//! It holds three distinct concerns:
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

use std::collections::VecDeque;
use std::future::Future;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use gitgit::server::vault::Vault;
use gitgit::server::vault_versioned::VersionedVault;
use serde::Serialize;
use tokio::task::JoinHandle;
use tracing_subscriber::Layer;

use crate::error::{AppError, AppResult};

/// Maximum log lines kept in memory for the in-process log viewer.
pub const LOG_BUFFER_CAPACITY: usize = 500;
/// Default port for the embedded server. Matches ADR-0020 §2.2.
pub const DEFAULT_BIND: &str = "127.0.0.1:38080";
/// App-data-relative vault root when no explicit override is given.
pub const DEFAULT_VAULT_DIR: &str = "vault";
/// Largest diff accepted for a single AI review, in bytes.
///
/// A bound, not a suggestion: the diff is copied into a prompt, sent to a
/// third-party endpoint, and accumulated again in the webview. An
/// unbounded textarea is a cheap way to spend an operator's tokens.
pub const REVIEW_MAX_DIFF_BYTES: usize = 256 * 1024;

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

impl ServerStatus {
    /// The one "not running" snapshot.
    ///
    /// `[FACT]` Three call sites used to spell this out: the poisoned-lock
    /// branch of `status`, the empty-slot branch of `status`, and the same
    /// empty-slot branch again inside `status_from_guard`. They are
    /// identical, and the `pid` value is the load-bearing field —
    /// `Home.svelte` renders `pid || '—'`, so `0` is the only value that
    /// makes the placeholder reachable. A future edit to one copy and not
    /// the others would be invisible until a user saw a PID next to
    /// "Stopped", which is the exact defect this was refactored away from.
    ///
    /// `[FACT]` On why zero and not this process's id: the embedded server
    /// shares the desktop process, so reporting our own pid is factually
    /// true and completely useless — the dashboard showed a live-looking
    /// PID next to "Stopped".
    ///
    /// Found by running the app. `routes-home.test.ts` asserts the em dash
    /// and passes — against a `pid: 0` fixture the mock supplied. Nothing
    /// ever asked the real command for a stopped snapshot, so both sides of
    /// that contract were tested and the contract itself was not.
    /// `status_reports_no_pid_when_stopped` now asks the manager directly.
    fn stopped() -> Self {
        ServerStatus {
            handle: String::from("embedded"),
            bind: String::new(),
            pid: 0,
            uptime_secs: None,
            running: false,
        }
    }
}

/// Internal record kept while a server is running.
struct Running {
    bind: String,
    started_at: Instant,
    /// The tokio task driving `axum::serve`. Held so `stop` can `abort()`
    /// it — dropping this handle would only detach the task, leaving the
    /// port bound.
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
                // `[FACT]` A poisoned mutex means some other call panicked
                // while holding it. The slot's contents are unknown and
                // cannot be read, so "not running" is the honest report:
                // claiming a running server with no `bind` would be worse.
                return ServerStatus::stopped();
            }
        };
        // `[FACT]` Single construction point. This match used to be
        // duplicated verbatim in `status_from_guard`, and PR #23 had to
        // patch the same `pid` line in two places — which is exactly the
        // shape that lets one copy drift and the other keep reporting the
        // old, wrong value. The two callers need the same snapshot; they
        // differ only in where the `MutexGuard` came from.
        self.status_from_guard(&guard)
    }

    /// Start the embedded server. Returns a [`ServerStatus`] snapshot on
    /// success, `ServerAlreadyRunning` if already up.
    pub async fn start_with<F, Fut>(&self, bind: String, spawn: F) -> AppResult<ServerStatus>
    where
        F: FnOnce(String) -> Fut,
        Fut: Future<Output = AppResult<JoinHandle<()>>>,
    {
        // Phase 1: hold the lock long enough to reject a double start.
        // Drop the guard before awaiting the spawn closure so the lock
        // isn't held across .await (MutexGuard is !Send). The result is
        // `Result<(), AppError>` and is consumed by `?` — binding it to a
        // name would only discard the Ok(()) immediately.
        {
            let guard = self
                .inner
                .lock()
                .map_err(|_| AppError::ServerManagerPoisoned)?;
            if guard.is_some() {
                return Err(AppError::ServerAlreadyRunning(std::process::id()));
            }
        }
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
    ///
    /// The handle is aborted AND awaited. Aborting alone is not enough to
    /// report "stopped": cancellation is delivered at the task's next await
    /// point, so returning immediately would let a caller observe
    /// `running: false` while `axum::serve` still holds the port.
    ///
    /// The previous implementation simply dropped the `Option<Running>`,
    /// which drops the `JoinHandle` — and dropping a tokio `JoinHandle`
    /// *detaches* the task rather than cancelling it. The server kept
    /// running and kept the port bound after `stop_server` had reported it
    /// down. `abort()` is what actually stops it.
    pub async fn stop(&self) -> AppResult<ServerStatus> {
        let taken = {
            let mut guard = self
                .inner
                .lock()
                .map_err(|_| AppError::ServerManagerPoisoned)?;
            guard.take()
        };
        let Some(running) = taken else {
            return Err(AppError::ServerNotRunning);
        };

        let prev = ServerStatus {
            handle: String::from("embedded"),
            bind: running.bind.clone(),
            pid: std::process::id(),
            uptime_secs: Some(running.started_at.elapsed().as_secs()),
            running: true,
        };

        running.join.abort();
        // Wait for the cancellation to land so the port is released before
        // this call reports the server as down. Err(Cancelled) is the
        // expected result and carries no information we need.
        let _ = running.join.await;

        Ok(prev)
    }

    /// The one place a [`ServerStatus`] is built from the manager's slot.
    ///
    /// `[FACT]` Both `status()` and `start_with` route through here. The
    /// alternative — a `ServerStatus` literal at each call site — is what
    /// let PR #23's `pid` fix be applied to one copy while the other kept
    /// the pre-fix value. A duplicated constructor is a duplicated bug.
    fn status_from_guard(&self, guard: &std::sync::MutexGuard<Option<Running>>) -> ServerStatus {
        match &**guard {
            Some(r) => ServerStatus {
                handle: String::from("embedded"),
                bind: r.bind.clone(),
                pid: std::process::id(),
                uptime_secs: Some(r.started_at.elapsed().as_secs()),
                running: true,
            },
            None => {
                // `[FACT]` Reachable: `status()` funnels here with an empty
                // slot, and `server_status` is what the UI calls. It was
                // briefly written off as dead code because `start_with`
                // only calls this helper right after assigning `Some(..)`
                // — but that says nothing about the other caller.
                ServerStatus::stopped()
            }
        }
    }
}

impl Default for ServerManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Owner of the single in-flight AI review stream.
///
/// The shape deliberately mirrors [`ServerManager`] — one slot, a mutex
/// held only across the bookkeeping and never across an `.await` — for
/// the same reason: two concurrent reviews would interleave their token
/// events into one UI text box, and there is no way to tell afterwards
/// which model wrote which sentence.
pub struct ReviewManager {
    inner: Mutex<Option<RunningReview>>,
}

/// Internal record of the stream that is currently being consumed.
struct RunningReview {
    session_id: String,
    /// Task draining the provider's `StreamReceiver` and emitting
    /// events. Held so `cancel` can `abort()` it — dropping the handle
    /// would only *detach* the task, which is precisely the defect
    /// `ServerManager::stop` documents for the embedded server.
    join: JoinHandle<()>,
}

impl ReviewManager {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    /// Session id of the running review, if any.
    ///
    /// Doubles as the "is anything running" test (`is_none()` means the
    /// slot is free) so there is exactly one accessor to keep correct.
    /// Its value also names the conflict in a "already streaming" error.
    pub fn running_session(&self) -> Option<String> {
        self.inner
            .lock()
            .ok()
            .and_then(|g| g.as_ref().map(|r| r.session_id.clone()))
    }

    /// Install `join` as *the* running review.
    ///
    /// On conflict the handle is handed back to the caller rather than
    /// dropped: the caller must abort it, because a dropped
    /// `JoinHandle` detaches the task and it would keep streaming into
    /// a session the UI has already been told is not running.
    pub fn install(
        &self,
        session_id: String,
        join: JoinHandle<()>,
    ) -> Result<(), (JoinHandle<()>, String)> {
        let mut guard = match self.inner.lock() {
            Ok(g) => g,
            Err(_) => return Err((join, String::from("<poisoned>"))),
        };
        // Inspect without taking: removing the incumbent here would
        // orphan a stream that is still running, leaving two live
        // streams and no way to cancel either.
        if let Some(existing) = guard.as_ref() {
            return Err((join, existing.session_id.clone()));
        }
        *guard = Some(RunningReview { session_id, join });
        Ok(())
    }

    /// Release the slot without touching the task, for `session_id` only.
    ///
    /// Called by the stream task itself on every exit path, so a
    /// finished stream does not make the *next* start look like a
    /// double start. The session-id comparison stops a late-exiting
    /// task from clearing a slot that a newer stream already claimed.
    pub fn release(&self, session_id: &str) {
        if let Ok(mut guard) = self.inner.lock() {
            if guard.as_ref().is_some_and(|r| r.session_id == session_id) {
                *guard = None;
            }
        }
    }

    /// Cancel the running review. Returns the session id that was
    /// stopped.
    ///
    /// The handle is aborted AND awaited, for the reason spelled out in
    /// [`ServerManager::stop`]: returning before the cancellation lands
    /// would let the UI report "stopped" while the task still holds the
    /// provider connection. Dropping the `StreamReceiver` is what
    /// actually releases the HTTP body — see
    /// [`gitgit::ai::provider::AiProvider::send_stream`].
    pub async fn cancel(&self) -> AppResult<String> {
        let taken = {
            let mut guard = self
                .inner
                .lock()
                .map_err(|_| AppError::ServerManagerPoisoned)?;
            guard.take()
        };
        let Some(running) = taken else {
            return Err(AppError::AiReviewNotRunning);
        };
        running.join.abort();
        let _ = running.join.await;
        Ok(running.session_id)
    }
}

impl Default for ReviewManager {
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
            self.value
                .push_str(&format!(" {}={:?}", field.name(), value));
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
    /// In-flight AI review stream, so `ai_review_cancel` has something
    /// to abort. Shared as an `Arc` because the stream task clears the
    /// slot on exit, which requires a handle that outlives the command
    /// that spawned it.
    pub reviews: Arc<ReviewManager>,
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
    pub fn new(vault_root: PathBuf, repos_dir: PathBuf, app_data_dir: PathBuf) -> AppResult<Self> {
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
            reviews: Arc::new(ReviewManager::new()),
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
pub async fn resolve_bind_with_default(requested: Option<String>) -> String {
    requested.unwrap_or_else(|| String::from(DEFAULT_BIND))
}

#[cfg(test)]
mod tests {
    // The crate lints deny `unwrap_used` / `expect_used` / `panic`, which
    // are the normal vocabulary of test assertions. They are re-enabled
    // for this module only, which is compiled exclusively under
    // `cfg(test)` and never reaches the shipped binary.
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex as StdMutex;

    /// The bind string the test helpers ask for. Port `0` makes the OS
    /// pick a free port, so the address actually bound is only known
    /// after the listener exists — which is exactly why the helpers below
    /// return it separately.
    const REQUESTED_BIND: &str = "127.0.0.1:0";

    /// Flips a flag when it is dropped. A value constructed *inside* a
    /// spawned task is therefore a witness for "this task was actually
    /// torn down" as opposed to "its `JoinHandle` was merely discarded".
    struct DropWitness(Arc<AtomicBool>);

    impl Drop for DropWitness {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    /// Start a server on `manager` whose task owns a real `TcpListener`
    /// on an ephemeral port, exactly like `spawn_embedded_server` does in
    /// production. Returns the address that was bound.
    async fn start_real_listener(manager: &ServerManager) -> std::net::SocketAddr {
        let addr_slot = Arc::new(StdMutex::new(None));

        let status = manager
            .start_with(String::from(REQUESTED_BIND), {
                let addr_slot = Arc::clone(&addr_slot);
                move |_bind| async move {
                    let listener = tokio::net::TcpListener::bind(REQUESTED_BIND)
                        .await
                        .expect("ephemeral port is bindable");
                    let addr = listener.local_addr().expect("listener reports local_addr");
                    *addr_slot.lock().expect("addr slot is not poisoned") = Some(addr);
                    let handle = tokio::spawn(async move {
                        if let Err(e) = axum::serve(listener, axum::Router::new()).await {
                            tracing::error!(error = %e, "test server exited");
                        }
                    });
                    Ok::<JoinHandle<()>, AppError>(handle)
                }
            })
            .await
            .expect("start_with succeeds");

        assert!(status.running, "a freshly started server reports running");
        assert_eq!(status.handle, "embedded");
        let addr = *addr_slot
            .lock()
            .expect("addr slot is not poisoned")
            .as_ref()
            .expect("the spawned task recorded its bound address");
        addr
    }

    /// Regression test for the detached-task defect.
    ///
    /// The previous `stop()` only did `*guard = None`, which drops the
    /// `JoinHandle`. Dropping a tokio `JoinHandle` *detaches* the task
    /// instead of cancelling it, so `axum::serve` kept running, the port
    /// stayed bound, and `stop_server` had already reported
    /// `running: false`. Re-binding the port is the user-visible symptom,
    /// so that is what this asserts: the port must be free the instant
    /// `stop()` resolves, not "eventually, if the process happens to
    /// notice".
    #[tokio::test]
    async fn stop_releases_the_listening_port_before_returning() {
        let manager = ServerManager::new();
        let addr = start_real_listener(&manager).await;

        // Precondition: while "running", the port really is occupied. If
        // this ever fails the test would pass vacuously below.
        assert!(
            std::net::TcpListener::bind(addr).is_err(),
            "precondition: {addr} must be occupied while the server is running"
        );

        let previous = manager.stop().await.expect("stop returns the prior status");
        assert!(previous.running, "stop reports the server it just stopped");
        // `ServerStatus.bind` echoes the *requested* bind string, not the
        // address the kernel resolved. With the production default
        // (`127.0.0.1:38080`) the two are the same string; they diverge
        // only when the caller asks for port 0, which is why this test
        // tracks the resolved address separately.
        assert_eq!(previous.bind, REQUESTED_BIND);
        assert!(!manager.status().running, "status flips to down after stop");

        // The regression: this bind fails under the old drop-only
        // implementation with "address already in use".
        std::net::TcpListener::bind(addr).expect("the port must be released once stop() resolves");

        assert!(
            manager.stop().await.is_err(),
            "a second stop reports that nothing is running"
        );
    }

    /// Companion to the port test: proves the *task* was cancelled rather
    /// than the listener happening to be released by some other path.
    #[tokio::test]
    async fn stop_cancels_the_task_instead_of_detaching_it() {
        let manager = ServerManager::new();
        let dropped = Arc::new(AtomicBool::new(false));

        manager
            .start_with(String::from(REQUESTED_BIND), {
                let dropped = Arc::clone(&dropped);
                move |_bind| async move {
                    // The witness is a *local of this closure body*, not
                    // of the spawned task's body. `tokio::spawn` builds
                    // the future eagerly, so the witness is moved into
                    // it at spawn time and is dropped when the task's
                    // future is dropped — whether or not the task ever
                    // got polled. Under `#[tokio::test]`'s
                    // current-thread runtime it never does, so a
                    // witness created inside the task body would never
                    // exist and this test would prove nothing.
                    let witness = DropWitness(Arc::clone(&dropped));
                    let handle = tokio::spawn(async move {
                        let _witness = witness;
                        std::future::pending::<()>().await;
                    });
                    Ok::<JoinHandle<()>, AppError>(handle)
                }
            })
            .await
            .expect("start_with succeeds");

        assert!(
            !dropped.load(Ordering::SeqCst),
            "the task is still alive before stop"
        );

        manager.stop().await.expect("stop returns the prior status");

        assert!(
            dropped.load(Ordering::SeqCst),
            "stop() must cancel the task; dropping a JoinHandle only detaches it"
        );
    }

    /// A second `start_with` while running is rejected, and the manager
    /// still reports the original bind rather than the rejected one.
    #[tokio::test]
    async fn start_is_rejected_while_a_server_is_already_running() {
        let manager = ServerManager::new();
        let addr = start_real_listener(&manager).await;

        let err = manager
            .start_with(String::from("127.0.0.1:1"), |_bind| async move {
                panic!("the spawn closure must not run for a rejected start");
            })
            .await
            .expect_err("a second start is refused");
        assert!(matches!(err, AppError::ServerAlreadyRunning(_)), "{err:?}");

        let status = manager.status();
        assert!(status.running);
        assert_eq!(
            status.bind, REQUESTED_BIND,
            "bind is unchanged by a refused start"
        );
        assert!(
            std::net::TcpListener::bind(addr).is_err(),
            "the refused start did not disturb the server that was already running"
        );

        manager.stop().await.expect("stop returns the prior status");
    }

    /// `stop` on a manager that never started is an error, and — because
    /// a failed stop must not consume the guard — the *same* manager is
    /// still able to start afterwards.
    #[tokio::test]
    async fn stop_without_start_reports_not_running_and_leaves_the_manager_usable() {
        let manager = ServerManager::new();

        let err = manager.stop().await.expect_err("nothing is running");
        assert!(matches!(err, AppError::ServerNotRunning), "{err:?}");
        assert!(!manager.status().running);

        let addr = start_real_listener(&manager).await;
        assert!(
            manager.status().running,
            "the manager still accepts a start"
        );
        assert_eq!(manager.status().bind, REQUESTED_BIND);
        assert!(
            std::net::TcpListener::bind(addr).is_err(),
            "the reused manager really did take a fresh listening port"
        );

        manager.stop().await.expect("stop returns the prior status");
        assert!(!manager.status().running);
    }

    /// `resolve_bind_with_default` is the single source of the default
    /// bind string; `commands::server` used to carry its own copy of the
    /// literal, which is exactly the kind of drift this pins down.
    #[tokio::test]
    async fn resolve_bind_falls_back_to_the_documented_default() {
        assert_eq!(
            resolve_bind_with_default(None).await,
            DEFAULT_BIND,
            "an absent bind resolves to DEFAULT_BIND"
        );
        assert_eq!(
            resolve_bind_with_default(Some(String::from("127.0.0.1:1234"))).await,
            "127.0.0.1:1234",
            "an explicit bind is passed through untouched"
        );
    }

    /// The contract between this command and `Home.svelte`.
    ///
    /// `[FACT]` The dashboard renders `{$server.pid || '—'}`, and
    /// `tests/unit/routes-home.test.ts` asserts that a stopped server shows
    /// the em dash. That test feeds a `pid: 0` fixture through a mocked
    /// `invoke`, so it never learned what this manager actually returns —
    /// and for a stopped server that was `std::process::id()`, which is
    /// never 0, so the placeholder was unreachable in the shipped app.
    ///
    /// `[FACT]` Found by running the app and looking at it: "Stopped" sat
    /// next to a live-looking PID. Each side of this contract had its own
    /// passing tests; nothing asked both.
    #[tokio::test]
    async fn status_reports_no_pid_when_stopped() {
        let manager = ServerManager::new();
        let status = manager.status();

        assert!(!status.running, "a fresh manager is not running");
        assert_eq!(
            status.pid, 0,
            "a stopped server owns no process, so it reports no pid; \
             anything else makes the dashboard's em dash unreachable"
        );
        assert_eq!(status.bind, "", "and no bind address either");
        assert!(status.uptime_secs.is_none(), "and no uptime");
    }

    /// The other half of the same contract: once something *is* running the
    /// pid has to be real, or the fix above would have traded a misleading
    /// value for a permanently blank one.
    #[tokio::test]
    async fn status_reports_the_real_pid_while_running() {
        let manager = ServerManager::new();
        manager
            .start_with(String::from(REQUESTED_BIND), |bind| async move {
                Ok(tokio::spawn(async move {
                    let _listener =
                        std::net::TcpListener::bind(&bind).expect("bind the requested address");
                    std::future::pending::<()>().await;
                }))
            })
            .await
            .expect("the server starts");

        let status = manager.status();
        assert!(status.running);
        assert_eq!(
            status.pid,
            std::process::id(),
            "the embedded server is this process, so that is its pid"
        );

        manager.stop().await.expect("stop returns the prior status");
    }
}
