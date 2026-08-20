# 02. 系统方式设计 / System Architecture Design

> **[PROPOSAL] IPA 共通框架 2013 章节定位：** P3.A2.T1 研讨硬件·软件构成


## 2.1 逻辑架构 / Logical Architecture

**[PROPOSAL]** 本平台的逻辑架构以 Phase 6 确定的 5 个原语（Node / Edge / Event / Policy / View，Agent 作为 Node 子类型）为中心，由 8 个子系统组成。

```
┌──────────────────────────────────────────────────────────────────────┐
│                       AI-Native Engineering Platform                  │
│                                                                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────┐ │
│  │ Git Server   │  │ Engineering  │  │  AI Gateway  │  │  Context │ │
│  │ Subsystem    │  │   Graph      │  │  Subsystem   │  │  Engine  │ │
│  │              │  │ Subsystem    │  │              │  │ Subsystem│ │
│  │ (GIT-REQ)    │  │ (GRF-REQ)    │  │ (AI-REQ)     │  │ (CTX-REQ)│ │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  └────┬─────┘ │
│         │                 │                 │              │       │
│  ┌──────┴─────────────────┴─────────────────┴──────────────┴─────┐ │
│  │            Platform Process (single monolith process)          │ │
│  │  - HTTP API  - MCP server  - Graph query  - Policy eval        │ │
│  │  - RBAC/ABAC  - Audit emit  - Hook dispatch  - View log        │ │
│  └─────────────────────────────┬──────────────────────────────────┘ │
│                                │                                    │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────┐ │
│  │ Agent        │  │ CI/CD        │  │ Security &   │  │  API /   │ │
│  │ Runtime      │  │ Subsystem    │  │ Access Ctrl  │  │  MCP /   │ │
│  │ Subsystem    │  │              │  │ Subsystem    │  │  Webhook │ │
│  │ (AGT-REQ)    │  │ (CI-REQ)     │  │ (SEC-REQ)    │  │ (API-REQ)│ │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  └────┬─────┘ │
│         │                 │                 │              │       │
└─────────┼─────────────────┼─────────────────┼──────────────┼───────┘
          │                 │                 │              │
          ▼                 ▼                 ▼              ▼
   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐  ┌────────┐
   │  Agent      │   │  CI Runner  │   │  Secrets    │  │ Ext.   │
   │ Workspace   │   │ (ephemeral) │   │  Store      │  │ Clients│
   │ (ephemeral) │   │             │   │ (encrypted) │  │        │
   └─────────────┘   └─────────────┘   └─────────────┘  └────────┘

          ▼                                      ▼
   ┌─────────────────────────────────────────────────────┐
   │   PostgreSQL 14+ (single instance, ACID)           │
   │   - nodes / edges / events / policies / views       │
   │   - issues / prs / reviews / ci_runs / agent_runs   │
   │   - users / roles / permissions / audit_redactions  │
   └─────────────────────────────────────────────────────┘
```

## 2.2 部署单位 / Deployable Units（来自 Phase 10 §4）

**[PROPOSAL]** MVP 的部署单位有以下 4 类：

| 单位 | 职责 | 生命周期 | 状态保持 |
|---|---|---|---|
| ① Platform Process | 包含 API/MCP/Git 包装/Graph Engine/AI Gateway/Context Engine/Agent Runtime Control Plane/Security 的单一进程 | 常驻（1〜N 实例，Cloud 时水平扩缩）| 无状态（除缓存外，全部从 PostgreSQL 来）|
| ② PostgreSQL | 唯一持久化存储 | 常驻（HA 部署在 Cloud 层单独处理）| 完整状态保留 |
| ③ Agent Workspace | Agent 执行的临时沙箱（AGT-REQ-005）| Platform Process 启动/关闭，每个执行单位都是临时的 | 仅执行期间 |
| ④ CI Runner | CI 执行的临时沙箱（CI-REQ）| Platform Process 启动/关闭 | 仅执行期间 |

**为什么不拆分为微服务（Phase 10 §4 的结论）：** MVP 的参考循环（Phase 9 §1）面向"1 个团队 / 1 个演示规模 / 自托管"，Phase 9 的 Definition of Done 第 9 步之外的所有访问模式都能在 PostgreSQL 索引 + 递归 CTE 的可测响应时间内完成（Phase 10 §1 推断）。拆分仅在"未来多节点 Cloud 部署（CLOUD-REQ-003）"时进行，且即使拆分后 Platform Process 的无状态化在当前设计中已实现（见 §2.4）。

## 2.3 物理架构 / Physical Architecture

### 2.3.1 Local 部署（MVP 标准形态）

**[PROPOSAL]** Local 部署遵守 OPS-REQ-005 的"小规模部署不要求编排器"原则，使用 Docker Compose（或 systemd + 直接二进制）启动。

```
┌─────────── Single Linux/macOS/Windows Machine ───────────┐
│                                                            │
│  ┌────────────┐  ┌─────────────┐  ┌──────────────────┐  │
│  │ platform   │  │ postgres    │  │ docker daemon /  │  │
│  │ process    │  │ (container) │  │ containerd       │  │
│  │ (binary)   │  │             │  │                  │  │
│  │            │  │             │  │  ├─ agent-ws-1  │  │
│  │ HTTP :8080 │  │ TCP :5432   │  │  ├─ ci-runner-1 │  │
│  │ SSH :2222  │  │             │  │  └─ ...         │  │
│  └─────┬──────┘  └──────┬──────┘  └────────┬─────────┘  │
│        │                │                  │             │
│        └──── unix socket / loopback ──────-┘             │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐ │
│  │ Persistent volumes                                    │ │
│  │  - postgres data                                      │ │
│  │  - git bare repos (filesystem, under platform process) │ │
│  │  - secrets store (encrypted at rest, §7.5)            │ │
│  │  - audit export (append-only, §7.2)                   │ │
│  └──────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────┘
```

### 2.3.2 Cloud 部署（V1 正式化，MVP 不需要）

**[PROPOSAL]** Cloud 部署与 Local 使用相同二进制 / 相同 schema。差异仅在部署包装和 PostgreSQL 的运维层级（Phase 10 §5）。

```
┌─────────── Cloud Region (Multi-AZ) ───────────┐
│                                                  │
│  ┌──────────────────────────────────────────┐  │
│  │  L7 Load Balancer                         │  │
│  └────────────────┬─────────────────────────┘  │
│                   ▼                             │
│  ┌──────────────────────────────────────────┐  │
│  │  Platform Process (stateless, N replicas)  │  │
│  │  - Horizontal scale-out                   │  │
│  │  - All state in PostgreSQL                │  │
│  └────────────────┬─────────────────────────┘  │
│                   ▼                             │
│  ┌──────────────────────────────────────────┐  │
│  │  Managed PostgreSQL (Primary + Replicas)  │  │
│  │  + Read replicas for graph query          │  │
│  │  + WAL archiving for PITR                 │  │
│  └──────────────────────────────────────────┘  │
│                                                  │
│  ┌──────────────────────────────────────────┐  │
│  │  Container Orchestrator (K3s/K8s)         │  │
│  │  - Agent Workspace / CI Runner scheduling │  │
│  │  - Network policy enforcement (§7.4)      │  │
│  └──────────────────────────────────────────┘  │
│                                                  │
│  ┌──────────────────────────────────────────┐  │
│  │  Object Storage (S3-compatible) — V1+     │  │
│  │  - LFS, artifacts, audit export           │  │
│  └──────────────────────────────────────────┘  │
└──────────────────────────────────────────────────┘
```

## 2.4 存储架构 / Storage Architecture（采用 Phase 10 §1）

**[PROPOSAL]** 单一 PostgreSQL 实例保存 Node/Edge/Event/Policy/View 的定义数据。MVP 不采用专用图数据库与事件日志的拆分（Phase 10 §1 结论）。

| 数据类型 | 存储位置 | 理由 |
|---|---|---|
| Node / Edge / Event / Policy / View 定义 | PostgreSQL `nodes` / `edges` / `events` / `policies` / `views` 表 | 单一事务内 ACID 写入，Node+Edge+Event 一致性免费（Phase 10 §1）|
| 用户 / 角色 / 权限 | PostgreSQL `users` / `roles` / `permissions` | SEC-REQ-001（RBAC/ABAC）|
| Issue / PR / Review / AgentRun | PostgreSQL（作为 Node）| GRF-REQ-001/006（这些是 Node 的子类型）|
| CI 执行结果 | PostgreSQL（Node + Event）| CI-REQ-001 |
| Git 对象 | 文件系统（裸仓库）| 系统 `git` 二进制直接管理。不存入 PostgreSQL |
| 密钥（API 密钥、Agent 凭据）| 加密专用存储（PostgreSQL `secrets` 表 + 信封加密）| SEC-REQ-005（与图数据隔离）|
| 审计导出 | 仅追加文件 + 外部 SIEM 集成（V1+）| 详见 [§7.2 审计](07-security-design.md#72-审计-audit) |

**重评估触发条件（Phase 10 §1.3 原话）：**
> 在以下任一条件下重新评估：(a) 实际运行遥测显示通过递归 CTE 的遍历查询在真实 Edge 数下超过了商定的延迟预算，或 (b) 需要无限制深度 / 图算法查询（DTWIN-REQ-003、WKFL-REQ-002）的 V2 功能实际进入开发。若发生迁移，作为从 `events` 日志可重建的二级索引引入，绝不作为主存储，也不需要改变 Node/Edge/Event 本身的数据模型（依据需求书 §16 的明确设计目标"后续新增 spec 概念永远不应需要新的结构原语"）。

## 2.5 进程间通信 / Inter-Process Communication

| 通信路径 | 用途 | 协议 |
|---|---|---|
| ① ↔ ② Platform ↔ PostgreSQL | 所有持久化 | TCP 5432 + TLS（生产环境）、unix socket（Local 单机）|
| ① ↔ ③ Platform ↔ Agent Workspace | 启动/观察/关闭、stdout/stderr 采集 | OCI/CRI 兼容的容器运行时 API |
| ① ↔ ④ Platform ↔ CI Runner | 同上 | 同上 |
| ① 内 子系统之间 | 函数调用（同一进程）| 函数调用 / 内部 channel |
| ① ↔ ①（Cloud 时）| 水平扩缩时同进程之间 | 不需要（无状态，全部经 PostgreSQL）|
| ① → 外部 MCP 客户端 | Agent 互操作 | MCP over HTTP/JSON-RPC |

跨进程通知（例如：API 进程 → CI Runner 的新任务通知）通过 PostgreSQL `LISTEN`/`NOTIFY` 实现。MVP 不引入 NATS JetStream 等消息代理（Phase 10 §2）。

## 2.6 Git 存储实现 / Git Storage Implementation（采用 Phase 10 §3）

**[PROPOSAL]** 混合方式：写入 / 协议类通过系统 `git` CLI 的子进程调用；读取类通过 libgit2 绑定进行。

| 操作 | 实现 | 理由 |
|---|---|---|
| clone / fetch / push / `receive-pack` | 系统 `git` CLI 子进程 | 与 GitHub / GitLab / Gitea / Forgejo 一致，继承多年正确性 |
| 包协商 / LFS / 部分 clone | 系统 `git` CLI 子进程 | 边缘情况的正确性优先 |
| 历史遍历 / tree / blob 读取 / diff | libgit2 进程内 | 减少图采集时的子进程开销 |
| Graph-aware 钩子（`pre-receive` / `post-receive` / `update`）| 标准 Git 钩子 → 回调平台 API | 标准扩展点，不是私有协议 |
| 垃圾回收 / 重打包 | `git gc --auto` 等价（V1, GIT-REQ-011）| 标准 Git 功能 |

---

**导航 / Navigation:**
[← 01. 系统概述](01-system-overview.md) · [README](README.md) · [03. 功能设计 →](03-functional-design.md)
