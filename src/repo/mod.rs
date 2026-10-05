//! Bare-repo storage helpers.

pub mod refs;
pub mod status;
pub mod store;

pub use status::{
    is_bare_repo, parse_porcelain_v1, parse_status_line, resolve_worktree, worktree_diff,
    worktree_status, DiffPayload, DiffTarget, StatusEntry, WorktreeStatus, MAX_DIFF_BYTES,
};
pub use store::{create_bare_repo, list_repos};
