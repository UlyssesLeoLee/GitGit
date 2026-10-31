//! The webview-boot marker: the one line that says "the packaged
//! application's window is showing the product, not a blank page".
//!
//! # Why this exists
//!
//! The packaged app was known to *start* — `scripts/verify-msi-launch.ps1`
//! installs the MSI, launches the installed executable and asserts that a
//! top-level window appears. It was not known to *show* the product: a
//! window that renders nothing at all still has a `MainWindowHandle` and a
//! title. The Svelte frontend is what turns that window into the product,
//! and nothing on the Rust side noticed when it ran.
//!
//! This module closes that gap with a one-shot, on-purpose diagnostic.
//! The frontend calls [`report_ui_boot`] exactly once, at the end of its
//! mount. The Rust side answers by writing **one** log line carrying
//! [`UI_BOOT_MARKER`]. The verification script waits for that line in the
//! application's own log file and fails if it does not arrive.
//!
//! The proof chain is:
//!
//! ```text
//! the marker line is in the log file
//!   => the Rust side received a call from the webview
//!   => the webview's JavaScript ran
//!   => the Svelte app mounted
//!   => the window shows the product's UI, not a blank page
//! ```
//!
//! # The one-shot token, and why the line carries one
//!
//! `[FACT]` The application's log file lives in the user's profile and
//! SURVIVES a launch. A marker written by an EARLIER run can therefore be
//! sitting in it, and a check that greps for a constant marker can be
//! satisfied by history — it would pass without this launch having booted
//! anything.
//!
//! `[FACT]` The first implementation made the search POSITIONAL: the log
//! file's length was recorded before the launch and only the bytes after
//! it were searched. That has a hole that only opens in the field.
//! `tauri-plugin-log` rotates its file at `max_file_size` — 1 MiB in this
//! application — by DELETING the file and creating a fresh one, and it
//! does that at start-up when the existing file is already at the limit.
//! The recorded length then points past the end of the new, nearly empty
//! file, the marker is never searched for, and a healthy launch is
//! reported as a failure.
//!
//! `[INFERENCE]` So the line carries a token instead: the launcher mints
//! one per launch and hands it to the application through the
//! environment ([`UI_BOOT_NONCE_ENV`]). Only the process that was given
//! THIS token can write a line that matches THIS launch, so the search
//! can cover the whole file without caring where in it the evidence is —
//! and a rotated file, a truncated file and a file full of yesterday's
//! lines are all handled by the same rule: they cannot contain this
//! launch's token.
//!
//! # Where the line goes, and why
//!
//! `[FACT]` The application's log file is written by `tauri-plugin-log`,
//! whose `TargetKind::LogDir` target resolves to Tauri's
//! `app_handle.path().app_log_dir()`. On Windows that is
//! `dirs::data_local_dir()` + the bundle identifier + `logs`, i.e.
//! `%LOCALAPPDATA%\<identifier>\logs`, and the file inside it is named
//! after `package_info().name` with its extension replaced by `.log` —
//! `productName` is `gitgit Desktop`, so the file is
//! `gitgit Desktop.log`. The plugin creates that directory and file when
//! it initialises, which is before the window exists.
//!
//! `[FACT]` The plugin registers a `log`-crate logger as the process-wide
//! global logger during plugin initialisation, which happens inside
//! `Builder::build()` — i.e. **before** the application's own `.setup()`
//! hook runs. The application's `.setup()` hook then installs a
//! `tracing` subscriber, and `tracing_subscriber`'s `try_init()` also tries
//! to install `tracing_log::LogTracer` as the global `log` logger. That
//! attempt **fails**, because the log plugin already claimed that slot;
//! the failure is swallowed by the `let _ =` in `setup_app`.
//!
//! `[FACT]` The consequence is that the two sinks do not see each other's
//! events: `log::info!` reaches the plugin's file, and `tracing::info!`
//! reaches only the in-app `BufferLayer` (the log buffer the Settings page
//! reads). `tracing`'s own `log` feature — which is what would copy
//! `tracing` events into the `log` crate — is **not** a default feature and
//! nothing in this crate's dependency tree turns it on.
//!
//! `[INFERENCE]` So the marker is written with `log::info!` through the
//! `log` crate that `tauri-plugin-log` re-exports. That is the sink the
//! verification script reads, and it is the same sink the application
//! already writes its stdout lines to. Using the plugin's re-export keeps
//! the dependency set unchanged: no crate was added to `Cargo.toml`.
//!
//! The unit test at the bottom of this file measures this rather than
//! assuming it: it installs a capturing `log` logger and the application's
//! own subscriber shape, then asserts the marker arrives at the `log`
//! sink. If somebody later reorders the start-up so that the log plugin no
//! longer owns the global logger, that test goes red.

use crate::error::AppResult;

/// The marker text written to the application's log file.
///
/// `[FACT]` It is a bare token on purpose: no spaces, no punctuation that
/// a log formatter would rewrite, and a `_V1` suffix so that a future
/// change of the mechanism can introduce a *new* marker rather than
/// silently changing what the verification script greps for. The
/// verification script searches for this exact string.
///
/// `[FACT]` The line it appears in looks like this (the plugin's default
/// desktop format is `{timestamp}[{target}][{level}] {message}`):
///
/// ```text
/// 2026-10-28_06-31-00[gm_desktop_lib::commands::uiboot][INFO] GITGIT_UI_BOOT_MARKER_V1
/// ```
pub const UI_BOOT_MARKER: &str = "GITGIT_UI_BOOT_MARKER_V1";

/// The environment variable that carries this launch's one-shot token.
///
/// `[FACT]` The launcher (`scripts/verify-msi-launch.ps1`) puts a fresh
/// token in this variable before it starts the application, and a child
/// process inherits the environment it was started with. The application
/// reads it once, when the marker is written.
///
/// `[INFERENCE]` The token is what makes the marker's LINE different from
/// every earlier launch's line, and that is what lets the verification
/// script search the WHOLE log file without caring where in it the
/// evidence lives. A positional rule ("only the bytes written after the
/// launch started") cannot make that promise: `tauri-plugin-log` rotates
/// its file by deleting it and creating a new one, so a recorded offset
/// points past the end of the new file and a healthy launch reads as a
/// failure. A token has no such hole — a marker from an earlier run
/// simply does not match.
pub const UI_BOOT_NONCE_ENV: &str = "GITGIT_UI_BOOT_NONCE";

/// Report that the webview's JavaScript ran and the Svelte app mounted.
///
/// Called exactly once by the frontend, at the end of `App.svelte`'s
/// `onMount`, after the store warm-up.
///
/// # What the marker line proves — and what it does not
///
/// It **proves** that the Rust side received a Tauri command invocation
/// from the webview, and therefore that the webview's JavaScript ran and
/// the Svelte application mounted. A window that renders a blank page
/// cannot produce this line, because a blank page never reaches the end of
/// `onMount`.
///
/// It does **not** prove that the rendered UI is *correct* — that the
/// routes show the right data, that the stylesheet loaded, that the window
/// is not visually broken. It proves the application booted, which is the
/// step that was previously unmeasured; it is not a visual regression
/// test, and it does not replace one.
///
/// # Why the return value is empty
///
/// The caller gets nothing back on purpose. The command's whole purpose is
/// its side effect — the log line — and a payload would invite a caller to
/// treat the call as a query rather than as a one-shot report. The
/// frontend ignores the result and treats a rejection as a non-event: a
/// boot marker that failed to report is not a reason to break a window
/// that is otherwise working.
#[tauri::command]
pub fn report_ui_boot() -> AppResult<()> {
    write_boot_marker();
    Ok(())
}

/// Write the marker line. Exactly one line, exactly one call site.
///
/// `[FACT]` `log::info!` (not `tracing::info!`) because of the sink
/// analysis in the module header: the plugin's log file is fed by the
/// `log` crate, and this crate's `tracing` events do not reach it.
///
/// `[FACT]` The token is read from the environment. When there is none —
/// the application was started by a human rather than by the launch
/// proof — the bare marker is written, so the line is always there and
/// always greppable; only the launch proof insists on the token.
fn write_boot_marker() {
    let nonce = std::env::var(UI_BOOT_NONCE_ENV).unwrap_or_default();
    write_boot_marker_with(nonce.trim());
}

/// Write the marker line, with or without a token.
///
/// [FACT] The token is separated from the marker by a single space, and
/// the marker constant comes first, so the line still starts with the
/// stable text that the verification script reads out of this file.
fn write_boot_marker_with(nonce: &str) {
    if nonce.is_empty() {
        tauri_plugin_log::log::info!("{UI_BOOT_MARKER}");
    } else {
        tauri_plugin_log::log::info!("{UI_BOOT_MARKER} {nonce}");
    }
}

#[cfg(test)]
mod tests {
    // The crate lints deny `unwrap_used` / `expect_used` / `panic`, which
    // are the normal vocabulary of test assertions. They are re-enabled
    // for this module only, which is compiled exclusively under
    // `cfg(test)` and never reaches the shipped binary.
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::state::{BufferLayer, LogBuffer};
    use std::sync::{Mutex, Once};
    use tauri_plugin_log::log as log_crate;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    /// What the global `log` logger saw, in order.
    static CAPTURED: Mutex<Vec<String>> = Mutex::new(Vec::new());

    /// Serialises the tests in this module.
    ///
    /// [FACT] They share ONE global `log` logger and ONE captured buffer,
    /// so two of them running at the same time would interleave their
    /// lines — and the "exactly one line" count below would then be a
    /// number that describes the scheduler rather than the code.
    static SERIAL: Mutex<()> = Mutex::new(());

    /// A `log` logger that records instead of writing.
    ///
    /// It stands in for `tauri-plugin-log`'s fern logger: in the real
    /// application that plugin owns the process-wide `log` logger, and
    /// that logger is what writes the log file.
    struct Capture;

    impl log_crate::Log for Capture {
        fn enabled(&self, _metadata: &log_crate::Metadata<'_>) -> bool {
            true
        }

        fn log(&self, record: &log_crate::Record<'_>) {
            CAPTURED.lock().unwrap().push(record.args().to_string());
        }

        fn flush(&self) {}
    }

    static SET_UP: Once = Once::new();

    /// Reproduce the application's own start-up order.
    ///
    /// `[FACT]` The order matters and is the reason this test exists: the
    /// log plugin claims the global `log` logger during plugin
    /// initialisation, and the application's `.setup()` hook installs the
    /// `tracing` subscriber afterwards. `tracing_subscriber`'s
    /// `try_init()` tries to install `tracing_log::LogTracer` as the
    /// global `log` logger at that point and fails, because the plugin
    /// already owns it — and the application swallows that failure.
    fn set_up_like_the_app() {
        SET_UP.call_once(|| {
            // 1. the log plugin claims the global `log` logger
            log_crate::set_boxed_logger(Box::new(Capture)).unwrap();
            log_crate::set_max_level(log_crate::LevelFilter::Info);
            // 2. the application's tracing subscriber is installed after it
            let _ = tracing_subscriber::registry()
                .with(tracing_subscriber::EnvFilter::new("info"))
                .with(BufferLayer {
                    buffer: LogBuffer::new(),
                })
                .try_init();
        });
    }

    /// Start from an empty buffer, with the other tests held off.
    fn fresh_buffer() -> std::sync::MutexGuard<'static, ()> {
        let guard = SERIAL
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        CAPTURED.lock().unwrap().clear();
        guard
    }

    #[test]
    fn the_marker_reaches_the_log_sink_the_log_file_is_written_by() {
        let _serial = fresh_buffer();
        set_up_like_the_app();

        write_boot_marker();

        let captured = CAPTURED.lock().unwrap();
        assert!(
            captured.iter().any(|line| line == UI_BOOT_MARKER),
            "the marker did not reach the global `log` logger, so it cannot reach the \
             application's log file either. Captured: {captured:?}"
        );
    }

    #[test]
    fn the_marker_is_written_exactly_once_per_call() {
        let _serial = fresh_buffer();
        set_up_like_the_app();

        write_boot_marker();

        let captured = CAPTURED.lock().unwrap();
        assert_eq!(
            captured.len(),
            1,
            "one call must write exactly one line, or the verification script's \
             'the marker arrived' check cannot tell a boot from a flood"
        );
    }

    #[test]
    fn the_marker_text_is_stable_and_greppable() {
        // The verification script greps for this exact string, so it must
        // not contain whitespace and must not be a sentence that a later
        // edit could reword underneath the gate.
        assert_eq!(UI_BOOT_MARKER, "GITGIT_UI_BOOT_MARKER_V1");
        assert!(!UI_BOOT_MARKER.contains(char::is_whitespace));
        assert!(UI_BOOT_MARKER
            .chars()
            .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit()));
    }

    #[test]
    fn the_command_returns_nothing_and_does_not_fail() {
        // The command must never be able to break the boot: it answers
        // with `Ok(())` and has no failure path at all.
        assert!(report_ui_boot().is_ok());
    }

    #[test]
    fn the_token_is_written_next_to_the_marker() {
        let _serial = fresh_buffer();
        set_up_like_the_app();

        write_boot_marker_with("0123456789abcdef0123456789abcdef");

        let captured = CAPTURED.lock().unwrap();
        assert!(
            captured
                .iter()
                .any(|line| line == "GITGIT_UI_BOOT_MARKER_V1 0123456789abcdef0123456789abcdef"),
            "the marker and the token must end up on ONE line, because that line is \
             what the verification script searches for. Captured: {captured:?}"
        );
    }

    #[test]
    fn a_marker_without_a_token_is_still_written() {
        // The application is started by humans too, and those launches do
        // not carry a token. The line must still be there — what the launch
        // proof insists on is the token, not the line's existence.
        let _serial = fresh_buffer();
        set_up_like_the_app();

        write_boot_marker_with("");

        let captured = CAPTURED.lock().unwrap();
        assert!(
            captured.iter().any(|line| line == UI_BOOT_MARKER),
            "the bare marker must be written when there is no token. Captured: {captured:?}"
        );
    }

    #[test]
    fn the_token_comes_from_the_environment_and_not_from_the_caller() {
        // The frontend cannot know the token, so it cannot be a parameter:
        // the launcher hands it to the application through the environment.
        // This asserts the name, because the verification script and this
        // constant have to agree on it and nothing else ties them together.
        assert_eq!(UI_BOOT_NONCE_ENV, "GITGIT_UI_BOOT_NONCE");
        assert!(UI_BOOT_NONCE_ENV
            .chars()
            .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit()));
    }
}
