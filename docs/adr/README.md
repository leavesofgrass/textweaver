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
- [ADR-0014: GUI toolkit (wxDragon)](0014-gui-toolkit.md): the GUI spike's findings. The GUI moved to Xilem; the wxDragon spike was removed once the Xilem GUI passed its second screen-reader session.
  - Status: superseded by [ADR-0027](0027-xilem-gui.md). It has 4 status updates.
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
- [ADR-0027: Xilem GUI](0027-xilem-gui.md): the all-Rust GUI on Masonry, Vello, Parley, AccessKit, and winit; our own document widget, the pinned versions, the accessibility checks, and what to send upstream.
  - Status: accepted.
- [ADR-0028: The Xilem GUI after the first listening session](0028-xilem-gui-after-the-session.md): two ways to announce (a live region, or UI Automation notifications), every list option in the tree, and where the GUI's memory goes.
  - Status: accepted (the first listening session answered: the live region and the background highlight stay the defaults).
- [ADR-0029: MathCAT speech](0029-mathcat-speech.md): MathCAT 0.7.6-rc.3 as a second math speech engine on its own thread, EPUB 3 MathML read as math, and braille waiting for MathCAT issue #827.
  - Status: accepted, behind the `mathcat` feature; its two dependency exceptions were approved.
- [ADR-0030: Interface translations](0030-interface-translations.md): the Fluent-subset catalog extended to every message, Spanish, French, German, Brazilian Portuguese, and Arabic built in, right-to-left display, a voice per language, and never going silent.
  - Status: accepted.
- [ADR-0031: Native RTF, ODT, and Word revisions](0031-native-rtf-odt-and-word-revisions.md): our own iterative RTF parser, ODT on roxmltree, Word and ODT comments as notes, tracked changes read as the final text or said in place, and limits for zip packages.
  - Status: accepted.
- [ADR-0032: Grammar, lint, highlighting, and clipboard crates](0032-grammar-lint-highlighting-clipboard.md): our own Markdown lint instead of rumdl, arboard where OSC 52 cannot reach, Unicode math and notes export without new crates, and harper-core and syntect held on one advisory.
  - Status: accepted; grammar is built only with the `grammar` feature, and highlighting is on by default.
- [ADR-0033: The GUI after further accessibility testing, and edit mode](0033-gui-session-2-and-edit-mode.md): no console window, the system's file chooser for Open, text size and font keys, every button naming its key, and edit mode in the document view.
  - Status: accepted.
- [ADR-0034: The rope after measurement: stay on ropey 1.6](0034-rope-after-measurement.md): why ropey 2 and crop wait, measured on edit traces, and when to look again.
  - Status: accepted.
- [ADR-0035: Native LaTeX subset, and email and web archives](0035-latex-email-and-web-archives.md): our own LaTeX tokenizer and two-pass parser with its limits, email and MHTML through mail-parser, and MathML read as math in every web page.
  - Status: accepted.
- [ADR-0036: Math braille and navigation on MathCAT](0036-math-braille-and-navigation.md): MathCAT 0.7.6-rc.3 vendored with the fix for issue #827, Nemeth (default) and UEB math in BRF files wrapped to 40 cells, and exploring a formula with MathCAT's navigation and its braille on the status line.
  - Status: accepted, behind the `mathcat` feature; the status line's order waits for a further listening session.
- [ADR-0037: Offline intelligence, part 1: extractive summaries without a model](0037-extractive-summaries.md): LexRank in-house on TF-IDF, sampled long texts, `tw summarize` and Summarize, difficult-word definitions, the RSVP flash check, and why embeddings stay off.
  - Status: accepted.
- [ADR-0039: Automated screen-reader checks beside the listening sessions](0039-automated-screen-reader-checks.md): the accessibility tree dumped on three systems and compared with main, and NVDA, Orca, and VoiceOver sessions on CI runners, which never replace a human listening session.
  - Status: proposed; every check reports and none fails a job. Answers so far: NVDA through Guidepup and Orca read the GUI; the tree dump works on Windows and macOS.
- [ADR-0041: Publishing templates, real Word footnotes, and PDF page labels](0041-publishing-templates.md): APA, AMA, large print, dyslexia-friendly, high contrast, and manuscript templates for EPUB, Word, and PDF, real Word footnotes, an EPUB cover with alternative text, and PDF pages labelled with their print pages.
  - Status: accepted; a check with Word and JAWS is still queued.

## Writing a new ADR

- Copy the shape of an existing one: `# ADR-NNNN: Title`, then `- Status:` and `- Date:` lines, then Context, Decision, Consequences, and See also.
- Get the date from the machine, never from memory.
- Add it to this index and to the [documentation index](../README.md#decisions).
- Record a bold choice together with its fallback.

## See also

- [Architecture](../dev/architecture.md): how the decisions fit together.
- [Documentation index](../README.md)
