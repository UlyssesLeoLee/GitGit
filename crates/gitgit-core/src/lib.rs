//! 核心领域类型：Node / Edge / Event / Policy / View 5 原语。详细设计 §01-data-layer §02-graph-engine
//!
//! 详细设计：见 `docs/design/detailed-design/` 对应章节
//! 实施前 QA：见 `docs/architecture/qa-checklist.md` 关联项

#![warn(missing_docs)]
#![allow(clippy::return_self_not_must_use)]

/// gitgit-core 的版本号 (从 workspace 继承)
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// gitgit-core 的占位类型 — 实际功能由后续 PR 填充
pub struct CorePlaceholder;

impl Default for CorePlaceholder {
    fn default() -> Self {
        Self
    }
}
