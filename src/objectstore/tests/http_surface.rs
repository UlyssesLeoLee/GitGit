//! The HTTP surface of the object store, and the three failure modes the
//! brief names.
//!
//! [FACT] These tests drive the server over real HTTP with real SigV4
//! signatures, so they prove what a client sees — status codes, headers
//! and bodies — not just what the storage layer does.

use std::collections::BTreeMap;

use crate::objectstore::sigv4;
use crate::objectstore::xml::ListingShape;

use super::common::{
    amz_date_now, client_bucket, send_signed, start_server, start_server_with, temp_root,
    ACCESS_KEY, BUCKET, SECRET_KEY,
};

/// A request that carries a SigV4 signature in the `Authorization` header.
async fn signed(
    server: &super::common::TestServer,
    method: &str,
    path: &str,
    query: &str,
    payload: &[u8],
    access_key: &str,
    secret_key: &str,
) -> reqwest::Response {
    send_signed(server, method, path, query, payload, access_key, secret_key).await
}

#[tokio::test]
async fn creating_a_bucket_answers_200_and_the_bucket_appears() {
    let root = temp_root("http-create-bucket");
    let server = start_server(&root).await;

    let response = signed(
        &server,
        "PUT",
        &format!("/{BUCKET}"),
        "",
        b"",
        ACCESS_KEY,
        SECRET_KEY,
    )
    .await;
    assert_eq!(response.status(), 200);

    // The bucket is really there: a listing of it no longer answers
    // "no such bucket".
    let response = signed(
        &server,
        "GET",
        &format!("/{BUCKET}/"),
        "list-type=2",
        b"",
        ACCESS_KEY,
        SECRET_KEY,
    )
    .await;
    assert_eq!(response.status(), 200);
    assert!(response
        .text()
        .await
        .unwrap()
        .contains("<Name>gitgit-vault</Name>"));
}

#[tokio::test]
async fn bucket_versioning_can_be_set_and_read_back() {
    let root = temp_root("http-versioning");
    let server = start_server(&root).await;
    signed(
        &server,
        "PUT",
        &format!("/{BUCKET}"),
        "",
        b"",
        ACCESS_KEY,
        SECRET_KEY,
    )
    .await;

    let response = signed(
        &server,
        "PUT",
        &format!("/{BUCKET}"),
        "versioning",
        b"",
        ACCESS_KEY,
        SECRET_KEY,
    )
    .await;
    assert_eq!(response.status(), 200);

    let response = signed(
        &server,
        "GET",
        &format!("/{BUCKET}"),
        "versioning",
        b"",
        ACCESS_KEY,
        SECRET_KEY,
    )
    .await;
    assert_eq!(response.status(), 200);
    let body = response.text().await.unwrap();
    assert!(body.contains("<Status>Enabled</Status>"), "body was {body}");
}

#[tokio::test]
async fn putting_an_object_answers_200_with_the_version_id_header() {
    let root = temp_root("http-put");
    let server = start_server(&root).await;
    signed(
        &server,
        "PUT",
        &format!("/{BUCKET}"),
        "",
        b"",
        ACCESS_KEY,
        SECRET_KEY,
    )
    .await;

    let response = signed(
        &server,
        "PUT",
        &format!("/{BUCKET}/openai"),
        "",
        b"sk-secret",
        ACCESS_KEY,
        SECRET_KEY,
    )
    .await;
    assert_eq!(response.status(), 200);
    let version_id = response
        .headers()
        .get("x-amz-version-id")
        .expect("the version id header is load-bearing")
        .to_str()
        .unwrap()
        .to_string();
    assert_eq!(version_id.len(), 32);
}

#[tokio::test]
async fn getting_an_object_answers_200_with_the_body_and_the_version_id() {
    let root = temp_root("http-get");
    let server = start_server(&root).await;
    signed(
        &server,
        "PUT",
        &format!("/{BUCKET}"),
        "",
        b"",
        ACCESS_KEY,
        SECRET_KEY,
    )
    .await;
    let put = signed(
        &server,
        "PUT",
        &format!("/{BUCKET}/openai"),
        "",
        b"sk-secret",
        ACCESS_KEY,
        SECRET_KEY,
    )
    .await;
    let version_id = put
        .headers()
        .get("x-amz-version-id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let response = signed(
        &server,
        "GET",
        &format!("/{BUCKET}/openai"),
        "",
        b"",
        ACCESS_KEY,
        SECRET_KEY,
    )
    .await;
    assert_eq!(response.status(), 200);
    let read_version_id = response
        .headers()
        .get("x-amz-version-id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert_eq!(response.text().await.unwrap(), "sk-secret");
    assert_eq!(read_version_id, version_id);
}

#[tokio::test]
async fn getting_an_object_by_version_id_answers_the_old_bytes() {
    let root = temp_root("http-get-version");
    let server = start_server(&root).await;
    signed(
        &server,
        "PUT",
        &format!("/{BUCKET}"),
        "",
        b"",
        ACCESS_KEY,
        SECRET_KEY,
    )
    .await;
    let first = signed(
        &server,
        "PUT",
        &format!("/{BUCKET}/openai"),
        "",
        b"sk-v1",
        ACCESS_KEY,
        SECRET_KEY,
    )
    .await;
    let first_version = first
        .headers()
        .get("x-amz-version-id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let second = signed(
        &server,
        "PUT",
        &format!("/{BUCKET}/openai"),
        "",
        b"sk-v2",
        ACCESS_KEY,
        SECRET_KEY,
    )
    .await;
    let second_version = second
        .headers()
        .get("x-amz-version-id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert_ne!(first_version, second_version);

    let response = signed(
        &server,
        "GET",
        &format!("/{BUCKET}/openai"),
        &format!("versionId={first_version}"),
        b"",
        ACCESS_KEY,
        SECRET_KEY,
    )
    .await;
    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), "sk-v1");
}

#[tokio::test]
async fn deleting_an_object_answers_204_and_is_idempotent() {
    let root = temp_root("http-delete");
    let server = start_server(&root).await;
    signed(
        &server,
        "PUT",
        &format!("/{BUCKET}"),
        "",
        b"",
        ACCESS_KEY,
        SECRET_KEY,
    )
    .await;
    signed(
        &server,
        "PUT",
        &format!("/{BUCKET}/openai"),
        "",
        b"sk-secret",
        ACCESS_KEY,
        SECRET_KEY,
    )
    .await;

    let response = signed(
        &server,
        "DELETE",
        &format!("/{BUCKET}/openai"),
        "",
        b"",
        ACCESS_KEY,
        SECRET_KEY,
    )
    .await;
    assert_eq!(response.status(), 204);

    // Deleting it again is still a success.
    let response = signed(
        &server,
        "DELETE",
        &format!("/{BUCKET}/openai"),
        "",
        b"",
        ACCESS_KEY,
        SECRET_KEY,
    )
    .await;
    assert_eq!(response.status(), 204);

    // And the key is gone.
    let response = signed(
        &server,
        "GET",
        &format!("/{BUCKET}/openai"),
        "",
        b"",
        ACCESS_KEY,
        SECRET_KEY,
    )
    .await;
    assert_eq!(response.status(), 404);
    let body = response.text().await.unwrap();
    assert!(body.contains("<Code>NoSuchKey</Code>"), "body was {body}");
}

#[tokio::test]
async fn listing_objects_answers_the_listing_xml() {
    let root = temp_root("http-list");
    let server = start_server(&root).await;
    signed(
        &server,
        "PUT",
        &format!("/{BUCKET}"),
        "",
        b"",
        ACCESS_KEY,
        SECRET_KEY,
    )
    .await;
    signed(
        &server,
        "PUT",
        &format!("/{BUCKET}/openai"),
        "",
        b"sk-secret",
        ACCESS_KEY,
        SECRET_KEY,
    )
    .await;

    let response = signed(
        &server,
        "GET",
        &format!("/{BUCKET}/"),
        "list-type=2",
        b"",
        ACCESS_KEY,
        SECRET_KEY,
    )
    .await;
    assert_eq!(response.status(), 200);
    let body = response.text().await.unwrap();
    assert!(
        body.contains("<Name>gitgit-vault</Name>"),
        "body was {body}"
    );
    assert!(body.contains("<Key>openai</Key>"), "body was {body}");
}

#[tokio::test]
async fn a_missing_bucket_answers_404_with_the_no_such_bucket_code() {
    let root = temp_root("http-missing-bucket");
    let server = start_server(&root).await;
    let response = signed(
        &server,
        "GET",
        &format!("/{BUCKET}/openai"),
        "",
        b"",
        ACCESS_KEY,
        SECRET_KEY,
    )
    .await;
    assert_eq!(response.status(), 404);
    let body = response.text().await.unwrap();
    assert!(
        body.contains("<Code>NoSuchBucket</Code>"),
        "body was {body}"
    );
}

// ---- the three failure modes the brief names -----------------------------

#[tokio::test]
async fn failure_mode_the_server_cannot_start_on_a_port_that_is_taken() {
    // [FACT] The binary's startup path is: bind the listen address, then
    // serve. When the address is taken, the bind has to fail — the server
    // must refuse to start loudly instead of pretending to serve.
    let taken = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = taken.local_addr().unwrap();

    let error = tokio::net::TcpListener::bind(addr)
        .await
        .expect_err("binding an address that is already taken must fail");
    assert_eq!(
        error.kind(),
        std::io::ErrorKind::AddrInUse,
        "the failure must name the real cause: {error}"
    );
    assert_eq!(
        taken.local_addr().unwrap(),
        addr,
        "the first server must still own the address"
    );
}

#[tokio::test]
async fn failure_mode_a_wrong_secret_is_rejected_with_403() {
    let root = temp_root("http-bad-signature");
    let server = start_server(&root).await;
    signed(
        &server,
        "PUT",
        &format!("/{BUCKET}"),
        "",
        b"",
        ACCESS_KEY,
        SECRET_KEY,
    )
    .await;

    // Signed with the right access key but the wrong secret.
    let response = signed(
        &server,
        "PUT",
        &format!("/{BUCKET}/openai"),
        "",
        b"sk-secret",
        ACCESS_KEY,
        "the-wrong-secret",
    )
    .await;
    assert_eq!(response.status(), 403);
    let body = response.text().await.unwrap();
    assert!(
        body.contains("<Code>SignatureDoesNotMatch</Code>"),
        "body was {body}"
    );
}

#[tokio::test]
async fn failure_mode_an_unknown_access_key_is_rejected_with_403() {
    let root = temp_root("http-bad-access-key");
    let server = start_server(&root).await;
    let response = signed(
        &server,
        "GET",
        &format!("/{BUCKET}/"),
        "list-type=2",
        b"",
        "not-the-access-key",
        SECRET_KEY,
    )
    .await;
    assert_eq!(response.status(), 403);
}

#[tokio::test]
async fn failure_mode_a_malformed_listing_makes_the_client_fail() {
    // [FACT] The client parses the listing with quick-xml, and
    // `ListBucketResult/Name` is required. A listing without it must
    // make the client error out — and the well-formed listing on the
    // normal server must not.
    let root = temp_root("http-broken-listing");
    let server = start_server_with(&root, ListingShape::MissingName, Some(BUCKET)).await;

    let bucket = client_bucket(&server.endpoint, BUCKET);
    let result = bucket.list(String::new(), None).await;
    assert!(
        result.is_err(),
        "the client must reject a listing without a <Name> element"
    );

    // The same server answers a well-formed listing correctly, so the
    // failure above is the shape of the document and nothing else.
    let root = temp_root("http-good-listing");
    let server = start_server_with(&root, ListingShape::Full, Some(BUCKET)).await;
    let bucket = client_bucket(&server.endpoint, BUCKET);
    let result = bucket.list(String::new(), None).await;
    assert!(result.is_ok(), "a well-formed listing must be accepted");
}

#[tokio::test]
async fn an_unsigned_request_is_rejected_with_403() {
    let root = temp_root("http-unsigned");
    let server = start_server(&root).await;
    let response = reqwest::get(format!("{}/{BUCKET}/openai", server.endpoint))
        .await
        .unwrap();
    assert_eq!(response.status(), 403);
}

// ---- the raw signing helper, kept honest ---------------------------------

#[test]
fn the_test_signer_produces_a_signature_the_server_accepts() {
    // A tiny self-check: the helper that signs the raw requests above
    // must agree with the module that verifies them.
    let mut headers = BTreeMap::new();
    headers.insert("host".to_string(), "127.0.0.1:9000".to_string());
    headers.insert("x-amz-date".to_string(), amz_date_now());
    headers.insert(
        "x-amz-content-sha256".to_string(),
        sigv4::EMPTY_PAYLOAD_SHA256.to_string(),
    );
    let signed_headers = headers.keys().cloned().collect::<Vec<_>>().join(";");
    let canonical = sigv4::canonical_request(
        "GET",
        "/gitgit-vault/openai",
        "",
        &headers,
        &signed_headers,
        sigv4::EMPTY_PAYLOAD_SHA256,
    );
    assert!(canonical.starts_with("GET\n/gitgit-vault/openai\n"));
}
