# Changelog

All notable changes to textweaver. Versions follow [Semantic Versioning](https://semver.org/); before 1.0 every release is an alpha and anything may change.

## [Unreleased]

### W4g: authoring extras

- Markdown lint in edit mode: Ctrl+F8 and Ctrl+Shift+F8 select the next or previous problem and say it, "Lint: heading level 3 after level 1; use level 2." Five rules: heading levels, list markers, trailing spaces, link references without a definition, and bare web addresses. `tw lint FILE...` checks files and exits 1 when there are problems. textweaver's own rules, not rumdl (ADR-0032).
- Copying works in terminals without OSC 52 (the old Windows console, macOS Terminal, GNOME Terminal and other VTE terminals): textweaver puts the text on the system clipboard itself and says so the first time. Over SSH and in tmux it still uses the terminal.
- Math as Unicode in the reading view (`x²`, `√2`, `1⁄2`), as Star showed it: `[reading] math_display = "unicode"`, or "Math on screen" in the settings list. Speech, edit mode, and math exploration use the source. Try `fixtures/g/math.md`.
- Notes and highlights export as BibTeX, BibLaTeX, RIS, or CSL-JSON records for Zotero or Pandoc: `tw marks FILE --export ris --output notes.ris`.
- Moving the caret onto a code block's first line names its language: "code, Python".
- Held for the owner's decision on RUSTSEC-2025-0141 (bincode unmaintained): grammar checking with Harper (Ctrl+F7, branch `wave4/g-grammar-harper`) and code highlighting with syntect (branch `wave4/g-highlight-syntect`).

### W4c1: MathCAT speech

- Math can be spoken by MathCAT, the engine NVDA and JAWS use, in ClearSpeak or SimpleSpeak, at the math verbosity, in the document's language: `[reading] math_engine = "mathcat"` or `"mathcat_simplespeak"` ("Math speech" in the settings screen). It needs a build with the new `mathcat` feature, which is off by default; textweaver's own math speech stays the default and the fallback. The highlight covers the whole formula while MathCAT reads it (ADR-0029).
- EPUB 3 books: MathML is read as math, using the book's TeX when a formula carries it; a formula with only `alttext` is read as that text, and an `epub:switch` is read once.
- New crate `textweaver-mathcat` on MathCAT 0.7.6-rc.3, pinned exactly. Braille (Nemeth and UEB) waits for MathCAT issue #827 to be fixed in a release.

### W4b: speed and memory

- Zip archives, and the EPUB and Word files built on zip, open whatever compression their members use: deflate, bzip2, LZMA, XZ, and PPMd, all in pure Rust. A member that says it is larger than 256 MB is refused before it is unpacked.
- Words and sentences are found two to three times as fast (ICU4X's segmenters), with the same boundaries. Planning a whole 10 MB document for reading takes about half the time it did.
- The reader asks the system for its light or dark setting only when that can change the theme, and no longer waits for the answer before building the reader.
- `--log debug` records how long the reader took to start.

### Keys: what changed

The default keys are now the quick navigation keys of NVDA's and JAWS's browse mode (Jon's decision). `preset = "classic"` under `[keyboard]` keeps the earlier keys; `preset = "screen-reader"` now means the default. Old key, then where its command went (terminal):

- `.` next sentence: `Alt+Down` or `Alt+.`. `.` now says the sentence.
- `,` previous sentence: `Alt+Up` or `Alt+,`. `,` now says the paragraph.
- `s` say the sentence: `.` or `Alt+Shift+S`. `s` is the next separator.
- `Shift+S` say the paragraph: `,`. `Shift+S` is the previous separator.
- `l` say the line: `Alt+Shift+L` (GUI `Ctrl+L`). `l` is the next list.
- `o`, `Shift+O` lists: `l`, `Shift+L`.
- `Shift+H` history back: `Backspace` or `Alt+Left`. `Shift+H` is the previous heading.
- `Shift+L` history forward: `\` or `Alt+Right`. `Shift+L` is the previous list.
- `k` scroll up: `Shift+J`. `k` is the next link.
- `Shift+K` link address: `Alt+Shift+K`. `Shift+K` is the previous link.
- `q`, `Shift+Q` quit: `Ctrl+Q`, which still asks. `q` is the next block quote.
- `Alt+Down`, `Alt+Up` notes: `F12`, `Shift+F12`, or `e`, `Shift+E`. `Alt+Down` and `Alt+Up` move by sentence.
- `'` and `"` notes: removed (`"` is Shift+2 on most non-US layouts).
- `Alt+Shift+D` add a reference (terminal): `Alt+B`, because Windows Terminal splits panes with it. The GUI keeps `Alt+Shift+D`.
- RSVP faster and slower: `Alt+Shift+PageUp` and `Alt+Shift+PageDown` as well, because Windows Terminal resizes panes with `Alt+Shift+Up` and `Alt+Shift+Down`.
- New: `g` graphics, `d` sections or chapters, `Ctrl+Down` and `Ctrl+Up` paragraphs, `Alt+Shift+Q` citations, `Alt+Shift+X` explore math, `Alt+Shift+Z` syllables, `Alt+Shift+J` difficult words. `p` still moves by paragraph; `Shift+W` still says the position.
- `1` to `6` and Shift with them follow the physical digit key on any layout: read from the console on Windows; the shifted digits of the US, UK, German, Spanish, Nordic, and Italian layouts elsewhere, and French with `[keyboard] digit_row = "azerty"`.

See [docs/keyboard.md](docs/keyboard.md#what-changed).

### W4s: the Xilem GUI before Jon's listening session

- `textweaver-xilem --announce uia` (Windows) announces with UI Automation Notification events instead of the live region, for comparing the two in NVDA and JAWS. `announce = "uia"` in a `[gui]` table of `settings.toml` does the same. The default is still the live region.
- List options and settings scrolled out of view are now in the accessibility tree, so a screen reader's object navigation reaches them: all 15 settings sections, and every option of a long list.
- Lists and the settings form say "1 of 15" (they said "2 of" with no total), and rows scrolled into view are drawn instead of blank.
- The UI Automation report checks both announcement paths and a long list scrolled to its end. See [ADR-0028](docs/adr/0028-xilem-gui-after-the-session.md), which also records where the GUI's memory goes (the graphics stack; the app itself is about 11 MB).

### W4a2: the Xilem GUI after the first screen reader session

- The live region stays the default for announcements, and the spoken word's background color stays the highlight. `[gui] announce = "uia"` is now a real setting, in the settings dialog under "Window"; `--announce` and `--select-spoken` stay as options. ADR-0028 is accepted.
- A long document's window slides while reading without losing the screen reader's place: the text that stays keeps its nodes, and the caret stays on the spoken word.
- The reading aids in the window: text spacing, the reading ruler and current line, bionic reading, difficult words, and RSVP in its own strip under the document. The RSVP word is never spoken by itself; a quiet status beside it says where you are.
- In a list, F1 and Alt+End repeat the list's introduction, as in the terminal. The status bar shows the terminal's title line: the reading state, the line, the mode, the rate, and the engine. "No document is open" names the Open key from the keymap.
- New guide: [docs/gui.md](docs/gui.md), also shipped in the GUI package as `GUI.md`.

### Usability pass: the terminal reader and `tw`

- A yes-or-no question ("Quit textweaver? y or n", "Delete this note? y or n", "Reload it? y or n", the Piper download and every other one) is now spoken even while textweaver is reading aloud. It went to the status line only, so a self-voicing user heard the reading go on and the next key press vanished into the question.
- The first run says a one-line welcome after the document opens: the keys that read, stop, move by heading, open the help, and quit, named from the keymap in effect.
- Startup warnings (an unreadable settings file, a bad keymap line) follow the "Opened" message instead of cutting it off, and share the status line with it.
- "Could not open" is one plain sentence: a missing file names the file and its folder, a folder says it is a folder, and no operating system error code is read aloud. `tw text`, `tw info`, and `tw search` say the same thing, and `tw open` refuses a folder before the reader starts.
- The F1 help names how to open a document and the library, the quick navigation keys (h, 1 to 6, l, i, t, k, q, s, g, d), the accessibility mode (Alt+Shift+A), single-key shortcuts (F9), the settings screen, choosing a voice, and restarting speech. Each line names at most two keys (the single key and a chord that works with single keys off), and a key bound in two layers is no longer read twice ("Tab or Tab").
- "No document is open. Press Control O ..." takes the key from the keymap.
- `textweaver --help` describes the reader and its first keys instead of "self-voicing ratatui frontend".
- `tw text big.pdf | head` no longer panics when the pipe closes; `tw define` no longer prints its failure twice; `tw marks` takes `--home` like the other commands.
- Docs: lists no longer close on `q` (a letter jumps to the next item), and Delete in the notes and highlights lists asks first. The findings and what is left are in [docs/research/usability-terminal.md](docs/research/usability-terminal.md).

### Terminal polish (Wave 4, Agent W4h)

- The title line says "Ready" until something is read, then "Stopped". A screen reader reading it at startup heard "Stopped".
- Keys named in messages are spoken by name by textweaver's own voice ("Control S", "Alt period"), so they are heard at every punctuation level; the status line and the screen reader keep the written form ("Ctrl+S"). Every key named in a message comes from the keymap.
- New: **Repeat message** (`'`, or `Alt+'` anywhere) says the last message again. **Say status** (`z`, or `Alt+End` anywhere) says the last message, then the mode, the reading state, the position, the rate, and the speech engine. Both are in the command palette and heard over the reading.
- In a list, **F1** or **Alt+End** says the list's introduction again (its name, how many items, the keys it takes), then the item you are on.
- **Escape** in edit mode with nothing being read says "Still editing. Ctrl+E finishes."
- The command palette says "Command. Type part of a name; Tab completes, Up and Down list matches." when it opens.
- Messages said while the speech engine starts ("Opened", a settings warning, the welcome) are said once it is ready, in order, instead of being lost.
- `tw` with no arguments prints a two-line hint and exits 0; `tw --help` keeps the full list. `tw search --json` and `tw info` no longer panic when a pipe closes.

### Added

- **Scanned pages (OCR).** Scanned PDFs and pictures (PNG, JPEG) are read by recognizing their text: English in process with the pure-Rust ocrs engine (models downloaded once with `tw ocr download`, 12.2 MB, after you agree), other languages with Tesseract when it is installed (`[reading] ocr_lang`). The recognized pages keep their headings, paragraphs, and page numbers. `tw ocr status` and `tw ocr read FILE`. See [ADR-0026](docs/adr/0026-ocr-and-student-formats.md).
- **More formats for students.** DAISY 3 books and DTBook (Bookshare zips too), PowerPoint slides with speaker notes, spreadsheets (CSV, TSV, ODS, XLSX) as tables, archives (ZIP, TAR, TAR.GZ, 7Z) with `book.zip!chapter.pdf` paths, and web pages (`tw open https://...`).
- **EPUB, Word, and PDF reading.** `textweaver` and `tw text` open EPUB (chapters from the table of contents), DOCX (headings, lists, tables, footnotes, alt text), and PDF. The PDF reader is pure Rust and on by default. It finds columns, removes running headers and page numbers, and recovers headings, lists, and tables. See [ADR-0010](docs/adr/0010-pdf-loader.md).
- **Conversion.** `tw convert` converts files and whole folders to Markdown, HTML, text, EPUB 3, Word, braille (BRF, UEB grade 1), and tagged PDF, on every core, skipping files already converted. It reads GFM, Obsidian, and Pandoc Markdown, turns LaTeX and ASCIIMath into MathML, uses MiniJinja templates, and can watch a folder. Pandoc is used only for formats with no native reader. See [docs/converting.md](docs/converting.md).
- **Bundled fonts and PDF layout.** Atkinson Hyperlegible Next and Mono and OpenDyslexic come with textweaver. PDF output uses them by default, and `tw convert` gains font, size, page size, margin, spacing, large print, title page, and contents options.
- **Math.** LaTeX and ASCIIMath are read aloud in natural English at three verbosity levels, with exact highlighting inside a formula. Prices such as "$5 and $10" are never taken for math. See [docs/math.md](docs/math.md).
- **Citations.** `tw cite` keeps a reference library: look up a DOI or ISBN, import and export BibTeX, BibLaTeX, RIS, and CSL-JSON, and format with CSL styles such as APA, MLA, Chicago, and IEEE. See [docs/citations.md](docs/citations.md).
- **Themes.** Star's 23 themes, every one checked for contrast, with Galaxy as the default. F5 cycles them; your own themes load from the `themes` folder; textweaver can follow the system's light, dark, or high-contrast setting. See [docs/themes.md](docs/themes.md).
- **Reading aids.** RSVP (one word at a time), bionic reading, the reading ruler, terminal text spacing, and the reading level. See [docs/reading-aids.md](docs/reading-aids.md).
- **DECtalk,** for a DECtalk you have installed and licensed, with word highlighting and exact subtitle timing. See [docs/dectalk.md](docs/dectalk.md).
- **Library list** (Alt+L): the documents in your library folders and your recent files.
- **A GUI spike** on wxDragon, with a Fonts dialog. It is not built by default. See [ADR-0014](docs/adr/0014-gui-toolkit.md).
- **Install scripts** for Linux (any distribution), macOS, and Windows, and update, speech-check, doctor, dev-check, and convert-folder helpers. See [scripts/README.md](scripts/README.md).
- **`cargo xtask bench`** times the reading and authoring hot paths.
- **Licence notices in every package.** `THIRD-PARTY-NOTICES.md` lists the Rust crates and the bundled data (fonts, SCOWL, the IBMTTS dictionaries, citation styles, and the Adobe font metrics), and `licenses/` holds the font and SCOWL licence files. The packages also carry every user guide and the offline pages in `docs/site/`.
- **Releases are built in CI**, Windows as well as macOS, with build provenance you can check with `gh attestation verify`. `cargo xtask release X.Y.Z` prepares a release and dates the changelog from the machine's clock.
- **A Linux package: an AppImage.** One file for x86_64 that runs on Debian, Ubuntu, Fedora, Arch, and most other distributions from 2022 on, with `textweaver`, `tw`, the Eloquence (Voxin) and DECtalk hosts, and the dictionaries. `--tw` runs `tw`; `--install` links `textweaver` and `tw` into `~/.local/bin` and adds a menu entry. A plain tarball is there for systems without FUSE. `scripts/install-linux.sh --release latest` downloads, checks, and installs either. See [docs/install.md](docs/install.md#linux).
- **Documentation.** A documentation index ([docs/README.md](docs/README.md)); new guides for reading, editing, notes, the library, speech, math, citations, audio export, the Obsidian vault, dictation, JSON-RPC, troubleshooting, and screen readers; an architecture guide; CONTRIBUTING.md; and interactive, accessible pages in `docs/site/` about the architecture, the speech pipeline, the keyboard, the features, and the reading aids. `tools/check_links.py` checks every link.
- **Settings export and import as JSON.** `tw settings export` saves every setting and key override to one JSON (or TOML) file; `tw settings import` checks it, backs up your files, and applies it, with `--dry-run` to preview each change. `tw settings path` and `tw settings reset` are new too. See [docs/settings.md](docs/settings.md).

- **Choose a voice** (Alt+V) lists the engine's voices; Enter selects one and speaks a sample.
- **A log file.** Warnings and errors go to `textweaver.log` in the state folder, rotated at 1 MB. `--log debug` (or `TEXTWEAVER_LOG`) logs more; `--log off` turns it off.
- **Files changed on disk.** Saving over a file that another program changed since you opened it asks first. A change to an open file with no unsaved edits offers a reload.
- **Speech recovers from an engine crash or a stall.** The reading goes on from the last word heard, and says "Speech restarted".
- **GFM task lists** say "checked" or "not checked", and the text of HTML in Markdown (`<p align>`, `<details>`, `<img alt>`) is read.
- `tw export-audio` uses your settings: voice, rate, pitch, volume, the preferred engine, table and footnote modes, and the `[export]` subtitles.
- **Restart speech** (Shift+F8) starts speech again with your current settings. When speech stops working, textweaver restarts it once by itself and says "Speech restarted."
- **Positions survive outside edits.** When a file changed in Obsidian, git, or another editor, your place, bookmarks, notes, and highlights are found again from the text they were on, and textweaver says once what moved and what it could not find.
- `[editing] undo_steps` and `undo_memory_mb` cap the undo history (1,000 steps or 50 MB by default).
- **Screen reader modes.** `[accessibility] mode` chooses who speaks: self-voicing (textweaver speaks everything), hybrid (textweaver reads documents aloud; your screen reader speaks messages, typing, and caret moves from the status line), or screen reader (textweaver is silent, and Space steps through the text a sentence at a time on the status line). Alt+Shift+A cycles and saves it, and `--mode` sets it for one run. On its first run with a screen reader, textweaver offers hybrid once. A quiet-screen option, a status-line cursor option, and a screen-reader keymap preset (`[keyboard] preset = "screen-reader"`) help too. See [docs/screen-readers.md](docs/screen-readers.md).
- **Structure while writing.** In edit mode, headings, lists, links, and tables can be moved through, and "say position" names the heading, in the text you are writing.
- **The outline** (Alt+O) lists the headings; type to filter them, and Enter jumps.
- **Spell check** on a built-in word list (SCOWL): next and previous misspelling (Alt+M, Alt+Shift+M), each spelled aloud; suggestions and your own word list (Alt+J); and a count of misspellings on save.
- **Citations while writing.** Alt+C inserts a citation from your library, with a page; Alt+Shift+D adds a reference by DOI or ISBN; the palette inserts a bibliography, checks citations, and imports references. `tw cite` gains `remove`, `styles`, and `check`.
- **Export and preview from the reader.** Palette commands export the open document, or the text you are editing, to HTML, PDF, Word, EPUB, or braille. "Preview in browser" writes a web page with MathML and rewrites it on each save. "Listen rendered" reads the text as it will render.
- **Citations and math in converted documents.** `tw convert` formats Pandoc citations in a CSL style and adds a References section (`--bibliography`, `--style`, `--no-citations`), and typesets math in PDF, Word, EPUB, and braille.
- **New documents from a template,** with front matter, the date, and a References heading (`[editing] author`).
- **A study sheet:** your notes and highlights as Markdown, grouped by the document's headings. A note is signalled with a sound when reading reaches it.
- **Reading and writing quick wins:** keys 1 to 6 jump to the next heading of that level; a word count; the address of the link at the cursor; following a link or footnote (Alt+Shift+F), wiki links included; table rows and cells (Ctrl+Alt+arrows); Enter continues a list; Tab and Shift+Tab move between table cells in edit mode; copy (Ctrl+C) to the system clipboard through the terminal (OSC 52); in edit mode cut, paste, select all, and deleting a word; Add note (Alt+N) in edit mode; typing echo cycled with Shift+F9; verbosity and punctuation cycled with Alt+Shift+V and Alt+Shift+N.
- **Favourite voices:** Space in the voice list marks one, and favourites are listed first. `[highlight] color` and `sentence_color` now colour the reading highlight, with a contrast warning.
- **Markdown:** wiki links, GFM alerts, math, and heading attributes are read, and strikethrough and horizontal rules are announced.
- `tw info --exact` counts words and sentences with the reader's full segmentation.
- **Citations in continuous reading** are skipped by default and said in words with Alt+Shift+Q (`[reading] citations = "off" | "words"`), with the highlight exact both ways. See [docs/citations.md](docs/citations.md#citations-in-continuous-reading-altshiftq).
- **Explore math** (Alt+Shift+X): move through a formula term by term, into fractions, scripts, and roots and back out, each part said and highlighted. See [docs/math.md](docs/math.md#exploring-a-formula-part-by-part).
- **Syllables and difficult words in the terminal reader** (Alt+Shift+Z, Alt+Shift+J): words drawn split into syllables with exact highlights, and rare words underlined and named on word moves at high verbosity.
- **Preview.** After a save, "Preview updated. Press F5 in the browser." `[preview] auto_reload` serves the preview from 127.0.0.1 with a secret address and reloads it after each save, landing on the heading nearest the caret; `live` also reloads after a typing pause. Both off by default. See [docs/editing.md](docs/editing.md#preview-in-the-browser).
- **Export progress.** A long export says "Still exporting to PDF, 2 seconds." and then every ten seconds.
- **Writers.** PDF output draws strikethrough; Word equations (OMML) read back from DOCX as LaTeX math.
- **GUI on macOS** says once when a built-in font falls back to a system font.
- **A settings screen** (Shift+F10, or "settings" in the palette): every setting with its value, filtered as you type; Left and Right change a value, Enter types one, Delete puts the default back. Each change is said and saved. See [docs/settings.md](docs/settings.md#the-settings-screen).
- **Large files open in the background.** A file of 512 KB or more opens on a helper thread: "Opening report.pdf. Escape cancels.", then "Still opening report.pdf, 3 seconds." The keys keep working, and Escape stops waiting.
- **JSON-RPC: lists, prompts, and settings.** `list_state`, `list_key`, `prompt_state`, `prompt_key`, `settings_schema`, `get_setting`, and `set_setting`. The server is woken as each word is heard instead of polling. See [docs/json-rpc.md](docs/json-rpc.md).
- **The app core for the GUI** (Wave 3): a document window of about 500,000 characters around the reading, a waker, an edit command for native text controls, and the settings schema. See [ADR-0024](docs/adr/0024-app-core-for-the-gui.md).

### Changed

- **Nothing slow waits on the keyboard.** The speech engine starts in the background, so the reader is ready at once and speaks when the engine is; `settings.toml` is written by the writer thread; and the misspelling count after a save follows a moment later instead of holding up the keys (0.6 s on 10 MB).
- **Say position moved** from `%` to Shift+W and Alt+Shift+Y.
- `tw speak`, `tw voices`, and `tw backends` read your settings, and take `--home`.
- Save As suggests a name from the first heading or the title, and asks before replacing a file. A crash, a closed terminal, Ctrl+C, or a stop signal saves your place and your unsaved work, and restores the terminal.
- Find and replace goes one match at a time, with match case and whole word choices.
- Entering edit mode on a 10 MB file takes about 50 ms instead of 276 ms, with less memory.
- Malformed or hostile files (deep nesting, impossible list and page numbers, binary files) can no longer stop a `tw convert` batch; Pandoc runs sandboxed, with a two-minute timeout.
- `tw info` is about three times faster on large files.
- **Nothing waits for the disk.** Saving, autosave, positions, bookmarks, notes, the library sidecars, and the check for a changed file run on one background writer; "Saved" is said when the file is written, and quitting waits for it (saying so if the disk is slow).
- **Nothing waits for the speech engine.** A restarting engine helper, the first 32-bit SAPI voice, and the audio device start in the background, so Stop and Pause always work at once; speech-dispatcher is never waited for after connecting; DECtalk is synthesized a sentence at a time, so Stop takes effect quickly.
- **Voices are listed once**, when the engine starts, and Choose Voice (Alt+V) opens at once, or says the voices are still loading and opens when they arrive.
- **Find** reads the document in pieces and keeps at most 10,000 matches around the cursor while counting them all; edit-mode Replace searches once per step and no longer builds a string per character.
- **The library** (Alt+L) is scanned in the background, with a count as it goes.
- Engine availability is checked once per run, not twice by `tw backends`.
- espeak-ng is loaded when textweaver starts instead of being linked in, so a build with the `espeak` engine runs with or without espeak-ng installed, and building it needs no espeak-ng development files. `TEXTWEAVER_ESPEAK_LIBRARY` names the library to load.
- Rate, pitch, and volume changes while reading are heard at once, on Eloquence and SAPI too, not two or three sentences later.
- Opening and reading large documents is much faster: on a 10 MB file, open to first speech went from 13 seconds to under a quarter of a second.
- Reading a long document starts at once: continuous reading is planned about ten minutes at a time. On a 10 MB file, next sentence while reading went from about 250 ms to 3 ms.
- The highlight is drawn as soon as its word is heard, not up to 40 ms later.
- A list says its first item after its title, and each item says where it is ("Chapter two, 2 of 5").
- The same message twice in a row is spoken twice by a screen reader: the status line blanks for a moment first.
- Entering edit mode on a large file is two to four times faster.

### Fixed

- An engine that keeps failing no longer floods you with errors: reading stops after three failures in a row.
- A damaged per-document state file or recent-files list is set aside as a `.bak` file instead of being overwritten, so notes and bookmarks are not lost.
- Saving writes through symbolic links, keeps file permissions, and refuses read-only and non-UTF-8 files (use Save As).
- F9 and the `[keyboard] character_keys` setting work, and Shift with the arrow keys selects.
- A speech volume above 100 in `settings.toml` is now set to 100 and reported, like other out-of-range values.
- Bookmarks and notes landed in the wrong place after saving and then quitting in edit mode.
- `[normalization] table_mode` and `footnote_mode` had no effect.
- Jumping to an ordered list item says "2. Walk the dog", not just "2.".
- AltGr characters (`@ [ ] { } \ | ~ €`) can be typed in edit mode and prompts on non-US Windows keyboards.
- Binary files (a PDF or Word file without its reader, a program) are refused with a clear message instead of being read as garbage; UTF-16 text without a byte order mark is decoded.
- speech-dispatcher counted as not available where `XDG_RUNTIME_DIR` is unset (containers), though `spd-say` worked.
- The `espeak` feature builds on Fedora 44 and current Arch: its FFI no longer runs bindgen over the system headers.
- Backspace and Delete remove a whole character (an emoji with its skin tone, a flag, a letter with its accent), not a piece of one.

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
