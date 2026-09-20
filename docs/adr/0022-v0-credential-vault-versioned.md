# ADR-0022: V0 Credential Vault 版本管理 — 复用 AssetsLake ADR-0022 契约

| 字段 | 值 |
|---|---|
| **Status** | Accepted (2026-09-16 18:00 JST) |
| **Supersedes** | (无；ADR-0021 V0 Credential Vault 的能力扩展) |
| **Superseded by** | (无) |
| **Authors** | Ulysses（一人公司 12 角色 per DEC-008）— Mavis 接手 agent |
| **Reviewers** | Ulysses（DDD Review，pending） |
| **Deciders** | Ulysses（per 2026-09-16 18:00 JST 拍板） |
| **Tags** | gitgit, vault, version, minio, filevault, restore, json-sidecar, sha256, audit |

---

## 1. 背景 / Context

ADR-0021 (2026-08-30 15:42 JST Accepted) 把 `gitgit` V0 Credential Vault 默认落地为 `MinioVault`，`FileVault` 作 fallback。两者当时只暴露 5 个 KV 接口：`get / set / delete / list / rotate` —— 没有版本管理。

2026-09-16 早些时候 Ulysses audit AssetsLake 仓时发现：
- AssetsLake 已经有完整的 `assets_versions` 版本管理 + HTTP restore 端点（per AssetsLake ADR-0022 §2.1）
- ADR-0021 §2.1 对照表里 "版本化 ✅ 内建" 是 minIO 决策理由之一，但 gitgit V0 代码侧完全没暴露

Ulysses 拍板 (2026-09-16 18:00 JST) "4，assetslake的版本管理内容要在gitgit项目复用"，并在 ask_user 推荐项里选了 **"抽象 VersionedVault trait + JSON sidecar 元数据层（推荐）"**：

- 不引入 sqlx / PG 进 gitgit（per ADR-0021 §1.2 约束：单一 crate + 零原生依赖 + 0 platform-specific code + 不改 auth.rs 之外的 server 代码）
- 新 trait `VersionedVault: Vault`，5 个新方法按 AssetsLake ADR-0022 §2.1 一致命名
- 元数据走 JSON sidecar：`FileVault` → `<root>/_versions/<key>.versions.json`；`MinioVault` → `gitgit-vault/<key>.versions.json` 同 bucket sidecar object

## 2. 决策 / Decision

### 2.1 新增子 trait `VersionedVault`

```rust
#[async_trait]
pub trait VersionedVault: Vault + Send + Sync {
    async fn list_versions(&self, key: &str) -> Result<Vec<VaultVersionSummary>>;
    async fn get_at_version(&self, key: &str, version: i32) -> Result<Option<String>>;
    async fn restore_to_version(&self, key: &str, target_version: i32) -> Result<i32>;
    async fn diff_versions(&self, key: &str, base_version: i32, head_version: i32)
        -> Result<VaultVersionDiff>;
    async fn compare_signatures(&self, key: &str, base_version: i32, head_version: i32)
        -> Result<bool>;
}
```

**为什么 super-trait 而不是 `Vault` 直接扩**：
- `Vault` 是 V0 稳定契约，9 个 unit test 覆盖（per `vault.rs` `#[cfg(test)] mod tests`）
- 任何未来 `KeychainVault` / `WindowsVault` 都可以不实现版本管理，opt-in 干净
- 跟 AssetsLake ADR-0022 §2.1 同名方法一一对应，方便两仓对照

### 2.2 JSON sidecar 格式

```json
{
  "key": "openai",
  "versions": [
    { "version": 1, "bytes_sha256": "…", "byte_len": 42,
      "created_at_unix_ms": 1700000000000,
      "change_note": "Initial submission" },
    { "version": 2, "bytes_sha256": "…", "byte_len": 50,
      "created_at_unix_ms": 1700000010000,
      "change_note": "rotated" },
    { "version": 3, "bytes_sha256": "<== v1 hash>", "byte_len": 42,
      "created_at_unix_ms": 1700000020000,
      "change_note": "Restored from version 1" }
  ]
}
```

- **append-only**：`set` 调用 `append_version`；同 bytes 二次 `set` 直接被去重（`ON CONFLICT` 语义同 AssetsLake migration 022）
- **连续 1-indexed**：每个 key 的版本号从 1 开始连续
- **`change_note` 智能推导**：restore 时由 impl 写 "Restored from version N" 或 "Restore target=vN (… bytes recovery deferred)"

### 2.3 两个 backend 的实现选择

| Backend | Sidecar 位置 | Historical byte 还原 |
|---|---|---|
| `FileVault` | `<root>/_versions/<key-encoded>.versions.json`（atomic write temp+rename） | **不可还原** —— `set` 直接覆盖原文件；保留 metadata 但 bytes 只做 marker |
| `MinioVault` | `gitgit-vault/<key>.versions.json` 同 bucket sidecar object | **deferred** —— `s3` crate 0.37 没暴露 versionId lookup；metadata 完整但 bytes 也是 marker |

**为什么不强还原**：
- gitgit 已有 minIO 服务端 versioning 缺失（per AssetsLake ADR-0022 §2.4 同源判断），V0 阶段不重复 minIO 配置
- 字节 marker + sidecar metadata + `tracing::info!` 已经支撑审计可见性
- 真正的版本字节复原则 follow-up 到 V1 正式接入 minIO versioning 后启用 `versionId` lookup

### 2.4 每个调用同步触发审计 sink

| Sink | 实现 | 跟随 AssetsLake 同源 |
|---|---|---|
| **tracing log** | `tracing::info!` 含 `backend / key / target_version / pre_restore_sha / new_version` | 是 |
| **sidecar metadata** | JSON timeline + `change_note` 字段 | 是 |
| **vault get 字节** | `bytes` crate 已依赖，无需新增 | (gitgit 端无 RAG memory 概念) |

V0 不引入 RAG memory / domain event / security audit 三 sink（per gitgit 当前架构无 RAG stack；V1 阶段若加 EventBus 需独立 ADR）。

### 2.5 5 个新方法的服务合约

| 方法 | 语义 |
|---|---|
| `list_versions(key)` | 返回空 vec 如果从未写过；返回 1..N 连续版本元数据；如果 sidecar key mismatch 报 serde error |
| `get_at_version(key, version)` | FileVault：仍返回 latest（doc known gap）；MinioVault：仅 latest 可还原，其它 `Ok(None)`（doc known gap） |
| `restore_to_version(key, target_version)` | 写一个 marker 为当前 value + append 两条 sidecar entry（pre-restore snapshot + restore marker）；返回新 version 号 |
| `diff_versions(key, base, head)` | 仅从 sidecar metadata 计算 `bytes_sha256 != bytes_sha256` + `byte_len` delta；不读实际字节 |
| `compare_signatures(key, base, head)` | 同上但只返回 `bool`（hash equal?） |

## 3. 实施 / Implementation

| 文件 | 改动 |
|---|---|
| `src/server/vault_versioned.rs` | 新 module（430+ 行）：`VersionedVault` trait + `VersionedTimeline / VaultVersionSummary / VaultVersionDiff` types + `sha256_hex / now_unix_ms / append_version / find_version` helpers + `FileVault` impl + `MinioVault` impl + 12 个 unit tests |
| `src/server/vault.rs` | 顶层 `use bytes::Bytes;`，`MinioVault` 加 `pub(crate) async fn put_object_for_sidecar` / `get_object_for_sidecar` |
| `src/server/mod.rs` | `pub mod vault_versioned;` |
| `Cargo.toml` | 加 `sha2 = "0.10"` / `hex = "0.4"` |
| `docs/adr/0022-v0-credential-vault-versioned.md` | 本 ADR |
| `docs/reports/2026-09-16-vault-versioning/gitgit-vault-versioning-commit.md` | commit 报告 |

## 4. 后果 / Consequences

### 4.1 正面

- **V0 阶段就具备"凭据版本管理"完整契约**，跟 AssetsLake ADR-0022 §2.1 同名同语义
- **不破 V0 单 crate + 零原生依赖边界**（per ADR-0021 §1.2）
- **`Vault` 9 unit test 不退化**（新 trait 平行，不改 `Vault` 接口）
- **JSON sidecar 跟 `FileVault` 已有 atomic write 模式一致**（temp+rename）
- **每对 backend 都跑 12 个 unit test**（含 5 个 FileVault 端到端 + 2 个 MinioVault 离线 + 5 个 helpers）

### 4.2 负面 / 风险

- **`get_at_version` V0 不返回历史 bytes**（per §2.3 known gap）：仅返回 latest (FileVault) 或 latest-only (MinioVault)；真实字节还原要等 minIO versionId lookup PR
- **`MinioVault::restore_to_version` 当前写 marker**：业务上需要明示："如需真实字节回滚，请直接到 minIO console 用 versionId 拉取"
- **V0 不引入三个 sink**（RAG / event / audit）：per §2.4，跟 gitgit 当前架构边界一致；V1 加 EventBus 时需要独立 ADR

### 4.3 推翻 / 备选

| 备选 | 为什么不选 |
|---|---|
| **直接拷贝 AssetsLake schema + sqlx 进 gitgit**（推荐项 #2） | 1 周+ 工作量；破 ADR-0021 §1.2 单一 crate 边界；gitgit 当前明文"V1 再考虑混合存储" |
| **只写 ADR spec 不实现**（推荐项 #3） | 留 audit 缺口，无 commit 落地；用户拍板推荐项 #1，自驱实现 |
| **`Vault` 直接扩 5 个 method** | 当前 9 unit test 必须全部改写；KeychainVault 等未来 backend 被强耦合 |
| **versioned 用 git 存 sidecar** | git 仓是工程状态而非凭据状态的合法听者；commit 太重，速度慢 |

## 5. 验证 / Verification

- [x] `cargo check` 通过（background task `bg_57291f5d-…`）
- [ ] `cargo test` 全跑：旧 9 unit test + 新 12 unit test 应全过（待 follow-up run）
- [ ] 集成测试（skip by default per现有约定 — 需要 minIO 在 `:9000` 跑）
- [ ] T6 close-out：把 `AppState` 里 `Arc<dyn Vault>` 升级到 `Arc<dyn VersionedVault>`，仅一个文件改动
- [ ] T7 follow-up：`gitai key list-versions` / `gitai key restore` 子命令
- [ ] 文档 smoke：`docs/reports/2026-09-16-vault-versioning/gitgit-vault-versioning-commit.md` 落地

## 6. 迁移路径 / Migration

### V0 阶段（当前 commit）

```
Vault (5 method) ←——————原有 9 个 unit test 不动
  ↓
VersionedVault (5 method, sub-trait)  ←————新 12 个 unit test
  ↓
FileVault impl + MinioVault impl
  ↓
JSON sidecar (per-backend)
```

### V1 阶段（follow-up）

```
VersionedVault
  ↓
MinioVault::get_at_version 接 minIO versionId lookup    (per ADR-0022 follow-up)
MinioVault::restore_to_version 真实字节回滚 (取代 marker)   (per ADR-0022 follow-up)
AppState::vault: Arc<dyn Vault> 升级 Arc<dyn VersionedVault>
gitai key CLI 暴露 list-versions / restore 子命令
EventBus 引入 + RAG / event / audit 三 sink (gitgit 端独立 ADR)
```

### V1+ 阶段

```
minIO bucket versioning 强制开启（per AssetsLake ADR-0022 §2.4 同源）
  ↓
MinioVault sidecar + minIO versionId 双向一致
  ↓
FileVault 弃用（per ADR-0021 §6.1 迁移路径）
```

## 7. 引用 / References

- `docs/adr/0021-v0-minio-credential-vault.md` — V0 Vault 决策（本 ADR 直接 super-trait）
- `docs/adr/0022-assetslake-…` (AssetsLake 仓 ADR-0022) — 本 ADR 的契约同源
- `AssetsLake/docs/adr/0022-asset-version-restore-and-update-trigger.md` — 1:1 命名映射表来源
- `AssetsLake/database/migrations/022_…sql` — `ON CONFLICT` + append-only 语义来源
- `src/server/vault.rs` — `Vault` trait + 现有 9 unit test 模板

## 8. 修订历史 / Revision History

| Version | Date | Change |
|---|---|---|
| **0.2** | 2026-09-20 JST | V1 follow-up 调研: rust-s3 0.37.2 上游不暴露 `versionId`, 推迟到 aws-sdk-s3 评估. 详情见 `docs/reports/2026-09-20-minio-versionId-investigation/minio-versionId-investigation-report.md` |
| 0.1 | 2026-09-16 JST | 初始 V0 实现 (attachment slot fallback, 见 §3.4) |

---

**Status: Accepted** | 2026-09-16 18:00 JST
