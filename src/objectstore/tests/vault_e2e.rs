//! Layer 3 of the correctness ladder: the credential vault, unchanged,
//! pointed at this repository's own object store.
//!
//! [FACT] The vault (`server::vault` / `server::vault_versioned`) is the
//! consumer that matters: it was written against real S3 semantics, and
//! it is the thing that has to keep working when the server moves from
//! minIO to this crate.
//!
//! [FACT] These tests are the in-process twin of the vault's own
//! `#[ignore]`d e2e tests: same backend, same operations, but the server
//! is started here, so the S3 path is covered by an ordinary `cargo test`
//! run instead of only by a hand-run fixture.

use crate::server::vault::{MinioVault, MinioVaultConfig, Vault};
use crate::server::vault_versioned::VersionedVault;

use super::common::{start_server_with_bucket, temp_root, ACCESS_KEY, BUCKET, SECRET_KEY};

/// A vault configuration pointed at the running server.
fn vault_config(endpoint: &str, key_prefix: &str) -> MinioVaultConfig {
    MinioVaultConfig {
        endpoint: endpoint.to_string(),
        bucket: BUCKET.to_string(),
        access_key: ACCESS_KEY.to_string(),
        secret_key: SECRET_KEY.to_string(),
        region: "us-east-1".to_string(),
        key_prefix: key_prefix.to_string(),
    }
}

#[tokio::test]
async fn the_vault_reads_and_writes_through_the_object_store() {
    let root = temp_root("vault-roundtrip");
    let server = start_server_with_bucket(&root).await;
    let vault = MinioVault::connect(&vault_config(&server.endpoint, "ut-roundtrip/")).unwrap();

    vault.set("openai", "sk-ut-secret").await.unwrap();
    assert_eq!(
        vault.get("openai").await.unwrap().as_deref(),
        Some("sk-ut-secret")
    );

    // A key that was never written is absent, not an error.
    assert_eq!(vault.get("definitely-not-there").await.unwrap(), None);

    // The listing carries the key.
    let keys = vault.list().await.unwrap();
    assert!(keys.iter().any(|k| k == "openai"), "list was {keys:?}");

    // A rotation replaces the value.
    vault.rotate("openai").await.unwrap();
    let rotated = vault.get("openai").await.unwrap();
    assert!(rotated.is_some());
    assert_ne!(rotated.as_deref(), Some("sk-ut-secret"));
    assert!(rotated.unwrap().starts_with("rotated-"));

    // A delete removes the value, and deleting it again is not an error.
    vault.delete("openai").await.unwrap();
    assert_eq!(vault.get("openai").await.unwrap(), None);
    vault.delete("openai").await.unwrap();
}

#[tokio::test]
async fn the_versioned_vault_records_a_distinct_version_id_per_write() {
    let root = temp_root("vault-versioned");
    let server = start_server_with_bucket(&root).await;
    let vault = MinioVault::connect(&vault_config(&server.endpoint, "ut-versioned/")).unwrap();

    assert_eq!(
        vault.set_with_version("openai", "sk-ut-v1").await.unwrap(),
        1
    );
    assert_eq!(
        vault.set_with_version("openai", "sk-ut-v2").await.unwrap(),
        2
    );
    assert_eq!(
        vault.set_with_version("openai", "sk-ut-v3").await.unwrap(),
        3
    );

    let versions = vault.list_versions("openai").await.unwrap();
    assert_eq!(versions.len(), 3);
    let ids: Vec<&str> = versions
        .iter()
        .map(|v| v.version_id.as_deref().unwrap_or(""))
        .collect();
    // [FACT] The version ids have to be there: without the
    // `x-amz-version-id` response header the credential timeline keeps
    // an empty `version_id` and no historical read is possible.
    for id in &ids {
        assert!(!id.is_empty(), "a version id is missing: {ids:?}");
    }
    assert_ne!(ids[0], ids[1]);
    assert_ne!(ids[1], ids[2]);
    assert_ne!(ids[0], ids[2]);
}

#[tokio::test]
async fn the_versioned_vault_reads_an_older_version_back() {
    let root = temp_root("vault-versioned-read");
    let server = start_server_with_bucket(&root).await;
    let vault = MinioVault::connect(&vault_config(&server.endpoint, "ut-versioned-read/")).unwrap();

    vault.set_with_version("openai", "sk-ut-v1").await.unwrap();
    vault.set_with_version("openai", "sk-ut-v2").await.unwrap();
    vault.set_with_version("openai", "sk-ut-v3").await.unwrap();

    assert_eq!(
        vault.get_at_version("openai", 1).await.unwrap().as_deref(),
        Some("sk-ut-v1")
    );
    assert_eq!(
        vault.get_at_version("openai", 2).await.unwrap().as_deref(),
        Some("sk-ut-v2")
    );
    assert_eq!(
        vault.get_at_version("openai", 3).await.unwrap().as_deref(),
        Some("sk-ut-v3")
    );

    // A version the timeline never saw is absent, not an error.
    assert_eq!(vault.get_at_version("nonexistent", 1).await.unwrap(), None);
}

#[tokio::test]
async fn the_historical_read_goes_through_the_version_id_and_not_the_attachment_slot() {
    // [FACT] The vault has a fallback: if the `versionId` read fails it
    // reads the attachment slot it wrote itself. Deleting the attachment
    // objects first therefore proves which path was taken — and the
    // answer has to be the `versionId` path, because that is the path
    // that works against a server that keeps no local copies.
    let root = temp_root("vault-versionid-path");
    let server = start_server_with_bucket(&root).await;
    let prefix = "ut-versionid-path/";
    let vault = MinioVault::connect(&vault_config(&server.endpoint, prefix)).unwrap();

    vault.set_with_version("openai", "sk-ut-v1").await.unwrap();
    vault.set_with_version("openai", "sk-ut-v2").await.unwrap();

    // Remove the attachment slots the vault wrote for itself.
    let bucket = super::common::client_bucket(&server.endpoint, BUCKET);
    bucket
        .delete_object(&format!("{prefix}openai.v1.bin"))
        .await
        .unwrap();
    bucket
        .delete_object(&format!("{prefix}openai.v2.bin"))
        .await
        .unwrap();

    let first = vault.get_at_version("openai", 1).await.unwrap();
    assert_eq!(
        first.as_deref(),
        Some("sk-ut-v1"),
        "the historical read must go through the version id"
    );
}
