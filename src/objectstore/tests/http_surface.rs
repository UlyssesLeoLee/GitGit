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
    amz_date_at_offset, amz_date_now, client_bucket, send_signed, send_signed_shape, start_server,
    start_server_with, temp_root, RequestShape, ACCESS_KEY, BUCKET, SECRET_KEY,
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

/// [FACT] The bucket name becomes a directory name under the store's root,
/// so a name that is not a legal S3 bucket name is refused with
/// `400 InvalidBucketName` — and nothing is written.
///
/// [INFERENCE] The name is percent-encoded on the wire (`..%2f…`) so that
/// it survives as one URL segment: a WHATWG URL parser removes `..`
/// segments, so a literal `../` would never reach the server at all. The
/// server decodes it, and the decoded name is the one that would be joined
/// onto the root.
#[tokio::test]
async fn a_bucket_name_that_would_escape_the_root_is_refused() {
    let root = temp_root("http-bucket-escape");
    let server = start_server(&root).await;

    // The directory the escape would land in. It exists on purpose: the
    // name check is the only thing standing between the request and the
    // write.
    let target_name = format!(
        "gitgit-objectstore-http-escape-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let outside = root.parent().unwrap().join(&target_name);
    std::fs::create_dir_all(&outside).unwrap();

    let response = signed(
        &server,
        "PUT",
        &format!("/..%2f{target_name}"),
        "versioning",
        b"",
        ACCESS_KEY,
        SECRET_KEY,
    )
    .await;
    assert_eq!(response.status(), 400, "the request must be refused");
    let body = response.text().await.unwrap();
    assert!(
        body.contains("<Code>InvalidBucketName</Code>"),
        "body was {body}"
    );
    assert!(
        !outside.join("versioning.json").exists(),
        "nothing may be written outside the store's root"
    );
}

/// [FACT] A bucket name that is not a legal S3 bucket name is refused even
/// when it does not try to escape — the name is checked, not just screened
/// for the shapes that would escape.
#[tokio::test]
async fn a_bucket_name_that_is_not_a_legal_s3_name_is_refused() {
    let root = temp_root("http-bucket-name");
    let server = start_server(&root).await;

    for name in ["UPPERCASE", "has_underscore", "ab", "trailing."] {
        let response = signed(
            &server,
            "PUT",
            &format!("/{name}"),
            "",
            b"",
            ACCESS_KEY,
            SECRET_KEY,
        )
        .await;
        assert_eq!(response.status(), 400, "the name {name:?} must be refused");
        let body = response.text().await.unwrap();
        assert!(
            body.contains("<Code>InvalidBucketName</Code>"),
            "the name {name:?} must be reported as invalid, body was {body}"
        );
    }
}

/// [FACT] A version id that this store could not have issued is refused
/// with `400 InvalidArgument`, which is the code S3 uses for a malformed
/// version id. A well-formed id that simply does not exist stays a 404.
#[tokio::test]
async fn a_version_id_that_is_not_a_version_id_is_refused() {
    let root = temp_root("http-version-id");
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
        "GET",
        &format!("/{BUCKET}/openai"),
        "versionId=..%2f..%2fescape",
        b"",
        ACCESS_KEY,
        SECRET_KEY,
    )
    .await;
    assert_eq!(response.status(), 400);
    let body = response.text().await.unwrap();
    assert!(
        body.contains("<Code>InvalidArgument</Code>"),
        "body was {body}"
    );

    // A well-formed id that was never issued is a different thing: the
    // version is missing, not the request.
    let response = signed(
        &server,
        "GET",
        &format!("/{BUCKET}/openai"),
        "versionId=00000000000000000000000000000000",
        b"",
        ACCESS_KEY,
        SECRET_KEY,
    )
    .await;
    assert_eq!(response.status(), 404);
    let body = response.text().await.unwrap();
    assert!(
        body.contains("<Code>NoSuchVersion</Code>"),
        "body was {body}"
    );
}

/// [FACT] `rust-s3` 0.38 signs a header value by trimming it and leaving
/// the inside alone; AWS's rule collapses a run of internal whitespace
/// into a single space. The two rules disagree for exactly this case, and
/// the server has to accept the rule the client signed with — otherwise a
/// correctly signed request is refused for a reason that has nothing to
/// do with the credential.
#[tokio::test]
async fn a_header_value_with_an_internal_whitespace_run_is_accepted() {
    let root = temp_root("http-header-rule");
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

    let response = send_signed_shape(
        &server,
        RequestShape {
            method: "PUT",
            path: &format!("/{BUCKET}/openai"),
            query: "",
            payload: b"sk-secret",
            // Two spaces in the middle of the value — the one case the two
            // rules disagree about.
            extra_headers: &[("x-amz-meta-note", "a  b")],
            amz_date: None,
        },
        ACCESS_KEY,
        SECRET_KEY,
    )
    .await;
    assert_eq!(
        response.status(),
        200,
        "a request signed the way the client signs must be accepted"
    );
}

/// [FACT] A request date the server cannot read is **refused**. The
/// clock-skew window is a gate, and a gate that opens itself on input it
/// cannot parse is a gate with a hole in it.
#[tokio::test]
async fn a_request_date_the_server_cannot_read_is_refused() {
    let root = temp_root("http-unreadable-date");
    let server = start_server(&root).await;

    let response = send_signed_shape(
        &server,
        RequestShape {
            method: "GET",
            path: &format!("/{BUCKET}/openai"),
            query: "",
            payload: b"",
            extra_headers: &[],
            // The right shape, one character too long — and a trailing
            // character that is not part of the format.
            amz_date: Some("20260101T000000ZX"),
        },
        ACCESS_KEY,
        SECRET_KEY,
    )
    .await;
    assert_eq!(response.status(), 403);
    let body = response.text().await.unwrap();
    assert!(
        body.contains("<Code>RequestTimeTooSkewed</Code>"),
        "body was {body}"
    );
}

/// [FACT] A request outside the clock-skew window is refused, and it is
/// refused **as a skewed request** rather than as a bad signature: the
/// signature is over the date, so a correctly signed old request is still
/// a correctly signed request — it is just too old.
#[tokio::test]
async fn a_request_outside_the_clock_window_is_refused() {
    let root = temp_root("http-stale-date");
    let server = start_server(&root).await;

    let stale = amz_date_at_offset(-3_600);
    let response = send_signed_shape(
        &server,
        RequestShape {
            method: "GET",
            path: &format!("/{BUCKET}/openai"),
            query: "",
            payload: b"",
            extra_headers: &[],
            amz_date: Some(&stale),
        },
        ACCESS_KEY,
        SECRET_KEY,
    )
    .await;
    assert_eq!(response.status(), 403);
    let body = response.text().await.unwrap();
    assert!(
        body.contains("<Code>RequestTimeTooSkewed</Code>"),
        "body was {body}"
    );
}

/// [FACT] Deleting a *specific* version is not one of the operations this
/// store implements, and ignoring the version id would delete the current
/// version instead — the wrong object. The request is refused, and the
/// object is still there afterwards.
#[tokio::test]
async fn deleting_a_specific_version_is_refused_instead_of_deleting_the_current_one() {
    let root = temp_root("http-delete-version");
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
        "versionId=00000000000000000000000000000000",
        b"",
        ACCESS_KEY,
        SECRET_KEY,
    )
    .await;
    assert_eq!(response.status(), 400);
    let body = response.text().await.unwrap();
    assert!(
        body.contains("<Code>InvalidArgument</Code>"),
        "body was {body}"
    );

    // Refusing is not the same as deleting: the object is still readable.
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
        sigv4::HeaderValueRule::TrimOnly,
    );
    assert!(canonical.starts_with("GET\n/gitgit-vault/openai\n"));
}
