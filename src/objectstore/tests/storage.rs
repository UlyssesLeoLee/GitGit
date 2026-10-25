//! Layer 1 of the correctness ladder: the storage layer.
//!
//! [FACT] These tests talk to the filesystem backend directly — no HTTP,
//! no signatures — so a failure here is unambiguously a storage bug.

use std::path::PathBuf;

use crate::objectstore::store::{FileStore, VersioningStatus};

use super::common::temp_root;

fn store(label: &str) -> (FileStore, PathBuf) {
    let root = temp_root(label);
    (FileStore::new(&root), root)
}

#[tokio::test]
async fn a_bucket_starts_out_missing_and_can_be_created_once() {
    let (store, _root) = store("bucket-create");
    assert!(!store.bucket_exists("gitgit-vault").await.unwrap());
    assert!(store.create_bucket("gitgit-vault").await.unwrap());
    assert!(store.bucket_exists("gitgit-vault").await.unwrap());
    // Creating it again is not an error — `mc mb --ignore-existing`.
    assert!(!store.create_bucket("gitgit-vault").await.unwrap());
}

#[tokio::test]
async fn versioning_reports_enabled_because_the_store_always_versions() {
    let (store, _root) = store("versioning");
    store.create_bucket("gitgit-vault").await.unwrap();
    // [FACT] The store always versions: every write creates a version
    // directory and the response carries an `x-amz-version-id`, because that
    // header is load-bearing for the credential timeline in
    // `server::vault_versioned`.
    //
    // [INFERENCE] So "Disabled" would be a false report about this store's
    // own behaviour, and a client that believed it would be wrong in a way
    // nothing would report. A bucket nobody has explicitly versioned still
    // reports Enabled, because that is what the store does.
    assert_eq!(
        store.get_versioning("gitgit-vault").await.unwrap(),
        VersioningStatus::Enabled
    );
    store
        .set_versioning("gitgit-vault", VersioningStatus::Enabled)
        .await
        .unwrap();
    assert_eq!(
        store.get_versioning("gitgit-vault").await.unwrap(),
        VersioningStatus::Enabled
    );
}

#[tokio::test]
async fn versioning_a_missing_bucket_is_an_error() {
    let (store, _root) = store("versioning-missing");
    assert!(store
        .set_versioning("gitgit-vault", VersioningStatus::Enabled)
        .await
        .is_err());
}

#[tokio::test]
async fn versioning_cannot_be_turned_off() {
    let (store, _root) = store("versioning-cannot-disable");
    store.create_bucket("gitgit-vault").await.unwrap();
    // [FACT] The store always versions, so a request to stop versioning is
    // refused with a clear error rather than accepted and then reported
    // back as if it had been honoured.
    let err = store
        .set_versioning("gitgit-vault", VersioningStatus::Disabled)
        .await
        .expect_err("versioning cannot be turned off in this store");
    assert!(
        err.to_string().contains("always versions"),
        "the error must say why the request was refused, got: {err}"
    );
    // And the bucket still reports the truth.
    assert_eq!(
        store.get_versioning("gitgit-vault").await.unwrap(),
        VersioningStatus::Enabled
    );
}

#[tokio::test]
async fn a_put_is_readable_and_reports_its_size_and_etag() {
    let (store, _root) = store("put-get");
    store.create_bucket("gitgit-vault").await.unwrap();
    let version_id = store
        .put("gitgit-vault", "openai", b"sk-test-123", Some("text/plain"))
        .await
        .unwrap();
    assert!(!version_id.is_empty());
    let object = store
        .get_current("gitgit-vault", "openai")
        .await
        .unwrap()
        .expect("the object must be there");
    assert_eq!(object.bytes, b"sk-test-123");
    assert_eq!(object.meta.size, 11);
    assert_eq!(object.meta.key, "openai");
    assert_eq!(object.meta.content_type.as_deref(), Some("text/plain"));
    assert_eq!(object.meta.etag.len(), 64);
    assert_eq!(object.version_id, version_id);
}

#[tokio::test]
async fn two_puts_produce_two_version_ids_and_the_second_is_current() {
    let (store, _root) = store("two-versions");
    store.create_bucket("gitgit-vault").await.unwrap();
    let first = store
        .put("gitgit-vault", "openai", b"sk-v1", None)
        .await
        .unwrap();
    let second = store
        .put("gitgit-vault", "openai", b"sk-v2", None)
        .await
        .unwrap();
    assert_ne!(first, second);

    let current = store
        .get_current("gitgit-vault", "openai")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(current.bytes, b"sk-v2");
    assert_eq!(current.version_id, second);
}

#[tokio::test]
async fn an_older_version_stays_readable_through_its_version_id() {
    let (store, _root) = store("read-old-version");
    store.create_bucket("gitgit-vault").await.unwrap();
    let first = store
        .put("gitgit-vault", "openai", b"sk-v1", None)
        .await
        .unwrap();
    store
        .put("gitgit-vault", "openai", b"sk-v2", None)
        .await
        .unwrap();

    let old = store
        .get_version("gitgit-vault", "openai", &first)
        .await
        .unwrap()
        .expect("the older version must still be there");
    assert_eq!(old.bytes, b"sk-v1");
    assert_eq!(old.version_id, first);
}

#[tokio::test]
async fn deleting_removes_the_current_version_but_keeps_the_older_one() {
    let (store, _root) = store("delete");
    store.create_bucket("gitgit-vault").await.unwrap();
    let first = store
        .put("gitgit-vault", "openai", b"sk-v1", None)
        .await
        .unwrap();
    let second = store
        .put("gitgit-vault", "openai", b"sk-v2", None)
        .await
        .unwrap();

    store.delete("gitgit-vault", "openai").await.unwrap();

    // The key is gone as far as the current view is concerned ...
    assert!(store
        .get_current("gitgit-vault", "openai")
        .await
        .unwrap()
        .is_none());
    // ... and so is the version that was deleted ...
    assert!(store
        .get_version("gitgit-vault", "openai", &second)
        .await
        .unwrap()
        .is_none());
    // ... but the older version is still readable.
    let old = store
        .get_version("gitgit-vault", "openai", &first)
        .await
        .unwrap()
        .expect("the older version must survive a delete");
    assert_eq!(old.bytes, b"sk-v1");
}

#[tokio::test]
async fn deleting_a_key_that_is_not_there_is_not_an_error() {
    let (store, _root) = store("delete-missing");
    store.create_bucket("gitgit-vault").await.unwrap();
    store.delete("gitgit-vault", "not-there").await.unwrap();
}

#[tokio::test]
async fn a_write_brings_a_deleted_key_back() {
    let (store, _root) = store("resurrect");
    store.create_bucket("gitgit-vault").await.unwrap();
    store
        .put("gitgit-vault", "openai", b"sk-v1", None)
        .await
        .unwrap();
    store.delete("gitgit-vault", "openai").await.unwrap();
    assert!(store
        .get_current("gitgit-vault", "openai")
        .await
        .unwrap()
        .is_none());

    store
        .put("gitgit-vault", "openai", b"sk-v2", None)
        .await
        .unwrap();
    let object = store
        .get_current("gitgit-vault", "openai")
        .await
        .unwrap()
        .expect("the key must be back");
    assert_eq!(object.bytes, b"sk-v2");
}

#[tokio::test]
async fn a_listing_reports_the_current_version_of_every_key() {
    let (store, _root) = store("list");
    store.create_bucket("gitgit-vault").await.unwrap();
    store
        .put("gitgit-vault", "alpha", b"1", None)
        .await
        .unwrap();
    store.put("gitgit-vault", "beta", b"2", None).await.unwrap();
    store
        .put("gitgit-vault", "gamma", b"3", None)
        .await
        .unwrap();

    let page = store.list("gitgit-vault", "", None, 1000).await.unwrap();
    let keys: Vec<&str> = page.entries.iter().map(|e| e.key.as_str()).collect();
    assert_eq!(keys, vec!["alpha", "beta", "gamma"]);
    assert!(!page.is_truncated);
    assert!(page.next_continuation_token.is_none());
}

#[tokio::test]
async fn a_listing_filters_by_prefix() {
    let (store, _root) = store("list-prefix");
    store.create_bucket("gitgit-vault").await.unwrap();
    store
        .put("gitgit-vault", "a/one", b"1", None)
        .await
        .unwrap();
    store
        .put("gitgit-vault", "a/two", b"2", None)
        .await
        .unwrap();
    store
        .put("gitgit-vault", "b/three", b"3", None)
        .await
        .unwrap();

    let page = store.list("gitgit-vault", "a/", None, 1000).await.unwrap();
    let keys: Vec<&str> = page.entries.iter().map(|e| e.key.as_str()).collect();
    assert_eq!(keys, vec!["a/one", "a/two"]);
}

#[tokio::test]
async fn a_listing_pages_and_the_token_round_trips() {
    let (store, _root) = store("list-paging");
    store.create_bucket("gitgit-vault").await.unwrap();
    for index in 0..5 {
        store
            .put("gitgit-vault", &format!("key-{index}"), b"x", None)
            .await
            .unwrap();
    }

    let mut seen: Vec<String> = Vec::new();
    let mut token: Option<String> = None;
    loop {
        let page = store
            .list("gitgit-vault", "", token.as_deref(), 2)
            .await
            .unwrap();
        for entry in &page.entries {
            seen.push(entry.key.clone());
        }
        if !page.is_truncated {
            assert!(page.next_continuation_token.is_none());
            break;
        }
        token = page.next_continuation_token;
        assert!(token.is_some());
    }
    assert_eq!(seen, vec!["key-0", "key-1", "key-2", "key-3", "key-4"]);
}

#[tokio::test]
async fn a_deleted_key_leaves_the_listing() {
    let (store, _root) = store("list-after-delete");
    store.create_bucket("gitgit-vault").await.unwrap();
    store
        .put("gitgit-vault", "alpha", b"1", None)
        .await
        .unwrap();
    store.put("gitgit-vault", "beta", b"2", None).await.unwrap();
    store.delete("gitgit-vault", "alpha").await.unwrap();

    let page = store.list("gitgit-vault", "", None, 1000).await.unwrap();
    let keys: Vec<&str> = page.entries.iter().map(|e| e.key.as_str()).collect();
    assert_eq!(keys, vec!["beta"]);
}

#[tokio::test]
async fn awkward_key_names_survive_a_round_trip() {
    let (store, _root) = store("awkward-keys");
    store.create_bucket("gitgit-vault").await.unwrap();
    let keys = [
        "openai/api_key",
        "key with spaces",
        "鍵に日本語",
        "../escape",
        "dots/../in/the/middle",
    ];
    for key in keys {
        store
            .put("gitgit-vault", key, key.as_bytes(), None)
            .await
            .unwrap();
    }
    for key in keys {
        let object = store
            .get_current("gitgit-vault", key)
            .await
            .unwrap()
            .expect("the key must be there");
        assert_eq!(object.bytes, key.as_bytes());
        assert_eq!(object.meta.key, key);
    }

    let page = store.list("gitgit-vault", "", None, 1000).await.unwrap();
    let mut listed: Vec<&str> = page.entries.iter().map(|e| e.key.as_str()).collect();
    listed.sort();
    let mut expected: Vec<&str> = keys.to_vec();
    expected.sort();
    assert_eq!(listed, expected);
}

#[tokio::test]
async fn an_awkward_key_never_escapes_the_bucket_directory() {
    let (store, root) = store("no-escape");
    store.create_bucket("gitgit-vault").await.unwrap();
    store
        .put("gitgit-vault", "../../escape", b"x", None)
        .await
        .unwrap();
    // The key is hashed into the file name, so nothing lands outside the
    // bucket directory.
    let bucket_dir = root.join("gitgit-vault");
    let escaped = root.join("escape.bin");
    assert!(!escaped.exists());
    assert!(bucket_dir.is_dir());
}

#[tokio::test]
async fn putting_into_a_bucket_that_does_not_exist_is_an_error() {
    let (store, _root) = store("put-missing-bucket");
    assert!(store
        .put("gitgit-vault", "openai", b"x", None)
        .await
        .is_err());
}
