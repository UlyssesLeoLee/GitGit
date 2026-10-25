//! The HTTP surface of the object store.
//!
//! [FACT] The client is `rust-s3` 0.38 in path-style mode, so every
//! request is `http://host:port/{bucket}` or `http://host:port/{bucket}/{key}`.
//!
//! ## Why the routing is done by hand
//!
//! [FACT] Two things make axum's path patterns the wrong tool here:
//!
//! * the client sends `GET /{bucket}/` — **with a trailing slash** — for a
//!   bucket-level listing, because `Bucket::url()` already ends in the
//!   bucket name and the listing appends `/`;
//! * the SigV4 signature covers the **raw** path exactly as it arrived on
//!   the wire, so a router that rewrote the path would rewrite the thing
//!   that is signed.
//!
//! So the handler splits the raw path itself: the first segment is the
//! bucket, the rest is the key.
//!
//! ## What the path may contain
//!
//! [FACT] The bucket name and the version id both end up as **directory
//! names** under the store's root, so both are checked before they are used
//! as one. A bucket name that is not a legal S3 bucket name is answered
//! with `400 InvalidBucketName`; a version id that this store could not
//! have issued is answered with `400 InvalidArgument`, which is the code S3
//! uses for a malformed version id.
//!
//! [INFERENCE] The check has to happen **after** the percent-decoding, not
//! before it: `..%2fescape` is a single URL segment (a WHATWG URL parser
//! does not treat `%2f` as a separator), and it only becomes `../escape`
//! once it is decoded. The decoded name is the one that would be joined
//! onto the root.
//!
//! ## A note on the order of the work
//!
//! [FACT] The request body is read before the request is answered, and the
//! declared payload hash is compared against the hash of the body that
//! actually arrived: the signature covers the *declared* hash, so the two
//! have to be checked against each other.
//!
//! [INFERENCE] The body is therefore buffered before the signature is
//! verified, so an unsigned request can still make the server hold up to
//! `MAX_BODY_BYTES` in memory before it is refused. The signature itself
//! does not need the body — it needs the declared hash and the headers —
//! so verifying it first would close that window. It is not done here
//! because the handler is the one place where the declared hash and the
//! body are both in hand, and reordering the two paths is a change to the
//! one piece of this server that decides whether a request is allowed at
//! all. The server binds to the loopback address by default and the bound
//! is a fixed 64 MB per request, so this is a **named trade-off** rather
//! than an unbounded hole.

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{header, HeaderName, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Router;

use crate::objectstore::sigv4::{
    self, SignedRequest, Sigv4Credentials, Sigv4Error, EMPTY_PAYLOAD_SHA256, UNSIGNED_PAYLOAD,
};
use crate::objectstore::store::{self, StoreError, VersioningStatus, DEFAULT_MAX_KEYS};
use crate::objectstore::xml::{self, ListingShape};
use crate::objectstore::ObjectStore;

/// The largest request body the store accepts.
const MAX_BODY_BYTES: usize = 64 * 1024 * 1024;

/// The header the client reads the version id from.
const VERSION_ID_HEADER: &str = "x-amz-version-id";

/// The state the handlers serve.
#[derive(Clone, Debug)]
struct ServerState {
    store: ObjectStore,
    listing_shape: ListingShape,
}

/// Build the router that serves the object store.
pub fn router(state: ObjectStore) -> Router {
    router_with_listing_shape(state, ListingShape::Full)
}

/// Build a router whose listing documents have the given shape.
///
/// [PROPOSAL] The broken shape exists so a test can prove that a
/// malformed listing really does make the client fail, rather than
/// assuming it.
pub(crate) fn router_with_listing_shape(state: ObjectStore, shape: ListingShape) -> Router {
    Router::new().fallback(handle).with_state(ServerState {
        store: state,
        listing_shape: shape,
    })
}

/// The one handler every request goes through.
async fn handle(State(state): State<ServerState>, request: Request) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let raw_path = uri.path().to_string();
    let raw_query = uri.query().unwrap_or("").to_string();

    let mut headers: BTreeMap<String, String> = BTreeMap::new();
    for (name, value) in request.headers().iter() {
        let name = name.as_str().to_ascii_lowercase();
        let value = value.to_str().unwrap_or("").to_string();
        // The first occurrence wins: SigV4 signs one value per name.
        headers.entry(name).or_insert(value);
    }

    let body = match axum::body::to_bytes(request.into_body(), MAX_BODY_BYTES).await {
        Ok(body) => body,
        Err(_) => {
            return error_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                "EntityTooLarge",
                "The request body is too large.",
            )
        }
    };

    if let Err(err) = verify_signature(
        state.store.credentials(),
        &method,
        &raw_path,
        &raw_query,
        &headers,
        &body,
    ) {
        return signature_error_response(&err);
    }

    let (bucket, key) = split_path(&raw_path);
    let query_pairs = sigv4::parse_query_pairs(&raw_query);
    let key = key.filter(|key| !key.is_empty());

    // [FACT] The bucket name becomes a directory name, so it is checked
    // before it is used as one. The check is here, at the boundary, rather
    // than only in the storage layer, so that a name that would escape the
    // root is answered with "that name is not valid" instead of a 500.
    // The signature has already been verified at this point, so an
    // unsigned request still gets 403 and not 400.
    if store::validate_bucket_name(&bucket).is_err() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "InvalidBucketName",
            &format!("The specified bucket is not valid: {bucket}"),
        );
    }

    match (method.as_str(), key) {
        ("PUT", None) => {
            if has_query_key(&query_pairs, "versioning") {
                put_bucket_versioning(&state, &bucket).await
            } else {
                put_bucket(&state, &bucket).await
            }
        }
        ("GET", None) => {
            if has_query_key(&query_pairs, "versioning") {
                get_bucket_versioning(&state, &bucket).await
            } else if query_value(&query_pairs, "list-type").as_deref() == Some("2") {
                list_objects(&state, &bucket, &query_pairs).await
            } else {
                // [FACT] The brief pins this: a bucket-level GET without a
                // recognised sub-resource answers "no such bucket".
                error_response(
                    StatusCode::NOT_FOUND,
                    "NoSuchBucket",
                    "The specified bucket does not exist.",
                )
            }
        }
        ("PUT", Some(key)) => put_object(&state, &bucket, &key, &body, &headers).await,
        ("GET", Some(key)) => {
            let version_id = query_value(&query_pairs, "versionId");
            get_object(&state, &bucket, &key, version_id.as_deref(), false).await
        }
        ("HEAD", Some(key)) => {
            let version_id = query_value(&query_pairs, "versionId");
            get_object(&state, &bucket, &key, version_id.as_deref(), true).await
        }
        ("DELETE", Some(key)) => delete_object(&state, &bucket, &key).await,
        _ => error_response(
            StatusCode::METHOD_NOT_ALLOWED,
            "MethodNotAllowed",
            "The specified method is not allowed against this resource.",
        ),
    }
}

/// Split the raw request path into its bucket and key.
///
/// The bucket is the first segment; the key is everything after it, still
/// percent-encoded on the wire and decoded here.
fn split_path(raw_path: &str) -> (String, Option<String>) {
    let trimmed = raw_path.strip_prefix('/').unwrap_or(raw_path);
    if trimmed.is_empty() {
        return (String::new(), None);
    }
    match trimmed.find('/') {
        Some(index) => (
            sigv4::percent_decode(&trimmed[..index]),
            Some(sigv4::percent_decode(&trimmed[index + 1..])),
        ),
        None => (sigv4::percent_decode(trimmed), None),
    }
}

/// Verify the signature that came with the request.
///
/// [FACT] Two variants exist: the `Authorization` header (used by every
/// ordinary request) and the query string (used by `presign_get`, which is
/// how the vault reads a historical version).
fn verify_signature(
    credentials: &Sigv4Credentials,
    method: &Method,
    raw_path: &str,
    raw_query: &str,
    headers: &BTreeMap<String, String>,
    body: &[u8],
) -> Result<(), Sigv4Error> {
    let authorization = headers.get("authorization").cloned();
    let query_pairs = sigv4::parse_query_pairs(raw_query);
    let has_query_signature = query_pairs
        .iter()
        .any(|(name, _)| name == "X-Amz-Signature");

    let payload_sha256 = if authorization.is_some() {
        headers
            .get("x-amz-content-sha256")
            .cloned()
            .unwrap_or_else(|| EMPTY_PAYLOAD_SHA256.to_string())
    } else if has_query_signature {
        UNSIGNED_PAYLOAD.to_string()
    } else {
        return Err(Sigv4Error::MissingSignature);
    };

    let request = SignedRequest {
        method: method.as_str(),
        path: raw_path,
        query: raw_query,
        headers,
        payload_sha256: &payload_sha256,
    };

    if let Some(authorization) = &authorization {
        // A replay of an old request still carries a valid signature, so
        // the date is checked against the server's clock.
        if let Some(date) = headers.get("x-amz-date") {
            if let Some(parsed) = sigv4::parse_amz_date(date) {
                let now = now_epoch_secs();
                if (now - parsed.epoch_secs).abs() > sigv4::MAX_CLOCK_SKEW_SECS {
                    return Err(Sigv4Error::DateOutsideWindow);
                }
            }
        }
        sigv4::verify_header(credentials, &request, authorization)?;
    } else {
        sigv4::verify_query(credentials, &request, now_epoch_secs())?;
    }

    // The payload hash is part of the signature, so it has to describe the
    // body that actually arrived.
    if payload_sha256 != UNSIGNED_PAYLOAD
        && payload_sha256.len() == 64
        && payload_sha256.chars().all(|c| c.is_ascii_hexdigit())
    {
        let actual = sigv4::sha256_hex(body);
        if actual != payload_sha256 {
            return Err(Sigv4Error::SignatureMismatch);
        }
    }

    Ok(())
}

/// `PUT /{bucket}` — create the bucket.
async fn put_bucket(state: &ServerState, bucket: &str) -> Response {
    match state.store.store().create_bucket(bucket).await {
        Ok(_) => StatusCode::OK.into_response(),
        Err(StoreError::InvalidBucketName(name)) => error_response(
            StatusCode::BAD_REQUEST,
            "InvalidBucketName",
            &format!("The specified bucket is not valid: {name}"),
        ),
        Err(err) => internal_error(&err),
    }
}

/// `PUT /{bucket}?versioning` — turn versioning on.
async fn put_bucket_versioning(state: &ServerState, bucket: &str) -> Response {
    if !bucket_exists(state, bucket).await {
        return error_response(
            StatusCode::NOT_FOUND,
            "NoSuchBucket",
            "The specified bucket does not exist.",
        );
    }
    match state
        .store
        .set_versioning(bucket, VersioningStatus::Enabled)
        .await
    {
        Ok(()) => StatusCode::OK.into_response(),
        Err(err) => internal_error(&err),
    }
}

/// `GET /{bucket}?versioning` — the versioning state of the bucket.
async fn get_bucket_versioning(state: &ServerState, bucket: &str) -> Response {
    if !bucket_exists(state, bucket).await {
        return error_response(
            StatusCode::NOT_FOUND,
            "NoSuchBucket",
            "The specified bucket does not exist.",
        );
    }
    match state.store.get_versioning(bucket).await {
        Ok(status) => xml_response(StatusCode::OK, xml::render_versioning(status)),
        Err(err) => internal_error(&err),
    }
}

/// `GET /{bucket}?list-type=2` — one page of the bucket's keys.
async fn list_objects(
    state: &ServerState,
    bucket: &str,
    query_pairs: &[(String, String)],
) -> Response {
    if !bucket_exists(state, bucket).await {
        return error_response(
            StatusCode::NOT_FOUND,
            "NoSuchBucket",
            "The specified bucket does not exist.",
        );
    }
    let prefix = query_value(query_pairs, "prefix").unwrap_or_default();
    let continuation_token = query_value(query_pairs, "continuation-token");
    let max_keys = query_value(query_pairs, "max-keys")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(DEFAULT_MAX_KEYS);

    match state
        .store
        .store()
        .list(bucket, &prefix, continuation_token.as_deref(), max_keys)
        .await
    {
        Ok(page) => {
            let body = xml::render_list_bucket(
                bucket,
                &page.prefix,
                &page.entries,
                page.is_truncated,
                page.next_continuation_token.as_deref(),
                page.max_keys,
                state.listing_shape,
            );
            xml_response(StatusCode::OK, body)
        }
        Err(StoreError::InvalidContinuationToken(token)) => error_response(
            StatusCode::BAD_REQUEST,
            "InvalidArgument",
            &format!("The continuation token is not valid: {token}"),
        ),
        Err(err) => internal_error(&err),
    }
}

/// `PUT /{bucket}/{key}` — store a new version of an object.
async fn put_object(
    state: &ServerState,
    bucket: &str,
    key: &str,
    body: &[u8],
    headers: &BTreeMap<String, String>,
) -> Response {
    if !bucket_exists(state, bucket).await {
        return error_response(
            StatusCode::NOT_FOUND,
            "NoSuchBucket",
            "The specified bucket does not exist.",
        );
    }
    let content_type = headers.get("content-type").map(String::as_str);
    match state
        .store
        .store()
        .put(bucket, key, body, content_type)
        .await
    {
        Ok(version_id) => {
            let etag = format!("\"{}\"", sigv4::sha256_hex(body));
            Response::builder()
                .status(StatusCode::OK)
                .header(header::ETAG, etag)
                .header(HeaderName::from_static(VERSION_ID_HEADER), version_id)
                .body(Body::empty())
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
        Err(StoreError::InvalidKey(_)) => error_response(
            StatusCode::BAD_REQUEST,
            "InvalidArgument",
            "The object key is not valid.",
        ),
        Err(StoreError::InvalidBucketName(name)) => error_response(
            StatusCode::BAD_REQUEST,
            "InvalidBucketName",
            &format!("The specified bucket is not valid: {name}"),
        ),
        Err(err) => internal_error(&err),
    }
}

/// `GET`/`HEAD /{bucket}/{key}` — read an object.
async fn get_object(
    state: &ServerState,
    bucket: &str,
    key: &str,
    version_id: Option<&str>,
    head_only: bool,
) -> Response {
    if !bucket_exists(state, bucket).await {
        return error_response(
            StatusCode::NOT_FOUND,
            "NoSuchBucket",
            "The specified bucket does not exist.",
        );
    }
    // [FACT] A version id becomes a directory name too, so a malformed one
    // is refused with the code S3 uses for it: `InvalidArgument`. A
    // well-formed id that this store never issued stays a 404
    // (`NoSuchVersion`), which is a different thing entirely.
    if let Some(version_id) = version_id {
        if store::validate_version_id(version_id).is_err() {
            return error_response(
                StatusCode::BAD_REQUEST,
                "InvalidArgument",
                "The specified version id is not valid.",
            );
        }
    }
    let result = match version_id {
        Some(version_id) => {
            state
                .store
                .store()
                .get_version(bucket, key, version_id)
                .await
        }
        None => state.store.store().get_current(bucket, key).await,
    };
    let object = match result {
        Ok(Some(object)) => object,
        Ok(None) => {
            return error_response(
                StatusCode::NOT_FOUND,
                if version_id.is_some() {
                    "NoSuchVersion"
                } else {
                    "NoSuchKey"
                },
                if version_id.is_some() {
                    "The specified version does not exist."
                } else {
                    "The specified key does not exist."
                },
            )
        }
        Err(StoreError::InvalidVersionId(_)) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "InvalidArgument",
                "The specified version id is not valid.",
            )
        }
        Err(StoreError::InvalidBucketName(name)) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "InvalidBucketName",
                &format!("The specified bucket is not valid: {name}"),
            )
        }
        Err(err) => return internal_error(&err),
    };

    let etag = format!("\"{}\"", object.meta.etag);
    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(header::ETAG, etag)
        .header(
            HeaderName::from_static(VERSION_ID_HEADER),
            object.version_id.clone(),
        );
    if let Some(content_type) = &object.meta.content_type {
        builder = builder.header(header::CONTENT_TYPE, content_type.clone());
    }
    let body = if head_only {
        Body::empty()
    } else {
        Body::from(object.bytes)
    };
    builder
        .body(body)
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

/// `DELETE /{bucket}/{key}` — remove the current version of an object.
async fn delete_object(state: &ServerState, bucket: &str, key: &str) -> Response {
    if !bucket_exists(state, bucket).await {
        return error_response(
            StatusCode::NOT_FOUND,
            "NoSuchBucket",
            "The specified bucket does not exist.",
        );
    }
    match state.store.store().delete(bucket, key).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => internal_error(&err),
    }
}

/// Whether a bucket exists.
async fn bucket_exists(state: &ServerState, bucket: &str) -> bool {
    matches!(state.store.bucket_exists(bucket).await, Ok(true))
}

/// The value of a query parameter.
fn query_value(query_pairs: &[(String, String)], name: &str) -> Option<String> {
    query_pairs
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.clone())
}

/// Whether a query parameter is present at all (its value may be empty).
fn has_query_key(query_pairs: &[(String, String)], name: &str) -> bool {
    query_pairs.iter().any(|(key, _)| key == name)
}

/// The current time in seconds since the Unix epoch.
fn now_epoch_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// An XML response.
fn xml_response(status: StatusCode, body: String) -> Response {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/xml")
        .body(Body::from(body))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

/// An S3 error response.
fn error_response(status: StatusCode, code: &str, message: &str) -> Response {
    xml_response(status, xml::render_error(code, message, None))
}

/// A 500 response for an unexpected storage failure.
fn internal_error(err: &impl std::fmt::Display) -> Response {
    tracing::error!(error = %err, "object store request failed");
    error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        "InternalError",
        "The object store could not complete the request.",
    )
}

/// The response for a signature that did not verify.
fn signature_error_response(err: &Sigv4Error) -> Response {
    match err {
        Sigv4Error::MissingSignature => error_response(
            StatusCode::FORBIDDEN,
            "AccessDenied",
            "The request must be signed.",
        ),
        Sigv4Error::UnknownAccessKey => error_response(
            StatusCode::FORBIDDEN,
            "InvalidAccessKeyId",
            "The access key id that was provided does not exist.",
        ),
        Sigv4Error::Expired => error_response(
            StatusCode::FORBIDDEN,
            "AccessDenied",
            "The presigned request has expired.",
        ),
        Sigv4Error::DateOutsideWindow => error_response(
            StatusCode::FORBIDDEN,
            "RequestTimeTooSkewed",
            "The request date is too far from the server's clock.",
        ),
        _ => error_response(
            StatusCode::FORBIDDEN,
            "SignatureDoesNotMatch",
            "The request signature does not match the signature the server calculated.",
        ),
    }
}
