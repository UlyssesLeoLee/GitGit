//! Streaming AI review commands — `ai_review_start` / `ai_review_cancel`.
//!
//! V0 task T9: "AI 评审 UI — push 前弹窗 + streaming token 显示", with the
//! acceptance criterion that tokens visibly stream into the GUI. This
//! module is the Rust half of that: it drives the OpenAI-compatible
//! transport in [`gitgit::ai::stream`] and republishes each event on a
//! Tauri event channel the Svelte layer subscribes to.
//!
//! # Four invariants worth stating up front
//!
//! 1. **The key comes from the environment only.** [`read_api_key_env`]
//!    reads `GITGIT_AI_API_KEY` and nothing else. There is no command
//!    parameter, no CLI flag, and no frontend field, because a key in a
//!    command argument is visible in `ps` and in shell history, and a
//!    key in the webview is a key in a file the user can open. Every
//!    error string that leaves this module is additionally scrubbed
//!    with [`gitgit::ai::sanitize::scrub_secret`] on the path where a
//!    provider-authored message could carry it.
//! 2. **Unsupported providers are refused before the network.** The
//!    check is [`gitgit::ai::registry::ProviderSpec::supports_streaming`],
//!    and the refusal is a typed error, not a fallback to a
//!    non-streaming call. Anthropic genuinely cannot stream, and a
//!    review that arrives in one lump with a "stream" spinner on it
//!    reads as a hang.
//! 3. **Cancel means the work stopped.** [`ReviewManager::cancel`]
//!    aborts the consuming task *and awaits it*, which drops the
//!    `StreamReceiver`; the producer's next send then fails and the
//!    in-flight HTTP response body is released. The UI's stop button
//!    therefore ends the request, it does not merely stop rendering it.
//! 4. **The diff is untrusted, the output is untrusted.** The diff is
//!    built by [`gitgit::ai::prompt::review`] as an explicitly untrusted
//!    message and passes through
//!    [`gitgit::ai::sanitize::sanitize_request`] before the call, so
//!    secrets are filtered and the content is tagged as data. Model
//!    output is re-emitted as plain text; nothing here interprets it.
//!
//! # Why the Tauri dependency is behind a trait
//!
//! [`spawn_review`] takes a [`ReviewEmitter`] rather than an
//! `AppHandle`. A command body that requires a live Tauri runtime can
//! only be exercised by a running app, and the behaviour worth testing
//! here — event ordering, cancellation, the key-leak guard — is exactly
//! the behaviour a recording emitter can observe deterministically. The
//! real emitter is [`TauriEmitter`], a three-line adapter.

use std::sync::Arc;

use gitgit::ai::provider::{ChatRequest, StreamEvent, StreamReceiver};
use gitgit::ai::{prompt, registry, sanitize};
use tauri::{AppHandle, Emitter, State};

use crate::error::{AppError, AppResult};
use crate::state::{DesktopState, ReviewManager, REVIEW_MAX_DIFF_BYTES};

/// Provider used when the UI does not name one. OpenAI-compatible, so
/// it streams.
pub const DEFAULT_PROVIDER: &str = "openai";

/// One token delta, emitted as it is decoded.
pub const EVENT_TOKEN: &str = "ai-review://token";
/// The model the provider says it served.
pub const EVENT_MODEL: &str = "ai-review://model";
/// The stream completed normally.
pub const EVENT_DONE: &str = "ai-review://done";
/// The stream failed. Carries provider/proxy error text, never a key.
pub const EVENT_FAILED: &str = "ai-review://failed";
/// A stream was cancelled from the UI.
pub const EVENT_CANCELLED: &str = "ai-review://cancelled";

/// One event on the review channel.
///
/// Serialized with a `type` tag so a single listener can dispatch, and
/// with `camelCase` field names because the consumer is TypeScript.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ReviewEvent {
    /// An incremental chunk of the review.
    Token { session_id: String, delta: String },
    /// The model the provider reported.
    Model { session_id: String, model: String },
    /// Terminal success. `tokens` counts the deltas received, so the UI
    /// can show progress without buffering a second copy of the text.
    Done { session_id: String, tokens: u32 },
    /// Terminal failure. `message` is provider or transport wording.
    Failed { session_id: String, message: String },
    /// Cancelled by the user.
    Cancelled { session_id: String },
}

impl ReviewEvent {
    /// Tauri event name for this variant.
    pub fn name(&self) -> &'static str {
        match self {
            ReviewEvent::Token { .. } => EVENT_TOKEN,
            ReviewEvent::Model { .. } => EVENT_MODEL,
            ReviewEvent::Done { .. } => EVENT_DONE,
            ReviewEvent::Failed { .. } => EVENT_FAILED,
            ReviewEvent::Cancelled { .. } => EVENT_CANCELLED,
        }
    }
}

/// What the UI learns when a review starts.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewStarted {
    /// Identifies this stream. Every event carries it, so a late event
    /// from a cancelled session can be discarded instead of appended
    /// to the next review's text.
    pub session_id: String,
    /// The registry key that was used.
    pub provider: String,
    /// The model that was requested.
    pub model: String,
    /// Always `true` here — a provider that cannot stream is refused
    /// rather than degraded. Exposed so the UI can assert the contract
    /// instead of assuming it.
    pub streaming: bool,
    /// How many untrusted messages the sanitizer rewrote. Surfaced so a
    /// diff carrying a committed secret is visible rather than silent.
    pub redactions: usize,
}

/// Everything `ai_review_start` accepts. A struct rather than four loose
/// parameters so the Tauri surface and the test surface are the same
/// shape.
#[derive(Debug, Clone, Default)]
pub struct ReviewArgs {
    pub diff: String,
    /// Registry key. Blank/absent means [`DEFAULT_PROVIDER`].
    pub provider: Option<String>,
    /// Blank/absent means the preset's default model.
    pub model: Option<String>,
    /// Blank/absent means the preset's own base URL. This is the flag
    /// `--ai-base-url` already provides on the CLI, and it is how a
    /// local runtime (ollama, vLLM, or a test double) is reached. It is
    /// not a secret: only the `Authorization` header is.
    pub base_url: Option<String>,
}

/// Transport for review events. Implemented by [`TauriEmitter`] in
/// production and by a recorder in tests.
pub trait ReviewEmitter: Send + Sync {
    /// Deliver one event. `Err` carries a description of the bridge
    /// failure only.
    fn emit(&self, event: &ReviewEvent) -> Result<(), String>;
}

/// Real emitter: Tauri events to every listening webview.
struct TauriEmitter(AppHandle);

impl ReviewEmitter for TauriEmitter {
    fn emit(&self, event: &ReviewEvent) -> Result<(), String> {
        self.0
            .emit(event.name(), event.clone())
            .map_err(|e| format!("emit {}: {e}", event.name()))
    }
}

/// Read the provider credential from the process environment.
///
/// Returns `None` for unset *and* for blank, so a stray
/// `GITGIT_AI_API_KEY=` does not become a bearer token of `""` that
/// produces a confusing 401 from the provider.
pub fn read_api_key_env() -> Option<String> {
    std::env::var(gitgit::ai::API_KEY_ENV)
        .ok()
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
}

/// Validate the request and return the trimmed diff.
///
/// Runs before the credential lookup, so an empty textarea reports the
/// empty textarea rather than complaining about a key the operator has
/// not been told is needed yet.
fn validate(args: &ReviewArgs) -> AppResult<&str> {
    let diff = args.diff.trim();
    if diff.is_empty() {
        return Err(AppError::AiReviewInvalid(String::from(
            "the diff is empty; paste the change you want reviewed",
        )));
    }
    if diff.len() > REVIEW_MAX_DIFF_BYTES {
        return Err(AppError::AiReviewInvalid(format!(
            "the diff is {} bytes; the limit is {REVIEW_MAX_DIFF_BYTES} bytes",
            diff.len()
        )));
    }
    Ok(diff)
}

/// Non-blank, trimmed override, or `None`.
fn clean(value: &Option<String>) -> Option<&str> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

/// Start a streaming review and publish its events through `emitter`.
///
/// Tauri-independent on purpose — see the module note on
/// [`ReviewEmitter`]. `api_key` is passed in (rather than read here) so
/// the ordering guarantee above is explicit and so tests can drive both
/// the present and absent case without touching process environment.
pub async fn spawn_review(
    manager: Arc<ReviewManager>,
    emitter: Arc<dyn ReviewEmitter>,
    api_key: Option<String>,
    args: ReviewArgs,
) -> AppResult<ReviewStarted> {
    let diff = validate(&args)?;
    let provider_key = clean(&args.provider).unwrap_or(DEFAULT_PROVIDER);
    let registry = registry::ProviderRegistry::builtin();
    // An unknown key fails here, with the known keys listed, and before
    // any credential is read.
    let spec = registry.get(provider_key)?;
    if !spec.supports_streaming() {
        return Err(AppError::AiReviewUnsupported(String::from(provider_key)));
    }
    // Fast-fail a second start before spending a network round trip.
    // `install` re-checks after the call, because two commands can pass
    // this point concurrently; the loser aborts its own task.
    if let Some(running) = manager.running_session() {
        return Err(AppError::AiReviewAlreadyRunning(running));
    }
    let api_key = api_key.ok_or(AppError::AiReviewNoKey)?;

    let spec = match clean(&args.base_url) {
        Some(url) => spec.with_base_url(url),
        None => spec.clone(),
    };
    let model = clean(&args.model)
        .map_or_else(|| spec.default_model().to_string(), str::to_string);

    let mut req = ChatRequest::new(model.clone(), prompt::review(diff));
    // Untrusted tagging + heuristic secret redaction, after the prompt
    // is built — sanitizing an empty request would be a no-op.
    let redactions = sanitize::sanitize_request(&mut req);

    let provider = spec.build(provider_key);
    // A provider that cannot stream refuses here rather than falling
    // back to `send`; the `supports_streaming` check above makes the
    // refusal reachable at all, and this is the backstop.
    let rx = provider.send_stream(&api_key, &req).await?;

    let session_id = new_session_id();
    let join = tokio::spawn(pump(
        Arc::clone(&manager),
        emitter,
        api_key,
        session_id.clone(),
        rx,
    ));
    if let Err((handle, previous)) = manager.install(session_id.clone(), join) {
        // Abort rather than drop: dropping a JoinHandle detaches.
        handle.abort();
        let _ = handle.await;
        return Err(AppError::AiReviewAlreadyRunning(previous));
    }

    Ok(ReviewStarted {
        session_id,
        provider: String::from(provider_key),
        model,
        streaming: true,
        redactions,
    })
}

/// Cancel the running review and announce it. Tauri-independent.
pub async fn cancel_review(
    manager: Arc<ReviewManager>,
    emitter: Arc<dyn ReviewEmitter>,
) -> AppResult<String> {
    let session_id = manager.cancel().await?;
    if let Err(e) = emitter.emit(&ReviewEvent::Cancelled {
        session_id: session_id.clone(),
    }) {
        // The cancel itself succeeded and the caller gets the session
        // id back either way. Failing the command here would tell the
        // UI that stopping did not work, which is the one thing that
        // would be a lie.
        tracing::warn!(error = %e, "could not emit review cancellation");
    }
    Ok(session_id)
}

/// Fresh stream identifier.
fn new_session_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Drain one stream channel into events.
async fn pump(
    manager: Arc<ReviewManager>,
    emitter: Arc<dyn ReviewEmitter>,
    api_key: String,
    session_id: String,
    mut rx: StreamReceiver,
) {
    // Clears the manager slot on every exit path, including the
    // cancellation-by-abort one, where this future is dropped rather
    // than returned from.
    let _slot = SlotGuard {
        manager: Arc::clone(&manager),
        session_id: session_id.clone(),
    };

    let mut tokens: u32 = 0;
    while let Some(item) = rx.recv().await {
        match item {
            Ok(StreamEvent::Token(delta)) => {
                tokens = tokens.saturating_add(1);
                let _ = emitter.emit(&ReviewEvent::Token {
                    session_id: session_id.clone(),
                    delta,
                });
            }
            Ok(StreamEvent::Model(model)) => {
                let _ = emitter.emit(&ReviewEvent::Model {
                    session_id: session_id.clone(),
                    model,
                });
            }
            Ok(StreamEvent::Finished) => {
                let _ = emitter.emit(&ReviewEvent::Done {
                    session_id: session_id.clone(),
                    tokens,
                });
                return;
            }
            Err(e) => {
                // Belt and braces: a provider or proxy that echoed the
                // `Authorization` header back inside an error body
                // would otherwise put the key in the UI and in the log
                // buffer. `send_stream` does not believe this can
                // happen either; this is the second, independent guard.
                let _ = emitter.emit(&ReviewEvent::Failed {
                    session_id: session_id.clone(),
                    message: sanitize::scrub_secret(&e.to_string(), &api_key),
                });
                return;
            }
        }
    }

    // The channel closed with neither `Finished` nor an error. That is
    // a truncated stream, and it is reported as a failure rather than
    // left to look like a finished review.
    let _ = emitter.emit(&ReviewEvent::Failed {
        session_id,
        message: String::from("the review stream ended before the provider finished"),
    });
}

/// Releases the manager slot when the stream task exits.
struct SlotGuard {
    manager: Arc<ReviewManager>,
    session_id: String,
}

impl Drop for SlotGuard {
    fn drop(&mut self) {
        self.manager.release(&self.session_id);
    }
}

/// Start a streaming AI review. Returns as soon as the request is
/// accepted; the text arrives on [`EVENT_TOKEN`] events.
#[tauri::command]
pub async fn ai_review_start(
    app: AppHandle,
    state: State<'_, DesktopState>,
    diff: String,
    provider: Option<String>,
    model: Option<String>,
    base_url: Option<String>,
) -> AppResult<ReviewStarted> {
    spawn_review(
        Arc::clone(&state.reviews),
        Arc::new(TauriEmitter(app)),
        read_api_key_env(),
        ReviewArgs {
            diff,
            provider,
            model,
            base_url,
        },
    )
    .await
}

/// Stop the running review. Returns the session id that was stopped.
#[tauri::command]
pub async fn ai_review_cancel(
    app: AppHandle,
    state: State<'_, DesktopState>,
) -> AppResult<String> {
    cancel_review(Arc::clone(&state.reviews), Arc::new(TauriEmitter(app))).await
}
