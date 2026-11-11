//! Axum HTTP layer for the smart-HTTP Git protocol.
//!
//! Endpoints:
//!   * `GET  /info/refs?service=git-upload-pack`    — advertise refs (read)
//!   * `GET  /info/refs?service=git-receive-pack`   — advertise refs (write)
//!   * `POST /git-upload-pack`                      — fetch / clone body
//!   * `POST /git-receive-pack`                     — push body (auth required)
//!
//! Repos are addressed by the first URL segment after the leading slash.
//! e.g. `/repos/demo.git/info/refs?service=git-upload-pack`.

use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Path, Request, State};
use axum::http::{header, HeaderValue, Response, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use bytes::{Bytes, BytesMut};
use futures_util::StreamExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use url::form_urlencoded;

use crate::config::Config;
use crate::error::GitGitError;
use crate::server::auth::{require_basic, AdminCredentialStore};
use crate::server::smart::announce_frame;
use crate::server::subprocess::{await_success, git_stateless_rpc};
use crate::server::vault_versioned::VersionedVault;

/// The size of one chunk read from the child's stdout.
const RPC_CHUNK_BYTES: usize = 64 * 1024;

/// How many chunks may be in flight between git and the client.
///
/// `[INFERENCE]` 16 chunks of 64 KiB is 1 MiB of buffer per request. The
/// point of the number is that it is a **bound**: the old code held the
/// whole pack in memory, so the memory a fetch cost grew with the
/// repository. This one does not.
const RPC_CHUNK_QUEUE: usize = 16;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    /// V0 Credential Vault backend. Held as `Arc<dyn VersionedVault>`
    /// (per ADR-0022) so handlers — including the gm-console REST API
    /// in `server::api` — can call the version-management surface
    /// (`list_versions`, `diff_versions`, `restore_to_version`, ...).
    /// Callers that only need the 5 KV operations of the super-trait
    /// `Vault` continue to work because `VersionedVault: Vault`.
    pub vault: Arc<dyn VersionedVault>,
    /// The admin basic-auth credential, resolved once at startup and
    /// cached here so the *synchronous* auth check never has to await a
    /// vault read. See [`crate::server::auth`] for the resolution order
    /// and for why it is cached.
    pub admin: Arc<AdminCredentialStore>,
}

impl AppState {
    /// Build state with a **generated** admin password.
    ///
    /// Nothing is compiled in, and nothing is guessed: a caller that has
    /// not resolved a credential gets a random one that nobody knows, so
    /// every protected route answers 401 until
    /// [`Self::with_admin`] is used. Tests use this and then call
    /// `state.admin.rotate_password(..)` to say which credential they
    /// are exercising; production start-up paths call
    /// [`crate::server::auth::resolve_admin_credential`] and
    /// [`Self::with_admin`].
    pub fn new(config: Config, vault: Arc<dyn VersionedVault>) -> Self {
        Self::with_admin(config, vault, Arc::new(AdminCredentialStore::generated()))
    }

    /// Build state with a caller-resolved admin credential.
    pub fn with_admin(
        config: Config,
        vault: Arc<dyn VersionedVault>,
        admin: Arc<AdminCredentialStore>,
    ) -> Self {
        Self {
            config: Arc::new(config),
            vault,
            admin,
        }
    }
}

/// Build the axum [`Router`] for the smart-HTTP server.
///
/// In addition to the Git-protocol routes under `/repos/*key`, this
/// also mounts the gm-console REST API at `/api/*` (see
/// [`crate::server::api`]). The two sub-routers share the same
/// [`AppState`] so API handlers can reach the versioned vault.
pub fn build_router(state: AppState) -> Router {
    let api = crate::server::api::build_api_router(state.clone());
    Router::new()
        .route("/repos/*key", get(handle_repo_any).post(handle_repo_any))
        .nest("/api", api)
        .with_state(state)
}

/// Look up the on-disk path of a bare repo, or 404.
#[allow(clippy::result_large_err)]
fn resolve_repo(state: &AppState, name: &str) -> Result<PathBuf, Response<Body>> {
    match state.config.repo_path(name) {
        Ok(path) => {
            if path.join("HEAD").is_file() {
                Ok(path)
            } else {
                Err(not_found(name))
            }
        }
        Err(_) => Err(not_found(name)),
    }
}

fn not_found(name: &str) -> Response<Body> {
    let body = format!("repo not found: {name}");
    (StatusCode::NOT_FOUND, body).into_response()
}

fn bad_request(msg: impl Into<String>) -> Response<Body> {
    (StatusCode::BAD_REQUEST, msg.into()).into_response()
}

fn internal_error(err: GitGitError) -> Response<Body> {
    tracing::error!(error = %err, "internal error");
    (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response()
}

fn unauthorized() -> Response<Body> {
    let mut resp = (StatusCode::UNAUTHORIZED, "authentication required").into_response();
    resp.headers_mut().insert(
        header::WWW_AUTHENTICATE,
        HeaderValue::from_static(r#"Basic realm="gitgit""#),
    );
    resp
}

/// Dispatch a request that arrived at `/repos/<name>.git/<endpoint>`.
///
/// `key` is the wildcard path after `/repos/`, e.g. `demo.git/info/refs`
/// or `demo.git/git-upload-pack`. We split on the last `/` to recover
/// the endpoint tail, then call the appropriate handler.
///
/// The query extractor is intentionally optional: the GET branch needs
/// `?service=...`, but POSTs to `/git-upload-pack` and `/git-receive-pack`
/// carry no query string at all.
async fn handle_repo_any(
    State(state): State<AppState>,
    Path(key): Path<String>,
    request: Request,
) -> Response<Body> {
    let Some((name, tail)) = key.split_once('/') else {
        return bad_request(format!(
            "expected /repos/<name>.git/<endpoint>, got /repos/{key}"
        ));
    };
    match (request.method().as_str(), tail) {
        ("GET", "info/refs") => {
            // Extract the `service` query parameter manually so a missing
            // or malformed value is a clean 400 instead of an axum extractor
            // error.
            let service = match request.uri().query() {
                Some(q) => match form_urlencoded::parse(q.as_bytes())
                    .find(|(k, _)| k == "service")
                    .map(|(_, v)| v.into_owned())
                {
                    Some(s) if s == "git-upload-pack" || s == "git-receive-pack" => s,
                    Some(s) => return bad_request(format!("unsupported service: {s}")),
                    None => return bad_request("missing service query parameter".to_string()),
                },
                None => return bad_request("missing service query parameter".to_string()),
            };
            serve_info_refs(&state, name, &service).await
        }
        ("POST", "git-upload-pack") => {
            serve_rpc(
                &state,
                name,
                "upload-pack",
                request,
                "application/x-git-upload-pack-result",
            )
            .await
        }
        ("POST", "git-receive-pack") => {
            if let Err(e) = require_basic(request.headers(), &state.admin.current()) {
                return match e {
                    GitGitError::Unauthenticated => unauthorized(),
                    other => internal_error(other),
                };
            }
            serve_rpc(
                &state,
                name,
                "receive-pack",
                request,
                "application/x-git-receive-pack-result",
            )
            .await
        }
        _ => bad_request(format!("unsupported /repos/{key} via {}", request.method())),
    }
}

/// `GET /repos/<name>.git/info/refs?service=git-{upload,receive}-pack`
async fn serve_info_refs(state: &AppState, name: &str, service: &str) -> Response<Body> {
    let subcmd = match service {
        "git-upload-pack" => "upload-pack",
        "git-receive-pack" => "receive-pack",
        other => {
            return bad_request(format!("unsupported service: {other}"));
        }
    };
    let repo = match resolve_repo(state, name) {
        Ok(p) => p,
        Err(resp) => return resp,
    };

    // Run `git <subcmd> --stateless-rpc --advertise-refs <repo>` and capture stdout.
    let mut child = match git_stateless_rpc(subcmd, &repo, true, &[]) {
        Ok(c) => c,
        Err(e) => return internal_error(e),
    };

    // We don't need stdin; drop it so the child can finish cleanly.
    drop(child.stdin);

    // Drain stdout into a Vec<u8>.
    let mut buf = Vec::with_capacity(4096);
    if let Err(e) = child.stdout.read_to_end(&mut buf).await {
        tracing::error!(error = %e, "failed reading git stdout");
        return internal_error(GitGitError::Io(e));
    }

    if let Err(e) = await_success(child.child, &format!("git {subcmd} --advertise-refs")).await {
        tracing::error!(error = %e, "git child failed");
        return internal_error(e);
    }

    // Prepend the smart-HTTP announcement frame.
    let mut body = announce_frame(service);
    body.append(&mut buf);

    // The Content-Type depends on which service the client requested:
    // upload-pack and receive-pack each have their own MIME type.
    let content_type = match service {
        "git-receive-pack" => "application/x-git-receive-pack-advertisement",
        // Default to upload-pack; the only other valid value is also
        // git-upload-pack and that's what the match above handles.
        _ => "application/x-git-upload-pack-advertisement",
    };

    let mut resp = match Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(body))
    {
        Ok(r) => r,
        Err(e) => return internal_error(GitGitError::Http(format!("response build: {e}"))),
    };
    resp.headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    resp.headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    resp
}

/// Generic smart-HTTP RPC: stream request body to git, stream git's stdout
/// back. `content_type` is the response Content-Type.
async fn serve_rpc(
    state: &AppState,
    name: &str,
    subcmd: &str,
    request: Request,
    content_type: &'static str,
) -> Response<Body> {
    let repo = match resolve_repo(state, name) {
        Ok(p) => p,
        Err(resp) => return resp,
    };

    let child = match git_stateless_rpc(subcmd, &repo, false, &[]) {
        Ok(c) => c,
        Err(e) => return internal_error(e),
    };

    // Spawn a task that streams the request body into the child's stdin.
    //
    // We use `into_data_stream` (rather than the `frame()` loop) because
    // HTTP/1.1 keep-alive clients such as `git` may leave the body stream
    // open even after sending all data: the `frame()` future will then
    // hang waiting for trailers that never come. `into_data_stream`
    // surfaces data frames only and completes at the end of the body or
    // when the upstream errors. A 64 MiB cap is a comfortable upper bound
    // for `git push` payloads in MVP.
    let body = request.into_body();
    let stdin = child.stdin;
    // The task owns the request body for as long as the client keeps
    // sending; it is not awaited here — the response may start before
    // the request is finished.
    let _copy_in = tokio::spawn(async move {
        use http_body_util::BodyExt;
        let mut sink = stdin;
        let mut src = BodyExt::into_data_stream(body);
        while let Some(chunk) = src.next().await {
            match chunk {
                Ok(bytes) => {
                    if sink.write_all(&bytes).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        let _ = sink.shutdown().await;
    });

    // The response body is git's stdout, **streamed**.
    //
    // `[FACT]` The old implementation read the whole thing into a
    // `Vec<u8>` before answering, so every concurrent fetch cost the
    // server the size of its pack — a repository with a large pack could
    // take the server down. The chunks now travel through a bounded
    // queue, so what the server holds is the queue's size, not the
    // pack's.
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Bytes, std::io::Error>>(RPC_CHUNK_QUEUE);
    // Signalled once git has produced its first chunk. It is what tells
    // the handler that git is alive and talking, as opposed to having
    // failed before saying anything.
    let (started_tx, mut started_rx) = tokio::sync::oneshot::channel::<()>();
    let stdout = child.stdout;
    let _pump = tokio::spawn(pump_chunks(stdout, tx, Some(started_tx)));

    // The child's exit status can no longer change the HTTP status code
    // once the body has started, so it is watched two ways: the first
    // chunk races the exit, and a failure after the response started is
    // logged and ends the body early (a truncated pack, which the git
    // client reports as an error).
    let (exit_tx, mut exit_rx) = tokio::sync::oneshot::channel::<Result<(), GitGitError>>();
    let child_process = child.child;
    let cmd = format!("git {subcmd} --stateless-rpc");
    let _watcher = tokio::spawn(async move {
        let result = await_success(child_process, &cmd).await;
        if let Err(e) = &result {
            tracing::error!(error = %e, "git rpc failed after the response had started");
        }
        let _ = exit_tx.send(result);
    });

    // The exit status becomes the answer when git produced no output:
    // a success is an empty body, a failure is an error status.
    let no_output =
        |result: Result<Result<(), GitGitError>, tokio::sync::oneshot::error::RecvError>| {
            match result {
                Ok(Ok(())) => rpc_response(content_type, Body::from(Vec::new())),
                Ok(Err(e)) => internal_error(e),
                Err(_) => rpc_response(content_type, Body::from(Vec::new())),
            }
        };

    let started = tokio::select! {
        // git is producing output: stream it.
        started = &mut started_rx => started,
        // git finished before saying anything.
        result = &mut exit_rx => {
            return no_output(result);
        }
    };
    if started.is_err() {
        // The pump ended without a chunk — git closed its stdout without
        // saying anything (it may have rejected the RPC input first).
        // **Only** a successful start signal means "git is talking"; a
        // dropped one means the opposite, so the exit status decides.
        let result = exit_rx.await;
        return no_output(result);
    }

    let stream = futures_util::stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|item| (item, rx))
    });

    rpc_response(content_type, Body::from_stream(stream))
}

/// Read from `reader` and hand the chunks to `tx`.
///
/// `started` is signalled as soon as the **first** chunk has been handed
/// over. That is what tells the handler "git is talking", as opposed to
/// "git failed before saying anything" — the two cases want different
/// answers and the difference has to be observable before the response
/// starts.
///
/// `[FACT]` The chunk is pushed to the queue **before** the reader is
/// drained, so a consumer sees the first bytes while git is still
/// writing. Reading the source to the end first would be the buffered
/// behaviour this replaced.
async fn pump_chunks<R>(
    mut reader: R,
    tx: tokio::sync::mpsc::Sender<Result<Bytes, std::io::Error>>,
    started: Option<tokio::sync::oneshot::Sender<()>>,
) where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut buf = BytesMut::with_capacity(RPC_CHUNK_BYTES);
    let mut started = started;
    loop {
        buf.resize(RPC_CHUNK_BYTES, 0);
        match reader.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => {
                buf.truncate(n);
                let chunk = buf.split().freeze();
                if tx.send(Ok(chunk)).await.is_err() {
                    break;
                }
                if let Some(signal) = started.take() {
                    let _ = signal.send(());
                }
            }
            Err(e) => {
                let _ = tx.send(Err(e)).await;
                break;
            }
        }
    }
}

/// A response with git's content type and the no-cache header git's
/// smart-HTTP protocol expects.
fn rpc_response(content_type: &'static str, body: Body) -> Response<Body> {
    let mut resp = match Response::builder().status(StatusCode::OK).body(body) {
        Ok(r) => r,
        Err(e) => return internal_error(GitGitError::Http(format!("response build: {e}"))),
    };
    resp.headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    resp.headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    resp
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::repo::create_bare_repo;
    use axum::body::to_bytes;
    use axum::http::{Request as HttpRequest, StatusCode as AxStatusCode};
    use std::sync::atomic::{AtomicU64, Ordering};
    use tower::ServiceExt;

    fn unique_temp(label: &str) -> std::path::PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!(
            "gitgit-test-http-{label}-{pid}-{n}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Password these tests authenticate with.
    ///
    /// Not `admin`: the compiled-in default is gone, and a test that
    /// used it would pass against code that still had the old constant.
    const TEST_PASSWORD: &str = "test-password";

    fn auth_header(user: &str, pass: &str) -> header::HeaderValue {
        use base64::Engine;
        // `[FACT]` The `Basic ` prefix is part of the wire format, not
        // decoration: `check_basic` does `strip_prefix("Basic ")` and
        // answers `false` for anything else. Omitting it made every
        // *valid*-credential case in this file report 401 — the
        // malformed-header answer, which is indistinguishable from a real
        // rejection unless you read the header builder.
        let raw = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(format!("{user}:{pass}"))
        );
        header::HeaderValue::from_str(&raw).unwrap()
    }

    /// Set up a `Config` whose `repos_dir` is a unique temp dir, and
    /// pre-create a real bare repo named `name` so that the GET
    /// `/info/refs?service=...` path can spawn a real `git
    /// upload-pack --advertise-refs` against it.
    async fn app_with_repo(label: &str, name: &str) -> (AppState, std::path::PathBuf) {
        let repos = unique_temp(label);
        let vault_root = unique_temp(&format!("{label}-vault"));
        let cfg = Config::new("127.0.0.1:0", repos.clone(), vault_root);
        create_bare_repo(&cfg, name).await.unwrap();
        let vault = std::sync::Arc::new(crate::server::vault::FileVault::new(&cfg.vault_file_root));
        let state = AppState::new(cfg, vault);
        // `AppState::new` starts with a random password nobody knows, so
        // every auth case here names the credential it is exercising.
        state.admin.rotate_password(TEST_PASSWORD);
        (state, repos)
    }

    #[tokio::test]
    async fn app_state_holds_config() {
        let repos = unique_temp("app-state");
        let vault_root = unique_temp("app-state-vault");
        let cfg = Config::new("127.0.0.1:9999", repos, vault_root);
        let vault = std::sync::Arc::new(crate::server::vault::FileVault::new(&cfg.vault_file_root));
        let state = AppState::new(cfg.clone(), vault);
        assert_eq!(state.config.bind, "127.0.0.1:9999");
        assert_eq!(state.config.repos_dir, cfg.repos_dir);
    }

    #[tokio::test]
    async fn info_refs_unknown_repo_returns_404() {
        let (state, _repos) = app_with_repo("notfound", "demo").await;
        let app = build_router(state);
        // No "demo2" repo on disk.
        let req = HttpRequest::builder()
            .method("GET")
            .uri("/repos/demo2.git/info/refs?service=git-upload-pack")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), AxStatusCode::NOT_FOUND);
        let body = to_bytes(resp.into_body(), 4096).await.unwrap();
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("repo not found"), "got: {s}");
    }

    #[tokio::test]
    async fn info_refs_missing_service_returns_400() {
        let (state, _repos) = app_with_repo("noservice", "demo").await;
        let app = build_router(state);
        // No `?service=...` query string at all.
        let req = HttpRequest::builder()
            .method("GET")
            .uri("/repos/demo.git/info/refs")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), AxStatusCode::BAD_REQUEST);
        let body = to_bytes(resp.into_body(), 4096).await.unwrap();
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("missing service"), "got: {s}");
    }

    #[tokio::test]
    async fn info_refs_unsupported_service_returns_400() {
        let (state, _repos) = app_with_repo("badservice", "demo").await;
        let app = build_router(state);
        // The router only knows git-upload-pack / git-receive-pack.
        let req = HttpRequest::builder()
            .method("GET")
            .uri("/repos/demo.git/info/refs?service=git-upload-archive")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), AxStatusCode::BAD_REQUEST);
        let body = to_bytes(resp.into_body(), 4096).await.unwrap();
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("unsupported service"), "got: {s}");
    }

    #[tokio::test]
    async fn receive_pack_without_auth_returns_401_with_www_authenticate() {
        let (state, _repos) = app_with_repo("noauth", "demo").await;
        let app = build_router(state);
        // POST receive-pack with NO Authorization header.
        let req = HttpRequest::builder()
            .method("POST")
            .uri("/repos/demo.git/git-receive-pack")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), AxStatusCode::UNAUTHORIZED);
        // RFC 7235: 401 must carry a WWW-Authenticate challenge.
        let challenge = resp
            .headers()
            .get(header::WWW_AUTHENTICATE)
            .expect("WWW-Authenticate must be present on 401");
        let s = challenge.to_str().unwrap();
        assert!(
            s.contains("Basic") && s.contains("gitgit"),
            "expected Basic challenge with realm=\"gitgit\", got: {s}"
        );
    }

    #[tokio::test]
    async fn receive_pack_with_wrong_password_returns_401() {
        let (state, _repos) = app_with_repo("wrongpw", "demo").await;
        let app = build_router(state);
        // A well-formed Basic header carrying the right username and the
        // wrong password.
        let req = HttpRequest::builder()
            .method("POST")
            .uri("/repos/demo.git/git-receive-pack")
            .header(
                header::AUTHORIZATION,
                auth_header("admin", "not-the-password"),
            )
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), AxStatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn receive_pack_with_the_historical_default_credential_returns_401() {
        // `admin` / `admin` was a `const` in `config`, so it shipped in
        // every binary. It must not authenticate a push now.
        let (state, _repos) = app_with_repo("olddefault", "demo").await;
        let app = build_router(state);
        let req = HttpRequest::builder()
            .method("POST")
            .uri("/repos/demo.git/git-receive-pack")
            .header(header::AUTHORIZATION, auth_header("admin", "admin"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), AxStatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn receive_pack_accepts_the_configured_credential() {
        // Asserted as "not 401" against a missing repo rather than as a
        // successful push: a real push needs a real packfile, and
        // spawning `git receive-pack` with an empty body would test git,
        // not our auth. Reaching the 404 is proof the auth gate opened.
        let (state, _repos) = app_with_repo("goodpw", "demo").await;
        let app = build_router(state);
        let req = HttpRequest::builder()
            .method("POST")
            .uri("/repos/does-not-exist.git/git-receive-pack")
            .header(header::AUTHORIZATION, auth_header("admin", TEST_PASSWORD))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            AxStatusCode::NOT_FOUND,
            "valid credentials must get past the auth gate"
        );
    }

    #[tokio::test]
    async fn info_refs_happy_path_returns_advertisement() {
        // End-to-end: a real bare repo, real `git upload-pack
        // --advertise-refs`, smart-HTTP announcement frame, and the
        // Content-Type the git client expects.
        let (state, _repos) = app_with_repo("happyrefs", "demo").await;
        let app = build_router(state);
        let req = HttpRequest::builder()
            .method("GET")
            .uri("/repos/demo.git/info/refs?service=git-upload-pack")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), AxStatusCode::OK);
        // Content-Type must be the upload-pack advertisement MIME.
        let ct = resp
            .headers()
            .get(header::CONTENT_TYPE)
            .expect("Content-Type must be present")
            .to_str()
            .unwrap();
        assert_eq!(ct, "application/x-git-upload-pack-advertisement");
        // Cache-Control: no-cache (per the wire-format requirement).
        let cc = resp.headers().get(header::CACHE_CONTROL).unwrap();
        assert_eq!(cc.to_str().unwrap(), "no-cache");
        // Body must start with the announcement frame: hex length +
        // "# service=git-upload-pack\n" + "0000" flush.
        let body = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
        let prefix = std::str::from_utf8(&body[0..4]).unwrap();
        let total: usize = usize::from_str_radix(prefix, 16).unwrap();
        assert!(total > 4, "announcement frame length too small: {total}");
        let line = std::str::from_utf8(&body[4..total]).unwrap();
        assert_eq!(line, "# service=git-upload-pack\n");
        assert_eq!(&body[total..total + 4], b"0000");
    }

    /// `[FACT]` The response body must reach the client **while git is
    /// still writing**, not after the whole pack has been read into
    /// memory.
    ///
    /// This is the test that fails if the handler goes back to buffering:
    /// a reader that drains the source first delivers nothing until the
    /// source is closed, and the receive below would time out.
    /// `[FACT]` A reader that ends without producing a chunk drops the
    /// start signal — it does **not** signal "output". The handler tells
    /// the two apart, and only a successful signal means git is talking.
    #[tokio::test]
    async fn an_empty_reader_drops_the_start_signal_instead_of_signalling_output() {
        let (tx, _rx) = tokio::sync::mpsc::channel::<Result<Bytes, std::io::Error>>(4);
        let (started_tx, started_rx) = tokio::sync::oneshot::channel::<()>();
        let _pump = tokio::spawn(pump_chunks(
            std::io::Cursor::new(Vec::<u8>::new()),
            tx,
            Some(started_tx),
        ));
        let started = started_rx.await;
        assert!(
            started.is_err(),
            "an empty source must not signal output — the exit status is the answer"
        );
    }

    /// `[FACT]` A git that rejects the request before producing any
    /// output is still an error, not an empty 200. The buffered handler
    /// answered with git's exit status; the streaming one has to keep
    /// that, even though it cannot change the status once a body has
    /// started.
    #[tokio::test]
    async fn a_git_that_produces_no_output_is_reported_as_an_error() {
        let (state, _repos) = app_with_repo("nooutput", "demo").await;
        let app = build_router(state);
        // A body that is not a pkt-line at all: git rejects it before
        // writing anything.
        let req = HttpRequest::builder()
            .method("POST")
            .uri("/repos/demo.git/git-upload-pack")
            .body(Body::from("not a pkt-line at all"))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_ne!(
            resp.status(),
            AxStatusCode::OK,
            "a git that produced nothing must not be answered with an empty 200"
        );
    }

    #[tokio::test]
    async fn the_response_chunks_arrive_before_the_source_is_drained() {
        use tokio::io::AsyncWriteExt;
        let (mut client, source) = tokio::io::duplex(64);
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<Bytes, std::io::Error>>(4);
        let (started_tx, started_rx) = tokio::sync::oneshot::channel::<()>();
        let _pump = tokio::spawn(pump_chunks(source, tx, Some(started_tx)));

        client.write_all(b"first").await.unwrap();

        let chunk = tokio::time::timeout(std::time::Duration::from_secs(10), rx.recv())
            .await
            .expect("the chunk must arrive before the source is closed")
            .expect("the queue must not be closed")
            .expect("the chunk must not be an error");
        assert_eq!(&chunk[..], b"first");
        // The source is still open — that is the whole point.
        started_rx
            .await
            .expect("the started signal must fire with the first chunk");
    }

    /// `[FACT]` A read error on the child's stdout has to reach the
    /// client as a failed body, not as a silently truncated one.
    #[tokio::test]
    async fn a_read_error_ends_the_body_with_the_error() {
        struct Failing;
        impl tokio::io::AsyncRead for Failing {
            fn poll_read(
                self: std::pin::Pin<&mut Self>,
                _cx: &mut std::task::Context<'_>,
                _buf: &mut tokio::io::ReadBuf<'_>,
            ) -> std::task::Poll<std::io::Result<()>> {
                std::task::Poll::Ready(Err(std::io::Error::other("boom")))
            }
        }
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<Bytes, std::io::Error>>(4);
        let _pump = tokio::spawn(pump_chunks(Failing, tx, None));
        let item = tokio::time::timeout(std::time::Duration::from_secs(10), rx.recv())
            .await
            .expect("the error must arrive")
            .expect("the queue must not be closed");
        let err = item.expect_err("the read error must be reported as an error");
        assert!(err.to_string().contains("boom"), "got: {err}");
    }
}
