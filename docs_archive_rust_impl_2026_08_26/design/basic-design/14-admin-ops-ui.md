# 14. 管理员运维界面 / Admin Operations UI

> **[PROPOSAL] IPA 共通框架 2013 章节定位：** P3.A2.T5 研讨可靠性·性能·运行性 (Admin 运维入口) / P3.A3.T1 评价系统方式方案 (Admin UI 作为可观测与运维的统一入口)
>
> **设计主张 / Design Claim:** 管理员运维界面是**独立子进程 + 专用鉴权 + 双因素认证**，与终端用户 UI（§5.6）物理隔离。本章定义 Admin UI 的形态、关键页面、权限边界、与 Admin API（详细设计 §13）的配套关系。

---

## 14.0 目的 / Purpose

平台管理员（系统运维 / SRE / 安全审计）需要一个**集中可见的运维界面**，用于：
- 监控 App 集群健康（§13.5）
- 触发 App 安装 / 升级 / 回滚（§13.6）
- 查看中心事件流、DLQ、死信（§13.4.4）
- 管理用户与权限（§7.1）
- 查看审计日志（§7.2）
- 配置系统参数（KEK、租户、网络策略等）

**为什么是独立进程？**

| 维度 | 终端用户 UI | Admin UI |
|---|---|---|
| 端口 | 3000（与终端 SPA 共用）| 3001（独立监听）|
| 鉴权 | OIDC + JWT（与终端用户同）| mTLS + 双因素（TOTP / WebAuthn）|
| 网络可达 | 与终端用户同网段 | 仅 ops 网络可达（生产环境通过堡垒机或 SSH 隧道）|
| 鉴权角色 | `role:user` | `role:admin`（独立 RBAC 域）|
| 日志 | 普通审计 | 强审计：所有操作写入 `admin_audit` 表 + OTel span + SIEM 转发 |
| 进程故障影响 | 影响终端用户 | **不影响**终端用户（隔离原则）|

**冲突告警**：与 §0.2.2 不包含内容"F14-7 缺口"对齐——Admin UI 的人类签核流程与整个平台的"无人类签核"问题共享同一缺口，必须在 Phase 16 启动前由人类干系人补全。

---

## 14.1 部署形态 / Deployment Topology

### 14.1.1 Local-First 模式（MVP）

```
D:\GitGit\
├─ platform-server (主进程，端口 3000) ── 终端用户 SPA
└─ platform-admin (Admin UI 子进程，端口 3001) ── Admin Web UI
            │
            └─→ 同进程访问同一 PostgreSQL（schema 隔离：admin schema 仅 admin 角色可读）
```

- Admin UI 进程在启动时检查 `admin_console.enabled` 配置
- 默认 **禁用** Admin UI（仅在 `local-ops` 模式下启用）
- 启用时强制要求设置 `ADMIN_BOOTSTRAP_TOKEN` 环境变量（首次启动用 token 登录，设置 admin 密码）

### 14.1.2 Cloud V1+ 模式

- Admin UI 独立 Deployment（K8s）
- 仅暴露在 `internal` ingress（不暴露到公网）
- 通过 OIDC 接入企业 SSO（Keycloak / Okta）
- 强制 mTLS（§7.6）
- 详细 K8s 部署清单见 §8 运维设计 + 详细设计 §13.3

---

## 14.2 关键页面与交互 / Key Pages & Interactions

### 14.2.1 仪表盘（Dashboard）

**路径**：`/admin/`

**展示**：

- 平台总览（节点数 / 用户数 / 24h 事件数 / DLQ 数量）
- App 集群健康矩阵（按 App 列出：state / version / instance count / last error）
- 关键 NFR SLO 实时状态（API P99 延迟 / 事件端到端延迟 / 错误率）
- 最近 24h Admin 操作流水（谁、什么时候、做了什么）

**交互**：
- 点击 App → 跳到 [§14.2.2 App 详情]
- 点击 SLO 红色告警 → 跳到告警详情（事件流）

### 14.2.2 App 详情页

**路径**：`/admin/apps/{app_id}`

**展示**：

- 当前 Manifest（`app.yaml` 全文，可对比历史版本）
- 当前 / 可用 版本对比
- Instance 列表（含健康、心跳、最近错误）
- 事件订阅 / 发布关系图（中心事件总线视图）
- 资源使用曲线（CPU / 内存 / 队列长度）

**关键操作**：

| 操作 | 按钮 | 二次确认 | 后果 |
|---|---|---|---|
| 升级 | `Upgrade to v1.4.3` | "请输入 'UPGRADE' 确认" | 走 §13.7 升级流程 |
| 回滚 | `Rollback to v1.4.2` | "请输入 'ROLLBACK' 确认" | 立即切换，旧 instance 优雅停机 |
| 禁用 | `Disable` | "请输入 'DISABLE' 确认" | 取消订阅、停止请求、优雅停机 |
| 启用 | `Enable` | "请输入 'ENABLE' 确认" | 重新安装 instance、恢复订阅 |
| 卸载 | `Uninstall` | 需双因素 + 二次确认 | 同 §13.6.3 + 可选清理存储 |
| 查看日志 | `View Logs` | 无 | 跳到日志查询页（按 instance / 时间） |

**[PROPOSAL]** 关键操作（升级 / 回滚 / 卸载）强制 **双因素认证**（TOTP）二次输入。

### 14.2.3 中心事件流

**路径**：`/admin/events`

**展示**：
- 实时事件流（WebSocket / SSE，按 event_type / app / tenant 过滤）
- 死信队列（DLQ）列表
- 单事件详情（含完整 envelope + 重投按钮）

**关键操作**：
- 重投死信事件（手动 `redeliver`）
- 暂停 / 恢复 Relay（紧急维护）
- 跳到事件的因果链 / 相关链

### 14.2.4 用户与权限

**路径**：`/admin/users`

**展示**：
- 用户列表 + 角色 + 所属租户
- 角色矩阵（`role:user` / `role:admin` / `role:auditor` / 自定义）
- 权限策略列表（ABAC 规则）

**关键操作**：
- 创建用户（需 admin）
- 分配角色（需 admin）
- 撤销角色（需 admin + 二次确认）
- 查看用户操作历史

### 14.2.5 审计日志

**路径**：`/admin/audit`

**展示**：
- 全部 admin 操作流水（不可变 `admin_audit` 表）
- 关键安全事件（登录失败、权限变更、密钥访问）
- 按用户 / 时间 / 操作类型过滤
- 导出（CSV / JSONL）

**权限**：所有 admin 角色可看；不可改 / 不可删。

### 14.2.6 系统配置

**路径**：`/admin/config`

**展示**：
- KEK 提供者配置（§7.5）
- 邮件 / Webhook 出站配置
- 备份策略
- NFR SLO 阈值

**关键操作**：
- 轮换 KEK（需 admin + 双因素）
- 修改 SLO 阈值（需 admin + 二次确认）
- 触发手动备份 / 恢复演练

---

## 14.3 鉴权与权限 / AuthN & AuthZ

### 14.3.1 鉴权流程

```
Admin 用户                         Admin UI (3001)                  Admin API (:3000/admin/v1)        PostgreSQL
══════════                         ══════════════                   ══════════════════                ═══════════

[1] 访问 /admin/                          │
     → 302 to /admin/login                │
                                           │
[2] 提交 username + password               │
     + TOTP code                           │
                                           │
[3] Admin UI 验证                          │
     - bcrypt 比对密码                     │
     - TOTP 验证                           │
     - 检查 mTLS 客户端证书（V1+ Cloud）│
                                           │
[4] 颁发 admin session cookie              │
     (HttpOnly, Secure, SameSite=Strict,   │
      TTL=15min, 滑动续期)                 │
                                           │
[5] 用户操作 (升级 App)                    │
     POST /admin/v1/apps/{id}/upgrade      │
     + admin session cookie                │
     + Idempotency-Key                     │
                                           │
[6]                                    ───▶│
                                        [Admin API 验证]                │
                                        - 解析 session                  │
                                        - 检查 role:admin               │
                                        - 检查资源级权限 (§14.3.2)       │
                                        - 双因素二次确认头               │
                                            │                           │
                                            │  ┌────────────────────────┘
                                            │  │ 鉴权 + ABAC 决策 (策略引擎 §3)
                                            │  │
                                            ▼  ▼
                                                          [PG RLS 检查 admin schema 权限]
```

### 14.3.2 权限矩阵（粗粒度）

| 资源 | 操作 | 角色要求 | 二次确认 | 双因素 |
|---|---|---|---|---|
| App 安装 | `POST /admin/v1/apps` | `role:admin` | 否 | 是 |
| App 升级 | `PUT /admin/v1/apps/{id}` | `role:admin` | 是 | 是 |
| App 回滚 | `POST /admin/v1/apps/{id}/rollback` | `role:admin` | 是 | 是 |
| App 卸载 | `DELETE /admin/v1/apps/{id}` | `role:admin` | 是 | 是 |
| 中心事件重投 | `POST /admin/v1/events/{id}/redeliver` | `role:admin` | 否 | 否 |
| KEK 轮换 | `POST /admin/v1/secrets/rotate-kek` | `role:admin` + `secrets:write` | 是 | 是 |
| 用户角色变更 | `PUT /admin/v1/users/{id}/roles` | `role:admin` | 是 | 否 |
| 审计日志查看 | `GET /admin/v1/audit` | `role:admin` 或 `role:auditor` | 否 | 否 |
| 审计日志导出 | `GET /admin/v1/audit/export` | `role:admin` | 是 | 否 |

### 14.3.3 Admin 独立鉴权边界（SEC-REQ-011）

**[PROPOSAL]** Admin 鉴权域与终端用户鉴权域**完全分离**：

- 不同 JWT issuer（`iss=platform-admin` vs `iss=platform-user`）
- 不同签名密钥（`ADMIN_JWT_SECRET` 与 `USER_JWT_SECRET`）
- 不同 cookie 域（`admin.platform.local` vs `app.platform.local`）
- 不同 RBAC 表（`admin_roles` vs `user_roles`）
- 终端用户 JWT 不可访问 `/admin/v1/*`（验证 `iss` 失败 → 403）
- Admin JWT 不可访问 `/api/v1/*`（验证 `iss` 失败 → 403）

**这是 OWASP Zero Trust 原则在 RBAC 层的具体实现。**

---

## 14.4 关键非功能要求

| 指标 | 目标 | 测量方法 |
|---|---|---|
| Admin UI 首屏加载 | ≤ 1s（本地）/ ≤ 3s（V1+ Cloud）| Lighthouse |
| Admin API P99 延迟 | ≤ 200ms（本地）/ ≤ 1s（Cloud）| OTel metrics |
| 强审计完整性 | 100% admin 操作入库（不可绕过）| 自动化测试：尝试直接调 Admin API 绕过 UI 仍能审计 |
| Admin UI 与终端 UI 故障隔离 | Admin 进程崩溃 ≤ 30s 重启；终端 UI 不受影响 | 故障注入测试 |
| 双因素旁路防御 | 升级 / 回滚 / KEK 轮换无 TOTP 必失败 | 自动化测试 |

详细 NFR-REQ 见 [§6 非功能设计](06-non-functional-design.md) + 详细设计 §13.2。

---

## 14.5 与现有章节的关系

| 章节 | 关系 |
|---|---|
| §11 API 设计 | API 端点族扩展 `/admin/v1/*`（详见 [§11.4 MCP 设计](11-api-design.md#114-mcp-设计) 后续）|
| §7 安全设计 | Admin 独立鉴权 = SEC-REQ-011（新增）；双因素 = SEC-REQ-012（新增）|
| §8 运维设计 | Admin UI 进程纳入 §8 部署形态；V1+ K8s 清单见 §8 补充 |
| §6 非功能设计 | 加 NFR-REQ-007（Admin 操作双因素）|
| §13 App 集群 | Admin UI 是 §13 集群管理 / 升级 / 回滚的视觉入口 |

---

## 14.6 关键 REQ-ID 新增

| REQ-ID | 简述 | 章节 |
|---|---|---|
| ADMIN-REQ-001 | Admin UI 独立子进程 + 独立端口 | §14.1 |
| ADMIN-REQ-002 | Admin 鉴权与终端用户鉴权域完全分离 | §14.3.3 |
| ADMIN-REQ-003 | 关键操作（升级 / 回滚 / KEK 轮换）强制双因素 | §14.3.2 |
| ADMIN-REQ-004 | Admin UI 故障不传播至终端 UI | §14.4 |
| ADMIN-REQ-005 | 全部 admin 操作入 `admin_audit` 表 + OTel span | §14.2.5 |

详细 API 端点目录与 DDL 见 [详细设计 §13. Admin API & Ops UI](../detailed-design/13-admin-api-and-ops-ui.md)。

---

**导航 / Navigation:**
[← 13. App 集群与可热插拔架构](13-app-cluster-and-plugins.md) · [README](README.md)
