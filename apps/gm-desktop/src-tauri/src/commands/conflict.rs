//! The conflict regions of a repository (ADR-0024 §3.1).
//!
//! `[FACT]` The regions come from the Rust core in `src/repo/conflict.rs`
//! — the parse of `git merge-file`'s output ✓ — and this command is the
//! desktop app's way of reaching it ✓.
//!
//! `[FACT]` A region is read from the **index** of a work tree that is in
//! the middle of a merge ✓: `git ls-files -u` lists one record per stage
//! of every conflicted path ✓ — stage 1 is the base, 2 is ours and 3 is
//! theirs ✓ (ADR §2.3) ✓ — and the core's `conflict_set` turns those
//! three blobs into the regions ✓.
//!
//! `[FACT]` The desktop keeps the bare repositories under `<name>.git` ✓
//! and a work tree per repository beside them ✓; the bare repository has
//! no index ✓, so the regions are read from the work tree ✓ — the one
//! that holds the merge in progress ✓.

use std::path::Path;

use serde::Serialize;
use tauri::State;

use crate::commands::repos::resolve_worktree_dir;
use crate::error::{AppError, AppResult};
use crate::state::DesktopState;

/// How many lines of context the webview gets on each side of a region.
const CONTEXT_LINES: usize = 3;

/// The conflict regions of `repo`, and the paths git has to handle itself.
///
/// `[FACT]` Two lists, because they answer two different questions ✓: the
/// regions the user can arbitrate here ✓, and the paths the core refused
/// to read ✓ — an add/delete or rename conflict has no base stage ✓, and
/// binary content is not text ✓ (ADR §3.7.4) ✓. Those paths are **not**
/// dropped ✓: a page that said "no conflict regions" while git still had
/// work to do would be lying ✓.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictRegionsDto {
    /// The regions, in the order the merged output produced them.
    pub regions: Vec<ConflictRegionDto>,
    /// The repository-relative paths the core leaves to git.
    pub needs_git: Vec<String>,
}

/// One conflict region, as the webview holds it.
///
/// `[FACT]` The field names mirror `ConflictRegion` in
/// `apps/gm-desktop/src/lib/api/types.ts` ✓, and the serde attribute
/// makes them camelCase on the wire ✓ — the same convention
/// `commands/ai.rs` uses for its own DTOs ✓.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictRegionDto {
    /// The region's stable id (ADR §3.7.2).
    pub id: String,
    /// The repository-relative path of the conflicted file.
    pub path: String,
    /// The base blob's object id, when the base was available.
    pub base_oid: String,
    /// The region's ordinal in the merged output.
    pub index: usize,
    /// The ours side's text, verbatim.
    pub ours: String,
    /// The base side's text, verbatim.
    pub base: String,
    /// The theirs side's text, verbatim.
    pub theirs: String,
    /// The base file's lines before the region.
    pub context_before: String,
    /// The base file's lines after the region.
    pub context_after: String,
    /// The scope the region lives in, as a human label (ADR §3.8).
    pub scope_label: String,
}

/// The conflict regions of `repo`.
///
/// `[FACT]` Every unmerged path in the work tree is read ✓, and each
/// path's regions are the ones `git merge-file` produced ✓ — the scope
/// layer never removes one ✓ (ADR §3.8.3) ✓.
///
/// `[FACT]` `root` is the working-tree root the repository name is
/// resolved under ✓ — the same argument `repo_status` and `repo_diff`
/// take ✓ — because the desktop keeps its repositories as bare
/// `<name>.git` directories ✓ and the checkouts live under the root the
/// user picked ✓. Without it the command would resolve a bare repository
/// ✓, and `git` answers `NotAWorkTree` there ✓.
///
/// `[FACT]` A repository with no merge in progress has no unmerged paths ✓
/// and therefore no regions ✓ — an empty list ✓, not an error ✓.
#[tauri::command]
pub async fn conflict_regions(
    state: State<'_, DesktopState>,
    repo: String,
    root: Option<String>,
) -> AppResult<ConflictRegionsDto> {
    let dir = resolve_worktree_dir(&state.repos_dir, &repo, root)?;
    let paths = unmerged_paths(&dir).await?;
    let mut regions = Vec::new();
    let mut needs_git = Vec::new();
    for rel in paths {
        let set = gitgit::repo::conflict_set(&dir, &rel)
            .await
            .map_err(|e| AppError::Gitgit(e.to_string()))?;
        if set.disposition == gitgit::repo::Disposition::NeedsGit {
            // `[FACT]` The core refused this path ✓ — no base stage ✓, or
            // binary content ✓. It is reported, not dropped ✓.
            needs_git.push(set.path.clone());
            continue;
        }
        let base_text = match &set.base_oid {
            Some(oid) => blob_text(&dir, oid).await?,
            None => String::new(),
        };
        let base_lines: Vec<&str> = base_text.lines().collect();
        for region in set.all_regions() {
            regions.push(ConflictRegionDto {
                id: region.id.clone(),
                path: set.path.clone(),
                base_oid: set.base_oid.clone().unwrap_or_default(),
                index: region.index,
                ours: region.ours.clone(),
                base: region.base.clone(),
                theirs: region.theirs.clone(),
                context_before: context_before(&base_lines, region.base_range.start),
                context_after: context_after(&base_lines, region.base_range.end),
                scope_label: region.scope.label.clone(),
            });
        }
    }
    Ok(ConflictRegionsDto { regions, needs_git })
}

/// The paths the index reports as unmerged, in the order `git` lists them.
///
/// `[FACT]` `git ls-files -u` prints one record per stage ✓ — so a
/// conflicted path is listed three times ✓ — and this keeps each path
/// once ✓, in the order it first appeared ✓.
async fn unmerged_paths(workdir: &Path) -> AppResult<Vec<String>> {
    let out =
        gitgit::server::subprocess::run_capture("git", &["ls-files", "-u", "-z"], Some(workdir))
            .await
            .map_err(|e| AppError::Git(e.to_string()))?;
    if !out.status.success() {
        return Err(AppError::Git(
            String::from_utf8_lossy(&out.stderr).into_owned(),
        ));
    }
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut paths: Vec<String> = Vec::new();
    for record in text.split('\0') {
        if record.is_empty() {
            continue;
        }
        // `<mode> <oid> <stage>\t<path>`
        let Some((_, path)) = record.split_once('\t') else {
            continue;
        };
        if !paths.iter().any(|p| p == path) {
            paths.push(path.to_string());
        }
    }
    Ok(paths)
}

/// The text of a blob, as a lossy string.
///
/// `[FACT]` A blob that is not UTF-8 comes back with replacement
/// characters ✓ — the core refuses binary content anyway ✓, and the
/// bytes are only read for the context lines around a region ✓.
async fn blob_text(workdir: &Path, oid: &str) -> AppResult<String> {
    let out =
        gitgit::server::subprocess::run_capture("git", &["cat-file", "blob", oid], Some(workdir))
            .await
            .map_err(|e| AppError::Git(e.to_string()))?;
    if !out.status.success() {
        return Err(AppError::Git(
            String::from_utf8_lossy(&out.stderr).into_owned(),
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The base file's lines before `start`, at most [`CONTEXT_LINES`] of them.
fn context_before(lines: &[&str], start: usize) -> String {
    let from = start.saturating_sub(CONTEXT_LINES);
    lines[from.min(lines.len())..start.min(lines.len())].join("\n")
}

/// The base file's lines after `end`, at most [`CONTEXT_LINES`] of them.
fn context_after(lines: &[&str], end: usize) -> String {
    let to = end.saturating_add(CONTEXT_LINES).min(lines.len());
    lines[end.min(lines.len())..to].join("\n")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn the_context_is_the_lines_around_the_region() {
        let lines: Vec<&str> = vec!["one", "two", "three", "four", "five"];
        // The region covers lines 2..3 ("three"), so the context is the
        // line before and the lines after.
        assert_eq!(context_before(&lines, 2), "one\ntwo");
        assert_eq!(context_after(&lines, 3), "four\nfive");
    }

    #[test]
    fn the_context_never_reaches_past_the_file() {
        let lines: Vec<&str> = vec!["only"];
        assert_eq!(context_before(&lines, 0), "");
        assert_eq!(context_after(&lines, 1), "");
        assert_eq!(context_after(&lines, 0), "only");
    }

    #[test]
    fn the_context_is_bounded() {
        let lines: Vec<&str> = vec!["a", "b", "c", "d", "e", "f", "g", "h"];
        // Three lines before index 8 are the lines 5, 6 and 7.
        assert_eq!(context_before(&lines, 8), "f\ng\nh");
        assert_eq!(context_after(&lines, 0), "a\nb\nc");
    }
}
