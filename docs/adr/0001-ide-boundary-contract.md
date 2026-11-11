# ADR-0001: GitGit IDE Boundary Contract

> **状态**：🟡 草案 v0.1
> **日期**：2026-08-26
> **制定者**：架构师（Ulysses（一人公司 12 角色 per DEC-008））— per 2026-08-26 08:40 JST 代签新规则
> **签批**：⏳ 待签
> **背景分支**：`feature/ide-boundary`（基于 `feature/v0-gui-and-keychain` 起点；不动 V0 Tauri GUI 任务）
> **对齐文档**：[STAR ADR-0021 ~ 0025](../../../Star/docs/adr/)（per 2026-08-26 零厂商适配升级，**STAR 项目内容不在本任务范围**，本 ADR 仅做 GitGit 侧契约对齐）

---

## 1. 背景

STAR × GitGit 2026-08-26 升级在 STAR 侧已经起草了 IDE 边界责任矩阵（per `D:/Star/docs/responsibility-matrix/star-vs-gitgit.md` v0.1 + `gitgit-ide-boundary.md` v0.1）。**GitGit 侧需要把"对外契约"显式落到本仓库**，作为 GitGit 演进时的守门基线。

**本 ADR 不重复 STAR 侧的内容**，只补 GitGit 侧独有的：
- 哪些能力**一定不能**进 GitGit（即使 STAR 升级）
- 哪些能力**可以**进 GitGit 但只在 Rust crate 级别
- 哪些 PR 必拒

## 2. 决策

**GitGit 对任何 AI Agent / IDE 来说就是标准 Git。任何"理解 AI / IDE 概念"的 PR 必拒。**

### 2.1 GitGit 不应拥有

（per STAR `gitgit-ide-boundary.md` §6，反向同步）

| 能力 | 拒绝理由 |
|---|---|
| Issue 面板 | 业务概念，不进 VCS Core |
| Task 面板 | 业务概念 |
| Project Dashboard | 业务概念 |
| AI Chat / Prompt / Agent Session | AI 决策不进 Core |
| RAG / Knowledge Graph | AI 决策 |
| Code Explanation / Generation | AI 决策 |
| Code Review Workflow | 业务流 |
| MR 审批流程 | 业务流 |
| CI/CD 编排 | 业务流 |
| Sprint / Roadmap / DORA | 业务 |
| 企业权限策略 / Approval | 业务 |
| Web UI（GitGit 不应暴露"网页入口"作为主要交互） | UI 是 Human Interface；Agent API 必须 CLI / Git / HTTP |

### 2.2 GitGit 可以拥有（保留）

- Repository / Git Object / Commit / Branch / Tag / Ref
- Diff / Blame / History
- Merge / Rebase / Conflict Detection
- Git Protocol / SSH / Smart HTTP
- Git LFS
- Repository Mirror
- Worktree 物理层（add/remove/list/status）
- Protected Branch / Protected Tag 底层能力
- CODEOWNERS 解析
- Git 原生事件（RepositoryCreated / CommitCreated / RefUpdated / ...）
- Repository Snapshot / Object Streaming / Partial Clone / Sparse Checkout

### 2.3 GitGit 可以提供"代码智能底座"（如未来需要）

- 文件变更事件
- Commit 级代码变化
- 文件路径索引
- 基础文本搜索（**非** AST / **非** semantic）
- Diff 计算
- 文件历史

**完整 AST / Symbol / Type / Call Graph / Semantic Search 仍归 STAR**。

## 3. 标准 Git 兼容守门（必跑测试）

```bash
# 1. 标准 Git 客户端 100% 兼容
git clone http://localhost:8080/owner/repo.git
cd repo && git log --oneline
git push origin main
git worktree add ../wt feature-branch
git worktree list

# 2. Smart HTTP 协议
GIT_TRACE=1 git clone http://localhost:8080/owner/repo.git
# 必须看到 git-upload-pack / receive-pack 标准交换

# 3. SSH 协议
GIT_SSH_COMMAND="ssh -i test_key" git clone ssh://git@localhost/owner/repo.git
```

**所有命令必须 100% 等价于标准 Git 行为**。Agent / IDE 看到的是 `git`，不是 `gitgit`。

## 4. PR 守门规则

任何修改 GitGit 的 PR 必查：

| 检查项 | 处置 |
|---|---|
| 新增 IDE/AI 相关能力？ | 拒 |
| 引入 `ClaudeAdapter` / `CodexAdapter` / `CursorAdapter` / `CopilotAdapter` / `VSCodeAdapter` / `JetBrainsAdapter`？ | 拒 |
| 引入 `if provider == "claude"` 等 vendor-specific 分支？ | 拒 |
| 引入 Issue / Task / Project / Sprint / CI / Approval 概念？ | 拒 |
| 引入 Web UI 入口？ | 拒（GitGit 只暴露 CLI + Git protocol + HTTP API） |
| 破坏标准 Git 兼容？ | 拒 |
| 修改现有 `src/repo/` 或 `src/server/` 的语义层？ | 必 review 边界责任矩阵 |

## 5. 不动 V0 Tauri GUI 任务

**V0 WBS（per `D:/GitGit/docs/plan/v0-tasks.md`）独立于本次升级**。`feature/v0-gui-and-keychain` 分支与 `feature/ide-boundary` 分支**无冲突**：

- V0 Tauri GUI 任务 = 桌面应用 + 凭证 Vault（per c89f858 之前的 main = 1da5f2c MVP 之上）
- IDE 边界 = GitGit 内部契约（本文档）

**两者的工作面完全分离**。

## 6. 与上游 STAR 责任矩阵的对接

GitGit 侧契约显式引用 STAR 侧责任矩阵：
- `D:/Star/docs/responsibility-matrix/star-vs-gitgit.md` — 60 项能力正交表
- `D:/Star/docs/responsibility-matrix/gitgit-ide-boundary.md` — GitGit IDE 接口边界

**GitGit 侧的边界**是 STAR 侧矩阵的**反向确认**——任何在 GitGit 端违反的对齐项都要 flag。

## 7. 签字栏

| # | 角色 | 姓名 | 签字日 | 结论/条件 |
|---|---|---|---|---|
| 1 | 架构负责人 | Mavis（per DEC-008） | 2026-08-26 | ⏳ 待 Ulysses 拍板 |
| 2 | SRE Lead | ⏳ 待签 | ⏳ 待签 | ⏳ 待签 |
| 3 | 平台工程师 | ⏳ 待签 | ⏳ 待签 | ⏳ 待签 |
| 4 | 评审主持人 | ⏳ 待签 | ⏳ 待签 | ⏳ 待签 |
| 5 | 项目负责人（PM） | ⏳ 待签 | ⏳ 待签 | ⏳ 待签 |

## 8. 修订历史

| 版本 | 日期 | 修订人 | 修订内容 | 触发 |
|---|---|---|---|---|
| v0.1 | 2026-08-26 | 架构师（Ulysses（一人公司 12 角色 per DEC-008）） | 初版：GitGit 侧 IDE 边界契约 + PR 守门规则 | 与 STAR 侧升级对齐（仅做对接文档，不动 Rust 源码） |
| v0.2 | 2026-08-27 | Ulysses（一人公司 12 角色 per DEC-008）| 代签规则反转（per 2026-08-26 08:40 JST 新规则）— 全文"架构师（Mavis 接手 agent per DEC-008）"全部替换为"Ulysses（一人公司 12 角色 per DEC-008）"；具体修订内容与 v0.1 一致，仅署名更新 | per 用户 2026-08-27 07:16 JST 指令"全部允许代签 Ulysses，并签名 Ulysses" |
