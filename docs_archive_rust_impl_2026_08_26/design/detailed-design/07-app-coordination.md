# 07. App 协调 / App Coordination

## 7.1 目标 / Purpose

实现 [基本设计 §12 App 群组信息互通设计](../basic-design/12-app-group-intercommunication.md) 的 4 种协调模式：**存储过程**（同步 / 事务性）、**LISTEN/NOTIFY**（快速通知）、**Outbox + Relay**（跨边界可靠）、**Saga**（长流程）。本章定义它们的实现细节、接口契约、调用流程、错误恢复。

## 7.2 模块结构

```
internal/coordination/
├── procs/
│   ├── registry.go           # 存储过程注册表
│   ├── caller.go             # 应用代码调用入口
│   ├── rls.go                # RLS 上下文管理
│   ├── policy.go             # 存储过程 Policy 评估
│   └── version.go            # 存储过程版本治理
├── notify/
│   ├── publisher.go          # NOTIFY 发送
│   ├── subscriber.go         # NOTIFY 接收
│   └── channel.go            # channel 命名空间
├── outbox/
│   ├── writer.go             # 写 outbox
│   ├── relay.go              # relay 进程
│   ├── backoff.go            # 指数退避
│   ├── dlq.go                # 死信处理
│   └── targets.go            # 目标解析 (webhook/SMTP/IM)
├── saga/
│   ├── engine.go             # Saga 引擎
│   ├── step.go               # 步骤定义
│   ├── compensation.go       # 补偿
│   └── persist.go            # 状态持久化
├── appgroup/
│   ├── group.go              # App Group 实体
│   ├── member.go             # 成员管理
│   └── crossgroup.go         # 跨群组策略
├── observability/
│   ├── metrics.go            # 协调指标
│   └── views.sql             # 运维视图
├── errors.go
└── coordination_test.go
```

## 7.3 App 群组模型

```go
// [IMPL] internal/coordination/appgroup/group.go

type AppGroup struct {
    ID          uuid.UUID
    Name        string
    Description string
    Members     []AppMember
    CreatedAt   time.Time
}

type AppMember struct {
    AppID     string             // 'ai-gateway','context-engine','pr-bot'
    Role      string             // 'owner','member','reader'
    APIKey    string             // 不入 DB, 运行时注入
    JoinedAt  time.Time
}

// App 调用时的"身份" 包含 app_group_id + app_id
// Platform 通过这个组合决定 RLS 上下文与 Policy 评估
type AppContext struct {
    AppGroupID  uuid.UUID
    AppID       string
    CallerType  string  // 'internal' (Platform 子系统) | 'external'
}
```

## 7.4 存储过程（PL/pgSQL）调用

### 7.4.1 存储过程注册表

```go
// [IMPL] internal/coordination/procs/registry.go

type ProcSpec struct {
    Name         string                 // 'coord_merge_pr'
    Args         []ProcArg
    Returns      string                 // 'jsonb' (统一)
    Description  string
    CoordType    string                 // 'C' (coordination) | 'Q' (query) | 'T' (transform) | 'S' (sync)
    RequiredPerm string                 // 必需 Policy action
    TimeoutMs    int
    Version      int                    // 用于版本治理
}

type ProcArg struct {
    Name     string
    Type     string             // 'uuid','text','jsonb','int','boolean'
    Required bool
    Default  string
}

var builtinProcs = map[string]ProcSpec{
    "coord_merge_pr": {
        Name: "coord_merge_pr",
        Args: []ProcArg{
            {Name: "p_pr_id", Type: "uuid", Required: true},
            {Name: "p_actor_id", Type: "uuid", Required: true},
            {Name: "p_actor_type", Type: "text", Required: true},
            {Name: "p_sha", Type: "text", Required: false, Default: ""},
        },
        Returns:      "jsonb",
        Description:  "Atomically merge a PR with full audit + outbox enqueue",
        CoordType:    "C",
        RequiredPerm: "pr.merge",
        TimeoutMs:    5000,
        Version:      1,
    },
    "query_pr_review_context": {
        Name: "query_pr_review_context",
        Args: []ProcArg{
            {Name: "p_pr_id", Type: "uuid", Required: true},
        },
        Returns:      "jsonb",
        Description:  "Get full review context: PR + linked issues + reviews + CI + evidence",
        CoordType:    "Q",
        RequiredPerm: "pr.read",
        TimeoutMs:    3000,
        Version:      1,
    },
    "transition_issue_state": {
        Name: "transition_issue_state",
        Args: []ProcArg{
            {Name: "p_issue_id", Type: "uuid", Required: true},
            {Name: "p_from_state", Type: "text", Required: true},
            {Name: "p_to_state", Type: "text", Required: true},
            {Name: "p_actor_id", Type: "uuid", Required: true},
            {Name: "p_reason", Type: "text", Required: false, Default: ""},
        },
        Returns:      "jsonb",
        Description:  "Validated state transition for issue",
        CoordType:    "T",
        RequiredPerm: "issue.write",
        TimeoutMs:    2000,
        Version:      1,
    },
    "sync_commit_to_graph": {
        Name: "sync_commit_to_graph",
        Args: []ProcArg{
            {Name: "p_commit_sha", Type: "text", Required: true},
            {Name: "p_repo_id", Type: "uuid", Required: true},
        },
        Returns:      "jsonb",
        Description:  "Sync git commit to graph node (idempotent)",
        CoordType:    "S",
        RequiredPerm: "graph.write",
        TimeoutMs:    3000,
        Version:      1,
    },
}
```

### 7.4.2 存储过程调用入口

```go
// [IMPL] internal/coordination/procs/caller.go

type Caller struct {
    db          *pgxpool.Pool
    registry    *Registry
    policy      *policy.Engine
    rls         *RLSContext
    metrics     *Metrics
    eventLog    *audit.Emitter
}

func (c *Caller) Call(ctx context.Context, appCtx AppContext, procName string, args map[string]any) (json.RawMessage, error) {
    // 1. 查注册表
    spec, ok := c.registry.Get(procName)
    if !ok { return nil, ErrUnknownProc }
    if !isCoordTypeAllowed(spec.CoordType) { return nil, ErrProcTypeNotAllowed }

    // 2. 校验参数
    finalArgs, err := validateAndFill(spec.Args, args)
    if err != nil { return nil, err }

    // 3. Policy 评估
    allowed, err := c.policy.Evaluate(ctx, PolicyInput{
        Subject:  appCtx.Subject(),       // App as actor
        Action:   spec.RequiredPerm,
        Resource: ResourceRef{Type: "coord", ID: procName},
    })
    if err != nil { return nil, err }
    if !allowed { return nil, ErrPolicyDenied }

    // 4. 设置 RLS 上下文 (PostgreSQL session GUC)
    tx, err := c.db.BeginTx(ctx, pgx.TxOptions{IsoLevel: pgx.ReadCommitted})
    if err != nil { return nil, err }
    defer tx.Rollback(ctx)
    c.rls.SetAppContext(ctx, tx, appCtx)

    // 5. 调用 (在 SECURITY DEFINER 函数内, 由 PL/pgSQL 处理业务)
    result, err := callProc(ctx, tx, spec, finalArgs, c.timeoutFromSpec(spec))
    if err != nil { return nil, err }

    // 6. 写审计 (跨表写由存储过程自身审计, 这里补一笔"调用"事件)
    c.eventLog.EmitAsync(ctx, "coordination.proc_invoked", appCtx, procName, args, time.Since(start))

    return result, tx.Commit(ctx)
}

func callProc(ctx context.Context, tx pgx.Tx, spec ProcSpec, args map[string]any, timeout time.Duration) (json.RawMessage, error) {
    ctx, cancel := context.WithTimeout(ctx, timeout)
    defer cancel()

    // 构造 SELECT coord_proc_name($1, $2, ...) 形式调用
    sql, sqlArgs := buildProcCallSQL(spec, args)
    var result []byte
    err := tx.QueryRow(ctx, sql, sqlArgs...).Scan(&result)
    return result, err
}

func buildProcCallSQL(spec ProcSpec, args map[string]any) (string, []any) {
    parts := make([]string, 0, len(spec.Args))
    sqlArgs := make([]any, 0, len(spec.Args))
    i := 1
    for _, arg := range spec.Args {
        parts = append(parts, fmt.Sprintf("$%d::%s", i, arg.Type))
        sqlArgs = append(sqlArgs, args[arg.Name])
        i++
    }
    return fmt.Sprintf("SELECT %s(%s)", spec.Name, strings.Join(parts, ",")), sqlArgs
}
```

### 7.4.3 存储过程版本治理

**[PROPOSAL]**

| 场景 | 行为 |
|---|---|
| 新增参数 | 必须有 DEFAULT, 旧调用方仍可工作 |
| 移除参数 | 禁止. 必须创建 `coord_v2_*` 新函数 |
| 修改语义 | 创建 `coord_v2_*`, 旧函数标 deprecated |
| 删除函数 | 仅在所有调用方迁移后, 经审批 |

代码侧支持：

```go
// [IMPL] internal/coordination/procs/version.go
func (c *Caller) CallWithVersion(ctx context.Context, appCtx AppContext, procName string, version int, args map[string]any) (json.RawMessage, error) {
    // 显式版本调用: call "coord_v2_merge_pr"
    name := deriveVersionedName(procName, version)
    return c.Call(ctx, appCtx, name, args)
}
```

存储过程自身加注释头：

```sql
-- DEPRECATED: use coord_v2_merge_pr; sunset 2027-01-01
CREATE OR REPLACE FUNCTION coord_merge_pr(...) ...
```

### 7.4.4 App 命名空间与跨 App 存储过程（App Cluster & Plugin 扩展）

**[PROPOSAL]** 平台 §13 将 App 升级为一级对象后，每个 App 拥有独立 schema 与独立 DB role。跨 App 存储过程调用必须满足：

- [PROPOSAL-REQ-COORD-APP-001] App 的存储过程必须位于 schema `app_<app_id>` 下，不得使用 `public` 或 `coord`
- [PROPOSAL-REQ-COORD-APP-002] 跨 schema 调用必须经 `coord_call_app_proc(target_app_id, proc_name, args)` 中介函数（强校验调用方权限与目标 App 状态）
- [PROPOSAL-REQ-COORD-APP-003] 调用方 App 必须在 manifest.spec.permissions.required_grants 中显式声明跨 App 权限
- [PROPOSAL-REQ-COORD-APP-004] 目标 App 必须在 `app_acl` 表中显式授权（默认拒绝）

```sql
-- 跨 App 存储过程调用中介（详细设计 §12.4.3 实现）
CREATE OR REPLACE FUNCTION coord_call_app_proc(
  p_target_app_id text,
  p_proc_name     text,
  p_args          jsonb
) RETURNS jsonb
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
  v_caller_app_id text := current_setting('app.current_app_id', true);
  v_target_role   text := 'app_' || p_target_app_id;
  v_target_schema text := 'app_' || p_target_app_id;
  v_full_name     text := v_target_schema || '.' || p_proc_name;
  v_result        jsonb;
BEGIN
  -- 1. 强校验调用方上下文
  IF v_caller_app_id IS NULL THEN
    RAISE EXCEPTION 'app.current_app_id GUC not set';
  END IF;

  -- 2. 校验调用方在 manifest 中声明了跨 App 权限
  IF NOT EXISTS (
    SELECT 1 FROM app_acl
    WHERE caller_app_id = v_caller_app_id
      AND target_app_id = p_target_app_id
      AND proc_name = p_proc_name
  ) THEN
    RAISE EXCEPTION 'caller % not authorized to call %.%',
      v_caller_app_id, p_target_app_id, p_proc_name;
  END IF;

  -- 3. 校验目标 App 健康
  IF NOT EXISTS (
    SELECT 1 FROM nodes
    WHERE id = p_target_app_id AND type = 'app'
      AND properties->>'state' IN ('healthy', 'degraded')
  ) THEN
    RAISE EXCEPTION 'target app % not healthy', p_target_app_id;
  END IF;

  -- 4. 切换角色执行（最小权限原则）
  EXECUTE format('SET LOCAL ROLE %I', v_target_role);
  EXECUTE format('SELECT %I(%L::jsonb)', p_proc_name, p_args::text) INTO v_result;
  RESET ROLE;

  -- 5. 审计
  INSERT INTO admin_audit (action, target_type, target_id, details)
  VALUES (
    'cross_app_proc_call',
    'app',
    p_target_app_id,
    jsonb_build_object(
      'caller_app_id', v_caller_app_id,
      'proc_name', p_proc_name,
      'success', true
    )
  );

  RETURN v_result;
END;
$$;

REVOKE ALL ON FUNCTION coord_call_app_proc FROM PUBLIC;
-- 仅授权给需要跨 App 调用的 App role
GRANT EXECUTE ON FUNCTION coord_call_app_proc TO app_caller_role_template;
```

`app_acl` 表 DDL：

```sql
CREATE TABLE app_acl (
  id              bigserial PRIMARY KEY,
  caller_app_id   text NOT NULL,
  target_app_id   text NOT NULL,
  proc_name       text NOT NULL,
  granted_by      text NOT NULL,                 -- 'human:uli'
  granted_at      timestamptz NOT NULL DEFAULT now(),
  expires_at      timestamptz,                   -- 可选过期
  UNIQUE (caller_app_id, target_app_id, proc_name)
);

CREATE INDEX idx_app_acl_caller ON app_acl(caller_app_id);
CREATE INDEX idx_app_acl_target ON app_acl(target_app_id);
```

### 7.4.5 中心事件发布触发器（与 §12.3.1 同源）

**[PROPOSAL]** §12.3.1 的 `trigger_event_publish()` 函数是所有 App 发布中心事件的唯一入口。此处补**调用约束**：

- [PROPOSAL-REQ-COORD-APP-005] App 发布事件前必须先在 `event_schema_registry` 注册 schema
- [PROPOSAL-REQ-COORD-APP-006] 发布事件必须在业务事务内（与 §7.6.1 Outbox 模式一致）
- [PROPOSAL-REQ-COORD-APP-007] 不允许 App 直接 INSERT/UPDATE/DELETE `event_stream` 表（强制通过 trigger_event_publish 函数）

```sql
-- 在 7.4 节补充：App 业务存储过程中发布事件的推荐模板
CREATE OR REPLACE FUNCTION app_github_pr_reviewer.record_pr_review(
  p_pr_id uuid,
  p_review jsonb
) RETURNS uuid
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
  v_event_id uuid;
BEGIN
  -- 业务变更（与事件 publish 在同一事务）
  INSERT INTO app_github_pr_reviewer.review_state (pr_id, review, created_at)
  VALUES (p_pr_id, p_review, now())
  ON CONFLICT (pr_id) DO UPDATE SET review = EXCLUDED.review, updated_at = now();

  -- 发布中心事件（§12.3.1 trigger_event_publish）
  v_event_id := trigger_event_publish(
    'review.completed',                     -- event_type
    jsonb_build_object(
      'pr_id', p_pr_id,
      'verdict', p_review->>'verdict',
      'comment_count', jsonb_array_length(p_review->'comments')
    )
    -- causation_id / correlation_id 可选
  );

  RETURN v_event_id;
END;
$$;
```

**关键不变量**：trigger_event_publish 内部事务与 `record_pr_review` 业务事务**同一个事务边界**，事件与业务变更原子提交（Outbox 模式）。

完整实现、性能预算、错误处理见 [详细设计 §12.3 中心事件总线实现](12-app-registry-and-plugin-loader.md#123-中心事件总线实现)。

---

## 7.5 LISTEN/NOTIFY 通知

### 7.5.1 发布

```go
// [IMPL] internal/coordination/notify/publisher.go
type Publisher struct {
    db *pgxpool.Pool
}

func (p *Publisher) Notify(ctx context.Context, channel string, payload any) error {
    data, err := json.Marshal(payload)
    if err != nil { return err }

    // PostgreSQL NOTIFY payload 上限 8 KB
    if len(data) > 8000 {
        return ErrPayloadTooLarge  // 改用 outbox
    }

    _, err = p.db.Exec(ctx, "SELECT pg_notify($1, $2)", channel, string(data))
    return err
}
```

### 7.5.2 订阅

```go
// [IMPL] internal/coordination/notify/subscriber.go
type Subscriber struct {
    conn     *pgx.Conn
    handlers map[string][]Handler
    mu       sync.RWMutex
}

type Handler func(ctx context.Context, payload []byte) error

func (s *Subscriber) Subscribe(ctx context.Context, channels []string, handler Handler) error {
    for _, ch := range channels {
        _, err := s.conn.Exec(ctx, "LISTEN "+quoteIdent(ch))
        if err != nil { return err }
    }

    go s.dispatch(ctx)
    return nil
}

func (s *Subscriber) dispatch(ctx context.Context) {
    for {
        notif, err := s.conn.WaitForNotification(ctx)
        if err != nil { return }
        s.mu.RLock()
        handlers := s.handlers[notif.Channel]
        s.mu.RUnlock()
        for _, h := range handlers {
            go func(h Handler) {
                if err := h(ctx, []byte(notif.Payload)); err != nil {
                    log.Error("notify handler failed", "channel", notif.Channel, "err", err)
                }
            }(h)
        }
    }
}
```

**[PROPOSAL]** 通知的命名空间：

| 前缀 | 含义 |
|---|---|
| `pr.*` | PR 状态变更 |
| `issue.*` | Issue 状态变更 |
| `agent_run.*` | Agent 运行进度 |
| `policy.*` | Policy 变更 |
| `coordination.*` | 协调事件 |

## 7.6 Outbox + Relay

### 7.6.1 Outbox 写入

**[PROPOSAL]** Outbox 写入发生在**主事务内**——这是 at-least-once 投递的基础。

```go
// [IMPL] internal/coordination/outbox/writer.go
type Writer struct {
    db *pgxpool.Pool
}

func (w *Writer) Enqueue(ctx context.Context, tx pgx.Tx, event OutboxEvent) error {
    _, err := tx.Exec(ctx, `
        INSERT INTO outbox (event_type, payload, target_filter, next_attempt_at)
        VALUES ($1, $2, $3, now())
    `, event.Type, event.Payload, event.TargetFilter)
    return err
}
```

**使用示例：**

```go
// [IMPL] 在 PL/pgSQL 存储过程 coord_merge_pr 中:
INSERT INTO outbox (event_type, payload, target_filter)
VALUES ('pr.merged',
        jsonb_build_object('pr_id', p_pr_id, 'actor_id', p_actor_id, 'repo_id', v_repo_id),
        jsonb_build_object('repo_id', v_repo_id));
```

### 7.6.2 Relay 主循环

```go
// [IMPL] internal/coordination/outbox/relay.go
type Relay struct {
    db        *pgxpool.Pool
    targets   *TargetResolver
    http      *http.Client
    metrics   *Metrics
    workerCount int       // 默认 4
    pollInterval time.Duration  // 默认 2s
    batchSize    int       // 默认 50
}

func (r *Relay) Run(ctx context.Context) error {
    // 选主: 仅一个 relay 进程 active (advisory lock)
    if !r.tryAcquireLeaderLock(ctx) { return nil /* 让另一个 instance 处理 */ }
    defer r.releaseLeaderLock(ctx)

    ticker := time.NewTicker(r.pollInterval)
    defer ticker.Stop()
    for {
        select {
        case <-ctx.Done(): return ctx.Err()
        case <-ticker.C:
            r.processBatch(ctx)
        }
    }
}

func (r *Relay) processBatch(ctx context.Context) {
    // 1. 取 pending 任务
    rows, err := r.db.Query(ctx, `
        SELECT id, event_type, payload, target_filter, attempts
        FROM outbox
        WHERE status = 'pending' AND next_attempt_at <= now()
        ORDER BY created_at
        LIMIT $1
        FOR UPDATE SKIP LOCKED
    `, r.batchSize)
    if err != nil { return }

    type job struct {
        ID          uuid.UUID
        EventType   string
        Payload     json.RawMessage
        TargetFilter json.RawMessage
        Attempts    int
    }
    var jobs []job
    for rows.Next() {
        var j job
        if err := rows.Scan(&j.ID, &j.EventType, &j.Payload, &j.TargetFilter, &j.Attempts); err != nil {
            continue
        }
        jobs = append(jobs, j)
    }
    rows.Close()

    // 2. 并发投递 (worker pool)
    sem := make(chan struct{}, r.workerCount)
    var wg sync.WaitGroup
    for _, j := range jobs {
        sem <- struct{}{}
        wg.Add(1)
        go func(j job) {
            defer func() { <-sem; wg.Done() }()
            r.deliver(ctx, j)
        }(j)
    }
    wg.Wait()
}
```

### 7.6.3 投递与重试

```go
// [IMPL] internal/coordination/outbox/relay.go
func (r *Relay) deliver(ctx context.Context, j job) {
    // 1. 解析 target_url
    targetURL, err := r.targets.Resolve(j.EventType, j.TargetFilter)
    if err != nil {
        r.markFailed(ctx, j.ID, err.Error(), r.nextAttempt(j.Attempts))
        return
    }

    // 2. 标记 in_flight
    r.db.Exec(ctx, "UPDATE outbox SET status='in_flight' WHERE id=$1", j.ID)

    // 3. 签名
    signature := hmacSign([]byte(secret), j.Payload)

    // 4. HTTP POST
    req, _ := http.NewRequestWithContext(ctx, "POST", targetURL, bytes.NewReader(j.Payload))
    req.Header.Set("Content-Type", "application/json")
    req.Header.Set("X-Webhook-Signature", "sha256="+hex.EncodeToString(signature))
    req.Header.Set("X-Webhook-Delivery-Id", j.ID.String())
    req.Header.Set("X-Platform-Event", j.EventType)

    resp, err := r.http.Do(req)
    if err != nil {
        r.markFailed(ctx, j.ID, err.Error(), r.nextAttempt(j.Attempts))
        return
    }
    defer resp.Body.Close()

    // 5. 处理响应
    if resp.StatusCode >= 200 && resp.StatusCode < 300 {
        r.markSent(ctx, j.ID)
    } else if resp.StatusCode >= 400 && resp.StatusCode < 500 && resp.StatusCode != 408 && resp.StatusCode != 429 {
        // 4xx (非 408/429) -> 永久失败, 进 DLQ
        r.markDLQ(ctx, j.ID, fmt.Sprintf("status %d", resp.StatusCode))
    } else {
        // 5xx 或 408/429 -> 临时失败, 重试
        r.markFailed(ctx, j.ID, fmt.Sprintf("status %d", resp.StatusCode), r.nextAttempt(j.Attempts))
    }
}

func (r *Relay) nextAttempt(attempts int) time.Time {
    backoff := time.Duration(1<<attempts) * time.Second  // 1s, 2s, 4s, 8s, 16s, 32s, 64s
    if backoff > 30*time.Minute { backoff = 30 * time.Minute }
    return time.Now().Add(backoff)
}
```

### 7.6.4 目标解析

```go
// [IMPL] internal/coordination/outbox/targets.go
type TargetResolver struct {
    db *pgxpool.Pool
}

func (r *TargetResolver) Resolve(eventType string, filter json.RawMessage) (string, error) {
    // 1. 查 webhook 表
    var webhooks []Webhook
    if err := r.db.Select(...).Where(...).Find(&webhooks).Error; err != nil {
        return "", err
    }

    // 2. 过滤匹配的 webhook
    var urls []string
    for _, w := range webhooks {
        if w.MatchesEvent(eventType) && w.MatchesFilter(filter) {
            urls = append(urls, w.TargetURL)
        }
    }
    if len(urls) == 0 {
        return "", ErrNoTarget
    }
    if len(urls) == 1 { return urls[0], nil }

    // 多个目标: fan-out
    return strings.Join(urls, ","), nil  // caller 负责拆开
}
```

### 7.6.5 死信队列

```go
// [IMPL] internal/coordination/outbox/dlq.go

func (r *Relay) handleDLQ(ctx context.Context) {
    // 每天一次, 把 status='dlq' 的事件汇总, 告警
    rows, _ := r.db.Query(ctx, `
        SELECT event_type, COUNT(*), MIN(created_at), MAX(created_at)
        FROM outbox
        WHERE status = 'dlq' AND created_at > now() - interval '24 hours'
        GROUP BY event_type
    `)
    for rows.Next() {
        var eventType string
        var count int
        var min, max time.Time
        rows.Scan(&eventType, &count, &min, &max)
        if count > 0 {
            alerting.Send(ctx, "outbox_dlq", map[string]any{
                "event_type": eventType, "count": count, "since": min,
            })
        }
    }
}
```

## 7.7 Saga 引擎

### 7.7.1 Saga 定义

```go
// [IMPL] internal/coordination/saga/step.go
type SagaDef struct {
    Type    string
    Steps   []StepDef
    Timeout time.Duration
}

type StepDef struct {
    Name         string
    Forward      StepFunc
    Compensate   StepFunc        // 失败时回滚
    RetryPolicy  RetryPolicy
    Timeout      time.Duration
}

type StepFunc func(ctx context.Context, sagaCtx SagaContext) (json.RawMessage, error)

type RetryPolicy struct {
    MaxAttempts int
    BackoffBase time.Duration
    BackoffMax  time.Duration
}

var PRFullLifecycle = SagaDef{
    Type: "pr_full_lifecycle",
    Steps: []StepDef{
        {
            Name: "validate_pr",
            Forward: validatePRStep,
            Compensate: noCompensate,
            Timeout: 5 * time.Second,
        },
        {
            Name: "trigger_ci",
            Forward: triggerCIStep,
            Compensate: cancelCIStep,
            Timeout: 30 * time.Second,
        },
        {
            Name: "ai_review",
            Forward: aiReviewStep,
            Compensate: noCompensate,
            Timeout: 5 * time.Minute,
        },
        {
            Name: "merge_candidate",
            Forward: mergeCandidateStep,
            Compensate: revertMergeStep,
            Timeout: 10 * time.Second,
        },
        {
            Name: "notify",
            Forward: notifyStep,
            Compensate: noCompensate,  // 已发出通知无法撤回
            Timeout: 5 * time.Second,
        },
    },
    Timeout: 1 * time.Hour,
}
```

### 7.7.2 引擎主循环

```go
// [IMPL] internal/coordination/saga/engine.go
type Engine struct {
    db      *pgxpool.Pool
    defs    map[string]SagaDef
    workers int  // 默认 4
}

func (e *Engine) Start(ctx context.Context, sagaType string, payload json.RawMessage) (uuid.UUID, error) {
    // 1. 创建 saga_instance
    sagaID := uuid.Must(uuid.NewV7())
    _, err := e.db.Exec(ctx, `
        INSERT INTO saga_instances (id, saga_type, state, payload, steps_total)
        VALUES ($1, $2, 'pending', $3, $4)
    `, sagaID, sagaType, payload, len(e.defs[sagaType].Steps))
    if err != nil { return uuid.Nil, err }
    return sagaID, nil
}

func (e *Engine) Run(ctx context.Context) error {
    ticker := time.NewTicker(5 * time.Second)
    defer ticker.Stop()
    for {
        select {
        case <-ctx.Done(): return ctx.Err()
        case <-ticker.C:
            e.processPending(ctx)
        }
    }
}

func (e *Engine) processPending(ctx context.Context) {
    // 1. 抢占 running 状态的 saga
    rows, err := e.db.Query(ctx, `
        UPDATE saga_instances
        SET state = 'running', last_event_at = now()
        WHERE state IN ('pending', 'running')
          AND last_event_at < now() - interval '5 seconds'  -- 防 worker 崩溃后无人推进
          AND (started_at IS NULL OR started_at > now() - interval '24 hours')
        RETURNING id, saga_type, payload, current_step, steps_done
    `)
    if err != nil { return }
    defer rows.Close()

    for rows.Next() {
        var (
            id      uuid.UUID
            stype   string
            payload json.RawMessage
            step    *string
            done    int
        )
        rows.Scan(&id, &stype, &payload, &step, &done)

        def, ok := e.defs[stype]
        if !ok { continue }

        nextIdx := done
        if nextIdx >= len(def.Steps) {
            // 完成
            e.complete(ctx, id)
            continue
        }
        e.runStep(ctx, id, def, nextIdx, payload)
    }
}

func (e *Engine) runStep(ctx context.Context, sagaID uuid.UUID, def SagaDef, idx int, payload json.RawMessage) {
    step := def.Steps[idx]

    // 1. 写步骤日志
    e.db.Exec(ctx, `
        INSERT INTO saga_step_log (saga_id, step_name, status)
        VALUES ($1, $2, 'running')
    `, sagaID, step.Name)

    // 2. 跑 forward
    result, err := step.Forward(ctx, SagaContext{ID: sagaID, Payload: payload})
    if err != nil {
        // 失败: 进入补偿
        e.compensate(ctx, sagaID, def, idx, err)
        return
    }

    // 3. 成功: 推进
    e.db.Exec(ctx, `
        UPDATE saga_instances
        SET current_step = $1, steps_done = $2, last_event_at = now()
        WHERE id = $3
    `, step.Name, idx+1, sagaID)
    e.db.Exec(ctx, `
        UPDATE saga_step_log
        SET ended_at = now(), status = 'success', result = $1
        WHERE saga_id = $2 AND step_name = $3 AND ended_at IS NULL
    `, result, sagaID, step.Name)
}

func (e *Engine) compensate(ctx context.Context, sagaID uuid.UUID, def SagaDef, failedIdx int, cause error) {
    e.db.Exec(ctx, `UPDATE saga_instances SET state = 'compensating', error = $1 WHERE id = $2`, cause.Error(), sagaID)

    // 倒序补偿已成功的步骤
    for i := failedIdx - 1; i >= 0; i-- {
        if def.Steps[i].Compensate == nil { continue }
        _ = def.Steps[i].Compensate(ctx, SagaContext{ID: sagaID})
        e.db.Exec(ctx, `
            UPDATE saga_step_log
            SET status = 'compensated'
            WHERE saga_id = $1 AND step_name = $2 AND ended_at IS NULL
        `, sagaID, def.Steps[i].Name)
    }

    e.db.Exec(ctx, `
        UPDATE saga_instances
        SET state = 'failed', completed_at = now()
        WHERE id = $1
    `, sagaID)
}
```

## 7.8 跨群组隔离

**[PROPOSAL]** 默认隔离；显式 `cross_group_policies` 允许跨群组访问。

```go
// [IMPL] internal/coordination/appgroup/crossgroup.go

// 调用前先检查
func (c *Coordinator) CheckCrossGroup(ctx context.Context, source, target AppGroup, resourceType string, action string) (bool, error) {
    var policy CrossGroupPolicy
    err := c.db.QueryRow(ctx, `
        SELECT allowed_actions, expires_at
        FROM cross_group_policies
        WHERE source_group = $1 AND target_group = $2 AND resource_type = $3
          AND (expires_at IS NULL OR expires_at > now())
    `, source.ID, target.ID, resourceType).Scan(&policy.AllowedActions, &policy.ExpiresAt)
    if err != nil {
        if errors.Is(err, pgx.ErrNoRows) { return false, nil }  // 默认拒绝
        return false, err
    }
    return slices.Contains(policy.AllowedActions, action), nil
}
```

## 7.9 监控指标

```go
// [IMPL] internal/coordination/observability/metrics.go
type Metrics struct {
    ProcDuration      *prometheus.HistogramVec  // labels: proc, coord_type
    ProcErrorRate     *prometheus.CounterVec    // labels: proc, code
    OutboxPending     prometheus.Gauge
    OutboxLag         *prometheus.HistogramVec  // labels: target_type
    OutboxDLQTotal    *prometheus.CounterVec
    SagaRunning       *prometheus.GaugeVec      // labels: saga_type
    SagaFailed        *prometheus.CounterVec
    SagaStepDuration  *prometheus.HistogramVec  // labels: saga_type, step
    LocksHeld         prometheus.Gauge
}
```

### 7.9.1 运维 SQL 视图

```sql
-- v_outbox_health
CREATE VIEW v_outbox_health AS
SELECT
    event_type,
    status,
    COUNT(*) AS count,
    MIN(EXTRACT(EPOCH FROM (now() - created_at))) AS min_age_sec,
    MAX(EXTRACT(EPOCH FROM (now() - created_at))) AS max_age_sec
FROM outbox
WHERE created_at > now() - interval '24 hours'
GROUP BY event_type, status;

-- v_slow_procs
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

## 7.10 错误定义

```go
// [IMPL] internal/coordination/errors.go
var (
    ErrUnknownProc          = NewError("coord_unknown_proc", 400, "unknown procedure")
    ErrProcTypeNotAllowed   = NewError("coord_proc_type_not_allowed", 403, "procedure type not allowed for caller")
    ErrPolicyDenied         = NewError("coord_policy_denied", 403, "coordination denied by policy")
    ErrPayloadTooLarge     = NewError("coord_payload_too_large", 413, "payload too large for NOTIFY, use outbox")
    ErrNoTarget             = NewError("outbox_no_target", 500, "no delivery target for event")
    ErrSagaNotFound         = NewError("saga_not_found", 404, "saga instance not found")
    ErrSagaStepTimeout      = NewError("saga_step_timeout", 504, "saga step exceeded timeout")
)
```

## 7.11 性能预算

| 指标 | 目标 | 备注 |
|---|---|---|
| 存储过程调用（不含 proc 自身）| < 5ms | 含事务、Policy 评估、Event 写 |
| 存储过程自身（典型 C 类）| < 50ms | 单事务 |
| LISTEN/NOTIFY 投递延迟 | < 10ms | 进程内 |
| Outbox relay 投递延迟 | < 2s (P95) | 含 HTTP |
| Saga 步进（5 步）| < 1 分钟 | 不含外部等待 |

## 7.12 测试

| 测试 | 目标 |
|---|---|
| 单元 | 注册表、参数校验、版本管理 |
| 集成 (testcontainers) | 真实 PG 调用 PL/pgSQL |
| Saga | forward 失败、补偿触发、最终态 |
| Outbox | 重试、DLQ、leader lock |
| RLS | 跨群组访问被拒、显式策略允许 |

---

**导航 / Navigation:**
[← 06. Git 服务器](06-git-server.md) · [README](README.md) · [08. API 处理器 →](08-api-handlers.md)
