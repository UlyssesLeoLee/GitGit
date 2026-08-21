# 数据库可观测性 / Database Observability（PostgreSQL 深度）

> **关联 OBS-REQ**: OBS-REQ-005（数据库可观测性） / OBS-REQ-006（中间件可观测性）
> **关联设计**: [`../design/detailed-design/01-data-layer.md`](../design/detailed-design/01-data-layer.md) / [`../design/detailed-design/09-security-impl.md`](../design/detailed-design/09-security-impl.md)
> **关联 ADR**: ADR-0003（能 PG 解决都用 PG） / ADR-0009（多租户 RLS）
> **目标**: 5 原语 / 中心事件 / 审计 / 信封加密 四类核心表的运行健康 + SQL 性能根因可定位

## 1. 监控范围

### 1.1 数据库清单

| 实例 | 角色 | 版本 | 容量基线 | 备注 |
|---|---|---|---|---|
| `pg-primary` | 主库（读写） | PostgreSQL 15+ | [TBD] – Benchmark Required | 唯一持久化（5 原语 + 中心事件 + 审计 + 凭证） |
| `pg-replica-1` | 同步流复制 | PostgreSQL 15+ | [TBD] – Benchmark Required | 只读副本（Graph / Policy / AI 走只读路径） |
| `pg-replica-2` | 异步报告副本 | PostgreSQL 15+ | [TBD] – Benchmark Required | 报表 / 数据导出（不服务在线请求） |
| `pgbouncer` | 连接池（Transaction 模式） | pgbouncer 1.22+ | 10000 client / 200 server | 业务服务不直连 PG，统一走 pgbouncer |

> **强约束**: 不引入 Redis / Memgraph / NATS / Valkey（ADR-0003）。所有数据库可观测性都围绕 PG 展开。

### 1.2 Schema 划分（来自 §01-data-layer）

| Schema | 用途 | 关键表 | 量级预期 |
|---|---|---|---|
| `tenant` | 5 原语领域表 | `node` / `edge` / `view` / `type_registry` / `permission` | 千万行级 |
| `core_auth` | 用户 / 会话 / WebAuthn | `user_account` / `session` / `webauthn_credential` | 百万级 |
| `core_secret` | 信封加密 Secrets | `secret_envelope` / `secret_version` | 十万级 |
| `coordination` | 中心事件 / Outbox | `event_journal` / `outbox` / `idempotency_key` | 亿行级（按月分区） |
| `app_registry` | App 元数据 / 版本 | `app` / `app_version` / `app_grant` | 千级 |
| `git_repo` | Git 仓库元数据 | `git_repository` / `git_lfs_object` | 万级 |
| `agent_runtime` | Agent Run / 状态 | `agent_run` / `agent_event` | 百万级 |
| `view_snapshot` | 视图快照 | `view_snapshot` | 十万级 |
| `audit` | 不可篡改审计 | `admin_audit`（哈希链） | 亿行级（按年分区） |

## 2. 采集架构

### 2.1 采集方式

```
┌─────────────────────┐
│  Application (Rust)  │ ─── sqlx 每次 query 自带 span ───┐
└─────────────────────┘                                     │
                                                             │  OTLP/gRPC
┌─────────────────────┐                                     │
│  pg_exporter         │ ─── scrape :9187 ──►               ▼
│  (Deployment)        │              ┌─────────────────────────────┐
└─────────────────────┘              │  OTel Collector              │
                                     │  ─────────────────────────   │
                                     │  Receiver: prometheus        │
                                     │  Processor: relabel + filter │
                                     │  Exporter:  remote_write ──► Prometheus
                                     └─────────────────────────────┘
```

**关键点**:
- `pg_exporter` 部署为 `Deployment`（非 DaemonSet），1 副本对每 PG 实例 1 个
- 自定义 SQL 指标（5 原语 / 中心事件）通过 `pg_exporter` 的 `queries.yaml` 扩展
- 业务 SQL 延迟 / 错误通过 `sqlx` 自带 `tracing` + `sqlx-stmt` 标签上报

### 2.2 必装扩展

| 扩展 | 用途 | 必需 |
|---|---|---|
| `pg_stat_statements` | 慢查询 / Top-N | **是** |
| `auto_explain` | 自动 EXPLAIN 慢查询 | **是**（>500ms 触发） |
| `pgstattuple` | 表 / 索引膨胀 | 是 |
| `pg_buffercache` | shared_buffers 命中 | 可选 |
| `wal2json` | admin_audit 不可篡改输出（ADR-0008） | **是** |
| `pg_cron` | 维护任务（vacuum / analyze） | 可选（外部 CronJob 替代） |

> **SUPERUSER 要求**: `pg_stat_statements` / `pgstattuple` 需要 `shared_preload_libraries`，集群初始化时一次性开启（见 `migrations/0014-seed`）。

## 3. PG 核心指标（PG-LEVEL）

### 3.1 连接与活动（PG-REQ-001）

| Metric 名 | 类型 | 来源 | 阈值 |
|---|---|---|---|
| `pg_stat_activity_count{state,datname}` | Gauge | `pg_stat_activity` | active > 100 持续 5m 告警 |
| `pg_stat_activity_max_tx_duration_seconds{datname}` | Gauge | `pg_stat_activity` | > 300s 告警（长事务） |
| `pg_stat_activity_idle_in_tx_count{datname}` | Gauge | `pg_stat_activity` | > 5 持续 10m 告警 |
| `pgbouncer_pools_client_active{db}` | Gauge | pgbouncer `SHOW POOLS` | 连接池使用率 > 80% |
| `pgbouncer_pools_server_active{db}` | Gauge | pgbouncer `SHOW POOLS` | > 100 持续 5m 告警 |
| `pgbouncer_pools_waiting{db}` | Gauge | pgbouncer `SHOW POOLS` | > 10 持续 1m **Critical**（请求阻塞） |

### 3.2 查询性能（PG-REQ-002）

| Metric 名 | 类型 | 来源 | 阈值 |
|---|---|---|---|
| `pg_stat_statements_calls_total{user,datname,fingerprint}` | Counter | `pg_stat_statements` | 趋势监控 |
| `pg_stat_statements_total_exec_time_seconds_total` | Counter | `pg_stat_statements` | 速率（QPS × 平均时间） |
| `pg_stat_statements_mean_exec_time_seconds` | Gauge | `pg_stat_statements` | 单条平均 > 200ms 关注 |
| `pg_stat_statements_rows_total` | Counter | `pg_stat_statements` | 异常突增（注入 / 拉表） |
| `pg_slow_queries_total{query_class}` | Counter | `auto_explain` 阈值 500ms | > 100/min 告警 |
| `pg_lock_waits_total{relation}` | Counter | `pg_stat_activity` + `pg_locks` | > 0 持续 30s 告警 |

> **Cardinality 警告**: `user` / `datname` / `fingerprint` 三个 label 至少两个组合。**强制**: fingerprint 必须归一化（参数化），否则 cardinality 爆炸。

### 3.3 事务与锁（PG-REQ-003）

| Metric 名 | 类型 | 来源 | 阈值 |
|---|---|---|---|
| `pg_stat_database_xact_commit{datname}` | Counter | `pg_stat_database` | 趋势 |
| `pg_stat_database_xact_rollback{datname}` | Counter | `pg_stat_database` | 失败率 > 1% 告警 |
| `pg_stat_database_deadlocks{datname}` | Counter | `pg_stat_database` | > 0 告警 |
| `pg_stat_database_conflicts{datname}` | Counter | `pg_stat_database` | 副本冲突 > 10/s 告警 |
| `pg_locks_count{mode,locktype,datname}` | Gauge | `pg_locks` | 持续 > 1000 关注 |
| `pg_stat_activity_waiting_count{wait_event_type,datname}` | Gauge | `pg_stat_activity` | LockWait > 10 持续 1m 告警 |

**关键 wait_event_type**:
- `Lock` → 锁等待
- `IO` → 磁盘 I/O
- `LWLock` → 内部轻量锁
- `Extension` → 扩展等待（wal2json 等）
- `Client` → 客户端网络
- `IPC` → 进程间通信

### 3.4 缓存与 I/O（PG-REQ-004）

| Metric 名 | 类型 | 来源 | 阈值 |
|---|---|---|---|
| `pg_stat_database_blks_hit{datname}` | Counter | `pg_stat_database` | — |
| `pg_stat_database_blks_read{datname}` | Counter | `pg_stat_database` | — |
| `pg_cache_hit_ratio{datname}` | Gauge | 计算 `hit / (hit+read)` | < 95% 关注 / < 90% 告警 |
| `pg_stat_io_operations_total{datname,backend_type,object,context}` | Counter | `pg_stat_io` (PG 16+) | 趋势 |
| `pg_wal_lsn_flush_lag_bytes` | Gauge | 复制监控 | 副本延迟 > 100MB 告警 |
| `pg_replication_lag_seconds{upstream}` | Gauge | `pg_stat_replication` | > 30s 告警 |

### 3.5 复制与高可用（PG-REQ-005）

| Metric 名 | 类型 | 来源 | 阈值 |
|---|---|---|---|
| `pg_replication_state{client_addr,state}` | Gauge(0/1) | `pg_stat_replication` | streaming ≠ 1 告警 |
| `pg_replication_wal_lsn_diff_bytes{client_addr}` | Gauge | `pg_stat_replication` | > 80MB 告警 |
| `pg_replication_flush_lag_seconds` | Gauge | `pg_stat_replication` | > 10s 告警 |
| `pg_replication_replay_lag_seconds` | Gauge | `pg_stat_replication` | > 60s 告警 |
| `pg_failover_promote_total` | Counter | 自定义心跳表 | 突增 Critical |

### 3.6 存储 / Vacuum / Checkpoint（PG-REQ-006）

| Metric 名 | 类型 | 来源 | 阈值 |
|---|---|---|---|
| `pg_disk_usage_bytes{path}` | Gauge | `pg_database_size` + `df` | > 80% 告警 / > 90% Critical |
| `pg_table_bloat_ratio{table,schema}` | Gauge | `pgstattuple` | > 30% 告警 |
| `pg_index_bloat_ratio{index,schema}` | Gauge | `pgstattuple` | > 50% 告警 |
| `pg_stat_user_tables_n_dead_tup{table,schema}` | Gauge | `pg_stat_user_tables` | 长期 > 100 万触发 vacuum 告警 |
| `pg_stat_user_tables_last_vacuum{table,schema}` | Gauge(timestamp) | `pg_stat_user_tables` | 24h 内未 vacuum 告警 |
| `pg_stat_bgwriter_checkpoints_timed_total` | Counter | `pg_stat_bgwriter` | 趋势（checkpoint 频率） |
| `pg_stat_bgwriter_checkpoints_req_total` | Counter | `pg_stat_bgwriter` | requested / total > 0.3 告警（max_wal_size 偏小） |

> **表膨胀**: 重点监控 `coordination.event_journal`（按月分区）、`audit.admin_audit`（按年分区）、`tenant.node`（核心 5 原语）。

## 4. 业务深度指标（APP-LEVEL）

> **目标**: 不只监控 PG 自身，还要看**业务表**的健康度。这才能在出问题时知道"哪个业务功能异常"。

### 4.1 5 原语（DOM-REQ-001）

| Metric 名 | 来源 SQL | 阈值 |
|---|---|---|
| `gitgit_node_total{type,tenant}` | `SELECT type, count(*) FROM tenant.node GROUP BY 1` | 趋势 |
| `gitgit_edge_total{kind,tenant}` | `SELECT kind, count(*) FROM tenant.edge GROUP BY 1` | 趋势 |
| `gitgit_node_orphans_total{tenant}` | 边引用检查 | > 0 告警 |
| `gitgit_view_stale_count{tenant}` | 视图版本与最新数据差 | > 100 告警 |
| `gitgit_audit_chain_broken{tenant}` | admin_audit 哈希链验证 | = 1 立即 **Critical** |

### 4.2 中心事件总线（CORE-EVT-001）

| Metric 名 | 来源 | 阈值 |
|---|---|---|
| `gitgit_event_journal_lag_seconds{topic}` | 最新事件 ts - now | > 30s 告警（消费延迟） |
| `gitgit_event_journal_unpublished{topic}` | `outbox WHERE status='pending'` | > 1000 告警 |
| `gitgit_event_journal_consumer_lag{consumer,topic}` | 消费 offset 差 | > 1000 告警 |
| `gitgit_idempotency_key_collision_total` | 应用层自报 | > 0 告警（业务 bug） |

### 4.3 凭证与信封（SEC-REQ-007）

| Metric 名 | 来源 | 阈值 |
|---|---|---|
| `gitgit_secret_envelope_total{tenant}` | `core_secret.secret_envelope` | 趋势 |
| `gitgit_secret_kek_rotation_due{tenant}` | `expires_at` 临近 | < 30d 告警 |
| `gitgit_secret_dek_reencrypt_pending{tenant}` | KEK 轮换后未重加密 | > 0 告警 |
| `gitgit_webauthn_credential_total{tenant}` | `core_auth.webauthn_credential` | 趋势 |

### 4.4 App Registry（APP-REQ-008）

| Metric 名 | 来源 | 阈值 |
|---|---|---|
| `gitgit_app_total{status}` | `app_registry.app` | 趋势 |
| `gitgit_app_grant_active{app,scope}` | `app_grant WHERE status='active'` | 趋势 |
| `gitgit_app_version_unused_days{app,version}` | `last_used_at` 距今 | > 90d 告警（清理） |

## 5. SQL 慢查询根因定位

### 5.1 自动 EXPLAIN 策略

`postgresql.conf`:

```ini
shared_preload_libraries = 'pg_stat_statements,auto_explain,pgstattuple'
auto_explain.log_min_duration = '500ms'
auto_explain.log_analyze = on
auto_explain.log_buffers = on
auto_explain.log_format = 'json'
auto_explain.sample_rate = 0.01   # 1% 采样，避免日志爆炸
```

`pg_stat_statements`:

```ini
pg_stat_statements.max = 10000
pg_stat_statements.track = top
pg_stat_statements.track_utility = off
```

### 5.2 慢查询 Top-N 查询（PG-REQ-007）

```sql
-- Top 20 by total time
SELECT
    substring(query for 200) AS query_fingerprint,
    calls,
    round(total_exec_time::numeric, 2) AS total_ms,
    round(mean_exec_time::numeric, 2) AS mean_ms,
    round((100 * total_exec_time / sum(total_exec_time) OVER ())::numeric, 2) AS pct_total
FROM pg_stat_statements
WHERE user != 'pg_exporter'
ORDER BY total_exec_time DESC
LIMIT 20;
```

### 5.3 锁等待查询（PG-REQ-008）

```sql
SELECT
    blocked_locks.pid     AS blocked_pid,
    blocked_activity.usename  AS blocked_user,
    blocking_locks.pid     AS blocking_pid,
    blocking_activity.usename AS blocking_user,
    blocked_activity.query    AS blocked_statement,
    blocking_activity.query   AS current_statement_in_blocking_process
FROM pg_catalog.pg_locks blocked_locks
JOIN pg_catalog.pg_stat_activity blocked_activity
  ON blocked_activity.pid = blocked_locks.pid
JOIN pg_catalog.pg_locks blocking_locks
  ON blocking_locks.locktype = blocked_locks.locktype
  AND blocking_locks.database IS NOT DISTINCT FROM blocked_locks.database
  AND blocking_locks.relation IS NOT DISTINCT FROM blocked_locks.relation
  AND blocking_locks.page IS NOT DISTINCT FROM blocked_locks.page
  AND blocking_locks.tuple IS NOT DISTINCT FROM blocked_locks.tuple
  AND blocking_locks.virtualxid IS NOT DISTINCT FROM blocked_locks.virtualxid
  AND blocking_locks.transactionid IS NOT DISTINCT FROM blocked_locks.transactionid
  AND blocking_locks.pid != blocked_locks.pid
JOIN pg_catalog.pg_stat_activity blocking_activity
  ON blocking_activity.pid = blocking_locks.pid
WHERE NOT blocked_locks.granted;
```

## 6. 备份与 PITR

### 6.1 备份健康（PG-REQ-009）

| Metric 名 | 来源 | 阈值 |
|---|---|---|
| `pg_backup_last_success_timestamp{type}` | 备份脚本上报 | > 26h 告警（每日全量） |
| `pg_backup_size_bytes{type}` | 备份脚本 | 趋势（异常膨胀 = 数据问题） |
| `pg_wal_archive_lag_seconds` | `pg_stat_archiver` | > 60s 告警 |
| `pg_wal_archive_failed_count_total` | `pg_stat_archiver` | > 0 告警 |

### 6.2 不可篡改审计（ADR-0008 / AUD-REQ-001）

| Metric 名 | 来源 | 阈值 |
|---|---|---|
| `gitgit_audit_hash_chain_ok` | 校验任务 | = 0 立即 **Critical**（合规事件） |
| `gitgit_audit_chain_repair_count` | 修复任务 | > 0 立即通知（不告警，但记录） |
| `gitgit_audit_wal_lag_seconds` | wal2json 输出 | > 30s 告警 |
| `gitgit_audit_unprocessed_bytes` | `pending` 大小 | > 100MB 告警 |

## 7. 告警规则样例（DB 域）

### 7.1 P1: 连接池耗尽

```yaml
- alert: PgBouncerWaitingClients
  expr: max by (db) (pgbouncer_pools_waiting) > 10
  for: 1m
  labels:
    severity: critical
    domain: database
  annotations:
    summary: "pgbouncer 池等待连接堆积（{{ $value }}）"
    impact: "所有新请求阻塞，业务雪崩"
    runbook: |
      1. 检查 SHOW POOLS; 各 db 的 sv_active / sv_idle
      2. 检查 SHOW CLIENTS; 长连接占用
      3. 检查 max_connections 与 pool_size 配置
      4. 若 PG 端慢 → 同步告警会拉过来
```

### 7.2 P1: 哈希链断裂

```yaml
- alert: AdminAuditChainBroken
  expr: gitgit_audit_chain_broken == 1
  labels:
    severity: critical
    domain: security
    compliance: true
  annotations:
    summary: "admin_audit 哈希链断裂"
    impact: "审计不可信，合规事件，需立即人工处置"
    runbook: |
      1. 立即锁定所有 admin 写操作
      2. 检查 wal2json 状态
      3. 启动 forensic 模式
      4. 联系合规负责人
```

### 7.3 P2: 缓存命中率下降

```yaml
- alert: PgCacheHitRatioLow
  expr: avg by (datname) (pg_cache_hit_ratio) < 0.90
  for: 10m
  labels:
    severity: warning
    domain: database
  annotations:
    summary: "{{ $labels.datname }} 缓存命中率 {{ $value | humanizePercentage }}"
    impact: "查询延迟上升，磁盘 I/O 增加"
    runbook: |
      1. shared_buffers 是否偏小（PG 自动调优建议 25% RAM）
      2. 是否大表扫描（检查 pg_stat_user_tables.seq_scan）
      3. 是否需要扩展工作集
```

### 7.4 P2: 长事务

```yaml
- alert: PgLongTransaction
  expr: max by (datname) (pg_stat_activity_max_tx_duration_seconds) > 300
  for: 2m
  labels:
    severity: warning
    domain: database
  annotations:
    summary: "{{ $labels.datname }} 存在 {{ $value }}s 长事务"
    impact: "vacuum 无法回收死元组，表膨胀"
    runbook: |
      1. SELECT pid, query, state FROM pg_stat_activity WHERE xact_start < now() - interval '5 min';
      2. 评估是否能 kill：生产环境先通知 owner
      3. 检查应用层事务边界是否合理
```

## 8. Cardinality 控制（DB 域强约束）

| Label | 基数预估 | 约束 |
|---|---|---|
| `datname` | ≤ 5 | 硬约束（不超过 schema 数量） |
| `user` | ≤ 30 | 业务用户 + 服务账号 + 监控账号 |
| `fingerprint` | ≤ 5000 | **强制归一化**（参数化） |
| `query_class` | ≤ 50 | 业务自定义（5 原语 / 中心事件 / 审计） |
| `tenant` | ≤ 100 | 多租户 ID（如启用） |

**强制规则**:
1. `pg_stat_statements` 输出必须经过 `pg_exporter` 的 `queries.yaml` 二次归一化
2. 业务 SQL 慢查询上报必须把参数替换为 `$1` / `$2` 占位符
3. 禁止把 `query` 完整文本作为 label

## 9. 性能开销自评（DB 域）

| 组件 | CPU 开销 | 内存开销 | 磁盘开销 | 备注 |
|---|---|---|---|---|
| `pg_exporter` | < 0.5 核 | ~ 50 MB | — | 每 15s scrape 一次 |
| `pg_stat_statements` | < 1% | 共享缓冲 ~ 1% RAM | — | 10k 条 |
| `auto_explain` | 仅慢查询 | — | 日志 1% × 业务查询 | sampling 0.01 |
| `pgstattuple` | 仅扩展扫描 | — | — | 每日一次离线任务 |
| `wal2json` | < 2% | 共享缓冲 | — | 持续输出 |

> **结论**: PG 域 telemetry 自开销 < 3% 业务性能，**允许**默认开启。

## 10. 不监控的项（明确剔除）

- ❌ 单条 SQL 文本（仅 fingerprint）
- ❌ 应用层业务数据（用户表 / 凭证内容）
- ❌ 完整 DDL 历史（pg_stat_statements 已涵盖 schema 变化）
- ❌ 单条连接的 client_addr 全量（仅保留前 24 bit 用于分片）
- ❌ 表 / 索引的物理位置（ctid）

## 11. 关联文档

- 上游: [`01-architecture.md`](01-architecture.md) §3 采集层
- 上游: [`02-metrics.md`](02-metrics.md) §4 命名规约
- 上游: [`03-logs.md`](03-logs.md) §4 PG 慢查询日志关联
- 上游: [`04-tracing.md`](04-tracing.md) §5 sqlx 链路
- 下游: [`07-dashboard-design.md`](07-dashboard-design.md) §4 40-Database
- 下游: [`08-alert-design.md`](08-alert-design.md) §3 DB 域告警
- 下游: [`09-slo-design.md`](09-slo-design.md) §4 PG SLI
- 下游: [`15-self-review-v2.md`](15-self-review-v2.md) §2 修订项

## 12. 需求 ID 索引

| 需求 ID | 标题 | 优先级 |
|---|---|---|
| OBS-REQ-005 | 数据库可观测性 | P0 |
| OBS-REQ-006 | 中间件可观测性 | P0 |
| PG-REQ-001 | 连接与活动 | P0 |
| PG-REQ-002 | 查询性能 | P0 |
| PG-REQ-003 | 事务与锁 | P0 |
| PG-REQ-004 | 缓存与 I/O | P1 |
| PG-REQ-005 | 复制与高可用 | P0 |
| PG-REQ-006 | 存储 / Vacuum | P1 |
| PG-REQ-007 | 慢查询 Top-N | P0 |
| PG-REQ-008 | 锁等待查询 | P0 |
| PG-REQ-009 | 备份与 PITR | P0 |
| DOM-REQ-001 | 5 原语业务深度 | P0 |
| CORE-EVT-001 | 中心事件深度 | P0 |
| SEC-REQ-007 | 凭证信封深度 | P0 |
| APP-REQ-008 | App Registry 深度 | P1 |
| AUD-REQ-001 | 不可篡改审计 | P0 |
