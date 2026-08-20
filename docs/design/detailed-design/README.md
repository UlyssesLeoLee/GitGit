# 详细设计书 — AI-Native Engineering Platform

> **AI-Native Engineering Platform — Detailed Design Document (详细设计书 / 内部设计)**
>
> **依据 / Reference:** IPA 通用框架 2013 "系统详细设计"流程交付物。本书直接消费 [`../basic-design/`](../basic-design/) 的所有决定，把它们细化为可实现的规格。
>
> **状态 / Status:** v1.0 draft. 输入为基本设计书 v1.0 + Phase 1-15 全部需求与红队/UX 评审输出。
>
> **目标读者 / Audience:** 实现工程师（首要）、代码审查者、运维工程师、SRE、未来接手的维护者。
>
> **与基本设计的关系：**
> - 基本设计答 **"做什么"**（功能、数据、接口、非功能）
> - 详细设计答 **"怎么做"**（模块、类、函数、算法、数据结构、并发、错误处理）
> - 详细设计不是"实现"，是实现前的最后一份规格；仍允许一些 [TBD] 用于实现阶段的微调
>
> **标签规约 / Tagging convention (沿用项目规约):**
> - `[FACT]` / `[UNVERIFIED-FACT]` / `[INFERENCE]` / `[PROPOSAL]` / `[TBD]` 同基本设计
> - 额外使用 `[IMPL]` 标记需要实现阶段直接照搬的代码片段

---

## 文件结构 / File Layout

```
docs/design/detailed-design/
├── README.md                          (本文件)
├── 00-overview.md                     模块结构、语言/运行时、跨切关注点
├── 01-data-layer.md                   PostgreSQL 完整 DDL、索引策略、RLS、DB role 分离
├── 02-graph-engine.md                 5 原语实现、类型注册、遍历、缓存
├── 03-policy-engine.md                RBAC/ABAC 评估、决策缓存、AI 策略
├── 04-agent-runtime.md                生命周期状态机、工作区隔离、凭证签发
├── 05-ai-gateway.md                   提供商抽象、提示清洗、密钥过滤
├── 06-git-server.md                   CLI+libgit2 混合、钩子回调
├── 07-app-coordination.md             存储过程接口、Outbox relay、Saga engine、App 命名空间
├── 08-api-handlers.md                 HTTP/MCP/CLI 处理器结构
├── 09-security-impl.md                信封加密实现、网络策略
├── 10-observability.md                OTel 仪表化、指标、追踪
├── 11-error-handling.md               错误代码、恢复、死信
├── 12-app-registry-and-plugin-loader.md   App Registry、Plugin Loader 进程、中心事件总线实现
└── 13-admin-api-and-ops-ui.md             Admin API 端点目录、admin_audit、Ops UI 前端架构、K8s 部署清单
```

---

## 章节导航

- [00 → 总体概述](00-overview.md)
- [01 → 数据层](01-data-layer.md)
- [02 → 图谱引擎](02-graph-engine.md)
- [03 → 策略引擎](03-policy-engine.md)
- [04 → Agent 运行时](04-agent-runtime.md)
- [05 → AI 网关](05-ai-gateway.md)
- [06 → Git 服务器](06-git-server.md)
- [07 → App 协调](07-app-coordination.md)
- [08 → API 处理器](08-api-handlers.md)
- [09 → 安全实现](09-security-impl.md)
- [10 → 可观测性](10-observability.md)
- [11 → 错误处理](11-error-handling.md)
- [12 → App Registry & Plugin Loader](12-app-registry-and-plugin-loader.md)
- [13 → Admin API & Ops UI](13-admin-api-and-ops-ui.md)

---

## 与基本设计章节的对应关系

| 基本设计章节 | 详细设计章节 |
|---|---|
| §02 架构 / §03 功能设计 | [00 总体概述](00-overview.md) 模块划分 |
| §04 数据设计 | [01 数据层](01-data-layer.md) 完整 DDL |
| §03.2 Engineering Graph / §04 数据 | [02 图谱引擎](02-graph-engine.md) |
| §03.7 Security / §07 安全 | [03 策略引擎](03-policy-engine.md) + [09 安全实现](09-security-impl.md) |
| §03.5 Agent Runtime | [04 Agent 运行时](04-agent-runtime.md) |
| §03.3 AI Gateway / §07.3 AI 安全 | [05 AI 网关](05-ai-gateway.md) |
| §03.1 Git Server | [06 Git 服务器](06-git-server.md) |
| §12 App 群组 | [07 App 协调](07-app-coordination.md) |
| §11 API 设计 | [08 API 处理器](08-api-handlers.md) |
| §06 非功能 / §08 运维 | [10 可观测性](10-observability.md) + [11 错误处理](11-error-handling.md) |
| **§13 App 集群与可热插拔架构** | **[12 App Registry & Plugin Loader](12-app-registry-and-plugin-loader.md)** |
| **§14 管理员运维界面** | **[13 Admin API & Ops UI](13-admin-api-and-ops-ui.md)** |

---

## 重要前置条件

1. **主语言已选定为 Rust**（[架构 / 技术选型文档](../../architecture/tech-selection.md)，状态 Accepted 2026-08-19）— 本书中的代码示例使用 Rust 风格伪代码/类型化伪代码（所有语言特定的 API 都在 `[IMPL]` 代码块中以注释标注目标语言）；详细实现将使用具体 Rust crate（sqlx / Axum / gix / Tonic 等）
2. **PostgreSQL 14+** 是确定的（基本设计 [§2.4](../basic-design/02-architecture.md#24-存储架构-storage-architecture采用-phase-10-1)）
3. **5 原语**作为图谱核心（基本设计 [§3.2](../basic-design/03-functional-design.md#32-工程图谱子系统-engineering-graph-subsystem)）— 详细设计的所有数据结构都从这出发
4. **AISEC-REQ-009(a) MVP non-negotiable**（[§7.2](../basic-design/07-security-design.md#72-审计-audit)）— DB role 分离设计在 [01 数据层](01-data-layer.md) 详写
5. **Git 写路径强约束**：禁止为"纯 Rust"重写，必须用 shell `git` 进程（[技术选型文档 §7.2](../../architecture/tech-selection.md#72-强约束来自-phase10-architecturemd-6-第-2-项)）

---

**文档结尾说明：** 本书为 v1.0 draft，作为实现阶段直接照搬的规格。实现过程中如有偏差必须更新本书。
