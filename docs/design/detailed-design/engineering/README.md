# 详细设计工程产物索引

本目录是 `docs/design/detailed-design/` 的**工程实现索引**。
详细设计书（00-13 共 14 文件）定义了"做什么 / 怎么做"，本目录定义"在哪里写代码"。

## 工程目录结构

```
GitGit/
├── Cargo.toml                          # workspace 根
├── Cargo.lock                          # (生成) 依赖锁定
├── crates/                             # 14 个 crate
│   ├── gitgit-errors/                  # 统一错误类型 (thiserror)
│   ├── gitgit-config/                  # figment 配置加载
│   ├── gitgit-observability/           # OTel + Prometheus + tracing
│   ├── gitgit-proto/                   # gRPC 自动生成代码
│   ├── gitgit-core/                    # 5 原语领域类型
│   ├── gitgit-graph/                   # 图谱引擎
│   ├── gitgit-policy/                  # 策略引擎
│   ├── gitgit-ai/                      # AI 网关
│   ├── gitgit-agent/                   # Agent 运行时
│   ├── gitgit-app/                     # App Registry + Plugin Loader
│   ├── gitgit-git/                     # Git 服务器
│   ├── gitgit-server/                   # 平台 binary (主进程)
│   ├── gitgit-admin/                   # Admin binary (独立子进程)
│   └── gitgit-cli/                     # CLI 客户端 binary
├── migrations/                         # 14 个 PostgreSQL migration
├── proto/                              # 5 个 .proto (gRPC 内部)
├── openapi/                            # openapi.yaml
├── config/                             # config.example.toml
├── cli/                                # cli.rs (clap derive)
└── docs/design/detailed-design/
    ├── 00-overview.md                  # 总览
    ├── 01-data-layer.md                # 数据层
    ├── ... (14 文件)
    ├── engineering/                    # ← 本目录
    │   ├── README.md                   # 本文件
    │   ├── crate-map.md                # crate 详细映射
    │   └── dependency-graph.md         # crate 依赖图
    └── diagrams/                       # 状态机 / sequence 图
        ├── 01-agent-state-machine.md
        ├── 02-event-bus-state-machine.md
        ├── 03-auth-flow.md
        ├── 04-git-push-flow.md
        └── 05-app-upgrade-flow.md
```

## 详细设计 → 工程文件 映射

| 详细设计章节 | crate / 工程文件 | 关键类型 / 函数 |
|---|---|---|
| §00 总览 | `Cargo.toml` workspace | members + profiles + lints |
| §00.2 顶层模块 | `crates/gitgit-{core,graph,policy,ai,agent,app,git,server,admin,cli}/` | module tree |
| §01 数据层 | `migrations/0001-0014` + `crates/gitgit-core/src/db.rs` | 14 表 + sqlx 仓库 |
| §01.3 DB role 分离 | `migrations/0013_db_role_separation.up.sql` | `gitgit_app` / `gitgit_audit_readonly` |
| §01.6 RLS | `migrations/0011_tenants_rls.up.sql` | `current_setting('app.tenant_id')` |
| §02 图谱引擎 | `crates/gitgit-graph/` | 5 原语 + 递归 CTE |
| §03 策略引擎 | `crates/gitgit-policy/` | RBAC + ABAC + AI 策略 |
| §04 Agent 运行时 | `crates/gitgit-agent/` + `proto/agent.proto` | 状态机 + 凭证 |
| §05 AI 网关 | `crates/gitgit-ai/` + `proto/ai.proto` | provider 抽象 + 提示词清洗 |
| §06 Git 服务器 | `crates/gitgit-git/` | gix 读 + shell git 写 |
| §07 App 协调 | `crates/gitgit-app/` + `migrations/0006-0007` | Saga + Outbox + 中心事件 |
| §08 API handlers | `crates/gitgit-server/` + `openapi/openapi.yaml` | Axum 路由 + 中间件 |
| §08.9 CLI | `cli/cli.rs` + `crates/gitgit-cli/` | clap derive |
| §09 安全实现 | `crates/gitgit-errors/src/error.rs` | 完整 40+ 错误 variant |
| §10 可观测性 | `crates/gitgit-observability/` | OTel + Prometheus |
| §11 错误处理 | `crates/gitgit-errors/src/error.rs` | AppError + ErrorCode |
| §12 App Registry | `crates/gitgit-app/` + `proto/internal.proto` | 加载器 + 沙箱 |
| §13 Admin API | `crates/gitgit-admin/` | 独立子进程 + 鉴权域 |

## 配置 / 配置示例

- `config/config.example.toml` — 完整 figment 配置 schema（30+ 段）

## 测试 / Benchmark / 部署

- `crates/*/tests/` — 单元测试 + 集成测试 (testcontainers)
- `crates/*/benches/` — criterion 基准
- `Dockerfile` + `docker-compose.yml` (待 V1+ 写)
- Helm Chart (V1+ Cloud)

## 实施顺序建议

1. **MVP 第 1 周**：`crates/gitgit-errors` + `crates/gitgit-config` + `crates/gitgit-observability` (基础设施)
2. **MVP 第 2 周**：`migrations/0001-0014` + `crates/gitgit-core` (数据层)
3. **MVP 第 3-4 周**：`crates/gitgit-graph` + `crates/gitgit-policy` (核心引擎)
4. **MVP 第 5-6 周**：`crates/gitgit-git` + `crates/gitgit-server` + `crates/gitgit-cli` (MVP 可运行)
5. **V1+**：`crates/gitgit-ai` + `crates/gitgit-agent` + `crates/gitgit-app` + `crates/gitgit-admin` (扩展)

## 关联文档

- 详细设计书：[`../`](../)
- 实施前 QA：[`../../../../../../architecture/qa-checklist.md`](../../../../../../architecture/qa-checklist.md)
- 技术选型：[`../../../../../../architecture/tech-selection.md`](../../../../../../architecture/tech-selection.md)
- 流程文档：[`../../../../../process/workflow.md`](../../../../../process/workflow.md)





