# 紧急改修 Runbook / Hotfix Runbook

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 125 |
| 阶段 | 维护 — 紧急改修 (Hotfix) |
| 主要交付物 | 紧急改修报告 |
| 责任人 | SRE + 实施 + EM |
| 关联设计文档 | [`./124-maintenance.md`](./124-maintenance.md) · [`../operations/114-incident.md`](../operations/114-incident.md) |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


## Hotfix 触发条件 / Trigger Conditions

- [ ] P0/P1 事件中已有明确最小修复
- [ ] 完整发布周期 (任务 102-107) 不可接受
- [ ] EM + SRE + SEC 三方共识

## 流程 / Process (Fast-track)

| 步骤 | 动作 | 目标时间 |
|---|---|---|
| 1. 创建 hotfix 分支 | git checkout -b hotfix/X.Y.Z+1 | 10 min |
| 2. 最小修复 | 代码 + 单测 | 1-4 h |
| 3. 紧急 CR | 1 名 TL + 1 名 SRE 评审 | 30 min |
| 4. 紧急 CI | 全量 | 15 min |
| 5. 部署到 staging | kubectl apply | 10 min |
| 6. Smoke test | — | 10 min |
| 7. 部署到生产 | — | 10 min |
| 8. 监控 + Hypercare 延长 | — | 24-48 h |

## 事后 / Post-Hotfix

- [ ] 1 周内补完整发布流程
- [ ] Post-mortem 走任务 148 复盘

---

**导航 / Navigation:**
[← 流程总览](../../workflow.md) · [← 流程 README](../../README.md)
