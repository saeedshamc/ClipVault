//! Platform-independent ClipVault core: models, storage, config, and IPC protocol.

pub mod config;
pub mod error;
pub mod hash;
pub mod model;
pub mod retention;
pub mod storage;

pub use config::{AppPaths, Config, LimitsConfig, RetentionConfig, StorageConfig, WatcherConfig};
pub use error::{CoreError, CoreResult};
pub use hash::{content_hash, hash_bytes};
pub use model::{ClipboardItem, FormatPayload, ItemKind, NewClipboardItem};
pub use retention::{evaluate_pressure, exceeds_item_limit, RetentionPressure, RetentionSnapshot};
pub use storage::{CaptureOutcome, RetentionReport, Store, SCHEMA_VERSION};

/// Crate version inherited from the workspace package version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Returns the ClipVault core library version string.
pub fn version() -> &'static str {
    VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_nonempty() {
        assert!(!version().is_empty());
    }
}
