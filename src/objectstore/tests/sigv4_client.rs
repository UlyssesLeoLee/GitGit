//! Layer 2 of the correctness ladder: the real client against the real
//! server.
//!
//! [FACT] The published AWS vectors prove that the *formula* is right.
//! These tests prove that the client and the server mean the same thing:
//! the `rust-s3` 0.38 client signs a request, this server verifies it,
//! and the answer is the answer the client expects.
//!
//! [FACT] Both signing variants are exercised: the `Authorization` header
//! that every ordinary request carries, and the query-string signature
//! that `presign_get` produces — the one the vault uses to read a
//! historical version.

use std::collections::HashMap;

use super::common::{
    client_bucket, client_bucket_with_keys, create_bucket, start_server, temp_root, ACCESS_KEY,
    BUCKET,
};

#[tokio::test]
async fn the_real_client_can_write_and_read_an_object() {
    let root = temp_root("client-roundtrip");
    let server = start_server(&root).await;
    let bucket = client_bucket(&server.endpoint, BUCKET);
    create_bucket(&server, BUCKET).await;

    bucket.put_object("openai", b"sk-secret").await.unwrap();
    let response = bucket.get_object("openai").await.unwrap();
    assert_eq!(response.status_code(), 200);
    assert_eq!(response.bytes(), &b"sk-secret"[..]);
    // [FACT] The version id header is what the credential timeline
    // records, so it has to be there on every read and write.
    assert!(response.headers().contains_key("x-amz-version-id"));
}

#[tokio::test]
async fn the_real_client_sees_a_missing_object_as_a_404() {
    let root = temp_root("client-missing");
    let server = start_server(&root).await;
    let bucket = client_bucket(&server.endpoint, BUCKET);
    create_bucket(&server, BUCKET).await;

    // `rust-s3` 0.38 does not check the status of a GET, so the status
    // is what tells the vault "the credential is absent".
    let response = bucket.get_object("not-there").await.unwrap();
    assert_eq!(response.status_code(), 404);
    let body = String::from_utf8_lossy(response.bytes()).to_string();
    assert!(
        body.contains("<Code>NoSuchKey</Code>"),
        "the vault classifies a 404 by its code, so the code must be exact: {body}"
    );
}

#[tokio::test]
async fn the_real_client_can_read_an_older_version_through_a_presigned_url() {
    // [FACT] This is the query-string variant of SigV4, and it is the
    // path the vault uses to read a historical version: `rust-s3` 0.38
    // cannot address a `versionId` on a plain `GetObject`, so it builds
    // a presigned GET with a custom `versionId` query parameter and
    // fires it with `reqwest`.
    let root = temp_root("client-presign");
    let server = start_server(&root).await;
    let bucket = client_bucket(&server.endpoint, BUCKET);
    create_bucket(&server, BUCKET).await;

    // The version id of the *first* write is the one that has to stay
    // readable after the second write.
    let first = bucket.put_object("openai", b"sk-v1").await.unwrap();
    let first_version_id = first.headers().get("x-amz-version-id").unwrap().to_string();
    bucket.put_object("openai", b"sk-v2").await.unwrap();
    let mut custom = HashMap::new();
    custom.insert("versionId".to_string(), first_version_id.clone());
    let url = bucket
        .presign_get("openai", 60, Some(custom))
        .await
        .unwrap();
    assert!(url.contains("X-Amz-Signature="));

    let response = reqwest::get(&url).await.unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), "sk-v1");
}

#[tokio::test]
async fn a_presigned_url_with_the_wrong_secret_is_rejected() {
    let root = temp_root("client-presign-bad");
    let server = start_server(&root).await;
    let bucket = client_bucket(&server.endpoint, BUCKET);
    create_bucket(&server, BUCKET).await;
    bucket.put_object("openai", b"sk-v1").await.unwrap();

    // A URL signed with a different secret must not open the object.
    let wrong = client_bucket_with_keys(&server.endpoint, BUCKET, ACCESS_KEY, "the-wrong-secret");
    let url = wrong.presign_get("openai", 60, None).await.unwrap();
    let response = reqwest::get(&url).await.unwrap();
    assert_eq!(response.status(), 403);
}

#[tokio::test]
async fn a_request_signed_with_the_wrong_secret_is_rejected() {
    let root = temp_root("client-wrong-secret");
    let server = start_server(&root).await;
    create_bucket(&server, BUCKET).await;

    let wrong = client_bucket_with_keys(&server.endpoint, BUCKET, ACCESS_KEY, "the-wrong-secret");
    let response = wrong.put_object("openai", b"sk-secret").await.unwrap();
    // [FACT] `rust-s3` 0.38 does not turn a non-2xx into an error on a
    // PUT — it hands the error document back as the body — so the status
    // code is what has to be checked.
    assert_eq!(
        response.status_code(),
        403,
        "a request signed with the wrong secret must not be accepted"
    );
}

#[tokio::test]
async fn the_real_client_can_list_and_paginate() {
    let root = temp_root("client-list");
    let server = start_server(&root).await;
    let bucket = client_bucket(&server.endpoint, BUCKET);
    create_bucket(&server, BUCKET).await;

    for index in 0..5 {
        bucket
            .put_object(&format!("key-{index}"), b"x")
            .await
            .unwrap();
    }

    // `Bucket::list` loops over the pages until the server stops handing
    // out a continuation token, so this proves the token round-trips.
    let pages = bucket.list(String::new(), None).await.unwrap();
    let mut keys: Vec<String> = Vec::new();
    for page in pages {
        for object in page.contents {
            keys.push(object.key);
        }
    }
    keys.sort();
    assert_eq!(keys, vec!["key-0", "key-1", "key-2", "key-3", "key-4"]);
}

#[tokio::test]
async fn the_real_client_can_delete_and_the_delete_is_idempotent() {
    let root = temp_root("client-delete");
    let server = start_server(&root).await;
    let bucket = client_bucket(&server.endpoint, BUCKET);
    create_bucket(&server, BUCKET).await;
    bucket.put_object("openai", b"sk-secret").await.unwrap();

    bucket.delete_object("openai").await.unwrap();
    let response = bucket.get_object("openai").await.unwrap();
    assert_eq!(response.status_code(), 404);

    // Deleting it again must not fail.
    bucket.delete_object("openai").await.unwrap();
}
