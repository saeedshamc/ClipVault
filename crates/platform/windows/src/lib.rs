//! Windows-specific platform backends for ClipVault.
//!
//! Real clipboard listening (`AddClipboardFormatListener`) and HKCU Run autostart
//! are implemented in later milestones. This crate currently exposes stubs so the
//! workspace builds on every host.

use clipvault_platform::{Autostart, ClipboardWatcher, PlatformError};

/// Placeholder Windows clipboard watcher.
#[derive(Debug, Default)]
pub struct WindowsClipboardWatcher {
    running: bool,
}

impl ClipboardWatcher for WindowsClipboardWatcher {
    fn start(&mut self) -> Result<(), PlatformError> {
        if cfg!(windows) {
            self.running = true;
            Ok(())
        } else {
            Err(PlatformError::Unavailable(
                "Windows clipboard watcher requires a Windows host".into(),
            ))
        }
    }

    fn stop(&mut self) -> Result<(), PlatformError> {
        self.running = false;
        Ok(())
    }
}

impl WindowsClipboardWatcher {
    /// Returns whether [`ClipboardWatcher::start`] has been called successfully.
    pub fn is_running(&self) -> bool {
        self.running
    }
}

/// Placeholder Windows autostart manager (HKCU Run key in a later milestone).
#[derive(Debug, Default)]
pub struct WindowsAutostart;

impl Autostart for WindowsAutostart {
    fn install(&self) -> Result<(), PlatformError> {
        if cfg!(windows) {
            Err(PlatformError::Operation(
                "Windows autostart is not implemented yet".into(),
            ))
        } else {
            Err(PlatformError::Unavailable(
                "Windows autostart requires a Windows host".into(),
            ))
        }
    }

    fn uninstall(&self) -> Result<(), PlatformError> {
        if cfg!(windows) {
            Err(PlatformError::Operation(
                "Windows autostart is not implemented yet".into(),
            ))
        } else {
            Err(PlatformError::Unavailable(
                "Windows autostart requires a Windows host".into(),
            ))
        }
    }

    fn is_installed(&self) -> Result<bool, PlatformError> {
        if cfg!(windows) {
            Ok(false)
        } else {
            Err(PlatformError::Unavailable(
                "Windows autostart requires a Windows host".into(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clipvault_platform::ClipboardWatcher;

    #[test]
    fn watcher_start_respects_target_os() {
        let mut watcher = WindowsClipboardWatcher::default();
        let result = watcher.start();
        if cfg!(windows) {
            assert!(result.is_ok());
            assert!(watcher.is_running());
        } else {
            assert!(result.is_err());
            assert!(!watcher.is_running());
        }
    }
}
