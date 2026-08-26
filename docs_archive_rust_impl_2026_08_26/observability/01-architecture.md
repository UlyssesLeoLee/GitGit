# Observability 总体架构 / Observability Architecture

> **关联 OBS-REQ**: OBS-REQ-001（建立统一 Observability Architecture）/ OBS-REQ-002（零业务侵入）/ OBS-REQ-003（Metric/Log/Trace 关联）
> **关联设计**: §10-observability.md（详细设计基础）
> **目标**: 业务代码只关心"创建指标 / 记录事件 / 创建 Span"，不关心数据发送到哪里

## 1. 分层架构

```
┌────────────────────────────────────────────────────────────────────────────┐
│                       Application Layer (Rust 业务代码)                      │
│                                                                             │
│   ┌─────────────────────────────────────────────────────────────────────┐ │
│   │              Observability SDK (gitgit-observability)               │ │
│   │                                                                     │ │
│   │   tracing::instrument!()  ──►  自动 Span + context                  │ │
│   │   metrics::counter!()     ──►  Counter / Histogram / Gauge        │ │
│   │   info!() / warn!() / error!() ──► 结构化日志 + trace_id 注入     │ │
│   │                                                                     │ │
│   │   ★ 业务代码 0 依赖 backend SDK（Prometheus / Loki / Tempo）         │ │
│   └─────────────────────────────────────────────────────────────────────┘ │
│                                  │                                         │
│                                  │ OpenTelemetry Protocol (OTLP)            │
│                                  ▼                                         │
└────────────────────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────────────────────┐
│                      Observability Infrastructure                          │
│                                                                             │
│   ┌───────────────────────┐      ┌──────────────────────┐                │
│   │  OTel Collector       │      │  admin_audit ───┐     │                │
│   │  (Deployment)         │      │  哈希链 + wal2json│     │                │
│   │                       │      └──────────────────────┘                │
│   │  Receivers:           │                                                │
│   │  • OTLP gRPC :4317    │                                                │
│   │  • OTLP HTTP :4318    │                                                │
│   │  • Prom :9090 (scrape) │                                                │
│   │  • pg_exporter :9187  │                                                │
│   │  • node_exporter :9100│                                                │
│   │  • kubelet :10250     │                                                │
│   │                       │                                                │
│   │  Processors:          │                                                │
│   │  • batch              │                                                │
│   │  • memory_limiter     │                                                │
│   │  • tail_sampling      │                                                │
│   │  • resource_detection │                                                │
│   │  • attributes/processor│                                                │
│   │  • filter (drop noisy)│                                                │
│   │  • transform (redact) │                                                │
│   │                       │                                                │
│   │  Exporters:           │                                                │
│   │  • prometheusremotewrite│                                                │
│   │  • otlphttp (Tempo)   │                                                │
│   │  • loki (logs)        │                                                │
│   │  • webhook (alerts)   │                                                │
│   └───────────────────────┘                                                │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────────────────────┐
│                         Backend Storage Layer                              │
│                                                                             │
│   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐  │
│   │ Prometheus  │   │     Loki    │   │    Tempo    │   │   Grafana   │  │
│   │  (Metrics)  │   │    (Logs)   │   │  (Traces)   │   │ (Visualize) │  │
│   │             │   │             │   │             │   │             │  │
│   │  30d ret.   │   │  30d hot    │   │  Head 7d    │   │  SSO/OIDC   │  │
│   │  TSDB       │   │  + S3 cold  │   │  Tail 30d   │   │  RBAC       │  │
│   │  RemoteWrite│   │             │   │  ObjectStore│   │  Alert mgr  │  │
│   └─────────────┘   └─────────────┘   └─────────────┘   └─────────────┘  │
│                                                                             │
│   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐                      │
│   │ AlertManager│   │  PG main    │   │  S3 / MinIO  │                      │
│   │  (路由)     │   │ (业务)      │   │ (cold store)│                      │
│   └─────────────┘   └─────────────┘   └─────────────┘                      │
│                                                                             │
└────────────────────────────────────────────────────────────────────────────┘
```

## 2. 数据流

### 2.1 Metrics 路径

```
业务代码 (metrics::counter!())
    │
    ▼
OpenTelemetry SDK (in-process)
    │
    │  OTLP gRPC 每 10s batch
    ▼
OTel Collector (DaemonSet)
    │
    │  Prometheus RemoteWrite
    ▼
Prometheus (StatefulSet)
    │
    │  PromQL query
    ▼
Grafana (Dashboard 渲染)
    │
    │  Alert rule 评估
    ▼
AlertManager → Slack / PagerDuty / Email
```

### 2.2 Logs 路径

```
业务代码 (tracing::info!())
    │
    ▼
tracing-subscriber (JSON formatter)
    │
    │  stdout (容器) → node journald / docker logs
    ▼
Kubernetes DaemonSet (Fluentbit / Vector)
    │
    │  HTTP push
    ▼
Loki (Distributor → Ingester → Store)
    │
    │  LogQL query
    ▼
Grafana (Log panel)
```

### 2.3 Traces 路径

```
业务代码 (#[tracing::instrument])
    │
    ▼
tracing-opentelemetry layer
    │
    │  OTLP gRPC batch (tail-sampled)
    ▼
OTel Collector
    │
    │  OTLP HTTP
    ▼
Tempo (Distributor → Ingester → Block)
    │
    │  TraceQL query
    ▼
Grafana (Trace panel)
```

### 2.4 Trace → Log → Metric 关联

```
Trace ID + Span ID (W3C Trace Context)
    │
    ├──► 注入到 log (tracing-subscriber 提取)
    │
    └──► 注入到 metric exemplar (histogram_bucket)
             │
             ▼
        Grafana: 点 metric → 跳转 trace → 点 span → 跳转 logs
```

## 3. 部署位置

**Namespace**: `observability`（独立 namespace，与业务隔离）

| 组件 | 类型 | Replica | 资源 |
|---|---|---|---|
| OTel Collector | DaemonSet (每 Node 1 个) | N (节点数) | 200m / 256Mi |
| OTel Collector (Gateway) | Deployment | 2 | 500m / 512Mi |
| Prometheus | StatefulSet | 1 (MVP) / 2 (V1+) | 1 / 4Gi |
| Loki (Distributor) | Deployment | 2 | 200m / 256Mi |
| Loki (Ingester) | StatefulSet | 1 | 500m / 512Mi |
| Loki (Querier) | Deployment | 2 | 500m / 1Gi |
| Tempo (Distributor) | Deployment | 2 | 200m / 256Mi |
| Tempo (Ingester) | StatefulSet | 1 | 500m / 1Gi |
| Tempo (Querier) | Deployment | 2 | 500m / 1Gi |
| Grafana | Deployment | 1 (MVP) / 2 (V1+) | 200m / 256Mi |
| AlertManager | Deployment | 1 | 100m / 128Mi |
| MinIO (cold store) | StatefulSet | 1 (MVP) / 4 (V1+) | 500m / 1Gi |

**MVP (Local-First)**: Prometheus + Loki + Tempo + Grafana 全部 binary 部署（不上 K3s），使用 sqlite / local file backend
**V1+ Cloud**: 上述 K3s 部署

## 4. 命名约定

| 项 | 规约 | 例 |
|---|---|---|
| Service name (label) | `service.name` | `gitgit-server`, `gitgit-admin`, `postgresql`, `redis` |
| Service version | `service.version` (semver) | `0.1.0` |
| Environment | `deployment.environment` | `local`, `staging`, `prod` |
| K8s namespace | `k8s.namespace.name` | `gitgit`, `observability` |
| Pod name | `k8s.pod.name` | `gitgit-server-7f9b-abcde` |
| Container runtime | `container.runtime` | `docker`, `containerd`, `cri-o` |
| Cloud provider | `cloud.provider` / `cloud.region` | `self-hosted`, `cn-north-1` |

## 5. Cardinality 控制

| 类型 | 低 Cardinality (安全) | 高 Cardinality (危险) |
|---|---|---|
| HTTP path | `/api/v1/repos/{id}` | `/api/v1/repos/{id}/files/{path}` |
| User ID | `user_id` (UUID, ~10k 唯一) | `email` (PII) |
| Status code | `200`, `404`, `500` | 完整 URL |
| Service | `gitgit-server` | per-request value |
| Method | `GET`, `POST` | 完整 HTTP headers |

**原则**: 业务 label cardinality < 100；如需高 cardinality 用 trace 而非 metric。

## 6. 关联 OBS-REQ

- **OBS-REQ-001**: ✅ 统一架构（4 层）
- **OBS-REQ-002**: ✅ 业务 0 依赖（仅 gitgit-observability）
- **OBS-REQ-003**: ✅ Metric/Log/Trace 关联（trace_id 注入）
- **OBS-REQ-004**: ✅ 性能基线预算（§14 性能评估）
- **OBS-REQ-005**: ✅ 数据生命周期（§15）
- **OBS-REQ-006**: ✅ 安全设计（§16）
- **OBS-REQ-007**: ✅ 部署设计（§17）
- **OBS-REQ-008**: ✅ 分阶段实施（§18）
