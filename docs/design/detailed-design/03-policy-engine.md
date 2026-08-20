# 03. 策略引擎 / Policy Engine

## 3.1 目标 / Purpose

实现 [基本设计 §3.7 安全与访问控制子系统](../basic-design/03-functional-design.md#37-安全与访问控制子系统-security-access-control-subsystem) 的核心决策逻辑：RBAC + ABAC 评估、AI 特有策略强制、决策缓存、失败模式、审计联动。

## 3.2 模块结构

```
internal/policy/
├── engine.go                 # 主入口: Evaluate() 决策
├── rbac/
│   ├── role.go              # 角色解析
│   └── resolver.go          # user -> roles 解析
├── abac/
│   ├── condition.go         # 条件表达式求值
│   ├── attribute.go         # 属性获取 (subject, resource, env)
│   └── cel.go               # CEL 表达式运行时 (备选)
├── aipolicy/                # AI 特有策略
│   ├── prompt_injection.go  # AISEC-REQ-001 标签检查
│   ├── mcp_scope.go         # AISEC-REQ-002 工具白名单
│   ├── secret_filter.go     # AISEC-REQ-004 密钥过滤
│   ├── privilege.go         # AISEC-REQ-005 委派范围
│   ├── policy_bypass.go     # AISEC-REQ-006 旁路检测
│   └── tagger.go            # 上下文标签器
├── cache/
│   ├── decision_cache.go    # 决策结果 LRU
│   └── invalidator.go       # 写时失效
├── audit/
│   └── emitter.go           # 评估结果写 events 表
├── rule/
│   ├── rule.go              # 策略规则 schema
│   ├── parser.go            # JSON/YAML 解析
│   └── builtin.go           # 内置规则
├── errors.go
└── engine_test.go
```

## 3.3 决策模型 / Decision Model

**[PROPOSAL]** 一次 Policy 评估 = 5 步：

```
┌────────────────────────────────────────────────────────────────┐
│                      Evaluate(input)                            │
│                                                                │
│  1. 解析 Subject       → (user_id, roles[], attrs)              │
│  2. 解析 Resource      → (type, id, attrs)                      │
│  3. 解析 Environment   → (ip, time, mfa_level, etc.)            │
│  4. 匹配规则集         → allowed_rules, denied_rules            │
│  5. 决策: deny > allow > default_deny                          │
│                                                                │
│  返回: Decision { Allow bool, Reason string, MatchedRules[] }   │
│  写 Event: policy.evaluated                                   │
└────────────────────────────────────────────────────────────────┘
```

### 3.3.1 输入 schema

```go
// [IMPL]
type PolicyInput struct {
    Subject  Subject
    Action   string                  // 'read','write','delete','merge','invoke',...
    Resource ResourceRef
    Context  map[string]any          // 自由上下文 (e.g., request body, IP)
}

type Subject struct {
    ID       uuid.UUID
    Type     string                  // 'user','agent_node'
    Roles    []string                // RBAC 角色
    Attrs    map[string]any          // ABAC 属性
}

type ResourceRef struct {
    Type string                      // 资源类型 (Node type, "view", "policy", etc.)
    ID   string                      // 资源 ID 或 "*"
}
```

### 3.3.2 输出 schema

```go
// [IMPL]
type Decision struct {
    Allow         bool
    Reason        string                            // 人类可读
    Code          string                            // 标准化 code: "policy_denied", "policy_allowed"
    MatchedRules  []RuleMatch                       // 命中的规则
    EvaluatedAt   time.Time
    EvaluationMs  int
    Source        string                            // 'cache' | 'fresh'
    TraceID       uuid.UUID
}

type RuleMatch struct {
    RuleID   uuid.UUID
    Effect   string                            // 'allow' | 'deny'
    Priority int
}
```

## 3.4 核心实现 / Core Engine

```go
// [IMPL] internal/policy/engine.go

type Engine struct {
    db           *pgxpool.Pool
    roleResolver *rbac.Resolver
    abac         *abac.Evaluator
    cache        *cache.DecisionCache
    audit        *audit.Emitter
    metrics      *Metrics
}

func (e *Engine) Evaluate(ctx context.Context, input PolicyInput) (Decision, error) {
    start := time.Now()
    traceID := uuid.Must(uuid.NewV7())

    // 0. 缓存检查
    cacheKey := buildCacheKey(input)
    if cached, hit := e.cache.Get(cacheKey); hit {
        cached.Source = "cache"
        return cached, nil
    }

    // 1. 加载 subject roles (RBAC)
    if input.Subject.Roles == nil {
        roles, err := e.roleResolver.RolesFor(ctx, input.Subject.ID, input.Subject.Type)
        if err != nil { return Decision{}, err }
        input.Subject.Roles = roles
    }

    // 2. 加载 resource attributes (从 nodes.properties 取)
    if input.Resource.ID != "*" {
        attrs, err := e.loadResourceAttrs(ctx, input.Resource)
        if err != nil { return Decision{}, err }
        input.Resource.Attrs = attrs
    }

    // 3. 加载 environment
    env := buildEnv(ctx)

    // 4. 匹配规则
    allowed, denied, err := e.matchRules(ctx, input, env)
    if err != nil { return Decision{}, err }

    // 5. 决策
    decision := Decision{
        TraceID:      traceID,
        EvaluatedAt:  start,
        EvaluationMs: int(time.Since(start).Milliseconds()),
        Source:       "fresh",
    }
    if len(denied) > 0 {
        decision.Allow = false
        decision.Code = "policy_denied"
        decision.Reason = "denied rule matched"
        decision.MatchedRules = denied
    } else if len(allowed) > 0 {
        decision.Allow = true
        decision.Code = "policy_allowed"
        decision.Reason = "allowed rule matched"
        decision.MatchedRules = allowed
    } else {
        // 默认拒绝 (deny-by-default)
        decision.Allow = false
        decision.Code = "policy_default_deny"
        decision.Reason = "no allow rule matched, default deny"
    }

    // 6. 缓存
    e.cache.Put(cacheKey, decision)

    // 7. 审计: 每次评估都写一条事件
    go e.audit.EmitAsync(ctx, decision, input, traceID)

    // 8. 指标
    e.metrics.RecordEvaluation(decision.Allow, time.Since(start))

    return decision, nil
}
```

## 3.5 规则匹配

### 3.5.1 规则 schema

**[PROPOSAL]**

```yaml
# 规则存储于 policies.rules JSONB 字段
rule_id: 0190e8a4-...
effect: allow                # allow | deny
priority: 100                # 同 effect 内优先级, 数字大者先匹配
description: "developer can write issues in repos they own"

# 匹配条件 (任一不满足即跳过)
match:
  subject:
    type: user                # user | agent_node | *
    role_in: [developer, maintainer]  # RBAC: 角色白名单
    attr:                      # ABAC: 属性条件
      team: backend
  
  action: write               # * 表示通配
  
  resource:
    type: issue               # 资源类型
    id: "*"                   # * 通配, 或具体 ID
    attr:
      state: [open, in_progress]  # 资源属性必须在此列表

  environment:                 # 可选
    ip_range: [10.0.0.0/8]
    time_window: "09:00-18:00 UTC"
    mfa_verified: true
```

### 3.5.2 匹配流程

```go
// [IMPL] internal/policy/engine.go
func (e *Engine) matchRules(ctx context.Context, input PolicyInput, env Env) ([]RuleMatch, []RuleMatch, error) {
    rows, err := e.db.Query(ctx, `
        SELECT id, rules, version
        FROM policies
        WHERE active = true
          AND policy_type = 'access'
        ORDER BY version DESC
    `)
    if err != nil { return nil, nil, err }

    var allowed, denied []RuleMatch
    for rows.Next() {
        var (
            id  uuid.UUID
            raw json.RawMessage
            ver int
        )
        if err := rows.Scan(&id, &raw, &ver); err != nil { return nil, nil, err }

        var rule Rule
        if err := json.Unmarshal(raw, &rule); err != nil { continue }

        if !matchRule(rule, input, env) { continue }

        match := RuleMatch{RuleID: id, Effect: rule.Effect, Priority: rule.Priority}
        if rule.Effect == "allow" {
            allowed = append(allowed, match)
        } else {
            denied = append(denied, match)
        }
    }
    return allowed, denied, nil
}
```

### 3.5.3 匹配函数

```go
// [IMPL] internal/policy/match.go
func matchRule(rule Rule, input PolicyInput, env Env) bool {
    // subject.type
    if rule.Match.Subject.Type != "*" && rule.Match.Subject.Type != input.Subject.Type {
        return false
    }
    // subject.role_in
    if len(rule.Match.Subject.RoleIn) > 0 {
        if !slicesIntersect(rule.Match.Subject.RoleIn, input.Subject.Roles) {
            return false
        }
    }
    // subject.attr (ABAC)
    if !matchAttrs(rule.Match.Subject.Attr, input.Subject.Attrs) {
        return false
    }
    // action
    if rule.Match.Action != "*" && rule.Match.Action != input.Action {
        return false
    }
    // resource
    if rule.Match.Resource.Type != input.Resource.Type {
        return false
    }
    if rule.Match.Resource.ID != "*" && rule.Match.Resource.ID != input.Resource.ID {
        return false
    }
    if !matchAttrs(rule.Match.Resource.Attr, input.Resource.Attrs) {
        return false
    }
    // environment
    if !matchEnv(rule.Match.Environment, env) {
        return false
    }
    return true
}
```

## 3.6 决策缓存

### 3.6.1 缓存 key 与 TTL

```go
// [IMPL] internal/policy/cache/decision_cache.go
type DecisionCache struct {
    cache *lru.Cache[string, Decision]
    ttl   time.Duration       // 默认 60s
}

func buildCacheKey(input PolicyInput) string {
    h := sha256.New()
    h.Write([]byte(input.Subject.ID.String()))
    h.Write([]byte("|"))
    h.Write([]byte(input.Action))
    h.Write([]byte("|"))
    h.Write([]byte(input.Resource.Type))
    h.Write([]byte("|"))
    h.Write([]byte(input.Resource.ID))
    // 不含 env (env 频繁变, 缓存命中率低)
    return base64.StdEncoding.EncodeToString(h.Sum(nil))
}
```

**[PROPOSAL]** 缓存仅包含 `(subject_id, action, resource_type, resource_id)` 四元组。环境属性（IP、时间）每次重评，避免环境绕过缓存。

### 3.6.2 缓存失效

```go
// [IMPL] internal/policy/cache/invalidator.go
type Invalidator struct {
    cache *DecisionCache
    bus  EventBus
}

// 订阅: policy 变更 / role 变更 / permission 变更
func (i *Invalidator) Start(ctx context.Context) {
    sub := i.bus.Subscribe([]string{
        "policy.updated", "role.assigned", "role.revoked",
        "permission.granted", "permission.revoked",
    })
    for event := range sub {
        // 失效所有相关缓存: 按 (subject_id) 或 (resource_id) 前缀清除
        i.cache.InvalidatePrefix(event.SubjectID.String())
    }
}
```

## 3.7 RBAC 角色解析

```go
// [IMPL] internal/policy/rbac/resolver.go
type Resolver struct {
    db *pgxpool.Pool
    cache *lru.Cache[uuid.UUID, []string]
}

func (r *Resolver) RolesFor(ctx context.Context, subjectID uuid.UUID, subjectType string) ([]string, error) {
    if cached, ok := r.cache.Get(subjectID); ok {
        return cached, nil
    }

    // 1. 直接角色
    var roles []string
    rows, err := r.db.Query(ctx, `
        SELECT r.name
        FROM user_roles ur
        JOIN roles r ON r.id = ur.role_id
        WHERE ur.user_id = $1
    `, subjectID)
    if err != nil { return nil, err }
    for rows.Next() {
        var name string
        if err := rows.Scan(&name); err != nil { return nil, err }
        roles = append(roles, name)
    }

    // 2. 继承角色 (层级)
    rows.Close()
    for _, r := range roles {
        parents, _ := r.parentRoles(ctx, r)
        roles = append(roles, parents...)
    }

    r.cache.Add(subjectID, roles)
    return roles, nil
}
```

## 3.8 ABAC 属性求值

**[PROPOSAL]** 简单条件用 JSON 比较；复杂条件用 **CEL (Common Expression Language)** 表达式。

```go
// [IMPL] internal/policy/abac/cel.go
import "github.com/google/cel-go/cel"

type Evaluator struct {
    env *cel.Env
}

func New() (*Evaluator, error) {
    env, err := cel.NewEnv(
        cel.Variable("subject", cel.MapType(cel.StringType, cel.DynType)),
        cel.Variable("resource", cel.MapType(cel.StringType, cel.DynType)),
        cel.Variable("env", cel.MapType(cel.StringType, cel.DynType)),
    )
    return &Evaluator{env: env}, err
}

func (e *Evaluator) Eval(expr string, input PolicyInput, env Env) (bool, error) {
    ast, issues := e.env.Compile(expr)
    if issues != nil && issues.Err() != nil { return false, issues.Err() }
    program, err := e.env.Program(ast)
    if err != nil { return false, err }
    out, _, err := program.Eval(map[string]any{
        "subject":  input.Subject.Attrs,
        "resource": input.Resource.Attrs,
        "env":      env,
    })
    if err != nil { return false, err }
    return out.Value() == true, nil
}
```

## 3.9 AI 特有策略 / AI-Specific Policies

### 3.9.1 提示词注入防御 (AISEC-REQ-001)

**[PROPOSAL]** 任何从外部源（Issue 评论、PR 描述、网页）取得的内容进入 Agent 上下文前必须打标签：

```go
// [IMPL] internal/policy/aipolicy/tagger.go
type TrustLevel string
const (
    TrustTrusted   TrustLevel = "trusted"     // 平台内部, 管理员书写
    TrustUntrusted TrustLevel = "untrusted"   // 来自用户或外部源
)

func TagContext(ctx context.Context, content string, source Source) Context {
    return Context{
        Content: content,
        Trust:   source.Trust,
        Source:  source.URL,
        Tags:    []string{"untrusted"} | nil,
    }
}
```

Agent 上下文中所有 Untrusted 段必须用结构化标签包起来（XML/JSON），并由 prompt template 指示模型不要将其当作指令执行。

### 3.9.2 MCP 工具白名单 (AISEC-REQ-002)

```go
// [IMPL] internal/policy/aipolicy/mcp_scope.go
func (p *MCPScopePolicy) AllowToolCall(ctx context.Context, agentID uuid.UUID, toolName string) (bool, error) {
    // 1. 加载 agent 凭据
    cred, err := p.credStore.Get(agentID)
    if err != nil { return false, err }

    // 2. 检查 tool 是否在 allowlist
    allowed := slices.Contains(cred.AllowedMCPTools, toolName)
    return allowed, nil
}
```

**AISEC-REQ-003 (V1) — Tool 描述完整性：**

```go
// [IMPL] 内部: tool 描述变化检测
func (p *MCPScopePolicy) CheckToolSchemaChanged(ctx context.Context, toolName, currentSchema string) (bool, error) {
    last, err := p.repo.GetApprovedSchema(toolName)
    if err != nil { return false, err }
    if last != currentSchema {
        return true, nil  // 描述变了, 需重审批
    }
    return false, nil
}
```

### 3.9.3 密钥过滤 (AISEC-REQ-004)

```go
// [IMPL] internal/policy/aipolicy/secret_filter.go
func FilterSecrets(ctx context.Context, props json.RawMessage) json.RawMessage {
    var p map[string]any
    if err := json.Unmarshal(props, &p); err != nil { return props }
    
    // 遍历 properties, 凡是标记为 secret 的字段, 替换为 "[REDACTED]"
    walkMap(p, func(key string, value any) (any, bool) {
        if isSecretField(key) {
            return "[REDACTED]", true
        }
        return value, false
    })
    
    out, _ := json.Marshal(p)
    return out
}

func isSecretField(key string) bool {
    return slices.Contains([]string{
        "api_key", "password", "token", "private_key", "ssh_key",
        "secret", "credential", "auth", "access_token",
    }, strings.ToLower(key))
}
```

### 3.9.4 委派范围包含 (AISEC-REQ-005)

**[PROPOSAL]** 子 Agent 的有效 scope 必须 ≤ 父 Agent 的 scope。

```go
// [IMPL] internal/policy/aipolicy/privilege.go
func (p *PrivilegePolicy) CheckDelegation(ctx context.Context, parentScope, childScope Scope) (bool, error) {
    // child 的每个允许项必须在 parent 中存在
    for action, resources := range childScope.Allowed {
        for resource := range resources {
            if !parentScope.IsAllowed(action, resource) {
                return false, nil
            }
        }
    }
    // child 的每个 deny 项必须与 parent 一致或更严
    for action, resources := range childScope.Denied {
        for resource := range resources {
            if !parentScope.IsDenied(action, resource) {
                return false, nil
            }
        }
    }
    return true, nil
}
```

### 3.9.5 Policy 旁路检测 (AISEC-REQ-006)

**[PROPOSAL]** 所有"由 Agent 生成的代码 / 配置"在落地前**必须**走一次 Policy 重评；不允许"Agent 自审"。

```go
// [IMPL] internal/policy/aipolicy/policy_bypass.go
// 在 commit hook / merge handler 中调用
func (e *Engine) EnforceAgentOutputReview(ctx context.Context, diff *GitDiff) error {
    // 即使是 agent run 生成的 commit, 仍需重走完整 Policy 评估
    for _, file := range diff.Files {
        if file.IsConfig() || file.IsCode() {
            decision, err := e.Evaluate(ctx, PolicyInput{
                Subject:  currentActor,        // 可能是 Agent
                Action:   "merge",
                Resource: ResourceRef{Type: "pr", ID: file.PRID},
            })
            if err != nil { return err }
            if !decision.Allow { return ErrPolicyDenied }
        }
    }
    return nil
}
```

## 3.10 失败模式 / Failure Modes

| 场景 | 行为 |
|---|---|
| Policy 表无数据 | 默认 deny（fail-closed）|
| 规则 JSON 解析失败 | 跳过该规则，记录错误，**不影响其他规则评估** |
| DB 查询超时 | 返回 `policy_evaluation_timeout` 错误，**不缓存** |
| ABAC 表达式求值失败 | 跳过该规则，**不影响其他规则** |
| 缓存 miss | 走完整评估，写缓存 |
| 审计写失败 | 评估结果仍返回，**异步重试审计** |

**[PROPOSAL]** **Fail-closed**：任何评估失败都默认拒绝，调用方需显示处理。

## 3.11 审计联动

```go
// [IMPL] internal/policy/audit/emitter.go
func (e *Emitter) EmitAsync(ctx context.Context, d Decision, input PolicyInput, traceID uuid.UUID) {
    payload := map[string]any{
        "decision":      d.Code,
        "subject_id":    input.Subject.ID,
        "subject_type":  input.Subject.Type,
        "action":        input.Action,
        "resource":      input.Resource,
        "matched_rules": d.MatchedRules,
        "evaluation_ms": d.EvaluationMs,
        "source":        d.Source,
        "trace_id":      traceID,
    }

    // 异步发, 失败重试
    go func() {
        if err := e.retryWithBackoff(func() error {
            return e.eventWriter.Append(ctx, Event{
                SubjectNodeID: &input.Resource.IDNodeUUID(), // 尽力转换
                EventType:     "policy.evaluated",
                ActorID:       &input.Subject.ID,
                Payload:       marshal(payload),
            })
        }, 3); err != nil {
            e.metrics.AuditFailures.Inc()
            log.Error("policy audit emit failed", "err", err)
        }
    }()
}
```

## 3.12 性能预算

| 指标 | 目标 | 备注 |
|---|---|---|
| 评估 (cache hit) | < 1ms | LRU 命中 |
| 评估 (cache miss) | < 5ms (P95) | 1k 规则集 |
| 评估 (DB 查询) | < 20ms (P95) | 含 1k 规则扫描 |
| 缓存命中率 | > 80% | 监控告警 < 60% |
| 决策写审计延迟 | < 100ms (P99) | 异步重试 |

## 3.13 错误定义

```go
// [IMPL] internal/policy/errors.go
var (
    ErrPolicyDenied            = NewError("policy_denied", 403, "policy denied")
    ErrPolicyEvaluationTimeout = NewError("policy_evaluation_timeout", 503, "policy evaluation timeout")
    ErrPolicyInvalidRule       = NewError("policy_invalid_rule", 500, "policy rule is invalid")
    ErrPolicyCacheUnavailable  = NewError("policy_cache_unavailable", 503, "policy cache unavailable, evaluating fresh")
    ErrAgentScopeNotAllowed    = NewError("agent_scope_not_allowed", 403, "agent tool call not in allowlist")
    ErrAgentPrivilegeEscalation = NewError("agent_privilege_escalation", 403, "delegated scope exceeds parent")
    ErrSecretLeakagePrevented  = NewError("secret_leakage_prevented", 400, "secret value redacted from request")
    ErrEvidenceMissingEpistemicStatus = NewError("evidence_missing_epistemic_status", 400, "evidence edge requires epistemic_status")
)
```

## 3.14 与 Graph Engine 的协作

**[PROPOSAL]** Node 创建时调用 Policy：

```
Graph Node Service.Create
    ↓
Policy Engine.Evaluate(subject=actor, action=create, resource=type)
    ↓ (Allow)
Node + Event 写入
    ↓
Policy 决策 → 审计 Event
```

详见 [02 图谱引擎](02-graph-engine.md) §2.4.3。

## 3.15 测试策略

| 测试 | 目标 |
|---|---|
| 单元 | 规则匹配、条件求值、缓存 key |
| 集成 | DB 真实查询、CEL 表达式 |
| 性能 | k6 模拟 100 RPS 评估 |
| 安全 | OWASP ABAC 漏洞测试（注入条件表达式）|
| 模糊 | 随机 PolicyInput fuzz |

---

**导航 / Navigation:**
[← 02. 图谱引擎](02-graph-engine.md) · [README](README.md) · [04. Agent 运行时 →](04-agent-runtime.md)
