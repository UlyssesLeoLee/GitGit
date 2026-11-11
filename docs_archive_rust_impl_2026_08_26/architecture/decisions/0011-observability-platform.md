# ADR-0011: 可观测性平台采用 OpenTelemetry + Prometheus + Loki + Tempo + Grafana

> **Status**: Accepted
> **Date**: 2026-08-20
> **Deciders**: SA + SRE Lead + TL
> **Consulted**: SEC + DBA + Backend Dev
> **Informed**: 全体

## Context and Problem Statement

可观测性现状分析（[`../../observability/00-current-state-analysis.md`](../../observability/00-current-state-analysis.md)）指出:

- 平台由 14 个 Rust crate / 3 binary 组成（K3s 部署，14 schema PG）
- 通信方式 7 种（HTTP / gRPC / WebSocket / MCP / PG / shell `git` / Wasm hostcall）
- 当前**无任何集中监控**，靠开发者本地调试 + 业务日志（`println!`）
- 团队规模假设 1 SRE + 多 Backend Dev，**无独立监控栈**工程资源

> **核心问题**: 在**业务优先**前提下，**最小侵入**地建立可观测性体系，覆盖 8 大目标（实时 / 异常发现 / 根因 / 性能 / 容量 / 自动化 / 可靠性 / 安全）。

## Decision Drivers

- **零业务侵入**: 业务代码不直接依赖任何后端 SDK（Prom / Loki / Tempo）
- **业界标准**: OTel 是 CNCF GA 项目，避免绑定单一厂商
- **K3s 友好**: 全部组件 Helm chart 成熟，资源可控
- **GitOps**: 100% IaC 化（ArgoCD / Kustomize）
- **合规要求**: 监控数据**本地化**（不用公网 S3 / 公有云监控）
- **开源优先**: 不引入 SaaS 监控（如 Datadog / NewRelic）
- **已实施约束**: 强约束（ADR-0003 不引入 Redis/NATS/Memgraph/Valkey；ADR-0008 哈希链审计）
- **成本可控**: 内部资源 ≤ 5 核 CPU / 16 GB / 1.8 TB/月
- **可维护**: SRE 1 人可独立运维

## Considered Options

### Option 1: OpenTelemetry + Prometheus + Loki + Tempo + Grafana（OGSTG）

> 本方案即选定方案。

- **优点**:
  - OTel 标准化数据格式，业务代码不绑死
  - Prometheus 业界事实标准，K8s 生态默认
  - Loki / Tempo 与 Grafana 同厂商，集成度最高
  - Grafana 9 大盘 + Alertmanager + SLO 体系成熟
  - 全部 CNCF / 开源，无厂商绑定
  - 业务侧**仅**依赖 1 个 crate（`gitgit-observability`）
- **缺点**:
  - Loki 全文检索能力弱于 ELK
  - Tempo 起步晚，运维经验少
  - 多组件集成复杂度高
  - 5 个组件需独立运维

### Option 2: Datadog（全 SaaS）

- **优点**: 一站式，开箱即用
- **缺点**:
  - **违反合规**（数据出云）
  - 成本高（10 台业务 + 1.8 TB/月 = 估算 $5k+/月）
  - 业务代码需要 Datadog SDK
  - 不可定制
  
❌ **拒绝**（合规 + 成本）

### Option 3: ELK（Elasticsearch + Logstash + Kibana）

- **优点**: 日志全文检索强
- **缺点**:
  - 资源消耗大（ES 至少 3 副本 × 8GB）
  - Metric / Trace 需另选型（ES APM 弱）
  - 业务代码需要 ES SDK
  - 运维复杂（ES 调参）

❌ **拒绝**（资源 + 多组件）

### Option 4: 自研（基于 PG 触发器 + tail）

- **优点**: 完全可控
- **缺点**:
  - **违反 DRY**（监控数据已在 OTel 链路重复）
  - 重新发明轮子
  - 不可观测性无法被自身监控

❌ **拒绝**（违反 OCP + 重复建设）

### Option 5: VictoriaMetrics + Vector + Jaeger + Grafana

- **优点**:
  - VM 比 Prom 资源更省
  - Vector 日志聚合快
  - Jaeger trace 成熟
- **缺点**:
  - 生态不如 Prom / Loki / Tempo 主流
  - Vector 与 Grafana 集成弱
  - 业务代码需多 SDK 适配

❌ **拒绝**（生态分散）

## Decision

**采用 Option 1（OGSTG）**，并强制以下约束:

1. **业务代码 0 依赖后端 SDK**: 业务 crate **不直接** import `prometheus` / `loki` / `tempo`，仅依赖 `gitgit-observability`
2. **唯一 OTel 入口**: `crates/gitgit-observability` 是**唯一** import `opentelemetry` 的 crate
3. **OTLP 协议**: 业务 → OTel Collector → Backend，统一 OTLP
4. **trace_id 跨进程传播**: W3C Trace Context（HTTP `traceparent` header / gRPC metadata）
5. **资源隔离**: 监控组件部署在独立 `observability` namespace + taint 隔离节点
6. **数据本地化**: 全部 S3 由内部 MinIO 提供，不用公网
7. **GitOps**: 100% IaC（ArgoCD + Kustomize）
8. **降级开关**: 全局 `OTEL_DISABLED=true` 一键关闭
9. **8 阶段分步实施**: 每阶段可独立回滚
10. **meta-observability**: OTel Collector / Prom / Loki / Tempo **自身**被监控

## Implementation Plan

### 4 层架构（详见 [`../../observability/01-architecture.md`](../../observability/01-architecture.md)）

```
┌──────────────────────────────────────────┐
│  Layer 1: Application (Rust 业务)         │
│   └─ gitgit-observability (唯一 OTel)    │
└──────────────┬───────────────────────────┘
               │ OTLP gRPC :4317
               ▼
┌──────────────────────────────────────────┐
│  Layer 2: 采集层                          │
│   └─ OTel Collector (DaemonSet + Gateway) │
│       - batch / memory_limiter            │
│       - tail_sampling                     │
│       - transform (脱敏)                  │
│       - filter (Cardinality)              │
└──────────────┬───────────────────────────┘
               │ remote_write / OTLP / loki
               ▼
┌──────────────────────────────────────────┐
│  Layer 3: 后端                            │
│   ├─ Prometheus (TSDB + Alertmanager)     │
│   ├─ Loki (logs)                          │
│   └─ Tempo (traces)                       │
└──────────────┬───────────────────────────┘
               │ PromQL / LogQL / TraceQL
               ▼
┌──────────────────────────────────────────┐
│  Layer 4: 接入层                          │
│   └─ Grafana (9 大盘 + OIDC + Alert)     │
└──────────────────────────────────────────┘
```

### 命名规约

- Metric: `<namespace>_<subject>_<verb>[_qualifier]_{unit}`
- 例: `gitgit_node_created_total` / `gitgit_event_publish_duration_seconds`

### Cardinality 强约束

- 业务 label 白名单（详见 [`../../observability/02-metrics.md`](../../observability/02-metrics.md) §5）
- `fingerprint` 强制归一化（lint 规则）
- `repo` 在 Prom 端聚合
- 总 series < 1M

### 8 阶段实施

1. Phase 0: 现状 + ADR + 文档（**已完成**）
2. Phase 1: 基础设施监控（kube-prometheus-stack）
3. Phase 2: 应用 Metrics（OTel SDK）
4. Phase 3: 日志集中化（Loki）
5. Phase 4: Distributed Trace（OTel + Tempo）
6. Phase 5: Dashboard 9 大盘
7. Phase 6: Alert 多窗口 + SLO Burn Rate
8. Phase 7: SLO 体系
9. Phase 8: 自动化运维

### 安全设计

- 监控组件独立 namespace + PodSecurity restricted
- NetworkPolicy default-deny + 白名单
- mTLS 通信
- 3 层脱敏（静态 / 字段名 / 值正则）
- OIDC 集成（复用 gitgit-admin 鉴权域）
- 镜像签名（cosign）+ SBOM
- 不暴露公网

### 性能与生命周期

- 业务侧开销 < 3% CPU / < 50 MB 内存
- 自动降级（OTEL_DISABLED / 采样率调整 / queue drop）
- Metrics 30d 原始 + 7d 5m 聚合 + 1y 1h 聚合
- Logs 分级：ERROR 1y / WARN 90d / INFO 30d / DEBUG 1d / audit 3y
- Traces 30d（5% 采样 + 关键路径 100%）

## Compliance with Existing ADRs

| ADR | 关系 | 影响 |
|---|---|---|
| ADR-0002（Git 写用 shell） | 兼容 | OTel 测 `git` 子进程延迟，**不**替代 `git` |
| ADR-0003（能 PG 解决都用 PG） | 兼容 | 监控数据走 Prom/Loki/Tempo，**不**用 PG 存 metric |
| ADR-0006（K3s 部署） | 强化 | 监控利用 K8s API（kube-state-metrics）+ Cilium |
| ADR-0007（零信任） | 强化 | 监控系统独立域 + OIDC + mTLS |
| ADR-0008（哈希链审计） | 强化 | `gitgit_audit_chain_broken` Critical 告警 |
| ADR-0009（多租户 RLS） | 兼容 | label `tenant` 受限 ≤ 100 |
| ADR-0010（Wasm App 沙箱） | 兼容 | App Bus 埋点不侵入 Wasm 沙箱 |

## Consequences

### Positive

- ✅ 业务代码**完全解耦**于监控后端（OpenTelemetry 是标准接口）
- ✅ 未来可平滑迁移到任何 OTel 兼容后端（如 Honeycomb / Lightstep / Datadog）
- ✅ 4 层架构清晰，每层可独立替换 / 升级
- ✅ GitOps + ArgoCD 提供完整审计 + 一键回滚
- ✅ 8 阶段分步实施降低单次变更风险
- ✅ 复用现有 14 个 crate 工程结构，引入 1 个新 crate（`gitgit-observability`）
- ✅ 自监控保证监控系统**不成为单点**
- ✅ SRE 可独立运维 5 个组件（社区成熟）

### Negative

- ⚠ 5 个组件需独立运维（Prom / Loki / Tempo / OTel Collector / Grafana）
- ⚠ OTel SDK 升级可能 breaking（季度评估 + 锁 minor）
- ⚠ Cardinality 失控风险（白名单 + 告警 + Lint 缓解）
- ⚠ 业务代码每个 crate 需 + 100~200 行改造（详见 [`../../observability/14-code-impact.md`](../../observability/14-code-impact.md)）
- ⚠ 监控数据增长需容量规划（5m 聚合 + 降采样 + lifecycle）

### Neutral

- 多 1 个 SaaS 监控的"选型"机会（已被 ADR 锁定）
- 团队需 OTel / PromQL / LogQL 培训（Phase 0 已文档化）

## Alternatives Considered in Detail

### 业务侧 OTel 依赖

| 方案 | 业务依赖 | 评估 |
|---|---|---|
| A. 业务直接依赖 opentelemetry | 14 crate 全引 | ❌ 侵入 |
| **B. gitgit-observability 封装** | **1 crate 间接** | **✅ 选中** |
| C. dyn SDK 模式（trait 抽象） | 仅依赖 trait | ⚠ 过设计，性能损耗 |

✅ 选 B：单一封装 crate，类型安全 + 0 性能损耗 + 易替换。

### OTel Collector 部署

| 方案 | 优势 | 劣势 |
|---|---|---|
| **A. DaemonSet + Gateway** | **每节点 1 副本 + 2 Gateway** | **多组件** |
| B. 仅 DaemonSet | 简单 | 单点 |
| C. 仅 Gateway Deployment | 简单 | 跨节点流量 |

✅ 选 A：DaemonSet 接收 + Gateway 聚合 + 导出，最稳。

### 后端存储

| 方案 | 评估 |
|---|---|
| **A. Prometheus + Loki + Tempo** | **✅ 选中**（开源 + 主流 + 集成） |
| B. Thanos（Prom HA） | v1 不实施，Prom 2 副本足够 |
| C. Cortex / Mimir | v2 评估 |
| D. 公网 S3 | ❌ 违反合规 |

## Verification

### 文档一致性

```bash
# 期望: 0 broken links, 0 日文（除 workflow.md §4）
powershell -File scripts/check-anchors.ps1
```

### 实施验证（每 Phase）

- [ ] Phase 1: kube-prometheus-stack 健康
- [ ] Phase 2: 业务 RED 指标有数据
- [ ] Phase 3: 业务日志可查询
- [ ] Phase 4: 端到端 trace 完整
- [ ] Phase 5: 9 大盘全部 panel 渲染
- [ ] Phase 6: critical 告警 5min 内响应
- [ ] Phase 7: 10 个 SLO 可视化
- [ ] Phase 8: 自动响应流程验证

### 性能验证

- [ ] 业务侧 P99 延迟增量 < 5ms
- [ ] 业务侧 CPU 增量 < 3%
- [ ] 业务侧内存增量 < 50MB
- [ ] OTel Collector drop 率 < 0.1%
- [ ] Prom / Loki / Tempo query P95 < 5s

## References

- 设计文档: [`../../observability/00-current-state-analysis.md`](../../observability/00-current-state-analysis.md) 至 [`15-self-review-v2.md`](../../observability/15-self-review-v2.md)
- 现有 ADR: [README.md](README.md)（0001-0010）
- Google SRE Workbook: SLO / Error Budget / Multi-Window Burn Rate
- OpenTelemetry: <https://opentelemetry.io/docs/specs/otel/>
- CNCF TAG Observability: <https://github.com/cncf/tag-observability>

## Decision Outcome

> **Status**: Accepted (2026-08-20)
>
> 本 ADR 经 SA / SRE Lead / TL 评审通过，进入 Phase 1 实施。

## 变更记录

| 日期 | 变更 | 作者 |
|---|---|---|
| 2026-08-20 | 初稿 + Accepted | SRE Lead |
