//! Where a provider's API key comes from — ADR-0024 §3.9.
//!
//! # The two sources and their order
//!
//! ADR-0024 §3.9 fixes the priority:
//!
//! ```text
//! resolve_ai_key(provider):
//!     vault.get("ai/<provider>")  ->  non-empty => use it
//!     otherwise std::env::var("GITGIT_AI_API_KEY") -> non-empty => use it
//!     otherwise None
//! ```
//!
//! `[INFERENCE]` The vault wins because it is the *explicit* asset: the
//! user typed the key into Settings, it is versioned, rotatable and
//! survives a restart. The environment variable is the ambient default
//! that usually comes from a launch script or CI. The reverse order would
//! mean a user who configured a key in the UI still had their requests
//! signed with somebody else's credential — a worse surprise than having
//! no key at all.
//!
//! # What this module is not
//!
//! * It does not read the environment. The reader lives with the caller
//!   that owns the process environment (the desktop shell), so the
//!   precedence rule can be exercised without touching a process-global.
//! * It does not store anything. Writing is the desktop shell's job, and
//!   the writer is the only path that ever sees the value.
//!
//! # The CLI is unaffected
//!
//! `[FACT]` `gitai` has no UI and still reads only
//! [`crate::ai::API_KEY_ENV`]: `Invocation::run` takes the key as an
//! argument (`src/ai/mod.rs:98`). Nothing in the CLI path calls this
//! module, so the CLI's "environment only" rule is unchanged.

/// The vault namespace that holds provider keys.
///
/// `[PROPOSAL]` ADR-0024 §3.9 item 1: "key namespace: `ai/<provider>`
/// (e.g. `ai/openai`)".
pub const AI_KEY_PREFIX: &str = "ai/";

/// The vault key that holds `provider`'s API key.
///
/// `[PROPOSAL]` ADR-0024 §3.9 item 1: the key is stored under
/// `ai/<provider>`.
pub fn ai_key_name(provider: &str) -> String {
    format!("{AI_KEY_PREFIX}{provider}")
}

/// True when `key` belongs to the AI namespace.
///
/// `[PROPOSAL]` ADR-0024 §3.9 item 3 requires the `ai/` namespace to be
/// kept out of the generic vault read path, so that a provider key can
/// never be handed to the webview.
pub fn is_ai_key(key: &str) -> bool {
    key.starts_with(AI_KEY_PREFIX)
}

/// True when `name` is a plain provider name.
///
/// `[FACT]` The vault maps a key onto a file path (`FileVault::path_for`,
/// `src/server/vault.rs:696`), and that helper refuses `..`, a leading
/// slash and a Windows drive letter — but it *allows* `/` inside a key,
/// because keys like `openai/api_key` are legitimate. A provider name
/// therefore has to be restricted separately: `ai/../secrets` would be a
/// legal vault key that lives outside the `ai/` namespace.
///
/// `[FACT]` The five registry presets are all lower-case ASCII words
/// (`anthropic`, `deepseek`, `ollama`, `openai`, `vllm` —
/// `src/ai/registry.rs:132`), so the accepted shape is deliberately
/// narrower than the registry.
pub fn is_valid_provider_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// The precedence rule: the vault value wins, the environment is the
/// fallback.
///
/// `[PROPOSAL]` ADR-0024 §3.9 item 2. A blank value counts as absent in
/// both sources: a stray `GITGIT_AI_API_KEY=` must not become a bearer
/// token of `""` that produces a confusing 401 from the provider, and an
/// empty vault entry is not a key either.
pub fn pick_ai_key(vault_value: Option<String>, env_value: Option<String>) -> Option<String> {
    fn usable(value: Option<String>) -> Option<String> {
        value
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    }
    usable(vault_value).or_else(|| usable(env_value))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn the_ai_namespace_is_a_prefix() {
        assert_eq!(ai_key_name("openai"), "ai/openai");
        assert!(is_ai_key("ai/openai"));
        assert!(is_ai_key("ai/"), "the bare prefix is still the namespace");
    }

    #[test]
    fn a_key_outside_the_namespace_is_not_an_ai_key() {
        assert!(!is_ai_key("openai"));
        assert!(!is_ai_key("openai/api_key"));
        assert!(
            !is_ai_key("aix/openai"),
            "the prefix must match the whole segment, not a prefix of it"
        );
    }

    #[test]
    fn a_provider_name_must_be_a_plain_name() {
        assert!(is_valid_provider_name("openai"));
        assert!(is_valid_provider_name("my-provider_2"));
        assert!(!is_valid_provider_name(""));
        assert!(!is_valid_provider_name("Openai"), "case is not normalized");
        assert!(
            !is_valid_provider_name("openai/../secrets"),
            "a path must not be able to leave the namespace"
        );
        assert!(!is_valid_provider_name("../secrets"));
        assert!(!is_valid_provider_name("open ai"));
    }

    #[test]
    fn the_vault_value_wins_over_the_environment() {
        let picked = pick_ai_key(
            Some(String::from("sk-from-vault")),
            Some(String::from("sk-from-env")),
        );
        assert_eq!(picked.as_deref(), Some("sk-from-vault"));
    }

    #[test]
    fn the_environment_is_the_fallback() {
        let picked = pick_ai_key(None, Some(String::from("sk-from-env")));
        assert_eq!(picked.as_deref(), Some("sk-from-env"));
    }

    #[test]
    fn neither_source_yields_no_key() {
        assert!(pick_ai_key(None, None).is_none());
    }

    #[test]
    fn a_blank_value_counts_as_absent() {
        // A stray `GITGIT_AI_API_KEY=` must not become a bearer token of
        // `""`, and an emptied vault entry must not shadow a real one.
        assert!(pick_ai_key(Some(String::from("   ")), None).is_none());
        assert!(pick_ai_key(None, Some(String::from("\t"))).is_none());
        assert_eq!(
            pick_ai_key(Some(String::from("  ")), Some(String::from("sk-env"))).as_deref(),
            Some("sk-env"),
            "a blank vault entry falls through to the environment"
        );
    }

    #[test]
    fn a_stored_key_is_trimmed() {
        let picked = pick_ai_key(Some(String::from("  sk-key  \n")), None);
        assert_eq!(picked.as_deref(), Some("sk-key"));
    }
}
