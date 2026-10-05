//! OpenAI-compatible wire protocol.
//!
//! Implements the request/response shape from archived design §5.5
//! (`openai.go`): `POST <base_url>/chat/completions` with a
//! `Bearer` authorization header.
//!
//! One adapter serves every OpenAI-compatible endpoint. The archived
//! design notes this explicitly at §5.5: "兼容端点 (本地 vLLM 也用同一
//! 客户端)" — vLLM reuses this client. The registry relies on the same
//! property for Ollama's OpenAI-compatible surface and for DeepSeek.

use async_trait::async_trait;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use crate::ai::provider::{AiProvider, ChatRequest, ChatResponse, ProviderCapabilities};
use crate::ai::stream::{stream_channel, SseDecoder, SseField, StreamEvent, StreamReceiver};
use crate::error::{GitGitError, Result};

/// A provider speaking the OpenAI chat-completions protocol.
///
/// The credential is deliberately **not** stored here: [`AiProvider::send`]
/// takes it per call, so rotating a key does not require rebuilding the
/// provider. A stored copy would be one more field that can end up in a
/// `Debug` dump.
#[derive(Debug, Clone)]
pub struct OpenAiProvider {
    name: String,
    base_url: String,
    models: &'static [&'static str],
}

impl OpenAiProvider {
    pub fn new(
        name: impl Into<String>,
        base_url: impl Into<String>,
        models: &'static [&'static str],
    ) -> Self {
        Self {
            name: name.into(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            models,
        }
    }
}

#[derive(Debug, Serialize)]
struct WireMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Serialize)]
struct WireRequest<'a> {
    model: &'a str,
    messages: Vec<WireMessage<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct WireResponse {
    #[serde(default)]
    choices: Vec<WireChoice>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    usage: Option<WireUsage>,
}

#[derive(Debug, Deserialize)]
struct WireChoice {
    #[serde(default)]
    message: Option<WireMessageOut>,
}

#[derive(Debug, Deserialize)]
struct WireMessageOut {
    #[serde(default)]
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WireUsage {
    #[serde(default)]
    prompt_tokens: Option<u32>,
    #[serde(default)]
    completion_tokens: Option<u32>,
}

/// One frame of a streaming response: the same envelope as
/// [`WireResponse`] but with a `delta` per choice instead of a whole
/// `message`.
#[derive(Debug, Deserialize)]
struct WireStreamChunk {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    choices: Vec<WireStreamChoice>,
    /// Providers signal mid-stream failures by sending an `error` object
    /// in a normal `data:` frame and then closing. Ignoring the field
    /// would let the stream look like it finished cleanly.
    #[serde(default)]
    error: Option<WireStreamError>,
}

#[derive(Debug, Deserialize)]
struct WireStreamChoice {
    #[serde(default)]
    delta: Option<WireDelta>,
    /// `stop` on a normal end, but also `content_filter`,
    /// `length`, and provider-specific refusals. Any value terminates.
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WireDelta {
    #[serde(default)]
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WireStreamError {
    #[serde(default)]
    message: Option<String>,
    #[serde(rename = "type", default)]
    kind: Option<String>,
}

/// Build a transport error for `provider`.
///
/// Deliberately not an `impl Fn(...)` factory: a closure returned from a
/// helper would capture the provider name by reference, and the returned
/// `impl Fn` type would have to name that lifetime. The call sites are few
/// enough that inlining the closure is clearer than the bound gymnastics.
fn http_err(provider: &str, what: &str, e: &reqwest::Error) -> GitGitError {
    // `reqwest::Error`'s Display includes the request URL, never headers,
    // so the API key — which only ever lives in a header we set — cannot
    // reach this string.
    GitGitError::Ai(format!("{provider}: {what}: {e}"))
}

#[async_trait]
impl AiProvider for OpenAiProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_system: true,
            // This adapter speaks the OpenAI-compatible streaming
            // protocol, so every preset built on it (openai, deepseek,
            // ollama, vllm) streams.
            supports_streaming: true,
            models: self.models,
        }
    }

    async fn send(&self, api_key: &str, req: &ChatRequest) -> Result<ChatResponse> {
        let body = self.wire_body(req, false)?;
        let url = format!("{}/chat/completions", self.base_url);
        let http = reqwest::Client::new();
        let resp = http
            .post(&url)
            .bearer_auth(api_key)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body)
            .send()
            .await
            .map_err(|e| http_err(&self.name, "request failed", &e))?;

        let status = resp.status();
        let raw = resp
            .text()
            .await
            .map_err(|e| http_err(&self.name, "reading response body failed", &e))?;

        if !status.is_success() {
            // The body is included because providers put the actionable
            // detail there ("model not found", "insufficient quota"). It
            // is the provider's error text, not our request, so it does
            // not contain our API key.
            return Err(GitGitError::Ai(format!(
                "{}: HTTP {} — {}",
                self.name,
                status.as_u16(),
                truncate(&raw, 400)
            )));
        }

        let parsed: WireResponse = serde_json::from_str(&raw)
            .map_err(|e| GitGitError::Ai(format!("{}: parse response: {e}", self.name)))?;

        let content = parsed
            .choices
            .first()
            .and_then(|c| c.message.as_ref())
            .and_then(|m| m.content.clone())
            .ok_or_else(|| {
                GitGitError::Ai(format!(
                    "{}: response carried no assistant content",
                    self.name
                ))
            })?;

        Ok(ChatResponse {
            content,
            model: parsed.model.unwrap_or_else(|| req.model.clone()),
            input_tokens: parsed.usage.as_ref().and_then(|u| u.prompt_tokens),
            output_tokens: parsed.usage.as_ref().and_then(|u| u.completion_tokens),
        })
    }

    async fn send_stream(&self, api_key: &str, req: &ChatRequest) -> Result<StreamReceiver> {
        let body = self.wire_body(req, true)?;
        let url = format!("{}/chat/completions", self.base_url);
        let http = reqwest::Client::new();
        let resp = http
            .post(&url)
            .bearer_auth(api_key)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body)
            .send()
            .await
            .map_err(|e| http_err(&self.name, "stream request failed", &e))?;

        let status = resp.status();
        if !status.is_success() {
            // Same reasoning as `send`: the provider's own error text is
            // the actionable part, and it cannot contain our key, which
            // only ever travels in a request header.
            let raw = resp
                .text()
                .await
                .map_err(|e| http_err(&self.name, "reading error body failed", &e))?;
            return Err(GitGitError::Ai(format!(
                "{}: HTTP {} — {}",
                self.name,
                status.as_u16(),
                truncate(&raw, 400)
            )));
        }

        // The read loop is detached so the caller gets its receiver
        // immediately. Cancelling is `drop(receiver)`: the next send
        // fails, the loop returns, and the response body is dropped,
        // which closes the connection.
        let (tx, rx) = stream_channel();
        let provider = self.name.clone();
        tokio::spawn(async move {
            pump(provider, resp, tx).await;
        });
        Ok(rx)
    }
}

impl OpenAiProvider {
    /// Serialize a request body.
    ///
    /// reqwest is built here without the `json` feature (the crate is
    /// pinned to rustls plus `stream`), so the body is serialized
    /// explicitly rather than via `RequestBuilder::json`.
    fn wire_body(&self, req: &ChatRequest, stream: bool) -> Result<Vec<u8>> {
        let messages: Vec<WireMessage<'_>> = req
            .messages
            .iter()
            .map(|m| WireMessage {
                role: m.role.as_openai(),
                content: &m.content,
            })
            .collect();

        serde_json::to_vec(&WireRequest {
            model: &req.model,
            messages,
            temperature: req.temperature,
            max_tokens: req.max_tokens,
            stream,
        })
        .map_err(|e| GitGitError::Ai(format!("{}: serialize request: {e}", self.name)))
    }
}

/// Drain one SSE response body into `tx`.
///
/// Every exit path either sends a terminal event or sends an error: a
/// caller that stops receiving without an error knows the stream ended,
/// and one that receives an error knows the output so far is incomplete.
async fn pump(provider: String, resp: reqwest::Response, tx: crate::ai::stream::StreamSender) {
    // `bytes_stream()` is `!Unpin`, and `StreamExt::next` needs `Unpin`;
    // pinning it in a Box is the cheap way to get a `StreamExt` that
    // works without depending on a pin-projection macro.
    let mut body = Box::pin(resp.bytes_stream());
    let mut decoder = SseDecoder::new();
    let mut terminated = false;

    while let Some(next) = body.next().await {
        let chunk = match next {
            Ok(bytes) => bytes,
            Err(e) => {
                // Mid-stream disconnect. Reported rather than treated as
                // a clean end: a half-written review is not a review.
                let _ = tx
                    .send(Err(http_err(&provider, "stream read failed", &e)))
                    .await;
                return;
            }
        };

        for field in decoder.push(&chunk) {
            match field {
                SseField::Ignored => {}
                SseField::Done => terminated = true,
                SseField::Data(payload) => {
                    let chunk: WireStreamChunk = match serde_json::from_str(&payload) {
                        Ok(c) => c,
                        Err(e) => {
                            // A frame we cannot parse means we do not know
                            // what the model said. Skipping it silently
                            // would drop model output with no signal.
                            let _ = tx
                                .send(Err(GitGitError::Ai(format!(
                                    "{provider}: parse stream frame: {e}"
                                ))))
                                .await;
                            return;
                        }
                    };

                    if let Some(err) = chunk.error {
                        let kind = err.kind.unwrap_or_else(|| "error".to_string());
                        let detail = err.message.unwrap_or_else(|| "no detail".to_string());
                        // Provider-authored text, not our request, so the
                        // API key cannot be in it. Capped for the same
                        // reason an HTTP error body is.
                        let _ = tx
                            .send(Err(GitGitError::Ai(format!(
                                "{provider}: stream error ({kind}): {}",
                                truncate(&detail, 400)
                            ))))
                            .await;
                        return;
                    }

                    if let Some(model) = chunk.model.filter(|m| !m.is_empty()) {
                        if tx.send(Ok(StreamEvent::Model(model))).await.is_err() {
                            return; // consumer gone: cancelled
                        }
                    }

                    for choice in &chunk.choices {
                        if let Some(token) = choice.delta.as_ref().and_then(|d| d.content.clone()) {
                            // Providers open every stream with a role-
                            // only delta whose `content` is null or empty.
                            // Emitting those as empty tokens would show up
                            // in the UI as blank appends.
                            if !token.is_empty()
                                && tx.send(Ok(StreamEvent::Token(token))).await.is_err()
                            {
                                return; // consumer gone: cancelled
                            }
                        }
                        if choice.finish_reason.is_some() {
                            terminated = true;
                        }
                    }
                }
            }

            if terminated {
                let _ = tx.send(Ok(StreamEvent::Finished)).await;
                return;
            }
        }
    }

    if terminated {
        let _ = tx.send(Ok(StreamEvent::Finished)).await;
    } else {
        let _ = tx
            .send(Err(GitGitError::Ai(format!(
                "{provider}: stream ended without a terminator \
                 (no `data: [DONE]` and no finish_reason)"
            ))))
            .await;
    }
}

/// Cap an error body so a verbose provider response cannot flood the
/// terminal or a CI log.
pub(crate) fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let kept: String = s.chars().take(max).collect();
    format!("{kept}… (truncated)")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::ai::provider::{ChatMessage, Role};

    fn p() -> OpenAiProvider {
        OpenAiProvider::new("test-openai", "http://127.0.0.1:1/v1", &["m"])
    }

    #[tokio::test]
    async fn connection_failure_does_not_leak_the_key() {
        // Port 1 is reserved and refuses instantly, so this exercises the
        // error path without a network dependency.
        let req = ChatRequest::new("m", vec![ChatMessage::system("hi")]);
        let err = p().send("sk-SUPERSECRET", &req).await.unwrap_err();
        let msg = err.to_string();
        assert!(!msg.contains("SUPERSECRET"), "api key leaked: {msg}");
    }

    #[test]
    fn trailing_slash_is_normalized() {
        // Without this, a configured `https://host/v1/` would produce
        // `https://host/v1//chat/completions`.
        let p = OpenAiProvider::new("n", "https://host/v1/", &["m"]);
        assert_eq!(p.base_url(), "https://host/v1");
    }

    #[test]
    fn wire_request_omits_unset_options_and_disables_streaming() {
        let req = WireRequest {
            model: "gpt-x",
            messages: vec![WireMessage {
                role: "user",
                content: "hi",
            }],
            temperature: None,
            max_tokens: None,
            stream: false,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"stream\":false"), "got: {json}");
        assert!(!json.contains("temperature"), "got: {json}");
        assert!(!json.contains("max_tokens"), "got: {json}");
    }

    #[test]
    fn anthropic_system_role_is_not_serialized_as_a_message() {
        // Guards the property the Anthropic adapter depends on.
        assert!(Role::System.as_anthropic().is_none());
        assert_eq!(Role::User.as_anthropic(), Some("user"));
    }

    #[test]
    fn truncate_caps_by_characters() {
        assert_eq!(truncate("abc", 10), "abc");
        let long = "é".repeat(20);
        let out = truncate(&long, 5);
        assert!(out.starts_with("ééééé"), "got: {out}");
        assert!(out.contains("truncated"));
    }
}
