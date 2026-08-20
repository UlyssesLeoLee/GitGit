# 00. 总体概述 / Overview

## 0.1 目标 / Purpose

本章定义 AI-Native Engineering Platform 的**代码级结构**——monorepo 布局、模块划分、运行时约定、跨切关注点的实现模式。后面的章节按模块逐一深入。

## 0.2 顶层模块划分

**[PROPOSAL]** 本平台采用**单仓 monorepo + 内部模块化**结构，与基本设计 [§2.1 逻辑架构](../basic-design/02-architecture.md#21-逻辑架构-logical-architecture) 的 8 个子系统一一对应。

```
platform/
├── cmd/
│   └── platform/                  # 主二进制入口
│       └── main.go  /  main.rs
│
├── internal/
│   ├── graph/                     # Engineering Graph 子系统 (对应 §3.2)
│   │   ├── node/
│   │   ├── edge/
│   │   ├── event/
│   │   ├── policy/
│   │   ├── view/
│   │   ├── traversal/             # 递归 CTE 封装
│   │   └── registry/              # 类型注册表
│   │
│   ├── authn/                     # 认证 (Local: password+SSH; Cloud V1: OIDC)
│   ├── authz/                     # 授权 (RBAC/ABAC, 见 03)
│   ├── policy/                    # Policy 引擎核心 (见 03)
│   │
│   ├── git/                       # Git Server (见 06)
│   │   ├── transport/             # HTTP/SSH 协议
│   │   ├── hook/                  # pre/post-receive 钩子
│   │   ├── hybrid/                # CLI + libgit2 适配
│   │   └── objects/               # bare 仓库管理
│   │
│   ├── agent/                     # Agent 运行时 (见 04)
│   │   ├── lifecycle/
│   │   ├── workspace/             # 容器编排接口
│   │   ├── credential/            # 范围受限凭证
│   │   ├── messaging/
│   │   └── graphlink/             # AgentRun Node 写回
│   │
│   ├── ai/                        # AI Gateway (见 05)
│   │   ├── provider/              # 各提供商适配
│   │   ├── sanitize/              # 提示词 / 响应清洗
│   │   └── budget/                # 成本 / token 追踪 (V1)
│   │
│   ├── context/                   # Context Engine (见 §3.4)
│   │
│   ├── ci/                        # CI/CD Runner (对应 §3.6)
│   │
│   ├── security/                  # 安全实现 (见 09)
│   │   ├── envelope/              # 信封加密
│   │   ├── kek/                   # KEK 管理
│   │   ├── tls/                   # TLS 配置
│   │   └── dbroles/               # AISEC-REQ-009(a) DB role 分离
│   │
│   ├── coordination/              # App 群组协调 (见 07)
│   │   ├── procs/                 # 存储过程注册
│   │   ├── outbox/                # Outbox relay
│   │   └── saga/                  # Saga engine
│   │
│   ├── api/                       # API 处理器 (见 08)
│   │   ├── http/
│   │   ├── mcp/
│   │   ├── cli/                   # 本进程调用
│   │   └── webhook/               # 出站投递
│   │
│   ├── audit/                     # 结构化审计写入
│   ├── obs/                       # 可观测性 (见 10)
│   ├── errors/                    # 错误框架 (见 11)
│   │
│   └── config/                    # 配置加载 (YAML + 环境变量)
│
├── migrations/                    # PostgreSQL migration 文件
│   ├── 0001_init.sql
│   ├── 0002_rls.sql
│   ├── 0003_db_roles.sql          # AISEC-REQ-009(a)
│   └── ...
│
├── deploy/
│   ├── docker-compose.yml
│   ├── Dockerfile
│   └── helm/                      # V1
│
├── api/
│   └── openapi/
│       └── v1.yaml                # OpenAPI 3.1 单一来源 (见 08)
│
├── scripts/
│   ├── gen-sdks/                  # OpenAPI → 客户端 SDK
│   ├── run-mock/                  # Prism mock server
│   └── ci/                        # CI 脚本
│
├── test/
│   ├── e2e/                       # Playwright + 自定义
│   ├── contract/                  # Pact
│   ├── load/                      # k6
│   └── security/                  # ZAP, CodeQL
│
└── docs/
    ├── requirements/              # 已有
    └── design/
        ├── basic-design/
        └── detailed-design/       # 本目录
```

## 0.3 语言与运行时 / Language & Runtime

**[ACCEPTED]** **主语言已选定为 Rust（edition 2021，最低 MSRV 1.75）**，详细选型依据与全套库清单见 [架构 / 技术选型文档](../../architecture/tech-selection.md)（对应需求定义书 §53 ADR 列表项 11，状态 Accepted 2026-08-19）。

**核心选型一览**（仅摘要，完整理由见技术选型文档）：

| 维度 | 选型 | 版本 |
|---|---|---|
| 主语言 | Rust | edition 2021，MSRV 1.75 |
| 异步运行时 | Tokio | 1.x multi-threaded |
| HTTP 框架 | Axum | 0.7+ |
| gRPC | Tonic | 0.12+ |
| PostgreSQL | sqlx | 0.8+（编译期 SQL 校验）|
| Git 读路径 | gix (gitoxide) | 0.66+（纯 Rust）|
| Git 写路径 | shell `git` 进程 | 强约束：禁止为"纯 Rust"重写（[phase10-architecture.md §6](../../requirements/phase10-architecture.md)）|
| 认证 | jsonwebtoken + argon2 | 9.x / 0.5+ |
| 加密 | RustCrypto 套件 | aes-gcm / sha2 / hmac |
| 可观测性 | tracing + OpenTelemetry | 0.1+ / 0.24+ |
| 配置 | figment | 0.10+ |
| CLI | clap | 4.x derive |
| 错误处理 | thiserror（库）+ anyhow（二进制）| 1.x |
| HTTP 客户端 | reqwest | 0.12+ |
| MCP 协议 | 自实现（~500 行）| 基于 axum + serde_json + reqwest |

### 0.3.1 关键决策摘要

**为什么 Rust（vs Go）**：

1. **内存安全 + 无 GC** — 处理不可信 Agent 输入（AISEC-REQ-001 提示词注入防御边界）时，Rust 编译期保证消除整类 RCE 漏洞
2. **生态成熟** — sqlx / Axum / Tonic / gix / RustCrypto 在 2024-2026 已生产稳定
3. **二进制小 + 启动快** — 满足 Local-First 单进程部署
4. **类型系统表达力** — `Result<T, E>` 强制错误处理 + `?` 传播符合 AISEC-REQ 集合对"错误不吞"的隐含要求

完整对比与拒绝方案见 [技术选型文档 §2](../../architecture/tech-selection.md#2-主语言决策rust-language-decision)。

### 0.3.2 运行时要求

| 项目 | 要求 | 备注 |
|---|---|---|
| 最低 OS | Linux 5.10+ / macOS 13+ / Windows 11 | 与基本设计一致 |
| 容器运行时 | Docker Engine 20.10+ / Podman 4+ / containerd 1.6+ | OCI 兼容 |
| PostgreSQL | 14+ | 必需 |
| 系统 `git` | 2.30+ | 写路径必需（强约束）|
| Rust 工具链 | rustc 1.75+ / cargo 1.75+ | MSRV 1.75 |
| 内存 | 最低 256MB（小型部署）| [TBD] Benchmark |
| 磁盘 | 1GB（不含 git 数据）| |

## 0.4 错误处理约定 / Error Handling Convention

**所有公共函数返回 `(result, error)` 或等价的 Result 类型。** 错误必须实现：

```go
// [IMPL] 伪代码（以 Go 为例）
type PlatformError interface {
    error
    Code() string       // 标准化错误码, e.g. "policy_denied"
    HTTPStatus() int    // 映射的 HTTP 状态码
    Details() map[string]any  // 详情负载
    Cause() error       // 包装的底层错误
}
```

详见 [11 错误处理](11-error-handling.md)。

## 0.5 日志约定 / Logging Convention

**[PROPOSAL]** 全平台统一使用**结构化 JSON 日志**（基本设计 [§8.4](../basic-design/08-operations-design.md#84-日志-logging)）。

```json
{
  "ts": "2026-08-19T06:18:00.123Z",
  "level": "info",
  "service": "platform",
  "trace_id": "0190e8a4-...",
  "span_id": "...",
  "actor_id": "0190e8a4-...",
  "event": "graph.node.created",
  "node_id": "0190e8a4-...",
  "duration_ms": 12,
  "msg": "node created"
}
```

**禁止：** printf 风格字符串拼接、敏感字段（密钥、token、密码）直接落日志、emoji 字符（`[PROPOSAL]` 用于文档可以，代码中禁止）。

## 0.6 配置约定 / Configuration Convention

**[PROPOSAL]** 配置来源优先级（高 → 低）：

1. 命令行 flag
2. 环境变量（前缀 `PLATFORM_`）
3. 配置文件 `~/.platform/config.yaml` 或 `--config` 指定路径
4. 内置默认值

配置 schema 用 struct tag 标注：

```go
// [IMPL] 伪代码
type Config struct {
    HTTP     HTTPConfig     `yaml:"http"`
    Postgres PostgresConfig `yaml:"postgres"`
    Git      GitConfig      `yaml:"git"`
    AI       AIConfig       `yaml:"ai"`
    Security SecurityConfig `yaml:"security"`
}
```

启动时严格校验（fail-fast），配置错误直接退出并打印字段名。

## 0.7 并发模型 / Concurrency Model

**[PROPOSAL]** 采用**结构化并发**：

| 模式 | 用法 | 库/机制 |
|---|---|---|
| 单飞 (singleflight) | 同一 key 的并发请求合并为一次实际调用 | `golang.org/x/sync/singleflight`（Go）/ `tokio::sync::OnceCell`（Rust）|
| Worker Pool | 异步任务（webhook 投递、Saga 步进、outbox relay）| 预创建 N 个 worker，channel 分发 |
| Read-Write Lock | 共享缓存（policy 决策、type registry）| `sync.RWMutex` / `parking_lot::RwLock` |
| Actor 隔离 | Agent Workspace 进程（详见 04）| 容器边界 |
| Connection Pool | PostgreSQL 连接 | `pgxpool` / `deadpool-postgres` |

**禁止：** 跨包共享可变全局状态、goroutine 泄漏（无 context cancel 路径）、无超时 channel 接收。

## 0.8 测试约定 / Testing Convention

| 层级 | 框架 | 覆盖目标 |
|---|---|---|
| 单元测试 | 语言标准（Go `testing` / Rust `cargo test`）+ 表格驱动 | 每个公共函数 1+ 例 |
| 集成测试 | `testcontainers`（启动真实 PostgreSQL/Redis）| DB schema、repository 层 |
| 端到端 | Playwright (UI) + 自定义 E2E（API/MCP/SSH）| 10 步参考循环 |
| Contract | Pact（HTTP）/ 自定义 schema test（MCP）| API 表面不漂移 |
| 性能 | k6 + pgbench | NFR 暂定等级 |
| 安全 | OWASP ZAP（HTTP）/ CodeQL（代码）/ 自定义 prompt injection suite | AISEC-REQ 6 项 |
| Chaos | toxiproxy / 自定义 kill-and-survive | GIT-REQ-010 行为 |

**命名：** 测试文件与被测文件同名，后缀 `_test.go` / `_test.rs`。E2E 测试放 `test/e2e/`。

## 0.9 性能预算 / Performance Budget

[PROPOSAL] 来自基本设计 [§6.2 性能与扩展性](../basic-design/06-non-functional-design.md#62-性能与扩展性performance-scalability-ipa-grade-②) 的具体目标：

| 操作 | 95p 目标 | 99p 目标 | 备注 |
|---|---|---|---|
| 读 API | < 200ms | < 500ms | [TBD] Benchmark |
| 写 API | < 500ms | < 1s | [TBD] Benchmark |
| 图遍历（4 hop）| < 1s | < 2s | [TBD] Benchmark |
| 策略评估 | < 1ms | < 5ms | 进程内缓存命中时 |
| 上下文组装 | < 200ms | < 500ms | 不含 AI 调用 |
| Git push（中等仓库 100MB）| < 5s | < 15s | 排除网络 |
| AI 推理（不含上游）| < 100ms 调度 overhead | — | 实际推理时间由上游决定 |

**性能回归检测：** 每夜跑基准，任何指标超阈值 10% 触发 CI 失败。

## 0.10 关键依赖 / Key Dependencies

**[ACCEPTED]** 核心依赖库选型（依据 [技术选型文档 §0](../../architecture/tech-selection.md#0-决策摘要-decision-summary) 已拍板）：

| 类别 | 库 / Crate | 用途 |
|---|---|---|
| HTTP server | `axum 0.7+` + `tower` | 公开 API 处理器与中间件 |
| gRPC | `tonic 0.12+` | 内部进程间 RPC |
| PostgreSQL | `sqlx 0.8+` (compile-time checked) | 数据库连接与编译期 SQL 校验 |
| Git | `gix 0.66+` (读路径) + 系统 `git` CLI (写路径) | Git 操作 |
| 容器运行时 | Docker Engine API / `containerd` | Agent/CI 沙箱编排 |
| 可观测性 | `tracing 0.1+` + `opentelemetry 0.24+` + `metrics 0.23+` | 结构化日志、分布式追踪与指标 |
| 加密 / 安全 | `RustCrypto` (`aes-gcm`, `sha2`, `hmac`) + `argon2` | 信封加密、哈希与 KDF |
| 认证 | `jsonwebtoken 9.x` | JWT 签发与校验 |
| 配置 | `figment 0.10+` + `serde_yaml` | 多源配置加载与校验 |
| JSON Schema | `jsonschema 0.18+` | App Manifest / 事件 Schema 校验 |
| 任务队列 / 异步 | Tokio channels + PG Outbox relay + Saga | 事件总线与分布式流程 |

## 0.11 部署包 / Deployment Artifacts

| 产物 | 用途 |
|---|---|
| `platform` 二进制 | 主进程，跨平台 |
| `platform-worker` 二进制 | 独立 worker（outbox relay、Saga engine），可与主进程同机或独立部署 |
| `platform-migrate` 二进制 | 数据库 migration 工具 |
| `platform-admin` 二进制 | 管理命令（创建初始 admin、强制 rotate KEK 等）|
| Docker 镜像 | `platform/server:vX.Y.Z`（多架构：amd64 + arm64）|
| Helm chart | V1 |

## 0.12 文档与代码同步 / Doc & Code Sync

- 每个公共 API 端点 → OpenAPI YAML（[08 API 处理器](08-api-handlers.md)）
- 每个 REQ-ID → 至少 1 个测试用例命名包含 REQ-ID
- ADR（架构决策记录）放在 `docs/adr/`，命名 `NNNN-<topic>.md`
- 本书的代码示例 (`[IMPL]`) → 实际代码的 1:1 蓝图，但允许格式调整

---

**导航 / Navigation:**
[← README](README.md) · [01. 数据层 →](01-data-layer.md)
