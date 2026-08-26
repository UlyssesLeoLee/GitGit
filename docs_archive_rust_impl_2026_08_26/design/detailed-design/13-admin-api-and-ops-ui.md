# 13. Admin API & Ops UI / 管理员 API 与运维界面后端

> **[PROPOSAL] IPA 共通框架 2013 章节定位：** P4 软件方式设计 / T-P4-A1-1 设计软件构成 (Admin 子进程架构) / T-P4-A1-6 设计安全架构 (独立鉴权域) / T-P4-A1-7 设计可观测性 (admin_audit)
>
> **上游文档：** [基本设计 §14. 管理员运维界面](../basic-design/14-admin-ops-ui.md)、[§11. API 设计](../basic-design/11-api-design.md)、[§7. 安全设计](../basic-design/07-security-design.md)、[§12. App Registry & Plugin Loader](12-app-registry-and-plugin-loader.md)
>
> **设计主张 / Design Claim:** Admin API 与终端 API 物理分离（不同进程 / 不同端口 / 不同鉴权密钥 / 不同 RBAC 表）。Ops UI（SvelteKit SPA）只通过 Admin API 访问平台能力。本章定义 Admin API 端点目录、鉴权中间件、admin_audit DDL、Ops UI 前端架构、V1+ K8s 部署清单。

---

## 13.0 目的 / Purpose

承接 [基本设计 §14](../basic-design/14-admin-ops-ui.md) 的"管理员运维界面"概念，本章落地：

1. Admin API 完整端点目录（`/admin/v1/*`）
2. Admin 独立鉴权中间件（mTLS + 双因素 + admin session cookie）
3. `admin_audit` 表 DDL（不可变、加密、转发 SIEM）
4. Ops UI 前端架构（SvelteKit + Svelte 5）
5. V1+ K8s 部署清单（独立 Deployment + NetworkPolicy）

---

## 13.1 Admin API 端点目录

所有端点前缀 `/admin/v1/`，鉴权要求 `role:admin`（粗粒度），细粒度权限见 [§13.2.2 权限矩阵](#1322-细粒度权限矩阵)。

### 13.1.1 App 集群管理

| Method | Path | 操作 | 关键参数 |
|---|---|---|---|
| `GET` | `/admin/v1/apps` | 列出所有 App | `?state=&page=&per_page=` |
| `GET` | `/admin/v1/apps/{app_id}` | App 详情 | — |
| `POST` | `/admin/v1/apps` | 安装新 App | body: `manifest_yaml`, `manifest_hash` |
| `PUT` | `/admin/v1/apps/{app_id}` | 升级 App | body: 新 `manifest_yaml` |
| `DELETE` | `/admin/v1/apps/{app_id}` | 卸载 App | `?purge_data=true` |
| `POST` | `/admin/v1/apps/{app_id}/rollback` | 回滚 | body: `{ "target_version": "1.4.2" }` |
| `POST` | `/admin/v1/apps/{app_id}/disable` | 禁用 | — |
| `POST` | `/admin/v1/apps/{app_id}/enable` | 启用 | — |
| `GET` | `/admin/v1/apps/{app_id}/instances` | 列 instance | `?state=` |
| `GET` | `/admin/v1/apps/{app_id}/instances/{instance_id}/logs` | 查 instance 日志 | `?since=&until=&level=` |
| `GET` | `/admin/v1/apps/{app_id}/events` | App 事件流 | `?event_type=&limit=` |
| `GET` | `/admin/v1/apps/{app_id}/audit` | App 状态转换历史 | `?limit=` |

### 13.1.2 中心事件总线管理

| Method | Path | 操作 | 关键参数 |
|---|---|---|---|
| `GET` | `/admin/v1/events/stream` | 实时事件流（SSE）| `?event_type=&app_id=&tenant_id=` |
| `GET` | `/admin/v1/events/dlq` | 死信列表 | `?page=&per_page=` |
| `POST` | `/admin/v1/events/dlq/{event_id}/redeliver` | 死信重投 | body: `{ "force": false }` |
| `POST` | `/admin/v1/events/relay/pause` | 暂停 Relay | body: `{ "reason": "..." }` |
| `POST` | `/admin/v1/events/relay/resume` | 恢复 Relay | — |
| `GET` | `/admin/v1/events/schemas` | 已注册 schema 列表 | `?app_id=` |
| `POST` | `/admin/v1/events/schemas` | 注册新 schema | body: `schema_ref`, `json_schema` |

### 13.1.3 用户与权限

| Method | Path | 操作 | 关键参数 |
|---|---|---|---|
| `GET` | `/admin/v1/users` | 用户列表 | `?role=&tenant_id=&page=` |
| `POST` | `/admin/v1/users` | 创建用户 | body: `email`, `display_name`, `initial_role` |
| `GET` | `/admin/v1/users/{user_id}` | 用户详情 | — |
| `PUT` | `/admin/v1/users/{user_id}` | 更新用户 | body: 可选字段 |
| `DELETE` | `/admin/v1/users/{user_id}` | 软删除 | — |
| `PUT` | `/admin/v1/users/{user_id}/roles` | 分配角色 | body: `{ "add": [...], "remove": [...] }` |
| `GET` | `/admin/v1/users/{user_id}/sessions` | 活跃会话 | — |
| `DELETE` | `/admin/v1/users/{user_id}/sessions/{session_id}` | 强制登出 | — |
| `GET` | `/admin/v1/roles` | 角色列表 | — |
| `POST` | `/admin/v1/roles` | 创建角色 | body: `name`, `permissions` |

### 13.1.4 审计与安全

| Method | Path | 操作 | 关键参数 |
|---|---|---|---|
| `GET` | `/admin/v1/audit` | 审计日志查询 | `?user=&action=&severity=&since=&until=&page=` |
| `GET` | `/admin/v1/audit/export` | 审计导出 | `?format=csv|jsonl&since=&until=` |
| `GET` | `/admin/v1/secrets/keys` | KEK 元数据列表（**不返回 key**）| — |
| `POST` | `/admin/v1/secrets/rotate-kek` | 轮换 KEK | body: `{ "new_kek_ref": "..." }` |
| `GET` | `/admin/v1/security/alerts` | 安全告警列表 | `?severity=&since=` |
| `GET` | `/admin/v1/security/policy-bypasses` | 策略绕过尝试 | — |

### 13.1.5 系统配置

| Method | Path | 操作 |
|---|---|---|
| `GET` | `/admin/v1/config` | 当前配置（敏感字段脱敏）|
| `PUT` | `/admin/v1/config` | 更新配置（热加载支持） |
| `GET` | `/admin/v1/config/slo-thresholds` | SLO 阈值 |
| `PUT` | `/admin/v1/config/slo-thresholds` | 更新 SLO 阈值 |
| `POST` | `/admin/v1/backup/trigger` | 触发手动备份 |
| `GET` | `/admin/v1/backup/history` | 备份历史 |

### 13.1.6 健康与监控

| Method | Path | 操作 |
|---|---|---|
| `GET` | `/admin/v1/health` | Admin API 自身健康 |
| `GET` | `/admin/v1/metrics/platform` | 平台指标（Prometheus 格式）|
| `GET` | `/admin/v1/metrics/apps` | 每个 App 的指标 |
| `GET` | `/admin/v1/slo/dashboard` | SLO 实时状态 |

### 13.1.7 通用约定

- **Content-Type**：`application/json`（除 SSE 外）
- **Idempotency-Key**：写操作必填，24h TTL
- **Rate Limit**：500 req/min（admin 配额高于终端）
- **错误格式**：与终端 API 一致（§11.3）
- **分页**：cursor-based（`?page=&per_page=`，最大 200）
- **OpenAPI**：`/admin/v1/openapi.json`（独立于 `/api/v1/openapi.json`）

---

## 13.2 鉴权中间件

### 13.2.1 鉴权流程

```rust
// 主 API 进程（或独立 Admin 进程）的 HTTP 中间件链
async fn admin_auth_middleware(req: Request, next: Next) -> Result<Response> {
  // 1. mTLS（V1+ Cloud）— 检查客户端证书
  #[cfg(feature = "cloud")]
  {
    let cert = req.client_cert()?;
    if !is_admin_cert(cert) {
      return Err(AdminError::Forbidden);
    }
  }

  // 2. 提取 admin session cookie
  let session = req.cookie("admin_session")
    .ok_or(AdminError::Unauthorized)?;

  // 3. 验证 JWT（独立签名密钥 ADMIN_JWT_SECRET）
  let claims = verify_admin_jwt(&session, &ADMIN_JWT_SECRET)?;

  // 4. 校验 iss=platform-admin
  if claims.iss != "platform-admin" {
    return Err(AdminError::Forbidden);  // 终端用户 JWT 不可访问
  }

  // 5. 校验 exp + 滑动续期
  if claims.exp < now() {
    return Err(AdminError::SessionExpired);
  }
  if claims.exp - now() < 300 {  // 5 分钟内过期
    extend_session(&claims)?;
  }

  // 6. 检查双因素标记
  if !claims.totp_verified {
    return Err(AdminError::TwoFactorRequired);
  }

  // 7. 检查 RBAC（粗粒度 role:admin）
  if !claims.roles.contains("admin") {
    return Err(AdminError::Forbidden);
  }

  // 8. 检查细粒度权限（资源级，ABAC）
  let required_perm = route_required_permission(&req);
  if !PolicyEngine::check(required_perm, &claims, &req).await? {
    return Err(AdminError::Forbidden);
  }

  // 9. 强制 2FA 的路由需要二次确认头
  if route_requires_recent_2fa(&req) && !claims.totp_verified_within(Duration::from_secs(60)) {
    return Err(AdminError::RecentTwoFactorRequired);
  }

  // 10. 写 admin_audit（异步）
  spawn(audit_log(req.clone(), claims.clone()));

  req.extensions_mut().insert(claims);
  next.run(req).await
}
```

### 13.2.2 细粒度权限矩阵

| 路由 | 必需权限 | 二次确认 | 双因素 |
|---|---|---|---|
| `POST /admin/v1/apps` | `apps:write` | 否 | 是 |
| `PUT /admin/v1/apps/{id}` | `apps:write` | 是（`X-Confirm: UPGRADE`）| 是 |
| `POST /admin/v1/apps/{id}/rollback` | `apps:write` | 是（`X-Confirm: ROLLBACK`）| 是 |
| `DELETE /admin/v1/apps/{id}` | `apps:write` + `apps:delete` | 是（`X-Confirm: DELETE`）| 是 |
| `POST /admin/v1/events/dlq/{id}/redeliver` | `events:write` | 否 | 否 |
| `POST /admin/v1/events/relay/pause` | `events:admin` | 是 | 是 |
| `POST /admin/v1/secrets/rotate-kek` | `secrets:write` | 是 | 是 |
| `PUT /admin/v1/users/{id}/roles` | `users:admin` | 是 | 否 |
| `GET /admin/v1/audit/export` | `audit:read` + `audit:export` | 是 | 否 |
| `PUT /admin/v1/config/slo-thresholds` | `config:write` | 是 | 否 |

### 13.2.3 双因素认证

**[PROPOSAL]** Admin 用户登录流程（继承 §7.1）：

```
[1] 用户名 + 密码
       │
       ▼
[2] bcrypt 验证 → 颁发 mfa_pending JWT（仅含 sub + 短 TTL 5min + 无 totp_verified）
       │
       ▼
[3] 客户端收 mfa_pending → 引导用户输入 TOTP / WebAuthn
       │
       ▼
[4] TOTP 验证通过 → 颁发完整 admin session JWT（15min TTL + totp_verified=true + iss=platform-admin）
       │
       ▼
[5] 设置 HttpOnly + Secure + SameSite=Strict cookie "admin_session"
```

WebAuthn 优先（YubiKey / TouchID），TOTP 兜底（Google Authenticator / 1Password）。

### 13.2.4 终端用户 JWT 与 Admin JWT 互不可见

```rust
// 关键不变量：两套签名密钥 + 两套 RBAC
const ADMIN_JWT_SECRET: &[u8] = include_bytes!("../secrets/admin_jwt.key");
const USER_JWT_SECRET: &[u8] = include_bytes!("../secrets/user_jwt.key");

// verify 函数禁止混用
fn verify_admin_jwt(token: &str) -> Result<Claims> {
  let key = DecodingKey::from_secret(ADMIN_JWT_SECRET);
  let mut validation = Validation::new(Algorithm::HS256);
  validation.set_issuer(&["platform-admin"]);  // 强制 issuer
  let token_data = decode::<Claims>(token, &key, &validation)?;
  if token_data.claims.iss != "platform-admin" {
    return Err(anyhow!("not an admin token"));
  }
  Ok(token_data.claims)
}
```

### 13.2.5 mTLS（V1+ Cloud）

```yaml
# Admin API 端点（K8s Ingress）
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: platform-admin
  annotations:
    nginx.ingress.kubernetes.io/auth-tls-secret: platform/admin-client-ca
    nginx.ingress.kubernetes.io/auth-tls-verify-client: "on"
    nginx.ingress.kubernetes.io/auth-tls-verify-depth: "2"
spec:
  tls:
    - hosts: [admin.platform.internal]
      secretName: platform-admin-tls
  rules:
    - host: admin.platform.internal
      http:
        paths:
          - path: /admin/v1
            pathType: Prefix
            backend:
              service:
                name: platform-admin
                port: { number: 3001 }
```

客户端证书由 Admin 用户自助签发（通过 SSO 引导），CA 私钥存于 HSM。

---

## 13.3 admin_audit 表 DDL

```sql
-- admin_audit — 不可变审计日志（append-only）
CREATE TABLE admin_audit (
  id              bigserial PRIMARY KEY,
  occurred_at     timestamptz NOT NULL DEFAULT now(),
  actor_user_id   text,                          -- 来自 admin JWT sub
  actor_ip        inet,
  actor_user_agent text,
  action          text NOT NULL,                 -- 'app.install', 'app.upgrade', 'user.role.assign', ...
  target_type     text,                          -- 'app', 'user', 'event', 'secret', ...
  target_id       text,
  request_id      text,                          -- 来自 X-Request-ID
  trace_id        text,                          -- 来自 traceparent
  severity        text NOT NULL DEFAULT 'info',  -- 'info' | 'warning' | 'critical'
  details         jsonb NOT NULL DEFAULT '{}'::jsonb,
  -- 防篡改：每行包含前一行 id 的 hash（hash chain）
  prev_hash       text,
  row_hash        text NOT NULL                  -- sha256(本行内容 + prev_hash)
);

CREATE INDEX idx_admin_audit_actor ON admin_audit(actor_user_id, occurred_at DESC);
CREATE INDEX idx_admin_audit_action ON admin_audit(action, occurred_at DESC);
CREATE INDEX idx_admin_audit_target ON admin_audit(target_type, target_id, occurred_at DESC);
CREATE INDEX idx_admin_audit_severity ON admin_audit(severity, occurred_at DESC)
  WHERE severity IN ('warning', 'critical');

-- 禁止 UPDATE/DELETE（即使 DB superuser 也无法直接改）
CREATE OR REPLACE FUNCTION admin_audit_no_modify() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
  RAISE EXCEPTION 'admin_audit is append-only';
  RETURN NULL;
END;
$$;

CREATE TRIGGER trg_admin_audit_no_update
BEFORE UPDATE OR DELETE ON admin_audit
FOR EACH ROW EXECUTE FUNCTION admin_audit_no_modify();
```

**校验链**：每小时 cron 跑一次 `verify_admin_audit_chain()`，重新计算每行 hash，发现不一致立即告警。

```sql
CREATE OR REPLACE FUNCTION verify_admin_audit_chain() RETURNS TABLE(id bigint, expected text, actual text)
LANGUAGE plpgsql AS $$
DECLARE
  v_prev text := '';
  v_row admin_audit%ROWTYPE;
  v_calculated text;
BEGIN
  FOR v_row IN SELECT * FROM admin_audit ORDER BY id ASC LOOP
    v_calculated := encode(digest(
      v_row.occurred_at::text || v_row.actor_user_id || v_row.action ||
      v_row.target_id || coalesce(v_row.details::text, '') || v_prev,
      'sha256'
    ), 'hex');
    IF v_calculated != v_row.row_hash THEN
      RETURN QUERY SELECT v_row.id, v_calculated, v_row.row_hash;
    END IF;
    v_prev := v_row.row_hash;
  END LOOP;
END;
$$;
```

**SIEM 转发**：OTel collector 监听 `admin_audit` 表变化（wal2json logical decoding），实时转发到 SIEM。

---

## 13.4 Ops UI 前端架构

### 13.4.1 技术栈

| 维度 | 选型 | 理由 |
|---|---|---|
| 框架 | SvelteKit 2.x | 轻量、SSR-friendly、Vite 构建快 |
| UI 库 | Svelte 5 + bits-ui（无头组件）| 高 a11y、低 runtime 成本 |
| 样式 | Tailwind CSS 4.x | 与 §5.6 终端 UI 复用 design tokens |
| 数据获取 | TanStack Query（Svelte 适配）| 缓存 / 乐观更新 / 后台刷新 |
| 实时数据 | SSE（Server-Sent Events）| 简单、双向够用（事件流 / 心跳）|
| 图表 | uPlot 或 visx | 大数据量下高性能 |
| 状态管理 | Svelte 5 runes | 内置，无需 Redux |
| 测试 | Vitest + Playwright | 单元 + E2E |
| 国际化 | i18n 框架 | i18next-sveltekit |
| 构建 | Vite 5 | 快速 HMR |

### 13.4.2 路由表

```
src/routes/
├─ +layout.svelte                      # 根布局（鉴权守卫 + nav）
├─ +layout.ts                          # 加载 admin session
├─ +page.svelte                        # 仪表盘
├─ apps/
│  ├─ +page.svelte                     # App 列表
│  ├─ [app_id]/
│  │  ├─ +page.svelte                  # App 详情
│  │  ├─ instances/
│  │  │  └─ [instance_id]/+page.svelte # Instance 详情
│  │  ├─ events/+page.svelte           # 事件历史
│  │  └─ audit/+page.svelte            # 状态转换审计
├─ events/
│  ├─ +page.svelte                     # 实时事件流
│  └─ dlq/+page.svelte                 # 死信列表
├─ users/
│  ├─ +page.svelte
│  └─ [user_id]/+page.svelte
├─ audit/+page.svelte
├─ config/+page.svelte
├─ login/
│  ├─ +page.svelte                     # 登录页（密码 + TOTP）
│  └─ mfa/+page.svelte                 # TOTP 输入
└─ +error.svelte                       # 错误页
```

### 13.4.3 鉴权守卫

```typescript
// src/routes/+layout.ts
import { redirect } from '@sveltejs/kit';
import { getAdminSession } from '$lib/api/admin';

export const load = async ({ url, fetch }) => {
  // 公开路径
  if (url.pathname.startsWith('/login') || url.pathname.startsWith('/mfa')) {
    return {};
  }

  const session = await getAdminSession(fetch);
  if (!session) {
    throw redirect(303, '/login');
  }
  if (session.requires_mfa) {
    throw redirect(303, '/mfa');
  }
  if (!session.roles.includes('admin')) {
    throw redirect(303, '/login?error=forbidden');
  }

  return { session };
};
```

### 13.4.4 关键组件：App 详情页

```svelte
<!-- src/routes/apps/[app_id]/+page.svelte -->
<script lang="ts">
  import { useQuery, useMutation } from '@tanstack/svelte-query';
  import { getApp, upgradeApp, rollbackApp } from '$lib/api/admin';

  let { data: app } = $derived.by(() => getApp(params.app_id));
  let { data: instances } = $derived.by(() => getAppInstances(params.app_id));
  let { data: metrics } = $derived.by(() => getAppMetrics(params.app_id));

  let confirmPhrase = $state('');
  const upgradeMutation = useMutation({
    mutationFn: (newManifest: string) => upgradeApp(params.app_id, newManifest),
    onSuccess: () => invalidateAll()
  });
</script>

<div class="app-detail">
  <header>
    <h1>{app.manifest.metadata.name}</h1>
    <span class="version">{app.current_version}</span>
    <span class="state" data-state={app.state}>{app.state}</span>
  </header>

  <section class="actions">
    {#if app.state === 'healthy'}
      <button onclick={() => upgradeMutation.mutate(newManifest)}>
        Upgrade to {app.available_version}
      </button>
      <button onclick={() => showRollbackDialog.set(true)}>Rollback</button>
    {/if}
    <button onclick={() => showDisableDialog.set(true)}>Disable</button>
  </section>

  <section class="instances">
    <h2>Instances ({instances.length})</h2>
    {#each instances as inst}
      <div class="instance" data-state={inst.state}>
        <span>{inst.instance_uuid}</span>
        <span class="version">{inst.app_version}</span>
        <span>Last heartbeat: {timeAgo(inst.last_heartbeat)}</span>
        {#if inst.is_leader}<span class="badge leader">LEADER</span>{/if}
        {#if inst.is_canary}<span class="badge canary">CANARY</span>{/if}
      </div>
    {/each}
  </section>

  <section class="metrics">
    <h2>Metrics (last 1h)</h2>
    <AppMetricsChart {metrics} />
  </section>
</div>

<!-- 二次确认对话框 -->
{#if showRollbackDialog}
  <Dialog>
    <h2>Rollback {app.id}</h2>
    <p>Type <code>ROLLBACK</code> to confirm:</p>
    <input bind:value={confirmPhrase} />
    <button disabled={confirmPhrase !== 'ROLLBACK'} onclick={doRollback}>
      Confirm Rollback
    </button>
  </Dialog>
{/if}
```

### 13.4.5 实时事件流（SSE）

```typescript
// src/lib/streams/event-stream.ts
export class EventStream {
  private es: EventSource;
  constructor(filter: { event_type?: string; app_id?: string }) {
    const qs = new URLSearchParams(filter as any).toString();
    this.es = new EventSource(`/admin/v1/events/stream?${qs}`, {
      withCredentials: true  // 发送 admin_session cookie
    });
  }

  on(event_type: string, handler: (event: any) => void) {
    this.es.addEventListener(event_type, (e) => {
      handler(JSON.parse(e.data));
    });
  }
}
```

```svelte
<!-- src/routes/events/+page.svelte -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { EventStream } from '$lib/streams/event-stream';

  let events = $state<any[]>([]);
  let stream: EventStream;

  onMount(() => {
    stream = new EventStream({});
    stream.on('*', (e) => {
      events = [e, ...events].slice(0, 500);  // 最近 500 条
    });
  });

  onDestroy(() => stream?.es.close());
</script>

<div class="event-stream">
  {#each events as e (e.event_id)}
    <div class="event-row" data-severity={e.severity}>
      <span class="time">{formatTime(e.occurred_at)}</span>
      <span class="type">{e.event_type}</span>
      <span class="app">{e.producer.app_id}</span>
      <span class="id">{e.event_id.slice(0, 8)}</span>
    </div>
  {/each}
</div>
```

---

## 13.5 V1+ K8s 部署清单

### 13.5.1 K8s 资源清单

```yaml
# admin-ui-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: platform-admin
  namespace: platform
  labels:
    app.kubernetes.io/component: admin-ui
    app.kubernetes.io/part-of: platform
spec:
  replicas: 2
  selector:
    matchLabels:
      app.kubernetes.io/component: admin-ui
  template:
    metadata:
      labels:
        app.kubernetes.io/component: admin-ui
    spec:
      # 强隔离：禁止调度到终端 UI 同节点
      affinity:
        podAntiAffinity:
          requiredDuringSchedulingIgnoredDuringExecution:
            - labelSelector:
                matchLabels:
                  app.kubernetes.io/component: api-server
              topologyKey: kubernetes.io/hostname
      containers:
        - name: admin-api
          image: registry.platform.local/platform-admin:v1.0.0
          ports:
            - name: http
              containerPort: 3001
          env:
            - name: ADMIN_JWT_SECRET
              valueFrom:
                secretKeyRef:
                  name: platform-secrets
                  key: admin_jwt_secret
            - name: ADMIN_LISTEN
              value: "0.0.0.0:3001"
            - name: OTEL_EXPORTER_OTLP_ENDPOINT
              value: http://otel-collector:4317
          livenessProbe:
            httpGet: { path: /admin/v1/health, port: http }
            initialDelaySeconds: 10
            periodSeconds: 30
          readinessProbe:
            httpGet: { path: /admin/v1/health, port: http }
            initialDelaySeconds: 5
            periodSeconds: 10
          resources:
            requests: { cpu: 200m, memory: 256Mi }
            limits:   { cpu: 1000m, memory: 1Gi }
          securityContext:
            runAsNonRoot: true
            runAsUser: 10001
            readOnlyRootFilesystem: true
            allowPrivilegeEscalation: false
            capabilities: { drop: [ALL] }
---
apiVersion: v1
kind: Service
metadata:
  name: platform-admin
  namespace: platform
spec:
  selector:
    app.kubernetes.io/component: admin-ui
  ports:
    - name: http
      port: 3001
      targetPort: http
---
# NetworkPolicy：仅允许 ops 网络访问
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: platform-admin-netpol
  namespace: platform
spec:
  podSelector:
    matchLabels:
      app.kubernetes.io/component: admin-ui
  policyTypes:
    - Ingress
  ingress:
    - from:
        - namespaceSelector:
            matchLabels:
              name: ops-bastion
        - podSelector:
            matchLabels:
              app.kubernetes.io/component: ingress-internal
      ports:
        - port: 3001
          protocol: TCP
---
# Ingress（仅 internal）
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: platform-admin
  namespace: platform
  annotations:
    nginx.ingress.kubernetes.io/auth-tls-secret: platform/admin-client-ca
    nginx.ingress.kubernetes.io/auth-tls-verify-client: "on"
    nginx.ingress.kubernetes.io/backend-protocol: "HTTPS"
spec:
  ingressClassName: internal-nginx
  tls:
    - hosts: [admin.platform.internal]
      secretName: platform-admin-tls
  rules:
    - host: admin.platform.internal
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service: { name: platform-admin, port: { number: 3001 } }
```

### 13.5.2 Secret 管理

```yaml
# admin-secrets.yaml (使用 External Secrets Operator)
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: platform-admin-secrets
  namespace: platform
spec:
  secretStoreRef:
    name: vault-backend
    kind: ClusterSecretStore
  target:
    name: platform-secrets
  data:
    - secretKey: admin_jwt_secret
      remoteRef:
        key: platform/admin/jwt_secret
        property: value
    - secretKey: admin_ca_cert
      remoteRef:
        key: platform/admin/ca
        property: cert
    - secretKey: admin_ca_key
      remoteRef:
        key: platform/admin/ca
        property: key
```

### 13.5.3 HPA（水平自动扩缩）

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: platform-admin
  namespace: platform
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: platform-admin
  minReplicas: 2
  maxReplicas: 5
  metrics:
    - type: Resource
      resource:
        name: cpu
        target: { type: Utilization, averageUtilization: 70 }
```

---

## 13.6 错误处理与可观测性

### 13.6.1 关键错误场景

| 场景 | 检测 | 恢复 |
|---|---|---|
| Admin 进程崩溃 | K8s liveness probe 失败 | 30s 内自动重启 |
| Admin API 鉴权失败激增 | OTel 指标 `admin_auth_failures_total` 阈值 | 触发安全告警 + 临时锁定源 IP |
| admin_audit 哈希链断裂 | 定时 verify_admin_audit_chain() 发现不匹配 | 立即告警 + 切只读模式 |
| 双因素旁路尝试 | 双因素标记缺失但用户尝试关键操作 | 强制 401 + admin_audit 记录 critical |
| mTLS 客户端证书过期 | K8s ingress 拒绝请求 | 用户收到 403 + 引导重新签发证书 |

### 13.6.2 关键 OTel 指标

| Metric | Type | Labels | 用途 |
|---|---|---|---|
| `admin_api_request_total` | counter | method, route, status | 流量 |
| `admin_api_request_duration_seconds` | histogram | method, route | P50/P99 延迟 |
| `admin_auth_failures_total` | counter | reason | 安全告警 |
| `admin_two_factor_required_total` | counter | route | 强制 2FA 触发率 |
| `admin_audit_rows_total` | gauge | — | 审计写入速率 |
| `admin_audit_chain_verification_failures_total` | counter | — | 链完整性 |
| `plugin_loader_install_duration_seconds` | histogram | app_id, outcome | 安装耗时 |
| `plugin_loader_upgrade_duration_seconds` | histogram | app_id, strategy | 升级耗时 |
| `event_stream_publish_total` | counter | event_type, app_id | 事件生产速率 |
| `event_stream_delivery_latency_seconds` | histogram | event_type | 端到端延迟 |
| `event_stream_dlq_total` | counter | event_type | 死信累计 |
| `app_heartbeat_stale_total` | gauge | app_id | 失联 instance |

---

## 13.7 关键 REQ-ID 详细映射

| REQ-ID | 实现位置 |
|---|---|
| ADMIN-REQ-001 (独立子进程) | §13.1 + §13.5.1 Deployment |
| ADMIN-REQ-002 (独立鉴权域) | §13.2.4 |
| ADMIN-REQ-003 (关键操作双因素) | §13.2.2 权限矩阵 |
| ADMIN-REQ-004 (故障隔离) | §13.5.1 podAntiAffinity + NetworkPolicy |
| ADMIN-REQ-005 (admin_audit) | §13.3 |

---

**导航 / Navigation:**
[← 12. App Registry & Plugin Loader](12-app-registry-and-plugin-loader.md) · [README](README.md)
