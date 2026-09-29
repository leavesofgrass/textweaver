## What and why

<!-- What this changes, and the reason. Link the issue it fixes, for example "Fixes #12". -->

## How it was checked

<!-- The commands you ran and their result lines. -->

## Accessibility

<!-- What you checked by ear or on a Braille display: which screen reader, Braille display, speech engine, and system. If you could not check something, say so; a maintainer will. Write "No user-facing change" if nothing a user sees or hears changed. -->

## Checklist

Tick what applies, and leave a note for anything you could not do. A draft pull request is welcome if you are stuck.

- [ ] `scripts/dev-check.sh` or `scripts\dev-check.ps1` passes (formatting, clippy, tests, rustdoc, the keyboard reference, links, and the site checks).
- [ ] The Docker run passes, if Linux code or features changed: `scripts/dev-check.sh --docker`.
- [ ] A new test fails without the change.
- [ ] Every new state change is announced, and every new spoken string reads well aloud.
- [ ] Nothing new is shown by color alone.
- [ ] New keys come from the keymap, and `docs/keyboard.md` is regenerated (`cargo xtask keyboard`) if a key or action changed.
- [ ] A new setting is used somewhere, not only stored, and `docs/settings-reference.md` is regenerated (`cargo xtask settings-doc`).
- [ ] New dependencies are in `[workspace.dependencies]`, `cargo deny check` and `cargo xtask deps --check` pass, and `cargo xtask notices` has been run.
- [ ] The docs say how to use the change, and `CHANGELOG.md` has an entry under "Unreleased" for anything users will notice.
- [ ] Dates in files and commit messages come from the computer's clock, with the weekday computed.
