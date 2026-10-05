//! Typed error surface for the Tauri command handlers exposed to the
//! Svelte frontend.
//!
//! Every `#[tauri::command]` returns `Result<T, AppError>`; the
//! generated TypeScript bindings on the Svelte side see `AppError` as a
//! structured value with `kind`, `message`, and `source` fields. The
//! Svelte error boundary then decides whether to surface the raw
//! message or replace it with a localized, user-friendly copy.
//!
//! Per the brief: "错误处理:每个 Tauri command 有 typed error,UI 层有
//! user-friendly 错误". This file is the typed-error half; the Svelte
//! side carries the i18n messages.

use serde::Serialize;
use thiserror::Error;

/// Error kinds returned to the Svelte frontend. New variants go here when
/// a new command family needs to surface a new failure mode.
#[derive(Debug, Error)]
pub enum AppError {
    /// The server is already running and `start_server` was called again.
    #[error("server is already running (pid={0})")]
    ServerAlreadyRunning(u32),

    /// The server is not running and a stop/status call was issued.
    #[error("server is not running")]
    ServerNotRunning,

    /// Failed to bind the listening socket for the embedded axum server.
    #[error("failed to bind server socket: {0}")]
    Bind(String),

    /// Failed to acquire the server-manager mutex (poisoned).
    #[error("server manager mutex poisoned")]
    ServerManagerPoisoned,

    /// Path / repository name validation failed.
    #[error("invalid repository name: {0}")]
    InvalidRepoName(String),

    /// A call into the gitgit library returned an error.
    #[error("gitgit library error: {0}")]
    Gitgit(String),

    /// A git subprocess call (git-log, git-show-ref, ...) failed.
    #[error("git subprocess failed: {0}")]
    Git(String),

    /// I/O on the host filesystem failed.
    #[error("io error: {0}")]
    Io(String),

    /// The credential vault rejected an operation.
    #[error("vault error: {0}")]
    Vault(String),

    /// A Tauri / OS-level bridge call (clipboard, shell, dialog) failed.
    #[error("bridge error: {0}")]
    Bridge(String),

    /// The requested provider's protocol has no streaming
    /// implementation. Carries the provider key so the UI can name it in
    /// a localized sentence. This is a refusal raised *before* any
    /// network round trip, not a failed call.
    #[error("{0} cannot stream: its protocol has no streaming implementation")]
    AiReviewUnsupported(String),

    /// `GITGIT_AI_API_KEY` is unset in the process that launched the app.
    ///
    /// The key is never accepted from the UI, so this is the only way to
    /// supply one. The message names the variable and nothing else — it
    /// must never carry a key value, including a wrong or partial one.
    #[error("no api key in the environment: set GITGIT_AI_API_KEY before launching the app")]
    AiReviewNoKey,

    /// The review request was rejected by input validation (empty diff,
    /// oversized diff, blank provider key).
    #[error("invalid review request: {0}")]
    AiReviewInvalid(String),

    /// A review stream was started while one was already running.
    #[error("a review is already streaming (session={0})")]
    AiReviewAlreadyRunning(String),

    /// `ai_review_cancel` was called with nothing in flight. Reported
    /// rather than silently succeeding so a UI cannot report a cancel
    /// that never stopped anything.
    #[error("no review is currently streaming")]
    AiReviewNotRunning,

    /// Catch-all for cases where the brief context doesn't predict a
    /// specific kind. Always carries the underlying source via
    /// `source` so it remains debuggable.
    #[error("internal error: {0}")]
    Internal(String),
}

impl AppError {
    /// Stable string identifier — used by the Svelte side to pick a
    /// localized message via the `i18n` catalog.
    pub fn kind(&self) -> &'static str {
        match self {
            AppError::ServerAlreadyRunning(_) => "ServerAlreadyRunning",
            AppError::ServerNotRunning => "ServerNotRunning",
            AppError::Bind(_) => "Bind",
            AppError::ServerManagerPoisoned => "ServerManagerPoisoned",
            AppError::InvalidRepoName(_) => "InvalidRepoName",
            AppError::Gitgit(_) => "Gitgit",
            AppError::Git(_) => "Git",
            AppError::Io(_) => "Io",
            AppError::Vault(_) => "Vault",
            AppError::Bridge(_) => "Bridge",
            AppError::AiReviewUnsupported(_) => "AiReviewUnsupported",
            AppError::AiReviewNoKey => "AiReviewNoKey",
            AppError::AiReviewInvalid(_) => "AiReviewInvalid",
            AppError::AiReviewAlreadyRunning(_) => "AiReviewAlreadyRunning",
            AppError::AiReviewNotRunning => "AiReviewNotRunning",
            AppError::Internal(_) => "Internal",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut st = s.serialize_struct("AppError", 3)?;
        st.serialize_field("kind", self.kind())?;
        st.serialize_field("message", &self.to_string())?;
        st.serialize_field("source", &format!("{:?}", self.kind()))?;
        st.end()
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}

impl From<gitgit::error::GitGitError> for AppError {
    fn from(e: gitgit::error::GitGitError) -> Self {
        AppError::Gitgit(format!("{e}"))
    }
}

impl From<gitgit::server::vault::VaultError> for AppError {
    fn from(e: gitgit::server::vault::VaultError) -> Self {
        AppError::Vault(format!("{e}"))
    }
}

/// Convenience alias.
pub type AppResult<T> = Result<T, AppError>;
