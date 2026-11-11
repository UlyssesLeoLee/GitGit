# ADR-0021: V0 Credential Vault 改用 minIO S3-compatible

| 字段 | 值 |
|---|---|
| **Status** | Accepted (2026-08-30 15:42 JST) / Amended (2026-10-25 — 服务端从 minIO 换成本仓自建，见 §1 修订注记) |
| **Supersedes** | (无；ADR-0020 §2.5 Vault 默认实现变更由本 ADR 触发) |
| **Superseded by** | (无) |
| **Authors** | Ulysses（一人公司 12 角色 per DEC-008）— Mavis 接手 agent |
| **Reviewers** | Ulysses（DDD Review，pending） |
| **Deciders** | Ulysses（per 2026-08-30 15:42 JST 拍板） |
| **Tags** | v0, vault, credential, minio, s3-compatible, keychain, license |

---

## 1. 背景 / Context

8/26 ADR-0020 决策 V0 Credential Vault 默认实现为 `FileVault`（落
`~/.config/gitgit/credentials.toml`，0600 权限），`WindowsVault`（keyring 调 Windows
Credential Manager）作为 V1 切换。

8/30 15:42 JST Ulysses 重新拍板：**Credential Vault 改用 minIO S3-compatible 对象存储**，
默认实现改为 `MinioVault`，`FileVault` 降级为 fallback，`WindowsVault` 暂不实现（V1
视 minIO 商业化评估再决定）。

本 ADR 是 8/30 15:42 JST 决策的正式记录，并补充原 FileVault 方案的 4 个备选对比。

### 1.1 原 FileVault 方案的问题（per 8/30 15:42 JST 复盘）

| 问题 | 影响 | 严重度 |
|---|---|---|
| 凭证值类型仅 `String` | API Key 含换行 / 二进制（如 SSH private key）需 base64 编码 | 中 |
| 跨节点访问困难 | V1 多机部署时 `~/.config/gitgit/` 不在 NFS 上 → 凭证漂移 | 高 |
| 备份手工 | 需自写 cron + rsync；无内建 versioning | 中 |
| 无审计日志 | `get/set/delete` 不记操作者 / 时间 | 中 |
| 平台绑定 | macOS / Linux 上 FileVault 权限模型与 Windows 不同 | 低 |

### 1.2 约束（per 2026-08-30 拍板）

- **范围限定 docs 层**：不改 `src/server/auth.rs` 或任何 `src/**/*.rs`
- **范围限定 docs 层**：不改 `Cargo.toml`（不引 minIO Rust 依赖）
- **R-05 不 push** (per 8/27 11:09 JST)：本 commit 留本地 ahead
- **代签规则** (per 2026-08-27 07:16 JST + 19:39/20:56/21:59 三次强化)：commit author = `Ulysses <ulysses@mavis.local>`，本 ADR 编制 = `Ulysses（一人公司 12 角色 per DEC-008）— Mavis 接手`
- **禁"per X 历史形态"回溯叙事** (per 2026-08-26 强证据)：仅 git log 实证

> **修订注记（2026-10-25）**：本 ADR 的**决策**仍然成立——凭据放在 S3 兼容的
> `gitgit-vault` bucket 里，`FileVault` 降级为 fallback。**变的是服务端**：
> 提供这套 S3 语义的服务端已从 minIO 换成本仓自建（`src/objectstore/` 与
> `gitgit-objectstore` 二进制）。
>
> `[FACT]` 原因：minIO 服务端是 GNU AGPLv3，而本仓以 HTTP 方式消费它——把
> 一个 AGPL 服务打进发行物会引出网络条款的义务。商业化路径上「评估切换
> Ceph RGW / SeaweedFS / Garage」因此没有走：**实际选的是第四条路——自建**。
>
> 1. §1 / §2 的历史决策文本**不改写**。它们记录的是 2026-08-30 15:42 JST
>    当时的判断与当时的事实。
> 2. 客户端侧没有变化：`rust-s3` 0.38 客户端（path-style 寻址 + SigV4）
>    照旧，所以凭据管理与版本化逻辑**不需要重做**。要自己写的只有服务端。
> 3. 名字跟着服务端一起退场：`MinioVault` → `S3Vault`（它一直是 S3 客户端，
>    不该以某个服务端命名）、`GITGIT_MINIO_*` → `GITGIT_S3_*`、
>    `deploy/minio/docker-compose.yml` → `deploy/objectstore/`、
>    `scripts/verify-minio-vault.ps1` → `scripts/verify-s3-vault.ps1`。
> 4. 部署夹具少掉一个外部依赖：`mc` 客户端与 docker-compose 都不再需要。
>    服务端自己的 `--bootstrap-bucket` 就是原来 `mc mb` + `mc version enable`
>    的替代。
> 5. 两条 vault 端到端测试摘掉了 `#[ignore]`：服务端是本仓的，测试可以自己
>    在进程内把它起起来（端口 0，桶显式开版本化），于是 S3 那条路被常规
>    测试套件与 CI 持续覆盖。
>
> `[FACT]` 详见 `docs/plan/self-hosted-object-store.md`（S3 面、架构落点与
> 正确性怎么证明）与 `docs/plan/v0-tasks.md` 的决策日志。

## 2. 决策 / Decision

**Credential Vault V0 默认实现改为 `MinioVault`（minIO S3-compatible 对象存储），落 `gitgit-vault` bucket**。

- `FileVault` 降级为 V0 fallback（仅 minIO 不可达时启用）
- `WindowsVault` 暂不实现（V1 视 minIO 商业化评估再决定）
- 详见 [ADR-0020](0020-v0-gui-tauri-svelte.md) §2.5 + §2.5.1 部署要求

### 2.1 minIO 决策理由

| 维度 | minIO | FileVault | WindowsVault | HashiCorp Vault |
|---|---|---|---|---|
| **类型灵活度** | 任意 binary (S3 object) | 仅 String | 仅 String | 任意 KV (含 binary) |
| **跨节点** | ✅ 原生 S3 protocol | ❌ 需 NFS | ❌ 单机 | ✅ Raft 集群 |
| **版本化** | ✅ 内建 object versioning | ❌ 手工 | ❌ 手工 | ✅ KV v2 versioning |
| **备份** | ✅ erasure coding 4+2 默认 | ❌ 手工 cron | ❌ 手工 | ✅ Raft snapshot |
| **审计** | ✅ minIO audit log (V1+) | ❌ 无 | ❌ 无 | ✅ 内建 audit log |
| **License** | AGPL-3.0 (社区) | 项目内置 | Apache-2.0 (keyring crate) | Business Source License (BSL) |
| **部署复杂度** | 中 (docker-compose) | 零 | 零 | 高 (Vault server + unseal) |
| **平台** | Win/macOS/Linux | Win/macOS/Linux | 仅 Win | Win/macOS/Linux |
| **5+ 凭证源扩展** | ✅ bucket + prefix | ⚠ 文件膨胀 | ⚠ keyring 容量限 | ✅ KV v2 path 层次 |
| **V0 工时** | 0.5 d (部署) + 1 d (MinioVault impl) | 0.5 d | 0.5 d | 2-3 d (server + token 引导) |

**minIO 胜出理由**：
1. 跨节点访问原生支持（V1+ 多机部署不需重写）
2. 备份 / 版本化 / 审计 内建，省去自建基础设施
3. AGPL-3.0 在 V0 dev 阶段无商业风险；商业发布前可评估切换 Ceph RGW / SeaweedFS / Garage（皆 Apache-2.0）
   —— `[FACT]` **2026-10-25 修订注记**：这一条后来没有按原样走。评估的结果是
   **第四条路：自建**（本仓自己实现 S3 兼容服务端），理由与代价见 §1 修订注记
   与 `docs/plan/self-hosted-object-store.md`。
4. 5+ 凭证源（10 provider 全部要存）扩展性好
5. 部署 docker-compose 单文件，Win/macOS/Linux 一致

### 2.2 备选方案对比

| 备选 | 优势 | 劣势 | 为什么不选 |
|---|---|---|---|
| **FileVault 维持默认** (原 8/26 决策) | 零部署、零依赖 | 跨节点 / 备份 / 审计全无 | 8/30 拍板推翻 |
| **HashiCorp Vault** (per archived 0007) | KV v2 行业标准、audit log、dynamic secrets | BSL 商业 license、unseal 流程复杂、需独立 server | V0 工时 2-3 d vs minIO 0.5 d；V1 评估时再迁移 |
| **PG 加密列** (PG 18.6 `pgcrypto` 扩展) | 与 gitgit 现有 PG 复用、无新组件 | 仅 String / bytea、备份靠 PG 备份、跨节点依赖 PG replication | 备份 / 审计需自建；不如 minIO 专业 |
| **minIO** (per 8/30 15:42 JST 拍板) | S3 生态、跨节点、原生 backup、type 灵活 | AGPL-3.0 商业风险、docker 部署依赖 | ✅ **本 ADR 决策** |
| **自建 S3 兼容服务端** (per 2026-10-25 拍板) | S3 语义不变、客户端（`rust-s3`）不动；本仓不再依赖任何第三方 AGPL 服务端 | 要自己实现并验证服务端 | ✅ **实际落地的那条**（§1 修订注记）；本行的「为什么不选」一列不适用——它是后来被选中的那条路 |

### 2.3 HashiCorp Vault (per archived 0007) 备忘

`D:/GitGit/docs_archive_rust_impl_2026_08_26/architecture/decisions/0007-vault.md` 8/19
起草时曾考虑 HashiCorp Vault。8/30 15:42 JST 拍板 minIO 之后，HashiCorp Vault 留 V1+
视场景再评估（如需 dynamic secrets / 短期 token 时）。

## 3. minIO 部署要求

| 项 | 要求 | V0 阶段 | V1+ |
|---|---|---|---|
| **License** | 社区版 AGPL-3.0 | dev/OSS 即可 | 商业部署前评估 Enterprise License 或切换 Ceph RGW / SeaweedFS / Garage |
| **部署形态** | docker-compose (dev) | ✅ | Kubernetes StatefulSet (V1+) |
| **Bucket** | `gitgit-vault` | 单桶 | 按 provider 拆桶 `gitgit-vault-ai` / `gitgit-vault-remote` |
| **Versioning** | 启用 | ✅ | ✅ |
| **Lifecycle** | 90d 自动 expire deleted objects | ✅ | ✅ |
| **Erasure coding** | 4+2 (minIO 默认) | ✅ (单节点也启用) | ✅ (4+2 跨节点) |
| **网络** | `localhost:9000` (dev) | HTTP 关闭 TLS (仅 localhost) | K8s Service + cert-manager HTTPS 强制 |
| **凭证** | minIO root user/password 由 FileVault 启动时引导存（**仅 dev 阶段递归引用 vault 自身**） | **禁止 .env 入仓** | KMS / Secret Manager |
| **备份** | dev 单节点 erasure coding 即可 | ✅ | 跨节点 site replication |
| **Client SDK** | Rust 端 `s3` crate (pure-Rust, AWS Signature V4) | V0 用 `s3` | V1 评估 `rust-s3` 切换 |

### 3.1 Docker Compose 部署片段（V0 dev 阶段）

```yaml
# docker-compose.yml (项目根目录，V0 dev 阶段)
version: '3.8'
services:
  minio:
    image: minio/minio:latest
    command: server /data --console-address ":9001"
    environment:
      MINIO_ROOT_USER: ${MINIO_ROOT_USER}      # 由 .env 注入；不入仓
      MINIO_ROOT_PASSWORD: ${MINIO_ROOT_PASSWORD}  # 由 .env 注入；不入仓
    ports:
      - "9000:9000"  # S3 API
      - "9001:9001"  # Web console (dev only)
    volumes:
      - minio_data:/data
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9000/minio/health/live"]
      interval: 10s
      timeout: 5s
      retries: 3

volumes:
  minio_data:
```

**`.env` 模板（不入仓，仅本机）**：
```bash
# .env.example (项目根目录模板)
MINIO_ROOT_USER=gitgit-vault-admin
MINIO_ROOT_PASSWORD=  # 第一次 docker-compose up 时生成强密码并存到 FileVault
```

**Bucket provisioning (T11 验收)**：
```bash
# mc (minIO client) 安装
docker run -it --rm --network host \
  -v $(pwd)/.mc-data:/root/.mc \
  minio/mc alias set local http://localhost:9000 "$MINIO_ROOT_USER" "$MINIO_ROOT_PASSWORD"

# 建桶
mc mb local/gitgit-vault

# 启用 versioning
mc version enable local/gitgit-vault

# 90d lifecycle
mc ilm add local/gitgit-vault --expiry-days 90

# 网络可达性验证
curl -s http://localhost:9000/gitgit-vault?list
```

### 3.2 Rust 端接入（V0 实现阶段 T6）

```rust
// V0 MinioVault 伪代码（T6 实现时填实）
use s3::{Bucket, Region};
use s3::creds::Credentials;

pub struct MinioVault {
    bucket: Box<Bucket>,
}

#[async_trait]
impl Vault for MinioVault {
    async fn get(&self, key: &str) -> Result<Option<String>> {
        // GET s3://gitgit-vault/<key>
        let resp = self.bucket.get_object(key).await?;
        Ok(Some(String::from_utf8(resp.bytes().to_vec())?))
    }

    async fn set(&self, key: &str, value: &str) -> Result<()> {
        // PUT s3://gitgit-vault/<key>
        self.bucket.put_object(key, value.as_bytes()).await?;
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        // DELETE s3://gitgit-vault/<key> (with versioning, 软删)
        self.bucket.delete_object(key).await?;
        Ok(())
    }
}
```

**注意**：本 ADR 阶段不实现代码（per 用户"docs 层不改 Rust" 拍板），T6 阶段才写。

## 4. 后果 / Consequences

### 4.1 正面

- 跨节点访问原生支持（V1+ 多机部署无重写）
- 备份 / 版本化 / 审计 内建，省 5+ 个自建组件
- 5+ 凭证源（10 provider 全部要存）扩展性好（bucket + prefix 分层）
- 部署 docker-compose 单文件，Win/macOS/Linux 一致
- V0 工时 0.5 d 部署 + 1 d MinioVault impl（比 HashiCorp Vault 少 1-1.5 d）

### 4.2 负面 / 风险

- **AGPL-3.0 商业风险**（高影响）— 商业部署前必须评估 Enterprise License 或切换
  Ceph RGW / SeaweedFS / Garage（皆 Apache-2.0）。V0 dev 阶段无影响。
  —— `[FACT]` **2026-10-25 修订注记**：这条风险已按 §1 修订注记所述处置——
  改为自建服务端，本仓不再消费任何第三方 AGPL 服务端。
- **minIO 部署依赖 docker**（中影响）— Win 11 上 `docker desktop` / `Rancher Desktop`
  需预装；无 docker 时 minIO binary standalone 也可起，但 V0 推荐 docker-compose
- **minIO root 凭证递归引用 vault 自身**（中影响）— V0 简化：FileVault 启动引导存；
  V1 推进 KMS / Secret Manager
- **AGPL-3.0 网络服务条款**（中影响）— 若 gitgit 未来提供 SaaS 网络服务，整个 gitgit
  代码库也需 AGPL-3.0 或兼容；V0 桌面应用不涉及，发布 SaaS 时再评估

### 4.3 推翻的备选

| 备选 | 为什么不选 |
|---|---|
| FileVault 维持默认 | 跨节点 / 备份 / 审计全无；8/30 拍板推翻 |
| HashiCorp Vault | BSL 商业 license；unseal 流程复杂；V0 工时 2-3 d vs minIO 0.5 d；V1 评估 |
| PG 加密列 (`pgcrypto`) | 备份 / 审计需自建；不如 minIO 专业；PG 仍存元数据但凭证独立 |
| WindowsVault (keyring) | 绑定 Windows 平台；UAC 弹窗体验差；V0 跨平台需求不满足 |

## 5. 验证 / Verification

V0 完成的验收清单（在 ADR-0020 §4 基础上新增 minIO 相关）：

- [ ] (T11) `docker compose up -d minio` 启动后 `curl http://localhost:9000/minio/health/live` 返回 200
- [ ] (T11) `mc mb local/gitgit-vault` 成功
- [ ] (T11) `mc version enable local/gitgit-vault` 成功
- [ ] (T6) `gitai key set openai <test_key>` 走 MinioVault PUT 成功
- [ ] (T6) `gitai key get openai` 走 MinioVault GET 读回原 key
- [ ] (T6) `gitai key delete openai` 走 MinioVault DELETE 成功（versioning 留 90d）
- [ ] (T6) minIO 不可达时自动降级 FileVault（fallback 路径验证）
- [ ] **不引入** minIO 官方 Go SDK 跨语言依赖（仅纯 Rust `s3` crate）
- [ ] 烟测 `scripts/smoke.ps1` 仍 6 秒 PASS（不退化）
- [ ] 单测 `cargo test` 5/5 仍过（V0 阶段不破坏 gitgit MVP 测试基线）

> `[FACT]` **2026-10-25 修订注记**：上面三条 `(T11)` 验收项写的是当时那套夹具
> 的命令（`docker compose` / `mc`），那套夹具已经退场。它们的**目的**没有变，
> 载体换了：现在由 `scripts/verify-s3-vault.ps1` 对着自建服务端逐条验证——
> 服务端能起、桶已建且版本化已开、桶非公开（未签名请求 403）、产品二进制
> `gitgit key set/get/list-keys/versions/delete` 全链路跑通、两条 vault 端到端
> 测试真的执行并通过。

## 6. 迁移路径

### 6.1 V0 阶段（当前）

```
FileVault (fallback)  ← T11 minIO 部署前
  ↓
MinioVault (默认)     ← T11 部署后
```

### 6.2 V1+ 阶段（待规划）

```
MinioVault (默认)
  ↓
  ├─ KMS / Secret Manager 接入 (解递归引用 root 凭证)
  ├─ 跨节点 site replication (备份)
  └─ HTTPS 强制 (cert-manager)
  ↓
  若商业化：评估切换 Ceph RGW / SeaweedFS / Garage
```

> `[FACT]` **2026-10-25 修订注记**：上面两条路径都已经不是现状。服务端换成了
> 本仓自建（§1 修订注记），类型名换成了 `S3Vault`，而「若商业化：评估切换
> Ceph RGW / SeaweedFS / Garage」这一条**已经被走过一遍**——评估结果是自建，
> 不是那三个。KMS / Secret Manager 与强制 HTTPS 仍未做。

## 7. 参考 / References

- [ADR-0020](0020-v0-gui-tauri-svelte.md) §2.5 Credential Vault + §2.5.1 部署要求
- [v0-tasks.md](../plan/v0-tasks.md) 任务 6 (Credential Vault) + 任务 11 (minIO 部署)
- [V0-MINIO-MIGRATION-REPORT.md](../reports/2026-08-30-minio-migration/V0-MINIO-MIGRATION-REPORT.md) v0.1 8/30 15:42 JST 迁移报告
- minIO 官方文档：https://min.io/docs/minio/linux/index.html
- minIO AGPL-3.0 license：https://github.com/minio/minio/blob/master/LICENSE
- `s3` crate (pure-Rust AWS Signature V4)：https://docs.rs/s3/latest/s3/
- 8/19 archived `D:/GitGit/docs_archive_rust_impl_2026_08_26/architecture/decisions/0007-vault.md`（HashiCorp Vault 旧方案，仅备忘）

---

## 8. 修订历史 / Revision History

| 版本 | 日期 (JST) | 改动 |
|---|---|---|
| **Accepted** | 2026-08-30 15:42 | 初始 Accepted：Credential Vault 默认实现改为 S3 兼容 bucket `gitgit-vault`，`FileVault` 降级 fallback |
| **Amended** | 2026-10-25 | 决策不变，**服务端从 minIO 换成本仓自建**（AGPLv3 退场）。新增 §1 修订注记、§2.2 自建一行、§2.1 理由三的修订注记、§4.2 风险处置、§5 验收载体、§6.2 现状。类型改名 `MinioVault` → `S3Vault`，夹具换成 `deploy/objectstore/`，两条 vault e2e 摘掉 `#[ignore]`。历史决策文本不改写 |

---

**Status: Accepted** | 2026-08-30 15:42 JST
**Amended** | 2026-10-25 — 服务端从 minIO 换成本仓自建（§1 修订注记）
