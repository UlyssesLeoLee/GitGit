# 自建对象存储：把 minIO 换成自己的 S3 兼容服务端

> `[FACT]` 本文所有事实由实测与官方文档得到。标注约定：`[FACT]` 实测或文档原文；
> `[INFERENCE]` 由事实推出；`[PROPOSAL]` 建议；`[UNVERIFIED-FACT]` 已知但未验证。

## 结论先行

`[FACT]` **minIO Enterprise License 采购不成**（2026-10-25 决定）。`[INFERENCE]` 选
**自建**：本仓自己实现一个 **S3 兼容的对象存储服务端**，替代 minIO。

`[FACT]` **形态不变**：ADR-0021 的「S3 兼容 bucket `gitgit-vault` + S3 客户端作为默认
实现、FileVault 降级为 V1 fallback」这条架构**不动**。要自己写的只是**服务端**；
客户端（`rust-s3` 0.38）照旧，所以**凭据管理与版本化逻辑不需要重做**。

`[FACT]` 这是与决策日志里另一条备选（「改用 Ceph RGW / SeaweedFS / Garage」）的**最大
差别**：那三条路都要重做客户端侧的东西，因为它们的 S3 语义与 minIO 并不完全一致；
自建则可以把服务端做到**恰好**满足本仓客户端发出的请求。

## 现状：M1 / M2a / M2b 各自做了什么

`[FACT]` 三条线按顺序落地（截至 2026-10-25）：

| 线 | 做了什么 | 现在的状态 |
| --- | --- | --- |
| **M1**（PR #67） | 实现本仓的 S3 兼容对象存储服务端：`src/objectstore/`（`store.rs` / `http.rs` / `sigv4.rs` / `xml.rs`）+ `gitgit-objectstore` 二进制 | 客户端实际发出的那七类操作全部实现并有测试；`--bootstrap-bucket` 建桶并开版本化，替代了 `mc mb` + `mc version enable` |
| **M2a** | 让 credential vault 的后端**可选**：`GITGIT_VAULT_BACKEND=file\|s3`（或 `--vault-backend`），S3 设置读 `GITGIT_S3_ENDPOINT` / `_BUCKET` / `_REGION` / `_KEY_PREFIX` / `_ACCESS_KEY` / `_SECRET_KEY` | 产品二进制第一次能走到 S3 那条路 |
| **M2b**（本线） | **minIO 退场**：改名 `MinioVault` → `S3Vault`、`MinioVaultConfig` → `S3VaultConfig`、旧的 `GITGIT_MINIO_*` → `GITGIT_S3_*`；夹具换成 `deploy/objectstore/`（无 `mc`、无 docker-compose）；两条 vault e2e 摘掉 `#[ignore]`，自己在进程内起服务端 | S3 那条路被常规测试套件与 CI 持续覆盖，不再依赖任何手工夹具 |

`[FACT]` **客户端一侧没有动**：`rust-s3` 0.38 客户端、`VersionedVault` trait、
ADR-0021 的架构决定、凭据管理与版本化逻辑，全部保持原样。这正是「只自己写服务端」
这条路的收益——也是本仓一贯的做法（`名字像许可证 ≠ 许可证`）。

## 为什么必须做

`[FACT]` minIO 服务端是 GNU AGPLv3（它自己的 `--version` banner 就这么写；当时的
夹具 `deploy/minio/docker-compose.yml` 开头就写着这一点，那份文件已随 minIO 一起
退场）。本仓以 HTTP 方式消费它。

`[FACT]` 当时的 `deploy/minio/docker-compose.yml` 开头写着：那是**开发与测试夹具**，
**不是出货组件**，产品运行时不依赖它（出货路径是 `.gitgit/vault` 下的 FileVault），
S3 那条路是可选的，靠环境变量打开。

`[INFERENCE]` 于是这里要消除的不是「运行期依赖」，而是**发行物里带着一个 AGPL 服务端**
这件事本身。把它换成本仓自己的代码，风险随依赖一起消失。

## 要实现的 S3 面（**实测钉死**，不是估计）

`[FACT]` 客户端是 `rust-s3` 0.38（`Cargo.toml`），`Region::Custom { region, endpoint }`
+ `with_path_style()` → **path-style 寻址**（`http://host:port/{bucket}/{key}`）。
`[FACT]` 客户端实际发出的请求只有这些：

| 用途 | 调用 | 服务端要能处理 |
| --- | --- | --- |
| 写入 | `bucket.put_object(key, bytes)` | `PUT /{bucket}/{key}` → 200，**响应头带 `x-amz-version-id`** |
| 读取 | `bucket.get_object(key)` | `GET /{bucket}/{key}` → 200/404，**响应头带 `x-amz-version-id`** |
| 按版本读 | `bucket.presign_get(key, 60, Some({"versionId": …}))` | `GET /{bucket}/{key}?versionId=X` → 200/404 |
| 删除 | `bucket.delete_object(key)` | `DELETE /{bucket}/{key}` → 204 |
| 列举 | `bucket.list(…)` | `GET /{bucket}?list-type=2&prefix=…&continuation-token=…&max-keys=…` → ListObjectsV2 的 XML |
| 建桶 | （服务端自己的 `--bootstrap-bucket`） | `PUT /{bucket}` → 200 |
| 桶版本化 | （服务端自己的 `--bootstrap-bucket`） | `PUT /{bucket}?versioning` 与 `GET /{bucket}?versioning` |

`[FACT]` **`x-amz-version-id` 是承重的，不是装饰**：`src/server/vault_versioned.rs` 的
`VaultVersionSummary` 把它记在时间线里；桶没开版本化，这个头就没有，每个凭据的
`version_id` 都会是空的。当时的夹具里那两行注释说的就是这件事。

`[FACT]` **两种 SigV4 变体都要验**：

1. **header 签名** —— `rust-s3` 对每个请求算的 `Authorization: AWS4-HMAC-SHA256 …`；
2. **query 签名** —— `presign_get` 把签名放进 **query string**（`X-Amz-Algorithm`、
   `X-Amz-Credential`、`X-Amz-Date`、`X-Amz-SignedHeaders`、`X-Amz-Signature`），
   然后用 `reqwest` 直接 GET。`[FACT]` `reqwest` 因此是本 crate 的**直接依赖**——
   注释里写明理由就是这条。

`[FACT]` 桶的创建与版本化启用原先由夹具里的 `mc`（minIO 自己的客户端）做。现在这两个
动作由**自建服务端自己的 `--bootstrap-bucket`** 做，不再需要 `mc`——
`deploy/objectstore/` 里连 docker-compose 也没有了，夹具少掉一个外部依赖。
## 架构

`[FACT]` 本仓是**单 crate**：`Cargo.toml` 里 `name = "gitgit"`，一个 lib（`src/lib.rs`）
一个 bin（`src/main.rs`）。HTTP 栈是 **axum 0.7** + tokio + tower-http。

`[FACT]` 落点（M1 已落地）：

- `src/objectstore/` —— lib 内的模块：`store.rs`（存储层）、`http.rs`（axum 路由）、
  `sigv4.rs`（签名校验）、`xml.rs`（ListObjectsV2 的 XML 输出）。
- `src/bin/gitgit-objectstore.rs` —— 第二个 bin target，跑这个服务。
  `Cargo.toml` 加了一条 `[[bin]]`。

`[FACT]` **不新增 crate**。理由与仓里既有的做法一致：新增依赖要先过
`scripts/frontend-advisory-check.ps1` 那一类的审计，而且 SigV4 校验的**正确性**最好由
本仓自己用**真实客户端**证明，而不是由第三方 crate 的语义决定。

`[FACT]` **后端用文件系统**，目录形如
`<root>/<bucket>/<version-id>/<key 的 sha256>.bin` + 同名 `.json`（键名、长度、
content-type、创建时间、etag）。`[INFERENCE]` 凭据 vault 的键数量很小，列举走扫描
是够用的；这条取舍要写进文档，别让它变成隐含假设。

## 正确性怎么证明

`[FACT]` 三层，缺一不可（**已全部落地并实测**）：

1. **存储层单元测试** —— put/get/delete/list/版本化的语义，含边界（空对象、含
   特殊字符的键、删除后读、版本号唯一性）。
2. **SigV4 校验的测试向量** —— 用 AWS SigV4 规范里那组公开的已知
   （密钥、请求、期望签名），再加**用真实 `rust-s3` 客户端签一次、服务端验一次**的
   往返测试。`[INFERENCE]` 往返测试才是证据：规范向量证明公式对，往返证明**我们的
   客户端和我们的服务端说的是同一件事**。
3. **端到端** —— 启动本仓的服务端，把 `S3Vault` 指向它，跑 vault 的测试套件
   （`src/server/vault*.rs` 的测试）——那套测试是针对真实 S3 语义写的，指向自建服务端
   之后必须**一字不改**地通过。

`[FACT]` 这一条是本 lane 的验收核心：**门禁必须被喂过失败输入才算数**。所以「服务端
起不来」「签名算错」「列举格式不对」这三种失败都要**实测发生过**，不能只跑通路径。

`[FACT]` **M2b 补上的最后一层**：两条 vault 端到端测试原本是 `#[ignore]` 的，
因为它们需要一个活的 S3 服务端。现在服务端是本仓的，测试可以自己在**进程内**把它
起起来——监听**端口 0**（让操作系统选，避免并行测试撞端口），桶**显式建出来并开
版本化**（`x-amz-version-id` 只在版本化桶上才有，而 `VaultVersionSummary` 依赖它）。
两条测试因此摘掉了 `#[ignore]`，在**不带 `--ignored`** 的普通 `cargo test` 里真的跑。

`[FACT]` 验收口径是「**不带 `--ignored` 也要真的跑**」：一条静默跳过的门禁不是门禁。

## 迁移（已完成）

`[FACT]` 随 minIO 一起退场的名字，已在 M2b 全部换掉：

| 退场前 | 现在 |
| --- | --- |
| `MinioVault` | `S3Vault`（它一直是 S3 客户端，不该以某个服务端命名） |
| `MinioVaultConfig` | `S3VaultConfig` |
| `GITGIT_MINIO_ENDPOINT` / `_BUCKET` / `_ACCESS_KEY` / `_SECRET_KEY` | `GITGIT_S3_*`（M2a 已占用这六个名字；M2b 删掉旧的） |
| `deploy/minio/docker-compose.yml` | `deploy/objectstore/`（起脚本 + 说明，无 `mc`、无 docker-compose） |
| `scripts/verify-minio-vault.ps1` | `scripts/verify-s3-vault.ps1` |

`[INFERENCE]` 名字跟着依赖一起改，和本仓一贯的做法一致（`名字像许可证 ≠ 许可证`、
`过时记述比没有记述更糟`）。`[FACT]` 但 **client 侧的行为不许变**：同一份
`rust-s3` 调用、同样的 path-style、同样的 `x-amz-version-id` 语义。

## 明确不做

`[FACT]` 以下三条**已守住**，M2b 没有碰：

- **不改** `rust-s3` 依赖、**不改** `VersionedVault` trait、**不改** ADR-0021 的架构决定。
- **不**实现 S3 的完整 API（分片上传、ACL、预签名 PUT、复制、条件请求、多桶、加密
  头）。`[INFERENCE]` 只实现本仓客户端真正发出的那七类操作；多出来的是没人用的
  负担，也是没人验证过的负担。
- **不**把自建服务端做成通用 S3 服务对外承诺兼容。`[FACT]` 它的验收标准是
  「本仓的客户端能正确使用它」，不是「通过第三方 S3 合规测试套件」。

## 未闭合的风险

- `[FACT]` ~~SigV4 语义是否**完全**一致，要靠第 2 层和第 3 层的往返测试证明~~ ——
  **已闭合**：M1 的往返测试与 M2b 的两条端到端测试都是绿的，且端到端那条走的是
  **真实 `rust-s3` 客户端**。
- `[FACT]` ~~服务端落地之前，风险登记里那条 minIO AGPL 风险**仍然开着**，
  **未采购即不得对外分发**的约束不变~~ —— **已闭合**：本仓不再消费任何第三方
  AGPL 服务端，服务端就是本仓自己的代码。风险登记里的那一行已改为「已处置」。
- `[FACT]` 本仓自身按 AGPL-3.0 发布（`LICENSE` 是 AGPL-3.0 全文，两个 manifest 都写
  `license = "AGPL-3.0"`）。**自建解决的是「依赖一个第三方 AGPL 服务端」，不是
  「本仓自己的许可证」** —— 后者是所有者自己的选择，不变。
- `[FACT]` **仍然开着**的两条：① 自建服务端是**文件系统**后端，跨节点共享要靠共享
  文件系统，ADR-0021 当初许诺的「跨节点原生访问」在 V0 范围内没有兑现；
  ② 强制 HTTPS、KMS / Secret Manager 仍未做（ADR-0021 §6.2）。
