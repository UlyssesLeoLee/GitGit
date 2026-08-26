# ADR-0020: V0 GUI / Tauri 2 / Svelte 5 / axum-embed 决策

| 字段 | 值 |
|---|---|
| **Status** | Accepted (2026-08-26) |
| **Supersedes** | (无) |
| **Superseded by** | (无) |
| **Authors** | Mavis（架构师，per DEC-008） |
| **Reviewers** | Ulysses（DDD Review，pending） |
| **Deciders** | Ulysses |
| **Tags** | gui, tauri, svelte, v0, keychain, ai-provider |

---

## 1. 背景 / Context

8/25 收尾的 gitgit MVP（commit `1da5f2c`）交付了一个可工作的 Rust
HTTP server：

- axum 0.7 + tokio + clap，单 crate `gitgit` (~900 行)
- 端到端烟测 6 秒过：clone / commit / push / branch / fetch / merge /
  无凭证拒绝
- 走 shell `git` 子进程（ADR-0002 禁纯 Rust 复写）
- 绑端口 `0.0.0.0:8080`，HTTP Basic auth（admin/admin）

需求（用户 8/26 17:00 拍板）：

1. 一个 Git 前端工具，**类似 Git + TortoiseGit**
2. **集成 API Key**——两个角度都要：AI provider key + Git remote PAT
3. 在 gitgit MVP 上**叠加**（不另起新项目）
4. 本轮交付 V0 + 两套 API Key（GUI 骨架 + AI + 多 remote）
5. 遇到技术难点：同类问题攒 3 个批量问，不每个都打断

环境约束（已验证 8/26 17:00）：

- Rust 1.98.0 ✅
- PostgreSQL 18.6（`D:\PostgreSQL\18\bin\psql.exe`），服务 `postgresql-x64-18 Running` ✅
- Git 2.41.0 ✅
- Node 22 + npm 路径在 PATH ✅（pnpm 需确认）
- Ollama 路径在 PATH（`C:\Users\leo19\AppData\Local\Programs\Ollama`）✅

## 2. 决策 / Decision

**采用 Tauri 2 + Svelte 5 单窗口 + axum 同进程 embed + sqlx + PG 18.6 落库
的架构**。具体如下：

### 2.1 GUI 框架：Tauri 2（Rust 端）+ Svelte 5（前端）

| 选择 | Tauri 2 | 备选：Electron / 原生 Qt / WPF |
|---|---|---|
| 包大小 | ~10 MB | Electron 100+ MB |
| 启动 | 1-2 秒 | Electron 3-5 秒 |
| Web 视图 | 系统 WebView2 (Win 11 默认) | Chromium 200+ MB |
| 与 Rust 后端集成 | `tauri::command` + `invoke` 直接调 | 需 IPC + 序列化 |
| Svelte 支持 | 官方模板 | 需手搭 |

**Tauri 2 是 Windows 11 平台最优解**——复用系统 WebView2，零额外 Chromium。

### 2.2 后端集成：同进程 embed axum（端口 38080）

Tauri 启动时在 `tauri::Builder::setup` 里 `tokio::spawn` 跑 `axum::serve(listener, router)`，
监听 `127.0.0.1:38080`。Svelte 前端 fetch 这个端口；和 CLI 命令行访问 gitgit 完全
同 API。

**为什么不用子进程**：
- spawn 启动慢 1-2 秒
- 状态不能共享（PG pool、credential vault、ai_call_cache 都要双份）
- debug 时排错路径长

**为什么不用 Tauri IPC**：
- 前端失去"普通 web 客户端"能力——Svelte 不能直接用 fetch
- 不能用 curl / Postman 调试
- 二者共用 HTTP 边界一致更好维护

### 2.3 数据库：PG 18.6 + sqlx

5 张新表（见 §2.6），`sqlx::migrate!` 静态管理。本轮不重写 graph 引擎
（参考 `docs_archive_rust_impl_2026_08_26/design/detailed-design/02-graph-engine.md`
的 DDL，V1 落 graph）。

### 2.4 Git 协议：沿用 gitgit 现有 shell `git` 进程（ADR-0002）

不引 gix、不引 libgit2 绑定。写路径必须用 `git` CLI。

### 2.5 API Key 集成：注册表 + factory 模式

10 个 provider 全部走同一 trait：

```rust
#[async_trait]
trait Provider {
    fn name(&self) -> &str;
    async fn call(&self, req: Request) -> Result<Response>;
}
```

AI provider（5 个）：

| Provider | 凭证来源 | 默认 base URL | 默认模型 |
|---|---|---|---|
| OpenAI | Credential Vault | `https://api.openai.com/v1` | gpt-4o-mini |
| Anthropic | Credential Vault | `https://api.anthropic.com` | claude-3-5-haiku-latest |
| DeepSeek | Credential Vault | `https://api.deepseek.com` | deepseek-chat |
| Ollama | (无) | `http://127.0.0.1:11434/v1` | qwen2.5-coder:7b |
| 自定义 OpenAI-compatible | Credential Vault | 用户填 | 用户填 |

Git remote provider（5 个）：

| Provider | 凭证来源 | 备注 |
|---|---|---|
| GitHub | Credential Vault | PAT，scope: `repo` |
| GitLab | Credential Vault | PAT，scope: `api` |
| Gitee | Credential Vault | PAT |
| Bitbucket | Credential Vault | App password |
| 自定义 HTTP Git server | Credential Vault | basic auth / PAT，含**本仓库 gitgit 自身** |

**Credential Vault** 抽象：

```rust
trait Vault {
    async fn get(&self, key: &str) -> Result<Option<String>>;
    async fn set(&self, key: &str, value: &str) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<()>;
}
```

实现：
- `WindowsVault` → `keyring` crate 调 Windows Credential Manager
- `FileVault` → 落 `~/.config/gitgit/credentials.toml`（0600 文件权限）
- MVP 默认 `FileVault`（避免 UAC 问题），V1 推 `WindowsVault` 为默认

### 2.6 SQL schema（V0 范围）

新 5 张表（`migrations/20260826_v0_gui_keychain.sql`）：

```sql
-- AI 调用结果缓存
CREATE TABLE ai_call_cache (
    id BIGSERIAL PRIMARY KEY,
    diff_hash CHAR(64) NOT NULL,    -- sha256 of diff content
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    prompt_hash CHAR(64) NOT NULL,
    response JSONB NOT NULL,
    input_tokens INTEGER,
    output_tokens INTEGER,
    cost_usd NUMERIC(10, 6),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ai_call_cache_diff_idx ON ai_call_cache(diff_hash, provider, model);

-- AI 评审历史
CREATE TABLE ai_review_runs (
    id BIGSERIAL PRIMARY KEY,
    repo_path TEXT NOT NULL,
    base_sha CHAR(40) NOT NULL,
    head_sha CHAR(40) NOT NULL,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    status TEXT NOT NULL,    -- pending / running / completed / failed
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at TIMESTAMPTZ,
    finding_json JSONB,
    error TEXT
);

-- 多 remote 配置
CREATE TABLE remote_configs (
    id BIGSERIAL PRIMARY KEY,
    repo_path TEXT NOT NULL,
    name TEXT NOT NULL,         -- "origin" / "gitee" / "gitlab-mirror" / ...
    provider TEXT NOT NULL,     -- github / gitlab / gitee / bitbucket / custom
    base_url TEXT NOT NULL,
    auth_kind TEXT NOT NULL,    -- pat / basic / none
    vault_key TEXT,             -- credential vault key for this remote
    last_sync_at TIMESTAMPTZ,
    last_sync_status TEXT,      -- ok / diverged / error
    UNIQUE (repo_path, name)
);

-- sync 历史
CREATE TABLE remote_sync_log (
    id BIGSERIAL PRIMARY KEY,
    repo_path TEXT NOT NULL,
    remote_name TEXT NOT NULL,
    src_sha CHAR(40),
    dst_sha CHAR(40),
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at TIMESTAMPTZ,
    status TEXT NOT NULL,        -- ok / diverged / rejected / error
    error TEXT
);

-- GUI 偏好
CREATE TABLE user_prefs (
    key TEXT PRIMARY KEY,
    value_json JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

### 2.7 边界与不做的事

本轮**不做**：

- TortoiseGit-style 右键菜单（V1 再做，UAC + Windows Defender 警告）
- 多 remote sync 的冲突解决（V1，policy 留 TODO：fast-forward 失败时拒
  绝还是 cherry-pick）
- LLM 调度 remote（"把 feat/foo 同步到所有 remote 并给每个 provider 写
  PR description"）—— 留 V1 末端
- 暗色 / 亮色主题切换（V0 用系统默认）
- i18n（V0 中英混排，能用就行）

## 3. 后果 / Consequences

### 3.1 正面

- 1 个 Rust 进程同时跑 server + GUI 后端，省 1-2 秒启动
- 单一 HTTP 边界，前端 / CLI / 调试工具共用
- 注册表模式加 provider 不动核心
- PG 18.6 + sqlx 静态校验，编译期抓 SQL 错

### 3.2 负面 / 风险

- Tauri 2 仍在 1.x → 2.0 过渡期（截至 8/26 最新 stable 是 2.1）
  → 用 `tauri = "2.1"` 锁版本，避免 minor 升级 breaking
- WebView2 依赖 Win 10+1809 — 你机器 Win 11 满足 ✅
- PG 18.6 是相对新版本，sqlx 0.8 完整支持（PG ≥ 11）
- 多 remote sync 的冲突策略留 V1，本轮**只支持 fast-forward sync**

### 3.3 推翻的备选

| 备选 | 为什么不选 |
|---|---|
| Electron | 启动慢 3-5x，包大 10x |
| 另起新 D:\GitGitGui 仓库 | 用户拍板"在 gitgit 上叠加" |
| 子进程 spawn gitgit | 启动慢、状态难共享 |
| Tauri IPC 不用 HTTP | 失去 fetch / curl 调试能力 |
| SQLite 替代 PG | 与 D:\GitGit 设计文档冲突（要 PG 落库） |
| 重写 graph 引擎 V0 | 不在 MVP 范围，V1 再说 |
| 写 TortoiseGit 风格右键菜单 | UAC + Windows Defender warning，V1 |

## 4. 验证 / Verification

V0 完成的验收清单：

- [ ] `cargo tauri dev` 启动后窗口出现，标题栏显示 "gitgit"
- [ ] 窗口内能看到本地至少 1 个 git 仓库（用 gitgit demo 测试）
- [ ] `cargo run -- gitai commit --from-diff` 真打通 OpenAI（用我自己的
      key 测试一次）
- [ ] `cargo run -- gitremote add gitee <url>` 把 remote 存到 PG
- [ ] `cargo tauri build` 出 `.msi` 安装包
- [ ] 烟测 `scripts/smoke.ps1` 仍 6 秒 PASS（不退化）
- [ ] 单测 `cargo test` 5/5 仍过

## 5. 参考 / References

- 8/25 收尾 commit `1da5f2c`（gitgit MVP fix）
- 8/19 `docs_archive_rust_impl_2026_08_26/architecture/decisions/` 内
  8 个 ADR 全部已 Accepted，未被本 ADR 推翻
- Tauri 2.1 文档：https://tauri.app/v2/
- sqlx 0.8 PG 文档：https://docs.rs/sqlx/0.8/sqlx/postgres/
- keyring crate：https://docs.rs/keyring/3/

---

**Status: Accepted** | 2026-08-26 17:00 JST
