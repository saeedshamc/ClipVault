//! Content hashing for clipboard payloads.

use crate::model::FormatPayload;

/// Compute a blake3 hex digest over the canonical bytes of all formats.
///
/// Formats are hashed in sorted MIME order so the same logical copy yields one
/// hash regardless of capture order.
pub fn content_hash(formats: &[FormatPayload]) -> String {
    let mut hasher = blake3::Hasher::new();
    let mut ordered: Vec<&FormatPayload> = formats.iter().collect();
    ordered.sort_by(|a, b| a.mime.cmp(&b.mime));

    for format in ordered {
        hasher.update(format.mime.as_bytes());
        hasher.update(&[0]);
        hasher.update(&format.size.to_le_bytes());
        hasher.update(&[0]);
        if let Some(data) = &format.inline_data {
            hasher.update(data);
        } else if let Some(path) = &format.blob_path {
            hasher.update(path.as_bytes());
        }
        hasher.update(&[0xff]);
    }

    hasher.finalize().to_hex().to_string()
}

/// Hash raw bytes (e.g. a single text capture) as blake3 hex.
pub fn hash_bytes(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::FormatPayload;

    #[test]
    fn order_independent_for_formats() {
        let a = vec![
            FormatPayload::text_plain("hi"),
            FormatPayload::text_html("<b>hi</b>"),
        ];
        let b = vec![
            FormatPayload::text_html("<b>hi</b>"),
            FormatPayload::text_plain("hi"),
        ];
        assert_eq!(content_hash(&a), content_hash(&b));
    }

    #[test]
    fn different_content_different_hash() {
        let a = vec![FormatPayload::text_plain("one")];
        let b = vec![FormatPayload::text_plain("two")];
        assert_ne!(content_hash(&a), content_hash(&b));
    }

    #[test]
    fn hash_bytes_stable() {
        assert_eq!(hash_bytes(b"abc"), hash_bytes(b"abc"));
        assert_ne!(hash_bytes(b"abc"), hash_bytes(b"abd"));
    }
}
