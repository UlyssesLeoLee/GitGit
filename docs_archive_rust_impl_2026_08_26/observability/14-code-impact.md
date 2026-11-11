# 代码影响分析 / Code Impact Analysis

> **关联 OBS-REQ**: OBS-REQ-025（代码影响） / OBS-REQ-026（最小侵入）
> **关联设计**: [`../design/detailed-design/`](../design/detailed-design/)（现有 14 文件） / [`11-deployment-design.md`](11-deployment-design.md) / [`13-implementation-phases.md`](13-implementation-phases.md)
> **目标**: 明确每个 crate / 服务的改造点 + 评级 + 风险点，**禁止**误伤业务逻辑

## 1. 改造总览

### 1.1 评级标准

| 等级 | 含义 | 审查要求 |
|---|---|---|
| **L0 (None)** | 零改动 | — |
| **L1 (Low)** | Cargo.toml 加依赖 / 配置 | 1 人 review |
| **L2 (Medium)** | main.rs 加 init / 业务模块加 instrument 宏 | 2 人 review |
| **L3 (High)** | 关键路径埋点 / 业务逻辑调整 | SRE + 模块 owner 双 review |

### 1.2 各 Crate 影响矩阵

| Crate | 影响等级 | 改动量 | 关键改动 |
|---|---|---|---|
| `gitgit-errors` | **L1** | + 30 行 | tracing span 关联 |
| `gitgit-config` | **L1** | + 50 行 | observability 段 |
| `gitgit-observability` | **新增** | + 600 行 | SDK 封装（**唯一** OTel 依赖点） |
| `gitgit-proto` | **L1** | + 10 行 | tracing interceptor |
| `gitgit-core` | **L2** | + 40 行 | 业务埋点（5 原语） |
| `gitgit-graph` | **L2** | + 30 行 | 递归 CTE 埋点 |
| `gitgit-policy` | **L2** | + 40 行 | 决策缓存埋点 |
| `gitgit-ai` | **L3** | + 100 行 | AI 网关埋点（成本敏感） |
| `gitgit-agent` | **L3** | + 80 行 | Run 状态埋点 |
| `gitgit-app` | **L2** | + 50 行 | Wasm 调用埋点 |
| `gitgit-git` | **L2** | + 60 行 | Git 协议埋点 |
| `gitgit-server` | **L3** | + 100 行 | HTTP / gRPC middleware + init |
| `gitgit-admin` | **L3** | + 100 行 | 同上（独立鉴权） |
| `gitgit-cli` | **L1** | + 20 行 | 可选上报 |

> **总改动**: ~ 1300 行新增（不含 gitgit-observability 自身）

## 2. 新增依赖（Cargo.toml）

### 2.1 workspace.dependencies 新增

```toml
# OpenTelemetry 核心
opentelemetry = { version = "0.27", features = ["trace", "metrics", "logs"] }
opentelemetry_sdk = { version = "0.27", features = ["rt-tokio", "trace", "metrics", "logs"] }
opentelemetry-otlp = { version = "0.27", default-features = false, features = ["grpc-tonic", "trace", "metrics", "logs"] }
opentelemetry-semantic-conventions = "0.16"

# tracing 生态
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json", "fmt"] }
tracing-opentelemetry = "0.28"
tracing-appender = "0.2"

# HTTP 自动埋点
tower = { version = "0.5", features = ["util"] }
tower-http = { version = "0.6", features = ["trace", "opentelemetry"] }

# gRPC 自动埋点
tonic = { version = "0.12", features = ["opentelemetry"] }
```

### 2.2 强约束

- **禁止 GPL/AGPL**: `cargo-deny` 必须通过
- **lockfile**: `Cargo.lock` 必带
- **MSRV**: 兼容 1.75
- **feature 最小化**: 避免引入 `full` feature

## 3. 各 Crate 详细改动

### 3.1 gitgit-observability（新增 crate）

> **唯一** OTel 直接依赖点。其他 crate 仅依赖本 crate。

#### `crates/gitgit-observability/Cargo.toml`

```toml
[package]
name = "gitgit-observability"
version = "0.1.0"
edition = "2021"
license = "Apache-2.0"
description = "统一 OTel SDK 封装，业务侧零 OTel 依赖"

[dependencies]
# workspace 依赖
opentelemetry = { workspace = true }
opentelemetry_sdk = { workspace = true }
opentelemetry-otlp = { workspace = true }
opentelemetry-semantic-conventions = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
tracing-opentelemetry = { workspace = true }
tracing-appender = { workspace = true }

# 内部依赖
gitgit-errors = { path = "../gitgit-errors" }
gitgit-config = { path = "../gitgit-config" }

# 第三方
async-trait = "0.1"
once_cell = "1.19"
```

#### `crates/gitgit-observability/src/lib.rs`

```rust
//! 统一可观测性 SDK 入口。
//! 业务 crate 仅依赖本 crate，不直接 import opentelemetry。

pub mod config;
pub mod meter;
pub mod tracer;
pub mod logger;
pub mod error_instrument;

pub use config::ObservabilityConfig;
pub use meter::{init_meter, meter, shutdown_meter};
pub use tracer::{init_tracer, attach_tracing, shutdown_tracer};
pub use logger::{init_logger, json_layer};
```

#### `crates/gitgit-observability/src/meter.rs`

```rust
use opentelemetry::{global, metrics::Meter, KeyValue};
use opentelemetry_otlp::{WithExportConfig, MetricExporter};
use opentelemetry_sdk::{
    metrics::{PeriodicReader, SdkMeterProvider},
    runtime, Resource,
};
use std::time::Duration;

use crate::config::ObservabilityConfig;
use crate::error_instrument::record_error;

pub fn init_meter(cfg: &ObservabilityConfig) -> SdkMeterProvider {
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(&cfg.otlp_endpoint)
        .with_timeout(Duration::from_secs(cfg.export_timeout_secs));

    let reader = PeriodicReader::builder(exporter, runtime::Tokio)
        .with_interval(Duration::from_secs(cfg.export_interval_secs))
        .build();

    let provider = SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(resource(cfg))
        .build();

    global::set_meter_provider(provider.clone());
    provider
}

pub fn meter() -> Meter {
    global::meter("gitgit")
}

pub fn shutdown_meter(provider: SdkMeterProvider) {
    let _ = provider.shutdown();
}

fn resource(cfg: &ObservabilityConfig) -> Resource {
    Resource::new(vec![
        KeyValue::new("service.name", cfg.service_name.clone()),
        KeyValue::new("service.version", cfg.service_version.clone()),
        KeyValue::new("service.namespace", "gitgit"),
        KeyValue::new("deployment.environment", cfg.environment.clone()),
    ])
}

pub fn record_error(err: &dyn std::error::Error, span: &str) {
    meter()
        .u64_counter("errors_total")
        .build()
        .add(1, &[
            KeyValue::new("error.type", err.to_string()),
            KeyValue::new("span.name", span.to_string()),
        ]);
}
```

#### `crates/gitgit-observability/src/tracer.rs`

```rust
use opentelemetry::{global, trace::TracerProvider as _};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    trace::{self as sdktrace, Sampler, TracerProvider as SdkTracerProvider},
    Resource, runtime,
};
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::config::ObservabilityConfig;

pub fn init_tracer(cfg: &ObservabilityConfig) -> SdkTracerProvider {
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(&cfg.otlp_endpoint)
        .with_timeout(Duration::from_secs(cfg.export_timeout_secs));

    let sampler = Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(cfg.trace_sample_ratio)));

    let provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(
            sdktrace::Config::default()
                .with_sampler(sampler)
                .with_resource(resource(cfg))
        )
        .install_batch(runtime::Tokio)
        .expect("failed to install tracer");

    global::set_tracer_provider(provider.clone());
    provider
}

pub fn attach_tracing(cfg: &ObservabilityConfig) {
    let tracer = global::tracer_provider().tracer(cfg.service_name.clone());
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&cfg.log_level));

    tracing_subscriber::registry()
        .with(filter)
        .with(otel_layer)
        .with(json_layer())
        .init();
}

pub fn shutdown_tracer(provider: SdkTracerProvider) {
    let _ = provider.shutdown();
}

fn resource(cfg: &ObservabilityConfig) -> Resource {
    Resource::new(vec![
        opentelemetry::KeyValue::new("service.name", cfg.service_name.clone()),
        opentelemetry::KeyValue::new("service.version", cfg.service_version.clone()),
        opentelemetry::KeyValue::new("deployment.environment", cfg.environment.clone()),
    ])
}
```

#### `crates/gitgit-observability/src/logger.rs`

```rust
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;

/// JSON 格式化日志层，注入 trace_id / span_id。
pub fn json_layer<S>() -> impl tracing_subscriber::Layer<S>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    tracing_subscriber::fmt::layer()
        .json()
        .with_current_span(true)
        .with_span_list(false)
        .with_target(true)
        .with_file(false)
        .with_line_number(false)
        .with_thread_ids(false)
        .with_thread_names(false)
}

pub fn init_logger() {
    let _ = tracing_log::LogTracer::init();
    // 实际 init 在 attach_tracing 内完成
}
```

#### `crates/gitgit-observability/src/config.rs`

```rust
use serde::Deserialize;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct ObservabilityConfig {
    pub service_name: String,
    pub service_version: String,
    pub environment: String,
    pub otlp_endpoint: String,
    pub export_interval_secs: u64,
    pub export_timeout_secs: u64,
    pub trace_sample_ratio: f64,
    pub log_level: String,
    pub enable_metrics: bool,
    pub enable_traces: bool,
    pub enable_logs: bool,
}

impl ObservabilityConfig {
    pub fn from_env() -> Self {
        Self {
            service_name: env::var("SERVICE_NAME").unwrap_or_else(|_| "gitgit".into()),
            service_version: env::var("SERVICE_VERSION").unwrap_or_else(|_| "0.1.0".into()),
            environment: env::var("ENVIRONMENT").unwrap_or_else(|_| "development".into()),
            otlp_endpoint: env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
                .unwrap_or_else(|_| "http://otel-collector.observability.svc:4317".into()),
            export_interval_secs: env::var("OTEL_METRIC_EXPORT_INTERVAL")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
            export_timeout_secs: env::var("OTEL_EXPORT_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
            trace_sample_ratio: env::var("OTEL_TRACE_SAMPLE_RATIO")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.05),
            log_level: env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
            enable_metrics: env::var("OTEL_METRICS_ENABLED")
                .map(|s| s == "true")
                .unwrap_or(true),
            enable_traces: env::var("OTEL_TRACES_ENABLED")
                .map(|s| s == "true")
                .unwrap_or(true),
            enable_logs: env::var("OTEL_LOGS_ENABLED")
                .map(|s| s == "true")
                .unwrap_or(true),
        }
    }
}
```

### 3.2 gitgit-server（业务主服务）改动

#### `crates/gitgit-server/Cargo.toml`

```toml
[dependencies]
gitgit-observability = { path = "../gitgit-observability" }
gitgit-errors = { path = "../gitgit-errors" }
# 移除直接的 opentelemetry 依赖
```

#### `crates/gitgit-server/src/main.rs` 改动

```rust
use gitgit_observability::{
    config::ObservabilityConfig, meter::*, tracer::*,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = ObservabilityConfig::from_env();

    // 1. 初始化可观测性
    let meter_provider = if cfg.enable_metrics {
        Some(init_meter(&cfg))
    } else {
        None
    };
    let tracer_provider = if cfg.enable_traces {
        Some(init_tracer(&cfg))
    } else {
        None
    };
    if cfg.enable_traces {
        attach_tracing(&cfg);
    }

    // 2. 业务启动
    let app = build_app(&cfg).await?;

    // 3. 优雅关闭
    tokio::select! {
        r = app.serve() => { r?; }
        _ = tokio::signal::ctrl_c() => {}
    }

    // 4. shutdown
    if let Some(p) = meter_provider { shutdown_meter(p); }
    if let Some(p) = tracer_provider { shutdown_tracer(p).await; }

    Ok(())
}

async fn build_app(cfg: &ObservabilityConfig) -> Result<axum::Router, Box<dyn std::error::Error>> {
    use tower_http::trace::TraceLayer;
    use opentelemetry::global;
    use opentelemetry_http::HeaderExtractor;

    let app = axum::Router::new()
        .route("/healthz", axum::routing::get(healthz))
        .route("/readyz", axum::routing::get(readyz))
        .nest("/api/v1", api_routes())
        // 关键：OTel HTTP middleware（自动生成 span + trace_id）
        .layer(TraceLayer::new_for_http())
        .layer(tower_http::opentelemetry::OpenTelemetryLayer::new());

    Ok(app)
}
```

### 3.3 gitgit-core（5 原语）改动

```rust
// crates/gitgit-core/src/node/repo.rs
use gitgit_observability::meter;
use opentelemetry::KeyValue;

pub struct NodeRepository {
    pool: PgPool,
}

impl NodeRepository {
    #[tracing::instrument(skip(self, input), fields(tenant = %input.tenant_id, node_type = %input.node_type))]
    pub async fn create(&self, input: CreateNodeInput) -> Result<Node, Error> {
        let _timer = meter()
            .f64_histogram("db_operation_duration_seconds")
            .build()
            .record(0.0, &[KeyValue::new("operation", "node.create")]);

        // 业务逻辑（不变）
        let node = sqlx::query_as!(...)
            .fetch_one(&self.pool)
            .await?;

        // 业务指标
        meter()
            .u64_counter("gitgit_node_created_total")
            .build()
            .add(1, &[
                KeyValue::new("type", input.node_type.clone()),
                KeyValue::new("tenant", input.tenant_id.clone()),
            ]);

        Ok(node)
    }
}
```

### 3.4 gitgit-ai（AI 网关）改动

> **特殊**: AI 埋点含**成本指标**（输入 token / 输出 token / 美元成本）。

```rust
// crates/gitgit-ai/src/gateway.rs
use gitgit_observability::meter;
use opentelemetry::KeyValue;
use std::time::Instant;

#[tracing::instrument(skip(self, req), fields(ai.vendor = %req.vendor, ai.model = %req.model))]
pub async fn call(&self, req: AiRequest) -> Result<AiResponse, Error> {
    let start = Instant::now();
    let result = self.client.post(&req.vendor.endpoint())
        .headers(/*...*/)
        .json(&req.body)
        .send()
        .await;

    let duration = start.elapsed().as_secs_f64();

    match result {
        Ok(resp) => {
            let body: AiResponse = resp.json().await?;
            // 业务指标
            meter().u64_counter("gitgit_ai_gateway_requests_total").build().add(
                1, &[
                    KeyValue::new("vendor", req.vendor.to_string()),
                    KeyValue::new("model", req.model.clone()),
                    KeyValue::new("result", "success"),
                ]
            );
            meter().f64_histogram("gitgit_ai_gateway_duration_seconds").build().record(
                duration, &[
                    KeyValue::new("vendor", req.vendor.to_string()),
                    KeyValue::new("model", req.model.clone()),
                ]
            );
            // 成本指标
            meter().u64_histogram("gitgit_ai_input_tokens").build().record(
                body.usage.input_tokens as u64, &[KeyValue::new("model", req.model.clone())]
            );
            meter().u64_histogram("gitgit_ai_output_tokens").build().record(
                body.usage.output_tokens as u64, &[KeyValue::new("model", req.model.clone())]
            );
            meter().f64_histogram("gitgit_ai_cost_usd").build().record(
                body.cost_usd, &[KeyValue::new("vendor", req.vendor.to_string())]
            );
            Ok(body)
        }
        Err(e) => {
            meter().u64_counter("gitgit_ai_gateway_requests_total").build().add(
                1, &[
                    KeyValue::new("vendor", req.vendor.to_string()),
                    KeyValue::new("model", req.model.clone()),
                    KeyValue::new("result", "error"),
                ]
            );
            Err(e.into())
        }
    }
}
```

### 3.5 gitgit-app（Wasm 沙箱）改动

```rust
// crates/gitgit-app/src/hostcall.rs
use gitgit_observability::meter;
use opentelemetry::KeyValue;

#[tracing::instrument(skip(state), fields(app.id = %app_id, app.call = %call_name))]
pub fn invoke(state: &AppState, app_id: &str, call_name: &str, args: &[u8]) -> Result<Vec<u8>, Error> {
    // capability check
    if !state.capability_table.allowed(app_id, call_name) {
        meter().u64_counter("gitgit_app_bus_capability_denied_total").build().add(
            1, &[
                KeyValue::new("app", app_id.to_string()),
                KeyValue::new("call", call_name.to_string()),
                KeyValue::new("capability", call_name.to_string()),
            ]
        );
        return Err(Error::PermissionDenied);
    }

    let result = state.runtime.call(app_id, call_name, args)?;
    Ok(result)
}
```

### 3.6 gitgit-git（Git 协议）改动

```rust
// crates/gitgit-git/src/write.rs
use gitgit_observability::meter;
use opentelemetry::KeyValue;

#[tracing::instrument(skip(self), fields(git.repo = %repo, git.verb = "push"))]
pub async fn push(&self, repo: &str, user: &str) -> Result<(), Error> {
    let start = std::time::Instant::now();

    // spawn git
    let output = tokio::process::Command::new("git")
        .arg("receive-pack")
        .arg(&self.repo_path(repo))
        .output()
        .await?;

    let duration = start.elapsed().as_secs_f64();

    let status_class = if output.status.success() { "2xx" } else { "5xx" };

    meter().u64_counter("gitgit_git_protocol_requests_total").build().add(
        1, &[
            KeyValue::new("protocol", "smart-http"),
            KeyValue::new("verb", "push"),
            KeyValue::new("status_class", status_class),
        ]
    );
    meter().f64_histogram("gitgit_git_protocol_request_duration_seconds").build().record(
        duration, &[
            KeyValue::new("protocol", "smart-http"),
            KeyValue::new("verb", "push"),
        ]
    );

    if !output.status.success() {
        return Err(Error::Git(output.stderr));
    }
    Ok(())
}
```

## 4. 配置变更

### 4.1 config/config.example.toml 新增

```toml
[observability]
service_name = "gitgit-server"
service_version = "0.1.0"
environment = "production"
otlp_endpoint = "http://otel-collector.observability.svc:4317"
export_interval_secs = 5
export_timeout_secs = 10
trace_sample_ratio = 0.05
log_level = "info"

# 降级开关
enable_metrics = true
enable_traces = true
enable_logs = true
```

### 4.2 K8s Deployment env 注入

```yaml
spec:
  template:
    spec:
      containers:
        - name: gitgit-server
          env:
            - name: OTEL_EXPORTER_OTLP_ENDPOINT
              value: "http://otel-collector.observability.svc:4317"
            - name: OTEL_METRIC_EXPORT_INTERVAL
              value: "5"
            - name: OTEL_TRACE_SAMPLE_RATIO
              value: "0.05"
            - name: RUST_LOG
              value: "info,gitgit_server=info,gitgit_core=info,sqlx=warn"
            - name: OTEL_METRICS_ENABLED
              value: "true"
            - name: OTEL_TRACES_ENABLED
              value: "true"
            - name: OTEL_LOGS_ENABLED
              value: "true"
            - name: SERVICE_NAME
              value: "gitgit-server"
            - name: SERVICE_VERSION
              value: "0.1.0"
            - name: ENVIRONMENT
              value: "production"
```

## 5. 数据库变更

### 5.1 pg_exporter 专用账号

```sql
-- migrations/0015-pg-exporter-role.sql
CREATE ROLE pg_exporter LOGIN PASSWORD '...' NOSUPERUSER NOCREATEDB NOCREATEROLE;
GRANT pg_read_all_stats TO pg_exporter;
GRANT CONNECT ON DATABASE gitgit TO pg_exporter;
GRANT USAGE ON SCHEMA pg_catalog TO pg_exporter;
-- 自定义查询权限
GRANT SELECT ON ALL TABLES IN SCHEMA tenant TO pg_exporter;
GRANT SELECT ON ALL TABLES IN SCHEMA coordination TO pg_exporter;
```

### 5.2 admin_audit 验证任务

> 监控告警依赖 `gitgit_audit_hash_chain_ok` 指标。

```sql
-- 已存在: admin_audit (ADR-0008)
-- 新增: 每 5min 跑一次的哈希链验证（外部 CronJob）
```

## 6. 部署 / 配置变更

### 6.1 新增资源

| 资源 | 路径 | 数量 |
|---|---|---|
| Helm chart | `deploy/observability/` | 1 |
| Kustomize base | `deploy/observability/base/` | 1 |
| Kustomize overlay | `deploy/observability/overlays/{prod,staging}` | 2 |
| ArgoCD Application | `deploy/argocd/observability.yaml` | 1 |
| NetworkPolicy | `deploy/observability/base/network-policies/` | 7 |
| Secret（git） | `deploy/observability/base/secrets/` | 3 |

### 6.2 现有资源变更

| 资源 | 变更 |
|---|---|
| `gitgit-server` Deployment | + env 注入（8 个） |
| `gitgit-admin` Deployment | + env 注入（8 个） |
| `gitgit-agent` Deployment | + env 注入（8 个） |
| `pg_exporter` Deployment | 新建 |

## 7. 测试影响

### 7.1 现有测试

| 测试类型 | 是否需要修改 |
|---|---|
| 单元测试 | ❌ 无（OTel init 是 main 入口） |
| 集成测试 | ⚠ 需新增 mock OTel collector |
| E2E 测试 | ⚠ 需新增 telemetry 验证 |
| 性能测试 | ⚠ **必须**新增（验证开销 < 3%） |

### 7.2 新增测试

| 测试 | 位置 | 优先级 |
|---|---|---|
| OTel init 单元测试 | `crates/gitgit-observability/tests/` | P0 |
| 业务埋点集成测试 | `crates/gitgit-server/tests/observability.rs` | P0 |
| Trace 传播测试 | `tests/e2e/trace_propagation.rs` | P0 |
| Log 脱敏测试 | `tests/security/log_redaction.rs` | P0 |
| 性能基准（OTel 开/关） | `benches/observability_overhead.rs` | P0 |
| 故障演练（OTel down） | `tests/chaos/otel_down.rs` | P1 |

## 8. 风险与缓解

| 风险 | 概率 | 影响 | 缓解 |
|---|---|---|---|
| OTel SDK 升级 breaking change | 中 | 中 | 锁定 minor version + 季度评估 |
| Cardinality 爆炸（业务 label 失控） | 中 | 高 | 强制白名单 + Cardinality 告警 |
| 业务代码 OTel 调用错误 | 中 | 中 | Code review + Linter |
| 采样率设置错误 | 中 | 中 | 启动时校验 + 监控采样率 |
| 主线程 OTel 调用阻塞 | 低 | 高 | 强制 BatchSpan + async export |
| 敏感数据进入 trace | 中 | 高 | transform processor + 静态扫描 |
| Prometheus / Loki / Tempo 故障 | 中 | 中 | 业务侧不阻塞（buffer 5min） |
| 升级需要停机 | 低 | 中 | 全部设计为无状态 + 滚动 |

## 9. 业务侵入度自评

| 业务代码 | 改动模式 | 是否侵入 |
|---|---|---|
| 业务 main.rs | + 5 行 init | **无侵入** |
| 业务函数埋点 | + 1 行 `#[instrument]` | **最小侵入**（仅元数据） |
| 业务错误处理 | 调用 `record_error` | **无侵入**（只读） |
| 业务计数器 | 偶尔 `meter().counter().add(1, ...)` | **可忽略** |
| 业务日志 | 不变（已用 `tracing`） | **零侵入** |

> **结论**: 业务代码**不感知** telemetry 上报目标，**仅**显式声明要测什么。

## 10. Review 清单

> 每个 PR 提交前 review 清单。

- [ ] OTel 调用**不**出现在业务热路径同步代码
- [ ] 业务 label 符合白名单（见 [`02-metrics.md`](02-metrics.md) §5）
- [ ] 敏感数据**未**出现在 span attribute / log field
- [ ] 业务函数 `#[instrument]` 跳过 `skip(self)` / `skip(state)`
- [ ] 单元测试覆盖 init / shutdown
- [ ] `cargo deny` 通过
- [ ] benchmark 性能开销 < 5%

## 11. 关联文档

- 上游: [`01-architecture.md`](01-architecture.md) §1 4 层架构
- 上游: [`11-deployment-design.md`](11-deployment-design.md)
- 上游: [`13-implementation-phases.md`](13-implementation-phases.md)
- 下游: [`15-self-review-v2.md`](15-self-review-v2.md) §7 代码复盘

## 12. 需求 ID 索引

| 需求 ID | 标题 | 优先级 |
|---|---|---|
| OBS-REQ-025 | 代码影响 | P0 |
| OBS-REQ-026 | 最小侵入 | P0 |
| CODE-REQ-001 | gitgit-observability 封装 | P0 |
| CODE-REQ-002 | 业务 crate 接入 | P0 |
| CODE-REQ-003 | AI 网关成本埋点 | P1 |
| CODE-REQ-004 | Git 协议埋点 | P0 |
| CODE-REQ-005 | App Bus 埋点 | P0 |
| CODE-REQ-006 | K8s env 注入 | P0 |
| CODE-REQ-007 | pg_exporter 账号 | P0 |
| CODE-REQ-008 | 新增测试 | P0 |
| CODE-REQ-009 | 性能基准 | P0 |
| CODE-REQ-010 | Review 清单 | P0 |
