//! Shared helpers for the object store's tests.
//!
//! [FACT] Every test binds port 0, so the operating system picks a free
//! port and parallel test runs cannot collide.

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use s3::creds::Credentials;
use s3::{Bucket, BucketConfiguration, Region};

use crate::objectstore::sigv4;
use crate::objectstore::store::VersioningStatus;
use crate::objectstore::xml::ListingShape;
use crate::objectstore::{ObjectStore, ObjectStoreCredentials};

/// The access key the tests sign with.
pub const ACCESS_KEY: &str = "gitgit-access-key";

/// The secret key the tests sign with.
pub const SECRET_KEY: &str = "gitgit-secret-key";

/// The bucket the tests use.
pub const BUCKET: &str = "gitgit-vault";

/// A fresh temporary directory for one test.
pub fn temp_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!(
        "gitgit-objectstore-test-{label}-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A running object store.
pub struct TestServer {
    /// The address it listens on.
    pub addr: SocketAddr,
    /// The endpoint URL a client should use.
    pub endpoint: String,
}

/// Start the object store in-process, on a port the OS picked.
pub async fn start_server(root: &Path) -> TestServer {
    start_server_with(root, ListingShape::Full, None).await
}

/// Start the object store with a bucket that already exists and has
/// versioning enabled — the state the vault's e2e tests need.
pub async fn start_server_with_bucket(root: &Path) -> TestServer {
    start_server_with(root, ListingShape::Full, Some(BUCKET)).await
}

/// Start the object store with a chosen listing shape and an optional
/// pre-provisioned bucket.
pub async fn start_server_with(
    root: &Path,
    listing_shape: ListingShape,
    bucket: Option<&str>,
) -> TestServer {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let credentials = ObjectStoreCredentials::new(ACCESS_KEY, SECRET_KEY);
    let state = ObjectStore::new(root, credentials).unwrap();
    if let Some(bucket) = bucket {
        state.create_bucket(bucket).await.unwrap();
        state
            .set_versioning(bucket, VersioningStatus::Enabled)
            .await
            .unwrap();
    }
    let app = crate::objectstore::http::router_with_listing_shape(state, listing_shape);
    // The task lives as long as the test's runtime; dropping it stops the
    // server with it.
    let _handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    TestServer {
        addr,
        endpoint: format!("http://{addr}"),
    }
}

/// Create a bucket through the client's own API.
///
/// [FACT] This is the `PUT /{bucket}` operation as the `rust-s3` client
/// performs it: a path-style request with a location-constraint body.
pub async fn create_bucket(server: &TestServer, bucket_name: &str) {
    let region = Region::Custom {
        region: "us-east-1".to_string(),
        endpoint: server.endpoint.clone(),
    };
    let credentials =
        Credentials::new(Some(ACCESS_KEY), Some(SECRET_KEY), None, None, None).unwrap();
    let response = Bucket::create_with_path_style(
        bucket_name,
        region,
        credentials,
        BucketConfiguration::default(),
    )
    .await
    .unwrap();
    assert_eq!(
        response.response_code, 200,
        "the bucket {bucket_name} could not be created"
    );
}

/// A `rust-s3` bucket handle pointed at the running server.
pub fn client_bucket(endpoint: &str, bucket_name: &str) -> Bucket {
    client_bucket_with_keys(endpoint, bucket_name, ACCESS_KEY, SECRET_KEY)
}

/// A `rust-s3` bucket handle with explicit credentials.
pub fn client_bucket_with_keys(
    endpoint: &str,
    bucket_name: &str,
    access_key: &str,
    secret_key: &str,
) -> Bucket {
    let region = Region::Custom {
        region: "us-east-1".to_string(),
        endpoint: endpoint.to_string(),
    };
    let credentials =
        Credentials::new(Some(access_key), Some(secret_key), None, None, None).unwrap();
    let bucket = Bucket::new(bucket_name, region, credentials).unwrap();
    *bucket.with_path_style()
}

/// The current time in the `YYYYMMDDTHHMMSSZ` form AWS uses.
pub fn amz_date_now() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = now.div_euclid(86_400);
    let time_of_day = now.rem_euclid(86_400);
    let (year, month, day) = crate::objectstore::store::civil_from_days(days);
    format!(
        "{year:04}{month:02}{day:02}T{:02}{:02}{:02}Z",
        time_of_day / 3_600,
        (time_of_day % 3_600) / 60,
        time_of_day % 60
    )
}

/// Sign a request the way the server expects it to be signed.
///
/// [FACT] This is the header variant of SigV4. It exists so a test can
/// drive the server with exact status codes, exact headers and — when it
/// has to — a deliberately wrong secret.
pub fn authorization_header(
    method: &str,
    path: &str,
    query: &str,
    headers: &BTreeMap<String, String>,
    payload: &[u8],
    access_key: &str,
    secret_key: &str,
) -> String {
    let region = "us-east-1";
    let amz_date = headers
        .get("x-amz-date")
        .expect("the caller must set x-amz-date")
        .clone();
    let date_stamp = &amz_date[..8];
    let payload_hash = sigv4::sha256_hex(payload);
    let canonical_query = sigv4::canonical_query_string(&sigv4::parse_query_pairs(query));
    let signed_headers = headers.keys().cloned().collect::<Vec<_>>().join(";");
    let canonical = sigv4::canonical_request(
        method,
        &sigv4::canonical_uri(path),
        &canonical_query,
        headers,
        &signed_headers,
        &payload_hash,
    );
    let scope = format!("{date_stamp}/{region}/s3/aws4_request");
    let string_to_sign = sigv4::string_to_sign(&amz_date, &scope, &canonical);
    let key = sigv4::signing_key(secret_key, date_stamp, region, "s3");
    let signature = hex::encode(sigv4::hmac_sha256(&key, string_to_sign.as_bytes()));
    format!(
        "AWS4-HMAC-SHA256 Credential={access_key}/{scope}, SignedHeaders={signed_headers}, Signature={signature}"
    )
}

/// Send a signed request to the running server.
pub async fn send_signed(
    server: &TestServer,
    method: &str,
    path: &str,
    query: &str,
    payload: &[u8],
    access_key: &str,
    secret_key: &str,
) -> reqwest::Response {
    let mut headers: BTreeMap<String, String> = BTreeMap::new();
    headers.insert("host".to_string(), server.addr.to_string());
    headers.insert("x-amz-date".to_string(), amz_date_now());
    headers.insert(
        "x-amz-content-sha256".to_string(),
        sigv4::sha256_hex(payload),
    );
    let authorization = authorization_header(
        method, path, query, &headers, payload, access_key, secret_key,
    );

    let method = reqwest::Method::from_bytes(method.as_bytes()).unwrap();
    let mut url = format!("http://{}{}", server.addr, path);
    if !query.is_empty() {
        url.push('?');
        url.push_str(query);
    }
    let client = reqwest::Client::new();
    client
        .request(method, url)
        .header("host", server.addr.to_string())
        .header("x-amz-date", headers["x-amz-date"].clone())
        .header(
            "x-amz-content-sha256",
            headers["x-amz-content-sha256"].clone(),
        )
        .header("authorization", authorization)
        .body(payload.to_vec())
        .send()
        .await
        .unwrap()
}
