# textweaver

An accessible, keyboard-first document reader and writer that speaks. textweaver reads documents aloud with a highlight that follows the spoken word exactly, lets you move by character, word, sentence, line, paragraph, heading, table, list, and link, and gives speech feedback while you write Markdown. It is built first for screen-reader users and students with print disabilities.

textweaver is a Rust reimplementation of the core of [Star](https://github.com/leavesofgrass/star). It learns from [Paperback](https://github.com/trypsynth/paperback) for document handling and accessibility, and from Omnivox for queued, multi-stream speech.

> **Status: alpha.** textweaver reads documents aloud with exact word highlighting, has an edit mode, notes, and a library, and runs on Windows, macOS, and Linux. It is usable for testing, not yet for daily reliance. See [CHANGELOG.md](CHANGELOG.md).

## Download

Windows and macOS packages are on the [releases page](https://github.com/leavesofgrass/textweaver/releases). New here? Read the [quick start](docs/quickstart.md): what to do in your first 30 seconds on Windows, macOS, and Linux. [docs/install.md](docs/install.md) has the details. The macOS build is not notarized yet; the guide shows how to open it anyway. On Linux, build from source.

## What it does

- `textweaver FILE`: a self-voicing terminal reader.
  - It opens text, Markdown, and HTML.
  - The highlight follows the spoken word exactly.
  - Move by character, word, sentence, line, paragraph, heading, table, list, and link.
  - Speech Cursor mode, bookmarks, notes and highlights, find, and navigation history.
  - Your position is restored when you reopen a document.
  - Edit mode, with typing echo and Markdown commands.
  - Keys are configurable, and single-key shortcuts can be switched off.
- `tw`: a command-line tool.
  - `tw text`, `tw info`, and `tw search` extract, inspect, and search a document.
  - `tw speak` and `tw export-audio` speak a document or write it to audio with subtitles.
  - `tw voices` and `tw backends` list voices and speech engines.
  - `tw library`, `tw vault`, `tw migrate-star`, and `tw eloquence` manage your library, your Obsidian vault, your Star data, and Eloquence.
- Speech engines:
  - ETI-Eloquence through its ECI engine, with exact word timing;
  - SAPI5 and OneCore voices on Windows;
  - Apple's voices on macOS, including Eloquence Reed;
  - espeak-ng and speech-dispatcher on Linux;
  - Omnivox.

Coming next:

- EPUB, DOCX, and PDF reading;
- conversion to HTML, EPUB, DOCX, PDF, and braille;
- math and citations;
- themes and reading aids;
- a native GUI built on wxWidgets.

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
| `xtask` | `cargo xtask dist`, `hosts`, `keyboard`, `parity` |
| `docs/` | Plan, ADRs, Star parity reference, tasks, Docker guide |
| `fixtures/` | Sample documents and Star's reference output for them |

## Documentation

- [Implementation plan](docs/plan.md)
- Architecture decisions: [workspace](docs/adr/0001-workspace-and-dependencies.md), [text model](docs/adr/0002-text-model.md), [speech threading](docs/adr/0003-speech-threading-and-event-timing.md), [rate, pitch, volume](docs/adr/0004-rate-pitch-volume.md), [narration and offset maps](docs/adr/0005-narration-and-offset-map.md), [keymap](docs/adr/0006-keymap-and-actions.md), [Eloquence](docs/adr/0007-eloquence-via-eci-host.md), [Apple speech](docs/adr/0008-apple-speech.md), [SAPI5](docs/adr/0009-sapi5-voices.md)
- [Installing a release](docs/install.md) and [making one](docs/releasing.md)
- [Getting ETI-Eloquence](docs/eloquence.md) (or run `tw eloquence`)
- [Star parity reference](docs/star-parity.md)
- [Tasks and ownership](docs/tasks.md)
- [Docker development container](docs/docker.md)

## Third-party data

- `third_party/ibmtts-dictionaries/`: the community IBMTTS pronunciation dictionaries by amirsol81, x0, thunderdrop and contributors (CC0 1.0), used by the Eloquence backend.

## License

GPL-3.0-or-later, like Star. See [LICENSE](LICENSE).
