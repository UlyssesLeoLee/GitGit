# 作业管理手册 / Job Management Handbook

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 111 |
| 阶段 | 运维 — 作业管理 (Job) |
| 主要交付物 | 作业管理报告 |
| 责任人 | SRE |
| 关联设计文档 | [`../../design/detailed-design/07-app-coordination.md`](../../../design/detailed-design/07-app-coordination.md) §7.4 (PL/pgSQL 调度) |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


## 作业清单 / Job Inventory

| 作业 ID | 名称 | 调度 | 超时 | 失败处理 |
|---|---|---|---|---|
| JOB-1 | [TEMPLATE] | Cron: 0 2 * * * | 30 min | 重试 3 次 + 告警 |

## 调度方式 / Scheduling


MVP 使用 PostgreSQL `pg_cron` 或 PL/pgSQL event-driven；详细见 [`../../design/detailed-design/07-app-coordination.md`](../../../design/detailed-design/07-app-coordination.md) §7.4。


## 运行检查 / Job Health Check

- [ ] 作业完成时间 ≤ 超时阈值
- [ ] 失败次数 ≤ 0 (last 24h)
- [ ] DLQ 长度 ≤ 0

---

**导航 / Navigation:**
[← 流程总览](../../workflow.md) · [← 流程 README](../../README.md)
