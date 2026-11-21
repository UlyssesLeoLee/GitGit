//! Conflict-region AI arbitration — ADR-0024, Lane B.
//!
//! # What this module is
//!
//! ADR-0024 (`docs/adr/0024-hunk-conflict-arbitration.md`) splits the
//! conflict-arbitration work into three lanes. This module is the Rust half
//! of **Lane B**: the region that goes to the model, and the rules that
//! decide whether the model's answer is allowed to become a suggestion.
//!
//! # What it does not do
//!
//! * It does not parse conflict regions. Lane A owns that
//!   ([`crate::repo::conflict`]). The id used here is *Lane A's* hash —
//!   [`crate::repo::conflict::region_id`] — so the id the model is asked to
//!   echo is the same id the rest of the product shows the user.
//! * It does not write anything. A suggestion is a suggestion; the ADR's
//!   write-back path ([`crate::repo::conflict::write_resolved`]) runs only
//!   after the user has accepted one.
//!
//! # The five checks (ADR §3.4.2)
//!
//! | # | check | refusal |
//! |---|---|---|
//! | 1 | the answer is not empty | "the suggestion is empty" |
//! | 2 | the answer carries no conflict-marker family | "the suggestion carries a conflict marker" |
//! | 3 | the answer is bounded | "the suggestion is too long" |
//! | 4 | the answer does not carry the base section back | "the suggestion carries the base section" |
//! | 5 | the answer is bound to the region it was asked about | "the answer was bound to another region" |
//!
//! `[INFERENCE]` Check 2 is the load-bearing one. A model asked to resolve a
//! conflict will sometimes "help" by reprinting the whole conflicted block,
//! markers included — that is not a resolution, it is the problem moved
//! somewhere else.
//!
//! `[INFERENCE]` Check 5 needs the answer to carry the region id, so the
//! answer format is part of the contract: the model's first line is the
//! region id, copied verbatim, and the replacement text follows it. The
//! prompt says so, and [`validate_answer`] refuses an answer that does not
//! comply rather than guessing which region it was meant for.

use crate::repo::conflict::region_id;

/// The most context lines sent on either side of the region.
///
/// `[PROPOSAL]` ADR §3.3: "the context is up to 20 lines before and after the
/// conflict region in the base file, and no other region". Twenty is a
/// conservative value: enough to see whether the region sits in a function
/// signature, an import list or a string literal, without putting the whole
/// file in the prompt.
pub const MAX_CONTEXT_LINES: usize = 20;

/// The most characters a suggestion may contain.
///
/// `[PROPOSAL]` ADR §3.4.2's third check: "bounded in length (at most the
/// region's ours side lines times 10, or at most 20,000 characters, whichever
/// is smaller)".
pub const MAX_SUGGESTION_CHARS: usize = 20_000;

/// How many suggestion lines one line of the region's ours side buys.
///
/// `[PROPOSAL]` ADR §3.4.2's third check, the line bound.
pub const SUGGESTION_LINES_PER_OURS_LINE: usize = 10;

/// The conflict-marker families ADR §3.4.2's second check rejects.
///
/// `[FACT]` `git merge-file --diff3 --marker-size=24` writes markers that are
/// 24 characters wide, but the seven-character forms are what a model
/// reproduces when it "helpfully" pastes a conflicted block (ADR §2.1).
/// Matching the seven-character prefix also matches the 24-character form,
/// because the latter starts with the former.
const CONFLICT_MARKERS: [&str; 4] = ["<<<<<<<", "|||||||", "=======", ">>>>>>>"];

/// The AI-facing view of one conflict region.
///
/// `[FACT]` The fields are the ones ADR §3.3 lists as "what is sent": the
/// repository-relative path, the file's language, the three sides, and the
/// context around the region.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictRegion {
    /// The repository-relative path of the conflicted file.
    pub path: String,
    /// The base blob's object id, when the base was available.
    ///
    /// `[FACT]` It is part of the region id (ADR §3.7.2), so it is not
    /// optional there. An empty string means "unknown base", which still
    /// produces a stable id — just not the same one Lane A computed for a
    /// path whose base *was* known.
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
}

impl ConflictRegion {
    /// The region's stable id (ADR §3.7.2).
    ///
    /// `[FACT]` This is Lane A's [`crate::repo::conflict::region_id`], called
    /// with the same inputs, so the id the model is asked to echo is the id
    /// the product shows the user. Computing it here rather than trusting a
    /// caller is what makes the id unforgeable: the id is a function of the
    /// region's content, not of anything the caller says about it.
    pub fn id(&self) -> String {
        region_id(
            &self.path,
            &self.base_oid,
            self.index,
            &self.ours,
            &self.base,
            &self.theirs,
        )
    }

    /// The file's language, derived from its extension.
    ///
    /// `[PROPOSAL]` ADR §3.3 sends the file's language ("rust" /
    /// "typescript") alongside the region. An unknown extension is reported
    /// as "unknown" rather than guessed: a wrong language hint is worse than
    /// none, because the model will style the suggestion after it.
    pub fn language(&self) -> &'static str {
        let ext = self
            .path
            .rsplit_once('.')
            .map(|(_, ext)| ext.to_ascii_lowercase())
            .unwrap_or_default();
        match ext.as_str() {
            "rs" => "rust",
            "ts" | "mts" | "cts" => "typescript",
            "tsx" => "typescript",
            "js" | "mjs" | "cjs" => "javascript",
            "svelte" => "svelte",
            "py" => "python",
            "go" => "go",
            "java" => "java",
            "rb" => "ruby",
            "cs" => "csharp",
            "php" => "php",
            "c" | "h" => "c",
            "cc" | "cpp" | "cxx" | "hpp" => "cpp",
            "sh" | "bash" => "shell",
            "md" => "markdown",
            "json" => "json",
            "yml" | "yaml" => "yaml",
            "toml" => "toml",
            _ => "unknown",
        }
    }

    /// The number of lines the ours side carries.
    ///
    /// `[INFERENCE]` The floor is one line. A region whose ours side is empty
    /// is a region the other side deleted, and a zero-line budget would
    /// refuse every possible replacement for it — including "leave it
    /// deleted", which is a legitimate answer.
    fn ours_lines(&self) -> usize {
        self.ours.lines().count().max(1)
    }

    /// True when the two sides carry the same text.
    ///
    /// `[PROPOSAL]` ADR §3.7.4: "the region's ours and theirs text are the
    /// same — the region exists but the two sides are equivalent; the default
    /// suggestion is keep ours". The comparison ignores surrounding
    /// whitespace: two sides that differ only in a trailing newline are the
    /// same text for a human reader.
    pub fn sides_are_equivalent(&self) -> bool {
        self.ours.trim() == self.theirs.trim()
    }

    /// The default choice for this region (ADR §3.7.4).
    pub fn default_choice(&self) -> DefaultChoice {
        if self.sides_are_equivalent() {
            DefaultChoice::KeepOurs
        } else {
            DefaultChoice::AskModel
        }
    }

    /// True when `text` carries the region's base section back.
    ///
    /// `[INFERENCE]` ADR §3.4.2's fourth check ("the suggestion does not
    /// contain the base side") is implemented as "the suggestion contains the
    /// base section's text". The base section is the text both sides
    /// replaced; a suggestion that pastes it back has not resolved anything.
    ///
    /// The check stands down when the ours side already carries the base
    /// text, because then "keep ours" and "paste the base back" are the same
    /// string and refusing it would refuse the most common suggestion there
    /// is.
    ///
    /// `[INFERENCE]` ADR §3.4.2's row 4 is worded "does not contain the
    /// base-side marker line". A base marker line is `|||||||`, which is
    /// already a member of the marker family that row 2 refuses, and the
    /// checks run in the ADR's order — so a suggestion carrying one is
    /// refused as [`SuggestionRejection::ConflictMarker`], and this check
    /// catches the wider case the row's own reason names ("the suggestion
    /// mixed in the base section").
    fn carries_base_section(&self, text: &str) -> bool {
        let base = self.base.trim();
        if base.is_empty() {
            return false;
        }
        if self.ours.trim().contains(base) {
            return false;
        }
        text.contains(base)
    }

    /// The replacement text for a choice that does not need the model.
    ///
    /// `[PROPOSAL]` ADR §3.7.1: "the conflict dialog stays fully usable
    /// without a key: keep ours / keep theirs / hand-edit all work as
    /// usual". This is that guarantee on the Rust side — the three options
    /// are a function of the region alone, so a missing credential cannot
    /// take them away.
    ///
    /// `[INFERENCE]` "Keep ours" and "keep theirs" are *not* put through
    /// the suggestion checks: the user picked a side, and a "keep theirs"
    /// that happens to equal the base text is a legitimate resolution
    /// rather than a model pasting the base back. A hand-written edit goes
    /// through the same checks as an AI suggestion (ADR §3.7.4).
    pub fn resolve(
        &self,
        choice: DialogChoice,
        manual: Option<&str>,
    ) -> Result<String, SuggestionRejection> {
        match choice {
            DialogChoice::KeepOurs => Ok(self.ours.clone()),
            DialogChoice::KeepTheirs => Ok(self.theirs.clone()),
            DialogChoice::Manual => validate_replacement(self, manual.unwrap_or("")),
        }
    }
}

/// The options the conflict dialog always offers (ADR §3.4.3).
///
/// `[PROPOSAL]` ADR §3.4.3: "the dialog always offers three options that do
/// not depend on the AI: keep ours / keep theirs / hand-edit. The AI
/// suggestion is a **fourth** option, and it is always last."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogChoice {
    /// Keep the ours side's text.
    KeepOurs,
    /// Keep the theirs side's text.
    KeepTheirs,
    /// The user's own text, which is validated like an AI suggestion.
    Manual,
}

impl DialogChoice {
    /// The three choices that work with no key configured.
    ///
    /// `[PROPOSAL]` ADR §3.7.1: the dialog does not degrade when there is
    /// no credential.
    pub const NO_AI: [DialogChoice; 3] = [
        DialogChoice::KeepOurs,
        DialogChoice::KeepTheirs,
        DialogChoice::Manual,
    ];
}

/// One region to arbitrate, with the id that binds its answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArbitrationUnit {
    /// The region's ordinal within its file.
    pub index: usize,
    /// The region's stable id (ADR §3.7.2).
    pub region_id: String,
    /// The region itself.
    pub region: ConflictRegion,
}

/// Split a file's regions into one arbitration unit per region.
///
/// `[PROPOSAL]` ADR §3.7.2: "a request carries exactly one region. Several
/// regions are never merged into one request (otherwise the model would
/// 'harmonize the style' across regions, which is exactly overreach)". This
/// function is the Rust side of that rule: the caller arbitrates one unit
/// at a time, and each unit carries the id its answer has to echo.
pub fn arbitration_units(regions: Vec<ConflictRegion>) -> Vec<ArbitrationUnit> {
    regions
        .into_iter()
        .map(|region| {
            let index = region.index;
            let region_id = region.id();
            ArbitrationUnit {
                index,
                region_id,
                region,
            }
        })
        .collect()
}

/// What the conflict dialog should offer first for a region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultChoice {
    /// The two sides carry the same text: the region exists but the sides are
    /// equivalent, so "keep ours" is the default and the model is not asked
    /// (ADR §3.7.4).
    KeepOurs,
    /// The two sides differ: the arbitration is the model's to suggest.
    AskModel,
}

/// Why a suggestion was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestionRejection {
    /// The answer carried no text.
    Empty,
    /// The answer carried a conflict marker.
    ConflictMarker,
    /// The answer was longer than the bound allows.
    TooLong,
    /// The answer carried the base section back.
    ContainsBaseSection,
    /// The answer did not carry the region id at all.
    MissingRegionId,
    /// The answer carried a different region's id.
    RegionMismatch,
}

impl SuggestionRejection {
    /// The reason shown to the user.
    ///
    /// `[PROPOSAL]` ADR §3.4.2 names a reason per check ("empty suggestion",
    /// "the suggestion carries a conflict marker", "the suggestion is too
    /// long", "the suggestion mixed in the base section"). The reasons are
    /// user-facing text, not error codes: the dialog shows them next to the
    /// region so the user knows why the fourth option is empty.
    pub fn reason(self) -> &'static str {
        match self {
            SuggestionRejection::Empty => "the suggestion is empty",
            SuggestionRejection::ConflictMarker => "the suggestion carries a conflict marker",
            SuggestionRejection::TooLong => "the suggestion is too long",
            SuggestionRejection::ContainsBaseSection => {
                "the suggestion carries the base section back"
            }
            SuggestionRejection::MissingRegionId => {
                "the answer did not carry the region id, so it cannot be bound to this region"
            }
            SuggestionRejection::RegionMismatch => "the answer was bound to another region",
        }
    }
}

impl std::fmt::Display for SuggestionRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.reason())
    }
}

/// The first conflict-marker family found in `text`, if any.
///
/// `[INFERENCE]` The test is a plain substring test rather than a
/// "line that starts with a marker" test. A marker buried in the middle of a
/// line is just as much a sign that the model pasted a conflicted block, and
/// the substring test is the conservative direction: it can refuse a
/// suggestion that merely mentions the marker characters, which is a nuisance,
/// and it cannot accept one that carries them.
fn conflict_marker(text: &str) -> Option<&'static str> {
    CONFLICT_MARKERS
        .iter()
        .find(|marker| text.contains(**marker))
        .copied()
}

/// True when `text` has the shape of a region id.
///
/// `[FACT]` A region id is the first 16 bytes of a SHA-256 digest, hex
/// encoded (ADR §3.7.2), so it is 32 lower-case hexadecimal characters.
fn looks_like_region_id(text: &str) -> bool {
    text.len() == 32
        && text
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

/// Split the answer into the claimed region id and the suggestion text.
///
/// The first line is the claimed id when it has the shape of one; otherwise
/// the whole answer is the suggestion text and no id was claimed.
///
/// `[INFERENCE]` The suggestion text is returned **verbatim**. It is what
/// replaces the region in the merged file, so a trailing newline that was
/// part of the region has to survive the round trip — trimming it would
/// join the region's last line to whatever follows it. Only the id line is
/// trimmed, and only for the comparison.
fn split_region_id(answer: &str) -> (Option<String>, &str) {
    let (first, rest) = answer.split_once('\n').unwrap_or((answer, ""));
    let first = first.trim();
    if looks_like_region_id(first) {
        (Some(first.to_string()), rest)
    } else {
        (None, answer)
    }
}

/// Validate a replacement text — the check set ADR §3.7.4 applies to a
/// hand-written edit as well ("the same validation as an AI suggestion").
///
/// The shared checks are ADR §3.4.2's second, third and fourth: no conflict
/// marker, a bounded length, and no base section carried back.
///
/// `[INFERENCE]` The empty check is *not* part of this set: an empty
/// replacement is a legitimate hand-written resolution (delete the region),
/// while an empty *model* answer is useless. The emptiness check therefore
/// lives in [`validate_answer`], which is the AI-only path.
pub fn validate_replacement(
    region: &ConflictRegion,
    text: &str,
) -> Result<String, SuggestionRejection> {
    if conflict_marker(text).is_some() {
        return Err(SuggestionRejection::ConflictMarker);
    }
    let max_lines = region
        .ours_lines()
        .saturating_mul(SUGGESTION_LINES_PER_OURS_LINE);
    if text.lines().count() > max_lines || text.chars().count() > MAX_SUGGESTION_CHARS {
        return Err(SuggestionRejection::TooLong);
    }
    if region.carries_base_section(text) {
        return Err(SuggestionRejection::ContainsBaseSection);
    }
    Ok(text.to_string())
}

/// Validate a model answer against the region it was asked about.
///
/// This is ADR §3.4.2's five checks, in the ADR's order:
///
/// 1. the answer is not empty;
/// 2. the answer carries no conflict-marker family;
/// 3. the answer is bounded;
/// 4. the answer does not carry the base section back;
/// 5. the answer is bound to the region's id.
///
/// Returns the accepted replacement text, or the reason the answer was
/// refused. A refusal is never partial: the caller gets either a suggestion
/// it may show or a reason it may display — never text it may apply.
pub fn validate_answer(
    region: &ConflictRegion,
    answer: &str,
) -> Result<String, SuggestionRejection> {
    let raw = answer.trim();
    if raw.is_empty() {
        return Err(SuggestionRejection::Empty);
    }
    // The marker check runs over the whole answer, before the id is split
    // off, so an answer that carries a marker is refused for that reason
    // rather than for a missing id.
    if conflict_marker(raw).is_some() {
        return Err(SuggestionRejection::ConflictMarker);
    }
    // The split runs over the *untrimmed* answer: the suggestion text is
    // what replaces the region in the merged file, so its trailing newline
    // is part of the text and must survive. Only the id line is trimmed.
    let (claimed_id, body) = split_region_id(answer);
    if body.trim().is_empty() {
        return Err(SuggestionRejection::Empty);
    }
    let suggestion = validate_replacement(region, body)?;
    match claimed_id {
        None => Err(SuggestionRejection::MissingRegionId),
        Some(id) if id != region.id() => Err(SuggestionRejection::RegionMismatch),
        Some(_) => Ok(suggestion),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// A region with three distinct sides.
    fn region() -> ConflictRegion {
        ConflictRegion {
            path: String::from("src/foo.rs"),
            base_oid: String::from("0123456789abcdef0123456789abcdef01234567"),
            index: 0,
            ours: String::from("let a = 1;\n"),
            base: String::from("let a = 0;\n"),
            theirs: String::from("let a = 2;\n"),
            context_before: String::from("fn main() {\n"),
            context_after: String::from("}\n"),
        }
    }

    /// The answer format the prompt asks for: the id on the first line, the
    /// replacement text after it.
    fn answer(region: &ConflictRegion, body: &str) -> String {
        format!("{}\n{}", region.id(), body)
    }

    #[test]
    fn the_region_id_is_the_lane_a_hash() {
        let r = region();
        let expected = crate::repo::conflict::region_id(
            &r.path,
            &r.base_oid,
            r.index,
            &r.ours,
            &r.base,
            &r.theirs,
        );
        assert_eq!(r.id(), expected, "the id must be Lane A's hash");
        assert_eq!(r.id().len(), 32, "16 bytes of hex");
        assert!(
            r.id()
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "the id must be lower-case hex: {}",
            r.id()
        );
    }

    #[test]
    fn the_region_id_changes_when_the_region_changes() {
        let mut r = region();
        let first = r.id();
        r.theirs = String::from("let a = 3;\n");
        assert_ne!(r.id(), first, "a changed region must have a changed id");
    }

    #[test]
    fn an_answer_with_the_right_id_is_accepted() {
        let r = region();
        let out = validate_answer(&r, &answer(&r, "let a = 1;\n")).unwrap();
        assert_eq!(out, "let a = 1;\n");
    }

    #[test]
    fn an_empty_answer_is_refused() {
        let r = region();
        let err = validate_answer(&r, "   \n\t ").unwrap_err();
        assert_eq!(err, SuggestionRejection::Empty);
    }

    #[test]
    fn an_answer_carrying_a_conflict_marker_is_refused() {
        // The case ADR §3.4.2 calls out: the model "helpfully" pastes the
        // whole conflicted block back.
        let r = region();
        let body = "<<<<<<< OURS\nlet a = 1;\n=======\nlet a = 0;\n>>>>>>> THEIRS\n";
        let err = validate_answer(&r, &answer(&r, body)).unwrap_err();
        assert_eq!(err, SuggestionRejection::ConflictMarker);
    }

    #[test]
    fn an_answer_carrying_a_base_marker_is_refused() {
        let r = region();
        let body = "||||||| BASE\nlet a = 0;\n=======\n";
        let err = validate_answer(&r, &answer(&r, body)).unwrap_err();
        assert_eq!(err, SuggestionRejection::ConflictMarker);
    }

    #[test]
    fn an_over_long_answer_is_refused() {
        let r = region();
        // The ours side is one line, so the line budget is ten.
        let body = "x\n".repeat(11);
        let err = validate_answer(&r, &answer(&r, &body)).unwrap_err();
        assert_eq!(err, SuggestionRejection::TooLong);
    }

    #[test]
    fn an_answer_within_the_line_budget_is_accepted() {
        let r = region();
        let body = "x\n".repeat(10);
        let out = validate_answer(&r, &answer(&r, &body)).unwrap();
        assert_eq!(out, body);
    }

    #[test]
    fn an_answer_that_pastes_the_base_section_back_is_refused() {
        let r = region();
        let body = "let a = 0;\n";
        let err = validate_answer(&r, &answer(&r, body)).unwrap_err();
        assert_eq!(err, SuggestionRejection::ContainsBaseSection);
    }

    #[test]
    fn keeping_ours_is_not_refused_as_pasting_the_base_back() {
        // The regression this guards: when the ours side already contains the
        // base text, "keep ours" and "paste the base back" are the same
        // string, and a check that cannot tell them apart refuses the most
        // common suggestion there is.
        let mut r = region();
        r.ours = String::from("let a = 0;\nlet b = 1;\n");
        let out = validate_answer(&r, &answer(&r, "let a = 0;\nlet b = 1;\n")).unwrap();
        assert_eq!(out, "let a = 0;\nlet b = 1;\n");
    }

    #[test]
    fn an_answer_bound_to_another_region_is_discarded() {
        // ADR §3.7.2: the answer must carry the id of the region it was
        // shown for. A different id means the answer is about something else.
        let r = region();
        let mut other = region();
        other.index = 1;
        let body = "let a = 1;\n";
        let err = validate_answer(&r, &answer(&other, body)).unwrap_err();
        assert_eq!(err, SuggestionRejection::RegionMismatch);
    }

    #[test]
    fn an_answer_without_a_region_id_is_refused() {
        let r = region();
        let err = validate_answer(&r, "let a = 1;\n").unwrap_err();
        assert_eq!(err, SuggestionRejection::MissingRegionId);
    }

    #[test]
    fn a_hand_written_edit_goes_through_the_same_checks() {
        // ADR §3.7.4: a manual edit is validated exactly like a suggestion.
        let r = region();
        assert!(validate_replacement(&r, "let a = 5;\n").is_ok());
        assert_eq!(
            validate_replacement(&r, "<<<<<<< OURS\n").unwrap_err(),
            SuggestionRejection::ConflictMarker
        );
        assert_eq!(
            validate_replacement(&r, &"x\n".repeat(11)).unwrap_err(),
            SuggestionRejection::TooLong
        );
        assert_eq!(
            validate_replacement(&r, "let a = 0;\n").unwrap_err(),
            SuggestionRejection::ContainsBaseSection
        );
    }

    #[test]
    fn a_hand_written_edit_may_be_empty() {
        // Deleting the region is a resolution, so the empty check belongs to
        // the AI path only.
        let r = region();
        assert_eq!(validate_replacement(&r, "").unwrap(), "");
    }

    #[test]
    fn equivalent_sides_default_to_keep_ours() {
        // ADR §3.7.4: the region exists but the two sides are equivalent.
        let mut r = region();
        r.theirs = r.ours.clone();
        assert!(r.sides_are_equivalent());
        assert_eq!(r.default_choice(), DefaultChoice::KeepOurs);
    }

    #[test]
    fn different_sides_default_to_asking_the_model() {
        let r = region();
        assert!(!r.sides_are_equivalent());
        assert_eq!(r.default_choice(), DefaultChoice::AskModel);
    }

    #[test]
    fn the_language_comes_from_the_extension() {
        let mut r = region();
        assert_eq!(r.language(), "rust");
        r.path = String::from("web/app.ts");
        assert_eq!(r.language(), "typescript");
        r.path = String::from("README");
        assert_eq!(r.language(), "unknown");
    }

    #[test]
    fn the_three_non_ai_choices_work_without_a_key() {
        // ADR §3.7.1: the dialog does not degrade when there is no
        // credential. None of these three options reads a key, so the
        // absence of one cannot take them away.
        let r = region();
        assert_eq!(DialogChoice::NO_AI.len(), 3);
        assert_eq!(
            r.resolve(DialogChoice::KeepOurs, None).unwrap(),
            "let a = 1;\n"
        );
        assert_eq!(
            r.resolve(DialogChoice::KeepTheirs, None).unwrap(),
            "let a = 2;\n"
        );
        assert_eq!(
            r.resolve(DialogChoice::Manual, Some("let a = 3;\n"))
                .unwrap(),
            "let a = 3;\n"
        );
    }

    #[test]
    fn a_hand_written_edit_is_validated_like_a_suggestion() {
        // ADR §3.7.4: "before saving, run the same validation as an AI
        // suggestion". The manual path goes through the shared checks.
        let r = region();
        assert_eq!(
            r.resolve(DialogChoice::Manual, Some("<<<<<<< OURS\n"))
                .unwrap_err(),
            SuggestionRejection::ConflictMarker
        );
        assert_eq!(
            r.resolve(DialogChoice::Manual, Some("let a = 0;\n"))
                .unwrap_err(),
            SuggestionRejection::ContainsBaseSection
        );
    }

    #[test]
    fn keeping_theirs_is_not_refused_when_theirs_equals_the_base() {
        // The user picking a side is not a model pasting the base back, so
        // the base-section check must not fire on it.
        let mut r = region();
        r.theirs = r.base.clone();
        assert_eq!(
            r.resolve(DialogChoice::KeepTheirs, None).unwrap(),
            "let a = 0;\n"
        );
    }

    #[test]
    fn several_regions_become_one_arbitration_unit_each() {
        // ADR §3.7.2: a request carries exactly one region, so a file with
        // several regions is arbitrated region by region.
        let mut second = region();
        second.index = 1;
        second.ours = String::from("let b = 1;\n");
        second.theirs = String::from("let b = 2;\n");
        let units = arbitration_units(vec![region(), second]);
        assert_eq!(units.len(), 2, "one unit per region");
        assert_eq!(units[0].index, 0);
        assert_eq!(units[1].index, 1);
    }

    #[test]
    fn each_arbitration_unit_carries_its_own_region_id() {
        // The id is what binds an answer back to its region, so a unit
        // whose id does not match its own region would be useless.
        let mut second = region();
        second.index = 1;
        let units = arbitration_units(vec![region(), second]);
        for unit in &units {
            assert_eq!(unit.region_id, unit.region.id());
        }
        assert_ne!(units[0].region_id, units[1].region_id);
    }
}
