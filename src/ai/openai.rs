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
use serde::{Deserialize, Serialize};

use crate::ai::provider::{AiProvider, ChatRequest, ChatResponse, ProviderCapabilities};
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
            models: self.models,
        }
    }

    async fn send(&self, api_key: &str, req: &ChatRequest) -> Result<ChatResponse> {
        let messages: Vec<WireMessage<'_>> = req
            .messages
            .iter()
            .map(|m| WireMessage {
                role: m.role.as_openai(),
                content: &m.content,
            })
            .collect();

        // reqwest is built here without the `json` feature (the crate is
        // pinned to rustls only), so the body is serialized explicitly
        // rather than via `RequestBuilder::json`.
        let body = serde_json::to_vec(&WireRequest {
            model: &req.model,
            messages,
            temperature: req.temperature,
            max_tokens: req.max_tokens,
            stream: false,
        })
        .map_err(|e| GitGitError::Ai(format!("{}: serialize request: {e}", self.name)))?;

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
