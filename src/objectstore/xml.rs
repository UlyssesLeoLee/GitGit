//! XML rendering for the S3 responses the object store produces.
//!
//! [FACT] The client (`rust-s3` 0.38) parses a listing with
//! `quick-xml`, and the shape it needs is narrow but exact:
//!
//! * `ListBucketResult/Name` is **required** — without it the client
//!   cannot deserialize the document at all;
//! * `ListBucketResult/Contents/Key`, `.../LastModified` and `.../Size`
//!   are required for every entry;
//! * `ETag`, `StorageClass` and `Owner` are optional.
//!
//! [FACT] The client's `Bucket::list` loop ends when a page carries no
//! `NextContinuationToken`, so that element must only be present when the
//! listing really is truncated — otherwise the client loops forever.

use crate::objectstore::store::{ListEntry, VersioningStatus};

/// The XML namespace S3 uses for its documents.
const S3_XMLNS: &str = "http://s3.amazonaws.com/doc/2006-03-01/";

/// How much of a listing document to render.
///
/// [PROPOSAL] The broken shape exists so that a test can prove the failure
/// mode "a malformed listing must make the client error out" instead of
/// assuming it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListingShape {
    /// The complete, correct document.
    Full,
    /// A document with the required `Name` element left out.
    MissingName,
}

/// Render a `ListObjectsV2` response.
pub fn render_list_bucket(
    bucket: &str,
    prefix: &str,
    entries: &[ListEntry],
    is_truncated: bool,
    next_continuation_token: Option<&str>,
    max_keys: usize,
    shape: ListingShape,
) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(&format!("<ListBucketResult xmlns=\"{S3_XMLNS}\">\n"));
    if shape != ListingShape::MissingName {
        out.push_str(&format!("  <Name>{}</Name>\n", escape_xml(bucket)));
    }
    out.push_str(&format!("  <Prefix>{}</Prefix>\n", escape_xml(prefix)));
    out.push_str(&format!("  <KeyCount>{}</KeyCount>\n", entries.len()));
    out.push_str(&format!("  <MaxKeys>{max_keys}</MaxKeys>\n"));
    out.push_str(&format!("  <IsTruncated>{is_truncated}</IsTruncated>\n"));
    if let Some(token) = next_continuation_token {
        out.push_str(&format!(
            "  <NextContinuationToken>{}</NextContinuationToken>\n",
            escape_xml(token)
        ));
    }
    for entry in entries {
        out.push_str("  <Contents>\n");
        out.push_str(&format!("    <Key>{}</Key>\n", escape_xml(&entry.key)));
        out.push_str(&format!(
            "    <LastModified>{}</LastModified>\n",
            escape_xml(&entry.last_modified)
        ));
        out.push_str(&format!(
            "    <ETag>\"{}\"</ETag>\n",
            escape_xml(&entry.etag)
        ));
        out.push_str(&format!("    <Size>{}</Size>\n", entry.size));
        out.push_str("    <StorageClass>STANDARD</StorageClass>\n");
        out.push_str("  </Contents>\n");
    }
    out.push_str("</ListBucketResult>\n");
    out
}

/// Render a `GetBucketVersioning` response.
pub fn render_versioning(status: VersioningStatus) -> String {
    let status = match status {
        VersioningStatus::Enabled => "Enabled",
        VersioningStatus::Disabled => "Disabled",
    };
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<VersioningConfiguration xmlns=\"{S3_XMLNS}\">\n  <Status>{status}</Status>\n</VersioningConfiguration>\n"
    )
}

/// Render an S3 error document.
///
/// [FACT] The client's `classify_s3_read` decides "the credential is
/// absent" from a 404 whose `<Code>` is exactly `NoSuchKey`; a 404 with
/// any other code means "the deployment is broken". The code therefore
/// carries meaning and must not be generic.
pub fn render_error(code: &str, message: &str, detail: Option<&str>) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<Error>\n");
    out.push_str(&format!("  <Code>{}</Code>\n", escape_xml(code)));
    out.push_str(&format!("  <Message>{}</Message>\n", escape_xml(message)));
    if let Some(detail) = detail {
        out.push_str(&format!("  <Detail>{}</Detail>\n", escape_xml(detail)));
    }
    out.push_str("</Error>\n");
    out
}

/// Escape the five XML entities.
pub fn escape_xml(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn the_listing_carries_the_elements_the_client_requires() {
        let entries = vec![ListEntry {
            key: "openai".to_string(),
            size: 12,
            etag: "abc".to_string(),
            last_modified: "2026-10-25T13:35:21.727000Z".to_string(),
        }];
        let xml = render_list_bucket(
            "gitgit-vault",
            "gitgit-vault/",
            &entries,
            false,
            None,
            1000,
            ListingShape::Full,
        );
        assert!(xml.contains("<Name>gitgit-vault</Name>"));
        assert!(xml.contains("<Key>openai</Key>"));
        assert!(xml.contains("<Size>12</Size>"));
        assert!(xml.contains("<LastModified>2026-10-25T13:35:21.727000Z</LastModified>"));
        assert!(xml.contains("<IsTruncated>false</IsTruncated>"));
        // No token unless the listing is truncated: the client loops until
        // the token is gone.
        assert!(!xml.contains("NextContinuationToken"));
    }

    #[test]
    fn a_truncated_listing_carries_the_next_token() {
        let xml = render_list_bucket("b", "", &[], true, Some("dG9rZW4"), 1, ListingShape::Full);
        assert!(xml.contains("<IsTruncated>true</IsTruncated>"));
        assert!(xml.contains("<NextContinuationToken>dG9rZW4</NextContinuationToken>"));
    }

    #[test]
    fn the_broken_listing_is_missing_the_required_name() {
        let xml = render_list_bucket("b", "", &[], false, None, 1000, ListingShape::MissingName);
        assert!(!xml.contains("<Name>"));
    }

    #[test]
    fn keys_are_xml_escaped() {
        let entries = vec![ListEntry {
            key: "a&b<c>d\"e'f".to_string(),
            size: 0,
            etag: "x".to_string(),
            last_modified: "2026-10-25T13:35:21.727000Z".to_string(),
        }];
        let xml = render_list_bucket("b", "", &entries, false, None, 1000, ListingShape::Full);
        assert!(xml.contains("a&amp;b&lt;c&gt;d&quot;e&apos;f"));
    }

    #[test]
    fn the_error_document_carries_the_code_the_client_reads() {
        let xml = render_error("NoSuchKey", "The specified key does not exist.", None);
        assert!(xml.contains("<Code>NoSuchKey</Code>"));
        assert!(xml.contains("<Message>The specified key does not exist.</Message>"));
    }

    #[test]
    fn the_versioning_document_names_the_status() {
        assert!(render_versioning(VersioningStatus::Enabled).contains("<Status>Enabled</Status>"));
        assert!(render_versioning(VersioningStatus::Disabled).contains("<Status>Disabled</Status>"));
    }
}
