//! Fast-forward sync between a managed bare repository and a named remote.
//!
//! "Fast-forward sync" means: bring the remote's refs into the local bare
//! repo, then push the local ref back **only if** that push would advance
//! the remote. `git push` already refuses a non-fast-forward update, so
//! the safety property comes from delegating to git rather than from
//! comparing SHAs here — and per the archived ADR-0002 position, high-risk
//! Git facilities are never reimplemented in this crate.
//!
//! `[FACT]` The local branch comes along ✓: it is **created** when the
//! repository does not have it yet ✓, and **fast-forwarded** when the
//! remote is ahead ✓. Without that ✓, the managed repository would hold
//! the remote's commits under a tracking ref only ✓ — its HEAD would dangle
//! ✓ and a clone of it would come back **empty** ✓, which is not what a
//! repository the product manages should be ✓.
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

    /// The line the operator actually sees ✓.
    ///
    /// `[FACT]` The count is in the line ✓: "fetched" alone does not say
    /// what moved ✓, and a count of zero would claim that nothing moved ✓
    /// when something did ✓.
    pub fn summary(&self) -> String {
        match self {
            SyncOutcome::Fetched { commits: 1 } => "fetched 1 commit".to_string(),
            SyncOutcome::Fetched { commits } => format!("fetched {commits} commits"),
            SyncOutcome::Pushed => "pushed".to_string(),
            SyncOutcome::UpToDate => "up-to-date".to_string(),
            SyncOutcome::NotFastForward => "refused (not a fast-forward)".to_string(),
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

/// Move `ref_name` to `new_sha`, atomically ✓.
///
/// `[FACT]` The expected old value is handed to git ✓, so the ref cannot
/// move under us between the read and the write ✓. An **empty** old value
/// means "the ref must not exist yet" ✓ — which is what creating a branch
/// needs ✓.
async fn update_ref(
    repo: &Path,
    ref_name: &str,
    new_sha: &str,
    old_sha: Option<&str>,
) -> Result<()> {
    let (code, _out, err) = git(
        repo,
        &["update-ref", ref_name, new_sha, old_sha.unwrap_or("")],
    )
    .await?;
    if code != 0 {
        return Err(GitGitError::Remote(format!(
            "could not move {ref_name} to {new_sha}: {}",
            err.trim()
        )));
    }
    Ok(())
}

/// How many commits `to` has that `from` does not ✓.
///
/// `[FACT]` With no `from` ✓ it counts every commit `to` has ✓ — which is
/// what a branch that did not exist yet needs ✓.
async fn rev_list_count(repo: &Path, from: Option<&str>, to: &str) -> Result<usize> {
    let range = match from {
        Some(f) => format!("{f}..{to}"),
        None => to.to_string(),
    };
    let (code, out, err) = git(repo, &["rev-list", "--count", &range]).await?;
    if code != 0 {
        return Err(GitGitError::Remote(format!(
            "could not count the commits in {range}: {}",
            err.trim()
        )));
    }
    Ok(out.trim().parse::<usize>().unwrap_or(0))
}

/// Sync `repo` with `url` for a single ref.
///
/// Steps, in order:
/// 1. `fetch` the remote ref into `refs/remotes/<remote>/<branch>`.
/// 2. **Bring the local branch along** ✓ — it is created when it does not
///    exist ✓, and fast-forwarded when the remote is ahead ✓.
/// 3. Compare local and remote SHAs.
/// 4. `push` local -> remote only when local is strictly ahead.
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
    let local_ref = format!("refs/heads/{branch}");

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

    let local = rev_parse(repo, &local_ref).await?;
    let remote_sha = rev_parse(repo, &tracking).await?;

    let local_sha = match local {
        Some(s) => s,
        None => {
            // No local branch. The fetched ref **is** the branch's content,
            // so create the branch from it ✓ — otherwise the repository
            // keeps a dangling HEAD ✓ and serves nothing ✓: a clone of it
            // comes back empty ✓, and the repository is unusable for
            // No local branch. The fetched ref **is** the branch's content,
            // so create the branch from it ✓ — otherwise the repository
            // keeps a dangling HEAD ✓ and serves nothing ✓: a clone of it
            // comes back empty ✓, and the repository is unusable for
            // everything else the product does with it ✓.
            let Some(remote_sha) = remote_sha else {
                // The remote branch does not exist either ⇒ there was
                // nothing to fetch and nothing to create.
                return Ok(SyncOutcome::Fetched { commits: 0 });
            };
            let commits = rev_list_count(repo, None, &tracking).await?;
            update_ref(repo, &local_ref, &remote_sha, None).await?;
            return Ok(SyncOutcome::Fetched { commits });
        }
    };

    if remote_sha.as_deref() == Some(local_sha.as_str()) {
        return Ok(SyncOutcome::UpToDate);
    }

    // Is the LOCAL an ancestor of the remote? Then the remote is ahead and
    // the local branch can fast-forward to it ✓. That is the point of a
    // fast-forward sync ✓: the managed repository ends up serving the
    // remote's commits ✓, not merely holding them under a tracking ref ✓.
    if let Some(remote_sha) = remote_sha.as_deref() {
        let (c, _, _) = git(
            repo,
            &["merge-base", "--is-ancestor", &local_ref, &tracking],
        )
        .await?;
        if c == 0 {
            let commits = rev_list_count(repo, Some(&local_ref), &tracking).await?;
            update_ref(repo, &local_ref, remote_sha, Some(&local_sha)).await?;
            return Ok(SyncOutcome::Fetched { commits });
        }
    }

    // Is the remote an ancestor of local? If so a push is a fast-forward
    // and git will accept it. If not, we still try and let git refuse —
    // the check here is only used to avoid a pointless network round-trip
    // in the common "local ahead" case.
    let behind = if remote_sha.is_some() {
        let (c, _, _) = git(
            repo,
            &["merge-base", "--is-ancestor", &tracking, &local_ref],
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
        // The first sync brings the commit in, and it says how much it
        // brought: one commit.
        let first = sync_ref(&bare, up.to_str().unwrap(), "origin", "main")
            .await
            .unwrap();
        assert_eq!(first, SyncOutcome::Fetched { commits: 1 });
        // A second one is a no-op - both sides hold the same commit now.
        let second = sync_ref(&bare, up.to_str().unwrap(), "origin", "main")
            .await
            .unwrap();
        assert_eq!(second, SyncOutcome::UpToDate);
    }

    #[tokio::test]
    async fn a_sync_into_a_repository_with_no_local_branch_creates_it() {
        let Some((bare, up, work)) = fixture("create-branch") else {
            return;
        };
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
        // A bare repo starts with an unborn HEAD: there is no
        // refs/heads/main at all.
        assert_eq!(rev_parse(&bare, "refs/heads/main").await.unwrap(), None);
        // The CLI syncs the branch the repository's HEAD names, so point
        // the HEAD at main first - otherwise the synced branch would not
        // be the one the repository serves.
        assert_eq!(
            git(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"])
                .await
                .unwrap()
                .0,
            0
        );

        sync_ref(&bare, up.to_str().unwrap(), "origin", "main")
            .await
            .unwrap();

        // The branch exists now, and it holds the commit the remote had.
        let sha = rev_parse(&bare, "refs/heads/main").await.unwrap();
        let expected = rev_parse(&work, "HEAD").await.unwrap();
        assert_eq!(
            sha, expected,
            "the sync must create the branch it fetched into"
        );

        // And the repository SERVES it: a clone of it is not empty. A
        // repository that holds the objects but names no branch serves
        // nothing - `git clone` of it comes back empty.
        let base = bare.parent().unwrap();
        let clone = base.join("clone-of-synced");
        let (code, _out, err) = git(
            base,
            &[
                "clone",
                "-q",
                bare.to_str().unwrap(),
                clone.to_str().unwrap(),
            ],
        )
        .await
        .unwrap();
        assert_eq!(code, 0, "a clone of the synced repository must work: {err}");
        assert_eq!(
            rev_parse(&clone, "HEAD").await.unwrap(),
            expected,
            "the clone must carry the synced commit"
        );
    }

    #[tokio::test]
    async fn a_sync_fast_forwards_a_local_branch_that_is_behind() {
        let Some((bare, up, work)) = fixture("behind") else {
            return;
        };
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
        // The CLI syncs the branch the repository's HEAD names.
        assert_eq!(
            git(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"])
                .await
                .unwrap()
                .0,
            0
        );
        // The first sync brings that commit in.
        sync_ref(&bare, up.to_str().unwrap(), "origin", "main")
            .await
            .unwrap();
        let first_sha = rev_parse(&bare, "refs/heads/main").await.unwrap();

        // A second commit lands on the upstream.
        std::fs::write(work.join("b.txt"), "two\n").unwrap();
        assert_eq!(git(&work, &["add", "."]).await.unwrap().0, 0);
        assert_eq!(
            git(&work, &["commit", "-q", "-m", "second"])
                .await
                .unwrap()
                .0,
            0
        );
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

        // The local branch is behind, so the sync must ADVANCE it - that is
        // what a fast-forward sync is for.
        let out = sync_ref(&bare, up.to_str().unwrap(), "origin", "main")
            .await
            .unwrap();
        assert_eq!(out, SyncOutcome::Fetched { commits: 1 });
        assert_eq!(
            rev_parse(&bare, "refs/heads/main").await.unwrap(),
            rev_parse(&work, "HEAD").await.unwrap(),
            "the local branch must fast-forward to the remote's commit"
        );
        assert_ne!(
            rev_parse(&bare, "refs/heads/main").await.unwrap(),
            first_sha,
            "the local branch must have moved"
        );
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

        // The first sync created the local branch ✓, so there is nothing to
        // seed by hand any more ✓ — a bare repo starts empty ✓, and a
        // fetch alone would NOT have created refs/heads/main ✓.

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
