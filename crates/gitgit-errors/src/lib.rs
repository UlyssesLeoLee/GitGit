//! 标准化错误类型 (thiserror)。详细设计 §11-error-handling.md
//!
//! 标准化错误类型 + ErrorCode 枚举 + HTTP 状态码映射
//!
//! 详细设计：`docs/design/detailed-design/11-error-handling.md`
//! 实施前 QA：见 `docs/architecture/qa-checklist.md` 关联项

#![warn(missing_docs)]
#![allow(clippy::return_self_not_must_use)]

/// gitgit-errors 的版本号 (从 workspace 继承)
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// gitgit-errors 的占位类型 — 实际功能由后续 PR 填充
pub struct ErrorsPlaceholder;

impl Default for ErrorsPlaceholder {
    fn default() -> Self {
        Self
    }
}


pub mod error;
pub use error::{AppError, AppResult, ErrorCode};
