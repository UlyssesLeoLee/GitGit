# V0 WBS — Tauri GUI + AI/Remote 集成

> **Plan ID**: V0 | **Branch**: `feature/v0-gui-and-keychain`
> **Owner**: Mavis (per DEC-008) | **Reviewer**: Ulysses (DDD Review)
> **Anchor**: gitgit MVP commit `1da5f2c` on `simplify/2026-08-26-mvp`
> **ADR**: [ADR-0020](../adr/0020-v0-gui-tauri-svelte.md)
> **Started**: 2026-08-26 17:06 JST

## 范围（in scope）

V0 = Tauri 2 GUI 骨架 + 两套 API Key（AI provider + Git remote），
**在 gitgit MVP 之上叠加**，单 Rust crate `gitgit` + Tauri 2 desktop app。

## 任务分解（11 个工作块，约 8-10 个工作日）

| # | 任务 | 工时 | 依赖 | 验收 | 状态（2026-10-02 核验） |
|---|---|---|---|---|---|
| 1 | **Tauri 2 脚手架** — `pnpm create tauri-app` 加 src-tauri/，能 `cargo tauri dev` 出窗口 | 0.5 d | — | 窗口出现，标题 "gitgit" | `[FACT]` 脚手架已落地；`tauri.conf.json` 实际 `productName = "gitgit Desktop"`，与验收写的 "gitgit" 不一致 |
| 2 | **axum 同进程 embed** — gitgit 现有 `axum::serve` 跑进 Tauri `setup()`，端口 38080 | 0.5 d | 1 | Svelte fetch `127.0.0.1:38080/info/refs` 拿到 200 | `[FACT]` 内嵌已落地（`src-tauri/src/lib.rs` + `state.rs:38` `127.0.0.1:38080`）；验收的 HTTP 200 未复现 → `[TBD]` |
| 3 | **Svelte 5 基础** — 路由、布局、主题（系统默认） | 1 d | 1 | 窗口能切 3 个 page（仓库列表 / 详情 / 设置） | `[FACT]` desktop 侧已落地（`svelte ^5.1.0`，7 个 route 在盘）。**注意 `apps/gm-console` 是 React 18，非 Svelte**；主题/i18n 现状见下方矛盾更正 |
| 4 | **仓库视图** — 列表 + status + log(最近 5) + diff 视图 | 1.5 d | 2, 3 | 选个 repo 看到 working tree status + commit graph | `[FACT]` **部分落地**：列表 + refs + commit graph 在盘（`routes/Repos.svelte` / `routes/RepoDetail.svelte`）；working tree status 视图与 diff 视图未找到 → 未落地 |
| 5 | **PG 18.6 + sqlx** — migration 5 张表 + `migrations/20260826_v0_gui_keychain.sql` | 0.5 d | — | `sqlx migrate run` 通过 | `[FACT]` **未落地，且该路线与现行代码冲突** — `Cargo.toml` 无 sqlx / 无 PG driver，`dev` 上无 `migrations/` 目录，keychain migration 从未提交。详见下方「矛盾更正 A」 |
| 6 | **Credential Vault (minIO)** — `trait Vault` + `MinioVault` 实现 (S3-compatible 对象存储) + `FileVault` V1 降级 fallback | 1 d | 5, 11 | `gitai key set openai <key>` PUT 到 `gitgit-vault` bucket，读回能验证 | `[FACT]` 已落地（`src/server/vault.rs` + `vault_versioned.rs`，`rust-s3 0.37`，ADR-0021/0022）。**但 CLI 真实形态是 `gitgit key set`，非验收写的 `gitai key set`**（见 T7） |
| 7 | **AI provider 注册表** — 5 个 provider + `gitai` 子命令（commit/explain/review） | 2 d | 6, 5 | `gitai commit --from-diff` 真打通 OpenAI，输出 commit message | `[FACT]` **已落地**（`src/ai/`，`gitai commit` / `explain` / `review` / `providers`）。`[FACT]` 验收「真打通 OpenAI」**未验证** — 需要真实 `GITGIT_AI_API_KEY`，本仓无凭据；provider 传输层由 mock HTTP server 覆盖。`[FACT]` 计数口径：**5 个具名 preset（openai / deepseek / ollama / vllm / anthropic）跑在 2 种 wire 协议上**，归档设计 §5.5 明确 Ollama 复用 OpenAI 兼容客户端，故未实现 5 份适配器（详见 `src/ai/registry.rs` 模块注释）。`[FACT]` 归档 §5.4 的 `Stream()` / `EstimateCost()` 标为 V1，**未实现也未 stub** |
| 8 | **remote provider 注册表** — `gitremote` 子命令（add/ls/rm/sync） | 1.5 d | 6, 5 | ~~`gitremote add gitee <url>` 存 PG~~ → `[PROPOSAL]`（Ulysses 2026-10-03 拍板）**验收改为**：`gitremote add <name> <url>` 存本地 + fast-forward sync 工作 | `[FACT]` **已落地**（`src/remote/`，`gitremote add/ls/rm/sync`）。`[FACT]` **验收口径已变更**：原「存 PG」与 ADR-0022 §2.1「不引入 sqlx / PG 到 gitgit」直接冲突，且 ADR-0022 已把「对接 AssetsLake schema + sqlx」列为**已否决方案**（风险 #2）。经 Ulysses 拍板，**保持 ADR-0022 不变**，存储改为 `.gitgit/remotes.json`（与 `FileVault` 根并列，非其内部）。`[FACT]` sync 只在 fast-forward 时推送，拒绝时返回 `refused (not a fast-forward)` 而非泛化失败 |
| 9 | **AI 评审 UI** — push 前弹窗 + streaming token 显示 | 1 d | 4, 7 | GUI 上能看到 token 逐字流 | `[FACT]` **已落地**（2026-10-05）— 验收口径「GUI 上能看到 token 逐字流」已由 mock SSE server 实测达成：事件序列 `model,token,…,done`，token 为 `["Hel","lo","wo","rld","!"]`。分两层：层 1 `src/ai/stream.rs`（纯 SSE 解码器 + `AiProvider::send_stream`，11 条无 socket 测试）；层 2 Tauri 命令 `ai_review_start` / `ai_review_cancel`（可取消、不阻塞 UI、密钥不入日志/错误/前端）+ `src/routes/Review.svelte` + `src/lib/stores/review.ts`。`[FACT]` i18n 两份目录各 146 键、键集与顺序完全一致（原 118）。`[FACT]` gm-desktop Rust 测试 8 → 28，vitest 76 passed，`svelte-check` 0 error。`[FACT]` Anthropic 走的是**显式拒绝**（`supports_streaming() == false` + 返回错误），不是静默降级——把非流式调用伪装成流式会让 UI 卡住且看起来像挂起。`[UNVERIFIED-FACT]` 未对真实 provider 验证：仓内无 `GITGIT_AI_API_KEY`。`[UNVERIFIED-FACT]` `TauriEmitter` 适配层（事件真正抵达 webview）未端到端验证。`[FACT]` 层 2 的测试抓到并修掉一个安全缺陷：401 响应体回显 `Authorization` 头会把密钥带进前端错误载荷，现已在命令层统一脱敏 |
| 10 | **MSI 打包 + 烟测 + ADR 收尾** | 0.5 d | 全部 | `cargo tauri build` 出 .msi，scripts/smoke.ps1 仍过 | `[FACT]` **已落地**（2026-10-04）— 4 个 MSI 全部产出并实测装卸：per-user / per-machine × en-US / zh-CN（6,742,016 B 与 6,737,920 B，SHA-256 已记录于 `docs/reports/2026-10-03-tauri-packaging/tauri-packaging.md`）。非提权会话（Medium integrity, `IsElevated=False`）实测：安装 exit 0 → `%LOCALAPPDATA%\Programs\gitgit Desktop\gm-desktop.exe` + 开始菜单快捷方式 + HKCU 6 个值；安装后二进制可启动（窗口标题 `gitgit Desktop`，40 线程，1 个 WebView2 子进程）；卸载 exit 0，目录/注册表/开始菜单全清。`[FACT]` 打包过程修掉一个致命缺陷：应用此前**启动即崩**——`tauri.conf.json` 里三段死 `plugins.*` 配置（log / notification / updater）在加载期即致命（后两者无配置类型；updater 的 `pubkey` 必填且无默认）。提交 `344bc87`。`[TBD]` 4 个包**均未代码签名**，Windows SmartScreen 会告警，需购买证书。`[TBD]` 完整 `tauri build` 仍不在 CI 内。`[FACT]` 两个变体写同一 bundle 输出路径（MSI 名 `{productName}_{version}_{arch}_{lang}.msi`，不含 install scope），进发布流水线前必须先解决互相覆盖 |
| 11 | **minIO 部署前置** — docker-compose 起 minIO 容器 + `gitgit-vault` bucket provisioning + 网络可达性验证 + TLS 关闭（dev 阶段） | 0.5 d | — | `mc alias set local http://localhost:9000 minio minio123` + `mc mb local/gitgit-vault` 成功，curl `http://localhost:9000/gitgit-vault?list` 返回 bucket 列表 | `[FACT]` **已落地**（2026-10-05）— `deploy/minio/docker-compose.yml`（dev/test 专用，端口仅发布到 127.0.0.1，镜像 tag 从运行镜像自身 `--version` 读出）+ `scripts/verify-minio-vault.ps1` 可复现验证。实测：minIO `RELEASE.2025-09-07T16-13-09Z` 启动后 `local/gitgit-vault versioning is enabled`（versioning 是承重项：`VaultVersionSummary` 依赖 `x-amz-version-id`，而 minIO 只对已开 versioning 的 bucket 返回该头）、bucket 为 `private`、未签名 `GET /gitgit-vault?list` 返回 **403**。**验收口径更正**：原写的「curl 返回 bucket 列表」在不签名时不可能成立（列举 bucket 需 SigV4 签名），诚实形式是 403——证明 bucket 存在且非公开；返回 200 反而是缺陷。`[FACT]` 让两条 `#[ignore]` minIO e2e 测试**首次真跑**，暴露并修复 2 个真实缺陷：`rust-s3` 0.37 `get_object` 不检查状态码、把 404 的 XML 错误文档当凭据返回（`is_not_found` 因此从未命中）；`get_at_version` 对从未写过的 key 返回 `Err` 而非契约要求的 `Ok(None)`。`[FACT]` 产品本身**未**使用 minIO：`build_vault()`(src/main.rs) 写死 `FileVault`，无 CLI 开关可切 `MinioVault`。`[FACT]` MinIO 服务端为 GNU AGPLv3（其 `--version` 自述），此处仅作开发测试夹具、不随产品分发。 |

**总**：约 10.5 天（1.5 周强）

## 状态核验（2026-10-02）

**核验基线**：`dev` @ `726ca3a`（= `origin/dev`）。
**核验方法**：状态只承认三类证据 —— ① `git log` 中的具体 commit SHA；② 磁盘上实际存在的文件；③ ADR 条款。

> **重要**：「验收」列描述的是**意图**，不是「已达成」的证据。本节状态不依据验收列反推。
> 证据分级标签按 `README.md` 约定：`[FACT]` / `[UNVERIFIED-FACT]` / `[INFERENCE]` / `[PROPOSAL]` / `[TBD]`。

| # | 状态 | 证据 |
|---|---|---|
| T1 | `[FACT]` 已落地 | commit `2f8b9fc`；`apps/gm-desktop/src-tauri/`；`apps/gm-desktop/package.json` `@tauri-apps/api ^2.1.1`；`tauri.conf.json` `productName = "gitgit Desktop"` |
| T2 | `[FACT]` 已落地（验收未复现） | `apps/gm-desktop/src-tauri/src/lib.rs` 内嵌 `axum::serve` + `ServeManager`；`src-tauri/src/state.rs:38` `DEFAULT_BIND = "127.0.0.1:38080"`。验收中的 `/info/refs` 返回 200 本次未复现 → `[TBD]` |
| T3 | `[FACT]` 已落地 | `apps/gm-desktop/package.json` `svelte ^5.1.0` + `svelte-spa-router ^4.0.1`；`apps/gm-desktop/src/routes/` 7 个 route（Home / Repos / RepoDetail / Settings / Graph / Vault / NotFound） |
| T4 | `[FACT]` 已落地 | 列表 `routes/Repos.svelte`；refs + commit graph `routes/RepoDetail.svelte`；**working tree status 视图 + diff 视图：`src/repo/status.rs`（porcelain v1 解析 + 三种 diff 目标 + 未跟踪文件合成 diff，25 个测试，其中 7 个跑在测试自建的真实 git 仓库上）、`src-tauri/src/commands/repos.rs` 的 `repo_status` / `repo_diff`、`src/lib/components/RepoWorktree.svelte`（由 `RepoDetail` 挂载）、`src/lib/stores/worktree.ts`（12 个测试）**。`[FACT]` 裸仓库无工作区，`git status` 退出 128，由 `is_bare_repo` 判定并返回 typed `NotAWorkTree`，UI 给出可执行提示而非 git 原始 stderr。`[TBD]` HTTP 层（`/api/repos/:name/status|diff`）未新增端点，本次只交付 GUI 路径 |
| T5 | `[FACT]` 未落地，路线与现行代码冲突 | 见下方「矛盾更正 A」 |
| T6 | `[FACT]` 已落地 | `src/server/vault.rs` + `src/server/vault_versioned.rs`；`Cargo.toml` `rust-s3 0.37`；ADR-0021 / ADR-0022；commits `bdb328b`（VersionedVault）、`0728213`（Vault CLI + AppState）、`eb8eddf`（sidecar restore）、`d3d184e`（versionId per ADR-0022） |
| T7 | `[FACT]` 已落地（传输层有测试，真实 OpenAI 未验证） | `src/ai/{mod,provider,registry,openai,anthropic,sanitize,prompt,diff}.rs`；`src/cli.rs` 新增 `Gitai` 变体；`src/main.rs` 新增 `run_gitai`。`[FACT]` 安全项按归档 AISEC-REQ-001/004 实现：不可信内容结构化打标 + 启发式密钥过滤（`src/ai/sanitize.rs`）。`[FACT]` 密钥只从 `GITGIT_AI_API_KEY` 环境变量读，不走 CLI flag（flag 会进 `ps` 和 shell history）。`[TBD]` 真实 OpenAI/Anthropic 端到端未跑通 — 缺凭据 |
| T8 | `[FACT]` 已落地；**验收口径经拍板变更** | `src/remote/{mod,sync}.rs`；`src/cli.rs` 新增 `Gitremote` 变体（add / ls / rm / sync）；`scripts/verify-gitremote-e2e.ps1` 为 CI 硬门禁。`[FACT]` 原验收「存 PG」与 ADR-0022 §2.1 冲突，2026-10-03 经 Ulysses 拍板：**ADR-0022 不变，验收改为「存本地 + fast-forward sync 工作」**，存储为 `.gitgit/remotes.json`。`[FACT]` `RemoteStore` 刻意做成可替换接口，将来若真需要外部存储，换实现不触碰 CLI 层 |
| T9 | `[FACT]` 已落地（2026-10-05）；真实 provider 未验证 | 层 1 `src/ai/stream.rs`（纯 SSE 解码器 + `AiProvider::send_stream`，11 条无 socket 测试）；层 2 Tauri 命令 `ai_review_start`/`ai_review_cancel` + `src/routes/Review.svelte` + `src/lib/stores/review.ts`。验收「GUI 上能看到 token 逐字流」由 mock SSE server 实测达成（`["Hel","lo","wo","rld","!"]`）。i18n 146 键双目录一致；gm-desktop Rust 8→28、vitest 76 passed。Anthropic 显式拒绝而非静默降级。`[UNVERIFIED-FACT]` 无 `GITGIT_AI_API_KEY`，未对真实 provider 验证；`TauriEmitter` 端到端未验。 |
| T10 | `[FACT]` 已落地（2026-10-04 补齐装卸实测） | `[FACT]` 4 个 MSI 产出并实测装卸（per-user/per-machine × en-US/zh-CN，SHA-256 见 `docs/reports/2026-10-03-tauri-packaging/tauri-packaging.md`）：非提权会话安装 exit 0、程序可启动、卸载 exit 0 且痕迹全清。`[FACT]` 修掉「启动即崩」：`tauri.conf.json` 三段死 `plugins.*` 配置在加载期致命（`344bc87`）。`[TBD]` 未代码签名 → SmartScreen 告警。`[TBD]` 完整 `tauri build` 仍不在 CI 内。`[FACT]` 双变体写同一 bundle 输出路径（MSI 文件名不含 install scope），发布流水线前须解决覆盖。`[UNVERIFIED-FACT]` macOS `.dmg` / Linux `.appimage` / `.deb` 无任何证据（维护机为 Windows，Tauri 不支持交叉打包这三类）。 |
| T11 | `[FACT]` 已落地（2026-10-05） | `deploy/minio/docker-compose.yml` + `scripts/verify-minio-vault.ps1`；实测 versioning enabled / bucket private / 未签名 list 403。两条 `#[ignore]` minIO e2e 测试首次实跑，暴露 2 个真实缺陷（404 错误文档被当凭据返回；`get_at_version` 缺 key 返回 Err）并已修复，提交 `46b07de`。验收口径已更正：列举 bucket 需 SigV4 签名，诚实答案是 403 而非「返回列表」。`[FACT]` 产品未接 minIO（`build_vault()` 写死 `FileVault`）；MinIO 服务端为 AGPLv3，仅作 dev 夹具不随产品分发。 |

### 矛盾更正 A —— T5「PG 18.6 + sqlx」与现行代码冲突

任务表 T5 要求 PG 18.6 + sqlx + `migrations/20260826_v0_gui_keychain.sql`，与 `dev` 实际状态冲突：

- `[FACT]` `Cargo.toml` **无 `sqlx`、无任何 PostgreSQL driver**。文件头注释写明：
  `"This is intentionally a single crate, NOT a workspace. The previous 14-crate design is archived in docs_archive_rust_impl_2026_08_26/."`
- `[FACT]` `dev` 上**无 `migrations/` 目录**。仓内曾存在的 10 张表 migration（`0001_primitives` … `0010_view_snapshots`）属于 14-crate 设计，已随 commit `5007883`（single-crate MVP skeleton）删除。
- `[FACT]` `migrations/20260826_v0_gui_keychain.sql` **从未被提交** —— `git log --all --diff-filter=A -- '*keychain*'` 返回空。
- **结论**：T5 的 PG/sqlx 路线未落地，且当前代码库无 PG 依赖。凭证数据实际落在 Vault（`rust-s3` / `FileVault`）而非 PG 表。T8「存 PG」因此同样无落点。

**更正动作**：T5 行状态标 `[FACT]` 未落地 / 路线冲突；下文「当前批次（执行中）」中 `migrations/20260826_v0_gui_keychain.sql` + `sqlx migrate run` 一项标注为已被上述证据取代，不作为待办执行依据。

### 矛盾更正 B —— T3「Svelte 5」与「不做（out of scope）」列表

- `[FACT]` **Svelte 5 属实，但仅 desktop 侧**：`apps/gm-desktop/package.json` 为 `svelte ^5.1.0`。需明确区分 —— `apps/gm-console` 是 **React 18**（`react ^18.3.1` + `react-router-dom ^6.27.0`），**不是 Svelte**。T3 的「Svelte 5」只对 `apps/gm-desktop` 成立。
- `[FACT]` 「不做（out of scope）」把 **暗 / 亮主题切换** 与 **i18n** 列为 V1，但二者**已在盘上**：
  `apps/gm-desktop/src/lib/components/ThemeToggle.svelte`、`LocaleSwitcher.svelte`、
  `lib/stores/theme.ts`、`lib/stores/locale.ts`、`lib/i18n/zh-CN.ts` + `lib/i18n/en.ts`。
  依据 **ADR-0023 §2.4**，二者系自 `apps/desktop` 吸收进 `gm-desktop`。
- `[FACT]` **ADR-0023 §3.3**：commit `03bdd7b` 已执行 `git rm -r apps/desktop`。`apps/` 现有且仅有 `gm-console` + `gm-desktop`。

**更正动作**：将 i18n / 主题切换从「不做（out of scope）」移出，改记为已随 ADR-0023 落地；T3 行加注 gm-console 为 React。

## 关键路径

```
T1 (Tauri 脚手架) → T2 (axum embed) → T4 (仓库视图)
                                       ↘ T9 (AI 评审 UI)
T11 (minIO 部署) ─┐
T5 (PG/sqlx) ─→ T6 (MinioVault) → T7 (AI provider)
                                 → T8 (remote provider)
                          T10 (打包) ← 全部
```

**T11 必须在 T6 之前完成**（minIO bucket 就绪 → MinioVault 才有写入目标）。

## 决策日志

- 2026-08-30 15:42 JST: 拍板 Credential Vault 从 FileVault 改为 minIO S3-compatible bucket `gitgit-vault` + `MinioVault` 默认实现；FileVault 降级为 V1 fallback（per ADR-0021）
- 2026-08-26 17:06: 拍板 V0 范围 = GUI 骨架 + 两套 API Key
- 2026-08-26 17:00: 环境核实（Rust 1.98.0 / PG 18.6 / Git 2.41 / Node 22）
- 详见 ADR-0020 §2 + [ADR-0021](../adr/0021-v0-minio-credential-vault.md)

## 当前批次（执行中）

> 2026-10-02 更正（证据见「矛盾更正 A」）：本批次原列的
> `migrations/20260826_v0_gui_keychain.sql` + `sqlx migrate run` **已被证据取代**——
> `Cargo.toml` 无 sqlx / 无 PG driver，`dev` 上无 `migrations/` 目录，该文件从未被提交。
> 下列两项**不作为待办执行依据**，是否重启 PG 路线需重新拍板 `[PROPOSAL]`。

**批次 1：环境 + 脚手架（T1 + T5）**
- 安装 `tauri-cli@^2.0` — `[FACT]` desktop 侧 `tauri:dev` / `tauri:build` 脚本已在 `apps/gm-desktop/package.json`
- `pnpm create tauri-app` 初始化 — `[FACT]` 已落地（commit `2f8b9fc`，`apps/gm-desktop/src-tauri/`）
- 验证 Tauri dev 能起窗口 — `[FACT]` 窗口标题实为 `gitgit Desktop`（`tauri.conf.json` `productName`）
- ~~写 `migrations/20260826_v0_gui_keychain.sql` + 跑 `sqlx migrate run`~~ — `[FACT]` 未落地且路线冲突，见「矛盾更正 A」
- ~~写 PG 连接配置到 `.env`（PG 18.6 socket + user）~~ — `[FACT]` 当前代码库无 PG 依赖，无此配置落点

## 风险登记

| 风险 | 概率 | 影响 | 缓解 |
|---|---|---|---|
| Tauri 2 编译时间长（首次 5-15 分钟） | 高 | 中 | 先 release-debug profile 编译一次，再切回 dev |
| pnpm 不可用 | 中 | 低 | 退回 npm 装 Svelte 模板 |
| WebView2 runtime 缺失 | 低 | 高 | Win 11 默认有；缺则引导装 Edge WebView2 |
| PG 18.6 新版 sqlx 兼容 | 低 | 中 | sqlx 0.8 已声明支持 PG ≥ 11 |
| **minIO AGPL-3.0 商业风险** | 中 | 高 | 商业部署需 Enterprise License；V0 阶段 dev/OSS 用 AGPL-3.0 即可，发布前评估 license 切换 (Ceph RGW / SeaweedFS / Garage 等备选) |
| **minIO 部署失败 / 网络不可达** | 中 | 高 | docker-compose 部署前先 `docker --version` + `docker compose version` 验证；T11 含网络可达性验证 curl 步骤；失败则降级 FileVault |
| **minIO 备份缺失** | 中 | 高 | 启用 versioning + 90d lifecycle policy；V1+ 推进跨节点复制（minIO erasure coding 默认 4+2 抗单盘故障） |
| **minIO root user 凭证管理**（递归引用 vault 自身） | 中 | 中 | minIO root 凭证由 FileVault 启动时引导存（仅 dev 阶段）；V1 推进 KMS / Secret Manager；**禁止把 root 凭证明文写 .env 入仓** |
| 凭证 vault 误写权限 | 中 | 高 | `MinioVault` 默认走 bucket policy（V0 简化：单租户 single-user access key 即可）；FileVault fallback 仍 0600 权限 |

## 不做（out of scope）

> 2026-10-02 更正（证据见「矛盾更正 B」）：原「暗 / 亮主题切换」与「i18n」两条与盘上事实冲突，
> 已移出本节 —— 二者已随 **ADR-0023 §2.4** 自 `apps/desktop` 吸收进 `apps/gm-desktop`，
> 代码在盘（`ThemeToggle.svelte` / `LocaleSwitcher.svelte` / `i18n/{zh-CN,en}.ts`）。

- 右键菜单 TortoiseGit 风格（V1）
- 多 remote 冲突解决（V1）
- LLM 调度 remote（V1 末端）
- ~~暗 / 亮主题切换（V1）~~ → `[FACT]` 已落地于 `apps/gm-desktop`（ADR-0023 §2.4）
- ~~i18n（V1）~~ → `[FACT]` 已落地于 `apps/gm-desktop`（ADR-0023 §2.4）

## 进度跟踪

每完成一个任务，commit 一次，格式：
```
feat(v0/T<n>): <description>
```

合并到 `feature/v0-gui-and-keychain` 的 commit 序列将是：
T1 → T5 → T2 → T3 → T4 → T11 → T6 → T7 → T8 → T9 → T10

> **修订说明 (8/30 15:42 JST)**：T11 (minIO 部署) 是 T6 (MinioVault) 的强依赖；commit 序列相应在 T4 后插入 T11。原 commit 序列 (T1 → T5 → T2 → T3 → T4 → T6 → T7 → T8 → T9 → T10) 仅 10 个 commit，新增 T11 后总 11 个 commit。本修订日无任何 commit 已 push，序列调整仅在文档侧生效。