# CI 接入指南 (V0.1)

本仓 GitHub Actions CI 由 `feature/gm-console-v0.1` merge 进 dev 后
落地。**目前是三份 workflow**（`[FACT]` 2026-10-05 核验 `.github/workflows/` 目录）:

- **`gm-console.yml`** — 网页版 CI (typecheck / lint / test / build)
- **`rust-backend.yml`** — Rust 后端 CI (fmt / clippy / test / release build / audit)
- **`gm-desktop.yml`** — 桌面端 CI (`build` = lint / svelte-check / test / build / cargo check;
  `msi` = Windows 上真实 release 打包并校验产物)

> 下方「当前状态」一节记录的是 **2026-10-02 时点**的状态，当时只有前两份 workflow，
> 且两者各 4 次 run 全红。该历史记录保留原样，未随本文修订而改写；
> 其后的实测结论见「修复后状态」及更晚 commit 的报告。

> `[FACT]` 注意用词：**workflow 文件"落地"了，但 CI 并没有"就位"。**
> 落地的是 YAML 定义；实际门禁一次都没跑通（8 次 run 全红）。
> 详见下方「当前状态」。

## 当前状态（2026-10-02 核验）

> `[FACT]` **自引入以来，两个 workflow 每一次跑都是红的。**
> 仓内可查的 GitHub Actions run 共 **8 次，8 次全红**，时间跨度 **2026-09-20 → 2026-09-26**
> （`gm-console` 4 次 / `rust-backend` 4 次），**无一条 green 记录**。

| workflow | run 数 | 结论 | 时间范围 | 首个失败 step | 该 step 之后被 skip 的实质 gate |
| --- | --- | --- | --- | --- | --- |
| `rust-backend` | 4 | 4 红 | 2026-09-20 → 2026-09-25 | step 5 `Check formatting` | 6 Clippy / 7 unit tests / 8 doc tests / 9 release build |
| `gm-console` | 4 | 4 红 | 2026-09-20 → 2026-09-26 | step 3 `Set up Node.js` | 4 Install pnpm / 5 deps / 6 typecheck / 7 lint / 8 unit tests / 9 coverage gate / 10 format check / 11 build |

**关键含义**：两个 workflow 都在 **setup 阶段**就红，其后的实质 gate **一次都没有执行过**。
所以以下内容**目前都不是 CI 验证过的**：

- `[FACT]` Rust 侧 —— clippy、unit tests（`cargo test --bins`）、doc tests、release build
- `[FACT]` 前端侧 —— typecheck、lint、unit tests、coverage gate、format check、build

> 换言之：**本仓当前没有任何一道实质 CI 门禁真正跑通过。**
> 任何"CI 绿了所以代码 OK"的推论在现状下都不成立。

### 失败根因（已核验）

**根因 1 —— `rust-backend.yml`：`rustfmt` 这道门从未在代码库上跑过**

- `[FACT]` step 5 `cargo fmt --all -- --check` 报出**仓库级 diff**，直接判红。
- `[INFERENCE]` 这是该 gate 首次真正作用于代码库，代码库从未按 `rustfmt` 全量格式化过，
  因此第一次跑必然红 —— 属于**首次执行暴露的存量问题**，不是回归。
- 连带后果：step 6–9 全部 `skipped`。

**根因 2 —— `gm-console.yml`：`cache: 'pnpm'` 声明在 pnpm 安装之前，且 lock 文件路径不存在**

- `[FACT]` step 3 `Set up Node.js` 报 `Unable to locate executable file: pnpm`。
- `[FACT]` step 3 的 `actions/setup-node@v4` 带了 `cache: 'pnpm'`，它需要能调用 `pnpm`
  才能计算缓存目录；但 pnpm 是在 **step 4 `pnpm/action-setup@v4`** 才安装的 —— **顺序反了**。
- `[FACT]` step 3 还指定了 `cache-dependency-path: apps/gm-console/pnpm-lock.yaml`，
  而**该文件不存在**。`git ls-files` 核验：仓内被 track 的唯一 lock 文件是
  `apps/gm-desktop/pnpm-lock.yaml`，`apps/gm-console/` 下**没有任何** lock 文件。
- 连带后果：step 4–11 全部 `skipped`。

### 修复后状态：`[FACT]` 已转绿（2026-10-02 实测）

> `[FACT]` 上方两个根因均已修复并合入 `dev`，**两个 workflow 在 `dev` 上首次全绿**。

| workflow | run ID | 结论 | 实质 gate 执行情况 |
| --- | --- | --- | --- |
| `rust-backend` | `36958860879` | **success** | 5 fmt / 6 clippy `-D warnings` / 7 unit tests / 8 doc tests / 9 release build —— **全部 success** |
| `gm-console` | `36958860895` | **success** | 6 typecheck / 7 lint / 8 unit tests / 9 **coverage gate** / 10 format check / 11 build —— **全部 success** |

两次 run 均基于 `dev` @ `a34d812`。

**这是本仓 CI 历史上的第一次全绿**（此前 8/8 全红）。上表列出的每一个「该 step 之后
被 skip 的实质 gate」，**现在都已经在 CI 上真实执行并通过**。历史遗留的未验证状态到此结束。

修复内容摘要（详见各 commit）：

- `[FACT]` `rust-backend`：全仓 `cargo fmt`；修复 8 个 clippy error（6× `result_large_err`、
  1× `map_identity`、1× `unused_variables`）；新增 `rust-toolchain.toml` 钉到 1.98.1，
  消除 `stable` 漂移导致绿门禁再次变红的隐患。
- `[FACT]` 修复一个真实缺陷：`src/server/http.rs` 用 `.merge(api)` 把 REST 路由挂在
  **根路径**，与 Git 协议的 `/repos/*key` 兜底路由冲突，axum 0.7 在构造期直接 panic，
  导致 23 个测试失败。改为 `.nest("/api", api)` —— 这与 `scripts/regression-it.ps1`
  （47 条断言打 `/api/*`）、`scripts/smoke.ps1`（Git 协议走根路径）以及
  `apps/gm-console/src/api/client.ts`（`baseURL: '/api'`）三方既有契约一致。
- `[FACT]` `gm-console`：补齐并提交 `apps/gm-console/pnpm-lock.yaml`（此前不存在，
  `--frozen-lockfile` 无法满足）；把 `pnpm/action-setup` 移到 `setup-node` **之前**；
  CI Node 版本 20 → 22（Node 20 "Iron" 已于 2026-04-30 EOL，
  且 `@testing-library/jest-dom@6.x` 声明 `engines.node: ">=22"`）。
- `[FACT]` 覆盖率门禁：`vite.config.ts` 的 70% 阈值**未被下调**。补测试把实测覆盖率
  从 **6.22% / functions 29.16% / branches 54.87%** 提升到
  **73.11% / 85.04% / 89.12%**，测试数 24 → 173，阈值原样通过。

### 第三条 workflow：`gm-desktop` 已纳入 CI（2026-10-02）

> `[FACT]` `apps/gm-desktop/`（Tauri 2 + Svelte 5 桌面端）此前**不在任何 workflow 的
> `paths` 触发范围内** —— 桌面端改坏了 CI 不会响。该缺口正是上一节那个生产构建
> 缺陷被长期掩盖的原因。

**`.github/workflows/gm-desktop.yml` 首次运行即全绿**：run `36975561146` → `success`
（基于 `dev` @ `f784ffd`），13 个 step 全部 `success`：

| step | 结果 | 覆盖内容 |
| --- | --- | --- |
| 3–5 | success | pnpm 先于 `setup-node`（Node 22）、`--frozen-lockfile` |
| 6 Lint | success | ESLint，此前因 devDependencies 缺 eslint 从未执行过 |
| 7 Svelte check | success | `svelte-check` 0 errors |
| 8 Unit tests | success | vitest 63 passed |
| 9 Build | success | Vite 生产构建，产出 dist（此前**无法构建**） |
| 10–11 | success | Rust 1.98.1 toolchain + Tauri 2 的 Linux 前置依赖（WebKitGTK/GTK） |
| 12 Cargo check | success | `src-tauri` 的 Rust 侧编译，此前完全无任何 workflow 覆盖 |

`[FACT]` 门禁范围刻意保持**轻量**：不做完整 `tauri build` / `.msi` / `tauri-action`，
那需要完整打包工具链与多 OS matrix。见下方「已知限制」第 2 条。

## 触发器定义

| workflow | 触发器 | 首次跑时机 |
| --- | --- | --- |
| `gm-console` | push to `dev` / `main` / `feature/gm-console-v0.1` 触及 `apps/gm-console/**` 或 `src/server/api.rs` | Ulysses 第一次 push dev 到 origin |
| `rust-backend` | push to `dev` / `main` / `feature/gm-console-v0.1` / `feature/gm-desktop-v0.1` 触及 `src/**` 或 `Cargo.toml` | 同上 |

两个 workflow 都用 `ubuntu-latest` 跑。`concurrency` + `cancel-in-progress`
防多 push 撞车。

`[FACT]` 两个 workflow **除了 push，还都配了 `pull_request` 触发器**
（target `dev` / `main`，paths 与上表一致）—— 原文档只列了 push，此处补全。

## Ulysses 接入步骤

### 1. 第一次 push dev 到 origin (Mavis 推)

```powershell
cd D:\GitGit
git checkout dev
git push origin dev
```

这一步触发两个 workflow 同时跑。预期:
- `rust-backend.yml`: ~3 min (含 cargo build cache miss + 75 test)
- `gm-console.yml`: ~5 min (含 pnpm install + vitest + vite build)

> `[TBD]` **注意：上述"预期"目前达不到。** 按「当前状态」一节的实测，两个 workflow
> 至今每次都红在 setup 步骤，实质 gate 一次没跑过。在 Lane A / Lane B 的修复合入
> `dev` 之前，push dev 的预期结果应当按 **red** 对待，而不是按上面两条的成功预期对待。

### 2. (可选) 跑一次手动试运行

GitHub UI: Actions → gm-console → Run workflow → branch: dev。
确认两 workflow 都成功。

> `[TBD]` 同上：手动试运行当前**预期也是红**。此步在 Lane A / Lane B 修复合入后才谈得上确认成功。

### 3. 后续 push 协议 (per 守门 9/8 15:29 自驱)

`feature/gm-console-v0.1` / `feature/gm-desktop-v0.1` 的 push 都会触发
相应 workflow。Mavis 已经写过 verifier + 自审代码, push 前会先在本地
跑 cargo test / pnpm test(若网络允许),不再每次都问 Ulysses。

### 4. 分支保护 (推荐, 接入后做)

GitHub repo → Settings → Branches → Add rule for `dev`:
- Require a pull request before merging
- Require status checks to pass before merging: `gm-console` + `rust-backend`

这是 CI 接入后的"门槛门"。

> `[TBD]` **暂勿启用。** 按「当前状态」，`gm-console` 与 `rust-backend` 目前每次都红；
> 若此时开启 "Require status checks to pass"，会把**所有**对 `dev` 的合并全部堵死
> （两个 check 永远不绿）。待 Lane A / Lane B 修复合入 `dev`、且这两个 check
> 至少各绿过一次之后，再启用本节的 status check 强制项。

## Secrets 配置 (当前不需要)

两个 workflow 都不需要 secret:
- `pnpm install` 走 npmjs.org 公网,无需 token
- `cargo test` 走 crates.io 公网,无需 token
- 不调任何需要 auth 的私有 registry

后续如果加 `apps/gm-desktop/src-tauri/` 的 Tauri 自动签名,会需要:
- `TAURI_SIGNING_PRIVATE_KEY` (base64 编码)
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
- 触发 Tauri updater server 的 GH_PAT

这些等本地 app 二期合并时再加。

## 已知限制 (V0.1 baseline)

1. **minIO e2e 不在 CI gate** — `#[ignore]` 测试需要 minIO server,
   V0.1 用自托管 runner 二期接入
2. **完整 Tauri 打包（`.msi` / `tauri-action` / 多 OS matrix）仍不在 CI** — 部分解除
   （2026-10-02）：`apps/gm-desktop/**` **已加入 paths**（新增 `gm-desktop.yml`），
   桌面端改动现在会触发 CI；`pnpm lint` / `pnpm check` / `pnpm test` / `pnpm build` /
   `src-tauri` 的 `cargo check` 已在 run `36975561146` 全绿。
   **仍未覆盖**的只有真正的跨平台打包：`cargo tauri build` 产 `.msi`/`.dmg`/`.deb`
   及其签名与 updater，需要 `tauri-action` + (macos-latest / windows-latest /
   ubuntu-latest) matrix。二期接入时在现有 `gm-desktop.yml` 上追加一个
   `needs: build` 的独立 job 即可。
3. **不跑 component / e2e** — vitest unit + playwright e2e 推到 V0.2
4. **~~实质 gate 零验证~~ —— 已解除（2026-10-02）** — `[FACT]` 该状态到此结束。
   `rust-backend` 的 fmt / clippy `-D warnings` / unit test / doc test / release build，
   与 `gm-console` 的 typecheck / lint / unit test / coverage gate / format check / build，
   **已在 run `36958860879` / `36958860895` 上全部真实执行并通过**。
   详见「修复后状态」。
5. **~~`gm-desktop` 的 `pnpm build` 无法通过~~ —— 已修复（2026-10-02）** — `[FACT]`
   该构建失败由 `svelte-spa-router@4.0.2` 引起：其 `Router.svelte:255` 导入并使用
   `afterUpdate`，Svelte 5 runes 模式禁止该 API，`vite build` 报
   `afterUpdate cannot be used in runes mode`。
   `[INFERENCE]` 该应用**从未有过可用的生产构建**，只是因为
   `apps/gm-desktop/**` 不在任何 workflow 的 `paths` 内，CI 看不到。
   修复方式：以 `$lib/router.ts`（约 90 行 hash 路由核心）+ `Router.svelte`
   替换该依赖，并移除 `svelte-spa-router`。同时修正了第二处契约不一致 ——
   原库是 hash 路由，而 `NotFound`/`RepoDetail`/`Repos`/`Sidebar` 用的是
   非 hash 锚点；现统一由 `href()` 产出 `#/...`。
   替换一个不可用的库却没有测试不可接受，故新增
   `tests/unit/router.test.ts`（23 个用例），测试数 40 → 63。
   `[FACT]` 完整 `tauri build` / `.msi` 仍**不在 CI 内**（见第 2 条）。

## Mavis 自动续做项 (per守门 #1 + 9/8 15:29 自驱)

- push dev 触发 CI 失败 → Mavis 自动诊断 + 修,不需 Ulysses 决策
- pnpm-lock.yaml 漂移 → Mavis 自驱 `pnpm install` + 锁文件更新
  - `[FACT]` 澄清:仓内被 track 的 pnpm lock 文件目前**只有**
    `apps/gm-desktop/pnpm-lock.yaml`;`apps/gm-console/` 下**没有** lock 文件
    (这正是「当前状态」根因 2 的一半)。修 CI 时需一并决定 gm-console 用哪份锁文件。
- 守门 #1 触发: docs 触达饱和 → Mavis 写新 docs commit, 不需问 Ulysses

## 触发器清单

`gm-console.yml` 的 `paths` 包含:
- `apps/gm-console/**` — 任何前端文件变动
- `src/server/api.rs` — 后端 API 表面(影响前端类型)
- `.github/workflows/gm-console.yml` — CI 文件本身修改

`rust-backend.yml` 的 `paths` 包含:
- `src/**` — 任何 Rust 源码变动
- `Cargo.toml` / `Cargo.lock` — 依赖变更
- `.github/workflows/rust-backend.yml` — CI 文件本身修改

## 调试 CI 失败

1. 看 GitHub Actions tab → run 详情 → 哪个 step 红了
2. 下载对应 artifact (coverage / dist) 复现
3. Mavis 自动修(per 9/8 15:29 自驱): 不需 Ulysses 介入

## 接入决策项 (Ulysses 拍板项)

仅这些要 Ulysses 决策, 其他 Mavis 自驱:

1. **是否启用 branch 保护** — 推荐 yes(接入后第二周做)
   - `[TBD]` 但**前置依赖**已变化:两个 check 当前全红,启用 status check 强制项会堵死所有合并。
     需等 Lane A / Lane B 修复合入 `dev` 且各绿过一次后再启用(见上节「分支保护」)。
2. **是否要 nightly build** — V0.1 不需要, V0.2 加 cron schedule
3. **是否要 RGS-CI-OPS 角色读 GitHub Actions** — 团队权限, 不在技术 CI 范围