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

## Later milestones

Checklists for Windows watcher, Linux X11/Wayland, autostart, privacy filters,
and packaging will be appended when those features are implemented.
