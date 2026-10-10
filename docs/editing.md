# Writing and editing

This guide covers edit mode in the terminal reader: typing with spoken feedback, Markdown formatting commands, undo, find and replace, saving, and getting back unsaved work after a crash. It is for anyone who writes or corrects documents in textweaver.

Keys are the terminal defaults. Where the graphical version uses a different key, this guide says so. The [keyboard reference](keyboard.md) lists every key.

## Start and finish editing: Ctrl+E

Open a document, then press **Ctrl+E**. textweaver stops reading and says "Edit mode on." It then names the keys to save and to finish, and reads the line the cursor is on. At low verbosity it says only "Edit mode on." and the line. The title line shows "Edit".

Press **Ctrl+E** again to finish. textweaver says "Edit mode off." If you have unsaved changes, it asks first; see [Leaving with unsaved changes](#leaving-with-unsaved-changes).

**Escape** stops reading, as everywhere. When nothing is being read, it leaves you in edit mode and says how to finish: "Still editing. Ctrl+E finishes." textweaver's own voice says the key as "Control E".

With no document open, **Ctrl+E** says "No document to edit. Press Ctrl+N for a new one." (textweaver's own voice says "Control N").

### What changes in edit mode

Edit mode shows the document's source, the text that is saved:

- A plain-text file shows its own text.
- A Markdown file shows its Markdown, with `#`, `*`, and the other marks.
- Any other format (HTML, EPUB, Word, PDF) shows the document converted to Markdown. Saving it writes a new Markdown file; the original is never changed.

Your place, bookmarks, notes, and highlights move with your edits. When you finish, textweaver rebuilds the reading view from the saved file and puts them back in the matching places.

In edit mode every key that is not a command types. Single-key reading keys such as `.` and `p` type their character. Chords such as **Ctrl+F** (find), **Ctrl+G** (go to), and **Ctrl+Home** still work.

## Type and move

- Letters, digits, and punctuation type as usual. **Enter** starts a new line. **Tab** types a tab.
- **Backspace** deletes the character before the cursor. **Delete** deletes the one after it. A character is what you see as one: an emoji with its skin tone, a flag, or a letter with its accent is deleted whole.
- **Left** and **Right**: move by character. You hear the character.
- **Ctrl+Left** and **Ctrl+Right**: move by word. You hear the word.
- **Up** and **Down**: move by line. You hear the new line.
- **Home** and **End**: the start or end of the line.
- **Ctrl+Home** and **Ctrl+End**: the start or end of the document.
- **PageUp** and **PageDown**: move one screen.
- Hold **Shift** with any of these to select. You hear the text added, then "selected", or the text removed, then "unselected".

At the edges you hear "Start of line.", "End of line.", "Top of document.", or "End of document."

More editing keys:

- **Ctrl+A**: select all the text. You hear "Selected all" and the number of words.
- **Alt+Backspace**: delete the word before the cursor. The graphical version uses **Ctrl+Backspace**.
- **Ctrl+Delete**: delete the word after the cursor.
- **Ctrl+C**: copy the selection. **Ctrl+X**: cut it. Both go to your computer's clipboard through the terminal; see [Copy, cut, and paste](#copy-cut-and-paste).
- **Ctrl+V**: paste. Formatted text becomes Markdown. See [Copy, cut, and paste](#copy-cut-and-paste).
- **Alt+Q**: paste as plain text. The graphical version uses **Ctrl+Shift+M**.

You hear what the word keys deleted, for example "three deleted."

## Structure while you write

The reading keys that move by structure work in edit mode too, on the Markdown as you have written it so far:

- **Alt+H** and **Alt+Shift+H**: the next and previous heading. The cursor lands on the heading's text, after the `#` marks.
- **Alt+O**: the outline, a list of the headings you can filter by typing; see [the reading guide](reading.md#the-outline-alto).
- **Alt+Shift+Y**: where am I, with the heading you are under.
- The table keys, **Ctrl+Alt** with the arrows, move by row and cell in a Markdown table and say the column header.
- From the command palette (**F2**), the list, list item, and link moves work too: type `next list item` or `next link`.

After you stop typing for a moment (about a third of a second), textweaver reads the structure again, so a heading you just typed can be reached at once. In a very long document this happens in the background and takes a little longer.

### Keyboard layouts with AltGr

On many European layouts the right Alt key (AltGr) types characters such as `@`, `[`, `{`, `|`, `~`, `€`, and `ą`. On Windows the terminal reports AltGr as Ctrl plus Alt. textweaver types the character whenever no command uses that chord, so these characters work in edit mode and in prompts. A real Ctrl+Alt shortcut uses a plain letter or digit, so the two do not collide.

## Typing echo

textweaver speaks what you type, so you know it arrived. Four settings in the `[editing]` section of `settings.toml` control it. All four are on by default.

- `echo_characters`: say each character as you type it.
- `echo_words`: say each word when you finish it with a space or punctuation.
- `echo_deletions`: say what Backspace or Delete removed.
- `echo_lines_on_move`: say the new line when Up, Down, PageUp, PageDown, Ctrl+Home, or Ctrl+End moves to another line. An empty line is "blank".

For example, to hear words but not single characters:

```toml
[editing]
echo_characters = false
```

Echo is part of self-voicing. When you start textweaver with `--no-speech`, it says nothing, and your screen reader echoes your typing as it does everywhere else. See [Using textweaver with a screen reader](screen-readers.md).

### Capital letters

`[speech] caps` sets how a capital letter is marked when textweaver says a single character:

- `"pitch"` (the default): the letter is said at a higher pitch.
- `"tone"`: a short tone plays before it.
- `"say_cap"`: textweaver says "cap" before it.
- `"none"`: no mark.

If the voice cannot change pitch, a tone is used. If the engine cannot play tones either, textweaver says "cap".

```toml
[speech]
caps = "say_cap"
```

## Markdown commands

Each command changes the selection, or the line the cursor is on. Each one is announced: for example "Bold." when it adds the marks, or "Bold removed." when the text was already bold. The commands toggle, so pressing a key twice undoes it. With nothing selected, bold, italic, and the other wrapping commands insert placeholder text and select it, so you can type over it.

- Bold: **Ctrl+B**. Writes `**text**`.
- Italic: **Alt+I**. The graphical version uses **Ctrl+I**. Writes `*text*`.
- Underline: **Ctrl+U**. Writes `<u>text</u>`, because Markdown has no underline.
- Strikethrough: **Alt+D**. The graphical version uses **Ctrl+Shift+X**. Writes `~~text~~`.
- Inline code: `` Alt+` ``, that is Alt with the backtick key. The graphical version uses `` Ctrl+` ``. Writes the text between backticks.
- Code block: **Alt+K**. The graphical version uses **Ctrl+Shift+K**. Fences the selected lines.
- Link: **Ctrl+K**. Writes `[text](https://)`. With nothing selected, `text` is selected so you can type the link text. With a selection, the address is selected so you can type it.
- Heading: **Alt+1**. The graphical version uses **Ctrl+Alt+1**. Each press raises the level of the current line by one: level 1, then 2, up to 6. Pressing it on a level-6 heading removes the heading. You hear "Heading level 2." and so on.
- Bulleted list: **Alt+8**. The graphical version uses **Ctrl+Shift+L**. Puts `- ` before each selected line.
- Numbered list: **Alt+7**. The graphical version uses **Ctrl+Shift+O**. Numbers each selected line. Bulleted and numbered lists turn into each other.
- Block quote: **Alt+9**. The graphical version uses **Ctrl+Shift+Q**. Puts `> ` before each selected line.
- Horizontal rule: **Alt+R**. The graphical version uses **Ctrl+Shift+R**. Inserts `---` on its own line.
- Insert a table: **Alt+T**. The graphical version uses **Ctrl+Shift+A**. textweaver asks "Table size, columns by rows, for example 3 by 2". Type the columns, then the rows, as `3 by 2`, `3x2`, or `3 2`. Enter alone makes 2 by 2. Up to 20 columns and 100 rows. The first header cell is selected afterwards.
- Add a table row: **Alt+W**. The graphical version uses **Ctrl+Shift+Enter**. Adds an empty row to the table at the cursor; the cursor goes to its first cell.
- Insert an image: **Alt+G**. The graphical version uses **Ctrl+Shift+I**. textweaver asks for the image file. It writes `![name](path)` and selects the description, so you can type a better one.

The Alt chords in the terminal replace window chords that terminals cannot send, such as **Ctrl+I**, which arrives as Tab.

Writing citations and math has its own guides: [citations](citations.md) and [math](math.md).

## Undo and redo

- **Ctrl+Z**: undo.
- **Ctrl+Y**: redo. The graphical version also has **Ctrl+Shift+Z**.

Typing and deleting are grouped into word-sized steps, so one undo removes about one word. Every formatting command, a paste, and a Replace All are each one step. You hear "Undo." or "Redo." and the current line. With nothing left, you hear "Nothing to undo." or "Nothing to redo."

textweaver keeps the last 1,000 steps, or 50 MB of them, whichever comes first; the oldest are forgotten. `undo_steps` and `undo_memory_mb` in `[editing]` change the limits (see [Settings](settings.md#editing)).

## Find and replace: Alt+F

Press **Alt+F** in edit mode. In the graphical version, **Ctrl+Shift+F** opens a panel with the same choices as fields, check boxes, and buttons; see [In the graphical version: the find and replace panel](#in-the-graphical-version-the-find-and-replace-panel).

1. The prompt says "Replace, find what". Type the text to find and press **Enter**. You hear how many matches there are, then "Replace with?"
2. Type the new text and press **Enter**.

In both prompts, **Up** and **Down** bring back what you typed before. The text to find shares its history with Find (**Ctrl+F**).

textweaver then goes through the matches one at a time, starting at the cursor. Each match is selected, and you hear its number, its line, and what it becomes, then the text of its line, for example "Match 2 of 5, line 12: teh becomes the. The line: teh cat sat on the mat." When the new text is empty, you hear "teh is removed" instead. Long matches are shortened, so the start of the line ("Match 2 of 5, line 12") always fits a 40-cell Braille display. A short list asks what to do. Press a letter, or move with Up and Down and press Enter:

- **r**, "Replace this one": replaces it and goes to the next match.
- **s**, "Skip this one": leaves it and goes to the next match.
- **a**, "Replace all the rest": says how many matches are left and asks once, for example "Replace all 4 remaining matches? y or n". **y** replaces them all; **n** goes back to the match.
- **c**, "Match case": on or off. Off (the default), `cat` also finds `Cat`.
- **w**, "Whole words only": on or off. On, `cat` does not find `catalog`.
- **x**, "Regular expression": on or off. On, the text to find is a regular expression, and the new text can use what it captured (see below).
- **l**, "Across lines": on or off. With a regular expression, `.` matches a line break too, so a match can run on from one line to the next.

After switching an option you hear the new setting and how many matches there are now. These are the same search options that Find uses, and they stay as you left them until you quit. Search options, in the Edit menu under Find and in the command palette, changes them without replacing anything.

After the last match, the search goes on from the top of the document and stops where it started. At the end you hear what was done, for example "Replaced 3, skipped 1." Press **Escape** to stop early: "Stopped. Replaced 1, skipped 0."

Each replacement is one undo step, and "Replace all the rest" is one step for all of them, so **Ctrl+Z** takes back the last thing you chose.

To find without replacing, use **Ctrl+F**, as when reading.

### Regular expressions

With **Regular expression** on, textweaver uses the Rust `regex` syntax:

- `cats?` finds "cat" and "cats"; `\d+` finds a number; `\bcat\b` finds the word "cat".
- `^` and `$` match at the start and end of a line. `\n` and `\s` can match a line break; with **Across lines** on, so can `.`.
- In the new text, `$1` is what the first group in parentheses matched, `${name}` is a named group such as `(?<year>\d{4})`, and `$$` is a dollar sign. Write `${1}st` rather than `$1st` when a letter or digit follows the group. With the option off, a dollar sign in the new text is just a dollar sign.
- For example, find `(\d{4})-(\d\d)-(\d\d)` and replace with `$3/$2/$1` to turn 2026-10-09 into 09/10/2026.
- A pattern that is not valid is said in words, with the character where it goes wrong, for example "Invalid pattern at character 3: unclosed group." Nothing is replaced. Press **Alt+F** and **Up** to fix it.
- A pattern that matches only a position, such as `^` alone, finds nothing: every match must hold at least one character.

### In the graphical version: the find and replace panel

In the graphical version, **Ctrl+Shift+F** in edit mode opens the Find and replace panel, a dialog that gathers the whole search in one place. It drives the same search as the terminal's prompts, with the same options, history, spoken previews, and undo steps, so a replacement behaves identically in both programs. The controls, in Tab order, are these:

- **Find what**: the text to find or, with Regular expression on, the pattern. **Enter** in this field finds the next match.
- A status line beneath it. While the pattern cannot be searched, it explains why, for example "Invalid pattern at character 3: unclosed group.", and textweaver says so once each time the reason changes. While a replacement is under way, it shows the match in question, for example "Match 2 of 5, line 12: teh becomes the."
- **Replace with**: the new text, which may refer to what a regular expression captured. **Enter** in this field replaces the match.
- Four check boxes: **Match case**, **Whole words**, **Regular expression**, and **Across lines**. **Space** switches the focused one. These are the search options described above, shared with Find and with the terminal's loop; when one changes during a replacement, the matches are counted again.
- **Find next** (F3): selects the next match. During a replacement it skips the match in question instead.
- **Replace**: the first press finds the first match after the cursor and says it; each further press replaces that match, as one undo step, and moves on to the next.
- **Replace all**: says how many matches would change and asks once. **Y** replaces them all as a single undo step; **N** returns to the panel.
- **Close** (Escape): closes the panel and returns the focus to the document. A replacement under way stops, and textweaver reports what was done, for example "Stopped. Replaced 2, skipped 1."

In either field, **Up** and **Down** bring back earlier entries; Find what shares its history with Find (**Ctrl+F**). **F3** and **Shift+F3** find the next and the previous match from any control in the panel, and the Help key (**F1**) reads a short summary of these keys. Counts and previews are announced politely, so they wait until your screen reader has finished speaking.

In a short window, less than 480 pixels high, the panel is compact: the drawn title and hint are omitted and the buttons hide their keys on screen, while the controls, their names, and their order remain the same.

## Save

### Save: Ctrl+S

Press **Ctrl+S**. You stay in edit mode. You hear "Saved", the file name, and "Still editing."

The file is written in the background, so a large file never holds up the keyboard: you can go on typing, and "Saved" comes as soon as the file is on disk. What you type while it is being written is not in that save. Quitting waits for every save to finish; if the disk is slow, you hear "Still saving. Please wait."

- A Markdown or plain-text file is saved in place. Its byte-order mark and line endings are kept.
- Any other format (HTML, EPUB, Word, PDF, and so on) is never overwritten. textweaver asks for a new file name ending in `.md`: "Save as, Enter for", then a suggested name. Press **Enter** to accept it, or type another. After that, **Ctrl+S** saves the new file in place.

**Ctrl+S** when you are not editing says "Nothing to save. Turn on edit mode with Ctrl+E to make changes."

### Save As: Alt+S

Press **Alt+S** to save under a new name. The graphical version uses **Ctrl+Shift+S**. The prompt suggests the current name. A name ending in `.md`, `.markdown`, `.txt`, or another plain-text or Markdown extension is kept. Any other ending becomes `.md`, so Markdown never lands in an `.html` or `.docx` file. A name without a folder goes in the same folder as the suggestion.

Save As asks before it writes over a file of the same name: "notes.md already exists. Replace it? y or n". In the graphical version, the system's Save dialog asks instead.

### New document: Ctrl+N

Press **Ctrl+N** to start a blank Markdown document in edit mode. You hear "New document ready for editing." The first save asks for a name; the suggestion is `document.md` in the folder you started textweaver in.

### What cannot be saved in place

Some files are refused, with a message saying why. Your text stays in the editor. Use Save As (**Alt+S**) to write a copy.

- A read-only file: "the file is read-only; use Save As to write a copy".
- A file that is not UTF-8, such as an old Windows-1252 or a UTF-16 file: "the file is not UTF-8 text, and saving would replace the characters that could not be read; use Save As to write a copy". textweaver reads such files correctly, but saving would change characters it had to guess.
- Converted formats, as above, are never overwritten.

The full message starts "Could not save:" and ends "Still editing."

## When the file changes on disk

textweaver notices when another program changes the open file, for example Obsidian, a text editor, or `git pull`.

- **Before saving.** If the file changed since you opened or last saved it, nothing is written, and **Ctrl+S** asks first: the file name, then "changed on disk since you opened it. Save over those changes? y or n". Press **y** to save over them. Press **n** to keep editing without saving; you hear "Not saved. Still editing. Save As, Alt+S, keeps both versions."
- **While reading or editing with no unsaved changes.** textweaver checks every two seconds, in the background. When the file changed, it asks: the file name, then "changed on disk. Reload it? y or n". Press **y** to load the new version, or **n** to keep the one you have ("Kept the open version."). It does not ask while it is reading aloud, while another question is open, or while you have unsaved changes.
- **The next time you open it.** Your reading position, bookmarks, notes, and highlights are found again in the changed text. Each is looked for by the words it was on: first near where it was, then anywhere in the file (a paragraph that moved), then by the most similar words nearby (a word changed inside it). Anything that cannot be found is placed at the same share of the way through and marked. You hear it once when the file opens, for example "The file changed; 3 bookmarks were moved to match, 1 bookmark could not be found and is marked." The bookmark, note, and highlight lists say "not found after the file changed" for a marked one; setting it again clears the mark.

## Leaving with unsaved changes

When you finish editing, open another file, start a new document, or quit with unsaved changes, textweaver says the document's name, then "has unsaved changes. Save, discard, or cancel? Press s, d, or c, or Up and Down and Enter. Escape cancels." A list appears with three choices:

- "Save, then continue" (**s**): saves, then does what you asked.
- "Discard the changes" (**d**): throws the changes away, then does what you asked. You hear "Changes discarded. Edit mode off."
- "Cancel, keep editing" (**c**): you stay in edit mode.

**Escape** also cancels; you hear "Still editing."

## Recovering unsaved work

While you edit, textweaver writes a recovery copy of your text every 20 seconds, but only while there are unsaved changes. If textweaver or the computer stops before you save, the copy is kept. Finishing edit mode normally deletes it.

The next time textweaver starts, it offers the copy: "textweaver closed with unsaved changes to", the title, "saved", the time, then "Recover them now? Up and Down choose, Enter confirms." The list has two choices:

- "Yes, recover", the title, "and keep editing": opens the document in edit mode with the recovered text. You hear "Recovered unsaved work in", the title, then "Remember to save." The text is marked unsaved.
- "No, discard the unsaved changes": deletes the copy.

Press **Escape** to decide later: "Recovery postponed. The unsaved work will be offered again next time."

The settings are in `[editing]`:

- `autosave_recovery` (default `true`): write recovery copies and offer them at startup.
- `autosave_interval_secs` (default `20`): seconds between copies. The smallest allowed value is 5.

The copies are in the `recovery` folder of the data folder. The [library guide](library.md) says where that is.

## Copy, cut, and paste

### Copy and cut

**Ctrl+C** copies the selection. When nothing is selected in reading mode, it copies the sentence at the cursor, and you hear "Copied the sentence:" followed by its first words; with a selection you hear "Copied:" and the first words instead. **Ctrl+X** cuts the selection in edit mode, and the cut is a single undo step. A long selection is summarized rather than read in full: you hear how many characters were copied, with the first and last few words.

In the terminal reader, textweaver sends copied text to the terminal, which places it on your computer's clipboard (the OSC 52 sequence, which also works over SSH). Windows Terminal, iTerm2, kitty, WezTerm, foot, Alacritty, and xterm accept it. The old Windows console window, macOS Terminal, and terminals built on VTE (GNOME Terminal, Tilix) do not, so there textweaver writes to the system clipboard itself; the first time, it says "Copied with the system clipboard, because this terminal cannot take copied text." Konsole receives both. Over SSH and inside tmux, textweaver always uses the terminal, because the system clipboard on that side belongs to the other computer.

### Paste: formatted text becomes Markdown

**Ctrl+V** pastes at the cursor in edit mode. When the clipboard holds formatted text, such as a passage copied from a web browser or a word processor, textweaver converts it to Markdown with its own HTML and RTF readers and its own Markdown writer, the same ones it uses to open and export documents. Headings, bulleted and numbered lists, bold and italic text, links, tables, and code survive the conversion, so the pasted passage keeps its structure in your document. You then hear what arrived, for example "Pasted as Markdown: 1 heading, 3 paragraphs, 1 list." When the clipboard holds only plain text, you hear how many lines or characters were pasted and how the text begins, for example "Pasted 3 lines: The results suggest that…"

A very long formatted passage is converted in the background, so the keyboard keeps responding; you hear "Converting the formatted text to paste." and the text appears when the conversion is done. Whatever its size, every paste is one undo step: **Ctrl+Z** removes the whole paste at once, and **Ctrl+Y** puts it back.

### Paste as plain text: Alt+Q, or Ctrl+Shift+M in the graphical version

When you want the words without their formatting, use **Paste as plain text**. It inserts only the clipboard's plain text, with no Markdown added. It is in the Edit menu, in the context menu, and in the command palette as "paste plain text". Many programs use **Ctrl+Shift+V** for this, but the keymap lists that key for choosing a voice in the graphical version, and a terminal cannot tell **Ctrl+Shift+V** from **Ctrl+V**, so textweaver uses the nearest free keys instead. You can move the command to any key you prefer in `keymap.toml`; the [keyboard guide](keyboard.md) explains how.

### Where the clipboard comes from in the terminal

Most terminals keep **Ctrl+V** or **Ctrl+Shift+V** (or a right-click) as their own paste command. That paste reaches textweaver as plain text, because a terminal passes on only the characters it was given; it is therefore always a plain-text paste. textweaver turns on bracketed paste, so the terminal sends the pasted text as one piece rather than as separate key presses. As a result, the paste is one undo step, keys inside the pasted text never run commands, and you hear how much was pasted and how it begins.

When the terminal passes **Ctrl+V** through to textweaver instead of pasting, textweaver reads the system clipboard itself, and formatted text from a browser or word processor becomes Markdown as described above. Over SSH and in tmux, where the system clipboard belongs to the other computer, textweaver pastes the text you last copied or cut in textweaver; when there is none, it says "Nothing copied in textweaver yet. Use your terminal's paste, for example Control Shift V."

In the graphical version, **Ctrl+V** pastes only the clipboard's plain text for now. Converting formatted text to Markdown is done in the terminal reader; see [Known limits](known-limits.md#the-graphical-version).

### The context menu: Ctrl+F10 in the terminal

The context menu gathers the commands that fit where the cursor is: Cut, Copy, Paste, Paste as plain text, and Select all, then Add a note, Highlight, Define the word, Read from here, and, on a link, Open link. Cut and the two paste commands appear only in edit mode. Each item is read with its key, for example "Copy, Ctrl+C", so the menu also teaches the shortcuts. The items are the same commands as in the menu bar, not copies of them, so a key you change in `keymap.toml` changes in both places.

In the terminal reader, **Ctrl+F10** opens the context menu as a list ("Context menu"), because **Shift+F10** opens Settings there. A terminal that reports the Applications key opens it with that key too. Move with the arrow keys or press an item's letter, press **Enter** to run the item, and press **Escape** to close the menu; you hear "Context menu closed." and the cursor is where it was. In the graphical version, **Shift+F10** is the context menu key.

Pasting into a prompt, such as Find, puts the text in the prompt.

## Citations

In edit mode, **Alt+C** inserts a citation: pick a reference from a list you can filter by typing, then give a page or other locator. **Alt+B** (**Alt+Shift+D** in the graphical version) adds a reference by DOI or ISBN. The command palette has `insert bibliography`, `check citations`, and `import references`. The [citations guide](citations.md#citations-while-reading-and-writing-in-textweaver) explains them.

## Spelling

textweaver checks spelling against a list of 225,038 English words (SCOWL, sizes 35 to 80, which includes American, British, Canadian, and Australian spellings) and your own word list.

- **Alt+M**: the next misspelled word. **Alt+Shift+M**: the previous one. You hear the word, then its letters: "recieve. r e c i e v e." In edit mode the word is selected, so typing replaces it. These keys work while reading too.
- **Alt+J**: suggestions for the misspelled word at the cursor, closest first, then "Add recieve to your word list" and "Leave it as it is". In edit mode, **Enter** on a suggestion replaces the word; that is one undo step.
- When you save, textweaver says how many possible misspellings are left, for example "3 possible misspellings." At high verbosity it also says "No misspellings."

These are never checked: code, math, link addresses, web and e-mail addresses, citation keys such as `[@doe2020]`, raw HTML, front matter, words with digits, words in capitals (acronyms such as NASA), words with capitals inside (such as iPhone), and single letters.

Your word list is `words.txt` in the data folder, one word per line; you can edit it in any text editor. The [library guide](library.md) says where the data folder is.

## Grammar

Grammar checking is built in: it is in the terminal reader, the textweaver app, `tw`, and the release packages. A build made without it (see [Building without grammar](#building-without-grammar)) says "Grammar checking is not in this version." when you press the keys below.

textweaver checks grammar offline with Harper, which knows American English. It looks for things such as "a apple", "the results was", a word typed twice, and a missing capital letter. Spelling is left to the spelling keys above, so a misspelled word is not reported twice.

- **Ctrl+F7**: the next grammar problem. **Ctrl+Shift+F7**: the previous one. The words are selected. You hear "Grammar:" and Harper's description of the problem, then "The words:" and the words, then the first fix when there is one, for example "Fix: an. Alt J lists fixes." At high verbosity you also hear the line number.
- **Alt+J** on a grammar problem lists its fixes, then "Leave it as it is". In edit mode, **Enter** on a fix makes the change; that is one undo step.

In edit mode on a Markdown file, textweaver checks the Markdown you write and leaves out code, math, and link addresses. While reading, it checks the document's text. These keys work in both.

### Building without grammar

Grammar checking makes each program about 11 MB larger and a clean build about 7 minutes longer, because Harper brings its dictionary, its rules, and a part-of-speech tagger. Since beta 1, `tw` (and the terminal reader in it) always has it. The app can be built without it: turn off the default features and name the ones you want, for example:

```bash
cargo build --release -p textweaver-xilem --no-default-features --features screenshot,renderer-vello,publish,lint,dictation,audio-export,opus,carta
```

See [Building](dev/building.md).

## Markdown lint

Some Markdown problems are plain to see and easy to miss by ear. In edit mode on a Markdown file, textweaver finds them for you:

- **Ctrl+F8**: the next problem. **Ctrl+Shift+F8**: the previous one. The problem is selected and you hear what it is, starting with "Lint": "Lint: heading level 3 after level 1; use level 2." At high verbosity you also hear the line number.

It checks five things:

- **Heading levels.** A heading more than one level below the one before it, such as a level 3 heading right after a level 1, leaves a gap in the outline and in a screen reader's list of headings.
- **List markers.** A bullet list that changes from `-` to `*` or `+` partway through. Markdown starts a new list there.
- **Trailing spaces.** Spaces or tabs at the end of a line, which you cannot hear. Exactly two spaces before more text are left alone, because they make a line break.
- **Link references.** `[text][intro]` or `[intro][]` with no `[intro]: address` line anywhere, which shows as plain brackets.
- **Bare web addresses.** An address typed into a sentence. Put it in angle brackets, `<https://example.org>`, or make it a link with a name, `[the course page](https://example.org)`.

Code blocks, inline code, math, front matter, and HTML are not checked, except for trailing spaces outside code blocks.

To check files from the command line, use `tw lint`. It prints each file, how many problems it has, and one line per problem, for example `Line 3: heading level 3 after level 1; use level 2.` It ends with status 1 when there are problems, so a script can check. `--json` prints them as JSON.

```bash
tw lint essay.md notes.md
```

## Listen to the rendered text

In edit mode, type `listen rendered` in the command palette. textweaver reads from the cursor what a reader of your finished document hears: no `#`, `*`, or link addresses, and citations formatted, such as "(Doe & Roe, 2020, p. 12)". You stay in edit mode, and the highlight follows in your Markdown. **Escape** stops. Outside edit mode it reads from the cursor, as **Enter** does.

## Export and preview

Type `export pdf`, `export docx`, `export html`, `export epub`, or `export brf` in the command palette to write the document you are editing, saved or not, in that format. textweaver first asks where, offering the document's name and folder with the format's extension; Enter accepts it. You hear "Exporting to PDF.", then, for a long export, "Still exporting to PDF, 2 seconds." and every ten seconds after. The [converting guide](converting.md#export-from-inside-the-reader) explains exports.

### The preview pane in the app

In the app, the preview can stand beside the editor instead of in a browser: **Alt+F5**, Show preview in the View menu, or `show preview` in the palette turns it on or off, and the choice is kept (`pane` in `[preview]`, off by default). It shows the document as the reading view draws it, parsed by textweaver's own Markdown reader rather than a web engine, and it is redrawn once you pause typing for 300 milliseconds (`pane_delay_ms`, 100 to 3000). The block you are editing is scrolled into view and marked with a band and an underline; the focus stays in the editor and nothing is announced, so the preview never interrupts your screen reader. F6 moves into it to read, and back. The [window guide](gui.md#the-preview-pane) describes narrow windows and the keys in full.

### Preview in the browser

`preview in browser` (File menu, Preview) opens the document as a web page, with math as MathML, in your default browser. The first preview of a session tells you, in one sentence, what the browser will do from then on; with the default setting you hear "Preview opens in your browser. Press F5 there after each save." Each save (**Ctrl+S**) writes the preview again, and you hear "Preview updated. Press F5 in the browser." By default the page does not reload by itself, because a reload returns your screen reader to the top of the page, and you should decide when that happens.

**Browser preview follows.** One setting decides whether, and when, the page reloads by itself. It has three values:

- **Nothing** (the default): the page changes only when you press F5 in the browser.
- **Each save**: the page reloads after every save.
- **Your typing**: the page reloads after every save, and also whenever you pause in your typing, without saving.

Choose it from the View menu (Browser preview follows), from the palette (`browser preview follows`, which moves to the next value and says it), or in Settings under Preview, where F1 explains it. Each choice is saved, and choosing again returns it to nothing. In `settings.toml` it reads:

```toml
[preview]
follow = "save"        # "off", "save", or "typing"
pane_delay_ms = 300    # the pause before following your typing
```

When the setting is not "nothing", run `preview in browser` again. The page now comes from a small web server that textweaver runs on your own computer only: the address starts `http://127.0.0.1:` and carries a random secret, so no other computer or program can read your document. After a reload the page scrolls to the heading nearest your cursor and puts the focus there, so your screen reader lands near the place you edited, and you hear "Preview updated."

- A reload still moves your screen reader's place to that heading, which is why the page follows nothing by default.
- The pause before following your typing is `pane_delay_ms`, 300 milliseconds unless you change it (100 to 3000). The side-by-side preview pane uses the same pause, so the two keep step. Reloads while you type are shown on the status line and not spoken.
- The server stops when you open another document or quit textweaver. Choosing nothing stops it at once; the browser then keeps the last page it had.
- Images and other files beside your document are served too, but nothing outside the document's folder.
- Older settings files keep working. `auto_reload = true` is read as following each save, and `auto_reload = true` with `live = true` as following your typing. The next save of your settings writes only `follow`.

### Preview in the terminal reader

In edit mode, **Shift+F4** (the palette's `Preview`, or Edit menu, Preview) replaces the Markdown source on screen with the reading view of the same document: headings, lists, and emphasis as the reader shows them, without the marks. You stay in edit mode, and you hear "Preview, read-only: the reading view at the same place." **Shift+F4** again returns to the source.

- **The place is kept.** The caret moves to the same character in the reading view. If it was inside markup, such as between the asterisks of `**bold**` or in a link's address, it lands on the nearest text, because that markup is not shown. Returning without moving puts the caret exactly where it was; returning after moving puts it on the same text in the source.
- **The preview is read-only.** Reading and navigation keys work as they do when reading. Typing says "Preview is read-only." Editing, file, and bookmark commands, such as Save, return to the source first and then run.
- **The view is current.** Until you edit, the preview is the document as it was opened; after an edit, the text you are editing is read again by the same Markdown reader.
- **No split view.** The terminal shows the source or the preview, never both side by side. A split would halve what each line of the screen, and so each line of a Braille display, can show, for no gain over switching.
- Plain-text files look the same in both views, so there the command says there is nothing to preview.

## Review a Word document's tracked changes

Reviewing a colleague's tracked changes is reading work more than writing work, so it lives in the reader: open the Word document, press **Ctrl+Shift+J** (or **Alt+A** in the terminal reader), and accept, reject, reply, and resolve from the changes list. [Tracked changes and comments](reading.md#tracked-changes-and-comments-ctrlshiftj-or-alta) in the reading guide describes the list and its keys.

When you are done, **Save changes to the Word file** writes your decisions into the original `.docx` in place, keeping a copy of the original beside it the first time (`report-original.docx`). textweaver changes only the marks your decisions touch, so the document's styles, numbering, headers, and anything else Word stored come back unaltered when your colleague opens it. Comments and replies you add are signed with `[authoring] author`, or "textweaver" while that setting is empty; textweaver never takes a name from your computer.

The changes list does not accept or reject while edit mode is on. Leave edit mode to review.

### Your own edits as tracked changes

Edit mode can also hand your own edits back to a Word user as tracked changes. Turn on **Track changes in Word files** in the settings (the Authoring section, or `[authoring] track_changes = true`; F1 on the setting explains it). It is off by default, because most edits are meant to stand on their own.

With it on, saving edit mode's work on a `.docx` file (**Ctrl+S**, or Save on leaving edit mode) no longer asks for a Markdown name. textweaver compares the text with what was last saved, word by word, and writes each change into the Word file itself: deleted words as a deletion and new words as an insertion, signed with `[authoring] author` (or "textweaver") and the time from your computer's clock. A reviewer opening the file in Word sees them in the Review tab and accepts or rejects them like any colleague's. The first save keeps a copy of the original beside it, as the review save does, and you hear "Saved 2 tracked changes in report.docx. The original is kept as report-original.docx."

Edit mode works on the document's text as Markdown, so what is tracked is wording, not formatting: making a word bold is not recorded. A change that runs across a paragraph break, or that falls inside a link or a field, cannot be placed as a tracked change. In that case textweaver writes nothing at all, says how many changes could not be tracked, and leaves your edits in the editor; **Save As** (Alt+S) keeps them as a Markdown file.

## Start from a template

Type `new from template` in the command palette. The list has three templates, Essay, Report, and Notes, and your own after them. Choose one, then type the title. textweaver starts a new document in edit mode with front matter, headings, and a References heading, for example:

```markdown
---
title: "On Bees"
author: "Jo Writer"
date: 2026-09-26
---

# On Bees

## Introduction
```

The date is today's date on your computer, in your time zone. The author comes from this setting, when you set it:

```toml
[authoring]
author = "Jo Writer"
```

The same name signs the comments and replies you add to a Word document's review, and your edits saved to a Word file as tracked changes (see below). Older settings files that put it under `[editing]` still work: the name is read as `[authoring] author`.

The cursor starts under the first section heading, and the document is new and unsaved: save it with **Ctrl+S**.

Your own templates are Markdown files in the `templates` folder of the configuration folder (the folder that holds `settings.toml`; the [settings guide](settings.md) says where). Write `{{title}}`, `{{author}}`, and `{{date}}` where those should go. The file name, without `.md`, is the template's name in the list.

## If something goes wrong

- **"Could not save: the file is read-only".** Use Save As (**Alt+S**) with a new name, or make the file writable.
- **A key types instead of running a command.** In edit mode only chords run commands. Press **Ctrl+E** to go back to reading.
- **A key such as Ctrl+Alt+Down does nothing, or switches your desktop.** Some desktops keep those keys. Give the command other keys in `keymap.toml`; see the [keyboard reference](keyboard.md).
- **A formatting key does nothing.** Your terminal may keep that chord. Run the command from the command palette (**F2**), for example `bold`, or give it another key in `keymap.toml`; see the [keyboard reference](keyboard.md).
- **"Could not write the recovery copy".** The data folder may be full or read-only. Your editing goes on; save soon.

## See also

- [Reading and moving around](reading.md): reading, navigation, find, and go to.
- [Bookmarks, notes, and highlights](notes.md): notes move with your edits.
- [Dictation](dictation.md): voice typing with `tw dictate`.
- [Citations](citations.md): writing citations and bibliographies.
- [Math](math.md): writing and reading math.
- [Keyboard reference](keyboard.md): every key in both frontends.
- [Documentation index](README.md)
