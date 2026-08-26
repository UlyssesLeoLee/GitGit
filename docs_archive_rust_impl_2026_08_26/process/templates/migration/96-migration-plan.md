# 迁移计划书 / Migration Plan

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 96 |
| 阶段 | 迁移 — 迁移计划 |
| 主要交付物 | 迁移计划书 |
| 责任人 | PM + DBA + SRE |
| 关联设计文档 | [`../../design/basic-design/09-migration-design.md`](../../../design/basic-design/09-migration-design.md) (Local → Cloud 迁移设计) |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


## 迁移场景 / Migration Scenario

| 场景 | 描述 | 源 | 目标 |
|---|---|---|---|
| L → C (MVP) | 本地自托管 → 云托管 | Local PG + 本地 Git | Cloud PG + Cloud Git |
| C → L (退云) | 云 → 本地 | Cloud | Local |
| L → L (跨环境) | 开发 → 测试 → 生产 | Local A | Local B |

## 迁移策略 / Strategy

- [ ] 导出：PostgreSQL pg_dump (custom format)
- [ ] Git 仓库：git bundle
- [ ] Secrets：envelope-encrypted export bundle
- [ ] 图数据：与应用 schema 一同 dump
- [ ] 传输：加密 (TLS) + 完整性校验 (SHA-256)

## 时间表 / Timeline

| 里程碑 | 日期 | 依赖 |
|---|---|---|
| 演练 (Rehearsal) | T-7d | 测试环境就绪 |
| 数据迁移 dry run | T-3d | — |
| 实际数据迁移 | T-1d | 应用停机 |
| 系统切换 | T-0 | 迁移演练通过 |
| 上线确认 | T+1d | — |

## 回滚方案 / Rollback


RTO ≤ 1h, RPO ≤ 15min。回滚条件：迁移后 24h 内 P0/P1 缺陷 ≥ 3 项。


## 签核

| 角色 | 签核 | 日期 |
|---|---|---|
| PM | ☐ | — |
| DBA | ☐ | — |
| SRE | ☐ | — |
| PO | ☐ | — |

---

**导航 / Navigation:**
[← 流程总览](../../workflow.md) · [← 流程 README](../../README.md)
