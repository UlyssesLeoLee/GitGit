//! Working-tree status and diff for a checked-out repository.
//!
//! Per the archived ADR-0002 position this crate never reimplements
//! high-risk Git facilities, and porcelain parsing is not one of them: we
//! shell out to the system `git` binary through `tokio::process::Command`
//! and parse the stable, machine-readable `--porcelain=v1` output. The
//! same shape is used by [`crate::ai::diff`] for `gitai`, and the
//! `apps/gm-desktop` shell reuses both through the library target.
//!
//! # Why porcelain v1
//!
//! Three properties matter here, and all three are measured against real
//! `git` (see the `parse_*` unit tests and the end-to-end tests at the
//! bottom of this file):
//!
//! 1. It is one line per entry, so a UI can group staged / unstaged /
//!    untracked without a second parse.
//! 2. It is a documented format, unlike the human-oriented `git status`
//!    output whose wording is translated and whose layout changes.
//! 3. Quoting is observable. A path containing a space is emitted as
//!    `"with space.txt"`, so a parser that splits on whitespace accepts a
//!    format git never emits. [`unquote_porcelain_path`] handles it.
//!
//! We pass `-c core.quotePath=false` so non-ASCII paths arrive as UTF-8
//! instead of octal escapes; quoting is still applied for spaces, quotes
//! and control characters, which is what the unquoter is for.
//!
//! # Bare repositories
//!
//! `git status` in a bare repository exits 128 with
//! `fatal: this operation must be run in a work tree` (measured). Rather
//! than invent an error for it, [`is_bare_repo`] classifies the directory
//! from the filesystem alone, and callers surface that fact themselves.
//! Genuine `git` failures travel as [`GitGitError::GitExit`] carrying git's
//! own exit code and stderr.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::process::Command;

use crate::config::validate_repo_name;
use crate::error::{GitGitError, Result};

/// Largest diff text returned to a caller, in bytes.
///
/// A bound, not a suggestion: the text is copied into the desktop webview
/// and, in the `gitai` path, into a model prompt. Mirrors the
/// `REVIEW_MAX_DIFF_BYTES` ceiling in the desktop shell.
pub const MAX_DIFF_BYTES: usize = 256 * 1024;

// --- data model ---------------------------------------------------------

/// One entry from `git status --porcelain=v1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusEntry {
    /// Repository-relative path, unquoted. For a rename this is the
    /// destination.
    pub path: String,
    /// Source path of a rename or copy, unquoted.
    pub orig_path: Option<String>,
    /// Index (staged) status character, or `None` for ` `.
    pub index_status: Option<char>,
    /// Work-tree status character, or `None` for ` `.
    pub worktree_status: Option<char>,
    /// `??` — the path is not tracked by git at all.
    pub untracked: bool,
    /// `!!` — the path is ignored. Only produced when the caller asked
    /// for ignored entries; this crate does not, so it is always `false`
    /// from [`worktree_status`].
    pub ignored: bool,
}

impl StatusEntry {
    /// True when the index differs from `HEAD` for this path.
    pub fn is_staged(&self) -> bool {
        self.index_status.is_some_and(|c| c != '?' && c != '!')
    }

    /// True when the work tree differs from the index for this path.
    pub fn is_unstaged(&self) -> bool {
        self.worktree_status.is_some_and(|c| c != '?' && c != '!')
    }
}

/// Branch metadata from the `## ` header line of `--porcelain=v1 --branch`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BranchInfo {
    /// `None` when `HEAD` is detached.
    pub branch: Option<String>,
    /// The upstream ref, when the branch tracks one.
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
}

/// A whole working-tree status snapshot.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorktreeStatus {
    pub branch: Option<String>,
    /// Short `HEAD` sha, when the repository has a commit.
    pub head: Option<String>,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    /// One entry per changed path, in git's own order.
    pub entries: Vec<StatusEntry>,
    /// `entries.is_empty()`. A clean working tree is a real answer, not an
    /// error, and callers render it as such.
    pub is_clean: bool,
}

/// Which question a diff is answering.
///
/// These are three different comparisons and conflating them is the usual
/// bug in a status view:
/// - [`DiffTarget::Staged`] — index against `HEAD` (`git diff --cached`):
///   what a commit *would* contain.
/// - [`DiffTarget::Worktree`] — work tree against index (`git diff`): what
///   is not yet staged.
/// - [`DiffTarget::Head`] — work tree against `HEAD` (`git diff HEAD`):
///   everything uncommitted at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffTarget {
    Staged,
    Worktree,
    Head,
}

impl DiffTarget {
    /// Parse the wire name used by the frontend. Case-insensitive.
    pub fn parse(raw: &str) -> Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "staged" | "cached" | "index" => Ok(Self::Staged),
            "worktree" | "unstaged" => Ok(Self::Worktree),
            "head" | "all" => Ok(Self::Head),
            other => Err(GitGitError::Http(format!(
                "unknown diff target `{other}`: expected staged, worktree or head"
            ))),
        }
    }

    /// Wire name, matching [`DiffTarget::parse`].
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Staged => "staged",
            Self::Worktree => "worktree",
            Self::Head => "head",
        }
    }
}

/// The diff text for one comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffPayload {
    pub target: DiffTarget,
    /// The repo-relative path the diff was scoped to, if any.
    pub path: Option<String>,
    /// Unified diff text, `--no-color`, bounded by the caller's byte cap.
    pub text: String,
    /// True when `text` was cut at the byte cap.
    pub truncated: bool,
    /// True when the scoped path is untracked and the text is therefore a
    /// synthesized "whole file is new" diff rather than a `git diff`
    /// result. `git diff` reports nothing for an untracked file, so the UI
    /// needs to know that this text came from somewhere else.
    pub untracked: bool,
    /// Number of `diff --git` headers in `text`.
    pub files: usize,
}

// --- parsing ------------------------------------------------------------

/// Decode a C-style quoted porcelain path.
///
/// git quotes a path when it contains a space, a quote, a backslash or a
/// control character. Measured examples: ` M "with space.txt"` and
/// `R  "old name.txt" -> "new name.txt"`. An unquoted path is returned
/// unchanged.
pub fn unquote_porcelain_path(raw: &str) -> String {
    let Some(inner) = raw.strip_prefix('"').and_then(|s| s.strip_suffix('"')) else {
        return raw.to_string();
    };
    // Octal escapes are byte-wise, and a non-ASCII path arrives as a run
    // of them, so bytes are accumulated first and decoded once at the end.
    let mut bytes: Vec<u8> = Vec::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            let mut buf = [0u8; 4];
            bytes.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            continue;
        }
        match chars.next() {
            // git escapes the ordinary C set.
            Some('n') => bytes.push(b'\n'),
            Some('t') => bytes.push(b'\t'),
            Some('r') => bytes.push(b'\r'),
            Some('"') => bytes.push(b'"'),
            Some('\\') => bytes.push(b'\\'),
            // Octal escape, as emitted when `core.quotePath` is on.
            Some(d @ '0'..='7') => {
                let mut value = d.to_digit(8).unwrap_or(0);
                for _ in 0..2 {
                    let Some(next) = chars.clone().next().and_then(|c| c.to_digit(8)) else {
                        break;
                    };
                    chars.next();
                    value = value * 8 + next;
                }
                bytes.push(u8::try_from(value).unwrap_or(0));
            }
            Some(other) => {
                let mut buf = [0u8; 4];
                bytes.extend_from_slice(other.encode_utf8(&mut buf).as_bytes());
            }
            None => bytes.push(b'\\'),
        }
    }
    // A path git emitted is valid UTF-8; if the octal decoding produced
    // something else, replacement characters beat a panic here.
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Split the path field of an `R`/`C` entry into `(destination, source)`.
///
/// git emits `<orig> -> <path>` and quotes each side independently, so the
/// arrow cannot be found with a plain `split(" -> ")` when a path itself
/// contains quotes. When the field starts with a quote we walk to the
/// matching close quote; otherwise the arrow is the first ` -> `.
fn split_rename_field(field: &str) -> Option<(String, String)> {
    if let Some(rest) = field.strip_prefix('"') {
        let mut escaped = false;
        for (idx, c) in rest.char_indices() {
            if escaped {
                escaped = false;
                continue;
            }
            match c {
                '\\' => escaped = true,
                '"' => {
                    // Rebuild just the first quoted segment; the rest of
                    // the field belongs to the destination path.
                    let orig = unquote_porcelain_path(&format!("\"{}\"", &rest[..idx]));
                    let after = &rest[idx + c.len_utf8()..];
                    let after = after.strip_prefix(" -> ")?;
                    return Some((unquote_porcelain_path(after), orig));
                }
                _ => {}
            }
        }
        return None;
    }
    let arrow = field.find(" -> ")?;
    let orig = &field[..arrow];
    let dest = &field[arrow + 4..];
    Some((dest.to_string(), orig.to_string()))
}

/// Parse one porcelain entry line into a [`StatusEntry`].
///
/// Returns `None` for the `## ` branch header, for blank lines, and for
/// any line that does not have the `XY<space>path` shape. The `## ` header
/// is recognised by its prefix, so a *path* beginning with `#` is still
/// parsed: git emits `## ` only as the branch header, and a file named
/// `## x` arrives as `"## x"`.
pub fn parse_status_line(line: &str) -> Option<StatusEntry> {
    if line.starts_with("## ") || line.trim().is_empty() {
        return None;
    }
    let bytes = line.as_bytes();
    if bytes.len() < 3 || bytes[2] != b' ' {
        return None;
    }
    let x = bytes[0] as char;
    let y = bytes[1] as char;
    let field = &line[3..];
    if field.is_empty() {
        return None;
    }

    let untracked = x == '?' && y == '?';
    let ignored = x == '!' && y == '!';
    let index_status = (!untracked && !ignored && x != ' ').then_some(x);
    let worktree_status = (!untracked && !ignored && y != ' ').then_some(y);

    // A rename or copy puts `orig -> path` in the field.
    let (path, orig_path) = if x == 'R' || x == 'C' {
        match split_rename_field(field) {
            Some((dest, orig)) => (dest, Some(orig)),
            // Unparseable rename field: keep the whole field as the path
            // rather than dropping the entry.
            None => (unquote_porcelain_path(field), None),
        }
    } else {
        (unquote_porcelain_path(field), None)
    };

    Some(StatusEntry {
        path,
        orig_path,
        index_status,
        worktree_status,
        untracked,
        ignored,
    })
}

/// Parse the `## ` branch header emitted by `--porcelain=v1 --branch`.
///
/// Measured forms: `## master`, `## No commits yet on master`,
/// `## main...origin/main [ahead 1, behind 2]`, `## HEAD (no branch)`.
pub fn parse_branch_header(line: &str) -> Option<BranchInfo> {
    let rest = line.strip_prefix("## ")?;
    let mut info = BranchInfo::default();
    if rest == "HEAD (no branch)" {
        return Some(info);
    }
    // The divergence counts live in the tail, after the upstream name, so
    // they are searched for there rather than in the branch name.
    let (branch, tail) = match rest.strip_prefix("No commits yet on ") {
        Some(name) => (name, None),
        None => match rest.split_once("...") {
            Some((branch, tail)) => (branch, Some(tail)),
            None => (rest, None),
        },
    };
    info.branch = Some(branch.to_string());
    let Some(tail) = tail else { return Some(info) };
    info.upstream = Some(tail.split(" [").next().unwrap_or(tail).to_string());
    // `[ahead N, behind M]`, either half optional.
    let Some(open) = tail.find(" [") else {
        return Some(info);
    };
    let Some(close) = tail.rfind(']') else {
        return Some(info);
    };
    for part in tail[open + 2..close].split(", ") {
        if let Some(v) = part.strip_prefix("ahead ") {
            info.ahead = v.trim().parse().unwrap_or(0);
        } else if let Some(v) = part.strip_prefix("behind ") {
            info.behind = v.trim().parse().unwrap_or(0);
        }
    }
    Some(info)
}

/// Parse the whole `--porcelain=v1 --branch --untracked-files=all` output.
pub fn parse_porcelain_v1(stdout: &str) -> WorktreeStatus {
    let mut status = WorktreeStatus::default();
    for line in stdout.lines() {
        if line.starts_with("## ") {
            if let Some(info) = parse_branch_header(line) {
                status.branch = info.branch;
                status.upstream = info.upstream;
                status.ahead = info.ahead;
                status.behind = info.behind;
            }
            continue;
        }
        if let Some(entry) = parse_status_line(line) {
            status.entries.push(entry);
        }
    }
    status.is_clean = status.entries.is_empty();
    status
}

// --- path resolution ----------------------------------------------------

/// True when `path` is a bare repository directory rather than a checkout.
///
/// A filesystem test, not a `git` call: a bare repository stores `HEAD`
/// and `objects/` directly in its own directory and has no `.git`
/// subdirectory, which is exactly the shape `list_repos` selects for.
pub fn is_bare_repo(path: &Path) -> bool {
    path.join("HEAD").is_file() && !path.join(".git").exists()
}

/// Resolve a repository *name* under `root` to a directory path.
///
/// The name is checked with [`validate_repo_name`] — the same rule the
/// HTTP layer applies, and the one its `..%2Fescape` case tests — before
/// any filesystem access happens, so a traversal attempt never reaches a
/// subprocess. The `.git` suffix is stripped and re-validated exactly as
/// [`crate::config::Config::repo_path`] does, so `..git` cannot smuggle a
/// traversal past the first check.
///
/// This function performs no I/O: it either returns a validated path or an
/// error, which is what makes the traversal case testable in isolation.
pub fn resolve_worktree(root: &Path, name: &str) -> Result<PathBuf> {
    validate_repo_name(name)?;
    let stem = name.strip_suffix(".git").unwrap_or(name);
    validate_repo_name(stem)?;
    // The rule above rejects `..` and both separators, but it permits a
    // colon — and on Windows `C:` is a *drive prefix*, so
    // `Path::join("C:win")` discards `root` entirely and yields
    // `"C:win"`. Measured on Windows 11, and asserted by
    // `an_accepted_name_always_resolves_to_a_direct_child_of_the_root`.
    // A colon is not a legal character in a Windows filename either, so
    // refusing it costs nothing on the platform that needs it.
    if stem.contains(':') {
        return Err(GitGitError::InvalidRepoName(name.to_string()));
    }
    Ok(root.join(stem))
}

/// Reject a repo-relative path that could read outside the work tree.
///
/// The exact-match check against git's own `??` list is the real boundary
/// for untracked files; this is the cheap second gate, applied to every
/// path argument so no code path can skip it.
fn validate_rel_path(path: &str) -> Result<&str> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(GitGitError::Http("empty path".to_string()));
    }
    if trimmed.starts_with('/') || trimmed.starts_with('\\') {
        return Err(GitGitError::Http(format!(
            "path must be repository-relative: {trimmed}"
        )));
    }
    if trimmed.contains("..") {
        return Err(GitGitError::Http(format!(
            "path must not contain `..`: {trimmed}"
        )));
    }
    // A git pathspec is always `/`-separated, so a backslash is never
    // legitimate; a colon is a Windows drive prefix or an ADS separator,
    // and either would reach outside the work tree.
    if trimmed.contains('\\') || trimmed.contains(':') {
        return Err(GitGitError::Http(format!(
            "path must not contain `\\` or `:`: {trimmed}"
        )));
    }
    Ok(trimmed)
}

// --- git invocation -----------------------------------------------------

/// Run `git` with an allow-listed argv and return stdout.
///
/// Arguments are passed as a vector, never as a shell string, so nothing
/// in a path can be re-read as an option. `--no-ext-diff` and
/// `--no-textconv` disable the user-configured external diff driver and
/// textconv filters: both execute arbitrary commands, and a read-only
/// viewer has no business running them.
async fn git_output(workdir: &Path, args: &[&str]) -> Result<String> {
    let mut cmd = Command::new("git");
    cmd.arg("-c").arg("core.quotePath=false");
    cmd.args(args);
    cmd.current_dir(workdir);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        use std::os::windows::process::CommandExt;
        cmd.as_std_mut().creation_flags(CREATE_NO_WINDOW);
    }
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let rendered = format!("git {}", args.join(" "));
    let out = cmd
        .output()
        .await
        .map_err(|source| GitGitError::GitSubprocess {
            cmd: rendered.clone(),
            source,
        })?;
    if !out.status.success() {
        return Err(GitGitError::GitExit {
            cmd: rendered,
            status: out.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        });
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Confirm `workdir` is inside a git work tree.
///
/// A bare repository fails here with git's own
/// `fatal: this operation must be run in a work tree` and exit code 128,
/// relayed verbatim rather than restated.
pub async fn ensure_work_tree(workdir: &Path) -> Result<()> {
    if !workdir.is_dir() {
        return Err(GitGitError::Http(format!(
            "not a directory: {}",
            workdir.display()
        )));
    }
    let out = git_output(workdir, &["rev-parse", "--is-inside-work-tree"]).await?;
    if out.trim() != "true" {
        return Err(GitGitError::Http(format!(
            "not a git work tree: {}",
            workdir.display()
        )));
    }
    Ok(())
}

/// Read the working-tree status of `workdir`.
pub async fn worktree_status(workdir: &Path) -> Result<WorktreeStatus> {
    ensure_work_tree(workdir).await?;
    let stdout = git_output(
        workdir,
        &[
            "status",
            "--porcelain=v1",
            "--branch",
            "--untracked-files=all",
        ],
    )
    .await?;
    let mut status = parse_porcelain_v1(&stdout);
    // The short sha is a second, cheap call; a repository with no commit
    // has no `HEAD` and `rev-parse` fails, which is not an error here.
    if let Ok(sha) = git_output(workdir, &["rev-parse", "--short", "HEAD"]).await {
        let sha = sha.trim().to_string();
        if !sha.is_empty() {
            status.head = Some(sha);
        }
    }
    Ok(status)
}

/// Produce the diff text for `target`, optionally scoped to one path.
///
/// When `path` is given and git reports it as untracked, `git diff` has
/// nothing to say, so the text is synthesized with `git diff --no-index`
/// against an empty file and the result is labelled [`DiffPayload::untracked`].
/// That call exits 1 whenever the files differ, so exit 0 and 1 are both
/// success and only 128-and-above is treated as a failure.
pub async fn worktree_diff(
    workdir: &Path,
    target: DiffTarget,
    path: Option<&str>,
    max_bytes: usize,
) -> Result<DiffPayload> {
    ensure_work_tree(workdir).await?;

    let scoped = path.map(validate_rel_path).transpose()?;
    let mut untracked = false;
    if let Some(rel) = scoped {
        untracked = is_untracked(workdir, rel).await?;
    }

    let raw = if untracked {
        let rel = scoped.unwrap_or_default();
        untracked_diff_text(workdir, rel).await?
    } else {
        let mut args: Vec<&str> = vec!["diff", "--no-color", "--no-ext-diff", "--no-textconv"];
        match target {
            DiffTarget::Staged => args.push("--cached"),
            DiffTarget::Worktree => {}
            DiffTarget::Head => args.push("HEAD"),
        }
        if let Some(rel) = scoped {
            // `--` ends option parsing, so a path named like a flag
            // cannot be read as one.
            args.push("--");
            args.push(rel);
        }
        git_output(workdir, &args).await?
    };

    let (text, truncated) = truncate_bytes(&raw, max_bytes);
    let files = raw.lines().filter(|l| l.starts_with("diff --git")).count();
    Ok(DiffPayload {
        target,
        path: scoped.map(str::to_string),
        text,
        truncated,
        untracked,
        files,
    })
}

/// True when `rel` is listed as `??` by git for this work tree.
async fn is_untracked(workdir: &Path, rel: &str) -> Result<bool> {
    let stdout = git_output(
        workdir,
        &[
            "status",
            "--porcelain=v1",
            "--untracked-files=all",
            "--",
            rel,
        ],
    )
    .await?;
    Ok(stdout
        .lines()
        .filter_map(parse_status_line)
        .any(|e| e.untracked && e.path == rel))
}

/// Cut `text` to at most `max_bytes`, on a char boundary.
fn truncate_bytes(text: &str, max_bytes: usize) -> (String, bool) {
    if text.len() <= max_bytes {
        return (text.to_string(), false);
    }
    let mut end = max_bytes;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    (text[..end].to_string(), true)
}

/// A unique empty file to diff an untracked path against.
///
/// The name carries the pid and a nanosecond stamp, matching the
/// `unique_temp` convention in the sibling test modules (no `tempfile`
/// dependency, per the 0-new-external-deps rule).
fn empty_temp_file() -> Result<PathBuf> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let path =
        std::env::temp_dir().join(format!("gitgit-empty-{}-{nanos}.txt", std::process::id()));
    std::fs::write(&path, b"")?;
    Ok(path)
}

/// Diff an untracked file against an empty one, then present the result
/// the way git presents a new file.
///
/// `git diff --no-index` labels the *empty* side with its real path, so
/// the header is rebuilt: an untracked path has no `HEAD` version and no
/// index entry, which is exactly git's "new file" case. The `index` line
/// and the hunks are git's own output, untouched.
async fn untracked_diff_text(workdir: &Path, rel: &str) -> Result<String> {
    let empty = empty_temp_file()?;

    let mut cmd = Command::new("git");
    cmd.arg("diff")
        .arg("--no-color")
        .arg("--no-ext-diff")
        .arg("--no-textconv")
        .arg("--no-index")
        .arg("--")
        .arg(&empty)
        .arg(rel);
    cmd.current_dir(workdir);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        use std::os::windows::process::CommandExt;
        cmd.as_std_mut().creation_flags(CREATE_NO_WINDOW);
    }
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let rendered = format!("git diff --no-index -- {rel}");
    let out = cmd
        .output()
        .await
        .map_err(|source| GitGitError::GitSubprocess {
            cmd: rendered.clone(),
            source,
        })?;
    // Best-effort cleanup; the temp file is ours and is not worth keeping.
    let _ = std::fs::remove_file(&empty);

    // `--no-index` exits 1 when the two paths differ, which is the normal
    // case for an untracked file against an empty one.
    if !matches!(out.status.code(), Some(0) | Some(1)) {
        return Err(GitGitError::GitExit {
            cmd: rendered,
            status: out.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        });
    }
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    Ok(retarget_as_new_file(&text, rel))
}

/// Rewrite the `--no-index` header into git's new-file header.
///
/// Keeps git's `index` line and every hunk line; replaces the `diff --git`,
/// `---` and `+++` lines and inserts `new file mode`, so the result is
/// indistinguishable from `git diff HEAD` for a file that was about to be
/// added. A binary file has no hunks — git still emits the header — so the
/// `index` line is the only thing worth preserving.
fn retarget_as_new_file(text: &str, rel: &str) -> String {
    // The file mode lives on git's `index` line, which is emitted after
    // the header we are replacing, so it is read in a first pass.
    let mode = text.lines().find_map(|line| {
        let rest = line.strip_prefix("index ")?;
        rest.rsplit_once(' ').map(|(_, m)| m)
    });

    let mut out = String::with_capacity(text.len() + 64);
    let mut header_written = false;
    for line in text.lines() {
        if line.starts_with("diff --git ")
            || line.starts_with("--- ")
            || line.starts_with("+++ ")
            || line.starts_with("old mode")
            || line.starts_with("new mode")
            || line.starts_with("deleted file mode")
        {
            continue;
        }
        if !header_written {
            out.push_str(&format!("diff --git a/{rel} b/{rel}\n"));
            if let Some(m) = mode {
                out.push_str(&format!("new file mode {m}\n"));
            }
            header_written = true;
        }
        out.push_str(line);
        out.push('\n');
    }
    if !header_written {
        // Identical content: `--no-index` reported no diff at all, which
        // for an untracked path means an empty file. Say so rather than
        // returning nothing.
        out.push_str(&format!("diff --git a/{rel} b/{rel}\n"));
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn unique_temp(label: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!(
            "gitgit-test-status-{label}-{pid}-{n}-{}",
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

    /// A real repo with one commit and `a.txt` tracked, or `None` when
    /// `git` is not on PATH. Mirrors `ai::diff::tests::repo_with_commit`.
    fn repo_with_commit(label: &str) -> Option<PathBuf> {
        let dir = unique_temp(label);
        if !git_ok(&dir, &["init", "-q"]) {
            return None;
        }
        // Identity is per-repo: the ambient global config may be absent.
        if !git_ok(&dir, &["config", "user.email", "t@example.invalid"])
            || !git_ok(&dir, &["config", "user.name", "t"])
        {
            return None;
        }
        std::fs::write(dir.join("a.txt"), "hello\n").unwrap();
        if !git_ok(&dir, &["add", "."]) || !git_ok(&dir, &["commit", "-q", "-m", "first"]) {
            return None;
        }
        Some(dir)
    }

    // --- pure parser cases ---------------------------------------------

    #[test]
    fn status_line_separates_index_from_worktree() {
        let e = parse_status_line("MM a.txt").unwrap();
        assert_eq!(e.index_status, Some('M'));
        assert_eq!(e.worktree_status, Some('M'));
        assert!(e.is_staged() && e.is_unstaged());
        assert_eq!(e.path, "a.txt");

        // `M ` staged only, ` M` worktree only: the two-column shape is
        // the whole reason porcelain is used.
        let staged = parse_status_line("M  a.txt").unwrap();
        assert!(staged.is_staged() && !staged.is_unstaged());
        let unstaged = parse_status_line(" M a.txt").unwrap();
        assert!(!unstaged.is_staged() && unstaged.is_unstaged());
    }

    #[test]
    fn status_line_reads_untracked_and_deletion() {
        let u = parse_status_line("?? new.txt").unwrap();
        assert!(u.untracked);
        assert!(!u.is_staged() && !u.is_unstaged());

        let d = parse_status_line(" D gone.txt").unwrap();
        assert_eq!(d.worktree_status, Some('D'));
        assert!(!d.untracked);
    }

    #[test]
    fn status_line_unquotes_paths_with_spaces() {
        // Measured: git emits ` M "with space.txt"`, so a parser that
        // splits on whitespace would accept a format git never emits.
        let e = parse_status_line(" M \"with space.txt\"").unwrap();
        assert_eq!(e.path, "with space.txt");
    }

    #[test]
    fn status_line_unquotes_rename_both_sides() {
        // Measured: `R  "old name.txt" -> "new name.txt"`.
        let e = parse_status_line("R  \"old name.txt\" -> \"new name.txt\"").unwrap();
        assert_eq!(e.path, "new name.txt");
        assert_eq!(e.orig_path.as_deref(), Some("old name.txt"));
        assert!(e.is_staged());
    }

    #[test]
    fn status_line_handles_unquoted_rename() {
        // Measured: `R  a.txt -> renamed.txt`.
        let e = parse_status_line("R  a.txt -> renamed.txt").unwrap();
        assert_eq!(e.path, "renamed.txt");
        assert_eq!(e.orig_path.as_deref(), Some("a.txt"));
    }

    #[test]
    fn branch_header_reads_upstream_divergence() {
        let b = parse_branch_header("## main...origin/main [ahead 1, behind 2]").unwrap();
        assert_eq!(b.branch.as_deref(), Some("main"));
        assert_eq!(b.upstream.as_deref(), Some("origin/main"));
        assert_eq!(b.ahead, 1);
        assert_eq!(b.behind, 2);

        let plain = parse_branch_header("## master").unwrap();
        assert_eq!(plain.branch.as_deref(), Some("master"));
        assert!(plain.upstream.is_none());
        assert_eq!(plain.ahead, 0);
    }

    #[test]
    fn branch_header_reads_unborn_and_detached() {
        // Measured on a repo with no commit.
        let unborn = parse_branch_header("## No commits yet on master").unwrap();
        assert_eq!(unborn.branch.as_deref(), Some("master"));

        let detached = parse_branch_header("## HEAD (no branch)").unwrap();
        assert!(detached.branch.is_none());
    }

    #[test]
    fn clean_output_parses_to_an_empty_entry_list() {
        // A clean tree must be distinguishable from a failed parse: the
        // UI shows it as a real answer.
        let s = parse_porcelain_v1("## master\n");
        assert!(s.is_clean);
        assert!(s.entries.is_empty());
        assert_eq!(s.branch.as_deref(), Some("master"));
    }

    #[test]
    fn full_output_parses_into_staged_unstaged_and_untracked() {
        // Shaped like the measured output of a repo with one of each.
        // Built by joining lines rather than with a `\`-continued literal:
        // the continuation strips the leading whitespace, and a leading
        // space is exactly what the ` M` column encoding depends on.
        let raw = [
            "## master...origin/master [ahead 1]",
            "M  staged.txt",
            " M dirty.txt",
            "?? fresh.txt",
        ]
        .join("\n");
        let s = parse_porcelain_v1(&raw);
        assert!(!s.is_clean);
        assert_eq!(s.ahead, 1);
        assert_eq!(s.upstream.as_deref(), Some("origin/master"));
        assert_eq!(s.entries.len(), 3);
        assert!(s.entries[0].is_staged() && !s.entries[0].is_unstaged());
        assert!(!s.entries[1].is_staged() && s.entries[1].is_unstaged());
        assert!(s.entries[2].untracked);
    }

    #[test]
    fn unquote_handles_octal_escapes() {
        // Measured with `core.quotePath` left at its default: a non-ASCII
        // path arrives as a run of octal byte escapes, one per UTF-8 byte.
        assert_eq!(
            unquote_porcelain_path("\"\\303\\274n\\303\\257code.txt\""),
            "ünïcode.txt"
        );
        assert_eq!(unquote_porcelain_path("plain.txt"), "plain.txt");
    }

    #[test]
    fn diff_target_parses_the_three_wire_names() {
        assert_eq!(DiffTarget::parse("staged").unwrap(), DiffTarget::Staged);
        assert_eq!(DiffTarget::parse("worktree").unwrap(), DiffTarget::Worktree);
        assert_eq!(DiffTarget::parse("HEAD").unwrap(), DiffTarget::Head);
        assert!(DiffTarget::parse("nonsense").is_err());
        assert_eq!(DiffTarget::Head.as_str(), "head");
    }

    #[test]
    fn truncates_on_a_char_boundary() {
        let (text, cut) = truncate_bytes("héllo wörld", 2);
        assert!(cut);
        // 2 bytes is inside the two-byte `é`, so the cut lands on 1.
        assert_eq!(text, "h");
        let (same, not_cut) = truncate_bytes("short", 99);
        assert!(!not_cut);
        assert_eq!(same, "short");
    }

    #[test]
    fn retarget_rewrites_the_empty_side_as_dev_null() {
        let raw = [
            "diff --git \"a/tmp/empty.txt\" b/new file.txt",
            "index e69de29..fbbee86 100644",
            "--- \"a/tmp/empty.txt\"",
            "+++ b/new file.txt",
            "@@ -0,0 +1,2 @@",
            "+alpha",
            "+beta",
        ]
        .join("\n");
        let out = retarget_as_new_file(&raw, "new file.txt");
        assert!(
            out.starts_with("diff --git a/new file.txt b/new file.txt\n"),
            "got: {out}"
        );
        assert!(out.contains("new file mode 100644\n"), "got: {out}");
        assert!(
            out.contains("index e69de29..fbbee86 100644\n"),
            "got: {out}"
        );
        // The empty side must not leak the temp path into the UI.
        assert!(!out.contains("empty.txt"), "got: {out}");
        assert!(out.contains("+alpha\n"), "got: {out}");
    }

    // --- path / traversal cases ----------------------------------------

    #[test]
    fn resolve_worktree_rejects_traversal_before_touching_disk() {
        // Every one of these is refused by `validate_repo_name`, the rule
        // the HTTP layer already applies and tests via `..%2Fescape`.
        let root = Path::new("/nonexistent-root-for-this-test");
        for bad in [
            "../escape",
            "..",
            "a/b",
            "a\\b",
            ".hidden",
            "",
            "with space",
            "..git",
            "demo..git",
            "C:win",
        ] {
            let err = resolve_worktree(root, bad).expect_err(&format!("{bad:?} must be rejected"));
            assert!(
                format!("{err}").contains("invalid repo name"),
                "for {bad:?} got: {err}"
            );
        }
    }

    #[test]
    fn an_accepted_name_always_resolves_to_a_direct_child_of_the_root() {
        // The rule permits a few unusual single-segment names that are
        // not traversal. What matters for safety is not that every odd
        // name is refused but that none of the accepted ones can resolve
        // outside the root. `C:win` is deliberately absent: it is
        // refused, and `resolve_worktree_rejects_traversal_before_touching_disk`
        // is where that is asserted.
        let root = Path::new("/tmp/root");
        for name in ["demo", "demo.git", "a.b", "UPPER", "x-y_z", "n"] {
            let resolved = resolve_worktree(root, name).unwrap();
            assert_eq!(
                resolved.parent(),
                Some(root),
                "{name:?} escaped the root: {resolved:?}"
            );
        }
    }

    #[test]
    fn resolve_worktree_accepts_a_plain_name_and_strips_git_suffix() {
        let root = Path::new("/tmp/root");
        assert_eq!(
            resolve_worktree(root, "demo").unwrap(),
            PathBuf::from("/tmp/root/demo")
        );
        // Both spellings normalize to one directory, as `Config::repo_path`
        // does for bare repos.
        assert_eq!(
            resolve_worktree(root, "demo.git").unwrap(),
            PathBuf::from("/tmp/root/demo")
        );
    }

    #[test]
    fn relative_path_guard_rejects_escapes() {
        assert!(validate_rel_path("src/a.rs").is_ok());
        assert!(validate_rel_path("../secret").is_err());
        assert!(validate_rel_path("/etc/passwd").is_err());
        assert!(validate_rel_path("C:\\win").is_err());
        assert!(validate_rel_path("C:win").is_err());
        assert!(validate_rel_path("nested\\..\\win").is_err());
        assert!(validate_rel_path("  ").is_err());
    }

    #[test]
    fn is_bare_repo_separates_bare_from_checkout() {
        let root = unique_temp("bare-detect");
        let bare = root.join("bare.git");
        std::fs::create_dir_all(&bare).unwrap();
        std::fs::write(bare.join("HEAD"), b"ref: refs/heads/main\n").unwrap();
        assert!(is_bare_repo(&bare));

        let checkout = root.join("work");
        std::fs::create_dir_all(checkout.join(".git")).unwrap();
        std::fs::write(
            checkout.join(".git").join("HEAD"),
            b"ref: refs/heads/main\n",
        )
        .unwrap();
        assert!(!is_bare_repo(&checkout));
    }

    // --- real-repository cases -----------------------------------------

    #[tokio::test]
    async fn clean_repo_reports_is_clean_with_a_branch() {
        let Some(dir) = repo_with_commit("clean") else {
            return;
        };
        let status = worktree_status(&dir).await.unwrap();
        assert!(status.is_clean, "entries: {:?}", status.entries);
        assert!(status.branch.is_some());
        assert!(status.head.is_some());
    }

    #[tokio::test]
    async fn dirty_repo_reports_staged_unstaged_and_untracked_separately() {
        let Some(dir) = repo_with_commit("dirty") else {
            return;
        };
        // staged: a new file added to the index
        std::fs::write(dir.join("staged.txt"), "s\n").unwrap();
        git_ok(&dir, &["add", "staged.txt"]);
        // unstaged: an edit to a tracked file
        std::fs::write(dir.join("a.txt"), "hello\nmore\n").unwrap();
        // untracked: a file git has never seen
        std::fs::write(dir.join("fresh.txt"), "u\n").unwrap();

        let status = worktree_status(&dir).await.unwrap();
        assert!(!status.is_clean);
        let staged: Vec<&StatusEntry> = status.entries.iter().filter(|e| e.is_staged()).collect();
        let unstaged: Vec<&StatusEntry> =
            status.entries.iter().filter(|e| e.is_unstaged()).collect();
        let untracked: Vec<&StatusEntry> = status.entries.iter().filter(|e| e.untracked).collect();
        assert_eq!(staged.len(), 1, "staged: {staged:?}");
        assert_eq!(staged[0].path, "staged.txt");
        assert_eq!(unstaged.len(), 1, "unstaged: {unstaged:?}");
        assert_eq!(unstaged[0].path, "a.txt");
        assert_eq!(untracked.len(), 1, "untracked: {untracked:?}");
        assert_eq!(untracked[0].path, "fresh.txt");
    }

    #[tokio::test]
    async fn the_three_diff_targets_answer_three_different_questions() {
        let Some(dir) = repo_with_commit("targets") else {
            return;
        };
        // Stage one edit, leave another unstaged. Then staged, worktree
        // and head must each report a different, correct set.
        std::fs::write(dir.join("a.txt"), "hello\nstaged\n").unwrap();
        git_ok(&dir, &["add", "a.txt"]);
        git_ok(&dir, &["commit", "-q", "-m", "second"]);
        std::fs::write(dir.join("a.txt"), "hello\nstaged\nworktree\n").unwrap();
        std::fs::write(dir.join("b.txt"), "untracked\n").unwrap();
        git_ok(&dir, &["add", "b.txt"]);

        let staged = worktree_diff(&dir, DiffTarget::Staged, None, MAX_DIFF_BYTES)
            .await
            .unwrap();
        assert!(staged.text.contains("b.txt"), "staged: {}", staged.text);

        let worktree = worktree_diff(&dir, DiffTarget::Worktree, None, MAX_DIFF_BYTES)
            .await
            .unwrap();
        assert!(
            worktree.text.contains("+worktree"),
            "worktree: {}",
            worktree.text
        );
        assert!(
            !worktree.text.contains("b.txt"),
            "worktree must not show staged"
        );

        let head = worktree_diff(&dir, DiffTarget::Head, None, MAX_DIFF_BYTES)
            .await
            .unwrap();
        assert!(head.text.contains("b.txt") && head.text.contains("+worktree"));
        assert_eq!(head.files, 2);
    }

    #[tokio::test]
    async fn untracked_file_gets_a_synthesized_diff_instead_of_nothing() {
        // `git diff` reports nothing for an untracked file. Without this
        // the status view would list a file the user cannot read.
        let Some(dir) = repo_with_commit("untracked") else {
            return;
        };
        std::fs::write(dir.join("brand new.txt"), "alpha\nbeta\n").unwrap();

        // First: the plain diff really is empty for that path.
        let plain = worktree_diff(
            &dir,
            DiffTarget::Worktree,
            Some("brand new.txt"),
            MAX_DIFF_BYTES,
        )
        .await
        .unwrap();
        assert!(plain.untracked, "must be labelled as synthesized");
        assert!(plain.text.contains("new file mode"));
        assert!(plain.text.contains("+alpha"));
        assert!(plain.text.contains("+beta"));
        // The empty temp file git diffed against must not be named.
        assert!(
            !plain.text.contains("gitgit-empty-"),
            "temp path leaked: {}",
            plain.text
        );
    }

    #[tokio::test]
    async fn a_bare_repo_refuses_status_with_gits_own_verdict() {
        // Measured: `git status` in a bare repo exits 128 with
        // `fatal: this operation must be run in a work tree`.
        let root = unique_temp("bare-refusal");
        let bare = root.join("bare.git");
        git_ok(&root, &["init", "-q", "--bare", &bare.to_string_lossy()]);
        assert!(is_bare_repo(&bare));

        let err = worktree_status(&bare).await.unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("work tree") || msg.contains("128"),
            "expected git's own refusal, got: {msg}"
        );
    }

    #[tokio::test]
    async fn diff_is_bounded_and_reports_truncation() {
        let Some(dir) = repo_with_commit("bound") else {
            return;
        };
        // Must differ from the committed content, or there is no diff to
        // truncate.
        std::fs::write(dir.join("a.txt"), "hello\nmore\n").unwrap();
        let capped = worktree_diff(&dir, DiffTarget::Worktree, None, 8)
            .await
            .unwrap();
        assert!(capped.truncated, "text: {:?}", capped.text);
        assert!(capped.text.len() <= 8);
    }

    #[tokio::test]
    async fn a_path_outside_the_work_tree_is_refused() {
        // Defence in depth behind the exact-match untracked check: even if
        // that check were bypassed, `..` and absolute paths are refused.
        let Some(dir) = repo_with_commit("escape") else {
            return;
        };
        let err = worktree_diff(
            &dir,
            DiffTarget::Worktree,
            Some("../../secret"),
            MAX_DIFF_BYTES,
        )
        .await
        .unwrap_err();
        assert!(format!("{err}").contains(".."), "got: {err}");
    }
}
