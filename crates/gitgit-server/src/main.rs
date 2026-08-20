//! gitgit-server binary entrypoint
//!
//! 详细设计：见 `docs/design/detailed-design/` 对应章节
//! 启动流程：config 加载 → observability 初始化 → DB 迁移 → 服务启动 → graceful shutdown

use anyhow::Context;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    info!("starting gitgit-server");
    gitgit_config::load()
        .context("failed to load configuration")?;
    gitgit_observability::init()
        .context("failed to initialize observability")?;
    // TODO: DB migration, server start, signal handling
    info!("gitgit-server exited cleanly");
    Ok(())
}
