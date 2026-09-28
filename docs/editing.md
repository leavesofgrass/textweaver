# Writing and editing

This guide covers edit mode in the terminal reader: typing with spoken feedback, Markdown formatting commands, undo, find and replace, saving, and getting back unsaved work after a crash. It is for anyone who writes or corrects documents in textweaver.

Keys are the terminal defaults. Where the GUI uses a different key, this guide says so. The [keyboard reference](keyboard.md) lists every key.

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
- **Backspace** deletes the character before the caret. **Delete** deletes the one after it. A character is what you see as one: an emoji with its skin tone, a flag, or a letter with its accent is deleted whole.
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
- **Alt+Backspace**: delete the word before the caret. The GUI uses **Ctrl+Backspace**.
- **Ctrl+Delete**: delete the word after the caret.
- **Ctrl+C**: copy the selection. **Ctrl+X**: cut it. Both go to your computer's clipboard through the terminal; see [Copy, cut, and paste](#copy-cut-and-paste).
- **Ctrl+V**: paste. See [Copy, cut, and paste](#copy-cut-and-paste).

You hear what the word keys deleted, for example "three deleted."

## Structure while you write

The reading keys that move by structure work in edit mode too, on the Markdown as you have written it so far:

- **Alt+H** and **Alt+Shift+H**: the next and previous heading. The caret lands on the heading's text, after the `#` marks.
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

textweaver keeps the last 1,000 steps, or 50 MB of them, whichever comes first; the oldest are forgotten. `undo_steps` and `undo_memory_mb` in `[editing]` change the limits (see [Settings](settings.md#editing)).

## Find and replace: Alt+F

Press **Alt+F** in edit mode. The GUI uses **Ctrl+Shift+F**.

1. The prompt says "Replace, find what". Type the text to find and press **Enter**. You hear how many matches there are, then "Replace with?"
2. Type the new text and press **Enter**.

textweaver then goes through the matches one at a time, starting at the caret. Each match is selected, and you hear where it is and its line, for example "Match 2 of 5, line 12: the cat sat on the mat." A short list asks what to do. Press a letter, or move with Up and Down and press Enter:

- **r**, "Replace this one": replaces it and goes to the next match.
- **s**, "Skip this one": leaves it and goes to the next match.
- **a**, "Replace all the rest": replaces this match and every one after it.
- **c**, "Match case": on or off. Off (the default), `cat` also finds `Cat`. You hear the new setting and how many matches there are now.
- **w**, "Whole words only": on or off. On, `cat` does not find `catalog`.

After the last match, the search goes on from the top of the document and stops where it started. At the end you hear what was done, for example "Replaced 3, skipped 1." Press **Escape** to stop early: "Stopped. Replaced 1, skipped 0."

Each replacement is one undo step, and "Replace all the rest" is one step for all of them, so **Ctrl+Z** takes back the last thing you chose.

To find without replacing, use **Ctrl+F**, as when reading.

## Save

### Save: Ctrl+S

Press **Ctrl+S**. You stay in edit mode. You hear "Saved", the file name, and "Still editing."

The file is written in the background, so a large file never holds up the keyboard: you can go on typing, and "Saved" comes as soon as the file is on disk. What you type while it is being written is not in that save. Quitting waits for every save to finish; if the disk is slow, you hear "Still saving. Please wait."

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

- **Before saving.** If the file changed since you opened or last saved it, nothing is written, and **Ctrl+S** asks first: the file name, then "changed on disk since you opened it. Save over those changes? y or n." Press **y** to save over them. Press **n** to keep editing without saving; you hear "Not saved. Still editing. Save As, Alt+S, keeps both versions."
- **While reading or editing with no unsaved changes.** textweaver checks every two seconds, in the background. When the file changed, it asks: the file name, then "changed on disk. Reload it? y or n." Press **y** to load the new version, or **n** to keep the one you have ("Kept the open version."). It does not ask while it is reading aloud, while another question is open, or while you have unsaved changes.
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

- **Ctrl+C** copies the selection. **Ctrl+X** cuts it; the cut is one undo step. textweaver sends the text to the terminal, which puts it on your computer's clipboard (the OSC 52 sequence; it works over SSH too). Windows Terminal, iTerm2, kitty, WezTerm, foot, Alacritty, and xterm pass it on.
- The old Windows console window, macOS Terminal, and terminals built on VTE (GNOME Terminal, Tilix) do not take text that way, so there textweaver puts it on the system clipboard itself. The first time, it says "Copied with the system clipboard, because this terminal cannot take copied text." In Konsole it does both. Over SSH and in tmux it always uses the terminal, because the system clipboard there belongs to the other computer.
- To paste from your computer's clipboard, use your terminal's paste command, such as **Ctrl+V** or **Ctrl+Shift+V** in Windows Terminal, or right-click.
- When the terminal passes **Ctrl+V** to textweaver instead of pasting, textweaver pastes the text you last copied or cut in textweaver. When there is none, it says "Nothing copied in textweaver yet. Use your terminal's paste, for example Control Shift V."

textweaver turns on bracketed paste, so the terminal sends pasted text as one piece, not as separate key presses. That means:

- the paste is one undo step;
- keys in the pasted text never run commands;
- you hear "Pasted", the number of characters, and the first words.

Pasting into a prompt, such as Find, puts the text in the prompt.

## Citations

In edit mode, **Alt+C** inserts a citation: pick a reference from a list you can filter by typing, then give a page or other locator. **Alt+B** (GUI **Alt+Shift+D**) adds a reference by DOI or ISBN. The command palette has `insert bibliography`, `check citations`, and `import references`. The [citations guide](citations.md#citations-while-reading-and-writing-in-textweaver) explains them.

## Spelling

textweaver checks spelling against a list of 225,038 English words (SCOWL, sizes 35 to 80, which includes American, British, Canadian, and Australian spellings) and your own word list.

- **Alt+M**: the next misspelled word. **Alt+Shift+M**: the previous one. You hear the word, then its letters: "recieve. r e c i e v e." In edit mode the word is selected, so typing replaces it. These keys work while reading too.
- **Alt+J**: suggestions for the misspelled word at the cursor, closest first, then "Add recieve to your word list" and "Leave it as it is". In edit mode, **Enter** on a suggestion replaces the word; that is one undo step.
- When you save, textweaver says how many possible misspellings are left, for example "3 possible misspellings." At high verbosity it also says "No misspellings."

These are never checked: code, math, link addresses, web and e-mail addresses, citation keys such as `[@doe2020]`, raw HTML, front matter, words with digits, words in capitals (acronyms such as NASA), words with capitals inside (such as iPhone), and single letters.

Your word list is `words.txt` in the data folder, one word per line; you can edit it in any text editor. The [library guide](library.md) says where the data folder is.

## Grammar

Grammar checking is off in the standard build, because it adds about 10 MB. To try it, build the reader with `cargo build --release -p textweaver-tui --features grammar`; without it, the keys below say that grammar checking isn't in this build.

textweaver checks grammar offline with Harper, which knows American English. It looks for things such as "a apple", "the results was", a word typed twice, and a missing capital letter. Spelling is left to the spelling keys above, so a misspelled word is not reported twice.

- **Ctrl+F7**: the next grammar problem. **Ctrl+Shift+F7**: the previous one. The words are selected. You hear "Grammar:" and Harper's description of the problem, then "The words:" and the words, then the first fix when there is one, for example "Fix: an. Alt J lists fixes." At high verbosity you also hear the line number.
- **Alt+J** on a grammar problem lists its fixes, then "Leave it as it is". In edit mode, **Enter** on a fix makes the change; that is one undo step.

In edit mode on a Markdown file, textweaver checks the Markdown you write and leaves out code, math, and link addresses. While reading, it checks the document's text. These keys work in both.

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

In edit mode, type `listen rendered` in the command palette. textweaver reads from the caret what a reader of your finished document hears: no `#`, `*`, or link addresses, and citations formatted, such as "(Doe & Roe, 2020, p. 12)". You stay in edit mode, and the highlight follows in your Markdown. **Escape** stops. Outside edit mode it reads from the cursor, as **Enter** does.

## Export and preview

Type `export pdf`, `export docx`, `export html`, `export epub`, or `export brf` in the command palette to write the document you are editing, saved or not, next to it in that format. You hear "Exporting to PDF.", then, for a long export, "Still exporting to PDF, 2 seconds." and every ten seconds after. The [converting guide](converting.md#export-from-inside-the-reader) explains exports.

### Preview in the browser

`preview in browser` opens the document as a web page, with math as MathML, in your default browser. Each save (**Ctrl+S**) writes the preview again, and you hear "Preview updated. Press F5 in the browser." By default the page does not reload by itself: a reload puts your screen reader back at the top of the page, so you choose when it happens.

**Automatic reloading.** Run `toggle preview auto reload` from the palette, or set it in `settings.toml`:

```toml
[preview]
auto_reload = true
live = false
```

Then run `preview in browser` again. The page now comes from a small web server that textweaver runs on your own computer only (the address starts `http://127.0.0.1:`, with a random secret in it, so no other computer or program can read your document). After each save the page reloads by itself, then scrolls to the heading nearest your caret and puts the focus there, so your screen reader lands near the place you edited. You hear "Preview updated."

- A reload still resets your screen reader's place in the page to that heading, which is why automatic reloading is off by default.
- `toggle preview live` (or `live = true`) also reloads the page when you pause typing for a second, without saving. It needs automatic reloading on. Live reloads are shown on the status line and not spoken.
- The server stops when you open another document or quit textweaver. Turning automatic reloading off stops it at once; the browser then shows the last page it had.
- Images and other files beside your document are served too, but nothing outside the document's folder.

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
[editing]
author = "Jo Writer"
```

The caret starts under the first section heading, and the document is new and unsaved: save it with **Ctrl+S**.

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
