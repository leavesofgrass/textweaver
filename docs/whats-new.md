# What is new

This page summarizes what each release brought, for readers who want the changes without the full record. The [changelog](https://github.com/leavesofgrass/textweaver/blob/main/CHANGELOG.md) has every detail, and it is the record to trust if the two differ.

## 0.1.0-beta.5

Not yet released.

Summary: live documents. textweaver can follow any MQTT broker and topic as a document that grows while you read it, as a log or as current values, with each new entry carried to a screen reader and a Braille display.

- **Live documents.** Open a live source and its messages become a document you read like any other, with headings, tables, find, and bookmarks. New entries are added at the end and never move your place: the cursor, selection, bookmarks, find matches, the spoken range, and the Braille display stay on the same text. textweaver only receives; it never publishes. See [Live documents](live-documents.md).
- **The log view.** One entry per message, as a list with the meaning first ("N0CALL, Ada Example. Mobile, portable antenna.") or as a table with real headers, switched with `Alt+Shift+F12` while you stay on the same entry. A message can be one check-in, the whole log as a list, or an object wrapping the list, whose plain values become a heading line. A later message with the same id, or the same callsign when there is no id, corrects an entry in place. Common field names are recognized with no setup, and other fields are kept and shown after them.
- **Unread, newest, and pause.** The title line and status bar show the unread count in place of a position that would change with every entry. `Alt+Shift+F5` says the unread entries, `Alt+Shift+F6` jumps to the newest, and `Alt+Shift+F1` pauses updates and resumes them as one change.
- **The values view.** For sensors: one line per field, meaning first ("Temperature: 21.4 C"), each updated in place. Labels, units, decimals, value words, and order are set per field in `settings.toml`; a value that has gone quiet reads "no recent value"; and each field has its own Show changes and Speak changes choices, saved with the source. See [The values view](live-documents.md#the-values-view).
- **New entries on the Braille display.** Each new or corrected entry is sent whole, as one polite message, so the screen reader shows it on the display. textweaver's own voice is off by default; turn the screen reader's speech off to read by Braille alone. Entries are never silenced by the announcement level, never interrupt reading, and arrive as one message when they come in a burst. In the terminal reader, the status line carries the newest entry in 40 cells. See [Announcements](live-documents.md#announcements-braille-and-speech).
- **Setting up a source in one form.** Tools, Live sources, Set up a live source opens a form with the name, host, encryption, port, topic, user name, and view. `Tab` and `Shift+Tab` move between the fields and keep what you typed, `Up Arrow` and `Down Arrow` change the encryption and the view in place, and `Enter` saves; a wrong field is said in words and the form opens again on it. Edit a live source, or `F2` in the Open live source list, opens the same form filled in. Passwords are asked once in a masked prompt and kept only in the system credential store. The `[live]` and `[live.sources.<id>]` settings cover the rest. See [The form](live-documents.md#the-form).
- **Live source files.** The broker's owner can send a small `.twlive` file instead of a host and topic to type. Open it like a document: textweaver fills the form from it for you to check, and Enter adds the source. A file is read with care: a password in it is never read, unknown keys are named and left out, and every value is cleaned. Save live source as file writes one, without a password. See [A live source file](live-documents.md#a-live-source-file).
- **The masked password field edits anywhere** in the textweaver app: in the middle as well as at the end, with an asterisk as an ordinary character, and screen readers treat it as a password field.
- **Connection messages in words.** Connecting, connected and encrypted, connected over Tailscale, connected and not encrypted, reconnecting, and the reasons for a failure, such as a wrong password or an untrusted certificate, each start with their meaning. A failure only you can fix stops the retries. See [Connection messages](live-documents.md#connection-messages).
- **The JSON log.** The entries as received, with their arrival times, are saved to a dated file in the `logs` folder of textweaver's data folder while the log runs and when it closes, and a saved log reopens in the same view. See [The JSON log](live-documents.md#the-json-log).
- **In the textweaver app,** a live document adds buttons to the toolbar (Pause updates, Jump to newest, Table view or List view, Save log, Close source, and in the values view Show changes and Speak changes), and the status bar and window title show the unread count and the connection in words. See [A live document's buttons and status bar](gui.md#a-live-documents-buttons-and-status-bar).
- **textweaver's own broker, and sending.** `tw broker` runs a small MQTT broker with no configuration file to write: accounts for readers and loggers (a reader can never publish), TLS with a key it makes and keeps, checked by its pin, a join code for a quick session, and plain connections across Tailscale. `tw send` publishes one check-in, and `tw share` publishes a Markdown or text file as a live document, again on each save. See [textweaver's own broker](live-broker.md#textweavers-own-broker).
- **A guide for the broker's side.** [Running the broker for a live log](live-broker.md) covers Mosquitto on Debian 13, accounts and read-only access, Tailscale, 44Net, TLS with a private certificate authority, testing with `mosquitto_sub` and `mosquitto_pub`, and example messages.
- **For contributors:** a new crate, `textweaver-mqtt`, holds the receive-only MQTT 3.1.1 client, behind the `mqtt` feature that release builds turn on.

## 0.1.0-beta.4

Released on Saturday, October 24, 2026.

Summary: links between documents with Back and Forward, a Braille view, much fuller support for Obsidian notes and vaults, CW, which reads in Morse code, takes Morse keyed from a key or paddle, and teaches it, selecting with the mouse in the textweaver app, and Settings organized by task.

- **Links to other documents open in textweaver.** Press Enter on a link while reading, as in a screen reader's browse mode, or Alt+Shift+F in any mode. A link to another document, such as the quick start's links to the other guides, opens it in textweaver at the heading the link names, and you hear where you arrived: "Opened Keyboard reference, at Edit mode." Back (Alt+Left or Backspace) returns to the link where you were reading, and Forward (Alt+Right) goes to the document again, as in a web browser. If your edits are unsaved, you are asked first. Ctrl+Space still always reads from the cursor. See [Follow a link or a footnote](reading.md#follow-a-link-or-a-footnote-enter-or-altshiftf).
- **"Paused." is always said.** Pausing is confirmed at every interface announcement level, minimal and off included, so you always know a pause took effect with a screen reader running.
- **Every command works in the textweaver app.** Scroll down and Scroll up (`j` and Shift+J) move the document one line without moving the cursor, as in the terminal reader, and line numbers (View, Line numbers) are drawn in the app's left margin. The app no longer answers "This command works in the terminal reader." See [Scrolling and line numbers](gui.md#scrolling-and-line-numbers).
- **The Braille view.** The textweaver app can draw the line you are reading as braille dots in a bar at the bottom of the screen, for learning and teaching braille and for checking spacing (Alt+Shift+F2, or Braille view under Reading aids in the View menu). It follows the cursor and the spoken sentence, as many cells at a time as your Braille display has (40 by default), in uncontracted or contracted braille (Alt+Shift+F3 switches). Optional teaching aids put the print under each cell and outline the cells, so blank cells show. It is visual only: screen readers find it as a picture named "Braille view" and are never sent its braille. In the terminal reader the same key shows a row of Unicode braille in self-voicing mode. See [The Braille view](reading-aids.md#the-braille-view).
- **Contracted braille in BRF files from the reader.** The new braille grade setting (`[braille] grade`) also decides whether a BRF file exported from the reader is uncontracted or contracted; contracted braille needs liblouis, which reader builds now support and which textweaver fetches on request. The typeform indicator that BRF files used for the first transcriber-defined typeform was a backtick, a character that embossers and Braille displays may not show; it is now the UEB indicator `@#`.
- **Obsidian highlights are highlights.** `==highlighted text==` is said "highlighted" at high verbosity, where struck-out text is said "strikethrough", and never "underline". Exported, it stays a highlight: `==` in Markdown, `<mark>` in HTML and EPUB, a highlight in Word, and a transcriber-defined typeform in BRF, named in the transcriber's note. See [Obsidian notes](converting.md#obsidian-notes).
- **Obsidian comments stay private.** `%%comments%%`, inline or across paragraphs, are left out of converted files in the `obsidian` flavor, Markdown and HTML included, as they already were from reading; a `%%` never closed hides the rest of the note, as in Obsidian. `tw convert --keep-comments` keeps them, and the setting **Read Obsidian comments** reads them aloud. Comments that held a code block are no longer read.
- **More of Obsidian's syntax read in words.** Custom task statuses (`- [/]` "in progress", `- [-]` "cancelled", `- [>]` "forwarded", and others), inline footnotes (`^[a note]`) in every footnote mode, embedded PDFs, sounds and videos ("Embedded PDF, slides, page 3"), a picture's size or description, and Mermaid diagrams ("Mermaid diagram source").
- **Obsidian links reach the whole vault.** A note's links and embeds are found anywhere in its vault, the folder holding `.obsidian`, and never outside it: by path, by name (the nearest note when two share a name, said once), then by title or alias. A note in no vault keeps its own folder, as before. `[[Note#^block]]` now opens at the block, and `[[Note#A#B]]` at heading B. Web pages converted with `--flavor obsidian` link to the right page in another folder. See [Obsidian vaults](vault.md#read-vault-notes-in-textweaver).
- **Properties.** Opening a note with front matter ends with "Properties: 4.", and **f** lists them, name first; lists of aliases and tags are read correctly everywhere. See [Properties](vault.md#properties-f).
- **Notes that link here, links to notes that don't exist, and tags,** as lists in the terminal reader and the textweaver app: in the command palette and the Reading menu under Go to. Enter on a note opens it; Enter on a tag lists its notes. From the command line: `tw vault backlinks`, `unresolved`, `tags`, and `properties`, which also lists Dataview fields. See [Ask the vault](vault.md#ask-the-vault).
- **Obsidian canvases.** A `.canvas` board opens as a document: a summary, then each card as a heading, text cards read as Markdown, file cards as links that open the note, groups as sections holding their cards, read top to bottom and left to right, and the connections in words, such as "From Study plan to Mitochondria: start here". See [Canvases](vault.md#canvases).
- **Writing Obsidian notes.** In edit mode, Insert wiki link lists the vault's notes, then a note's headings and blocks, and writes `[[Note#Heading]]` at the cursor. Toggle task (Ctrl+L, or Ctrl+Enter in the textweaver app, as in Obsidian) checks and unchecks a task and says which; Cycle task status steps through in progress, done, cancelled, and the other statuses; Insert callout writes a callout of a type you choose from a list. See [Obsidian notes: wiki links, tasks, and callouts](editing.md#obsidian-notes-wiki-links-tasks-and-callouts).
- **CW: reading in Morse code.** CW is a speech engine beside the others: choose it, and Play sends the document as a tone in Morse code, exactly as written. The highlight and RSVP follow each word as it is sent, the rate keys change the character speed, which the title line shows ("CW 20 wpm, effective 10") in place of the speech rate, and audio export writes Morse code. Speeds use Farnsworth spacing (20 words per minute characters at an effective 10 by default, 5 to 50), the tone is 600 Hz with soft edges that do not click, and structure is sent as prosigns: BT around headings and between paragraphs, AR at the end. For a Braille display, the status line shows each character as it is sent. `tw cw monitor` prints what a Morse key adapter sends, MIDI or keys, in words, and `tw cw ports` lists the MIDI inputs. See [CW: reading in Morse code](cw.md).
- **Key Morse code into a document.** In edit mode, Alt+Shift+F11 turns CW keying on: Morse code from a key or paddle is typed at the cursor, with a space after each word. textweaver does the keying, with an iambic keyer (mode B by default, or mode A, both with dot and dash memory, or a straight key) that adapts to your speed, and plays a sidetone while the key is down, at a pitch of its own (CW sidetone pitch, 600 Hz by default). MIDI adapters such as the N6ARA TinyMIDI and the CTR2 MIDI work in both the terminal reader and the textweaver app; keyboard adapters (VBand, Vail, the TinyMIDI in keyboard mode) work in the textweaver app. The status line shows each character as it is decoded, for a Braille display, and RSVP can show the word being keyed. See [Key Morse code into a document](cw.md#key-morse-code-into-a-document).
- **Morse code practice.** The Koch method, in the order Learn CW Online teaches: lesson 1 is K and M, and a round of 90 percent or better adds the next character. Random character and random word drills too. textweaver sends a group, you type what you heard, and it says the score in words ("4 of 5. Missed: K as R.") before the next group. Nothing is shown before you answer. In Tools, Morse code practice. See [Practice and training](cw.md#practice-and-training).
- **Select with the mouse in the textweaver app.** Drag across the text to select it, in reading mode and in edit mode; Shift with a click extends the selection, a double click selects a word, and a triple click a paragraph (a line of the source in edit mode). Dragging to the top or bottom edge scrolls the text. Copy, Cut, Paste, and the context menu act on a mouse selection exactly as on one made with the keyboard. A drag is announced once, when you let go, so a screen reader hears the selection and not every position the pointer passed. A click still places the cursor, also on a link, which Enter follows. See [Selecting with the mouse](gui.md#selecting-with-the-mouse).
- **Settings, organized by task.** Settings now open on short sections named for what you do: Voice and speech, Pauses and verbosity, Speaking text, Accessibility and announcements, Reading, Morse code, Braille, Display, and so on, the same in the terminal reader and the textweaver app. Voice and speech follows the speech engine: the voice, rate, and pitch with a speaking voice, only the chosen engine's own options, and with CW the Morse code speeds and tone instead of a voice. Settings for another system are not listed. All settings, and a search, still find every setting, and one that does not apply now says why: "Applies when Eloquence is the speech engine." In the textweaver app, the rows for tables of values, such as Speed presets and Voices by language, now say their name first to a screen reader ("Speed presets, 4 entries"). `settings.toml` is unchanged. See [The settings screen](settings.md#the-settings-screen).
- **For contributors:** the UI Automation report (`crates/textweaver-xilem/tools/uia-report.ps1`) follows the reorganized Settings sections. The Windows accessibility tree dump (`tools/a11y/tree-dump.ps1`) keeps asking accessibility-cli for the tree for up to 40 seconds and always stops the app, instead of ending at its first try. See [Testing](dev/testing.md#the-accessibility-tree-on-every-gui-change).

## 0.1.0-beta.3

Released on Saturday, October 17, 2026.

Summary: one program, `textweaver`, that opens the app, with `tw` for the terminal; study cards go to Anki and back, a reading queue that follows you between computers, dictation you can pause, and fewer programs in the package.

- **One program.** `textweaver` now opens the textweaver app, which most people want. The terminal reader and the commands are the same program: run `tw`, or `textweaver --tw` (`--cli` also works). `textweaver convert ...` and the other commands still run as before, so scripts keep working; `textweaver FILE` now opens the file in the app. `textweaver-gui` is gone. On Windows, `tw.exe` is a small console program that runs `textweaver.exe` in the terminal; use `tw` in terminals there. The install scripts upgrade an earlier install, removing the old app and its shortcut; the Start menu has "textweaver app" and "textweaver terminal". Settings, data, and the JSON-RPC interface are unchanged. See [Installing textweaver](install.md) and [The command line](command-line.md#one-program).

- **Anki.** Export cards to Anki writes a document's study cards as an Anki package (`.apkg`), keeping each card's grades and schedule. Import Anki cards adds a package's Basic and Basic-and-reversed notes to the open document and says what it skipped. From the command line: `tw study export --anki FILE` and `tw study import FILE`. See [Take cards to Anki and back](notes.md#take-cards-to-anki-and-back).
- **The reading queue.** A list of documents to read next, kept with the bookshelf. Ctrl+Enter in the library list adds or removes a document, and Reading queue in the File menu lists it. A document leaves the queue when you read it to the end. The setting After a document ends decides whether textweaver stops, names the next document, or reads it at once; it never starts a document you did not ask for. Also `tw library queue`. See [The library](library.md#the-reading-queue).
- **The queue syncs between computers,** with the places switch. A document added on one computer joins the queue on the others, and one taken out leaves it everywhere. A document another computer queued and this one lacks keeps its place, listed as "not on this computer". See [Syncing between computers](sync.md#the-reading-queue).
- **The review list,** beside Continue reading: documents you have read aloud, the one read longest ago first. Also `tw library review`.
- **Pause and resume dictation** (Alt+Shift+F9). The phrase being spoken is finished and typed first; what the microphone hears while paused is discarded. The textweaver app shows a Start dictation button in edit mode, and Pause dictation and Stop dictation buttons during a dictation. See [Dictating](dictation.md).
- **Escape leaves edit mode** in the terminal reader and the textweaver app, as Ctrl+E does, once nothing is open and nothing is being read; your edits and your place are kept. Leaving edit mode in a Markdown file now always returns you to the word you were editing, not to where editing began. In the app, the Edit button moved from the header to the toolbar; in edit mode it is named Done editing (Ctrl+E, Escape) and stands directly before Start dictation. See [Start and finish editing](editing.md#start-and-finish-editing-ctrle).
- **Each highlight name has its own color and shape in the textweaver app,** as in the terminal reader, so no two names differ by color alone. A screen reader hears the name in words on the text it covers.
- **Formatted text pastes as Markdown on macOS and Linux too,** as it already did on Windows.
- **Keys changed.** Ctrl+Shift+V (Cmd+Shift+V on macOS) now pastes as plain text in the textweaver app, as in most programs. Choose a voice moved to Ctrl+Alt+V (Cmd+Option+V on macOS); in the terminal reader Alt+V keeps working. F11 is now full screen in the textweaver app (Alt+Enter still works), and the rate is on + (or =) and - while reading, as in the terminal reader; F11 and Shift+F11 no longer change it. Keys you moved in `keymap.toml` are unchanged.
- **Fewer programs in the package.** An antivirus program had flagged two small engine helper programs. Eloquence, SAPI 5, DECtalk, and eSpeak NG now run in a copy of `textweaver`, and one 32-bit host, `textweaver-host-x86.exe`, runs 32-bit engines on Windows. The Windows package holds three programs instead of eleven: `textweaver.exe`, `tw.exe`, and the 32-bit host. Engines still run in their own process, and word timing is as exact as before. Old helper programs left by an update can be deleted. See [Installing textweaver](install.md) and [Troubleshooting](troubleshooting.md).
- **The self-test's answer is said once** by a screen reader, instead of twice.
- **The app's shortcut is named "textweaver app"** instead of "textweaver window", and the terminal reader's is "textweaver terminal" instead of "textweaver"; the install script removes the old Windows shortcuts.
- **Smaller download, and fewer release files.** The Windows package is about 38 percent smaller than beta 2's, since the app and the terminal program no longer each carry the same code. A Linux tarball update keeps the `tw` link. The `-gui.AppImage.zsync` files of the separate app AppImages (0.1.0-alpha.9 and earlier) are no longer published. The Linux menu entries are "textweaver" for the app and "textweaver terminal reader".
- **Guides rewritten** for a university-level reader, and the troubleshooting guide now explains antivirus heuristic warnings and how to check a package against `SHA256SUMS.txt`.

## 0.1.0-beta.2

Released on Saturday, October 10, 2026.

- **The speech rate works again with Eloquence and DECtalk.** In beta 1, a Windows SAPI 5 voice saved during a fallback kept both engines from changing speed: the rate number moved, but the voice did not, and an "unknown voice" error appeared. Now the rate, pitch, and volume always apply, both engines go faster than 430 words per minute, and that error is gone.
- **Choosing a speech engine in Settings switches to it right away.**
- **Speech keeps reading with RSVP on:** Play/Pause always reads aloud, and the RSVP word follows along.
- **Focus mode and full screen in the app:** the buttons and bars fade when the mouse is still, so that the text has the whole display. Keyboard reading keeps them hidden; moving the mouse brings them back. Full screen is Alt+Enter.
- **Settings is easier to read:** the highlight palette shows how many names it holds, and a SAPI voice shows its name.
- **Every guide is up to date** with beta 1.

## 0.1.0-beta.1

Released on Saturday, October 10, 2026. textweaver is now in beta.

Summary: one download per system, study tools, richer marks and review, more formats, update checks, and a graphical version that catches up to the terminal reader.

- **One download per system.** Each package holds the app, the terminal program `tw` (with `textweaver` as a second name), the speech engine helpers, and the guides. The separate downloads whose names ended in `-gui` ended with 0.1.0-alpha.9. Plain `tw` or `tw FILE` opens the terminal reader. See [Installing textweaver](install.md).
- **Updates.** textweaver asks once whether it may check for updates, checks at most once a day, and never downloads without asking. Help, Check for updates and `tw update` do it on demand. See [Updates](updates.md).
- **Components.** ffmpeg, liblouis, and Pandoc are fetched for you, from their official releases, when a feature needs them. Components can also come from your own repository or folder. See [Optional components](components.md).
- Study cards made from notes, highlights, and headings, graded in words and brought back when due, with a self-test you can answer aloud and recall prompts at the end of a section. See [Study with textweaver](notes.md#study-with-textweaver).
- A highlight palette: names such as "important" on Alt+1 to Alt+5, each with its own color and shape in the terminal reader. Notes can link to each other, and the links export as a knowledge graph in seven formats.
- Tracked changes and comments from a Word file can be listed, accepted, or rejected, and your own edits can be saved back to the Word file as tracked changes.
- Find and replace accepts regular expressions, with a panel in the app. Formatted text pasted into the terminal reader, and into the app on Windows, becomes Markdown. There is a context menu, and a preview pane beside the editor (Alt+F5).
- Org, reStructuredText, MediaWiki, DokuWiki, and Jira markup open and convert. Braille (BRF) files open as books, and DAISY 2.02 and 3 books play their recorded narration.
- HTML pages take their typography from your reading settings.
- eSpeak NG runs in its own helper program on Windows. Speech never stays silent while any engine can work.
- The app has buttons you can customize, the guides stored inside it, and one search over commands, keys, settings, and guides. Its menu bar is hidden by default on Windows (Alt or F10 shows it). Export asks where to save.
- Code blocks are read by default, and there is a developer profile example.
- Known limits: the app draws every highlight name the same way, and on macOS and Linux it pastes plain text. Anki import and export and the reading queue were planned for beta 3. A native Windows ARM64 package is not planned. See [Known limits](known-limits.md).

## 0.1.0-alpha.9

Summary: the app is easier to start and to read, and a document can become a read-along page, a video, or Vorbis audio.

- A first run of at most three steps, each of which can be skipped. If a screen reader is running, textweaver reads less on its own.
- A Reading settings dialog gathers the reading choices in one place.
- The textweaver app works in narrow window sizes, and it draws list bullets and numbers.
- Written pauses in a document, such as a break tag, are honored by every speech engine.
- Export audio can write a read-along web page that highlights each word as it plays.
- With ffmpeg installed, Export audio can also make a video with captions and chapters.
- Audio can be written as Ogg Vorbis.
- Where am I reports the time remaining in the document.
- HTML pages offer a choice of theme.
- The command line follows one set of rules, with short one-line errors.
- Help reaches the guides, and there are start pages for students and staff, a list of known limits, and an accessibility statement.

## 0.1.0-alpha.8

Summary: reading is faster, pauses fall where a passage ends, and medical and scientific text is spoken as a clinician would speak it.

- Reading pauses briefly after headings, paragraphs, and list items.
- Medical and scientific text, identifiers, and units are spoken correctly.
- The app has a Contents panel (Ctrl+1) and a Notes panel (Ctrl+2) beside the document. F6 moves between its parts.
- Closing the app with unsaved edits now asks first, so that no work is lost.
- Questions about deleting or replacing start on No, so that a stray Enter keeps what you have.
- The app's menus and title bar follow your theme.
- Dictation and other models are offered as downloads, and are fetched only after you agree.
- Converting a folder leaves a report that can be kept with an accommodation file.
- Large documents open and edit more quickly.
- On a Braille display, status text is no longer truncated, and lines put the meaning first.

## 0.1.0-alpha.7

Summary: notes and reading places can follow you between computers, with no account.

- Sync through a folder you choose. A USB stick or a tool such as Syncthing moves the files. See [Syncing between computers](sync.md).
- Shift+F5 reports the state of sync.
- Continue reading lists the documents you had open.
- A document's title, author, DOI, and ISBN can be entered by hand.
- The voice manager covers every speech engine, and a voice can be previewed without being chosen.
- Grammar checking is built in.
- Audio export writes Opus files.
- Choosing the Lexend font offers to download it, and asks first.

## 0.1.0-alpha.6

Summary: menus, a file browser, and more kinds of documents.

- F10 opens the menus, in six languages.
- Browse files walks through folders and archives as one list.
- Interface announcements can be set to off, minimal, normal, or full.
- Obsidian notes, JSON files, Jupyter notebooks, and SVG drawings open and read as structured documents.
- PDF comments become notes, PDF links and filled-in forms are read, and sideways scans are rotated upright.
- Braille files carry bold, italic, and underline, and tables in three layouts.
- Dictation works while you edit, and shows words as you speak them.
- The app has native menus, and it draws notes and highlights with shapes as well as color.

## Earlier releases

Releases before alpha.6 built the reader, the app, speech engines, OCR, and the writers. Read the [changelog](https://github.com/leavesofgrass/textweaver/blob/main/CHANGELOG.md) for them.

## See also

- [Known limits](known-limits.md)
- [Roadmap](roadmap.md)
