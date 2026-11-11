# 10. 可观测性 / Observability

## 10.1 目标 / Purpose

实现 [基本设计 §8.3 监控 / §8.4 日志](../basic-design/08-operations-design.md#83-监控-monitoringobs-req-v1) 的可观测性栈。覆盖：OpenTelemetry 仪表化、指标目录、结构化日志、分布式追踪、健康检查。

## 10.2 模块结构

```
internal/obs/
├── tracer/
│   ├── tracer.go              # OTel Tracer provider 初始化
│   ├── propagation.go         # W3C Trace Context 传播
│   └── span.go                # Span 工具 (attributes, events)
├── metrics/
│   ├── meter.go               # OTel Meter provider
│   ├── catalog.go             # 指标目录 (常量)
│   ├── http.go                # HTTP 中间件指标
│   ├── db.go                  # DB 指标
│   ├── queue.go               # 队列/Outbox 指标
│   └── business.go            # 业务指标
├── log/
│   ├── logger.go              # 结构化 logger
│   ├── fields.go              # 字段标准化
│   └── redactor.go            # 敏感字段 redact
├── health/
│   ├── liveness.go
│   ├── readiness.go
│   └── checks.go              # 健康检查项
├── exporter/
│   ├── otlp.go                # OTLP 导出
│   ├── prometheus.go          # Prometheus 暴露
│   └── stdout.go              # 本地/调试
├── errors.go
└── obs_test.go
```

## 10.3 Tracer 初始化

```go
// [IMPL] internal/obs/tracer/tracer.go
import (
    "go.opentelemetry.io/otel"
    "go.opentelemetry.io/otel/sdk/trace"
    "go.opentelemetry.io/otel/exporters/otlp/otlptrace/otlptracegrpc"
    sdktrace "go.opentelemetry.io/otel/sdk/trace"
)

func InitTracer(ctx context.Context, cfg Config) (func(context.Context) error, error) {
    // 1. 构造 exporter
    var exporter trace.SpanExporter
    switch cfg.Exporter {
    case "otlp":
        exp, err := otlptracegrpc.New(ctx,
            otlptracegrpc.WithEndpoint(cfg.OTLPEndpoint),
            otlptracegrpc.WithInsecure(),
        )
        if err != nil { return nil, err }
        exporter = exp
    case "stdout":
        exporter = stdouttrace.New(stdouttrace.WithPrettyPrint())
    default:
        exporter = stdouttrace.New()  // 本地默认
    }

    // 2. 构造 provider
    provider := sdktrace.NewTracerProvider(
        sdktrace.WithBatcher(exporter,
            sdktrace.WithBatchTimeout(5*time.Second),
            sdktrace.WithMaxExportBatchSize(512),
        ),
        sdktrace.WithResource(resource.NewWithAttributes(
            semconv.SchemaURL,
            semconv.ServiceName("platform"),
            semconv.ServiceVersion(cfg.Version),
        )),
        sdktrace.WithSampler(sdktrace.AlwaysSample()),  // MVP 总是采样
    )
    otel.SetTracerProvider(provider)
    otel.SetTextMapPropagator(propagation.NewCompositeTextMapPropagator(
        propagation.TraceContext{},
        propagation.Baggage{},
    ))

    return provider.Shutdown, nil
}
```

## 10.4 Span 工具

```go
// [IMPL] internal/obs/tracer/span.go
func StartSpan(ctx context.Context, name string, opts ...trace.SpanStartOption) (context.Context, trace.Span) {
    return otel.Tracer("platform").Start(ctx, name, opts...)
}

func RecordError(span trace.Span, err error) {
    if err == nil { return }
    span.RecordError(err)
    span.SetStatus(codes.Error, err.Error())
}

func SetAttributes(span trace.Span, attrs ...attribute.KeyValue) {
    span.SetAttributes(attrs...)
}

// 业务属性: 通用属性 key
var (
    AttrActorID    = attribute.Key("platform.actor.id")
    AttrActorType  = attribute.Key("platform.actor.type")
    AttrRunID      = attribute.Key("platform.run_id")
    AttrRepoID     = attribute.Key("platform.repo.id")
    AttrAction     = attribute.Key("platform.action")
    AttrResourceType = attribute.Key("platform.resource.type")
    AttrResourceID = attribute.Key("platform.resource.id")
    AttrPolicyDecision = attribute.Key("platform.policy.decision")
    AttrErrorCode  = attribute.Key("platform.error.code")
)
```

### 10.4.1 业务 Span 模板

```go
// [IMPL] 业务代码使用模板
func (h *ReposHandler) Create(ctx context.Context, req CreateRepoRequest) (*Repo, error) {
    ctx, span := tracer.StartSpan(ctx, "api.repos.create",
        trace.WithAttributes(
            tracer.AttrActorID.String(actor.ID.String()),
            tracer.AttrAction.String("repo.create"),
            tracer.AttrResourceType.String("organization"),
            tracer.AttrResourceID.String(req.OrgID),
        ),
    )
    defer span.End()

    // ... 业务逻辑, 用 tracer.RecordError(span, err) 记录错误
}
```

## 10.5 指标目录

### 10.5.1 HTTP 指标

```go
// [IMPL] internal/obs/metrics/http.go
var (
    HTTPRequestsTotal = prometheus.NewCounterVec(
        prometheus.CounterOpts{
            Name: "platform_http_requests_total",
            Help: "Total HTTP requests by method, path, status",
        },
        []string{"method", "path_template", "status"},
    )

    HTTPRequestDuration = prometheus.NewHistogramVec(
        prometheus.HistogramOpts{
            Name:    "platform_http_request_duration_seconds",
            Help:    "HTTP request duration in seconds",
            Buckets: []float64{0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10},
        },
        []string{"method", "path_template", "status"},
    )

    HTTPRequestsInFlight = prometheus.NewGauge(
        prometheus.GaugeOpts{
            Name: "platform_http_requests_in_flight",
            Help: "Current number of HTTP requests being processed",
        },
    )
)
```

### 10.5.2 DB 指标

```go
// [IMPL] internal/obs/metrics/db.go
var (
    DBQueryDuration = prometheus.NewHistogramVec(
        prometheus.HistogramOpts{
            Name:    "platform_db_query_duration_seconds",
            Help:    "DB query duration",
            Buckets: []float64{0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5},
        },
        []string{"operation", "table"},
    )

    DBConnections = prometheus.NewGaugeVec(
        prometheus.GaugeOpts{
            Name: "platform_db_connections",
            Help: "DB connection pool stats",
        },
        []string{"state"},  // 'active','idle','waiting'
    )

    DBSlowQueries = prometheus.NewCounter(
        prometheus.CounterOpts{
            Name: "platform_db_slow_queries_total",
            Help: "Number of queries exceeding 1s threshold",
        },
    )
)
```

### 10.5.3 业务指标

```go
// [IMPL] internal/obs/metrics/business.go
var (
    GraphNodesTotal = prometheus.NewGauge(
        prometheus.GaugeOpts{
            Name: "platform_graph_nodes_total",
            Help: "Total nodes in graph (approximate, sampled every 60s)",
        },
    )

    GraphEdgesTotal = prometheus.NewGauge(
        prometheus.GaugeOpts{
            Name: "platform_graph_edges_total",
            Help: "Total edges in graph",
        },
    )

    PolicyEvaluations = prometheus.NewCounterVec(
        prometheus.CounterOpts{
            Name: "platform_policy_evaluations_total",
            Help: "Policy evaluations by decision",
        },
        []string{"decision"},  // 'allow','deny','default_deny'
    )

    PolicyEvaluationDuration = prometheus.Histogram(
        prometheus.HistogramOpts{
            Name:    "platform_policy_evaluation_duration_seconds",
            Help:    "Policy evaluation duration",
            Buckets: []float64{0.0001, 0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1},
        },
    )

    AgentRunsInProgress = prometheus.NewGaugeVec(
        prometheus.GaugeOpts{
            Name: "platform_agent_runs_in_progress",
            Help: "Currently running agent runs by status",
        },
        []string{"status"},
    )

    AgentRunsTotal = prometheus.NewCounterVec(
        prometheus.CounterOpts{
            Name: "platform_agent_runs_total",
            Help: "Total agent runs by terminal status",
        },
        []string{"status"},
    )

    AICallsTotal = prometheus.NewCounterVec(
        prometheus.CounterOpts{
            Name: "platform_ai_calls_total",
            Help: "Total AI calls by model and result",
        },
        []string{"model", "result"},  // 'success','error','rate_limit','policy_denied'
    )

    AICallTokens = prometheus.CounterVec(
        prometheus.CounterOpts{
            Name: "platform_ai_tokens_total",
            Help: "Total AI tokens consumed by type and model",
        },
        []string{"type", "model"},  // type: 'input','output'
    )

    AICallCostUSD = prometheus.NewCounterVec(
        prometheus.CounterOpts{
            Name: "platform_ai_cost_usd_total",
            Help: "Total AI cost in USD by model",
        },
        []string{"model"},
    )

    OutboxPending = prometheus.NewGaugeVec(
        prometheus.GaugeOpts{
            Name: "platform_outbox_pending",
            Help: "Pending outbox events by type",
        },
        []string{"event_type"},
    )

    OutboxDLQ = prometheus.NewCounterVec(
        prometheus.CounterOpts{
            Name: "platform_outbox_dlq_total",
            Help: "Outbox events moved to DLQ",
        },
        []string{"event_type"},
    )

    GitPushDuration = prometheus.Histogram(
        prometheus.HistogramOpts{
            Name:    "platform_git_push_duration_seconds",
            Help:    "Git push duration",
            Buckets: []float64{0.1, 0.5, 1, 2.5, 5, 10, 30, 60, 300},
        },
    )

    SecretRotations = prometheus.NewCounter(
        prometheus.CounterOpts{
            Name: "platform_secret_rotations_total",
            Help: "Total secret rotations performed",
        },
    )
)
```

## 10.6 结构化日志

```go
// [IMPL] internal/obs/log/logger.go
import "go.uber.org/zap"

func NewLogger(cfg Config) (*zap.Logger, error) {
    encoderCfg := zap.NewProductionEncoderConfig()
    encoderCfg.TimeKey = "ts"
    encoderCfg.MessageKey = "msg"
    encoderCfg.LevelKey = "level"
    encoderCfg.CallerKey = "caller"
    encoderCfg.EncodeTime = zapcore.ISO8601TimeEncoder

    var encoder zapcore.Encoder
    if cfg.Format == "console" {
        encoder = zapcore.NewConsoleEncoder(encoderCfg)
    } else {
        encoder = zapcore.NewJSONEncoder(encoderCfg)
    }

    return zap.New(zapcore.NewCore(encoder, zapcore.AddSync(os.Stdout), zap.InfoLevel)), nil
}
```

### 10.6.1 字段标准化

```go
// [IMPL] internal/obs/log/fields.go
type LogFields struct {
    TraceID    string
    SpanID     string
    ActorID    string
    ActorType  string
    Event      string
    RequestID  string
    RunID      string
    Error      string
}

func (f LogFields) ToZap() []zap.Field {
    var fields []zap.Field
    if f.TraceID != "" { fields = append(fields, zap.String("trace_id", f.TraceID)) }
    if f.SpanID != "" { fields = append(fields, zap.String("span_id", f.SpanID)) }
    if f.ActorID != "" { fields = append(fields, zap.String("actor_id", f.ActorID)) }
    if f.ActorType != "" { fields = append(fields, zap.String("actor_type", f.ActorType)) }
    if f.Event != "" { fields = append(fields, zap.String("event", f.Event)) }
    if f.RequestID != "" { fields = append(fields, zap.String("request_id", f.RequestID)) }
    if f.RunID != "" { fields = append(fields, zap.String("run_id", f.RunID)) }
    if f.Error != "" { fields = append(fields, zap.String("error", f.Error)) }
    return fields
}
```

### 10.6.2 敏感字段 redact

```go
// [IMPL] internal/obs/log/redactor.go
var sensitiveKeys = map[string]bool{
    "password": true, "token": true, "api_key": true,
    "secret": true, "authorization": true, "private_key": true,
    "ssh_key": true, "access_token": true, "refresh_token": true,
}

func RedactMap(m map[string]any) map[string]any {
    for k, v := range m {
        if sensitiveKeys[strings.ToLower(k)] {
            m[k] = "[REDACTED]"
            continue
        }
        if nested, ok := v.(map[string]any); ok {
            RedactMap(nested)
        }
    }
    return m
}

func RedactString(s string) string {
    // 启发式: 匹配 sk-xxxx, ghp_xxxx, etc.
    for _, p := range []string{
        `sk-[A-Za-z0-9]{20,}`,
        `ghp_[A-Za-z0-9]{36}`,
        `-----BEGIN [A-Z ]*PRIVATE KEY-----[\s\S]+-----END`,
    } {
        re := regexp.MustCompile(p)
        s = re.ReplaceAllString(s, "[REDACTED]")
    }
    return s
}
```

## 10.7 健康检查

### 10.7.1 Liveness (`/healthz`)

```go
// [IMPL] internal/obs/health/liveness.go
func LivenessHandler(w http.ResponseWriter, r *http.Request) {
    // 仅检查: 进程是否在响应?
    // 不要依赖外部资源 (DB 等), 否则短暂网络抖动就会被 kill
    w.WriteHeader(200)
    w.Write([]byte("ok"))
}
```

### 10.7.2 Readiness (`/readyz`)

```go
// [IMPL] internal/obs/health/readiness.go
type Checker func(ctx context.Context) error

type Readiness struct {
    checks map[string]Checker
}

func (r *Readiness) Register(name string, c Checker) {
    r.checks[name] = c
}

func (r *Readiness) Handler() http.Handler {
    return http.HandlerFunc(func(w http.ResponseWriter, req *http.Request) {
        ctx, cancel := context.WithTimeout(req.Context(), 3*time.Second)
        defer cancel()

        results := make(map[string]string)
        overall := "ok"
        for name, c := range r.checks {
            if err := c(ctx); err != nil {
                results[name] = "fail: " + err.Error()
                overall = "degraded"
            } else {
                results[name] = "ok"
            }
        }

        status := 200
        if overall != "ok" { status = 503 }
        w.Header().Set("Content-Type", "application/json")
        w.WriteHeader(status)
        json.NewEncoder(w).Encode(map[string]any{
            "status": overall, "checks": results,
        })
    })
}
```

### 10.7.3 内置检查项

```go
// [IMPL]
func DefaultChecks(db *pgxpool.Pool, gitBinary string) []Check {
    return []Check{
        {
            Name: "postgres",
            Check: func(ctx context.Context) error {
                return db.Ping(ctx)
            },
        },
        {
            Name: "git_binary",
            Check: func(ctx context.Context) error {
                cmd := exec.CommandContext(ctx, gitBinary, "--version")
                return cmd.Run()
            },
        },
        {
            Name: "disk_space",
            Check: func(ctx context.Context) error {
                var stat syscall.Statfs_t
                if err := syscall.Statfs("/var/lib/platform", &stat); err != nil {
                    return err
                }
                freeGB := float64(stat.Bavail) * float64(stat.Bsize) / 1e9
                if freeGB < 1.0 { return fmt.Errorf("only %.2f GB free", freeGB) }
                return nil
            },
        },
    }
}
```

## 10.8 Prometheus 暴露

```go
// [IMPL] internal/obs/exporter/prometheus.go
import "github.com/prometheus/client_golang/prometheus/promhttp"

func NewHandler() http.Handler {
    return promhttp.HandlerFor(
        prometheus.DefaultGatherer,
        promhttp.HandlerOpts{
            EnableOpenMetrics: true,
        },
    )
}

// 路由: r.GET("/metrics", m.Public, obs.NewHandler())
```

## 10.9 Span 跨进程传播

```go
// [IMPL] HTTP 请求的 trace 上下文传播
func InjectTraceContext(ctx context.Context, req *http.Request) {
    otel.GetTextMapPropagator().Inject(ctx, propagation.HeaderCarrier(req.Header))
}

func ExtractTraceContext(ctx context.Context, req *http.Request) context.Context {
    return otel.GetTextMapPropagator().Extract(ctx, propagation.HeaderCarrier(req.Header))
}

// W3C Trace Context 头格式:
// traceparent: 00-<trace-id>-<span-id>-<flags>
// tracestate: <vendor-specific>
```

## 10.10 告警规则 (V1)

```yaml
# prometheus/alerts.yaml
groups:
  - name: platform-critical
    rules:
      - alert: AISEC009Violated
        expr: platform_db_role_check_failed > 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "AISEC-REQ-009(a) violation detected"
          description: "events table is mutable from non-owner role"

      - alert: OutboxDLQGrowing
        expr: increase(platform_outbox_dlq_total[1h]) > 10
        for: 5m
        labels:
          severity: warning

      - alert: PolicyEvaluationTimeout
        expr: rate(platform_policy_evaluations_total{decision="timeout"}[5m]) > 0.01
        for: 1m
        labels:
          severity: warning

      - alert: AISpendSpike
        expr: increase(platform_ai_cost_usd_total[1h]) > 100
        for: 5m
        labels:
          severity: warning

  - name: platform-performance
    rules:
      - alert: HighP99Latency
        expr: histogram_quantile(0.99, rate(platform_http_request_duration_seconds_bucket[5m])) > 1
        for: 5m
        labels:
          severity: warning

      - alert: CacheHitRateLow
        expr: rate(platform_cache_hits_total[5m]) / rate(platform_cache_lookups_total[5m]) < 0.6
        for: 10m
        labels:
          severity: info
```

## 10.11 启动时初始化

```go
// [IMPL] cmd/platform/main.go 启动序列
func main() {
    // 1. 加载配置
    cfg := config.Load()

    // 2. 初始化 logger
    logger, _ := obs.NewLogger(cfg.Log)
    defer logger.Sync()

    // 3. 初始化 tracer
    shutdownTracer, _ := obs.InitTracer(context.Background(), cfg.Obs)
    defer shutdownTracer(context.Background())

    // 4. 初始化 metrics
    obs.RegisterMetrics()

    // 5. 连接 DB (启动时检查 AISEC-REQ-009(a))
    db, _ := pgxpool.New(ctx, cfg.Database.DSN())
    defer db.Close()
    if err := security.EnforceEventsImmutability(db); err != nil {
        logger.Fatal("AISEC-009(a) violated", zap.Error(err))
    }

    // 6. 初始化 readiness
    readiness := obs.NewReadiness()
    for _, c := range obs.DefaultChecks(db, cfg.Git.Binary) {
        readiness.Register(c.Name, c.Check)
    }

    // 7. 启动服务
    server := apihttp.NewServer(cfg, db, logger, readiness)
    server.Run()
}
```

## 10.12 错误定义

```go
// [IMPL] internal/obs/errors.go
var (
    ErrExporterFailed     = NewError("obs_exporter_failed", 500, "OTel exporter failed")
    ErrMetricsInitFailed  = NewError("obs_metrics_init_failed", 500, "metrics initialization failed")
    ErrLogEncodingFailed  = NewError("obs_log_encoding_failed", 500, "log encoding failed")
    ErrRedactFailed       = NewError("obs_redact_failed", 500, "failed to redact sensitive field")
)
```

## 10.13 性能预算

| 指标 | 目标 |
|---|---|
| Tracer overhead (per span) | < 100us |
| Metric 记录 | < 10us |
| Logger (JSON) | < 50us per line |
| Redact 启发式 | < 1ms per call |
| Health check (readiness) | < 500ms total |

## 10.14 测试

| 测试 | 目标 |
|---|---|
| 单元 | Redact, Field 转换, Span helper |
| 集成 (OTel collector mock) | Exporter, propagation |
| 端到端 | trace 跨 HTTP / DB / 队列传播 |
| 性能 | overhead 不超预算 |

---

**导航 / Navigation:**
[← 09. 安全实现](09-security-impl.md) · [README](README.md) · [11. 错误处理 →](11-error-handling.md)
