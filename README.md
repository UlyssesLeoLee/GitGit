# GitGit

本地优先的 Git HTTP 服务器 + 版本化凭据保险库 + Tauri 桌面端。

> **状态：可运行的 MVP，尚未达到可发布状态。**
>
> 本仓库有真实实现、真实测试和真实 CI 门禁，并且能产出可安装的安装包
> （Windows `.msi`，以及 macOS `.dmg` / Linux `.deb` / Linux `.AppImage`）。
> **HTTP API 鉴权已于 2026-10-05 关闭**（P0）；**MSI 代码签名链路已于 2026-10-07
> 就位并在 tag 发布路径强制**，但仍缺一张采购的证书。详见
> [已知未决问题](#已知未决问题)。在这两项关闭之前，本项目**不应**被当作
> 可商售产品分发。

本文件只陈述**实测**状态。凡本文出现数字，均为在指定 commit 上跑出来的，不是估计值。
未能验证的一律写在「已知未决问题」里，不写在正文里。

## 许可证

本项目采用 **GNU Affero General Public License v3.0（AGPL-3.0）**，
以顶层 [`LICENSE`](LICENSE) 全文为准（35,184 字节，含 AGPL 特有的第 13 条
"Remote Network Interaction"）。

`Cargo.toml`、`apps/gm-desktop/src-tauri/Cargo.toml`、
`apps/gm-desktop/package.json`、`apps/gm-console/package.json` 均已声明
`AGPL-3.0`，与该文件一致。

`[FACT]` 此前这四个位置写的是 `Apache-2.0`／缺失，与 `LICENSE` 直接冲突，
本仓库因此一度不能声明自己采用什么许可证、也不应被分发。冲突已按
「以 AGPL-3.0 为准」的决定解决（2026-10-05）。

**选择 AGPL 而非 Apache-2.0 的实际含义，请先读这一段：**

- 本项目是**网络服务器**。AGPL 第 13 条要求：如果你修改了本项目并通过网络
  提供服务（哪怕只是自己内部使用），**必须**向这些网络用户提供你修改后版本的
  **完整对应源码**。Apache-2.0 没有这条要求。
- 因此：闭源改造后内网自用是**不允许**的（内网用户同样算「通过网络与之交互」）；
  对外分发二进制必须同时提供你的修改版完整源码。
- 义务落在**修改者**身上。未修改的原版不产生新义务——因为本仓库本身已公开
  源码，这一条天然满足。
- 源码获取方式：本仓库本身的公开地址即为原版对应源码的提供方式。若你分发的是
  修改版，需由你附带该修改版的完整源码。

`[PROPOSAL]` 若后续要改变许可证（例如改为 MIT/Apache-2.0 以便闭源分发），
需要同时替换 `LICENSE` 全文与上述四个 manifest 声明，并在本节写明迁移理由 ——
单改一处就会重新制造本轮刚关闭的这类冲突。

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
`/api/*` 自 2026-10-05 起强制 HTTP Basic 鉴权（`GET /api/health` 除外），
凭据来源见 [P0 一节](#p0--http-api-鉴权已关闭)。

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

### 实测快照

> 本表分两段。**2026-10-05 复核行**是本轮（`/api/*` 鉴权 + 凭据外置 +
> AGPL §5(d) 合规）亲自重跑出来的数字。**历史行**是更早快照，保留是为了
> 能看出数字是怎么变过来的，不是当前状态。

| 项 | 数值 | 怎么来的 |
| --- | --- | --- |
| 根 crate 测试 | **214 passed / 0 failed / 2 ignored** | 2026-10-05 本地重跑 `cargo test --locked` |
| 根 crate fmt / clippy | 0 / 0 | 2026-10-05 本地重跑（`--all-targets --locked -D warnings`） |
| `src-tauri` 测试 | **28 passed / 0 failed** | 2026-10-05 本地重跑 `cargo test --locked --lib` |
| `src-tauri` fmt / clippy | 0 / 0 | 2026-10-05 本地重跑 |
| `apps/gm-desktop` 测试 | **407 passed / 27 files / 0 skipped** | 2026-10-05 本地实测，CI 复核 |
| `apps/gm-desktop` 覆盖率 | lines 95.01% / branches 78.79% / functions 94.59% / statements 94.34% | 2026-10-05 本地实测，`pnpm test:coverage` **exit 0** |
| `apps/gm-console` 测试 | **201 passed / 16 files** | 2026-10-05 本地实测 |
| `apps/gm-console` 覆盖率 | statements 94.1% / branches 87.72% / functions 82.39% / lines 94.1% | 2026-10-05 本地实测，门槛 70 / 70 / 60 / 55 |
| i18n 键一致性（`gm-desktop`） | `en` 190 / `zh-CN` 190，键集完全一致 | 2026-10-05 本地实测 |
| i18n 键一致性（`gm-console`） | `en` 7 组 / `zh-CN` 7 组，结构一致 | 2026-10-05 本地实测 |
| RUSTSEC 公告 | 0（扫描 240 个依赖） | CI `audit` job，**本轮未重跑**，沿用历史值 |
| 根 crate 测试（历史，`6d24b21`） | 186 passed / 0 failed / 2 ignored | CI 实测于 `f690486` |
| `apps/gm-desktop` 测试（历史，`6d24b21`） | 378 passed / 25 files / 0 skipped | 本地实测，CI 复核 |
| `apps/gm-console` 测试（历史，`6d24b21`） | 194 passed / 15 files | 本地实测 |

`apps/gm-desktop` 的覆盖率阈值（lines/statements 70、functions 60、branches 55）声明在
`vite.config.ts`，CI 的 `Coverage gate` step 现在**真的会求值它们**。
这一度曾经只是"看起来像门禁"：CI 跑的是 `pnpm test`（不带 `--coverage`），
而当时 `test:coverage` 是 exit 非零的。2026-10-05 补齐路由组件与 stores/api 层的测试后，
`All files` 从 42.55% 提到 94.78%，门禁才真正成立。阈值未作任何下调。
`apps/gm-console` 的四个阈值同样**未下调**（70 / 70 / 60 / 55）。

MSI 产物（CI 实测于 `d81f285`）：perUser 15,194,299 B / perMachine 15,193,981 B /
manifest 640 B / dist 271,164 B，共四个 artifact 并存。**本轮未重建，未复核。**

## 已知未决问题

以下条目列在这里是为了让任何评估者看到完整图景，而不是只看到绿 CI。
标记为「已关闭」的条目保留在此并写明结论与日期，而不是删掉——一个消失的
阻断项比一个带日期的阻断项更难复核。

### 已关闭 — 许可证声明冲突（原 P0）

顶层 `LICENSE` 是 **GNU AGPL-3.0 全文**（35,184 字节，首两行为
`GNU AFFERO GENERAL PUBLIC LICENSE Version 3`），而 `Cargo.toml` 与
`apps/gm-desktop/src-tauri/Cargo.toml` 此前都声明 `license = "Apache-2.0"`。
两者不可能同时为真。

**已按「以 AGPL-3.0 为准」解决（2026-10-05）**：两个 manifest 改为
`license = "AGPL-3.0"`，两个 `package.json` 补上同一声明，README 新增
[许可证](#许可证) 章节写明选择 AGPL 的实际约束（尤其是第 13 条的
网络交互源码义务）。分发前请先读该章节。

### P0 — HTTP API 鉴权（已关闭）

**已关闭**。`/api/*` 现强制 HTTP Basic 鉴权，编译期硬编码的 `admin` / `admin`
已从二进制移除。

- `src/server/api.rs` 的 `build_api_router()` 在**子路由**上施加
  `route_layer(require_api_auth)`，未认证返回 401。逐 handler 加检查的写法被
  明确否决：那是「下一个端点被忘记加检查」的形成方式。`GET /api/health`
  刻意留在层外——需要管理员密码才能探活的探针，在密码正是问题所在时无法
  告诉你服务是否正常。
- `auth_optional` 这个 no-op 与 `src/config.rs` 的 `ADMIN_USER` / `ADMIN_PASS`
  常量已删除。
- 凭据外置，解析顺序：`GITGIT_ADMIN_PASS` / `GITGIT_ADMIN_USER` → 保险库
  `gitgit.password`（桌面端 Settings 写入的那一项）→ 启动时生成的 128-bit
  随机密码（CLI 打印一次到控制台）。
- **非回环绑定 + 随机密码 → 拒绝启动**（`enforce_exposure_policy`），CLI 与
  桌面内嵌服务器都调用。通配地址配一个没人知道的随机密码，比拒绝启动更糟。
- 桌面侧此前「`bind` 由前端传入且零校验」的路径，现在同样受该策略约束。
- Web 控制台 `apps/gm-console` 新增登录门（`src/components/LoginGate.tsx`），
  凭据只存内存：刻意不做 `localStorage`，也不用 `VITE_*` 变量——后者会被
  内联进发给每个浏览器的 JS 包。

**反向验证**：删掉 `.route_layer(require_api_auth)` 会让 7 条鉴权测试变红，
其中包含一张覆盖全部 `/api` 路由的路由矩阵，诊断信息指名道姓地说明是哪条
路由没被鉴权层覆盖。恢复后全绿。

**本轮明确未解决的残留风险**（不是缺陷，是尚未提供的能力）：Basic 凭据是
base64 而非加密，**无 TLS**；桌面端生成的密码只出现在日志里（无控制台），
这会 fail closed，但意味着在 Settings 里设密码之前，Web 控制台无法对接加宽
后的绑定。完整清单见 [`SECURITY.md`](SECURITY.md)。

### P1 — Windows 安装包缺一张代码签名证书

**签名链路本身已经关闭，剩下的只是证书。**

`[FACT]` 2026-10-07：`apps/gm-desktop/scripts/sign-msi.ps1` 用 PFX + RFC 3161
时间戳签名 4 个 MSI 变体，签完重算 `manifest.json` 的哈希并记录 `signing` 块。
`release.yml` 的 `msi` job 以 `-Required` 调用它 —— **没有证书就直接变红**，
不会发出一个自己以为签过名的发布。`gm-desktop.yml` 的日常 `msi` job 不带
`-Required`：GitHub 不向 fork 的 PR 下发 secrets，强求只会把每个 fork PR 弄红
而不增加任何安全性。两者共用同一个脚本，差别只有这一个开关。

`[FACT]` 签名已用真实的 `signtool.exe` 对 4 个**实际产出的** MSI 本地验证过，
覆盖了正确路径与两条错误路径（无证书 + `-Required` → exit 1；PFX 损坏 → exit 1）。

仍然缺的是**一张采购的代码签名证书**，这不在技术范围内。配置方式：

| Secret | 内容 |
| --- | --- |
| `MSI_SIGN_PFX_BASE64` | `signing.pfx` 文件本身的 base64（不是 `.cer`） |
| `MSI_SIGN_PFX_PASSWORD` | 导出该 pfx 用的密码 |

```powershell
[Convert]::ToBase64String([IO.File]::ReadAllBytes('signing.pfx'))
```

在配置之前，**打 tag 会让 release 在预检后的 `msi` job 变红**，这是设计意图
而不是故障。

### ~~P1 — 覆盖率门禁是声明而非门禁~~ 已关闭

`apps/gm-desktop` 的覆盖率阈值曾经**没有任何 CI job 求值**：`Unit tests` 步骤
调用的是 `pnpm test`（即不带 `--coverage` 的 `vitest run`），`vite.config.ts` 里
的 `thresholds` 块从未被读取。配置文件里一道没人求值的阈值，读起来像门禁，实际不是。

`[FACT]` 2026-10-05 已关闭。`gm-desktop.yml` 现在有独立的 `Coverage gate` 步骤
执行 `pnpm test:coverage`，实测 95.03% lines / 94.4% statements / 94.59% functions /
78.99% branches，对应阈值 70/70/60/55。

该步骤的失败判定不只看 vitest 的退出码，还有四道 guard：确认确实有通过的测试、
没有 forks worker 崩溃、磁盘上的测试文件数与 vitest 报告的 `Test Files (N)` 一致、
汇总行不含 failed。另有 `set -o pipefail` —— GitHub 用 `bash -e` 执行 `run:`
块而**不含** `pipefail`，`pnpm test:coverage | tee` 会返回 `tee` 的 0，把测试失败
吞掉。实测：

```
bash -e -c 'false | tee /dev/null'            -> exit 0
bash -e -o pipefail -c 'false | tee /dev/null' -> exit 1
```

### P1 — AI 链路未对**真实云端 provider** 端到端验证

`src/ai/` 的 provider 实现有完整单元测试（mock SSE server 实测 token 序列
`["Hel","lo","wo","rld","!"]`），但仓内没有 `GITGIT_AI_API_KEY`，
**从未真正调用过 OpenAI 或 Anthropic 的线上端点**。

`[FACT]` 事件链路本身**已在真机验证**（2026-10-05）：对本地流式端点
（`127.0.0.1:38999`，SSE 逐帧吐 7 个 token）实跑 `pnpm tauri:dev`，7 个 token
逐个穿过 Tauri IPC 抵达 webview 并按序拼接；首帧携带的 `model` 值也被页面显示。
Mock 日志确认 Rust 侧确实发出了 `Authorization: Bearer`，且未泄漏到事件里。

所以「AI 评审能否流式出字」已经不再是未知项；仍然未知的是**真实厂商端点**的
行为：鉴权失败形态、限流响应、以及各家 SSE 方言与本仓解析器的差异。

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
notarization。Windows 的 4 个 MSI 变体已有签名链路并在发布路径强制，但未配置
证书前打 tag 会 fail-closed（见上一节）。

`[FACT]` `.deb` 依赖**不是**空数组：`tauri.conf.json` 的
`bundle.linux.deb.depends` 声明了 `libwebkit2gtk-4.1-0` 与 `libgtk-3-0`
（2026-10-05 起）。`verify-bundle.sh` 会把声明列表读回来，逐条断言控制文件里的
`Depends:` 都写到了；缺一条、或列表为空，都是 `die`（硬失败），不再只是打印 NOTE。

> 此前此处记的是「依赖为空数组，bundler 不写 `Depends:`，校验脚本只打印 NOTE」。
> 两处都已过时：配置在 2026-10-05 被补上并同时升级为硬失败。过时记述比没有记述更糟，
> 它会让读者以为这条路径无人看管。

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
