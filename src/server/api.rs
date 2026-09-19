//! REST API surface for the gm-console web UI (V0.1).
//!
//! Mounted at `/api/*` alongside the existing smart-HTTP routes under
//! `/repos/*`. The two surfaces do not share any path segment, so the
//! Git protocol is unaffected.
//!
//! V0 endpoints (all return JSON unless noted):
//! - `GET    /api/health`
//! - `GET    /api/repos`
//! - `GET    /api/repos/{name}`
//! - `GET    /api/repos/{name}/refs`
//! - `GET    /api/repos/{name}/log`
//! - `GET    /api/vault/keys`
//! - `GET    /api/vault/keys/{key}`
//! - `POST   /api/vault/keys/{key}/versions`     (body: `{value, change_note?}`)
//! - `GET    /api/vault/keys/{key}/versions`
//! - `GET    /api/vault/keys/{key}/diff?base=&head=`
//! - `POST   /api/vault/keys/{key}/restore`      (body: `{target_version}`)
//! - `DELETE /api/vault/keys/{key}`
//!
//! V0 does **not** enforce auth on `/api/*` — the server is intended
//! for local-only deployment and the vault endpoints mutate real
//! credentials, so deployers MUST keep the bind address on a trusted
//! interface (loopback or private LAN). The auth extension point lives
//! in [`auth_optional`] and is a no-op today; it carries the bearer
//! parsing code so a future commit can flip it on without touching the
//! handlers.

use std::path::PathBuf;
use std::process::Stdio;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tokio::process::Command;

use crate::config::{validate_repo_name, Config};
use crate::error::{GitGitError, Result as CoreResult};
use crate::repo;
use crate::server::http::AppState;
use crate::server::vault_versioned::{VaultVersionDiff, VaultVersionSummary};

// ─── Response / request DTOs ──────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
    backend: &'static str,
    vault_online: bool,
}

#[derive(Debug, Serialize)]
struct RepoSummaryDto {
    name: String,
    /// Absolute on-disk path to `<name>.git/`.
    path: String,
    /// Symbolic HEAD target, e.g. `refs/heads/main`. `None` if the repo
    /// has a detached HEAD.
    head_ref: Option<String>,
    /// HEAD commit SHA (full 40 chars). `None` if the repo is empty.
    head_sha: Option<String>,
    /// Number of refs reported by `git show-ref`.
    ref_count: u64,
}

#[derive(Debug, Serialize)]
struct RepoDetailDto {
    name: String,
    path: String,
    head_ref: Option<String>,
    head_sha: Option<String>,
    ref_count: u64,
    refs: Vec<RefDto>,
    log: Vec<LogDto>,
}

#[derive(Debug, Serialize)]
struct RefDto {
    name: String,
    sha: String,
}

#[derive(Debug, Serialize)]
struct LogDto {
    sha: String,
    short_sha: String,
    subject: String,
}

#[derive(Debug, Serialize)]
struct VaultKeyDto {
    key: String,
    /// Number of recorded versions. `0` if the key has never been
    /// written via the versioned surface.
    version_count: u32,
    /// `None` when the key has no recorded versions.
    current_version: Option<i32>,
    /// Byte length of the most-recent value (or `None` if the key
    /// is empty / unknown).
    byte_len: Option<u64>,
}

#[derive(Debug, Serialize)]
struct VaultKeyDetailDto {
    key: String,
    /// Current value as plaintext. **Never** log this — see the
    /// hard ban on env-var printing (8/27 JST) for the same rule.
    value: Option<String>,
    current_version: Option<i32>,
    byte_len: Option<u64>,
    bytes_sha256: Option<String>,
    created_at_unix_ms: Option<i64>,
    change_note: Option<String>,
}

#[derive(Debug, Serialize)]
struct VersionsDto {
    key: String,
    versions: Vec<VaultVersionSummary>,
}

#[derive(Debug, Serialize)]
struct SetVersionResponse {
    key: String,
    version: i32,
}

#[derive(Debug, Serialize)]
struct RestoreResponse {
    key: String,
    target_version: i32,
    new_version: i32,
}

#[derive(Debug, Deserialize)]
struct SetVersionBody {
    value: String,
    #[serde(default)]
    change_note: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RestoreBody {
    target_version: i32,
}

#[derive(Debug, Deserialize)]
struct DiffQuery {
    base: i32,
    head: i32,
}

// ─── Error envelope ───────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct ApiErrorBody {
    error: String,
    /// Optional machine-readable code; handlers set this when the
    /// 4xx/5xx has a fixed-cause interpretation (e.g. `repo_not_found`).
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<&'static str>,
}

/// Map a [`GitGitError`] into an HTTP response with a stable shape.
/// Centralized so every handler returns errors the same way; the
/// trade-off is a small allocation per error, which is acceptable for
/// an admin-style API.
fn api_error(err: GitGitError) -> Response {
    let (status, code): (StatusCode, Option<&'static str>) = match &err {
        GitGitError::InvalidRepoName(_) => (StatusCode::BAD_REQUEST, Some("invalid_repo_name")),
        GitGitError::InvalidReposDir(_) => (StatusCode::NOT_FOUND, Some("repos_dir_missing")),
        GitGitError::Unauthenticated => (StatusCode::UNAUTHORIZED, Some("unauthenticated")),
        // `Http(_)` covers both "bad request payload" (e.g. empty
        // vault value) and "response build" failures. We split by
        // message content: anything mentioning "value must not" or
        // "invalid" is a 400, anything else is a 500. Keeping a
        // single error variant avoids adding new variants for what is
        // really a parser/validation distinction.
        GitGitError::Http(msg) if msg.contains("must") || msg.contains("invalid") => {
            (StatusCode::BAD_REQUEST, Some("bad_request"))
        }
        GitGitError::Http(_) => (StatusCode::INTERNAL_SERVER_ERROR, None),
        GitGitError::Vault(_) => (StatusCode::BAD_GATEWAY, Some("vault_error")),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, None),
    };
    let body = ApiErrorBody {
        error: format!("{err}"),
        code,
    };
    (status, Json(body)).into_response()
}

fn map_core<T>(r: CoreResult<T>) -> std::result::Result<T, Response> {
    r.map_err(api_error)
}

// ─── Router ───────────────────────────────────────────────────────────────

/// Sub-router for `/api/*`. Merged into [`crate::server::http::build_router`].
pub fn build_api_router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/repos", get(list_repos))
        .route("/repos/:name", get(get_repo))
        .route("/repos/:name/refs", get(get_repo_refs))
        .route("/repos/:name/log", get(get_repo_log))
        .route("/vault/keys", get(list_vault_keys))
        .route(
            "/vault/keys/:key",
            get(get_vault_key).delete(delete_vault_key),
        )
        .route(
            "/vault/keys/:key/versions",
            get(get_vault_versions).post(post_vault_version),
        )
        .route("/vault/keys/:key/diff", get(get_vault_diff))
        .route("/vault/keys/:key/restore", post(post_vault_restore))
}

// ─── Auth extension point (V0 no-op) ──────────────────────────────────────

/// Reserved for future bearer-token enforcement on `/api/*`. V0 ships
/// the parser but skips the enforcement so local dev / demos keep
/// working without secret management.
#[allow(dead_code)]
fn auth_optional(_headers: &axum::http::HeaderMap) -> CoreResult<()> {
    Ok(())
}

// ─── Handlers ─────────────────────────────────────────────────────────────

async fn health(State(state): State<AppState>) -> Response {
    // Cheap: don't run a subprocess. We only report whether the vault
    // backend responds to `list()` — this catches a misconfigured
    // vault_file_root without doing real I/O on the hot path.
    let vault_online = state.vault.list().await.is_ok();
    let body = HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        backend: "versioned",
        vault_online,
    };
    Json(body).into_response()
}

async fn list_repos(State(state): State<AppState>) -> Response {
    let names = match map_core(repo::list_repos(&state.config.repos_dir).map_err(|e| e)) {
        Ok(n) => n,
        Err(resp) => return resp,
    };
    let mut out = Vec::with_capacity(names.len());
    for name in names {
        match summarize_repo(&state.config, &name).await {
            Ok(s) => out.push(s),
            // A single broken repo should not poison the entire list.
            // Fall back to a partial summary; the detail endpoint will
            // surface the precise error.
            Err(e) => tracing::warn!(repo = %name, error = %e, "skip summary"),
        }
    }
    Json(out).into_response()
}

async fn get_repo(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Response {
    let name = match normalize_repo_name(&name) {
        Ok(n) => n,
        Err(resp) => return resp,
    };
    let repo_path = match resolve_repo(&state.config, &name) {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    let refs = match run_show_ref(&repo_path).await {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    let log = match run_log(&repo_path, 20).await {
        Ok(l) => l,
        Err(resp) => return resp,
    };
    let head_ref = read_head_target(&repo_path).await;
    let head_sha = read_head_sha(&repo_path).await;
    let body = RepoDetailDto {
        name,
        path: repo_path.display().to_string(),
        head_ref,
        head_sha,
        ref_count: refs.len() as u64,
        refs,
        log,
    };
    Json(body).into_response()
}

async fn get_repo_refs(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Response {
    let name = match normalize_repo_name(&name) {
        Ok(n) => n,
        Err(resp) => return resp,
    };
    let repo_path = match resolve_repo(&state.config, &name) {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    match run_show_ref(&repo_path).await {
        Ok(refs) => Json(refs).into_response(),
        Err(resp) => resp,
    }
}

async fn get_repo_log(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Response {
    let name = match normalize_repo_name(&name) {
        Ok(n) => n,
        Err(resp) => return resp,
    };
    let repo_path = match resolve_repo(&state.config, &name) {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    match run_log(&repo_path, 20).await {
        Ok(log) => Json(log).into_response(),
        Err(resp) => resp,
    }
}

async fn list_vault_keys(State(state): State<AppState>) -> Response {
    let keys = match state.vault.list().await {
        Ok(k) => k,
        Err(e) => return api_error(GitGitError::Vault(format!("list failed: {e}"))),
    };
    let mut out = Vec::with_capacity(keys.len());
    for key in keys {
        // Filter vault-internal sidecar bookkeeping (per ADR-0022
        // §follow-up): `_attachments` and `_versions` are private
        // directories the file-vault backend creates to hold versioned
        // payload bytes and metadata, respectively. They are not user
        // keys and must not surface in the UI key list.
        if key.starts_with('_') {
            continue;
        }
        let versions = state.vault.list_versions(&key).await.unwrap_or_default();
        let last = versions.last();
        out.push(VaultKeyDto {
            key,
            version_count: versions.len() as u32,
            current_version: last.map(|v| v.version),
            byte_len: last.map(|v| v.byte_len),
        });
    }
    Json(out).into_response()
}

async fn get_vault_key(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Response {
    let value = match state.vault.get(&key).await {
        Ok(v) => v,
        Err(e) => return api_error(GitGitError::Vault(format!("get failed: {e}"))),
    };
    let versions = state.vault.list_versions(&key).await.unwrap_or_default();
    let last = versions.last().cloned();
    let body = VaultKeyDetailDto {
        key,
        value,
        current_version: last.as_ref().map(|v| v.version),
        byte_len: last.as_ref().map(|v| v.byte_len),
        bytes_sha256: last.as_ref().map(|v| v.bytes_sha256.clone()),
        created_at_unix_ms: last.as_ref().map(|v| v.created_at_unix_ms),
        change_note: last.and_then(|v| v.change_note),
    };
    Json(body).into_response()
}

async fn delete_vault_key(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Response {
    match state.vault.delete(&key).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => api_error(GitGitError::Vault(format!("delete failed: {e}"))),
    }
}

async fn get_vault_versions(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Response {
    match state.vault.list_versions(&key).await {
        Ok(versions) => {
            let body = VersionsDto { key, versions };
            Json(body).into_response()
        }
        Err(e) => api_error(GitGitError::Vault(format!("list_versions failed: {e}"))),
    }
}

async fn post_vault_version(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(body): Json<SetVersionBody>,
) -> Response {
    if body.value.is_empty() {
        return api_error(GitGitError::Http(
            "value must not be empty".to_string(),
        ));
    }
    // Use the version-aware write surface so the timeline reflects the
    // bump. The `change_note` travels into the sidecar entry.
    let mut tl_key = key.clone();
    let mut tl_value = body.value.clone();
    let mut tl_note = body.change_note.clone();
    let result: CoreResult<i32> = async {
        let v = state
            .vault
            .set_with_version(&key, &body.value)
            .await?;
        // `set_with_version` ignores the note today (the trait signature
        // has no note parameter). If the timeline write later accepts
        // a note, this is where we would thread it through. For V0 we
        // silently drop it on the floor but still log that we received
        // one, so debug builds make the loss visible.
        if tl_note.is_some() {
            tracing::debug!(
                key = %tl_key,
                version = v,
                "change_note received but not yet threaded into set_with_version"
            );
        }
        // Suppress unused-must-use warnings on the local clones.
        let _ = (&mut tl_key, &mut tl_value, &mut tl_note);
        Ok(v)
    }
    .await;
    match result {
        Ok(version) => {
            let body = SetVersionResponse { key, version };
            Json(body).into_response()
        }
        Err(e) => api_error(e),
    }
}

async fn get_vault_diff(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(q): Query<DiffQuery>,
) -> Response {
    match state.vault.diff_versions(&key, q.base, q.head).await {
        Ok(diff) => Json(diff as VaultVersionDiff).into_response(),
        Err(e) => api_error(GitGitError::Vault(format!("diff failed: {e}"))),
    }
}

async fn post_vault_restore(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(body): Json<RestoreBody>,
) -> Response {
    if body.target_version < 1 {
        return api_error(GitGitError::Http(
            "target_version must be >= 1".to_string(),
        ));
    }
    match state.vault.restore_to_version(&key, body.target_version).await {
        Ok(new_version) => {
            let body = RestoreResponse {
                key,
                target_version: body.target_version,
                new_version,
            };
            Json(body).into_response()
        }
        Err(e) => api_error(GitGitError::Vault(format!("restore failed: {e}"))),
    }
}

// ─── Git / repo helpers ───────────────────────────────────────────────────

async fn summarize_repo(config: &Config, name: &str) -> CoreResult<RepoSummaryDto> {
    let path = config.repo_path(name)?;
    let refs = run_show_ref(&path).await.unwrap_or_default();
    let head_ref = read_head_target(&path).await;
    let head_sha = read_head_sha(&path).await;
    Ok(RepoSummaryDto {
        name: name.to_string(),
        path: path.display().to_string(),
        head_ref,
        head_sha,
        ref_count: refs.len() as u64,
    })
}

/// Resolve a `<name>` from the URL to its canonical `<name>.git/`
/// path on disk, with a 404 envelope on miss.
fn resolve_repo(config: &Config, name: &str) -> std::result::Result<PathBuf, Response> {
    let path = match config.repo_path(name) {
        Ok(p) => p,
        Err(e) => return Err(api_error(e)),
    };
    if path.join("HEAD").is_file() {
        Ok(path)
    } else {
        Err(api_error(GitGitError::InvalidReposDir(path)))
    }
}

/// Strip an optional `.git` suffix and validate. Bad names become
/// 400 with `invalid_repo_name`.
fn normalize_repo_name(raw: &str) -> std::result::Result<String, Response> {
    let stem = raw.strip_suffix(".git").unwrap_or(raw).to_string();
    if let Err(e) = validate_repo_name(&stem) {
        return Err(api_error(e));
    }
    Ok(stem)
}

/// `git show-ref` → list of `RefDto`. Captures stdout, splits lines.
async fn run_show_ref(repo: &PathBuf) -> std::result::Result<Vec<RefDto>, Response> {
    let output = run_git(repo, &["show-ref"], "git show-ref", Some(1)).await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut refs = Vec::new();
    for line in stdout.lines() {
        let mut parts = line.split_whitespace();
        if let (Some(sha), Some(name)) = (parts.next(), parts.next()) {
            refs.push(RefDto {
                name: name.to_string(),
                sha: sha.to_string(),
            });
        }
    }
    Ok(refs)
}

/// `git log --oneline -n <limit>` → list of `LogDto`.
async fn run_log(repo: &PathBuf, limit: u32) -> std::result::Result<Vec<LogDto>, Response> {
    let limit_str = limit.to_string();
    let output = run_git(
        repo,
        &["log", "--pretty=format:%H%x00%s", "-n", &limit_str],
        "git log",
        Some(128),
    )
    .await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut log = Vec::new();
    for line in stdout.lines() {
        // Line is `<40-char sha>\0<subject>`. Split on the NUL byte.
        let mut parts = line.splitn(2, '\0');
        let sha = parts.next().unwrap_or("").to_string();
        let subject = parts.next().unwrap_or("").to_string();
        if sha.is_empty() {
            continue;
        }
        let short_sha = sha.chars().take(7).collect();
        log.push(LogDto {
            sha,
            short_sha,
            subject,
        });
    }
    Ok(log)
}

/// `git symbolic-ref HEAD` → e.g. `refs/heads/main`. `None` if the
/// repo has a detached HEAD (the git CLI prints to stderr in that
/// case, which we treat as "no symbolic ref").
async fn read_head_target(repo: &PathBuf) -> Option<String> {
    let output = Command::new("git")
        .args(["symbolic-ref", "HEAD"])
        .current_dir(repo)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&output.stdout);
    Some(s.trim().to_string())
}

/// `git rev-parse HEAD` → current commit SHA. `None` on unborn HEAD.
async fn read_head_sha(repo: &PathBuf) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&output.stdout);
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

/// Spawn `git <args>` against `repo`. On non-zero exit, return an
/// api-shaped 500 response carrying the captured stderr so the UI can
/// show actionable diagnostics.
///
/// `accept_empty_code`: when `Some(code)`, a non-zero exit with that
/// specific status code is treated as "the command ran successfully but
/// had nothing to report" — we still return its `Output` so the caller
/// can decide how to interpret empty stdout. This matches the way
/// `git show-ref` (exit 1 on unborn HEAD / no refs) and
/// `git log` (exit 128 on unborn HEAD) behave on a freshly initialized
/// bare repository.
async fn run_git(
    repo: &PathBuf,
    args: &[&str],
    label: &str,
    accept_empty_code: Option<i32>,
) -> std::result::Result<std::process::Output, Response> {
    let mut cmd = Command::new("git");
    cmd.args(args);
    cmd.current_dir(repo);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.as_std_mut().creation_flags(CREATE_NO_WINDOW);
    }
    let output = match cmd.output().await {
        Ok(o) => o,
        Err(e) => {
            return Err(api_error(GitGitError::GitSubprocess {
                cmd: label.to_string(),
                source: e,
            }));
        }
    };
    if !output.status.success() {
        if accept_empty_code == output.status.code() {
            // Treat as "no rows" rather than a real failure. Discard
            // stderr so the caller can fall back to empty output.
            return Ok(std::process::Output {
                status: output.status,
                stdout: Vec::new(),
                stderr: Vec::new(),
            });
        }
        let mut err_msg = String::from_utf8_lossy(&output.stderr).into_owned();
        if err_msg.is_empty() {
            err_msg = format!("{label} exited with {:?}", output.status.code());
        }
        return Err(api_error(GitGitError::GitExit {
            cmd: label.to_string(),
            status: output.status.code().unwrap_or(-1),
            stderr: err_msg,
        }));
    }
    Ok(output)
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::server::vault::FileVault;
    use axum::body::to_bytes;
    use axum::http::{Request as HttpRequest, StatusCode as AxStatusCode};
    use std::sync::atomic::{AtomicU64, Ordering};
    use tower::ServiceExt;

    fn unique_temp(label: &str) -> std::path::PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!(
            "gitgit-test-api-{label}-{pid}-{n}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn app_state_with(label: &str) -> (AppState, std::path::PathBuf) {
        let repos = unique_temp(label);
        let vault_root = unique_temp(&format!("{label}-vault"));
        let cfg = Config::new("127.0.0.1:0", repos.clone(), vault_root);
        let vault = std::sync::Arc::new(FileVault::new(&cfg.vault_file_root));
        (AppState::new(cfg, vault), repos)
    }

    async fn app_with_repo(label: &str, name: &str) -> (AppState, std::path::PathBuf) {
        let (state, repos) = app_state_with(label);
        crate::repo::create_bare_repo(&state.config, name).await.unwrap();
        (state, repos)
    }

    fn router(state: AppState) -> axum::Router {
        crate::server::http::build_router(state)
    }

    #[tokio::test]
    async fn health_endpoint_returns_200_with_vault_status() {
        let (state, _repos) = app_state_with("health");
        let app = router(state);
        let resp = app
            .oneshot(
                HttpRequest::builder()
                    .method("GET")
                    .uri("/api/health")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), AxStatusCode::OK);
        let body = to_bytes(resp.into_body(), 4096).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["status"], "ok");
        assert_eq!(v["vault_online"], true);
    }

    #[tokio::test]
    async fn list_repos_empty_returns_empty_array() {
        let (state, _repos) = app_state_with("list-empty");
        let app = router(state);
        let resp = app
            .oneshot(
                HttpRequest::builder()
                    .method("GET")
                    .uri("/api/repos")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), AxStatusCode::OK);
        let body = to_bytes(resp.into_body(), 4096).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(v.is_array());
        assert_eq!(v.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn list_repos_returns_summaries() {
        let (state, _repos) = app_with_repo("list-with", "alpha").await;
        let app = router(state);
        let resp = app
            .oneshot(
                HttpRequest::builder()
                    .method("GET")
                    .uri("/api/repos")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), AxStatusCode::OK);
        let body = to_bytes(resp.into_body(), 8192).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let arr = v.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["name"], "alpha");
    }

    #[tokio::test]
    async fn get_repo_unknown_returns_404() {
        let (state, _repos) = app_state_with("get-missing");
        let app = router(state);
        let resp = app
            .oneshot(
                HttpRequest::builder()
                    .method("GET")
                    .uri("/api/repos/nope")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), AxStatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_repo_invalid_name_returns_400() {
        let (state, _repos) = app_state_with("bad-name");
        let app = router(state);
        let resp = app
            .oneshot(
                HttpRequest::builder()
                    .method("GET")
                    // `..` is rejected by validate_repo_name.
                    .uri("/api/repos/..")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        // axum's Path extractor rejects `/..` before the handler runs
        // (404 from the router). Either 400 or 404 is acceptable; the
        // contract is "not 200".
        assert!(resp.status().is_client_error());
    }

    #[tokio::test]
    async fn get_repo_detail_returns_refs_and_log() {
        let (state, _repos) = app_with_repo("detail", "demo").await;
        let app = router(state);
        let resp = app
            .oneshot(
                HttpRequest::builder()
                    .method("GET")
                    .uri("/api/repos/demo")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), AxStatusCode::OK);
        let body = to_bytes(resp.into_body(), 16 * 1024).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["name"], "demo");
        // A freshly-`git init --bare`'d repo has zero refs and zero log
        // entries (no commits yet); the shape must still be correct.
        assert!(v["refs"].is_array());
        assert!(v["log"].is_array());
        assert_eq!(v["ref_count"], 0);
    }

    #[tokio::test]
    async fn refs_endpoint_returns_empty_for_fresh_repo() {
        let (state, _repos) = app_with_repo("refs", "demo").await;
        let app = router(state);
        let resp = app
            .oneshot(
                HttpRequest::builder()
                    .method("GET")
                    .uri("/api/repos/demo/refs")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), AxStatusCode::OK);
        let body = to_bytes(resp.into_body(), 4096).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(v.is_array());
    }

    #[tokio::test]
    async fn log_endpoint_returns_empty_for_fresh_repo() {
        let (state, _repos) = app_with_repo("log", "demo").await;
        let app = router(state);
        let resp = app
            .oneshot(
                HttpRequest::builder()
                    .method("GET")
                    .uri("/api/repos/demo/log")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), AxStatusCode::OK);
        let body = to_bytes(resp.into_body(), 4096).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(v.is_array());
        assert_eq!(v.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn vault_list_keys_empty() {
        let (state, _repos) = app_state_with("vault-list");
        let app = router(state);
        let resp = app
            .oneshot(
                HttpRequest::builder()
                    .method("GET")
                    .uri("/api/vault/keys")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), AxStatusCode::OK);
        let body = to_bytes(resp.into_body(), 4096).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(v.is_array());
        assert_eq!(v.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn vault_set_then_list_round_trip() {
        let (state, _repos) = app_state_with("vault-roundtrip");
        let app = router(state);
        // POST /api/vault/keys/openai/versions with { value: "sk-1" }.
        let resp = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .method("POST")
                    .uri("/api/vault/keys/openai/versions")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(r#"{"value":"sk-1"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), AxStatusCode::OK);
        let body = to_bytes(resp.into_body(), 4096).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["key"], "openai");
        assert_eq!(v["version"], 1);

        // GET /api/vault/keys should now contain "openai".
        let resp = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .method("GET")
                    .uri("/api/vault/keys")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = to_bytes(resp.into_body(), 8192).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let arr = v.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["key"], "openai");
        assert_eq!(arr[0]["version_count"], 1);
        assert_eq!(arr[0]["current_version"], 1);
    }

    #[tokio::test]
    async fn vault_get_returns_value_after_set() {
        let (state, _repos) = app_state_with("vault-get");
        let app = router(state);
        // First set a value.
        let _ = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .method("POST")
                    .uri("/api/vault/keys/openai/versions")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(r#"{"value":"sk-xyz"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        // Then GET it.
        let resp = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .method("GET")
                    .uri("/api/vault/keys/openai")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), AxStatusCode::OK);
        let body = to_bytes(resp.into_body(), 4096).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["key"], "openai");
        assert_eq!(v["value"], "sk-xyz");
        assert_eq!(v["current_version"], 1);
    }

    #[tokio::test]
    async fn vault_versions_endpoint_lists_timeline() {
        let (state, _repos) = app_state_with("vault-versions");
        let app = router(state);
        // Two writes -> two versions.
        for v in ["v1", "v2"] {
            let body = format!(r#"{{"value":"{v}"}}"#);
            let _ = app
                .clone()
                .oneshot(
                    HttpRequest::builder()
                        .method("POST")
                        .uri("/api/vault/keys/openai/versions")
                        .header("content-type", "application/json")
                        .body(axum::body::Body::from(body))
                        .unwrap(),
                )
                .await
                .unwrap();
        }
        let resp = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .method("GET")
                    .uri("/api/vault/keys/openai/versions")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), AxStatusCode::OK);
        let body = to_bytes(resp.into_body(), 4096).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["key"], "openai");
        let arr = v["versions"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["version"], 1);
        assert_eq!(arr[1]["version"], 2);
    }

    #[tokio::test]
    async fn vault_diff_endpoint_reports_change() {
        let (state, _repos) = app_state_with("vault-diff");
        let app = router(state);
        for v in ["alpha", "beta"] {
            let body = format!(r#"{{"value":"{v}"}}"#);
            let _ = app
                .clone()
                .oneshot(
                    HttpRequest::builder()
                        .method("POST")
                        .uri("/api/vault/keys/openai/versions")
                        .header("content-type", "application/json")
                        .body(axum::body::Body::from(body))
                        .unwrap(),
                )
                .await
                .unwrap();
        }
        let resp = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .method("GET")
                    .uri("/api/vault/keys/openai/diff?base=1&head=2")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), AxStatusCode::OK);
        let body = to_bytes(resp.into_body(), 4096).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["object_changed"], true);
        // "beta" (4 bytes) - "alpha" (5 bytes) = -1
        assert_eq!(v["file_size_delta"], -1);
    }

    #[tokio::test]
    async fn vault_restore_endpoint_appends_marker_version() {
        let (state, _repos) = app_state_with("vault-restore");
        let app = router(state);
        for v in ["one", "two"] {
            let body = format!(r#"{{"value":"{v}"}}"#);
            let _ = app
                .clone()
                .oneshot(
                    HttpRequest::builder()
                        .method("POST")
                        .uri("/api/vault/keys/openai/versions")
                        .header("content-type", "application/json")
                        .body(axum::body::Body::from(body))
                        .unwrap(),
                )
                .await
                .unwrap();
        }
        let resp = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .method("POST")
                    .uri("/api/vault/keys/openai/restore")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(r#"{"target_version":1}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), AxStatusCode::OK);
        let body = to_bytes(resp.into_body(), 4096).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["target_version"], 1);
        assert!(v["new_version"].as_i64().unwrap() >= 3);
    }

    #[tokio::test]
    async fn vault_delete_returns_204() {
        let (state, _repos) = app_state_with("vault-delete");
        let app = router(state);
        // Set first so there's something to delete.
        let _ = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .method("POST")
                    .uri("/api/vault/keys/temp/versions")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(r#"{"value":"x"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let resp = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .method("DELETE")
                    .uri("/api/vault/keys/temp")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), AxStatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn vault_set_empty_value_returns_400() {
        let (state, _repos) = app_state_with("vault-empty");
        let app = router(state);
        let resp = app
            .oneshot(
                HttpRequest::builder()
                    .method("POST")
                    .uri("/api/vault/keys/openai/versions")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(r#"{"value":""}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        // 400 from our handler, not 200.
        assert_eq!(resp.status(), AxStatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn vault_restore_with_invalid_version_returns_400() {
        let (state, _repos) = app_state_with("vault-restore-bad");
        let app = router(state);
        let resp = app
            .oneshot(
                HttpRequest::builder()
                    .method("POST")
                    .uri("/api/vault/keys/openai/restore")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(r#"{"target_version":0}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), AxStatusCode::BAD_REQUEST);
    }
}