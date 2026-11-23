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

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
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

// --- the write-back (ADR-0024 §3.6) -------------------------------------

/// Which side of a region the user chose.
///
/// `[FACT]` The wire values are the ADR §3.4.3 vocabulary — ours, theirs,
/// ai, manual — plus the two the library keeps for itself, `base` and
/// `both`. The command accepts all six so the mapping is complete; the
/// dialog offers four, and `base` / `both` stay in the library.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConflictResolveChoiceKind {
    /// Keep the ours side.
    Ours,
    /// Keep the theirs side.
    Theirs,
    /// Keep the base side — drop both sides' changes.
    Base,
    /// Keep both sides' changes.
    Both,
    /// The model's answer, accepted by the user.
    Ai,
    /// A hand-written replacement.
    Manual,
}

/// One region's decision, as the webview sends it.
///
/// `[FACT]` `index` is the region's ordinal in the merged output
/// (`Region::index`), which is the key `compose_resolved` looks the choice
/// up by. The webview already carries that number on every region
/// (`ConflictRegion.index`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictResolveChoiceDto {
    /// The region's ordinal in the merged output.
    pub index: usize,
    /// Which side won.
    pub choice: ConflictResolveChoiceKind,
    /// The replacement text. Required for `ai` and `manual`, ignored by
    /// the other four.
    #[serde(default)]
    pub text: Option<String>,
}

/// What the write-back did.
///
/// `[FACT]` The path that was written back and staged, and how many
/// regions it carried. The application does not drive the merge itself
/// (ADR §3.6): the user commits.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictResolveDto {
    /// The repository-relative path that was written back and staged.
    pub path: String,
    /// How many conflict regions the path carried.
    pub regions: usize,
}

/// Write the resolved file back into the work tree and stage it.
///
/// `[FACT]` ADR-0024 §3.6: the application writes the merged file back and
/// runs `git add` on the path, so git sees the conflict as resolved. The
/// application does **not** drive the merge itself — the user commits.
///
/// `[FACT]` The command handles **one path**. A file can carry several
/// regions, and the regions of a merge span several files, so the webview
/// groups the decisions by path and calls this once per path.
///
/// `[FACT]` `root` is the working-tree root the repository name is resolved
/// under — the same argument `conflict_regions` takes, for the same
/// reason: the desktop keeps its repositories as bare `<name>.git`
/// directories and the checkouts live under the root the user picked.
#[tauri::command]
pub async fn conflict_resolve(
    state: State<'_, DesktopState>,
    repo: String,
    root: Option<String>,
    path: String,
    choices: Vec<ConflictResolveChoiceDto>,
) -> AppResult<ConflictResolveDto> {
    let dir = resolve_worktree_dir(&state.repos_dir, &repo, root)?;
    resolve_path(&dir, &path, &choices).await
}

/// The write-back for one path.
///
/// `[FACT]` Two phases, and the order is the point:
///
/// 1. **Compose.** `conflict_set` reads the path's three stages out of the
///    index and runs the merge, and `compose_resolved` splices the
///    choices into the merged text. An undecided region is an error here,
///    never a default — so a path with a missing decision fails **before**
///    a single byte is written.
/// 2. **Write.** `write_resolved` validates the path, writes the file and
///    runs `git add` on it. If the add fails, the error says so; the file
///    is on disk and the index still holds the conflict, which is exactly
///    what the user has to be told.
pub(crate) async fn resolve_path(
    dir: &Path,
    path: &str,
    choices: &[ConflictResolveChoiceDto],
) -> AppResult<ConflictResolveDto> {
    // --- phase 1: compose. Nothing is written until this succeeds. ---
    let set = gitgit::repo::conflict_set(dir, path)
        .await
        .map_err(|e| AppError::Gitgit(e.to_string()))?;

    // `[FACT]` A path the core refused to read has no regions, and the
    // merged text is empty — so composing it would write an EMPTY file
    // over a path git still has to resolve. Those paths are git's
    // (ADR §3.7.4) and the write-back does not touch them.
    if set.disposition == gitgit::repo::Disposition::NeedsGit {
        return Err(AppError::Gitgit(format!(
            "{path} is a path git has to handle itself (ADR-0024 §3.7.4); the write-back does not touch it"
        )));
    }
    if !set.has_conflicts() {
        return Err(AppError::Gitgit(format!(
            "{path} has no conflict region to arbitrate"
        )));
    }

    let mut map: BTreeMap<usize, gitgit::repo::RegionChoice> = BTreeMap::new();
    for choice in choices {
        // `[FACT]` A choice for a region the path does not have is a
        // mistake, not a spare: accepting it would drop the decision
        // silently while the path still looked arbitrated.
        if choice.index >= set.regions.len() {
            return Err(AppError::Gitgit(format!(
                "{path} has {} conflict region(s); region {} does not exist",
                set.regions.len(),
                choice.index
            )));
        }
        let region_choice = match choice.choice {
            ConflictResolveChoiceKind::Ours => gitgit::repo::RegionChoice::Ours,
            ConflictResolveChoiceKind::Theirs => gitgit::repo::RegionChoice::Theirs,
            ConflictResolveChoiceKind::Base => gitgit::repo::RegionChoice::Base,
            ConflictResolveChoiceKind::Both => gitgit::repo::RegionChoice::Both,
            // `[FACT]` `ai` and `manual` are the same thing to the core: a
            // hand-written replacement (`RegionChoice::Custom`).
            ConflictResolveChoiceKind::Ai | ConflictResolveChoiceKind::Manual => {
                let text = choice.text.as_ref().ok_or_else(|| {
                    AppError::Gitgit(format!(
                        "the {} choice for region {} of {path} needs a replacement text",
                        format!("{:?}", choice.choice).to_lowercase(),
                        choice.index
                    ))
                })?;
                gitgit::repo::RegionChoice::Custom(text.clone())
            }
        };
        if map.insert(choice.index, region_choice).is_some() {
            return Err(AppError::Gitgit(format!(
                "{path} was given two choices for region {}",
                choice.index
            )));
        }
    }

    // `[FACT]` Every region needs a choice. The check names the path and
    // the region, so the message says which file is not ready — and an
    // undecided region is an error, never a default.
    for region in set.all_regions() {
        if !map.contains_key(&region.index) {
            return Err(AppError::Gitgit(format!(
                "{path} has no choice for conflict region {}",
                region.index
            )));
        }
    }

    // `[FACT]` `compose_resolved` refuses the same thing from the other
    // side, and its refusal is passed on as it is: a region the scope
    // layer judged overlapping has no keep-both resolution, and that is
    // not a decision this command may make.
    let resolved =
        gitgit::repo::compose_resolved(&set, &map).map_err(|e| AppError::Gitgit(e.to_string()))?;

    // --- phase 2: write the file and stage it. ---
    gitgit::repo::write_resolved(dir, path, &resolved)
        .await
        .map_err(|e| AppError::Gitgit(e.to_string()))?;

    Ok(ConflictResolveDto {
        path: set.path.clone(),
        regions: set.regions.len(),
    })
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

    // --- the write-back (ADR-0024 §3.6) ---------------------------------

    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// A fresh temporary directory for one fixture.
    fn unique_temp(label: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "gm-desktop-conflict-{label}-{}-{n}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Run `git` in `dir`, reporting success as a bool.
    fn git_ok(dir: &Path, args: &[&str]) -> bool {
        std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// The stdout of a `git` call in `dir`.
    fn git_text(dir: &Path, args: &[&str]) -> String {
        let out = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .expect("git must be runnable");
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    /// A repository with one conflicted path, or `None` when git is
    /// missing or the merge did not conflict.
    ///
    /// `[FACT]` A real merge: `main` and `side` each change the same line
    /// of `f.txt`, and the merge is left unresolved, so the index carries
    /// stages 1, 2 and 3.
    fn repo_with_conflict(label: &str) -> Option<PathBuf> {
        let dir = unique_temp(label);
        if !git_ok(&dir, &["init", "-q", "-b", "main"]) {
            return None;
        }
        if !git_ok(&dir, &["config", "user.email", "t@example.invalid"])
            || !git_ok(&dir, &["config", "user.name", "t"])
        {
            return None;
        }
        std::fs::write(dir.join("f.txt"), "l1\nl2\nl3\n").unwrap();
        if !git_ok(&dir, &["add", "."]) || !git_ok(&dir, &["commit", "-q", "-m", "base"]) {
            return None;
        }
        if !git_ok(&dir, &["checkout", "-q", "-b", "side"]) {
            return None;
        }
        std::fs::write(dir.join("f.txt"), "l1\nT2\nl3\n").unwrap();
        if !git_ok(&dir, &["commit", "-q", "-am", "side"]) {
            return None;
        }
        if !git_ok(&dir, &["checkout", "-q", "main"]) {
            return None;
        }
        std::fs::write(dir.join("f.txt"), "l1\nO2\nl3\n").unwrap();
        if !git_ok(&dir, &["commit", "-q", "-am", "ours"]) {
            return None;
        }
        // The merge is expected to conflict; a clean merge means the
        // fixture is not testing what it claims to.
        git_ok(&dir, &["merge", "side"]);
        if git_text(&dir, &["ls-files", "-u"]).trim().is_empty() {
            return None;
        }
        Some(dir)
    }

    /// A repository with one path git has to resolve itself: a modify /
    /// delete conflict, which has no stage 2.
    ///
    /// `[FACT]` The core refuses such a path (ADR §3.7.4): without the
    /// ours side there is no three-way merge to run.
    fn repo_with_needs_git_path(label: &str) -> Option<PathBuf> {
        let dir = unique_temp(label);
        if !git_ok(&dir, &["init", "-q", "-b", "main"]) {
            return None;
        }
        if !git_ok(&dir, &["config", "user.email", "t@example.invalid"])
            || !git_ok(&dir, &["config", "user.name", "t"])
        {
            return None;
        }
        std::fs::write(dir.join("f.txt"), "l1\nl2\n").unwrap();
        if !git_ok(&dir, &["add", "."]) || !git_ok(&dir, &["commit", "-q", "-m", "base"]) {
            return None;
        }
        if !git_ok(&dir, &["checkout", "-q", "-b", "side"]) {
            return None;
        }
        std::fs::write(dir.join("f.txt"), "l1\nT2\n").unwrap();
        if !git_ok(&dir, &["commit", "-q", "-am", "side"]) {
            return None;
        }
        if !git_ok(&dir, &["checkout", "-q", "main"]) {
            return None;
        }
        if !git_ok(&dir, &["rm", "-q", "f.txt"]) || !git_ok(&dir, &["commit", "-q", "-am", "ours"])
        {
            return None;
        }
        git_ok(&dir, &["merge", "side"]);
        if git_text(&dir, &["ls-files", "-u"]).trim().is_empty() {
            return None;
        }
        Some(dir)
    }

    /// A choice for region `index`.
    fn choice(index: usize, kind: ConflictResolveChoiceKind) -> ConflictResolveChoiceDto {
        ConflictResolveChoiceDto {
            index,
            choice: kind,
            text: None,
        }
    }

    /// A choice for region `index` that carries a replacement text.
    fn custom(
        index: usize,
        kind: ConflictResolveChoiceKind,
        text: &str,
    ) -> ConflictResolveChoiceDto {
        ConflictResolveChoiceDto {
            index,
            choice: kind,
            text: Some(text.to_string()),
        }
    }

    #[tokio::test]
    async fn the_write_back_writes_the_composed_file_and_the_path_is_no_longer_unmerged() {
        let Some(dir) = repo_with_conflict("write-back") else {
            return;
        };
        // `[FACT]` Before the write-back the path is unmerged: git's
        // porcelain v1 says `UU`.
        let before = git_text(&dir, &["status", "--porcelain=v1"]);
        assert!(
            before.starts_with("UU"),
            "expected an unmerged path, got: {before}"
        );

        let dto = resolve_path(&dir, "f.txt", &[choice(0, ConflictResolveChoiceKind::Ours)])
            .await
            .unwrap();
        assert_eq!(dto.path, "f.txt");
        assert_eq!(dto.regions, 1);

        // The file on disk is the composed file.
        assert_eq!(
            std::fs::read_to_string(dir.join("f.txt")).unwrap(),
            "l1\nO2\nl3\n"
        );
        // And git no longer reports the conflict.
        let after = git_text(&dir, &["status", "--porcelain=v1"]);
        assert!(
            !after.contains("UU"),
            "the path must no longer be unmerged: {after}"
        );
    }

    #[tokio::test]
    async fn a_missing_choice_is_refused_and_not_one_byte_is_written() {
        let Some(dir) = repo_with_conflict("missing-choice") else {
            return;
        };
        let before = std::fs::read_to_string(dir.join("f.txt")).unwrap();
        assert!(
            before.contains("<<<<<<<"),
            "the fixture must be a conflicted file: {before}"
        );

        let err = resolve_path(&dir, "f.txt", &[]).await.unwrap_err();
        let message = format!("{err}");
        assert!(
            message.contains("no choice"),
            "the refusal must say the decision is missing: {message}"
        );
        assert!(
            message.contains("f.txt"),
            "the refusal must name the path: {message}"
        );

        // Not one byte was written: the file still carries the markers.
        assert_eq!(std::fs::read_to_string(dir.join("f.txt")).unwrap(), before);
        // And the path is still unmerged.
        let after = git_text(&dir, &["status", "--porcelain=v1"]);
        assert!(
            after.starts_with("UU"),
            "the path must still be unmerged: {after}"
        );
    }

    #[tokio::test]
    async fn a_path_outside_the_work_tree_is_refused() {
        let dir = unique_temp("escape");
        let err = resolve_path(&dir, "../escape.txt", &[]).await.unwrap_err();
        let message = format!("{err}");
        assert!(
            message.contains(".."),
            "the refusal must name the offending path: {message}"
        );
        // The refusal came from the path validation, which runs before the
        // write: nothing was created anywhere.
        let entries: Vec<_> = std::fs::read_dir(&dir).unwrap().collect();
        assert!(entries.is_empty(), "nothing may be written: {entries:?}");
    }

    #[tokio::test]
    async fn a_failed_git_add_is_reported() {
        let Some(dir) = repo_with_conflict("locked-index") else {
            return;
        };
        // `[FACT]` A locked index is the real way `git add` fails: git
        // refuses to create `index.lock` and exits non-zero.
        std::fs::write(dir.join(".git").join("index.lock"), "").unwrap();

        let err = resolve_path(&dir, "f.txt", &[choice(0, ConflictResolveChoiceKind::Ours)])
            .await
            .unwrap_err();
        let message = format!("{err}");
        assert!(
            message.contains("add"),
            "the message must name the step that failed: {message}"
        );
        assert!(
            message.contains("index.lock"),
            "the message must carry git's own reason: {message}"
        );
        // The index still holds the conflict: the write-back did not
        // pretend the path was resolved.
        let after = git_text(&dir, &["status", "--porcelain=v1"]);
        assert!(
            after.starts_with("UU"),
            "the path must still be unmerged: {after}"
        );
    }

    #[tokio::test]
    async fn the_ai_text_lands_in_the_file() {
        let Some(dir) = repo_with_conflict("ai-text") else {
            return;
        };
        // `[FACT]` The text replaces the **conflict region**, not the file:
        // `compose_resolved` splices it in where the region was, so it is
        // the region's replacement — the same thing the dialog's hand-edit
        // box is prefilled with (the ours side's text).
        resolve_path(
            &dir,
            "f.txt",
            &[custom(0, ConflictResolveChoiceKind::Ai, "AI\n")],
        )
        .await
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join("f.txt")).unwrap(),
            "l1\nAI\nl3\n"
        );
    }

    #[tokio::test]
    async fn the_manual_text_lands_in_the_file() {
        let Some(dir) = repo_with_conflict("manual-text") else {
            return;
        };
        resolve_path(
            &dir,
            "f.txt",
            &[custom(0, ConflictResolveChoiceKind::Manual, "BY HAND\n")],
        )
        .await
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join("f.txt")).unwrap(),
            "l1\nBY HAND\nl3\n"
        );
    }

    #[tokio::test]
    async fn the_ai_choice_without_a_replacement_text_is_refused() {
        let Some(dir) = repo_with_conflict("ai-without-text") else {
            return;
        };
        let err = resolve_path(&dir, "f.txt", &[choice(0, ConflictResolveChoiceKind::Ai)])
            .await
            .unwrap_err();
        let message = format!("{err}");
        assert!(
            message.contains("replacement text"),
            "the refusal must say the text is missing: {message}"
        );
        // Nothing was written.
        assert!(
            std::fs::read_to_string(dir.join("f.txt"))
                .unwrap()
                .contains("<<<<<<<"),
            "the file must still carry the markers"
        );
    }

    #[tokio::test]
    async fn a_choice_for_a_region_the_path_does_not_have_is_refused() {
        let Some(dir) = repo_with_conflict("stray-region") else {
            return;
        };
        let err = resolve_path(&dir, "f.txt", &[choice(7, ConflictResolveChoiceKind::Ours)])
            .await
            .unwrap_err();
        let message = format!("{err}");
        assert!(
            message.contains("region 7"),
            "the refusal must name the stray region: {message}"
        );
    }

    #[tokio::test]
    async fn two_choices_for_the_same_region_are_refused() {
        let Some(dir) = repo_with_conflict("double-choice") else {
            return;
        };
        let err = resolve_path(
            &dir,
            "f.txt",
            &[
                choice(0, ConflictResolveChoiceKind::Ours),
                choice(0, ConflictResolveChoiceKind::Theirs),
            ],
        )
        .await
        .unwrap_err();
        let message = format!("{err}");
        assert!(
            message.contains("two choices"),
            "the refusal must say the region was decided twice: {message}"
        );
    }

    #[tokio::test]
    async fn a_path_git_has_to_handle_is_not_touched() {
        let Some(dir) = repo_with_needs_git_path("needs-git") else {
            return;
        };
        // `[FACT]` The content is read **before** the call, so the
        // comparison below is between two different moments and not
        // between a thing and itself.
        let path = dir.join("f.txt");
        let before = std::fs::read_to_string(&path).unwrap();

        let err = resolve_path(&dir, "f.txt", &[choice(0, ConflictResolveChoiceKind::Ours)])
            .await
            .unwrap_err();
        let message = format!("{err}");
        assert!(
            message.contains("git has to handle"),
            "the refusal must say the path is git's: {message}"
        );
        // The file git has to resolve is still untouched.
        //
        // `[FACT]` git leaves a file in the work tree for this conflict —
        // ours deleted it and theirs changed it, so there is no "ours"
        // side to write and git leaves what it has — but the write-back
        // did not touch it. So the claim is not "the file is not there"
        // and not "the file carries the markers": it is that the content
        // is byte for byte what it was before the call.
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            before,
            "the write-back must not touch the path git has to handle"
        );
        // And it is still unmerged.
        let after = git_text(&dir, &["status", "--porcelain=v1"]);
        assert!(
            after.contains('U'),
            "the path must still be unmerged: {after}"
        );
    }
}
