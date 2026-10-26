# Changelog

All notable changes to textweaver. Versions follow [Semantic Versioning](https://semver.org/); before 1.0 every release is an alpha and anything may change.

## [Unreleased]

### Live documents

- New crate `textweaver-mqtt`: the receive-only MQTT 3.1.1 client for live sources, not yet connected to the app. One std thread per source, with rumqttc 0.25.1 for its packet codec only (default features off; `deny.toml` bans its TLS and WebSocket features) and TLS through the ring-backed rustls already in the tree, trusting the bundled Mozilla roots or a CA file; there is no way to skip certificate checks. It never publishes. It reconnects with backoff (1 s doubling to 60 s) and subscribes again after every login, and stops retrying when only the user can fix the problem, so a wrong password never hammers a broker. Connecting, connected and disconnected are reported once per change, not once per attempt, each reason a sentence in words with its meaning first, including Mosquitto 2's answer to a wrong password ("Not authorized: the broker refused the login. Check the user name and password."). Connected says whether the link is TLS and whether the broker is on a Tailscale address (a hint). A subscribed topic with no message for a set time (60 s) is named once, since Mosquitto grants a topic its ACL denies and then sends nothing. Every resolved address is tried at once, so `localhost` no longer costs two seconds on Windows when the broker listens on IPv4 only (TLS by `localhost` to Mosquitto: 94 ms, from 2142 ms in the spike). Payloads are capped (256 KiB, an oversize message skipped as it streams in and named; JSON nesting 8 deep; 5000 values; strings 2000 characters), and control characters, terminal escape sequences and bidirectional overrides are stripped from every topic, key and string. The client id is `tw-` and 16 random hex digits, never a personal identifier.
- Tests: a loopback broker inside the test program, plain and TLS with certificates made at test time (login, refusals, retained lists, QoS 1, keepalive, reconnecting, the quiet hint, caps, cleaning, a slow reader); an ignored test against Debian 13's Mosquitto 2.0.21 in Docker, run by `crates/textweaver-mqtt/docker/run-mosquitto-test.sh`; and a cargo-fuzz target, `mqtt`, for the packet reader and the payload decoder.
- **New: `[live]` settings**, in a new Settings section, Live documents: `announce_new_entries` (on: each new entry, whole, goes to the screen reader and so to the Braille display), `speak_new_entries` (off: textweaver's own voice stays quiet), `min_interval_seconds` (10, 0 to 3600: the values view's fewest seconds between announcements of one field), `quiet_hint_seconds` (60, 0 to 3600, 0 never: when a silent source gets a hint in words), and `log_folder` (empty: `logs` in the data folder).
- **New: live sources, `[live.sources.<id>]`**, edited in `settings.toml`: `name`, `host`, `port` (1883, or 8883 with TLS), `tls` (`off`, `system`, `ca_file`), `ca_file`, `username`, `topics`, `view` (`log` or `values`), `id_field`, and `[[live.sources.<id>.fields]]` with `path` (dotted, or a JSON Pointer), `label`, `unit`, `decimals`, `values`, `show`, `speak` and `order`. A value that does not fit is dropped with a warning in words ("live.sources.net.port 70000 is outside 1 to 65535; using the default"), and the rest of the source is kept. Sources and the log folder are this computer's settings and never sync.
- **New: live source passwords** are kept only in the system credential store, under the account `live:<id>`, beside the GitHub token, and asked for in a masked prompt (purpose `live_password` over JSON-RPC). A version without the credential store says so in words instead of asking. The credential store is now its own feature of `textweaver-components`, `credentials`, which `download` brings.
- **New: the live document core.** A live source (such as an MQTT broker, connected in a later change) feeds a log that is read like any document: the text changes in place while the reader's cursor, selection, bookmarks, find matches and spoken range stay on the same text, so a new entry never moves the screen reader or the Braille display. One JSON object adds an entry at the end; a JSON list, or an object wrapping one list, is the whole log and replaces it, keeping the reader on the same entry; an entry whose id (a configured field, or `id`, `uuid`, `serial`, `serialno` or `seq`) or, with none, whose callsign matches an earlier one corrects that entry in place. Common field names are recognized with no setup (callsign, name, location, status, traffic, notes, time, and their usual variants); other fields are kept and shown after them, so nothing is dropped. Text that is not JSON becomes an entry; a binary or oversized message is left out and said.
- The log shows as a list, one entry per line with the meaning first ("N0CALL, Ada Example. Mobile, portable antenna."), or as a table with real headers; switching keeps the reader on the same entry. The title line's position is frozen for a live document and shows the unread count instead ("3 unread"), counted from the furthest entry the reader has reached. Jump to newest moves to the last entry. Updates can be paused (up to 1,000 messages wait; a whole log replaces what waited) and resume as one change.
- A live document is read only: edit mode is refused in words, no reading position is saved, it gets no library or recent entry, it is never synced, and Save writes its JSON log.
- **New: the JSON log.** The entries as received, with their arrival and correction times, are written atomically to `live-log-YYYY-MM-DD-HHMM.json` (prefixed with the source's id when it has one) in the `logs` folder of textweaver's data folder, created on the first entry: at most every 10 seconds while the log runs, and when it closes. Opening such a file shows it in the same log view, read only.
- **The terminal reader with a live document open**, for a screen reader and a 40-cell Braille display. The status line carries each new or corrected entry, meaning first, cut at a word boundary to one Braille line of 40 cells, with "..." where words were left out (the log keeps them all), and the unread count after it when the whole still fits ("N0CALL, Ada.  3 unread"). The line is taken once per entry, so moving through the log does not change it and the screen reader does not read it again; any other message keeps the status line until the next entry. The title line's unread count changes with the reader's own keys and with pausing or the source stopping, not with each entry, so the title row stays still while entries arrive. A new entry draws the screen once and an idle log draws nothing; the reader's row, the rows above it and the hardware cursor stay where they were, and a short log grows into the empty rows below. Entries are taken on the reader's next pass of its event loop, at most 40 ms after they arrive, so the terminal reader needs no waker. A JSON log textweaver saved opens as a live log from `tw FILE` and the open prompt; it is recognized by its content, and any other JSON file opens as before.
- The commands, keys, settings and the MQTT source that use this core come in later changes; the core is reached through `App::open_live_source` and the `textweaver_app::live` module.

### Security

- tokio 1.53.1 to 1.53.3 in the lock, for RUSTSEC-2026-0386 (`AsyncFd` constructors). cargo deny, which checks every feature, found it through AccessKit's Linux adapter and rumqttc; textweaver-mqtt compiles tokio through rumqttc but never starts a runtime.

## [0.1.0-beta.4] - 2026-10-24

### CW (Morse code)

- **New: CW as a speech engine** (`[speech] backend = "cw"`, "CW (Morse code)" in Settings). Play sends the document in International Morse code (ITU-R M.1677-1) as a tone, exactly as written: the text normalizer is skipped, and a character Morse code does not have is skipped and counted on the status line when the reading ends. Word times come from the computed schedule, so the highlight and RSVP follow the word being sent exactly and stop with the audio when it pauses; audio export writes CW with the same word times. Speeds are PARIS words per minute with Farnsworth spacing by the ARRL formula. The rate keys change the character speed by one word per minute and the effective speed in proportion; the pitch keys move the tone by semitones. While CW is the engine, the title line (the status bar in the textweaver app) shows the CW speeds, "CW 20 wpm, effective 10" or "CW 20 wpm" without Farnsworth spacing, in place of the speech rate, and Say Status says them in words. Each element rises and falls on a 5 ms raised cosine, so it does not click. CW is opt-in: it is never a fallback engine ([CW](docs/cw.md)).
- **New: `[speech.cw]` settings**: `wpm` (20) and `effective_wpm` (10), each 5 to 50; `tone_hz` (600, 200 to 1500); `volume` (50 percent); `prosigns` (on: BT after headings and paragraphs, AR at the end of the document, and `<BT>` written in a text sent as a prosign); `status_line` (on: the characters being sent shown on the status line, newest last, in 40 cells, so a screen reader shows them on a Braille display; nothing extra is announced).
- **New: `tw cw monitor`** prints MIDI messages (notes on and off with their number and channel, controllers) and key presses and releases in words, with the time each arrived, for recording what a Morse key adapter sends. It only prints. MIDI comes through midir (MIT), on Windows through its WinRT backend so Bluetooth LE MIDI devices such as the N6ARA TinyMIDI are listed; `--port`, `--no-keys`, `--no-midi` and `--seconds` choose what it listens to, and `tw cw ports` lists the MIDI inputs.
- **New: Morse code practice** (Tools, Morse code practice; `cw_practice_koch`, `cw_practice_characters`, `cw_practice_words`, `cw_practice_progress`). Koch-method lessons in the order Learn CW Online teaches (from its source, `inc/functions.php`), starting with K and M at the CW character speed with Farnsworth spacing, and random character and word drills. Each group opens a prompt (purpose `cw_copy` over JSON-RPC); the learner types what they heard, and textweaver says the score in words, count first, with each miss ("4 of 5. Missed: K as R. Sent: K M R K M."), then sends the next group. Answers are lined up by edit distance, so one missed character costs one. A Koch round at `practice_threshold` percent (90) or better adds the next character. Nothing shows the group before the answer. With another speech engine, groups go to a CW engine started for practice, after the spoken message and `practice_delay_ms`. Progress (the Koch lesson and the latest 20 rounds) is kept in `cw-practice.json` in the data folder ([Practice and training](docs/cw.md#practice-and-training)).
- **New: practice settings in `[speech.cw]`**: `practice_threshold` (90, 50 to 100 percent), `practice_group` (5 characters, 2 to 10), `practice_groups` (10 groups a round, 1 to 50), `practice_delay_ms` (1000, 0 to 5000).
- New crate `textweaver-cw`: the Morse tables, timing, schedule and tone renderer, pure, with tests of the tables, PARIS and Farnsworth timing, and a golden hash of a rendered clip. The speech crate gains `Caps::RAW_TEXT` for engines that take the text exactly as written, and `SpeechService::respeak`.
- **New: CW keying** (`cw_keying`, Alt+Shift+F11 in both frontends, Edit menu). In edit mode, Morse code from a key or paddle is typed at the cursor; leaving edit mode ends keying. MIDI key adapters (default notes dit 20, dah 21, straight key 30, any channel: the N6ARA TinyMIDI's and the CTR2 MIDI's) work in both frontends; keyboard adapters (Left and Right Control for VBand and Vail, `[` and `]` for the TinyMIDI in keyboard mode) in the textweaver app, which hands their presses and releases to `App::cw_adapter_key` and gives them to nothing else while keying. textweaver does the keying on its own thread: an iambic keyer (mode B, mode A, or straight; dot and dash memory) at the character speed, and a decoder that learns the operator's speed from the last 24 elements, with character and word breaks at 2 and 5 units of silence. Letters are typed in lower case; AR, AS, CT, SK and SN as `<AR>` and so on (the form the CW engine sends as the prosign); BT starts a new paragraph; KN is `(`; HH, eight dits or more, erases the word just keyed. The status line shows the characters keyed, in 40 cells, without speech; Alt+Shift+R (RSVP) in edit mode shows the word being keyed. The sidetone plays at its own pitch, **CW sidetone pitch** (`[speech.cw] sidetone_hz`, 200 to 1500 Hz, 600 by default, beside the CW sidetone switch in Settings), apart from the CW tone of sent Morse code, on the speech engine's output device, made sample by sample with the same raised-cosine edges (`Player::start_generator` in the engine host). A new sidetone pitch or level is heard at once while keying, with the phase kept so it does not click.
- **New: keying settings in `[speech.cw]`**: `keyer` (`iambic_b`, `iambic_a`, `straight`), `swap_paddles`, `sidetone` (on), `dit_note`, `dah_note`, `straight_note` (0 to 127) and `midi_channel` (0 for any, or 1 to 16). The notes and the channel are per machine in sync.
- The keyer, decoder, adapter mapping and sidetone oscillator are in `textweaver-cw` (pure, tested with a virtual clock: squeeze and memory in modes A and B, straight-key decoding from 5 to 30 words per minute with 20 percent jitter, MIDI messages of both adapters and the CTR2's Extended BTN notes). MIDI inputs moved from `tw cw monitor` to `textweaver_app::cw_midi`, which both use (the app's `cw-midi` feature, on through the CLI's).

### Obsidian notes

- **Highlights are read and written as highlights.** `==highlighted text==` was announced nowhere and exported as underlined text (`<u>` in Markdown, an underline in Word and EPUB, the underline typeform in BRF). At high verbosity, continuous reading now says "highlighted" before it, as it says "strikethrough" before struck-out text. Markdown export writes `==text==`, HTML export and EPUB `<mark>`, Word a highlight (`w:highlight`), and BRF the first transcriber-defined typeform no highlight name uses in the document, named "highlighted text" in the transcriber's note (with all five in use it is left unmarked and the report says so). Markdown export also escapes doubled `=` and `%` in plain text, so they do not turn into a highlight or a comment when read back ([Obsidian notes](docs/converting.md#obsidian-notes)).
- **Comments are left out of converted files.** With `--flavor obsidian`, `tw convert` published `%%comments%%` in HTML and Markdown output. They are now left out of every output, inline or spanning several blocks, outside code; a `%%` that is never closed hides the rest of the note, as in Obsidian. `--keep-comments` keeps them (`RenderOptions::keep_comments`, `LoadOptions::keep_comments`). Reading already left them out, but read a code block inside a comment; one rule, shared by the reader and the renderer (`textweaver_formats::obsidian_source`), now hides everything inside. The new setting **Read Obsidian comments** (`[reading] read_comments`, off by default) reads them, without their `%%` marks.
- **New: inline footnotes.** `^[a note written in place]` was read as typed. It is now a footnote like any other, numbered after the note's own, in every footnote mode, and an endnote in HTML with the `obsidian` and `pandoc` flavors.
- **New: custom task statuses in words.** `- [/] Draft` was read "left bracket slash right bracket Draft". The item's label now says the status, as `[ ]` and `[x]` say "not checked" and "checked": "in progress", "cancelled", "forwarded", "scheduled", "important", "question", "starred", "quote", and the letters themes use; any other character is "marked" and the character.
- **Embedded files say what they are.** `![[slides.pdf#page=3]]` reads "Embedded PDF, slides, page 3" instead of the raw target; sounds, videos, and other files read "Embedded audio, talk", "Embedded video, demo", "Embedded file, data.xlsx", each a link to the file. A picture's size is read ("diagram.png, 300 pixels wide", "640 by 480 pixels"), and text after `|` that is not a size is its description.
- **Mermaid diagrams are named.** A code block fenced `mermaid` is read after "Mermaid diagram source" (skipped: "Mermaid diagram skipped"), and its first line says "code, Mermaid diagram".
- The canonical text version is now 11, so cached Markdown documents are read again.
- BRF: the first transcriber-defined typeform was written with a backtick, a character outside braille ASCII that embossers and Braille displays may not show; its indicator is now `@#` (dots 4, 3-4-5-6), as UEB writes it.

### Obsidian vaults

- **Links and embeds resolve across the vault, never outside it.** The vault is the nearest folder above a note holding `.obsidian`; one resolver in `textweaver-vault` serves the reader's links (Alt+Shift+F), its embeds, `tw convert --flavor obsidian`, and the vault commands. Order: a path from the note's folder, a path from the vault root, a name or path ending anywhere in the vault (the nearest note first; "2 notes named Dup; the nearest opened." is said once), then a front matter title or alias. A path leading outside the vault is refused. A note in no vault keeps its own folder and the folders below it, as before; the separate search depths (4 for links, 8 for embeds) are gone ([Obsidian vaults](docs/vault.md#read-vault-notes-in-textweaver)).
- **Block links open at the block.** `[[Note#^id]]` and `[[#^id]]` go to the block (they said "No heading called ^id"); a missing block says "No block id in this document." `[[Note#A#B]]` opens at heading B. A link to a missing note says "Link to a note that doesn't exist:" and its name.
- **One front matter parser.** The reader reads YAML front matter with the vault crate's parser, so block lists (`aliases:` then `- name` lines) and quoted values are kept; `alias`, `tag` and `cssclass` count as `aliases`, `tags` and `cssclasses`, and an alias with spaces stays one alias (import too). Cached documents are read again once.
- **Properties.** Opening a note with front matter ends with "Properties: 4."; `note_properties` (**f**) lists them, name first, lists joined by commas, checkboxes as "checked" or "not checked" ([Properties](docs/vault.md#properties-f)).
- **Notes that link here, links to notes that don't exist, tags in this note, and tags in the vault,** as lists in both frontends (command palette, and Reading, Go to): Enter opens a note or lists a tag's notes, nested tags included. A vault index (names, titles, aliases, tags, links; at most 20,000 notes, 4 MB a note, 256 MB in all) is built on first use and refreshed from file times before each.
- **`tw vault backlinks`, `unresolved`, `tags` (with `--tag`), and `properties`,** read-only, each with `--json`; `properties` also lists Dataview `key::` fields, which only the vault commands read ([Ask the vault](docs/vault.md#ask-the-vault)).
- **New: canvases.** `.canvas` files (JSON Canvas 1.0, `CanvasLoader`, `textweaver_formats::canvas`) open as documents: a summary ("Canvas: 4 cards, 1 group, 3 connections."), each node a heading; text cards with their first line as the heading and the rest read as Markdown, headings inside a card one level below it; file cards as links ("Mitochondria, Energy, note") that Alt+Shift+F follows through the vault resolver; link cards as web links; groups as sections holding the nodes whose centers they hold (the smallest group), one heading level down, to level 6; preset colors in words. Reading order: rows top to bottom, a node joining a row when its top is above the middle of the row's first node, left to right within a row. Connections last, under "Connections": "From A to B: label", "Both ways between A and B", "Between A and B". Bounded: the JSON loader's size and nesting limits, 5,000 nodes and 20,000 connections. Converts to every output ([Canvases](docs/vault.md#canvases)).
- **New: editing aids for Obsidian notes** in edit mode, in both frontends: `insert_wiki_link` lists the vault's notes (the vault index), then "The whole note" and the note's headings and block ids, and writes `[[Note]]`, `[[Note#Heading]]` or `[[Note#^id]]` (the selection becomes the shown text; a duplicate name is written as its vault path; a link into the open note as `[[#Heading]]`); `toggle_task` (Ctrl+L in the terminal, Ctrl+Enter in the app, Cmd+Option+Enter on macOS: Obsidian's keys) checks or unchecks the task on each selected line, or makes the line a task, and says "Checked.", "Not checked." or "New task, not checked."; `cycle_task_status` steps `[ ]`, `[/]`, `[x]`, `[-]`, `[>]`, `[!]`, `[?]` and says the status; `insert_callout` lists the thirteen callout types and writes `> [!type]` with a body line, quoting selected lines. Each is one undo step; Insert and Format menus, and the palette. Editor: `MarkdownOp::ToggleTask`, `MarkdownOp::CycleTaskStatus`, `markdown::wiki_link`, `markdown::insert_callout`, `markdown::task_status` ([Writing and editing](docs/editing.md#obsidian-notes-wiki-links-tasks-and-callouts)).
- **Converted web pages link across folders.** With `--flavor obsidian`, `[[Plan]]` links to the page of the note it resolves to, such as `../Projects/Plan.html`.

### Reading and speech

- **Links to other documents open in textweaver, with Back and Forward.** Enter on a link in reading mode now follows it, as in a screen reader's browse mode; Ctrl+Space still always reads from the cursor (Option+Space in the terminal reader on macOS, where Enter alone used to be the read key), and Alt+Shift+F follows in every mode as before. A link to a local file, relative to the document or absolute and in any format textweaver opens, opens it in the same reading view at the heading the link's anchor names (`keyboard.md#edit-mode`, by the GitHub anchor rule or the heading's text), or at its start. Arriving is said in words: "Opened Keyboard reference, at Edit mode. Back: Alt+Left or Backspace." A link followed from unsaved edits asks first, as opening a file does, and then arrives at the heading; before, the heading and the way back were lost. Documents opened by links form one history, like a browser's: Back (Alt+Left, Backspace) returns to the link at the exact reading position, once no earlier jump is left in the new document rather than only before any jump there, and says "Back in Quick start:" with the words there; Forward (Alt+Right, backslash) returns to the document left. A missing file is named and the document stays; web and mail links still ask before opening in the browser ([Follow a link or a footnote](docs/reading.md#follow-a-link-or-a-footnote-enter-or-altshiftf)).

- "Paused." is said at every interface announcement level, minimal and off included, so pausing is always confirmed with a screen reader running.

### Braille

- **New: the Braille view.** The textweaver app draws the current line, or the sentence being spoken, as braille dots in a bar at the bottom of the app, under RSVP when both are on, as a learning and teaching tool and for checking spacing. Show or hide it with Alt+Shift+F2 or Braille view under Reading aids in the View menu (`[braille] viewer`, off by default). It shows `[braille] display_cells` cells at a time (40 by default) and pans to the part with the spoken word or the cursor, breaking between words where it can; every space is a cell. The dots are drawn at the standard cell proportions and scale with the document text size: a raised dot is a filled disc, an empty position a thin ring, so the two differ by shape, never by color alone; eight-dot cells get a fourth row. Teaching options, off by default: a print line under each cell or run of cells (`viewer_print_line`), exact in uncontracted braille and paired word by word through each word's contracted beginnings in contracted braille, and cell outlines (`viewer_cell_outlines`). It fades with the bars in focus mode. To screen readers it is one image named "Braille view" with a description saying it is visual only; its braille is never in the accessibility tree and never announced. In the terminal reader the same key shows the line in Unicode braille on a row above the status line, in self-voicing mode only, so a screen reader never reads it out ([The Braille view](docs/reading-aids.md#the-braille-view)).
- **New: the braille grade setting.** `[braille] grade` chooses uncontracted (grade 1, the default) or contracted (grade 2) Unified English Braille for the Braille view and for BRF files exported from the reader; Alt+Shift+F3 switches it and says which. Contracted braille goes through liblouis with the BRF writer's table, and falls back to uncontracted, saying so, when liblouis is missing. Reader builds now include the BRF writer's liblouis support.
- The native UEB translator can say which print character each braille cell stands for (`ueb::translate_mapped`), and the BRF writer's liblouis bridge can return Unicode braille with dots 7 and 8 (`louis_unicode`).

### The textweaver app

- **Scroll down and Scroll up work in the app.** `j` and Shift+J (and Reading, Go to, Scroll down and Scroll up) said "This command works in the terminal reader." They now move the document by one drawn line, a wrapped paragraph one line at a time, and leave the cursor where it is; the app says what the terminal reader says, through one shared method (`App::scrolled_to`): the line at the top at high verbosity, and "Top of document." or "Bottom of document." at the ends. The view keeps its place until the cursor or the spoken word moves (`DocumentView::scroll_line`).
- **Line numbers work in the app.** View, Line numbers, and the command palette's "toggle line numbers", turn on `[display] show_line_numbers`, shared with the terminal reader, and the app draws each line's number in the left margin in the dimmed text color, left of the "reading from here" mark. They are drawn only; screen readers and Braille displays get the text without them. There is no key in the app, where F6 moves between regions. The setting now appears in the app's Settings dialog under Display ([Scrolling and line numbers](docs/gui.md#scrolling-and-line-numbers)).
- **New: selecting with the mouse**, in reading mode and in edit mode. Dragging selects (the document view's own `DocState` selection, the one Shift and the cursor keys make), Shift with a click extends it, a double click selects a word (Parley's `Selection::word_from_point`, without trailing white space), and a triple click a paragraph with its line break, which in edit mode is a line of the source; dragging after a double or triple click extends by words or paragraphs. A drag at the top or bottom edge scrolls a drawn line at a time, faster past the edge. The view draws the selection as it grows but reports it once, when the button comes up: one `DocAction::CaretMoved`, whose echo is the selected text as for Shift with a caret key, and the AccessKit selection keeps its value until then, so a screen reader is told once. A click still places the cursor (on a link too; Enter follows it), the right-click menu keeps a selection it lands on, and pointer moves still reach focus mode's reveal ([Selecting with the mouse](docs/gui.md#selecting-with-the-mouse)).
- No command is the terminal reader's alone any more: the app's menus and command palette list every command, and the parity test now fails if one is left out without a reason recorded in `docs/gui.md`.
- **Settings rows for tables say their name.** Speed presets, Voices by language, Rate and pitch per voice, and the Colors dialog's highlight palette entries were named only by their value through UI Automation, AT-SPI and macOS accessibility ("4 entries", "none"), because AccessKit names a label by its value. Each row now says the setting first ("Speed presets, 4 entries").
- New tests check that every setting of every Settings section, about fifty in Speech, is in the accessibility tree under its own name, and that Down reaches the last one and scrolls it into view, in a window the size of a maximized one on a 1080p screen.

### Settings

- **Changed: settings are organized by task**, in 24 short sections (at most 15 settings each) named for what the user does, in the same order in the terminal reader's settings screen, the textweaver app's Settings dialog and the generated settings reference: Voice and speech; Pauses and verbosity; Speaking text; Accessibility and announcements; Reading; Documents; Highlight; Math; Braille; Morse code; Morse practice; Display; Colors; Spacing, font and ruler; Word aids; Word by word (RSVP); Editing; Library and study tools; Dictation; Keyboard and language; Toolbars and panels; Export; Sync; Updates and components. The placement lives in `textweaver_app::settings_sections::SECTIONS`; a test keeps every setting in exactly one section. `settings.toml`, `tw settings` and JSON-RPC keep their keys; JSON-RPC's `settings_schema` gains `section_id`.
- **New: engine-aware sections.** `settings_sections::applies` decides, in one place for both frontends, whether a setting does anything given the speech engine (the one chosen, or the one running when the choice is automatic) and the system: no voice, rate or pitch while CW is the engine; CW prosigns and status line only with CW; each engine's own options only with that engine; OneCore voices and Code Factory's Eloquence only on Windows, the Apple engine choice only on macOS. Sections list only what applies. While CW is the engine, the sound of Morse code (character and effective speed, tone, level) moves from Morse code to the top of Voice and speech; keying and practice use it with any engine.
- **New: All settings, and search that finds everything.** The last item of the section list is All settings; typing searches every setting from anywhere on the screen. A setting that does not apply now says why after its value ("Applies when Eloquence is the speech engine."), and so does a change to it.
- **Changed: the terminal reader's settings screen** opens on the sections, each with its count ("Morse practice, 4 settings"), after the recently changed settings. Enter or Right opens a section, Escape goes back to the sections on the section left, and Escape there closes. A change that alters what an open section lists says the new count.
- **Fixed:** Piper is offered as a speech engine in Settings; Panel beside the document is no longer listed in the terminal reader, which has no panel; a duplicate schema entry for Preview pause is gone.
- The settings reference (`docs/settings-reference.md`) is grouped by the new sections, opens with a list of which sections hold each `settings.toml` table's keys, and says for each setting when it applies. `docs/settings.md` lists every setting by section, with full keys.

### For contributors

- **The UI Automation report follows the reorganized Settings.** `crates/textweaver-xilem/tools/uia-report.ps1` names the form after the first section in the Sections list ("Voice and speech settings") instead of a fixed name, and, because the dialog shows one section's rows at a time and Voice and speech has no on/off setting, chooses the following sections through SelectionItemPattern until one shows a check box (Pauses and verbosity), then toggles it through TogglePattern and back as before. If Select does not change the section, the report tries Invoke and then focus, and fails, naming the way that worked. The app's own time limit in the report is now 150 seconds, so a busy machine still reaches the font list.
- **The Windows accessibility tree dump retries again.** `tools/a11y/tree-dump.ps1` asks accessibility-cli for the tree every two seconds for up to 40 seconds, but Windows PowerShell made the tool's first error fatal, so one try made before the textweaver app showed its window ended the script and left the app running. The script now checks the tool's exit code, keeps trying, always stops the app, and on failure prints the end of the app's log. accessibility-cli's "Could not find window for PID n (found m top-level windows)" means the app had no window yet; m counts every top-level window on the desktop.

### Packages and the build

- The links in the packaged QUICKSTART.md and INSTALL.md now reach the guides in the package's docs folder, so following them works in an installed copy.

### Package sizes

- `textweaver-0.1.0-beta.4-linux-aarch64.AppImage`: 43.3 MB (45,394,440 bytes)
- `textweaver-0.1.0-beta.4-linux-aarch64.tar.gz`: 45.6 MB (47,770,646 bytes)
- `textweaver-0.1.0-beta.4-linux-x86_64.AppImage`: 46.3 MB (48,527,864 bytes)
- `textweaver-0.1.0-beta.4-linux-x86_64.tar.gz`: 48.7 MB (51,106,803 bytes)
- `textweaver-0.1.0-beta.4-macos-universal.zip`: 76.7 MB (80,373,686 bytes)
- `textweaver-0.1.0-beta.4-windows-x86_64.zip`: 49.3 MB (51,712,645 bytes)

## [0.1.0-beta.3] - 2026-10-17

### Changed

- **`textweaver` now opens the app; `tw` is the terminal.** textweaver is one program instead of three, so the download is simpler. `textweaver`, alone or with a file, opens the textweaver app. The terminal reader and the commands are the same program: use `tw`, or `textweaver --tw` (`--cli` means the same). A command given to `textweaver`, such as `textweaver convert`, still runs the command line, so scripts keep working, but `textweaver FILE` now opens the file in the app rather than the terminal reader. `textweaver-gui` is gone; the old launcher, `textweaver.exe`, that started `tw.exe` is gone too, since `textweaver.exe` is now the program. On Windows, `tw.exe` is a small console program that runs `textweaver.exe --tw` with the same arguments and waits for it, so terminals, screen readers, and Braille displays work with the terminal reader as before; in terminals there, use `tw`. On Linux and macOS `tw` is a link to `textweaver` (on macOS, to the program inside `textweaver.app`). Settings, data folders, and the JSON-RPC interface are unchanged ([The command line](docs/command-line.md#one-program)).
- **Fewer programs in the package, the same engines.** An antivirus program flagged two of textweaver's small engine host programs with a heuristic detection. The separate host for each engine is gone: `textweaver` now runs Eloquence, SAPI 5 voices, DECtalk, and eSpeak NG in a copy of itself, started in a hidden host mode (`--engine-host`), and on Windows one 32-bit host, `textweaver-host-x86.exe`, runs every 32-bit engine and voice. The Windows package holds three programs instead of eleven (`textweaver.exe`, `tw.exe`, and the 32-bit host), and the Linux package no host programs. Engines still run in their own process, so a crash cannot close textweaver, and word timing is as exact as before.
- `TEXTWEAVER_ECI_HOST`, `TEXTWEAVER_SAPI_HOST`, `TEXTWEAVER_SAPI_HOST_X86`, `TEXTWEAVER_DECTALK_HOST`, and `TEXTWEAVER_ESPEAK_HOST` still work. They may name a separate host from an earlier release, which is used as before, or any other program, which is started in host mode. The old host programs (`textweaver-eci-host.exe` and the like) are no longer needed; an update may leave them in the folder, and they can be deleted.
- `cargo xtask hosts` builds only the 32-bit host (Windows); elsewhere there is no host to build. The Linux install script no longer runs it.
- **Ctrl+Shift+V now pastes as plain text in edit mode, and Choose a voice moved to Ctrl+Alt+V.** In the textweaver app, Ctrl+Shift+V (Cmd+Shift+V on macOS) pastes the clipboard's plain text, as in most programs; Ctrl+Shift+M still works. Choose a voice is Ctrl+Alt+V in the app (Cmd+Option+V on macOS) and also in the terminal reader, where Alt+V keeps working. In the terminal reader, Ctrl+Shift+V stays the terminal's own paste, which is always plain text. If you moved either command in `keymap.toml`, your keys are unchanged.
- **F11 is now full screen in the textweaver app, and the rate keys match the terminal reader.** F11 switches full screen, as in most Windows and Linux programs; Alt+Enter still does (Option+Enter on macOS, where macOS keeps F11 for the desktop). F11 and Shift+F11 no longer change the rate in the app, and Shift+F11 is unbound. While reading, + (or =) and - change the rate in the app as in the terminal reader, from the main keys or the number pad; on macOS Cmd+Option+] and Cmd+Option+[ stay. In the terminal reader F11 is still the next chapter. If you moved these commands in `keymap.toml`, your keys are unchanged.
- **Escape leaves edit mode.** In the terminal reader and the textweaver app, Escape in edit mode first closes whatever is open on top (a menu, list, panel, prompt, find bar or dialog), then stops reading if it is under way, then clears a search's marked matches; with none of these, it leaves edit mode exactly as Ctrl+E does: the edits are kept (with the same question about unsaved changes), the cursor stays where it was, a dictation under way is finished first, and the announcement is "Edit mode off." The hint "Still editing. Ctrl+E finishes." is gone. The JSON-RPC `stop` method still only stops reading ([Editing](docs/editing.md#start-and-finish-editing-ctrle)).
- **The app's Edit button moved to the toolbar.** It was on the header as "Start editing" and "Finish editing"; it is now one toolbar button, after Next paragraph, named "Edit" (Ctrl+E) in reading mode and "Done editing" (Ctrl+E, Escape) in edit mode, where it stands directly before Start dictation. Pressing it moves the focus to the document, as Ctrl+E leaves it. The header's defaults are Open, Font, Settings and Commands. A header or toolbar you customized keeps your choice; if the bar holding the button is hidden, Done editing appears on the other bar while you edit. The terminal reader's key hints are unchanged ([What is in the app](docs/gui.md#what-is-in-the-app)).
- **Leaving edit mode keeps the cursor's place.** In a Markdown file, finishing with Ctrl+E (or Escape, or Done editing) without changing the text put the reading cursor back where it was when editing began, often the top heading. It now lands on the block and word the editor's cursor was on, as it already did after a change ([Editing](docs/editing.md#start-and-finish-editing-ctrle)).

### Dictation

- **New: pause and resume dictation.** Pause dictation (Alt+Shift+F9, in the Edit menu and the command palette) pauses the microphone during a dictation and resumes it. The phrase being spoken is finished and typed first, so no words are lost; what the microphone hears while paused is discarded, and the session, the loaded model and the text stay. "Dictation paused." and "Dictation resumed." are said, and the terminal reader's status line begins "Dictation paused." while it lasts.
- **New: dictation buttons in the textweaver app.** While a dictation is under way, the toolbar shows Pause dictation (Resume dictation while paused) and Stop dictation beside the other buttons, before Slower and Faster, or on the header when the toolbar is hidden. They leave when the dictation ends, and the focus moves to the document if it was on one of them.
- **New: a Start dictation button in edit mode.** In edit mode the textweaver app's toolbar shows Start dictation, before Slower and Faster, which runs Dictate (Ctrl+Shift+F9) and names that key to a screen reader. When no dictation model is installed, it says so and offers the download, as the key does. During a dictation it gives way to Pause dictation and Stop dictation; leaving edit mode removes it, and the focus moves to the document if it was on it.

### Study

- **New: study cards go to Anki and back.** Export cards to Anki writes a document's cards as an Anki package (`.apkg`) beside it, in a deck named after the document, each note tagged with the document and the card's kind; graded cards keep their interval, ease, due day, and grades. Import Anki cards adds a package's Basic and Basic-and-reversed notes to the open document's cards as plain text, with their grades, and says what it skipped: "Imported 120 cards. Skipped 4 cloze notes." Importing the same package twice adds nothing. From the command line: `tw study export --anki FILE` and `tw study import FILE` ([Take cards to Anki and back](docs/notes.md#take-cards-to-anki-and-back)). AnkiConnect sync stays dropped.

### Library

- **New: the reading queue.** A list of documents to read next, kept with the bookshelf, so it is there the next time textweaver starts. **Ctrl+Enter** in the library list adds the focused document or takes it out; **Add to reading queue** in the File menu does the same for the open document; **Reading queue** in the File menu lists it, where **Enter** opens a document and **Delete** takes one out. A document leaves the queue when it is read to the end. The setting **After a document ends** (`[reading] after_end`) decides what follows the end of a document: nothing (`"stop"`, the default), the next document named, "Next: Week 4. Enter to read." (`"ask"`), or the next document read at once (`"next"`). A document is never started unless you ask. From the command line: `tw library queue`, with `add`, `remove` and `next`.
- **New: the review list,** beside Continue reading: the documents you have read aloud, the one read longest ago first, such as "Pharmacology 2, 21 days, 80 percent, 3 notes". It is a convenience for finding older readings, not a study method. From the command line: `tw library review`.
- **The reading queue syncs between computers,** with the places switch in sync. A document added on one computer joins the queue on the others, in the order documents were added; one taken out, or read to the end, leaves it everywhere and does not come back from a computer that had not heard, unless it is added again later. A document queued on another computer and not found on this one keeps its place, listed as "Week 4, not on this computer"; Enter on it says so, and it becomes an ordinary entry once you open it here. The queue goes to the sync folder as a file of its own, `queue.json`, which earlier versions never read. [Syncing between computers](docs/sync.md#the-reading-queue) has the rules.

### The app

- **Each highlight name has its own color and shape in the textweaver app,** as in the terminal reader: the name's color as the band, with black or white text, whichever reads better, and its shape drawn (a line, a double line, bold text, a dotted line, brackets, or a small square at the start), so no two names differ by color alone. A screen reader is given the name in words ("Highlight, important") on the text it covers, with the band and the line as text attributes ([Highlight colors](docs/notes.md#highlight-colors)).
- **The self-test's answer is said once.** After Enter revealed an answer, the app said it, and a screen reader then said the prompt's new text, with the answer, a second time. The prompt now keeps the text the screen reader knows until the focus moves; coming back to it reads the answer. The same holds for other changes the app says on a list's focused row, such as reversing a study card.
- **Formatted text pastes as Markdown on macOS and Linux too.** Ctrl+V (Cmd+V) in edit mode reads the clipboard's HTML (the pasteboard's on macOS, the Wayland or X11 clipboard's on Linux) and pastes headings, lists, emphasis, links, tables, and code as Markdown, as on Windows and in the terminal reader. Without HTML, the plain text is pasted.

### Help and documentation

- The troubleshooting guide explains antivirus warnings that name a heuristic detection: what they mean, how to check the package against `SHA256SUMS.txt`, and how to report a false positive to the vendor.

### Packages and the build

- **One program in every package.** `cargo xtask dist` builds `textweaver` (the `textweaver` binary of `crates/textweaver-xilem`, which runs `textweaver-cli`'s library for the terminal) and, on Windows, `tw.exe`, the launcher in the new crate `textweaver-launcher`, which uses only the standard library. The package check fails if a Windows package holds any program but `textweaver.exe`, `tw.exe`, and `textweaver-host-x86.exe`. `textweaver-cli` keeps a terminal-only `tw` binary for development and its tests; it is not packaged. The Windows package measured 48.8 MB in a local `cargo xtask dist`, 37.7 percent smaller than beta 2's 78.3 MB, since the app and the terminal program no longer each carry the shared code.
- **Upgrading an earlier install.** `install-windows.ps1` replaces the install folder's files as before, which removes `textweaver-gui.exe`; it makes the shortcuts "textweaver app" (`textweaver.exe`) and "textweaver terminal" (`tw.exe` in Windows Terminal), removes the old "textweaver" terminal shortcut only when it starts a program in the install folder, and lists the programs it installed in `install-manifest.txt`. `install-linux.sh`, `install-macos.sh`, and the AppImage's `--install` remove the `textweaver-gui` link and menu entry they made. The in-app update and `tw update` move an earlier release's `textweaver-gui` aside after the swap, start `textweaver` in its place, and point the Start menu shortcuts made by the install script at the new programs.
- **Linux menu entries.** The app's entry is `textweaver.desktop` (`Exec=textweaver`), and the terminal reader's is `textweaver-terminal.desktop` (`Exec=tw`, in a terminal). The program loads Fontconfig when the app starts instead of linking it, so `tw` runs on servers and in containers without it.
- **Linux tarball updates keep the `tw` link.** An update now unpacks a link in the package when it points at a plain path inside the package, as `tw -> textweaver` does, instead of refusing the package.
- **Release assets.** The `-gui.AppImage.zsync` copies for the app AppImages of 0.1.0-alpha.9 and earlier are no longer published; a release has nine files.
- **The app's shortcut is named "textweaver app".** The Start menu shortcut that `install-windows.ps1 -Gui` makes and the menu entry from `install-linux.sh --gui` were named "textweaver window". On Windows the script removes the old shortcut, `textweaver window.lnk`, when it makes or removes the new one, and only when that shortcut starts `textweaver-gui.exe`; on Linux the menu entry is rewritten in place. The documentation's navigation and the terminal's note for a command that only the app has say "the textweaver app" too.

### For contributors

- `cargo xtask release` no longer rewrites lines that record history. A line ending with the comment `<!-- history -->`, and the "Last listening check" line, keep the version they name. The beta 2 release changed such lines by mistake, and they had to be restored by hand; one, about the app AppImages of 0.1.0-alpha.9 and earlier, is corrected now. The release guide says how to mark a history line.
- The Orca session in the screen-reader checks passes again. It had failed since Saturday, October 3, 2026, because the app started each session as a first run, and the first run's Language list and Optional components list took the focus from the document. The Orca, NVDA, and VoiceOver sessions now start with a state folder whose settings file marks the optional components list and the question about updates as already shown, so it is not a first run and nothing asks for the focus. The Orca session also sets interface announcements to normal, since with Orca running the automatic level is minimal, which leaves out the "Paused." its pause step checks. The AT-SPI session also writes its report when it breaks or is stopped, saying why, instead of leaving none.

### Package sizes

- `textweaver-0.1.0-beta.3-linux-aarch64.AppImage`: 42.9 MB (44,931,592 bytes)
- `textweaver-0.1.0-beta.3-linux-aarch64.tar.gz`: 45.1 MB (47,309,223 bytes)
- `textweaver-0.1.0-beta.3-linux-x86_64.AppImage`: 45.8 MB (48,056,824 bytes)
- `textweaver-0.1.0-beta.3-linux-x86_64.tar.gz`: 48.3 MB (50,595,181 bytes)
- `textweaver-0.1.0-beta.3-macos-universal.zip`: 75.8 MB (79,457,191 bytes)
- `textweaver-0.1.0-beta.3-windows-x86_64.zip`: 48.8 MB (51,160,505 bytes)

## [0.1.0-beta.2] - 2026-10-10

In short: beta 2 fixes the speech rate, and brings every document up to date.

### Reading and speech

- **The speech rate changes again with Eloquence and DECtalk.** A voice saved while another engine spoke (a Windows SAPI 5 voice during a fallback) made both engines refuse every change of voice, rate, pitch, and volume, so the rate stayed fixed while the number on screen moved, and DECtalk could fail to start. Another engine's voice is now set aside quietly, and rate, pitch, and volume always apply; the "unknown voice" error no longer appears. Both engines now speed up past 430 words per minute.
- **Choosing a speech engine in Settings switches to it at once,** instead of waiting for Restart speech.
- **Speech no longer goes silent with RSVP.** With RSVP shown, Play/Pause started RSVP's silent word display instead of speech, so turning RSVP on while paused and then resuming gave silence. Play/Pause now always reads aloud, and RSVP follows the speech; Alt+Shift+P still starts RSVP on its own.
- An environment variable that names a helper program (`TEXTWEAVER_FFMPEG`, `TEXTWEAVER_PANDOC`) now wins over a copy in the components folder.

### The app

- **New: focus mode and full screen.** With View, Focus mode on (`[gui] focus_mode`, off by default), the header, the toolbar, the status bar and the panel fade once the mouse rests for three seconds, and the text takes the window; moving the mouse brings them back, and keys never do. Full screen (Alt+Enter, or View, Full screen) does the same. The hiding is visual only: screen readers, the Tab order and the focus are unchanged, and nothing is announced.

### Settings and data

- Settings shows the highlight palette as a count of entries, not raw data running over the Highlight section, and a SAPI voice by its name rather than its registry path.

### Help and documentation

- Every guide, the README, and the developer documentation were checked against beta 1 and brought up to date: one download per system, `tw` as the one terminal program, the beta stage, privacy (updates, fetched helpers, the credential store), and the settings that were missing from the settings guide.

### Package sizes

- `textweaver-0.1.0-beta.2-linux-aarch64.AppImage`: 66.4 MB (69,638,664 bytes)
- `textweaver-0.1.0-beta.2-linux-aarch64.tar.gz`: 70.7 MB (74,160,981 bytes)
- `textweaver-0.1.0-beta.2-linux-x86_64.AppImage`: 71.7 MB (75,176,440 bytes)
- `textweaver-0.1.0-beta.2-linux-x86_64.tar.gz`: 76.3 MB (80,014,927 bytes)
- `textweaver-0.1.0-beta.2-macos-universal.zip`: 126.1 MB (132,202,558 bytes)
- `textweaver-0.1.0-beta.2-windows-x86_64.zip`: 78.3 MB (82,128,223 bytes)

## [0.1.0-beta.1] - 2026-10-10

In short: beta 1 adds study tools (study cards with SM-2 scheduling and sync, a self-test, recall prompts), links between notes with seven export formats, tracked changes and comments written back to Word files, a named highlight palette, regular-expression find and replace, Markdown paste, a preview pane, polished HTML, DAISY 2.02 books and recorded DAISY narration, braille (BRF) files such as NLS BARD downloads, five more formats through carta, offline searchable help, optional components fetched for you (including from your own repository), update checks, and one download per system holding both programs.

### Reading and speech

- **Stop at the end of a section or after a set time,** and **repeat the current sentence slower** (Shift+X).
- **Recorded narration in DAISY 3 and 2.02 books** plays as you read, the highlight following each phrase; speech reads any text without audio (`[reading] book_audio`).
- **An eSpeak NG helper program** runs eSpeak NG out of process on Windows, so any installed eSpeak NG works with any build and a crash cannot take the reader down.
- **Speech never stays silent while an engine works:** engines fall back in order, each source is named ("Eloquence (OpenEVV, direct)"), and engines that cannot exist on the system are hidden. OpenEVV's root dictionary now loads in the background after speech starts.

### Study and notes

- **Study cards** from highlights, notes and headings; a study session graded Again, Hard, Good or Easy; **SM-2 scheduling** ("Due today: 7 cards"); cards and grades **sync** between computers. `tw study due`.
- **Self-test** from the study sheet, and optional **recall prompts** at section ends.
- **Links between notes** with the ten star relation types, "What links here", and `tw notes links`; **knowledge-graph export** as JSON, DOT, Markdown, GraphML, Mermaid, PlantUML and a CSV edge list.
- **A named highlight palette** (important, define, question, example, review): Alt+1 to Alt+5, a distinct shape and braille typeform for each name, filtering and collecting by name.

### Documents and conversion

- **Tracked changes and comments:** a list with author and date in words; accept or reject one, by author, or all; reply, resolve and add comments; **write the decisions back into the Word file** in place, with a backup; optional **tracked edits** from edit mode under `[authoring] author`.
- **DAISY 2.02 books** open from `ncc.html`, their folder, or a zip.
- **Braille files (BRF and BRL),** such as NLS BARD's braille downloads, are read as print through liblouis, with "Show original Braille".
- **carta** reads Org, reStructuredText, MediaWiki, DokuWiki and Jira, and writes AsciiDoc, Typst, LaTeX, MediaWiki and Org, in pure Rust.
- **Polished HTML pages:** typography from your reading settings, highlighted code that never relies on color alone, figures, print and forced-colors styles.

### Writing and editing

- **Find and replace with regular expressions,** captures, spoken previews ("Match 2 of 5, line 12: teh becomes the"), and a panel in the app.
- **Formatted text pastes as Markdown** (in the terminal reader, and in the app on Windows); paste as plain text with Ctrl+Shift+M in the app and Alt+Q in the terminal reader.
- **A context menu** (Applications key, Shift+F10, right-click), native on Windows.
- **A preview pane** beside the editor (Alt+F5), and one setting for the browser preview, "Browser preview follows".

### The app

- The keyboard list and the command palette filter as you type, with short names and the key on each row; F1 explains the row.
- **Customizable buttons**; the toolbar's navigation pair is now previous and next paragraph.
- The Colors dialog measures every row and each highlight name, and respects Windows high contrast.
- The menu bar is hidden by default on Windows (Alt or F10 shows it), menus have no duplicate commands, the window is opaque on every graphics card, and the loom logo is the icon.
- "Updated to" is said once after an update.

### The terminal reader and the command line

- **`tw` is the one terminal program:** `tw` or `tw FILE` opens the reader, `tw COMMAND` runs headless, and `textweaver` is a second name for it.
- `tw changes accept|reject FILE` replace `--accept-all` and `--reject-all`, which still work through beta 1.

### Settings and data

- **A developer profile example** (`docs/examples/developer-profile.toml`), and `[export] audio_format` for the format offered first.
- Code blocks are read by default (`[speech] skip_code` is off).
- Export asks where to save and under what name.

### Help and documentation

- **Help, Documentation opens the bundled guides in textweaver;** **Search help** finds commands, keys, settings and guide sections.
- The guides were revised for university-level readers; the GUI is "the textweaver app", not "the window".
- Every error message gives a next step.

### Components and updates

- **Optional components from your own repository or folder** (`components.toml`), with a GitHub token kept only in the system credential store.
- **ffmpeg, liblouis and Pandoc fetched for you** when a feature needs them, from their official releases.
- **Update checks:** asked on first run, at most once a day, never downloading without asking; packages verified against the release checksums. `tw update`.

### Packages and the build

- **One download per system and processor,** holding the app, `tw`, every engine helper and the complete documentation; the separate `-gui` packages are gone. The Linux AppImage starts the app by default.
- Every package carries the complete user documentation, and a missing guide is a warning, never a failure.
- Copyright (C) 2026 Jon Pielaet, in `NOTICE`, the README, `--version` and About.

### Package sizes

- `textweaver-0.1.0-beta.1-linux-aarch64.AppImage`: 66.4 MB (69,634,568 bytes)
- `textweaver-0.1.0-beta.1-linux-aarch64.tar.gz`: 70.7 MB (74,170,533 bytes)
- `textweaver-0.1.0-beta.1-linux-x86_64.AppImage`: 71.7 MB (75,135,480 bytes)
- `textweaver-0.1.0-beta.1-linux-x86_64.tar.gz`: 76.3 MB (79,988,217 bytes)
- `textweaver-0.1.0-beta.1-macos-universal.zip`: 126.0 MB (132,158,701 bytes)
- `textweaver-0.1.0-beta.1-windows-x86_64.zip`: 78.3 MB (82,108,049 bytes)

## [0.1.0-alpha.9] - 2026-10-04

In short: this is the last alpha before beta 1, so it finishes and tidies what is there. The window gains a reading settings dialog, a narrow layout, drawn list markers, and a first run of at most three skippable steps. Documents can be exported as a read-along web page, as Ogg Vorbis audio, and as an MP4 video with the spoken word marked. Captions gain karaoke tags and chapters. The command line follows one set of rules. Themes are checked for contrast, and the result is said in words. Written pauses are honored in every speech engine. The documentation has start pages for students and staff, a known-limits page, and an accessibility statement.

### Reading and speech

- **Written pauses.** `<break time>` and `<break strength>` in a document are read as pauses of that length in every speech engine. The markup is removed before speech and screen. New setting: `[speech] markup_pauses`.
- **Reading passes.** Shift+F reads the full text, the first sentences only, or the headings only.
- **Document overview.** A short spoken summary of what the document holds.
- **Pictures without a description** are read as "graphic, no description" instead of being skipped.
- **Print page numbers** are read back from EPUB and HTML files that carry them.
- **Where am I** ends with the time left at the current rate. The window's status bar shows the same time left, changing once a minute.
- **Say Status on a Places row** answers Alt+End, like every other list.

### Documents and conversion

- **HTML pages ask which theme to use.** Export, preview, and batch to HTML ask "Theme for the HTML page?", starting from your reading theme or your last answer; Escape cancels. `tw convert --theme NAME` picks one without asking. The page uses the theme's own colors.
- **Add a folder to the library** is a command in the window and in the terminal reader.
- **Clear the reading statistics** from the reader, not only from the command line.

### Audio, video and captions

- **The read-along page.** Export audio can write one HTML file that holds the text, the audio, and word-by-word highlighting. It works offline, has named buttons, and uses no live region.
- **Ogg Vorbis.** Export audio writes `.ogg` as Vorbis (libvorbis, built in). `.opus` stays Ogg Opus.
- **Karaoke video.** With ffmpeg on the PATH, Export audio writes an MP4 with the spoken word in bold and underlined, a caption track, and chapters. The format is hidden when ffmpeg is missing.
- **Richer captions.** WebVTT gains karaoke tags, word lines, and a NOTE block that names the voice and rate. ASS gains karaoke by outline. Chapter files and localized chapter names are written. `tw export-audio --karaoke --chapters` and the Export settings choose them.
- **Captions from dictation.** `tw dictate --captions` writes caption files from Whisper's segments, marked as machine captions.

### The window

- **The Reading settings dialog** gathers the 17 reading rows, with Voices and the WCAG and Generous spacing presets. It fits a 420 by 320 window.
- **A narrow layout.** Below 800 pixels the bars wrap and fold, key labels are hidden but kept, panels stack, and the status bar never overlaps. The View menu has Header and Toolbar commands.
- **The measure.** New setting `[display] measure` sets the line length (66 characters by default, 25 to 90, 0 fills the window).
- **Drawn structure.** List bullets, numbers, and nesting are drawn, as are the empty-window hint, an Editing badge with a dashed border, and a play-shaped mark for "reading from here". Text for screen readers is unchanged.
- **Marks and focus.** Every mark shape follows the design system's order, and the five mark colors come from the theme. Control borders reach 3 to 1, and buttons and the document get a double focus ring. Highlight granularity, lead words, and the on-off setting apply in the window.
- **Interface text follows** the Windows and GNOME text scale. The RSVP panel follows the font size and the reading font.
- **First run** has at most three skippable steps and infers hybrid mode when a screen reader is running. "Ask again about first-run choices" is in the Help menu.
- **Two modes, named.** "textweaver reads aloud" and "my screen reader reads", in the View menu and Settings, with a "Speak textweaver's messages" choice.
- **Keys.** Alt+Shift+A, F3 and Shift+F3 for Find next and previous, and Alt+Shift+S for Speech Cursor.
- **Dialogs** share one style with a Close button in every kind. Settings filters as you type, F1 gives the long help, and a recent block comes first. Themes show "meets AA" or "below AA" in words, and Lamplight is listed as a soft dark theme.
- **Trouble is kept.** The window writes a log file, saves your place after a failure, and says in words when graphics cannot start. It uses a low-power graphics adapter, remembers its place and size on this computer, and follows the system's caret blink.
- **A lettermark icon,** and the logo in the documents and the site.

### The terminal reader

- **Quiet screen is on by default in hybrid mode** (`QuietScreen` auto, on, or off).
- **Sticky status context.** An edge message keeps the item it belongs to, and say-all keeps the sentence first.
- **Quieter and leaner.** List hints, a terminal title that is the document's name, one terminal probe at start, and a leaner redraw. The ruler mask is a blended color.
- **The title line keeps "modified" inside 40 cells** at line numbers of three digits.
- **The reading ruler shows its edges** in High Contrast.

### The command line

- **One rule set.** Errors take one line and end with a next step. Exit codes are 0 for success, 1 for a failure, and 2 for wrong use. `--out`, `--to`, `--lang`, `--json`, `--home`, and `-y` mean the same everywhere, and old spellings still work as hidden aliases. Questions go to standard error.
- **Verbs as subcommands** in `library`, `stats`, and `dictate`. `tw sync stop` stops syncing. `tw speak -` reads standard input. `tw cite check` exits 1 when a citation is bad. `tw settings path` lists every folder. `tw open` and the read-only `outline`, `notes`, `highlights`, and `info` are in JSON-RPC.
- **One standard output writer** that ends quietly when a pipe closes. One components status line, and the doctor reports the window, components, and OCR.
- **Messages for window users** no longer name `tw`; they name the place in the app.

### Languages

- **Fewer, clearer messages.** Eleven duplicate message groups are merged, Settings help is one short sentence with F1 for the rest, and the guides say "cursor", "speech engine", and "the window" throughout.
- **US spelling** in messages ("Canceled"). The new English text waits for a native-speaker review of the five translations.

### Settings and data

- **Settings written by another program are kept** when textweaver saves. Saves happen at most once a second, and identical writes are skipped.
- **Library folders** are typed with semicolons, so a name such as "Readings, Fall 2026" keeps its comma.
- **A declined font download** is not asked again in the same session.
- **Settings from alpha.1 to alpha.8 still load,** checked against kept sample files.

### Documentation

- **Start pages** for students and staff, a known-limits page, a privacy page, an accessibility statement, and a what's new page. The quick start and install start with the window.
- **The command line page** states the rules, and a test checks each one against `tw --help`.
- **Honest statuses** in the roadmap and the star comparison. The roadmap's "Planned" is now "Not in the 0.1 series".
- **The Help menu** reaches the quick start, the documentation, and Report a problem (which asks before a browser opens). About lists the facts a report needs.

### Packages and the build

- **Packages:** the window's zip is unchanged. Ogg Vorbis adds libvorbis (BSD-3-Clause), built from source.
- **Tests never reach a real speech engine.** They choose from test doubles, and the development check runs eSpeak NG silent.
- **The release tool refuses** a changelog that has the same heading twice.

### For contributors

- **Checklist tests** cover every list and dialog (named, modal, labeled) and prompts and the position line in 40 Braille cells. `docs/dev/checklist-tests.md` maps each checklist row to its test.
- **Comments** in the message catalogs no longer name waves or agents.

### Package sizes

- `textweaver-0.1.0-alpha.9-linux-aarch64-gui.AppImage`: 37.0 MB (38,816,264 bytes)
- `textweaver-0.1.0-alpha.9-linux-aarch64-gui.tar.gz`: 38.8 MB (40,664,840 bytes)
- `textweaver-0.1.0-alpha.9-linux-aarch64.AppImage`: 56.3 MB (59,054,600 bytes)
- `textweaver-0.1.0-alpha.9-linux-aarch64.tar.gz`: 59.6 MB (62,492,875 bytes)
- `textweaver-0.1.0-alpha.9-linux-x86_64-gui.AppImage`: 39.7 MB (41,589,240 bytes)
- `textweaver-0.1.0-alpha.9-linux-x86_64-gui.tar.gz`: 41.6 MB (43,570,381 bytes)
- `textweaver-0.1.0-alpha.9-linux-x86_64.AppImage`: 60.6 MB (63,531,512 bytes)
- `textweaver-0.1.0-alpha.9-linux-x86_64.tar.gz`: 64.2 MB (67,290,407 bytes)
- `textweaver-0.1.0-alpha.9-macos-universal-gui.zip`: 63.7 MB (66,806,013 bytes)
- `textweaver-0.1.0-alpha.9-macos-universal.tar.gz`: 109.9 MB (115,282,022 bytes)
- `textweaver-0.1.0-alpha.9-windows-x86_64-gui.zip`: 42.4 MB (44,471,409 bytes)
- `textweaver-0.1.0-alpha.9-windows-x86_64.zip`: 67.7 MB (70,986,837 bytes)

## [0.1.0-alpha.8] - 2026-10-03

This release is about speed you can measure, hearing where things end, and text from health sciences read the way a clinician would say it. Reading pauses after headings, paragraphs, and list items. Medical and scientific text, identifiers, and units are said correctly. The window gains Contents and Notes panels, F6 between its regions, and system file and folder choosers. Dictation and other models are offered as downloads. Conversions leave a report for an accommodation file. Piper answers a Stop and restart much sooner, and large documents open and edit faster. The translations of the new messages wait for a native speaker's review.

### Security

- **Links open without a shell.** On Windows a link's address was opened through `cmd /C start`, so an address with `&` in it could run a command after one "y". Links and files now open through `ShellExecuteW` on Windows, and `open` or `xdg-open` with the address as one argument on macOS and Linux. Only `http`, `https`, and `mailto` addresses and files that exist are opened; anything else (`ms-msdt:`, `javascript:`) is refused, and you hear why: "Not opened: ms-msdt link blocked."

### The terminal reader and Braille

- **Status text is never cut.** The status area grows to as many rows as its text needs, counted the way it is word-wrapped. Before, it stopped at three rows, so a long line or paragraph put there in screen-reader mode lost its end before NVDA, JAWS or the Braille display saw it, and a message near a multiple of the width could lose its last word. A long line put there by a caret move is cut at 600 characters, as text read in place already was.
- **A still screen in hybrid and screen-reader modes.** The status area keeps a fixed height there, enough for 600 characters and at most half the screen, so the document no longer moves up and down with each message.
- **Title parts in a Braille order.** In edit and Speech Cursor modes the mode and "modified" come right after the position: "Line 12 of 400, 3%, Edit, modified, Ready", so they are inside 40 cells. Say Status and the window's position label follow. In hybrid and screen-reader modes the title line ends with the document's title alone, without "textweaver:".
- **The keyboard shortcuts list leads with the command.** Each line reads "Play or pause: Alt+P or Space. Play or pause reading from the current word. Reading": the name, its keys, what it does, then its category. The keys are inside 40 cells, and a first-letter jump goes to a command, not a category. Both readers show the list.
- **Cell 1 holds text in hybrid and screen-reader modes.** Line numbers start at the left edge ("12  Text"), the reading ruler marks its line with underline and bold without a mark column, and the hints and the empty screen start at the edge.
- **New setting: the key hints line** (`[display] hints`, `"auto"` by default; stays on this computer). Automatic shows it when textweaver speaks for itself and hides it in hybrid and screen-reader modes; `"on"` and `"off"` choose. See [Braille-first layout](docs/reading.md#braille-first-layout).

### The GUI

- **Contents and Notes panels.** Ctrl+1 shows the document's headings, and Ctrl+2 its notes, in a panel beside the document, from the same lists as the outline (Alt+O) and the notes list. Enter goes to a heading or note and stays in the panel, Shift+Enter goes and returns to the document, and Escape returns. The row at the caret is marked with a bar and ", current". The panel is a navigation landmark, never takes the focus unless its key is pressed, and costs nothing while closed. The window remembers it (`[gui] sidebar`).
- **F6 moves between the window's regions.** F6 and Shift+F6 move between the header, the panel, the document, and the toolbar, as in other Windows programs. F6 used to turn line numbers on or off, which the window does not draw; it still does in the terminal reader.
- **The title bar and menus follow the theme.** A dark theme such as Galaxy now gets a dark title bar, and on Windows a dark menu bar and dark drop-down menus, even when Windows itself is light; a light theme gets light ones. With Windows High Contrast on, Windows draws them in your contrast colors. See [Colors and high contrast](docs/gui.md#colors-and-high-contrast).
- **New setting: Hide the menu bar** (`[gui] auto_hide_menu`, off by default, Windows only). The menu bar takes no room until Alt, F10, or Alt with a menu's letter shows it, and hides again when the menu closes. NVDA and JAWS still say "menu bar" on Alt and F10. See [Menus](docs/gui.md#menus).
- **Enter in the command palette runs the command you heard.** After Up or Down moved through the matches, Enter ran the first match instead of the one announced. It now runs the selected one.
- **Shift+F1 describes the window's own commands.** "What does this key do?" followed by Ctrl+D, Ctrl+Plus, or Ctrl+, opened the font list, changed the size, or opened the settings. Now, as for every other key, you hear its name, what it does, its keys, and where it is in the menus.
- **Shift+Y obeys single-key shortcuts.** With single-key shortcuts off (F9), Shift+Y no longer lists the highlights in the window, as in the terminal reader.
- **A clearer message when the document on the command line cannot be opened,** the same as Ctrl+O gives, with a next step: "Could not open notes.md: there is no file named notes.md in Documents. Check the name."
- **Typed text in a prompt follows the theme.** The command palette's filter and every typed prompt drew their text in a near-white whatever the theme, so it was faint in Galaxy Light; it now uses the theme's text color, and selected text sits on a band that keeps it readable.
- **Play's focus ring shows.** On Play's colored fill the focus ring is now drawn in the page color, at least 3 to 1 against the fill in the four main themes.
- **Faster menus.** A setting no menu shows, such as the rate, no longer rebuilds the menu bar, and a rebuild no longer works out the settings and the menus' reserved letters again.
- **Closing the window no longer loses unsaved edits.** The close button and Alt+F4 closed at once, so edits made since the last save were lost. With unsaved edits the window now asks save, discard, or cancel, as Ctrl+Q does; Cancel keeps it open. See [Closing the window](docs/gui.md#closing-the-window).
- **Questions that delete, remove, or replace start on No.** Enter in "Replace it?", "Remove the voice?", or "Delete this note?" now keeps things, and the first button says Delete, Remove, or Replace instead of Yes. Y and N answer as before.
- **The caret reaches your screen reader before the screen is drawn.** The window sent the accessibility update only after the graphics card had drawn and shown the frame, so NVDA, JAWS and the Braille display waited for it. The update now goes first.
- **Faster start.** The system's light or dark setting is read while the rest of the window starts, and not at all when `--theme` is given or you picked a theme, as in the terminal reader.
- **The command palette lists only commands the window runs.** Scroll down, Scroll up, and Line numbers, which only the terminal reader has, are no longer listed in the window, and their keys (`j`, Shift+J) say "This command works in the terminal reader." instead of doing nothing.
- **The window behind a dialog stays readable.** Buttons and text behind an open dialog took Masonry's fixed light gray, about 2 to 1 on Galaxy Light's buttons; they now take the theme's disabled color, at least 3 to 1 on buttons and panels (Windows' own disabled color in high contrast). Play no longer keeps its accent fill there.
- **The reading ruler's band shows by its shape.** The rows around the reading line get a thin bar at their start, half the reading line's, as the terminal draws them, and a blank line's row no longer reaches over the heading after it.
- **Settings: switches say on or off, and the arrows work.** A switch shows its value in words beside it, so it never rests on the switch's color. A click on the left arrow beside a number or a choice steps it back, and on the right arrow forward; before, either arrow stepped a choice forward or opened a number for typing.
- **`--screenshot` in a build without screenshots ends with an error.** `--screenshot`, `--review-screenshots`, and `--measure-frames` opened an ordinary window in a build without the `screenshot` feature; they now say the option is not in the build and open nothing.
- **The spoken sentence is underlined and the spoken word bold.** The window drew both as color bands only, though every theme asks for an underlined sentence and a bold word, as the terminal draws them. Now the sentence has a line under it in every theme, and under a Windows contrast theme, where it had no mark at all, the line is drawn in your text color. The word is bold without moving anything on the line. Notes, highlights, and search matches inside the sentence being read no longer disappear under its band.
- **Two new theme checks.** The spoken word's band must reach 3 to 1 against the page and against the sentence's band. Every built-in theme passes; a theme of your own that falls short still loads, and you are told what to fix.
- **The keyboard preset applies from the start.** `preset = "classic"` under `[keyboard]` now takes effect when the window opens, as in the terminal reader; before, the GUI used the default keys until a settings import or sync.
- **The voice manager's buttons and keys.** Enter or Space presses the focused button, and the Help and Say Status keys (Alt+End) work with the focus on a button, as in the list. Remove is unavailable on a voice that cannot be removed: it says "Remove, unavailable", your screen reader says "unavailable", and its description gives the reason.
- **The document's name on focus.** The document is named with its title first, such as "Reading check, document", so NVDA and JAWS say which document it is when the window or the document takes the focus.

### The library and export

- **A cleared detail shows at once.** Clearing a title, author, DOI, or ISBN you typed shows the document's own value in the library list right away, without reading the list again.
- **Export uses the reading font.** Export to PDF or EPUB in the reader uses your reading font, a downloaded Lexend too, as `tw convert --font` does. A font that is not on this computer keeps the usual font.
- **Conversion reports with provenance.** `tw convert`, batch convert, and export from the reader leave a report for an accommodation file: each source's name and SHA-256, textweaver's version, the date and time from the clock, the output format, and what could not be made accessible (images without descriptions, tables whose columns do not line up or that have no header row, math that did not parse), each with the heading, page, and line where it is. A batch writes `conversion-report.md` with a section per file; a file on its own gets `essay.pdf.report.md` beside its output. `--report-format json` writes JSON, and `--no-report` writes none. The report names files only by their name inside the converted folder. textweaver always says where the report is, and how many files have items not made accessible. It replaces `conversion-report.txt`.
- **Stop means stop in audio export.** A stop asked for while the audio is being encoded (FLAC, MP3, Opus, or through ffmpeg, which is ended) or just as the last sentence finishes now ends as "stopped" with no file left, never as written.
- **WebVTT captions escape their text.** `<`, `&`, and `>` in caption text are written as `&lt;`, `&amp;`, and `&gt;` in WebVTT, so a cue with "x < y" or "-->" no longer breaks players. SRT is unchanged.

### Choosing files and folders

- **The system's file chooser for every file.** In the GUI, Save as (in the document's folder, offering its name), Insert image, Import references, and profile import and export now open the system's own Open or Save dialog, titled for what they do and listing only the files they read, as Open and the settings import and export already did. No more typing a full path.
- **The system's folder chooser.** Audio export's "Another folder", the batch conversion folders, and the sync folder open the system's folder chooser in the GUI, starting in the document's folder. If it cannot open, the file browser's list chooses the folder instead.
- **F4 browses from a prompt.** In the terminal reader, and in a typed prompt in the GUI, F4 in any prompt for a file opens the file browser; the path chosen fills the prompt, and Enter confirms it. For a file to write, F4 chooses the folder and keeps the name. Escape goes back to the prompt as it was. The prompt names the key when it opens ("Import settings from file. F4 to browse.").
- Tab completes a path in every prompt for a file, not only Open, Save as and Insert image.

### Themes

- **Following the system theme again.** One F5 used to stop the theme from following the system for good. Turning "Follow the system theme" off and on again now follows it from the next start.

### Words

- **The first-run welcome names the five keys you need first, in order:** "Welcome to textweaver. Ctrl+O opens a document. Space starts and pauses reading, and Escape stops. F2 lists every command. F1 opens the help." It comes before the hint that no document is open, in the window and the terminal reader, and the quick start lists the same five keys. Help (F1) now has the command palette and the list of every key as its second and third lines.
- **Settings read grammatically:** "1 word", "1 row", "2 words", where they said "1 words" and "1 rows". Settings that are a multiple of the font size (line height and the other spacings) and the highlight speed show the number alone, and their help says what it multiplies.
- **Questions in the window are named by the question alone,** "Quit textweaver?", since the Yes and No buttons show the keys; the status line and the terminal reader keep "y or n". Every question ends "y or n", lower case.
- **Reset all colors asks first:** "Reset every color to the theme's? y or n". No keeps every color, and the Colors dialog opens again where it was.
- **Errors say what to do next.** Ten common failures gained a next step, such as "Could not save: ... Still editing. Try Save as." and "Could not scan the library: ... Check the library folders in Settings." When "Open it?" is asked again after a stray key, the whole question is asked, so you still know which file it means.
- **The window's buttons:** Edit is now "Start editing" (its pair is "Finish editing"), Commands shows its key as F2 instead of ":", and each button's description is a short line of its own, such as "Run any command by its name.", instead of the command's full help. Stop, Slower, Faster, and the sentence buttons have no description, since their names say it.
- **The color helps say "Choose a name, or type a hex code"** instead of "#rrggbb", which a screen reader reads as "number sign r r g g b b". The five mark colors say where they are used today: the terminal reader uses three, and the window none yet.
- **One word for each thing:** "cursor", not "caret"; "speech engine", not "backend"; "window", not "GUI"; "version", not "build"; "words per minute"; the RSVP position "center". Messages that sent you to "Choose Voice" name the Voices command, and "Live preview on" names Reload preview automatically.
- **Grammar:** "Table row removed." and "Horizontal rule removed." instead of "Table row added removed.", and "Could not export to PDF: ..." instead of "Exported to PDF failed: ...".
- **The two test languages** (accented English and right-to-left test text) are no longer offered in the Interface language list. They can still be typed in `settings.toml`.
- The Settings command's help no longer promises a filter the window's dialog does not have.
- The new English is drafted in the five other languages too; those drafts wait for a native-speaker review.

### Documentation

- **Window problems in Troubleshooting:** the window does not open or is blank, your screen reader does not read the window's messages, and the file chooser does not open.
- **The guides match the code:** Speech Cursor in the window, self-voicing echo with `--self-voicing`, which keys ask a question again, the window's font list and size keys (no Fonts dialog), no spacing presets or warnings, Save As asking first, the export message, the font size and RSVP ranges of the Settings dialog, the settings screen's filter and recent settings as the terminal reader's, HTML's own stylesheet, and the voices key in each program.
- **The documentation site** keeps text to about 75 characters a line, with normal letter spacing.
- US spelling throughout the English messages and the documentation ("color", "license", "center"), checked by `cargo xtask docs --check`.
- The window is named `textweaver-gui` in the guides, the quick start gives the window's keys beside the terminal reader's, and README and CONTRIBUTING start with the window and say that the build needs cmake.
- Dates and internal names are gone from the user guides and the keyboard reference's "What changed": they name the release instead.

### Optional components and the dictation model

- **Dictation offers to download its model.** The first Dictate without the Whisper model asks "Dictation needs the Whisper model, 79.3 MB, license MIT, unconfirmed. Download it now? y or n". A yes downloads it, with progress said every 10 percent and Escape to stop, then dictation starts, with no restart. A no is remembered until textweaver starts again. Tools, Download the dictation model asks the same at any time, and `tw dictate download [--yes]` does it from the command line; `tw dictate` offers it when the model is missing.
- **The real reason, in words,** when the model cannot be used: a missing file, a damaged file (the download is offered again), or a folder that does not exist, instead of one general message.
- **The Dictation model setting** chooses Whisper base.en (the default, 79.3 MB) or small.en (251 MB, more accurate).
- **Manage optional components** (Tools menu): every model, font, and voice textweaver can download, with whether it is installed, its size, its license, and what needs it, and Download, Verify, Remove, and Install from a zip file or a folder. `tw components list | download | verify | remove | install` does the same, and `tw info` ends with how many are installed.
- **A first-run list** of the optional components in the window and the terminal reader, shown once, with nothing chosen: Space chooses, Download the chosen ones gets them, Escape skips.
- **Install from a file** for computers without internet: a downloaded zip or a folder, each file checked against the same pins; anything else is refused with the reason.
- **A mirror** (`[components] mirror` or `TEXTWEAVER_COMPONENTS_MIRROR`), tried before the public sources, may list components of its own; it can add components but never change a built-in one's pins. See [Optional components](docs/components.md).
- **Downloads go on where they stopped,** and a folder placed by hand that already holds the right files is used as it is.

### Speech: pauses at headings, paragraphs and list items

- **Hear where a heading, a paragraph or a list item ends.** Reading now leaves a short silence after each one: 400 ms after a heading, 300 after a paragraph and 150 after a list item, shorter in proportion above 265 words per minute and never under a quarter. No words are added.
- **New settings:** "Pause after headings", "Pause after paragraphs" and "Pause after list items" (`[speech] pause_heading_ms`, `pause_paragraph_ms`, `pause_list_item_ms`, 0 to 3000; 0 turns one off). They sync with your other settings.
- **Every engine.** With Eloquence, Windows voices, DECtalk and Piper the pause is silence that textweaver plays between two sentences, and the next sentence is still prepared while the first is spoken, so speech starts no later. Apple voices, Speech Dispatcher and eSpeak NG get the next sentence once the pause has passed.
- **Stop, Pause and skipping are never held up.** Stop ends a pause at once, Pause holds it, and skipping goes straight to the next sentence. The highlight stays on the last word during the pause and moves as soon as the next word sounds. See [Pauses at headings, paragraphs and list items](docs/speech.md#pauses-at-headings-paragraphs-and-list-items).

### Speech: clinical and scientific text

- **Identifiers are read digit by digit.** A number after CPT, PMID, NCT, ZIP, DOI, ISBN, or phone (also telephone, tel, and fax), and a US phone number such as "503-555-0123", is read as digits: "PMID 31769816" is "PMID three one seven six nine eight one six", where it was "thirty-one million ...". The highlight follows each digit.
- **Error-prone medical abbreviations are spelled, never expanded.** The abbreviations on The Joint Commission's "Do Not Use" list and the ISMP list of error-prone abbreviations (QD, QOD, U, IU, MS, MgSO4, cc, SC, SQ, HS, the ear and eye abbreviations, TPA, HCTZ, and others) are read letter by letter, "Q D", and no expansion, built in or your own, can replace them. "µg" is read "micrograms".
- **Symbols outside math are named:** Greek letters ("TNF-α" is "TNF alpha"), the micro sign, the minus sign, "⇌" ("in equilibrium with"), arrows, and powers of ten ("× 10^9" is "times ten to the ninth"). Greek letters are named at every punctuation level.
- **With Eloquence too.** The identifier rule and the error-prone list also apply to engines that read numbers and abbreviations themselves.
- **New: a medical lexicon** (`[normalization.medical_lexicon]`, off by default). Drug names, clinical terms, and eponyms are read from a built-in list of respellings, such as "warfarin" as "WAR-fuh-rin" and "Guillain-Barré" as "ghee-YAN bah-RAY", and safe dosing abbreviations are said in words: "q6h PRN" is "every 6 hours as needed". Your own file, `medical-lexicon.toml` in the settings folder or the file `overlay` names, wins over the built-in entries. See [The medical lexicon](docs/speech.md#the-medical-lexicon).
- **Tall Man drug names stay whole.** Split caps no longer cuts "hydrOXYzine" into "hydr OX Yzine" or "DOPamine" into "DO Pamine".
- **Units are said in full.** "5 mg" is "5 milligrams", and mcg, mL, L, mmol, mEq, kg, °C, and others the same way; a slash after the unit is "per" ("2 mg/kg/day" is "2 milligrams per kilogram per day", "10^9/L" is "ten to the ninth per liter"), and "120/80 mmHg" is "120 over 80 millimeters of mercury".
- **Ranges and decimal commas.** An en dash between numbers is "to" ("6–8 weeks" is "6 to 8 weeks"), and a decimal comma before a unit or a percent sign is a decimal point ("2,5%" is "two point five percent", where it was "2,five percent"). "1,000%" is "one thousand percent".
- **24-hour times gain no AM or PM.** "08:05" is "eight oh five" and "15:30" is "fifteen thirty", where they were "eight oh five AM" and "three thirty PM". A time such as "3:45" reads as before.

### Fixed: saving, and the programs textweaver starts

- **Saves wait for a file another program holds.** On Windows, the settings, the library, the reading places, the citation library, conversion output, the full-text index, the document cache and the reading fonts now wait briefly (about 0.4 s in all) and try again when another program, such as OneDrive, Obsidian, an antivirus scanner or the search indexer, has the file open, as saving a document already did. Before, they failed at once. Every save is also synced to disk before it replaces the old file, and two saves at the same moment can no longer share a temporary file.
- **No console window flashes up from the window.** Converting audio with ffmpeg (audio export, and dictation of a file that is not WAV), contracted Braille through liblouis, and reading the Windows color setting no longer open a console window for a moment, which a screen reader announced as a focus change.
- **Every feature finds a program the same way.** ffmpeg, Whisper, Piper, Tesseract, Omnivox and Speech Dispatcher are looked up on `PATH` by one rule; on Windows that is each `PATHEXT` extension (`.exe`, `.cmd`, ...) and then the bare name, so a program one feature finds is found by all of them.
- **Messages from other programs keep their accents.** An error message from ffmpeg, Whisper or liblouis that is not UTF-8 (an accented user name in a path, written in the Windows code page) is read as Windows-1252 instead of showing replacement marks.

### Faster

- **Opening an EPUB or DAISY book with many headings.** Each section's name was found by walking the book's text from its start once per heading; it is now one pass.
- **The window lays out and draws each paragraph with its own styles.** Before, laying out or painting a paragraph looked at every styled span in the window (every link, bold word and code span); each paragraph now keeps the spans that touch it, built once when the text changes.
- **Text that needs no change costs nothing to prepare.** Each preparation step (abbreviations, numbers, punctuation, split caps, the lexicons) now leaves a sentence it does not change without copying it or building anything, and the error-prone abbreviation check looks at a word's capture groups only when the word is on the list. Most sentences pass most steps unchanged, so preparing text for speech allocates far less memory.
- **Speech starts and stops sooner.** The sound output now asks the audio device for a 30 ms buffer instead of leaving the size to the audio library (about 43 ms). After a stop, a restart, a skip, or a rate change, the first word is heard about 12 ms sooner, and a stop goes quiet sooner by the same amount, with every engine whose audio textweaver plays (SAPI 5, Eloquence, DECtalk, and Piper). The highlight runs closer to the sound for the same reason. A device that refuses the size gets the library's own.
- **No stale sound after a stop, no wait before a restart.** The few milliseconds of speech the output had already taken are dropped at a stop instead of played, and new speech that arrives while the output is playing silence starts at the next sample instead of after the rest of that silence.
- **Piper voices answer a Stop and restart much sooner.** From Stop and Read from cursor to the first sound, Piper took 440 ms at the median on the 1 MB test document on the owner's machine (830 to 980 ms under a heavier load); it now takes 190 ms, and about 120 ms when the reading restarts at the next sentence. Three changes do it. Piper now works only as far ahead as it needs, so a Stop rarely finds it in the middle of a sentence it cannot cut short. A reading starts with a short phrase (the first words up to a natural break, such as before "and" or "in"), so the first sound comes after a short synthesis; once enough audio is queued, it goes back to whole sentences, for the best intonation. And the pronunciations of what comes next are worked out ahead and remembered, which matters because the built-in eSpeak phonemizer takes about 90 ms each time, however short the text. Word timing and the highlight are unchanged, and a long reading still plays without gaps.
- **The window keeps up on one very long line.** A document that is one long paragraph, such as a text file with no line breaks, made the GUI rebuild and resend every text run of the paragraph for each spoken word, and draw every line of it, on screen or not: about 37 ms a word on a 1 MB one-line file, and 180 ms with the reading aids on. Now a word costs only the lines it leaves and reaches, and only the lines on screen are drawn, with their syllable marks and ruler rows: under 1 ms a word plain and about 1.3 ms with every aid on, 3 text runs sent to the screen reader instead of 1,442. Ordinary documents send about half as many runs per word too.
- **The highlight stays on the word past 64 KB of one line.** In a paragraph longer than 64 KB in one style, the text layout lost track of where its characters were, so the word band and the caret could be drawn at the wrong place. Fixed in the GUI's copy of the Parley text library.
- **On Linux, an output that runs dry grows its buffer.** When the audio system reports underruns (ALSA and JACK do), the output opens again between readings with twice the buffer, up to 100 ms, so speech never stutters on a slow or busy machine. Nothing is cut off by the reopen.
- **Math with very long names opens in moments.** A formula with a very long run of letters, in a MathML file, an EPUB, or a Word document's equations, took time that grew with the square of its length: a 512 KB run inside deeply nested math took over three minutes to open. It now takes time in step with its length: that formula opens in a quarter of a second.
- **Reading starts with less work.** Planning what to say for a whole 10 MB document makes a third fewer allocations (836,038 to 538,984) and uses 11 MB less memory; the text is no longer copied out of the document piece by piece.
- **Typing in a large document.** Each edit to a 10 MB document cost about 22 ms when the paragraph table was needed again; it now costs about 1 ms. Markers are shifted only where the edit reaches, and the table of blank lines is updated instead of rebuilt.
- **The first Read after opening** no longer builds the paragraph and marker tables on the input thread: opening builds them while it loads (25 ms or more on 10 MB).
- **SSML and DECtalk voices** no longer compile their markup patterns for every sentence (about 0.3 ms a sentence, on the speech thread).
- **Starting in another language** parses only that language's messages, not all five translations (41 to 6 ms in Spanish).
- **Loading** sizes the text once instead of growing it.

### For contributors

- **Gates that can fail**: each CI gate was broken once on purpose and checked to go red; `docs/dev/testing.md`, "Gates and what breaks them", lists each break and its result. The lean reader (`textweaver-tui --no-default-features`) had no CI job at all and is now built on Linux.
- **Report-only steps say why**: every `continue-on-error` step has a comment with its reason and a review point (the alpha.9 readiness check), and an xtask test fails on one without. Nine became standing: building accessibility-cli, the accessibility tree comparison (a broken comparison fails; a changed tree still never does), and the tree upload, on each system. Eleven stay report-only, each with its reason.
- **Skips say so, and fail where the tool should exist**: a test whose tool is missing prints "SKIPPED, not checked", and fails instead where CI names the tool in `TEXTWEAVER_REQUIRE_TOOLS` (ffmpeg on Linux CI, pandoc in the nightly Docker job, `lou_translate` in the braille job). `cargo xtask regen --check --require-all`, now run by CI's docs job, fails a step whose tool is missing. The braille job also fails when its test filter matches no test.
- **No test touches the real data folders**: `cargo xtask sentinel record` and `check` list textweaver's platform folders, and an empty `TEXTWEAVER_HOME`, around CI's test step, and fail on any file a test added, changed, or removed there.
- **Release checks**: each package job checks that its build folder holds exactly its files, and the checksums job that the published release holds exactly the 17 expected assets (`cargo xtask release-assets`). The terminal packages are smoke-tested beyond `--version` (open a fixture, `tw info`, `tw components list`, convert one file; `tools/package-smoke.sh`), and the Linux tarball and AppImage again in a clean Debian container with no network, as an unprivileged user (`tools/clean-install-smoke.sh`). The GUI checks are unchanged.
- **Ace by DAISY in the second-tool checks**: every fixture EPUB is checked with Ace 1.4.6 (`@daisy/ace-cli`, locked in `tools/ace/package-lock.json`, run with no install scripts in the runner's Chrome); a critical or serious violation fails unless `tools/second_tool_allowlist.txt` names it. A converted file with no report from epubcheck, Ace, or veraPDF now fails too.
- **A message style guide,** [docs/dev/messages.md](docs/dev/messages.md): twelve rules for everything textweaver says and shows, from meaning first and errors with a next step to one word for each thing and the 40-cell rule. Style tests in the lexicon crate check the English catalog; where English still breaks a rule, the test names the ids, and those lists may only shrink.
- **Three more docs checks** in `cargo xtask docs --check`: the user guides name releases, not waves or dates; the theme count in the documentation index matches the theme files; and every image has alternative text and a "Description:" line beside it.
- **The settings reference** reads units grammatically and no longer prints "Choices: ." for the theme.
- **One atomic write and one process module in `textweaver-core`**: `textweaver_core::fs::write_atomic` (unique synced temp file, rename retried while Windows reports the file in use, permissions kept on request) replaces seven copies in store, editor, convert, formats (two), cite and fonts. `textweaver_core::process` has `find_program` (one `PATHEXT` rule), `command` and `hide_window` (`CREATE_NO_WINDOW`), `run_with_timeout`, and `decode_output` (UTF-8 when valid, else Windows-1252 on Windows); the PATH lookups in export, dictation, speech, piper and ocr and the spawns in export, dictation, writers, theme and a11y use it. fonts, ocr and theme now depend on core.
- **Dead code removed**: 17 public functions and constants no code called (R4's list; Apple's `last_word_offsets` is kept until it can be built on macOS).
- **`cargo doc -p textweaver-formats` builds without features**: the link to `Registry::with_pandoc` falls back to `Registry` when the `pandoc` feature is off.
- **The ADR index names the unused numbers** 0038, 0040 and 0047, and what each was reserved for.
- **A size budget for the packages**: `cargo xtask dist`, `gui-dist`, and `appimage` print each package's size against the last release's, recorded in `xtask/package-sizes.toml`, and fail when one grew more than 10 percent without a note under `[notes]` saying why. After a release, `cargo xtask release VERSION --sizes` records the published sizes, clears the notes, and adds a "Package sizes" list to the release notes. The file starts with alpha.7's sizes.
- **The reader is packaged without `tw`'s features**: `cargo xtask dist` builds `textweaver` and `tw` in separate cargo runs, so the reader no longer gets `tw`'s features through feature unification (opening a web address, the OCR model download). It saved 43 KB: the reader keeps an HTTP client of its own for citation lookups. `cargo xtask gui-dist --no-screenshot` builds the GUI package without its screenshot harness; the package keeps the harness for now, because the release workflow's GUI checks draw a `--screenshot` with it. [docs/dev/testing.md](docs/dev/testing.md#package-sizes-october-2-2026) has the sizes before and after.
- **One downloader**: the new `textweaver-components` crate pins files by size and SHA-256 and downloads them through a `.part` file, the check, and a rename, with resume, progress, cancel, a lock, and one neutral User-Agent. The OCR models, Lexend, and Piper voices moved onto it; a test in the app enforces the registry, and the fake fetcher records every request.
- **Research for the next waves.** `docs/dev/research/` holds the research reports and the wave plan for alpha.8, alpha.9, and later: a performance audit, speech engines and runtimes, text-to-speech use cases for students with disabilities, health sciences workflows, GUI and visual design, and law and standards. Nothing in it is a decision; decisions stay in the ADRs.
- **The bench gate is a two-way ratchet against a committed baseline,** `xtask/bench-baseline.json`, one entry per platform. Peak heap and allocation counts fail when they grow more than 25 percent, and also when they fall more than 25 percent without the baseline being updated, so the floor never goes stale; times fail when they grow more than 50 percent, against an entry measured on the gate's own runner type. A failed run never becomes the baseline: only `cargo xtask bench --update-baseline --reason TEXT` writes it, on main (or for a platform's first entry). See `docs/dev/testing.md`, "The gate: a two-way ratchet".
- **New bench measurements:** segmentation (`sentences_ms`, `words_ms`), normalization (`normalize_ms`), an edit on the loaded document (`apply_ms`, `blank_lines_ms`), stop to speak (`stop_to_speak`), and stop to first audio (`stop_to_first_audio`), stamped by the speech service when each reading first sounds (`SpeechService::first_audio`). `--engine piper` and `--engine sapi` time stop to first audio with a real engine on a silent output; Piper only with voices already installed, never downloading.
- **Pathological inputs:** one generated file per loader, with a 512 KB token, lists nested 200 deep, a 2,000-row table, and a 1 MB line, must load and plan within 10 seconds each in `cargo xtask bench --quick`.
- **`cargo xtask frames`** measures 200 moves of the spoken word through the GUI's driver and whole widget tree, with no window on screen (`textweaver-xilem --measure-frames MOVES`), on the 1 MB and one-line corpora, plain and with every reading aid on, at 100 and 200 percent: the median, 95th percentile, and worst move, and the allocations and accessibility nodes per move, in words. The counts are gated by the two-way ratchet against `xtask/frames-baseline.json`; it is the gate for the next wave's visual work.
- **GUI `--log`** writes the startup phases with their times, and every 200 highlight moves the driver's refresh time (the old highlight timing was a zero nothing read). The speech service's debug log says, every 200 scheduled words, how late they lit up and how far apart its timer steps were (for Windows's timer tick).
- **A nightly profile** of `tw info` and of the 10 MB narration plan, uploaded as flame graphs and as plain-text lists of the hottest functions; and the GUI frame times, reported in the nightly summary.

### Package sizes

- `textweaver-0.1.0-alpha.8-linux-aarch64-gui.AppImage`: 36.4 MB (38,136,328 bytes)
- `textweaver-0.1.0-alpha.8-linux-aarch64-gui.tar.gz`: 38.0 MB (39,869,137 bytes)
- `textweaver-0.1.0-alpha.8-linux-aarch64.AppImage`: 55.5 MB (58,206,728 bytes)
- `textweaver-0.1.0-alpha.8-linux-aarch64.tar.gz`: 58.8 MB (61,618,186 bytes)
- `textweaver-0.1.0-alpha.8-linux-x86_64-gui.AppImage`: 38.9 MB (40,835,576 bytes)
- `textweaver-0.1.0-alpha.8-linux-x86_64-gui.tar.gz`: 40.7 MB (42,714,711 bytes)
- `textweaver-0.1.0-alpha.8-linux-x86_64.AppImage`: 59.8 MB (62,687,736 bytes)
- `textweaver-0.1.0-alpha.8-linux-x86_64.tar.gz`: 63.3 MB (66,325,497 bytes)
- `textweaver-0.1.0-alpha.8-macos-universal-gui.zip`: 62.4 MB (65,411,740 bytes)
- `textweaver-0.1.0-alpha.8-macos-universal.tar.gz`: 108.2 MB (113,489,596 bytes)
- `textweaver-0.1.0-alpha.8-windows-x86_64-gui.zip`: 41.7 MB (43,735,774 bytes)
- `textweaver-0.1.0-alpha.8-windows-x86_64.zip`: 66.8 MB (70,080,418 bytes)

## [0.1.0-alpha.7] - 2026-10-01

### Keys: what changed

- **New key: Shift+F5 says how sync stands,** and names this computer and the others. It is the same in the terminal and the GUI, on every system, Mac included.
- **F2 in the library list** edits the focused document's details, as F2 renames in the bookmarks list.
- **Tab and Shift+Tab** move between the fields of the details form, in the terminal and the GUI, and say each field's label and value. JSON-RPC's `prompt_key` takes `shift_tab` for it.
- **The Say Status key previews a voice** in the voice manager: Alt+End (Option+End in the Mac GUI), as it previews the focused row in the file browser. If you changed the key in `keymap.toml`, the voice manager names yours.
- **The voice manager lists every engine:** Alt+V in the terminal, Ctrl+Shift+V in the GUI, Cmd+Shift+V in the Mac GUI.
- **The grammar keys work in every build:** Ctrl+F7 and Ctrl+Shift+F7 (Cmd+F7 and Cmd+Shift+F7 in the Mac GUI; Option+F7 and Option+Shift+F7 in the terminal on a Mac), and Alt+J for the fixes (Cmd+Option+J in the Mac GUI).
- **New commands without a key,** in the menus and the command palette: Continue reading and Edit details in the File menu; Set up sync, Sync now, Go to another computer's place, Replaced notes, and Stop syncing on this computer in Tools, Sync. Give any of them a key in `keymap.toml`.

### Sync between computers

- **Sync through a folder you choose** ([ADR-0049](docs/adr/0049-sync-beyond-the-place.md)): a folder kept in step by Syncthing, OneDrive, Dropbox, or iCloud, or a USB stick you carry. There is no account, no server, and no network code in textweaver; it only reads and writes files there. Documents are recognized on each computer by their contents, wherever they are on disk and whatever their names. The guide is [Syncing between computers](docs/sync.md).
- **What syncs,** each group with its own switch: reading places (one per computer), notes, highlights, bookmarks, reading statistics, settings, profiles, key overrides, the word list, the glossary and pronunciations, and favorite voices. Recent files, machine settings, caches, logs, and the documents themselves never sync.
- **Set it up from Tools, Sync** or the command palette: choose the folder in the file browser, name this computer (never its real name; at most 40 characters, so it fits a Braille line), and choose the groups. Sync status (Shift+F5), Sync now, Go to another computer's place, Replaced notes, and Stop syncing on this computer are beside it.
- **A place from another computer** is chosen at open by `[sync] position_policy`: `newest` (the default), `furthest`, or `ask` ("lab at 42 percent. Go there? Y or N"). You hear which computer it came from. Your place never moves on its own after that; a place that arrives while you read is only offered, once.
- **A note edited on two computers:** the newest edit wins, and you are told which note was replaced. The text that lost is kept on this computer, the last 20 per document, and Replaced notes puts one back.
- **Deletions are remembered.** Deleting a note, a highlight, or a bookmark leaves a small record, so a copy from another computer cannot bring it back. An edit made after the deletion still wins, and you hear so.
- **Settings and word lists:** the portable settings (rate, punctuation, verbosity, the reading aids, the theme and colors, the Braille and math codes, the interface language, and more) sync, the newest change to each winning. The engine, the voice, the volume, the access mode, the key preset, every path, and the sync settings stay on each computer. The [settings reference](docs/settings-reference.md) says for every setting whether it syncs.
- **Profiles keep their voice on each computer.** A profile syncs its portable settings only; the voice, the speech engine, and the access mode it was saved with stay on the computer that saved them, and an arriving profile keeps this computer's own. Which profile is in use never syncs.
- **Key overrides** are labeled with the system they were made on. Windows and Linux share theirs; a Mac's are kept but not used on Windows or Linux, and the reverse, and Sync status counts them.
- **Favorite voices** travel too. One not installed here is listed last in the voice manager as "not on this computer"; Enter only says so, and Space takes it off your favorites.
- **Nothing changes mid-sentence.** Sync messages, and settings taken in from another computer, wait while textweaver reads aloud, then come as one short summary at the pause: "Settings: 3 changes from laptop." They follow your interface announcement level.
- **A document that may be the same book:** two computers' copies with the same contents are joined at once. One that only shares a DOI or an ISBN is never matched on its own; textweaver asks first, and a no is remembered.
- **One document, one id.** When two computers opened the same book before they ever synced, the next time it opens the two ids are joined into one, with both computers' notes, highlights, bookmarks, and places.
- **Safe across a crash.** Changes taken in from another computer and not yet saved here are kept on this computer, so a crash or power cut in between never lets this computer's older version win.
- **Problems are said meaning first,** on a status line that starts with "Sync": the folder missing (a USB stick that is out; everything is saved here meanwhile), a folder written by a newer textweaver (read only), another computer's clock more than a day ahead, a damaged file skipped, or a folder that cannot be written. The troubleshooting guide covers the missing folder and the newer format.
- **Before the first merge,** this computer's reading state is copied once to `state-before-sync`, so turning sync on can be undone.
- **Privacy:** no name, path, or computer name is written to the sync folder. Documents and computers are known by random ids and the names you chose; a title travels only when the document states one, never one made from its file name, and an author only when it is not this computer's user or computer name. The folder is not encrypted.
- **On the command line:** `tw sync setup`, `tw sync status`, and `tw sync now`, each with `--json`.
- **The older setting moves on its own.** `[reading] sync_conflict_policy` is now `[sync] position_policy`; `highest_progress` becomes `furthest` and `manual` becomes `ask`.
- **The old progress file is still read.** With sync on, places go to the sync folder, and a library folder's `.textweaver/progress.json` (from an older textweaver, or converted from Star) is only read, so its places are still honored.
- **Values from a newer textweaver pass through exactly.** A number in a sync file that this version does not know about, however large, is written back as it was read; a very large one could change in its last digit.
- **An older group file reads as this format,** as a document's record does, so merging it never changes the format number it is written with.
- New messages for sync, in all six languages; the translations need a native speaker's review.

### State files

- **State files are now format 2.** Every older file, and every Star import, loads unchanged.
- **Bookmarks have ids.** Every bookmark gets a stable id, so bookmarks from two computers are matched by id, not by name. Existing bookmarks get one when their file is read, and keep their names and places. Two bookmarks that arrive with the same name are renamed apart.
- **Longer ids.** New notes and highlights get 64-bit ids; older ids stay as they are.

### The library

- **Continue reading.** A new command, in the File menu under Library and in the command palette, lists the documents on this computer with a reading place from any computer, newest first: "Cells, 42 percent, laptop, 2 hours ago". Enter opens one. Only documents found on this computer are listed. `tw library --continue` prints the same list, with `--json` too.
- **Search by what your other computers know.** With sync on, a document's title, author, DOI, ISBN, and kind of file travel with it, so the library's filter and `tw library --search` find a document here by a DOI only another computer knew. The newest detail wins; the date it was first added keeps the earliest.
- **Edit a document's details by hand.** Type a document's title, author, DOI, and ISBN yourself: **Edit details** in the File menu and the command palette (no key of its own) for the open document, or **F2** on a document in the library list. The form has one labeled line per field ("Author, 2 of 4"); Tab and Shift+Tab move between them and say each field's label and value, Enter saves, and Escape cancels and says so. A DOI or ISBN that is not one is said, and the form opens again on it. In the GUI each field is its own labeled dialog. What you type wins over the document's own details in the library, its filter, and `tw library --search`, and with sync on it reaches your other computers, newest edit winning per field; clearing a field brings back the document's own value. `tw library edit FILE --title ... --author ... --doi ... --isbn ...` does the same, with `--json`.
- **Statistics from every computer.** The statistics list and `tw stats` add every computer's reading of a document together, and show each computer's share on request (`tw stats --by-computer`). If the sync folder is slow, the list does not wait, and says "Other computers skipped: folder slow."

### Speech

- **Speech follows the sound device.** When a headset or another device goes away, speech moves to the current default device at once and the reading goes on; before, it waited a second and only noticed while audio was waiting. Sound errors no longer print over the terminal reader's screen.
- **Choose the sound device.** `[speech] output_device` (Output device in the settings screen) keeps speech on one device by its id, and `tw backends --devices` lists the devices with their ids. A device that is not connected falls back to the default. It applies to Eloquence, the Windows voices, DECtalk, and Piper; eSpeak NG, Speech Dispatcher, and the Apple voices play on the default device. The setting belongs to each computer and never syncs.
- **Piper's pitch sounds cleaner.** Raising or lowering a Piper voice's pitch now uses a band-limited resampler, so a raised voice no longer has a metallic edge.

### Voices

- **The GUI's voice manager covers every engine.** Ctrl+Shift+V opens a dialog named "Choose a voice" with every voice on the computer (Eloquence, SAPI 5 and OneCore, DECtalk, eSpeak NG, Piper, and Apple's on macOS), with Language and Engine filter buttons, then Use voice, Preview, Favorite, Remove, and Fetch the Piper voice list. Each button has a name, its key, and a description for NVDA and JAWS.
- **Preview a voice without choosing it.** The Say Status key (Alt+End) in the voice manager, in the terminal and the GUI, or the GUI's Preview button, speaks a sample in the focused voice, starting with its name. A voice of another engine is heard by starting that engine for the sample.
- **Other engines' voices join the open list.** They are listed in the background when the voice manager first opens; they now appear as they arrive, with the focus kept, instead of after opening the list again. The favorites no engine here has follow them, as "not on this computer".
- **Delete on a voice that cannot be removed** says so and leaves the voice manager open; before, it closed.

### Writing

- **Grammar checking is built in.** Ctrl+F7 and Ctrl+Shift+F7 (Alt+J for the fixes) now work in the terminal reader, the window, and the release packages, with no special build. It makes the programs about 11 MB larger. A build without the default features leaves it out ([Editing](docs/editing.md#building-without-grammar)).

### Reading fonts

- **Lexend on first choice.** Choosing Lexend, in Settings or in the window's font list, when it is not installed asks first: "Download the Lexend font, 206 KB, SIL Open Font License? y or n". On a yes, its two files come from the Lexend project at a fixed version, each checked by its SHA-256, and are kept with the license in the data folder. The window uses it at once, and PDF and EPUB output find it by name (`tw convert --font lexend`). A no keeps the choice and uses another reading font. The GUI's font list says "Lexend (download, 206 KB)" or "Lexend (downloaded)". Lexend's license now ships with textweaver.

### Audio export

- **Opus files.** Export audio, in the File menu, and `tw export-audio --out book.opus` write Ogg Opus files with no ffmpeg: the smallest of the formats, about 14 MB an hour, at 32 kilobits per second in one channel. The reader offers Opus after MP3: "Opus: the smallest, made for speech", and the Export audio command's description names it. The title, author, and chapters go in as Vorbis comments, the same as FLAC. textweaver encodes them with libopus, the reference encoder, built in; its BSD license is in the notices.
- **Export audio in the terminal reader.** The terminal reader's File menu and command palette now have Export audio, with Opus, as the GUI and `tw` do; before, its default build left the command out. Only a lean build of the reader (`--no-default-features`) goes without it.

### Braille files

- **Emphasis in contracted braille.** Grade 2 BRF files now carry the UEB indicators for bold, italic, and underline, as grade 1 files do, placed around liblouis's contractions.
- **Passages over paragraphs.** Capitals or emphasis that go on over several paragraphs or list items open again at the start of each and end once, after the last (UEB Rules 8.5.5 and 9.9.1). Each heading stands alone.
- **Backslashes in contracted braille.** A paragraph with a backslash in it, such as a Windows path, is no longer lost or changed in grade 2: liblouis read the backslash as the start of an escape, so "C:\data" came back empty and "\n" became a line break. The backslash is now written as UEB's backslash symbol.

### PDF files

- **Captions in six languages.** "Tabla 2:", "Abbildung 3.", "Tableau 1 -", "Tabela 4.", and "الشكل ٣:" are found as captions, as "Figure 3." and "Table 2:" were.

### The GUI

- **The test window gives the focus back.** A window started with `--background` for automated checks hands the foreground straight back when UI Automation activates it, so a check run on your own computer no longer leaves textweaver in front of your work.
- **Masonry's debug trace log** is written only where `MASONRY_DENSE_LOG_DIR` points, and never to the system's temporary folder; before, every debug start wrote one there.

### Packages, CI, and checks

- **The install scripts check for cmake** before building from source, and say how to install it; installing a release package is unchanged.
- The nightly fuzzes the sync record and the sync group files, 35 targets in all, and lists the Apple voice tests again.
- CI and the nightly run each crate's tests through its one test program.

### For contributors

- **Building from source needs cmake** for the Opus encoder (libopus, built from bundled source). Build `textweaver-tui`, `textweaver-cli`, or `textweaver-xilem` without the `opus` feature to do without it; Opus then goes through ffmpeg. The development container installs it. See `docs/dev/building.md`.
- **One integration-test program per crate.** Every crate's integration tests are modules of `tests/it/main.rs`; a new test is a module there, never a new file in `tests/`. Run one with `cargo test -p CRATE --test it -- MODULE::`. Programs with `harness = false` stay their own. See `docs/dev/testing.md`, "Where tests go".
- **`cargo xtask regen` rebuilds every generated file** in one command: the third-party notices, the settings reference, the keyboard reference, the `docs/site` data, and the crate counts in the docs. `cargo xtask regen --check` changes nothing and reports each on one line, such as "keyboard: FAIL, out of date; run cargo xtask regen". `dev-check` runs it, so the settings reference, the docs indexes, and the notices are now checked locally as in CI.
- **Timing tests move the clock themselves.** The engine host's start deadline, stall timer, and sound-output reopen wait read a `Clock` that tests set by hand, and the flaky wall-clock checks (the slow-disk opening test, DECtalk's slow start) now check the order of events. The engine host's own test program and the Apple voice tests answer `--list` as cargo-nextest expects.
- **A new crate, `textweaver-sync`,** on core and store only: the hybrid logical clock, the merge types, the record and group files, and document identity. `cargo xtask deps --check` keeps it there, and refuses a lean reader that reaches grammar checking (harper-core).
- `cargo xtask bench --only sync-id` times document identity on a 100 MB file.
- **The minimum Rust version is 1.95,** up from 1.94: grammar checking's harper-core 2.11.0, now built in by default, does not build with 1.94. The pinned toolchain stays 1.96.
- `cargo xtask deps --check` also holds the lean reader to no audio export (`textweaver-export`, LAME, and libopus). The nightly's two sync fuzz crashes are regression tests and fuzz seeds (`fixtures/w7f`).
- Rubato is built without its FFT resampler by default, so the lean reader does not compile realfft; dictation asks for it.

## [0.1.0-alpha.6] - 2026-09-30

### Keys: what changed

- **F10 opens the menus** in the terminal, in every preset, as F10 does in Windows programs. Previous chapter, which had F10, keeps `Alt+PageUp` and `Shift+D` (browse).
- **New keys:** `Ctrl+F9` cycles the interface announcements, `Ctrl+Shift+F9` starts and stops dictation, and `Shift+F1` says what the next key does and where it is in the menus, without running it. On a Mac they are `Cmd+F9`, `Cmd+Shift+F9`, and `Shift+F1`.
- **New commands without a key,** in the menus and the command palette: Browse files, Batch convert, Export audio, Colors, and About.
- **On macOS the GUI uses Mac keys,** not Ctrl renamed: Option with the arrows moves by word and paragraph, Command with the arrows goes to the ends, `Cmd+[` and `Cmd+]` go back and forward, Command runs commands, and chords with Alt and a letter are Command+Option. Nothing takes VoiceOver's Ctrl+Option. The keyboard reference has a GUI on macOS column.
- Some terminals keep F10 for themselves; then run `menu` from the command palette (F2).

### Menus and the palette

- **Menus: F10.** File, Edit, View, Reading, Speech, Tools, and Help, with every command and its keys, in six languages. In the terminal they are a list: "Menus, 1 of 7, File"; Enter or Right opens, a letter moves to its item, Left goes back, Escape closes. Switches say "checked", choices their value. File, Recent documents lists what you opened last with your place in it.
- **The command palette** says each command name first with its menu: "Export PDF, File: ...". `ep` finds Export PDF; the best matches come first. With nothing typed, your recent commands come first, said as recent. Ctrl+L (or Tab in the GUI) moves to the list of matches.
- **What does this key do** (Shift+F1) and **About** in the Help menu.
- Menus and the palette are built from one list of commands, so both always offer the same things under the same names ([ADR-0043](docs/adr/0043-menus-and-the-palette-from-one-model.md)).

### The file browser

- **File, Browse files** ([ADR-0045](docs/adr/0045-a-file-browser-on-the-list-model.md)) walks through folders and archives as one list, from the File menu or the command palette. It starts on Places: the open document's folder, the start folder, the library's folders, and the drives on Windows.
- **Rows say the name first,** then the kind and size ("notes.md, Markdown, 12 KB"), and the position last ("3 of 40"), so a Braille display shows the name in its first cells.
- **Archives open like folders:** zip, tar, tar.gz, and 7z, and archives inside them, four deep. Nothing is unpacked; a document inside opens as `course.zip!week1/notes.md`, and its position, bookmarks, and notes stay with that path. `__MACOSX` and `.DS_Store` are left out. A damaged, too large, or too deeply nested archive is refused with a sentence.
- **Backspace goes up** and lands on the row you left, out of an archive too. Typing filters by name.
- **Alt+End previews** the focused row: a document's title and first sentence, read in the background.
- **Readable files only,** with Ctrl+A for every file; Ctrl+R sorts by name, date, or size.
- **Choosing a folder** for another command: Ctrl+Enter, or the "Choose this folder" row where a terminal cannot send Ctrl+Enter. In the Mac GUI the browser's keys use Cmd.
- The browser only opens and chooses; it never copies, moves, renames, or deletes a file.

### Announcements, colors, and settings

- **Say less about the interface:** `[accessibility] interface_announcements` (off, minimal, normal, full; Ctrl+F9) decides how much textweaver says about itself: lists opening and closing, progress, hints, and routine confirmations. Errors, questions, and answers to what you asked are always said. Automatic is minimal with a screen reader and normal when self-voicing.
- **Colors** (View, Colors): a color for the reading ruler, difficult words, syllable marks, misspellings, lint marks, search matches, the selection, the focus, links, headings, the status bar, notes, and bookmarks, from named colors (blue and orange first) or `#rrggbb`, each with its contrast said ("contrast 6.2 to 1, good"). Every mark keeps its underline or bold whatever its color.
- **The settings screen** lists the five settings you changed last at the top, says the default that Delete puts back, and says a setting's help on F1.
- **Import settings** names the first changes before asking. A renamed setting keeps its value. Importing settings in the GUI keeps the GUI's own keys.
- Numbers are grouped as your language writes them ("12.345" in German).

### Documents and conversion

- **Obsidian notes read the way Obsidian shows them.** A callout of any type says its type first, in words ("Warning: Hot surface"), and a foldable one says once whether it starts collapsed; `[!type]` is never read. The reader and `tw convert` share one set of callout rules. Embedded notes (`![[note]]`, `![[note#Heading]]`, `![[note#^id]]`) are read in place between "Embedded from Note" and "End of embed", from the note's own folder only, two levels deep, cycles refused; embedded pictures are graphics named by their file; tags read "tag physics slash waves"; `==highlights==` are marked; `%%comments%%` are not read; block ids are kept as link targets.
- **JSON files** read with a heading per key, so `h` moves by key, and no brackets, braces, or quotes; invalid JSON is read as plain text with a warning that says where it broke. **JSON Lines** give a heading per line.
- **Jupyter notebooks** open natively, cell by cell: text cells as Markdown, code cells named by their language ("Python code"), and outputs as quotes and graphics.
- **SVG drawings** read their title, description, titled parts, and text, or "Drawing with no description"; a drawing inside a web page is read the same way instead of being dropped.
- **Content MathML** is read as math, and `.mml` files open as one formula.
- **LaTeX:** macros with up to nine arguments and `\newenvironment` expand; `\bibliography` and `\printbibliography` list the cited works under "References"; `\multicolumn` and `\multirow` cells say what they span; `\includegraphics[alt=...]` is described by its alt text.
- The design for the new formats is in [ADR-0044](docs/adr/0044-obsidian-json-svg-and-content-mathml.md).
- Text sent to Pandoc in an older encoding is converted to UTF-8 first.
- **Batch convert from the File menu.** Choose a folder in the file browser, then a format (Markdown first, then PDF, HTML, plain text, EPUB, Word, braille), then where the files go (a `converted` folder, beside each file, or another folder), and answer "Convert 48 files to PDF into ...? y or n". It runs in the background while you read, says its progress in tens of percent at most every ten seconds, and Escape asks before stopping, leaving no half-written file. At the end the counts are said, the failures are listed name first with Enter opening one, and the list is saved as `conversion-report.txt`.
- **`tw convert` saves a report.** `conversion-report.txt` in the output folder (or the folder converted in place) has the summary sentence, then each failure and warning, the file's name first. Each run replaces it; `--no-report` leaves it out, and a report is never converted as a document.
- New messages, in all six languages: "it is not a readable JSON file", "... Jupyter notebook", "... SVG drawing", and "... MathML formula", said after "Could not open"; and the batch conversion's questions and progress.

### PDF and OCR

- **PDF comments are notes.** Sticky notes, highlights, underlines, and strike-outs from a PDF viewer become notes on the text they mark, with replies and whether they are resolved, as Word comments do: "Note: Comment by Ada Example: Cite a source for this. Reply by Bo Example: Added a citation. Resolved." A highlight with nothing typed in it says "Highlighted". Hidden notes are not read.
- **PDF links work.** Web and email addresses are said and offered to open; a link to another part of the PDF goes to the heading there, or to the page, said as "Page 3" and its first line. Links that would run a program or a script are left out.
- **Filled-in PDF forms are read**, label first and then the answer: "Name: Ada Example", "Student ID, required: empty", "I agree to the terms: checked", "Signature: not signed". The printed label and the line to write on are not read twice; buttons are left out; an XFA form is named in a warning.
- **Sideways and upside-down scans read in order.** Each scanned page is turned upright before it is recognized, judged from the page itself with nothing downloaded.
- **Scanned tables are tables** when their rows and columns line up, with the first row of words as the header.
- **Captions by pattern.** "Figure 3." and "Table 2:" are found in PDFs; a table's caption names the table, a figure's caption is read as a graphic, and a caption set large or bold is no longer a heading.
- New message, in all six languages: "Page 3", said first when a link goes to a page.
- The design is in [ADR-0048](docs/adr/0048-pdf-annotations-links-and-forms.md).

### Braille

- **Tables in braille files, three ways.** `[braille] table_format` and `tw convert --table-format` choose how BRF files lay out tables: `linear` (the default, one row per line with semicolons between entries), `listed` (each row starts in cell 5 with the first column's heading and entry, and each other entry follows on its own line after its column heading and a colon), or `stairstep` (each entry two cells to the right of the one before, the column headings in a transcriber's note at the same steps). Listed and stairstep follow BANA's Braille Formats (2016), 11.16 and 11.18; a transcriber's note says how the table is laid out, a blank entry is three guide dots, and a row stays on one braille page when it fits. A table of more than four columns is listed instead of stairstep, with a warning. Every table now has a blank line before and after it (11.2.5d).
- **Bold, italic, and underline in braille.** BRF files write the UEB typeform indicators (The Rules of Unified English Braille, 2013, section 9): a symbol indicator for one emphasized letter, a word indicator before each of one or two emphasized words (with a terminator only where the emphasis stops inside a word), and a passage indicator and terminator around three words or more. A heading all in one typeform leaves them out, since its place already shows it. Grade 2 through liblouis leaves them out for now.
- **Capitals passages.** Three or more words in capitals get the capitals passage indicator once and the terminator after the last (the Rules, section 8), instead of a word indicator before each word; one or two capitalized words keep their word indicators.
- New settings messages, in all six languages: "Braille tables" and its three choices.

### Dictation

- **Dictate in edit mode** (Edit menu, the palette, or `Ctrl+Shift+F9`), in the terminal reader and the GUI: what you say is typed at the cursor, phrase by phrase at each pause, with spoken commands such as "new line" and "period" applied; each phrase is one step for Undo. Outside edit mode it asks whether to turn edit mode on first.
- **Words while you talk.** The status line shows the words Whisper is sure of as they come, starting "Dictating:", the newest last, within 40 characters for a Braille display; they are never changed once shown. By default they are said once, at each pause, so the microphone does not hear textweaver's voice; `[dictation] speak_while_recording` says them as they come.
- **Never slower than before.** Short phrases arrive at their pause, as quickly as transcribing each phrase alone; long sentences show their first words about four seconds in. On a busy computer the words wait for the pause.
- **Nothing is lost.** Leaving edit mode, opening or starting another document, or quitting while dictating types the last phrase first; a phrase with no words recognized says so.
- Dictation streams on the Whisper model textweaver already uses; no new model ([ADR-0042](docs/adr/0042-streaming-dictation.md)).
- `tw dictate --live` prints each group of words as it is committed; `--live --file` plays a recording in at speaking pace.
- `[dictation] model_dir` names the Whisper model's folder.
- Fixed: every resampled recording (anything not already at 16 kHz) had a garbled first fraction of a second.
- New messages, in all six languages: "Dictating:", "Dictating. Speak, then press ... to stop.", "Finishing dictation.", "Dictation done.", "No words recognized in that phrase.", and the dictation errors.

### Audio export

- **Export audio from the File menu.** Choose a format (FLAC first, then MP3 and WAV; M4B only when ffmpeg is found, and when it is not, that is said in words), then where the file goes (beside the document, or another folder chosen in the file browser), and answer "Export essay.flac with Microsoft David at 200 words a minute, into ...? y or n", which names the voice and speed used. It runs in the background while you read, says "Exporting audio, 30 percent." at most every ten seconds, and Escape asks before stopping, leaving no file behind. At the end: "Wrote essay.flac: 42 minutes and 5 seconds, 12 chapters. Open it? y or n." Only engines that can write audio files are used. New messages in all six languages.
- **MP3 without ffmpeg.** `tw export-audio --out book.mp3` and Export audio in the reader write MP3 themselves, with the LAME encoder built in: variable bit rate, quality 5, suited to a speaking voice, with the exact length recorded for players. The title, author, and chapters go in as ID3 chapter tags. LAME is under the GNU LGPL; `THIRD-PARTY-NOTICES.md` explains what that means.
- **FLAC without ffmpeg.** `tw export-audio --out book.flac` writes a lossless FLAC file, about half the size of WAV, with nothing else installed. It carries the title, author, and chapters as Vorbis comments (`CHAPTER001`, `CHAPTER001NAME`, and so on).
- **Only M4B still needs ffmpeg.** The reader offers FLAC, MP3, and WAV without it. When ffmpeg is missing for M4B (or for MP3 in a build without the built-in encoder), the message suggests `.flac` or `.wav`.
- **WAV files have chapters.** The title, author, and chapters go into the WAV as an ID3 tag with chapter frames, as MP3 files have them.
- **Word times from Apple voices.** Exports with AVSpeech on macOS 14 and later carry each word's time, so word-level subtitles are exact.
- **A voice that cannot be loaded stops the export** with the reason, instead of the audio quietly using another voice.

### The GUI

- **Native menus** ([ADR-0046](docs/adr/0046-native-menus-in-the-gui.md)). On Windows, a standard menu bar that NVDA and JAWS read as any program's: Alt or F10 enters it, Alt with a menu's letter opens it (Alt+F for File), and each item is read with its key ("Open, Ctrl+O") and as checked or not checked. On macOS, the menu bar at the top of the screen, with each command's key as its shortcut. On Linux, and with `--list-menus`, F10 shows the menus as a list inside the window, as in the terminal. Choosing an item closes an open dialog first, as Escape would.
- **Every command works in the window** as in the terminal reader, from the same keys, the menus, and the palette, except the three that only mean something in a terminal (scrolling by lines and line numbers). Markdown lint works in edit mode; Browse files, Batch convert, Export audio, and Dictate are in the window's menus.
- **Starting like the terminal:** the first-run welcome and the language list, the question about hybrid mode, and unsaved work offered back. The speech engine starts in the background, so the window is ready at once.
- **Messages said at startup wait for your screen reader,** then are said once, instead of being lost. When the window takes the focus, the screen reader says its title; in the self-voicing mode textweaver says it too.
- **Notes, highlights, bookmarks, and search matches are drawn,** each with a shape as well as a color, and the spoken word's colors follow `[highlight]`. Exploring a formula and Speech Cursor's line keys work as in the terminal.
- **Copy, cut, and paste** go through your system's clipboard, in edit mode too, and the selection is the one you see.
- **Closing a dialog puts the focus back** where it was.
- **The Colors dialog** (View, Colors, or File, Settings, Colors) says each color in words with its contrast; a small sample is drawn beside it, never the only cue. Reset all colors puts back every one.
- **Export and import settings** use your system's Save and Open dialogs, in TOML or JSON. The font list uses the same list as every other list.
- **Windows High Contrast:** the window draws with your contrast theme's own colors, and follows it live when you turn it on or off.
- **On macOS the document is a read-only text area,** which VoiceOver reads with its text commands.
- Fixed: a crash when a caret key moved onto an empty line, and a crash when a second dialog of the same kind opened.

### Speed

- **A key in the GUI's edit mode** takes about 2 ms instead of 24 in a 1 MB document: only the paragraph around the edit is sent to the screen reader again, not the whole window.
- **EPUB, Word, braille, and PDF output of large documents is much faster.** Building the document's structure grew with the square of its size; 10 MB of Markdown now converts to EPUB in about 1 second instead of 25, and a 50,000-item list in under a second instead of 11. The output is the same.
- **Faster opening:** a document no longer waits for earlier saves to reach the disk before it opens (up to two seconds on a slow disk), and a large document's text check runs while it loads.
- Entering edit mode, and moving by line in it, says the line's first sentence or first 200 characters and "line continues", not the whole line (a 1 MB line held the first typed echo back by about a quarter of a second).
- The terminal reader draws only when something changed, and keeps what each frame needs until the document or the view changes; the difficult-word list is no longer loaded while the aid is off.

### JSON-RPC

- `insert` types text at the caret in edit mode, and is refused outside it.
- `list_key` knows the file browser's keys: `Details`, `ChooseHere`, `Sort`, and `ShowAll`.

### Packages, CI, and checks

- **Install the GUI with the scripts.** `install-windows.ps1 -Gui`, `install-macos.sh --gui`, and `install-linux.sh --release latest --gui` install the GUI beside the reader, with a shortcut or menu entry named "textweaver window". Running the script again, or the update script, keeps it; `--no-gui` removes it.
- **One GUI package for every Mac.** The macOS GUI is universal (Apple silicon and Intel): `textweaver-VERSION-macos-universal-gui.zip`.
- **The Linux GUI speaks with the Linux engines** the terminal package has (espeak-ng, speech-dispatcher, Omnivox); `cargo xtask gui-dist` names any engine it leaves out.
- **Braille is checked by a second tool.** CI builds liblouis 3.39.0 and reads textweaver's BRF files back to print, in grades 1 and 2.
- **Real engines in CI.** A workflow exports a document through espeak-ng on Linux, SAPI 5 and OneCore voices on Windows, and AVSpeech on macOS 14 and 15, and fails only when an engine cannot export or falls back to another; length, level, speed, and word times are reported.
- **Screen reader checks:** the GUI's accessibility tree dump (Windows, macOS, and Linux) and the Orca session are standing checks; the NVDA session stays report-only ([ADR-0039](docs/adr/0039-automated-screen-reader-checks.md)).
- The nightly release-mode tests run under cargo-nextest, and a test that passes only on a retry is named as flaky; a test keeps the nightly fuzz list in step with the fuzz targets, and the new formats (JSON, SVG, Obsidian, PDF comments and forms) are fuzzed every night.
- The fake engine-host tests check the order of events, not the clock, and their pause tests run at real time, and they passed 40 runs of 40 under load.
- `cargo xtask release` lists every other line still naming the old version; CI's docs job keeps a build cache.

### For contributors

- `App::choose_folder` and `App::choose_file` hand a chosen path to a command (batch conversion and audio export use them); `ListKey` gains `Details`, `ChooseHere`, `Sort`, and `ShowAll`; `archive::list_path` and `archive::is_junk` ([ADR-0045](docs/adr/0045-a-file-browser-on-the-list-model.md)).
- `Converter::run_plan_with` reports each file as it finishes and stops when asked, never leaving a half-written file; `tests/matrix.rs` converts a fixture for every loader to Markdown and PDF, and a new loader needs one; `cargo run --release -p textweaver-writers --example bench_blocks -- FILE` times the block tree and each writer.

### Documentation

- **The documentation site:** every table is named by the heading above it, so NVDA's and JAWS's table keys say which table it is; Up and Down Arrow in search say the highlighted result's title and place; a code block or table wider than the page takes focus, so the arrow keys scroll it.
- **Screen reader guide:** Orca says every key by default; how to turn that down.

## [0.1.0-alpha.5] - 2026-09-29

The fifth alpha. Braille comes first: every status line, list, and prompt puts the meaning in the first 40 cells, and math can be written in Nemeth or UEB braille. LaTeX, email, and web archives open without Pandoc; documents can be summarized with no model; publishing templates make APA and AMA papers with real Word footnotes; and the GUI ships in the release on every system, with syllables, the voice manager, a question dialog, and its labels in six languages.

### Testing

- **Listening check (Tuesday, September 29, 2026):** the owner listened to the `cargo xtask listen` samples from this build. ETI-Eloquence and SAPI 5 (Microsoft David) were clear at the normal, fast, and low settings; the low samples were pitched down as intended. The owner used the GUI and the terminal reader with NVDA and JAWS earlier in the week. DECtalk is not installed on the test machine. The Braille display session, the third GUI session, and the check of documents and summaries are still to come.

### Reading and Braille

- **Meaning first on a 40-cell Braille display.** The title line starts with the position and the reading state ("Line 12 of 400, 3%, Reading"); in screen-reader and hybrid modes it starts at the first cell, and lists cover the window with no border. List items say their place first ("3 of 12, Chapter two, level 2"), in all six languages. A prompt's line keeps its label short, so what you type starts within 40 cells. "Open it?" questions come before the folder or the address.
- **Pages in a PDF.** Go To takes a page (a plain number is a page in a PDF; `line 12` is still a line; `p iv` goes by the printed label). Say Position starts with the page, the title line shows it, and a PDF with no headings lists its pages in the outline.
- **`y` and `n` answer "Open it?"** while a list is shown, in the window and over JSON-RPC as in the terminal.
- **Library search by author, DOI, and ISBN.** The library list (Alt+L, GUI Ctrl+Shift+B) now filters as you type, and it and `tw library --search` match the title, path, author, DOI, ISBN, and text. A DOI or ISBN matches however it is written. The author, DOI, and ISBN come from the document (front matter, Word and EPUB authors, web page meta tags, a DOI or ISBN near the start of the text) and from `tw cite`'s record of the same work.
- **Star's settings profiles** are imported by `tw migrate-star`, one report line each.
- **Define word** opens the dictionary file on a helper thread the first time, saying "Dictionary still loading." once; the list opens when it is ready.
- **The statistics list** stays open on its row when Enter turns statistics on or off.
- **RSVP and flashing.** A test checks RSVP at 1,500 words a minute against WCAG 2.3.1's three-flashes limit: the terminal box and the window's panel stay under the threshold, and a cap is ready for any frontend that draws much larger words.
- The screen reader guide's Braille section is written for the HumanWare Mantis Q40, with NVDA and JAWS braille settings to try and a checklist.

### Documents and conversion

- **LaTeX opens without Pandoc,** in the reader and in `tw`: numbered sections as headings, lists, tables with header rows, captions ("Table 1: ..."), math as math, `\ref` numbers and links to sections, `\cite` as citations, footnotes, code, your own `\newcommand` shortcuts, and `\input` files from the document's own folder only. A command textweaver does not know is left out and its text read, and the document's warnings name it ([ADR-0035](docs/adr/0035-latex-email-and-web-archives.md)).
- **Email (EML):** the subject, then From, To, Cc, and the date with its weekday, then the message, with quoted lines read as a quote; attachments are listed with their sizes, not opened.
- **Web pages saved as one file (MHTML, MHT)** read as the page, with links leading to the pages they named on the web and pictures described by the descriptions saved with them.
- **Math written in MathML in a web page** is read as math, as it already was in EPUB books.
- A picture's description in a web page no longer runs into the next word.
- New messages, in all six languages: "it is not a readable LaTeX file", "... email message", and "... web archive", said after "Could not open".
- **Summaries without a model.** `tw summarize FILE` prints a document's most central sentences, one per line, in document order (`--sentences N`, `--json`). In the reader, Summarize in the command palette lists them for the selection, the chapter at the cursor, or the whole document; Enter goes to a sentence and says it. `[summary] sentences` sets how many, 5 by default. The method is LexRank, written in-house, with no download ([ADR-0037](docs/adr/0037-extractive-summaries.md)).

### Math

- **Math in braille files.** In a build with MathCAT, `tw convert --to brf` and the reader's `export brf` write math in the Nemeth Code (the default), between the Nemeth switch indicators inside UEB text, or in UEB mathematics with `--math-code ueb` or `[braille] math_code = "ueb"`. Lines stay 40 cells, and an indicator never ends up on a different line from what it belongs to. A formula MathCAT cannot write is written as its spoken words, and the summary says so once. See [docs/math.md](docs/math.md#math-in-braille-files).
- **Explore math with MathCAT.** With `math_engine = "mathcat"`, Alt+Shift+X moves through a formula with MathCAT's navigation; each step is said once, and the status line shows the braille of the part you reached, for a Braille display. `"builtin"` stays the default and is unchanged.
- MathCAT 0.7.6-rc.3 is vendored with the fix for its issue #827, which made math braille while exploring impossible in textweaver's builds. The fix is written up for upstream.

### Writing and publishing

- **Publishing templates** for EPUB, Word, and PDF: `tw convert --template apa` (an APA 7 student paper: title page, double spacing, APA headings, page numbers, hanging references), `ama` (an AMA 11 manuscript with a word count), `large-print`, `dyslexia-friendly`, `high-contrast`, and `manuscript`. Headings, lists, tables, and descriptions stay exactly as a screen reader expects; the layout options still apply on top ([ADR-0041](docs/adr/0041-publishing-templates.md)).
- **Real Word footnotes.** Footnotes in Word documents are Word's own: numbered by Word, at the foot of the page, and announced as footnotes by JAWS and NVDA. textweaver reads them back.
- **An EPUB cover** with every template: the title and author, described as "Cover: Title, by Author." and listed as the cover.
- **Print page numbers in PDF.** A PDF made from a document with print pages (a DAISY book, an EPUB page list, a scanned PDF) labels each page with its print page, so "go to page 42" in a PDF reader finds print page 42.
- **Difficult-word definitions.** With `[reading_aids] difficult_definitions` on (off by default), difficult words marked, and high verbosity, a word move onto a difficult word also says its first definition from the define-word dictionary.

### The GUI

- **Parley 0.11.1**, the text layout library, with no loss of speed or memory. `--graphics` (or `[gui] graphics`) picks one graphics interface; Vulkan alone saves about 26 MB.
- **Syllables are drawn** in the window (Alt+Shift+Z); the text a screen reader gets is unchanged.
- **The voice manager** works in the window (Ctrl+Shift+V).
- **Yes-or-no questions** are a dialog with Yes and No, answered with Y, N, or Escape for no.
- **Every drawn label comes from the catalog,** in all six interface languages, and changes live with the interface language. The settings dialog starts on a plain setting.
- **Edit mode:** caret and selection moves are spoken in self-voicing mode, Tab types a tab (Ctrl+Tab leaves the document), and misspellings are marked on screen.
- **The GUI ships in the release:** `textweaver-VERSION-windows-x86_64-gui.zip`, `textweaver-VERSION-macos-aarch64-gui.zip` (`textweaver.app`, Apple silicon), and for Linux x86_64 and aarch64 an AppImage and a tarball whose names end in `-gui`. Supported on Windows; on macOS and Linux built and checked automatically, not yet heard with a screen reader. Each is attested and in `SHA256SUMS.txt`. See [docs/install.md](docs/install.md#the-app).
- The GUI package now speaks with every engine the terminal package does: it carries the Eloquence, SAPI 5, and DECtalk engine hosts and the pronunciation dictionaries (Windows and Linux), the define-word dictionary, and every licence file, and on Windows it needs no Visual C++ runtime.

### Speech and dictation

- **Settings:** `[speech.dectalk]`, `[speech.piper]`, `[speech.voice_params]`, and `[editing] author` are typed settings, listed in the settings reference and the settings screen. `[reading_aids.font] fetch_missing`, which never did anything, was removed; an old `settings.toml` line is dropped. The "every setting is used" test now checks the reading aids' own tables too.
- **Speed presets** (F8) are said preset first: "Study, rate 200."
- Streaming dictation was measured on the in-process Whisper: for long speech, the first words can be committed about 3 seconds after speaking begins instead of at the end. It is planned for the next alpha.

### Speed

- Large Markdown files open faster: 10 MB loads in about 275 ms instead of 420, with a sixth of the allocations, because the file is parsed once instead of twice.
- Reading a whole document from the top plans its speech faster: 10 MB in about 610 ms instead of 770.

### Packages, CI, and checks

- The release workflow checks each GUI package before upload: the files, `--version`, and a screenshot drawn without a display; on macOS also a silent reading in a background window.
- `cargo xtask release VERSION --dry-run` reports an empty changelog section with the other problems instead of stopping.
- The nightly checks can pass again: the fuzz targets are built for glibc (the prebuilt cargo-fuzz defaulted to musl, where the sanitizer cannot link), the release-mode tests have time to finish, and the minimum Rust version is 1.94, what the dependencies need.
- The formats crate has an `images` feature: the picture and scanned-page loaders without the in-process OCR engine. The fuzz targets use it, so they no longer compile that engine.
- **Screen reader checks in CI:** the GUI's accessibility tree is dumped and compared on Windows, macOS, and Linux; NVDA (through Guidepup) and Orca read the GUI in scripted sessions, reported without failing the build ([ADR-0039](docs/adr/0039-automated-screen-reader-checks.md)). VoiceOver cannot yet be started on GitHub's Mac runners.
- Dependabot no longer proposes upgrades to the pinned GUI stack, MathCAT pre-releases, resvg, or sha1 and sha2 0.11.
- The rope stays ropey 1.6, after measuring the alternative ([ADR-0034](docs/adr/0034-rope-after-measurement.md)).

### Documentation

- The documentation site is built with Zensical and published at https://leavesofgrass.github.io/textweaver/.
- The README is short and starts with the quick start; the guides are updated for everything above; the design records read in a neutral voice; and there is a path for a first contribution.

## [0.1.0-alpha.4] - 2026-09-28

The fourth alpha. The keys follow NVDA's and JAWS's browse mode; textweaver speaks and shows its words in six languages; RTF and OpenDocument files open without Pandoc; Word comments and tracked changes are read; the Xilem GUI passed the owner's two screen reader sessions and draws the reading aids; and Linux gets AppImages for x86_64 and aarch64 (arm64), the first Linux packages. Changes from Wave 4 come first, by area; the additions since 0.1.0-alpha.3 from the earlier waves follow under Added, Changed, and Fixed.

### Testing

- **Listening check (Monday, September 28, 2026):** the owner used the release build with NVDA and JAWS in two sessions. ETI-Eloquence read with the words highlighted correctly in the GUI; the owner's findings from the second session (a console window, the Open dialog, font and text size, button shortcuts) were fixed before this release. SAPI 5 and Eloquence wrote clean samples with `cargo xtask listen`; DECtalk is not installed on the test machine. The owner's Braille display session comes in the next wave.

### Keys: what changed

The default keys are now the quick navigation keys of NVDA's and JAWS's browse mode (the owner's decision). `preset = "classic"` under `[keyboard]` keeps the earlier keys; `preset = "screen-reader"` now means the default. Old key, then where its command went (terminal):

- `.` next sentence: `Alt+Down` or `Alt+.`. `.` now says the sentence.
- `,` previous sentence: `Alt+Up` or `Alt+,`. `,` now says the paragraph.
- `s` say the sentence: `.` or `Alt+Shift+S`. `s` is the next separator.
- `Shift+S` say the paragraph: `,`. `Shift+S` is the previous separator.
- `l` say the line: `Alt+Shift+L` (GUI `Ctrl+L`). `l` is the next list.
- `o`, `Shift+O` lists: `l`, `Shift+L`.
- `Shift+H` history back: `Backspace` or `Alt+Left`. `Shift+H` is the previous heading.
- `Shift+L` history forward: `\` or `Alt+Right`. `Shift+L` is the previous list.
- `k` scroll up: `Shift+J`. `k` is the next link.
- `Shift+K` link address: `Alt+Shift+K`. `Shift+K` is the previous link.
- `q`, `Shift+Q` quit: `Ctrl+Q`, which still asks. `q` is the next block quote.
- `Alt+Down`, `Alt+Up` notes: `F12`, `Shift+F12`, or `e`, `Shift+E`. `Alt+Down` and `Alt+Up` move by sentence.
- `'` and `"` notes: removed (`"` is Shift+2 on most non-US layouts).
- `Alt+Shift+D` add a reference (terminal): `Alt+B`, because Windows Terminal splits panes with it. The GUI keeps `Alt+Shift+D`.
- RSVP faster and slower: `Alt+Shift+PageUp` and `Alt+Shift+PageDown` as well, because Windows Terminal resizes panes with `Alt+Shift+Up` and `Alt+Shift+Down`.
- New: `g` graphics, `d` sections or chapters, `Ctrl+Down` and `Ctrl+Up` paragraphs, `Alt+Shift+Q` citations, `Alt+Shift+X` explore math, `Alt+Shift+Z` syllables, `Alt+Shift+J` difficult words. `p` still moves by paragraph; `Shift+W` still says the position.
- `1` to `6` and Shift with them follow the physical digit key on any layout: read from the console on Windows; the shifted digits of the US, UK, German, Spanish, Nordic, and Italian layouts elsewhere, and French with `[keyboard] digit_row = "azerty"`.

See [docs/keyboard.md](docs/keyboard.md#what-changed).

### The terminal reader and `tw`

- A yes-or-no question ("Quit textweaver? y or n", "Delete this note? y or n", "Reload it? y or n", the Piper download and every other one) is now spoken even while textweaver is reading aloud. It went to the status line only, so a self-voicing user heard the reading go on and the next key press vanished into the question.
- The first run says a one-line welcome after the document opens: the keys that read, stop, move by heading, open the help, and quit, named from the keymap in effect.
- Startup warnings (an unreadable settings file, a bad keymap line) follow the "Opened" message instead of cutting it off, and share the status line with it.
- "Could not open" is one plain sentence: a missing file names the file and its folder, a folder says it is a folder, and no operating system error code is read aloud. `tw text`, `tw info`, and `tw search` say the same thing, and `tw open` refuses a folder before the reader starts.
- The F1 help names how to open a document and the library, the quick navigation keys (h, 1 to 6, l, i, t, k, q, s, g, d), the accessibility mode (Alt+Shift+A), single-key shortcuts (F9), the settings screen, choosing a voice, and restarting speech. Each line names at most two keys (the single key and a chord that works with single keys off), and a key bound in two layers is no longer read twice ("Tab or Tab").
- "No document is open. Press Control O ..." takes the key from the keymap.
- `textweaver --help` describes the reader and its first keys instead of "self-voicing ratatui frontend".
- `tw text big.pdf | head` no longer panics when the pipe closes; `tw define` no longer prints its failure twice; `tw marks` takes `--home` like the other commands.
- Docs: lists no longer close on `q` (a letter jumps to the next item), and Delete in the notes and highlights lists asks first. The findings and what is left are in the terminal usability research (kept outside the repository).
- The title line says "Ready" until something is read, then "Stopped". A screen reader reading it at startup heard "Stopped".
- Keys named in messages are spoken by name by textweaver's own voice ("Control S", "Alt period"), so they are heard at every punctuation level; the status line and the screen reader keep the written form ("Ctrl+S"). Every key named in a message comes from the keymap.
- New: **Repeat message** (`'`, or `Alt+'` anywhere) says the last message again. **Say status** (`z`, or `Alt+End` anywhere) says the last message, then the mode, the reading state, the position, the rate, and the speech engine. Both are in the command palette and heard over the reading.
- In a list, **F1** or **Alt+End** says the list's introduction again (its name, how many items, the keys it takes), then the item you are on.
- **Escape** in edit mode with nothing being read says "Still editing. Ctrl+E finishes."
- The command palette says "Command. Type part of a name; Tab completes, Up and Down list matches." when it opens.
- Messages said while the speech engine starts ("Opened", a settings warning, the welcome) are said once it is ready, in order, instead of being lost.
- `tw` with no arguments prints a two-line hint and exits 0; `tw --help` keeps the full list. `tw search --json` and `tw info` no longer panic when a pipe closes.

### Documents

- RTF and OpenDocument text (ODT, OTT, and flat FODT) open in the reader and in `tw` without Pandoc: headings, lists with their numbers, tables with header rows, footnotes, links, and pictures' descriptions. RTF in older code pages, such as Cyrillic and Japanese, reads correctly. Try `fixtures/c2/handout.rtf` and `fixtures/c2/notes.odt` (ADR-0031).
- Comments in Word and ODT files, with their replies and whether they are resolved, become notes on the text they are about when the document opens, so reading tells you when you reach one: "Note: Comment by Ada Example: say how salty." Try `fixtures/c2/comments.docx`.
- Tracked changes in Word, ODT, and RTF files: `[reading] revisions = "marked"` says each change where it is, "(deleted by Ada Example: three) (inserted by Ada Example: five)"; `"final"` reads the text with every change accepted, as before; `"auto"`, the default, says them at high verbosity.
- Word, OpenDocument, EPUB, and PowerPoint files with more than 50,000 files inside, overlapping files, or a file that claims to unpack to more than 1,000 times its size are refused, and no package unpacks past 1 gigabyte.
- A damaged RTF or ODT file is named plainly: "Could not open notes.odt: it is not a readable OpenDocument text file; it may be damaged."
- Zip archives, and the EPUB and Word files built on zip, open whatever compression their members use: deflate, bzip2, LZMA, XZ, and PPMd, all in pure Rust. A member that says it is larger than 256 MB is refused before it is unpacked.
- EPUB 3 books: MathML is read as math, using the book's TeX when a formula carries it; a formula with only `alttext` is read as that text, and an `epub:switch` is read once.

### Math

- Math can be spoken by MathCAT, the engine NVDA and JAWS use, in ClearSpeak or SimpleSpeak, at the math verbosity, in the document's language: `[reading] math_engine = "mathcat"` or `"mathcat_simplespeak"` ("Math speech" in the settings screen). It needs a build with the new `mathcat` feature, which is off by default; textweaver's own math speech stays the default and the fallback. The highlight covers the whole formula while MathCAT reads it (ADR-0029).
- New crate `textweaver-mathcat` on MathCAT 0.7.6-rc.3, pinned exactly. Braille (Nemeth and UEB) waits for MathCAT issue #827 to be fixed in a release.
- Math as Unicode in the reading view (`x²`, `√2`, `1⁄2`), as Star showed it: `[reading] math_display = "unicode"`, or "Math on screen" in the settings list. Speech, edit mode, and math exploration use the source. Try `fixtures/g/math.md`.

### Writing and notes

- Markdown lint in edit mode: Ctrl+F8 and Ctrl+Shift+F8 select the next or previous problem and say it, "Lint: heading level 3 after level 1; use level 2." Five rules: heading levels, list markers, trailing spaces, link references without a definition, and bare web addresses. `tw lint FILE...` checks files and exits 1 when there are problems. textweaver's own rules, not rumdl (ADR-0032).
- Copying works in terminals without OSC 52 (the old Windows console, macOS Terminal, GNOME Terminal and other VTE terminals): textweaver puts the text on the system clipboard itself and says so the first time. Over SSH and in tmux it still uses the terminal.
- Notes and highlights export as BibTeX, BibLaTeX, RIS, or CSL-JSON records for Zotero or Pandoc: `tw marks FILE --export ris --output notes.ris`.
- Moving the caret onto a code block's first line names its language: "code, Python".
- Code blocks are highlighted in the terminal view (syntect with bat's syntaxes), with colors from the theme and never color alone.
- Grammar checking with Harper, offline, in edit mode (Ctrl+F7 and Ctrl+Shift+F7; Alt+J lists fixes). It adds about 10 MB, so it is built only with `--features grammar` and is not in the packages.

### Languages

- textweaver speaks and shows its own words in English, Spanish, French, German, Portuguese, or Arabic: messages, lists, help, keyboard shortcuts, the command palette, and the settings screen. `[interface] language`, "Interface language" on the settings screen, or `tw settings language es` (ADR-0030).
- The language changes at once: the change is said in the new language, then the title line.
- The voice follows the language when the speech engine has one for it. When it has none, the current voice keeps speaking and textweaver says so; it never goes silent. `[speech.voices_by_language]` picks a voice per language.
- The first run starts with the list of languages, your system's language first.
- Right-to-left text is reordered for display in terminals that do not do it themselves, and never for screen readers: `[interface] rtl`.
- textweaver's own voice says key names in the interface's language; the status line keeps their written names.
- The command palette also finds commands by their help in your language.
- For translators and developers: every message is in `crates/textweaver-lexicon/locales/*.ftl`; `en-XA` shows each one bracketed, and the new `pseudo` step of `scripts/dev-check` fails on any that was missed.

### The GUI

- `textweaver-xilem --announce uia` (Windows) announces with UI Automation Notification events instead of the live region, for comparing the two in NVDA and JAWS. `announce = "uia"` in a `[gui]` table of `settings.toml` does the same. The default is still the live region.
- List options and settings scrolled out of view are now in the accessibility tree, so a screen reader's object navigation reaches them: all 15 settings sections, and every option of a long list.
- Lists and the settings form say "1 of 15" (they said "2 of" with no total), and rows scrolled into view are drawn instead of blank.
- The UI Automation report checks both announcement paths and a long list scrolled to its end. See [ADR-0028](docs/adr/0028-xilem-gui-after-the-session.md), which also records where the GUI's memory goes (the graphics stack; the app itself is about 11 MB).
- The live region stays the default for announcements, and the spoken word's background color stays the highlight. `[gui] announce = "uia"` is now a real setting, in the settings dialog under "Window"; `--announce` and `--select-spoken` stay as options. ADR-0028 is accepted.
- A long document's window slides while reading without losing the screen reader's place: the text that stays keeps its nodes, and the caret stays on the spoken word.
- The reading aids in the window: text spacing, the reading ruler and current line, bionic reading, difficult words, and RSVP in its own strip under the document. The RSVP word is never spoken by itself; a quiet status beside it says where you are.
- In a list, F1 and Alt+End repeat the list's introduction, as in the terminal. The status bar shows the terminal's title line: the reading state, the line, the mode, the rate, and the engine. "No document is open" names the Open key from the keymap.
- New guide: [docs/gui.md](docs/gui.md), also shipped in the GUI package as `GUI.md`.
- The wxDragon GUI spike (`textweaver-gui`, ADR-0014) is removed now that the Xilem GUI has passed the owner's second screen reader session; the Xilem GUI (`textweaver-xilem`) is textweaver's GUI.
- After the second session: no console window opens with the GUI on Windows (`--help`, `--version`, and errors still reach the terminal it was started from, or a message box); Open shows the system's own file chooser, and Ctrl+Shift+G types a path instead; Ctrl+Plus, Ctrl+Minus, and Ctrl+0 size the text and Ctrl+D chooses the font, each said and saved; every button has its key from the keymap as its shortcut key, which screen readers say when set to, and shows it ("Open… (Ctrl+O)"). In the GUI the rate moved from Ctrl+= and Ctrl+- to F11 and Shift+F11. See [ADR-0033](docs/adr/0033-gui-session-2-and-edit-mode.md).
- Edit mode in the GUI: Ctrl+E or the Edit button makes the document a multi-line edit. Typing, undo, formatting, and saving are the terminal's; the screen reader echoes typing (textweaver does in the self-voicing mode). Spell check, citations while writing, export, and the browser preview work in the window.

### Speed and memory

- Words and sentences are found two to three times as fast (ICU4X's segmenters), with the same boundaries. Planning a whole 10 MB document for reading takes about half the time it did.
- The reader asks the system for its light or dark setting only when that can change the theme, and no longer waits for the answer before building the reader.
- `--log debug` records how long the reader took to start.

### Robustness

Found by the new fuzz targets and the second-tool checks:

- Fixed: a BibTeX crossref loop overflowed the stack; loops, and chains deeper than 8, are cut.
- Fixed: a damaged lexicon file could crash textweaver; its headword map's checksum is checked when it opens.
- Fixed: front matter keys that start with YAML syntax or with spaces were lost in vault export; they are quoted.
- Fixed: exporting Markdown with a nested list to EPUB, PDF, DOCX, or braille crashed. A one-item list, and a list that is an item's only content, keep their nesting too, and lists nested past 64 levels are written as paragraphs instead of overflowing the stack.
- Fixed: every EPUB failed two epubcheck rules (RSC-011 and RSC-005).
- Fixed: `tw serve` could be made to allocate a terabyte; a message over 16 MiB now ends the session.
- Fixed: on macOS, an engine host could outlive a crashed textweaver; each host now exits when textweaver is gone.

### Packages, CI, and checks

- Nine new fuzz targets: LaTeX math, ASCIIMath, BibTeX, RIS, CSL-JSON, themes, the lexicon, vault import, and JSON-RPC, with `cargo xtask fuzz-seed`.
- A settings reference generated from the settings schema (`docs/settings-reference.md`, `cargo xtask settings-doc`), and `cargo xtask docs --check` for the ADR index, crate counts, "See also" sections, and index links.
- epubcheck and veraPDF check the writers' EPUB and PDF output (`second-tool.yml`).
- **Linux packages for aarch64 (arm64)** as well as x86_64: the AppImage and the tarball, built on GitHub's arm64 runner and checked on Debian and Fedora. `scripts/install-linux.sh --release` installs them on arm64 computers.
- `cargo xtask release` stops until the listening check in the release guide is recorded for the version (`cargo xtask release VERSION --listened` writes the machine's date), and until the changelog is grouped by area.
- The `Release` workflow started by hand with no tag is a dry run: every package is built and checked, and kept as a workflow artifact.
- CI runs the pseudo-locale test, which fails on any message missing from the translation catalog, and puts the Xilem GUI's AT-SPI report in the run summary.

### Added

- **Scanned pages (OCR).** Scanned PDFs and pictures (PNG, JPEG) are read by recognizing their text: English in process with the pure-Rust ocrs engine (models downloaded once with `tw ocr download`, 12.2 MB, after you agree), other languages with Tesseract when it is installed (`[reading] ocr_lang`). The recognized pages keep their headings, paragraphs, and page numbers. `tw ocr status` and `tw ocr read FILE`. See [ADR-0026](docs/adr/0026-ocr-and-student-formats.md).
- **More formats for students.** DAISY 3 books and DTBook (Bookshare zips too), PowerPoint slides with speaker notes, spreadsheets (CSV, TSV, ODS, XLSX) as tables, archives (ZIP, TAR, TAR.GZ, 7Z) with `book.zip!chapter.pdf` paths, and web pages (`tw open https://...`).
- **EPUB, Word, and PDF reading.** `textweaver` and `tw text` open EPUB (chapters from the table of contents), DOCX (headings, lists, tables, footnotes, alt text), and PDF. The PDF reader is pure Rust and on by default. It finds columns, removes running headers and page numbers, and recovers headings, lists, and tables. See [ADR-0010](docs/adr/0010-pdf-loader.md).
- **Conversion.** `tw convert` converts files and whole folders to Markdown, HTML, text, EPUB 3, Word, braille (BRF, UEB grade 1), and tagged PDF, on every core, skipping files already converted. It reads GFM, Obsidian, and Pandoc Markdown, turns LaTeX and ASCIIMath into MathML, uses MiniJinja templates, and can watch a folder. Pandoc is used only for formats with no native reader. See [docs/converting.md](docs/converting.md).
- **Bundled fonts and PDF layout.** Atkinson Hyperlegible Next and Mono and OpenDyslexic come with textweaver. PDF output uses them by default, and `tw convert` gains font, size, page size, margin, spacing, large print, title page, and contents options.
- **Math.** LaTeX and ASCIIMath are read aloud in natural English at three verbosity levels, with exact highlighting inside a formula. Prices such as "$5 and $10" are never taken for math. See [docs/math.md](docs/math.md).
- **Citations.** `tw cite` keeps a reference library: look up a DOI or ISBN, import and export BibTeX, BibLaTeX, RIS, and CSL-JSON, and format with CSL styles such as APA, MLA, Chicago, and IEEE. See [docs/citations.md](docs/citations.md).
- **Themes.** Star's 23 themes, every one checked for contrast, with Galaxy as the default. F5 cycles them; your own themes load from the `themes` folder; textweaver can follow the system's light, dark, or high-contrast setting. See [docs/themes.md](docs/themes.md).
- **Reading aids.** RSVP (one word at a time), bionic reading, the reading ruler, terminal text spacing, and the reading level. See [docs/reading-aids.md](docs/reading-aids.md).
- **DECtalk,** for a DECtalk you have installed and licensed, with word highlighting and exact subtitle timing. See [docs/dectalk.md](docs/dectalk.md).
- **Library list** (Alt+L): the documents in your library folders and your recent files.
- **Install scripts** for Linux (any distribution), macOS, and Windows, and update, speech-check, doctor, dev-check, and convert-folder helpers. See [scripts/README.md](scripts/README.md).
- **`cargo xtask bench`** times the reading and authoring hot paths.
- **Licence notices in every package.** `THIRD-PARTY-NOTICES.md` lists the Rust crates and the bundled data (fonts, SCOWL, the IBMTTS dictionaries, citation styles, and the Adobe font metrics), and `licenses/` holds the font and SCOWL licence files. The packages also carry every user guide and the offline pages in `docs/site/`.
- **Releases are built in CI**, Windows as well as macOS, with build provenance you can check with `gh attestation verify`. `cargo xtask release X.Y.Z` prepares a release and dates the changelog from the machine's clock.
- **A Linux package: an AppImage.** One file for x86_64 that runs on Debian, Ubuntu, Fedora, Arch, and most other distributions from 2022 on, with `textweaver`, `tw`, the Eloquence (Voxin) and DECtalk hosts, and the dictionaries. `--tw` runs `tw`; `--install` links `textweaver` and `tw` into `~/.local/bin` and adds a menu entry. A plain tarball is there for systems without FUSE. `scripts/install-linux.sh --release latest` downloads, checks, and installs either. See [docs/install.md](docs/install.md#linux).
- **Documentation.** A documentation index ([docs/README.md](docs/README.md)); new guides for reading, editing, notes, the library, speech, math, citations, audio export, the Obsidian vault, dictation, JSON-RPC, troubleshooting, and screen readers; an architecture guide; CONTRIBUTING.md; and interactive, accessible pages in `docs/site/` about the architecture, the speech pipeline, the keyboard, the features, and the reading aids. `tools/check_links.py` checks every link.
- **Settings export and import as JSON.** `tw settings export` saves every setting and key override to one JSON (or TOML) file; `tw settings import` checks it, backs up your files, and applies it, with `--dry-run` to preview each change. `tw settings path` and `tw settings reset` are new too. See [docs/settings.md](docs/settings.md).

- **Choose a voice** (Alt+V) lists the engine's voices; Enter selects one and speaks a sample.
- **A log file.** Warnings and errors go to `textweaver.log` in the state folder, rotated at 1 MB. `--log debug` (or `TEXTWEAVER_LOG`) logs more; `--log off` turns it off.
- **Files changed on disk.** Saving over a file that another program changed since you opened it asks first. A change to an open file with no unsaved edits offers a reload.
- **Speech recovers from an engine crash or a stall.** The reading goes on from the last word heard, and says "Speech restarted".
- **GFM task lists** say "checked" or "not checked", and the text of HTML in Markdown (`<p align>`, `<details>`, `<img alt>`) is read.
- `tw export-audio` uses your settings: voice, rate, pitch, volume, the preferred engine, table and footnote modes, and the `[export]` subtitles.
- **Restart speech** (Shift+F8) starts speech again with your current settings. When speech stops working, textweaver restarts it once by itself and says "Speech restarted."
- **Positions survive outside edits.** When a file changed in Obsidian, git, or another editor, your place, bookmarks, notes, and highlights are found again from the text they were on, and textweaver says once what moved and what it could not find.
- `[editing] undo_steps` and `undo_memory_mb` cap the undo history (1,000 steps or 50 MB by default).
- **Screen reader modes.** `[accessibility] mode` chooses who speaks: self-voicing (textweaver speaks everything), hybrid (textweaver reads documents aloud; your screen reader speaks messages, typing, and caret moves from the status line), or screen reader (textweaver is silent, and Space steps through the text a sentence at a time on the status line). Alt+Shift+A cycles and saves it, and `--mode` sets it for one run. On its first run with a screen reader, textweaver offers hybrid once. A quiet-screen option, a status-line cursor option, and a screen-reader keymap preset (`[keyboard] preset = "screen-reader"`) help too. See [docs/screen-readers.md](docs/screen-readers.md).
- **Structure while writing.** In edit mode, headings, lists, links, and tables can be moved through, and "say position" names the heading, in the text you are writing.
- **The outline** (Alt+O) lists the headings; type to filter them, and Enter jumps.
- **Spell check** on a built-in word list (SCOWL): next and previous misspelling (Alt+M, Alt+Shift+M), each spelled aloud; suggestions and your own word list (Alt+J); and a count of misspellings on save.
- **Citations while writing.** Alt+C inserts a citation from your library, with a page; Alt+Shift+D adds a reference by DOI or ISBN; the palette inserts a bibliography, checks citations, and imports references. `tw cite` gains `remove`, `styles`, and `check`.
- **Export and preview from the reader.** Palette commands export the open document, or the text you are editing, to HTML, PDF, Word, EPUB, or braille. "Preview in browser" writes a web page with MathML and rewrites it on each save. "Listen rendered" reads the text as it will render.
- **Citations and math in converted documents.** `tw convert` formats Pandoc citations in a CSL style and adds a References section (`--bibliography`, `--style`, `--no-citations`), and typesets math in PDF, Word, EPUB, and braille.
- **New documents from a template,** with front matter, the date, and a References heading (`[editing] author`).
- **A study sheet:** your notes and highlights as Markdown, grouped by the document's headings. A note is signalled with a sound when reading reaches it.
- **Reading and writing quick wins:** keys 1 to 6 jump to the next heading of that level; a word count; the address of the link at the cursor; following a link or footnote (Alt+Shift+F), wiki links included; table rows and cells (Ctrl+Alt+arrows); Enter continues a list; Tab and Shift+Tab move between table cells in edit mode; copy (Ctrl+C) to the system clipboard through the terminal (OSC 52); in edit mode cut, paste, select all, and deleting a word; Add note (Alt+N) in edit mode; typing echo cycled with Shift+F9; verbosity and punctuation cycled with Alt+Shift+V and Alt+Shift+N.
- **Favourite voices:** Space in the voice list marks one, and favourites are listed first. `[highlight] color` and `sentence_color` now colour the reading highlight, with a contrast warning.
- **Markdown:** wiki links, GFM alerts, math, and heading attributes are read, and strikethrough and horizontal rules are announced.
- `tw info --exact` counts words and sentences with the reader's full segmentation.
- **Citations in continuous reading** are skipped by default and said in words with Alt+Shift+Q (`[reading] citations = "off" | "words"`), with the highlight exact both ways. See [docs/citations.md](docs/citations.md#citations-in-continuous-reading-altshiftq).
- **Explore math** (Alt+Shift+X): move through a formula term by term, into fractions, scripts, and roots and back out, each part said and highlighted. See [docs/math.md](docs/math.md#exploring-a-formula-part-by-part).
- **Syllables and difficult words in the terminal reader** (Alt+Shift+Z, Alt+Shift+J): words drawn split into syllables with exact highlights, and rare words underlined and named on word moves at high verbosity.
- **Preview.** After a save, "Preview updated. Press F5 in the browser." `[preview] auto_reload` serves the preview from 127.0.0.1 with a secret address and reloads it after each save, landing on the heading nearest the caret; `live` also reloads after a typing pause. Both off by default. See [docs/editing.md](docs/editing.md#preview-in-the-browser).
- **Export progress.** A long export says "Still exporting to PDF, 2 seconds." and then every ten seconds.
- **Writers.** PDF output draws strikethrough; Word equations (OMML) read back from DOCX as LaTeX math.
- **GUI on macOS** says once when a built-in font falls back to a system font.
- **A settings screen** (Shift+F10, or "settings" in the palette): every setting with its value, filtered as you type; Left and Right change a value, Enter types one, Delete puts the default back. Each change is said and saved. See [docs/settings.md](docs/settings.md#the-settings-screen).
- **Large files open in the background.** A file of 512 KB or more opens on a helper thread: "Opening report.pdf. Escape cancels.", then "Still opening report.pdf, 3 seconds." The keys keep working, and Escape stops waiting.
- **JSON-RPC: lists, prompts, and settings.** `list_state`, `list_key`, `prompt_state`, `prompt_key`, `settings_schema`, `get_setting`, and `set_setting`. The server is woken as each word is heard instead of polling. See [docs/json-rpc.md](docs/json-rpc.md).
- **The app core for the GUI** (Wave 3): a document window of about 500,000 characters around the reading, a waker, an edit command for native text controls, and the settings schema. See [ADR-0024](docs/adr/0024-app-core-for-the-gui.md).

### Changed

- **Nothing slow waits on the keyboard.** The speech engine starts in the background, so the reader is ready at once and speaks when the engine is; `settings.toml` is written by the writer thread; and the misspelling count after a save follows a moment later instead of holding up the keys (0.6 s on 10 MB).
- **Say position moved** from `%` to Shift+W and Alt+Shift+Y.
- `tw speak`, `tw voices`, and `tw backends` read your settings, and take `--home`.
- Save As suggests a name from the first heading or the title, and asks before replacing a file. A crash, a closed terminal, Ctrl+C, or a stop signal saves your place and your unsaved work, and restores the terminal.
- Find and replace goes one match at a time, with match case and whole word choices.
- Entering edit mode on a 10 MB file takes about 50 ms instead of 276 ms, with less memory.
- Malformed or hostile files (deep nesting, impossible list and page numbers, binary files) can no longer stop a `tw convert` batch; Pandoc runs sandboxed, with a two-minute timeout.
- `tw info` is about three times faster on large files.
- **Nothing waits for the disk.** Saving, autosave, positions, bookmarks, notes, the library sidecars, and the check for a changed file run on one background writer; "Saved" is said when the file is written, and quitting waits for it (saying so if the disk is slow).
- **Nothing waits for the speech engine.** A restarting engine helper, the first 32-bit SAPI voice, and the audio device start in the background, so Stop and Pause always work at once; speech-dispatcher is never waited for after connecting; DECtalk is synthesized a sentence at a time, so Stop takes effect quickly.
- **Voices are listed once**, when the engine starts, and Choose Voice (Alt+V) opens at once, or says the voices are still loading and opens when they arrive.
- **Find** reads the document in pieces and keeps at most 10,000 matches around the cursor while counting them all; edit-mode Replace searches once per step and no longer builds a string per character.
- **The library** (Alt+L) is scanned in the background, with a count as it goes.
- Engine availability is checked once per run, not twice by `tw backends`.
- espeak-ng is loaded when textweaver starts instead of being linked in, so a build with the `espeak` engine runs with or without espeak-ng installed, and building it needs no espeak-ng development files. `TEXTWEAVER_ESPEAK_LIBRARY` names the library to load.
- Rate, pitch, and volume changes while reading are heard at once, on Eloquence and SAPI too, not two or three sentences later.
- Opening and reading large documents is much faster: on a 10 MB file, open to first speech went from 13 seconds to under a quarter of a second.
- Reading a long document starts at once: continuous reading is planned about ten minutes at a time. On a 10 MB file, next sentence while reading went from about 250 ms to 3 ms.
- The highlight is drawn as soon as its word is heard, not up to 40 ms later.
- A list says its first item after its title, and each item says where it is ("Chapter two, 2 of 5").
- The same message twice in a row is spoken twice by a screen reader: the status line blanks for a moment first.
- Entering edit mode on a large file is two to four times faster.

### Fixed

- An engine that keeps failing no longer floods you with errors: reading stops after three failures in a row.
- A damaged per-document state file or recent-files list is set aside as a `.bak` file instead of being overwritten, so notes and bookmarks are not lost.
- Saving writes through symbolic links, keeps file permissions, and refuses read-only and non-UTF-8 files (use Save As).
- F9 and the `[keyboard] character_keys` setting work, and Shift with the arrow keys selects.
- A speech volume above 100 in `settings.toml` is now set to 100 and reported, like other out-of-range values.
- Bookmarks and notes landed in the wrong place after saving and then quitting in edit mode.
- `[normalization] table_mode` and `footnote_mode` had no effect.
- Jumping to an ordered list item says "2. Walk the dog", not just "2.".
- AltGr characters (`@ [ ] { } \ | ~ €`) can be typed in edit mode and prompts on non-US Windows keyboards.
- Binary files (a PDF or Word file without its reader, a program) are refused with a clear message instead of being read as garbage; UTF-16 text without a byte order mark is decoded.
- speech-dispatcher counted as not available where `XDG_RUNTIME_DIR` is unset (containers), though `spd-say` worked.
- The `espeak` feature builds on Fedora 44 and current Arch: its FFI no longer runs bindgen over the system headers.
- Backspace and Delete remove a whole character (an emoji with its skin tone, a flag, a letter with its accent), not a piece of one.

## [0.1.0-alpha.3] - 2026-09-25

The first release with downloadable packages: Windows (x86_64) and macOS (universal, not notarized). Start with the [quick start](docs/quickstart.md) (`QUICKSTART.md` in each package); [docs/install.md](docs/install.md) has the details.

### Added

- **Edit mode** in the terminal reader. It has typing echo, Markdown formatting commands, undo and redo, and find and replace. Autosave snapshots are offered back after a crash.
- **Notes and highlights.** Add, list, step through, and delete notes. Highlight the selection or sentence. Notes and highlights move with your edits.
- **Quitting asks first.** It says "Quit textweaver? y or n". Press y to quit; n, a, or Escape cancels. Deleting a note asks the same way.
- **Single-key shortcuts can be turned off** (F9), so dictation and typing never trigger commands. Every command stays reachable with a modifier key or the command palette.
- **Library** support: library folders, recent files, and a bookshelf. `tw library` and `tw marks` list them. `tw migrate-star` imports settings, positions, bookmarks, notes, highlights, and the bookshelf from Star.
- **Audio export.** `tw export-audio` writes WAV, or MP3 and M4B with chapters when ffmpeg is installed. It also writes SRT or WebVTT subtitles with sentence or word cues.
- **speech-dispatcher backend** (Linux), with an index mark before every word for exact highlighting.
- **One engine host for all out-of-process engines.** It runs Eloquence and SAPI5 in both 64-bit and 32-bit builds. `cargo xtask hosts` builds them all.
- **Obsidian vault export and import** (`tw vault`) and **dictation** through a Whisper subprocess (`tw dictate`).
- **JSON-RPC server** (`tw serve --stdio`) and the in-process reader (`tw open`).
- **Community pronunciation lexicon** from the IBMTTS dictionaries. It is off by default and used only with engines that do not normalize text themselves.
- **Release packaging:** `cargo xtask dist`, and a workflow that builds the macOS package.

### Changed

- Speech statuses carry a reading generation, so a status from an earlier reading can never move the highlight.
- Code Factory's Eloquence (installed with some screen readers) is used only when you opt in with `TEXTWEAVER_ECI_CODE_FACTORY=1`. Its SAPI voices are hidden unless you opt in the same way.

## [0.1.0-alpha.2] - 2026-09-25

### Added

- **Apple speech on macOS.** There are two backends. `nsspeech` uses the classic engine and is the quickest to respond. `avspeech` uses AVSpeechSynthesizer and gives exact word highlighting. Both prefer Eloquence Reed when it is installed.
- `--voice` accepts plain names such as `Reed` or `Zira`.
- **SAPI5 voices on Windows,** including OneCore voices, with word highlighting.
- **Eloquence through its ECI engine,** with exact word timing:
  - on Windows, the 64-bit OpenEVV library, or a 32-bit `eci.dll`;
  - on Linux, Voxin.
- `tw eloquence` explains how to get Eloquence and shows what textweaver found.

## [0.1.0-alpha.1] - 2026-09-25

### Added

- **The first working reader.**
  - `textweaver FILE` reads text, Markdown, and HTML aloud. The highlight follows the spoken word.
  - Move by character, word, sentence, line, paragraph, heading, table, list, and link.
  - Speech Cursor mode, bookmarks, find, and navigation history.
  - Your position is restored when you reopen a document.
  - Keys are configurable.
- `tw`, the command-line tool: `text`, `info`, `search`, `speak`, `voices`, and `backends`.
- Speech backends: espeak-ng (Linux), Omnivox, and a silent backend.

[0.1.0-beta.4]: https://github.com/leavesofgrass/textweaver/releases/tag/v0.1.0-beta.4
[0.1.0-beta.3]: https://github.com/leavesofgrass/textweaver/releases/tag/v0.1.0-beta.3
[0.1.0-beta.2]: https://github.com/leavesofgrass/textweaver/releases/tag/v0.1.0-beta.2
[0.1.0-beta.1]: https://github.com/leavesofgrass/textweaver/releases/tag/v0.1.0-beta.1
[0.1.0-alpha.9]: https://github.com/leavesofgrass/textweaver/releases/tag/v0.1.0-alpha.9
[0.1.0-alpha.8]: https://github.com/leavesofgrass/textweaver/releases/tag/v0.1.0-alpha.8
[0.1.0-alpha.7]: https://github.com/leavesofgrass/textweaver/releases/tag/v0.1.0-alpha.7
[0.1.0-alpha.6]: https://github.com/leavesofgrass/textweaver/releases/tag/v0.1.0-alpha.6
[0.1.0-alpha.5]: https://github.com/leavesofgrass/textweaver/releases/tag/v0.1.0-alpha.5
[0.1.0-alpha.4]: https://github.com/leavesofgrass/textweaver/releases/tag/v0.1.0-alpha.4
[0.1.0-alpha.3]: https://github.com/leavesofgrass/textweaver/releases/tag/v0.1.0-alpha.3
[0.1.0-alpha.2]: https://github.com/leavesofgrass/textweaver/releases/tag/v0.1.0-alpha.2
[0.1.0-alpha.1]: https://github.com/leavesofgrass/textweaver/releases/tag/v0.1.0-alpha.1
