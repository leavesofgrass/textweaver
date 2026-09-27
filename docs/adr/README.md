# Architecture decision records

Each ADR records one decision: its context, the choice, and its consequences. ADRs keep their decisions; when later work changes one, a dated "Status update" line is added under its date instead of rewriting it. A new ADR that replaces an old one says so, and the old one gets a status update pointing to it.

Statuses: **accepted** is in force; **proposed** is still being tried; **superseded** has been replaced by the ADR it names.

## The records

- [ADR-0001: Workspace layout and dependency policy](0001-workspace-and-dependencies.md): one crate per job, one table of approved dependencies, and no async runtime in speech.
  - Status: accepted, with 3 status updates.
- [ADR-0002: Text model](0002-text-model.md): canonical text in a rope plus markers, and character positions.
  - Status: accepted, with 2 status updates.
- [ADR-0003: Speech threading and event timing](0003-speech-threading-and-event-timing.md): the speech thread, generations, word timing, and pause.
  - Status: accepted, with 3 status updates.
- [ADR-0004: Rate, pitch, and volume](0004-rate-pitch-volume.md): engine-independent voice settings.
  - Status: accepted, with 1 status update.
- [ADR-0005: Narration and the OffsetMap](0005-narration-and-offset-map.md): spoken text built together with its map back to the document.
  - Status: accepted, with 1 status update.
- [ADR-0006: Keymap, actions, and announcements](0006-keymap-and-actions.md): actions, layers, overrides, and announcing every change.
  - Status: accepted, with 2 status updates.
- [ADR-0007: ETI-Eloquence through an ECI host process](0007-eloquence-via-eci-host.md): Eloquence with exact word timing, and its licensing.
  - Status: accepted, with 1 status update.
- [ADR-0008: Apple speech on macOS](0008-apple-speech.md): the `nsspeech` and `avspeech` backends.
  - Status: accepted, with 1 status update.
- [ADR-0009: SAPI5 voices on Windows](0009-sapi5-voices.md): 64-bit and 32-bit voices in host processes.
  - Status: accepted, with 1 status update.
- [ADR-0010: PDF loader](0010-pdf-loader.md): a pure Rust PDF reader with column-aware reading order.
  - Status: accepted, with 3 status updates.
- [ADR-0011: Audio export](0011-audio-export.md): sentence-by-sentence synthesis, exact subtitles, and chapters.
  - Status: accepted, with 1 status update.
- [ADR-0012: One engine-host protocol and playback client](0012-engine-host.md): the shared protocol for Eloquence, SAPI5, and DECtalk.
  - Status: accepted, with 3 status updates.
- [ADR-0013: Dictation through a Whisper program](0013-dictation.md): voice typing with a Whisper subprocess.
  - Status: accepted, with 3 status updates; in part superseded by ADR-0023 (Whisper in-process).
- [ADR-0014: GUI toolkit (wxDragon)](0014-gui-toolkit.md): the GUI spike's findings. Wave 3 moves the GUI to Xilem, and the spike stays as a fallback.
  - Status: proposed. Wave 3 replaces it with a Xilem GUI (Agent W3b, ADR-0023); the wxDragon spike stays as a fallback until then. It has 2 status updates.
- [ADR-0015: JSON-RPC server (`tw serve --stdio`)](0015-json-rpc.md): `tw serve --stdio`, its methods, and notifications.
  - Status: accepted, with 2 status updates.
- [ADR-0016: Rendering and bulk conversion](0016-rendering-and-conversion.md): Markdown to accessible HTML, and fast, incremental conversion.
  - Status: accepted, with 3 status updates.
- [ADR-0017: Native writers (EPUB 3, DOCX, BRF, tagged PDF)](0017-writers.md): EPUB 3, DOCX, BRF braille, and tagged PDF, and their accessibility checks.
  - Status: accepted, with 2 status updates.
- [ADR-0018: Math](0018-math.md): LaTeX and ASCIIMath parsing, MathML, and spoken math.
  - Status: accepted, with 2 status updates.
- [ADR-0019: Citations](0019-citations.md): the reference library, lookup, and CSL formatting.
  - Status: accepted, with 3 status updates.
- [ADR-0020: Themes](0020-themes.md): Star's palettes, contrast rules, and output for every frontend.
  - Status: accepted, with 2 status updates.
- [ADR-0021: DECtalk through a host process](0021-dectalk.md): DECtalk with word timing, and its licensing.
  - Status: accepted, with 2 status updates.
- [ADR-0022: Reading aids](0022-reading-aids.md): RSVP, bionic reading, spacing, fonts, the ruler, and more, as pure data.
  - Status: accepted, with 2 status updates.
- [ADR-0023: Piper voices and Whisper dictation in-process on RTen](0023-in-process-neural-speech.md): neural voices with word timing from the model, and in-process dictation, on a pure-Rust ONNX runtime.
  - Status: accepted.
- [ADR-0024: App core for the GUI](0024-app-core-for-the-gui.md): the document window, shared list and prompt state, the waker, the replace-range edit, the settings schema, and work moved off the input thread.
  - Status: accepted.
- [ADR-0025: Define word offline, and the message catalog](0025-lexicon-and-message-catalog.md): Open English WordNet and CMUdict in an fst and zstd file, and a Fluent-subset catalog with pseudo-locales.
  - Status: accepted.
- [ADR-0026: OCR, and formats for students](0026-ocr-and-student-formats.md): OCR of scanned pages (ocrs in process, Tesseract as the fallback), DAISY, PowerPoint, spreadsheets, archives, and web pages.
  - Status: accepted.

## Writing a new ADR

- Copy the shape of an existing one: `# ADR-NNNN: Title`, then `- Status:` and `- Date:` lines, then Context, Decision, Consequences, and See also.
- Get the date from the machine, never from memory.
- Add it to this index and to the [documentation index](../README.md#decisions).
- Record a bold choice together with its fallback (Wave 3's rule).

## See also

- [Architecture](../dev/architecture.md): how the decisions fit together.
- [Documentation index](../README.md)
