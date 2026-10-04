# textweaver

<img src="docs/assets/textweaver-logo.svg" alt="textweaver logo: the letters t and w woven on a loom" width="128" height="128">

textweaver is an accessible, keyboard-first document reader and writer that speaks: it reads documents aloud with a highlight that follows the spoken word exactly, moves by character, word, sentence, heading, table, and more, and echoes what you type while you write Markdown. It is built first for screen-reader users and for students with print disabilities, and it is a Rust reimplementation of the core of [star](https://github.com/leavesofgrass/star). It runs on Windows, macOS, and Linux.

> **Status: alpha.** textweaver is ready for testing, not yet for daily reliance. The newest release is 0.1.0-alpha.8. See [CHANGELOG.md](CHANGELOG.md), [what is new](docs/whats-new.md), and [known limits](docs/known-limits.md).

Start here: [students](docs/start-students.md), [accommodation staff](docs/start-staff.md), [privacy](docs/privacy.md), and the [accessibility statement](docs/accessibility.md).

## Quick start

On Windows, download the window package, `textweaver-VERSION-windows-x86_64-gui.zip`, from the [releases page](https://github.com/leavesofgrass/textweaver/releases), extract it, and run `textweaver-gui.exe`. Windows may warn once, because the program is not code-signed (see [Installing textweaver](docs/install.md#the-gui)). Press **Ctrl+O** to open a document and **Space** to hear it. **F1** opens the help, and **F3** lists every key.

For the terminal reader on any system, download that system's package instead, or [build it](#building), and from the folder you extracted run:

```bash
textweaver QUICKSTART.md
```

It reads its own quick start aloud. Once it opens:

- **Space** starts and pauses reading.
- **h** jumps to the next heading.
- **Ctrl+E** switches to edit mode, to write.
- **?** lists every key.

[docs/quickstart.md](docs/quickstart.md) has your first 30 seconds in full, for every system and the GUI. [Installing textweaver](docs/install.md) covers every package and how to check a download.

## What it does

- Reads text, Markdown, HTML, EPUB, Word, RTF, OpenDocument, LaTeX, email, saved web pages, PDF, DAISY, PowerPoint, spreadsheets, and archives aloud, with OCR for scans and pictures.
- Moves by character, word, sentence, line, paragraph, heading, table, list, or link, and remembers your place when you reopen a file.
- Edits Markdown with typing echo, spell check, an outline, and citations.
- Exports audio, braille, and other formats, and keeps a library and reading notes.
- Speaks math aloud, and works alongside a screen reader, or in place of one.
- Comes as a terminal reader (`textweaver`), a command-line tool (`tw`), and a native GUI (`textweaver-gui`), sharing documents, keys, settings, and voices.

## Building

You need Rust; rustup installs the version pinned in `rust-toolchain.toml`. You also need cmake 3.16 or later, for the Opus encoder; see [Building](docs/dev/building.md#cmake-for-opus-audio-export).

```bash
cargo build --workspace
cargo test --workspace
```

On Linux, the build needs pkg-config and the ALSA development files (`libasound2-dev` on Debian and Ubuntu); [docs/dev/docker.md](docs/dev/docker.md) has a container with everything. [docs/dev/building.md](docs/dev/building.md) has the full setup for every system.

## Contributing

We'd love your help. [CONTRIBUTING.md](CONTRIBUTING.md) covers the code rules, the checks (`scripts/dev-check.sh`), and how to send a change.

## Documentation

The [documentation index](docs/README.md) lists every guide, grouped for users, contributors, and design decisions. The [interactive pages](docs/site/index.html) explain the architecture, the speech pipeline, the keyboard, and the reading aids with diagrams you can explore by keyboard; open `docs/site/index.html` in any browser, offline.

## License

GPL-3.0-or-later, like star. See [LICENSE](LICENSE).
