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
use axum::extract::{Path, Query, Request, State};
use axum::http::{header, HeaderValue, Response, StatusCode};
use axum::middleware::{self, Next};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::config::Config;
use crate::error::GitGitError;
use crate::server::auth::require_basic;
use crate::server::smart::announce_frame;
use crate::server::subprocess::{git_stateless_rpc, await_success};

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            config: Arc::new(config),
        }
    }
}

/// Build the axum [`Router`] for the smart-HTTP server.
pub fn build_router(state: AppState) -> Router {
    let receive_pack = Router::new()
        .route("/git-receive-pack", post(handle_receive_pack))
        .route_layer(middleware::from_fn(auth_middleware));

    Router::new()
        .route("/info/refs", get(handle_info_refs))
        .route("/git-upload-pack", post(handle_upload_pack))
        .merge(receive_pack)
        .with_state(state)
}

/// Query parameters for `/info/refs`.
#[derive(Debug, Deserialize)]
pub struct InfoRefsQuery {
    pub service: String,
}

/// Parse the repo name out of a URL like `/repos/demo.git/info/refs` or
/// `/repos/demo.git/git-upload-pack`. The first segment is always `/repos/`
/// (the path the brief asks git clients to use).
fn parse_repo_name(path: &str) -> Option<(&str, &str)> {
    // Strip leading `/`, split into segments.
    let trimmed = path.trim_start_matches('/');
    let mut iter = trimmed.split('/');
    let first = iter.next()?;
    if first != "repos" {
        return None;
    }
    let name = iter.next()?;
    if name.is_empty() {
        return None;
    }
    // The remainder is everything after `<name>.git`. We only need the
    // first segment of the remainder to dispatch on the endpoint, so we
    // can borrow it from the input string.
    let after_name = trimmed.strip_prefix("repos/")?.strip_prefix(name)?;
    // `after_name` starts with `/` (e.g. `/info/refs`) or is empty.
    let suffix = after_name.trim_start_matches('/');
    Some((name, suffix))
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

/// Auth middleware: runs only on the receive-pack router. The wrapped handler
/// will already have parsed headers, but middleware is the simplest place to
/// enforce auth without per-handler boilerplate.
async fn auth_middleware(request: Request, next: Next) -> Response<Body> {
    match require_basic(request.headers()) {
        Ok(()) => next.run(request).await,
        Err(GitGitError::Unauthenticated) => unauthorized(),
        Err(e) => internal_error(e),
    }
}

/// `GET /info/refs?service=git-upload-pack`
async fn handle_info_refs(
    State(state): State<AppState>,
    Path(path): Path<String>,
    Query(q): Query<InfoRefsQuery>,
) -> Response<Body> {
    let Some((name, _suffix)) = parse_repo_name(&path) else {
        return bad_request("expected /repos/<name>.git/info/refs");
    };
    let subcmd = match q.service.as_str() {
        "git-upload-pack" => "upload-pack",
        "git-receive-pack" => "receive-pack",
        other => {
            return bad_request(format!("unsupported service: {other}"));
        }
    };
    let repo = match resolve_repo(&state, name) {
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
        return internal_error(GitGitError::Io(e));
    }

    if let Err(e) = await_success(child.child, &format!("git {subcmd} --advertise-refs")).await {
        return internal_error(e);
    }

    // Prepend the smart-HTTP announcement frame.
    let mut body = announce_frame(&q.service);
    body.append(&mut buf);

    let mut resp = match Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(body))
    {
        Ok(r) => r,
        Err(e) => return internal_error(GitGitError::Http(format!("response build: {e}"))),
    };
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/x-git-upload-pack-advertisement"),
    );
    resp.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache"),
    );
    resp
}

/// `POST /git-upload-pack`
async fn handle_upload_pack(
    State(state): State<AppState>,
    Path(path): Path<String>,
    request: Request,
) -> Response<Body> {
    let Some((name, _suffix)) = parse_repo_name(&path) else {
        return bad_request("expected /repos/<name>.git/git-upload-pack");
    };
    serve_rpc(&state, name, "upload-pack", request, "application/x-git-upload-pack-result").await
}

/// `POST /git-receive-pack` (auth already enforced by middleware).
async fn handle_receive_pack(
    State(state): State<AppState>,
    Path(path): Path<String>,
    request: Request,
) -> Response<Body> {
    let Some((name, _suffix)) = parse_repo_name(&path) else {
        return bad_request("expected /repos/<name>.git/git-receive-pack");
    };
    serve_rpc(
        &state,
        name,
        "receive-pack",
        request,
        "application/x-git-receive-pack-result",
    )
    .await
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
    let body = request.into_body();
    let stdin = child.stdin;
    let copy_in = tokio::spawn(async move {
        use http_body_util::BodyExt;
        let mut sink = stdin;
        let mut src = body;
        while let Some(frame) = src.frame().await {
            match frame {
                Ok(frame) => {
                    if let Ok(data) = frame.into_data() {
                        if sink.write_all(&data).await.is_err() {
                            break;
                        }
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
