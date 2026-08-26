//! HTTP Basic auth for the smart-HTTP receive-pack endpoint.
//!
//! Per the brief, only one hard-coded user (`admin` / `admin`) is supported.
//! Reads are open; writes require auth.

use axum::http::header::AUTHORIZATION;
use axum::http::HeaderMap;

use crate::config::{ADMIN_PASS, ADMIN_USER};
use crate::error::{GitGitError, Result};

/// `true` iff the `Authorization` header carries valid `admin:admin` basic
/// credentials.
pub fn check_basic(headers: &HeaderMap) -> Result<bool> {
    let Some(value) = headers.get(AUTHORIZATION) else {
        return Ok(false);
    };
    let Ok(value) = value.to_str() else {
        return Ok(false);
    };
    let Some(rest) = value.strip_prefix("Basic ") else {
        return Ok(false);
    };
    let bytes = match base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        rest.trim(),
    ) {
        Ok(b) => b,
        Err(_) => return Ok(false),
    };
    let s = match std::str::from_utf8(&bytes) {
        Ok(s) => s,
        Err(_) => return Ok(false),
    };
    let Some((user, pass)) = s.split_once(':') else {
        return Ok(false);
    };
    Ok(user == ADMIN_USER && pass == ADMIN_PASS)
}

/// Enforce basic auth. Returns [`GitGitError::Unauthenticated`] if missing
/// or wrong, which the HTTP layer translates to 401.
pub fn require_basic(headers: &HeaderMap) -> Result<()> {
    if check_basic(headers)? {
        Ok(())
    } else {
        Err(GitGitError::Unauthenticated)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    use base64::Engine;

    fn hdrs(value: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(AUTHORIZATION, HeaderValue::from_str(value).unwrap());
        h
    }

    #[test]
    fn missing_header_is_denied() {
        let h = HeaderMap::new();
        assert!(!check_basic(&h).unwrap());
        assert!(require_basic(&h).is_err());
    }

    #[test]
    fn correct_credentials_pass() {
        let v = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(b"admin:admin")
        );
        let h = hdrs(&v);
        assert!(check_basic(&h).unwrap());
        assert!(require_basic(&h).is_ok());
    }

    #[test]
    fn wrong_password_is_denied() {
        let v = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(b"admin:nope")
        );
        let h = hdrs(&v);
        assert!(!check_basic(&h).unwrap());
    }

    #[test]
    fn malformed_authorization_is_denied() {
        let h = hdrs("Bearer xyz");
        assert!(!check_basic(&h).unwrap());
    }
}
