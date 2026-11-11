//! OpenAI-compatible Server-Sent-Events decoding.
//!
//! This module is deliberately **pure**: it turns bytes into decoded SSE
//! fields and has no knowledge of HTTP, providers, or credentials. That
//! split is what makes the awkward parts of the protocol — a chunk that
//! splits a line in half, a `data:` field with no space after the colon,
//! a comment line — testable without a socket.
//!
//! # Wire format
//!
//! The OpenAI-compatible streaming shape is a sequence of frames, each
//! one or more `field: value` lines terminated by a blank line:
//!
//! ```text
//! data: {"choices":[{"delta":{"content":"Hel"}}]}
//!
//! data: [DONE]
//! ```
//!
//! Per the SSE specification the single space after the colon is part of
//! the framing, not the payload, so exactly one leading U+0020 is
//! stripped. Fields other than `data` (`event:`, `id:`, `retry:`) and
//! comment lines (a leading `:`) carry nothing this transport needs and
//! are dropped — see [`SseField::Ignored`].
//!
//! # What a provider cannot do
//!
//! The Anthropic streaming shape is different (it emits named
//! `content_block_delta` events over a differently-shaped JSON body), so
//! this decoder does **not** apply to
//! [`crate::ai::anthropic`]. A caller that asks an Anthropic-backed
//! provider to stream gets an explicit error rather than a silent
//! fall back to a non-streaming request — see
//! [`crate::ai::provider::AiProvider::send_stream`].

use tokio::sync::mpsc;

use crate::error::{GitGitError, Result};

/// The terminator frame every OpenAI-compatible stream ends with.
pub const SSE_DONE: &str = "[DONE]";

/// Backpressure bound for a stream channel.
///
/// Small on purpose: the consumer is a UI, and a slow UI should make the
/// provider wait rather than accumulate an unbounded token buffer in
/// memory.
pub const STREAM_CHANNEL_CAPACITY: usize = 64;

/// One decoded field line from an SSE body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SseField {
    /// A `data:` payload with the single leading space removed.
    Data(String),
    /// `data: [DONE]` — the provider signalled end of stream.
    Done,
    /// Everything else: blank separator lines, `:` comments, `event:` /
    /// `id:` / `retry:` fields, and any field this decoder does not
    /// recognise. Per the SSE specification an unrecognised field is
    /// ignored rather than treated as an error.
    Ignored,
}

/// Incremental SSE line decoder.
///
/// Feed it raw network bytes with [`SseDecoder::push`]; it returns every
/// line the chunk completed. Bytes that arrive mid-line stay buffered
/// until the terminating newline shows up, which is what makes a
/// fragmented TCP chunk a non-event.
#[derive(Debug, Default)]
pub struct SseDecoder {
    buf: Vec<u8>,
}

impl SseDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed one network chunk; returns the lines it closed.
    pub fn push(&mut self, chunk: &[u8]) -> Vec<SseField> {
        self.buf.extend_from_slice(chunk);
        let mut out = Vec::new();
        while let Some(nl) = self.buf.iter().position(|b| *b == b'\n') {
            let mut line: Vec<u8> = self.buf.drain(..=nl).collect();
            // Drop the '\n' and, for CRLF servers, the '\r' immediately
            // before it. Only a trailing '\r' is removed, so a '\r' that
            // is genuinely part of the payload survives.
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            out.push(classify(&line));
        }
        out
    }

    /// Bytes held back because no newline has arrived yet.
    pub fn pending(&self) -> usize {
        self.buf.len()
    }
}

/// Classify one complete (newline-stripped) line.
fn classify(line: &[u8]) -> SseField {
    // A split multi-byte character cannot survive to this point — a line
    // is only decoded once its terminator arrived, so the whole line is
    // present. Lossy conversion still guards against a provider that
    // emits genuinely invalid UTF-8: degrading those bytes is better
    // than discarding a frame of otherwise-usable model output.
    let text = String::from_utf8_lossy(line);
    if text.is_empty() || text.starts_with(':') {
        return SseField::Ignored;
    }
    let Some(payload) = text.strip_prefix("data:") else {
        return SseField::Ignored;
    };
    // Exactly one leading space is framing, per the SSE specification.
    let payload = payload.strip_prefix(' ').unwrap_or(payload);
    if payload == SSE_DONE {
        return SseField::Done;
    }
    SseField::Data(payload.to_string())
}

/// One event on a decoded chat-completion stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamEvent {
    /// An incremental chunk of assistant text.
    Token(String),
    /// The model the provider says it served. Recorded rather than
    /// assumed, for the same reason [`crate::ai::provider::ChatResponse`]
    /// records it.
    Model(String),
    /// The stream completed normally. Always the last event.
    Finished,
}

/// What travels over a stream channel.
///
/// Errors are *in-band* rather than a channel close because a partial
/// stream is a distinct, reportable outcome: a UI that cannot tell
/// "provider errored" from "provider finished" will show a truncated
/// review as a complete one.
pub type StreamItem = Result<StreamEvent>;

/// Receiving half of a stream channel.
pub type StreamReceiver = mpsc::Receiver<StreamItem>;

/// Sending half of a stream channel.
pub type StreamSender = mpsc::Sender<StreamItem>;

/// Open a bounded stream channel.
pub fn stream_channel() -> (StreamSender, StreamReceiver) {
    mpsc::channel(STREAM_CHANNEL_CAPACITY)
}

/// The error a provider returns when asked to stream but cannot.
///
/// Shared so the desktop layer and the CLI report the same wording
/// rather than each inventing one.
pub fn unsupported(provider: &str) -> GitGitError {
    GitGitError::Ai(format!(
        "{provider}: streaming not supported for this provider"
    ))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn data_of(fields: &[SseField]) -> Vec<String> {
        fields
            .iter()
            .filter_map(|f| match f {
                SseField::Data(d) => Some(d.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_complete_frame_decodes_to_its_data() {
        let mut d = SseDecoder::new();
        let out = d.push(b"data: {\"choices\":[]}\n\n");
        assert_eq!(
            out,
            vec![
                SseField::Data("{\"choices\":[]}".to_string()),
                SseField::Ignored
            ]
        );
    }

    #[test]
    fn a_fragmented_chunk_splitting_a_line_is_reassembled() {
        // The real failure this guards: a 6-byte TCP read landing in the
        // middle of a JSON payload. Emitting the fragment as a whole
        // frame would produce a JSON parse error on every chunk.
        let mut d = SseDecoder::new();
        let whole = b"data: {\"choices\":[{\"delta\":{\"content\":\"Hel\"}}]}\n\n";
        let mut fields = Vec::new();
        for piece in whole.chunks(6) {
            fields.extend(d.push(piece));
        }
        assert_eq!(
            data_of(&fields),
            vec!["{\"choices\":[{\"delta\":{\"content\":\"Hel\"}}]}"]
        );
    }

    #[test]
    fn bytes_before_a_newline_stay_buffered() {
        let mut d = SseDecoder::new();
        assert!(d.push(b"data: {\"a\":").is_empty());
        assert!(d.pending() > 0, "partial line must not be dropped");
        let out = d.push(b"1}\n");
        assert_eq!(data_of(&out), vec!["{\"a\":1}"]);
        assert_eq!(d.pending(), 0);
    }

    #[test]
    fn data_field_without_a_space_after_the_colon_is_accepted() {
        // Some OpenAI-compatible servers emit `data:{…}`. Requiring the
        // space would drop every frame from those endpoints.
        let mut d = SseDecoder::new();
        let out = d.push(b"data:{\"choices\":[]}\n");
        assert_eq!(data_of(&out), vec!["{\"choices\":[]}"]);
    }

    #[test]
    fn only_one_leading_space_is_stripped() {
        // The payload is ` {"x":1}` — a leading space is real content.
        let mut d = SseDecoder::new();
        let out = d.push(b"data:  {\"x\":1}\n");
        assert_eq!(data_of(&out), vec![" {\"x\":1}"]);
    }

    #[test]
    fn comment_and_non_data_fields_are_ignored() {
        let mut d = SseDecoder::new();
        let out = d.push(
            b": keep-alive\n\
               event: message\n\
               id: 42\n\
               retry: 1000\n\
               \n\
               data: real\n\
               some-unknown-field: x\n",
        );
        assert_eq!(
            data_of(&out),
            vec!["real"],
            "only the data field may reach the caller"
        );
        assert!(!out.contains(&SseField::Done));
    }

    #[test]
    fn crlf_line_endings_are_handled() {
        let mut d = SseDecoder::new();
        let out = d.push(b"data: one\r\ndata: two\r\n");
        assert_eq!(data_of(&out), vec!["one", "two"]);
    }

    #[test]
    fn done_terminator_is_recognised_in_both_spellings() {
        let mut d = SseDecoder::new();
        let out = d.push(b"data: [DONE]\ndata:[DONE]\n");
        assert_eq!(
            out,
            vec![SseField::Done, SseField::Done],
            "the no-space spelling must terminate too"
        );
    }

    #[test]
    fn a_payload_that_merely_contains_done_is_not_a_terminator() {
        // An exact match only: a model that reviews a diff containing
        // the literal text `[DONE]` must not cut its own stream short.
        let mut d = SseDecoder::new();
        let out = d.push(b"data: the review mentions [DONE] here\n");
        assert_eq!(data_of(&out), vec!["the review mentions [DONE] here"]);
    }

    #[test]
    fn invalid_utf8_degrades_instead_of_discarding_the_frame() {
        let mut d = SseDecoder::new();
        // Built by concatenation rather than as an array literal: the
        // literal would mix `&[u8; N]` and an integer literal, which does
        // not infer a single element type.
        let chunk: Vec<u8> = [b"data: a".as_slice(), &[0xff][..], b"b\n"].concat();
        let out = d.push(&chunk);
        let got = data_of(&out);
        assert_eq!(got.len(), 1, "frame must survive: {got:?}");
        assert!(
            got[0].starts_with('a') && got[0].ends_with('b'),
            "got: {got:?}"
        );
    }

    #[test]
    fn unsupported_names_the_provider() {
        let msg = unsupported("anthropic").to_string();
        assert!(msg.contains("anthropic"), "got: {msg}");
        assert!(
            msg.contains("streaming not supported"),
            "the refusal must be explicit, not a silent fallback: {msg}"
        );
    }
}
