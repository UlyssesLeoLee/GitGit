# ADR-0014: WebAuthn 凭证库选型 (V1+ Cloud Admin 双因素)

> **Status**: Proposed
> **Date**: 2026-08-23
> **Deciders**: SA + TL + EM
> **Consulted**: SEC + PO
> **Informed**: 全部

## Context and Problem Statement

[基本设计 §14.3.3](../../design/basic-design/14-admin-ops-ui.md) 与 [§7.5 密钥管理](../../design/basic-design/07-security-design.md#75-密钥管理-secrets-managementsec-req-005-sec-req-010) 提到 V1+ Cloud Admin 远程访问需要双因素(2FA)。技术选型 [§8](../../architecture/tech-selection.md#8-认证与凭证) 选 `jsonwebtoken` + `argon2`,但未涉及 WebAuthn / FIDO2 凭证。

候选方案:

- 自实现 WebAuthn 注册/认证 (~500 行 Rust,基于 `ring` 或 `RustCrypto`)
- 使用 `webauthn-rs` crate(成熟,~3000 stars,WASM 友好)
- 第三方 SaaS(如 Auth0 / Clerk)— 引入外部依赖,违反"能标准协议不发明"原则

## Decision Drivers

- [DRIVER-1] V1+ Cloud Admin 双因素必须支持 FIDO2 / WebAuthn(OWASP 推荐)
- [DRIVER-2] 凭证(私钥)只能由本地硬件(YubiKey / TouchID / Windows Hello)保存,服务端永远不存明文
- [DRIVER-3] 与 [ADR-0017 Vault 集成](#) 协调:challenge / attestation 可放 PostgreSQL,Vault 仅存派生密钥
- [DRIVER-4] 依赖最小化原则(技术选型 §15 强约束)

## Considered Options

1. **Option A — 使用 `webauthn-rs` crate** (推荐)
2. **Option B — 自实现**
3. **Option C — 第三方 SaaS (Auth0 / Clerk)**

## Decision Outcome

**Chosen option**: "Option A — 使用 webauthn-rs crate", because 库活跃维护、覆盖 WebAuthn Level 2+ 标准、依赖可控(仅 `ring` + `base64` + `url`)。自实现 500 行很难保证协议所有 corner case 正确。

### 具体配置

| 维度 | 选型 | 备注 |
|---|---|---|
| 库 | `webauthn-rs` 0.3+ (latest stable) | 配合 `tracing` 记录注册/认证事件 |
| Attestation | `None` (基础);V1+ 启用 `Direct` 用于 YubiKey 验证 | 与 [§7.5 密钥管理](../../design/basic-design/07-security-design.md#75-密钥管理-secrets-managementsec-req-005-sec-req-010) 一致 |
| User verification | `Preferred` → 强制 | 安全等级递进 |
| 凭证存储 | PostgreSQL `webauthn_credentials` 表(密文 + credential_id) | 不存私钥(私钥在硬件) |
| Challenge TTL | 60 秒,一次性 | 防 replay |
| Rate limit | 注册 1 次/天 per user;认证 5 次失败锁定 15 分钟 | 与 SSH 锁定策略一致([详设 §6.9.2](../../design/detailed-design/06-git-server.md)) |

### Consequences

**Good:**
- [+] 协议正确性由库保证;省下协议测试成本
- [+] 与 YubiKey / TouchID / Windows Hello 开箱即用
- [+] V1+ Cloud Admin 远程访问满足 SEC-REQ-007 强认证要求

**Bad:**
- [-] 引入新 crate(增加二进制 ~1MB);需评估 [REQ-TS-019 依赖审计] 合规
- [-] `webauthn-rs` API 仍在演进,需锁版本(0.3.x 期间不自动升级)

### Confirmation

[CONFIRM] 详设 §06 SSH 部分 + [specs/test-specification.md §7.5 安全测试](../../specs/test-specification.md#7-安全设计测试):
- TC-AUTH-010: WebAuthn 注册流程 happy path
- TC-AUTH-011: 凭证私钥不在服务端任何日志/表中出现(grep 验证)
- TC-AUTH-012: 过期 challenge 拒绝
- TC-AUTH-013: 失败 5 次锁定
- TC-AUTH-014: 与 YubiKey 5C NFC 实际验证(物理设备)

## Pros and Cons of the Options

### Option A — `webauthn-rs` crate
[+] 协议正确;活跃维护;依赖可控
[-] 引入新依赖;API 演进中

### Option B — 自实现
[+] 依赖最小;可控
[-] WebAuthn CBOR / COSE / signature 验证细节多;测试成本高;协议 bug 风险高

### Option C — 第三方 SaaS
[+] 零实现
[-] 引入外部依赖;违反"Local-First"与"凭证不离本机"原则;数据出境风险

## References

- [技术选型 §8 认证与凭证](../tech-selection.md#8-认证与凭证)
- [基本设计 §7.5 密钥管理](../../design/basic-design/07-security-design.md#75-密钥管理-secrets-managementsec-req-005-sec-req-010)
- [基本设计 §14.3.3 Admin 鉴权域](../../design/basic-design/14-admin-ops-ui.md)
- [ADR-0017: HashiCorp Vault 集成细节](#)
- [OWASP Cheat Sheet — Web Authentication](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html)
- [W3C WebAuthn Level 2 Spec](https://www.w3.org/TR/webauthn-2/)

## Revision History

| Date | Author | Change |
|---|---|---|
| 2026-08-23 | Mavis (AI 起草) | Initial draft (Proposed) |
| YYYY-MM-DD | — | Status → Accepted (待 SA + TL + EM + SEC 签核) |
