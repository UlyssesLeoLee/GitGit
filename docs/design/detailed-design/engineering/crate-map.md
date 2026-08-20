# Crate 详细映射

14 个 crate 的详细职责与依赖关系。

## 层级

```
Layer 0 (无业务依赖) ────────────
  errors, config, observability

Layer 1 (领域核心) ───────────────
  core (5 原语), proto (gRPC)

Layer 2 (领域引擎) ───────────────
  graph, policy

Layer 3 (应用服务) ───────────────
  ai, agent, app, git

Layer 4 (binary) ─────────────────
  server (主进程), admin (独立子进程), cli (客户端)
```

## 详细职责

| Crate | 行数目标 | 关键模块 | 状态 |
|---|---|---|---|
| `gitgit-errors` | 500 | `AppError`, `ErrorCode`, `AppResult<T>` | ✅ 已就位 |
| `gitgit-config` | 800 | `Config`, `load()`, `validate()` | ⏳ 占位 |
| `gitgit-observability` | 1200 | `tracing_init`, `metrics`, `axum_layer` | ⏳ 占位 |
| `gitgit-proto` | (生成) | `gitgit.graph.v1`, `gitgit.policy.v1`, ... | ⏳ 生成 |
| `gitgit-core` | 2500 | `Node`, `Edge`, `Event`, `Policy`, `View` 领域类型 + Repository | ⏳ 占位 |
| `gitgit-graph` | 3500 | `traverse()`, `materialize_view()`, `search()` | ⏳ 占位 |
| `gitgit-policy` | 2800 | `Engine`, `evaluate()`, `cache` | ⏳ 占位 |
| `gitgit-ai` | 3000 | `Provider`, `OpenAI`, `Anthropic`, `Local`, `scrub()` | ⏳ 占位 |
| `gitgit-agent` | 4000 | `Runtime`, `state_machine`, `workspace`, `credentials` | ⏳ 占位 |
| `gitgit-app` | 3500 | `Registry`, `Loader`, `EventBus`, `Sandbox` | ⏳ 占位 |
| `gitgit-git` | 4500 | `Server`, `gix_read`, `shell_write`, `hooks` | ⏳ 占位 |
| `gitgit-server` | 5000 | `main()`, `routes`, `middleware`, `state` | ⏳ 占位 |
| `gitgit-admin` | 3500 | `main()`, `admin_routes`, `audit`, `mfa` | ⏳ 占位 |
| `gitgit-cli` | 1500 | `main()`, `clap derive` | ⏳ 占位 |

## 实施检查清单

- [ ] 14 个 crate 都有 `Cargo.toml` + `src/lib.rs` + `README.md`
- [ ] `Cargo.lock` 提交到 git (二进制可复现)
- [ ] 每个 crate 有 `tests/` 目录
- [ ] 至少 gitgit-server / gitgit-errors 有实际代码
