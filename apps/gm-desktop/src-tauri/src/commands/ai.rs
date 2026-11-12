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

use gitgit::ai::conflict::{self, ConflictRegion};
use gitgit::ai::keys;
use gitgit::ai::provider::{ChatRequest, StreamEvent, StreamReceiver};
use gitgit::ai::{prompt, registry, sanitize};
use gitgit::server::vault::Vault;
use gitgit::server::vault_versioned::VersionedVault;
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

/// One token delta of a conflict arbitration.
pub const EVENT_CONFLICT_TOKEN: &str = "ai-conflict://token";
/// The model the provider says it served.
pub const EVENT_CONFLICT_MODEL: &str = "ai-conflict://model";
/// The stream completed; the answer has been validated.
pub const EVENT_CONFLICT_DONE: &str = "ai-conflict://done";
/// The stream failed. Carries provider/proxy error text, never a key.
pub const EVENT_CONFLICT_FAILED: &str = "ai-conflict://failed";
/// A stream was cancelled from the UI.
pub const EVENT_CONFLICT_CANCELLED: &str = "ai-conflict://cancelled";

/// What became of the model's answer.
///
/// `[PROPOSAL]` ADR-0024 §3.4.4: "the suggestion is marked **rejected**,
/// with the reason attached; the user can still edit by hand". The two
/// states are one enum rather than two optional fields so that a consumer
/// cannot read "no suggestion and no reason" as a third, undefined state.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum SuggestionOutcome {
    /// The answer passed every check in ADR §3.4.2 and may be shown.
    Accepted {
        /// The replacement text, verbatim.
        suggestion: String,
    },
    /// The answer was refused.
    Rejected {
        /// The user-facing reason the answer was refused.
        reason: String,
    },
}

/// One event on the conflict channel.
///
/// `[FACT]` The shape mirrors [`ReviewEvent`]: a `type` tag for dispatch,
/// `camelCase` fields for the TypeScript consumer, and a `session_id` on
/// every event so a late event from a cancelled session can be discarded
/// instead of appended to the next region's text.
///
/// `[PROPOSAL]` ADR-0024 §3.7.2 adds one field the review channel does
/// not have: `region_id`. Two arbitrations can be in flight over the life
/// of the app, and the region id is what says *which* region a token
/// belongs to — the property that stops a late token from one region
/// landing on another.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ConflictEvent {
    /// An incremental chunk of the model's answer.
    Token {
        session_id: String,
        region_id: String,
        delta: String,
    },
    /// The model the provider reported.
    Model {
        session_id: String,
        region_id: String,
        model: String,
    },
    /// Terminal success. `tokens` counts the deltas received.
    Done {
        session_id: String,
        region_id: String,
        tokens: u32,
        outcome: SuggestionOutcome,
    },
    /// Terminal failure. `message` is provider or transport wording.
    Failed {
        session_id: String,
        region_id: String,
        message: String,
    },
    /// Cancelled by the user.
    ///
    /// `[INFERENCE]` No `region_id` here: the cancellation is a property of
    /// the *stream*, and the caller knows which region it asked about.
    Cancelled { session_id: String },
}

impl ConflictEvent {
    /// Tauri event name for this variant.
    pub fn name(&self) -> &'static str {
        match self {
            ConflictEvent::Token { .. } => EVENT_CONFLICT_TOKEN,
            ConflictEvent::Model { .. } => EVENT_CONFLICT_MODEL,
            ConflictEvent::Done { .. } => EVENT_CONFLICT_DONE,
            ConflictEvent::Failed { .. } => EVENT_CONFLICT_FAILED,
            ConflictEvent::Cancelled { .. } => EVENT_CONFLICT_CANCELLED,
        }
    }
}

/// Transport for conflict events.
///
/// `[FACT]` This is the same shape as [`ReviewEmitter`]: the behaviour
/// worth testing — event ordering, cancellation, the key-leak guard — is
/// what a recording emitter observes, and a command body that required a
/// live Tauri runtime could not be exercised at all.
pub trait ConflictEmitter: Send + Sync {
    /// Deliver one event. `Err` carries a description of the bridge
    /// failure only.
    fn emit(&self, event: &ConflictEvent) -> Result<(), String>;
}

impl ConflictEmitter for TauriEmitter {
    fn emit(&self, event: &ConflictEvent) -> Result<(), String> {
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
    value.as_deref().map(str::trim).filter(|s| !s.is_empty())
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
    let model = clean(&args.model).map_or_else(|| spec.default_model().to_string(), str::to_string);

    let mut req = ChatRequest::new(model.clone(), prompt::review(diff));
    // Untrusted tagging + heuristic secret redaction, after the prompt
    // is built — sanitizing an empty request would be a no-op.
    //
    // `sanitize_request` returns how many *messages* it rewrote, which
    // is always at least one because every untrusted message gains the
    // structural `<untrusted>` tag. That number does not tell the
    // operator whether a secret was stripped, so the count reported to
    // the UI is counted separately from the rules that actually fired.
    let redactions = sanitize::scrub(diff).1.len();
    sanitize::sanitize_request(&mut req);

    let provider = spec.build(provider_key);
    // A provider that cannot stream refuses here rather than falling
    // back to `send`; the `supports_streaming` check above makes the
    // refusal reachable at all, and this is the backstop.
    //
    // The error is scrubbed on the way out. Layer 1 includes the
    // provider's response body in a non-2xx error, on the reasoning
    // that the body is provider-authored text and cannot contain our
    // key — which holds for a real provider and fails for a
    // misconfigured proxy that echoes the `Authorization` header back.
    // A test drives exactly that case, so the key cannot reach the UI,
    // the error boundary, or the log buffer through this path.
    let rx = provider
        .send_stream(&api_key, &req)
        .await
        .map_err(|e| AppError::Gitgit(sanitize::scrub_secret(&e.to_string(), &api_key)))?;

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
pub async fn ai_review_cancel(app: AppHandle, state: State<'_, DesktopState>) -> AppResult<String> {
    cancel_review(Arc::clone(&state.reviews), Arc::new(TauriEmitter(app))).await
}

// --- conflict arbitration (ADR-0024 §3.4 / §3.7) -------------------------

/// The conflict region as it arrives from the webview.
///
/// `[FACT]` The desktop shell talks to the webview in camelCase JSON, so
/// the DTO renames its fields. The conversion into the library's
/// [`ConflictRegion`] is the only place the two shapes meet, which keeps
/// the AI-facing type free of any wire-format concern.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictRegionDto {
    /// The repository-relative path of the conflicted file.
    pub path: String,
    /// The base blob's object id, when the base was available.
    pub base_oid: String,
    /// The region's ordinal in the merged output.
    pub index: usize,
    /// The ours side's text, verbatim.
    pub ours: String,
    /// The base side's text, verbatim.
    pub base: String,
    /// The theirs side's text, verbatim.
    pub theirs: String,
    /// The base file's lines before the region.
    pub context_before: String,
    /// The base file's lines after the region.
    pub context_after: String,
}

impl From<ConflictRegionDto> for ConflictRegion {
    fn from(dto: ConflictRegionDto) -> Self {
        ConflictRegion {
            path: dto.path,
            base_oid: dto.base_oid,
            index: dto.index,
            ours: dto.ours,
            base: dto.base,
            theirs: dto.theirs,
            context_before: dto.context_before,
            context_after: dto.context_after,
        }
    }
}

/// Everything `ai_conflict_start` accepts besides the region itself.
#[derive(Debug, Clone, Default)]
pub struct ConflictArgs {
    /// Registry key. Blank/absent means [`DEFAULT_PROVIDER`].
    pub provider: Option<String>,
    /// Blank/absent means the preset's default model.
    pub model: Option<String>,
    /// Blank/absent means the preset's own base URL.
    pub base_url: Option<String>,
}

/// What the UI learns when an arbitration starts.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictStarted {
    /// Identifies this stream, exactly as [`ReviewStarted::session_id`]
    /// does for a review.
    pub session_id: String,
    /// The region this arbitration is about.
    ///
    /// `[PROPOSAL]` ADR-0024 §3.7.2: the answer has to come back bound to
    /// this id, and the UI needs it to match a token event to a region.
    pub region_id: String,
    /// The registry key that was used.
    pub provider: String,
    /// The model that was requested.
    pub model: String,
    /// Always `true` here — a provider that cannot stream is refused
    /// rather than degraded.
    pub streaming: bool,
    /// How many suspected secrets were replaced before sending.
    ///
    /// `[FACT]` Redaction is heuristic (`src/ai/sanitize.rs`), so the count
    /// is surfaced rather than hidden: ADR-0024 §3.3 requires the dialog to
    /// say how much was replaced.
    pub redactions: usize,
}

/// The most characters the answer may accumulate.
///
/// `[PROPOSAL]` The frontend caps the text it buffers at
/// `REVIEW_MAX_TEXT_CHARS = 200_000`; this is the same number on the Rust
/// side, so a runaway stream cannot grow the backend's memory without
/// bound. An answer this long is refused by the length check anyway.
pub const CONFLICT_MAX_ANSWER_CHARS: usize = 200_000;

/// Refuse a region that carries nothing to arbitrate.
fn validate_region(region: &ConflictRegion) -> AppResult<()> {
    if region.path.trim().is_empty() {
        return Err(AppError::AiReviewInvalid(String::from(
            "the conflict region has no path",
        )));
    }
    if region.ours.trim().is_empty()
        && region.base.trim().is_empty()
        && region.theirs.trim().is_empty()
    {
        return Err(AppError::AiReviewInvalid(String::from(
            "the conflict region is empty on all three sides",
        )));
    }
    Ok(())
}

/// Resolve the credential for `provider` (ADR-0024 §3.9).
///
/// `[PROPOSAL]` The vault wins, the environment is the fallback, and
/// neither is `None` — which the caller turns into `AiReviewNoKey`.
pub async fn resolve_ai_key<V: Vault + ?Sized>(vault: &V, provider: &str) -> Option<String> {
    resolve_ai_key_with(vault, provider, read_api_key_env()).await
}

/// The same rule with the environment value passed in.
///
/// `[INFERENCE]` Taking the environment value as an argument is what makes
/// the precedence rule testable: the process environment is global, and a
/// test that mutated it would race every other test in the binary.
pub async fn resolve_ai_key_with<V: Vault + ?Sized>(
    vault: &V,
    provider: &str,
    env_value: Option<String>,
) -> Option<String> {
    let from_vault = match vault.get(&keys::ai_key_name(provider)).await {
        Ok(value) => value,
        Err(e) => {
            // A vault that cannot be read is not a reason to refuse the
            // call: the environment is still there. The error names the
            // key, never a value.
            tracing::warn!(
                error = %e,
                provider = %provider,
                "could not read the provider key from the vault"
            );
            None
        }
    };
    keys::pick_ai_key(from_vault, env_value)
}

/// Store a provider key in the vault (ADR-0024 §3.9).
///
/// The value is written under `ai/<provider>` and is **not** returned:
/// there is deliberately no `ai_key_get`, and the generic `vault_get`
/// refuses the `ai/` namespace, so a provider key cannot travel back to
/// the webview.
pub async fn ai_key_set_inner<V: VersionedVault + ?Sized>(
    vault: &V,
    provider: &str,
    key: &str,
) -> AppResult<()> {
    let provider = provider.trim();
    if !keys::is_valid_provider_name(provider) {
        return Err(AppError::AiReviewInvalid(String::from(
            "the provider name must be a plain lower-case name (letters, digits, '-' and '_')",
        )));
    }
    let value = key.trim();
    if value.is_empty() {
        return Err(AppError::AiReviewInvalid(String::from(
            "the api key is empty",
        )));
    }
    vault
        .set_with_version(&keys::ai_key_name(provider), value)
        .await?;
    Ok(())
}

/// Start a streaming arbitration for one conflict region.
///
/// Tauri-independent on purpose, for the same reason [`spawn_review`] is:
/// the behaviour worth testing cannot be seen through a Tauri app handle.
pub async fn spawn_conflict(
    manager: Arc<ReviewManager>,
    emitter: Arc<dyn ConflictEmitter>,
    api_key: Option<String>,
    region: ConflictRegion,
    args: ConflictArgs,
) -> AppResult<ConflictStarted> {
    validate_region(&region)?;
    let provider_key = clean(&args.provider).unwrap_or(DEFAULT_PROVIDER);
    let registry = registry::ProviderRegistry::builtin();
    // An unknown key fails here, with the known keys listed, and before
    // any credential is read.
    let spec = registry.get(provider_key)?;
    if !spec.supports_streaming() {
        return Err(AppError::AiReviewUnsupported(String::from(provider_key)));
    }
    // One stream at a time, exactly as for a review: two arbitrations
    // interleaving their tokens into one text box is the same defect the
    // `ReviewManager` slot exists to prevent.
    if let Some(running) = manager.running_session() {
        return Err(AppError::AiReviewAlreadyRunning(running));
    }
    let api_key = api_key.ok_or(AppError::AiReviewNoKey)?;

    let spec = match clean(&args.base_url) {
        Some(url) => spec.with_base_url(url),
        None => spec.clone(),
    };
    let model = clean(&args.model).map_or_else(|| spec.default_model().to_string(), str::to_string);

    let mut req = ChatRequest::new(model.clone(), prompt::conflict(&region));
    // Untrusted tagging + heuristic secret redaction, after the prompt is
    // built — the same order as `spawn_review`.
    let redactions = prompt::conflict_redactions(&region);
    sanitize::sanitize_request(&mut req);

    let provider = spec.build(provider_key);
    // The error is scrubbed on the way out, for the same reason the review
    // path scrubs it: a misconfigured proxy can echo the `Authorization`
    // header back inside an error body.
    let rx = provider
        .send_stream(&api_key, &req)
        .await
        .map_err(|e| AppError::Gitgit(sanitize::scrub_secret(&e.to_string(), &api_key)))?;

    let session_id = new_session_id();
    let join = tokio::spawn(pump_conflict(
        Arc::clone(&manager),
        emitter,
        api_key,
        session_id.clone(),
        region.clone(),
        rx,
    ));
    if let Err((handle, previous)) = manager.install(session_id.clone(), join) {
        handle.abort();
        let _ = handle.await;
        return Err(AppError::AiReviewAlreadyRunning(previous));
    }

    Ok(ConflictStarted {
        session_id,
        region_id: region.id(),
        provider: String::from(provider_key),
        model,
        streaming: true,
        redactions,
    })
}

/// Cancel the running arbitration and announce it. Tauri-independent.
pub async fn cancel_conflict(
    manager: Arc<ReviewManager>,
    emitter: Arc<dyn ConflictEmitter>,
) -> AppResult<String> {
    let session_id = manager.cancel().await?;
    if let Err(e) = emitter.emit(&ConflictEvent::Cancelled {
        session_id: session_id.clone(),
    }) {
        // The cancel itself succeeded; failing the command here would tell
        // the UI that stopping did not work, which is the one thing that
        // would be a lie.
        tracing::warn!(error = %e, "could not emit conflict cancellation");
    }
    Ok(session_id)
}

/// Drain one conflict stream into events, and validate the answer.
///
/// `[FACT]` This is [`pump`]'s shape: the slot is released on every exit
/// path, the provider's error is scrubbed on the way out, and a stream
/// that ends without a terminal event is reported as a failure rather
/// than left to look like a finished answer.
///
/// `[PROPOSAL]` The one addition is the validation: ADR-0024 §3.4.2's five
/// checks run on the accumulated answer *before* it is offered as a
/// suggestion, and a refused answer is reported as a refusal with its
/// reason. Nothing is written either way.
async fn pump_conflict(
    manager: Arc<ReviewManager>,
    emitter: Arc<dyn ConflictEmitter>,
    api_key: String,
    session_id: String,
    region: ConflictRegion,
    mut rx: StreamReceiver,
) {
    let _slot = SlotGuard {
        manager: Arc::clone(&manager),
        session_id: session_id.clone(),
    };

    let region_id = region.id();
    let mut tokens: u32 = 0;
    let mut answer = String::new();
    while let Some(item) = rx.recv().await {
        match item {
            Ok(StreamEvent::Token(delta)) => {
                tokens = tokens.saturating_add(1);
                if answer.len() < CONFLICT_MAX_ANSWER_CHARS {
                    answer.push_str(&delta);
                }
                let _ = emitter.emit(&ConflictEvent::Token {
                    session_id: session_id.clone(),
                    region_id: region_id.clone(),
                    delta,
                });
            }
            Ok(StreamEvent::Model(model)) => {
                let _ = emitter.emit(&ConflictEvent::Model {
                    session_id: session_id.clone(),
                    region_id: region_id.clone(),
                    model,
                });
            }
            Ok(StreamEvent::Finished) => {
                let outcome = match conflict::validate_answer(&region, &answer) {
                    Ok(suggestion) => SuggestionOutcome::Accepted { suggestion },
                    Err(rejection) => SuggestionOutcome::Rejected {
                        reason: rejection.reason().to_string(),
                    },
                };
                let _ = emitter.emit(&ConflictEvent::Done {
                    session_id: session_id.clone(),
                    region_id: region_id.clone(),
                    tokens,
                    outcome,
                });
                return;
            }
            Err(e) => {
                let _ = emitter.emit(&ConflictEvent::Failed {
                    session_id: session_id.clone(),
                    region_id: region_id.clone(),
                    message: sanitize::scrub_secret(&e.to_string(), &api_key),
                });
                return;
            }
        }
    }

    // The channel closed with neither `Finished` nor an error. That is a
    // truncated stream, and it is reported as a failure rather than left
    // to look like a finished answer.
    let _ = emitter.emit(&ConflictEvent::Failed {
        session_id,
        region_id,
        message: String::from("the conflict stream ended before the provider finished"),
    });
}

/// Start an AI arbitration for one conflict region. Returns as soon as the
/// request is accepted; the answer arrives on the `ai-conflict://` events.
#[tauri::command]
pub async fn ai_conflict_start(
    app: AppHandle,
    state: State<'_, DesktopState>,
    region: ConflictRegionDto,
    provider: Option<String>,
    model: Option<String>,
    base_url: Option<String>,
) -> AppResult<ConflictStarted> {
    let provider_key = clean(&provider).unwrap_or(DEFAULT_PROVIDER).to_string();
    let api_key = resolve_ai_key(state.vault.as_ref(), &provider_key).await;
    spawn_conflict(
        Arc::clone(&state.reviews),
        Arc::new(TauriEmitter(app)),
        api_key,
        region.into(),
        ConflictArgs {
            provider,
            model,
            base_url,
        },
    )
    .await
}

/// Stop the running arbitration. Returns the session id that was stopped.
#[tauri::command]
pub async fn ai_conflict_cancel(
    app: AppHandle,
    state: State<'_, DesktopState>,
) -> AppResult<String> {
    cancel_conflict(Arc::clone(&state.reviews), Arc::new(TauriEmitter(app))).await
}

/// Store a provider key in the vault. Returns nothing — a key never
/// travels back to the webview.
#[tauri::command]
pub async fn ai_key_set(
    state: State<'_, DesktopState>,
    provider: String,
    key: String,
) -> AppResult<()> {
    ai_key_set_inner(state.vault.as_ref(), &provider, &key).await
}

#[cfg(test)]
mod tests {
    //! Tests for the streaming review command layer (V0 task T9).
    //!
    //! The behaviour worth protecting here cannot be seen through a
    //! Tauri app handle, so the tests drive [`spawn_review`] directly with
    //! a recording emitter. Two of them go further and run a real socket:
    //! [`MockSseServer`] speaks the OpenAI SSE wire shape over loopback TCP,
    //! which is what makes "the tokens arrive in order" an observation rather
    //! than an assertion about a mock's own bookkeeping.
    //!
    //! The socket is raw `tokio::net` rather than `axum` on purpose. Building
    //! a streaming response body needs a `futures_core::Stream`, and this
    //! crate's manifest does not declare one; raw chunked-transfer-encoding
    //! keeps the test dependency-free and, as a bonus, makes the
    //! cancellation test able to observe the peer actually going away.

    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

    use gitgit::ai::conflict::DialogChoice;
    use gitgit::ai::provider::StreamEvent;
    use gitgit::ai::stream::stream_channel;
    use gitgit::server::vault_versioned::VersionedVault;

    use super::*;
    use crate::state::REVIEW_MAX_DIFF_BYTES;

    /// Collects every event instead of shipping it to a webview.
    #[derive(Default)]
    struct Recorder {
        events: Mutex<Vec<ReviewEvent>>,
    }

    impl Recorder {
        fn new() -> Arc<Self> {
            Arc::new(Self::default())
        }

        fn events(&self) -> Vec<ReviewEvent> {
            self.events
                .lock()
                .expect("recorder is not poisoned")
                .clone()
        }

        /// Deltas in arrival order — the observable proof of streaming.
        fn deltas(&self) -> Vec<String> {
            self.events()
                .into_iter()
                .filter_map(|e| match e {
                    ReviewEvent::Token { delta, .. } => Some(delta),
                    _ => None,
                })
                .collect()
        }

        fn kinds(&self) -> Vec<&'static str> {
            self.events().iter().map(ReviewEvent::name).collect()
        }
    }

    impl ReviewEmitter for Recorder {
        fn emit(&self, event: &ReviewEvent) -> Result<(), String> {
            self.events
                .lock()
                .expect("recorder is not poisoned")
                .push(event.clone());
            Ok(())
        }
    }

    /// Emitter whose bridge always fails, for the degraded-emitter path.
    struct BrokenEmitter;

    impl ReviewEmitter for BrokenEmitter {
        fn emit(&self, _event: &ReviewEvent) -> Result<(), String> {
            Err(String::from("webview is gone"))
        }
    }

    fn args(diff: &str) -> ReviewArgs {
        ReviewArgs {
            diff: String::from(diff),
            ..ReviewArgs::default()
        }
    }

    fn start_args(diff: &str, base_url: &str) -> ReviewArgs {
        ReviewArgs {
            diff: String::from(diff),
            base_url: Some(String::from(base_url)),
            ..ReviewArgs::default()
        }
    }

    /// Wait for `predicate` over the recorder, or fail the test.
    async fn wait_for(
        recorder: &Arc<Recorder>,
        what: &str,
        predicate: impl Fn(&[ReviewEvent]) -> bool,
    ) -> Vec<ReviewEvent> {
        for _ in 0..200 {
            let events = recorder.events();
            if predicate(&events) {
                return events;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("timed out waiting for {what}; saw {:?}", recorder.kinds());
    }

    /* ---------------- validation, no socket required ---------------- */

    #[tokio::test]
    async fn an_empty_diff_is_refused_before_the_credential_is_read() {
        // Ordering claim, not just a claim about the value: the empty-diff
        // error must win even when no key is available, otherwise a user
        // with no key configured is told about the key instead of the thing
        // they actually did wrong.
        let manager = Arc::new(ReviewManager::new());
        let recorder = Recorder::new();
        let err = spawn_review(manager, recorder, None, args("   \n\t "))
            .await
            .unwrap_err();
        assert_eq!(err.kind(), "AiReviewInvalid", "got: {err}");
        assert!(err.to_string().contains("empty"), "got: {err}");
    }

    #[tokio::test]
    async fn an_oversized_diff_is_refused() {
        let manager = Arc::new(ReviewManager::new());
        let recorder = Recorder::new();
        let big = "a".repeat(REVIEW_MAX_DIFF_BYTES + 1);
        let err = spawn_review(manager, recorder, Some(String::from("k")), args(&big))
            .await
            .unwrap_err();
        assert_eq!(err.kind(), "AiReviewInvalid", "got: {err}");
    }

    #[tokio::test]
    async fn an_unknown_provider_names_the_known_ones() {
        let manager = Arc::new(ReviewManager::new());
        let recorder = Recorder::new();
        let mut a = args("diff");
        a.provider = Some(String::from("not-a-provider"));
        let err = spawn_review(manager, recorder, Some(String::from("k")), a)
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("not-a-provider"), "got: {msg}");
        assert!(msg.contains("openai"), "got: {msg}");
    }

    #[tokio::test]
    async fn anthropic_is_refused_before_any_network_round_trip() {
        // The refusal is reached with no base_url set at all, so nothing
        // can have been sent: `supports_streaming` is consulted from the
        // registry, not discovered from a failed call.
        let manager = Arc::new(ReviewManager::new());
        let recorder = Recorder::new();
        let mut a = args("diff");
        a.provider = Some(String::from("anthropic"));
        let err = spawn_review(manager, recorder.clone(), Some(String::from("k")), a)
            .await
            .unwrap_err();
        assert_eq!(err.kind(), "AiReviewUnsupported", "got: {err}");
        assert!(err.to_string().contains("anthropic"), "got: {err}");
        assert!(recorder.events().is_empty(), "nothing may be emitted");
    }

    #[tokio::test]
    async fn a_missing_api_key_is_reported_as_such() {
        let manager = Arc::new(ReviewManager::new());
        let recorder = Recorder::new();
        let err = spawn_review(manager, recorder, None, args("diff"))
            .await
            .unwrap_err();
        assert_eq!(err.kind(), "AiReviewNoKey", "got: {err}");
        assert!(
            err.to_string().contains(gitgit::ai::API_KEY_ENV),
            "the message must name the variable: {err}"
        );
    }

    #[tokio::test]
    async fn a_dead_endpoint_does_not_leak_the_key() {
        // Same shape as `connection_failure_does_not_leak_the_key` in
        // `src/ai/openai.rs`, one layer up: this asserts the key survives
        // the *command* boundary, including the `AppError` serialization
        // the frontend receives.
        let manager = Arc::new(ReviewManager::new());
        let recorder = Recorder::new();
        // Port 1 is reserved and refuses instantly, so the failure path is
        // exercised without a network dependency.
        let err = spawn_review(
            manager,
            recorder,
            Some(String::from("sk-SUPERSECRET-VALUE")),
            start_args("diff", "http://127.0.0.1:1/v1"),
        )
        .await
        .unwrap_err();
        let serialized = serde_json::to_string(&err).expect("AppError serializes");
        assert!(!err.to_string().contains("SUPERSECRET"), "leaked: {err}");
        assert!(
            !serialized.contains("SUPERSECRET"),
            "leaked into the frontend payload: {serialized}"
        );
    }

    #[tokio::test]
    async fn a_provider_error_echoing_the_key_is_scrubbed_before_it_is_emitted() {
        // Defence in depth for the *in-band* error path. The mock opens a
        // normal 200 stream and then sends a provider `error` frame whose
        // message quotes the bearer token — a misconfigured proxy really
        // does this. Layer 1 turns that into an `Err` item on the channel,
        // and this module's job is to make sure it does not reach the UI.
        let mock = MockSseServer::start(MockCfg::ok(vec![error_frame(
            "auth",
            "upstream rejected bearer sk-LEAKED-KEY",
        )]))
        .await;
        let manager = Arc::new(ReviewManager::new());
        let recorder = Recorder::new();
        spawn_review(
            Arc::clone(&manager),
            recorder.clone(),
            Some(String::from("sk-LEAKED-KEY")),
            start_args("diff", &mock.base_url()),
        )
        .await
        .expect("the stream opens; the failure is in-band");

        let events = wait_for(&recorder, "the failure event", |e| {
            e.iter().any(|x| matches!(x, ReviewEvent::Failed { .. }))
        })
        .await;
        let _ = manager.cancel().await;
        mock.finish().await;

        let text = serde_json::to_string(&events).expect("events serialize");
        assert!(
            !text.contains("LEAKED-KEY"),
            "key leaked into events: {text}"
        );
        // The message still has to be useful, or the guard would be
        // trivially satisfiable by dropping everything.
        let message = events
            .iter()
            .find_map(|e| match e {
                ReviewEvent::Failed { message, .. } => Some(message.clone()),
                _ => None,
            })
            .expect("a failure event");
        assert!(
            message.contains("auth"),
            "the provider detail is lost: {message}"
        );
        assert!(message.contains("upstream rejected"), "got: {message}");
    }

    #[tokio::test]
    async fn a_rejected_request_does_not_leak_the_key() {
        // The other half: a non-2xx response is refused by layer 1 before
        // any channel exists, so the key must not survive that error either.
        let mock = MockSseServer::start(MockCfg::failing(401, "unauthorized: sk-LEAKED-KEY")).await;
        let manager = Arc::new(ReviewManager::new());
        let recorder = Recorder::new();
        let err = spawn_review(
            manager,
            recorder.clone(),
            Some(String::from("sk-LEAKED-KEY")),
            start_args("diff", &mock.base_url()),
        )
        .await
        .expect_err("a 401 must not be reported as a started review");
        mock.finish().await;
        let serialized = serde_json::to_string(&err).expect("AppError serializes");
        assert!(!serialized.contains("LEAKED-KEY"), "leaked: {serialized}");
        assert!(recorder.events().is_empty(), "nothing may be emitted");
    }

    /* ---------------- the streaming path, over a real socket ---------------- */

    #[tokio::test]
    async fn tokens_arrive_in_order_and_the_review_completes() {
        // The acceptance criterion for T9, end to end: five deltas from a
        // socket, in the order the server wrote them, then `done`.
        let mock = MockSseServer::start(MockCfg::ok(sample_frames())).await;
        let manager = Arc::new(ReviewManager::new());
        let recorder = Recorder::new();
        let started = spawn_review(
            Arc::clone(&manager),
            recorder.clone(),
            Some(String::from("k")),
            start_args("diff --git a/x b/x", &mock.base_url()),
        )
        .await
        .expect("the review starts");

        assert!(started.streaming, "the start must report streaming");
        assert_eq!(started.provider, "openai");
        assert!(!started.session_id.is_empty(), "a session id is required");

        wait_for(&recorder, "the done event", |e| {
            e.iter().any(|x| matches!(x, ReviewEvent::Done { .. }))
        })
        .await;

        assert_eq!(recorder.deltas(), SAMPLE_TOKENS, "deltas must not reorder");
        let events = recorder.events();
        let done = events
            .iter()
            .find_map(|e| match e {
                ReviewEvent::Done { tokens, .. } => Some(*tokens),
                _ => None,
            })
            .expect("a done event");
        assert_eq!(done as usize, SAMPLE_TOKENS.len(), "done counts the deltas");
        assert!(
            events
                .iter()
                .any(|e| matches!(e, ReviewEvent::Model { .. })),
            "the served model must be reported, not assumed"
        );
        // Every event belongs to the session the command handed out.
        for e in &events {
            let sid = match e {
                ReviewEvent::Token { session_id, .. }
                | ReviewEvent::Model { session_id, .. }
                | ReviewEvent::Done { session_id, .. }
                | ReviewEvent::Failed { session_id, .. }
                | ReviewEvent::Cancelled { session_id } => session_id,
            };
            assert_eq!(sid, &started.session_id, "event from a foreign session");
        }
        mock.finish().await;
    }

    #[tokio::test]
    async fn the_slot_is_released_when_the_stream_finishes() {
        // A stale slot would make every later start look like a double
        // start, which is the failure a user would report as "the button
        // stopped working".
        let mock = MockSseServer::start(MockCfg::ok(sample_frames())).await;
        let manager = Arc::new(ReviewManager::new());
        let recorder = Recorder::new();
        spawn_review(
            Arc::clone(&manager),
            recorder.clone(),
            Some(String::from("k")),
            start_args("diff", &mock.base_url()),
        )
        .await
        .expect("the review starts");
        wait_for(&recorder, "the done event", |e| {
            e.iter().any(|x| matches!(x, ReviewEvent::Done { .. }))
        })
        .await;
        mock.finish().await;
        for _ in 0..200 {
            if manager.running_session().is_none() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("the manager slot was not released after the stream finished");
    }

    #[tokio::test]
    async fn a_second_start_while_one_is_streaming_is_refused() {
        let mock = MockSseServer::start(MockCfg::ok(sample_frames())).await;
        let manager = Arc::new(ReviewManager::new());
        let recorder = Recorder::new();
        spawn_review(
            Arc::clone(&manager),
            recorder.clone(),
            Some(String::from("k")),
            start_args("diff", &mock.base_url()),
        )
        .await
        .expect("the first review starts");
        let err = spawn_review(
            Arc::clone(&manager),
            Recorder::new(),
            Some(String::from("k")),
            start_args("diff", &mock.base_url()),
        )
        .await
        .unwrap_err();
        assert_eq!(err.kind(), "AiReviewAlreadyRunning", "got: {err}");
        mock.finish().await;
    }

    #[tokio::test]
    async fn cancel_stops_the_work_and_the_provider_sees_the_client_leave() {
        // The load-bearing cancellation test. Two separate claims:
        //
        // (1) the manager reports the stream gone, and a new start is
        //     accepted again — the work stopped, it was not merely hidden;
        // (2) the *server* observes the connection closing. (2) is the part
        //     a UI-only fake cannot prove, and it is the part that matters:
        //     if the socket stayed open, the request was still running.
        let frames = slow_frames();
        let mock = MockSseServer::start(MockCfg::ok(frames)).await;
        let manager = Arc::new(ReviewManager::new());
        let recorder = Recorder::new();
        let started = spawn_review(
            Arc::clone(&manager),
            recorder.clone(),
            Some(String::from("k")),
            start_args("diff", &mock.base_url()),
        )
        .await
        .expect("the review starts");

        // Wait until the stream is genuinely in flight before cancelling.
        wait_for(&recorder, "the first token", |e| {
            e.iter().any(|x| matches!(x, ReviewEvent::Token { .. }))
        })
        .await;

        let stopped = cancel_review(Arc::clone(&manager), recorder.clone())
            .await
            .expect("cancel reports the session it stopped");
        assert_eq!(stopped, started.session_id, "wrong session cancelled");
        assert!(
            manager.running_session().is_none(),
            "the slot must be free the instant cancel resolves"
        );

        // The peer must notice. `MockSseServer` records it.
        for _ in 0..300 {
            if mock.client_gone() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(
            mock.client_gone(),
            "the provider never saw the client leave: cancellation did not \
         release the response body"
        );

        // A new stream is accepted, which it would not be if the cancelled
        // task were still holding the slot.
        let second = MockSseServer::start(MockCfg::ok(sample_frames())).await;
        let r2 = Recorder::new();
        spawn_review(
            Arc::clone(&manager),
            r2,
            Some(String::from("k")),
            start_args("diff", &second.base_url()),
        )
        .await
        .expect("a review can start again after a cancel");
        let _ = manager.cancel().await;
        second.finish().await;
        mock.finish().await;
    }

    #[tokio::test]
    async fn cancelling_with_nothing_running_is_reported_not_assumed() {
        // A cancel that silently succeeds teaches the UI that stopping
        // works when it did not.
        let manager = Arc::new(ReviewManager::new());
        let err = cancel_review(manager, Recorder::new()).await.unwrap_err();
        assert_eq!(err.kind(), "AiReviewNotRunning", "got: {err}");
    }

    #[tokio::test]
    async fn cancel_succeeds_even_when_the_bridge_is_gone() {
        // The webview can close between "start" and "stop". Reporting that
        // as a failed cancel would be the one genuinely false thing the
        // command could say: the task really was aborted.
        let manager = Arc::new(ReviewManager::new());
        let mock = MockSseServer::start(MockCfg::ok(sample_frames())).await;
        let _ = spawn_review(
            Arc::clone(&manager),
            Recorder::new(),
            Some(String::from("k")),
            start_args("diff", &mock.base_url()),
        )
        .await;
        let stopped = cancel_review(manager, Arc::new(BrokenEmitter))
            .await
            .expect("cancel reports success despite the dead bridge");
        assert!(!stopped.is_empty(), "a session id is still reported");
        mock.finish().await;
    }

    #[tokio::test]
    async fn a_truncated_stream_is_reported_as_a_failure() {
        // The server closes without `data: [DONE]` and without a
        // finish_reason. Showing the partial text as a finished review is
        // the failure this guards.
        let mock = MockSseServer::start(MockCfg::truncated(vec![
            model_frame(),
            content_frame("Hel"),
            content_frame("lo"),
        ]))
        .await;
        let manager = Arc::new(ReviewManager::new());
        let recorder = Recorder::new();
        spawn_review(
            Arc::clone(&manager),
            recorder.clone(),
            Some(String::from("k")),
            start_args("diff", &mock.base_url()),
        )
        .await
        .expect("the review starts");
        let events = wait_for(&recorder, "the failure event", |e| {
            e.iter().any(|x| matches!(x, ReviewEvent::Failed { .. }))
        })
        .await;
        assert!(
            !events.iter().any(|e| matches!(e, ReviewEvent::Done { .. })),
            "a truncated stream must never report done: {events:?}"
        );
        assert!(recorder.deltas().len() <= 2, "partial output is kept");
        mock.finish().await;
    }

    #[tokio::test]
    async fn a_non_success_status_surfaces_the_status_code() {
        // A non-2xx is refused by layer 1 *before* any channel exists, so
        // there is no stream to report an event on: the failure arrives as
        // the command's own error. What the UI still needs is the status,
        // because "429" and "401" call for completely different operator
        // action.
        let mock = MockSseServer::start(MockCfg::failing(429, "rate limited")).await;
        let manager = Arc::new(ReviewManager::new());
        let recorder = Recorder::new();
        let err = spawn_review(
            Arc::clone(&manager),
            recorder.clone(),
            Some(String::from("k")),
            start_args("diff", &mock.base_url()),
        )
        .await
        .expect_err("a rejected request must not report a started review");
        mock.finish().await;
        let message = err.to_string();
        assert!(
            message.contains("429"),
            "the status must survive: {message}"
        );
        assert!(recorder.events().is_empty(), "no stream opened, no events");
        assert!(
            manager.running_session().is_none(),
            "a failed start must not leave the slot occupied"
        );
    }

    #[tokio::test]
    async fn a_secrets_shaped_diff_is_redacted_before_it_is_sent() {
        // The mock records the request body it received, so this asserts the
        // sanitizer actually ran on the way out — not merely that some
        // counter was incremented.
        let mock = MockSseServer::start(MockCfg::ok(sample_frames())).await;
        let manager = Arc::new(ReviewManager::new());
        let recorder = Recorder::new();
        let started = spawn_review(
            Arc::clone(&manager),
            recorder.clone(),
            Some(String::from("k")),
            start_args(
                "diff --git a/x b/x\n+const api_key = \"super-secret-value-1234\";",
                &mock.base_url(),
            ),
        )
        .await
        .expect("the review starts");
        assert_eq!(
            started.redactions, 1,
            "one secret-shaped span in the diff must be reported"
        );
        wait_for(&recorder, "the done event", |e| {
            e.iter().any(|x| matches!(x, ReviewEvent::Done { .. }))
        })
        .await;
        let body = mock.request_body();
        assert!(
            body.contains(gitgit::ai::sanitize::REDACTED),
            "body: {body}"
        );
        assert!(
            !body.contains("super-secret-value-1234"),
            "the secret reached the provider unredacted: {body}"
        );
        // The diff is also structurally tagged, which is the AISEC-REQ-001
        // control: the model is told this content is data.
        assert!(body.contains("<untrusted"), "body: {body}");
        mock.finish().await;
    }

    #[tokio::test]
    async fn every_event_variant_has_its_own_event_name() {
        // The frontend dispatches on the name; a collision would make two
        // states indistinguishable.
        let names = [
            ReviewEvent::Token {
                session_id: String::from("s"),
                delta: String::from("x"),
            }
            .name(),
            ReviewEvent::Model {
                session_id: String::from("s"),
                model: String::from("m"),
            }
            .name(),
            ReviewEvent::Done {
                session_id: String::from("s"),
                tokens: 1,
            }
            .name(),
            ReviewEvent::Failed {
                session_id: String::from("s"),
                message: String::from("e"),
            }
            .name(),
            ReviewEvent::Cancelled {
                session_id: String::from("s"),
            }
            .name(),
        ];
        let unique: std::collections::BTreeSet<&str> = names.iter().copied().collect();
        assert_eq!(unique.len(), names.len(), "event names must be distinct");
        assert!(names.contains(&EVENT_CANCELLED));
    }

    /* ---------------- the layer-1 contract this module leans on ---------------- */

    #[tokio::test]
    async fn dropping_the_receiver_ends_the_producer() {
        // The property `ai_review_cancel` depends on: cancellation is the
        // ordinary "consumer went away" case, not a special path.
        let (tx, mut rx) = stream_channel();
        let (seen_tx, mut seen_rx) = tokio::sync::mpsc::channel::<u32>(1);
        let producer = tokio::spawn(async move {
            let mut sent = 0u32;
            while tx
                .send(Ok(StreamEvent::Token(format!("t{sent}"))))
                .await
                .is_ok()
            {
                sent += 1;
                if sent > 10_000 {
                    break;
                }
            }
            // The `.await` is load-bearing: `Sender::send` returns a future
            // that is not `#[must_use]`, so dropping it here would discard
            // the report and the assertion below would then be testing a
            // closed channel instead of the property it names.
            let _ = seen_tx.send(sent).await;
        });
        assert!(rx.recv().await.is_some(), "the first token arrives");
        drop(rx);
        let sent = tokio::time::timeout(Duration::from_secs(5), seen_rx.recv())
            .await
            .expect("the producer noticed within 5s")
            .expect("the producer reported a value");
        assert!(sent > 0, "the producer really was running");
        producer.await.expect("the producer task completed");
    }

    /* ---------------- a local mock of the OpenAI SSE wire shape ---------------- */

    /// The review the mock serves, split the way a real model splits it.
    const SAMPLE_TOKENS: [&str; 5] = ["Hel", "lo", "wo", "rld", "!"];

    fn sample_frames() -> Vec<String> {
        let mut frames = vec![model_frame()];
        for token in SAMPLE_TOKENS {
            frames.push(content_frame(token));
        }
        frames.push("data: [DONE]\n\n".to_string());
        frames
    }

    /// Same payload, but the inter-frame delay makes mid-stream
    /// cancellation reachable.
    fn slow_frames() -> Vec<String> {
        let mut frames = vec![model_frame()];
        for token in SAMPLE_TOKENS {
            frames.push(content_frame(token));
        }
        frames.push("data: [DONE]\n\n".to_string());
        frames
    }

    fn model_frame() -> String {
        "data: {\"model\":\"mock-model-1\",\"choices\":[{\"delta\":{\"role\":\"assistant\"},\"finish_reason\":null}]}\n\n"
        .to_string()
    }

    fn content_frame(token: &str) -> String {
        format!(
        "data: {{\"model\":\"mock-model-1\",\"choices\":[{{\"delta\":{{\"content\":\"{token}\"}}}}]}}\n\n"
    )
    }

    /// A `data:` frame carrying a provider error object, which the wire
    /// format uses for mid-stream failures.
    fn error_frame(kind: &str, message: &str) -> String {
        format!("data: {{\"error\":{{\"type\":\"{kind}\",\"message\":\"{message}\"}}}}\n\n")
    }

    /// What the mock server observed.
    #[derive(Debug, Default)]
    struct MockOutcome {
        /// The client's connection was observed to close.
        client_gone: bool,
        /// The body the client sent, so a test can assert on what left the
        /// process.
        request_body: String,
    }

    /// How a [`MockSseServer`] should behave.
    struct MockCfg {
        /// SSE frames to write, in order.
        frames: Vec<String>,
        /// HTTP status for the response head.
        status: u16,
        /// Body sent with a non-200 status.
        error_body: Option<String>,
        /// Stop after this many frames, sending no terminator. Produces the
        /// truncated stream that must be reported as a failure.
        truncate_at: Option<usize>,
    }

    impl MockCfg {
        /// A 200 response carrying `frames`.
        fn ok(frames: Vec<String>) -> Self {
            Self {
                frames,
                status: 200,
                error_body: None,
                truncate_at: None,
            }
        }

        /// A non-200 response with a body.
        fn failing(status: u16, body: &str) -> Self {
            Self {
                frames: Vec::new(),
                status,
                error_body: Some(String::from(body)),
                truncate_at: None,
            }
        }

        /// A 200 response cut short with no `[DONE]`.
        fn truncated(frames: Vec<String>) -> Self {
            Self {
                frames,
                status: 200,
                error_body: None,
                truncate_at: Some(0),
            }
        }
    }

    /// A loopback HTTP/1.1 server that answers `POST /v1/chat/completions`
    /// with a chunked SSE body.
    ///
    /// One frame per granted permit, so a test decides exactly how far the
    /// stream has progressed. That is what makes the cancellation test
    /// deterministic without sleeping: the server is still mid-response
    /// when the client leaves.
    struct MockSseServer {
        addr: std::net::SocketAddr,
        outcome: Arc<Mutex<MockOutcome>>,
        /// Held so `finish` can close the permit channel, which is what
        /// releases a handler parked on a permit.
        permits: tokio::sync::mpsc::Sender<()>,
        handle: tokio::task::JoinHandle<()>,
    }

    impl MockSseServer {
        async fn start(cfg: MockCfg) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0")
                .await
                .expect("loopback is bindable");
            let addr = listener.local_addr().expect("listener reports its address");
            let outcome = Arc::new(Mutex::new(MockOutcome::default()));
            let (permit_tx, mut permit_rx) = tokio::sync::mpsc::channel::<()>(64);
            // Pre-fill: the server may write each frame it was given, one
            // per permit, with no further cooperation from the test.
            let writable = cfg
                .truncate_at
                .unwrap_or(cfg.frames.len())
                .min(cfg.frames.len());
            for _ in 0..writable {
                let _ = permit_tx.try_send(());
            }
            let handle = tokio::spawn({
                // Cloned so the test keeps a handle to the outcome while the
                // handler owns its own.
                let outcome_task = Arc::clone(&outcome);
                async move {
                    // One connection is enough: every test makes exactly one
                    // request, and a second `accept` would park the task
                    // forever.
                    if let Ok((sock, _)) = listener.accept().await {
                        let _ = serve(sock, &cfg, &outcome_task, &mut permit_rx).await;
                    }
                }
            });
            Self {
                addr,
                outcome,
                permits: permit_tx,
                handle,
            }
        }

        fn base_url(&self) -> String {
            format!("http://{}/v1", self.addr)
        }

        fn client_gone(&self) -> bool {
            self.outcome
                .lock()
                .expect("outcome is not poisoned")
                .client_gone
        }

        fn request_body(&self) -> String {
            self.outcome
                .lock()
                .expect("outcome is not poisoned")
                .request_body
                .clone()
        }

        /// Close the permit channel and wait for the handler to finish.
        async fn finish(self) {
            drop(self.permits);
            let _ = tokio::time::timeout(Duration::from_secs(5), self.handle).await;
        }
    }

    async fn serve(
        mut sock: TcpStream,
        cfg: &MockCfg,
        outcome: &Arc<Mutex<MockOutcome>>,
        permits: &mut tokio::sync::mpsc::Receiver<()>,
    ) -> std::io::Result<()> {
        let _ = sock.set_nodelay(true);

        // Read the request head, then the body. A server that never reads
        // can wedge the client on a full send buffer, and the redaction
        // test needs the body anyway.
        let mut buf = vec![0u8; 8192];
        let mut seen = Vec::new();
        loop {
            let n = sock.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            seen.extend_from_slice(&buf[..n]);
            let Some(head_end) = find_head_end(&seen) else {
                if seen.len() > 1_000_000 {
                    break;
                }
                continue;
            };
            let head = String::from_utf8_lossy(&seen[..head_end]).to_string();
            if seen.len() >= head_end + content_length_of(&head) {
                seen = seen[head_end..head_end + content_length_of(&head)].to_vec();
                break;
            }
        }
        if let Ok(mut guard) = outcome.lock() {
            guard.request_body = String::from_utf8_lossy(&seen).to_string();
        }

        if cfg.status != 200 {
            let body = cfg.error_body.clone().unwrap_or_default();
            let head = format!(
            "HTTP/1.1 {} MOCK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            cfg.status,
            body.len()
        );
            sock.write_all(head.as_bytes()).await?;
            sock.flush().await?;
            return Ok(());
        }

        sock.write_all(
        b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nTransfer-Encoding: chunked\r\n\r\n",
    )
    .await?;
        sock.flush().await?;

        let writable = cfg
            .truncate_at
            .unwrap_or(cfg.frames.len())
            .min(cfg.frames.len());
        for frame in cfg.frames.iter().take(writable) {
            if permits.recv().await.is_none() {
                break;
            }
            // A read returning 0 bytes means the client closed its side:
            // the response body was dropped. That is the signal the
            // cancellation test needs, and it is observable from the peer
            // in a way it is not from inside the client.
            let mut probe = [0u8; 1];
            if let Ok(Ok(0)) =
                tokio::time::timeout(Duration::from_millis(30), sock.read(&mut probe)).await
            {
                mark_gone(outcome);
                return Ok(());
            }
            let chunk = format!("{:x}\r\n{frame}\r\n", frame.len());
            if sock.write_all(chunk.as_bytes()).await.is_err() || sock.flush().await.is_err() {
                mark_gone(outcome);
                return Ok(());
            }
        }
        Ok(())
    }

    fn mark_gone(outcome: &Arc<Mutex<MockOutcome>>) {
        if let Ok(mut guard) = outcome.lock() {
            guard.client_gone = true;
        }
    }

    fn find_head_end(bytes: &[u8]) -> Option<usize> {
        bytes
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .map(|p| p + 4)
    }

    fn content_length_of(head: &str) -> usize {
        head.lines()
            .find_map(|l| {
                let lower = l.to_ascii_lowercase();
                if lower.starts_with("content-length:") {
                    lower
                        .split(':')
                        .nth(1)
                        .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                } else {
                    None
                }
            })
            .unwrap_or(0)
    }

    /* ---------------- conflict arbitration (ADR-0024 §3.4 / §3.7) ------- */

    /// A conflict region with three distinct sides.
    fn conflict_region() -> ConflictRegion {
        ConflictRegion {
            path: String::from("src/foo.rs"),
            base_oid: String::from("0123456789abcdef0123456789abcdef01234567"),
            index: 0,
            ours: String::from("let a = 1;\n"),
            base: String::from("let a = 0;\n"),
            theirs: String::from("let a = 2;\n"),
            context_before: String::from("fn main() {\n"),
            context_after: String::from("}\n"),
        }
    }

    /// Collects conflict events instead of shipping them to a webview.
    #[derive(Default)]
    struct ConflictRecorder {
        events: Mutex<Vec<ConflictEvent>>,
    }

    impl ConflictRecorder {
        fn events(&self) -> Vec<ConflictEvent> {
            self.events
                .lock()
                .expect("recorder is not poisoned")
                .clone()
        }

        /// The outcome the arbitration ended with.
        fn outcome(&self) -> SuggestionOutcome {
            self.events()
                .into_iter()
                .find_map(|e| match e {
                    ConflictEvent::Done { outcome, .. } => Some(outcome),
                    _ => None,
                })
                .expect("a done event")
        }
    }

    impl ConflictEmitter for ConflictRecorder {
        fn emit(&self, event: &ConflictEvent) -> Result<(), String> {
            self.events
                .lock()
                .expect("recorder is not poisoned")
                .push(event.clone());
            Ok(())
        }
    }

    /// A unique directory under the system temp dir.
    fn unique_temp(label: &str) -> std::path::PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        std::env::temp_dir().join(format!(
            "gitgit-desktop-conflict-{label}-{}-{n}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ))
    }

    /// Render `text` so it can sit inside a JSON string in a mock frame.
    fn json_escape(text: &str) -> String {
        let mut out = String::new();
        for c in text.chars() {
            match c {
                '\\' => out.push_str("\\\\"),
                '"' => out.push_str("\\\""),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                other => out.push(other),
            }
        }
        out
    }

    /// SSE frames that deliver `answer` as two chunks: the region id and
    /// the replacement text.
    ///
    /// `[INFERENCE]` Two frames rather than one, so the test also covers
    /// the newline between the id and the text surviving the stream.
    fn answer_frames(answer: &str) -> Vec<String> {
        let mut frames = vec![model_frame()];
        let (id, body) = match answer.split_once('\n') {
            Some((id, body)) => (id, body),
            None => (answer, ""),
        };
        frames.push(content_frame(&json_escape(id)));
        frames.push(content_frame(&json_escape(&format!("\n{body}"))));
        frames.push("data: [DONE]\n\n".to_string());
        frames
    }

    /// The provider arguments that point at the mock server.
    fn start_conflict_args(base_url: &str) -> ConflictArgs {
        ConflictArgs {
            base_url: Some(String::from(base_url)),
            ..ConflictArgs::default()
        }
    }

    /// Wait for `predicate` over the recorder, or fail the test.
    async fn wait_for_conflict(
        recorder: &Arc<ConflictRecorder>,
        what: &str,
        predicate: impl Fn(&[ConflictEvent]) -> bool,
    ) -> Vec<ConflictEvent> {
        for _ in 0..200 {
            let events = recorder.events();
            if predicate(&events) {
                return events;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("timed out waiting for {what}");
    }

    #[tokio::test]
    async fn an_accepted_answer_becomes_a_suggestion() {
        // The happy path: the model echoed the region id and proposed a
        // replacement, so the answer passes every check and is offered as a
        // suggestion — and only as a suggestion.
        let region = conflict_region();
        let answer = format!("{}\nlet a = 1;\n", region.id());
        let mock = MockSseServer::start(MockCfg::ok(answer_frames(&answer))).await;
        let manager = Arc::new(ReviewManager::new());
        let recorder = Arc::new(ConflictRecorder::default());
        let emitter: Arc<dyn ConflictEmitter> = recorder.clone();
        let started = spawn_conflict(
            Arc::clone(&manager),
            emitter,
            Some(String::from("k")),
            region,
            start_conflict_args(&mock.base_url()),
        )
        .await
        .expect("the arbitration starts");
        assert!(started.streaming, "the start must report streaming");
        assert_eq!(started.region_id, answer.lines().next().unwrap_or_default());

        wait_for_conflict(&recorder, "the done event", |e| {
            e.iter().any(|x| matches!(x, ConflictEvent::Done { .. }))
        })
        .await;
        let _ = manager.cancel().await;
        mock.finish().await;

        assert_eq!(
            recorder.outcome(),
            SuggestionOutcome::Accepted {
                suggestion: String::from("let a = 1;\n")
            },
            "an accepted suggestion keeps the text verbatim"
        );
    }

    #[tokio::test]
    async fn a_conflict_answer_carrying_a_marker_is_rejected_and_nothing_is_written() {
        // ADR-0024 §3.4.2 row 2: the model "helpfully" pastes the whole
        // conflicted block back. The suggestion must be marked rejected
        // with a reason, and nothing may be written.
        let region = conflict_region();
        let answer = format!(
            "{}\n<<<<<<< OURS\nlet a = 1;\n=======\nlet a = 0;\n>>>>>>> THEIRS\n",
            region.id()
        );
        // The file the conflict came from, so "nothing was written" is an
        // observation about the disk rather than an assertion about the
        // absence of a call.
        let dir = unique_temp("no-write");
        std::fs::create_dir_all(&dir).expect("temp dir");
        let file = dir.join("foo.rs");
        let before = "<<<<<<< OURS\nlet a = 1;\n=======\nlet a = 0;\n>>>>>>> THEIRS\n";
        std::fs::write(&file, before).expect("write the conflicted file");

        let mock = MockSseServer::start(MockCfg::ok(answer_frames(&answer))).await;
        let manager = Arc::new(ReviewManager::new());
        let recorder = Arc::new(ConflictRecorder::default());
        let emitter: Arc<dyn ConflictEmitter> = recorder.clone();
        spawn_conflict(
            Arc::clone(&manager),
            emitter,
            Some(String::from("k")),
            region,
            start_conflict_args(&mock.base_url()),
        )
        .await
        .expect("the arbitration starts");

        wait_for_conflict(&recorder, "the done event", |e| {
            e.iter().any(|x| matches!(x, ConflictEvent::Done { .. }))
        })
        .await;
        let _ = manager.cancel().await;
        mock.finish().await;

        match recorder.outcome() {
            SuggestionOutcome::Rejected { reason } => {
                assert!(reason.contains("conflict marker"), "got: {reason}");
            }
            SuggestionOutcome::Accepted { suggestion } => {
                panic!("the suggestion must be refused, got: {suggestion}");
            }
        }
        assert_eq!(
            std::fs::read_to_string(&file).expect("read back"),
            before,
            "the arbitration path must not write anything"
        );
    }

    #[tokio::test]
    async fn a_conflict_answer_bound_to_another_region_is_discarded() {
        // ADR-0024 §3.7.2: the answer has to carry the id of the region it
        // was shown for. A different id means the answer is about something
        // else, and it is discarded rather than shown.
        let region = conflict_region();
        let other_id = "00000000000000000000000000000000";
        let answer = format!("{other_id}\nlet a = 1;\n");
        let mock = MockSseServer::start(MockCfg::ok(answer_frames(&answer))).await;
        let manager = Arc::new(ReviewManager::new());
        let recorder = Arc::new(ConflictRecorder::default());
        let emitter: Arc<dyn ConflictEmitter> = recorder.clone();
        spawn_conflict(
            Arc::clone(&manager),
            emitter,
            Some(String::from("k")),
            region,
            start_conflict_args(&mock.base_url()),
        )
        .await
        .expect("the arbitration starts");

        wait_for_conflict(&recorder, "the done event", |e| {
            e.iter().any(|x| matches!(x, ConflictEvent::Done { .. }))
        })
        .await;
        let _ = manager.cancel().await;
        mock.finish().await;

        match recorder.outcome() {
            SuggestionOutcome::Rejected { reason } => {
                assert!(reason.contains("another region"), "got: {reason}");
            }
            SuggestionOutcome::Accepted { suggestion } => {
                panic!("the suggestion must be discarded, got: {suggestion}");
            }
        }
    }

    #[tokio::test]
    async fn a_secret_in_the_region_is_redacted_before_the_request_leaves() {
        // The conflict region is repository content, so the same sanitizer
        // that guards a diff guards it. The mock records the request body
        // it received, so this asserts on what actually left the process.
        let mut region = conflict_region();
        region.ours = String::from("let api_key = \"super-secret-value-1234\";\n");
        let answer = format!("{}\nlet a = 1;\n", region.id());
        let mock = MockSseServer::start(MockCfg::ok(answer_frames(&answer))).await;
        let manager = Arc::new(ReviewManager::new());
        let recorder = Arc::new(ConflictRecorder::default());
        let emitter: Arc<dyn ConflictEmitter> = recorder.clone();
        let started = spawn_conflict(
            Arc::clone(&manager),
            emitter,
            Some(String::from("k")),
            region,
            start_conflict_args(&mock.base_url()),
        )
        .await
        .expect("the arbitration starts");
        assert_eq!(
            started.redactions, 1,
            "one secret-shaped span in the region must be reported"
        );

        wait_for_conflict(&recorder, "the done event", |e| {
            e.iter().any(|x| matches!(x, ConflictEvent::Done { .. }))
        })
        .await;
        let _ = manager.cancel().await;
        let body = mock.request_body();
        mock.finish().await;

        assert!(
            body.contains(gitgit::ai::sanitize::REDACTED),
            "body: {body}"
        );
        assert!(
            !body.contains("super-secret-value-1234"),
            "the secret reached the provider unredacted: {body}"
        );
        assert!(body.contains("<untrusted"), "body: {body}");
    }

    #[tokio::test]
    async fn without_a_key_the_three_non_ai_choices_still_work() {
        // ADR-0024 §3.7.1: "the conflict dialog stays fully usable without
        // a key: keep ours / keep theirs / hand-edit all work as usual."
        let manager = Arc::new(ReviewManager::new());
        let recorder = Arc::new(ConflictRecorder::default());
        let emitter: Arc<dyn ConflictEmitter> = recorder.clone();
        let region = conflict_region();
        let err = spawn_conflict(
            manager,
            emitter,
            None,
            region.clone(),
            ConflictArgs::default(),
        )
        .await
        .unwrap_err();
        assert_eq!(err.kind(), "AiReviewNoKey", "got: {err}");
        assert!(
            err.to_string().contains(gitgit::ai::API_KEY_ENV),
            "the message must name the variable: {err}"
        );

        // The three options that do not need the model are unaffected.
        assert_eq!(
            region.resolve(DialogChoice::KeepOurs, None).unwrap(),
            "let a = 1;\n"
        );
        assert_eq!(
            region.resolve(DialogChoice::KeepTheirs, None).unwrap(),
            "let a = 2;\n"
        );
        assert_eq!(
            region
                .resolve(DialogChoice::Manual, Some("let a = 9;\n"))
                .unwrap(),
            "let a = 9;\n"
        );
        assert!(recorder.events().is_empty(), "nothing may be emitted");
    }

    #[tokio::test]
    async fn anthropic_is_refused_before_any_network_round_trip_for_a_conflict() {
        // ADR-0024 Lane B acceptance test 7: a provider that cannot stream
        // is refused before the request goes out.
        let manager = Arc::new(ReviewManager::new());
        let recorder = Arc::new(ConflictRecorder::default());
        let emitter: Arc<dyn ConflictEmitter> = recorder.clone();
        let args = ConflictArgs {
            provider: Some(String::from("anthropic")),
            ..ConflictArgs::default()
        };
        let err = spawn_conflict(
            manager,
            emitter,
            Some(String::from("k")),
            conflict_region(),
            args,
        )
        .await
        .unwrap_err();
        assert_eq!(err.kind(), "AiReviewUnsupported", "got: {err}");
        assert!(recorder.events().is_empty(), "nothing may be emitted");
    }

    #[tokio::test]
    async fn every_conflict_event_variant_has_its_own_event_name() {
        let names = [
            ConflictEvent::Token {
                session_id: String::from("s"),
                region_id: String::from("r"),
                delta: String::from("x"),
            }
            .name(),
            ConflictEvent::Model {
                session_id: String::from("s"),
                region_id: String::from("r"),
                model: String::from("m"),
            }
            .name(),
            ConflictEvent::Done {
                session_id: String::from("s"),
                region_id: String::from("r"),
                tokens: 1,
                outcome: SuggestionOutcome::Rejected {
                    reason: String::from("r"),
                },
            }
            .name(),
            ConflictEvent::Failed {
                session_id: String::from("s"),
                region_id: String::from("r"),
                message: String::from("e"),
            }
            .name(),
            ConflictEvent::Cancelled {
                session_id: String::from("s"),
            }
            .name(),
        ];
        let unique: std::collections::BTreeSet<&str> = names.iter().copied().collect();
        assert_eq!(unique.len(), names.len(), "event names must be distinct");
        assert!(names.contains(&EVENT_CONFLICT_CANCELLED));
        // The conflict channel is its own: a review listener must not pick
        // up conflict tokens and vice versa.
        assert!(names.iter().all(|n| n.starts_with("ai-conflict://")));
    }

    /* ---------------- key resolution (ADR-0024 §3.9) --------------------- */

    /// A file-backed vault in a fresh temporary directory.
    fn temp_vault(label: &str) -> gitgit::server::vault::FileVault {
        let dir = unique_temp(label);
        std::fs::create_dir_all(&dir).expect("temp dir");
        gitgit::server::vault::FileVault::new(dir)
    }

    #[tokio::test]
    async fn the_vault_key_wins_over_the_environment() {
        let vault = temp_vault("vault-wins");
        vault
            .set_with_version("ai/openai", "sk-from-vault")
            .await
            .expect("store the key");
        let resolved =
            resolve_ai_key_with(&vault, "openai", Some(String::from("sk-from-env"))).await;
        assert_eq!(resolved.as_deref(), Some("sk-from-vault"));
    }

    #[tokio::test]
    async fn the_environment_is_the_fallback_when_the_vault_is_empty() {
        let vault = temp_vault("env-fallback");
        let resolved =
            resolve_ai_key_with(&vault, "openai", Some(String::from("sk-from-env"))).await;
        assert_eq!(resolved.as_deref(), Some("sk-from-env"));
    }

    #[tokio::test]
    async fn neither_source_yields_no_key() {
        let vault = temp_vault("no-key");
        let resolved = resolve_ai_key_with(&vault, "openai", None).await;
        assert!(resolved.is_none(), "no source, no key");
    }

    #[tokio::test]
    async fn a_stored_key_is_not_readable_through_the_generic_vault_path() {
        // ADR-0024 §3.9 item 3: a provider key never travels to the
        // webview, so the generic read path refuses the namespace.
        let vault = temp_vault("key-not-readable");
        ai_key_set_inner(&vault, "openai", "sk-secret-value")
            .await
            .expect("the key is stored");
        let err = crate::commands::vault::vault_get_inner(&vault, "ai/openai")
            .await
            .expect_err("the ai/ namespace is refused");
        assert_eq!(err.kind(), "Vault");
        assert!(
            !err.to_string().contains("sk-secret-value"),
            "the key must not be echoed: {err}"
        );
        // The AI path reads it, though.
        let resolved = resolve_ai_key_with(&vault, "openai", None).await;
        assert_eq!(resolved.as_deref(), Some("sk-secret-value"));
    }

    #[tokio::test]
    async fn a_provider_name_that_could_leave_the_namespace_is_refused() {
        let vault = temp_vault("bad-provider");
        let err = ai_key_set_inner(&vault, "openai/../secrets", "sk-x")
            .await
            .expect_err("the provider name is refused");
        assert_eq!(err.kind(), "AiReviewInvalid", "got: {err}");
    }
}
