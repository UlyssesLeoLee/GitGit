# ADR-0023: V0 Desktop App 整合 — 吸收 `apps/desktop` 的 knowledge-graph 能力到 `apps/gm-desktop`

| 字段 | 值 |
|---|---|
| **Status** | Accepted (2026-09-26 JST) |
| **Supersedes** | (无) |
| **Superseded by** | (无) |
| **Authors** | Ulysses — MinimaxM3 接手 agent |
| **Reviewers** | Ulysses (DDD Review, pending) |
| **Deciders** | Ulysses (per 2026-09-26 03:19 JST "确认") |
| **Tags** | gitgit, desktop, tauri, svelte, knowledge-graph, consolidation, gm-desktop, uis |

---

## 1. 背景 / Context

2026-09-19 起 `dev` 上同时存在两套 Tauri 2 + Svelte 5 桌面应用:

- **`apps/desktop`** (ULYS-129, PR #11) — 本地只读 knowledge-graph viewer,直读 `docs/requirements/` 渲染 Node/Edge 图谱。1,834 行,**无 ADR 依据**,与 gitgit server crate 零耦合。
- **`apps/gm-desktop`** (ADR-0020) — "Git + TortoiseGit" 本地壳,内嵌 axum 调用 gitgit server,做仓库管理 / Credential Vault / AI Provider Key / 托盘。2,529 行,**架构层与 ADR-0020 决议绑定**,已 commit `2f8b9fc` 但**未通过完整 build 验证**(实测 `cargo check` 报 7+ 编译错误:`vault_versioned` 模块不存在 / `tauri_plugin_log::Config` API 变化 / `axum` 缺依赖 / 类型不匹配等 — 见 §5 已知风险)。

两者均命名 "GitGit 桌面应用" 但覆盖面完全不同,易混淆。2026-09-26 02:35 JST Ulysses 拍板:**"选择更优秀的,并吸收另一分支优点"**。

## 2. 决策 / Decision

**以 `apps/gm-desktop` 为主干,吸收 `apps/desktop` 的 knowledge-graph 能力作为 `gm-desktop` 的第 6 个路由 `/graph`。合并完成后,在独立 commit 中 `git rm -r apps/desktop`**。

理由(按权重):

1. **`gm-desktop` 是 ADR-0020 正式决议** — 撤销/废弃它需新 ADR,而非简单代码删除。
2. **`gm-desktop` 复用 gitgit server crate**(Cargo `path = "../../.."`),架构层与 "gitgit 平台 = 一组 Rust crates" 对齐;`apps/desktop` 是闭门 demo,与 gitgit 血脉断裂。
3. **`gm-desktop` 是产品** — 一键启停 / 凭证保险库 / 仓库管理是真实用户场景;`apps/desktop` 是 knowledge-graph *查看器*,缺写操作就不是产品。
4. **`gm-desktop` 工程度更高** — i18n (zh-CN + en) / 暗色模式 / 4 个 vitest 单测 / ErrorBoundary / ToastHost / LocaleSwitcher / ThemeToggle / ServerStatusBar / 托盘 / 多语言 — `apps/desktop` 仅 1 个 Node smoke 测试。

## 3. 吸收清单 / Absorption Map — 拆 PR-B / PR-C / PR-D

吸收计划按依赖链拆 3 个 PR,每个独立 build/test 可验证,可独立 revert:

### 3.1 PR-B (本 PR) — Rust 端知识图谱引擎

| 来源 (`apps/desktop`) | 目的地 (`apps/gm-desktop/src-tauri/`) | 说明 |
|---|---|---|
| `src-tauri/src/graph.rs` (378 行 + 4 个单测) | `src/graph.rs` | markdown → Node/Edge 解析引擎,直读 `docs/requirements/`,带 4 个 `graph::tests` 单测 |
| `src-tauri/src/commands.rs` 中 graph_* / docs_* 7 个 IPC | `src/commands/graph.rs` | 子模块化,与 `server/vault/repos/auth/system` 平级 |
| (依赖) `regex` + `walkdir` | `src-tauri/Cargo.toml` | 增量依赖,版本号与 `apps/desktop` 对齐 |
| (接入) `mod graph; pub mod graph;` | `src-tauri/src/lib.rs` + `src/commands/mod.rs` | 仅注册 mod + `invoke_handler!` 7 个 |

本 PR 不做前端路由/UI;只动 Rust。`cargo check`/`cargo test --lib` 通过即可独立合并。
后续 PR-B 单独使用 `feature/consolidate-desktop` 分支 → `dev`。

### 3.2 PR-C (后续) — Svelte 前端 `/graph` 路由

| 来源 (`apps/desktop`) | 目的地 (`apps/gm-desktop/src/`) | 说明 |
|---|---|---|
| `src/lib/parser.ts` (234 行) | `src/lib/graph/parser.ts` | 前端 TS 解析逻辑,与 Rust 实现 1:1 对应 |
| `src/lib/api.ts` (152 行) | `src/lib/api/graph.ts` | Tauri invoke 包装,接入 `gm-desktop` 现有的 `lib/api/tauri.ts` 风格 |
| `src/lib/store.ts` (159 行) | `src/lib/stores/graph.ts` | svelte 5 runes 的 writable |
| `src/components/{Topbar,Sidebar,GraphView,Detail}.svelte` | `src/routes/Graph.svelte` (新) | 三栏布局整合为单路由,复用 `gm-desktop` 现有 `Sidebar` |
| `scripts/smoke.mjs` (152 行) | `tests/unit/graph-parser.test.ts` | 改写为 vitest,与现有 4 个测试并列 |
| README §"What it does" + "Live parse numbers" | `gm-desktop/README.md` §"Graph viewer (V0.1+)" 新章节 | 保留"Local-first engineering knowledge graph viewer" 定位 |

PR-C 依赖: PR-B 已合 + gm-desktop 上游 svelte-check 预存在 106 错误的 fix (新工作)。

`apps/gm-desktop/src/App.svelte` 当前路由表:
```
/           → Home
/repos      → Repos
/repos/:name → RepoDetail
/vault      → Vault
/settings   → Settings
```
PR-C 新增:`/graph` → Graph(knowledge-graph viewer)。`Sidebar` 第 5 项导航加 `t('nav.graph')` + zh-CN/en 文案。

### 3.3 PR-D (后续 cleanup) — `apps/desktop` 删除

合并 PR-B + PR-C 后,**单独立一个 cleanup commit**(`chore(desktop): remove apps/desktop per ADR-0023 §3.3`)执行 `git rm -r apps/desktop`。根 `/.gitignore` 中对应的 `apps/desktop` ignore 条目同时清理。git 历史保留 — `b0ed0e6` 仍可通过 `git show b0ed0e6 -- apps/desktop` 回放。

注:apps/desktop 的 `scripts/smoke.mjs` 保留在 git 历史即可,不再 forward-port (PR-C 已改写为 vitest)。

## 4. 风险与缓解 / Risks

| 风险 | 缓解 |
|---|---|
| `gm-desktop` **从未完整 build 通过**(实测 7+ 编译错误,见 §5) | 本 PR 优先修复 build,再合 graph 能力 — 或拆 2 个 PR:PR-A 修 gm-desktop build,PR-B 吸收 desktop |
| `gitgit::server::vault_versioned` 模块在 gitgit server crate 中已被 ULYS-123 (commit d3d184e) 重命名为 `minio` 命名空间,gm-desktop 未跟进 | PR-A 同步重命名,或临时 `pub mod vault_versioned;` 兼容垫片 |
| `tauri_plugin_log::Config::default()` 在 tauri-plugin-log v2.9 移除 | 用 `tauri_plugin_log::Builder::new()` 或保持 `init()` 简版 |
| `axum` 在 gm-desktop `Cargo.toml` 缺失(`server.rs` 用 `axum::serve`) | 在 `[dependencies]` 加 `axum = { version = "0.7", default-features = false, features = ["http1", "tokio"] }`,或让 gitgit server crate 重导出 |
| CI (`.github/workflows/`) 是否覆盖 gm-desktop 子 crate | gm-console CI 在 `apps/gm-console`,gm-desktop 需单独加 workflow |
| 跨平台签名 / 自动更新 / 真实图标占位(沿用 gm-desktop 已有的 V0.1 占位) | 不在本 PR 范围,沿用 ADR-0020 §2.7 占位策略 |

## 5. 已知前置问题 / Pre-existing Blockers (非本 ADR 引入)

2026-09-26 实测 `apps/gm-desktop/src-tauri/` `cargo check --offline` 报以下**预存在**错误(均在原始 2f8b9fc commit 引入,从未修复):

```
error[E0255]: the name `commands` is defined multiple times  // lib.rs 缺 mod 声明
error[E0432]: unresolved imports `gitgit::server::vault_versioned::{VersionEntry, VersionDiff}`
error[E0433]: cannot find `Config` in `tauri_plugin_log`      // API 变化
error[E0425]: cannot find function `init_with_config` in `tauri_plugin_log`
error[E0433]: cannot find module or crate `axum` in this scope  // Cargo.toml 缺 axum
error[E0308]: mismatched types in `commands::server::spawn_embedded_server` return
error[E0308]: mismatched types in `commands::vault` `Ok(new_version)` 期望 i32
```

**这些错误阻塞 ADR-0023 的 build 验证**。处理路径:

- **方案 A(已选 ✓, 2026-09-26 ~13:49)**:拆 2 个 PR — PR-A #14 `fix(desktop): gm-desktop 7 build errors per pre-existing audit` 已合 `549bd0c`,PR-B 本分支 `feat(desktop): absorb apps/desktop into gm-desktop per ADR-0023` 待提交。两者合并后才能完整 `cargo check` + `npm run build`。
- ~~方案 B~~:在 `feature/consolidate-desktop` 一个 PR 内既修 build bug 又加 graph 能力 — 已废弃,commit 历史可读性差。

## 6. 范围 / Scope

### 6.1 In scope — PR-B (本 ADR 单 PR)

Rust 端知识图谱引擎 — 仅 `apps/gm-desktop/src-tauri/` 域内改动:

- `apps/gm-desktop/src-tauri/src/graph.rs` (新,378 行 + 4 单测)— 从 `apps/desktop/src-tauri/src/graph.rs` 拷,不改逻辑
- `apps/gm-desktop/src-tauri/src/commands/graph.rs` (新,144 行,7 个 IPC)— 从 `apps/desktop/src-tauri/src/commands.rs` 拆,改 `Result<T, AppError>` → `Result<T, String>` 与 `gm-desktop` 风格统一
- `apps/gm-desktop/src-tauri/src/commands/mod.rs` 加 `pub mod graph;`
- `apps/gm-desktop/src-tauri/src/lib.rs`: 加 `mod graph;` + `invoke_handler!` 末尾追加 7 个 graph command
- `apps/gm-desktop/src-tauri/Cargo.toml` 加 `regex = "1"` + `walkdir = "2"`
- `apps/gm-desktop/src-tauri/Cargo.lock` 由 cargo 自动更新
- `docs/adr/0023-v0-desktop-consolidation.md` (本决策文档)

### 6.2 PR-C (后续,独立 ADR-PR)

前端 `/graph` 路由 + tests + i18n + README — 独立 PR,依赖 gm-desktop 上游 svelte-check 错误修复:

- `apps/gm-desktop/src/lib/api/graph.ts`(新)
- `apps/gm-desktop/src/lib/graph/parser.ts`(新)
- `apps/gm-desktop/src/lib/stores/graph.ts`(新)
- `apps/gm-desktop/src/routes/Graph.svelte`(新,3 栏布局)
- `apps/gm-desktop/src/App.svelte` 注册 `/graph` 路由
- `apps/gm-desktop/src/lib/components/Sidebar.svelte` 加 Graph 导航项
- `apps/gm-desktop/src/lib/i18n/{zh-CN,en}.ts` 加 `nav.graph` + `graph.*` 文案
- `apps/gm-desktop/tests/unit/graph-parser.test.ts`(新,把 `apps/desktop/scripts/smoke.mjs` 改 vitest)
- `apps/gm-desktop/README.md` 加 `/graph` 章节

### 6.3 PR-D (后续 cleanup)

- `apps/desktop` 单 commit `git rm -r` + 根 `/.gitignore` 清理

### 6.4 Out of scope(任何 PR)

- gm-desktop 既有 7+ build error 修复(见 §5 — **已 PR-A #14 合** 549bd0c,不再属本 ADR 范畴)
- gm-desktop 上游 svelte-check 106 错误 — 是 Svelte 5 + svelte-spa-router 兼容性预存在债,与本 ADR 同源但需独立 PR 修(PR-X)
- `apps/desktop` 的 `scripts/smoke.mjs` 保留在 git 历史即可,不再 forward-port(改写为 vitest 优先,见 PR-C)
- 跨平台签名 / 自动更新 V0 占位升级(沿用 ADR-0020 §2.7)
- `apps/gm-desktop/src-tauri/icons/` 真实图标替换(非本 ADR 任务)
- 关闭本 issue ULYS-129 — D-Boy 拍板后由 `done` 流程关闭

## 7. 验收 / Acceptance — 分 PR 度量

### 7.1 PR-B (本) 验收

| 项 | 度量 |
|---|---|
| `apps/gm-desktop/src-tauri/` `cargo check --offline` | 0 error (12 个 warning 是 unused_imports 预存在债,等同 PR-A 状态) |
| `apps/gm-desktop/src-tauri/` `cargo test --offline --lib` | 至少 4 个 graph 测试通过 (`graph::tests::kind_of_maps_prefix_to_kind` 等) |
| `git diff --stat` | 仅触 `apps/gm-desktop/src-tauri/` + `docs/adr/0023-*` + `apps/gm-desktop/src-tauri/Cargo.lock`,无其他 |
| `cargo check` (根 gitgit crate) | 通过,根 crate 未受影响 |
| `git ls-remote origin dev` | 含本 PR commit + 引用 PR-A #14 的 squash `549bd0c` |

### 7.2 PR-C (后续) 验收

| 项 | 度量 |
|---|---|
| `apps/gm-desktop/` `pnpm test` (vitest) | 至少 5 个用例通过 (现有 4 + 新 `graph-parser`) |
| `apps/gm-desktop/` `pnpm build` (svelte-check + vite build) | 0 error(注:上游 106 个预存在 svelte-check 错误需 PR-X 同步修) |
| `vitest run tests/unit/graph-parser.test.ts` | 输出 `≥430 requirements / ≥135 nodes / ≥435 edges` (原 desktop smoke 数) |

### 7.3 PR-D (后续) 验收

| 项 | 度量 |
|---|---|
| `apps/desktop/` 在 dev 上不存在 | `git ls-tree HEAD apps/desktop` 0 输出 |
| `apps/desktop` 在 git 历史可回放 | `git show b0ed0e6:apps/desktop/README.md` 有内容 |

## 8. 时间线 / Timeline

- 2026-09-19 `gm-desktop` v0.1 骨架 commit `2f8b9fc` 上 dev(ADR-0020)— 但 build error 已潜伏
- 2026-09-25 `apps/desktop` v0.1.0 经 PR #11 merge `b0ed0e6` 到 dev
- 2026-09-26 02:35 Ulysses 拍板 "选择更优秀的,并吸收另一分支优点"
- 2026-09-26 03:19 Ulysses "确认" ADR-0023 三项(主干选 `gm-desktop` / 单 commit `git rm -r apps/desktop` / 现在写 ADR-0023)
- 2026-09-26 ~03:30 本 ADR 起 + 开工 `feature/consolidate-desktop` worktree
- 2026-09-26 ~04:00 §5 预存在 build error 实测发现,拆 PR 决策点
- 2026-09-26 ~13:49 **Ulysses 拍板 "A"** (路径 A 选): PR-A `fix/desktop/gm-desktop-build-errors` 优先
- 2026-09-26 ~13:49 PR-A 合 #14 `549bd0c` (single squash): 7 个 cargo error 清零
- 2026-09-26 ~13:50 PR-B 开工: `feature/consolidate-desktop` 基于 `origin/dev 549bd0c`,吸收 graph.rs + 7 commands + ADR-0023 文档
- 2026-09-26 ~14:00 PR-B 范围收敛: 仅 Rust 端 (避免前端膨胀到 svelte-check 预存在 106 错误一并背锅),前端 `/graph` 路由拆 PR-C,`apps/desktop` 删除拆 PR-D
