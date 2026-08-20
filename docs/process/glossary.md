# 项目用语集 / Project Glossary (PM / 流程专业)

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | — |
| 阶段 | 全流程 |
| 主要交付物 | 项目用语集 |
| 责任人 | 工程负责人 |
| 关联设计文档 | [`../../design/basic-design/appendix-d-glossary.md`](../../design/basic-design/appendix-d-glossary.md) (技术用语集) |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


本文件是**流程管理 / 项目管理**维度的用语集，与 [`appendix-d-glossary.md`](../../design/basic-design/appendix-d-glossary.md) 的**技术维度**用语集互补。


## 过程角色 / Process Roles

| 缩写 | 中文 | 英文 | 说明 |
|---|---|---|---|
| PO | 产品负责人 | Product Owner | 需求优先级 + 业务验收签核（QA-001） |
| EM | 工程经理 | Engineering Manager | 技术决策 + 工程进度 |
| TL | 技术负责人 | Tech Lead | 架构选型 + 代码评审最终仲裁 |
| SA | 系统架构师 | System Architect | 系统方式设计（任务 22）责任人 |
| BA | 业务分析师 | Business Analyst | 用户要求 / 业务要件（任务 10-11） |
| QA | 测试工程师 | Quality Assurance | 测试计划 / 实施（任务 59-89） |
| SRE | 运维工程师 | Site Reliability Engineer | 部署 / 监控 / 应急响应（任务 102-117） |
| DBA | 数据库管理员 | Database Administrator | 数据迁移 / 备份恢复（任务 99-100, 112） |
| SEC | 安全审计 | Security Auditor | AISEC-REQ / 安全测试 / F14-7 签核 |
| SM | Scrum Master | Scrum Master | 会议节奏 + 障碍移除（任务 140） |

## 阶段术语 / Phase Terms

| 缩写 | 中文 | 英文 | 对应任务 |
|---|---|---|---|
| UR | 用户要求 | User Requirements | 10 |
| BR | 业务要件 | Business Requirements | 11 |
| SR | 系统要件 | System Requirements | 12 |
| FR | 功能要件 | Functional Requirements | 13 |
| NFR | 非功能要件 | Non-Functional Requirements | 14 |
| SA | 系统方式设计 | System Architecture | 22 |
| BD | 基本设计 | Basic Design | 22-41 |
| DD | 详细设计 | Detailed Design | 42-52 |
| PG | 编码 | Programming | 54 |
| SAST | 静态分析 | Static Application Security Testing | 55 |
| CR | 代码评审 / 变更要求 | Code Review / Change Request | 56 / 118 |
| UT | 单元测试 | Unit Test | 59-65 |
| IT | 集成测试 | Integration Test | 66-75 |
| ITa | 内部集成测试 | Internal IT | 69 |
| ITb | 外部集成测试 | External IT | 70 |
| ST | 系统测试 | System Test | 76-89 |
| PT | 性能测试 | Performance Test | 80 |
| OT | 运用测试 | Operations Test | 88 |
| UAT | 用户验收测试 | User Acceptance Test | 90-95 |
| B/R | 备份恢复测试 | Backup/Restore Test | 86 |
| KT | 知识移交 | Knowledge Transfer | 149 |

## 管理术语 / Management Terms

| 缩写 | 中文 | 英文 | 说明 |
|---|---|---|---|
| WBS | 工作分解结构 | Work Breakdown Structure | 任务 132 |
| RACI | 责任分配矩阵 | Responsible / Accountable / Consulted / Informed | 本文件 §1 |
| DoD | 完成定义 | Definition of Done | 每阶段退出条件 |
| DoR | 就绪定义 | Definition of Ready | 任务进入条件 |
| SLA | 服务等级协议 | Service Level Agreement | 运维可用性承诺 |
| SLO | 服务等级目标 | Service Level Objective | 内部 SLA 目标 |
| RTO | 恢复时间目标 | Recovery Time Objective | NFR-REQ-001 |
| RPO | 恢复点目标 | Recovery Point Objective | NFR-REQ-001 |
| MTBF | 平均故障间隔 | Mean Time Between Failures | NFR-REQ-001 |
| MTTR | 平均恢复时间 | Mean Time To Recover | NFR-REQ-001 |

## 签核术语 / Sign-off Terms

| 术语 | 含义 | 对应 |
|---|---|---|
| Baseline 化 | 锁定当前版本为正式基线，后续变更走 CR 流程 | 任务 21 / 41 / 52 / 89 / 144 |
| Sign-off | 正式签核（PO + EM + SEC 三方） | 任务 20 / 41 / 52 / 89 / 94 / 103 / 145 |
| Go / No-Go | 发布判定会上的二元决策 | 任务 103 |
| Hypercare | 上线后初期高强度运维期（通常 1-4 周） | 任务 108 |

## IPA 用语简表 / IPA Term Index


完整对照见 [`workflow.md §4`](workflow.md) 术语对照表（76 个日文工程术语）。最常用：

| 日文 | 中文 | 英文 |
|---|---|---|
| 超上流 | 超前期 | Pre-Stream |
| 要件定義 | 需求定义 | Requirements Definition |
| 基本設計 | 基本设计 | Basic Design |
| 詳細設計 | 详细设计 | Detailed Design |
| 実装 | 实现 | Implementation |
| 単体試験 | 单元测试 | Unit Test |
| 結合試験 | 集成测试 | Integration Test |
| — | 系统测试 | System Test |
| 受入試験 | 验收测试 | Acceptance Test |
| 運用 | 运维 | Operations |
| 保守 | 维护 | Maintenance |
| 終結 | 终结 | Closure |

---

**导航 / Navigation:**
[← 流程总览](workflow.md) · [← 流程 README](README.md)
