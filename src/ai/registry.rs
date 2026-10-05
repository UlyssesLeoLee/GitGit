//! Provider registry (archived design §5.2 `provider/registry.go`).
//!
//! # Why five presets over two adapters
//!
//! `docs/plan/v0-tasks.md` T7 sizes this as "5 个 provider". The archived
//! design names three adapter files (`openai.go`, `anthropic.go`,
//! `ollama.go`) and states at §5.5 that Ollama reuses the OpenAI-compatible
//! client. The two counts are not in conflict — they count different
//! things:
//!
//! - **wire protocols implemented: 2** (OpenAI-compatible, Anthropic).
//! - **named, ready-to-use presets registered: 5**.
//!
//! Every OpenAI-compatible vendor is a preset differing only in base URL
//! and default model, so a fifth *adapter* would be a copy of the first
//! with a different string. Counting them as separate adapters would
//! overstate what was built. The registry is the honest place to express
//! them, and `--ai-base-url` covers the long tail of hosts that have no
//! preset.
//!
//! Presets, all real endpoints with published APIs:
//!
//! | key          | protocol  | base URL                        |
//! |--------------|-----------|---------------------------------|
//! | `openai`     | openai    | `https://api.openai.com/v1`     |
//! | `deepseek`   | openai    | `https://api.deepseek.com/v1`   |
//! | `ollama`     | openai    | `http://localhost:11434/v1`     |
//! | `vllm`       | openai    | `http://localhost:8000/v1`      |
//! | `anthropic`  | anthropic | `https://api.anthropic.com/v1`  |

use std::collections::BTreeMap;

use crate::ai::anthropic::AnthropicProvider;
use crate::ai::openai::OpenAiProvider;
use crate::ai::provider::AiProvider;
use crate::error::{GitGitError, Result};

/// Models advertised per preset. Not exhaustive — any model the endpoint
/// accepts may be passed with `--ai-model`.
const OPENAI_MODELS: &[&str] = &["gpt-4o", "gpt-4o-mini", "gpt-4.1", "o4-mini"];
const DEEPSEEK_MODELS: &[&str] = &["deepseek-chat", "deepseek-reasoner"];
const OLLAMA_MODELS: &[&str] = &["llama3.1", "qwen2.5", "codellama"];
const VLLM_MODELS: &[&str] = &["local-model"];
const ANTHROPIC_MODELS: &[&str] = &["claude-sonnet-4-20250514", "claude-opus-4-20250514"];

/// One selectable provider, before an API key is attached.
#[derive(Debug, Clone)]
pub enum ProviderSpec {
    OpenAi {
        base_url: String,
        default_model: String,
        models: &'static [&'static str],
    },
    Anthropic {
        base_url: String,
        default_model: String,
        models: &'static [&'static str],
    },
}

impl ProviderSpec {
    /// Instantiate the spec into a live provider.
    ///
    /// No credential is attached here: [`crate::ai::provider::AiProvider::send`]
    /// takes the key per call, so a rotated key does not require a rebuild.
    pub fn build(&self, name: &str) -> Box<dyn AiProvider> {
        match self {
            ProviderSpec::OpenAi {
                base_url, models, ..
            } => Box::new(OpenAiProvider::new(name, base_url.clone(), models)),
            ProviderSpec::Anthropic {
                base_url, models, ..
            } => Box::new(AnthropicProvider::new(name, base_url.clone(), models)),
        }
    }

    /// The model used when the caller does not pass `--ai-model`.
    pub fn default_model(&self) -> &str {
        match self {
            ProviderSpec::OpenAi { default_model, .. }
            | ProviderSpec::Anthropic { default_model, .. } => default_model,
        }
    }

    /// The endpoint this spec targets, for `--dry-run` reporting.
    pub fn base_url(&self) -> &str {
        match self {
            ProviderSpec::OpenAi { base_url, .. } | ProviderSpec::Anthropic { base_url, .. } => {
                base_url
            }
        }
    }

    /// Whether this spec's protocol has a streaming implementation.
    ///
    /// Lets a caller refuse a streaming request *before* a network round
    /// trip, with the reason, instead of discovering it at call time.
    pub fn supports_streaming(&self) -> bool {
        match self {
            ProviderSpec::OpenAi { .. } => true,
            ProviderSpec::Anthropic { .. } => false,
        }
    }

    /// A copy with `base_url` replaced, for `--ai-base-url`.
    pub fn with_base_url(&self, base_url: &str) -> ProviderSpec {
        let base_url = base_url.trim_end_matches('/').to_string();
        match self {
            ProviderSpec::OpenAi {
                default_model,
                models,
                ..
            } => ProviderSpec::OpenAi {
                base_url,
                default_model: default_model.clone(),
                models,
            },
            ProviderSpec::Anthropic {
                default_model,
                models,
                ..
            } => ProviderSpec::Anthropic {
                base_url,
                default_model: default_model.clone(),
                models,
            },
        }
    }
}

/// The built-in preset table.
fn builtin_specs() -> BTreeMap<&'static str, ProviderSpec> {
    let mut m = BTreeMap::new();
    m.insert(
        "anthropic",
        ProviderSpec::Anthropic {
            base_url: "https://api.anthropic.com/v1".to_string(),
            default_model: ANTHROPIC_MODELS[0].to_string(),
            models: ANTHROPIC_MODELS,
        },
    );
    m.insert(
        "deepseek",
        ProviderSpec::OpenAi {
            base_url: "https://api.deepseek.com/v1".to_string(),
            default_model: DEEPSEEK_MODELS[0].to_string(),
            models: DEEPSEEK_MODELS,
        },
    );
    m.insert(
        "ollama",
        ProviderSpec::OpenAi {
            base_url: "http://localhost:11434/v1".to_string(),
            default_model: OLLAMA_MODELS[0].to_string(),
            models: OLLAMA_MODELS,
        },
    );
    m.insert(
        "openai",
        ProviderSpec::OpenAi {
            base_url: "https://api.openai.com/v1".to_string(),
            default_model: OPENAI_MODELS[0].to_string(),
            models: OPENAI_MODELS,
        },
    );
    m.insert(
        "vllm",
        ProviderSpec::OpenAi {
            base_url: "http://localhost:8000/v1".to_string(),
            default_model: VLLM_MODELS[0].to_string(),
            models: VLLM_MODELS,
        },
    );
    m
}

/// A named set of providers, resolvable by key.
///
/// `BTreeMap` rather than `HashMap` so `gitai providers` lists keys in a
/// stable order; a command whose output reshuffles between runs is hard to
/// diff in a test or a bug report.
#[derive(Debug, Clone)]
pub struct ProviderRegistry {
    specs: BTreeMap<String, ProviderSpec>,
}

impl ProviderRegistry {
    /// The registry of built-in presets.
    pub fn builtin() -> Self {
        Self {
            specs: builtin_specs()
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        }
    }

    /// Look up a preset by key.
    pub fn get(&self, key: &str) -> Result<&ProviderSpec> {
        self.specs.get(key).ok_or_else(|| {
            let known = self.keys().join(", ");
            GitGitError::Ai(format!(
                "unknown ai provider `{key}`; known providers: {known}"
            ))
        })
    }

    /// Every registered key, sorted.
    pub fn keys(&self) -> Vec<&str> {
        self.specs.keys().map(String::as_str).collect()
    }

    pub fn len(&self) -> usize {
        self.specs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.specs.is_empty()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::builtin()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn five_presets_are_registered() {
        // T7 is sized at 5 providers. This pins the count so a silent
        // removal is caught rather than discovered at runtime.
        assert_eq!(ProviderRegistry::builtin().len(), 5);
    }

    #[test]
    fn keys_are_sorted_and_stable() {
        let reg = ProviderRegistry::builtin();
        let keys = reg.keys();
        assert_eq!(
            keys,
            vec!["anthropic", "deepseek", "ollama", "openai", "vllm"]
        );
    }

    #[test]
    fn every_preset_is_constructible() {
        // Each preset must instantiate into some adapter. Guards against a
        // variant arm that builds nothing.
        for key in ProviderRegistry::builtin().keys() {
            let reg = ProviderRegistry::builtin();
            let spec = reg.get(key).unwrap();
            let p = spec.build(key);
            assert_eq!(p.name(), key);
            assert!(!p.capabilities().models.is_empty(), "{key} has no models");
        }
    }

    #[test]
    fn unknown_key_lists_the_known_ones() {
        // The error has to be actionable: a bare "not found" sends the
        // operator to the docs when the answer is on the same line.
        let err = ProviderRegistry::builtin().get("nope").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("nope"));
        assert!(msg.contains("openai"), "got: {msg}");
    }

    #[test]
    fn base_url_override_replaces_and_normalizes() {
        let reg = ProviderRegistry::builtin();
        let spec = reg.get("openai").unwrap();
        let overridden = spec.with_base_url("https://proxy.internal/v1/");
        match overridden {
            ProviderSpec::OpenAi { base_url, .. } => {
                assert_eq!(base_url, "https://proxy.internal/v1")
            }
            other => panic!("variant changed under override: {other:?}"),
        }
    }

    #[test]
    fn local_presets_point_at_loopback() {
        // ollama and vllm are local runtimes; a typo here would send a
        // developer's diff to a public host.
        let reg = ProviderRegistry::builtin();
        for key in ["ollama", "vllm"] {
            let spec = reg.get(key).unwrap();
            let p = spec.build(key);
            assert!(
                p.base_url().contains("localhost"),
                "{key} points at {}",
                p.base_url()
            );
        }
    }
}
