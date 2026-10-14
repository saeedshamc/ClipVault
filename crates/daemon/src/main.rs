//! ClipVault daemon entry point.
//!
//! Milestone v0.1.0 only prints version information. Watcher, storage, and IPC
//! arrive in later releases.

use anyhow::Result;

fn main() -> Result<()> {
    println!("clipvault-daemon {}", clipvault_core::version());
    Ok(())
}
