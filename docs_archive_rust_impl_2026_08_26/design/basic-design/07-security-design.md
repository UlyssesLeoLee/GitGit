# 07. 安全设计 / Security Design

> **[PROPOSAL] IPA 共通框架 2013 章节定位：** P3.A2.T6 研讨安全方式


本章反映 IPA 标准差距分析 Phase 14（F14-3, F14-4, F14-9）与红队评审 Phase 11（R6, R7, R8）。

## 7.1 认证 / 授权 / Authentication & Authorization

**认证（Authentication）：**

- Local：用户名 + 密码（Argon2id 哈希）或 SSH 公钥
- Cloud（V1）：OIDC / SAML 标准 IdP 集成（Auth0、Okta、Keycloak、Azure AD 等）

**授权（Authorization）：**

- RBAC：角色（admin、maintainer、developer、reporter、guest 等）
- ABAC：跨 Node / Edge / Policy 的属性基础评估
- 评估由 [§3.7 安全与访问控制子系统](03-functional-design.md#37-安全与访问控制子系统-security-access-control-subsystem) 统一处理
- 评估结果作为 Event 记录到 `events` 表

**Agent 的待遇（O8/O9）：**

- Agent 与 Human 同为 Node 子类型（AGT-REQ-007）
- 每个 Agent 拥有范围受限凭据（SEC-REQ-002）
- deny-by-default（SEC-REQ-004）
- 需人类审批的操作通过 UX-REQ-003 一键审批（AGT-REQ-002）

## 7.2 审计 / Audit

**结构化审计（SEC-REQ-003）：**

- `events` 表作为主审计日志（与图谱原语一体，不是独立子系统）
- 仅追加约束（DB 级别）
- 记录所有 Node 修改、Edge 添加、Policy 评估、Agent 执行、审批操作

**导出（SEC-REQ-006, V1）：**

- 导出到仅追加文件
- 防篡改验证（HMAC 链或 Merkle 根）
- 外部 SIEM 集成（Splunk、Elastic 等）

**例外删除（GDPR 等）：**

- 物理删除不可能，但提供逻辑删除 + 载荷 redact 的例外路径（ADR-9 未定）
- 所有例外操作作为审计 Event 记录，操作者、原因、范围必填

**对 Platform process 完全被攻陷的耐受性（AISEC-REQ-009 (a), MVP 必填）：**

Phase 10 §4 的 monolith-first 设计将 Policy 评估、凭据签发、沙箱启动、审计 Event 写入这 4 个功能集中到单一进程。为防止应用层完全被攻陷时审计轨迹本身被改写，**在 DB 角色级别剥夺 Platform Process 对 `events` 表的 `UPDATE` / `DELETE` 权限**。Platform Process 的常规读写用 DB 角色仅被授予 INSERT 权限。这样即便应用层代码被攻陷，审计日志在结构上只能是写入状态。这是 Phase 11 RT-10（红队评审最重要的发现）的 Accepted-and-fixed 反映，是 MVP 发布判定的 **non-negotiable 项**。

## 7.3 AI 安全 / AI Security（AISEC-REQ）

| 威胁 | 防御 | 负责需求 |
|---|---|---|
| 提示词注入（直接）| 系统 / 用户 / 工具三段隔离，结构化输入校验 | AISEC-REQ-001 |
| 提示词注入（间接 / 来自 PR/Issue 等不可信上下文）| 检索到用户可控数据时，强制使用 XML 边界符号打标（如 `<untrusted_user_input role="...">`）并附带前置系统转义，明确告知模型禁止执行其内命令 | AISEC-REQ-001 |
| MCP 工具过度授权 | 每个工具范围设置，deny-by-default | AISEC-REQ-002 |
| 密钥进 AI 载荷 | 密钥在 AI 调用前从 Secrets Store 过滤与脱敏 | AISEC-REQ-004 |
| Agent 操作权限提升 | 每次执行签发最小权限 token，执行后失效；容器默认配置 `internal` 网络隔离，阻断非白名单公网直连 | AISEC-REQ-005 |
| Policy 绕过 | Policy 评估同步、进程内、代码路径审计对象，无旁路 | AISEC-REQ-006 |
| 内部特权分离评审 | (a) DB 角色级仅追加强制 = MVP (P0)，(b) 评审实施 = V1 前 | AISEC-REQ-009 (a)(b) |

## 7.4 网络安全 / Network Security（SEC-REQ-008, Phase 14 F14-3）

**[PROPOSAL]** 网络控制在 Local 与 Cloud 下采用不同前提：

| 对象 | Local（单机）| Cloud（V1）|
|---|---|---|
| 外部通信 | mTLS（客户端 ↔ Platform）| mTLS（客户端 ↔ Platform）|
| 内部通信 | loopback / unix socket（默认）| mTLS（Platform ↔ PostgreSQL, Platform ↔ secrets backend）|
| Agent Workspace | loopback 网络（与其他 Workspace 隔离）| 通过 NetworkPolicy 实现沙箱间分离 |
| 入口控制 | 主机 OS 防火墙 | WAF / L7 LB 过滤 |
| 出口控制 | 默认 deny（仅 AI 调用目标在 allowlist）| 默认 deny（VPC egress control）|

## 7.5 密钥管理 / Secrets Management（SEC-REQ-005, SEC-REQ-010）

**[PROPOSAL]** 三层信封加密：

1. **KEK（密钥加密密钥）：** 由环境变量或 KMS 管理。Platform 启动时获取。
2. **DEK（数据加密密钥）：** 每个密钥一个。AES-256-GCM。用 KEK wrap 后存入 `secrets.dek_wrapped`。
3. **明文密钥：** 用 AES-256-GCM 加密后存入 `secrets.encrypted_blob`。

轮换：DEK 定期重 wrap（节奏 V1 时 [`TBD` ]）。撤销即时物理删除。

**与图数据隔离：** `secrets` 表位于与 `nodes` / `edges` / `events` 不同的 schema，外部 SQL 失去读权限。读写仅通过 `secrets` 服务。

## 7.6 信任边界 / Trust Boundary

```
┌──────── Untrusted Zone ────────┐
│  External clients (browser,    │
│  git CLI, MCP client, CI)      │
└──────────────┬─────────────────┘
               │ mTLS
               ▼
┌─────── Platform Process ───────┐
│  ┌─────────────────────────┐   │
│  │ API / MCP / Git Handler │   │  ← 认证 / Policy 评估
│  └────────┬────────────────┘   │
│           ▼                    │
│  ┌─────────────────────────┐   │
│  │  Core Engine            │   │  ← 进程内，sandbox
│  │  (Graph/AI/Ctx/AGT/CI)  │   │
│  └────────┬────────────────┘   │
│           │ mTLS / unix socket │
└───────────┼────────────────────┘
            ▼
┌─────── Trusted Zone ──────────┐
│  PostgreSQL (TLS)              │
│  Secrets Store (encrypted)     │
│  Git Object Storage            │
└────────────────────────────────┘

┌───── Ephemeral Sandboxes ─────┐
│  Agent Workspace (loopback)   │  ← 隔离，tear-down
│  CI Runner (loopback)         │  ← 隔离，tear-down
└────────────────────────────────┘
```

## 7.7 App 沙箱权限边界 / App Sandbox Permission Boundary

**[PROPOSAL]** §13 / 详细设计 §12 把 App 升格为一级对象后，必须为 App 设立**显式沙箱边界**，防止恶意或编写不当的 App 越权访问其他 App 数据或核心系统表。

### 7.7.1 沙箱机制

| 维度 | 隔离手段 | 说明 |
|---|---|---|
| **DB schema** | 每个 App 独立 schema `app_<app_id>` | App 存储过程 / 业务表只能在该 schema 下创建 |
| **DB role** | 每个 App 独立 login role `app_<app_id>` | 默认 deny，跨 schema / 跨 App 调用必须显式 grant |
| **PL/pgSQL** | `SECURITY DEFINER` + 显式 `SET search_path` | App 存储过程调用 `trigger_event_publish` / `coord_call_app_proc` 等平台函数时，由平台函数强校验调用方 |
| **HTTP 入口** | 路径命名空间 `/api/apps/<app_id>/*` | 路由由 Plugin Loader 动态注册，路径冲突启动时拒绝 |
| **进程** | MVP 阶段同进程多 instance（按 app_id 路由）；V1+ K8s 每 App 独立 Pod | 通过进程边界实现 OS 级隔离 |
| **资源** | manifest.spec.resource_limits 强制限额（CPU / memory / storage）| 超限自动 kill（V1+ K8s 限流）|

### 7.7.2 显式 deny-by-default（AISEC-REQ-013 强化）

**[PROPOSAL-REQ-SEC-APP-001]**（**MVP non-negotiable**，与 AISEC-REQ-009(a) 强化并列）

- App DB role 默认对 `public` schema、`nodes` / `edges` / `events` / `secrets` / `event_stream` 表**全部无权限**
- 仅 manifest.spec.permissions.required_grants 中显式声明的权限被授予
- manifest.spec.permissions.forbidden_grants 必须非空（deny-by-default 明示）
- 平台 `trigger_event_publish` / `coord_call_app_proc` 等 SECURITY DEFINER 函数强校验调用方上下文（`app.current_app_id` GUC）
- App 永远无法直接 `INSERT INTO event_stream` —— 必须通过 `trigger_event_publish` 函数

### 7.7.3 跨 App 调用安全约束

**[PROPOSAL-REQ-SEC-APP-002]**

- 跨 App 存储过程调用必须经 `coord_call_app_proc(target_app_id, proc_name, args)` 中介（详细设计 §7.4.4）
- 必须在 `app_acl` 表显式授权（双方 manifest 都声明 + admin 审批）
- 默认拒绝任何跨 App 调用

### 7.7.4 App 升级安全约束

**[PROPOSAL-REQ-SEC-APP-003]**

- App 升级时新版本沙箱与旧版本并存（schema 同名 + role 同名）→ 升级前必须先备份
- 升级回滚 = 切换 instance 流量 + 旧版本沙箱仍可用
- 旧版本沙箱数据保留 N 天（默认 7），过期清理
- 升级前 admin 必填 `change_reason` + 双因素 + 二次确认

## 7.8 Admin 独立鉴权域 / Admin Separate Auth Domain

**[PROPOSAL]** 管理员操作与终端用户操作**完全分离**鉴权域，理由详见 §14.3.3 / 详细设计 §13.2.4。

### 7.8.1 分离的 4 个维度

| 维度 | 终端用户 | Admin |
|---|---|---|
| **JWT issuer** | `iss=platform-user` | `iss=platform-admin` |
| **签名密钥** | `USER_JWT_SECRET` | `ADMIN_JWT_SECRET`（独立存储于 HSM / Vault）|
| **RBAC 表** | `user_roles` | `admin_roles`（独立）|
| **会话域** | cookie `app.platform.local` | cookie `admin.platform.local` |

### 7.8.2 强约束

**[PROPOSAL-REQ-SEC-ADMIN-001]**（**MVP non-negotiable**）

- 终端用户 JWT 访问 `/admin/v1/*` 必须 403（issuer 校验失败）
- Admin JWT 访问 `/api/v1/*` 必须 403（issuer 校验失败）
- 任何尝试用错域 JWT 跨域访问 → 写 `admin_audit` severity=critical + OTel 安全告警
- Admin JWT 短 TTL（15 分钟）+ 滑动续期
- Admin 登录后首次访问关键操作必须双因素（TOTP / WebAuthn）二次确认

### 7.8.3 双因素认证流程

**[PROPOSAL-REQ-SEC-ADMIN-002]**

1. 用户名 + 密码 → 颁发 `mfa_pending` JWT（短 TTL 5min，无 totp_verified 标记）
2. 客户端收到 mfa_pending → 引导用户输入 TOTP / WebAuthn
3. TOTP/WebAuthn 验证通过 → 颁发完整 `admin` JWT（15min TTL，totp_verified=true）
4. 设置 HttpOnly + Secure + SameSite=Strict cookie `admin_session`
5. 关键操作（升级/回滚/KEK 轮换）二次确认头（X-Confirm: UPGRADE 等）

### 7.8.4 admin_audit 不可篡改

**[PROPOSAL-REQ-SEC-ADMIN-003]**

- 所有 admin 操作入 `admin_audit` 表（详细设计 §13.3 DDL）
- 哈希链（每行包含前一行 hash）防篡改
- PG 触发器禁止 UPDATE/DELETE（即使 superuser）
- OTel collector 通过 wal2json 实时转发 SIEM
- 每小时定时 `verify_admin_audit_chain()` 验证完整性，发现断裂立即告警

---

---

**导航 / Navigation:**
[← 06. 非功能设计](06-non-functional-design.md) · [README](README.md) · [08. 运维设计 →](08-operations-design.md)
