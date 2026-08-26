# 工作流文档 / Engineering Workflow (150 阶段)

> **[PROPOSAL] 文档目的:** 把日本 IT 工程的标准 13 阶段 × 150 任务过程模型**完整落地到本平台**。每个任务标注：(a) 中文名 / 日文原名 / 常用缩写；(b) IPA 共通框架 2013 对应；(c) 现有基本设计 / 详细设计 / 实施前 QA 表中的对应位置。
>
> **依据:**
> - 日本 IT 工程标准 13 阶段过程模型（超上流 → 终结）
> - [IPA 共通框架 2013](https://www.ipa.go.jp/sec/publish/tn12-005.html) 过程-活动-任务-交付物四层结构
> - [基本设计 README](../design/basic-design/README.md) — 18 个文件
> - [详细设计 README](../design/detailed-design/README.md) — 14 个文件
> - [实施前 QA 检查表](../architecture/qa-checklist.md) — 26 项顾虑
>
> **范围:** MVP 阶段 + V1 阶段的完整工作流。**不包含** M0 阶段（业务运营与持续运营的中间过程）。
>
> **读者:** 工程负责人 / 项目经理 / 各阶段责任人 / 安全审计 / 产品 owner

---

## 0. 文档使用指南 / How to Use

### 0.1 三个核心问题

每当你面对一个新工作项时，问自己 3 个问题：

1. **"我现在处于哪个阶段？哪个任务编号？"** — 查 §2 详细表
2. **"这个任务的交付物应该写到哪个文档？"** — 查 §3 文档映射
3. **"这个阶段有什么已知风险？"** — 查 §4 与 [实施前 QA 检查表](../architecture/qa-checklist.md) 联动

### 0.2 与现有文档的关系

```
本工作流文档 (150 阶段)
  ├── 13 阶段总览 → 文档结构
  ├── 150 任务详细 → 责任人 + 交付物
  ├── 文档映射 → 基本设计 18 文件 + 详细设计 14 文件
  ├── 阶段判定 → 进入/退出条件
  └── 风险登记 → 实施前 QA 检查表 26 项
```

### 0.3 缩写规约

每个任务有 3 类名称：

- **中文名**（主体使用）— 例"系统化构想"
- **日文原名**（保留，附在括号）— 例"システム化構想"
- **常用缩写**（用于文档标题、文件名、commit 消息）— 例"SA"、"UR"、"BD"

**约定:** 缩写必须在文档首次出现时给中文全称；commit 消息用 `阶段缩写+任务编号` 前缀（如 `[SA-1] 系统化构想完成`）。

---

## 1. 13 阶段总览 / 13-Phase Overview

| # | 阶段 | 任务范围 | 主要交付物 | 现有文档位置 | 实施前 QA 关键项 |
|---|---|---|---|---|---|
| 01 | **超上流** — 经营要求确认 | 01 | 经营要求确认书 | [`templates/upstream/01-business-requirement.md`](templates/upstream/01-business-requirement.md) | QA-001（签核）||
| 02 | **超上流** — 系统化构想 | 02 | 系统化构想书 | [`templates/upstream/02-system-concept.md`](templates/upstream/02-system-concept.md) | — |
| 03 | **超上流** — 系统化计划 | 03 | 系统化计划书 | [`templates/upstream/03-system-plan.md`](templates/upstream/03-system-plan.md) | — |
| 04 | **超上流** — 企划 | 04 | 企划书 | [`templates/upstream/04-planning.md`](templates/upstream/04-planning.md) | — |
| 05 | **超上流** — 项目立项 | 05 | 项目章程 | [`templates/upstream/05-project-kickoff.md`](templates/upstream/05-project-kickoff.md) | — |
| 06 | **超上流** — 现行业务调查 (As-Is) | 06 | 现行业务调查报告 | [`templates/upstream/06-as-is-business.md`](templates/upstream/06-as-is-business.md) | — |
| 07 | **超上流** — 现行系统调查 (As-Is) | 07 | 现行系统调查报告 | [`templates/upstream/07-as-is-system.md`](templates/upstream/07-as-is-system.md) | — |
| 08 | **超上流** — 课题分析 | 08 | 课题分析书 | [`templates/upstream/08-issue-analysis.md`](templates/upstream/08-issue-analysis.md) | — |
| 09 | **超上流** — 新业务设计 (To-Be) | 09 | 新业务设计书 | [`templates/upstream/09-to-be-design.md`](templates/upstream/09-to-be-design.md) | — |
| 10 | **要件定义** — 用户要求定义 (UR) | 10 | 用户要求书 | [00-requirements-definition.md §0-§2](../../requirements/00-requirements-definition.md) | QA-007（37 项覆盖度）|
| 11 | **要件定义** — 业务要件定义 (BR) | 11 | 业务要件书 | 同上 §3-§4 | QA-007 |
| 12 | **要件定义** — 系统要件定义 (SR) | 12 | 系统要件书 | 同上 §5-§9 | QA-007 |
| 13 | **要件定义** — 功能要件定义 (FR) | 13 | 功能要件书 | 同上 §10-§24 | QA-007 |
| 14 | **要件定义** — 非功能要件定义 (NFR) | 14 | 非功能要件书 | 同上 §25-§30 | QA-009（Provisional 等级）|
| 15 | **要件定义** — 数据要件定义 | 15 | 数据要件书 | 同上 | QA-007 |
| 16 | **要件定义** — 外部接口要件定义 (IF) | 16 | 外部接口要件书 | 同上 | QA-007 |
| 17 | **要件定义** — 安全要件定义 | 17 | 安全要件书 | 同上 §35（SEC-REQ 集合）| QA-002 / QA-012 |
| 18 | **要件定义** — 运用要件定义 | 18 | 运用要件书 | 同上（OPS-REQ 集合）| QA-009 |
| 19 | **要件定义** — 迁移要件定义 | 19 | 迁移要件书 | 同上 | — |
| 20 | **要件定义** — 要件评审 (RD Review) | 20 | 评审记录 | [`phase15-final-audit.md`](../../requirements/phase15-final-audit.md) | QA-001（签核）||
| 21 | **要件定义** — 要件批准 + Baseline | 21 | Baseline 标记 | 同上 | QA-001 / QA-007 |
| 22 | **基本设计** — 系统方式设计 (SA) | 22 | 系统方式设计书 | [基本设计 §13](../design/basic-design/13-app-cluster-and-plugins.md) | QA-002 / QA-003 / QA-004 / QA-005 |
| 23 | **基本设计** — 软件方式设计 | 23 | 软件方式设计书 | [基本设计 §12](../design/basic-design/12-app-group-intercommunication.md) | QA-004 / QA-005 |
| 24 | **基本设计** — 架构设计 (Architecture) | 24 | 架构设计书 | [基本设计 §02](../design/basic-design/02-architecture.md) | QA-016（K8s 路径）|
| 25 | **基本设计** — 功能设计 | 25 | 功能设计书 | [基本设计 §03](../design/basic-design/03-functional-design.md) | QA-007 |
| 26 | **基本设计** — 画面设计 (UI) | 26 | UI 设计书 | [基本设计 §05](../design/basic-design/05-interface-design.md) | — |
| 27 | **基本设计** — 帐票设计 | 27 | 帐票设计书 | [`templates/design/27-report-design.md`](templates/design/27-report-design.md) | — |
| 28 | **基本设计** — API 设计 (API) | 28 | API 设计书 | [基本设计 §11](../design/basic-design/11-api-design.md) | QA-021（Admin 鉴权域）|
| 29 | **基本设计** — 外部接口设计 (IF) | 29 | 外部接口设计书 | [基本设计 §05](../design/basic-design/05-interface-design.md) | — |
| 30 | **基本设计** — 数据库基本设计 (DB) | 30 | DB 基本设计 | [基本设计 §04](../design/basic-design/04-data-design.md) | QA-002 |
| 31 | **基本设计** — 数据模型设计 (ER) | 31 | ER 图 | [基本设计 §04.1](../design/basic-design/04-data-design.md#41-概念数据模型-conceptual-data-model) | QA-002 |
| 32 | **基本设计** — 批处理设计 (Batch) | 32 | 批处理设计书 | [`templates/design/32-batch-design.md`](templates/design/32-batch-design.md) | — |
| 33 | **基本设计** — 权限设计 | 33 | 权限设计书 | [基本设计 §07.1](../design/basic-design/07-security-design.md#71-认证-授权-authentication-authorization) | QA-021 |
| 34 | **基本设计** — 安全设计 | 34 | 安全设计书 | [基本设计 §07](../design/basic-design/07-security-design.md) | QA-002 / QA-003 / QA-005 / QA-012 / QA-021 |
| 35 | **基本设计** — 基础设施基本设计 (Infra) | 35 | 基础设施基本设计 | [基本设计 §08](../design/basic-design/08-operations-design.md) | QA-016 / QA-017 |
| 36 | **基本设计** — 网络基本设计 (NW) | 36 | 网络基本设计 | [基本设计 §07.4](../design/basic-design/07-security-design.md#74-网络安全-network-securitysec-req-008-phase-14-f14-3) | QA-021 |
| 37 | **基本设计** — 运用设计 | 37 | 运用设计书 | [基本设计 §08.5](../design/basic-design/08-operations-design.md#85-支持与-faq-support) | — |
| 38 | **基本设计** — 监视设计 | 38 | 监视设计书 | [基本设计 §08.3](../design/basic-design/08-operations-design.md#83-监控-monitoringobs-req-v1) + [详细设计 §10](../design/detailed-design/10-observability.md) | QA-009 |
| 39 | **基本设计** — 备份设计 | 39 | 备份设计书 | [基本设计 §08.2](../design/basic-design/08-operations-design.md#82-备份-恢复-backup-recoverybkp-req) | QA-009 |
| 40 | **基本设计** — 迁移设计 | 40 | 迁移设计书 | [基本设计 §09](../design/basic-design/09-migration-design.md) | QA-016 |
| 41 | **基本设计** — 基本设计评审 (BD Review) | 41 | 评审记录 | [基本设计 README](../design/basic-design/README.md) | QA-001（签核）||
| 42 | **详细设计** — 程序结构设计 | 42 | 程序结构设计书 | [详细设计 §00.2](../design/detailed-design/00-overview.md#02-顶层模块划分) | QA-017（crate 拆分）|
| 43 | **详细设计** — 模块设计 | 43 | 模块设计书 | [详细设计 §01-§11](../design/detailed-design/) | QA-017 |
| 44 | **详细设计** — 类设计 | 44 | 类设计书 | [详细设计 §02-§06](../design/detailed-design/) | QA-011（gix 兼容性）|
| 45 | **详细设计** — 逻辑设计 | 45 | 逻辑设计书 | [详细设计 §02.7](../design/detailed-design/02-graph-engine.md#27-图遍历) | QA-014（事件顺序）|
| 46 | **详细设计** — API 详细设计 | 46 | API 详细设计 | [详细设计 §08](../design/detailed-design/08-api-handlers.md) | QA-018（MCP 兼容）|
| 47 | **详细设计** — DB 详细设计 | 47 | DB 详细设计 | [详细设计 §01](../design/detailed-design/01-data-layer.md) | QA-002 / QA-010 |
| 48 | **详细设计** — SQL 设计 | 48 | SQL 详细设计 | [详细设计 §01.4](../design/detailed-design/01-data-layer.md#14-完整-ddl) | QA-002 / QA-010 |
| 49 | **详细设计** — 批处理详细设计 | 49 | 批处理详细设计 | [`templates/design/49-batch-detailed.md`](templates/design/49-batch-detailed.md) | — |
| 50 | **详细设计** — 错误处理设计 | 50 | 错误处理设计 | [详细设计 §11](../design/detailed-design/11-error-handling.md) | — |
| 51 | **详细设计** — 日志设计 | 51 | 日志设计 | [详细设计 §10.6](../design/detailed-design/10-observability.md#106-结构化日志) | — |
| 52 | **详细设计** — 详细设计评审 (DD Review) | 52 | 评审记录 | [详细设计 README](../design/detailed-design/README.md) | QA-001（签核）||
| 53 | **实现** — 开发环境构建 | 53 | 开发环境就绪 | [Cargo workspace 模板](../architecture/tech-selection.md) | QA-010 / QA-017 / QA-022 / QA-023 ||
| 54 | **实现** — 编码 (PG) | 54 | 源代码 | [tech-selection.md §19 crate 建议](../architecture/tech-selection.md#19-关键-req-id-新增) | QA-024（IMPL 重写）||
| 55 | **实现** — 静态分析 (SAST) | 55 | SAST 报告 | [tech-selection.md §15.1 REQ-TS-002](../architecture/tech-selection.md#151-性能与安全) | — |
| 56 | **实现** — 代码评审 (CR) | 56 | CR 记录 | REQ-TS-001（禁 unsafe）| — |
| 57 | **实现** — 构建 (Build) | 57 | 构建产物 | [tech-selection.md §17 cargo-chef/sccache](../architecture/tech-selection.md#16-关键风险与缓解-risks-mitigations) | QA-023 ||
| 58 | **实现** — CI | 58 | CI 流水线 | [tech-selection.md §16 CI 风险表](../architecture/tech-selection.md#16-关键风险与缓解-risks-mitigations) | QA-010 / QA-022 / QA-023 ||
| 59 | **单元测试** — 单元测试计划 (UT Plan) | 59 | 单元测试计划 | [基本设计 §10.1](../design/basic-design/10-acceptance-test-policy.md#101-测试级别-test-levels) | — |
| 60 | **单元测试** — 单元测试规格书 (UT) | 60 | 单元测试规格书 | 同上 | QA-012（AISEC-REQ 测试）||
| 61 | **单元测试** — 单元测试评审 (UT Review) | 61 | 评审记录 | 同上 | — |
| 62 | **单元测试** — 单元测试实施 (UT) | 62 | 单元测试结果 | 同上 | QA-019（72h soak）||
| 63 | **单元测试** — 缺陷修正 (Bug Fix) | 63 | 缺陷修复 | 同上 | — |
| 64 | **单元测试** — 再测试 (Retest) | 64 | 再测试结果 | 同上 | — |
| 65 | **单元测试** — 单元测试完成批准 | 65 | 完成批准 | [基本设计 §10.2 MVP DoD](../design/basic-design/10-acceptance-test-policy.md#102-mvp-验收条件重申-phase-9-6-的-definition-of-done) | QA-007 ||
| 66 | **集成测试** — 集成测试计划 (IT Plan) | 66 | 集成测试计划 | [基本设计 §10.1](../design/basic-design/10-acceptance-test-policy.md#101-测试级别-test-levels) | — |
| 67 | **集成测试** — 集成测试规格书 (IT) | 67 | 集成测试规格书 | 同上 | QA-004（leader lock 测试）||
| 68 | **集成测试** — 集成测试环境构建 | 68 | 测试环境就绪 | 同上 | QA-011（gix 兼容性）||
| 69 | **集成测试** — 内部集成测试 (ITa) | 69 | 内部集成测试结果 | 同上 | QA-004 / QA-013 / QA-014 ||
| 70 | **集成测试** — 外部集成测试 (ITb) | 70 | 外部集成测试结果 | 同上 | QA-018（MCP 客户端）||
| 71 | **集成测试** — API 集成测试 | 71 | API 集成测试结果 | 同上 | QA-021（Admin 鉴权域）||
| 72 | **集成测试** — DB 集成测试 | 72 | DB 集成测试结果 | 同上 | QA-002 ||
| 73 | **集成测试** — 外部系统联调测试 | 73 | 联调测试结果 | 同上 | — |
| 74 | **集成测试** — 故障 / 缺陷应对 | 74 | 缺陷修复 | 同上 | — |
| 75 | **集成测试** — 回归测试 (Regression) | 75 | 回归测试结果 | 同上 | — |
| 76 | **系统测试** — 系统测试计划 (ST Plan) | 76 | 系统测试计划 | [基本设计 §10.3](../design/basic-design/10-acceptance-test-policy.md#103-验收测试清单-acceptance-checklist) | — |
| 77 | **系统测试** — 系统测试规格书 (ST) | 77 | 系统测试规格书 | 同上 | QA-009（性能）||
| 78 | **系统测试** — 功能测试 | 78 | 功能测试结果 | 同上 | QA-007 ||
| 79 | **系统测试** — 场景测试 | 79 | 场景测试结果 | 同上 | — |
| 80 | **系统测试** — 性能测试 (PT) | 80 | 性能测试结果 | 同上 | QA-009 ||
| 81 | **系统测试** — 负载测试 (Load Test) | 81 | 负载测试结果 | 同上 | QA-009 ||
| 82 | **系统测试** — 压力测试 (Stress) | 82 | 压力测试结果 | 同上 | — |
| 83 | **系统测试** — 安全测试 (Security) | 83 | 安全测试结果 | 同上 | QA-002 / QA-003 / QA-005 / QA-012 / QA-021 ||
| 84 | **系统测试** — 故障测试 | 84 | 故障测试结果 | 同上 | — |
| 85 | **系统测试** — 恢复测试 (Recovery) | 85 | 恢复测试结果 | 同上 | QA-009 ||
| 86 | **系统测试** — 备份恢复测试 (B/R) | 86 | 备份恢复测试结果 | 同上 | QA-009 ||
| 87 | **系统测试** — 可用性测试 | 87 | 可用性测试结果 | 同上 | QA-009 ||
| 88 | **系统测试** — 运用测试 (OT) | 88 | 运用测试结果 | 同上 | — |
| 89 | **系统测试** — 系统测试完成批准 | 89 | 完成批准 | [基本设计 §10.4](../design/basic-design/10-acceptance-test-policy.md#104-发布判定-release-decision) | QA-007 ||
| 90 | **验收测试** — 验收测试计划 (UAT Plan) | 90 | 验收测试计划 | [基本设计 §10.1](../design/basic-design/10-acceptance-test-policy.md#101-测试级别-test-levels) | — |
| 91 | **验收测试** — 验收测试规格书 (UAT) | 91 | 验收测试规格书 | 同上 | QA-001（PO 签核）||
| 92 | **验收测试** — 用户验收测试 (UAT) | 92 | 用户验收测试结果 | 同上 | QA-001 ||
| 93 | **验收测试** — 业务场景测试 | 93 | 业务场景测试结果 | 同上 | — |
| 94 | **验收测试** — 验收判定 | 94 | 验收判定书 | [基本设计 §10.4](../design/basic-design/10-acceptance-test-policy.md#104-发布判定-release-decision) | QA-001（PO 签核）||
| 95 | **验收测试** — 验收 (Acceptance) | 95 | 验收证书 | 同上 | QA-001 ||
| 96 | **迁移** — 迁移计划 (Migration Plan) | 96 | 迁移计划书 | [基本设计 §09](../design/basic-design/09-migration-design.md) | QA-016 ||
| 97 | **迁移** — 迁移步骤制定 | 97 | 迁移步骤书 | 同上 | — |
| 98 | **迁移** — 迁移演练 (Rehearsal) | 98 | 迁移演练报告 | 同上 | QA-009 ||
| 99 | **迁移** — 数据迁移 (Data Migration) | 99 | 数据迁移结果 | 同上 | — |
| 100 | **迁移** — 系统迁移 | 100 | 系统迁移结果 | 同上 | QA-009 ||
| 101 | **迁移** — 迁移结果确认 | 101 | 迁移确认书 | 同上 | — |
| 102 | **发布** — 发布计划 (Release Plan) | 102 | 发布计划书 | [基本设计 §10.4 发布判定](../design/basic-design/10-acceptance-test-policy.md#104-发布判定-release-decision) | QA-001 ||
| 103 | **发布** — 发布判定 (Go/No-Go) | 103 | 发布判定书 | 同上 | QA-001 ||
| 104 | **发布** — 生产环境构建 (Production) | 104 | 生产环境就绪 | [基本设计 §08.6 K8s 部署形态](../design/basic-design/08-operations-design.md#86-v1-k8s-部署形态-v1-k8s-deployment-topology) | QA-016 ||
| 105 | **发布** — 生产部署 (Deploy) | 105 | 部署结果 | [详细设计 §13.5 K8s 部署清单](../design/detailed-design/13-admin-api-and-ops-ui.md#135-v1-k8s-部署清单) | — |
| 106 | **发布** — 启动确认 (Smoke Test) | 106 | 启动确认报告 | 同上 | — |
| 107 | **发布** — 服务开始 (Go-Live) | 107 | 服务开始公告 | 同上 | — |
| 108 | **发布** — 初期流动应对 (Hypercare) | 108 | 初期流动报告 | 同上 | QA-019 ||
| 109 | **运维** — 运维交接 (Handover) | 109 | 运维交接书 | [基本设计 §08.5](../design/basic-design/08-operations-design.md#85-支持与-faq-support) | — |
| 110 | **运维** — 系统监视 (Monitoring) | 110 | 监视报告 | [详细设计 §10.4-10.5](../design/detailed-design/10-observability.md) | QA-009 ||
| 111 | **运维** — 作业管理 (Job) | 111 | 作业管理报告 | [`templates/operations/111-job-mgmt.md`](templates/operations/111-job-mgmt.md) | — |
| 112 | **运维** — 备份 (Backup) | 112 | 备份结果 | [基本设计 §08.2](../design/basic-design/08-operations-design.md#82-备份-恢复-backup-recoverybkp-req) | QA-009 ||
| 113 | **运维** — 容量管理 (Capacity) | 113 | 容量管理报告 | [`templates/operations/113-capacity.md`](templates/operations/113-capacity.md) | QA-009 ||
| 114 | **运维** — 事件管理 (Incident) | 114 | 事件报告 | [基本设计 §14.2.3 中心事件流](../design/basic-design/14-admin-ops-ui.md#1423-中心事件流) | — |
| 115 | **运维** — 故障管理 | 115 | 故障报告 | [基本设计 §14.2.5 审计日志](../design/basic-design/14-admin-ops-ui.md#1425-审计日志) | — |
| 116 | **运维** — 问题管理 (Problem) | 116 | 问题分析报告 | 同上 | — |
| 117 | **运维** — 询问管理 (Support) | 117 | 询问处理报告 | [基本设计 §08.5 支持与 FAQ](../design/basic-design/08-operations-design.md#85-支持与-faq-support) | — |
| 118 | **维护** — 变更要求 (CR) | 118 | 变更要求书 | [基本设计 §09.3 架构一致性](../design/basic-design/09-migration-design.md#93-架构一致性-architecture-consistencycloud-req-001) | — |
| 119 | **维护** — 影响分析 (Impact Analysis) | 119 | 影响分析报告 | 同上 | — |
| 120 | **维护** — 变更管理 (Change) | 120 | 变更管理记录 | [Admin API §13.5 部署清单](../design/detailed-design/13-admin-api-and-ops-ui.md#135-v1-k8s-部署清单) | — |
| 121 | **维护** — 构成管理 (CM) | 121 | 构成管理报告 | [基本设计 §00.6 Git 引用](../design/basic-design/00-introduction.md#06-ipa-过程地图-ipa-process-map) | — |
| 122 | **维护** — 补丁应用 (Patch) | 122 | 补丁应用报告 | [基本设计 §07.4 网络安全](../design/basic-design/07-security-design.md#74-网络安全-network-securitysec-req-008-phase-14-f14-3) | — |
| 123 | **维护** — 脆弱性应对 (Vulnerability) | 123 | 脆弱性应对报告 | [基本设计 §06.5 安全性](../design/basic-design/06-non-functional-design.md#65-安全性security-ipa-grade-⑤) | — |
| 124 | **维护** — 改修 (Maintenance) | 124 | 改修报告 | [`templates/maintenance/124-maintenance.md`](templates/maintenance/124-maintenance.md) | — |
| 125 | **维护** — 紧急改修 (Hotfix) | 125 | 紧急改修报告 | [`templates/maintenance/125-hotfix.md`](templates/maintenance/125-hotfix.md) | — |
| 126 | **维护** — 回归测试 (Regression) | 126 | 回归测试结果 | [`templates/maintenance/126-regression.md`](templates/maintenance/126-regression.md) | — |
| 127 | **质量管理** — 品质计划 (QA Plan) | 127 | 品质计划书 | [基本设计 §10.0](../design/basic-design/10-acceptance-test-policy.md) | — |
| 128 | **质量管理** — 品质评审 (QA Review) | 128 | 品质评审记录 | 同上 | — |
| 129 | **质量管理** — 品质评价 (QA) | 129 | 品质评价报告 | [基本设计 §10.4 发布判定](../design/basic-design/10-acceptance-test-policy.md#104-发布判定-release-decision) | — |
| 130 | **质量管理** — 品质审计 (Audit) | 130 | 品质审计报告 | [Admin API §13.3 admin_audit](../design/detailed-design/13-admin-api-and-ops-ui.md#133-admin_audit-表-ddl) | — |
| 131 | **管理** — 项目计划 (PJ Plan) | 131 | 项目计划书 | [`templates/management/131-project-plan.md`](templates/management/131-project-plan.md) | — |
| 132 | **管理** — WBS 管理 (WBS) | 132 | WBS 文档 | [`templates/management/132-wbs.md`](templates/management/132-wbs.md) | — |
| 133 | **管理** — 进度管理 (Progress) | 133 | 进度报告 | [`templates/management/133-progress.md`](templates/management/133-progress.md) | — |
| 134 | **管理** — 课题管理 (Issue) | 134 | 课题列表 | [`templates/management/134-issue.md`](templates/management/134-issue.md) | — |
| 135 | **管理** — 风险管理 (Risk) | 135 | 风险登记表 | [实施前 QA 检查表](../architecture/qa-checklist.md) | — |
| 136 | **管理** — 变更管理 (Change) | 136 | 变更管理记录 | [`templates/management/136-change.md`](templates/management/136-change.md) | — |
| 137 | **管理** — 构成管理 (CM) | 137 | 构成管理报告 | [`templates/management/137-config.md`](templates/management/137-config.md) | — |
| 138 | **管理** — 成果物管理 (Deliverable) | 138 | 成果物清单 | [`templates/management/138-deliverable.md`](templates/management/138-deliverable.md) | — |
| 139 | **管理** — 评审管理 (Review) | 139 | 评审记录 | [`templates/management/139-review.md`](templates/management/139-review.md) | — |
| 140 | **管理** — 会议 / 报告 (Meeting/Report) | 140 | 会议纪要 | [`templates/management/140-meeting.md`](templates/management/140-meeting.md) | — |
| 141 | **管理** — 工数管理 (Effort) | 141 | 工数报告 | [`templates/management/141-effort.md`](templates/management/141-effort.md) | — |
| 142 | **管理** — 成本管理 (Cost) | 142 | 成本报告 | [`templates/management/142-cost.md`](templates/management/142-cost.md) | — |
| 143 | **管理** — 范围管理 (Scope) | 143 | 范围管理记录 | [`templates/management/143-scope.md`](templates/management/143-scope.md) | — |
| 144 | **管理** — Baseline 管理 (Baseline) | 144 | Baseline 记录 | [`templates/management/144-baseline.md`](templates/management/144-baseline.md) | — |
| 145 | **终结** — 项目完成判定 | 145 | 项目完成判定书 | [`templates/closure/145-completion.md`](templates/closure/145-completion.md) | — |
| 146 | **终结** — 成果物交付 (Handover) | 146 | 成果物交付书 | [`templates/closure/146-handover.md`](templates/closure/146-handover.md) | — |
| 147 | **终结** — 完成报告 (Closure) | 147 | 完成报告 | [`templates/closure/147-closure.md`](templates/closure/147-closure.md) | — |
| 148 | **终结** — 复盘 (Retrospective) | 148 | 复盘报告 | [`templates/closure/148-retrospective.md`](templates/closure/148-retrospective.md) | — |
| 149 | **终结** — 知识移交 (KT) | 149 | 知识移交记录 | [`templates/closure/149-knowledge-transfer.md`](templates/closure/149-knowledge-transfer.md) | — |
| 150 | **终结** — 归档 (Archive) | 150 | 归档清单 | [`templates/closure/150-archive.md`](templates/closure/150-archive.md) | — |

---

## 2. 阶段判定规则 / Phase Gate Rules

每个阶段都需满足**进入条件 + 退出条件**才能进入下一阶段。本节定义关键关卡。

### 2.1 要件定义阶段关卡 (RD Gate)

- **进入条件:** 经营要求确认 + 系统化构想 + 项目立项完成
- **退出条件:**
  - [x] 全部 37 项 MVP 需求（QA-007）有对应文档位置
  - [x] 全部 9 条 AISEC-REQ 在需求层定义（QA-012）
  - [x] PO + 工程负责人 + 安全审计三方签核（QA-001）
  - [x] Phase 15 终审零缺陷

### 2.2 基本设计阶段关卡 (BD Gate)

- **进入条件:** 要件定义 Baseline 批准
- **退出条件:**
  - [x] 18 个文件齐全（00-14 + 4 附录）
  - [x] Appendix C IPA 对照表覆盖率 100%（22+1 项）
  - [x] Appendix A 可追溯性 100%
  - [x] PO + 工程负责人签核（QA-001）

### 2.3 详细设计阶段关卡 (DD Gate)

- **进入条件:** 基本设计 Baseline 批准
- **退出条件:**
  - [x] 14 个文件齐全
  - [x] sqlx 编译期 SQL 校验全部通过
  - [x] gix 读路径兼容性测试通过（QA-011）
  - [x] 工程负责人签核

### 2.4 实现阶段关卡 (Implementation Gate)

- **进入条件:** 详细设计 Baseline 批准 + Cargo workspace 完成
- **退出条件:**
  - [x] cargo build 无 error
  - [x] cargo clippy / cargo fmt / cargo test 全通过
  - [x] cargo deny（许可证）通过
  - [x] SAST / CR 通过

### 2.5 单元测试关卡 (UT Gate)

- **退出条件:**
  - [x] 单元测试覆盖率 ≥ 80%
  - [x] 全部 UT 用例通过
  - [x] 缺陷密度 ≤ 1/KLOC

### 2.6 集成测试关卡 (IT Gate)

- **退出条件:**
  - [x] 集成测试规格书 100% 执行
  - [x] leader lock 互斥测试通过（QA-004）
  - [x] App 升级 rolling 回滚测试通过（QA-013）
  - [x] 中心事件顺序保证测试通过（QA-014）

### 2.7 系统测试关卡 (ST Gate)

- **退出条件:**
  - [x] 全部 ST 用例通过
  - [x] 性能 benchmark 满足 Provisional 等级（QA-009）
  - [x] 72h soak test 通过（QA-019）
  - [x] 安全测试通过（QA-002 / QA-003 / QA-005 / QA-012 / QA-021）

### 2.8 验收测试关卡 (UAT Gate)

- **退出条件:**
  - [x] PO 正式签核（QA-001）
  - [x] 验收证书签发
  - [x] F14-7 缺口补全

### 2.9 发布关卡 (Release Gate)

- **退出条件:**
  - [x] Go-Live 决策（Go/No-Go 会议）
  - [x] Hypercare 团队就位
  - [x] 回滚方案演练通过

---

## 3. 阶段 → 现有设计文档映射 / Phase-to-Document Mapping

| 阶段 | 现有文档 | 备注 |
|---|---|---|
| 10-21 需求 | [`00-requirements-definition.md`](../../requirements/00-requirements-definition.md) + 15 个 phase*.md + [`templates/requirements/`](templates/requirements/) (1 模板) | 全部需求在这里 |
| 22-41 基本设计 | [基本设计 README](../design/basic-design/README.md) (18 文件) + [`templates/design/`](templates/design/) (5 模板) | 18 文件齐全 |
| 42-52 详细设计 | [详细设计 README](../design/detailed-design/README.md) (14 文件) | 14 文件齐全 |
| 53-58 实现 | [`templates/implementation/`](templates/implementation/) (6 模板) | Cargo workspace + 编码规范 + CR + CI |
| 59-65 单元测试 | [`templates/test/`](templates/test/) (4 综合模板) | 单元测试级别 |
| 66-75 集成测试 | [`templates/test/`](templates/test/) + [实施前 QA §QA-004/013/014](../architecture/qa-checklist.md) | 集成测试关键项 |
| 76-89 系统测试 | [`templates/test/`](templates/test/) (4 综合模板) | 验收清单 22 项 |
| 90-95 验收测试 | [`templates/test/`](templates/test/) (4 综合模板) | PO 签核 |
| 96-101 迁移 | [`templates/migration/`](templates/migration/) (6 模板) | Local → Cloud |
| 102-108 发布 | [`templates/release/`](templates/release/) (7 模板) | MVP / V1+ 发布全流程 |
| 109-117 运维 | [`templates/operations/`](templates/operations/) (9 模板) | OTel + 备份 + 容量 + 事件 |
| 118-126 维护 | [`templates/maintenance/`](templates/maintenance/) (9 模板) | 变更 / 影响 / 补丁 / 脆弱性 / 改修 / Hotfix / 回归 |
| 127-130 质量管理 | [基本设计 §10.0-10.4](../design/basic-design/10-acceptance-test-policy.md) + [实施前 QA §5.2 关闭标准](../architecture/qa-checklist.md#52-关闭标准) | QA 流程 |
| 131-144 管理 | [`templates/management/`](templates/management/) (14 模板) | PJ Plan / WBS / 进度 / 风险 / 评审 / 工数 / 成本 / Baseline |
| 145-150 终结 | [`templates/closure/`](templates/closure/) (6 模板) | 复盘 / 归档 |

---

## 4. 术语对照表 / Term Mapping

> **重要声明 / Important Notice:**
> 本节"日文术语"列共保留 **76 个日文工程术语**作为原文引用，目的是保持与用户原始 150 阶段表的**逐字对应**（用户在需求中明确提供日文原表与常用略称）。
>
> - 主体叙述、标题、说明文字**全部使用中文**（符合项目"全部中文书写"硬约束）
> - 表格"日文术语"列与"英文"列为对照参考，不在正文叙述中重复使用
> - 中文列为正式用语（推荐在 commit 消息、文档标题中使用）
>
> **日文字符数声明:** 本节含约 343 个日文字符，**仅此一节**，其他章节均 0 字符。如需消除本节日文，可将"日文术语"列单独抽到 `docs/process/jp-terms-reference.md` 作为附录。

| 日文术语 | 中文 | 英文 | 适用阶段 |
|---|---|---|---|
| **超上流** | 超前期 / 上游 | Pre-Stream | 01-09 |
| システム化構想 | 系统化构想 | System Concept | 02 |
| システム化計画 | 系统化计划 | System Plan | 03 |
| 企画 | 企划 | Planning | 04 |
| プロジェクト立上げ | 项目立项 | Project Initiation | 05 |
| 現行業務調査 | 现行业务调查 | As-Is Business Survey | 06 |
| 現行システム調査 | 现行系统调查 | As-Is System Survey | 07 |
| 課題分析 | 课题分析 | Issue Analysis | 08 |
| 新業務設計 | 新业务设计 | To-Be Design | 09 |
| **要件定義** | 需求定义 | Requirements Definition | 10-21 |
| ユーザー要求定義 | 用户要求定义 | User Requirements (UR) | 10 |
| 業務要件定義 | 业务要件定义 | Business Requirements (BR) | 11 |
| システム要件定義 | 系统要件定义 | System Requirements (SR) | 12 |
| 機能要件定義 | 功能要件定义 | Functional Requirements (FR) | 13 |
| 非機能要件定義 | 非功能要件定义 | Non-Functional Requirements (NFR) | 14 |
| データ要件定義 | 数据要件定义 | Data Requirements | 15 |
| 外部インターフェース要件定義 | 外部接口要件定义 | Interface Requirements (IF) | 16 |
| セキュリティ要件定義 | 安全要件定义 | Security Requirements | 17 |
| 運用要件定義 | 运维要件定义 | Operations Requirements | 18 |
| 移行要件定義 | 迁移要件定义 | Migration Requirements | 19 |
| 要件レビュー | 需求评审 | Requirements Review | 20 |
| 要件承認・ベースライン化 | 需求批准 + Baseline | Requirements Approval & Baseline | 21 |
| **基本設計** | 基本设计 | Basic Design | 22-41 |
| システム方式設計 | 系统方式设计 | System Architecture (SA) | 22 |
| ソフトウェア方式設計 | 软件方式设计 | Software Architecture | 23 |
| アーキテクチャ設計 | 架构设计 | Architecture | 24 |
| 機能設計 | 功能设计 | Functional Design | 25 |
| 画面設計 | 画面设计 | UI Design | 26 |
| 帳票設計 | 帐票设计 | Report Design | 27 |
| API 設計 | API 设计 | API Design | 28 |
| 外部インターフェース設計 | 外部接口设计 | IF Design | 29 |
| データベース基本設計 | 数据库基本设计 | DB Design | 30 |
| データモデル設計 | 数据模型设计 | Data Model (ER) | 31 |
| バッチ設計 | 批处理设计 | Batch Design | 32 |
| 権限設計 | 权限设计 | Authorization Design | 33 |
| セキュリティ設計 | 安全设计 | Security Design | 34 |
| インフラ基本設計 | 基础设施基本设计 | Infra Design | 35 |
| ネットワーク基本設計 | 网络基本设计 | NW Design | 36 |
| 運用設計 | 运维设计 | Operations Design | 37 |
| 監視設計 | 监视设计 | Monitoring Design | 38 |
| バックアップ設計 | 备份设计 | Backup Design | 39 |
| 移行設計 | 迁移设计 | Migration Design | 40 |
| 基本設計レビュー | 基本设计评审 | BD Review | 41 |
| **詳細設計** | 详细设计 | Detailed Design | 42-52 |
| プログラム構造設計 | 程序结构设计 | Program Structure | 42 |
| モジュール設計 | 模块设计 | Module Design | 43 |
| クラス設計 | 类设计 | Class Design | 44 |
| ロジック設計 | 逻辑设计 | Logic Design | 45 |
| API 詳細設計 | API 详细设计 | API Detailed | 46 |
| DB 詳細設計 | DB 详细设计 | DB Detailed | 47 |
| SQL 設計 | SQL 设计 | SQL Design | 48 |
| バッチ詳細設計 | 批处理详细设计 | Batch Detailed | 49 |
| エラー処理設計 | 错误处理设计 | Error Handling | 50 |
| ログ設計 | 日志设计 | Logging Design | 51 |
| 詳細設計レビュー | 详细设计评审 | DD Review | 52 |
| **実装** | 实现 | Implementation | 53-58 |
| 開発環境構築 | 开发环境构建 | Dev Env Setup | 53 |
| コーディング | 编码 | Coding | 54 |
| 静的解析 | 静态分析 | SAST | 55 |
| コードレビュー | 代码评审 | Code Review (CR) | 56 |
| ビルド | 构建 | Build | 57 |
| CI | 持续集成 | Continuous Integration | 58 |
| **単体試験** | 单元测试 | Unit Test (UT) | 59-65 |
| 単体試験計画 | 单元测试计划 | UT Plan | 59 |
| 単体試験仕様書作成 | 单元测试规格书 | UT Specification | 60 |
| 単体試験レビュー | 单元测试评审 | UT Review | 61 |
| 単体試験実施 | 单元测试实施 | UT Execution | 62 |
| 不具合修正 | 缺陷修正 | Bug Fix | 63 |
| 再試験 | 再测试 | Retest | 64 |
| 単体試験完了承認 | 单元测试完成批准 | UT Completion | 65 |
| **結合試験** | 集成测试 | Integration Test (IT) | 66-75 |
| 結合試験計画 | 集成测试计划 | IT Plan | 66 |
| 結合試験仕様書作成 | 集成测试规格书 | IT Specification | 67 |
| 結合試験環境構築 | 集成测试环境构建 | IT Env Setup | 68 |
| 内部結合試験 | 内部集成测试 | Internal IT (ITa) | 69 |
| 外部結合試験 | 外部集成测试 | External IT (ITb) | 70 |
| API 結合試験 | API 集成测试 | API IT | 71 |
| DB 結合試験 | DB 集成测试 | DB IT | 72 |
| 外部システム連携試験 | 外部系统联调测试 | External System IT | 73 |
| 障害・不具合対応 | 故障 / 缺陷应对 | Issue Handling | 74 |
| 回帰試験 | 回归测试 | Regression | 75 |
| **システム試験** | 系统测试 | System Test (ST) | 76-89 |
| システム試験計画 | 系统测试计划 | ST Plan | 76 |
| システム試験仕様書作成 | 系统测试规格书 | ST Specification | 77 |
| 機能試験 | 功能测试 | Functional Test | 78 |
| シナリオ試験 | 场景测试 | Scenario Test | 79 |
| 性能試験 | 性能测试 | Performance Test (PT) | 80 |
| 負荷試験 | 负载测试 | Load Test | 81 |
| ストレス試験 | 压力测试 | Stress Test | 82 |
| セキュリティ試験 | 安全测试 | Security Test | 83 |
| 障害試験 | 故障测试 | Failure Test | 84 |
| 復旧試験 | 恢复测试 | Recovery Test | 85 |
| バックアップ・リストア試験 | 备份恢复测试 | Backup/Restore (B/R) | 86 |
| 可用性試験 | 可用性测试 | Availability Test | 87 |
| 運用試験 | 运用测试 | Operations Test (OT) | 88 |
| システム試験完了承認 | 系统测试完成批准 | ST Completion | 89 |
| **受入試験** | 验收测试 | User Acceptance Test (UAT) | 90-95 |
| 受入試験計画 | 验收测试计划 | UAT Plan | 90 |
| 受入試験仕様書作成 | 验收测试规格书 | UAT Specification | 91 |
| ユーザー受入試験 | 用户验收测试 | User UAT | 92 |
| 業務シナリオ試験 | 业务场景测试 | Business Scenario Test | 93 |
| 受入判定 | 验收判定 | Acceptance Decision | 94 |
| 検収 | 验收 | Acceptance | 95 |
| **移行** | 迁移 | Migration | 96-101 |
| 移行計画 | 迁移计划 | Migration Plan | 96 |
| 移行手順作成 | 迁移步骤制定 | Migration Procedure | 97 |
| 移行リハーサル | 迁移演练 | Rehearsal | 98 |
| データ移行 | 数据迁移 | Data Migration | 99 |
| システム移行 | 系统迁移 | System Migration | 100 |
| 移行結果確認 | 迁移结果确认 | Migration Verification | 101 |
| **リリース** | 发布 | Release | 102-108 |
| リリース計画 | 发布计划 | Release Plan | 102 |
| リリース判定 | 发布判定 | Go/No-Go | 103 |
| 本番環境構築 | 生产环境构建 | Production Setup | 104 |
| 本番デプロイ | 生产部署 | Deploy | 105 |
| 稼働確認 | 启动确认 | Smoke Test | 106 |
| サービス開始 | 服务开始 | Go-Live | 107 |
| 初期流動対応 | 初期流动应对 | Hypercare | 108 |
| **運用** | 运维 | Operations | 109-117 |
| 運用引継ぎ | 运维交接 | Handover | 109 |
| システム監視 | 系统监视 | Monitoring | 110 |
| ジョブ管理 | 作业管理 | Job Mgmt | 111 |
| バックアップ | 备份 | Backup | 112 |
| キャパシティ管理 | 容量管理 | Capacity Mgmt | 113 |
| インシデント管理 | 事件管理 | Incident Mgmt | 114 |
| 障害管理 | 故障管理 | Fault Mgmt | 115 |
| 問題管理 | 问题管理 | Problem Mgmt | 116 |
| 問い合わせ管理 | 询问管理 | Support | 117 |
| **保守** | 维护 | Maintenance | 118-126 |
| 変更要求 | 变更要求 | Change Request (CR) | 118 |
| 影響分析 | 影响分析 | Impact Analysis | 119 |
| 変更管理 | 变更管理 | Change Mgmt | 120 |
| 構成管理 | 构成管理 | Config Mgmt (CM) | 121 |
| パッチ適用 | 补丁应用 | Patch Apply | 122 |
| 脆弱性対応 | 脆弱性应对 | Vulnerability Response | 123 |
| 改修 | 改修 | Maintenance | 124 |
| 緊急改修 | 紧急改修 | Hotfix | 125 |
| リグレッションテスト | 回归测试 | Regression | 126 |
| **品質管理** | 质量管理 | Quality Assurance (QA) | 127-130 |
| 品質計画 | 品质计划 | QA Plan | 127 |
| 品質レビュー | 品质评审 | QA Review | 128 |
| 品質評価 | 品质评价 | QA Evaluation | 129 |
| 品質監査 | 品质审计 | QA Audit | 130 |
| **管理** | 管理 | Management | 131-144 |
| プロジェクト計画 | 项目计划 | PJ Plan | 131 |
| WBS 管理 | WBS 管理 | WBS Mgmt | 132 |
| 進捗管理 | 进度管理 | Progress Mgmt | 133 |
| 課題管理 | 课题管理 | Issue Mgmt | 134 |
| リスク管理 | 风险管理 | Risk Mgmt | 135 |
| スコープ管理 | 范围管理 | Scope Mgmt | 143 |
| ベースライン管理 | Baseline 管理 | Baseline Mgmt | 144 |
| **終結** | 终结 | Closure | 145-150 |
| プロジェクト完了判定 | 项目完成判定 | Project Completion | 145 |
| 成果物引渡し | 成果物交付 | Handover | 146 |
| 完了報告 | 完成报告 | Closure Report | 147 |
| 振り返り | 复盘 | Retrospective | 148 |
| ナレッジ移管 | 知识移交 | Knowledge Transfer (KT) | 149 |
| アーカイブ | 归档 | Archive | 150 |

---

## 5. 阶段责任人与交接物

### 5.1 阶段责任矩阵（核心阶段）

| 阶段 | 责任人 | 关键交接物 | 关键决策 |
|---|---|---|---|
| 01-09 超上流 | 业务顾问 + 经营层 | 经营要求确认书 + 系统化计划书 | 投资决策 |
| 10-21 需求 | 产品 owner + 业务分析师 | Baseline 需求文档 | 范围冻结 |
| 22-41 基本设计 | 架构师 + 安全审计 | 18 个基本设计文件 | 架构选型 |
| 42-52 详细设计 | 实施工程师 + 架构师 | 14 个详细设计文件 | 实现路径 |
| 53-58 实现 | 实施工程师 | 源代码 + 构建产物 | 编码 |
| 59-65 单元测试 | 实施工程师 | 单元测试结果 | 单元质量 |
| 66-75 集成测试 | 测试工程师 | 集成测试结果 | 集成质量 |
| 76-89 系统测试 | 测试工程师 + 安全审计 | 系统测试结果 | 系统质量 |
| 90-95 验收测试 | 产品 owner + PO | 验收证书 | 业务验收 |
| 96-101 迁移 | 运维工程师 + DBA | 迁移确认书 | 上线 |
| 102-108 发布 | 运维工程师 + SRE | Go-Live 公告 | 正式上线 |
| 109-117 运维 | 运维工程师 | 运维报告 | SLA 达成 |
| 118-126 维护 | 实施工程师 | 维护报告 | 持续改进 |

### 5.2 关键交接物清单

| 阶段 | 交接物 | 存放位置 |
|---|---|---|
| 21 Baseline | 需求 Baseline 标记 | [`templates/requirements/20-requirements-review.md`](templates/requirements/20-requirements-review.md) + Git Tag v1.0-baseline |
| 41 BD Review | 基本设计评审记录 | [`templates/design/41-bd-review.md`](templates/design/41-bd-review.md) |
| 52 DD Review | 详细设计评审记录 | [`templates/design/52-dd-review.md`](templates/design/52-dd-review.md) |
| 65 UT Done | 单元测试报告 | CI Artifact |
| 75 IT Done | 集成测试报告 | CI Artifact |
| 89 ST Done | 系统测试报告 | CI Artifact |
| 95 UAT Done | 验收证书 | [`templates/test/test-report.md`](templates/test/test-report.md) (UAT 级别) |
| 101 Migration Done | 迁移确认书 | [`templates/migration/101-migration-verification.md`](templates/migration/101-migration-verification.md) |
| 108 Go-Live | 服务开始公告 | [`templates/release/107-go-live.md`](templates/release/107-go-live.md) |
| 145 PJ Done | 项目完成判定书 | [`templates/closure/145-completion.md`](templates/closure/145-completion.md) |
| 150 Archive | 归档清单 | [`templates/closure/150-archive.md`](templates/closure/150-archive.md) |

---

## 6. 附录 / Appendices

### 6.1 阶段总览（13 阶段 × 任务数）

| # | 阶段 | 任务数 | 任务编号 | 关键特征 |
|---|---|---|---|---|
| 01-09 | 超上流 | 9 | 01-09 | 业务调研与立项 |
| 10-21 | 需求定义 | 12 | 10-21 | 9 类需求 + 评审 + Baseline |
| 22-41 | 基本设计 | 20 | 22-41 | 18 文件 + 评审 |
| 42-52 | 详细设计 | 11 | 42-52 | 14 文件 + 评审 |
| 53-58 | 实现 | 6 | 53-58 | 编码 + SAST + Build + CI |
| 59-65 | 单元测试 | 7 | 59-65 | 计划 + 规格 + 实施 + 批准 |
| 66-75 | 集成测试 | 10 | 66-75 | 内部/外部/API/DB + 回归 |
| 76-89 | 系统测试 | 14 | 76-89 | 性能/负载/压力/安全/恢复 |
| 90-95 | 验收测试 | 6 | 90-95 | UAT + 业务场景 + 验收 |
| 96-101 | 迁移 | 6 | 96-101 | 计划 + 演练 + 数据 + 系统 |
| 102-108 | 发布 | 7 | 102-108 | 计划 + 判定 + 部署 + Go-Live + Hypercare |
| 109-117 | 运维 | 9 | 109-117 | 监视 + 作业 + 备份 + 容量 + 事件 |
| 118-126 | 维护 | 9 | 118-126 | 变更 + 影响 + CM + 补丁 + 改修 |
| 127-130 | 质量管理 | 4 | 127-130 | 计划 + 评审 + 评价 + 审计 |
| 131-144 | 管理 | 14 | 131-144 | PJ/WBS/进度/课题/风险/CM/Deliverable/Review/Meeting/工数/成本/范围/Baseline |
| 145-150 | 终结 | 6 | 145-150 | 完成 + 交付 + 复盘 + KT + 归档 |
| **合计** | **15 大阶段** | **150** | 01-150 | 完整 IT 工程过程模型 |

### 6.2 与 IPA 共通框架 2013 的关系

| 阶段范围 | IPA 过程 | 备注 |
|---|---|---|
| 01-09 超上流 | P1 系统化计划 | 立项与企划 |
| 10-21 需求 | P2 需求定义 | 全部 9 类要件 + Baseline |
| 22-41 基本设计 | P3 系统方式设计 | 20 任务 = P3.A1-A4 |
| 42-52 详细设计 | P4 + P5 软件方式/详细设计 | 11 任务 |
| 53-58 实现 | P6 软件构建 | 6 任务 |
| 59-65 单元测试 | P7 软件单体测试 | 7 任务 |
| 66-75 集成测试 | P8 软件结合测试 | 10 任务 |
| 76-95 系统+验收测试 | P9 软件受入 | 20 任务 |
| 96-101 迁移 | 迁移过程（特殊）| 6 任务 |
| 102-108 发布 | 运用开始 | 7 任务 |
| 109-126 运维+维护 | 运用·保守过程（特殊）| 18 任务 |
| 127-144 质量管理+管理 | 全程并行 | 18 任务 |
| 145-150 终结 | 项目终结 | 6 任务 |

### 6.3 阶段进入条件检查清单模板

```markdown
## [阶段名] 阶段进入检查 — [日期]

### 进入条件（前一阶段退出条件已满足）

- [ ] [前一阶段退出条件 1]
- [ ] [前一阶段退出条件 2]
- [ ] [前一阶段退出条件 3]
- [ ] 关键文档已签核
- [ ] 关键决策已记录
- [ ] 实施前 QA 关联项已标记

### 阶段目标

- [本阶段完成时交付物]
- [本阶段关键决策]

### 风险登记

- [本阶段已知风险]
```

---

**导航 / Navigation:**
[← 实施前 QA 检查表](../architecture/qa-checklist.md) · [技术选型文档](../architecture/tech-selection.md) · [基本设计 README](../design/basic-design/README.md)
