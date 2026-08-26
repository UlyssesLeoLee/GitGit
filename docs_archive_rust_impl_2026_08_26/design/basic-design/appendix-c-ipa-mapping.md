# Appendix C — IPA 共通框架 2013 过程·交付物 对照表

> 本附录是 [00. 介绍 §0.0 依据宣言](00-introduction.md#00-依据宣言-compliance-declaration) 的细节展开。读者可据此判断本书每一章在 IPA 共通框架 2013 生命周期中的位置，以及哪些过程-活动-任务被本书**覆盖**、哪些被**省略**、哪些被**显式延后**。

## C.1 范围与方法

- **范围：** 仅覆盖本书 8 个主流程过程（开发主流程），不涉及运行·保守过程（这部分由运维 Runbook 承担，超出本书）
- **方法：** 自 IPA 共通框架 2013 的 9 个过程（系统化计划 / 需求定义 / 系统方式设计 / 软件方式设计 / 软件详细设计 / 软件构建 / 软件单体测试 / 软件结合测试 / 软件受入）出发，逆向对照本书 16 个正文 / 附录章节
- **覆盖符号：**
  - ✅ = 完整覆盖（含详细的章/节/表）
  - 🟡 = 部分覆盖（提到但未深入）
  - ⏸ = 显式延后（标 TBD 或 V1+）
  - ➖ = 不在本书范围

## C.2 过程级覆盖矩阵

| IPA 共通框架 2013 过程 | 阶段 | 本书覆盖 | 主要交付物（本书） |
|---|---|---|---|
| P1. 系统化计划过程 (System Planning) | 上游 | ➖ | 项目计划不在本书范围 |
| P2. 需求定义过程 (Requirements Definition) | 上游 | ➖ | 需求定义书 v1.0 在 [`../../requirements/`](../../requirements/)（独立） |
| **P3. 系统方式设计过程 (System Architecture Design)** | **本书主体** | **✅** | **本基本设计书 = [`../`](../) 全部 16 文件** |
| **P4. 软件方式设计过程 (Software Architecture Design)** | 详细设计 | ✅ | [`../detailed-design/00-overview.md`](../detailed-design/00-overview.md), [`04-agent-runtime.md`](../detailed-design/04-agent-runtime.md), [`07-app-coordination.md`](../detailed-design/07-app-coordination.md) |
| **P5. 软件详细设计过程 (Software Detailed Design)** | 详细设计 | ✅ | [`../detailed-design/01-data-layer.md`](../detailed-design/01-data-layer.md), [`02-graph-engine.md`](../detailed-design/02-graph-engine.md), [`03-policy-engine.md`](../detailed-design/03-policy-engine.md), [`05-ai-gateway.md`](../detailed-design/05-ai-gateway.md), [`06-git-server.md`](../detailed-design/06-git-server.md), [`08-api-handlers.md`](../detailed-design/08-api-handlers.md), [`09-security-impl.md`](../detailed-design/09-security-impl.md) |
| P6. 软件构建过程 (Software Construction) | 实现 | ➖ | 源代码 + 单元测试，在 `src/`（未来实现阶段） |
| P7. 软件单体测试 (Software Unit Test) | 实现 | 🟡 | [10. 受入测试方针](10-acceptance-test-policy.md) §10.1 提到级别 |
| P8. 软件结合测试 (Software Integration Test) | 实现 | 🟡 | [10. 受入测试方针](10-acceptance-test-policy.md) §10.1 |
| **P9. 软件受入过程 (Software Acceptance)** | 验收 | **✅** | **[10. 受入测试方针](10-acceptance-test-policy.md) 全文** |
| P10. 软件保守过程 (Software Maintenance) | 运维 | 🟡 | [08. 运维设计](08-operations-design.md) 覆盖运维主体 |

## C.3 P3 系统方式设计过程 详细对应

> **IPA 共通框架 2013 §3.4 系统方式设计过程 构成：** A1 确定系统要求 / A2 研讨系统方式 / A3 评价系统方式 / A4 编写系统方式设计书

### C.3.1 A1 确定系统要求

| 任务 | 本书对应章节 | 覆盖状态 |
|---|---|---|
| T-A1-1 识别功能要求 | [03. 功能设计](03-functional-design.md) §3.1-3.8 全部 8 子系统 | ✅ |
| T-A1-2 识别非功能要求 | [06. 非功能设计](06-non-functional-design.md) §6.1-6.6 全部 6 大项 | ✅ |
| T-A1-3 确定系统边界 | [01. 系统概述](01-system-overview.md) §1.3 系统边界 / [02. 系统方式设计](02-architecture.md) §2.1 逻辑架构 | ✅ |
| T-A1-4 识别外部接口 | [11. API 设计](11-api-design.md) §11.1 协议栈总览 | ✅ |
| T-A1-5 识别系统构成要素 | [02. 系统方式设计](02-architecture.md) §2.2 部署单位 | ✅ |
| T-A1-6 研讨数据模型 | [04. 数据设计](04-data-design.md) §4.1 概念数据模型 | ✅ |
| T-A1-7 识别安全要求 | [07. 安全设计](07-security-design.md) §7.1-7.6 全部 6 子系统 | ✅ |
| T-A1-8 识别迁移要求 | [09. 迁移设计](09-migration-design.md) §9.1 迁移场景 | ✅ |

### C.3.2 A2 研讨系统方式

| 任务 | 本书对应章节 | 覆盖状态 |
|---|---|---|
| T-A2-1 编制硬件·软件构成候选 | [02. 系统方式设计](02-architecture.md) §2.2-2.6 | ✅ |
| T-A2-2 编制系统功能构成候选 | [03. 功能设计](03-functional-design.md) §3.1-3.8 | ✅ |
| T-A2-3 编制数据构成候选 | [04. 数据设计](04-data-design.md) §4.1-4.6 | ✅ |
| T-A2-4 编制接口构成候选 | [11. API 设计](11-api-design.md) 全文 | ✅ |
| T-A2-5 研讨可靠性·性能·运行性候选 | [06. 非功能设计](06-non-functional-design.md) §6.1-6.3 | ✅ |
| T-A2-6 研讨安全方式候选 | [07. 安全设计](07-security-design.md) 全文 | ✅ |
| T-A2-7 研讨迁移方式候选 | [09. 迁移设计](09-migration-design.md) §9.2 迁移步骤 | ✅ |
| T-A2-8 研讨 App 群组信息互通方式候选 | [12. App 群组信息互通设计](12-app-group-intercommunication.md) 全文 + [13. App 集群与可热插拔架构](13-app-cluster-and-plugins.md) 全文（升级为中心事件总线 + 集群模型）| ✅ (本书独立专章 + 升级专章) |

### C.3.3 A3 评价系统方式

| 任务 | 本书对应章节 | 覆盖状态 |
|---|---|---|
| T-A3-1 评价系统方式方案 | [Appendix A — 设计依据的可追溯性](appendix-a-traceability.md) 全文 | ✅ |
| T-A3-2 反映评价结果 | [Appendix A](appendix-a-traceability.md) "反映位置"列 | ✅ |

### C.3.4 A4 编写系统方式设计书

| 任务 | 本书对应章节 | 覆盖状态 |
|---|---|---|
| T-A4-1 确定系统方式设计书构成 | [00. 介绍 §0.7 本书构成](00-introduction.md#07-本书构成-document-structure) | ✅ |
| T-A4-2 编写系统方式设计书 | 全部 16 文件 | ✅ |
| T-A4-3 实施系统方式设计书评审 | [Appendix B — 残留 TBD 项目清单](appendix-b-tbd.md)（人类签核的占位） | 🟡 |
| T-A4-4 获取人类干系人承认 | [00. 介绍 §0.2.2 不包含](00-introduction.md#022-不包含-out-of-scope)（Phase 14 F14-7 缺口） | ⏸ |

## C.4 P4 + P5 软件方式/详细设计过程 详细对应

> **IPA 共通框架 2013 §3.5/3.6 软件方式设计过程 / 软件详细设计过程 构成：**
> P4 活动：A1 设计软件方式 / A2 评价软件方式 / A3 编写软件设计书
> P5 活动：A1 设计软件详细方式 / A2 评价软件详细方式 / A3 编写软件详细设计书

### C.4.1 P4 任务 → 详细设计章节 对应

| 任务 | 详细设计章节 |
|---|---|
| T-P4-A1-1 设计软件构成 | [00. 总体概述 §2 顶层模块划分](../detailed-design/00-overview.md#02-顶层模块划分) |
| T-P4-A1-2 设计子系统间接口 | [08. API 处理器](../detailed-design/08-api-handlers.md) |
| T-P4-A1-3 设计数据存储 | [01. 数据层](../detailed-design/01-data-layer.md) |
| T-P4-A1-4 设计事务控制 | [01. 数据层 §1.8 事务边界](../detailed-design/01-data-layer.md#18-事务边界-transaction-boundaries) |
| T-P4-A1-5 设计错误处理方式 | [11. 错误处理](../detailed-design/11-error-handling.md) |
| T-P4-A1-6 设计安全架构 | [09. 安全实现](../detailed-design/09-security-impl.md) |
| T-P4-A1-7 设计可观测性 | [10. 可观测性](../detailed-design/10-observability.md) |

### C.4.2 P5 任务 → 详细设计章节 对应

| 任务 | 详细设计章节 |
|---|---|
| T-P5-A1-1 设计类/函数规格 | [02. 图谱引擎](../detailed-design/02-graph-engine.md), [03. 策略引擎](../detailed-design/03-policy-engine.md), [04. Agent 运行时](../detailed-design/04-agent-runtime.md), [05. AI 网关](../detailed-design/05-ai-gateway.md) |
| T-P5-A1-2 设计 DB Schema 详细 | [01. 数据层 §1.4 完整 DDL](../detailed-design/01-data-layer.md#14-完整-ddl) |
| T-P5-A1-3 设计 API Handler 详细 | [08. API 处理器](../detailed-design/08-api-handlers.md) |
| T-P5-A1-4 设计状态机 | [04. Agent 运行时 §4.4 状态机](../detailed-design/04-agent-runtime.md#44-状态机) |
| T-P5-A1-5 设计并发/排他控制 | [07. App 协调 §7.6 Outbox + Relay](../detailed-design/07-app-coordination.md#76-outbox-relay) |
| T-P5-A1-6 设计 PL/pgSQL 函数 | [12. App 群组信息互通设计 §存储过程 API 端点](../detailed-design/07-app-coordination.md#74-存储过程plpgsql调用) |

## C.5 P9 软件受入过程 对应

> **IPA 共通框架 2013 §3.9 软件受入过程 构成：** A1 验收测试的计划 / A2 实施验收测试 / A3 评价验收测试 / A4 验收测试的完结

| 任务 | 本书对应章节 |
|---|---|
| T-A1 验收测试的计划 | [10. 受入测试方针 §10.1 测试级别](10-acceptance-test-policy.md#101-测试级别-test-levels) |
| T-A2 实施验收测试 | [10. 受入测试方针 §10.2 MVP 验收条件](10-acceptance-test-policy.md#102-mvp-验收条件重申-phase-9-6-的-definition-of-done) |
| T-A3 评价验收测试 | [10. 受入测试方针 §10.4 发布判定](10-acceptance-test-policy.md#104-发布判定-release-decision) |
| T-A4 验收测试的完结 | [10. 受入测试方针 §10.4](10-acceptance-test-policy.md#104-发布判定-release-decision)（PO 正式签核 = Phase 14 F14-7 缺口） |

## C.6 IPA 共通框架 2013 以外的标准对应

| 标准 | 本书对应章节 |
|---|---|
| **非功能要求等级 2018 改订版** | [06. 非功能设计](06-non-functional-design.md)（6 大项分类直接对应 NFR Grade 6 大项） |
| **信息安全管理策** (源自 ISO/IEC 27001) | [07. 安全设计](07-security-design.md) §7.1-7.6 |
| **ISO/IEC 25010** (系统及软件质量模型) | [06. 非功能设计](06-non-functional-design.md)（8 质量特性与 6 大项交叉） |
| **W3C Trace Context** | [10. 可观测性 §10.9 跨进程传播](../detailed-design/10-observability.md#109-span-跨进程传播) |
| **OpenTelemetry** | [10. 可观测性](../detailed-design/10-observability.md) 全文 |
| **JSON Schema Draft 2020-12** | [02. 图谱引擎 §2.4 类型注册表](../detailed-design/02-graph-engine.md#24-类型注册表-type-registry) |
| **JWT (RFC 7519)** | [04. Agent 运行时 §4.7 凭据签发](../detailed-design/04-agent-runtime.md#47-凭据签发-credential-issuance) |
| **REST (RFC 9110)** | [08. API 处理器 §8.3 路由表](../detailed-design/08-api-handlers.md#83-路由表) |
| **OpenAPI 3.1** | [08. API 处理器 §8.10 OpenAPI 治理](../detailed-design/08-api-handlers.md#810-openapi-治理) |
| **CEL (Common Expression Language)** | [03. 策略引擎 §3.8 ABAC 属性求值](../detailed-design/03-policy-engine.md#38-abac-属性求值) |

## C.7 受入基准 (Acceptance Criteria) 一览

**[PROPOSAL]** 本书受入的基本设计完成度判据（对应 IPA A4 T4-3 评审）：

- [x] 全部 37 项 MVP 需求至少出现在一本书的章节中（直接对应 [§3 功能设计](03-functional-design.md) 或 [§6 非功能设计](06-non-functional-design.md)）
- [x] 全部 9 条 AISEC-REQ 在 [§3.7.4](03-functional-design.md#374-aisec-reqai-安全的整合) + [§7.3](07-security-design.md#73-ai-安全-ai-securityaisec-req) 出现
- [x] 全部 3 条 NFR-REQ 在 [§6.1-6.6](06-non-functional-design.md) 出现
- [x] 全部 6 条 SEC-REQ-008/009/010 + SEC-REQ-001-007 在 [§3.7](03-functional-design.md#37-安全与访问控制子系统-security-access-control-subsystem) + [§7](07-security-design.md) 出现
- [x] 全部 6 个子系统功能（Git/Graph/AI/Ctx/Agent/CI/Sec/API）在 [§3](03-functional-design.md) 出现
- [x] 至少 1 处数据 schema (PostgreSQL DDL) 在 [§4](04-data-design.md) 给出
- [x] 至少 1 处 API 契约 (HTTP) 在 [§11](11-api-design.md) 给出
- [x] 至少 1 处安全设计（信封加密）在 [§7.5](07-security-design.md#75-密钥管理-secrets-managementsec-req-005-sec-req-010) 给出
- [x] 至少 1 处迁移设计在 [§9](09-migration-design.md) 给出
- [x] 至少 1 处受入测试方针在 [§10](10-acceptance-test-policy.md) 给出
- [x] App 升格为一级对象（App / AppInstance / AppDeployment 节点化）在 [§13.1](13-app-cluster-and-plugins.md#131-概念模型app-作为一级平台对象) 给出
- [x] App Manifest schema 在 [§13.2](13-app-cluster-and-plugins.md#132-app-manifest-appyaml-schema) 给出
- [x] 中心事件总线在 [§13.4](13-app-cluster-and-plugins.md#134-中心事件总线-central-event-bus) 给出
- [x] App 集群健康心跳在 [§13.5](13-app-cluster-and-plugins.md#135-app-集群模型) 给出
- [x] 热插拔生命周期 + 升级策略在 [§13.6-13.7](13-app-cluster-and-plugins.md#136-热插拔与生命周期-hot-plug-lifecycle) 给出
- [x] Admin UI 独立子进程 + 独立鉴权域在 [§14.1 / §14.3](14-admin-ops-ui.md#141-部署形态-deployment-topology) 给出
- [x] admin_audit 不可篡改（哈希链）在 [§14.2.5](14-admin-ops-ui.md#1425-审计日志) 给出
- [x] App 沙箱 DB role 隔离（AISEC-REQ-013 强化）在 [§7.7](07-security-design.md#77-app-沙箱权限边界-app-sandbox-permission-boundary) 给出
- [x] Plugin API 端点族 `/api/apps/<app_id>/*` 在 [§11.12](11-api-design.md#1112-plugin-api-端点族-plugin-api-endpoint-family) 给出
- [x] V1+ K8s 部署形态在 [§8.6](08-operations-design.md#86-v1-k8s-部署形态-v1-k8s-deployment-topology) 给出
- [ ] 全部章节"人类签核"获取 — **F14-7 缺口**，标注 [TBD]

---

**导航 / Navigation:**
[← Appendix B — 残留 TBD](appendix-b-tbd.md) · [README](README.md) · [Appendix D — 用语集 →](appendix-d-glossary.md)
