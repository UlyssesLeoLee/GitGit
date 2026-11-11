//! Collect git input for the `gitai` subcommands.
//!
//! Reuses the `git` subprocess wrapper rather than re-implementing diff
//! parsing, consistent with the archived ADR-0002 position: high-risk Git
//! facilities are delegated to the `git` binary, never reimplemented.
//!
//! Every invocation passes an explicit allow-listed argument vector. The
//! repository path and the diff range reach this module from CLI flags, so
//! a shell-string build would be an injection surface; there is no shell
//! in this path at all.

use std::path::Path;
use std::process::Stdio;

use tokio::process::Command;

use crate::error::{GitGitError, Result};

/// Diff inputs, in the order `gitai` prefers them.
///
/// Explicit ranges win over the working tree: if the operator names a
/// range, silently substituting their uncommitted changes would answer a
/// different question than the one they asked.
#[derive(Debug, Clone, Default)]
pub struct DiffRequest {
    /// An explicit revision range, e.g. `HEAD~1..HEAD`.
    pub range: Option<String>,
    /// Include uncommitted working-tree changes.
    pub include_worktree: bool,
    /// Restrict to these paths.
    pub paths: Vec<String>,
}

/// Run `git <args>` in `repo` and return stdout, or a typed error.
///
/// The error carries git's own stderr, which is what actually tells an
/// operator whether they typo'd a revision or are not in a repository.
async fn git_output(repo: &Path, args: &[&str]) -> Result<String> {
    let mut cmd = Command::new("git");
    cmd.args(args);
    cmd.current_dir(repo);
    #[cfg(windows)]
    {
        // Without this, Windows pops a console window per invocation.
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        use std::os::windows::process::CommandExt;
        cmd.as_std_mut().creation_flags(CREATE_NO_WINDOW);
    }
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let rendered = format!("git {}", args.join(" "));
    let out = cmd
        .output()
        .await
        .map_err(|source| GitGitError::GitSubprocess {
            cmd: rendered.clone(),
            source,
        })?;

    if !out.status.success() {
        return Err(GitGitError::GitExit {
            cmd: rendered,
            status: out.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        });
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Validate that `repo` is inside a git work tree.
///
/// Called before any diff so the failure message is "not a repository"
/// rather than a confusing `fatal: bad revision 'HEAD~1'` from a nested
/// directory that happens to have a `.git` file far above it.
pub async fn ensure_repository(repo: &Path) -> Result<()> {
    if !repo.is_dir() {
        return Err(GitGitError::Ai(format!(
            "not a directory: {}",
            repo.display()
        )));
    }
    git_output(repo, &["rev-parse", "--is-inside-work-tree"]).await?;
    Ok(())
}

/// Produce the diff text for a [`DiffRequest`].
///
/// Returns an error rather than an empty string when there is nothing to
/// send: an empty diff handed to a model produces a confident commit
/// message describing no change, which is worse than an explicit failure.
pub async fn collect_diff(repo: &Path, req: &DiffRequest) -> Result<String> {
    ensure_repository(repo).await?;

    let mut parts: Vec<String> = Vec::new();

    if let Some(range) = &req.range {
        if !range.starts_with('-') {
            // A leading `-` would make git read the next argument as an
            // option; `--` terminates option parsing.
            let mut args = vec!["diff", range.as_str()];
            push_pathspecs(&mut args, &req.paths);
            let text = git_output(repo, &args).await?;
            if !text.trim().is_empty() {
                parts.push(text);
            }
        } else {
            return Err(GitGitError::Ai(format!(
                "invalid revision range `{range}`: must not start with `-`"
            )));
        }
    }

    if req.include_worktree {
        let mut args = vec!["diff", "HEAD"];
        push_pathspecs(&mut args, &req.paths);
        let text = git_output(repo, &args).await?;
        if !text.trim().is_empty() {
            parts.push(text);
        }
    }

    if parts.is_empty() {
        return Err(GitGitError::Ai(
            "no changes to send: the diff is empty. Stage or commit something, \
             or pass an explicit range."
                .to_string(),
        ));
    }

    Ok(parts.join("\n"))
}

fn push_pathspecs<'a>(args: &mut Vec<&'a str>, paths: &'a [String]) {
    if paths.is_empty() {
        return;
    }
    args.push("--");
    args.extend(paths.iter().map(String::as_str));
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn unique_temp(label: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "gitgit-test-diff-{label}-{}-{n}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A throwaway repo with one commit, used by the tests below. Skipped
    /// when `git` is unavailable so the suite does not fail on a machine
    /// without it.
    fn repo_with_commit(label: &str) -> Option<PathBuf> {
        let dir = unique_temp(label);
        let run = |args: &[&str]| -> bool {
            let ok = std::process::Command::new("git")
                .args(args)
                .current_dir(&dir)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            ok
        };
        if !run(&["init", "-q"]) {
            return None;
        }
        // Identity must be set per-repo: the ambient global config may be
        // absent on CI, and `commit` fails without it.
        if !run(&["config", "user.email", "t@example.invalid"])
            || !run(&["config", "user.name", "t"])
        {
            return None;
        }
        std::fs::write(dir.join("a.txt"), "hello\n").unwrap();
        if !run(&["add", "."]) || !run(&["commit", "-q", "-m", "first"]) {
            return None;
        }
        Some(dir)
    }

    #[tokio::test]
    async fn non_repository_is_rejected_with_a_clear_error() {
        let dir = unique_temp("not-a-repo");
        let err = ensure_repository(&dir).await.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("rev-parse") || msg.contains("not a git repository"),
            "unhelpful error: {msg}"
        );
    }

    #[tokio::test]
    async fn missing_directory_is_rejected() {
        let err = ensure_repository(Path::new("/definitely/not/here"))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("not a directory"), "got: {err}");
    }

    #[tokio::test]
    async fn empty_diff_is_an_error_not_empty_string() {
        // The failure this guards: an empty diff produces a confident
        // commit message describing nothing, which is worse than a clear
        // refusal.
        let Some(dir) = repo_with_commit("empty") else {
            return;
        };
        let req = DiffRequest {
            range: None,
            include_worktree: false,
            paths: vec![],
        };
        let err = collect_diff(&dir, &req).await.unwrap_err();
        assert!(err.to_string().contains("no changes"), "got: {err}");
    }

    #[tokio::test]
    async fn worktree_diff_captures_an_uncommitted_edit() {
        let Some(dir) = repo_with_commit("worktree") else {
            return;
        };
        std::fs::write(dir.join("a.txt"), "hello\nworld\n").unwrap();
        let req = DiffRequest {
            range: None,
            include_worktree: true,
            paths: vec![],
        };
        let diff = collect_diff(&dir, &req).await.unwrap();
        assert!(diff.contains("+world"), "got: {diff}");
    }

    #[tokio::test]
    async fn range_starting_with_dash_is_rejected() {
        let Some(dir) = repo_with_commit("dash") else {
            return;
        };
        let req = DiffRequest {
            range: Some("--upload-pack=evil".to_string()),
            include_worktree: false,
            paths: vec![],
        };
        let err = collect_diff(&dir, &req).await.unwrap_err();
        assert!(
            err.to_string().contains("must not start with"),
            "got: {err}"
        );
    }

    #[test]
    fn pathspecs_are_terminated() {
        // Without `--`, a path named like a git option would be parsed as
        // one.
        let paths = vec!["--upload-pack=x".to_string()];
        let mut args = vec!["diff", "HEAD"];
        push_pathspecs(&mut args, &paths);
        assert_eq!(args, vec!["diff", "HEAD", "--", "--upload-pack=x"]);
    }
}
