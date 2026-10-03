//! Task prompts for the `gitai` subcommands.
//!
//! Each builder returns `(system, untrusted_user_content)`. The split is
//! structural, not cosmetic: [`crate::ai::sanitize::sanitize_request`]
//! tags the second half as untrusted and redacts secrets from it, while
//! the first half is operator-authored instructions that survive verbatim.
//! A builder that inlined the diff into the system prompt would silently
//! opt the diff out of both protections.

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
}
