# 开发环境就绪检查单 / Dev Environment Readiness Checklist

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 53 |
| 阶段 | 实现 — 开发环境构建 |
| 主要交付物 | 开发环境就绪 |
| 责任人 | 实施工程师 + SRE |
| 关联设计文档 | [`../../architecture/tech-selection.md`](../../../architecture/tech-selection.md) §13-§17 (强约束) · [`../../design/detailed-design/00-overview.md`](../../../design/detailed-design/00-overview.md) §0.11 (部署包) |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


## 环境矩阵 / Environment Matrix

| 角色 | OS | 工具链 | 验证命令 |
|---|---|---|---|
| Primary dev | Linux x86_64 / macOS aarch64 | Rust 1.75+ / sqlx-cli / cargo-chef | rustc --version |
| Windows dev | Windows 11 + WSL2 | 同上 | wsl rustc --version |
| CI runner | Linux x86_64 | 同上 + sccache | — |

## 本地就绪检查 / Local Readiness

- [ ] [TEMPLATE] Rust 工具链已安装: `rustup toolchain install 1.75.0`
- [ ] [TEMPLATE] sqlx-cli 已安装: `cargo install sqlx-cli --version 0.7.x`
- [ ] [TEMPLATE] PostgreSQL 15+ 已启动: `pg_isready`
- [ ] [TEMPLATE] cargo-deny 已安装: `cargo install cargo-deny`
- [ ] [TEMPLATE] cargo-chef 已安装: `cargo install cargo-chef`
- [ ] [TEMPLATE] pre-commit 已配置: `.pre-commit-config.yaml`
- [ ] [TEMPLATE] VSCode / Rust Analyzer 已配置 workspace
- [ ] [TEMPLATE] git hooks 已安装: `lefthook install`

## CI 就绪检查 / CI Readiness

- [ ] GitHub Actions / GitLab CI runner 已就位
- [ ] PostgreSQL service container 可用
- [ ] sccache 缓存已配置（CI 命中率达 80%+）
- [ ] testcontainers-rs 可拉取 Docker 镜像

## 已知环境差异 / Known Environment Differences

| 差异 | 影响 | 缓解 |
|---|---|---|
| Windows + WSL2 vs Linux | 文件 IO 慢 30% | 本地开发用 WSL2，CI 用 Linux |
| macOS aarch64 vs Linux x86_64 | gix 部分汇编路径不同 | CI 跑双平台 |

---

**导航 / Navigation:**
[← 流程总览](../../workflow.md) · [← 流程 README](../../README.md)
