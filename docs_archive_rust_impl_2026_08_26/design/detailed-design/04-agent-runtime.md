# 04. Agent 运行时 / Agent Runtime

## 4.1 目标 / Purpose

实现 [基本设计 §3.5 Agent Runtime 子系统](../basic-design/03-functional-design.md#35-agent-运行时子系统-agent-runtime-subsystem) 的 7 项 MVP 需求（AGT-REQ-001〜007）和 V1 邻近的 008/009。覆盖：生命周期状态机、隔离工作区、范围受限凭证签发、Policy-gated 执行流、资源上限、消息/状态协议、Agent 作为图谱一级节点。

## 4.2 模块结构

```
internal/agent/
├── lifecycle/
│   ├── state.go              # 状态枚举 + 转换规则
│   ├── statemachine.go       # 状态机
│   ├── controller.go         # 控制器 (事件 → 状态转换)
│   └── lifecycle_test.go
├── workspace/
│   ├── spec.go               # 工作区规格 (镜像、卷、网络、env)
│   ├── docker.go             # Docker Engine API 实现
│   ├── containerd.go         # containerd 实现 (V1+)
│   └── cleanup.go            # 强制清理 (timeout / leak)
├── credential/
│   ├── scope.go              # 范围定义与检查
│   ├── token.go              # 短期 token 签发
│   ├── storage.go            # 凭据存储 (PostgreSQL + 信封加密)
│   └── rotation.go           # 轮换
├── messaging/
│   ├── protocol.go           # 消息协议 (JSON-RPC over stdio/HTTP)
│   ├── router.go             # 消息路由
│   └── status.go             # 状态报告
├── artifact/
│   ├── capture.go            # 产物捕获 (commit, diff, files)
│   └── store.go              # 产物存储 (git objects + graph)
├── resources/
│   ├── limits.go             # 资源限制 (CPU/mem/time/token)
│   ├── monitor.go            # 实时监控
│   └── enforcer.go           # 强制 (超出即 kill)
├── graphlink/
│   ├── writer.go             # Agent Run 写回 graph
│   └── readback.go           # 从 graph 读 Agent 历史
├── approval/
│   ├── gate.go               # 人类审批门
│   └── pending.go            # 待审批管理
├── errors.go
└── types.go                  # 公共类型
```

## 4.3 数据类型

```go
// [IMPL] internal/agent/types.go

type Agent struct {
    ID            uuid.UUID          // Node id (agent)
    Name          string
    Version       string
    Runtime       string             // 'codex','claude-code','cursor','custom'
    Capabilities  []string           // ['code_edit','read_repo','invoke_ci',...]
    DefaultScope  Scope
    CreatedAt     time.Time
}

type AgentRun struct {
    ID            uuid.UUID          // Node id (agent_run)
    AgentID       uuid.UUID
    ParentRunID   *uuid.UUID         // 委派链 (AGT-REQ-009)
    Task          json.RawMessage
    Status        RunStatus
    StartedAt     time.Time
    EndedAt       *time.Time
    Error         string
    ResourceUsage ResourceUsage
    Outputs       []Artifact
}

type RunStatus string
const (
    RunStatusPending    RunStatus = "pending"
    RunStatusApproved   RunStatus = "approved"   // 人类审批通过
    RunStatusSpawning   RunStatus = "spawning"
    RunStatusRunning    RunStatus = "running"
    RunStatusPaused     RunStatus = "paused"
    RunStatusSucceeded  RunStatus = "succeeded"
    RunStatusFailed     RunStatus = "failed"
    RunStatusCancelled  RunStatus = "cancelled"
    RunStatusTimeout    RunStatus = "timeout"
)

type Scope struct {
    AllowedActions map[string][]string  // action -> [resource_type1, ...]
    DeniedActions  map[string][]string
    MCPAllowlist   []string
    RepoAllowlist  []uuid.UUID
    BranchAllowlist []string
    ExpiresAt      *time.Time
}

type ResourceUsage struct {
    CPUSeconds    float64
    MemoryMaxMB   int
    WallSeconds   int
    TokensUsed    int
    CostUSD       float64
}

type Artifact struct {
    Kind        string  // 'commit','pr','file','log','diff'
    Ref         string  // git SHA / file path / pr_id
    CapturedAt  time.Time
    Metadata    json.RawMessage
}
```

## 4.4 状态机

### 4.4.1 状态转换图

```
                  ┌─────────┐
                  │ pending │  (创建后)
                  └────┬────┘
                       │ approval_required (policy decision)
                       ▼
                ┌──────────────┐
                │ needs_approval│
                └────┬─────────┘
                     │ human_approve
                     ▼
              ┌──────────┐
              │ approved │
              └────┬─────┘
                   │ spawn
                   ▼
              ┌──────────┐
              │ spawning │
              └────┬─────┘
                   │ container_ready
                   ▼
              ┌──────────┐         pause
              │ running  │◄────pause────┐
              └─┬──┬──┬─┘              │
                │  │  │                │
    succeed     │  │  │ timeout        │
                │  │  │                │
                ▼  ▼  ▼                │
       ┌────────┐ ┌──────┐ ┌─────────┐ │
       │succeeded│ │failed│ │ timeout │ │
       └────────┘ └──────┘ └─────────┘ │
                                       │
       ┌────────┐                       │
       │cancelled│  (任意时刻可取消)     │
       └────────┘ ─────────────────────►
```

### 4.4.2 状态机实现

```go
// [IMPL] internal/agent/lifecycle/statemachine.go
type StateMachine struct {
    mu sync.Mutex
    run *AgentRun
    listeners []StateChangeListener
}

var validTransitions = map[RunStatus][]RunStatus{
    RunStatusPending:     {RunStatusNeedsApproval, RunStatusApproved, RunStatusCancelled},
    RunStatusNeedsApproval: {RunStatusApproved, RunStatusCancelled, RunStatusFailed},
    RunStatusApproved:    {RunStatusSpawning, RunStatusCancelled, RunStatusFailed},
    RunStatusSpawning:    {RunStatusRunning, RunStatusFailed, RunStatusTimeout},
    RunStatusRunning:     {RunStatusPaused, RunStatusSucceeded, RunStatusFailed, RunStatusTimeout},
    RunStatusPaused:      {RunStatusRunning, RunStatusCancelled, RunStatusFailed},
    // 终态: succeeded/failed/cancelled/timeout 不可再转换
}

func (sm *StateMachine) Transition(to RunStatus) error {
    sm.mu.Lock()
    defer sm.mu.Unlock()

    from := sm.run.Status
    allowed := validTransitions[from]
    if !slices.Contains(allowed, to) {
        return fmt.Errorf("invalid transition: %s -> %s", from, to)
    }
    sm.run.Status = to
    if isTerminal(to) && sm.run.EndedAt == nil {
        t := time.Now()
        sm.run.EndedAt = &t
    }
    for _, l := range sm.listeners { l(sm.run.ID, from, to) }
    return nil
}

func isTerminal(s RunStatus) bool {
    return slices.Contains([]RunStatus{
        RunStatusSucceeded, RunStatusFailed, RunStatusCancelled, RunStatusTimeout,
    }, s)
}
```

## 4.5 生命周期控制器

```go
// [IMPL] internal/agent/lifecycle/controller.go

type Controller struct {
    state     *StateMachine
    workspace Workspace
    cred      *credential.Issuer
    policy    *policy.Engine
    graph     *graphlink.Writer
    audit     *audit.Emitter
}

func (c *Controller) Start(ctx context.Context, req StartRequest) (*AgentRun, error) {
    // 1. 加载 Agent 定义
    agent, err := c.agentRepo.Get(ctx, req.AgentID)
    if err != nil { return nil, err }

    // 2. 创建 AgentRun Node (graph)
    runID := uuid.Must(uuid.NewV7())
    run := &AgentRun{
        ID: runID, AgentID: agent.ID, Task: req.Task,
        Status: RunStatusPending, StartedAt: time.Now(),
    }
    if err := c.graph.CreateAgentRun(ctx, run, req.Invoker); err != nil {
        return nil, err
    }
    sm := NewStateMachine(run)

    // 3. 委派范围检查 (AGT-REQ-005)
    if req.ParentRunID != nil {
        parentScope, _ := c.cred.GetScope(ctx, *req.ParentRunID)
        childScope := effectiveScope(agent, parentScope)
        ok, _ := c.policy.CheckDelegation(ctx, parentScope, childScope)
        if !ok { sm.Transition(RunStatusFailed); return nil, ErrPrivilegeEscalation }
    }

    // 4. Policy 评估: invoke 这个 agent 是否允许
    decision, err := c.policy.Evaluate(ctx, PolicyInput{
        Subject:  req.Invoker,
        Action:   "agent.invoke",
        Resource: ResourceRef{Type: "agent", ID: agent.ID.String()},
    })
    if err != nil { sm.Transition(RunStatusFailed); return nil, err }
    if !decision.Allow {
        if decision.RequiresApproval() {
            sm.Transition(RunStatusNeedsApproval)
            c.approval.Pending(ctx, runID, decision)
            return run, ErrApprovalRequired
        }
        sm.Transition(RunStatusFailed)
        return run, ErrPolicyDenied
    }

    // 5. 签发范围受限凭据
    scope := effectiveScope(agent, req.ParentScope)
    token, err := c.cred.Issue(ctx, runID, scope)
    if err != nil { sm.Transition(RunStatusFailed); return nil, err }

    // 6. 检查是否需要人类审批 (特殊操作)
    if requiresHumanApproval(agent, req.Task) {
        sm.Transition(RunStatusNeedsApproval)
        c.approval.Pending(ctx, runID, decision)
        return run, ErrApprovalRequired
    }

    // 7. 启动工作区 + 进入 spawning
    sm.Transition(RunStatusApproved)
    sm.Transition(RunStatusSpawning)
    ws, err := c.workspace.Spawn(ctx, WorkspaceSpec{
        RunID:     runID,
        AgentID:   agent.ID,
        Image:     agent.RuntimeImage(),
        Resources: req.Resources,
        Token:     token,
    })
    if err != nil { sm.Transition(RunStatusFailed); return nil, err }

    // 8. 启动 agent 进程
    sm.Transition(RunStatusRunning)
    go c.monitorAndCleanup(ctx, sm, ws, req)
    return run, nil
}
```

## 4.6 工作区隔离 / Workspace Isolation

### 4.6.1 工作区规格

```go
// [IMPL] internal/agent/workspace/spec.go
type WorkspaceSpec struct {
    RunID     uuid.UUID
    AgentID   uuid.UUID

    // 镜像
    Image     string                 // 'codex:1.0','claude-code:2.1','custom:latest'
    
    // 资源
    Resources ResourceLimits
    
    // 网络
    Network   NetworkSpec            // loopback / NetworkPolicy
    
    // 存储
    Mounts    []MountSpec
    
    // 环境
    Env       []string               // 注入 PLATFORM_AGENT_TOKEN 等
    
    // 安全
    User      string                 // 非 root
    ReadOnly  bool
    NoNewPrivileges bool
}

type ResourceLimits struct {
    CPUQuota    float64  // 0.5 = 半核
    MemoryMax   string   // '512m','1g'
    PidsMax     int      // 防止 fork bomb
    WallTimeout time.Duration
    TokenLimit  int      // AI token 预算
    CostLimit   float64  // USD
}

type NetworkSpec struct {
    Mode     string   // 'loopback', 'none', 'egress_filtered'
    EgressAllow []string  // 允许的出站目标 (域名白名单)
}

type MountSpec struct {
    Type     string  // 'bind' | 'tmpfs' | 'volume'
    Source   string
    Target   string
    ReadOnly bool
}
```

### 4.6.2 Docker Engine API 实现

```go
// [IMPL] internal/agent/workspace/docker.go

type DockerBackend struct {
    client *docker.Client
}

func (d *DockerBackend) Spawn(ctx context.Context, spec WorkspaceSpec) (Workspace, error) {
    // 1. 资源限制
    hostCfg := &container.HostConfig{
        Resources: container.Resources{
            CPUQuota:  int64(spec.Resources.CPUQuota * 100000),
            Memory:    parseMemory(spec.Resources.MemoryMax),
            PidsLimit: ptr(spec.Resources.PidsMax),
        },
        RestartPolicy: container.RestartPolicy{MaximumRetryCount: 0},
        ReadonlyRootfs: spec.ReadOnly,
        SecurityOpt:   []string{"no-new-privileges"},
        CapDrop:        []string{"ALL"},
        NetworkMode:    networkMode(spec.Network),  // 'none' for air-gapped
    }

    // 2. 卷挂载
    for _, m := range spec.Mounts {
        hostCfg.Mounts = append(hostCfg.Mounts, mount.Mount{
            Type:   mount.Type(m.Type),
            Source: m.Source,
            Target: m.Target,
            ReadOnly: m.ReadOnly,
        })
    }

    // 3. 环境变量
    env := append(spec.Env,
        fmt.Sprintf("PLATFORM_RUN_ID=%s", spec.RunID),
        fmt.Sprintf("PLATFORM_AGENT_TOKEN_FILE=/run/platform/token"),
    )

    // 4. 创建 + 启动
    resp, err := d.client.ContainerCreate(ctx, &container.Config{
        Image: spec.Image,
        Env:   env,
        User:  spec.User,
    }, hostCfg, nil, nil, fmt.Sprintf("agent-run-%s", spec.RunID))
    if err != nil { return nil, err }

    if err := d.client.ContainerStart(ctx, resp.ID, types.ContainerStartOptions{}); err != nil {
        d.client.ContainerRemove(ctx, resp.ID, types.ContainerRemoveOptions{Force: true})
        return nil, err
    }

    return &dockerWorkspace{client: d.client, id: resp.ID, spec: spec}, nil
}
```

### 4.6.3 清理保证

**[PROPOSAL]** 进程崩溃时容器必须被回收。三重保险：

1. **正常退出：** 控制器在 `defer` 中执行清理
2. **超时：** WallTimeout 由 Docker 强制 (`--stop-timeout`)
3. **进程崩溃：** Platform 启动时扫描 `agent-run-*` 命名的孤儿容器并清理

```go
// [IMPL] internal/agent/workspace/cleanup.go
func (w *dockerWorkspace) Cleanup(ctx context.Context) error {
    return w.client.ContainerRemove(ctx, w.id, types.ContainerRemoveOptions{
        Force: true,  // 即使在跑也强制删除
    })
}

func (b *DockerBackend) CleanupOrphans(ctx context.Context) (int, error) {
    containers, err := b.client.ContainerList(ctx, types.ContainerListOptions{
        All: true, Filters: filters.NewArgs(filters.Arg("name", "agent-run-")),
    })
    if err != nil { return 0, err }

    removed := 0
    for _, c := range containers {
        // 仅清理启动时间 > 1h 前的孤儿
        if time.Since(time.Unix(c.Created, 0)) > time.Hour {
            b.client.ContainerRemove(ctx, c.ID, types.ContainerRemoveOptions{Force: true})
            removed++
        }
    }
    return removed, nil
}
```

## 4.7 凭据签发 / Credential Issuance

### 4.7.1 Token 格式

```go
// [IMPL] internal/agent/credential/token.go

type Token struct {
    RunID     uuid.UUID
    Subject   uuid.UUID          // Agent Node id
    Scope     Scope
    IssuedAt  time.Time
    ExpiresAt time.Time
    Signature []byte             // HMAC-SHA256 或 Ed25519
}

func (t Token) Encoded() string {
    // JWT-like 格式: header.payload.signature
    payload := map[string]any{
        "run_id":    t.RunID,
        "subject":   t.Subject,
        "scope":     t.Scope,
        "iat":       t.IssuedAt.Unix(),
        "exp":       t.ExpiresAt.Unix(),
    }
    // ... 编码 + 签名
}
```

**[PROPOSAL]** Token 生命周期：

- **签发：** 启动时
- **TTL：** 默认 1 小时（可配置）
- **续期：** 仅在执行未结束时可续期 1 次
- **撤销：** 立即生效（run 结束时）

### 4.7.2 范围继承

```go
// [IMPL] internal/agent/credential/scope.go
func effectiveScope(agent Agent, parentScope *Scope) Scope {
    base := agent.DefaultScope
    if parentScope == nil {
        return base
    }
    // 子范围必须 ≤ 父范围
    return Scope{
        AllowedActions: intersectMaps(parentScope.AllowedActions, base.AllowedActions),
        DeniedActions:  unionMaps(parentScope.DeniedActions, base.DeniedActions),
        MCPAllowlist:   intersect(parentScope.MCPAllowlist, base.MCPAllowlist),
        RepoAllowlist:  intersect(parentScope.RepoAllowlist, base.RepoAllowlist),
        BranchAllowlist: intersect(parentScope.BranchAllowlist, base.BranchAllowlist),
        ExpiresAt:      minTime(parentScope.ExpiresAt, base.ExpiresAt),
    }
}
```

### 4.7.3 凭据存储

**[PROPOSAL]** 凭据短期（1h）**不**入 PostgreSQL；只在内存中保留（Agent Run 生命周期内）。`credential/token.go` 的 `TokenStore`：

```go
// [IMPL]
type TokenStore struct {
    mu     sync.RWMutex
    tokens map[uuid.UUID]Token  // RunID -> Token
}

func (s *TokenStore) Issue(runID uuid.UUID, scope Scope) Token {
    s.mu.Lock()
    defer s.mu.Unlock()
    t := Token{
        RunID:     runID,
        Subject:   uuid.Must(uuid.NewV7()),
        Scope:     scope,
        IssuedAt:  time.Now(),
        ExpiresAt: time.Now().Add(time.Hour),
    }
    s.tokens[runID] = t
    return t
}
```

长期凭据（如 Agent 的 API key）走 Secrets Store（见 [09 安全实现](09-security-impl.md)），不入 TokenStore。

## 4.8 资源限制与监控

### 4.8.1 强制执行

```go
// [IMPL] internal/agent/resources/enforcer.go
func (e *Enforcer) Enforce(ctx context.Context, run *AgentRun, ws Workspace) {
    timer := time.AfterFunc(run.WallTimeout, func() {
        // WallTimeout 触发, 强制 kill 容器
        ws.Cleanup(ctx)
        e.controller.Fail(run.ID, "wall_timeout")
    })

    ticker := time.NewTicker(5 * time.Second)
    defer ticker.Stop()
    defer timer.Stop()

    for {
        select {
        case <-ctx.Done(): return
        case <-ticker.C:
            stats, err := ws.Stats(ctx)
            if err != nil { continue }

            // CPU 超限
            if stats.CPUPercent > run.ResourceLimits.CPUQuota * 100 {
                e.controller.Pause(run.ID, "cpu_exceeded")
            }
            // 内存超限
            if stats.MemoryMB > run.ResourceLimits.MemoryMaxMB {
                e.controller.Fail(run.ID, "memory_exceeded")
            }
            // Token 超限
            if run.ResourceUsage.TokensUsed > run.ResourceLimits.TokenLimit {
                e.controller.Fail(run.ID, "token_exceeded")
            }
            // 成本超限
            if run.ResourceUsage.CostUSD > run.ResourceLimits.CostLimit {
                e.controller.Fail(run.ID, "cost_exceeded")
            }

            // 报告资源使用 (写 graph)
            e.updateGraph(ctx, run, stats)
        }
    }
}
```

### 4.8.2 实时使用写 graph

**[PROPOSAL]** 资源使用每 30s 写一条 Event（不写 AgentRun.properties，避免写放大）：

```json
{
  "event_type": "agent_run.resource_usage",
  "actor_id": "<agent_node_id>",
  "payload": {
    "run_id": "...",
    "cpu_percent": 45.2,
    "memory_mb": 312,
    "tokens_used": 12453,
    "cost_usd": 0.12
  }
}
```

## 4.9 人类审批门 / Human Approval Gate

```go
// [IMPL] internal/agent/approval/gate.go

type Gate struct {
    db *pgxpool.Pool
    notifier NotificationService
}

func (g *Gate) Pending(ctx context.Context, runID uuid.UUID, reason string) error {
    return g.db.Exec(ctx, `
        INSERT INTO agent_run_approvals (run_id, reason, status, requested_at)
        VALUES ($1, $2, 'pending', now())
    `, runID, reason)
}

func (g *Gate) Approve(ctx context.Context, runID, approverID uuid.UUID, comment string) error {
    tx, err := g.db.Begin(ctx)
    if err != nil { return err }
    defer tx.Rollback(ctx)

    // 1. 更新 approval 记录
    _, err = tx.Exec(ctx, `
        UPDATE agent_run_approvals
        SET status = 'approved', approver_id = $1, comment = $2, decided_at = now()
        WHERE run_id = $3 AND status = 'pending'
    `, approverID, comment, runID)
    if err != nil { return err }

    // 2. 触发 Agent Run 状态从 needs_approval -> approved
    // (通过 EventBus 解耦)
    g.bus.Publish(Event{
        Type:   "agent_run.approved",
        RunID:  runID,
        Actor:  approverID,
    })

    return tx.Commit(ctx)
}

func (g *Gate) Reject(ctx context.Context, runID, approverID uuid.UUID, reason string) error {
    // 类似, 但状态变 cancelled
    // ...
}
```

**[PROPOSAL]** 审批需要时可设置超时（默认 24h），超时后自动 reject。实现：扫表 + cron。

## 4.10 消息/状态协议

**[PROPOSAL]** Agent ↔ Platform 通信用 **JSON-RPC over stdin/stdout**（同进程容器）或 **HTTP**（外部 Agent）。

### 4.10.1 协议

```json
// 请求 (Platform -> Agent)
{"jsonrpc":"2.0","id":1,"method":"task.create","params":{"run_id":"...","task":{...}}}

// 响应 (Agent -> Platform)
{"jsonrpc":"2.0","id":1,"result":{"accepted":true}}

// 通知 (Agent -> Platform, 进度)
{"jsonrpc":"2.0","method":"progress","params":{"run_id":"...","step":"editing_file","percent":42}}
```

### 4.10.2 状态路由

```go
// [IMPL] internal/agent/messaging/router.go
type Router struct {
    agent Agent
    state *StateMachine
    run   *AgentRun
}

func (r *Router) HandleMessage(msg Message) error {
    switch msg.Method {
    case "task.create":
        return r.handleTaskCreate(msg.Params)
    case "progress":
        return r.handleProgress(msg.Params)
    case "complete":
        return r.handleComplete(msg.Params)
    case "error":
        return r.handleError(msg.Params)
    case "request_human_approval":
        return r.handleApprovalRequest(msg.Params)
    case "request_ai_decision":
        return r.handleAIDecision(msg.Params)
    }
    return ErrUnknownMethod
}
```

## 4.11 产物捕获 / Artifact Capture

```go
// [IMPL] internal/agent/artifact/capture.go
func CaptureFromContainer(ctx context.Context, ws Workspace, run *AgentRun) ([]Artifact, error) {
    var artifacts []Artifact

    // 1. 捕获 git diff (从容器挂载的 git repo)
    diff, err := ws.Exec(ctx, []string{"git", "diff", "HEAD"}, "/workspace")
    if err == nil && len(diff) > 0 {
        commit, err := commitDiff(ctx, ws, diff)
        if err == nil {
            artifacts = append(artifacts, Artifact{
                Kind: "commit", Ref: commit.SHA, CapturedAt: time.Now(),
            })
        }
    }

    // 2. 捕获 stdout / stderr
    logs, _ := ws.ReadLogs(ctx)
    artifacts = append(artifacts, Artifact{Kind: "log", Ref: logsPath, ...})

    // 3. 列出变更文件
    files, _ := ws.Exec(ctx, []string{"git", "status", "--porcelain"}, "/workspace")
    for _, f := range parseFiles(files) {
        artifacts = append(artifacts, Artifact{Kind: "file", Ref: f.Path, ...})
    }

    return artifacts, nil
}
```

## 4.12 与 Graph 联动

**[PROPOSAL]** AgentRun 启动时创建 Node + Edge：

```
Agent (Node)  ──spawned──>  AgentRun (Node)
   ↑                          │
   │                          ├─ created_by ──> User (Node)
   │                          ├─ operates_on ──> Repository (Node)
   │                          ├─ uses_credential ──> Scope (在 properties)
   │                          └─ produces ──> Commit / PR / File (Node)
```

每条 Edge 创建走标准 Edge Service（[02 图谱引擎](02-graph-engine.md)）。

## 4.13 错误定义

```go
// [IMPL] internal/agent/errors.go
var (
    ErrApprovalRequired       = NewError("agent_approval_required", 202, "human approval required")
    ErrPrivilegeEscalation    = NewError("agent_privilege_escalation", 403, "delegated scope exceeds parent")
    ErrWorkspaceSpawnFailed   = NewError("workspace_spawn_failed", 500, "failed to spawn agent workspace")
    ErrResourceLimitExceeded  = NewError("agent_resource_limit_exceeded", 429, "agent exceeded resource limit")
    ErrTokenExpired           = NewError("agent_token_expired", 401, "agent token expired")
    ErrInvalidStateTransition = NewError("agent_invalid_state_transition", 500, "invalid state machine transition")
    ErrAgentNotFound          = NewError("agent_not_found", 404, "agent definition not found")
    ErrAgentRunNotFound       = NewError("agent_run_not_found", 404, "agent run not found")
)
```

## 4.14 性能预算

| 指标 | 目标 |
|---|---|
| 启动到 running | < 5s (MVP) |
| 容器清理 | < 30s (超时后) |
| 消息往返 | < 50ms |
| 资源使用写图 | < 100ms |

## 4.15 测试策略

| 测试 | 目标 |
|---|---|
| 单元 | 状态机、scope 计算、token 编解码 |
| 集成 (testcontainers) | 真实 Docker 启动容器 |
| E2E | 启动 → 审批 → 执行 → 产物捕获 |
| 安全 | 容器逃逸检测、scope 旁路检测 |
| 性能 | 并发 Agent 启动延迟、Token 签发 QPS |

---

**导航 / Navigation:**
[← 03. 策略引擎](03-policy-engine.md) · [README](README.md) · [05. AI 网关 →](05-ai-gateway.md)
