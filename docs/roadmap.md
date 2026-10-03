# Roadmap

textweaver aims for terminal-first Markdown reading and authoring, with speech and word highlighting that never stall, drift, or lose your place. It should be fast on large files, work the same on Windows, macOS, and Linux, and offer a GUI for readers who want one, without giving up any of that.

This page is a plain-language summary of where the project stands. For the full history of decisions, see the [ADRs](adr/README.md); for what changed release by release, see [CHANGELOG.md](../CHANGELOG.md).

## What works today

- **Reading and speech.** Reading pauses briefly after headings, paragraphs, and list items, and says medical and scientific text (identifiers, units, error-prone abbreviations) the way a clinician would. The terminal reader (`textweaver`) and command-line tool (`tw`) read large files fast, with word highlighting that follows the audio exactly, never guessing. An all-Rust GUI, built on Xilem, runs on Windows, macOS, and Linux, with native menus and a command palette shared with the terminal reader. Speech engines: ETI-Eloquence, Windows SAPI5 voices, Apple's voices, espeak-ng, speech-dispatcher, DECtalk, and Piper neural voices with model-accurate word timing.
- **The interface.** Menus (F10) and a command palette (F2) built from one shared list of commands, in six languages. A file browser walks folders and archives as one list, for opening a document or choosing a folder for another command. Batch conversion runs a whole folder to a chosen format in the background.
- **Formats.** Text, Markdown, HTML, PDF, EPUB, DOCX, RTF, ODT (including tracked changes and comments), LaTeX, DAISY 3, PowerPoint, spreadsheets, archives, email, saved web pages, Obsidian notes, JSON, SVG drawings, content MathML, and Jupyter notebooks all open natively. OCR reads scanned PDF pages and images, in process by default, with no network access needed for the base features. PDF comments become notes, and PDF links, filled-in forms, and scanned tables are read.
- **Writing.** Edit mode with structure-aware navigation while typing, spell check, an outline, clipboard support, find and replace, templates, citations (DOI and ISBN lookup, CSL formatting, insert-as-you-write), and dictation that types what you say, phrase by phrase, in the reader and the GUI's edit mode.
- **Output.** `tw convert` and in-reader export write HTML, EPUB, DOCX, braille (BRF), and tagged PDF, including publishing templates (APA and AMA manuscripts, large print, dyslexia-friendly, high contrast) and audio export with subtitles and chapters. FLAC, WAV, MP3, and Opus audio export need nothing else installed; only M4B still needs ffmpeg.
- **Math.** LaTeX and ASCIIMath are parsed, spoken, shown as MathML, and available in Nemeth and UEB braille.
- **Reading aids.** RSVP, bionic reading, a reading ruler, text spacing, dyslexia-friendly fonts, difficult-word marking, reading level, and syllable display, all drawn in the terminal reader and the GUI.
- **Accessibility.** Three accessibility modes (self-voicing, hybrid, and screen-reader), tested with NVDA, JAWS, VoiceOver, and Orca, with automated accessibility checks running in CI alongside human listening sessions before every release. Six interface languages, including right-to-left layout. Braille output carries typeform and capitals indicators, and tables in three layouts.
- **Study tools.** Offline dictionary lookups, reading statistics, settings profiles, extractive summaries with no downloaded model, notes and highlights with Obsidian vault export and import, and a document's title, author, DOI, and ISBN editable by hand.
- **Sync between computers.** Notes, highlights, bookmarks, reading places, statistics, settings, and word lists stay in step through a folder you choose, such as Syncthing or a USB stick, with no account and no server ([Syncing between computers](sync.md)).

See the [features page](site/features.html) for the full, current list with each item's status, and [Star features not yet planned](star-gaps.md) for a detailed comparison with textweaver's predecessor.

## Being built next

- **Release readiness** for the next alpha: packaging, dependency, and CI polish so each release ships cleanly on every platform.
- **The GUI catching up to the terminal reader.** The voice manager covers every speech engine, with preview; the syllable display, the difficult-word overlay, and the Contents and Notes panels are in the GUI too. What is left is polish.
- **Stabilization:** fixes from hands-on test sessions, a long soak test, a week of clean nightly runs, and a native speaker's review of the translations.

## Planned

- Document translation.
- Karaoke-style video export.
- Quick-open for feeds, Wikipedia, and PubMed.
- More speech engines (Coqui, Festival, Qt Speech).
- Pauses written as markup in the text (SSML style). Pauses after headings, paragraphs, and list items already work.
- Ogg Vorbis and AAC audio export, and a cover image for audiobooks.
- A plugin system.
- An in-app update checker, and a guided first-run tour.
- Source code read as a structured document, rather than plain text.
- Knowledge-graph export and concept extraction from notes.

Deliberately out of scope for now: spaced-repetition study tools (Anki-style review) and cloud speech engines.

## See also

- [Star features not yet planned](star-gaps.md)
- [Releasing](dev/releasing.md)
- [Architecture](dev/architecture.md)
- [Documentation index](README.md)
