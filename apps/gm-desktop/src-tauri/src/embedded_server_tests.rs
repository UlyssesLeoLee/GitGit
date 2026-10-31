//! The desktop app's EMBEDDED server, asked for a smart-HTTP path over a
//! real socket.
//!
//! # What this closes
//!
//! `[FACT]` The repository's plan document (`docs/plan/v0-tasks.md`, the T2
//! row) records an open `[TBD]`: *"Tauri 内嵌形态下的同一条路径未被端到端请求过"* —
//! the smart-HTTP path has never been requested in the form the product
//! actually ships.
//!
//! `[FACT]` The router itself is not untested. The library crate has an
//! end-to-end case for `GET /repos/<name>.git/info/refs?service=git-upload-pack`
//! (`src/server/http.rs`, `info_refs_happy_path_returns_advertisement`): a
//! real bare repo, a real `git upload-pack --advertise-refs`, and the
//! advertisement frame. But that case drives the router through
//! `tower::ServiceExt::oneshot` — no socket, no listener, no port.
//!
//! `[FACT]` What the desktop app actually runs is
//! [`crate::run_embedded_router`]: it resolves the admin credential from
//! the vault, enforces the exposure policy on the bind address, builds the
//! router from the resulting state, binds a listener and serves it with
//! `axum::serve`. That path — the credential resolution, the exposure
//! policy, the listener, the socket — had no test at all, and the
//! repository has been bitten by exactly that class of gap before (a
//! server whose auth surface differs from the shipped one is the comment
//! on [`crate::run_embedded_router`] itself).
//!
//! So this module asks the embedded server the question over a real
//! socket, and looks at the answer.
//!
//! # What it asserts
//!
//! Not "200 OK". The response has to BE an upload-pack advertisement, and
//! it has to carry the repository's actual commit — a route that answers
//! 200 with an empty body would satisfy a status-code assertion and prove
//! nothing.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

/// Build a unique temp directory.
///
/// `[FACT]` We cannot pull in the `tempfile` crate: the repository's
/// 0-new-external-deps rule forbids a new dependency, and the library
/// crate's own test helpers synthesise uniqueness the same way (process
/// id + an atomic counter + a nanosecond timestamp).
fn unique_temp(label: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!(
        "gitgit-desktop-embedded-{label}-{pid}-{n}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Run `git` and fail loudly if it did not succeed.
async fn git(dir: &PathBuf, args: &[&str]) -> String {
    let output = tokio::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .await
        .unwrap_or_else(|e| panic!("git {args:?} could not be started: {e}"));
    assert!(
        output.status.success(),
        "git {args:?} in {} failed: {}",
        dir.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Create `<repos_dir>/<name>.git` as a bare repo holding ONE real commit.
///
/// `[FACT]` A bare repo with no commits advertises no refs at all — the
/// advertisement would be an empty ref list, which would make the
/// assertion below vacuous. So the repo is built by committing in a
/// throwaway work repo and cloning it bare, which is also what a user
/// ends up with after `git clone --bare`.
///
/// Returns the commit's id, so the caller can insist that the
/// advertisement carries it.
async fn create_repo_with_a_commit(repos_dir: &PathBuf, name: &str) -> String {
    let work = unique_temp("repo-work");
    git(&work, &["init", "--quiet"]).await;
    git(
        &work,
        &[
            "-c",
            "user.email=embedded-test@gitgit.local",
            "-c",
            "user.name=gitgit embedded test",
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            "the commit the advertisement must carry",
        ],
    )
    .await;
    let sha = git(&work, &["rev-parse", "HEAD"]).await;

    let target = repos_dir.join(format!("{name}.git"));
    std::fs::create_dir_all(repos_dir).unwrap();
    git(
        repos_dir,
        &[
            "clone",
            "--quiet",
            "--bare",
            work.to_str().unwrap(),
            target.to_str().unwrap(),
        ],
    )
    .await;
    sha
}

/// The embedded server, asked for a smart-HTTP path over a real socket.
///
/// `[FACT]` The server is bound to `127.0.0.1:0`, so the OS picks a free
/// port — the product's own port (38080) is never taken, and a test can
/// never collide with a running copy of the app. The address the listener
/// actually got is the one that is requested.
#[tokio::test]
async fn the_embedded_server_answers_a_smart_http_upload_pack_request() {
    let repos_dir = unique_temp("repos");
    let vault_root = unique_temp("vault");
    let sha = create_repo_with_a_commit(&repos_dir, "demo").await;

    // The bind address the desktop ships with is a loopback address, and
    // the exposure policy requires one for a locally generated
    // credential — so this is the same shape the app runs with.
    let (handle, addr) =
        crate::run_embedded_router("127.0.0.1:0".to_string(), vault_root, repos_dir)
            .await
            .unwrap();

    let url = format!("http://{addr}/repos/demo.git/info/refs?service=git-upload-pack");
    let response = reqwest::get(&url)
        .await
        .unwrap_or_else(|e| panic!("the embedded server did not answer {url}: {e}"));

    assert_eq!(
        response.status(),
        200,
        "the smart-HTTP advertisement path must answer 200 for a repo that exists"
    );
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert!(
        content_type.starts_with("application/x-git-upload-pack-advertisement"),
        "the answer must be an upload-pack advertisement, not something else; got {content_type:?}"
    );

    let body = response
        .bytes()
        .await
        .unwrap_or_else(|e| panic!("the advertisement body could not be read: {e}"));
    let text = String::from_utf8_lossy(&body);

    // The advertisement is pkt-line framed. The first frame announces the
    // service; the frames after it carry the refs.
    assert!(
        text.contains("service=git-upload-pack"),
        "the advertisement must announce the service; got: {text}"
    );
    assert!(
        text.contains("refs/heads/"),
        "the advertisement must carry the repository's refs; got: {text}"
    );
    assert!(
        text.contains(&sha),
        "the advertisement must carry the commit the repository actually holds \
         ({sha}); got: {text}"
    );

    // Stop the server: a test that leaves the listener running holds the
    // port and the thread for the rest of the process's life.
    handle.abort();
}
