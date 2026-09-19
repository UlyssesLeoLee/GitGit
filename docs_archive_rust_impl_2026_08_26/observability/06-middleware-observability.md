# 中间件可观测性 / Middleware Observability

> **关联 OBS-REQ**: OBS-REQ-006（中间件可观测性） / OBS-REQ-013（统一接入）
> **关联设计**: [`../design/detailed-design/06-git-server.md`](../design/detailed-design/06-git-server.md) / [`../design/detailed-design/07-app-platform.md`](../design/detailed-design/07-app-platform.md) / [`../architecture/decisions/0010-app-sandbox-wasm-wasi.md`](../architecture/decisions/0010-app-sandbox-wasm-wasi.md)
> **关联 ADR**: ADR-0003（能 PG 解决都用 PG） / ADR-0006（K3s 部署） / ADR-0010（App 沙箱 Wasm+WASI）
> **目标**: K3s / Git 协议 / 中心事件总线 / App Bus（Wasm hostcall）四大非 PG 组件深度可观测

## 1. 中间件清单

> **强约束（ADR-0003）**: 不引入 Kafka / RabbitMQ / Redis / NATS / Memgraph / Valkey。本项目"中间件"特指 K3s + Git 协议 + 中心事件总线（PG 内置）+ App Bus（Wasm）。

| 组件 | 角色 | 实现 | 关键观测点 |
|---|---|---|---|
| K3s | 容器编排 | k3s 1.30+ | Node / Pod / Deployment / Service / Ingress / NetworkPolicy |
| Git 协议（Smart HTTP / SSH） | 代码托管 | gix 0.66 读 + shell `git` 写 | Push / Pull / Clone / Receive-pack 延迟 |
| 中心事件总线 | 5 原语之间的事件流 | PG `event_journal` + LISTEN/NOTIFY + 消费者 worker | 写入延迟 / 消费延迟 / 积压 / 重放 |
| App Bus | App 与宿主通信 | Wasm hostcall（wasmtime）+ capability check | 调用次数 / 延迟 / capability 拒绝 / 资源限制 |
| pgbouncer | PG 连接池 | pgbouncer 1.22+ | 见 [`05-database-observability.md`](05-database-observability.md) |
| OTel Collector | 遥测汇聚 | otel-collector-contrib 0.110+ | 见 §5 |

## 2. K3s 集群可观测性

### 2.1 部署形态（来自 ADR-0006）

| 节点 | 角色 | 数量 | 资源 |
|---|---|---|---|
| `k3s-server-*` | Control Plane + etcd 嵌入式 | 3 | 4C / 8G / 80G |
| `k3s-agent-*` | 工作节点（业务 Pod） | 3+ | 8C / 16G / 200G |
| `k3s-edge-*` | 边缘节点（Git 写） | 2+ | 4C / 8G / 1T |

### 2.2 核心指标（K8S-REQ-001）

| Metric 名 | 来源 | 阈值 |
|---|---|---|
| `kube_node_status_condition{condition,status}` | kube-state-metrics | Ready≠True 告警 |
| `kube_node_cpu_usage_ratio{node}` | node-exporter cAdvisor | > 0.85 持续 5m 告警 |
| `kube_node_memory_usage_ratio{node}` | node-exporter cAdvisor | > 0.90 持续 5m 告警 |
| `kube_node_disk_usage_ratio{node,path}` | node-exporter | > 0.85 告警 / > 0.95 Critical |
| `kube_node_network_receive_bytes_total{node}` | node-exporter | 趋势 |
| `kube_node_network_transmit_bytes_total{node}` | node-exporter | 趋势 |
| `kube_node_load1{node}` | node-exporter | > 核数 持续 5m 告警 |

### 2.3 Pod / Deployment（K8S-REQ-002）

| Metric 名 | 来源 | 阈值 |
|---|---|---|
| `kube_pod_status_phase{namespace,pod,phase}` | kube-state-metrics | Pending / Failed ≠ 0 告警 |
| `kube_pod_container_status_restarts_total{namespace,pod,container}` | kube-state-metrics | 突增告警 |
| `kube_pod_container_status_waiting_reason{reason}` | kube-state-metrics | CrashLoopBackOff / ImagePullBackOff 告警 |
| `kube_pod_container_resource_limits_cpu_cores{namespace,pod}` | kube-state-metrics | — |
| `kube_pod_container_resource_requests_cpu_cores{namespace,pod}` | kube-state-metrics | 与 limit 差值 |
| `kube_deployment_status_replicas_ready{namespace,deployment}` | kube-state-metrics | < desired 告警 |
| `kube_deployment_spec_replicas{namespace,deployment}` | kube-state-metrics | — |
| `kube_horizontalpodautoscaler_status_current_replicas{hpa}` | kube-state-metrics | < min 告警 |
| `kube_horizontalpodautoscaler_status_desired_replicas{hpa}` | kube-state-metrics | — |

### 2.4 K3s Control Plane 专用（K8S-REQ-003）

| Metric 名 | 来源 | 阈值 |
|---|---|---|
| `k3s_server_api_request_total{verb,resource,code}` | k3s 内置 metrics :6443 | 5xx > 1% 告警 |
| `k3s_server_etcd_disk_wal_fsync_duration_seconds_bucket` | k3s etcd | p99 > 50ms 告警 |
| `k3s_server_etcd_object_count` | k3s etcd | 趋势 |
| `k3s_server_etcd_db_size_bytes` | k3s etcd | > 8GB 告警（默认 quota） |
| `k3s_agent_kubelet_pleg_relist_duration_seconds` | kubelet | p99 > 30s 告警 |
| `k3s_agent_kubelet_runtime_operations_total{operation}` | kubelet | 失败 > 0 告警 |

### 2.5 NetworkPolicy 健康（K8S-REQ-004）

| Metric 名 | 来源 | 阈值 |
|---|---|---|
| `cilium_policy_l7_parse_errors_total` | Cilium Hubble | > 0 告警 |
| `cilium_drop_count_total{reason}` | Cilium Hubble | PolicyDenied 突增 告警 |
| `cilium_forward_count_total{direction,verdict}` | Cilium Hubble | 趋势 |

> **强约束**: 项目默认用 **Cilium CNI**（非默认 Flannel），提供 L7 NetworkPolicy + Hubble 流日志。Flannel 模式下功能受限。

### 2.6 Ingress（K8S-REQ-005）

| Metric 名 | 来源 | 阈值 |
|---|---|---|
| `nginx_ingress_controller_requests{namespace,ingress}` | ingress-nginx | 趋势 |
| `nginx_ingress_controller_request_duration_seconds_bucket{ingress,status}` | ingress-nginx | p95 / p99 |
| `nginx_ingress_controller_request_size_bytes_bucket` | ingress-nginx | — |
| `nginx_ingress_controller_nginx_up{ingress}` | ingress-nginx | = 0 告警 |
| `nginx_ingress_controller_config_last_reload_successful` | ingress-nginx | = 0 告警 |

## 3. Git 协议可观测性

### 3.1 实现分层（来自 §06-git-server）

| 路径 | 实现 | 理由 |
|---|---|---|
| 读路径（clone / fetch / pull） | gix 0.66 Rust SDK | 性能 + 不依赖外部 git |
| 写路径（push / receive-pack） | shell `git` 子进程 | 100% 兼容 Git LFS / submodule / hooks（ADR-0002） |
| SSH 协议 | russh 0.3 | 自托管 ssh-key 管理 |
| HTTP Smart 协议 | Axum 0.7 + custom handler | 对接 gix 读 / git 写 |

### 3.2 核心指标（GIT-REQ-001）

| Metric 名 | 类型 | 来源 | 阈值 |
|---|---|---|---|
| `gitgit_git_protocol_requests_total{protocol,verb,status_class}` | Counter | 业务埋点 | 趋势 |
| `gitgit_git_protocol_request_duration_seconds{protocol,verb}` | Histogram | 业务埋点 | p95 > 5s 告警 |
| `gitgit_git_pack_bytes_total{protocol,verb,direction}` | Counter | 业务埋点 | 突增告警（拉大对象） |
| `gitgit_git_active_repositories{visibility,has_lfs}` | Gauge | 业务埋点 | 趋势 |
| `gitgit_git_clone_concurrent{repo}` | Gauge | 业务埋点 | > 5 关注（同一 repo） |
| `gitgit_git_push_concurrent{repo}` | Gauge | 业务埋点 | > 3 关注 |
| `gitgit_git_lfs_upload_bytes_total{repo}` | Counter | 业务埋点 | 趋势 |
| `gitgit_git_lfs_download_bytes_total{repo}` | Counter | 业务埋点 | 趋势 |

### 3.3 写路径深度（GIT-REQ-002）

> **重点**: 写路径走 shell `git`，需要深度观测子进程健康。

| Metric 名 | 来源 | 阈值 |
|---|---|---|
| `gitgit_git_write_spawn_duration_seconds` | spawn Command | p99 > 200ms 告警（子进程启动慢） |
| `gitgit_git_write_exit_code_total{exit_code}` | Command 退出 | 非 0 突增告警 |
| `gitgit_git_write_pack_objects_duration_seconds{repo}` | Git pack-objects 阶段 | p99 > 30s 告警 |
| `gitgit_git_write_index_lock_wait_seconds` | 锁等待 | > 5s 告警（多 push 冲突） |
| `gitgit_git_write_pre_receive_hook_duration_seconds` | hook | p99 > 500ms 告警 |
| `gitgit_git_write_post_receive_hook_duration_seconds` | hook | p99 > 2s 告警 |

### 3.4 SSH 协议（GIT-REQ-003）

| Metric 名 | 来源 | 阈值 |
|---|---|---|
| `gitgit_ssh_connections_active{user_class}` | russh | > 1000 关注 |
| `gitgit_ssh_auth_failures_total{user,reason}` | russh | 突增告警（暴力破解） |
| `gitgit_ssh_session_duration_seconds` | russh | p95 |
| `gitgit_ssh_bandwidth_bytes_total{direction}` | russh | 趋势 |

### 3.5 LFS（GIT-REQ-004）

| Metric 名 | 来源 | 阈值 |
|---|---|---|
| `gitgit_lfs_objects_total{repo,bucket}` | 业务 | 趋势 |
| `gitgit_lfs_upload_duration_seconds{repo,size_bucket}` | 业务 | p95 |
| `gitgit_lfs_download_duration_seconds{repo,size_bucket}` | 业务 | p95 |
| `gitgit_lfs_storage_bytes{repo}` | 业务 | 趋势 |
| `gitgit_lfs_orphan_objects_total` | 业务离线任务 | > 0 告警 |

## 4. 中心事件总线可观测性

### 4.1 实现（来自 §01-data-layer）

| 层级 | 实现 |
|---|---|
| 持久化 | PG `coordination.event_journal`（按月分区，BRIN 索引） |
| 实时通知 | PG `LISTEN/NOTIFY`（payload < 8KB） |
| 跨实例分发 | 应用层 `outbox` 模式 + 轮询 worker |
| 幂等 | `idempotency_key` 表 + 唯一约束 |
| 重放 | 顺序 offset + 保存 30 天 |

### 4.2 核心指标（EVT-REQ-001）

| Metric 名 | 类型 | 来源 | 阈值 |
|---|---|---|---|
| `gitgit_event_published_total{topic,tenant,result}` | Counter | 业务埋点 | 趋势 |
| `gitgit_event_publish_duration_seconds{topic}` | Histogram | 业务埋点 | p95 > 50ms 告警 |
| `gitgit_event_consumed_total{topic,consumer,result}` | Counter | 业务埋点 | 失败 > 0 告警 |
| `gitgit_event_consume_duration_seconds{topic,consumer}` | Histogram | 业务埋点 | p95 |
| `gitgit_event_journal_lag_seconds{topic,consumer}` | Gauge | 业务 | > 30s 告警 |
| `gitgit_event_journal_unpublished_count{topic}` | Gauge | SQL | > 1000 告警 |
| `gitgit_event_outbox_pending_bytes{topic}` | Gauge | SQL | > 100MB 告警 |
| `gitgit_event_idempotency_hit_total{topic}` | Counter | 业务 | 趋势 |
| `gitgit_event_dead_letter_total{topic,reason}` | Counter | 业务 | > 0 告警 |

### 4.3 业务语义事件类别（EVT-REQ-002）

| Topic | 来源 | 频率预期 |
|---|---|---|
| `node.created` / `node.updated` / `node.deleted` | tenant 写入 | 高（万/秒级） |
| `edge.*` | tenant 写入 | 高 |
| `view.invalidated` | 视图依赖变更 | 中 |
| `app.installed` / `app.upgraded` | App Registry | 低 |
| `secret.rotated` | Secret | 低 |
| `agent.run.started` / `agent.run.completed` | Agent | 中 |
| `audit.admin.action` | admin_audit | 低 |
| `webhook.delivery.*` | Webhook 出口 | 中 |

### 4.4 健康检查命令（EVT-REQ-003）

```sql
-- 队列积压（per topic）
SELECT
    topic,
    count(*) FILTER (WHERE status = 'pending') AS pending,
    count(*) FILTER (WHERE status = 'consumed') AS consumed,
    max(published_at) FILTER (WHERE status = 'consumed') AS last_consumed
FROM coordination.event_journal
WHERE published_at > now() - interval '1 hour'
GROUP BY topic;

-- 消费延迟（per consumer）
SELECT
    consumer,
    max(now() - published_at) FILTER (WHERE status = 'consumed') AS max_lag,
    avg(now() - published_at) FILTER (WHERE status = 'consumed') AS avg_lag
FROM coordination.event_journal
WHERE published_at > now() - interval '5 minutes'
GROUP BY consumer;

-- outbox 健康
SELECT
    topic,
    count(*) AS pending_count,
    pg_size_pretty(sum(octet_length(payload))) AS total_size
FROM coordination.outbox
WHERE status = 'pending'
GROUP BY topic;
```

## 5. App Bus（Wasm hostcall）可观测性

### 5.1 沙箱模型（ADR-0010）

| 项 | 值 |
|---|---|
| Runtime | wasmtime 24+ |
| 能力模型 | WASI Preview1 + 自定义 hostcall |
| 资源限制 | fuel = 10M / memory = 64MB / wall = 30s |
| 调用方向 | App → Host（单向，host 不能调 App） |
| 状态 | 每个 App Run 一个独立实例 |

### 5.2 核心指标（APP-REQ-009）

| Metric 名 | 来源 | 阈值 |
|---|---|---|
| `gitgit_app_bus_invocations_total{app,call,verdict}` | wasmtime host | 趋势 |
| `gitgit_app_bus_invoke_duration_seconds{app,call}` | wasmtime host | p95 |
| `gitgit_app_bus_capability_denied_total{app,call,capability}` | wasmtime host | 突增告警（App 越权尝试） |
| `gitgit_app_bus_fuel_exhausted_total{app}` | wasmtime host | > 0 告警（计算资源不足） |
| `gitgit_app_bus_memory_exceeded_total{app}` | wasmtime host | > 0 告警 |
| `gitgit_app_bus_wall_timeout_total{app}` | wasmtime host | > 0 告警 |
| `gitgit_app_bus_panic_total{app}` | wasmtime host | > 0 告警 |
| `gitgit_app_concurrent_instances{app,status}` | 业务 | 趋势 |
| `gitgit_app_instance_duration_seconds{app}` | 业务 | p95 |

### 5.3 资源限制健康（APP-REQ-010）

| Metric 名 | 来源 | 阈值 |
|---|---|---|
| `gitgit_app_fuel_used_ratio{app,instance}` | wasmtime | > 0.9 告警（接近耗尽） |
| `gitgit_app_memory_used_bytes{app,instance}` | wasmtime | > 60MB 告警（接近限制） |
| `gitgit_app_instance_lifecycle_state{app,state}` | 业务 | 状态分布 |

### 5.4 Capability 拒绝明细（APP-REQ-011）

```promql
# Top 10 capability 拒绝
topk(10,
  sum by (app, call, capability) (
    rate(gitgit_app_bus_capability_denied_total[5m])
  )
)
```

## 6. 未来中间件引入规约（FORWARD-LOOKING）

> **强约束**: 当前架构**不**引入以下中间件。如未来评估需要新增，**必须**先满足以下可观测性前置条件。

### 6.1 Kafka（如未来引入）

> 必装 exporters: `kafka_exporter` :9308
> 关键指标: `kafka_consumergroup_lag` / `kafka_topic_partition_current_offset` / `kafka_server_broker_topic_metrics_messages_in_total`
> 必带采样: producer / consumer 端到端 trace（kafka 头注入 traceparent）

### 6.2 Redis（如未来引入）

> 必装 exporters: `redis_exporter` :9121
> 关键指标: `redis_connected_clients` / `redis_memory_used_bytes` / `redis_keyspace_hits_total` / `redis_keyspace_misses_total`
> **警告**: 命中率 < 90% 必须调查（避免成为 PG 旁路）

### 6.3 对象存储 S3（如未来引入 LFS 替代）

> 必装 exporters: 自定义 S3 metrics（list_objects / head_object 延迟）
> 关键指标: `s3_request_duration_seconds{operation,bucket}` / `s3_5xx_errors_total{bucket}`

### 6.4 外部 API（AI 网关 / OIDC IdP）

> 实现位置: `gitgit-ai` / `core_auth`
> 关键指标: `gitgit_external_api_requests_total{vendor,endpoint,result}` / `gitgit_external_api_duration_seconds{vendor,endpoint}` / `gitgit_external_api_rate_limited_total{vendor}`

## 7. OTel Collector 自身可观测性

### 7.1 内部 metrics（自报）

OTel Collector 内置自报端点 :8888（Prometheus 格式），必监控:

| Metric 名 | 来源 | 阈值 |
|---|---|---|
| `otelcol_exporter_queue_size{exporter}` | OTel | > 5000 告警（积压） |
| `otelcol_exporter_queue_capacity{exporter}` | OTel | 队列已满丢弃告警 |
| `otelcol_exporter_send_failed_metric_points_total` | OTel | > 0 告警 |
| `otelcol_exporter_sent_metric_points_total` | OTel | 趋势 |
| `otelcol_processor_batch_batch_send_size` | OTel | 趋势 |
| `otelcol_processor_batch_timeout_trigger_send_total` | OTel | 趋势 |
| `otelcol_receiver_accepted_metric_points_total{receiver}` | OTel | 趋势 |
| `otelcol_receiver_refused_metric_points_total{receiver}` | OTel | > 0 告警 |
| `otelcol_exporter_send_failed_spans_total` | OTel | > 0 告警 |
| `otelcol_exporter_send_failed_log_records_total` | OTel | > 0 告警 |

### 7.2 健康探针

| 探针 | 端点 | 间隔 |
|---|---|---|
| Liveness | `http://otel-collector:13133/` | 10s |
| Readiness | `http://otel-collector:13133/` | 5s |
| 自报 metrics | `http://otel-collector:8888/metrics` | 15s scrape |
| zPages（debug） | `http://otel-collector:55679/debug/...` | 仅 debug 模式启用 |

## 8. 关联文档

- 上游: [`01-architecture.md`](01-architecture.md) §2 组件清单
- 上游: [`02-metrics.md`](02-metrics.md) §5 应用层
- 上游: [`04-tracing.md`](04-tracing.md) §3 跨服务 trace
- 下游: [`07-dashboard-design.md`](07-dashboard-design.md) §5 50-Middleware
- 下游: [`08-alert-design.md`](08-alert-design.md) §3 中间件告警
- 下游: [`11-deployment-design.md`](11-deployment-design.md) §3 部署清单

## 9. 需求 ID 索引

| 需求 ID | 标题 | 优先级 |
|---|---|---|
| OBS-REQ-006 | 中间件可观测性 | P0 |
| OBS-REQ-013 | 统一接入 | P0 |
| K8S-REQ-001 | Node 核心 | P0 |
| K8S-REQ-002 | Pod / Deployment | P0 |
| K8S-REQ-003 | K3s Control Plane | P0 |
| K8S-REQ-004 | NetworkPolicy | P1 |
| K8S-REQ-005 | Ingress | P1 |
| GIT-REQ-001 | Git 协议核心 | P0 |
| GIT-REQ-002 | Git 写路径深度 | P0 |
| GIT-REQ-003 | SSH 协议 | P1 |
| GIT-REQ-004 | LFS | P1 |
| EVT-REQ-001 | 中心事件核心 | P0 |
| EVT-REQ-002 | 业务语义事件 | P1 |
| EVT-REQ-003 | 事件总线健康 | P0 |
| APP-REQ-009 | App Bus 调用 | P0 |
| APP-REQ-010 | App 资源限制 | P0 |
| APP-REQ-011 | Capability 拒绝 | P0 |
