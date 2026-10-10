# textweaver documentation

<img src="assets/textweaver-logo.svg" alt="textweaver logo: the letters t and w woven on a loom" width="128" height="128">

textweaver is a document reader and writer that speaks for itself. It reads aloud with a highlight that follows the spoken word, works from the keyboard in a terminal and in a window, and converts, exports, and edits documents. It is its own project, with roots in star, the Python reader that came before it.

This is the index of every textweaver document. It is grouped for three audiences:

- [People who use textweaver](#for-users): reading, writing, speech, and the tools.
- [People who work on textweaver](#for-contributors): how it is built, tested, and released.
- [The design decisions](#decisions): the architecture decision records (ADRs).

New here? Start with the [quick start](quickstart.md).

There are also [interactive pages](#interactive-pages) that explain textweaver with diagrams you can explore by keyboard. They work offline in any browser.

## Start here

- [For students](start-students.md): from download to hearing a document.
- [For accommodation staff](start-staff.md): converting course material, preparing computers, and working without internet.
- [What is new](whats-new.md): each release in plain words.
- [Known limits](known-limits.md): what does not work yet, and what has not been tested.
- [Privacy](privacy.md): what stays on your computer, and when textweaver uses the network.
- [Accessibility statement](accessibility.md): what has been tested, how, and what has not.

## For users

### Getting started

- [Quick start](quickstart.md): your first 30 seconds on Windows, macOS, and Linux.
- [Installing textweaver](install.md): the packages, the install scripts, and checking a download.
- [Using textweaver with a screen reader](screen-readers.md): the three accessibility modes (self-voicing, hybrid, and screen reader), settings for NVDA and JAWS, and terminal keys that clash.
- [Troubleshooting](troubleshooting.md): the log file, diagnosing speech, and common problems.

### Reading

- [Reading and moving around](reading.md): opening files, reading aloud, moving by unit, Speech Cursor, find, go to, and history.
- [Keyboard reference](keyboard.md): every key in the terminal and the GUI, generated from the keymap.
- [The textweaver window (GUI)](gui.md): starting it, what is in the window, its keys, announcements, and the reading aids it draws.
- [The window in pictures](window-in-pictures.md): eight pictures of the window, each with a description of what it shows.
- [Bookmarks, notes, and highlights](notes.md): marking your place and your thoughts.
- [Reading aids](reading-aids.md): RSVP, bionic reading, the reading ruler, text spacing, fonts, and reading level.
- [Themes](themes.md): the 24 built-in color themes, following your system, and writing your own.
- [Math](math.md): hearing math read aloud, and writing it in Markdown.

### Writing

- [Writing and editing](editing.md): edit mode, typing echo, Markdown commands, undo, saving, the outline, spell check, clipboard, templates, and find and replace.
- [Citations](citations.md): inserting citations while writing, `tw cite`, the reference library, DOI and ISBN lookup, and citation styles.
- [Dictation](dictation.md): `tw dictate`, turning speech in an audio file into text.

### Speech

- [Speech engines and voices](speech.md): the engines, choosing a voice, rate, pitch, and volume.
- [Getting ETI-Eloquence](eloquence.md): Eloquence on macOS, Linux, and Windows.
- [Using DECtalk](dectalk.md): a DECtalk you have installed and licensed.
- [Audio export](audio-export.md): `tw export-audio`, audiobooks with chapters, and subtitles.

### Files, the library, and settings

- [Converting documents](converting.md): `tw convert` to HTML, EPUB, Word, braille, PDF, and more; exporting and previewing from inside the reader.
- [The library](library.md): library folders, recent files, sync between computers, and importing from star.
- [Optional components](components.md): the models, fonts, and voices textweaver downloads only when you agree; Manage optional components, the first-run list, `tw components`, installing from a file, and a mirror.
- [Syncing between computers](sync.md): notes, highlights, bookmarks, places, statistics, and settings through a folder you choose; what never syncs, setting it up, what you hear, and privacy.
- [The Obsidian vault](vault.md): exporting notes and highlights to a vault, and importing them back.
- [Settings](settings.md): where settings live, every setting, and export, import, and reset.
- [Settings reference](settings-reference.md): every setting with its default, label, help, and values, generated from the settings schema.
- [The command line](command-line.md): the rules every `tw` command follows: `--out`, `--to`, `--json`, `--home`, questions and `--yes`, exit codes, and error lines.
- [Scripts](../scripts/README.md): install, update, speech check, doctor, and folder conversion.

## For contributors

The developer documents are in [dev/](dev/), and the decision records in [adr/](adr/README.md).

### Building and working on textweaver

- [CONTRIBUTING.md](../CONTRIBUTING.md): what textweaver is for, setting up, the checks, accessibility expectations, commit style, and how to propose a change.
- [Good first issues](dev/good-first-issues.md): small, well-scoped tasks for a first contribution, each with where to look and how to check it.
- [Writing messages](dev/messages.md): the style guide for everything textweaver says and shows, and the tests that keep it.
- [Building](dev/building.md): Rust, Python, what each system needs, Docker, the GUI, the lean reader, the helper scripts, and the repository layout.
- [Testing](dev/testing.md): the checks every change must pass, the tests, and the [benchmarks](dev/testing.md#benchmarks).
- [The polish checklist and its tests](dev/checklist-tests.md): each row of the consistency, accessibility, visual, and documentation checklist, with the test that holds it or the owner's session that covers it.
- [Architecture](dev/architecture.md): the crates, the dependency rules, the threads, and the path from a file to a highlighted word.
- [The eSpeak NG helper](dev/espeak-helper.md): how eSpeak NG runs in its own helper program, how textweaver chooses between it and the in-process backend, and the measurement behind the choice.
- [The crates](dev/architecture.md#the-crates): what each of the 37 crates does, with its ADRs.
- [CI](../CONTRIBUTING.md#ci): the workflows and what they check.
- [Docker development container](dev/docker.md): building and testing Linux features on any machine, and Voxin.
- [Fuzzing](../fuzz/README.md): the 35 cargo-fuzz targets, run every night: the document loaders (RTF, ODT, Word revisions, LaTeX, Obsidian, JSON, SVG, and email among them), PDF annotations, the math and citation parsers, themes, the lexicon, vault import, JSON-RPC, the settings and state files, the sync folder's records and group files, and the engine-host protocol.
- [Releasing](dev/releasing.md): making a release, the Linux AppImage, and what the packages hold.
- [Third-party data](dev/third-party-data.md): the bundled pronunciation dictionaries, fonts, and word lists, and their licenses.
- [JSON-RPC](json-rpc.md): driving textweaver from an editor or another program with `tw serve --stdio`.
- [Roadmap](roadmap.md): what works today, what is being built next, and what is planned.
- [Research for the next waves](dev/research/README.md): the research reports and the wave plan for alpha.8, alpha.9, and later: performance, speech engines, use cases, design, and law and standards.
- [star features not yet planned](star-gaps.md): star features with their status in textweaver.
- [Documentation coverage](dev/docs-coverage.md): each feature, the guide that covers it, and whether the guide passes.
- [CHANGELOG.md](../CHANGELOG.md): what changed in each release.

## Decisions

Each ADR records one decision: the context, the choice, and its consequences. A dated status update under the date says what changed later. [The ADR index](adr/README.md) lists them with their statuses.

- [ADR-0001: Workspace layout and dependency policy](adr/0001-workspace-and-dependencies.md): one crate per job, one table of approved dependencies, and no async runtime in speech.
- [ADR-0002: Text model](adr/0002-text-model.md): canonical text in a rope plus markers, and character positions.
- [ADR-0003: Speech threading and event timing](adr/0003-speech-threading-and-event-timing.md): the speech thread, generations, word timing, and pause.
- [ADR-0004: Rate, pitch, and volume](adr/0004-rate-pitch-volume.md): engine-independent voice settings.
- [ADR-0005: Narration and the OffsetMap](adr/0005-narration-and-offset-map.md): spoken text built together with its map back to the document.
- [ADR-0006: Keymap, actions, and announcements](adr/0006-keymap-and-actions.md): actions, layers, overrides, and announcing every change.
- [ADR-0007: ETI-Eloquence through an ECI host process](adr/0007-eloquence-via-eci-host.md): Eloquence with exact word timing, and its licensing.
- [ADR-0008: Apple speech on macOS](adr/0008-apple-speech.md): the `nsspeech` and `avspeech` backends.
- [ADR-0009: SAPI5 voices on Windows](adr/0009-sapi5-voices.md): 64-bit and 32-bit voices in host processes.
- [ADR-0010: PDF loader](adr/0010-pdf-loader.md): a pure Rust PDF reader with column-aware reading order.
- [ADR-0011: Audio export](adr/0011-audio-export.md): sentence-by-sentence synthesis, exact subtitles, and chapters.
- [ADR-0012: One engine-host protocol and playback client](adr/0012-engine-host.md): the shared protocol for Eloquence, SAPI5, and DECtalk.
- [ADR-0013: Dictation through a Whisper program](adr/0013-dictation.md): voice typing with a Whisper subprocess.
- [ADR-0014: GUI toolkit (wxDragon)](adr/0014-gui-toolkit.md): the GUI spike's findings. The GUI moved to Xilem; the wxDragon spike was removed once the Xilem GUI passed its second screen-reader session.
- [ADR-0015: JSON-RPC server](adr/0015-json-rpc.md): `tw serve --stdio`, its methods, and notifications.
- [ADR-0016: Rendering and bulk conversion](adr/0016-rendering-and-conversion.md): Markdown to accessible HTML, and fast, incremental conversion.
- [ADR-0017: Native writers](adr/0017-writers.md): EPUB 3, DOCX, BRF braille, and tagged PDF, and their accessibility checks.
- [ADR-0018: Math](adr/0018-math.md): LaTeX and ASCIIMath parsing, MathML, and spoken math.
- [ADR-0019: Citations](adr/0019-citations.md): the reference library, lookup, and CSL formatting.
- [ADR-0020: Themes](adr/0020-themes.md): star's palettes, contrast rules, and output for every frontend.
- [ADR-0021: DECtalk through a host process](adr/0021-dectalk.md): DECtalk with word timing, and its licensing.
- [ADR-0022: Reading aids](adr/0022-reading-aids.md): RSVP, bionic reading, spacing, fonts, the ruler, and more, as pure data.
- [ADR-0023: Piper voices and Whisper dictation in-process on RTen](adr/0023-in-process-neural-speech.md): neural voices with word timing from the model, and in-process dictation, on a pure-Rust ONNX runtime.
- [ADR-0024: App core for the GUI](adr/0024-app-core-for-the-gui.md): the document window, shared list and prompt state, the waker, the replace-range edit, the settings schema, and work moved off the input thread.
- [ADR-0025: Define word offline, and the message catalog](adr/0025-lexicon-and-message-catalog.md): Open English WordNet and CMUdict in an fst and zstd file, and a Fluent-subset catalog with pseudo-locales.
- [ADR-0026: OCR, and formats for students](adr/0026-ocr-and-student-formats.md): OCR of scanned pages (ocrs in process, Tesseract as the fallback), DAISY, PowerPoint, spreadsheets, archives, and web pages.
- [ADR-0027: Xilem GUI](adr/0027-xilem-gui.md): the all-Rust GUI on Masonry, Vello, Parley, AccessKit, and winit; our own document widget, the pinned versions, and the accessibility checks.
- [ADR-0028: The Xilem GUI after the first listening session](adr/0028-xilem-gui-after-the-session.md): two ways to announce, every list option in the tree, and where the GUI's memory goes.
- [ADR-0029: MathCAT speech](adr/0029-mathcat-speech.md): MathCAT as a second math speech engine, EPUB 3 MathML read as math, and what waits for math braille.
- [ADR-0030: Interface translations](adr/0030-interface-translations.md): every message from the catalog, five built-in languages, right-to-left display, and a voice per language.
- [ADR-0031: Native RTF, ODT, and Word revisions](adr/0031-native-rtf-odt-and-word-revisions.md): RTF and OpenDocument without Pandoc, comments as notes, tracked changes, and limits for zip packages.
- [ADR-0032: Grammar, lint, highlighting, and clipboard crates](adr/0032-grammar-lint-highlighting-clipboard.md): the authoring extras' crates, and two held for later review.
- [ADR-0033: The GUI after further accessibility testing, and edit mode](adr/0033-gui-session-2-and-edit-mode.md): the file chooser, text size and font keys, a key on every button, and edit mode in the window.
- [ADR-0034: The rope after measurement](adr/0034-rope-after-measurement.md): stay on ropey 1.6; ropey 2 and crop measured, and when to look again.
- [ADR-0035: Native LaTeX subset, and email and web archives](adr/0035-latex-email-and-web-archives.md): LaTeX, email, and web archives read natively, and MathML in web pages.
- [ADR-0036: Math braille and navigation on MathCAT](adr/0036-math-braille-and-navigation.md): Nemeth and UEB math in braille files, and exploring a formula with MathCAT and its braille.
- [ADR-0037: Extractive summaries without a model](adr/0037-extractive-summaries.md): LexRank in-house, `tw summarize` and Summarize, difficult-word definitions, and the RSVP flash check.
- [ADR-0039: Automated screen-reader checks beside the listening sessions](adr/0039-automated-screen-reader-checks.md): the accessibility tree on three systems, and NVDA, Orca, and VoiceOver sessions on CI runners.
- [ADR-0041: Publishing templates, real Word footnotes, and PDF page labels](adr/0041-publishing-templates.md): APA, AMA, and reading templates for EPUB, Word, and PDF, Word footnotes, the EPUB cover, and print page labels.
- [ADR-0042: Streaming dictation and dictation in the reader](adr/0042-streaming-dictation.md): words shown while you talk, each phrase typed at its pause, a gate so dictation is never slower, and the Dictate command in edit mode.
- [ADR-0043: Menus and the palette from one model](adr/0043-menus-and-the-palette-from-one-model.md): menus from one model, the palette's names and ranking, interface announcements, macOS keys, and colors.
- [ADR-0044: Obsidian, JSON, SVG and content MathML in the reader](adr/0044-obsidian-json-svg-and-content-mathml.md): Obsidian notes, JSON and notebooks, SVG drawings, content MathML, and more LaTeX, read natively.
- [ADR-0045: A file browser on the list model](adr/0045-a-file-browser-on-the-list-model.md): folders and archives browsed as one list, a preview, and choosing folders for other commands; it never changes a file.
- [ADR-0046: Native menus in the GUI](adr/0046-native-menus-in-the-gui.md): the window's menus from the app's one model, native on Windows and macOS, a list on Linux, with the keys shown and handled by the keymap alone.
- [ADR-0048: PDF annotations, links and forms](adr/0048-pdf-annotations-links-and-forms.md): PDF comments as notes, links, filled-in forms, captions, and sideways and tabular scans.
- [ADR-0049: Sync beyond the place](adr/0049-sync-beyond-the-place.md): notes, highlights, bookmarks, places, and portable settings synced through a folder you choose, each computer writing only its own files, with no account or server.

## Interactive pages

The pages in `docs/site/` explain textweaver with diagrams and demonstrations. Each diagram has a full text version next to it, every control works from the keyboard, and changes are announced to screen readers. Open `docs/site/index.html` in any browser; nothing is downloaded.

- [Overview](site/index.html): what textweaver is, and a map of the other pages.
- [Architecture](site/architecture.html): choose a crate to hear its job, its ADRs, and what it depends on.
- [Speech pipeline](site/speech-pipeline.html): step from a key press to a highlighted word, with a worked offset map.
- [Keyboard](site/keyboard.html): search and filter every key by category, layer, frontend, and platform.
- [Features](site/features.html): everything textweaver does, with its status.
- [Reading aids](site/reading-aids.html): try RSVP, bionic reading, and the reading ruler.
- [About the pages](site/README.md): how they are made and regenerated.

## See also

- [README](../README.md): what textweaver is, and how to build it.
- [Quick start](quickstart.md): the fastest way in.
