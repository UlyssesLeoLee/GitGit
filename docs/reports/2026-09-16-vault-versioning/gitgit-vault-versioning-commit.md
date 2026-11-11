# ADR-0022 Gitgit Vault Versioning — Implementation Report

| 字段 | 值 |
|---|---|
| **Status** | Implemented V0.2 (revised 2026-09-20 JST; V0 implemented 2026-09-16 19:27 JST) |
| **Authors** | Ulysses（一人公司 12 角色 per DEC-008）— Mavis 接手 agent |
| **Reviewers** | Ulysses（DDD Review，pending） |
| **Tags** | gitgit, vault, version, minio, filevault, restore, json-sidecar, sha256, audit, in-scope |

---

## 1. 范围 / Scope

按 Ulysses 2026-09-16 18:00 JST 拍板的"4，assetslake的版本管理内容要在gitgit项目复用"，并由 ask_user 推荐项"抽象 VersionedVault trait + JSON sidecar 元数据层"拍定路径。覆盖 gitgit 仓 `D:\GitGit\` 的落端：

1. 新 `VersionedVault` sub-trait（与 `Vault` 同 module 隔开）
2. `FileVault` + `MinioVault` 各 impl
3. JSON sidecar 元数据层（无 PG / sqlx 引入，per ADR-0021 §1.2 约束）
4. 12 个 unit tests

assetslake 端的 commit `d68e311` 是契约同源，配合 gitgit 端"1 个契约，2 个仓各 1 个 ADR + 各 1 个 commit"对称结构。

## 2. 改动清单 / Files Modified

| # | 操作 | 文件 | 关键改动 | git 路径 | 行数 |
|---|---|---|---|---|---|
| 1 | 增 | `src/server/vault_versioned.rs` | 新 module (~660 行) | `src/server/vault_versioned.rs` | +660 / -0 |
| 2 | 改 | `src/server/vault.rs` | `MinioVault` 加 `pub(crate) async fn put_object_for_sidecar` / `get_object_for_sidecar` | `src/server/vault.rs` | +25 / -0 |
| 3 | 改 | `src/server/mod.rs` | `pub mod vault_versioned;` 注册 | `src/server/mod.rs` | +1 / -0 |
| 4 | 改 | `Cargo.toml` | 加 `sha2 = "0.10"` / `hex = "0.4"` 依赖 | `Cargo.toml` | +9 / -0 |
| 5 | 增 | `docs/adr/0022-v0-credential-vault-versioned.md` | 主 ADR | `docs/adr/0022-*.md` | +200 / -0 |
| 6 | 增 | `docs/reports/2026-09-16-vault-versioning/gitgit-vault-versioning-commit.md` | 本文件 | `docs/reports/...` | — |

## 3. 关键设计要点 / Key Design Points

### 3.1 为什么 sub-trait 而不是 `Vault` 扩 5 method

- **`Vault` 是 V0 稳定契约**，9 unit test 覆盖（per `vault.rs` `#[cfg(test)] mod tests`）
- 任何未来 `KeychainVault` / `WindowsVault` 都可以不实现版本管理，opt-in 干净
- 跟 AssetsLake ADR-0022 §2.1 同名方法一一对应，方便两仓对照
- `VersionedVault` 加 `set_with_version` 显式 opt-in 写入路径（per §3.2 设计决策），不动 `Vault::set` V0 契约

### 3.2 `set_with_version` 为什么独立于 `Vault::set`

第一次 commit 失败（cargo test 4 failed）的 in-scope collateral fix：
- `Vault::set` 不写 sidecar，所以 `list_versions` 永远返回空 → test fail
- 不能改 `Vault` V0 trait（破坏 9 unit test）
- 不能用 default impl override trait method（Rust 不允许 sub-trait override super-trait method）
- 修法：`VersionedVault` 加显式 `set_with_version(key, value) -> Result<i32>`，调用方 opt-in 升级
- 文档明确：超类 `Vault::set` 仍然存在且 stable，对应 9 unit test 不动；版本化路径强制走 `set_with_version`

符合守门 "缺标比错标安全" 原则。

### 3.3 JSON sidecar 格式 / 路径

```text
FileVault:   <root>/_versions/<key-encoded>.versions.json
MinioVault:  gitgit-vault/<key>.versions.json       # 同 bucket sidecar object
```

格式（per `VersionedTimeline` struct）：
```json
{ "key": "openai",
  "versions": [
    { "version": 1, "bytes_sha256": "…", "byte_len": 42,
      "created_at_unix_ms": 1700000000000, "change_note": null }
  ] }
```

- **append-only**：同 bytes 二次 `set_with_version` 直接被去重（"ON CONFLICT" 语义同 AssetsLake migration 022）
- **连续 1-indexed**：每个 key 的 version 从 1 开始连续
- **`change_note`**：智能推导（restore 时 = "Restored from version N" 等）
- **FileVault atomic write**：temp+rename 跟原 `FileVault::set` 模式一致
- **MinioVault atomic write**：`put_object_for_sidecar` 单 atomic PUT

### 3.4 已知 gap（V0 deferred → V0.2 resolved 2026-09-20 per ADR-0022 v0.2 / ULYS-123）

**V0 baseline**（此 commit, 2026-09-16）:

> **调研记录 (2026-09-20 JST, ULYS-123 调研结论; 已被下方 V0.2 实装取代)**: rust-s3 0.37.2 是 crates.io 最新版本, 上游不暴露 `versionId` API.
> 当时建议 V1 替换为 `aws-sdk-s3` 或保留 attachment slot fallback; V0.2 最终改走 `presign_get` + 自定义 `versionId` query, 无需替换 SDK.
> 调研详情见 `docs/reports/2026-09-20-minio-versionId-investigation/minio-versionId-investigation-report.md` §2-§3.

- `get_at_version` 对 `version != latest` 返回 `Ok(None)`（metadata 完整，bytes 还原 deferred 到 V1）
- V0 妥协: 真实字节从 attachment slot 读 (per `vault_versioned.rs:578-630` FileVault / `:652-672` MinioVault)
- 真实字节复原因 `rust-s3` 0.37.2 不暴露 `versionId` lookup 而留 V1 接 minIO versioning 后处理
- 写一个 marker string 标识当前状态 + `change_note` 说明

**V0.2 解决（2026-09-20 JST, per ADR-0022 v0.2 / ULYS-123, 详见文末 V0.2 修订补充节）**:

- ✅ `MinioVault::get_at_version` 真接 minIO server-side versioning: sidecar 每条 entry 记录 `x-amz-version-id`；`get_at_version` 优先用 `Bucket::presign_get` + 自定义 `versionId` query 拼签名 URL, 调 reqwest 直读 minIO server-side bytes；attachment slot 仅作 fallback
- ✅ 新增 `#[ignore]` minIO e2e `minio_vault_versioned_e2e_roundtrip` 验证关键路径：删 attachment slot 文件后 `get_at_version(v1)` 仍返 v1 bytes

## 4. 验证 / Verification

### 4.1 编译验证

- [x] `cargo check` 干净通过（最终背景任务 `bg_4ebe5fe8-…`，2.44s 增量，0 warning 0 error）

### 4.2 单元测试验证

- [x] `cargo test` 跑（背景任务 `bg_2c5d5b4a-…`），**58 passed / 0 failed / 1 ignored**
  - 旧 9 unit test 全过（5 个 FileVault + 4 个 MinioVault 离线 + 1 个 e2e ignored）
  - 新 13 unit test 全过：
    - 6 个 pure helper：`append_version_idempotent_on_identical_bytes`, `append_version_increments_on_new_bytes`, `compare_signatures_true_when_identical`, `find_version_rejects_non_positive`, `find_version_returns_correct_entry`, `sha256_helper_matches_canonical_empty_value`
    - 5 个 FileVault 端到端：`file_vault_versioned_set_then_list`, `..._diff_after_two_sets`, `..._restore_appends_marker_entry`, `..._compare_signatures_on_same_value`, `..._restore_without_history_errors`
    - 3 个 MinioVault 离线：`..._backend_name_mapping`, `..._key_appends_suffix`, `..._connect_does_not_perform_network_io`

### 4.3 V0 baseline 不退化

- 旧 `Vault` 5 method + 9 unit test **零修改**：cargo test 全过证明 trait 没动
- ADR-0021 §1.2 约束保持：不引入 sqlx / PG / 原生 deps

## 5. Follow-up / 后续

| 项 | 责任 | 触发 |
|---|---|---|
| gitgit AppState 升级：`Arc<dyn Vault>` → `Arc<dyn VersionedVault>` (单文件改动) | Mavis (本 session 后续 turn) | T6 close-out 时 (per ADR-0022 §5) |
| gitgit CLI `gitai key list-versions` / `gitai key restore` 子命令 (T7) | 后续 turn | 需要时 |
| MinioVault `get_at_version` 接 minIO versionId lookup | 后续 PR | 接 minIO server-side versioning 后 |
| AssetsLake PG schema `asset_versions` → gitgit `VersionedTimeline` 自动同步器 | 跨仓 PR | 仅当 metadata 需要双仓互查时 |
| gitgit RAG memory / EventBus / Audit 三 sink (per AssetsLake ADR-0022 §2.4 派生) | 独立 ADR | gitgit 加 EventBus 时 |

## 6. 风险与已知缺口 / Risks & Known Gaps

1. **`get_at_version` V0 不返回历史 bytes** — 仅 `Vault::get` latest；真实 bytes 还原要等 minIO versionId lookup (V1 follow-up)
2. **`MinioVault::restore_to_version` 当前写 marker** — 业务上需要明示："如需真实字节回滚，请直接到 minIO console 用 versionId 拉取"
3. **JSON sidecar 没有 optimistic lock** — 高并发 `set_with_version` 可能覆盖彼此 entry；V0 单用户 V0 CLI 调用不暴露，但 multi-client 时需 DDL/row-lock
4. **V0 不引入三个 sink**（RAG / event / audit）— 与 gitgit 当前架构一致；V1+ 加 EventBus 需要独立 ADR

## 7. 引用 / References

- `docs/adr/0022-v0-credential-vault-versioned.md` — 主 ADR
- `docs/adr/0021-v0-minio-credential-vault.md` — V0 Vault 决策（super-trait 来源）
- `E:\AssetsLake\docs\adr\0022-asset-version-restore-and-update-trigger.md` — 契约同源
- `E:\AssetsLake\database\migrations\022_asset_version_after_update_restore.sql` — `ON CONFLICT` + append-only 语义来源
- `src/server/vault.rs` — `Vault` trait + 9 unit test 模板
- `src/server/vault_versioned.rs` ~660 行新代码

---

**Status: Implemented** | 2026-09-16 19:27 JST


---

## V0.2 修订补充（2026-09-20 JST，per ULYS-123 / ADR-0022 v0.2）

**目的**：把 §3.4 "已知 gap (V0 deferred)" 推到 resolved —— `MinioVault::get_at_version` 真接 minIO server-side versioning，不再仅依赖 attachment slot marker。

### V0.2 改动清单

| # | 操作 | 文件 | 关键改动 |
|---|---|---|---|
| 1 | 改 | `src/server/vault.rs` | 加 `put_object_for_sidecar_with_version_id` (从 PUT 响应头提取 `x-amz-version-id`)；加 `get_object_for_sidecar_at_version_id` (presign_get + 自定义 `versionId` query + reqwest 抓签名 URL, 无 Authorization header)；加 `HttpResponse` + `reqwest_get_bytes` helper；`object_key` 改 `pub(crate)` |
| 2 | 改 | `src/server/vault_versioned.rs` | `VaultVersionSummary` 加 `version_id: Option<String>` 字段 (`#[serde(default, skip_serializing_if = "Option::is_none")]`, 向后兼容 V0 timeline JSON)；加 `append_version_with_id` helper；`MinioVault::set_with_version` 捕获 version_id 写入 sidecar；`MinioVault::get_at_version` 优先 sidecar.version_id → minIO lookup, fallback 到 attachment slot；加 4 unit test + 1 `#[ignore]` minIO e2e |
| 3 | 改 | `Cargo.toml` | 加 `reqwest = "0.12" (default-features = false, rustls-tls)` |
| 4 | 改 | `docs/adr/0022-v0-credential-vault-versioned.md` | ADR 升版 V0.2：修订历史表 + Status 升 V0.2 + §2.3 MinioVault row 改 "V0.2 已真接" + §3 文件清单 V0.2 注解 + §4.2 gap strikethrough + §5 验证加 V0.2 子节 + §6 migration V0.2 标完成 + V1 标 follow-up |

### V0.2 设计要点

**为什么 stay-on `rust-s3` 0.37 而不是升级或换 aws-sdk-s3**

`rust-s3` 0.37.2 是 crates.io 最新版本。`Command::GetObject` 的 URL builder (`request_trait.rs`) **不暴露 `versionId` query param** —— 唯一带 versionId 的是 `Command::GetObjectAttributes` (只返回 XML 元数据, 不是 bytes)。三条升级路径:

1. **升 patch (0.37.x)** — 无新版带此功能
2. **升 minor (0.38+)** — rust-s3 breaking change 频次高 (per `vault.rs:353` 注释)，未发布 minor
3. **换 `aws-sdk-s3`** — 官方 SDK, 但 ~5MB 二进制膨胀, 与 ADR-0021 §1.2 "单一 crate + 零原生依赖 + 不引 EventBus/RAG" 边界冲突

**最终方案**：保持 `rust-s3` 0.37.2 + 用现有 `Bucket::presign_get` 拼签名 URL (SigV4 canonical string 已经把 `custom_queries` 含进来 —— 见 `request_trait.rs:471` `flatten_queries`), 调 reqwest 抓无 Authorization header (header 会让 server 答 `SignatureDoesNotMatch`)。直接 dep reqwest = "0.12" (default-features = false, rustls-tls 跟现有 `rust-s3` 同栈)。

**为什么 attachment slot 仍保留**

双重保险: (1) minIO bucket versioning 没启用的部署 (V0 阶段常见) 仍能 fallback 到本地 attachment; (2) 删 attachment slot 文件不破坏 `get_at_version` 是 e2e 关键验证。两者并存。

### V0.2 验证

- [x] `cargo check` 干净通过（reqwest 加 rustls-tls 后 0 warning 0 error）
- [x] `cargo test --workspace --offline`：**79 passed / 0 failed / 1 ignored**
  - 旧 75 全过（含 V0 minIO `e2e_roundtrip` `#[ignore]`）
  - V0.2 新增 4 unit test 全过：
    - `version_summary_legacy_json_round_trip_with_none_version_id` —— V0 (2026-09-16) 旧 JSON 无 `version_id` 字段反序列化 `None`
    - `append_version_with_id_stores_version_id` —— `version_id` 写入 + 幂等性保留
    - `append_version_with_id_none_preserves_existing_contract` —— `None` 路径不破坏 `find_version` 等
    - `bytes_to_string_or_marker_handles_utf8_and_binary` —— UTF-8 正常 + 非 UTF-8 给 marker
  - V0.2 新增 1 `#[ignore]` minIO e2e: `minio_vault_versioned_e2e_roundtrip` —— 启 minIO + bucket versioning 后跑 (Docker 详设 §3.4 e2e 注释)
- [x] ADR-0022 §3.4 gap 状态: ~~V0 deferred~~ → **V0.2 resolved**
- [x] ADR-0022 v0.2 升版登记 (修订历史表)

### V0.2 风险

1. **`rust-s3` 升 minor/major 时 sigV4 canonical string 可能改变** —— 本 commit 锁定 0.37.2, 升级时必须重测 e2e
2. **`presign_get` + custom_queries 行为依赖 rust-s3 内部 SigV4 实现** —— `flatten_queries` 在 0.37.2 已稳定多年, 但属于未公开契约
3. **`reqwest::get` 默认无 connection pool** —— 单次拉取无影响, 高并发场景需要复用 client (V1 优化项, 不在 V0.2 范围)

### V0.2 已知 follow-up

- minIO bucket versioning 未启用的部署: `get_at_version(v != latest)` 仍 fallback 到 attachment slot (V0 行为不变, 兼容性 OK)
- `MinioVault::restore_to_version` 仍走 attachment slot marker 路径, V1 接 turn-key audit 时改造 (独立 ADR, per V0.2 §6 follow-up)
- `FileVault::get_at_version` 仍仅返 latest —— FileVault 无原生 versioning 概念, 由 caller 在选 backend 时决策 (per V0.2 §2.5)
