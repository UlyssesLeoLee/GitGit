# 08. API 处理器 / API Handlers

## 8.1 目标 / Purpose

实现 [基本设计 §11 API 设计](../basic-design/11-api-design.md) 的全部协议栈处理器层。覆盖：HTTP REST 路由、middleware 链、MCP tool 实现、CLI 子命令映射、错误响应格式、OpenAPI 同步。

## 8.2 模块结构

```
internal/api/
├── http/
│   ├── router.go              # 路由表
│   ├── server.go              # HTTP server 启动/优雅关闭
│   ├── middleware/
│   │   ├── request_id.go
│   │   ├── trace.go
│   │   ├── auth.go
│   │   ├── rate_limit.go
│   │   ├── idempotency.go
│   │   ├── cors.go
│   │   ├── recovery.go
│   │   ├── logger.go
│   │   └── openapi_validator.go
│   ├── handlers/
│   │   ├── repos.go
│   │   ├── issues.go
│   │   ├── pulls.go
│   │   ├── graph.go
│   │   ├── agents.go
│   │   ├── policies.go
│   │   ├── audit.go
│   │   ├── views.go
│   │   ├── webhooks.go
│   │   ├── healthz.go
│   │   ├── coordinate.go      # 存储过程 RPC
│   │   └── internal/
│   │       └── git_hook.go
│   ├── response.go            # 统一响应/错误格式
│   └── openapi.go             # OpenAPI 3.1 治理
├── mcp/
│   ├── server.go              # MCP server
│   ├── tools/
│   │   ├── read_node.go
│   │   ├── traverse_graph.go
│   │   ├── issue_ops.go
│   │   ├── pr_ops.go
│   │   ├── repo_ops.go
│   │   ├── invoke_agent.go
│   │   ├── get_view.go
│   │   └── search.go
│   └── schema.go              # tool schema 治理
├── cli/
│   ├── cmd/
│   │   ├── root.go
│   │   ├── repo.go
│   │   ├── issue.go
│   │   ├── agent.go
│   │   ├── audit.go
│   │   ├── export.go
│   │   ├── import.go
│   │   ├── webhooks.go
│   │   └── platform.go
│   └── output.go              # JSON / 人类可读输出
├── auth/
│   ├── basic.go               # HTTP Basic
│   ├── ssh.go                 # SSH (V1)
│   └── oidc.go                # OIDC (V1)
├── pagination/
│   └── cursor.go              # cursor-based 分页
├── idempotency/
│   └── store.go               # Idempotency-Key 实现
├── response/
│   └── error.go               # 标准化错误响应
├── webhook/
│   ├── sender.go              # 出站 webhook
│   └── signer.go              # HMAC 签名
└── errors.go
```

## 8.3 路由表

```go
// [IMPL] internal/api/http/router.go

func RegisterRoutes(r *Router, h *Handlers, m *Middleware) {
    // ===== 公共 =====
    r.GET("/api/v1/healthz", m.Public, h.Healthz.Liveness)
    r.GET("/api/v1/readyz",   m.Public, h.Healthz.Readiness)
    r.GET("/api/v1/openapi.json", m.Public, h.OpenAPI.Spec)

    // ===== 仓库 =====
    r.GET("/api/v1/repos",                m.Auth, m.RateLimit, h.Repos.List)
    r.POST("/api/v1/repos",               m.Auth, m.RateLimit, m.Idempotency, h.Repos.Create)
    r.GET("/api/v1/repos/{id}",           m.Auth, m.RateLimit, h.Repos.Get)
    r.PATCH("/api/v1/repos/{id}",         m.Auth, m.RateLimit, h.Repos.Update)
    r.DELETE("/api/v1/repos/{id}",        m.Auth, m.RateLimit, h.Repos.Delete)

    // ===== Issue =====
    r.GET("/api/v1/repos/{id}/issues",         m.Auth, m.RateLimit, h.Issues.List)
    r.POST("/api/v1/repos/{id}/issues",        m.Auth, m.RateLimit, m.Idempotency, h.Issues.Create)
    r.GET("/api/v1/repos/{id}/issues/{iid}",   m.Auth, m.RateLimit, h.Issues.Get)
    r.PATCH("/api/v1/repos/{id}/issues/{iid}", m.Auth, m.RateLimit, h.Issues.Update)
    r.POST("/api/v1/repos/{id}/issues/{iid}/transitions", m.Auth, m.RateLimit, h.Issues.Transition)

    // ===== PR =====
    r.GET("/api/v1/repos/{id}/pulls",               m.Auth, m.RateLimit, h.Pulls.List)
    r.POST("/api/v1/repos/{id}/pulls",              m.Auth, m.RateLimit, m.Idempotency, h.Pulls.Create)
    r.GET("/api/v1/repos/{id}/pulls/{pid}",         m.Auth, m.RateLimit, h.Pulls.Get)
    r.POST("/api/v1/repos/{id}/pulls/{pid}/merge",  m.Auth, m.RateLimit, m.Idempotency, h.Pulls.Merge)

    // ===== Graph =====
    r.GET("/api/v1/graph/nodes/{id}",     m.Auth, m.RateLimit, h.Graph.GetNode)
    r.POST("/api/v1/graph/nodes",         m.Auth, m.RateLimit, m.Idempotency, h.Graph.CreateNode)
    r.GET("/api/v1/graph/traverse",       m.Auth, m.RateLimit, h.Graph.Traverse)

    // ===== Agent =====
    r.GET("/api/v1/agents",                  m.Auth, m.RateLimit, h.Agents.List)
    r.POST("/api/v1/agents",                 m.Auth, m.RateLimit, m.Idempotency, h.Agents.Create)
    r.POST("/api/v1/agents/{id}/runs",       m.Auth, m.RateLimit, m.Idempotency, h.Agents.Run)
    r.GET("/api/v1/agent-runs/{id}",         m.Auth, m.RateLimit, h.Agents.GetRun)
    r.POST("/api/v1/agent-runs/{id}/cancel", m.Auth, m.RateLimit, h.Agents.Cancel)
    r.POST("/api/v1/agent-runs/{id}/approve", m.Auth, m.RateLimit, h.Agents.Approve)

    // ===== Policy =====
    r.GET("/api/v1/policies",     m.Auth, m.RateLimit, h.Policies.List)
    r.POST("/api/v1/policies",    m.Auth, m.RateLimit, h.Policies.Create)
    r.PATCH("/api/v1/policies/{id}", m.Auth, m.RateLimit, h.Policies.Update)

    // ===== Audit =====
    r.GET("/api/v1/audit/events", m.Auth, m.RateLimit, m.AuditOnly, h.Audit.Search)

    // ===== View =====
    r.POST("/api/v1/views/{id}/invoke", m.Auth, m.RateLimit, m.Idempotency, h.Views.Invoke)

    // ===== Webhook =====
    r.POST("/api/v1/webhooks",       m.Auth, m.RateLimit, h.Webhooks.Create)
    r.GET("/api/v1/webhooks",        m.Auth, m.RateLimit, h.Webhooks.List)
    r.DELETE("/api/v1/webhooks/{id}", m.Auth, m.RateLimit, h.Webhooks.Delete)

    // ===== Coordinate (存储过程 RPC) =====
    r.POST("/api/v1/coordinate/{proc_name}", m.Auth, m.RateLimit, m.Idempotency, h.Coordinate.Call)

    // ===== MCP =====
    r.Mount("/mcp/v1", mcp.NewServer(...))

    // ===== 内部 (仅 loopback / mTLS) =====
    r.POST("/internal/git/hook/post-receive", m.InternalOnly, h.Internal.GitHook.PostReceive)
    r.POST("/internal/coord/proc-invoked",    m.InternalOnly, h.Internal.Coord.ProcInvoked)
}
```

## 8.4 Middleware 链

### 8.4.1 执行顺序

```
Request
  ↓
1. Recovery (panic → 500)
  ↓
2. Request ID (X-Request-Id)
  ↓
3. Trace (X-Trace-Id)
  ↓
4. Logger (请求开始/结束日志)
  ↓
5. CORS (仅 Web UI 路径)
  ↓
6. Rate Limit (per IP / per user / per agent)
  ↓
7. Idempotency (写操作)
  ↓
8. Auth (Basic / Bearer / SSH)
  ↓
9. Audit (写操作前)
  ↓
10. OpenAPI Schema 校验 (仅 POST/PUT/PATCH)
  ↓
11. Handler
  ↓
12. Response Formatter
```

### 8.4.2 Request ID

```go
// [IMPL] internal/api/http/middleware/request_id.go
func RequestID(next http.Handler) http.Handler {
    return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
        id := r.Header.Get("X-Request-Id")
        if id == "" {
            id = uuid.Must(uuid.NewV7()).String()
        }
        w.Header().Set("X-Request-Id", id)
        ctx := context.WithValue(r.Context(), ctxKeyRequestID, id)
        next.ServeHTTP(w, r.WithContext(ctx))
    })
}
```

### 8.4.3 认证

```go
// [IMPL] internal/api/http/middleware/auth.go
type AuthMiddleware struct {
    basicAuth *auth.BasicAuthenticator
    oidcAuth  *auth.OIDCAuthenticator     // V1
}

func (a *AuthMiddleware) Authenticate(next http.Handler) http.Handler {
    return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
        actor, err := a.authenticate(r)
        if err != nil {
            a.writeError(w, 401, "auth_missing_credentials", "authentication required")
            return
        }
        ctx := context.WithValue(r.Context(), ctxKeyActor, actor)
        next.ServeHTTP(w, r.WithContext(ctx))
    })
}

func (a *AuthMiddleware) authenticate(r *http.Request) (Actor, error) {
    auth := r.Header.Get("Authorization")
    if strings.HasPrefix(auth, "Basic ") {
        return a.basicAuth.Parse(auth[6:])
    }
    if strings.HasPrefix(auth, "Bearer ") {
        return a.oidcAuth.Parse(auth[7:])  // V1
    }
    return Actor{}, ErrUnauthorized
}
```

### 8.4.4 Idempotency

```go
// [IMPL] internal/idempotency/store.go
type Store struct {
    db *pgxpool.Pool
    ttl time.Duration  // 默认 24h
}

type IdempotencyRecord struct {
    Key         string
    Endpoint    string
    RequestHash string
    Status      int
    ResponseBody []byte
    ExpiresAt   time.Time
}

func (s *Store) Check(ctx context.Context, key, endpoint, reqHash string) (*IdempotencyRecord, bool, error) {
    // 1. 查记录
    var rec IdempotencyRecord
    err := s.db.QueryRow(ctx, `
        SELECT key, endpoint, request_hash, status, response_body, expires_at
        FROM idempotency_keys
        WHERE key = $1 AND expires_at > now()
    `, key).Scan(&rec.Key, &rec.Endpoint, &rec.RequestHash, &rec.Status, &rec.ResponseBody, &rec.ExpiresAt)
    if err != nil {
        if errors.Is(err, pgx.ErrNoRows) { return nil, false, nil }
        return nil, false, err
    }

    // 2. 同 key + 同 endpoint + 同 hash -> 缓存命中
    if rec.Endpoint == endpoint && rec.RequestHash == reqHash {
        return &rec, true, nil
    }
    // 同 key + 不同 hash -> 冲突
    return nil, false, ErrIdempotencyKeyConflict
}

func (s *Store) Save(ctx context.Context, rec IdempotencyRecord) error {
    _, err := s.db.Exec(ctx, `
        INSERT INTO idempotency_keys (key, endpoint, request_hash, status, response_body, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (key) DO NOTHING
    `, rec.Key, rec.Endpoint, rec.RequestHash, rec.Status, rec.ResponseBody, rec.ExpiresAt)
    return err
}
```

Middleware 用法：

```go
// [IMPL] internal/api/http/middleware/idempotency.go
func Idempotency(store *idempotency.Store) func(http.Handler) http.Handler {
    return func(next http.Handler) http.Handler {
        return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
            if r.Method == "GET" || r.Method == "HEAD" {
                next.ServeHTTP(w, r); return
            }
            key := r.Header.Get("Idempotency-Key")
            if key == "" { next.ServeHTTP(w, r); return }

            body, _ := io.ReadAll(r.Body)
            r.Body = io.NopCloser(bytes.NewReader(body))
            reqHash := sha256Hex(body)
            endpoint := r.Method + " " + r.URL.Path

            rec, hit, err := store.Check(r.Context(), key, endpoint, reqHash)
            if err != nil {
                if errors.Is(err, ErrIdempotencyKeyConflict) {
                    writeError(w, 422, "idempotency_key_conflict", "key reused with different payload")
                    return
                }
                // 错误 -> 拒绝请求 (fail-closed)
                writeError(w, 503, "idempotency_unavailable", err.Error())
                return
            }
            if hit {
                // 重放: 直接返回缓存响应
                w.Header().Set("Content-Type", "application/json")
                w.Header().Set("X-Idempotent-Replay", "true")
                w.WriteHeader(rec.Status)
                w.Write(rec.ResponseBody)
                return
            }

            // 包装 ResponseWriter 以捕获响应
            recw := &recordingWriter{ResponseWriter: w, body: &bytes.Buffer{}}
            next.ServeHTTP(recw, r)

            // 保存
            store.Save(r.Context(), IdempotencyRecord{
                Key: key, Endpoint: endpoint, RequestHash: reqHash,
                Status: recw.status, ResponseBody: recw.body.Bytes(),
                ExpiresAt: time.Now().Add(24 * time.Hour),
            })
        })
    }
}
```

### 8.4.5 Rate Limit

```go
// [IMPL] internal/api/http/middleware/rate_limit.go
type Limiter struct {
    store *RateLimitStore  // Redis (V1) 或 in-memory (MVP)
    perUserPerMin  int
    perAgentPerMin int
    perIPPerMin    int
}

func (l *Limiter) Limit(next http.Handler) http.Handler {
    return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
        actor, _ := ActorFromContext(r.Context())
        key := l.keyFor(r, actor)
        if !l.allow(key) {
            w.Header().Set("Retry-After", "60")
            w.Header().Set("X-RateLimit-Limit", "1000")
            w.Header().Set("X-RateLimit-Remaining", "0")
            writeError(w, 429, "rate_limit_exceeded", "rate limit exceeded")
            return
        }
        next.ServeHTTP(w, r)
    })
}
```

## 8.5 错误响应格式

```go
// [IMPL] internal/api/response/error.go
type ErrorResponse struct {
    Error ErrorPayload `json:"error"`
}

type ErrorPayload struct {
    Code            string         `json:"code"`
    Message         string         `json:"message"`
    Details         map[string]any `json:"details,omitempty"`
    TraceID         string         `json:"trace_id"`
    DocumentationURL string        `json:"documentation_url,omitempty"`
}

func WriteError(w http.ResponseWriter, status int, code, message string, details map[string]any) {
    traceID := traceIDFromContext(...)
    payload := ErrorResponse{
        Error: ErrorPayload{
            Code: code, Message: message, Details: details,
            TraceID: traceID,
            DocumentationURL: fmt.Sprintf("https://docs.example.com/errors/%s", code),
        },
    }
    w.Header().Set("Content-Type", "application/json")
    w.WriteHeader(status)
    json.NewEncoder(w).Encode(payload)
}
```

## 8.6 Handler 通用结构

```go
// [IMPL] internal/api/http/handlers/repos.go
type ReposHandler struct {
    repoRepo    *repository.RepoRepository
    graph       *graphlink.Writer
    policy      *policy.Engine
    eventLog    *audit.Emitter
}

func (h *ReposHandler) Create(w http.ResponseWriter, r *http.Request) {
    var req CreateRepoRequest
    if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
        WriteError(w, 400, "validation_invalid_json", err.Error(), nil)
        return
    }
    if err := validate.Struct(&req); err != nil {
        WriteError(w, 400, "validation_required_field_missing", err.Error(), nil)
        return
    }

    actor, _ := ActorFromContext(r.Context())

    // 1. Policy 评估
    allowed, err := h.policy.Evaluate(r.Context(), PolicyInput{
        Subject: actor, Action: "repo.create",
        Resource: ResourceRef{Type: "organization", ID: req.OrgID},
    })
    if err != nil { WriteError(w, 503, "policy_evaluation_timeout", err.Error(), nil); return }
    if !allowed { WriteError(w, 403, "policy_denied", "not allowed to create repo in this org", nil); return }

    // 2. 调用服务
    repo, err := h.repoRepo.Create(r.Context(), req)
    if err != nil {
        switch {
        case errors.Is(err, repository.ErrAlreadyExists):
            WriteError(w, 409, "resource_already_exists", err.Error(), nil)
        default:
            WriteError(w, 500, "internal_error", err.Error(), nil)
        }
        return
    }

    // 3. 写 Event
    h.eventLog.EmitAsync(r.Context(), "repo.created", actor, repo)

    // 4. 响应
    w.Header().Set("Location", fmt.Sprintf("/api/v1/repos/%s", repo.ID))
    w.WriteHeader(201)
    json.NewEncoder(w).Encode(repo)
}
```

## 8.7 Pagination

```go
// [IMPL] internal/pagination/cursor.go

// Cursor 是 base64 编码的 JSON: {"sort":"created_at","order":"desc","value":"2026-08-19T..."}
type Cursor struct {
    Sort  string
    Order string  // 'asc' | 'desc'
    Value string
}

func Encode(c Cursor) string {
    data, _ := json.Marshal(c)
    return base64.URLEncoding.EncodeToString(data)
}

func Decode(s string) (Cursor, error) {
    data, err := base64.URLEncoding.DecodeString(s)
    if err != nil { return Cursor{}, err }
    var c Cursor
    return c, json.Unmarshal(data, &c)
}

type Page[T any] struct {
    Data       []T    `json:"data"`
    Pagination struct {
        NextCursor string `json:"next_cursor"`
        HasMore    bool   `json:"has_more"`
    } `json:"pagination"`
}
```

## 8.8 MCP Tool 实现

```go
// [IMPL] internal/api/mcp/server.go
type Server struct {
    tools   map[string]ToolHandler
    policy  *policy.Engine
    graph   *graph.Engine
    agents  *agent.Controller
}

type ToolHandler func(ctx context.Context, params json.RawMessage) (json.RawMessage, error)

func NewServer(p *policy.Engine, g *graph.Engine, a *agent.Controller) *Server {
    s := &Server{
        tools:  map[string]ToolHandler{},
        policy: p, graph: g, agents: a,
    }
    s.register("read_node", s.toolReadNode)
    s.register("traverse_graph", s.toolTraverseGraph)
    s.register("list_issues", s.toolListIssues)
    s.register("create_issue", s.toolCreateIssue)
    s.register("read_pr", s.toolReadPR)
    s.register("create_pr", s.toolCreatePR)
    s.register("request_review", s.toolRequestReview)
    s.register("read_repo", s.toolReadRepo)
    s.register("list_repos", s.toolListRepos)
    s.register("invoke_agent", s.toolInvokeAgent)
    s.register("get_view", s.toolGetView)
    s.register("search", s.toolSearch)
    return s
}

func (s *Server) register(name string, h ToolHandler) {
    s.tools[name] = h
}

// ServeHTTP 实现 MCP over HTTP/JSON-RPC
func (s *Server) ServeHTTP(w http.ResponseWriter, r *http.Request) {
    var req struct {
        JSONRPC string          `json:"jsonrpc"`
        ID      json.RawMessage  `json:"id"`
        Method  string           `json:"method"`
        Params  json.RawMessage  `json:"params"`
    }
    if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
        writeMCPError(w, nil, -32700, "parse error", nil)
        return
    }

    actor := actorFromMCPAuth(r)

    switch req.Method {
    case "tools/list":
        s.handleList(w, req.ID)
    case "tools/call":
        s.handleCall(w, r.Context(), req.ID, req.Params, actor)
    default:
        writeMCPError(w, req.ID, -32601, "method not found", nil)
    }
}

func (s *Server) handleCall(w http.ResponseWriter, ctx context.Context, id json.RawMessage, params json.RawMessage, actor Actor) {
    var p struct {
        Name      string          `json:"name"`
        Arguments json.RawMessage `json:"arguments"`
    }
    if err := json.Unmarshal(params, &p); err != nil {
        writeMCPError(w, id, -32602, "invalid params", nil)
        return
    }

    handler, ok := s.tools[p.Name]
    if !ok {
        writeMCPError(w, id, -32601, "unknown tool", nil)
        return
    }

    // 工具调用前 Policy 评估
    allowed, err := s.policy.Evaluate(ctx, PolicyInput{
        Subject: actor,
        Action: deriveActionFromTool(p.Name),
        Resource: ResourceRef{Type: "tool", ID: p.Name},
    })
    if err != nil { writeMCPError(w, id, -32000, "policy error: "+err.Error(), nil); return }
    if !allowed { writeMCPError(w, id, -32001, "policy denied", nil); return }

    result, err := handler(ctx, p.Arguments)
    if err != nil {
        writeMCPError(w, id, -32002, "tool error: "+err.Error(), nil)
        return
    }

    writeMCPResult(w, id, map[string]any{
        "content": []map[string]any{
            {"type": "json", "data": result},
        },
    })
}
```

**单个 tool 示例：**

```go
// [IMPL] internal/api/mcp/tools/read_node.go
func (s *Server) toolReadNode(ctx context.Context, params json.RawMessage) (json.RawMessage, error) {
    var p struct {
        NodeID uuid.UUID `json:"node_id"`
    }
    if err := json.Unmarshal(params, &p); err != nil { return nil, err }

    node, err := s.graph.GetNode(ctx, p.NodeID)
    if err != nil { return nil, err }

    return json.Marshal(node)
}
```

## 8.9 CLI 子命令

```go
// [IMPL] internal/api/cli/cmd/root.go
var rootCmd = &cobra.Command{
    Use:   "platform",
    Short: "AI-Native Engineering Platform CLI",
    PersistentPreRun: func(cmd *cobra.Command, args []string) {
        // 加载配置
        // 验证 auth
    },
}

func init() {
    rootCmd.PersistentFlags().StringP("config", "c", "~/.platform/config.yaml", "config file")
    rootCmd.PersistentFlags().StringP("output", "o", "human", "output format: human|json")
    rootCmd.PersistentFlags().BoolP("quiet", "q", false, "suppress non-error output")
}

func Execute() error {
    return rootCmd.Execute()
}

// 子命令注册
func init() {
    rootCmd.AddCommand(repoCmd)
    rootCmd.AddCommand(issueCmd)
    rootCmd.AddCommand(agentCmd)
    rootCmd.AddCommand(auditCmd)
    rootCmd.AddCommand(exportCmd)
    rootCmd.AddCommand(importCmd)
    rootCmd.AddCommand(webhookCmd)
}
```

**示例：repo create**

```go
// [IMPL] internal/api/cli/cmd/repo.go
var repoCmd = &cobra.Command{Use: "repo", Short: "manage repositories"}

var repoCreateCmd = &cobra.Command{
    Use:   "create --name=<name>",
    Short: "create a new repository",
    RunE: func(cmd *cobra.Command, args []string) error {
        name, _ := cmd.Flags().GetString("name")
        visibility, _ := cmd.Flags().GetString("visibility")
        orgID, _ := cmd.Flags().GetString("org")

        // 调 HTTP API (CLI 与 HTTP 共用后端)
        client := apiClient()
        body := map[string]any{"name": name, "visibility": visibility, "org_id": orgID}
        resp, err := client.Post("/api/v1/repos", body)
        if err != nil { return err }

        if outputFormat() == "json" {
            fmt.Println(string(resp))
        } else {
            var repo struct{ ID, Name string }
            json.Unmarshal(resp, &repo)
            fmt.Printf("Created repository %s (id: %s)\n", repo.Name, repo.ID)
        }
        return nil
    },
}

func init() {
    repoCreateCmd.Flags().String("name", "", "repository name (required)")
    repoCreateCmd.Flags().String("visibility", "private", "public|private|internal")
    repoCreateCmd.Flags().String("org", "", "organization ID (required)")
    repoCreateCmd.MarkFlagRequired("name")
    repoCreateCmd.MarkFlagRequired("org")
    repoCmd.AddCommand(repoCreateCmd)
}
```

## 8.10 OpenAPI 治理

```go
// [IMPL] internal/api/http/openapi.go
// 启动时加载 OpenAPI YAML, 启动时 + 写操作前 校验
func (s *Server) LoadOpenAPI(path string) error {
    data, err := os.ReadFile(path)
    if err != nil { return err }
    s.openapi, err = openapi3.NewLoader().LoadFromData(data)
    return err
}

// 路由表与 OpenAPI 同步:
// - 启动时校验每个注册的 path 在 OpenAPI 中存在
// - 每个 handler 的 request/response schema 来自 OpenAPI
// 缺失则 fail-fast
```

## 8.11 错误定义

```go
// [IMPL] internal/api/errors.go
var (
    ErrUnauthorized            = NewError("auth_missing_credentials", 401, "authentication required")
    ErrInvalidToken            = NewError("auth_invalid_token", 401, "invalid or expired token")
    ErrPolicyDenied            = NewError("policy_denied", 403, "operation denied by policy")
    ErrIdempotencyKeyConflict  = NewError("idempotency_key_conflict", 422, "idempotency key reused with different payload")
    ErrRateLimitExceeded       = NewError("rate_limit_exceeded", 429, "rate limit exceeded")
    ErrNotFound                = NewError("resource_not_found", 404, "resource not found")
    ErrAlreadyExists           = NewError("resource_already_exists", 409, "resource already exists")
    ErrStateConflict           = NewError("resource_state_conflict", 409, "resource in conflicting state")
    ErrInternal                = NewError("internal_error", 500, "internal server error")
)
```

## 8.12 性能预算

| 指标 | 目标 | 备注 |
|---|---|---|
| Middleware 总开销 | < 5ms | 不含业务 |
| 简单 GET 端到端 | < 50ms | 缓存命中时 |
| 写操作端到端 | < 200ms | 含事务 |
| 路由解析 | < 1ms | trie 路由 |
| 错误响应 | < 1ms | 纯 CPU |

## 8.13 测试

| 测试 | 目标 |
|---|---|
| 单元 | handler、middleware、formatter |
| 集成 (testcontainers) | 真实 PG + 完整请求 |
| Contract | Pact against OpenAPI |
| 端到端 | curl/Playwright |
| 性能 | k6 各端点 |
| 幂等 | 同 key 多次请求, 副作用仅 1 次 |

---

**导航 / Navigation:**
[← 07. App 协调](07-app-coordination.md) · [README](README.md) · [09. 安全实现 →](09-security-impl.md)
