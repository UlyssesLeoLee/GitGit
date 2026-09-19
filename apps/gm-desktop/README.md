# gitgit Desktop (V0.1, per ADR-0020)

`gm-desktop` 是 `gitgit` MVP 的本地桌面壳。基于 [Tauri 2.x](https://tauri.app/) + [Svelte 5](https://svelte.dev/) 实现,跨 Windows / macOS / Linux 三大平台。

## 背景 / Why this exists

`gitgit serve` 已经能跑;但 CLI 不足以体现"产品化"。V0.1 桌面壳替代并超越 CLI 的关键场景:

- 一键启停本地服务(可视化 PID / 端口 / 日志流)
- 仓库列表 + 详情(refs / branch graph / 最近 commits / 克隆 URL)
- Credential Vault 可视化管理(版本 / diff / restore / rotate / delete)
- 系统托盘 + 关闭即隐藏(常驻后台)
- 错误边界 + 多语言 + 暗色模式
- 自动更新骨架(V0 占位,V1 真正推送)

完整 ADR 见 [`docs/adr/0020-v0-gui-tauri-svelte.md`](../../docs/adr/0020-v0-gui-tauri-svelte.md)。

## 技术栈 / Tech stack

| 层 | 技术 |
|---|---|
| 桌面壳 | Tauri 2.x (`tauri = "2"`) |
| 前端 | Svelte 5 + TypeScript (strict) + Vite 5 |
| 样式 | Tailwind CSS 3 |
| 路由 | `svelte-spa-router` |
| 表单 | `felte` + `zod` |
| 测试 | `vitest` + `@testing-library/svelte` |
| 后端 (Tauri Rust 侧) | `tokio` + `reqwest` + 复用 `gitgit` crate 内部模块 |

## 工作目录 / Layout

```
apps/gm-desktop/
├── README.md                 # 本文
├── package.json              # 前端依赖
├── vite.config.ts            # Vite + Vitest 配置
├── tsconfig.json             # TS strict
├── tailwind.config.js
├── postcss.config.js
├── svelte.config.js
├── index.html
├── src/                      # Svelte 前端
│   ├── main.ts
│   ├── App.svelte
│   ├── routes/               # 路由组件
│   ├── lib/
│   │   ├── api/              # Tauri command 包装
│   │   ├── components/       # 通用组件
│   │   ├── i18n/             # zh-CN + en
│   │   ├── stores/           # 状态管理
│   │   └── utils/
│   └── mocks/                # 开发态 mock handlers
├── src-tauri/                # Tauri Rust 侧
│   ├── Cargo.toml
│   ├── build.rs
│   ├── tauri.conf.json
│   ├── capabilities/
│   ├── icons/                # 占位,正式 build 前需要真实图标
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── state.rs
│       ├── error.rs
│       └── commands/
├── tests/
│   └── unit/                 # vitest 单元测试
├── scripts/
└── .github-workflow-snippet.md   # Ulysses 接入 CI 用
```

## 本地开发 / Local development

环境要求:

- **Rust** ≥ 1.75(`rustup default stable`)
- **Node** ≥ 22 + **pnpm** ≥ 10
- **Tauri prerequisites**(各平台不一样,见 https://tauri.app/start/prerequisites/)
- **WebView2**(Windows 11 默认带)

### 跑起来

```bash
# 1. 装前端依赖
cd apps/gm-desktop
pnpm install

# 2. 启动桌面开发模式(会同时拉起 Svelte HMR + Tauri shell)
pnpm tauri:dev
```

打开窗口后默认进入 `127.0.0.1:38080` 的服务管理界面。点 "启动服务" 即会启动内嵌的 `axum` 路由(per ADR-0020 §2.2)。

### 纯前端调试(无 Tauri shell)

```bash
pnpm dev        # vite dev server on :5173
```

这种情况下 `src/mocks/handlers.ts` 会接管所有 `invoke()` 调用,提供模拟数据。
**实际生产代码仍走 Tauri 调用**,mock 只为方便 UI 调试。

## 测试 / Tests

```bash
# 单元测试 (vitest)
pnpm test

# 带覆盖率 (>= 70% lines per brief)
pnpm test:coverage

# TypeScript 类型检查
pnpm check

# ESLint
pnpm lint

# Rust 侧测试
cd src-tauri && cargo test
```

## 跨平台构建 / Cross-platform build

### macOS(本机 + 通用)

```bash
pnpm tauri:build --target aarch64-apple-darwin
pnpm tauri:build --target x86_64-apple-darwin
pnpm tauri:build --target universal-apple-darwin
```

产物在 `src-tauri/target/<triple>/release/bundle/`:
- `dmg/gitgit Desktop_0.1.0_*.dmg`

> **签名要求**:Apple Developer ID + `APPLE_CERTIFICATE` 等环境变量,见 Tauri 文档。本机首次构建可以**跳过签名**(`--no-codesign`);CI 必须签名。

### Windows

```bash
pnpm tauri:build
```

产物:
- `msi/gitgit Desktop_0.1.0_x64_en-US.msi`

> **代码签名占位**:本期不强制(开发模式可关 `bundle.windows.signCommand`),但生产**必须**配置 EV 证书。V1 再补。

### Linux

```bash
pnpm tauri:build --target x86_64-unknown-linux-gnu
```

产物:
- `appimage/gitgit-desktop_0.1.0_amd64.AppImage`
- `deb/gitgit-desktop_0.1.0_amd64.deb`

> **deb / AppImage 签名**:V0 占位,V1 + GPG key 接入。

CI 接入详见 [.github-workflow-snippet.md](.github-workflow-snippet.md)。

## 自动更新 / Auto-update

骨架已集成 `tauri-plugin-updater`,但 V0.1 默认关闭(per worker brief "本期不真正推送更新,但代码骨架完整")。`tauri.conf.json` 中 `plugins.updater.active = false`。

V1 启用:

1. 在 CI 把 `tauri build` 产物上传到公开 endpoint(如 GitHub Releases)
2. 在 `tauri.conf.json` 里把 `endpoints` 指向真地址
3. 把 `updater.active` 设回 `true`
4. 在 capabilities/default.json 增 `updater:default` 权限

## 已知缺口 / Known gaps (V0.1 诚实清单)

> per 守门 #1 缺标比错标安全,显式列出:

- **前端覆盖率未跑实际阈值验证** —— `--coverage` 阈值在 `vite.config.ts` 里写死,但首跑覆盖率快照未必立刻满足 70%。建议 V1 起 CI 直接 fail-fast 在低于 70%。
- **`tauri build` 未在 PR-CI 跑过** —— 创建完整 `.msi` / `.dmg` 需下载 WebView2 SDK + WiX toolset,本会话不打包。Ulysses 接入 CI 时务必先在 runner 上 dry-run。
- **`icons/` 当前是占位** —— 见 `src-tauri/icons/` 注释,正式打包前需要补真实图标 (32x32.png, 128x128.png, icon.png, icon.ico, icon.icns)。
- **Tauri WebDriver + Playwright e2e**:骨架未配置(本会话时间约束)。`tests/` 仅有 vitest 单元测试,e2e 旅程留到 V1 真接入工作(详见 [.github-workflow-snippet.md](.github-workflow-snippet.md))。
- **跨平台签名占位**:`tauri.conf.json.bundle.windows.wix` / `.macOS` / `.linux.deb` 都按占位配置,**正式发布前必须**配置 EV / Developer ID / GPG 证书。
- **`/api/*` REST 调用尚未合入**:worker-A 的 gm-console 网页版会注入 REST 端点;`src/lib/api/http.ts` 已就位但 V0.1 还没调用。worker-A 合入后再走 fetch。
- **bundle size < 30MB 目标**:未测量。开发占位下不应超过,但需要在 CI 加一个上传 artifact 后 `wc -c` 检查的步骤。
- **i18n catalog 未翻译完** —— en/zh-CN 等价 key,但部分简写如「输入 key + value required」是英文 hardcode,留到 V1 全量翻译。

## 修订历史

| 修订 | 作者 | 审批 | 修订人 | 日期 | 备注 |
|---|---|---|---|---|---|
| v0.1.0 | Ulysses | 架构师(Mavis 接手 agent per DEC-008)+自审 | Ulysses | 2026-09-19 | 初版,本地 app 骨架 + 5 域 Lead 临时签字 (per 9/8 第 6 次强化) |
