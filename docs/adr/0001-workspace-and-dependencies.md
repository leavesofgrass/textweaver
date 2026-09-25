# ADR-0001: Workspace layout and dependency policy

- Status: accepted
- Date: 2026-09-25

## Context

textweaver reimplements Star (Python, 45K lines) in Rust. Four agents build it in parallel, so the crate boundaries have to let each agent compile and test alone against a stable contract. Star's history shows the cost of the opposite: 40+ GUI and TUI mixins sharing state, and a long tail of optional dependencies.

## Decision

**One Cargo workspace, one crate per job**, with a strict dependency direction:

```
core ← text, speech, keymap, a11y, store, editor
text ← formats
all of the above ← app ← tui, cli
```

- `textweaver-core` holds only leaf types shared across crates: `CharPos`, `CharRange`, `Direction`, `Bias`, `Unit`, `MarkerKind`, `OffsetMap`, `Edit`, `EditOutcome`, `Rate`, `Pitch`, `Volume`, `Utterance`, and the preference enums (`Verbosity`, `PunctuationLevel`, `HighlightGranularity`, `CapsIndication`). The orchestrator owns it; agents request changes.
- `speech` depends only on `core`. It receives `Utterance`s, never a `Document`.
- `store` depends only on `core`. Keymap overrides are stored as strings and interpreted by `keymap`.
- `a11y` depends only on `core`. The speech announcer takes a callback, so it does not depend on `speech`.
- `editor` works on a `ropey::Rope` plus `core::Edit`; `text::Document::apply` is the document side of the same edit.

**Every third-party crate is declared once** in `[workspace.dependencies]` in the root `Cargo.toml`, and crates opt in with `name.workspace = true`. That table is the approved list. Adding to it is an orchestrator decision; agents ask in their report. At Phase 0 it holds: `ropey` 1.6, `unicode-segmentation`, `regex`, `pulldown-cmark`, `scraper`, `zip`, `roxmltree`, `serde`, `serde_json`, `toml`, `directories`, `thiserror`, `anyhow`, `bitflags`, `log`, `clap`, `ratatui`, `crossterm`, `espeakng-sys`, `speech-dispatcher`, `tts`, `rodio`, `proptest`, `insta`, `tempfile`.

**Rules:**
- No async runtime. Threads and `std::sync::mpsc` channels.
- `thiserror` in libraries, `anyhow` only in binaries (`tui`, `cli`, `xtask`).
- Engines and heavy formats behind cargo features, off by default: `textweaver-speech/{espeak, omnivox, speechd, tts-crate}`, `textweaver-formats/{paperback, pandoc}`, `textweaver-a11y/live-region`.
- Workspace lints: `unsafe_code = "deny"` (the espeak backend's FFI module opts out with `#[allow(unsafe_code)]` and a `// SAFETY:` comment on every block), `missing_docs = "warn"`, clippy `all = "warn"`. CI and `-D warnings` make all of them errors.
- Edition 2024. Toolchain pinned in `rust-toolchain.toml` to 1.96 so Windows, CI, and the Docker image agree; `rust-version = "1.85"` states the true minimum (edition 2024).

**Environments.** Native Windows (Jon's machine) and Linux in Docker (`docker/Dockerfile`, `compose.yaml`, see `docs/docker.md`). CI runs fmt, clippy, tests, and rustdoc on Ubuntu, macOS, and Windows; only Ubuntu enables the features that link system libraries.

**`live-region` is not approved for the terminal build.** Version 0.3.2 depends on wxDragon, which would pull wxWidgets into every build. It returns with the GUI in wave 3 behind `textweaver-a11y/live-region`.

## Consequences

- Each agent can run `cargo test -p <crate>` without the other agents' work.
- Moving a type into `core` is the standard fix for a would-be cycle; it is cheap but centralizes change requests on the orchestrator.
- Features that link system libraries (espeak-ng) are exercised only on Linux in CI and Docker; Windows and macOS rely on native backends from wave 3.
