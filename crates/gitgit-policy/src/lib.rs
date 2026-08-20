//! 策略引擎：RBAC+ABAC、AI 特有策略、决策模型。详细设计 §03-policy-engine.md
//!
//! 详细设计：见 `docs/design/detailed-design/` 对应章节
//! 实施前 QA：见 `docs/architecture/qa-checklist.md` 关联项

#![warn(missing_docs)]
#![allow(clippy::return_self_not_must_use)]

/// gitgit-policy 的版本号 (从 workspace 继承)
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// gitgit-policy 的占位类型 — 实际功能由后续 PR 填充
pub struct PolicyPlaceholder;

impl Default for PolicyPlaceholder {
    fn default() -> Self {
        Self
    }
}
