//! 主服务 binary：HTTP API + gRPC + 平台进程。详细设计 §08-api-handlers.md + §13-admin-api-and-ops-ui.md
//!
//! 详细设计：见 `docs/design/detailed-design/` 对应章节
//! 实施前 QA：见 `docs/architecture/qa-checklist.md` 关联项

#![warn(missing_docs)]
#![allow(clippy::return_self_not_must_use)]

/// gitgit-server 的版本号 (从 workspace 继承)
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// gitgit-server 的占位类型 — 实际功能由后续 PR 填充
pub struct ServerPlaceholder;

impl Default for ServerPlaceholder {
    fn default() -> Self {
        Self
    }
}
