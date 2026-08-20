# 06. Git 服务器 / Git Server

## 6.1 目标 / Purpose

实现 [基本设计 §3.1 Git Server 子系统](../basic-design/03-functional-design.md#31-git-server-子系统-git-server-subsystem) 的混合 Git 实现（CLI 子进程 + libgit2）。覆盖：HTTP/SSH 协议、混合读写、graph-aware 钩子回调、bare 仓库管理、认证。

## 6.2 模块结构

```
internal/git/
├── transport/
│   ├── http.go               # smart HTTP 协议
│   ├── ssh.go                # SSH subsystem
│   └── upload_pack.go        # 服务端 git-upload-pack
├── hybrid/
│   ├── writer.go             # 写操作 (走 CLI)
│   ├── reader.go             # 读操作 (走 libgit2)
│   ├── router.go             # 决定走哪条
│   └── migration.go          # CLI/libgit2 桥接
├── hook/
│   ├── manager.go            # 钩子安装/管理
│   ├── callback.go           # 钩子回调到 Platform API
│   └── policy_gate.go        # 钩子内的 Policy 评估
├── objects/
│   ├── repo.go               # bare 仓库管理
│   ├── lfs.go                # LFS 占位对象 (V1)
│   └── gc.go                 # GC / repack (V1)
├── auth/
│   ├── http_auth.go          # HTTP Basic / Bearer
│   ├── ssh_auth.go           # SSH 公钥
│   └── authorize.go          # 读/写权限判定
├── types/
│   ├── pack.go               # pack-protocol 数据结构
│   └── refs.go               # ref 列表
├── errors.go
└── server_test.go
```

## 6.3 写路径：CLI 子进程

**[PROPOSAL]** 所有写操作通过真实 `git` 二进制子进程完成。

```go
// [IMPL] internal/git/hybrid/writer.go

type Writer struct {
    executor *exec.Executor
    repos    *RepoStore
    hook     *hook.Manager
    policy   *policy.Engine
}

func (w *Writer) ReceivePack(ctx context.Context, repoID uuid.UUID, packData io.Reader, actor Actor) error {
    // 1. 写权限校验
    allowed, err := w.policy.Evaluate(ctx, PolicyInput{
        Subject:  actor,
        Action:   "git.push",
        Resource: ResourceRef{Type: "repository", ID: repoID.String()},
    })
    if err != nil { return err }
    if !allowed { return ErrPolicyDenied }

    // 2. 锁 repo (避免并发 push)
    repoPath := w.repos.Path(repoID)
    lockPath := filepath.Join(repoPath, "platform.lock")
    lock, err := os.OpenFile(lockPath, os.O_CREATE|os.O_EXCL|os.O_WRONLY, 0600)
    if err != nil { return ErrPushInProgress }
    defer os.Remove(lockPath)
    defer lock.Close()

    // 3. 调 git receive-pack 子进程
    cmd := w.executor.CommandContext(ctx, "git", "receive-pack", "--stateless-rpc", "--advertise-refs", repoPath)
    cmd.Stdin = packData
    output, err := cmd.CombinedOutput()
    if err != nil {
        return fmt.Errorf("git receive-pack failed: %w, output: %s", err, output)
    }

    // 4. 钩子在子进程内触发 (见 6.5)
    // 钩子失败 -> 已写入的对象需回滚
    // 策略: post-receive 失败时, ref 已被更新. 需要显式 rollback.
    // 详见 6.5.2

    // 5. 解析新 ref, 写 graph
    newRefs, err := w.parseNewRefs(ctx, repoPath)
    if err != nil { return err }
    for _, ref := range newRefs {
        // 创建 Commit Node + 与 Branch 建 Edge
        w.graphlink.SyncRef(ctx, repoID, ref)
    }

    return nil
}
```

## 6.4 读路径：libgit2

```go
// [IMPL] internal/git/hybrid/reader.go (伪代码, libgit2/gix 绑定)

```rust
// [IMPL] crates/platform-git/src/hybrid/reader.rs

pub struct GitReader {
    // 读路径优先使用纯 Rust gix 库，遇到未知/损坏/高阶特性时自动降级回退到 shell git CLI
}

impl GitReader {
    pub async fn read_blob(&self, repo_path: &Path, blob_sha: &str) -> Result<Vec<u8>, PlatformError> {
        // 1. 优先尝试 gix 纯 Rust 高性能读取
        match gix::open(repo_path) {
            Ok(repo) => {
                if let Ok(id) = gix::ObjectId::from_hex(blob_sha.as_bytes()) {
                    if let Ok(object) = repo.find_object(id) {
                        return Ok(object.data.to_vec());
                    }
                }
            }
            Err(e) => {
                tracing::warn!(error = ?e, "gix open failed, falling back to git CLI");
            }
        }

        // 2. 降级容灾路径：spawn git cat-file -p <blob_sha>
        let output = tokio::process::Command::new("git")
            .args(["-C", repo_path.to_str().unwrap(), "cat-file", "-p", blob_sha])
            .output()
            .await?;

        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err(PlatformError::GitObjectNotFound(blob_sha.to_string()))
        }
    }
}
```

**[ACCEPTED]** 读路径默认由纯 Rust `gix` 承担；写路径（push / pack negotiation / hook 调度）强约束调用系统 `git` 进程，且读路径发生解析异常时自动降级至系统 `git` CLI，杜绝协议层 500 崩溃。

## 6.5 Graph-Aware 钩子

### 6.5.1 钩子安装

**[PROPOSAL]** 平台在创建仓库时，自动在 bare 仓库的 `hooks/` 目录安装标准 Git 钩子（`pre-receive`, `post-receive`, `update`），钩子执行 `git-receive-pack` 流程后调用 Platform API。

```bash
# bare_repo/hooks/post-receive
#!/bin/sh
# 标准 Git 钩子
# Platform 注入: 把所有更新 ref 通过 stdin (line: old_sha new_sha refname)
# 然后调用 Platform 回调 API

while read oldrev newrev refname; do
    # 调 Platform 内部 API
    curl -sS -X POST "http://127.0.0.1:8080/internal/git/hook/post-receive" \
        -H "Authorization: Bearer ${PLATFORM_HOOK_TOKEN}" \
        -H "Content-Type: application/json" \
        -d "{
            \"repo_id\": \"${PLATFORM_REPO_ID}\",
            \"old_sha\": \"${oldrev}\",
            \"new_sha\": \"${newrev}\",
            \"refname\": \"${refname}\"
        }"
done
```

### 6.5.2 钩子回调

```go
// [IMPL] internal/git/hook/callback.go

type Manager struct {
    policy  *policy.Engine
    graph   *graphlink.Writer
    timeout time.Duration  // 默认 5s
}

func (m *Manager) PostReceive(ctx context.Context, req PostReceiveRequest) error {
    ctx, cancel := context.WithTimeout(ctx, m.timeout)
    defer cancel()

    // 1. 策略评估 (GRAPH-AWARE 部分, 详见 GIT-REQ-006 amendment)
    // - 此次 push 涉及的 commits 是否触发了受限分支?
    // - 是否绕过了必须的 CI?
    allowed, err := m.policy.Evaluate(ctx, PolicyInput{
        Subject:  req.Actor,
        Action:   "git.push.commit",
        Resource: ResourceRef{Type: "branch", ID: req.RefName},
        Context: map[string]any{
            "old_sha": req.OldSHA,
            "new_sha": req.NewSHA,
            "commits": req.NewCommits,
        },
    })
    if err != nil { return err }  // 失败 = fail-closed
    if !allowed { return ErrPolicyDenied }

    // 2. 同步 graph
    if isBranchCreate(req) {
        m.graph.CreateBranchNode(ctx, req.RepoID, req.RefName, req.NewSHA)
    } else if isBranchDelete(req) {
        m.graph.MarkBranchDeleted(ctx, req.RepoID, req.RefName)
    } else {
        // 普通 push: 为新 commits 创建 Node
        for _, sha := range req.NewCommits {
            m.graph.SyncCommit(ctx, req.RepoID, sha, req.RefName, req.Actor)
        }
    }

    // 3. 触发 CI
    for _, sha := range req.NewCommits {
        m.ci.TriggerOnCommit(ctx, req.RepoID, sha, req.RefName)
    }

    return nil
}
```

**[PROPOSAL]** `pre-receive` 钩子更严格（在写入前评估），`post-receive` 用于触发副作用。MVP 用 `post-receive` 即可；`pre-receive` 留 V1。

### 6.5.3 钩子失败回滚

**[PROPOSAL]** 钩子在 post-receive 失败时的回滚策略：

1. **轻量回滚：** 仅撤销新 commits (ref reset to old_sha)
2. **重量回滚：** 整 ref 不可达 (实际不用)

```bash
# 失败时回滚 ref
git update-ref "${refname}" "${oldrev}"
# 发告警 Event
curl ... /internal/events/alert
```

## 6.6 HTTP 传输（smart HTTP）

```go
// [IMPL] internal/git/transport/http.go

type HTTPHandler struct {
    repos    *RepoStore
    writer   *Writer
    reader   *Reader
    auth     *HTTPAuth
}

func (h *HTTPHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
    // URL: /git/{repo_name}.git/{service}
    // service: info/refs, git-upload-pack, git-receive-pack

    repoName := extractRepoName(r.URL.Path)
    repoID, err := h.repos.ResolveName(repoName)
    if err != nil { http.Error(w, "not found", 404); return }

    actor, err := h.auth.Authenticate(r)
    if err != nil { http.Error(w, "unauthorized", 401); return }

    path := strings.TrimPrefix(r.URL.Path, "/git/"+repoName+".git/")
    path = strings.TrimPrefix(path, "/")

    switch {
    case strings.HasPrefix(path, "info/refs"):
        h.serveInfoRefs(w, r, repoID)
    case strings.HasPrefix(path, "git-upload-pack"):
        h.serveUploadPack(w, r, repoID, actor)
    case strings.HasPrefix(path, "git-receive-pack"):
        h.serveReceivePack(w, r, repoID, actor)
    default:
        http.NotFound(w, r)
    }
}

func (h *HTTPHandler) serveReceivePack(w http.ResponseWriter, r *http.Request, repoID uuid.UUID, actor Actor) {
    if r.Method != "POST" { http.Error(w, "method not allowed", 405); return }

    w.Header().Set("Content-Type", "application/x-git-receive-pack-result")
    w.WriteHeader(200)

    err := h.writer.ReceivePack(r.Context(), repoID, r.Body, actor)
    if err != nil {
        // 写错误到 body (按 git protocol)
        fmt.Fprintf(w, "ERR %s\n", err.Error())
    }
}
```

## 6.7 SSH 传输

**[PROPOSAL]** SSH 实现较复杂，MVP 仅支持 HTTP，SSH 留 V1。结构占位：

```go
// [IMPL] internal/git/transport/ssh.go (V1)
type SSHServer struct {
    hostKey   ssh.Signer
    authorizedKeys map[string]ssh.PublicKey  // user -> key
    handler   *HTTPHandler  // 复用 smart HTTP 逻辑
}
```

## 6.8 bare 仓库管理

```go
// [IMPL] internal/git/objects/repo.go
type RepoStore struct {
    baseDir string  // {data_dir}/git/
}

func (s *RepoStore) Create(ctx context.Context, repoID uuid.UUID, name string) error {
    path := s.Path(repoID)
    if err := os.MkdirAll(filepath.Dir(path), 0700); err != nil { return err }
    cmd := exec.CommandContext(ctx, "git", "init", "--bare", path)
    if out, err := cmd.CombinedOutput(); err != nil {
        return fmt.Errorf("git init: %w, %s", err, out)
    }

    // 安装 Platform 钩子
    if err := s.installHooks(ctx, path, repoID); err != nil { return err }
    return nil
}

func (s *RepoStore) installHooks(ctx context.Context, repoPath string, repoID uuid.UUID) error {
    hooksDir := filepath.Join(repoPath, "hooks")
    postReceive := filepath.Join(hooksDir, "post-receive")

    tmpl := postReceiveTemplate
    tmpl = strings.ReplaceAll(tmpl, "${PLATFORM_REPO_ID}", repoID.String())
    tmpl = strings.ReplaceAll(tmpl, "${PLATFORM_API_URL}", apiURL())
    tmpl = strings.ReplaceAll(tmpl, "${PLATFORM_HOOK_TOKEN}", hookToken())

    return os.WriteFile(postReceive, []byte(tmpl), 0700)
}

func (s *RepoStore) Path(repoID uuid.UUID) string {
    return filepath.Join(s.baseDir, repoID.String()+".git")
}
```

## 6.9 认证

### 6.9.1 HTTP Basic

```go
// [IMPL] internal/git/auth/http_auth.go
type HTTPAuth struct {
    userRepo *UserStore
}

func (a *HTTPAuth) Authenticate(r *http.Request) (Actor, error) {
    username, password, ok := r.BasicAuth()
    if !ok { return Actor{}, ErrUnauthorized }

    user, err := a.userRepo.Authenticate(r.Context(), username, password)
    if err != nil { return Actor{}, ErrUnauthorized }
    return ActorFromUser(user), nil
}
```

### 6.9.2 SSH (V1)

**[TBD]** 完整 SSH 实现涉及：
- host key 管理
- authorized_keys 解析
- git 协议 subsystem dispatch
- 公钥 vs 密码的双因子

## 6.10 LFS 支持 (V1)

```go
// [IMPL] internal/git/objects/lfs.go (V1 占位)

type LFSBackend interface {
    // 上传: PUT /{repo}.git/info/lfs/objects/{oid}
    Upload(ctx context.Context, repoID uuid.UUID, oid string, data io.Reader) error
    // 下载: GET /{repo}.git/info/lfs/objects/{oid}
    Download(ctx context.Context, repoID uuid.UUID, oid string) (io.ReadCloser, error)
    // 检查: GET /{repo}.git/info/lfs/objects/{oid} HEAD
    Exists(ctx context.Context, repoID uuid.UUID, oid string) (bool, error)
}

// 存储后端: Local FS (MVP) | S3 (V1)
type LFSStorage struct {
    backend string  // 'fs' or 's3'
    fsRoot  string
    s3      *s3.Client
}
```

## 6.11 GC / 重打包 (V1)

```go
// [IMPL] internal/git/objects/gc.go (V1 占位)

func ScheduleGC(repoID uuid.UUID) {
    // 每仓库每周一次, 由 cron worker 触发
    // `git gc --auto --quiet`
    // 必须避开 push 时段
}

func RunGC(ctx context.Context, repoPath string) error {
    // 1. 短暂锁住 ref 更新
    // 2. `git gc --auto --prune=now --quiet`
    // 3. 解锁
    return nil
}
```

## 6.12 协议错误与状态码

| 场景 | 状态码 | 错误信息 |
|---|---|---|
| 未认证 | 401 | "authentication required" |
| 认证失败 | 401 | "invalid credentials" |
| 仓库不存在 | 404 | "repository not found" |
| 无写权限 | 403 | "push denied by policy" |
| Hook 失败 | 200 (含 ERR 行) | "ERR hook_policy_denied" |
| 仓库 lock 冲突 | 409 | "push in progress, retry" |
| 服务器错误 | 500 | "internal error" |

## 6.13 错误定义

```go
// [IMPL] internal/git/errors.go
var (
    ErrPolicyDenied       = NewError("git_policy_denied", 403, "git operation denied by policy")
    ErrHookTimeout        = NewError("git_hook_timeout", 504, "hook callback timed out (fail-closed)")
    ErrHookPolicyFail     = NewError("git_hook_policy_fail", 403, "hook policy gate failed")
    ErrRepoNotFound       = NewError("git_repo_not_found", 404, "repository not found")
    ErrRepoLockConflict   = NewError("git_repo_locked", 409, "repository is being written to")
    ErrUnauthorized       = NewError("git_unauthorized", 401, "authentication required")
    ErrPushRejected       = NewError("git_push_rejected", 422, "push rejected (pre-receive check)")
)
```

## 6.14 性能预算

| 指标 | 目标 | 备注 |
|---|---|---|
| push (100MB pack) | < 5s | 不含网络 |
| clone (1GB repo) | < 60s | 含 LFS |
| 钩子回调 | < 1s (P95) | 钩子超时 5s |
| 读取 tree (CLI) | < 100ms | 单文件 |
| 读取 tree (libgit2) | < 20ms (V1) | |

## 6.15 测试

| 测试 | 目标 |
|---|---|
| 单元 | CLI 调用, ref 解析, 钩子安装 |
| 集成 | 真实 git CLI + bare repo |
| 端到端 | `git push` → 钩子 → graph |
| 钩子 | 注入失败 → ref 回滚 |
| 性能 | k6 模拟 push 延迟 |

---

**导航 / Navigation:**
[← 05. AI 网关](05-ai-gateway.md) · [README](README.md) · [07. App 协调 →](07-app-coordination.md)
