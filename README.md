# textweaver

An accessible, keyboard-first document reader and writer that speaks. textweaver reads documents aloud with a highlight that follows the spoken word exactly. You can move by character, word, sentence, line, paragraph, heading, table, list, and link. While you write Markdown, it tells you what you type. It is built first for screen-reader users and for students with print disabilities.

textweaver is a Rust reimplementation of the core of [Star](https://github.com/leavesofgrass/star). It learns from [Paperback](https://github.com/trypsynth/paperback) for document handling and accessibility, and from Omnivox for queued, multi-stream speech.

> **Status: alpha.** textweaver reads documents aloud with exact word highlighting. It has an edit mode with spell check, an outline, and citations; notes, a library, conversion to many formats, audio export, math, and modes for working with a screen reader. It runs on Windows, macOS, and Linux. It is ready for testing, not yet for daily reliance. The newest release is 0.1.0-alpha.4. See [CHANGELOG.md](CHANGELOG.md).

## Start here

- New to textweaver? The [quick start](#quick-start) below gets you reading in under a minute, and [docs/quickstart.md](docs/quickstart.md) has the full version.
- Every guide is listed in the [documentation index](docs/README.md), grouped for users, contributors, and design decisions.
- The [interactive pages](docs/site/index.html) explain the architecture, the speech pipeline, the keyboard, and the reading aids. Open `docs/site/index.html` in any browser. They work offline.

## Download

Windows, macOS, and Linux packages are on the [releases page](https://github.com/leavesofgrass/textweaver/releases). [docs/install.md](docs/install.md) has the details, including how to check a download. The macOS build is not notarized yet; the install guide shows how to open it anyway.

## Quick start

### Windows

Download the `.zip`, extract it, then in that folder:

```powershell
textweaver QUICKSTART.md
```

### macOS

Download the `.tar.gz`, extract it, allow it to run once (`xattr -dr com.apple.quarantine .`), then:

```bash
./textweaver QUICKSTART.md
```

### Linux

Download the `.AppImage`, make it executable, then:

```bash
chmod +x textweaver-*-linux-x86_64.AppImage && ./textweaver-*-linux-x86_64.AppImage QUICKSTART.md
```

Once it opens, **Space** starts and pauses reading, **h** jumps to the next heading, and **?** lists every key. [docs/quickstart.md](docs/quickstart.md) has your first 30 seconds in full, install scripts for every system, and the writing keys.

## What it does

textweaver has two programs and a window. `textweaver FILE` is a self-voicing terminal reader: it opens text, Markdown, HTML, EPUB, Word, RTF, OpenDocument, LaTeX, email, saved web pages, PDF, DAISY, PowerPoint, spreadsheets, and archives, with OCR for scans and pictures, and an edit mode for writing Markdown. `tw` is a command-line tool for converting, speaking, exporting audio, citing sources, and managing your library from a terminal or a script. `textweaver-xilem FILE` is the window: a native, screen-reader-accessible GUI that shares documents, keys, settings, and voices with the terminal reader. [docs/README.md](docs/README.md) lists every guide, and the [roadmap](docs/roadmap.md) says what comes next.

## Building

You need Rust. rustup installs the right version (1.96) from `rust-toolchain.toml`.

```bash
cargo build --workspace
```

```bash
cargo test --workspace
```

On Linux, the build needs pkg-config and the ALSA development files (`libasound2-dev` on Debian and Ubuntu). The Docker development image has everything; see [docs/dev/docker.md](docs/dev/docker.md).

Before you send a change, run the checks CI runs. [CONTRIBUTING.md](CONTRIBUTING.md) explains them, and [docs/dev/building.md](docs/dev/building.md) has the full setup for every system, the scripts, the repository layout, and the third-party data textweaver bundles.

```bash
scripts/dev-check.sh
```

## License

GPL-3.0-or-later, like Star. See [LICENSE](LICENSE).

## See also

- [Documentation index](docs/README.md): every guide, grouped by audience.
- [Quick start](docs/quickstart.md): your first 30 seconds.
- [Building](docs/dev/building.md): the full setup, scripts, and repository layout.
- [CONTRIBUTING.md](CONTRIBUTING.md): how to build, check, and send changes.
- [CHANGELOG.md](CHANGELOG.md): what changed in each release.
