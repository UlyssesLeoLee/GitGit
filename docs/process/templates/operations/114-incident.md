# 事件响应 Runbook / Incident Response Runbook

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 114 |
| 阶段 | 运维 — 事件管理 (Incident) |
| 主要交付物 | 事件报告 |
| 责任人 | SRE |
| 关联设计文档 | [`./115-fault.md`](./115-fault.md) · [`../../design/basic-design/14-admin-ops-ui.md`](../../../design/basic-design/14-admin-ops-ui.md) §14.2.3 |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


## 事件分类 / Incident Classification

| 级别 | 定义 | 响应时间 | 升级 |
|---|---|---|---|
| P0 | 全站不可用 | ≤ 5min | 全体 |
| P1 | 核心功能受损 | ≤ 15min | EM |
| P2 | 次要功能受损 | ≤ 1h | TL |
| P3 | 一般问题 | ≤ 1 工作日 | — |

## 响应流程 / Response Process

| 步骤 | 动作 | 负责人 |
|---|---|---|
| 1. 检测 | 告警 / 用户报告 | SRE on-call |
| 2. 定级 | Severity + 影响范围 | SRE on-call |
| 3. 通知 | Slack + PagerDuty | SRE on-call |
| 4. 缓解 | 回滚 / 限流 / 隔离 | SRE + EM |
| 5. 修复 | 代码 / 配置 | 实施 |
| 6. 复盘 | Post-mortem (5 工作日内) | EM |

## 事件记录 / Incident Record

| 字段 | 值 |
|---|---|
| INC ID | INC-NNN |
| 时间 | — |
| 级别 | P0/P1/P2/P3 |
| 影响 | — |
| MTTR | — |
| 根因 | — |
| 措施 | — |

---

**导航 / Navigation:**
[← 流程总览](../../workflow.md) · [← 流程 README](../../README.md)
