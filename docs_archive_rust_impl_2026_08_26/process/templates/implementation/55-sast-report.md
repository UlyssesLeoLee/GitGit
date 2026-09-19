# 静态分析报告 / SAST Report

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 55 |
| 阶段 | 实现 — 静态分析 (SAST) |
| 主要交付物 | SAST 报告 |
| 责任人 | 实施工程师 + SEC |
| 关联设计文档 | [`../../architecture/tech-selection.md`](../../../architecture/tech-selection.md) §15.1 (性能与安全) · clippy / cargo-audit / cargo-geiger |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


## 运行元数据 / Run Metadata

| 字段 | 值 |
|---|---|
| 运行日期 | YYYY-MM-DD |
| commit | git rev-parse HEAD |
| 工具 | clippy / cargo-audit / cargo-geiger / semgrep (Rust) |

## Clippy 结果

| 类别 | 数量 | 阻断阈值 |
|---|---|---|
| error | — | 0 (CI 阻断) |
| warn (pedantic) | — | ≤ 10 |
| warn (style) | — | ≤ 50 |

## Cargo Audit (CVE)

| 依赖 | CVE | 严重度 | 状态 |
|---|---|---|---|
| [TEMPLATE] | — | Critical/High/Medium/Low | Open/Fixed/Accepted |

## Cargo Geiger (unsafe 使用)


[TEMPLATE] `unsafe_code = "forbid"` 已写入 `[lints]` 段；如有使用须有 ADR 编号。


## Semgrep (Rust 规则)


[TEMPLATE] 例：sql-injection, hardcoded-secret, weak-crypto。


## 签核

| 角色 | 签核 | 日期 |
|---|---|---|
| 实施 | ☐ | — |
| SEC | ☐ | — |

---

**导航 / Navigation:**
[← 流程总览](../../workflow.md) · [← 流程 README](../../README.md)
