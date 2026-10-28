# V0 WBS — Tauri GUI + AI/Remote 集成

> **Plan ID**: V0 | **Branch**: `feature/v0-gui-and-keychain`
> **Owner**: Mavis (per DEC-008) | **Reviewer**: Ulysses (DDD Review)
> **Anchor**: gitgit MVP commit `1da5f2c` on `simplify/2026-08-26-mvp`
> **ADR**: [ADR-0020](../adr/0020-v0-gui-tauri-svelte.md)
> **Started**: 2026-08-26 17:06 JST

> ## 2026-10-24 核验补充（全量复核）
>
> **核验基线**：`dev` @ `2838541`（= `origin/dev`）。
> **核验日期**：2026-10-24。
> **核验方法**：状态只承认三类证据 —— ① `git log` 中的具体 commit SHA；② 磁盘上实际存在的文件；③ ADR 条款。
> 本轮对本文档的每一条状态断言逐条回查：`git log --all` 核 SHA 存在性与祖先关系、
> `git ls-files` / `Test-Path` 核路径存在、`Read` / `Select-String` 核代码当前形态、
> 并实跑 `pnpm test` 与 `pnpm test:coverage` 取当前覆盖率。
> `[FACT]` 下方每条新断言都附 commit SHA 或 `file:line`；无来源者一律降级为
> `[UNVERIFIED-FACT]` / `[TBD]`，不写成 `[FACT]`。
>
> **本轮复核的结论摘要**（细节见各任务行与「状态核验（2026-10-24）」）：
>
> - `[FACT]` **2026-10-05 补充块中「已被测试钉住、尚未修复」的五条产品缺陷已全部修复**，
>   修复 commit `c14c131`（2026-10-05 15:08 JST，*fix(gm-desktop): five defects the
>   route tests pinned as current behaviour*）。该 commit 同时补了第 6 条（not-found
>   标题错显 "Loading…"）。当前代码形态：`RepoDetail.svelte` 走 `cloneUrl` 命令而非
>   硬编码 `127.0.0.1:38080`；`stores/vault.ts` 版本列取 `versions.reduce(max(e.version))`
>   的真实最高版本号；`stores/vault.ts` 导出 `invalidateVaultVersions()` 且在
>   set/restore/rotate/delete 四处调用；`Home.svelte` 用 `refreshesInFlight` 计数器
>   而非单个 `refreshing` 布尔。
> - `[FACT]` **T10 除签名外的三条「尚未」全部过时**：CI 现在跑真实 release `tauri build`；
>   MSI 两个 install scope 已各自产出 scope 限定文件名；`.dmg` / `.AppImage` / `.deb`
>   三个平台 CI 全部构建并校验，且许可证交付已逐平台测量。
>   （AppImage 的许可证机制与 AGPL 引用更正见下方「追补更正 1」；§6(d) 新增项见「追补更正 2」。）
> - `[FACT]` **T10「4 个包均未代码签名」需拆开说**：签名链路已存在且在发布路径强制
>   （`release.yml` 用 `-Required`，缺证书直接红）；但仓库未采购证书，所以
>   **今天产出的 MSI 仍然没有 Authenticode 签名**，`.dmg` 只有 ad-hoc 签名、无公证。
> - `[FACT]` **T9「`TauriEmitter` 未端到端验证」已关闭**：README.md 已记录 2026-10-05
>   对本地 SSE 端点 `127.0.0.1:38999` 实跑 `pnpm tauri:dev`，7 个 token 逐个穿过 Tauri
>   IPC 抵达 webview 并按序拼接（commit `9f93e62`）。仍然未验证的是**真实云端 provider**
>   —— 仓内无 `GITGIT_AI_API_KEY`，这一条保持开放。
> - `[FACT]` **T2 的验收从未被复现，且至今仍未复现**，但原因已从「未复现」变成
>   「验收写错了」：`<server>/info/refs` 这个路径在代码里**不存在**（smart-HTTP 路由是
>   `/repos/<name>.git/info/refs`，见 `src/server/http.rs:88`），桌面端前端也从不 fetch
>   它（Svelte 侧只走 Tauri 命令与 `/api`）。等价能力**已**被
>   `scripts/smoke.ps1` 在 CLI 形态上真实跑通并实测。
> - `[FACT]` **T4「HTTP 层未新增端点」成立**：`src/server/api.rs:243-266` 的路由表里
>   没有 `/api/repos/:name/status` 也没有 `/diff`。**T11「产品未使用 minIO」在
>   2026-10-25 已不成立**：M2a 起 `build_vault()` 由 `GITGIT_VAULT_BACKEND=file|s3`
>   （或 `--vault-backend`）选择，`s3` 分支构建 `S3Vault`（原 `MinioVault`）。
> - `[FACT]` 本文档另发现 7 处此前未被列出的失实/过期陈述，见「本轮新增更正」一节。
>
> ### 追补更正（2026-10-24 05:13 PT，PR #45 证据到达后）
>
> 本节两条更正覆盖上文「T10 除签名外的三条「尚未」全部过时」一句，以及「状态核验」T10 行中
> 与 AppImage 许可证相关的部分。**上文其余复核结论不变。**
>
> **更正 1 —— AppImage 确有许可证机制；且「AGPL-3.0 §4(a)」这个引用本身是错的。**
>
> `[FACT]` **机制存在。** `bundle.linux.appimage.files` 是 AppImage bundler 读取的第三个键：
> `tauri-utils` 的 `AppImageConfig` 声明 `files: HashMap<PathBuf, PathBuf>`（注释「The files to
> include in the Appimage Binary」，上游 tag `tauri-cli-v2.12.1`，正是 `apps/gm-desktop/pnpm-lock.yaml`
> 锁定的 `@tauri-apps/cli` 版本 `2.12.1`）；`crates/tauri-bundler/src/bundle/linux/appimage/linuxdeploy.rs`
> 调 `fs_utils::copy_custom_files(&settings.appimage().files, &data_dir)`，随后把 `data_dir/usr/`
> 复制进 AppDir；`crates/tauri-bundler/src/utils/fs_utils.rs` 的 `copy_custom_files` 以 map 的
> **键为目标路径**、**值为源路径**，并创建父目录。故 `usr/` 前缀的目标即可把 License 放进镜像内。
> `[FACT]` 缺口由**配置**闭合，不需重打包，因此「产物仍能运行」这一风险不存在。
> `[FACT]` 落地改动为 **PR #45**（`fix/licence-content-not-name`），commit `88a6a0a`
> （*fix(bundle): the AppImage carries the licence, and the citation names a clause that exists*）：
> 它把 AppImage 的许可证检查由「打印诊断」改为「断言」，并新增按**内容**（而非文件名）判定的用例。
> `[FACT]` **CI 已证明许可证真的落进了镜像。** PR #45 的 run `40092589002`（job `appimage (unsigned)`）
> 的日志为：
>
> ```
> bytes 8-10  : 414902 (expect 414902 for an AppImage type-2 runtime)
> ok  licence: usr/share/doc/gitgit-desktop/LICENSE
> ```
>
> 同一段日志还证实了字节切片修复：此前 `cut -c9-14` 读的是偏移 4-6（EI_CLASS/EI_DATA/EI_VERSION），
> 打印出 `020101` 却标成 "bytes 8-10"；修正后打印的是 type-2 runtime 真正的 magic `414902`。
>
> `[FACT]` 镜像里另有 113 个依赖库的 copyright 文件，被逐个列为「not the product's licence」。
> 它们的**存在**正是当初按名字判定报错文件的原因，所以列出而不隐藏。
>
> `[FACT]` **引用也要改。** 「AGPL-3.0 §4(a)」指向一个不存在的子节：AGPL-3.0 §4 **没有字母子节**
> （本仓 `LICENSE:183-195` 可查）。「随 Program 一并给每位接收者一份 License 副本」是 §4 首段的
> 收尾句（`LICENSE:190-191`，「and give all recipients a copy of this License along with the
> Program」）；目标码接收方由 §6 覆盖（§6 把 §4 / §5 适用于目标码），而 §6 **才有**字母子节。
> `[FACT]` PR #45 修正了仓库内每一处该引用，并新增两个用例：读 `LICENSE` 原文、按标题定位
> §4 与 §6，断言义务落在 §4、§4 无字母子节而 §6 有。
> `[FACT]` **本文档统一写「§4」，不再写「§4(a)」** —— 理由如上，一处说明即足。
>
> **更正 2 —— 新增一项此前无人记录的合规项：AGPL-3.0 §6(d)。**
>
> `[FACT]` §6(d) 规定对象码经「指定地点」交付时的义务，本仓 `LICENSE:263-275` 原文：
> 「Convey the object code by offering access from a designated place (gratis or for a charge),
> and offer equivalent access to the Corresponding Source in the same way through the same place
> at no further charge. ... provided you maintain clear directions next to the object code saying
> where to find the Corresponding Source.」
> `[FACT]` 本产品的分发方式正是「指定地点」—— GitHub Releases。
> `[FACT]` 许可证**确已交付**（deb / msi / dmg / appimage；PR #41 已合并，appimage 由 PR #45 补上）。
> `[FACT]` **这两点已于 2026-10-24 检查完毕，事实写回如下。**
>
> `[FACT]` **仓库是公开仓库**（`gh repo view UlyssesLeoLee/GitGit` → `visibility: PUBLIC`），
> 所以「指定地点」就是该仓库的发布页
> `https://github.com/UlyssesLeoLee/GitGit/releases`。
>
> `[FACT]` **发布页此前没有这些内容。** 草稿 release `404746754`（tag `v0.1.0`，2026-10-06）的
> 正文只有「Automated draft for `v0.1.0`」、一段签名说明和一段「这是草稿、发布由人决定」，
> **没有** Corresponding Source 段落、**没有**提到许可证、**没有**给出源代码位置；其 7 个安装包
> 资产中也没有源码包。tag `v0.1.0` 的源码归档本身是可达的
> （`https://codeload.github.com/UlyssesLeoLee/GitGit/tar.gz/refs/tags/v0.1.0` → HEAD 200）。
>
> `[FACT]` **本轮的改动：** `release.yml` 的 `draft-release` 正文新增 `## Corresponding Source`
> 段落，写明**已解析的 TAG**（`needs.verify-tag.outputs.tag`，不是 `github.ref_name`）、
> **许可证**（AGPL-3.0 及其文本所在处）、**源代码提供位置**（同一发布页、同一方式、
> at no further charge），并写明 `.msi` / `.dmg` 各自带什么、不带什么。
>
> `[FACT]` **对象码旁的「明确指引」：** 新增仓库内**提交**文件 `CORRESPONDING-SOURCE.md`
> （根目录，与 `LICENSE` 同级），经既有 `bundle.linux.deb.files` /
> `bundle.linux.appimage.files` 打进 `.deb` 与 `.AppImage`，落点
> `usr/share/doc/gitgit-desktop/CORRESPONDING-SOURCE.md`，与许可证副本同目录。
> 它是提交进仓的固定文件，不是构建期生成的 —— 生成出来的「指引」是仓库里并不存在的内容的
> 转述，会与它所指向的源码漂移。
> `verify-bundle.sh` 按**内容**判定（三个短语：标题句 `Corresponding Source for this build`、
> `AGPL-3.0`、指定地点 `https://github.com/UlyssesLeoLee/GitGit/releases`），
> `.deb` / `.AppImage` 分支是**断言**，缺失即红；`dmg` 分支只打印诊断。
>
> `[FACT]` **两个产物仍然只有许可证、没有指引文件，原因是机制限制，不是遗漏：**
> - `.msi`：`wix/main.wxs` 的 `{{license}}` 槽位装的是许可证 RTF（`LICENSE.rtf`，与可执行文件
>   同组件），该槽位**已被占用**；另一个槽位 `bundle.resources` 经 run `37644440611` 实测会破坏
>   MSI 构建（`WixTools314/light.exe`），因此现有门禁把 `bundle.resources` 钉死为 `[]`。
>   本轮**没有**改动 WiX 模板，也没有引入未经实测的新机制。
> - `.dmg`：`bundle.licenseFile` 由 create-dmg 以 `--eula` 接收，落为 **UDIF EULA 资源**
>   （TEXT/styl/STR#），挂载时呈现；镜像内**没有**可挂指引的文件，dmg bundler 也不读
>   `bundle.resources`，配置里没有能放进去的键。
>
> `[PROPOSAL]` **合法性判断仍然开放。** 以上记录的是**事实**（仓库公开性、指定地点、各产物
> 实际携带的内容），**不构成**「是否符合 §6(d)」的结论 —— 该项属许可证理解问题，应由负责法律
> 判断的人裁定，**不应靠阅读文本来下结论**；本节仍不作「是否违规」的判断。
> 若要为 `.msi` / `.dmg` 也补上指引文件，需要一个**尚未验证可行**的机制
> （WiX fragment 路径 `bundle.windows.wix.fragmentPaths`），本轮不引入。

> ## 2026-10-05 核验补充（前端测试与启动路径）
>
> > **本块已被上方「2026-10-24 核验补充」部分取代，保留为历史记录。**
> > 其中「已被测试钉住、尚未修复的产品缺陷」一节（末尾五条）**已于 2026-10-05 被
> > commit `c14c131` 全部修复**，不再是当前状态 —— 详见上方 2026-10-24 块第一条。
> > 其余关于 `App.svelte` 启动崩溃、`tailwind.config.js` 深色选择器、覆盖率门禁
> > 三段描述**仍然成立**（逐条复核：`apps/gm-desktop/tailwind.config.js:4` 的
> > `darkMode` 仍含 `[data-theme="auto"]`，修正由 `stores/theme.ts` 写入解析后的实际
> > 调色板来做，而非改 `tailwind.config.js`）。
>
> `[FACT]` **桌面端此前根本无法启动。** `App.svelte` 曾在 `<svelte:head>` 里放
> `<html lang=... data-theme=...>`。Svelte 把 head 内容编译成 `$.from_html`，
> 即赋给 `<template>` 的 `innerHTML`，而 HTML 解析器在 "in template" 插入模式下
> **忽略嵌套的 `<html>` 开始标签**。实测：`t.innerHTML = '<html></html>'` 后
> `childNodes.length === 0`、`firstChild === null`，Svelte 随即对 `null` 调
> `.cloneNode()` → `TypeError: Cannot read properties of null (reading 'cloneNode')`，
> 根组件挂载即抛。WebView2 用同一套规范解析器，所以不是 jsdom 的问题。
>
> `[FACT]` 之所以一直没人发现：MSI 能打包、能静默安装、报 exit 0，但**没有任何人启动过装出来的程序**。
> 而 `App.svelte` 的覆盖率一直是 0%，"完全起不来"因此在测试里完全不可见。
> 修法见 commit `0e354af`；`lang` 此前无人写入，现由 `stores/locale` 补上。
>
> `[FACT]` 另修：`tailwind.config.js` 的 `darkMode` 把 `[data-theme="auto"]`
> 也列为深色选择器，而 `applyToDocument` 写的是原始模式串，
> 导致 **auto 模式 + 亮色系统仍渲染深色**。现改为写入解析后的实际调色板。
>
> `[FACT]` **覆盖率门禁第一次成为真门禁。** `vite.config.ts` 声明的
> lines/statements 70、functions 60、branches 55 长期无人求值（CI 跑的是
> 不带 `--coverage` 的 `pnpm test`，而 `test:coverage` 当时 exit 非零）。
> 补齐 routes / stores / api 测试后 `All files` 由 42.55% 升至 94.78% lines，
> `test:coverage` 首次 exit 0，workflow 遂新增独立的 `Coverage gate` step
> （不与单测 step 合并，避免跑两遍；且守卫 fail-closed，会先剥 ANSI 再校验
> 确有测试通过）。**阈值未作任何下调。**
>
> `[FACT]` 已被测试钉住、尚未修复的产品缺陷（均有独立用例记录现状）：
> `RepoDetail.svelte:75` 双花括号导致渲染出多余的 `}`；
> `RepoDetail.svelte:41` clone URL 硬编码 `127.0.0.1:38080` 而未走 `clone_url`；
> `stores/vault.ts:35` 版本列取的是键的列表下标而非最新版本号；
> `Vault.svelte:79,86` restore/rotate 后经 `vaultVersions` 复读的仍是缓存，列表不刷新；
> `Home.svelte:29-36` 多个日志读取共用一个 `refreshing` 标志，较早那次调用的
> `finally` 会清掉较新那次的锁。
>
> > **⚠ 本段已被 commit `c14c131`（2026-10-05 15:08 JST）取代 —— 这五条缺陷
> > 全部已修复，不再是「尚未修复」。** 保留原文是因为它记录了当时的状态，而过时记述
> > 比没有记述更糟。当前形态与逐条对应见上方「2026-10-24 核验补充」块。
> > 该 commit 还修了本段未列出的第 6 条：`RepoDetail.svelte` 的 not-found 标题错显
> > "Loading…"（`{#if detail}{detail.name}{:else}{common.loading}{/if}`）。

## 范围（in scope）

V0 = Tauri 2 GUI 骨架 + 两套 API Key（AI provider + Git remote），
**在 gitgit MVP 之上叠加**，单 Rust crate `gitgit` + Tauri 2 desktop app。

## 任务分解（11 个工作块，约 8-10 个工作日）

| # | 任务 | 工时 | 依赖 | 验收 | 状态（2026-10-02 核验） |
|---|---|---|---|---|---|
| 1 | **Tauri 2 脚手架** — `pnpm create tauri-app` 加 src-tauri/，能 `cargo tauri dev` 出窗口 | 0.5 d | — | 窗口出现，标题 "gitgit" | `[FACT]` 脚手架已落地；`tauri.conf.json` 实际 `productName = "gitgit Desktop"`，与验收写的 "gitgit" 不一致 |
| 2 | **axum 同进程 embed** — gitgit 现有 `axum::serve` 跑进 Tauri `setup()`，端口 38080 | 0.5 d | 1 | Svelte fetch `127.0.0.1:38080/info/refs` 拿到 200 | `[FACT]` 内嵌已落地（`apps/gm-desktop/src-tauri/src/lib.rs:242` `build_router(state)` + `state.rs:37` `DEFAULT_BIND = "127.0.0.1:38080"`）。`[FACT]` **验收至今未复现，且验收本身写错了**：`<server>/info/refs` 这条路径在代码里不存在 —— smart-HTTP 路由是 `/repos/<name>.git/info/refs`（`src/server/http.rs:88` `.route("/repos/*key", ...)`），桌面端 Svelte 侧也从不 fetch 它（`src/lib/api/http.ts` 只走 `/api`）。`[FACT]` 等价能力已实测：`scripts/smoke.ps1:190-197` 以 `/repos/<name>.git/info/refs?service=git-upload-pack` 为就绪探针，`docs/reports/2026-10-21-regression-st/REGRESSION-TEST-REPORT.md` 记录该里程碑 pass 且随后真实 `git clone` 成功。→ `[TBD]` 保留：**Tauri 内嵌形态下的同一条路径未被 Svelte 端到端请求过** |
| 3 | **Svelte 5 基础** — 路由、布局、主题（系统默认） | 1 d | 1 | 窗口能切 3 个 page（仓库列表 / 详情 / 设置） | `[FACT]` desktop 侧已落地（`apps/gm-desktop/package.json` `svelte ^5.1.0`）；`apps/gm-desktop/src/routes/` 现有 **8** 个 route（Home / Repos / RepoDetail / Settings / Graph / Vault / Review / NotFound）—— 比本文档 2026-10-02 核验时的 7 个多出 `Review.svelte`（commit `cc41e00`）。`[FACT]` **路由库已不是 `svelte-spa-router`**：commit `6963340`（2026-10-02）以 `apps/gm-desktop/src/lib/router.ts` 自研 hash 路由替换了它（`svelte-spa-router@4.0.2` 在 Svelte 5 runes 模式下调用 `afterUpdate`，`vite build` 直接失败）。`[FACT]` **注意 `apps/gm-console` 是 React 18，非 Svelte**（`react ^18.3.1`；`react-router-dom` 已由 `^6.27.0` 升到 `^7.18.4`，commit `946a452`）；主题/i18n 现状见下方矛盾更正 |
| 4 | **仓库视图** — 列表 + status + log(最近 5) + diff 视图 | 1.5 d | 2, 3 | 选个 repo 看到 working tree status + commit graph | `[FACT]` **部分落地**：列表 + refs + commit graph 在盘（`routes/Repos.svelte` / `routes/RepoDetail.svelte`）；working tree status 视图与 diff 视图未找到 → 未落地 |
| 5 | **PG 18.6 + sqlx** — migration 5 张表 + `migrations/20260826_v0_gui_keychain.sql` | 0.5 d | — | `sqlx migrate run` 通过 | `[FACT]` **未落地，且该路线与现行代码冲突** — `Cargo.toml` 无 sqlx / 无 PG driver（2026-10-24 复核：`Cargo.toml` 中 `sqlx` 匹配数为 0），`dev` 上无 `migrations/` 目录。`[FACT]` **「keychain migration 从未提交」需更正**：commit `a34d812`（2026-10-02）把该脚本归档到了 `docs_archive_rust_impl_2026_08_26/migrations/20260826120000_v0_gui_keychain.sql`（文件名带 sqlx 时间戳前缀，与本文档引用的 `migrations/20260826_v0_gui_keychain.sql` 本就不同名）。**路线未落地的结论不变**，变的是「从未被提交」这句 —— 它被归档进仓了，只是从未作为活代码提交。详见下方「矛盾更正 A」 |
| 6 | **Credential Vault (S3)** — `trait Vault` + `S3Vault` 实现 (S3-compatible 对象存储) + `FileVault` V1 降级 fallback | 1 d | 5, 11 | `gitai key set openai <key>` PUT 到 `gitgit-vault` bucket，读回能验证 | `[FACT]` 已落地（`src/server/vault.rs` + `vault_versioned.rs`，ADR-0021/0022）。`[FACT]` **`rust-s3` 版本已不是 0.37** —— `Cargo.toml` 现为 `rust-s3 = { version = "0.38", ... }`（commit `b1025a3`）。**但 CLI 真实形态是 `gitgit key set`，非验收写的 `gitai key set`**（见 T7）。`[FACT]` **2026-10-25 起类型名是 `S3Vault`**（原 `MinioVault`）：它是 S3 客户端，不该以某个服务端命名；服务端已换成本仓自建 |
| 7 | **AI provider 注册表** — 5 个 provider + `gitai` 子命令（commit/explain/review） | 2 d | 6, 5 | `gitai commit --from-diff` 真打通 OpenAI，输出 commit message | `[FACT]` **已落地**（`src/ai/`，`gitai commit` / `explain` / `review` / `providers`）。`[FACT]` 验收「真打通 OpenAI」**未验证** — 需要真实 `GITGIT_AI_API_KEY`，本仓无凭据；provider 传输层由 mock HTTP server 覆盖。`[FACT]` 计数口径：**5 个具名 preset（openai / deepseek / ollama / vllm / anthropic）跑在 2 种 wire 协议上**，归档设计 §5.5 明确 Ollama 复用 OpenAI 兼容客户端，故未实现 5 份适配器（详见 `src/ai/registry.rs` 模块注释）。`[FACT]` **「`Stream()` 未实现」需更正**：归档 §5.4 的 `Stream()` 并未标 V1（只有 `EstimateCost()` 标了 `(V1)`），且它**已实现** —— 形态从 callback 改为 channel：`AiProvider::send_stream`（`src/ai/provider.rs:207`，OpenAI 实现在 `src/ai/openai.rs:223`）。`[FACT]` **仍未实现的是 `EstimateCost()`**，且是刻意不 stub（`src/ai/provider.rs:13-16`） |
| 8 | **remote provider 注册表** — `gitremote` 子命令（add/ls/rm/sync） | 1.5 d | 6, 5 | ~~`gitremote add gitee <url>` 存 PG~~ → `[PROPOSAL]`（Ulysses 2026-10-03 拍板）**验收改为**：`gitremote add <name> <url>` 存本地 + fast-forward sync 工作 | `[FACT]` **已落地**（`src/remote/`，`gitremote add/ls/rm/sync`）。`[FACT]` **验收口径已变更**：原「存 PG」与 ADR-0022 §2.1「不引入 sqlx / PG 到 gitgit」直接冲突，且 ADR-0022 已把「对接 AssetsLake schema + sqlx」列为**已否决方案**（风险 #2）。经 Ulysses 拍板，**保持 ADR-0022 不变**，存储改为 `.gitgit/remotes.json`（与 `FileVault` 根并列，非其内部）。`[FACT]` sync 只在 fast-forward 时推送，拒绝时返回 `refused (not a fast-forward)` 而非泛化失败 |
| 9 | **AI 评审 UI** — push 前弹窗 + streaming token 显示 | 1 d | 4, 7 | GUI 上能看到 token 逐字流 | `[FACT]` **已落地**（2026-10-05）— 验收口径「GUI 上能看到 token 逐字流」已由 mock SSE server 实测达成：事件序列 `model,token,…,done`，token 为 `["Hel","lo","wo","rld","!"]`。分两层：层 1 `src/ai/stream.rs`（纯 SSE 解码器 + `AiProvider::send_stream`，11 条无 socket 测试）；层 2 Tauri 命令 `ai_review_start` / `ai_review_cancel`（可取消、不阻塞 UI、密钥不入日志/错误/前端）+ `src/routes/Review.svelte` + `src/lib/stores/review.ts`。`[FACT]` i18n 两份目录**各 194 键**、键集与顺序完全一致（2026-10-24 实测；本文档 2026-10-05 记的 146 键、commit `c14c131` 记的 185 键均已被后续 commit 推过）。`[FACT]` gm-desktop Rust 测试 8 → 28（`src-tauri` 内 `#[test]`/`#[tokio::test]` 属性 2026-10-24 计 30 处），vitest 76 → **508 passed / 35 files**（2026-10-24 实跑 `pnpm test`，全绿），`svelte-check` 0 error。`[FACT]` Anthropic 走的是**显式拒绝**（`supports_streaming() == false` + 返回错误），不是静默降级——把非流式调用伪装成流式会让 UI 卡住且看起来像挂起。`[UNVERIFIED-FACT]` 未对**真实云端 provider** 验证：仓内无 `GITGIT_AI_API_KEY`。`[FACT]` **`TauriEmitter` 适配层已端到端验证，本文档旧记述过时**：2026-10-05 对本地 SSE 端点 `127.0.0.1:38999`（逐帧吐 7 个 token）实跑 `pnpm tauri:dev`，7 个 token 逐个穿过 Tauri IPC 抵达 webview 并按序拼接，首帧 `model` 值被页面显示，mock 日志确认 `Authorization: Bearer` 未泄漏进事件（README.md 记述，commit `9f93e62`）。`[FACT]` 层 2 的测试抓到并修掉一个安全缺陷：401 响应体回显 `Authorization` 头会把密钥带进前端错误载荷，现已在命令层统一脱敏 |
| 10 | **MSI 打包 + 烟测 + ADR 收尾** | 0.5 d | 全部 | `cargo tauri build` 出 .msi，scripts/smoke.ps1 仍过 | `[FACT]` **已落地**（2026-10-04）— 4 个 MSI 全部产出并实测装卸：per-user / per-machine × en-US / zh-CN（6,742,016 B 与 6,737,920 B，SHA-256 已记录于 `docs/reports/2026-10-03-tauri-packaging/tauri-packaging.md`）。非提权会话（Medium integrity, `IsElevated=False`）实测：安装 exit 0 → `%LOCALAPPDATA%\Programs\gitgit Desktop\gm-desktop.exe` + 开始菜单快捷方式 + HKCU 6 个值；安装后二进制可启动（窗口标题 `gitgit Desktop`，40 线程，1 个 WebView2 子进程）；卸载 exit 0，目录/注册表/开始菜单全清。`[FACT]` **2026-10-25 起该观察进入 CI**：`.github/workflows/gm-desktop.yml` 新增 `msi-launch` job——装好**打包后的** MSI、启动它、断言弹出**顶层窗口**（实测窗口标题 `gitgit Desktop`），随后停掉进程并卸载；它与 `msi-install` 分开，后者仍刻意不启动应用（Linux 容器里 webkit 应用起不来，那类失败与打包无关）。`[FACT]` 打包过程修掉一个致命缺陷：应用此前**启动即崩**——`tauri.conf.json` 里三段死 `plugins.*` 配置（log / notification / updater）在加载期即致命（后两者无配置类型；updater 的 `pubkey` 必填且无默认）。提交 `344bc87`。`[FACT]` **「完整 `tauri build` 不在 CI 内」已过时** —— CI 现在跑真实 release 打包：`.github/workflows/gm-desktop.yml` 的 `msi` job（windows-latest）跑 `build-msi.ps1 -Scope both`，该脚本执行 `pnpm tauri build --bundles msi --ci`；`.github/workflows/gm-desktop-bundle.yml` 另跑 `dmg`（macos-latest）、`deb` / `appimage`（ubuntu-latest）三个 job；`.github/workflows/release.yml` 重复全部四个 target。`[FACT]` **「两个变体写同一 bundle 输出路径」已过时** —— bundler 命名本身仍不含 install scope（`{productName}_{version}_{arch}_{lang}.msi`），但 `apps/gm-desktop/scripts/build-msi.ps1:380-387` 把每个产物移出共享目录并重命名为 `{product}_{version}_{arch}_{scope}_{lang}.msi`，且脚本自身断言两个 scope 的哈希互不相同（`:424-429`）；CI 的 `msi` job 分别上传 `*_perMachine_*.msi` 与 `*_perUser_*.msi`（`if-no-files-found: error`），覆盖问题在工具层已解。`[FACT]` **签名状态需分两层说**：签名**链路**已存在且在发布路径强制 —— `apps/gm-desktop/scripts/sign-msi.ps1` 用 PFX + RFC 3161 时间戳签名并重算 `manifest.json` 哈希，`release.yml` 的 `msi` job 以 `-Required` 调用（缺证书直接红），`gm-desktop.yml` 的日常 `msi` job 不带 `-Required`（fork PR 无 secrets）。`[TBD]` **但仓库未采购证书，`MSI_SIGN_PFX_BASE64` / `MSI_SIGN_PFX_PASSWORD` 未配置，因此今天产出的 MSI 仍无 Authenticode 签名**，打 `v*` tag 会在发布路径的 `msi` job 变红；`.dmg` 只有 ad-hoc 签名（`APPLE_SIGNING_IDENTITY: '-'`）、无 notarization；`.deb` / `.appimage` 无签名概念 |
| 11 | **对象存储部署前置** — 起一个 S3 兼容服务端 + `gitgit-vault` bucket provisioning + 网络可达性验证 + TLS 关闭（dev 阶段） | 0.5 d | — | `mc alias set local http://localhost:9000 minio minio123` + `mc mb local/gitgit-vault` 成功，curl `http://localhost:9000/gitgit-vault?list` 返回 bucket 列表 | `[FACT]` **已落地（2026-10-05），并于 2026-10-25 换掉载体** — 最初的夹具是 `deploy/minio/docker-compose.yml` + `scripts/verify-minio-vault.ps1`（dev/test 专用，端口仅发布到 127.0.0.1）。实测：minIO `RELEASE.2025-09-07T16-13-09Z` 启动后 `local/gitgit-vault versioning is enabled`（versioning 是承重项：`VaultVersionSummary` 依赖 `x-amz-version-id`，而服务端只对已开 versioning 的 bucket 返回该头）、bucket 为 `private`、未签名 `GET /gitgit-vault?list` 返回 **403**。**验收口径更正**：原写的「curl 返回 bucket 列表」在不签名时不可能成立（列举 bucket 需 SigV4 签名），诚实形式是 403——证明 bucket 存在且非公开；返回 200 反而是缺陷。`[FACT]` 让两条 `#[ignore]` e2e 测试**首次真跑**，暴露并修复 2 个真实缺陷：`rust-s3` 0.37 `get_object` 不检查状态码、把 404 的 XML 错误文档当凭据返回（`is_not_found` 因此从未命中）；`get_at_version` 对从未写过的 key 返回 `Err` 而非契约要求的 `Ok(None)`。`[FACT]` **2026-10-25 minIO 退场**：服务端换成本仓自建（`src/objectstore/` + `gitgit-objectstore`），夹具换成 `deploy/objectstore/`（**无 `mc`、无 docker-compose**——服务端自己的 `--bootstrap-bucket` 就是 `mc mb` + `mc version enable` 的替代），验证脚本换成 `scripts/verify-s3-vault.ps1`。`[FACT]` **产品本身现在真的走 S3 了**（M2a）：`GITGIT_VAULT_BACKEND=s3`（或 `--vault-backend s3`）选中 `S3Vault`，六项设置读 `GITGIT_S3_*`；两条 vault e2e 测试摘掉 `#[ignore]`，自己在进程内起服务端（端口 0 + 显式开版本化的桶），于是 S3 那条路被常规测试套件与 CI 持续覆盖。`[FACT]` 服务端密钥**只走环境变量** `GITGIT_OBJECTSTORE_SECRET_KEY`，没有 `--secret-key` 旗标 |

**总**：约 10.5 天（1.5 周强）

## 状态核验（2026-10-24 复核；首次核验 2026-10-02）

**首次核验基线**：`dev` @ `726ca3a`（2026-09-27）。
**本次复核基线**：`dev` @ `2838541`（2026-10-24）。
**核验方法**：状态只承认三类证据 —— ① `git log` 中的具体 commit SHA；② 磁盘上实际存在的文件；③ ADR 条款。

> **重要**：「验收」列描述的是**意图**，不是「已达成」的证据。本节状态不依据验收列反推。
> 证据分级标签按 `README.md` 约定：`[FACT]` / `[UNVERIFIED-FACT]` / `[INFERENCE]` / `[PROPOSAL]` / `[TBD]`。
>
> `[FACT]` **本表已于 2026-10-24 逐行复核**（方法见上方「2026-10-24 核验补充」块）。
> 2026-10-02 的原始判定可从本文档的 git 历史回放；本表是**当前状态**。

| # | 状态 | 证据 |
|---|---|---|
| T1 | `[FACT]` 已落地 | commit `2f8b9fc`；`apps/gm-desktop/src-tauri/`；`apps/gm-desktop/package.json` `@tauri-apps/api ^2.1.1`；`tauri.conf.json` `productName = "gitgit Desktop"` |
| T2 | `[FACT]` 已落地；**验收至今未复现（验收本身写错）** | `apps/gm-desktop/src-tauri/src/lib.rs:242` `build_router(state)` + `axum::serve`；`apps/gm-desktop/src-tauri/src/state.rs:37` `DEFAULT_BIND = "127.0.0.1:38080"`。`[FACT]` 验收写的 `<server>/info/refs` **在代码里不存在**：smart-HTTP 路由是 `/repos/<name>.git/info/refs`（`src/server/http.rs:88`），且桌面端前端从不 fetch 它。`[FACT]` 等价能力已实测：`scripts/smoke.ps1:190-197` 以该路径为就绪探针，`docs/reports/2026-10-21-regression-st/REGRESSION-TEST-REPORT.md` 记录里程碑 pass + 真实 `git clone` 成功。→ `[TBD]` **Tauri 内嵌形态下的同一条路径未被端到端请求过** |
| T3 | `[FACT]` 已落地 | `apps/gm-desktop/package.json` `svelte ^5.1.0`；`apps/gm-desktop/src/routes/` **8** 个 route（Home / Repos / RepoDetail / Settings / Graph / Vault / Review / NotFound）。`[FACT]` 路由库**不是** `svelte-spa-router` —— commit `6963340`（2026-10-02）以自研 `apps/gm-desktop/src/lib/router.ts` 替换之（`svelte-spa-router@4.0.2` 在 Svelte 5 runes 模式调用 `afterUpdate`，`vite build` 失败）。`[FACT]` `apps/gm-console` 是 React 18（`react ^18.3.1`；`react-router-dom` 已升 `^7.18.4`，commit `946a452`） |
| T4 | `[FACT]` 已落地；**HTTP 层仍无端点** | 列表 `routes/Repos.svelte`；refs + commit graph `routes/RepoDetail.svelte`；working tree status 视图 + diff 视图：`src/repo/status.rs`（porcelain v1 解析 + 三种 diff 目标 + 未跟踪文件合成 diff，25 个测试，其中 7 个跑在测试自建的真实 git 仓库上）、`apps/gm-desktop/src-tauri/src/commands/repos.rs` 的 `repo_status` / `repo_diff`、`src/lib/components/RepoWorktree.svelte`（由 `RepoDetail` 挂载）、`src/lib/stores/worktree.ts`。`[FACT]` 裸仓库无工作区，`git status` 退出 128，由 `is_bare_repo` 判定并返回 typed `NotAWorkTree`，UI 给出可执行提示而非 git 原始 stderr。`[FACT]` **工作区目录由用户设置（commit `5088f7d`）**：`src/lib/stores/worktreeRoot.ts` 持久化于 `localStorage` 键 `gm-desktop.worktreeRoot`（校验读取，损坏值回落为「未设置」），`src/lib/components/WorktreeRootPicker.svelte` 提供目录选择 / 清除 / 显示当前值，挂在 `routes/Settings.svelte` 与 `NotAWorkTree` 面板内；`RepoWorktree.svelte` 的 `effectiveRoot = root ?? $worktreeRoot` 供 `repo_status` / `repo_diff`（25 个 vitest 用例，含「持久化值到达命令」的断言）。取消选择不改动原值。`[TBD]` **HTTP 层仍未新增端点** —— `src/server/api.rs:243-266` 的路由表里没有 `/api/repos/:name/status`，也没有 `/diff`（只有 vault 的 `/api/vault/keys/:key/diff`）。`[TBD]` 目录选择器未在真实 Tauri 运行时中实跑，需打包后的应用才能验证 |
| T5 | `[FACT]` 未落地，路线与现行代码冲突 | 见下方「矛盾更正 A」（2026-10-24 已复核该节的三条证据） |
| T6 | `[FACT]` 已落地 | `src/server/vault.rs` + `src/server/vault_versioned.rs`；ADR-0021 / ADR-0022；commits `bdb328b`（VersionedVault）、`0728213`（Vault CLI + AppState）、`eb8eddf`（sidecar restore）、`d3d184e`（versionId per ADR-0022）。`[FACT]` `Cargo.toml` 的 `rust-s3` 现为 **`0.38`**（commit `b1025a3`），本文档旧记的 0.37 已过期 |
| T7 | `[FACT]` 已落地（传输层有测试，真实 OpenAI 未验证） | `src/ai/{mod,provider,registry,openai,anthropic,sanitize,prompt,diff}.rs`；`src/cli.rs` 新增 `Gitai` 变体；`src/main.rs` 新增 `run_gitai`。`[FACT]` 安全项按归档 AISEC-REQ-001/004 实现：不可信内容结构化打标 + 启发式密钥过滤（`src/ai/sanitize.rs`）。`[FACT]` 密钥只从 `GITGIT_AI_API_KEY` 环境变量读，不走 CLI flag（flag 会进 `ps` 和 shell history）。`[TBD]` 真实 OpenAI/Anthropic 端到端未跑通 — 缺凭据 |
| T8 | `[FACT]` 已落地；**验收口径经拍板变更** | `src/remote/{mod,sync}.rs`；`src/cli.rs` 新增 `Gitremote` 变体（add / ls / rm / sync）；`scripts/verify-gitremote-e2e.ps1` 为 CI 硬门禁（`.github/workflows/rust-backend.yml:200`）。`[FACT]` 原验收「存 PG」与 ADR-0022 §2.1 冲突，2026-10-03 经 Ulysses 拍板：**ADR-0022 不变，验收改为「存本地 + fast-forward sync 工作」**，存储为 `.gitgit/remotes.json`（ADR-0022 §2.3.1 记录了这次变更）。`[FACT]` `RemoteStore` 刻意做成可替换接口，将来若真需要外部存储，换实现不触碰 CLI 层 |
| T9 | `[FACT]` 已落地（2026-10-05）；**真实 provider 未验证，但事件链路已验证** | 层 1 `src/ai/stream.rs`（纯 SSE 解码器 + `AiProvider::send_stream`，11 条无 socket 测试）；层 2 Tauri 命令 `ai_review_start`/`ai_review_cancel` + `src/routes/Review.svelte` + `src/lib/stores/review.ts`。验收「GUI 上能看到 token 逐字流」由 mock SSE server 实测达成（`["Hel","lo","wo","rld","!"]`）。`[FACT]` i18n **194 键**双目录一致（2026-10-24 实测；旧记 146 / commit `c14c131` 记的 185 均已过期）；`src-tauri` 内 `#[test]`/`#[tokio::test]` 属性 30 处；vitest **508 passed / 35 files**（2026-10-24 实跑全绿）。Anthropic 显式拒绝而非静默降级。`[FACT]` **`TauriEmitter` 端到端已验证**（2026-10-05，本地 SSE 端点 `127.0.0.1:38999` 实跑 `pnpm tauri:dev`，7 token 逐个穿过 Tauri IPC 抵达 webview 并按序拼接，commit `9f93e62`）—— 本文档旧记「端到端未验」已关闭。`[UNVERIFIED-FACT]` 仍未验证的是**真实云端 provider**：无 `GITGIT_AI_API_KEY` |
| T10 | `[FACT]` 已落地（2026-10-04 补齐装卸实测）；**三条「尚未」已全部关闭** | `[FACT]` 4 个 MSI 产出并实测装卸（per-user/per-machine × en-US/zh-CN，SHA-256 见 `docs/reports/2026-10-03-tauri-packaging/tauri-packaging.md`）：非提权会话安装 exit 0、程序可启动、卸载 exit 0 且痕迹全清。`[FACT]` 修掉「启动即崩」：`tauri.conf.json` 三段死 `plugins.*` 配置在加载期致命（`344bc87`）。`[FACT]` **完整 `tauri build` 已在 CI 内** —— `.github/workflows/gm-desktop.yml` 的 `msi` job（windows-latest）跑 `build-msi.ps1 -Scope both` → `pnpm tauri build --bundles msi --ci`；`.github/workflows/gm-desktop-bundle.yml` 跑 `dmg`（macos-latest）/ `deb` / `appimage`（ubuntu-latest）；`.github/workflows/release.yml` 重复全部四个 target。`[FACT]` **双变体覆盖问题已解** —— `apps/gm-desktop/scripts/build-msi.ps1:380-387` 把产物重命名为 `{product}_{version}_{arch}_{scope}_{lang}.msi` 并移出共享目录，脚本自身断言两 scope 哈希互不相同（`:424-429`）；CI 分别上传两个 scope 的产物（`if-no-files-found: error`）。`[FACT]` **`.dmg` / `.appimage` / `.deb` 已有 CI 证据** —— 三个 job 均在 CI 中构建并校验（`gm-desktop-bundle.yml`），`.deb` 另有 `ubuntu:24.04` 容器里的真机安装 job；许可证交付已逐平台测量（commit `31afa4b` / `a33bf38`：deb 走 `bundle.linux.deb.files`、msi 走 `bundle.licenseFile` + 本仓 WiX 模板、dmg 走 `bundle.licenseFile` → create-dmg `--eula`；**appimage 亦有机制** —— `bundle.linux.appimage.files`，PR #45）。`[FACT]` **原「appimage 无任何机制、`verify-bundle.sh` 测量后如实报告为 AGPL-3.0 §4(a) 开放缺口」两处均已过时** —— 机制层面：`bundle.linux.appimage.files` 是 AppImage bundler 读取的第三个键（`tauri-utils` 的 `AppImageConfig.files: HashMap<PathBuf, PathBuf>`，上游 tag `tauri-cli-v2.12.1` 即 `pnpm-lock.yaml` 锁定的 `@tauri-apps/cli` 版本；`linuxdeploy.rs` 调 `fs_utils::copy_custom_files(&settings.appimage().files, &data_dir)` 后再复制 `data_dir/usr/`；`copy_custom_files` 以 map 的**键作目标**、值作源），故 `usr/` 前缀的目标即可把 License 放进镜像，缺口靠配置闭合、无需重打包；引用层面：AGPL-3.0 §4 **无字母子节**（`LICENSE:183-195`），应写 §4。`[FACT]` **CI 已证明许可证真的落进了镜像** —— PR #45 的 run `40092589002`（job `appimage (unsigned)`）日志为 `ok  licence: usr/share/doc/gitgit-desktop/LICENSE`；同段日志的 `bytes 8-10  : 414902` 另证实了 type-2 magic 的字节切片修复。详见「追补更正 1」。`[TBD]` **唯一仍未闭合的是签名证书** —— 签名链路已存在且在发布路径强制（`release.yml` 的 `msi` job 以 `-Required` 调用 `sign-msi.ps1`，缺证书直接红），但仓库未采购证书，故今天产出的 MSI 仍无 Authenticode 签名；`.dmg` 仅 ad-hoc 签名、无公证 |
| T11 | `[FACT]` 已落地（2026-10-05）；**载体已于 2026-10-25 换掉** | 最初的夹具是 `deploy/minio/docker-compose.yml` + `scripts/verify-minio-vault.ps1`；实测 versioning enabled / bucket private / 未签名 list 403。两条 `#[ignore]` e2e 测试首次实跑，暴露 2 个真实缺陷（404 错误文档被当凭据返回；`get_at_version` 缺 key 返回 Err）并已修复，提交 `46b07de`。验收口径已更正：列举 bucket 需 SigV4 签名，诚实答案是 403 而非「返回列表」。`[FACT]` **minIO 退场**：服务端换成本仓自建（`src/objectstore/` + `gitgit-objectstore`），夹具换成 `deploy/objectstore/`（无 `mc`、无 docker-compose），验证脚本换成 `scripts/verify-s3-vault.ps1`；两条 e2e 测试摘掉 `#[ignore]`，自己在进程内起服务端。`[FACT]` **产品现在真的接 S3 了**（M2a 起）：`GITGIT_VAULT_BACKEND=s3` 可选中 `S3Vault`，六项设置读 `GITGIT_S3_*` |

### 矛盾更正 A —— T5「PG 18.6 + sqlx」与现行代码冲突

任务表 T5 要求 PG 18.6 + sqlx + `migrations/20260826_v0_gui_keychain.sql`，与 `dev` 实际状态冲突：

- `[FACT]` `Cargo.toml` **无 `sqlx`、无任何 PostgreSQL driver**。文件头注释写明：
  `"This is intentionally a single crate, NOT a workspace. The previous 14-crate design is archived in docs_archive_rust_impl_2026_08_26/."`
- `[FACT]` `dev` 上**无 `migrations/` 目录**（`git ls-tree -r HEAD -- migrations/` 无输出；`dev` 的 tree 里没有任何 `migrations/` 路径）。仓内曾存在的 10 张表 migration（`0001_primitives` … `0010_view_snapshots`）属于 14-crate 设计，已随 commit `5007883`（single-crate MVP skeleton）删除。
- `[FACT]` **「`migrations/20260826_v0_gui_keychain.sql` 从未被提交」需更正（2026-10-24）**。该脚本**已被提交过**，commit `a34d812`（2026-10-02，*archive(migrations): V0 PG/sqlx 迁移脚本归档*）把它归档进 `docs_archive_rust_impl_2026_08_26/migrations/20260826120000_v0_gui_keychain.sql`（92 行）。
  `[FACT]` `git log --all --diff-filter=A -- '*keychain*'` 2026-10-24 实测返回 **1 条**（`a34d812`），不是空。
  `[FACT]` **文件名本就与本文档的引用不一致**：磁盘上的真名带 sqlx 时间戳前缀 `20260826120000_`，而 ADR-0020 与本文档引用的 `20260826_v0_gui_keychain.sql` 没有那个前缀 —— 这条悬空引用自 2026-10-02 起就存在，`a34d812` 的 commit message 自己也点出了它。
  `[FACT]` **该脚本从未被任何代码执行过**：`src/**/*.rs` 中 `migrations` 零引用，`Cargo.toml` 无 sqlx。
- **结论（不变）**：T5 的 PG/sqlx 路线未落地，且当前代码库无 PG 依赖。凭证数据实际落在 Vault（`rust-s3` / `FileVault`）而非 PG 表。T8「存 PG」因此同样无落点。
  变的是这一条的**措辞**：不是「从未被提交」，而是「**被归档进了仓，但没有作为活代码落地**」。

**更正动作**：T5 行状态标 `[FACT]` 未落地 / 路线冲突；下文「当前批次（执行中）」中 `migrations/20260826_v0_gui_keychain.sql` + `sqlx migrate run` 一项标注为已被上述证据取代，不作为待办执行依据。

### 矛盾更正 B —— T3「Svelte 5」与「不做（out of scope）」列表

- `[FACT]` **Svelte 5 属实，但仅 desktop 侧**：`apps/gm-desktop/package.json` 为 `svelte ^5.1.0`。需明确区分 —— `apps/gm-console` 是 **React 18**（`react ^18.3.1`；`react-router-dom` 已由 `^6.27.0` 升到 `^7.18.4`，commit `946a452`），**不是 Svelte**。T3 的「Svelte 5」只对 `apps/gm-desktop` 成立。
- `[FACT]` 「不做（out of scope）」把 **暗 / 亮主题切换** 与 **i18n** 列为 V1，但二者**已在盘上**：
  `apps/gm-desktop/src/lib/components/ThemeToggle.svelte`、`LocaleSwitcher.svelte`、
  `lib/stores/theme.ts`、`lib/stores/locale.ts`、`lib/i18n/zh-CN.ts` + `lib/i18n/en.ts`。
  `[FACT]` **「依 ADR-0023 §2.4，二者系自 `apps/desktop` 吸收进 `gm-desktop`」这句两处皆错，已更正（2026-10-24）**：
  1. `[FACT]` **ADR-0023 没有 §2.4** —— 该 ADR 的章节是 §1 / §2 / §3.1–§3.3 / §4 / §5 / §6 / §7 / §8，`§2` 下只有 4 条编号理由，没有 §2.4 子节。
  2. `[FACT]` **主题与 i18n 从来不在 `apps/desktop` 里** —— `git log --all -- 'apps/desktop/src/lib/i18n' 'apps/desktop/src/lib/stores/theme.ts' 'apps/desktop/src/lib/stores/locale.ts' 'apps/desktop/tailwind.config.js' 'apps/desktop/src/lib/components/ThemeToggle.svelte' 'apps/desktop/src/lib/components/LocaleSwitcher.svelte'` 返回**空**，这些路径在 `apps/desktop` 存在的全部历史里从未出现过。二者随 `gm-desktop` 自己的骨架 commit `2f8b9fc`（2026-09-19）一起落地，**早于** ADR-0023（2026-09-26 起草）。
  3. `[FACT]` **ADR-0023 §2 理由 4 说的恰是反面**：「`gm-desktop` 工程度更高 —— i18n (zh-CN + en) / 暗色模式 / … / LocaleSwitcher / ThemeToggle …，`apps/desktop` 仅 1 个 Node smoke 测试」。即主题与 i18n 是 `gm-desktop` **本来就有的**，不是从 `apps/desktop` 吸收来的。
  4. `[FACT]` **真正被吸收的是 knowledge-graph 能力**：ADR-0023 §3.1（PR-B，Rust 端图谱引擎）与 §3.2（PR-C，`/graph` 路由）。PR-C 对 i18n 的动作是**往既有目录加键**（§6.2：`apps/gm-desktop/src/lib/i18n/{zh-CN,en}.ts` 加 `nav.graph` + `graph.*` 文案），不是引入 i18n 本身。
- `[FACT]` **ADR-0023 §3.3**：commit `03bdd7b` 已执行 `git rm -r apps/desktop`。`apps/` 现有且仅有 `gm-console` + `gm-desktop`。

**更正动作**：将 i18n / 主题切换从「不做（out of scope）」移出，改记为已随 `gm-desktop` 骨架 `2f8b9fc` 落地（ADR-0023 只是确认了 `apps/desktop` 的删除，不涉及这两项）；T3 行加注 gm-console 为 React。

## 关键路径

```
T1 (Tauri 脚手架) → T2 (axum embed) → T4 (仓库视图)
                                       ↘ T9 (AI 评审 UI)
T11 (对象存储部署) ─┐
T5 (PG/sqlx) ─→ T6 (S3Vault) → T7 (AI provider)
                                 → T8 (remote provider)
                          T10 (打包) ← 全部
```

**T11 必须在 T6 之前完成**（bucket 就绪 → `S3Vault` 才有写入目标）。

`[FACT]` 2026-10-25：T11 的载体从「minIO 容器」换成「本仓自建服务端」，
T6 的实现名从 `MinioVault` 换成 `S3Vault`；依赖关系本身没变。

## 本轮新增更正（2026-10-24 复核发现，原文档未列出）

以下 7 条是本轮复核**新发现**的失实或过期陈述，不在原始委派清单内。每条都附可复核的证据。

1. `[FACT]` **「`TauriEmitter` 未端到端验证」已关闭** —— commit `9f93e62`（2026-10-06）关闭了
   README 的同名 known-issue 条目，并记录了实测：本地 SSE 端点 `127.0.0.1:38999` 逐帧吐 7 个
   token，实跑 `pnpm tauri:dev`，7 个 token 逐个穿过 Tauri IPC 抵达 webview 并按序拼接，首帧
   `model` 值被页面显示，mock 日志确认 `Authorization: Bearer` 未泄漏进事件。
   `[TBD]` **仍然开放的是真实云端 provider**（OpenAI / Anthropic 线上端点）—— 仓内无
   `GITGIT_AI_API_KEY`。原文档把这两件事混成一条「都未验证」，掩盖了已关闭的那一半。

2. `[FACT]` **`svelte-spa-router` 早已不在依赖里** —— 原文档 T3 行与状态核验 T3 行都写
   `svelte-spa-router ^4.0.1`。commit `6963340`（2026-10-02）以本仓自研的
   `apps/gm-desktop/src/lib/router.ts`（约 90 行 hash 路由核心）替换了它 ——
   `svelte-spa-router@4.0.2` 的 `Router.svelte` 调用 `afterUpdate`，Svelte 5 runes 模式禁止该
   API，`vite build` 直接失败，即该桌面端**从未有过可用的生产构建**。
   `[FACT]` `apps/gm-desktop/package.json` 现在**不含** `svelte-spa-router`。

3. `[FACT]` **`rust-s3` 已升到 0.38** —— 原文档 T6 行与状态核验 T6 行都写 `rust-s3 0.37`。
   `Cargo.toml` 现为 `rust-s3 = { version = "0.38", default-features = false, features =
   ["tokio-rustls-tls"] }`（commit `b1025a3`，清 RUSTSEC advisory）。
   `[FACT]` 但 `Cargo.toml` 里那条解释 0.37 行为的注释**仍然保留**（说明为何不能用
   `versionId`），两者不矛盾。

4. `[FACT]` **「归档 §5.4 的 `Stream()` 未实现」不成立** —— 原文档 T7 行写「`Stream()` /
   `EstimateCost()` 标为 V1，未实现也未 stub」。归档 §5.4 的 `Stream()` **并未标 V1**
   （只有 `EstimateCost()` 标了 `(V1)`），而且它**已实现**：`AiProvider::send_stream`
   （`src/ai/provider.rs:207`，OpenAI 实现在 `src/ai/openai.rs:223`），只是形态从 callback
   改成了 channel（`src/ai/stream.rs` 的 `StreamSender` / `StreamReceiver`）。
   `[FACT]` **仍未实现且刻意不 stub 的是 `EstimateCost()`** —— `src/ai/provider.rs:13-16`
   明写「a stub that returns a zero cost would be worse than a missing method」。

5. `[FACT]` **ADR-0023 没有 §2.4，且主题/i18n 并非自 `apps/desktop` 吸收而来** ——
   原文档「矛盾更正 B」与「不做（out of scope）」两处都写「依据 **ADR-0023 §2.4**，二者系自
   `apps/desktop` 吸收进 `gm-desktop`」。该 ADR 无 §2.4 子节；且主题/i18n 在 `apps/desktop`
   的**全部历史里从未存在过**（`git log --all` 查其 i18n / theme / locale / tailwind 路径返回
   空），二者随 `gm-desktop` 自己的骨架 `2f8b9fc`（2026-09-19）落地，早于 ADR-0023 起草。
   ADR-0023 §2 理由 4 说的恰是反面（`gm-desktop` 工程度更高，`apps/desktop` 仅 1 个 Node
   smoke 测试）。真正被吸收的是 knowledge-graph 能力（§3.1 PR-B / §3.2 PR-C）。
   详见「矛盾更正 B」。

6. `[FACT]` **`react-router-dom` 已由 `^6.27.0` 升到 `^7.18.4`** —— 原文档「矛盾更正 B」
   引用旧版本。commit `946a452`（2026-10-12，*fix/gm-console-react-router-7*）。
   `[FACT]` `react` 本身仍是 `^18.3.1`，「gm-console 是 React 18 不是 Svelte」这条结论不变。

7. `[FACT]` **`migrations/20260826_v0_gui_keychain.sql` 并非「从未被提交」** ——
   commit `a34d812`（2026-10-02）已把它归档进
   `docs_archive_rust_impl_2026_08_26/migrations/20260826120000_v0_gui_keychain.sql`。
   `git log --all --diff-filter=A -- '*keychain*'` 2026-10-24 实测返回 1 条而非空。
   路线未落地的结论不变，措辞需改。详见「矛盾更正 A」。

## 决策日志

- 2026-10-25 09:50 PT: 拍板 **minIO 退场、S3 vault 接回产品** —— 名字、夹具、测试三件事一次做完。`[FACT]` (1) **改名**：`MinioVault` → `S3Vault`、`MinioVaultConfig` → `S3VaultConfig`、旧的 `GITGIT_MINIO_*` → `GITGIT_S3_*`（M2a 已占用这六个名字，M2b 删掉旧的）。理由：它是 S3 客户端，用某个服务端的名字给客户端命名正是 minIO 退场的根因；`[INFERENCE]` 名字像许可证 ≠ 许可证。`[FACT]` (2) **夹具**：`deploy/minio/docker-compose.yml` → `deploy/objectstore/`，`scripts/verify-minio-vault.ps1` → `scripts/verify-s3-vault.ps1`；**少掉一个外部依赖**——`mc` 客户端与 docker-compose 都不再需要，服务端自己的 `--bootstrap-bucket` 就是 `mc mb` + `mc version enable` 的替代。`[FACT]` (3) **测试**：两条 vault e2e 摘掉 `#[ignore]`，自己在进程内起服务端（**端口 0** 让 OS 选、**桶显式建出来并开版本化**，因为 `x-amz-version-id` 只在版本化桶上才有），验收口径是「不带 `--ignored` 也要真的跑」——一条静默跳过的门禁不是门禁。`[FACT]` (4) **顺手修的**：`S3VaultConfig` 原来 derive 了 `Debug`，会把 secret key 明文打出来；改为手写 `Debug` 并用测试钉住（这是本 lane 之前就存在的缺陷，与改名同批修掉）。`[INFERENCE]` ADR 与计划文档**修订不重写**：历史决策文本保持原样，现状由「修订注记（日期）」承载
- 2026-10-25 21:50 JST: 拍板 **深门禁强制化的做法** = 把 `paths:` 过滤从 workflow 层**下沉到 job 内**（job 被 `if:` 跳过会报 Success，满足 required check；workflow 被 `paths:` 过滤则根本不跑，check 永远 pending、反而堵死 PR）。依据是 GitHub 官方文档两处原文，已记入 `docs/plan/deep-gate-enforcement.md`。实现要点：检测步骤**放在作业内部**而非独立 `detect` 作业（否则检测失败会跳过门禁、报绿色放行）；诊断性的 `if: always()` 上传**不加**区域条件（它们的价值就是交出失败那一步的产出）；**覆盖范围一个字节没变**——5 个区域的模式列表就是原 `paths:` 块逐字，并由 `workflow-gate.test.ts` 钉住做防漂移锚点。本批次先做了 `rust-backend` / `gm-console` / `frontend-audit` 三个 workflow（PR #59），后做了 `gm-desktop.yml` / `gm-desktop-bundle.yml`（PR #62，其 `msi` / `msi-launch` 刚在 PR #57 稳定）—— **2026-10-25 晚全部五个 workflow 完成**。**2026-10-25 22:35 与 23:20 JST 两次更新：分支保护已改** —— `dev` / `main` 的 required status check 现在是 `repo gate (always runs)` 加上**全部 14 条**深门禁 context，只 PATCH 了 `required_status_checks`，其余开关（strict / 管理员同受约束 / 线性历史 / 对话解决 / 禁删 / 禁强推）前后逐一核对未变。实证：**PR #60 与 PR #61（只改 `docs/**`，不命中任何区域）上 5 条深门禁全部报绿且未做活**（耗时 6–18s，而 #59 上同一条是 56s–2m39s），即「报状态、但不适用」这条正是设计要的行为；`repo gate` 仍是 44s（它总是真跑）。**PR #62** 上新扩展的 9 条（含三个消费者作业）全部真跑并通过。
- 2026-10-25 11:37 JST: 拍板 **发布通路的签名策略** = 强制签名不变（`sign-msi.ps1 -Required`），采购代码签名证书并配置 `MSI_SIGN_PFX_BASE64` / `MSI_SIGN_PFX_PASSWORD`；同时把 draft release 标题里的 `(unsigned draft)` 去掉——该标题与「强制签名」自相矛盾，且无证书时根本不产出任何 release（PR #55）
- 2026-10-25 11:37 JST: 拍板 **第一次真正跑通发布通路** = 切一个预发布 tag（如 `v0.1.0-rc.1`）做端到端验证。**该步骤依赖上一条的证书**，证书未到位前推 tag 只会得到一个 msi 变红、且不产出任何 release 的 run，并留下一个切不掉的 tag
- 2026-10-25 11:37 JST: 拍板 **`dev` / `main` 开启 branch protection** = 要求 PR、要求线性历史、要求分支最新、stale review 失效、**管理员同样受约束**；required status check = `repo gate (always runs)`（新增的 `.github/workflows/gate.yml`，无 `paths:` 过滤，故每个 PR 都会报状态）。**当时的遗留问题**：深门禁（打包 / Rust 测试 / 审计 / 两个 ratchet）仍是 path 过滤的，因而**未被自动强制**——GitHub 文档明写 path 过滤掉的 workflow 会让 required check 永远 pending 从而堵死 PR（PR #55 实地验证：`rust-backend` 的 paths 一个都不匹配所以根本没跑）。要「深门禁也自动强制」需把 path 过滤下沉到 job 级（让 job 跳过而非 workflow 不跑），是五个 workflow 的重构 —— **2026-10-25 晚全部五个完成**（`rust-backend` / `gm-console` / `frontend-audit` 见 PR #59，`gm-desktop` / `gm-desktop-bundle` 见 PR #62），见 `docs/plan/deep-gate-enforcement.md`；**2026-10-25 22:35 与 23:20 JST 分支保护已改**（详见本日志同日 21:50 那一条）
- 2026-10-25 11:37 JST: 拍板 **minIO 的 AGPL-3.0 商业风险** = 采购 minIO Enterprise License。**发布前必须完成采购；未采购即不得对外分发。** 备选（若采购不成）= 改用 Ceph RGW / SeaweedFS / Garage，届时要重做凭据管理与版本化逻辑
- 2026-10-25 06:40 PT（**取代上一条**）: 拍板 **minIO Enterprise License 采购不成，改为自建对象存储** —— 本仓自己实现一个 S3 兼容的对象存储服务端，替代 minIO。`[FACT]` 理由：minIO 服务端是 GNU AGPLv3，而本仓以 HTTP 方式消费它；自建后本仓不再依赖任何第三方 AGPL 服务端。`[INFERENCE]` 形态**不变**：ADR-0021 的「S3 兼容 bucket `gitgit-vault` + S3 客户端作为默认实现、FileVault 降级为 V1 fallback」这条架构不动，要自己写的只是**服务端**；客户端（`rust-s3` 0.38，path-style 寻址 + SigV4）**照旧**，所以凭据管理与版本化逻辑**不需要重做**——这是相对「换用 Ceph RGW / SeaweedFS / Garage」那条备选的最大差别。`[FACT]` 要自己实现的 S3 面已被实测钉死，只有七类操作（bucket 建/查 versioning、object put/get/delete、`?versionId=` 读、ListObjectsV2 列举），加上**两种 SigV4 变体**（header 签名与 `presign_get` 用的 query 签名）。`[FACT]` `MinioVault` / `GITGIT_MINIO_*` 这两个名字届时会随 minIO 一起退场。
- 2026-10-25 06:40 PT: 拍板 **MSI 代码签名证书暂缓采购** —— 上面两条「发布通路的签名策略」与「第一次真正跑通发布通路」**继续阻塞**。`[FACT]` 阻塞是真的：发布通路的 `msi` 作业以 `-Required` 调 `sign-msi.ps1`，无证书即**失败关闭**，不产出任何 release；推 tag 只会得到一个 msi 变红、不产出 release 的 run，并留下一个切不掉的 tag。`[INFERENCE]` 这段时间该做的是把**不依赖证书**的部分做完（自建对象存储、其余深门禁与商业化缺口），而不是降低签名要求。
- 2026-08-30 15:42 JST: 拍板 Credential Vault 从 FileVault 改为 minIO S3-compatible bucket `gitgit-vault` + `MinioVault` 默认实现；FileVault 降级为 V1 fallback（per ADR-0021）
- 2026-08-26 17:06: 拍板 V0 范围 = GUI 骨架 + 两套 API Key
- 2026-08-26 17:00: 环境核实（Rust 1.98.0 / PG 18.6 / Git 2.41 / Node 22）
- 详见 ADR-0020 §2 + [ADR-0021](../adr/0021-v0-minio-credential-vault.md)

## 当前批次（执行中）

> 2026-10-02 更正（证据见「矛盾更正 A」）：本批次原列的
> `migrations/20260826_v0_gui_keychain.sql` + `sqlx migrate run` **已被证据取代**——
> `Cargo.toml` 无 sqlx / 无 PG driver，`dev` 上无 `migrations/` 目录。
> 下列两项**不作为待办执行依据**，是否重启 PG 路线需重新拍板 `[PROPOSAL]`。
>
> 2026-10-24 补充：该脚本已被 commit `a34d812` 归档进
> `docs_archive_rust_impl_2026_08_26/migrations/`（从未作为活代码执行），
> 「从未被提交」的旧措辞已在「矛盾更正 A」更正。

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
| **minIO AGPL-3.0 商业风险** | 中 | 高 | `[DECIDED 2026-10-25 06:40 PT]` **改为自建对象存储，不再采购 minIO Enterprise License**（采购不成，改为自己实现一个 S3 兼容服务）。`[FACT]` 动机：minIO 服务端是 GNU AGPLv3，本仓以 HTTP 方式连它，把一个 AGPL 服务打进发行物会引出网络条款的义务。`[INFERENCE]` 自建则本仓不再依赖任何第三方 AGPL 服务端，风险随依赖一起消失。`[FACT]` 代价与取舍见 `docs/plan/self-hosted-object-store.md`（S3 面已被实测钉死、架构落点、正确性怎么证明、以及明确不做的部分）：客户端（`rust-s3`）与 ADR-0021 的「S3 兼容 bucket」形态**保持不变**，要自己写的是服务端；`MinioVault` 这个名字届时会随 minIO 一起退场。**风险未闭合前（服务端未落地前），仍不得对外分发。** |
| **minIO 部署失败 / 网络不可达** | 中 | 高 | `[CLOSED 2026-10-25]` **服务端换成本仓自建**（`src/objectstore/` + `gitgit-objectstore`），夹具不再依赖容器运行时：起脚本 `deploy/objectstore/start-objectstore.ps1` 自带就绪探针，`scripts/verify-s3-vault.ps1` 做端到端验证（桶已建且版本化已开、未签名请求 403、产品二进制 `gitgit key set/get/list-keys/versions/delete` 全链路跑通、两条 vault e2e 测试真跑并通过）。选 `file` 后端仍是可用的降级路径 |
| **minIO 备份缺失** | 中 | 高 | `[CLOSED 2026-10-25]` **改为自建服务端后不再适用**（minIO 的 erasure coding 与 site replication 都是它的特性）。自建服务端是文件系统后端：`versioning` 始终保留（每次写入一个版本目录，`set_versioning(Disabled)` 会被拒绝），删除只移除当前版本、旧版本仍可按 `versionId` 读回。**仍未做**：跨节点复制与 90d lifecycle —— V1+ 议题 |
| **minIO root user 凭证管理**（递归引用 vault 自身） | 中 | 中 | `[CLOSED 2026-10-25]` **递归引用消失了**：服务端不再需要「root user」概念，它只有一对 access key / secret key，且**密钥只走环境变量** `GITGIT_OBJECTSTORE_SECRET_KEY`（没有 `--secret-key` 旗标，理由同 `src/ai/mod.rs`：`ps` 可见 + shell history）。vault 侧那对凭据同样只走 `GITGIT_S3_ACCESS_KEY` / `GITGIT_S3_SECRET_KEY`。**仍未做**：KMS / Secret Manager（V1+ 议题） |
| 凭证 vault 误写权限 | 中 | 高 | `S3Vault`（原 `MinioVault`）默认走 bucket policy（V0 简化：单租户 single-user access key 即可）；FileVault fallback 仍 0600 权限。`[FACT]` 自建服务端每个请求都验 SigV4 签名，未签名一律 403 |
| **AGPL-3.0 §6(d) 交付义务：事实已记录，合法性待裁定**（2026-10-24 新增并补记，见「追补更正 2」） | 未评估 | 未评估 | `[FACT]` 义务存在：对象码经指定地点交付时，须在同一地点以同等方式提供 Corresponding Source，并在对象码旁保留「指向 Corresponding Source 的明确指引」（`LICENSE:263-275`）。`[FACT]` 仓库为**公开**仓库；**指定地点**＝发布页 `https://github.com/UlyssesLeoLee/GitGit/releases`，对象码与源代码同址、同方式、at no further charge。`[FACT]` 发布页正文已新增 `## Corresponding Source` 段落，写明已解析的 TAG、AGPL-3.0 及其文本所在处、源 tag 归档 URL。`[FACT]` 对象码旁的指引＝仓内提交文件 `CORRESPONDING-SOURCE.md`，经 `bundle.linux.deb.files` / `bundle.linux.appimage.files` 打进 `.deb` 与 `.AppImage`，`verify-bundle.sh` 按**内容**断言其存在。`[FACT]` `.msi` 仍只有许可证 RTF、`.dmg` 的许可证是 UDIF EULA 资源，两者都**没有**指引文件（机制限制见「追补更正 2」）。`[PROPOSAL]` **合法性判断仍然开放** —— 本项只记录事实，由负责法律判断的人裁定，本项不作「是否违规」的判断 |

## 不做（out of scope）

> 2026-10-02 更正（证据见「矛盾更正 B」）：原「暗 / 亮主题切换」与「i18n」两条与盘上事实冲突，
> 已移出本节 —— 二者已随 **`gm-desktop` 骨架 commit `2f8b9fc`** 落地，代码在盘
> （`ThemeToggle.svelte` / `LocaleSwitcher.svelte` / `i18n/{zh-CN,en}.ts`）。
> 2026-10-24 更正：本节原先标注的 ADR-0023 §2.4 依据**有误** —— 该 ADR 无 §2.4 子节，
> 且这两项从未存在于 `apps/desktop`（其全部历史里无这些路径），并非自其吸收而来。
> ADR-0023 §2 理由 4 记的是相反的事实（`gm-desktop` 本就工程度更高）。
> 详见「矛盾更正 B」与「本轮新增更正」第 5 条。

- 右键菜单 TortoiseGit 风格（V1）
- 多 remote 冲突解决（V1）
- LLM 调度 remote（V1 末端）
- ~~暗 / 亮主题切换（V1）~~ → `[FACT]` 已落地于 `apps/gm-desktop`（骨架 commit `2f8b9fc`）
- ~~i18n（V1）~~ → `[FACT]` 已落地于 `apps/gm-desktop`（骨架 commit `2f8b9fc`）

## 进度跟踪

每完成一个任务，commit 一次，格式：
```
feat(v0/T<n>): <description>
```

合并到 `feature/v0-gui-and-keychain` 的 commit 序列将是：
T1 → T5 → T2 → T3 → T4 → T11 → T6 → T7 → T8 → T9 → T10

> **修订说明 (8/30 15:42 JST)**：T11 (minIO 部署) 是 T6 (MinioVault) 的强依赖；commit 序列相应在 T4 后插入 T11。原 commit 序列 (T1 → T5 → T2 → T3 → T4 → T6 → T7 → T8 → T9 → T10) 仅 10 个 commit，新增 T11 后总 11 个 commit。本修订日无任何 commit 已 push，序列调整仅在文档侧生效。

> **2026-10-24 复核补充**：`[FACT]` 分支 `feature/v0-gui-and-keychain` **现已不存在**
> （`git branch -a --list '*v0-gui-and-keychain*'` 返回空），本文档头部的 Branch 字段是历史记录。
> `[FACT]` 上列全部 11 个任务对应的 commit **均已落在 `dev` 上**
> （`git branch --contains` 逐个核过：`2f8b9fc` / `bdb328b` / `0728213` / `eb8eddf` /
> `d3d184e` / `46b07de` / `5088f7d` / `677eb1b` / `cc41e00` / `c14c131` / `344bc87`）。
> `[FACT]` T5 一项**从未落地**，故实际序列里没有对应 commit —— 见「矛盾更正 A」。