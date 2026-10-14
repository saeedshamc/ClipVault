//! Clipboard change events emitted by platform watchers.

use clipvault_core::model::FormatPayload;

/// A captured clipboard change ready for persistence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardEvent {
    pub formats: Vec<FormatPayload>,
    pub searchable_text: String,
    pub source_app: Option<String>,
}
