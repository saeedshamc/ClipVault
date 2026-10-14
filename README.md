# ClipVault

Fast, private, always-on clipboard history for Windows and Linux.

ClipVault runs a tiny background daemon that captures clipboard changes (text,
HTML, images, file lists), stores them locally with full-text search, and opens
an on-demand history window via a global hotkey or tray icon.

> **Current release: v0.4.0** — Windows text watcher, daemon IPC, and CLI.
> Linux capture, rich formats, and UI arrive later — see
> [docs/ROADMAP.md](docs/ROADMAP.md).

## Goals

- **Private by default** — respect OS “do not store” clipboard signals, app
  blocklists, regex ignore rules, optional SQLCipher encryption
- **Reliable** — daemon survives UI crashes; supervised watcher with backoff;
  single-instance; clean shutdown
- **Fast** — event-driven clipboard watching (no polling as the primary path);
  SQLite + FTS5; content-addressed blobs for large images
- **Cross-platform** — Windows and Linux (X11; Wayland via `wlr-data-control`
  where available)

## Architecture

Two processes:

| Process | Role |
|---------|------|
| `clipvault-daemon` | Always-on: watch clipboard, write DB, serve IPC |
| `clipvault-ui` | On-demand Tauri UI talking to the daemon over IPC |

IPC uses a versioned JSON protocol over a Unix domain socket (Linux) or named
pipe (Windows), restricted to the current user.

## Workspace

```
crates/core/                 shared model, storage, config, IPC protocol
crates/platform/             ClipboardWatcher / Autostart / SourceApp traits
crates/platform/windows/     Win32 backends
crates/platform/linux/       X11 / Wayland backends
crates/daemon/               headless daemon binary
crates/ui/                   history UI (Tauri in a later milestone)
```

## Build

Requirements: [Rust](https://rustup.rs/) (see `rust-toolchain.toml`).

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# Daemon (Windows: captures CF_UNICODETEXT)
cargo run -p clipvault-daemon -- run

# CLI against a running daemon
cargo run -p clipvault-daemon -- ping
cargo run -p clipvault-daemon -- list
cargo run -p clipvault-daemon -- search hello
cargo run -p clipvault-daemon -- pause
cargo run -p clipvault-daemon -- resume
```

## Platform support (honest)

| Platform | Clipboard | Autostart | Notes |
|----------|-----------|-----------|-------|
| Windows 10/11 | Planned (listener API) | Planned (HKCU Run) | Primary target |
| Linux X11 | Planned (XFixes) | Planned (XDG + systemd user) | Primary Linux path |
| Linux Wayland (wlroots/KDE) | Planned (`wlr-data-control`) | Same | GNOME unsupported for direct capture |
| macOS | Not planned | — | Out of scope for 1.0 |

Details will live in `docs/SUPPORT_MATRIX.md` once capture is implemented.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE](LICENSE))

at your option.
