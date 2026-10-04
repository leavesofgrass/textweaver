# Documentation coverage

This page lists each feature, the user guide that covers it, and whether the guide passes. It was written in a documentation sweep on Sunday, October 4, 2026, from the changelog, the keymap, the settings reference, `tw --help`, and the window's dialogs. "Pass" means the guide explains the feature and matches the code. "Pass, after fix" means the sweep added or corrected the text. "Fix" means a gap is still open.

The generated pages (`keyboard.md` and `settings-reference.md`) list every key and setting, so this page tracks the guides that explain them.

## Reading and speech

- Written pauses, `markup_pauses`: speech.md. Pass.
- Reading passes, Shift+F: reading.md. Pass.
- Document overview: reading.md. Pass.
- Where am I, with time left: reading.md. Pass, after fix.
- Pictures with no description: reading.md. Pass.
- Print page numbers: reading.md. Pass.
- Quick navigation keys, NVDA and JAWS style: reading.md and keyboard.md. Pass.
- Speech Cursor: reading.md and gui.md. Pass.
- Summaries: reading.md. Pass.
- Reading statistics, including clearing them in the reader: reading.md. Pass.
- Speech engines and voices: speech.md, eloquence.md, dectalk.md. Pass.
- Accessibility modes and quiet screen: screen-readers.md. Pass.

## Reading aids, notes, and math

- RSVP, bionic reading, ruler, spacing, fonts, syllables, difficult words: reading-aids.md. Pass.
- Bookmarks, notes, highlights, study sheet: notes.md. Pass.
- Math: math.md. Pass.
- Themes, Lamplight, contrast in words: themes.md. Pass.

## Writing

- Edit mode, templates, listen rendered, recovery copies: editing.md. Pass.
- Citations and the reference library: citations.md. Pass.
- Dictation and captions from dictation: dictation.md. Pass.

## Export and conversion

- Audio formats, Ogg Vorbis, M4B: audio-export.md. Pass.
- Read-along page: audio-export.md. Pass.
- Karaoke video: audio-export.md. Pass.
- Captions, karaoke tags, chapters: audio-export.md. Pass.
- Converting, HTML theme question, batch, watching a folder: converting.md. Pass.
- Preview in a browser and live preview: editing.md and converting.md. Pass.

## Library, sync, and settings

- Library folders, Add a folder, Continue reading, document details: library.md. Pass.
- Sync: sync.md. Pass.
- Obsidian vault: vault.md. Pass.
- Settings, profiles, export, import, reset: settings.md. Pass.
- Optional components: components.md. Pass.
- Importing from star: library.md. Pass.

## The window

- Layout, narrow layout, Header and Toolbar: gui.md. Pass.
- Reading settings dialog: gui.md. Pass.
- Colors dialog and high contrast: gui.md. Pass.
- First run and Ask again: gui.md. Pass.
- Size and place kept, text scale, caret blink, low-power graphics, log, save after failure: gui.md. Pass, after fix.
- Graphics choice (`--graphics`): gui.md and troubleshooting.md. Pass.
- Announcement levels: gui.md and screen-readers.md. Pass.

## The command line

- Rules for every command: command-line.md. Pass.
- Each `tw` command (open, text, info, search, speak, voices, backends, eloquence, convert, export-audio, library, vault, dictate, marks, lint, migrate-star, cite, settings, define, stats, summarize, sync, serve, ocr, components): the guide named for it, and command-line.md. Pass.
- JSON-RPC, including `open`, `outline`, `notes`, `highlights`, and `info`: json-rpc.md. Pass.

## Framing

- docs/README.md opening: describes textweaver as its own project with roots in star. Pass, after fix.
- star-gaps.md opening: no longer calls textweaver a port. Pass, after fix.
- docs/site/index.html and docs/site/README.md: Pass, after fix. The start page named two programs and now names three.
- Other guides mention star only for importing and for the history of a feature. Pass.

## Still open

- The 293 keyboard actions were checked key by key against the guides by the owner, Sunday, October 4, 2026. The generated keyboard page is the key list. Pass.
- German, French, Portuguese, and Arabic translations are not yet checked by native speakers: see known-limits.md. Fix.
