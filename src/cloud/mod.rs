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
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
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
    ///
    /// `[FACT]` Only the disk's **own** variable is read here ✓ — the
    /// generic fallback belongs to the registry ✓, because the registry is
    /// the only place that knows how many disks are configured ✓.
    pub fn access_key(&self) -> Result<String> {
        read_secret(&access_key_env(&self.name)).ok_or_else(|| {
            GitGitError::Cloud(format!(
                "no access key for the cloud disk {:?}: set {}",
                self.name,
                access_key_env(&self.name)
            ))
        })
    }

    /// The secret key for this disk, read from the environment.
    ///
    /// `[FACT]` Only the disk's **own** variable is read here ✓ — see
    /// [`CloudDiskEntry::access_key`] ✓.
    pub fn secret_key(&self) -> Result<String> {
        read_secret(&secret_key_env(&self.name)).ok_or_else(|| {
            GitGitError::Cloud(format!(
                "no secret key for the cloud disk {:?}: set {}",
                self.name,
                secret_key_env(&self.name)
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
    /// The key the object is stored under ✓ — the **disk-local** name ✓,
    /// so it is the name a restore takes ✓.
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
    /// The directories the snapshot holds that hold **no file** ✓.
    ///
    /// `[FACT]` A file inside a directory recreates that directory on the
    /// way back ✓ — so the only directory a restore can lose is one with
    /// **nothing** in it ✓. That is not a cosmetic loss: a bare repository
    /// without `refs/` is not a repository git will touch ✓.
    ///
    /// `[FACT]` The list is empty for a version-1 snapshot ✓ — those never
    /// carried it ✓ — so restoring one leaves the directories it could not
    /// know about missing ✓.
    #[serde(default)]
    pub directories: Vec<String>,
}

/// The magic that starts every snapshot file.
///
/// `[FACT]` It is the ASCII text `GITGIT-SNAPSHOT` ✓ followed by a
/// newline ✓ — so a snapshot that someone opened by mistake is obvious ✓.
pub const SNAPSHOT_MAGIC: &[u8] = b"GITGIT-SNAPSHOT\n";

/// The version of the snapshot format this build writes.
///
/// `[FACT]` Version 2 adds the list of **empty** directories to the
/// metadata ✓ — version 1 wrote only the files ✓, so a restore of a
/// version-1 snapshot cannot bring back a directory that held no file ✓.
pub const SNAPSHOT_VERSION: u32 = 2;

/// The version before the empty-directory list existed.
///
/// `[FACT]` A snapshot of that version is still readable ✓ — the field it
/// lacks has a default ✓ — but it cannot restore an empty directory ✓,
/// because it never recorded one ✓.
pub const LEGACY_SNAPSHOT_VERSION: u32 = 1;

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
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| {
            GitGitError::Cloud(format!("the cloud disk registry is unwritable: {e}"))
        })?;
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

    /// The credentials for `entry`.
    ///
    /// `[FACT]` The disk's own variables win ✓.
    ///
    /// `[FACT]` The generic variables are a fallback for a **single-disk**
    /// setup ✓ — the ADR says so ✓ — and they are used **only when
    /// exactly one disk is configured** ✓. With more than one, a generic
    /// credential would authenticate one disk with a secret meant for
    /// another connection ✓, and the failure would name the wrong
    /// variable ✓.
    pub fn credentials_for(&self, entry: &CloudDiskEntry) -> Result<(String, String)> {
        let single = self.disks.len() == 1;
        let access = entry.access_key().or_else(|_| {
            if single {
                read_secret(FALLBACK_ACCESS_KEY_ENV).ok_or_else(|| {
                    GitGitError::Cloud(format!(
                        "no access key for the cloud disk {:?}: set {} (or {})",
                        entry.name,
                        access_key_env(&entry.name),
                        FALLBACK_ACCESS_KEY_ENV
                    ))
                })
            } else {
                Err(GitGitError::Cloud(format!(
                    "no access key for the cloud disk {:?}: set {} (the generic {} is only used when one disk is configured)",
                    entry.name,
                    access_key_env(&entry.name),
                    FALLBACK_ACCESS_KEY_ENV
                )))
            }
        })?;
        let secret = entry.secret_key().or_else(|_| {
            if single {
                read_secret(FALLBACK_SECRET_KEY_ENV).ok_or_else(|| {
                    GitGitError::Cloud(format!(
                        "no secret key for the cloud disk {:?}: set {} (or {})",
                        entry.name,
                        secret_key_env(&entry.name),
                        FALLBACK_SECRET_KEY_ENV
                    ))
                })
            } else {
                Err(GitGitError::Cloud(format!(
                    "no secret key for the cloud disk {:?}: set {} (the generic {} is only used when one disk is configured)",
                    entry.name,
                    secret_key_env(&entry.name),
                    FALLBACK_SECRET_KEY_ENV
                )))
            }
        })?;
        Ok((access, secret))
    }

    /// The only connected disk, when there is exactly one ✓.
    pub fn only(&self) -> Result<&CloudDiskEntry> {
        match self.disks.len() {
            1 => match self.disks.values().next() {
                Some(entry) => Ok(entry),
                None => Err(GitGitError::Cloud(
                    "no cloud disk is connected (gitgit cloud add ...)".to_string(),
                )),
            },
            0 => Err(GitGitError::Cloud(
                "no cloud disk is connected (gitgit cloud add ...)".to_string(),
            )),
            n => Err(GitGitError::Cloud(format!(
                "{n} cloud disks are connected; name one with --disk"
            ))),
        }
    }
}

/// The key an object named `name` is stored under on a disk whose prefix
/// is `prefix` ✓.
///
/// `[FACT]` The prefix belongs to the **disk** ✓, not to the caller ✓:
/// everything above this module speaks *disk-local* names ✓, and the disk
/// adds its own prefix on the way out ✓. One place composes the key ✓, so
/// a snapshot cannot be written under one prefix and looked for under
/// another ✓.
pub fn object_key(prefix: &str, name: &str) -> String {
    format!("{prefix}{name}")
}

/// The disk-local name of the object stored under `key` on a disk whose
/// prefix is `prefix` ✓.
///
/// `[FACT]` A key that does not carry the prefix is returned unchanged ✓:
/// the listing strips what it added ✓, and a key from another disk is not
/// silently renamed into this one ✓.
pub fn local_name(prefix: &str, key: &str) -> String {
    key.strip_prefix(prefix).unwrap_or(key).to_string()
}

/// A remote that holds snapshots.
///
/// `[FACT]` Every name in this trait is **disk-local** ✓: the disk's own
/// key prefix is applied by the disk ✓, on the way in ✓ and on the way out
/// ✓, and the listing reports the local names ✓ — so a caller that
/// writes `snapshots/demo/x.tar` and later lists or restores that same
/// name is talking about the same object ✓, whatever prefix the disk was
/// configured with ✓.
#[async_trait::async_trait]
pub trait CloudDisk: Send + Sync {
    /// Store `bytes` under the local name `key` ✓ and return the server's
    /// version id ✓, when it has one ✓.
    async fn put(&self, key: &str, bytes: &[u8]) -> Result<Option<String>>;

    /// Read the object at the local name `key` ✓.
    async fn get(&self, key: &str) -> Result<Vec<u8>>;

    /// List the objects whose local name starts with `prefix` ✓.
    async fn list(&self, prefix: &str) -> Result<Vec<CloudObject>>;

    /// Delete the object at the local name `key` ✓.
    async fn delete(&self, key: &str) -> Result<()>;
}

/// A cloud disk reached over S3-compatible HTTP.
pub struct S3CloudDisk {
    bucket: Box<s3::Bucket>,
    key_prefix: String,
}

impl S3CloudDisk {
    /// Connect to the disk described by `entry`, with the credentials the
    /// registry resolved for it.
    ///
    /// `[FACT]` The credentials are passed in ✓ rather than read from the
    /// environment here ✓, because the **registry** is the place that
    /// knows how many disks there are ✓ — and the generic fallback is only
    /// for a single-disk setup ✓ (ADR-0025 §2.2) ✓.
    pub async fn connect(
        entry: &CloudDiskEntry,
        access_key: &str,
        secret_key: &str,
    ) -> Result<Self> {
        entry.validate()?;
        let region = s3::Region::Custom {
            region: entry.region.clone(),
            endpoint: entry.endpoint.clone(),
        };
        let credentials =
            s3::creds::Credentials::new(Some(access_key), Some(secret_key), None, None, None)
                .map_err(|e| {
                    GitGitError::Cloud(format!("the cloud credentials are unusable: {e}"))
                })?;
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
    ///
    /// `[FACT]` The disk's prefix is applied here ✓ — and only here ✓.
    pub fn key(&self, name: &str) -> String {
        object_key(&self.key_prefix, name)
    }
}

#[async_trait::async_trait]
impl CloudDisk for S3CloudDisk {
    async fn put(&self, key: &str, bytes: &[u8]) -> Result<Option<String>> {
        let response = self
            .bucket
            .put_object(&self.key(key), bytes)
            .await
            .map_err(|e| GitGitError::Cloud(format!("the snapshot could not be uploaded: {e}")))?;
        let headers = response.headers();
        Ok(headers
            .get("x-amz-version-id")
            .or_else(|| headers.get("X-Amz-Version-Id"))
            .cloned())
    }

    async fn get(&self, key: &str) -> Result<Vec<u8>> {
        let object = self.bucket.get_object(&self.key(key)).await.map_err(|e| {
            GitGitError::Cloud(format!("the snapshot could not be downloaded: {e}"))
        })?;
        Ok(object.bytes().to_vec())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<CloudObject>> {
        let pages = self
            .bucket
            .list(self.key(prefix), None)
            .await
            .map_err(|e| GitGitError::Cloud(format!("the snapshots could not be listed: {e}")))?;
        let mut out = Vec::new();
        for page in pages {
            for object in page.contents {
                out.push(CloudObject {
                    // The listing reports the disk-local name ✓, so the
                    // name a caller sees is the name it can restore from ✓.
                    key: local_name(&self.key_prefix, &object.key),
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
            .delete_object(&self.key(key))
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
    fn the_disk_prefix_is_added_on_the_way_out_and_stripped_on_the_way_in() {
        // The prefix belongs to the disk. Everything above it speaks
        // disk-local names, so a snapshot written as
        // `snapshots/demo/x.tar` lands under `<prefix>snapshots/demo/x.tar`
        // and is listed and restored under the name it was written with.
        let prefix = "gitgit/";
        let name = "snapshots/demo/x.tar";
        assert_eq!(object_key(prefix, name), "gitgit/snapshots/demo/x.tar");
        assert_eq!(
            local_name(prefix, &object_key(prefix, name)),
            "snapshots/demo/x.tar"
        );
        // A key that carries no prefix is not renamed into this disk.
        assert_eq!(local_name(prefix, "other/x.tar"), "other/x.tar");
        // An empty prefix is the disk's own key space.
        assert_eq!(object_key("", name), name);
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

    #[test]
    fn the_generic_credential_is_a_fallback_for_one_disk_only() {
        // `[FACT]` The generic variables are documented for a single-disk
        // setup. With two disks, a disk whose own variable is missing must
        // NOT authenticate with a secret meant for another connection.
        let mut one = CloudRegistry::default();
        one.disks.insert("oss".to_string(), entry("oss"));
        let mut two = CloudRegistry::default();
        two.disks.insert("oss".to_string(), entry("oss"));
        two.disks.insert("cos".to_string(), entry("cos"));

        // The disk's own variable wins, in both setups.
        std::env::set_var(access_key_env("oss"), "own-access");
        std::env::set_var(secret_key_env("oss"), "own-secret");
        let (access, secret) = one.credentials_for(one.get("oss").unwrap()).unwrap();
        assert_eq!(access, "own-access");
        assert_eq!(secret, "own-secret");

        // Without it, the generic variables are the fallback — but only
        // while there is one disk.
        std::env::remove_var(access_key_env("oss"));
        std::env::remove_var(secret_key_env("oss"));
        std::env::set_var(FALLBACK_ACCESS_KEY_ENV, "generic-access");
        std::env::set_var(FALLBACK_SECRET_KEY_ENV, "generic-secret");
        let (access, secret) = one.credentials_for(one.get("oss").unwrap()).unwrap();
        assert_eq!(access, "generic-access");
        assert_eq!(secret, "generic-secret");

        let err = two
            .credentials_for(two.get("oss").unwrap())
            .expect_err("the generic credential is not a fallback for two disks");
        assert!(
            format!("{err}").contains("only used when one disk is configured"),
            "got: {err}"
        );

        std::env::remove_var(FALLBACK_ACCESS_KEY_ENV);
        std::env::remove_var(FALLBACK_SECRET_KEY_ENV);
    }
}
