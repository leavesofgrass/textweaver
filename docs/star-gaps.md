# Star features not yet planned

This inventory was made on 2026-09-25. It lists Star features (D:\star) that `docs/plan.md` and `docs/tasks.md` did not cover then. For each one it gives the Star code location, and the test file and count where Star has tests. Priorities are for students with print disabilities.

Some of the areas were already covered at the time: themes (Agent Q); math (O); citations (P); batch conversion and folder watching (A2, L); braille output (M); and large-document paging, which plan §6.1 handles by design.

## Core: plan these next

- **Reading aids.** Star's wiki page "Accessibility and WCAG reading aids" describes them.
  - **RSVP.** Shows one word at a time, at 9 screen positions, with the words before and after. Code: `gui/main_window.py:62-262`, `gui/mixin_aiddialogs.py`, `tui/mixin_rsvp.py`. Tests: `test_rsvp.py` (19).
  - **Reading ruler.** A band that follows the caret line. Code: `gui/main_window.py:263-349`.
  - **Text spacing** (WCAG 1.4.12): line height and letter and word spacing. Code: `gui/mixin_aiddialogs.py:256-333`.
  - **Bionic reading:** word starts in bold. Code: `gui/mixin_fontspacing.py:265-300`.
  - **Syllable splitting:** `read·a·bil·i·ty`, shown only. Code: `syllables.py` (uses pyphen). Tests: `test_syllables.py` (11).
  - **Current-line highlight.** Code: `gui/mixin_commands.py:353-362`.
  - **Reading fonts:** OpenDyslexic, Atkinson Hyperlegible, and Lexend, downloaded when first chosen. Code: `fonts.py`. Tests: `test_fonts.py` (12).
- **DAISY 3 / DTBook** (Bookshare), including DAISY zips. Code: `documents/ebook.py:122-232`.
- **OCR** of images and scanned PDF pages, with Tesseract. It runs only on pages with no text layer. Code: `documents/misc.py:7-23`, `documents/pdf.py:183-195`.
- **Better PDF reading order:** remove running headers, footers, and page numbers, and mark captions. Code: `documents/pdf.py:7-161`. Tests: `test_pdf_layout.py` (9).
- **Archives:** ZIP and TAR, with 7z and RAR optional. Opening one lists its readable files, and `book.zip!inner.pdf` opens a file inside. Code: `archive.py`. Tests: `test_archive.py` (16).
- **Opening a web page by URL.** Code: `documents/misc.py:24-56`.
- **PPTX:** slide titles become headings, speaker notes follow each slide, and images get descriptions. Code: `documents/office.py:266-367`.
- **Offline dictionary:** WordNet definitions, synonyms, and pronunciation, plus a custom glossary. Code: `dictionary.py`. Tests: `test_dictionary.py` (17).
- **Spell check in edit mode.** Code: `spellcheck.py`.
- **Accessible publishing templates:**
  - Large Print, Dyslexia-friendly, High Contrast, and academic-manuscript stylesheets;
  - Word templates for APA student papers and AMA manuscripts;
  - an EPUB cover and a table-of-contents depth setting;
  - single-file HTML.
  - Code: `publish.py`, `publish_styles/`. Tests: `test_publish.py` (28).
- **GUI Contents and Notes panels.** Code: `gui/mixin_toc.py`, `gui/mixin_annotations.py:51-110`.
- **Math as Unicode in the plain reading view** (`x²`, `√2`). Code: `mathrender.py`.

## Medium

- **Formats:** spreadsheets (XLSX, CSV, and TSV, as tables); notebooks and source code; Unicode math in the text view.
- **Summarize:** extractive, with LexRank. Code: `summarize.py`.
- **Translate** a document. Code: `translate.py`. Tests: 11.
- **Difficult-word overlay,** by word frequency. Code: `vocab.py`.
- **Reading statistics:** time read, progress, and sessions. Code: `stats.py`.
- **Settings profiles:** named sets of voice, theme, and spacing settings, with import and export. Code: `gui/mixin_presets.py`.
- **Piper neural voices,** with a catalog of voices to download, and a **Voice Manager** that lists, previews, and marks favorites. Code: `tts/piper*.py`, `gui/mixin_voices.py`.
- **Interface translations:** Spanish, French, German, Portuguese, and Arabic, with right-to-left layout. Code: `i18n.py`, `locale/`. Tests: 44.
- **Clipboard copy,** including the terminal escape code that works over SSH. Code: `tui/mixin_docops.py:18-145`. Tests: 13.
- **Dependency report** (`--deps`) and a **crash log.** Code: `diagnostics.py`, `gui/runner.py:55-85`.
- **Document metadata:** edit it per document, and search the library by it. Code: `discovery.py`. Tests: 24.
- **Notes export** as BibTeX, RIS, JSON, or plain text.
- **Study tools,** out of scope under plan §2:
  - the spaced-repetition scheduler (FSRS) and review screen (`sr.py`, `gui/mixin_review.py`);
  - AnkiConnect sync and `.apkg` export.

## Peripheral

- **Knowledge graph and concept extraction.** Typed links between notes; exports to SVG, DOT, PlantUML, and JSON. Code: `graph.py`, `ner.py`.
- **Karaoke video export.** Code: `video.py`. Tests: 18.
- **Feeds, Wikipedia, and PubMed** quick open.
- **More engines:** Coqui, Festival, Qt speech, and ElevenLabs cloud voices.
- **SSML pauses.**
- **Audio export to OGG and AAC,** and an M4B cover image.
- **Infrastructure:**
  - plugins;
  - an update checker;
  - a guided tour and a welcome page;
  - a key-code inspector;
  - line numbers and syntax highlighting in the terminal;
  - auto-play when a document opens;
  - tapping Ctrl alone to pause.
