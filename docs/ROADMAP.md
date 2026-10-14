# Roadmap

Milestone plan for ClipVault. Each version ends with a release commit and an
annotated tag. Items marked **stub** exist only as compile-time placeholders.

## Released

### v0.1.0 — Repo foundation

- Git hooks rejecting attribution trailers in commit messages
- Workspace layout, toolchain pin, dual license, CI, README

### v0.2.0 — Core model + storage

- Clipboard item model, TOML config, SQLite (WAL) with forward-only migrations
- FTS5 search, CRUD API, unit tests

### v0.3.0 — Dedupe + retention

- blake3 content hashing, pin-aware retention by count/age/total size
- Configurable size limits with skip-and-log for oversized items

### v0.4.0 — Windows watcher + daemon MVP

- Message-only window + `AddClipboardFormatListener` (text)
- Daemon IPC + CLI: `list`, `search`, `delete`, `clear`, `pause`, `resume`

## Planned

### v0.5.0 — Reliability (Windows)

- Single-instance, supervised watcher restart, graceful shutdown, tracing logs
- HKCU Run autostart + `install-autostart` / `uninstall-autostart`

### v0.6.0 — Linux X11

- XFixes CLIPBOARD (optional PRIMARY), INCR transfers
- XDG autostart + optional systemd user unit

### v0.7.0 — Rich formats

- Images, HTML/RTF, file lists, blob storage, multi-format items
- Self-copy loop prevention, paste-back from history

### v0.8.0 — Privacy

- OS privacy formats, app blocklist, regex rules, pause hotkey
- Optional SQLCipher with OS keychain

### v0.9.0 — Tauri UI

- Global hotkey, search, pin/delete, keyboard navigation, tray, themes

### v0.10.0 — Wayland

- `wlr-data-control`, compositor detection, documented GNOME fallbacks

### v0.11.0 — Packaging

- Windows installer (NSIS or WiX), Linux `.deb` + AppImage

### v1.0.0 — Hardening

- Fuzz/stress tests, DB recovery, soak script, full docs, history audit

## Open issues / intentional stubs

| Location | Stub | Target milestone |
|----------|------|------------------|
| `clipvault-platform-windows` autostart | Not implemented | v0.5.0 |
| `clipvault-platform-linux` watcher/autostart | Returns unavailable/not-implemented | v0.6.0 |
| `clipvault-ui` | Version-only binary (no Tauri yet) | v0.9.0 |
| Rich clipboard formats / paste-back | Text only today | v0.7.0 |
