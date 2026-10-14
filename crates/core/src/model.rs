//! Domain model for clipboard history items.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// High-level classification of a clipboard capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    /// Plain text only.
    Text,
    /// HTML (may also carry plain text as a sibling format).
    Html,
    /// RTF payload.
    Rtf,
    /// Image bytes (PNG/JPEG/etc.).
    Image,
    /// List of file paths / URI list.
    Files,
    /// More than one unrelated rich format stored together.
    Mixed,
}

impl ItemKind {
    /// Stable string form persisted in SQLite.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Html => "html",
            Self::Rtf => "rtf",
            Self::Image => "image",
            Self::Files => "files",
            Self::Mixed => "mixed",
        }
    }

    /// Parse a persisted kind string.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "text" => Some(Self::Text),
            "html" => Some(Self::Html),
            "rtf" => Some(Self::Rtf),
            "image" => Some(Self::Image),
            "files" => Some(Self::Files),
            "mixed" => Some(Self::Mixed),
            _ => None,
        }
    }
}

/// One concrete representation of a clipboard item (e.g. `text/plain`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormatPayload {
    /// MIME-like type, e.g. `text/plain`, `text/html`, `image/png`.
    pub mime: String,
    /// Byte length of the payload.
    pub size: u64,
    /// Small payloads stored inline in the database.
    pub inline_data: Option<Vec<u8>>,
    /// Large payloads stored as content-addressed files (path relative to blobs dir).
    pub blob_path: Option<String>,
}

impl FormatPayload {
    /// Create an inline text format.
    pub fn text_plain(text: impl Into<String>) -> Self {
        let bytes = text.into().into_bytes();
        let size = bytes.len() as u64;
        Self {
            mime: "text/plain".into(),
            size,
            inline_data: Some(bytes),
            blob_path: None,
        }
    }

    /// Create an inline HTML format.
    pub fn text_html(html: impl Into<String>) -> Self {
        let bytes = html.into().into_bytes();
        let size = bytes.len() as u64;
        Self {
            mime: "text/html".into(),
            size,
            inline_data: Some(bytes),
            blob_path: None,
        }
    }
}

/// A single history entry that may contain multiple formats from one copy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipboardItem {
    /// Database primary key. `None` before insert.
    pub id: Option<i64>,
    /// Content hash (blake3 hex in later milestones; opaque string for storage).
    pub content_hash: String,
    /// Primary kind for filtering/UI.
    pub kind: ItemKind,
    pub created_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
    pub pinned: bool,
    /// Sum of format sizes in bytes.
    pub total_size: u64,
    /// Best-effort source application identifier.
    pub source_app: Option<String>,
    /// Text used for FTS indexing (usually plain text).
    pub searchable_text: String,
    /// All captured formats for faithful paste-back.
    pub formats: Vec<FormatPayload>,
}

/// Fields required to insert a new history item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewClipboardItem {
    pub content_hash: String,
    pub kind: ItemKind,
    pub pinned: bool,
    pub source_app: Option<String>,
    pub searchable_text: String,
    pub formats: Vec<FormatPayload>,
}

impl NewClipboardItem {
    /// Compute total size from formats.
    pub fn total_size(&self) -> u64 {
        self.formats.iter().map(|f| f.size).sum()
    }

    /// Basic validation before persistence.
    pub fn validate(&self) -> Result<(), String> {
        if self.content_hash.is_empty() {
            return Err("content_hash must not be empty".into());
        }
        if self.formats.is_empty() {
            return Err("at least one format is required".into());
        }
        for format in &self.formats {
            if format.mime.is_empty() {
                return Err("format mime must not be empty".into());
            }
            if format.inline_data.is_none() && format.blob_path.is_none() {
                return Err(format!(
                    "format {} needs inline_data or blob_path",
                    format.mime
                ));
            }
            if let Some(data) = &format.inline_data {
                if data.len() as u64 != format.size {
                    return Err(format!(
                        "format {} size {} does not match inline_data length {}",
                        format.mime,
                        format.size,
                        data.len()
                    ));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_kind_roundtrip() {
        for kind in [
            ItemKind::Text,
            ItemKind::Html,
            ItemKind::Rtf,
            ItemKind::Image,
            ItemKind::Files,
            ItemKind::Mixed,
        ] {
            assert_eq!(ItemKind::parse(kind.as_str()), Some(kind));
        }
    }

    #[test]
    fn new_item_requires_formats() {
        let item = NewClipboardItem {
            content_hash: "abc".into(),
            kind: ItemKind::Text,
            pinned: false,
            source_app: None,
            searchable_text: "hi".into(),
            formats: vec![],
        };
        assert!(item.validate().is_err());
    }

    #[test]
    fn text_plain_helper_sets_size() {
        let fmt = FormatPayload::text_plain("hello");
        assert_eq!(fmt.size, 5);
        assert_eq!(fmt.mime, "text/plain");
    }
}
