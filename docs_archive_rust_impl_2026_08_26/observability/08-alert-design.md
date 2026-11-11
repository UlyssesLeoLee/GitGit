# Alert 设计 / Alert Design

> **关联 OBS-REQ**: OBS-REQ-009（Alert 设计） / OBS-REQ-010（SLO Burn Rate）
> **关联设计**: [`09-slo-design.md`](09-slo-design.md)（SLO/SLI） / [`07-dashboard-design.md`](07-dashboard-design.md)（panel 链接） / [`02-metrics.md`](02-metrics.md)
> **目标**: **禁止**单指标阈值直报；**强制**多窗口 + 持续时间 + SLO Burn Rate；每条告警必须**可操作**
> **核心原则**: 一条告警 = 一个 on-call 工程师**知道下一步做什么**

## 1. 设计红线

### 1.1 禁止 / 强制

| ❌ 禁止 | ✅ 强制 |
|---|---|
| `CPU > 80%` 直接告警 | 持续 5m + 同比 / 环比 + 影响 |
| 单窗口 | 多窗口（`for: 5m` + 严重性升级） |
| 单指标 | 多指标关联（症状 + 影响 + 根因） |
| 无 runbook | runbook 必带（至少 4 步） |
| 无 owner | owner team + Slack 频道 |
| 无影响描述 | "影响 X 用户数 / Y 业务功能" |
| 发送后沉默 | 自动 resolution 通知 + 静默期合并 |

### 1.2 告警质量检查（自评清单）

每条告警上线前**必须**通过 6 项检查:

| # | 检查项 |
|---|---|
| 1 | **可路由**: 告警带 `severity` + `domain` + `team` 标签 |
| 2 | **可触发**: PromQL 在 1m 内必有结果（不会"无目标"） |
| 3 | **可抑制**: 与更高 / 更低优先级告警不重复 |
| 4 | **可操作**: runbook 至少 4 步，第一步 ≤ 1 min 完成 |
| 5 | **可观察**: 5m 内可以验证是否恢复（自动 resolve） |
| 6 | **可复盘**: 含足够 context（不是 "system is down"） |

## 2. 告警等级

### 2.1 四级

| 等级 | 含义 | 响应时间 | 通道 | 抑制 |
|---|---|---|---|---|
| `critical` | 用户已可见损害，**立即响应** | 5 min | 短信 + 电话 + Slack #oncall | 否（最高） |
| `high` | 30min 内可能升级为 critical | 30 min | 电话 + Slack | 是 |
| `warning` | 趋势恶化，1h 内关注 | 1 h | Slack #alerts | 是 |
| `info` | 通知 / 状态变更 | 1 d | Slack #ops-info | 是 |

### 2.2 抑制矩阵

```
critical  >  high     (suppress)
high      >  warning  (suppress)
warning   >  info     (suppress)
同 team 同 component  >  同 team 跨 component  (suppress)
新告警    >  已存在同症状  (suppress via fingerprint)
```

> **举例**: `NodeUnreachable` (critical) 触发时，自动抑制该 node 上所有 Pod 级告警。

## 3. 告警域分类

| Domain | 含义 | 主管团队 | 路由 |
|---|---|---|---|
| `infrastructure` | Node / OS / 网络 | SRE | `sre-primary` |
| `kubernetes` | K3s / Pod / NetworkPolicy | SRE | `sre-k8s` |
| `database` | PG / pgbouncer | SRE / DBA | `sre-db` |
| `application` | 业务服务 / API | Backend Dev | `backend-{crate}` |
| `middleware` | Git / 事件 / App Bus | SRE / Backend Dev | `sre-mw` |
| `security` | 审计 / 凭证 / 越权 | Security | `sec-oncall` |
| `slo` | SLO 错误预算燃烧 | SRE | `sre-slo` |
| `compliance` | 合规事件（哈希链等） | Security / 合规 | `sec-compliance` |

## 4. 告警结构

### 4.1 标准告警格式（YAML）

```yaml
- alert: <AlertName>
  expr: <PromQL>
  for: <duration>
  labels:
    severity: <critical|high|warning|info>
    domain: <domain>
    team: <team>
    slo: <关联 SLO 名称, 可选>
    compliance: <true|false, 默认 false>
  annotations:
    summary: "<= 80 字符的一句话>"
    description: |
      <多行详细：现象 / 影响 / 时间>
    impact: |
      <影响描述：多少用户 / 哪些功能 / 多久>
    runbook: |
      <编号步骤，每步可执行>
    dashboard: <dashboard URL>
    logs_query: <LogQL 链接>
    trace_query: <Trace 查询链接>
    silencer: <静默条件, 如 specific_pod>
```

### 4.2 PromQL 模式

#### 模式 A: 持续时间 + 多窗口

```promql
# 5m 内 + 30m 内 双重确认
(
  avg_over_time(cpu_usage[5m]) > 0.85
  and
  avg_over_time(cpu_usage[30m]) > 0.85
)
```

#### 模式 B: 同比 / 环比

```promql
# 当前 vs 昨日同时段
sum(rate(http_requests_total[5m]))
  /
sum(rate(http_requests_total[5m] offset 1d)) < 0.5
```

#### 模式 C: 异常突增 / 突降

```promql
# 突增 3 倍中位数
sum(rate(http_5xx_total[5m]))
  >
3 * quantile_over_time(0.5, sum(rate(http_5xx_total[5m]))[1h:5m])
```

#### 模式 D: SLO Burn Rate

```promql
# 1h 短期 + 6h 长期双窗口（Google SRE Workbook）
(
  slo:sli_error:ratio_rate1h{slo="api-availability"} > (14.4 * 0.001)
  and
  slo:sli_error:ratio_rate6h{slo="api-availability"} > (6 * 0.001)
)
```

## 5. Critical 告警清单（必发）

### 5.1 平台基础（CRT-INFRA）

#### INFRA-01: Node Unreachable
```yaml
- alert: NodeUnreachable
  expr: up{job="node-exporter"} == 0
  for: 2m
  labels:
    severity: critical
    domain: infrastructure
    team: sre
  annotations:
    summary: "节点 {{ $labels.instance }} 失联"
    impact: |
      该节点上所有 Pod 无法被调度 / 健康检查
    runbook: |
      1. SSH 节点: ssh {{ $labels.instance }}
      2. 检查 `uptime` / `dmesg | tail -50`
      3. 检查 K3s agent: `systemctl status k3s-agent`
      4. 检查磁盘 / 内存: `df -h && free -h`
      5. 如需排空: `kubectl drain {{ $labels.instance }} --ignore-daemonsets`
```

#### INFRA-02: 磁盘满
```yaml
- alert: DiskSpaceCritical
  expr: |
    (1 - (node_filesystem_avail_bytes{fstype!~"tmpfs|overlay",mountpoint!~"/proc.*|/sys.*"} 
          / node_filesystem_size_bytes{fstype!~"tmpfs|overlay",mountpoint!~"/proc.*|/sys.*"})) * 100
    > 95
  for: 5m
  labels:
    severity: critical
    domain: infrastructure
    team: sre
  annotations:
    summary: "节点 {{ $labels.instance }} 磁盘 {{ $labels.mountpoint }} 使用 {{ printf \"%.1f\" $value }}%"
    impact: |
      K3s 可能驱逐 Pod，PG 可能 crash
    runbook: |
      1. 立即清理: `kubectl logs --tail=1000 ...` 容器日志
      2. 清理 docker: `docker system prune -a`
      3. 清理 /var/log: `journalctl --vacuum-size=100M`
      4. 扩容: 申请新 EBS 卷
```

### 5.2 数据库（CRT-DB）

#### DB-01: PG Primary Down
```yaml
- alert: PostgresDown
  expr: pg_up{instance="pg-primary"} == 0
  for: 1m
  labels:
    severity: critical
    domain: database
    team: sre-db
  annotations:
    summary: "PostgreSQL 主库 down"
    impact: |
      所有写操作失败，业务雪崩
    runbook: |
      1. 检查 PG 进程: `kubectl exec -it pg-primary-0 -- ps aux | grep postgres`
      2. 查看 PG 日志: `kubectl logs pg-primary-0 --tail=200`
      3. 检查 etcd / K3s 控制面
      4. 触发 failover: `pg-promote-replica.sh`
      5. 通知用户
```

#### DB-02: admin_audit 哈希链断裂
```yaml
- alert: AdminAuditChainBroken
  expr: gitgit_audit_hash_chain_broken == 1
  labels:
    severity: critical
    domain: security
    team: sec-oncall
    compliance: true
  annotations:
    summary: "admin_audit 哈希链断裂（合规事件）"
    impact: |
      审计不可信，可能存在篡改；合规报告需重做
    runbook: |
      1. 立即锁定所有 admin 写: `gitgit-admin lock --reason=audit-broken`
      2. 检查 wal2json 状态: `kubectl logs wal2json-*`
      3. 启动 forensic 模式: 仅读 + 留证
      4. 联系合规负责人 + 法务
      5. 全量审计 24h 内所有 admin 操作
```

#### DB-03: pgbouncer 等待堆积
```yaml
- alert: PgBouncerWaiting
  expr: max by (db) (pgbouncer_pools_waiting) > 10
  for: 1m
  labels:
    severity: critical
    domain: database
    team: sre-db
  annotations:
    summary: "pgbouncer {{ $labels.db }} 等待连接 {{ $value }}"
    impact: |
      所有新请求阻塞，业务雪崩
    runbook: |
      1. `kubectl exec -it pgbouncer -- psql -c "SHOW POOLS;"`
      2. 检查 `sv_active` / `sv_idle` / `sv_used`
      3. `SHOW CLIENTS;` 看长连接
      4. 临时扩容: 增加 `pool_size`
      5. 根因: 是否 PG 端慢查询 / 锁
```

### 5.3 应用（CRT-APP）

#### APP-01: 错误率
```yaml
- alert: HttpErrorRateHigh
  expr: |
    (
      sum by (service) (rate(http_requests_total{status=~"5.."}[5m]))
      /
      sum by (service) (rate(http_requests_total[5m]))
    ) > 0.05
  for: 5m
  labels:
    severity: critical
    domain: application
    team: backend-{{ $labels.service }}
  annotations:
    summary: "{{ $labels.service }} 5xx 错误率 {{ printf \"%.2f\" (mul $value 100) }}%"
    impact: |
      约 {{ printf \"%.0f\" (rate(http_requests_total[5m]) * 60 * 5) }} 个请求失败
    runbook: |
      1. 查看 30-Application 错误面板
      2. 检查最新部署: `gitgit-cli deploy list --service={{ $labels.service }}`
      3. 检查依赖: PG / Redis / 外部 API
      4. 如需回滚: `gitgit-cli deploy rollback {{ $labels.service }}`
      5. 拉取 trace: 查看 5xx Span 详情
```

#### APP-02: P99 延迟
```yaml
- alert: HttpP99LatencyHigh
  expr: |
    histogram_quantile(0.99,
      sum by (le, service) (rate(http_request_duration_seconds_bucket[5m]))
    ) > 2
  for: 10m
  labels:
    severity: critical
    domain: application
    team: backend-{{ $labels.service }}
  annotations:
    summary: "{{ $labels.service }} P99 延迟 {{ printf \"%.2f\" $value }}s"
    impact: |
      用户感知慢请求；可能影响 SLO
    runbook: |
      1. 查看 70-Performance 慢 Span
      2. 检查 SQL 性能: 40-Database Top 20
      3. 检查下游依赖: 中心事件 / 外部 API
      4. 检查资源: CPU / 内存
      5. 必要时扩容
```

### 5.4 中间件（CRT-MW）

#### MW-01: Git Push 失败
```yaml
- alert: GitPushFailure
  expr: |
    sum by (repo) (rate(gitgit_git_protocol_requests_total{verb="push",status_class="5xx"}[5m])) > 0.5
  for: 5m
  labels:
    severity: high
    domain: middleware
    team: sre-mw
  annotations:
    summary: "{{ $labels.repo }} Git Push 5xx 速率高"
    impact: |
      开发者无法 push 代码
    runbook: |
      1. 查看 50-Middleware Git 协议面板
      2. 检查 gitgit-git 服务状态
      3. 检查磁盘空间（LFS / bare repo）
      4. 检查 SSH 通道
```

#### MW-02: 中心事件积压
```yaml
- alert: EventJournalBacklog
  expr: max by (topic) (gitgit_event_journal_unpublished_count) > 10000
  for: 5m
  labels:
    severity: high
    domain: middleware
    team: sre-mw
  annotations:
    summary: "中心事件 {{ $labels.topic }} 积压 {{ $value }}"
    impact: |
      消费方最终一致延迟，视图陈旧
    runbook: |
      1. 查看消费方状态: `gitgit-cli event consumer list`
      2. 扩容 consumer: `kubectl scale deploy/gitgit-server --replicas=+2`
      3. 跳过期消息（仅非关键 topic）: `gitgit-cli event skip --topic=...`
```

#### MW-03: App Bus 越权
```yaml
- alert: AppBusCapabilityDenied
  expr: |
    sum by (app, capability) (rate(gitgit_app_bus_capability_denied_total[5m])) > 0.1
  for: 10m
  labels:
    severity: high
    domain: security
    team: sec-oncall
  annotations:
    summary: "App {{ $labels.app }} 频繁被拒 {{ $labels.capability }}"
    impact: |
      App 功能受限 / 可能越权尝试
    runbook: |
      1. 检查 App 行为: 80-Security 越权面板
      2. 评估: 是 bug 还是攻击
      3. 如是攻击: 立即吊销 App 凭证
      4. 如是 bug: 通知 App 开发者
```

### 5.5 SLO（CRT-SLO）

#### SLO-01: 1h Burn Rate 高（Google SRE Workbook 模式）

```yaml
- alert: SLOAvailabilityBurnRateHigh
  expr: |
    (
      max by (slo) (slo:sli_error:ratio_rate1h{slo=~"api-.*"}) > (14.4 * 0.001)
      and
      max by (slo) (slo:sli_error:ratio_rate6h{slo=~"api-.*"}) > (6 * 0.001)
    )
  for: 2m
  labels:
    severity: critical
    domain: slo
    team: sre-slo
    slo: '{{ $labels.slo }}'
  annotations:
    summary: "SLO {{ $labels.slo }} 1h Burn Rate 高（{{ printf \"%.1f\" (mul $value 100) }}x 预算）"
    impact: |
      30 天错误预算将在 {{ printf \"%.1f\" (div 720 $value) }}h 内耗尽
    runbook: |
      1. 查看 90-SLO Burn Rate 面板
      2. 关联 trace: 查看近 1h 错误 Span
      3. 评估是否回滚最新部署
      4. 必要时启用维护模式
```

> **详细 SLO 规则**: 见 [`09-slo-design.md`](09-slo-design.md) §5

## 6. Warning 告警（趋势恶化）

| 名称 | 触发 | 阈值 |
|---|---|---|
| CPU 高水位 | `cpu_usage > 0.75` 持续 10m | warning |
| 内存高水位 | `mem_usage > 0.80` 持续 10m | warning |
| 慢查询占比 | `slow_queries / total_queries > 0.05` 持续 15m | warning |
| 缓存命中率低 | `cache_hit_ratio < 0.95` 持续 30m | warning |
| 错误率上升 | 当前错误率 > 昨日同时段 2 倍 | warning |
| Vault 接近限额 | 配额 > 80% | warning |
| 磁盘增长速率 | 预测 7 天后满 | warning |
| SLO 慢烧 | 6h burn rate > 3x | warning |
| Backup 失败 | 上次备份 > 26h 未成功 | warning |
| 凭证轮换临近 | KEK 距过期 < 30d | warning |

## 7. Info 告警（通知类）

| 名称 | 触发 | 用途 |
|---|---|---|
| 部署完成 | `gitgit_deploy_total{result="success"}` 突增 | 通知团队 |
| 视图快照生成 | `view_snapshot_created_total` | 业务通知 |
| 慢查询 Top 1 出现新 fingerprint | 检测新模式 | 优化机会 |
| ConfigMap 变更 | 检测 helm release diff | 审计 |
| 用户密码修改 | `webauthn_credential_added_total` | 安全通知 |
| Agent Run 完成 | 长任务完成 | 业务通知 |

## 8. 告警去重与合并

### 8.1 合并规则

```yaml
# alertmanager.yml 片段
route:
  group_by: ['alertname', 'cluster', 'service']
  group_wait: 30s
  group_interval: 5m
  repeat_interval: 4h
  receiver: 'sre-oncall'
  routes:
    - match_re:
        severity: critical
      receiver: 'pagerduty'
      group_wait: 10s
    - match_re:
        severity: high
      receiver: 'slack-high'
      group_wait: 30s
```

### 8.2 抑制（Inhibit Rules）

```yaml
inhibit_rules:
  # Critical 抑制同节点上所有 high
  - source_match:
      severity: 'critical'
      domain: 'infrastructure'
    target_match:
      severity: 'high'
    equal: ['instance']

  # PG Primary down 抑制 PG 业务告警
  - source_match:
      alertname: 'PostgresDown'
    target_match:
      domain: 'database'
    equal: ['cluster']

  # K3s 不可达抑制所有 Pod 告警
  - source_match:
      alertname: 'KubeNodeUnreachable'
    target_match_re:
      alertname: 'KubePod.*'
    equal: ['node']
```

### 8.3 静默（Silences）

| 维护场景 | 静默条件 | 持续时间 |
|---|---|---|
| 计划内维护 | `alertname =~ "Node.*"` + `instance=~"..."` | 维护窗口 |
| 已知 bug | `alertname = "X"`, `team = "Y"` | 修复前 |
| 测试环境 | `env = "staging"` | 永久（如需） |
| 容量扩容 | `alertname =~ "Disk.*"`, `cluster = "..."` | 扩容完成 |

## 9. 通知通道

| 通道 | 用法 | 实现 |
|---|---|---|
| PagerDuty | critical | alertmanager → PD API |
| Slack | high / warning / info | alertmanager → Slack webhook |
| 短信 | critical（oncall 必接） | alertmanager → 短信网关 |
| 电话 | critical（5min 无响应） | PD → 备用 |
| 邮件 | info / 总结 | alertmanager → SMTP |
| Webhook | 自动化响应 | alertmanager → custom |

## 10. 告警响应 SLA

| Severity | 首次响应 | 初步处置 | 解决 |
|---|---|---|---|
| critical | 5 min | 15 min | 4 h |
| high | 30 min | 1 h | 24 h |
| warning | 4 h | 24 h | 7 d |
| info | 1 d | — | — |

> **统计指标**: MTTA (Mean Time To Acknowledge) / MTTR (Mean Time To Resolve) 每月报告。

## 11. 告警治理

### 11.1 告警疲劳控制

- 每个 on-call shift 告警数 ≤ 10 / 班
- 误报率 < 10%
- 沉默 > 7d 的告警 → 重新评估
- 每月 review 一次 Top 10 告警

### 11.2 告警规则部署

```yaml
# kustomization 模式
deploy/observability/prometheus/
├── base/
│   ├── kustomization.yaml
│   └── rules/
│       ├── infrastructure.yaml
│       ├── database.yaml
│       ├── application.yaml
│       ├── middleware.yaml
│       ├── security.yaml
│       └── slo.yaml
└── overlays/
    ├── prod/
    └── staging/
```

> **强约束**: 所有 rule 走 PR review + 自动 lint（`promtool check rules`）。

## 12. 关联文档

- 上游: [`09-slo-design.md`](09-slo-design.md) §5 SLO 告警
- 上游: [`05-database-observability.md`](05-database-observability.md) / [`06-middleware-observability.md`](06-middleware-observability.md)
- 下游: [`07-dashboard-design.md`](07-dashboard-design.md)（告警状态链接到 panel）
- 下游: [`10-security-design.md`](10-security-design.md) §3 通道安全
- 下游: [`11-deployment-design.md`](11-deployment-design.md) §5 告警规则部署
- 下游: [`13-implementation-phases.md`](13-implementation-phases.md) Phase 6

## 13. 需求 ID 索引

| 需求 ID | 标题 | 优先级 |
|---|---|---|
| OBS-REQ-009 | Alert 设计 | P0 |
| OBS-REQ-010 | SLO Burn Rate | P0 |
| ALT-REQ-001 | 等级与抑制 | P0 |
| ALT-REQ-002 | 6 项质量检查 | P0 |
| ALT-REQ-003 | Critical 告警清单 | P0 |
| ALT-REQ-004 | 多窗口 + 持续时间 | P0 |
| ALT-REQ-005 | Runbook 必带 | P0 |
| ALT-REQ-006 | 通道与去重 | P0 |
| ALT-REQ-007 | 告警疲劳控制 | P1 |
| ALT-REQ-008 | 治理流程 | P1 |
