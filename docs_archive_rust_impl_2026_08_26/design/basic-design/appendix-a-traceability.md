# Appendix A — 设计依据与需求追溯性矩阵 (Traceability Matrix)

> **依据框架 / Framework Compliance:** 严格按照 **IPA 共通框架 2013**（P3 系统方式设计过程 T-A3-1/T-A3-2 方式评价与追溯）编写。
> 
> 本附录分为两个部分：
> - **A.1 核心架构决策溯源 (Architectural Decisions Traceability)**：记录从需求调研 (Phase 1-15) 到基本设计 / 详细设计的关键架构决策演化。
> - **A.2 完整需求追溯矩阵 (Full Requirements Traceability Matrix - RTM)**：将需求定义书中的全部 126 项需求（FR / NFR / SEC / AISEC / OPS / CLOUD 等）全量映射到基本设计章节、详细设计章节及验证测试级别 (UT / IT / ST / UAT)。

---

## A.1 核心架构决策溯源

| 设计决策 | 起源 | 反映位置 |
|---|---|---|
| 5 原语（Node/Edge/Event/Policy/View）| Phase 6 | [§2.1 逻辑架构](02-architecture.md#21-逻辑架构-logical-architecture)、[§3.2 Engineering Graph 子系统](03-functional-design.md#32-工程图谱子系统-engineering-graph-subsystem)、[§4.1 概念数据模型](04-data-design.md#41-概念数据模型-conceptual-data-model) |
| 单一 PostgreSQL | Phase 10 §1 | [§2.4 存储架构](02-architecture.md#24-存储架构-storage-architecture采用-phase-10-1)、[§4.2 物理 schema](04-data-design.md#42-物理-schema-physical-schemapostgresql) |
| 混合 Git（CLI + gix/libgit2）| Phase 10 §3 | [§2.6 Git 存储实现](02-architecture.md#26-git-存储实现-git-storage-implementation采用-phase-10-3)、[详细设计 §06](../detailed-design/06-git-server.md) |
| 单一进程 + 4 个部署单位 | Phase 10 §4 | [§2.2 部署单位](02-architecture.md#22-部署单位-deployable-units来自-phase-10-4) |
| MVP 37 项需求 | Phase 9 §5 | [§3 功能设计](03-functional-design.md) 全文 |
| RBAC/ABAC + 结构化审计 | 需求 §35、Phase 11 R6/R7/R8 | [§3.7 安全与访问控制子系统](03-functional-design.md#37-安全与访问控制子系统-security-access-control-subsystem)、[§7.1 认证授权](07-security-design.md#71-认证-授权-authentication-authorization)、[§7.2 审计](07-security-design.md#72-审计-audit) |
| 三层信封加密 | SEC-REQ-005, SEC-REQ-010（Phase 14）| [§4.2 物理 schema](04-data-design.md#42-物理-schema-physical-schemapostgresql)、[§7.5 密钥管理](07-security-design.md#75-密钥管理-secrets-managementsec-req-005-sec-req-010) |
| mTLS + 网络隔离 | SEC-REQ-008（Phase 14 F14-3）| [§7.4 网络安全](07-security-design.md#74-网络安全-network-securitysec-req-008-phase-14-f14-3) |
| 持续漏洞管理 | SEC-REQ-009（Phase 14 F14-4）| [§6.3 运行与维护性](06-non-functional-design.md#63-运行与维护性operability-maintainability-ipa-grade-③)、[§6.5 安全性](06-non-functional-design.md#65-安全性security-ipa-grade-⑤) |
| DB 角色级 `events` 仅追加（红队最大发现防御）| AISEC-REQ-009(a)（Phase 11 RT-10）| [§7.2 审计](07-security-design.md#72-审计-audit)、[§3.7.4 AISEC-REQ 整合](03-functional-design.md#374-aisec-reqai-安全的整合) |
| Evidence 类型 Edge 的 `epistemic_status` | GRF-REQ-010 修正案（Phase 13）| [§3.2.3 V1 功能](03-functional-design.md#323-v1-功能)、[§4.2 物理 schema](04-data-design.md#42-物理-schema-physical-schemapostgresql) |
| 钩子回调的 fail-closed | GIT-REQ-006 修正案（Phase 13）| [§3.1.3 V1 功能](03-functional-design.md#313-v1-功能)、[§2.6 Git 存储实现](02-architecture.md#26-git-存储实现-git-storage-implementation采用-phase-10-3) |
| Provisional NFR 等级 | Phase 14 F14-1/F14-2/F14-6 | [§6.1 可用性](06-non-functional-design.md#61-可用性availability-ipa-grade-①)、[§6.2 性能与扩展性](06-non-functional-design.md#62-性能与扩展性performance-scalability-ipa-grade-②)、[§6.3 运行与维护性](06-non-functional-design.md#63-运行与维护性operability-maintainability-ipa-grade-③) |
| 系统环境与生态 | Phase 14 F14-5（NFR-REQ-003, V1-timed）| [§6.6 系统环境与生态](06-non-functional-design.md#66-系统环境与生态system-environment-ecology-ipa-grade-⑥)、[§8.1.3 支持的操作系统矩阵](08-operations-design.md#813-支持的操作系统矩阵nfr-req-003-v1-正式化) |
| Local = Cloud 架构 | CLOUD-REQ-001, Phase 10 §5 | [§2.3 物理架构](02-architecture.md#23-物理架构-physical-architecture)、[§9.3 架构一致性](09-migration-design.md#93-架构一致性-architecture-consistencycloud-req-001) |
| App 群组协调（存储过程 / Outbox / Saga）| 本书新增（[§12](12-app-group-intercommunication.md)）| §12 全部、[详细设计 §07](../detailed-design/07-app-coordination.md) |
| **主语言 Rust + 全套库选型** | 需求 §53 ADR 列表项 11 + phase10 §2 §6 §7 拍板 | [架构 / 技术选型文档](../../architecture/tech-selection.md) Accepted 2026-08-19；[详细设计 §0.3](../detailed-design/00-overview.md#03-语言与运行时-language-runtime) |
| **App 升格为一级对象** | 本书新增（[§13](13-app-cluster-and-plugins.md)）| [§13.1 概念模型](13-app-cluster-and-plugins.md#131-概念模型app-作为一级平台对象) |
| **App Manifest (app.yaml) schema** | 本书新增（[§13](13-app-cluster-and-plugins.md)）| [§13.2 Manifest schema](13-app-cluster-and-plugins.md#132-app-manifest-appyaml-schema)、[详细设计 §12.1](../detailed-design/12-app-registry-and-plugin-loader.md) |
| **中心事件总线（PG LISTEN/NOTIFY + Outbox 扩展）**| 本书新增（[§13](13-app-cluster-and-plugins.md)）| [§13.4 中心事件总线](13-app-cluster-and-plugins.md#134-中心事件总线-central-event-bus)、[详细设计 §12.3](../detailed-design/12-app-registry-and-plugin-loader.md#123-中心事件总线实现) |
| **App 集群健康心跳** | 本书新增（[§13](13-app-cluster-and-plugins.md)）| [§13.5 App 集群模型](13-app-cluster-and-plugins.md#135-app-集群模型)、[详细设计 §12.2.4](../detailed-design/12-app-registry-and-plugin-loader.md#1224-心跳与健康检查) |
| **App 热插拔（rolling / blue-green / canary）** | 本书新增（[§13](13-app-cluster-and-plugins.md)）| [§13.6-13.7 热插拔与升级](13-app-cluster-and-plugins.md#136-热插拔与生命周期-hot-plug-lifecycle)、[详细设计 §12.2](../detailed-design/12-app-registry-and-plugin-loader.md) |
| **Admin UI 独立子进程 + 独立鉴权域** | 本书新增（[§14](14-admin-ops-ui.md)）| [§14.1 / §14.3](14-admin-ops-ui.md#141-部署形态-deployment-topology)、[详细设计 §13.2](../detailed-design/13-admin-api-and-ops-ui.md#132-鉴权中间件) |
| **admin_audit 不可篡改（哈希链 + SIEM 转发）**| 本书新增（[§14](14-admin-ops-ui.md)）| [§14.2.5 审计日志](14-admin-ops-ui.md#1425-审计日志)、[详细设计 §13.3](../detailed-design/13-admin-api-and-ops-ui.md#133-admin_audit-表-ddl) |
| **App 沙箱 DB role 隔离（AISEC-REQ-013 强化）** | AISEC-REQ-009(a) 强化 | [§7.7 App 沙箱权限边界](07-security-design.md#77-app-沙箱权限边界-app-sandbox-permission-boundary)、[详细设计 §12.1.9 / §12.4](../detailed-design/12-app-registry-and-plugin-loader.md) |
| **跨 App 存储过程调用中介（coord_call_app_proc）**| 本书新增（[§13](13-app-cluster-and-plugins.md)）| [详细设计 §07.4.4](../detailed-design/07-app-coordination.md#744-app-命名空间与跨-app-存储过程app-cluster-plugin-扩展) |
| **Plugin API 端点族 (/api/apps/<app_id>/*)** | 本书新增（[§11](11-api-design.md)）| [§11.12 Plugin API 端点族](11-api-design.md#1112-plugin-api-端点族-plugin-api-endpoint-family) |
| **V1+ K8s 部署形态（App Pod 化）**| 本书新增（[§8](08-operations-design.md)）| [§8.6 V1+ K8s 部署形态](08-operations-design.md#86-v1-k8s-部署形态-v1-k8s-deployment-topology)、[详细设计 §13.5](../detailed-design/13-admin-api-and-ops-ui.md#135-v1-k8s-部署清单) |

---

## A.2 完整需求追溯矩阵 (Requirements Traceability Matrix - RTM)

> **测试级别说明 (IPA P7-P9):**
> - **UT**: 单体测试 (Unit Test)
> - **IT**: 结合/集成测试 (Integration Test)
> - **ST**: 系统测试 (System Test)
> - **UAT**: 验收测试 (User Acceptance Test)

| 需求 ID | 需求概要 | 需求定义位置 | 基本设计对应 | 详细设计对应 | 测试级别 |
|---|---|---|---|---|---|
| **GIT-REQ-001** | Git Smart HTTP & SSH Transport | 需求 §17, §38 | [基本设计 §03.1](03-functional-design.md#31-git-server-子系统-git-server-subsystem) | [详细设计 §06.2](../detailed-design/06-git-server.md#62-模块结构) | IT, ST |
| **GIT-REQ-002** | Bare Repository Storage & Layout | 需求 §17, §38 | [基本设计 §02.6](02-architecture.md#26-git-存储实现-git-storage-implementation采用-phase-10-3) | [详细设计 §06.4](../detailed-design/06-git-server.md#64-读路径libgit2) | UT, IT |
| **GIT-REQ-003** | Commit / Tree / Blob Object Reads | 需求 §17, §38 | [基本设计 §03.1](03-functional-design.md#31-git-server-子系统-git-server-subsystem) | [详细设计 §06.3](../detailed-design/06-git-server.md#63-写路径cli-子进程) | UT, IT |
| **GIT-REQ-004** | Branch / Tag Ref Updates & Leases | 需求 §17, §38 | [基本设计 §03.1](03-functional-design.md#31-git-server-子系统-git-server-subsystem) | [详细设计 §06.4](../detailed-design/06-git-server.md#64-读路径libgit2) | IT, ST |
| **GIT-REQ-005** | Pre-receive / Post-receive Hooks | 需求 §17, §38 | [基本设计 §03.1](03-functional-design.md#31-git-server-子系统-git-server-subsystem) | [详细设计 §06.5](../detailed-design/06-git-server.md#65-graph-aware-钩子) | IT, ST |
| **GIT-REQ-006** | Fail-Closed Policy Enforcement on Push | 需求 §17, §38 | [基本设计 §03.1.3](03-functional-design.md#313-v1-功能) | [详细设计 §06.5](../detailed-design/06-git-server.md#65-graph-aware-钩子) | IT, ST |
| **GIT-REQ-007** | Git LFS Support | 需求 §17, §38 | [基本设计 §03.1.3](03-functional-design.md#313-v1-功能) | [详细设计 §06.2](../detailed-design/06-git-server.md#62-模块结构) | IT |
| **GIT-REQ-008** | Commit Signature Verification (GPG/SSH) | 需求 §17, §38 | [基本设计 §07.1](07-security-design.md#71-认证-授权-authentication-authorization) | [详细设计 §09.2](../detailed-design/09-security-impl.md) | UT, IT |
| **GIT-REQ-009** | Shallow / Partial Clone & Sparse Checkout | 需求 §17, §38 | [基本设计 §03.1.3](03-functional-design.md#313-v1-功能) | [详细设计 §06.3](../detailed-design/06-git-server.md#63-写路径cli-子进程) | IT |
| **GIT-REQ-010** | Storage Resiliency & Crash Recovery | 需求 §17, §38 | [基本设计 §06.1](06-non-functional-design.md#61-可用性availability-ipa-grade-①) | [详细设计 §06.4](../detailed-design/06-git-server.md#64-读路径libgit2) | ST |
| **GIT-REQ-011** | Git Garbage Collection (pack/prune) | 需求 §17, §38 | [基本设计 §08.1](08-operations-design.md#81-部署-deployment) | [详细设计 §06.4](../detailed-design/06-git-server.md#64-读路径libgit2) | IT, OT |
| **GRF-REQ-001** | 5 Core Primitives (Node/Edge/Event/Policy/View) | 需求 §25, §39 | [基本设计 §03.2](03-functional-design.md#32-工程图谱子系统-engineering-graph-subsystem) | [详细设计 §02.2](../detailed-design/02-graph-engine.md#22-模块结构-module-layout) | UT, IT |
| **GRF-REQ-002** | Type Registry & Schema Validation | 需求 §25, §39 | [基本设计 §03.2](03-functional-design.md#32-工程图谱子系统-engineering-graph-subsystem) | [详细设计 §02.4](../detailed-design/02-graph-engine.md#24-类型注册表-type-registry) | UT, IT |
| **GRF-REQ-003** | Graph Traversal (Recursive CTE) | 需求 §25, §39 | [基本设计 §04.3](04-data-design.md#43-图遍历-graph-traversal) | [详细设计 §02.7](../detailed-design/02-graph-engine.md#27-图遍历) | UT, IT |
| **GRF-REQ-004** | Node & Edge Immutability / Versioning | 需求 §25, §39 | [基本设计 §04.2](04-data-design.md#42-物理-schema-physical-schemapostgresql) | [详细设计 §01.4](../detailed-design/01-data-layer.md#14-完整-ddl) | UT, IT |
| **GRF-REQ-005** | Graph Projection & Views | 需求 §25, §39 | [基本设计 §03.2](03-functional-design.md#32-工程图谱子系统-engineering-graph-subsystem) | [详细设计 §02.6](../detailed-design/02-graph-engine.md) | UT, IT |
| **GRF-REQ-006** | Graph-Driven Event Emission | 需求 §25, §39 | [基本设计 §03.2](03-functional-design.md#32-工程图谱子系统-engineering-graph-subsystem) | [详细设计 §02.5](../detailed-design/02-graph-engine.md) | IT, ST |
| **GRF-REQ-007** | High-Throughput Node Ingestion | 需求 §25, §39 | [基本设计 §06.2](06-non-functional-design.md#62-性能与扩展性performance-scalability-ipa-grade-②) | [详细设计 §01.5](../detailed-design/01-data-layer.md) | ST, PT |
| **GRF-REQ-008** | In-Process Graph Caching | 需求 §25, §39 | [基本设计 §03.2](03-functional-design.md#32-工程图谱子系统-engineering-graph-subsystem) | [详细设计 §02.9](../detailed-design/02-graph-engine.md#29-缓存-caching) | UT, ST |
| **GRF-REQ-009** | Multi-Branch Graph Linkage | 需求 §25, §39 | [基本设计 §03.2](03-functional-design.md#32-工程图谱子系统-engineering-graph-subsystem) | [详细设计 §02.7](../detailed-design/02-graph-engine.md#27-图遍历) | IT, ST |
| **GRF-REQ-010** | Epistemic Status for Evidence Edges | 需求 §25, §39 | [基本设计 §03.2.3](03-functional-design.md#323-v1-功能) | [详细设计 §01.4](../detailed-design/01-data-layer.md#14-完整-ddl) | UT, IT |
| **GRF-REQ-011** | Graph Query DSL / Filter Syntax | 需求 §25, §39 | [基本设计 §03.2](03-functional-design.md#32-工程图谱子系统-engineering-graph-subsystem) | [详细设计 §02.8](../detailed-design/02-graph-engine.md) | UT, IT |
| **AGT-REQ-001** | Agent Lifecycle State Machine | 需求 §28, §42 | [基本设计 §03.5](03-functional-design.md#35-agent-运行时子系统-agent-runtime-subsystem) | [详细设计 §04.4](../detailed-design/04-agent-runtime.md#44-状态机) | UT, IT |
| **AGT-REQ-002** | Human Approve/Reject Gate | 需求 §28, §42 | [基本设计 §03.5](03-functional-design.md#35-agent-运行时子系统-agent-runtime-subsystem) | [详细设计 §04.5](../detailed-design/04-agent-runtime.md) | IT, UAT |
| **AGT-REQ-003** | Agent Messaging & Status Interface | 需求 §28, §42 | [基本设计 §03.5](03-functional-design.md#35-agent-运行时子系统-agent-runtime-subsystem) | [详细设计 §04.8](../detailed-design/04-agent-runtime.md) | IT, ST |
| **AGT-REQ-004** | Agent Artifact Capture | 需求 §28, §42 | [基本设计 §03.5](03-functional-design.md#35-agent-运行时子系统-agent-runtime-subsystem) | [详细设计 §04.9](../detailed-design/04-agent-runtime.md) | UT, IT |
| **AGT-REQ-005** | Workspace Isolation (Container/Sandbox) | 需求 §15, §28 | [基本设计 §02.3](02-architecture.md#23-物理架构-physical-architecture) | [详细设计 §04.6](../detailed-design/04-agent-runtime.md#46-工作区隔离-workspace-isolation) | IT, ST |
| **AGT-REQ-006** | Resource Limits on Agent Execution | 需求 §28, §42 | [基本设计 §03.5](03-functional-design.md#35-agent-运行时子系统-agent-runtime-subsystem) | [详细设计 §04.6](../detailed-design/04-agent-runtime.md#46-工作区隔离-workspace-isolation) | ST |
| **AGT-REQ-007** | Agent Node Graph Integration | 需求 §13, §28 | [基本设计 §03.5](03-functional-design.md#35-agent-运行时子系统-agent-runtime-subsystem) | [详细设计 §04.9](../detailed-design/04-agent-runtime.md) | IT, ST |
| **AGT-REQ-008** | Multi-Agent Coordination via Graph | 需求 §10, §28 | [基本设计 §03.5](03-functional-design.md#35-agent-运行时子系统-agent-runtime-subsystem) | [详细设计 §04.8](../detailed-design/04-agent-runtime.md) | ST |
| **AGT-REQ-009** | Subagent / Delegated-Scope Execution | 需求 §28, §42 | [基本设计 §03.5.3](03-functional-design.md#353-v1-功能) | [详细设计 §04.7](../detailed-design/04-agent-runtime.md#47-凭据签发-credential-issuance) | IT, ST |
| **AI-REQ-001** | Unified AI Gateway Abstraction | 需求 §13, §27 | [基本设计 §03.3](03-functional-design.md#33-ai-网关子系统-ai-gateway-subsystem) | [详细设计 §05.2](../detailed-design/05-ai-gateway.md#54-provider-接口) | UT, IT |
| **AI-REQ-002** | Cost and Token Observability per Request | 需求 §27, §41 | [基本设计 §03.3](03-functional-design.md#33-ai-网关子系统-ai-gateway-subsystem) | [详细设计 §05.5](../detailed-design/05-ai-gateway.md) | IT, ST |
| **AI-REQ-003** | Data Sensitivity Routing Policy | 需求 §27, §41 | [基本设计 §03.3](03-functional-design.md#33-ai-网关子系统-ai-gateway-subsystem) | [详细设计 §05.3](../detailed-design/05-ai-gateway.md) | IT, ST |
| **AI-REQ-004** | Provider Fallback & Degradation | 需求 §27, §41 | [基本设计 §03.3](03-functional-design.md#33-ai-网关子系统-ai-gateway-subsystem) | [详细设计 §05.4](../detailed-design/05-ai-gateway.md) | IT, ST |
| **AI-REQ-005** | Model Swap without Code Changes | 需求 §27, §41 | [基本设计 §03.3](03-functional-design.md#33-ai-网关子系统-ai-gateway-subsystem) | [详细设计 §05.2](../detailed-design/05-ai-gateway.md#54-provider-接口) | IT, UAT |
| **AI-REQ-006** | Prompt & Response Caching | 需求 §27, §41 | [基本设计 §03.3.3](03-functional-design.md#333-v1-功能) | [详细设计 §05.6](../detailed-design/05-ai-gateway.md) | UT, ST |
| **AISEC-REQ-001** | Prompt Injection Containment | 需求 §36 | [基本设计 §07.3](07-security-design.md#73-ai-安全-ai-securityaisec-req) | [详细设计 §05.7](../detailed-design/05-ai-gateway.md#57-提示词清洗-aisec-req-001) | IT, ST |
| **AISEC-REQ-002** | MCP Tool/Server Allowlisting | 需求 §36 | [基本设计 §07.3](07-security-design.md#73-ai-安全-ai-securityaisec-req) | [详细设计 §03.6](../detailed-design/03-policy-engine.md#39-ai-特有策略-ai-specific-policies) | IT, ST |
| **AISEC-REQ-003** | MCP Tool Description Integrity | 需求 §36 | [基本设计 §07.3](07-security-design.md#73-ai-安全-ai-securityaisec-req) | [详细设计 §08.6](../detailed-design/08-api-handlers.md#88-mcp-tool-实现) | UT, IT |
| **AISEC-REQ-004** | Secret Exclusion from AI Payloads | 需求 §36 | [基本设计 §07.3](07-security-design.md#73-ai-安全-ai-securityaisec-req) | [详细设计 §05.7](../detailed-design/05-ai-gateway.md#57-提示词清洗-aisec-req-001) | UT, IT |
| **AISEC-REQ-005** | Agent Privilege Escalation Prevention | 需求 §36 | [基本设计 §07.3](07-security-design.md#73-ai-安全-ai-securityaisec-req) | [详细设计 §04.7](../detailed-design/04-agent-runtime.md#47-凭据签发-credential-issuance) | IT, ST |
| **AISEC-REQ-006** | Agent Output Policy Bypass Defense | 需求 §36 | [基本设计 §07.3](07-security-design.md#73-ai-安全-ai-securityaisec-req) | [详细设计 §03.6](../detailed-design/03-policy-engine.md#39-ai-特有策略-ai-specific-policies) | IT, ST |
| **AISEC-REQ-007** | Rate-Limited & Anomaly-Flagged Behavior | 需求 §36 | [基本设计 §07.3](07-security-design.md#73-ai-安全-ai-securityaisec-req) | [详细设计 §04.6](../detailed-design/04-agent-runtime.md#46-工作区隔离-workspace-isolation) | ST |
| **AISEC-REQ-008** | External MCP Server Sandboxing | 需求 §36 | [基本设计 §07.3](07-security-design.md#73-ai-安全-ai-securityaisec-req) | [详细设计 §12.4](../detailed-design/12-app-registry-and-plugin-loader.md#124-沙箱执行环境) | IT, ST |
| **AISEC-REQ-009** | DB Role Separation (events Append-Only) | 需求 §35, Phase 11 | [基本设计 §07.2](07-security-design.md#72-审计-audit) | [详细设计 §01.3](../detailed-design/01-data-layer.md#13-db-role-分离aisec-req-009a-mvp-必填) | UT, IT, ST |
| **API-REQ-001** | Single Documented API Surface | 需求 §47 | [基本设计 §11.1](11-api-design.md#112-协议栈总览) | [详细设计 §08.3](../detailed-design/08-api-handlers.md#83-路由表) | IT, ST |
| **API-REQ-002** | Webhook Delivery for Graph Events | 需求 §47 | [基本设计 §11.6](11-api-design.md#116-webhook-outbound-api) | [详细设计 §08.8](../detailed-design/08-api-handlers.md#88-mcp-tool-实现) | IT, ST |
| **API-REQ-003** | Policy-Gated & Audited API Access | 需求 §47 | [基本设计 §07.1](07-security-design.md#71-认证-授权-authentication-authorization) | [详细设计 §08.4](../detailed-design/08-api-handlers.md) | IT, ST |
| **API-REQ-004** | API Rate Limiting & Abuse Protection | 需求 §47 | [基本设计 §06.2](06-non-functional-design.md#62-性能与扩展性performance-scalability-ipa-grade-②) | [详细设计 §08.4](../detailed-design/08-api-handlers.md) | ST |
| **API-REQ-005** | Curated MCP Tool Surface | 需求 §47 | [基本设计 §11.4](11-api-design.md#114-mcp-设计) | [详细设计 §08.6](../detailed-design/08-api-handlers.md#88-mcp-tool-实现) | IT, ST |
| **ART-REQ-001** | Artifact Storage & Versioning | 需求 §32 | [基本设计 §03.2](03-functional-design.md#32-工程图谱子系统-engineering-graph-subsystem) | [详细设计 §01.4](../detailed-design/01-data-layer.md#14-完整-ddl) | UT, IT |
| **ART-REQ-002** | Content Addressing & Integrity Hashing | 需求 §32 | [基本设计 §04.2](04-data-design.md#42-物理-schema-physical-schemapostgresql) | [详细设计 §01.4](../detailed-design/01-data-layer.md#14-完整-ddl) | UT, IT |
| **ART-REQ-003** | Artifact Access Authorization | 需求 §32 | [基本设计 §07.1](07-security-design.md#71-认证-授权-authentication-authorization) | [详细设计 §03.5](../detailed-design/03-policy-engine.md) | IT, ST |
| **BKP-REQ-001** | Local & Cloud Database Backup | 需求 §48 | [基本设计 §08.2](08-operations-design.md#82-备份-恢复-backup-recoverybkp-req) | [详细设计 §01.7](../detailed-design/01-data-layer.md) | ST, OT |
| **BKP-REQ-002** | Point-in-Time Recovery (PITR) | 需求 §48 | [基本设计 §08.2](08-operations-design.md#82-备份-恢复-backup-recoverybkp-req) | [详细设计 §01.7](../detailed-design/01-data-layer.md) | ST, OT |
| **BKP-REQ-003** | Backup Verification & Health Test | 需求 §48 | [基本设计 §08.2](08-operations-design.md#82-备份-恢复-backup-recoverybkp-req) | [详细设计 §01.7](../detailed-design/01-data-layer.md) | ST, OT |
| **CDX-REQ-001** | Codebase Symbol & Reference Graph | 需求 §44 | [基本设计 §03.2](03-functional-design.md#32-工程图谱子系统-engineering-graph-subsystem) | [详细设计 §02.4](../detailed-design/02-graph-engine.md#24-类型注册表-type-registry) | UT, IT |
| **CDX-REQ-002** | Incremental Indexing on Push | 需求 §44 | [基本设计 §03.1](03-functional-design.md#31-git-server-子系统-git-server-subsystem) | [详细设计 §06.5](../detailed-design/06-git-server.md#65-graph-aware-钩子) | IT, ST |
| **CDX-REQ-003** | AST-Level Dependency Extraction | 需求 §44 | [基本设计 §03.4](03-functional-design.md#34-上下文引擎子系统-context-engine-subsystem) | [详细设计 §02.7](../detailed-design/02-graph-engine.md#27-图遍历) | UT, IT |
| **CDX-REQ-004** | Hybrid Semantic / Lexical Search | 需求 §44 | [基本设计 §03.4.3](03-functional-design.md#343-v1-功能) | [详细设计 §05.6](../detailed-design/05-ai-gateway.md) | IT, ST |
| **CI-REQ-001** | Ephemeral Runner Execution | 需求 §29, §43 | [基本设计 §03.6](03-functional-design.md#36-cicd-子系统-cicd-subsystem) | [详细设计 §04.6](../detailed-design/04-agent-runtime.md#46-工作区隔离-workspace-isolation) | IT, ST |
| **CI-REQ-002** | Graph Event Triggered Workflows | 需求 §29, §43 | [基本设计 §03.6](03-functional-design.md#36-cicd-子系统-cicd-subsystem) | [详细设计 §02.5](../detailed-design/02-graph-engine.md) | IT, ST |
| **CI-REQ-003** | Run Isolation & Secret Masking | 需求 §29, §43 | [基本设计 §07.5](07-security-design.md#75-密钥管理-secrets-managementsec-req-005-sec-req-010) | [详细设计 §04.7](../detailed-design/04-agent-runtime.md#47-凭据签发-credential-issuance) | IT, ST |
| **CI-REQ-004** | Workflow Status & Artifact Graph Nodes | 需求 §29, §43 | [基本设计 §03.6](03-functional-design.md#36-cicd-子系统-cicd-subsystem) | [详细设计 §04.9](../detailed-design/04-agent-runtime.md) | IT, ST |
| **CI-REQ-005** | Caching & Fast Feedback | 需求 §29, §43 | [基本设计 §06.2](06-non-functional-design.md#62-性能与扩展性performance-scalability-ipa-grade-②) | [详细设计 §04.6](../detailed-design/04-agent-runtime.md#46-工作区隔离-workspace-isolation) | ST |
| **CI-REQ-006** | Self-Hosted Runner Registration | 需求 §29, §43 | [基本设计 §03.6.3](03-functional-design.md#363-v1-功能) | [详细设计 §04.6](../detailed-design/04-agent-runtime.md#46-工作区隔离-workspace-isolation) | IT, ST |
| **CLOUD-REQ-001** | Zero Incompatible Feature Drift (Local = Cloud) | 需求 §37, §50 | [基本设计 §02.3](02-architecture.md#23-物理架构-physical-architecture) | [详细设计 §00.2](../detailed-design/00-overview.md#02-顶层模块划分) | ST, UAT |
| **CLOUD-REQ-002** | Local-to-Cloud Migration Path | 需求 §37, §50 | [基本设计 §09.1](09-migration-design.md#91-迁移场景-migration-scenarios) | [详细设计 §01.7](../detailed-design/01-data-layer.md) | ST, UAT |
| **CLOUD-REQ-003** | Cloud Multi-Tenant DB Schema & RLS | 需求 §37, §50 | [基本设计 §04.2](04-data-design.md#42-物理-schema-physical-schemapostgresql) | [详细设计 §01.6](../detailed-design/01-data-layer.md#16-行级安全-rls) | IT, ST |
| **CLOUD-REQ-004** | Cloud KMS / OIDC Provider Integration | 需求 §37, §50 | [基本设计 §07.5](07-security-design.md#75-密钥管理-secrets-managementsec-req-005-sec-req-010) | [详细设计 §09.3](../detailed-design/09-security-impl.md) | IT, ST |
| **CTX-REQ-001** | Graph-Driven Relevant Context Assembly | 需求 §26, §40 | [基本设计 §03.4](03-functional-design.md#34-上下文引擎子系统-context-engine-subsystem) | [详细设计 §02.7](../detailed-design/02-graph-engine.md#27-图遍历) | UT, IT |
| **CTX-REQ-002** | Token Budget Optimization for Prompts | 需求 §26, §40 | [基本设计 §03.4](03-functional-design.md#34-上下文引擎子系统-context-engine-subsystem) | [详细设计 §05.5](../detailed-design/05-ai-gateway.md) | UT, IT |
| **CTX-REQ-003** | Differential Context on Incremental Edits | 需求 §26, §40 | [基本设计 §03.4](03-functional-design.md#34-上下文引擎子系统-context-engine-subsystem) | [详细设计 §02.7](../detailed-design/02-graph-engine.md#27-图遍历) | UT, IT |
| **CTX-REQ-004** | Policy-Aware Context Redaction | 需求 §26, §40 | [基本设计 §07.3](07-security-design.md#73-ai-安全-ai-securityaisec-req) | [详细设计 §05.7](../detailed-design/05-ai-gateway.md#57-提示词清洗-aisec-req-001) | IT, ST |
| **CTX-REQ-005** | Dynamic Context Window Sizing | 需求 §26, §40 | [基本设计 §03.4.3](03-functional-design.md#343-v1-功能) | [详细设计 §05.5](../detailed-design/05-ai-gateway.md) | UT, IT |
| **DATA-REQ-001** | Full Data Export via Standard Formats | 需求 §46 | [基本设计 §04.5](04-data-design.md#45-数据生命周期-data-lifecycle) | [详细设计 §01.7](../detailed-design/01-data-layer.md) | ST, UAT |
| **DATA-REQ-002** | Deterministic Export/Import Roundtrip | 需求 §46 | [基本设计 §09.2](09-migration-design.md#92-迁移步骤-migration-steps以-lc-1-为例) | [详细设计 §01.7](../detailed-design/01-data-layer.md) | ST, UAT |
| **DATA-REQ-003** | Data Deletion & Retention Lifecycle | 需求 §46 | [基本设计 §04.5](04-data-design.md#45-数据生命周期-data-lifecycle) | [详细设计 §01.7](../detailed-design/01-data-layer.md) | IT, ST |
| **DATA-REQ-004** | Audit Trail Export Portability | 需求 §46 | [基本设计 §07.2](07-security-design.md#72-审计-audit) | [详细设计 §13.3](../detailed-design/13-admin-api-and-ops-ui.md#133-admin_audit-表-ddl) | IT, ST |
| **DATA-REQ-005** | Schema Evolution & Migration Safety | 需求 §46 | [基本设计 §04.4](04-data-design.md) | [详细设计 §01.4](../detailed-design/01-data-layer.md#14-完整-ddl) | UT, IT |
| **DIFF-REQ-001** | Unified Graph & Git Diff Engine | 需求 §30 | [基本设计 §03.1](03-functional-design.md#31-git-server-子系统-git-server-subsystem) | [详细设计 §02.7](../detailed-design/02-graph-engine.md#27-图遍历) | UT, IT |
| **DIFF-REQ-002** | Semantic Diff for Requirements / ADRs | 需求 §30 | [基本设计 §03.2](03-functional-design.md#32-工程图谱子系统-engineering-graph-subsystem) | [详细设计 §02.7](../detailed-design/02-graph-engine.md#27-图遍历) | UT, IT |
| **DTWIN-REQ-001** | Engineering Knowledge Graph Digital Twin | 需求 §45 | [基本设计 §03.2](03-functional-design.md#32-工程图谱子系统-engineering-graph-subsystem) | [详细设计 §02.2](../detailed-design/02-graph-engine.md#22-模块结构-module-layout) | ST, UAT |
| **DTWIN-REQ-002** | Architecture Drift Detection | 需求 §45 | [基本设计 §03.2.3](03-functional-design.md#323-v1-功能) | [详细设计 §02.7](../detailed-design/02-graph-engine.md#27-图遍历) | ST |
| **DTWIN-REQ-003** | Impact Analysis via Graph Traversal | 需求 §45 | [基本设计 §03.2.3](03-functional-design.md#323-v1-功能) | [详细设计 §02.7](../detailed-design/02-graph-engine.md#27-图遍历) | ST |
| **ITC-REQ-001** | Cross-App Graph Event Coordination | 需求 §33, §47 | [基本设计 §12.2](12-app-group-intercommunication.md) | [详细设计 §07.2](../detailed-design/07-app-coordination.md) | IT, ST |
| **ITC-REQ-002** | Central Event Bus & App Manifest Coordination | 需求 §33, §47 | [基本设计 §13.4](13-app-cluster-and-plugins.md#134-中心事件总线-central-event-bus) | [详细设计 §12.3](../detailed-design/12-app-registry-and-plugin-loader.md#123-中心事件总线实现) | IT, ST |
| **NFR-REQ-001** | Availability & Disaster Recovery (Provisional) | 需求 §35, Phase 14 | [基本设计 §06.1](06-non-functional-design.md#61-可用性availability-ipa-grade-①) | [详细设计 §01.7](../detailed-design/01-data-layer.md) | ST |
| **NFR-REQ-002** | Performance & Operability (Provisional) | 需求 §35, Phase 14 | [基本设计 §06.2-6.3](06-non-functional-design.md#62-性能与扩展性performance-scalability-ipa-grade-②) | [详细设计 §10.4](../detailed-design/10-observability.md) | ST, PT |
| **NFR-REQ-003** | System Environment Matrix (V1-timed) | 需求 §35, Phase 14 | [基本设计 §06.6](06-non-functional-design.md#66-系统环境与生态system-environment-ecology-ipa-grade-⑥) | [详细设计 §00.3](../detailed-design/00-overview.md#03-语言与运行时-language-runtime) | ST, UAT |
| **OBS-REQ-001** | OpenTelemetry Structured Tracing | 需求 §49 | [基本设计 §08.3](08-operations-design.md#83-监控-monitoringobs-req-v1) | [详细设计 §10.3](../detailed-design/10-observability.md#103-tracer-初始化) | IT, ST |
| **OBS-REQ-002** | Prometheus Metrics Export | 需求 §49 | [基本设计 §08.3](08-operations-design.md#83-监控-monitoringobs-req-v1) | [详细设计 §10.4](../detailed-design/10-observability.md#105-指标目录) | IT, ST |
| **OBS-REQ-003** | Structured JSON Logging & Correlation ID | 需求 §49 | [基本设计 §08.4](08-operations-design.md#84-日志-logging) | [详细设计 §10.6](../detailed-design/10-observability.md#106-结构化日志) | UT, IT |
| **OPS-REQ-001** | Single-Binary Local Deployment & Self-Check | 需求 §37, §48 | [基本设计 §08.1](08-operations-design.md#81-部署-deployment) | [详细设计 §00.11](../detailed-design/00-overview.md#011-部署包-deployment-artifacts) | ST, OT |
| **OPS-REQ-002** | Automated DB Migrations at Startup | 需求 §37, §48 | [基本设计 §08.1](08-operations-design.md#81-部署-deployment) | [详细设计 §01.4](../detailed-design/01-data-layer.md#14-完整-ddl) | UT, IT |
| **OPS-REQ-003** | Health & Readiness Probe Endpoints | 需求 §37, §48 | [基本设计 §11.2](11-api-design.md) | [详细设计 §08.3](../detailed-design/08-api-handlers.md#83-路由表) | UT, IT |
| **OPS-REQ-004** | Admin Ops UI & Incident Monitoring | 需求 §37, §48 | [基本设计 §14.2](14-admin-ops-ui.md#142-关键页面与交互-key-pages-interactions) | [详细设计 §13.4](../detailed-design/13-admin-api-and-ops-ui.md#134-ops-ui-前端架构) | ST, UAT |
| **OPS-REQ-005** | Graceful Shutdown & Drain Support | 需求 §37, §48 | [基本设计 §08.1](08-operations-design.md#81-部署-deployment) | [详细设计 §00.7](../detailed-design/00-overview.md#07-并发模型-concurrency-model) | IT, ST |
| **REL-REQ-001** | Release & Tag Node Creation | 需求 §31 | [基本设计 §03.2](03-functional-design.md#32-工程图谱子系统-engineering-graph-subsystem) | [详细设计 §02.4](../detailed-design/02-graph-engine.md#24-类型注册表-type-registry) | UT, IT |
| **REL-REQ-002** | Release Gate Evaluation via Policy | 需求 §31 | [基本设计 §10.4](10-acceptance-test-policy.md#104-发布判定-release-decision) | [详细设计 §03.5](../detailed-design/03-policy-engine.md) | IT, ST |
| **REL-REQ-003** | Automated Changelog Generation from Graph | 需求 §31 | [基本设计 §03.2.3](03-functional-design.md#323-v1-功能) | [详细设计 §02.7](../detailed-design/02-graph-engine.md#27-图遍历) | IT, ST |
| **SEC-REQ-001** | Human & Agent Identity Authentication | 需求 §35 | [基本设计 §07.1](07-security-design.md#71-认证-授权-authentication-authorization) | [详细设计 §09.2](../detailed-design/09-security-impl.md) | UT, IT |
| **SEC-REQ-002** | RBAC & ABAC Policy Engine Enforcement | 需求 §35 | [基本设计 §07.1](07-security-design.md#71-认证-授权-authentication-authorization) | [详细设计 §03.3](../detailed-design/03-policy-engine.md#37-rbac-角色解析) | UT, IT, ST |
| **SEC-REQ-003** | Tamper-Resistant Structured Audit Log | 需求 §35 | [基本设计 §07.2](07-security-design.md#72-审计-audit) | [详细设计 §01.3](../detailed-design/01-data-layer.md#13-db-role-分离aisec-req-009a-mvp-必填) | IT, ST |
| **SEC-REQ-004** | Short-Lived Scoped Agent Credentials | 需求 §35 | [基本设计 §07.1](07-security-design.md#71-认证-授权-authentication-authorization) | [详细设计 §04.7](../detailed-design/04-agent-runtime.md#47-凭据签发-credential-issuance) | UT, IT |
| **SEC-REQ-005** | Three-Layer Envelope Encryption (KEK/DEK) | 需求 §35 | [基本设计 §07.5](07-security-design.md#75-密钥管理-secrets-managementsec-req-005-sec-req-010) | [详细设计 §09.2](../detailed-design/09-security-impl.md#93-信封加密实现) | UT, IT |
| **SEC-REQ-006** | Ephemeral Workspace Network Isolation | 需求 §35 | [基本设计 §07.4](07-security-design.md#74-网络安全-network-securitysec-req-008-phase-14-f14-3) | [详细设计 §04.6](../detailed-design/04-agent-runtime.md#46-工作区隔离-workspace-isolation) | IT, ST |
| **SEC-REQ-007** | Admin & Security Operations Segregation | 需求 §35 | [基本设计 §07.8](07-security-design.md#78-admin-独立鉴权域-admin-separate-auth-domain) | [详细设计 §13.2](../detailed-design/13-admin-api-and-ops-ui.md#132-鉴权中间件) | IT, ST |
| **SEC-REQ-008** | mTLS Inter-Service & App Sandbox Network | 需求 §35, Phase 14 | [基本设计 §07.4](07-security-design.md#74-网络安全-network-securitysec-req-008-phase-14-f14-3) | [详细设计 §09.4](../detailed-design/09-security-impl.md) | IT, ST |
| **SEC-REQ-009** | Continuous Vulnerability Management | 需求 §35, Phase 14 | [基本设计 §06.3](06-non-functional-design.md#63-运行与维护性operability-maintainability-ipa-grade-③) | [详细设计 §00.8](../detailed-design/00-overview.md#08-测试约定-testing-convention) | ST, CI |
| **SEC-REQ-010** | Hardware / KMS Key Rotation & KEK Re-wrap | 需求 §35, Phase 14 | [基本设计 §07.5](07-security-design.md#75-密钥管理-secrets-managementsec-req-005-sec-req-010) | [详细设计 §09.2](../detailed-design/09-security-impl.md#93-信封加密实现) | IT, ST |
| **SRCH-REQ-001** | Unified Multi-Primitive Search Engine | 需求 §44 | [基本设计 §03.2](03-functional-design.md#32-工程图谱子系统-engineering-graph-subsystem) | [详细设计 §02.8](../detailed-design/02-graph-engine.md) | UT, IT |
| **SRCH-REQ-002** | Lexical Full-Text Search (PostgreSQL tsvector) | 需求 §44 | [基本设计 §04.2](04-data-design.md#42-物理-schema-physical-schemapostgresql) | [详细设计 §01.4](../detailed-design/01-data-layer.md#14-完整-ddl) | UT, IT |
| **SRCH-REQ-003** | Graph-Scoped Search Filtering | 需求 §44 | [基本设计 §03.2](03-functional-design.md#32-工程图谱子系统-engineering-graph-subsystem) | [详细设计 §02.8](../detailed-design/02-graph-engine.md) | UT, IT |
| **SRCH-REQ-004** | Vector Embedding & Semantic Similarity | 需求 §44 | [基本设计 §03.4.3](03-functional-design.md#343-v1-功能) | [详细设计 §05.6](../detailed-design/05-ai-gateway.md) | IT, ST |
| **UX-REQ-001** | Stable Skeleton + Emergent Context UI | 需求 §34 | [基本设计 §05.6](05-interface-design.md#56-ui-概述-ui-overview) | [详细设计 §13.4](../detailed-design/13-admin-api-and-ops-ui.md#134-ops-ui-前端架构) | ST, UAT |
| **UX-REQ-002** | Graph Node Navigation & Exploration | 需求 §34 | [基本设计 §05.6](05-interface-design.md#56-ui-概述-ui-overview) | [详细设计 §13.4](../detailed-design/13-admin-api-and-ops-ui.md#134-ops-ui-前端架构) | ST, UAT |
| **UX-REQ-003** | Timeline View for Graph Events & Audits | 需求 §34 | [基本设计 §14.2.3](14-admin-ops-ui.md#1423-中心事件流) | [详细设计 §13.4](../detailed-design/13-admin-api-and-ops-ui.md#134-ops-ui-前端架构) | ST, UAT |
| **UX-REQ-004** | Policy Simulation & Explainability UI | 需求 §34 | [基本设计 §14.2.4](14-admin-ops-ui.md#1425-审计日志) | [详细设计 §13.4](../detailed-design/13-admin-api-and-ops-ui.md#134-ops-ui-前端架构) | ST, UAT |
| **UX-REQ-005** | Real-Time Agent Stream & Interventions | 需求 §34 | [基本设计 §05.6](05-interface-design.md#56-ui-概述-ui-overview) | [详细设计 §04.8](../detailed-design/04-agent-runtime.md) | IT, ST |
| **UX-REQ-006** | Command Palette & Fast Switcher | 需求 §34 | [基本设计 §05.6](05-interface-design.md#56-ui-概述-ui-overview) | [详细设计 §13.4](../detailed-design/13-admin-api-and-ops-ui.md#134-ops-ui-前端架构) | ST, UAT |
| **UX-REQ-007** | Light / Dark Theme & Responsive UI | 需求 §34 | [基本设计 §05.6](05-interface-design.md#56-ui-概述-ui-overview) | [详细设计 §13.4](../detailed-design/13-admin-api-and-ops-ui.md#134-ops-ui-前端架构) | UAT |
| **A11Y-REQ-001** | WCAG 2.1 AA Conformance & Keyboard Reachability | 需求 §34 | [基本设计 §05.6](05-interface-design.md#56-ui-概述-ui-overview) | [详细设计 §13.4](../detailed-design/13-admin-api-and-ops-ui.md#134-ops-ui-前端架构) | UAT |
| **WKFL-REQ-001** | 10-Step Reference Loop Orchestration | 需求 §33 | [基本设计 §03.2](03-functional-design.md#32-工程图谱子系统-engineering-graph-subsystem) | [详细设计 §02.5](../detailed-design/02-graph-engine.md) | IT, ST, UAT |
| **WKFL-REQ-002** | Branch-to-Merge Human Review Gate | 需求 §33 | [基本设计 §10.2](10-acceptance-test-policy.md#102-mvp-验收条件重申-phase-9-6-的-definition-of-done) | [详细设计 §04.5](../detailed-design/04-agent-runtime.md) | IT, ST, UAT |

---

**导航 / Navigation:**
[← 10. 验收测试方针](10-acceptance-test-policy.md) · [README](README.md) · [Appendix B →](appendix-b-tbd.md)
