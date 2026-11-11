# Distributed Tracing 设计 / Distributed Tracing Design

> **关联 OBS-REQ**: OBS-TRC-001 ~ OBS-TRC-010
> **统一 SDK**: `tracing` + `tracing-opentelemetry` + `opentelemetry-otlp`
> **Backend**: Tempo (ObjectStore 持久化) + S3 / MinIO 冷存
> **关联**: 与 [§02 Metrics](02-metrics.md) / [§03 Logs](03-logs.md) 通过 W3C Trace Context 关联

## 1. 标准

- **W3C Trace Context** (https://www.w3.org/TR/trace-context/)
- **W3C Baggage** (跨服务透传业务字段)
- **OpenTelemetry Semantic Conventions** (Span 命名 / 属性规约)

## 2. 关键概念

| 概念 | 定义 |
|---|---|
| **Trace** | 一次端到端请求的完整调用链 |
| **Span** | Trace 中的一次操作（HTTP / DB / RPC / 自定义） |
| **SpanKind** | `SERVER` / `CLIENT` / `PRODUCER` / `CONSUMER` / `INTERNAL` |
| **TraceID** | 32 字符 hex（16 bytes） |
| **SpanID** | 16 字符 hex（8 bytes） |
| **ParentSpanID** | 父 SpanID（root span 的 parent = 0） |
| **Attributes** | Span 上的 key=value（用于过滤和聚合） |
| **Events** | Span 上的离散事件（带 timestamp） |
| **Links** | Span 与其他 Trace 的关联 |

## 3. 服务间调用链

### 3.1 HTTP 路径

```
Client
  │
  │  HTTP + traceparent: 00-{trace_id}-{span_id}-01
  ▼
API Gateway / Reverse Proxy
  │
  │  注入 trace_id 到日志 + metric exemplar
  ▼
gitgit-server (Axum handler)
  │
  │  http_client.call(repo_service)
  ▼
Service A (gRPC)
  │
  │  sqlx.query("INSERT ...")
  ▼
PostgreSQL
```

每个跨进程调用都必须传递 `traceparent` header。

### 3.2 gRPC 路径（内部）

Tonic 自动注入 `grpc-trace-bin` header（基于 W3C Trace Context）。

### 3.3 异步任务（Saga / Outbox）

```rust
// EventBus 派发 (后台 worker)
#[tracing::instrument(
    skip_all,
    fields(
        otel.kind = "consumer",
        messaging.system = "postgresql",
        messaging.destination = "event_stream",
        messaging.operation = "process",
        event_id = %event.id,
        event_type = %event.type,
    ),
)]
async fn process_event(event: Event) {
    // 从 event.correlation_id 提取上游 trace_id
    // 创建 linked span
}
```

### 3.4 DB 调用

```rust
// sqlx 自动支持 tracing（需开启 sqlx::query! 宏）
let row = sqlx::query!("SELECT * FROM nodes WHERE id = $1", id)
    .fetch_one(&pool)
    .await?;
// 产出 span: db.system=postgresql, db.statement="SELECT * FROM nodes ...", db.rows=1
```

## 4. Span 命名规约

### 4.1 HTTP / gRPC

按 OpenTelemetry Semantic Conventions：

| 服务类型 | 命名 | 例 |
|---|---|---|
| HTTP server | `<method> <route>` | `POST /api/v1/repos/{id}` |
| gRPC server | `<package>.<service>/<method>` | `gitgit.graph.v1.GraphService/CreateNode` |
| DB client | `<db.operation> <db.sql.table>` | `INSERT nodes`, `SELECT nodes` |
| Redis client | `<command>` | `GET`, `HSET` |
| 自定义 | `<noun.verb>` | `policy.evaluate`, `app.upgrade` |

### 4.2 关键 Span 属性（必填）

| 服务类型 | 必填属性 |
|---|---|
| HTTP server | `http.method`, `http.route`, `http.status_code` |
| HTTP client | `http.method`, `http.url`, `http.status_code` |
| gRPC server | `rpc.system`, `rpc.service`, `rpc.method` |
| gRPC client | `rpc.system`, `rpc.service`, `rpc.method`, `rpc.grpc.status_code` |
| DB | `db.system` (=postgresql), `db.statement` (sanitized), `db.operation` |
| 自定义 | `gitgit.<subsystem>.<event>` |

### 4.3 业务 Span（关键路径）

```rust
#[tracing::instrument(
    name = "policy.evaluate",
    skip_all,
    fields(
        gitgit.policy.subject_id = %input.subject_id,
        gitgit.policy.action = %input.action,
        gitgit.policy.resource_type = %input.resource_type,
        gitgit.policy.result = tracing::field::Empty,  // 结束时填
        otel.kind = "internal",
    ),
)]
pub async fn evaluate(input: PolicyInput) -> Result<bool, AppError> {
    // ...
    tracing::Span::current().record("gitgit.policy.result", "allow");
    // ...
}
```

## 5. Trace Context 传播

### 5.1 HTTP 入站

```rust
// Axum middleware 自动提取 (tower-http::trace::TraceLayer)
use tower_http::trace::TraceLayer;

let app = Router::new()
    .route("/api/v1/repos", post(create_repo))
    .layer(TraceLayer::new_for_http());
```

`TraceLayer` 自动从 `traceparent` header 提取，注入到当前 span。

### 5.2 HTTP 出站

```rust
// reqwest 默认不传递 trace context，需手动
let client = reqwest::Client::builder()
    .default_headers(/* inject traceparent from current span */)
    .build()?;
```

或者使用 `tracing-actix-web` 风格的 helper。

### 5.3 gRPC

Tonic 自动通过 `grpc-trace-bin` metadata 传递。

### 5.4 DB

sqlx 0.8 + `tracing` feature 自动产出 child span。

### 5.5 后台任务

```rust
// 从外部 trace_id 恢复
#[tracing::instrument(
    fields(
        trace_id = %event.correlation_id,  // 如果是 trace_id
        parent_span_id = tracing::field::Empty,
    ),
)]
async fn handle_event(event: Event) {
    // 如果 event.correlation_id 是 trace context
    // 用 opentelemetry::global::get_text_map_propagator 重建
}
```

## 6. 采样策略

### 6.1 Tail Sampling（推荐）

由 OTel Collector tail_sampling processor 实现：

```yaml
processors:
  tail_sampling:
    decision_wait: 10s
    num_traces: 100000
    expected_new_traces_per_sec: 1000
    policies:
      # 错误全采样
      - name: errors
        type: status_code
        status_code: { status_codes: [ERROR] }

      # 慢请求全采样
      - name: slow-requests
        type: latency
        latency: { threshold_ms: 1000 }

      # Admin / Auth 关键路径全采样
      - name: auth-admin
        type: string_attribute
        string_attribute:
          key: http.route
          values: [/api/v1/auth/*, /admin/*, /api/v1/admin/*]

      # AI 网关心全采样（成本高）
      - name: ai-gateway
        type: string_attribute
        string_attribute:
          key: rpc.service
          values: [gitgit.ai.v1.AIGateway]

      # 健康检查不采样
      - name: drop-health
        type: string_attribute
        string_attribute:
          values: [/healthz, /readyz]
      # 其它按概率
      - name: probabilistic
        type: probabilistic
        probabilistic: { sampling_percentage: 5 }  # 5% 默认
```

### 6.2 业务级强制采样

| 路径 | 采样率 |
|---|---|
| `/healthz`, `/readyz` | 0% |
| `/api/v1/auth/*` | 100% |
| `/admin/*`, `/api/v1/admin/*` | 100% |
| AI 网关调用 | 100% |
| App 升级路径 | 100% |
| 其他 2xx 成功 | 5% |
| 4xx 错误 | 100% |
| 5xx 错误 | 100% |
| 慢请求 (> 1s) | 100% |

## 7. Trace 数据保留

| Tier | 存储 | 保留 | 成本 |
|---|---|---|---|
| Hot | Tempo (in-memory) | 7 天 | 高 |
| Warm | Tempo + ObjectStore (S3) | 30 天 | 中 |
| Cold | S3 Glacier | 1 年 | 低 |

**采样后**:
- 1000 req/s × 5% = 50 trace/s
- 平均每个 trace 50 spans × 2 KB = 100 KB
- 每日: 50 × 86400 × 100 KB = **432 GB/日** (看似大但实际有压缩)
- 压缩 + 索引后: 实际 ~30-50 GB/日
- 7 天 hot: **~250 GB**
- 30 天 warm: **~1 TB**

## 8. 性能影响评估

| 项 | 开销 |
|---|---|
| Span 创建 | ~1 μs / span |
| Trace 序列化 | ~10 μs / trace |
| OTLP batch 发送 | 网络 1-2 MB / 5s |
| 业务 P99 延迟增加 | **< 2 ms** (目标) |

**强制约束**（OBS-REQ-004）:
- 当 OTel Collector 不可达时，**业务不能受影响** — SDK 切到 `NoopTracer`
- 当 buffer 满时，**降级采样**而不是 block 业务

## 9. 关联 OBS-TRC

- **OBS-TRC-001**: ✅ W3C Trace Context 传播
- **OBS-TRC-002**: ✅ HTTP / gRPC / DB / 异步 覆盖
- **OBS-TRC-003**: ✅ Span 命名规约
- **OBS-TRC-004**: ✅ 必填属性
- **OBS-TRC-005**: ✅ Tail sampling
- **OBS-TRC-006**: ✅ 业务级强制采样
- **OBS-TRC-007**: ✅ 数据保留 (Hot/Warm/Cold)
- **OBS-TRC-008**: ✅ 性能影响 < 2ms
- **OBS-TRC-009**: ✅ Collector 不可达降级
- **OBS-TRC-010**: ✅ Trace → Log/Metric 关联
