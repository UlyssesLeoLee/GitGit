# 12. App Registry & Plugin Loader / App 注册表与插件加载器

> **[PROPOSAL] IPA 共通框架 2013 章节定位：** P4 软件方式设计 / T-P4-A1-1 设计软件构成 (App 一级化扩展) / T-P4-A1-3 设计数据存储 (App Registry + 中心事件表) / T-P4-A1-5 设计错误处理方式 (App 加载失败回滚)
>
> **上游文档：** [基本设计 §13. App 集群与可热插拔架构](../basic-design/13-app-cluster-and-plugins.md)、[§11. API 设计](../basic-design/11-api-design.md)、[§7. App 协调](07-app-coordination.md)
>
> **设计主张 / Design Claim:** App Registry 复用图谱引擎（§2）作为存储；Plugin Loader 是独立服务进程（与主 API 进程分离）；事件分发复用 §7.6 Outbox + §7.7 Saga 引擎。本章定义完整 DDL、Loader 协议、事件分发实现细节、沙箱权限边界。

---

## 12.0 目的 / Purpose

把"App 集群 + 可热插拔 + 中心事件总线"从基本设计 §13 的概念落到可实现的详细规格：

- App / AppInstance / AppDeployment 在 `nodes` 表的扩展 DDL
- 中心事件表的 DDL（含 schema registry、订阅关系）
- Plugin Loader 进程的注册协议、升级协议、热加载协议
- 沙箱执行环境（DB role 隔离、PL/pgSQL 命名空间、HTTP 入口命名空间）
- 与 §7.6 Outbox Relay / §7.7 Saga 引擎的协作点

---

## 12.1 App Registry DDL

### 12.1.1 类型注册表扩展

```sql
-- 复用 §1.4.3 type_registry（已有），仅 INSERT 三行
INSERT INTO type_registry (type, kind, schema, version, description) VALUES
  ('app',           'node',     'app_v1.json',           1, 'App — 平台应用/插件一级对象'),
  ('app_instance',  'node',     'app_instance_v1.json',  1, 'AppInstance — App 的运行实例'),
  ('app_deployment','event',    'app_deployment_v1.json',1, 'AppDeployment — App 状态转换事件');
```

### 12.1.2 App / AppInstance DDL（基于 nodes 表扩展）

```sql
-- App 节点 — properties JSONB 包含 manifest 完整内容
-- 已存在 nodes 表，本节定义 App 节点特定的 JSON Schema 约束

-- App properties JSONB schema
CREATE OR REPLACE FUNCTION app_properties_schema() RETURNS jsonb
LANGUAGE sql IMMUTABLE AS $$
  $${
    "type": "object",
    "required": ["manifest_yaml", "manifest_hash", "state", "current_version"],
    "properties": {
      "manifest_yaml":      { "type": "string", "minLength": 1 },
      "manifest_hash":      { "type": "string", "pattern": "^sha256:[a-f0-9]{64}$" },
      "state":              { "enum": ["draft", "installed", "upgrading", "healthy", "degraded", "disabled", "rolled_back"] },
      "current_version":    { "type": "string", "pattern": "^[0-9]+\\.[0-9]+\\.[0-9]+(-[a-zA-Z0-9.]+)?$" },
      "available_version":  { "type": "string" },
      "min_instances":      { "type": "integer", "minimum": 1, "default": 1 },
      "max_instances":      { "type": "integer", "minimum": 1, "default": 5 },
      "instance_count":     { "type": "integer", "minimum": 0 },
      "installed_at":       { "type": "string", "format": "date-time" },
      "installed_by":       { "type": "string" },
      "last_health_check":  { "type": "string", "format": "date-time" },
      "last_deployment_id": { "type": "string" }
    }
  }$$::jsonb
$$;

-- 触发器：插入/更新 App 节点前校验 properties
CREATE OR REPLACE FUNCTION enforce_app_node_properties() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
  v_schema jsonb := app_properties_schema();
BEGIN
  IF NEW.type = 'app' THEN
    PERFORM validate_json_schema(v_schema, NEW.properties);
    -- 额外校验：manifest_hash 必须等于 manifest_yaml 的 sha256
    IF 'sha256:' || encode(digest(NEW.properties->>'manifest_yaml', 'sha256'), 'hex')
       != NEW.properties->>'manifest_hash' THEN
      RAISE EXCEPTION 'manifest_hash does not match manifest_yaml content';
    END IF;
  END IF;
  RETURN NEW;
END;
$$;

CREATE TRIGGER trg_app_node_properties
BEFORE INSERT OR UPDATE ON nodes
FOR EACH ROW WHEN (NEW.type = 'app')
EXECUTE FUNCTION enforce_app_node_properties();
```

### 12.1.3 AppInstance DDL

```sql
-- AppInstance 节点 — properties JSONB
CREATE OR REPLACE FUNCTION app_instance_properties_schema() RETURNS jsonb
LANGUAGE sql IMMUTABLE AS $$
  $${
    "type": "object",
    "required": ["app_id", "app_version", "instance_uuid", "state", "started_at", "hostname", "pid"],
    "properties": {
      "app_id":        { "type": "string", "pattern": "^[a-z][a-z0-9-]{2,62}$" },
      "app_version":   { "type": "string" },
      "instance_uuid": { "type": "string", "format": "uuid" },
      "state":         { "enum": ["starting", "healthy", "degraded", "draining", "stopped", "failed"] },
      "started_at":    { "type": "string", "format": "date-time" },
      "hostname":      { "type": "string" },
      "pid":           { "type": "integer" },
      "last_heartbeat":{ "type": "string", "format": "date-time" },
      "last_error":    { "type": "object" },
      "is_leader":     { "type": "boolean", "default": false },
      "is_canary":     { "type": "boolean", "default": false },
      "metrics":       { "type": "object" }
    }
  }$$::jsonb
$$;

-- AppInstance 唯一性约束：同一 app_id + instance_uuid 不能有两条
CREATE UNIQUE INDEX IF NOT EXISTS uq_app_instance
  ON nodes ((properties->>'app_id'), (properties->>'instance_uuid'))
  WHERE type = 'app_instance';
```

### 12.1.4 中心事件 schema 注册表

```sql
-- event_schema_registry — 事件 schema 版本化（避免硬编码）
CREATE TABLE event_schema_registry (
  schema_ref         text PRIMARY KEY,    -- 形如 "github-pr-reviewer/pr.opened@v1"
  app_id             text NOT NULL,
  event_type         text NOT NULL,
  version            int  NOT NULL,
  json_schema        jsonb NOT NULL,     -- JSON Schema 文档
  registered_at      timestamptz NOT NULL DEFAULT now(),
  registered_by      text NOT NULL,
  UNIQUE (app_id, event_type, version)
);

CREATE INDEX idx_event_schema_app_event ON event_schema_registry(app_id, event_type);
```

### 12.1.5 事件订阅关系（边表扩展）

```sql
-- 复用 edges 表，event_subscription 边类型
-- 当订阅关系变化时，更新 App 节点 properties.subscriptions_cache（缓存，避免每次 join）

-- 边类型校验约束
ALTER TABLE edges ADD CONSTRAINT chk_app_event_edge
  CHECK (
    edge_type NOT IN ('app_publishes_event', 'app_consumes_event')
    OR source_type = 'app'
  );
```

### 12.1.6 中心事件表（pub/sub 持久层）

```sql
-- event_stream — 中心事件持久化（Outbox 模式，事务内写入）
CREATE TABLE event_stream (
  event_id           uuid PRIMARY KEY,
  event_type         text NOT NULL,
  schema_ref         text NOT NULL REFERENCES event_schema_registry(schema_ref),
  producer_app_id    text NOT NULL,
  producer_instance  text,
  producer_version   text NOT NULL,
  tenant_id          text,                 -- V1+ 租户隔离
  payload            jsonb NOT NULL,
  metadata           jsonb NOT NULL DEFAULT '{}'::jsonb,
  occurred_at        timestamptz NOT NULL DEFAULT now(),
  -- 分发状态
  processed_at       timestamptz,
  delivered_at       timestamptz,
  delivery_attempts  int  NOT NULL DEFAULT 0,
  last_error         text,
  -- 因果/相关链
  causation_id       uuid REFERENCES event_stream(event_id),
  correlation_id     uuid,
  -- 索引
  created_at         timestamptz NOT NULL DEFAULT now()
);

-- 按 producer + 顺序读
CREATE INDEX idx_event_stream_producer_time
  ON event_stream (producer_app_id, occurred_at)
  WHERE processed_at IS NULL;

-- 按 event_type 读
CREATE INDEX idx_event_stream_type_time
  ON event_stream (event_type, occurred_at DESC);

-- DLQ
CREATE INDEX idx_event_stream_dlq
  ON event_stream (delivery_attempts, last_error)
  WHERE delivery_attempts >= 5 AND processed_at IS NULL;
```

### 12.1.7 心跳表

```sql
CREATE TABLE app_heartbeats (
  app_id             text NOT NULL,
  instance_id        text NOT NULL,           -- 形如 "github-pr-reviewer:7f3a-..."
  ts                 timestamptz NOT NULL DEFAULT now(),
  status             text NOT NULL,           -- 'healthy' / 'degraded' / 'draining'
  metrics            jsonb NOT NULL DEFAULT '{}'::jsonb,
  PRIMARY KEY (app_id, instance_id)
);

-- 自动清理 7 天前的心跳
CREATE INDEX idx_app_heartbeats_ts ON app_heartbeats(ts);
```

### 12.1.8 App 状态机日志（不可变）

```sql
CREATE TABLE app_state_transitions (
  id                 bigserial PRIMARY KEY,
  app_id             text NOT NULL,
  from_state         text,
  to_state           text NOT NULL,
  actor              text NOT NULL,           -- 'human:uli' / 'system:plugin-loader' / 'auto:health-check'
  reason             text,
  manifest_hash      text,
  manifest_version   text,
  occurred_at        timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_app_state_transitions_app ON app_state_transitions(app_id, occurred_at DESC);
```

### 12.1.9 RLS 策略（沙箱权限）

```sql
-- App DB role 隔离（AISEC-REQ-013，沙箱权限边界）
-- 创建时由 Plugin Loader 调用 §1.3.2 中的 ensure_app_role 函数

-- 通用模板：每个 App 一个 schema + 一个 role
-- 例子：github-pr-reviewer
CREATE SCHEMA IF NOT EXISTS app_github_pr_reviewer;
CREATE ROLE app_github_pr_reviewer LOGIN PASSWORD :'generated_pwd';

-- 默认禁止跨 schema 访问
REVOKE ALL ON SCHEMA public FROM app_github_pr_reviewer;

-- 仅授予 manifest 中声明的权限
GRANT USAGE ON SCHEMA app_github_pr_reviewer TO app_github_pr_reviewer;
GRANT SELECT, INSERT, UPDATE ON ALL TABLES IN SCHEMA app_github_pr_reviewer TO app_github_pr_reviewer;

-- 必须显式：禁止 INSERT/UPDATE/DELETE events 表（任何 App 都不能直接改事件流）
-- 走 trigger_event_publish() 存储过程，存储过程 SECURITY DEFINER 写入 event_stream
```

---

## 12.2 Plugin Loader 进程

### 12.2.1 进程模型

**Plugin Loader** 是独立的后台进程（与主 API 进程分离），负责：

1. 监听 App 节点状态变化（`LISTEN app_state_changed`）
2. 调用 App Manager 执行 install / upgrade / rollback / disable / uninstall
3. 维护 instance 池（按 min/max 启动 / 停止 instance）
4. 写入心跳到 `app_heartbeats`
5. 写 `app_state_transitions`

```
┌────────────────────────────────────────────────────────────────┐
│                       Plugin Loader 进程                        │
├────────────────────────────────────────────────────────────────┤
│  状态机驱动:                                                    │
│    监听 nodes WHERE type='app' AND state IN                   │
│    ('installed', 'upgrading', 'disabled', ...)                  │
│                                                                 │
│  App Manager 组件：                                             │
│    - ManifestValidator   校验 app.yaml schema                  │
│    - SandboxPreparer     创建 schema / role / 权限             │
│    - ProcedureInstaller  加载 PL/pgSQL 存储过程                 │
│    - InstanceSupervisor  启动 / 停止 instance 进程             │
│    - SubscriptionManager 维护中心事件订阅                       │
│    - HealthMonitor       检查心跳、超时判定                    │
│                                                                 │
│  Event Bus 客户端：                                              │
│    - publish(event)  → 写 event_stream（事务内）              │
│    - subscribe(event_type, handler)  ← Relay 派发              │
└────────────────────────────────────────────────────────────────┘
```

### 12.2.2 安装协议

```rust
// 伪代码 — Plugin Loader 主循环
async fn install_app(manifest_yaml: &str, actor: &str) -> Result<AppId> {
  // 1. 校验 Manifest
  let manifest = Manifest::parse(manifest_yaml)?;
  manifest.validate()?;  // 见 §13.2.2 必填字段

  // 2. 检查依赖是否就绪
  for dep in &manifest.dependencies.apps {
    let dep_app = AppRegistry::get(&dep.id)?;
    if !dep_app.is_state("healthy") {
      return Err("dependency not healthy");
    }
  }

  // 3. 事务内：写 App 节点 + state_transition
  let mut tx = pg.begin().await?;
  let app_id = manifest.metadata.id.clone();

  // 检查 ID 唯一性
  if AppRegistry::exists(&app_id)? {
    return Err("app id already exists");
  }

  // 写 App 节点
  Nodes::insert(tx, Node {
    id: app_id.clone(),
    type: "app".into(),
    properties: json!({
      "manifest_yaml": manifest_yaml,
      "manifest_hash": sha256(manifest_yaml),
      "state": "installed",
      "current_version": manifest.metadata.version,
      "installed_at": now(),
      "installed_by": actor,
      ...
    }),
  })?;

  // 写状态转换
  AppStateTransitions::insert(tx, StateTransition {
    app_id: app_id.clone(),
    from_state: None,
    to_state: "installed",
    actor,
    manifest_hash: ...,
    manifest_version: manifest.metadata.version,
  })?;

  tx.commit().await?;

  // 4. 沙箱准备（独立事务，失败回滚上一步）
  SandboxPreparer::prepare(&manifest).await?;

  // 5. 启动首个 instance
  let instance = InstanceSupervisor::start(&manifest, /*is_canary=*/ false).await?;

  // 6. 写 Instance 节点
  Nodes::insert(pg, Node {
    id: instance.node_id,
    type: "app_instance".into(),
    properties: instance.properties,
  })?;

  // 7. 注册事件订阅
  SubscriptionManager::sync(&manifest).await?;

  // 8. 发布 app.installed 事件
  EventBus::publish("app.installed", &app_id, json!({
    "app_id": app_id,
    "version": manifest.metadata.version,
    "actor": actor,
  })).await?;

  Ok(app_id)
}
```

### 12.2.3 升级协议（rolling 策略）

```rust
async fn upgrade_app(app_id: &AppId, new_manifest: Manifest, actor: &str) -> Result<()> {
  let app = AppRegistry::get(app_id)?;
  let old_version = app.current_version();
  let new_version = new_manifest.metadata.version;

  // 1. 进入 upgrading 状态
  set_state(app_id, "upgrading").await?;

  // 2. 加载新版 Manifest 校验
  new_manifest.validate()?;

  // 3. 准备新版沙箱（不卸载旧版）
  SandboxPreparer::prepare_version(&new_manifest, /*shadow=*/ true).await?;

  // 4. 启动新 instance（与旧 instance 并存）
  let new_instance = InstanceSupervisor::start(&new_manifest, /*is_canary=*/ false).await?;
  write_instance_node(&new_instance).await?;

  // 5. 等待健康检查通过（≤ 30s）
  if !new_instance.wait_healthy(Duration::from_secs(30)).await? {
    // 失败回滚：停新 instance，状态 rolled_back
    InstanceSupervisor::stop(&new_instance).await?;
    set_state(app_id, "rolled_back").await?;
    return Err("new instance failed health check");
  }

  // 6. 平滑切换流量（按 strategy 决定是瞬时切还是分批切）
  match new_manifest.spec.upgrade.strategy {
    "rolling" => {
      // 20% → 50% → 100% 切流量，每步 5s
      TrafficSplitter::shift(&app_id, 0.20).await?;
      sleep(5).await;
      if !health_ok(&app_id).await {
        return rollback();
      }
      TrafficSplitter::shift(&app_id, 0.50).await?;
      sleep(5).await;
      if !health_ok(&app_id).await {
        return rollback();
      }
      TrafficSplitter::shift(&app_id, 1.00).await?;
    },
    "blue-green" => {
      TrafficSplitter::shift(&app_id, 1.00).await?;  // 瞬时
    },
    "canary" => {
      let percent = new_manifest.spec.upgrade.canary_traffic_percent.unwrap_or(10);
      TrafficSplitter::shift(&app_id, percent as f64 / 100.0).await?;
      sleep(Duration::from_secs(new_manifest.spec.upgrade.canary_observation_seconds.unwrap_or(300))).await;
      if !health_ok(&app_id).await {
        return rollback();
      }
      TrafficSplitter::shift(&app_id, 1.00).await?;
    },
    _ => return Err("unknown strategy"),
  }

  // 7. 停止旧 instance
  for old_instance in InstanceSupervisor::list(&app_id, /*version=*/
  old_version).await? {
    InstanceSupervisor::graceful_stop(&old_instance, Duration::from_secs(30)).await?;
  }

  // 8. 更新 App 节点 current_version
  set_version(app_id, new_version).await?;
  set_state(app_id, "healthy").await?;

  // 9. 清理旧版沙箱（仅当持久数据已迁移）
  if !new_manifest.spec.persistence.keep_old_data {
    SandboxPreparer::cleanup_version(&app_id, old_version).await?;
  }

  // 10. 发布 app.upgraded 事件
  EventBus::publish("app.upgraded", &app_id, json!({
    "app_id": app_id,
    "old_version": old_version,
    "new_version": new_version,
    "actor": actor,
  })).await?;

  Ok(())
}
```

### 12.2.4 心跳与健康检查

```rust
// InstanceSupervisor 启动时立即注册心跳任务
async fn heartbeat_task(app_id: &AppId, instance_id: &str) {
  loop {
    interval(5s).tick().await;
    let metrics = collect_instance_metrics().await;  // CPU/内存/队列深度
    sqlx::query!(
      "INSERT INTO app_heartbeats (app_id, instance_id, status, metrics)
       VALUES ($1, $2, $3, $4)
       ON CONFLICT (app_id, instance_id) DO UPDATE
         SET ts = now(), status = EXCLUDED.status, metrics = EXCLUDED.metrics",
      app_id, instance_id, instance_status(), metrics
    ).execute(&pool).await.ok();
  }
}

// 平台健康监控（独立 goroutine / task）
async fn health_monitor() {
  loop {
    interval(10s).tick().await;
    // 找出心跳超时的 instance
    let stale = sqlx::query_as!(StaleInstance,
      "SELECT app_id, instance_id, ts FROM app_heartbeats
       WHERE ts < now() - interval '30 seconds'"
    ).fetch_all(&pool).await.unwrap_or_default();

    for s in stale {
      // 标记 instance failed
      sqlx::query!(
        "UPDATE nodes SET properties = jsonb_set(properties, '{state}', '\"failed\"')
         WHERE type = 'app_instance' AND properties->>'instance_uuid' = $1",
        s.instance_id
      ).execute(&pool).await.ok();

      // 发布 instance_lost 事件
      EventBus::publish("instance.lost", &s.app_id, json!({
        "instance_id": s.instance_id,
        "last_heartbeat": s.ts,
      })).await.ok();
    }

    // 检查 App 整体：若 instance_count < min_instances → degraded
    sqlx::query!(
      "UPDATE nodes n SET properties = jsonb_set(properties, '{state}', '\"degraded\"')
       WHERE type = 'app' AND state IN ('healthy', 'installed')
         AND (SELECT count(*) FROM nodes WHERE type = 'app_instance'
              AND properties->>'app_id' = n.id
              AND (properties->>'state')::text IN ('healthy', 'degraded'))
             < (n.properties->>'min_instances')::int"
    ).execute(&pool).await.ok();
  }
}
```

---

## 12.3 中心事件总线实现

### 12.3.1 Publish 路径

```sql
-- 触发器函数：所有 App 写入事件必须通过此函数
-- SECURITY DEFINER — 允许 App role 调用，但内部强校验

CREATE OR REPLACE FUNCTION trigger_event_publish(
  p_event_type    text,
  p_payload       jsonb,
  p_causation_id  uuid DEFAULT NULL,
  p_correlation_id uuid DEFAULT NULL
) RETURNS uuid
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
  v_app_id          text := current_setting('app.current_app_id', true);
  v_app_version     text := current_setting('app.current_app_version', true);
  v_instance_id     text := current_setting('app.current_instance_id', true);
  v_tenant_id       text := current_setting('app.current_tenant_id', true);
  v_schema_ref      text;
  v_event_id        uuid;
BEGIN
  -- 强校验：必须设置 app context
  IF v_app_id IS NULL THEN
    RAISE EXCEPTION 'app.current_app_id GUC must be set';
  END IF;

  -- 校验 schema_ref 必须已注册
  v_schema_ref := v_app_id || '/' || p_event_type || '@v1';
  IF NOT EXISTS (SELECT 1 FROM event_schema_registry WHERE schema_ref = v_schema_ref) THEN
    RAISE EXCEPTION 'event schema not registered: %', v_schema_ref;
  END IF;

  v_event_id := gen_random_uuid_v7();

  INSERT INTO event_stream (
    event_id, event_type, schema_ref,
    producer_app_id, producer_instance, producer_version,
    tenant_id, payload, metadata,
    causation_id, correlation_id
  ) VALUES (
    v_event_id, p_event_type, v_schema_ref,
    v_app_id, v_instance_id, v_app_version,
    v_tenant_id, p_payload,
    jsonb_build_object('enqueued_at', now()::text),
    p_causation_id, p_correlation_id
  );

  -- NOTIFY 触发 Relay 立即派发
  PERFORM pg_notify('event_stream_new', v_event_id::text);

  RETURN v_event_id;
END;
$$;

-- 关键：只授予 EXECUTE，不授予 INSERT/UPDATE/DELETE event_stream
REVOKE ALL ON event_stream FROM PUBLIC;
GRANT EXECUTE ON FUNCTION trigger_event_publish TO PUBLIC;
```

### 12.3.2 Subscribe 路径（App 注册订阅）

```sql
-- 关系存在 edges 表中：app_consumes_event
-- 当 App 启动时，订阅关系由 SubscriptionManager 同步：

-- 1. App 注册订阅（启动时）：
INSERT INTO edges (source_id, source_type, edge_type, target_id, target_type, properties)
VALUES ($1, 'app', 'app_consumes_event', $2, 'event_type', jsonb_build_object('handler', $3, 'filter', $4))
ON CONFLICT DO NOTHING;

-- 2. Event Relay（复用 §7.6.2）查询订阅者：
SELECT e.source_id AS app_id, e.properties->>'handler' AS handler, e.properties->>'filter' AS filter
FROM edges e
WHERE e.edge_type = 'app_consumes_event'
  AND e.target_id = $event_type
  AND EXISTS (
    SELECT 1 FROM nodes n
    WHERE n.id = e.source_id
      AND n.type = 'app'
      AND n.properties->>'state' IN ('healthy', 'degraded')
  );
```

### 12.3.3 死信处理

```sql
-- Event Relay 在 delivery_attempts >= 5 时将事件标记为 DLQ
UPDATE event_stream
SET processed_at = now(),
    last_error = 'max retries exceeded'
WHERE event_id = $1;

-- 触发 dlq.alert 事件（不在 event_stream 内，避免循环）
-- 由 Event Relay 通过 admin audit 通道通知
INSERT INTO admin_audit (action, target, severity, details)
VALUES ('event_dlq', $event_type, 'warning', $details);
```

---

## 12.4 沙箱执行环境

### 12.4.1 DB 角色创建流程

```rust
async fn prepare_app_role(manifest: &Manifest) -> Result<DbRole> {
  let role_name = format!("app_{}", manifest.metadata.id);
  let pwd = generate_secure_password(32);

  pg.execute(&format!(
    "CREATE ROLE {} LOGIN PASSWORD '{}'",
    role_name, pwd
  )).await?;

  // 默认隔离
  pg.execute(&format!("REVOKE ALL ON SCHEMA public FROM {}", role_name)).await?;
  pg.execute(&format!("REVOKE ALL ON DATABASE {} FROM {}", db_name, role_name)).await?;

  // 仅授予 manifest 声明的权限
  for grant in &manifest.spec.permissions.required_grants {
    pg.execute(&format!("GRANT {} TO {}", grant, role_name)).await?;
  }

  // 显式禁止
  for forbidden in &manifest.spec.permissions.forbidden_grants {
    pg.execute(&format!("REVOKE {} FROM {}",
      forbidden.replace("GRANT ", ""), role_name)).await?;
  }

  // 关键：events 表永远禁止直接写入
  pg.execute(&format!(
    "REVOKE INSERT, UPDATE, DELETE, TRUNCATE ON event_stream FROM {}", role_name
  )).await?;

  Ok(DbRole { name: role_name, password: pwd })
}
```

### 12.4.2 启动 Instance 时设置 GUC

```rust
async fn start_instance(manifest: &Manifest, is_canary: bool) -> Result<Instance> {
  let pg_conn = pg.connect_with_role(&manifest.spec.permissions.db_role).await?;

  // 设置 App 上下文（让 trigger_event_publish 能识别 producer）
  pg.execute(&format!(
    "SET app.current_app_id = '{}'", manifest.metadata.id
  )).await?;
  pg.execute(&format!(
    "SET app.current_app_version = '{}'", manifest.metadata.version
  )).await?;
  pg.execute(&format!(
    "SET app.current_instance_id = '{}'", instance_uuid
  )).await?;

  // ... 启动进程
}
```

### 12.4.3 存储过程命名空间

- 每个 App 拥有独立 schema：`app_<app_id>`
- 存储过程安装到该 schema：`CREATE OR REPLACE FUNCTION app_<app_id>.ensure_review_state(...) ...`
- 跨 schema 调用需 `SECURITY INVOKER` 显式校验
- 存储过程版本管理：见 §7.4.3（详细）

### 12.4.4 HTTP 入口命名空间

- App 暴露的 HTTP 端点路径前缀 `/api/apps/<app_id>/...`
- 路由表在启动时由 Plugin Loader 注册到主 API 进程的内存中
- 主 API 进程收到 `/api/apps/<app_id>/...` 请求 → 查 instance 池 → 路由到 healthy instance

```rust
// 主 API 进程的路由扩展（详细设计 §8.3 补充）
async fn handle_app_route(app_id: &str, path: &str) -> Response {
  // 1. 查 instance 池
  let instances = InstanceSupervisor::list_healthy(app_id).await?;
  if instances.is_empty() {
    return Response::service_unavailable();
  }

  // 2. 按 TrafficSplitter 选 instance
  let target = TrafficSplitter::pick(app_id, &instances).await?;

  // 3. 转发（进程内 call 或 HTTP 转发）
  proxy_to_instance(target, path).await
}
```

---

## 12.5 错误处理与回滚

### 12.5.1 加载失败回滚

| 失败点 | 自动回滚动作 |
|---|---|
| Manifest 校验失败 | 无副作用，返回 400 |
| 依赖未就绪 | 无副作用，返回 409 |
| Schema 创建失败 | 删除已创建的 schema/role，返回 500 |
| Instance 启动失败 | 删除已创建资源，回滚 App 节点到不存在 |
| 健康检查超时 | 删除 instance，状态 rolled_back |
| 流量切换中失败 | 全部切回旧 instance，新 instance 停机 |
| 旧 instance 停止失败 | 强制 SIGKILL（10s 后），记录告警 |

### 12.5.2 升级失败回滚

升级过程中任何步骤失败 → 全部回滚到 `previous_version`：

1. 停止所有新 version instance
2. 清理新 version 沙箱
3. 旧 version instance 仍保持 healthy（rolling 期间 2 版本共存）
4. App 节点 current_version 保持原值
5. 写 `app_state_transitions` to_state=rolled_back
6. 发布 `app.rolled_back` 事件
7. 通知 Admin UI（写 admin_audit）

---

## 12.6 与 §7.6 Outbox Relay / §7.7 Saga 的协作

**§7.6 Outbox 表** = `event_stream`（重命名，字段基本一致）

**§7.6.2 Relay 进程** = Event Relay（同一个进程）

**§7.7 Saga 引擎** = 不变，但 Saga 现在可以跨 App 编排（步骤可以是 App handler）

**新增**：
- `event_schema_registry` 表（schema 强制）
- `app_heartbeats` 表
- `app_state_transitions` 表
- `edges.app_publishes_event` / `edges.app_consumes_event` 边类型

---

## 12.7 关键性能预算

| 指标 | 目标 | 测量方法 |
|---|---|---|
| App 加载时间（无依赖）| ≤ 5s | OTel span |
| App 升级 rolling 单步切换 | ≤ 5s × 3 步 = 15s | OTel span |
| 心跳上报间隔 | 5s ± 0.5s | OTel metrics |
| 中心事件端到端（P50）| ≤ 500ms | OTel span: enqueue → delivered |
| 中心事件端到端（P99）| ≤ 5s | OTel span |
| 事件派发并发 | 100 events/s (单 Relay) | 基准测试 |
| DLQ 告警延迟 | ≤ 30s | OTel metrics |

---

## 12.8 关键 REQ-ID 详细映射

| APP-REQ | 实现位置 |
|---|---|
| APP-REQ-001 (App 一级化) | §12.1.1-1.3 + §12.1.2 |
| APP-REQ-002 (Manifest schema 校验) | §12.1.2 + §12.2.2 step 1 |
| APP-REQ-003 (AppInstance 唯一性) | §12.1.3 unique index |
| APP-REQ-004 (中心事件复用 PG) | §12.1.6 + §12.3 |
| APP-REQ-005 (envelope schema) | §13.4.1 + §12.3.1 trigger_event_publish |
| APP-REQ-006 (一致性 + 至少一次) | §13.4.3 + §12.6 |
| APP-REQ-007 (心跳 ≤ 30s) | §12.2.4 |
| APP-REQ-008 (rolling/blue-green/canary) | §12.2.3 |
| APP-REQ-009 (人类审批) | 继承 §4.9 + §13.6 |
| APP-REQ-010 (错误率自动回滚) | §12.5.2 + §13.7.4 |
| APP-REQ-011 (优雅停机) | §12.4 + §13.6.4 |

---

**导航 / Navigation:**
[← 11. 错误处理](11-error-handling.md) · [README](README.md) · [13. Admin API & Ops UI →](13-admin-api-and-ops-ui.md)
