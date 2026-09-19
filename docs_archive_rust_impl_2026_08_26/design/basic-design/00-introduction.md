# 00. 介绍 / Introduction

> **[PROPOSAL] IPA 共通框架 2013 章节定位：** §0.7 本书构成 / 4 个任务 (T4-1/2/3)


## 0.0 依据宣言 / Compliance Declaration

**[PROPOSAL]** 本设计书严格按照日本 **独立行政法人情报处理推进机构（IPA，情報処理推進機構）** 发布的以下框架与标准编写：

| 标准编号 | 名称 | 中文译名 | 本书依据 |
|---|---|---|---|
| **IPA 共通框架 2013** (Common Frame 2013) | 通用框架 2013 / 共通框架 2013 | 系统生命周期全过程框架 | 全部章节的过程、活动、任务、交付物结构 |
| **非功能要求等级 2018 改订版** (NFR Grade) | 非功能要求等级 | 6 大项（可用性 / 性能·扩展性 / 运行·保守性 / 迁移性 / 安全性 / 系统环境）| [06. 非功能设计](06-non-functional-design.md) 全文 |
| **IPA 安全设计评审指南** | IPA 安全设计评审指南 | AISEC-REQ 系列对应 | [07. 安全设计](07-security-design.md) §7.3 |

> **合规范围声明：** 本书对应 IPA 共通框架 2013 的以下过程与交付物：
> - **基本设计** = `系统方式设计过程` + `软件受入测试支持过程` 的设计侧交付物
> - **详细设计** = `软件方式设计过程` + `软件详细设计过程` 的设计侧交付物

详细的过程-活动-任务-交付物对照表见 [Appendix C — IPA 过程·交付物 对照表](appendix-c-ipa-mapping.md)。

## 0.1 目的 / Purpose

本文档定义 AI-Native Engineering Platform（以下简称"本平台"或"Platform"）的**基本设计（外部设计）**。基本设计将需求定义书（[`docs/requirements/00-requirements-definition.md`](../../requirements/00-requirements-definition.md)）中达成的共识，固化为**实现启动前必须确定**的方案、功能、数据、接口、非功能、运维、安全等各项设计判断，作为详细设计（内部设计）的输入。

> **IPA 共通框架 2013 对应过程：** `系统方式设计过程` 的 `A4 编写系统方式设计书`
> **对应任务 (Corresponding Task)：**
> - T4-1 确定系统方式设计书构成
> - T4-2 编写系统方式设计书正文
> - T4-3 实施系统方式设计书评审

## 0.2 范围 / Scope

### 0.2.1 包含 (In Scope for this design document)

- 本平台 MVP 范围（Phase 9 确定的 37 项需求）中，以及与 MVP 直接相关的 V1 邻近需求中，方案设计层面需要确定的内容
- Local-First 与 Cloud-Ready 两种部署形态的方案设计
- 主要 8 个子系统的功能、数据、接口、非功能设计
- IPA 标准 Phase 14 差距分析（F14-1〜F14-9）中 Accepted-and-fixed 部分的反映

### 0.2.2 不包含 (Out of Scope)

- 详细设计（类/函数内部结构、数据结构内部表示、代码级优化）— 后续在 [`../detailed-design/`](../detailed-design/) 单独产出
- 单个页面 UI 规格（mockup、像素级排版）— 由设计规格文档单独管理
- 基准实测值（[06. 非功能设计](06-non-functional-design.md) 与 [08. 运维设计](08-operations-design.md) 中标注 TBD 的项目）
- 法律与合规的最终判定（F14-7 记录的人类干系人正式签核流程的缺位）

> **IPA 共通框架 2013 对应章节：** 本书范围是 `系统方式设计过程` A1〜A3 活动的成果物与 `需求定义过程` 的下游输入的交点。

## 0.3 用语定义 / Terms and Definitions

本章只列 IPA 共通框架相关的重要术语。完整的用语集（含本平台专有术语、略语、命名规约）见 [Appendix D — 用语集](appendix-d-glossary.md)。

| IPA 术语 | 中文 | 本书用途 |
|---|---|---|
| 过程 (Process) | 过程 | 共通框架 2013 的最上层结构（需求定义、系统方式设计、软件方式设计等）|
| 活动 (Activity) | 活动 | 过程内的子单位（A1〜A4）|
| 任务 (Task) | 任务 | 活动内的具体工作步骤 |
| 交付物 (Deliverable) | 交付物 | 过程产出的文档或制品 |
| 系统方式设计书 | 系统方式设计书 | 即本"基本设计书" |
| 程序方式设计书 | 程序方式设计书 | 与"详细设计书"对应 |
| 非功能要求等级 | 非功能要求等级 | IPA 提供的 NFR 评估方法论，本书 [§6](06-non-functional-design.md) 采用 |
| 验收测试 | 验收测试 | 由 `软件受入过程` 覆盖，对应本书 [§10](10-acceptance-test-policy.md) |

## 0.4 参考文档 / Reference Documents

| 类别 | 文档 | 本书主要引用位置 |
|---|---|---|
| 需求定义 | [`docs/requirements/00-requirements-definition.md`](../../requirements/00-requirements-definition.md) | §3, §7, §13-15, §17-47, §48-49, §52-54 |
| 产品原语 | [`docs/requirements/phase6-primitives.md`](../../requirements/phase6-primitives.md) | §2, §5, §7（原语定义）|
| MVP 缩减 | [`docs/requirements/phase9-mvp-reduction.md`](../../requirements/phase9-mvp-reduction.md) | §3, §5, §6（MVP 确定）|
| 方案设计 | [`docs/requirements/phase10-architecture.md`](../../requirements/phase10-architecture.md) | §3, §4, §5, §6, §7（本书输入）|
| IPA 标准差距 | [`docs/requirements/phase14-ipa-compliance-review.md`](../../requirements/phase14-ipa-compliance-review.md) | [06](06-non-functional-design.md), [07](07-security-design.md), [08](08-operations-design.md)（NFR 反映依据）|
| 红队评审 | [`docs/requirements/phase11-red-team.md`](../../requirements/phase11-red-team.md) | [06](06-non-functional-design.md), [07](07-security-design.md)（安全设计强化依据）|
| UX 评审 | [`docs/requirements/phase12-ux-review.md`](../../requirements/phase12-ux-review.md) | [03](03-functional-design.md), [05](05-interface-design.md)（UX 设计）|
| 终审 | [`docs/requirements/phase15-final-audit.md`](../../requirements/phase15-final-audit.md) | 贯穿（一致性核查）|
| **IPA 共通框架 2013**（外部）| [IPA 官网](https://www.ipa.go.jp/sec/publish/tn12-005.html) | 全文依据 |
| **非功能要求等级 2018 改订版**（外部）| [IPA 官网](https://www.ipa.go.jp/sec/softwareengine/rellib/）| [06](06-non-functional-design.md) |
| **詳細设计书（本书姊妹文档）**| [`../detailed-design/`](../detailed-design/) | 全部详细实现规格 |

## 0.5 设计原则 / Design Principles（继承自需求定义书 §13）

实现决策的优先级如下。原则之间冲突时，"更靠左"的原则优先（与需求定义书 §13 保持一致）。

1. **Git-Native** — 对标准 Git 协议的完全兼容不可妥协（GIT-REQ-001）。
2. **Local-First** — 单机、完全离线运行是设计的第一级对象（OPS-REQ-001）。
3. **Cloud-Ready** — Local/Cloud 共用同一代码库与同一数据模型（CLOUD-REQ-001）。
4. **AI-Native** — AI 调用通过单一内部网关抽象（AI-REQ-001）发起。
5. **Agent-Native** — Agent 作为 Node subtype 与人类处于平等地位（AGT-REQ-007）。
6. **Graph-Native** — 新增功能用 Node/Edge/Event/Policy/View 的组合表达（Phase 6 结论）。
7. **Evidence-Native** — "为什么"的陈述通过 Evidence 类型的 Edge 结构化（GRF-REQ-010）。
8. **Human Authority** — 审批在上下文内可见、可即时操作（UX-REQ-003）。

> **IPA 共通框架 2013 对应：** 本设计原则列表 = `需求定义过程 A2 识别干系人要求` 的下游约束 + `系统方式设计过程 A3 选定系统方式` 的选择准则。

此外，Phase 10 中明确的实现哲学（4 个"能"原则）应用于本书所有设计判断：

- **能单体解决，不提前微服务化** — 单一进程能解决的就不做微服务化。
- **能 PostgreSQL 解决，不提前增加数据库** — PostgreSQL 能满足的就不增加其他数据存储。
- **能事件解决，不直接形成服务耦合** — 事件能解决的就不做服务间直接耦合。
- **能标准协议解决，不发明私有协议** — 标准协议能解决的不发明私有协议。

## 0.6 IPA 过程地图 / IPA Process Map

**[PROPOSAL]** 本平台开发生命周期与 IPA 共通框架 2013 的过程对应如下。

```
┌──────────────────────────────────────────────────────────────────────┐
│              IPA 共通框架 2013 (Common Frame 2013)                       │
│                  系统生命周期过程（开发主流程）                         │
│                                                                       │
│   P1: 系统化计划                                                        │
│         ↓                                                              │
│   P2: 需求定义  ←───────────────┐                                     │
│         ↓ (需求定义书)             │                                     │
│   P3: 系统方式设计  ← 本书       │ (反馈/迭代)                          │
│         ↓ (基本设计书 = 本书)       │                                     │
│   P4: 软件方式设计  ←┐           │                                     │
│   P5: 软件详细设计  ←┤ 详细设计书                                  │
│         ↓ (详细设计书)        │  (../detailed-design/)                │
│   P6: 软件构建                                                          │
│         ↓                                                              │
│   P7: 软件受入  ←───── 验收测试  (本书 [§10](10-acceptance-test-policy.md))  │
│                                                                       │
│   本书涉及: P3 + P4 + P5 + P7(测试方针)                                │
└──────────────────────────────────────────────────────────────────────┘
```

**本平台在 IPA 框架中的位置：**

| 阶段 | IPA 过程 | 本平台交付物 | 本书位置 |
|---|---|---|---|
| P2 | `需求定义过程` (Requirements Definition) | 需求定义书 v1.0 | [`../../requirements/`](../../requirements/)（独立） |
| **P3** | **`系统方式设计过程` (System Architecture Design)** | **基本设计书 = 本书** | **本书主体** |
| P4 | `软件方式设计过程` (Software Architecture Design) | 详细设计书 P4 部分 | [`../detailed-design/00-overview.md`](../detailed-design/00-overview.md) |
| P5 | `软件详细设计过程` (Software Detailed Design) | 详细设计书 P5 部分 | 同上 |
| P6 | `软件构建过程` (Software Construction) | 源代码 + 单元测试 | 不在本书范围 |
| P7 | `软件受入过程` (Software Acceptance) | 受入测试结果 | 部分涉及：[10. 受入测试方针](10-acceptance-test-policy.md) |

**P3 任务 → 本书章节 对应：**

| IPA 共通框架 2013 任务 | 本书章节 |
|---|---|
| T-A1-1: 识别功能要求 | [§3 功能设计](03-functional-design.md) |
| T-A1-2: 识别非功能要求 | [§6 非功能设计](06-non-functional-design.md) |
| T-A1-3: 确定系统边界 | [§2 系统方式设计](02-architecture.md) §2.1 逻辑架构 |
| T-A2-1: 研讨硬件·软件构成 | [§2 系统方式设计](02-architecture.md) §2.2-2.6 |
| T-A2-2: 研讨系统功能构成 | [§3 功能设计](03-functional-design.md) |
| T-A2-3: 研讨数据构成 | [§4 数据设计](04-data-design.md) |
| T-A2-4: 研讨接口 | [§5 / §11 接口设计](05-interface-design.md) / [§11 API 设计](11-api-design.md) |
| T-A2-5: 研讨可靠性·性能·运行性 | [§6 非功能设计](06-non-functional-design.md) §6.1-6.3 |
| T-A2-6: 研讨安全性 | [§7 安全设计](07-security-design.md) |
| T-A2-7: 研讨迁移性 | [§9 迁移设计](09-migration-design.md) |
| T-A3-1: 评价系统方式方案 | [Appendix A — 设计依据的可追溯性](appendix-a-traceability.md) |
| T-A4-1: 确定系统方式设计书构成 | 本章 (00) |

## 0.7 本书构成 / Document Structure

本书共 **14 章正文 + 4 附录**，按以下顺序构成。

**正文（按 IPA 共通框架 2013 任务分类）：**

| 编号 | 章节 | 主要内容 | 对应 IPA 任务 |
|---|---|---|---|
| [00](00-introduction.md) | 介绍 | 依据声明 / 目的 / 范围 / 用语 / 参照 / 原则 | T-A4-1 |
| [01](01-system-overview.md) | 系统概述 | 角色 / 定位 / 边界 / 运行环境前提 | T-A1-3 |
| [02](02-architecture.md) | 系统方式设计 | 逻辑 / 部署 / 物理 / 存储 / 进程间 / Git 存储 | T-A2-1 |
| [03](03-functional-design.md) | 功能设计 | 8 子系统功能 | T-A1-1, T-A2-2 |
| [04](04-data-design.md) | 数据设计 | 概念模型 / 物理 schema / 遍历 / 生命周期 | T-A2-3 |
| [05](05-interface-design.md) | 接口设计概述 | 协议栈总览 | T-A2-4 (总览) |
| [06](06-non-functional-design.md) | 非功能设计 | IPA 6 大项等级 | T-A1-2, T-A2-5 |
| [07](07-security-design.md) | 安全设计 | 认证 / 审计 / AI 安全 / 网络 / 密钥 | T-A2-6 |
| [08](08-operations-design.md) | 运维设计 | 部署 / 备份 / 监控 / 日志 | T-A2-5 运行部分 |
| [09](09-migration-design.md) | 迁移设计 | Local → Cloud | T-A2-7 |
| [10](10-acceptance-test-policy.md) | 受入测试方针 | 级别 / DoD / 清单 / 发布 | P7 受入支持 |
| [11](11-api-design.md) | API 设计（独立专章）| HTTP/MCP/Git/Webhook/CLI 全栈契约 | T-A2-4 详细 |
| [12](12-app-group-intercommunication.md) | App 群组信息互通设计（独立专章）| 存储过程 / Outbox / Saga | 补充：多 App 协调 |
| [13](13-app-cluster-and-plugins.md) | **App 集群与可热插拔架构（独立专章）**| App 一级化 / Manifest / 中心事件总线 / 集群 / 升级策略 | 扩展：T-A1-5 / T-A2-6 / T-A2-8 |
| [14](14-admin-ops-ui.md) | **管理员运维界面（独立专章）**| Admin 独立子进程 / 鉴权域分离 / 强审计 | 扩展：T-A2-5 / T-A3-1 |

**附录：**

| 附录 | 内容 | IPA 对应 |
|---|---|---|
| [Appendix A](appendix-a-traceability.md) | 设计依据可追溯性 | T-A3-1 评价的输出 |
| [Appendix B](appendix-b-tbd.md) | 残留 TBD 项目清单 | P3 活动未决项 |
| [Appendix C](appendix-c-ipa-mapping.md) | IPA 过程·交付物 对照表 | T-A4-2 索引 |
| [Appendix D](appendix-d-glossary.md) | 用语集 | 共通框架共通语汇对齐 |

---

**导航 / Navigation:**
[← README](README.md) · [01. 系统概述 →](01-system-overview.md)
