# 01. 数据层 / Data Layer

## 1.1 目标 / Purpose

本章定义 PostgreSQL 数据库的**完整 schema、索引策略、行级安全（RLS）、DB role 分离（AISEC-REQ-009(a)）和迁移流程**。所有上层模块（Graph Engine、Policy Engine、Agent Runtime 等）的实现都基于本章的 schema。

## 1.2 命名与规约 / Naming Convention

**[PROPOSAL]**

| 类别 | 规约 | 示例 |
|---|---|---|
| 表名 | 复数 snake_case | `nodes`, `edges`, `events` |
| 列名 | snake_case | `node_type`, `created_at` |
| 主键 | `id`（UUID v7）| `id UUID PRIMARY KEY` |
| 外键 | `{referenced_table_singular}_id` | `from_node_id UUID REFERENCES nodes(id)` |
| 时间戳 | `created_at`, `updated_at`, `occurred_at` | TIMESTAMPTZ, NOT NULL DEFAULT now() |
| 布尔 | `is_*` 或 `active` | `active BOOLEAN NOT NULL DEFAULT true` |
| 枚举 | TEXT + CHECK 约束 | `status TEXT CHECK (status IN (...))` |
| 索引 | `idx_{table}_{columns}` | `idx_nodes_type` |
| 唯一约束 | `uniq_{table}_{columns}` | `uniq_users_username` |
| 检查约束 | `chk_{table}_{rule}` | `chk_evidence_epistemic` |

## 1.3 DB role 分离（AISEC-REQ-009(a) MVP 必填）

**[PROPOSAL]** 这是 Phase 11 RT-10（红队最大发现）的核心防御：Platform 进程的常规 DB role 对 `events` 表**只授 INSERT**，无 UPDATE/DELETE。详细原理见基本设计 [§7.2 审计](../basic-design/07-security-design.md#72-审计-audit)。

### 1.3.1 role 矩阵

| Role | 用途 | 权限范围 |
|---|---|---|
| `platform_owner` | 启动时 migration、admin 任务 | ALL（仅 DBA 持有）|
| `platform_runtime` | **Platform 进程运行时使用** | 全部表的 SELECT/INSERT/UPDATE/DELETE，**但 events 表只有 INSERT** |
| `platform_readonly` | 备份、报表、debug | 全部表的 SELECT |
| `platform_outbox_relay` | Outbox relay 进程 | `outbox` SELECT/UPDATE，外部目标投递 |
| `platform_saga` | Saga engine | `saga_instances`, `saga_step_log` 全部权限 |
| `platform_external_app` | 外部 App 群组调用存储过程 | 预定义存储过程 EXECUTE |

### 1.3.2 创建 role 的 SQL（必须在 migration 初始运行）

```sql
-- 001_roles.sql
-- 注意: 此 SQL 由 platform_owner 执行, 不在 platform_runtime 上下文中运行

-- 1. 创建 role
CREATE ROLE platform_runtime LOGIN PASSWORD 'CHANGE_ME_VIA_SECRET';
CREATE ROLE platform_readonly LOGIN PASSWORD 'CHANGE_ME_VIA_SECRET';
CREATE ROLE platform_outbox_relay LOGIN PASSWORD 'CHANGE_ME_VIA_SECRET';
CREATE ROLE platform_saga LOGIN PASSWORD 'CHANGE_ME_VIA_SECRET';
CREATE ROLE platform_external_app LOGIN PASSWORD 'CHANGE_ME_VIA_SECRET';

-- 2. 默认权限: schema usage + 通用表读
GRANT USAGE ON SCHEMA public TO platform_runtime, platform_readonly,
    platform_outbox_relay, platform_saga, platform_external_app;

-- 3. platform_readonly: 全部表 SELECT
GRANT SELECT ON ALL TABLES IN SCHEMA public TO platform_readonly;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES TO platform_readonly;

-- 4. platform_runtime: 全部表 SELECT/INSERT/UPDATE/DELETE
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO platform_runtime;
ALTER DEFAULT PRIVILEGES IN SCHEMA public
    GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO platform_runtime;

-- 5. ★ events 表: 平台 runtime 仅 INSERT ★
REVOKE UPDATE, DELETE ON TABLE events FROM platform_runtime;
ALTER TABLE events ENABLE ROW LEVEL SECURITY;
-- RLS policy: runtime 可看到所有 event, 但只能 INSERT, 不能改写
CREATE POLICY events_runtime_read ON events
    FOR SELECT TO platform_runtime USING (true);
CREATE POLICY events_runtime_insert ON events
    FOR INSERT TO platform_runtime WITH CHECK (true);
-- UPDATE/DELETE 没有 policy, 等同拒绝

-- 6. platform_outbox_relay: 仅 outbox 表 SELECT/UPDATE
GRANT SELECT, UPDATE ON TABLE outbox TO platform_outbox_relay;
-- INSERT 仍由 platform_runtime 完成

-- 7. platform_saga: saga 两张表
GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE saga_instances, saga_step_log TO platform_saga;
-- 同样: events 表只允许 SELECT/INSERT (与 runtime 共享)

-- 8. platform_external_app: 仅 EXECUTE 存储过程 (见 07)
-- 存储过程的 EXECUTE 单独 GRANT, 不需要表权限
```

### 1.3.3 防御验证 / Defense Verification

**[PROPOSAL]** 在 CI 中必须有一个**集成测试**验证：

```go
// [IMPL] 伪代码 (Go)
func TestEventsImmutabilityForPlatformRole(t *testing.T) {
    db := connectAs(t, "platform_runtime")
    _, err := db.Exec("UPDATE events SET event_type='tampered' WHERE seq=1")
    require.Error(t, err, "runtime role must not UPDATE events")

    _, err = db.Exec("DELETE FROM events WHERE seq=1")
    require.Error(t, err, "runtime role must not DELETE events")
}

func TestEventsWritableForOwnerRole(t *testing.T) {
    db := connectAs(t, "platform_owner")
    // owner 可写 (admin 任务)
    _, err := db.Exec("UPDATE events SET payload='{}' WHERE seq=1")
    require.NoError(t, err)
}
```

该测试是 **MVP 发布 non-negotiable**（同 AISEC-REQ-009(a)）。

## 1.4 完整 DDL

### 1.4.1 5 原语核心表 / Primitive Tables

```sql
-- 002_primitives.sql

-- 节点表
CREATE TABLE nodes (
    id              UUID PRIMARY KEY,
    node_type       TEXT NOT NULL,
    properties      JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at      TIMESTAMPTZ
);

-- 边表
CREATE TABLE edges (
    id              UUID PRIMARY KEY,
    from_node_id    UUID NOT NULL REFERENCES nodes(id) ON DELETE RESTRICT,
    to_node_id      UUID NOT NULL REFERENCES nodes(id) ON DELETE RESTRICT,
    edge_type       TEXT NOT NULL,
    properties      JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT chk_no_self_edge CHECK (from_node_id <> to_node_id)
);

-- 事件表 (不可变，按月声明式范围分区)
CREATE TABLE events (
    id               UUID NOT NULL DEFAULT gen_random_uuid(),
    seq              BIGSERIAL,
    subject_node_id  UUID REFERENCES nodes(id),
    event_type       TEXT NOT NULL,
    actor_id         UUID,
    payload          JSONB NOT NULL DEFAULT '{}'::jsonb,
    occurred_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (id, occurred_at)
) PARTITION BY RANGE (occurred_at);

-- 默认兜底分区与按月分区示例
CREATE TABLE events_default PARTITION OF events DEFAULT;
-- 实际生产环境中由 migration/cron 提前创建:
-- CREATE TABLE events_y2026m08 PARTITION OF events 
--     FOR VALUES FROM ('2026-08-01 00:00:00+00') TO ('2026-09-01 00:00:00+00');

-- 策略表
CREATE TABLE policies (
    id              UUID PRIMARY KEY,
    policy_type     TEXT NOT NULL,
    rules           JSONB NOT NULL,
    version         INT NOT NULL,
    active          BOOLEAN NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT chk_policies_version_positive CHECK (version > 0)
);

-- 视图表
CREATE TABLE views (
    id              UUID PRIMARY KEY,
    view_type       TEXT NOT NULL,
    parameters      JSONB NOT NULL,
    owner_id        UUID REFERENCES nodes(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

### 1.4.2 权限表 / Permissions

```sql
-- 003_permissions.sql
CREATE TABLE users (
    id              UUID PRIMARY KEY,
    username        TEXT NOT NULL UNIQUE,
    password_hash   TEXT,                   -- Argon2id; SSH-only 用户为 NULL
    ssh_public_key  TEXT,
    is_agent        BOOLEAN NOT NULL DEFAULT false,
    is_disabled     BOOLEAN NOT NULL DEFAULT false,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_login_at   TIMESTAMPTZ
);

CREATE TABLE roles (
    id              UUID PRIMARY KEY,
    name            TEXT NOT NULL UNIQUE,
    description     TEXT
);

CREATE TABLE user_roles (
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id         UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    granted_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    granted_by      UUID REFERENCES users(id),
    PRIMARY KEY (user_id, role_id)
);

-- ABAC: 属性基础访问控制
CREATE TABLE permissions (
    id              UUID PRIMARY KEY,
    subject_id      UUID NOT NULL,           -- Human or Agent (User.id 或 Agent Node.id)
    subject_type    TEXT NOT NULL CHECK (subject_type IN ('user','agent_node')),
    resource_id     UUID,                    -- 资源节点 ID (NULL = 通配)
    resource_type   TEXT,                    -- 资源类型 (NULL = 通配)
    action          TEXT NOT NULL,           -- 'read','write','delete','merge','invoke',...
    effect          TEXT NOT NULL CHECK (effect IN ('allow','deny')),
    conditions      JSONB NOT NULL DEFAULT '{}'::jsonb,  -- 高级条件 (e.g. 时间窗, IP 段)
    expires_at      TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

### 1.4.3 类型注册表 / Type Registry

```sql
-- 004_type_registry.sql
-- 节点类型注册 (动态 schema 校验依据)
CREATE TABLE node_type_registry (
    node_type       TEXT PRIMARY KEY,
    schema          JSONB NOT NULL,           -- JSON Schema
    description     TEXT,
    is_abstract     BOOLEAN NOT NULL DEFAULT false,
    parent_type     TEXT REFERENCES node_type_registry(node_type),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE edge_type_registry (
    edge_type       TEXT PRIMARY KEY,
    from_types      TEXT[] NOT NULL,           -- 允许的起点 node_type 列表
    to_types        TEXT[] NOT NULL,           -- 允许的终点 node_type 列表
    schema          JSONB NOT NULL,
    description     TEXT,
    is_directed     BOOLEAN NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 初始化内置类型 (基本设计 §16 词汇)
INSERT INTO node_type_registry (node_type, schema, description) VALUES
    ('organization', '{"type":"object","required":["name"]}'::jsonb, '组织'),
    ('project', '{"type":"object","required":["name","org_id"]}'::jsonb, '项目'),
    ('repository', '{"type":"object","required":["name","project_id"]}'::jsonb, '仓库'),
    ('requirement', '{"type":"object","required":["title","body"]}'::jsonb, '需求'),
    ('issue', '{"type":"object","required":["title","repo_id"]}'::jsonb, '问题'),
    ('adr', '{"type":"object","required":["title","context","decision"]}'::jsonb, '架构决策记录'),
    ('commit', '{"type":"object","required":["sha","repo_id"]}'::jsonb, '提交'),
    ('symbol', '{"type":"object","required":["name","file_path"]}'::jsonb, '代码符号'),
    ('pull_request', '{"type":"object","required":["title","head","base","repo_id"]}'::jsonb, 'PR/MR'),
    ('review', '{"type":"object","required":["pr_id","reviewer_id"]}'::jsonb, '评审'),
    ('test', '{"type":"object","required":["name"]}'::jsonb, '测试'),
    ('deployment', '{"type":"object","required":["env","commit_id"]}'::jsonb, '部署'),
    ('release', '{"type":"object","required":["version","repo_id"]}'::jsonb, '发布'),
    ('human', '{"type":"object","required":["username"]}'::jsonb, '人类用户节点'),
    ('agent', '{"type":"object","required":["name","version"]}'::jsonb, 'Agent 定义'),
    ('agent_run', '{"type":"object","required":["agent_id","task"]}'::jsonb, 'Agent 执行实例'),
    ('incident', '{"type":"object","required":["title","severity"]}'::jsonb, '事故');
```

### 1.4.4 密钥 / Secrets

```sql
-- 005_secrets.sql
CREATE TABLE secrets (
    id              UUID PRIMARY KEY,
    owner_id        UUID NOT NULL REFERENCES nodes(id),
    secret_type     TEXT NOT NULL,             -- 'api_key','ssh_key','token','cert'
    encrypted_blob  BYTEA NOT NULL,            -- AES-256-GCM 密文
    dek_wrapped     BYTEA NOT NULL,            -- DEK, 由 KEK 加密
    iv              BYTEA NOT NULL,            -- GCM nonce (96-bit)
    aad             BYTEA,                     -- GCM additional authenticated data
    kek_id          TEXT NOT NULL,             -- 哪个 KEK 加密了此 DEK (支持 key rotation)
    version         INT NOT NULL DEFAULT 1,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    rotated_at      TIMESTAMPTZ,
    expires_at      TIMESTAMPTZ,
    CONSTRAINT chk_secrets_version_positive CHECK (version > 0)
);

-- 单独的 secrets schema, 默认对 platform_runtime 不可见
CREATE SCHEMA secrets;
ALTER TABLE secrets SET SCHEMA secrets;
-- platform_runtime 通过专门的 secrets service (代码侧) 访问, 不直接查表
REVOKE ALL ON secrets.secrets FROM platform_runtime;
GRANT SELECT, INSERT, UPDATE, DELETE ON secrets.secrets TO platform_owner;
-- 应用代码以 platform_owner 短期令牌调用 secrets service
```

### 1.4.5 协调结构 / Coordination

```sql
-- 006_coordination.sql

-- App 群组
CREATE TABLE app_groups (
    id              UUID PRIMARY KEY,
    name            TEXT NOT NULL UNIQUE,
    description     TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- App 群组成员
CREATE TABLE app_group_members (
    app_group_id    UUID NOT NULL REFERENCES app_groups(id) ON DELETE CASCADE,
    app_id          TEXT NOT NULL,             -- 应用标识 (非 UUID, 如 "ai-gateway", "context-engine")
    role            TEXT NOT NULL DEFAULT 'member',  -- 'owner','member','reader'
    joined_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (app_group_id, app_id)
);

-- 跨群组策略
CREATE TABLE cross_group_policies (
    id              UUID PRIMARY KEY,
    source_group    UUID NOT NULL REFERENCES app_groups(id),
    target_group    UUID NOT NULL REFERENCES app_groups(id),
    resource_type   TEXT NOT NULL,
    allowed_actions TEXT[] NOT NULL,
    expires_at      TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT chk_no_self_cross_group CHECK (source_group <> target_group)
);

-- Outbox 表 (跨边界可靠投递)
CREATE TABLE outbox (
    id              UUID PRIMARY KEY,
    event_type      TEXT NOT NULL,
    payload         JSONB NOT NULL,
    target_filter   JSONB,
    target_url      TEXT,                      -- 由 relay 填入
    status          TEXT NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending','in_flight','sent','dlq')),
    attempts        INT NOT NULL DEFAULT 0,
    last_error      TEXT,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    sent_at         TIMESTAMPTZ
);
CREATE INDEX idx_outbox_pending ON outbox (next_attempt_at)
    WHERE status = 'pending';

-- Saga 表
CREATE TABLE saga_instances (
    id            UUID PRIMARY KEY,
    saga_type     TEXT NOT NULL,
    state         TEXT NOT NULL DEFAULT 'pending'
                  CHECK (state IN ('pending','running','completed','compensating','failed')),
    current_step  TEXT,
    steps_total   INT,
    steps_done    INT NOT NULL DEFAULT 0,
    payload       JSONB NOT NULL,
    started_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_event_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at  TIMESTAMPTZ,
    error         TEXT
);

CREATE TABLE saga_step_log (
    saga_id    UUID NOT NULL REFERENCES saga_instances(id) ON DELETE CASCADE,
    step_name  TEXT NOT NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ended_at   TIMESTAMPTZ,
    status     TEXT CHECK (status IN ('success','failed','compensated')),
    result     JSONB,
    error      TEXT,
    PRIMARY KEY (saga_id, step_name, started_at)
);
```

### 1.4.6 视图与版本 / View Snapshots (CTX-REQ-002 精确重建支持)

```sql
-- 007_view_snapshots.sql
-- View 调用的快照记录, 用于按 seq 重放重建
CREATE TABLE view_invocations (
    id              UUID PRIMARY KEY,
    view_id         UUID NOT NULL REFERENCES views(id),
    actor_id        UUID NOT NULL,
    parameters      JSONB NOT NULL,
    event_seq_watermark BIGINT NOT NULL,       -- 调用时 events.seq 的当前值
    result_summary  JSONB,
    duration_ms     INT,
    invoked_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_view_invocations_view ON view_invocations (view_id, invoked_at);
```

## 1.5 索引策略 / Index Strategy

### 1.5.1 通用索引

```sql
-- 008_indices.sql

-- 节点
CREATE INDEX idx_nodes_type ON nodes (node_type) WHERE deleted_at IS NULL;
CREATE INDEX idx_nodes_props_gin ON nodes USING GIN (properties);
CREATE INDEX idx_nodes_created_at ON nodes (created_at);

-- 边: 双向遍历都需
CREATE INDEX idx_edges_from_type ON edges (from_node_id, edge_type);
CREATE INDEX idx_edges_to_type ON edges (to_node_id, edge_type);
CREATE INDEX idx_edges_type ON edges (edge_type);
-- 复合: "某类型边在某个时间窗" 用于审计
CREATE INDEX idx_edges_type_created ON edges (edge_type, created_at);

-- 事件
CREATE INDEX idx_events_node_time ON events (subject_node_id, occurred_at);
CREATE INDEX idx_events_type_time ON events (event_type, occurred_at);
CREATE INDEX idx_events_actor_time ON events (actor_id, occurred_at) WHERE actor_id IS NOT NULL;
-- seq 已隐式建索引 (PRIMARY KEY)

-- 权限
CREATE INDEX idx_perm_subject ON permissions (subject_id, resource_id) WHERE expires_at IS NULL OR expires_at > now();
CREATE INDEX idx_perm_resource ON permissions (resource_id, action) WHERE expires_at IS NULL OR expires_at > now();

-- 用户
CREATE INDEX idx_users_username ON users (username) WHERE NOT is_disabled;
```

### 1.5.2 索引选择原则

| 场景 | 索引选择 |
|---|---|
| 单点等值查询 | B-tree on column |
| 范围 / 排序 | B-tree on (column1, column2) 复合 |
| JSON 字段查询 | GIN on JSONB column |
| 部分索引（仅活跃行）| `WHERE deleted_at IS NULL` 或 `WHERE active` |
| 全文搜索 | GIN on `to_tsvector('english', properties->>'text')` |
| 高基数时间序 | BRIN on `created_at`（超大规模表）|

**[PROPOSAL]** 索引不是越多越好。每个索引降低写吞吐 5-10%。MVP 默认加上述必备索引，新增索引必须给出 EXPLAIN ANALYZE 证据。

## 1.6 行级安全 (RLS)

**[PROPOSAL]** 在多租户或群组隔离场景下，启用 PostgreSQL 原生 RLS。

### 1.6.1 节点 RLS

```sql
-- 009_rls.sql
ALTER TABLE nodes ENABLE ROW LEVEL SECURITY;
ALTER TABLE edges ENABLE ROW LEVEL SECURITY;
ALTER TABLE events ENABLE ROW LEVEL SECURITY;
ALTER TABLE policies ENABLE ROW LEVEL SECURITY;
ALTER TABLE views ENABLE ROW LEVEL SECURITY;

-- 单租户 MVP: 全部放行
-- 后续群组隔离扩展时, 创建基于 app_group 的 policy

-- 简化: 单租户时, owner + runtime 都全可见
CREATE POLICY nodes_all ON nodes
    FOR ALL TO platform_runtime USING (true) WITH CHECK (true);
CREATE POLICY edges_all ON edges
    FOR ALL TO platform_runtime USING (true) WITH CHECK (true);
-- events 由 AISEC-REQ-009(a) 已有 INSERT-only policy (见 1.3.2 第 5 步)
```

### 1.6.2 群组级 RLS（V1+ 多 App 群组场景）

```sql
-- 群组隔离: 节点 properties 必须带 app_group_id 字段 (类型层强制)
-- 详见 07 App 协调

CREATE POLICY nodes_by_group ON nodes
    FOR ALL TO platform_external_app
    USING (properties->>'app_group_id' = current_setting('app.current_group_id'))
    WITH CHECK (properties->>'app_group_id' = current_setting('app.current_group_id'));
```

## 1.7 查询模式 / Query Patterns

### 1.7.1 节点 CRUD

```sql
-- 创建
INSERT INTO nodes (id, node_type, properties)
VALUES ($1, $2, $3)
RETURNING id, created_at;

-- 读取 (含 deleted 过滤)
SELECT id, node_type, properties, created_at, updated_at
FROM nodes
WHERE id = $1 AND deleted_at IS NULL;

-- 更新 (逻辑删除)
UPDATE nodes
SET properties = properties || $2::jsonb,
    updated_at = now()
WHERE id = $1 AND deleted_at IS NULL
RETURNING updated_at;

-- 软删除
UPDATE nodes
SET deleted_at = now()
WHERE id = $1 AND deleted_at IS NULL;
```

### 1.7.2 边创建

```sql
INSERT INTO edges (id, from_node_id, to_node_id, edge_type, properties)
VALUES ($1, $2, $3, $4, $5)
ON CONFLICT (from_node_id, to_node_id, edge_type) DO NOTHING
RETURNING id;
-- 唯一约束: 防止重复边 (仅当业务允许)
-- 如需 (from, to, type) 唯一, 加 UNIQUE INDEX
```

### 1.7.3 图遍历（递归 CTE）

```sql
-- 下游遍历: 从起点向 to_node 方向 (有向图)
WITH RECURSIVE downstream(node_id, depth, path) AS (
    SELECT $1::uuid, 1, ARRAY[$1]::uuid[]
    UNION ALL
    SELECT e.to_node_id, d.depth + 1, d.path || e.to_node_id
    FROM downstream d
    JOIN edges e ON e.from_node_id = d.node_id
    WHERE d.depth < $2::int
      AND NOT (e.to_node_id = ANY(d.path))  -- 防环
      AND e.edge_type = ANY($3::text[])
)
SELECT n.id, n.node_type, n.properties, d.depth
FROM downstream d
JOIN nodes n ON n.id = d.node_id
WHERE n.deleted_at IS NULL;
```

**[PROPOSAL]** 深度限制默认 4，硬上限 8（防 DoS）。详细见 [02 图谱引擎](02-graph-engine.md)。

### 1.7.4 权限查询

```sql
-- 给定 (subject, action, resource), 返回最终 effect
-- 注意: deny 优先, 任意 deny 匹配即拒绝
WITH matched AS (
    SELECT effect
    FROM permissions
    WHERE subject_id = $1
      AND action = $2
      AND (
          (resource_id = $3 AND resource_type = $4)
          OR (resource_id IS NULL AND resource_type = $4)
          OR (resource_id IS NULL AND resource_type IS NULL)
      )
      AND (expires_at IS NULL OR expires_at > now())
)
SELECT
    EXISTS (SELECT 1 FROM matched WHERE effect = 'deny') AS denied,
    EXISTS (SELECT 1 FROM matched WHERE effect = 'allow') AS allowed;
-- 规则: deny 优先. 有 deny 即 false. 否则有 allow 即 true. 否则默认 false.
```

## 1.8 事务边界 / Transaction Boundaries

**[PROPOSAL]** 单个事务边界规则：

| 操作 | 事务边界 |
|---|---|
| 单 Node CRUD | 1 个事务 |
| Node + 关联 Edge | 1 个事务（保证原子性）|
| 跨 Node + Edge + Event | 1 个事务（基本操作）|
| 存储过程（见 07）| 整个 PL/pgSQL 函数体 1 事务 |
| Outbox 写入 | 与触发它的主事务合并（同一事务内）|

**禁止：** 长事务（> 1s 应当异步化）、跨多个 HTTP 请求的事务、嵌套显式事务（仅在存储过程内允许）。

## 1.9 迁移管理 / Migration Management

**[PROPOSAL]**

- 工具：`golang-migrate`（Go）/ `sqlx-migrate`（Rust）/ `node-pg-migrate`（Node）
- 文件命名：`NNNN_description.sql`（4 位序号 + 下划线 + 蛇形描述）
- 每个 migration 必须**可逆**：提供 `NNNN_description.down.sql`
- CI 中所有 migration 必须能从头重建数据库
- **不修改已合并的 migration**：新变更新建文件
- 大表变更：先 `BEGIN; ALTER TABLE ADD COLUMN ... DEFAULT ...; ALTER TABLE ALTER COLUMN ... DROP DEFAULT; COMMIT;` 避免长时间锁表

```bash
# migration 工作流
migrate create -ext sql -dir migrations add_user_status
# 编辑 0099_add_user_status.up.sql 和 .down.sql
migrate up    # 应用
migrate down 1  # 回滚一条
```

## 1.10 备份与恢复 / Backup & Recovery

详见基本设计 [§8.2 备份/恢复](../basic-design/08-operations-design.md#82-备份-恢复-backup-recoverybkp-req)。

数据库角色（PostgreSQL 自己的角色，不是平台业务角色）的备份：使用 `pg_dumpall --roles-only` 单独备份 role 定义。

## 1.11 监控指标 / Monitoring Metrics

[PROPOSAL] 来自 DB schema 的关键指标：

| 指标 | 类型 | 来源 |
|---|---|---|
| `db.connection_pool.active` | gauge | pgxpool 统计 |
| `db.connection_pool.idle` | gauge | 同上 |
| `db.query.duration` | histogram | query 中间件 |
| `db.slow_queries` | counter | 慢查询日志计数（> 1s）|
| `db.deadlocks` | counter | PostgreSQL `pg_stat_database` |
| `db.replication_lag` | gauge | `pg_stat_replication`（V1 Cloud）|
| `db.size` | gauge | `pg_database_size()` |

详见 [10 可观测性](10-observability.md)。

## 1.12 错误处理 / Error Handling

DB 层错误标准化：

| SQLSTATE | 含义 | 平台错误码 |
|---|---|---|
| `23000` | 完整性约束违反 | `db_integrity_violation` |
| `23001` | 唯一约束 | `db_unique_violation` |
| `23503` | 外键约束 | `db_foreign_key_violation` |
| `23514` | 检查约束 | `db_check_violation` |
| `40001` | 序列化失败（需重试）| `db_serialization_failure` |
| `40P01` | 死锁（需重试）| `db_deadlock` |
| `08000`-`08999` | 连接类 | `db_unavailable` |
| 其他 | 未知 | `db_error` |

**重试策略：** 序列化失败与死锁可重试 3 次（指数退避 10ms/50ms/200ms），其他不重试。详见 [11 错误处理](11-error-handling.md)。

---

**导航 / Navigation:**
[← 00. 总体概述](00-overview.md) · [README](README.md) · [02. 图谱引擎 →](02-graph-engine.md)
