# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0] - 2026-10-15

### Added

- Versioned length-prefixed JSON IPC protocol (`PROTOCOL_VERSION = 1`)
- Windows text clipboard watcher via message-only window +
  `AddClipboardFormatListener` (sequence-number dedupe, OpenClipboard retries)
- Daemon runtime: config load, SQLite store, watcher ingest, retention on capture
- Local IPC server (named pipe on Windows, Unix socket on Linux) via `interprocess`
- CLI: `run`, `ping`, `list`, `search`, `delete`, `clear`, `pause`, `resume`,
  `status`
- Manual test checklist for Windows desktop verification

### Known limitations

- Linux watcher remains a stub (X11 lands in v0.6.0)
- Text-only capture (`CF_UNICODETEXT`); rich formats in v0.7.0
- No single-instance lock or supervised restart yet (v0.5.0)

## [0.3.0] - 2026-10-15

### Added

- blake3 content hashing for multi-format clipboard payloads
- `Store::capture` with dedupe (refresh `last_used_at`) and oversized skip+log
- Retention pass by age, item count, and total size; pinned items are never
  auto-deleted
- Unit tests for hashing, capture outcomes, and retention edge cases

## [0.2.0] - 2026-10-15

### Added

- Clipboard domain model with multi-format payloads (`text/plain`, `text/html`, …)
- TOML configuration with defaults for storage paths, size limits, retention, and
  watcher pause / PRIMARY capture flags
- Platform path resolution via the `directories` crate
- SQLite storage in WAL mode with forward-only migrations and `schema_version`
- FTS5 full-text search with safe query tokenization
- CRUD APIs: insert, get, list, search, pin, touch, delete, clear, counts
- Unit tests for model, config, migrations, and storage

### Changed

- Minimum Rust version raised to 1.85 (edition 2024 support for dependencies)

## [0.1.0] - 2026-10-15

### Added

- Repository foundation: commit-message hooks, `.gitignore`, pinned Rust toolchain
- Cargo workspace with crate stubs
- Dual licensing (MIT OR Apache-2.0)
- GitHub Actions CI on `windows-latest` and `ubuntu-latest`
- README skeleton and milestone roadmap

[Unreleased]: https://github.com/saeedshamc/ClipVault/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/saeedshamc/ClipVault/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/saeedshamc/ClipVault/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/saeedshamc/ClipVault/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/saeedshamc/ClipVault/releases/tag/v0.1.0
