# CI 接入指南 (V0.1)

本仓 GitHub Actions CI 由 `feature/gm-console-v0.1` merge 进 dev 后
落地。两份 workflow 在 `.github/workflows/`:

- **`gm-console.yml`** — 网页版 CI (typecheck / lint / test / build)
- **`rust-backend.yml`** — Rust 后端 CI (fmt / clippy / test / release build)

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

### 修复后状态：`[PENDING Lane A / Lane B]`

> `[TBD]` **本文档不声称 CI 已修复、已变绿或已通过。**
> 上述两个 workflow 由 **Lane A（`rust-backend.yml`）** 与 **Lane B（`gm-console.yml`）**
> 在各自 worktree 中并行修复中，修复结果**尚未合入 `dev`**，其结果本 Lane 无法预知。
>
> **待两条 lane 合入 `dev` 后，由 parent 依据 `dev` 上真实的 run 记录回填本节**：
> 用修复后实测的 run 数、红绿结论、首个失败 step 替换上表，并把本节的 `[PENDING]`
> 与「8 次全红」一并更新。在那之前，实质 gate 一律按**未验证**对待。

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
2. **Tauri 跨平台 build 不在 CI** — 更正（2026-10-02）：`apps/gm-desktop/src-tauri/`
   **已经在 `dev` 上**（`[FACT]` commit `2f8b9fc`；ADR-0023 §3.3 中 `03bdd7b`
   `git rm -r apps/desktop` 删掉的是**另一个**目录）。它不进 CI 的真实原因**不是**
   "还没 merge 进 dev"，而是**两个 workflow 的 `paths` 过滤都没有覆盖
   `apps/gm-desktop/**`** —— `[FACT]` `git grep gm-desktop .github/workflows/`
   只命中 `rust-backend.yml` 的 `branches:` 列表，paths 里没有该目录，
   即该目录的改动**根本不会触发**任何 workflow。
   二期用 `tauri-action` + matrix (macos-latest / windows-latest / ubuntu-latest)
   接入时，需同时把 `apps/gm-desktop/**` 加进 paths。
3. **不跑 component / e2e** — vitest unit + playwright e2e 推到 V0.2
4. **实质 gate 零验证** — 见「当前状态」：`rust-backend` 的 clippy / unit test /
   doc test / release build，与 `gm-console` 的 typecheck / lint / unit test /
   coverage / format check / build，**从未在 CI 上执行过一次**。
   本节其余限制是叠加在这一条之上的，**不能**理解为"除这些外其余 gate 都在正常把关"。

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