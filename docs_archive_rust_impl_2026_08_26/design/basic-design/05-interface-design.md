# 05. 接口设计概述 / Interface Design (Overview)

> **[PROPOSAL] IPA 共通框架 2013 章节定位：** P3.A2.T4 研讨接口构成 (总览)


> **本章为概览章。** API 设计的完整规范、契约、版本治理、限流策略、错误格式、OpenAPI 治理等已在 **[11. API 设计](11-api-design.md)** 单独详述。本章仅保留高层接口目录与协议栈选择，便于初次了解。

## 5.1 协议栈总览

| 协议 | 端点形态 | 目标用户 | 详细章节 |
|---|---|---|---|
| HTTP(S) REST + JSON | `/api/v{MAJOR}/...` | 人类开发者、CI 脚本、外部服务 | [§11.3 API 设计](11-api-design.md#113-http-api-设计) |
| MCP over HTTP/JSON-RPC | `/mcp/v{MAJOR}/...` | 外部 AI Agent（Codex / Claude Code / Cursor 等）| [§11.4 MCP 设计](11-api-design.md#114-mcp-设计) |
| Git Smart HTTP | `/git/{repo}.git/...` | Git CLI、CI runner | [§11.5 Git 协议 API](11-api-design.md#115-git-协议-api) |
| SSH | `git@host:{repo}.git` | Git CLI | [§11.5 Git 协议 API](11-api-design.md#115-git-协议-api) |
| Webhook (出站) | 用户注册回调 URL | 外部系统（CI、通知、IM）| [§11.6 Webhook](11-api-design.md#116-webhook-outbound-api) |
| CLI (子进程) | `platform {verb} {target} --flag=...` | 运维、自动化 | [§11.7 CLI API](11-api-design.md#117-cli-api) |
| 存储过程 RPC | `/api/v1/coordinate/{proc-name}` | 跨 App 群组协调 | [§12.4 存储过程协调](12-app-group-intercommunication.md#124-模式-①存储过程作为协调中枢) |

## 5.2 HTTP API 端点目录（概要）

| Method | Path | 用途 | 必要权限 |
|---|---|---|---|
| GET | `/api/v1/repos` | 仓库列表 | 认证必填 |
| POST | `/api/v1/repos` | 创建仓库 | `repo:create` |
| GET | `/api/v1/repos/{id}/issues` | Issue 列表 | 仓库读 |
| POST | `/api/v1/repos/{id}/issues` | 创建 Issue | 仓库写 |
| GET | `/api/v1/agents` | Agent 列表 | `agent:read` |
| POST | `/api/v1/agents/{id}/runs` | 启动 Agent | `agent:invoke` + policy gate |
| GET | `/api/v1/agent-runs/{id}` | Agent 运行状态 | `agent:read` |
| POST | `/api/v1/approvals/{pending_id}` | 人类审批 | `approval:act` |
| GET | `/api/v1/graph/nodes/{id}` | 节点读取 | Policy |
| GET | `/api/v1/graph/traverse` | 图查询（递归 CTE）| Policy |
| GET | `/api/v1/views/{view_id}/invoke` | 视图调用 | Policy |
| GET | `/api/v1/audit/events` | 审计事件查询 | `audit:read` |
| POST | `/api/v1/coordinate/{proc-name}` | 存储过程 RPC | Policy + 群组配额 |

所有写入端点必须先经 Policy 评估再执行。评估失败返回 403 + 标准化错误码。详细错误格式、幂等、限流、版本治理见 [§11.3](11-api-design.md#113-http-api-设计)。

## 5.3 Git 协议

- HTTP（smart HTTP）— 主流
- SSH — `git@host:repo.git` 形式
- `git://` — MVP 不支持（V1 及以后再考虑）

## 5.4 MCP 服务器（V1+）

**[PROPOSAL]** V1 实现的 MCP 服务器向 Agent 公开以下 tool 集合：

- `read_node` / `write_node` / `traverse_graph`
- `read_issue` / `create_issue` / `update_issue`
- `read_pr` / `create_pr` / `request_review`
- `read_repo` / `list_repos`
- `invoke_agent`（需人类审批的操作）
- `get_view` / `search`

所有 tool 调用经 Policy 评估。Agent 凭据范围之外的 tool 不可访问。完整 tool 列表、授权模型、tool 描述治理见 [§11.4](11-api-design.md#114-mcp-设计)。

## 5.5 Webhook（V1+）

- 基于事件的投递（push, issue.created, pr.merged, agent.completed 等）
- 投递重试（指数退避、签名 payload）
- 投递日志作为 Event 记录

详细重试策略、签名、失败回查见 [§11.6](11-api-design.md#116-webhook-outbound-api)。

## 5.6 UI 概述 / UI Overview

**[PROPOSAL]** MVP UI 是 Web SPA（框架待实现阶段决定，TBD）。主要页面：

1. **Dashboard** — 个人相关 Issue / PR / AgentRun / Review
2. **Repository 详情** — 代码树、Issue 列表、CI 状态、图视图
3. **Issue / PR 详情** — 评论、评审、相关图谱
4. **Agent Run 详情** — 执行日志、产物、审批按钮（UX-REQ-003）
5. **Graph Explorer** — 节点 / 边的可视化与查询
6. **Policy 控制台** — Policy 规则定义（管理员）
7. **Audit Log 查看器** — 事件搜索（管理员）

UX-REQ-003 的人类权威原则要求，Agent 审批在对应 PR / AgentRun 页面上一键可操作。

---

**导航 / Navigation:**
[← 04. 数据设计](04-data-design.md) · [README](README.md) · [06. 非功能设计 →](06-non-functional-design.md)
