//! Repository-related commands: list, inspect, refs, log, clone URL,
//! open in system file manager.
//!
//! The git operations (refs, log) shell out to the system `git` binary
//! via `tokio::process::Command` so we honour ADR-0002 (no pure-Rust
//! libgit2 / gix bindings). Output parsing is intentionally tolerant —
//! the UI falls back to "Unknown" when fields are missing.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::Serialize;
use tauri::State;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

use crate::error::{AppError, AppResult};
use crate::state::DesktopState;

/// Lightweight descriptor returned to the UI list.
#[derive(Debug, Clone, Serialize)]
pub struct RepoSummary {
    pub name: String,
    pub path: String,
    pub default_branch: String,
    pub size_bytes: u64,
}

/// Detailed descriptor with refs and the most-recent commits.
#[derive(Debug, Clone, Serialize)]
pub struct RepoDetail {
    pub name: String,
    pub path: String,
    pub default_branch: String,
    pub refs: Vec<RefEntry>,
    pub commits: Vec<CommitEntry>,
}

/// One row in `git show-ref`.
#[derive(Debug, Clone, Serialize)]
pub struct RefEntry {
    pub sha: String,
    pub name: String,
    pub kind: String, // "local" / "remote" / "tag"
}

/// One row in `git log --pretty=...`.
#[derive(Debug, Clone, Serialize)]
pub struct CommitEntry {
    pub sha: String,
    pub short_sha: String,
    pub author: String,
    pub email: String,
    pub message: String,
    pub date_iso: String,
}

/// Working-tree status, as sent to the UI.
///
/// Mirrors `gitgit::repo::status::WorktreeStatus`; the desktop owns the
/// wire shape, as it does for [`RepoSummary`] and [`RepoDetail`].
#[derive(Debug, Clone, Serialize)]
pub struct RepoStatus {
    pub branch: Option<String>,
    pub head: Option<String>,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub entries: Vec<StatusEntry>,
    /// `entries` is empty. A clean tree is an answer, not an error.
    pub is_clean: bool,
}

/// One changed path in the status list.
#[derive(Debug, Clone, Serialize)]
pub struct StatusEntry {
    pub path: String,
    pub orig_path: Option<String>,
    /// `"M"`, `"A"`, … or `null` when the column is blank.
    pub index_status: Option<String>,
    pub worktree_status: Option<String>,
    pub staged: bool,
    pub unstaged: bool,
    pub untracked: bool,
}

/// Diff text plus the metadata the UI needs to label it honestly.
#[derive(Debug, Clone, Serialize)]
pub struct RepoDiff {
    /// `"staged"`, `"worktree"` or `"head"`.
    pub target: String,
    pub path: Option<String>,
    pub text: String,
    pub truncated: bool,
    /// True when the path is untracked and the text was synthesized,
    /// because `git diff` reports nothing for such a file.
    pub untracked: bool,
    pub files: usize,
}

/// List the bare repos under `state.repos_dir`. Delegates to
/// `gitgit::repo::list_repos` so we share the discovery code with the
/// CLI's `gitgit list`.
#[tauri::command]
pub fn list_repos(state: State<'_, DesktopState>) -> AppResult<Vec<RepoSummary>> {
    let names =
        gitgit::repo::list_repos(&state.repos_dir).map_err(|e| AppError::Gitgit(format!("{e}")))?;

    let mut out = Vec::with_capacity(names.len());
    for name in names {
        let path = state.repos_dir.join(format!("{name}.git"));
        let default_branch = read_default_branch(&path).unwrap_or_else(|_| String::from("main"));
        let size_bytes = dir_size_bytes(&path).unwrap_or(0);
        out.push(RepoSummary {
            name,
            path: path.to_string_lossy().into_owned(),
            default_branch,
            size_bytes,
        });
    }
    Ok(out)
}

/// Return refs + the last `limit` commits for `name`.
#[tauri::command]
pub async fn repo_detail(
    state: State<'_, DesktopState>,
    name: String,
    limit: Option<usize>,
) -> AppResult<RepoDetail> {
    let path = state.repos_dir.join(format!("{name}.git"));
    if !path.join("HEAD").is_file() {
        return Err(AppError::InvalidRepoName(name));
    }
    let default_branch = read_default_branch(&path).unwrap_or_else(|_| String::from("main"));
    let refs = read_refs(&path).await.unwrap_or_default();
    let commits = read_log(&path, limit.unwrap_or(20))
        .await
        .unwrap_or_default();
    Ok(RepoDetail {
        name,
        path: path.to_string_lossy().into_owned(),
        default_branch,
        refs,
        commits,
    })
}

/// Compose a clone URL for `name` against the embedded server. The
/// `auth` flag embeds the admin password from the vault — V0.1 only
/// (the embedded server runs over plain HTTP, so a transport-level
/// safer scheme is V1 work).
#[tauri::command]
pub async fn clone_url(
    _state: State<'_, DesktopState>,
    name: String,
    server_bind: Option<String>,
) -> AppResult<String> {
    let bind = server_bind.unwrap_or_else(|| String::from(crate::state::DEFAULT_BIND));
    // We deliberately omit credential embedding at this layer. The
    // URL alone is enough for `git clone <url>` to work; the embedded
    // server treats reads as open per `auth::require_basic` semantics.
    //
    // The scheme is unconditionally http, not chosen by host: the embedded
    // server speaks plain HTTP in V0 (see the note above this command).
    // An earlier version branched on whether the bind was loopback, but
    // both arms returned "http" -- a decision that looked real and decided
    // nothing. TLS terminates at V1, at the proxy, never in here.
    Ok(format!("http://{bind}/repos/{name}.git"))
}

/// Open the repo directory in the host file manager. Resolution:
/// Windows → `explorer`, macOS → `open`, Linux → `xdg-open`.
#[tauri::command]
pub async fn open_repo_in_shell(state: State<'_, DesktopState>, name: String) -> AppResult<()> {
    let path = state.repos_dir.join(format!("{name}.git"));
    if !path.is_dir() {
        return Err(AppError::InvalidRepoName(name));
    }
    open_in_file_manager(&path).await
}

// --- working-tree status and diff ---------------------------------------

/// Resolve a repository name to a working-tree directory.
///
/// The name goes through `gitgit::config::validate_repo_name` — the same
/// rule the HTTP layer applies, and the one its `..%2Fescape` case
/// tests — *before* any filesystem access or subprocess spawn, so a
/// traversal attempt never becomes a `git -C <path>` argument. This is
/// the first statement of both status commands for that reason.
///
/// `root` defaults to the app's own `repos` directory. A caller may pass
/// a different root to inspect a checkout elsewhere; the name is still
/// validated identically, and the join can only ever produce a direct
/// child of `root` because `..`, `/`, `\`, a leading `.` and whitespace
/// are all rejected upstream.
fn resolve_worktree_dir(repos_dir: &Path, name: &str, root: Option<String>) -> AppResult<PathBuf> {
    let base = match root {
        Some(r) if !r.trim().is_empty() => PathBuf::from(r),
        _ => repos_dir.to_path_buf(),
    };
    let path = gitgit::repo::resolve_worktree(&base, name)
        .map_err(|e| AppError::InvalidRepoName(e.to_string()))?;

    // The desktop's own repositories are bare, stored as `<name>.git` by
    // `create_bare_repo`. A bare repository is a real answer with a
    // different shape rather than a git failure — `git status` there
    // exits 128 — so it gets its own message instead of being passed up
    // as a confusing raw stderr.
    if !path.is_dir() {
        let mut bare = path.clone();
        if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
            bare.set_file_name(format!("{file_name}.git"));
        }
        if gitgit::repo::is_bare_repo(&bare) {
            return Err(AppError::NotAWorkTree(bare.to_string_lossy().into_owned()));
        }
        return Err(AppError::InvalidRepoName(name.to_string()));
    }
    if gitgit::repo::is_bare_repo(&path) {
        return Err(AppError::NotAWorkTree(path.to_string_lossy().into_owned()));
    }
    Ok(path)
}

/// Working-tree status for `name`.
///
/// A repository with no changes returns `is_clean: true` with an empty
/// `entries` list, which the UI renders as "clean" rather than as an
/// error.
#[tauri::command]
pub async fn repo_status(
    state: State<'_, DesktopState>,
    name: String,
    root: Option<String>,
) -> AppResult<RepoStatus> {
    let dir = resolve_worktree_dir(&state.repos_dir, &name, root)?;
    let status = gitgit::repo::worktree_status(&dir)
        .await
        .map_err(|e| AppError::Git(e.to_string()))?;
    Ok(RepoStatus {
        branch: status.branch,
        head: status.head,
        upstream: status.upstream,
        ahead: status.ahead,
        behind: status.behind,
        is_clean: status.is_clean,
        entries: status
            .entries
            .into_iter()
            .map(|e| {
                // Derived before the fields are moved out: `is_staged`
                // and `is_unstaged` borrow `e`.
                let (staged, unstaged) = (e.is_staged(), e.is_unstaged());
                let untracked = e.untracked;
                StatusEntry {
                    path: e.path,
                    orig_path: e.orig_path,
                    index_status: e.index_status.map(|c| c.to_string()),
                    worktree_status: e.worktree_status.map(|c| c.to_string()),
                    staged,
                    unstaged,
                    untracked,
                }
            })
            .collect(),
    })
}

/// Diff text for `name`.
///
/// `target` selects which of the three different questions is being asked
/// (`staged` = index vs HEAD, `worktree` = work tree vs index, `head` =
/// work tree vs HEAD) and an unknown value is refused rather than being
/// silently treated as one of them. `path` scopes the diff to one
/// repository-relative path.
#[tauri::command]
pub async fn repo_diff(
    state: State<'_, DesktopState>,
    name: String,
    target: String,
    path: Option<String>,
    root: Option<String>,
) -> AppResult<RepoDiff> {
    let target = gitgit::repo::DiffTarget::parse(&target)
        .map_err(|_| AppError::InvalidDiffTarget(target.clone()))?;
    let dir = resolve_worktree_dir(&state.repos_dir, &name, root)?;
    let payload =
        gitgit::repo::worktree_diff(&dir, target, path.as_deref(), gitgit::repo::MAX_DIFF_BYTES)
            .await
            .map_err(|e| AppError::Git(e.to_string()))?;
    Ok(RepoDiff {
        target: payload.target.as_str().to_string(),
        path: payload.path,
        text: payload.text,
        truncated: payload.truncated,
        untracked: payload.untracked,
        files: payload.files,
    })
}

// --- pure helpers ------------------------------------------------------

fn read_default_branch(repo: &Path) -> std::io::Result<String> {
    let head = std::fs::read_to_string(repo.join("HEAD"))?;
    let head = head.trim_start_matches("ref:").trim();
    head.rsplit('/')
        .next()
        .map(|s| s.to_string())
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "HEAD is empty"))
}

fn dir_size_bytes(path: &Path) -> std::io::Result<u64> {
    let mut total: u64 = 0;
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        if ft.is_dir() {
            total += dir_size_bytes(&entry.path())?;
        } else {
            total += entry.metadata()?.len();
        }
    }
    Ok(total)
}

async fn read_refs(repo: &PathBuf) -> std::io::Result<Vec<RefEntry>> {
    let mut cmd = Command::new("git");
    cmd.arg("show-ref");
    cmd.arg("--dereference");
    cmd.current_dir(repo);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let output = cmd.output().await?;
    if !output.status.success() {
        return Err(std::io::Error::other(format!(
            "git show-ref exited with {:?}",
            output.status.code()
        )));
    }
    let mut entries = Vec::new();
    let s = String::from_utf8_lossy(&output.stdout);
    for line in s.lines() {
        let mut parts = line.split_whitespace();
        let Some(sha) = parts.next() else { continue };
        let Some(name) = parts.next() else { continue };
        let kind = if name.starts_with("refs/heads/") {
            "local"
        } else if name.starts_with("refs/remotes/") {
            "remote"
        } else if name.starts_with("refs/tags/") {
            "tag"
        } else {
            "other"
        };
        entries.push(RefEntry {
            sha: sha.to_string(),
            name: name.to_string(),
            kind: kind.to_string(),
        });
    }
    Ok(entries)
}

async fn read_log(repo: &PathBuf, limit: usize) -> std::io::Result<Vec<CommitEntry>> {
    let fmt = "%H%x1f%h%x1f%an%x1f%ae%x1f%ad%x1f%s";
    let mut cmd = Command::new("git");
    cmd.arg("log");
    cmd.arg("--all");
    cmd.arg(format!("--pretty=format:{fmt}"));
    cmd.arg("--date=iso-strict");
    cmd.arg("-n");
    cmd.arg(limit.to_string());
    cmd.current_dir(repo);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let output = cmd.output().await?;
    if !output.status.success() {
        return Err(std::io::Error::other(format!(
            "git log exited with {:?}",
            output.status.code()
        )));
    }
    let s = String::from_utf8_lossy(&output.stdout);
    let mut out = Vec::new();
    for line in s.lines() {
        let cols: Vec<&str> = line.split('\u{1f}').collect();
        if cols.len() < 6 {
            continue;
        }
        out.push(CommitEntry {
            sha: cols[0].to_string(),
            short_sha: cols[1].to_string(),
            author: cols[2].to_string(),
            email: cols[3].to_string(),
            date_iso: cols[4].to_string(),
            message: cols[5].to_string(),
        });
    }
    Ok(out)
}

async fn open_in_file_manager(path: &PathBuf) -> AppResult<()> {
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| AppError::Bridge(format!("explorer: {e}")))?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| AppError::Bridge(format!("open: {e}")))?;
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| AppError::Bridge(format!("xdg-open: {e}")))?;
        return Ok(());
    }
    #[allow(unreachable_code)]
    Err(AppError::Bridge("unsupported platform".into()))
}

#[allow(dead_code)]
async fn read_string_from(mut reader: tokio::process::ChildStdout) -> String {
    let mut buf = String::new();
    let _ = reader.read_to_string(&mut buf).await;
    buf
}
