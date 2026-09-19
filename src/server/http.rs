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
use futures_util::StreamExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use url::form_urlencoded;

use crate::config::Config;
use crate::error::GitGitError;
use crate::server::auth::require_basic;
use crate::server::smart::announce_frame;
use crate::server::subprocess::{git_stateless_rpc, await_success};
use crate::server::vault_versioned::VersionedVault;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    /// V0 Credential Vault backend (per ADR-0021 §1.2 + ADR-0022
    /// §follow-up). Held as `Arc<dyn VersionedVault>` so the gm-console
    /// REST surface (`src/server/api.rs`) and future handlers can call
    /// the versioned write/diff/restore methods directly. Callers that
    /// only need the 5 KV operations can auto-deref to the `Vault`
    /// super-trait. `src/main.rs::build_vault` already returns this
    /// type, so no constructor change is needed there.
    pub vault: Arc<dyn VersionedVault>,
}

impl AppState {
    pub fn new(config: Config, vault: Arc<dyn VersionedVault>) -> Self {
        Self {
            config: Arc::new(config),
            vault,
        }
    }
}

/// Build the axum [`Router`] for the smart-HTTP server.
pub fn build_router(state: AppState) -> Router {
    // All endpoints are addressed under `/repos/<name>.git/...`. A single
    // catch-all wildcard captures the full tail and dispatches in-handler
    // to the appropriate smart-HTTP sub-handler. Authentication for
    // receive-pack is enforced in the dispatch handler itself.
    //
    // The gm-console REST surface (per gm-console v0.1 brief) lives under
    // `/api/*` and is registered via `nest` so it cannot collide with
    // `/repos/*` Git-protocol paths.
    Router::new()
        .route("/repos/*key", get(handle_repo_any).post(handle_repo_any))
        .nest("/api", crate::server::api::build_api_router())
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
            serve_rpc(&state, name, "upload-pack", request, "application/x-git-upload-pack-result").await
        }
        ("POST", "git-receive-pack") => {
            if let Err(e) = require_basic(request.headers()) {
                return match e {
                    GitGitError::Unauthenticated => unauthorized(),
                    other => internal_error(other),
                };
            }
            serve_rpc(&state, name, "receive-pack", request, "application/x-git-receive-pack-result").await
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
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(content_type),
    );
    resp.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache"),
    );
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
    let copy_in = tokio::spawn(async move {
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

    // Spawn a task that drains stdout into a shared buffer.
    let stdout = child.stdout;
    let copy_out = tokio::spawn(async move {
        let mut src = stdout;
        let mut buf = Vec::with_capacity(8192);
        let _ = src.read_to_end(&mut buf).await;
        buf
    });

    // Wait for both tasks to finish.
    let (in_result, out_buf) = tokio::join!(copy_in, copy_out);
    let _ = in_result;
    let out_buf = out_buf.unwrap_or_default();

    // Now wait for the git child to exit.
    if let Err(e) = await_success(child.child, &format!("git {subcmd} --stateless-rpc")).await {
        return internal_error(e);
    }

    let mut resp = match Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(out_buf))
    {
        Ok(r) => r,
        Err(e) => return internal_error(GitGitError::Http(format!("response build: {e}"))),
    };
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(content_type),
    );
    resp.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache"),
    );
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
    use base64::Engine;
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
        (AppState::new(cfg, vault), repos)
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
        // Send a Basic header that decodes to "admin:nope".
        let bad = base64::engine::general_purpose::STANDARD.encode(b"admin:nope");
        let req = HttpRequest::builder()
            .method("POST")
            .uri("/repos/demo.git/git-receive-pack")
            .header(header::AUTHORIZATION, format!("Basic {bad}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), AxStatusCode::UNAUTHORIZED);
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
}
