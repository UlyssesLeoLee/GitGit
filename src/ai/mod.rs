//! `gitai` — AI-assisted Git workflows (V0 task T7).
//!
//! ## Scope
//!
//! T7 in `docs/plan/v0-tasks.md` specifies "5 个 provider + `gitai`
//! 子命令（commit/explain/review）", with the acceptance criterion
//! "`gitai commit --from-diff` 真打通 OpenAI，输出 commit message".
//!
//! What ships here:
//!
//! - [`provider::AiProvider`] — the provider abstraction (archived design
//!   §5.4).
//! - [`registry::ProviderRegistry`] — five named presets over two wire
//!   protocols. See that module for why the count is expressed as
//!   presets rather than adapters.
//! - [`openai`] / [`anthropic`] — the two wire implementations.
//! - [`sanitize`] — the security pass: untrusted-content tagging
//!   (AISEC-REQ-001) and secret redaction (AISEC-REQ-004).
//! - [`diff`] — git input collection via the `git` subprocess.
//! - [`prompt`] — the three task prompts.
//!
//! ## What is deliberately absent
//!
//! The archived §5.4 interface also declares streaming and cost
//! estimation, and §5.6 marks both V1. They are not stubbed — a stub
//! would be indistinguishable from a working implementation at the call
//! site. They are simply not present, so adding them later is a visible
//! change rather than a silent one.
//!
//! ## The one thing to read before trusting this
//!
//! Redaction in [`sanitize`] is heuristic. It catches the shapes in that
//! module's rule table and misses the rest. It is a meaningful reduction
//! in disclosure risk, not a guarantee, and `gitai` says so in its help
//! text. Anyone treating repository content as safe to ship to a third
//! party because "gitai scrubs it" has misread the module.

pub mod anthropic;
pub mod diff;
pub mod openai;
pub mod prompt;
pub mod provider;
pub mod registry;
pub mod sanitize;

use provider::{ChatRequest, ChatResponse};

/// Environment variable holding the provider API key.
///
/// Kept out of the CLI flag surface on purpose: an API key passed as an
/// argument is visible in `ps` output and lands in shell history.
pub const API_KEY_ENV: &str = "GITGIT_AI_API_KEY";

/// The three `gitai` tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Task {
    /// Write a commit message.
    Commit,
    /// Explain a change.
    Explain,
    /// Review a diff for problems.
    Review,
}

impl Task {
    /// The subcommand name, as typed.
    pub fn as_str(self) -> &'static str {
        match self {
            Task::Commit => "commit",
            Task::Explain => "explain",
            Task::Review => "review",
        }
    }
}

/// One complete `gitai` invocation's inputs.
pub struct Invocation {
    pub task: Task,
    pub provider_key: String,
    pub base_url_override: Option<String>,
    pub model_override: Option<String>,
    pub diff: String,
}

impl Invocation {
    /// Run the invocation end to end: resolve the provider, build the
    /// prompt, sanitize it, call the API.
    ///
    /// Returns the assistant text. Ordering matters — the request is
    /// sanitized *after* the prompt is built, because that is where the
    /// untrusted content finally exists, and sanitizing before would mean
    /// tagging an empty string.
    pub async fn run(&self, api_key: &str) -> crate::error::Result<String> {
        let registry = registry::ProviderRegistry::builtin();
        let spec = registry.get(&self.provider_key)?;

        let spec = match &self.base_url_override {
            Some(url) => spec.with_base_url(url),
            None => spec.clone(),
        };

        let model = self
            .model_override
            .clone()
            .unwrap_or_else(|| spec.default_model().to_string());

        let messages = match self.task {
            Task::Commit => prompt::commit(&self.diff),
            Task::Explain => prompt::explain(&self.diff),
            Task::Review => prompt::review(&self.diff),
        };

        let mut req = ChatRequest::new(model, messages);
        let redacted = sanitize::sanitize_request(&mut req);

        let provider = spec.build(&self.provider_key);
        let resp: ChatResponse = provider.send(api_key, &req).await?;

        // Surface redaction on stderr rather than stdout: stdout is the
        // command's product, and a caller piping it into a commit message
        // file should not pick up a warning.
        if redacted > 0 {
            eprintln!(
                "gitgit: sanitized {} untrusted message(s) before sending; \
                 redaction is heuristic, see `gitai --help`",
                redacted
            );
        }

        Ok(resp.content)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn task_names_match_the_subcommands() {
        assert_eq!(Task::Commit.as_str(), "commit");
        assert_eq!(Task::Explain.as_str(), "explain");
        assert_eq!(Task::Review.as_str(), "review");
    }

    #[test]
    fn api_key_is_an_env_var_not_a_flag() {
        // If this ever moves to a CLI flag it becomes visible in `ps` and
        // in shell history.
        assert!(!API_KEY_ENV.is_empty());
        assert!(API_KEY_ENV
            .chars()
            .all(|c| c.is_ascii_uppercase() || c == '_'));
    }

    #[tokio::test]
    async fn unknown_provider_fails_before_any_network_call() {
        // The error must name the provider, and must not attempt a
        // request — a bad key typo should not cost a network round trip
        // or leak anything.
        let inv = Invocation {
            task: Task::Commit,
            provider_key: "definitely-not-a-provider".to_string(),
            base_url_override: None,
            model_override: None,
            diff: "diff".to_string(),
        };
        let err = inv.run("k").await.unwrap_err();
        assert!(
            err.to_string().contains("definitely-not-a-provider"),
            "got: {err}"
        );
    }
}
