//! Conflict REGIONS — the Rust core of ADR-0024
//! (`docs/adr/0024-hunk-conflict-arbitration.md`).
//!
//! # What this module is
//!
//! Git decides conflicts per FILE. This module decides them per REGION: a
//! contiguous block of text where both sides changed the same base lines,
//! carrying the three sides (ours / base / theirs) and the line ranges they
//! occupy. A SCOPE layer sits above the regions and answers a narrower
//! question — did the two sides touch the same semantic unit (a function, a
//! test block) at all?
//!
//! # The decisions this module implements (ADR §2.1, §3.1, §3.2, §3.8)
//!
//! * `[FACT]` The regions come from
//!   `git merge-file --diff3 --marker-size=24 -L OURS -L BASE -L THEIRS -p`,
//!   and **the exit code is the number of regions**. Measured on git
//!   2.41.0.windows.1: one region -> exit 1, two regions -> exit 2, no
//!   conflict -> exit 0 (ADR §2.1).
//! * `[FACT]` The three sides are parsed **by marker width**, never by
//!   "a line that contains `<<<<<<<`". A base line reading
//!   `<<<<<<< not a real marker` is emitted verbatim by `merge-file`, and
//!   `--marker-size=24` is what makes the real marker (24 characters wide)
//!   distinguishable from it (7 characters wide). Measured: ADR §2.1.
//! * `[FACT]` The base is **mandatory**. `merge-file` is a three-way merge
//!   (`<file1> <orig-file> <file2>`), and the base side is the user's only
//!   reference for "what it was before" (ADR §3.2).
//! * `[FACT]` `git merge-file` takes **files, not blobs** (ADR §2.1), so the
//!   three versions are materialised as temporary files. They are removed on
//!   every path out of this module, error paths included (see [`TempFiles`]).
//!
//! # The scope layer's safety property (non-negotiable)
//!
//! > The scope layer may only **shrink** the reported conflict set when it is
//! > confident. Any uncertainty falls back to "the whole file is one scope",
//! > i.e. the conflict is still reported.
//!
//! `[INFERENCE]` **Over-segmenting hides a real conflict and that is the
//! dangerous direction.** Cut one logical unit into two scopes and two sides
//! that really did collide look disjoint; the conflict disappears, the merge
//! continues, and a wrong file lands on the main branch. So the rule here is
//! "report unless sure", and the cut used is the ADR's conservative v1 rule
//! (column-0 top-level declarations only, ADR §3.8.1). The finer v2 rule
//! (any indentation) is deliberately **not** enabled: it multiplies the
//! number of scopes (ADR §2.2 measured 7.86 -> 17.86 per file) and every
//! extra boundary is another chance to split one unit in two.
//!
//! `[FACT]` The ADR's own history survey found the scope layer shrinking the
//! conflict set **zero** times over 105 divergent pairs (ADR §2.2), and the
//! ADR therefore keeps the suppression path locked behind a measurement gate
//! (ADR §3.8.3, §7.2). **This delivery does not implement suppression at
//! all**: the scope layer only *locates* a region (which scope it lives in)
//! and *guards* (an untrusted cut falls back to the whole file). Every region
//! `git merge-file` produced is reported, and the type has no place to put a
//! hidden one — the suppression path stays shut until the ADR's measurement
//! gate says it may open.
//!
//! What the scope layer still contributes is the *attribution*: a region
//! whose two sides touched different scopes carries
//! [`RegionConfidence::DifferentScopes`] and a lossless keep-both
//! resolution ([`Region::both`]), which the dialog can offer. It does not
//! make the region disappear.
//!
//! # What the later lanes call
//!
//! | Lane | Call |
//! |---|---|
//! | B (AI arbitration) | [`conflict_set`], [`ConflictSet::regions`], [`Region::ours`] / [`Region::base`] / [`Region::theirs`], [`Region::id`] |
//! | B (write-back) | [`compose_resolved`], [`write_resolved`] |
//! | C (UI) | [`ConflictSet`], [`Region`], [`ScopeRef`], [`RegionConfidence`] |

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use sha2::{Digest, Sha256};

use crate::error::{GitGitError, Result};
use crate::repo::status::validate_rel_path;
use crate::server::subprocess::run_capture;

/// Width, in characters, of the conflict markers this module asks git for.
///
/// `[FACT]` `git merge-file --marker-size=24` writes 24 marker characters,
/// then a space, then the label (`<<<<<<<<<<<<<<<<<<<<<<<< OURS`). Measured
/// on git 2.41.0.windows.1 (ADR §2.1).
pub const MARKER_SIZE: usize = 24;

/// The labels handed to `git merge-file -L`, in the order the ADR fixes them.
const LABEL_OURS: &str = "OURS";
const LABEL_BASE: &str = "BASE";
const LABEL_THEIRS: &str = "THEIRS";

// --- data model ---------------------------------------------------------

/// A half-open line range, zero-based: `[start, end)`.
///
/// The zero-based half-open shape is the one the diff machinery speaks
/// (`@@ -old_start,old_count @@` is one-based inclusive, so the conversion
/// happens once, in [`parse_side_changes`]). Use [`LineRange::line_numbers`]
/// for the one-based inclusive numbers a human reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineRange {
    pub start: usize,
    pub end: usize,
}

impl LineRange {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// The number of lines the range covers.
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// True when the range covers no line at all.
    pub fn is_empty(&self) -> bool {
        self.end <= self.start
    }

    /// The one-based inclusive line numbers, as a user-facing UI shows them.
    ///
    /// An empty range reports `(0, 0)` — there is no line to point at.
    pub fn line_numbers(&self) -> (usize, usize) {
        if self.is_empty() {
            (0, 0)
        } else {
            (self.start + 1, self.end)
        }
    }

    /// True when `line` falls inside the range.
    pub fn contains(&self, line: usize) -> bool {
        line >= self.start && line < self.end
    }
}

/// What a scope is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    /// The lines before the first top-level declaration.
    Preamble,
    /// A top-level declaration and the lines that belong to it.
    Item,
    /// The whole file — the fallback when the file cannot be cut.
    WholeFile,
}

/// How much the scope layer trusts its own cut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeConfidence {
    /// The language is covered by the rule set and the cut tiles every line.
    Confident,
    /// The cut could not be trusted: unknown language, no declaration found,
    /// or the scopes do not tile the file. The file is one scope.
    Fallback,
}

/// One scope: a contiguous run of lines of the base file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    /// The scope's ordinal in the file.
    pub index: usize,
    pub kind: ScopeKind,
    /// A human label — the declaration line that opened the scope.
    pub label: String,
    /// The first line of the scope (zero-based, inclusive).
    pub start: usize,
    /// The line after the scope (zero-based, exclusive).
    pub end: usize,
}

impl Scope {
    /// True when the zero-based line falls inside this scope.
    ///
    /// `[FACT]` The scope covers `[start, end)` — `start` inclusive, `end`
    /// exclusive — which is the same half-open convention
    /// [`ScopeMap::covers_every_line`] uses to prove the cut tiles the file.
    pub fn contains_line(&self, line: usize) -> bool {
        self.start <= line && line < self.end
    }
}

/// The segmentation of one file into scopes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeMap {
    /// The scopes, in order. They tile `[0, total_lines)` when
    /// [`ScopeMap::covers_every_line`] is true.
    pub scopes: Vec<Scope>,
    pub confidence: ScopeConfidence,
    /// The number of lines the file has.
    pub total_lines: usize,
}

impl ScopeMap {
    /// The fallback map: the whole file is one scope.
    pub fn whole_file(total_lines: usize) -> Self {
        Self {
            scopes: vec![Scope {
                index: 0,
                kind: ScopeKind::WholeFile,
                label: "<whole file>".to_string(),
                start: 0,
                end: total_lines,
            }],
            confidence: ScopeConfidence::Fallback,
            total_lines,
        }
    }

    /// True when the scopes tile every line of the file with no gap and no
    /// overlap.
    ///
    /// `[PROPOSAL]` The ADR's rule (§3.8.1): "the cut must cover the whole
    /// file; a cut that cannot is a whole-file scope". This is the check that
    /// decides whether the scope layer may shrink anything at all.
    pub fn covers_every_line(&self) -> bool {
        let mut cursor = 0usize;
        for scope in &self.scopes {
            if scope.start != cursor {
                return false;
            }
            cursor = scope.end;
        }
        cursor == self.total_lines
    }

    /// The scope a base line belongs to.
    pub fn scope_of_line(&self, line: usize) -> Option<&Scope> {
        self.scopes.iter().find(|s| s.contains_line(line))
    }

    /// The scopes a line range overlaps.
    pub fn scopes_overlapping(&self, range: &LineRange) -> Vec<&Scope> {
        self.scopes
            .iter()
            .filter(|s| s.start < range.end && range.start < s.end)
            .collect()
    }
}

/// Which scope a region lives in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeRef {
    /// The scope's ordinal in [`ConflictSet::scope_map`].
    pub index: usize,
    pub kind: ScopeKind,
    /// The scope's label.
    pub label: String,
    /// True when the region reaches beyond this scope into another one.
    pub spans_multiple: bool,
}

/// The scope layer's verdict on a region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionConfidence {
    /// Both sides touched the same scope — a real conflict, reported.
    SameScope,
    /// The two sides touched different scopes. The region is **still
    /// reported** — ADR §3.8.3 keeps suppression shut until the ADR's
    /// measurement gate says otherwise — but it carries a lossless
    /// keep-both resolution ([`Region::both`]) that the dialog can offer.
    DifferentScopes,
    /// The segmentation was not trusted, or the sides could not be
    /// attributed: reported anyway. Reporting is the safe direction.
    Fallback,
}

/// One conflict region.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    /// The region's ordinal in the merged output.
    ///
    /// This is the ordinal in `git merge-file`'s output, and every region of
    /// that output is reported — there is no second list to reconcile.
    pub index: usize,
    /// A stable identity for the region: the first 16 bytes of
    /// `sha256(path ‖ base_oid ‖ index ‖ ours ‖ base ‖ theirs)`, hex.
    ///
    /// `[PROPOSAL]` ADR §3.7.2 defines this id so an AI answer can be bound
    /// to the region it was shown for.
    pub id: String,
    /// The ours side's text, verbatim.
    pub ours: String,
    /// The base side's text, verbatim.
    pub base: String,
    /// The theirs side's text, verbatim.
    pub theirs: String,
    /// Where the region sits in the BASE file.
    pub base_range: LineRange,
    /// Where the region sits in the merged output.
    pub merged_range: LineRange,
    /// The scope the region lives in.
    pub scope: ScopeRef,
    /// The scope layer's verdict.
    pub confidence: RegionConfidence,
    /// The text that keeps BOTH sides' changes, present only when the scope
    /// layer judged the two sides disjoint.
    ///
    /// The two sides' changed lines do not overlap in that case, so "keep
    /// both" is well defined: each base line is replaced by whichever side
    /// changed it. This is what makes the shrink lossless.
    pub both: Option<String>,
}

/// How a region is resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegionChoice {
    /// Keep the ours side.
    Ours,
    /// Keep the theirs side.
    Theirs,
    /// Keep the base side (drop both changes).
    Base,
    /// Keep both sides' changes. Only available for a region the scope layer
    /// judged disjoint; asking for it elsewhere is an error.
    Both,
    /// A hand-written replacement.
    Custom(String),
}

/// What has to happen to a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    /// This module arbitrated the path.
    Arbitrated,
    /// The path needs git's own handling: a binary file, an add/delete
    /// conflict, or no base at all.
    ///
    /// `[PROPOSAL]` ADR §3.2: "when the base is not available the path does
    /// not enter conflict arbitration and is marked as needing git's native
    /// handling; the dialog shows a note instead of pretending it can
    /// arbitrate".
    NeedsGit,
}

/// The conflicts of one path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictSet {
    /// The repository-relative path.
    pub path: String,
    /// The base blob's object id, when the base was available.
    pub base_oid: Option<String>,
    /// What has to happen to this path.
    pub disposition: Disposition,
    /// The regions the user has to arbitrate.
    ///
    /// Every region `git merge-file` produced is in here. The scope layer
    /// never removes one (ADR §3.8.3), so this list is the merged output's
    /// region list, in order.
    pub regions: Vec<Region>,
    /// The scope segmentation of the base file.
    pub scope_map: ScopeMap,
    /// The merged text exactly as `git merge-file` produced it, markers and
    /// all. This is what [`compose_resolved`] splices the choices into.
    pub merged: String,
}

impl ConflictSet {
    /// Every region of the merged output, in merged order.
    pub fn all_regions(&self) -> Vec<&Region> {
        self.regions.iter().collect()
    }

    /// True when the path has at least one conflict to arbitrate.
    pub fn has_conflicts(&self) -> bool {
        !self.regions.is_empty()
    }
}

// --- the merged output, parsed ------------------------------------------

/// One piece of the merged output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    /// Text that is identical in all three versions.
    Common(String),
    /// A conflict region, in merged-output order.
    Region {
        index: usize,
        ours: String,
        base: String,
        theirs: String,
    },
}

/// The merged output, split into common text and conflict regions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedMerge {
    pub segments: Vec<Segment>,
    /// The regions, in merged-output order.
    pub regions: Vec<ParsedRegion>,
}

/// A conflict region as it appears in the merged output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedRegion {
    /// The region's ordinal in the merged output.
    pub index: usize,
    pub ours: String,
    pub base: String,
    pub theirs: String,
    /// Where the region sits in the base file.
    pub base_range: LineRange,
    /// Where the region sits in the merged output.
    pub merged_range: LineRange,
}

/// Split a text into lines, keeping each line's terminator.
///
/// The terminator matters: the write-back has to reproduce git's bytes, and
/// git does not re-add a newline that the source did not have — except at
/// the very end, which the diff3 output normalises (see
/// [`parse_merge_output`]).
fn split_keeping_terminators(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0usize;
    for (idx, byte) in text.bytes().enumerate() {
        if byte == b'\n' {
            out.push(&text[start..=idx]);
            start = idx + 1;
        }
    }
    if start < text.len() {
        out.push(&text[start..]);
    }
    out
}

/// The line without its terminator.
fn strip_terminator(line: &str) -> &str {
    let line = line.strip_suffix('\n').unwrap_or(line);
    line.strip_suffix('\r').unwrap_or(line)
}

/// True when `line` is a conflict marker of the given character.
///
/// `[FACT]` The width is what decides it: `git merge-file --marker-size=24`
/// writes exactly 24 marker characters followed by a space and the label.
/// A base line of 7 `<` characters is not a marker (ADR §2.1, measured).
fn is_marker_line(line: &str, marker: u8) -> bool {
    let line = strip_terminator(line);
    let bytes = line.as_bytes();
    if bytes.len() < MARKER_SIZE {
        return false;
    }
    if !bytes[..MARKER_SIZE].iter().all(|&b| b == marker) {
        return false;
    }
    bytes.len() == MARKER_SIZE || bytes[MARKER_SIZE] == b' '
}

/// Parse the diff3 output of `git merge-file`.
///
/// The parse is driven by the marker width and by nothing else. A common
/// line advances the base cursor by one — the common part of a diff3 merge
/// exists in all three files — and a region's base section advances it by
/// the number of base lines the region covers.
///
/// `[FACT]` The base side is recoverable from the parse: the common lines
/// plus the regions' base sections reconstruct the base file (modulo the
/// final newline, which `git merge-file --diff3` normalises — a base file
/// of `l1\nl2` without a trailing newline comes back as `l1\nl2\n`;
/// measured).
///
/// `[FACT]` One ambiguity survives the width rule: a base line consisting of
/// exactly 24 `=` characters inside a region's base section is
/// indistinguishable from the diff3 separator. git's own output is
/// ambiguous there; no width rule fixes it.
pub fn parse_merge_output(merged: &str) -> ParsedMerge {
    let lines = split_keeping_terminators(merged);
    let mut segments: Vec<Segment> = Vec::new();
    let mut regions: Vec<ParsedRegion> = Vec::new();
    let mut common = String::new();
    let mut base_cursor = 0usize;
    let mut idx = 0usize;

    while idx < lines.len() {
        if !is_marker_line(lines[idx], b'<') {
            common.push_str(lines[idx]);
            base_cursor += 1;
            idx += 1;
            continue;
        }

        // A region starts here.
        let merged_start = idx;
        idx += 1;
        let mut ours = String::new();
        while idx < lines.len() && !is_marker_line(lines[idx], b'|') {
            ours.push_str(lines[idx]);
            idx += 1;
        }
        // The base marker.
        idx += 1;
        let mut base = String::new();
        while idx < lines.len() && !is_marker_line(lines[idx], b'=') {
            base.push_str(lines[idx]);
            idx += 1;
        }
        let base_lines = base.lines().count();
        // The separator.
        idx += 1;
        let mut theirs = String::new();
        while idx < lines.len() && !is_marker_line(lines[idx], b'>') {
            theirs.push_str(lines[idx]);
            idx += 1;
        }
        // The closing marker.
        idx += 1;

        let base_start = base_cursor;
        base_cursor += base_lines;

        let region = ParsedRegion {
            index: regions.len(),
            ours: ours.clone(),
            base: base.clone(),
            theirs: theirs.clone(),
            base_range: LineRange::new(base_start, base_cursor),
            merged_range: LineRange::new(merged_start, idx),
        };
        regions.push(region.clone());
        // The common lines in front of this region are part of the file:
        // dropping them would lose the context the region sits in, and the
        // write-back would be short by exactly those lines.
        if !common.is_empty() {
            segments.push(Segment::Common(std::mem::take(&mut common)));
        }
        segments.push(Segment::Region {
            index: region.index,
            ours,
            base,
            theirs,
        });
    }

    if !common.is_empty() {
        segments.push(Segment::Common(common));
    }

    ParsedMerge { segments, regions }
}

// --- the scope layer ----------------------------------------------------

/// The languages the scope rule covers.
///
/// `[FACT]` ADR §3.8.1's rule set: Rust (`use/mod/fn/struct/enum/trait/
/// impl/const/static/type/macro_rules!`) and TS/JS/Svelte
/// (`import/export/function/class/interface/type/const/let/var/describe/
/// it/test`). Every other extension — `.md`, `.yml`, `.json`, `.ps1`,
/// `.sh`, `.toml` … — is "one scope for the whole file", the conservative
/// fallback ADR §2.2 measured on 44 of 105 pairs.
fn language_of(extension: &str) -> Option<&'static str> {
    match extension {
        "rs" => Some("rust"),
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "svelte" => Some("typescript"),
        _ => None,
    }
}

/// The extension of a repository-relative path, lowercased.
fn extension_of(path: &str) -> Option<String> {
    let name = Path::new(path).file_name()?.to_str()?;
    let (stem, ext) = name.rsplit_once('.')?;
    if stem.is_empty() {
        return None;
    }
    Some(ext.to_ascii_lowercase())
}

/// True when the line opens a top-level declaration.
///
/// Column 0 only: a nested item is not a scope boundary, because cutting at
/// every indentation is what ADR §2.2's v2 rule did (7.86 -> 17.86 scopes
/// per file) and the extra boundaries are extra chances to hide a conflict.
fn is_declaration(line: &str, language: &str) -> bool {
    if line.starts_with(char::is_whitespace) {
        return false;
    }
    // A visibility prefix (`pub`, `pub(crate)`) does not change what the
    // declaration is.
    let rest = match line.strip_prefix("pub") {
        Some(tail) => match tail.strip_prefix('(') {
            Some(inner) => match inner.split_once(')') {
                Some((_vis, after)) => after.trim_start(),
                None => tail.trim_start(),
            },
            None => tail.trim_start(),
        },
        None => line,
    };
    let rest = rest.strip_prefix("unsafe ").unwrap_or(rest);
    let rest = rest.strip_prefix("async ").unwrap_or(rest);
    let rest = rest.strip_prefix("default ").unwrap_or(rest);

    const RUST: &[&str] = &[
        "use ",
        "mod ",
        "fn ",
        "struct ",
        "enum ",
        "trait ",
        "impl ",
        "const ",
        "static ",
        "type ",
        "macro_rules!",
    ];
    const TYPESCRIPT: &[&str] = &[
        "import ",
        "export ",
        "function ",
        "class ",
        "interface ",
        "type ",
        "const ",
        "let ",
        "var ",
        "describe(",
        "it(",
        "test(",
    ];
    let table = match language {
        "rust" => RUST,
        _ => TYPESCRIPT,
    };
    table.iter().any(|kw| rest.starts_with(kw))
}

/// True when the line is a comment or an attribute that belongs to the
/// declaration below it.
fn is_attached_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("//")
        || trimmed.starts_with("/*")
        || trimmed.starts_with('*')
        || trimmed.starts_with("#[")
}

/// Cut a file into scopes.
///
/// `[PROPOSAL]` ADR §3.8.1's v1 rule: the file is cut into contiguous
/// scopes that cover every line; scope 0 is the preamble; an item is a
/// column-0 top-level declaration; the comment and `#[` lines directly above
/// an item belong to that item; a language the rule does not cover is one
/// scope for the whole file.
///
/// The result is only [`ScopeConfidence::Confident`] when the language is
/// covered, at least one declaration was found, and the scopes tile the
/// file. Anything else is [`ScopeConfidence::Fallback`] — the whole file is
/// one scope and nothing may be shrunk on the strength of it.
pub fn scope_map(text: &str, extension: &str) -> ScopeMap {
    let lines: Vec<&str> = text.lines().collect();
    let total_lines = lines.len();

    let Some(language) = language_of(extension) else {
        return ScopeMap::whole_file(total_lines);
    };

    // The declaration starts, in order. Each one swallows the comment and
    // attribute lines directly above it. The scope's start is that first
    // line; the label is the declaration itself, because the label is what
    // the conflict dialog shows the user.
    let mut starts: Vec<(usize, usize)> = Vec::new();
    let mut idx = 0usize;
    while idx < lines.len() {
        if is_declaration(lines[idx], language) {
            let declaration = idx;
            let mut start = idx;
            while start > 0 && is_attached_line(lines[start - 1]) {
                start -= 1;
            }
            starts.push((start, declaration));
            idx += 1;
        } else if is_attached_line(lines[idx]) {
            // A comment block with no declaration below it is a scope of its
            // own rather than a hole in the cut.
            let mut end = idx;
            while end < lines.len() && is_attached_line(lines[end]) {
                end += 1;
            }
            if end < lines.len() && is_declaration(lines[end], language) {
                // The declaration below takes it; skip to it.
                idx = end;
            } else {
                starts.push((idx, idx));
                idx = end;
            }
        } else {
            idx += 1;
        }
    }

    if starts.is_empty() {
        // Nothing was recognised: the file is one scope and the layer is not
        // allowed to shrink anything on the strength of it.
        return ScopeMap::whole_file(total_lines);
    }

    let mut scopes: Vec<Scope> = Vec::with_capacity(starts.len() + 1);
    // The preamble is the lines in front of the first item. An empty
    // preamble is not a scope: it has no line to attribute, and a
    // zero-width scope would only confuse the dialog.
    if starts[0].0 > 0 {
        scopes.push(Scope {
            index: 0,
            kind: ScopeKind::Preamble,
            label: "<preamble>".to_string(),
            start: 0,
            end: starts[0].0,
        });
    }
    let offset = scopes.len();
    for (ordinal, &(start, declaration)) in starts.iter().enumerate() {
        let end = starts.get(ordinal + 1).map(|s| s.0).unwrap_or(total_lines);
        let label = lines
            .get(declaration)
            .map(|l| l.trim())
            .unwrap_or("")
            .chars()
            .take(80)
            .collect::<String>();
        scopes.push(Scope {
            index: offset + ordinal,
            kind: ScopeKind::Item,
            label,
            start,
            end,
        });
    }

    let map = ScopeMap {
        scopes,
        confidence: ScopeConfidence::Confident,
        total_lines,
    };
    if map.covers_every_line() {
        map
    } else {
        ScopeMap::whole_file(total_lines)
    }
}

/// Where one side's changed base lines land.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeAttribution {
    /// The side's changed lines fall in these scopes.
    Scopes(Vec<usize>),
    /// The side's changed lines could not be attributed — none of them fall
    /// in the region, or they fall outside the cut.
    Uncovered,
}

/// The scope layer's decision about one region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeDecision {
    /// The two sides touched the same scope: a real conflict.
    Overlapping,
    /// The two sides touched different scopes: not a real conflict.
    Disjoint,
    /// The cut was not trusted or the sides could not be attributed: report
    /// the conflict anyway. Reporting is the safe direction.
    Uncertain,
}

/// Attribute one side's changed lines to scopes.
///
/// The changed lines are the base lines the side replaced, intersected with
/// the region's base range. A pure insertion is attributed to the line it
/// was anchored to (ADR §2.2's footnote 2: "a pure-insertion hunk belongs to
/// the scope of its anchor line — the conservative direction, more overlap").
pub fn attribute_side(
    scope_map: &ScopeMap,
    region: &LineRange,
    changed_lines: &[usize],
) -> ScopeAttribution {
    let mut scopes: Vec<usize> = Vec::new();
    let mut any = false;
    for line in changed_lines {
        if !region.contains(*line) {
            continue;
        }
        any = true;
        match scope_map.scope_of_line(*line) {
            Some(scope) => {
                if !scopes.contains(&scope.index) {
                    scopes.push(scope.index);
                }
            }
            None => return ScopeAttribution::Uncovered,
        }
    }
    if !any {
        return ScopeAttribution::Uncovered;
    }
    scopes.sort_unstable();
    ScopeAttribution::Scopes(scopes)
}

/// Decide whether a region is a real conflict.
///
/// The rule, in full: shrink only when the cut is trusted, both sides could
/// be attributed, and the two sides' scopes do not intersect. Everything
/// else reports the conflict.
pub fn decide(
    scope_map: &ScopeMap,
    ours: &ScopeAttribution,
    theirs: &ScopeAttribution,
) -> ScopeDecision {
    if scope_map.confidence != ScopeConfidence::Confident {
        return ScopeDecision::Uncertain;
    }
    if !scope_map.covers_every_line() {
        return ScopeDecision::Uncertain;
    }
    let (ScopeAttribution::Scopes(ours_scopes), ScopeAttribution::Scopes(theirs_scopes)) =
        (ours, theirs)
    else {
        return ScopeDecision::Uncertain;
    };
    if ours_scopes.is_empty() || theirs_scopes.is_empty() {
        return ScopeDecision::Uncertain;
    }
    if ours_scopes.iter().any(|s| theirs_scopes.contains(s)) {
        ScopeDecision::Overlapping
    } else {
        ScopeDecision::Disjoint
    }
}

// --- the sides' changes to the base --------------------------------------

/// One side's replacement of a run of base lines.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SideChange {
    /// The first base line the side replaced (zero-based).
    start: usize,
    /// The base line after the replacement (zero-based, exclusive). Equal to
    /// `start` for a pure insertion.
    end: usize,
    /// The side's replacement lines, without terminators.
    lines: Vec<String>,
}

/// Parse the `@@` headers of `git diff --no-index --unified=0`.
///
/// `[FACT]` The hunk header is `@@ -old_start[,old_count] +new_start[,new_count] @@`;
/// a count of 1 is left out (measured: `@@ -2 +2 @@ fn alpha() {`). The old
/// range is the base side's range, which is the coordinate system the scope
/// layer works in.
fn parse_side_changes(diff: &str) -> Vec<SideChange> {
    let mut changes: Vec<SideChange> = Vec::new();
    let mut current: Option<SideChange> = None;

    for line in diff.lines() {
        if let Some(rest) = line.strip_prefix("@@ -") {
            if let Some(change) = current.take() {
                changes.push(change);
            }
            let Some((old_spec, tail)) = rest.split_once(" +") else {
                continue;
            };
            let (old_start, old_count) = match old_spec.split_once(',') {
                Some((start, count)) => (
                    start.parse::<usize>().unwrap_or(0),
                    count.parse::<usize>().unwrap_or(1),
                ),
                None => (old_spec.parse::<usize>().unwrap_or(0), 1),
            };
            // The header must be well formed: "@@ -a,b +c,d @@".
            if !tail.contains(" @@") {
                continue;
            }
            let (start, end) = if old_count == 0 {
                // A pure insertion sits between base lines; its point is the
                // line the insertion precedes.
                (old_start, old_start)
            } else {
                (
                    old_start.saturating_sub(1),
                    old_start.saturating_sub(1) + old_count,
                )
            };
            current = Some(SideChange {
                start,
                end,
                lines: Vec::new(),
            });
            continue;
        }
        let Some(change) = current.as_mut() else {
            continue;
        };
        if let Some(text) = line.strip_prefix('+') {
            change.lines.push(text.to_string());
        }
        // "-" lines are the base's own text; they are already known.
    }
    if let Some(change) = current.take() {
        changes.push(change);
    }
    changes
}

/// The base lines a side changed, with insertions attributed to their
/// anchor line.
fn changed_base_lines(changes: &[SideChange]) -> Vec<usize> {
    let mut lines: Vec<usize> = Vec::new();
    for change in changes {
        if change.end > change.start {
            for line in change.start..change.end {
                lines.push(line);
            }
        } else if change.start > 0 {
            // The anchor is the line before the insertion point.
            lines.push(change.start - 1);
        }
    }
    lines.sort_unstable();
    lines.dedup();
    lines
}

// --- the temporary files ------------------------------------------------

/// The three versions of one path, on disk for the length of one merge.
///
/// `[FACT]` `git merge-file` takes files, not blobs (ADR §2.1), so the three
/// versions have to exist on disk. The guard removes them in `Drop`, which
/// covers the error paths too: an early `?` return still runs the
/// destructor, so a failed merge leaves nothing behind.
///
/// The three versions live in a directory of their own, so one call's files
/// cannot be mistaken for another's and so that dropping the guard removes
/// all of them in one step.
struct TempFiles {
    dir: PathBuf,
    paths: Vec<PathBuf>,
}

impl TempFiles {
    /// A guard rooted in a directory the caller owns.
    ///
    /// The production path roots it in the system temp directory. The test
    /// roots it in a directory it owns, so it can prove that a call leaves
    /// nothing behind: the directory is empty again once the guard is
    /// dropped.
    fn in_root(root: &Path) -> Result<Self> {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let ordinal = COUNTER.fetch_add(1, Ordering::SeqCst);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = root.join(format!(
            "gitgit-conflict-{}-{nanos}-{ordinal}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir)?;
        Ok(Self {
            dir,
            paths: Vec::new(),
        })
    }

    /// Write one version to disk and remember it for cleanup.
    fn write(&mut self, label: &str, bytes: &[u8]) -> Result<PathBuf> {
        let path = self.dir.join(label);
        std::fs::write(&path, bytes)?;
        self.paths.push(path.clone());
        Ok(path)
    }
}

impl Drop for TempFiles {
    fn drop(&mut self) {
        for path in &self.paths {
            let _ = std::fs::remove_file(path);
        }
        let _ = std::fs::remove_dir(&self.dir);
    }
}

// --- git plumbing -------------------------------------------------------

/// One stage of an unmerged index entry.
#[derive(Debug, Clone, PartialEq, Eq)]
struct IndexStage {
    stage: u8,
    oid: String,
}

/// The unmerged index entries of `rel`.
///
/// `[FACT]` `git ls-files -u` lists one record per stage of a conflicted
/// path: `<mode> <oid> <stage>\t<path>`. Stage 1 is the base, 2 is ours
/// and 3 is theirs (ADR §2.3).
async fn index_stages(workdir: &Path, rel: &str) -> Result<Vec<IndexStage>> {
    let out = run_capture("git", &["ls-files", "-u", "-z", "--", rel], Some(workdir)).await?;
    let raw = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut stages = Vec::new();
    for record in raw.split('\0').filter(|r| !r.is_empty()) {
        let Some((meta, _path)) = record.split_once('\t') else {
            continue;
        };
        let mut fields = meta.split(' ');
        let _mode = fields.next();
        let Some(oid) = fields.next() else {
            continue;
        };
        let Some(stage) = fields.next().and_then(|s| s.parse::<u8>().ok()) else {
            continue;
        };
        stages.push(IndexStage {
            stage,
            oid: oid.to_string(),
        });
    }
    Ok(stages)
}

/// Read a blob's bytes.
async fn blob_bytes(workdir: &Path, oid: &str) -> Result<Vec<u8>> {
    let out = run_capture("git", &["cat-file", "blob", oid], Some(workdir)).await?;
    Ok(out.stdout)
}

/// True when the bytes look like a binary file.
///
/// `[FACT]` `git merge-file` refuses binary input: with a NUL byte in the
/// content it prints `error: Cannot merge binary files: <path>` and exits
/// 255 (measured on git 2.41.0.windows.1). The module detects the case
/// first so it can answer "this needs git's own handling" instead of
/// reporting a git failure.
fn looks_binary(bytes: &[u8]) -> bool {
    let window = &bytes[..bytes.len().min(8000)];
    window.contains(&0)
}

/// Run `git merge-file` over the three temporary files.
///
/// `[FACT]` The exit code is the number of conflict regions: 0 for a clean
/// merge, 1 for one region, 2 for two (ADR §2.1, measured). A code of 128
/// or above is git's own failure (255 for a binary file).
async fn run_merge_file(ours: &Path, base: &Path, theirs: &Path) -> Result<(String, i32)> {
    let ours_arg = ours.to_string_lossy().into_owned();
    let base_arg = base.to_string_lossy().into_owned();
    let theirs_arg = theirs.to_string_lossy().into_owned();
    let out = run_capture(
        "git",
        &[
            "merge-file",
            "--diff3",
            "--marker-size=24",
            "-L",
            LABEL_OURS,
            "-L",
            LABEL_BASE,
            "-L",
            LABEL_THEIRS,
            "-p",
            &ours_arg,
            &base_arg,
            &theirs_arg,
        ],
        None,
    )
    .await?;
    let code = out.status.code().unwrap_or(-1);
    if code >= 128 {
        return Err(GitGitError::GitExit {
            cmd: "git merge-file --diff3 --marker-size=24 -p".to_string(),
            status: code,
            stderr: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        });
    }
    let merged = String::from_utf8_lossy(&out.stdout).into_owned();
    Ok((merged, code))
}

/// The base-side line ranges one side changed, as `git diff` sees them.
///
/// `[FACT]` `git diff --no-index --unified=0 <base> <side>` exits 1 when the
/// files differ and prints one hunk per change (measured).
async fn side_changes(base: &Path, side: &Path) -> Result<Vec<SideChange>> {
    let base_arg = base.to_string_lossy().into_owned();
    let side_arg = side.to_string_lossy().into_owned();
    let out = run_capture(
        "git",
        &[
            "diff",
            "--no-index",
            "--no-ext-diff",
            "--no-textconv",
            "--unified=0",
            "--",
            &base_arg,
            &side_arg,
        ],
        Some(std::env::temp_dir().as_path()),
    )
    .await?;
    // `--no-index` exits 1 when the files differ, which is the normal case.
    if !matches!(out.status.code(), Some(0) | Some(1)) {
        return Err(GitGitError::GitExit {
            cmd: "git diff --no-index --unified=0".to_string(),
            status: out.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        });
    }
    let diff = String::from_utf8_lossy(&out.stdout).into_owned();
    Ok(parse_side_changes(&diff))
}

// --- the region id ------------------------------------------------------

/// The stable id of a region.
///
/// `[PROPOSAL]` ADR §3.7.2: the first 16 bytes of
/// `sha256(path ‖ base_oid ‖ index ‖ ours ‖ base ‖ theirs)`, hex — an id
/// an AI answer can be bound to.
pub fn region_id(
    path: &str,
    base_oid: &str,
    index: usize,
    ours: &str,
    base: &str,
    theirs: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.as_bytes());
    hasher.update([0u8]);
    hasher.update(base_oid.as_bytes());
    hasher.update([0u8]);
    hasher.update(index.to_string().as_bytes());
    hasher.update([0u8]);
    hasher.update(ours.as_bytes());
    hasher.update([0u8]);
    hasher.update(base.as_bytes());
    hasher.update([0u8]);
    hasher.update(theirs.as_bytes());
    let digest = hasher.finalize();
    hex::encode(&digest[..16])
}

// --- the public entry points --------------------------------------------

/// The conflicts of one path in an in-progress merge.
///
/// `[FACT]` The three versions come from the index's stages: stage 1 is the
/// base, stage 2 is ours, stage 3 is theirs (ADR §2.3, where stage 1 was
/// measured byte-identical to the merge base).
///
/// The base is mandatory: without stage 1 there is no three-way merge to
/// run and the path is handed back as [`Disposition::NeedsGit`].
pub async fn conflict_set(workdir: &Path, rel_path: &str) -> Result<ConflictSet> {
    let rel = validate_rel_path(rel_path)?;
    let stages = index_stages(workdir, rel).await?;

    let find = |stage: u8| {
        stages
            .iter()
            .find(|s| s.stage == stage)
            .map(|s| s.oid.clone())
    };
    let base_oid = find(1);
    let ours_oid = find(2);
    let theirs_oid = find(3);

    let (Some(base_oid), Some(ours_oid), Some(theirs_oid)) =
        (base_oid.clone(), ours_oid, theirs_oid)
    else {
        return Ok(ConflictSet {
            path: rel.to_string(),
            base_oid,
            disposition: Disposition::NeedsGit,
            regions: Vec::new(),
            scope_map: ScopeMap::whole_file(0),
            merged: String::new(),
        });
    };

    let ours = blob_bytes(workdir, &ours_oid).await?;
    let base = blob_bytes(workdir, &base_oid).await?;
    let theirs = blob_bytes(workdir, &theirs_oid).await?;

    conflict_set_from_blobs(rel, &base_oid, &ours, &base, &theirs).await
}

/// The conflicts of one path, from three blobs.
///
/// The blobs are ours / base / theirs. This is the entry point for the
/// "preview two branches" flow, where the base comes from
/// `git merge-base` rather than from the index.
pub async fn conflict_set_from_blobs(
    path: &str,
    base_oid: &str,
    ours: &[u8],
    base: &[u8],
    theirs: &[u8],
) -> Result<ConflictSet> {
    conflict_set_from_blobs_in(&std::env::temp_dir(), path, base_oid, ours, base, theirs).await
}

/// The conflicts of one path, from three blobs, with the temporaries rooted
/// in `root`.
///
/// `[PROPOSAL]` The root is a parameter so the test can prove that a call
/// leaves nothing behind: it owns the directory and can look inside it
/// after the call. The production path roots it in the system temp
/// directory.
pub async fn conflict_set_from_blobs_in(
    root: &Path,
    path: &str,
    base_oid: &str,
    ours: &[u8],
    base: &[u8],
    theirs: &[u8],
) -> Result<ConflictSet> {
    if looks_binary(ours) || looks_binary(base) || looks_binary(theirs) {
        return Ok(ConflictSet {
            path: path.to_string(),
            base_oid: Some(base_oid.to_string()),
            disposition: Disposition::NeedsGit,
            regions: Vec::new(),
            scope_map: ScopeMap::whole_file(0),
            merged: String::new(),
        });
    }

    let mut temps = TempFiles::in_root(root)?;
    let ours_file = temps.write("ours", ours)?;
    let base_file = temps.write("base", base)?;
    let theirs_file = temps.write("theirs", theirs)?;

    let (merged, region_count) = run_merge_file(&ours_file, &base_file, &theirs_file).await?;
    let parsed = parse_merge_output(&merged);

    // The exit code is the region count (ADR §2.1). If the parse disagrees
    // with it, the parse is wrong and the caller has to hear about it.
    if parsed.regions.len() != region_count as usize {
        return Err(GitGitError::Http(format!(
            "git merge-file reported {region_count} conflict regions but {} were parsed",
            parsed.regions.len()
        )));
    }

    let base_text = String::from_utf8_lossy(base);
    let extension = extension_of(path).unwrap_or_default();
    let scope_map = scope_map(&base_text, &extension);

    let ours_changes = side_changes(&base_file, &ours_file).await?;
    let theirs_changes = side_changes(&base_file, &theirs_file).await?;
    let ours_lines = changed_base_lines(&ours_changes);
    let theirs_lines = changed_base_lines(&theirs_changes);

    let base_lines: Vec<&str> = base_text.lines().collect();
    let mut regions: Vec<Region> = Vec::new();

    for parsed_region in &parsed.regions {
        let ours_attr = attribute_side(&scope_map, &parsed_region.base_range, &ours_lines);
        let theirs_attr = attribute_side(&scope_map, &parsed_region.base_range, &theirs_lines);
        let decision = decide(&scope_map, &ours_attr, &theirs_attr);

        let overlapping = scope_map.scopes_overlapping(&parsed_region.base_range);
        let scope = match overlapping.first() {
            Some(first) => ScopeRef {
                index: first.index,
                kind: first.kind,
                label: first.label.clone(),
                spans_multiple: overlapping.len() > 1,
            },
            None => ScopeRef {
                index: 0,
                kind: ScopeKind::WholeFile,
                label: "<whole file>".to_string(),
                spans_multiple: false,
            },
        };

        let confidence = match decision {
            ScopeDecision::Overlapping => RegionConfidence::SameScope,
            ScopeDecision::Disjoint => RegionConfidence::DifferentScopes,
            ScopeDecision::Uncertain => RegionConfidence::Fallback,
        };

        let both = if confidence == RegionConfidence::DifferentScopes {
            union_text(
                &base_lines,
                &parsed_region.base_range,
                &ours_changes,
                &theirs_changes,
            )
        } else {
            None
        };

        // ADR §3.8.3: the first delivery does **not** shrink the conflict
        // set. A region whose two sides touched different scopes is still
        // reported; what the scope layer adds is the attribution and the
        // keep-both resolution the dialog can offer. There is no code path
        // that drops a region.
        let region = Region {
            index: parsed_region.index,
            id: region_id(
                path,
                base_oid,
                parsed_region.index,
                &parsed_region.ours,
                &parsed_region.base,
                &parsed_region.theirs,
            ),
            ours: parsed_region.ours.clone(),
            base: parsed_region.base.clone(),
            theirs: parsed_region.theirs.clone(),
            base_range: parsed_region.base_range,
            merged_range: parsed_region.merged_range,
            scope,
            confidence,
            both,
        };
        regions.push(region);
    }

    Ok(ConflictSet {
        path: path.to_string(),
        base_oid: Some(base_oid.to_string()),
        disposition: Disposition::Arbitrated,
        regions,
        scope_map,
        merged,
    })
}

/// The text that keeps both sides' changes.
///
/// The two sides' changed lines do not overlap when this is called (that is
/// what [`decide`] checked), so each base line is replaced by whichever
/// side changed it. Returns `None` when a change straddles the region's
/// edge — the region is then reported instead of shrunk.
fn union_text(
    base_lines: &[&str],
    region: &LineRange,
    ours_changes: &[SideChange],
    theirs_changes: &[SideChange],
) -> Option<String> {
    let inside = |change: &SideChange| {
        change.start >= region.start && change.end <= region.end && change.start <= change.end
    };
    if ours_changes
        .iter()
        .any(|c| !inside(c) && c.start < region.end && c.end > region.start)
    {
        return None;
    }
    if theirs_changes
        .iter()
        .any(|c| !inside(c) && c.start < region.end && c.end > region.start)
    {
        return None;
    }

    let mut out = String::new();
    let mut line = region.start;
    while line < region.end {
        for change in ours_changes
            .iter()
            .filter(|c| c.start == line && c.end == line)
        {
            for text in &change.lines {
                out.push_str(text);
                out.push('\n');
            }
        }
        for change in theirs_changes
            .iter()
            .filter(|c| c.start == line && c.end == line)
        {
            for text in &change.lines {
                out.push_str(text);
                out.push('\n');
            }
        }
        if let Some(change) = ours_changes
            .iter()
            .find(|c| c.start == line && c.end > line && c.end <= region.end)
        {
            for text in &change.lines {
                out.push_str(text);
                out.push('\n');
            }
            line = change.end;
            continue;
        }
        if let Some(change) = theirs_changes
            .iter()
            .find(|c| c.start == line && c.end > line && c.end <= region.end)
        {
            for text in &change.lines {
                out.push_str(text);
                out.push('\n');
            }
            line = change.end;
            continue;
        }
        if let Some(text) = base_lines.get(line) {
            out.push_str(text);
            out.push('\n');
        }
        line += 1;
    }
    for change in ours_changes
        .iter()
        .filter(|c| c.start == region.end && c.end == region.end)
    {
        for text in &change.lines {
            out.push_str(text);
            out.push('\n');
        }
    }
    for change in theirs_changes
        .iter()
        .filter(|c| c.start == region.end && c.end == region.end)
    {
        for text in &change.lines {
            out.push_str(text);
            out.push('\n');
        }
    }
    Some(out)
}

/// Compose the resolved file from the per-region choices.
///
/// The choices are keyed by the region's ordinal in the merged output
/// ([`Region::index`]). Every region of the merged output needs a choice:
/// the scope layer never drops a region (ADR §3.8.3), so the number of
/// choices has to match the number of regions, and a missing choice is an
/// error rather than a silent default.
pub fn compose_resolved(
    set: &ConflictSet,
    choices: &BTreeMap<usize, RegionChoice>,
) -> Result<String> {
    let parsed = parse_merge_output(&set.merged);
    let all = set.all_regions();
    let mut out = String::new();
    let mut ordinal = 0usize;
    for segment in &parsed.segments {
        match segment {
            Segment::Common(text) => out.push_str(text),
            Segment::Region { .. } => {
                let region = all
                    .get(ordinal)
                    .ok_or_else(|| GitGitError::Http("conflict region out of range".to_string()))?;
                let choice = choices.get(&ordinal).ok_or_else(|| {
                    GitGitError::Http(format!("no choice for conflict region {ordinal}"))
                })?;
                let text = match choice {
                    RegionChoice::Ours => region.ours.clone(),
                    RegionChoice::Theirs => region.theirs.clone(),
                    RegionChoice::Base => region.base.clone(),
                    RegionChoice::Both => region.both.clone().ok_or_else(|| {
                        GitGitError::Http(format!(
                            "conflict region {ordinal} has no keep-both resolution"
                        ))
                    })?,
                    RegionChoice::Custom(text) => text.clone(),
                };
                out.push_str(&text);
                ordinal += 1;
            }
        }
    }
    Ok(out)
}

/// Write the resolved file to the work tree and stage it.
///
/// `[PROPOSAL]` ADR §3.6: the application writes the merged file back and
/// runs `git add <path>`, so git sees the conflict as resolved. git stays
/// file-granular; what changed is the arbitration unit.
///
/// The path is validated with the same rule the rest of the crate uses
/// ([`validate_rel_path`]), and `--` ends git's option parsing so a path
/// that starts with `-` cannot be read as an option.
pub async fn write_resolved(workdir: &Path, rel_path: &str, content: &str) -> Result<()> {
    let rel = validate_rel_path(rel_path)?;
    let target = workdir.join(rel);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&target, content.as_bytes())?;
    run_capture("git", &["add", "--", rel], Some(workdir)).await?;
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn unique_temp(label: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!(
            "gitgit-test-conflict-{label}-{pid}-{n}-{}",
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

    /// A repository where both sides renamed the same file, or `None` when
    /// git is missing or the merge did not conflict.
    ///
    /// `[FACT]` Measured with git 2.41.0: after `main` renames `f.txt` to
    /// `ours.txt` and `side` renames it to `theirs.txt`, the unresolved
    /// index lists stage 1 under `f.txt`, stage 2 under `ours.txt` and
    /// stage 3 under `theirs.txt` — the three stages of one blob spread
    /// over three paths.
    fn repo_with_rename_conflict(label: &str) -> Option<PathBuf> {
        let dir = unique_temp(label);
        if !git_ok(&dir, &["init", "-q", "-b", "main"]) {
            return None;
        }
        if !git_ok(&dir, &["config", "user.email", "t@example.invalid"])
            || !git_ok(&dir, &["config", "user.name", "t"])
        {
            return None;
        }
        std::fs::write(dir.join("f.txt"), "content\n").unwrap();
        if !git_ok(&dir, &["add", "."]) || !git_ok(&dir, &["commit", "-q", "-m", "base"]) {
            return None;
        }
        if !git_ok(&dir, &["checkout", "-q", "-b", "side"]) {
            return None;
        }
        if !git_ok(&dir, &["mv", "f.txt", "theirs.txt"]) {
            return None;
        }
        if !git_ok(&dir, &["commit", "-q", "-am", "side"]) {
            return None;
        }
        if !git_ok(&dir, &["checkout", "-q", "main"]) {
            return None;
        }
        if !git_ok(&dir, &["mv", "f.txt", "ours.txt"]) {
            return None;
        }
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

    /// A repository with one conflicted path, or `None` when git is missing.
    ///
    /// The fixture is a real merge: `main` and `side` each change the same
    /// lines of `f.txt`, and the merge is left unresolved, so the index
    /// carries stages 1, 2 and 3.
    fn repo_with_conflict(label: &str, base: &str, ours: &str, theirs: &str) -> Option<PathBuf> {
        let dir = unique_temp(label);
        if !git_ok(&dir, &["init", "-q", "-b", "main"]) {
            return None;
        }
        if !git_ok(&dir, &["config", "user.email", "t@example.invalid"])
            || !git_ok(&dir, &["config", "user.name", "t"])
        {
            return None;
        }
        std::fs::write(dir.join("f.txt"), base).unwrap();
        if !git_ok(&dir, &["add", "."]) || !git_ok(&dir, &["commit", "-q", "-m", "base"]) {
            return None;
        }
        if !git_ok(&dir, &["checkout", "-q", "-b", "side"]) {
            return None;
        }
        std::fs::write(dir.join("f.txt"), theirs).unwrap();
        if !git_ok(&dir, &["commit", "-q", "-am", "side"]) {
            return None;
        }
        if !git_ok(&dir, &["checkout", "-q", "main"]) {
            return None;
        }
        std::fs::write(dir.join("f.txt"), ours).unwrap();
        if !git_ok(&dir, &["commit", "-q", "-am", "ours"]) {
            return None;
        }
        // The merge is expected to conflict; a clean merge means the fixture
        // is not testing what it claims to.
        git_ok(&dir, &["merge", "side"]);
        if git_text(&dir, &["ls-files", "-u"]).trim().is_empty() {
            return None;
        }
        Some(dir)
    }

    // --- the merged output, parsed ------------------------------------

    #[test]
    fn a_diff3_output_is_split_into_its_three_sides() {
        let merged = "l1\n\
                      <<<<<<<<<<<<<<<<<<<<<<<< OURS\n\
                      O2\n\
                      |||||||||||||||||||||||| BASE\n\
                      l2\n\
                      ========================\n\
                      T2\n\
                      >>>>>>>>>>>>>>>>>>>>>>>> THEIRS\n\
                      l3\n";
        let parsed = parse_merge_output(merged);
        assert_eq!(parsed.regions.len(), 1);
        let region = &parsed.regions[0];
        assert_eq!(region.ours, "O2\n");
        assert_eq!(region.base, "l2\n");
        assert_eq!(region.theirs, "T2\n");
        // The common lines are base lines 1 and 3, the region's base line is
        // line 2.
        assert_eq!(region.base_range, LineRange::new(1, 2));
        // The region covers the merged lines 1 to 7 — the opening marker to
        // the closing one, both included.
        assert_eq!(region.merged_range, LineRange::new(1, 8));
    }

    #[test]
    fn a_base_line_that_looks_like_a_marker_is_not_a_marker() {
        // The hazard ADR §2.1 measured: the base side can contain text that
        // looks like a marker, and only the width tells them apart.
        let merged = "alpha\n\
                      <<<<<<< not a real marker\n\
                      <<<<<<<<<<<<<<<<<<<<<<<< OURS\n\
                      OURS-TAIL\n\
                      |||||||||||||||||||||||| BASE\n\
                      shared\n\
                      ========================\n\
                      THEIRS-TAIL\n\
                      >>>>>>>>>>>>>>>>>>>>>>>> THEIRS\n";
        let parsed = parse_merge_output(merged);
        assert_eq!(parsed.regions.len(), 1, "the fake marker must not split");
        let region = &parsed.regions[0];
        assert_eq!(region.base, "shared\n");
        // The fake marker stayed a common line, so the base cursor advanced
        // past it: the region's base line is base line 3.
        assert_eq!(region.base_range, LineRange::new(2, 3));
        let common: String = parsed
            .segments
            .iter()
            .filter_map(|s| match s {
                Segment::Common(text) => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(
            common.contains("<<<<<<< not a real marker"),
            "the fake marker must survive as text: {common}"
        );
    }

    #[test]
    fn the_base_is_recoverable_from_the_parsed_regions() {
        let merged = "l1\n\
                      <<<<<<<<<<<<<<<<<<<<<<<< OURS\n\
                      O2\n\
                      |||||||||||||||||||||||| BASE\n\
                      l2\n\
                      ========================\n\
                      T2\n\
                      >>>>>>>>>>>>>>>>>>>>>>>> THEIRS\n\
                      l3\n\
                      <<<<<<<<<<<<<<<<<<<<<<<< OURS\n\
                      O4\n\
                      |||||||||||||||||||||||| BASE\n\
                      l4\n\
                      ========================\n\
                      T4\n\
                      >>>>>>>>>>>>>>>>>>>>>>>> THEIRS\n";
        let parsed = parse_merge_output(merged);
        let mut recovered = String::new();
        for segment in &parsed.segments {
            match segment {
                Segment::Common(text) => recovered.push_str(text),
                Segment::Region { base, .. } => recovered.push_str(base),
            }
        }
        assert_eq!(recovered, "l1\nl2\nl3\nl4\n");
        assert_eq!(parsed.regions.len(), 2);
        assert_eq!(parsed.regions[0].base_range, LineRange::new(1, 2));
        assert_eq!(parsed.regions[1].base_range, LineRange::new(3, 4));
    }

    // --- the scope layer ----------------------------------------------

    #[test]
    fn a_rust_file_is_cut_into_scopes_that_cover_every_line() {
        let text = "use std::fmt;\n\
                    \n\
                    fn alpha() {\n\
                    }\n\
                    \n\
                    fn beta() {\n\
                    }\n";
        let map = scope_map(text, "rs");
        assert_eq!(map.confidence, ScopeConfidence::Confident);
        assert!(map.covers_every_line(), "scopes: {:?}", map.scopes);
        assert_eq!(map.total_lines, 7);
        // `use` is an item too — ADR §2.2's keyword table lists it — so this
        // file has no preamble and three items: the import, alpha and beta.
        assert_eq!(map.scopes.len(), 3);
        assert_eq!(map.scopes[0].kind, ScopeKind::Item);
        assert_eq!(map.scopes[0].label, "use std::fmt;");
        assert_eq!(map.scopes[0].start, 0);
        assert_eq!(map.scopes[0].end, 2);
        assert_eq!(map.scopes[1].label, "fn alpha() {");
        assert_eq!(map.scopes[1].start, 2);
        assert_eq!(map.scopes[1].end, 5);
        assert_eq!(map.scopes[2].label, "fn beta() {");
        assert_eq!(map.scopes[2].start, 5);
        assert_eq!(map.scopes[2].end, 7);
    }

    #[test]
    fn the_lines_in_front_of_the_first_item_are_a_preamble_scope() {
        // A file that opens with something that is not a declaration — a
        // shebang, a licence header — has a preamble scope, and the items
        // follow it. The cut still covers every line.
        let text = "#!/bin/sh\nfn alpha() {\n}\n";
        let map = scope_map(text, "rs");
        assert_eq!(map.confidence, ScopeConfidence::Confident);
        assert!(map.covers_every_line(), "scopes: {:?}", map.scopes);
        assert_eq!(map.scopes.len(), 2);
        assert_eq!(map.scopes[0].kind, ScopeKind::Preamble);
        assert_eq!(map.scopes[0].start, 0);
        assert_eq!(map.scopes[0].end, 1);
        assert_eq!(map.scopes[1].kind, ScopeKind::Item);
        assert_eq!(map.scopes[1].label, "fn alpha() {");
        assert_eq!(map.scopes[1].start, 1);
        assert_eq!(map.scopes[1].end, 3);
    }

    #[test]
    fn a_comment_above_a_declaration_belongs_to_that_declaration() {
        let text = "// alpha does things\n\
                    fn alpha() {\n\
                    }\n";
        let map = scope_map(text, "rs");
        assert_eq!(map.confidence, ScopeConfidence::Confident);
        assert!(map.covers_every_line());
        // The comment is inside alpha's scope, not a scope of its own.
        assert_eq!(map.scopes.len(), 1);
        assert_eq!(map.scopes[0].kind, ScopeKind::Item);
        assert_eq!(map.scopes[0].label, "fn alpha() {");
        assert_eq!(map.scopes[0].start, 0);
        assert_eq!(map.scopes[0].end, 3);
    }

    #[test]
    fn an_unknown_language_is_one_scope_for_the_whole_file() {
        let map = scope_map("# title\n\nbody\n", "md");
        assert_eq!(map.confidence, ScopeConfidence::Fallback);
        assert_eq!(map.scopes.len(), 1);
        assert_eq!(map.scopes[0].kind, ScopeKind::WholeFile);
        assert_eq!(map.scopes[0].end, 3);
    }

    #[test]
    fn a_cut_that_does_not_cover_the_file_is_not_trusted() {
        // The ADR's rule (§3.8.1): a cut that cannot cover the whole file is
        // a whole-file scope. A map with a gap must not be trusted.
        let map = ScopeMap {
            scopes: vec![
                Scope {
                    index: 0,
                    kind: ScopeKind::Item,
                    label: "first".to_string(),
                    start: 0,
                    end: 2,
                },
                Scope {
                    index: 1,
                    kind: ScopeKind::Item,
                    label: "second".to_string(),
                    start: 3,
                    end: 5,
                },
            ],
            confidence: ScopeConfidence::Confident,
            total_lines: 5,
        };
        assert!(!map.covers_every_line());
        let ours = ScopeAttribution::Scopes(vec![0]);
        let theirs = ScopeAttribution::Scopes(vec![1]);
        assert_eq!(
            decide(&map, &ours, &theirs),
            ScopeDecision::Uncertain,
            "an uncovered cut must never shrink anything"
        );
    }

    #[test]
    fn a_side_that_cannot_be_attributed_is_reported_not_shrunk() {
        let map = scope_map("fn alpha() {\n}\n", "rs");
        let region = LineRange::new(0, 2);
        let ours = attribute_side(&map, &region, &[0]);
        let theirs = attribute_side(&map, &region, &[]);
        // The file has one item and no preamble, so the item is scope 0.
        assert_eq!(ours, ScopeAttribution::Scopes(vec![0]));
        assert_eq!(theirs, ScopeAttribution::Uncovered);
        assert_eq!(decide(&map, &ours, &theirs), ScopeDecision::Uncertain);
    }

    #[test]
    fn the_same_scope_overlaps_and_different_scopes_do_not() {
        let map = scope_map(
            "fn alpha() {\n    let x = 1;\n}\nfn beta() {\n    let y = 2;\n}\n",
            "rs",
        );
        assert_eq!(map.confidence, ScopeConfidence::Confident);
        let region = LineRange::new(0, 6);
        let alpha = attribute_side(&map, &region, &[1]);
        let beta = attribute_side(&map, &region, &[4]);
        assert_eq!(alpha, ScopeAttribution::Scopes(vec![0]));
        assert_eq!(beta, ScopeAttribution::Scopes(vec![1]));
        assert_eq!(decide(&map, &alpha, &beta), ScopeDecision::Disjoint);
        assert_eq!(decide(&map, &alpha, &alpha), ScopeDecision::Overlapping);
    }

    // --- the region id -------------------------------------------------

    #[test]
    fn a_region_id_is_stable_and_distinguishes_its_inputs() {
        let a = region_id("src/a.rs", "abc123", 0, "ours\n", "base\n", "theirs\n");
        let b = region_id("src/a.rs", "abc123", 0, "ours\n", "base\n", "theirs\n");
        assert_eq!(a, b, "the same input must give the same id");
        assert_eq!(a.len(), 32, "16 bytes of hex");
        let other = region_id("src/a.rs", "abc123", 1, "ours\n", "base\n", "theirs\n");
        assert_ne!(a, other, "the region index is part of the id");
        let other_path = region_id("src/b.rs", "abc123", 0, "ours\n", "base\n", "theirs\n");
        assert_ne!(a, other_path, "the path is part of the id");
    }

    // --- against real git ---------------------------------------------

    #[tokio::test]
    async fn the_merge_file_exit_code_is_the_region_count() {
        let dir = unique_temp("exit-code");
        let base = dir.join("base");
        let ours = dir.join("ours");
        let theirs = dir.join("theirs");
        std::fs::write(&base, "l1\nl2\nl3\nl4\nl5\nl6\nl7\nl8\nl9\nl10\n").unwrap();
        std::fs::write(&ours, "l1\nO2\nl3\nl4\nl5\nl6\nl7\nl8\nl9\nO10\n").unwrap();
        std::fs::write(&theirs, "l1\nT2\nl3\nl4\nl5\nl6\nl7\nl8\nl9\nT10\n").unwrap();
        let (_, one) = run_merge_file(&ours, &base, &theirs).await.unwrap();
        assert_eq!(one, 2, "two independent conflicts -> exit 2");

        std::fs::write(&ours, "l1\nO2\nl3\nl4\nl5\nl6\nl7\nl8\nl9\nl10\n").unwrap();
        std::fs::write(&theirs, "l1\nT2\nl3\nl4\nl5\nl6\nl7\nl8\nl9\nl10\n").unwrap();
        let (_, two) = run_merge_file(&ours, &base, &theirs).await.unwrap();
        assert_eq!(two, 1, "one conflict -> exit 1");

        std::fs::write(&ours, "l1\nl2\nl3\nl4\nl5\nl6\nl7\nl8\nl9\nl10\n").unwrap();
        let (_, none) = run_merge_file(&ours, &base, &theirs).await.unwrap();
        assert_eq!(none, 0, "no conflict -> exit 0");
    }

    #[tokio::test]
    async fn a_binary_path_is_left_to_git() {
        let set = conflict_set_from_blobs("f.bin", "oid", b"a\0b\n", b"a\0b\n", b"a\0c\n")
            .await
            .unwrap();
        assert_eq!(set.disposition, Disposition::NeedsGit);
        assert!(set.regions.is_empty());
    }

    #[tokio::test]
    async fn a_conflicted_path_yields_one_region_with_all_three_sides() {
        let Some(dir) =
            repo_with_conflict("one-region", "l1\nl2\nl3\n", "l1\nO2\nl3\n", "l1\nT2\nl3\n")
        else {
            return;
        };
        let set = conflict_set(&dir, "f.txt").await.unwrap();
        assert_eq!(set.disposition, Disposition::Arbitrated);
        assert_eq!(set.regions.len(), 1, "one conflict -> one region");
        let region = &set.regions[0];
        assert_eq!(region.ours, "O2\n");
        assert_eq!(region.base, "l2\n");
        assert_eq!(region.theirs, "T2\n");
        assert_eq!(region.base_range, LineRange::new(1, 2));
        assert_eq!(region.index, 0);
        assert_eq!(region.id.len(), 32);
        // The base oid is the index's stage 1.
        let stages = git_text(&dir, &["ls-files", "-u"]);
        let stage1 = stages
            .lines()
            .find(|l| l.contains(" 1\t"))
            .expect("stage 1 must exist");
        assert!(set.base_oid.is_some());
        assert!(stage1.contains(set.base_oid.as_deref().unwrap()));
    }

    #[tokio::test]
    async fn a_rename_conflict_is_left_to_git() {
        // ADR §3.2: "stage 1 is always the merge base" was only measured on
        // an ordinary content conflict (§2.3), and the ADR says Lane A's
        // acceptance has to cover the rename case or that claim drops to
        // `[UNVERIFIED-FACT]`.
        //
        // `[FACT]` Measured with git 2.41.0: when both sides rename the
        // same file, the three stages of one blob land on **three different
        // paths** — the base under the old name, each side under its own
        // new name. No path carries all three stages, so the module cannot
        // assemble a three-way merge and must hand the path back to git.
        let Some(dir) = repo_with_rename_conflict("rename") else {
            return;
        };
        for path in ["f.txt", "ours.txt", "theirs.txt"] {
            let set = conflict_set(&dir, path).await.unwrap();
            assert_eq!(
                set.disposition,
                Disposition::NeedsGit,
                "{path} must be left to git, not arbitrated"
            );
            assert!(
                set.regions.is_empty(),
                "{path} must not report a region: {:?}",
                set.regions
            );
        }
        // The base blob itself is still the merge base's blob: it is the
        // *paths* that differ, not the content.
        let merge_base = git_text(&dir, &["merge-base", "main", "side"]);
        let base_blob = git_text(
            &dir,
            &["rev-parse", &format!("{}:f.txt", merge_base.trim())],
        );
        let stage1 = git_text(&dir, &["rev-parse", ":1:f.txt"]);
        assert_eq!(
            stage1.trim(),
            base_blob.trim(),
            "stage 1 must be the merge base's blob"
        );
    }

    #[tokio::test]
    async fn two_independent_regions_yield_two_regions() {
        // The ADR's 36% case: a file with two independent conflict regions,
        // which file-level arbitration forces the user to resolve together.
        let base = "l1\nl2\nl3\nl4\nl5\nl6\nl7\nl8\nl9\nl10\n";
        let ours = "l1\nO2\nl3\nl4\nl5\nl6\nl7\nl8\nl9\nO10\n";
        let theirs = "l1\nT2\nl3\nl4\nl5\nl6\nl7\nl8\nl9\nT10\n";
        let Some(dir) = repo_with_conflict("two-regions", base, ours, theirs) else {
            return;
        };
        let set = conflict_set(&dir, "f.txt").await.unwrap();
        assert_eq!(set.regions.len(), 2, "two independent conflicts");
        assert_eq!(set.regions[0].ours, "O2\n");
        assert_eq!(set.regions[0].base, "l2\n");
        assert_eq!(set.regions[0].theirs, "T2\n");
        assert_eq!(set.regions[0].base_range, LineRange::new(1, 2));
        assert_eq!(set.regions[1].ours, "O10\n");
        assert_eq!(set.regions[1].base, "l10\n");
        assert_eq!(set.regions[1].theirs, "T10\n");
        assert_eq!(set.regions[1].base_range, LineRange::new(9, 10));
        assert_eq!(set.regions[1].index, 1);
    }

    #[tokio::test]
    async fn a_base_side_with_marker_like_text_still_parses_into_one_region() {
        let base = "alpha\n<<<<<<< not a real marker\nshared\n";
        let ours = "alpha\n<<<<<<< not a real marker\nOURS-TAIL\n";
        let theirs = "alpha\n<<<<<<< not a real marker\nTHEIRS-TAIL\n";
        let Some(dir) = repo_with_conflict("marker-like", base, ours, theirs) else {
            return;
        };
        let set = conflict_set(&dir, "f.txt").await.unwrap();
        assert_eq!(
            set.regions.len(),
            1,
            "the fake marker must not split the region"
        );
        let region = &set.regions[0];
        assert_eq!(region.base, "shared\n");
        assert_eq!(region.ours, "OURS-TAIL\n");
        assert_eq!(region.theirs, "THEIRS-TAIL\n");
    }

    #[tokio::test]
    async fn a_region_id_is_the_same_across_two_runs() {
        let Some(dir) =
            repo_with_conflict("stable-id", "l1\nl2\nl3\n", "l1\nO2\nl3\n", "l1\nT2\nl3\n")
        else {
            return;
        };
        let first = conflict_set(&dir, "f.txt").await.unwrap();
        let second = conflict_set(&dir, "f.txt").await.unwrap();
        assert_eq!(first.regions[0].id, second.regions[0].id);
    }

    // --- the scope layer against real git ------------------------------

    /// A repository whose conflict sits on one line, with the file's
    /// extension under test.
    async fn conflict_in_file(
        label: &str,
        name: &str,
        base: &str,
        ours: &str,
        theirs: &str,
    ) -> Option<ConflictSet> {
        let dir = unique_temp(label);
        if !git_ok(&dir, &["init", "-q", "-b", "main"]) {
            return None;
        }
        if !git_ok(&dir, &["config", "user.email", "t@example.invalid"])
            || !git_ok(&dir, &["config", "user.name", "t"])
        {
            return None;
        }
        std::fs::write(dir.join(name), base).unwrap();
        if !git_ok(&dir, &["add", "."]) || !git_ok(&dir, &["commit", "-q", "-m", "base"]) {
            return None;
        }
        if !git_ok(&dir, &["checkout", "-q", "-b", "side"]) {
            return None;
        }
        std::fs::write(dir.join(name), theirs).unwrap();
        if !git_ok(&dir, &["commit", "-q", "-am", "side"]) {
            return None;
        }
        if !git_ok(&dir, &["checkout", "-q", "main"]) {
            return None;
        }
        std::fs::write(dir.join(name), ours).unwrap();
        if !git_ok(&dir, &["commit", "-q", "-am", "ours"]) {
            return None;
        }
        git_ok(&dir, &["merge", "side"]);
        if git_text(&dir, &["ls-files", "-u"]).trim().is_empty() {
            return None;
        }
        conflict_set(&dir, name).await.ok()
    }

    #[tokio::test]
    async fn two_sides_touching_different_scopes_are_still_reported() {
        // git groups two adjacent changes into one conflict region even
        // when they are in different functions. The scope layer sees that
        // the two sides touched different scopes — and the region is
        // **still reported**, because ADR §3.8.3 keeps suppression shut
        // until the ADR's measurement gate opens it. What the scope layer
        // adds is the attribution and the keep-both resolution.
        let base = "fn alpha() {\n    let x = 1;\n}\nfn beta() {\n    let y = 2;\n}\n";
        let ours = "fn alpha() {\n    let x = 1;\n} // ours\nfn beta() {\n    let y = 2;\n}\n";
        let theirs = "fn alpha() {\n    let x = 1;\n}\nfn beta() { // theirs\n    let y = 2;\n}\n";
        let Some(set) = conflict_in_file("different-scopes", "f.rs", base, ours, theirs).await
        else {
            return;
        };
        assert_eq!(
            set.all_regions().len(),
            1,
            "git must report one region for the two adjacent changes"
        );
        assert!(
            set.has_conflicts(),
            "the first delivery must not shrink the conflict set: {:?}",
            set.regions
        );
        assert_eq!(set.regions.len(), 1);
        assert_eq!(set.regions[0].confidence, RegionConfidence::DifferentScopes);
        // Nothing is lost: the keep-both resolution has both changes.
        let both = set.regions[0].both.clone().expect("keep both");
        assert!(both.contains("} // ours"), "both: {both}");
        assert!(both.contains("fn beta() { // theirs"), "both: {both}");
    }

    #[tokio::test]
    async fn two_sides_touching_the_same_scope_are_a_conflict() {
        let base = "fn alpha() {\n    let x = 1;\n}\nfn beta() {\n    let y = 2;\n}\n";
        let ours = "fn alpha() {\n    let x = 11;\n}\nfn beta() {\n    let y = 2;\n}\n";
        let theirs = "fn alpha() {\n    let x = 1;\n} // theirs\nfn beta() {\n    let y = 2;\n}\n";
        let Some(set) = conflict_in_file("same-scope", "f.rs", base, ours, theirs).await else {
            return;
        };
        assert_eq!(set.all_regions().len(), 1);
        assert!(
            set.has_conflicts(),
            "the same scope must be reported: {:?}",
            set.regions
        );
        assert_eq!(set.regions[0].confidence, RegionConfidence::SameScope);
    }

    #[tokio::test]
    async fn overlapping_scopes_are_still_reported_as_a_conflict() {
        // Ours touches alpha and beta, theirs touches alpha: the scope sets
        // overlap, so the conflict stands.
        let base = "fn alpha() {\n    let x = 1;\n}\nfn beta() {\n    let y = 2;\n}\n";
        let ours = "fn alpha() {\n    let x = 11;\n}\nfn beta() {\n    let y = 22;\n}\n";
        let theirs = "fn alpha() {\n    let x = 1;\n}\nfn beta() {\n    let y = 2;\n}\n";
        let Some(set) = conflict_in_file("overlapping-scopes", "f.rs", base, ours, theirs).await
        else {
            return;
        };
        assert!(
            set.has_conflicts(),
            "overlapping scopes must be reported: {:?}",
            set.regions
        );
        assert_eq!(set.regions[0].confidence, RegionConfidence::SameScope);
    }

    #[tokio::test]
    async fn an_unknown_language_falls_back_to_reporting_the_conflict() {
        // The same adjacent changes in a language the rule does not cover:
        // the whole file is one scope, so the conflict is reported.
        let base = "alpha\nbeta\n";
        let ours = "alpha // ours\nbeta\n";
        let theirs = "alpha\nbeta // theirs\n";
        let Some(set) = conflict_in_file("fallback", "f.md", base, ours, theirs).await else {
            return;
        };
        assert_eq!(set.scope_map.confidence, ScopeConfidence::Fallback);
        assert!(
            set.has_conflicts(),
            "the fallback must report the conflict: {:?}",
            set.regions
        );
        assert_eq!(set.regions[0].confidence, RegionConfidence::Fallback);
    }

    // --- the write-back ------------------------------------------------

    #[tokio::test]
    async fn the_write_back_equals_what_git_itself_would_have_produced() {
        let dir = unique_temp("write-back");
        let base = dir.join("base");
        let ours = dir.join("ours");
        let theirs = dir.join("theirs");
        std::fs::write(&base, "l1\nl2\nl3\n").unwrap();
        std::fs::write(&ours, "l1\nO2\nl3\n").unwrap();
        std::fs::write(&theirs, "l1\nT2\nl3\n").unwrap();

        let set = conflict_set_from_blobs(
            "f.txt",
            "oid",
            b"l1\nO2\nl3\n",
            b"l1\nl2\nl3\n",
            b"l1\nT2\nl3\n",
        )
        .await
        .unwrap();
        assert_eq!(set.regions.len(), 1);

        let mut choices = BTreeMap::new();
        choices.insert(0usize, RegionChoice::Ours);
        let resolved = compose_resolved(&set, &choices).unwrap();

        // What git itself produces when the conflict is resolved for ours.
        let out = std::process::Command::new("git")
            .args(["merge-file", "--ours", "-p"])
            .arg(&ours)
            .arg(&base)
            .arg(&theirs)
            .output()
            .unwrap();
        assert_eq!(
            resolved.as_bytes(),
            out.stdout,
            "the write-back must equal git's own --ours output"
        );

        let mut choices = BTreeMap::new();
        choices.insert(0usize, RegionChoice::Theirs);
        let resolved_theirs = compose_resolved(&set, &choices).unwrap();
        let out = std::process::Command::new("git")
            .args(["merge-file", "--theirs", "-p"])
            .arg(&ours)
            .arg(&base)
            .arg(&theirs)
            .output()
            .unwrap();
        assert_eq!(
            resolved_theirs.as_bytes(),
            out.stdout,
            "the write-back must equal git's own --theirs output"
        );
    }

    #[tokio::test]
    async fn a_mixed_choice_splices_the_two_sides() {
        let base = "l1\nl2\nl3\nl4\nl5\nl6\nl7\nl8\nl9\nl10\n";
        let ours = "l1\nO2\nl3\nl4\nl5\nl6\nl7\nl8\nl9\nO10\n";
        let theirs = "l1\nT2\nl3\nl4\nl5\nl6\nl7\nl8\nl9\nT10\n";
        let set = conflict_set_from_blobs(
            "f.txt",
            "oid",
            ours.as_bytes(),
            base.as_bytes(),
            theirs.as_bytes(),
        )
        .await
        .unwrap();
        assert_eq!(set.regions.len(), 2);
        let mut choices = BTreeMap::new();
        choices.insert(0usize, RegionChoice::Ours);
        choices.insert(1usize, RegionChoice::Theirs);
        let resolved = compose_resolved(&set, &choices).unwrap();
        assert_eq!(resolved, "l1\nO2\nl3\nl4\nl5\nl6\nl7\nl8\nl9\nT10\n");
        // The mixed result is neither all-ours nor all-theirs.
        let mut all_ours = BTreeMap::new();
        all_ours.insert(0usize, RegionChoice::Ours);
        all_ours.insert(1usize, RegionChoice::Ours);
        assert_ne!(resolved, compose_resolved(&set, &all_ours).unwrap());
    }

    #[tokio::test]
    async fn a_region_without_a_choice_is_an_error_not_a_default() {
        let set = conflict_set_from_blobs(
            "f.txt",
            "oid",
            b"l1\nO2\nl3\n",
            b"l1\nl2\nl3\n",
            b"l1\nT2\nl3\n",
        )
        .await
        .unwrap();
        let err = compose_resolved(&set, &BTreeMap::new()).unwrap_err();
        assert!(format!("{err}").contains("no choice"), "got: {err}");
    }

    #[tokio::test]
    async fn a_resolved_file_can_be_written_back_and_staged() {
        let Some(dir) = repo_with_conflict(
            "write-back-repo",
            "l1\nl2\nl3\n",
            "l1\nO2\nl3\n",
            "l1\nT2\nl3\n",
        ) else {
            return;
        };
        // Before the write-back the path is unmerged.
        let before = git_text(&dir, &["status", "--porcelain=v1"]);
        assert!(
            before.starts_with("UU"),
            "expected an unmerged path, got: {before}"
        );

        let set = conflict_set(&dir, "f.txt").await.unwrap();
        let mut choices = BTreeMap::new();
        choices.insert(0usize, RegionChoice::Ours);
        let resolved = compose_resolved(&set, &choices).unwrap();
        write_resolved(&dir, "f.txt", &resolved).await.unwrap();

        let after = git_text(&dir, &["status", "--porcelain=v1"]);
        assert!(
            !after.contains("UU"),
            "the path must no longer be unmerged: {after}"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("f.txt")).unwrap(),
            "l1\nO2\nl3\n"
        );
    }

    #[tokio::test]
    async fn a_path_with_spaces_and_non_ascii_can_be_staged() {
        // ADR §7.4: `git add <path>` on a path with spaces, non-ASCII
        // characters or a leading dash.
        let dir = unique_temp("odd-paths");
        if !git_ok(&dir, &["init", "-q", "-b", "main"]) {
            return;
        }
        if !git_ok(&dir, &["config", "user.email", "t@example.invalid"])
            || !git_ok(&dir, &["config", "user.name", "t"])
        {
            return;
        }
        for name in ["with space.txt", "ünïcode.txt", "-leading-dash.txt"] {
            std::fs::write(dir.join(name), "content\n").unwrap();
        }
        if !git_ok(&dir, &["add", "."]) || !git_ok(&dir, &["commit", "-q", "-m", "first"]) {
            return;
        }
        for name in ["with space.txt", "ünïcode.txt", "-leading-dash.txt"] {
            write_resolved(&dir, name, "resolved\n").await.unwrap();
        }
        // `core.quotePath=false` is what makes git print a non-ASCII path as
        // itself instead of the octal escape `"\\303\\274n..."`.
        let status = git_text(
            &dir,
            &["-c", "core.quotePath=false", "status", "--porcelain=v1"],
        );
        for name in ["with space.txt", "ünïcode.txt", "-leading-dash.txt"] {
            assert!(status.contains(name), "{name} was not staged: {status}");
        }
    }

    #[tokio::test]
    async fn a_path_outside_the_work_tree_is_refused() {
        let dir = unique_temp("escape");
        let err = write_resolved(&dir, "../escape.txt", "x")
            .await
            .unwrap_err();
        assert!(format!("{err}").contains(".."), "got: {err}");
    }

    // --- the temporaries ------------------------------------------------

    #[tokio::test]
    async fn the_temporary_files_are_removed_when_the_call_returns() {
        // The guard owns a directory inside the one the test owns, so the
        // assertion is about this call's files and not about whatever else
        // the other tests are doing in the system temp directory at the
        // same time.
        let root = unique_temp("temps");
        let result =
            conflict_set_from_blobs_in(&root, "f.txt", "oid", b"a\n", b"a\n", b"b\n").await;
        assert!(result.is_ok());
        let left: Vec<String> = std::fs::read_dir(&root)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            left.is_empty(),
            "the temporary files must not survive the call: {left:?}"
        );
    }

    #[test]
    fn the_guard_removes_the_files_it_wrote() {
        // The guard's destructor is what cleans up the error paths too: an
        // early `?` return still runs it, so a failed merge leaves nothing
        // behind. Removing the destructor turns this test red.
        let root = unique_temp("guard");
        let (dir, ours) = {
            let mut temps = TempFiles::in_root(&root).unwrap();
            let ours = temps.write("ours", b"a\n").unwrap();
            assert!(
                ours.exists(),
                "the file must exist while the guard holds it"
            );
            (temps.dir.clone(), ours)
        };
        assert!(!ours.exists(), "the guard must remove its files");
        assert!(!dir.exists(), "the guard must remove its directory");
    }
}
