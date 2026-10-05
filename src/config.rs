//! Minimal runtime configuration for the `gitgit` binary.

use std::path::PathBuf;

use crate::error::{GitGitError, Result};

/// Bind address the server listens on.
///
/// `[FACT]` Was `0.0.0.0:8080`, which made every interface of every
/// network the machine was attached to reachable by default — with no
/// authentication layer on `/api/*` (`src/server/api.rs`:
/// `auth_optional` is a no-op, is never called, and carries
/// `#[allow(dead_code)]`) and with `GET /api/vault/keys/:key` returning
/// stored credentials in the clear. A default that exposes a
/// credential store to the local network is not a default any product
/// should ship.
///
/// Loopback is the safe default, and it is also what this repository's
/// own architecture notes already prescribe
/// (`docs_archive_rust_impl_2026_08_26/architecture/tech-selection.md`:
/// `admin_listen: "127.0.0.1:3001"  # 默认仅本机`) and what the desktop
/// shell has always used (`apps/gm-desktop/src-tauri/src/state.rs`:
/// `127.0.0.1:38080`). Only the CLI disagreed.
///
/// Exposure is still one flag away, and deliberately explicit:
/// `gitgit serve --bind 0.0.0.0:8080`. Opting in to network exposure is
/// now something a person types on purpose rather than something they
/// inherit by omitting an argument.
///
/// `[UNVERIFIED-FACT]` The default was changed on the grounds that no
/// code in this repository depends on the wildcard address — the only
/// occurrences were this constant and three prose references. Any
/// existing deployment relying on LAN reachability without passing
/// `--bind` will stop being reachable; that is the intended effect, not
/// a regression, and `--bind 0.0.0.0:8080` restores it.
pub const DEFAULT_BIND: &str = "127.0.0.1:8080";

/// Subdirectory under the current working directory where bare repos live.
pub const DEFAULT_REPOS_DIR: &str = "repos";

/// Default on-disk root for the V0 Credential Vault (per ADR-0021).
///
/// Per ADR-0021 §1.2 the MVP is a single-crate binary with zero
/// platform-specific code; the credential backend is therefore the
/// `FileVault` rooted here. Operators can override via
/// `GITGIT_VAULT_FILE_ROOT` (or `--vault-file-root` on the CLI). The
/// commit that exposes a `MinioVault` runtime toggled by
/// `GITGIT_VAULT_BACKEND=minio` follows in V0 close-out.
pub const DEFAULT_VAULT_FILE_ROOT: &str = ".gitgit/vault";

/// Default *username* for the admin basic-auth account.
///
/// A username is an identifier, not a secret, so keeping it as a constant
/// is fine. The password is deliberately NOT here — see
/// [`ADMIN_PASS_ENV`] and [`crate::server::auth::resolve_admin_credential`].
pub const DEFAULT_ADMIN_USER: &str = "admin";

/// Environment variable naming the admin username.
pub const ADMIN_USER_ENV: &str = "GITGIT_ADMIN_USER";

/// Environment variable holding the admin password.
///
/// Named for the existing convention (`GITGIT_VAULT_FILE_ROOT`,
/// `GITGIT_VAULT_BACKEND`, `GITGIT_AI_API_KEY`): `GITGIT_` + subject.
/// This is the highest-precedence source for the password; the vault key
/// [`ADMIN_PASSWORD_VAULT_KEY`] is consulted when it is unset.
pub const ADMIN_PASS_ENV: &str = "GITGIT_ADMIN_PASS";

/// Vault key under which the desktop Settings page stores the admin
/// password.
///
/// The desktop shell wrote to this key long before anything read it, so
/// the key is the contract that already exists; the server now honours it
/// on startup instead of ignoring it.
pub const ADMIN_PASSWORD_VAULT_KEY: &str = "gitgit.password";

/// `true` iff a `--bind` value names a loopback interface.
///
/// Used by the startup exposure policy: a non-loopback bind with a
/// password nobody chose is refused rather than silently accepted.
///
/// Accepted forms are the literal loopback IPv4 range (`127.0.0.0/8`),
/// the loopback IPv6 address (`::1`), and the hostname `localhost`.
/// Anything else — including `0.0.0.0`, an empty host, or an
/// unresolvable name — is treated as non-loopback, so the policy fails
/// closed on ambiguity.
pub fn is_loopback_bind(bind: &str) -> bool {
    let host = match bind.rsplit_once(':') {
        // No port, or a bare IPv6 literal with no port: nothing to strip.
        None => bind,
        Some((host, _port)) => host,
    };
    // Bracketed IPv6, e.g. `[::1]:8080`.
    let host = host.trim_start_matches('[').trim_end_matches(']');
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    match host.parse::<std::net::IpAddr>() {
        Ok(std::net::IpAddr::V4(v4)) => v4.is_loopback(),
        Ok(std::net::IpAddr::V6(v6)) => v6.is_loopback(),
        // Not an IP at all: not something we can call loopback.
        Err(_) => false,
    }
}

/// Resolved, validated configuration for one server invocation.
#[derive(Debug, Clone)]
pub struct Config {
    /// Address to bind (e.g. `127.0.0.1:8080`).
    pub bind: String,
    /// Directory under which bare repos are stored (`<dir>/<name>.git/`).
    pub repos_dir: PathBuf,
    /// V0 Credential Vault root directory (per ADR-0021 §1.2 default).
    /// Currently only used by `FileVault`; reserved for a minIO backend
    /// in V0 close-out.
    pub vault_file_root: PathBuf,
}

impl Config {
    /// Build a config from explicit values, defaulting the bind address
    /// and the vault file root.
    pub fn new(
        bind: impl Into<String>,
        repos_dir: impl Into<PathBuf>,
        vault_file_root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            bind: bind.into(),
            repos_dir: repos_dir.into(),
            vault_file_root: vault_file_root.into(),
        }
    }

    /// Ensure that `repos_dir` exists and is a directory. Creates it if missing.
    pub fn ensure_repos_dir(&self) -> Result<()> {
        if !self.repos_dir.exists() {
            std::fs::create_dir_all(&self.repos_dir)?;
            return Ok(());
        }
        if !self.repos_dir.is_dir() {
            return Err(GitGitError::InvalidReposDir(self.repos_dir.clone()));
        }
        Ok(())
    }

    /// Ensure that `vault_file_root` exists and is a directory. Creates
    /// it if missing. Mirrors `ensure_repos_dir`'s semantics.
    pub fn ensure_vault_root(&self) -> Result<()> {
        if !self.vault_file_root.exists() {
            std::fs::create_dir_all(&self.vault_file_root)?;
            return Ok(());
        }
        if !self.vault_file_root.is_dir() {
            return Err(GitGitError::InvalidReposDir(self.vault_file_root.clone()));
        }
        Ok(())
    }

    /// Resolve the on-disk path for a repo name: `<repos_dir>/<name>.git`.
    ///
    /// Git clients address bare repos with the conventional `.git` suffix
    /// (e.g. `demo.git`), so we accept both `demo` and `demo.git` and
    /// normalize to a single canonical form (`demo.git`) on disk.
    pub fn repo_path(&self, name: &str) -> Result<PathBuf> {
        validate_repo_name(name)?;
        let stem = name.strip_suffix(".git").unwrap_or(name);
        // Re-validate after stripping the suffix to make sure a name
        // like `..git` doesn't smuggle a path traversal.
        validate_repo_name(stem)?;
        Ok(self.repos_dir.join(format!("{stem}.git")))
    }
}

/// Validate that a repository name is safe to embed in a path.
///
/// Rejects empty names, names with `..`, names with path separators, and
/// control characters. The name may contain `.` (e.g. `demo.git`) since
/// that is the conventional bare-repo name suffix.
pub fn validate_repo_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(GitGitError::InvalidRepoName(name.to_string()));
    }
    if name.contains('/') || name.contains('\\') {
        return Err(GitGitError::InvalidRepoName(name.to_string()));
    }
    // Reject `..` as a whole segment (path traversal). We disallow any
    // occurrence of `..` to be safe even within a name like `foo..bar`,
    // since `foo..bar` could be ambiguous.
    if name == ".." || name.contains("..") {
        return Err(GitGitError::InvalidRepoName(name.to_string()));
    }
    // Reject names that start with a dot (hidden / traversal-like).
    if name.starts_with('.') {
        return Err(GitGitError::InvalidRepoName(name.to_string()));
    }
    // Reject whitespace and control characters.
    if name.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(GitGitError::InvalidRepoName(name.to_string()));
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn default_bind_is_loopback() {
        assert!(is_loopback_bind(DEFAULT_BIND));
    }

    #[test]
    fn loopback_binds_are_recognised() {
        for bind in [
            "127.0.0.1:8080",
            "127.0.0.53:9000",
            "localhost:8080",
            "LOCALHOST:8080",
            "[::1]:8080",
            "127.0.0.1",
        ] {
            assert!(is_loopback_bind(bind), "{bind} should be loopback");
        }
    }

    #[test]
    fn non_loopback_binds_are_rejected() {
        // `0.0.0.0` is the wildcard: it is the exact case the exposure
        // policy has to refuse, so it must not read as loopback.
        for bind in [
            "0.0.0.0:8080",
            "192.168.1.10:8080",
            "10.0.0.5:9000",
            "example.test:8080",
            "[::]:8080",
            ":8080",
            "",
        ] {
            assert!(!is_loopback_bind(bind), "{bind} must not be loopback");
        }
    }

    /// The regression guard for "a password baked into an executable is a
    /// password everyone has".
    ///
    /// `[FACT]` This crate used to ship `pub const ADMIN_PASS: &str =
    /// "admin";` in this very file, and `server::auth` compared every
    /// request against it. A const is not a secret: it is in the binary,
    /// in the debug symbols, and in anyone who runs `strings` on it.
    ///
    /// Asserting on the *source tree* is the only check that catches a
    /// re-introduction, because by the time a credential is in the binary
    /// no runtime test can tell it from a legitimately configured one.
    /// Scoped to `src/` on purpose — `scripts/smoke.ps1` legitimately
    /// mentions the historical default and is not compiled in.
    ///
    /// The needles are assembled at run time out of fragments. Written
    /// out in full they would be sitting in `config.rs` — which is
    /// inside the very directory being scanned — and the test would
    /// always find itself.
    #[test]
    fn no_compiled_source_hard_codes_the_admin_password() {
        const USER_FRAGMENT: &str = "admin";
        const PASS_FRAGMENT: &str = "admin";
        let colon_pair = [USER_FRAGMENT, PASS_FRAGMENT].join(":");
        let const_decl = ["ADMIN_PASS", ": &str"].concat();

        let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut offenders: Vec<String> = Vec::new();
        let mut stack = vec![src_dir.clone()];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                    continue;
                }
                let body = std::fs::read_to_string(&path).unwrap();
                let rel = path
                    .strip_prefix(env!("CARGO_MANIFEST_DIR"))
                    .unwrap_or(&path)
                    .display()
                    .to_string();
                // `[FACT]` Comments are not compiled in, and the file that
                // has to explain this change is also the file that has to
                // be scanned. Matching the raw text meant the guard found
                // its own documentation — the `ADMIN_PASS: &str` in this
                // function's doc comment, and the historical pair quoted in
                // `server::auth` — and failed on a tree with no credential
                // in it. Lines that open a `//` comment or sit inside a
                // `/** ... */` block are dropped first. This is the same
                // rule the rest of the repo's source scans already use.
                let code: String = body
                    .lines()
                    .filter(|line| {
                        let t = line.trim_start();
                        !t.starts_with("//") && !t.starts_with('*')
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                if code.contains(&colon_pair) {
                    // Assembled at run time: written out in full, this
                    // message would be the very needle the guard looks
                    // for, sitting in a scanned file.
                    offenders.push(format!(
                        "{rel}: literal {}:{} credential",
                        USER_FRAGMENT, PASS_FRAGMENT
                    ));
                }
                if code.contains(&const_decl) {
                    offenders.push(format!("{rel}: compiled-in password constant"));
                }
            }
        }
        assert!(
            offenders.is_empty(),
            "a compiled-in admin credential is back:\n  {}",
            offenders.join("\n  ")
        );
    }
}
