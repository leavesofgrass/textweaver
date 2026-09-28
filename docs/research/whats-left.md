# What is left: the inventory

Written on Sunday, September 27, 2026, by the roadmap planner (Fable 5.1), for the owner and the orchestrator. Wave 4's first sub-wave (W4h, W4s, W4c1) is running; nothing from Wave 4 has merged yet. The companion document is [the 2026 roadmap](roadmap-2026.md), which turns this inventory into milestones.

What it is built on: `CLAUDE.md`; `docs/roadmap.md`; the whole of `docs/history/tasks.md`; `docs/research/` (the Wave 4 orchestration plan with the owner's answers and the measured build sizes, the Wave 5 plan, the usability pass, the Wave 4 plan review and research, the Xilem and pure-Rust research); every ADR; `README.md`, `CHANGELOG.md`, `docs/README.md`, the quick start, and the feature guides; the audit of September 2026; `docs/star-gaps.md`; the git log; and the wiki's Star and textweaver pages, read only. Nothing was built, run, downloaded, or deleted.

**Versioning, from the owner.** textweaver keeps iterating alpha releases (0.1.0-alpha.4, alpha.5, and so on) until it is feature-complete and relatively stable. The final alpha is what most projects would call 1.0: feature-complete. Beta and stable versions come after that, only when the owner says. Every release happens only when he asks. So this inventory speaks of the "final alpha", not of a beta, and the roadmap plans readiness, not release dates.

## How to read each item

Every item has four facts, in this order:

- **Status:** done; in Wave 4 (with the agent); Wave 5 (with the agent from the Wave 5 plan, or "new" when this inventory adds it); Wave 6 or later; or unplanned.
- **From:** where the item came from: the roadmap, Star parity (`docs/star-gaps.md` and the wiki's Star pages), a UX finding (the usability pass or the owner's sessions), a deferred item (a wave's status line or "left" note), or an ADR follow-up.
- **Value** to the owner's goal, stated in words. His goal: terminal-first Markdown reading and authoring, with speech, Braille, and highlighting that never stall or lose your place, for students with print disabilities. High means it serves that goal directly or removes a way to stall or lose your place; medium means it serves students broadly; low means it is peripheral.
- **Size:** small (under a day), medium (a few days), large (one to two weeks).

"Done" items are listed only where they anchor what is left, so the reader can see the shape of each area. The full done list is in `CHANGELOG.md` and the status lines in `docs/history/tasks.md`.

## Where things stand today

- Newest release: 0.1.0-alpha.3, Friday, September 25, 2026 (Windows and macOS packages; no Linux package yet). `main` is far ahead of it: Wave 3 added the Xilem GUI, OCR and the student formats, Piper voices and in-process dictation, define word, profiles, and statistics.
- Tests at the end of Wave 3: about 1,933 native on Windows and 1,940 in Docker with all features. Fuzzing: 15 targets, all in the nightly matrix since commit `132382f`.
- Wave 4 (adopted plan) has three sub-waves: 4a (W4h terminal polish, W4s GUI session prep, W4c1 MathCAT speech), 4b (W4g authoring extras, W4b speed and memory, W4a2 the GUI after the session), 4c (W4c2 documents, W4d translations, W4a3 GUI edit mode, W4f platforms and CI as a light fourth). The owner's sessions 1 and 2 gate it.
- Wave 5 is planned in `docs/research/wave5-plan.md` and refined in the roadmap.

## Reading and speech

- **Speech never stalls, never loops, never dies silently.** Status: done (P1a, P2a): capped restarts, a speech-thread panic caught, hosts tied to the parent, a stall watchdog, audio device recovery, the engine's first start in the background. From: the roadmap. Value: high; it is the goal. Size: n/a.
- **Announcements queued until the engine is ready.** Status: in Wave 4 (W4h). From: UX finding 11. Value: high; a self-voicing user hears nothing at startup on a slow engine today. Size: small.
- **Spoken key names in textweaver's own voice.** Status: in Wave 4 (W4h). From: UX finding 1. Value: high; "Alt+." is silent at low punctuation. Size: medium.
- **"Say status" and "repeat last message" actions.** Status: in Wave 4 (W4h). From: UX finding 3. Value: high; the only way for a self-voicing user to hear the title line again. Size: small.
- **"Ready" before the first play; Escape in edit mode says how to finish; the palette's opening sentence; a key that repeats a list's introduction; `tw` with no arguments.** Status: in Wave 4 (W4h). From: UX findings 2, 4, 5, 6, 8. Value: medium; polish that a new user meets first. Size: small each.
- **Unbound keys stay silent.** Status: done by decision (the owner, September 27, 2026). From: UX finding 7. Value: n/a.
- **The owner's check by ear of the NVDA and JAWS settings in `docs/screen-readers.md`.** Status: in Wave 4 (session 1 covers the terminal reader; each setting is still marked "verify on the owner's machine"). From: deferred (P2c). Value: high; the guide is what another blind user follows. Size: small (the owner's time, then the orchestrator removes the marks).
- **Windows Terminal against the classic console with JAWS and NVDA, recorded.** Status: unplanned (the guide says "has not been recorded yet"). From: deferred (P2c). Value: medium. Size: small (one session note).
- **MathCAT speech (ClearSpeak and SimpleSpeak) as a second math engine.** Status: in Wave 4 (W4c1, ADR-0029). From: the roadmap and the Wave 4 research. Value: medium; math is what students meet in courses, and MathCAT is what NVDA and JAWS users already hear. Size: medium.
- **Formula navigation through MathCAT.** Status: Wave 5 (W5c4). From: deferred (W4c1's brief keeps the built-in tree). Value: medium. Size: medium.
- **Startup speed and lazy loading of engines, the lexicon, and fonts.** Status: in Wave 4 (W4b). From: the roadmap. Value: high; every start is heard. Size: medium.
- **Faster find and segmentation (memchr, aho-corasick, `icu_segmenter` measured).** Status: in Wave 4 (W4b). From: the roadmap. Value: medium. Size: medium.
- **The rope decision (ropey 2 or crop), measured in Wave 4, decided in Wave 5.** Status: in Wave 4 (W4b measures), Wave 5 (W5r decides, ADR-0034). From: the roadmap and ADR-0002. Value: high if a trace a user feels is slow; otherwise a closed question. Size: small to decide; large to migrate.
- **Difficult-word definitions from the lexicon at high verbosity.** Status: Wave 5 (W5s). From: Star parity (W4e's brief). Value: medium. Size: small.
- **Extractive summaries without a model (`tw summarize` and a Summarize command).** Status: Wave 5 (W5s, ADR-0037). From: Star parity. Value: medium; a study aid Star had. Size: small to medium.
- **PDF page navigation: go to a page, say the page.** Status: new for Wave 5 (W5x; the `PageBreak` markers exist, the go-to prompt does not take a page number). From: ADR-0010 follow-up. Value: high; PDF is students' main format, and citations and lecturers refer to pages. Size: small.
- **MathML in HTML input read as math.** Status: new for Wave 5 (W5c3, small). Today a converted HTML page reads a formula as its symbols run together (`docs/math.md`). W4c1 does the EPUB path only. From: a doc note. Value: medium. Size: small.
- **Omnivox word-level highlighting.** Status: unplanned. From: ADR-0003 follow-up. Value: low; Omnivox is a niche engine. Size: medium.
- **Streaming dictation (words appear while speaking).** Status: unplanned (Wave 6 candidate). From: ADR-0013 follow-up. Value: medium for authoring by voice. Size: large.
- **DECtalk tested against a licensed DECtalk.** Status: unplanned; needs a licensed copy. From: ADR-0021 follow-up. Value: low to medium (the owner's engine is Eloquence). Size: small once a copy exists.
- **Real-engine tests (SAPI, espeak-ng, Voxin) on runners that can run them; the 32-bit hosts tested.** Status: unplanned. From: the roadmap (Phase 2 "more coverage") and the audit. Value: medium; a fake engine hides a silent engine. Size: medium.
- **Tap Ctrl to pause.** Status: dropped for the terminal (no key-up); possible in the GUI. From: Star parity. Value: low. Size: small (GUI only).
- **SSML pauses; Coqui, Festival, Qt speech, ElevenLabs cloud voices.** Status: dropped. From: Star parity. Value: low.

## Braille

- **BRF output, UEB grade 1 in pure Rust; grade 2 through the optional `liblouis` feature.** Status: done (Agent M, ADR-0017). Value: high.
- **The Braille display pass over the terminal reader: meaning first on every status line and list item, short messages, the key fact in the first 40 cells, a test that keeps it so.** Status: Wave 5 (W5x). From: the Wave 5 plan and `CLAUDE.md`'s Braille rule. Value: high; this is what the owner's display shows. Size: small to medium.
- **A tested "Braille displays" section in `docs/screen-readers.md`, and the owner's Braille session B1 as a gate.** Status: Wave 5 (W5x writes, the owner tests). From: the Wave 5 plan; the guide says "has not been tested yet". Value: high. Size: small.
- **Math braille (Nemeth and UEB) in BRF and on the display, through MathCAT.** Status: Wave 5 (W5c4, ADR-0036), gated on MathCAT issue #827 or a vendored patch. From: the roadmap and the Wave 4 research. Value: high for a Braille-reading student in a math course. Size: medium.
- **Grade 2 (contracted) braille in pure Rust.** Status: Wave 6 or later. From: ADR-0017 follow-up. Value: medium; the display contracts through the screen reader already, so this is for BRF files and embossing; `liblouis` covers it where installed. Size: large.
- **BRF extras: BANA table formats beyond linear rows, typeform (bold, italic) indicators, the capitals passage indicator.** Status: unplanned. From: ADR-0017 follow-up. Value: medium for embossed course material. Size: medium.
- **Braille checks in the release listening checklist.** Status: new for Wave 5 (W5p adds the Braille lines from session B1 to the checklist in `docs/dev/releasing.md`). From: `CLAUDE.md` ("tested like speech"). Value: high. Size: small.
- **The GUI's caret line on the display during reading and editing.** Status: in Wave 4 (session 2 and W4a3's checklist), Wave 5 (session 3). From: the Wave 5 plan. Value: high. Size: the owner's sessions.

## Authoring

- **Edit mode with structure, outline, spell check, clipboard, citations while writing, templates, find and replace, export and preview from the reader.** Status: done (P1b, P2b). Value: high.
- **Grammar checking (harper-core) behind a feature.** Status: in Wave 4 (W4g, ADR-0032). From: the Wave 4 research. Value: high for a blind author who cannot skim. Size: medium.
- **Markdown lint and format (rumdl, or our own rules).** Status: in Wave 4 (W4g). From: the Wave 4 research. Value: medium. Size: medium.
- **Code highlighting in the terminal (syntect), never color alone.** Status: in Wave 4 (W4g). From: Star parity. Value: low to medium. Size: small.
- **A native clipboard fallback (arboard) where OSC 52 is unavailable.** Status: in Wave 4 (W4g). From: Star parity (the old console and macOS Terminal cannot copy today). Value: medium. Size: small.
- **Unicode math in the plain reading view.** Status: in Wave 4 (W4g). From: Star parity. Value: medium. Size: small.
- **Notes export to BibTeX, RIS, and JSON.** Status: in Wave 4 (W4g). From: Star parity. Value: medium. Size: small.
- **`y` and `n` in the export and citation "Open it?" lists.** Status: Wave 5 (W5x). From: UX finding 9. Value: low. Size: tiny.
- **Library search by DOI, ISBN, and author; document metadata on the library entry.** Status: Wave 5 (W5x). From: Star parity (`discovery.py`). Value: medium for a student's reading list. Size: small.
- **Star profiles imported by `tw migrate-star`.** Status: Wave 5 (W5x). From: deferred (W3e). Value: low (the owner's own migration). Size: tiny.
- **Study lists (definitions, statistics) on the app's list model, so the GUI gets them.** Status: Wave 5 (W5x). From: deferred (W3e). Value: medium. Size: small.
- **Typing over JSON-RPC (`insert`).** Status: unplanned. From: ADR-0015 follow-up. Value: low to medium (editor integration). Size: small.
- **Accessible publishing templates: Word templates for APA student papers and AMA manuscripts, an EPUB cover, Star's stylesheets (large print, dyslexia-friendly, high contrast, manuscript).** Status: Wave 6 candidate. From: Star parity (`publish.py`, 28 tests). Value: medium to high for a student handing in papers; APA is what courses ask for. Size: medium.
- **Real Word footnotes in DOCX output; SVG images in DOCX and PDF; page labels in PDF from print page breaks; `Lang` on spans; table header association beyond column scope.** Status: unplanned. From: ADR-0017 follow-up. Value: medium (the footnotes and page labels), low (the rest). Size: small each.
- **Citations: hayagriva's `csl-json` feature; known style differences (Vancouver, conference papers, Chicago and IEEE quirks).** Status: unplanned. From: ADR-0019 follow-up. Value: low to medium. Size: small.
- **Interface strings that name a key come from the keymap, never a fixed string (a test).** Status: in Wave 4 (W4h). From: UX finding 6. Value: medium. Size: small.
- **Study tools (spaced repetition, Anki), the knowledge graph exports, concept extraction.** Status: dropped. From: Star parity. Value: low for the goal.

## Formats

- **Native reading of text, Markdown, HTML, PDF, EPUB, DOCX, DAISY 3 and DTBook, PowerPoint, spreadsheets, archives, web addresses, images and scanned PDFs through OCR; Pandoc as a sandboxed fallback.** Status: done (Waves 1 to 3, ADR-0010, ADR-0026). Value: high.
- **RTF and ODT natively; DOCX comments and tracked changes as notes and revisions.** Status: in Wave 4 (W4c2, ADR-0031). From: the Wave 4 research. Value: medium to high; handouts and marked-up drafts. Size: large.
- **EPUB 3 MathML read as math.** Status: in Wave 4 (W4c1). From: the Wave 4 research. Value: medium. Size: small.
- **A native LaTeX subset (sections, lists, tables, cite, ref, footnotes, math).** Status: Wave 5 (W5c3, ADR-0035). From: deferred (Wave 4 review). Value: medium to high for course notes and problem sets. Size: medium.
- **EML and MHTML (emails and saved pages) through mail-parser.** Status: Wave 5 (W5c3). From: deferred (Wave 4 review). Value: medium. Size: small to medium.
- **PDF: better link targets and annotations, form fields, vertical and right-to-left scripts, CJK CMaps beyond Identity; captions marked by pattern.** Status: unplanned. From: ADR-0010 follow-up. Value: medium (links and annotations: lecturers' comments), low (the rest). Size: medium.
- **OCR: rotated scans, table structure, handwriting; `.xls`; password-protected archives; following links inside web pages.** Status: unplanned. From: ADR-0026 follow-up. Value: medium (rotated scans are common in library scans), low (the rest). Size: medium.
- **Markdown edge cases: inline extensions split by emphasis, block ids on tight list items; prices such as "$5-$10" read as math in `tw convert`.** Status: unplanned. From: ADR-0016 and ADR-0018 follow-ups. Value: low. Size: small.
- **Notebooks and source code as documents (beyond plain text).** Status: unplanned. From: Star parity. Value: low. Size: medium.
- **The reading fonts: Lexend downloaded when first chosen (`[fonts] fetch_missing` is stored and never used).** Status: new for Wave 5 (W5x, or W5a4 in the GUI's font chooser). From: Star parity and the Star lesson "a stored setting must work". Value: medium. Size: small.

## Math

- **LaTeX and ASCIIMath parsed, MathML in output, spoken math with exact highlighting, the exploration mode, syllables and difficult words in the terminal.** Status: done (Agent O, P2e). Value: high.
- **MathCAT speech.** Status: in Wave 4 (W4c1). See "Reading and speech".
- **Math braille and MathCAT navigation.** Status: Wave 5 (W5c4). See "Braille".
- **Unicode math in the plain view.** Status: in Wave 4 (W4g). See "Authoring".
- **MathML in HTML input.** Status: new for Wave 5 (W5c3). See "Reading and speech".
- **Fuzz targets for the LaTeX and ASCIIMath parsers.** Status: Wave 5 (W5m). From: the Wave 5 plan. Value: high (a student's own writing goes through them). Size: small.
- **Math in the GUI: verbosity and the ASCIIMath delimiter follow the settings.** Status: verify in Wave 4 (session 1). `docs/math.md` says the wxDragon spike ignores them; the Xilem GUI runs on the app core and should follow them. From: ADR-0018 follow-up. Value: medium. Size: small.

## The GUI

- **The Xilem GUI: `DocumentView`, the announcer, dialogs on the app's models, the settings dialog, the command palette, fonts, themes, windowing, packaging by hand, the UI Automation report, the AT-SPI check, the macOS smoke test.** Status: done (W3b, ADR-0027); not yet heard by the owner. Value: high for the many users who want a window; the owner works in the terminal.
- **The owner's session 1 (NVDA, then JAWS): the two designs (highlight as selection or background; live regions or a UIA notification).** Status: in Wave 4 (gate of 4a). From: the roadmap. Value: high. Size: the owner's time.
- **The UIA notification option, clipped options kept in the tree, memory growth attributed.** Status: in Wave 4 (W4s, ADR-0028 draft). From: the Wave 4 research. Value: high. Size: medium.
- **The session's fixes; a window slide that keeps the screen reader's place; the reading aids; parity essentials (outline, notes, access modes, tables, links); `docs/gui.md`.** Status: in Wave 4 (W4a2). From: the roadmap. Value: high. Size: large.
- **Edit mode on Parley 0.8, with citations, export, preview, and spell check in the GUI.** Status: in Wave 4 (W4a3, ADR-0033). From: the roadmap. Value: high. Size: large.
- **The wxDragon spike removed, with `gui.yml`.** Status: in Wave 4 (the orchestrator, after session 2). From: the owner's decision. Value: n/a. Size: small.
- **The Parley upgrade to 0.11 with the ADR-0027 measurements repeated.** Status: Wave 5 (W5a4). From: deferred (Wave 4 review). Value: medium; it moves the accessibility bridge into our code and unblocks ranged styles. Size: medium.
- **Syllables and difficult words drawn in the GUI.** Status: Wave 5 (W5a4). From: Star parity (`docs/settings.md` says "the GUI does not yet"). Value: medium. Size: small.
- **The voice manager dialog in the GUI.** Status: Wave 5 (W5a4). From: deferred (W3f). Value: medium. Size: small.
- **Drawn labels from the message catalog (the GUI in the five languages).** Status: Wave 5 (W5a4). From: deferred (W4d's brief). Value: medium. Size: small.
- **Memory growth (98 to 172 MB working set) fixed wherever it lives.** Status: in Wave 4 (W4s attributes; W4a2 may fix), Wave 5 (W5m takes what is left). From: ADR-0027 follow-up. Value: high. Size: medium.
- **The command palette and font chooser lists on the app's models.** Status: unplanned (ADR-0027 says they are still local). From: ADR-0027 follow-up. Value: low to medium (consistency). Size: small.
- **A system theme from the platform palette, and Windows High Contrast respected.** Status: partly (W3b follows the system's scheme; ADR-0020 still lists the platform palette as open). From: ADR-0020 follow-up. Value: medium for low-vision users. Size: small.
- **Upstream proposals: Masonry (AccessKit bump, action routing, labels, test fonts), AccessKit (UIA notification, text ranges that survive run changes, clipped nodes), offering `DocumentView` and the announcer.** Status: unplanned; after the owner's sessions. From: ADR-0027 follow-up. Value: medium (keeps the vendored patch small). Size: medium.
- **Orca heading navigation in the GUI (waits on AccessKit PR #758, AT-SPI Collection).** Status: blocked upstream. From: the Xilem research. Value: medium for Linux users. Size: small once merged.
- **VoiceOver and Orca by a person.** Status: unplanned; no Mac, no Linux desktop tester. From: the roadmap. Value: medium. Size: a tester's time.
- **GUI Contents and Notes panels.** Status: in Wave 4 (W4a2's outline and notes list). From: Star parity. Value: medium. Size: covered.
- **Tap Ctrl to pause in the GUI.** Status: unplanned. Value: low. Size: small.

## The terminal reader and the CLI

- **The reader: NVDA and JAWS style keys by default, the classic preset, three access modes, the settings screen, lists and prompts on the app's models, large files opened in the background.** Status: done. Value: high.
- **Terminal polish (W4h's list).** Status: in Wave 4. See "Reading and speech".
- **`tw search --json` and `tw info --json` on a closed pipe.** Status: in Wave 4 (W4h). From: UX finding 10. Value: low. Size: tiny.
- **Settings that are stored but not typed or not used: `[speech.dectalk]`, `[speech.voice_params]`, and the engine sections that `tw settings export` skips; `[fonts] fetch_missing` never read.** Status: new for Wave 5 (W5x, tiny; the store is its). From: `docs/speech.md` and `docs/settings.md`; the Star lesson. Value: medium; the "every setting is used" test was meant to catch this. Size: small.
- **AltGr characters on non-US layouts.** Status: done (CHANGELOG "Fixed"). From: the audit. Value: medium.
- **A friendlier `tw` with no arguments; the quick start's `q` line; `py -3` in the docs.** Status: in Wave 4 (W4h). Value: low. Size: tiny.
- **The lexicon loaded off the input thread on the first "define word".** Status: Wave 5 (W5x; in the Wave 5 plan it also appears under W5m's branch 2, which this inventory resolves: W5x owns it). From: deferred (W3e). Value: medium; a 10 MB file read on the input thread stalls the reader once. Size: small.
- **Interface translations: Spanish, French, German, Portuguese, Arabic; right-to-left display behind a setting; a first-run language choice; per-language default voices that never go silent; the pseudo-locale check in CI.** Status: in Wave 4 (W4d, ADR-0030). From: Star parity. Value: medium; Star had them, and the catalog is built (W3e). Size: large.
- **An update check inside the program; a guided tour; a key-code inspector.** Status: unplanned (partly: the update scripts and the quick start exist). From: Star parity. Value: low. Size: small.
- **Feeds, Wikipedia, and PubMed quick open; plugins; karaoke video export; OGG output and the M4B cover.** Status: dropped. From: Star parity. Value: low.

## Accessibility checks

- **The UI Automation report (Windows), the AT-SPI dump under Xvfb (Linux), the macOS smoke test, the terminal contrast test, the keyboard checks (conflicts, reachability, WCAG 2.1.4).** Status: done. Value: high.
- **The Orca check in CI for the Xilem GUI (`gui-xilem.yml`).** Status: in Wave 4 (W4f). From: the roadmap. Value: medium. Size: small.
- **Automated screen-reader tests: the accessibility-cli tree dump on three systems as a CI artifact with a diff; the Guidepup NVDA spike; an Orca script under Xvfb; a VoiceOver attempt on a macOS runner.** Status: Wave 5 (W5t, ADR-0039 if standing). From: deferred (Wave 4 review). Value: medium; a green run never replaces the owner's session, but a tree diff catches regressions nobody hears. Size: medium.
- **A photometric check of RSVP at high rates against WCAG 2.3.1 (flashing).** Status: new for Wave 5 (W5s, which touches `textweaver-aids`; a data test, no screen). From: ADR-0022 follow-up. Value: medium; a safety check for photosensitive users. Size: small.
- **Tagged PDF checked with veraPDF or PAC; EPUB with epubcheck; grade 2 braille with liblouis in CI.** Status: unplanned (the tools are not installed; they could run in the container or a CI job). From: ADR-0017 follow-up. Value: medium; PDF/UA claims should be checked by a second tool. Size: small to medium.
- **Braille display checks in every report's checklist, and in the release listening checklist.** Status: Wave 5 (the common rules; W5p). From: the Wave 5 plan. Value: high. Size: small.
- **The `live-region` crate unused by `textweaver-a11y`.** Status: unplanned; moot once the wxDragon spike goes. From: ADR-0001 follow-up. Value: low. Size: tiny (remove the dependency).

## Translations

- **The Fluent-subset catalog with English, `en-XA`, and `ar-XB`; the study features' messages in it.** Status: done (W3e, ADR-0025). Value: medium.
- **Every interface string in the catalog; the five languages; right-to-left; the first-run choice; per-language voices.** Status: in Wave 4 (W4d). See "The terminal reader and the CLI".
- **Every new message after W4d is a catalog id; the checker allows English fallback with a "needs translation" list.** Status: Wave 5 (step 0 and the common rules). From: the Wave 5 plan. Value: medium. Size: small.
- **The GUI's drawn labels from the catalog.** Status: Wave 5 (W5a4). See "The GUI".
- **Document translation offline (OPUS-MT on rten).** Status: Wave 5 (W5e) only with the owner's approval of the model downloads; otherwise Wave 6. From: Star parity (`translate.py`). Value: medium for students reading in a second language; low for the owner's own goal. Size: large.
- **A switch to `fluent-bundle` if a language needs attributes, functions, or number formatting.** Status: unplanned unless W4d finds the need. From: ADR-0025 follow-up. Value: low. Size: medium.

## Platforms and packaging

- **Windows x86_64 zip, macOS universal tarball (ad hoc signed, not notarized), Linux x86_64 AppImage with zsync and a tarball (built, not yet released), checksums, provenance attestations, install and update scripts.** Status: done (P1c, P2d, W3-era). The AppImage waits for the next release. Value: high.
- **The aarch64 AppImage on GitHub's arm64 runners.** Status: in Wave 4 (W4f). From: deferred (P2d). Value: medium (Raspberry Pi and arm laptops). Size: small.
- **The GUI's packages (Windows zip with `textweaver-gui.exe`, macOS `.app`, Linux tarball and AppImage) in the release workflow.** Status: Wave 5 (W5p) if the owner says the GUI ships as supported. From: ADR-0027 follow-up. Value: high for GUI users. Size: small.
- **Signing: Windows Authenticode and Apple notarization.** Status: unplanned; needs paid certificates. From: the roadmap ("when funding allows"). Value: medium; SmartScreen and Gatekeeper warnings frighten students. Size: small once funded.
- **macOS tested by a person (the Apple backends, VoiceOver, Terminal.app).** Status: unplanned; no Mac. From: ADR-0008 follow-up. Value: medium; macOS is a shipped package that nobody has heard. Size: a tester's time.
- **The 32-bit Linux DECtalk host (`i686`).** Status: unplanned. From: ADR-0021 follow-up. Value: low. Size: small.
- **Docker parity: the container, per-agent volumes, the shared cache, the hygiene script.** Status: done (Wave 4 prep). Value: high for the method.
- **A merge gate (pull requests with required checks).** Status: declined by the owner for now. From: the roadmap. Value: low while one person merges. Size: small.
- **Pruning merged branches.** Status: unplanned; only after a wave, only with the owner's approval, never by an agent. From: the roadmap. Value: low. Size: tiny.
- **Windows on arm64.** Status: unplanned. From: nowhere yet. Value: low today. Size: small (a runner and a job).

## Quality: tests, fuzzing, performance, memory

- **The benchmark gate on peak heap and allocations, the soak test, Miri, AddressSanitizer, release-mode tests, the MSRV check, `cargo hack`, `cargo-deny`, Dependabot, the keyboard and deps checks, the TUI render tests, the loader property tests.** Status: done (P2d, P1d, P1b). Value: high.
- **Fuzz targets in the nightly matrix: markdown, html, epub, docx, pdf, settings, keymap, state, frame, daisy, pptx, sheet, archive, image, web (15).** Status: done (commit `132382f`; the Wave 5 plan's "nine" is out of date). Value: high.
- **Fuzz targets for `rtf`, `odt`, `docx_revisions`.** Status: in Wave 4 (W4c2, with W4f adding the matrix lines). Value: high. Size: small.
- **Fuzz targets for the math parsers, the citation importers, the theme reader, the lexicon file, the vault importer, the JSON-RPC decoder; `cargo xtask fuzz-seed`.** Status: Wave 5 (W5m). From: the Wave 5 plan. Value: high; a hostile file must never take the reader down. Size: medium.
- **Fuzz targets for the LaTeX and EML loaders.** Status: Wave 5 (W5c3). Value: high. Size: small.
- **Peak allocation counter and `--log` memory numbers as tools.** Status: in Wave 4 (W4b). Value: high. Size: small.
- **Allocation and peak-memory cuts; binary size with `publish` on and off; one concrete cut.** Status: in Wave 4 (W4b). From: the roadmap. Value: medium. Size: medium.
- **Bundling only the CSL styles used.** Status: unplanned (hayagriva's archive is all or nothing). From: deferred (P2d). Value: low. Size: medium.
- **Timing tests that wait for signals, run 40 times under load.** Status: done as a rule (Wave 4 lessons). Value: high.
- **`WordTrack` built off the input thread for large documents.** Status: unplanned. From: ADR-0022 follow-up. Value: medium; a stall on a large file with difficult words on. Size: small.
- **The rope proptests for ADR-0002's invariants.** Status: Wave 5 (W5r). Value: high. Size: small.
- **Two `ruzstd` encoder bugs reported upstream.** Status: unplanned. From: ADR-0025 follow-up. Value: low. Size: tiny.

## Documentation

- **The user guides, the developer docs, the ADR index, the interactive site, the link and site checks.** Status: done (Agents Y, DOC2, W3c). Value: high.
- **The doc pass against the code, file by file, before the release notes.** Status: in Wave 4 (W4f, a listed set of guides). From: the roadmap. Value: high; a guide that lies costs a blind user an hour. Size: medium.
- **Stale passages found today:** `README.md`'s "Coming next, in Wave 3" (all of it landed); `docs/roadmap.md`'s status ("Wave 3 is planned", "P2e is running"); `docs/README.md` (ADR-0023 missing from the Decisions list, ADR-0027 out of order, "29 crates" against 33 folders, "Roadmap: Phases 1 and 2 (done), and Wave 3"); `docs/screen-readers.md`'s GUI section, which still describes the wxDragon preview; `CHANGELOG.md`'s Unreleased section, which does not name the Xilem GUI, Piper voices, in-process dictation, or define word; the ADR index's status line for ADR-0014 (the file still says "proposed") and its count for ADR-0020. Status: in Wave 4 (W4f) where its list covers them; Wave 5 (W5p, and W5x for the screen-reader guide) for the rest. Value: high. Size: small.
- **`cargo xtask settings-doc --check` (the settings reference generated from the schema) and `cargo xtask docs --check` (the ADR index, the Decisions list, the crate count, "See also" on every guide).** Status: Wave 5 (W5p). From: the Wave 5 plan. Value: high; the docs that drift most get a tool. Size: small.
- **`docs/gui.md`: how to start the GUI, its keys, its settings.** Status: in Wave 4 (W4a2). Value: high for GUI users. Size: small.
- **The Braille section of the screen-reader guide.** Status: Wave 5 (W5x). See "Braille".
- **Release notes for the next alpha, grouped by area, "Keys: what changed" kept first.** Status: in Wave 4 (W4f writes them for alpha.4), each later wave for its alpha. From: the roadmap. Value: high. Size: small.
- **The wiki's Wave 4 and Wave 5 pages filled in by the orchestrator.** Status: ongoing (the orchestrator only). Value: medium.

## Releases

- **0.1.0-alpha.3 (Friday, September 25, 2026).** Status: done. The newest release.
- **0.1.0-alpha.4: the first release with the Linux AppImage, the NVDA and JAWS style keys, Wave 3 (the Xilem GUI as experimental, OCR and the student formats, Piper, in-process dictation, define word, profiles, statistics), and Wave 4.** Status: readiness prepared in Wave 4 (W4f: the release workflow run on a branch, the dated listening checklist, the notes). The owner releases when he says. Value: high. Size: the readiness page is small.
- **`cargo xtask release` checks that the listening checklist is dated.** Status: in Wave 4 (W4f). Value: medium. Size: small.
- **0.1.0-alpha.5 and later: one readiness point per wave.** Status: Wave 5 (W5p, "alpha release readiness"), then each wave. Value: high. Size: small per wave.
- **The feature-complete final alpha, then beta and stable when the owner starts them.** Status: planned in the roadmap with explicit criteria. Value: high.

## The Star features textweaver still lacks

From `docs/star-gaps.md` (checked against Star's code on September 26, 2026) and the wiki's Star pages (the hub, the features reference, the usage guide, the release history to 0.1.31). Each says where it stands now.

What the wiki says users relied on. There is no "most used features" page. The evidence is the owner's own testing and the audits: word highlighting that stays in sync (the reason textweaver exists); a default engine that never goes silent; screen reader habits (caret browsing, follow-scroll, the highlight colors); the features that could not fail through a missing package (dictate, transcribe, summarize, translate, difficult words, spell check); publishing one real coursework document end to end; and tables in dense documents, which Star's audit called the finding with the largest real-world cost. textweaver already covers each of these except summaries, translation, and the publishing templates.

Star features the wiki lists that `docs/star-gaps.md` does not:

- **Speed presets** (skim 350, normal 265, study 200, slow 150 words per minute, cycled by one key). Status: missing; textweaver has rate keys and per-voice rates. Value: medium; a study habit. Size: small. Unplanned; a Wave 6 quick win.
- **Lightweight markup read natively** (reStructuredText, AsciiDoc, Org, MediaWiki, Textile, Creole, R Markdown, Jupyter notebooks). Status: partly; `tw convert` sends them to Pandoc when it is installed. Value: low to medium (Org and Jupyter for some students). Size: medium each. Unplanned.
- **RAR archives.** Status: missing (zip, tar, and 7z are done). Value: low. Size: small. Unplanned.
- **A pronunciation dictionary, code skipped in speech, the highlight lead.** Status: done (`[speech] pronunciations`, `[speech] skip_code`, `[highlight] lead_words`).
- **Five highlight styles with lead and lag, and follow-scroll at 40 percent of the view.** Status: partly; textweaver has word and sentence colors and the lead; the styles and the scroll position are not settings. Value: low. Size: small. Unplanned.
- **Publishing (Star's F9): styled EPUB, DOCX, and single-file HTML with the accessibility templates and CSL citations.** Status: partly; see the templates item below.

Still missing, and planned:

- Grammar checking, Markdown lint, code highlighting, the native clipboard, Unicode math in the reading view, notes export to BibTeX, RIS, and JSON: Wave 4 (W4g).
- RTF and ODT natively, Word comments and tracked changes: Wave 4 (W4c2).
- Interface translations in five languages, right to left: Wave 4 (W4d).
- Reading aids drawn in the GUI, the GUI's Contents and Notes panels (the outline and the notes list), GUI edit mode: Wave 4 (W4a2, W4a3).
- Summaries (LexRank): Wave 5 (W5s).
- Document metadata edited per document and searched in the library: Wave 5 (W5x).
- Star profiles imported: Wave 5 (W5x).
- Syllables and difficult words drawn in the GUI; the voice manager in the GUI: Wave 5 (W5a4).
- Document translation: Wave 5 (W5e) with approval, else Wave 6.
- LaTeX input without Pandoc: Wave 5 (W5c3). Star sent LaTeX to Pandoc; textweaver's subset reads course notes natively.
- Lexend and other reading fonts downloaded when first chosen: Wave 5 (new).

Still missing, and not planned:

- The accessible publishing templates: Word templates for APA student papers and AMA manuscripts, the EPUB cover, and Star's stylesheets (large print, dyslexia-friendly, high contrast, manuscript). Wave 6 candidate; the most valuable of the unplanned Star items for a student.
- Captions marked by pattern in PDF reading order.
- Notebooks and source code as structured documents.
- The knowledge graph exports (SVG, DOT, PlantUML, JSON) and concept extraction.
- Karaoke video export.
- Feeds, Wikipedia, and PubMed quick open.
- SSML pauses; Coqui, Festival, Qt speech, and cloud voices.
- OGG output and the M4B cover image.
- Plugins; an update checker inside the program; a guided tour; a key-code inspector; tapping Ctrl to pause.
- Study tools (spaced repetition, Anki): dropped by plan.

Done since the gaps list was written, so the list is stale there: define word, reading statistics, settings profiles, OCR, DAISY, archives, web addresses, PPTX, spreadsheets, Piper voices, the voice manager in the terminal, in-process dictation, syllables and difficult words in the terminal, math exploration. W5p's doc pass should refresh `docs/star-gaps.md`.

## The items no plan covers yet

Ranked by value to the owner's goal. Each names the wave the roadmap proposes.

1. **PDF page navigation** (go to page, say the page, pages in the position report). High. Wave 5, W5x.
2. **Settings stored but never used or never typed** (`[fonts] fetch_missing`; `[speech.dectalk]`, `[speech.voice_params]`, and the untyped engine sections). Medium, and a Star lesson. Wave 5, W5x.
3. **MathML in HTML input.** Medium. Wave 5, W5c3.
4. **The stale passages listed under Documentation** that W4f's list does not cover. High. Wave 5, W5p and W5x.
5. **The RSVP flashing check (WCAG 2.3.1).** Medium. Wave 5, W5s.
6. **The accessible publishing templates** (APA and AMA Word templates, the EPUB cover, the stylesheets). Medium to high. Wave 6.
7. **`WordTrack` off the input thread** for large documents with difficult words on. Medium. Wave 6.
8. **PDF link targets, annotations, and form fields.** Medium. Wave 6.
9. **Rotated scans in OCR.** Medium. Wave 6.
10. **Tagged PDF and EPUB checked by a second tool (veraPDF or PAC, epubcheck), and liblouis grade 2 in CI.** Medium. Wave 6.
11. **Real-engine tests on runners that can run them, and the 32-bit hosts tested.** Medium. Wave 6 or the stabilization wave.
12. **Streaming dictation.** Medium. Wave 6 or later.
13. **BRF extras** (BANA tables, typeform indicators, the capitals passage indicator) and **grade 2 in pure Rust.** Medium. Wave 6 or later.
14. **macOS and Linux heard by a person** (VoiceOver, Orca, Terminal.app, GNOME Terminal). Medium. Needs a tester or a Mac; a question for the owner.
15. **Upstream proposals to Masonry and AccessKit.** Medium. After the owner's sessions.
16. **Windows Terminal against the classic console, recorded.** Medium. Session 1's write-up.
17. **The command palette and font chooser lists on the app's models; the system theme from the platform palette.** Low to medium. Wave 6.
18. **Typing over JSON-RPC.** Low to medium. Wave 6 or later.
19. **DECtalk against a licensed copy; the `i686` DECtalk host.** Low. When a copy exists.
20. **Omnivox word highlighting; the `live-region` dependency removed; the `ruzstd` bugs reported; the CSL styles bundle; Windows on arm64.** Low. Whenever an agent has a free hour.

## See also

- [The 2026 roadmap](roadmap-2026.md): the milestones, the critical path, Wave 5 refined, and the questions for the owner.
- [Wave 5 plan](wave5-plan.md): the briefs this inventory checks.
- [Wave 4 orchestration plan](wave4-orchestration.md): what is running now, with the owner's answers.
- [Star features not yet planned](../star-gaps.md): the parity list.
- [Roadmap](../roadmap.md): the phases up to Wave 3.
- [Tasks and agent briefs](../history/tasks.md)
- [Documentation index](../README.md)
