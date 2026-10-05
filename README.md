# GitGit

本地优先的 Git HTTP 服务器 + 版本化凭据保险库 + Tauri 桌面端。

> **状态：可运行的 MVP，尚未达到可发布状态。**
>
> 本仓库有真实实现、真实测试和真实 CI 门禁，并且能在 Windows 上产出可安装的 MSI。
> 但仍有 **两个 P0 阻断项未解决**（许可证声明冲突、HTTP API 无鉴权且默认监听所有网卡），
> 详见 [已知未决问题](#已知未决问题)。在这些关闭之前，本项目**不应**被当作可商售产品分发。
> 另有一条 P1：Windows 安装包未做代码签名。

本文件只陈述**实测**状态。凡本文出现数字，均为在指定 commit 上跑出来的，不是估计值。
未能验证的一律写在「已知未决问题」里，不写在正文里。

## 仓库结构

| 路径 | 是什么 |
| --- | --- |
| `src/` | 根 Rust crate —— CLI、Git HTTP server、版本化凭据保险库、AI provider |
| `apps/gm-desktop/` | Tauri 2 + Svelte 5 桌面端 |
| `apps/gm-console/` | React + Vite 网页管理端 |
| `deploy/minio/` | 本地 minIO 夹具，供保险库 e2e 测试使用 |
| `scripts/` | 回归测试与验证脚本（PowerShell） |
| `.github/workflows/` | 三个 CI workflow |
| `docs/adr/` | 现行架构决策记录（ADR-0001 / 0020–0023） |
| `docs/plan/v0-tasks.md` | V0 任务台账，逐项带证据标注的实测状态 |
| `docs/reports/` | 各批次实现与验证报告 |
| `docs_archive_rust_impl_2026_08_26/` | **已归档**：早期 14-crate 架构与 15 阶段需求定义书，见文末 |

根 crate 是**单一 crate，不是 workspace**（`Cargo.toml` 中显式注明）。
`apps/gm-desktop/src-tauri/` 是独立 crate，通过 `path = "../../.."` 依赖根 crate，
因此它有自己独立的 `Cargo.lock`。

## 快速开始

### 前置

- Rust **1.98.1** —— `rust-toolchain.toml` 已钉版，不要用 `stable`
- Node.js **22** + pnpm **9**
- `git` 可执行文件在 `PATH` 中（本项目以 shell 调用 `git` 子进程，不使用 libgit2/gix）

### 根 crate：编译与测试

```bash
cargo build
cargo test                       # 单元 + 集成测试
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

`Cargo.toml` 开启了 `unsafe_code = "forbid"` 与 `clippy::unwrap_used`/`expect_used`/`panic` 的 `deny`。

### 根 crate：跑起来

```bash
gitgit init <name>               # 在 ./repos/ 下建一个裸仓库
gitgit list                      # 列出仓库
gitgit serve                     # 启动 HTTP server
gitgit key set openai <secret>   # 写入凭据保险库
gitgit key ls
gitgit gitai review              # AI 代码评审
gitgit gitremote add <name> <url>
```

顶层参数：`--bind`（默认 `127.0.0.1:8080`，仅本机；需要局域网访问时显式传 `--bind 0.0.0.0:8080`）、`--repos-dir`、`--vault-file-root`。

### HTTP API

`/api` 子路由由 `src/server/api.rs` 的 `build_api_router()` 构造：

| 方法 | 路径 |
| --- | --- |
| GET | `/api/health` |
| GET | `/api/repos` · `/api/repos/:name` · `/api/repos/:name/refs` · `/api/repos/:name/log` |
| GET | `/api/vault/keys` |
| GET / DELETE | `/api/vault/keys/:key` |
| GET / POST | `/api/vault/keys/:key/versions` |
| GET | `/api/vault/keys/:key/diff` |
| POST | `/api/vault/keys/:key/restore` |

Git 自身的 smart-HTTP 路由由 `src/server/http.rs` 的 `build_router()` 提供。
**注意：这些 `/api` 路由目前没有任何鉴权层**，详见已知未决问题。

### 桌面端

```bash
cd apps/gm-desktop
pnpm install
pnpm tauri:dev                   # 真实 Tauri 窗口
pnpm test                        # vitest
pnpm check                       # svelte-check
```

### 网页管理端

```bash
cd apps/gm-console
pnpm install
pnpm dev
pnpm test
```

## 质量门禁

三个 CI workflow，全部以 `dev` 为触发分支：

| workflow | 跑什么 |
| --- | --- |
| `rust-backend.yml` | fmt、clippy `-D warnings`、单元测试、doc 测试、release build，以及**独立的 `audit` job** |
| `gm-desktop.yml` | lint、svelte-check、vitest、Vite 生产构建、`src-tauri` 的 fmt/clippy/test；`msi` job 在 Windows 上真实打包并校验产物 |
| `gm-console.yml` | typecheck、lint、vitest、coverage 门禁、format check、生产构建 |

`audit` 做成独立 job 而不是 `test` 里的一个 step：CVE 应该在秒级失败，而不是等完整个 release 构建。
该 job 同时区分「扫描到公告」与「公告库不可达」这两种不同的失败。

### 实测快照（`6d24b21`）

| 项 | 数值 | 怎么来的 |
| --- | --- | --- |
| 根 crate 测试 | **186 passed / 0 failed / 2 ignored** | CI 实测于 `f690486`；`f690486 → 6d24b21` 之间 `src/`、`Cargo.toml`、`Cargo.lock` 零改动，故结论延续，并已本地重跑复核 |
| 根 crate fmt / clippy | 0 / 0 | 本地重跑复核 |
| RUSTSEC 公告 | 0（扫描 240 个依赖） | CI `audit` job |
| `apps/gm-desktop` 测试 | **378 passed / 25 files / 0 skipped** | 本地实测，CI 复核 |
| `apps/gm-desktop` 覆盖率 | lines 94.78% / branches 78.43% / functions 95.17% / statements 94.23% | 本地实测，`pnpm test:coverage` **exit 0** |
| `apps/gm-console` 测试 | **194 passed / 15 files** | 本地实测 |
| `apps/gm-console` 覆盖率 | **lines 93.63%** | 本地实测，门槛 lines 70 / branches 60 |
| i18n 键一致性 | `en` 184 / `zh-CN` 184，键集完全一致 | 本地实测 |

`apps/gm-desktop` 的覆盖率阈值（lines/statements 70、functions 60、branches 55）声明在
`vite.config.ts`，CI 的 `Coverage gate` step 现在**真的会求值它们**。
这一度曾经只是"看起来像门禁"：CI 跑的是 `pnpm test`（不带 `--coverage`），
而当时 `test:coverage` 是 exit 非零的。2026-10-05 补齐路由组件与 stores/api 层的测试后，
`All files` 从 42.55% 提到 94.78%，门禁才真正成立。阈值未作任何下调。

MSI 产物（CI 实测于 `d81f285`）：perUser 15,194,299 B / perMachine 15,193,981 B /
manifest 640 B / dist 271,164 B，共四个 artifact 并存。

## 已知未决问题

以下条目**没有**被修复。列在这里是为了让任何评估者看到完整图景，而不是只看到绿 CI。

### P0 — 许可证声明自相冲突

顶层 `LICENSE` 是 **GNU AGPL-3.0 全文**（35,184 字节，首两行为
`GNU AFFERO GENERAL PUBLIC LICENSE Version 3`），而 `Cargo.toml` 与
`apps/gm-desktop/src-tauri/Cargo.toml` 都声明 `license = "Apache-2.0"`。
两者不可能同时为真，本文件因此**不**声明本项目采用何种许可证。
在冲突解决之前，不应分发本项目。

### P0 — HTTP API 无鉴权（默认监听所有网卡的部分已修复）

**已修复（默认绑定）**：`src/config.rs` 的 `DEFAULT_BIND` 原为 `0.0.0.0:8080`，
即默认把服务暴露到本机所在的所有网卡。现已改为 `127.0.0.1:8080`，仅本机可达。
需要局域网访问时显式 `--bind 0.0.0.0:8080`。这与桌面端一直使用的
`127.0.0.1:38080`（`src-tauri/src/state.rs`）以及仓内架构文档自述的
「默认仅本机」原则一致。`src/cli.rs` 新增 3 条测试钉住该默认值，
其中一条专门断言显式通配地址仍然可用——收紧默认不等于移除能力。

**仍未修复（无鉴权）**：完整链路已逐行核实：

- `src/server/api.rs` 中的 `auth_optional` 是 no-op，**从未被调用**，且挂着 `#[allow(dead_code)]`
- `build_api_router()` 与 `build_router()` 都不加任何鉴权层
- `GET /api/vault/keys/:key` 会把凭据明文放进 JSON 响应；`DELETE` 与 `POST .../restore` 同样无鉴权
- 全仓唯一的鉴权 `require_basic` 只在 git push 路径上，且凭据硬编码为 `admin` / `admin`
  （`src/config.rs`）
- 桌面侧的 `bind` 由前端传入且零校验（`state.rs` → `commands/server.rs`）

把默认绑定收回本机，收窄了**默认**暴露面，但只要显式传 `--bind 0.0.0.0:8080`，
`/api` 依旧是零鉴权且明文返回凭据。剩余部分需要 `/api` 强制鉴权 + 凭据外置，
会改变现有本地无凭据流程，属于需要人类拍板的产品决策，尚未实施。

细节见 [`SECURITY.md`](SECURITY.md)。

### P1 — Windows 安装包未做代码签名

4 个 MSI 变体全部未签名，触发 SmartScreen 告警。需要购买代码签名证书，不在技术范围内。

### P1 — 覆盖率门禁是声明而非门禁

`apps/gm-desktop` 声明的覆盖率阈值没有任何 CI job 求值。详见上文「质量门禁」。

### P1 — AI 链路未对真实 provider 端到端验证

`src/ai/` 的 provider 实现有完整单元测试（mock SSE server 实测 token 序列
`["Hel","lo","wo","rld","!"]`），但仓内没有 `GITGIT_AI_API_KEY`，
**从未真正调用过 OpenAI 或 Anthropic**。`TauriEmitter` 的事件是否真正抵达 webview 同样未验证。

### P2 — 跨平台产物已在 CI 构建，但仍未签名、未在真机安装验证

`tauri.conf.json` 声明了 `["msi","dmg","appimage","deb"]` 四种 target，
此前只有 MSI 被构建过。现已新增 `.github/workflows/gm-desktop-bundle.yml`，
在对应 runner 上产出并校验：

| 产物 | runner | 校验方式 |
|---|---|---|
| `.dmg` | `macos-latest` | `hdiutil imageinfo` + 只读挂载，断言 app 二进制 ≥ 1 MiB |
| `.deb` | `ubuntu-latest` | `dpkg-deb --info` / `--contents`，版本必须与 `tauri.conf.json` 一致 |
| `.appimage` | `ubuntu-latest` | ELF magic + `file` + 5 MiB 下限 |

`[FACT]` 三个 job 首次运行即全部通过。但**均未签名**，也**未在任何真实机器上
安装运行过**：CI 只验证产物结构，不验证「装得上、跑得起来」。macOS 未做
notarization，Windows 四个 MSI 变体同样未签名（见上一节）。

`[UNVERIFIED-FACT]` `.deb` 依赖 `bundle.linux.deb.depends` 为空数组，
bundler 因此不写 `Depends:`，真实安装时不会拉取 webkit2gtk。校验脚本对此
只打印 NOTE 而不失败，因为它属于本仓不拥有的配置决策。

### P2 — 工作区根目录选择器未在真实窗口验证

对话框在 `invoke` 边界被 stub 掉，真实 Tauri 窗口下的行为未验证。

## 文档索引

| 目录 | 内容 |
| --- | --- |
| [`docs/adr/`](docs/adr/) | 现行架构决策记录 |
| [`docs/plan/v0-tasks.md`](docs/plan/v0-tasks.md) | V0 任务分解 T1–T11，带证据标注的实测状态 |
| [`docs/reports/`](docs/reports/) | 各批次实现报告与回归测试记录 |
| [`.github/CI.md`](.github/CI.md) | 三个 CI workflow 的实测状态与历史失败根因 |
| [`CONTRIBUTING.md`](CONTRIBUTING.md) | 贡献指南 |
| [`SECURITY.md`](SECURITY.md) | 安全策略与未修复的已知问题 |

## 事实标注约定

本仓库对论断强制标注，实测与推断必须分开：

| 标记 | 含义 |
| --- | --- |
| `[FACT]` | 有一手来源或实测支撑，并标注来源 |
| `[UNVERIFIED-FACT]` | 有来源但未经一手验证，不可当作确证事实 |
| `[INFERENCE]` | 基于已知事实的合理推论，未独立核实 |
| `[PROPOSAL]` | 本项目自己的设计主张，不归因于任何第三方 |
| `[TBD]` | 现阶段无法确认 |

把推断写成事实是被明确禁止的。本仓库已经因为「写入仓库的根因分析是推断而非实测」
而误导过后来者，所以这条约定不是形式主义。

---

## 附录：已归档的研究档案

以下内容是**真实的历史工作产物**，但描述的是早期 14-crate 架构，与当前实现**已经不一致**。
保留它们是为了可回溯，但**阅读时请以代码为准**。

最显著的不一致：归档的技术选型文档仍列出 `sqlx` 与 `gix`，而 `Cargo.toml` 中两者都不存在
——实际实现是 shell `git` 子进程 + `rust-s3`。

| 目录 | 内容 |
| --- | --- |
| [`docs_archive_rust_impl_2026_08_26/requirements/`](docs_archive_rust_impl_2026_08_26/requirements/) | 15 阶段需求定义过程产物，含 Phase 11 红队评审、Phase 12 UX 红队评审、Phase 15 终审验收 |
| [`docs_archive_rust_impl_2026_08_26/design/`](docs_archive_rust_impl_2026_08_26/design/) | 基本设计书与详细设计书，严格按日本 IPA 共通框架 2013 编写 |
| [`docs_archive_rust_impl_2026_08_26/process/workflow.md`](docs_archive_rust_impl_2026_08_26/process/workflow.md) | 150 个任务 × 13 阶段的工程过程模型 |

该档案自身记录了一个当时成立、现在已不成立的判断，即「尚未有一个真正跑起来的 MVP 验证过」。
以本文上文的实测数据为准。
