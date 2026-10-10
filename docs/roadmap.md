# Roadmap

textweaver aims for terminal-first Markdown reading and authoring, with speech and word highlighting that never stall, drift, or lose your place. It should be fast on large files, work the same on Windows, macOS, and Linux, and offer a GUI for readers who want one, without giving up any of that.

This page summarizes where the project stands. For the full history of decisions, see the [ADRs](adr/README.md); for what changed release by release, see [CHANGELOG.md](../CHANGELOG.md).

## What works today

- **Reading and speech.** Reading pauses briefly after headings, paragraphs, and list items, and says medical and scientific text (identifiers, units, error-prone abbreviations) the way a clinician would. The terminal reader (`textweaver`) and command-line tool (`tw`) read large files fast, with word highlighting that follows the audio exactly, never guessing. An all-Rust GUI, built on Xilem, runs on Windows, macOS, and Linux, with native menus and a command palette shared with the terminal reader. Speech engines: ETI-Eloquence, Windows SAPI5 voices, Apple's voices, espeak-ng, speech-dispatcher, DECtalk, and Piper neural voices with model-accurate word timing.
- **The interface.** Menus (F10) and a command palette (F2) built from one shared list of commands, in six languages. A file browser walks folders and archives as one list, for opening a document or choosing a folder for another command. Batch conversion runs a whole folder to a chosen format in the background.
- **Formats.** Text, Markdown, HTML, PDF, EPUB, DOCX, RTF, ODT (including tracked changes and comments), LaTeX, DAISY 3 and 2.02, PowerPoint, spreadsheets, archives, email, saved web pages, Obsidian notes, JSON, SVG drawings, content MathML, and Jupyter notebooks all open natively. OCR reads scanned PDF pages and images, in process by default, with no network access needed for the base features. PDF comments become notes, and PDF links, filled-in forms, and scanned tables are read.
- **Writing.** Edit mode with structure-aware navigation while typing, spell check, an outline, clipboard support, find and replace, templates, citations (DOI and ISBN lookup, CSL formatting, insert-as-you-write), and dictation that types what you say, phrase by phrase, in the reader and the GUI's edit mode.
- **Output.** `tw convert` and in-reader export write HTML, EPUB, DOCX, braille (BRF), and tagged PDF, including publishing templates (APA and AMA manuscripts, large print, dyslexia-friendly, high contrast) and audio export with subtitles and chapters. FLAC, WAV, MP3, and Opus audio export need nothing else installed; only M4B still needs ffmpeg.
- **Math.** LaTeX and ASCIIMath are parsed, spoken, shown as MathML, and available in Nemeth and UEB braille.
- **Reading aids.** RSVP, bionic reading, a reading ruler, text spacing, dyslexia-friendly fonts, difficult-word marking, reading level, and syllable display, all drawn in the terminal reader and the GUI.
- **Accessibility.** Three accessibility modes (self-voicing, hybrid, and screen-reader), listened to with NVDA and JAWS and a Braille display on Windows, with automated accessibility checks on Windows and Linux on every release. VoiceOver on macOS and Orca on Linux have had some basic testing, and more extensive testing is planned. See the [accessibility statement](accessibility.md). Six interface languages, including right-to-left layout. Braille output carries typeform and capitals indicators, and tables in three layouts.
- **Study tools.** Offline dictionary lookups, reading statistics, settings profiles, extractive summaries with no downloaded model, notes and highlights with Obsidian vault export and import, and a document's title, author, DOI, and ISBN editable by hand.
- **Sync between computers.** Notes, highlights, bookmarks, reading places, statistics, settings, and word lists stay in step through a folder you choose, such as Syncthing or a USB stick, with no account and no server ([Syncing between computers](sync.md)).

See the [features page](site/features.html) for the full, current list with each item's status, and [star features not yet planned](star-gaps.md) for a detailed comparison with textweaver's predecessor.

## What beta 1 adds

Beta 1 follows the last alpha with the features below. Each is described in its own guide.

- **Study.** Study cards made from notes, highlights, and headings, graded in words and scheduled with SM-2, synced with your notes, with `tw study due`; a self-test from the study sheet that can be answered aloud; recall prompts at the end of a section ([Study with textweaver](notes.md#study-with-textweaver)).
- **Marks.** Named highlights (Alt+1 to Alt+5), each with its own color and shape in the terminal reader and its own typeform in braille files; links between notes, `tw notes links`, and an export of the knowledge graph to JSON, DOT, GraphML, Mermaid, PlantUML, CSV, or a Markdown list ([Bookmarks, notes, and highlights](notes.md)).
- **Review and editing.** Tracked changes and comments in a list, accepted or rejected one at a time or all at once, `tw changes`, and your own edits saved to a Word file as tracked changes; find and replace with regular expressions and a panel in the graphical version; paste as Markdown or as plain text, and a context menu; a preview of the reading view in the terminal reader ([Reading](reading.md#tracked-changes-and-comments-ctrlshiftj-or-alta), [Writing and editing](editing.md)).
- **Formats.** Org, reStructuredText, MediaWiki, DokuWiki, and Jira markup read and written through carta; braille files (BRF) that open as books; DAISY 2.02 books opened from their folder, with the narrator's recording played where the book has one; polished HTML pages that take their typography from your reading settings ([Converting documents](converting.md)).
- **Speech.** eSpeak NG runs in its own helper program on Windows, and a layer plays recorded audio in place of speech ([Speech engines and voices](speech.md)).
- **The graphical version.** Customizable header and toolbar buttons, offline guides under Help with one search over commands, keys, settings and guides, and a keyboard list and command palette with a filter, groups, and short rows ([The textweaver app](gui.md)).
- **Components.** A components folder searched first for helper programs, your own components source (including a private GitHub repository), and an unpack action for archives ([Optional components](components.md)).

## Moved to beta 2

These were planned for beta 1 and are now in beta 2:

- Windows on ARM64 packages.
- Anki import and export.
- The reading queue.
- The full rewrite of the guides. Beta 1 corrects wording and fills gaps; beta 2 rewrites the guides for a university-level reader throughout.

## Being built next

- **Stabilization:** fixes from hands-on test sessions, a long soak test, a week of clean nightly runs, and native-speaker review of the German, French, Portuguese, and Arabic translations (English and Spanish are checked).
- **The graphical version catching up to the terminal reader:** drawing each highlight name's own shape, and converting pasted formatted text to Markdown ([Known limits](known-limits.md#the-graphical-version)).

## Not in the 0.1 series

The 0.1 series ends with a feature-complete final alpha or beta. None of these is in it, and none is promised for later. The [known limits](known-limits.md) page lists what does not work yet.

- Document translation.
- Quick-open for feeds, Wikipedia, and PubMed.
- More speech engines (Coqui, Festival, Qt Speech).
- Pauses written as speech markup (SSML). Pauses after headings, paragraphs, and list items, and the break tag, already work.
- AAC audio export, and a cover image for audiobooks.
- A plugin system.
- An update check inside the program, and a guided first-run tour. The update scripts update an installed textweaver, and Help, Quick start opens the quick start.
- Source code read as a structured document, rather than plain text.
- Concept extraction from notes. The knowledge graph itself exports already ([Export the knowledge graph](notes.md#export-the-knowledge-graph)).

Dropped on purpose: AnkiConnect sync, FSRS scheduling, and cloud speech engines. [star features not yet planned](star-gaps.md) marks each dropped item.

## See also

- [Known limits](known-limits.md)
- [star features not yet planned](star-gaps.md)
- [Releasing](dev/releasing.md)
- [Architecture](dev/architecture.md)
- [Documentation index](README.md)
