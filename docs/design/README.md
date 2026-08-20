# docs/design/ — 设计文档

设计阶段的产出目录。

## 目录结构 / Directory Layout

```
docs/design/
├── README.md                    # 本文件
├── basic-design/                # 基本设计书 (依据 IPA 通用框架 2013)
│   ├── README.md
│   ├── 00-introduction.md
│   ├── 01-system-overview.md
│   ├── 02-architecture.md
│   ├── 03-functional-design.md
│   ├── 04-data-design.md
│   ├── 05-interface-design.md
│   ├── 06-non-functional-design.md
│   ├── 07-security-design.md
│   ├── 08-operations-design.md
│   ├── 09-migration-design.md
│   ├── 10-acceptance-test-policy.md
│   ├── 11-api-design.md
│   ├── 12-app-group-intercommunication.md
│   ├── 13-app-cluster-and-plugins.md        # App 集群与可热插拔架构
│   ├── 14-admin-ops-ui.md                  # 管理员运维界面
│   ├── appendix-a-traceability.md
│   ├── appendix-b-tbd.md
│   ├── appendix-c-ipa-mapping.md
│   └── appendix-d-glossary.md
└── detailed-design/             # 详细设计书 (内部设计)
    ├── README.md
    ├── 00-overview.md
    ├── 01-data-layer.md
    ├── 02-graph-engine.md
    ├── 03-policy-engine.md
    ├── 04-agent-runtime.md
    ├── 05-ai-gateway.md
    ├── 06-git-server.md
    ├── 07-app-coordination.md
    ├── 08-api-handlers.md
    ├── 09-security-impl.md
    ├── 10-observability.md
    ├── 11-error-handling.md
    ├── 12-app-registry-and-plugin-loader.md   # App Registry + Plugin Loader + 中心事件总线
    └── 13-admin-api-and-ops-ui.md             # Admin API + Ops UI + K8s 部署
```

## 架构决策 / Architecture Decision Records

[`architecture/`](../architecture/) — ADR 类文档。记录关键技术选型决策与拒绝方案。当前文档：

- **[技术选型文档](../architecture/tech-selection.md)** — 主语言 Rust + 全套库选型（对应需求定义书 §53 ADR 列表项 11，状态 Accepted 2026-08-19）
- **[实施前 QA 检查表](../architecture/qa-checklist.md)** — Phase 16 启动前必须走完的 26 项顾虑与疑问（按风险等级分类：🔴 Critical 6 / 🟠 High 8 / 🟡 Medium 9 / 🟢 Low 3）

## 工程过程模型 / Process Model

[`process/`](../process/) — 工程过程模型，明确定义从立项到归档的完整过程、阶段产出物、责任主体与判定规则。

- **[150 阶段工作流](../process/workflow.md)** — 13 主阶段 × 150 任务的瀑布-迭代混合工程过程模型（基于日本 IPA 上流工程共通框架 2013 改编），全部使用中文书写；§4 保留 76 个 IPA 日文工程术语原文作为术语对照。

## 基本设计书 / Basic Design (外部设计)

[`basic-design/`](basic-design/) — 答 "做什么"：功能、数据、接口、非功能、安全。

入口：[`basic-design/README.md`](basic-design/README.md)

**严格按照 IPA 共通框架 2013（独立行政法人情报处理推进机构 / Information-technology Promotion Agency）编写**，所有正文与附录使用中文书写。详细 IPA 位置映射见 [Appendix C — IPA 过程·交付物 对照表](basic-design/appendix-c-ipa-mapping.md)，用语对齐见 [Appendix D — 用语集](basic-design/appendix-d-glossary.md)。

关键内容：
- 5 原语图谱 + 单一 PostgreSQL + 单进程
- 8 子系统功能设计
- IPA 非功能要求等级 6 大项
- [API 设计](basic-design/11-api-design.md) 单独成章
- [App 群组信息互通](basic-design/12-app-group-intercommunication.md)（含存储过程方案）
- [App 集群与可热插拔架构](basic-design/13-app-cluster-and-plugins.md)（App 一级化 / Manifest / 中心事件总线 / 集群 / 升级）
- [管理员运维界面](basic-design/14-admin-ops-ui.md)（独立子进程 / 鉴权域 / 强审计）

**主语言选型：** Rust（edition 2021，MSRV 1.75）+ Tokio + Axum + sqlx + gix（读路径） + shell `git`（写路径）— 完整选型见 [架构 / 技术选型文档](../architecture/tech-selection.md)

## 详细设计书 / Detailed Design (内部设计)

[`detailed-design/`](detailed-design/) — 答 "怎么做"：模块、类、函数、算法、数据结构、并发、错误处理。

入口：[`detailed-design/README.md`](detailed-design/README.md)

关键内容：
- 模块结构 + 语言/运行时约定
- [数据层完整 DDL](detailed-design/01-data-layer.md)（含 AISEC-REQ-009(a) DB role 分离）
- [图谱引擎](detailed-design/02-graph-engine.md)（5 原语、类型注册、递归 CTE）
- [策略引擎](detailed-design/03-policy-engine.md)（RBAC+ABAC、AI 特有策略）
- [Agent 运行时](detailed-design/04-agent-runtime.md)（状态机、工作区、凭证）
- [AI 网关](detailed-design/05-ai-gateway.md)（提供商抽象、提示词清洗）
- [Git 服务器](detailed-design/06-git-server.md)（CLI+libgit2 混合）
- [App 协调](detailed-design/07-app-coordination.md)（存储过程、Outbox、Saga）
- [API 处理器](detailed-design/08-api-handlers.md)（HTTP/MCP/CLI 处理器）
- [安全实现](detailed-design/09-security-impl.md)（信封加密、KEK、TLS）
- [可观测性](detailed-design/10-observability.md)（OTel 指标/追踪/日志）
- [错误处理](detailed-design/11-error-handling.md)（标准化错误框架）
- [App Registry & Plugin Loader](detailed-design/12-app-registry-and-plugin-loader.md)（DDL / 加载器协议 / 中心事件总线实现 / 沙箱）
- [Admin API & Ops UI](detailed-design/13-admin-api-and-ops-ui.md)（端点目录 / 鉴权 / admin_audit / 前端架构 / K8s 部署）

## 命名规约 / Naming Convention

- 章号 2 位 + 英文 kebab-case
- 附录：`appendix-X-<topic>.md`
- 文件名简短不冲突

## 相关 / Related

- 需求定义书：[`../requirements/`](../requirements/)
- 本书直接输入：
  - [`../requirements/00-requirements-definition.md`](../requirements/00-requirements-definition.md) — Baseline v1.0
  - [`../requirements/phase9-mvp-reduction.md`](../requirements/phase9-mvp-reduction.md) — MVP 37 项
  - [`../requirements/phase10-architecture.md`](../requirements/phase10-architecture.md) — 架构输入
  - [`../requirements/phase14-ipa-compliance-review.md`](../requirements/phase14-ipa-compliance-review.md) — IPA 差距分析
