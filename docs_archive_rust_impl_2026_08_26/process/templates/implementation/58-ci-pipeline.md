# CI 流水线设计 / CI Pipeline Design

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 58 |
| 阶段 | 实现 — CI |
| 主要交付物 | CI 流水线 |
| 责任人 | SRE + 实施工程师 |
| 关联设计文档 | [`../../architecture/tech-selection.md`](../../../architecture/tech-selection.md) §16 (风险) · GitHub Actions / GitLab CI |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


## 流水线阶段 / Pipeline Stages

| 阶段 | 任务 | 超时 | 失败处理 |
|---|---|---|---|
| Lint | cargo fmt --check + cargo clippy | 5 min | 阻断 |
| Build | cargo build --workspace | 15 min (sccache) | 阻断 |
| Unit Test | cargo test --workspace | 10 min | 阻断 |
| Integration Test | cargo test --test it (testcontainers) | 20 min | 阻断 |
| SAST | cargo audit + semgrep | 5 min | High/Critical 阻断 |
| License Check | cargo deny | 2 min | GPL/AGPL 阻断 |
| Coverage | cargo llvm-cov | 5 min | < 80% 警告 |
| Docker Build | docker build | 10 min | 阻断 |

## 触发条件 / Triggers

| 触发 | 流水线 |
|---|---|
| PR opened / synchronized | Lint + Build + Unit Test + SAST + License |
| push to main | 全量 |
| nightly (cron) | 全量 + Performance Bench + Soak Test (短时) |
| release tag | 全量 + Release Build + Sign |

## 缓存策略 / Cache

- [ ] sccache 跨 PR 共享 cargo build 缓存
- [ ] testcontainers 镜像缓存
- [ ] Rust target/ 目录按 commit hash 缓存

## 通知 / Notification


PR 状态检查 + Slack #eng-ci 频道（失败时）。


---

**导航 / Navigation:**
[← 流程总览](../../workflow.md) · [← 流程 README](../../README.md)
