# ADR-0016: OpenTelemetry Collector 部署模式

> **Status**: Proposed
> **Date**: 2026-08-23
> **Deciders**: SA + TL + EM
> **Consulted**: SRE + SEC
> **Informed**: 全部

## Context and Problem Statement

[详细设计 §10.5 可观测性](../../design/detailed-design/10-observability.md) 需要 OTel Collector 收集 traces / metrics / logs。

候选部署模式:

- **进程内嵌**(Sidecar 进程同 PID 命名空间)— 零额外运维,适合 MVP
- **同节点 DaemonSet / systemd unit** — 资源隔离,适合多平台进程
- **集中 Gateway 集群** — 适合大规模 V1+ Cloud
- **混合** — 不同信号(traces/metrics/logs)用不同模式

## Decision Drivers

- [DRIVER-1] MVP 是 Local-First 单进程,Collector 也应该单实例
- [DRIVER-2] V1+ K8s 时与 K8s 生态对齐
- [DRIVER-3] 资源开销低(MVP 单 VM 部署,内存预算 ~256MB)
- [DRIVER-4] 故障隔离:Collector 挂掉不影响平台主进程
- [DRIVER-5] 与 [ADR-0015 SIEM 适配器](#) 协调

## Considered Options

1. **Option A — MVP 进程内嵌;V1+ DaemonSet** (推荐)
2. **Option B — MVP 同节点 sidecar;V1+ Gateway 集群**
3. **Option C — 全程进程内嵌**

## Decision Outcome

**Chosen option**: "Option A — MVP 进程内嵌;V1+ DaemonSet", because 与平台"单进程" 原则一致;Collector 挂掉不会影响平台主进程(独立 task);V1+ K8s 化时零迁移成本。

### 各阶段拓扑

| 阶段 | Collector 模式 | 部署方式 | 备注 |
|---|---|---|---|
| MVP (Local) | **进程内嵌** | 平台二进制 fork 出独立 tokio task | 通过 `opentelemetry-otlp` exporter 直发到本地文件 + 本地端口 |
| V0.5 (单 VM staging) | **独立 sidecar 进程** | systemd unit `platform-otel-collector.service` | 与平台同生命周期 |
| V1+ K8s | **DaemonSet** | K8s `DaemonSet` manifest | 每节点一个 Collector,自动伸缩 |
| V1+ 大规模 Cloud | **Gateway 集群** | K8s `Deployment` 多副本 + HPA | 仅当单节点 Collector 容量成为瓶颈时 |

### MVP 阶段进程内嵌具体实现

```rust
// [IMPL] crates/platform-observability/src/collector.rs (MVP)
//
// 启动顺序:
//   1) tracing_subscriber 初始化 (本地 stdout / 文件)
//   2) metrics_exporter_prometheus 启动 (端口 9090)
//   3) otlp::SpanExporter::builder().with_tonic().build() → 本地文件
//   4) 自定义 EventAuditExporter → admin_audit 表
//
// 故障处理:
//   - Collector 启动失败 → 平台主进程不退出,仅 log warn
//   - 后端不可达 → 采样降级,buffer 暂存,启动 30s backoff retry
```

### Consequences

**Good:**
- [+] MVP 零额外进程,运维负担最小
- [+] V1+ 切 DaemonSet 时仅改 deployment 拓扑,代码不变
- [+] 故障隔离符合预期

**Bad:**
- [-] 进程内嵌时若 Collector 阻塞,可能影响主进程(需要 backpressure 限速)
- [-] V1+ Gateway 阶段需要重新设计 exporter 拓扑

### Confirmation

[CONFIRM] 详设 §10 + [specs/test-specification.md §10](../../specs/test-specification.md#10-可观测性测试):
- TC-OBS-010: MVP 启动 5s 内 Prometheus 端口暴露且有基础指标
- TC-OBS-011: kill -9 OTel Collector task 后,平台主进程继续运行,30s 后 collector 自动重启
- TC-OBS-012: 单次 batch 10000 spans 不会阻塞主请求路径 P99 (overhead < 50ms)
- TC-OBS-013: V1+ DaemonSet 部署后,所有平台 pod 的 trace 自动汇聚到同节点 collector

## Pros and Cons of the Options

### Option A — MVP 进程内嵌;V1+ DaemonSet
[+] 与单进程原则一致;故障隔离;V1+ 零迁移
[-] MVP 需要 backpressure 限速,避免阻塞

### Option B — 全程 sidecar / Gateway
[+] 资源隔离清晰
[-] MVP 阶段就引入额外进程,违反"单进程"原则

### Option C — 全程进程内嵌
[+] 最简
[-] V1+ K8s 时无法水平扩展,违背 Cloud 部署预期

## References

- [ADR-0015: SIEM 适配器](#)
- [ADR-0006: OCI 容器 + K8s 部署 V1+ Cloud](../../architecture/decisions/0006-采用-OCI-容器Docker-containerd-+-K8s-部署-V1+-Cloud.md)
- [详细设计 §10 可观测性](../../design/detailed-design/10-observability.md)
- [OpenTelemetry Collector — Deployment Modes](https://opentelemetry.io/docs/collector/deployment/)

## Revision History

| Date | Author | Change |
|---|---|---|
| 2026-08-23 | Mavis (AI 起草) | Initial draft (Proposed) |
| YYYY-MM-DD | — | Status → Accepted (待 SA + TL + EM 签核) |
