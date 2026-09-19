# GitGit IDE Boundary Test Checklist

> **状态**：🟡 草案 v0.1
> **日期**：2026-08-26
> **用途**：PR 守门 / CI 必跑
> **背景分支**：`feature/ide-boundary`

---

## 0. 目的

把 [ADR-0001](adr/0001-ide-boundary-contract.md) §4 + [ide-boundary.md](ide-boundary.md) §2 的守门规则转成**可执行**的 checklist。CI 必跑，失败即拒绝合并。

## 1. PR 检查清单（人工 review 必查）

任何修改 GitGit 的 PR，reviewer 必查以下 8 项：

| # | 检查项 | 通过标准 |
|---|---|---|
| 1 | 新增 IDE/AI 相关能力？ | **否** |
| 2 | 引入 vendor 命名空间（`ClaudeAdapter` / `CodexAdapter` / `CursorAdapter` / `CopilotAdapter` / `VSCodeAdapter` / `JetBrainsAdapter`）？ | **否** |
| 3 | 引入 vendor-specific 分支（`if provider == "claude"` 等）？ | **否** |
| 4 | 引入 Issue / Task / Project / Sprint / CI / Approval 概念？ | **否** |
| 5 | 引入 Web UI 主入口（HTML / SPA）？ | **否**（GitGit 只暴露 CLI + Git protocol + HTTP API） |
| 6 | 破坏标准 Git 兼容？ | **否** |
| 7 | 修改 `src/repo/` 或 `src/server/` 语义层？ | 必走 [STAR 责任矩阵](../../../Star/docs/responsibility-matrix/star-vs-gitgit.md) 对照 |
| 8 | 修改 Cargo.toml 新增 vendor-specific 依赖？ | **否** |

## 2. CI 必跑（自动化）

### 2.1 grep 守门（必须为空）

```bash
#!/usr/bin/env bash
# scripts/check-ide-boundary.sh
set -e

ROOT="$(git rev-parse --show-toplevel)"

# 1. 不含 vendor 命名空间
if grep -rE "ClaudeAdapter|CodexAdapter|CursorAdapter|CopilotAdapter|VSCodeAdapter|JetBrainsAdapter" "$ROOT/src/"; then
  echo "❌ vendor 命名空间禁止"
  exit 1
fi

# 2. 不含 vendor-specific 分支
if grep -rE 'if.*provider.*==.*"claude"|if.*provider.*==.*"codex"|if.*provider.*==.*"gpt"|if.*provider.*==.*"gemini"' "$ROOT/src/"; then
  echo "❌ vendor-specific 分支禁止"
  exit 1
fi

# 3. 不含业务概念（注释除外）
if grep -rE '"ide"|"agent"|"task"|"issue"|"context"|"rag"|"sprint"|"roadmap"|"approval"' "$ROOT/src/" | grep -vE '^\s*//|^\s*\*'; then
  echo "❌ 业务概念禁止在 src/ 出现"
  exit 1
fi

# 4. 不含 Web UI 入口
if find "$ROOT/src/" -name "*.html" -o -name "*.htm" 2>/dev/null | head -5; then
  echo "❌ GitGit 不应提供 Web UI"
  exit 1
fi

echo "✅ IDE boundary check passed"
```

### 2.2 标准 Git 兼容（必须 100% 通过）

```bash
#!/usr/bin/env bash
# scripts/test-git-compat.sh
set -e

# 起本地 server
cargo run --release -- server --port 18080 &
SERVER_PID=$!
sleep 3

# 创建测试仓库
mkdir -p /tmp/gitgit-test-source
cd /tmp/gitgit-test-source
git init --bare
cd -

# 客户端用标准 git 试
git clone http://localhost:18080/test.git /tmp/gitgit-test-client
cd /tmp/gitgit-test-client
git log --oneline
git status
echo "test" > README.md
git add .
git commit -m "test"
git push origin master
git worktree add ../wt-test -b feature/test
git worktree list
git worktree remove ../wt-test
cd -
rm -rf /tmp/gitgit-test-source /tmp/gitgit-test-client

kill $SERVER_PID

echo "✅ Standard Git compat test passed"
```

### 2.3 Smart HTTP 协议

```bash
#!/usr/bin/env bash
# scripts/test-smart-http.sh
set -e

cargo run --release -- server --port 18081 &
SERVER_PID=$!
sleep 3

# 必须看到标准 git-upload-pack
GIT_TRACE=1 git clone http://localhost:18081/test.git /tmp/gitgit-smart 2>&1 | grep -E "git-upload-pack|info/refs"
RESULT=$?

# 必须看到 git-receive-pack
cd /tmp/gitgit-smart
echo "x" > x.txt
git add . && git commit -m "x"
GIT_TRACE=1 git push origin master 2>&1 | grep -E "git-receive-pack"

cd -
rm -rf /tmp/gitgit-smart
kill $SERVER_PID

if [ $RESULT -eq 0 ]; then
  echo "✅ Smart HTTP protocol test passed"
else
  echo "❌ Smart HTTP protocol test failed"
  exit 1
fi
```

## 3. CI 配置（per repo `.github/workflows/`）

```yaml
# .github/workflows/ide-boundary.yml
name: IDE Boundary Check

on: [push, pull_request]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: IDE boundary grep
        run: bash scripts/check-ide-boundary.sh
      - name: Standard Git compat
        run: bash scripts/test-git-compat.sh
      - name: Smart HTTP protocol
        run: bash scripts/test-smart-http.sh
```

## 4. 不在范围内（out of scope）

- ❌ 实际写代码改动（V0 WBS 仍独立推进）
- ❌ 改 Cargo.toml 新增 vendor 依赖
- ❌ 改 src/repo/ 或 src/server/ 语义
- ❌ 任何 IDE 集成 UI / IDE plugin
- ❌ 任何 AI 适配器代码

## 5. 修订历史

| 版本 | 日期 | 修订人 | 修订内容 | 触发 |
|---|---|---|---|---|
| v0.1 | 2026-08-26 | 架构师（Ulysses（一人公司 12 角色 per DEC-008）） | 初版：3 段 CI 守门 + PR checklist | 与 STAR 侧升级对齐（仅对接文档） |
| v0.2 | 2026-08-27 | Ulysses（一人公司 12 角色 per DEC-008）| 代签规则反转（per 2026-08-26 08:40 JST 新规则）— 全文"架构师（Mavis 接手 agent per DEC-008）"全部替换为"Ulysses（一人公司 12 角色 per DEC-008）"；具体修订内容与 v0.1 一致，仅署名更新 | per 用户 2026-08-27 07:16 JST 指令"全部允许代签 Ulysses，并签名 Ulysses" |
