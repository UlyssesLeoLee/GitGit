//! Filesystem backend of the self-hosted object store.
//!
//! ## Layout
//!
//! ```text
//! <root>/<bucket>/versioning.json                    {"status":"Enabled"}
//! <root>/<bucket>/versions/<version-id>/<key>.bin    the stored bytes
//! <root>/<bucket>/versions/<version-id>/<key>.json   the stored metadata
//! <root>/<bucket>/tombstones/<key>.json              the key was deleted
//! ```
//!
//! [FACT] `<key>` is the SHA-256 of the key name's UTF-8 bytes, hex
//! encoded. A key may contain `/` or any other character, so hashing it
//! gives a safe file name; the original key is kept in the `.json`
//! sidecar and is what a listing reports.
//!
//! [FACT] `<version-id>` is a UUIDv4 in its simple form (32 lowercase hex
//! characters). Every write creates a new version directory, so a key's
//! history is the set of version directories that hold its `.json`.
//!
//! ## What "the current version" means
//!
//! The current version of a key is the version directory with the newest
//! `created` timestamp; the version id breaks a tie. A key that was
//! deleted has **no** current version — the delete writes a tombstone and
//! removes the current version's files, but older versions stay on disk
//! and remain readable through `?versionId=`.
//!
//! [FACT] The tombstone is what makes a plain `GET` answer 404 after a
//! delete while an old `versionId` still resolves. Without it, "remove the
//! current version, keep the older ones" would make the plain read fall
//! back to the previous version, which is not what a delete means.
//!
//! ## The listing is a scan, and that is a known trade-off
//!
//! [INFERENCE] A listing walks every version directory of the bucket and
//! reads every `.json` sidecar to work out the current version of each
//! key. That is O(versions) work per listing, which is the right trade for
//! a credential vault that holds a handful of keys, and it keeps the
//! on-disk format to a single directory per write. **If the key count
//! grows, this is the extension point**: a per-key index file (or an
//! embedded database) would turn the scan into a lookup. The trade-off is
//! called out here so it does not become an implicit assumption.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

/// The default page size of a listing, matching S3.
pub const DEFAULT_MAX_KEYS: usize = 1000;

/// The largest page size a listing honours, matching S3.
pub const MAX_MAX_KEYS: usize = 1000;

/// Whether a bucket keeps its version history.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum VersioningStatus {
    /// The bucket was never versioned.
    Disabled,
    /// The bucket keeps the history of every write.
    Enabled,
}

/// The metadata stored next to an object's bytes.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VersionMeta {
    /// The original object key.
    pub key: String,
    /// The length of the stored bytes.
    pub size: u64,
    /// The hex SHA-256 of the stored bytes; also the `ETag`.
    pub etag: String,
    /// When the version was written, RFC 3339 in UTC.
    pub created: String,
    /// The content type the writer announced, if any.
    #[serde(default)]
    pub content_type: Option<String>,
}

/// An object together with the bytes it holds.
#[derive(Clone, Debug)]
pub struct StoredObject {
    /// The version id the object is stored under.
    pub version_id: String,
    /// The object's metadata.
    pub meta: VersionMeta,
    /// The stored bytes.
    pub bytes: Vec<u8>,
}

/// One key of a listing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListEntry {
    /// The object key.
    pub key: String,
    /// The length of the current version.
    pub size: u64,
    /// The `ETag` of the current version.
    pub etag: String,
    /// When the current version was written.
    pub last_modified: String,
}

/// One page of a listing.
#[derive(Clone, Debug)]
pub struct ListPage {
    /// The keys of this page, in ascending key order.
    pub entries: Vec<ListEntry>,
    /// Whether more keys follow this page.
    pub is_truncated: bool,
    /// The token that fetches the next page.
    pub next_continuation_token: Option<String>,
    /// The prefix the listing was filtered by.
    pub prefix: String,
    /// The page size that was applied.
    pub max_keys: usize,
}

/// Errors the storage layer can raise.
#[derive(Debug, Error)]
pub enum StoreError {
    /// A filesystem operation failed.
    #[error("object store io error: {0}")]
    Io(#[from] std::io::Error),
    /// A sidecar could not be encoded or decoded.
    #[error("object store metadata error: {0}")]
    Serde(#[from] serde_json::Error),
    /// The bucket name is not a legal S3 bucket name.
    #[error("invalid bucket name: {0:?}")]
    InvalidBucketName(String),
    /// The version id is not one this store could have issued.
    #[error("invalid version id: {0:?}")]
    InvalidVersionId(String),
    /// The object key is empty.
    #[error("invalid object key: {0:?}")]
    InvalidKey(String),
    /// The continuation token is not one this server issued.
    #[error("invalid continuation token: {0}")]
    InvalidContinuationToken(String),
    /// The bucket does not exist.
    #[error("no such bucket: {0}")]
    NoSuchBucket(String),
    /// Versioning cannot be turned off.
    ///
    /// [FACT] This store always versions, because the `x-amz-version-id`
    /// response header is load-bearing for the credential timeline in
    /// `server::vault_versioned`. Accepting a request to stop versioning and
    /// then reporting the state the client asked for would be a false report
    /// about this store's own behaviour.
    #[error("this object store always versions: {0:?} cannot be honoured")]
    VersioningCannotBeDisabled(VersioningStatus),
}

/// The tombstone that marks a key as deleted.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Tombstone {
    key: String,
    deleted_version: String,
    deleted: String,
}

/// The state of a bucket's versioning.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct VersioningFile {
    status: VersioningStatus,
}

/// The continuation token of a listing.
///
/// [PROPOSAL] The token is opaque to the client: it carries the last key
/// that was returned plus an offset, so a page can be resumed exactly where
/// the previous one stopped.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct ContinuationToken {
    last: String,
    offset: usize,
}

/// A filesystem-backed object store.
#[derive(Clone, Debug)]
pub struct FileStore {
    root: PathBuf,
}

impl FileStore {
    /// Build a store rooted at `root`. The directory does not have to
    /// exist yet.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The root directory of the store.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Create the root directory if it is missing.
    pub async fn ensure_root(&self) -> Result<(), StoreError> {
        tokio::fs::create_dir_all(&self.root).await?;
        Ok(())
    }

    /// The directory of a bucket.
    fn bucket_dir(&self, bucket: &str) -> PathBuf {
        self.root.join(bucket)
    }

    /// The directory that holds every version of a bucket.
    fn versions_dir(&self, bucket: &str) -> PathBuf {
        self.bucket_dir(bucket).join("versions")
    }

    /// The directory that holds the delete markers of a bucket.
    fn tombstones_dir(&self, bucket: &str) -> PathBuf {
        self.bucket_dir(bucket).join("tombstones")
    }

    /// The file that holds the versioning state of a bucket.
    fn versioning_path(&self, bucket: &str) -> PathBuf {
        self.bucket_dir(bucket).join("versioning.json")
    }

    /// The directory of one version.
    fn version_dir(&self, bucket: &str, version_id: &str) -> PathBuf {
        self.versions_dir(bucket).join(version_id)
    }

    /// The file that holds the bytes of one version.
    fn blob_path(&self, bucket: &str, version_id: &str, key_hash: &str) -> PathBuf {
        self.version_dir(bucket, version_id)
            .join(format!("{key_hash}.bin"))
    }

    /// The file that holds the metadata of one version.
    fn meta_path(&self, bucket: &str, version_id: &str, key_hash: &str) -> PathBuf {
        self.version_dir(bucket, version_id)
            .join(format!("{key_hash}.json"))
    }

    /// The file that marks a key as deleted.
    fn tombstone_path(&self, bucket: &str, key_hash: &str) -> PathBuf {
        self.tombstones_dir(bucket).join(format!("{key_hash}.json"))
    }

    /// Whether the bucket exists.
    pub async fn bucket_exists(&self, bucket: &str) -> Result<bool, StoreError> {
        validate_bucket_name(bucket)?;
        Ok(self.bucket_dir(bucket).is_dir())
    }

    /// Create the bucket. Returns `true` when the bucket was created and
    /// `false` when it already existed — creating a bucket twice is not an
    /// error, which is what `mc mb --ignore-existing` does.
    pub async fn create_bucket(&self, bucket: &str) -> Result<bool, StoreError> {
        validate_bucket_name(bucket)?;
        let dir = self.bucket_dir(bucket);
        if dir.is_dir() {
            return Ok(false);
        }
        tokio::fs::create_dir_all(self.versions_dir(bucket)).await?;
        tokio::fs::create_dir_all(self.tombstones_dir(bucket)).await?;
        Ok(true)
    }

    /// The versioning state of a bucket.
    ///
    /// [FACT] This store **always** versions: every write creates a version
    /// directory and the response carries an `x-amz-version-id`, because that
    /// header is load-bearing for the credential timeline in
    /// `server::vault_versioned` — without it every stored credential keeps
    /// an empty `version_id`.
    ///
    /// [INFERENCE] So the honest answer for a bucket that has never been
    /// explicitly versioned is `Enabled`. Reporting `Disabled` would be a
    /// false report about this server's own behaviour, and a client that
    /// trusted it would be wrong in a way nothing would report.
    pub async fn get_versioning(&self, bucket: &str) -> Result<VersioningStatus, StoreError> {
        validate_bucket_name(bucket)?;
        match tokio::fs::read(self.versioning_path(bucket)).await {
            Ok(bytes) => {
                let file: VersioningFile = serde_json::from_slice(&bytes)?;
                Ok(file.status)
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(VersioningStatus::Enabled),
            Err(err) => Err(err.into()),
        }
    }

    /// Turn versioning on for a bucket.
    ///
    /// [FACT] Versioning cannot be turned off here: the store always keeps
    /// every version, because the `x-amz-version-id` response header is
    /// load-bearing for the credential timeline in `server::vault_versioned`.
    /// A request to disable it is refused with a clear error rather than
    /// accepted and then reported back as if it had been honoured.
    pub async fn set_versioning(
        &self,
        bucket: &str,
        status: VersioningStatus,
    ) -> Result<(), StoreError> {
        validate_bucket_name(bucket)?;
        if !self.bucket_dir(bucket).is_dir() {
            return Err(StoreError::NoSuchBucket(bucket.to_string()));
        }
        if status != VersioningStatus::Enabled {
            return Err(StoreError::VersioningCannotBeDisabled(status));
        }
        let file = VersioningFile { status };
        let bytes = serde_json::to_vec_pretty(&file)?;
        write_atomic(&self.versioning_path(bucket), &bytes).await?;
        Ok(())
    }

    /// Store a new version of `key` and return its version id.
    pub async fn put(
        &self,
        bucket: &str,
        key: &str,
        bytes: &[u8],
        content_type: Option<&str>,
    ) -> Result<String, StoreError> {
        validate_bucket_name(bucket)?;
        validate_key(key)?;
        if !self.bucket_dir(bucket).is_dir() {
            return Err(StoreError::NoSuchBucket(bucket.to_string()));
        }

        let version_id = Uuid::new_v4().simple().to_string();
        let key_hash = key_hash(key);
        let meta = VersionMeta {
            key: key.to_string(),
            size: bytes.len() as u64,
            etag: sha256_hex(bytes),
            created: now_rfc3339(),
            content_type: content_type.map(str::to_string),
        };

        let dir = self.version_dir(bucket, &version_id);
        tokio::fs::create_dir_all(&dir).await?;
        write_atomic(&self.blob_path(bucket, &version_id, &key_hash), bytes).await?;
        let meta_bytes = serde_json::to_vec_pretty(&meta)?;
        write_atomic(&self.meta_path(bucket, &version_id, &key_hash), &meta_bytes).await?;

        // A write brings a deleted key back.
        let tombstone = self.tombstone_path(bucket, &key_hash);
        if tombstone.exists() {
            tokio::fs::remove_file(&tombstone).await?;
        }

        Ok(version_id)
    }

    /// The current version of a key.
    pub async fn get_current(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<Option<StoredObject>, StoreError> {
        validate_bucket_name(bucket)?;
        validate_key(key)?;
        let key_hash = key_hash(key);
        // A tombstoned key has no current version.
        if self.tombstone_path(bucket, &key_hash).exists() {
            return Ok(None);
        }
        let Some(version_id) = self.find_current_version(bucket, &key_hash).await? else {
            return Ok(None);
        };
        self.read_version(bucket, &version_id, &key_hash).await
    }

    /// One specific version of a key.
    pub async fn get_version(
        &self,
        bucket: &str,
        key: &str,
        version_id: &str,
    ) -> Result<Option<StoredObject>, StoreError> {
        validate_bucket_name(bucket)?;
        validate_key(key)?;
        validate_version_id(version_id)?;
        let key_hash = key_hash(key);
        if !self.blob_path(bucket, version_id, &key_hash).exists() {
            return Ok(None);
        }
        self.read_version(bucket, version_id, &key_hash).await
    }

    /// Remove the current version of a key.
    ///
    /// The key stops being visible — a plain read answers "not found" —
    /// but older versions stay on disk and remain readable through their
    /// version id. Deleting a key that is not there is not an error.
    pub async fn delete(&self, bucket: &str, key: &str) -> Result<(), StoreError> {
        validate_bucket_name(bucket)?;
        validate_key(key)?;
        let key_hash = key_hash(key);
        let Some(object) = self.get_current(bucket, key).await? else {
            return Ok(());
        };

        tokio::fs::remove_file(self.blob_path(bucket, &object.version_id, &key_hash)).await?;
        tokio::fs::remove_file(self.meta_path(bucket, &object.version_id, &key_hash)).await?;
        // Prune the version directory when this write was its only key.
        let dir = self.version_dir(bucket, &object.version_id);
        if let Ok(mut entries) = tokio::fs::read_dir(&dir).await {
            if entries
                .next_entry()
                .await
                .map(|e| e.is_none())
                .unwrap_or(false)
            {
                let _ = tokio::fs::remove_dir(&dir).await;
            }
        }

        let tombstone = Tombstone {
            key: key.to_string(),
            deleted_version: object.version_id,
            deleted: now_rfc3339(),
        };
        let bytes = serde_json::to_vec_pretty(&tombstone)?;
        tokio::fs::create_dir_all(self.tombstones_dir(bucket)).await?;
        write_atomic(&self.tombstone_path(bucket, &key_hash), &bytes).await?;
        Ok(())
    }

    /// One page of the bucket's keys.
    pub async fn list(
        &self,
        bucket: &str,
        prefix: &str,
        continuation_token: Option<&str>,
        max_keys: usize,
    ) -> Result<ListPage, StoreError> {
        validate_bucket_name(bucket)?;
        let max_keys = max_keys.clamp(1, MAX_MAX_KEYS);

        // Step 1: the current version of every key in the bucket.
        let mut current: HashMap<String, (String, String, VersionMeta)> = HashMap::new();
        let versions_dir = self.versions_dir(bucket);
        if versions_dir.is_dir() {
            let mut version_dirs = tokio::fs::read_dir(&versions_dir).await?;
            while let Some(entry) = version_dirs.next_entry().await? {
                let version_id = entry.file_name().to_string_lossy().to_string();
                if !entry.path().is_dir() {
                    continue;
                }
                let mut files = tokio::fs::read_dir(entry.path()).await?;
                while let Some(file) = files.next_entry().await? {
                    let name = file.file_name().to_string_lossy().to_string();
                    if !name.ends_with(".json") {
                        continue;
                    }
                    let bytes = match tokio::fs::read(file.path()).await {
                        Ok(bytes) => bytes,
                        Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
                        Err(err) => return Err(err.into()),
                    };
                    let meta: VersionMeta = match serde_json::from_slice(&bytes) {
                        Ok(meta) => meta,
                        // A sidecar that cannot be parsed is skipped rather
                        // than failing the whole listing: one damaged file
                        // must not take the bucket offline.
                        Err(_) => continue,
                    };
                    let replace = match current.get(&meta.key) {
                        None => true,
                        Some((created, vid, _)) => (&meta.created, &version_id) > (created, vid),
                    };
                    if replace {
                        current.insert(
                            meta.key.clone(),
                            (meta.created.clone(), version_id.clone(), meta),
                        );
                    }
                }
            }
        }

        // Step 2: drop the keys that were deleted.
        let tombstones_dir = self.tombstones_dir(bucket);
        if tombstones_dir.is_dir() {
            let mut files = tokio::fs::read_dir(&tombstones_dir).await?;
            while let Some(file) = files.next_entry().await? {
                let name = file.file_name().to_string_lossy().to_string();
                if !name.ends_with(".json") {
                    continue;
                }
                let bytes = match tokio::fs::read(file.path()).await {
                    Ok(bytes) => bytes,
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(err) => return Err(err.into()),
                };
                let tombstone: Tombstone = match serde_json::from_slice(&bytes) {
                    Ok(tombstone) => tombstone,
                    Err(_) => continue,
                };
                current.remove(&tombstone.key);
            }
        }

        // Step 3: order by key, then apply the prefix filter.
        let mut entries: Vec<ListEntry> = current
            .into_values()
            .map(|(_, _, meta)| ListEntry {
                key: meta.key,
                size: meta.size,
                etag: meta.etag,
                last_modified: meta.created,
            })
            .collect();
        entries.sort_by(|a, b| a.key.cmp(&b.key));
        entries.retain(|entry| entry.key.starts_with(prefix));

        // Step 4: resume after the token, if there is one.
        let mut start = 0usize;
        if let Some(token) = continuation_token {
            let token = decode_continuation_token(token)?;
            start = match entries.iter().position(|entry| entry.key == token.last) {
                Some(index) => index + 1 + token.offset,
                // The key the token points at is gone; resume after the
                // last key that is still smaller than it.
                None => entries.partition_point(|entry| entry.key < token.last),
            };
        }

        // Step 5: cut the page and build the next token.
        let remaining = entries.len().saturating_sub(start);
        let take = remaining.min(max_keys);
        let page: Vec<ListEntry> = entries[start..start + take].to_vec();
        let is_truncated = remaining > take;
        let next_continuation_token = if is_truncated {
            page.last().map(|entry| {
                encode_continuation_token(&ContinuationToken {
                    last: entry.key.clone(),
                    offset: 0,
                })
            })
        } else {
            None
        };

        Ok(ListPage {
            entries: page,
            is_truncated,
            next_continuation_token,
            prefix: prefix.to_string(),
            max_keys,
        })
    }

    /// The version id that holds the current version of a key.
    async fn find_current_version(
        &self,
        bucket: &str,
        key_hash: &str,
    ) -> Result<Option<String>, StoreError> {
        let versions_dir = self.versions_dir(bucket);
        if !versions_dir.is_dir() {
            return Ok(None);
        }
        let mut best: Option<(String, String)> = None;
        let mut version_dirs = tokio::fs::read_dir(&versions_dir).await?;
        while let Some(entry) = version_dirs.next_entry().await? {
            if !entry.path().is_dir() {
                continue;
            }
            let version_id = entry.file_name().to_string_lossy().to_string();
            let meta_path = self.meta_path(bucket, &version_id, key_hash);
            let bytes = match tokio::fs::read(&meta_path).await {
                Ok(bytes) => bytes,
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
                Err(err) => return Err(err.into()),
            };
            let meta: VersionMeta = match serde_json::from_slice(&bytes) {
                Ok(meta) => meta,
                Err(_) => continue,
            };
            let replace = match &best {
                None => true,
                Some((created, vid)) => (&meta.created, &version_id) > (created, vid),
            };
            if replace {
                best = Some((meta.created, version_id));
            }
        }
        Ok(best.map(|(_, version_id)| version_id))
    }

    /// Read one version of a key.
    async fn read_version(
        &self,
        bucket: &str,
        version_id: &str,
        key_hash: &str,
    ) -> Result<Option<StoredObject>, StoreError> {
        let blob = self.blob_path(bucket, version_id, key_hash);
        let bytes = match tokio::fs::read(&blob).await {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err.into()),
        };
        let meta_bytes = tokio::fs::read(self.meta_path(bucket, version_id, key_hash)).await?;
        let meta: VersionMeta = serde_json::from_slice(&meta_bytes)?;
        Ok(Some(StoredObject {
            version_id: version_id.to_string(),
            meta,
            bytes,
        }))
    }
}

/// The hash of a key name — the file name its bytes are stored under.
pub fn key_hash(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

/// Hex-encoded SHA-256.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Reject bucket names that are not legal S3 bucket names.
///
/// [FACT] S3 bucket names are lowercase, 3-63 characters long, and made of
/// letters, digits, dots and hyphens.
///
/// [FACT] The length is checked on **both** ends: the name has to be at
/// least 3 characters long and at most 63. The upper bound is S3's; the
/// lower one is the same rule, and it was written down here before the code
/// enforced it — the check now matches what this comment says.
///
/// [FACT] This check is the store's **containment boundary**, not a
/// cosmetic rule. The bucket name arrives from the request path, is
/// percent-decoded, and is then joined onto the root directory. Two shapes
/// would escape the root, and both are refused here:
///
/// * a relative name with `..` in it (`../escape`) — the join walks up out
///   of the root;
/// * an **absolute** name (`C:/escape`) — on Windows a `Path::join` with an
///   absolute argument *replaces* the path it is joining onto, so the root
///   would be dropped on the floor entirely.
///
/// [INFERENCE] The name reaches the server percent-encoded often enough
/// that the `..` does not have to be literal: `..%2fescape` is one URL
/// segment (a WHATWG URL parser does not treat `%2f` as a separator), and
/// the server decodes it into `../escape`. The check therefore happens
/// **after** the decoding, which is where the name becomes a path.
pub fn validate_bucket_name(bucket: &str) -> Result<(), StoreError> {
    let acceptable = bucket.len() >= 3
        && bucket.len() <= 63
        && bucket
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.')
        && !bucket.starts_with('.')
        && !bucket.ends_with('.')
        && !bucket.contains("..");
    if acceptable {
        Ok(())
    } else {
        Err(StoreError::InvalidBucketName(bucket.to_string()))
    }
}

/// Reject a version id this store could not have issued.
///
/// [FACT] A version id is a UUIDv4 in its simple form: 32 lowercase hex
/// characters. The id arrives from the client (as `?versionId=`) and
/// becomes a directory name, so it is checked before it is used as one.
///
/// [FACT] The check is the same kind of boundary as the bucket name's: a
/// version id of `../../escape` would walk out of the bucket directory,
/// and an absolute one (`C:/escape`) would replace it.
pub fn validate_version_id(version_id: &str) -> Result<(), StoreError> {
    let acceptable = version_id.len() == 32
        && version_id
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c));
    if acceptable {
        Ok(())
    } else {
        Err(StoreError::InvalidVersionId(version_id.to_string()))
    }
}

/// Reject an empty object key.
pub fn validate_key(key: &str) -> Result<(), StoreError> {
    if key.is_empty() {
        return Err(StoreError::InvalidKey(key.to_string()));
    }
    Ok(())
}

/// Write a file atomically: write to a temporary file in the same
/// directory, then rename it over the target.
async fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let mut temporary = path.to_path_buf();
    temporary.set_file_name(format!(".tmp.{}", Uuid::new_v4().simple()));
    tokio::fs::write(&temporary, bytes).await?;
    if let Err(err) = tokio::fs::rename(&temporary, path).await {
        let _ = tokio::fs::remove_file(&temporary).await;
        return Err(err.into());
    }
    Ok(())
}

/// Encode a continuation token into its opaque wire form.
fn encode_continuation_token(token: &ContinuationToken) -> String {
    let json = serde_json::to_vec(token).unwrap_or_default();
    URL_SAFE_NO_PAD.encode(json)
}

/// Decode a continuation token.
fn decode_continuation_token(raw: &str) -> Result<ContinuationToken, StoreError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(raw)
        .map_err(|_| StoreError::InvalidContinuationToken(raw.to_string()))?;
    serde_json::from_slice(&bytes)
        .map_err(|_| StoreError::InvalidContinuationToken(raw.to_string()))
}

/// The current time as an RFC 3339 UTC timestamp.
///
/// [FACT] The format is fixed-width (`2026-10-25T13:35:21.727000Z`), so
/// comparing two of them as strings compares them as instants. That is what
/// lets "the newest version" be a string comparison.
pub fn now_rfc3339() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_micros() as i64)
        .unwrap_or(0);
    rfc3339_from_micros(now)
}

/// Format microseconds since the Unix epoch as an RFC 3339 UTC timestamp.
pub fn rfc3339_from_micros(micros: i64) -> String {
    let seconds = micros.div_euclid(1_000_000);
    let fraction = micros.rem_euclid(1_000_000);
    let days = seconds.div_euclid(86_400);
    let time_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = time_of_day / 3_600;
    let minute = (time_of_day % 3_600) / 60;
    let second = time_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{fraction:06}Z")
}

/// The civil date of a day count since the Unix epoch (Howard Hinnant's
/// `civil_from_days`).
pub fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = (shifted - era * 146_097) as u64;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era as i64 + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_prime + 2) / 5 + 1) as u32;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    } as u32;
    let year = if month <= 2 { year + 1 } else { year };
    (year, month, day)
}

/// A metadata map, keyed by lowercase header name.
///
/// [FACT] `rust-s3` reads the `x-amz-version-id` response header by its
/// lowercase name, so the server has to spell it that way.
pub type HeaderMap = BTreeMap<String, String>;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn rfc3339_formats_the_epoch() {
        assert_eq!(rfc3339_from_micros(0), "1970-01-01T00:00:00.000000Z");
    }

    #[test]
    fn rfc3339_is_fixed_width_so_strings_sort_as_instants() {
        let early = rfc3339_from_micros(1_000_000);
        let late = rfc3339_from_micros(2_000_000);
        assert!(early < late);
        assert_eq!(early.len(), late.len());
    }

    #[test]
    fn bucket_names_are_validated() {
        assert!(validate_bucket_name("gitgit-vault").is_ok());
        assert!(validate_bucket_name("").is_err());
        assert!(validate_bucket_name("UPPER").is_err());
        assert!(validate_bucket_name("has/slash").is_err());
        assert!(validate_bucket_name("..").is_err());
    }
}
