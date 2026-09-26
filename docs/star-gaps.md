# Star features not yet planned

This inventory was made on Friday, September 25, 2026. It lists Star features (D:\star) that `docs/plan.md` and `docs/tasks.md` did not cover then. For each one it gives the Star code location, and the test file and count where Star has tests. Priorities are for students with print disabilities.

Each item now has a **status**, checked against `main` on Saturday, September 26, 2026:

- **done**: textweaver does it.
- **partly**: some of it is done; the note says what is missing.
- **missing**: not done yet. The [roadmap](roadmap.md) says which of these come next.
- **dropped**: out of scope on purpose.

Updated after Phase 2 on Saturday, September 26, 2026. Items that Wave 3 takes on say which agent; the briefs are in [tasks.md](tasks.md). Wave 3 prefers pure-Rust, in-process solutions, so several items below name the Rust crate planned in place of Star's Python or C dependency.

## Covered by Wave 2

These areas were being built when the list was made. All are now on `main`:

- Themes (Agent Q). Status: done. See [themes.md](themes.md).
- Math (Agent O). Status: done. Math is spoken with exact highlighting and becomes MathML in HTML. See [math.md](math.md).
- Citations (Agent P). Status: done in the reader since Phase 2 (Agent P2b): Alt+C inserts from a filtered picker with a locator, Alt+Shift+D adds by DOI or ISBN, and the palette inserts a bibliography and checks citations. Continuous reading still reads a citation as written. See [citations.md](citations.md).
- Batch conversion and folder watching (Agents A2 and L). Status: done. See [converting.md](converting.md).
- Braille output (Agent M). Status: done for grade 1; grade 2 needs the `liblouis` feature and liblouis installed. See [converting.md](converting.md).
- Large-document paging, which plan §6.1 handles by design. Status: done for the terminal reader, which plans reading in windows. The GUI's window model is planned for Wave 3 (Agent W3a), for the Xilem GUI (Agent W3b).

## Core: plan these next

- **Reading aids.** Star's wiki page "Accessibility and WCAG reading aids" describes them. [reading-aids.md](reading-aids.md) is the user guide.
  - **RSVP.** Shows one word at a time, at 9 screen positions, with the words before and after. Code: `gui/main_window.py:62-262`, `gui/mixin_aiddialogs.py`, `tui/mixin_rsvp.py`. Tests: `test_rsvp.py` (19). Status: done.
  - **Reading ruler.** A band that follows the caret line. Code: `gui/main_window.py:263-349`. Status: done.
  - **Text spacing** (WCAG 1.4.12): line height and letter and word spacing. Code: `gui/mixin_aiddialogs.py:256-333`. Status: done. The terminal approximates it with blank lines and spaces.
  - **Bionic reading:** word starts in bold. Code: `gui/mixin_fontspacing.py:265-300`. Status: done.
  - **Syllable splitting:** `read·a·bil·i·ty`, shown only. Code: `syllables.py` (uses pyphen). Tests: `test_syllables.py` (11). Status: partly. The `textweaver-aids` library does it by rules, with no dictionary; the reader does not show it yet.
  - **Current-line highlight.** Code: `gui/mixin_commands.py:353-362`. Status: done.
  - **Reading fonts:** OpenDyslexic, Atkinson Hyperlegible, and Lexend, downloaded when first chosen. Code: `fonts.py`. Tests: `test_fonts.py` (12). Status: partly. OpenDyslexic and Atkinson Hyperlegible Next and Mono are bundled and used for PDF and EPUB output; choosing a font is in the wxDragon GUI spike only, and Lexend must be installed by hand (the download is designed in the library but not offered yet). The font chooser moves to the Xilem GUI in Wave 3 (Agent W3b), with the bundled fonts loaded straight into Parley.
- **DAISY 3 / DTBook** (Bookshare), including DAISY zips. Code: `documents/ebook.py:122-232`. Status: missing; planned for Wave 3 (Agent W3d), in spine order with NCX navigation.
- **OCR** of images and scanned PDF pages, with Tesseract. It runs only on pages with no text layer. Code: `documents/misc.py:7-23`, `documents/pdf.py:183-195`. Status: missing; planned for Wave 3 (Agent W3d): the pure-Rust `ocrs` engine in process first, with a Tesseract subprocess as a fallback for other languages. A scanned PDF loads as one sentence saying it has no text.
- **Better PDF reading order:** remove running headers, footers, and page numbers, and mark captions. Code: `documents/pdf.py:7-161`. Tests: `test_pdf_layout.py` (9). Status: done, except that captions are not marked by pattern ([ADR-0010](adr/0010-pdf-loader.md)).
- **Archives:** ZIP and TAR, with 7z and RAR optional. Opening one lists its readable files, and `book.zip!inner.pdf` opens a file inside. Code: `archive.py`. Tests: `test_archive.py` (16). Status: missing; planned for Wave 3 (Agent W3d): ZIP and TAR, with 7z optional.
- **Opening a web page by URL.** Code: `documents/misc.py:24-56`. Status: missing; planned for Wave 3 (Agent W3d).
- **PPTX:** slide titles become headings, speaker notes follow each slide, and images get descriptions. Code: `documents/office.py:266-367`. Status: missing; planned for Wave 3 (Agent W3d).
- **Offline dictionary:** WordNet definitions, synonyms, and pronunciation, plus a custom glossary. Code: `dictionary.py`. Tests: `test_dictionary.py` (17). Status: missing; planned for Wave 3 (Agent W3e): your glossary, then WordNet, then CMUdict pronunciations, offline.
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

- **Formats:** spreadsheets (XLSX, CSV, and TSV, as tables); notebooks and source code; Unicode math in the text view. Status: missing. Spreadsheets as tables are planned for Wave 3 (Agent W3d). Source code opens as plain text.
- **Summarize:** extractive, with LexRank. Code: `summarize.py`. Status: missing.
- **Translate** a document. Code: `translate.py`. Tests: 11. Status: missing.
- **Difficult-word overlay,** by word frequency. Code: `vocab.py`. Status: partly. The `textweaver-aids` library marks rare words with SCOWL's word levels built in; the reader does not show the marks yet.
- **Reading statistics:** time read, progress, and sessions. Code: `stats.py`. Status: missing; planned for Wave 3 (Agent W3e), with `tw stats` and an opt-out.
- **Settings profiles:** named sets of voice, theme, and spacing settings, with import and export. Code: `gui/mixin_presets.py`. Status: partly. All settings can be exported and imported as JSON ([settings.md](settings.md)); named profiles are planned for Wave 3 (Agent W3e).
- **Piper neural voices,** with a catalog of voices to download, and a **Voice Manager** that lists, previews, and marks favorites. Code: `tts/piper*.py`, `gui/mixin_voices.py`. Status: partly. Choose voice (Alt+V) lists the engine's voices and speaks a sample, and Space marks favourites, which are listed first (Phase 1, Agent P1b). Piper voices, in process through `tract` or `candle`, and a voice manager across every engine are planned for Wave 3 (Agent W3f).
- **Interface translations:** Spanish, French, German, Portuguese, and Arabic, with right-to-left layout. Code: `i18n.py`, `locale/`. Tests: 44. Status: missing. Wave 3 (Agent W3e) builds the message catalogue with English, a pseudo-locale, and a right-to-left check; translations come later.
- **Clipboard copy,** including the terminal escape code that works over SSH. Code: `tui/mixin_docops.py:18-145`. Tests: 13. Status: done. Ctrl+C copies the selection, or the sentence at the cursor, through the terminal's OSC 52 code, which works over SSH (Phase 1, Agent P1b); edit mode also has cut, paste, and select all (Phase 2, Agent P2b). There is no native clipboard fallback, so terminals without OSC 52 (the old Windows console, macOS Terminal) cannot copy yet.
- **Dependency report** (`--deps`) and a **crash log.** Code: `diagnostics.py`, `gui/runner.py:55-85`. Status: done. `scripts/doctor.sh` and `scripts\doctor.ps1` print a system report, and textweaver writes a rotating log file ([troubleshooting.md](troubleshooting.md)).
- **Document metadata:** edit it per document, and search the library by it. Code: `discovery.py`. Tests: 24. Status: missing. Library search covers titles, paths, and text.
- **Notes export** as BibTeX, RIS, JSON, or plain text. Status: partly. Notes and highlights export to an Obsidian vault as Markdown ([vault.md](vault.md)), and since Phase 2 as a Markdown study sheet grouped by the document's headings ([notes.md](notes.md#export-a-study-sheet)); the other formats are missing.
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
  - line numbers and syntax highlighting in the terminal. Status: partly. Line numbers are done (F6); syntax highlighting is missing.
  - auto-play when a document opens. Status: done (`[speech] auto_play`).
  - tapping Ctrl alone to pause. Status: missing.

## See also

- [Roadmap](roadmap.md): what comes next, in order.
- [Star parity reference](star-parity.md): what Star does, in detail.
- [Features page](site/features.html): what textweaver does today, with its status.
- [Implementation plan](plan.md): the original scope.
- [Documentation index](README.md)
