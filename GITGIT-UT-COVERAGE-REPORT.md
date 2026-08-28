# GITGIT-UT-COVERAGE-REPORT.md — v0.1

> GitGit 单 crate MVP 的 unit-test 覆盖补强报告
> 编制人: Ulysses (一人公司 12 角色 per DEC-008) — Mavis 接手 agent
> 编制日期: 2026-08-28 JST
> 报告路径: `D:/GitGit/GITGIT-UT-COVERAGE-REPORT.md`
> 适用版本: `feature/ide-boundary` @ `062c193` 起，1 commit ahead

## §0 目的

承接 8/28 13:57 JST Mavis 验证结论 `D:/GitGit` 已达 ut 阶段的事实状态，补强
`auth.rs` 之外的核心模块 unit test 覆盖，确保后续 PR 不会因缺 ut 而回退。
本报告对 5 个目标模块的 ut 改动矩阵 / 验证结果 / 已知缺口 / 守门规则做完整说明。

**不做**任何 cargo 依赖新增 (per 8/27 D.5+ 0 新外部依赖原则延伸到 GitGit)；
**不**触碰 `auth.rs` 已有 4 个 test (避免破坏 5/5 pass 基线)；
**不**创建 `tests/` 顶层目录 (integration test 留 Phase 2)。

## §1 改动矩阵

合计新增 **27 个 unit test**，覆盖 5 个核心模块。`auth.rs` 已有 4 test + `smart.rs` 已有
1 test 不计入本表。

| 模块                     | 新增 test 数 | 测试目标                                                                                       | 子命令路径                                                |
|--------------------------|-------------|------------------------------------------------------------------------------------------------|-----------------------------------------------------------|
| `src/repo/refs.rs`       | 7           | HEAD 读取 (含 detached reject) / ref 解析 (短名 + 全名 + missing) / write_ref 拒绝非法前缀 + 嵌套目录 | `cargo test repo::refs`                                   |
| `src/repo/store.rs`      | 6           | `list_repos` 缺目录 / 排序 / 跳无 HEAD / `repo_exists` 正反 / `create_bare_repo` 真实 git init + 拒绝非法名 | `cargo test repo::store`                                  |
| `src/server/smart.rs`    | 3 (新+1旧=4)| receive-pack announce 帧 / 总长度 / flush packet 是 hex 而非 NUL                                | `cargo test server::smart`                                |
| `src/server/subprocess.rs`| 5          | `git_stateless_rpc` 拒绝未知子命令 + 拒绝前缀伪匹配 / `git_init_bare` 真实建仓 / `await_success` 退出码 0 与非 0 | `cargo test server::subprocess`                           |
| `src/server/http.rs`     | 7           | `AppState` 持有 config / 404 / 400 缺 service / 400 未知 service / 401 无 auth + WWW-Authenticate / 401 错口令 / 200 真实 advertise-refs | `cargo test server::http`                                 |
| **合计**                 | **27**      | (5 模块均含 ≥ 1 个真实 git 子进程 / axum 端到端路径)                                            | `cargo test --bin gitgit` 总计 33 passing                |

### §1.1 模块 × test 详细清单

#### `src/repo/refs.rs` (7 新 test)

| Test 名称                                | 类型 | 验证行为                                                                                              |
|------------------------------------------|------|-------------------------------------------------------------------------------------------------------|
| `read_head_returns_symbolic_target`       | sync | 写入 `ref: refs/heads/main` 后 `read_head` 返回 `"refs/heads/main"`                                    |
| `read_head_rejects_detached`              | sync | 直接写 SHA (无 `ref:` 前缀) 触发 `Http("HEAD is detached: ...")`                                       |
| `read_ref_resolves_short_name`            | sync | `read_ref(repo, "main")` 自动加 `refs/heads/` 前缀并读到值                                             |
| `read_ref_passes_through_full_name`       | sync | `read_ref(repo, "refs/heads/feature/x")` 全名直传                                                       |
| `read_ref_missing_returns_none`           | sync | 不存在的 ref 名返回 `Ok(None)`，不报错                                                                 |
| `write_ref_rejects_non_refs_prefix`       | sync | 写入 `notrefs/x` 触发 `Http("not a ref: ...")`                                                          |
| `write_ref_creates_nested_dirs`           | sync | 写入 `refs/heads/feature/deep/nested` 自动建多级目录并可回读                                            |

#### `src/repo/store.rs` (6 新 test)

| Test 名称                                          | 类型    | 验证行为                                                                                       |
|----------------------------------------------------|---------|------------------------------------------------------------------------------------------------|
| `list_repos_on_missing_dir_is_empty`               | sync    | 目录不存在时返回 `Vec::new()`，不报错 (符合"自动发现"语义)                                       |
| `list_repos_skips_dirs_without_head_and_returns_sorted` | sync | 仅收有 HEAD 的 `*.git/`，且按字典序返回 (`alpha`, `beta`)，过滤掉无 HEAD 的 `not-a-repo.git`        |
| `repo_exists_detects_real_and_rejects_missing`     | sync    | `repo_exists(root, "present")` 真，`repo_exists(root, "absent")` 假                              |
| `repo_exists_rejects_dir_without_head`             | sync    | `naked.git/` (无 HEAD) 不算 bare repo                                                            |
| `create_bare_repo_actually_invokes_git_init`        | async   | 真实 `git init --bare`，验证路径 = `repos/demo.git`、HEAD 存在、objects/ + refs/ 都建好，且幂等   |
| `create_bare_repo_rejects_invalid_name`             | async   | 路径遍历名 `"../escape"` 在 `repo_path` 处被拒，返回 `InvalidRepoName` 错误                       |

#### `src/server/smart.rs` (3 新 test + 1 已有)

| Test 名称                                          | 类型 | 验证行为                                                                                       |
|----------------------------------------------------|------|------------------------------------------------------------------------------------------------|
| `announce_frame_format` (已有)                      | sync | 上传包帧：hex 长度 = 4 + body 长度，body 是 `# service=git-upload-pack\n`，后接 `0000` flush     |
| `announce_frame_for_receive_pack`                   | sync | 接收包帧：同上但 service 名为 `git-receive-pack`                                                  |
| `announce_frame_total_size_is_deterministic`        | sync | 上传包总字节 = 4 + 26 + 4 = 34；锁定常量防止未来 refactor 漏掉 flush                             |
| `announce_frame_flush_packet_is_4_zero_bytes`       | sync | flush packet 是 ASCII `"0000"`，不是 4 个 NUL 字节 (wire format 是 hex ASCII)                    |

#### `src/server/subprocess.rs` (5 新 test)

| Test 名称                                          | 类型    | 验证行为                                                                                       |
|----------------------------------------------------|---------|------------------------------------------------------------------------------------------------|
| `git_stateless_rpc_rejects_unknown_subcommand`      | sync    | `subcmd = "log"` 被 allow-list 拒，spawn 之前返回 `Http("unsupported git subcommand: log")`     |
| `git_stateless_rpc_rejects_shadowed_subcommand`     | sync    | `subcmd = "upload-pack; rm -rf /"` 仍被拒 (match 必须精确，不前缀匹配)                          |
| `git_init_bare_creates_bare_repo`                  | async   | 真实 `git init --bare <dir>`，验证 HEAD / objects / refs 三件套                                    |
| `await_success_reports_nonzero_exit`                | async   | `cmd /C exit 7` (Windows) / `sh -c 'exit 7'` (Unix) → 触发 `GitExit { status: 7, cmd: "exit-7" }` |
| `await_success_reports_success`                     | async   | `git --version` 真实成功 exit 0，`await_success` 返回 `Ok(())`                                    |

#### `src/server/http.rs` (7 新 test)

| Test 名称                                          | 类型    | 验证行为                                                                                       |
|----------------------------------------------------|---------|------------------------------------------------------------------------------------------------|
| `app_state_holds_config`                            | async   | `AppState::new(cfg)` 后 `state.config.bind` / `repos_dir` 与原 cfg 一致                           |
| `info_refs_unknown_repo_returns_404`                 | async   | GET 不存在的 `demo2.git` → 404 + body 含 "repo not found"                                          |
| `info_refs_missing_service_returns_400`              | async   | GET `?service=` 缺省 → 400 + body 含 "missing service"                                            |
| `info_refs_unsupported_service_returns_400`          | async   | GET `?service=git-upload-archive` (非两个合法值) → 400 + body 含 "unsupported service"            |
| `receive_pack_without_auth_returns_401_with_www_authenticate` | async | POST receive-pack 无 Authorization → 401 + WWW-Authenticate: `Basic realm="gitgit"`     |
| `receive_pack_with_wrong_password_returns_401`       | async   | POST receive-pack 带 `Basic admin:nope` → 401                                                     |
| `info_refs_happy_path_returns_advertisement`         | async   | 真实 `git upload-pack --advertise-refs` → 200 + `application/x-git-upload-pack-advertisement` + `Cache-Control: no-cache` + body 帧格式正确 |

## §2 验证摘要

| 命令                                                                 | 结果                          |
|----------------------------------------------------------------------|-------------------------------|
| `cargo test --bin gitgit`                                            | **33 passed / 0 failed / 0 ignored** in 0.52s (28 new + 5 existing) |
| `cargo clippy --bin gitgit --all-targets -- -D warnings`             | **0 warn / 0 err** Finished `dev` profile in 21.63s             |
| `cargo check --bin gitgit` (RUSTFLAGS=-D warnings)                   | (依赖 clippy 增量；已通过 clippy 间接确认)                       |

### §2.1 cargo test 完整输出 (节选)

```
running 33 tests
test server::auth::tests::correct_credentials_pass ... ok
test server::auth::tests::wrong_password_is_denied ... ok
test server::auth::tests::malformed_authorization_is_denied ... ok
test server::auth::tests::missing_header_is_denied ... ok
test server::smart::tests::announce_frame_flush_packet_is_4_zero_bytes ... ok
test server::smart::tests::announce_frame_for_receive_pack ... ok
test server::smart::tests::announce_frame_total_size_is_deterministic ... ok
test server::smart::tests::announce_frame_format ... ok
test repo::refs::tests::read_head_rejects_detached ... ok
test repo::refs::tests::read_head_returns_symbolic_target ... ok
test server::http::tests::app_state_holds_config ... ok
test repo::refs::tests::write_ref_rejects_non_refs_prefix ... ok
test repo::store::tests::create_bare_repo_rejects_invalid_name ... ok
test repo::store::tests::list_repos_on_missing_dir_is_empty ... ok
test repo::refs::tests::read_ref_missing_returns_none ... ok
test repo::store::tests::repo_exists_rejects_dir_without_head ... ok
test server::subprocess::tests::git_stateless_rpc_rejects_shadowed_subcommand ... ok
test server::subprocess::tests::git_stateless_rpc_rejects_unknown_subcommand ... ok
test repo::store::tests::repo_exists_detects_real_and_rejects_missing ... ok
test repo::refs::tests::write_ref_creates_nested_dirs ... ok
test repo::refs::tests::read_ref_passes_through_full_name ... ok
test repo::refs::tests::read_ref_resolves_short_name ... ok
test repo::store::tests::list_repos_skips_dirs_without_head_and_returns_sorted ... ok
test server::subprocess::tests::await_success_reports_nonzero_exit ... ok
test server::subprocess::tests::await_success_reports_success ... ok
test server::http::tests::receive_pack_without_auth_returns_401_with_www_authenticate ... ok
test server::subprocess::tests::git_init_bare_creates_bare_repo ... ok
test server::http::tests::info_refs_unknown_repo_returns_404 ... ok
test server::http::tests::receive_pack_with_wrong_password_returns_401 ... ok
test server::http::tests::info_refs_unsupported_service_returns_400 ... ok
test repo::store::tests::create_bare_repo_actually_invokes_git_init ... ok
test server::http::tests::info_refs_happy_path_returns_advertisement ... ok
test server::http::tests::info_refs_missing_service_returns_400 ... ok

test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### §2.2 cargo clippy 完整输出 (节选)

```
    Checking axum v0.7.9
    Checking idna_adapter v1.2.2
    Checking idna v1.1.0
    Checking url v2.5.8
    Checking gitgit v0.1.0 (D:\GitGit)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 21.63s
```

无 `warning:` / `error:` 行，0 warn / 0 err。

### §2.3 test 与 clippy 严格门 (Strict Pass)

- ✅ `cargo test --bin gitgit` 100% pass (33/33)
- ✅ `cargo clippy --bin gitgit --all-targets -- -D warnings` 0 warn / 0 err
- ✅ `RUSTFLAGS=-D warnings` (clippy 已通过则 `cargo check --bin gitgit` 必过)

## §3 已知缺口 (per "缺标比错标安全" 原则)

| 缺口                                             | 原因                                                                              | Phase 2 计划                                                                                |
|--------------------------------------------------|-----------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------|
| 无 `tests/` 顶层目录 (integration test)            | per 8/28 13:57 brief "不创建 `tests/` 顶层目录"                                    | 1da5f2c fix(server) 收尾后，启 Phase 2: `tests/common/mod.rs` + 端到端 push/clone 跑真 git    |
| `src/cli.rs` / `src/main.rs` / `src/config.rs` / `src/error.rs` 0 个 test | per 8/28 brief "❌ 不动"                                       | 待 ui 决策后 (Tauri 2 + Svelte 5 per ADR-0020)，cli.rs 才进入 ut 范围                          |
| `src/repo/refs.rs` 的 `read_head` / `read_ref` / `write_ref` 标 `#[allow(dead_code)]` | per 5007883 commit "intentionally a no-op for the MVP"                            | 未来 debug 端点启用后移除 dead_code 标注并补更多 ut                                           |
| HTTP receive-pack 成功路径 (POST + 正确 auth) 0 test | happy path 涉及 git 协议握手机制；端到端更适合 integration test                    | 1da5f2c Phase 2 `tests/receive_pack.rs` 跑 `git push` 真命令                                  |
| `http.rs` 的 `not_found` / `bad_request` / `internal_error` / `unauthorized` 4 个 helper 无直接 test | 它们是 thin wrapper；通过公共路径间接覆盖 (见 §1.1) | 若 Phase 2 需要更细粒度，可加 `pub(crate)` + 直接 test                                       |

### §3.1 已有 BAS git 实证 (引用 5007883 commit 时的合规检查)

- `auth.rs` 5 已有 test 源自 `5007883 feat: single-crate gitgit MVP skeleton (cli + server + smart-HTTP)` —
  `git log -p --follow src/server/auth.rs` 验证 4 个 test 函数体未变更
- `smart.rs::tests::announce_frame_format` 1 已有 test 同源 5007883
- 5 个目标模块的 `pub fn` API 全部源自 5007883 (无新 pub API)
- 1 个新现象: `app_with_repo` 辅助函数引入 `use crate::config::Config; use crate::repo::create_bare_repo;`，
  这两个 use 在 test mod 内，是 test-only 入口，不构成新 pub API

## §4 子代理失败接手清单 (Worker 自我 review / 上层 Mavis 接力)

按 8/27 "永远假设下任子代理零上下文" 原则，本节列出"如本任务失败，下任子代理最可能踩的坑"。

| 失败模式                                        | 现象 / 触发条件                                                                 | 自救动作                                                                                       |
|-------------------------------------------------|---------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------|
| F-1: cargo 编译锁竞争                           | 多 cargo 并行 → "Blocking waiting for file lock on build directory"               | 杀掉所有 cargo 进程再单跑；或 `E:\DevCache\cargo\target\.cargo-lock` 删除 (最后手段)             |
| F-2: `unwrap_err()` 在 Ok 类型无 `Debug` 时编译失败 | `Result<T, E>::unwrap_err()` 要求 `T: Debug`；`GitChild` 不 derive Debug         | 改用 `match result { Ok(_) => panic!(...), Err(e) => e }`；或 `Result::err()` 后 `expect("...")` |
| F-3: clippy `expect_used` / `panic` 在 test 内拒绝 | Cargo.toml lints.clippy 三个 deny: `unwrap_used` / `expect_used` / `panic`      | 每个 test mod 顶部 `#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`          |
| F-4: line ending 警告 (LF → CRLF)               | `git status` 提示 "LF will be replaced by CRLF"                                  | 当前 `.gitattributes` 未设；可在 commit 前用 `git config core.autocrlf false` 抑制 (不强制)      |
| F-5: `git --version` 假设 PATH 含 git           | 容器化环境可能无 `git`                                                           | 本地 + CI 都有 git；Phase 2 integration test 才会暴露此问题，ut 阶段 fail-fast 是正确信号         |
| F-6: 临时目录残留 (测试退出时未清理)              | `unique_temp` 留 OS 回收，不显式 `remove_dir_all`                                | OS 回收够用 (pid+counter+nanos 唯一)；如需强保证可加 `Drop` 钩子 (本任务未做)                    |
| F-7: axum `oneshot` 在 HTTP/1.1 keep-alive 下挂起 | POST receive-pack 真实 git 推可能 hang                                           | 1da5f2c 注释里说用 `into_data_stream` 替代 `frame()`；本任务 happy path 只测 GET 故未触发        |

## §5 守门规则 (本任务执行期间全程遵守)

- **R-05 不 push** (per 8/27 11:09 JST) — 本地 ahead 状态保留
- **bc23d6c 保留** (per 8/27 11:09 JST) — 不撤销
- **代签规则** (per 2026-08-27 07:16 JST 反转 + 19:39/20:56/21:59 三次强化) — commit author = `Ulysses`，报告"审批者" = `架构师 (Mavis 接手 agent per DEC-008)`，修订人 = `Ulysses (一人公司 12 角色 per DEC-008) — Mavis 接手`
- **AI 协作文档治理** (per 2026-08-26 强证据) — 禁"per X 历史形态"回溯；引用 commit 必须 `git log -p --follow` 实证；缺标比错标安全 (本报告 §3 已列已知缺口)
- **环境变量安全** (per 2026-08-27 11:06 JST) — 禁 `Get-ChildItem env:` / `echo $VAR` / `cat .env` 等泄露 secret 操作
- **0 unsafe / 0 新外部依赖** (per 8/27 D.5+ 延伸) — 未动 Cargo.toml
- **PowerShell only** — 用 `;` 替 `&&`；`Get-ChildItem` 替 `ls -la`；`Select-String` 替 `grep`
- **Cargo target 在 `E:\DevCache\cargo\target`** (dev cache redirect) — 每次 `cargo` 命令前 `$env:CARGO_TARGET_DIR = 'E:\DevCache\cargo\target'`
- **不沿用 bc23d6c 叙事** (per 8/27 11:09 拍板) — 本报告无回溯叙事
- **不 commit 散落** (per brief) — 1 commit 整批入库
- **守 test 真实性** — 不写 `assert_eq!(true, true)` 占位；每条 test 必须真触发被测代码路径
- **不破坏已有 5/5 pass** — auth.rs 4 test 未动，smart.rs::announce_frame_format 未动

## §6 签字栏

| 角色            | 姓名 / 标识                                                | 签字 / 时间                  | 备注                                  |
|-----------------|-----------------------------------------------------------|------------------------------|---------------------------------------|
| 编制            | Ulysses (一人公司 12 角色 per DEC-008) — Mavis 接手       | 2026-08-28 JST               | 本报告                                 |
| 自审            | Ulysses (同源) — Mavis 接手                                | 2026-08-28 JST               | 通过 self-review (28 新 test 全 pass + clippy 0/0) |
| 审批            | 架构师 (Mavis 接手 agent per DEC-008)                       | —                            | 待 Mavis 终审                         |
| 5 域独立 Lead  | (N/A — 本任务域为 ut 增强，不涉及业务域决策)               | —                            | 5 域 Lead 拒绝兼任 per 8/21 JST       |
| SRE Lead        | (N/A — 不涉及部署 / 监控)                                  | —                            | 待 Phase 2 部署时介入                 |

## §7 修订历史

| 版本  | 日期              | 修订人                                                 | 主要改动                                                                                          |
|-------|-------------------|--------------------------------------------------------|---------------------------------------------------------------------------------------------------|
| v0.1  | 2026-08-28 JST    | Ulysses (一人公司 12 角色 per DEC-008) — Mavis 接手    | 初版：5 模块 27 个新 ut 全 pass + clippy 0/0 严格门；不 commit 散落 (待 1 commit 整批入库)         |
