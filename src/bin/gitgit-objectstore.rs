//! `gitgit-objectstore` — the self-hosted, S3-compatible object store.
//!
//! The credential vault stores its credentials in an S3-compatible bucket.
//! That bucket used to live on a minIO server, which is GNU AGPLv3 — a
//! licence this product cannot carry, because it consumes the server over
//! HTTP. This binary is the replacement: the same S3 surface, served by
//! this repository, with no third-party server and no AGPL.
//!
//! # Running it
//!
//! ```text
//! GITGIT_OBJECTSTORE_SECRET_KEY=<secret> \
//!   gitgit-objectstore --root <dir> --bind <addr> --access-key <key>
//! ```
//!
//! Every setting can also come from the environment. **The command-line
//! flag wins over the environment variable, and the environment variable
//! wins over the default** — the same precedence the `gitgit` binary uses
//! for `--bind`, `--repos-dir` and `--vault-file-root`.
//!
//! | Setting | Flag | Environment variable | Default |
//! | --- | --- | --- | --- |
//! | data directory | `--root <dir>` | `GITGIT_OBJECTSTORE_ROOT` | `./.gitgit/objectstore` |
//! | listen address | `--bind <addr>` | `GITGIT_OBJECTSTORE_BIND` | `127.0.0.1:9000` |
//! | access key | `--access-key <key>` | `GITGIT_OBJECTSTORE_ACCESS_KEY` | *(none — required)* |
//! | secret key | *(none — environment only)* | `GITGIT_OBJECTSTORE_SECRET_KEY` | *(none — required)* |
//!
//! # Why the secret key is not a flag
//!
//! [FACT] A long-lived secret in the process arguments is visible to other
//! users through `ps`, and it is kept in shell history. This server guards
//! the credential store, so its secret gets the same treatment the AI
//! provider key already gets in `src/ai/mod.rs` (`GITGIT_AI_API_KEY`): the
//! secret is **environment only**.
//!
//! [FACT] The **access key** may still be an argument, because it is an
//! identifier and not a secret: it names the client, it does not
//! authenticate it, and an operator who chose it already knows it. The rule
//! is "a secret never goes in the process arguments; an identifier may".
//!
//! [FACT] The access key and the secret key have **no default**: a server
//! with a hard-coded credential is a server with a published credential.
//! The server refuses to start without them.
//!
//! # Bucket provisioning
//!
//! [FACT] The minIO fixture used `mc mb --ignore-existing` followed by
//! `mc version enable`, both in a second container. This binary does the
//! same thing itself, so nothing but this binary has to be installed:
//!
//! ```text
//! GITGIT_OBJECTSTORE_SECRET_KEY=<secret> \
//!   gitgit-objectstore --root <dir> --bind <addr> --access-key <key> \
//!     --bootstrap-bucket gitgit-vault
//! ```
//!
//! The bucket is created if it does not exist, and versioning is enabled
//! on it. Both steps are idempotent, so the flag can be passed on every
//! start.
//!
//! # Pointing the credential vault at this server
//!
//! The vault's configuration (`MinioVaultConfig` in `server::vault`)
//! needs six values. This server answers to all of them:
//!
//! | Vault setting | Value for this server |
//! | --- | --- |
//! | `endpoint` | `http://127.0.0.1:9000` (the `--bind` address) |
//! | `bucket` | `gitgit-vault` (or whatever `--bootstrap-bucket` created) |
//! | `region` | `us-east-1` — the server accepts **any** region, because the signing key is derived from the region in the client's credential scope |
//! | `access_key` | the `--access-key` value |
//! | `secret_key` | the `GITGIT_OBJECTSTORE_SECRET_KEY` value |
//! | `key_prefix` | whatever the caller wants, e.g. `gitgit-vault/` |
//!
//! # The three bootstrap operations, exactly
//!
//! [FACT] Addressing is path style: `http://host:port/{bucket}/{key}`.
//! Every request is signed with SigV4; the `Authorization` header carries
//! the signature.
//!
//! 1. **Create the bucket** — `PUT /{bucket}`
//!    * request: `PUT /gitgit-vault` with an empty body;
//!    * response: `200 OK`, empty body. Creating a bucket that already
//!      exists is also `200` — it is not an error.
//! 2. **Enable bucket versioning** — `PUT /{bucket}?versioning`
//!    * request: `PUT /gitgit-vault?versioning` with an empty body;
//!    * response: `200 OK`, empty body. The bucket must exist, otherwise
//!      `404` with `<Code>NoSuchBucket</Code>`.
//! 3. **Put one object** — `PUT /{bucket}/{key}`
//!    * request: `PUT /gitgit-vault/openai` with the object's bytes;
//!    * response: `200 OK`, an `ETag` header, and the **`x-amz-version-id`**
//!      header — the version id the credential timeline records. Without it
//!      every stored credential keeps an empty `version_id`.
//!
//! [FACT] Reading an object back is `GET /{bucket}/{key}` (200 with the
//! bytes and the same `x-amz-version-id` header, 404 with
//! `<Code>NoSuchKey</Code>` when it is not there), reading a historical
//! version is `GET /{bucket}/{key}?versionId=…` (a presigned URL), and
//! deleting is `DELETE /{bucket}/{key}` (204, and deleting a key that is
//! not there is also 204).

use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
use gitgit::objectstore::store::VersioningStatus;
use gitgit::objectstore::{ObjectStore, ObjectStoreCredentials};

/// The environment variable that relocates the data directory.
const ROOT_ENV: &str = "GITGIT_OBJECTSTORE_ROOT";
/// The environment variable that relocates the listen address.
const BIND_ENV: &str = "GITGIT_OBJECTSTORE_BIND";
/// The environment variable that carries the access key.
const ACCESS_KEY_ENV: &str = "GITGIT_OBJECTSTORE_ACCESS_KEY";
/// The environment variable that carries the secret key.
const SECRET_KEY_ENV: &str = "GITGIT_OBJECTSTORE_SECRET_KEY";

/// Where the objects live when nothing says otherwise.
const DEFAULT_ROOT: &str = "./.gitgit/objectstore";
/// Where the server listens when nothing says otherwise.
const DEFAULT_BIND: &str = "127.0.0.1:9000";

/// The self-hosted S3-compatible object store.
#[derive(Debug, Parser)]
#[command(
    name = "gitgit-objectstore",
    version,
    about = "Self-hosted, S3-compatible object store for the gitgit credential vault"
)]
struct Cli {
    /// The directory the objects are stored in.
    #[arg(long, value_name = "DIR")]
    root: Option<PathBuf>,

    /// The address to listen on, e.g. 127.0.0.1:9000.
    #[arg(long, value_name = "ADDR")]
    bind: Option<String>,

    /// The S3 access key the server verifies signatures against.
    ///
    /// [FACT] An access key is an identifier, not a secret: it names the
    /// client and does not authenticate it, so it may be an argument. The
    /// secret key may not — see the module documentation.
    #[arg(long, value_name = "KEY")]
    access_key: Option<String>,

    /// Create this bucket if it is missing and enable versioning on it.
    #[arg(long, value_name = "BUCKET")]
    bootstrap_bucket: Option<String>,
}

impl Cli {
    /// The data directory: flag, then environment, then the default.
    fn root(&self) -> PathBuf {
        if let Some(root) = &self.root {
            return root.clone();
        }
        match env_value(ROOT_ENV) {
            Some(value) => PathBuf::from(value),
            None => PathBuf::from(DEFAULT_ROOT),
        }
    }

    /// The listen address: flag, then environment, then the default.
    fn bind(&self) -> String {
        if let Some(bind) = &self.bind {
            return bind.clone();
        }
        env_value(BIND_ENV).unwrap_or_else(|| DEFAULT_BIND.to_string())
    }

    /// The access key: flag, then environment. There is no default.
    ///
    /// [FACT] The access key is an identifier, so it may be an argument.
    /// It still may not be empty: an empty access key is a credential
    /// anybody can name.
    fn access_key(&self) -> anyhow::Result<String> {
        let value = self
            .access_key
            .clone()
            .or_else(|| env_value(ACCESS_KEY_ENV))
            .ok_or_else(|| {
                anyhow::anyhow!("no access key: pass --access-key or set {ACCESS_KEY_ENV}")
            })?;
        if value.trim().is_empty() {
            return Err(anyhow::anyhow!(
                "the access key is empty: pass --access-key or set {ACCESS_KEY_ENV}"
            ));
        }
        Ok(value)
    }

    /// The secret key: **the environment only**. There is no default.
    ///
    /// [FACT] A secret in the process arguments is visible in `ps` output
    /// and lands in shell history, so there is deliberately no
    /// `--secret-key`.
    fn secret_key(&self) -> anyhow::Result<String> {
        resolve_secret(env_value(SECRET_KEY_ENV))
    }
}

/// Resolve the secret key from the environment value.
///
/// [FACT] Split out so the rule can be tested without touching the
/// process environment: the secret comes from the environment, and there
/// is no fallback when it is not there.
fn resolve_secret(value: Option<String>) -> anyhow::Result<String> {
    value.ok_or_else(|| {
        anyhow::anyhow!("no secret key: set {SECRET_KEY_ENV} (there is no --secret-key flag)")
    })
}

/// Read an environment variable, treating an empty value as "not set".
fn env_value(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(value) if !value.trim().is_empty() => Some(value),
        _ => None,
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init()
        .ok();

    let cli = Cli::parse();
    let root = cli.root();
    let bind = cli.bind();
    let credentials = ObjectStoreCredentials::new(cli.access_key()?, cli.secret_key()?);

    let store = ObjectStore::new(&root, credentials)
        .with_context(|| format!("the object store root {} is unusable", root.display()))?;

    if let Some(bucket) = &cli.bootstrap_bucket {
        let created = store
            .create_bucket(bucket)
            .await
            .with_context(|| format!("the bucket {bucket} could not be created"))?;
        store
            .set_versioning(bucket, VersioningStatus::Enabled)
            .await
            .with_context(|| format!("versioning could not be enabled on {bucket}"))?;
        tracing::info!(
            bucket = %bucket,
            created,
            "bucket is ready and versioning is enabled"
        );
    }

    let address: SocketAddr = bind
        .parse()
        .with_context(|| format!("{bind} is not a valid listen address"))?;
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .with_context(|| format!("failed to bind to {address}"))?;

    tracing::info!(
        %address,
        root = %root.display(),
        "object store is listening"
    );

    let app = gitgit::objectstore::router(store);
    axum::serve(listener, app)
        .await
        .context("the object store stopped serving")?;
    Ok(())
}

/// The credential surface of this binary.
///
/// [FACT] The tests live in a named module rather than in a bare
/// `mod tests`, because a binary target's test paths have no crate
/// prefix: a bare `mod tests` would land in the repository's unit-test
/// count contract as one bucket *per test* (`tests::<fn>`), and this way
/// they land in one bucket (`credentials::tests`).
#[cfg(test)]
mod credentials {
    #[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    mod tests {
        use super::super::*;

        #[test]
        fn the_secret_key_cannot_be_supplied_as_an_argument() {
            // [FACT] A long-lived secret in the process arguments is visible in
            // `ps` output and lands in shell history. If this ever moves to a
            // CLI flag, this test fails — the same rule the AI provider key
            // holds in `src/ai/mod.rs`.
            let parsed = Cli::try_parse_from(["gitgit-objectstore", "--secret-key", "hunter2"]);
            assert!(
                parsed.is_err(),
                "--secret-key must not be accepted: the secret is visible in `ps`"
            );
        }

        #[test]
        fn the_access_key_may_be_an_argument() {
            // [FACT] The access key is an identifier, not a secret: it names
            // the client and does not authenticate it, so the rule ("a secret
            // never goes in the process arguments; an identifier may") lets it
            // be a flag.
            let cli =
                Cli::try_parse_from(["gitgit-objectstore", "--access-key", "gitgit"]).unwrap();
            assert_eq!(cli.access_key.as_deref(), Some("gitgit"));
            assert_eq!(cli.access_key().unwrap(), "gitgit");
        }

        #[test]
        fn the_secret_key_comes_from_the_environment_and_has_no_fallback() {
            assert_eq!(
                resolve_secret(Some("s3cret".to_string())).unwrap(),
                "s3cret"
            );
            // There is no default and no fallback: without the environment
            // variable the server refuses to start.
            let error = resolve_secret(None).expect_err("there must be no fallback secret");
            assert!(
                error.to_string().contains(SECRET_KEY_ENV),
                "the error must name the environment variable: {error}"
            );
        }

        #[test]
        fn an_empty_access_key_is_refused() {
            // An empty access key is a credential anybody can name, so the
            // server must not start with one.
            let cli = Cli::try_parse_from(["gitgit-objectstore", "--access-key", "   "]).unwrap();
            assert!(
                cli.access_key().is_err(),
                "an empty access key must be refused"
            );
        }
    }
}
