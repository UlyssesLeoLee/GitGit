# Logs 设计 / Logs Design

> **关联 OBS-REQ**: OBS-LOG-001 ~ OBS-LOG-010
> **统一 SDK**: `tracing` + `tracing-subscriber` (JSON layer) + OpenTelemetry log bridge
> **Backend**: Loki（主存） + S3 / MinIO（冷存）
> **关联**: 与 [§02 Metrics](02-metrics.md) 和 [§04 Tracing](04-tracing.md) 通过 `trace_id` 关联

## 1. 日志格式

### 1.1 结构化 JSON

每条日志必须为一行 JSON：

```json
{
  "timestamp": "2026-08-20T15:30:00.123456789Z",
  "level": "INFO",
  "service.name": "gitgit-server",
  "service.version": "0.1.0",
  "deployment.environment": "prod",
  "instance.id": "gitgit-server-7f9b-abcde",
  "trace_id": "5e9c7a8b2f1d4e3a8c7b9d0e1f2a3b4c",
  "span_id": "1a2b3c4d5e6f7a8b",
  "request_id": "req-8f3a-4b2c-9d1e-7a6f5c4b3a2d",
  "tenant_id": "550e8400-e29b-41d4-a716-446655440000",
  "actor_id": "user-1234",
  "actor_type": "user",
  "module": "gitgit_server::handlers::repos",
  "event": "repo.created",
  "error_code": null,
  "duration_ms": 142,
  "message": "Repository created",
  "attributes": {
    "repo_id": "...",
    "repo_name": "org/repo"
  }
}
```

### 1.2 字段规约

| 字段 | 类型 | 必填 | 规约 |
|---|---|---|---|
| `timestamp` | RFC3339 纳秒 | ✅ | 必须含时区（`Z` 或 `+08:00`） |
| `level` | string enum | ✅ | `TRACE` / `DEBUG` / `INFO` / `WARN` / `ERROR` |
| `service.name` | string | ✅ | 见命名约定 §4 |
| `service.version` | semver | ✅ | git describe tag |
| `deployment.environment` | enum | ✅ | `local` / `dev` / `staging` / `prod` |
| `instance.id` | UUID / Pod 名 | ✅ | K8s 注入 `POD_NAME` |
| `trace_id` | 32 hex chars | ⚠️ | 有 trace 上下文时必须 |
| `span_id` | 16 hex chars | ⚠️ | 有 trace 上下文时必须 |
| `request_id` | UUID | ⚠️ | HTTP 请求级别 |
| `tenant_id` | UUID | ⚠️ | 多租户场景 |
| `actor_id` | string | ⚠️ | 触发主体（user/agent/admin）|
| `actor_type` | enum | ⚠️ | `user` / `agent` / `admin` / `system` |
| `module` | path | ✅ | 例 `gitgit_server::handlers::repos::create` |
| `event` | enum | ⚠️ | 事件类型（替代旧 message） |
| `error_code` | string | ⚠️ | ERROR 级别时必填（来自 `AppError::code()`） |
| `duration_ms` | number | ⚠️ | 业务操作时长 |
| `message` | string | ⚠️ | 人类可读，**不能含敏感信息** |
| `attributes` | object | ⚠️ | 业务相关扩展字段 |

## 2. 日志等级策略

| 环境 | 默认 level | 显式设置 |
|---|---|---|
| `local` | `DEBUG` | 通过 `RUST_LOG=gitgit_server=trace` 覆盖 |
| `dev` | `DEBUG` | `RUST_LOG=info,sqlx=warn` |
| `staging` | `INFO` | `RUST_LOG=info,sqlx=warn,hyper=warn` |
| `prod` | `INFO` | `RUST_LOG=info,warn`（不开启 debug） |

**规则**:
- `ERROR`: 业务失败 / 系统错误 / 异常路径（强制 trace_id）
- `WARN`: 可恢复的异常（DB 慢查询 / 重试 / 限流）
- `INFO`: 关键业务事件（创建 / 删除 / 状态变化）
- `DEBUG`: 详细执行路径（仅 dev/local 开启）
- `TRACE`: 协议级（仅 local）

## 3. 强制日志事件清单

| Event | Level | 必填字段 | 关联 |
|---|---|---|---|
| `service.startup` | INFO | version, config_hash | OBS-LOG-001 |
| `service.shutdown` | INFO | reason, uptime_sec | — |
| `http.request.start` | DEBUG | method, path, trace_id | — |
| `http.request.end` | INFO (5xx) / DEBUG | method, path, status, duration_ms, trace_id | OBS-LOG-002 |
| `auth.login.success` | INFO | user_id, mfa_used, ip | SEC-REQ-001 |
| `auth.login.failure` | WARN | username, reason, ip | — |
| `auth.mfa.challenge` | INFO | user_id, mfa_type | SEC-REQ-012 |
| `auth.mfa.failure` | WARN | user_id, reason | — |
| `auth.token.refresh` | DEBUG | user_id, client_id | — |
| `auth.token.expired` | INFO | user_id | — |
| `db.query.slow` | WARN | sql_hash, duration_ms, table | OBS-LOG-005 |
| `db.connection.acquire.timeout` | ERROR | pool, wait_ms | — |
| `db.deadlock` | ERROR | sql_hash, table | — |
| `git.operation.start` | DEBUG | op, repo_id | — |
| `git.operation.end` | INFO (fail) / DEBUG | op, repo_id, result, duration_ms | — |
| `git.ref.rejected` | WARN | repo_id, ref, reason | — |
| `git.hook.failed` | ERROR | hook, exit_code, stderr | OBS-LOG-010 |
| `app.install` | INFO | app_id, version | — |
| `app.upgrade.start` | INFO | app_id, from_version, to_version, strategy | — |
| `app.upgrade.complete` | INFO | app_id, version, duration_ms | — |
| `app.upgrade.failed` | ERROR | app_id, reason | — |
| `app.heartbeat.stale` | WARN | app_id, instance_id, last_seen_sec | NFR-REQ-005 |
| `app.sandbox.violation` | ERROR | app_id, violation_type, attempted_op | AISEC-REQ-013 |
| `eventbus.publish` | DEBUG | event_id, type, target_count | — |
| `eventbus.delivery.fail` | WARN | event_id, subscriber, attempt, error | — |
| `eventbus.dlq.overflow` | ERROR | subscriber, dlq_size | — |
| `ai.request` | INFO | provider, model, duration_ms, cost_microcents | — |
| `ai.prompt_injection.detected` | WARN | app_id, pattern, blocked | AISEC-REQ-001 |
| `ai.mcp.tool.denied` | WARN | tool, app_id, reason | AISEC-REQ-002 |
| `agent.run.start` | INFO | run_id, agent_id, parent_run_id | — |
| `agent.run.complete` | INFO | run_id, duration_ms | — |
| `agent.run.failed` | ERROR | run_id, error_code, message | — |
| `agent.await_approval.timeout` | WARN | run_id, hours_waiting | — |
| `policy.deny` | INFO | subject_id, action, resource_type, reason | — |
| `admin.action` | INFO | actor_id, action, target, result | SEC-REQ-011 |
| `kek.rotate.start` | INFO | operator, old_version, new_version | SEC-REQ-010 |
| `kek.rotate.complete` | INFO | duration_ms, re_encrypted_count | — |
| `kek.rotate.failed` | ERROR | reason | — |

## 4. 脱敏策略

### 4.1 禁止记录的字段（hard deny）

下列字段**永远**不进入日志（无论 level）：

```
password / passwd / pwd
token / access_token / refresh_token / jwt
secret / client_secret / api_secret
api_key / apikey
authorization (整个 header)
cookie / set-cookie / session_id
private_key / private-key
ssh_private_key / pgp_private
database_url  (含密码)
encryption_key / kek / dek (明文)
mfa_secret (明文，TOTP seed)
```

### 4.2 实现方式

**3 层防护**：

1. **静态扫描**（CI 阶段）：
   - 工具：`cargo-geiger` + 自研 `redact-scan` lint
   - 在 PR 时扫描 `tracing::info!("password = {}", ...)` 类模式

2. **结构化字段名过滤**：
   - 字段名正则匹配：`(?i)(password|token|secret|api_key|authorization|cookie|private_key)` → 不写入日志
   - 实现位置：tracing-subscriber custom layer + serde skip_serializing_if

3. **值正则过滤**（保守）：
   - `sk-[A-Za-z0-9]{20,}` (OpenAI key)
   - `ghp_[A-Za-z0-9]{20,}` (GitHub PAT)
   - `eyJ[A-Za-z0-9_-]+\.eyJ...` (JWT)
   - 实现位置：tracing-subscriber formatter 层，匹配后替换为 `<REDACTED:jwt>`

### 4.3 代码示例

```rust
// ❌ 禁止
tracing::info!("user login: password={}", password);

// ✅ 正确
tracing::info!(user_id = %user.id, "user login success");

// ❌ 禁止
tracing::warn!("auth failed: token={}", token);

// ✅ 正确（field 名含 token 自动 redact）
tracing::warn!(token_expired = true, "auth failed");
```

## 5. 采样策略

### 5.1 错误日志（ERROR）— 100% 采样

所有 ERROR 必进入日志存储，无采样。

### 5.2 WARN 采样

- 默认 100%
- 重复错误聚合：相同 `error_code` + `module` + `trace_id` 在 1 分钟内只记录 1 次

### 5.3 INFO 采样

- 业务关键事件（创建/删除/状态变化）— 100%
- HTTP 成功请求 4xx/5xx — 100%
- HTTP 2xx 成功 — 默认 10%，可配置（按 path 差异化）

### 5.4 DEBUG 采样

- 仅 local / dev 开启
- prod 不采样

## 6. 保留策略

| 数据类型 | Hot (Loki) | Cold (S3 / MinIO) | 总保留 |
|---|---|---|---|
| ERROR | 30 天 | 1 年 | 1 年 |
| WARN | 14 天 | 90 天 | 90 天 |
| INFO | 7 天 | 30 天 | 30 天 |
| DEBUG / TRACE | 1 天 | — | 1 天 |
| admin_audit (单独) | 永久 | 永久 (S3 + Object Lock) | 永久 |

**清理策略**：
- 每日 02:00 UTC 触发
- 通过 Loki 的 `compactor` 组件执行
- Cold 转移到 S3 IA / Glacier 降低存储成本

## 7. 容量估算

按 100 仓库 / 10 并发 Agent / 1000 req/s 估算：

| Level | 每日条数 | 平均行 | 每日数据 |
|---|---|---|---|
| ERROR | 100 | 800 bytes | 80 KB |
| WARN | 5,000 | 600 bytes | 3 MB |
| INFO | 8,640,000 (100/s) | 400 bytes | 3.4 GB |
| DEBUG (local only) | — | — | 0 (prod) |
| **合计 INFO+WARN+ERROR** | — | — | **~3.5 GB/日** |

→ 30 天 = **~105 GB** (Loki)
→ 1 年 cold = **~1.3 TB** (S3)

## 8. 关联 OBS-LOG

- **OBS-LOG-001**: ✅ 统一 JSON 格式
- **OBS-LOG-002**: ✅ trace_id 关联
- **OBS-LOG-003**: ✅ 强制字段
- **OBS-LOG-004**: ✅ 等级策略
- **OBS-LOG-005**: ✅ 采样策略
- **OBS-LOG-006**: ✅ 保留策略
- **OBS-LOG-007**: ✅ 脱敏 3 层
- **OBS-LOG-008**: ✅ 容量估算
- **OBS-LOG-009**: ✅ 关键事件清单
- **OBS-LOG-010**: ✅ 强制 trace 关联
