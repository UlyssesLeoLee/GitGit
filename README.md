# GitGit

AI-Native Engineering Platform

本仓库目前的主要内容不是代码，而是一份完整走完 **13 个主阶段 + 2 个补充阶段** 的产品需求调研与定义程序，产出对象是一个暂命名为 **AI-Native Engineering Platform** 的、Local-First / Cloud-Ready / Git-Native / AI-Native / Agent-Native / Graph-Native 的可自托管软件工程平台的正式需求定义书。

所有产出物位于 [`docs/requirements/`](docs/requirements/)。

---

---

> `[FACT]` **2026-11-05 起，本仓库同时包含一个已实现的工作流（V0）**：本地优先的 Git
> HTTP 服务器 + 版本化凭据保险库 + Tauri 桌面端。它的可运行状态、实测快照与未决问题
> 见下面「当前实现（V0）」一节。上面「状态」一节描述的是**需求定义程序**的
> 范围（Baseline v1.0 本身不含实现代码 ✓），不是本仓库今天的内容。

## 这是什么

Git 管理代码版本；这个项目要调研并定义一个更进一步的平台——管理"软件工程状态"本身的版本。它管理的对象不止 Repository / Branch / Commit / Tag，还包括 Requirement、Issue、ADR、PR、Review、Test、CI、Artifact、Release、Deployment、Incident、Agent、AgentRun、Prompt、Skill、Policy 等，最终形成一个可查询的 **Engineering Knowledge Graph**。

整个需求定义过程严格区分**事实与设计**，全文使用统一标记：

| 标记 | 含义 |
|---|---|
| `[FACT]` | 有官方/一手来源支撑并标注访问日期的可验证事实 |
| `[UNVERIFIED-FACT]` | 有来源但未经一手验证（如仅见于第三方聚合站），不可直接当作确证事实使用 |
| `[INFERENCE]` | 基于已知事实的合理推论，未独立核实 |
| `[PROPOSAL]` | 本项目自己的设计主张，明确不归因于任何竞品 |
| `[TBD]` | 现阶段无法确认，留待后续调研/评审/决策 |

禁止把自己的设计构想包装成竞品已有的事实，也禁止为了让文档"看起来完整"而编造未经证实的数字（例如性能指标一律标注 `TBD – Benchmark Required`，而不是凭空给出数字）。

---

## 阶段全景

```
主程序（Phase 1-13）
Phase 1-5   竞品调研 & Gap 分析     ──▶ phase1-5-research.md
Phase 6     产品原语发现            ──▶ phase6-primitives.md
Phase 7     需求草拟（带ID）        ──▶ phase7-elicitation.md
Phase 8     55节正式需求定义书      ──▶ 00-requirements-definition.md
Phase 9     MVP 精简（最小完整闭环）──▶ phase9-mvp-reduction.md
Phase 10    架构设计（能删就删）    ──▶ phase10-architecture.md
Phase 11    红队评审（自我攻击）    ──▶ phase11-red-team.md
Phase 12    UX 红队评审             ──▶ phase12-ux-review.md
Phase 13    最终基线 Baseline v1.0  ──▶ phase13-final-baseline.md

补充阶段（在 Baseline v1.0 之上追加）
Phase 14    日本 IPA 标准合规评审    ──▶ phase14-ipa-compliance-review.md
Phase 15    终审验收（一致性核查）   ──▶ phase15-final-audit.md
```

每个阶段的产出都以独立文件保存、独立提交、经过 PR 走查后合入 `main`，任何阶段都可以单独回溯查阅推理过程，不是一次性生成的"完整幻觉"。

---

## 文档索引（`docs/requirements/`）

| 文件 | 内容概要 |
|---|---|
| [`phase1-5-research.md`](docs/requirements/phase1-5-research.md) | 对 GitHub、GitLab、Gitea/Forgejo、Bitbucket、Sourcegraph、OpenAI Codex、Claude Code、Cursor、Linear 等的深度竞品调研；能力矩阵；Commodity/Differentiator/Emerging/Experimental 四象限 Gap 分析 |
| [`phase6-primitives.md`](docs/requirements/phase6-primitives.md) | 从 Observation→Pattern→Constraint→Primitive→Interaction→Workflow→Capability 的涌现式推导；最终 MVP 原语集合为 **Node / Edge / Event / Policy / View**（Agent 作为 Node 子类型，Action/Evidence 延后，Intent/Context 被砍） |
| [`phase7-elicitation.md`](docs/requirements/phase7-elicitation.md) | 按 GIT / GRF / AGT / AI / CTX / SEC / CI / UX / OPS / CLOUD 十个前缀分类、带唯一 ID、含验收标准的原子需求草稿 |
| [`00-requirements-definition.md`](docs/requirements/00-requirements-definition.md) | **主文档**：55 节正式需求定义书 + ADR 待定清单附录（Baseline v1.0） |
| [`phase9-mvp-reduction.md`](docs/requirements/phase9-mvp-reduction.md) | 用"最小完整闭环"（Repository→Issue→AI Context→Agent Branch→Code Change→CI→AI Review→Human Approval→Merge→Graph Update）作为唯一标准，严格砍需求而非堆功能数量 |
| [`phase10-architecture.md`](docs/requirements/phase10-architecture.md) | Principal Architect 视角的架构评审：技术栈逐项评估、图存储方案决策、Git 存储实现方式决策、服务边界、Local→Cloud 迁移路径、"哪些组件被砍掉"清单 |
| [`phase11-red-team.md`](docs/requirements/phase11-red-team.md) | 从 17 个角度对整个方案发起真实攻击（是否只是 GitHub Clone、是否 AI 包装、是否过度工程、Agent 执行安全面、本地部署是否过重等），17 条发现 |
| [`phase12-ux-review.md`](docs/requirements/phase12-ux-review.md) | UX 红队评审："稳定骨架 + 涌现式上下文"是否真的可执行，Ambient AI 预算（80/15/5 目标）是否会退化成 Chat，9 条发现 |
| [`phase13-final-baseline.md`](docs/requirements/phase13-final-baseline.md) | **收官文档**：对全部 26 条红队/UX 发现逐一处置并实际修订主文档；三大护城河（Moats）分析；正面回答"如果 GitHub/GitLab 明天 AI 提升十倍，我们为何还存在"；宣告 Baseline v1.0 |
| [`phase14-ipa-compliance-review.md`](docs/requirements/phase14-ipa-compliance-review.md) | 按日本 IPA 标准（非機能要求グレード / 上流工程共通フレーム / 情報セキュリティ指南）对 Baseline v1.0 做合规差距分析，9 条发现，新增 NFR-REQ×3 + SEC-REQ×3 |
| [`phase15-final-audit.md`](docs/requirements/phase15-final-audit.md) | 终审验收：需求 ID 连续性/唯一性、跨文档引用完整性、链接有效性、各处声明数字与实际内容的一致性核查，3 条缺陷及修复记录 |

---

## 关键结论速览

### 三大护城河（诚实版本，非营销话术）

1. **统一可查询的工程图谱** —— 留存型护城河而非获客型护城河：每一次循环迭代都在积累图数据，让后续查询和溯源越来越值钱，但新团队接入时图谱是空的，第一天没有价值。
2. **真正的自托管 + 云一致性 + Agent 原生执行** —— 自托管一致性部分持久可信；但"Agent 原生执行"的能力优势建立在前沿模型质量之上,会随所有厂商模型进步而被稀释。
3. **人机统一审计轨迹** —— 三者中最可能随 AI 能力提升而**变得更有价值**的护城河（Agent 越自主，统一审计越重要），但也是技术上最容易被竞争对手直接复制的一个。

三者都**没有真正的网络效应**——项目坦诚承认，没有为了让故事好看而编造。

### 终极判断题

> 如果 GitHub 和 GitLab 明天把 AI 能力提升十倍，这个产品为什么还存在？

诚实回答：会继续存在，但只对真正看重自托管控制权、含 Agent 的审计严谨性、以及积累的工程图谱——胜过原始 AI 能力本身——的那部分组织而言；会是一个更小、更持久的细分市场，而不是 GitHub/GitLab 的大众替代品。且明确承认：这个论证目前只停留在纸面，尚未有一个真正跑起来的 MVP 验证过。

### MVP 范围

MVP 需求数演进：45（Phase 9 初始）→ **37**（Phase 9 按最小完整闭环砍减）→ **43**（Phase 13 红队/UX 评审后补回 4 条）→ **45**（Phase 14 IPA 合规补 2 条）。数字有涨有落，每一步的增减理由都可追溯，不是单向膨胀。

### 评审发现处置统计

**Phase 11 红队 + Phase 12 UX 评审（共 26 条）**

| 处置结果 | 数量 |
|---|---|
| 已接受并实际修复主文档 | 19 |
| 已接受但延后至 V1/V2 | 4 |
| 已驳回（附论证） | 2 |
| 需人类决策（无法靠分析解决） | 1 |

**Phase 14 IPA 合规评审（共 9 条）**：已修复 5 / 延后 V1 共 3 / 需人类决策 1。

**Phase 15 终审（共 3 条）**：全部已修复，均为汇总表记账错误，无实质内容缺失。

全部 4 条 Critical 级发现（3 条红队 + 1 条 UX）均已获得实际修复,而非被归入"待办事项"敷衍了事。

---

## 阅读建议

- 只想看结论：直接看 [`phase13-final-baseline.md`](docs/requirements/phase13-final-baseline.md) 的 §4（三大护城河）、§5（终极问题）、§6（Baseline 声明）。
- 想看完整正式需求：看 [`00-requirements-definition.md`](docs/requirements/00-requirements-definition.md)，目录附有全部 55 节链接。
- 想看某个具体判断是怎么来的：按上面的阶段顺序从 Phase 1 往后读，每个阶段都写明了方法、证据来源和推理过程。
- 想看这个项目"骗不骗人"：重点看 Phase 11（红队评审）、Phase 12（UX 红队评审）和 Phase 15（终审验收）——前两个阶段的存在是为了防止前面的阶段自说自话，Phase 15 则是逐条核对各处声明的数字与实际内容是否一致（结果确实查出了 3 处记账错误并已修正）。

---

## 状态

**Baseline v1.1**（Phase 1–13 主程序完成，Phase 14 IPA 合规补充、Phase 15 终审验收、Phase 16 sync（2026-09-01）已并入）。

明确排除在本基线范围之外的内容：具体实现代码、UI 线框图/原型、法律与许可证审查、benchmark 实测数据、市场定位（Go-to-Market）决策、正式 ADR 文档产出物。这些留待后续阶段处理。

`[FACT]` **F14-7（整个程序全程没有人类干系人正式签核环节）已在 Phase 16 sync（2026-09-01）中关闭**，
证据见 [`docs_archive_rust_impl_2026_08_26/process/F14-7-signoff-gap.md`](docs_archive_rust_impl_2026_08_26/process/F14-7-signoff-gap.md)：Ulysses
（一人公司 12 角色，per DEC-008）已对全部 8 份评审记录模板完成签核（per 8/27 19:39/20:56/21:59
JST 三次强化 代签授权），见 commit `b11f6c5`。本段早先把 F14-7 记作「需人类决策的开放问题」，
那是 Baseline v1.0 时的状态；**当前的开放问题以主文档 §54 为准**。

---

## 当前实现（V0）：本地优先的 Git HTTP 服务器 + 版本化凭据保险库 + Tauri 桌面端

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
| `deploy/objectstore/` | 本仓自建的 S3 兼容对象存储夹具，供保险库 e2e 测试使用 |
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
| `gm-desktop.yml` | lint、svelte-check、vitest、Vite 生产构建、`src-tauri` 的 fmt/clippy/test；`msi` job 在 Windows 上真实打包并校验产物，`msi-install` job 再把每个变体**真装上去**、断言装到位、卸干净 |
| `gm-console.yml` | typecheck、lint、vitest、coverage 门禁、format check、生产构建 |

`audit` 做成独立 job 而不是 `test` 里的一个 step：CVE 应该在秒级失败，而不是等完整个 release 构建。
该 job 同时区分「扫描到公告」与「公告库不可达」这两种不同的失败。

`[FACT]` 2026-10-25 起，Windows 主产物也有**真机安装证明**了。此前
`verify-msi.ps1` 只做结构校验——用 `msiexec /a` 拆出 FILE 表、用 COM 读 MSI
表，它自己的最后一行写着 `no install, no elevation`。而 `.deb` 早在 PR #46 就
拿到了裸机容器里的真装证明（`deb installs and links on a stock Ubuntu`），
**Windows 主产物反倒是唯一没有安装证明的那个**。新增的 `msi-install` job 逐变体
执行：先断言机器是干净的（否则后一个变体可能悄悄继承了前一个的安装）→ 真装 →
断言文件落位且与包内载荷**逐字节相同**、包已注册、作用域与包自己声明的一致 →
真卸 → 断言注册项与文件都已消失。四个变体（perMachine/perUser × en-US/zh-CN）
**必须逐个来**：它们共用 UpgradeCode，Windows Installer 会把第二次安装当成升级
并**移除**第一个，一起装最后只剩一个，而 job 会报四次成功。
`[FACT]` 该证明只覆盖「装得上、装得对、卸得干净」，**不覆盖程序能跑起来**——
那是另一个 job（`msi-launch`）的事：它装好**打包后的**应用、启动它、断言弹出顶层窗口
（2026-10-25 实测，窗口标题 `gitgit Desktop`）。两者分开，是为了让「Linux 容器里起不来」
这种与打包无关的失败不至于把安装证明带红。

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

`[FACT]` 证书**怎么进 CI**——secret 名、谁设置、什么时候可用、PR 与 fork 上的
差别——记在 [`.github/CI.md`](.github/CI.md) 的「Secrets 配置」一节，那是唯一一处
完整说明。

`[FACT]` 2026-11-11 起，`release.yml` 的 `msi` job **在构建之前**先断言证书在不在
（`Assert the signing material is configured` 步，跑
`apps/gm-desktop/scripts/assert-signing-config.ps1 -Required`）。缺证书时**第一分钟
就红**，而不是构建十几分钟后才红；消息里说清缺的是哪个变量、证书怎么投递、以及
**不产出任何 release**。

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

### P2 — 跨平台产物已在 CI 构建并安装验证；`.dmg` 未公证、Linux 的 GUI 从未运行过

`tauri.conf.json` 声明了 `["msi","dmg","appimage","deb"]` 四种 target，
此前只有 MSI 被构建过。现已新增 `.github/workflows/gm-desktop-bundle.yml`，
在对应 runner 上产出并校验：

| 产物 | runner | 校验方式 |
|---|---|---|
| `.dmg` | `macos-latest` | `hdiutil imageinfo` + 只读挂载，断言 app 二进制 ≥ 1 MiB；**并从挂载卷里真的启动该应用**，断言 webview 报告了「UI 已挂载」 |
| `.deb` | `ubuntu-latest` | `dpkg-deb --info` / `--contents`，版本必须与 `tauri.conf.json` 一致 |
| `.appimage` | `ubuntu-latest` | ELF magic + `file` + 5 MiB 下限 |
| `.deb`（第二道） | `ubuntu:24.04` 容器 | **真机安装**：`apt-get install ./pkg/*.deb` → `dpkg-query` 断言 `install ok installed` → `ldd` 无 `=> not found` → `desktop-file-validate` |

`[FACT]` 四个 job 均已实测通过。`.dmg` 与 `.appimage` **均未签名**；macOS 未做
notarization，会触发 Gatekeeper。Windows 的 4 个 MSI 变体已有签名链路并在发布
路径强制，但未配置证书前打 tag 会 fail-closed（见上一节）。

`[FACT]` **Windows 上的 GUI 已经被观测运行过（2026-10-31 起）。** `msi-launch`
作业装好**打包后的** per-machine `.msi`、启动它，并断言 webview 报告了「UI 已挂载」
——证据是应用自己日志文件里的那一行标记（CI 日志：
`ok the webview reported that the UI booted: 'GITGIT_UI_BOOT_MARKER_V1 …'`）。

`[FACT]` **macOS 上的 GUI 也已经被观测运行过（2026-11-05 起）。** `dmg` 作业把镜像
**只读挂载**后，**从挂载卷里启动**应用本体（`Contents/MacOS/gm-desktop`），并要求
webview 在应用**自己的日志文件**里写下带「本次启动一次性 token」的那一行标记。实测
（run `42032236510`，`dmg (unsigned, ad-hoc app)` 作业全绿）：

```
==> launch proof for gitgit Desktop_0.1.0_aarch64.dmg (binary 'gm-desktop', identifier 'com.gitgit.desktop')
    ok  mounted read-only at /var/folders/.../T/gm-desktop-dmg-launch-73d1166eeea4489e855c9bd805c7c5de/volume
    ok  application bundle: .../volume/gitgit Desktop.app
    webview-boot marker: GITGIT_UI_BOOT_MARKER_V1 7e5dc073954940659cb54ddea5ce797e
==> starting '.../volume/gitgit Desktop.app/Contents/MacOS/gm-desktop'; waiting for the webview-boot marker
    started (pid 22816)
    ok  the webview reported that the UI booted: '/Users/runner/Library/Logs/com.gitgit.desktop/gitgit Desktop.log'
    ok  log directory: /Users/runner/Library/Logs/com.gitgit.desktop
```

`[FACT]` 日志路径 **`/Users/runner/Library/Logs/com.gitgit.desktop/gitgit Desktop.log`**
与 Tauri 在 macOS 上的 `app_log_dir()`（`~/Library/Logs/<bundle identifier>`）一致；
该 identifier 是从 `tauri.conf.json` 读出来的，不是写死的。

`[FACT]` **判定逻辑没有第二份实现**：macOS 侧直接 dot-source Windows 侧那一份
（`apps/gm-desktop/scripts/msi-launch-log.ps1`——文件名里的 `msi` 是历史名，逻辑本身
与平台无关），两个平台因此不会在「什么算证据」这件事上漂移。标记行带**每次启动
一次性**的 token（`GITGIT_UI_BOOT_NONCE`），所以应用日志里上一轮留下的标记满足不了
本轮的门禁。

`[FACT]` **红路径也是证明的一部分**：同一个作业里另有一个 step，**不启动应用**地跑
一遍证明并**要求它失败**（唯一能写下这枚 token 的进程从未启动，那行标记不可能出现）。
实测同一个 run 里它报的是 `LAUNCH PROOF FAILED: the webview never reported that the
UI booted …`，随后 step 自己为绿——证明这条路是**红**的，不是被跳过的。

> `[FACT]` 第一轮 run（`42030461126`）里，这个红路径 step 自己红了（exit 1）而
> **原始日志里没有任何错误记录**：那一步里唯一的非零退出码是子进程的，而它**本来
> 就该是 1**。所以它现在自带显式 `exit 0`，并在开头关掉
> `$PSNativeCommandUseErrorActionPreference`——宿主 PowerShell 会不会把子进程的退出码
> 传播成会话退出码，是 runner 上 PowerShell 版本的属性，不是本仓的属性。

`[FACT]` **它没有证明窗口看起来对。** macOS 没有 `PrintWindow`，Windows 那条像素
判据（`apps/gm-desktop/scripts/window-capture.ps1`）不在本条范围内；这一条证明的是
「应用启动了，并把产品渲染进了 webview」，不是「界面正确」。它不替代视觉回归测试。

所以「GUI 从未运行过」这句话如今**只对 Linux 成立**，见下面 `.deb` 一节的
`[UNVERIFIED-FACT]`。

#### `.deb` 已在裸机安装通过（2026-10-07，run `37553686524`）

`[FACT]` `deb installs and links on a stock Ubuntu` job 首次运行即通过。实测输出：

```
Setting up gitgit-desktop (0.1.0) ...
    ok  gitgit-desktop -> /usr/bin/gm-desktop (31995040 B)
    ok  127 libraries resolved
    ok  /usr/share/applications/gitgit Desktop.desktop
```

**127 个共享库在 `ubuntu:24.04` 裸机上全部解析成功**，`ldd` 无一条 `=> not found`。
这是该 job 存在的理由：若 `bundle.linux.deb.depends` 缺失或不全，`verify-bundle.sh`
的**全部**结构校验依然会通过，而应用在一台干净机器上根本起不来。

> 为什么必须是容器而不是 `ubuntu-latest`：`deb` **构建** job 在同一台 runner 上已经
> 装了 `libwebkit2gtk-4.1-dev` 等开发依赖，缺依赖的包照样装得上。`ubuntu:24.04`
> 上什么都没有，这才让它成为「用户真正拥有的那种机器」。

> 这次 job 首推时**整组断言一条都没执行**：`container:` job 的每个 `run:` step 跑在
> `/bin/sh`（ubuntu:24.04 上是 dash）而非 bash，`set -o pipefail` 在第 1 行就打死
> 进程，失败信息却指向从未运行过的 `apt-get install` 一步。已由
> `tests/unit/deb-install-gate.test.ts` 的全仓不变式「bash-only 语法不得落在解析为
> `sh` / `pwsh` 的 step 里」覆盖。

`[UNVERIFIED-FACT]` **GUI 仍然从未被观测运行过。** 该 job 刻意不启动窗口：webkit/GTK
应用在无 dbus、无 seat、无硬件加速的容器里启动失败的原因与打包无关，而这类会误红的
门禁最终只会被关掉。此边界写在 job 内并由门禁强制，防止它日后悄悄漂进 README。

`[FACT]` `.deb` 依赖**不是**空数组：`tauri.conf.json` 的
`bundle.linux.deb.depends` 声明了 `libwebkit2gtk-4.1-0` 与 `libgtk-3-0`
（2026-10-05 起）。`verify-bundle.sh` 会把声明列表读回来，逐条断言控制文件里的
`Depends:` 都写到了；缺一条、或列表为空，都是 `die`（硬失败），不再只是打印 NOTE。
2026-10-07 的裸机安装是这一声明的外部验证：`apt` 为它拉入了 222 个包。

> 此前此处记的是「依赖为空数组，bundler 不写 `Depends:`，校验脚本只打印 NOTE」。
> 两处都已过时：配置在 2026-10-05 被补上并同时升级为硬失败。过时记述比没有记述更糟，
> 它会让读者以为这条路径无人看管。

### P2 — 发布路径自 2026-10-06 起未再运行，此后新增的门禁均未被执行过

`[FACT]` `release.yml` 一共只运行过两次，且都在 2026-10-06：run `37469291624`（成功）
与 `37464092079`（失败），两次的 tag 都是 `v0.1.0`。

`[UNVERIFIED-FACT]` 因此，2026-10-06 之后加进这条路径的每一道门禁都**只是配置好了、
从未被执行过**：MSI 签名强制（#34，`-Required`）、manifest↔资产对应校验（#36）、
`Cargo.lock` 门禁（#38），以及 2026-10-24 才补进 `release.yml` 的 `.deb` 裸机安装证明
（`deb-install` job）。它们写在文件里，但一次都没有在真实的 release run 里被求值过；
它们能不能通过，未知。

`[FACT]` 仓库当前配置了 **0 个** GitHub secrets（`gh api repos/UlyssesLeoLee/GitGit/actions/secrets`
→ `total_count: 0`）。所以今天推 tag，`msi` job 会在签名一步直接变红，`draft-release`
（`needs:` 依赖它）不会运行，**不会产出任何 release**。这是设计意图（见上面 P1 一节），
但直接后果就是：这条路径上的门禁至今没有一次在真实运行中被求值。

`[FACT]` 现存 draft release `404746754`（tag `v0.1.0`）构建自 commit `64741bb`
（2026-10-06），**早于 PR #41**（`31afa4b`，2026-10-24，把 AGPL-3.0 许可证装进每个装得下
它的安装包）。该 draft 的四个 target 产物（`.dmg` / `.deb` / `.AppImage` / 4 个 MSI 变体，
共 7 个安装包）**均不含任何一份 AGPL-3.0 许可证副本**：其中的 `.deb` 已被下载并手工解包，
里面只有 5 个文件（二进制、一个 `.desktop`、三个图标），没有任何 licence / copyright /
copying / notice 文件（记录在 PR #41 的提交说明里）；另外三个 target 则是在 `64741bb`
上**根本不存在**能装进许可证的机制——那时的 `tauri.conf.json` 既没有 `licenseFile`，
也没有 `bundle.linux.deb.files` 之类的映射。

`[FACT]` 该 draft **没有发布**：`draft=true`、`published=null`，未认证访问其资产返回 404。
因此它不构成当前存在的暴露面。

`[PROPOSAL]` 处理方式是**用新的 release 取代它**（supersede），而不是删除：删除会抹掉这次
构建存在过的证据，而这份证据正是本节记录的内容。本次核查**没有**发布、删除或修改该 draft。

#### 手动运行（`workflow_dispatch`）此前构建的是分支，不是 tag

`[FACT]` 2026-10-24：这条路径的手动触发方式此前**装错了树**。`release.yml` 里每个 job 的
`actions/checkout@v4` 都没有 `ref:`，而没有 `ref:` 时 checkout 取的是**触发本次运行的那个 ref**。
tag push 时那恰好是 tag，所以 tag 通路是「碰巧对的」；`workflow_dispatch` 时那是操作员选的
**分支**，而 `tag` 输入只被 preflight 读了一次。于是：

- 四个构建 job 编译的是**分支**的树，release 却挂着 tag 的名字；
- 版本校验拿**分支**的 `tauri.conf.json` 去比对 **tag** 的名字——一个树里的数字和另一个树里的
  名字作比较，它可以在产物来自另一棵树的情况下依然显示绿色；
- release 正文写的是分支名（`Automated draft for ${{ github.ref_name }}`）；
- 并发组以 `github.ref` 为键，所以「dispatch tag X」和「push tag X」落在**不同**的组里，
  可以同时对同一个 tag 跑。

`[FACT]` 该缺陷由 commit `d2142d5` 修复：tag 现在被解析一次并作为 job output 发布，每个 job 都按它
checkout，release 正文与并发组也都指向它。修法与证据见该 commit。

`[UNVERIFIED-FACT]` 修好之后，这条路径**依然没有在真实 release run 里跑过**——本节上面的
结论不变：推 tag 会先在 `msi` 的签名一步变红，而 `workflow_dispatch` 属于对外可见的发布动作，
本轮没有做。

### P2 — 工作区根目录选择器未在真实窗口验证

对话框在 `invoke` 边界被 stub 掉，真实 Tauri 窗口下的行为未验证。

## 实现相关文档索引

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
| [`docs_archive_rust_impl_2026_08_26/design/`](docs_archive_rust_impl_2026_08_26/design/) | 基本设计书与详细设计书，严格按日本 IPA 共通框架 2013 编写 |
| [`docs_archive_rust_impl_2026_08_26/process/workflow.md`](docs_archive_rust_impl_2026_08_26/process/workflow.md) | 150 个任务 × 13 阶段的工程过程模型 |

需求定义过程产物**不在归档里**：它们是活文档，在 [`docs/requirements/`](docs/requirements/)
（见上文「文档索引」）。归档曾经留着一份同样的拷贝，两份内容逐字节相同——现在只留活的那一份，
免得有人改了归档那份还以为改到了产品。

该档案自身记录了一个当时成立、现在已不成立的判断，即「尚未有一个真正跑起来的 MVP 验证过」。
以本文上文的实测数据为准。
