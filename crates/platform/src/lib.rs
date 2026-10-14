//! Platform abstraction traits for ClipVault.
//!
//! OS-specific implementations live in `clipvault-platform-windows` and
//! `clipvault-platform-linux`. This crate only defines the contracts.

pub mod event;

use std::sync::mpsc::Sender;

use thiserror::Error;

pub use event::ClipboardEvent;

/// Errors produced by platform backends.
#[derive(Debug, Error)]
pub enum PlatformError {
    /// The platform backend is not available in this environment.
    #[error("platform backend unavailable: {0}")]
    Unavailable(String),
    /// An OS-level operation failed.
    #[error("platform operation failed: {0}")]
    Operation(String),
}

/// Watches the system clipboard for changes without polling as the primary mechanism.
pub trait ClipboardWatcher {
    /// Start watching and send captured events on `tx`.
    ///
    /// Implementations must prefer OS notifications over polling.
    fn start(&mut self, tx: Sender<ClipboardEvent>) -> Result<(), PlatformError>;

    /// Stop watching and release OS resources.
    fn stop(&mut self) -> Result<(), PlatformError>;
}

/// Manages login/session autostart for the daemon.
pub trait Autostart {
    /// Install autostart so the daemon runs in the user graphical session.
    fn install(&self) -> Result<(), PlatformError>;

    /// Remove previously installed autostart entries.
    fn uninstall(&self) -> Result<(), PlatformError>;

    /// Return whether autostart is currently installed.
    fn is_installed(&self) -> Result<bool, PlatformError>;
}

/// Resolves the application that produced a clipboard change, when the OS allows it.
pub trait SourceApp {
    /// Best-effort source application name or identifier.
    fn source_app(&self) -> Result<Option<String>, PlatformError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    struct NoopWatcher;

    impl ClipboardWatcher for NoopWatcher {
        fn start(&mut self, _tx: Sender<ClipboardEvent>) -> Result<(), PlatformError> {
            Ok(())
        }

        fn stop(&mut self) -> Result<(), PlatformError> {
            Ok(())
        }
    }

    #[test]
    fn noop_watcher_starts_and_stops() {
        let mut w = NoopWatcher;
        let (tx, _rx) = mpsc::channel();
        assert!(w.start(tx).is_ok());
        assert!(w.stop().is_ok());
    }
}
