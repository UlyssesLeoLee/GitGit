# CI 接入指南 (V0.1)

本仓 GitHub Actions CI 由 `feature/gm-console-v0.1` merge 进 dev 后
就位。两份 workflow 在 `.github/workflows/`:

- **`gm-console.yml`** — 网页版 CI (typecheck / lint / test / build)
- **`rust-backend.yml`** — Rust 后端 CI (fmt / clippy / test / release build)

## 当前状态 (2026-09-19 16:50 JST)

| workflow | 触发器 | 首次跑时机 |
| --- | --- | --- |
| `gm-console` | push to `dev` / `main` / `feature/gm-console-v0.1` 触及 `apps/gm-console/**` 或 `src/server/api.rs` | Ulysses 第一次 push dev 到 origin |
| `rust-backend` | push to `dev` / `main` / `feature/gm-console-v0.1` / `feature/gm-desktop-v0.1` 触及 `src/**` 或 `Cargo.toml` | 同上 |

两个 workflow 都用 `ubuntu-latest` 跑。`concurrency` + `cancel-in-progress`
防多 push 撞车。

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

### 2. (可选) 跑一次手动试运行

GitHub UI: Actions → gm-console → Run workflow → branch: dev。
确认两 workflow 都成功。

### 3. 后续 push 协议 (per 守门 9/8 15:29 自驱)

`feature/gm-console-v0.1` / `feature/gm-desktop-v0.1` 的 push 都会触发
相应 workflow。Mavis 已经写过 verifier + 自审代码, push 前会先在本地
跑 cargo test / pnpm test(若网络允许),不再每次都问 Ulysses。

### 4. 分支保护 (推荐, 接入后做)

GitHub repo → Settings → Branches → Add rule for `dev`:
- Require a pull request before merging
- Require status checks to pass before merging: `gm-console` + `rust-backend`

这是 CI 接入后的"门槛门"。

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
2. **Tauri 跨平台 build 不在 CI** — apps/gm-desktop/src-tauri/ 还没
   merge 进 dev; 二期用 `tauri-action` + matrix (macos-latest /
   windows-latest / ubuntu-latest) 接入
3. **不跑 component / e2e** — vitest unit + playwright e2e 推到 V0.2

## Mavis 自动续做项 (per守门 #1 + 9/8 15:29 自驱)

- push dev 触发 CI 失败 → Mavis 自动诊断 + 修,不需 Ulysses 决策
- pnpm-lock.yaml 漂移 → Mavis 自驱 `pnpm install` + 锁文件更新
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
2. **是否要 nightly build** — V0.1 不需要, V0.2 加 cron schedule
3. **是否要 RGS-CI-OPS 角色读 GitHub Actions** — 团队权限, 不在技术 CI 范围