//! HTTP server for the `gitgit` MVP.

pub mod api;
pub mod auth;
pub mod http;
pub mod smart;
pub mod subprocess;
pub mod vault;
pub mod vault_versioned;

pub use http::{build_router, AppState};
