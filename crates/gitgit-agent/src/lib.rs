//! Agent 运行时：状态机、工作区隔离、凭证签发。详细设计 §04-agent-runtime.md
//!
//! 详细设计：见 `docs/design/detailed-design/` 对应章节
//! 实施前 QA：见 `docs/architecture/qa-checklist.md` 关联项

#![warn(missing_docs)]
#![allow(clippy::return_self_not_must_use)]

/// gitgit-agent 的版本号 (从 workspace 继承)
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// gitgit-agent 的占位类型 — 实际功能由后续 PR 填充
pub struct AgentPlaceholder;

impl Default for AgentPlaceholder {
    fn default() -> Self {
        Self
    }
}
