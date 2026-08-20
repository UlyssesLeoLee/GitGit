# Appendix B — 残留 TBD 项目清单（本书未确定）

| 项目 | 位置 | 解决方法 |
|---|---|---|
| 暂定 NFR 等级（RPO, RTO, 响应时间等）实测值 | [§6 非功能设计](06-non-functional-design.md) 全文 | 基准测试程序执行后，V1 正式化 |
| 支持的 OS / 架构矩阵 | [§8.1.3 支持的操作系统矩阵](08-operations-design.md#813-支持的操作系统矩阵nfr-req-003-v1-正式化) | NFR-REQ-003 在 V1 制定（Phase 14 F14-5 反映）|
| 自然灾害对策的 DR site 设计 | [§6.1 可用性](06-non-functional-design.md#61-可用性availability-ipa-grade-①) | V1+ |
| 加密密钥管理（KMS 集成 vs 本地）选择 | [§7.5 密钥管理](07-security-design.md#75-密钥管理-secrets-managementsec-req-005-sec-req-010) | 实现阶段决定，ADR-7 相关 |
| MCP 工具范围机制具体规格 | [§3.5 Agent Runtime 子系统](03-functional-design.md#35-agent-运行时子系统-agent-runtime-subsystem)、[§11.4 MCP 设计](11-api-design.md#114-mcp-设计) | ADR-8（Phase 10 §7）单独 ADR 化 |
| Agent Workspace 隔离强度（容器 vs gVisor vs Firecracker）| [§2.3 物理架构](02-architecture.md#23-物理架构-physical-architecture)、[§3.5 Agent Runtime](03-functional-design.md#35-agent-运行时子系统-agent-runtime-subsystem) | ADR-6（Phase 10 §7）在 V1 前决定 |
| GDPR 例外删除路径 | [§4.5 数据生命周期](04-data-design.md#45-数据生命周期-data-lifecycle)、[§7.2 审计](07-security-design.md#72-审计-audit) | ADR-9（Phase 10 §7）Legal Review 待决 |
| ~~语言 / 框架选择（Rust vs Go）~~ | [§2.1 逻辑架构](02-architecture.md#21-逻辑架构-logical-architecture) | **已决议 (Accepted 2026-08-19)**：选定 Rust (edition 2021, MSRV 1.75) + Tokio + Axum + sqlx + gix/shell git，详见 [技术选型文档](../../architecture/tech-selection.md) |
| Webhook 投递重试规格详细 | [§11.6 Webhook](11-api-design.md#116-webhook-outbound-api) | V1 详细设计时 |
| UI 框架 | [§5.6 UI 概述](05-interface-design.md#56-ui-概述-ui-overview) | 实现阶段决定 |
| 产品正式名称 | [§1.1 系统名称与目标](01-system-overview.md#11-系统名称与目标-system-name-target) | PRD 确定 |
| 人类干系人正式签核 | [§10.4 发布判定](10-acceptance-test-policy.md#104-发布判定-release-decision) | Phase 14 F14-7 需人类解决 |
| App 群组 V1 公开协调 API 的 schema 锁定 | [§12.4.5 端点形式](12-app-group-intercommunication.md#1245-端点形式) | V1 实施前与首个外部集成方联调时确定 |
| 存储过程版本治理（`coord_v2_*` 命名约定的实际使用）| [§12.4.7 存储过程的版本治理](12-app-group-intercommunication.md#1247-存储过程的版本治理) | 首个存储过程发生不兼容变更时确定 |

---

**导航 / Navigation:**
[← Appendix A — 可追溯性](appendix-a-traceability.md) · [README](README.md) · [← 00. 介绍 回到](00-introduction.md)
