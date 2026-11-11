//! Anthropic wire protocol.
//!
//! The archived design at §5.5 marks this adapter `[PROPOSAL]`: "Anthropic
//! 兼容 provider 类似，但消息格式与 tool call 表达不同（`tool_use` block
//! 等）". V0 ships the message-format half only — `gitai` V0 has no tool
//! calling, so there is no `tool_use` block to express.
//!
//! Three concrete differences from the OpenAI protocol, all handled here:
//!
//! 1. Auth is an `x-api-key` header, not `Authorization: Bearer`.
//! 2. The system prompt is a **top-level** `system` field, not a message
//!    with `role: "system"`. [`Role::as_anthropic`] returning `None` for
//!    `System` is what makes the split unambiguous.
//! 3. `max_tokens` is **required** by the API, not optional.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::ai::openai::truncate;
use crate::ai::provider::{AiProvider, ChatRequest, ChatResponse, ProviderCapabilities};
use crate::error::{GitGitError, Result};

/// The `anthropic-version` this adapter pins.
///
/// Anthropic requires the header and treats it as a compatibility switch:
/// changing it can change response shape. Pinning one value keeps a
/// provider-side default change from silently altering our parsing.
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Fallback when a caller does not request a budget, because the API
/// rejects a request with no `max_tokens`.
const DEFAULT_MAX_TOKENS: u32 = 1024;

/// A provider speaking the Anthropic messages protocol.
///
/// As with [`crate::ai::openai::OpenAiProvider`], the credential is not
/// stored on the struct — it is supplied per [`AiProvider::send`] call.
#[derive(Debug, Clone)]
pub struct AnthropicProvider {
    name: String,
    base_url: String,
    models: &'static [&'static str],
}

impl AnthropicProvider {
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
    // Top-level, and omitted entirely when empty: sending `"system": ""`
    // is rejected by the API.
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<WireMessage<'a>>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct WireResponse {
    #[serde(default)]
    content: Vec<WireBlock>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    usage: Option<WireUsage>,
}

#[derive(Debug, Deserialize)]
struct WireBlock {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WireUsage {
    #[serde(default)]
    input_tokens: Option<u32>,
    #[serde(default)]
    output_tokens: Option<u32>,
}

#[async_trait]
impl AiProvider for AnthropicProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_system: true,
            // Anthropic's streaming protocol is a different SSE frame
            // shape (`event: content_block_delta` with the text under
            // `delta.text`) and a different request body (`stream: true`
            // plus a mandatory `anthropic-version` header). It is not
            // implemented, so it is advertised as unsupported rather
            // than advertised and then failing at call time.
            supports_streaming: false,
            models: self.models,
        }
    }

    async fn send(&self, api_key: &str, req: &ChatRequest) -> Result<ChatResponse> {
        // Split the system prompt out of the message list. `as_anthropic`
        // yields None for Role::System precisely so this is a total split
        // rather than a filter that could silently drop messages.
        let mut system: Option<String> = None;
        let mut messages = Vec::new();
        for m in &req.messages {
            match m.role.as_anthropic() {
                Some(role) => messages.push(WireMessage {
                    role,
                    content: &m.content,
                }),
                None => {
                    system = Some(match system {
                        Some(prev) => format!("{prev}\n\n{}", m.content),
                        None => m.content.clone(),
                    });
                }
            }
        }

        if messages.is_empty() {
            return Err(GitGitError::Ai(format!(
                "{}: request has no user or assistant message",
                self.name
            )));
        }

        let body = serde_json::to_vec(&WireRequest {
            model: &req.model,
            system,
            messages,
            // The API requires this field, so the Option is resolved here
            // rather than passed through as optional.
            max_tokens: req.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
            temperature: req.temperature,
        })
        .map_err(|e| GitGitError::Ai(format!("{}: serialize request: {e}", self.name)))?;

        let url = format!("{}/messages", self.base_url);
        let http = reqwest::Client::new();
        let resp = http
            .post(&url)
            // Anthropic auth is a dedicated header, not a bearer token.
            .header("x-api-key", api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body)
            .send()
            .await
            .map_err(|e| GitGitError::Ai(format!("{}: request failed: {e}", self.name)))?;

        let status = resp.status();
        let raw = resp
            .text()
            .await
            .map_err(|e| GitGitError::Ai(format!("{}: reading response failed: {e}", self.name)))?;

        if !status.is_success() {
            return Err(GitGitError::Ai(format!(
                "{}: HTTP {} — {}",
                self.name,
                status.as_u16(),
                truncate(&raw, 400)
            )));
        }

        let parsed: WireResponse = serde_json::from_str(&raw)
            .map_err(|e| GitGitError::Ai(format!("{}: parse response: {e}", self.name)))?;

        // Anthropic returns a block list; take the first `text` block and
        // ignore `thinking` / `tool_use` blocks, which V0 never requests.
        let content = parsed
            .content
            .iter()
            .find(|b| b.kind == "text")
            .and_then(|b| b.text.clone())
            .ok_or_else(|| {
                GitGitError::Ai(format!("{}: response carried no text block", self.name))
            })?;

        Ok(ChatResponse {
            content,
            model: parsed.model.unwrap_or_else(|| req.model.clone()),
            input_tokens: parsed.usage.as_ref().and_then(|u| u.input_tokens),
            output_tokens: parsed.usage.as_ref().and_then(|u| u.output_tokens),
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::ai::provider::{ChatMessage, Role, Trust};

    #[test]
    fn system_prompt_is_lifted_to_the_top_level_field() {
        let mut req = ChatRequest::new(
            "claude-x",
            vec![
                ChatMessage::system("You are terse."),
                ChatMessage {
                    role: Role::User,
                    content: "hi".to_string(),
                    trust: Trust::Untrusted,
                },
            ],
        );
        // Reproduce the split the adapter performs.
        let mut system = None;
        let mut count = 0;
        for m in &req.messages {
            match m.role.as_anthropic() {
                Some(_) => count += 1,
                None => system = Some(m.content.clone()),
            }
        }
        assert_eq!(system.as_deref(), Some("You are terse."));
        assert_eq!(count, 1, "the system message must not become a turn");
        req.messages.clear();
    }

    #[test]
    fn max_tokens_defaults_because_the_api_requires_it() {
        let wire = WireRequest {
            model: "m",
            system: None,
            messages: vec![WireMessage {
                role: "user",
                content: "hi",
            }],
            max_tokens: DEFAULT_MAX_TOKENS,
            temperature: None,
        };
        let json = serde_json::to_string(&wire).unwrap();
        assert!(json.contains("\"max_tokens\":1024"), "got: {json}");
        assert!(!json.contains("\"system\""), "got: {json}");
    }

    #[test]
    fn trailing_slash_is_normalized() {
        let p = AnthropicProvider::new("n", "https://api.anthropic.com/v1/", &["m"]);
        assert_eq!(p.base_url(), "https://api.anthropic.com/v1");
    }
}
