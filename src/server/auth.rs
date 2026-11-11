//! HTTP Basic auth for the smart-HTTP Git protocol **and** the `/api/*`
//! REST surface.
//!
//! Reads are open; writes require auth — and so does every `/api` route
//! except `GET /api/health` (see [`crate::server::api`]).
//!
//! # Where the credential comes from
//!
//! `[FACT]` This module used to compare every request against two
//! compile-time constants, `config::ADMIN_USER` / `config::ADMIN_PASS`,
//! both of them `"admin"`. That is not a credential, it is a constant
//! that happens to be printed in a book: anyone with the binary already
//! had it. The password is now resolved at startup and never lives in
//! the binary.
//!
//! Resolution order, highest precedence first (see
//! [`resolve_admin_credential`]):
//! 1. `GITGIT_ADMIN_PASS` — an operator who deploys sets this.
//! 2. the vault key `gitgit.password` — what the desktop Settings page
//!    writes, so that page's password finally has an effect.
//! 3. a freshly generated random password, printed once at startup.
//!
//! # Why the value is cached rather than resolved per request
//!
//! The check has to stay **synchronous**:
//!
//! * `git-receive-pack` is in the middle of streaming a packfile to a
//!   subprocess, and making its auth gate `async` buys nothing — the
//!   credential cannot change mid-push anyway.
//! * Reading the vault is `async` (`vault.get(..).await`). Resolving per
//!   request would put a file read and a JSON parse in front of *every*
//!   request, including each of the two per push.
//!
//! So the resolved value is held in an [`AdminCredentialStore`] — an
//! `Arc` behind an `RwLock` — that the request path reads with a cheap
//! clone. The lock is written exactly once per process unless a
//! rotation says otherwise, so the read path is uncontended in practice.
//!
//! An earlier draft of this decision suggested a `tokio::sync::OnceCell`.
//! That was rejected: a `OnceCell` is write-once, and the desktop app
//! can change the password at runtime, so it would force either a
//! silently-stale credential or a process restart with no way to say so.
//! See [`AdminCredentialStore::rotate_password`].

use std::sync::{Arc, RwLock};

use axum::http::header::AUTHORIZATION;
use axum::http::HeaderMap;

use crate::config::{
    is_loopback_bind, ADMIN_PASSWORD_VAULT_KEY, ADMIN_PASS_ENV, ADMIN_USER_ENV, DEFAULT_ADMIN_USER,
};
use crate::error::{GitGitError, Result};
use crate::server::vault::Vault;

/// The admin username + password in force for this process.
///
/// Never logged and never serialized. [`Debug`] is implemented by hand
/// so that a stray `{:?}` in a log line cannot leak the password.
pub struct AdminCredential {
    pub user: String,
    pass: String,
}

impl AdminCredential {
    pub fn new(user: impl Into<String>, pass: impl Into<String>) -> Self {
        Self {
            user: user.into(),
            pass: pass.into(),
        }
    }

    /// The password, for the startup banner that has to tell the operator
    /// what was generated. Callers must not log the result.
    pub fn password(&self) -> &str {
        &self.pass
    }
}

impl std::fmt::Debug for AdminCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdminCredential")
            .field("user", &self.user)
            .field("pass", &"<redacted>")
            .finish()
    }
}

/// Where a resolved password came from.
///
/// The distinction matters for one thing only: a **generated** password
/// on a **non-loopback** bind is refused (see [`enforce_exposure_policy`]).
/// A password an operator typed is their decision to make.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialSource {
    /// `GITGIT_ADMIN_PASS`.
    Env,
    /// The vault key `gitgit.password`.
    Vault,
    /// Generated at startup because nothing was configured.
    Generated,
}

impl CredentialSource {
    pub fn as_str(self) -> &'static str {
        match self {
            CredentialSource::Env => ADMIN_PASS_ENV,
            CredentialSource::Vault => ADMIN_PASSWORD_VAULT_KEY,
            CredentialSource::Generated => "generated",
        }
    }
}

/// A resolved credential plus provenance.
#[derive(Debug)]
pub struct ResolvedCredential {
    pub credential: Arc<AdminCredential>,
    pub source: CredentialSource,
}

/// Cached, rotatable home of the admin credential.
///
/// Reads are synchronous and cheap: an `Arc` clone out of an `RwLock`.
pub struct AdminCredentialStore {
    inner: RwLock<Arc<AdminCredential>>,
}

impl AdminCredentialStore {
    pub fn new(credential: AdminCredential) -> Self {
        Self {
            inner: RwLock::new(Arc::new(credential)),
        }
    }

    /// Adopt an already-shared credential (typically the output of
    /// [`resolve_admin_credential`]) without copying the secret.
    pub fn from_shared(credential: Arc<AdminCredential>) -> Self {
        Self {
            inner: RwLock::new(credential),
        }
    }

    /// A store holding a random password nobody has been told yet.
    ///
    /// Used by [`crate::server::http::AppState::new`], which is the
    /// constructor every existing test uses. It deliberately does *not*
    /// fall back to a known password: a test that wants to authenticate
    /// must say which credential it is using by calling
    /// [`Self::rotate_password`].
    pub fn generated() -> Self {
        Self::new(AdminCredential::new(
            DEFAULT_ADMIN_USER,
            generate_password(),
        ))
    }

    /// Current credential. Synchronous by design — see the module docs.
    pub fn current(&self) -> Arc<AdminCredential> {
        // A poisoned lock means some other thread panicked while holding
        // it. The data itself is an immutable `Arc`, so it is still
        // valid; refusing to serve would be strictly worse than serving
        // the last-known-good credential.
        match self.inner.read() {
            Ok(g) => Arc::clone(&g),
            Err(poisoned) => Arc::clone(&poisoned.into_inner()),
        }
    }

    /// Replace the password in force, immediately.
    ///
    /// Takes effect for the very next request. There is no window in
    /// which the old password is still accepted once this returns.
    pub fn rotate_password(&self, pass: impl Into<String>) {
        let next = AdminCredential::new(self.current().user.clone(), pass);
        match self.inner.write() {
            Ok(mut g) => *g = Arc::new(next),
            Err(poisoned) => *poisoned.into_inner() = Arc::new(next),
        }
    }
}

impl std::fmt::Debug for AdminCredentialStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdminCredentialStore")
            .field("current", &self.current())
            .finish()
    }
}

/// Generate a 128-bit random password as 32 lowercase hex characters.
///
/// `[FACT]` `uuid` is already a direct dependency of this crate
/// (`Cargo.toml`, `features = ["v4"]`), so this needs no new dependency.
/// `Uuid::new_v4` draws from the OS CSPRNG and is infallible, which keeps
/// this free of `unwrap`/`panic` (deny-listed crate-wide).
pub fn generate_password() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

/// Resolve the admin credential for this process.
///
/// Order, highest precedence first:
/// 1. `GITGIT_ADMIN_PASS` / `GITGIT_ADMIN_USER` from the environment,
/// 2. the vault key `gitgit.password` (what the desktop Settings page
///    writes — the whole point of reading it is that page stops lying),
/// 3. a generated random password.
///
/// An empty or whitespace-only environment value is treated as unset
/// rather than honoured, matching how `main.rs` already handles
/// `GITGIT_VAULT_FILE_ROOT`.
///
/// A vault read that *errors* is not fatal: the caller is told via
/// `source` only on success, so we fall through to generation. That is a
/// deliberate choice — a broken vault should degrade to "unknown random
/// password" rather than refusing to start a Git server.
pub async fn resolve_admin_credential<V: Vault + ?Sized>(vault: &V) -> Result<ResolvedCredential> {
    let user = match std::env::var(ADMIN_USER_ENV) {
        Ok(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => DEFAULT_ADMIN_USER.to_string(),
    };

    if let Ok(v) = std::env::var(ADMIN_PASS_ENV) {
        if !v.trim().is_empty() {
            return Ok(ResolvedCredential {
                credential: Arc::new(AdminCredential::new(user, v)),
                source: CredentialSource::Env,
            });
        }
    }

    match vault.get(ADMIN_PASSWORD_VAULT_KEY).await {
        Ok(Some(pass)) if !pass.trim().is_empty() => Ok(ResolvedCredential {
            credential: Arc::new(AdminCredential::new(user, pass)),
            source: CredentialSource::Vault,
        }),
        Ok(_) => Ok(ResolvedCredential {
            credential: Arc::new(AdminCredential::new(user, generate_password())),
            source: CredentialSource::Generated,
        }),
        Err(e) => {
            tracing::warn!(
                error = %e,
                key = ADMIN_PASSWORD_VAULT_KEY,
                "could not read the admin password from the vault; generating one instead"
            );
            Ok(ResolvedCredential {
                credential: Arc::new(AdminCredential::new(user, generate_password())),
                source: CredentialSource::Generated,
            })
        }
    }
}

/// Refuse to start on an exposed interface with a password nobody chose.
///
/// `[FACT]` The bind default is loopback (`config::DEFAULT_BIND`), where
/// an unknown generated password is not an exposure — the port is not
/// reachable from off the machine. On `0.0.0.0` it very much is: the
/// server would come up, protect itself with a secret printed once into
/// a log the operator may not be reading, and serve a vault whose keys
/// `GET /api/vault/keys/:key` returns in the clear to anyone on the LAN.
///
/// So the combination "not loopback" + "password was generated rather
/// than configured" is an error. Naming the env var is the point: the
/// fix is one line.
pub fn enforce_exposure_policy(bind: &str, source: CredentialSource) -> Result<()> {
    if is_loopback_bind(bind) || source != CredentialSource::Generated {
        return Ok(());
    }
    Err(GitGitError::Http(format!(
        "refusing to bind {bind}: it is not a loopback address and no admin password \
         was configured, so the only password available would be one generated at \
         startup. Set {ADMIN_PASS_ENV} (and optionally {ADMIN_USER_ENV}) to the \
         password you want, or bind a loopback address such as {}.",
        crate::config::DEFAULT_BIND
    )))
}

/// `true` iff the `Authorization` header carries valid basic credentials
/// for `credential`.
///
/// Written as a single predicate rather than a `parse_basic` helper that
/// returns `(user, pass)` pairs: the decoded bytes are owned by this
/// function, so handing out `&str` slices into them is a lifetime error.
/// The comparison happens here, while the buffer is still alive.
///
/// A malformed header is a `false`, not an error. Nothing about a bad
/// `Authorization` is exceptional enough to deserve a status of its own —
/// the caller answers `401` either way, and reporting "this header was
/// malformed" separately would only create a second way to say "no".
pub fn check_basic(headers: &HeaderMap, credential: &AdminCredential) -> bool {
    let Some(value) = headers.get(AUTHORIZATION) else {
        return false;
    };
    let Ok(value) = value.to_str() else {
        return false;
    };
    let Some(rest) = value.strip_prefix("Basic ") else {
        return false;
    };
    let Ok(bytes) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, rest.trim())
    else {
        return false;
    };
    let Ok(decoded) = std::str::from_utf8(&bytes) else {
        return false;
    };
    // A password may itself contain `:`; only the first one separates.
    let Some((user, pass)) = decoded.split_once(':') else {
        return false;
    };
    user == credential.user && pass == credential.password()
}

/// Enforce basic auth. Returns [`GitGitError::Unauthenticated`] if missing
/// or wrong, which the HTTP layer translates to 401.
pub fn require_basic(headers: &HeaderMap, credential: &AdminCredential) -> Result<()> {
    if check_basic(headers, credential) {
        Ok(())
    } else {
        Err(GitGitError::Unauthenticated)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    use base64::Engine;

    fn hdrs(value: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(AUTHORIZATION, HeaderValue::from_str(value).unwrap());
        h
    }

    fn cred() -> AdminCredential {
        AdminCredential::new("admin", "hunter2")
    }

    fn encoded(user: &str, pass: &str) -> String {
        format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(format!("{user}:{pass}"))
        )
    }

    #[test]
    fn missing_header_is_denied() {
        let h = HeaderMap::new();
        assert!(!check_basic(&h, &cred()));
        assert!(require_basic(&h, &cred()).is_err());
    }

    #[test]
    fn configured_credentials_pass() {
        let h = hdrs(&encoded("admin", "hunter2"));
        assert!(check_basic(&h, &cred()));
        assert!(require_basic(&h, &cred()).is_ok());
    }

    #[test]
    fn wrong_password_is_denied() {
        let h = hdrs(&encoded("admin", "nope"));
        assert!(!check_basic(&h, &cred()));
    }

    #[test]
    fn malformed_authorization_is_denied() {
        let h = hdrs("Bearer xyz");
        assert!(!check_basic(&h, &cred()));
    }

    /// The compiled-in `admin` / `admin` pair must no longer authenticate.
    ///
    /// Against the code this replaces, `check_basic` compared against
    /// `config::ADMIN_PASS`, which was the literal `"admin"` — so this
    /// assertion passed for `admin:admin` and the server accepted it
    /// from anyone on the network.
    #[test]
    fn the_old_admin_admin_pair_is_rejected() {
        let h = hdrs(&encoded("admin", "admin"));
        assert!(
            !check_basic(&h, &cred()),
            "the historical default credential must not authenticate"
        );
    }

    #[test]
    fn wrong_username_is_denied() {
        let h = hdrs(&encoded("root", "hunter2"));
        assert!(!check_basic(&h, &cred()));
    }

    #[test]
    fn a_password_containing_a_colon_is_accepted_whole() {
        // `split_once(':')` keeps the rest verbatim, so a password with a
        // colon round-trips instead of being truncated at the first one.
        let c = AdminCredential::new("admin", "a:b:c");
        let h = hdrs(&encoded("admin", "a:b:c"));
        assert!(check_basic(&h, &c));
    }

    #[test]
    fn generated_passwords_differ_between_calls() {
        // If this ever becomes constant the "generate and print once"
        // story is a lie: every deployment would share a password.
        let a = generate_password();
        let b = generate_password();
        assert_ne!(a, b);
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn store_rotation_takes_effect_on_the_next_read() {
        let store = AdminCredentialStore::new(AdminCredential::new("admin", "first"));
        assert!(check_basic(
            &hdrs(&encoded("admin", "first")),
            &store.current()
        ));
        store.rotate_password("second");
        let current = store.current();
        assert!(check_basic(&hdrs(&encoded("admin", "second")), &current));
        // The old password stops working immediately, not "eventually".
        assert!(!check_basic(&hdrs(&encoded("admin", "first")), &current));
    }

    #[test]
    fn store_rotation_preserves_the_username() {
        let store = AdminCredentialStore::new(AdminCredential::new("operator", "first"));
        store.rotate_password("second");
        assert_eq!(store.current().user, "operator");
    }

    #[test]
    fn generated_store_accepts_nobody_by_default() {
        // `AppState::new` builds one of these, so a test that forgets to
        // set a credential gets a 401 rather than a free pass.
        let store = AdminCredentialStore::generated();
        assert!(!check_basic(
            &hdrs(&encoded("admin", "admin")),
            &store.current()
        ));
    }

    #[test]
    fn debug_output_does_not_contain_the_password() {
        let c = AdminCredential::new("admin", "hunter2");
        let rendered = format!("{c:?} {c:?}");
        assert!(!rendered.contains("hunter2"), "leaked in: {rendered}");
        assert!(rendered.contains("<redacted>"));
    }

    #[test]
    fn non_loopback_bind_with_a_generated_password_is_refused() {
        let err = enforce_exposure_policy("0.0.0.0:8080", CredentialSource::Generated);
        assert!(err.is_err());
        let msg = err.unwrap_err().to_string();
        // The error has to be actionable: it must name the env var that
        // fixes it, or it is just a refusal.
        assert!(msg.contains(ADMIN_PASS_ENV), "unhelpful error: {msg}");
    }

    #[test]
    fn non_loopback_bind_with_a_configured_password_is_allowed() {
        assert!(enforce_exposure_policy("0.0.0.0:8080", CredentialSource::Env).is_ok());
        assert!(enforce_exposure_policy("0.0.0.0:8080", CredentialSource::Vault).is_ok());
    }

    #[test]
    fn loopback_bind_with_a_generated_password_is_allowed() {
        // Zero setup on the default interface is the point of generating.
        assert!(enforce_exposure_policy("127.0.0.1:8080", CredentialSource::Generated).is_ok());
    }
}
