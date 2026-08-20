# 备份恢复 Runbook / Backup & Restore Runbook

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 112 |
| 阶段 | 运维 — 备份 |
| 主要交付物 | 备份结果 |
| 责任人 | DBA + SRE |
| 关联设计文档 | [`../../design/basic-design/08-operations-design.md`](../../../design/basic-design/08-operations-design.md) §8.2 · BKP-REQ-001/002/003 |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


## 备份策略 / Backup Strategy

| 类型 | 频率 | 保留 | 存储 |
|---|---|---|---|
| Full backup (pg_basebackup) | Daily 02:00 | 30 天 | S3 |
| WAL archive | Continuous | 7 天 | S3 |
| Git bundle | Daily 03:00 | 30 天 | S3 |
| Secrets (encrypted) | Daily 03:30 | 30 天 | S3 |

## RTO / RPO

| 指标 | 目标 | 实测 (每季演练) |
|---|---|---|
| RTO | ≤ 1h | — |
| RPO | ≤ 15min | — |

## 恢复步骤 / Restore Procedure

| 步骤 | 命令 | 预计耗时 |
|---|---|---|
| 1. 启动临时 PG | pg_createcluster | 5 min |
| 2. 恢复 base backup | pg_restore | 20 min |
| 3. Apply WAL | recovery.conf | 10 min |
| 4. 验证 | psql + checksum | 10 min |

## 季度演练 / Quarterly Drill

- [ ] 从最近一次 backup 实际恢复
- [ ] 记录 RTO 实测
- [ ] 更新 Runbook

---

**导航 / Navigation:**
[← 流程总览](../../workflow.md) · [← 流程 README](../../README.md)
