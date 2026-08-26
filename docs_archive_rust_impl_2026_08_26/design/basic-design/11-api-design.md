# 11. API 设计 / API Design

> **[PROPOSAL] IPA 共通框架 2013 章节定位：** P3.A2.T4 研讨接口构成 (详细)


> 本章从原 `05-interface-design.md` 中抽出并扩展，覆盖 HTTP/MCP/Webhook/CLI 全部对外 API 面的设计规范、契约、版本治理与跨切关注点。原 `05` 保留为接口概览，详情以本章为准。

## 11.1 设计目标与原则

**目标：** 提供一套**机器可读、人类可写、长期可演进**的对外 API 表面，让人类开发者、外部 Agent、外部服务都能以**一致的方式**调用平台能力。

**原则（依本平台 8 大设计原则延伸）：**

1. **契约优先 (Contract-first)** — 所有公开端点必须在 OpenAPI 3.1 规范中先存在，再实现。规范即文档，规范即测试 fixture 源。
2. **可演进 (Evolvable)** — URI 中必须包含版本号（`/api/v1/...`）。同一资源的语义变更必须走新版本，禁止原地 breaking change。
3. **幂等 (Idempotent)** — 所有非 GET 写操作必须支持 `Idempotency-Key` 头，避免网络重试导致重复副作用。
4. **可观测 (Observable)** — 每个请求都带 trace id（`X-Trace-Id` 头），结构化记录到 `events` 表，跨服务可串联。
5. **Policy 在前 (Policy-first)** — 端点本身只做协议解析与参数校验，**业务级授权判断全部由 Policy 引擎承担**（参见 [§3.7 Security & Access Control Subsystem](03-functional-design.md#37-安全与访问控制子系统-security-access-control-subsystem)）。
6. **错误标准化 (Standardized errors)** — 错误响应体格式跨所有端点统一，便于客户端统一处理。

## 11.2 协议栈总览

| 协议 | 端点形态 | 目标用户 | 章节 |
|---|---|---|---|
| HTTP(S) REST + JSON | `/api/v{MAJOR}/...` | 人类开发者、CI 脚本、外部服务 | [§11.3](#113-http-api-设计) |
| MCP over HTTP/JSON-RPC | `/mcp/v{MAJOR}/...` | 外部 AI Agent（Codex/Claude Code/Cursor 等）| [§11.4](#114-mcp-设计) |
| Git Smart HTTP | `/git/{repo}.git/...` | Git CLI、CI runner | [§11.5](#115-git-协议-api) |
| SSH | `git@host:{repo}.git` | Git CLI | [§11.5](#115-git-协议-api) |
| Webhook (outbound) | 用户注册回调 URL | 外部系统（CI、通知、IM）| [§11.6](#116-webhook-outbound-api) |
| CLI (subprocess) | `platform {verb} {target} --flag=...` | 运维、自动化 | [§11.7](#117-cli-api) |

**版本号策略：** `{MAJOR}` 表示不兼容变更（必须新前缀）；`{MINOR}` 在 OpenAPI 3.1 中体现（同一前缀下加字段）；`{PATCH}` 仅文档/示例。**当前稳定版本：v1**。v0 阶段（开发期）可省略，但 GA 后必须显式带版本。

## 11.3 HTTP API 设计

### 11.3.1 资源命名与 URI 设计

**[PROPOSAL]** URI 设计遵循以下约定：

| 约定 | 示例 | 备注 |
|---|---|---|
| 名词复数，kebab-case | `/api/v1/repos`, `/api/v1/agent-runs` | 避免动词、驼峰 |
| 嵌套深度 ≤ 3 层 | `/api/v1/repos/{repoId}/issues/{issueId}` | 超过则改为顶层资源 + ID 引用 |
| ID 用 UUID v7 (时间排序) | `/api/v1/repos/0190e8a4-...` | 见 [§11.3.5](#1135-id-设计) |
| 集合端点支持过滤 | `?state=open&assignee=me&page=2&pageSize=50` | 过滤参数必须预先在 OpenAPI 声明 |
| 集合端点 cursor 分页 | `?cursor={opaque}&pageSize=50` | 见 [§11.3.6](#1136-分页) |
| 关联端点用 `/relationships/` | `/api/v1/prs/{id}/relationships/reviewers` | 替代部分嵌套 |

### 11.3.2 HTTP 方法语义

| Method | 幂等 | 安全 | 用途 | 成功返回 |
|---|---|---|---|---|
| `GET` | ✓ | ✓ | 读 | 200 + 资源 |
| `POST` | ✗（除非带 Idempotency-Key）| ✗ | 创建 | 201 + 资源 + `Location` 头 |
| `PUT` | ✓ | ✗ | 整体替换 | 200/204 |
| `PATCH` (JSON Merge Patch) | ✗ | ✗ | 局部更新 | 200 + 资源 |
| `DELETE` | ✓ | ✗ | 删除（逻辑删除）| 204 |
| `HEAD` | ✓ | ✓ | 元信息（存在性、`ETag`）| 头 only |

### 11.3.3 请求与响应头

**必备请求头：**

| Header | 必填 | 说明 |
|---|---|---|
| `Authorization` | ✓ (除 public 端点) | 参见 [§7.1 认证授权](07-security-design.md#71-认证-授权-authentication-authorization) |
| `X-Trace-Id` | 自动生成 | 客户端可主动提供；缺失则服务端生成 UUID v7 |
| `Idempotency-Key` | 写操作推荐 | 写操作的幂等键，详见 [§11.3.7](#1137-幂等性) |
| `Accept` | 推荐 | 显式声明接受 `application/json` |
| `Content-Type` | 写操作必填 | 写操作必填 `application/json` 或 `application/merge-patch+json` |

**必备响应头：**

| Header | 说明 |
|---|---|
| `X-Trace-Id` | 回显请求 trace id |
| `X-Request-Id` | 平台内部的请求 ID（与 trace id 区分） |
| `RateLimit-Limit` / `RateLimit-Remaining` / `RateLimit-Reset` | 限流状态（V1 启用） |
| `ETag` | GET/PUT 资源用，弱 ETag |
| `Location` | POST 201 时新资源 URI |
| `Deprecation` + `Sunset` | 弃用端点（RFC 8594） |

### 11.3.4 错误响应格式

**[PROPOSAL]** 所有错误响应体统一为：

```json
{
  "error": {
    "code": "policy_denied",
    "message": "Agent credential does not have 'merge' scope on this repository",
    "details": {
      "policy_id": "0190e8a4-...",
      "actor_id": "0190e8a4-...",
      "resource_id": "0190e8a4-...",
      "action": "merge",
      "required_scope": "merge"
    },
    "trace_id": "0190e8a4-...",
    "documentation_url": "https://docs.example.com/errors/policy_denied"
  }
}
```

**错误码命名规范：** `{domain}_{reason}`，全小写，下划线分隔。前端可基于 `code` 编程决策，`message` 给人看。

| 域 | 示例 code | HTTP 状态 |
|---|---|---|
| 输入验证 | `validation_required_field_missing`, `validation_invalid_uuid` | 400 |
| 认证 | `auth_missing_credentials`, `auth_invalid_token`, `auth_expired` | 401 |
| 授权 | `policy_denied`, `policy_evaluation_timeout` | 403 |
| 资源 | `resource_not_found`, `resource_already_exists`, `resource_state_conflict` | 404 / 409 |
| 限流 | `rate_limit_exceeded` | 429 |
| 上游 | `git_protocol_error`, `postgres_unavailable` | 502 / 503 |
| 内部 | `internal_error` (兜底，不暴露细节) | 500 |

### 11.3.5 ID 设计

| 资源 | ID 类型 | 生成方 | 排序性 |
|---|---|---|---|
| Node, Edge, Policy, View 等内部对象 | UUID v7 | PostgreSQL `gen_random_uuid()` (v15+) 或应用层 `uuid_v7()` | 时间排序（有利于 B-tree 局部性） |
| Event `seq` | BIGSERIAL | PostgreSQL `bigserial` | 严格单调，与 UUID 不同（Event 不需要去中心化） |
| 仓库人类可读路径 | slug | 仓库创建时显式指定 | N/A（路径即身份）|
| 跨服务引用 | UUID v7 | 创建方 | 时间排序 |

**为何不用 UUID v4：** v7 自带时间前缀，在 PostgreSQL B-tree 索引上比 v4 显著减少 page split，对事件日志 / 节点表的写入吞吐有可测量影响。Event 用 BIGSERIAL 单独走是因为它需要严格单调（用于 replay 锚点）。

### 11.3.6 分页

**[PROPOSAL]** 统一使用 **cursor-based 分页**，不推荐 offset（大数据集上 offset 性能差且不稳定）。

```
GET /api/v1/repos?pageSize=50&cursor=eyJzb3J0IjpbImNyZWF0ZWRfYXQiXSwidmFsdWUiOlsiMjAyNi0wOC0xOVQwNjoxODowMFoiXX0
```

**响应体：**
```json
{
  "data": [ ... 50 items ... ],
  "pagination": {
    "next_cursor": "eyJ...",
    "has_more": true
  }
}
```

**默认/最大 pageSize：** 默认 20，最大 100。超过返回 400 `validation_page_size_too_large`。

**排序：** 默认按 ID 倒序（最新在前）。所有集合端点必须支持 `?sort=field:direction`。

### 11.3.7 幂等性

**[PROPOSAL]** 所有非 GET 写操作接受 `Idempotency-Key` 头（建议必填，v1 强制要求在 [§11.3.7.1](#11371-强制场景) 场景下）。

**机制：**
1. 客户端生成唯一 key（建议 UUID v7）随请求发出
2. 服务端在 `idempotency_keys` 表记录 (key, endpoint, request_hash, response_status, response_body, expires_at)
3. 24 小时内同 key + 同 request_hash：直接返回缓存响应，不重放副作用
4. 同 key + 不同 request_hash：返回 422 `idempotency_key_conflict`
5. key 过期（>24h）：视为新请求

**强制场景（v1 必须，其余推荐）：**
- 资金/配额类操作（AI 推理、CI 启动计费）
- 不可逆副作用（merge、deployment、secret rotation）
- 批量操作（`POST /api/v1/agent-runs:batch`）

### 11.3.8 版本治理

| 规则 | 说明 |
|---|---|
| 路径版本化 | 所有公开端点必须 `/api/v{MAJOR}/...` |
| v1 是稳定的 | v1 进入维护后只接受 backward-compatible 变更 |
| 新字段 = minor 变更 | 不需要新版本，直接在 OpenAPI 中加 optional 字段 |
| 行为变更 = major 变更 | 必须新版本 `v2/...`；旧版本可继续运行 N 个月（Sunset 头） |
| 弃用流程 | 端点先标记 `Deprecation: true` + `Sunset: <date>`（至少 6 个月预告期），再下线 |
| OpenAPI 单一来源 | `/api/v1/openapi.json` 暴露完整规范；SDK 生成、Mock server、Contract test 都从这拉 |

### 11.3.9 主要端点目录（v1）

| Method | Path | 用途 | 权限 |
|---|---|---|---|
| GET | `/api/v1/repos` | 仓库列表 | 认证必填 |
| POST | `/api/v1/repos` | 创建仓库 | `repo:create` |
| GET | `/api/v1/repos/{repoId}` | 仓库详情 | 仓库读权限 |
| GET | `/api/v1/repos/{repoId}/issues` | Issue 列表 | 仓库读权限 |
| POST | `/api/v1/repos/{repoId}/issues` | 创建 Issue | 仓库写权限 |
| GET | `/api/v1/repos/{repoId}/issues/{issueId}` | Issue 详情 | 仓库读权限 |
| PATCH | `/api/v1/repos/{repoId}/issues/{issueId}` | 更新 Issue | 仓库写权限 |
| GET | `/api/v1/repos/{repoId}/pulls` | PR/MR 列表 | 仓库读权限 |
| POST | `/api/v1/repos/{repoId}/pulls` | 创建 PR | 仓库写权限 |
| POST | `/api/v1/repos/{repoId}/pulls/{prId}/merge` | 合并 PR | `pr:merge` + Policy |
| GET | `/api/v1/graph/nodes/{nodeId}` | 图节点查询 | Policy |
| GET | `/api/v1/graph/traverse` | 图遍历（再帰 CTE） | Policy + 深度限制 |
| POST | `/api/v1/graph/nodes` | 创建节点 | Policy + 类型白名单 |
| GET | `/api/v1/agents` | Agent 列表 | `agent:read` |
| POST | `/api/v1/agents/{agentId}/runs` | 启动 Agent | `agent:invoke` + Policy |
| GET | `/api/v1/agent-runs/{runId}` | Agent 运行状态 | `agent:read` |
| POST | `/api/v1/agent-runs/{runId}/cancel` | 取消 Agent | `agent:cancel` |
| GET | `/api/v1/policies` | Policy 规则列表 | `policy:read` |
| POST | `/api/v1/policies` | 新建 Policy 规则 | `policy:write` |
| GET | `/api/v1/audit/events` | 审计事件查询 | `audit:read` |
| GET | `/api/v1/views/{viewId}/invoke` | 视图调用 | Policy |
| POST | `/api/v1/webhooks` | 注册 Webhook | `webhook:write` |
| GET | `/api/v1/healthz` | Liveness | public |
| GET | `/api/v1/readyz` | Readiness | public |

完整端点（含请求/响应 schema）见仓库 `/api/v1/openapi.json`。

## 11.4 MCP 设计

### 11.4.1 定位

**[PROPOSAL]** 平台在 V1 充当 **MCP Server**，将图遍历、视图调用、Agent 启动等能力暴露为标准 MCP tools，供外部 AI Agent（Codex、Claude Code、Cursor 等）调用。本平台**不**是 MCP Client（不主动调外部 MCP 工具），除非后续 AGT-REQ-009 委派场景需要。

### 11.4.2 工具集（v1 候选）

| Tool 名 | 用途 | 输入 schema | 输出 |
|---|---|---|---|
| `read_node` | 按 ID 读节点 | `{node_id: UUID}` | Node 对象 |
| `traverse_graph` | 图遍历 | `{start_id, edge_types, max_depth, filter}` | 节点列表 |
| `list_issues` | 列 Issue | `{repo_id, state?, assignee?, page}` | Issue 列表 |
| `create_issue` | 建 Issue | `{repo_id, title, body, labels?}` | Issue 对象 |
| `read_pr` | 读 PR | `{pr_id}` | PR 对象 |
| `create_pr` | 建 PR | `{repo_id, head, base, title, body}` | PR 对象 |
| `request_review` | 请求评审 | `{pr_id, reviewers}` | 204 |
| `read_repo` | 读仓库 | `{repo_id}` | Repo 对象 |
| `list_repos` | 列仓库 | `{org_id?, page}` | Repo 列表 |
| `invoke_agent` | 启动 Agent | `{agent_id, task, context}` | AgentRun 对象 |
| `get_view` | 调用视图 | `{view_id, params}` | 视图结果 |
| `search` | 全文搜索（V1+） | `{query, scope}` | 命中列表 |

### 11.4.3 授权模型

MCP tool 调用必须带 **Agent credential**（OAuth 2.0 token 或短期 JWT）。每个 Agent 的 scope 在 `permissions` 表中按 `(subject_id=agent, resource_id, action, effect)` 显式表达。Tool 内部不重复做 scope 校验，全部委托 Policy 引擎。

### 11.4.4 Tool 描述的完整性

**[PROPOSAL]** 每个 MCP tool 必须在 `tools/list` 响应中提供：
- 完整 JSON Schema (input)
- 自然语言描述（一句）
- 使用示例（一句）
- 失败模式列表（一句）
- 关联 Policy 引用 ID

**AISEC-REQ-003（MCP tool description integrity, V1/P1）** 要求平台在 tool 描述/Schema 变化时标记为 Policy-relevant Event，需要重新审批才能继续使用。详见 [§7.3 AI 安全性](07-security-design.md#73-ai-安全-ai-securityaisec-req)。

## 11.5 Git 协议 API

| 操作 | 协议 | URL 模板 | 端点 |
|---|---|---|---|
| clone | HTTP | `https://host/{repo}.git` | `/git/{repo}.git/info/refs?service=git-upload-pack` |
| fetch | HTTP | 同上 | `/git/{repo}.git/git-upload-pack` |
| push | HTTP | `https://host/{repo}.git` | `/git/{repo}.git/git-receive-pack` |
| clone/fetch/push | SSH | `git@host:{repo}.git` | SSH subsystem `git-upload-pack` / `git-receive-pack` |
| LFS | HTTP | `https://host/{repo}.git/info/lfs` | LFS batch API（V1） |

**实现：** 全部走真实 `git` 二进制子进程（参见 [§2.6 Git 存储实现](02-architecture.md#26-git-存储实现-git-storage-implementation采用-phase-10-3)）。平台不发明新协议。

**Graph-aware 钩子：** 通过 Git 标准 `pre-receive` / `post-receive` / `update` 钩子回调到平台 API（参见 [§4.1 Git Server Subsystem](03-functional-design.md#31-git-server-子系统-git-server-subsystem)）。

## 11.6 Webhook (Outbound API)

### 11.6.1 注册模型

```http
POST /api/v1/webhooks
{
  "target_url": "https://example.com/hook",
  "events": ["issue.created", "pr.merged", "agent_run.completed"],
  "secret": "shared-hmac-secret-for-signing",  // 由平台加密存储
  "active": true
}
```

### 11.6.2 事件 payload

```json
{
  "id": "0190e8a4-...",          // 投递唯一 ID
  "event": "pr.merged",
  "delivery_attempt": 1,
  "occurred_at": "2026-08-19T06:18:00Z",
  "data": {
    "pr": { ... PR 完整对象 ... },
    "actor": { ... },
    "repository": { ... }
  }
}
```

### 11.6.3 投递保证与重试

| 维度 | 设计 |
|---|---|
| 至少一次 | 重试直到收到 2xx 或耗尽次数 |
| 重试策略 | 指数退避：1s, 5s, 30s, 5m, 30m, 2h, 12h（共 7 次，约 15h 跨度）|
| 超时 | 单次投递 10s 连接超时 + 30s 整体超时 |
| 签名 | `X-Webhook-Signature: sha256=<hex>` 头，HMAC-SHA256(secret, body) |
| 失败记录 | 失败时写一条 `webhook_delivery_failed` Event，含 reason / status_code / response_body 前 1KB |
| 接收方幂等 | 平台提供 `X-Webhook-Delivery-Id` 头，接收方用做去重 |

### 11.6.4 失败回查

```http
GET /api/v1/webhooks/{webhookId}/deliveries?status=failed&since=...
```

## 11.7 CLI API

**目标：** 让人在 shell 里能完成 80% 日常操作，无需打开 UI。

```bash
platform repo create --name=acme-api --visibility=private
platform issue list --repo=acme-api --state=open --json
platform agent run --agent=reviewer --pr=acme-api#42 --wait
platform audit events --actor=me --since=24h --json
platform export --include=graph,git,secrets --out=backup-2026-08-19.tar
platform import backup-2026-08-19.tar
platform webhooks test --id=0190e8a4-...  # 触发一次测试投递
```

**设计原则：**
- 全部子命令与 HTTP API 一一对应（同一后端逻辑）
- 默认人类可读输出；`--json` 切到 JSON
- `--quiet` 抑制非错误输出
- 退出码：0 成功 / 1 一般错误 / 2 策略拒绝 / 3 网络/上游错误
- 配置文件 `~/.platform/config.yaml` 支持多 profile 切换

## 11.8 跨切关注点

### 11.8.1 限流（V1）

| 维度 | 默认 | 备注 |
|---|---|---|
| 每用户 / 每分钟 | 1000 req/min | 可由 Policy 覆盖 |
| 每 Agent / 每分钟 | 60 req/min | MCP 工具更严 |
| 写操作额外令牌 | 写操作 1 配额单位，读 0.1 | 鼓励读多写少 |
| 突发 | 允许 2x 突发（token bucket） | 短时尖峰不立刻 429 |

**响应：** 超限返回 429 + `Retry-After` 头。

### 11.8.2 速率限制响应

```json
{
  "error": {
    "code": "rate_limit_exceeded",
    "message": "60 requests per minute exceeded for agent credential",
    "details": {
      "limit": 60,
      "window": "60s",
      "retry_after": 17
    }
  }
}
```

### 11.8.3 审计

**[PROPOSAL]** 所有 API 调用都记录一条 Event，schema：

```json
{
  "event_type": "api.request",
  "actor_id": "...",                    // Human or Agent Node
  "payload": {
    "method": "POST",
    "path": "/api/v1/repos",
    "status": 201,
    "trace_id": "...",
    "duration_ms": 23,
    "request_size_bytes": 142,
    "response_size_bytes": 312
  }
}
```

写入由 [§7.2 审计](07-security-design.md#72-审计-audit) 描述的 append-only 通道保障。

### 11.8.4 CORS（仅 Web UI 需要）

| Header | 取值 |
|---|---|
| `Access-Control-Allow-Origin` | 由部署时配置（不允许 `*`） |
| `Access-Control-Allow-Methods` | `GET, POST, PUT, PATCH, DELETE, OPTIONS` |
| `Access-Control-Allow-Headers` | `Authorization, Content-Type, X-Trace-Id, Idempotency-Key` |
| `Access-Control-Allow-Credentials` | `true`（本地部署用 cookie 认证时） |
| `Access-Control-Max-Age` | `3600` |

### 11.8.5 国际化（V1+ 议题，MVP 仅英文）

`Accept-Language` 头识别；错误消息可本地化。**所有 REQ-ID、enum 值、字段名不本地化**，永远是英文。

## 11.9 OpenAPI 规范治理

| 维度 | 规则 |
|---|---|
| 工具 | OpenAPI 3.1.0（YAML 源 + JSON 发布物） |
| 仓库位置 | `api/openapi/v1.yaml` (源) + `api/openapi/v1.json` (发布物) |
| 单一来源 | 端点先在 YAML 中存在，再写实现；PR review 必须包含 YAML diff |
| 客户端生成 | `make gen-sdks` 目标语言：TypeScript、Python、Go、Ruby |
| Mock server | `make run-mock` 启动 Prism mock server 用于前端/Agent 并行开发 |
| Contract test | Pact 或 Dredd 跑在 CI 上，验证实现 vs 规范不漂移 |
| 变更流程 | breaking change 必须开 RFC issue + 维护者评审 |

## 11.10 API 一致性测试

| 层级 | 测试 |
|---|---|
| 单元 | 各 handler 内部逻辑（无 HTTP） |
| 集成 | `httptest` + 真实数据库（隔离环境） |
| Contract | Pact：客户端契约 + 服务端契约互验 |
| 端到端 | Playwright 走完 10 步参考循环 |
| 安全 | OWASP API Top 10 自动化扫描（每 PR） |
| 性能 | k6 跑读 / 写 / 遍历三类基准（每夜） |

## 11.11 Admin API 端点族 / Admin API Endpoint Family

**[PROPOSAL]** 平台 §14 / §13.1 定义的 Admin 运维界面背后是一组独立的 Admin API。**端点族**：

```
/admin/v1/                 # 全部 admin 端点统一前缀
├─ /apps                   # App 集群管理
├─ /events                 # 中心事件流与死信
├─ /users                  # 用户与角色
├─ /audit                  # 审计日志
├─ /secrets                # KEK / 密钥管理
├─ /security               # 安全告警 / 策略绕过
├─ /config                 # 系统配置
├─ /backup                 # 备份与恢复
├─ /health                 # 健康检查
├─ /metrics                # Prometheus 指标
├─ /slo                    # SLO 仪表盘
└─ /openapi.json           # 独立 OpenAPI 文档
```

**与终端 API 的物理隔离**：

| 维度 | 终端 API (`/api/v1/*`) | Admin API (`/admin/v1/*`) |
|---|---|---|
| 进程 | 主 API 进程 | 独立 Admin 子进程（V1+ K8s 独立 Deployment）|
| 端口 | 3000 | 3001（V1+ Cloud 暴露在 `admin.platform.internal`）|
| 鉴权 | OIDC + JWT（`iss=platform-user`）| mTLS + JWT（`iss=platform-admin`）+ 双因素 |
| RBAC | `user_roles` 表 | `admin_roles` 表（独立）|
| 审计 | 普通 `events` 表 | `admin_audit` 不可变表 + 哈希链 + SIEM 转发 |
| 公开 | 是 | 否（V1+ 仅 internal ingress）|
| 限流 | 用户级 | admin 配额更高（500 req/min）|

**关键不变量**：

- [PROPOSAL-REQ-API-ADMIN-001] 终端用户 JWT 访问 `/admin/v1/*` 必须 403（`iss` 不匹配）
- [PROPOSAL-REQ-API-ADMIN-002] Admin JWT 访问 `/api/v1/*` 必须 403（域分离）
- [PROPOSAL-REQ-API-ADMIN-003] Admin 写操作必须在 `admin_audit` 留痕（middleware 强制）
- [PROPOSAL-REQ-API-ADMIN-004] 关键写操作（升级/回滚/KEK 轮换）必须双因素 + 二次确认头
- [PROPOSAL-REQ-API-ADMIN-005] OpenAPI 规范独立生成、独立 lint、独立 Pact 契约测试

完整端点目录、权限矩阵、DDL、OpenAPI 片段见 [详细设计 §13.1 Admin API 端点目录](../detailed-design/13-admin-api-and-ops-ui.md#131-admin-api-端点目录)。

## 11.12 Plugin API 端点族 / Plugin API Endpoint Family

**[PROPOSAL]** 每个 App 暴露的入口端点统一前缀 `/api/apps/<app_id>/...`，由 Plugin Loader 在 App 安装时**动态注册**到主 API 进程的路由表。

**注册方式**：

```yaml
# app.yaml 中的 entrypoints.http
entrypoints:
  http:
    - path: /api/apps/github-pr-reviewer/health
      method: GET
      handler: health
    - path: /api/apps/github-pr-reviewer/review
      method: POST
      handler: create_review
      auth_required: true
```

**关键约定**：

- [PROPOSAL-REQ-API-PLUGIN-001] 路径必须以 `/api/apps/<app_id>/` 开头（命名空间隔离）
- [PROPOSAL-REQ-API-PLUGIN-002] 路径禁止与核心 API（`/api/v1/*` `/admin/v1/*`）冲突（Plugin Loader 启动时校验）
- [PROPOSAL-REQ-API-PLUGIN-003] 路由转发到 healthy instance，循环负载均衡
- [PROPOSAL-REQ-API-PLUGIN-004] App 升级时新版本 instance 与旧版本并存，平滑切流（见 §13.7）
- [PROPOSAL-REQ-API-PLUGIN-005] App 禁用时该前缀全部 503
- [PROPOSAL-REQ-API-PLUGIN-006] 每个 App 路由自带 OTel span 属性 `app.id` `app.version` `app.instance_id`
- [PROPOSAL-REQ-API-PLUGIN-007] 错误格式与核心 API 一致（§11.3.4）

**路由注册协议**（Plugin Loader → 主 API 进程）：

```rust
// Plugin Loader 通过内部 IPC（Unix socket）注册路由
pub async fn register_routes(app_id: &str, instance_id: &str, routes: Vec<RouteDef>) -> Result<()> {
  let cmd = RouterCommand::Register {
    app_id: app_id.to_string(),
    instance_id: instance_id.to_string(),
    routes,
  };
  router_ipc.send(cmd).await?;

  // 等待 router ACK
  let ack = router_ipc.recv_ack().await?;
  if !ack.success {
    return Err("router rejected routes");
  }
  Ok(())
}

// 主 API 进程收到 Register 后：
// 1. 校验路径前缀 /api/apps/<app_id>/
// 2. 校验与现有路由无冲突
// 3. 写入 in-memory 路由表
// 4. ACK Plugin Loader
```

**App 路由 vs 核心路由优先级**：

```
请求路径匹配顺序：
1. /admin/v1/*         → Admin 路由表
2. /api/v1/*           → 核心 API 路由表
3. /api/apps/{id}/*    → App 路由表（动态注册）
4. /mcp/*              → MCP 工具路由
5. 其他                → 404
```

**安全边界**：

- App 路由的鉴权由 App 自身 handler 决定（manifest.spec.entrypoints.http[*].auth_required）
- 主 API 进程不做应用级鉴权，仅做：网络可达性 / 速率限制 / 路径校验 / OTel span 注入
- 跨 App 鉴权由协调层（§7.4.4）通过 `coord_call_app_proc` 中介函数处理

详细实现（流量分流 / 错误处理 / OTel）见 [详细设计 §12.4.4 HTTP 入口命名空间](../detailed-design/12-app-registry-and-plugin-loader.md#1244-http-入口命名空间) + [§8.3 路由表](../detailed-design/08-api-handlers.md#83-路由表)。

---

**导航：**
[← 10. 受入测试方针](10-acceptance-test-policy.md) · [README](README.md) · [12. App 群组信息互通设计 →](12-app-group-intercommunication.md)
