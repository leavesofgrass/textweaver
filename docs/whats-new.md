# What is new

This page summarizes what each release brought, for readers who want the changes without the full record. The [changelog](https://github.com/leavesofgrass/textweaver/blob/main/CHANGELOG.md) has every detail, and it is the record to trust if the two differ.

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
- Known limits: the app draws every highlight name the same way, and on macOS and Linux it pastes plain text. Anki import and export and the reading queue come in beta 3. A native Windows ARM64 package is not planned. See [Known limits](known-limits.md).

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
