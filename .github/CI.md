# CI 接入指南 (V0.1)

本仓 GitHub Actions CI 由 `feature/gm-console-v0.1` merge 进 dev 后
落地。**目前是五份 workflow**（`[FACT]` 2026-10-07 核验 `.github/workflows/` 目录）:

- **`gm-console.yml`** — 网页版 CI (typecheck / lint / test / build)
- **`rust-backend.yml`** — Rust 后端 CI (fmt / clippy / test / release build / audit)
- **`gm-desktop.yml`** — 桌面端 CI (`build` = lint / svelte-check / test / build / cargo check;
  `msi` = Windows 上真实 release 打包并校验产物)
- **`gm-desktop-bundle.yml`** — 桌面端跨平台打包 CI (`dmg` = macOS / `deb` = Linux /
  `appimage` = Linux / `deb-install` = **裸机容器里真装一次**)，产物必须通过结构校验
  才算通过。详见「跨平台打包 CI」与「`.deb` 裸机安装 job」两节
- **`release.yml`** — **仅由打 `v*` tag（或 `workflow_dispatch`）触发**，产出
  `draft: true` 的草稿 release。发布与否是人的决定，不是这个 workflow 的决定。

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

## 跨平台打包 CI：`gm-desktop-bundle.yml`（2026-10-05）

> `[FACT]` `apps/gm-desktop/src-tauri/tauri.conf.json` 声明
> `"targets": ["msi", "dmg", "appimage", "deb"]`，但在本文修订之前，CI **只构建过
> `msi`**。`dmg` / `appimage` / `deb` 从未被任何一次 run 产出过 —— 也就是说
> 配置文件在断言三个从未被验证的平台。

`gm-desktop-bundle.yml` 补上这三个平台，每个平台一个 job，互不依赖
（`[FACT]` 没有任何 `needs:`，与 msi lane 的理由一致：一个 gate 变红不得掩盖另一个）。

### 现在构建什么

| 平台 | target | runner | 结构校验（判定"产物为真"的依据） | 签名 |
| --- | --- | --- | --- | --- |
| macOS | `dmg` | `macos-latest` | `hdiutil imageinfo` 解析镜像 → `hdiutil attach -readonly -nobrowse` **只读挂载** → 断言 `.app` 内 `Contents/MacOS/gm-desktop` ≥ 1 MiB、`Info.plist` 含 manifest 版本 | `[FACT]` **未签名**。`APPLE_SIGNING_IDENTITY=-` 仅为 ad-hoc 签名；**未公证**（notarization 需要 Apple ID / App Store Connect key，本项目没有） |
| Linux | `deb` | `ubuntu-latest` | `dpkg-deb --info` 解析 control 字段、`Version:` 必须等于 `tauri.conf.json` → `--contents` 必须含 `usr/bin/gm-desktop` ≥ 1 MiB 与 `.desktop` 条目 | `[FACT]` 不需要签名，也没有任何签名 |
| Linux | `appimage` | `ubuntu-latest` | 前 4 字节必须是 ELF magic `\x7fELF` + `file(1)` 必须是 64 位 ELF 可执行 + 5 MiB 体积下限 | `[FACT]` 不需要签名 |
| Linux | `deb`（第二道） | `ubuntu:24.04` **容器** | **真机安装**：见下节 | `[FACT]` 不需要签名 |

`[FACT]` 三者的体积下限与"内嵌二进制 ≥ 1 MiB"下限，量级依据是
`docs/reports/2026-10-05-bundle-ci/README.md` 实测的 MSI 内嵌可执行文件 19,577,344 B；
下限取 1–5 MiB，只为区分"真包"与"空包/改名包"，不会成为真包变红的原因。

`[FACT]` 每个 upload 都是 `if-no-files-found: error`（失败诊断用的那一个除外，它是
`if: failure()` + `ignore`，与 msi job 同一处理），理由沿用 msi job 已写明的：
默认 `warn` 会让一个什么都没产出的 job **保持绿色**。

`[FACT]` artifact 名字里的版本来自 verify 步骤写出的 `manifest.json`（workflow 里
`jq -r .version`），不是写死的字符串；`manifest.json` 写完会**重新读回**并与磁盘上的
文件逐项比对 size/sha256，与 `verify-msi.ps1` 对 `msi-out/manifest.json` 的处理一致。

## `.deb` 裸机安装 job：`deb installs and links on a stock Ubuntu`（2026-10-07）

`[FACT]` run `37553686524` 首次运行即通过，实测输出：

```
Setting up gitgit-desktop (0.1.0) ...
    ok  gitgit-desktop -> /usr/bin/gm-desktop (31995040 B)
    ok  127 libraries resolved
    ok  /usr/share/applications/gitgit Desktop.desktop
```

### 为什么容器是承重结构，不是附带配置

`[FACT]` `deb` **构建** job 在 `ubuntu-latest` 上装 `libwebkit2gtk-4.1-dev` 等开发
依赖。一个缺依赖的 `.deb` 在那台机器上照样装得上——依赖已经在那里了。
`ubuntu:24.04` 什么都没有，这才是「用户真正拥有的那种机器」。
所以拿掉 `container:` 是让本 job 静默退化成演出的最可能方式，而门禁把这件事定义为
**失败**而非风格建议。

### 本 job 首推时整组断言一条都没执行

`[FACT]` 首推（run `37551742758`）失败信息：

```
/__w/_temp/1b0f7435-....sh: 1: set: Illegal option -o pipefail
##[error]Process completed with exit code 2.
```

`[FACT]` 带 `container:` 的 job，每个 `run:` step 跑在 `/bin/sh`（ubuntu:24.04 上是
dash），而不是 hosted Ubuntu 默认的 bash。`set -o pipefail` 不是 POSIX；最后一个 step
还用了 `<(...)` 进程替换，dash 连语法都没有。**同一份 YAML 去掉 `container:` 就会跑
bash**，step 内容一个字都不用改，所以 step 本身没有任何东西会提示这个变化。

`[FACT]` 误导性更强的一点：GitHub 把失败归给 `apt-get install` 那一步，而包根本没被
碰过。本 job 存在的每一条断言——`dpkg-query` 状态、`ldd` 解析、`desktop-file-validate`——
在那一刻都仍未被证明。

`[FACT]` 修复是 job 级 `defaults.run.shell: bash`（不是逐步骤写），这样日后新增的
step 继承 bash 而不是静默退回 dash。

### 门禁

`tests/unit/deb-install-gate.test.ts`，14 例。其中一组是**全仓**不变式：
按 runner 的真实优先级（step `shell:` > job `defaults.run.shell` > workflow
`defaults.run.shell` > 平台默认）解析每个 `run:` step 的实际 shell，bash-only 语法
不得落在解析为 `sh` / `pwsh` 的 step 上。

三个变异，各自单独跑完整 470 例：

| 变异 | 结果 |
| --- | --- |
| 删掉 `deb-install` 的 `defaults.run.shell: bash` | 2 红 / 468 绿 |
| **新增**一个无关 workflow 里的 container job（`deb-install` 保持正确） | 2 红 / 468 绿，指向新文件 |
| 该 container job 保留，但把 bash-only 语法写进注释 | 1 红 / 469 绿 |

第二个变异是证明它是全仓不变式、而不是照着出 bug 的那个 job 拟合出来的。
第三个证明注释里提到 `set -o pipefail` 不等于代码用了它。

### 刻意不验证的部分

`[UNVERIFIED-FACT]` **GUI 从未被观测运行过。** 本 job 不启动窗口：webkit/GTK 应用在
无 dbus、无 seat、无硬件的容器里启动失败的原因与打包无关，而会因这些原因变红的门禁
最终只会被关掉。已证明的是打包能控制的部分：包在一台什么都没有的机器上装得上、
依赖表足以**链接**二进制、启动元数据格式正确。该边界写在 job 内并由门禁强制，
防止它日后悄悄漂进文档。

### 已由实跑定论的部分（2026-10-05 首跑，2026-10-07 全部绿灯）

> ~~`[UNVERIFIED-FACT]` **这份 workflow 一次都没有跑过。**~~
> **已过时。** run `37551742758` / `37553686524` 已四次构建并校验通过，
> `dmg` / `deb` / `appimage` / `deb installs and links` 全部 `success`。
> 下表逐条给出当时的判定。

首次 run 的待定清单及其**实际结果**：

| 待定项 | 实际结果 |
| --- | --- |
| 1. AppImage 构建时的联网下载 | `[FACT]` **通过。** `AppRun-{arch}`、linuxdeploy `07333c6`、可选 plugin 三项下载均未阻断构建 |
| 2. `.dmg` 的 `bundle_dmg.sh` 在 headless runner 上 | `[FACT]` **通过。** `--skip-jenkins` 分支足够，未触发 `TAURI_BUNDLER_DMG_IGNORE_CI` |
| 3. `hdiutil attach` 能否在 runner 上成功 | `[FACT]` **通过。** 只读挂载成功，`.app` 内二进制 ≥ 1 MiB 断言成立 |
| 4. `macos-latest` 的架构 | `[FACT]` 按预期不确定；校验脚本刻意不断言 arch 段，实测产出 `aarch64` |
| 5. `src-tauri/Cargo.lock` 陈旧 | `[FACT]` **不成立**——实测已推翻，见下 |

`[FACT]` 第 5 条同样被推翻了。原文称「`src-tauri/Cargo.lock` 仍然是陈旧的」，
并据此推断「`tauri build` 不带 `--locked`，会在 runner 上重新解析并改写 lock，
因此产物不是从已提交的 lock 构建的」。

2026-10-07 在 `apps/gm-desktop/src-tauri/` 实测：

```
cargo metadata --locked --format-version 1
→ exit 0，解析出完整依赖图（3,405,069 B 的 metadata）
→ Cargo.lock 的 SHA-256 前后一致
```

`--locked` 的语义是「断言 `Cargo.lock` 不会发生改变」——若 lock 与 manifest 不一致
或缺项，cargo 直接报错退出。它以 0 退出并解析出整张图，且 lock 逐字节未变，说明
lock 与 `Cargo.toml`（含 `gitgit = { path = "../../.." }` 的按路径依赖）是自洽且完整的。

因此「产物不是从已提交的 lock 构建的」这一推论不成立：cargo 在 lock 自洽时按 lock
构建，并不会「重新解析」。`[INFERENCE]` 仍然成立的那一小部分：`tauri build` 确实
没有显式传 `--locked`，所以这个性质目前是**成立的**而非被**保证的**——它依赖
「lock 恰好是自洽的」这一事实，而不是被门禁钉住。要把它变成保证，需要在打包
路径上让 cargo 收到 `--locked`。

### `.deb` 的 `Depends:` 字段 —— 此前记述已双重过时

> ~~**已知问题：`.deb` 不会有 `Depends:` 字段**~~
> **已过时。** 本节原有五条论断全部作废，保留在此是因为「过时记述比没有记述更糟」——
> 它会让读者以为这条路径无人看管。

- ~~`[FACT]` `tauri.conf.json` 中 `bundle.linux.deb.depends` 是 `[]`。~~
  → `[FACT]` 自 2026-10-05 起声明了 `libwebkit2gtk-4.1-0` 与 `libgtk-3-0`。
- ~~`[INFERENCE]` 产出的 `.deb` 不会声明任何依赖，装到干净机器上大概率起不来。~~
  → `[FACT]` **已被推翻**，见下面的实测。
- ~~`[FACT]` `verify-bundle.sh` 检出这一点后只打印 NOTE 不判红。~~
  → `[FACT]` 2026-10-05 起升级为 `die`（硬失败）：列表为空、或声明的依赖没写进
  控制文件，都会让 job 变红。

`[FACT]` **外部验证**（run `37553686524`，2026-10-07）：`ubuntu:24.04` 裸机上
`apt-get install ./pkg/*.deb` 为这份依赖声明拉入 **222 个包**（131 MB），随后
`ldd` 报告 **127 个共享库全部解析成功**，无一条 `=> not found`。也就是说
`[INFERENCE]` 那一行担心的「装到干净机器上起不来」，实测没有发生。

## 触发器定义

> `[FACT]` 下表逐字段核对自五个 workflow 的 `on:` 块，不是凭印象写的。**paths 过滤
> 本身是有历史教训的**：`gm-desktop.yml` 曾漏掉根 `Cargo.toml` / `Cargo.lock`，
> 于是一次只改这两个文件的提交绕过了整个 workflow，一个陈旧 lock 造成的失败
> 整整一天没人看见。现在两个桌面 workflow 都显式列出根 manifest。

| workflow | push | `pull_request` | 其它 |
| --- | --- | --- | --- |
| `gm-console` | `dev` / `main` / `feature/gm-console-v0.1` | `dev` / `main` | — |
| `rust-backend` | `dev` / `main` / `feature/gm-console-v0.1` / `feature/gm-desktop-v0.1` | `dev` / `main` | — |
| `gm-desktop` | `dev` / `main` | `dev` / `main` | — |
| `gm-desktop-bundle` | `dev` / `main` | `dev` / `main` | `workflow_dispatch` |
| `release` | — | — | **tag `v*`** + `workflow_dispatch`（需传 `tag`） |

`[FACT]` `paths` 过滤：

- `gm-console`：`apps/gm-console/**`、`src/server/api.rs`、自身
- `rust-backend`：`src/**`、`Cargo.toml`、`Cargo.lock`、`scripts/**`、自身
- `gm-desktop`：`apps/gm-desktop/**`、**根 `Cargo.toml` / `Cargo.lock`**、自身
- `gm-desktop-bundle`：`apps/gm-desktop/**`、**根 `Cargo.toml` / `Cargo.lock`**、
  **自身与 `gm-desktop.yml`**

`[FACT]` 后两条是有理由的，不是顺手多写：`gm-desktop-bundle` 要跑
`tauri build`，而 `src-tauri/Cargo.toml` 通过 `path = "../../.."` 按路径依赖根
crate，所以改根 manifest 能打断它的构建；而一个构建同一 crate 的 workflow 必须对
「那个 crate 怎么被构建」的变化有反应，所以它同时监听 `gm-desktop.yml`。

`[FACT]` `release` 的触发条件与前四个根本不同：前四个由 push / PR 触发，
`release` 只由 **打 tag** 或 `workflow_dispatch` 触发。这就是为什么「打 tag」是一个
不可撤销的动作，也是为什么 MSI 证书未配置时 release 会在 `msi` job **失败关闭**
而不是静默产出未签名包——`draft: true` 是整条发布链路的安全性支点。

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
   —— **2026-10-05 再次解除（部分）**：`gm-desktop-bundle.yml` 已落地，
   `.dmg` / `.deb` / `.appimage` 三个 target 各自有独立 job 并通过结构校验，
   见「跨平台打包 CI」一节。`[FACT]` 本条的"追加 `needs: build`"建议**未被采纳**：
   msi lane 的报告已论证 `needs` 会让一个 gate 变红掩盖另一个，且慢 job 与快 job
   不应共享 `cancel-in-progress: true`（见该报告第 6 节），故另起文件而非追加 job。
   `[FACT]` **仍未覆盖**：代码签名、Apple 公证（notarization）、Tauri updater 签名
   产物、以及把产物发布到 release —— 本仓无任何证书与 Apple 凭据。
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
   `[FACT]` ~~完整 `tauri build` / `.msi` 仍不在 CI 内（见第 2 条）~~
   → **已过时。** `.msi` 已由 `gm-desktop.yml` 的 `msi` job 在 `windows-latest` 上
   真实打包并校验（PR #33 / #34），`release.yml` 也已在 tag 上端到端跑通过一次，
   产出 11 个资产的 draft release。见「跨平台打包 CI」与 `.dmg` / `.deb` 两节。

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

`gm-desktop-bundle.yml` 的 `paths` 包含:
- `apps/gm-desktop/**` — 桌面端任何变动
- `.github/workflows/gm-desktop-bundle.yml` — CI 文件本身
- `.github/workflows/gm-desktop.yml` — 姊妹 workflow，其工具链/前置依赖变动会影响本 workflow
- `Cargo.toml` / `Cargo.lock` — 桌面端 crate 以 **path** 依赖根 crate，根清单变动会影响 `--locked` 类步骤
- `src/**` — `[FACT]` 与 `gm-desktop.yml` **有意不同**：桌面端二进制链接根 crate 的
  library target，`src/**` 的改动同样能让 macOS / Linux 上的 `tauri build` 编译失败
- `[PROPOSAL]` `gm-desktop.yml` 缺 `src/**`，存在同样的漏检面；该文件不属本 lane，未改

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