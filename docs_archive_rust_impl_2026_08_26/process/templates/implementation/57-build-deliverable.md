# 构建交付物清单 / Build Deliverable Manifest

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 57 |
| 阶段 | 实现 — 构建 (Build) |
| 主要交付物 | 构建产物 |
| 责任人 | 实施工程师 + SRE |
| 关联设计文档 | [`../../architecture/tech-selection.md`](../../../architecture/tech-selection.md) §17 (验收标准) · Cargo workspace |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


## 构建元数据 / Build Metadata

| 字段 | 值 |
|---|---|
| 构建日期 | YYYY-MM-DD |
| commit | git rev-parse HEAD |
| Rust 工具链 | rustc 1.75.0 |
| profile | release / debug |
| 目标平台 | x86_64-unknown-linux-gnu / aarch64-apple-darwin / ... |

## 构建产物 / Artifacts

| crate | 二进制 | 大小 (stripped) | SHA-256 |
|---|---|---|---|
| gitgit-core | — | — | — |
| gitgit-server | — | — | — |
| gitgit-admin | — | — | — |
| gitgit-cli | — | — | — |

## 构建性能 / Build Performance

| 指标 | 目标 | 实测 |
|---|---|---|
| 首次构建 | ≤ 30 min (含 deps) | — |
| 增量构建 (cargo-chef) | ≤ 5 min | — |
| CI 缓存命中率 (sccache) | ≥ 80% | — |

## 已知警告 / Known Warnings


[TEMPLATE] 例：1 个 deprecation warning 来自 sqlx 0.7.x，等待 0.8 升级。


---

**导航 / Navigation:**
[← 流程总览](../../workflow.md) · [← 流程 README](../../README.md)
