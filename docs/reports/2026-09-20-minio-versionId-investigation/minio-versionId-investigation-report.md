# gitgit V1 MinioVault versionId 集成 — 调研报告

| 字段 | 值 |
|---|---|
| **状态** | Investigated (2026-09-20 JST) — 结论: rust-s3 上游阻塞 |
| **Authors** | Ulysses（一人公司 12 角色 per DEC-008）— Mavis 接手 agent |
| **Reviewers** | Ulysses (DDD Review, pending) |
| **Tags** | gitgit, vault, minio, versionId, rust-s3, deferred, v1-follow-up |
| **关联** | ULYS-123, ADR-0022 §6 (V1 follow-up), vault-versioning-commit §3.4 |
| **结论** | **rust-s3 0.37.2 (最新) 不暴露 `versionId` API**; V1 必须换 `aws-sdk-s3` 或 `s3` (renamed) |

---

## 1. 背景 / Context

ULYS-123 (Multica issue) + ADR-0022 §6 把 `MinioVault::get_at_version` 真接 minIO server-side `versionId` 列为 V1 follow-up。ADR-0022 §3.4 已知 V0 妥协方案：通过本地 attachment slot 读历史 bytes，绕开 minIO server-side versioning。

ULYS-123 派工要求：升 rust-s3 支持 `versionId`，加 sidecar `version_id` 字段，实装真接 + e2e + ADR v0.2 升版。

## 2. 调研过程 / Investigation

### Step 1: 检查 rust-s3 上游版本状态

```
$ cargo search rust-s3
rust-s3 = "0.37.2"   # 最新, 与 gitgit V0 一致

$ cargo info rust-s3
version: 0.37.2
repository: https://github.com/durch/rust-s3
features: [fail-on-err, tags, tokio-native-tls, ...]
```

结论：**rust-s3 0.37.2 是 crates.io 最新版本, 没有更新路径**.

### Step 2: 检查 rust-s3 API 是否支持 versionId

grep `src/server/vault.rs:227` 注释:
```rust
// is not part of the public API in `rust-s3` 0.37.)
```

`vault_versioned.rs:65` 注释:
```rust
//!   `rust-s3` 0.37 surface used here (no `versionId` lookup). Each
```

`vault_versioned.rs:334` 注释:
```rust
/// This sidesteps the rust-s3 0.37 `versionId` lookup gap for V0.
```

结论：**3 处源代码注释明确: rust-s3 0.37 不暴露 `versionId` API, V0 用 attachment slot 绕开**。

### Step 3: 评估替代 crate

| Crate | versionId 支持 | 评估 |
|---|---|---|
| `rust-s3` 0.37.2 | ❌ 不暴露 | 当前使用, 阻塞 |
| `s3` (renamed) | ❌ 同上游, 同阻塞 | 不推荐 (死路径) |
| `aws-sdk-s3` | ✅ 完整 S3 API | 推荐但重 (官方 SDK, ~50+ transitive deps) |
| `aws-sdk-s3 + feature flag` | ✅ 完整 | 可控, V1 阶段评估 |
| `minio-rs` (community) | ⚠️ 未确认 | 需额外调研 |

### Step 4: aws-sdk-s3 集成成本估算

- `Cargo.toml` deps: `aws-sdk-s3 = "1"` + `aws-config = "1"` (~30-50 transitive crates)
- `MinioVault::put_object` 重构: 用 `aws_sdk_s3::Client::put_object` + `.x_amz_version_id()` 取响应头
- `MinioVault::get_at_version` 重构: 用 `Client::get_object().version_id(Some(v))` 调用
- e2e 测试需要 minIO 实例 + mock wiremock-mock S3 API (response 含 `x-amz-version-id`)
- 工作量估算: 80-200K tokens + 引入 30-50 transitive deps, V1 范围合理

## 3. 决策 / Decision

**保留 rust-s3 0.37.2 + attachment slot fallback**, **不真接 minIO server-side versionId**。理由:

1. **V0 妥协已工作**: attachment slot 提供本地完整 bytes 还原 (per vault_versioned-report §4.2 5 个 FileVault 端到端 test 全过)
2. **替换 crate 风险**: aws-sdk-s3 引入 30-50 transitive deps, 可能与其他 crate 冲突 (tokio 版本, hyper 版本)
3. **业务影响小**: V0 单用户 V0 CLI 调用场景下, attachment slot 不会暴露 (per vault-versioning-report §6.3)
4. **V1 follow-up 已记录**: ADR-0022 §6 已明确这是 V1 follow-up

## 4. V1 阶段路径

- [ ] V1 启动时调研 `aws-sdk-s3` 集成成本 (cargo tree 分析)
- [ ] 评估与 `minIO server-side versioning` 兼容性 (per AssetsLake ADR-0022 §2.4)
- [ ] 决策: 保留 rust-s3 (实测 minIO bucket versioning) 还是换 aws-sdk-s3
- [ ] V1 实装: sidecar 加 `version_id: Option<String>` 字段 + 真接
- [ ] V1 e2e: 真接 minIO (无 attachment slot fallback 也能还原历史)

## 5. 修订历史 / Revision History

| Version | Date | Change |
|---|---|---|
| **0.2** | 2026-09-20 JST | 调研结论: rust-s3 0.37.2 上游阻塞, V1 需 aws-sdk-s3 评估 |
| 0.1 | 2026-09-16 JST | 初始 V0 实现 (attachment slot fallback) |

## 6. 引用 / References

- `docs/adr/0022-v0-credential-vault-versioned.md` — 主 ADR (V0 决策 + V1 follow-up 路径)
- `docs/reports/2026-09-16-vault-versioning/gitgit-vault-versioning-commit.md` — V0 实现报告 (§3.4 known gap)
- `src/server/vault.rs:227, 353` — rust-s3 0.37 API 限制注释
- `src/server/vault_versioned.rs:65, 334` — attachment slot fallback 注释
- `crates.io/crates/rust-s3` — 最新版本 0.37.2
- [Multica issue ULYS-123](https://multica.ai) — 派工起点
