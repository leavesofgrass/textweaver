# Changelog

All notable changes to textweaver. Versions follow [Semantic Versioning](https://semver.org/); before 1.0 every release is an alpha and anything may change.

## [Unreleased]

### Added

- **Settings export and import as JSON.** `tw settings export` saves every setting and key override to one JSON (or TOML) file; `tw settings import` checks it, backs up your files, and applies it, with `--dry-run` to preview each change. `tw settings path` and `tw settings reset` are new too. See [docs/settings.md](docs/settings.md).

- **Choose a voice** (Alt+V) lists the engine's voices; Enter selects one and speaks a sample.
- **A log file.** Warnings and errors go to `textweaver.log` in the state folder, rotated at 1 MB. `--log debug` (or `TEXTWEAVER_LOG`) logs more; `--log off` turns it off.
- **Files changed on disk.** Saving over a file that another program changed since you opened it asks first. A change to an open file with no unsaved edits offers a reload.
- **Speech recovers from an engine crash or a stall.** The reading goes on from the last word heard, and says "Speech restarted".
- **GFM task lists** say "checked" or "not checked", and the text of HTML in Markdown (`<p align>`, `<details>`, `<img alt>`) is read.
- `tw export-audio` uses your settings: voice, rate, pitch, volume, the preferred engine, table and footnote modes, and the `[export]` subtitles.

### Changed

- Reading a long document starts at once: continuous reading is planned about ten minutes at a time. On a 10 MB file, next sentence while reading went from about 250 ms to 3 ms.
- The highlight is drawn as soon as its word is heard, not up to 40 ms later.
- A list says its first item after its title, and each item says where it is ("Chapter two, 2 of 5").
- The same message twice in a row is spoken twice by a screen reader: the status line blanks for a moment first.
- Entering edit mode on a large file is two to four times faster.

### Fixed

- A speech volume above 100 in `settings.toml` is now set to 100 and reported, like other out-of-range values.
- Bookmarks and notes landed in the wrong place after saving and then quitting in edit mode.
- `[normalization] table_mode` and `footnote_mode` had no effect.
- Jumping to an ordered list item says "2. Walk the dog", not just "2.".
- AltGr characters (`@ [ ] { } \ | ~ €`) can be typed in edit mode and prompts on non-US Windows keyboards.
- Binary files (a PDF or Word file without its reader, a program) are refused with a clear message instead of being read as garbage; UTF-16 text without a byte order mark is decoded.
- speech-dispatcher counted as not available where `XDG_RUNTIME_DIR` is unset (containers), though `spd-say` worked.
- The `espeak` feature builds on Fedora 44 and current Arch: its FFI no longer runs bindgen over the system headers.

## [0.1.0-alpha.3] - 2026-09-25

The first release with downloadable packages: Windows (x86_64) and macOS (universal, not notarized). Start with the [quick start](docs/quickstart.md) (`QUICKSTART.md` in each package); [docs/install.md](docs/install.md) has the details.

### Added

- **Edit mode** in the terminal reader. It has typing echo, Markdown formatting commands, undo and redo, and find and replace. Autosave snapshots are offered back after a crash.
- **Notes and highlights.** Add, list, step through, and delete notes. Highlight the selection or sentence. Notes and highlights move with your edits.
- **Quitting asks first.** It says "Quit textweaver? y or n". Press y to quit; n, a, or Escape cancels. Deleting a note asks the same way.
- **Single-key shortcuts can be turned off** (F9), so dictation and typing never trigger commands. Every command stays reachable with a modifier key or the command palette.
- **Library** support: library folders, recent files, and a bookshelf. `tw library` and `tw marks` list them. `tw migrate-star` imports settings, positions, bookmarks, notes, highlights, and the bookshelf from Star.
- **Audio export.** `tw export-audio` writes WAV, or MP3 and M4B with chapters when ffmpeg is installed. It also writes SRT or WebVTT subtitles with sentence or word cues.
- **speech-dispatcher backend** (Linux), with an index mark before every word for exact highlighting.
- **One engine host for all out-of-process engines.** It runs Eloquence and SAPI5 in both 64-bit and 32-bit builds. `cargo xtask hosts` builds them all.
- **Obsidian vault export and import** (`tw vault`) and **dictation** through a Whisper subprocess (`tw dictate`).
- **JSON-RPC server** (`tw serve --stdio`) and the in-process reader (`tw open`).
- **Community pronunciation lexicon** from the IBMTTS dictionaries. It is off by default and used only with engines that do not normalize text themselves.
- **Release packaging:** `cargo xtask dist`, and a workflow that builds the macOS package.

### Changed

- Speech statuses carry a reading generation, so a status from an earlier reading can never move the highlight.
- Code Factory's Eloquence (installed with some screen readers) is used only when you opt in with `TEXTWEAVER_ECI_CODE_FACTORY=1`. Its SAPI voices are hidden unless you opt in the same way.

## [0.1.0-alpha.2] - 2026-09-25

### Added

- **Apple speech on macOS.** There are two backends. `nsspeech` uses the classic engine and is the quickest to respond. `avspeech` uses AVSpeechSynthesizer and gives exact word highlighting. Both prefer Eloquence Reed when it is installed.
- `--voice` accepts plain names such as `Reed` or `Zira`.
- **SAPI5 voices on Windows,** including OneCore voices, with word highlighting.
- **Eloquence through its ECI engine,** with exact word timing:
  - on Windows, the 64-bit OpenEVV library, or a 32-bit `eci.dll`;
  - on Linux, Voxin.
- `tw eloquence` explains how to get Eloquence and shows what textweaver found.

## [0.1.0-alpha.1] - 2026-09-25

### Added

- **The first working reader.**
  - `textweaver FILE` reads text, Markdown, and HTML aloud. The highlight follows the spoken word.
  - Move by character, word, sentence, line, paragraph, heading, table, list, and link.
  - Speech Cursor mode, bookmarks, find, and navigation history.
  - Your position is restored when you reopen a document.
  - Keys are configurable.
- `tw`, the command-line tool: `text`, `info`, `search`, `speak`, `voices`, and `backends`.
- Speech backends: espeak-ng (Linux), Omnivox, and a silent backend.

[0.1.0-alpha.3]: https://github.com/leavesofgrass/textweaver/releases/tag/v0.1.0-alpha.3
[0.1.0-alpha.2]: https://github.com/leavesofgrass/textweaver/releases/tag/v0.1.0-alpha.2
[0.1.0-alpha.1]: https://github.com/leavesofgrass/textweaver/releases/tag/v0.1.0-alpha.1
