# What is new

This page tells students and readers, in plain words, what each release brought. The [changelog](https://github.com/leavesofgrass/textweaver/blob/main/CHANGELOG.md) has every detail, and it is the record to trust if the two differ.

## 0.1.0-beta.1

Released on Saturday, October 10, 2026. textweaver is now in beta.

In short: one download per system, study tools, richer marks and review, more formats, update checks, and a graphical version that catches up to the terminal reader.

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
- Known limits: the app draws every highlight name the same way, and on macOS and Linux it pastes plain text. Anki import and export and the reading queue come in beta 2. A native Windows ARM64 package is not planned. See [Known limits](known-limits.md).

## 0.1.0-alpha.9

In short: the app is easier to start and to read, and a document can become a read-along page, a video, or Vorbis audio.

- A short first run, at most three steps, each one skippable. If a screen reader is running, textweaver reads less on its own.
- A Reading settings dialog gathers the reading choices in one place.
- The window works in narrow sizes, and list bullets and numbers are drawn.
- Written pauses in a document, such as a break tag, are honored by every speech engine.
- Export audio can write a read-along web page that highlights each word as it plays.
- With ffmpeg installed, Export audio can also make a video with captions and chapters.
- Audio can be written as Ogg Vorbis.
- Where am I tells you the time left in the document.
- HTML pages ask which theme to use.
- The command line follows one set of rules, with short one-line errors.
- Help reaches the guides, and there are start pages for students and staff, a list of known limits, and an accessibility statement.

## 0.1.0-alpha.8

In short: reading is faster, stops sound where things end, and medical and science text is said the way a clinician would say it.

- Reading pauses after headings, paragraphs, and list items.
- Medical and science text, identifiers, and units are said correctly.
- The window has a Contents panel (Ctrl+1) and a Notes panel (Ctrl+2) beside the document. F6 moves between the parts of the window.
- Closing the window with unsaved edits now asks first, so you do not lose work.
- Questions about deleting or replacing start on No, so a stray Enter keeps your things.
- The window's menus and title bar follow your theme.
- Dictation and other models are offered as downloads, and only when you say yes.
- Converting a folder leaves a report you can keep with an accommodation file.
- Large documents open and edit faster.
- On the Braille display, status text is never cut off, and lines put the meaning first.

## 0.1.0-alpha.7

In short: your notes and reading places can follow you between computers, with no account.

- Sync through a folder you choose. A USB stick or a tool such as Syncthing carries it. See [Syncing between computers](sync.md).
- Shift+F5 says how sync stands.
- Continue reading lists the documents you were in.
- You can type a document's title, author, DOI, and ISBN yourself.
- The voice manager covers every speech engine, and you can preview a voice without choosing it.
- Grammar checking is built in.
- Audio export writes Opus files.
- Choosing the Lexend font offers to download it, and asks first.

## 0.1.0-alpha.6

In short: menus and a file browser, and more kinds of documents.

- F10 opens the menus, in six languages.
- Browse files walks through folders and archives as one list.
- You can say less: interface announcements can be off, minimal, normal, or full.
- Obsidian notes, JSON files, Jupyter notebooks, and SVG drawings open and read sensibly.
- PDF comments become notes, PDF links and filled-in forms are read, and sideways scans are turned upright.
- Braille files carry bold, italic, and underline, and tables in three layouts.
- Dictation works while you edit, and shows words as you talk.
- The window has native menus and draws notes and highlights with shapes as well as color.

## Earlier releases

Releases before alpha.6 built the reader, the app, speech engines, OCR, and the writers. Read the [changelog](https://github.com/leavesofgrass/textweaver/blob/main/CHANGELOG.md) for them.

## See also

- [Known limits](known-limits.md)
- [Roadmap](roadmap.md)
