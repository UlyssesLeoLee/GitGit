# Appendix D — 用语集 / Glossary

> 本附录对齐 **IPA 共通框架 2013** 的"共通语汇"（标准术语），并补充本平台专有术语与略语。读者在阅读任何章节时遇到不熟悉的术语可在此查阅。

---

## D.1 IPA 共通框架 2013 标准术语

### D.1.1 过程 (Process)

| 中文 | 英文 | 本书用途 |
|---|---|---|
| 过程 | Process | 共通框架最上层结构。例：需求定义过程 / 系统方式设计过程 |
| 活动 | Activity | 过程内子单位。例：P3.A1 / P3.A2 / P3.A3 / P3.A4 |
| 任务 | Task | 活动内具体工作步骤。例：P3.A1.T1「识别功能要求」 |
| 交付物 | Deliverable | 过程产出的文档 / 制品。例：需求定义书 / 系统方式设计书 |
| 输入 / 输出 | Input / Output | 任务的依赖与产出 |
| 阶段 | Phase | 多个过程的逻辑分组。例：需求定义阶段 / 开发阶段 |
| 评审 | Review | 过程间的质量关卡 |
| 可追溯性 | Traceability | 需求↔设计↔实现↔测试的关联 |
| 工作流 | Workflow | 任务执行的顺序与并行结构 |
| 关卡 | Gate | 阶段间的进入/退出条件 |

### D.1.2 制品 (Artifact)

| 中文 | 英文 | 本书对应 |
|---|---|---|
| 系统方式设计书 | System Architecture Design Document | 即"本基本设计书" |
| 程序方式设计书 | Program Architecture Design Document | 即"详细设计书" 的一部分 |
| 软件详细设计书 | Software Detailed Design Document | 即"详细设计书" 的一部分 |
| 软件需求书 | Software Requirements Specification | 即"需求定义书" |
| 验收测试规格书 | Acceptance Test Specification | [10. 受入测试方针](10-acceptance-test-policy.md) |
| 非功能要求等级 | NFR Grade | [06. 非功能设计](06-non-functional-design.md) 主体 |
| 共通框架 | Common Frame | IPA 共通框架 2013 |

### D.1.3 设计术语

| 中文 | 英文 | 本书用途 |
|---|---|---|
| 系统边界 | System Boundary | [01. 系统概述 §1.3 系统边界](01-system-overview.md#13-系统边界-system-boundary) |
| 功能需求 | Functional Requirement | 8 子系统（GRF/AGT/AI/...）|
| 非功能需求 | Non-Functional Requirement | 6 大项 (可用性/性能/...）|
| 接口 | Interface | HTTP API / Git protocol / MCP / Webhook / CLI |
| 硬件构成 | Hardware Configuration | [§2.2 部署单位](02-architecture.md#22-部署单位-deployable-units来自-phase-10-4) |
| 软件构成 | Software Configuration | 同上 |
| 数据模型 | Data Model | [04. 数据设计](04-data-design.md) |
| 迁移方式 | Migration Method | [09. 迁移设计](09-migration-design.md) |
| 可靠性 | Reliability | [06.1 可用性](06-non-functional-design.md#61-可用性availability-ipa-grade-①) |
| 维护性 | Maintainability | [06.3 运行与维护性](06-non-functional-design.md#63-运行与维护性operability-maintainability-ipa-grade-③) |
| 可移植性 | Portability | [06.4 可迁移性](06-non-functional-design.md#64-可迁移性migration-portability-ipa-grade-④) |
| 扩展性 | Scalability | [06.2 性能与扩展性](06-non-functional-design.md#62-性能与扩展性performance-scalability-ipa-grade-②) |

### D.1.4 安全术语 (IPA 安全相关)

| 中文 | 英文 | 本书用途 |
|---|---|---|
| 访问控制 | Access Control | [§7.1 认证授权](07-security-design.md#71-认证-授权-authentication-authorization) |
| 认证 | Authentication | 同上 |
| 授权 | Authorization | 同上 |
| 审计日志 | Audit Log | [§7.2 审计](07-security-design.md#72-审计-audit) |
| 未授权访问 | Unauthorized Access | AISEC-REQ 全部 |
| 密钥管理 | Key Management | [§7.5 密钥管理](07-security-design.md#75-密钥管理-secrets-managementsec-req-005-sec-req-010) |
| 加密 | Encryption | 信封加密 / TLS / AEAD |
| 多因子认证 | MFA | V1+ OIDC |
| 零信任 | Zero Trust | Agent 默认 deny (SEC-REQ-004) |

## D.2 非功能要求等级 (NFR Grade) 6 大项

**IPA 非功能要求等级 2018 改订版** 的 6 大项分类（[06. 非功能设计](06-non-functional-design.md) 全部采用）：

| 序号 | 大项 | 子项数 | 本书章节 |
|---|---|---|---|
| ① | 可用性 (Availability) | 3 子项 | [06.1 可用性](06-non-functional-design.md#61-可用性availability-ipa-grade-①) |
| ② | 性能·扩展性 (Performance / Scalability) | 6 子项 | [06.2 性能与扩展性](06-non-functional-design.md#62-性能与扩展性performance-scalability-ipa-grade-②) |
| ③ | 运行·保守性 (Operability / Maintainability) | 6 子项 | [06.3 运行与维护性](06-non-functional-design.md#63-运行与维护性operability-maintainability-ipa-grade-③) |
| ④ | 迁移性 (Migration / Portability) | 5 子项 | [06.4 可迁移性](06-non-functional-design.md#64-可迁移性migration-portability-ipa-grade-④) |
| ⑤ | 安全性 (Security) | 5 子项 | [06.5 安全性](06-non-functional-design.md#65-安全性security-ipa-grade-⑤) + [07. 安全设计](07-security-design.md) |
| ⑥ | 系统环境·生态 (System Environment / Ecology) | 5 子项 | [06.6 系统环境与生态](06-non-functional-design.md#66-系统环境与生态system-environment-ecology-ipa-grade-⑥) |

## D.3 本平台专有术语

### D.3.1 5 原语 (Five Primitives)

| 原语 | 英文 | 含义 |
|---|---|---|
| 节点 (Node) | Node | 图谱中的有类型对象（含 Repository / Issue / PR / Agent / **App** / **AppInstance** 等）|
| 边 (Edge) | Edge | 节点之间的有向、带类型连接（含 **app_depends_on** / **app_publishes_event** / **app_consumes_event** 等）|
| 事件 (Event) | Event | 不可变追加型记录（兼审计日志；含 **app_deployment** 状态转换事件）|
| 策略 (Policy) | Policy | 评估输入并返回 allow/deny 的规则集 |
| 视图 (View) | View | 命名查询，可带参数 |

### D.3.2 节点类型 (Node Types)

完整的 Node 类型注册表见 [`../detailed-design/01-data-layer.md` §1.4.3 类型注册表](../detailed-design/01-data-layer.md#143-类型注册表-type-registry) 与 [`02-graph-engine.md` §2.4 类型注册表](../detailed-design/02-graph-engine.md#24-类型注册表-type-registry)。

常用节点类型简表：

| 类型 | 含义 | 文档位置 |
|---|---|---|
| `repository` | 仓库 | [§4.1 概念数据模型](04-data-design.md#41-概念数据模型-conceptual-data-model) |
| `issue` | 问题 | 同上 |
| `pull_request` | PR / MR | 同上 |
| `agent` | Agent 定义 | 同上 |
| `agent_run` | Agent 执行实例 | 同上 |
| `incident` | 事故 | 同上 |
| `requirement` | 需求 | 同上 |
| `adr` | 架构决策记录 | 同上 |

### D.3.3 边类型 (Edge Types)

| 类型 | 含义 | 例 |
|---|---|---|
| `blocks` | A 阻塞 B | `issue -blocks-> issue` |
| `implements` | A 实现 B | `pr -implements-> requirement` |
| `reviewed_by` | A 被 B 评审 | `pr -reviewed_by-> human` |
| `generated_by` | A 由 B 生成 | `commit -generated_by-> agent_run` |
| `motivated_by` | A 由 B 引起 | `incident -motivated_by-> issue` |
| `cancels` | A 取消 B | 反向 / 软删除 |
| `evidence` | A 是 B 的证据 | 带 `epistemic_status` 必填字段 |

完整边类型注册表与 Edge 服务实现见 [`../detailed-design/02-graph-engine.md` §2.4 类型注册表](../detailed-design/02-graph-engine.md#24-类型注册表-type-registry)。

### D.3.4 App 集群与可热插拔术语（§13 / §14 引入）

| 术语 | 英文 | 含义 | 文档位置 |
|---|---|---|---|
| App | Application | 平台一级部署单元，与 Repository / Agent 平级 | [§13.1](13-app-cluster-and-plugins.md#131-概念模型app-作为一级平台对象) |
| AppInstance | App Instance | App 的运行实例（一实例一节点）| [§13.1](13-app-cluster-and-plugins.md#131-概念模型app-作为一级平台对象) |
| AppDeployment | App Deployment | App 状态转换事件（不可变）| [§13.6.2](13-app-cluster-and-plugins.md#1362-状态转换事件) |
| App Manifest | app.yaml | App 的"身份证 + 能力清单 + 依赖声明"| [§13.2](13-app-cluster-and-plugins.md#132-app-manifest-appyaml-schema) |
| Plugin Loader | Plugin Loader | 监听 App 节点变化、启动/停止 instance 的独立进程 | [详细设计 §12.2](../detailed-design/12-app-registry-and-plugin-loader.md#122-plugin-loader-进程) |
| 中心事件总线 | Central Event Bus | 跨 App 事件分发基础设施（PG LISTEN/NOTIFY + Outbox 扩展）| [§13.4](13-app-cluster-and-plugins.md#134-中心事件总线-central-event-bus) |
| 事件 Schema Registry | Event Schema Registry | 事件 schema 版本化注册表 | [详细设计 §12.1.4](../detailed-design/12-app-registry-and-plugin-loader.md#1214-中心事件-schema-注册表) |
| 死信队列 | Dead-Letter Queue (DLQ) | 事件重试耗尽后的兜底表 | [§13.4.4](13-app-cluster-and-plugins.md#1344-dead-letter-queue死信) |
| 升级策略 | Upgrade Strategy | rolling / blue-green / canary 三选一 | [§13.7](13-app-cluster-and-plugins.md#137-升级策略-upgrade-strategy) |
| 优雅停机 | Graceful Shutdown | 拒绝新请求 → 完成 in-flight → flush outbox → 退出 | [§13.6.4](13-app-cluster-and-plugins.md#1364-优雅停机) |
| 沙箱 | Sandbox | App 独立 schema + 独立 DB role + 显式权限边界 | [§7.7](07-security-design.md#77-app-沙箱权限边界-app-sandbox-permission-boundary) |
| Admin UI | Admin UI | 平台管理员运维界面（独立子进程、独立鉴权域）| [§14](14-admin-ops-ui.md) |
| Admin API | Admin API | `/admin/v1/*` 端点族，与终端 API 物理隔离 | [§11.11](11-api-design.md#1111-admin-api-端点族-admin-api-endpoint-family) |
| Plugin API | Plugin API | `/api/apps/<app_id>/*` 端点族，App 动态注册 | [§11.12](11-api-design.md#1112-plugin-api-端点族-plugin-api-endpoint-family) |
| admin_audit | admin_audit | Admin 操作不可变审计表（哈希链 + SIEM 转发）| [§14.2.5](14-admin-ops-ui.md#1425-审计日志) |
| 双因素认证 | Two-Factor Auth (2FA) | TOTP 或 WebAuthn，关键操作必填 | [§14.3.3](14-admin-ops-ui.md#1433-admin-独立鉴权边界sec-req-011) |
| Pod | K8s Pod | V1+ Cloud 每 App 一个 Pod | [§8.6](08-operations-design.md#86-v1-k8s-部署形态-v1-k8s-deployment-topology) |

## D.4 略语 / Abbreviations

### D.4.1 IPA 相关

| 略语 | 全称 | 中文 |
|---|---|---|
| IPA | Information-technology Promotion Agency, Japan | 独立行政法人情报处理推进机构 |
| IPA/SEC | IPA Software Engineering Center | IPA 软件工程中心 |
| NFR | Non-Functional Requirements | 非功能需求 |
| ABAC | Attribute-Based Access Control | 属性基础访问控制 |
| RBAC | Role-Based Access Control | 角色基础访问控制 |
| NFR Grade | Non-Functional Requirements Grade | 非功能要求等级 |

### D.4.2 平台技术略语

| 略语 | 全称 | 中文 | 出处 |
|---|---|---|---|
| MCP | Model Context Protocol | 模型上下文协议 | [§11.4 MCP 设计](11-api-design.md#114-mcp-设计) |
| DDL | Data Definition Language | 数据定义语言 | [§4.2 物理 schema](04-data-design.md#42-物理-schema-physical-schemapostgresql) |
| DML | Data Manipulation Language | 数据操作语言 | - |
| DSN | Data Source Name | 数据源连接串 | - |
| JWT | JSON Web Token (RFC 7519) | JSON Web 令牌 | [§4.7.1 Token 格式](../detailed-design/04-agent-runtime.md#471-token-格式) |
| UUID v7 | Universally Unique Identifier v7 (RFC 9562) | UUID 第 7 版（时间排序） | [§4.5 节点 ID 设计](../detailed-design/02-graph-engine.md#24-类型注册表-type-registry) |
| GIN | Generalized Inverted Index | 通用倒排索引（PG 索引类型） | [§1.5 索引策略](../detailed-design/01-data-layer.md#15-索引策略-index-strategy) |
| BRIN | Block Range Index | 块范围索引（PG 索引类型）| 同上 |
| RLS | Row-Level Security (PG) | 行级安全策略 | [§1.6 行级安全](../detailed-design/01-data-layer.md#16-行级安全-rls) |
| CTE | Common Table Expression (SQL) | 公用表表达式 | [§4.7.3 递归 CTE](../detailed-design/01-data-layer.md#173-图遍历递归-cte) |
| KEK | Key Encryption Key | 密钥加密密钥 | [§7.5 密钥管理](07-security-design.md) |
| DEK | Data Encryption Key | 数据加密密钥 | 同上 |
| AEAD | Authenticated Encryption with Associated Data | 带关联数据的认证加密 | AES-256-GCM |
| AEAD AAD | Associated Authenticated Data | AEAD 关联数据 | [§7.5 密钥管理 §AAD 设计](07-security-design.md#75-密钥管理-secrets-managementsec-req-005-sec-req-010) |
| OTel | OpenTelemetry | 开放遥测标准 | [§10 可观测性](../detailed-design/10-observability.md) |
| OTLP | OpenTelemetry Protocol | OTel 传输协议 | 同上 |
| SSE | Server-Sent Events | 服务器推送事件 | [§5.12 流式响应](../detailed-design/05-ai-gateway.md#512-流式响应) |
| mTLS | Mutual TLS | 双向 TLS 认证 | [§9.6.3 mTLS](../detailed-design/09-security-impl.md#963-mtls-cloud) |
| WAL | Write-Ahead Log (PG) | 预写日志 | [§8.2 备份恢复](08-operations-design.md#82-备份-恢复-backup-recoverybkp-req) |
| PITR | Point-In-Time Recovery | 时点恢复 | 同上 |
| GC | Garbage Collection | 垃圾回收 | [§3.1 Git Server](03-functional-design.md#31-git-server-子系统-git-server-subsystem) |
| LFS | Large File Storage (Git) | 大文件存储 | [§6.10 LFS 支持](../detailed-design/06-git-server.md#610-lfs-支持-v1) |
| OCI | Open Container Initiative | 开放容器标准 | [§4.6 工作区隔离](../detailed-design/04-agent-runtime.md#46-工作区隔离-workspace-isolation) |
| CRI | Container Runtime Interface | 容器运行时接口 | 同上 |
| GUC | Grand Unified Configuration (PG `current_setting`) | PG 会话级配置 | [§7.4 存储过程](../detailed-design/07-app-coordination.md#74-存储过程plpgsql调用) |
| JSONB | JSON Binary (PG 类型) | PG 二进制 JSON 类型 | [§4.1 概念数据模型](04-data-design.md#41-概念数据模型-conceptual-data-model) |
| CEL | Common Expression Language (Google) | 通用表达式语言 | [§3.8 ABAC](../detailed-design/03-policy-engine.md#38-abac-属性求值) |
| SPA | Single Page Application | 单页应用 | [§5.6 UI 概述](05-interface-design.md#56-ui-概述-ui-overview) |
| PL/pgSQL | Procedural Language / PostgreSQL | PG 过程化 SQL | [§7.4 存储过程](../detailed-design/07-app-coordination.md#74-存储过程plpgsql调用) |
| DPoP | Demonstrating Proof-of-Possession (OAuth) | OAuth 防重放扩展 | V1+ (TBD) |
| OpenAPI 3.1 | OpenAPI Specification 3.1 | OpenAPI 规范 3.1 | [§8.10 OpenAPI 治理](../detailed-design/08-api-handlers.md#810-openapi-治理) |
| Pact | Pact Contract Testing | 契约测试工具 | [§8.10 OpenAPI 治理](../detailed-design/08-api-handlers.md#810-openapi-治理) |
| Saga | Long-running Transaction Pattern | 长事务模式 | [§7.7 Saga engine](../detailed-design/07-app-coordination.md#77-saga-引擎) |
| Outbox | Transactional Outbox | 事务性发件箱 | [§7.6 Outbox + Relay](../detailed-design/07-app-coordination.md#76-outbox-relay) |
| HMAC | Hash-based Message Authentication Code | 基于哈希的消息认证码 | [§11.6 Webhook 签名](11-api-design.md#116-webhook-outbound-api) |
| DLQ | Dead-Letter Queue | 死信队列 | [§7.6.5 死信队列](../detailed-design/07-app-coordination.md#765-死信队列) |
| CRDT | Conflict-free Replicated Data Type | 无冲突复制数据类型 | V1+ (TBD) |
| LRU | Least Recently Used (cache) | 最近最少使用缓存 | [§2.9 缓存](../detailed-design/02-graph-engine.md#29-缓存-caching) |
| TLS | Transport Layer Security | 传输层安全 | [§9.6 TLS](../detailed-design/09-security-impl.md#96-tls) |
| DLQ | Dead-Letter Queue | 死信队列 | [§7.6.5 死信队列](../detailed-design/07-app-coordination.md#765-死信队列) |
| 2FA | Two-Factor Authentication | 双因素认证 | [§14.3.3 双因素认证](14-admin-ops-ui.md#1433-admin-独立鉴权边界sec-req-011) |
| WebAuthn | Web Authentication (W3C) | 浏览器双因素标准 | [§14.3.3 双因素认证](14-admin-ops-ui.md#1433-admin-独立鉴权边界sec-req-011) |
| TOTP | Time-based One-Time Password | 基于时间的一次性密码 | [§14.3.3 双因素认证](14-admin-ops-ui.md#1433-admin-独立鉴权边界sec-req-011) |
| SIEM | Security Information & Event Management | 安全信息与事件管理 | [§13.3 admin_audit DDL](../detailed-design/13-admin-api-and-ops-ui.md#133-admin_audit-表-ddl) |
| HSM | Hardware Security Module | 硬件安全模块 | [§13.2.5 mTLS](../detailed-design/13-admin-api-and-ops-ui.md#1325-mtlsv1-cloud) |
| K8s | Kubernetes | 容器编排系统 | [§8.6 V1+ K8s 部署形态](08-operations-design.md#86-v1-k8s-部署形态-v1-k8s-deployment-topology) |
| Helm | Helm Chart (K8s) | K8s 应用打包工具 | [§13.5.1 K8s 资源清单](../detailed-design/13-admin-api-and-ops-ui.md#1351-k8s-资源清单) |
| HPA | Horizontal Pod Autoscaler (K8s) | K8s 水平自动扩缩 | [§13.5.3 HPA](../detailed-design/13-admin-api-and-ops-ui.md#1353-hpa水平自动扩缩) |
| SSE | Server-Sent Events | 服务器推送事件 | [§13.4.5 实时事件流](../detailed-design/13-admin-api-and-ops-ui.md#1345-实时事件流sse) |
| ACK | Acknowledgement | 确认应答 | [§12.4.4 路由注册协议](../detailed-design/12-app-registry-and-plugin-loader.md#1244-http-入口命名空间) |

### D.4.3 Rust 生态核心库（[技术选型文档](../../architecture/tech-selection.md) §0 决策摘要）

| 库 | 全称 | 中文 | 出处 / 用途 |
|---|---|---|---|
| **Rust** | Rust Programming Language | Rust 编程语言 | [§0 / §2 主语言](../../architecture/tech-selection.md#2-主语言决策rust-language-decision) |
| MSRV | Minimum Supported Rust Version | 最低支持 Rust 版本 | 1.75（[§0 决策摘要](../../architecture/tech-selection.md#0-决策摘要-decision-summary)）|
| Edition | Rust Edition | Rust 版本（编译期语法）| 2021 |
| **Tokio** | Tokio Async Runtime | Tokio 异步运行时 | [§3 运行时与异步](../../architecture/tech-selection.md#3-运行时与异步tokio-async-runtime) |
| **Axum** | Axum Web Framework | Axum Web 框架 | [§4 Web 框架](../../architecture/tech-selection.md#4-web-框架axum-http-framework) |
| **Tonic** | Tonic gRPC | Tonic gRPC 框架 | [§5 gRPC](../../architecture/tech-selection.md#5-grpctonic-inter-service-rpc) |
| **sqlx** | SQLx (Rust) | Rust 异步 SQL 工具包 | [§6 PostgreSQL 驱动](../../architecture/tech-selection.md#6-postgresql-驱动sqlx-database-driver) |
| **gix** | gix (gitoxide) | Rust 原生 Git 库 | [§7.1 Git 库选型](../../architecture/tech-selection.md#71-决策) |
| **hyper** | hyper (HTTP) | hyper HTTP 库 | Axum 底层 |
| **tower** | tower (Middleware) | Tower 中间件生态 | Axum 中间件层基础 |
| **tokio-postgres** | tokio-postgres | Tokio 异步 PG 驱动 | sqlx 内部使用 |
| **reqwest** | reqwest HTTP Client | reqwest HTTP 客户端 | [§10 HTTP 客户端](../../architecture/tech-selection.md#10-http-客户端reqwest-http-client) |
| **jsonwebtoken** | jsonwebtoken (Rust) | JWT 签发/验证 | [§8 认证](../../architecture/tech-selection.md#8-认证-凭证jsonwebtoken-argon2-auth) |
| **argon2** | argon2 (Rust) | Argon2id 密码哈希 | [§8 认证](../../architecture/tech-selection.md#8-认证-凭证jsonwebtoken-argon2-auth) |
| **RustCrypto** | RustCrypto 套件 | Rust 加密原语套件 | [§9 加密原语](../../architecture/tech-selection.md#9-加密原语rustcrypto-cryptography) |
| **aes-gcm** | AES-256-GCM (Rust) | AES-GCM 加密 | 信封加密 |
| **sha2** | SHA-2 (Rust) | SHA-256 哈希 | 数据指纹 |
| **hmac** | HMAC (Rust) | HMAC 消息认证 | Webhook 签名 |
| **tracing** | tracing (Rust) | Rust 结构化日志 + 追踪 | [§11 可观测性](../../architecture/tech-selection.md#11-可观测性tracing-opentelemetry-observability) |
| **OpenTelemetry** | OpenTelemetry (Rust SDK) | OTel Rust SDK | [§11 可观测性](../../architecture/tech-selection.md#11-可观测性tracing-opentelemetry-observability) |
| **OTel** | OpenTelemetry | 开放遥测标准 | 同上 |
| **OTLP** | OpenTelemetry Protocol | OTel 传输协议 | 同上 |
| **metrics** | metrics-rs | Rust 指标抽象 | Prometheus 导出 |
| **figment** | figment (Config) | Rust 配置加载 | [§12 配置](../../architecture/tech-selection.md#12-配置figment-configuration) |
| **clap** | Command Line Argument Parser | Rust CLI 解析 | [§13 其他库](../../architecture/tech-selection.md#13-其他关键库) |
| **thiserror** | thiserror (Error) | Rust 库错误定义 | 库代码 |
| **anyhow** | anyhow (Error) | Rust 二进制错误处理 | 二进制 |
| **serde** | serde (Serialize) | Rust 序列化框架 | API 边界 + 存储 |
| **chrono** | chrono (Time) | Rust 时间库 | PostgreSQL TIMESTAMPTZ 友好 |
| **uuid** | uuid (UUID) | Rust UUID 库 | 节点 ID v7 |
| **jsonschema** | jsonschema (Rust) | Rust JSON Schema 校验 | App Manifest / 事件 Schema |
| **Cargo** | Cargo (Rust) | Rust 包管理器与构建工具 | 编译 + 依赖管理 |
| **cargo-chef** | cargo-chef | Docker 依赖缓存 | CI 构建加速 |
| **sccache** | sccache | 远程编译缓存 | CI 构建加速 |
| **cargo-deny** | cargo-deny | 依赖许可证审计 | CI 检查禁止 GPL/AGPL |
| **MSRV** | Minimum Supported Rust Version | 最低支持 Rust 版本 | 1.75 |
| **OCI** | Open Container Initiative | 开放容器标准 | [§13 其他库](../../architecture/tech-selection.md#13-其他关键库) |
| **MCP** | Model Context Protocol | 模型上下文协议 | 自实现（§14）|

## D.5 命名规约 / Naming Conventions

详见 [`01. 数据层 §1.2 命名与规约`](../detailed-design/01-data-layer.md#12-命名与规约-naming-convention) 与 [§4.1 命名规约](../detailed-design/04-agent-runtime.md#43-数据类型) 中相关章节。

**REQ-ID 规约**（贯穿全文档）：

| 前缀 | 含义 | 例 |
|---|---|---|
| `GIT-REQ-NNN` | Git 相关 | GIT-REQ-001 |
| `GRF-REQ-NNN` | Graph（图谱）相关 | GRF-REQ-008 |
| `AGT-REQ-NNN` | Agent 相关 | AGT-REQ-007 |
| `AI-REQ-NNN` | AI / Gateway 相关 | AI-REQ-001 |
| `CTX-REQ-NNN` | Context Engine 相关 | CTX-REQ-001 |
| `SEC-REQ-NNN` | 安全 / RBAC 相关 | SEC-REQ-008 |
| `AISEC-REQ-NNN` | AI 安全（提示词注入等）相关 | AISEC-REQ-001 |
| `CI-REQ-NNN` | CI / CD 相关 | CI-REQ-001 |
| `OPS-REQ-NNN` | 运维 / 部署相关 | OPS-REQ-001 |
| `CLOUD-REQ-NNN` | Cloud 部署相关 | CLOUD-REQ-001 |
| `UX-REQ-NNN` | UX / 用户体验相关 | UX-REQ-003 |
| `API-REQ-NNN` | API 设计相关 | API-REQ-001 |
| `DATA-REQ-NNN` | 数据 / 移植性相关 | DATA-REQ-001 |
| `CDX-REQ-NNN` | Codex / 外部 AI Agent 集成相关 | CDX-REQ-001 |
| `SRCH-REQ-NNN` | 搜索相关 | SRCH-REQ-001 |
| `ART-REQ-NNN` | 制品 / 制品库相关 | ART-REQ-001 |
| `OBS-REQ-NNN` | 可观测性相关 | OBS-REQ-001 |
| `BKP-REQ-NNN` | 备份 / 恢复相关 | BKP-REQ-001 |
| `REL-REQ-NNN` | 可靠性相关 | REL-REQ-001 |
| `NFR-REQ-NNN` | 非功能要求等级（Phase 14 新增）| NFR-REQ-001 |
| `APP-REQ-NNN` | **App 集群与可热插拔（§13 新增）**| APP-REQ-001 |
| `ADMIN-REQ-NNN` | **Admin 运维界面（§14 新增）**| ADMIN-REQ-001 |
| `PROPOSAL-REQ-APP-NNN` | **App Manifest 校验规则（§13.2.2 新增）**| PROPOSAL-REQ-APP-001 |
| `PROPOSAL-REQ-EVT-NNN` | **中心事件不变量（§13.4.3 新增）**| PROPOSAL-REQ-EVT-001 |
| `PROPOSAL-REQ-SEC-APP-NNN` | **App 沙箱安全约束（§7.7 新增）**| PROPOSAL-REQ-SEC-APP-001 |
| `PROPOSAL-REQ-SEC-ADMIN-NNN` | **Admin 独立鉴权域约束（§7.8 新增）**| PROPOSAL-REQ-SEC-ADMIN-001 |
| `PROPOSAL-REQ-COORD-APP-NNN` | **App 命名空间与跨 App 存储过程（§7.4.4 新增）**| PROPOSAL-REQ-COORD-APP-001 |
| `PROPOSAL-REQ-API-ADMIN-NNN` | **Admin API 端点族约束（§11.11 新增）**| PROPOSAL-REQ-API-ADMIN-001 |
| `PROPOSAL-REQ-API-PLUGIN-NNN` | **Plugin API 端点族约束（§11.12 新增）**| PROPOSAL-REQ-API-PLUGIN-001 |
| `PROPOSAL-REQ-OPS-K8S-NNN` | **V1+ K8s 部署形态约束（§8.6 新增）**| PROPOSAL-REQ-OPS-K8S-001 |
| `TECH-REQ-NNN` | **技术选型（[架构 / 技术选型文档](../../architecture/tech-selection.md)）**| TECH-REQ-001 |

## D.6 阶段标记 / Phase Tags

| 标签 | 含义 | 用法 |
|---|---|---|
| `[FACT]` | 一次源验证过的事实 | 文档内引用 |
| `[UNVERIFIED-FACT]` | 多个二次源印证但本会话未取得一次源 | 文档内引用 |
| `[INFERENCE]` | 基于已知事实的合理推断 | 文档内推断 |
| `[PROPOSAL]` | 本项目自身的设计主张 | 设计建议，未与外部源对应 |
| `[TBD]` | 未确定 | 后续阶段 / 待决策 |
| `[IMPL]` | 需要实现阶段直接照搬的代码片段 | 详细设计中的代码示例 |

---

**导航 / Navigation:**
[← Appendix C — IPA 过程·交付物 对照表](appendix-c-ipa-mapping.md) · [README](README.md)
