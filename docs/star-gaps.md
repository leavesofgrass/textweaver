# Star features not yet planned

This inventory was made on Friday, September 25, 2026. It lists Star features (D:\star) that `docs/history/plan.md` and `docs/history/tasks.md` did not cover then. For each one it gives the Star code location, and the test file and count where Star has tests. Priorities are for students with print disabilities.

Each item now has a **status**, checked against `main` on Saturday, September 26, 2026:

- **done**: textweaver does it.
- **partly**: some of it is done; the note says what is missing.
- **missing**: not done yet. The [roadmap](roadmap.md) says which of these come next.
- **dropped**: out of scope on purpose.

Updated after Phase 2 on Saturday, September 26, 2026. Items that Wave 3 takes on say which agent; the briefs are in [tasks.md](history/tasks.md). Wave 3 prefers pure-Rust, in-process solutions, so several items below name the Rust crate planned in place of Star's Python or C dependency.

**Reconciled against `main` on Monday, September 28, 2026** (`0.1.0-alpha.4`), using [what is left](research/whats-left.md#the-star-features-textweaver-still-lacks)'s "done since the gaps list was written" note and the Wave 4 entries in `CHANGELOG.md`. That note lists define word, reading statistics, settings profiles, OCR, DAISY, archives, web addresses, PPTX, spreadsheets, Piper voices, the voice manager in the terminal, in-process dictation, syllables and difficult words in the terminal, and math exploration as done since this list was first written; Wave 4 additionally shipped interface translations, the native clipboard fallback, notes export, Unicode math in the reading view, and syntax highlighting in the terminal. The rows below carry each one forward; `research/whats-left.md` stays the newer, fuller inventory for anything not covered here.

## Covered by Wave 2

These areas were being built when the list was made. All are now on `main`:

- Themes (Agent Q). Status: done. See [themes.md](themes.md).
- Math (Agent O). Status: done. Math is spoken with exact highlighting and becomes MathML in HTML. See [math.md](math.md).
- Citations (Agent P). Status: done in the reader since Phase 2 (Agent P2b): Alt+C inserts from a filtered picker with a locator, Alt+Shift+D adds by DOI or ISBN, and the palette inserts a bibliography and checks citations. Continuous reading skips citations by default, and says them in words with Alt+Shift+Q (Agent P2e). See [citations.md](citations.md).
- Batch conversion and folder watching (Agents A2 and L). Status: done. See [converting.md](converting.md).
- Braille output (Agent M). Status: done for grade 1; grade 2 needs the `liblouis` feature and liblouis installed. See [converting.md](converting.md).
- Large-document paging, which plan §6.1 handles by design. Status: done for the terminal reader, which plans reading in windows. The GUI's window model is planned for Wave 3 (Agent W3a), for the Xilem GUI (Agent W3b).

## Core: plan these next

- **Reading aids.** Star's wiki page "Accessibility and WCAG reading aids" describes them. [reading-aids.md](reading-aids.md) is the user guide.
  - **RSVP.** Shows one word at a time, at 9 screen positions, with the words before and after. Code: `gui/main_window.py:62-262`, `gui/mixin_aiddialogs.py`, `tui/mixin_rsvp.py`. Tests: `test_rsvp.py` (19). Status: done.
  - **Reading ruler.** A band that follows the caret line. Code: `gui/main_window.py:263-349`. Status: done.
  - **Text spacing** (WCAG 1.4.12): line height and letter and word spacing. Code: `gui/mixin_aiddialogs.py:256-333`. Status: done. The terminal approximates it with blank lines and spaces.
  - **Bionic reading:** word starts in bold. Code: `gui/mixin_fontspacing.py:265-300`. Status: done.
  - **Syllable splitting:** `read·a·bil·i·ty`, shown only. Code: `syllables.py` (uses pyphen). Tests: `test_syllables.py` (11). Status: done in the terminal reader (Alt+Shift+Z); the `textweaver-aids` library does it by rules, with no dictionary. Not yet drawn in the Xilem GUI; planned for Wave 5 (W5a4).
  - **Current-line highlight.** Code: `gui/mixin_commands.py:353-362`. Status: done.
  - **Reading fonts:** OpenDyslexic, Atkinson Hyperlegible, and Lexend, downloaded when first chosen. Code: `fonts.py`. Tests: `test_fonts.py` (12). Status: partly. OpenDyslexic and Atkinson Hyperlegible Next and Mono are bundled and used for PDF and EPUB output, and loaded straight into Parley for the Xilem GUI, where Ctrl+D chooses the font (the wxDragon spike this row used to describe is removed). Lexend still must be installed by hand; downloading it when first chosen is unplanned, a Wave 5 candidate.
- **DAISY 3 / DTBook** (Bookshare), including DAISY zips. Code: `documents/ebook.py:122-232`. Status: done (Wave 3, Agent W3d), in spine order with NCX navigation.
- **OCR** of images and scanned PDF pages, with Tesseract. It runs only on pages with no text layer. Code: `documents/misc.py:7-23`, `documents/pdf.py:183-195`. Status: done (Wave 3, Agent W3d): the pure-Rust `ocrs` engine in process by default (models downloaded once with `tw ocr download`, checked by SHA-256), with a Tesseract subprocess as the fallback for other languages ([ADR-0026](adr/0026-ocr-and-student-formats.md)). `tw ocr status` and `tw ocr read FILE`. Rotated scans are unplanned, a Wave 6 candidate.
- **Better PDF reading order:** remove running headers, footers, and page numbers, and mark captions. Code: `documents/pdf.py:7-161`. Tests: `test_pdf_layout.py` (9). Status: done, except that captions are not marked by pattern ([ADR-0010](adr/0010-pdf-loader.md)).
- **Archives:** ZIP and TAR, with 7z and RAR optional. Opening one lists its readable files, and `book.zip!inner.pdf` opens a file inside. Code: `archive.py`. Tests: `test_archive.py` (16). Status: done (Wave 3, Agent W3d): ZIP and TAR, with 7z. RAR is unplanned.
- **Opening a web page by URL.** Code: `documents/misc.py:24-56`. Status: done (Wave 3, Agent W3d).
- **PPTX:** slide titles become headings, speaker notes follow each slide, and images get descriptions. Code: `documents/office.py:266-367`. Status: done (Wave 3, Agent W3d).
- **Offline dictionary:** WordNet definitions, synonyms, and pronunciation, plus a custom glossary. Code: `dictionary.py`. Tests: `test_dictionary.py` (17). Status: done (Wave 3, Agent W3e): your glossary, then Open English WordNet 2025 through morphy, with CMUdict pronunciations respelled for reading aloud, all offline (Ctrl+Shift+D or Alt+E, and `tw define`; [reading.md](reading.md#define-a-word-ctrlshiftd-or-alte)).
- **Spell check in edit mode.** Code: `spellcheck.py`. Status: done (Phase 2, Agent P2b) on SCOWL with a personal word list: next and previous misspelling, spelled aloud, suggestions, and a count on save. See [editing.md](editing.md#spelling).
- **Accessible publishing templates:**
  - Large Print, Dyslexia-friendly, High Contrast, and academic-manuscript stylesheets;
  - Word templates for APA student papers and AMA manuscripts;
  - an EPUB cover and a table-of-contents depth setting;
  - single-file HTML.
  - Code: `publish.py`, `publish_styles/`. Tests: `test_publish.py` (28).
  - Status: partly. `tw convert` writes single-file accessible HTML (following the system's dark, light, and high-contrast settings), large-print PDF, PDF in OpenDyslexic, and a PDF table of contents with a depth option. Since Phase 2 (Agent P2b) the reader exports the open document to HTML, PDF, Word, EPUB, and braille from the command palette, previews it in the browser, and starts a new document from a template with front matter and a References heading. The Word templates, the EPUB cover, and Star's stylesheets are missing.
- **GUI Contents and Notes panels.** Code: `gui/mixin_toc.py`, `gui/mixin_annotations.py:51-110`. Status: missing in the GUI; the Xilem GUI's dialogs are planned for Wave 3 (Agent W3b). The terminal reader lists notes (Shift+A), moves by heading, and lists the headings with type-to-filter (the outline, Alt+O).
- **Math as Unicode in the plain reading view** (`x²`, `√2`). Code: `mathrender.py`. Status: missing. The reader shows the LaTeX source and speaks it as English; HTML output has MathML.

## Medium

- **Formats:** spreadsheets (XLSX, CSV, and TSV, as tables); notebooks and source code; Unicode math in the text view. Status: partly. Spreadsheets (Wave 3, Agent W3d) and Unicode math in the reading view (`x²`, `√2`; `[reading] math_display = "unicode"`, Wave 4) are done. Notebooks and source code as structured documents are unplanned; source code still opens as plain text.
- **Summarize:** extractive, with LexRank. Code: `summarize.py`. Status: missing.
- **Translate** a document. Code: `translate.py`. Tests: 11. Status: missing.
- **Difficult-word overlay,** by word frequency. Code: `vocab.py`. Status: done in the terminal reader (Alt+Shift+J), on the `textweaver-aids` library's rules with SCOWL's word levels built in. Not yet drawn in the Xilem GUI; planned for Wave 5 (W5a4).
- **Reading statistics:** time read, progress, and sessions. Code: `stats.py`. Status: done (Wave 3, Agent W3e): time read aloud, the furthest point, and sessions per document, with a most-read list (Ctrl+Shift+Y or Alt+Y), `tw stats`, an opt-out, and Star's statistics imported ([reading.md](reading.md#reading-statistics-ctrlshifty-or-alty)).
- **Settings profiles:** named sets of voice, theme, and spacing settings, with import and export. Code: `gui/mixin_presets.py`. Status: done (Wave 3, Agent W3e): named profiles of voice, rate, theme, font, spacing, highlight, and access mode, switched, saved, renamed, deleted, imported, and exported in the reader (Ctrl+Shift+U or Alt+U) and with `tw settings profile` ([settings.md](settings.md#settings-profiles)). Star's profiles are not imported yet.
- **Piper neural voices,** with a catalog of voices to download, and a **Voice Manager** that lists, previews, and marks favorites. Code: `tts/piper*.py`, `gui/mixin_voices.py`. Status: done for voices (Wave 3, Agent W3f): Piper voices run in process on RTen, a pure-Rust ONNX runtime, with word timing from the model ([ADR-0023](adr/0023-in-process-neural-speech.md)); Choose voice (Alt+V) lists them, speaks a sample, and marks favourites, which are listed first (Phase 1, Agent P1b, extended to Piper in Wave 3). A voice manager across every engine in the Xilem GUI is planned for Wave 5 (W5a4); the terminal's choose-voice list is done.
- **Interface translations:** Spanish, French, German, Portuguese, and Arabic, with right-to-left layout. Code: `i18n.py`, `locale/`. Tests: 44. Status: done (Wave 3 Agent W3e's message catalogue, extended to every message by Wave 4 Agent W4d, [ADR-0030](adr/0030-interface-translations.md)): `[interface] language` speaks and shows textweaver's own words, lists, help, keyboard shortcuts, the command palette, and the settings screen in all six languages, the voice follows the language when the engine has one, and right-to-left text is reordered for display where the terminal does not do it itself.
- **Clipboard copy,** including the terminal escape code that works over SSH. Code: `tui/mixin_docops.py:18-145`. Tests: 13. Status: done. Ctrl+C copies the selection, or the sentence at the cursor, through the terminal's OSC 52 code, which works over SSH (Phase 1, Agent P1b); edit mode also has cut, paste, and select all (Phase 2, Agent P2b). Wave 4 (Agent W4g) added a native clipboard fallback (`arboard`) for terminals without OSC 52 (the old Windows console, macOS Terminal, GNOME Terminal and other VTE terminals), said the first time it is used; SSH and tmux still use the terminal's own code.
- **Dependency report** (`--deps`) and a **crash log.** Code: `diagnostics.py`, `gui/runner.py:55-85`. Status: done. `scripts/doctor.sh` and `scripts\doctor.ps1` print a system report, and textweaver writes a rotating log file ([troubleshooting.md](troubleshooting.md)).
- **Document metadata:** edit it per document, and search the library by it. Code: `discovery.py`. Tests: 24. Status: missing. Library search covers titles, paths, and text.
- **Notes export** as BibTeX, RIS, JSON, or plain text. Status: done (Wave 4, Agent W4g): `tw marks FILE --export FORMAT` writes BibTeX, BibLaTeX, RIS, or CSL-JSON records for Zotero or Pandoc. Notes and highlights also export to an Obsidian vault as Markdown ([vault.md](vault.md)), and since Phase 2 as a Markdown study sheet grouped by the document's headings ([notes.md](notes.md#export-a-study-sheet)). Plain-text export is not a separate format; CSL-JSON or the study sheet cover that need.
- **Study tools,** out of scope under plan §2:
  - the spaced-repetition scheduler (FSRS) and review screen (`sr.py`, `gui/mixin_review.py`);
  - AnkiConnect sync and `.apkg` export.
  - Status: dropped.

## Peripheral

- **Knowledge graph and concept extraction.** Typed links between notes; exports to SVG, DOT, PlantUML, and JSON. Code: `graph.py`, `ner.py`. Status: partly. Vault import and export keep typed links between notes; the graph exports and concept extraction are missing.
- **Karaoke video export.** Code: `video.py`. Tests: 18. Status: missing.
- **Feeds, Wikipedia, and PubMed** quick open. Status: missing.
- **More engines:** Coqui, Festival, Qt speech, and ElevenLabs cloud voices. Status: missing. speech-dispatcher, which can drive Festival, and DECtalk were added instead.
- **SSML pauses.** Status: missing.
- **Audio export to OGG and AAC,** and an M4B cover image. Status: partly. M4B audiobooks are AAC; OGG and the cover image are missing.
- **Infrastructure:**
  - plugins. Status: missing.
  - an update checker. Status: partly. `scripts/update.sh` and `scripts\update.ps1` update an installed textweaver; there is no check inside the program.
  - a guided tour and a welcome page. Status: partly. The [quick start](quickstart.md) opens as a document; there is no tour.
  - a key-code inspector. Status: missing.
  - line numbers and syntax highlighting in the terminal. Status: done. Line numbers (F6) and, since Wave 4 (Agent W4g), code blocks highlighted with syntect and bat's syntaxes, in colors from the theme and never color alone.
  - auto-play when a document opens. Status: done (`[speech] auto_play`).
  - tapping Ctrl alone to pause. Status: missing.

## See also

- [Roadmap](roadmap.md): what comes next, in order.
- [Star parity reference](history/star-parity.md): what Star does, in detail.
- [Features page](site/features.html): what textweaver does today, with its status.
- [Implementation plan](history/plan.md): the original scope.
- [Documentation index](README.md)
