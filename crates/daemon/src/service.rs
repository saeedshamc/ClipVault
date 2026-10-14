//! Daemon supervision: config, store, watcher, IPC.

use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{Context, Result};
use clipvault_core::{AppPaths, Config, Store};
use clipvault_platform::ClipboardWatcher;

use crate::app::{self, RuntimeState};
use crate::ipc;

pub fn run_daemon() -> Result<()> {
    let config = load_config()?;
    let paths = AppPaths::resolve(&config)?;
    paths.ensure_directories()?;

    let store = Store::open(&paths.database_file)
        .with_context(|| format!("open db {}", paths.database_file.display()))?;
    let report = store.apply_retention(&config)?;
    if report.deleted > 0 {
        tracing::info!(deleted = report.deleted, "startup retention complete");
    }

    let runtime = Arc::new(Mutex::new(RuntimeState {
        paused: config.watcher.paused,
    }));

    let (tx, rx) = mpsc::channel();
    let mut watcher = create_watcher();
    watcher
        .start(tx)
        .context("failed to start clipboard watcher")?;
    tracing::info!("clipboard watcher started");

    let store_for_events = Store::open(&paths.database_file)?;
    let config_for_events = config.clone();
    let runtime_for_events = Arc::clone(&runtime);
    thread::Builder::new()
        .name("clipvault-ingest".into())
        .spawn(move || {
            while let Ok(event) = rx.recv() {
                app::ingest_event(
                    &store_for_events,
                    &config_for_events,
                    &runtime_for_events,
                    event,
                );
            }
        })
        .context("spawn ingest thread")?;

    let store_for_ipc = Store::open(&paths.database_file)?;
    let config_for_ipc = config.clone();
    let runtime_for_ipc = Arc::clone(&runtime);

    // Blocking accept loop (named pipe / unix socket).
    let serve_result = ipc::serve_forever(move |req| {
        app::handle_request(&store_for_ipc, &config_for_ipc, &runtime_for_ipc, req)
    });

    let _ = watcher.stop();
    serve_result
}

fn load_config() -> Result<Config> {
    let defaults = Config::default();
    let paths = AppPaths::resolve(&defaults)?;
    paths.ensure_directories()?;
    let config = Config::load_or_default(&paths.config_file)?;
    if !paths.config_file.exists() {
        config.save(&paths.config_file)?;
        tracing::info!(path = %paths.config_file.display(), "wrote default config");
    }
    Ok(config)
}

fn create_watcher() -> Box<dyn ClipboardWatcher + Send> {
    #[cfg(windows)]
    {
        Box::new(clipvault_platform_windows::WindowsClipboardWatcher::default())
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(clipvault_platform_linux::LinuxClipboardWatcher::default())
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        struct Unsupported;
        impl ClipboardWatcher for Unsupported {
            fn start(
                &mut self,
                _tx: std::sync::mpsc::Sender<clipvault_platform::ClipboardEvent>,
            ) -> Result<(), clipvault_platform::PlatformError> {
                Err(clipvault_platform::PlatformError::Unavailable(
                    "unsupported platform".into(),
                ))
            }
            fn stop(&mut self) -> Result<(), clipvault_platform::PlatformError> {
                Ok(())
            }
        }
        Box::new(Unsupported)
    }
}
