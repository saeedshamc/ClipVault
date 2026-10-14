# Manual testing checklists

Automated tests cover what can run in CI. Platform-specific clipboard and
autostart behavior needs a real desktop session. Use these checklists when a
milestone lands the related feature.

## v0.1.0 — Foundation

- [ ] `cargo fmt --all -- --check` passes locally
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes
- [ ] `cargo test --workspace` passes
- [ ] `cargo run -p clipvault-daemon` prints the version
- [ ] `cargo run -p clipvault-ui` prints the version
- [ ] Commit with a forbidden marker in the message is rejected by `.githooks/commit-msg`

## v0.2.0 — Core storage

- [ ] `cargo test -p clipvault-core` passes (includes file-backed reopen test)
- [ ] Saving a custom `config.toml` and reloading preserves `limits.max_items`
- [ ] (Optional) Open the resolved DB path after a future daemon run and confirm
      `schema_migrations.version = 1`

## Later milestones

Checklists for Windows watcher, Linux X11/Wayland, autostart, privacy filters,
and packaging will be appended when those features are implemented.
