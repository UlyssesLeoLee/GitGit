# Metrics 设计 / Metrics Design

> **关联 OBS-REQ**: OBS-MET-001 ~ OBS-MET-020
> **原则**: RED（Rate / Errors / Duration）+ USE（Utilization / Saturation / Errors）
> **统一 SDK**: `opentelemetry` (0.24) + `opentelemetry-prometheus` (0.16) + `prometheus` (0.13)
> **业务代码**: 通过 `crates/gitgit-observability` 间接使用

## 1. 命名约定

```
<namespace>_<subject>_<verb>[_qualifier]_{unit}
```

- **小写 + 下划线**（Prometheus 风格）
- 单位后缀（`_seconds`, `_bytes`, `_total`, `_ratio`）
- 例：`http_server_request_duration_seconds`, `db_connection_pool_size`

## 2. Infra Metrics (Node / Container / OS)

来自 `node_exporter` + `cAdvisor` + `kubelet`。

### 2.1 CPU

| Metric | 类型 | 标签 | 单位 | 关联 |
|---|---|---|---|---|
| `node_cpu_utilization_ratio` | Gauge | `node`, `mode` (user/system/iowait) | ratio 0-1 | USE-Utilization |
| `node_cpu_load1` | Gauge | `node` | — | OS load |
| `node_cpu_saturation_steal_ratio` | Gauge | `node` | ratio | USE-Saturation |

### 2.2 Memory

| Metric | 类型 | 标签 | 单位 |
|---|---|---|---|
| `node_memory_utilization_bytes` | Gauge | `node` | bytes |
| `node_memory_available_bytes` | Gauge | `node` | bytes |
| `node_memory_swap_used_bytes` | Gauge | `node` | bytes |
| `node_memory_cached_bytes` | Gauge | `node` | bytes |

### 2.3 Disk

| Metric | 类型 | 标签 | 单位 |
|---|---|---|---|
| `node_disk_io_read_bytes_total` | Counter | `device` | bytes |
| `node_disk_io_write_bytes_total` | Counter | `device` | bytes |
| `node_disk_usage_ratio` | Gauge | `mountpoint` | ratio |
| `node_disk_inodes_free` | Gauge | `mountpoint` | — |
| `node_filesystem_files` | Gauge | `mountpoint`, `fstype` | — |

### 2.4 Network

| Metric | 类型 | 标签 | 单位 |
|---|---|---|---|
| `node_network_receive_bytes_total` | Counter | `device` | bytes |
| `node_network_transmit_bytes_total` | Counter | `device` | bytes |
| `node_network_drops_total` | Counter | `device` | packets |
| `node_network_errors_total` | Counter | `device` | packets |

## 3. Container Metrics

来自 cAdvisor（kubelet 内建）。

| Metric | 类型 | 标签 | 关联 OBS-REQ |
|---|---|---|---|
| `container_cpu_usage_seconds_total` | Counter | `namespace`, `pod`, `container` | OBS-MET-010 |
| `container_memory_usage_bytes` | Gauge | `namespace`, `pod`, `container` | OBS-MET-011 |
| `container_network_io_bytes_total` | Counter | `namespace`, `pod` | OBS-MET-012 |
| `container_fs_usage_bytes` | Gauge | `namespace`, `pod`, `container` | OBS-MET-013 |
| `container_oom_kills_total` | Counter | `namespace`, `pod`, `container` | OBS-MET-014 |
| `container_restarts_total` | Counter | `namespace`, `pod`, `container` | OBS-MET-015 |

## 4. Kubernetes Metrics

来自 `kube-state-metrics`。

| Metric | 类型 | 标签 | 关联 |
|---|---|---|---|
| `kube_pod_status_phase` | Gauge | `namespace`, `pod`, `phase` | OBS-MET-020 |
| `kube_pod_container_status_restarts_total` | Counter | `namespace`, `pod`, `container` | CrashLoopBackOff 检测 |
| `kube_pod_container_status_waiting_reason` | Gauge | `namespace`, `pod`, `container`, `reason` | CrashLoopBackOff / ImagePullBackOff / ErrImagePull |
| `kube_deployment_status_replicas` | Gauge | `namespace`, `deployment` | 副本状态 |
| `kube_deployment_spec_replicas` | Gauge | `namespace`, `deployment` | 期望副本 |
| `kube_deployment_status_replicas_unavailable` | Gauge | `namespace`, `deployment` | 滚动升级 |
| `kube_node_status_condition` | Gauge | `node`, `condition` | Node Ready / NotReady |
| `kube_job_status_active` | Gauge | `namespace`, `job` | 批处理状态 |

## 5. Application Metrics (RED + USE)

### 5.1 HTTP Server (gitgit-server :3000 + gitgit-admin :3001)

| Metric | 类型 | 标签 | 阶段 | 关联 |
|---|---|---|---|---|
| `http_server_requests_total` | Counter | `service`, `method`, `path` (template), `status_class` (2xx/4xx/5xx) | RED-Rate | OBS-MET-101 |
| `http_server_request_duration_seconds` | Histogram | `service`, `method`, `path`, `status_class` | RED-Duration | OBS-MET-102 |
| `http_server_requests_in_flight` | Gauge | `service` | USE-Saturation | OBS-MET-103 |
| `http_server_request_body_size_bytes` | Histogram | `service`, `method`, `path` | — | OBS-MET-104 |
| `http_server_response_size_bytes` | Histogram | `service`, `method`, `path` | — | OBS-MET-105 |

**路径模板**（避免 cardinality 爆炸）：
- `POST /api/v1/repos/{id}` 而非完整 URL
- `GET /api/v1/graph/nodes/{id}` 模板化
- 静态资源 `/static/*` 单 bucket

### 5.2 gRPC Server (gitgit-server :50051)

| Metric | 类型 | 标签 | 阶段 | 关联 |
|---|---|---|---|---|
| `grpc_server_started_total` | Counter | `service`, `method` | RED-Rate | OBS-MET-110 |
| `grpc_server_handled_total` | Counter | `service`, `method`, `status` (OK/ERROR) | RED-Errors | OBS-MET-111 |
| `grpc_server_handled_duration_seconds` | Histogram | `service`, `method` | RED-Duration | OBS-MET-112 |
| `grpc_server_stream_messages_sent_total` | Counter | `service`, `method` | — | OBS-MET-113 |
| `grpc_server_stream_messages_received_total` | Counter | `service`, `method` | — | OBS-MET-114 |

### 5.3 Database (PostgreSQL via sqlx + pg_exporter)

| Metric | 类型 | 标签 | 阶段 | 关联 |
|---|---|---|---|---|
| `db_connections_active` | Gauge | `pool` | USE-Saturation | OBS-MET-120 |
| `db_connections_idle` | Gauge | `pool` | USE-Utilization | OBS-MET-121 |
| `db_connections_max` | Gauge | `pool` | USE | OBS-MET-122 |
| `db_query_duration_seconds` | Histogram | `op` (select/insert/update/delete), `table` | RED-Duration | OBS-MET-123 |
| `db_query_errors_total` | Counter | `op`, `error_kind` | RED-Errors | OBS-MET-124 |
| `db_transactions_total` | Counter | `result` (commit/rollback) | — | OBS-MET-125 |
| `db_deadlocks_total` | Counter | `database` | USE-Errors | OBS-MET-126 |
| `db_locks_held` | Gauge | `database` | USE-Saturation | OBS-MET-127 |
| `db_cache_hit_ratio` | Gauge | `database` | — | OBS-MET-128 |
| `db_slow_queries_total` | Counter | `database` (阈值 > 1s) | RED-Errors | OBS-MET-129 |

**关键查询（pg_stat_statements）**：
- `pg_stat_user_tables` (seq_scan / idx_scan / n_tup_ins/upd/del)
- `pg_stat_user_indexes` (idx_scan / idx_tup_read / idx_tup_fetch)
- `pg_stat_activity` (active connections, longest transaction)
- `pg_locks` (held locks)
- `pg_stat_statements` (query timing, planning time)

### 5.4 Git Service (gix + shell git)

| Metric | 类型 | 标签 | 关联 |
|---|---|---|---|
| `git_operations_total` | Counter | `op` (read/write/clone/push), `result` (ok/error) | RED-Rate + RED-Errors |
| `git_operation_duration_seconds` | Histogram | `op` | RED-Duration |
| `git_objects_read_total` | Counter | `object_type` (blob/tree/commit/tag) | — |
| `git_objects_written_total` | Counter | `object_type` | — |
| `git_repo_size_bytes` | Gauge | `repo_id` (UUID) | — |
| `git_pack_objects_total` | Counter | `result` | — |
| `git_gc_duration_seconds` | Histogram | `op` (gc/pack) | — |
| `git_hook_execution_total` | Counter | `hook` (pre-receive/post-receive), `result` | — |

### 5.5 App / Plugin Subsystem (Wasm + WASI)

| Metric | 类型 | 标签 | 关联 |
|---|---|---|---|
| `app_install_total` | Counter | `app_id`, `result` | — |
| `app_uninstall_total` | Counter | `app_id`, `result` | — |
| `app_upgrade_total` | Counter | `app_id`, `strategy` (rolling/blue_green/canary), `result` | — |
| `app_upgrade_duration_seconds` | Histogram | `strategy`, `result` | — |
| `app_heartbeat_stale_seconds` | Gauge | `app_id`, `instance_id` | NFR-REQ-005 |
| `app_instance_count` | Gauge | `app_id`, `status` (healthy/degraded/failed) | — |
| `app_sandbox_violations_total` | Counter | `app_id`, `violation_type` | AISEC-REQ-013 |
| `app_manifest_validation_failures_total` | Counter | `app_id`, `reason` | — |

### 5.6 Event Bus (中心事件)

| Metric | 类型 | 标签 | 关联 |
|---|---|---|---|
| `eventbus_published_total` | Counter | `event_type`, `target` (specific/broadcast) | NFR-REQ-006 |
| `eventbus_publish_duration_seconds` | Histogram | `target` | — |
| `eventbus_delivered_total` | Counter | `event_type`, `subscriber`, `result` | — |
| `eventbus_delivery_duration_seconds` | Histogram | `subscriber` | — |
| `eventbus_outbox_pending` | Gauge | `target` | — |
| `eventbus_outbox_oldest_pending_seconds` | Gauge | — | 30s 告警 |
| `eventbus_dlq_size` | Gauge | `subscriber` | 阈值 > 100 |
| `eventbus_retry_total` | Counter | `subscriber`, `attempt` | — |

### 5.7 AI Gateway

| Metric | 类型 | 标签 | 关联 |
|---|---|---|---|
| `ai_request_total` | Counter | `provider`, `model`, `result` | — |
| `ai_request_duration_seconds` | Histogram | `provider`, `model` | RED-Duration |
| `ai_request_tokens_total` | Counter | `provider`, `model`, `direction` (input/output) | — |
| `ai_request_cost_microcents` | Counter | `provider`, `model` | — |
| `ai_provider_errors_total` | Counter | `provider`, `error_kind` | RED-Errors |
| `ai_rate_limit_total` | Counter | `provider` | — |
| `ai_prompt_injection_detected_total` | Counter | `app_id`, `pattern` | AISEC-REQ-001 |
| `ai_mcp_tool_invocations_total` | Counter | `tool`, `result` (allow/deny/error) | AISEC-REQ-002 |
| `ai_mcp_tool_denied_total` | Counter | `tool`, `reason` | — |
| `ai_cache_hits_total` | Counter | `kind` (exact/semantic) | — |
| `ai_cache_hit_ratio` | Gauge | `kind` | — |

### 5.8 Agent Runtime

| Metric | 类型 | 标签 | 关联 |
|---|---|---|---|
| `agent_run_total` | Counter | `agent_id`, `state`, `result` | AGT-REQ |
| `agent_run_duration_seconds` | Histogram | `agent_id` | — |
| `agent_state_transitions_total` | Counter | `from_state`, `to_state` | — |
| `agent_awaiting_approval_seconds` | Histogram | `agent_id` | 24h 告警 |
| `agent_workspace_size_bytes` | Gauge | `run_id` | — |
| `agent_resource_cpu_seconds` | Counter | `run_id` | — |
| `agent_resource_memory_peak_bytes` | Gauge | `run_id` | — |
| `agent_credentials_active` | Gauge | `subject_type` | — |
| `agent_credentials_issued_total` | Counter | `subject_type` | — |
| `agent_subagent_invocations_total` | Counter | `parent_agent_id` | — |

### 5.9 Policy Engine

| Metric | 类型 | 标签 | 关联 |
|---|---|---|---|
| `policy_evaluations_total` | Counter | `result` (allow/deny) | — |
| `policy_evaluation_duration_seconds` | Histogram | `policy_type` | — |
| `policy_cache_hit_ratio` | Gauge | — | — |
| `policy_denied_total` | Counter | `action`, `resource_type` | — |
| `policy_matched_policies` | Histogram | `policy_id` | — |

### 5.10 Auth / Security

| Metric | 类型 | 标签 | 关联 |
|---|---|---|---|
| `auth_login_attempts_total` | Counter | `result` (ok/fail), `mfa` (yes/no) | — |
| `auth_token_issued_total` | Counter | `token_type` (access/refresh) | — |
| `auth_token_validations_total` | Counter | `result` (ok/expired/invalid) | — |
| `auth_mfa_required_total` | Counter | `result` (enrolled/skipped) | SEC-REQ-012 |
| `auth_mfa_failures_total` | Counter | `user_id` (low-card) | — |
| `auth_webauthn_assertions_total` | Counter | `result` (ok/fail), `reason` | — |
| `auth_password_failures_total` | Counter | — | Brute force 检测 |
| `kek_rotation_total` | Counter | `result` (ok/fail) | SEC-REQ-010 |
| `admin_audit_logged_total` | Counter | `action`, `result` | SEC-REQ-003 / 011 |

### 5.11 Self-Test / Health

| Metric | 类型 | 标签 | 关联 |
|---|---|---|---|
| `selftest_total` | Counter | `check`, `result` (ok/fail) | — |
| `selftest_duration_seconds` | Histogram | `check` | — |
| `health_up` | Gauge | `instance` | K8s readiness |
| `dependency_up` | Gauge | `dependency` (db/redis/git_provider) | — |

## 6. RED/USE 对照

| 服务 | Rate | Errors | Duration | Utilization | Saturation |
|---|---|---|---|---|---|
| gitgit-server (HTTP) | ✅ http_server_requests_total | ✅ via status_class | ✅ http_server_request_duration_seconds | ✅ container_cpu_usage | ✅ http_server_requests_in_flight |
| gitgit-server (gRPC) | ✅ grpc_server_started_total | ✅ grpc_server_handled_total {status=ERROR} | ✅ grpc_server_handled_duration_seconds | ✅ | — |
| PostgreSQL | ✅ db_query_total | ✅ db_query_errors_total | ✅ db_query_duration_seconds | ✅ db_connections_active/max | ✅ db_connections_active |
| Git 服务 | ✅ git_operations_total | ✅ git_operations_total {result=error} | ✅ git_operation_duration_seconds | — | — |
| Event Bus | ✅ eventbus_published_total | — | ✅ eventbus_publish_duration_seconds | ✅ eventbus_outbox_pending | ✅ eventbus_outbox_oldest_pending_seconds |
| App 沙箱 | ✅ app_install_total | ✅ {result=error} | ✅ app_upgrade_duration_seconds | ✅ app_instance_count | ✅ app_heartbeat_stale_seconds |
| AI 网关 | ✅ ai_request_total | ✅ ai_provider_errors_total | ✅ ai_request_duration_seconds | — | ✅ ai_rate_limit_total |
| Agent | ✅ agent_run_total | ✅ {result=error} | ✅ agent_run_duration_seconds | ✅ agent_credentials_active | ✅ agent_awaiting_approval_seconds |

## 7. Cardinality 监控

每个 metric 必须记录：
- `metric_name`
- `cardinality_estimate` (每天 unique label combos)
- 实际部署后写入 PromQL `topk(10, count by(__name__)({...}))` 定期 review

## 8. Exemplar（关联 Trace）

只有 Histogram 配 Exemplar：

```rust
metrics::histogram!("http_server_request_duration_seconds", "method" => "POST", "path" => "/api/v1/repos", "status_class" => "2xx")
    .record(duration.as_secs_f64());
```

OTel SDK 自动注入当前 `trace_id` / `span_id` 作为 exemplar。Grafana 点 metric bucket → 跳到 Tempo trace。

## 9. 关联 OBS-MET

- **OBS-MET-001 ~ 020** (infra/k8s) → §2-4
- **OBS-MET-101 ~ 105** (HTTP) → §5.1
- **OBS-MET-110 ~ 114** (gRPC) → §5.2
- **OBS-MET-120 ~ 129** (DB) → §5.3
- (其他 详见各小节末尾)
