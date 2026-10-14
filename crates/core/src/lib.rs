//! Platform-independent ClipVault core: models, storage, config, and IPC protocol.
//!
//! Concrete storage and IPC land in later milestones; this crate starts as the
//! shared foundation that every other crate depends on.

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
