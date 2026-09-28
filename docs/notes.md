# Bookmarks, notes, and highlights

This guide covers the three ways to mark what you read: bookmarks (named places), notes (your own text attached to a passage), and highlights (passages marked in colour). It is for students and anyone who studies with textweaver. It also covers where they are kept, how to list them from the command line, and how to take them to Obsidian.

Keys are the terminal defaults. Most are single browse keys, which work while reading. Where the GUI uses a different key, this guide says so. With single-key shortcuts turned off (**F9**), run these commands from the command palette (**F2**) by the names given here.

## Bookmarks

A bookmark is a named place in a document.

### Add a bookmark: m

Press **m**. The bookmark goes on the word being read, or on the word at the cursor. It is named `mark1`, `mark2`, and so on, using the first free name. You hear "Bookmark mark1 set at 42 percent." If a bookmark is already on that word, you hear "Bookmark", its name, "is already here." The GUI also has **Ctrl+M**.

### Move between bookmarks: b and Shift+B

- **b**: the next bookmark.
- **Shift+B**: the previous bookmark.

You hear "Bookmark", the name, and a preview of the text. After the last one, textweaver wraps to the first and says "Wrapped." With no bookmarks you hear "No bookmarks."

### List bookmarks: Shift+M

Press **Shift+M**. You hear "Bookmarks", the count, then "Enter goes to one, Delete deletes it, F2 renames it." Each item says the name, the line, the percentage, and the start of the text.

In the list:

- **Enter** goes to the bookmark.
- **F2** renames it. The prompt says "New bookmark name, Enter keeps it". Type the new name and press **Enter**. You hear "Bookmark mark1 renamed to", then the new name. A name another bookmark already uses is refused.
- **Delete** deletes it at once, without asking. You hear "Bookmark", the name, "deleted." The list stays open on the next bookmark.
- **Escape** closes the list.

The command palette also has `rename_bookmark` and `delete_bookmark`. Each acts on the bookmark at the cursor, or opens the list when there is none there.

## Notes

A note is your own text attached to a passage: the selection, or the sentence at the cursor.

### Add a note: a

1. To attach the note to more or less than one sentence, select the text first with **Shift** and the arrow keys. Otherwise the note goes on the sentence being read, or the sentence at the cursor.
2. Press **a**, or **Alt+N**, which also works in edit mode and with single-key shortcuts off. The prompt says "Note".
3. Type the note and press **Enter**.

You hear "Note added on:", then the start of the passage.

Words starting with `#` in a note become tags. For example, "Check this for the #exam" is tagged `exam`. Tags are stored in lower case, without the `#`. When a note has tags you hear "Note added with tags exam on:" and the passage.

The note also keeps a copy of the passage it was attached to, up to 120 characters. That copy is its anchor. It keeps the note meaningful if the text later changes.

### Move between notes: e and Shift+E

- **e** or **F12**: the next note.
- **Shift+E** or **Shift+F12**: the previous note.

In the classic keys (`preset = "classic"` under `[keyboard]`), **Alt+Down** and **Alt+Up** step through notes as before. By default they move by sentence.

You hear "Note 2 of 5:", the note, then "On:" and the passage. After the last note, textweaver wraps to the first and says "Wrapped." With no notes you hear "No notes."

These keys step through notes only, not highlights.

### Hear that a note is here

While textweaver reads, it signals each note's passage once, as the reading reaches it: a short rising two-tone sound, when the voice can play tones. At normal verbosity and above, the status line also shows the note, for example "Note: Check this for the exam", without interrupting the reading.

When a word move (Left or Right) takes you into a note's passage, you hear the word and then "Has a note:" and the start of the note, at normal verbosity and above.

### Export a study sheet

Press **F2** for the command palette and type `export study sheet`. textweaver writes your notes and highlights as a Markdown file next to the document, named after it: `essay.md` gives `essay-study-sheet.md`. You hear how many notes and highlights went in and where the file is, then "Open it? y or n."

The study sheet is grouped by the headings of the document, in order, so it follows the structure of what you read. Each passage is quoted, with your note under it:

```markdown
# Study sheet: Essay

Exported from textweaver on 2026-09-26.

## Methods

- > We measured things carefully.

  Check the method (tags: exam)
```

Highlights are listed the same way, with their colour. A new document that was never saved has no folder yet; its study sheet goes to the folder textweaver was started in.

### List notes: Shift+A

Press **Shift+A**. The GUI also has **Ctrl+Shift+N**. You hear "Notes", the count, then "Enter goes to a note, Delete deletes it, F2 edits it." Each item says the note, the line, and the passage.

In the list:

- **Enter** goes to the note.
- **F2** edits the note. You hear "Editing note:" and its text. The prompt says "Edit note, Enter keeps it". Type the new text and press **Enter**; tags are read again from the new text. You hear "Note updated." Enter on an empty prompt leaves the note as it was.
- **Delete** asks "Delete this note? y or n". Press **y** to delete it; you hear "Note deleted:" and the start of the note. Press **n**, **a**, or **Escape** to keep it; you hear "Kept." and the list comes back.
- **Escape** closes the list.

This list shows notes only. Highlights have their own list.

## Highlights

A highlight marks a passage, as a highlighter pen does on paper.

### Highlight: y

1. Select the text with **Shift** and the arrow keys, or leave nothing selected to highlight the sentence at the cursor.
2. Press **y**.

You hear "Highlighted:" and the start of the passage. At high verbosity you also hear the percentage.

Press **y** again on a highlighted passage, with nothing selected, to remove the highlight. You hear "Highlight removed:" and the passage.

### Highlight colours

Highlights made in textweaver are yellow. There is no key to choose another colour yet.

Highlights imported from Star, or from a synced folder, can have other colours. textweaver knows five by name: yellow, green, cyan, pink, and orange. Other colours are kept and shown by their code. On screen a highlight is marked by the theme's highlight style, which never relies on colour alone.

### List highlights: Shift+Y

Press **Shift+Y**. You hear "Highlights", the count, then "Enter goes to one, Delete removes it." Each item says the passage, the line, and the colour name.

In the list, **Enter** goes to the highlight, **Delete** asks "Remove this highlight? y or n" and removes it on **y**, and **Escape** closes the list. In the command palette this command is `list_highlights`.

## Delete a note or highlight at the cursor: Delete

When reading, move to a note or a highlight and press **Delete**. textweaver asks "Delete this note or highlight? y or n". Press **y** to delete it, or **n**, **a**, or **Escape** to keep it. A note under the cursor is deleted before a highlight. With nothing there you hear "No note or highlight here."

Delete in the notes and highlights lists asks the same way. Only the bookmarks list deletes at once, without asking.

## How marks move when you edit

Bookmarks, notes, and highlights are tied to the text, not to a line number. When you edit the document (see [Writing and editing](editing.md)), they move with the text around them:

- Text typed before a mark pushes it along.
- A note whose whole passage is deleted is kept. It stays at the place of the deletion, and its anchor still holds the old passage.
- A highlight whose whole passage is deleted is removed.
- A bookmark whose word is deleted moves to the place of the deletion.

### When the file changes in another program

Your document may change while textweaver is closed: you edit it in Obsidian, `git pull` brings a new version, or another program rewrites it. textweaver keeps the text each mark was on: about 40 characters for your reading position and each bookmark, and the passage for each note and highlight. When you open the changed document, it finds each mark again:

1. its text near where it was;
2. its text anywhere else in the document, the nearest place winning, for a paragraph that moved;
3. the most similar text nearby, for a small edit inside the passage itself;
4. failing all three, the same share of the way through the document. Such a mark is marked as not found.

Marks still on their text are left alone. textweaver says once what moved and what it could not find, for example: "The file changed; 3 bookmarks were moved to match, 1 bookmark could not be found and is marked."

## Where they are stored

Everything about one document is kept in one file in the state folder: the reading position, the history of jumps, the bookmarks, the notes, and the highlights. The state folder is `state` inside the data folder. The [library guide](library.md) says where the data folder is on each system.

The file is named after the document's file name plus a code made from its full path, for example `essay.md-9f3c01a2b4d5e6f7.json`. So two files with the same name in different folders keep separate marks. Moving or renaming a document starts it with no marks.

Bookmarks, notes, and highlights are saved as soon as you change them. The reading position is saved when you quit, when you open another document, and every 30 seconds while it changes. The saving happens in the background, so a slow disk never holds up a key; quitting waits for it, and says "Still saving" if it takes more than a moment.

While you are in edit mode, marks are saved when you leave edit mode.

If a state file cannot be read, for example after a sync conflict or a hand edit, textweaver does not overwrite it. It renames it to `<name>.corrupt-<time>.bak` and starts that document with no marks, so you can recover them from the copy. See [Troubleshooting](troubleshooting.md).

## List marks from the command line: tw marks

```bash
tw marks essay.md
```

`tw marks` prints a document's saved reading position, its bookmarks, its notes, and its highlights, each with its percentage, character position, line, and saved time, plus the text of that line. If the document is in a library folder, it also prints the position synced from other computers. It only reads; it never changes anything.

For a program or a script, print JSON instead:

```bash
tw marks essay.md --json
```

The JSON also has the document's state key and its history of jumps.

## Export notes as references: tw marks --export

Your notes and highlights can go into a reference manager such as Zotero, or into a BibTeX file, as Star's notes export did. Each note and each highlight becomes one record:

- the title and author are the document's;
- the passage you noted or highlighted is the record's abstract;
- your note is the record's note, followed by "Cites doe2020." when the note has a citation key; a highlight's note is its color, for example "Highlighted, yellow.";
- your tags are its keywords;
- its date is the day you made the note.

Choose the format after `--export`: `bibtex`, `biblatex`, `ris`, or `json` (CSL-JSON, which Zotero and Pandoc read):

```bash
tw marks essay.md --export ris --output essay-notes.ris
```

Without `--output`, the records are printed. Keys are made from the file name: `essay-note-1`, `essay-note-2`, `essay-highlight-1`. In CSL-JSON each record also says where it is, for example "34 percent".

## Export to an Obsidian vault

`tw vault export` writes a document's notes and highlights into an Obsidian vault as Markdown notes. Name each document with `--document`:

```bash
tw vault export C:\Users\me\Vault --document essay.md
```

[The vault guide](vault.md) explains the options, the notes it writes, and importing from a vault.

## Sync between computers

When a document is in a library folder, its reading position is copied to a small file in that folder, `.textweaver/progress.json`. If the folder is synced by Dropbox, OneDrive, Syncthing, or iCloud, another computer picks up your place. Only the position and a count of notes travel this way; the notes, highlights, and bookmarks themselves stay on the computer where you made them. [The library guide](library.md) explains sync and how conflicts are settled.

## If something goes wrong

- **"Nothing here to attach a note to."** The cursor is on an empty line. Move to text, or select some.
- **A mark is missing after reopening.** The document may have moved or been renamed; marks follow the full path. Check with `tw marks` on the old path.
- **A note is in the wrong place after an edit outside textweaver.** textweaver looks for the note's passage again when the file changed (see [When the file changes in another program](#when-the-file-changes-in-another-program)). If the passage was rewritten or deleted, the note could not be found: it is marked, and put at the same share of the way through the document. The note's anchor still shows the passage it was made on.
- **The keys do nothing.** Single-key shortcuts may be off. Press **F9**, or use the command palette names: `add_bookmark`, `list_bookmarks`, `next_bookmark`, `previous_bookmark`, `add_note`, `list_notes`, `next_note`, `previous_note`, `highlight_selection`, `list_highlights`, and `delete_note`.

## See also

- [Reading and moving around](reading.md): selecting text, and the history of jumps.
- [Writing and editing](editing.md): edit mode, where marks move with your edits.
- [The library](library.md): library folders, sync, and where the state folder is.
- [Obsidian vaults](vault.md): exporting notes and highlights to Obsidian.
- [Keyboard reference](keyboard.md): every key in both frontends.
- [ADR-0002: Text model](adr/0002-text-model.md): how positions and marks move with edits.
- [Documentation index](README.md)
