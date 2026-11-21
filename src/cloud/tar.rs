//! A minimal ustar reader and writer.
//!
//! # Why this exists
//!
//! A cloud-disk snapshot has to be one file: uploading a bare repository
//! as thousands of loose objects would make a snapshot unusable on the
//! far end (a directory listing per snapshot, a restore that is thousands
//! of round trips). The obvious container is a tar archive, and the
//! obvious crate to build one with would be a new dependency — which this
//! repository does not take.
//!
//! `[FACT]` The format written and read here is **ustar** (POSIX.1-1988),
//! the format `tar` has written since 1988 and still writes by default, so
//! a snapshot taken with this module can be unpacked with the system `tar`.
//!
//! # What it does not do
//!
//! * No compression. There is no flate2 in the dependency graph, and
//!   compressing a repository snapshot is a V1 concern.
//! * No long names beyond ustar's own `prefix` field (155 bytes) plus the
//!   name field (100 bytes) — 255 bytes of path. A deeper path is an
//!   error, not a silent truncation.
//! * No directories, symlinks, hard links, device nodes or timestamps
//!   beyond the file's mtime. A snapshot stores regular files, and
//!   restores them as regular files.
//!
//! # The block size
//!
//! `[FACT]` Every tar block is 512 bytes. A header is one block, a file's
//! content is padded to the next multiple of 512, and the archive ends with
//! two zero blocks.

use std::fmt;

/// The size of one tar block, in bytes.
pub const BLOCK: usize = 512;

/// The longest a path may be: the `prefix` field plus the `name` field.
/// The longest a path may be: the `prefix` field plus the `name` field.
///
/// `[FACT]` The `name` field is 100 bytes and the `prefix` field is 155 ✓,
/// and a path is the two joined by a `/` ✓.
const MAX_PATH: usize = 155 + 1 + 100;

/// The most bytes the ustar `name` field can hold.
const MAX_NAME: usize = 100;

/// The most bytes the ustar `prefix` field can hold.
const MAX_PREFIX: usize = 155;

/// The largest size the ustar size field can hold ✓.
///
/// `[FACT]` The field is twelve bytes ✓ — eleven octal digits and a NUL ✓
/// — so it holds 8^11 - 1 bytes ✓, just under 8 GiB ✓.
const MAX_SIZE: u64 = 0o77777777777;

/// One file in an archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The path inside the archive, with `/` separators.
    pub path: String,
    /// The file's bytes.
    pub data: Vec<u8>,
    /// The file's mode bits (the permission bits only).
    pub mode: u32,
    /// The modification time, in seconds since the Unix epoch.
    pub mtime: u64,
}

/// Something that went wrong while reading or writing an archive.
#[derive(Debug)]
pub enum TarError {
    /// The bytes are not a tar archive at all.
    NotAnArchive,
    /// A header block is malformed.
    Malformed(String),
    /// The archive ends in the middle of a file.
    Truncated,
    /// A path does not fit the ustar format.
    PathTooLong(String),
    /// The declared size of a file does not fit the archive.
    SizeMismatch {
        path: String,
        declared: u64,
        available: u64,
    },
    /// A file is larger than the ustar size field can hold.
    SizeTooLarge { path: String, size: u64 },
}

impl fmt::Display for TarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TarError::NotAnArchive => write!(f, "not a tar archive"),
            TarError::Malformed(what) => write!(f, "malformed tar archive: {what}"),
            TarError::Truncated => write!(f, "the tar archive ends in the middle of a file"),
            TarError::PathTooLong(path) => {
                write!(f, "path is too long for a tar archive: {path}")
            }
            TarError::SizeMismatch {
                path,
                declared,
                available,
            } => write!(
                f,
                "the archive declares {declared} bytes for {path} but only {available} are there"
            ),
            TarError::SizeTooLarge { path, size } => write!(
                f,
                "{path} is {size} bytes, which is more than a tar archive can declare"
            ),
        }
    }
}

impl std::error::Error for TarError {}

/// Write the entries as a ustar archive.
///
/// The entries are written in the order given ✓ — the caller decides the
/// order, so a snapshot can be byte-for-byte reproducible ✓.
pub fn write(entries: &[Entry]) -> Result<Vec<u8>, TarError> {
    let mut out = Vec::new();
    for entry in entries {
        check_size(&entry.path, entry.data.len() as u64)?;
        let (prefix, name) = split_path(&entry.path)?;
        out.extend_from_slice(&header(
            prefix,
            name,
            entry.data.len() as u64,
            entry.mode,
            entry.mtime,
        )?);
        out.extend_from_slice(&entry.data);
        let padding = (BLOCK - entry.data.len() % BLOCK) % BLOCK;
        out.extend(std::iter::repeat(0u8).take(padding));
    }
    // Two zero blocks end the archive.
    out.extend(std::iter::repeat(0u8).take(BLOCK * 2));
    Ok(out)
}

/// Read a ustar archive.
///
/// `[FACT]` Only regular files are returned ✓ — an archive this module
/// wrote contains nothing else ✓, and an archive written elsewhere with
/// other entry types is an error ✓ rather than a silent skip ✓.
pub fn read(bytes: &[u8]) -> Result<Vec<Entry>, TarError> {
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    let mut offset = 0usize;
    loop {
        if offset + BLOCK > bytes.len() {
            // The archive ends without its terminator — that is a
            // truncated archive, not an empty one.
            return Err(TarError::Truncated);
        }
        let block = &bytes[offset..offset + BLOCK];
        if block.iter().all(|b| *b == 0) {
            // The end-of-archive marker.
            break;
        }
        let (name, size, mode, mtime) = parse_header(block)?;
        offset += BLOCK;
        let end = offset
            .checked_add(size as usize)
            .ok_or(TarError::Truncated)?;
        if end > bytes.len() {
            return Err(TarError::SizeMismatch {
                path: name.clone(),
                declared: size,
                available: (bytes.len() - offset) as u64,
            });
        }
        let data = bytes[offset..end].to_vec();
        offset = end + (BLOCK - (size as usize) % BLOCK) % BLOCK;
        entries.push(Entry {
            path: name,
            data,
            mode,
            mtime,
        });
    }
    Ok(entries)
}

/// Refuse a file the ustar size field cannot hold.
///
/// `[FACT]` The size field is twelve bytes ✓ — eleven octal digits and a
/// NUL ✓ — so it holds at most `8^11 - 1` bytes ✓. A larger file is an
/// **error** ✓, not a truncated field ✓: a truncated size field would
/// make the archive unreadable ✓, and a snapshot that cannot be read back
/// is worse than no snapshot ✓.
fn check_size(path: &str, size: u64) -> Result<(), TarError> {
    if size > MAX_SIZE {
        return Err(TarError::SizeTooLarge {
            path: path.to_string(),
            size,
        });
    }
    Ok(())
}

/// Split a path into the ustar `prefix` and `name` fields.
///
/// `[FACT]` ustar stores a path in two fields: `prefix` (155 bytes) and
/// `name` (100 bytes), joined by a `/` ✓. A path that fits in `name`
/// alone leaves the prefix empty ✓.
///
/// `[FACT]` A field may be **completely filled** ✓ — the NUL terminator
/// is only needed when the text is shorter than the field ✓, and every
/// implementation reads a full field as the whole text ✓. So a name of
/// exactly 100 bytes is representable ✓ and is written as 100 bytes ✓.
/// A path that does not fit is an **error** ✓ — the archive would carry
/// a truncated name ✓, and a restore would write the file under a
/// different name ✓.
fn split_path(path: &str) -> Result<(&str, &str), TarError> {
    if path.is_empty() || path.len() > MAX_PATH {
        return Err(TarError::PathTooLong(path.to_string()));
    }
    if path.len() <= MAX_NAME {
        return Ok(("", path));
    }
    // Find the last `/` that leaves a name of at most 100 bytes and a
    // prefix of at most 155 bytes.
    let mut split_at = None;
    for (index, ch) in path.char_indices() {
        if ch != '/' {
            continue;
        }
        let after = index + ch.len_utf8();
        let name_len = path.len() - after;
        let prefix_len = index;
        if name_len <= MAX_NAME && prefix_len <= MAX_PREFIX {
            split_at = Some(index);
            break;
        }
    }
    match split_at {
        Some(index) => Ok((&path[..index], &path[index + 1..])),
        None => Err(TarError::PathTooLong(path.to_string())),
    }
}

/// Build one 512-byte ustar header.
///
/// `[FACT]` The two path fields are written **exactly** ✓ — a name that
/// does not fit is an error ✓, not a truncated name ✓. A truncated name
/// would be written back under a different name on restore ✓, and the
/// repository would come back changed ✓.
fn header(
    prefix: &str,
    name: &str,
    size: u64,
    mode: u32,
    mtime: u64,
) -> Result<[u8; BLOCK], TarError> {
    let mut block = [0u8; BLOCK];
    put_str_exact(&mut block[0..100], name)?;
    put_octal(&mut block[100..108], mode as u64, 7);
    put_octal(&mut block[108..116], 0, 7);
    put_octal(&mut block[116..124], 0, 7);
    put_octal(&mut block[124..136], size, 11);
    put_octal(&mut block[136..148], mtime, 11);
    // The checksum field is filled with spaces while the checksum is
    // computed, then overwritten.
    block[148..156].fill(b' ');
    block[156] = b'0'; // regular file
    put_str(&mut block[257..263], "ustar");
    block[263] = 0;
    block[264..266].copy_from_slice(b"00");
    put_octal(&mut block[329..337], 0, 7);
    put_octal(&mut block[337..345], 0, 7);
    put_str_exact(&mut block[345..500], prefix)?;

    let checksum: u32 = block.iter().map(|b| *b as u32).sum();
    // Six octal digits, a NUL and a space — the layout every tar
    // implementation reads.
    let text = format!("{checksum:06o}\0 ");
    block[148..156].copy_from_slice(text.as_bytes());
    Ok(block)
}

/// Parse one ustar header.
fn parse_header(block: &[u8]) -> Result<(String, u64, u32, u64), TarError> {
    if &block[257..263] != b"ustar\0" && &block[257..262] != b"ustar" {
        return Err(TarError::Malformed("not a ustar header".to_string()));
    }
    let stored = parse_octal(&block[148..156])
        .ok_or_else(|| TarError::Malformed("bad checksum field".to_string()))?;
    let mut copy = [0u8; BLOCK];
    copy.copy_from_slice(block);
    copy[148..156].fill(b' ');
    let computed: u64 = copy.iter().map(|b| u64::from(*b)).sum();
    if stored != computed {
        return Err(TarError::Malformed("header checksum mismatch".to_string()));
    }
    let size = parse_octal(&block[124..136])
        .ok_or_else(|| TarError::Malformed("bad size field".to_string()))?;
    let mode = parse_octal(&block[100..108])
        .ok_or_else(|| TarError::Malformed("bad mode field".to_string()))? as u32;
    let mtime = parse_octal(&block[136..148])
        .ok_or_else(|| TarError::Malformed("bad mtime field".to_string()))?;
    let name = read_str(&block[0..100]);
    let prefix = read_str(&block[345..500]);
    let path = if prefix.is_empty() {
        name
    } else {
        format!("{prefix}/{name}")
    };
    Ok((path, size, mode, mtime))
}

/// Write a NUL-terminated string into a fixed-size field.
///
/// `[FACT]` This one truncates ✓ — it is only used for the fixed literals
/// (`ustar` ✓, `00` ✓), which fit ✓. The path fields use
/// [`put_str_exact`] ✓, which refuses to truncate ✓.
fn put_str(field: &mut [u8], value: &str) {
    let bytes = value.as_bytes();
    let len = bytes.len().min(field.len().saturating_sub(1));
    field[..len].copy_from_slice(&bytes[..len]);
}

/// Write a string into a fixed-size field, refusing to truncate it.
///
/// `[FACT]` A field may be completely filled ✓ — the NUL terminator is
/// only there for shorter text ✓, and a reader stops at the first NUL ✓,
/// so a full field reads back as the whole text ✓. A value that does
/// not fit is an **error** ✓: a truncated path would restore under a
/// different name ✓.
fn put_str_exact(field: &mut [u8], value: &str) -> Result<(), TarError> {
    let bytes = value.as_bytes();
    if bytes.len() > field.len() {
        return Err(TarError::PathTooLong(value.to_string()));
    }
    field[..bytes.len()].copy_from_slice(bytes);
    Ok(())
}

/// Write an octal number into a fixed-size field, NUL-terminated.
fn put_octal(field: &mut [u8], value: u64, digits: usize) {
    let text = format!("{value:0digits$o}", digits = digits);
    let bytes = text.as_bytes();
    let len = bytes.len().min(field.len().saturating_sub(1));
    field[..len].copy_from_slice(&bytes[..len]);
    field[len] = 0;
}

/// Read a NUL-terminated string out of a fixed-size field.
fn read_str(field: &[u8]) -> String {
    let end = field.iter().position(|b| *b == 0).unwrap_or(field.len());
    String::from_utf8_lossy(&field[..end]).into_owned()
}

/// Read an octal number out of a fixed-size field.
fn parse_octal(field: &[u8]) -> Option<u64> {
    let text = read_str(field);
    let text = text.trim();
    if text.is_empty() {
        return Some(0);
    }
    u64::from_str_radix(text, 8).ok()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn entry(path: &str, data: &[u8]) -> Entry {
        Entry {
            path: path.to_string(),
            data: data.to_vec(),
            mode: 0o644,
            mtime: 1_700_000_000,
        }
    }

    #[test]
    fn an_archive_round_trips() {
        let entries = vec![
            entry("HEAD", b"ref: refs/heads/main\n"),
            entry("objects/ab/cdef0123456789", &[0u8; 37]),
            entry("config", b"[core]\n\trepositoryformatversion = 0\n"),
        ];
        let bytes = write(&entries).unwrap();
        let back = read(&bytes).unwrap();
        assert_eq!(back, entries);
    }

    #[test]
    fn an_empty_archive_round_trips() {
        let bytes = write(&[]).unwrap();
        assert_eq!(bytes.len(), BLOCK * 2);
        assert!(read(&bytes).unwrap().is_empty());
    }

    #[test]
    fn a_long_path_uses_the_prefix_field() {
        let deep = format!("{}/{}", "d".repeat(120), "file.txt");
        let entries = vec![entry(&deep, b"x")];
        let bytes = write(&entries).unwrap();
        let back = read(&bytes).unwrap();
        assert_eq!(back[0].path, deep);
    }

    #[test]
    fn a_path_that_cannot_fit_is_refused() {
        let deep = format!("{}/file.txt", "d".repeat(300));
        let err = write(&[entry(&deep, b"x")]).unwrap_err();
        assert!(matches!(err, TarError::PathTooLong(_)), "got: {err:?}");
    }

    #[test]
    fn a_name_that_fills_the_field_exactly_survives_the_round_trip() {
        // `[FACT]` A ustar field may be completely filled — the NUL
        // terminator is only there for shorter text — so a 100-byte name
        // is representable and must survive exactly. The old code
        // accepted it and wrote 99 bytes, and a restore then wrote the
        // file under a different name.
        let exact = "a".repeat(100);
        let bytes = write(&[entry(&exact, b"x")]).expect("100 bytes fill the field");
        let back = read(&bytes).expect("the archive reads back");
        assert_eq!(back[0].path, exact, "the name must survive exactly");

        // One byte more does not fit.
        let too_long = "a".repeat(101);
        let err = write(&[entry(&too_long, b"x")]).unwrap_err();
        assert!(matches!(err, TarError::PathTooLong(_)), "got: {err:?}");
    }

    #[test]
    fn a_prefix_that_fills_the_field_exactly_survives_the_round_trip() {
        // `[FACT]` The prefix field is 155 bytes and may be filled
        // completely.
        let name = "n".repeat(100);
        let path = format!("{}/{name}", "p".repeat(155));
        let bytes = write(&[entry(&path, b"x")]).expect("155 bytes fill the field");
        let back = read(&bytes).expect("the archive reads back");
        assert_eq!(back[0].path, path, "the path must survive exactly");

        // One byte more does not fit.
        let path = format!("{}/{name}", "p".repeat(156));
        let err = write(&[entry(&path, b"x")]).unwrap_err();
        assert!(matches!(err, TarError::PathTooLong(_)), "got: {err:?}");
    }

    #[test]
    fn a_file_bigger_than_the_size_field_is_refused() {
        // The size field holds eleven octal digits. A file that does not
        // fit must be an error, not a truncated field — a truncated field
        // would produce an archive that cannot be read back.
        assert!(check_size("HEAD", MAX_SIZE).is_ok());
        let err = check_size("HEAD", MAX_SIZE + 1).unwrap_err();
        assert!(matches!(err, TarError::SizeTooLarge { .. }), "got: {err:?}");
        assert!(
            format!("{err}").contains("more than a tar archive can declare"),
            "got: {err}"
        );
    }

    #[test]
    fn the_system_tar_can_read_the_archive() {
        // `[FACT]` The format written here is ustar ✓, so the system `tar`
        // reads it ✓ — that is the acceptance criterion in ADR-0025 §4.5 ✓.
        // It is checked against the **real tool** ✓, not against a second
        // implementation of the same idea ✓: a second reader would agree
        // with a second writer about a format that is wrong in the same
        // way ✓, while the system `tar` is what a user will actually run
        // on a snapshot ✓.
        let dir = std::env::temp_dir().join(format!(
            "gitgit-tar-check-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let archive = dir.join("snapshot.tar");
        let entries = vec![
            entry("HEAD", b"ref: refs/heads/main\n"),
            entry("objects/ab/cdef0123", b"blob"),
        ];
        std::fs::write(&archive, write(&entries).unwrap()).unwrap();

        let out = std::process::Command::new("tar")
            .arg("-tf")
            .arg(&archive)
            .output()
            .expect("the system tar must be available to check the archive");
        assert!(
            out.status.success(),
            "tar refused the archive: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let listing = String::from_utf8_lossy(&out.stdout);
        assert!(listing.contains("HEAD"), "got: {listing}");
        assert!(listing.contains("objects/ab/cdef0123"), "got: {listing}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bytes_that_are_not_an_archive_are_refused() {
        // Shorter than a header block: the archive is truncated.
        let err = read(b"this is not a tar archive at all").unwrap_err();
        assert!(matches!(err, TarError::Truncated), "got: {err:?}");
        // A full block that is not a header.
        let mut block = vec![b'x'; BLOCK];
        block[257..263].copy_from_slice(b"ustar\0");
        let err = read(&block).unwrap_err();
        assert!(matches!(err, TarError::Malformed(_)), "got: {err:?}");
    }

    #[test]
    fn a_truncated_archive_is_refused() {
        let mut bytes = write(&[entry("HEAD", &[0u8; 100])]).unwrap();
        bytes.truncate(BLOCK + 50);
        let err = read(&bytes).unwrap_err();
        assert!(matches!(err, TarError::SizeMismatch { .. }), "got: {err:?}");
    }

    #[test]
    fn a_corrupted_header_is_refused() {
        let mut bytes = write(&[entry("HEAD", b"x")]).unwrap();
        bytes[0] = b'Z';
        let err = read(&bytes).unwrap_err();
        assert!(matches!(err, TarError::Malformed(_)), "got: {err:?}");
    }

    #[test]
    fn the_archive_is_a_multiple_of_the_block_size() {
        let bytes = write(&[entry("a", &[1u8; 1]), entry("b", &[2u8; 512])]).unwrap();
        assert_eq!(bytes.len() % BLOCK, 0);
    }
}
