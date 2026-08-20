# ADR-0007: 密钥管理用 Vault / KMS（不在 DB 直接存明文）

> **Status**: Accepted
> **Date**: 2026-08-19
> **Deciders**: SA + TL + EM
> **Consulted**: SEC + PO
> **Informed**: 全体

## Context and Problem Statement

SE-REQ-005 / 010 要求：KEK/DEK 信封加密，硬件 / KMS 集成。考虑 Vault / AWS KMS / 自建。

## Decision Drivers

- SEC-REQ-005 三层信封加密
- SEC-REQ-010 KEK 轮换
- Local-First 离线可用

## Considered Options

1. **AWS KMS 强绑定**
2. **HashiCorp Vault (Transit) + 本地 file-based fallback**
3. **PostgreSQL pgcrypto + 文件 fallback (MVP)**

## Decision Outcome

**Chosen option**: "**MVP = Option C**（无新组件，pgcrypto + 文件）；**V1+ = Option B**（Vault Transit，可选 KMS）", because **MVP = Option C**（无新组件，pgcrypto + 文件）；**V1+ = Option B**（Vault Transit，可选 KMS）。MVP 不阻塞，待 V1 用户规模起来后升级。

### Consequences

**Good:**
- [+] MVP 简单
- [+] V1+ 平滑升级路径
- [+] 信封加密满足 SEC-REQ-005

**Bad:**
- [-] MVP 阶段 KEK 轮换需人工
- [-] 用户需自管主密码

### Confirmation

tech-selection.md §9 / 详细设计 §09 (security-impl) / 实施前 QA QA-005

## Pros and Cons of the Options

### AWS KMS 强绑定

[+] 硬件级 HSM
[-] 云锁定；Local-First 不可用

### HashiCorp Vault (Transit) + 本地 file-based fallback

[+] 本地 / 云都可用；Transit 引擎统一接口
[-] 运维 Vault 自身

### PostgreSQL pgcrypto + 文件 fallback (MVP)

[+] (see chosen)
[-] (see chosen)


## References

[`../tech-selection.md` §9](../tech-selection.md#9-加密原语rustcrypto-cryptography) · SEC-REQ-005 / 010

## Revision History

| Date | Author | Change |
|---|---|---|
| YYYY-MM-DD | — | Initial draft |
| YYYY-MM-DD | — | Status → Accepted |
