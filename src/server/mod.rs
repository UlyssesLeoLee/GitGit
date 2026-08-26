//! HTTP server for the `gitgit` MVP.

pub mod auth;
pub mod http;
pub mod smart;
pub mod subprocess;

pub use http::{build_router, AppState};
