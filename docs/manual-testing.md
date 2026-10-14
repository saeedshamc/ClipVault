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

## v0.4.0 — Windows watcher + daemon IPC

Run from a desktop session (not Session 0 / pure headless without user profile):

1. `cargo run -p clipvault-daemon -- run`
2. In another terminal:
   - `cargo run -p clipvault-daemon -- ping`
   - Copy some text in Notepad
   - `cargo run -p clipvault-daemon -- list`
   - `cargo run -p clipvault-daemon -- search <word>`
   - `cargo run -p clipvault-daemon -- pause` then copy again (must not appear)
   - `cargo run -p clipvault-daemon -- resume`
   - `cargo run -p clipvault-daemon -- clear`
3. Confirm logs never print clipboard contents (only kind/size/hash prefix)

### Known gaps in v0.4.0

- Linux watcher is still a stub (no X11 capture yet)
- Only `CF_UNICODETEXT` is captured (HTML/images later)
- No single-instance lock / supervised restart yet (v0.5.0)
- Named pipe ACL is default (current-user restriction hardening later)

## Later milestones

Checklists for Linux X11/Wayland, autostart, privacy filters, and packaging will
be appended when those features are implemented.
