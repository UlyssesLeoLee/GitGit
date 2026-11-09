//! Bare-repo storage helpers.

pub mod conflict;
pub mod refs;
pub mod status;
pub mod store;

pub use conflict::{
    compose_resolved, conflict_set, conflict_set_from_blobs, parse_merge_output, region_id,
    scope_map, write_resolved, ConflictSet, Disposition, LineRange, Region, RegionChoice,
    RegionConfidence, ScopeConfidence, ScopeKind, ScopeMap, ScopeRef,
};
pub use status::{
    is_bare_repo, parse_porcelain_v1, parse_status_line, resolve_worktree, worktree_diff,
    worktree_status, DiffPayload, DiffTarget, StatusEntry, WorktreeStatus, MAX_DIFF_BYTES,
};
pub use store::{create_bare_repo, list_repos};
