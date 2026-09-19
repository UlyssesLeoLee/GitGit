//! Tauri command handlers grouped by domain.
//!
//! Every command returns `Result<T, AppError>`; the generated
//! TypeScript binding sees `AppError` as `{ kind, message }` per the
//! `Serialize` impl in `crate::error`.

pub mod auth;
pub mod repos;
pub mod server;
pub mod system;
pub mod vault;
