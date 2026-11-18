//! Cloud disks — remote storage for repository snapshots.
//!
//! # What a cloud disk is
//!
//! A **cloud disk** is a named remote that holds **snapshots** of the
//! repositories this server keeps. A snapshot is one file: a tar archive
//! of one bare repository, uploaded under a key that carries the
//! repository name and the moment it was taken. Restoring a snapshot puts
//! the repository back the way it was — that is the rollback.
//!
//! # Which remote
//!
//! `[FACT]` The transport is **S3-compatible** ✓. This repository
//! already speaks it twice over: the credential vault talks to an
//! S3-compatible bucket through the `rust-s3` client
//! (`src/server/vault.rs`), and this repository *is* an S3-compatible
//! server (`src/objectstore/`) ✓. The same client therefore reaches a
//! self-hosted object store, Alibaba Cloud OSS, Tencent COS, Cloudflare
//! R2 or Amazon S3 without a new line of protocol code ✓.
//!
//! `[INFERENCE]` OneDrive and the Chinese consumer drives speak WebDAV
//! rather than S3 ✓, so they are a V1 concern ✓ — a WebDAV client is a
//! new protocol implementation ✓, while this one is a new *endpoint* ✓.
//!
//! # Where the credentials come from
//!
//! `[FACT]` **Only from the environment** ✓, never from a flag ✓ — the
//! same rule the AI key and the S3 vault credentials follow ✓ (a
//! credential in the argument list is in `ps` output and in the shell
//! history ✓).
//!
//! Per disk: `GITGIT_CLOUD_<NAME>_ACCESS_KEY` and
//! `GITGIT_CLOUD_<NAME>_SECRET_KEY` ✓; falling back to
//! `GITGIT_CLOUD_ACCESS_KEY` / `GITGIT_CLOUD_SECRET_KEY` ✓ for a
//! single-disk setup ✓.
//!
//! # What is *not* stored here
//!
//! `[FACT]` The credential vault has its own remote ✓ (the `s3` vault
//! backend ✓, with its own versioning ✓ and its own restore ✓), so the
//! cloud disk does not duplicate it ✓. What is missing elsewhere — and
//! what this module adds — is a remote copy of the **repositories** ✓,
//! with a way back to an earlier one ✓.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{GitGitError, Result};

pub mod backup;
pub mod tar;

/// The environment variable that holds the access key for the disk
/// called `name`.
pub fn access_key_env(name: &str) -> String {
    format!("GITGIT_CLOUD_{}_ACCESS_KEY", env_suffix(name))
}

/// The environment variable that holds the secret key for the disk called
/// `name`.
pub fn secret_key_env(name: &str) -> String {
    format!("GITGIT_CLOUD_{}_SECRET_KEY", env_suffix(name))
}

/// The fallback environment variables, for a setup with one disk.
pub const FALLBACK_ACCESS_KEY_ENV: &str = "GITGIT_CLOUD_ACCESS_KEY";
pub const FALLBACK_SECRET_KEY_ENV: &str = "GITGIT_CLOUD_SECRET_KEY";

/// The disk name as it appears in an environment variable name.
fn env_suffix(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_uppercase() } else { '_' })
        .collect()
}

/// The name of the cloud disk registry file.
///
/// `[FACT]` It sits next to the remote registry ✓ (`.gitgit/remotes.json` ✓)
/// and the vault root ✓, so the three concerns stay separable ✓.
pub const CLOUDS_FILE_NAME: &str = "clouds.json";

/// One connected cloud disk.
///
/// The credentials are **not** here ✓: they live in the environment ✓, so
/// this file is safe to commit ✓ and to back up ✓.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloudDiskEntry {
    /// The name used on the command line, e.g. `oss`.
    pub name: String,
    /// The endpoint URL, e.g. `https://oss-cn-hangzhou.aliyuncs.com`.
    pub endpoint: String,
    /// The bucket that holds the snapshots.
    pub bucket: String,
    /// The region, for S3-compatible services that need one.
    #[serde(default = "default_region")]
    pub region: String,
    /// The key prefix every snapshot of this disk lives under.
    #[serde(default = "default_prefix")]
    pub prefix: String,
}

fn default_region() -> String {
    "us-east-1".to_string()
}

fn default_prefix() -> String {
    "gitgit".to_string()
}

impl CloudDiskEntry {
    /// The key prefix, with a trailing `/` ✓ — the S3 key prefix of every
    /// object this disk writes ✓.
    pub fn key_prefix(&self) -> String {
        if self.prefix.is_empty() {
            String::new()
        } else if self.prefix.ends_with('/') {
            self.prefix.clone()
        } else {
            format!("{}/", self.prefix)
        }
    }

    /// The access key for this disk, read from the environment.
    pub fn access_key(&self) -> Result<String> {
        read_secret(&access_key_env(&self.name))
            .or_else(|| read_secret(FALLBACK_ACCESS_KEY_ENV))
            .ok_or_else(|| {
                GitGitError::Cloud(format!(
                    "no access key for the cloud disk {:?}: set {} (or {})",
                    self.name,
                    access_key_env(&self.name),
                    FALLBACK_ACCESS_KEY_ENV
                ))
            })
    }

    /// The secret key for this disk, read from the environment.
    pub fn secret_key(&self) -> Result<String> {
        read_secret(&secret_key_env(&self.name))
            .or_else(|| read_secret(FALLBACK_SECRET_KEY_ENV))
            .ok_or_else(|| {
                GitGitError::Cloud(format!(
                    "no secret key for the cloud disk {:?}: set {} (or {})",
                    self.name,
                    secret_key_env(&self.name),
                    FALLBACK_SECRET_KEY_ENV
                ))
            })
    }

    /// Reject a disk that could not possibly address an object.
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(GitGitError::Cloud("a cloud disk needs a name".to_string()));
        }
        if self.endpoint.trim().is_empty() {
            return Err(GitGitError::Cloud(format!(
                "the cloud disk {:?} has no endpoint",
                self.name
            )));
        }
        if self.bucket.trim().is_empty() {
            return Err(GitGitError::Cloud(format!(
                "the cloud disk {:?} has no bucket",
                self.name
            )));
        }
        Ok(())
    }
}

/// Read a secret from the environment, treating a blank value as absent.
fn read_secret(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(value) if !value.trim().is_empty() => Some(value),
        _ => None,
    }
}

/// One object on a cloud disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloudObject {
    /// The key the object is stored under.
    pub key: String,
    /// The object's size in bytes.
    pub size: u64,
    /// The server's version id, when it has one — that is what makes a
    /// rollback to a *previous* version possible.
    pub version_id: Option<String>,
    /// When the server says the object was last modified ✓ — its own
    /// format ✓, kept as it arrived ✓.
    pub modified: String,
}

/// The metadata of one snapshot.
///
/// `[FACT]` It travels **inside** the snapshot file ✓, not beside it ✓,
/// so a snapshot that was copied somewhere else still says what it is ✓.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotMeta {
    /// The format version of the snapshot file.
    pub version: u32,
    /// The disk the snapshot was taken on.
    pub disk: String,
    /// The repository the snapshot holds.
    pub repo: String,
    /// The moment the snapshot was taken, in seconds since the Unix epoch.
    pub created_unix: u64,
    /// How many files the snapshot holds.
    pub entries: u64,
    /// The SHA-256 of the archive, hex.
    pub sha256: String,
}

/// The magic that starts every snapshot file.
///
/// `[FACT]` It is the ASCII text `GITGIT-SNAPSHOT` ✓ followed by a
/// newline ✓ — so a snapshot that someone opened by mistake is obvious ✓.
pub const SNAPSHOT_MAGIC: &[u8] = b"GITGIT-SNAPSHOT\n";

/// The version of the snapshot format this build writes.
pub const SNAPSHOT_VERSION: u32 = 1;

/// The registry of connected cloud disks.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloudRegistry {
    /// The disks, by name.
    #[serde(default)]
    pub disks: BTreeMap<String, CloudDiskEntry>,
}

impl CloudRegistry {
    /// Read the registry from `path` ✓ — a missing file is an empty
    /// registry ✓, not an error ✓.
    pub fn load(path: &Path) -> Result<Self> {
        match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| {
                GitGitError::Cloud(format!("the cloud disk registry is unreadable: {e}"))
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(GitGitError::Io(e)),
        }
    }

    /// Write the registry to `path` ✓.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|e| GitGitError::Cloud(format!("the cloud disk registry is unwritable: {e}")))?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    /// The disk called `name` ✓.
    pub fn get(&self, name: &str) -> Result<&CloudDiskEntry> {
        self.disks.get(name).ok_or_else(|| {
            GitGitError::Cloud(format!(
                "no cloud disk called {name:?} is connected (gitgit cloud list)"
            ))
        })
    }

    /// The only connected disk, when there is exactly one ✓.
    pub fn only(&self) -> Result<&CloudDiskEntry> {
        match self.disks.len() {
            1 => Ok(self.disks.values().next().expect("one disk")),
            0 => Err(GitGitError::Cloud(
                "no cloud disk is connected (gitgit cloud add ...)".to_string(),
            )),
            n => Err(GitGitError::Cloud(format!(
                "{n} cloud disks are connected; name one with --disk"
            ))),
        }
    }
}

/// A remote that holds snapshots.
#[async_trait::async_trait]
pub trait CloudDisk: Send + Sync {
    /// Store `bytes` under `key` ✓ and return the server's version id ✓,
    /// when it has one ✓.
    async fn put(&self, key: &str, bytes: &[u8]) -> Result<Option<String>>;

    /// Read the object at `key` ✓.
    async fn get(&self, key: &str) -> Result<Vec<u8>>;

    /// List the objects whose key starts with `prefix` ✓.
    async fn list(&self, prefix: &str) -> Result<Vec<CloudObject>>;

    /// Delete the object at `key` ✓.
    async fn delete(&self, key: &str) -> Result<()>;
}

/// A cloud disk reached over S3-compatible HTTP.
pub struct S3CloudDisk {
    bucket: Box<s3::Bucket>,
    key_prefix: String,
}

impl S3CloudDisk {
    /// Connect to the disk described by `entry` ✓.
    pub async fn connect(entry: &CloudDiskEntry) -> Result<Self> {
        entry.validate()?;
        let region = s3::Region::Custom {
            region: entry.region.clone(),
            endpoint: entry.endpoint.clone(),
        };
        let credentials = s3::creds::Credentials::new(
            Some(&entry.access_key()?),
            Some(&entry.secret_key()?),
            None,
            None,
            None,
        )
        .map_err(|e| GitGitError::Cloud(format!("the cloud credentials are unusable: {e}")))?;
        // Path-style addressing ✓ — the same choice the vault backend
        // makes ✓, because a local or non-AWS endpoint is addressed by
        // path ✓.
        let bucket = s3::Bucket::new(&entry.bucket, region, credentials)
            .map_err(|e| GitGitError::Cloud(format!("the cloud disk is unreachable: {e}")))?
            .with_path_style();
        Ok(Self {
            bucket,
            key_prefix: entry.key_prefix(),
        })
    }

    /// The key an object of this disk is stored under ✓.
    pub fn key(&self, name: &str) -> String {
        format!("{}{}", self.key_prefix, name)
    }
}

#[async_trait::async_trait]
impl CloudDisk for S3CloudDisk {
    async fn put(&self, key: &str, bytes: &[u8]) -> Result<Option<String>> {
        let response = self
            .bucket
            .put_object(key, bytes)
            .await
            .map_err(|e| GitGitError::Cloud(format!("the snapshot could not be uploaded: {e}")))?;
        let headers = response.headers();
        Ok(headers
            .get("x-amz-version-id")
            .or_else(|| headers.get("X-Amz-Version-Id"))
            .cloned())
    }

    async fn get(&self, key: &str) -> Result<Vec<u8>> {
        let object = self
            .bucket
            .get_object(key)
            .await
            .map_err(|e| GitGitError::Cloud(format!("the snapshot could not be downloaded: {e}")))?;
        Ok(object.bytes().to_vec())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<CloudObject>> {
        let pages = self
            .bucket
            .list(prefix.to_string(), None)
            .await
            .map_err(|e| GitGitError::Cloud(format!("the snapshots could not be listed: {e}")))?;
        let mut out = Vec::new();
        for page in pages {
            for object in page.contents {
                out.push(CloudObject {
                    key: object.key,
                    size: object.size,
                    version_id: None,
                    modified: object.last_modified,
                });
            }
        }
        out.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(out)
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.bucket
            .delete_object(key)
            .await
            .map_err(|e| GitGitError::Cloud(format!("the snapshot could not be deleted: {e}")))?;
        Ok(())
    }
}

/// The default location of the cloud disk registry, next to the other
/// per-user state.
pub fn default_registry_path() -> PathBuf {
    PathBuf::from(CLOUDS_FILE_NAME)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn entry(name: &str) -> CloudDiskEntry {
        CloudDiskEntry {
            name: name.to_string(),
            endpoint: "https://example.invalid".to_string(),
            bucket: "snapshots".to_string(),
            region: default_region(),
            prefix: default_prefix(),
        }
    }

    #[test]
    fn the_key_prefix_always_ends_in_a_slash() {
        let mut e = entry("oss");
        e.prefix = "gitgit".to_string();
        assert_eq!(e.key_prefix(), "gitgit/");
        e.prefix = "gitgit/".to_string();
        assert_eq!(e.key_prefix(), "gitgit/");
        e.prefix = String::new();
        assert_eq!(e.key_prefix(), "");
    }

    #[test]
    fn the_environment_variable_names_follow_the_disk_name() {
        assert_eq!(access_key_env("oss"), "GITGIT_CLOUD_OSS_ACCESS_KEY");
        assert_eq!(secret_key_env("my-disk"), "GITGIT_CLOUD_MY_DISK_SECRET_KEY");
    }

    #[test]
    fn a_disk_without_an_endpoint_or_a_bucket_is_refused() {
        let mut e = entry("oss");
        e.endpoint = String::new();
        assert!(e.validate().is_err());
        let mut e = entry("oss");
        e.bucket = String::new();
        assert!(e.validate().is_err());
        assert!(entry("oss").validate().is_ok());
    }

    #[test]
    fn a_missing_registry_is_an_empty_registry() {
        let dir = std::env::temp_dir().join(format!(
            "gitgit-clouds-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let path = dir.join("clouds.json");
        let registry = CloudRegistry::load(&path).unwrap();
        assert!(registry.disks.is_empty());
        assert!(registry.get("oss").is_err());
        assert!(registry.only().is_err());
    }

    #[test]
    fn a_registry_round_trips() {
        let dir = std::env::temp_dir().join(format!(
            "gitgit-clouds-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let path = dir.join("clouds.json");
        let mut registry = CloudRegistry::default();
        registry.disks.insert("oss".to_string(), entry("oss"));
        registry.save(&path).unwrap();
        let back = CloudRegistry::load(&path).unwrap();
        assert_eq!(back.disks.len(), 1);
        assert_eq!(back.get("oss").unwrap().bucket, "snapshots");
        assert_eq!(back.only().unwrap().name, "oss");
    }

    #[test]
    fn several_disks_cannot_be_picked_without_a_name() {
        let mut registry = CloudRegistry::default();
        registry.disks.insert("oss".to_string(), entry("oss"));
        registry.disks.insert("cos".to_string(), entry("cos"));
        assert!(registry.only().is_err());
    }
}
