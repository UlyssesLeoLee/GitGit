# 系统监视 Runbook / Monitoring Runbook

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 110 |
| 阶段 | 运维 — 系统监视 |
| 主要交付物 | 监视报告 |
| 责任人 | SRE |
| 关联设计文档 | [`../../design/detailed-design/10-observability.md`](../../../design/detailed-design/10-observability.md) · OTel + Prometheus |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


## 监视对象 / Monitoring Targets

| 类别 | 对象 | 采集方式 |
|---|---|---|
| 应用 | HTTP API P50/P99 | OTel + Prometheus |
| 数据库 | PG 连接数 / 慢查询 | pg_exporter |
| 事件总线 | LISTEN/NOTIFY 队列深度 | 自定义 exporter |
| App 健康 | app_heartbeats stale | SQL 查询 |

## 告警规则 / Alert Rules

| 告警 ID | 条件 | 严重度 | 通知渠道 |
|---|---|---|---|
| ALT-1 | API P99 > 1s 持续 5min | High | Slack #ops |
| ALT-2 | DB 连接数 > 80% | High | Slack #ops |
| ALT-3 | App instance stale > 90s | Critical | PagerDuty |
| ALT-4 | 事件 DLQ 增长 > 100/min | High | Slack #ops |

## 日常巡检 / Daily Check

- [ ] Grafana 主面板无异常
- [ ] 昨日事件数 vs 上周同日 ±20%
- [ ] 无新告警
- [ ] 备份成功

---

**导航 / Navigation:**
[← 流程总览](../../workflow.md) · [← 流程 README](../../README.md)
