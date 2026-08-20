# 实施编程前 QA 检查表 / Pre-Implementation QA Checklist

> **[PROPOSAL] 用途:** Phase 16 实施启动**之前**必须由工程负责人（+ 安全审计 + 产品 owner）走完的本检查表。表中每一项都是"如果不解决就实施、后期会反咬一口"的具体顾虑。
>
> **关联文档:**
> - [技术选型文档](tech-selection.md) — 主语言 Rust + 19 强约束
> - [基本设计 §10 受入测试方针](../design/basic-design/10-acceptance-test-policy.md) — MVP DoD
> - [基本设计 §0.2.2 不包含](../design/basic-design/00-introduction.md#022-不包含-out-of-scope) — F14-7 缺口
> - [Appendix B 残留 TBD](../design/basic-design/appendix-b-tbd.md) — 未决议题
> - [phase14-ipa-compliance-review.md §F14-1〜F14-9](../requirements/phase14-ipa-compliance-review.md) — IPA 差距分析

---

## 0. 风险等级定义 / Risk Severity

| 等级 | 含义 | 不解决的后果 | 决策人 |
|---|---|---|---|
| **🔴 Critical** | 不解决**无法启动实施**或会导致**生产事故** | Phase 16 阻断 / 数据丢失 / 安全漏洞被利用 | 工程负责人 + 安全审计 |
| **🟠 High** | 不解决会导致**重大返工**或**MVP 不达 DoD** | 关键功能缺失 / 验收不通过 | 工程负责人 + 产品 owner |
| **🟡 Medium** | 不解决会导致**已知 V1+ 缺口** | 推迟到 V1+ 但需记录 | 工程负责人 |
| **🟢 Low** | 锦上添花 | 不影响 MVP | 实施工程师 |

**总览：**
- 🔴 Critical: **6 项**
- 🟠 High: **8 项**
- 🟡 Medium: **9 项**
- 🟢 Low: **3 项**
- **共 26 项**

---

## 1. 🔴 Critical（不解决无法启动）

### QA-001 — F14-7 缺口：人类干系人正式签核流程缺位

| 字段 | 内容 |
|---|---|
| **影响范围** | 整个 Phase 16 启动的合法性；需求 → 设计 → 实现的"无人类签核"问题 |
| **当前状态** | §0.2.2 不包含内容明文标注此缺口；[00-requirements-definition.md §53 第 11 项](../requirements/00-requirements-definition.md) 也未补 |
| **后果（不解决）** | 平台以"AI 自治生成"为完整流程，无人对最终方案负责；遇监管 / 客户审查时无签核证据 |
| **缓解措施** | (1) 由产品 owner + 工程负责人 + 安全审计三方会议签核 v1.0；(2) 在 README 与设计封面记录"签核日期 + 签核人 + 签核范围"；(3) Phase 16 启动前必须完成 |
| **建议决策人** | 产品 owner + 工程负责人 + 安全审计 三方 |
| **状态** | 🟡 待启动 |
| **关联** | [phase14-ipa-compliance-review.md F14-7](../requirements/phase14-ipa-compliance-review.md) |

### QA-002 — AISEC-REQ-009(a) DB role 分离的 DDL 完整度

| 字段 | 内容 |
|---|---|
| **影响范围** | 平台核心安全保证（DB 层防御"全平台被攻陷也能保留 Event 日志"）|
| **当前状态** | 详细设计 [01-data-layer.md §1.3](../design/detailed-design/01-data-layer.md#13-db-role-分离aisec-req-009a-mvp-必填) 设计完整，但**完整 DDL 落地**需 Phase 16 验证（包括 5 个默认 deny role + 编译期 PIV 校验）|
| **后果（不解决）** | 红队最大发现防御失效；AISEC-REQ-001〜008 的所有假设（"Platform process 可信"）不成立 |
| **缓解措施** | (1) 在 sqlx migration 00-init 中实现 5 个 role；(2) 自动化测试：直接用 attacker role 尝试 `INSERT INTO events` → 应被 PG 拒绝；(3) CI 必含此测试 |
| **建议决策人** | 安全审计 + 工程负责人 |
| **状态** | 🟡 待 Phase 16 启动首日落地 |

### QA-003 — App 沙箱 AISEC-REQ-013 强化的拒绝授权清单（forbidden_grants）

| 字段 | 内容 |
|---|---|
| **影响范围** | App 不能越权写 event_stream；[§7.7.2](../design/basic-design/07-security-design.md#772-显式-deny-by-defaultaisec-req-013-强化) 强制 manifest.spec.permissions.forbidden_grants 必须非空 |
| **当前状态** | 概念定义完整（[§7.7.2](../design/basic-design/07-security-design.md#772-显式-deny-by-defaultaisec-req-013-强化) + 详设 [§12.1.9](../design/detailed-design/12-app-registry-and-plugin-loader.md#1219-rls-策略沙箱权限)），但**具体每个 App 类型的默认 forbidden_grants 模板**未列 |
| **后果（不解决）** | App 开发时需手动逐条声明；遗漏则安全漏洞 |
| **缓解措施** | (1) 在 `platform-loader` 内置 4 类 App 默认模板（webhook-consumer / http-api / mcp-tool-provider / storage-procedure）；(2) ManifestValidator 拒绝空 forbidden_grants；(3) 提供 `cargo run -- validate-manifest <app.yaml>` 工具 |
| **建议决策人** | 安全审计 + 工程负责人 |
| **状态** | 🟡 待 Phase 16 落地 |

### QA-004 — Plugin Loader 与中心事件 Relay 的 leader lock 单例保证

| 字段 | 内容 |
|---|---|
| **影响范围** | 高可用：若 2 个 Plugin Loader 实例同时启动且都抢到 leader lock（PG advisory lock 失效场景），App 升级会双重执行 |
| **当前状态** | 设计用 PG advisory lock（详设 [§12.2](../design/detailed-design/12-app-registry-and-plugin-loader.md#122-plugin-loader-进程)）单实例保证；但 PG advisory lock 在 PG failover 时行为需验证 |
| **后果（不解决）** | App 升级重复触发；事件 Relay 重复派发；中心事件去重依赖完全失效 |
| **缓解措施** | (1) 集成测试：模拟 2 个实例同时启动，验证 advisory lock 互斥；(2) 强制 Plugin Loader / Event Relay 在 deployment 模板中 `replicas: 1`（MV P阶段）；(3) 监控：若 2 个实例都持有 lock 则告警（理论上不会发生）|
| **建议决策人** | 工程负责人 |
| **状态** | 🟡 待 Phase 16 集成测试验证 |

### QA-005 — admin_audit 哈希链 + SIEM 转发的可靠性

| 字段 | 内容 |
|---|---|
| **影响范围** | 强审计保证：Admin 操作不可篡改 |
| **当前状态** | DDL 完整（详设 [§13.3](../design/detailed-design/13-admin-api-and-ops-ui.md#133-admin_audit-表-ddl)）；哈希链 + verify 函数有；OTel wal2json → SIEM 集成**待 ADR-005 落地** |
| **后果（不解决）** | Admin 操作无法独立审计；遇安全事件无法回溯 |
| **缓解措施** | (1) MVP 阶段 SIEM 转发可推迟到 V1+（本地写文件即可）；(2) 哈希链 + verify_admin_audit_chain() 在 MVP 必含；(3) CI 加自动化测试：尝试 UPDATE admin_audit → 应被 PG 拒绝 |
| **建议决策人** | 安全审计 + 工程负责人 |
| **状态** | 🟡 待 Phase 16 落地（SIEM 推迟到 V1+）|

### QA-006 — 8 个 ADR 待写

| 字段 | 内容 |
|---|---|
| **影响范围** | Phase 16 启动时多出"库选型 / 部署模式"等决策点 |
| **当前状态** | [tech-selection.md §18](../architecture/tech-selection.md#18-后续-adr-引用-future-adrs) 列出 8 个待写 ADR（gix 读路径边界 / OCI 容器标准 / WebAuthn 凭证库 / SIEM 适配器 / OTel Collector 部署 / Vault 集成 / OCI Plugin 包格式 / 7 等）|
| **后果（不解决）** | 实施时临时决策，文档不一致；后期追溯困难 |
| **缓解措施** | Phase 16 启动前**至少完成 ADR-002 (gix 读路径边界) 与 ADR-003 (OCI 容器标准)**；其余 6 个可推迟到对应功能实现前 1 周 |
| **建议决策人** | 工程负责人 |
| **状态** | 🟡 待 Phase 16 启动前 1 周 |

---

## 2. 🟠 High（不解决会导致重大返工 / MVP 不达 DoD）

### QA-007 — MVP 37 项需求的逐项覆盖度验证

| 字段 | 内容 |
|---|---|
| **影响范围** | MVP DoD；[§10 受入测试方针](../design/basic-design/10-acceptance-test-policy.md) |
| **当前状态** | [§10.1 测试级别](../design/basic-design/10-acceptance-test-policy.md#101-测试级别-test-levels) + [Appendix C §C.7 受入基准](../design/basic-design/appendix-c-ipa-mapping.md#c7-受入基准-acceptance-criteria-一览) 列出 11+1 项；本轮新增 §13/§14 后增加到 22+1 项 |
| **后果（不解决）** | 某些 MVP 需求未在基本设计落地（可能实际有但未引用），验收时漏判 |
| **缓解措施** | (1) 实施前用 grep 把每个 `GIT-REQ-NNN` / `GRF-REQ-NNN` 等映射到基本设计章节；(2) Appendix B 列出"未覆盖"项清单 |
| **建议决策人** | 工程负责人 + 产品 owner |
| **状态** | 🟡 待 Phase 16 启动前 1 周 |

### QA-008 — App 集群 + 插件 + Admin UI 升级为 MVP 后 phase9 文档未同步更新

| 字段 | 内容 |
|---|---|
| **影响范围** | MVP DoD 与实现范围的一致性 |
| **当前状态** | 用户已在对话层确认"升级"为 MVP 必填；但 [`phase9-mvp-reduction.md`](../requirements/phase9-mvp-reduction.md) 与 [`phase10-architecture.md`](../requirements/phase10-architecture.md) 文档**未同步**反映"插件热插拔 + 集群管理 + Admin UI"为 MVP 必填 |
| **后果（不解决）** | 实施时按"原 37 项 + 临时加"散乱进行；DoD 校验与文档不一致 |
| **缓解措施** | (1) Phase 16 启动前**先重审** `phase9-mvp-reduction.md` 把 §13/§14 的 11+5+4+3=23 条新 REQ 列入 MVP 必填；(2) `phase10-architecture.md` §2 §6 §7 也同步更新 |
| **建议决策人** | 产品 owner + 工程负责人 |
| **状态** | 🟡 待 Phase 16 启动前 |

### QA-009 — 性能 / 扩展性 / 灾害对策的 Provisional 等级

| 字段 | 内容 |
|---|---|
| **影响范围** | [§6 非功能设计](../design/basic-design/06-non-functional-design.md) 多处标 `[TBD] - Benchmark Required` |
| **当前状态** | NFR-REQ-001（可用性 + 灾害对策 RTO/RPO）、NFR-REQ-002（性能/扩展性 + 运用/保守性）数值均为暂定等级；NFR-REQ-003（系统环境/生态 V1-timed）也是 |
| **后果（不解决）** | MVP 上线时无法回答"能扛多少 QPS"；客户咨询时缺乏数据 |
| **缓解措施** | (1) Phase 16 启动后**首月**做基线 benchmark（k6 + wrk + 自研 micro-bench）；(2) 用结果回填 §6.1-6.6 的 Provisional 等级；(3) V1 发布前把 Provisional 改为正式等级 |
| **建议决策人** | 工程负责人 |
| **状态** | 🟡 待 Phase 16 启动首月 |

### QA-010 — sqlx 编译期 SQL 校验需 CI 有 PostgreSQL

| 字段 | 内容 |
|---|---|
| **影响范围** | CI/CD 流水线 |
| **当前状态** | `sqlx::query!` 宏在编译时连接 `DATABASE_URL` 校验 SQL；若 CI 没有 PostgreSQL 实例则编译失败 |
| **后果（不解决）** | CI 流水线编译失败；开发者本地需有 PostgreSQL 才能编译 |
| **缓解措施** | (1) CI 用 `services: postgres:` GitHub Actions service container；(2) 开发者本地用 docker-compose 启动 PostgreSQL；(3) 备选：改用 `sqlx::query`（非宏）放弃编译期校验（**不推荐**）|
| **建议决策人** | 工程负责人 + DevOps |
| **状态** | 🟡 待 Phase 16 启动首日 |

### QA-011 — gix 写路径成熟度（gitoxide）

| 字段 | 内容 |
|---|---|
| **影响范围** | Git 协议层；本设计强约束 REQ-TS-011 写路径用 shell `git`，但**读路径**仍依赖 gix |
| **当前状态** | gix 0.66+ 读路径稳定；写路径 pack/negotiation 标记 `[UNVERIFIED-FACT]` 仍不够稳定；本设计**写路径**已强约束用 shell `git`（REQ-TS-011），但需在 Phase 16 验证 gix **读路径**对 git CLI 2.30+ 的兼容性 |
| **后果（不解决）** | 边缘 case（submodule、LFS、partial clone、sparse checkout）可能解析失败 |
| **缓解措施** | (1) 在 phase10-architecture.md §3 列出的"git CLI 2.30+"子功能集做集成测试矩阵；(2) gix 解析失败时优雅降级到 shell `git` 读路径（spawn `git log` / `git cat-file`）|
| **建议决策人** | 工程负责人 |
| **状态** | 🟡 待 Phase 16 集成测试验证 |

### QA-012 — AISEC-REQ 集合（001-009）的 9 条 AI 安全需求集成测试

| 字段 | 内容 |
|---|---|
| **影响范围** | 平台 AI 安全主保证；MVP DoD 关键项 |
| **当前状态** | 9 条 AISEC-REQ 在 [§3.7.4](../design/basic-design/03-functional-design.md#374-aisec-reqai-安全的整合) + [§7.3](../design/basic-design/07-security-design.md#73-ai-安全-ai-securityaisec-req) 都有定义；但**集成测试用例**未列 |
| **后果（不解决）** | 实施时凭直觉开发，验收时无对照基线 |
| **缓解措施** | (1) 实施前编写 9 条 AISEC-REQ 对应的 9 个集成测试用例（attacker 模型 + 期望行为）；(2) CI 必含 |
| **建议决策人** | 安全审计 + 工程负责人 |
| **状态** | 🟡 待 Phase 16 启动首周 |

### QA-013 — App 升级 rolling 策略的事务性边界

| 字段 | 内容 |
|---|---|
| **影响范围** | App 升级失败时是否完整回滚 |
| **当前状态** | [§13.7.1 rolling](../design/basic-design/13-app-cluster-and-plugins.md#1371-rolling默认) + 详设 [§12.2.3 升级协议](../design/detailed-design/12-app-registry-and-plugin-loader.md#1223-升级协议rolling-策略) 描述；错误率超阈值自动回滚；但**跨进程事务**边界（App 业务事务 vs. 升级元数据事务）需仔细设计 |
| **后果（不解决）** | 升级期间半成品状态泄漏；Event stream 部分事件已派发但 instance 已停机 |
| **缓解措施** | (1) 升级前先在 metadata 写"upgrading"状态；(2) 各步骤失败 → 立即切回 + 清理；(3) 集成测试：模拟升级 50% 失败 → 验证完整回滚 |
| **建议决策人** | 工程负责人 |
| **状态** | 🟡 待 Phase 16 集成测试验证 |

### QA-014 — 中心事件总线的顺序保证

| 字段 | 内容 |
|---|---|
| **影响范围** | 跨 App 事件因果链（PROPOSAL-REQ-EVT-004）|
| **当前状态** | [§13.4.3 PROPOSAL-REQ-EVT-004](../design/basic-design/13-app-cluster-and-plugins.md#1343-关键不变量) 要求同 partition_key 内严格有序；通过 PG 事务 + `processed_at IS NULL` + 单 Relay 进程消费实现 |
| **后果（不解决）** | 事件乱序导致下游 App 状态错乱（如 `pr.opened` 在 `pr.synchronize` 之后到达）|
| **缓解措施** | (1) 单 Relay 进程顺序消费（已设计）；(2) 跨 partition 顺序**不保证**需在 schema_ref 文档化；(3) 集成测试：注入 1000 events 同 partition_key → 验证下游按序接收 |
| **建议决策人** | 工程负责人 |
| **状态** | 🟡 待 Phase 16 集成测试验证 |

---

## 3. 🟡 Medium（推迟到 V1+ 但需记录）

### QA-015 — F14-5 反映 NFR-REQ-003 系统环境矩阵 V1-timed

| 字段 | 内容 |
|---|---|
| **影响范围** | V1 平台兼容性矩阵（[§6.6](../design/basic-design/06-non-functional-design.md#66-系统环境与生态system-environment-ecology-ipa-grade-⑥)）|
| **当前状态** | MVP 仅承诺 Linux x86_64 / macOS aarch64 / Windows x86_64；ARM Linux / 旧版 Windows / 商业 Unix 推迟到 V1 |
| **后果（不解决）** | V1 发布前需重新验证完整矩阵 |
| **缓解措施** | (1) 在 [`appendix-b-tbd.md`](../design/basic-design/appendix-b-tbd.md) 明确登记；(2) V1 启动前做兼容性测试 |
| **建议决策人** | 工程负责人 |
| **状态** | 🟢 V1 启动前重审 |

### QA-016 — V1+ Cloud K8s Pod 化的迁移路径

| 字段 | 内容 |
|---|---|
| **影响范围** | V1+ Cloud 部署；MVP → Cloud 升级 |
| **当前状态** | [§8.6 V1+ K8s 部署形态](../design/basic-design/08-operations-design.md#86-v1-k8s-部署形态-v1-k8s-deployment-topology) 描述迁移路径；App Manifest 抽象按"可拆 Pod"设计 |
| **后果（不解决）** | V1 启动时可能需重写部分代码 |
| **缓解措施** | (1) MVP 阶段就保证 App Manifest schema 不绑定单进程（已做）；(2) V1 启动前做一次"App 拆 Pod"试点（取 1 个真实 App）；(3) Plugin Loader V1+ 改为 K8s Operator 模式 |
| **建议决策人** | 工程负责人 |
| **状态** | 🟢 V1 启动前重审 |

### QA-017 — Cargo workspace crate 拆分

| 字段 | 内容 |
|---|---|
| **影响范围** | 编译时间 / 依赖边界 / 测试隔离 |
| **当前状态** | tech-selection.md 报告结尾给出 9 个 crate 建议拆分；未在 `Cargo.toml` 落地 |
| **后果（不解决）** | 单 crate 编译时间 5-10 分钟（不可接受）|
| **缓解措施** | Phase 16 启动首日写 `Cargo.toml` workspace 模板 + 9 个 crate 骨架 |
| **建议决策人** | 工程负责人 |
| **状态** | 🟡 待 Phase 16 启动首日 |

### QA-018 — MCP 协议自实现的客户端兼容性

| 字段 | 内容 |
|---|---|
| **影响范围** | 平台与 Claude Desktop / Cursor / Continue 等 MCP 客户端的互操作 |
| **当前状态** | [技术选型 §14](../architecture/tech-selection.md#14-mcp-协议实现) 决定自实现；未做客户端兼容性测试 |
| **后果（不解决）** | 实际部署时客户端连接失败 |
| **缓解措施** | (1) 与 3 个主流客户端（Claude Desktop / Cursor / Continue）做集成测试；(2) 跟踪 MCP 协议版本（目前 2024-11-05）|
| **建议决策人** | 工程负责人 |
| **状态** | 🟡 待 Phase 16 MVP 验收 |

### QA-019 — 长时间运行的 Tokio task 内存增长

| 字段 | 内容 |
|---|---|
| **影响范围** | 7×24 平台运行稳定性 |
| **当前状态** | 平台需持续运行；Tokio task 结构体泄漏需验证 |
| **后果（不解决）** | 内存缓慢增长 → 几天后 OOM |
| **缓解措施** | (1) 72 小时 soak test；(2) `dhat` / `heaptrack` 内存分析；(3) tracing instrumentation 检测 task 数量与生命周期 |
| **建议决策人** | 工程负责人 |
| **状态** | 🟡 待 Phase 16 启动首月 |

### QA-020 — Appendix B 残留 TBD 项目清理

| 字段 | 内容 |
|---|---|
| **影响范围** | Phase 16 启动前的"未决议题"清单 |
| **当前状态** | [appendix-b-tbd.md](../design/basic-design/appendix-b-tbd.md) 列出所有未决议题 |
| **后果（不解决）** | Phase 16 实施时遇到 TBD 临时决策，文档与代码不一致 |
| **缓解措施** | (1) Phase 16 启动前**逐项过** appendix-b；(2) 每项 TBD 决定：解决 / 推迟到 V1+ / 接受风险 |
| **建议决策人** | 工程负责人 + 产品 owner |
| **状态** | 🟡 待 Phase 16 启动前 1 周 |

### QA-021 — Admin UI 鉴权域分离的具体测试

| 字段 | 内容 |
|---|---|
| **影响范围** | SEC-REQ-011 强制约束 |
| **当前状态** | [详设 §13.2.4](../design/detailed-design/13-admin-api-and-ops-ui.md#1324-终端用户-jwt-与-admin-jwt-互不可见) 设计完整；具体测试用例未列 |
| **后果（不解决）** | 实施时易混淆两套密钥 / cookie 域 |
| **缓解措施** | (1) 集成测试：终端用户 JWT 访问 `/admin/v1/*` → 403；(2) Admin JWT 访问 `/api/v1/*` → 403；(3) CI 必含 |
| **建议决策人** | 安全审计 + 工程负责人 |
| **状态** | 🟡 待 Phase 16 启动首周 |

### QA-022 — Cargo-deny 许可证检查

| 字段 | 内容 |
|---|---|
| **影响范围** | 合规性：禁止 GPL/AGPL 依赖 |
| **当前状态** | [tech-selection.md §16 风险表](../architecture/tech-selection.md#16-关键风险与缓解-risks-mitigations) 提出 CI 加 `cargo-deny`；未落地 |
| **后果（不解决）** | 误引入 GPL 依赖 → 平台必须开源 |
| **缓解措施** | Phase 16 启动首日加 `deny.toml` 配置 + CI 步骤 |
| **建议决策人** | 工程负责人 |
| **状态** | 🟡 待 Phase 16 启动首日 |

### QA-023 — Cargo-chef + sccache 编译缓存

| 字段 | 内容 |
|---|---|
| **影响范围** | CI 构建时间（目标：≤ 3 分钟）|
| **当前状态** | tech-selection.md §16 提出；未落地 |
| **后果（不解决）** | CI 完整构建 15-20 分钟（不可接受）|
| **缓解措施** | (1) Dockerfile 用 `cargo-chef`；(2) CI 用 `sccache` 远程缓存；(3) 拆分 workspace crate 减少重编译 |
| **建议决策人** | DevOps + 工程负责人 |
| **状态** | 🟡 待 Phase 16 启动首日 |

---

## 4. 🟢 Low（锦上添花）

### QA-024 — 详细设计 `[IMPL]` 标记的 Rust 代码示例

| 字段 | 内容 |
|---|---|
| **影响范围** | 实施时需重写为可编译的 Rust |
| **当前状态** | 各章都有 `[IMPL]` 块（如 [详设 §7.4.2 存储过程调用入口](../design/detailed-design/07-app-coordination.md#742-存储过程调用入口)）；是伪代码 / 类型化伪代码，不是可直接 cargo build 的代码 |
| **后果（不解决）** | 实施时需重写；可能引入 bug |
| **缓解措施** | Phase 16 实施时直接写新代码；不必照搬 `[IMPL]` 块（伪代码级别）|
| **建议决策人** | 实施工程师 |
| **状态** | 🟢 Phase 16 实施时重写 |

### QA-025 — 平台数据导出/导入 (DATA-REQ-001) 验证流程

| 字段 | 内容 |
|---|---|
| **影响范围** | "无锁定"承诺的可信度 |
| **当前状态** | 需求 §DATA-REQ-001 + §DATA-REQ-002 明确要求导出/导入的结构化差异验证；`phase15-final-audit.md` 标记"待 V1 试点" |
| **后果（不解决）** | 客户对"无锁定"声明存疑 |
| **缓解措施** | (1) MVP 阶段用现有 pg_dump/pg_restore 即可（数据完整性自动保证）；(2) V1 阶段做"独立第三方审计导出/导入流程"|
| **建议决策人** | 产品 owner |
| **状态** | 🟢 V1 启动前重审 |

### QA-026 — OCI Plugin Image Manifest Schema 标准化

| 字段 | 内容 |
|---|---|
| **影响范围** | App 分发格式（V1+ 第三方 App 生态）|
| **当前状态** | tech-selection.md §18 列出 ADR-008 待写；MVP 阶段 App 不通过 OCI 分发（直接从 Git 加载）|
| **后果（不解决）** | V1+ 第三方 App 上架时可能需重定义格式 |
| **缓解措施** | V1 启动前写 ADR-008；考虑 OCI Artifact spec 复用 |
| **建议决策人** | 工程负责人 |
| **状态** | 🟢 V1 启动前 |

---

## 5. QA 流程 / QA Process

### 5.1 时序

```
[Phase 15 终审]
  ↓
[Phase 16 启动前 2 周]  跑本 QA 表
  ↓ 标记每项状态
  ↓
[Phase 16 启动前 1 周]  Critical + High 必须全关闭
  ↓
[Phase 16 启动首日]    落实 Cargo workspace / cargo-deny / CI PostgreSQL / sqlx compile-time check
  ↓
[Phase 16 启动首周]    AISEC-REQ 集成测试用例 / admin_audit 哈希链测试 / Admin 鉴权域分离测试
  ↓
[Phase 16 启动首月]    Performance benchmark / 72h soak test / 验证 Provisional 等级
  ↓
[Phase 16 验收]        MVP DoD 逐项验证
  ↓
[V1 启动前 1 月]       Medium 项收尾 / 8 个 ADR 全部完成
```

### 5.2 关闭标准

每项 QA 关闭需满足：

- [x] 有具体解决方案（或明确接受风险）
- [x] 责任人有签字
- [x] 解决方案落地（如果是 Critical / High）
- [x] 关闭后写入 [Appendix B 残留 TBD](../design/basic-design/appendix-b-tbd.md) 或本表「已关闭」区

### 5.3 责任分配

| 角色 | 负责的 QA 项 |
|---|---|
| **产品 owner** | QA-001 / QA-007 / QA-008 / QA-020 / QA-025 |
| **安全审计** | QA-001 / QA-002 / QA-003 / QA-005 / QA-012 / QA-021 |
| **工程负责人** | QA-004 / QA-006 / QA-009 / QA-010 / QA-011 / QA-013 / QA-014 / QA-015 / QA-016 / QA-017 / QA-018 / QA-019 / QA-020 / QA-022 / QA-023 / QA-026 |
| **DevOps** | QA-010 / QA-022 / QA-023 |
| **实施工程师** | QA-024 |

---

## 6. 关联文档清单

- [技术选型文档](tech-selection.md) — 主语言 + 库选型
- [基本设计](../design/basic-design/) — 18 个文件（00-14 + 4 附录）
- [详细设计](../design/detailed-design/) — 14 个文件
- [需求定义书](../requirements/00-requirements-definition.md) — 37 项 MVP 需求
- [需求 phase14 IPA 差距](../requirements/phase14-ipa-compliance-review.md) — F14-1〜F14-9
- [需求 phase15 终审](../requirements/phase15-final-audit.md) — 一致性核查

---

**导航 / Navigation:**
[← 技术选型文档](tech-selection.md) · [需求定义书](../requirements/00-requirements-definition.md) · [基本设计 README](../design/basic-design/README.md)
