# 13. App 集群与可热插拔架构 / App Cluster & Hot-Pluggable Architecture

> **[PROPOSAL] IPA 共通框架 2013 章节定位：** P3.A1.T5 识别系统构成要素 (App 一级化扩展) / P3.A2.T6 研讨安全方式 (App 沙箱) / P3.A2.T8 研讨 App 群组信息互通方式 (升级为中心事件总线 + 集群)
>
> **设计主张 / Design Claim:** 将"App"从 §12 的"协调对象"**升级为平台一级部署单元**，与 Repository / Agent 平级。本章定义了 (a) App 概念模型、(b) App Manifest schema、(c) 中心事件总线、(d) App 集群模型、(e) 热插拔与升级机制。Phase 16 实现前必须先定，V1+ Cloud 部署再上 K8s Pod 化。

---

## 13.0 目的 / Purpose

§12 已确立"多 App 信息互通"的协调机制，但**没有把 App 抽象为一等公民**——App 注册、版本、热插拔、集群健康、灰度升级均无独立支撑。本章把 App 升级为平台一级概念，让每个功能可被独立打包、独立升级、独立下线。

**与现有原则的关系：**

| 原则 | 与本章的关系 |
|---|---|
| Git-Native | App Manifest (app.yaml) 存于 Git 仓库内，App 升级通过 Git push 触发（与 PR / Branch 联动）|
| Local-First | MVP 阶段 App 共享同一进程 + 同一 PostgreSQL（按 schema 隔离）；V1+ Cloud 才拆 K8s Pod |
| Graph-Native | App 是 Node 的 subtype（与 Repository / Agent 平级）|
| Agent-Native | Agent 可作为 App 内部模块；App 升级不破坏 Agent 凭证 |
| Evidence-Native | App 升级事件 = Edge 类型 `app_upgraded_by` + Evidence |
| Human Authority | App 升级的"上线"动作需人类审批（继承 §4.9 模式）|

---

## 13.1 概念模型：App 作为一级平台对象

**[PROPOSAL]** App 升格为平台一级对象，与 Repository / Agent 平级，享受相同的 CRUD / 审计 / 事件 / 权限。

```
平台对象图谱（5 原语应用）
═══════════════════════════════════════════════════════════════
Node (一级)
  ├─ Repository        — 代码仓库
  ├─ Issue / PR / ...  — 工作流对象
  ├─ Agent / AgentRun  — 智能体
  ├─ **App**           — 应用/插件（一级，新增）
  └─ AppInstance       — App 的部署实例（节点）
  └─ AppDeployment     — 一次部署动作（事件）

Edge
  ├─ app_depends_on    — App 间的依赖（DAG）
  ├─ app_instance_of   — AppInstance → App
  ├─ app_published_event — App → 中心事件
  └─ app_consumes_event — App → 中心事件

Event
  └─ app_installed / app_upgraded / app_rolled_back / app_disabled

Policy
  └─ ABAC 规则校验 App 安装/升级的来源可信度、目标租户范围

View
  └─ apps_health_grid / apps_version_matrix / apps_event_flow
```

**关键决定：**

1. **App 是 Node subtype**（不是独立表），复用 5 原语引擎（详细设计 §2 图谱引擎）。
2. **AppInstance 是 Node subtype**（每个部署实例一个 Node），便于追踪"App X 跑了 N 个实例"的多 Pod 场景。
3. **AppDeployment 是 Event**（不可变），记录每次安装/升级/回滚。
4. **App 间依赖用 Edge 表达**（`app_depends_on`），便于做启动顺序拓扑排序。

---

## 13.2 App Manifest (app.yaml) schema

每个 App 在其 Git 仓库根目录或 `apps/<app_id>/app.yaml` 包含一份 Manifest，作为该 App 的"身份证 + 能力清单 + 依赖声明"。

```yaml
# app.yaml — 最小完整示例
apiVersion: platform.io/v1          # Manifest schema 版本
kind: App
metadata:
  id: github-pr-reviewer            # 平台唯一 ID（小写 kebab-case）
  name: "GitHub PR 自动评审"
  version: 1.4.2                    # 语义化版本
  description: "监听 PR 事件，自动调用 LLM 生成评审意见"
  authors: ["team-platform@example.com"]
  license: Apache-2.0
spec:
  type: webhook-consumer            # 见 13.2.1 App 类型枚举
  entrypoints:                      # App 注册到平台的入口
    webhook:
      - event: pr.opened
        handler: handle_pr_opened   # App 内部函数名
      - event: pr.synchronize
        handler: handle_pr_updated
    http:
      - path: /api/apps/github-pr-reviewer/health
        method: GET
        handler: health
    mcp_tools:                     # 暴露为 MCP 工具
      - name: review_pr
        handler: mcp_review_pr
  consumes_events:                  # 中心事件订阅
    - pr.opened
    - pr.synchronize
    - pr.closed
  publishes_events:                 # 中心事件发布
    - review.completed
    - review.suggested
  storage:                          # PL/pgSQL 存储过程
    procedures:
      - name: ensure_review_state
        schema: app_github_pr_reviewer
        version: "1.4.2"
  permissions:                      # RLS 上下文所需的 App 角色
    db_role: app_github_pr_reviewer
    required_grants:
      - SELECT ON nodes
      - INSERT ON edges
      - SELECT, INSERT, UPDATE ON review_state
    forbidden_grants:               # 显式禁止（AISEC-REQ-009 强化）
      - INSERT, UPDATE, DELETE ON events
      - ANY GRANT TO PUBLIC
  dependencies:                     # 启动顺序依赖
    apps:
      - id: core-ai-gateway
        version: ">=1.0.0 <2.0.0"
    system:                          # 系统组件依赖
      - postgres: ">=14"
      - git-protocol: ">=2"
  upgrade:                          # 升级策略（见 13.7）
    strategy: rolling               # rolling | blue-green | canary
    health_check:
      endpoint: /api/apps/github-pr-reviewer/health
      timeout_seconds: 5
    rollback:                        # 回滚策略
      on_health_check_fail: true
      on_error_rate_above: 0.05
      cooldown_seconds: 60
  resource_limits:                   # 资源配额
    cpu_millicores: 500
    memory_mib: 512
    storage_mib: 1024
  observability:                     # 必填（§10）
    metrics_prefix: app_pr_reviewer
    span_attributes:
      - app.id
      - app.version
      - app.instance_id
status:                              # 运行时由平台填充（只读）
  state: installed                   # installed | upgrading | healthy | degraded | disabled
  installed_at: "2026-08-19T12:00:00Z"
  installed_by: "human:uli"
  last_health_check: "2026-08-19T18:00:00Z"
```

### 13.2.1 App 类型枚举

| `spec.type` | 含义 | 典型形态 |
|---|---|---|
| `webhook-consumer` | 订阅平台事件，触发回调 | PR/Issue 自动化、通知 |
| `http-api` | 暴露 HTTP 端点 | 同步 API 扩展 |
| `mcp-tool-provider` | 暴露 MCP 工具 | AI Agent 工具集 |
| `cli-extension` | CLI 子命令扩展 | `git <app>` 命令 |
| `storage-procedure` | 仅注册 PL/pgSQL 存储过程 | 数据层扩展 |
| `event-router` | 路由/转换事件 | 集成桥接（Slack、邮件）|
| `health-monitor` | 提供健康检查 | 自定义 SLO 监控 |

### 13.2.2 必填字段校验

`app.yaml` 入库（`apps` 表）前由 App Registry 服务（详细设计 §12）做 schema 校验，违反任一条规则即拒绝：

- [PROPOSAL-REQ-APP-001] `metadata.id` 必须 `^[a-z][a-z0-9-]{2,62}$` 且全局唯一
- [PROPOSAL-REQ-APP-002] `metadata.version` 必须符合 SemVer 2.0
- [PROPOSAL-REQ-APP-003] `spec.permissions.forbidden_grants` 必须为非空（deny-by-default）
- [PROPOSAL-REQ-APP-004] `spec.dependencies.apps` 不能形成循环（DAG 检测）
- [PROPOSAL-REQ-APP-005] `spec.entrypoints` 各 handler 必须在 App 包内实际存在
- [PROPOSAL-REQ-APP-006] `spec.consumes_events` + `spec.publishes_events` 不得自相矛盾

---

## 13.3 App Registry 存储模型

**[PROPOSAL]** App 注册表复用图谱引擎，App / AppInstance / AppDeployment 都是 Node subtype。详见详细设计 §12.1 DDL。

```
表 13.3 — App Registry 关键关系
┌────────────────────────────────────────────────────────────┐
│ type_registry (扩展自 §1.4.3)                              │
│   ├─ 'app'              → Node kind = App                 │
│   ├─ 'app_instance'     → Node kind = AppInstance        │
│   └─ 'app_deployment'   → Node kind = Event (immutable)  │
└────────────────────────────────────────────────────────────┘
        │
        ▼ Node 表（§1.4 已有）
┌────────────────────────────────────────────────────────────┐
│ nodes (扩展)                                                │
│   type='app' 时，properties JSONB 包含：                  │
│     {                                                       │
│       "manifest_yaml": "<完整 app.yaml>",                  │
│       "manifest_hash": "sha256:...",                       │
│       "state": "installed|upgrading|healthy|degraded|disabled",│
│       "current_version": "1.4.2",                          │
│       "available_version": "1.4.3",                        │
│       "instance_count": 3                                  │
│     }                                                       │
└────────────────────────────────────────────────────────────┘
        │
        ▼ Edge 表
┌────────────────────────────────────────────────────────────┐
│ edges (扩展)                                                │
│   - app_depends_on: App A → App B（A 启动需 B 先健康）  │
│   - app_instance_of: AppInstance → App                    │
│   - app_published_event: App → 中心事件类型              │
│   - app_consumes_event: App → 中心事件类型                │
└────────────────────────────────────────────────────────────┘
```

**AppInstance 唯一性约束（detail §12.1 DDL）**：

- `nodes` 表的 `id` 当 `type='app_instance'` 时 = `app_id` + `:` + `instance_uuid`（如 `github-pr-reviewer:7f3a-...`）
- 同一 App 允许 N 个 instance，但要求 `node_uniq(instance_id)` 唯一

---

## 13.4 中心事件总线 / Central Event Bus

**[PROPOSAL]** 中心事件总线 = **PostgreSQL LISTEN/NOTIFY + 现有 §7.6 Outbox + 现有 §7.7 Saga 引擎**的扩展，**不引入新组件**（NFR-REQ-003 强化：能 PG 解决不增加）。

### 13.4.1 事件 schema（统一 envelope）

所有跨 App 事件统一格式（继承 §7.6 Outbox envelope，扩展 App 维度字段）：

```json
{
  "event_id": "uuid-v7",
  "event_type": "pr.opened",         // 命名空间.动作
  "schema_version": 1,
  "occurred_at": "2026-08-19T18:00:00Z",
  "producer": {
    "app_id": "github-pr-reviewer",
    "app_version": "1.4.2",
    "instance_id": "github-pr-reviewer:7f3a-...",
    "trace_id": "...",
    "span_id": "..."
  },
  "tenant_id": "...",                // V1+ App 群组隔离（§6.x）
  "payload": { /* 事件特定 body */ },
  "metadata": {
    "causation_id": "...",           // 因果链上游
    "correlation_id": "...",         // 相关链
    "redelivery_count": 0,
    "schema_ref": "github-pr-reviewer/pr.opened@v1"
  }
}
```

### 13.4.2 事件分发路径

```
Publisher App                         中心事件总线                       Subscriber App(s)
═════════════                         ══════════════                      ═══════════════
                                     
[1] App 调用中心 SDK / 直写           ┌─────────────────┐
    trigger_event_publish()           │ outbox 表       │
         │                            │ (PG 现有)       │
         ▼                            └────────┬────────┘
[2] INSERT INTO outbox                          │
    (事务内，原子)                               │ LISTEN/NOTIFY
         │                                       │
         │                              ┌────────▼────────┐
         │                              │ Event Relay     │  (PG 现有)
         │                              │ 进程（单进程）  │
         │                              └────────┬────────┘
         │                                       │
         │                                       │ 查 subscribers
         │                                       │  (nodes WHERE
         │                                       │   type='app' AND
         │                                       │   consumes_events
         │                                       │   contains event_type)
         │                                       │
         │                              ┌────────▼────────┐
         │                              │ 派发 (HTTP webhook│
         │                              │  或本地进程内调用)│
         │                              └────────┬────────┘
[3] App 端 handler 处理                      │
    （可发新事件）                             │
         │                                    │
         └───────[4] ACK + dead-letter 监控 ──┘
```

### 13.4.3 关键不变量

- [PROPOSAL-REQ-EVT-001] **强一致性**：事件 publish 与业务变更在**同一 PG 事务**内提交（Outbox 模式）
- [PROPOSAL-REQ-EVT-002] **不丢**：Relay 进程崩溃重启后从 `outbox.processed_at IS NULL` 续传
- [PROPOSAL-REQ-EVT-003] **至少一次**：消费端需实现幂等（`event_id` 去重，参见 §3.7.4 模式）
- [PROPOSAL-REQ-EVT-004] **顺序保证**：同 `partition_key`（默认 `producer.app_id`）内严格有序；跨 partition 不保证
- [PROPOSAL-REQ-EVT-005] **schema 兼容**：发布端必须声明 `schema_ref`；订阅端校验自身声明的 schema_ref 与发布端匹配

### 13.4.4 Dead-Letter Queue（死信）

事件消费失败 N 次（默认 5 次，指数退避）后转入 `outbox_dead_letter` 表，触发 `dlq.alert` 事件。Admin UI（§14）展示并支持手动重投。

详细 DDL 与 Relay 实现见详细设计 §7.6 + §12.2。

---

## 13.5 App 集群模型

**[PROPOSAL]** 同一 App 的多个运行实例构成"集群"（cluster）。MVP 阶段：集群 = 同一进程内的多 instance（共享 PG，schema 隔离）。V1+ Cloud：集群 = K8s 多 Pod（独立进程，PG 共享）。

### 13.5.1 集群视图

```
App 集群 (例: github-pr-reviewer)
═══════════════════════════════════════════════════════════
  App metadata.id: github-pr-reviewer
  App metadata.version: 1.4.2
  State: healthy
  Instances: 3

  ┌────────────────────┐
  │ Instance 7f3a-001  │  ← leader (lock 持有)
  │  Status: healthy    │
  │  Last heartbeat: 1s │
  │  Active handlers: 2 │
  └────────────────────┘
  ┌────────────────────┐
  │ Instance 7f3a-002  │  ← hot standby
  │  Status: healthy    │
  │  Last heartbeat: 2s │
  └────────────────────┘
  ┌────────────────────┐
  │ Instance 7f3a-003  │  ← canary (10% 流量)
  │  Status: healthy    │
  │  Version: 1.4.3-rc  │
  │  Last heartbeat: 1s │
  └────────────────────┘
```

### 13.5.2 健康传播协议

每个 instance 每 5 秒（可配置）写心跳到 `app_heartbeats` 表：

```sql
INSERT INTO app_heartbeats (app_id, instance_id, ts, status, metrics)
VALUES (...)
ON CONFLICT (app_id, instance_id) DO UPDATE SET ts = EXCLUDED.ts, ...;
```

平台定时（每 10s）检查：
- 若 instance 心跳超过 30s 未更新 → 标记 `degraded`
- 若 instance 心跳超过 90s 未更新 → 标记 `failed`，触发 `instance_lost` 事件，Event Relay 自动从分发列表移除
- 若 instance 数量低于 `manifest.min_instances` → App 整体标记 `degraded`

### 13.5.3 Leader 选举

**[PROPOSAL]** 复用 §7.6.2 已有的 Outbox Relay leader lock 模式（PG advisory lock）。每个需要"主备"的 App 选 1 个 leader，hot standby 仅在 leader failed 时接管。

无中心协调服务 = 符合 "能 PG 解决不增加" 原则。

### 13.5.4 集群视图 = Node subgraph

- `App` 节点 = 集群抽象
- `AppInstance` 节点 = 单实例
- `app_instance_of` 边 = 集群成员关系
- Admin UI（§14）通过 View `apps_health_grid` 一次查询出所有 App 集群的拓扑与健康

---

## 13.6 热插拔与生命周期 / Hot Plug & Lifecycle

### 13.6.1 状态机

```
                    ┌──────────┐
                    │ draft    │  (Manifest 已解析，未安装)
                    └────┬─────┘
                         │ install
                         ▼
                    ┌──────────┐
       ┌────────────│ installed│────────────┐
       │ upgrade    └────┬─────┘  disable    │
       │                 │                   │
       ▼                 ▼                   ▼
  ┌──────────┐      ┌──────────┐        ┌──────────┐
  │upgrading │─────▶│ healthy  │        │ disabled │
  └────┬─────┘      └────┬─────┘        └────┬─────┘
       │ fail             │ degrade          │ enable
       ▼                  ▼                  ▼
  ┌──────────┐      ┌──────────┐        ┌──────────┐
  │ rolled   │      │ degraded │        │ installed│
  │ back     │      └──────────┘        └──────────┘
  └──────────┘
```

### 13.6.2 状态转换事件

每次状态转换产生一条 `app_deployment` 事件（不可变）：

```json
{
  "event_id": "uuid-v7",
  "event_type": "app.installed",
  "app_id": "github-pr-reviewer",
  "app_version": "1.4.2",
  "from_state": null,
  "to_state": "installed",
  "actor": "human:uli",
  "manifest_hash": "sha256:...",
  "trace_id": "..."
}
```

### 13.6.3 安装 / 升级 / 回滚 / 禁用 流程

| 操作 | 触发方式 | 流程 |
|---|---|---|
| **install** | Admin API `POST /admin/v1/apps` + Manifest | 校验 schema → 创建 `App` 节点 → 初始化存储过程 → 启动 instance → 注册订阅 → 状态 `installed` |
| **upgrade** | Admin API `PUT /admin/v1/apps/{id}` + 新 Manifest | 同上 + 旧 instance 走升级策略（§13.7） |
| **rollback** | Admin UI 按钮 / Admin API `POST /admin/v1/apps/{id}/rollback` | 立即切回 `current_version`，旧 instance 优雅停机，新 instance 启 `previous_version` |
| **disable** | Admin API `POST /admin/v1/apps/{id}/disable` | 取消事件订阅、停止接受新 HTTP 请求、优雅停机 → 状态 `disabled`（App 节点保留，可重新 enable）|
| **uninstall** | Admin API `DELETE /admin/v1/apps/{id}` | 同 disable + 删除 App 节点 + 清理存储过程（仅当 `manifest.persistence='purge-on-uninstall'`，否则保留）|

### 13.6.4 优雅停机

实例收到 SIGTERM / disable 信号后：
1. 停止接受新 HTTP/MCP 请求（立即拒绝 503）
2. 等待 in-flight 请求完成（最多 30s，可配置）
3. 取消事件订阅
4. flush outbox pending events
5. 写最终心跳 `status=draining`
6. 退出进程

---

## 13.7 升级策略 / Upgrade Strategy

**[PROPOSAL]** 升级策略由 `manifest.spec.upgrade.strategy` 决定，平台不做强约束。

### 13.7.1 rolling（默认）

```
old_v1 (100%) ──┐
                 ├─→ health check 5s ──→ 切换 20% 流量 ──→ health 5s ──→ 50% ──→ 100% ──→ 旧实例停机
new_v1 (0%)  ──┘
```

- 每步失败立即全量回滚到 old_v1
- 升级期间 2 个版本共存

### 13.7.2 blue-green

- 同时启动 new 与 old 实例
- 流量瞬时切换到 new
- 失败立即切回 old
- 资源消耗翻倍，零停机

### 13.7.3 canary

- 先启 1 个 new 实例（`canary` 标签）
- 流量按 `manifest.spec.upgrade.canary_traffic_percent`（默认 10%）路由
- 观察 5 分钟（可配置）→ 错误率 < 阈值 → 全量；否则回滚

### 13.7.4 错误率监控

升级过程中平台持续监控：
- 错误率（4xx + 5xx）超过 `rollback.on_error_rate_above`（默认 5%）
- 平均延迟超过 baseline 1.5 倍
- 事件 DLQ 数量激增

任一触发 → 自动回滚 + 写 `app_deployment` 事件 `to_state=rolled_back`。

---

## 13.8 与现有 §7/§11/§12 的关系

| 现有章节 | 与本章关系 |
|---|---|
| **§11 API 设计** | API 端点族扩展 `/admin/v1/*` + `/plugin/v1/*`（见 13.6 操作表）|
| **§7 安全设计** | App 沙箱 = AISEC-REQ-013（新增）；Admin 独立鉴权 = SEC-REQ-011（新增）|
| **§8 运维设计** | 补充 K8s 部署形态（V1+）；MVP 仍用 Local 单进程多 App |
| **§6 非功能设计** | 加 NFR-REQ-004（插件隔离 SLA）/ NFR-REQ-005（集群健康传播 ≤ 30s）/ NFR-REQ-006（中心事件端到端 ≤ 5s）/ NFR-REQ-007（Admin 操作双因素）|
| **§12 App 群组信息互通** | §12 的"多 App 协调" = 本章中心事件总线的 V0（无 Manifest、无 Registry、无集群视图）；§12 内容作为"已存在的能力"被本章吸收，不重写 |
| **§7 App 协调（详细）** | 详细设计 §7.4 PL/pgSQL 加"按 App 命名空间"；§7.6/§7.7 不变 |

**向前兼容**：

- 现有 §12 的协调场景（多 App 信息互通）继续工作
- 现有 §7.6 Outbox / §7.7 Saga 不破坏
- 升级是**叠加**（adoption），不是替换（migration）

---

## 13.9 关键 REQ-ID 新增

为支持 App 集群与可热插拔架构，本章新增 11 条 REQ：

| REQ-ID | 简述 | 章节 |
|---|---|---|
| APP-REQ-001 | App 升格为一级 Node subtype | §13.1 |
| APP-REQ-002 | App Manifest schema 必填字段校验 | §13.2.2 |
| APP-REQ-003 | AppInstance 一实例一节点 + 唯一性 | §13.3 |
| APP-REQ-004 | 中心事件总线复用 PG Outbox + LISTEN/NOTIFY | §13.4 |
| APP-REQ-005 | 事件 envelope 包含 producer / trace / schema_ref | §13.4.1 |
| APP-REQ-006 | 事件强一致性 + 至少一次 + 顺序保证 | §13.4.3 |
| APP-REQ-007 | App 集群健康心跳 ≤ 30s 检测 | §13.5.2 |
| APP-REQ-008 | App 升级支持 rolling/blue-green/canary | §13.7 |
| APP-REQ-009 | App 安装/升级需人类审批（继承 §4.9 模式）| §13.0 / §13.6 |
| APP-REQ-010 | 升级错误率/延迟超阈值自动回滚 | §13.7.4 |
| APP-REQ-011 | App 优雅停机 30s + flush outbox | §13.6.4 |

详细 DDL、SDK、API 端点目录见 [详细设计 §12. App Registry & Plugin Loader](../detailed-design/12-app-registry-and-plugin-loader.md)。

---

**导航 / Navigation:**
[← 12. App 群组信息互通设计](12-app-group-intercommunication.md) · [README](README.md) · [14. 管理员运维界面 →](14-admin-ops-ui.md)
