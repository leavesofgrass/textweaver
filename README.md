# textweaver

An accessible, keyboard-first document reader and writer that speaks. textweaver reads documents aloud with a highlight that follows the spoken word exactly. You can move by character, word, sentence, line, paragraph, heading, table, list, and link. While you write Markdown, it tells you what you type. It is built first for screen-reader users and for students with print disabilities.

textweaver is a Rust reimplementation of the core of [Star](https://github.com/leavesofgrass/star). It learns from [Paperback](https://github.com/trypsynth/paperback) for document handling and accessibility, and from Omnivox for queued, multi-stream speech.

> **Status: alpha.** textweaver reads documents aloud with exact word highlighting. It has an edit mode, notes, a library, conversion to many formats, audio export, math, and citations. It runs on Windows, macOS, and Linux. It is ready for testing, not yet for daily reliance. See [CHANGELOG.md](CHANGELOG.md).

## Start here

- New to textweaver? Read the [quick start](docs/quickstart.md). It covers your first 30 seconds on Windows, macOS, and Linux.
- Every guide is listed in the [documentation index](docs/README.md), grouped for users, contributors, and design decisions.
- The [roadmap](docs/roadmap.md) says what comes next, quick wins first.
- The [interactive pages](docs/site/index.html) explain the architecture, the speech pipeline, the keyboard, and the reading aids. Open `docs/site/index.html` in any browser. They work offline.

## Download

Windows and macOS packages are on the [releases page](https://github.com/leavesofgrass/textweaver/releases). [docs/install.md](docs/install.md) has the details. The macOS build is not notarized yet; the install guide shows how to open it anyway. On Linux, build from source with `scripts/install-linux.sh`.

## What it does

textweaver has two programs.

- `textweaver FILE` is a self-voicing terminal reader.
  - It opens text, Markdown, HTML, EPUB, Word (DOCX), and PDF. Other formats, such as OpenDocument or RTF, can be converted to Markdown first with `tw convert`, which uses Pandoc for them when it is installed.
  - The highlight follows the spoken word exactly, even when numbers, abbreviations, or math are read in words.
  - You move by character, word, sentence, line, paragraph, heading, table, list, list item, link, and chapter.
  - It has Speech Cursor mode, bookmarks, notes and highlights, find, go to, and navigation history.
  - It remembers your place in every document.
  - Edit mode gives typing echo, Markdown formatting commands, undo, find and replace, and autosave recovery.
  - Reading aids: RSVP (one word at a time), bionic reading, a reading ruler, and 23 themes checked for contrast.
  - Every key can be changed, and single-key shortcuts can be turned off with F9.
  - It works alongside JAWS, NVDA, VoiceOver, and Orca. Start it with `--no-speech` to let your screen reader do the talking.
- `tw` is a command-line tool.
  - `tw text`, `tw info`, and `tw search` extract, describe, and search a document.
  - `tw convert` converts files and whole folders to Markdown, HTML, text, EPUB, Word, braille (BRF), and tagged PDF. It uses every processor core, and it can watch a folder. With Pandoc installed, it also reads formats textweaver has no reader for.
  - `tw speak` and `tw export-audio` speak a document, or write it to WAV, MP3, or an M4B audiobook with chapters and subtitles.
  - `tw voices`, `tw backends`, and `tw eloquence` list voices and speech engines.
  - `tw cite` keeps a reference library: DOI and ISBN lookup, BibTeX, RIS, CSL-JSON, and citation styles.
  - `tw library`, `tw marks`, `tw vault`, and `tw migrate-star` manage your library, your saved places, your Obsidian vault, and your Star data.
  - `tw dictate` turns speech in an audio file into text with Whisper.
  - `tw settings` exports, imports, and resets your settings as JSON.
  - `tw serve --stdio` lets editors and other programs drive textweaver over JSON-RPC.
- Speech engines:
  - ETI-Eloquence through its ECI engine, with exact word timing;
  - SAPI5 and OneCore voices on Windows, 64-bit and 32-bit;
  - Apple's voices on macOS, including Eloquence Reed;
  - espeak-ng and speech-dispatcher on Linux;
  - DECtalk, when you have a licensed copy;
  - Omnivox.

Coming next (the [roadmap](docs/roadmap.md) has the full list):

- the native GUI (a working spike exists; see [ADR-0014](docs/adr/0014-gui-toolkit.md));
- dictation from the microphone;
- exploring math term by term in the reader;
- Linux packages.

## Building

You need Rust. rustup installs the right version (1.96) from `rust-toolchain.toml`.

```bash
cargo build --workspace
```

```bash
cargo test --workspace
```

On Linux, the espeak-ng backend needs the espeak-ng development files (`libespeak-ng-dev` on Debian and Ubuntu). The Docker development image has everything; see [docs/docker.md](docs/docker.md).

```bash
docker compose build dev
```

```bash
docker compose run --rm -T dev cargo test --workspace --all-features
```

Before you send a change, run the checks CI runs. [CONTRIBUTING.md](CONTRIBUTING.md) explains them and the rest of the workflow.

```bash
scripts/dev-check.sh
```

## Scripts

The `scripts/` folder has installers and helpers for every system. Each script has `--help` and `--dry-run`. It says what it will do before it does it, and asks before it uses sudo or changes your PATH. The full list is in [scripts/README.md](scripts/README.md).

- `install-linux.sh`: build and install from source on Debian, Ubuntu, Fedora, Arch, openSUSE, or Alpine.
- `install-macos.sh`: install the newest macOS release, or build from source.
- `install-windows.ps1`: install the newest Windows release, or build from source.
- `update.sh` and `update.ps1`: update an installed textweaver.
- `speech-check.sh` and `speech-check.ps1`: report the speech engines, voices, and audio.
- `doctor.sh` and `doctor.ps1`: a system report to paste into a bug report.
- `dev-check.sh` and `dev-check.ps1`: run CI's checks locally.
- `convert-folder.sh` and `convert-folder.ps1`: convert a folder of Markdown to HTML, EPUB, PDF, or another format.
- `voxin-docker.sh`: run the Eloquence tests or `tw speak` with Voxin in the Docker container.

## Repository layout

- `crates/`: the Rust crates, one per job. [docs/architecture.md](docs/architecture.md) describes each one, how they depend on each other, and how a document becomes speech.
- `xtask/`: maintenance tasks, run as `cargo xtask bench`, `dist`, `hosts`, `eci-host`, `sapi-host`, `keyboard`, and `parity`.
- `scripts/`: installers, update, speech check, doctor, dev-check, and folder conversion.
- `tools/`: helper programs, among them the link checker (`check_links.py`), the site data generator (`gen_site_data.py`), and the engine spikes.
- `docs/`: user guides, contributor guides, the ADRs, and the interactive pages in `docs/site/`. Start at [docs/README.md](docs/README.md).
- `fixtures/`: sample documents for tests, and Star's reference output for them.
- `third_party/`: pronunciation dictionaries, fonts, and word lists, each with its licence.
- `docker/`, `compose.yaml`, `compose.voxin.yaml`: the Linux development container.

## Third-party data

- `third_party/ibmtts-dictionaries/`: the community IBMTTS pronunciation dictionaries by amirsol81, x0, thunderdrop and contributors (CC0 1.0), used by the Eloquence backend.
- `third_party/fonts/`: fonts built into textweaver for PDF and EPUB output and the GUI, each with its licence file and a README giving its source, version, and SHA-256:
  - Atkinson Hyperlegible Next and Atkinson Hyperlegible Mono, copyright 2020-2024 The Atkinson Hyperlegible Next Project Authors and The Atkinson Hyperlegible Mono Project Authors (Braille Institute of America), SIL Open Font License 1.1.
  - OpenDyslexic, copyright 2019 Abbie Gonzalez, with Reserved Font Name OpenDyslexic, SIL Open Font License 1.1.
- `third_party/scowl/`: word levels for difficult-word marking, derived from SCOWL (Spell Checker Oriented Word Lists) version 2, release 2026.02.25, copyright 2000-2026 Kevin Atkinson, with the Australian English data copyright 2016 Benjamin Titze; MIT-like licence. The full notices are in `third_party/scowl/Copyright`, and `tools/scowl_levels.py` rebuilds the list.

## License

GPL-3.0-or-later, like Star. See [LICENSE](LICENSE).

## See also

- [Documentation index](docs/README.md): every guide, grouped by audience.
- [Quick start](docs/quickstart.md): your first 30 seconds.
- [Architecture](docs/architecture.md): the crates and how speech and highlighting work.
- [Roadmap](docs/roadmap.md): what comes next.
- [CONTRIBUTING.md](CONTRIBUTING.md): how to build, check, and send changes.
- [CHANGELOG.md](CHANGELOG.md): what changed in each release.
