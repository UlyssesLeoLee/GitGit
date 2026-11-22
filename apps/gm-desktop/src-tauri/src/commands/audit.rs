//! The conflict-arbitration audit trail — ADR-0024 §3.7.3.
//!
//! # What this module is
//!
//! ADR-0024 §3.7.3 asks for **one audit record per arbitration**: the trail
//! that says which region was arbitrated, which provider and model were
//! consulted, what was redacted before the request left the machine, what the
//! model answered, and what the human finally chose.
//!
//! # The layer split (the decision this lane makes)
//!
//! `[INFERENCE]` The **front end starts the record and the Rust side writes
//! it.** The choice is only known in the webview — the user picks a side, and
//! the dialog is what offers the sides — so the webview has to say something.
//! Everything else in the record is known only in the backend: the model the
//! provider *reported* (which is not the model that was requested), the
//! redaction count, the digest of the prompt that was actually sent, and the
//! answer itself. So the webview sends the decision, and the backend writes
//! the record.
//!
//! `[INFERENCE]` The backend therefore re-derives everything it can rather
//! than believing the webview: the **region id is recomputed** from the
//! region's own content (`ConflictRegion::id`), never taken from the caller.
//! A caller that claims a decision about a region it did not arbitrate gets
//! a record about the region it actually sent.
//!
//! # What the record deliberately does not carry
//!
//! `[FACT]` **The API key never enters the record, the log, or a flag**
//! (ADR-0024 §3.7.3, §3.9). Three things enforce that here:
//!
//! 1. the model's answer is scrubbed of the credential before it is stored
//!    (`sanitize::scrub_secret`) — a misconfigured proxy can echo the
//!    `Authorization` header back inside the answer, and the answer is the
//!    only untrusted text in the record;
//! 2. the written line is checked against the resolved credential before it
//!    is appended, and the write is **refused** if the credential is in it;
//! 3. the key is never a field: the record has no field for it.
//!
//! # The file
//!
//! `[PROPOSAL]` The trail is one **append-only JSON Lines** file under the
//! desktop app's app-data directory: `<app-data>/conflict-audit.jsonl`. One
//! JSON object per line, appended, never rewritten. It is deliberately not in
//! the repository (it is not the project's history) and not in the vault (it
//! is not a credential, and the vault is versioned and readable through a
//! different surface).
//!
//! `[INFERENCE]` The ADR's field list has no repository name, so the trail is
//! per **installation**: the path in a record is repository-relative, and the
//! file says which regions were arbitrated, not which repository they came
//! from. Naming the repository in the record is a change to the ADR's field
//! list, not to this implementation.
//!
//! # `applied_at`
//!
//! `[INFERENCE]` There is no merge driver (ADR-0024 §3.6): the application
//! does not drive the merge, and the user still runs `git commit` themselves.
//! So there is no moment at which "the resolution was applied" that this
//! module could observe. `applied_at` is the moment the **choice was made** —
//! the moment the user picked a side — and that is what the ADR says.

use std::collections::VecDeque;
use std::io::Write;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use gitgit::ai::conflict::ConflictRegion;
use gitgit::ai::provider::ChatMessage;
use gitgit::ai::sanitize;
use gitgit::server::vault_versioned::sha256_hex;
use serde::Serialize;
use tauri::State;

use crate::commands::ai::{resolve_ai_key, ConflictRegionDto, CONFLICT_MAX_ANSWER_CHARS};
use crate::error::{AppError, AppResult};
use crate::state::DesktopState;

/// The name of the audit file, under the desktop app's app-data directory.
pub const AUDIT_FILE_NAME: &str = "conflict-audit.jsonl";

/// How many arbitrations the backend remembers at once.
///
/// `[INFERENCE]` The facts are held so that a decision made minutes after the
/// answer can still be described in full (which model answered, what the
/// prompt was). The bound keeps a long-lived session from growing without
/// limit: the oldest arbitration is dropped first.
pub const MAX_REMEMBERED_ARBITRATIONS: usize = 64;

/// What the human chose (ADR-0024 §3.7.3's `choice` field).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArbitrationChoice {
    /// Keep the ours side's text.
    Ours,
    /// Keep the theirs side's text.
    Theirs,
    /// Apply the model's suggestion.
    Ai,
    /// Apply the user's own hand-written text.
    Manual,
}

impl ArbitrationChoice {
    /// The wire name, and the name the record carries.
    pub fn as_str(self) -> &'static str {
        match self {
            ArbitrationChoice::Ours => "ours",
            ArbitrationChoice::Theirs => "theirs",
            ArbitrationChoice::Ai => "ai",
            ArbitrationChoice::Manual => "manual",
        }
    }

    /// The choice named by `value`, or a refusal.
    ///
    /// `[INFERENCE]` An unknown name is refused rather than coerced into a
    /// default: a record that says "ours" because the caller sent nonsense
    /// is worse than no record, because it is a *wrong* record.
    pub fn parse(value: &str) -> AppResult<Self> {
        match value {
            "ours" => Ok(ArbitrationChoice::Ours),
            "theirs" => Ok(ArbitrationChoice::Theirs),
            "ai" => Ok(ArbitrationChoice::Ai),
            "manual" => Ok(ArbitrationChoice::Manual),
            other => Err(AppError::AiReviewInvalid(format!(
                "unknown arbitration choice: {other} (expected ours, theirs, ai or manual)"
            ))),
        }
    }
}

/// One audit record: the whole of ADR-0024 §3.7.3's field list.
///
/// `[FACT]` The field names are the ADR's own, so the record reads against
/// the ADR without a translation table. The file is a backend artifact, not
/// a wire format between the webview and the backend, so it does not use the
/// desktop's camelCase convention.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuditRecord {
    /// The region's stable id (ADR-0024 §3.7.2), recomputed by the backend.
    pub region_id: String,
    /// The repository-relative path of the conflicted file.
    pub path: String,
    /// The registry key of the provider that was consulted; empty when no
    /// model was consulted.
    pub provider: String,
    /// The model the **provider reported**; empty when it reported none.
    pub model: String,
    /// How many suspected secrets were replaced before the request was sent.
    pub redactions: usize,
    /// The digest of the (redacted) prompt that was sent.
    pub prompt_sha256: String,
    /// The digest of the answer recorded in `answer_text`.
    pub answer_sha256: String,
    /// The answer, bounded and scrubbed of the credential.
    pub answer_text: String,
    /// What the human chose: `ours` / `theirs` / `ai` / `manual`.
    pub choice: String,
    /// When the choice was made (RFC 3339, UTC).
    pub applied_at: String,
}

/// What the backend saw during one arbitration.
///
/// `[INFERENCE]` This is the half of the record the webview cannot produce
/// honestly: the model the provider *reported*, the redaction count, the
/// digest of the prompt that was sent, and the answer. The backend keeps it
/// for the duration of the session so that the decision — which happens
/// later, in the webview — can be described in full.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArbitrationFacts {
    /// The registry key of the provider that was consulted.
    pub provider: String,
    /// The model the provider reported. Empty until it reports one, and
    /// empty forever if it never does — which is the honest answer, and
    /// deliberately *not* the model that was requested.
    pub model: String,
    /// How many suspected secrets were replaced before sending.
    pub redactions: usize,
    /// The digest of the prompt that was sent.
    pub prompt_sha256: String,
    /// The digest of the answer.
    pub answer_sha256: String,
    /// The answer, bounded and scrubbed.
    pub answer_text: String,
}

impl ArbitrationFacts {
    /// The facts of an arbitration that has not consulted a model.
    fn empty() -> Self {
        Self {
            provider: String::new(),
            model: String::new(),
            redactions: 0,
            prompt_sha256: String::new(),
            answer_sha256: String::new(),
            answer_text: String::new(),
        }
    }
}

/// The arbitrations the backend remembers, oldest first.
fn facts_store() -> &'static Mutex<VecDeque<(String, ArbitrationFacts)>> {
    static STORE: OnceLock<Mutex<VecDeque<(String, ArbitrationFacts)>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(VecDeque::new()))
}

/// Remember what an arbitration looked like.
///
/// `[INFERENCE]` The entry is replaced, not appended to: the facts grow as
/// the arbitration proceeds (the model arrives, then the answer), and the
/// record must describe the *last* state of it.
pub fn remember_facts(region_id: &str, facts: ArbitrationFacts) {
    let Ok(mut store) = facts_store().lock() else {
        // `[INFERENCE]` A poisoned mutex means another thread panicked while
        // holding it. Losing an audit fact must not take the arbitration
        // down with it, so the write is dropped and the arbitration
        // continues.
        return;
    };
    if let Some(slot) = store.iter_mut().find(|(id, _)| id == region_id) {
        slot.1 = facts;
        return;
    }
    while store.len() >= MAX_REMEMBERED_ARBITRATIONS {
        store.pop_front();
    }
    store.push_back((String::from(region_id), facts));
}

/// What the backend remembers about `region_id`, if it arbitrated it.
pub fn recall_facts(region_id: &str) -> Option<ArbitrationFacts> {
    let store = facts_store().lock().ok()?;
    store
        .iter()
        .find(|(id, _)| id == region_id)
        .map(|(_, facts)| facts.clone())
}

/// The digest of the prompt that was sent.
///
/// `[FACT]` The digest covers the **messages**, in order, each one as its
/// wire role, a NUL, the content, and another NUL. The NUL separators are
/// what make the digest unambiguous: without them a message boundary and a
/// newline inside a message would hash the same.
///
/// `[INFERENCE]` The model is deliberately **not** part of the digest: the
/// record has a `model` field for it, and the digest is about the prompt.
pub fn prompt_digest(messages: &[ChatMessage]) -> String {
    let mut buf: Vec<u8> = Vec::new();
    for message in messages {
        buf.extend_from_slice(message.role.as_openai().as_bytes());
        buf.push(0);
        buf.extend_from_slice(message.content.as_bytes());
        buf.push(0);
    }
    sha256_hex(&buf)
}

/// The answer as it is recorded: scrubbed of the credential, and bounded.
///
/// `[FACT]` The bound is [`CONFLICT_MAX_ANSWER_CHARS`] — the same number the
/// answer itself is capped at, so the recorded answer is the whole answer
/// unless the credential was in it (the redaction marker is longer than the
/// span it replaces, so the bound is applied *after* the scrub).
///
/// `[INFERENCE]` The digest is taken over the text that is **stored**, so
/// `sha256(answer_text) == answer_sha256` holds for the record as it lies on
/// disk. A digest of the pre-scrub text would not be verifiable by whoever
/// reads the file.
pub fn bounded_answer(answer: &str, api_key: &str) -> (String, String) {
    let scrubbed = sanitize::scrub_secret(answer, api_key);
    let bounded: String = scrubbed.chars().take(CONFLICT_MAX_ANSWER_CHARS).collect();
    let digest = sha256_hex(bounded.as_bytes());
    (digest, bounded)
}

/// Build the record for one decision.
///
/// `[INFERENCE]` The region id is **recomputed** from the region's own
/// content rather than taken from the caller: the id is what binds a
/// decision to a region, so a caller that named a different region would
/// otherwise be able to write a record about it.
///
/// `[INFERENCE]` The model fields come from the facts, so they are empty for
/// a decision that never consulted a model (keep ours / keep theirs / hand
/// edit). "No model was consulted" is the honest answer there, and it is
/// distinguishable from "a model was consulted and said nothing".
pub fn build_record(
    region: &ConflictRegion,
    choice: ArbitrationChoice,
    facts: Option<&ArbitrationFacts>,
    applied_at: String,
) -> AuditRecord {
    let facts = facts.cloned().unwrap_or_else(ArbitrationFacts::empty);
    AuditRecord {
        region_id: region.id(),
        path: region.path.clone(),
        provider: facts.provider,
        model: facts.model,
        redactions: facts.redactions,
        prompt_sha256: facts.prompt_sha256,
        answer_sha256: facts.answer_sha256,
        answer_text: facts.answer_text,
        choice: String::from(choice.as_str()),
        applied_at,
    }
}

/// The current time, as RFC 3339 in UTC.
fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Append one line to the audit file under `dir`.
///
/// `[FACT]` The file is opened in **append** mode and never truncated, so a
/// record that is already on disk cannot be changed by a later arbitration.
/// The write is flushed before the call returns: a trail that loses its last
/// record when the app is killed is not a trail.
fn append_line(dir: &Path, line: &str) -> AppResult<()> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(AUDIT_FILE_NAME);
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    file.write_all(line.as_bytes())?;
    file.write_all(b"\n")?;
    file.flush()?;
    Ok(())
}

/// Refuse a line that carries the credential.
///
/// `[FACT]` The check is on the **whole line** — the serialized record, not
/// one field — because the credential could arrive inside the answer, which
/// is the only provider-authored text in the record. An empty key is not a
/// credential and is not searched for: `str::contains("")` is always true,
/// so an empty key would refuse every record.
fn refuse_if_carries_secret(line: &str, api_key: Option<&str>) -> AppResult<()> {
    match api_key {
        Some(key) if !key.is_empty() && line.contains(key) => {
            Err(AppError::Internal(String::from(
                "the audit record would have carried the provider key; it was not written",
            )))
        }
        _ => Ok(()),
    }
}

/// Write one record to the trail under `dir`.
///
/// `[FACT]` The line is checked against the credential **before** it is
/// appended, so a record that carries the credential is never written — not
/// "written and then removed", which would leave the key in the file's
/// history if the file were ever rotated.
fn write_record(dir: &Path, record: &AuditRecord, api_key: Option<&str>) -> AppResult<()> {
    let line = serde_json::to_string(record)
        .map_err(|e| AppError::Internal(format!("could not serialize the audit record: {e}")))?;
    refuse_if_carries_secret(&line, api_key)?;
    append_line(dir, &line)
}

/// Write the audit record for one decision (ADR-0024 §3.7.3).
///
/// `[FACT]` The webview sends the **region** and the **choice**; everything
/// else in the record is the backend's own account of what happened.
#[tauri::command]
pub async fn conflict_audit_record(
    state: State<'_, DesktopState>,
    region: ConflictRegionDto,
    choice: String,
) -> AppResult<()> {
    let choice = ArbitrationChoice::parse(&choice)?;
    let region: ConflictRegion = region.into();
    let facts = recall_facts(&region.id());
    let record = build_record(&region, choice, facts.as_ref(), now_rfc3339());

    // The key discipline, checked on the whole record rather than on one
    // field: the credential must not appear anywhere in what is written.
    //
    // `[INFERENCE]` The key is resolved rather than carried around, so the
    // check runs against the credential that is actually configured for this
    // provider. A record that names no provider consulted no model, and
    // carries no provider-authored text, so there is nothing to check.
    let api_key = if record.provider.is_empty() {
        None
    } else {
        resolve_ai_key(state.vault.as_ref(), &record.provider).await
    };

    write_record(&state.app_data_dir, &record, api_key.as_deref())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use gitgit::ai::provider::Role;

    /// A region with three distinct sides.
    ///
    /// `[INFERENCE]` The path is this module's own: the backend keeps one
    /// set of arbitration facts **per region id**, and the region id is a
    /// hash of the region's content. A region shared with another test
    /// would give two tests one set of facts, and the second to finish
    /// would overwrite the first.
    fn region() -> ConflictRegion {
        ConflictRegion {
            path: String::from("src/audit-region.rs"),
            base_oid: String::from("0123456789abcdef0123456789abcdef01234567"),
            index: 0,
            ours: String::from("let a = 1;\n"),
            base: String::from("let a = 0;\n"),
            theirs: String::from("let a = 2;\n"),
            context_before: String::from("fn main() {\n"),
            context_after: String::from("}\n"),
        }
    }

    /// Facts of an arbitration that consulted a model.
    fn facts() -> ArbitrationFacts {
        ArbitrationFacts {
            provider: String::from("openai"),
            model: String::from("served-model"),
            redactions: 2,
            prompt_sha256: "a".repeat(64),
            answer_sha256: "b".repeat(64),
            answer_text: String::from("let a = 3;\n"),
        }
    }

    /// A directory that no other test uses.
    fn temp_dir(label: &str) -> std::path::PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("gm-desktop-audit-{label}-{n}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    /// The lines of the audit file under `dir`.
    fn lines(dir: &std::path::Path) -> Vec<String> {
        let path = dir.join(AUDIT_FILE_NAME);
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        text.lines().map(String::from).collect()
    }

    #[test]
    fn a_record_carries_every_field_the_adr_lists() {
        // ADR-0024 §3.7.3's table, field by field. A record that is missing
        // one of them is a record that cannot answer the question the ADR
        // asked it to answer.
        let record = build_record(
            &region(),
            ArbitrationChoice::Ai,
            Some(&facts()),
            String::from("2026-11-21T07:05:00+00:00"),
        );
        let value = serde_json::to_value(&record).expect("the record serializes");
        for field in [
            "region_id",
            "path",
            "provider",
            "model",
            "redactions",
            "prompt_sha256",
            "answer_sha256",
            "answer_text",
            "choice",
            "applied_at",
        ] {
            assert!(
                value.get(field).is_some(),
                "the record has no {field}: {value}"
            );
        }
        assert_eq!(record.region_id, region().id(), "the id is recomputed");
        assert_eq!(record.path, "src/audit-region.rs");
        assert_eq!(record.model, "served-model");
        assert_eq!(record.redactions, 2);
        assert_eq!(record.choice, "ai");
        assert_eq!(record.applied_at, "2026-11-21T07:05:00+00:00");
    }

    #[test]
    fn a_decision_that_consulted_no_model_says_so() {
        // "No model was consulted" is the honest answer for keep-ours, and
        // it is distinguishable from "a model was consulted and said
        // nothing": the latter would carry a provider name.
        let record = build_record(
            &region(),
            ArbitrationChoice::Ours,
            None,
            String::from("2026-11-21T07:05:00+00:00"),
        );
        assert_eq!(record.provider, "");
        assert_eq!(record.model, "");
        assert_eq!(record.redactions, 0);
        assert_eq!(record.prompt_sha256, "");
        assert_eq!(record.answer_sha256, "");
        assert_eq!(record.answer_text, "");
        assert_eq!(record.choice, "ours");
        // The region is still identified: the decision is about *something*.
        assert_eq!(record.region_id, region().id());
    }

    #[test]
    fn two_arbitrations_append_two_records_and_the_first_is_unchanged() {
        // The acceptance criterion: the trail is append-only. The second
        // record must not disturb the first one.
        let dir = temp_dir("append");
        let first = build_record(
            &region(),
            ArbitrationChoice::Ours,
            Some(&facts()),
            String::from("2026-11-21T07:05:00+00:00"),
        );
        let second = build_record(
            &region(),
            ArbitrationChoice::Theirs,
            Some(&facts()),
            String::from("2026-11-21T07:06:00+00:00"),
        );
        append_line(&dir, &serde_json::to_string(&first).expect("serializes")).expect("write");
        append_line(&dir, &serde_json::to_string(&second).expect("serializes")).expect("write");

        let written = lines(&dir);
        assert_eq!(
            written.len(),
            2,
            "two arbitrations, two records: {written:?}"
        );
        assert_eq!(
            written[0],
            serde_json::to_string(&first).expect("serializes")
        );
        assert_eq!(
            written[1],
            serde_json::to_string(&second).expect("serializes")
        );
        // The first line is byte-identical, not merely equal.
        let raw = std::fs::read(dir.join(AUDIT_FILE_NAME)).expect("read");
        let first_line = String::from_utf8_lossy(&raw)
            .lines()
            .next()
            .unwrap_or_default()
            .to_string();
        assert_eq!(first_line, written[0], "the first record was rewritten");
    }

    #[test]
    fn the_record_never_carries_the_api_key() {
        // The answer is the only provider-authored text in the record, and a
        // misconfigured proxy can echo the `Authorization` header back inside
        // it. The scrub happens where the answer is captured, so the text
        // that is stored cannot carry the credential.
        let (digest, text) =
            bounded_answer("the key is sk-SUPERSECRET-VALUE\n", "sk-SUPERSECRET-VALUE");
        assert!(!text.contains("SUPERSECRET"), "the key survived: {text}");
        assert!(text.contains(sanitize::REDACTED), "got: {text}");
        assert_eq!(digest, sha256_hex(text.as_bytes()));

        // And the whole record is checked against the key before it is
        // written.
        let mut facts = facts();
        facts.answer_text = text;
        facts.answer_sha256 = digest;
        let record = build_record(
            &region(),
            ArbitrationChoice::Ai,
            Some(&facts),
            String::from("2026-11-21T07:05:00+00:00"),
        );
        let line = serde_json::to_string(&record).expect("serializes");
        assert!(
            !line.contains("SUPERSECRET"),
            "the key is in the record: {line}"
        );
    }

    #[test]
    fn a_record_that_carries_the_key_is_refused_and_never_written() {
        // The scrub above is the first line of defence; this is the last one.
        // A record that somehow carried the credential must not reach the
        // file at all — not be written and then removed, which would leave
        // the key in the file's own bytes.
        let dir = temp_dir("refuse");
        let key = "sk-SUPERSECRET-VALUE";
        let mut facts = facts();
        facts.answer_text = String::from("the key is sk-SUPERSECRET-VALUE\n");
        facts.answer_sha256 = sha256_hex(facts.answer_text.as_bytes());
        let record = build_record(
            &region(),
            ArbitrationChoice::Ai,
            Some(&facts),
            String::from("2026-11-21T07:05:00+00:00"),
        );

        let err = write_record(&dir, &record, Some(key)).expect_err("the record is refused");
        assert_eq!(err.kind(), "Internal", "got: {err}");
        assert!(
            !err.to_string().contains("SUPERSECRET"),
            "the error message carries the key: {err}"
        );
        assert!(
            !dir.join(AUDIT_FILE_NAME).exists(),
            "the refused record was written anyway"
        );
        assert!(
            lines(&dir).is_empty(),
            "the refused record left a line behind"
        );
    }

    #[test]
    fn one_decision_appends_exactly_one_line() {
        // The acceptance criterion on the backend side: one decision, one
        // record, one line.
        let dir = temp_dir("one");
        let record = build_record(
            &region(),
            ArbitrationChoice::Ours,
            Some(&facts()),
            String::from("2026-11-21T07:05:00+00:00"),
        );
        write_record(&dir, &record, Some("sk-SOME-OTHER-KEY")).expect("written");
        assert_eq!(lines(&dir).len(), 1, "one decision, one line");
        // A record that carries no key is written; the guard does not
        // refuse records merely because a key is configured.
        let second = build_record(
            &region(),
            ArbitrationChoice::Theirs,
            Some(&facts()),
            String::from("2026-11-21T07:06:00+00:00"),
        );
        write_record(&dir, &second, Some("sk-SOME-OTHER-KEY")).expect("written");
        assert_eq!(lines(&dir).len(), 2, "two decisions, two lines");
    }

    #[test]
    fn the_digest_of_the_stored_answer_is_the_digest_in_the_record() {
        // Whoever reads the file can check the record against itself.
        let (digest, text) = bounded_answer("hello\n", "k");
        assert_eq!(digest, sha256_hex(text.as_bytes()));
        assert_eq!(digest.len(), 64, "a sha-256 digest is 64 hex characters");
    }

    #[test]
    fn the_prompt_digest_separates_the_messages() {
        // Two different prompts must not share a digest, and the same prompt
        // must always produce the same one.
        let a = vec![
            ChatMessage::system("one"),
            ChatMessage::untrusted(Role::User, "two"),
        ];
        let b = vec![
            ChatMessage::system("one"),
            ChatMessage::untrusted(Role::User, "three"),
        ];
        assert_eq!(prompt_digest(&a), prompt_digest(&a), "the digest is stable");
        assert_ne!(prompt_digest(&a), prompt_digest(&b));
        assert_eq!(prompt_digest(&a).len(), 64);
    }

    #[test]
    fn an_unknown_choice_is_refused_rather_than_coerced() {
        // A record that says "ours" because the caller sent nonsense is
        // worse than no record: it is a wrong record.
        let err = ArbitrationChoice::parse("whatever").unwrap_err();
        assert_eq!(err.kind(), "AiReviewInvalid", "got: {err}");
        assert!(err.to_string().contains("whatever"), "got: {err}");
    }

    #[test]
    fn every_choice_the_adr_lists_round_trips() {
        for (text, expected) in [
            ("ours", ArbitrationChoice::Ours),
            ("theirs", ArbitrationChoice::Theirs),
            ("ai", ArbitrationChoice::Ai),
            ("manual", ArbitrationChoice::Manual),
        ] {
            assert_eq!(ArbitrationChoice::parse(text).unwrap(), expected);
            assert_eq!(expected.as_str(), text);
        }
    }

    #[test]
    fn the_facts_of_an_arbitration_are_remembered_and_recalled() {
        let region = region();
        let id = region.id();
        remember_facts(&id, facts());
        let recalled = recall_facts(&id).expect("the facts are remembered");
        assert_eq!(recalled.model, "served-model");
        assert_eq!(recalled.redactions, 2);
        // A different region has no facts: the record of a decision about a
        // region that was never arbitrated says that no model was consulted.
        assert!(recall_facts("ffffffffffffffffffffffffffffffff").is_none());
    }
}
