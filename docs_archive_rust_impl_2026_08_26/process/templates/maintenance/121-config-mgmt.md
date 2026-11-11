# 构成管理报告 / Configuration Management Report

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 121 |
| 阶段 | 维护 — 构成管理 (CM) |
| 主要交付物 | 构成管理报告 |
| 责任人 | SRE |
| 关联设计文档 | [`../../design/basic-design/00-introduction.md`](../../../design/basic-design/00-introduction.md) §0.6 (Git 引用) · [`../../architecture/tech-selection.md`](../../../architecture/tech-selection.md) |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


## 配置基线 / Config Baseline

| 项 | 版本 | 基线日期 |
|---|---|---|
| 源代码 | vX.Y.Z | — |
| 依赖 (Cargo.lock) | — | — |
| DB schema | migration #N | — |
| Helm chart | vX.Y | — |

## 变更跟踪 / Change Tracking


所有源代码变更在 git 跟踪；DB schema 变更在 migration 目录；Helm chart 变更在 git tag。


---

**导航 / Navigation:**
[← 流程总览](../../workflow.md) · [← 流程 README](../../README.md)
