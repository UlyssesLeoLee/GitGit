//! Wrapper around `tokio::process::Command` for shelling out to the `git` CLI.
//!
//! Per ADR-0002 (in the archived docs), the MVP **never** re-implements
//! high-risk Git facilities (pack generation, pack parsing, ref negotiation)
//! in pure Rust. We spawn `git` directly and stream its output back to the
//! HTTP client.

use std::path::Path;
use std::process::Stdio;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::process::{Child, Command};

use crate::error::{GitGitError, Result};

/// `git init --bare` against the given target path.
pub async fn git_init_bare(path: &Path) -> Result<()> {
    run_capture(
        "git",
        &["init", "--bare", &path.display().to_string()],
        None,
    )
    .await?;
    Ok(())
}

/// Returned by [`git_stateless_rpc`]: the running child plus its stdin pipe.
pub struct GitChild {
    /// The child process. Killing it cancels the in-flight RPC.
    pub child: Child,
    /// Stdin of the child, for streaming the request body.
    pub stdin: Box<dyn AsyncWrite + Unpin + Send>,
    /// Stdout of the child, for streaming the response body.
    pub stdout: Box<dyn AsyncRead + Unpin + Send>,
}

/// Spawn `git <subcmd> --stateless-rpc <repo>` with the repo as cwd.
///
/// `subcmd` is one of `upload-pack` or `receive-pack`. The caller streams
/// the request body to the returned stdin and copies the returned stdout to
/// the HTTP response.
pub fn git_stateless_rpc(
    subcmd: &str,
    repo: &Path,
    advertise: bool,
    extra_args: &[&str],
) -> Result<GitChild> {
    // Validate the subcommand against a fixed allow-list. We never let the
    // caller pass arbitrary CLI flags through to `git` here.
    match subcmd {
        "upload-pack" | "receive-pack" => {}
        other => {
            return Err(GitGitError::Http(format!(
                "unsupported git subcommand: {other}"
            )));
        }
    }

    let mut cmd = Command::new("git");
    cmd.arg(subcmd).arg("--stateless-rpc");
    if advertise {
        cmd.arg("--advertise-refs");
    }
    for a in extra_args {
        cmd.arg(a);
    }
    // The repository path is given as the final positional argument.
    cmd.arg(repo.display().to_string());

    // Hide the console window that Windows would otherwise pop up.
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        apply_no_window(&mut cmd, CREATE_NO_WINDOW);
    }

    cmd.current_dir(repo)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let mut child = cmd.spawn().map_err(|source| GitGitError::GitSubprocess {
        cmd: format!("git {subcmd} --stateless-rpc"),
        source,
    })?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| GitGitError::Http("git stdin unavailable".to_string()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| GitGitError::Http("git stdout unavailable".to_string()))?;

    Ok(GitChild {
        child,
        stdin: Box::new(stdin),
        stdout: Box::new(stdout),
    })
}

/// Drain a child's stderr, return non-zero exit info on failure.
///
/// Used by capture-style helpers (currently only `git init --bare`).
pub async fn await_success(mut child: Child, cmd: &str) -> Result<()> {
    let status = child.wait().await.map_err(|source| GitGitError::GitSubprocess {
        cmd: cmd.to_string(),
        source,
    })?;
    if !status.success() {
        let mut stderr = String::new();
        if let Some(mut s) = child.stderr.take() {
            use tokio::io::AsyncReadExt;
            let _ = s.read_to_string(&mut stderr).await;
        }
        return Err(GitGitError::GitExit {
            cmd: cmd.to_string(),
            status: status.code().unwrap_or(-1),
            stderr,
        });
    }
    Ok(())
}

async fn run_capture(cmd: &str, args: &[&str], cwd: Option<&Path>) -> Result<std::process::Output> {
    let mut c = Command::new(cmd);
    c.args(args);
    if let Some(dir) = cwd {
        c.current_dir(dir);
    }
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        apply_no_window(&mut c, CREATE_NO_WINDOW);
    }
    c.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|source| GitGitError::GitSubprocess {
            cmd: format!("{cmd} {}", args.join(" ")),
            source,
        })
}

#[cfg(windows)]
fn apply_no_window(cmd: &mut Command, flags: u32) {
    use std::os::windows::process::CommandExt;
    cmd.as_std_mut().creation_flags(flags);
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
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!(
            "gitgit-test-subproc-{label}-{pid}-{n}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn git_stateless_rpc_rejects_unknown_subcommand() {
        // The allow-list is the security boundary: any subcmd other than
        // upload-pack / receive-pack must produce a clean Http error
        // BEFORE we spawn a process. A `git foo` would otherwise
        // potentially execute arbitrary user-supplied side.
        let dir = unique_temp("reject-sub");
        let err = match git_stateless_rpc("log", &dir, false, &[]) {
            Ok(_) => panic!("expected error for subcmd 'log'"),
            Err(e) => e,
        };
        let msg = format!("{err}");
        assert!(msg.contains("unsupported git subcommand"), "got: {msg}");
    }

    #[test]
    fn git_stateless_rpc_rejects_shadowed_subcommand() {
        // Even with the right prefix, a partial match must be rejected
        // (e.g. "upload-pack; rm -rf /"). The match is exact.
        let dir = unique_temp("reject-shadow");
        let err = match git_stateless_rpc("upload-pack; rm -rf /", &dir, false, &[]) {
            Ok(_) => panic!("expected error for subcmd 'upload-pack; rm -rf /'"),
            Err(e) => e,
        };
        let msg = format!("{err}");
        assert!(msg.contains("unsupported git subcommand"), "got: {msg}");
    }

    #[tokio::test]
    async fn git_init_bare_creates_bare_repo() {
        // This test requires the real `git` binary on PATH.
        let target = unique_temp("init-bare").join("foo.git");
        git_init_bare(&target).await.unwrap();
        // Bare repo signature: HEAD + objects/ + refs/.
        assert!(target.join("HEAD").is_file(), "HEAD missing");
        assert!(target.join("objects").is_dir());
        assert!(target.join("refs").is_dir());
    }

    #[tokio::test]
    async fn await_success_reports_nonzero_exit() {
        // Spawn a deliberately-failing command and confirm await_success
        // surfaces the exit code via GitGitError::GitExit. We use the
        // platform's `false` equivalent: on both Windows and Unix shells,
        // `cmd /C exit 7` (Windows) or `sh -c 'exit 7'` (Unix) returns 7.
        let mut cmd = if cfg!(windows) {
            let mut c = Command::new("cmd");
            c.args(["/C", "exit 7"]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", "exit 7"]);
            c
        };
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            apply_no_window(&mut cmd, CREATE_NO_WINDOW);
        }
        let child = cmd.spawn().unwrap();
        let err = await_success(child, "exit-7").await.unwrap_err();
        match err {
            GitGitError::GitExit { status, cmd, .. } => {
                assert_eq!(status, 7, "expected exit code 7, got {status}");
                assert_eq!(cmd, "exit-7");
            }
            other => panic!("expected GitExit, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn await_success_reports_success() {
        // Symmetric: a real successful command must not produce an error.
        // Use `git --version` which is universally available wherever
        // `git` is on PATH and exits 0.
        let mut cmd = Command::new("git");
        cmd.arg("--version");
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            apply_no_window(&mut cmd, CREATE_NO_WINDOW);
        }
        let child = cmd.spawn().unwrap();
        await_success(child, "git --version").await.unwrap();
    }
}
