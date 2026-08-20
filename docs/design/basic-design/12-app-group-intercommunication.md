# 12. App 群组信息互通设计 / App Group Intercommunication Design

> **[PROPOSAL] IPA 共通框架 2013 章节定位：** P3.A2.T8 研讨 App 群组信息互通方式 (补充)


> 本章定义"多个应用（包括平台内部子系统、外部集成应用、同部署内的关联服务）如何通过平台实现受控、有序、可观测的信息互通"。重点设计**存储过程（PL/pgSQL）作为协调中枢**这一架构选择，以及与 LISTEN/NOTIFY、Outbox、Saga 模式的边界与组合。

## 12.1 背景与动机

### 12.1.1 现状约束

本平台采用 **monolith-first** 设计（参见 [§2.2 部署单位](02-architecture.md#22-部署单位-deployable-units来自-phase-10-4)），单个 Platform Process 内 8 个子系统通过**函数调用**通信，不存在跨进程协调需求。但以下三类场景在 V1 / Cloud 阶段必然出现：

| 场景 | 例子 |
|---|---|
| **水平扩缩** | Cloud 模式下 N 个 Platform Process 实例同时跑（[§2.3.2 Cloud 部署](02-architecture.md#232-cloud-部署v1-正式化mvp-不需要)），同一资源被两个实例同时改的风险 |
| **外部 App 群组** | 客户公司把平台当作团队内多个内部服务（CI、文档、监控、Issue 跟踪）的**共享数据底座**，这些服务需要相互看到对方写入的数据 |
| **跨域工作流** | 一个 PR 流程会触发 5+ 个系统动作（CI → AI 评审 → 安全扫描 → 自动 merge 候选 → 通知），这些动作分布在不同应用中 |

### 12.1.2 设计目标

让"应用群组"内的数据流满足：

1. **原子性** — 跨多个表 / 多个对象的写入要么全成功要么全失败
2. **可观测** — 任何协调动作都留下 Event，可在 Audit Log 查
3. **Policy 统一** — 不绕过授权（即便走存储过程也要走 Policy 引擎）
4. **可恢复** — 协调中途失败有明确恢复路径（saga / outbox）
5. **不发明协议** — 优先用 PostgreSQL 原生能力（PL/pgSQL、LISTEN/NOTIFY、CTE、SKIP LOCKED），不引入 Kafka 之类

## 12.2 App 群组的定义

**[PROPOSAL]** 一个 **App Group** 是满足以下条件的应用集合：

| 属性 | 说明 |
|---|---|
| 共同身份 | 共享一个 `app_group_id`（UUID v7），由平台管理员显式创建 |
| 共同数据契约 | 群组内每个 app 共享一套**已声明的协调点（coordination points）**：存储过程 / 事件通道 / 共享表视图 |
| 共同 Policy 命名空间 | 群组级 RBAC 规则统一管理，不在每个 app 内部复制 |
| 共同审计范围 | 群组内所有 app 产生的 Event 自动归到同一可查询视图 |

**示例群组：**
- `app_group:acme-cicd` — 包含 `app:ci-runner`、`app:notifier`、`app:pr-bot`
- `app_group:acme-observability` — 包含 `app:metrics-exporter`、`app:trace-collector`
- `app_group:platform-internal` — 包含 `app:ai-gateway`、`app:context-engine`、`app:graph-core`（即 Platform Process 内部子系统）

## 12.3 互通模式全景

```
┌─────────────────────────────────────────────────────────────────────┐
│                        App Group: acme-cicd                         │
│                                                                     │
│  ┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐ │
│  │  ci-     │     │  noti-   │     │  pr-bot  │     │  scan-   │ │
│  │  runner  │     │  fier    │     │  (Agent) │     │  sec     │ │
│  │  (App)   │     │  (App)   │     │  (App)   │     │  (App)   │ │
│  └────┬─────┘     └────┬─────┘     └────┬─────┘     └────┬─────┘ │
│       │                 │                │                │       │
│       │   ① Stored Proc │ ② LISTEN/     │ ③ Outbox       │ ④ Saga│
│       │     (sync txn)  │    NOTIFY     │    (guaranteed │   (long)│
│       │                 │    (async)    │     delivery)  │       │
│       └────────┬────────┴────────┬──────┴────────┬───────┘       │
│                │                 │               │                │
│                ▼                 ▼               ▼                │
│         ┌─────────────────────────────────────────────────┐       │
│         │   PostgreSQL 14+ (shared state + event log)     │       │
│         │   - nodes / edges / events / app_groups         │       │
│         │   - PL/pgSQL stored procs (coordination points)  │       │
│         │   - LISTEN/NOTIFY channels                      │       │
│         │   - outbox table                                │       │
│         └─────────────────────────────────────────────────┘       │
└─────────────────────────────────────────────────────────────────────┘
```

### 12.3.1 模式选择决策表

| 需求 | 推荐模式 | 理由 |
|---|---|---|
| 多表原子写 | **① 存储过程** | 单一事务，避免应用层多步调用中途崩溃 |
| 异步通知，发送方不需等接收方 | **② LISTEN/NOTIFY** | 零延迟、零新组件 |
| 必须可靠送达外部（HTTP webhook、SMTP） | **③ Outbox + Relay** | 数据库事务与外部投递解耦 |
| 长流程（> 1 分钟）、多步、可补偿 | **④ Saga** | 每步独立事务 + 补偿动作 |
| 互斥资源访问（同一时刻只能一个 writer） | ⑤ Advisory Lock | PostgreSQL 原生，开销极小 |

> **核心立场：** 存储过程承担**同步 / 事务性**协调；LISTEN/NOTIFY 承担**快速通知**；Outbox 承担**跨边界可靠投递**；Saga 承担**长流程**。四种模式互补，不互斥。

## 12.4 模式 ①：存储过程作为协调中枢

### 12.4.1 为什么选存储过程

[INFERENCE] 业界对"业务逻辑是否该放进数据库"长期争议。本平台的立场：

| 适合放进存储过程 | 不适合放进存储过程 |
|---|---|
| 跨多表的强一致写入 | 业务规则多变、需要频繁发版的逻辑 |
| 必须在数据库层强制的不变量 | 需要调用外部 HTTP / AI 服务的逻辑 |
| 高频执行、数据库内即可完成的复合查询 | 需要丰富表达力（递归、模式匹配）的复杂判断 |
| 跨 App 共享的"协调语义" | App 自身专属的内部状态机 |

App Group 互通天然属于前者：协调点是**跨 App 共享的契约**，且经常涉及多表写入。让它存在于数据库内部，能确保**没有 App 能绕过协调直接写底层表**。

### 12.4.2 存储过程分类

| 类别 | 用途 | 示例 |
|---|---|---|
| **C — Coordination（协调）** | 跨表原子写入多对象，触发相应 Event | `pr_merge_with_checks(repo_id, pr_id, actor_id)` → 写 PR Node + Merge Edge + Approval Event + 触发 webhook outbox |
| **Q — Query（查询）** | 复杂多步查询，结果集作为统一契约 | `get_pr_review_context(pr_id)` → 返回 PR + 关联 Issue + 评审 + CI 状态 + Evidence 链 |
| **T — Transform（变换）** | 受控的状态转换（状态机转移） | `transition_issue_state(issue_id, from_state, to_state, actor_id, reason)` → 校验转移合法性 + 写 Edge + Event |
| **S — Sync（同步）** | 跨数据源同步（Git obj ↔ graph Node） | `sync_commit_to_graph(commit_sha, repo_id)` → 写 Commit Node + 与 Branch/Author 建 Edge |

### 12.4.3 存储过程命名与签名规约

**[PROPOSAL]**

```
{verb}_{noun}_{qualifier} (p_param_name IN type, p_actor_id IN uuid, OUT o_result jsonb)
```

- `verb ∈ {get_, list_, create_, update_, delete_, transition_, merge_, sync_, archive_}`
- 第一个参数永远是 `p_actor_id`（人或 Agent 的 Node ID，用于 Policy 决策）
- 输出统一 `jsonb`，便于跨语言消费
- 不返回错误用 RAISE EXCEPTION；客户端拿到 SQLSTATE

### 12.4.4 安全边界

**关键原则：存储过程必须与 HTTP API 等价安全，绝不能成为 Policy 绕过路径。**

| 控制 | 实施 |
|---|---|
| 调用前 Policy 评估 | 存储过程**不**自己做权限判断，调用前由应用层（Platform Process 或外部 App）走 Policy 引擎；Policy 评估通过的 token 写入 `app.rls_context` |
| 行级安全 (RLS) | 启用 PostgreSQL RLS，所有 `nodes` / `edges` / `events` 表按 `app_group_id` + `permissions` 过滤；存储过程在 SECURITY DEFINER 模式下也要遵守 RLS |
| 调用审计 | 每个存储过程调用写一条 `coordination.proc_invoked` Event，含 `proc_name, params_hash, actor_id, duration, result_count` |
| 限流 | `app_group` 维度每分钟调用次数上限，由 `app_group_quotas` 表控制 |
| 模式白名单 | 只能调 `coord_` / `query_` / `trans_` / `sync_` 前缀的存储过程，禁止 `pg_*` / `lo_*` / 任意 PL 注入 |

### 12.4.5 端点形式

存储过程通过专用 endpoint 暴露，不混入通用 API：

```http
POST /api/v1/coordinate/{proc-name}
Authorization: Bearer <actor_token>
Content-Type: application/json
X-App-Group-Id: <uuid>

{
  "params": {
    "p_repo_id": "0190e8a4-...",
    "p_pr_id": "0190e8a4-...",
    "p_decision": "approve"
  }
}
```

**响应：**

```json
{
  "result": { ... jsonb 返回 ... },
  "audit_event_id": "0190e8a4-...",
  "duration_ms": 12
}
```

### 12.4.6 一个完整示例：PR 合并的协调存储过程

```sql
-- 模式 ① C 类（协调）
-- 用途：合并 PR 并原子地完成所有相关写入

CREATE OR REPLACE FUNCTION coord_merge_pr(
    p_pr_id       UUID,
    p_actor_id    UUID,
    p_actor_type  TEXT,    -- 'human' or 'agent'
    p_sha         TEXT     -- merge commit SHA (空 = server 算)
) RETURNS JSONB
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = 'public'
AS $$
DECLARE
    v_pr         RECORD;
    v_repo_id    UUID;
    v_audit_id   UUID;
    v_seq        BIGINT;
    v_result     JSONB;
BEGIN
    -- 1. 锁定 PR 行 (防止并发 merge)
    SELECT * INTO v_pr
    FROM nodes
    WHERE id = p_pr_id
      AND node_type = 'pull_request'
      AND deleted_at IS NULL
    FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'PR not found: %', p_pr_id
            USING ERRCODE = 'resource_not_found';
    END IF;
    v_repo_id := (v_pr.properties->>'repo_id')::UUID;

    -- 2. 状态校验（仅 open/approved 可合并）
    IF v_pr.properties->>'state' NOT IN ('open', 'approved') THEN
        RAISE EXCEPTION 'PR is in non-mergeable state: %',
            v_pr.properties->>'state'
            USING ERRCODE = 'resource_state_conflict';
    END IF;

    -- 3. CI 必须通过
    IF NOT EXISTS (
        SELECT 1 FROM edges e
        JOIN nodes n ON n.id = e.to_node_id
        WHERE e.from_node_id = p_pr_id
          AND e.edge_type = 'has_ci_run'
          AND n.properties->>'status' = 'success'
    ) THEN
        RAISE EXCEPTION 'CI has not passed for this PR'
            USING ERRCODE = 'policy_denied';
    END IF;

    -- 4. 原子地写 PR 状态变更 + 合并 Edge + Audit Event
    UPDATE nodes
    SET properties = jsonb_set(properties, '{state}', '"merged"'),
        updated_at = now()
    WHERE id = p_pr_id;

    INSERT INTO edges (from_node_id, to_node_id, edge_type, properties)
    VALUES (p_pr_id, v_repo_id, 'merged_into',
            jsonb_build_object('actor_id', p_actor_id,
                              'actor_type', p_actor_type,
                              'merged_at', now()));

    -- 5. 写协调审计 Event (单事务内)
    INSERT INTO events (subject_node_id, event_type, actor_id, payload)
    VALUES (p_pr_id, 'coordination.proc_invoked', p_actor_id,
            jsonb_build_object(
                'proc_name', 'coord_merge_pr',
                'params_hash', encode(digest(p_pr_id::text || p_actor_id::text, 'sha256'), 'hex'),
                'caller_type', p_actor_type
            ))
    RETURNING seq INTO v_seq;

    -- 6. 提交到 outbox（webhook/通知异步发）
    INSERT INTO outbox (event_type, payload, target_filter)
    VALUES ('pr.merged',
            jsonb_build_object('pr_id', p_pr_id, 'repo_id', v_repo_id, 'actor_id', p_actor_id),
            jsonb_build_object('repo_id', v_repo_id));

    v_audit_id := gen_random_uuid();
    v_result := jsonb_build_object(
        'pr_id', p_pr_id,
        'state', 'merged',
        'audit_event_seq', v_seq,
        'webhook_outbox_id', v_audit_id
    );
    RETURN v_result;
END;
$$;

-- 强制 RLS 不被绕过
ALTER FUNCTION coord_merge_pr(UUID, UUID, TEXT, TEXT) SECURITY DEFINER;
REVOKE EXECUTE ON FUNCTION coord_merge_pr FROM PUBLIC;
GRANT EXECUTE ON FUNCTION coord_merge_pr TO platform_app_role;
```

**事务保证：** 整个 PL/pgSQL 函数体在单事务中执行，CRUD Edge + Event + outbox 全部成功或全部回滚。应用层只需要"调一次"，没有"先 A 再 B 再 C"的多步协调崩溃风险。

### 12.4.7 存储过程的版本治理

| 维度 | 规则 |
|---|---|
| 命名空间 | 按群组：`app_group.{group_slug}.{verb}_{noun}` 或全局 `coord_{verb}_{noun}` |
| 弃用 | `pg_proc` 表 `prosrc` 加注释头 `-- DEPRECATED: use coord_v2_merge_pr; sunset 2027-01-01` |
| 跨版本兼容 | 增参默认有 DEFAULT；删参需走新名（`coord_v2_*`） |
| 回滚 | 用 `pg_create_restore_point()` 在每次发布前打点，错误时 `pg_restore_point()` 恢复 |
| 测试 | 每个存储过程必须有 plpgsql 单元测试（`pgTAP` 或自写 SQL 脚本），跑在 CI |

## 12.5 模式 ②：LISTEN/NOTIFY 异步通知

### 12.5.1 用法

**[PROPOSAL]** 在需要"通知但不等待"的场景使用，例如：
- `notify_pr_state_changed(pr_id)` 触发 UI 的实时刷新
- `notify_agent_run_progress(run_id)` 让上游服务更新自己的 dashboard

```sql
-- 在存储过程末尾
PERFORM pg_notify(
    'pr_state_changed',
    json_build_object('pr_id', p_pr_id, 'new_state', 'merged')::text
);
```

### 12.5.2 限制

| 限制 | 影响 |
|---|---|
| Payload 上限 8 KB | 复杂通知要走 outbox 表 |
| 进程重启时丢失未消费的通知 | 关键通知必须配合 outbox 双写 |
| 同一 channel 多 listener 顺序无保证 | 通知不可作为业务决策的唯一依据 |

## 12.6 模式 ③：Outbox + Relay 跨边界可靠投递

**[PROPOSAL]** 涉及外部系统（webhook、SMTP、Slack、IM bot）的投递走 outbox 模式：

```
┌────────────┐   同一事务写   ┌────────────┐   relay poll    ┌────────────┐
│ Storage    │ ───────────▶  │  outbox    │ ──────────────▶ │  HTTP/SMTP │
│ Procedure  │                │  table     │  at-least-once  │  target    │
└────────────┘                └────────────┘  retry exp.back │            │
                                  ▲                          └────────────┘
                                  │
                                  │ on success → DELETE
                                  │ on permanent fail → 'dlq'
```

**outbox 表 schema：**

```sql
CREATE TABLE outbox (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type      TEXT NOT NULL,
    payload         JSONB NOT NULL,
    target_filter   JSONB,                  -- 决定投递给谁
    target_url      TEXT,                   -- 实际 URL（relay 填入）
    status          TEXT DEFAULT 'pending', -- pending | in_flight | sent | dlq
    attempts        INT DEFAULT 0,
    last_error      TEXT,
    next_attempt_at TIMESTAMPTZ DEFAULT now(),
    created_at      TIMESTAMPTZ DEFAULT now(),
    sent_at         TIMESTAMPTZ
);
CREATE INDEX idx_outbox_pending ON outbox (next_attempt_at) WHERE status = 'pending';
```

**relay 进程：**
- 单实例运行（由 advisory lock 选主，防止多 Platform Process 同时 relay）
- 每 5s 扫描 `WHERE status = 'pending' AND next_attempt_at <= now() LIMIT 100`
- 用 `SELECT ... FOR UPDATE SKIP LOCKED` 取任务，多 relay 实例安全并发
- 投递成功 → `status = 'sent'`；失败 → 指数退避 `next_attempt_at = now() + 2^attempts`；7 次后 → `dlq`

## 12.7 模式 ④：Saga 长流程

**[PROPOSAL]** 跨多个 App、跨较长时间（> 1 分钟）的工作流用 Saga 模式，例如 "PR 提交后启动评审 → CI → 安全扫描 → 自动 merge 候选 → 通知"。

**实现方式：**

```sql
CREATE TABLE saga_instances (
    id            UUID PRIMARY KEY,
    saga_type     TEXT NOT NULL,        -- 'pr_full_lifecycle'
    state         TEXT NOT NULL,        -- 'pending' | 'running' | 'completed' | 'compensating' | 'failed'
    current_step  TEXT,
    steps_total   INT,
    steps_done    INT DEFAULT 0,
    payload       JSONB NOT NULL,
    started_at    TIMESTAMPTZ DEFAULT now(),
    last_event_at TIMESTAMPTZ DEFAULT now(),
    completed_at  TIMESTAMPTZ,
    error         TEXT
);

CREATE TABLE saga_step_log (
    saga_id    UUID,
    step_name  TEXT,
    started_at TIMESTAMPTZ,
    ended_at   TIMESTAMPTZ,
    status     TEXT,  -- 'success' | 'failed' | 'compensated'
    result     JSONB,
    error      TEXT
);
```

**驱动：** Platform Process 内置 Saga Engine，扫描 `state = 'running'` 的实例，调度下一步。每个 step 调用一个存储过程或外部 API。失败时按 saga 定义执行补偿动作。

## 12.8 数据一致性边界总结

```
┌─────────────────────────────────────────────────────────────────┐
│  强一致 (ACID)                                                  │
│  - 同一事务内的多表写 → 存储过程 / 单次事务                     │
│  - 适用范围：节点+边+事件+outbox 的组合写                       │
├─────────────────────────────────────────────────────────────────┤
│  最终一致 (within seconds)                                      │
│  - LISTEN/NOTIFY 通知接收方 → 接收方重新读 DB 校对              │
│  - 适用范围：UI 实时刷新、跨实例 cache 失效                    │
├─────────────────────────────────────────────────────────────────┤
│  最终一致 (within minutes)                                      │
│  - Outbox + relay 投递外部系统                                  │
│  - 适用范围：Webhook、IM 通知、外部系统同步                    │
├─────────────────────────────────────────────────────────────────┤
│  最终一致 (within hours)                                        │
│  - Saga 跨步骤状态推进                                          │
│  - 适用范围：长流程 (PR 全生命周期、Agent 训练 pipeline)        │
└─────────────────────────────────────────────────────────────────┘
```

## 12.9 跨 App Group 互通

**[PROPOSAL]** 不同 App Group 之间默认**数据隔离**。互通通过显式的 **cross_group_policy** 表达：

```sql
CREATE TABLE cross_group_policies (
    source_group    UUID,
    target_group    UUID,
    resource_type   TEXT,        -- 'node' | 'edge' | 'event'
    allowed_actions TEXT[],      -- ['read', 'reference', 'link']
    expires_at      TIMESTAMPTZ
);
```

读取时 RLS 引擎检查 `cross_group_policies`；通过后该资源对源群组可见。

## 12.10 可观测性

### 12.10.1 监控指标

| 指标 | 类型 | 用途 |
|---|---|---|
| `coordination.proc.duration` | histogram | 存储过程耗时 |
| `coordination.proc.error_rate` | counter | 错误率（按 proc 名分桶）|
| `coordination.outbox.lag` | gauge | pending → sent 时间差 |
| `coordination.outbox.dlq` | counter | 死信累计 |
| `coordination.saga.running` | gauge | 运行中 saga 数 |
| `coordination.saga.failed` | counter | 失败 saga 数 |
| `coordination.locks.held` | gauge | 当前持有的 advisory lock 数 |

### 12.10.2 关键 SQL 视图（运维自服务）

```sql
-- 跨 App Group 互通概览
CREATE VIEW v_cross_group_flow AS
SELECT
    cgp.source_group,
    cgp.target_group,
    cgp.resource_type,
    COUNT(e.id) AS interactions_24h
FROM cross_group_policies cgp
LEFT JOIN events e
    ON e.event_type LIKE 'cross_group.%'
   AND e.occurred_at > now() - interval '24 hours'
GROUP BY cgp.source_group, cgp.target_group, cgp.resource_type;

-- 慢存储过程 top 10
CREATE VIEW v_slow_procs AS
SELECT
    payload->>'proc_name' AS proc,
    AVG((payload->>'duration_ms')::numeric) AS avg_ms,
    COUNT(*) AS calls
FROM events
WHERE event_type = 'coordination.proc_invoked'
  AND occurred_at > now() - interval '1 hour'
GROUP BY 1
ORDER BY avg_ms DESC
LIMIT 10;
```

## 12.11 反模式（明确禁止）

| 反模式 | 后果 | 替代 |
|---|---|---|
| 业务逻辑全塞存储过程 | DB 成为单点、无法独立部署、版本治理噩梦 | 存储过程仅做**协调**与**不变量强制** |
| LISTEN/NOTIFY 作为业务决策唯一依据 | 消息丢失导致数据漂移 | 关键通知双写 outbox |
| App 直接互相调 HTTP（绕过平台） | 失去 Policy / Audit / 鉴权 | 全部走平台 API 或存储过程 |
| Saga 不写补偿 | 失败后无恢复路径 | 每个 forward step 必有 compensate step |
| 跨群组默认互通 | 数据泄露、Privacy 灾难 | 默认隔离 + 显式 cross_group_policy |

## 12.12 演进路径

| 阶段 | 内容 |
|---|---|
| **MVP** | 模式 ① 存储过程（仅内部子系统用，不暴露给外部 App），模式 ② NOTIFY（内部） |
| **V1** | 模式 ① 暴露 `/api/v1/coordinate/*` 端点，模式 ③ Outbox 上线，relay 内置 |
| **V2** | 模式 ④ Saga Engine 独立服务，模式 ① 暴露 sandbox（允许临时 PL/pgSQL 调试） |
| **V3+** | 多 PostgreSQL 实例时引入 Logical Replication / CDC |

---

**导航：**
[← 11. API 设计](11-api-design.md) · [README](README.md) · [Appendix A — 追踪表 →](appendix-a-traceability.md)
