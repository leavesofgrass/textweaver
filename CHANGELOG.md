# Changelog

All notable changes to textweaver. Versions follow [Semantic Versioning](https://semver.org/); before 1.0 every release is an alpha and anything may change.

## [Unreleased]

### Braille files

- **Emphasis in contracted braille.** Grade 2 BRF files now carry the UEB indicators for bold, italic, and underline, as grade 1 files do, placed around liblouis's contractions.
- **Passages over paragraphs.** Capitals or emphasis that go on over several paragraphs or list items open again at the start of each and end once, after the last (UEB Rules 8.5.5 and 9.9.1). Each heading stands alone.

### PDF files

- **Captions in six languages.** "Tabla 2:", "Abbildung 3.", "Tableau 1 -", "Tabela 4.", and "الشكل ٣:" are found as captions, as "Figure 3." and "Table 2:" were.

### State files, ready for sync

- **Bookmarks have ids.** Every bookmark gets a stable id, so bookmarks from two computers are matched by id, not by name. Existing bookmarks get one when their file is read, and keep their names and places.
- **Deletions are remembered.** Deleting a note, a highlight, or a bookmark leaves a small record, so a copy from another computer cannot bring it back. An edit made after the deletion still wins.
- **Replaced notes are kept.** When another computer's newer edit replaces a note's text, the older text is kept on this computer (the last 20 per document) and can be put back. The command for it comes with sync in the reader.
- **Longer ids.** New notes and highlights get 64-bit ids; older ids stay as they are.
- State files are now format 2. Every older file, and every Star import, loads unchanged.

### The library across computers

- **Continue reading.** A new command, in the File menu under Library and in the command palette, lists the documents on this computer with a reading place from any computer, newest first: "Cells, 42 percent, laptop, 2 hours ago". Enter opens one. `tw library --continue` prints the same list, with `--json` too.
- **Search by what your other computers know.** With sync on, a document's title, author, DOI, ISBN, and kind of file travel with it, so the library's filter and `tw library --search` find a document here by a DOI only another computer knew. The newest detail wins; the date it was first added keeps the earliest.
- **Statistics from every computer.** The statistics list and `tw stats` add every computer's reading of a document together, and show each computer's share on request (`tw stats --by-computer`).
- **The old progress file is still read.** With sync on, places go to the sync folder, and a library folder's `.textweaver/progress.json` (from an older textweaver, or converted from Star) is only read, so its places are still honored.

### Reading fonts

- **Lexend on first choice.** Choosing Lexend, in Settings or in the window's font list, when it is not installed asks first: "Download the Lexend font, 206 KB, SIL Open Font License? y or n". On a yes, its two files come from the Lexend project at a fixed version, each checked by its SHA-256, and are kept with the license in the data folder. The window uses it at once, and PDF and EPUB output find it by name (`tw convert --font lexend`). A no keeps the choice and uses another reading font. Lexend's license now ships with textweaver.

### Writing

- **Grammar checking is built in.** Ctrl+F7 and Ctrl+Shift+F7 (Alt+J for the fixes) now work in the terminal reader, the window, and the release packages, with no special build. It makes the programs about 11 MB larger. A build without the default features leaves it out.

### Speech

- **Speech follows the sound device.** When a headset or another device goes away, speech moves to the current default device at once and the reading goes on; before, it waited a second and only noticed while audio was waiting. Sound errors no longer print over the terminal reader's screen.
- **Choose the sound device.** `[speech] output_device` (Output device in the settings screen) keeps speech on one device by its id, and `tw backends --devices` lists the devices with their ids. A device that is not connected falls back to the default. It applies to Eloquence, the Windows voices, DECtalk, and Piper.
- **Piper's pitch sounds cleaner.** Raising or lowering a Piper voice's pitch now uses a band-limited resampler, so a raised voice no longer has a metallic edge.

### For contributors

- **Timing tests move the clock themselves.** The engine host's start deadline, stall timer, and sound-output reopen wait read a `Clock` that tests set by hand, and the flaky wall-clock checks (the slow-disk opening test, DECtalk's slow start) now check the order of events. The engine host's own test program and the Apple voice tests answer `--list` as cargo-nextest expects.
- **`cargo xtask regen` rebuilds every generated file** in one command: the third-party notices, the settings reference, the keyboard reference, the `docs/site` data, and the crate counts in the docs. `cargo xtask regen --check` changes nothing and reports each on one line, such as "keyboard: FAIL, out of date; run cargo xtask regen". `dev-check` runs it, so the settings reference, the docs indexes, and the notices are now checked locally as in CI.

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
- **The GUI ships in the release:** `textweaver-VERSION-windows-x86_64-gui.zip`, `textweaver-VERSION-macos-aarch64-gui.zip` (`textweaver.app`, Apple silicon), and for Linux x86_64 and aarch64 an AppImage and a tarball whose names end in `-gui`. Supported on Windows; on macOS and Linux built and checked automatically, not yet heard with a screen reader. Each is attested and in `SHA256SUMS.txt`. See [docs/install.md](docs/install.md#the-gui).
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

[0.1.0-alpha.6]: https://github.com/leavesofgrass/textweaver/releases/tag/v0.1.0-alpha.6
[0.1.0-alpha.5]: https://github.com/leavesofgrass/textweaver/releases/tag/v0.1.0-alpha.5
[0.1.0-alpha.4]: https://github.com/leavesofgrass/textweaver/releases/tag/v0.1.0-alpha.4
[0.1.0-alpha.3]: https://github.com/leavesofgrass/textweaver/releases/tag/v0.1.0-alpha.3
[0.1.0-alpha.2]: https://github.com/leavesofgrass/textweaver/releases/tag/v0.1.0-alpha.2
[0.1.0-alpha.1]: https://github.com/leavesofgrass/textweaver/releases/tag/v0.1.0-alpha.1
