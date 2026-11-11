# 03. 功能设计 / Functional Design

> **[PROPOSAL] IPA 共通框架 2013 章节定位：** P3.A1.T1 识别功能要求 / P3.A2.T2 研讨系统功能构成


本章针对 8 个子系统，描述 MVP（Phase 9 确定的 37 项需求）和 V1 邻近需求中需要在方案设计层面确定的内容。每个子系统按"职责、MVP 功能、V1 功能、接口、状态"5 个维度描述。

## 3.1 Git Server 子系统 / Git Server Subsystem

### 3.1.1 职责
- 通过标准 Git 协议（HTTP、SSH；git:// MVP 不支持）提供 clone / fetch / push
- 裸仓库的文件系统管理
- 分支 / 标签操作、merge / rebase
- 通过服务端钩子实现图谱联动

### 3.1.2 MVP 功能（Phase 9 §3，GIT-REQ）
- **GIT-REQ-001** 标准 Git 协议合规
- **GIT-REQ-002** 标准分支 / 标签操作
- **GIT-REQ-003** merge / rebase
- **GIT-REQ-010** 与 AI 子系统解耦的可用性（通过同进程内代码路径隔离实现，Phase 10 §4）

### 3.1.3 V1 功能
- **GIT-REQ-004** LFS 支持
- **GIT-REQ-005** 子模块 / worktree
- **GIT-REQ-006** 服务端钩子 + Policy 评估回调（GIT-REQ-006 修正案：钩子回调的延迟预算与 fail-closed，Phase 13）
- **GIT-REQ-007** 部分 / shallow / sparse clone
- **GIT-REQ-008** 签名 commit / tag 验证
- **GIT-REQ-009** 镜像 / 迁移导入
- **GIT-REQ-011** 定期 GC / 重打包

### 3.1.4 接口与容灾
- **入站：** HTTP（smart HTTP）、SSH（`git-receive-pack` / `git-upload-pack`）
- **出站：** `git` CLI 子进程、`gix` (gitoxide) 纯 Rust 库
- **容灾与降级机制：** 读路径优先使用 `gix`；当 `gix` 遇到非常规 Ref 编码、损坏或未支持的 Git 对象时，记录 Warning 日志并自动回退调用系统 `git CLI`（`git cat-file` / `git log`），确保 100% 协议兼容与零 500 异常。
- **回调：** 通过标准 Git 钩子调用平台 API

### 3.1.5 状态
- 无状态。裸仓库位于文件系统，与 Platform Process 重启独立持久化。

## 3.2 工程图谱子系统 / Engineering Graph Subsystem

### 3.2.1 职责
- 基于 5 个原语（Node / Edge / Event / Policy / View）的 CRUD 与查询
- 类型注册表管理（组织、Project、Repository、Requirement、Issue、ADR、Commit、Symbol、PR、Review、Test、Deployment、Release、Human、Agent、AgentRun、Policy、Incident 等）
- 不可变事件日志（兼作审计日志，SEC-REQ-003）
- 基于递归 CTE 的图查询（带深度限制）

### 3.2.2 MVP 功能（Phase 9 §3，GRF-REQ）
- **GRF-REQ-001** 类型化 Node 注册表
- **GRF-REQ-002** 类型化有向 Edge
- **GRF-REQ-003** 不可变 Event 日志
- **GRF-REQ-004** Policy 评估原语
- **GRF-REQ-005** View 原语
- **GRF-REQ-006** Node 类型注册表（需求词汇种子）
- **GRF-REQ-007** Edge 类型注册表
- **GRF-REQ-008** 图查询 API（人类 / Agent 双方）

### 3.2.3 V1 功能
- **GRF-REQ-009** Action-as-Event 约定（进行中状态查询）
- **GRF-REQ-010** Evidence-as-specialized-Edge 约定（Phase 13 修正案：`epistemic_status` enum 必填，`verified` / `asserted` / `attested`）
- **GRF-REQ-011** Requirement / ADR 追溯性 Edge

### 3.2.4 接口
- **入站（内部）：** 子系统间 API（函数调用）
- **入站（外部）：** HTTP API + MCP
- **出站：** PostgreSQL

### 3.2.5 状态
- PostgreSQL `nodes` / `edges` / `events` / `policies` / `views` 表。
- `events` 仅追加。例外删除只能通过 SEC-REQ-003 修正案的审计路径（GDPR / 留存用，ADR-9 尚未决定）。

## 3.3 AI 网关子系统 / AI Gateway Subsystem

### 3.3.1 职责
- 通过单一内部 API 路由到多个 AI 提供商 / 模型
- 提示词 / 响应的结构化日志（Event 化）
- 基于敏感度的路由（MVP 单一默认，V1 正式化）
- **成本与配额熔断器（Token Budget & Circuit Breaker）：** 强制维护租户级滑动窗口 Token 预算，当超出预设预算阈值时自动熔断阻断调用。

### 3.3.2 MVP 功能
- **AI-REQ-001** 单一默认提供商（OpenAI 兼容、Anthropic 兼容、本地模型之一可配置）— Phase 9 缩减后的 MVP 确定
- **AI-REQ-002** 成本 / token 可观测性与硬熔断（超限即阻断，防止 API 账单雪崩）

### 3.3.3 V1 功能
- **AI-REQ-003** 基于敏感度的路由（气隙环境限定本地）
- **AI-REQ-004** 提供商智能 Fallback 与优雅降级
- **AI-REQ-005** 模型动态平滑切换

### 3.3.4 接口
- **入站：** Context Engine（CTX-REQ-001）的 retrieve-and-prompt 调用、Agent Runtime 的直接调用
- **出站：** 各 AI 提供商的 HTTP API、本地模型运行时（Ollama、vLLM 等）HTTP API

### 3.3.5 状态
- 提示词 / 响应作为 Event 记录（可审计）
- 提供商凭据由 Secrets Store（[§7.5 密钥管理](07-security-design.md#75-密钥管理-secrets-managementsec-req-005-sec-req-010)）管理，与图数据隔离

## 3.4 上下文引擎子系统 / Context Engine Subsystem

### 3.4.1 职责
- 为任务从图结构中组装"最小必要"的上下文
- 组装结果作为 View 调用记录到 Event 日志
- 人类与 Agent 使用同一 API

### 3.4.2 MVP 功能
- **CTX-REQ-001** 默认最小上下文检索
- **CTX-REQ-002** 上下文组装是记录在案的 View 调用

### 3.4.3 V1 功能
- **CTX-REQ-003** 预算管理（token 上限 / 深度上限强制）
- **CTX-REQ-004** 缓存
- **CTX-REQ-005** 与模型无关的提示词模板

### 3.4.4 接口
- **入站：** AI Gateway（retrieve-and-prompt）、HTTP API
- **出站：** Engineering Graph 查询（GRF-REQ-008）

### 3.4.5 状态
- 仅运行时临时缓存。持久化仅限 View 调用日志。

## 3.5 Agent 运行时子系统 / Agent Runtime Subsystem

### 3.5.1 职责
- Agent 生命周期管理（spawn → 执行 → 完成 / 失败 / 超时）
- 范围受限凭据的签发（AGT-REQ-005）
- 隔离执行环境（Agent Workspace，默认仅内部网络无外网访问）
- 所有 Action 的 Policy 门控
- 人类审批门（AGT-REQ-002）与死循环熔断（单次 AgentRun 硬性限制最大 15 步）
- **意图导向（Intent-First）PR 结构化总结**：自动提炼变更意图、高危变更预警与图谱影响面，防止人类工程师审查疲劳

### 3.5.2 MVP 功能（AGT-REQ 7 项）
- **AGT-REQ-001** 厂商中立生命周期
- **AGT-REQ-002** Policy 控制的执行（含审批门）
- **AGT-REQ-003** 消息 / 状态接口
- **AGT-REQ-004** 产物采集 / 图谱联动
- **AGT-REQ-005** 隔离工作区 + 范围受限凭据
- **AGT-REQ-006** 资源上限（时间、内存、成本、单 Run 步数 $\le 15$）
- **AGT-REQ-007** Agent 作为 Node 子类型加入图谱

### 3.5.3 V1 功能
- **AGT-REQ-008** 通过 MCP 的工具访问（Phase 9 确定）
- **AGT-REQ-009** 子 Agent / 委派范围

### 3.5.4 接口
- **入站：** HTTP API（人类用户）、MCP（外部 Agent 客户端）
- **出站：** AI Gateway、Engineering Graph、容器运行时（Agent Workspace 启动）

### 3.5.5 状态
- Agent 节点 / AgentRun 节点持久化到图谱
- 仅执行期间的状态封闭在 Agent Workspace 内
- 凭据由 Secrets Store（[§7.5](07-security-design.md#75-密钥管理-secrets-managementsec-req-005-sec-req-010)）管理

## 3.6 CI/CD 子系统 / CI/CD Subsystem

### 3.6.1 职责
- CI 执行的隔离环境提供（CI Runner）
- 结果的图谱化（CI Run 节点 + Event）
- Agent 启动 / Agent 消费结果

### 3.6.2 MVP 功能
- **CI-REQ-001** CI 执行作为一级节点加入图谱

### 3.6.3 V1 功能
- **CI-REQ-002** 测试结果作为 Evidence 类型 Edge
- **CI-REQ-003** 流水线定义的图谱化
- **CI-REQ-004** 与现有 CI 生态兼容（webhook / exec）
- **CI-REQ-005** Agent 可启动 / Agent 可消费 CI
- **CI-REQ-006** Deployment 作为 Policy 控制的 Action 约定

### 3.6.4 接口
- **入站：** HTTP API、PostgreSQL `LISTEN`/`NOTIFY` 的任务通知
- **出站：** 容器运行时（CI Runner 启动）

### 3.6.5 状态
- CI Run 节点 / Test Result Edge 持久化到图谱
- 仅执行期间的状态封闭在 CI Runner 内

## 3.7 安全与访问控制子系统 / Security & Access Control Subsystem

### 3.7.1 职责
- RBAC/ABAC 评估
- Policy 评估核心（GRF-REQ-004 的实现）
- 结构化审计轨迹（SEC-REQ-003）
- 人类与 Agent 的同等待遇（O8/O9）
- 认证（Local：用户名密码或 SSH 公钥；Cloud：OIDC 等标准 IdP 集成，V1）

### 3.7.2 MVP 功能（SEC-REQ 5 项）
- **SEC-REQ-001** RBAC/ABAC 评估
- **SEC-REQ-002** Agent 凭据范围化
- **SEC-REQ-003** 结构化审计轨迹
- **SEC-REQ-004** Agent 的 deny-by-default
- **SEC-REQ-005** 密钥与图数据的隔离

### 3.7.3 V1 功能
- **SEC-REQ-006** 审计导出（SIEM 集成）
- **SEC-REQ-007** 审计防篡改增强
- **SEC-REQ-008** 网络层安全控制（mTLS / 服务间加密 / 网络隔离）— Phase 14 F14-3 反映
- **SEC-REQ-009** 持续安全风险管理（依赖漏洞扫描、补丁节奏）— Phase 14 F14-4 反映
- **SEC-REQ-010** 加密管理（at-rest / in-transit 显式）— Phase 14 F14-9 反映

### 3.7.4 AISEC-REQ（AI 安全）的整合
需求定义书 §36 的 9 条 AISEC-REQ 中，MVP 确定的 6 条作为横切控制整合实现到 Agent Runtime / AI Gateway / Context Engine：

**MVP 范围（P0，含 Phase 13 强化）：**

- **AISEC-REQ-001** 提示词注入防御 — 不信任内容按 data 处理，通过 provenance 标签与可信指令区分
- **AISEC-REQ-002** MCP 工具的范围化 — Agent 的 allowlist 之外的 MCP 服务器不可调用
- **AISEC-REQ-004** 密钥值不进 AI 载荷 — AI Gateway 发出载荷前剥离 SEC-REQ-005 标记的密钥
- **AISEC-REQ-005** Agent 操作的权限提升防御 — 委派链全程的传递性范围包含
- **AISEC-REQ-006** Policy 绕过检测 — 生成的代码 / 配置也走与人类创作相同的 Policy 门
- **AISEC-REQ-009** Platform process 完整性 — **(a)** 在 **DB 角色级别**剥夺 Platform Process 对 `events` 表的 `UPDATE` / `DELETE` 权限（仅 INSERT 授权），即便应用层完全被攻陷也无法改写审计轨迹；**(b)** 内部特权分离评审在 V1 前完成（受容量限制时）。本条是 Phase 11 RT-10（红队最大发现）的 Accepted-and-fixed 反映。

**V1/V2 范围（参考）：** AISEC-REQ-003（MCP 工具描述完整性，V1/P1）、AISEC-REQ-007（速率限制 + 异常检测，V2/P2）、AISEC-REQ-008（外部 MCP 服务器沙箱，V1/P1）

### 3.7.5 接口
- **入站：** 来自所有子系统的 Policy 评估调用
- **出站：** Engineering Graph（评估结果 Event 化）、Secrets Store

### 3.7.6 状态
- Policy 规则存在 `policies` 表，版本化，进程内缓存
- 评估结果作为 Event 记录到 `events`

## 3.8 API / MCP / Webhook 子系统 / API / MCP / Webhook Subsystem

### 3.8.1 职责
- 为人类 / Agent 双方提供文档化的 API
- MCP 服务器功能（响应外部 Agent 的 tool 调用）
- Webhook 投递（V1）

### 3.8.2 MVP 功能
- **API-REQ-001** 文档化 / Policy 控制的 HTTP API 表面

### 3.8.3 V1 功能
- **API-REQ-003** 限流 / 版本化
- **AGT-REQ-008** MCP 工具访问
- **CDX-REQ** Codex / 各种 AI Agent 集成
- Webhook 投递

### 3.8.4 接口
- **入站：** HTTP(S)（REST + JSON）、MCP over HTTP/JSON-RPC
- **出站：** Webhook 投递（V1）

### 3.8.5 状态
- 无状态调用。状态持久化到图谱。

---

**导航 / Navigation:**
[← 02. 系统方式设计](02-architecture.md) · [README](README.md) · [04. 数据设计 →](04-data-design.md)
