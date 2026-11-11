//! AWS Signature Version 4 verification for the self-hosted object store.
//!
//! [FACT] The client is `rust-s3` 0.38, which signs every request with
//! SigV4 in one of two shapes:
//!
//! 1. **header variant** — `Authorization: AWS4-HMAC-SHA256
//!    Credential=<ak>/<date>/<region>/s3/aws4_request,
//!    SignedHeaders=<a;b;c>, Signature=<hex>`;
//! 2. **query variant** — the same material in the query string
//!    (`X-Amz-Algorithm`, `X-Amz-Credential`, `X-Amz-Date`,
//!    `X-Amz-Expires`, `X-Amz-SignedHeaders`, `X-Amz-Signature`), which is
//!    what `Bucket::presign_get` produces. The vault reads a historical
//!    version through that path, because `rust-s3` 0.38 cannot address a
//!    `versionId` on a plain `GetObject`.
//!
//! [FACT] A custom query parameter (`versionId`) participates in the
//! signature, so it has to be part of the canonical query — otherwise the
//! signature would not cover the version that is being read.
//!
//! ## Why the region is not configured
//!
//! [FACT] AWS derives the signing key from the *credential scope* that the
//! client sent, not from a server-side configuration. The operator's region
//! string is therefore taken verbatim from the scope: any region works, and
//! a mismatch is impossible by construction. This is why
//! [`Sigv4Credentials`] carries no `region` field.
//!
//! ## Canonicalisation rules (and why they look like this)
//!
//! [FACT] The canonicalisation reproduces what the **client** signs, because
//! the server has to reproduce the client's view of the request:
//!
//! * **canonical URI** — percent-decode the raw request path, then
//!   re-encode it with everything except the RFC 3986 unreserved set
//!   escaped (`/` is *not* escaped in a path);
//! * **canonical query** — decode the raw query as
//!   `application/x-www-form-urlencoded` (`+` is a space), re-encode each
//!   name and value with the unreserved set only (`/` *is* escaped in a
//!   query), then sort by (name, value);
//! * **canonical headers** — the signed headers, in the order the
//!   `SignedHeaders` list gives, each value trimmed;
//! * **payload hash** — the `x-amz-content-sha256` header for the header
//!   variant, the literal `UNSIGNED-PAYLOAD` for the query variant.
//!
//! [FACT] **One place where two rules disagree, and both are accepted.**
//! AWS's rule for a canonical header value trims the ends *and* collapses
//! a run of internal whitespace into a single space; the published
//! `get-header-value-trim` vector pins that down (`"a   b   c"` becomes
//! `"a b c"`). `rust-s3` 0.38 trims the ends and leaves the inside alone.
//! The two produce the same canonical request for every header value
//! without an internal whitespace run — which is every value this
//! product's client actually signs (`host`, `x-amz-content-sha256`,
//! `x-amz-date`, `accept`, `content-length`, `content-type`,
//! `content-md5`).
//!
//! [INFERENCE] The server therefore verifies against **both** rules: the
//! canonical request is built once per rule, and the request is accepted
//! if the signature matches either. That is not a widening of what a valid
//! signature is — each candidate is still an HMAC over the request under
//! the secret key — it is the refusal to decide on the client's behalf
//! which of the two conventions it follows.
//!
//! [FACT] `X-Amz-Signature` is never part of the canonical query: it is
//! the signature itself.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};
use thiserror::Error;

/// SHA-256 of the empty byte string — the payload hash of every request
/// that carries no body.
pub const EMPTY_PAYLOAD_SHA256: &str =
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

/// The literal AWS uses in place of a payload hash when the payload is not
/// covered by the signature (presigned requests).
pub const UNSIGNED_PAYLOAD: &str = "UNSIGNED-PAYLOAD";

/// The only signing algorithm this server accepts.
pub const ALGORITHM: &str = "AWS4-HMAC-SHA256";

/// The only service this server signs for.
pub const SERVICE: &str = "s3";

/// Credentials a request is verified against.
///
/// [FACT] There is deliberately no `region` field: AWS derives the signing
/// key from the credential scope that the *client* sent, so a server that
/// pins its own region would reject perfectly valid clients.
///
/// [FACT] `Debug` is written by hand and redacts the secret key, following
/// the repository's own rule for credentials (`S3Vault` does the same, and
/// `s3_vault_debug_does_not_leak_credentials` holds it in place). Nothing
/// logs this today, but a derived `Debug` that prints the secret is one
/// `tracing::info!` away from leaking it.
#[derive(Clone, PartialEq, Eq)]
pub struct Sigv4Credentials {
    /// The access key (the "user name" half of the credential).
    pub access_key: String,
    /// The secret key (the "password" half of the credential).
    pub secret_key: String,
}

impl Sigv4Credentials {
    /// Build a credential pair.
    pub fn new(access_key: impl Into<String>, secret_key: impl Into<String>) -> Self {
        Self {
            access_key: access_key.into(),
            secret_key: secret_key.into(),
        }
    }
}

impl std::fmt::Debug for Sigv4Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sigv4Credentials")
            .field("access_key", &self.access_key)
            .field("secret_key", &"<redacted>")
            .finish()
    }
}

/// The parts of an HTTP request that a SigV4 signature covers.
#[derive(Clone, Debug)]
pub struct SignedRequest<'a> {
    /// HTTP method, uppercase (`GET`, `PUT`, ...).
    pub method: &'a str,
    /// The **raw** request path, still percent-encoded exactly as it
    /// arrived on the wire.
    pub path: &'a str,
    /// The **raw** query string, without the leading `?`.
    pub query: &'a str,
    /// Request headers, keyed by lowercase name.
    pub headers: &'a BTreeMap<String, String>,
    /// The payload hash that goes into the canonical request.
    pub payload_sha256: &'a str,
}

/// Everything that can go wrong while verifying a signature.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum Sigv4Error {
    /// No `Authorization` header and no query signature.
    #[error("the request carries no signature")]
    MissingSignature,
    /// The `Authorization` header could not be parsed.
    #[error("malformed Authorization header: {0}")]
    MalformedAuthorization(String),
    /// The algorithm is not `AWS4-HMAC-SHA256`.
    #[error("unsupported signing algorithm: {0}")]
    UnsupportedAlgorithm(String),
    /// A required query parameter is missing.
    #[error("missing query parameter: {0}")]
    MissingQueryParameter(&'static str),
    /// The credential scope could not be parsed.
    #[error("malformed credential scope: {0}")]
    MalformedScope(String),
    /// The service in the credential scope is not `s3`.
    #[error("unsupported service in credential scope: {0}")]
    UnsupportedService(String),
    /// The access key is not the one this server knows.
    #[error("unknown access key")]
    UnknownAccessKey,
    /// A header named in `SignedHeaders` did not arrive.
    #[error("signed header is missing from the request: {0}")]
    MissingSignedHeader(String),
    /// The signature does not match the one the server computed.
    #[error("signature does not match")]
    SignatureMismatch,
    /// The request date is outside the accepted clock-skew window.
    #[error("request date is outside the accepted clock-skew window")]
    DateOutsideWindow,
    /// The request date is not a date this server can read.
    #[error("the request date could not be read: {0:?}")]
    MalformedDate(String),
    /// The presigned URL has expired.
    #[error("the presigned request has expired")]
    Expired,
}

/// How far a request date may be from the server's clock, in seconds.
///
/// [FACT] AWS allows 15 minutes of skew for header-signed requests.
pub const MAX_CLOCK_SKEW_SECS: i64 = 15 * 60;

/// Verify a signature that was carried in the `Authorization` header.
///
/// `authorization` is the raw header value.
pub fn verify_header(
    creds: &Sigv4Credentials,
    req: &SignedRequest<'_>,
    authorization: &str,
) -> Result<(), Sigv4Error> {
    let mut credential: Option<&str> = None;
    let mut signed_headers: Option<&str> = None;
    let mut signature: Option<&str> = None;

    // [FACT] The header is
    // `AWS4-HMAC-SHA256 Credential=…,SignedHeaders=…,Signature=…`:
    // the algorithm is separated from the first parameter by a **space**,
    // and the parameters are separated by commas.
    let mut segments = authorization.split(',');
    let first = segments.next().unwrap_or("").trim();
    let (algorithm, first_parameter) = match first.split_once(' ') {
        Some((algorithm, parameter)) => (algorithm, parameter.trim()),
        None => (first, ""),
    };

    let parameters = std::iter::once(first_parameter).chain(segments.map(|part| part.trim()));
    for part in parameters {
        if part.is_empty() {
            continue;
        }
        if let Some(value) = part.strip_prefix("Credential=") {
            credential = Some(value.trim());
        } else if let Some(value) = part.strip_prefix("SignedHeaders=") {
            signed_headers = Some(value.trim());
        } else if let Some(value) = part.strip_prefix("Signature=") {
            signature = Some(value.trim());
        } else {
            return Err(Sigv4Error::MalformedAuthorization(part.to_string()));
        }
    }

    if algorithm != ALGORITHM {
        return Err(Sigv4Error::UnsupportedAlgorithm(algorithm.to_string()));
    }
    let credential = credential
        .ok_or_else(|| Sigv4Error::MalformedAuthorization("missing Credential".to_string()))?;
    let signed_headers = signed_headers
        .ok_or_else(|| Sigv4Error::MalformedAuthorization("missing SignedHeaders".to_string()))?;
    let signature = signature
        .ok_or_else(|| Sigv4Error::MalformedAuthorization("missing Signature".to_string()))?;

    verify(creds, req, signature, signed_headers, credential)
}

/// Verify a signature that was carried in the query string.
///
/// [FACT] This is the shape `Bucket::presign_get` produces, and it is the
/// only way the vault can read a historical version (`rust-s3` 0.38 has no
/// `versionId` on a plain `GetObject`).
pub fn verify_query(
    creds: &Sigv4Credentials,
    req: &SignedRequest<'_>,
    now_epoch_secs: i64,
) -> Result<(), Sigv4Error> {
    let pairs = parse_query_pairs(req.query);
    let lookup = |name: &str| {
        pairs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    };

    let algorithm =
        lookup("X-Amz-Algorithm").ok_or(Sigv4Error::MissingQueryParameter("X-Amz-Algorithm"))?;
    if algorithm != ALGORITHM {
        return Err(Sigv4Error::UnsupportedAlgorithm(algorithm.to_string()));
    }
    let credential =
        lookup("X-Amz-Credential").ok_or(Sigv4Error::MissingQueryParameter("X-Amz-Credential"))?;
    let date = lookup("X-Amz-Date").ok_or(Sigv4Error::MissingQueryParameter("X-Amz-Date"))?;
    let signed_headers = lookup("X-Amz-SignedHeaders")
        .ok_or(Sigv4Error::MissingQueryParameter("X-Amz-SignedHeaders"))?;
    let signature =
        lookup("X-Amz-Signature").ok_or(Sigv4Error::MissingQueryParameter("X-Amz-Signature"))?;

    // The presigned request is only valid inside its own expiry window.
    let expires = lookup("X-Amz-Expires")
        .and_then(|value| value.parse::<i64>().ok())
        .ok_or(Sigv4Error::MissingQueryParameter("X-Amz-Expires"))?;
    let signed_at = parse_amz_date(date)
        .ok_or_else(|| Sigv4Error::MalformedScope(format!("unparseable X-Amz-Date: {date}")))?;
    if now_epoch_secs > signed_at.epoch_secs + expires {
        return Err(Sigv4Error::Expired);
    }

    // The query variant signs the `host` header only, but the string to
    // sign still needs the request timestamp. Inject it into a private
    // copy of the header map; it is not in `SignedHeaders`, so it cannot
    // change the canonical headers.
    let mut headers = req.headers.clone();
    headers.insert("x-amz-date".to_string(), date.to_string());
    let inner = SignedRequest {
        method: req.method,
        path: req.path,
        query: req.query,
        headers: &headers,
        payload_sha256: UNSIGNED_PAYLOAD,
    };

    verify(creds, &inner, signature, signed_headers, credential)
}

/// How a signed header's value is normalised before it is signed.
///
/// [FACT] The two rules differ in exactly one case: a header value that
/// carries a run of internal whitespace. AWS's rule collapses the run to
/// a single space; `rust-s3`'s rule leaves it alone.
///
/// [FACT] Which one a client uses is the client's own business — the
/// server has to accept the rule the client signed with, or the request
/// fails for a reason that has nothing to do with the credential.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeaderValueRule {
    /// AWS's rule: trim the ends, and collapse a run of internal
    /// whitespace into a single space.
    ///
    /// [FACT] This is what the published AWS test suite pins down — the
    /// `get-header-value-trim` vector sends `"a   b   c"` and its
    /// canonical request spells it `"a b c"`.
    Collapse,
    /// `rust-s3` 0.38's rule: trim the ends, and leave what is between
    /// them exactly as it arrived.
    ///
    /// [FACT] `rust-s3` signs with `value.to_str()?.trim()` and nothing
    /// more. It is the rule this product's own client uses, because the
    /// vault's client is `rust-s3`.
    TrimOnly,
}

/// The core verification step.
///
/// `credential` is the full credential scope as it appears in the request:
/// `<access-key>/<date>/<region>/<service>/aws4_request`.
pub fn verify(
    creds: &Sigv4Credentials,
    req: &SignedRequest<'_>,
    signature: &str,
    signed_headers: &str,
    credential: &str,
) -> Result<(), Sigv4Error> {
    let scope = CredentialScope::parse(credential)?;
    if scope.service != SERVICE {
        return Err(Sigv4Error::UnsupportedService(scope.service.clone()));
    }
    if scope.access_key != creds.access_key {
        return Err(Sigv4Error::UnknownAccessKey);
    }

    let amz_date = req
        .headers
        .get("x-amz-date")
        .ok_or_else(|| Sigv4Error::MissingSignedHeader("x-amz-date".to_string()))?;
    if !amz_date.starts_with(&scope.date) {
        return Err(Sigv4Error::MalformedScope(format!(
            "scope date {} does not match request date {amz_date}",
            scope.date
        )));
    }

    // [FACT] A header the client named in `SignedHeaders` but did not send
    // is a missing header, not a bad signature. Saying so is the honest
    // answer, and it is a closed door either way: the signature could not
    // have been computed over a header that is not there.
    for name in signed_headers.split(';').filter(|s| !s.is_empty()) {
        if !req.headers.contains_key(name) {
            return Err(Sigv4Error::MissingSignedHeader(name.to_string()));
        }
    }

    let canonical_query = canonical_query_string(&parse_query_pairs(req.query));
    let canonical_uri = canonical_uri(req.path);
    // [FACT] The string to sign carries the *scope*, not the whole
    // credential: the access key is not part of it.
    let scope_string = scope.scope_string();
    let key = signing_key(
        &creds.secret_key,
        &scope.date,
        &scope.region,
        &scope.service,
    );

    // [FACT] The two header-value rules are both accepted, because the
    // rule a client signs with is the client's own business: AWS's rule
    // collapses an internal whitespace run and `rust-s3`'s rule does not.
    // Accepting both does not widen what a valid signature is — each
    // candidate is still an HMAC over the request under the secret key, so
    // a request that matches neither is refused exactly as before.
    for rule in [HeaderValueRule::Collapse, HeaderValueRule::TrimOnly] {
        let canonical = canonical_request(
            req.method,
            &canonical_uri,
            &canonical_query,
            req.headers,
            signed_headers,
            req.payload_sha256,
            rule,
        );
        let string_to_sign = string_to_sign(amz_date, &scope_string, &canonical);
        let expected = hex::encode(hmac_sha256(&key, string_to_sign.as_bytes()));
        if constant_time_eq(expected.as_bytes(), signature.as_bytes()) {
            return Ok(());
        }
    }
    Err(Sigv4Error::SignatureMismatch)
}

/// The parsed credential scope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CredentialScope {
    /// The access key.
    pub access_key: String,
    /// The date stamp, `YYYYMMDD`.
    pub date: String,
    /// The region the client signed for.
    pub region: String,
    /// The service the client signed for (`s3`).
    pub service: String,
    /// The terminator, `aws4_request`.
    pub terminator: String,
}

impl CredentialScope {
    /// Parse `<access-key>/<date>/<region>/<service>/<terminator>`.
    pub fn parse(credential: &str) -> Result<Self, Sigv4Error> {
        let parts: Vec<&str> = credential.split('/').collect();
        if parts.len() != 5 {
            return Err(Sigv4Error::MalformedScope(credential.to_string()));
        }
        Ok(Self {
            access_key: parts[0].to_string(),
            date: parts[1].to_string(),
            region: parts[2].to_string(),
            service: parts[3].to_string(),
            terminator: parts[4].to_string(),
        })
    }
}

impl std::fmt::Display for CredentialScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}/{}/{}/{}/{}",
            self.access_key, self.date, self.region, self.service, self.terminator
        )
    }
}

impl CredentialScope {
    /// The credential scope proper: `<date>/<region>/<service>/<terminator>`.
    ///
    /// [FACT] This — and not the whole credential, which also carries the
    /// access key — is what goes into the string to sign.
    pub fn scope_string(&self) -> String {
        format!(
            "{}/{}/{}/{}",
            self.date, self.region, self.service, self.terminator
        )
    }
}

/// Build the canonical request string.
///
/// [FACT] The shape is fixed by AWS:
/// `METHOD\nURI\nQUERY\nHEADERS\n\nSIGNED_HEADERS\nPAYLOAD_HASH`.
///
/// [FACT] `rule` says how a signed header's value is normalised. The two
/// rules produce the same canonical request for every header value that
/// has no internal run of whitespace, which is every value the client
/// this product ships actually signs.
pub fn canonical_request(
    method: &str,
    canonical_uri: &str,
    canonical_query: &str,
    headers: &BTreeMap<String, String>,
    signed_headers: &str,
    payload_sha256: &str,
    rule: HeaderValueRule,
) -> String {
    let mut lines: Vec<String> = Vec::new();
    for name in signed_headers.split(';').filter(|s| !s.is_empty()) {
        let value = headers
            .get(name)
            .map(|v| normalize_header_value(v, rule))
            .unwrap_or_default();
        lines.push(format!("{name}:{value}"));
    }
    format!(
        "{method}\n{canonical_uri}\n{canonical_query}\n{}\n\n{signed_headers}\n{payload_sha256}",
        lines.join("\n")
    )
}

/// Build the string to sign.
pub fn string_to_sign(amz_date: &str, scope: &str, canonical_request: &str) -> String {
    format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}",
        sha256_hex(canonical_request.as_bytes())
    )
}

/// Derive the SigV4 signing key.
///
/// [FACT] `service` is a parameter so that one implementation serves both
/// the published AWS test vectors (which use the generic service name
/// `service`) and this product (which uses `s3`).
pub fn signing_key(secret_key: &str, date: &str, region: &str, service: &str) -> [u8; 32] {
    let key = hmac_sha256(format!("AWS4{secret_key}").as_bytes(), date.as_bytes());
    let key = hmac_sha256(&key, region.as_bytes());
    let key = hmac_sha256(&key, service.as_bytes());
    hmac_sha256(&key, b"aws4_request")
}

/// HMAC-SHA256 (RFC 2104) over `sha2`.
///
/// [FACT] This crate has no HMAC dependency, so the construction is
/// spelled out here: `H(K⊕opad ‖ H(K⊕ipad ‖ msg))`.
pub fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    const BLOCK: usize = 64;
    let mut padded = [0u8; BLOCK];
    if key.len() > BLOCK {
        let digest = Sha256::digest(key);
        padded[..digest.len()].copy_from_slice(&digest);
    } else {
        padded[..key.len()].copy_from_slice(key);
    }

    let mut inner_pad = [0x36u8; BLOCK];
    let mut outer_pad = [0x5cu8; BLOCK];
    for i in 0..BLOCK {
        inner_pad[i] ^= padded[i];
        outer_pad[i] ^= padded[i];
    }

    let mut inner = Sha256::new();
    inner.update(inner_pad);
    inner.update(message);
    let inner_digest = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(outer_pad);
    outer.update(inner_digest);
    outer.finalize().into()
}

/// Hex-encoded SHA-256.
pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// The canonical URI for a raw request path.
///
/// [FACT] `rust-s3` decodes the URL path and re-encodes it with the
/// unreserved set only, so `/` survives and everything else is escaped.
pub fn canonical_uri(raw_path: &str) -> String {
    uri_encode(&percent_decode(raw_path), false)
}

/// Percent-encode a string the way AWS does.
///
/// `encode_slash` decides whether `/` is escaped; AWS escapes it in query
/// components and keeps it in paths.
pub fn uri_encode(value: &str, encode_slash: bool) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(*byte as char);
        } else if *byte == b'/' && !encode_slash {
            out.push('/');
        } else {
            out.push('%');
            out.push_str(&format!("{byte:02X}"));
        }
    }
    out
}

/// Percent-decode a string. Invalid UTF-8 becomes the replacement
/// character, exactly like `percent_encoding::percent_decode_str(..)
/// .decode_utf8_lossy()` which `rust-s3` uses.
pub fn percent_decode(value: &str) -> String {
    decode_bytes(value, false)
}

/// Decode an `application/x-www-form-urlencoded` component: `+` is a
/// space and `%XX` is a byte.
fn form_decode(value: &str) -> String {
    decode_bytes(value, true)
}

fn decode_bytes(value: &str, plus_is_space: bool) -> String {
    let bytes = value.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'%' && index + 2 < bytes.len() {
            if let (Some(high), Some(low)) =
                (hex_value(bytes[index + 1]), hex_value(bytes[index + 2]))
            {
                out.push(high * 16 + low);
                index += 3;
                continue;
            }
        }
        if plus_is_space && byte == b'+' {
            out.push(b' ');
            index += 1;
            continue;
        }
        out.push(byte);
        index += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Split a raw query string into decoded (name, value) pairs.
///
/// [FACT] `rust-s3` parses the query with `url::Url::query_pairs`, which is
/// `application/x-www-form-urlencoded`: `+` decodes to a space.
pub fn parse_query_pairs(raw_query: &str) -> Vec<(String, String)> {
    raw_query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| match pair.find('=') {
            Some(index) => (form_decode(&pair[..index]), form_decode(&pair[index + 1..])),
            None => (form_decode(pair), String::new()),
        })
        .collect()
}

/// Build the canonical query string.
///
/// [FACT] Each name and value is re-encoded with the unreserved set only
/// (so `/` becomes `%2F`), the pairs are sorted by (name, value), and
/// `X-Amz-Signature` is left out — it is the signature, not an input to it.
pub fn canonical_query_string(pairs: &[(String, String)]) -> String {
    let mut encoded: Vec<(String, String)> = pairs
        .iter()
        .filter(|(name, _)| name != "X-Amz-Signature")
        .map(|(name, value)| (uri_encode(name, true), uri_encode(value, true)))
        .collect();
    encoded.sort();
    encoded
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join("&")
}

/// Normalise a header value for the canonical request.
///
/// [FACT] Both rules trim the ends. They differ in what they do to a run
/// of whitespace *inside* the value:
///
/// * [`HeaderValueRule::Collapse`] — AWS's rule: the run becomes one
///   space. This is what the published `get-header-value-trim` vector
///   pins down (`"a   b   c"` becomes `"a b c"`).
/// * [`HeaderValueRule::TrimOnly`] — `rust-s3` 0.38's rule: the inside of
///   the value is left exactly as it arrived (`"a   b   c"` stays as it
///   is). It signs with `value.to_str()?.trim()` and nothing more.
///
/// [FACT] The server accepts both, because the rule a client signs with
/// is the client's own business. A request that matches neither is
/// refused.
fn normalize_header_value(value: &str, rule: HeaderValueRule) -> String {
    let trimmed = value.trim();
    if rule == HeaderValueRule::TrimOnly {
        return trimmed.to_string();
    }
    let mut out = String::with_capacity(trimmed.len());
    let mut in_space = false;
    for ch in trimmed.chars() {
        if ch.is_whitespace() {
            if !in_space {
                out.push(' ');
                in_space = true;
            }
        } else {
            out.push(ch);
            in_space = false;
        }
    }
    out
}

/// Compare two byte strings without an early exit.
fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in left.iter().zip(right.iter()) {
        diff |= a ^ b;
    }
    diff == 0
}

/// A parsed `X-Amz-Date` / `x-amz-date` timestamp.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AmzDate {
    /// Seconds since the Unix epoch.
    pub epoch_secs: i64,
    /// The `YYYYMMDD` date stamp.
    pub date_stamp: String,
}

/// Parse the `YYYYMMDDTHHMMSSZ` timestamp format AWS uses.
pub fn parse_amz_date(value: &str) -> Option<AmzDate> {
    let bytes = value.as_bytes();
    if bytes.len() != 16 || bytes[8] != b'T' || bytes[15] != b'Z' {
        return None;
    }
    let year: i64 = value.get(0..4)?.parse().ok()?;
    let month: u32 = value.get(4..6)?.parse().ok()?;
    let day: u32 = value.get(6..8)?.parse().ok()?;
    let hour: u32 = value.get(9..11)?.parse().ok()?;
    let minute: u32 = value.get(11..13)?.parse().ok()?;
    let second: u32 = value.get(13..15)?.parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let days = days_from_civil(year, month, day);
    let epoch_secs =
        days * 86_400 + i64::from(hour) * 3_600 + i64::from(minute) * 60 + i64::from(second);
    Some(AmzDate {
        epoch_secs,
        date_stamp: value.get(0..8)?.to_string(),
    })
}

/// Days between the Unix epoch and a civil date (Howard Hinnant's
/// `days_from_civil`).
pub fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = (year - era * 400) as u64;
    let month = if month > 2 { month - 3 } else { month + 9 } as u64;
    let day_of_year = (153 * month + 2) / 5 + u64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era as i64 - 719_468
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// [FACT] The credentials must not leak through `Debug`.
    ///
    /// The repository's own rule for credentials is written down on
    /// `S3Vault`, which redacts the secret and has
    /// `s3_vault_debug_does_not_leak_credentials` to hold it in place.
    /// This is the same rule on the server side of the same credential.
    #[test]
    fn the_debug_print_of_the_credentials_does_not_leak_the_secret() {
        let creds = Sigv4Credentials::new("an-access-key", "a-secret-key");
        let printed = format!("{creds:?}");
        assert!(
            !printed.contains("a-secret-key"),
            "the secret key must not appear in the debug print, got: {printed}"
        );
        assert!(
            printed.contains("an-access-key"),
            "the access key is an identifier, not a secret, so it may be printed: {printed}"
        );
    }

    // [FACT] The published AWS SigV4 test-suite vectors. They live in
    // AWS's own `aws4_testsuite` and are mirrored in botocore at
    // `tests/unit/auth/aws4_testsuite/<case>/<case>.{creq,authz,sts}`.
    //
    // [FACT] The suite signs with the *generic* service name `service`,
    // which is why `signing_key` takes the service as a parameter: the
    // product signs with `s3`, the vectors with `service`, and one
    // implementation serves both.
    const TEST_ACCESS_KEY: &str = "AKIDEXAMPLE";
    const TEST_SECRET_KEY: &str = "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY";
    const TEST_REGION: &str = "us-east-1";
    const TEST_DATE_STAMP: &str = "20150830";
    const TEST_AMZ_DATE: &str = "20150830T123600Z";
    const TEST_SCOPE: &str = "20150830/us-east-1/service/aws4_request";

    fn headers(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_lowercase(), v.to_string()))
            .collect()
    }

    /// Sign a canonical request the way AWS does, so the test can compare
    /// the result against a published expected signature.
    fn sign(
        secret: &str,
        date_stamp: &str,
        region: &str,
        service: &str,
        amz_date: &str,
        canonical: &str,
    ) -> String {
        let scope = format!("{date_stamp}/{region}/{service}/aws4_request");
        let sts = string_to_sign(amz_date, &scope, canonical);
        let key = signing_key(secret, date_stamp, region, service);
        hex::encode(hmac_sha256(&key, sts.as_bytes()))
    }

    #[test]
    fn hmac_matches_the_rfc_2104_test_case() {
        // [FACT] RFC 2104 §4 test case 1: key = 20 bytes of 0x0b,
        // data = "Hi There", digest = b0344c61d8db38535ca8afceaf0bf12b.
        let key = [0x0bu8; 20];
        let digest = hmac_sha256(&key, b"Hi There");
        assert_eq!(
            hex::encode(digest),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }

    #[test]
    fn empty_payload_hash_is_the_canonical_empty_digest() {
        assert_eq!(sha256_hex(b""), EMPTY_PAYLOAD_SHA256);
    }

    // ---- the two header-value rules -------------------------------------

    /// [FACT] A request signed under **either** header-value rule has to
    /// verify. AWS's rule collapses a run of internal whitespace and
    /// `rust-s3`'s rule leaves it alone; which one a client uses is the
    /// client's own business, and the server refuses to decide for it.
    ///
    /// [FACT] The value used here is `"a   b   c"` — the one shape the two
    /// rules disagree about.
    #[test]
    fn a_request_signed_under_either_header_value_rule_verifies() {
        let creds = Sigv4Credentials::new(TEST_ACCESS_KEY, TEST_SECRET_KEY);
        let map = headers(&[
            ("host", "example.amazonaws.com"),
            ("x-amz-date", TEST_AMZ_DATE),
            ("my-header2", "\"a   b   c\""),
        ]);
        let signed_headers = "host;my-header2;x-amz-date";
        let scope = format!("{TEST_DATE_STAMP}/{TEST_REGION}/s3/aws4_request");
        let credential = format!("{TEST_ACCESS_KEY}/{scope}");
        let request = SignedRequest {
            method: "GET",
            path: "/",
            query: "",
            headers: &map,
            payload_sha256: EMPTY_PAYLOAD_SHA256,
        };

        for rule in [HeaderValueRule::Collapse, HeaderValueRule::TrimOnly] {
            let canonical = canonical_request(
                "GET",
                "/",
                "",
                &map,
                signed_headers,
                EMPTY_PAYLOAD_SHA256,
                rule,
            );
            let signature = sign(
                TEST_SECRET_KEY,
                TEST_DATE_STAMP,
                TEST_REGION,
                "s3",
                TEST_AMZ_DATE,
                &canonical,
            );
            assert_eq!(
                verify(&creds, &request, &signature, signed_headers, &credential),
                Ok(()),
                "a request signed under the {rule:?} rule must verify"
            );
        }
    }

    /// [FACT] The two rules really do differ, so the test above is not
    /// passing for the wrong reason.
    #[test]
    fn the_two_header_value_rules_disagree_about_an_internal_whitespace_run() {
        let map = headers(&[("my-header2", "\"a   b   c\"")]);
        let collapsed = canonical_request(
            "GET",
            "/",
            "",
            &map,
            "my-header2",
            EMPTY_PAYLOAD_SHA256,
            HeaderValueRule::Collapse,
        );
        let trimmed = canonical_request(
            "GET",
            "/",
            "",
            &map,
            "my-header2",
            EMPTY_PAYLOAD_SHA256,
            HeaderValueRule::TrimOnly,
        );
        assert!(collapsed.contains("\"a b c\""));
        assert!(trimmed.contains("\"a   b   c\""));
        assert_ne!(collapsed, trimmed);
    }

    /// [FACT] A header the client named in `SignedHeaders` but did not
    /// send is a **missing header**, not a bad signature. The gate has to
    /// say what is wrong with the request.
    #[test]
    fn a_signed_header_that_did_not_arrive_is_reported_as_missing() {
        let creds = Sigv4Credentials::new(TEST_ACCESS_KEY, TEST_SECRET_KEY);
        let map = headers(&[
            ("host", "example.amazonaws.com"),
            ("x-amz-date", TEST_AMZ_DATE),
        ]);
        let request = SignedRequest {
            method: "GET",
            path: "/",
            query: "",
            headers: &map,
            payload_sha256: EMPTY_PAYLOAD_SHA256,
        };
        let credential =
            format!("{TEST_ACCESS_KEY}/{TEST_DATE_STAMP}/{TEST_REGION}/s3/aws4_request");
        assert_eq!(
            verify(
                &creds,
                &request,
                "0000000000000000000000000000000000000000000000000000000000000000",
                "host;x-amz-date;x-not-sent",
                &credential,
            ),
            Err(Sigv4Error::MissingSignedHeader("x-not-sent".to_string()))
        );
    }

    // ---- published AWS vectors -----------------------------------------

    #[test]
    fn aws_vector_get_vanilla() {
        // [FACT] AWS `aws4_testsuite/get-vanilla`.
        let expected_canonical = concat!(
            "GET\n",
            "/\n",
            "\n",
            "host:example.amazonaws.com\n",
            "x-amz-date:20150830T123600Z\n",
            "\n",
            "host;x-amz-date\n",
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        let map = headers(&[
            ("host", "example.amazonaws.com"),
            ("x-amz-date", TEST_AMZ_DATE),
        ]);
        let canonical = canonical_request(
            "GET",
            "/",
            "",
            &map,
            "host;x-amz-date",
            EMPTY_PAYLOAD_SHA256,
            HeaderValueRule::Collapse,
        );
        assert_eq!(canonical, expected_canonical);

        let sts = string_to_sign(TEST_AMZ_DATE, TEST_SCOPE, &canonical);
        assert_eq!(
            sts,
            concat!(
                "AWS4-HMAC-SHA256\n",
                "20150830T123600Z\n",
                "20150830/us-east-1/service/aws4_request\n",
                "bb579772317eb040ac9ed261061d46c1f17a8133879d6129b6e1c25292927e63"
            )
        );

        let signature = sign(
            TEST_SECRET_KEY,
            TEST_DATE_STAMP,
            TEST_REGION,
            "service",
            TEST_AMZ_DATE,
            &canonical,
        );
        assert_eq!(
            signature,
            "5fa00fa31553b73ebf1942676e86291e8372ff2a2260956d9b8aae1d763fbf31"
        );
    }

    #[test]
    fn aws_vector_get_vanilla_query_order_key_case() {
        // [FACT] AWS `aws4_testsuite/get-vanilla-query-order-key-case`: the
        // request carries `Param2=value2&Param1=value1` and the canonical
        // query sorts them by name.
        let expected_canonical = concat!(
            "GET\n",
            "/\n",
            "Param1=value1&Param2=value2\n",
            "host:example.amazonaws.com\n",
            "x-amz-date:20150830T123600Z\n",
            "\n",
            "host;x-amz-date\n",
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        let map = headers(&[
            ("host", "example.amazonaws.com"),
            ("x-amz-date", TEST_AMZ_DATE),
        ]);
        let pairs = parse_query_pairs("Param2=value2&Param1=value1");
        let canonical = canonical_request(
            "GET",
            "/",
            &canonical_query_string(&pairs),
            &map,
            "host;x-amz-date",
            EMPTY_PAYLOAD_SHA256,
            HeaderValueRule::Collapse,
        );
        assert_eq!(canonical, expected_canonical);

        let signature = sign(
            TEST_SECRET_KEY,
            TEST_DATE_STAMP,
            TEST_REGION,
            "service",
            TEST_AMZ_DATE,
            &canonical,
        );
        assert_eq!(
            signature,
            "b97d918cfa904a5beff61c982a1b6f458b799221646efd99d3219ec94cdf2500"
        );
    }

    #[test]
    fn aws_vector_post_vanilla() {
        // [FACT] AWS `aws4_testsuite/post-vanilla`.
        let map = headers(&[
            ("host", "example.amazonaws.com"),
            ("x-amz-date", TEST_AMZ_DATE),
        ]);
        let canonical = canonical_request(
            "POST",
            "/",
            "",
            &map,
            "host;x-amz-date",
            EMPTY_PAYLOAD_SHA256,
            HeaderValueRule::Collapse,
        );
        let expected_canonical = concat!(
            "POST\n",
            "/\n",
            "\n",
            "host:example.amazonaws.com\n",
            "x-amz-date:20150830T123600Z\n",
            "\n",
            "host;x-amz-date\n",
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(canonical, expected_canonical);

        let signature = sign(
            TEST_SECRET_KEY,
            TEST_DATE_STAMP,
            TEST_REGION,
            "service",
            TEST_AMZ_DATE,
            &canonical,
        );
        assert_eq!(
            signature,
            "5da7c1a2acd57cee7505fc6676e4e544621c30862966e37dddb68e92efbe5d6b"
        );
    }

    #[test]
    fn aws_vector_get_header_value_trim() {
        // [FACT] AWS `aws4_testsuite/get-header-value-trim`: the request
        // sends `My-Header2: "a   b   c"` and the canonical request spells
        // it `"a b c"` — trimmed, and internal runs of spaces collapsed.
        let map = headers(&[
            ("host", "example.amazonaws.com"),
            ("my-header1", "value1"),
            ("my-header2", "\"a   b   c\""),
            ("x-amz-date", TEST_AMZ_DATE),
        ]);
        let canonical = canonical_request(
            "GET",
            "/",
            "",
            &map,
            "host;my-header1;my-header2;x-amz-date",
            EMPTY_PAYLOAD_SHA256,
            HeaderValueRule::Collapse,
        );
        let expected_canonical = concat!(
            "GET\n",
            "/\n",
            "\n",
            "host:example.amazonaws.com\n",
            "my-header1:value1\n",
            "my-header2:\"a b c\"\n",
            "x-amz-date:20150830T123600Z\n",
            "\n",
            "host;my-header1;my-header2;x-amz-date\n",
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(canonical, expected_canonical);

        let signature = sign(
            TEST_SECRET_KEY,
            TEST_DATE_STAMP,
            TEST_REGION,
            "service",
            TEST_AMZ_DATE,
            &canonical,
        );
        assert_eq!(
            signature,
            "acc3ed3afb60bb290fc8d2dd0098b9911fcaa05412b367055dee359757a9c736"
        );
    }

    #[test]
    fn aws_vector_get_unreserved_keeps_the_unreserved_set() {
        // [FACT] AWS `aws4_testsuite/get-unreserved`: the unreserved
        // characters (`-._~` and the alphanumerics) survive unescaped.
        let path = "/-._~0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
        assert_eq!(canonical_uri(path), path);
        let map = headers(&[
            ("host", "example.amazonaws.com"),
            ("x-amz-date", TEST_AMZ_DATE),
        ]);
        let canonical = canonical_request(
            "GET",
            &canonical_uri(path),
            "",
            &map,
            "host;x-amz-date",
            EMPTY_PAYLOAD_SHA256,
            HeaderValueRule::Collapse,
        );
        let signature = sign(
            TEST_SECRET_KEY,
            TEST_DATE_STAMP,
            TEST_REGION,
            "service",
            TEST_AMZ_DATE,
            &canonical,
        );
        assert_eq!(
            signature,
            "07ef7494c76fa4850883e2b006601f940f8a34d404d0cfa977f52a65bbf5f24f"
        );
    }

    #[test]
    fn aws_vector_get_utf8_encodes_the_path_as_utf8() {
        // [FACT] AWS `aws4_testsuite/get-utf8`: the path `ሴ` is
        // canonicalised as the percent-encoded UTF-8 bytes.
        let canonical = canonical_uri("/%E1%88%B4");
        assert_eq!(canonical, "/%E1%88%B4");
        let map = headers(&[
            ("host", "example.amazonaws.com"),
            ("x-amz-date", TEST_AMZ_DATE),
        ]);
        let canonical_request_string = canonical_request(
            "GET",
            &canonical,
            "",
            &map,
            "host;x-amz-date",
            EMPTY_PAYLOAD_SHA256,
            HeaderValueRule::Collapse,
        );
        let signature = sign(
            TEST_SECRET_KEY,
            TEST_DATE_STAMP,
            TEST_REGION,
            "service",
            TEST_AMZ_DATE,
            &canonical_request_string,
        );
        assert_eq!(
            signature,
            "8318018e0b0f223aa2bbf98705b62bb787dc9c0e678f255a891fd03141be5d85"
        );
    }

    #[test]
    fn s3_documentation_vector_uses_the_s3_service_name() {
        // [FACT] The "Example: GET Object" vector from the AWS S3 API
        // reference ("Signature Calculation for the Authorization Header"),
        // which signs with the service name `s3` — the name this product
        // uses. Its secret key is the one the reference pairs with that
        // signature.
        let map = headers(&[
            ("host", "examplebucket.s3.amazonaws.com"),
            ("range", "bytes=0-9"),
            ("x-amz-content-sha256", EMPTY_PAYLOAD_SHA256),
            ("x-amz-date", "20130524T000000Z"),
        ]);
        let canonical = canonical_request(
            "GET",
            "/test.txt",
            "",
            &map,
            "host;range;x-amz-content-sha256;x-amz-date",
            EMPTY_PAYLOAD_SHA256,
            HeaderValueRule::Collapse,
        );
        let expected_canonical = concat!(
            "GET\n",
            "/test.txt\n",
            "\n",
            "host:examplebucket.s3.amazonaws.com\n",
            "range:bytes=0-9\n",
            "x-amz-content-sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855\n",
            "x-amz-date:20130524T000000Z\n",
            "\n",
            "host;range;x-amz-content-sha256;x-amz-date\n",
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(canonical, expected_canonical);

        let signature = sign(
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            "20130524",
            "us-east-1",
            "s3",
            "20130524T000000Z",
            &canonical,
        );
        assert_eq!(
            signature,
            "f0e8bdb87c964420e857bd35b5d6ed310bd44f0170aba48dd91039c6036bdb41"
        );
    }

    // ---- encoding rules -------------------------------------------------

    #[test]
    fn query_encoding_escapes_the_slash_and_leaves_the_unreserved_set() {
        assert_eq!(uri_encode("a/b", true), "a%2Fb");
        assert_eq!(uri_encode("a/b", false), "a/b");
        assert_eq!(uri_encode("~-_.", true), "~-_.");
        assert_eq!(uri_encode(" ", true), "%20");
        assert_eq!(uri_encode("+", true), "%2B");
    }

    #[test]
    fn query_pairs_treat_plus_as_a_space() {
        let pairs = parse_query_pairs("a=1+2&b=%2F&c");
        assert_eq!(
            pairs,
            vec![
                ("a".to_string(), "1 2".to_string()),
                ("b".to_string(), "/".to_string()),
                ("c".to_string(), String::new()),
            ]
        );
        assert_eq!(canonical_query_string(&pairs), "a=1%202&b=%2F&c=");
    }

    #[test]
    fn the_signature_parameter_is_not_part_of_the_canonical_query() {
        let pairs = parse_query_pairs("X-Amz-Signature=deadbeef&X-Amz-Date=20150830T123600Z");
        assert_eq!(
            canonical_query_string(&pairs),
            "X-Amz-Date=20150830T123600Z"
        );
    }

    #[test]
    fn amz_dates_parse_into_epoch_seconds() {
        let parsed = parse_amz_date("20150830T123600Z").unwrap();
        assert_eq!(parsed.date_stamp, "20150830");
        assert_eq!(parsed.epoch_secs, 1_440_938_160);
        assert!(parse_amz_date("nonsense").is_none());
    }

    #[test]
    fn credential_scope_round_trips() {
        let scope =
            CredentialScope::parse("AKIDEXAMPLE/20150830/us-east-1/s3/aws4_request").unwrap();
        assert_eq!(scope.access_key, "AKIDEXAMPLE");
        assert_eq!(scope.date, "20150830");
        assert_eq!(scope.region, "us-east-1");
        assert_eq!(scope.service, "s3");
        assert_eq!(scope.terminator, "aws4_request");
        assert_eq!(
            scope.to_string(),
            "AKIDEXAMPLE/20150830/us-east-1/s3/aws4_request"
        );
        assert_eq!(
            scope.scope_string(),
            "20150830/us-east-1/s3/aws4_request",
            "the string to sign carries the scope, not the access key"
        );
        assert!(CredentialScope::parse("too/few").is_err());
    }

    #[test]
    fn an_authorization_header_in_the_clients_shape_is_accepted() {
        // [FACT] `rust-s3` 0.38 writes the header as
        // `AWS4-HMAC-SHA256 Credential=…,SignedHeaders=…,Signature=…`:
        // the algorithm is separated from the first parameter by a
        // **space**, and the parameters by commas. Parsing it as if the
        // algorithm were comma-separated rejects every real request, so
        // this test pins the shape.
        let credentials = Sigv4Credentials::new(TEST_ACCESS_KEY, TEST_SECRET_KEY);
        let mut map = headers(&[
            ("host", "example.amazonaws.com"),
            ("x-amz-date", TEST_AMZ_DATE),
        ]);
        map.insert(
            "x-amz-content-sha256".to_string(),
            EMPTY_PAYLOAD_SHA256.to_string(),
        );

        let canonical = canonical_request(
            "GET",
            "/",
            "",
            &map,
            "host;x-amz-content-sha256;x-amz-date",
            EMPTY_PAYLOAD_SHA256,
            HeaderValueRule::Collapse,
        );
        let scope = format!("{TEST_DATE_STAMP}/{TEST_REGION}/s3/aws4_request");
        let string_to_sign = string_to_sign(TEST_AMZ_DATE, &scope, &canonical);
        let key = signing_key(TEST_SECRET_KEY, TEST_DATE_STAMP, TEST_REGION, "s3");
        let signature = hex::encode(hmac_sha256(&key, string_to_sign.as_bytes()));

        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={TEST_ACCESS_KEY}/{scope},SignedHeaders=host;x-amz-content-sha256;x-amz-date,Signature={signature}"
        );
        let request = SignedRequest {
            method: "GET",
            path: "/",
            query: "",
            headers: &map,
            payload_sha256: EMPTY_PAYLOAD_SHA256,
        };
        assert_eq!(
            verify_header(&credentials, &request, &authorization),
            Ok(())
        );

        // The same header with a space after each comma — the shape the
        // AWS documentation prints — is accepted as well.
        let spaced = authorization
            .replace(",SignedHeaders", ", SignedHeaders")
            .replace(",Signature", ", Signature");
        assert_eq!(verify_header(&credentials, &request, &spaced), Ok(()));

        // And a wrong signature is still rejected.
        let tampered = authorization.replace(&signature, &"0".repeat(64));
        assert_eq!(
            verify_header(&credentials, &request, &tampered),
            Err(Sigv4Error::SignatureMismatch)
        );
    }
}
