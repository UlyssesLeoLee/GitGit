# 数据库 Migrations

按版本顺序管理 schema 变更。每次 migration 包含：
- `NNNN_xxx.up.sql` — 应用变更
- `NNNN_xxx.down.sql` — 回滚变更（用于开发环境）

## 命名规则
- 序号 `NNNN` 4 位递增
- `xxx` kebab-case 简短描述

## 列表（按 §01-data-layer.md 14 节）

| # | 名称 | 主要内容 | 关联 REQ |
|---|---|---|---|
| 0001 | primitives | nodes / edges / events / policies / views 5 原语 | GRF-REQ-001/004, AISEC-REQ-009(a) |
| 0002 | type_registry | 类型注册表 + JSON Schema | GRF-REQ-002 |
| 0003 | permissions | roles / permissions / role_permissions / user_roles | SEC-REQ-002 |
| 0004 | users_auth | users / sessions / WebAuthn / TOTP | SEC-REQ-001/004 |
| 0005 | secrets | 信封加密：secrets / keks (3 层) | SEC-REQ-005/010 |
| 0006 | coordination | coord_sagas / outbox / call_log | (Saga / Outbox) |
| 0007 | app_registry | apps / heartbeats / subscriptions / DLQ | APP-REQ-001/004, AISEC-REQ-013 |
| 0008 | git_repos | git_repositories | GIT-REQ-001/002 |
| 0009 | agents_runs | agents / agent_runs / credentials | AGT-REQ-001/002/004/009 |
| 0010 | view_snapshots | 物化视图 (CTX-REQ-002 重建支持) | CTX-REQ-002 |
| 0011 | tenants_rls | tenants + RLS 启用 (CLOUD-REQ-003) | CLOUD-REQ-003 |
| 0012 | audit | audit_log + 哈希链 + 防篡改触发器 | SEC-REQ-003/011 |
| 0013 | db_role_separation | gitgit_app / gitgit_audit_readonly roles | AISEC-REQ-009(a) |
| 0014 | seed | 初始 permissions | — |

## 应用顺序

本地开发用 sqlx-cli：
```bash
sqlx migrate run
```

## sqlx offline mode

CI 中用：
```bash
cargo sqlx prepare --workspace
```

生成 `.sqlx/` 目录供离线编译使用。
