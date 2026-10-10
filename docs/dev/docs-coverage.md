# Documentation coverage

This page lists each feature, the user guide that covers it, and whether the guide passes. It was written in a documentation sweep on Sunday, October 4, 2026, and extended on Saturday, October 10, 2026, from the changelog, the keymap, the settings reference, `tw --help`, and the window's dialogs. "Pass" means the guide explains the feature and matches the code. "Pass, after fix" means the sweep added or corrected the text. "Fix" means a gap is still open.

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

## The app

- Layout, narrow layout, Header and Toolbar: gui.md. Pass.
- Reading settings dialog: gui.md. Pass.
- Colors dialog and high contrast: gui.md. Pass.
- First run and Ask again: gui.md. Pass.
- Size and place kept, text scale, caret blink, low-power graphics, log, save after failure: gui.md. Pass, after fix.
- Graphics choice (`--graphics`): gui.md and troubleshooting.md. Pass.
- Announcement levels: gui.md and screen-readers.md. Pass.

## The command line

- Rules for every command: command-line.md. Pass.
- Each `tw` command (open, text, info, search, speak, voices, backends, eloquence, convert, export-audio, library, vault, dictate, marks, lint, migrate-star, cite, settings, define, stats, summarize, sync, serve, ocr, components, changes, notes links, study, update): the guide named for it, and command-line.md. Pass.
- JSON-RPC, including `open`, `outline`, `notes`, `highlights`, and `info`: json-rpc.md. Pass.

## Framing

- docs/README.md opening: describes textweaver as its own project with roots in star. Pass, after fix.
- star-gaps.md opening: no longer calls textweaver a port. Pass, after fix.
- docs/site/index.html and docs/site/README.md: Pass, after fix. The start page named two programs and now names three.
- Other guides mention star only for importing and for the history of a feature. Pass.

## Beta 1, audited on Saturday, October 10, 2026

Every guide was read again for beta 1, one by one, against the code and the changelog. The features new since alpha.9:

- One download per system, `tw` as the one terminal program, `textweaver` as its second name: install.md, command-line.md, reading.md, quickstart.md. Pass, after fix.
- Update checks, Help, Check for updates, `tw update`: updates.md, install.md, privacy.md, troubleshooting.md. Pass, after fix.
- Components fetched for you (ffmpeg, liblouis, Pandoc), components from your own source, the token in the credential store: components.md, audio-export.md, reading.md, privacy.md. Pass, after fix.
- The eSpeak NG helper on Windows, speech that never stays silent: speech.md, dev/espeak-helper.md. Pass.
- Carta formats, DAISY 2.02 and recorded narration, braille (BRF) reading, polished HTML: converting.md, reading.md. Pass.
- Tracked changes and write-back, `tw changes`: reading.md, editing.md. Pass.
- Study cards with SM-2, the self-test, recall prompts, `tw study due`: notes.md, reading.md, settings.md. Pass, after fix.
- Links between notes, graph exports, `tw notes links`: notes.md, gui.md. Pass.
- The highlight palette, Alt+1 to Alt+5: notes.md, settings.md, gui.md. Pass, after fix.
- Regular-expression find and replace and its panel: editing.md, gui.md. Pass.
- Markdown paste (Windows app, terminal reader) and the plain-text limit on macOS and Linux: editing.md, known-limits.md. Pass, after fix.
- The context menu, the preview pane, Browser preview follows: gui.md, editing.md, settings.md. Pass, after fix.
- Customizable buttons, offline help, Search help, the menu bar hidden by default on Windows: gui.md, reading.md. Pass.
- The developer profile and `[export] audio_format`, code blocks read by default, Export asks where to save: settings.md, speech.md, audio-export.md. Pass, after fix.
- Version wording (beta), the app named "the app", Windows ARM64 not planned, Anki and the reading queue in beta 2: every guide, known-limits.md, roadmap.md, whats-new.md. Pass, after fix.

## Still open

- The 293 keyboard actions were checked key by key against the guides by the owner, Sunday, October 4, 2026. The generated keyboard page is the key list. Pass.
- German, French, Portuguese, and Arabic translations are not yet checked by native speakers: see known-limits.md. Fix.
