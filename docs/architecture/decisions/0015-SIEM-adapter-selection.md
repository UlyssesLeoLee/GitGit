# ADR-0015: SIEM 适配器 (admin_audit / events 转发)

> **Status**: Proposed
> **Date**: 2026-08-23
> **Deciders**: SA + TL + EM
> **Consulted**: SEC + SRE
> **Informed**: 全部

## Context and Problem Statement

[基本设计 §14.2.5](../../design/basic-design/14-admin-ops-ui.md) 与 [详设 §10.5 可观测性](../../design/detailed-design/10-observability.md) 要求 admin_audit / events 实时转发到 SIEM,以满足 SEC-REQ-004 强审计要求。

[ADR-0009 SIEM 集成方案](../../architecture/decisions/0009-SIEM-集成方案wal2json-到-Kafka-Vector-直-webhook.md) 已选 wal2json → Kafka / Vector / webhook 三选一,但没明确**具体 SIEM 适配**。本 ADR 解决"目标 SIEM 选型"。

候选 SIEM: Splunk / ELK (Elastic Stack) / Datadog / Microsoft Sentinel / AWS CloudWatch / 自建 Loki

## Decision Drivers

- [DRIVER-1] MVP 阶段不强依赖外部 SIEM(详设 §10 推迟到 V1+);MVP 用本地文件 + OTel 收集即可
- [DRIVER-2] V1+ Cloud 阶段需要选 1-2 个 SIEM 适配器
- [DRIVER-3] 不能 lock-in 到单一 SIEM(避免厂商绑定)
- [DRIVER-4] 与 [ADR-0016 OTel Collector 部署模式](#) 协调

## Considered Options

1. **Option A — OTel Collector + 多 SIEM exporter** (推荐)
2. **Option B — 直写 Splunk HEC API**
3. **Option C — 仅本地文件,不接 SIEM**

## Decision Outcome

**Chosen option**: "Option A — OTel Collector + 多 SIEM exporter", because 与 [ADR-0013 containerd](#) / [ADR-0016 OTel](#) 的"标准协议优先"原则一致;OTel 支持几乎所有主流 SIEM 的 exporter,未来切换 SIEM 零代码改动。

### 各阶段配置

| 阶段 | SIEM 目标 | 通道 |
|---|---|---|
| MVP | 本地 JSONL 文件 + 哈希链验证 | [§10.6 admin_audit](../../design/detailed-design/10-observability.md#106-审计事件转发) |
| V0.5 (单 VM staging) | 平台自带 Loki / Grafana(开发自检) | OTel logs exporter |
| V1+ Cloud | 用户自选(由部署方配置) | OTel Collector `logging` / `otlp` / `splunk_hec` / `elasticsearch` 等 exporter |

### 具体 exporter 列表 (V1+)

| SIEM | OTel Exporter | 备注 |
|---|---|---|
| Splunk | `splunk_hec` | HEC token 注入 OTel Collector config |
| ELK (Elastic) | `elasticsearch` | index 模式 `platform-events-YYYY.MM.DD` |
| Datadog | `datadog` | API key 注入 |
| Loki | `loki` | 适合轻量自建 |
| AWS CloudWatch | `awscloudwatchlogs` | AWS-only |
| Azure Sentinel | `azuremonitor` | Azure-only |
| 通配 (任何 OTLP 后端) | `otlp` | 默认通道;OTel 原生 |

### Consequences

**Good:**
- [+] 适配器可插拔;新 SIEM 零代码改动
- [+] 与 OpenTelemetry 生态完全一致;其他监控/告警系统也可复用
- [+] 部署方自选 SIEM,避免厂商锁定

**Bad:**
- [-] OTel Collector 自身需要运维(V1+ 引入 K8s 后由 [ADR-0016](#) 解决)
- [-] Exporter 配置分散,需要文档化(详设 §10.5 给示例)

### Confirmation

[CONFIRM] 详设 §13.3 admin_audit + [specs/test-specification.md §10](../../specs/test-specification.md#10-可观测性测试):
- TC-OBS-001: admin_audit 写入后 1s 内出现在 OTel Collector 接收端
- TC-OBS-002: 哈希链验证函数 `verify_admin_audit_chain()` 通过(详设 §13.3)
- TC-OBS-003: 篡改 admin_audit 任何一行后,后续行的 prev_hash 校验全部失败
- TC-OBS-004: 切换 exporter(从 `logging` 切 `otlp`)后,新事件正确路由到新目标(回归测试)

## Pros and Cons of the Options

### Option A — OTel Collector + 多 exporter
[+] 标准化;可插拔;未来 SIEM 切换零代码
[-] 需要运维 OTel Collector 本身

### Option B — 直写 Splunk HEC
[+] 直连简单
[-] Lock-in;多 SIEM 需重复实现

### Option C — 仅本地文件
[+] 零外部依赖
[-] 不满足 SEC-REQ-004 强审计要求;不可扩展

## References

- [ADR-0009: SIEM 集成方案](../../architecture/decisions/0009-SIEM-集成方案wal2json-到-Kafka-Vector-直-webhook.md)
- [ADR-0016: OTel Collector 部署模式](#)
- [基本设计 §14.2.5 SIEM 适配](../../design/basic-design/14-admin-ops-ui.md)
- [详细设计 §10 可观测性](../../design/detailed-design/10-observability.md)
- [OpenTelemetry Collector — Exporters](https://opentelemetry.io/docs/collector/configuration/#exporters)

## Revision History

| Date | Author | Change |
|---|---|---|
| 2026-08-23 | Mavis (AI 起草) | Initial draft (Proposed) |
| YYYY-MM-DD | — | Status → Accepted (待 SA + TL + EM + SEC 签核) |
