# 运维交接书 / Operations Handover

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 109 |
| 阶段 | 运维 — 运维交接 |
| 主要交付物 | 运维交接书 |
| 责任人 | 实施 + SRE |
| 关联设计文档 | [`../../design/basic-design/08-operations-design.md`](../../../design/basic-design/08-operations-design.md) §8.5 |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


## 交接清单 / Handover Checklist

- [ ] 系统架构图 + 部署拓扑
- [ ] 凭据与密钥（已轮换）
- [ ] 监控面板 (Grafana)
- [ ] 告警规则 (Prometheus)
- [ ] Runbook（见 [`./110-monitoring.md`](./110-monitoring.md) 等）
- [ ] 事故响应流程
- [ ] 变更管理流程
- [ ] 应急联系人清单

## SLA 承诺 / SLA Commitment

| 指标 | 目标 |
|---|---|
| 可用性 | ≥ 99.5% |
| 响应时间 P99 | ≤ 500ms |
| 事件响应 P1 | ≤ 15min |

## 培训 / Training

- [ ] [TEMPLATE] 运维团队 1 天培训 + Q&A

## 签核

| 角色 | 签核 |
|---|---|
| 实施负责人 | ☐ |
| SRE 负责人 | ☐ |

---

**导航 / Navigation:**
[← 流程总览](../../workflow.md) · [← 流程 README](../../README.md)
