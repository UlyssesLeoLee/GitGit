# 技术选型文档 / Technology Selection

> **[PROPOSAL] 状态:** Accepted (2026-08-19)
>
> **决策人:** 工程负责人（对应需求定义书 §53 ADR 列表项 11）
>
> **替代文档:** 本文档替代了 [00-requirements-definition.md §53 第 11 项](../requirements/00-requirements-definition.md) 标记的开放项"Implementation language/framework choice (Rust/Tokio/Axum vs. alternatives)"，并对应 [phase10-architecture.md §2 §6 §7](../requirements/phase10-architecture.md) 标记为 `[TBD]` 的语言/框架决策。
>
> **本选型与 IPA 框架的关系:** 本决策不改变 IPA 共通框架 2013 的过程结构（仍 P3-P9），仅是 P6 软件构建过程的实现选择。

---

## 0. 决策摘要 / Decision Summary

| 维度 | 决策 | 备选 | 状态 |
|---|---|---|---|
| **主语言** | **Rust（edition 2021，最低 MSRV 1.75）**| Go 1.22+ | 已选定 |
| **异步运行时** | **Tokio 1.x（多线程 runtime）**| smol / async-std | 已选定（Rust 决策的后果）|
| **Web 框架** | **Axum 0.7+（基于 hyper + tower）**| actix-web / Rocket | 已选定 |
| **gRPC 框架** | **Tonic 0.12+**| grpc-rs | 已选定（用于内部服务间通信）|
| **PostgreSQL 驱动** | **sqlx 0.8（compile-time checked queries）**| tokio-postgres / diesel | 已选定 |
| **PostgreSQL 连接池** | sqlx 自带 / deadpool-postgres | bb8 / deadpool | sqlx 自带足够 |
| **图遍历查询** | **原生 sqlx + 递归 CTE**（PostgreSQL 处理）| Memgraph 独立部署 | 拒绝（违反"能 PG 解决不增加"原则）|
| **Git 协议库** | **gix (gitoxide) 0.66+**（Rust 原生）| libgit2 绑定 / shell git CLI | 已选定（Rust 原生 + 性能优势）|
| **Git CLI 调用** | 写路径（pack/negotiation）继续 shell `git` 进程 | 全部走 gix | 强约束：禁止为"纯 Rust"重写高风险 Git 设施（[phase10-architecture.md §6 第 2 项](../requirements/phase10-architecture.md)）|
| **本地 Git 进程** | tokio::process::Command 异步 spawn | std::process::Command | 已选定（Tokio 集成）|
| **认证 / 凭证** | **jsonwebtoken 9.x**（JWT 签发/验证）| josekit | 已选定 |
| **密码哈希** | **argon2 0.5+**（Argon2id，OWASP 推荐）| bcrypt | 已选定（更安全 + OWASP 2023 推荐）|
| **加密原语** | **RustCrypto 套件**（aes-gcm / chacha20poly1305 / sha2 / hmac）| ring / openssl | 已选定（纯 Rust + 审计友好）|
| **HTTP 客户端** | **reqwest 0.12+**（异步 + TLS）| hyper 直接 | 已选定 |
| **Telegram 风格配置** | **figment + serde_yaml**（配置 + 校验）| config-rs | 已选定 |
| **结构化日志** | **tracing 0.1+ + tracing-subscriber**| slog / log | 已选定（与 OTel 集成最佳）|
| **可观测性** | **opentelemetry 0.24+ + opentelemetry-otlp 0.17+**| 其它 | 已选定 |
| **Prometheus 指标导出** | **metrics 0.23+ + metrics-exporter-prometheus 0.15+**| prometheus crate 直写 | 已选定 |
| **数据库迁移** | **sqlx-migrate 0.8+**（sqlx 自带）| refinery / sqlx-cli | 已选定 |
| **JSON Schema 校验** | **jsonschema 0.18+**| valico | 已选定（用于 App Manifest / 事件 Schema 校验）|
| **UUID** | **uuid 1.x（含 v7）**| nanoid | 已选定（v7 时间排序 + 128 位）|
| **时区 / 时间** | **chrono 0.4+**（PostgreSQL TIMESTAMPTZ 友好）| time | 已选定 |
| **正则** | **regex 1.x**（lazy_static + RegexSet 缓存）| fancy-regex | 已选定 |
| **压缩** | **flate2 + tar + zstd**（多种算法）| 单一 | 已选定 |
| **CLI 解析** | **clap 4.x（derive 模式）**| structopt | 已选定（合并到 clap）|
| **错误处理** | **thiserror 1.x + anyhow 1.x**| 自定义 | 已选定（thiserror 用于库，anyhow 用于二进制）|
| **异步通道** | tokio::sync::mpsc / tokio::sync::watch | crossbeam | 已选定（Tokio 生态）|
| **序列化** | **serde 1.x + serde_json / serde_yaml / bincode**| rmp-serde | 已选定 |
| **MCP 协议实现** | 自实现（规范简单，~500 行）+ reqwest | rmcp | 已选定（保持依赖最小）|
| **App Plugin 沙箱** | OCI containerd（Agent workspace）/ Wasmtime（V1+ 可选）| Docker 唯一 | 已选定（容器标准 + V1+ 沙箱选项）|
| **密钥管理（生产）** | **HashiCorp Vault**（外部 secret store）| 仅 env / 文件 | 已选定（V1+ Cloud）|
| **本地开发密钥** | 文件（OS 文件权限 0600）| Vault | MVP 简化为文件 |
| **PL/pgSQL 存储过程** | **直接写在 migration 文件里**（sqlx 管理版本）| 独立 PL 工具 | 已选定 |
| **消息队列（V1+ Cloud）** | **PostgreSQL LISTEN/NOTIFY + 现有 outbox**（不引入新组件）| NATS / Redis Streams | 拒绝（违反"能 PG 解决"原则）|

**关键拒绝项：**

1. **Memgraph 独立部署** — 拒绝。理由：与"能 PostgreSQL 解决不增加"原则冲突；图遍历用 PG 递归 CTE 已足够（[phase10-architecture.md §6 决策矩阵](../requirements/phase10-architecture.md)）
2. **NATS JetStream** — 拒绝。理由：中心事件总线用 PG Outbox + LISTEN/NOTIFY 即可（[§13.4](../design/basic-design/13-app-cluster-and-plugins.md#134-中心事件总线-central-event-bus)）；不引入新组件
3. **MinIO / S3** — 拒绝（暂缓到 V1+）。理由：MVP 本地文件系统足够；V1+ Cloud 才评估 S3 兼容对象存储
4. **Valkey / Redis** — 拒绝。理由：决策缓存用 in-process LRU（[§2.9](../design/detailed-design/02-graph-engine.md#29-缓存-caching)）；不引入独立缓存服务
5. **自实现 Git 协议** — 强约束禁止。理由：phase10-architecture.md §6 明确"禁止为 Rust 原则重复实现成熟高风险 Git 基础设施"；写路径（pack/negotiation）继续 shell `git` 进程
6. **独立 Admin 进程用不同语言** — 拒绝。理由：所有进程统一 Rust；Admin UI 前端是 SvelteKit 独立（与 §14 一致）

---

## 1. 上下文与约束 / Context & Constraints

### 1.1 需求侧（不可违背）

来自 [`00-requirements-definition.md`](../requirements/00-requirements-definition.md)：

- **[FACT]** 平台 37 项 MVP 需求**无一项**对实现语言有强约束（§53 第 11 项明文："no MVP requirement is language-load-bearing"）
- **[PROPOSAL]** AISEC-REQ 集合要求进程处理不可信 Agent 输入时具备**强内存安全保证**（避免 RCE 类漏洞）
- **[PROPOSAL]** 单一进程承载 Policy 评估、凭证签发、沙箱 spawn、审计 Event 写入（[phase10-architecture.md §4.1](../requirements/phase10-architecture.md)），这 4 个能力被 AISEC-REQ-001〜008 共同假设"可信"
- **[FACT]** Phase 10 §6 候选栈为 `Rust/Tokio/Axum/gRPC/PostgreSQL/pgvector/Memgraph/MinIO-S3/Valkey/NATS JetStream/K3s-Kubernetes/OpenTelemetry`

### 1.2 架构侧（强约束）

来自 [`phase10-architecture.md`](../requirements/phase10-architecture.md)：

- **架构输入 §2 §6 §7** — Rust/Tokio/Axum 标记为 `[TBD]`，需 ADR 决策
- **架构输入 §6 第 2 项** — 明确禁止"为 Rust 原则重复实现成熟高风险 Git 基础设施" → 写路径必须用 shell `git`，不能用纯 Rust 库替代
- **架构输入 §7 第 11 项** — Implementation language/framework 列为开放项 11
- **架构输入 §7 第 12 项** — `git` CLI vs `libgit2`/`gix` 边界开放项 12

### 1.3 设计侧（已落地约束）

来自 [`basic-design/00-introduction.md` §0.5](../design/basic-design/00-introduction.md) 设计原则：

- **能单体解决，不提前微服务化** — 拒绝引入 Memgraph / NATS / Valkey 等独立服务
- **能 PostgreSQL 解决，不提前增加数据库** — 拒绝 Memgraph；图遍历用 PG 递归 CTE
- **能事件解决，不直接形成服务耦合** — 中心事件用 PG Outbox 而非 NATS
- **能标准协议解决，不发明私有协议** — 强约束；Git 用标准协议；HTTP 用标准 REST

### 1.4 平台特征（隐含约束）

- Local-First 单进程 → 库体积小、启动快、二进制小
- 长生命周期（生产部署持续运行） → 内存安全 + 错误处理健全
- 处理不可信内容（Agent 拉取的 PR / Issue 评论） → 内存安全保证
- 中心事件总线高频（100+ events/s 单 Relay） → 异步 + 高并发

---

## 2. 主语言决策：Rust / Language Decision

### 2.1 决策

**[ACCEPTED]** 主语言为 **Rust（edition 2021，最低 MSRV 1.75）**。

理由按重要性排序：

1. **内存安全 + 无 GC** — 处理不可信 Agent 输入（AISEC-REQ-001 提示词注入防御边界）时，Rust 的编译期保证消除整类 RCE 漏洞（use-after-free / buffer overflow / data race）；Go 的 GC 边界外仍需手工审计。Phase 10 §2 明文："Memory safety without a GC matters for a process handling untrusted agent input (AISEC-REQ set) and long-running Git/CI workloads."

2. **生态成熟度** — Rust 的 PostgreSQL（sqlx）、Git（gix / libgit2 绑定）、Web（Axum）、gRPC（Tonic）、加密（RustCrypto）等核心库已生产稳定；Tokio 异步生态成熟度足够

3. **二进制小 + 启动快** — Local-First 单进程 + 冷启动 ≤ 5s 的目标下，Rust 单二进制 ~10-30MB vs Go ~30-50MB

4. **类型系统表达力** — Phase 10 §6 注："更适合生产加固"（vs Go "更适合 MVP 快速交付"）。我们选长期主义

5. **错误处理** — `Result<T, E>` 类型强制 + `?` 操作符 + `thiserror` 库让错误传播显式可控，符合平台 AISEC-REQ 集合对"错误不吞"的隐含要求

### 2.2 拒绝的方案

**Go 1.22+** — 拒绝原因：

- GC 边界外仍有内存安全隐患（虽然比 C++ 小很多）
- Phase 10 §2 已注明 "Go (simpler concurrency model, faster iteration for MVP, large ecosystem for HTTP/gRPC/Postgres)" 是快速 MVP 优势
- 本项目目标不是 6 周交付 MVP，而是建立"长期可信 + 可加固"的基础；Rust 的学习曲线一次性投入，长期收益高
- 选 Go 等于在 AISEC-REQ 的内存安全诉求上做妥协

**C++ 20** — 拒绝原因：

- 内存安全完全靠人工审计
- 与"处理不可信 Agent 输入"的 AISEC-REQ 集合完全不兼容
- 编译时间 + 构建系统复杂度不适合迭代

**TypeScript (Node.js) / Bun / Deno** — 拒绝原因：

- 单线程事件循环无法利用多核（虽然有 worker_threads）
- npm 依赖链的供应链安全风险
- 平台需处理二进制 Git 对象（pack 文件），TS 不擅长

---

## 3. 运行时与异步：Tokio / Async Runtime

### 3.1 决策

**[ACCEPTED]** 异步运行时为 **Tokio 1.x（multi-threaded runtime）**。

### 3.2 选型依据

- 唯一与 Axum / Tonic / sqlx / tokio-postgres 生态深度集成的异步运行时
- `tokio::spawn` + `tokio::select!` + `tokio::time` 让并发模型清晰
- 支持 graceful shutdown（`tokio::signal::ctrl_c()` + `CancellationToken`）

### 3.3 拒绝的方案

- **smol / async-std** — 生态小，与 Axum/Tonic 集成差
- **直接用 std::thread + channels** — 与 axum/tonic 不兼容，工作量翻倍

### 3.4 关键约定

```rust
// 所有异步函数返回 Result<T, PlatformError>
async fn install_app(manifest: Manifest) -> Result<AppId, PlatformError> {
    // ...
}

// 关键路径禁止 .unwrap()，用 ? 传播
let app = nodes::find(id).await?;  // 失败立刻返回

// 长任务用 CancellationToken
let token = CancellationToken::new();
let task = tokio::spawn(async move {
    loop {
        tokio::select! {
            _ = token.cancelled() => break,
            _ = tick.tick() => do_work().await?,
        }
    }
    Ok::<_, PlatformError>(())
});
```

---

## 4. Web 框架：Axum / HTTP Framework

### 4.1 决策

**[ACCEPTED]** HTTP 框架为 **Axum 0.7+**（基于 hyper + tower）。

### 4.2 选型依据

- 由 tokio 团队直接维护，与 Tokio 生态深度集成
- `tower::Layer` 中间件生态（认证、限流、追踪、压缩、CORS）丰富
- 类型安全路由（`/api/v1/repos/:repo_id`，编译器检查参数类型）
- 与 Tonic（gRPC）可共享错误类型（`Into<axum::response::Response>` for tonic）

### 4.3 拒绝的方案

- **actix-web** — 性能略高但与 Tokio 生态隔离；其自带的 actix-rt 与 tokio 互操作麻烦
- **Rocket** — 0.5 之前与 Tokio 集成差；同步优先
- **hyper 直接** — 太底层；重复造中间件

### 4.4 关键中间件栈

```rust
let app = Router::new()
    .route("/api/v1/*path", api_v1_routes())
    .route("/api/apps/:app_id/*path", app_routes())
    .route("/admin/v1/*path", admin_v1_routes())
    .route("/mcp/*path", mcp_routes())
    .layer(TraceLayer::new_for_http())              // tracing
    .layer(TimeoutLayer::new(Duration::from_secs(30)))
    .layer(CompressionLayer::new())
    .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024))  // 10MB
    .layer(AuthSessionLayer)                        // JWT 会话
    .layer(CorsLayer::permissive());                // CORS
```

---

## 5. gRPC：Tonic / Inter-Service RPC

### 5.1 决策

**[ACCEPTED]** gRPC 框架为 **Tonic 0.12+**。

### 5.2 选型依据

- Tokio 原生，Axum 同门（hyper 生态）
- 编译期类型检查（`.proto` → Rust 代码）
- 支持 HTTP/2 + TLS + 流式 RPC

### 5.3 内部 gRPC 服务清单

平台内部组件间 gRPC（不暴露公网）：

| 服务 | Proto 包 | 用途 |
|---|---|---|
| `event_relay.v1.Relay` | `event_relay.proto` | Event Relay ↔ Plugin Loader / App Instance |
| `plugin_loader.v1.Loader` | `plugin_loader.proto` | Plugin Loader ↔ 主 API 进程 |
| `admin.v1.Audit` | `admin_audit.proto` | 主进程 → Admin 进程（admin_audit 转发）|
| `agent.v1.Workspace` | `agent_workspace.proto` | Agent 运行时 ↔ 容器运行时 |

外部 HTTP API（`/api/v1/*` `/admin/v1/*`）走 Axum，不走 gRPC。

---

## 6. PostgreSQL 驱动：sqlx / Database Driver

### 6.1 决策

**[ACCEPTED]** PostgreSQL 驱动为 **sqlx 0.8+**（compile-time checked queries + 异步）。

### 6.2 选型依据

- **编译期 SQL 校验**：`sqlx::query!` 宏在编译时连接数据库校验 SQL 类型；编译失败立即报错
- **零运行时依赖**：不需 ORM 抽象；直接写 SQL（与本平台"PG 是 single source of truth"原则匹配）
- **async 原生**：`tokio::spawn` 友好
- **连接池内置**：`PgPool` 直接使用

### 6.3 拒绝的方案

- **tokio-postgres** — 需手写大量样板；连接池需 deadpool-postgres 组合
- **diesel / diesel-async** — ORM 抽象过重；与本平台"显式 SQL"哲学不符
- **sea-orm** — 同上

### 6.4 关键模式

```rust
// 编译期校验：DB schema 与 query 强一致
let row = sqlx::query!(
    r#"SELECT id, state FROM nodes WHERE type = 'app' AND id = $1"#,
    app_id
)
.fetch_optional(&pool)
.await?;

// 事务封装
let mut tx = pool.begin().await?;
sqlx::query!("INSERT INTO ...", ...).execute(&mut *tx).await?;
sqlx::query!("UPDATE ...", ...).execute(&mut *tx).await?;
tx.commit().await?;
```

### 6.5 migration

使用 `sqlx::migrate!()` 宏（编译期嵌入 migration 文件），运行期启动时自动迁移。

---

## 7. Git 库：gix（gitoxide）/ Git Protocol Library

### 7.1 决策

**[ACCEPTED]** 读路径用 **gix (gitoxide) 0.66+**（Rust 原生）。写路径**强约束**：继续用 shell `git` 进程（tokio::process::Command spawn）。

### 7.2 强约束（来自 phase10-architecture.md §6 第 2 项）

> 禁止为"纯 Rust"原则重复实现成熟、高风险的 Git 基础设施。

具体含义：

- **写路径（clone / push / fetch / GC）必须用 shell `git`** — pack 格式、negotiation、HTTP 协议细节复杂，gix 在这些路径上的成熟度不足
- **读路径（commit 遍历、tree 解析、blob 读取、log 格式化）用 gix** — 这些是不涉及 pack 协商的纯数据结构操作，gix 成熟且性能好

### 7.3 选型依据：gix vs libgit2

| 维度 | gix (gitoxide) | libgit2 绑定 |
|---|---|---|
| 语言 | 纯 Rust | C 库 + FFI |
| 性能 | 优秀（专为 Rust 优化）| 良好 |
| 维护活跃度 | 高（gitoxide 团队全职投入）| 中（上游 libgit2 慢）|
| 编译期集成 | 纯 cargo | 需要 cc + 系统 libgit2 |
| 安全性 | 纯 Rust 内存安全 | FFI 边界有内存安全风险 |
| 写路径支持 | 部分（[UNVERIFIED-FACT] 仍不够稳定）| 成熟 |
| 二进制大小 | 小 | 需要 libgit2.so/dylib/dll |
| 与 Tokio 集成 | 纯 Rust 友好 | 阻塞 C 调用需 spawn_blocking |

**结论**：读路径用 gix（纯 Rust + 高性能 + 内存安全 + Tokio 友好），写路径用 shell git（成熟 + 符合强约束）。

### 7.4 关键模式

```rust
// 读路径：gix
let repo = gix::open(repo_path)?;
let mut commit_walker = repo.rev_walk([commit_id]);
for commit_info in commit_walker.by_ref() {
    let commit = commit_info?;
    // 解析 tree、blob...
}

// 写路径：shell git
let output = tokio::process::Command::new("git")
    .args(["-C", repo_path, "push", "origin", branch])
    .output()
    .await?;
```

---

## 8. 认证 / 凭证：jsonwebtoken + argon2 / Auth

### 8.1 决策

- **JWT 签发/验证**：**jsonwebtoken 9.x**（HS256 / RS256，标准 JOSE）
- **密码哈希**：**argon2 0.5+**（Argon2id，OWASP 2023 推荐）
- **API Token 哈希**：SHA-256（plain text token 入库前哈希，与 GitHub PAT 模型一致）

### 8.2 选型依据

- `jsonwebtoken` 是 Rust 生态最广泛使用的 JOSE 库
- `argon2` 是 OWASP 推荐（替代 bcrypt / scrypt）
- 强约束：**终端 / Admin 双 JWT issuer + 独立签名密钥**（[§7.8](../design/basic-design/07-security-design.md#78-admin-独立鉴权域-admin-separate-auth-domain)）

---

## 9. 加密原语：RustCrypto / Cryptography

### 9.1 决策

**[ACCEPTED]** 加密原语使用 **RustCrypto 套件**：

| 用途 | 库 |
|---|---|
| AES-256-GCM（信封加密）| `aes-gcm` 0.10+ |
| ChaCha20-Poly1305（备选）| `chacha20poly1305` 0.10+ |
| SHA-256 | `sha2` 0.10+ |
| HMAC-SHA256 | `hmac` 0.12+ |
| Argon2id KDF | `argon2` 0.5+ |
| 随机数 | `rand` 0.8+（使用 `OsRng`）|

### 9.2 拒绝的方案

- **ring** — 非纯 Rust；BoringSSL 绑定；与 RustCrypto 兼容性差
- **openssl** — 系统库依赖；C FFI；与 Rust 异步生态集成麻烦
- **手写加密** — 强约束禁止（[§7.5](../design/basic-design/07-security-design.md#75-密钥管理-secrets-managementsec-req-005-sec-req-010)）

---

## 10. HTTP 客户端：reqwest / HTTP Client

### 10.1 决策

**[ACCEPTED]** HTTP 客户端为 **reqwest 0.12+**（异步 + native TLS + 异步流式 body）。

### 10.2 用途

- 平台对外 Webhook 出站（[§11.6](../design/basic-design/11-api-design.md#116-webhook-outbound-api)）
- AI Provider 调用（OpenAI / Anthropic / 自托管）
- 跨进程 HTTP（V1+ Cloud 内部）

---

## 11. 可观测性：tracing + OpenTelemetry / Observability

### 11.1 决策

| 维度 | 库 |
|---|---|
| 结构化日志 | `tracing` 0.1+ + `tracing-subscriber` 0.3+ |
| 分布式追踪 | `opentelemetry` 0.24+ + `opentelemetry-otlp` 0.17+ |
| 指标 | `metrics` 0.23+ + `metrics-exporter-prometheus` 0.15+ |
| 进程内 APM | `tracing-actix-web` / `tower-http::trace` |

### 11.2 关键约定

- 每个 span 必填字段：`app.id` / `app.version` / `app.instance_id` / `tenant.id` / `trace_id` / `span_id`（[§10.4.1](../design/detailed-design/10-observability.md#1041-业务-span-模板)）
- 关键操作 `tracing::info!` 入结构化日志
- 错误 `tracing::error!` + OTel span status = Error

---

## 12. 配置：figment / Configuration

### 12.1 决策

**[ACCEPTED]** 配置加载为 **figment 0.10+**（多源合并 + 强类型校验 + 环境变量替换）。

### 12.2 配置源优先级

1. CLI 参数（最高）
2. 环境变量（`PLATFORM_*`）
3. `config/platform.yaml`（生产）
4. `config/platform.local.yaml`（本地覆盖）
5. 内置默认值（最低）

### 12.3 关键配置项

```yaml
server:
  api_listen: "0.0.0.0:3000"
  admin_listen: "127.0.0.1:3001"  # 默认仅本机
database:
  url: "postgres://platform:***@localhost/platform"
  max_connections: 50
ai_gateway:
  providers:
    openai:
      api_key_ref: "vault:secret/platform/ai/openai"  # V1+ Cloud
      model: "gpt-4o"
secrets:
  kek_provider: "env"  # MVP
  env_var: "PLATFORM_KEK"
```

---

## 13. 其他关键库

| 维度 | 库 | 备注 |
|---|---|---|
| CLI 解析 | `clap` 4.x（derive 模式）| 与 figment 集成 |
| 错误处理 | `thiserror` 1.x（库）+ `anyhow` 1.x（二进制）| 库代码用 thiserror 暴露具体错误类型 |
| 序列化 | `serde` 1.x + `serde_json` / `serde_yaml` / `bincode` | API 边界用 JSON，存储用 bincode |
| 时区 / 时间 | `chrono` 0.4+（PostgreSQL TIMESTAMPTZ 友好）| 不用 `time` 库以避免与 sqlx 类型转换麻烦 |
| UUID | `uuid` 1.x（含 v7）| 节点 ID / Event ID 全用 v7 |
| 正则 | `regex` 1.x（lazy_static 缓存）| URL 校验、policy 表达式 |
| 压缩 | `flate2` + `tar` + `zstd` | 多种算法 |
| JSON Schema 校验 | `jsonschema` 0.18+ | App Manifest / 事件 Schema |
| MCP 协议 | 自实现（~500 行）+ `reqwest` | 规范简单；避免依赖膨胀 |
| 异步取消 | `tokio_util::sync::CancellationToken` | graceful shutdown |
| 数值 | `rust_decimal` 2.x | 货币 / 精确小数（如需）|
| 模板 / 邮件 | `handlebars` 4.x + `lettre` 0.11 | V1+ |

---

## 14. MCP 协议实现

### 14.1 决策

**[ACCEPTED]** MCP（Model Context Protocol）**自实现**（约 500 行 Rust），基于 `axum` + `serde_json` + `reqwest`。

### 14.2 理由

- MCP 规范（Anthropic 提出）相对简单：JSON-RPC 2.0 over HTTP + SSE
- 生态 `rmcp` 库活跃度一般；自实现可控 + 依赖最少
- 与 `axum` 路由深度集成（`/mcp/tools/list` `/mcp/tools/call` 等）

### 14.3 关键路径

```
POST /mcp/v1/rpc
  → JSON-RPC dispatch
  → method: tools/list | tools/call | resources/list
  → 鉴权（Bearer token 或 mTLS）
  → 调用内部 handler
  → 返回 result 或 error
```

---

## 15. 强约束与禁止项 / Hard Constraints

> 本节列出 **必须遵守** 的技术约束。违反任一条 → PR 拒绝。

### 15.1 性能与安全

- **[REQ-TS-001]** 禁止 `unsafe` Rust 代码（除 1-2 处明确标注的低层优化）
- **[REQ-TS-002]** 关键路径禁止 `.unwrap()` / `.expect()`；用 `?` 传播错误
- **[REQ-TS-003]** 禁止 `panic!` 在生产代码中（除 startup 配置缺失）
- **[REQ-TS-004]** 全部异步函数返回 `Result<T, PlatformError>`；不带 `unwrap` 的 anyhow
- **[REQ-TS-005]** 全部 SQL 走 `sqlx::query!` 宏（编译期校验）；禁止字符串拼接
- **[REQ-TS-006]** 全部密码 / 凭证走 argon2id；禁止 MD5 / SHA-1 / bcrypt
- **[REQ-TS-007]** 全部加密走 RustCrypto；禁止自实现加密原语
- **[REQ-TS-008]** 全部随机数走 `OsRng`；禁止 `thread_rng()` 用于密钥
- **[REQ-TS-009]** 全部进程间通信（V1+ Cloud）走 mTLS；禁止明文 HTTP
- **[REQ-TS-010]** 禁止把 JWT secret / KEK / DB password 硬编码；走环境变量或 Vault

### 15.2 架构

- **[REQ-TS-011]** 禁止为"纯 Rust"原则重写成熟高风险 Git 设施（写路径必须用 shell `git`）
- **[REQ-TS-012]** 禁止引入 Memgraph / NATS / Valkey / Redis 等独立服务（违反"能 PG 解决"原则）
- **[REQ-TS-013]** 禁止 Rust 二进制依赖 Node.js / Python / 其它语言运行时（违反"纯二进制"原则）
- **[REQ-TS-014]** 禁止 Rust 后端依赖 npm 包（前端 SvelteKit 除外）
- **[REQ-TS-015]** 全部 public 库 API 走 `thiserror` 定义具体错误类型；禁止 `anyhow` 跨 crate
- **[REQ-TS-016]** 全部 async 函数显式返回 `Result`；禁止 `impl Future<Output = ()>`

### 15.3 文档

- **[REQ-TS-017]** 任何库选型变更需更新本文档 + 写 ADR（`docs/architecture/000X-*.md`）
- **[REQ-TS-018]** 任何被拒绝的库（见第 0 节）需在 ADR 中说明拒绝原因
- **[REQ-TS-019]** 任何新库引入前需在 `docs/architecture/000X-*.md` 写 1-page ADR

---

## 16. 关键风险与缓解 / Risks & Mitigations

| 风险 | 概率 | 影响 | 缓解 |
|---|---|---|---|
| Rust 学习曲线 | 中 | 初期开发慢 2-3 月 | (a) 培训 + 现有 Rust 经验;(b) 关键模块先用类型化伪代码 + TDD |
| gix 写路径成熟度 | 低 | 高 | 强约束：写路径走 shell `git`；gix 仅做读路径 |
| 编译时间长 | 中 | 开发迭代慢 | (a) `cargo-chef` 缓存依赖编译;(b) `sccache` 远程缓存;(c) 拆分 crate 减少重编译 |
| 二进制膨胀（依赖多）| 中 | 部署包大 | (a) `lto = "fat"` + `codegen-units = 1`;(b) 定期 `cargo bloat` 审查 |
| Tokio 与 sync 代码互操作 | 中 | 数据竞争 | 严格分层：业务层 100% async；底层 sync（如 rsa 私钥解密）走 `spawn_blocking` |
| 生态碎片化（库选择多）| 中 | 维护成本 | 本文档为权威来源；任何新增需 ADR |
| 平台代码中含 GPL/AGPL 依赖 | 低 | 许可证不合规 | CI 加 `cargo-deny` 检查（禁止 GPL/AGPL 仅允许 MIT/Apache-2.0）|

---

## 17. 验收标准 / Acceptance Criteria

本文档被接受需满足：

- [x] **AC-1** Rust 选型理由覆盖 AISEC-REQ 的内存安全诉求
- [x] **AC-2** 库选型清单与现有 [00-requirements-definition.md §0-§54](../requirements/00-requirements-definition.md) 无冲突
- [x] **AC-3** 库选型与现有 [phase10-architecture.md §2 §6 §7](../requirements/phase10-architecture.md) 的强约束一致
- [x] **AC-4** 库选型与现有 [basic-design/00-introduction.md §0.5](../design/basic-design/00-introduction.md) 设计原则一致
- [x] **AC-5** 强约束（REQ-TS-001〜019）覆盖性能 / 安全 / 架构 / 文档
- [x] **AC-6** 关键风险与缓解措施列出

---

## 18. 后续 ADR 引用 / Future ADRs

本 ADR 拍板主语言后，以下子决策仍需后续 ADR：

| ADR # | 主题 | 关联文档 |
|---|---|---|
| ADR-002 | gix 读路径具体模块边界 | phase10-architecture.md §7 项 12 |
| ADR-003 | OCI 容器标准 vs Docker-only | basic-design/05-interface-design.md |
| ADR-004 | WebAuthn 凭证库选型（V1+ Cloud Admin 双因素）| basic-design/14-admin-ops-ui.md §14.3.3 |
| ADR-005 | SIEM 适配器（Splunk / ELK / Datadog）| basic-design/14-admin-ops-ui.md §14.2.5 |
| ADR-006 | OTel Collector 部署模式（Sidecar vs DaemonSet vs Gateway）| detailed-design/10-observability.md |
| ADR-007 | HashiCorp Vault 集成细节 | basic-design/07-security-design.md §7.5 |
| ADR-008 | OCI Plugin 包格式（Image Manifest schema）| basic-design/13-app-cluster-and-plugins.md §13.2 |

---

## 19. 关键 REQ-ID 新增

| REQ-ID | 简述 | 章节 |
|---|---|---|
| TECH-REQ-001 | 主语言 Rust（edition 2021，MSRV 1.75） | §2 |
| TECH-REQ-002 | 异步运行时 Tokio 1.x | §3 |
| TECH-REQ-003 | Web 框架 Axum 0.7+ | §4 |
| TECH-REQ-004 | gRPC Tonic 0.12+ | §5 |
| TECH-REQ-005 | PostgreSQL sqlx 0.8+ 编译期校验 | §6 |
| TECH-REQ-006 | Git 库 gix (gitoxide) 0.66+ 仅读路径 | §7 |
| TECH-REQ-007 | Git 写路径强约束 shell `git` | §7.2 |
| TECH-REQ-008 | 认证 jsonwebtoken + argon2 | §8 |
| TECH-REQ-009 | 加密原语 RustCrypto | §9 |
| TECH-REQ-010 | HTTP 客户端 reqwest 0.12+ | §10 |
| TECH-REQ-011 | 可观测性 tracing + OTel + metrics | §11 |
| TECH-REQ-012 | 配置 figment | §12 |
| TECH-REQ-013 | MCP 协议自实现（~500 行）| §14 |
| TECH-REQ-014 | 强约束 19 条（§15.1-15.3）| §15 |
| TECH-REQ-015 | 8 项 ADR 待写 | §18 |

---

**导航 / Navigation:**
[← 详细设计 §13. Admin API & Ops UI](../design/detailed-design/13-admin-api-and-ops-ui.md) · [需求定义书 §53 ADR 列表项 11](../requirements/00-requirements-definition.md) · [phase10-architecture.md §2 §6 §7](../requirements/phase10-architecture.md)
