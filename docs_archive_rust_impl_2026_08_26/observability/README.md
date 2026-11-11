# 可观测性体系 / Observability Platform

> GitGit 平台（AI-Native Engineering Platform）的完整可观测性设计
> **版本**: v1.0
> **日期**: 2026-08-20
> **状态**: Phase 0 文档完成，进入 Phase 1 实施

## 文档索引（16 篇 + ADR + 配置）

| # | 文档 | 主题 | 状态 |
|---|---|---|---|
| 00 | [00-current-state-analysis.md](00-current-state-analysis.md) | 现有系统可观测性现状 | ✅ |
| 01 | [01-architecture.md](01-architecture.md) | 4 层总体架构 | ✅ |
| 02 | [02-metrics.md](02-metrics.md) | Metrics 命名 + 指标体系 | ✅ |
| 03 | [03-logs.md](03-logs.md) | 日志规约 + JSON 字段 | ✅ |
| 04 | [04-tracing.md](04-tracing.md) | Distributed Trace 设计 | ✅ |
| 05 | [05-database-observability.md](05-database-observability.md) | PostgreSQL 深度可观测性 | ✅ |
| 06 | [06-middleware-observability.md](06-middleware-observability.md) | K3s / Git / Event Bus / App Bus | ✅ |
| 07 | [07-dashboard-design.md](07-dashboard-design.md) | Grafana 9 大盘设计 | ✅ |
| 08 | [08-alert-design.md](08-alert-design.md) | Alert + SLO Burn Rate | ✅ |
| 09 | [09-slo-design.md](09-slo-design.md) | SLI / SLO / Error Budget | ✅ |
| 10 | [10-security-design.md](10-security-design.md) | 监控安全 + RBAC + NetworkPolicy | ✅ |
| 11 | [11-deployment-design.md](11-deployment-design.md) | Helm + GitOps + K3s 部署 | ✅ |
| 12 | [12-performance-retention.md](12-performance-retention.md) | 性能影响 + 数据生命周期 | ✅ |
| 13 | [13-implementation-phases.md](13-implementation-phases.md) | 8 阶段分步实施 | ✅ |
| 14 | [14-code-impact.md](14-code-impact.md) | 业务代码改造清单 | ✅ |
| 15 | [15-self-review-v2.md](15-self-review-v2.md) | 12 项自审 + 修订 v2 | ✅ |
| — | [requirements-traceability.md](requirements-traceability.md) | 需求追溯矩阵（280+ ID） | ✅ |
| — | [ADR-0011](../architecture/decisions/0011-observability-platform.md) | 架构决策记录 | ✅ |

## 部署资产（14 YAML）

| 路径 | 用途 |
|---|---|
| [`../../../deploy/observability/base/namespace.yaml`](../../../deploy/observability/base/namespace.yaml) | namespace + ResourceQuota + LimitRange |
| [`../../../deploy/observability/base/serviceaccount.yaml`](../../../deploy/observability/base/serviceaccount.yaml) | SA + ClusterRole (最小权限) |
| [`../../../deploy/observability/base/otel-collector-daemonset.yaml`](../../../deploy/observability/base/otel-collector-daemonset.yaml) | OTel Collector DaemonSet |
| [`../../../deploy/observability/base/otel-collector-gateway.yaml`](../../../deploy/observability/base/otel-collector-gateway.yaml) | OTel Collector Gateway Deployment |
| [`../../../deploy/observability/otel-collector/config.yaml`](../../../deploy/observability/otel-collector/config.yaml) | OTel Collector DaemonSet 配置 |
| [`../../../deploy/observability/otel-collector/config-gateway.yaml`](../../../deploy/observability/otel-collector/config-gateway.yaml) | OTel Collector Gateway 配置 |
| [`../../../deploy/observability/prometheus-rules/prometheus.yaml`](../../../deploy/observability/prometheus-rules/prometheus.yaml) | Prometheus 抓取配置 |
| [`../../../deploy/observability/prometheus-rules/infrastructure.yaml`](../../../deploy/observability/prometheus-rules/infrastructure.yaml) | 节点 / K3s / OTel 告警 |
| [`../../../deploy/observability/prometheus-rules/database.yaml`](../../../deploy/observability/prometheus-rules/database.yaml) | PG / 审计 / 凭证告警 |
| [`../../../deploy/observability/prometheus-rules/application.yaml`](../../../deploy/observability/prometheus-rules/application.yaml) | 业务 RED / 中间件告警 |
| [`../../../deploy/observability/prometheus-rules/slo.yaml`](../../../deploy/observability/prometheus-rules/slo.yaml) | SLO recording + 告警 |
| [`../../../deploy/observability/alertmanager/alertmanager.yaml`](../../../deploy/observability/alertmanager/alertmanager.yaml) | 路由 + 抑制 + 通道 |
| [`../../../deploy/observability/grafana/grafana.ini`](../../../deploy/observability/grafana/grafana.ini) | Grafana 主配置 |
| [`../../../deploy/observability/grafana/provisioning-datasources.yaml`](../../../deploy/observability/grafana/provisioning-datasources.yaml) | DataSource provisioning |
| [`../../../deploy/observability/grafana/provisioning-dashboards.yaml`](../../../deploy/observability/grafana/provisioning-dashboards.yaml) | Dashboard provider |
| [`../../../deploy/observability/network-policies/default-deny.yaml`](../../../deploy/observability/network-policies/default-deny.yaml) | NetworkPolicy default-deny + 白名单 |

## 核心架构

```
┌─────────────────────────────────────────────────┐
│  Layer 1: Application (Rust 业务)               │
│   └─ gitgit-observability (唯一 OTel 入口)      │
└──────────────────┬──────────────────────────────┘
                   │ OTLP gRPC :4317
                   ▼
┌─────────────────────────────────────────────────┐
│  Layer 2: 采集层 (OTel Collector)               │
│   ├─ DaemonSet (每节点 1 副本)                  │
│   └─ Gateway Deployment (2 副本, tail_sampling) │
└──────────────────┬──────────────────────────────┘
                   │ remote_write / OTLP / loki
                   ▼
┌─────────────────────────────────────────────────┐
│  Layer 3: 后端                                  │
│   ├─ Prometheus (TSDB + Alertmanager)           │
│   ├─ Loki (logs)                                │
│   └─ Tempo (traces)                             │
└──────────────────┬──────────────────────────────┘
                   │ PromQL / LogQL / TraceQL
                   ▼
┌─────────────────────────────────────────────────┐
│  Layer 4: 接入 (Grafana 9 大盘 + OIDC)          │
└─────────────────────────────────────────────────┘
```

## 9 大盘索引

| 编号 | 名称 | Audience | 文档 |
|---|---|---|---|
| 00 | System Overview | 所有人 | [07 §3](07-dashboard-design.md#3-00-system-overview) |
| 10 | Infrastructure | SRE | [07 §4](07-dashboard-design.md#4-10-infrastructure) |
| 20 | Kubernetes | SRE | [07 §5](07-dashboard-design.md#5-20-kubernetes) |
| 30 | Application | Backend Dev | [07 §6](07-dashboard-design.md#6-30-application) |
| 40 | Database | SRE / DBA | [07 §7](07-dashboard-design.md#7-40-database) |
| 50 | Middleware | SRE | [07 §8](07-dashboard-design.md#8-50-middleware) |
| 60 | Network | SRE | [07 §9](07-dashboard-design.md#9-60-network) |
| 70 | Performance | SRE | [07 §10](07-dashboard-design.md#10-70-performance) |
| 80 | Security | Security | [07 §11](07-dashboard-design.md#11-80-security) |
| 90 | SLO | PM / EM | [07 §12](07-dashboard-design.md#12-90-slo) |

## 8 阶段实施路线

| Phase | 名称 | 周期 | 文档 |
|---|---|---|---|
| 0 | 现状 + ADR | 1 周 | [13 §2](13-implementation-phases.md#2-phase-0) ✅ |
| 1 | 基础设施监控 | 2 周 | [13 §3](13-implementation-phases.md#3-phase-1) |
| 2 | 应用 Metrics | 2 周 | [13 §4](13-implementation-phases.md#4-phase-2) |
| 3 | 日志集中化 | 1.5 周 | [13 §5](13-implementation-phases.md#5-phase-3) |
| 4 | Distributed Trace | 2 周 | [13 §6](13-implementation-phases.md#6-phase-4) |
| 5 | Dashboard 9 大盘 | 1.5 周 | [13 §7](13-implementation-phases.md#7-phase-5) |
| 6 | Alert 多窗口 | 1.5 周 | [13 §8](13-implementation-phases.md#8-phase-6) |
| 7 | SLO 体系 | 1 周 | [13 §9](13-implementation-phases.md#9-phase-7) |
| 8 | 自动化运维 | 2 周 | [13 §10](13-implementation-phases.md#10-phase-8) |

**总周期**: 12.5-16 周

## 需求 ID 体系（280+）

| 前缀 | 含义 | 数量 |
|---|---|---|
| `OBS-REQ-*` | 顶层需求 | 28 |
| `OBS-MET-*` | Metric 指标 | 130 |
| `OBS-LOG-*` | Log 字段 | 10 |
| `OBS-TRC-*` | Trace Span | 10 |
| `OBS-ALT-*` | Alert 规则 | 30 |
| `OBS-SLO-*` | SLO 定义 | 10 |
| `OBS-SEC-*` | 安全控制 | 12 |
| `OBS-DEP-*` | 部署资源 | 12 |
| `OBS-PERF-*` | 性能需求 | 5 |
| `OBS-RET-*` | 保留策略 | 7 |
| `OBS-IMPL-*` | 实施阶段 | 10 |
| `OBS-REV-*` | 修订项 | 5 |
| `OBS-RISK-*` | 风险 | 7 |
| 子域前缀 | 域内子需求 | ~ 30+ |

详见 [requirements-traceability.md](requirements-traceability.md)

## 关键原则（不可妥协）

1. **零业务侵入**: 业务代码 0 依赖后端 SDK（仅 `gitgit-observability` 1 个 crate）
2. **OTel 标准化**: 业务 → OTel Collector → Backend
3. **业务优先**: 资源不足时降低 telemetry，**不**影响核心服务
4. **数据本地化**: 全部 S3 走内部 MinIO，不用公网
5. **GitOps**: 100% IaC（ArgoCD + Kustomize）
6. **8 阶段分步**: 每阶段可独立回滚
7. **自监控**: OTel / Prom / Loki / Tempo 自身被监控
8. **不开公网**: 监控系统仅 Grafana 通过 Ingress 暴露

## 验证清单

```bash
# 1. anchor 0 破损
powershell -File scripts/check-anchors.ps1

# 2. 0 日文字符（除 workflow.md §4 例外）
python scripts/final-jp-check.py

# 3. Prometheus rules 语法
promtool check rules deploy/observability/prometheus-rules/*.yaml

# 4. Prometheus 配置
promtool check config deploy/observability/prometheus-rules/prometheus.yaml

# 5. OTel Collector 配置
otelcol-contrib validate --config=deploy/observability/otel-collector/config.yaml
otelcol-contrib validate --config=deploy/observability/otel-collector/config-gateway.yaml

# 6. Alertmanager 配置
amtool check-config deploy/observability/alertmanager/alertmanager.yaml
```

## 状态

- ✅ **Phase 0**: 文档 + ADR + 配置完成（2026-08-20）
- 🔜 **Phase 1**: 基础设施监控（helm install kube-prometheus-stack）
- 🔜 **Phase 2**: 应用 Metrics（gitgit-observability crate 实施）

## 修订记录

| 日期 | 修订 | 修订项 | 文档 |
|---|---|---|---|
| 2026-08-20 | v1.0 初稿 | 全部 16 篇 + ADR + 14 YAML | — |
| 待 v2 | 自审修订 10 项 | 见 15-self-review-v2 §2 | 15 |

## 关联文档

- 设计根: [`../design/README.md`](../design/README.md)
- 架构决策: [`../architecture/decisions/0011-observability-platform.md`](../architecture/decisions/0011-observability-platform.md)
- 过程: [`../process/workflow.md`](../process/workflow.md)
- 部署资产: [`../../deploy/observability/`](../../deploy/observability/)
