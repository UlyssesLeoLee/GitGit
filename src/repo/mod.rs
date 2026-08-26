//! Bare-repo storage helpers.

pub mod refs;
pub mod store;

pub use store::{create_bare_repo, list_repos};
