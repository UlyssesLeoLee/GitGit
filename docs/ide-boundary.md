# GitGit IDE Boundary — 对接契约

> **状态**：🟡 草案 v0.1
> **日期**：2026-08-26
> **背景分支**：`feature/ide-boundary`
> **签批**：⏳ 待签（per [ADR-0001](adr/0001-ide-boundary-contract.md) §7）
> **不重复内容**：本仓库对应章节均镜像 [STAR `gitgit-ide-boundary.md`](../../../Star/docs/responsibility-matrix/gitgit-ide-boundary.md)，但**只**包含 GitGit 侧独立约束。STAR 侧完整 60 项正交表由 STAR 仓库持有。

---

## 1. 本仓库对外契约（GitGit 视角）

### 1.1 对任何 AI Agent / IDE 暴露的能力

| 能力 | 暴露方式 |
|---|---|
| 标准 Git 协议（clone / fetch / push / pull / worktree） | `git` 命令 + Smart HTTP + SSH |
| Repository 列表 / 详情 | REST API `/api/v1/repos` (Phase D+) |
| Commit / Tree / Blob 元数据 | REST API |
| Diff / Blame / History | REST API + 标准 git 命令 |
| Webhook 注册 | REST API |
| Git 原生事件（RepositoryCreated / CommitCreated / RefUpdated / ...） | WebHook 推送 |

### 1.2 GitGit **不**暴露（必拒）

| 能力 | 拒绝理由 |
|---|---|
| `gitgit issue` / `gitgit task` / `gitgit project` | 业务概念不属于 VCS Core |
| `gitgit ai-review` / `gitgit context` | AI 决策不属于 VCS Core |
| `gitgit mr-approve` | 业务流不属于 VCS Core |
| `gitgit ci-run` | 业务编排 |
| `gitgit sprint` / `gitgit roadmap` | 业务 |
| `gitgit admin` / `gitgit enterprise-policy` | 业务 |
| 任何 Web UI 主入口 | UI 是 Human Interface，Agent API 必须 CLI / Git / HTTP |
| Vendor-specific 命名空间（`ClaudeAdapter` / `CodexAdapter` / ...） | Zero Vendor Cooperation |

### 1.3 GitGit **可以**提供（已存在 / 未来允许）

- 完整 Git 对象模型（commit / tree / blob / tag）
- 完整 Git 协议（v1 / v2）
- Worktree 物理层（add / remove / list / status / prune / move / repair / unlock）
- LFS 标准实现
- 仓库镜像（mirror / fetch / push mirror）
- 受保护分支 / 受保护 tag 的**底层**能力（判定 + 拒绝）
- CODEOWNERS 文件解析
- Git 原生事件流
- Repository Snapshot / Partial Clone / Sparse Checkout / 大文件支持
- 文件级历史
- 基础文本搜索（**非** AST / **非** semantic）

## 2. 必跑测试（守门）

### 2.1 标准 Git 兼容测试

```bash
# 这些命令必须 100% 等价于标准 Git 行为
git clone http://localhost:8080/owner/repo.git
cd repo
git log --oneline
git status
git diff HEAD~1
git blame README.md
git branch
git checkout -b feature/x
git commit -m "test"
git push origin feature/x
git worktree add ../wt feature/x
git worktree list
git worktree remove ../wt
```

### 2.2 Smart HTTP 协议测试

```bash
# 必须看到标准 git-upload-pack / receive-pack 交换
GIT_TRACE=1 git clone http://localhost:8080/owner/repo.git 2>&1 | head -50
# 日志必须显示：
#   GET /info/refs?service=git-upload-pack
#   POST /git-upload-pack
#   GET /info/refs?service=git-receive-pack
#   POST /git-receive-pack
```

### 2.3 守门 grep 测试（CI 必跑）

```bash
# 1. 不含 vendor 命名空间
grep -rE "ClaudeAdapter|CodexAdapter|CursorAdapter|CopilotAdapter|VSCodeAdapter|JetBrainsAdapter" src/
# 必须为空

# 2. 不含 IDE/AI 业务概念
grep -rE '"ide"|"agent"|"task"|"issue"|"context"|"rag"|"sprint"|"roadmap"|"approval"' src/ | grep -vE '^\s*//|^\s*\*'
# 应只在注释或变量名出现，无业务逻辑

# 3. 不含 vendor-specific 分支
grep -rE 'if.*provider.*==.*"claude"|if.*provider.*==.*"codex"|if.*provider.*==.*"gpt"' src/
# 必须为空
```

## 3. 灰色地带处置

| 场景 | 处置 |
|---|---|
| "GitGit 提供 CI 跑测试" | 拒。CI 是 STAR 责任。GitGit 只发 WebHook 通知。 |
| "GitGit 暴露 PR 评审 UI" | 拒。MR 流程是 STAR 责任。GitGit 只暴露底层 commit/branch 状态。 |
| "GitGit 内置 JWT 认证" | 拒。Auth 边界是 STAR 责任。GitGit 用 SSH key + PAT。 |
| "GitGit 暴露 '我的待办' API" | 拒。这是 STAR domain-work-item。 |
| "GitGit 提供 LSP server 暴露 commit 级代码智能" | **可**。这是 GitGit "代码智能底座"范围（per STAR ADR §8.3）。完整 AST/Symbol/Type 必须放 STAR。 |

## 4. 与 V0 WBS 的隔离

| 任务 | 分支 | 内容 |
|---|---|---|
| V0 WBS T1~T10（c89f858 之前） | `feature/v0-gui-and-keychain` | 桌面应用 + 凭证 Vault |
| IDE 边界契约（本文） | `feature/ide-boundary` | GitGit 内部契约文档 |

**两者工作面完全分离**。本契约不约束 V0 Tauri GUI 实现，也不被 V0 反向约束。

## 5. 修订历史

| 版本 | 日期 | 修订人 | 修订内容 | 触发 |
|---|---|---|---|---|
| v0.1 | 2026-08-26 | 架构师（Ulysses（一人公司 12 角色 per DEC-008）） | 初版 | 与 STAR 侧升级对齐（仅对接文档） |
| v0.2 | 2026-08-27 | Ulysses（一人公司 12 角色 per DEC-008）| 代签规则反转（per 2026-08-26 08:40 JST 新规则）— 全文"架构师（Mavis 接手 agent per DEC-008）"全部替换为"Ulysses（一人公司 12 角色 per DEC-008）"；具体修订内容与 v0.1 一致，仅署名更新 | per 用户 2026-08-27 07:16 JST 指令"全部允许代签 Ulysses，并签名 Ulysses" |
