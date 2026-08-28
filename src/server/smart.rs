//! Smart-HTTP protocol helpers for `git upload-pack` / `git receive-pack`.
//!
//! The wire format is the same one `git http-backend` (CGI) emits:
//!
//! ```text
//! 001e# service=git-upload-pack\n
//! 0000
//! <raw pkt-line stream from `git upload-pack --stateless-rpc --advertise-refs`>
//! ```
//!
//! The leading four bytes are a **hex** length prefix: 0x30 0x30 0x31 0x65 is
//! the string "001e" (decimal 30), which counts the bytes that follow
//! including the trailing newline.
//!
//! On the POST side, the request body is fed straight to the git subprocess
//! and the subprocess stdout is the response body. The git client knows
//! how to parse both because it speaks the same `pkt-line` framing.

/// Build the announcement frame for a smart-HTTP GET.
///
/// Example output for `git-upload-pack`:
///
/// ```text
/// 001e# service=git-upload-pack\n
/// 0000
/// ```
pub fn announce_frame(service: &str) -> Vec<u8> {
    // Length prefix includes the leading '#' line but not the trailing NUL.
    // Format: "001e# service=<svc>\n" (4 hex + 1 '#' + 1 ' ' + "service=" + svc + '\n')
    let prefix = format!("# service={service}\n");
    let total = 4 + prefix.len();
    let header = format!("{:04x}{}", total, prefix);
    let mut out = Vec::with_capacity(header.len() + 5);
    out.extend_from_slice(header.as_bytes());
    out.extend_from_slice(b"0000");
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// Helper: parse the first 4 bytes as a hex length prefix and return
    /// (prefix_len, body_len) where body_len is the number of bytes that
    /// follow the 4-byte prefix within the framed line.
    fn parse_prefix(frame: &[u8]) -> (usize, usize) {
        let len_hex = std::str::from_utf8(&frame[0..4]).unwrap();
        let total = usize::from_str_radix(len_hex, 16).unwrap();
        (4, total - 4)
    }

    #[test]
    fn announce_frame_format() {
        let f = announce_frame("git-upload-pack");
        // First 4 bytes are the hex length of the line that follows (including the 4-byte prefix itself).
        let len: usize = usize::from_str_radix(
            std::str::from_utf8(&f[0..4]).unwrap(),
            16,
        )
        .unwrap();
        assert_eq!(len, 4 + "# service=git-upload-pack\n".len());
        // The body of the line begins with "# service=git-upload-pack\n".
        let line = std::str::from_utf8(&f[4..len]).unwrap();
        assert_eq!(line, "# service=git-upload-pack\n");
        // Then a flush packet.
        assert_eq!(&f[len..len + 4], b"0000");
    }

    #[test]
    fn announce_frame_for_receive_pack() {
        // Both services must be supported. The body line differs only in
        // the service name; framing rules are identical.
        let f = announce_frame("git-receive-pack");
        let (_, body_len) = parse_prefix(&f);
        let body = std::str::from_utf8(&f[4..4 + body_len]).unwrap();
        assert_eq!(body, "# service=git-receive-pack\n");
        // Total length must equal 4 (prefix) + body + 4 (flush packet).
        assert_eq!(f.len(), 4 + body_len + 4);
        // Flush packet is always exactly "0000".
        assert_eq!(&f[4 + body_len..], b"0000");
    }

    #[test]
    fn announce_frame_total_size_is_deterministic() {
        // Lock down the exact byte length so a future refactor that
        // accidentally drops the trailing NUL flush packet (or changes
        // the prefix width) is caught by a unit test.
        let f = announce_frame("git-upload-pack");
        // "# service=git-upload-pack\n" is 26 bytes; prefix=4, flush=4.
        assert_eq!(f.len(), 4 + 26 + 4);
    }

    #[test]
    fn announce_frame_flush_packet_is_4_zero_bytes() {
        // The flush packet "0000" is 4 ASCII zeros, not 4 NUL bytes. The
        // wire format is hex ASCII.
        let f = announce_frame("git-upload-pack");
        let (_, body_len) = parse_prefix(&f);
        let flush = &f[4 + body_len..4 + body_len + 4];
        assert_eq!(flush, b"0000");
        // Specifically: not the NUL byte (0x00) repeated.
        assert_ne!(flush, &[0u8, 0, 0, 0]);
    }
}
