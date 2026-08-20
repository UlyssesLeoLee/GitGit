# 容量管理计划 / Capacity Management Plan

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 113 |
| 阶段 | 运维 — 容量管理 (Capacity) |
| 主要交付物 | 容量管理报告 |
| 责任人 | SRE |
| 关联设计文档 | [`../../design/basic-design/08-operations-design.md`](../../../design/basic-design/08-operations-design.md) §8.3 · NFR-REQ-002 |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


## 当前容量 / Current Capacity

| 资源 | 当前使用 | 总量 | 利用率 |
|---|---|---|---|
| CPU | — | — | —% |
| Memory | — | — | —% |
| Disk | — | — | —% |
| DB connections | — | — | —% |

## 增长预测 / Growth Forecast


[TEMPLATE] 例：6 个月内仓库数预计增长 3 倍。


## 扩容触发 / Scale-out Triggers

| 指标 | 阈值 | 动作 |
|---|---|---|
| CPU > 70% 持续 1h | 扩容 + 1 节点 | kubectl scale |
| DB connections > 80% | 扩容 pg_bouncer | 调整 HPA |
| Disk > 80% | 扩容 PV | 人工 + 告警 |

## 容量规划 / Capacity Plan


[TEMPLATE] 例：本季度不需扩容；下季度需要 DB 从 4 vCPU 升 8 vCPU。


---

**导航 / Navigation:**
[← 流程总览](../../workflow.md) · [← 流程 README](../../README.md)
