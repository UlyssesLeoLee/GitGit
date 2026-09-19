# 11. 错误处理 / Error Handling

## 11.1 目标 / Purpose

定义全平台统一的**错误类型、错误码、错误传播、错误响应格式、恢复策略**。与 [08 API 处理器](08-api-handlers.md) 的 HTTP 错误响应、§1 数据层 的 SQLSTATE 映射、§9 安全实现 的密钥错误等子系统协同。

## 11.2 模块结构

```
internal/errors/
├── errors.go                # PlatformError 接口 + 构造
├── code.go                  # 错误码常量
├── wrap.go                  # Wrap / Unwrap
├── classify.go              # HTTP 状态映射
├── recovery.go              # 恢复策略
├── retry.go                 # 重试判定
├── deadletter.go            # 死信
└── errors_test.go
```

## 11.3 错误接口

```go
// [IMPL] internal/errors/errors.go

type PlatformError interface {
    error
    Code() string                  // 标准化错误码 (e.g., "policy_denied")
    HTTPStatus() int               // HTTP 状态码 (e.g., 403)
    Severity() Severity            // info/warn/error/critical
    Details() map[string]any       // 详情 payload
    Cause() error                  // 包装的底层错误 (支持 Unwrap)
    StackTrace() []string          // 调用栈 (开发模式)
    IsRetryable() bool             // 是否可重试
    TraceID() string               // 关联 trace_id
}

type Severity string
const (
    SeverityInfo     Severity = "info"
    SeverityWarn     Severity = "warn"
    SeverityError    Severity = "error"
    SeverityCritical Severity = "critical"
)

type Error struct {
    Code     string
    Message  string
    Status   int
    Severity Severity
    Details  map[string]any
    Cause    error
    Stack    []string
    Retryable bool
    Trace    string
}

func (e *Error) Error() string {
    if e.Cause != nil {
        return fmt.Sprintf("%s: %s: %v", e.Code, e.Message, e.Cause)
    }
    return fmt.Sprintf("%s: %s", e.Code, e.Message)
}

func (e *Error) Unwrap() error { return e.Cause }

// 构造器
func New(code string, status int, message string) *Error {
    return &Error{
        Code: code, Message: message, Status: status,
        Severity: SeverityError,
        Stack:    captureStack(),
    }
}

func NewWithSeverity(code string, status int, message string, sev Severity) *Error {
    e := New(code, status, message)
    e.Severity = sev
    return e
}
```

## 11.4 错误码注册表

**[PROPOSAL]** 所有错误码集中定义，避免散落。

```go
// [IMPL] internal/errors/code.go

// 通用
const (
    CodeInternal               = "internal_error"
    CodeInvalidRequest         = "validation_failed"
    CodeUnauthorized           = "auth_missing_credentials"
    CodeInvalidToken           = "auth_invalid_token"
    CodeExpired                = "auth_expired"
    CodePolicyDenied           = "policy_denied"
    CodePolicyTimeout          = "policy_evaluation_timeout"
    CodeRateLimit              = "rate_limit_exceeded"
    CodeIdempotencyConflict    = "idempotency_key_conflict"
    CodeNotFound               = "resource_not_found"
    CodeAlreadyExists          = "resource_already_exists"
    CodeStateConflict          = "resource_state_conflict"
)

// 数据层
const (
    CodeDBError              = "db_error"
    CodeDBUnavailable        = "db_unavailable"
    CodeDBIntegrityViolation = "db_integrity_violation"
    CodeDBUniqueViolation    = "db_unique_violation"
    CodeDBForeignKeyViolation = "db_foreign_key_violation"
    CodeDBDeadlock            = "db_deadlock"
    CodeDBSerializationFailure = "db_serialization_failure"
)

// 存储过程 / 协调
const (
    CodeCoordUnknownProc        = "coord_unknown_proc"
    CodeCoordProcTypeNotAllowed = "coord_proc_type_not_allowed"
    CodeCoordPayloadTooLarge    = "coord_payload_too_large"
    CodeOutboxNoTarget          = "outbox_no_target"
    CodeSagaNotFound            = "saga_not_found"
    CodeSagaStepTimeout         = "saga_step_timeout"
)

// Git
const (
    CodeGitPolicyDenied    = "git_policy_denied"
    CodeGitHookTimeout     = "git_hook_timeout"
    CodeGitHookPolicyFail  = "git_hook_policy_fail"
    CodeGitRepoNotFound    = "git_repo_not_found"
    CodeGitRepoLocked      = "git_repo_locked"
    CodeGitPushRejected    = "git_push_rejected"
)

// Agent / AI
const (
    CodeAgentApprovalRequired    = "agent_approval_required"
    CodeAgentPrivilegeEscalation = "agent_privilege_escalation"
    CodeAgentResourceExceeded    = "agent_resource_limit_exceeded"
    CodeAgentTokenExpired        = "agent_token_expired"
    CodeAgentInvalidState        = "agent_invalid_state_transition"
    CodeAIPolicyDenied           = "ai_policy_denied"
    CodeAIBudgetExceeded         = "ai_budget_exceeded"
    CodeAINoProvider             = "ai_no_provider"
    CodeAIProviderUnavailable    = "ai_provider_unavailable"
    CodeAIProviderRateLimit      = "ai_provider_rate_limit"
    CodeAISecretRedacted         = "ai_secret_redacted"
)

// 安全
const (
    CodeKEKNotConfigured   = "kek_not_configured"
    CodeCiphertextCorrupt  = "ciphertext_corrupt"
    CodeWrappedDEKCorrupt  = "wrapped_dek_corrupt"
    CodeSecretExpired      = "secret_expired"
    CodeTLSConfigInvalid   = "tls_config_invalid"
    CodeAISEC009Violated   = "aisec009a_violated"
)
```

## 11.5 HTTP 状态映射

```go
// [IMPL] internal/errors/classify.go

func HTTPStatus(err error) int {
    if e, ok := err.(*Error); ok { return e.Status }
    switch {
    case errors.Is(err, context.DeadlineExceeded): return 504
    case errors.Is(err, context.Canceled): return 499  // client closed
    default: return 500
    }
}

func Code(err error) string {
    if e, ok := err.(*Error); ok { return e.Code }
    return CodeInternal
}

func Severity(err error) Severity {
    if e, ok := err.(*Error); ok { return e.Severity }
    return SeverityError
}

func IsRetryable(err error) bool {
    if e, ok := err.(*Error); ok { return e.Retryable }
    // 默认: 未知错误不可重试 (避免放大问题)
    return false
}
```

## 11.6 包装与传播

```go
// [IMPL] internal/errors/wrap.go

func Wrap(err error, code string, status int, message string) *Error {
    return &Error{
        Code: code, Message: message, Status: status,
        Severity: SeverityError, Cause: err,
        Stack:    captureStack(),
    }
}

func Wrapf(err error, code string, status int, format string, args ...any) *Error {
    return Wrap(err, code, status, fmt.Sprintf(format, args...))
}

func WithDetails(err error, details map[string]any) *Error {
    e, ok := err.(*Error)
    if !ok { return Wrap(err, CodeInternal, 500, err.Error()) }
    e.Details = details
    return e
}

func WithRetryable(err error) *Error {
    e, ok := err.(*Error)
    if !ok { return Wrap(err, CodeInternal, 500, err.Error()) }
    e.Retryable = true
    return e
}

func WithSeverity(err error, sev Severity) *Error {
    e, ok := err.(*Error)
    if !ok { return Wrap(err, CodeInternal, 500, err.Error()) }
    e.Severity = sev
    return e
}
```

### 11.6.1 使用示例

```go
// [IMPL]
func (s *RepoService) Create(ctx context.Context, req CreateRepoRequest) (*Repo, error) {
    if err := validate.Struct(&req); err != nil {
        return nil, errors.Wrap(err, errors.CodeInvalidRequest, 400, "validation failed").
            WithDetails(map[string]any{"field_errors": err.(validator.ValidationErrors)})
    }

    repo, err := s.repoRepo.Create(ctx, req)
    if err != nil {
        if errors.Is(err, pgx.ErrNoRows) {
            return nil, errors.New(errors.CodeNotFound, 404, "org not found")
        }
        var pgErr *pgconn.PgError
        if errors.As(err, &pgErr) && pgErr.Code == "23505" {
            return nil, errors.Wrap(err, errors.CodeAlreadyExists, 409, "repo already exists").
                WithDetails(map[string]any{"constraint": pgErr.ConstraintName})
        }
        return nil, errors.Wrap(err, errors.CodeDBError, 500, "db error")
    }
    return repo, nil
}
```

## 11.7 重试策略

```go
// [IMPL] internal/errors/retry.go

type RetryPolicy struct {
    MaxAttempts int
    InitialWait time.Duration
    MaxWait     time.Duration
    Multiplier  float64
}

var DefaultRetryPolicy = RetryPolicy{
    MaxAttempts: 3,
    InitialWait: 100 * time.Millisecond,
    MaxWait:     5 * time.Second,
    Multiplier:  2.0,
}

// 哪些错误默认重试
var retryableErrors = map[string]bool{
    CodeDBDeadlock:             true,
    CodeDBSerializationFailure: true,
    CodeAIBudgetExceeded:       false,  // 不重试 (已超预算)
    CodeAIProviderRateLimit:    true,
    CodeAIProviderUnavailable:  true,
    CodeDBUnavailable:          true,
    CodeOutboxNoTarget:         false,  // 配置错误
}

func ShouldRetry(err error) bool {
    if e, ok := err.(*Error); ok {
        return e.Retryable || retryableErrors[e.Code]
    }
    return false
}

func Retry(ctx context.Context, policy RetryPolicy, op func() error) error {
    var lastErr error
    wait := policy.InitialWait
    for attempt := 1; attempt <= policy.MaxAttempts; attempt++ {
        err := op()
        if err == nil { return nil }
        lastErr = err
        if !ShouldRetry(err) { return err }
        if attempt == policy.MaxAttempts { return lastErr }

        select {
        case <-ctx.Done(): return ctx.Err()
        case <-time.After(wait):
        }
        wait = time.Duration(float64(wait) * policy.Multiplier)
        if wait > policy.MaxWait { wait = policy.MaxWait }
    }
    return lastErr
}
```

### 11.7.1 各子系统的重试决策

| 错误类别 | 重试 | 备注 |
|---|---|---|
| DB deadlock / serialization | ✓ 最多 3 次 | 指数退避 |
| AI provider 5xx | ✓ 最多 3 次 | 1s/5s/30s |
| AI provider 429 | ✓ 遵守 Retry-After | 最多 3 次 |
| HTTP 5xx (上游) | ✓ | 1s/5s/30s |
| Outbox 投递失败 | ✓ | 1s/2s/4s/...30min |
| Saga 步骤失败 | ✗ | 走补偿 |
| 4xx (业务) | ✗ | |
| Policy denied | ✗ | |
| Auth 失败 | ✗ | |

## 11.8 错误响应格式 (HTTP)

```go
// [IMPL] 与 [08 API 处理器 §8.5](08-api-handlers.md#85-错误响应格式) 一致
// 任何 HTTP 处理器出口用 errors.WriteHTTPError(w, err) 一行处理

func WriteHTTPError(w http.ResponseWriter, err error) {
    code := Code(err)
    status := HTTPStatus(err)
    severity := Severity(err)
    traceID := traceIDFromContext(...)

    // 1. 写 Event (统一审计)
    audit.EmitAsync(traceID, "api.error", map[string]any{
        "code": code, "status": status, "severity": severity,
        "message": err.Error(),  // 注意: 不含敏感信息
    })

    // 2. 响应
    payload := map[string]any{
        "error": map[string]any{
            "code":     code,
            "message":  publicMessage(err),  // 见 11.9
            "details":  publicDetails(err),
            "trace_id": traceID,
            "documentation_url": fmt.Sprintf("https://docs.example.com/errors/%s", code),
        },
    }
    w.Header().Set("Content-Type", "application/json")
    w.WriteHeader(status)
    json.NewEncoder(w).Encode(payload)
}
```

## 11.9 公开消息 vs 内部消息

**[PROPOSAL]** 错误有两个版本：
- **公开消息** (给客户端): 用户友好的描述，**不含敏感信息**
- **内部消息** (给开发者/日志): 详细技术信息

```go
// [IMPL]
func publicMessage(err error) string {
    e, ok := err.(*Error)
    if !ok { return "internal error" }

    // 白名单: 这些 code 公开内部消息
    safeToExpose := map[string]bool{
        CodeInvalidRequest:  true,
        CodeNotFound:        true,
        CodeAlreadyExists:   true,
        CodeStateConflict:   true,
        CodePolicyDenied:    true,
        CodeRateLimit:       true,
        CodeIdempotencyConflict: true,
    }
    if safeToExpose[e.Code] { return e.Message }
    return "internal error, see logs (trace_id)"
}

func publicDetails(err error) map[string]any {
    e, ok := err.(*Error)
    if !ok { return nil }
    // 不暴露内部结构 (file path, stack trace, 凭证, 等)
    safeKeys := map[string]bool{
        "field_errors": true, "constraint": true, "retry_after": true,
    }
    safe := make(map[string]any)
    for k, v := range e.Details {
        if safeKeys[k] { safe[k] = v }
    }
    return safe
}
```

## 11.10 Panic 恢复

```go
// [IMPL] internal/errors/recovery.go
// HTTP middleware 用
func RecoveryMiddleware(next http.Handler) http.Handler {
    return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
        defer func() {
            if err := recover(); err != nil {
                traceID := traceIDFromContext(r.Context())
                logger.Error("panic recovered",
                    zap.Any("error", err),
                    zap.String("trace_id", traceID),
                    zap.String("path", r.URL.Path),
                )
                // 写 Event
                audit.EmitAsync(traceID, "api.panic", map[string]any{
                    "panic": err, "path": r.URL.Path,
                })
                WriteHTTPError(w, New(CodeInternal, 500, "internal error"))
            }
        }()
        next.ServeHTTP(w, r)
    })
}
```

## 11.11 死信处理

```go
// [IMPL] internal/errors/deadletter.go

// 通用死信入队函数
func ToDeadLetter(ctx context.Context, originalErr error, payload any, dlq DLQ) error {
    entry := DLQEntry{
        OriginalError: originalErr.Error(),
        Payload:       payload,
        Timestamp:     time.Now(),
        TraceID:       traceIDFromContext(ctx),
    }
    return dlq.Enqueue(ctx, entry)
}
```

应用：outbox、Saga、AI 调用失败 3 次后的终态。

## 11.12 错误日志与告警

```go
// [IMPL] 错误日志策略
func logError(err error) {
    e, ok := err.(*Error)
    if !ok { logger.Error("untyped error", zap.Error(err)); return }

    fields := []zap.Field{
        zap.String("code", e.Code),
        zap.String("severity", string(e.Severity)),
        zap.String("message", e.Message),
        zap.Bool("retryable", e.Retryable),
    }
    if e.Cause != nil { fields = append(fields, zap.Error(e.Cause)) }

    switch e.Severity {
    case SeverityCritical:
        logger.Error("critical error", fields...)
        alerting.Send(e)  // 立刻告警
    case SeverityError:
        logger.Error("error", fields...)
    case SeverityWarn:
        logger.Warn("warning", fields...)
    default:
        logger.Info("info", fields...)
    }
}
```

## 11.13 错误码注册表的 OpenAPI 同步

**[PROPOSAL]** 每个错误码应有 OpenAPI 文档说明。实现：CI 校验 `errors/code.go` 与 `api/openapi/v1.yaml` 的 `components/responses/Error*` 同步。

```yaml
# api/openapi/v1.yaml (片段)
components:
  responses:
    PolicyDenied:
      description: "Operation denied by policy"
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'
    RateLimitExceeded:
      description: "Rate limit exceeded"
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorResponse'
```

## 11.14 性能预算

| 指标 | 目标 |
|---|---|
| New (构造) | < 1us |
| Wrap | < 1us |
| HTTPStatus / Code | < 100ns |
| WriteHTTPError | < 1ms (不含 logger) |
| Retry 判定 | < 100ns |

## 11.15 测试

| 测试 | 目标 |
|---|---|
| 单元 | 构造、wrap、unwrap、分类、retry 判定 |
| 集成 | HTTP 响应格式、middleware 链路 |
| E2E | panic 恢复、retry 行为 |
| 一致性 | 错误码注册表 vs OpenAPI 同步 |

---

**导航 / Navigation:**
[← 10. 可观测性](10-observability.md) · [README](README.md)
