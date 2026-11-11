# 架构决策记录 / Architecture Decision Records (ADR)

本目录记录**架构级别的技术决策**。每个 ADR 是一份短期、聚焦、可追溯的决策文档，遵循 [Michael Nygard 格式](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions)。

**流程管理类决策**走 [`../../process/decision-log.md`](../../process/decision-log.md)。

## 命名规约

- 文件名：`NNNN-short-kebab-case.md`（NNNN 为 4 位递增数字）
- 状态：`Proposed` / `Accepted` / `Deprecated` / `Superseded by ADR-NNNN`

## 索引 / Index

| ADR | 标题 | 状态 | 日期 |
|---|---|---|---|
| [0000](0000-template.md) | ADR 模板 | Template | — |
| [0001](0001-采用-Rust-作为主语言.md) | 采用 Rust 作为主语言 | Accepted | 2026-08-19 |
| [0002](0002-Git-读路径用-gix，写路径用-shell-`git`.md) | Git 读路径用 gix，写路径用 shell `git` | Accepted | 2026-08-19 |
| [0003](0003-采用-PostgreSQL-单一存储（不引入-Redis-NATS-Memgraph-等）.md) | 采用 PostgreSQL 单一存储（不引入 Redis/NATS/Memgraph 等） | Accepted | 2026-08-19 |
| [0004](0004-App-一级化与中心事件总线（不引入独立-Service-Mesh）.md) | App 一级化与中心事件总线（不引入独立 Service Mesh） | Accepted | 2026-08-19 |
| [0005](0005-Admin-运维界面独立子进程-+-独立鉴权域.md) | Admin 运维界面独立子进程 + 独立鉴权域 | Accepted | 2026-08-19 |
| [0006](0006-采用-OCI-容器化（Docker---containerd）+-K8s-用于-V1+-Cloud.md) | 采用 OCI 容器化（Docker / containerd）+ K8s 用于 V1+ Cloud | Accepted | 2026-08-19 |
| [0007](0007-密钥管理用-Vault---KMS（不在-DB-直接存明文）.md) | 密钥管理用 Vault / KMS（不在 DB 直接存明文） | Accepted | 2026-08-19 |
| [0008](0008-Plugin---App-Manifest-Schema-(app.yaml)-与版本兼容策略.md) | Plugin / App Manifest Schema (app.yaml) 与版本兼容策略 | Accepted | 2026-08-19 |
| [0009](0009-SIEM-适配策略（wal2json-→-Kafka---Vector---直接-webhook）.md) | SIEM 适配策略（wal2json → Kafka / Vector / 直接 webhook） | Accepted | 2026-08-19 |
| [0010](0010-app-sandbox-wasm-wasi.md) | App 沙箱采用 Wasm + WASI | Accepted | 2026-08-20 |
| [0011](0011-observability-platform.md) | 可观测性平台采用 OpenTelemetry + Prometheus + Loki + Tempo + Grafana | Accepted | 2026-08-20 |
| [0012](0012-gix-read-path-module-boundary.md) | gix 读路径与 shell `git` 写路径的模块边界 | Proposed | 2026-08-23 |
| [0013](0013-OCI-container-standard-vs-Docker-only.md) | Agent Workspace 容器标准 — OCI / containerd vs Docker-only | Proposed | 2026-08-23 |
| [0014](0014-WebAuthn-credential-library-selection.md) | WebAuthn 凭证库选型（V1+ Cloud Admin 双因素）| Proposed | 2026-08-23 |
| [0015](0015-SIEM-adapter-selection.md) | SIEM 适配器（admin_audit / events 转发）| Proposed | 2026-08-23 |
| [0016](0016-OTel-Collector-deployment-mode.md) | OpenTelemetry Collector 部署模式 | Proposed | 2026-08-23 |
| [0017](0017-HashiCorp-Vault-integration.md) | HashiCorp Vault 集成细节（MVP 本地文件 / V1+ Vault KV v2）| Proposed | 2026-08-23 |
| [0018](0018-OCI-Plugin-package-format.md) | OCI Plugin 包格式（App / Plugin Image Manifest schema）| Proposed | 2026-08-23 |
| [0019](0019-ADR-batch-0012-0018-体系化收尾.md) | ADR 编号体系化收尾（0012-0018 批次）| Proposed | 2026-08-23 |

## 流程

1. **提出** — 任何人提一份 ADR（状态 `Proposed`），用 `0000-template.md`。
2. **评审** — SA + TL + EM 评审；SEC / PO 视相关程度 Consulted。
3. **决策** — `Accepted` / `Rejected` / `Deprecated` / `Superseded by ADR-NNNN`。
4. **落地** — 关联设计文档更新 + 实施前 QA 表更新。
5. **归档** — 永不被删除；`Superseded` 时新 ADR 引用旧的。

## 状态机

```
Proposed ──→ Accepted ──→ Deprecated
    │            │
    │            └──→ Superseded by ADR-NNNN
    │
    └──→ Rejected
```

## 引用约定

- 在基本设计 / 详细设计文档中，引用 ADR 用：`见 ADR-NNNN: 标题`。
- 在 commit 消息中：`[ADR-NNNN] 简短说明`。
- 在 PR 描述中：`Closes / Supersedes / Refs ADR-NNNN`。
