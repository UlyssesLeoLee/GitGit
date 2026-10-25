//! Minimal runtime configuration for the `gitgit` binary.

use std::path::PathBuf;

use crate::error::{GitGitError, Result};
use crate::server::vault::MinioVaultConfig;

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
/// `GITGIT_VAULT_FILE_ROOT` (or `--vault-file-root` on the CLI).
///
/// `[FACT]` The comment here used to promise "a `MinioVault` runtime
/// toggled by `GITGIT_VAULT_BACKEND=minio`" in V0 close-out. That
/// commit had never landed: `build_vault()` hard-coded the `FileVault`,
/// so the S3 implementation was unreachable from the product binary and
/// only the test suite could reach it. The toggle now exists, spelled
/// `GITGIT_VAULT_BACKEND` with the values `file` (this default) and
/// `s3`.
pub const DEFAULT_VAULT_FILE_ROOT: &str = ".gitgit/vault";

/// Environment variable that selects the credential-vault backend.
///
/// `[FACT]` The name was promised by the doc comment on
/// [`DEFAULT_VAULT_FILE_ROOT`] before anything read it. It is read
/// today, and an unrecognised value is an error rather than a silent
/// fallback to the file vault.
pub const VAULT_BACKEND_ENV: &str = "GITGIT_VAULT_BACKEND";

/// Default S3-compatible endpoint for the `s3` credential-vault backend.
///
/// `[FACT]` This is the address a local S3-compatible server listens on
/// by default. It is a default, not a promise that anything is
/// listening there.
pub const DEFAULT_S3_ENDPOINT: &str = "http://127.0.0.1:9000";

/// Default bucket name for the `s3` credential-vault backend.
pub const DEFAULT_S3_BUCKET: &str = "gitgit-vault";

/// Default SigV4 region for the `s3` credential-vault backend.
///
/// `[FACT]` S3-compatible stores accept any region string; `us-east-1`
/// is the SigV4 default and keeps the signing parameters boring.
pub const DEFAULT_S3_REGION: &str = "us-east-1";

/// The S3 key-prefix value that means "not configured".
///
/// `[FACT]` The key prefix has **no default**. `src/server/vault.rs`
/// documents the prefix as "the natural way to isolate multiple gitgit
/// instances or to share a single bucket across unrelated tooling
/// without collision" -- a shared default would defeat the one job the
/// prefix has. Two instances that both fell back to the same prefix
/// would write the same object keys and the same version sidecars, and
/// each could read or overwrite the other's credentials.
///
/// `[FACT]` The prefix also has to be stable across restarts: a
/// namespace derived per boot would leave an instance unable to find
/// its own history.
///
/// `[FACT]` The vault's own contract still allows an empty prefix
/// (objects live at the bucket root). What this layer refuses is to
/// *supply* one, because then "not configured" and "explicitly the
/// bucket root" would be the same value -- and the first one is a
/// collision waiting to happen.
pub const S3_KEY_PREFIX_UNSET: &str = "";

/// Environment variable holding the S3-compatible endpoint.
pub const S3_ENDPOINT_ENV: &str = "GITGIT_S3_ENDPOINT";

/// Environment variable holding the bucket name.
pub const S3_BUCKET_ENV: &str = "GITGIT_S3_BUCKET";

/// Environment variable holding the SigV4 region.
pub const S3_REGION_ENV: &str = "GITGIT_S3_REGION";

/// Environment variable holding the S3 access key.
pub const S3_ACCESS_KEY_ENV: &str = "GITGIT_S3_ACCESS_KEY";

/// Environment variable holding the S3 secret key.
pub const S3_SECRET_KEY_ENV: &str = "GITGIT_S3_SECRET_KEY";

/// Environment variable holding the object key prefix.
pub const S3_KEY_PREFIX_ENV: &str = "GITGIT_S3_KEY_PREFIX";

/// Which credential-vault backend the server should use.
///
/// `[FACT]` Until the backend became selectable, the product binary
/// could only ever use the local file vault: `build_vault()` in
/// `src/main.rs` constructed a `FileVault` unconditionally, which left
/// the S3 implementation (`MinioVault`) reachable only from the test
/// suite. This enum is what makes the choice real.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VaultBackend {
    /// The local file vault — the default.
    #[default]
    File,
    /// The S3-compatible object store.
    S3,
}

impl VaultBackend {
    /// The canonical name of this backend, as spelled by
    /// `GITGIT_VAULT_BACKEND` and `--vault-backend`.
    pub const fn as_str(self) -> &'static str {
        match self {
            VaultBackend::File => "file",
            VaultBackend::S3 => "s3",
        }
    }
}

impl std::fmt::Display for VaultBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for VaultBackend {
    type Err = GitGitError;

    /// Parse a backend name, case-insensitively.
    ///
    /// `[FACT]` An unknown value is an error, never a silent fallback to
    /// the file vault. Falling back would leave an operator believing
    /// they were writing to object storage while their credentials went
    /// to a local directory.
    fn from_str(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "file" => Ok(VaultBackend::File),
            "s3" => Ok(VaultBackend::S3),
            other => Err(GitGitError::InvalidVaultBackend(other.to_string())),
        }
    }
}

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
#[derive(Clone)]
pub struct Config {
    /// Address to bind (e.g. `127.0.0.1:8080`).
    pub bind: String,
    /// Directory under which bare repos are stored (`<dir>/<name>.git/`).
    pub repos_dir: PathBuf,
    /// V0 Credential Vault root directory (per ADR-0021 §1.2 default).
    /// Used by the `file` backend; the `s3` backend ignores it.
    pub vault_file_root: PathBuf,
    /// Which credential-vault backend to use.
    pub vault_backend: VaultBackend,
    /// Settings for the `s3` backend.
    ///
    /// `[FACT]` This is the existing [`MinioVaultConfig`] struct, not a
    /// second copy of the same six fields: the S3 implementation already
    /// defines them, and duplicating them here would be one more place
    /// for the two to drift apart.
    pub s3_vault: MinioVaultConfig,
}

/// Redact a credential for debug output.
///
/// `[FACT]` `MinioVaultConfig` derives `Debug`, so it prints the secret
/// key in the clear. A `Config` that carries one must not print it.
fn redact(secret: &str) -> &str {
    if secret.trim().is_empty() {
        "<unset>"
    } else {
        "<redacted>"
    }
}

impl std::fmt::Debug for Config {
    /// Print the configuration without the credentials.
    ///
    /// `[FACT]` The derived `Debug` would have printed the S3 secret key
    /// in the clear. The vault layer already redacts its own credentials
    /// (`MinioVault`'s `Debug`), so the configuration does the same.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("bind", &self.bind)
            .field("repos_dir", &self.repos_dir)
            .field("vault_file_root", &self.vault_file_root)
            .field("vault_backend", &self.vault_backend)
            .field("s3_endpoint", &self.s3_vault.endpoint)
            .field("s3_bucket", &self.s3_vault.bucket)
            .field("s3_region", &self.s3_vault.region)
            .field("s3_key_prefix", &self.s3_vault.key_prefix)
            .field("s3_access_key", &redact(&self.s3_vault.access_key))
            .field("s3_secret_key", &redact(&self.s3_vault.secret_key))
            .finish()
    }
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
            vault_backend: VaultBackend::default(),
            s3_vault: MinioVaultConfig {
                endpoint: DEFAULT_S3_ENDPOINT.to_string(),
                bucket: DEFAULT_S3_BUCKET.to_string(),
                region: DEFAULT_S3_REGION.to_string(),
                // No default key prefix: the prefix is the isolation
                // between two instances that share a bucket.
                key_prefix: S3_KEY_PREFIX_UNSET.to_string(),
                // No default credentials: a made-up key would fail at
                // the first request with a 403 that reads like a
                // permissions problem.
                access_key: String::new(),
                secret_key: String::new(),
            },
        }
    }

    /// The S3 vault configuration, or an error if it is incomplete.
    ///
    /// `[FACT]` Credentials have no default. Selecting the `s3` backend
    /// without an access key and a secret key is an error, because the
    /// alternative — connecting with an invented credential — produces a
    /// 403 on the first read and leaves the operator with no way to
    /// tell that the key they configured was never used.
    ///
    /// `[FACT]` The key prefix has no default either, for a different
    /// reason: the prefix is the isolation. An instance that fell back
    /// to a shared prefix would write the same object keys as another
    /// instance on the same bucket and could read or overwrite its
    /// credentials. The prefix is checked first because it is a
    /// property of the deployment, while the credentials are the
    /// secrets the deployment uses.
    pub fn s3_vault_config(&self) -> Result<MinioVaultConfig> {
        if self.s3_vault.key_prefix.trim().is_empty() {
            return Err(GitGitError::MissingVaultKeyPrefix(
                S3_KEY_PREFIX_ENV,
                "s3-key-prefix",
            ));
        }
        if self.s3_vault.access_key.trim().is_empty() {
            return Err(GitGitError::MissingVaultCredential(
                "an access key",
                S3_ACCESS_KEY_ENV,
            ));
        }
        if self.s3_vault.secret_key.trim().is_empty() {
            return Err(GitGitError::MissingVaultCredential(
                "a secret key",
                S3_SECRET_KEY_ENV,
            ));
        }
        Ok(self.s3_vault.clone())
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

    /// `[FACT]` The backend name is operator-facing text: it comes from
    /// an environment variable or a command line, and people type it in
    /// whatever case they are used to. The parsing has to accept that
    /// and still be exact about which backend it selected — a test that
    /// only checked "it parsed" would pass for a parser that always
    /// returned the file vault.
    #[test]
    fn vault_backend_names_parse_to_the_backend_they_name() {
        assert_eq!("file".parse::<VaultBackend>().unwrap(), VaultBackend::File);
        assert_eq!("s3".parse::<VaultBackend>().unwrap(), VaultBackend::S3);
        assert_eq!("S3".parse::<VaultBackend>().unwrap(), VaultBackend::S3);
        assert_eq!("File".parse::<VaultBackend>().unwrap(), VaultBackend::File);
        assert_eq!(" s3 ".parse::<VaultBackend>().unwrap(), VaultBackend::S3);
    }

    /// `[FACT]` An unknown backend name is an error, not a fallback to
    /// the local file vault. The fallback would be the dangerous kind:
    /// an operator who asked for object storage would keep writing
    /// credentials to a local directory while believing they were in
    /// S3. The message has to name the values that do work, otherwise
    /// the operator has to read the source to find out.
    #[test]
    fn an_unknown_vault_backend_is_an_error_that_names_the_valid_values() {
        for bad in ["minio", "bogus", "object-store", ""] {
            let err = bad
                .parse::<VaultBackend>()
                .expect_err("an unknown backend must not resolve to a backend");
            let msg = err.to_string();
            assert!(
                msg.contains("file") && msg.contains("s3"),
                "the error must name the valid values, got: {msg}"
            );
        }
    }

    /// `[FACT]` Absence of a selection means the local file vault. The
    /// default has to be the backend that works with no configuration
    /// at all — the S3 backend needs credentials, and inventing them is
    /// not an option.
    #[test]
    fn the_default_backend_is_the_local_file_vault() {
        assert_eq!(VaultBackend::default(), VaultBackend::File);
        let cfg = Config::new("127.0.0.1:8080", "repos", "vault");
        assert_eq!(cfg.vault_backend, VaultBackend::File);
        assert_eq!(cfg.vault_backend.as_str(), "file");
    }

    /// `[FACT]` The S3 settings have documented defaults; the key
    /// prefix and the credentials do not. A default access key would
    /// be a credential nobody chose, which is exactly what this
    /// repository has been removing from the binary (see
    /// `no_compiled_source_hard_codes_the_admin_password`). A default
    /// key prefix would be worse: the prefix is the isolation between
    /// two instances that share a bucket, so a shared default would put
    /// them on the same object keys.
    #[test]
    fn the_s3_settings_have_defaults_but_the_prefix_and_the_credentials_do_not() {
        let cfg = Config::new("127.0.0.1:8080", "repos", "vault");
        assert_eq!(cfg.s3_vault.endpoint, DEFAULT_S3_ENDPOINT);
        assert_eq!(cfg.s3_vault.bucket, DEFAULT_S3_BUCKET);
        assert_eq!(cfg.s3_vault.region, DEFAULT_S3_REGION);
        assert_eq!(cfg.s3_vault.key_prefix, S3_KEY_PREFIX_UNSET);
        assert!(
            cfg.s3_vault.key_prefix.is_empty(),
            "the key prefix must not have a default: it is the isolation between instances"
        );
        assert!(
            cfg.s3_vault.access_key.is_empty(),
            "an access key must not have a default"
        );
        assert!(
            cfg.s3_vault.secret_key.is_empty(),
            "a secret key must not have a default"
        );
    }

    /// `[FACT]` Selecting the S3 backend without credentials is an
    /// error that says what is missing and how to supply it. This is
    /// the guard against an operator who believes they are writing to
    /// object storage while the process talks to nothing at all.
    #[test]
    fn selecting_s3_without_credentials_is_an_error() {
        let mut cfg = Config::new("127.0.0.1:8080", "repos", "vault");
        cfg.vault_backend = VaultBackend::S3;
        cfg.s3_vault.key_prefix = "a-prefix/".to_string();

        let err = cfg
            .s3_vault_config()
            .expect_err("the s3 backend must not start without credentials");
        let msg = err.to_string();
        assert!(
            msg.contains(S3_ACCESS_KEY_ENV),
            "the error must name the environment variable, got: {msg}"
        );

        cfg.s3_vault.access_key = "an-access-key".to_string();
        let err = cfg
            .s3_vault_config()
            .expect_err("the s3 backend must not start without a secret key");
        let msg = err.to_string();
        assert!(
            msg.contains(S3_SECRET_KEY_ENV),
            "the error must name the environment variable, got: {msg}"
        );

        cfg.s3_vault.secret_key = "a-secret-key".to_string();
        let ok = cfg
            .s3_vault_config()
            .expect("both credentials present means the configuration is usable");
        assert_eq!(ok.bucket, DEFAULT_S3_BUCKET);
        assert_eq!(ok.key_prefix, "a-prefix/");
    }

    /// `[FACT]` The key prefix has no default, so selecting the S3
    /// backend without one is an error. The prefix is the isolation
    /// between instances that share a bucket: a shared default would
    /// put two of them on the same object keys and the same version
    /// sidecars, where either could read or overwrite the other's
    /// credentials.
    ///
    /// The error names both ways to supply it, and says why it is
    /// required -- an operator who hits it has to understand what the
    /// prefix is for, not just that something is missing.
    #[test]
    fn selecting_s3_without_a_key_prefix_is_an_error() {
        let mut cfg = Config::new("127.0.0.1:8080", "repos", "vault");
        cfg.vault_backend = VaultBackend::S3;
        cfg.s3_vault.access_key = "an-access-key".to_string();
        cfg.s3_vault.secret_key = "a-secret-key".to_string();

        let err = cfg
            .s3_vault_config()
            .expect_err("the s3 backend must not start without a key prefix");
        let msg = err.to_string();
        assert!(
            msg.contains(S3_KEY_PREFIX_ENV),
            "the error must name the environment variable, got: {msg}"
        );
        assert!(
            msg.contains("s3-key-prefix"),
            "the error must name the command-line flag, got: {msg}"
        );
        assert!(
            msg.contains("isolate"),
            "the error must say what the prefix is for, got: {msg}"
        );

        // A whitespace-only prefix is "not configured" as well.
        cfg.s3_vault.key_prefix = "   ".to_string();
        assert!(
            cfg.s3_vault_config().is_err(),
            "a blank prefix must not count as configured"
        );

        cfg.s3_vault.key_prefix = "a-prefix/".to_string();
        cfg.s3_vault_config()
            .expect("an explicit prefix makes the configuration usable");
    }

    /// `[FACT]` The debug output of a configuration must not carry the
    /// secret key. `MinioVaultConfig` derives `Debug` and prints it in
    /// the clear, so the configuration redacts it instead of forwarding
    /// it.
    #[test]
    fn the_debug_output_does_not_carry_the_s3_secret_key() {
        let mut cfg = Config::new("127.0.0.1:8080", "repos", "vault");
        cfg.s3_vault.access_key = "an-access-key".to_string();
        cfg.s3_vault.secret_key = "super-secret-value".to_string();
        let printed = format!("{cfg:?}");
        assert!(
            !printed.contains("super-secret-value"),
            "the secret key leaked into the debug output: {printed}"
        );
        assert!(
            !printed.contains("an-access-key"),
            "the access key leaked into the debug output: {printed}"
        );
        assert!(printed.contains(VaultBackend::File.as_str()));
    }
}
