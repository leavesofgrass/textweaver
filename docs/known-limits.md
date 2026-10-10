# Known limits

textweaver is in beta. This page states what it does not do yet, or does not do well, so that you can decide what to rely on. Read it before you use textweaver for something important. Where a limit has a workaround or a planned fix, the next step is given with it.

## What has been tested, and what has not

- **Windows with a screen reader** is the combination that has been tested by ear: NVDA and JAWS, and a 40-cell Braille display. Two listening sessions went through the graphical version.
- **macOS and Linux** packages are built and checked automatically on every release. VoiceOver and Orca have had some basic testing, but not every release is tested with them by ear. More extensive testing is planned.
- **Other Braille displays and cell widths** have not been tried.
- **Settings marked "not yet verified"** in [the screen reader guide](screen-readers.md) have not been tried with a screen reader.

See [the accessibility statement](accessibility.md) for the full record.

## The program

- **Not code-signed.** Windows warns the first time you start it. On a Mac you remove the quarantine flag once. See [Installing textweaver](install.md).
- **No native Windows ARM64 package is planned.** On a Windows computer with an ARM processor, use the x86-64 package: Windows runs it under emulation.
- **No update check inside the program.** Update with the scripts, or download the new release. It does tell you, once, when it has been updated.
- **No guided first-run tour.** The [quick start](quickstart.md) is a document you open and read.
- **Some translations are not reviewed.** The interface has six languages. Native speakers have checked English and Spanish. German, French, Portuguese, and Arabic have not been checked yet. Feedback on any translation is welcome: use Help, Report a problem, or the [issue tracker](https://github.com/leavesofgrass/textweaver/issues).
- **Settings, keys, and file formats may still change** between releases. Read the [changelog](https://github.com/leavesofgrass/textweaver/blob/main/CHANGELOG.md) before you update.

## The graphical version

These limits apply to the textweaver app (`textweaver-gui`). The terminal reader does not have them, unless stated.

- **Every highlight name is drawn the same way.** In the terminal reader, each name in your highlight palette has its own color and its own shape, so a name never depends on color alone. The graphical version draws all highlights with one mark. The names are still kept, listed, filtered, and exported, and they show as shapes in the terminal reader and as typeforms in braille files. Next step: use the highlights list (Shift+Y), which says each name, or the terminal reader, to see the shapes. Drawing each shape in the graphical version is planned ([Highlight colors](notes.md#highlight-colors)).
- **A screen reader may say the answer twice in the self-test.** After you press Enter on a prompt in the self-test, the graphical version reveals the answer, and a screen reader may announce the revealed answer twice. The answer is correct both times. Next step: use the self-test in the terminal reader if the repeat gets in the way; a fix for the graphical version is planned.
- **Ctrl+Shift+V pastes instead of opening Choose a voice.** The keyboard reference lists Ctrl+Shift+V for Choose a voice in the graphical version, but in the app the key pastes. Next step: open the voice manager from the menus (F10) or the command palette (F2, then type `voice`), or give the command another key in `keymap.toml` (see the [keyboard reference](keyboard.md)).
- **Formatted text pastes as plain text in the graphical version on macOS and Linux.** On Windows, Ctrl+V in edit mode turns formatted text from a browser or word processor into Markdown. On macOS and Linux the graphical version pastes the clipboard's plain text. Next step: paste in the terminal reader, which converts it; beta 2 brings the conversion to the graphical version on macOS and Linux.
- **Some commands exist only in the terminal reader.** The app says "This command works in the terminal reader." when you press the key for one ([What only the terminal reader does](gui.md#what-only-the-terminal-reader-does)).

## Reading and speech

- **Cloud voices, Coqui, Festival, and Qt Speech** are not offered.
- **Eloquence and DECtalk** need your own licensed copy. textweaver does not include them.
- **DAISY audio timing in MP3 books is untested on real books.** The playback of a talking book's recorded narration is tested with built files and with WAV clips. How accurately the highlight follows the clips in a real DAISY book whose audio is MP3 has not been checked. If the highlight drifts, set **Book audio** to **speech only** ([Talking books](reading.md#talking-books-the-recorded-narration)) and report the book's source and format.
- **Source code** opens as plain text. It is not read as a structured document.

## Documents

- **Translating a document** is not offered.
- **RAR archives** do not open. ZIP, TAR, and 7z do. Extract a RAR archive first; the message says so.
- **Scanned pages** are read by OCR, and OCR makes mistakes. Check anything important against the original.

## Output

- **A cover image for audiobooks** is not added.
- **AAC audio** is not written. Ogg Vorbis, Opus, MP3, FLAC, WAV, and M4B are.

## Study tools

- **Study cards are scheduled with SM-2 only.** Cards are made from notes, highlights, and headings, studied, graded in words, and brought back when due ([Study with cards](notes.md#study-with-cards)). A card counts as due by the clock, from half a day before its interval ends, not by the calendar day in your time zone. FSRS scheduling is dropped on purpose.
- **Anki import and export come in beta 2.** Until then, cards stay in textweaver and sync between your computers with your notes. AnkiConnect sync is dropped on purpose.
- **The reading queue comes in beta 2.** There is no list of documents to read next that carries from one session to the next. Next step: the library's recent files (Alt+L) and bookmarks.
- **Knowledge graphs from notes are not drawn.** Links between notes are lists instead: each note's links, "What links here", and `tw notes links` ([notes guide](notes.md#links-between-notes)). The graph exports to files that other tools draw, with a Markdown list as their text equivalent ([Export the knowledge graph](notes.md#export-the-knowledge-graph)).

## Documentation

- **The guides are being rewritten for beta 2.** Beta 1 corrects wording and fills gaps, but a full rewrite of the guides comes in beta 2. If a guide contradicts what the program does, the program is right; please report the page.

## What this page does not promise

textweaver does not claim to improve reading speed, comprehension, or comfort for any condition. The reading aids are options to try. Keep the ones that help you.

## See also

- [Roadmap](roadmap.md)
- [star features not yet planned](star-gaps.md)
- [Troubleshooting](troubleshooting.md)
