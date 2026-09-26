# textweaver

An accessible, keyboard-first document reader and writer that speaks. textweaver reads documents aloud with a highlight that follows the spoken word exactly, lets you move by character, word, sentence, line, paragraph, heading, table, list, and link, and gives speech feedback while you write Markdown. It is built first for screen-reader users and students with print disabilities.

textweaver is a Rust reimplementation of the core of [Star](https://github.com/leavesofgrass/star). It learns from [Paperback](https://github.com/trypsynth/paperback) for document handling and accessibility, and from Omnivox for queued, multi-stream speech.

> **Status: Phase 0 (contract).** The workspace, the shared core types, the ADRs, and compiling stubs for every crate are in place. Reading, speech, and the terminal UI are being built in Wave 1. Nothing here is ready for daily use yet.

## What it will do (first release)

- `textweaver FILE`: a self-voicing terminal reader for text, Markdown, and HTML, with synchronized highlighting, position restore, bookmarks, find, navigation history, configurable keys, and edit mode with typing echo.
- `tw`: a scripting CLI: extract canonical text (`tw text`), inspect (`tw info`), search (`tw search`), speak or write audio (`tw speak`), list voices and speech backends (`tw voices`, `tw backends`).
- Speech backends: ETI-Eloquence through its ECI engine with exact word timing, espeak-ng in-process with audio-clock word timing (Linux), Omnivox over its speech-server protocol, and a silent backend; native Windows and macOS voices come later.

Later: EPUB, DOCX, and PDF; audio and subtitle export; a library with full-text search and Obsidian vault import and export; dictation; a native GUI built on wxWidgets.

## Building

Requirements: Rust 1.96 (installed automatically from `rust-toolchain.toml` by rustup).

```bash
cargo build --workspace
```

```bash
cargo test --workspace
```

On Linux, the espeak-ng backend needs `libespeak-ng-dev` and clang. The Docker development image has everything; see [docs/docker.md](docs/docker.md).

```bash
docker compose build dev
```

```bash
docker compose run --rm -T dev cargo test --workspace --all-features
```

## Repository layout

| Path | What |
|---|---|
| `crates/textweaver-core` | Positions, units, offset maps, edits, voice parameters, utterances |
| `crates/textweaver-text` | Document model, units, navigation, history, search, narration |
| `crates/textweaver-formats` | Loaders and registry |
| `crates/textweaver-speech` | Speech backends, speech service, pacing, normalization |
| `crates/textweaver-store` | Settings, keymap file, per-document state, sync |
| `crates/textweaver-keymap` | Actions, key chords, layers, defaults |
| `crates/textweaver-a11y` | Announcements and verbosity |
| `crates/textweaver-editor` | Undo, Markdown commands, typing echo, autosave |
| `crates/textweaver-app` | UI-independent application core |
| `crates/textweaver-tui` | The `textweaver` terminal reader |
| `crates/textweaver-cli` | The `tw` command |
| `xtask` | `cargo xtask keyboard`, `cargo xtask parity` |
| `docs/` | Plan, ADRs, Star parity reference, tasks, Docker guide |
| `fixtures/` | Sample documents and Star's reference output for them |

## Documentation

- [Implementation plan](docs/plan.md)
- Architecture decisions: [workspace](docs/adr/0001-workspace-and-dependencies.md), [text model](docs/adr/0002-text-model.md), [speech threading](docs/adr/0003-speech-threading-and-event-timing.md), [rate, pitch, volume](docs/adr/0004-rate-pitch-volume.md), [narration and offset maps](docs/adr/0005-narration-and-offset-map.md), [keymap](docs/adr/0006-keymap-and-actions.md), [Eloquence](docs/adr/0007-eloquence-via-eci-host.md), [Apple speech](docs/adr/0008-apple-speech.md), [SAPI5](docs/adr/0009-sapi5-voices.md)
- [Star parity reference](docs/star-parity.md)
- [Tasks and ownership](docs/tasks.md)
- [Docker development container](docs/docker.md)

## License

GPL-3.0-or-later, like Star. See [LICENSE](LICENSE).
