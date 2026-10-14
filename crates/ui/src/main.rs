//! ClipVault UI entry point.
//!
//! The Tauri frontend lands in milestone v0.9.0. Until then this binary only
//! reports the workspace version so packaging and CI have a compile target.

use anyhow::Result;

fn main() -> Result<()> {
    println!("clipvault-ui {}", clipvault_core::version());
    Ok(())
}
