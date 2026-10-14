# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
- Cargo workspace with `clipvault-core`, `clipvault-platform`,
  `clipvault-platform-windows`, `clipvault-platform-linux`, `clipvault-daemon`,
  and `clipvault-ui` stubs
- Dual licensing (MIT OR Apache-2.0)
- GitHub Actions CI (fmt, clippy, test) on `windows-latest` and `ubuntu-latest`
- README skeleton and milestone roadmap

[Unreleased]: https://github.com/saeedshamc/ClipVault/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/saeedshamc/ClipVault/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/saeedshamc/ClipVault/releases/tag/v0.1.0
