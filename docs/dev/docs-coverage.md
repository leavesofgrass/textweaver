# Documentation coverage

This page lists each feature, the user guide that covers it, and whether the guide passes. It was written in a documentation sweep on Sunday, October 4, 2026, extended on Saturday, October 10, 2026, again for beta 4 on Friday, October 23, 2026, and for beta 5 on Thursday, October 29, 2026, from the changelog, the keymap, the settings reference, `tw --help`, and the window's dialogs. "Pass" means the guide explains the feature and matches the code. "Pass, after fix" means the sweep added or corrected the text. "Fix" means a gap is still open.

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
- Version wording (beta), the app named "the app", Windows ARM64 not planned, Anki and the reading queue in beta 3: every guide, known-limits.md, roadmap.md, whats-new.md. Pass, after fix.

## Beta 4, audited on Friday, October 23, 2026

Every guide was read again against the beta 4 entries of the changelog. The features new since beta 3:

- Links to other documents, with Back and Forward; "Paused." at every announcement level: reading.md, gui.md, quickstart.md, keyboard.md, start-students.md. Pass, after fix.
- Scrolling and line numbers in the app: gui.md, reading.md, settings.md. Pass.
- The Braille view and the braille grade setting: reading-aids.md, screen-readers.md, settings.md, converting.md, troubleshooting.md, known-limits.md, site/reading-aids.html. Pass, after fix.
- Obsidian highlights, comments, task statuses, inline footnotes, embeds, and Mermaid diagrams: converting.md, notes.md, settings.md. Pass.
- Obsidian vaults: links and embeds across the vault, block links, properties, backlinks, unresolved links, tags, `tw vault` questions, and converted pages: vault.md, reading.md, command-line.md. Pass, after fix.
- Obsidian canvases: vault.md, converting.md, reading.md. Pass.
- Insert wiki link, toggle and cycle task, insert callout: editing.md, gui.md. Pass.
- CW as a speech engine, its settings, and audio export in Morse code: cw.md, speech.md, audio-export.md, settings.md. Pass, after fix.
- CW keying, key adapters, and `tw cw monitor` and `tw cw ports`: cw.md, editing.md, gui.md, command-line.md, troubleshooting.md. Pass, after fix.
- Morse code practice (Koch lessons and drills), `cw_copy` in JSON-RPC: cw.md, json-rpc.md. Pass.
- The `textweaver-cw` crate, the vault resolver and index, canvases in formats, and the tree dump retries: dev/architecture.md, dev/testing.md, site/architecture.html. Pass, after fix.

## Beta 5, written on Monday, October 26, 2026 and extended for the release on Thursday, October 29, 2026

The release sweep read every guide against the beta 5 entries of the changelog. The features new since beta 4:

- Live documents: the log view, the values view, stale values, the document view, announcements, the JSON log, read only, and the terminal reader's and the app's buttons and status: live-documents.md, gui.md, reading.md, settings.md, accessibility.md, screen-readers.md, privacy.md, known-limits.md, roadmap.md, whats-new.md. Pass.
- Live sources: the connection messages, the one form, Edit, `.twlive` files, passwords, pinned keys and join codes, and the Tools, Live documents menu: live-documents.md, keyboard.md, settings.md, json-rpc.md, troubleshooting.md. Pass.
- The broker's side, with Mosquitto, Tailscale, 44Net, TLS, and example messages: live-broker.md. Pass, with the Tailscale access rule and the broker's behavior after a reboot marked as not yet tested.
- textweaver's own broker (`tw broker`, accounts, the pinned key, join codes, and the broker in the app) and the example in `docs/examples/live/`: live-broker.md, live-documents.md, command-line.md, privacy.md. Pass.
- Publishing (`tw publish`, with `tw share` and `tw send` as hidden aliases; Publish this document live; Publish a check-in) and the live document message format: live-broker.md, live-documents.md, command-line.md. Pass.
- Join links, Copy the join code and link, pasting to fill the form, the masked join code field and Show (`F3`), QR codes (`tw join-qr`, `--qr`, Show the join QR code, Join from a QR image, clipboard images), and the context menus: live-documents.md, live-broker.md, command-line.md, keyboard.md, json-rpc.md, known-limits.md, screen-readers.md. Pass.
- The live keys and settings reference: keyboard.md and settings-reference.md, generated. Pass.
- The crates `textweaver-mqtt`, `textweaver-broker` and `textweaver-qr`, the live document route through the app, the `mqtt` feature, the four fuzz targets (`mqtt`, `twlive`, `broker`, `qr`), and the Docker test with Mosquitto: dev/architecture.md, dev/building.md, dev/testing.md, fuzz/README.md, site/architecture.html, CONTRIBUTING.md. Pass, after fix.

## Still open

- The 293 keyboard actions were checked key by key against the guides by the owner, Sunday, October 4, 2026. The generated keyboard page is the key list. Pass.
- German, French, Portuguese, and Arabic translations are not yet checked by native speakers: see known-limits.md. Fix.
