//! 图谱引擎：递归 CTE、类型注册表、视图、缓存。详细设计 §02-graph-engine.md
//!
//! 详细设计：见 `docs/design/detailed-design/` 对应章节
//! 实施前 QA：见 `docs/architecture/qa-checklist.md` 关联项

#![warn(missing_docs)]
#![allow(clippy::return_self_not_must_use)]

/// gitgit-graph 的版本号 (从 workspace 继承)
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// gitgit-graph 的占位类型 — 实际功能由后续 PR 填充
pub struct GraphPlaceholder;

impl Default for GraphPlaceholder {
    fn default() -> Self {
        Self
    }
}
