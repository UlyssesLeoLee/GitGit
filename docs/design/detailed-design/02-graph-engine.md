# 02. 图谱引擎 / Graph Engine

## 2.1 目标 / Purpose

实现 [基本设计 §3.2 Engineering Graph 子系统](../basic-design/03-functional-design.md#32-工程图谱子系统-engineering-graph-subsystem) 的 5 个原语（Node、Edge、Event、Policy、View），提供 CRUD、遍历、类型校验、缓存能力。本章是平台差异化主轴的实现核心。

## 2.2 模块结构 / Module Layout

```
internal/graph/
├── node/
│   ├── node.go              # Node entity 定义与基础方法
│   ├── repository.go        # DB CRUD (使用 01 schema)
│   ├── service.go           # 业务规则 (类型校验, 默认值, 触发 Event)
│   └── node_test.go
│
├── edge/
│   ├── edge.go
│   ├── repository.go
│   ├── service.go
│   └── edge_test.go
│
├── event/
│   ├── event.go             # 不可变事件结构
│   ├── writer.go            # INSERT-only 写入器
│   └── query.go             # 按 subject_node_id / event_type / actor 检索
│
├── policy/
│   ├── policy.go
│   ├── repository.go
│   └── evaluator.go         # 03 policy engine 引用
│
├── view/
│   ├── view.go
│   ├── invocation.go        # 调用日志 (CTX-REQ-002 精确重建)
│   └── renderer.go          # 视图渲染
│
├── traversal/
│   ├── recursive.go         # 递归 CTE 封装
│   ├── cycle.go             # 防环逻辑
│   └── depth.go             # 深度控制
│
├── registry/
│   ├── types.go             # 类型注册表接口
│   ├── builtin.go           # 内置类型 (初始化)
│   └── validator.go         # JSON Schema 校验
│
├── cache/
│   ├── lru.go               # 节点/边 LRU 缓存
│   └── invalidation.go      # 写时失效
│
└── errors.go                # 领域错误
```

## 2.3 核心类型 / Core Types

```go
// [IMPL] 伪代码

// Node 实体
type Node struct {
    ID         uuid.UUID
    NodeType   string
    Properties json.RawMessage
    CreatedAt  time.Time
    UpdatedAt  time.Time
    DeletedAt  *time.Time
}

// Edge 实体
type Edge struct {
    ID         uuid.UUID
    FromNodeID uuid.UUID
    ToNodeID   uuid.UUID
    EdgeType   string
    Properties json.RawMessage
    CreatedAt  time.Time
}

// Event 不可变
type Event struct {
    Seq            int64
    SubjectNodeID  *uuid.UUID
    EventType      string
    ActorID        *uuid.UUID
    Payload        json.RawMessage
    OccurredAt     time.Time
}

// Policy
type Policy struct {
    ID         uuid.UUID
    PolicyType string
    Rules      json.RawMessage
    Version    int
    Active     bool
    CreatedAt  time.Time
}

// View
type View struct {
    ID         uuid.UUID
    ViewType   string
    Parameters json.RawMessage
    OwnerID    *uuid.UUID
    CreatedAt  time.Time
}
```

## 2.4 类型注册表 / Type Registry

**[PROPOSAL]** 类型注册表是图谱扩展能力的核心。所有 Node/Edge 创建必须先在注册表中找到对应类型定义，否则拒绝。注册表同时承担 JSON Schema 校验与默认属性注入。

### 2.4.1 类型定义格式

```go
// [IMPL] 伪代码
type NodeTypeDef struct {
    Name        string
    Description string
    IsAbstract  bool                  // 抽象类型不能直接创建
    ParentType  string                // 继承
    Schema      *jsonschema.Schema    // 属性校验
    Defaults    map[string]any        // 创建时自动注入的默认值
    Hooks       *NodeHooks            // 创建/更新/删除钩子
}

type EdgeTypeDef struct {
    Name        string
    Description string
    FromTypes   []string              // 允许的起点 node_type 白名单
    ToTypes     []string              // 允许的终点 node_type 白名单
    IsDirected  bool
    Schema      *jsonschema.Schema
}
```

### 2.4.2 注册表初始化

**[IMPL]** 内置类型在 Platform 启动时同步加载（从 [01 数据层 §1.4.3](01-data-layer.md#143-类型注册表-type-registry) 的 SQL 种子 + Go 代码内的硬编码类型）。两者必须一致，由 [测试 2.4.4](#244-类型注册表一致性测试) 保证。

```go
// [IMPL] internal/graph/registry/builtin.go
var builtinNodeTypes = []NodeTypeDef{
    {Name: "issue", Schema: mustParseSchema(`{
        "type": "object",
        "required": ["title", "repo_id"],
        "properties": {
            "title": {"type": "string", "minLength": 1, "maxLength": 500},
            "body": {"type": "string"},
            "state": {"type": "string", "enum": ["open", "in_progress", "closed", "wontfix"]},
            "labels": {"type": "array", "items": {"type": "string"}},
            "assignee_id": {"type": "string", "format": "uuid"},
            "repo_id": {"type": "string", "format": "uuid"}
        }
    }`)},
    {Name: "pull_request", Schema: ...},
    // ...
}

var builtinEdgeTypes = []EdgeTypeDef{
    {Name: "blocks", FromTypes: []string{"issue"}, ToTypes: []string{"issue"}},
    {Name: "implements", FromTypes: []string{"pr","commit"}, ToTypes: []string{"requirement"}},
    {Name: "reviewed_by", FromTypes: []string{"pull_request"}, ToTypes: []string{"human","agent_run"}},
    // ...
}
```

### 2.4.3 创建节点时的校验流程

```go
// [IMPL] internal/graph/node/service.go
func (s *Service) Create(ctx context.Context, actor Actor, nodeType string, props json.RawMessage) (Node, error) {
    // 1. 查 type registry
    typeDef, err := s.registry.GetNodeType(nodeType)
    if err != nil { return Node{}, err }

    // 2. 抽象类型不能直接创建
    if typeDef.IsAbstract {
        return Node{}, ErrAbstractTypeNode
    }

    // 3. Policy 评估 (Actor 能否创建此类型节点)
    allowed, err := s.policy.Evaluate(ctx, PolicyInput{
        Subject: actor,
        Action:  "create",
        Resource: ResourceRef{Type: nodeType, ID: "*"},
    })
    if err != nil { return Node{}, err }
    if !allowed { return Node{}, ErrPolicyDenied }

    // 4. JSON Schema 校验 + 注入默认值
    final, err := s.registry.ValidateAndFill(nodeType, props)
    if err != nil { return Node{}, err }

    // 5. 生成 UUID v7
    id, err := uuid.NewV7()
    if err != nil { return Node{}, err }

    // 6. 写 Node + 触发 Event (同事务)
    err = s.txRunner.Run(ctx, func(tx Tx) error {
        if err := s.nodes.Insert(tx, Node{ID: id, NodeType: nodeType, Properties: final, ...}); err != nil {
            return err
        }
        return s.events.Append(tx, Event{
            SubjectNodeID: &id,
            EventType:     "node.created",
            ActorID:       actor.ID,
            Payload:       mustMarshal(map[string]any{"node_type": nodeType, "properties": final}),
        })
    })
    if err != nil { return Node{}, err }

    // 7. 失效缓存
    s.cache.InvalidateNode(id)
    return Node{ID: id, NodeType: nodeType, Properties: final, ...}, nil
}
```

### 2.4.4 类型注册表一致性测试

**[PROPOSAL]** CI 必须运行：

```go
// [IMPL] internal/graph/registry/registry_test.go
func TestBuiltinTypesMatchSQLSeed(t *testing.T) {
    // 1. 启动 testcontainers
    db := startPostgres(t)
    applyMigrations(t, db)

    // 2. 加载 builtin
    reg := NewRegistryFromBuiltin()

    // 3. 加载 SQL seed
    sqlTypes := loadTypesFromSQLSeed(t, db)

    // 4. 双向比对
    for _, t := range builtinNodeTypes {
        require.Contains(t, sqlTypes, t.Name, "builtin missing in SQL seed")
    }
    for _, t := range sqlTypes {
        require.Contains(t, builtinNodeTypes, t, "SQL seed has type not in builtin")
    }
}
```

## 2.5 Edge 服务

### 2.5.1 创建边

```go
// [IMPL] internal/graph/edge/service.go
func (s *Service) Create(ctx context.Context, actor Actor, fromID, toID uuid.UUID, edgeType string, props json.RawMessage) (Edge, error) {
    // 1. 加载两个节点 (含 deleted_at IS NULL 检查)
    from, err := s.nodes.Get(ctx, fromID)
    if err != nil { return Edge{}, err }
    to, err := s.nodes.Get(ctx, toID)
    if err != nil { return Edge{}, err }

    // 2. 查 edge type 注册表
    typeDef, err := s.registry.GetEdgeType(edgeType)
    if err != nil { return Edge{}, err }

    // 3. 类型白名单检查
    if !slices.Contains(typeDef.FromTypes, from.NodeType) {
        return Edge{}, fmt.Errorf("edge type %q cannot start from node type %q", edgeType, from.NodeType)
    }
    if !slices.Contains(typeDef.ToTypes, to.NodeType) {
        return Edge{}, fmt.Errorf("edge type %q cannot end at node type %q", edgeType, to.NodeType)
    }

    // 4. 自环检查
    if fromID == toID {
        return Edge{}, ErrSelfLoop
    }

    // 5. 特殊: Evidence 类型边必须有 epistemic_status (GRF-REQ-010 amendment)
    if edgeType == "evidence" {
        var p map[string]any
        if err := json.Unmarshal(props, &p); err != nil {
            return Edge{}, err
        }
        status, _ := p["epistemic_status"].(string)
        if !slices.Contains([]string{"verified","asserted","attested"}, status) {
            return Edge{}, ErrEvidenceMissingEpistemicStatus
        }
    }

    // 6. Policy 评估
    allowed, err := s.policy.Evaluate(ctx, PolicyInput{...})
    if !allowed { return Edge{}, ErrPolicyDenied }

    // 7. 事务内写 Edge + Event
    err = s.txRunner.Run(ctx, func(tx Tx) error {
        edge := Edge{ID: uuid.Must(uuid.NewV7()), FromNodeID: fromID, ToNodeID: toID, EdgeType: edgeType, Properties: props, ...}
        if err := s.edges.Insert(tx, edge); err != nil {
            return err
        }
        return s.events.Append(tx, Event{
            SubjectNodeID: &fromID,
            EventType:     "edge.created",
            ActorID:       actor.ID,
            Payload: mustMarshal(map[string]any{"edge_type": edgeType, "to_id": toID, "properties": props}),
        })
    })
    s.cache.InvalidateEdge(fromID, toID, edgeType)
    return edge, err
}
```

### 2.5.2 软删除与更正模式

**[PROPOSAL]** 边不可物理删除（基本设计 [§4.5 数据生命周期](../basic-design/04-data-design.md#45-数据生命周期-data-lifecycle)）。要"取消"一条边：

1. 创建一条**反向** `cancels` 边
2. 查询逻辑视图中过滤 `cancels` 边指向的边

```go
// [IMPL] internal/graph/edge/service.go
func (s *Service) Cancel(ctx context.Context, actor Actor, edgeID uuid.UUID, reason string) error {
    edge, err := s.edges.Get(ctx, edgeID)
    if err != nil { return err }

    cancelEdge := Edge{
        ID:         uuid.Must(uuid.NewV7()),
        FromNodeID: edge.ToNodeID,  // 反向
        ToNodeID:   edge.FromNodeID,
        EdgeType:   "cancels",
        Properties: mustMarshal(map[string]any{"cancelled_edge_id": edgeID, "reason": reason}),
    }
    return s.txRunner.Run(ctx, func(tx Tx) error {
        if err := s.edges.Insert(tx, cancelEdge); err != nil { return err }
        return s.events.Append(tx, Event{
            SubjectNodeID: &edge.FromNodeID,
            EventType:     "edge.cancelled",
            ActorID:       actor.ID,
            Payload: mustMarshal(map[string]any{"edge_id": edgeID, "reason": reason}),
        })
    })
}
```

## 2.6 Event 写入器

**[PROPOSAL]** Event 写入器是 AISEC-REQ-009(a) 的执行点。它在 DB 层只通过 INSERT 与 `events` 表交互。**应用代码不会持有 UPDATE/DELETE 权限**。

```go
// [IMPL] internal/graph/event/writer.go
type Writer struct {
    db *pgxpool.Pool  // 连接 platform_runtime role
}

func (w *Writer) Append(ctx context.Context, tx Tx, e Event) error {
    // Append 是唯一与 events 表交互的路径
    // INSERT-only 语义: 任何尝试 UPDATE/DELETE 会被 DB role 拒绝 (见 01.3.2)
    _, err := tx.Exec(ctx, `
        INSERT INTO events (subject_node_id, event_type, actor_id, payload, occurred_at)
        VALUES ($1, $2, $3, $4, COALESCE($5, now()))
    `, e.SubjectNodeID, e.EventType, e.ActorID, e.Payload, e.OccurredAt)
    return err
}

// 注意: 不要实现 Update() 或 Delete() 方法. 物理上不可.
```

## 2.7 图遍历

### 2.7.1 递归 CTE 封装

```rust
// [IMPL] crates/platform-graph/src/traversal/recursive.rs
pub struct TraversalOptions {
    pub start_node_id: Uuid,
    pub direction: Direction,         // Downstream, Upstream, Both
    pub edge_types: Vec<String>,      // 空 = 全部
    pub node_type_filter: Vec<String>,// 终点节点类型过滤
    pub max_depth: u32,               // 默认 4, 硬上限 6 (防止环路与高阶图爆炸)
    pub limit: u32,                   // 默认 1000, 硬上限 5000
    pub include_start: bool,
}

pub struct TraversalResult {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub truncated: bool,
}

pub async fn traverse(
    pool: &PgPool,
    opts: &TraversalOptions,
) -> Result<TraversalResult, PlatformError> {
    let max_depth = opts.max_depth.clamp(1, 6);
    let limit = opts.limit.clamp(1, 5000);

    // 1. 开启只读事务并设置 2000ms 语句级超时保护
    let mut tx = pool.begin().await?;
    sqlx::query!("SET LOCAL statement_timeout = '2000ms'")
        .execute(&mut *tx)
        .await?;

    let (sql, args) = build_recursive_cte(opts, max_depth, limit);
    // 执行查询与组装结果
    // ...
    tx.commit().await?;
    Ok(result)
}
```

### 2.7.2 SQL 构造

```go
// [IMPL] internal/graph/traversal/recursive.go
func buildRecursiveCTE(opts TraversalOptions) (string, []any) {
    // 方向: downstream (to_node 方向) / upstream (from_node 方向) / both (双向)
    directionClause := ""
    switch opts.Direction {
    case Downstream:
        directionClause = "e.from_node_id = d.node_id"
    case Upstream:
        directionClause = "e.to_node_id = d.node_id"
    case Both:
        directionClause = "(e.from_node_id = d.node_id OR e.to_node_id = d.node_id)"
    }

    edgeTypeFilter := ""
    if len(opts.EdgeTypes) > 0 {
        edgeTypeFilter = fmt.Sprintf("AND e.edge_type = ANY($%d)", argIdx)
        // ...
    }

    return fmt.Sprintf(`
        WITH RECURSIVE walk(node_id, depth, path) AS (
            SELECT $1::uuid, 1, ARRAY[$1]::uuid[]
            UNION ALL
            SELECT
                CASE WHEN %s THEN e.to_node_id ELSE e.from_node_id END,
                w.depth + 1,
                w.path || CASE WHEN %s THEN e.to_node_id ELSE e.from_node_id END
            FROM walk w
            JOIN edges e ON %s
            WHERE w.depth < $%d
              AND NOT (CASE WHEN %s THEN e.to_node_id ELSE e.from_node_id END = ANY(w.path))
              %s
        )
        SELECT DISTINCT n.id, n.node_type, n.properties, w.depth
        FROM walk w
        JOIN nodes n ON n.id = w.node_id
        WHERE n.deleted_at IS NULL
        ORDER BY w.depth
        LIMIT $%d
    `, directionClause, directionClause, directionClause, depthIdx, directionClause, edgeTypeFilter, limitIdx), args
}
```

### 2.7.3 防环与深度双控制

**[PROPOSAL]** 防环两道防线：

1. **应用层：** `path` 数组记录已访问节点，新候选若已存在则跳过
2. **DB 层：** `WHERE NOT (node = ANY(path))` 在 CTE 递归部分

**深度双控制：** 应用层 `MaxDepth` 限制 + DB 层 `WHERE depth < $N` 限制。即使应用层有 bug，DB 层也兜底。

### 2.7.4 性能优化

| 优化 | 说明 |
|---|---|
| 早期 LIMIT | 递归 CTE 完成后才 LIMIT，浪费。可在递归过程中携带 `LIMIT count` 子句（PG 14+ 支持）|
| 物化路径缓存 | 频繁查询的路径可缓存 `path_cache` 表 |
| 物化视图 | 高频"X 节点的下游所有 Node"可建物化视图，定时刷新 |
| 类型过滤下推 | 终点类型过滤应在 WHERE 中而非应用层 |
| 索引覆盖 | `(from_node_id, edge_type)` 复合索引（已在 01.5.1）|

## 2.8 视图 (View) 系统

### 2.8.1 视图定义

```go
// [IMPL] internal/graph/view/view.go
type View struct {
    ID         uuid.UUID
    ViewType   string
    Parameters json.RawMessage  // 视图参数
    OwnerID    *uuid.UUID
}

type ViewResult struct {
    Items     []any
    Truncated bool
    Cursor    string
}

// Renderer 渲染一个 view: 给定 parameters, 产出结果集
type Renderer interface {
    Render(ctx context.Context, params json.RawMessage, limit int) (ViewResult, error)
}
```

### 2.8.2 内置视图类型

**[PROPOSAL]**

| 视图类型 | 参数 | 渲染器 |
|---|---|---|
| `my_open_issues` | `{assignee_id, repo_ids?, limit?}` | SELECT * FROM nodes WHERE node_type='issue' AND ... |
| `prs_for_review` | `{reviewer_id, state?}` | 复杂 JOIN |
| `agent_runs_in_flight` | `{actor_id?, since?}` | SELECT FROM nodes + edges |
| `incident_timeline` | `{incident_id, since?}` | 按事件时间线回放 |
| `graph_downstream` | `{start_id, edge_types?, max_depth?}` | 调用 traversal 引擎 |

### 2.8.3 调用日志（CTX-REQ-002 精确重建）

```go
// [IMPL] internal/graph/view/invocation.go
func (v *Service) Invoke(ctx context.Context, actor Actor, viewID uuid.UUID, params json.RawMessage) (ViewResult, error) {
    view, err := v.repo.Get(viewID)
    if err != nil { return ViewResult{}, err }

    // Policy 评估
    allowed, err := v.policy.Evaluate(ctx, PolicyInput{
        Subject: actor,
        Action:  "view.invoke",
        Resource: ResourceRef{Type: "view", ID: viewID.String()},
    })
    if !allowed { return ViewResult{}, ErrPolicyDenied }

    // 渲染
    renderer, err := v.registry.GetRenderer(view.ViewType)
    if err != nil { return ViewResult{}, err }

    result, err := renderer.Render(ctx, view.Parameters, 1000)
    if err != nil { return ViewResult{}, err }

    // 记调用日志: 含 events.seq watermark (用于精确重建)
    err = v.txRunner.Run(ctx, func(tx Tx) error {
        // 取得当前 events.seq 水位
        var watermark int64
        if err := tx.QueryRow(ctx, "SELECT COALESCE(MAX(seq), 0) FROM events").Scan(&watermark); err != nil {
            return err
        }
        return v.repo.RecordInvocation(tx, view.ID, actor.ID, params, watermark, result, duration)
    })
    return result, err
}
```

**[PROPOSAL]** View 调用日志使"按 seq 重放"成为可能：给定 `event_seq_watermark`，可以重建 View 当时看到的图谱状态。

## 2.9 缓存 / Caching

**[PROPOSAL]** 三级缓存策略：

| 层级 | 用途 | 实现 | 失效时机 |
|---|---|---|---|
| L1 进程内 | 单节点读热点 | LRU（1 万项）| 写时失效 |
| L2 跨进程 | 多实例共享 | Redis (V1+) | 写时失效 + TTL |
| L3 DB | 权威 | PostgreSQL | — |

### 2.9.1 LRU 实现要点

```go
// [IMPL] internal/graph/cache/lru.go
type NodeCache struct {
    cache *lru.Cache[uuid.UUID, *Node]
}

func NewNodeCache(maxEntries int) (*NodeCache, error) {
    c, err := lru.New(maxEntries)
    if err != nil { return nil, err }
    return &NodeCache{cache: c}, nil
}

func (c *NodeCache) Get(id uuid.UUID) (*Node, bool) {
    n, ok := c.cache.Get(id)
    return n, ok
}

func (c *NodeCache) Put(n *Node) {
    c.cache.Add(n.ID, n)
}

func (c *NodeCache) Invalidate(id uuid.UUID) {
    c.cache.Remove(id)
}
```

**[PROPOSAL]** 缓存 key 包含 `deleted_at`，避免软删除后命中旧值。

### 2.9.2 缓存命中监控

```go
// [IMPL]
type CacheMetrics struct {
    Hits   prometheus.Counter
    Misses prometheus.Counter
    Invalidations prometheus.Counter
    Size   prometheus.Gauge
}
```

命中率低于 60% 时告警（说明缓存价值不大，可考虑调整 maxEntries 或淘汰策略）。

## 2.10 一致性保证 / Consistency Guarantees

**[PROPOSAL]**

| 保证 | 实现 |
|---|---|
| 写后读一致 | 写后失效缓存 + 强制从 DB 读 |
| Node + Edge + Event 原子 | 单事务 |
| 跨节点引用完整性 | 外键 + 应用层校验 |
| 不可变事件 | DB role + 无 Update/Delete 方法 |
| 类型安全 | registry 校验 |

**最终一致**（不在此保证）：

- L2 缓存（Redis，V1+）
- Outbox 投递（at-least-once）
- Saga 推进

## 2.11 错误定义

```go
// [IMPL] internal/graph/errors.go
var (
    ErrNodeNotFound           = errors.New("graph: node not found")
    ErrEdgeNotFound           = errors.New("graph: edge not found")
    ErrNodeDeleted            = errors.New("graph: node is deleted")
    ErrAbstractTypeNode       = errors.New("graph: cannot create instance of abstract node type")
    ErrEdgeTypeNotAllowed     = errors.New("graph: edge type not allowed between these node types")
    ErrSelfLoop               = errors.New("graph: self-loop edge not allowed")
    ErrEvidenceMissingEpistemicStatus = errors.New("graph: evidence edge must have epistemic_status")
    ErrPolicyDenied           = errors.New("graph: policy denied")
    ErrDepthLimitExceeded     = errors.New("graph: traversal depth limit exceeded")
    ErrCycleDetected          = errors.New("graph: cycle detected in traversal")
    ErrTypeNotRegistered      = errors.New("graph: node/edge type not in registry")
    ErrSchemaValidation      = errors.New("graph: properties failed schema validation")
)
```

详见 [11 错误处理](11-error-handling.md) 的标准化映射。

## 2.12 测试策略 / Testing Strategy

| 测试 | 目标 |
|---|---|
| 单元 | 类型注册、SQL 构造、缓存逻辑 |
| 集成 (testcontainers) | 真实 PG 下的 CRUD、事务、约束 |
| 端到端 | Phase 9 §6 DoD 第 9 步的图查询 |
| 性能 | k6 跑 4 hop 遍历的 P95/P99 |
| 模糊 | JSON Schema fuzz 测试（随机 props 输入）|

## 2.13 性能预算 / Performance Budget

| 操作 | 目标 | 备注 |
|---|---|---|
| Node 单读 | < 5ms (cache hit) / < 30ms (DB) | LRU 命中率 > 80% 目标 |
| Edge 创建 | < 30ms | 含事务提交 |
| 4 hop 遍历 | < 500ms | 1k 节点结果内 |
| 8 hop 遍历 (硬上限) | < 2s | 超过应物化 |

---

**导航 / Navigation:**
[← 01. 数据层](01-data-layer.md) · [README](README.md) · [03. 策略引擎 →](03-policy-engine.md)
