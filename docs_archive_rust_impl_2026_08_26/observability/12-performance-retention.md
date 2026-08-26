# 性能影响评估 + 数据生命周期 / Performance Impact & Retention

> **关联 OBS-REQ**: OBS-REQ-020（性能影响） / OBS-REQ-021（数据生命周期） / OBS-REQ-022（降级策略）
> **关联设计**: [`02-metrics.md`](02-metrics.md) §6 性能开销 / [`03-logs.md`](03-logs.md) §5 保留 / [`04-tracing.md`](04-tracing.md) §4 采样
> **核心原则**: 业务优先；可观测性**必须** < 业务开销；资源不足时**降低 telemetry** 而非影响核心服务

## 1. 业务侧开销（Agent 端）

### 1.1 OTel SDK Rust 性能开销

| 维度 | 不开启 | 默认（OTLP + 1s batch） | 性能优化（5s batch + 5% 采样） | 极限优化（30s batch + 1% 采样） |
|---|---|---|---|---|
| **CPU 增量** | 0% | < 3% | < 1% | < 0.3% |
| **内存增量** | 0 MB | < 50 MB | < 30 MB | < 20 MB |
| **网络增量** | 0 | < 1 KB/s/QPS | < 0.5 KB/s/QPS | < 0.1 KB/s/QPS |
| **P99 延迟** | 基准 | + 0.5ms | + 0.2ms | + 0.05ms |
| **错误率** | 基准 | + 0% | + 0% | + 0% |

> **结论**: 默认配置下业务侧**可忽略**。极限优化模式用于资源紧张时。

### 1.2 各项埋点开销细项

#### Metrics 埋点

```rust
// Counter 增量
let counter = meter()
    .u64_counter("http_requests_total")
    .build();
counter.add(1, &[KeyValue::new("status", "200")]);
// 单次 add: < 100ns

// Histogram 增量
let histogram = meter()
    .f64_histogram("http_request_duration_seconds")
    .build();
histogram.record(0.123, &[KeyValue::new("status", "200")]);
// 单次 record: < 200ns
```

| 操作 | CPU | 内存 | 备注 |
|---|---|---|---|
| Counter add | < 100ns | 0 | 无锁 |
| Histogram record | < 200ns | bucket 数组 1KB | 预分配 |
| Gauge set | < 50ns | 0 | 无锁 |
| UpDownCounter add | < 100ns | 0 | — |

#### Span 埋点

```rust
#[tracing::instrument]
async fn handle_request() {
    // Span 创建: < 1us
    // 属性添加: < 200ns / attribute
    // 结束: < 1us
    // 序列化到 OTLP: 异步（不阻塞业务）
}
```

| 操作 | CPU | 内存 | 备注 |
|---|---|---||
| Span 创建 | < 1us | 1KB | 短 span 立即释放 |
| Span 嵌套 | < 200ns / 层级 | 200B / 层级 | 5 层内 < 1us |
| Batch 导出 | 5s 后台任务 | 2KB / span | 异步，不阻塞 |

#### Log 埋点

```rust
info!(
    user_id = %user.id,
    action = "login",
    result = "success",
    "user action"
);
// 单条 info!: < 1us
// 序列化 JSON: < 5us
// 写入 batch buffer: < 500ns
```

| 等级 | CPU | 备注 |
|---|---|---|
| trace / debug | < 2us | 通常被关闭 |
| info | < 1us | 默认开启 |
| warn | < 1us | 默认开启 |
| error | < 1us | 默认开启 |

### 1.3 OTel Collector Batch / Buffer 行为

```yaml
# batch processor
batch:
  timeout: 5s       # 默认 5s 强制 flush
  send_batch_size: 8192  # 满 8192 条立即 flush
  send_batch_max_size: 10000

# memory_limiter
memory_limiter:
  check_interval: 1s
  limit_mib: 2048   # 2GB 上限
  spike_limit_mib: 512

# queue
sending_queue:
  enabled: true
  num_consumers: 10
  queue_size: 5000
  # 队列满 → 丢弃最旧（**不会阻塞业务**）
```

### 1.4 降级策略

> **强约束**: 当资源不足时，**必须**降级 telemetry，绝不阻塞业务。

| 触发条件 | 自动行为 |
|---|---|
| OTel Collector 内存 > 90% | 降低 batch size |
| 业务 Pod 内存 > 80% | 关闭 trace，保留 metric + log |
| 业务 Pod CPU > 80% | 采样率降至 1% |
| 网络出口超限 | 切换到压缩 + 批大小提升 |
| 后端故障 | 内存 buffer 至 5min 数据后丢弃（不写盘） |

```yaml
# 自动降级
processors:
  memory_limiter:
    check_interval: 1s
    limit_mib: 2048
    spike_limit_mib: 512
    # 达到 limit 时，**自动 drop** 不阻塞
  probabilistic_sampler:
    sampling_percentage: 100  # 默认 100%
    # 运行时可通过 control plane 调整
```

## 2. 采集侧开销（Collector 端）

### 2.1 OTel Collector 资源

| 部署形态 | 副本 | CPU | 内存 | 网络（每副本） |
|---|---|---|---|---|
| DaemonSet | 每节点 1 | 200m / 1 核 | 512Mi / 2Gi | 1 MB/s |
| Gateway | 2 | 500m / 2 核 | 1Gi / 4Gi | 5 MB/s |

> **强约束**: DaemonSet 必须 `requests.cpu` ≥ 200m，否则驱逐其他 Pod。

### 2.2 Processor 开销

| Processor | CPU | 内存 | 备注 |
|---|---|---|---|
| batch | 低 | 中 | 5s 缓冲 |
| memory_limiter | 极低 | 监控 | 1s 间隔 |
| tail_sampling | 中 | 中 | 决策需要等待 |
| attributes/limit | 中 | 低 | label 白名单 |
| transform | 高 | 中 | 复杂规则 |
| filter | 中 | 低 | 简单规则 |
| resource | 极低 | 低 | — |

### 2.3 Exporter 开销

| Exporter | CPU | 网络 | 备注 |
|---|---|---|---|
| prometheusremotewrite | 中 | 100 KB/s | 压缩 |
| otlphttp (Tempo) | 中 | 50 KB/s | 压缩 |
| loki | 中 | 200 KB/s | 压缩 |
| webhook (alert) | 极低 | < 1 KB/s | 偶发 |

## 3. 后端开销（Prometheus / Loki / Tempo）

### 3.1 Prometheus

| 项 | 值 | 备注 |
|---|---|---|
| CPU | 1 核 / 100k 活跃 series | TSDB 写入 + 查询 |
| 内存 | 3 GB / 100k series | 索引 + 头块 |
| 磁盘 | 1.5 KB / sample | 原始 + 压缩 |
| 写入吞吐 | 100k samples/s | 默认 |

> **经验值**: 100 万 series → 4 核 / 16GB / 200GB SSD / 30d 保留

### 3.2 Loki

| 项 | 值 | 备注 |
|---|---|---|
| ingester CPU | 0.5 核 / 100 MB/s | 流式写入 |
| ingester 内存 | 1 GB / 100 MB/s | chunk 缓冲 |
| querier CPU | 0.5 核 / query | 查询相关 |
| 存储 | 1.3x 压缩后 / 原始 | 取决于压缩 |
| 网络 | 100 MB/s 入口 | 短时峰值 |

> **经验值**: 100 GB/天 日志 → 3 副本（4 核 / 8GB / 200GB SSD）+ 50GB/天 存储

### 3.3 Tempo

| 项 | 值 | 备注 |
|---|---|---|
| distributor CPU | 0.5 核 | 流式写入 |
| ingester CPU | 0.5 核 / 1000 span/s | flush block |
| querier CPU | 0.5 核 / query | 查询 |
| 存储 | 原始 span 大小 × 1.2 | 压缩后 |
| 索引 | 10% 原始 | TraceID 索引 |

> **经验值**: 1M span/天 → 2 副本（4 核 / 8GB）+ 200GB/月 存储

## 4. 数据量预估

### 4.1 业务基线假设

> **基线值**（[TBD] – Benchmark Required）:
- 业务 RPS: 1000 QPS
- 平均 Span 数 / 请求: 20
- 平均 Log 数 / 请求: 5 (info + warn/error)
- 平均 Metric 数 / 请求: 30 (histogram bucket)

### 4.2 日数据量预估

| 类型 | 速率 | 单条大小 | 日数据量 |
|---|---|---|---|
| **Metrics** | 1000 QPS × 30 = 30k samples/s | ~ 100B | ~ 250 GB / day (raw) |
| **Metrics 压缩后** | — | — | ~ 30 GB / day |
| **Logs** | 1000 QPS × 5 = 5k entries/s | ~ 500B | ~ 200 GB / day (raw) |
| **Logs 压缩后** | — | — | ~ 20 GB / day |
| **Traces** | 1000 QPS × 20 = 20k spans/s | ~ 1KB | ~ 1.5 TB / day (raw) |
| **Traces 采样 5%** | 1000 spans/s | ~ 1KB | ~ 80 GB / day |
| **Traces 压缩后** | — | — | ~ 8 GB / day |

> **总计**: ~ 60 GB/天 压缩后，~ 1.8 TB/月

### 4.3 月存储预估

| 数据 | 原始 | 压缩后 | 保留 |
|---|---|---|---|
| Metrics | 7.5 TB | 900 GB | 30d |
| Logs | 6 TB | 600 GB | 30d (分级) |
| Traces | 2.4 TB | 240 GB | 30d |
| Grafana DB | — | 1 GB | 永久 |
| Alertmanager | — | 5 GB | 7d |
| **总计** | **~ 16 TB** | **~ 1.75 TB** | — |

> **存储成本**: 假设 1TB SSD 0.5元/GB/月 = 875 元/月（内部 MinIO 估算）

## 5. 保留策略（Retention）

### 5.1 Metrics 保留

| 数据 | 保留 | 降采样 | 存储 |
|---|---|---|---|
| **Raw samples** | 30 天 | — | 100% |
| **5m 聚合** | 180 天 | `avg_over_time` | 10% |
| **1h 聚合** | 1 年 | `avg_over_time` | 2% |
| **SLO recording** | 1 年 | — | 单独命名空间 |

> **降采样规则** (`promtool tsdb create-blocks-from`):

```bash
# 每日 04:00 执行
promtool tsdb create-blocks-from \
  --start 2026-08-20T00:00:00Z \
  --end 2026-08-21T00:00:00Z \
  --max-block-chunk-segment-size 0 \
  --out /data/5m-aggregated \
  /etc/prometheus/prometheus.yml \
  <<EOF
matchers:
- '{__name__=~".+"}'
EOF
```

### 5.2 Logs 保留

| Level | 保留 | 备注 |
|---|---|---|
| **ERROR** | 1 年 | 含上下文 |
| **WARN** | 90 天 | — |
| **INFO** | 30 天 | 业务事件 |
| **DEBUG** | 1 天 | 默认关闭 |
| **TRACE** | 不保留 | 业务禁用 |
| **审计 (audit.*)** | 3 年 | 合规 |

> **Loki retention policy** (`limits_config`):

```yaml
limits_config:
  retention_period: 720h   # 30d 默认
  # per-stream retention
  retention_stream:
    - selector: '{level="error"}'
      period: 8760h   # 1y
    - selector: '{level="warn"}'
      period: 2160h   # 90d
    - selector: '{audit="true"}'
      period: 26280h  # 3y
```

### 5.3 Traces 保留

| 类型 | 保留 | 备注 |
|---|---|---|
| 完整 trace（错误/慢/Auth/Admin/AI） | 30 天 | tail_sampling 全采样 |
| 5% 采样（其他） | 30 天 | 随机采样 |
| SLO 关键 trace | 90 天 | 标记 `slo_critical=true` |

> **Tempo retention** (`storage.trace`):

```yaml
storage:
  trace:
    backend: s3
    s3:
      bucket: tempo
    wal:
      path: /var/tempo/wal
    pool:
      max_workers: 100
  # retention 在 compactor 中配置
compactor:
  compaction:
    block_retention: 720h   # 30d
```

## 6. 数据生命周期管理

### 6.1 阶段

```
Active (0-7d)         → 完整数据，高频访问
   │
Warm (7-30d)          → 完整数据，低频访问
   │
Cold (30-90d)         → 聚合数据
   │
Archive (90d-1y)      → 降采样数据
   │
Delete (> 1y)         → 自动删除（合规保留除外）
```

### 6.2 跨 tier 迁移

| 阶段 | 存储类型 | 介质 | 性能 |
|---|---|---|---|
| Active | SSD | NVMe | 高 IOPS |
| Warm | SSD | SATA SSD | 中 IOPS |
| Cold | HDD | 7.2K RPM | 低 IOPS |
| Archive | 对象存储 | MinIO + 备份带 | 极低 |

> **简化实现**: 项目 v1 暂不实现自动跨 tier 迁移，全部在 MinIO 单一存储层。**未来扩展点**。

## 7. 容量规划

### 7.1 监控资源容量

| 项 | 当前预估 | 6 月 | 1 年 | 2 年 |
|---|---|---|---|---|
| Metrics rate | 30k samples/s | 100k | 300k | 1M |
| Log rate | 5k/s | 20k | 50k | 200k |
| Trace rate | 20k span/s | 100k | 300k | 1M |
| 存储 | 1.8 TB/月 | 6 TB | 18 TB | 60 TB |
| 监控 CPU | 5 核 | 15 核 | 40 核 | 120 核 |
| 监控内存 | 16 GB | 48 GB | 128 GB | 384 GB |

### 7.2 监控组件扩容策略

| 信号 | 动作 |
|---|---|
| Prometheus 内存 > 70% | 副本数 +1，或 shard |
| Loki chunk 写入延迟 > 1s | ingester 副本 +1 |
| Tempo query 延迟 > 5s | querier 副本 +1 |
| OTel Collector drop 率 > 1% | 升级资源 / 分流 |

### 7.3 Prometheus 分片（未来）

> **v1 不实施**。当单 Prometheus 达到性能瓶颈时实施。

```yaml
# prometheus-shard-1.yml: 抓取应用层
# prometheus-shard-2.yml: 抓取基础设施层
# 共享 query layer（Thanos Query 或 Prometheus Federation）
```

## 8. 性能监控（meta-observability）

### 8.1 监控自身

| 指标 | 告警阈值 |
|---|---|
| `prometheus_tsdb_head_series` | > 1M 告警 |
| `prometheus_rule_evaluation_duration_seconds` | p99 > 5s 告警 |
| `loki_ingester_chunks_flushed_total` | 趋势 |
| `loki_ingester_streams_total` | > 100k 告警 |
| `tempo_ingester_spans_received_total` | 趋势 |
| `otelcol_exporter_queue_size` | > 5000 告警 |
| `otelcol_processor_dropped_spans` | > 0 告警 |
| `otelcol_processor_dropped_metric_points` | > 0 告警 |
| `otelcol_processor_dropped_log_records` | > 0 告警 |

### 8.2 自动降级编排

```yaml
# 伪代码：业务降级编排
class TelemetryBudget:
    def on_resource_warning(self, metric: str, value: float):
        if metric == "pod_memory" and value > 0.8:
            self.disable("traces")
        elif metric == "pod_cpu" and value > 0.8:
            self.set_sampling_rate(0.01)
        elif metric == "otel_queue_full":
            self.set_sampling_rate(self.sampling_rate * 0.5)
```

## 9. 业务侧接入成本

### 9.1 改造量

| 改造点 | 行数（每 crate） | 影响 |
|---|---|---|
| 引入 `opentelemetry` + `opentelemetry-otlp` | + 30 | Cargo.toml |
| 初始化 tracer / meter / logger | + 50 | main.rs |
| HTTP middleware 集成 | + 20 | tower-http |
| 业务关键路径埋点 | + 5-10 / 端点 | 业务代码 |
| sqlx 集成 | + 5 | 自动 |
| tracing 集成 | + 0 | 已有 |

> **每 crate 总改造**: < 200 行

### 9.2 性能 vs 可观测性 tradeoff

| 选项 | 性能开销 | 可观测性 |
|---|---|---|
| 完全关闭 | 0% | ❌ 无 |
| 仅 metric | < 0.5% | ⚠ 基础 |
| metric + log | < 1% | ✅ 推荐 |
| metric + log + trace | < 3% | ✅ 完整 |
| 包含 profiling | 5-10% | ✅✅ 极致 |

> **默认**: metric + log + trace（3%）+ 关键端点 100% 采样

## 10. 故障演练（验证开销）

### 10.1 演练场景

| 场景 | 操作 | 验证 |
|---|---|---|
| 后端故障 | 关停 Prometheus | 业务不受影响（OTel 内存 buffer 5min） |
| 出口网络故障 | 阻断 OTLP 出口 | 业务延迟 +0.5ms（buffer） |
| 业务高负载 | 压测 10x | OTel 自动降级到 1% 采样 |
| OTel 内存爆 | 触发 memory_limiter | 业务无感，drop old data |

### 10.2 验证脚本

```bash
#!/bin/bash
# scripts/chaos-test-otel.sh
# 1. 关闭 Prometheus
kubectl scale --replicas=0 -n observability statefulset/prometheus
# 2. 业务持续 5min 压测
hey -n 30000 -c 100 http://gitgit-server.gitgit:3000/api/health
# 3. 检查业务延迟
hey ...  # 业务延迟 P99 < 1s 为通过
# 4. 恢复 Prometheus
kubectl scale --replicas=2 -n observability statefulset/prometheus
```

## 11. 关联文档

- 上游: [`02-metrics.md`](02-metrics.md) §6
- 上游: [`03-logs.md`](03-logs.md) §5
- 上游: [`04-tracing.md`](04-tracing.md) §4
- 下游: [`15-self-review-v2.md`](15-self-review-v2.md) §6 性能复盘

## 12. 需求 ID 索引

| 需求 ID | 标题 | 优先级 |
|---|---|---|
| OBS-REQ-020 | 性能影响评估 | P0 |
| OBS-REQ-021 | 数据生命周期 | P0 |
| OBS-REQ-022 | 降级策略 | P0 |
| PERF-REQ-001 | 业务侧开销 < 3% | P0 |
| PERF-REQ-002 | 降级编排 | P0 |
| PERF-REQ-003 | 容量规划 | P0 |
| PERF-REQ-004 | meta-observability | P1 |
| PERF-REQ-005 | 故障演练 | P1 |
| RET-REQ-001 | Metrics 30d 原始 + 降采样 | P0 |
| RET-REQ-002 | Logs 分级 1d-3y | P0 |
| RET-REQ-003 | Traces 30d 采样 | P0 |
| RET-REQ-004 | 跨 tier 迁移（未来） | P2 |
| RET-REQ-005 | 容量扩容触发 | P0 |
