# Reading and moving around

This guide covers the terminal reader, `textweaver`: opening a document, reading it aloud, and moving through it by sentence, paragraph, heading, and more. It is for anyone who reads with textweaver, with or without a screen reader.

Keys are the terminal defaults. Where the GUI uses a different key, this guide says so. The [keyboard reference](keyboard.md) lists every key in both frontends. Many keys are single keys, such as `h` for the next heading. Those are called browse keys. They work while you read, not while you edit or type in a prompt.

The browse keys follow the quick navigation keys of NVDA's and JAWS's browse mode: `h` for headings, `1` to `6` for heading levels, `l` for lists, `k` for links, and so on, with Shift for the previous one. They changed on Saturday, September 26, 2026. The [keyboard reference](keyboard.md#what-changed) lists every change, and `preset = "classic"` under `[keyboard]` brings back the earlier keys.

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

### From the library: Alt+L

Press **Alt+L** to list the documents in your library folders and the files you opened recently. Use **Up** and **Down** to move, and **Enter** to open one. The GUI uses **Ctrl+Shift+B**. The [library guide](library.md) explains library folders.

### What you hear when a document opens

textweaver says "Opened", then the title. If you read this document before, it goes back to where you stopped and says so, for example "Opened Cells. Resumed at 42 percent." See [Your place is remembered](#your-place-is-remembered).

If the file cannot be opened, you hear "Could not open", the path, and the reason.

## Which files open

The reader opens these formats itself:

- Plain text: `.txt`, `.text`, `.log`.
- Markdown: `.md`, `.markdown`, `.mdown`, `.mkd`, `.mkdn`, `.mdwn`, `.mdtxt`, `.rmd`.
- HTML: `.html`, `.htm`, `.xhtml`, `.xht`.
- EPUB: `.epub`.
- Word: `.docx`, `.docm`.
- PDF: `.pdf`.

A file with any other extension is read as plain text. A file that is not text at all is refused: a program, an image, an audio file, a zip archive (an OpenDocument file is one), an old Word `.doc`, or an RTF file. The message says what the file looks like, for example: "report.odt is not a text file; it looks like a zip archive (such as a Word document or an EPUB). textweaver cannot read it as text."

The reader does not use Pandoc. To read an OpenDocument, RTF, LaTeX, or other such file, convert it to Markdown first, then open the Markdown. `tw convert` uses Pandoc for these formats, so Pandoc must be installed:

```bash
tw convert essay.odt --to md
```

The [converting guide](converting.md) explains `tw convert`.

A PDF must have a text layer. A scanned PDF is a picture of the pages, and textweaver has no text recognition (OCR) yet; it is planned for Wave 3. Such a PDF reads as one sentence: "This PDF has no text layer. It is probably a scanned image, so its text must be recognized (OCR) before it can be read aloud."

## What the screen shows

The screen has four parts, from top to bottom.

1. **The title line.** It starts with "textweaver:" and the document's title. On the right it shows, in this order: the mode when it is not plain reading (for example "Speech Cursor" or "Edit"), "modified" when there are unsaved edits, the reading state ("Reading", "Paused", or "Stopped"), the line and percentage ("line 12 of 300, 4%"), the rate ("265 wpm"), and the speech engine. On a narrow screen the last parts are left out first.
2. **The document.** The text, with the spoken word highlighted while reading.
3. **The status line.** It shows every announcement: what textweaver just said or would have said. It grows to three rows for a long message. Screen readers read it as it changes. See [Using textweaver with a screen reader](screen-readers.md).
4. **The key hint line.** It shows a few useful keys for the current mode. When a prompt is open (Find, Go to, Open file, and so on), this line becomes the prompt, and you type there.

The terminal's cursor always sits where your attention is: on the word being spoken while reading, on the Speech Cursor line, on the prompt, on the chosen item of a list, or else on the reading cursor. Screen readers and screen magnifiers follow it.

Lists, such as the help, bookmarks, notes, and the library, appear in a box over the document.

Code blocks are drawn in the theme's code colors. When a block names its language (```` ```python ````), its keywords, strings, comments, numbers, and names get colors from the theme too, and the kinds differ by more than color: keywords are bold and comments italic. The text itself never changes. Moving the caret onto the block's first line says its language, for example "code, Python".

## Read aloud

### Play and pause: Space

Press **Space** to start reading from the cursor. Press it again to pause. Press it once more to go on. Reading goes on from the last word you heard, so you may hear one word twice, but you never miss one. If you move the cursor while paused, reading goes on from the new place.

**Alt+P** does the same, and still works when single-key shortcuts are off. The GUI uses **Ctrl+Shift+Space**.

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

- **c**: say the character. Terminal chord **Alt+Shift+C**; GUI **Ctrl+Shift+C**.
- **w**: say the word. Terminal chord **Alt+Shift+W**; GUI **Ctrl+Shift+W**.
- **.** (period): say the sentence. Terminal chord **Alt+Shift+S**; GUI **Ctrl+Shift+E**.
- **Alt+Shift+L**: say the line. GUI **Ctrl+L**.
- **,** (comma): say the paragraph.
- **v**: read the selected text. With nothing selected, you hear "No selection."

An empty line is read as "blank". Pressing **Space** right after one of these keys reads on from the cursor.

### Read again

- **;** or **Alt+;**: read again from the start of the current sentence.
- **r** or **Ctrl+R**: read again from the start of the current paragraph.

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
- **Shift+P**, **[**, or **Ctrl+Up**: previous paragraph. The GUI also has **Ctrl+Shift+P**.

### Headings

There are two kinds of heading keys. One reads from the heading. The other only moves.

- **>**: read from the next heading. The GUI also has **Ctrl+H**.
- **<**: read from the previous heading. The GUI also has **Ctrl+Shift+H**.
- **h**, **}**, or **Alt+H**: move to the next heading without reading.
- **Shift+H**, **{**, or **Alt+Shift+H**: move to the previous heading without reading.
- **1** to **6**: the next heading at that level. **Shift** with the digit: the previous one. textweaver matches the digit key itself, so this works on any keyboard layout; see [the keyboard reference](keyboard.md#terminal-notes).

You hear the heading level and text, for example "Heading level 2: Methods". **Alt+H** and **Alt+Shift+H** are chords, so they also work in edit mode and with single-key shortcuts turned off. They are terminal keys; the GUI has **Ctrl+H** and **Ctrl+Shift+H**.

### The outline: Alt+O

Press **Alt+O** for a list of the document's headings, in order, each with its level: "Methods, level 2". You hear how many there are and which heading you are under: "Outline, 12 headings. Type to filter, Enter goes to a heading, Escape closes. You are under Methods."

- Type part of a heading to filter the list. Only the headings that hold every word you type stay, and you hear how many match. **Backspace** removes a letter; Space is part of the filter.
- **Up**, **Down**, **Home**, and **End** move through the list.
- **Enter** goes to the heading. It is a jump, so **Alt+Left** comes back.

The outline works while reading and while editing; in edit mode it lists the headings as you have written them so far.

### Tables, lists, links, and more

- **t**: next table. **Shift+T**: previous table. The GUI also has **Ctrl+T** and **Ctrl+Shift+T**.
- **l**: next list. **Shift+L**: previous list.
- **i**: next list item. **Shift+I**: previous list item.
- **k** or **u**: next link. **Shift+K** or **Shift+U**: previous link.
- **q**: next block quote. **Shift+Q**: previous block quote.
- **s**: next separator (a horizontal rule). **Shift+S**: previous separator.
- **g**: next graphic (an image, read by its alt text). **Shift+G**: previous graphic.
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
- **Shift+D**, **F10**, or **Alt+PageUp**: previous chapter. More than five words into a chapter, this goes back to its start instead.

Chapters are the book's sections when the document has them (EPUB chapters, Word sections, PDF bookmarks). Otherwise they are the level-1 headings. A document with neither says "This document has no chapters." Some terminal programs keep F10 and F11 for themselves; the Alt chords always work. The GUI uses only the Alt chords.

### Start and end

- **Home** or **Ctrl+Home**: the start of the document ("Top of document").
- **End** or **Ctrl+End**: the last word of the document ("End of document").

### Pages and scrolling

- **PageDown** and **PageUp**: move one screen, less four lines. You hear "Page, line", the line number, and a preview.
- **j** and **Shift+J**: scroll the view down or up one line without moving the cursor.

### The caret keys

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

- Search ignores case.
- To search with a regular expression, write it between slashes, for example `/colou?r/`.
- The search starts at the cursor. When there is no match after it, it wraps to the top.

You hear the match number, the count, and the line, for example "Match 2 of 5:" followed by the text of that line. With no match, you hear "No matches for", then your text.

### Next and previous match

- **F3** or **n**: next match.
- **F4** or **Shift+N**: previous match.

At the end, the search wraps and says "Wrapped to top." or "Wrapped to bottom." If you have not searched yet, these keys open the Find prompt. The GUI uses **n** and **Shift+N** only, because it keeps **F3** for the keyboard list.

### Clear the search

Press **Escape** when nothing is being read. You hear "Search cleared."

`tw search` searches a file from the command line, with options for case and whole words:

```bash
tw search essay.md "mitochondria" --whole-word
```

## Go to a line or a percentage: Ctrl+G

Press **Ctrl+G**. The prompt says "Go to line, percent, start, or end". Type one of these and press **Enter**:

- a line number, such as `42` or `line 42`;
- a percentage, such as `50%` or `50 percent`;
- a character position, counted from 0, such as `char 120` or `character 120`;
- `start`, `top`, `beginning`, or `begin`;
- `end` or `bottom`.

You hear the percentage, the line, and a preview. Anything else gives: "Not a go-to target:", your text, then "Type a line number, a percentage such as 50%, start, or end."

Line numbers are lines of the document's text, as in Speech Cursor mode. Press **F6** to show them on screen. There is no way to go to a printed page number yet.

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

## Go back and forward

textweaver keeps a history of your jumps, like the Back button of a web browser.

- **Backspace** or **Alt+Left**: go back to where you were before the last jump.
- **\\** (backslash) or **Alt+Right**: go forward again.

You hear "Back, line", the number, and a preview. With nothing to go back to, you hear "No earlier history."

There is one rule for what counts as a jump. Every move by a sentence or anything larger records the place you left, once. That includes paragraphs, headings, tables, lists, list items, links, chapters, finds, go to, the start or end of the document, bookmarks, and notes. Moves by word, line, or page, scrolling, and Speech Cursor line moves are not recorded. Going back and forward are not recorded either.

The history keeps the last 50 places. Change that with `[reading] nav_history_size`. It is saved with the document, so it is still there when you open the document again.

## Where am I: Shift+W or Alt+Shift+Y

Press **Shift+W**, or **Alt+Shift+Y** from any mode. You hear the line, the number of lines, and the percentage, for example "Line 12 of 300, 4 percent." In a table you also hear where in it: "Table, row 2 of 5, column 3 of 4." At normal verbosity you also hear the word number and the heading above you: "Under heading Methods." At high verbosity you also hear the document's title and the mode, when it is not plain reading. This works in edit mode too, on the headings as you have written them.

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

While textweaver reads, the spoken word is highlighted on screen. The highlight never relies on colour alone, so it shows even with colour turned off.

These settings are in the `[highlight]` section of `settings.toml`:

- `enabled` (default `true`): highlight the spoken text at all.
- `granularity` (default `"word"`): `"word"` highlights the word, `"sentence"` the whole sentence, and `"both"` the sentence with the word inside it.
- `lead_words` (default `1`): where the highlight sits, from -5 to 5. At 1 it is on the word you hear. At 2 it runs one word ahead; at 0, one word behind. It moves only the drawn highlight, never your saved place.
- `speed` (default `1.0`): from 0.5 to 1.5. It speeds up or slows down the highlight for engines that do not report words, where textweaver estimates the timing.

- `color` and `sentence_color`: the colours of the word and sentence highlight, such as `"#ff8800"` or `"yellow"`. The terminal reader draws them over the [colour theme](themes.md) and warns when one does not stand out from the text.

If the highlight runs ahead of or behind the voice with an engine that does report words, change `[speech] latency_offset_ms`. The [speech guide](speech.md) explains it.

For more help reading, such as one word at a time (RSVP), bionic reading, and a reading ruler, see [Reading aids](reading-aids.md).

## Faster, slower, and the voice

- **+** or **=**: faster. **-**: slower. Each step is 20 words per minute. The GUI also has **Ctrl+=** and **Ctrl+-**.
- **F8**: cycle the speed presets: skim, normal, study, slow.
- **Alt+V**: choose a voice. The GUI uses **Ctrl+Shift+V**.

The [speech guide](speech.md) covers rate, pitch, volume, and voices.

## Announcements and verbosity

textweaver tells you about every change: a move, a mode, a setting. How much it says is set by `[speech] verbosity`:

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

## Spelling

**Alt+M** moves to the next misspelled word and **Alt+Shift+M** to the previous one, while reading or editing. You hear the word, then its letters: "recieve. r e c i e v e." **Alt+J** lists suggestions. The [editing guide](editing.md#spelling) has the details.

## Define a word: Ctrl+Shift+D or Alt+E

**Ctrl+Shift+D** in the GUI, or **Alt+E** in the terminal, defines the word at the cursor, or the words you selected (`ice cream`). textweaver looks in your own glossary first, then in Open English WordNet, and says how the word is pronounced, from the CMU Pronouncing Dictionary. Everything is on your computer: nothing goes to the internet.

The senses come up in a list. The first item is the pronunciation, respelled with the stressed syllable in capitals ("Pronounced RUN-ing."). Each sense then says its headword, its part of speech, which sense it is, the definition, an example, synonyms, opposites, and what it is a kind of:

> dog, noun, 1 of 7: a member of the genus Canis (probably descended from the common wolf)... For example: the dog barked all night. Synonyms: domestic dog, Canis familiaris. A kind of: canine, domestic animal.

Up and Down move through the senses. Enter copies one to the clipboard, for a note. Escape closes the list.

- Inflected words find their base forms: `running` finds `running` and `run`, `geese` finds `goose`, and `wider` finds `wide`.
- With no word at the cursor, or no document open, textweaver asks which word to define. The command palette's `define word` does the same.
- **Your glossary** is a text file with one `term: definition` line per sense, or a JSON file in Star's format. Put it in the settings folder as `glossary.txt`, or name it with `[lexicon] glossary` ([settings.md](settings.md#lexicon)). Its senses come first, before WordNet's. An edited glossary is read again the next time you define a word.
- `tw define WORD` does the same from the command line, as Markdown or, with `--json`, as JSON.
- The dictionary is a 10 MB file, `lexicon/lexicon-en.twlex`, installed beside the program. If it is missing, textweaver says so and searches your glossary only.

## Reading statistics: Ctrl+Shift+Y or Alt+Y

textweaver counts the time it spends reading each document aloud, the furthest point you reached, and the sessions: each time you open a document and read it. **Ctrl+Shift+Y** in the GUI, or **Alt+Y** in the terminal, lists:

- the total time read, over how many sessions and documents;
- this document's time, furthest point, and sessions;
- the ten documents you read most (Enter opens one);
- whether statistics are on (Enter turns them off or on).

Statistics are saved every 30 seconds while reading, and when a document closes. `tw stats` prints them, `tw stats --json` prints everything, and `tw stats --clear` removes them. To stop recording, turn them off in the list or set `[stats] enabled = false` ([settings.md](settings.md#stats)). `tw migrate-star` brings Star's reading statistics over.

## The command palette: F2

Press **F2** to run any command by name. **Alt+X** and **:** open it too. The GUI uses **F2** and **:**. You hear "Command. Type part of a name; Tab completes, Up and Down list matches." (at low verbosity, just "Command"); the bottom line shows "Command".

1. Type part of a command's name, such as `next head`.
2. Press **Tab** to complete it. You hear how many commands match and the first few names.
3. Or press **Down** and **Up** to go through the matching commands. You hear each command's name, what it does, and its keys.
4. Press **Enter** to run it.

An unknown name gives "Unknown command:" and your text.

These keys work in every prompt, including Find, Go to, and Open file:

- **Up** and **Down**: earlier answers to the same prompt (except in the palette, where they go through the commands). The last 50 are kept until you quit.
- **Ctrl+A** and **Home**: to the start. **Ctrl+E** and **End**: to the end.
- **Ctrl+U**: delete to the start. **Ctrl+K**: delete to the end. **Ctrl+W**: delete the word before the caret.
- **Escape** or **Ctrl+G**: cancel.

## Help: ? and F1

- **?**: list every keyboard shortcut with its current keys, including your own changes. Up and Down move, **Enter** runs the command, **Escape** closes. The GUI also opens this list with **F3**.
- **F1**: open the help, a short list of the most useful keys.

In any list: **Up** and **Down** move, **PageUp** and **PageDown** move ten items, **Home** and **End** go to the first and last, a letter jumps to the next item starting with it, **Enter** chooses, and **Escape** or **Backspace** closes. At the ends you hear "Top of list." or "End of list." **F1** or **Alt+End** says the list's introduction again (its name, how many items it has, and the keys it takes), then the item you are on, such as "3 of 12". **Alt+'** says the last message again.

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
- `--theme NAME`: use this colour theme for this run only; it is not saved. The help lists the built-in themes. You can also name a theme in your themes folder. See [Themes](themes.md).
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
