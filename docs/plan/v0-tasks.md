# V0 WBS — Tauri GUI + AI/Remote 集成

> **Plan ID**: V0 | **Branch**: `feature/v0-gui-and-keychain`
> **Owner**: Mavis (per DEC-008) | **Reviewer**: Ulysses (DDD Review)
> **Anchor**: gitgit MVP commit `1da5f2c` on `simplify/2026-08-26-mvp`
> **ADR**: [ADR-0020](../adr/0020-v0-gui-tauri-svelte.md)
> **Started**: 2026-08-26 17:06 JST

## 范围（in scope）

V0 = Tauri 2 GUI 骨架 + 两套 API Key（AI provider + Git remote），
**在 gitgit MVP 之上叠加**，单 Rust crate `gitgit` + Tauri 2 desktop app。

## 任务分解（10 个工作块，约 8-10 个工作日）

| # | 任务 | 工时 | 依赖 | 验收 |
|---|---|---|---|---|
| 1 | **Tauri 2 脚手架** — `pnpm create tauri-app` 加 src-tauri/，能 `cargo tauri dev` 出窗口 | 0.5 d | — | 窗口出现，标题 "gitgit" |
| 2 | **axum 同进程 embed** — gitgit 现有 `axum::serve` 跑进 Tauri `setup()`，端口 38080 | 0.5 d | 1 | Svelte fetch `127.0.0.1:38080/info/refs` 拿到 200 |
| 3 | **Svelte 5 基础** — 路由、布局、主题（系统默认） | 1 d | 1 | 窗口能切 3 个 page（仓库列表 / 详情 / 设置） |
| 4 | **仓库视图** — 列表 + status + log(最近 5) + diff 视图 | 1.5 d | 2, 3 | 选个 repo 看到 working tree status + commit graph |
| 5 | **PG 18.6 + sqlx** — migration 5 张表 + `migrations/20260826_v0_gui_keychain.sql` | 0.5 d | — | `sqlx migrate run` 通过 |
| 6 | **Credential Vault** — `trait Vault` + `FileVault` 实现 + `WindowsVault` (V1 切换) | 1 d | 5 | `gitai key set openai <key>` 存到文件，读回能验证 |
| 7 | **AI provider 注册表** — 5 个 provider + `gitai` 子命令（commit/explain/review） | 2 d | 6, 5 | `gitai commit --from-diff` 真打通 OpenAI，输出 commit message |
| 8 | **remote provider 注册表** — 5 个 provider + `gitremote` 子命令（add/ls/sync） | 1.5 d | 6, 5 | `gitremote add gitee <url>` 存 PG + fast-forward sync 工作 |
| 9 | **AI 评审 UI** — push 前弹窗 + streaming token 显示 | 1 d | 4, 7 | GUI 上能看到 token 逐字流 |
| 10 | **MSI 打包 + 烟测 + ADR 收尾** | 0.5 d | 全部 | `cargo tauri build` 出 .msi，scripts/smoke.ps1 仍过 |

**总**：约 10 天（1.5 周）

## 关键路径

```
T1 (Tauri 脚手架) → T2 (axum embed) → T4 (仓库视图)
                                       ↘ T9 (AI 评审 UI)
                  T5 (PG/sqlx) → T6 (Vault) → T7 (AI provider)
                                          → T8 (remote provider)
                          T10 (打包) ← 全部
```

## 决策日志

- 2026-08-26 17:06: 拍板 V0 范围 = GUI 骨架 + 两套 API Key
- 2026-08-26 17:00: 环境核实（Rust 1.98.0 / PG 18.6 / Git 2.41 / Node 22）
- 详见 ADR-0020 §2

## 当前批次（执行中）

**批次 1：环境 + 脚手架（T1 + T5）**
- 安装 `tauri-cli@^2.0`
- `pnpm create tauri-app` 初始化
- 验证 Tauri dev 能起窗口
- 写 `migrations/20260826_v0_gui_keychain.sql` + 跑 `sqlx migrate run`
- 写 PG 连接配置到 `.env`（PG 18.6 socket + user）

## 风险登记

| 风险 | 概率 | 影响 | 缓解 |
|---|---|---|---|
| Tauri 2 编译时间长（首次 5-15 分钟） | 高 | 中 | 先 release-debug profile 编译一次，再切回 dev |
| pnpm 不可用 | 中 | 低 | 退回 npm 装 Svelte 模板 |
| WebView2 runtime 缺失 | 低 | 高 | Win 11 默认有；缺则引导装 Edge WebView2 |
| PG 18.6 新版 sqlx 兼容 | 低 | 中 | sqlx 0.8 已声明支持 PG ≥ 11 |
| 凭证 vault 误写权限 | 中 | 高 | 文件 fallback 默认 0600；Windows Vault 需 `chmod` 0x0 |

## 不做（out of scope）

- 右键菜单 TortoiseGit 风格（V1）
- 多 remote 冲突解决（V1）
- LLM 调度 remote（V1 末端）
- 暗 / 亮主题切换（V1）
- i18n（V1）

## 进度跟踪

每完成一个任务，commit 一次，格式：
```
feat(v0/T<n>): <description>
```

合并到 `feature/v0-gui-and-keychain` 的 commit 序列将是：
T1 → T5 → T2 → T3 → T4 → T6 → T7 → T8 → T9 → T10
