//! Shared daemon state and request handling.

use std::sync::{Arc, Mutex};

use clipvault_core::ipc::{
    Command, ItemSummary, Request, Response, ResponseResult, PROTOCOL_VERSION,
};
use clipvault_core::model::{ItemKind, NewClipboardItem};
use clipvault_core::{CaptureOutcome, Config, Store};
use clipvault_platform::ClipboardEvent;

/// Mutable runtime flags shared between watcher and IPC.
#[derive(Debug, Default)]
pub struct RuntimeState {
    pub paused: bool,
}

/// Process a single IPC request against the store / runtime flags.
pub fn handle_request(
    store: &Store,
    _config: &Config,
    runtime: &Mutex<RuntimeState>,
    req: Request,
) -> Response {
    if req.v != PROTOCOL_VERSION {
        return Response::err(
            req.id,
            format!(
                "unsupported protocol version {}; server speaks {PROTOCOL_VERSION}",
                req.v
            ),
        );
    }

    let result = match req.cmd {
        Command::Ping => Ok(ResponseResult::Pong {
            version: clipvault_core::version().to_string(),
        }),
        Command::List { limit, offset } => {
            store
                .list(limit, offset)
                .map(|items| ResponseResult::Items {
                    items: items.iter().map(ItemSummary::from_item).collect(),
                })
        }
        Command::Search { query, limit } => {
            store
                .search(&query, limit)
                .map(|items| ResponseResult::Items {
                    items: items.iter().map(ItemSummary::from_item).collect(),
                })
        }
        Command::Delete { id } => store.delete(id).map(|()| ResponseResult::Deleted { id }),
        Command::Clear => store.clear().map(|deleted| ResponseResult::Cleared {
            deleted: deleted as u64,
        }),
        Command::Pause => {
            if let Ok(mut rt) = runtime.lock() {
                rt.paused = true;
            }
            Ok(ResponseResult::Empty)
        }
        Command::Resume => {
            if let Ok(mut rt) = runtime.lock() {
                rt.paused = false;
            }
            Ok(ResponseResult::Empty)
        }
        Command::Status => {
            let paused = runtime.lock().map(|rt| rt.paused).unwrap_or(false);
            match (store.count(), store.total_size()) {
                (Ok(item_count), Ok(total_bytes)) => Ok(ResponseResult::Status {
                    paused,
                    item_count,
                    total_bytes,
                }),
                (Err(err), _) | (_, Err(err)) => Err(err),
            }
        }
    };

    match result {
        Ok(result) => Response::ok(req.id, result),
        Err(err) => Response::err(req.id, err.to_string()),
    }
}

/// Persist a clipboard event unless capture is paused.
pub fn ingest_event(
    store: &Store,
    config: &Config,
    runtime: &Arc<Mutex<RuntimeState>>,
    event: ClipboardEvent,
) {
    let paused = runtime
        .lock()
        .map(|rt| rt.paused || config.watcher.paused)
        .unwrap_or(true);
    if paused {
        tracing::debug!("capture paused; ignoring clipboard event");
        return;
    }

    let kind = if event.formats.len() > 1 {
        ItemKind::Mixed
    } else {
        ItemKind::Text
    };

    let new_item = NewClipboardItem {
        content_hash: String::new(),
        kind,
        pinned: false,
        source_app: event.source_app,
        searchable_text: event.searchable_text,
        formats: event.formats,
    };

    match store.capture(new_item, config) {
        Ok(CaptureOutcome::Inserted { id }) => {
            tracing::info!(id, "captured clipboard item");
            if let Err(err) = store.apply_retention(config) {
                tracing::warn!(error = %err, "retention pass failed");
            }
        }
        Ok(CaptureOutcome::Deduped { id }) => {
            tracing::debug!(id, "deduped clipboard item");
        }
        Ok(CaptureOutcome::SkippedOversized {
            size,
            limit,
            hash_prefix,
        }) => {
            tracing::warn!(size, limit, hash_prefix = %hash_prefix, "skipped oversized item");
        }
        Err(err) => tracing::error!(error = %err, "failed to capture clipboard item"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clipvault_core::ipc::{Command, ResponseBody, ResponseResult};
    use clipvault_core::model::FormatPayload;

    #[test]
    fn pause_and_status_roundtrip() {
        let store = Store::open_in_memory().expect("store");
        let config = Config::default();
        let runtime = Mutex::new(RuntimeState::default());

        let pause = handle_request(&store, &config, &runtime, Request::new(1, Command::Pause));
        assert!(matches!(
            pause.body,
            ResponseBody::Ok {
                result: ResponseResult::Empty
            }
        ));

        let status = handle_request(&store, &config, &runtime, Request::new(2, Command::Status));
        match status.body {
            ResponseBody::Ok {
                result:
                    ResponseResult::Status {
                        paused, item_count, ..
                    },
            } => {
                assert!(paused);
                assert_eq!(item_count, 0);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn ingest_respects_pause() {
        let store = Store::open_in_memory().expect("store");
        let config = Config::default();
        let runtime = Arc::new(Mutex::new(RuntimeState { paused: true }));
        let event = ClipboardEvent {
            formats: vec![FormatPayload::text_plain("secret")],
            searchable_text: "secret".into(),
            source_app: None,
        };
        ingest_event(&store, &config, &runtime, event);
        assert_eq!(store.count().expect("count"), 0);
    }

    #[test]
    fn list_after_ingest() {
        let store = Store::open_in_memory().expect("store");
        let config = Config::default();
        let runtime = Arc::new(Mutex::new(RuntimeState::default()));
        let event = ClipboardEvent {
            formats: vec![FormatPayload::text_plain("hello ipc")],
            searchable_text: "hello ipc".into(),
            source_app: None,
        };
        ingest_event(&store, &config, &runtime, event);
        let resp = handle_request(
            &store,
            &config,
            &runtime,
            Request::new(
                1,
                Command::List {
                    limit: 10,
                    offset: 0,
                },
            ),
        );
        match resp.body {
            ResponseBody::Ok {
                result: ResponseResult::Items { items },
            } => {
                assert_eq!(items.len(), 1);
                assert!(items[0].searchable_text.contains("hello"));
            }
            other => panic!("unexpected {other:?}"),
        }
    }
}
