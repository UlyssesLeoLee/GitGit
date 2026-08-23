# 测试规格书 / Test Specification — 基于详细设计 v1.0

> **AI-Native Engineering Platform — Test Specification (测试规格书)**
>
> **依据 / Reference:** [`../design/detailed-design/`](../design/detailed-design/) 14 章详细设计 v1.0 + [`../design/basic-design/10-acceptance-test-policy.md`](../design/basic-design/10-acceptance-test-policy.md) MVP DoD。
>
> **范围 / Scope:** 本规格覆盖 14 章详细设计的全部可测单元 + MVP 37 项需求 + SEC-REQ 集合 + NFR-REQ 集合。粒度到"可执行的测试用例"（TC-NNN），但不包含具体测试代码（实施时由 QA + 实施工程师按本规格落地）。
>
> **状态 / Status:** v1.0 draft (2026-08-23, AI 起草)
>
> **关联 / Related:**
> - 详细设计：[`../design/detailed-design/`](../design/detailed-design/)
> - 基本设计：[`../design/basic-design/`](../design/basic-design/)
> - 测试模板：[`../process/templates/test/test-specification.md`](../process/templates/test/test-specification.md)
> - 测试计划：[`../process/templates/test/test-plan.md`](../process/templates/test/test-plan.md)
> - 实施前 QA：[`../architecture/qa-checklist.md`](../architecture/qa-checklist.md)
> - ADR 索引：[`../architecture/decisions/README.md`](../architecture/decisions/README.md)
> - F14-7 签核缺口：[`../process/F14-7-signoff-gap.md`](../process/F14-7-signoff-gap.md)
>
> **标签规约 / Tagging convention:**
> - `TC-NNN` — 测试用例 ID（前缀按章节: `TC-DL` 数据层, `TC-GE` 图谱, `TC-PE` 策略, `TC-AR` Agent 运行时, `TC-AI` AI 网关, `TC-GIT` Git, `TC-ACO` App 协调, `TC-API` API, `TC-SEC` 安全, `TC-OBS` 可观测性, `TC-ERR` 错误, `TC-APP` App Registry, `TC-ADM` Admin, `TC-NFR` 非功能）
> - 严重度：P0 (Critical) / P1 (High) / P2 (Medium) / P3 (Low)
> - 测试级别：UT (单元) / IT (集成) / ST (系统) / UAT (验收)
> - 关联需求：`REQ-NNN` / `NFR-REQ-NNN` / `SEC-REQ-NNN` / `AISEC-REQ-NNN` / `AI-REQ-NNN` / `TECH-REQ-NNN`

---

## 0. 总体测试策略 / Test Strategy

| 测试级别 | 范围 | 责任人 | 时机 | 对应流程任务 |
|---|---|---|---|---|
| **UT** | 单函数 / 单类 / 单模块 | 实施工程师 | commit 前 | 59-65 |
| **IT** | 模块间 / DB / 外部依赖(mocks) | QA + 实施 | PR 合并前 | 66-75 |
| **ST** | 全系统 + 性能 + 安全 | QA + SEC | Release 前 | 76-89 |
| **UAT** | 业务场景 + 验收 | PO + QA | Go-live 前 | 90-95 |

**MVP 37 项需求 100% 覆盖**：每条 MVP 需求至少 1 个 ST 用例 + 1 个 IT 用例。

**AISEC-REQ 集合强制覆盖**：所有 AISEC-REQ-NNN 至少 1 个 ST 安全用例 + 自动化回归。

**测试资产复用**：所有 ST 用例需在 CI 中可重复执行；性能 / 安全基准值固化在 [§15 非功能测试](#15-非功能测试non-functional-test)。

---

## 1. §00 总体概述测试 / Overview Tests

> 详细设计 [§00](../design/detailed-design/00-overview.md) 描述 monorepo 结构 + 模块划分 + 跨切关注点。

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-OV-001 | Monorepo 模块依赖方向正确 | 完整 cargo workspace | `cargo metadata --no-deps --format-version 1` | 所有 `internal/*` 模块不依赖 `cmd/*` | P0 | TECH-REQ-001 |
| TC-OV-002 | 启动 cold start ≤ 5s | 空 DB + 单 VM 4C/8G | `time platform start --config minimal.yaml` | wall time < 5s, RSS < 256MB | P0 | NFR-REQ-002 |
| TC-OV-003 | 健康检查端点 | 平台已启动 | `GET /health` | HTTP 200 + `{"status":"ok","version":"..."}` | P0 | NFR-REQ-001 |
| TC-OV-004 | graceful shutdown | 平台已启动 | `SIGTERM` → 30s 内退出, drain in-flight | exit 0, in-flight 请求 0 失败 | P0 | NFR-REQ-005 |
| TC-OV-005 | 内存安全(无 unsafe 块) | 全代码库 | `grep -rn 'unsafe' crates/` | 仅 [REQ-TS-001] 标注的 1-2 处 | P1 | TECH-REQ-001, AISEC-REQ-001 |
| TC-OV-006 | 日志结构化 | 平台运行中 | 任意一条 INFO 日志 | JSON 格式, 含 trace_id / span_id / timestamp | P1 | §10.4 |

---

## 2. §01 数据层测试 / Data Layer Tests

> 详细设计 [§01](../design/detailed-design/01-data-layer.md) 完整 DDL + AISEC-REQ-009(a) DB role 分离。

### 2.1 DDL 完整性

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-DL-001 | 5 原语表 (node/edge/event/policy/view) 存在 | 全新 DB | `\dt` | 5 个表 + 索引/外键齐全 | P0 | §01 DDL |
| TC-DL-002 | DB migration 版本可重放 | 已部署 v0.1 | `sqlx migrate run` (v0.2) | 成功 + 无 downtime | P0 | §01.6 |
| TC-DL-003 | 软删除列 (deleted_at) 一致 | — | 检查所有表 | 均含 `deleted_at TIMESTAMPTZ NULL` | P0 | §01.4 |

### 2.2 AISEC-REQ-009(a) DB Role 分离（🔴 Critical）

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-DL-010 | 5 个 deny-by-default role 创建 | 全新 DB | `\du` | `app_writer / app_reader / event_writer / admin_auditor / migration` 5 个 role | P0 | AISEC-REQ-009(a) QA-002 |
| TC-DL-011 | attacker role 不能 INSERT events | DB 已建 + 用 attacker role 连接 | `INSERT INTO events ...` | `ERROR: permission denied` | P0 | AISEC-REQ-009(a) |
| TC-DL-012 | app_writer role 不能 DELETE users | — | `DELETE FROM users WHERE id=1` | `ERROR: permission denied` | P0 | AISEC-REQ-009(a) |
| TC-DL-013 | app_reader role 不能写 | — | `UPDATE users SET ...` | `ERROR: permission denied` | P0 | AISEC-REQ-009(a) |
| TC-DL-014 | event_writer role 只能 INSERT event_stream | — | `INSERT INTO event_stream` | OK; `UPDATE event_stream` | 拒绝 | P0 | AISEC-REQ-009(a) |
| TC-DL-015 | admin_auditor role 仅可读 + 写 admin_audit | — | `INSERT INTO admin_audit` | OK; `INSERT INTO users` | 拒绝 | P0 | AISEC-REQ-009(a) |

### 2.3 RLS (Row Level Security)

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-DL-020 | App 只能读自己的命名空间 | 多 App 数据 + 切换 SET app.current_app_id | `SELECT * FROM node` | 仅返回 current_app_id 对应行 | P0 | §01.5 |
| TC-DL-021 | User 跨 tenant 隔离 | 多 tenant 数据 | User A 读 User B 数据 | 返回空集 (RLS filter) | P0 | §01.5 |
| TC-DL-022 | bypassrls role 谨慎使用 | — | 检查代码 | 仅 `migration` role 用, 业务代码零使用 | P0 | §01.5 |

---

## 3. §02 图谱引擎测试 / Graph Engine Tests

> 详细设计 [§02](../design/detailed-design/02-graph-engine.md) 5 原语 + 类型注册 + 递归 CTE + 缓存。

### 3.1 5 原语 CRUD

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-GE-001 | Node 创建 + 类型校验 | — | `POST /graph/nodes {type:"issue", ...}` | 201, 返回 id | P0 | §02.2 |
| TC-GE-002 | Node 类型注册表 | — | `SELECT * FROM node_types` | 5+ 内置类型 (issue/commit/file/agent_run/user) | P0 | §02.3 |
| TC-GE-003 | Edge 创建 + 来源边类型校验 | — | `POST /graph/edges` | 201, 校验 from/to 类型兼容性 | P0 | §02.4 |
| TC-GE-004 | Event 不可变 | — | `UPDATE events SET ...` | 拒绝(只有 INSERT + hash 链) | P0 | §02.5 |
| TC-GE-005 | Policy 表达式解析 | — | 给定 DSL `actor.role == "admin"` | 编译为可执行闭包 | P1 | §02.6 |
| TC-GE-006 | View 物化更新 | — | 写 100 个 node 后 | 关联 view 表 row 数同步 | P1 | §02.7 |

### 3.2 递归 CTE 图遍历

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|----|---|
| TC-GE-010 | 4 hop 遍历性能 | 10K node + 50K edge 合成图 | `SELECT * FROM graph_traverse(?, 4)` | < 1s (95p),  返回正确路径 | P0 | NFR-REQ-002 |
| TC-GE-011 | 循环引用检测 | 构造 A→B→A | 遍历 | 终止 + 不死循环 | P0 | §02.8 |
| TC-GE-012 | 跨类型边遍历 | 异构图 (issue→commit→file) | 4 hop | 跨类型正确 | P1 | §02.8 |
| TC-GE-013 | 遍历深度限制 | — | depth=10 超过内置 max=8 | 拒绝 + 报错 | P1 | §02.8 |
| TC-GE-014 | 大图压测 (10K 节点) | 合成图 | 4 hop x 100 并发 | P99 < 2s | P2 | NFR-REQ-002 |

### 3.3 缓存

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-GE-020 | in-process LRU 缓存 | — | 重复相同 query 100 次 | 第 2 次起 < 1ms (命中) | P1 | §02.9 |
| TC-GE-021 | 缓存失效 | — | 写新 node 后 | 相关缓存 key 失效 | P0 | §02.9 |
| TC-GE-022 | 缓存容量 | — | 写入超过 10000 entries | LRU 淘汰, 内存 < 100MB | P2 | §02.9 |

---

## 4. §03 策略引擎测试 / Policy Engine Tests

> 详细设计 [§03](../design/detailed-design/03-policy-engine.md) RBAC + ABAC + AI 策略。

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-PE-001 | RBAC 角色基础 | User role=admin | 读 user 表 | allow | P0 | SEC-REQ-001 |
| TC-PE-002 | RBAC 角色拒绝 | User role=viewer | 写 user 表 | deny + 审计 | P0 | SEC-REQ-001 |
| TC-PE-003 | ABAC 属性 (time-of-day) | 策略 `deny writes 22:00-06:00` | 在 23:00 写 | deny | P0 | §03.2 |
| TC-PE-004 | ABAC 属性 (IP range) | 策略 `deny from !10.0.0.0/8` | 从外部 IP 写 | deny | P0 | §03.2 |
| TC-PE-005 | 策略决策缓存 | — | 相同 (subject, action, resource) 1000 次 | 命中率 > 95% | P1 | §03.4 |
| TC-PE-006 | 策略评估 P99 | — | 单次评估 | < 5ms | P0 | NFR-REQ-002 |
| TC-PE-007 | 策略冲突 (deny override) | 多策略: 1 allow + 1 deny | 评估 | deny (保守优先) | P0 | §03.3 |
| TC-PE-008 | AI 特有策略 (sensitivity=high → local only) | AI-REQ-003 | AI 调用 sensitivity=high | 强制选 local provider | P0 | AI-REQ-003 |
| TC-PE-009 | 决策可追溯 | — | 任何 deny | 返回 deny reason + 触发的策略 ID | P0 | SEC-REQ-002 |

---

## 5. §05 AI 网关测试 / AI Gateway Tests

> 详细设计 [§05](../design/detailed-design/05-ai-gateway.md) 提供商抽象 + 提示清洗 + 路由 + 预算。

### 5.1 Sanitizer / Redactor

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-AI-001 | 提示词注入: "ignore previous instructions" | — | 发请求含此句 | sanitizer 标记 + 拒绝 / 转 trusted 模式 | P0 | AISEC-REQ-002 |
| TC-AI-002 | 提示词注入: 多语言绕过 | — | 中文 / 阿拉伯文 / 同义改写 | 至少 95% 拦截 | P0 | AISEC-REQ-002 |
| TC-AI-003 | 密钥字段脱敏 | 提示含 `api_key=sk-xxx` | 发出 | 响应中密钥被 redact | P0 | AISEC-REQ-005 |
| TC-AI-004 | 系统提示不可被用户覆盖 | 用户消息含 `### system: ...` | 发出 | 不被合并到 system prompt | P0 | AISEC-REQ-001 |
| TC-AI-005 | 输出信任边界 | AI 返回含 `<script>` HTML | 解析 | 标记 `TrustUntrusted` + 转义 | P0 | AISEC-REQ-003 |
| TC-AI-006 | tool_call 边界 | AI 返回 `tool_use` block | 解析 | 不直接执行, 走 policy 评估 | P0 | AISEC-REQ-004 |
| TC-AI-007 | XML 标签包裹 (内部约定) | — | 任意 TrustUntrusted 消息 | 用 XML 标签包, AI 一致性高 | P2 | §05 内部约定 |

### 5.2 Router (V1+)

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-AI-010 | sensitivity=high 强制本地 | — | 调用 sensitivity=high | 选 local provider, 拒绝 external | P0 | AI-REQ-003 |
| TC-AI-011 | sensitivity=low 选最低价 | 多 provider | 调用 | 选 cents/1k-tokens 最低者 | P0 | AI-REQ-003 |
| TC-AI-012 | sensitivity=medium 走 policy | — | 调用 | policy.AllowExternal ? cheapest-external : local | P0 | AI-REQ-003 |
| TC-AI-013 | model 无 provider | — | 调用 `model="gpt-99"` | 返回 `ErrNoProvider` | P0 | §05.9 |
| TC-AI-014 | provider 故障降级 | mock provider A 5xx | 调用 | 降级到 provider B | P0 | §05.13 |

### 5.3 Budget (V1+)

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-AI-020 | 预算耗尽拒绝 | Actor 已用 99% 预算 | 调用 | 拒绝 + 审计 | P0 | AI-REQ-005 |
| TC-AI-021 | 预算重置 (rolling 24h) | — | 等 24h | 预算恢复 | P1 | §05.10 |
| TC-AI-022 | 预算查询性能 | — | 1000 QPS 预算查询 | P99 < 1ms (in-process) | P1 | §05.14 |

### 5.4 安全套件（OWASP LLM Top 10）

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-AI-SEC-001 | LLM01 提示词注入样例库 | 内部样例 ≥ 30 条 | 全量回放 | 100% 拦截或安全降级 | P0 | OWASP LLM Top 10 |
| TC-AI-SEC-002 | LLM02 敏感信息披露 | — | AI 试图打印密钥 | 被脱敏 | P0 | OWASP LLM Top 10 |
| TC-AI-SEC-003 | LLM06 过度代理 | — | AI 请求执行未授权 tool | policy 拒绝 | P0 | OWASP LLM Top 10 |
| TC-AI-SEC-004 | LLM07 系统提示泄漏 | — | 用户尝试读取 system prompt | 拒绝 / 混淆 | P0 | OWASP LLM Top 10 |
| TC-AI-SEC-005 | LLM08 向量投毒 (V1+) | — | 注入恶意嵌入 | 检测 + 隔离 | P1 | OWASP LLM Top 10 |
| TC-AI-SEC-006 | LLM09 误信息 | — | AI 给出无证据断言 | 输出含置信度标签 | P1 | OWASP LLM Top 10 |
| TC-AI-SEC-007 | LLM10 模型 DoS | — | 大 payload | 限流 + 拒绝 | P0 | OWASP LLM Top 10 |
| TC-AI-SEC-008..010 | (保留 8-10 用于扩展) | — | — | — | — | — |

---

## 6. §06 Git 服务器测试 / Git Server Tests

> 详细设计 [§06](../design/detailed-design/06-git-server.md) CLI + libgit2 混合 + 钩子回调。

### 6.1 读路径 (gix)

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-GIT-001 | gix vs shell git log 一致性 | 真实仓库 | 对比 gix 输出与 `git log` 输出 | byte-equal (除 timestamp) | P0 | ADR-0012 |
| TC-GIT-002 | gix vs shell git ls-tree 一致性 | — | 对比 | byte-equal | P0 | ADR-0012 |
| TC-GIT-003 | gix vs shell git cat-file -p 一致性 | — | 对比 | byte-equal | P0 | ADR-0012 |
| TC-GIT-004 | gix clone 性能 | 1GB 仓库 | `gix clone` | < shell git 80% 时间 | P1 | §06.3 |
| TC-GIT-005 | gix fetch 增量更新 | — | fetch 10 commits | < 500ms | P1 | NFR-REQ-002 |

### 6.2 写路径 (shell git)

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-GIT-010 | shell git push 成功 | — | `git push` HTTP | 200, ref 更新 | P0 | §06.5 |
| TC-GIT-011 | pack-objects 由 shell git 处理 | — | push 大文件 | 正确打包, 不走 gix | P0 | ADR-0012 |
| TC-GIT-012 | receive-pack 进程触发 hook | — | push | pre/post-receive 触发 | P0 | §06.7 |
| TC-GIT-013 | pre-receive 拒绝 (V1) | pre-receive 失败 | push | 拒绝, 错误信息返回 client | P0 | §06.7 |

### 6.3 钩子

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-GIT-020 | post-receive 触发 Platform API | push 后 | 检查 Event 表 | 写入 `git.push` event | P0 | §06.7 |
| TC-GIT-021 | post-receive 失败不影响 push | hook 失败 | push | push 仍成功, 失败记 Event | P0 | §06.7 |
| TC-GIT-022 | hook 同步 vs 异步策略 | — | 1000 commits push | 不阻塞 client | P1 | §06.7 |

### 6.4 SSH (V1+, ADR-0014)

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-GIT-030 | SSH 暂未实现 (V0.x) | — | 尝试 SSH | 拒绝 + 提示"用 HTTP" | P2 | §06.9.2, ADR-0014 |
| TC-GIT-031 | V1 SSH host key 加载 (实现后) | — | SSH 连接 | 使用 Vault 中 Ed25519 host key | P0 | ADR-0014, ADR-0017 |
| TC-GIT-032 | V1 双因素 (WebAuthn + SSH key) | — | SSH 连接 + 浏览器确认 | 通过 | P0 | ADR-0014 |

### 6.5 LFS (V1+)

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-GIT-040 | LFS upload | — | `git lfs push` | 走 OCI / S3 backend | P1 | §06.10 |
| TC-GIT-041 | LFS download | — | clone LFS 文件 | 正确下载 | P1 | §06.10 |

---

## 7. §07 App 协调测试 / App Coordination Tests

> 详细设计 [§07](../design/detailed-design/07-app-coordination.md) 存储过程 + Outbox + Saga。

### 7.1 存储过程

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-ACO-001 | `coord_v1_create_app` 存储过程 | — | `CALL coord_v1_create_app(...)` | 插入 app 表 + outbox event | P0 | §07.3 |
| TC-ACO-002 | 存储过程 idempotent | — | 同 ID 调用 2 次 | 仅 1 条 app 记录, 2 条 outbox | P0 | §07.3 |
| TC-ACO-003 | 存储过程版本治理 (v1/v2) | v1 + v2 都有 | 调用 v1 | v1 行为不变 | P1 | §12.4.7 |

### 7.2 Outbox Relay

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-ACO-010 | outbox event 被 relay 派发 | — | 写 100 event | 100 个下游消费 | P0 | §07.4 |
| TC-ACO-011 | relay leader lock 互斥 | 启动 2 个 relay | — | 仅 1 个持 lock, 另 1 idle | P0 | QA-004, §07.4 |
| TC-ACO-012 | relay failover 30s 内 | kill leader | — | 30s 内另一实例接管 | P0 | NFR-REQ-005 |
| TC-ACO-013 | event 去重 | — | 同一 event_id 派发 2 次 | 下游仅收到 1 次 | P0 | §07.4 |

### 7.3 Saga

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-ACO-020 | saga 补偿回滚 | 中间步骤失败 | 启动 saga | 全部回滚 + 审计 | P0 | §07.5 |
| TC-ACO-021 | saga 超时 | — | saga 卡住 60s | 触发超时补偿 | P0 | §07.5 |
| TC-ACO-022 | saga 状态持久化 | saga 进行中 | kill 平台 | 重启后 saga 恢复 | P0 | §07.5 |

---

## 8. §08 API 处理器测试 / API Handler Tests

> 详细设计 [§08](../design/detailed-design/08-api-handlers.md) HTTP / MCP / CLI 处理器。

### 8.1 HTTP

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-API-001 | REST 路由 dispatch | — | 任意 endpoint | 正确 handler | P0 | §08.1 |
| TC-API-002 | 统一错误响应 | — | 触发 4xx/5xx | `{"code","message","trace_id"}` | P0 | §11 |
| TC-API-003 | 请求 ID 透传 | — | `X-Request-Id: foo` | 响应同 ID + 日志同 ID | P0 | §10.3 |
| TC-API-004 | 读 API P95 性能 | — | 1000 RPS 读 | P95 < 200ms | P0 | NFR-REQ-002 |
| TC-API-005 | 写 API P95 性能 | — | 200 RPS 写 | P95 < 500ms | P0 | NFR-REQ-002 |
| TC-API-006 | 限流 (rate limit) | — | 10001 req/min | 10001 拒绝 | P0 | §08.4 |

### 8.2 MCP (Model Context Protocol)

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-API-010 | JSON-RPC 2.0 协议 | — | `POST /mcp/v1/rpc` 合法请求 | 200 + result | P0 | §08.2 |
| TC-API-011 | MCP tools/list | — | 请求 | 列出可用 tools | P0 | §08.2 |
| TC-API-012 | MCP tools/call | — | 合法 tool 调用 | 执行 + 返回 | P0 | §08.2 |
| TC-API-013 | MCP 鉴权 | 无 token | 调用 | 401 | P0 | §08.2 |
| TC-API-014 | MCP 工具范围 (per-agent) | — | agent A 调用 agent B 的 tool | deny (per ADR-0020 待写) | P1 | §11.4 |

### 8.3 CLI

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-API-020 | `platform init` | — | 在空目录运行 | 创建配置文件 + DB schema | P0 | §08.3 |
| TC-API-021 | `platform backup` | — | 运行 | tar 到指定目录 | P0 | §08.3 |
| TC-API-022 | `platform restore` | — | 运行 | 从 backup 恢复 | P0 | §08.3 |
| TC-API-023 | `platform validate-manifest` | — | 跑示例 manifest | 报告错误或 OK | P0 | QA-003 |

---

## 9. §09 安全实现测试 / Security Implementation Tests

> 详细设计 [§09](../design/detailed-design/09-security-impl.md) 信封加密 + 网络策略。

### 9.1 KEK / DEK (ADR-0017)

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-SEC-010 | KEK 文件 0600 权限 | — | `ls -l keys/kek.bin` | `-rw------- platform` | P0 | ADR-0017 |
| TC-SEC-011 | 篡改 KEK 文件 | 改一个字节 | 启动平台 | 启动失败 + 提示"KEK 校验失败" | P0 | ADR-0017 |
| TC-SEC-012 | 90 天 KEK 轮换 | 模拟 90 天 | 触发轮换 | 业务读写正常 + 审计 | P0 | ADR-0017 |
| TC-SEC-013 | 篡改 encrypted_payload 任意字节 | — | 读 | 解密失败 + 审计 | P0 | ADR-0017 |
| TC-SEC-014 | 进程崩溃重启 DEK 可解封 | — | kill -9 后重启 | 旧 DEK 仍可解封 | P0 | ADR-0017 |
| TC-SEC-015 | V1+ Vault 不可达启动失败 | VAULT_ADDR 设, Vault 停机 | 启动 | 失败, 不降级 | P0 | ADR-0017 |

### 9.2 网络 / TLS

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-SEC-020 | 仅 TLS 1.3 接受 | — | TLS 1.2 客户端连接 | 拒绝 | P0 | §09.3 |
| TC-SEC-021 | 自签证书警告 | 浏览器 | 访问 | 显示证书错误 | P0 | §09.3 |
| TC-SEC-022 | mTLS (V1+) | V1 部署 | 客户端无证书 | 拒绝 | P1 | §09.3 |

### 9.3 凭据

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-SEC-030 | 密码 Argon2id 哈希 | — | 写 user | 存储 hash 而非明文 | P0 | SEC-REQ-008 |
| TC-SEC-031 | JWT 签名 Ed25519 | — | 发 token | 可用对应公钥验签 | P0 | §09.2 |
| TC-SEC-032 | JWT 过期拒绝 | 过期 token | 用之 | 401 | P0 | §09.2 |

### 9.4 admin_audit (哈希链)

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-SEC-040 | admin_audit 哈希链验证 | 100 条 audit | 调 `verify_admin_audit_chain()` | true | P0 | §13.3, QA-005 |
| TC-SEC-041 | 篡改任意一行 | 改 audit[50] | 调 verify | false + 报错 | P0 | §13.3 |
| TC-SEC-042 | UPDATE admin_audit 被拒 | — | `UPDATE` | DB 拒绝 (RLS 或 trigger) | P0 | AISEC-REQ-009(a) |

### 9.5 认证 (WebAuthn, V1+, ADR-0014)

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-AUTH-010 | WebAuthn 注册 happy path | V1 部署 | 注册流程 | 凭证入库 | P0 | ADR-0014 |
| TC-AUTH-011 | 凭证私钥不在服务端 | — | grep DB + 日志 | 零命中 | P0 | ADR-0014 |
| TC-AUTH-012 | 过期 challenge 拒绝 | — | 60s 后用 | 拒绝 | P0 | ADR-0014 |
| TC-AUTH-013 | 失败 5 次锁定 | — | 5 次失败 | 锁定 15 分钟 | P0 | ADR-0014 |
| TC-AUTH-014 | YubiKey 物理验证 | 实际 YubiKey | 注册 + 认证 | 成功 | P0 | ADR-0014 |

---

## 10. §10 可观测性测试 / Observability Tests

> 详细设计 [§10](../design/detailed-design/10-observability.md) OTel 仪表化 + 指标 + 追踪 + 日志。

### 10.1 Metrics

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-OBS-001 | Prometheus 端点 | 平台启动 | `GET /metrics` | 200 + 业务指标 | P0 | §10.2 |
| TC-OBS-002 | 关键指标存在 | — | 检查 | `platform_http_requests_total`, `platform_db_pool_*`, `platform_ai_tokens_total` 等 | P0 | §10.2 |
| TC-OBS-003 | 指标 cardinality < 10K | — | scrape 24h | 总 cardinality < 10000 | P1 | §10.2 |

### 10.2 Traces

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-OBS-010 | 5s 内 OTel 端点暴露 | 启动 | `curl :4317` | OK (进程内嵌, ADR-0016) | P0 | ADR-0016 |
| TC-OBS-011 | kill -9 collector 后主进程存活 | — | kill collector | 主进程继续, 30s 内 collector 重启 | P0 | ADR-0016 |
| TC-OBS-012 | trace 关联日志 | — | 同一请求 | 日志含 `trace_id`, 可 join | P0 | §10.4 |
| TC-OBS-013 | OTLP export 到后端 | — | 配 OTLP endpoint | 后端收到 | P0 | §10.2 |

### 10.3 Logs

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-OBS-020 | 结构化 JSON 日志 | — | 任意日志行 | JSON 格式 | P0 | §10.4 |
| TC-OBS-021 | 日志禁敏感字段 | — | grep 密钥/token/password | 零命中 | P0 | AISEC-REQ-005 |
| TC-OBS-022 | 日志级别正确 | — | 触发 warn | 出现 warn 行; 触发 debug | 不出现 (默认 info) | P1 | §10.4 |

### 10.4 admin_audit → SIEM (ADR-0015)

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-OBS-030 | admin_audit 1s 内到 OTel Collector | — | 写一条 audit | 1s 内 collector 收到 | P0 | ADR-0015 |
| TC-OBS-031 | 哈希链 verify 通过 | 1000 条 audit | 调 verify | true | P0 | QA-005 |
| TC-OBS-032 | 篡改后续行 verify 失败 | 改 audit[500] | 调 verify | false | P0 | QA-005 |
| TC-OBS-033 | exporter 切换 | `logging` → `otlp` | 切换 | 新 event 走 otlp | P1 | ADR-0015 |

---

## 11. §11 错误处理测试 / Error Handling Tests

> 详细设计 [§11](../design/detailed-design/11-error-handling.md) 错误代码 + 恢复 + 死信。

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-ERR-001 | 错误码统一 | — | 触发 10 个不同错误 | 全部含 `code + message + trace_id` | P0 | §11.1 |
| TC-ERR-002 | 4xx 不计入 metrics error | — | 触发 404 | `platform_errors_total{code="404"}` 但 HTTP 200 (counter) | P1 | §11.1 |
| TC-ERR-003 | 5xx 触发告警 | — | 触发 500 | PagerDuty / 邮件告警 | P0 | §11.2 |
| TC-ERR-004 | 死信队列 (DLQ) | — | outbox 派发失败 3 次 | 入 DLQ, 审计 | P0 | §11.3 |
| TC-ERR-005 | 错误信息不泄漏内部 | — | 触发 DB 错误 | 响应只含"内部错误", 不含 SQL | P0 | AISEC-REQ-007 |
| TC-ERR-006 | panic 恢复 | — | 触发 panic | 不 crash 主进程, log + metric | P0 | §11.4 |

---

## 12. §12 App Registry & Plugin Loader 测试

> 详细设计 [§12](../design/detailed-design/12-app-registry-and-plugin-loader.md) App Registry + Plugin Loader 进程 + 中心事件总线 + 沙箱。

### 12.1 OCI 镜像 (ADR-0013, ADR-0018)

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-APP-001 | 启动后 5s 拉取 + 运行 OCI 镜像 | — | 构建 + push 镜像 | 5s 内运行 | P0 | ADR-0013 |
| TC-APP-002 | App 不能访问其他 App fs | 2 个 App | App A 读 App B /data | 拒绝 | P0 | AISEC-REQ-013 |
| TC-APP-003 | App OOM 上报 Event | — | App 触发 OOM | 写入 `app.oom` event | P0 | ADR-0013 |
| TC-APP-004 | forbidden_grants 非空才允许加载 | manifest 空 forbidden_grants | 加载 | 拒绝 | P0 | AISEC-REQ-013, QA-003 |
| TC-APP-005 | 4 类 App 模板可生成 | webhook / http-api / mcp / storage | 跑模板 | 全部生成合法 manifest | P0 | QA-003 |
| TC-APP-006 | RLS 在 App context 下生效 | App A 写 | App B 读 | 不可见 | P0 | §12.1.9 |

### 12.2 Plugin Loader 单例

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-APP-010 | 2 个 Plugin Loader 实例仅 1 个 active | 启动 2 个 | — | 1 active + 1 idle (PG advisory lock) | P0 | QA-004, §12.2 |
| TC-APP-011 | leader failover | kill leader | — | 30s 内另一实例接管 | P0 | §12.2 |

### 12.3 多架构 (ADR-0018)

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-APP-020 | 多架构镜像 (amd64 + arm64) | arm64 节点 | 拉取 | 自动选 arm64 manifest | P1 | ADR-0018 |
| TC-APP-021 | 单架构镜像 (仅 amd64) | arm64 节点 + 仅 amd64 manifest | 拉取 | 拒绝 + 提示"无 arm64 manifest" | P1 | ADR-0018 |

### 12.4 镜像签名 (V0.5+, ADR-0018)

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-APP-030 | 篡改镜像拒绝 | V0.5+ 签名验证启用 | 改 1 字节 | 拒绝 + 审计 | P0 | ADR-0018 |
| TC-APP-031 | 合法签名通过 | — | 加载 | OK | P0 | ADR-0018 |
| TC-APP-032 | 无签名 (V0.5+) | 启用强制签名 | 加载未签名 | 拒绝 | P0 | ADR-0018 |

### 12.5 中心事件总线

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-APP-040 | 事件 100/s 单 relay | — | 写入 1000 event | 全部派发, P99 < 100ms | P0 | §13.4 |
| TC-APP-041 | 事件去重 | — | 同 event_id 派发 2 次 | 下游仅收 1 次 | P0 | §13.4 |
| TC-APP-042 | 事件顺序 | — | 同一 actor 顺序写 | 下游按序收 | P1 | §13.4 |

---

## 13. §13 Admin API & Ops UI 测试

> 详细设计 [§13](../design/detailed-design/13-admin-api-and-ops-ui.md) Admin API + admin_audit + 前端 + K8s。

### 13.1 Admin API 鉴权

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-ADM-001 | admin 鉴权域独立 | — | user token 调 admin API | 403 | P0 | §14.3.3 |
| TC-ADM-002 | admin 角色必需 | — | user with role=admin | 200 | P0 | §14.3.3 |
| TC-ADM-003 | 失败 5 次锁定 | — | 5 次失败 | 锁定 15 分钟 | P0 | §14.3.3 |

### 13.2 Admin 端点

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-ADM-010 | 端点目录完整性 | — | 跑 `platform admin list-routes` | 列出全部 admin 端点 + 文档 | P0 | §13.1 |
| TC-ADM-011 | 用户管理端点 | — | CRUD user | OK | P0 | §13.1 |
| TC-ADM-012 | 密钥轮换端点 | — | `POST /admin/keys/rotate` | 触发 KEK 轮换 | P0 | ADR-0017 |
| TC-ADM-013 | App 启停端点 | — | `POST /admin/apps/{id}/start` | 启动 App | P0 | §13.1 |

### 13.3 强审计

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-ADM-020 | admin_audit 自动写入 | — | 任意 admin 操作 | 1 条 audit (含 actor / action / target / timestamp) | P0 | §13.3 |
| TC-ADM-021 | audit 不可篡改 | — | `UPDATE admin_audit` | 拒绝 | P0 | AISEC-REQ-009(a) |
| TC-ADM-022 | audit 可查询 | — | `GET /admin/audit?actor=X` | 返回 X 的所有操作 | P0 | §13.3 |

### 13.4 Ops UI 前端

| ID | 标题 | 前置条件 | 步骤 | 预期 | 严重度 | 关联 |
|---|---|---|---|---|---|---|
| TC-ADM-030 | 登录页面加载 | — | 浏览器访问 | 渲染 + 200 | P0 | §13.5 |
| TC-ADM-031 | 仪表盘数据 | — | 登录后看 | 实时指标 (CPU / 内存 / 队列) | P0 | §13.5 |
| TC-ADM-032 | 审计查询 UI | — | 浏览器点审计 tab | 列表 + 过滤 + 详情 | P0 | §13.5 |
| TC-ADM-033 | XSS 防御 | — | 输入 `<script>alert(1)</script>` | 转义, 不执行 | P0 | OWASP Top 10 |

---

## 14. 覆盖矩阵 / Coverage Matrix

### 14.1 MVP 37 项需求覆盖

| 需求 ID | TC 数量 | 状态 |
|---|---|---|
| MVP-01 (App 群组管理) | TC-ACO-001~003, TC-APP-001~006 | 14 |
| MVP-02 (中心事件总线) | TC-APP-040~042 | 3 |
| MVP-03 (Engineering Graph 5 原语) | TC-GE-001~006 | 6 |
| MVP-04 (Policy 引擎) | TC-PE-001~009 | 9 |
| MVP-05 (Git 服务器 HTTP) | TC-GIT-001~022 | 13 |
| MVP-06 (AI 网关) | TC-AI-001~014, TC-AI-SEC-001~007 | 22 |
| MVP-07 (Agent Runtime) | (在 §4 详设的 testspec 中扩展) | — |
| MVP-08~37 | (见 [phase9-mvp-reduction.md §各需求] ) | TBD |
| **小计** | **67+** | — |

### 14.2 NFR 覆盖

| NFR | TC 数量 | 状态 |
|---|---|---|
| NFR-REQ-001 可用性 | TC-OV-002, TC-OV-004, TC-ACO-012 | 3 |
| NFR-REQ-002 性能 | TC-OV-002, TC-GE-010, TC-PE-006, TC-API-004, TC-API-005 | 5 |
| NFR-REQ-003 OS 矩阵 | (推迟 V1, [Appendix B TBD](../../design/basic-design/appendix-b-tbd.md)) | TBD |
| NFR-REQ-004 扩展性 | (V0.5 验证) | TBD |
| NFR-REQ-005 运维性 | TC-OV-004, TC-ACO-012 | 2 |
| NFR-REQ-006 安全性 | (见 §15 性能 / §15 安全) | — |

### 14.3 AISEC-REQ 覆盖

| AISEC-REQ | TC | 状态 |
|---|---|---|
| AISEC-REQ-001 内存安全 | TC-OV-005 | ✅ |
| AISEC-REQ-002 提示词注入 | TC-AI-001, TC-AI-002 | ✅ |
| AISEC-REQ-003 TrustUntrusted | TC-AI-005 | ✅ |
| AISEC-REQ-004 Tool 边界 | TC-AI-006 | ✅ |
| AISEC-REQ-005 密钥脱敏 | TC-AI-003, TC-OBS-021 | ✅ |
| AISEC-REQ-007 错误不泄漏 | TC-ERR-005 | ✅ |
| AISEC-REQ-009(a) DB role 分离 | TC-DL-010~015 | ✅ |
| AISEC-REQ-013 App 沙箱 | TC-APP-002, TC-APP-004, TC-APP-006 | ✅ |

### 14.4 ADR 覆盖

| ADR | TC |
|---|---|
| ADR-0012 gix 边界 | TC-GIT-001~003, TC-GIT-011 |
| ADR-0013 OCI 容器 | TC-APP-001, TC-APP-003 |
| ADR-0014 WebAuthn | TC-AUTH-010~014, TC-GIT-030~032 |
| ADR-0015 SIEM 适配 | TC-OBS-030~033 |
| ADR-0016 OTel Collector | TC-OBS-010~012 |
| ADR-0017 Vault 集成 | TC-SEC-010~015, TC-ADM-012 |
| ADR-0018 OCI Plugin 格式 | TC-APP-020, TC-APP-021, TC-APP-030~032 |
| ADR-0019 体系化 | (元决策, 无直接 TC) |

---

## 15. 非功能测试 / Non-Functional Test

### 15.1 性能 / Performance

| 指标 | 目标 | 测试方法 | TC |
|---|---|---|---|
| **冷启动时间** | < 5s | `time platform start` (空 DB, 单 VM 4C/8G) | TC-OV-002 |
| **读 API P99 延迟** | < 500ms | k6 1000 RPS, 1 分钟 | TC-API-004 |
| **写 API P99 延迟** | < 1s | k6 200 RPS, 1 分钟 | TC-API-005 |
| **图查询 4 hop P99** | < 1s (MVP) / < 500ms (V1) | 10K node + 50K edge 合成图 | TC-GE-010 |
| **策略评估 P99** | < 5ms | 1000 评估/秒 | TC-PE-006 |
| **AI 网关 overhead** | < 50ms (不含推理) | mock provider | TC-AI (通用) |
| **事件派发吞吐** | > 100 events/s | 1000 事件批量 | TC-APP-040 |
| **内存稳态** | < 512MB (MVP) / < 2GB (V1) | 24h soak test | TC-OV-002 |
| **DB 连接池利用率** | < 80% | 24h 生产负载 | TC-DL (通用) |

### 15.2 安全 / Security

| 项 | 测试方法 | TC |
|---|---|---|
| **OWASP Top 10 (Web)** | OWASP ZAP 自动扫描 | TC-ADM-033 |
| **OWASP LLM Top 10** | 内部样例库 + 红队 | TC-AI-SEC-001~010 |
| **SQL 注入** | sqlmap 自动 | TC-API (通用) |
| **XSS** | DOM XSS 测试集 | TC-ADM-033 |
| **CSRF** | 跨站请求伪造测试 | TC-API (通用) |
| **权限提升** | 角色矩阵负向测试 | TC-PE-002, TC-DL-012~015 |
| **密钥泄漏** | grep 整个代码 + 日志 + DB | TC-SEC-031, TC-OBS-021 |
| **依赖漏洞** | `cargo audit` 自动化 | (CI 任务) |
| **容器逃逸** | 已知 CVE 复现 | TC-APP-002 |

### 15.3 兼容 / Compatibility

| OS | 架构 | 支持等级 | TC |
|---|---|---|---|
| Linux | x86_64 | **P0** (MVP) | (推迟 V1 矩阵, [Appendix B TBD](../../design/basic-design/appendix-b-tbd.md)) |
| Linux | aarch64 | **P0** (V1+) | TC-APP-020 |
| macOS | aarch64 (Apple Silicon) | P1 (开发环境) | (推迟 V1) |
| Windows | x86_64 + WSL2 | P1 (开发环境) | (推迟 V1) |

### 15.4 可靠性 / Reliability

| 场景 | 目标 | TC |
|---|---|---|
| **PG failover 30s 内恢复** | leader lock 不丢失 | TC-ACO-012, TC-APP-011 |
| **进程崩溃重启数据不丢** | outbox / saga 恢复 | TC-ACO-022 |
| **24h soak 无 OOM** | 内存稳态 < 2GB | (V1 验证) |
| **网络分区不脑裂** | leader lock 互斥 | TC-APP-010, QA-004 |

---

## 16. 测试环境 / Test Environment

| 环境 | 用途 | 数据集 |
|---|---|---|
| **dev (本地)** | 实施工程师 UT | 合成小数据集 (< 1K 节点) |
| **ci** | PR 集成测试 | 合成中数据集 (10K 节点) |
| **staging** | QA 系统测试 | 脱敏生产快照 |
| **prod-like** | 性能 / 压测 | 同 staging, 但 K8s 部署 |
| **prod** | UAT + 监控 | 真实 |

---

## 17. CI 集成 / CI Integration

所有 TC 在 CI 中按以下矩阵自动执行:

| TC 级别 | PR 触发 | Merge 触发 | Nightly | Release 前 |
|---|---|---|---|---|
| UT (单函数) | ✅ | ✅ | ✅ | ✅ |
| IT (模块) | ✅ | ✅ | ✅ | ✅ |
| ST (系统) | ❌ (慢) | ✅ | ✅ | ✅ |
| UAT (业务) | ❌ | ❌ | ❌ | ✅ |
| 性能 (k6) | ❌ | ❌ | ✅ | ✅ |
| 安全 (OWASP ZAP / sqlmap) | ❌ | ✅ (增量) | ✅ (全量) | ✅ |
| 24h soak | ❌ | ❌ | ❌ | ✅ |

---

## 18. 签核 / Sign-off

> **F14-7 缺口状态:** 本规格与详细设计同步处于 **AI 起草 + 未人类签核** 状态。
> Phase 16 启动前需 QA Lead + EM + (安全相关 TC 增 SEC) 签核。

| 角色 | 责任 | 状态 |
|---|---|---|
| **QA Lead** | TC 完整性 + 可执行性 | 🟡 待签核 |
| **EM** | TC 覆盖度 + 资源估算 | 🟡 待签核 |
| **SEC** | §9 / §15.2 / §12 安全 TC 完整性 | 🟡 待签核 |
| **PO** | 业务场景 UAT 充分性 | 🟡 待签核 |
| **SRE** | §15.3 / §15.4 可靠性 TC | 🟡 待签核 |

---

## 19. 修订历史 / Revision History

| Date | Author | Change |
|---|---|---|
| 2026-08-23 | Mavis (AI 起草) | v1.0 初始版本,基于 [详细设计 v1.0](../design/detailed-design/) 全 14 章 + 8 个新 ADR (0018-0017) + 7 大 AISEC-REQ 集合 |

---

**导航 / Navigation:**
[← 测试计划](../process/templates/test/test-plan.md) · [← 详细设计 README](../design/detailed-design/README.md) · [← 设计 README](../design/README.md)
