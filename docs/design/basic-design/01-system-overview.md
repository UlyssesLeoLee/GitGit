# 01. 系统概述 / System Overview

> **[PROPOSAL] IPA 共通框架 2013 章节定位：** P3.A1.T3 确定系统边界 / P3.A1.T5 识别系统构成要素


## 1.1 系统名称与目标 / System Name & Target

**名称：** AI-Native Engineering Platform（暂定名，正式名将在 PRD 中确定）

**目标用户 / Personas（来自需求定义书 §5）：**

| 角色 | 主要图操作 | 核心需求 |
|---|---|---|
| Developer | Commit/PR/Review/Issue 节点 | 快速可预期的 Git/PR 工作流；不藏信息的 AI 辅助 |
| AI Agent | Agent/AgentRun 节点 + Policy 控制的 Action | 明确的范围、最小必要上下文、不阻塞小事的审批路径 |
| Reviewer | Review 节点 + Evidence 边 | "测试通过"和"策略满足"是图上的事实，无需手验 |
| Architect | ADR/Requirement 节点，`depends_on`/`supersedes` 边 | 决策影响的传播路径可查询，而不是靠记忆或 grep |
| Manager | 跨 Issue/PR/AgentRun 的视图 | 单一可信源，不依赖五个不同工具的导出对账 |
| 事故响应者 | Incident 节点 + `caused_by` 边 | 从症状到根因（含 Agent 操作的提交）的高速追溯 |

## 1.2 系统定位 / System Positioning

继承需求定义书 §4 的定位。简述如下：

> **"自托管（Self-hostable）且云就绪（Cloud-ready），以贯穿 Requirement → ADR → Issue → PR → Test → Release → Incident → AgentRun 的一级工程知识图谱为核心工程系统记录。"**

差异化主轴（依据 Phase 5 Gap Analysis）：

1. 贯穿全生命周期的一级知识图谱（把 Git 特性从 commodity 升格为"图谱宿主"）
2. 真正的自托管 + 云一致 + Agent 原生执行
3. 人类与 Agent 在结构上平等对待的统一审计轨迹

## 1.3 系统边界 / System Boundary

**包含 (in scope):**

- Git 托管（clone、fetch、push、branch、tag、merge、rebase）
- Issue、PR、Review（含评论线程）的图谱化
- 单工作流内的 CI 执行及其结果的图谱化
- 图查询 API（MCP、HTTP、CLI）
- AI 调用的内部网关
- 上下文引擎（带边界、可审计）
- Agent 运行时（生命周期、Policy 门控、隔离执行）
- 认证授权（RBAC/ABAC）与结构化审计

**不包含 (out of scope — 详见需求定义书 §15):**

- Wiki / 通用文档产品
- Jira/Linear 级别的完整 PM 套件
- Slack 级别的聊天/通话
- Codespaces 级别的完整云端 IDE
- 自建 Kubernetes 集群管理产品
- 内置 IDE / 代码编辑器
- 自主基础模型的开发/托管
- 非 Git VCS 后端

## 1.4 运行环境前提 / Operating Environment Assumptions

**[PROPOSAL]** 本书采用以下前提：

| 项目 | 前提 | 依据 |
|---|---|---|
| 支持的操作系统 | Linux（x86_64 / aarch64）、macOS（aarch64）、Windows（x86_64）| OPS-REQ-001（自托管） / Phase 14 F14-5（对应矩阵）|
| 运行模式 | Local（单二进制 + Docker Compose）/ Cloud（容器编排器上）| CLOUD-REQ-001 |
| 运行时依赖 | 系统 `git` 2.30+、libc 兼容、PostgreSQL 14+ | [02. 系统方式设计](02-architecture.md) §2.3 决策 |
| AI 调用 | 配置的提供商（MVP：单一默认）| AI-REQ-001（Phase 9 缩减后）|
| 网络 | Local 部署可完全离线运行（OPS-REQ-001）| OPS-REQ-001 |

支持平台的正式矩阵将在 [08. 运维设计 §8.1.3](08-operations-design.md#813-支持的操作系统矩阵nfr-req-003-v1-正式化) 与 Phase 14 F14-5（NFR-REQ-003，V1 制定）中后续确定。

---

**导航 / Navigation:**
[← 00. 介绍](00-introduction.md) · [README](README.md) · [02. 系统方式设计 →](02-architecture.md)
