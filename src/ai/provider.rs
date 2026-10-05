//! Provider abstraction for the `gitai` subcommands (V0 T7).
//!
//! Ported from the archived design
//! `docs_archive_rust_impl_2026_08_26/design/detailed-design/05-ai-gateway.md`
//! §5.4 (`Provider` interface) and §5.3 (data types). The archive's Go
//! sketch is a design reference only; this is an independent Rust
//! implementation.
//!
//! Scope note `[FACT]`: streaming is implemented for the
//! OpenAI-compatible protocol (four of the five registry presets) and is
//! **refused explicitly** for Anthropic, whose SSE frame shape differs.
//! The refusal is a returned error, never a silent non-streaming call
//! presented to the user as a stream. `EstimateCost()` remains
//! unimplemented rather than stubbed — a stub that returns a zero cost
//! would be worse than a missing method, because a caller cannot tell
//! the difference.

use async_trait::async_trait;

use crate::ai::stream::unsupported;
use crate::error::Result;

/// Who authored a piece of prompt content.
///
/// This is not cosmetic: it drives the structured-tagging rule in
/// [`crate::ai::sanitize`], which is the V0 implementation of
/// AISEC-REQ-001 in the archived requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trust {
    /// Operator-authored text (a system prompt we generated, a CLI flag).
    Trusted,
    /// Content that originated outside the operator's intent: repository
    /// diffs, file contents, commit messages written by third parties.
    Untrusted,
}

/// Chat role. Deliberately an enum rather than a `String` so a provider
/// adapter cannot be handed a role it does not implement.
///
/// `[FACT]` V0 prompts are system + user only. There is no `Assistant`
/// variant because no V0 code path constructs one; adding it back for a
/// future multi-turn feature is a visible change rather than dead weight
/// shipped now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    System,
    User,
}

impl Role {
    /// Wire name per the OpenAI `chat/completions` schema.
    pub fn as_openai(self) -> &'static str {
        match self {
            Role::System => "system",
            Role::User => "user",
        }
    }

    /// Wire name per the Anthropic `messages` schema. Note that Anthropic
    /// carries the system prompt in a **top-level** `system` field rather
    /// than as a message with role `system`; [`crate::ai::anthropic`]
    /// performs that split, which is why this mapping exists separately
    /// from [`Role::as_openai`].
    ///
    /// Returns `None` for [`Role::System`] — that `None` is the signal
    /// the Anthropic adapter uses to lift the system prompt out of the
    /// turn list.
    pub fn as_anthropic(self) -> Option<&'static str> {
        match self {
            Role::System => None,
            Role::User => Some("user"),
        }
    }
}

/// One message in a prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
    /// Defaults to [`Trust::Untrusted`]. The safe default is deliberate:
    /// a caller that forgets to classify new content gets the protective
    /// path, not the permissive one.
    pub trust: Trust,
}

impl ChatMessage {
    /// Operator-authored content. Trusted verbatim.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
            trust: Trust::Trusted,
        }
    }

    /// External content. Gets wrapped and secret-filtered before it ever
    /// leaves the machine.
    pub fn untrusted(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            trust: Trust::Untrusted,
        }
    }
}

/// A non-streaming chat completion request.
#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
}

impl ChatRequest {
    pub fn new(model: impl Into<String>, messages: Vec<ChatMessage>) -> Self {
        Self {
            model: model.into(),
            messages,
            temperature: None,
            max_tokens: None,
        }
    }
}

/// A non-streaming chat completion response.
#[derive(Debug, Clone)]
pub struct ChatResponse {
    /// Text content of the assistant turn.
    pub content: String,
    /// Model the provider says it actually served. Providers may alias or
    /// silently substitute, so this is recorded rather than assumed.
    pub model: String,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
}

/// What a provider can do. Advertised so the CLI can fail with a useful
/// message before spending a network round-trip.
#[derive(Debug, Clone)]
pub struct ProviderCapabilities {
    /// Whether the provider accepts a system role at all. Local models
    /// behind some runtimes do not.
    pub supports_system: bool,
    /// Whether [`AiProvider::send_stream`] is implemented for this
    /// provider. Checked by the UI before offering a streaming control,
    /// so an operator is not invited into a call that will be refused.
    pub supports_streaming: bool,
    /// Model identifiers known to work with this provider. Not exhaustive —
    /// a caller may pass any string the endpoint accepts.
    pub models: &'static [&'static str],
}

/// A single AI provider endpoint.
#[async_trait]
pub trait AiProvider: Send + Sync {
    /// Stable registry key, e.g. `openai`.
    fn name(&self) -> &str;

    /// Base URL the adapter appends its protocol path to. Overridable so
    /// operators can point at a proxy, a vLLM box, or a test double.
    fn base_url(&self) -> &str;

    fn capabilities(&self) -> ProviderCapabilities;

    /// Perform one non-streaming completion.
    ///
    /// Implementations must never include the API key in an error message.
    /// Callers surface these errors to the terminal, so a leaked key in
    /// `Debug` output would end up in scrollback and CI logs.
    async fn send(&self, api_key: &str, req: &ChatRequest) -> Result<ChatResponse>;

    /// Perform one completion, yielding tokens as they arrive.
    ///
    /// # Why a channel rather than a callback
    ///
    /// The two obvious alternatives both lose a property this transport
    /// needs. A callback forces the transport to own the consumer's
    /// error handling, and gives a caller no way to stop the read loop
    /// from the outside. An `async fn` that streams into a caller-supplied
    /// sink is worse: it cannot express "the consumer went away".
    ///
    /// A `mpsc::Receiver` does both jobs. Dropping the receiver makes
    /// the producer's next send fail, which ends the read loop and drops
    /// the in-flight response body — so *cancellation is the ordinary
    /// "consumer left" case* rather than a special path that can be
    /// forgotten. It also needs no new dependency: `tokio` is already
    /// present.
    ///
    /// # Errors are in-band
    ///
    /// Failures after the first byte travel as `Err` items on the
    /// channel rather than closing it. A mid-stream failure and a clean
    /// finish are different outcomes and a UI that cannot tell them apart
    /// will show a truncated answer as a complete one.
    ///
    /// # The default is a refusal, not a fallback
    ///
    /// The default implementation returns
    /// [`crate::ai::stream::unsupported`]. It never falls back to
    /// [`AiProvider::send`]: a provider that cannot stream must say so,
    /// because a non-streaming call dressed up as a stream shows the
    /// operator a frozen UI and no tokens, which reads as a hang rather
    /// than an unsupported feature.
    async fn send_stream(&self, _api_key: &str, _req: &ChatRequest) -> Result<StreamReceiver> {
        Err(unsupported(self.name()))
    }
}

/// Re-exported so callers that only need the stream types do not have to
/// reach into [`crate::ai::stream`] separately.
pub use crate::ai::stream::{StreamEvent, StreamItem, StreamReceiver, StreamSender};
