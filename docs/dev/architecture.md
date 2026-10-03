# Architecture

This guide explains how textweaver is put together: which crate does what, which way the crates depend on each other, which threads and processes run, and how a file on disk becomes a spoken word with its highlight. It is for contributors, and for anyone who wants to know why the highlight never drifts. Each part links to the decision record (ADR) that explains the choice, and to the crate that holds the code.

The [interactive architecture page](../site/architecture.html) shows the same crate map: choose a crate to see its job, its ADRs, and what it depends on. The [speech pipeline page](../site/speech-pipeline.html) steps through the data flow below one stage at a time.

## The big picture

textweaver is one Cargo workspace with 36 crates and a maintenance crate, `xtask`. Two programs come out of it:

- `textweaver`, the terminal reader, built from `crates/textweaver-tui`;
- `tw`, the command-line tool, built from `crates/textweaver-cli`.

A third program, `textweaver-xilem`, is the GUI, on Xilem, Linebender's all-Rust toolkit ([ADR-0027](../adr/0027-xilem-gui.md)). The first GUI, a feasibility spike on wxDragon ([ADR-0014](../adr/0014-gui-toolkit.md)), was removed once the Xilem GUI passed its second screen-reader test session.

Three helper programs run speech engines in their own processes: `textweaver-eci-host` (Eloquence), `textweaver-sapi-host` (SAPI5 voices), and `textweaver-dectalk-host` (DECtalk), each in a 64-bit build and, on Windows, a 32-bit `-x86` build. `cargo xtask hosts` builds them.

Everything a user does goes through one application core, `textweaver-app`. The terminal reader, the GUI, and the JSON-RPC server (`tw serve`) are thin frontends over it. They turn keys or messages into commands, and draw or send what the core returns. Lists and prompts are the core's too: a frontend sends list and prompt keys, and the core moves the focus, filters, completes, and says "item, 3 of 12", the same everywhere.

## The crates

The crates are grouped here by the part of the system they serve. For each crate: what it does, the ADRs that decided it, and the workspace crates it depends on (normal dependencies only; test-only dependencies are left out).

### Foundation

- **`textweaver-core`**: shared leaf types. Positions (`CharPos`, `CharRange`), units, marker kinds, the `OffsetMap`, edits and how positions move across them, rate, pitch, and volume, the `Utterance` handed to speech, and small preference enums. Also the two helpers every crate shares: `fs::write_atomic`, the one atomic save (synced temp file, rename retried while Windows reports the file in use), and `process`, which finds programs on `PATH` by one rule, starts them without a console window, and decodes their output by one rule. ADRs: [0002](../adr/0002-text-model.md), [0005](../adr/0005-narration-and-offset-map.md). Depends on nothing in the workspace.
- **`textweaver-components`**: optional components (W8a-d): the models, fonts, and voices textweaver downloads only after the reader agrees. One pin type (size and SHA-256, or git's blob SHA-1), the one downloader (a `.part` file in a staging folder, the check, then a rename; files that check out are kept, a stopped download resumes, a lock keeps two downloads of one component apart), install from a zip or a folder, a mirror's manifest of extra components, the fake fetcher for tests, and the neutral User-Agent. HTTP is its `download` feature, so the lean reader links no HTTP client. Depends on no workspace crate; the OCR, fonts, and Piper crates and the app depend on it. See [Optional components](../components.md).

### Documents

- **`textweaver-text`**: the document model. A `Document` is canonical text in a rope plus `Marker`s. Units (grapheme, word, sentence, line, paragraph), navigation, go to, history, search, narration (`narrate::plan`), and the heading-anchor rule (`slug`) are pure functions of a document and a position. ADRs: [0002](../adr/0002-text-model.md), [0005](../adr/0005-narration-and-offset-map.md). Depends on core.
- **`textweaver-formats`**: loaders. Text, Markdown (with Obsidian's callouts, embeds, tags, and block ids), HTML, EPUB, DOCX, PDF, DAISY, PowerPoint, spreadsheets, archives, JSON and notebooks, SVG, MathML, LaTeX, email, and web pages, a registry that picks the loader by extension, a document cache, and exports to Markdown, HTML, and text. The loaders refuse binary files and limit nesting depth and counters, so a hostile file cannot crash a batch. Its default `ocr` feature adds OCR for scanned PDF pages and image files, through `textweaver-ocr` and its in-process ocrs engine; the smaller `images` feature, which `ocr` turns on, gives the picture loader and the scanned-page path without that engine (Tesseract only), for the fuzz targets. An optional `pandoc` feature adds the one Pandoc loader, sandboxed and with a timeout; only `tw convert` registers it. The optional `bibliography` feature reads LaTeX bibliographies through cite; convert turns it on. It also holds the callout rules the renderer shares. ADRs: [0002](../adr/0002-text-model.md), [0010](../adr/0010-pdf-loader.md), [0026](../adr/0026-ocr-and-student-formats.md), [0044](../adr/0044-obsidian-json-svg-and-content-mathml.md). Depends on core and text, with its default `ocr` feature on ocr, and with `bibliography` on cite.
- **`textweaver-ocr`**: OCR of scanned pages: the pure-Rust `ocrs` engine in process by default, a Tesseract subprocess as the fallback, model downloads checked by SHA-256 (off by default; `tw ocr download` turns it on), and PDF page images decoded through `hayro`. ADR: [0026](../adr/0026-ocr-and-student-formats.md). Depends on nothing in the workspace.
- **`textweaver-math`**: LaTeX and ASCIIMath parsed into one tree, written as MathML, spoken as English with an offset map, navigable part by part, and found in plain text without mistaking prices for math. ADR: [0018](../adr/0018-math.md). Depends on core.
- **`textweaver-cite`**: the reference library, DOI and ISBN lookup, BibTeX, RIS, and CSL-JSON, CSL formatting, and Pandoc citation keys. ADR: [0019](../adr/0019-citations.md). Depends on core.

### Speech

- **`textweaver-speech`**: the `SpeechBackend` trait, the `SpeechService` and its thread, the queue, pacing, the normalization pipeline, and the in-process backends (`null`, `recording`, `espeak`, `omnivox`, `speechd`), with the backend registry. Its optional `mathcat` feature routes math to `textweaver-mathcat` instead of `textweaver-math` when the `[reading] math_engine` setting asks for it. ADRs: [0003](../adr/0003-speech-threading-and-event-timing.md), [0004](../adr/0004-rate-pitch-volume.md), [0005](../adr/0005-narration-and-offset-map.md), [0029](../adr/0029-mathcat-speech.md). Depends on core and math, and, with the `mathcat` feature, on mathcat.
- **`textweaver-mathcat`**: math speech through MathCAT (ClearSpeak and SimpleSpeak), on its own dedicated thread so a MathCAT call can never block speech, with `textweaver-math` as the fallback when MathCAT cannot speak an expression. Pinned to `0.7.6-rc.3`, with its rules embedded at build time; braille output waits on a MathCAT issue. ADR: [0029](../adr/0029-mathcat-speech.md). Depends on core and math.
- **`textweaver-enginehost`**: everything the out-of-process engines share: the framed protocol, starting and watching a host process, the playback client with its audio clock, WAV writing, and the host side's request reader. ADR: [0012](../adr/0012-engine-host.md). Depends on core and speech.
- **`textweaver-eci`**: ETI-Eloquence through its ECI library, in a host process, with the community dictionaries. ADRs: [0007](../adr/0007-eloquence-via-eci-host.md), [0012](../adr/0012-engine-host.md). Depends on core, speech, and enginehost.
- **`textweaver-sapi`**: Windows SAPI5 and OneCore voices, in 64-bit and 32-bit host processes. ADRs: [0009](../adr/0009-sapi5-voices.md), [0012](../adr/0012-engine-host.md). Depends on core, speech, and enginehost.
- **`textweaver-dectalk`**: a user-installed DECtalk, in a host process. ADRs: [0021](../adr/0021-dectalk.md), [0012](../adr/0012-engine-host.md). Depends on core, speech, and enginehost.
- **`textweaver-apple`**: Apple's voices on macOS, as the `nsspeech` and `avspeech` backends. Empty on other systems. ADR: [0008](../adr/0008-apple-speech.md). Depends on core and speech.
- **`textweaver-piper`**: Piper neural voices, run in process on RTen (a pure-Rust ONNX runtime), with word timing read from the model instead of estimated. Phonemes come from an installed `libespeak-ng` when present, else a built-in pure-Rust eSpeak NG port; voice downloads from Hugging Face are checked by hash and stay behind the `download` feature. ADR: [0023](../adr/0023-in-process-neural-speech.md). Depends on core, speech, and enginehost.
- **`textweaver-engines`**: the one backend registry every frontend shares: the speech crate's built-ins plus Eloquence, SAPI5, Apple, DECtalk, and Piper, each configured from the settings, and the speech service configuration. It has the in-process engine features (`espeak`, `omnivox`, `speechd`, `mathcat`) and re-exports the engine crates. ADRs: [0001](../adr/0001-workspace-and-dependencies.md), [0012](../adr/0012-engine-host.md), [0023](../adr/0023-in-process-neural-speech.md). Depends on core, speech, store, eci, sapi, dectalk, apple, and piper.
- **`textweaver-export`**: reads a document into WAV, MP3, or M4B with chapters, and writes SRT or WebVTT subtitles. ADR: [0011](../adr/0011-audio-export.md). Depends on core, speech, and text.

### State and input

- **`textweaver-store`**: settings and key overrides, per-document state (position, history, bookmarks, notes, highlights), recent files, the library and its full-text index, folder sidecars and their merge rules, settings import and export, and the Star migration. It owns the one notes model (`Note`, `Highlight`, `Relation`, `RelationType`) and the saved form of the `[reading_aids]` settings (`reading_aids`). ADR: [0001](../adr/0001-workspace-and-dependencies.md). Depends on core.
- **`textweaver-sync`**: sync through a folder the user chooses (ADR-0049, "Sync beyond the place"): the hybrid logical clock, the merge types (newest-wins registers by id with deletion records, add-wins sets, per-computer counters), one record per document (each computer's place, bookmarks, notes, highlights, and statistics) whose merge lists what was replaced, and the folder format, in which each computer writes only its own files. It also keeps the install marker that gives a copied state folder a fresh id, and document identity: a random sync id per document, found by the file's SHA-256, then the SHA-256 of its text, then its library folder's id and path inside it (a DOI or ISBN is only suggested), kept in `sync-ids.json` in the data folder and worked out on the app's background writer. No networking. Depends on core and store.
- **`textweaver-keymap`**: every action, key chords, layers, the default keys for the terminal and the GUI, overrides, conflict checks, and the generated help. ADR: [0006](../adr/0006-keymap-and-actions.md). Depends on core.
- **`textweaver-a11y`**: the `Announcer` trait, the announcement catalog, verbosity, the accessibility mode with `route` (which decides whether each message, echo, caret move, and piece of read text goes to the voice, the status line, or both), and screen reader detection (`detect`). ADR: [0006](../adr/0006-keymap-and-actions.md). Depends on core.
- **`textweaver-editor`**: undo and redo over a rope, Markdown commands, find and replace, typing echo, autosave and recovery, and saving. Depends on core.

### Application

- **`textweaver-app`**: the application core. `App` owns all mutable state. Frontends send `Command`s to `App::dispatch` and act on the `Effect`s it returns; `App::poll_speech` applies speech status. It also holds reading aids, themes, notes, the library list, the writer thread (`writer`, `writes`), finding marks again after outside edits (`relocate`), the structure of Markdown source while editing (`structure`), the authoring features (outline, spelling, citations, export, preview, templates), and the JSON-RPC server (`rpc`). It also holds what a GUI needs: the document window (`window`), the list and prompt state every frontend shares (`list_model`), the waker (`wake`), the settings schema and settings screen (`settings_schema`), opening in the background (`opening`), and `Command::ReplaceRange` for native text controls. ADRs: [0003](../adr/0003-speech-threading-and-event-timing.md), [0006](../adr/0006-keymap-and-actions.md), [0015](../adr/0015-json-rpc.md), [0024](../adr/0024-app-core-for-the-gui.md), [0025](../adr/0025-lexicon-and-message-catalog.md). Depends on core, text, formats, speech, engines, store, keymap, a11y, editor, aids, theme, math, lexicon (define word, and the message catalog), and summary, and, with its `publish` feature (on by default and in releases) for export, preview, and citations in the reader, on render, convert, and cite. Without `publish` those commands say they are not in this build. With its `dictation` feature (on by default) it depends on dictation, for the Dictate command in edit mode ([ADR-0042](../adr/0042-streaming-dictation.md)); without it the command stays hidden.

### Frontends

- **`textweaver-tui`**: the terminal reader, on ratatui and crossterm; builds the `textweaver` program. ADRs: [0006](../adr/0006-keymap-and-actions.md), [0020](../adr/0020-themes.md), [0022](../adr/0022-reading-aids.md). Depends on app, engines, theme, and aids. Its default features (`publish`, `grammar`, `audio-export`, `opus`, and others) turn on the app's; `cargo build -p textweaver-tui --no-default-features` builds a lean reader without export, preview, citations, grammar checking, and Export audio.
- **`textweaver-cli`**: the `tw` program. One module per subcommand. `tw open` and `tw serve` run the terminal reader and the JSON-RPC server in process. ADRs: [0015](../adr/0015-json-rpc.md), [0016](../adr/0016-rendering-and-conversion.md), [0011](../adr/0011-audio-export.md). Depends on app (with `publish`), engines, tui, convert, render, writers, export, cite, vault, and dictation.
- **`textweaver-xilem`**: the all-Rust GUI on Masonry (Xilem's widget layer), Vello, Parley, AccessKit, and winit, with its own `DocumentView`; builds `textweaver-xilem`, to become `textweaver-gui`. Masonry is vendored under `third_party/xilem`, patched to AccessKit 0.25. ADR: [0027](../adr/0027-xilem-gui.md). Depends on app, theme, and fonts. Not a default member of the workspace.

### Output and study tools

- **`textweaver-render`**: Markdown to accessible HTML, with two engines, four flavors, math as MathML, and MiniJinja templates. ADRs: [0016](../adr/0016-rendering-and-conversion.md), [0018](../adr/0018-math.md). Depends on text (for the heading slug rule), math, and formats without its default features (for the callout rules the reader shares, [ADR-0044](../adr/0044-obsidian-json-svg-and-content-mathml.md)).
- **`textweaver-convert`**: converts files and folder trees on every core, skipping what is up to date, and watches folders. It formats citations with a References section. ADR: [0016](../adr/0016-rendering-and-conversion.md). Depends on core, text, formats, render, writers, and cite.
- **`textweaver-writers`**: EPUB 3, DOCX, BRF braille, and tagged PDF, written from a `Document`, with math typeset in each. ADR: [0017](../adr/0017-writers.md). Depends on core, text, fonts, and math.
- **`textweaver-fonts`**: the bundled fonts (Atkinson Hyperlegible Next and Mono, OpenDyslexic), a scan of installed fonts (once per process, `system::installed`), and the one place a font choice is resolved (`choice`: the family a reader picked, the reading fonts, platform fallbacks, and which family to use). The reading aids, the writers, and the GUIs all use it. ADRs: [0017](../adr/0017-writers.md), [0022](../adr/0022-reading-aids.md). Depends on nothing in the workspace.
- **`textweaver-theme`**: the 23 themes, user themes, contrast checks, and output for the terminal, the GUI, and CSS. ADR: [0020](../adr/0020-themes.md). Depends on nothing in the workspace.
- **`textweaver-aids`**: RSVP, bionic reading, text spacing, fonts, the reading ruler, difficult words, reading level, and syllables, as pure data in and out. It converts between the saved `[reading_aids]` settings in store and its own working types (`settings`). It also shortens a difficult word's first definition from any dictionary the caller passes (`definitions`), and checks RSVP against WCAG 2.3.1's flash limit (`rsvp::flash`). ADRs: [0022](../adr/0022-reading-aids.md), [0037](../adr/0037-extractive-summaries.md). Depends on core, text, store, and fonts.
- **`textweaver-summary`**: extractive summaries without a model: LexRank on TF-IDF cosine similarity, computed without building the similarity matrix, over the text crate's sentences, sampled evenly in long texts. `tw summarize` and the Summarize command use it. ADR: [0037](../adr/0037-extractive-summaries.md). Depends on core and text.
- **`textweaver-vault`**: Obsidian vault export and import of notes and highlights, on the store's own `Note` and `Highlight` (one notes model). Ported from Star; no ADR. Depends on core and store.
- **`textweaver-dictation`**: the `Dictation` trait, the Whisper subprocess backend, spoken commands, Whisper in-process on RTen with the microphone, and live dictation: speech found as the audio arrives and words committed while the speaker talks (`vad`, `stream`). ADRs: [0013](../adr/0013-dictation.md), [0023](../adr/0023-in-process-neural-speech.md), [0042](../adr/0042-streaming-dictation.md). Depends on core.
- **`textweaver-lexicon`**: language and study aids: offline "define word" (a glossary, the Open English WordNet, and CMUdict pronunciations, in an `fst` index with zstd-compressed entries) and the Fluent-subset interface message catalog, with its pseudo-locales for translation testing. ADRs: [0025](../adr/0025-lexicon-and-message-catalog.md), [0030](../adr/0030-interface-translations.md). Depends on nothing in the workspace.

### Tools

- **`xtask`**: `cargo xtask bench` and `startup` (with a baseline gate), `soak`, `dist`, `appimage`, `release`, `hosts`, `eci-host`, `sapi-host`, `keyboard` (writes [keyboard.md](../keyboard.md)), `deps` (checks the dependency direction), `notices` (writes `THIRD-PARTY-NOTICES.md`), and `parity` (writes the Star parity report to `target/parity-report.md`). Depends on formats, keymap, and text, and on app for the benchmarks.

## Dependency direction

Dependencies point down, from the frontends to the foundation, and never back up. Cargo refuses cycles, so this is enforced by the build. The rules that keep the design clean ([ADR-0001](../adr/0001-workspace-and-dependencies.md)):

- `textweaver-core` depends on no other workspace crate. Everything else may depend on it.
- `textweaver-speech` never sees a `Document`. It receives `Utterance`s: text plus an offset map. It depends only on core and math, so it can be tested with no document, no frontend, and no audio.
- Engine crates (eci, sapi, dectalk, apple) depend on speech, not the other way round. `textweaver-engines` registers them with the speech registry; the app and the frontends use that crate, not each engine crate.
- `textweaver-sync` depends only on core and store, and outside them only on serde, serde_json, sha2 (document identity), thiserror, and log: it reads and writes files in a folder, and never opens a network connection.
- The editor works on a rope and core's `Edit`. The app applies the same edit to the `Document` with `Document::apply`, so markers, bookmarks, and notes move with it.
- `textweaver-app` is the only crate that knows about everything. Frontends depend on the app, never on each other, except that the CLI runs the TUI in process for `tw open`.
- Output crates (render, convert, writers) do not depend on speech or the app, so conversion works with no speech at all.

Two rules changed after ADR-0001 was written. Speech depends on math, because math is spoken inside the normalization pipeline. And the app can depend on render, convert, and cite, because the reader exports, previews, and inserts citations; that is the app's `publish` feature, on by default and in releases. Documents still stay out of speech. Store depends only on core again: the reading-aid settings types are store's own, and aids converts them.

Three more engine-shaped crates were added on the same pattern. `textweaver-piper` ([ADR-0023](../adr/0023-in-process-neural-speech.md)) depends on speech and enginehost exactly like eci, sapi, and dectalk, and `textweaver-engines` registers it the same way, behind the speech crate's own feature flags rather than a host process. `textweaver-mathcat` ([ADR-0029](../adr/0029-mathcat-speech.md)) depends on math, and speech depends on it only behind its `mathcat` feature, so the lean reader still links neither MathCAT nor a second math engine. `textweaver-ocr` and `textweaver-lexicon` depend on nothing in the workspace; formats reaches ocr behind its own `ocr` feature (on by default), and the app depends on lexicon directly for define word and the message catalog.

`cargo xtask deps --check` checks these rules and runs in CI. It refuses a forbidden edge: core depending on anything, speech reaching text or formats, store reaching more than core, or the reader reaching the conversion and citation stack without its default features. It resolves cargo features as Cargo does, and reports what the reader reaches only through `publish` as allowed.

The crates in levels, from the bottom up. Each crate depends only on crates in lower levels:

1. core, fonts, theme, lexicon, and ocr, which depend on no workspace crate;
2. text, math, cite, store, keymap, a11y, editor, and dictation;
3. formats, aids, speech, render, writers, vault, and mathcat;
4. enginehost, apple, export, and convert;
5. eci, sapi, dectalk, and piper;
6. engines;
7. app;
8. tui, xilem, and xtask;
9. cli.

## Threads and processes

textweaver keeps the user interface responsive by never waiting for speech on the interface thread ([ADR-0003](../adr/0003-speech-threading-and-event-timing.md)).

### The input and interface thread

In the terminal reader, the main thread runs the event loop in `crates/textweaver-tui/src/lib.rs`. Each turn of the loop:

1. applies any speech status that arrived (`App::poll_speech`) and runs housekeeping (`App::tick`): it applies what the writer thread finished, queues the autosave snapshot and the periodic position save, and opens lists that were being prepared in the background (the library, the voices);
2. on macOS, pumps the main run loop for Apple's `avspeech` backend;
3. draws the screen, and parks the terminal's hardware cursor where attention is (the prompt, the list item, the Speech Cursor line, the spoken word, or the caret), so screen readers and magnifiers follow it;
4. waits briefly for a key, then turns the key into a `Command` through the keymap and dispatches it.

Nothing on this thread blocks on audio or on the disk. A key press is handled in milliseconds even while a long document is being read or saved. Other slow work goes to short-lived background threads too: exporting from the reader, looking up a DOI or ISBN, parsing the Markdown source of a file of 256 KB or more when edit mode opens it, opening a file of 512 KB or more (with progress, and Escape to stop waiting), counting misspellings after a save, and starting the speech engine the first time.

A frontend does not have to poll for this work. `App::set_waker` takes a callback that the speech thread, the writer thread, and every background job call when they have something to apply; the frontend then calls `App::poll_speech` and `App::tick` on its own thread. A GUI posts an event to its event loop from the callback. `App::tick_interval` says how long it may sleep when nothing calls it ([ADR-0024](../adr/0024-app-core-for-the-gui.md)). The terminal reader still waits on the keyboard with a short timeout.

`tw serve --stdio` runs the same core on its calling thread and reads JSON-RPC messages on a second thread ([ADR-0015](../adr/0015-json-rpc.md)). The app's waker wakes it as each word is heard; otherwise it looks for work every 250 milliseconds at most.

### The speech thread

`SpeechService::spawn` starts one speech thread. It builds the engine backend on that thread, from a factory, because many engines must stay on the thread that created them (SAPI and WinRT COM apartments, espeak-ng's global state). The app talks to it through a command channel and reads `SpeechStatus` from a status channel.

The speech thread owns the queue, the normalization pipeline, the generation counter, and the playback clock. While speech is active it calls the backend's `poll` every few milliseconds, which is how `stop` and `pause` reach an engine in the middle of a sentence. Nothing on it waits for long: an engine host that is starting, the audio device that is opening, and speech-dispatcher's replies are all picked up in `poll` (Phase 2).

Each backend lists its voices once, when it starts, into a `VoiceCache` that any thread reads without waiting; SAPI fills its cache from a background thread. If the speech thread dies, the app goes silent at once and restarts speech in place, once by itself and then on the Restart Speech command, starting the new service on a helper thread (`crates/textweaver-app/src/restart.rs`).

### Engine hosts

Eloquence, SAPI5 voices, and DECtalk each run in a separate host process ([ADR-0012](../adr/0012-engine-host.md)). A host synthesizes into memory and streams PCM audio and word positions back over a pipe. In textweaver's process:

- a reader thread decodes the host's replies;
- the playback client feeds the audio to the output device (through rodio, on its own audio thread) and keeps the audio clock;
- word events are emitted as each word is heard.

A host that crashes or stalls is killed and started again, and reading resumes from the last word heard, at most three times for one sentence. Hosts never outlive textweaver: a host exits when its input closes, on Windows every host is in a Job Object that ends with textweaver, and on Linux each asks for a signal when its parent dies. A new host starts without holding up the speech thread: requests queue until it reports ready, which `poll` notices. A host also lets a 32-bit engine work with 64-bit textweaver, and keeps a proprietary engine at arm's length from the reader.

### The writer thread

Every file the app writes while it runs goes through one writer thread (`crates/textweaver-app/src/writer.rs`): saves, autosave snapshots and their deletion, reading positions, bookmarks, notes, and highlights, the library sidecars, the bookshelf and recent list, `settings.toml`, and the two-second check for a change on disk. The interface thread hands it a copy of the text (a rope clone, which costs nothing) and reads the result on its next tick: "Saved", an error, or a question about a file that changed on disk. Saves of the same document's state that queue up are collapsed into the newest. Quitting waits for the writer, at most ten seconds, saying so when it takes more than a moment.

### Other workers

- `tw convert` runs conversions on a rayon thread pool, one document per worker ([ADR-0016](../adr/0016-rendering-and-conversion.md)).
- Dictation runs a Whisper program as a subprocess and reads its output on a worker thread ([ADR-0013](../adr/0013-dictation.md)).
- Audio export runs synthesis on the calling thread, utterance by utterance ([ADR-0011](../adr/0011-audio-export.md)).
- The Library command scans the library folders on a background thread and shows the count found as it goes.

## From file to highlighted word

This is the path a document takes from the disk to your ears, with the highlight following along. The [speech pipeline page](../site/speech-pipeline.html) walks through it one step at a time.

1. **Load.** `textweaver-formats` picks a loader by the file's extension and builds a `Document`: canonical text in a rope, plus markers for headings, paragraphs, lists, tables, links, code, math, strikethrough, horizontal rules, pages, and sections ([ADR-0002](../adr/0002-text-model.md)). Positions are `CharPos`, counts of Unicode scalar values. They are what gets saved.
2. **Units.** `textweaver-text` finds words and sentences with Unicode text segmentation (UAX #29), with an abbreviation list so "Dr." does not end a sentence. Navigation moves by these units and by markers.
3. **Plan.** When you press Space, the app asks `narrate::plan` for utterances covering about ten minutes of speech from the cursor (32,768 characters, ended at a sentence boundary). Each utterance is about one sentence. Structure the eye sees but the ear would miss, such as "heading level 2" or a table's row and column, is added as inserted speech ([ADR-0005](../adr/0005-narration-and-offset-map.md)). When that window has been read, the next one is planned.
4. **Hand over.** The app calls `SpeechService::read`, which returns a new **reading generation** (see below).
5. **Normalize.** Just before an utterance enters the two-sentence lookahead, the speech thread runs it through the normalization pipeline, on its own and never across sentences: math first, then Markdown residue, your pronunciations, the community lexicon, abbreviations, numbers, split capitals, and punctuation. Each transform returns its text and an offset map, and the maps are composed, so the final map still points into the document. Transforms an engine does itself, such as Eloquence's numbers and abbreviations, are skipped for that engine.
6. **Speak.** The backend's `speak` starts the utterance and returns at once. The engine, in process or in a host, starts producing audio.
7. **Word events.** The engine reports each word as a byte range in the spoken text, and, for engines that know it, the word's time on the audio clock.
   - Audio-clock engines (espeak-ng) have each word scheduled at its audio time plus the latency offset (`[speech] latency_offset_ms`, 120 milliseconds by default).
   - Engines whose audio textweaver plays itself (Eloquence, SAPI, DECtalk, Apple's `avspeech`) report each word as it is heard, and it is used at once.
   - Engines with no word events (Omnivox) are paced by a timer from the effective words per minute.
8. **Generation check.** Events from an older generation are dropped before anything looks at them. See below.
9. **Map back.** The byte range goes through the utterance's offset map to a range of document characters. An expanded token, such as "$5" read as "five dollars", highlights the whole token; inserted speech highlights nothing.
10. **Highlight.** The service sends `SpeechStatus::Position`. The app checks its reading generation, moves the highlight (shifted by `[highlight] lead_words`), and moves the cursor if `[reading] cursor_follows_speech` is on. The terminal reader draws it on the next turn of its loop and parks the hardware cursor on the word.

Pausing records the last word that was confirmed as heard. Resuming starts from there, so a word may be repeated, but none is ever skipped.

## Offset maps

An `OffsetMap` ([ADR-0005](../adr/0005-narration-and-offset-map.md), `crates/textweaver-core`) connects the spoken text to the document. It is a sorted list of spans. Each span pairs a byte range of spoken text with a character range of source text, and has one of four kinds:

- **Literal**: the same text, character for character. A word inside maps to exactly its source characters.
- **Expanded**: spoken words that stand for a whole source token, such as "Doctor" for "Dr." or "squared" for `^2`. Any word inside highlights the whole token.
- **Inserted**: spoken words with no source, such as "heading level 2". They highlight nothing. If you pause inside one, reading resumes at its anchor.
- **Elided**: source that is not spoken, such as Markdown's `**` or the dollar signs around math. It has no spoken text.

A small example, the Markdown `The area is $x^2$ today.`, is spoken as "The area is x squared today.". This is the real map, from `tw speak --backend null --json`:

- "The area is " is literal: spoken bytes 0 to 12, source characters 0 to 12.
- The first `$` is elided: source character 12.
- "x" is literal: spoken byte 12, source character 13.
- The space after it is inserted: spoken byte 13, anchored at source character 14.
- "squared" is expanded from `^2`: spoken bytes 14 to 21, source characters 14 to 16.
- The second `$` is elided: source character 16.
- " today." is literal: spoken bytes 21 to 28, source characters 17 to 24.

When the engine reports the word "squared" (spoken bytes 14 to 21), the map finds the expanded span and highlights `^2`, characters 14 to 16. The word "x" highlights character 13, so the two words together cover the whole `x^2`.

The map answers three questions: which source range a spoken word came from (for the highlight), where in the spoken text to start for a cursor position (for reading from the cursor), and where to resume after a pause. Every map is checked by `OffsetMap::check_invariants`, and property tests generate random maps and compositions.

The same idea is used elsewhere: audio export maps word times back through offset maps so subtitles show the document's own text, and the syllable display in `textweaver-aids` maps its dotted words back to the document.

## Reading generations

Speech is asynchronous, so an event can arrive after the reading it belongs to has been replaced. Star's worst highlight bugs came from exactly that: a late "done" from the previous sentence stopped the current highlight. textweaver guards against it at two levels.

- **Utterance generations, inside the speech service.** Every utterance carries an id: a generation and a chunk number. The service bumps the generation before every stop or restart. Its event sink drops any event whose generation is not current before the service logic sees it, so a late `Word` or `Finished` can never move the highlight of a newer reading ([ADR-0003](../adr/0003-speech-threading-and-event-timing.md)).
- **Reading generations, between the service and the app.** Every `read` returns a `ReadingGeneration`, a number, and every `Position`, `Paused`, `Stopped`, and `Finished` status carries it. The app remembers the generation of the reading it started and ignores statuses from any other. Checking one number replaced guessing from positions, which failed, for example, when a reading started at a skipped code block.

On engines that report words, changing the rate, pitch, or volume while reading restarts the reading from the last confirmed word, with a new generation, so the change is heard at once.

## Announcements

Every state change is announced through the `Announcer` trait in `textweaver-a11y` ([ADR-0006](../adr/0006-keymap-and-actions.md)). The app filters by `[speech] verbosity` and sends each announcement to:

- the status line, which the terminal reader draws and screen readers read;
- the speech service, when the accessibility mode lets textweaver speak, but never over the reading itself unless it is an error;
- JSON-RPC clients, as `announcement` notifications.

Before anything is spoken or written to the status line, the app asks `textweaver_a11y::route` where it goes. The answer depends on the kind of output (a message, typing echo, a caret move, read text, or a Speech Cursor line) and on `[accessibility] mode`: self-voicing, hybrid, or screen reader. So a screen reader that reads the status line never hears the same thing twice. New code must route its announcements the same way. [Using textweaver with a screen reader](../screen-readers.md) describes the modes for users.

The GUI sends announcements to the screen reader as UI Automation notifications on Windows, and as a live region on macOS and Linux.

## The GUI's view of a document

A GUI does not lay out a whole document. It shows a `DocWindow` (`crates/textweaver-app/src/window.rs`): about 500,000 UTF-16 units around the focus, starting and ending on paragraph boundaries. The window slides forward while reading, saying what left its front and what joined its end, and recenters after a jump. Positions in the window are converted to and from the text control's own units (UTF-16 for Windows and macOS controls, UTF-8 bytes for Parley and AccessKit, chars for GTK) through the document's `DisplayIndex`, which is the rope itself and answers in `O(log n)`. `Session::revision` changes whenever the text changes, so the GUI knows to reload. Edits made in a native text control come back as `Command::ReplaceRange`. The terminal reader keeps its own slicing: a screenful of wrapped rows from the viewport's top line ([ADR-0024](../adr/0024-app-core-for-the-gui.md)).

## Persistence

`textweaver-store` keeps settings in `settings.toml` and key overrides in `keymap.toml`, written only when they change, atomically, keeping keys it does not know. The app's settings schema (`crates/textweaver-app/src/settings_schema.rs`) is generated from the store's own keys, with a label, help, and range or choices for each; it drives the settings screen, the GUI's settings dialog, and JSON-RPC. Each document's position, history, bookmarks, notes, and highlights are in a state file named after the document; positions are saved every 30 seconds while they move, on quit, and on switching documents, by the writer thread. The state file also keeps the text's length and hash and, with each position and bookmark, the 40 characters it was on: when the file changed outside textweaver, positions, bookmarks, notes, and highlights are found again from their text on the next open (`crates/textweaver-app/src/relocate.rs`). Library folders can hold a sidecar, `.textweaver/progress.json`, so positions follow a folder between computers. [Settings](../settings.md) and [the library](../library.md) describe the files and folders.

## See also

- [Interactive architecture page](../site/architecture.html): choose a crate to see its job, ADRs, dependencies, and dependents.
- [Speech pipeline, step by step](../site/speech-pipeline.html): the data flow above, one stage at a time, with the offset map example.
- [CONTRIBUTING.md](../../CONTRIBUTING.md): building, testing, and sending changes.
- [Roadmap](../roadmap.md): the planned changes to this architecture, including the GUI and the stability work.
- [ADR-0001: Workspace and dependencies](../adr/0001-workspace-and-dependencies.md), [ADR-0003: Speech threading](../adr/0003-speech-threading-and-event-timing.md), and [ADR-0005: Narration and the OffsetMap](../adr/0005-narration-and-offset-map.md): the three decisions this guide leans on most.
- [Documentation index](../README.md)
