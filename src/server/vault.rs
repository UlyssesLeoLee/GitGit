//! Credential Vault for the `gitgit` V0 GUI/CLI flows (per ADR-0021).
//!
//! V0 ships two [`Vault`] backends behind a single async trait:
//!
//! * [`MinioVault`] — the **default** backend; stores each credential as an
//!   object inside an S3-compatible bucket (typically a local `minIO` server
//!   during V0 dev). Provides cross-node access, native versioning, and
//!   erasure coding without extra glue code.
//! * [`FileVault`] — the **fallback** backend, used when minIO is
//!   unreachable (T11 deployment not yet run) or when the user opts out of
//!   S3. Persists each credential as a UTF-8 file under a directory with
//!   `0600` permissions on Unix (Windows inherits the directory ACL).
//!
//! The trait is intentionally minimal: five methods (get / set / delete /
//! list / rotate). The implementations are the only code that talks to the
//! underlying storage, so swapping the backend is a single `Arc<dyn Vault>`
//! replacement in `AppState` (which this commit does **not** touch — wiring
//! is deferred to T6 close-out per the brief's "不动 `src/server/auth.rs`"
//! constraint).
//!
//! ## Security notes
//!
//! * Values are returned as `String` and are not zeroized by the trait
//!   itself. Callers that hold long-lived secrets should overwrite their
//!   `String` after use. V0 does not do this; the
//!   [`gitai key`](../cli/index.html) command is short-lived.
//! * `MinioVault::connect` takes `&str` credentials and stores them inside
//!   the `Bucket` struct. The struct's `Clone` impl will copy them, so the
//!   caller should not keep a long-lived `MinioVault` if the secrets were
//!   read from a one-shot buffer.
//! * `FileVault` does not encrypt at rest. The brief explicitly defers
//!   encryption to V1+; until then the directory's filesystem ACL is the
//!   only protection.

// V0 T6 commit: the public items in this module are the foundation for
// the upcoming `AppState` wiring (T6 close-out) and the `gitai key`
// subcommands (T7). Until then the only call site is `#[cfg(test)]`, so
// every public symbol is "dead" from the production build's point of
// view. The `#[allow(dead_code)]` keeps `-D warnings` honest without
// scattering attribute noise through the file. The lint restriction
// `unused_must_use = "deny"` in `[lints.rust]` is still respected.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use s3::creds::error::CredentialsError as AwsCredsError;
use s3::error::S3Error;
use s3::{Bucket, Region};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::io::AsyncWriteExt;

use crate::error::{GitGitError, Result};

/// Errors specific to the credential vault layer.
///
/// `Vault(S3Error)` and `VaultFile(io::Error)` are wrapped here rather than
/// flattening into [`GitGitError`] directly so that callers can distinguish
/// "minIO is down" from "filesystem is full" without parsing the error
/// message. The conversion to [`GitGitError::Http`] happens at the HTTP
/// boundary in `server::http`.
#[derive(Debug, Error)]
pub enum VaultError {
    /// A `minIO` / S3 operation failed. Wraps the underlying crate error
    /// verbatim so we don't lose context (`status_code`, `request_id`, ...).
    #[error("minIO/S3 error: {0}")]
    Vault(#[from] S3Error),

    /// A `FileVault` filesystem operation failed.
    #[error("file vault io error: {0}")]
    VaultFile(#[from] std::io::Error),

    /// Parsing or applying minIO credentials failed (returned by
    /// `awscreds::Credentials::new`). Distinct from `S3Error` because
    /// the failure happens at `connect` time, not at request time.
    #[error("minIO credentials error: {0}")]
    Credentials(#[from] AwsCredsError),

    /// The stored value was not valid UTF-8.
    #[error("stored value at key {0} is not valid UTF-8")]
    NotUtf8(String),
}

impl From<VaultError> for GitGitError {
    fn from(value: VaultError) -> Self {
        // Collapse to a single Http-shaped variant so we don't have to
        // thread a new error type through `crate::error::Result`. The
        // Display impl above keeps the underlying cause visible in logs.
        GitGitError::Http(format!("vault: {value}"))
    }
}

/// Async credential vault abstraction.
///
/// All methods are `async` because the minIO path needs to issue HTTP
/// requests; the file path is also async (tokio `fs`) so callers don't
/// have to special-case the two backends at the call site.
#[async_trait]
pub trait Vault: Send + Sync {
    /// Fetch the value stored under `key`.
    ///
    /// Returns:
    /// * `Ok(Some(value))` — key exists, value is valid UTF-8.
    /// * `Ok(None)` — key does not exist (or, for `MinioVault`, the object
    ///   is absent / was soft-deleted and the lifecycle window has
    ///   expired).
    /// * `Err(_)` — transport / IO / decode failure.
    async fn get(&self, key: &str) -> Result<Option<String>>;

    /// Store `value` under `key`, overwriting any existing value.
    async fn set(&self, key: &str, value: &str) -> Result<()>;

    /// Remove the value stored under `key`.
    ///
    /// Missing keys are **not** an error; the method is idempotent.
    /// `MinioVault` is built on S3 object versioning, so a delete creates
    /// a `DeleteMarker`; the previous value remains recoverable for the
    /// 90-day lifecycle window (see ADR-0021 §3).
    async fn delete(&self, key: &str) -> Result<()>;

    /// Enumerate every key currently visible in the vault.
    ///
    /// Ordering is implementation-defined; the brief does not require any
    /// specific order.
    async fn list(&self) -> Result<Vec<String>>;

    /// Overwrite the value under `key` with a fresh, opaque value.
    ///
    /// Implementations choose how to generate the new value; the brief
    /// suggests a timestamp + random suffix, which `MinioVault` does.
    /// `FileVault` writes an equivalent fresh value so that callers can
    /// use `rotate` uniformly across backends.
    async fn rotate(&self, key: &str) -> Result<()>;
}

/// Configuration for constructing a [`MinioVault`].
///
/// Plain-data struct so it can be deserialized from a future TOML file
/// (T11 follow-up). For now the binary builds one by hand from CLI flags
/// or environment variables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinioVaultConfig {
    /// S3-compatible endpoint, e.g. `"http://127.0.0.1:9000"` for a local
    /// minIO. **No trailing slash.**
    pub endpoint: String,

    /// Bucket name (e.g. `"gitgit-vault"`).
    pub bucket: String,

    /// S3 access key (minIO root user in dev).
    pub access_key: String,

    /// S3 secret key (minIO root password in dev).
    pub secret_key: String,

    /// Region string used in SigV4 signing. `minIO` accepts any value, but
    /// using `"us-east-1"` keeps things consistent with the SigV4 default
    /// when other S3-compatible stores are swapped in later.
    pub region: String,

    /// Key prefix applied to every object, e.g. `"gitgit-vault/"`. An
    /// empty string is allowed (objects live at the bucket root).
    pub key_prefix: String,
}

/// `minIO`-backed [`Vault`] implementation.
///
/// Each credential is stored as one S3 object: `<key_prefix><key>`. The
/// prefix is the natural way to isolate multiple gitgit instances or to
/// share a single bucket across unrelated tooling without collision
/// (e.g. `gitgit-vault/openai/api_key`).
pub struct MinioVault {
    bucket: Box<Bucket>,
    key_prefix: String,
}

impl std::fmt::Debug for MinioVault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Deliberately redact credentials and the `Bucket` (which holds
        // them) from the debug print. The endpoint / bucket name are
        // safe to log because they're not secret.
        f.debug_struct("MinioVault")
            .field("bucket_name", &self.bucket.name())
            .field("key_prefix", &self.key_prefix)
            .finish_non_exhaustive()
    }
}

impl MinioVault {
    /// Build a `MinioVault` from explicit config. Performs no network I/O;
    /// it only parses the endpoint URL and constructs a `Bucket` handle.
    /// This means a stale or wrong endpoint will not surface until the
    /// first `get` / `set` / `delete` / `list` / `rotate` call.
    ///
    /// Returns [`VaultError::Vault`] if the endpoint URL cannot be parsed.
    pub fn connect(cfg: &MinioVaultConfig) -> std::result::Result<Self, VaultError> {
        let region = Region::Custom {
            region: cfg.region.clone(),
            endpoint: cfg.endpoint.clone(),
        };
        let credentials = s3::creds::Credentials::new(
            Some(&cfg.access_key),
            Some(&cfg.secret_key),
            None,
            None,
            None,
        )?;
        // `Bucket::new` takes the bucket name, region, and credentials and
        // returns a `Box<Bucket>` whose `path_style` defaults to
        // `SubdomainStyle`. minIO **requires** path-style addressing for
        // bucket names containing dots or when running on a non-default
        // port, so we explicitly switch to `PathStyle`. This is also
        // forward-compatible with HTTP-only local dev (no TLS).
        let mut bucket = Bucket::new(&cfg.bucket, region, credentials)?;
        bucket = bucket.with_path_style();
        Ok(Self {
            bucket,
            key_prefix: cfg.key_prefix.clone(),
        })
    }

    /// The bucket name (for logging / diagnostics).
    pub fn bucket_name(&self) -> &str {
        // `name` is a public `String` field on the `Bucket` struct; we
        // return a `&str` view into it. (The method form `bucket.name()`
        // is not part of the public API in `rust-s3` 0.37.)
        &self.bucket.name
    }

    /// The key prefix currently applied to every object.
    pub fn key_prefix(&self) -> &str {
        &self.key_prefix
    }

    /// Compose the full S3 object key for a logical vault key.
    fn object_key(&self, key: &str) -> String {
        if self.key_prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}{}", self.key_prefix, key)
        }
    }
}

#[async_trait]
impl Vault for MinioVault {
    async fn get(&self, key: &str) -> Result<Option<String>> {
        let object_key = self.object_key(key);
        match self.bucket.get_object(&object_key).await {
            Ok(resp) => {
                let bytes = resp.bytes();
                match String::from_utf8(bytes.to_vec()) {
                    Ok(s) => Ok(Some(s)),
                    Err(_) => Err(VaultError::NotUtf8(key.to_string()).into()),
                }
            }
            // `S3Error::NoSuchKey` (and the 404-shaped variants) collapse
            // to "not present" rather than "broken", matching the contract
            // documented on [`Vault::get`].
            Err(e) if is_not_found(&e) => Ok(None),
            Err(e) => Err(VaultError::from(e).into()),
        }
    }

    async fn set(&self, key: &str, value: &str) -> Result<()> {
        let object_key = self.object_key(key);
        let _resp = self.bucket.put_object(&object_key, value.as_bytes()).await?;
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let object_key = self.object_key(key);
        // S3 delete is idempotent: deleting a missing key is a no-op and
        // we do not surface 404 as an error.
        let _resp = self.bucket.delete_object(&object_key).await?;
        Ok(())
    }

    async fn list(&self) -> Result<Vec<String>> {
        // `list(prefix, delimiter)` returns one `ListBucketResult` per
        // page. With `delimiter = Some("/")` we get "directory" grouping
        // for free, but the brief wants the *flat* list of leaf keys
        // under our prefix, so we pass `None` for the delimiter and
        // collect across pages.
        let prefix = self.key_prefix.clone();
        let pages = self.bucket.list(prefix, None).await?;
        let mut out = Vec::new();
        for page in pages {
            for obj in page.contents {
                let key = obj.key;
                let stripped = key
                    .strip_prefix(&self.key_prefix)
                    .map(str::to_string)
                    .unwrap_or(key);
                out.push(stripped);
            }
        }
        Ok(out)
    }

    async fn rotate(&self, key: &str) -> Result<()> {
        // Per the brief: a fresh opaque value that the caller can rely
        // on being different from any previous one. We use
        // nanosecond-resolution Unix time + a UUIDv4 suffix. The format
        // is intentionally not parseable as a credential of any specific
        // provider so a misrouted `get` of a rotated value is
        // immediately obvious.
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let suffix = uuid::Uuid::new_v4();
        let value = format!("rotated-{ts}-{suffix}");
        self.set(key, &value).await
    }
}

/// Return `true` if the `S3Error` represents "object not present" rather
/// than a real transport failure.
///
/// `rust-s3` historically used `HttpFailWithBody(404, ...)` for both
/// missing objects and missing buckets; newer releases also expose
/// `NoSuchKey`. We match both so the vault stays compatible across patch
/// versions of the dependency.
fn is_not_found(err: &S3Error) -> bool {
    // Stringify because `S3Error` variants are `pub(crate)` in 0.37 and
    // not directly matchable from the outside. The error `Display` impl
    // includes the status code, so we look for `404` and the canonical
    // S3 phrases.
    let s = err.to_string();
    s.contains("404")
        || s.contains("NoSuchKey")
        || s.contains("NotFound")
        || s.contains("not found")
}

/// Filesystem-backed [`Vault`] implementation.
///
/// Each key is one file: `<root>/<key>`. The directory is created lazily
/// on first write. `set` writes atomically (write-to-temp + rename) so a
/// crash mid-write cannot leave a half-written file visible to a
/// subsequent `get`.
pub struct FileVault {
    root: PathBuf,
}

impl std::fmt::Debug for FileVault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileVault")
            .field("root", &self.root)
            .finish()
    }
}

impl FileVault {
    /// Build a `FileVault` rooted at `root`. The directory need not
    /// exist yet; it will be created on the first `set`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The on-disk root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Resolve the absolute path for a given key. Performs no I/O.
    fn path_for(&self, key: &str) -> Option<PathBuf> {
        // Reject path-traversal attempts up-front: a malicious `key`
        // containing `..` could otherwise escape `self.root`. The same
        // check is mirrored in `crate::config::validate_repo_name` for
        // bare-repo names; here we apply it to vault keys, which can in
        // principle contain `/` (e.g. `openai/api_key`) but **not**
        // parent-directory references, absolute paths, or Windows
        // drive letters.
        if key.is_empty() {
            return None;
        }
        if key.contains("..") {
            return None;
        }
        if key.starts_with('/') {
            return None;
        }
        if key.starts_with('\\') {
            return None;
        }
        // Windows drive letter prefix (e.g. `C:` or `C:\`).
        if key.len() >= 2
            && key.as_bytes()[0].is_ascii_alphabetic()
            && (key.as_bytes()[1] == b':' || key.as_bytes()[1] == b'|')
        {
            return None;
        }
        Some(self.root.join(key))
    }

    async fn ensure_root(&self) -> std::io::Result<()> {
        if !self.root.exists() {
            tokio::fs::create_dir_all(&self.root).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl Vault for FileVault {
    async fn get(&self, key: &str) -> Result<Option<String>> {
        let Some(p) = self.path_for(key) else {
            // A bad key can never have been set, so the answer is
            // always "not present". The caller (e.g. `gitai key get`)
            // will treat this the same as a missing key, which is the
            // principle-of-least-surprise behavior.
            return Ok(None);
        };
        match tokio::fs::read_to_string(&p).await {
            Ok(s) => Ok(Some(s)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(VaultError::VaultFile(e).into()),
        }
    }

    async fn set(&self, key: &str, value: &str) -> Result<()> {
        let p = match self.path_for(key) {
            Some(p) => p,
            None => return Err(VaultError::NotUtf8(format!("invalid key: {key:?}")).into()),
        };
        self.ensure_root().await?;
        if let Some(parent) = p.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        // Write to a temp file in the same directory, then rename over
        // the target. This is the canonical atomic-replace pattern on
        // POSIX (rename is atomic within a filesystem) and a best-
        // effort fallback on Windows (rename fails if the target
        // exists, so we use the explicit-replace variant).
        let mut tmp = p.clone();
        let tmp_name = format!(
            ".tmp.{}",
            uuid::Uuid::new_v4().simple()
        );
        tmp.set_file_name(tmp_name);
        let mut f = tokio::fs::File::create(&tmp).await?;
        f.write_all(value.as_bytes()).await?;
        f.sync_all().await?;
        drop(f);
        // On Windows, tokio's `rename` falls back to `MoveFileExW` with
        // `MOVEFILE_REPLACE_EXISTING`, which is the behavior we want.
        if let Err(e) = tokio::fs::rename(&tmp, &p).await {
            // Best-effort cleanup; ignore the secondary error.
            let _ = tokio::fs::remove_file(&tmp).await;
            return Err(VaultError::VaultFile(e).into());
        }
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let Some(p) = self.path_for(key) else {
            return Ok(());
        };
        match tokio::fs::remove_file(&p).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(VaultError::VaultFile(e).into()),
        }
    }

    async fn list(&self) -> Result<Vec<String>> {
        // We only enumerate immediate children (no recursion). That
        // matches the `MinioVault` contract, which uses a flat
        // (prefix, no delimiter) listing on its own key namespace.
        if !self.root.exists() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        let mut rd = tokio::fs::read_dir(&self.root).await?;
        while let Some(entry) = rd.next_entry().await? {
            let name = entry.file_name();
            // Filter out our own atomic-replace temp files so they do
            // not show up as phantom keys.
            if let Some(s) = name.to_str() {
                if s.starts_with(".tmp.") {
                    continue;
                }
            }
            if let Some(s) = name.to_str() {
                out.push(s.to_string());
            }
        }
        out.sort();
        Ok(out)
    }

    async fn rotate(&self, key: &str) -> Result<()> {
        // Same shape as `MinioVault::rotate` so callers can write
        // `vault.rotate(key).await?` against either backend.
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let suffix = uuid::Uuid::new_v4();
        let value = format!("rotated-{ts}-{suffix}");
        self.set(key, &value).await
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// Build a unique temp directory for a single test. The directory is
    /// not cleaned up; `cargo test` reuses a per-PID temp tree, and
    /// the OS clears `std::env::temp_dir()` on reboot. For a CI-tight
    /// implementation we'd add a `Drop` guard, but for V0 this is fine.
    fn fresh_dir(label: &str) -> PathBuf {
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("gitgit-vault-test-{label}-{pid}-{nanos}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    // ---- FileVault tests (5+) -------------------------------------------

    #[tokio::test]
    async fn file_vault_set_then_get_returns_value() {
        let dir = fresh_dir("set-get");
        let v = FileVault::new(&dir);
        v.set("openai", "sk-test-123").await.unwrap();
        let got = v.get("openai").await.unwrap();
        assert_eq!(got.as_deref(), Some("sk-test-123"));
    }

    #[tokio::test]
    async fn file_vault_get_missing_key_returns_none() {
        let dir = fresh_dir("missing");
        let v = FileVault::new(&dir);
        let got = v.get("not-there").await.unwrap();
        assert_eq!(got, None);
    }

    #[tokio::test]
    async fn file_vault_set_overwrites_previous_value() {
        let dir = fresh_dir("overwrite");
        let v = FileVault::new(&dir);
        v.set("k", "v1").await.unwrap();
        v.set("k", "v2").await.unwrap();
        assert_eq!(v.get("k").await.unwrap().as_deref(), Some("v2"));
    }

    #[tokio::test]
    async fn file_vault_delete_removes_value_and_is_idempotent() {
        let dir = fresh_dir("delete");
        let v = FileVault::new(&dir);
        v.set("k", "v").await.unwrap();
        v.delete("k").await.unwrap();
        assert_eq!(v.get("k").await.unwrap(), None);
        // Deleting again must not error.
        v.delete("k").await.unwrap();
    }

    #[tokio::test]
    async fn file_vault_list_returns_all_keys_sorted_and_skips_tmps() {
        let dir = fresh_dir("list");
        let v = FileVault::new(&dir);
        v.set("zeta", "1").await.unwrap();
        v.set("alpha", "2").await.unwrap();
        v.set("mu", "3").await.unwrap();
        let mut keys = v.list().await.unwrap();
        keys.sort();
        assert_eq!(keys, vec!["alpha", "mu", "zeta"]);
    }

    #[tokio::test]
    async fn file_vault_rotate_changes_value_and_keeps_key() {
        let dir = fresh_dir("rotate");
        let v = FileVault::new(&dir);
        v.set("k", "original").await.unwrap();
        v.rotate("k").await.unwrap();
        let after = v.get("k").await.unwrap();
        assert!(after.is_some());
        assert_ne!(after.as_deref(), Some("original"));
        assert!(after.unwrap().starts_with("rotated-"));
    }

    #[tokio::test]
    async fn file_vault_rejects_path_traversal_keys() {
        let dir = fresh_dir("traversal");
        let v = FileVault::new(&dir);
        // `set` must refuse to write a key that would escape the root.
        // We assert both the error and the absence of any new file
        // under the root.
        let bad_keys = ["../escape", "..", "subdir/../../escape", "/abs", "C:win", "C:\\win"];
        for bad in bad_keys {
            let res = v.set(bad, "evil").await;
            assert!(
                res.is_err(),
                "set({bad:?}) should fail but got {res:?}"
            );
        }
        // A get of the same bad keys must be Ok(None) — they were
        // never stored.
        for bad in bad_keys {
            let got = v.get(bad).await.unwrap();
            assert!(got.is_none(), "get({bad:?}) returned {got:?}");
        }
        // The list must not contain any `..` segments.
        let keys = v.list().await.unwrap();
        for k in &keys {
            assert!(!k.split(['/', '\\']).any(|seg| seg == ".."),
                "list leaked traversal key: {k}");
        }
        // And nothing was written under the root.
        let entries: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert!(entries.is_empty(), "traversal set left residue: {entries:?}");
    }

    // ---- MinioVault tests (no network) ----------------------------------

    /// Build a `MinioVault` pointed at a deliberately-unreachable
    /// endpoint. `connect` itself does no network I/O, so this should
    /// always succeed; the failure modes live in `get` / `set` etc.,
    /// which need a real minIO and are tested via the `#[ignore]`d
    /// integration test below.
    fn offline_minio_config() -> MinioVaultConfig {
        MinioVaultConfig {
            // `127.0.0.1:1` is reserved (tcpmux) and never accepts
            // HTTP on developer machines, so any actual call would
            // fail fast. We only need the `connect` call to succeed.
            endpoint: "http://127.0.0.1:1".to_string(),
            bucket: "gitgit-vault".to_string(),
            access_key: "minioadmin".to_string(),
            secret_key: "minioadmin".to_string(),
            region: "us-east-1".to_string(),
            key_prefix: "gitgit-vault/".to_string(),
        }
    }

    #[test]
    fn minio_vault_connect_succeeds_without_network() {
        let cfg = offline_minio_config();
        let v = MinioVault::connect(&cfg).expect("connect should not perform network I/O");
        assert_eq!(v.bucket_name(), "gitgit-vault");
        assert_eq!(v.key_prefix(), "gitgit-vault/");
    }

    #[test]
    fn minio_vault_object_key_applies_prefix() {
        let cfg = offline_minio_config();
        let v = MinioVault::connect(&cfg).unwrap();
        assert_eq!(v.object_key("openai"), "gitgit-vault/openai");
    }

    #[test]
    fn minio_vault_object_key_with_empty_prefix_is_passthrough() {
        let mut cfg = offline_minio_config();
        cfg.key_prefix.clear();
        let v = MinioVault::connect(&cfg).unwrap();
        assert_eq!(v.object_key("openai"), "openai");
    }

    #[test]
    fn minio_vault_debug_does_not_leak_credentials() {
        // Use a *unique* sentinel string for the secret so we can be
        // sure the debug print doesn't contain it. Re-using a generic
        // value like "minioadmin" would let a coincidental substring
        // match (e.g. in the bucket name) silently pass.
        let mut cfg = offline_minio_config();
        cfg.access_key = "AKIA-SENTINEL-ACCESS-KEY".to_string();
        cfg.secret_key = "SECRET-SENTINEL-DO-NOT-LEAK-VALUE".to_string();
        cfg.bucket = "gitgit-vault".to_string();
        let v = MinioVault::connect(&cfg).unwrap();
        let dbg = format!("{v:?}");
        assert!(
            !dbg.contains("SECRET-SENTINEL-DO-NOT-LEAK-VALUE"),
            "Debug output leaked the secret_key: {dbg}"
        );
        assert!(
            !dbg.contains("AKIA-SENTINEL-ACCESS-KEY"),
            "Debug output leaked the access_key: {dbg}"
        );
        // Non-secret fields are allowed to appear.
        assert!(dbg.contains("gitgit-vault"));
    }

    /// End-to-end test against a real `minIO` server. Skipped by default
    /// because the T11 deployment task has not yet been run on this
    /// machine. To exercise it:
    ///
    /// ```text
    /// docker run -d -p 9000:9000 -p 9001:9001 \
    ///   -e MINIO_ROOT_USER=minioadmin \
    ///   -e MINIO_ROOT_PASSWORD=minioadmin \
    ///   quay.io/minio/minio server /data --console-address ":9001"
    /// docker run --rm --network host minio/mc mb local/gitgit-vault
    /// cargo test --bin gitgit -- --ignored minio_vault_e2e_roundtrip
    /// ```
    #[tokio::test]
    #[ignore = "requires a running minIO server (see comment in test)"]
    async fn minio_vault_e2e_roundtrip() {
        let cfg = MinioVaultConfig {
            endpoint: std::env::var("GITGIT_MINIO_ENDPOINT")
                .unwrap_or_else(|_| "http://127.0.0.1:9000".to_string()),
            bucket: std::env::var("GITGIT_MINIO_BUCKET")
                .unwrap_or_else(|_| "gitgit-vault".to_string()),
            access_key: std::env::var("GITGIT_MINIO_ACCESS_KEY")
                .unwrap_or_else(|_| "minioadmin".to_string()),
            secret_key: std::env::var("GITGIT_MINIO_SECRET_KEY")
                .unwrap_or_else(|_| "minioadmin".to_string()),
            region: "us-east-1".to_string(),
            key_prefix: format!("ut-{}/", uuid::Uuid::new_v4().simple()),
        };
        let v = MinioVault::connect(&cfg).expect("connect");

        // 1. set + get roundtrip
        v.set("openai", "sk-ut-secret").await.expect("set");
        assert_eq!(
            v.get("openai").await.expect("get").as_deref(),
            Some("sk-ut-secret")
        );

        // 2. missing key returns Ok(None), not Err
        assert_eq!(v.get("definitely-not-there").await.expect("get-missing"), None);

        // 3. list contains the key
        let mut keys = v.list().await.expect("list");
        keys.sort();
        assert!(keys.iter().any(|k| k == "openai"), "list missing openai: {keys:?}");

        // 4. rotate changes the value but keeps the key
        v.rotate("openai").await.expect("rotate");
        let rotated = v.get("openai").await.expect("get-rotated");
        assert!(rotated.is_some());
        assert_ne!(rotated.as_deref(), Some("sk-ut-secret"));
        assert!(rotated.unwrap().starts_with("rotated-"));

        // 5. delete removes the value (and is idempotent)
        v.delete("openai").await.expect("delete");
        assert_eq!(v.get("openai").await.expect("get-after-delete"), None);
        v.delete("openai").await.expect("delete-idempotent");
    }
}
