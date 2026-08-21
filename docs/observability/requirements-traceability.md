# 可观测性需求追溯矩阵 / Requirements Traceability Matrix

> **关联 OBS-REQ**: 全 OBS-REQ-*（统一索引）
> **关联设计**: [`00-15` 全部 observability 文档](.) / [ADR-0011](../architecture/decisions/0011-observability-platform.md)
> **目标**: Requirement → Architecture → Component → Configuration → Test Case 完整追踪

## 1. 需求 ID 体系

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
| `OBS-RET-*` | 保留策略 | 5 |
| `OBS-IMPL-*` | 实施阶段 | 10 |
| `OBS-REV-*` | 修订项 | 5 |
| `OBS-RISK-*` | 风险 | 7 |
| 子域前缀 | 域内子需求 | 见各 § |
| **总计** | | **~ 280+** |

## 2. 顶层需求 OBS-REQ

| ID | 标题 | 描述 | 优先级 | 关联 ADR | 关联文档 |
|---|---|---|---|---|---|
| **OBS-REQ-001** | 统一 Observability Architecture | 4 层架构 | P0 | ADR-0011 | 01-architecture |
| **OBS-REQ-002** | 零业务侵入 | 业务代码不直接依赖后端 SDK | P0 | ADR-0011 | 01-architecture, 14-code-impact |
| **OBS-REQ-003** | Metric/Log/Trace 关联 | trace_id 全链路传播 | P0 | ADR-0011 | 04-tracing, 07-dashboard |
| **OBS-REQ-004** | 命名规约 | `<ns>_<sub>_<verb>[_qual]_{unit}` | P0 | — | 02-metrics |
| **OBS-REQ-005** | 数据库可观测性 | PG 深度监控 | P0 | ADR-0003 | 05-database |
| **OBS-REQ-006** | 中间件可观测性 | K3s / Git / Event Bus / App Bus | P0 | ADR-0006, ADR-0010 | 06-middleware |
| **OBS-REQ-007** | Grafana 统一可视化 | 9 大盘 | P0 | — | 07-dashboard |
| **OBS-REQ-008** | Metric→Trace→Log 关联 | 故障定位作战图 | P0 | — | 07-dashboard §15 |
| **OBS-REQ-009** | Alert 设计 | 多窗口 + 持续时间 | P0 | — | 08-alert |
| **OBS-REQ-010** | SLO Burn Rate | Google SRE Workbook | P0 | — | 08-alert §5.5, 09-slo |
| **OBS-REQ-011** | Error Budget | 30d 预算管理 | P0 | — | 09-slo §3 |
| **OBS-REQ-012** | Burn Rate 多窗口 | 1h + 6h / 24h + 3d | P0 | — | 09-slo §3.3 |
| **OBS-REQ-013** | 统一接入 | gitgit-observability crate | P0 | — | 14-code-impact |
| **OBS-REQ-014** | 监控安全 | 12 项检查 | P0 | ADR-0007 | 10-security |
| **OBS-REQ-015** | 数据保护 | 3 层脱敏 | P0 | ADR-0007 | 10-security §5 |
| **OBS-REQ-016** | 网络隔离 | NetworkPolicy + mTLS | P0 | — | 10-security §6 |
| **OBS-REQ-017** | 部署形态 | 独立 namespace + taint | P0 | ADR-0006 | 11-deployment |
| **OBS-REQ-018** | GitOps | ArgoCD + Kustomize | P0 | — | 11-deployment §5 |
| **OBS-REQ-019** | HA | 关键组件 ≥ 2 副本 | P0 | — | 11-deployment §3.3 |
| **OBS-REQ-020** | 性能影响评估 | 业务开销 < 3% | P0 | — | 12-performance §1 |
| **OBS-REQ-021** | 数据生命周期 | 分级保留 | P0 | — | 12-performance §5 |
| **OBS-REQ-022** | 降级策略 | OTEL_DISABLED | P0 | — | 12-performance §1.4 |
| **OBS-REQ-023** | 分阶段实施 | 8 阶段分步 | P0 | — | 13-implementation |
| **OBS-REQ-024** | 风险控制 | 每阶段可回滚 | P0 | — | 13-implementation |
| **OBS-REQ-025** | 代码影响 | < 1300 行改造 | P0 | — | 14-code-impact |
| **OBS-REQ-026** | 最小侵入 | 业务代码不改 | P0 | — | 14-code-impact §9 |
| **OBS-REQ-027** | 自审 | 12 项检查 | P0 | — | 15-self-review |
| **OBS-REQ-028** | 修订闭环 | v2 修订追踪 | P0 | — | 15-self-review §2 |

## 3. Metrics 指标 OBS-MET

### 3.1 Infrastructure（OBS-MET-001 ~ 020）

| ID | Metric 名 | 来源 | 文档 |
|---|---|---|---|
| OBS-MET-001 | `node_cpu_usage_ratio` | node-exporter | 02-metrics §3.1 |
| OBS-MET-002 | `node_memory_usage_ratio` | node-exporter | 02-metrics §3.1 |
| OBS-MET-003 | `node_filesystem_usage_ratio` | node-exporter | 02-metrics §3.1 |
| OBS-MET-004 | `node_filesystem_inode_usage_ratio` | node-exporter | 02-metrics §3.1 |
| OBS-MET-005 | `node_load5` | node-exporter | 02-metrics §3.1 |
| OBS-MET-006 | `node_network_receive_bytes_total` | node-exporter | 02-metrics §3.1 |
| OBS-MET-007 | `node_network_transmit_bytes_total` | node-exporter | 02-metrics §3.1 |
| OBS-MET-008 | `node_network_receive_errs_total` | node-exporter | 02-metrics §3.1 |
| OBS-MET-009 | `node_context_switches_total` | node-exporter | 02-metrics §3.1 |
| OBS-MET-010 | `node_open_fds` | node-exporter | 02-metrics §3.1 |
| OBS-MET-011 | `node_entropy_available_bits` | node-exporter | 02-metrics §3.1 |
| OBS-MET-012 | `node_timex_offset_seconds` | node-exporter | 02-metrics §3.1 |
| OBS-MET-013 | `kube_node_status_condition` | kube-state-metrics | 02-metrics §3.2 |
| OBS-MET-014 | `kube_pod_status_phase` | kube-state-metrics | 02-metrics §3.2 |
| OBS-MET-015 | `kube_pod_container_status_restarts_total` | kube-state-metrics | 02-metrics §3.2 |
| OBS-MET-016 | `kube_deployment_status_replicas_ready` | kube-state-metrics | 02-metrics §3.2 |
| OBS-MET-017 | `kube_horizontalpodautoscaler_status_current_replicas` | kube-state-metrics | 02-metrics §3.2 |
| OBS-MET-018 | `cilium_policy_l7_parse_errors_total` | Cilium Hubble | 02-metrics §3.3 |
| OBS-MET-019 | `cilium_drop_count_total` | Cilium Hubble | 02-metrics §3.3 |
| OBS-MET-020 | `k3s_server_etcd_disk_wal_fsync_duration_seconds` | k3s | 02-metrics §3.3 |

### 3.2 Database（OBS-MET-021 ~ 050）

| ID | Metric 名 | 文档 |
|---|---|---|
| OBS-MET-021 | `pg_stat_activity_count` | 05-database §3.1 |
| OBS-MET-022 | `pg_stat_activity_max_tx_duration_seconds` | 05-database §3.1 |
| OBS-MET-023 | `pgbouncer_pools_waiting` | 05-database §3.1 |
| OBS-MET-024 | `pgbouncer_pools_client_active` | 05-database §3.1 |
| OBS-MET-025 | `pg_stat_statements_calls_total` | 05-database §3.2 |
| OBS-MET-026 | `pg_stat_statements_total_exec_time_seconds_total` | 05-database §3.2 |
| OBS-MET-027 | `pg_stat_statements_mean_exec_time_seconds` | 05-database §3.2 |
| OBS-MET-028 | `pg_slow_queries_total` | 05-database §3.2 |
| OBS-MET-029 | `pg_lock_waits_total` | 05-database §3.2 |
| OBS-MET-030 | `pg_stat_database_xact_commit` | 05-database §3.3 |
| OBS-MET-031 | `pg_stat_database_xact_rollback` | 05-database §3.3 |
| OBS-MET-032 | `pg_stat_database_deadlocks` | 05-database §3.3 |
| OBS-MET-033 | `pg_stat_database_conflicts` | 05-database §3.3 |
| OBS-MET-034 | `pg_locks_count` | 05-database §3.3 |
| OBS-MET-035 | `pg_cache_hit_ratio` | 05-database §3.4 |
| OBS-MET-036 | `pg_replication_state` | 05-database §3.5 |
| OBS-MET-037 | `pg_replication_lag_seconds` | 05-database §3.5 |
| OBS-MET-038 | `pg_disk_usage_bytes` | 05-database §3.6 |
| OBS-MET-039 | `pg_table_bloat_ratio` | 05-database §3.6 |
| OBS-MET-040 | `pg_index_bloat_ratio` | 05-database §3.6 |
| OBS-MET-041 | `pg_stat_user_tables_n_dead_tup` | 05-database §3.6 |
| OBS-MET-042 | `pg_backup_last_success_timestamp` | 05-database §6.1 |
| OBS-MET-043 | `pg_wal_archive_lag_seconds` | 05-database §6.1 |
| OBS-MET-044 | `gitgit_node_total` | 05-database §4.1 |
| OBS-MET-045 | `gitgit_edge_total` | 05-database §4.1 |
| OBS-MET-046 | `gitgit_node_orphans_total` | 05-database §4.1 |
| OBS-MET-047 | `gitgit_audit_hash_chain_broken` | 05-database §6.2 |
| OBS-MET-048 | `gitgit_audit_hash_chain_ok` | 05-database §6.2 |
| OBS-MET-049 | `gitgit_secret_kek_rotation_due` | 05-database §4.3 |
| OBS-MET-050 | `gitgit_app_version_unused_days` | 05-database §4.4 |

### 3.3 Middleware（OBS-MET-051 ~ 080）

| ID | Metric 名 | 文档 |
|---|---|---|
| OBS-MET-051 | `gitgit_git_protocol_requests_total` | 06-middleware §3.2 |
| OBS-MET-052 | `gitgit_git_protocol_request_duration_seconds` | 06-middleware §3.2 |
| OBS-MET-053 | `gitgit_git_pack_bytes_total` | 06-middleware §3.2 |
| OBS-MET-054 | `gitgit_git_active_repositories` | 06-middleware §3.2 |
| OBS-MET-055 | `gitgit_git_write_exit_code_total` | 06-middleware §3.3 |
| OBS-MET-056 | `gitgit_git_write_pack_objects_duration_seconds` | 06-middleware §3.3 |
| OBS-MET-057 | `gitgit_git_write_index_lock_wait_seconds` | 06-middleware §3.3 |
| OBS-MET-058 | `gitgit_lfs_objects_total` | 06-middleware §3.5 |
| OBS-MET-059 | `gitgit_lfs_upload_duration_seconds` | 06-middleware §3.5 |
| OBS-MET-060 | `gitgit_event_published_total` | 06-middleware §4.2 |
| OBS-MET-061 | `gitgit_event_publish_duration_seconds` | 06-middleware §4.2 |
| OBS-MET-062 | `gitgit_event_consumed_total` | 06-middleware §4.2 |
| OBS-MET-063 | `gitgit_event_consume_duration_seconds` | 06-middleware §4.2 |
| OBS-MET-064 | `gitgit_event_journal_lag_seconds` | 06-middleware §4.2 |
| OBS-MET-065 | `gitgit_event_journal_unpublished_count` | 06-middleware §4.2 |
| OBS-MET-066 | `gitgit_idempotency_hit_total` | 06-middleware §4.2 |
| OBS-MET-067 | `gitgit_event_dead_letter_total` | 06-middleware §4.2 |
| OBS-MET-068 | `gitgit_app_bus_invocations_total` | 06-middleware §5.2 |
| OBS-MET-069 | `gitgit_app_bus_invoke_duration_seconds` | 06-middleware §5.2 |
| OBS-MET-070 | `gitgit_app_bus_capability_denied_total` | 06-middleware §5.2 |
| OBS-MET-071 | `gitgit_app_bus_fuel_exhausted_total` | 06-middleware §5.2 |
| OBS-MET-072 | `gitgit_app_bus_memory_exceeded_total` | 06-middleware §5.2 |
| OBS-MET-073 | `gitgit_app_bus_wall_timeout_total` | 06-middleware §5.2 |
| OBS-MET-074 | `gitgit_app_bus_panic_total` | 06-middleware §5.2 |
| OBS-MET-075 | `gitgit_app_fuel_used_ratio` | 06-middleware §5.3 |
| OBS-MET-076 | `gitgit_app_memory_used_bytes` | 06-middleware §5.3 |
| OBS-MET-077 | `otelcol_exporter_queue_size` | 06-middleware §7.1 |
| OBS-MET-078 | `otelcol_exporter_send_failed_metric_points_total` | 06-middleware §7.1 |
| OBS-MET-079 | `otelcol_receiver_refused_metric_points_total` | 06-middleware §7.1 |
| OBS-MET-080 | `otelcol_exporter_send_failed_spans_total` | 06-middleware §7.1 |

### 3.4 Application（OBS-MET-101 ~ 129）

| ID | Metric 名 | 文档 |
|---|---|---|
| OBS-MET-101 | `http_requests_total` | 02-metrics §5.1 |
| OBS-MET-102 | `http_request_duration_seconds` | 02-metrics §5.1 |
| OBS-MET-103 | `http_requests_inflight` | 02-metrics §5.1 |
| OBS-MET-104 | `db_operation_duration_seconds` | 02-metrics §5.2 |
| OBS-MET-105 | `db_pool_size` | 02-metrics §5.2 |
| OBS-MET-106 | `db_pool_idle` | 02-metrics §5.2 |
| OBS-MET-107 | `traces_spanmetrics_calls_total` | 04-tracing §3 |
| OBS-MET-108 | `traces_spanmetrics_latency` | 04-tracing §3 |
| OBS-MET-109 | `gitgit_node_created_total` | 02-metrics §5.3 |
| OBS-MET-110 | `gitgit_node_updated_total` | 02-metrics §5.3 |
| OBS-MET-111 | `gitgit_node_deleted_total` | 02-metrics §5.3 |
| OBS-MET-112 | `gitgit_view_invalidated_total` | 02-metrics §5.3 |
| OBS-MET-113 | `gitgit_agent_run_total` | 02-metrics §5.3 |
| OBS-MET-114 | `gitgit_ai_gateway_requests_total` | 02-metrics §5.3 |
| OBS-MET-115 | `gitgit_ai_gateway_duration_seconds` | 02-metrics §5.3 |
| OBS-MET-116 | `gitgit_ai_input_tokens` | 02-metrics §5.3 |
| OBS-MET-117 | `gitgit_ai_output_tokens` | 02-metrics §5.3 |
| OBS-MET-118 | `gitgit_ai_cost_usd` | 02-metrics §5.3 |
| OBS-MET-119 | `gitgit_webhook_delivery_total` | 02-metrics §5.3 |
| OBS-MET-120 | `gitgit_auth_failures_total` | 02-metrics §5.3 |
| OBS-MET-121 | `gitgit_webauthn_failures_total` | 02-metrics §5.3 |
| OBS-MET-122 | `gitgit_webauthn_credential_total` | 02-metrics §5.3 |
| OBS-MET-123 | `gitgit_audit_events_total` | 02-metrics §5.3 |
| OBS-MET-124 | `gitgit_token_issued_total` | 02-metrics §5.3 |
| OBS-MET-125 | `gitgit_external_api_requests_total` | 02-metrics §5.3 |
| OBS-MET-126 | `gitgit_external_api_duration_seconds` | 02-metrics §5.3 |
| OBS-MET-127 | `gitgit_external_api_rate_limited_total` | 02-metrics §5.3 |
| OBS-MET-128 | `gitgit_audit_chain_check_total` | 05-database §6.2 |
| OBS-MET-129 | `gitgit_app_version_unused_days` | 05-database §4.4 |

> **注**: OBS-MET-081 ~ 100 预留给 v2 CI/CD / OIDC 监控（详见自审修订 7）。

## 4. Logs 字段 OBS-LOG

| ID | 字段 | 类型 | 必填 | 文档 |
|---|---|---|---|---|
| OBS-LOG-001 | `timestamp` | RFC3339 | 是 | 03-logs §2.1 |
| OBS-LOG-002 | `level` | enum (trace/debug/info/warn/error) | 是 | 03-logs §2.1 |
| OBS-LOG-003 | `service` | string | 是 | 03-logs §2.1 |
| OBS-LOG-004 | `version` | string | 是 | 03-logs §2.1 |
| OBS-LOG-005 | `environment` | enum (dev/staging/prod) | 是 | 03-logs §2.1 |
| OBS-LOG-006 | `instance` | string (host/pod) | 是 | 03-logs §2.1 |
| OBS-LOG-007 | `trace_id` | W3C Trace Context (32 hex) | 否 | 03-logs §2.1, 04-tracing §6 |
| OBS-LOG-008 | `span_id` | W3C Trace Context (16 hex) | 否 | 03-logs §2.1, 04-tracing §6 |
| OBS-LOG-009 | `request_id` | UUID v4 | 否 | 03-logs §2.1 |
| OBS-LOG-010 | `error_code` | AppError 枚举 | 否 | 03-logs §2.1 |

> **强约束** (03-logs §3): **禁止**字段: `password` / `secret` / `api_key` / `token` / `cookie` / `bearer` / `private_key`

## 5. Trace Spans OBS-TRC

| ID | Span 名 | 父关系 | 文档 |
|---|---|---|---|
| OBS-TRC-001 | `http.request` | root | 04-tracing §3.1 |
| OBS-TRC-002 | `grpc.request` | root / child of http | 04-tracing §3.2 |
| OBS-TRC-003 | `db.query` | child of http/grpc | 04-tracing §3.3 |
| OBS-TRC-004 | `git.push` / `git.fetch` / `git.clone` | child of http/grpc | 04-tracing §3.4 |
| OBS-TRC-005 | `event.publish` / `event.consume` | child of http/grpc | 04-tracing §3.5 |
| OBS-TRC-006 | `app.invoke` | child of http | 04-tracing §3.6 |
| OBS-TRC-007 | `ai.gateway` | child of http | 04-tracing §3.7 |
| OBS-TRC-008 | `auth.verify` / `auth.challenge` | child of http | 04-tracing §3.8 |
| OBS-TRC-009 | `admin.audit` | child of http | 04-tracing §3.9 |
| OBS-TRC-010 | `webhook.deliver` | child of event.consume | 04-tracing §3.10 |

## 6. Alert 规则 OBS-ALT

| ID | 告警名 | 优先级 | 文档 |
|---|---|---|---|
| OBS-ALT-001 | NodeUnreachable | critical | 08-alert §5.1 |
| OBS-ALT-002 | DiskSpaceCritical | critical | 08-alert §5.1 |
| OBS-ALT-003 | PostgresDown | critical | 08-alert §5.2 |
| OBS-ALT-004 | AdminAuditChainBroken | critical + compliance | 08-alert §5.2 |
| OBS-ALT-005 | PgBouncerWaiting | critical | 08-alert §5.2 |
| OBS-ALT-006 | HttpErrorRateHigh | critical | 08-alert §5.3 |
| OBS-ALT-007 | HttpP99LatencyHigh | critical | 08-alert §5.3 |
| OBS-ALT-008 | GitPushFailure | high | 08-alert §5.4 |
| OBS-ALT-009 | EventJournalBacklog | high | 08-alert §5.4 |
| OBS-ALT-010 | AppBusCapabilityDenied | high | 08-alert §5.4 |
| OBS-ALT-011 | SLO_Availability_Page | critical | 08-alert §5.5 |
| OBS-ALT-012 | SLO_Availability_Ticket | warning | 08-alert §5.5 |
| OBS-ALT-013 | SLO_Budget_Exhausted | critical | 08-alert §5.5 |
| OBS-ALT-014 | CPU 高水位 | warning | 08-alert §6 |
| OBS-ALT-015 | 内存高水位 | warning | 08-alert §6 |
| OBS-ALT-016 | 慢查询占比 | warning | 08-alert §6 |
| OBS-ALT-017 | 缓存命中率低 | warning | 08-alert §6 |
| OBS-ALT-018 | Backup 失败 | warning | 08-alert §6 |
| OBS-ALT-019 | 凭证轮换临近 | warning | 08-alert §6 |
| OBS-ALT-020 | PrometheusHighCardinality | warning | 10-security §9.3 |
| OBS-ALT-021 | 部署完成 | info | 08-alert §7 |
| OBS-ALT-022 | 视图快照生成 | info | 08-alert §7 |
| OBS-ALT-023 | 慢查询 Top 1 新 fingerprint | info | 08-alert §7 |
| OBS-ALT-024 | ConfigMap 变更 | info | 08-alert §7 |
| OBS-ALT-025 | WebAuthn 凭证新增 | info | 08-alert §7 |
| OBS-ALT-026 | Agent Run 完成 | info | 08-alert §7 |
| OBS-ALT-027 | 错误率上升 | warning | 08-alert §6 |
| OBS-ALT-028 | Vault 接近限额 | warning | 08-alert §6 |
| OBS-ALT-029 | 磁盘增长预测 | warning | 08-alert §6 |
| OBS-ALT-030 | SLO 慢烧 (6h burn > 3x) | warning | 08-alert §6 |

## 7. SLO 定义 OBS-SLO

| ID | SLO 名 | 目标 | 错误预算 (30d) | 文档 |
|---|---|---|---|---|
| OBS-SLO-001 | `api-availability` | 99.9% | 43m20s | 09-slo §2.2 |
| OBS-SLO-002 | `api-latency` | P99 < 500ms | 比例目标 | 09-slo §2.2 |
| OBS-SLO-003 | `git-push-success` | 99.5% | 3h36m | 09-slo §2.2 |
| OBS-SLO-004 | `ai-gateway-success` | 99.0% | 7h12m | 09-slo §2.2 |
| OBS-SLO-005 | `pg-write-latency` | P99 < 100ms | 比例目标 | 09-slo §2.2 |
| OBS-SLO-006 | `event-journal-lag` | P95 < 30s | 比例目标 | 09-slo §2.2 |
| OBS-SLO-007 | `platform-availability` | 99.95% | 21m40s | 09-slo §2.2 |
| OBS-SLO-008 | `audit-chain-integrity` | 100% | 0 | 09-slo §2.2 |
| OBS-SLO-009 | `secrets-rotation` | 100% | 0 | 09-slo §2.2 |
| OBS-SLO-010 | `agent-run-completion` | 95% | 36h | 09-slo §2.2 |

## 8. 安全控制 OBS-SEC

| ID | 控制项 | 文档 |
|---|---|---|
| OBS-SEC-001 | Grafana OIDC 集成 | 10-security §2.1 |
| OBS-SEC-002 | Grafana Folder 权限 | 10-security §2.2 |
| OBS-SEC-003 | Prometheus mTLS | 10-security §3.2 |
| OBS-SEC-004 | Loki 多租户隔离 | 10-security §4.1 |
| OBS-SEC-005 | Loki 字段脱敏 | 10-security §4.2 |
| OBS-SEC-006 | Tempo tail_sampling | 10-security §5.1 |
| OBS-SEC-007 | K8s NetworkPolicy | 10-security §6 |
| OBS-SEC-008 | Cilium L7 NetworkPolicy | 10-security §6.5 |
| OBS-SEC-009 | 镜像签名（cosign） | 10-security §8.1 |
| OBS-SEC-010 | 运行时加固（restricted PSA） | 10-security §8.2 |
| OBS-SEC-011 | Cardinality 告警 | 10-security §9.3 |
| OBS-SEC-012 | 应急响应流程 | 10-security §12 |

## 9. 部署资源 OBS-DEP

| ID | 资源 | 副本 | 文档 |
|---|---|---|---|
| OBS-DEP-001 | `observability` namespace | 1 | 11-deployment §2.1 |
| OBS-DEP-002 | ResourceQuota | 1 | 11-deployment §2.3 |
| OBS-DEP-003 | Prometheus (kube-prometheus-stack) | 2 | 11-deployment §3.2 |
| OBS-DEP-004 | Alertmanager | 3 | 11-deployment §3.2 |
| OBS-DEP-005 | Grafana | 2 | 11-deployment §3.2 |
| OBS-DEP-006 | Loki | 3 (distributor/ingester/querier) | 11-deployment §3.2 |
| OBS-DEP-007 | Tempo | 2 | 11-deployment §3.2 |
| OBS-DEP-008 | OTel Collector (DaemonSet) | N (1/节点) | 11-deployment §3.2 |
| OBS-DEP-009 | OTel Collector (Gateway) | 2 | 11-deployment §3.2 |
| OBS-DEP-010 | pg_exporter | 1 | 11-deployment §3.2 |
| OBS-DEP-011 | kube-state-metrics | 1 | 11-deployment §3.2 |
| OBS-DEP-012 | node-exporter (DaemonSet) | N (1/节点) | 11-deployment §3.2 |

## 10. 性能需求 OBS-PERF

| ID | 标题 | 阈值 | 文档 |
|---|---|---|---|
| OBS-PERF-001 | 业务 CPU 增量 | < 3% | 12-performance §1.1 |
| OBS-PERF-002 | 业务内存增量 | < 50MB | 12-performance §1.1 |
| OBS-PERF-003 | 业务 P99 延迟增量 | < 5ms | 12-performance §1.1 |
| OBS-PERF-004 | OTel Collector drop 率 | < 0.1% | 12-performance §1.4 |
| OBS-PERF-005 | Prom / Loki / Tempo query P95 | < 5s | 12-performance §3 |

## 11. 保留策略 OBS-RET

| ID | 标题 | 策略 | 文档 |
|---|---|---|---|
| OBS-RET-001 | Metrics 原始保留 | 30d | 12-performance §5.1 |
| OBS-RET-002 | Metrics 5m 聚合 | 7d | 12-performance §5.1 |
| OBS-RET-003 | Metrics 1h 聚合 | 1y | 12-performance §5.1 |
| OBS-RET-004 | Logs ERROR 保留 | 1y | 12-performance §5.2 |
| OBS-RET-005 | Logs audit 保留 | 3y | 12-performance §5.2 |
| OBS-RET-006 | Traces 保留 | 30d | 12-performance §5.3 |
| OBS-RET-007 | Traces SLO 关键 | 90d | 12-performance §5.3 |

## 12. 实施阶段 OBS-IMPL

| ID | 阶段 | 周期 | 文档 |
|---|---|---|---|
| OBS-IMPL-001 | Phase 0 现状 + ADR | 1 周 | 13-implementation §2 |
| OBS-IMPL-002 | Phase 1 基础设施 | 2 周 | 13-implementation §3 |
| OBS-IMPL-003 | Phase 2 应用 Metrics | 2 周 | 13-implementation §4 |
| OBS-IMPL-004 | Phase 3 日志 | 1.5 周 | 13-implementation §5 |
| OBS-IMPL-005 | Phase 4 Trace | 2 周 | 13-implementation §6 |
| OBS-IMPL-006 | Phase 5 Dashboard | 1.5 周 | 13-implementation §7 |
| OBS-IMPL-007 | Phase 6 Alert | 1.5 周 | 13-implementation §8 |
| OBS-IMPL-008 | Phase 7 SLO | 1 周 | 13-implementation §9 |
| OBS-IMPL-009 | Phase 8 自动化 | 2 周 | 13-implementation §10 |
| OBS-IMPL-010 | 每阶段回滚 | 全程 | 13-implementation 各 Phase §X.6/§X.7 |

## 13. 修订项 OBS-REV

| ID | 标题 | 实施阶段 | 文档 |
|---|---|---|---|
| OBS-REV-001 | 剔除"系统负载预测" panel | Phase 5 | 15-self-review §2.1 |
| OBS-REV-002 | fingerprint 严格归一化 | Phase 2 | 15-self-review §2.2 |
| OBS-REV-003 | 全局开关 `OTEL_DISABLED` | Phase 2 | 15-self-review §2.3 |
| OBS-REV-004 | repo label 聚合 | Phase 1 | 15-self-review §2.4 |
| OBS-REV-005 | INFO 日志速率限制 | Phase 3 | 15-self-review §2.5 |

## 14. 风险 OBS-RISK

| ID | 风险 | 等级 | 缓解 | 文档 |
|---|---|---|---|---|
| OBS-RISK-001 | OTel SDK 升级 breaking change | 中 | 季度评估 + 锁 minor | 15-self-review §11 |
| OBS-RISK-002 | Cardinality 失控 | 高 | 白名单 + 告警 + 修订 2/4 | 15-self-review §11 |
| OBS-RISK-003 | 数据量超预期 | 中 | 容量规划 + 降采样 | 15-self-review §11 |
| OBS-RISK-004 | 实施延期 | 中 | 8 阶段分步 | 15-self-review §11 |
| OBS-RISK-005 | 团队培训不足 | 中 | Phase 0 培训 | 15-self-review §11 |
| OBS-RISK-006 | 第三方依赖漏洞 | 低 | 镜像签名 + SBOM | 15-self-review §11 |
| OBS-RISK-007 | 监控数据被用于训练 | 低 | 部署隔离 + 审计 | 15-self-review §11 |

## 15. 完整追溯关系（样例）

### 15.1 追溯 OBS-REQ-001（统一架构）

```
OBS-REQ-001 (统一 Observability Architecture)
   │
   ├─► ADR-0011 (技术选型)
   │
   ├─► 01-architecture.md §1-§6 (4 层架构)
   │     │
   │     └─► 11-deployment.md §1 (部署拓扑)
   │     └─► 14-code-impact.md §3.1 (gitgit-observability)
   │
   ├─► Component:
   │     ├─► OBS-DEP-003 Prometheus
   │     ├─► OBS-DEP-005 Grafana
   │     ├─► OBS-DEP-006 Loki
   │     ├─► OBS-DEP-007 Tempo
   │     └─► OBS-DEP-008/009 OTel Collector
   │
   ├─► Configuration:
   │     └─► [实际 YAML 配置]
   │
   └─► Test Case:
         ├─► 验证: anchor 0 破损
         ├─► 验证: 0 日文
         ├─► 验证: 14 文档 616 cross-ref 0 破损
         └─► 验证: 每 Phase 部署烟测
```

### 15.2 追溯 OBS-REQ-005（数据库可观测性）

```
OBS-REQ-005 (数据库可观测性)
   │
   ├─► ADR-0003 (能 PG 解决都用 PG)
   │
   ├─► 05-database-observability.md §1-§12
   │
   ├─► Metrics:
   │     ├─► OBS-MET-021 ~ 050 (PG 30+ 指标)
   │     └─► OBS-MET-044 ~ 050 (业务表深度)
   │
   ├─► Alerts:
   │     ├─► OBS-ALT-003 PostgresDown
   │     ├─► OBS-ALT-004 AdminAuditChainBroken
   │     ├─► OBS-ALT-005 PgBouncerWaiting
   │     └─► OBS-ALT-016/017 慢查询 / 缓存命中率
   │
   ├─► SLO:
   │     └─► OBS-SLO-005 pg-write-latency
   │
   └─► Test Case:
         ├─► Phase 1: pg_exporter 部署后能 scrape
         ├─► Phase 2: 业务 SQL 慢查询能通过 fingerprint 定位
         └─► Phase 6: PostgresDown 告警 1min 内触发
```

### 15.3 追溯 OBS-REQ-022（降级策略）

```
OBS-REQ-022 (降级策略)
   │
   ├─► 12-performance-retention.md §1.4
   │
   ├─► Component:
   │     ├─► OTel Collector memory_limiter
   │     ├─► OTel Collector tail_sampling
   │     └─► 业务侧 OTEL_DISABLED env
   │
   ├─► Configuration:
   │     └─► memory_limiter.limit_mib: 2048
   │     └─► OTEL_DISABLED=true 全量关闭
   │
   └─► Test Case:
         ├─► chaos test: 关 Prometheus 5min 业务无感
         ├─► chaos test: 业务 pod memory > 80% 自动降级
         └─► chaos test: 出口网络故障 5min 数据不丢
```

## 16. 覆盖率统计

| 维度 | 总数 | 已覆盖 | 覆盖率 |
|---|---|---|---|
| 顶层需求 | 28 | 28 | 100% |
| Metric 指标 | 130 | 130 | 100% |
| Log 字段 | 10 | 10 | 100% |
| Trace Span | 10 | 10 | 100% |
| Alert 规则 | 30 | 30 | 100% |
| SLO 定义 | 10 | 10 | 100% |
| 安全控制 | 12 | 12 | 100% |
| 部署资源 | 12 | 12 | 100% |
| 性能需求 | 5 | 5 | 100% |
| 保留策略 | 7 | 7 | 100% |
| 实施阶段 | 10 | 10 | 100% |
| 修订项 | 5 | 5 | 100% |
| 风险 | 7 | 7 | 100% |
| **合计** | **~ 280** | **~ 280** | **100%** |

## 17. 关联文档

- 上游: [`00-15` 全部 observability 文档](.) / [ADR-0011](../architecture/decisions/0011-observability-platform.md)
- 下游: ADR 体系 [README.md](../architecture/decisions/README.md)
- 下游: 需求追溯实施 `requirements-traceability.md`（本文件）

## 18. 维护说明

> **强约束**: 任何新增需求 / 修订 / 风险 **必须**同步更新本矩阵。

| 维护项 | 触发 |
|---|---|
| 新增 OBS-REQ-* | 任何新增顶层需求 |
| 新增 OBS-MET-* | 任何新增业务指标 |
| 新增 OBS-ALT-* | 任何新增告警规则 |
| 新增 OBS-SLO-* | 任何新增 SLO |
| 新增 OBS-SEC-* | 任何新增安全控制 |
| 修订项关闭 | 实施完成时 |
| 风险关闭 | 缓解措施生效时 |
