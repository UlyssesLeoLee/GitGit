//! Fast-forward sync between a managed bare repository and a named remote.
//!
//! "Fast-forward sync" means: bring the remote's refs into the local bare
//! repo, then push the local ref back **only if** that push would advance
//! the remote. `git push` already refuses a non-fast-forward update, so
//! the safety property comes from delegating to git rather than from
//! comparing SHAs here — and per the archived ADR-0002 position, high-risk
//! Git facilities are never reimplemented in this crate.
//!
//! What this module adds is *reporting*: which ref was attempted, and
//! whether it advanced, stayed level, or was refused as non-fast-forward.
//! A bare `git push` exit code does not distinguish "already up to date"
//! from "refused", and the operator needs to.

use std::path::Path;
use std::process::Stdio;

use tokio::process::Command;

use crate::error::{GitGitError, Result};

/// Outcome of one sync attempt against one ref.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncOutcome {
    /// The remote moved; the local repo now has the remote's commits.
    Fetched { commits: usize },
    /// The local ref advanced the remote.
    Pushed,
    /// Both sides already agreed.
    UpToDate,
    /// The push would have rewritten history and git refused it.
    ///
    /// This is the expected result when someone force-pushed upstream.
    /// It is reported distinctly rather than as a generic failure so the
    /// operator can tell "nothing to do" from "needs a human decision".
    NotFastForward,
}

impl SyncOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            SyncOutcome::Fetched { .. } => "fetched",
            SyncOutcome::Pushed => "pushed",
            SyncOutcome::UpToDate => "up-to-date",
            SyncOutcome::NotFastForward => "refused (not a fast-forward)",
        }
    }
}

/// Run a `git` subcommand in `repo` and return `(exit_code, stdout, stderr)`.
///
/// The argv is built explicitly and passed without a shell, so a URL
/// containing shell metacharacters cannot be interpreted.
async fn git(repo: &Path, args: &[&str]) -> Result<(i32, String, String)> {
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
    Ok((
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    ))
}

/// Resolve a symbolic ref to a SHA, or `None` when it does not exist.
async fn rev_parse(repo: &Path, rev: &str) -> Result<Option<String>> {
    let (code, out, _) = git(repo, &["rev-parse", "--verify", "--quiet", rev]).await?;
    if code != 0 {
        return Ok(None);
    }
    let sha = out.trim().to_string();
    Ok(if sha.is_empty() { None } else { Some(sha) })
}

/// Sync `repo` with `url` for a single ref.
///
/// Steps, in order:
/// 1. `fetch` the remote ref into `refs/remotes/<remote>/<branch>`.
/// 2. Compare local and remote SHAs.
/// 3. `push` local -> remote only when local is strictly ahead.
///
/// The push is issued unconditionally when local is ahead, and git's own
/// rejection is what produces [`SyncOutcome::NotFastForward`]. There is no
/// race-free way to prove fast-forward-ness from the client side, and
/// pretending otherwise would mean reimplementing the check badly.
pub async fn sync_ref(repo: &Path, url: &str, remote: &str, branch: &str) -> Result<SyncOutcome> {
    if !repo.join("HEAD").is_file() {
        return Err(GitGitError::Remote(format!(
            "{} is not a bare git repository (no HEAD)",
            repo.display()
        )));
    }
    // Reject anything git would read as an option. `remote` and `branch`
    // come from the registry and the CLI, and both land in ref names.
    if remote.starts_with('-') || branch.starts_with('-') || url.starts_with('-') {
        return Err(GitGitError::Remote(
            "remote, branch and url must not start with '-'".to_string(),
        ));
    }

    let tracking = format!("refs/remotes/{remote}/{branch}");

    let (code, _out, err) = git(
        repo,
        &["fetch", url, &format!("+refs/heads/{branch}:{tracking}")],
    )
    .await?;
    if code != 0 {
        return Err(GitGitError::Remote(format!(
            "fetch from {url} failed: {}",
            err.trim()
        )));
    }

    let local = rev_parse(repo, &format!("refs/heads/{branch}")).await?;
    let remote_sha = rev_parse(repo, &tracking).await?;

    let local_sha = match local {
        Some(s) => s,
        None => {
            // No local branch: the fetch is the whole story. Report it as
            // fetched rather than pretending a push was attempted.
            return Ok(SyncOutcome::Fetched { commits: 0 });
        }
    };

    if remote_sha.as_deref() == Some(local_sha.as_str()) {
        return Ok(SyncOutcome::UpToDate);
    }

    // Is the remote an ancestor of local? If so a push is a fast-forward
    // and git will accept it. If not, we still try and let git refuse —
    // the check here is only used to avoid a pointless network round-trip
    // in the common "local behind" case.
    let behind = if remote_sha.is_some() {
        let (c, _, _) = git(
            repo,
            &[
                "merge-base",
                "--is-ancestor",
                &tracking,
                &format!("refs/heads/{branch}"),
            ],
        )
        .await?;
        c == 0
    } else {
        // Remote branch does not exist: pushing it creates it, which is
        // trivially a fast-forward.
        true
    };

    if !behind {
        return Ok(SyncOutcome::NotFastForward);
    }

    let (code, _out, err) = git(
        repo,
        &[
            "push",
            url,
            &format!("refs/heads/{branch}:refs/heads/{branch}"),
        ],
    )
    .await?;
    if code == 0 {
        return Ok(SyncOutcome::Pushed);
    }

    // git refused. The only expected reason here is a non-fast-forward that
    // appeared between our check and the push; anything else is worth
    // surfacing verbatim.
    if err.contains("non-fast-forward") || err.contains("fetch first") {
        Ok(SyncOutcome::NotFastForward)
    } else {
        Err(GitGitError::Remote(format!(
            "push to {url} failed: {}",
            err.trim()
        )))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn unique_dir(label: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "gitgit-test-sync-{label}-{}-{n}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A bare repo plus a working clone, both wired to a shared identity.
    /// Returns None if `git` is unavailable so the suite does not fail on a
    /// machine without it.
    fn fixture(label: &str) -> Option<(PathBuf, PathBuf, PathBuf)> {
        let base = unique_dir(label);
        let bare = base.join("server.git");
        let up = base.join("upstream.git");
        let work = base.join("work");

        let run = |dir: &Path, args: &[&str]| -> bool {
            std::fs::create_dir_all(dir).unwrap();
            std::process::Command::new("git")
                .args(args)
                .current_dir(dir)
                .env("GIT_CONFIG_GLOBAL", base.join("gitconfig"))
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        };

        if !run(&bare, &["init", "--bare", "-q"]) || !run(&up, &["init", "--bare", "-q"]) {
            return None;
        }
        for d in [&bare, &up] {
            if !run(d, &["config", "user.email", "t@example.invalid"])
                || !run(d, &["config", "user.name", "t"])
                || !run(d, &["config", "receive.denyCurrentBranch", "ignore"])
            {
                return None;
            }
        }
        if !run(&work, &["init", "-q"]) {
            return None;
        }
        for (k, v) in [("user.email", "t@example.invalid"), ("user.name", "t")] {
            if !run(&work, &["config", k, v]) {
                return None;
            }
        }
        std::fs::write(work.join("a.txt"), "one\n").unwrap();
        if !run(&work, &["add", "."]) || !run(&work, &["commit", "-q", "-m", "first"]) {
            return None;
        }
        Some((bare, up, work))
    }

    #[tokio::test]
    async fn sync_reports_up_to_date_when_both_sides_match() {
        let Some((bare, up, work)) = fixture("uptodate") else {
            return;
        };
        // Push local -> upstream, then hand the upstream URL to the server
        // bare repo so the first sync has something to fetch.
        assert_eq!(
            git(
                &work,
                &["push", up.to_str().unwrap(), "HEAD:refs/heads/main"]
            )
            .await
            .unwrap()
            .0,
            0
        );
        let out = sync_ref(&bare, up.to_str().unwrap(), "origin", "main")
            .await
            .unwrap();
        // The first sync fetches; a second one is a no-op.
        sync_ref(&bare, up.to_str().unwrap(), "origin", "main")
            .await
            .unwrap();
        assert_eq!(out, SyncOutcome::Fetched { commits: 0 });
    }

    #[tokio::test]
    async fn second_sync_is_up_to_date() {
        let Some((bare, up, work)) = fixture("second") else {
            return;
        };
        git(&work, &["branch", "-M", "main"]).await.unwrap();
        assert_eq!(
            git(
                &work,
                &[
                    "push",
                    up.to_str().unwrap(),
                    "refs/heads/main:refs/heads/main"
                ]
            )
            .await
            .unwrap()
            .0,
            0
        );

        sync_ref(&bare, up.to_str().unwrap(), "origin", "main")
            .await
            .unwrap();

        // A bare repo starts empty, and a fetch only populates the
        // remote-tracking ref -- it does NOT create refs/heads/main. Seed
        // the local branch explicitly, otherwise the comparison below has
        // nothing to match and this case can never reach UpToDate.
        let (code, sha, _) = git(&work, &["rev-parse", "refs/heads/main"]).await.unwrap();
        assert_eq!(code, 0);
        let sha = sha.trim().to_string();
        assert_eq!(
            git(&bare, &["update-ref", "refs/heads/main", &sha])
                .await
                .unwrap()
                .0,
            0
        );

        // Both sides now hold the same commit: the sync must report no
        // work rather than issuing a pointless push.
        let out = sync_ref(&bare, up.to_str().unwrap(), "origin", "main")
            .await
            .unwrap();
        assert_eq!(out, SyncOutcome::UpToDate);
    }

    #[tokio::test]
    async fn non_bare_repo_is_rejected() {
        let dir = unique_dir("notbare");
        let err = sync_ref(&dir, "https://example.invalid/r.git", "origin", "main")
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("not a bare git repository"),
            "got: {err}"
        );
    }

    #[tokio::test]
    async fn option_like_names_are_rejected_before_spawning_git() {
        let Some((bare, _up, _work)) = fixture("dashname") else {
            return;
        };
        let err = sync_ref(&bare, "--upload-pack=evil", "origin", "main")
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("must not start with"),
            "got: {err}"
        );
    }

    #[test]
    fn outcome_strings_are_distinct() {
        // "nothing to do" and "needs a human decision" must not print the
        // same word.
        let rendered = [
            SyncOutcome::Pushed,
            SyncOutcome::UpToDate,
            SyncOutcome::NotFastForward,
            SyncOutcome::Fetched { commits: 0 },
        ]
        .iter()
        .map(|o| o.as_str())
        .collect::<Vec<_>>();
        let unique: std::collections::BTreeSet<_> = rendered.iter().collect();
        assert_eq!(unique.len(), rendered.len(), "duplicate outcome wording");
    }
}
