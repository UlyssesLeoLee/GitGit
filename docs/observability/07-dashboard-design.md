# Grafana Dashboard 设计 / Dashboard Design

> **关联 OBS-REQ**: OBS-REQ-007（Grafana 统一可视化） / OBS-REQ-008（Metric→Trace→Log 关联）
> **关联设计**: [`01-architecture.md`](01-architecture.md) §4 接入层 / [`02-metrics.md`](02-metrics.md) / [`04-tracing.md`](04-tracing.md) / [`05-database-observability.md`](05-database-observability.md) / [`06-middleware-observability.md`](06-middleware-observability.md)
> **目标**: **9 大盘** = 故障定位作战图，每个 panel 必须能回答具体定位问题，**禁止**装饰性图表
> **呈现**: 100% 由 `dashboard provisioning` 文件生成，**禁止** UI 手工编辑后漂移

## 1. 设计原则

### 1.1 每个 Panel 必须回答的 5 个问题（强约束）

> **设计红线**: 任何 panel **必须**能回答以下至少 1 个问题，否则不写。

| 问题 | 缩写 | 含义 |
|---|---|---|
| Q1 | **OK?** | 当前是否正常？（stat / gauge / 阈值色） |
| Q2 | **Where?** | 哪里异常？（by label 分组：namespace / pod / tenant / db） |
| Q3 | **When?** | 什么时间开始？（timeseries + 突增标注 / 异常点高亮） |
| Q4 | **Impact?** | 影响范围？（副本不足 / 慢请求占比 / 用户数） |
| Q5 | **Next?** | 下一步如何定位？（链接到 trace / log / runbook） |

### 1.2 命名规约

| 对象 | 命名 | 示例 |
|---|---|---|
| Dashboard 文件名 | `<num>-<topic>-<audience>.yaml` | `40-database.yaml` |
| Dashboard Title | `GitGit / <topic> / <audience>` | `GitGit / Database / SRE` |
| Panel Title | 中文 ≤ 16 字 | `PG 长事务` |
| Variable | `$env` / `$tenant` / `$namespace` / `$instance` | — |
| Tag | `domain:<x>`, `layer:<y>`, `audience:<z>` | `domain:database,layer:db,audience:sre` |

### 1.3 关键 Audience 划分

| Audience | 关注点 | Dashboard 数量 |
|---|---|---|
| **SRE** | 故障定位 + 容量规划 + SLO 健康 | 9 大盘全集 |
| **Backend Dev** | 业务 SQL / Span / 错误堆栈 | 30-Application / 40-Database（聚焦视图） |
| **Security** | 审计 + 凭证 + 越权 | 80-Security |
| **PM / EM** | 业务健康 + 用户影响 + SLO Burn Rate | 90-SLO |

### 1.4 Panel 类型规约

| 类型 | 用途 | 禁用场景 |
|---|---|---|
| `stat` | 单一当前值（OK/QPS/Error%） | 不带阈值色 |
| `timeseries` | 趋势 / 历史对比 | 永远不用 pie chart |
| `gauge` | 容量水位 | 永远不用 `bargauge` |
| `table` | Top-N / 列表 | 永远不用 text 模拟 |
| `logs` | 实时日志 | 永远不用单独的 log dashboard |
| `traces` | Trace 列表 | 仅在 30-Application 用 |
| `nodeGraph` | 服务依赖 | 仅在 30-Application |

> **禁止**: 3D / 仪表盘动画 / 渐变色 / 装饰性图标。`grafana_dashboard` 全部用 dark theme + 紧凑布局。

## 2. 9 大盘清单

| 编号 | 名称 | Audience | Panel 数（目标） | 主要回答 |
|---|---|---|---|---|
| 00 | System Overview | 所有人 | 8 | 整体健康？SLO？事件？ |
| 10 | Infrastructure | SRE | 12 | 节点资源？网络？磁盘？ |
| 20 | Kubernetes | SRE | 14 | Pod / Deployment / 网络策略？ |
| 30 | Application | Backend Dev / SRE | 16 | RED 指标 / Trace / 错误？ |
| 40 | Database | SRE / DBA | 18 | PG 健康 / SQL 性能 / 业务表？ |
| 50 | Middleware | SRE | 12 | Git / 事件 / App Bus / K3s？ |
| 60 | Network | SRE | 8 | Ingress / DNS / Cilium？ |
| 70 | Performance | SRE | 10 | 慢查询 / 慢 Span / 资源？ |
| 80 | Security | Security | 10 | 审计 / 凭证 / 越权？ |
| 90 | SLO | PM / EM | 8 | SLO 目标 / 错误预算 / Burn Rate？ |

> **总数**: 116 panel，**禁止**超过 150（人眼阅读上限）。

## 3. 00-System Overview

### 3.1 Panel 设计

| Panel | 类型 | 数据源 | 关键 PromQL / LogQL | 回答 |
|---|---|---|---|---|
| 整体健康 | `stat` | Prometheus | `vector(1) and (sum(up{job=~".+"}) == count(up{job=~".+"}))` 变体 | OK? |
| SLO 错误预算 | `gauge` | Prometheus | `1 - slo:budget:burned{...}` | Q1+Q4 |
| 关键服务 SLO 状态 | `stat` × 5 | Prometheus | SLO 服务各自可用性 | OK? |
| 最近 1h 告警 | `logs` | Loki | `{severity="critical"}` | Q3 |
| 事件流（5 原语） | `timeseries` | Prometheus | `rate(gitgit_event_published_total[1m])` | Q2 |
| 用户活跃（5m） | `stat` | Prometheus | `count(count by (user_id) (...) )` | Q4 |
| 当前未结案 incident | `table` | Grafana Incident | (从 IRM 拉) | Q5 |
| 系统日志关键字 | `logs` | Loki | `{service=~"gitgit-.*"} \|~ "ERROR\|FATAL"` | Q3 |

### 3.2 Layout

```
┌──────────────────────────────────────────────────────────┐
│  System Health (stat)       │  SLO Budget Remaining (g) │
├─────────────────────────────┼────────────────────────────┤
│  SLO Targets (stat × 5)                                  │
├──────────────────────────────────────────────────────────┤
│  Recent Critical Alerts (logs)                           │
├──────────────────────────────┬───────────────────────────┤
│  Event Bus Rate (timeseries) │  Active Users (stat)      │
├──────────────────────────────┴───────────────────────────┤
│  Open Incidents (table)                                 │
└──────────────────────────────────────────────────────────┘
```

### 3.3 关键 PromQL

```promql
# 整体健康（0=有组件 down，1=全部 up）
clamp_max(
  sum(up{job=~"gitgit-.*|otel-collector|prometheus|pg-exporter"})
  /
  count(up{job=~"gitgit-.*|otel-collector|prometheus|pg-exporter"}),
  1
)

# SLO 错误预算剩余（30 天滚动）
1 - (
  sum(increase(slo:sli_error:ratio_5m{slo="api-availability"}[30d]))
  /
  sum(increase(slo:sli_error:ratio_5m{slo="api-availability"}[30d]) > 0)
) * (30 * 24 * 60 / 60)  # 归一化到 30d 窗口
```

## 4. 10-Infrastructure

### 4.1 Panel 设计

| Panel | 类型 | PromQL | 回答 |
|---|---|---|---|
| Node CPU 使用率 | `timeseries` | `100 - (avg by (instance) (rate(node_cpu_seconds_total{mode="idle"}[5m])) * 100)` | Q2+Q3 |
| Node 内存使用率 | `timeseries` | `(1 - (node_memory_MemAvailable_bytes / node_memory_MemTotal_bytes)) * 100` | Q2+Q3 |
| Node 磁盘使用率 | `bargauge` | `(1 - (node_filesystem_avail_bytes{fstype!~"tmpfs|overlay"} / node_filesystem_size_bytes)) * 100` | Q4 |
| 磁盘 Inode 使用率 | `timeseries` | `(1 - (node_filesystem_files_free / node_filesystem_files)) * 100` | Q2 |
| Load Average | `timeseries` | `node_load5` | Q2 |
| 网络流量 | `timeseries` × 2 | `rate(node_network_receive_bytes_total[5m])` / `..._transmit_...` | Q4 |
| 上下文切换 / 中断 | `timeseries` | `rate(node_context_switches_total[5m])` | Q2 |
| 文件描述符 | `timeseries` | `node_open_fds / node_filefd_maximum` | Q4 |
| 熵池 | `stat` | `node_entropy_available_bits` < 1000 红 | OK? |
| 时间同步 | `stat` | `node_timex_offset_seconds` > 1s 红 | OK? |
| NTP offset | `timeseries` | `node_timex_offset_seconds` | Q3 |
| 节点列表（含角色） | `table` | kube_node_labels | Q2 |

### 4.2 关键告警阈值

- CPU > 85% 持续 5m 黄色
- CPU > 95% 持续 2m 红色
- 内存 > 90% 持续 5m 红色
- 磁盘 > 85% 黄色 / > 95% 红色

## 5. 20-Kubernetes

### 5.1 Panel 设计

| Panel | 类型 | 关键 PromQL | 回答 |
|---|---|---|---|
| Pod 总数 vs Ready | `timeseries` | `sum(kube_pod_status_phase)` / `...{phase="Running"}` | Q1 |
| CrashLoopBackOff Pods | `table` | `kube_pod_container_status_waiting_reason{reason="CrashLoopBackOff"}` | Q2 |
| Pending Pods | `table` | `kube_pod_status_phase{phase="Pending"}` | Q2 |
| Pod 重启 Top 10 | `bar` | `topk(10, increase(kube_pod_container_status_restarts_total[1h]))` | Q2 |
| Deployment 副本不足 | `table` | `kube_deployment_status_replicas_unavailable` | Q4 |
| StatefulSet 副本不足 | `table` | `kube_statefulset_status_replicas_unavailable` | Q4 |
| HPA 当前 vs 期望 | `timeseries` | `kube_horizontalpodautoscaler_status_current_replicas` | Q2 |
| Job 失败 | `table` | `kube_job_status_failed` | Q2 |
| CronJob 下次执行 | `table` | `kube_cronjob_next_schedule_time` | Q3 |
| 资源使用 vs Limit | `timeseries` | `container_cpu_usage_seconds_total / container_spec_cpu_quota` | Q4 |
| 调度延迟 | `timeseries` | `kube_pod_start_time - kube_pod_created` | Q3 |
| 镜像拉取错误 | `table` | `kube_pod_container_status_waiting_reason{reason="ImagePullBackOff"}` | Q2 |
| K3s API Server 延迟 | `timeseries` | `apiserver_request_duration_seconds` p99 | Q3 |
| etcd WAL fsync | `timeseries` | `etcd_disk_wal_fsync_duration_seconds` p99 | Q3 |

## 6. 30-Application

### 6.1 RED 主面板

| Panel | 类型 | PromQL | 回答 |
|---|---|---|---|
| RPS 总览 | `timeseries` | `sum(rate(http_requests_total[1m]))` by (service) | Q2 |
| 错误率 | `timeseries` | `sum(rate(http_requests_total{status=~"5.."}[5m])) / sum(rate(http_requests_total[5m]))` by (service) | Q1+Q2 |
| P50 / P95 / P99 Latency | `timeseries` × 3 | `histogram_quantile(0.50, sum by (le, service) (rate(http_request_duration_seconds_bucket[5m])))` | Q2 |
| Apdex | `gauge` | (满意 + 容忍/2) / 总数 | OK? |
| 活跃 Span | `stat` | `rate(traces_spanmetrics_calls_total[1m])` | Q4 |
| 错误 Span | `timeseries` | `rate(traces_spanmetrics_calls_total{status_code="STATUS_CODE_ERROR"}[5m])` by (service) | Q2 |
| 慢 Span Top 10 | `bar` | `topk(10, rate(traces_spanmetrics_latency_bucket[5m]))` | Q2 |

### 6.2 服务依赖图

```promql
# 数据源：traces + spanmetrics
# nodeGraph 类型，自动从 trace spanmetrics 推断边
```

| Panel | 类型 | 数据源 | 回答 |
|---|---|---|---|
| Service Map | `nodeGraph` | traces | Q2 |
| 数据库调用热点 | `table` | traces | Q2 |
| 外部 API 热点 | `table` | traces | Q2 |

### 6.3 业务语义（5 原语）

| Panel | 类型 | PromQL | 回答 |
|---|---|---|---|
| Node 创建速率 | `timeseries` | `sum(rate(gitgit_node_created_total[5m]))` by (type, tenant) | Q2 |
| Node 删除速率 | `timeseries` | `sum(rate(gitgit_node_deleted_total[5m]))` by (type, tenant) | Q2 |
| 视图失效速率 | `timeseries` | `sum(rate(gitgit_view_invalidated_total[5m]))` | Q2 |
| Agent Run 状态 | `table` | `sum by (status) (gitgit_agent_run_total)` | Q2 |
| AI 网关调用 | `timeseries` | `sum(rate(gitgit_ai_gateway_requests_total[5m]))` by (provider) | Q2 |
| Webhook 投递 | `timeseries` | `sum(rate(gitgit_webhook_delivery_total[5m]))` by (result) | Q2 |
| App Bus 拒绝 | `timeseries` | `sum(rate(gitgit_app_bus_capability_denied_total[5m]))` by (app) | Q2 |

## 7. 40-Database

### 7.1 PG 健康（USE 方法）

| Panel | 类型 | PromQL | 回答 |
|---|---|---|---|
| 活跃连接 / 限制 | `gauge` | `pg_stat_activity_count{state="active"} / pg_settings_max_connections` | Q4 |
| 等待客户端 | `timeseries` | `pgbouncer_pools_waiting` | Q2 |
| 缓存命中率 | `gauge` | `pg_cache_hit_ratio` | OK? |
| 事务提交 / 回滚 | `timeseries` | `pg_stat_database_xact_commit` / `xact_rollback` | Q2 |
| 死锁 | `stat` | `sum(rate(pg_stat_database_deadlocks[5m]))` | Q1 |
| 锁等待 | `timeseries` | `pg_locks_count{mode="waiting"}` | Q2 |
| 复制延迟 | `stat` | `pg_replication_replay_lag_seconds` | Q1 |
| WAL fsync 延迟 | `timeseries` | `pg_stat_wal_writer_fsync_duration_seconds` p99 | Q3 |

### 7.2 SQL 性能（RED）

| Panel | 类型 | PromQL | 回答 |
|---|---|---|---|
| Top 20 慢查询 | `table` | 见 [`05-database-observability.md`](05-database-observability.md) §5.2 | Q2 |
| Top 20 调用次数 | `table` | 同上，按 `calls` 排序 | Q4 |
| Auto Explain 触发 | `timeseries` | `pg_auto_explain_count` | Q2 |
| 临时文件 | `timeseries` | `pg_stat_database_temp_bytes` | Q2 |
| 表膨胀 Top 10 | `table` | `pg_table_bloat_ratio` top 10 | Q2 |
| 索引膨胀 Top 10 | `table` | `pg_index_bloat_ratio` top 10 | Q2 |
| 检查点频率 | `timeseries` | `pg_stat_bgwriter_checkpoints_req_total` | Q2 |
| Vacuum 滞后 | `table` | `now() - pg_stat_user_tables_last_autovacuum` > 24h | Q2 |

### 7.3 业务表深度

| Panel | 类型 | PromQL / SQL | 回答 |
|---|---|---|---|
| 5 原语 Node 增长 | `timeseries` | `gitgit_node_total{type}` | Q2 |
| 5 原语 Edge 增长 | `timeseries` | `gitgit_edge_total{kind}` | Q2 |
| 中心事件积压 | `timeseries` | `gitgit_event_journal_unpublished_count{topic}` | Q2 |
| 中心事件消费延迟 | `timeseries` | `gitgit_event_journal_lag_seconds` | Q3 |
| 审计链状态 | `stat` | `gitgit_audit_hash_chain_broken` | OK? |
| 凭证 KEK 轮换到期 | `table` | `gitgit_secret_kek_rotation_due < 30d` | Q3 |
| App 注册数 | `timeseries` | `gitgit_app_total{status}` | Q2 |
| App 版本未使用 | `table` | `gitgit_app_version_unused_days > 90` | Q4 |

## 8. 50-Middleware

| Panel | 类型 | 关键 PromQL | 回答 |
|---|---|---|---|
| Git 协议 RPS | `timeseries` | `sum(rate(gitgit_git_protocol_requests_total[5m]))` by (protocol, verb) | Q2 |
| Git Push 延迟 | `timeseries` | `histogram_quantile(0.95, rate(gitgit_git_protocol_request_duration_seconds_bucket{verb="push"}[5m]))` | Q2 |
| Git 写子进程错误 | `timeseries` | `sum(rate(gitgit_git_write_exit_code_total[5m]))` by (exit_code) | Q2 |
| Git 写 index lock 等待 | `timeseries` | `histogram_quantile(0.99, rate(gitgit_git_write_index_lock_wait_seconds_bucket[5m]))` | Q2 |
| Git LFS 上传 / 下载 | `timeseries` × 2 | `rate(gitgit_lfs_upload_bytes_total[5m])` | Q2 |
| Git LFS 孤儿对象 | `stat` | `gitgit_lfs_orphan_objects_total` | Q4 |
| SSH 认证失败 | `timeseries` | `sum(rate(gitgit_ssh_auth_failures_total[5m]))` by (user, reason) | Q2 |
| 中心事件发布延迟 | `timeseries` | `histogram_quantile(0.95, rate(gitgit_event_publish_duration_seconds_bucket[5m]))` | Q2 |
| 中心事件消费延迟 | `timeseries` | `histogram_quantile(0.95, rate(gitgit_event_consume_duration_seconds_bucket[5m]))` | Q2 |
| App Bus 调用延迟 | `timeseries` | `histogram_quantile(0.95, rate(gitgit_app_bus_invoke_duration_seconds_bucket[5m]))` by (call) | Q2 |
| App Bus capability 拒绝 | `timeseries` | `sum(rate(gitgit_app_bus_capability_denied_total[5m]))` by (app, capability) | Q2 |
| OTel Collector 队列 | `timeseries` | `otelcol_exporter_queue_size` | Q4 |

## 9. 60-Network

| Panel | 类型 | 关键 PromQL | 回答 |
|---|---|---|---|
| Ingress RPS | `timeseries` | `sum(rate(nginx_ingress_controller_requests[5m]))` by (ingress) | Q2 |
| Ingress P99 延迟 | `timeseries` | `histogram_quantile(0.99, ...)` | Q2 |
| Ingress 5xx | `timeseries` | `sum(rate(nginx_ingress_controller_requests{status=~"5.."}[5m]))` | Q2 |
| Cilium 策略拒绝 | `timeseries` | `sum(rate(cilium_drop_count_total{reason="PolicyDenied"}[5m]))` | Q2 |
| DNS 解析延迟 | `timeseries` | `histogram_quantile(0.99, rate(coredns_dns_request_duration_seconds_bucket[5m]))` | Q2 |
| 节点网络收发错误 | `timeseries` | `rate(node_network_receive_errs_total[5m])` | Q2 |
| 跨节点 TCP 重传 | `timeseries` | `rate(node_netstat_Tcp_RetransSegs[5m])` | Q2 |
| Service Mesh 流量 | `table` | cilium / hubble | Q2 |

## 10. 70-Performance

| Panel | 类型 | 关键 PromQL | 回答 |
|---|---|---|---|
| Top 20 慢 Span | `bar` | `topk(20, rate(traces_spanmetrics_latency_bucket{...}[5m]))` | Q2 |
| Top 20 慢 SQL | `bar` | (Prometheus → trace → log) | Q2 |
| CPU 火焰图 | (Pyroscope) | 持续 profile | Q2 |
| HTTP 长尾 | `heatmap` | `rate(http_request_duration_seconds_bucket[5m])` | Q2 |
| GC 暂停（Rust 无 GC） | N/A | — | — |
| 内存分配 | `timeseries` | rust 进程 rss / heap | Q2 |
| 上下文切换 / 进程 | `timeseries` | `rate(node_context_switches_total[5m])` / 进程数 | Q2 |
| 网络往返时间 | `timeseries` | `tcp_rtt` 估算 | Q2 |
| 磁盘 IO 延迟 | `timeseries` | `rate(node_disk_io_time_seconds_total[5m])` | Q2 |
| 系统负载预测 | `timeseries` | (来自 forecast) | Q3 |

## 11. 80-Security

| Panel | 类型 | 关键 PromQL | 回答 |
|---|---|---|---|
| 认证失败 Top 10 用户 | `table` | `topk(10, sum by (user, ip) (rate(gitgit_auth_failures_total[5m])))` | Q2 |
| WebAuthn 失败 | `timeseries` | `sum(rate(gitgit_webauthn_failures_total[5m]))` by (reason) | Q2 |
| admin_audit 哈希链 | `stat` | `gitgit_audit_chain_broken` | OK? |
| admin_audit 速率 | `timeseries` | `sum(rate(gitgit_audit_events_total[5m]))` by (action) | Q2 |
| App capability 越权 | `table` | `topk(20, gitgit_app_bus_capability_denied_total)` | Q2 |
| Webhook 出口 | `table` | `gitgit_webhook_external_requests_total` | Q2 |
| Secrets KEK 轮换状态 | `table` | `gitgit_secret_kek_rotation_due` | Q3 |
| WebAuthn 凭证总数 | `stat` | `gitgit_webauthn_credential_total` | 趋势 |
| Token 颁发 / 撤销 | `timeseries` | `rate(gitgit_token_issued_total[5m])` | Q2 |
| 网络拒绝（策略） | `timeseries` | `rate(cilium_drop_count_total{reason="PolicyDenied"}[5m])` | Q2 |

## 12. 90-SLO

| Panel | 类型 | 关键 PromQL | 回答 |
|---|---|---|---|
| 核心服务可用性 SLO | `gauge` × 5 | `1 - slo:sli_error:ratio_5m{slo=~"api-.*"}` | OK? |
| 错误预算剩余（30d） | `bargauge` | (30d burn) | Q4 |
| Burn Rate 1h / 6h | `timeseries` × 2 | `slo:sli_error:ratio_rate_1h{slo=~"api-.*"}` | Q3 |
| SLO 状态总览 | `table` | `slo_info` | OK? |
| 月度预算燃烧 | `timeseries` | `1 - (1 - sum(slo:sli_error:ratio_5m[30d])) / (1 - (1 - 0.999)^(30*24))` | Q4 |
| MTTR 历史 | `table` | incident MTTR | Q3 |
| 错误预算告警（多窗口） | `timeseries` | `(slo:sli_error:ratio_rate_1h > 14.4) and (slo:sli_error:ratio_rate_6h > 6)` | Q3 |
| 用户可见错误 | `timeseries` | `sum(rate(http_requests_total{status=~"5.."}[5m]))` | Q4 |

## 13. Cardinality 控制

| 大盘 | Variable 总数 | 限制 |
|---|---|---|
| 任意大盘 | `$env`, `$tenant`, `$namespace`, `$instance`, `$service`, `$topic` | ≤ 6 |
| 单个 panel label 组合 | — | ≤ 4 |
| Timeseries panel | 渲染线数 | ≤ 50 |
| Table panel | 行数 | ≤ 100 |

**强制**: 每个 panel 必带 `max data points` 与 `interval` 限制，避免采样爆炸。

## 14. Provisioning 规约

### 14.1 目录结构

```
deploy/observability/
├── grafana/
│   ├── provisioning/
│   │   ├── datasources/
│   │   │   ├── prometheus.yaml
│   │   │   ├── loki.yaml
│   │   │   ├── tempo.yaml
│   │   │   └── pyroscope.yaml       # 可选
│   │   └── dashboards/
│   │       ├── default.yaml         # provider
│   │       └── dashboards/          # 实际 JSON
│   │           ├── 00-system.json
│   │           ├── 10-infra.json
│   │           ├── ...
│   │           └── 90-slo.json
```

### 14.2 provider 配置（default.yaml）

```yaml
apiVersion: 1
providers:
  - name: gitgit-dashboards
    orgId: 1
    folder: GitGit
    type: file
    disableDeletion: false
    updateIntervalSeconds: 30
    allowUiUpdates: false        # 禁止 UI 编辑后漂移
    options:
      path: /var/lib/grafana/dashboards
      foldersFromFilesStructure: true
```

> **强约束**: `allowUiUpdates: false`，所有 dashboard 修改必须走 PR。

## 15. 链接与 Drill-Down 规约

| 触发 | 目标 | 实现 |
|---|---|---|
| panel 中 trace_id | 30-Application → 跳到 Tempo | `dataLink` |
| panel 中 user_id | 80-Security 过滤 | `dataLink` |
| panel 中 sql_fingerprint | 40-Database 详情 | `dataLink` |
| 任意 panel "查看 runbook" | `https://runbooks.example.com/...` | `external link` |
| 任意 panel "告警状态" | Alertmanager 详情 | `external link` |
| 任意 panel "Pod 详情" | Kubernetes Dashboard | `external link` |

## 16. 权限隔离

| Dashboard | Audience | 可见角色 |
|---|---|---|
| 00, 10, 20, 50, 60, 70, 90 | SRE | `role:sre` |
| 30, 40 | Backend Dev | `role:backend-dev` |
| 80 | Security | `role:security` |
| 90 | PM / EM | `role:em`, `role:pm` |

> 通过 Grafana Folder Permission + RBAC 实现（详见 [`10-security-design.md`](10-security-design.md)）。

## 17. 关联文档

- 上游: [`02-metrics.md`](02-metrics.md) §4 命名规约
- 上游: [`04-tracing.md`](04-tracing.md) §8 Trace 关联
- 上游: [`05-database-observability.md`](05-database-observability.md) / [`06-middleware-observability.md`](06-middleware-observability.md)
- 下游: [`08-alert-design.md`](08-alert-design.md)（panel → alert 双向链接）
- 下游: [`10-security-design.md`](10-security-design.md) §2 Grafana RBAC
- 下游: [`11-deployment-design.md`](11-deployment-design.md) §4 Grafana 部署

## 18. 需求 ID 索引

| 需求 ID | 标题 | 优先级 |
|---|---|---|
| OBS-REQ-007 | Grafana 统一可视化 | P0 |
| OBS-REQ-008 | Metric→Trace→Log 关联 | P0 |
| DASH-REQ-001 | 9 大盘全集 | P0 |
| DASH-REQ-002 | Panel 5 问 | P0 |
| DASH-REQ-003 | Provisioning 100% 化 | P0 |
| DASH-REQ-004 | Drill-Down 链接 | P0 |
| DASH-REQ-005 | 权限隔离 | P0 |
