//! A self-hosted, S3-compatible object store.
//!
//! [FACT] The credential vault talks to an S3-compatible bucket
//! (`gitgit-vault`) through the `rust-s3` 0.38 client. The server that
//! bucket used to live on was minIO, which is GNU AGPLv3 — a licence the
//! product cannot carry, because it consumes the server over HTTP. This
//! module is the replacement: the same S3 surface, served by this
//! repository.
//!
//! [FACT] The *shape* does not change: the client is untouched, so the
//! credential management and the versioning logic do not have to be
//! redone. Only the server is new.
//!
//! ## What is implemented
//!
//! Exactly seven operations, because those are the seven the client
//! actually uses:
//!
//! | Operation | Request | Response |
//! | --- | --- | --- |
//! | create bucket | `PUT /{bucket}` | 200 |
//! | set bucket versioning | `PUT /{bucket}?versioning` | 200 |
//! | get bucket versioning | `GET /{bucket}?versioning` | 200 + XML |
//! | put object | `PUT /{bucket}/{key}` | 200 + `x-amz-version-id` |
//! | get object | `GET /{bucket}/{key}` | 200 + body + `x-amz-version-id` |
//! | get object by version | `GET /{bucket}/{key}?versionId=…` | 200 + body |
//! | delete object | `DELETE /{bucket}/{key}` | 204 (idempotent) |
//! | list objects | `GET /{bucket}?list-type=2&…` | 200 + `ListBucketResult` XML |
//!
//! [FACT] Addressing is **path style** (`http://host:port/{bucket}/{key}`),
//! which is what `rust-s3` uses with `with_path_style()`.
//!
//! [FACT] The `x-amz-version-id` response header is load-bearing: the
//! credential timeline in `server::vault_versioned` records it, and
//! without it every stored credential keeps an empty `version_id`.
//!
//! ## Using it from a test
//!
//! ```text
//! let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
//! let addr = listener.local_addr()?;
//! let creds = gitgit::objectstore::ObjectStoreCredentials::new("ak", "sk");
//! let app = gitgit::objectstore::router(
//!     gitgit::objectstore::ObjectStore::new("./tmp/store", creds)?,
//! );
//! let server = tokio::spawn(async move { axum::serve(listener, app).await });
//! println!("serving on http://{addr}");
//! ```
//!
//! [FACT] The example is a `text` block on purpose: a compiled doctest
//! would add a test that lands in no module bucket, which would make the
//! repository's unit-test count contract harder to re-agree. The same
//! code path is compiled and run by the tests in `src/objectstore/tests/`.
//!
//! Binding port 0 lets the operating system pick a free port, so parallel
//! test runs cannot collide.

pub mod http;
pub mod sigv4;
pub mod store;
pub mod xml;

#[cfg(test)]
// The crate denies `unwrap_used`, `expect_used` and `panic`; a test is
// exactly where those belong — the same convention the rest of the
// repository's test modules use.
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use thiserror::Error;

pub use sigv4::Sigv4Credentials as ObjectStoreCredentials;
pub use store::VersioningStatus;

/// The error type of the object store's public API.
#[derive(Debug, Error)]
pub enum ObjectStoreError {
    /// The storage layer failed.
    #[error(transparent)]
    Store(#[from] store::StoreError),
    /// The root directory could not be prepared.
    #[error("object store root {0} could not be prepared: {1}")]
    Root(String, std::io::Error),
}

/// The state the server serves.
///
/// [FACT] It is cheap to clone — the storage backend and the credentials
/// live behind an `Arc` — so a test can hand a copy to the router and
/// keep one for provisioning.
#[derive(Clone, Debug)]
pub struct ObjectStore {
    inner: Arc<ObjectStoreInner>,
}

#[derive(Debug)]
struct ObjectStoreInner {
    store: store::FileStore,
    credentials: sigv4::Sigv4Credentials,
}

impl ObjectStore {
    /// Build the server state.
    ///
    /// The root directory is created if it does not exist. Nothing else is
    /// read from the environment: everything the server needs is an
    /// argument, so a test can build one without touching the machine it
    /// runs on.
    pub fn new(
        root: impl Into<PathBuf>,
        credentials: ObjectStoreCredentials,
    ) -> Result<Self, ObjectStoreError> {
        let root = root.into();
        let store = store::FileStore::new(&root);
        // Creating the directory here turns a typo in the root path into
        // an error at construction time rather than at the first request.
        std::fs::create_dir_all(store.root())
            .map_err(|err| ObjectStoreError::Root(root.display().to_string(), err))?;
        Ok(Self {
            inner: Arc::new(ObjectStoreInner { store, credentials }),
        })
    }

    /// The root directory of the store.
    pub fn root(&self) -> &Path {
        self.inner.store.root()
    }

    /// The credentials the server verifies signatures against.
    pub fn credentials(&self) -> &sigv4::Sigv4Credentials {
        &self.inner.credentials
    }

    /// Whether a bucket exists.
    pub async fn bucket_exists(&self, bucket: &str) -> Result<bool, ObjectStoreError> {
        Ok(self.inner.store.bucket_exists(bucket).await?)
    }

    /// Create a bucket. Returns `true` when it was created, `false` when
    /// it already existed — creating a bucket twice is not an error.
    pub async fn create_bucket(&self, bucket: &str) -> Result<bool, ObjectStoreError> {
        Ok(self.inner.store.create_bucket(bucket).await?)
    }

    /// Turn versioning on or off for a bucket.
    pub async fn set_versioning(
        &self,
        bucket: &str,
        status: VersioningStatus,
    ) -> Result<(), ObjectStoreError> {
        Ok(self.inner.store.set_versioning(bucket, status).await?)
    }

    /// The versioning state of a bucket.
    pub async fn get_versioning(&self, bucket: &str) -> Result<VersioningStatus, ObjectStoreError> {
        Ok(self.inner.store.get_versioning(bucket).await?)
    }

    /// The storage backend, for callers that need the raw operations.
    pub fn store(&self) -> &store::FileStore {
        &self.inner.store
    }
}

/// The axum router that serves the object store.
///
/// [FACT] The routing is done by hand rather than with axum's path
/// patterns, for two reasons: the client sends `GET /{bucket}/` (with a
/// trailing slash) for a bucket-level listing, and the signature covers
/// the **raw** path exactly as it arrived — a router that rewrites the
/// path would rewrite the thing that is signed.
pub fn router(state: ObjectStore) -> axum::Router {
    http::router(state)
}
