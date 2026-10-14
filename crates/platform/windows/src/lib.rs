//! Windows-specific platform backends for ClipVault.

mod watcher;

pub use watcher::WindowsClipboardWatcher;

use clipvault_platform::{Autostart, PlatformError};

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
