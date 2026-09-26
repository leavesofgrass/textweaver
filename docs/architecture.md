# Architecture

This guide explains how textweaver is put together: which crate does what, which way the crates depend on each other, which threads and processes run, and how a file on disk becomes a spoken word with its highlight. It is for contributors, and for anyone who wants to know why the highlight never drifts. Each part links to the decision record (ADR) that explains the choice, and to the crate that holds the code.

The [interactive architecture page](site/architecture.html) shows the same crate map: choose a crate to see its job, its ADRs, and what it depends on. The [speech pipeline page](site/speech-pipeline.html) steps through the data flow below one stage at a time.

## The big picture

textweaver is one Cargo workspace with 28 crates and a maintenance crate, `xtask`. Two programs come out of it:

- `textweaver`, the terminal reader, built from `crates/textweaver-tui`;
- `tw`, the command-line tool, built from `crates/textweaver-cli`.

A third program, `textweaver-gui`, is a feasibility spike for the native GUI ([ADR-0014](adr/0014-gui-toolkit.md)). It is not built by default.

Three helper programs run speech engines in their own processes: `textweaver-eci-host` (Eloquence), `textweaver-sapi-host` (SAPI5 voices), and `textweaver-dectalk-host` (DECtalk), each in a 64-bit build and, on Windows, a 32-bit `-x86` build. `cargo xtask hosts` builds them.

Everything a user does goes through one application core, `textweaver-app`. The terminal reader, the GUI spike, and the JSON-RPC server (`tw serve`) are thin frontends over it. They turn keys or messages into commands, and draw or send what the core returns.

## The crates

The crates are grouped here by the part of the system they serve. For each crate: what it does, the ADRs that decided it, and the workspace crates it depends on (normal dependencies only; test-only dependencies are left out).

### Foundation

- **`textweaver-core`**: shared leaf types. Positions (`CharPos`, `CharRange`), units, marker kinds, the `OffsetMap`, edits and how positions move across them, rate, pitch, and volume, the `Utterance` handed to speech, and small preference enums. ADRs: [0002](adr/0002-text-model.md), [0005](adr/0005-narration-and-offset-map.md). Depends on nothing in the workspace.

### Documents

- **`textweaver-text`**: the document model. A `Document` is canonical text in a rope plus `Marker`s. Units (grapheme, word, sentence, line, paragraph), navigation, go to, history, search, and narration (`narrate::plan`) are pure functions of a document and a position. ADRs: [0002](adr/0002-text-model.md), [0005](adr/0005-narration-and-offset-map.md). Depends on core.
- **`textweaver-formats`**: loaders. Text, Markdown, HTML, EPUB, DOCX, and PDF, a registry that picks the loader by extension, a document cache, and exports to Markdown, HTML, and text. An optional `pandoc` feature adds a Pandoc loader. ADRs: [0002](adr/0002-text-model.md), [0010](adr/0010-pdf-loader.md). Depends on core and text.
- **`textweaver-math`**: LaTeX and ASCIIMath parsed into one tree, written as MathML, spoken as English with an offset map, navigable part by part, and found in plain text without mistaking prices for math. ADR: [0018](adr/0018-math.md). Depends on core.
- **`textweaver-cite`**: the reference library, DOI and ISBN lookup, BibTeX, RIS, and CSL-JSON, CSL formatting, and Pandoc citation keys. ADR: [0019](adr/0019-citations.md). Depends on core.

### Speech

- **`textweaver-speech`**: the `SpeechBackend` trait, the `SpeechService` and its thread, the queue, pacing, the normalization pipeline, and the in-process backends (`null`, `recording`, `espeak`, `omnivox`, `speechd`), with the backend registry. ADRs: [0003](adr/0003-speech-threading-and-event-timing.md), [0004](adr/0004-rate-pitch-volume.md), [0005](adr/0005-narration-and-offset-map.md). Depends on core and math.
- **`textweaver-enginehost`**: everything the out-of-process engines share: the framed protocol, starting and watching a host process, the playback client with its audio clock, WAV writing, and the host side's request reader. ADR: [0012](adr/0012-engine-host.md). Depends on core and speech.
- **`textweaver-eci`**: ETI-Eloquence through its ECI library, in a host process, with the community dictionaries. ADRs: [0007](adr/0007-eloquence-via-eci-host.md), [0012](adr/0012-engine-host.md). Depends on core, speech, and enginehost.
- **`textweaver-sapi`**: Windows SAPI5 and OneCore voices, in 64-bit and 32-bit host processes. ADRs: [0009](adr/0009-sapi5-voices.md), [0012](adr/0012-engine-host.md). Depends on core, speech, and enginehost.
- **`textweaver-dectalk`**: a user-installed DECtalk, in a host process. ADRs: [0021](adr/0021-dectalk.md), [0012](adr/0012-engine-host.md). Depends on core, speech, and enginehost.
- **`textweaver-apple`**: Apple's voices on macOS, as the `nsspeech` and `avspeech` backends. Empty on other systems. ADR: [0008](adr/0008-apple-speech.md). Depends on core and speech.
- **`textweaver-export`**: reads a document into WAV, MP3, or M4B with chapters, and writes SRT or WebVTT subtitles. ADR: [0011](adr/0011-audio-export.md). Depends on core, speech, and text.

### State and input

- **`textweaver-store`**: settings and key overrides, per-document state (position, history, bookmarks, notes, highlights), recent files, the library and its full-text index, folder sidecars and their merge rules, settings import and export, and the Star migration. ADR: [0001](adr/0001-workspace-and-dependencies.md). Depends on core and aids (for the `[reading_aids]` settings types).
- **`textweaver-keymap`**: every action, key chords, layers, the default keys for the terminal and the GUI, overrides, conflict checks, and the generated help. ADR: [0006](adr/0006-keymap-and-actions.md). Depends on core.
- **`textweaver-a11y`**: the `Announcer` trait, the announcement catalogue, and verbosity. ADR: [0006](adr/0006-keymap-and-actions.md). Depends on core.
- **`textweaver-editor`**: undo and redo over a rope, Markdown commands, find and replace, typing echo, autosave and recovery, and saving. Designed in section 6.6 of [the plan](plan.md). Depends on core.

### Application

- **`textweaver-app`**: the application core. `App` owns all mutable state. Frontends send `Command`s to `App::dispatch` and act on the `Effect`s it returns; `App::poll_speech` applies speech status. It also holds the backend registry wiring, reading aids, themes, notes, the library list, and the JSON-RPC server (`rpc`). ADRs: [0003](adr/0003-speech-threading-and-event-timing.md), [0006](adr/0006-keymap-and-actions.md), [0015](adr/0015-json-rpc.md). Depends on core, text, formats, speech, store, keymap, a11y, editor, aids, theme, eci, sapi, apple, and dectalk.

### Frontends

- **`textweaver-tui`**: the terminal reader, on ratatui and crossterm; builds the `textweaver` program. ADRs: [0006](adr/0006-keymap-and-actions.md), [0020](adr/0020-themes.md), [0022](adr/0022-reading-aids.md). Depends on app, theme, and aids.
- **`textweaver-cli`**: the `tw` program. One module per subcommand. `tw open` and `tw serve` run the terminal reader and the JSON-RPC server in process. ADRs: [0015](adr/0015-json-rpc.md), [0016](adr/0016-rendering-and-conversion.md), [0011](adr/0011-audio-export.md). Depends on app, tui, convert, render, writers, export, cite, vault, and dictation.
- **`textweaver-gui`**: the GUI spike on wxDragon. ADR: [0014](adr/0014-gui-toolkit.md). Depends on app, aids, and fonts. Not a default member of the workspace.

### Output and study tools

- **`textweaver-render`**: Markdown to accessible HTML, with two engines, four flavors, math as MathML, and MiniJinja templates. ADRs: [0016](adr/0016-rendering-and-conversion.md), [0018](adr/0018-math.md). Depends on math.
- **`textweaver-convert`**: converts files and folder trees on every core, skipping what is up to date, and watches folders. ADR: [0016](adr/0016-rendering-and-conversion.md). Depends on core, text, formats, render, and writers.
- **`textweaver-writers`**: EPUB 3, DOCX, BRF braille, and tagged PDF, written from a `Document`. ADR: [0017](adr/0017-writers.md). Depends on core, text, and fonts.
- **`textweaver-fonts`**: the bundled fonts (Atkinson Hyperlegible Next and Mono, OpenDyslexic) and a scan of installed fonts. ADRs: [0017](adr/0017-writers.md), [0022](adr/0022-reading-aids.md). Depends on nothing in the workspace.
- **`textweaver-theme`**: the 23 themes, user themes, contrast checks, and output for the terminal, the GUI, and CSS. ADR: [0020](adr/0020-themes.md). Depends on nothing in the workspace.
- **`textweaver-aids`**: RSVP, bionic reading, text spacing, fonts, the reading ruler, difficult words, reading level, and syllables, as pure data in and out. ADR: [0022](adr/0022-reading-aids.md). Depends on core, text, and fonts.
- **`textweaver-vault`**: Obsidian vault export and import of notes and highlights. Ported from Star; no ADR. Depends on core and store.
- **`textweaver-dictation`**: the `Dictation` trait, the Whisper subprocess backend, and spoken commands. ADR: [0013](adr/0013-dictation.md). Depends on core.

### Tools

- **`xtask`**: `cargo xtask bench`, `dist`, `hosts`, `eci-host`, `sapi-host`, `keyboard` (writes [keyboard.md](keyboard.md)), and `parity` (writes [the parity report](parity-report.md)). Depends on app, formats, keymap, and text.

## Dependency direction

Dependencies point down, from the frontends to the foundation, and never back up. Cargo refuses cycles, so this is enforced by the build. The rules that keep the design clean ([ADR-0001](adr/0001-workspace-and-dependencies.md)):

- `textweaver-core` depends on no other workspace crate. Everything else may depend on it.
- `textweaver-speech` never sees a `Document`. It receives `Utterance`s: text plus an offset map. It depends only on core and math, so it can be tested with no document, no frontend, and no audio.
- Engine crates (eci, sapi, dectalk, apple) depend on speech, not the other way round. The app registers them with the speech registry.
- The editor works on a rope and core's `Edit`. The app applies the same edit to the `Document` with `Document::apply`, so markers, bookmarks, and notes move with it.
- `textweaver-app` is the only crate that knows about everything. Frontends depend on the app, never on each other, except that the CLI runs the TUI in process for `tw open`.
- Output crates (render, convert, writers) do not depend on speech or the app, so conversion works with no speech at all.

Two rules changed after ADR-0001 was written: speech depends on math, because math is spoken inside the normalization pipeline, and store depends on aids, for the reading-aid settings types. Both still keep documents out of speech.

The crates in levels, from the bottom up. Each crate depends only on crates in lower levels:

1. core, fonts, and theme, which depend on no workspace crate;
2. text, math, cite, keymap, a11y, editor, and dictation;
3. formats, aids, speech, render, and writers;
4. enginehost, apple, export, store, and convert;
5. eci, sapi, dectalk, and vault;
6. app;
7. tui, gui, and xtask;
8. cli.

## Threads and processes

textweaver keeps the user interface responsive by never waiting for speech on the interface thread ([ADR-0003](adr/0003-speech-threading-and-event-timing.md)).

### The input and interface thread

In the terminal reader, the main thread runs the event loop in `crates/textweaver-tui/src/lib.rs`. Each turn of the loop:

1. applies any speech status that arrived (`App::poll_speech`) and runs housekeeping, such as autosave and the periodic position save (`App::tick`);
2. on macOS, pumps the main run loop for Apple's `avspeech` backend;
3. draws the screen, and parks the terminal's hardware cursor where attention is (the prompt, the list item, the Speech Cursor line, the spoken word, or the caret), so screen readers and magnifiers follow it;
4. waits briefly for a key, then turns the key into a `Command` through the keymap and dispatches it.

Nothing on this thread blocks on audio. A key press is handled in milliseconds even while a long document is being read.

`tw serve --stdio` runs the same core on its calling thread and reads JSON-RPC messages on a second thread, polling speech every 20 milliseconds while it waits ([ADR-0015](adr/0015-json-rpc.md)).

### The speech thread

`SpeechService::spawn` starts one speech thread. It builds the engine backend on that thread, from a factory, because many engines must stay on the thread that created them (SAPI and WinRT COM apartments, espeak-ng's global state). The app talks to it through a command channel and reads `SpeechStatus` from a status channel.

The speech thread owns the queue, the normalization pipeline, the generation counter, and the playback clock. While speech is active it calls the backend's `poll` every few milliseconds, which is how `stop` and `pause` reach an engine in the middle of a sentence.

### Engine hosts

Eloquence, SAPI5 voices, and DECtalk each run in a separate host process ([ADR-0012](adr/0012-engine-host.md)). A host synthesizes into memory and streams PCM audio and word positions back over a pipe. In textweaver's process:

- a reader thread decodes the host's replies;
- the playback client feeds the audio to the output device (through rodio, on its own audio thread) and keeps the audio clock;
- word events are emitted as each word is heard.

A host that crashes or stalls is killed and started again, and reading resumes from the last word heard. A host also lets a 32-bit engine work with 64-bit textweaver, and keeps a proprietary engine at arm's length from the reader.

### Other workers

- `tw convert` runs conversions on a rayon thread pool, one document per worker ([ADR-0016](adr/0016-rendering-and-conversion.md)).
- Dictation runs a Whisper program as a subprocess and reads its output on a worker thread ([ADR-0013](adr/0013-dictation.md)).
- Audio export runs synthesis on the calling thread, utterance by utterance ([ADR-0011](adr/0011-audio-export.md)).

## From file to highlighted word

This is the path a document takes from the disk to your ears, with the highlight following along. The [speech pipeline page](site/speech-pipeline.html) walks through it one step at a time.

1. **Load.** `textweaver-formats` picks a loader by the file's extension and builds a `Document`: canonical text in a rope, plus markers for headings, paragraphs, lists, tables, links, code, pages, and sections ([ADR-0002](adr/0002-text-model.md)). Positions are `CharPos`, counts of Unicode scalar values. They are what gets saved.
2. **Units.** `textweaver-text` finds words and sentences with Unicode text segmentation (UAX #29), with an abbreviation list so "Dr." does not end a sentence. Navigation moves by these units and by markers.
3. **Plan.** When you press Space, the app asks `narrate::plan` for utterances covering about ten minutes of speech from the cursor (32,768 characters, ended at a sentence boundary). Each utterance is about one sentence. Structure the eye sees but the ear would miss, such as "heading level 2" or a table's row and column, is added as inserted speech ([ADR-0005](adr/0005-narration-and-offset-map.md)). When that window has been read, the next one is planned.
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

An `OffsetMap` ([ADR-0005](adr/0005-narration-and-offset-map.md), `crates/textweaver-core`) connects the spoken text to the document. It is a sorted list of spans. Each span pairs a byte range of spoken text with a character range of source text, and has one of four kinds:

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

- **Utterance generations, inside the speech service.** Every utterance carries an id: a generation and a chunk number. The service bumps the generation before every stop or restart. Its event sink drops any event whose generation is not current before the service logic sees it, so a late `Word` or `Finished` can never move the highlight of a newer reading ([ADR-0003](adr/0003-speech-threading-and-event-timing.md)).
- **Reading generations, between the service and the app.** Every `read` returns a `ReadingGeneration`, a number, and every `Position`, `Paused`, `Stopped`, and `Finished` status carries it. The app remembers the generation of the reading it started and ignores statuses from any other. Checking one number replaced guessing from positions, which failed, for example, when a reading started at a skipped code block.

On engines that report words, changing the rate, pitch, or volume while reading restarts the reading from the last confirmed word, with a new generation, so the change is heard at once.

## Announcements

Every state change is announced through the `Announcer` trait in `textweaver-a11y` ([ADR-0006](adr/0006-keymap-and-actions.md)). The app filters by `[speech] verbosity` and sends each announcement to:

- the status line, which the terminal reader draws and screen readers read;
- the speech service, when self-voicing is on, but never over the reading itself unless it is an error;
- JSON-RPC clients, as `announcement` notifications.

The GUI spike sends announcements to the screen reader as UI Automation notifications through the `live-region` crate.

## Persistence

`textweaver-store` keeps settings in `settings.toml` and key overrides in `keymap.toml`, written only when they change, atomically, keeping keys it does not know. Each document's position, history, bookmarks, notes, and highlights are in a state file named after the document; positions are saved every 30 seconds while they move, on quit, and on switching documents. Library folders can hold a sidecar, `.textweaver/progress.json`, so positions follow a folder between computers. [Settings](settings.md) and [the library](library.md) describe the files and folders.

## See also

- [Interactive architecture page](site/architecture.html): choose a crate to see its job, ADRs, dependencies, and dependents.
- [Speech pipeline, step by step](site/speech-pipeline.html): the data flow above, one stage at a time, with the offset map example.
- [CONTRIBUTING.md](../CONTRIBUTING.md): building, testing, and sending changes.
- [Implementation plan](plan.md): the original design, and the Phase 0 amendments.
- [ADR-0001: Workspace and dependencies](adr/0001-workspace-and-dependencies.md), [ADR-0003: Speech threading](adr/0003-speech-threading-and-event-timing.md), and [ADR-0005: Narration and the OffsetMap](adr/0005-narration-and-offset-map.md): the three decisions this guide leans on most.
- [Documentation index](README.md)
