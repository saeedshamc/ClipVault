//! Versioned JSON IPC protocol between daemon and clients.

use serde::{Deserialize, Serialize};

use crate::model::{ClipboardItem, ItemKind};

/// Protocol version supported by this build.
pub const PROTOCOL_VERSION: u32 = 1;

/// Request envelope sent by CLI/UI clients.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    pub v: u32,
    pub id: u64,
    #[serde(flatten)]
    pub cmd: Command,
}

/// Response envelope returned by the daemon.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Response {
    pub v: u32,
    pub id: u64,
    #[serde(flatten)]
    pub body: ResponseBody,
}

/// Command payloads (serde externally tagged via `cmd` field).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Command {
    Ping,
    List {
        #[serde(default = "default_limit")]
        limit: u32,
        #[serde(default)]
        offset: u32,
    },
    Search {
        query: String,
        #[serde(default = "default_limit")]
        limit: u32,
    },
    Delete {
        id: i64,
    },
    Clear,
    Pause,
    Resume,
    Status,
}

fn default_limit() -> u32 {
    50
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResponseBody {
    Ok { result: ResponseResult },
    Err { error: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseResult {
    Pong {
        version: String,
    },
    Items {
        items: Vec<ItemSummary>,
    },
    Cleared {
        deleted: u64,
    },
    Deleted {
        id: i64,
    },
    Status {
        paused: bool,
        item_count: u64,
        total_bytes: u64,
    },
    Empty,
}

/// Compact item view for IPC list/search (never includes full binary blobs).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemSummary {
    pub id: i64,
    pub kind: ItemKind,
    pub pinned: bool,
    pub total_size: u64,
    pub content_hash_prefix: String,
    pub searchable_text: String,
    pub source_app: Option<String>,
    pub created_at: String,
    pub last_used_at: String,
}

impl ItemSummary {
    pub fn from_item(item: &ClipboardItem) -> Self {
        Self {
            id: item.id.unwrap_or_default(),
            kind: item.kind,
            pinned: item.pinned,
            total_size: item.total_size,
            content_hash_prefix: item.content_hash.chars().take(12).collect(),
            searchable_text: truncate_chars(&item.searchable_text, 200),
            source_app: item.source_app.clone(),
            created_at: item.created_at.to_rfc3339(),
            last_used_at: item.last_used_at.to_rfc3339(),
        }
    }
}

fn truncate_chars(input: &str, max: usize) -> String {
    if input.chars().count() <= max {
        input.to_string()
    } else {
        let mut out: String = input.chars().take(max).collect();
        out.push('…');
        out
    }
}

impl Request {
    pub fn new(id: u64, cmd: Command) -> Self {
        Self {
            v: PROTOCOL_VERSION,
            id,
            cmd,
        }
    }
}

impl Response {
    pub fn ok(id: u64, result: ResponseResult) -> Self {
        Self {
            v: PROTOCOL_VERSION,
            id,
            body: ResponseBody::Ok { result },
        }
    }

    pub fn err(id: u64, error: impl Into<String>) -> Self {
        Self {
            v: PROTOCOL_VERSION,
            id,
            body: ResponseBody::Err {
                error: error.into(),
            },
        }
    }
}

/// Encode one length-prefixed JSON message (`u32` LE + UTF-8 JSON).
pub fn encode_message<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    let json = serde_json::to_vec(value).map_err(|err| err.to_string())?;
    if json.len() > u32::MAX as usize {
        return Err("IPC message too large".into());
    }
    let mut out = Vec::with_capacity(4 + json.len());
    out.extend_from_slice(&(json.len() as u32).to_le_bytes());
    out.extend_from_slice(&json);
    Ok(out)
}

/// Decode one length-prefixed JSON message from a complete buffer.
pub fn decode_message<T: for<'de> Deserialize<'de>>(buf: &[u8]) -> Result<(T, usize), String> {
    if buf.len() < 4 {
        return Err("incomplete IPC header".into());
    }
    let len = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    let total = 4 + len;
    if buf.len() < total {
        return Err("incomplete IPC payload".into());
    }
    let value = serde_json::from_slice(&buf[4..total]).map_err(|err| err.to_string())?;
    Ok((value, total))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip() {
        let req = Request::new(
            7,
            Command::Search {
                query: "hello".into(),
                limit: 10,
            },
        );
        let bytes = encode_message(&req).expect("encode");
        let (decoded, n): (Request, usize) = decode_message(&bytes).expect("decode");
        assert_eq!(n, bytes.len());
        assert_eq!(decoded, req);
    }

    #[test]
    fn response_error_roundtrip() {
        let resp = Response::err(3, "boom");
        let bytes = encode_message(&resp).expect("encode");
        let (decoded, _): (Response, usize) = decode_message(&bytes).expect("decode");
        assert_eq!(decoded, resp);
    }
}
