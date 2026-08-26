# ADR-0005: Admin 运维界面独立子进程 + 独立鉴权域

> **Status**: Accepted
> **Date**: 2026-08-19
> **Deciders**: SA + TL + EM
> **Consulted**: SEC + PO
> **Informed**: 全体

## Context and Problem Statement

Admin 操作 (升级 / KEK 轮换 / 用户角色变更) 风险高。需要：(a) 与终端用户鉴权完全分离 (b) 强审计 (c) 双因素。

## Decision Drivers

- SEC-REQ-011 鉴权域分离
- SEC-REQ-012 关键操作双因素
- admin_audit 不可篡改 (哈希链 + SIEM)

## Considered Options

1. **Admin 作为同一进程的 /admin/* 路由**
2. **Admin 独立子进程 + 不同 JWT issuer / 签名密钥 / RBAC 表**

## Decision Outcome

**Chosen option**: "Option B", because Option B。安全 > 复用。Admin 子进程 :3001，平台 :3000。

### Consequences

**Good:**
- [+] 鉴权域完全独立 (issuer / 签名密钥 / RBAC 表分离)
- [+] admin_audit 哈希链 + wal2json → SIEM
- [+] Admin 子进程可独立做安全审计

**Bad:**
- [-] 代码部分重复 (admin 也需要 user management)
- [-] 运维多 1 个进程

### Confirmation

基本设计 §14 (Admin Ops UI) / 详细设计 §13 (Admin API) / 实施前 QA QA-021

## Pros and Cons of the Options

### Admin 作为同一进程的 /admin/* 路由

[+] 代码共享；部署简单
[-] 同进程 → 鉴权域难完全分离；admin 漏洞 → 全平台失陷

### Admin 独立子进程 + 不同 JWT issuer / 签名密钥 / RBAC 表

[+] 进程级隔离；鉴权域完全独立；可独立扩缩容
[-] 代码部分重复；部署 2 个 binary


### Option C

[+] ...
[-] ...

## References

[`../../design/basic-design/14-admin-ops-ui.md`](../../design/basic-design/14-admin-ops-ui.md) · SEC-REQ-011 / 012

## Revision History

| Date | Author | Change |
|---|---|---|
| YYYY-MM-DD | — | Initial draft |
| YYYY-MM-DD | — | Status → Accepted |
