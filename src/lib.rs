//! `gitgit` library crate.
//!
//! Exists so that companion tooling (today: `apps/gm-desktop/src-tauri/`,
//! tomorrow: anything else) can reuse the same vault / config / subprocess
//! helpers that the `gitgit` CLI binary already ships. The implementation
//! is identical to the binary path: `src/main.rs` still owns CLI dispatch,
//! this file only re-exports the existing public modules so external
//! callers see them through `gitgit::...` rather than through
//! crate-internal `mod` declarations.
//!
//! Per the V0 desktop-shell worker brief, this lib surface is the
//! integration boundary. The brief forbids modifying
//! `vault / auth / http`; this file does neither — it simply compiles the
//! existing source files into a library target.

pub mod cli;
pub mod config;
pub mod error;
pub mod repo;
pub mod server;
