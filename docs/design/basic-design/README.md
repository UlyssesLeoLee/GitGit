# 基本设计书 — AI-Native Engineering Platform

> **AI-Native Engineering Platform — Basic Design Document (基本设计书)**
>
> **依据框架 / Framework Compliance:** 严格按照 **IPA 共通框架 2013**（独立行政法人情报处理推进机构 / Information-technology Promotion Agency 发布的系统生命周期通用框架）编写。采用 IPA/SEC《非功能要求等级 (2018 改订版)》的 6 大项分类，并整合 Phase 14 IPA 标准差距分析 (F14-1〜F14-9) 的 Accepted-and-fixed 反映。**全部章节使用中文书写**。
>
> **状态 / Status:** v1.0 draft. 输入源：`docs/requirements/00-requirements-definition.md` (Baseline v1.0)、`phase6-primitives.md`、`phase9-mvp-reduction.md`、`phase10-architecture.md`、`phase14-ipa-compliance-review.md`。
>
> **目标读者 / Audience:** 内部架构师 / 实现工程师 / 未来外部受托评审方。同时面向实现方、运维方、安全审计方三类读者。
>
> **IPA 共通框架 2013 位置：** 本书对应 `系统方式设计过程`（P3）的全部 4 活动 + `软件受入过程`（P9）的支持部分。详细对应见 [00. 介绍 §0.6 IPA 过程地图](00-introduction.md#06-ipa-过程地图-ipa-process-map) 与 [Appendix C — IPA 过程·交付物 对照表](appendix-c-ipa-mapping.md)。
>
> **标签规约 / Tagging convention (沿用项目既有约定):**
> - `[FACT]` — 一次源验证过的事实
> - `[UNVERIFIED-FACT]` — 多个二次源印证但本会话未取得一次源
> - `[INFERENCE]` — 基于已知事实的合理推断
> - `[PROPOSAL]` — 本项目自身的设计主张（不归因于外部资料）
> - `[TBD]` — 未确定（后续阶段 / 待决策）

---

## 文件结构 / File Layout

本书按 IPA 共通框架 2013 的过程活动分类拆分为 **14 章正文 + 4 附录**。

**正文：**

| 文件 | 内容 | IPA 共通框架 2013 任务 |
|---|---|---|
| [00-introduction.md](00-introduction.md) | 依据声明 / 目的 / 范围 / 用语 / 参照 / 原则 / IPA 过程地图 | T-A4-1 设计书构成 |
| [01-system-overview.md](01-system-overview.md) | 系统概述（角色 / 定位 / 边界 / 运行环境前提） | T-A1-3 系统边界 / T-A1-5 系统构成要素 |
| [02-architecture.md](02-architecture.md) | 系统方式设计（逻辑 / 部署 / 物理 / 存储 / 进程间 / Git 存储） | T-A2-1 硬件软件构成 |
| [03-functional-design.md](03-functional-design.md) | 功能设计（8 子系统） | T-A1-1 功能要求 / T-A2-2 功能构成 |
| [04-data-design.md](04-data-design.md) | 数据设计（概念模型 / 物理 schema / 遍历 / 生命周期 / 移植性） | T-A1-6 数据模型 / T-A2-3 数据构成 |
| [05-interface-design.md](05-interface-design.md) | 接口设计概述（详情见 11）| T-A2-4 接口构成（总览）|
| [06-non-functional-design.md](06-non-functional-design.md) | 非功能设计（IPA 6 大项 + 集群/插件/Admin 扩展）| T-A1-2 非功能要求 / T-A2-5 可靠性/性能/运行性 |
| [07-security-design.md](07-security-design.md) | 安全设计（认证 / 审计 / AI 安全 / 网络 / 密钥 / 信任边界 / App 沙箱 / Admin 鉴权域） | T-A2-6 安全方式 |
| [08-operations-design.md](08-operations-design.md) | 运维设计（部署 / 备份 / 监控 / 日志 / 支持 / V1+ K8s） | T-A2-5 运行性 |
| [09-migration-design.md](09-migration-design.md) | 迁移设计（Local → Cloud）| T-A2-7 迁移方式 |
| [10-acceptance-test-policy.md](10-acceptance-test-policy.md) | 受入测试方针（级别 / MVP DoD / 清单 / 发布判定 / 回归） | P9 软件受入过程 全部活动 |
| [11-api-design.md](11-api-design.md) | API 设计（HTTP/MCP/Git/Webhook/CLI 全栈契约 + Admin/Plugin 端点族） | T-A2-4 接口构成（详细）|
| [12-app-group-intercommunication.md](12-app-group-intercommunication.md) | App 群组信息互通设计（存储过程 / Outbox / Saga） | 补充：P3.A2.T8 App 协调方式 |
| **[13-app-cluster-and-plugins.md](13-app-cluster-and-plugins.md)** | **App 集群与可热插拔架构（App 一级化 / Manifest / 中心事件总线 / 集群 / 升级）** | **扩展：T-A1-5 / T-A2-6 / T-A2-8** |
| **[14-admin-ops-ui.md](14-admin-ops-ui.md)** | **管理员运维界面（独立子进程 / 鉴权域 / 强审计）** | **扩展：T-A2-5 / T-A3-1** |

**附录：**

| 附录 | 内容 | IPA 共通框架 2013 对应 |
|---|---|---|
| [Appendix A — 可追溯性](appendix-a-traceability.md) | 设计依据可追溯性 | T-A3-1 方式评价的输出 |
| [Appendix B — 残留 TBD](appendix-b-tbd.md) | 残留 TBD 项目清单 | P3 活动未决项 |
| **[Appendix C — IPA 过程·交付物 对照表](appendix-c-ipa-mapping.md)** | **本书每一章在 IPA 框架中的位置** | **T-A4-2 索引（必须）** |
| **[Appendix D — 用语集](appendix-d-glossary.md)** | **IPA 标准术语 + 本平台专有术语 + 略语** | **共通语汇对齐（必须）** |

---

## 章节导航 / Cross-Chapter Navigation

- [00 → 介绍](00-introduction.md)
- [01 → 系统概述](01-system-overview.md)
- [02 → 系统方式设计](02-architecture.md)
- [03 → 功能设计](03-functional-design.md)
- [04 → 数据设计](04-data-design.md)
- [05 → 接口设计概述](05-interface-design.md)
- [06 → 非功能设计](06-non-functional-design.md)
- [07 → 安全设计](07-security-design.md)
- [08 → 运维设计](08-operations-design.md)
- [09 → 迁移设计](09-migration-design.md)
- [10 → 受入测试方针](10-acceptance-test-policy.md)
- [11 → API 设计](11-api-design.md)
- [12 → App 群组信息互通设计](12-app-group-intercommunication.md)
- [13 → App 集群与可热插拔架构](13-app-cluster-and-plugins.md)
- [14 → 管理员运维界面](14-admin-ops-ui.md)
- [Appendix A → 可追溯性](appendix-a-traceability.md)
- [Appendix B → 残留 TBD](appendix-b-tbd.md)
- [Appendix C → IPA 过程·交付物 对照表](appendix-c-ipa-mapping.md)
- [Appendix D → 用语集](appendix-d-glossary.md)

---

**文档结尾说明：** 本书为 v1.0 draft，作为本项目实现阶段决策的基础资料。正式版的晋升须经需求定义书 §54 的 Open Questions 群解决后，由 Phase 16 (实现) 启动决策完成。
