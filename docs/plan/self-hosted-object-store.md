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

## 为什么必须做

`[FACT]` minIO 服务端是 GNU AGPLv3（它自己的 `--version` banner 就这么写，见
`deploy/minio/docker-compose.yml` 的注释）。本仓以 HTTP 方式消费它。

`[FACT]` `deploy/minio/docker-compose.yml` 自己的开头写着：那是**开发与测试夹具**，
**不是出货组件**，产品运行时不依赖它（出货路径是 `.gitgit/vault` 下的 FileVault），
S3 那条路是可选的，靠 `GITGIT_MINIO_*` 环境变量打开。

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
| 建桶 | （夹具脚本 `mc mb`） | `PUT /{bucket}` → 200 |
| 桶版本化 | （夹具脚本 `mc version enable`） | `PUT /{bucket}?versioning` 与 `GET /{bucket}?versioning` |

`[FACT]` **`x-amz-version-id` 是承重的，不是装饰**：`src/server/vault_versioned.rs` 的
`VaultVersionSummary` 把它记在时间线里；桶没开版本化，这个头就没有，每个凭据的
`version_id` 都会是空的（`deploy/minio/docker-compose.yml` 里那两行注释说的就是这件事）。

`[FACT]` **两种 SigV4 变体都要验**：

1. **header 签名** —— `rust-s3` 对每个请求算的 `Authorization: AWS4-HMAC-SHA256 …`；
2. **query 签名** —— `presign_get` 把签名放进 **query string**（`X-Amz-Algorithm`、
   `X-Amz-Credential`、`X-Amz-Date`、`X-Amz-SignedHeaders`、`X-Amz-Signature`），
   然后用 `reqwest` 直接 GET。`[FACT]` `reqwest` 因此是本 crate 的**直接依赖**——
   注释里写明理由就是这条。

`[FACT]` 桶的创建与版本化启用，目前由夹具里的 `mc`（minIO 自己的客户端）做。换成自建
服务端后，这两个动作由**自带的引导流程**做，不再需要 `mc`。

## 架构

`[FACT]` 本仓是**单 crate**：`Cargo.toml` 里 `name = "gitgit"`，一个 lib（`src/lib.rs`）
一个 bin（`src/main.rs`）。HTTP 栈是 **axum 0.7** + tokio + tower-http。

`[PROPOSAL]` 落点：

- `src/objectstore/` —— lib 内的模块：`store.rs`（存储层）、`http.rs`（axum 路由）、
  `sigv4.rs`（签名校验）、`xml.rs`（ListObjectsV2 的 XML 输出）。
- `src/bin/gitgit-objectstore.rs` —— 第二个 bin target，跑这个服务。
  `Cargo.toml` 加一条 `[[bin]]`。

`[PROPOSAL]` **不新增 crate**。理由与仓里既有的做法一致：新增依赖要先过
`scripts/frontend-advisory-check.ps1` 那一类的审计，而且 SigV4 校验的**正确性**最好由
本仓自己用**真实客户端**证明，而不是由第三方 crate 的语义决定。

`[PROPOSAL]` **后端用文件系统**，目录形如
`<root>/<bucket>/<version-id>/<key 的 sha256>.bin` + 同名 `.json`（键名、长度、
content-type、创建时间、etag）。`[INFERENCE]` 凭据 vault 的键数量很小，列举走扫描
是够用的；这条取舍要写进文档，别让它变成隐含假设。

## 正确性怎么证明

`[PROPOSAL]` 三层，缺一不可：

1. **存储层单元测试** —— put/get/delete/list/版本化的语义，含边界（空对象、含
   特殊字符的键、删除后读、版本号唯一性）。
2. **SigV4 校验的测试向量** —— 用 AWS SigV4 规范里那组公开的已知
   （密钥、请求、期望签名），再加**用真实 `rust-s3` 客户端签一次、服务端验一次**的
   往返测试。`[INFERENCE]` 往返测试才是证据：规范向量证明公式对，往返证明**我们的
   客户端和我们的服务端说的是同一件事**。
3. **端到端** —— 启动本仓的服务端，把 `MinioVault` 指向它，跑**现有的 vault 测试套件**
   （`src/server/vault*.rs` 的测试）——那套测试是针对真实 S3 语义写的，指向自建服务端
   之后必须**一字不改**地通过。

`[FACT]` 这一条是本 lane 的验收核心：**门禁必须被喂过失败输入才算数**。所以「服务端
起不来」「签名算错」「列举格式不对」这三种失败都要**实测发生过**，不能只跑通路径。

## 迁移（服务端落地之后，另一步）

`[FACT]` 届时会随 minIO 一起退场的名字：

- `MinioVault` → （S3 客户端的名字；它一直是 S3 客户端，不该以某个服务端命名）
- `GITGIT_MINIO_*` 环境变量 → 对应的新名字
- `deploy/minio/docker-compose.yml` → 自建服务端的启动方式
- `scripts/verify-minio-vault.ps1` → 同上

`[INFERENCE]` 名字跟着依赖一起改，和本仓一贯的做法一致（`名字像许可证 ≠ 许可证`、
`过时记述比没有记述更糟`）。`[FACT]` 但 **client 侧的行为不许变**：同一份
`rust-s3` 调用、同样的 path-style、同样的 `x-amz-version-id` 语义。

## 明确不做

- **不改** `rust-s3` 依赖、**不改** `VersionedVault` trait、**不改** ADR-0021 的架构决定。
- **不**实现 S3 的完整 API（分片上传、ACL、预签名 PUT、复制、条件请求、多桶、加密
  头）。`[INFERENCE]` 只实现本仓客户端真正发出的那七类操作；多出来的是没人用的
  负担，也是没人验证过的负担。
- **不**把自建服务端做成通用 S3 服务对外承诺兼容。`[FACT]` 它的验收标准是
  「本仓的客户端能正确使用它」，不是「通过第三方 S3 合规测试套件」。

## 未闭合的风险

- `[UNVERIFIED-FACT]` 本仓客户端与服务端之间的 SigV4 语义是否**完全**一致，要靠
  第 2 层和第 3 层的往返测试证明，不能靠读规范断言。
- `[FACT]` 服务端落地之前，风险登记里那条 minIO AGPL 风险**仍然开着**，
  **未采购即不得对外分发**的约束不变。
- `[FACT]` 本仓自身按 AGPL-3.0 发布（`LICENSE` 是 AGPL-3.0 全文，两个 manifest 都写
  `license = "AGPL-3.0"`）。**自建解决的是「依赖一个第三方 AGPL 服务端」，不是
  「本仓自己的许可证」** —— 后者是所有者自己的选择，不变。
