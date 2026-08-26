# ADR-0003: 采用 PostgreSQL 单一存储（不引入 Redis/NATS/Memgraph 等）

> **Status**: Accepted
> **Date**: 2026-08-19
> **Deciders**: SA + TL + EM
> **Consulted**: SEC + PO
> **Informed**: 全体

## Context and Problem Statement

需求：图谱存储 + 事件流 + 缓存 + 队列。考虑单一 PG vs 引入多组件。

## Decision Drivers

- 运维简单 (Local-First 自托管)
- 事务一致性 (图 + 事件原子写)
- 避免外部依赖 (K8s 部署复杂度)

## Considered Options

1. **PostgreSQL 单一存储 (含 LISTEN/NOTIFY + 物化视图)**
2. **PG + Redis + NATS + Memgraph**

## Decision Outcome

**Chosen option**: "Option A", because Option A。原则：**能 PG 解决的都用 PG**。图谱用 PG 递归 CTE 已满足 MVP；事件流用 LISTEN/NOTIFY + Outbox；缓存用 PG 物化视图。

### Consequences

**Good:**
- [+] 单一 DB 备份 (pg_dump)
- [+] Local-First 真正轻量
- [+] 事务保证事件不丢

**Bad:**
- [-] 图遍历深度 > 5 跳需手工优化
- [-] 高 QPS 场景需读副本

### Confirmation

tech-selection.md §6 (sqlx) / 详细设计 §01 (data layer) / 实施前 QA QA-002

## Pros and Cons of the Options

### PostgreSQL 单一存储 (含 LISTEN/NOTIFY + 物化视图)

[+] 一个进程；事务一致；备份简单；运维简单
[-] 图遍历性能可能不如专用图库

### PG + Redis + NATS + Memgraph

[+] 每组件最优
[-] 4 个组件运维；数据一致性难；Local-First 部署重


### Option C

[+] ...
[-] ...

## References

[`../tech-selection.md` §6](../tech-selection.md#6-postgresql-驱动sqlx-database-driver)

## Revision History

| Date | Author | Change |
|---|---|---|
| YYYY-MM-DD | — | Initial draft |
| YYYY-MM-DD | — | Status → Accepted |
