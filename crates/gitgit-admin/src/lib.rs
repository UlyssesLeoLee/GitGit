//! Admin 独立子进程：鉴权域分离 + admin_audit。详细设计 §13-admin-api-and-ops-ui.md + SEC-REQ-011
//!
//! 详细设计：见 `docs/design/detailed-design/` 对应章节
//! 实施前 QA：见 `docs/architecture/qa-checklist.md` 关联项

#![warn(missing_docs)]
#![allow(clippy::return_self_not_must_use)]

/// gitgit-admin 的版本号 (从 workspace 继承)
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// gitgit-admin 的占位类型 — 实际功能由后续 PR 填充
pub struct AdminPlaceholder;

impl Default for AdminPlaceholder {
    fn default() -> Self {
        Self
    }
}
