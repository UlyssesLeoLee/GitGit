//! Task prompts for the `gitai` subcommands.
//!
//! Each builder returns `(system, untrusted_user_content)`. The split is
//! structural, not cosmetic: [`crate::ai::sanitize::sanitize_request`]
//! tags the second half as untrusted and redacts secrets from it, while
//! the first half is operator-authored instructions that survive verbatim.
//! A builder that inlined the diff into the system prompt would silently
//! opt the diff out of both protections.

use crate::ai::conflict::{ConflictRegion, MAX_CONTEXT_LINES};
use crate::ai::provider::{ChatMessage, Role, Trust};

/// `gitai commit` — write a commit message from a diff.
pub fn commit(diff: &str) -> Vec<ChatMessage> {
    vec![
        ChatMessage::system(
            "You write git commit messages. Given a diff, reply with the commit \
             message text and nothing else: no preamble, no explanation, no \
             markdown code fences, no \"here is the message\" framing.\n\
             Format: a subject line of at most 72 characters in the imperative \
             mood (\"add\", not \"added\" / \"adds\"), optionally followed by a \
             blank line and a body of wrapped prose explaining what changed and \
             why. Describe what the diff does, not how the diff is formatted.",
        ),
        ChatMessage {
            role: Role::User,
            content: format!(
                "Write a commit message for this diff.\n\n<diff_summary>\n{}\n</diff_summary>",
                summarize(diff)
            ),
            trust: Trust::Untrusted,
        },
    ]
}

/// `gitai explain` — explain what a commit or diff did.
pub fn explain(diff: &str) -> Vec<ChatMessage> {
    vec![
        ChatMessage::system(
            "You explain changes to people who will review them. Lead with what \
             the change does, then why it is likely being made. Call out \
             behaviour changes, data migrations, and anything that could break a \
             caller. Be specific about file and symbol names. If the diff is \
             ambiguous, say so rather than inventing intent.",
        ),
        ChatMessage {
            role: Role::User,
            content: format!("Explain this change.\n\n{}", summarize(diff)),
            trust: Trust::Untrusted,
        },
    ]
}

/// `gitai review` — flag risks in a diff.
pub fn review(diff: &str) -> Vec<ChatMessage> {
    vec![
        ChatMessage::system(
            "You review diffs for correctness, security, and maintainability. \
             Report only findings you can point at in the diff. For each, give \
             the file, the concern, and why it matters. Rank by severity. If \
             you find nothing worth flagging, say so plainly — do not invent \
             issues to fill the space.",
        ),
        ChatMessage {
            role: Role::User,
            content: format!("Review this diff.\n\n{}", summarize(diff)),
            trust: Trust::Untrusted,
        },
    ]
}

/// ADR-0024 §3.3 — arbitrate **one** conflict region.
///
/// The system turn is operator-authored and trusted. The user turn carries
/// the region — the three sides and the context around them — and is
/// therefore untrusted in its entirety: the region is content from the other
/// branch, and the other branch is somebody else's.
///
/// `[FACT]` The region text must appear in exactly one untrusted turn and
/// nowhere else. A builder that inlined the region into the system prompt
/// would silently opt it out of both the structural tag and the secret
/// redaction, which is the same property the other builders here hold.
///
/// `[PROPOSAL]` The reply format is part of the contract: the model's first
/// line is the region id, copied verbatim, and the replacement text follows.
/// [`crate::ai::conflict::validate_answer`] refuses an answer that does not
/// carry the id rather than guessing which region it was meant for.
pub fn conflict(region: &ConflictRegion) -> Vec<ChatMessage> {
    vec![
        ChatMessage::system(
            "You arbitrate one Git merge conflict region. The three sides of the \
             conflict and the lines around it are given below.\n\
             Reply with exactly two parts and nothing else:\n\
             1. the first line is the region id, copied verbatim from the request, \
             and nothing else on that line;\n\
             2. after that line, the replacement text for the whole region — the \
             text that should take the place of the conflicting block.\n\
             Do not explain, do not use markdown code fences, do not repeat the \
             region id anywhere else. The replacement must not contain conflict \
             markers. Keep the surrounding code's language, indentation and style. \
             If the two sides are equivalent, repeat that text unchanged. If you \
             cannot tell which side is right, reply with the ours side unchanged.",
        ),
        ChatMessage {
            role: Role::User,
            content: conflict_user_content(region),
            trust: Trust::Untrusted,
        },
    ]
}

/// The untrusted body of a conflict request.
///
/// Every field in it comes from the repository, so every field is data and
/// none of it is an instruction.
fn conflict_user_content(region: &ConflictRegion) -> String {
    format!(
        "Resolve one merge conflict region.\n\n\
         region_id: {id}\n\
         path: {path}\n\
         language: {language}\n\n\
         --- ours ---\n\
         <ours>\n{ours}\n</ours>\n\n\
         --- base ---\n\
         <base>\n{base}\n</base>\n\n\
         --- theirs ---\n\
         <theirs>\n{theirs}\n</theirs>\n\n\
         --- context before the region (up to {MAX_CONTEXT_LINES} lines) ---\n\
         {before}\n\n\
         --- context after the region (up to {MAX_CONTEXT_LINES} lines) ---\n\
         {after}\n",
        id = region.id(),
        path = region.path,
        language = region.language(),
        ours = region.ours,
        base = region.base,
        theirs = region.theirs,
        before = clamp_context_before(&region.context_before),
        after = clamp_context_after(&region.context_after),
    )
}

/// How many secret-shaped spans a conflict request carries.
///
/// `[FACT]` The count is taken over exactly the text that will be sent —
/// the untrusted body — so the number the UI shows ("N suspected secrets
/// were replaced") describes the request that actually left the machine
/// rather than a re-run of the scrubber over an approximation of it.
pub fn conflict_redactions(region: &ConflictRegion) -> usize {
    crate::ai::sanitize::scrub(&conflict_user_content(region)).1.len()
}

/// Keep the last [`MAX_CONTEXT_LINES`] lines of the context before a region.
///
/// The lines nearest the region are the ones that matter, so the *end* of the
/// "before" context is what survives a clamp.
fn clamp_context_before(text: &str) -> String {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    if lines.len() <= MAX_CONTEXT_LINES {
        return text.to_string();
    }
    let kept: String = lines[lines.len() - MAX_CONTEXT_LINES..].concat();
    format!(
        "{kept}\n[truncated by gitgit: showing the last {MAX_CONTEXT_LINES} of {} context \
         lines. The rest of the context was not sent to the provider.]\n",
        lines.len()
    )
}

/// Keep the first [`MAX_CONTEXT_LINES`] lines of the context after a region.
fn clamp_context_after(text: &str) -> String {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    if lines.len() <= MAX_CONTEXT_LINES {
        return text.to_string();
    }
    let kept: String = lines[..MAX_CONTEXT_LINES].concat();
    format!(
        "{kept}\n[truncated by gitgit: showing the first {MAX_CONTEXT_LINES} of {} context \
         lines. The rest of the context was not sent to the provider.]\n",
        lines.len()
    )
}

/// How much of a diff to send.
///
/// `[INFERENCE]` A full diff of a large refactor can exceed the model's
/// context window, and the failure mode when that happens is opaque — a
/// truncated request that looks like a bad model rather than a too-big
/// input. Clamping here with an explicit notice lets the CLI report
/// "showing the first N of M characters" instead of silently sending less.
pub const MAX_DIFF_CHARS: usize = 60_000;

/// Clamp `diff` to [`MAX_DIFF_CHARS`], appending a visible marker when
/// anything was dropped.
///
/// The marker is part of the returned string and is therefore subject to
/// redaction like any other untrusted content; it contains no secrets, so
/// it is a no-op for the scrubber.
pub fn summarize(diff: &str) -> String {
    if diff.chars().count() <= MAX_DIFF_CHARS {
        return diff.to_string();
    }
    let kept: String = diff.chars().take(MAX_DIFF_CHARS).collect();
    let total = diff.chars().count();
    format!(
        "{kept}\n\n[truncated by gitgit: showing the first {MAX_DIFF_CHARS} of {total} \
         characters. The rest of the diff was not sent to the provider.]"
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// A region whose three sides and two context blocks are all
    /// distinguishable, so a leak can be spotted by name.
    fn conflict_region() -> ConflictRegion {
        ConflictRegion {
            path: String::from("src/foo.rs"),
            base_oid: String::from("0123456789abcdef0123456789abcdef01234567"),
            index: 0,
            ours: String::from("OURS-SIDE-TEXT\n"),
            base: String::from("BASE-SIDE-TEXT\n"),
            theirs: String::from("THEIRS-SIDE-TEXT\n"),
            context_before: String::from("BEFORE-CONTEXT\n"),
            context_after: String::from("AFTER-CONTEXT\n"),
        }
    }

    #[test]
    fn every_builder_keeps_the_diff_out_of_the_system_prompt() {
        // The security property: untrusted content must arrive as an
        // untrusted *user* turn, never inside operator instructions.
        for build in [commit as fn(&str) -> Vec<ChatMessage>, explain, review] {
            let msgs = build("DIFFBODY");
            let system = msgs.iter().find(|m| m.role == Role::System).unwrap();
            assert!(
                !system.content.contains("DIFFBODY"),
                "diff leaked into system prompt: {}",
                system.content
            );
            let untrusted: Vec<_> = msgs
                .iter()
                .filter(|m| m.trust == Trust::Untrusted)
                .collect();
            assert_eq!(untrusted.len(), 1, "expected exactly one untrusted turn");
            assert!(
                untrusted[0].content.contains("DIFFBODY"),
                "diff missing from the untrusted turn"
            );
        }
    }

    #[test]
    fn short_diff_is_passed_through_unchanged() {
        assert_eq!(summarize("small diff"), "small diff");
    }

    #[test]
    fn oversized_diff_is_clamped_and_says_so() {
        let big = "x".repeat(MAX_DIFF_CHARS + 5_000);
        let out = summarize(&big);
        assert!(out.len() < big.len(), "output was not clamped");
        assert!(out.contains("truncated by gitgit"), "no notice: {out}");
        // The notice must state the real total so the operator can see
        // how much was withheld rather than guessing.
        assert!(
            out.contains(&(MAX_DIFF_CHARS + 5_000).to_string()),
            "notice omits the true size"
        );
    }

    /// ADR-0024 §3.3, and the property every builder in this file holds:
    /// untrusted content arrives as an untrusted *user* turn, never inside
    /// operator instructions.
    #[test]
    fn the_conflict_prompt_keeps_the_region_out_of_the_system_prompt() {
        let region = conflict_region();
        let msgs = conflict(&region);
        let system = msgs
            .iter()
            .find(|m| m.role == Role::System)
            .expect("the builder emits a system turn");
        for side in [&region.ours, &region.base, &region.theirs] {
            assert!(
                !system.content.contains(side.as_str()),
                "the region leaked into the system prompt: {}",
                system.content
            );
        }
        let untrusted: Vec<_> = msgs
            .iter()
            .filter(|m| m.trust == Trust::Untrusted)
            .collect();
        assert_eq!(untrusted.len(), 1, "expected exactly one untrusted turn");
        for side in [&region.ours, &region.base, &region.theirs] {
            assert!(
                untrusted[0].content.contains(side.as_str()),
                "a side of the region is missing from the untrusted turn"
            );
        }
        // "Only in one untrusted message" is stronger than "in an untrusted
        // message": the region must not also ride along in a second turn,
        // because that turn would not be the one the sanitizer rewrote.
        for side in [&region.ours, &region.base, &region.theirs] {
            let carriers = msgs
                .iter()
                .filter(|m| m.content.contains(side.as_str()))
                .count();
            assert_eq!(carriers, 1, "the region text rides in {carriers} messages");
        }
    }

    /// ADR-0024 §3.7.2: a request carries exactly one region, so a second
    /// region's text must not appear in the first region's request.
    #[test]
    fn a_conflict_request_carries_exactly_one_region() {
        let mut other = conflict_region();
        other.index = 1;
        other.ours = String::from("OTHER-REGION-OURS\n");
        other.base = String::from("OTHER-REGION-BASE\n");
        other.theirs = String::from("OTHER-REGION-THEIRS\n");
        let msgs = conflict(&conflict_region());
        let body = msgs
            .iter()
            .find(|m| m.trust == Trust::Untrusted)
            .expect("the builder emits an untrusted turn");
        assert!(
            !body.content.contains("OTHER-REGION-OURS"),
            "a second region leaked into the request: {}",
            body.content
        );
    }

    /// ADR-0024 §3.3: the region is repository content, so a secret-shaped
    /// span inside it is replaced before the request leaves the machine.
    #[test]
    fn a_secret_in_the_region_is_redacted_before_the_request_leaves() {
        let mut region = conflict_region();
        region.ours = String::from("let api_key = \"super-secret-value-1234\";\n");
        let mut req = crate::ai::provider::ChatRequest::new("model", conflict(&region));
        crate::ai::sanitize::sanitize_request(&mut req);
        let untrusted = req
            .messages
            .iter()
            .find(|m| m.trust == Trust::Untrusted)
            .expect("the builder emits an untrusted turn");
        assert!(
            !untrusted.content.contains("super-secret-value-1234"),
            "the secret survived the sanitizer: {}",
            untrusted.content
        );
        assert!(
            untrusted.content.contains(crate::ai::sanitize::REDACTED),
            "the secret was not replaced by the redaction marker: {}",
            untrusted.content
        );
        assert_eq!(
            conflict_redactions(&region),
            1,
            "one secret-shaped span in the region must be reported"
        );
    }

    #[test]
    fn the_conflict_prompt_carries_the_region_id_and_the_context() {
        // The id is what binds the answer back to this region (ADR §3.7.2),
        // so it has to be in the request.
        let region = conflict_region();
        let msgs = conflict(&region);
        let untrusted = msgs
            .iter()
            .find(|m| m.trust == Trust::Untrusted)
            .expect("the builder emits an untrusted turn");
        assert!(
            untrusted.content.contains(&region.id()),
            "the region id must be sent: {}",
            untrusted.content
        );
        assert!(
            untrusted.content.contains("BEFORE-CONTEXT"),
            "the context before the region must be sent"
        );
        assert!(
            untrusted.content.contains("AFTER-CONTEXT"),
            "the context after the region must be sent"
        );
    }

    #[test]
    fn an_over_long_context_is_clamped_and_says_so() {
        let mut region = conflict_region();
        region.context_before = "BEFORE\n".repeat(MAX_CONTEXT_LINES + 5);
        let msgs = conflict(&region);
        let untrusted = msgs
            .iter()
            .find(|m| m.trust == Trust::Untrusted)
            .expect("the builder emits an untrusted turn");
        assert!(
            untrusted.content.contains("truncated by gitgit"),
            "a clamp without a notice is a silent clamp: {}",
            untrusted.content
        );
        assert_eq!(
            untrusted.content.matches("BEFORE\n").count(),
            MAX_CONTEXT_LINES,
            "only the lines nearest the region survive the clamp"
        );
    }
}
