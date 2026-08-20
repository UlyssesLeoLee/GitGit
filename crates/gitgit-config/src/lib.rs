//! figment 配置 schema 与加载逻辑。详细设计 §12-app-registry §13-admin
//!
//! 详细设计：见 `docs/design/detailed-design/` 对应章节
//! 实施前 QA：见 `docs/architecture/qa-checklist.md` 关联项

#![warn(missing_docs)]
#![allow(clippy::return_self_not_must_use)]

/// gitgit-config 的版本号 (从 workspace 继承)
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// gitgit-config 的占位类型 — 实际功能由后续 PR 填充
pub struct ConfigPlaceholder;

impl Default for ConfigPlaceholder {
    fn default() -> Self {
        Self
    }
}
