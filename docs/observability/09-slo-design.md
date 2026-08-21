# SLO / SLI 设计 / SLO & Error Budget Design

> **关联 OBS-REQ**: OBS-REQ-010（SLO/SLI） / OBS-REQ-011（Error Budget） / OBS-REQ-012（Burn Rate）
> **关联设计**: [`08-alert-design.md`](08-alert-design.md)（SLO 告警） / [`02-metrics.md`](02-metrics.md) / [`07-dashboard-design.md`](07-dashboard-design.md)（90-SLO）
> **方法**: Google SRE Workbook（SLO + Error Budget + Multi-Window Burn Rate）
> **目标**: 把"系统是否健康"从**感觉**变成**数字**，让工程决策有量化依据

## 1. SLO 总览

### 1.1 核心 SLO 表

| ID | 名称 | SLI | 目标 | 窗口 | 错误预算（30d） |
|---|---|---|---|---|---|
| SLO-API-01 | `api-availability` | 成功请求比例 | 99.9% | 30d | 43m20s |
| SLO-API-02 | `api-latency` | 端到端 P99 延迟 | < 500ms | 30d | 比例目标 |
| SLO-API-03 | `git-push-success` | Push 成功率 | 99.5% | 30d | 3h36m |
| SLO-API-04 | `ai-gateway-success` | AI 网关成功率 | 99.0% | 30d | 7h12m |
| SLO-DATA-01 | `pg-write-latency` | 主库写 P99 | < 100ms | 30d | 比例目标 |
| SLO-DATA-02 | `event-journal-lag` | 事件消费延迟 | P95 < 30s | 30d | 比例目标 |
| SLO-PLAT-01 | `platform-availability` | 平台综合（无 critical 告警） | 99.95% | 30d | 21m40s |
| SLO-SEC-01 | `audit-chain-integrity` | 哈希链 100% 完整 | 100% | 持续 | 0 |
| SLO-SEC-02 | `secrets-rotation` | KEK 轮换合规率 | 100% | 30d | 0 |
| SLO-RUN-01 | `agent-run-completion` | Agent Run 完成率 | 95% | 30d | 36h |

> **可调整**: 实际目标值需在 SRE + PM 协商后定。本表为初版（[TBD] – Benchmark Required）。

### 1.2 SLO 文档结构

```yaml
# slo/api-availability.yaml
apiVersion: sloth.slok.dev/v1
kind: SLO
metadata:
  name: api-availability
  labels:
    service: gitgit-server
    tier: critical
spec:
  description: "REST API 端到端可用性"
  service: gitgit-server
  sli:
    events:
      error_query: |
        sum(rate(http_requests_total{service="gitgit-server",status=~"5.."}[{{ .window }}]))
      total_query: |
        sum(rate(http_requests_total{service="gitgit-server"}[{{ .window }}]))
  objectives:
    - name: availability-99.9
      target: 0.999
      window: 30d
    - name: availability-99.0
      target: 0.99
      window: 30d
  alerting:
    name: api-availability
    page_alert:
      labels:
        severity: critical
      annotations:
        summary: "API 可用性 SLO 错误率超 1h 14.4x 预算"
    ticket_alert:
      labels:
        severity: warning
```

## 2. SLI 设计

### 2.1 三大类 SLI

| 类型 | 公式 | 适用 |
|---|---|---|
| **可用性 SLI** | `success_events / total_events` | API / Push / 任务 |
| **延迟 SLI** | `fast_events / total_events` | API / DB / 事件消费 |
| **正确性 SLI** | `correct_events / total_events` | AI / 业务计算 |

> **关键原则**: SLI **必须**由**用户视角**的成功事件定义，不是内部组件状态。

### 2.2 各 SLO 的 SLI 实现

#### SLO-API-01: API 可用性

```promql
# error
sum(rate(http_requests_total{service="gitgit-server",status=~"5.."}[5m]))

# total
sum(rate(http_requests_total{service="gitgit-server"}[5m]))

# SLI 比率 = 1 - error/total
```

**关键决策**:
- **4xx 算不算错误？** → 算（用户请求未成功），但单独区分 4xx/5xx 用于调试
- **健康检查算不算？** → 排除 `path=~"/healthz|/readyz"`
- **admin endpoint 算不算？** → **算**（与外部 API 同样重要，SEC-REQ-011）

#### SLO-API-02: API 延迟

```promql
# 快请求: latency < 500ms
sum(rate(http_request_duration_seconds_bucket{service="gitgit-server",le="0.5"}[5m]))

# 总请求
sum(rate(http_request_duration_seconds_count{service="gitgit-server"}[5m]))

# SLI = fast/total
```

**关键决策**:
- **目标值**: P99 < 500ms（v1）
- **何时重审**: 真实流量达到 1000 QPS 后
- **采样窗口**: 5m（与 alert 5m 一致）

#### SLO-API-03: Git Push 成功

```promql
# error: push 失败（4xx + 5xx）
sum(rate(gitgit_git_protocol_requests_total{verb="push",status_class=~"4xx|5xx"}[5m]))

# total: push 全部
sum(rate(gitgit_git_protocol_requests_total{verb="push"}[5m]))
```

#### SLO-API-04: AI 网关成功

```promql
# error: 5xx + 超时 + 不可用
sum(rate(gitgit_ai_gateway_requests_total{result=~"error|timeout"}[5m]))

# total
sum(rate(gitgit_ai_gateway_requests_total[5m]))
```

**特殊点**: AI 调用偶发 provider 故障，但用户不能感知。

#### SLO-DATA-01: PG 写延迟

```promql
# 快写: duration < 100ms
sum(rate(pg_stat_statements_total_exec_time_seconds_bucket{query_class="write",le="0.1"}[5m]))

# 总写
sum(rate(pg_stat_statements_total_exec_time_count{query_class="write"}[5m]))
```

**注**: PG 写入包含 `INSERT` / `UPDATE` / `DELETE`。

#### SLO-DATA-02: 事件消费延迟

```promql
# 快消费: lag < 30s
sum(rate(gitgit_event_journal_lag_seconds_bucket{le="30"}[5m]))

# 总消费记录
sum(rate(gitgit_event_journal_lag_seconds_count[5m]))
```

#### SLO-PLAT-01: 平台综合

```promql
# error: 任意 critical 告警 firing
count(ALERTS{alertstate="firing",severity="critical"})

# total: 60 (分钟) × 30 (天) = 43200 期望可用分钟
# SLI = (total - error) / total
```

> **争议**: 平台综合 SLO 是**派生**指标。30d 内 critical 告警累积时长不应超过 21m40s。

#### SLO-SEC-01: 审计链完整性

```promql
# error: 哈希链断裂 1 次 = 100% 失败
max(gitgit_audit_hash_chain_broken)

# total: 检查执行次数
sum(rate(gitgit_audit_chain_check_total[5m]))

# SLI = 1 - (error/total)
```

> **SLO = 100%**: 任何一次断裂 = SLO 失守。

## 3. Error Budget

### 3.1 30 天预算

| SLO 目标 | 30d 预算 | 含义 |
|---|---|---|
| 99.9% | 43m20s | 每月 43 分钟 20 秒可失败 |
| 99.5% | 3h36m | 每月 3 小时 36 分钟可失败 |
| 99.0% | 7h12m | 每月 7 小时 12 分钟可失败 |
| 95.0% | 36h | 每月 36 小时可失败 |
| 100% | 0 | 不允许失败 |

### 3.2 预算燃烧速率（Burn Rate）

**公式**:
```
burn_rate = (1 - current_availability) / (1 - SLO_target)
```

**示例**: SLO 99.9% 的服务
- 当前 30d 可用性 99% → burn_rate = (1 - 0.99) / (1 - 0.999) = 0.01 / 0.001 = 10x
- 含义: 错误预算在 30d / 10 = 3d 内耗尽

### 3.3 多窗口 Burn Rate Alert（Google SRE Workbook）

| 报警 | 1h burn | 6h burn | 24h burn | 3d burn | 响应 |
|---|---|---|---|---|---|
| **Page（critical）** | > 14.4x | > 6x | — | — | 5 min 响应 |
| **Ticket（warning）** | — | — | > 1x | > 1x | 24h 内 review |

> **关键**: 双窗口要求（同时满足）才触发，避免短暂抖动误报。

**PromQL 实现**:

```promql
# Page: 1h 快 + 6h 中 双窗口
(
  max by (slo) (slo:sli_error:ratio_rate1h{slo="api-availability"}) > (14.4 * 0.001)
  and
  max by (slo) (slo:sli_error:ratio_rate6h{slo="api-availability"}) > (6 * 0.001)
)

# Ticket: 24h + 3d 双窗口
(
  max by (slo) (slo:sli_error:ratio_rate24h{slo="api-availability"}) > 0.001
  and
  max by (slo) (slo:sli_error:ratio_rate3d{slo="api-availability"}) > 0.001
)
```

## 4. 预算耗尽后的处理

### 4.1 治理流程

```
错误预算耗尽 / 接近耗尽
   │
   ▼
PM + SRE + Tech Lead 联合 review
   │
   ├── 影响分析
   │     - 哪些功能受 SLO 影响
   │     - 多少用户受影响
   │     - 收入影响估算
   │
   ├── 根因分析
   │     - 最近部署？
   │     - 流量变化？
   │     - 依赖变化？
   │
   └── 决策
         │
         ├── 修复优先（冻结非关键功能）
         ├── 调整 SLO（需多方同意）
         └── 投资容量 / 性能
```

### 4.2 自动化响应（实验性）

| 预算剩余 | 自动行为 |
|---|---|
| < 30d 预算的 10% | 禁止非关键部署（CI 检查） |
| < 30d 预算的 5% | 触发 incident 自动开 P2 |
| = 0 | 冻结所有非紧急变更 |

## 5. SLO 告警规则（完整）

### 5.1 核心 SLO Alert Group

```yaml
groups:
  - name: slo-alerts
    interval: 30s
    rules:
      # === Page Alert: critical 1h + 6h ===
      - alert: SLO_Availability_Page
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
        annotations:
          summary: "SLO {{ $labels.slo }} Page: 1h+6h 预算燃烧"
          runbook: "https://runbooks.example.com/slo/page"

      # === Ticket Alert: 24h + 3d ===
      - alert: SLO_Availability_Ticket
        expr: |
          (
            max by (slo) (slo:sli_error:ratio_rate24h{slo=~"api-.*"}) > 0.001
            and
            max by (slo) (slo:sli_error:ratio_rate3d{slo=~"api-.*"}) > 0.001
          )
        for: 30m
        labels:
          severity: warning
          domain: slo
          team: sre-slo
        annotations:
          summary: "SLO {{ $labels.slo }} Ticket: 24h+3d 预算燃烧"
          runbook: "https://runbooks.example.com/slo/ticket"

      # === 预算耗尽 ===
      - alert: SLO_Budget_Exhausted
        expr: slo:current_burn_rate_30d{slo=~"api-.*"} >= 1.0
        for: 5m
        labels:
          severity: critical
          domain: slo
          team: sre-slo
        annotations:
          summary: "SLO {{ $labels.slo }} 30d 错误预算耗尽"
          impact: "需启动 SLO 治理流程"
          runbook: |
            1. 立即召集 PM + SRE + Tech Lead
            2. 启动 incident
            3. 冻结非关键变更
            4. 启动根因分析
```

## 6. SLI Recording Rules

```yaml
groups:
  - name: slo-recording
    interval: 30s
    rules:
      # SLI error rate 5m
      - record: slo:sli_error:ratio_5m
        expr: |
          sum by (slo) (
            rate(slo_error_events_total[5m])
          ) / ignoring(__name__)
          sum by (slo) (
            rate(slo_total_events_total[5m])
          )

      # 1h / 6h / 24h / 3d burn rate
      - record: slo:sli_error:ratio_rate1h
        expr: slo:sli_error:ratio_5m
      - record: slo:sli_error:ratio_rate6h
        expr: |
          sum_over_time(slo:sli_error:ratio_5m[6h]) / 720
      - record: slo:sli_error:ratio_rate24h
        expr: |
          sum_over_time(slo:sli_error:ratio_5m[24h]) / 2880
      - record: slo:sli_error:ratio_rate3d
        expr: |
          sum_over_time(slo:sli_error:ratio_5m[3d]) / 8640

      # 30d 燃烧
      - record: slo:current_burn_rate_30d
        expr: |
          (
            1 - (
              sum by (slo) (increase(slo_success_events_total[30d]))
              /
              sum by (slo) (increase(slo_total_events_total[30d]))
            )
          ) / (1 - on(slo) slo_info_target)
```

## 7. SLO 评审

### 7.1 评审周期

| 频率 | 评审内容 |
|---|---|
| **每日** | SLO dashboard 检查（晨会） |
| **每周** | 错误预算燃烧 review（周会） |
| **每月** | SLO 目标 vs 实际 / 调整（月初） |
| **每季** | SLO 与业务目标对齐 / 重新定义（季初） |

### 7.2 评审指标

| 指标 | 含义 |
|---|---|
| **MTTA** | Mean Time To Acknowledge（响应时间） |
| **MTTR** | Mean Time To Resolve（解决时间） |
| **Budget Burn Rate** | 30d 预算燃烧速率 |
| **Compliance %** | 实际可用性 vs SLO 目标 |
| **Page Frequency** | critical 告警触发频次 |

### 7.3 SLO 调整流程

```
提议: 某 SLO 目标值需调整
   │
   ▼
SRE + PM + Tech Lead 联合评审
   │
   ├── 影响业务？ (SLA 合同)
   ├── 历史合理性？ (过去 90d 数据)
   ├── 是否为技术债？ (应改善而非降标)
   │
   ▼
   ├── 拒绝（不允许降标除非客观条件变化）
   ├── 通过 (更新 SLO 文档 + 通知)
   └── 暂缓 (设定观察期)
```

## 8. 与告警的对应

> **重要**: SLO alert **不替代**基础设施告警。两者互补。

| 类型 | 触发条件 | 响应 |
|---|---|---|
| 基础设施告警 | 资源 / 组件级异常 | 立即修复（即使 SLO 还没受影响） |
| SLO 告警 | 用户感知的服务降级 | 立即修复（用户已受影响） |
| 预算警告 | 长期趋势 | 周会讨论 |

## 9. 不该有的 SLO（反模式）

| ❌ 反模式 | 原因 |
|---|---|
| "100% 可用" | 不现实，反而导致过度投入 |
| "0 延迟" | 不可能 |
| "覆盖所有路径" | 不可观测 = 不可管理 |
| "实时性 SLO"（如 < 1s）| 多数用户感知不到差异 |
| "包含维护窗口" | 应分开计算 |

## 10. 关联文档

- 上游: [`02-metrics.md`](02-metrics.md) §2 SLI 指标
- 上游: [`08-alert-design.md`](08-alert-design.md) §5.5 SLO 告警
- 下游: [`07-dashboard-design.md`](07-dashboard-design.md) §12 90-SLO
- 下游: [`15-self-review-v2.md`](15-self-review-v2.md) §3 SLO 反模式

## 11. 需求 ID 索引

| 需求 ID | 标题 | 优先级 |
|---|---|---|
| OBS-REQ-010 | SLO/SLI 设计 | P0 |
| OBS-REQ-011 | Error Budget | P0 |
| OBS-REQ-012 | Burn Rate | P0 |
| SLO-REQ-001 | 核心 10 SLO | P0 |
| SLO-REQ-002 | 多窗口 Burn Rate | P0 |
| SLO-REQ-003 | 预算耗尽流程 | P0 |
| SLO-REQ-004 | SLO 评审 | P1 |
| SLO-REQ-005 | Recording Rules | P0 |
| SLO-REQ-006 | 自动化响应（实验） | P2 |
