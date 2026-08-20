# 06. 非功能设计 / Non-Functional Design

> **[PROPOSAL] IPA 共通框架 2013 章节定位：** P3.A1.T2 识别非功能要求 / P3.A2.T5 研讨可靠性·性能·运行性


本章按 IPA/SEC《非功能要求等级》(非功能要求等级) 的 6 大项分类构成。每个类别整合 Phase 14 的差距分析（F14-1〜F14-6）及其 Accepted-and-fixed 反映（NFR-REQ-001〜003）。

> **注：** MVP 阶段的性能 / 可用性目标具体数值为 [TBD] – Benchmark Required。`[PROPOSAL]` 列出暂定 Provisional 等级，基准测试后正式确定。遵循 IPA 通用等级的"先定暂定等级、再细化"原则。

## 6.1 可用性（Availability）— IPA Grade ①

| 子项 | 暂定等级（Provisional MVP）| 正式目标 | 负责需求 |
|---|---|---|---|
| 持续性（计划 / 非计划停机容许）| L2/5（允许计划性维护窗、单实例）| 0.99 以上（V1），0.999（Cloud）| NFR-REQ-001（Phase 14 F14-1 反映）|
| 抗故障性（冗余）| L1/5（MVP：单节点）| 主动-备用（V1）| CLOUD-REQ-003 |
| 灾害对策（RTO/RPO）| RTO 24h, RPO 24h（MVP, Local）| RTO 1h, RPO 1h（Cloud, V1）| **NFR-REQ-001（Phase 14 F14-1+F14-2 反映，F14-2 已作为"companion tier statement"折入 NFR-REQ-001，修正 BKP-REQ-002）** |

**设计：**

- Local 部署单进程。GIT-REQ-010（与 AI 子系统解耦的可用性）通过同进程内代码路径隔离实现（Phase 10 §4）。
- Cloud 部署为主动-主动 Platform Process × 托管 PostgreSQL（RPO = WAL 间隔，[`TBD`] Benchmark Required）。
- 计划性维护通过 `/api/v1/admin/maintenance` 标志对外通知，通过 UX-REQ-002 在 Dashboard 显示横幅。

## 6.2 性能与扩展性（Performance / Scalability）— IPA Grade ②

| 子项 | 暂定等级（Provisional MVP）| 正式目标 |
|---|---|---|
| 响应时间（读 API）| 95p < 200ms（目标）、[TBD] Benchmark | 同上 |
| 响应时间（写 API）| 95p < 500ms（目标）、[TBD] Benchmark | 同上 |
| 吞吐 | 100 req/s（目标）、[TBD] Benchmark | 1,000 req/s（Cloud）|
| 图查询（4 hop）| < 1s（目标）、[TBD] Benchmark | < 500ms（V1）|
| 同时连接用户数 | 50（MVP Local）| 5,000（Cloud）|
| 水平扩展余地 | 可水平扩缩（无状态实例）| CLOUD-REQ-003 |

**设计：**

- 单一 PostgreSQL 实例满足 MVP 全部需求的前提。递归 CTE 的 Edge 数上限达到时按 Phase 10 §1.3 的重评估触发条件再评估。
- 读密集型图查询在 Cloud 时分流到读副本。
- Policy 评估用进程内缓存，目标 1 评估 < 1ms，[`TBD`] Benchmark Required。
- 上下文组装（CTX-REQ-001）带深度限制 + 结果缓存（CTX-REQ-004, V1）。

## 6.3 运行与维护性（Operability / Maintainability）— IPA Grade ③

| 子项 | 暂定等级 | 正式目标 | 负责需求 |
|---|---|---|---|
| 监控 | L2/5（仅基本指标）| L3/5（V1）| OBS-REQ-001/002/003 |
| 日志保留 | 30 天（MVP）| 90 天（V1）、SIEM 集成（V1+）| OBS-REQ, SEC-REQ-006 |
| 计划维护窗 | 月 1 次 4h（MVP, [TBD]）| 月 1 次 2h（V1）| **NFR-REQ-002（Phase 14 F14-6 反映，F14-6 已折入 NFR-REQ-002 — 作为 Performance/Scalability + Operability/Maintainability 等级的统一管理）** |
| 补丁应用节奏 | 安全补丁：7 天内（V1）| 同上 | SEC-REQ-009（Phase 14 F14-4 反映）|
| GC / 重打包（Git）| 每周（V1, GIT-REQ-011）| 同上 | GIT-REQ-011 |
| 运维文档 | 安装、配置、备份、恢复步骤 | 同上 + V1 增加监控 / 运维 Runbook | OPS-REQ-005 |

**设计：**

- 指标 / 追踪 / 日志的导出格式遵循 OpenTelemetry（Phase 10 §2）。
- 监控后端（Prometheus 等）MVP 不内置，仅提供 OTel exporter（V1 由部署者提供）。
- 运维 Runbook 在 V1 整理。

## 6.4 可迁移性（Migration / Portability）— IPA Grade ④

| 子项 | 设计 | 负责需求 |
|---|---|---|
| 数据导出完整性 | Graph + Git + Secrets 三个系统统一 export | DATA-REQ-001 |
| 格式开放性 | 标准格式（PostgreSQL dump, git bundle, 加密 secrets bundle）| DATA-REQ-002, GIT-REQ-009 |
| 迁移工具验证 | round-trip 测试（export → import → structural diff）| DATA-REQ-001 |
| "无回拨"保证 | 默认无外部通信（明确 opt-in 时才进行 AI 调用）| DATA-REQ-004 |
| 避免厂商锁定 | 仅使用标准协议 / 格式，无自创格式 | Phase 10 的 4 个"能"原则 |

**本类别是 Phase 5 差距分析 §2 中本项目最强领域的评价。**

## 6.5 安全性（Security）— IPA Grade ⑤

详见 [§7 安全设计](07-security-design.md)。反映 Phase 14 的 F14-3, F14-4, F14-9：

| 子项 | 设计 | 负责需求 |
|---|---|---|
| 访问 / 使用限制 | RBAC/ABAC、deny-by-default、范围受限 Agent 凭据 | SEC-REQ-001/002/004 |
| 数据保密 | TLS in-transit、encryption at-rest（PostgreSQL TDE 等价）、secrets 信封加密 | SEC-REQ-010（Phase 14 F14-9）|
| 防篡改 / 防破坏 | 结构化审计、tamper-evident Event 日志、仅追加约束 | SEC-REQ-003 |
| 网络层控制 | mTLS（Cloud）、内部服务间 TLS、网络隔离 | SEC-REQ-008（Phase 14 F14-3）|
| 持续安全风险管理 | 持续漏洞扫描、补丁节奏、定期评审 | SEC-REQ-009（Phase 14 F14-4）|

## 6.6 系统环境与生态（System Environment / Ecology）— IPA Grade ⑥

| 子项 | 设计 | 负责需求 |
|---|---|---|
| 支持的 OS / 架构 | Linux x86_64/aarch64、macOS aarch64、Windows x86_64 | **NFR-REQ-003**（Phase 14 F14-5 反映，V1-timed）|
| 容器运行时 | Docker / containerd / Podman（兼容 OCI）| OPS-REQ-001 |
| 必要资源（Local）| CPU 2 core、RAM 4GB、Disk 20GB（MVP 100 仓库假设）| OPS-REQ-005 |
| 电源 / 消耗 | 标准服务器 / 笔记本电脑 | N/A（本平台不依赖 IaaS）|
| 废弃 / 数据处置 | 全数据完整删除步骤文档化，V1 实现指南 | **NFR-REQ-003**（V1）|

## 6.7 App 集群与可热插拔的扩展非功能要求

**[PROPOSAL]** §13 / §14 / 详细设计 §12-13 引入的"App 一级化 + 中心事件总线 + 插件热插拔 + Admin 运维界面"带来 4 项新 NFR-REQ，与原 6 大项正交。完整 REQ-ID 在 §13.9 / §14.6 列出。

| 子项 | 设计 | 负责需求 |
|---|---|---|
| **插件沙箱隔离** | App 独立 schema + 独立 DB role + PL/pgSQL SECURITY DEFINER 校验 + 显式 `required_grants` / `forbidden_grants`；禁止 App 直接 INSERT/UPDATE/DELETE event_stream（必须通过 trigger_event_publish 函数）；App 跨 schema 调用必须经 `coord_call_app_proc` 中介且 `app_acl` 表显式授权 | **NFR-REQ-004**（新增）|
| **集群健康传播延迟** | App instance 每 5s 心跳写 `app_heartbeats` 表；平台每 10s 检查 stale instance（>30s 未心跳 → degraded，>90s → failed）；Event Relay 自动从分发列表移除失败 instance | **NFR-REQ-005**（新增）|
| **中心事件端到端延迟** | 业务事务内同步写入 `event_stream`（Outbox 模式）；Relay 通过 LISTEN/NOTIFY 实时唤醒；派发到 subscriber P50 ≤ 500ms，P99 ≤ 5s；DLQ 监控 30s 内告警 | **NFR-REQ-006**（新增）|
| **Admin 操作双因素 + 强审计** | 关键操作（App 升级 / 回滚 / KEK 轮换 / 用户角色变更）必须双因素（TOTP 或 WebAuthn）；全部 admin 操作入 `admin_audit` 表（哈希链不可篡改）+ OTel span + SIEM 转发；Admin 鉴权域与终端用户鉴权域完全分离（独立 issuer / 签名密钥 / RBAC 表） | **NFR-REQ-007**（新增）|

**与原 NFR-REQ-001〜003 的关系**：

- NFR-REQ-001（可用性 + 灾害对策）：**增强**——平台可用性 = 核心 API 可用性 + Admin API 可用性 + 所有 App 健康综合判定
- NFR-REQ-002（性能/扩展性 + 运行/保守性）：**增强**——增加 App 安装/升级/回滚的耗时预算（§12.7）
- NFR-REQ-003（系统环境/生态）：**增强**——K8s 部署形态成为 V1+ Cloud 的"一等部署"（§8.6）

**测量方法**：

| NFR-REQ | 测量方法 | 频率 |
|---|---|---|
| NFR-REQ-004 插件隔离 | 自动化测试：恶意 App 尝试直接 `INSERT INTO event_stream` → 应被 PG 权限拒绝 | 每 PR |
| NFR-REQ-005 集群健康传播 | k6 注入 instance crash → 测量 30s/90s 阈值触发 → 自动化断言 | 每夜 |
| NFR-REQ-006 中心事件延迟 | k6 注入 100 events/s → 测量 P50/P99 派发延迟 | 每夜 |
| NFR-REQ-007 双因素 | 自动化测试：尝试带 admin JWT 但 totp_verified=false 调升级 → 必须 401 | 每 PR |

---

**导航 / Navigation:**
[← 05. 接口设计概述](05-interface-design.md) · [README](README.md) · [07. 安全设计 →](07-security-design.md)
