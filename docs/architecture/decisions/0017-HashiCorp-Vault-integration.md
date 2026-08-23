# ADR-0017: HashiCorp Vault 集成细节 (MVP 本地文件 / V1+ Vault KV v2)

> **Status**: Proposed
> **Date**: 2026-08-23
> **Deciders**: SA + TL + EM
> **Consulted**: SEC + SRE
> **Informed**: 全部

## Context and Problem Statement

[基本设计 §7.5 密钥管理](../../design/basic-design/07-security-design.md#75-密钥管理-secrets-managementsec-req-005-sec-req-010) 提到 SEC-REQ-005~010 要求 KEK (Key Encryption Key) 与 DEK (Data Encryption Key) 分层管理。

[ADR-0007 密钥管理](../../architecture/decisions/0007-密钥管理-Vault-KMS替代-DB-直接存储方案.md) 选 Vault / KMS 作为 V1+ Cloud 的密钥管理方案,但**MVP 阶段具体怎么落**未明确:

- KEK 存哪里?(本地文件 / KMS / Vault)
- DEK 怎么派生?(envelope encryption)
- 密钥轮换策略?
- App 凭证、AI Provider API key、SSH host key 等不同种类的密钥如何分类?

## Decision Drivers

- [DRIVER-1] MVP 阶段零外部依赖,Local-First 单进程
- [DRIVER-2] V1+ Cloud 阶段接 Vault,路径平滑
- [DRIVER-3] KEK 不直接接触明文数据,DEK 失效不影响其他数据
- [DRIVER-4] 密钥轮换不能影响正在运行的事务
- [DRIVER-5] 与 [ADR-0014 WebAuthn](#) 协调:WebAuthn 凭证派生密钥也可放 Vault

## Considered Options

1. **Option A — MVP 本地文件 (0600) / V1+ Vault KV v2** (推荐)
2. **Option B — MVP 阶段用 SOPS + age**
3. **Option C — 全程自己实现 KEK/DEK 派生,延迟到 V1+ 再接 Vault**

## Decision Outcome

**Chosen option**: "Option A — MVP 本地文件 (0600) / V1+ Vault KV v2", because MVP 阶段不引入外部依赖;V1+ 平滑迁移;envelope encryption 模式与 Vault 推荐用法一致。

### MVP 阶段实现

```text
$PLATFORM_DATA_DIR/
├── keys/
│   ├── kek.bin          # 32 bytes, file mode 0600, owner = platform user
│   ├── kek.pub          # 用于审计 (不参与加解密)
│   └── audit.log        # 密钥派生 / 解封事件 (仅审计, 不含密钥)
└── platform.db          # PostgreSQL, 表 app_creds / user_creds / ai_provider_keys
                          # DEK 字段已用 KEK 加密后存储 (envelope encryption)
```

### 密钥层级

| 层 | 用途 | 存储 | 轮换 |
|---|---|---|---|
| **KEK** (Key Encryption Key) | 加密 DEK | MVP: 本地文件 0600;V1+: Vault KV v2 | 90 天 (生成新 KEK → 重新包装所有 DEK) |
| **DEK** (Data Encryption Key) | 加密业务数据(用户密码 hash、App 凭证、AI API key) | PostgreSQL `encrypted_payload` 字段 | 每次写入新数据时随机生成 |
| **派生密钥** | 加密日志、临时缓存 | 进程内 (不落盘) | 进程退出即销毁 |

### 各类密钥的具体存储位置

| 密钥种类 | 存储 | 加密 | 备注 |
|---|---|---|---|
| KEK | 本地 `keys/kek.bin` (0600) | — | 仅 platform user 可读 |
| DEK (per-record) | PostgreSQL `encrypted_payload` | AES-256-GCM(KEK) | envelope encryption |
| 用户密码 hash | `users.password_hash` | Argon2id | OWASP 推荐,不需要 KEK |
| JWT 签名密钥 | 本地 `keys/jwt-signing.bin` (0600) | — | Ed25519 私钥 |
| SSH host key (V1+) | Vault KV v2 `secret/data/ssh/host_key` | Vault 自管 | 90 天轮换 |
| AI Provider API key | PostgreSQL `ai_provider_keys.encrypted_key` | envelope(KEK) | 明文仅在进程内使用 |
| App App 凭证 | PostgreSQL `app_creds.encrypted_secret` | envelope(KEK) | |

### 轮换流程

```text
KEK 轮换 (90 天 1 次):
  1) 生成 new_kek (32 bytes random)
  2) 对所有 encrypted_payload:
     dek = AES-256-GCM-decrypt(old_kek, payload.dek)
     new_payload.dek = AES-256-GCM-encrypt(new_kek, dek)
     payload = new_payload
  3) 写 new_kek 到 keys/kek.bin (覆盖前先备份到 keys/kek.bin.bak)
  4) 审计 event: kek.rotated
  5) 删除 keys/kek.bin.bak (验证 7 天无回滚后)
```

### V1+ Vault 迁移路径

```text
MVP 本地 KEK → V1+ Vault:
  1) 部署方启动 Vault, 配置 KV v2 secret engine at "secret/"
  2) 平台启动时检测 VAULT_ADDR env:
     - 未设置 → 走本地文件 (向后兼容)
     - 已设置 → 走 Vault;启动时 unseal & read secret/platform/kek
  3) Vault 不可达 → 启动失败 (V1+ 不允许降级到本地)
```

### Consequences

**Good:**
- [+] MVP 零外部依赖
- [+] V1+ 切 Vault 仅需启动时检测 env,业务代码不变
- [+] Envelope encryption 保证 DEK 泄漏不影响其他记录
- [+] 轮换流程可审计

**Bad:**
- [-] MVP 阶段 KEK 与 DEK 在同进程(虽然 KEK 本身是密文,不直接接触业务数据)
- [-] 轮换 90 天 1 次需要 background task,需测试 [REQ-TS-014] 强约束

### Confirmation

[CONFIRM] 详设 §09 安全实现 + [specs/test-specification.md §9](../../specs/test-specification.md#9-安全实现测试):
- TC-SEC-010: KEK 文件权限为 0600 且 owner = platform user (自动化 fs 检查)
- TC-SEC-011: 篡改 KEK 文件后,平台启动失败并提示"KEK 校验失败"
- TC-SEC-012: 90 天 KEK 轮换 task 触发后,所有 DEK 重新包装,业务读写仍正常
- TC-SEC-013: 篡改 `encrypted_payload` 任意一字节,解密失败并审计
- TC-SEC-014: 进程崩溃重启后,DEK 仍能解封(证明 KEK 与 DEK 分层)
- TC-SEC-015: V1+ 模式下,Vault 不可达时启动失败,不允许降级

## Pros and Cons of the Options

### Option A — MVP 本地文件 / V1+ Vault
[+] MVP 简单;V1+ 平滑;符合 Vault 推荐用法
[-] MVP 阶段 KEK 与 DEK 同进程

### Option B — SOPS + age
[+] 静态加密强;适合配置文件加密
[-] 不是 runtime KEK 方案,不适合 envelope encryption

### Option C — 自实现 KEK/DEK,V1+ 接 Vault
[+] 最简 MVP
[-] V1+ 需要重写抽象层,代码冗余

## References

- [ADR-0007: 密钥管理 Vault / KMS 替代 DB 直接存储](../../architecture/decisions/0007-密钥管理-Vault-KMS替代-DB-直接存储方案.md)
- [ADR-0014: WebAuthn 凭证库选型](#)
- [基本设计 §7.5 密钥管理](../../design/basic-design/07-security-design.md#75-密钥管理-secrets-managementsec-req-005-sec-req-010)
- [详细设计 §09 安全实现](../../design/detailed-design/09-security-impl.md)
- [Vault KV v2 — Secrets Engine](https://developer.hashicorp.com/vault/docs/secrets/kv/kv-v2)
- [NIST SP 800-57 — Key Management](https://csrc.nist.gov/publications/detail/sp/800-57-part-1/rev-5/final)

## Revision History

| Date | Author | Change |
|---|---|---|
| 2026-08-23 | Mavis (AI 起草) | Initial draft (Proposed) |
| YYYY-MM-DD | — | Status → Accepted (待 SA + TL + EM + SEC 签核) |
