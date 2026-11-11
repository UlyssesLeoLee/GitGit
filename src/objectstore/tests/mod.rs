//! The object store's test suite, in the three layers the brief names:
//!
//! 1. [`storage`] — the filesystem backend, with no HTTP and no
//!    signatures, so a failure there is unambiguously a storage bug;
//! 2. [`sigv4_client`] — the real `rust-s3` client signing requests that
//!    this server verifies, plus the published AWS vectors in
//!    `sigv4`'s own unit tests;
//! 3. [`vault_e2e`] — the credential vault, unchanged, pointed at this
//!    server.
//!
//! [`http_surface`] covers the HTTP surface itself and the three failure
//! modes (the server that cannot start, a signature that does not match,
//! and a malformed listing).

mod common;
mod http_surface;
mod sigv4_client;
mod storage;
mod vault_e2e;
