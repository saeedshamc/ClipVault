//! Linux-specific platform backends for ClipVault.
//!
//! X11 (XFixes), Wayland (`wlr-data-control`), XDG autostart, and systemd user
//! units are implemented in later milestones. This crate currently exposes stubs
//! so the workspace builds on every host.

use std::sync::mpsc::Sender;

use clipvault_platform::{Autostart, ClipboardEvent, ClipboardWatcher, PlatformError};

/// Placeholder Linux clipboard watcher.
#[derive(Debug, Default)]
pub struct LinuxClipboardWatcher {
    running: bool,
}

impl ClipboardWatcher for LinuxClipboardWatcher {
    fn start(&mut self, _tx: Sender<ClipboardEvent>) -> Result<(), PlatformError> {
        if cfg!(target_os = "linux") {
            self.running = true;
            Ok(())
        } else {
            Err(PlatformError::Unavailable(
                "Linux clipboard watcher requires a Linux host".into(),
            ))
        }
    }

    fn stop(&mut self) -> Result<(), PlatformError> {
        self.running = false;
        Ok(())
    }
}

impl LinuxClipboardWatcher {
    /// Returns whether [`ClipboardWatcher::start`] has been called successfully.
    pub fn is_running(&self) -> bool {
        self.running
    }
}

/// Placeholder Linux autostart manager (XDG desktop entry / systemd user unit later).
#[derive(Debug, Default)]
pub struct LinuxAutostart;

impl Autostart for LinuxAutostart {
    fn install(&self) -> Result<(), PlatformError> {
        if cfg!(target_os = "linux") {
            Err(PlatformError::Operation(
                "Linux autostart is not implemented yet".into(),
            ))
        } else {
            Err(PlatformError::Unavailable(
                "Linux autostart requires a Linux host".into(),
            ))
        }
    }

    fn uninstall(&self) -> Result<(), PlatformError> {
        if cfg!(target_os = "linux") {
            Err(PlatformError::Operation(
                "Linux autostart is not implemented yet".into(),
            ))
        } else {
            Err(PlatformError::Unavailable(
                "Linux autostart requires a Linux host".into(),
            ))
        }
    }

    fn is_installed(&self) -> Result<bool, PlatformError> {
        if cfg!(target_os = "linux") {
            Ok(false)
        } else {
            Err(PlatformError::Unavailable(
                "Linux autostart requires a Linux host".into(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn watcher_start_respects_target_os() {
        let mut watcher = LinuxClipboardWatcher::default();
        let (tx, _rx) = mpsc::channel();
        let result = watcher.start(tx);
        if cfg!(target_os = "linux") {
            assert!(result.is_ok());
            assert!(watcher.is_running());
        } else {
            assert!(result.is_err());
            assert!(!watcher.is_running());
        }
    }
}
