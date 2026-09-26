## What and why

<!-- What this changes, and the reason. Link the issue or roadmap item. -->

## How it was checked

<!-- The commands you ran and their result lines. Say what was checked by ear, with which engine and screen reader. -->

## Checklist

- [ ] `cargo fmt --all --check` passes.
- [ ] Clippy and tests pass on this platform: `scripts/dev-check.sh` or `scripts\dev-check.ps1`.
- [ ] The Docker all-features run passes, if Linux code or features changed: `scripts/dev-check.sh --docker`.
- [ ] `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` passes for the changed crates.
- [ ] `cargo xtask keyboard --check` passes, and `docs/keyboard.md` is regenerated if a key or action changed.
- [ ] `cargo xtask deps --check` passes (no forbidden crate edges).
- [ ] New dependencies are in `[workspace.dependencies]`, `cargo deny check` passes, and `cargo xtask notices` has been run.
- [ ] A new test fails without the change.
- [ ] Every new state change is announced, and every new spoken string reads well aloud.
- [ ] A new setting is used somewhere, not only stored.
- [ ] `CHANGELOG.md` has an entry under `[Unreleased]` for a change users will notice.
- [ ] Dates in files and commit messages come from the machine, with the weekday computed.
