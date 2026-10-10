# Reading and moving around

This guide covers the terminal reader, `textweaver`: opening a document, reading it aloud, and moving through it by sentence, paragraph, heading, and more. It is for anyone who reads with textweaver, with or without a screen reader.

Keys are the terminal defaults. Where the window uses a different key, this guide says so. The [keyboard reference](keyboard.md) lists every key in both frontends. Many keys are single keys, such as `h` for the next heading. Those are called browse keys. They work while you read, not while you edit or type in a prompt.

The browse keys follow the quick navigation keys of NVDA's and JAWS's browse mode: `h` for headings, `1` to `6` for heading levels, `l` for lists, `k` for links, and so on, with Shift for the previous one. They changed in 0.1.0-alpha.4. The [keyboard reference](keyboard.md#what-changed) lists every change, and `preset = "classic"` under `[keyboard]` brings back the earlier keys.

## Open a document

### From the command line

```bash
textweaver essay.md
```

`tw open` does the same thing, with the same options:

```bash
tw open essay.md
```

`tw open` refuses a file that does not exist ("no such file"). Plain `textweaver` with no file starts with no document open. It says: "No document is open. Press Ctrl+O to open one, Ctrl+N for a new one, or F1 for help." Keys named in messages are written this way on the status line; textweaver's own voice says them by name, such as "Control O" and "Alt period", so they are heard whatever the punctuation level.

### From inside the reader: Ctrl+O

Press **Ctrl+O**. The bottom line becomes a prompt called "Open file". Type the path and press **Enter**. Quotes around the path are removed, so a path copied with quotes works. A relative path is read from the folder you started textweaver in. Press **Escape** to cancel.

Rather not type it? Press **F4** in the prompt to choose the file in the [file browser](#from-the-file-browser-file-browse-files). See [Choose a path with F4](#choose-a-path-with-f4).

### From the library: Alt+L

Press **Alt+L** to list the documents in your library folders and the files you opened recently. Use **Up** and **Down** to move, and **Enter** to open one. The window uses **Ctrl+Shift+B**. The [library guide](library.md) explains library folders.

### From the file browser: File, Browse files

The file browser walks through folders and archives as one list, and opens what you choose. Open it from the File menu (**F10**, then File, then Browse files) or from the command palette: type `browse` and press **Enter**. It has no key of its own; give `browse_files` one in `keymap.toml` if you use it often. In the window it opens in the window's list dialog, from the File menu or the command palette, and the Say Status key previews the focused row there too.

It starts on **Places**: the open document's folder (focused), the folder you started textweaver in, your library folders, and your drives on Windows (the root folder on Linux and macOS). Each row says the name first, then what it is, so a Braille display shows the name in its first cells:

- "notes.md, Markdown, 12 KB"
- "Week 1, folder, 12 items"
- "course.zip, zip archive, 3.4 MB"

Your screen reader, or textweaver's voice, says where the row is after it: "notes.md, Markdown, 12 KB, 3 of 40".

The keys:

- **Up**, **Down**, **Home**, **End**, **Page Up**, and **Page Down** move.
- **Enter** opens a document, or goes into a folder or an archive. **Right** goes into a folder or an archive too.
- **Backspace** or **Left** goes up. Out of a folder or an archive, you land on its row, so Backspace twice from a folder inside `course.zip` brings you back to `course.zip`.
- Typing filters by name ("f, 1 item matches cour."). **Backspace** takes the filter back one letter at a time before it goes up.
- **Alt+End**, the Say Status key, says a preview of the focused row: a document's title and first sentence, an archive's count of files and the first names in it, a folder's full path and first names. The document is read in the background, so the keys never wait.
- **Ctrl+R** sorts: by name, then by date (newest first), then by size (largest first). Folders stay first.
- **Ctrl+A** shows every file, hidden ones included; press it again for readable files only. The introduction says how many files are hidden ("week1, 3 items. 1 file hidden.").
- **F1** says where you are and these keys.
- **Escape** closes the browser, and you are back where you were in your document.

**Archives** (zip, and tar, tar.gz, and 7z) open like folders, and folders and archives inside them open the same way, up to four archives deep. Nothing is unpacked onto your disk. A document inside an archive opens with a path such as `course.zip!week1/notes.md`, and its reading position, bookmarks, and notes are kept under that path, so they are there next time. Files that are never documents, such as `__MACOSX` and `.DS_Store`, are left out. An archive that is damaged, too large, or nested too deeply is not opened; you hear a sentence that says which.

**The browser only opens and chooses files.** It never copies, moves, renames, or deletes anything; **Delete** and **F2** say so.

**Choosing a folder.** Commands that need a folder, such as converting a folder of documents, open the same browser to choose it. They say what the folder is for when the browser opens. **Ctrl+Enter** chooses the focused folder, or the folder you are in. Some terminals cannot send Ctrl+Enter, so the first row of each folder, "Choose this folder", does the same with **Enter**. Folders inside archives cannot be chosen. In the window on a Mac the browser's keys use Cmd instead of Ctrl, and Cmd+Shift+Period shows every file, as in the Mac's own file dialogs.

### What you hear when a document opens

textweaver says "Opened", then the title. If you read this document before, it goes back to where you stopped and says so, for example "Opened Cells. Resumed at 42 percent." See [Your place is remembered](#your-place-is-remembered).

If the file cannot be opened, you hear "Could not open", the path, and the reason.

## Which files open

The reader opens these formats itself:

- Plain text: `.txt`, `.text`, `.log`.
- Markdown: `.md`, `.markdown`, `.mdown`, `.mkd`, `.mkdn`, `.mdwn`, `.mdtxt`, `.rmd`.
- HTML: `.html`, `.htm`, `.xhtml`, `.xht`.
- EPUB: `.epub`.
- Word: `.docx`, `.docm`, with comments as notes.
- RTF: `.rtf`.
- OpenDocument text: `.odt`, `.ott`, `.fodt`, with comments as notes.
- LaTeX: `.tex`, `.latex`, `.ltx`, with the files it includes from its own folder.
- Email: `.eml`, headers first, attachments listed.
- Web pages saved as one file: `.mhtml`, `.mht`.
- PDF: `.pdf`, with comments as notes, links you can follow, and filled-in form fields read label first; scanned PDFs through text recognition (below).
- Pictures of text: `.png`, `.jpg`, `.jpeg`, through text recognition.
- DAISY 3 books and DTBook: `.opf`, `.xml`, `.dtbook`, and a DAISY book in a zip.
- DAISY 2.02 books: open the book's `ncc.html`, or the zip it came in. The text is read, not the recorded audio.
- Braille files: `.brf`, `.brl`, read as print through liblouis (below), and each volume of a braille book in a zip.
- PowerPoint: `.pptx`, `.pptm`, `.ppsx`, `.potx`, with the speaker notes.
- Spreadsheets, as tables: `.csv`, `.tsv`, `.tab`, `.ods`, `.xlsx`, `.xlsm`, `.xlsb`.
- Archives: `.zip`, `.tar`, `.tgz`, `.gz`, `.7z`. Opening one lists the files inside that textweaver can read; `course.zip!week1/notes.md` opens one directly.

The [converting guide](converting.md#formats-textweaver-reads) says what is read from each format. A file with any other extension is read as plain text. A file that is not text at all is refused: a program, an audio file, or an old Word `.doc`. The message says what the file looks like, for example: "report.bin is not a text file; it looks like a program. textweaver cannot read it as text."

Org mode (`.org`), reStructuredText (`.rst`, `.rest`), and MediaWiki (`.wiki`, `.mediawiki`) files open directly, read by carta, a converter built into textweaver; see [Org, reStructuredText, and wiki markup](converting.md#org-restructuredtext-and-wiki-markup). The lean reader, built without its default features, reads them as plain text.

The reader does not use Pandoc. To read a Textile, DocBook, or other such file, convert it to Markdown first, then open the Markdown. `tw convert` uses Pandoc for these formats, so Pandoc must be installed:

```bash
tw convert essay.textile --to md
```

The [converting guide](converting.md) explains `tw convert`.

A scanned PDF is a picture of the pages. textweaver recognizes its text (OCR) and reads it like any other PDF, and says that it did, since recognized text can contain mistakes. English needs a one-time download, `tw ocr download`, which asks first; other languages need Tesseract. Until an engine can run, such a PDF reads as one sentence that begins "This PDF has no text layer" and says what is missing. See [Scanned pages](converting.md#scanned-pages-ocr).

### Braille files (BRF)

A BRF file (Braille Ready Format) is a braille book as plain text: each character stands for one braille cell, in the braille ASCII code that embossers and notetakers use, and the file is laid out in braille lines and pages, usually 40 cells by 25 lines. Libraries for blind and print-disabled readers distribute books this way; the braille downloads of the NLS BARD service, for instance, are BRF files in a zip, often one file per volume. Open the zip to see its volumes, and follow a link to open one, or open `book.zip!volume1.brf` directly.

textweaver reads a BRF file as print, so speech, search, notes and export work as they do for any book. It does this in two steps.

1. **It rebuilds the layout.** Braille transcribers follow conventions (BANA's *Braille Formats*, 2016), and textweaver reads them back: a centered line is a heading, a line beginning in cell 5 after a blank line is a subheading, a line indented two cells starts a paragraph, and a line at the margin continues the paragraph above, across a page if need be. The braille page number at the foot of each page is taken out of the text; each braille page becomes a page you can reach with "go to page" and that the title line names. A print page change (a line of dots 3-6 ending in a number) is read as "Print page 12", as a braille reader meets it. A running head repeated at the top of every page is read once.
2. **It translates the braille back to print** with [liblouis](https://liblouis.io/), the translator most braille software uses, in contracted or uncontracted braille. The code is set by **Braille code of BRF files** in Settings (`[braille] brf_code`): UEB (Unified English Braille, the default) for books produced since 2016, or EBAE (English Braille American Edition) for older American books. If a book reads oddly, with stray letters where words should be, it is probably in the other code: change the setting and open the file again.

The layout rules are heuristics, so poetry, tables and forms may come out with lines joined or split where the transcriber did not intend it. When a file has no layout textweaver can follow (for example, no indents and no blank lines at all), each braille line is read as its own paragraph instead. Mathematics in the Nemeth Code and computer braille are not translated reliably.

**Show original Braille** (in the View menu and the command palette) lists the lines of the braille page the cursor is on, exactly as the file has them, in Unicode braille. A Braille display shows them as the original cells, which is useful for checking a transcription, a mathematical expression, or a layout the print reading lost. Escape returns to the book.

**Without liblouis**, the file opens as braille: the same headings, paragraphs and pages, but each cell shown as a Unicode braille pattern, which a Braille display renders as dots. textweaver says so when the file opens, and how to fix it: install liblouis (from [liblouis.io](https://liblouis.io/) on Windows, or your distribution's `liblouis` package, which provides `lou_translate`, on Linux and macOS), then open the file again.

Only braille files that are distributed in the clear are read. Protected talking books, such as the audio books of NLS BARD, are not opened: textweaver does not touch any library's protection or terms.

## What the screen shows

The screen has four parts, from top to bottom.

1. **The title line.** It starts with "textweaver:" and the document's title. On the right it shows, in this order: the line and percentage ("line 12 of 300, 4%"), the reading state ("Ready" before you first read, then "Reading", "Paused", or "Stopped"), "modified" when there are unsaved edits, the accessibility mode in hybrid and screen-reader modes, the rate ("265 wpm"), and the speech engine. In edit and Speech Cursor modes the mode ("Edit" or "Speech Cursor") and "modified" come right after the percentage, before the reading state. On a narrow screen the last parts are left out first.
2. **The document.** The text, with the spoken word highlighted while reading.
3. **The status line.** It shows every announcement: what textweaver just said or would have said. It grows to as many rows as a long message needs, so nothing is cut before a screen reader or Braille display reads it. Screen readers read it as it changes. See [Using textweaver with a screen reader](screen-readers.md).
4. **The key hint line.** It shows a few useful keys for the current mode. While a list is open it shows the list's keys instead: Enter chooses, Escape closes, F1 says the list's keys, and in the menus Left goes back. When a prompt is open (Find, Go to, Open file, and so on), this line becomes the prompt, and you type there. In hybrid and screen-reader modes the hints are hidden by default; see [Braille-first layout](#braille-first-layout).

The terminal's cursor always sits where your attention is: on the word being spoken while reading, on the Speech Cursor line, on the prompt, on the chosen item of a list, or else on the reading cursor. Screen readers and screen magnifiers follow it.

Lists, such as the help, bookmarks, notes, and the library, appear in a box over the document when textweaver speaks for itself. In hybrid and screen-reader modes they cover the document with no box.

The terminal window's own title becomes the document's name when you open one, so Alt+Tab and your screen reader's "read title" key name it. Terminals that keep a title stack, such as xterm, get their old title back when textweaver closes.

### Braille-first layout

In hybrid and screen-reader modes, the screen is laid out for a Braille display, which shows one line of about 40 cells at a time. Each line starts with its meaning in the first cell:

- **The title line** starts with the position: "Line 12 of 400, 3%, Ready", then the other parts, then the document's title, without "textweaver:". In edit mode it reads "Line 212 of 400, Edit, modified, Ready, 51%": the percentage moves to the end, so "modified" is inside the first 40 cells even at three-digit lines. The window's status bar shows the same parts, also starting with a capital.
- **The status area** has a fixed height, big enough for the longest text textweaver puts there (600 characters), and at most half the screen. The document does not move up and down with each message, and a long line or paragraph is shown in full.
- **Line numbers** start at the left edge ("12  Text"), and the reading ruler marks its line with underline and bold only, with no mark in a column of its own.
- **The key hint line** is hidden, since F1, the keyboard shortcuts list (`?`), and the menus name the keys. `[display] hints = "on"` shows it again, starting at the edge. A prompt still uses that line.
- **The empty screen**, with no document open, starts its lines at the edge.

The keyboard shortcuts list (`?`) leads every row with the command's short name, then its key: "Find next, F3". The name always fits a 40-cell line, and so does the whole row for all but a few keys with two modifiers. With the Braille-first layout the row reads just like that; otherwise the key is drawn at the right edge.

- **Type to filter**, as in Settings. The list keeps the commands whose name, keys, group, or menu hold every word you typed, and says how many match: "12 of 226 commands match." Backspace takes letters off.
- **F1** on a row says what the command does, with all its keys: "Find next: F3 or Ctrl+G. Find the next match. Search".
- **Page Down** and **Page Up** move to the next and previous group (Reading, Navigation, Search, and so on). Entering a group says its name and how many commands it has first, then the row.
- **Enter** runs the command, and **Escape** closes the list.

Code blocks are drawn in the theme's code colors. When a block names its language (```` ```python ````), its keywords, strings, comments, numbers, and names get colors from the theme too, and the kinds differ by more than color: keywords are bold and comments italic. The text itself never changes. Moving the cursor onto the block's first line says its language, for example "code, Python".

## Read aloud

### Play and pause: Space

Press **Space** to start reading from the cursor. Press it again to pause. Press it once more to go on. Reading goes on from the last word you heard, so you may hear one word twice, but you never miss one. If you move the cursor while paused, reading goes on from the new place.

**Alt+P** does the same, and still works when single-key shortcuts are off. The window uses **Ctrl+Shift+Space**.

When textweaver pauses, it says "Paused."

### Read from the cursor: Enter

Press **Enter**, or **Ctrl+Space**, to read continuously from the cursor.

### Read the whole document: Shift+R

Press **Shift+R** to read from the very start.

### Stop: Escape

Press **Escape** to stop. textweaver says "Stopped." **Ctrl+X** also stops.

If nothing is being read and a search is active, Escape clears the search and says "Search cleared."

### Read one piece without moving

These keys read a piece of text where the cursor is. They do not move the cursor.

- **c**: say the character. Terminal chord **Alt+Shift+C**; window **Ctrl+Shift+C**.
- **w**: say the word. Terminal chord **Alt+Shift+W**; window **Ctrl+Shift+W**.
- **.** (period): say the sentence. Terminal chord **Alt+Shift+S**; window **Ctrl+Shift+E**.
- **Alt+Shift+L**: say the line. Window **Ctrl+L**.
- **,** (comma): say the paragraph.
- **v**: read the selected text. With nothing selected, you hear "No selection."

An empty line is read as "blank". Pressing **Space** right after one of these keys reads on from the cursor.

### Read again

- **;** or **Alt+;**: read again from the start of the current sentence.
- **r** or **Ctrl+R**: read again from the start of the current paragraph.
- **Shift+X**: repeat slower. textweaver says the current sentence again, 60 words per minute slower than your rate, then goes back to your rate. While reading on, reading continues after the sentence at the usual speed; otherwise it stops after the sentence. It is also **Repeat slower** in the Reading menu and the command palette. Your rate setting does not change.

### Stop at the end of a section

To read one section at a time, set **Stop at section end** in Settings (`[reading] stop_at`):

- **never** (the default): reading goes on to the end of the document.
- **next heading**: reading stops just before the next heading of any level.
- **next chapter**: reading stops just before the next chapter: a section break when the document has them, otherwise a level 1 heading.

When reading stops, you hear "End of section." and the key that goes on, such as "End of section. Ctrl+Space to go on." The cursor is on the next heading, so **Enter** in the document, or the read key, reads the next section, which stops at its end in turn.

With **Recall prompts** on in Settings (`[reading] recall_prompts`, off by default), the stop asks you to recall the section instead: "Say what you remember from Methods. Ctrl+Space to go on." With **Stop at section end** set to never, recall prompts stop at the next heading. [Study with textweaver](notes.md#study-with-textweaver) explains them and the self-test.

### Reading timer

To read for a set time, set **Reading timer** in Settings (`[reading] stop_after_minutes`) to a number of minutes; 0, the default, turns it off. When that much reading time has passed, reading finishes the sentence it is in and stops, and you hear "Time is up after 20 minutes." and the key that goes on. The cursor is on the next sentence.

Only reading time counts: pausing stops the clock, and resuming starts it again. Stopping with **Escape**, reaching the end of the document, or the timer running out starts the clock over at the next reading.

### Skim: reading passes, Shift+F

To get the gist of a long chapter before reading it properly, change the reading pass. Press **Shift+F**, or choose **Reading pass** in the Reading menu or the command palette. Each press moves to the next pass, and you hear its name:

- "Pass: first sentences." Reading says the headings and the first sentence of each paragraph, list item, and table row.
- "Pass: headings." Reading says only the headings.
- "Pass: full text." Reading says everything again.

In a skim, reading from the cursor or from the start begins by naming the pass, every time, so a skim is never taken for the whole text. The pass changes only reading on: the keys that read one sentence, paragraph, or line still read it in full. The pass lasts until you change it or quit; the next time you start textweaver, reading says the full text. (The `skim` speed preset is something else: a reading speed.)

## Move around

Every move says where you arrived. When you are not reading, you hear a short preview of the text, up to eight words. While reading, textweaver jumps and keeps reading from the new place; the preview goes to the status line only, because the reading itself is what you hear.

When there is nothing more to move to, you hear, for example, "No next heading." or "End of document."

By default, moves stop at the ends of the document. To wrap around to the other end, set this in `settings.toml`:

```toml
[reading]
wrap_navigation = true
```

A wrapped move starts with "Wrapped." Find, bookmarks, and notes always wrap, whatever this setting says.

### Sentences

- **Alt+Down** or **Alt+.**: next sentence, as in JAWS.
- **Alt+Up** or **Alt+,**: previous sentence. If you are more than three words into a sentence, this goes back to its start instead.

Windows Terminal moves between panes with **Alt+Down** and **Alt+Up**; **Alt+.** and **Alt+,** always work. See [the screen reader guide](screen-readers.md#windows-terminal-keys-that-clash).

### Paragraphs

- **p**, **]**, **Ctrl+Down**, or **Ctrl+P**: next paragraph.
- **Shift+P**, **[**, or **Ctrl+Up**: previous paragraph. The window also has **Ctrl+Shift+P**.

### Headings

There are two kinds of heading keys. One reads from the heading. The other only moves.

- **>**: read from the next heading. The window also has **Ctrl+H**.
- **<**: read from the previous heading. The window also has **Ctrl+Shift+H**.
- **h**, **}**, or **Alt+H**: move to the next heading without reading.
- **Shift+H**, **{**, or **Alt+Shift+H**: move to the previous heading without reading.
- **1** to **6**: the next heading at that level. **Shift** with the digit: the previous one. textweaver matches the digit key itself, so this works on any keyboard layout; see [the keyboard reference](keyboard.md#terminal-notes).

You hear the heading level and text, for example "Heading level 2: Methods". **Alt+H** and **Alt+Shift+H** are chords, so they also work in edit mode and with single-key shortcuts turned off. They are terminal keys; the window has **Ctrl+H** and **Ctrl+Shift+H**.

### The outline: Alt+O

Press **Alt+O** for a list of the document's headings, in order, each with its level: "Methods, level 2". You hear how many there are and which heading you are under: "Outline, 12 headings. Type to filter, Enter goes to a heading, Escape closes. You are under Methods."

- Type part of a heading to filter the list. Only the headings that hold every word you type stay, and you hear how many match. **Backspace** removes a letter; Space is part of the filter.
- **Up**, **Down**, **Home**, and **End** move through the list.
- **Enter** goes to the heading. It is a jump, so **Alt+Left** comes back.

The outline works while reading and while editing; in edit mode it lists the headings as you have written them so far.

### Tables, lists, links, and more

- **t**: next table. **Shift+T**: previous table. The window also has **Ctrl+T** and **Ctrl+Shift+T**.
- **l**: next list. **Shift+L**: previous list.
- **i**: next list item. **Shift+I**: previous list item.
- **k** or **u**: next link. **Shift+K** or **Shift+U**: previous link.
- **q**: next block quote. **Shift+Q**: previous block quote.
- **s**: next separator (a horizontal rule). **Shift+S**: previous separator.
- **g**: next graphic (an image, read by its alt text). **Shift+G**: previous graphic. A picture with no description at all is not skipped: you hear "Graphic, no description", and reading says "graphic, no description" where it is. A picture marked as decorative (`alt=""` in a web page) stays silent.
- **d**: next section or chapter. **Shift+D**: previous one. See [Chapters](#chapters).

A table is announced with its row count ("Table, 3 rows"). A list is announced with its item count ("List, 5 items"). A numbered list item is read with its number ("3. Buy milk"). A nested item says its level ("List item, level 2").

### Inside a table: Ctrl+Alt and the arrows

In a table, move the way screen readers do:

- **Ctrl+Alt+Down** and **Ctrl+Alt+Up**: the next or previous row, in the same column.
- **Ctrl+Alt+Right** and **Ctrl+Alt+Left**: the next or previous cell in the row.

Each move says the column's header with the cell, for example "Row 3, Age: 41" for a new row and "Age: 41" within a row. On the header row you hear "Header row" and the header. At the edges you hear "End of table.", "Start of table.", "End of row.", or "Start of row." At high verbosity each move also says "Row 3 of 5, column 2 of 4". Where am I (below) says the row and column too. Outside a table these keys say "Not in a table."

Some desktops keep **Ctrl+Alt** with the arrows for switching workspaces; give the four commands other keys in `keymap.toml` if yours does (`table_next_row`, `table_previous_row`, `table_next_column`, `table_previous_column`).

### Follow a link or a footnote: Alt+Shift+F

Put the cursor on a link and press **Alt+Shift+F**:

- A link to a heading in the same document, such as `[see Methods](#methods)`, goes to that heading.
- A link to another file, such as `[chapter 2](chapter-2.md)` or `[part](notes/a.md#part-two)`, opens it (at the heading, when the link names one). An Obsidian wiki link such as `[[chapter 2]]` finds `chapter 2.md` in the folder or below it. You hear "Followed the link to chapter-2.md. Back: Alt+Left." Press **Alt+Left** before jumping anywhere else in the new file, and you are back at the link in the first file.
- A web or mail link is said, with a question: "Web link: https://example.org. Open it? y or n." Press **y** to open it in your browser.

On a footnote reference, **Alt+Shift+F** goes to the note; on the note, it goes back to the reference. When footnotes are read in place (the default, `[normalization] footnote_mode = "inline"`), it says the note instead.

**Alt+Shift+K** says a link's address without following it.

### Chapters

- **d**, **F11**, or **Alt+PageDown**: next chapter.
- **Shift+D** or **Alt+PageUp**: previous chapter. More than five words into a chapter, this goes back to its start instead.

Chapters are the book's sections when the document has them (EPUB chapters, Word sections, PDF bookmarks). Otherwise they are the level-1 headings. A document with neither says "This document has no chapters." Some terminal programs keep F11 for themselves; the Alt chords always work. The window uses only the Alt chords.

### Start and end

- **Home** or **Ctrl+Home**: the start of the document ("Top of document").
- **End** or **Ctrl+End**: the last word of the document ("End of document").

### Pages and scrolling

- **PageDown** and **PageUp**: move one screen, less four lines. You hear "Page, line", the line number, and a preview.
- **j** and **Shift+J**: scroll the view down or up one line without moving the cursor.

### The cursor keys

The arrow keys move the reading cursor. They stop any reading first.

- **Right** and **Left**: the next or previous word. You hear the word.
- **Down** and **Up**: the next or previous line that has text, keeping close to the same column. You hear the whole line.

### Select text

Hold **Shift** with the arrow keys to select.

- **Shift+Right** and **Shift+Left**: extend the selection by a word.
- **Shift+Down** and **Shift+Up**: extend it by a line.

You hear the text that was added, then "selected", or the text that was removed, then "unselected". At low verbosity you hear only the text. Some terminal programs keep **Shift+Up** and **Shift+Down** for scrolling their own window.

Press **v** to hear the selection. Notes and highlights use the selection too; see [Bookmarks, notes, and highlights](notes.md).

## Speech Cursor mode: line by line

Speech Cursor mode reads exactly one line at a time. Use it to check a list, a table, code, or a poem line by line.

A line here is a line of the document's text, not a row on the screen. A long paragraph is one line, however it wraps. List items and table rows are each on their own line.

### Turn it on: Tab

Press **Tab**. textweaver reads the line the cursor is on. The status line shows: "Speech Cursor on, line", the number, then "Up and Down read lines, Enter reads on, Tab or Escape leaves."

### Keys in Speech Cursor mode

- **Down** or **j**: read the next line.
- **Up** or **k**: read the previous line.
- **r**: read the current line again.
- **PageDown** and **PageUp**: move to the next or previous paragraph and read its first line.
- **Enter**: leave the mode and read on from this line.
- **Tab**: leave the mode. textweaver says "Speech Cursor off."
- **Escape**: stop and leave. textweaver says "Stopped. Speech Cursor off."

An empty line is read as "blank". The mode never wraps: at the first or last line you hear "Top of document." or "End of document."

Other moves still work. A heading, find, or bookmark jump moves the Speech Cursor to the line of the target and reads that line. When you leave, the cursor goes to the first word of the line.

## Find text

### Start a search: Ctrl+F or /

Press **Ctrl+F** or **/**. Type what to find and press **Enter**.

- Search ignores case, unless you turn on **Match case** in the search options (below).
- To search with a regular expression, write it between slashes, for example `/colou?r/`, or turn on **Regular expression** in the search options.
- The search starts at the cursor. When there is no match after it, it wraps to the top.
- A regular expression that is not valid is said in words, with the character where it goes wrong, for example "Invalid pattern at character 3: unclosed group." Nothing is searched.

You hear the match number, the count, and the line, for example "Match 2 of 5:" followed by the text of that line. With no match, you hear "No matches for", then your text.

### Next and previous match

- **F3** or **n**: next match.
- **F4** or **Shift+N**: previous match (**Shift+F3** in the window).

At the end, the search wraps and says "Wrapped to top." or "Wrapped to bottom." If you have not searched yet, these keys open the Find prompt. The window uses **F3** and **Shift+F3**, as other Windows programs do, besides **n** and **Shift+N**.

### Search options

**Search options**, in the Edit menu under Find and in the command palette, is a short list of four switches that Find and Find and replace share. Press **Enter** on one, or its letter, to turn it on or off; you hear its new state, and the list stays open until **Escape**.

- **Match case** (**c**): off by default, so `cat` also finds `Cat`.
- **Whole words only** (**w**): on, `cat` does not find `catalog`.
- **Regular expression** (**x**): the text to find is a regular expression, such as `colou?r` or `\d+`. `^` and `$` match at the start and end of a line, and `\n` or `\s` can match a line break.
- **Across lines** (**l**): with a regular expression, `.` matches a line break too, so a match can run on from one line to the next.

The options last until you quit, and all start off. When one is on, the Find prompt says so as it opens, for example "Find. Options on: regular expression."

### Clear the search

Press **Escape** when nothing is being read. You hear "Search cleared."

`tw search` searches a file from the command line, with options for case and whole words:

```bash
tw search essay.md "mitochondria" --whole-word
```

## Go to a line, a page, or a percentage: Ctrl+G

Press **Ctrl+G**. In a document with pages (a PDF, or any other paged format), the prompt says "Go to page, or line 12, percent, start, or end"; otherwise it says "Go to line, percent, start, or end". Type one of these and press **Enter**:

- in a paged document, a plain number, such as `12`, which is a page (`line 12` is still a line);
- a page by its printed label, `page 12` or `p 12`, in any document with pages; a number that matches no printed label is the nth page; the printed label wins, so `p 1` is the page printed "1" even after pages numbered i to x;
- a line number, such as `line 42` (or a plain number in a document with no pages);
- a percentage, such as `50%` or `50 percent`;
- a character position, counted from 0, such as `char 120` or `character 120`;
- `start`, `top`, `beginning`, or `begin`;
- `end` or `bottom`.

You hear the page or the percentage, the line, and a preview. Anything else gives, in a paged document: "Not a go-to target:", your text, then "Type a page number, line and a number, a percentage such as 50%, start, or end." Elsewhere: "Type a line number, a percentage such as 50%, start, or end."

Line numbers are lines of the document's text, as in Speech Cursor mode. Press **F6** to show them on screen.

### Pages in a PDF

A PDF, and any other paged format, carries its printed page labels. That includes a DAISY book, a slide deck, and an EPUB or web page that marks its print page numbers, as most textbooks from accessible-format publishers do: the numbers are not read aloud, but **Ctrl+G** then `p 112` goes to print page 112. **Say Position** (**Shift+W**) and the title line name the page first: "Page 12 of 30." When the document has no headings, the outline (**Alt+O**) lists its pages instead: "Page 12: its first words," one per page.

## Citations while reading

Citations in Pandoc's style, such as `[@doe2020, p. 12]`, `[see @doe2020; @roe2021]`, `[-@doe2020]`, and `@doe2020 [p. 3]`, are read from your reference library (see [Citations](citations.md)).

- **By default, continuous reading skips citations.** Reading aloud, reading from the cursor, saying a sentence or paragraph, and "listen to the rendered text" pass over a bracketed citation as if it were not there. An in-text citation, such as `@doe2020 argues`, is part of the sentence, so its authors are said: "Doe and Roe argues".
- **Alt+Shift+Q** turns citations on or off, and saves the choice. You hear "Citations on." or "Citations off." While reading continuously, reading goes on from the word you were on, with the new setting. It is also `toggle citations` in the command palette.
- **With citations on**, each is said in words: "Doe and Roe, 2020, page 12". A key that is in no library is read as the key.
- **Word moves** (Right and Left) and **Alt+Shift+K** say a citation in words whatever the setting: "Citation: Doe and Roe, 2020, On reading, page 12."

The highlight stays exact either way: a skipped citation is never highlighted, and while a citation is said in words the whole citation is highlighted. Which `@` marks count as citations follows the converter's rule for its usual Markdown flavor: a bracketed citation when one of its keys is in a library, and an in-text one when all its keys are. Citations in code and math are never touched. In `settings.toml`:

```toml
[reading]
citations = "off"    # or "words"
```

## Explore math: Alt+Shift+X

Put the cursor on a formula, such as `$\frac{a+b}{2}$`, and press **Alt+Shift+X**. You hear the whole expression, then move through it:

- **Right** and **Left**: the next or previous term at this level.
- **Down**: into the part: a fraction's numerator, a script, a root, the inside of brackets.
- **Up**: back out.
- **Home** and **End**: the first and last term at this level.
- **Space** or **Enter**: say the part again.
- **Escape**: leave. Any other key leaves too, and does what it usually does.

Each step says the part and its role, such as "numerator, a plus b", and highlights it; the cursor moves there. At an edge you hear "Last term.", "First term.", "No parts inside.", or "Whole expression." How much is said follows `[normalization] math_verbosity`. See [Math](math.md).

## Summaries

A summary is a short list of the sentences that best stand for a text: the ones that share the most words with the rest of it. textweaver picks them itself, on your computer, with no model and no download (the method is LexRank; see [ADR-0037](adr/0037-extractive-summaries.md)). The sentences are the document's own, word for word, in the order they appear.

**In the reader**, open the command palette and choose **Summarize** (it has no key of its own; you can give it one in the key settings). What it summarizes:

- the selection, when you have selected text;
- else the chapter you are in, when the document has chapters (the same chapters Next Chapter moves between);
- else the whole document.

You hear, for example, "Chapter summary, 5 sentences. Enter goes to the sentence and says it." Then the list works like the others: Down and Up say "2 of 5" and the sentence, and **Enter** moves the cursor to that sentence and says it. Escape closes the list.

Headings, tables, code, and footnotes are never summary sentences, and neither are sentences of fewer than four words. A text with none left says "Nothing to summarize". Very long texts (more than about 600,000 characters, some 100,000 words) are read in samples spread evenly through them, so a summary still takes a fraction of a second; the list's introduction then says "from samples of this long text".

**From the command line**, `tw summarize` prints the sentences one per line, with nothing before them:

```sh
tw summarize essay.md
tw summarize book.epub --sentences 10
tw summarize notes.docx --json
```

`--json` adds each sentence's position, line number, and score. How many sentences both give is `[summary] sentences` in `settings.toml`, 5 unless you change it (1 to 50):

```toml
[summary]
sentences = 7
```

The stop words the method leaves out ("the", "and", "of") are English. Documents in other languages still get a summary, a little less sharp.

## Tracked changes and comments: Ctrl+Shift+J or Alt+A

A Word document (and an OpenDocument or RTF file) can carry tracked changes, the edits a reviewer made with Track Changes on, and comments. textweaver reads the final text by default; `[reading] revisions` in the settings chooses whether the changes are also said in place ("(inserted by Ada Example: renal)").

**The changes list.** Press **Ctrl+Shift+J** in the window or **Alt+A** in the terminal reader, or choose **Changes and comments** from the Bookmarks menu or the command palette. Every change and every comment thread is one row, in document order, with what it is first:

- "Inserted: 'renal', by Ada Example, Tuesday, March 3, 2026"
- "Deleted: 'rarely', by Bo Example, date not recorded"
- "Moved here: 'check the labs first', by Ada Example, Thursday, March 5, 2026"
- "Comment by Bo Example: check this date, 1 reply, resolved"

The date is the one the document gives, said in full. When the document gives none, the row says "date not recorded"; textweaver never guesses one. A move is two rows, "Moved away" where the text was and "Moved here" where it went. The two halves are decided together, as Word decides them: accepting either one keeps the text in its new place, and rejecting either one returns it to where it was.

**Keys in the list:**

- **Enter** goes to the change or comment and says its line.
- **A** accepts the change, **R** rejects it.
- **Shift+A** and **Shift+R** accept or reject every change by the same author.
- On a comment: **F2** replies, **Space** resolves it or opens it again, **Delete** deletes it and its replies (after a y or n question).
- **N** adds a comment to the selection, or to the sentence at the cursor.

To accept or reject every change at once, use **Accept all changes** or **Reject all changes** from the Bookmarks menu or the palette. Each asks once before it acts ("Accept all 12 changes? y or n"), as Replace all does, because a whole review is a large thing to undo; press **n** and nothing changes. **Add comment** adds a comment without opening the list.

Accepting an insertion keeps its text; rejecting it removes the text. Accepting a deletion removes the text; rejecting it puts the text back. The document you are reading changes at once, so reading, search, and the study tools see the result. Comments are notes too: a reply, a resolve, or a delete shows in the notes list as well. Replies and new comments carry the name in `[authoring] author` and the date from the clock. The setting is empty until you fill it, and textweaver never takes a name from your computer or your account; while it is empty, what you add is signed "textweaver". (Settings files that still have the older `[editing] author` keep their name: it is read as `[authoring] author`.)

**Saving to the Word file.** Accepting and rejecting change the document you are reading, not the file. To write your decisions into the Word file itself, choose **Save changes to the Word file** from the Bookmarks menu or the command palette. textweaver then edits the original `.docx` in place rather than writing a new one, so its styles, numbering, headers, and everything else textweaver does not read stay exactly as Word left them. Accepted insertions and rejected deletions become ordinary text; accepted deletions and rejected insertions disappear; and formatting changes and deleted paragraph breaks, which the list does not show, follow your decisions once every change has been decided the same way. Replies, resolved marks, deleted threads, and new comments go into the file's comments, where Word shows them as a thread.

Before the first save, textweaver keeps a copy of the original beside it, named so that it says what it is: `report.docx` is copied to `report-original.docx` (or `report-original-2.docx`, if that name is taken). You hear "Saved the changes in report.docx. The original is kept as report-original.docx." Later saves in the same session write only the file. Saving works for `.docx` files; an OpenDocument or RTF file's changes appear in the list and can be decided, and export writes the result in another format. Open the saved file in Word to confirm the result: the Review tab should show no tracked changes once every change has been decided.

Changes stay as they are in edit mode: leave edit mode to accept or reject them.

**From the command line**, `tw changes` prints the same rows, one per line:

```sh
tw changes draft.docx
tw changes draft.docx --json
tw changes draft.docx --accept-all --out final.md
tw changes draft.docx --reject-all --out original.docx
tw changes draft.docx --accept-all --in-place
```

`--json` prints each change (its kind, text, author, date, and position) and each comment thread as the document records them. `--accept-all` or `--reject-all` with `--out FILE` writes the document with every change decided, in the format the file name's extension names: Markdown (`.md`), plain text (`.txt`), HTML, or `.docx`, `.epub`, `.pdf`, and `.brf`; the original file is left alone. With `--in-place` instead, the Word file itself is changed, exactly as Save changes to the Word file does it, after the original is copied to `draft-original.docx`.

## Go back and forward

textweaver keeps a history of your jumps, like the Back button of a web browser.

- **Backspace** or **Alt+Left**: go back to where you were before the last jump.
- **\\** (backslash) or **Alt+Right**: go forward again.

You hear "Back, line", the number, and a preview. With nothing to go back to, you hear "No earlier history."

There is one rule for what counts as a jump. Every move by a sentence or anything larger records the place you left, once. That includes paragraphs, headings, tables, lists, list items, links, chapters, finds, go to, the start or end of the document, bookmarks, and notes. Moves by word, line, or page, scrolling, and Speech Cursor line moves are not recorded. Going back and forward are not recorded either.

The history keeps the last 50 places. Change that with `[reading] nav_history_size`. It is saved with the document, so it is still there when you open the document again.

## Document overview

Choose **Document overview** in the Say menu (under Reading) or the command palette. You hear the title, then how many headings, tables, pictures, and footnotes the document has, then about how long reading the rest takes at your current rate: "Cells. Headings: 12, tables: 3, pictures: 4, footnotes: 21. About 38 minutes left." The time counts from the cursor.

## Where am I: Shift+W or Alt+Shift+Y

Press **Shift+W**, or **Alt+Shift+Y** from any mode. You hear the line, the number of lines, and the percentage, for example "Line 12 of 300, 4 percent." In a PDF, or any other paged format, the page comes first: "Page 12 of 30. Line 400 of 2000, 20 percent." In a table you also hear where in it: "Table, row 2 of 5, column 3 of 4." At normal verbosity you also hear the word number and the heading above you: "Under heading Methods." It ends with the time left at your current rate: "About 3 minutes left." The window's status bar shows the same time left, and changes it once a minute. At high verbosity you also hear the document's title and the mode, when it is not plain reading. This works in edit mode too, on the headings as you have written them.

## Hear it again: ' and z

- **Repeat message**: **'** (apostrophe), or **Alt+'** from any mode, says the last message again, as the status line shows it.
- **Say status**: **z**, or **Alt+End** from any mode, says the last message, then the status the title line shows: the mode, whether the document is modified, "Ready", "Reading", "Paused", or "Stopped", the line and percentage, the accessibility mode, the rate, and the speech engine. In an open list it says the list's introduction and the item you are on instead.

Both are heard over the reading, which then goes on. They are in the command palette as "say status" and "repeat message". With a screen reader, its own "read current line" key (NVDA+Up, Insert+Up in JAWS) reads the status line too.

## Your place is remembered

textweaver saves your place when you quit, when you open another document, and every 30 seconds while your place changes. When you open the document again, it goes to the first word at or after the saved place and says "Resumed at", then the percentage.

To always start at the top instead, set:

```toml
[reading]
auto_resume = false
```

If the document is in a library folder that another computer also uses, the place may come from that computer. The [library guide](library.md) explains how.

If the file changed outside textweaver, in Obsidian, git, or another editor, your place, bookmarks, notes, and highlights are found again from the text they were on, and textweaver says once what moved and what it could not find. See [How marks move when you edit](notes.md#how-marks-move-when-you-edit).

`tw marks` shows the saved place of a document without opening it:

```bash
tw marks essay.md
```

## The reading highlight

While textweaver reads, the spoken word is highlighted on screen. The highlight never relies on color alone, so it shows even with color turned off.

These settings are in the `[highlight]` section of `settings.toml`:

- `enabled` (default `true`): highlight the spoken text at all.
- `granularity` (default `"word"`): `"word"` highlights the word, `"sentence"` the whole sentence, and `"both"` the sentence with the word inside it.
- `lead_words` (default `1`): where the highlight sits, from -5 to 5. At 1 it is on the word you hear. At 2 it runs one word ahead; at 0, one word behind. It moves only the drawn highlight, never your saved place.
- `speed` (default `1.0`): from 0.5 to 1.5. It speeds up or slows down the highlight for engines that do not report words, where textweaver estimates the timing.

- `color` and `sentence_color`: the colors of the word and sentence highlight, such as `"#ff8800"` or `"yellow"`. The terminal reader draws them over the [color theme](themes.md) and warns when one does not stand out from the text.

If the highlight runs ahead of or behind the voice with an engine that does report words, change `[speech] latency_offset_ms`. The [speech guide](speech.md) explains it.

For more help reading, such as one word at a time (RSVP), bionic reading, and a reading ruler, see [Reading aids](reading-aids.md).

## Faster, slower, and the voice

- **+** or **=**: faster. **-**: slower. Each step is 20 words per minute. The window also has **F11** and **Shift+F11**; there, **Ctrl+=** and **Ctrl+-** change the text size.
- **F8**: cycle the speed presets: skim, normal, study, slow.
- **Alt+V**: choose a voice. The window uses **Ctrl+Shift+V**.

The [speech guide](speech.md) covers rate, pitch, volume, and voices.

## Announcements and verbosity

textweaver tells you about every change: a move, a mode, a setting. How much it says is set by `[speech] verbosity` (below), and how much it says about itself by the interface announcements (the next section). `[speech] verbosity`:

```toml
[speech]
verbosity = "normal"
```

There are three levels. Each one says everything the level below it says.

- `"low"`: only what you need. Errors, the ends of the document, the text you moved to, and answers to what you asked (your position, the rate, a search result). A move to a heading says just the heading text: "Methods".
- `"normal"` (the default): adds structure names and state changes. The same move says "Heading level 2: Methods". You also hear "Paused.", "Stopped.", "Cancelled.", and "Speech Cursor off."
- `"high"`: adds the line and percentage to every move, and the title and mode to Where am I. The same move says "Heading level 2, line 12, 4 percent: Methods". You also hear "Done reading." when the document ends.

Announcements never interrupt reading, except errors. While reading, they go to the status line.

Change the level while textweaver runs with **Alt+Shift+V**: low, normal, high, then low again. You hear the new level ("Verbosity: high."), and it is saved.

**Alt+Shift+N** does the same for how much punctuation is spoken (`[speech] punctuation`): none, some, all. The change is heard at once and saved.

### Interface announcements: Ctrl+F9

Your screen reader and Braille display already tell you when a list opens or a dialog closes. The interface announcements decide how much textweaver says about itself on top of that: lists and menus opening and closing, progress, hints, tips, and confirmations of routine changes. Reading speech and math are not affected.

- **Off**: only errors, questions waiting for your answer, and answers to what you asked (where you are, a count, the item you moved to).
- **Minimal**: also the result of each command you run ("Bionic reading on", "Saved", the name of a list or menu you opened).
- **Normal**: also routine confirmations, lists and dialogs closing, progress at a calm pace (at most every ten seconds), and tips.
- **Full**: also hints about keys and counts in progress.

An error, a question, and the answer to something you asked are said at every level. **Ctrl+F9** cycles off, minimal, normal, full, and the change is always said. With `automatic`, the default, it is minimal with a screen reader (screen-reader and hybrid modes) and normal when textweaver speaks for itself. It is also View, Interface announcements in the menus, and in `settings.toml`:

```toml
[accessibility]
interface_announcements = "minimal"
```

## Spelling

**Alt+M** moves to the next misspelled word and **Alt+Shift+M** to the previous one, while reading or editing. You hear the word, then its letters: "recieve. r e c i e v e." **Alt+J** lists suggestions. The [editing guide](editing.md#spelling) has the details.

## Define a word: Ctrl+Shift+D or Alt+E

**Ctrl+Shift+D** in the window, or **Alt+E** in the terminal, defines the word at the cursor, or the words you selected (`ice cream`). textweaver looks in your own glossary first, then in Open English WordNet, and says how the word is pronounced, from the CMU Pronouncing Dictionary. Everything is on your computer: nothing goes to the internet.

The senses come up in a list. The first item is the pronunciation, respelled with the stressed syllable in capitals ("Pronounced RUN-ing."). Each sense then says its headword, its part of speech, which sense it is, the definition, an example, synonyms, opposites, and what it is a kind of:

> dog, noun, 1 of 7: a member of the genus Canis (probably descended from the common wolf)... For example: the dog barked all night. Synonyms: domestic dog, Canis familiaris. A kind of: canine, domestic animal.

Up and Down move through the senses. Enter copies one to the clipboard, for a note. Escape closes the list.

- Inflected words find their base forms: `running` finds `running` and `run`, `geese` finds `goose`, and `wider` finds `wide`.
- With no word at the cursor, or no document open, textweaver asks which word to define. The command palette's `define word` does the same.
- **Your glossary** is a text file with one `term: definition` line per sense, or a JSON file in star's format. Put it in the settings folder as `glossary.txt`, or name it with `[lexicon] glossary` ([settings.md](settings.md#lexicon)). Its senses come first, before WordNet's. An edited glossary is read again the next time you define a word.
- `tw define WORD` does the same from the command line, as Markdown or, with `--json`, as JSON.
- The dictionary is a 10 MB file, `lexicon/lexicon-en.twlex`, installed beside the program. If it is missing, textweaver says so and searches your glossary only.

## Reading statistics: Ctrl+Shift+Y or Alt+Y

textweaver counts the time it spends reading each document aloud, the furthest point you reached, and the sessions: each time you open a document and read it. **Ctrl+Shift+Y** in the window, or **Alt+Y** in the terminal, lists:

- the total time read, over how many sessions and documents;
- this document's time, furthest point, and sessions;
- the ten documents you read most (Enter opens one);
- whether statistics are on (Enter turns them off or on).

Statistics are saved every 30 seconds while reading, and when a document closes. `tw stats` prints them, `tw stats --json` prints everything, and `tw stats clear` removes them. In the reader, the last item of the list, "Remove the reading statistics", removes them too, after a yes or no; it is there once something is recorded. To stop recording, turn them off in the list or set `[stats] enabled = false` ([settings.md](settings.md#stats)). `tw migrate-star` brings star's reading statistics over.

## The menus: F10

Press **F10** to open the menus: File, Edit, View, Reading, Speech, Tools, and Help. Every command is in them, with its keys. In the terminal they are a list, place first on the Braille display: "Menus, 1 of 7, File".

- **Up** and **Down** move; **Enter** or **Right** opens a menu or runs a command.
- A letter moves to the item with that access key, as in a Windows menu: **x** in the File menu lands on "Export as".
- **Left** or **Backspace** goes back up; **Escape** closes the menus, and you are where you were.
- Each item says its name, then its state for a switch ("Bionic reading, checked") or its value ("Reading ruler: current line"), then its keys ("Open, Ctrl+O").
- **F1** says where you are and these keys.

Some terminal programs keep F10 for themselves; then type `menu` in the command palette.

File, Recent documents lists the last documents you opened, with your place in each ("essay.md, 43 percent"); the digits 1 to 8 choose them. File, Settings has the settings screen, Colors, Profiles, and Export settings and Import settings.

## What does this key do: Shift+F1

Press **Shift+F1**, then any key: you hear what the key does, its keys, and where the command is in the menus ("Export PDF: Export the document as a tagged PDF next to it. Keys: the command palette. In the menus: File, Export as, Export PDF."). The key is not run.

## The Help menu

- **Search help** searches all of help at once; see [Search help](#search-help) below.
- **Quick start** opens the quick start guide packaged with textweaver as a document. When there is none beside the program, it offers the online page.
- **Documentation** opens the complete user guides that come with textweaver, as documents in textweaver itself, so they work without a network connection. You start on the documentation index; its headings move as in any document, Follow link (**Alt+Shift+F**) on a link opens that guide in place, and **Backspace** or **Alt+Left** brings you back to where you were. If a guide the index lists is missing from your copy of textweaver, you hear which one ("Guide missing: dictation.md. The other guides open."); the rest of the documentation still opens. A copy of textweaver built from source reads the guides in the repository's `docs` folder.
- **Online documentation** and **Report a problem** say their web address and ask "y or n" before a browser opens. Nothing is sent from textweaver; you write the report yourself.
- **About textweaver** lists the facts a problem report needs, one per line: version, build, license, the speech engine in use and those found, how many optional components are installed, and the settings, data and cache folders. Include them in your report.

## The command palette: F2

Press **F2** to run any command by name. **Alt+X** and **:** open it too. The window uses **F2** and **:**. You hear "Command. Type part of a name; Tab completes, Up and Down list matches." (at low verbosity, just "Command"); the bottom line shows "Command".

1. Type part of a command's name, such as `next head`, or its first letters: `ep` finds Export PDF.
2. Press **Tab** to complete it. You hear how many commands match and the first few names.
3. Or press **Down** and **Up** to go through the matching commands. Each is said as its short name, then its key: "Find next, F3".
4. Press **Enter** to run it.
5. Or press **Ctrl+L** to hear the matches as a list, and choose one with **Enter**. In the list, **F1** says what the focused command does.

With nothing typed, the commands you ran last from the palette or the menus come first, each said as "recent".

Matches are ordered: the exact name, names that start with what you typed, names whose words start with your letters (`ep`, or `exp pd`), names with your letters in order, then commands whose help has every word you typed. Names match in your interface language and in English.

An unknown name gives "Unknown command:" and your text.

These keys work in every prompt, including Find, Go to, and Open file:

- **Up** and **Down**: earlier answers to the same prompt (except in the palette, where they go through the commands). The last 50 are kept until you quit. Find and the "find what" of Find and replace share their history.
- **Ctrl+A** and **Home**: to the start. **Ctrl+E** and **End**: to the end.
- **Ctrl+U**: delete to the start. **Ctrl+K**: delete to the end. **Ctrl+W**: delete the word before the cursor.
- **Escape** or **Ctrl+G**: cancel.

### Choose a path with F4

Every prompt that asks for a file has **F4**, the browse key: Open file, Save as, Image file (Insert image), Import references, Import settings, Export settings, and importing and exporting profiles. When the prompt opens you hear its name and the key, such as "Import settings from file. F4 to browse."

1. Press **F4**. The file browser opens on the places, saying what it is for: "Choose the file: Import settings from file". It lists folders and only the files the prompt can use (settings files are TOML and JSON, images are PNG, JPEG, GIF, SVG, WebP and BMP, references are BibTeX, RIS and CSL-JSON).
2. Move with the browser's usual keys, and press **Enter** on the file.
3. The prompt comes back with the full path in it, and you hear "mine.toml chosen. Enter confirms." Press **Enter** to use it, or edit it first.

For a file textweaver writes (Save as, and the exports), F4 chooses the folder instead: press **Ctrl+Enter** on the folder (or choose "Choose this folder"). The prompt comes back with the name you typed, or the name offered, in that folder.

**Escape** in the browser goes back to the prompt as you left it, with what you had typed. Tab completes a path in each of these prompts too.

## Help: ? and F1

- **?**: list every keyboard shortcut with its current keys, including your own changes. Up and Down move, **Enter** runs the command, **Escape** closes. In the window, the Help menu opens it too.
- **F1**: open the help, a short list of the most useful keys. Typing in it starts Search help with what you typed.

### Search help

Search help, in the Help menu and the command palette, looks through everything help knows in one search: the name, keys and description of every command, every setting and its help, and the headings and text of the guides that come with textweaver. Typing a letter in the F1 help starts the same search.

- Type words, and the list keeps only the topics that contain all of them, saying how many remain: "8 matches." **Backspace** removes a letter. You may type a question: in "how do I export audio", textweaver ignores the question words and searches for "export audio".
- Commands come first, then settings, then sections of the guides, with the sections whose heading matches before those that only mention your words. Each row begins with the topic and ends with what kind it is: "Export audio, command", "Rate, setting in Speech", or "Subtitles, in Audio export".
- **Enter** on a command says what it does and which keys run it, and keeps the list open. On a setting it opens Settings at that setting. On a guide section it opens the guide at that heading; **Backspace** or **Alt+Left** returns afterwards, as after following a link.
- **F1** on a row says the topic's help, or the first words of a guide section, without leaving the list.

The guides are divided into sections the first time you search, and the result is kept in textweaver's cache folder, so later searches start at once. When the guides change, for example after an update, the sections are rebuilt.

In any list: **Up** and **Down** move, **PageUp** and **PageDown** move ten items, **Home** and **End** go to the first and last, a letter jumps to the next item starting with it, **Enter** chooses, and **Escape** or **Backspace** closes. At the ends you hear "Top of list." or "End of list."; with `cursor = "status"` the status line keeps the item after it, such as "End of list. 12 of 12, Conclusion", so your Braille display still shows where you are. **F1** or **Alt+End** says the list's introduction again (its name, how many items it has, and the keys it takes), then the item you are on, such as "3 of 12". **Alt+'** says the last message again.

## Turn single-key shortcuts off: F9

Speech recognition, dictation, and switch or scanning keyboards can type a letter by accident, and a letter such as `h` or `.` is a command in textweaver. To prevent this, press **F9**. You hear "Single-key shortcuts off." Press it again to turn them back on.

While they are off, letters, punctuation, and **Space** never run commands. Chords with **Ctrl** or **Alt**, the arrow keys, and the function keys still work, and so does the command palette. This follows WCAG 2.1.4, Character Key Shortcuts. The choice is saved. You can also set it in `settings.toml`:

```toml
[keyboard]
character_keys = false
```

Some commands have only single keys, such as next table. Run those from the command palette, or give them a chord in `keymap.toml`. The [keyboard reference](keyboard.md) lists them and explains `keymap.toml`.

## Quit

Press **Ctrl+Q**. textweaver asks "Quit textweaver? y or n". (In the classic preset, **q** and **Shift+Q** quit too, after the same question.)

- Press **y** to quit. Your place is saved.
- Press **n**, **a**, or **Escape** to stay. You hear "Cancelled."
- Any other key repeats the question.

If you are editing and have unsaved changes, textweaver asks whether to save them first. See [Writing and editing](editing.md).

## The textweaver command line

```bash
textweaver --help
```

`textweaver` takes one optional argument, the document to open, and these options. `tw open` takes the same options, but needs the document.

- `--no-speech`: do not speak at all, in screen-reader mode. There is no self-voicing and no reading aloud. Use it when your screen reader should do all the talking; it reads the status line and follows the cursor. See [Using textweaver with a screen reader](screen-readers.md).
- `--mode MODE`: the accessibility mode for this run, not saved: `self-voicing`, `hybrid` (textweaver reads documents aloud and your screen reader speaks messages and typing), or `screen-reader` (textweaver is silent). **Alt+Shift+A** changes the mode and saves it. See [Using textweaver with a screen reader](screen-readers.md#three-modes).
- `--backend ID`: use this speech engine for this run, instead of the one in your settings. `tw backends` lists the engine ids. An engine that is not available falls back to the automatic choice, and textweaver says so.
- `--home FOLDER`: keep settings, reading positions, and the log under this folder. It works like the `TEXTWEAVER_HOME` environment variable. Use it for a portable copy, or to try things without touching your own settings.
- `--theme NAME`: use this color theme for this run only; it is not saved. The help lists the built-in themes. You can also name a theme in your themes folder. See [Themes](themes.md).
- `--log LEVEL`: write a log to `textweaver.log` in the state folder. The levels are `off`, `error`, `warn` (the default), `info`, `debug`, and `trace`. `--log` alone means `debug`. See [Troubleshooting](troubleshooting.md).
- `-h` or `--help`: print the help.
- `-V` or `--version`: print the version.

## If something goes wrong

- **No speech.** See [Troubleshooting](troubleshooting.md), "No speech at all".
- **A key does nothing.** Your terminal may not send it. Try the other key for the same command, or the command palette. See "Terminal notes" in the [keyboard reference](keyboard.md).
- **Letters do nothing.** Single-key shortcuts may be off. Press **F9**.
- **The PDF reads in a strange order.** See [Troubleshooting](troubleshooting.md).

## See also

- [Keyboard reference](keyboard.md): every key in both frontends, and how to change keys.
- [Bookmarks, notes, and highlights](notes.md): marking and annotating what you read.
- [Using textweaver with a screen reader](screen-readers.md): JAWS, NVDA, VoiceOver, and Orca.
- [Reading aids](reading-aids.md): RSVP, bionic reading, the reading ruler, and text spacing.
- [Speech engines and voices](speech.md): engines, voices, rate, pitch, and volume.
- [ADR-0002: Text model](adr/0002-text-model.md): why lines and positions work the way they do.
- [ADR-0003: Speech threading and event timing](adr/0003-speech-threading-and-event-timing.md): how the highlight follows speech.
- [ADR-0005: Narration and the offset map](adr/0005-narration-and-offset-map.md): how spoken text maps back to the document.
- [ADR-0006: Keymap, actions, and announcements](adr/0006-keymap-and-actions.md): keys, layers, and announcements.
- [Documentation index](README.md)
