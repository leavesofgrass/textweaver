# Star features not yet planned

Star (`D:\star`) is the Python program textweaver reimplements in Rust. This page tracks Star's features against textweaver's current status, for anyone comparing the two or looking for what is left to port. Priorities favor students with print disabilities, Star's original audience.

Each item has a status:

- **done**: textweaver does it.
- **partly**: some of it is done; the note says what is missing.
- **missing**: not done yet. See the [roadmap](roadmap.md) for what is planned next.
- **dropped**: out of scope on purpose.

This page is kept current as features land; see [CHANGELOG.md](../CHANGELOG.md) for when each one shipped.

## Reading aids

Star's reading aids are described in its "Accessibility and WCAG reading aids" notes; [reading-aids.md](reading-aids.md) is textweaver's user guide to the same features.

- **RSVP.** Shows one word at a time, at nine screen positions, with the words before and after. Status: done.
- **Reading ruler.** A band that follows the caret line. Status: done.
- **Text spacing** (WCAG 1.4.12): line height and letter and word spacing. Status: done. The terminal reader approximates it with blank lines and spaces.
- **Bionic reading:** word starts in bold. Status: done.
- **Syllable splitting:** `read·a·bil·i·ty`, shown only, by rule rather than dictionary. Status: done, in the terminal reader (Alt+Shift+Z) and drawn in the GUI.
- **Current-line highlight.** Status: done.
- **Reading fonts:** OpenDyslexic, Atkinson Hyperlegible, and Lexend. Status: done. OpenDyslexic and Atkinson Hyperlegible Next and Mono are bundled, used for PDF and EPUB output, and loaded directly for the GUI, where a font-choice key cycles them. Lexend is downloaded the first time it is chosen, as in Star, but only after asking with its size and license, and each file is checked by its SHA-256 before it is kept in the data folder; the GUI, PDF, and EPUB then use it (see [Reading aids](reading-aids.md#lexend-on-first-choice)).

## Formats and documents

- **DAISY 3 / DTBook** (the Bookshare format), including DAISY zips. Status: done, in spine order with NCX navigation.
- **OCR** of images and scanned PDF pages. Status: done: a pure-Rust engine runs in process by default (models are downloaded once with `tw ocr download`, checked by SHA-256), with a Tesseract subprocess as a fallback for other languages ([ADR-0026](adr/0026-ocr-and-student-formats.md)). Scans made sideways or upside down are turned upright before they are read, and scanned tables whose rows and columns line up are read as tables ([ADR-0048](adr/0048-pdf-annotations-links-and-forms.md)).
- **Better PDF reading order:** removing running headers, footers, and page numbers, and marking captions. Status: done. Captions are marked by pattern ("Figure 3.", "Table 2:"), in English, Spanish, French, German, Portuguese, and Arabic ([ADR-0010](adr/0010-pdf-loader.md), [ADR-0048](adr/0048-pdf-annotations-links-and-forms.md)).
- **Archives:** ZIP and TAR, with 7z. Opening one lists its readable files, and `book.zip!inner.pdf` opens a file inside. Status: done. RAR is not planned.
- **Opening a web page by URL.** Status: done.
- **PPTX:** slide titles become headings, speaker notes follow each slide, and images get descriptions. Status: done.
- **Offline dictionary:** WordNet definitions, synonyms, and pronunciation, plus a custom glossary. Status: done: your glossary, then Open English WordNet through morphy, with CMUdict pronunciations respelled for reading aloud, all offline (Ctrl+Shift+D or Alt+E, and `tw define`; see [reading.md](reading.md#define-a-word-ctrlshiftd-or-alte)).
- **Spell check in edit mode.** Status: done, on the built-in SCOWL list with a personal word list: next and previous misspelling, spelled aloud, suggestions, and a count on save. See [editing.md](editing.md#spelling).
- **Accessible publishing templates:**
  - Large Print, Dyslexia-friendly, High Contrast, and academic-manuscript stylesheets;
  - Word templates for APA student papers and AMA manuscripts;
  - an EPUB cover and a table-of-contents depth setting;
  - single-file HTML.
  - Status: mostly done. `tw convert` writes single-file accessible HTML (following the system's dark, light, and high-contrast settings), large-print PDF, PDF in OpenDyslexic, and a PDF table of contents with a depth option, plus APA, AMA, large-print, dyslexia-friendly, high-contrast, and manuscript templates for EPUB, Word, and PDF, with an EPUB cover. The reader exports the open document to HTML, PDF, Word, EPUB, and braille from the command palette, previews it in the browser, and starts a new document from a template with front matter and a References heading.
- **GUI Contents and Notes panels.** Status: done. The window shows the headings (Ctrl+1) or the notes (Ctrl+2) in a panel beside the document, from the same lists as the outline (Alt+O) and the notes list; Enter goes to a row, F6 moves between the panel and the document, and the panel never takes the focus unasked (see [The Contents and Notes panels](gui.md#the-contents-and-notes-panels)). The terminal reader lists notes (Shift+A) and the headings with type-to-filter (the outline, Alt+O).
- **Math as Unicode in the plain reading view** (`x²`, `√2`). Status: done (`[reading] math_display = "unicode"`); the LaTeX source is also available, spoken as English, and HTML output carries MathML.

## Medium priority

- **Formats:** spreadsheets (XLSX, CSV, and TSV, as tables) are done. Jupyter notebooks open natively, cell by cell ([ADR-0044](adr/0044-obsidian-json-svg-and-content-mathml.md)). Source code as a structured document is not planned; it still opens as plain text.
- **Summarize:** extractive, with LexRank, no downloaded model. Status: done (`tw summarize`, and Summarize in the command palette; [ADR-0037](adr/0037-extractive-summaries.md)).
- **Translate a document.** Status: missing.
- **Difficult-word overlay,** by word frequency. Status: done, in the terminal reader (Alt+Shift+J) and drawn in the GUI, on SCOWL's word levels.
- **Reading statistics:** time read, progress, and sessions. Status: done: time read aloud, the furthest point, and sessions per document, with a most-read list (Ctrl+Shift+Y or Alt+Y), `tw stats`, an opt-out, and Star's statistics imported ([reading.md](reading.md#reading-statistics-ctrlshifty-or-alty)).
- **Settings profiles:** named sets of voice, theme, and spacing settings, with import and export. Status: done: named profiles of voice, rate, theme, font, spacing, highlight, and access mode, switched, saved, renamed, deleted, imported, and exported in the reader (Ctrl+Shift+U or Alt+U) and with `tw settings profile` ([settings.md](settings.md#settings-profiles)). `tw migrate-star` imports Star's profiles ([library.md](library.md#import-from-star-tw-migrate-star)).
- **Piper neural voices,** with a catalog of voices to download, and a voice manager that lists, previews, and marks favorites. Status: done for voices: Piper voices run in process on a pure-Rust ONNX runtime, with word timing read from the model ([ADR-0023](adr/0023-in-process-neural-speech.md)); Choose voice (Alt+V) lists them, speaks a sample, and marks favorites, which are listed first. The voice manager covers every engine in both readers, and previews a voice without choosing it (the Say Status key, or the GUI's Preview button); in the GUI it is a dialog with filter and action buttons ([The window](gui.md#voices)).
- **Interface translations:** Spanish, French, German, Portuguese, and Arabic, with right-to-left layout. Status: done ([ADR-0030](adr/0030-interface-translations.md)): `[interface] language` speaks and shows textweaver's own words, lists, help, keyboard shortcuts, the command palette, and the settings screen in all six languages, the voice follows the language when the engine has one, and right-to-left text is reordered for display where the terminal does not do it itself.
- **Clipboard copy,** including the terminal escape code that works over SSH. Status: done. Ctrl+C copies the selection, or the sentence at the cursor, through the terminal's OSC 52 code, which works over SSH; edit mode also has cut, paste, and select all. A native clipboard fallback (`arboard`) covers terminals without OSC 52 (the old Windows console, macOS Terminal, GNOME Terminal and other VTE terminals), announced the first time it is used; SSH and tmux still use the terminal's own code.
- **Dependency report** (`--deps`) and a crash log. Status: done. `scripts/doctor.sh` and `scripts\doctor.ps1` print a system report, and textweaver writes a rotating log file ([troubleshooting.md](troubleshooting.md)).
- **Document metadata:** edit it per document, and search the library by it. Status: done. The library list's filter and `tw library --search` match title, path, author, DOI, ISBN, and text, from the document's own metadata, its text, and `tw cite`'s record ([library.md](library.md#search-by-author-doi-and-isbn)). Edit details (the File menu, or F2 in the library list) and `tw library edit` set a document's title, author, DOI, and ISBN by hand, and what you type wins over the document's own ([library.md](library.md#edit-a-documents-details)).
- **Notes export** as BibTeX, RIS, JSON, or plain text. Status: done. `tw marks FILE --export FORMAT` writes BibTeX, BibLaTeX, RIS, or CSL-JSON records for Zotero or Pandoc. Notes and highlights also export to an Obsidian vault as Markdown ([vault.md](vault.md)), and as a Markdown study sheet grouped by the document's headings ([notes.md](notes.md#export-a-study-sheet)). Plain-text export is not a separate format; CSL-JSON or the study sheet cover that need.
- **Study tools,** out of scope on purpose:
  - the spaced-repetition scheduler (FSRS) and review screen;
  - AnkiConnect sync and `.apkg` export.
  - Status: dropped.

## Peripheral

- **Knowledge graph and concept extraction.** Typed links between notes; exports to SVG, DOT, PlantUML, and JSON. Status: partly. Vault import and export keep typed links between notes; the graph exports and concept extraction are missing.
- **Karaoke video export.** Status: missing.
- **Feeds, Wikipedia, and PubMed** quick open. Status: missing.
- **More engines:** Coqui, Festival, Qt speech, and cloud voices. Status: missing. speech-dispatcher, which can drive Festival, and DECtalk were added instead.
- **SSML pauses.** Status: missing.
- **Audio export to OGG and AAC,** and an M4B cover image. Status: partly. M4B audiobooks are AAC, and Ogg Opus files are written in process ([audio-export.md](audio-export.md)); Ogg Vorbis and the cover image are missing.
- **Infrastructure:**
  - plugins. Status: missing.
  - an update checker. Status: partly. `scripts/update.sh` and `scripts\update.ps1` update an installed textweaver; there is no check inside the program.
  - a guided tour and a welcome page. Status: partly. The [quick start](quickstart.md) opens as a document; there is no tour.
  - a key-code inspector. Status: missing.
  - line numbers and syntax highlighting in the terminal. Status: done. Line numbers (F6), and code blocks highlighted with syntect and bat's syntaxes, in colors from the theme and never color alone.
  - auto-play when a document opens. Status: done (`[speech] auto_play`).
  - tapping Ctrl alone to pause. Status: missing.

## See also

- [Roadmap](roadmap.md): what comes next, in order.
- [Features page](site/features.html): what textweaver does today, with its status.
- [Documentation index](README.md)
