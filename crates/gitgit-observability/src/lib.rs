//! OTel / Prometheus / tracing 初始化与中间件。详细设计 §10-observability.md
//!
//! 详细设计：见 `docs/design/detailed-design/` 对应章节
//! 实施前 QA：见 `docs/architecture/qa-checklist.md` 关联项

#![warn(missing_docs)]
#![allow(clippy::return_self_not_must_use)]

/// gitgit-observability 的版本号 (从 workspace 继承)
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// gitgit-observability 的占位类型 — 实际功能由后续 PR 填充
pub struct ObservabilityPlaceholder;

impl Default for ObservabilityPlaceholder {
    fn default() -> Self {
        Self
    }
}

/// 初始化可观测性 (MVP stub)
///
/// MVP: 啥也不做, 仅作占位让 main.rs 编译通过
/// V0.5: tracing_subscriber fmt layer + EnvFilter (RUST_LOG)
/// V1+: OTel Collector (Sidecar → DaemonSet, 详设 §10 + ADR-0016)
pub fn init() -> Result<(), anyhow::Error> {
    // MVP 占位: 不初始化任何 subscriber
    // 实际实现 (V0.5+):
    //   tracing_subscriber::registry()
    //       .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
    //       .with(tracing_subscriber::fmt::layer().json())
    //       .init();
    Ok(())
}
