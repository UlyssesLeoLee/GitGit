# ADR-0009: SIEM 适配策略（wal2json → Kafka / Vector / 直接 webhook）

> **Status**: Accepted
> **Date**: 2026-08-19
> **Deciders**: SA + TL + EM
> **Consulted**: SEC + PO
> **Informed**: 全体

## Context and Problem Statement

admin_audit 需不可篡改 + 实时转发到 SIEM。考虑：直推 SIEM / Kafka 中转 / Vector 适配。

## Decision Drivers

- admin_audit 哈希链
- 实时性 ≤ 30s 延迟
- 适配多 SIEM (ELK / Splunk / Datadog)

## Considered Options

1. **直推 SIEM (webhook)**
2. **PostgreSQL wal2json → Kafka → SIEM 适配器**
3. **PostgreSQL wal2json → Vector → 多 SIEM (ELK/Splunk/Datadog)**

## Decision Outcome

**Chosen option**: "**MVP = Option A**（直推 webhook，SIEM 端负责持久化）；**V1+ = Option C**（Vector 适配多 SIEM）", because **MVP = Option A**（直推 webhook，SIEM 端负责持久化）；**V1+ = Option C**（Vector 适配多 SIEM）。MVP 不阻塞。

### Consequences

**Good:**
- [+] MVP 简单
- [+] V1+ 多 SIEM 适配
- [+] 审计日志链路可追溯

**Bad:**
- [-] MVP 阶段 SIEM 故障需重试
- [-] V1+ 增加 Vector 运维

### Confirmation

基本设计 §14.2.5 / 详细设计 §13.3 / 实施前 QA QA-005

## Pros and Cons of the Options

### 直推 SIEM (webhook)

[+] 简单
[-] SIEM 故障 → 数据积压 / 丢失

### PostgreSQL wal2json → Kafka → SIEM 适配器

[+] 解耦；高可靠；可重放
[-] 需运维 Kafka；增加组件

### PostgreSQL wal2json → Vector → 多 SIEM (ELK/Splunk/Datadog)

[+] (see chosen)
[-] (see chosen)


## References

[`../../design/basic-design/14-admin-ops-ui.md#1425-审计日志`](../../design/basic-design/14-admin-ops-ui.md#1425-审计日志) · [`../../design/detailed-design/13-admin-api-and-ops-ui.md#133-admin_audit-表-ddl`](../../design/detailed-design/13-admin-api-and-ops-ui.md#133-admin_audit-表-ddl)

## Revision History

| Date | Author | Change |
|---|---|---|
| YYYY-MM-DD | — | Initial draft |
| YYYY-MM-DD | — | Status → Accepted |
