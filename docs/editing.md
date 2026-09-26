# Writing and editing

This guide covers edit mode in the terminal reader: typing with spoken feedback, Markdown formatting commands, undo, find and replace, saving, and getting back unsaved work after a crash. It is for anyone who writes or corrects documents in textweaver.

Keys are the terminal defaults. Where the GUI uses a different key, this guide says so. The [keyboard reference](keyboard.md) lists every key.

## Start and finish editing: Ctrl+E

Open a document, then press **Ctrl+E**. textweaver stops reading and says "Edit mode on." It then names the keys to save and to finish, and reads the line the cursor is on. At low verbosity it says only "Edit mode on." and the line. The title line shows "Edit".

Press **Ctrl+E** again to finish. textweaver says "Edit mode off." If you have unsaved changes, it asks first; see [Leaving with unsaved changes](#leaving-with-unsaved-changes).

With no document open, **Ctrl+E** says "No document to edit. Press Control N for a new one."

### What changes in edit mode

Edit mode shows the document's source, the text that is saved:

- A plain-text file shows its own text.
- A Markdown file shows its Markdown, with `#`, `*`, and the other marks.
- Any other format (HTML, EPUB, Word, PDF) shows the document converted to Markdown. Saving it writes a new Markdown file; the original is never changed.

Your place, bookmarks, notes, and highlights move with your edits. When you finish, textweaver rebuilds the reading view from the saved file and puts them back in the matching places.

In edit mode every key that is not a command types. Single-key reading keys such as `.` and `p` type their character. Chords such as **Ctrl+F** (find), **Ctrl+G** (go to), and **Ctrl+Home** still work.

## Type and move

- Letters, digits, and punctuation type as usual. **Enter** starts a new line. **Tab** types a tab.
- **Backspace** deletes the character before the caret. **Delete** deletes the one after it.
- **Left** and **Right**: move by character. You hear the character.
- **Ctrl+Left** and **Ctrl+Right**: move by word. You hear the word.
- **Up** and **Down**: move by line. You hear the new line.
- **Home** and **End**: the start or end of the line.
- **Ctrl+Home** and **Ctrl+End**: the start or end of the document.
- **PageUp** and **PageDown**: move one screen.
- Hold **Shift** with any of these to select. You hear the text added, then "selected", or the text removed, then "unselected".

At the edges you hear "Start of line.", "End of line.", "Top of document.", or "End of document."

textweaver has no copy or cut command yet. To paste, use your terminal's own paste command. See [Pasting](#pasting).

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

Each command changes the selection, or the line the caret is on. Each one is announced: for example "Bold." when it adds the marks, or "Bold removed." when the text was already bold. The commands toggle, so pressing a key twice undoes it. With nothing selected, bold, italic, and the other wrapping commands insert placeholder text and select it, so you can type over it.

- Bold: **Ctrl+B**. Writes `**text**`.
- Italic: **Alt+I**. The GUI uses **Ctrl+I**. Writes `*text*`.
- Underline: **Ctrl+U**. Writes `<u>text</u>`, because Markdown has no underline.
- Strikethrough: **Alt+D**. The GUI uses **Ctrl+Shift+X**. Writes `~~text~~`.
- Inline code: `` Alt+` ``, that is Alt with the backtick key. The GUI uses `` Ctrl+` ``. Writes the text between backticks.
- Code block: **Alt+K**. The GUI uses **Ctrl+Shift+K**. Fences the selected lines.
- Link: **Ctrl+K**. Writes `[text](https://)`. With nothing selected, `text` is selected so you can type the link text. With a selection, the address is selected so you can type it.
- Heading: **Alt+1**. The GUI uses **Ctrl+Alt+1**. Each press raises the level of the current line by one: level 1, then 2, up to 6. Pressing it on a level-6 heading removes the heading. You hear "Heading level 2." and so on.
- Bulleted list: **Alt+8**. The GUI uses **Ctrl+Shift+L**. Puts `- ` before each selected line.
- Numbered list: **Alt+7**. The GUI uses **Ctrl+Shift+O**. Numbers each selected line. Bulleted and numbered lists turn into each other.
- Block quote: **Alt+9**. The GUI uses **Ctrl+Shift+Q**. Puts `> ` before each selected line.
- Horizontal rule: **Alt+R**. The GUI uses **Ctrl+Shift+R**. Inserts `---` on its own line.
- Insert a table: **Alt+T**. The GUI uses **Ctrl+Shift+A**. textweaver asks "Table size, columns by rows, for example 3 by 2". Type the columns, then the rows, as `3 by 2`, `3x2`, or `3 2`. Enter alone makes 2 by 2. Up to 20 columns and 100 rows. The first header cell is selected afterwards.
- Add a table row: **Alt+W**. The GUI uses **Ctrl+Shift+Enter**. Adds an empty row to the table at the caret; the caret goes to its first cell.
- Insert an image: **Alt+G**. The GUI uses **Ctrl+Shift+I**. textweaver asks for the image file. It writes `![name](path)` and selects the description, so you can type a better one.

The Alt chords in the terminal replace GUI chords that terminals cannot send, such as **Ctrl+I**, which arrives as Tab.

Writing citations and math has its own guides: [citations](citations.md) and [math](math.md).

## Undo and redo

- **Ctrl+Z**: undo.
- **Ctrl+Y**: redo. The GUI also has **Ctrl+Shift+Z**.

Typing and deleting are grouped into word-sized steps, so one undo removes about one word. Every formatting command, a paste, and a Replace All are each one step. You hear "Undo." or "Redo." and the current line. With nothing left, you hear "Nothing to undo." or "Nothing to redo."

## Find and replace: Alt+F

Press **Alt+F** in edit mode. The GUI uses **Ctrl+Shift+F**.

1. The prompt says "Replace, find what". Type the text to find and press **Enter**. Matching ignores case. You hear how many matches there are, then "Replace with?"
2. Type the new text and press **Enter**. Every match is replaced at once. You hear "Replaced", then the count.

The whole replacement is one undo step, so **Ctrl+Z** puts everything back. Press **Escape** at either prompt to cancel.

To find without replacing, use **Ctrl+F**, as when reading.

## Save

### Save: Ctrl+S

Press **Ctrl+S**. You stay in edit mode. You hear "Saved", the file name, and "Still editing."

- A Markdown or plain-text file is saved in place. Its byte-order mark and line endings are kept.
- Any other format (HTML, EPUB, Word, PDF, and so on) is never overwritten. textweaver asks for a new file name ending in `.md`: "Save as, Enter for", then a suggested name. Press **Enter** to accept it, or type another. After that, **Ctrl+S** saves the new file in place.

**Ctrl+S** when you are not editing says "Nothing to save. Turn on edit mode with Ctrl+E to make changes."

### Save As: Alt+S

Press **Alt+S** to save under a new name. The GUI uses **Ctrl+Shift+S**. The prompt suggests the current name. A name ending in `.md`, `.markdown`, `.txt`, or another plain-text or Markdown extension is kept. Any other ending becomes `.md`, so Markdown never lands in an `.html` or `.docx` file. A name without a folder goes in the same folder as the suggestion.

Save As writes over a file of the same name without asking. Check the name first.

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

- **Before saving.** If the file changed since you opened or last saved it, **Ctrl+S** asks first: the file name, then "changed on disk since you opened it. Save over those changes? y or n." Press **y** to save over them. Press **n** to keep editing without saving; you hear "Not saved. Still editing. Save As, Alt+S, keeps both versions."
- **While reading or editing with no unsaved changes.** textweaver checks every two seconds. When the file changed, it asks: the file name, then "changed on disk. Reload it? y or n." Press **y** to load the new version, or **n** to keep the one you have ("Kept the open version."). It does not ask while it is reading aloud, while another question is open, or while you have unsaved changes.

## Leaving with unsaved changes

When you finish editing, open another file, start a new document, or quit with unsaved changes, textweaver says the document's name, then "has unsaved changes. Save, discard, or cancel? Up and Down choose, Enter confirms, Escape cancels." A list appears with three choices:

- "Save, then continue": saves, then does what you asked.
- "Discard the changes": throws the changes away, then does what you asked. You hear "Changes discarded. Edit mode off."
- "Cancel, keep editing": you stay in edit mode.

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

## Pasting

Paste with your terminal's paste command, such as **Ctrl+V** or **Ctrl+Shift+V** in Windows Terminal, or right-click. textweaver turns on bracketed paste, so the terminal sends pasted text as one piece, not as separate key presses. That means:

- the paste is one undo step;
- keys in the pasted text never run commands;
- you hear "Inserted", then the number of characters.

Pasting into a prompt, such as Find, puts the text in the prompt.

## If something goes wrong

- **"Could not save: the file is read-only".** Use Save As (**Alt+S**) with a new name, or make the file writable.
- **A key types instead of running a command.** In edit mode only chords run commands. Press **Ctrl+E** to go back to reading.
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
