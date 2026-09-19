# 现有系统可观测性现状分析 / Current Observability State Analysis

> **范围**: GitGit 平台（AI-Native Engineering Platform）
> **基于**: [`../design/detailed-design/`](../design/detailed-design/) 14 文件 + [`../architecture/tech-selection.md`](../architecture/tech-selection.md)
> **观察角度**: 基于**设计文档**而非运行实例（项目尚未真正部署）
> **关联 OBS-REQ**: OBS-REQ-001（建立统一 Observability Architecture）

## 1. 现有架构盘点

### 1.1 服务清单（来自 Cargo workspace）

| Crate / Binary | 类型 | 端口 | 通信方式 |
|---|---|---|---|
| `gitgit-server` | 主平台 binary | 3000 (HTTP) | Axum HTTP + Tower middleware |
| `gitgit-server` | gRPC 内部 | 50051 | Tonic gRPC |
| `gitgit-admin` | Admin 独立子进程 | 3001 (HTTP) | Axum HTTP，独立鉴权域 (SEC-REQ-011) |
| `gitgit-cli` | CLI 客户端 binary | — | clap derive + reqwest |
| `gitgit-core` | 5 原语领域类型库 | — | sqlx Repository pattern |
| `gitgit-graph` | 图谱引擎 | — | 递归 CTE + 类型注册 |
| `gitgit-policy` | 策略引擎 | — | RBAC + ABAC + 决策缓存 |
| `gitgit-ai` | AI 网关 | — | reqwest + rmcp + 提示词清洗 |
| `gitgit-agent` | Agent 运行时 | — | 状态机 + 工作区隔离 + 短期凭证 |
| `gitgit-app` | App Registry + Plugin Loader | — | Wasm + WASI (ADR-0010) |
| `gitgit-git` | Git 服务 | — | gix 读 + shell `git` 写 (ADR-0002) |

### 1.2 数据存储

| 类型 | 用途 | 详细设计 |
|---|---|---|
| PostgreSQL 15+ | 唯一持久化（5 原语 + 用户 + App + 中心事件） | §01-data-layer |
| Git bare repo 文件系统 | Git 对象存储 | §06-git-server §6.4 |
| LFS 对象存储 | 大文件 | §06-git-server §6.6 |
| 信封加密 Secrets | KEK/DEK 三层加密 | §09-security-impl |

**关键约束**: "能 PG 解决都用 PG" (ADR-0003) — 不引入 Redis/NATS/Memgraph 等独立服务

### 1.3 通信方式清单

| 方式 | 用例 | 实现 |
|---|---|---|
| HTTP/1.1 + JSON | 外部 REST API | Axum 0.7 + tower-http |
| gRPC | 内部 5 个 service | Tonic 0.12 + prost |
| WebSocket | 实时事件推送 / Admin UI | Axum WS |
| MCP (Model Context Protocol) | AI 客户端集成 | rmcp 0.1 |
| PostgreSQL wire protocol | DB 通信 | sqlx 0.8 + tokio-postgres |
| shell `git` 子进程 | 写路径 | std::process::Command |
| Wasm hostcall | App 沙箱 (WASI) | wasmtime |

### 1.4 部署形态（来自 ADR-0006）

- **MVP**: 单 binary + systemd / Docker (Local-First)
- **V1+ Cloud**: K3s 集群 + Helm chart + ArgoCD
- **App Pod 化**: 详细设计 §08.6 (V1+ K8s 部署形态)

### 1.5 现有可观测性覆盖（从设计文档盘点）

| 信号 | 状态 | 位置 |
|---|---|---|
| **Metrics** | ⏳ 部分覆盖 | `crates/gitgit-observability/` 仅有占位 lib.rs；§10-observability.md 列了 OTel + Prometheus + tracing 计划 |
| **Logs** | ⏳ 部分覆盖 | §10.6 计划用 tracing + tracing-subscriber + JSON 输出；但 redact 字段仅在 config 列出 |
| **Traces** | ❌ 未实现 | §10.3 Tracer 初始化是占位；无 OpenTelemetry Collector 配置 |
| **Events** | ❌ 未实现 | 仅有 admin_audit 表（哈希链 + 防篡改），未对接 SIEM |
| **Dashboards** | ❌ 未实现 | 0 个 Grafana dashboard |
| **Alerts** | ❌ 未实现 | 0 条 alert 规则 |
| **SLO/SLI** | ❌ 未实现 | NFR-REQ-001/002 仅有 Provisional 等级，无 Error Budget |

### 1.6 缺失能力总结

1. **统一接入层** 缺失 — 业务代码需要直接依赖 Prometheus / Loki / Tempo SDK
2. **Collector 缺失** — 没有 OpenTelemetry Collector；trace 直接打 backend 风险高
3. **关联缺失** — Metric / Log / Trace 没有 trace_id 关联
4. **采样策略缺失** — 高 QPS 场景下无 head/tail sampling
5. **脱敏缺失** — 仅 config 列出字段，代码无实现
6. **告警缺失** — 0 条 alert
7. **仪表盘缺失** — 0 个 dashboard
8. **SLO 缺失** — 仅有等级，无 burn rate
9. **DB 可观测性缺失** — 14 个 migration 没有 pg_stat_statements / lock monitor
10. **安全缺失** — 监控系统访问控制未定义

## 2. 现有 NFR 与可观测性相关约束

来自 [`../design/basic-design/06-non-functional-design.md`](../design/basic-design/06-non-functional-design.md)：

| NFR-REQ | 内容 | 影响 |
|---|---|---|
| NFR-REQ-001 | 可用性 + 灾害对策（Provisional）| SLO 设计基础 |
| NFR-REQ-002 | 性能 + 可操作性（Provisional）| 性能 baseline |
| NFR-REQ-003 | 系统环境矩阵 | OS / 架构可观测性 |
| NFR-REQ-004 | 插件沙箱隔离 | App 沙箱 metrics 强制 |
| NFR-REQ-005 | 集群健康传播延迟 | App 心跳 metric |
| NFR-REQ-006 | 中心事件端到端延迟 | Event Bus metric |
| NFR-REQ-007 | Admin 操作双因素 + 强审计 | audit_audit 强制 export |

## 3. 现有 SEC 约束

| SEC-REQ | 内容 | 监控关联 |
|---|---|---|
| SEC-REQ-001/004 | 身份认证 | auth 事件 metric + alert |
| SEC-REQ-003 | 防篡改审计 | admin_audit → SIEM 强制 (ADR-0009) |
| SEC-REQ-008 | mTLS | TLS 握手错误率 metric |
| SEC-REQ-009 | 持续漏洞管理 | 漏洞扫描 metric |
| SEC-REQ-010 | KEK 轮换 | 轮换事件 metric |
| SEC-REQ-011 | Admin 独立鉴权域 | 鉴权失败 metric 按 issuer 分桶 |
| SEC-REQ-012 | 关键操作双因素 | MFA 事件 metric |
| SEC-REQ-013 | App 沙箱 DB role 隔离 | 越权访问 metric + alert |

## 4. 现有 AISEC 约束

| AISEC-REQ | 内容 | 监控关联 |
|---|---|---|
| AISEC-REQ-001 | Prompt injection 防御 | 检测事件 metric + alert |
| AISEC-REQ-002 | MCP tool allowlist | 拒绝率 metric |
| AISEC-REQ-006 | Agent 输出 policy bypass | 检测事件 |
| AISEC-REQ-007 | Agent 异常行为 | 频率 / 异常率 metric |
| AISEC-REQ-008 | 外部 MCP 沙箱 | 沙箱违规 metric |
| AISEC-REQ-009(a) | events 表 append-only | UPDATE/DELETE 触发器告警 |
| AISEC-REQ-013 | App 沙箱 DB role 隔离 | App → DB 越权告警 |

## 5. 痛点总结

1. **可观测性代码占 0%** — `crates/gitgit-observability/` 是空 lib.rs
2. **没有 Collector** — trace 直接打 backend 是反模式
3. **没有脱敏实现** — 仅列了字段名
4. **没有关联** — metric/log/trace 三套独立，无法故障定位
5. **没有 Alert** — P0 事件无自动通知
6. **没有 SLO** — Provisional 等级无法量化 burn rate

## 6. 结论

**项目处于"可观测性 0 起步"状态**。本设计文档的目标是用 **最小侵入方式**补齐完整的 Observe → Detect → Correlate → Diagnose → Alert → Recover 闭环。
