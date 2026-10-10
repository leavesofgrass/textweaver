# Obsidian vaults

`tw vault` moves your study notes between textweaver and an Obsidian vault. `tw vault export` writes the notes and highlights you made while reading into the vault as ordinary Markdown notes, linked to each other so they show up in Obsidian's graph. `tw vault import` reads a vault back: it brings your edits to those notes back onto their documents, and it turns your own Obsidian notes and their `[[links]]` into textweaver notes you can list and move through. Use it if you read in textweaver and organize or review in Obsidian, or if you keep a vault and want its links inside textweaver.

This guide is written to be read with a screen reader. Each section starts with the command, then explains it.

## Before you start

- A vault is simply a folder of Markdown files. Obsidian keeps its own settings in a folder called `.obsidian` inside it. `tw vault` works on the folder; Obsidian does not need to be open or even installed.
- The vault commands are part of `tw`, the command-line tool. The reader windows have no vault menu yet.
- Notes and highlights are made while you read. See [the notes guide](notes.md) for how to add them. To check what a document has, run `tw marks` with the document's file name. It lists the document's notes and highlights.
- textweaver keeps each document's notes and highlights in its own data folder, never inside the document. On Windows that is `%APPDATA%\leavesofgrass\textweaver\data\state`. On macOS it is `~/Library/Application Support/org.leavesofgrass.textweaver/state`, and on Linux `~/.local/share/textweaver/state`. If the `TEXTWEAVER_HOME` environment variable is set, it is `data\state` inside that folder.
- Import only reads the vault. It never changes, moves, or deletes a file in it.
- Export never overwrites a note you wrote yourself. It only replaces files that textweaver wrote before.
- Give the vault's full path, such as `D:\Notes\Biology`, not a short relative one such as `Biology`. Links found during an import are stored with the path exactly as you typed it, so a relative path only works when you run `tw` from the same folder again.
- The examples use a vault at `D:\Notes\Biology` and a document at `D:\Courses\lecture.md`. Put your own paths in their place. Paths with spaces need quotation marks around them.

## Options for tw vault

```powershell
tw vault --help
```

The command has this shape: `tw vault`, then `import` or `export`, then the vault folder, then any options. Some options belong to one action only. An option given to the other action is ignored without a warning.

- `--document FILE` (export only): a document whose notes and highlights to export. Export needs at least one. Repeat the option for several documents.
- `--folder FOLDER` (export only): a folder inside the vault for new notes, such as `textweaver`. Without it, new notes go in the vault's top folder.
- `--no-document-notes` (export only): do not write the document note that each document otherwise gets. Its highlights are then not exported either.
- `--mode graph` or `--mode library` (import only): what to import. `graph`, the default, brings in notes and their links. `library` adds the vault's notes to the library as documents and does nothing else; see [Library mode](#library-mode).
- `--link-relation NAME` (import only): the kind of link a plain `[[link]]` becomes, such as `SUPPORTS`. Without it, plain links become `SEE_ALSO`.
- `--dry-run` (import only): read the vault and report what an import would do, without storing anything.
- `--json` (both): print the full result as JSON, for scripts.

## Export notes and highlights to a vault

```powershell
tw vault export "D:\Notes\Biology" --document "D:\Courses\lecture.md"
```

This writes one Markdown note for every note on `lecture.md`, plus one document note for `lecture.md` itself that holds its highlights and links to its notes. If the vault folder does not exist yet, it is created.

To export several documents at once, repeat `--document`:

```powershell
tw vault export "D:\Notes\Biology" --document "D:\Courses\lecture.md" --document "D:\Courses\lab.md"
```

When it finishes, you hear one sentence, for example:

"Exported 2 notes and 1 highlight to Biology."

The name at the end is the vault folder's name. Other things you may hear:

- "lab has no notes or highlights." That document has nothing to export, so it is skipped. You hear the same if the file name is misspelled, so check it if you expected notes.
- "Nothing to export." None of the documents had notes or highlights. Nothing is written.
- "lecture could not be opened, so highlights are exported without their text." The notes are still exported, but each highlight's quote says only which characters it covers, such as "Characters 30 to 55", and percentages are left out.
- "...; 2 links left out because the linked note is not in the vault." A note links to a note on a document you did not export. See [If something goes wrong](#if-something-goes-wrong).

Without `--document`, export stops with "Name the documents to export with --document FILE". There is no option yet to export every document at once.

### What a note file holds

Here is a real exported note. The long folder path in `source` is replaced by `D:\Courses` to keep it short.

```markdown
---
textweaver_id: lec-n1
title: Mitochondria make energy.
source: D:\Courses\lecture.md
document: "[[Lecture 3 Energy]]"
position: 19
pct: 19
tags: [exam, review]
updated: "2026-09-26T07:35:20Z"
---

Remember this for the quiz on Friday. #review

## Links

- SUPPORTS:: [[Ribosomes protein builders]] - same lecture
```

The block between the two lines of three dashes is the front matter. Obsidian shows it as the note's properties. The keys, in the order they are written:

- `textweaver_id`: the note's id. textweaver uses it to find this file again, even after you rename it or move it to another folder in the vault. Do not change it.
- `title`: the text the note is attached to: the words you selected, or the sentence at the cursor. If there is no such text, the note's first line is used.
- `source`: the full path of the document the note belongs to.
- `document`: a link to the document's document note. It is left out with `--no-document-notes`.
- `position`: where the note is in the document, counted in characters from the start.
- `pct`: the same place as a percentage. It is left out when the document could not be opened.
- `tags`: the note's tags. This key is always written, as `[]` when there are none.
- `cite`: the note's citation, written only when it has one.
- `updated`: when the note last changed, in UTC.

Values that could confuse Obsidian, such as a title containing a colon, are put in quotation marks.

After the front matter comes the note's own text. Then, if the note links to other notes, a `## Links` section lists them. Each line is a Dataview inline field: a dash, the kind of link in capitals, two colons, and a link to the other note. A comment on the link, if there is one, follows after " - ". Obsidian's graph shows these links as lines between the notes, and the Dataview plugin can query them.

The kinds of link are `CONFLICTS_WITH`, `SUPPORTS`, `IS_EXAMPLE_OF`, `CITES`, `CONTRADICTS`, `DEFINES`, `EXTENDS`, `SEE_ALSO`, `PRECEDES`, and `FOLLOWS`. A kind textweaver does not know, written by a newer version, is exported in the same capitals and comes back unchanged on import. Any other field name the import does not know, such as `related::`, makes a plain link, as star did.

### What the document note holds

Each exported document also gets a document note, named after the document's title. Here is the real one for the lecture, with the path shortened the same way:

```markdown
---
textweaver_doc: 34d0c99a54db
title: "Lecture 3: Energy"
source: D:\Courses\lecture.md
tags: [textweaver-document]
---

# Lecture 3: Energy

## Highlights

%% textweaver-highlight id=h1 start=19 end=44 color=#ffff00 ts=1790400000 %%
> Mitochondria make energy. ^hl-h1

Highlighted yellow, 19%.

## Notes

- [[Mitochondria make energy]] (19%): Remember this for the quiz.
- [[Ribosomes protein builders]] (45%): Proteins are built here.
```

- The front matter has `textweaver_doc` (the document note's id), `title`, `source`, and the tag `textweaver-document`.
- Under `## Highlights`, each highlight has three parts. First, a comment between `%%` marks that holds the highlight's exact place and color; Obsidian hides these comments when it shows the note, and textweaver reads them back on import. Second, the highlighted text as a quote, ending with a block id such as `^hl-h1`. Third, a line saying its color and where it is.
- Quotes are the highlighted text with line breaks joined into spaces. A quote longer than 600 characters is cut short and ends with an ellipsis.
- The block id lets you link to one highlight from any note in Obsidian, for example `[[Lecture 3 Energy#^hl-h1]]`.
- Under `## Notes`, each of the document's notes is a link, with its place as a percentage and its first line.

### How files are named

- A note file is named after the note's `title`, that is, the text the note is about. If that is empty, the note's first line is used, and failing that, its id.
- A document note is named after the document's title. If the document has no title, its file name is used, without the extension.
- These characters cannot be in a name and become spaces: `<` `>` `:` `"` `/` `\` `|` `?` `*`, the square brackets `[` and `]`, `#`, `^`, and control characters such as tabs. Obsidian's links would break on the brackets, `#`, `^`, and `|`.
- Runs of spaces become one space. Dots and spaces at the end are removed, and so is a trailing `.md`.
- A name is at most 120 characters long.
- A name that would be empty becomes `note`. A name that Windows reserves for devices, such as `CON`, `NUL`, `COM1`, or `LPT1`, gets " note" added, as in `CON note`.
- If a name is already used in the folder, by your own note or by another note in the same export, a number is added: `The Cell 2`, `The Cell 3`, and so on. Upper and lower case count as the same name, as they do in Obsidian.

In the example, the note about "Ribosomes: protein builders" became `Ribosomes protein builders.md`, and "Lecture 3: Energy" became `Lecture 3 Energy.md`.

### Choose a folder for new notes

```powershell
tw vault export "D:\Notes\Biology" --folder textweaver --document "D:\Courses\lecture.md"
```

`--folder` puts new notes in a folder inside the vault, here `D:\Notes\Biology\textweaver`. The folder is created if needed. Notes that were exported before stay where they are, even if they are somewhere else in the vault.

### Leave out the document notes

```powershell
tw vault export "D:\Notes\Biology" --no-document-notes --document "D:\Courses\lecture.md"
```

This writes only the note files. Highlights live in the document notes, so with this option they are not exported, and the notes have no `document` key.

### Export again

```powershell
tw vault export "D:\Notes\Biology" --document "D:\Courses\lecture.md"
```

Running the same export again updates the notes it wrote before instead of adding copies. You hear, for example:

"Exported 2 notes and 1 highlight to Biology, updating 3 existing files."

- Each note is found again by the id in its front matter, anywhere in the vault except the `.obsidian` and `.trash` folders, even if you renamed or moved it. Its file keeps your new name and place.
- An exported file is replaced as a whole. Anything you typed into it in Obsidian since the last import is lost. Import first, then export; see [Bring your Obsidian edits back](#bring-your-obsidian-edits-back).
- A note or highlight you deleted in textweaver is not deleted from the vault. Delete its file in Obsidian if you no longer want it.
- If a document moves to another folder, its next export makes a new document note, because the document note's id comes from the document's path.

## Import a vault

```powershell
tw vault import "D:\Notes\Biology"
```

This reads every note in the vault and stores what it finds in textweaver. When it finishes, you hear one sentence, for example:

"Imported 3 notes from Biology: 3 documents added to the library, 4 links; 2 linked notes could not be found."

Other parts you may hear in that sentence:

- "2 notes on documents updated": notes that textweaver exported were brought back onto their documents.
- "1 highlight restored": a highlight from a document note was added back to its document.
- "1 note could not be read", followed by a line for each such file, "Could not read" with the file and the reason.

What import reads:

- Every file ending in `.md` in the vault and all its folders, in any letter case.
- It skips the `.obsidian` and `.trash` folders. Other folders whose names start with a dot, such as `.git`, are read like any other.
- Pictures, PDFs, canvases, and other attachments are not read.
- Links to folders (symbolic links) are not followed.

Each note is one of three kinds, and each kind is handled differently.

- A note textweaver exported for a note on a document (it has `textweaver_id` and `source` in its front matter) updates that note on that document. See [Bring your Obsidian edits back](#bring-your-obsidian-edits-back).
- A document note textweaver exported (it has `textweaver_doc` and `source`) brings back its highlights.
- Any other note is one of your own. It gets one textweaver note of its own, described next.

### Your own notes

Each of your own vault notes gets one textweaver note, attached to the very start of that note's file. You can see it by opening the vault note in textweaver and listing its notes, or with `tw marks`:

```powershell
tw marks "D:\Notes\Biology\Cell.md"
```

It holds:

- The note's title: the `title` in its front matter, or else the file name without `.md`.
- As its text, the note's first line of content, with heading marks and link brackets removed, up to 200 characters. An empty note uses its title instead.
- Its tags: the front matter `tags` (or `tag`), then every `#tag` in the text, without the `#`, plus the tag `obsidian-note` that marks it as coming from a vault.
- Its links to other notes in the vault, each with its kind, as described below. The notes list and `tw marks` do not show links yet.

Importing the same vault again refreshes these notes. It never adds a second one. Links are rebuilt from the files every time, so a link you removed in Obsidian disappears from textweaver on the next import.

### How links are read

- `[[Note]]` links to the note whose file name, front matter `title`, or one of whose `aliases` is "Note". Upper and lower case do not matter. If two notes share a name, the first one found, in order of their paths, wins.
- `[[Note#Heading]]` and `[[Note#^block]]` link to the whole note. The heading or block part is not kept.
- `[[Note|shown text]]` links to "Note".
- A Dataview field such as `supports:: [[Note]]` gives a link of that kind. The field name can be written in any case, with spaces or hyphens, such as `is example of::` or `See-Also::`. If the field starts its line, the rest of the line after the link becomes a comment on the link.
- A field name that is not one of the ten kinds, such as `author:: [[Note]]`, counts as a plain link.
- A plain `[[link]]` becomes `SEE_ALSO`, or the kind you name with `--link-relation`.
- The same kind of link to the same note, written twice, counts once.
- Embeds, `![[Note]]` and `![[picture.png]]`, do not become links.
- Links inside code, in backticks or code blocks, and inside `%% comments %%` are ignored, as Obsidian ignores them.
- Only notes inside the vault can be linked to. A link to a note that is not there is counted in "could not be found". So is a link to a document note that textweaver wrote.

### Choose the kind of plain links

```powershell
tw vault import "D:\Notes\Biology" --link-relation supports
```

Every plain `[[link]]` in the vault now becomes a `SUPPORTS` link instead of `SEE_ALSO`. Typed Dataview fields keep their own kind. The name can be in any case, with spaces or hyphens instead of underscores. A name textweaver does not know stops the import before anything is read, with a message listing the ten kinds. For `--link-relation related` it says:

```text
Error: Unknown relation "related"; choose one of CONFLICTS_WITH, SUPPORTS, IS_EXAMPLE_OF, CITES, CONTRADICTS, DEFINES, EXTENDS, SEE_ALSO, PRECEDES, FOLLOWS
```

There is no setting for this yet. Give the option each time.

### Check first with a dry run

```powershell
tw vault import "D:\Notes\Biology" --dry-run
```

A dry run reads the whole vault and says what it found, but stores nothing. The sentence starts with "Dry run, nothing stored." and goes on like a real import, for example:

"Dry run, nothing stored. Imported 3 notes from Biology: 3 documents added to the library, 4 links; 2 linked notes could not be found."

The counts compare the vault with an empty textweaver, not with what you already have. So a dry run may count notes and highlights you already have. For example, on the same vault, a dry run said "2 notes on documents updated, 1 highlight restored" where the real import said "1 note on a document updated", because only one note had changed and the highlight was already there.

To see every note it found, add `--json`:

```powershell
tw vault import "D:\Notes\Biology" --dry-run --json
```

For each note, the JSON gives its path, title, aliases, tags, id, summary (its first line), text, citation, time, kind (`plain`, `annotation`, or `document`), and the links found in it, each with its kind and target. At the end come the counts of links found, links not found, and files that could not be read.

### Library mode

```powershell
tw vault import "D:\Notes\Biology" --mode library
```

Library mode adds every vault note to textweaver's library as a document, and does nothing else: no notes, links, or highlights are stored. The notes then appear on your bookshelf (`tw library`, Alt+L in the reader). Graph mode, the default, adds the vault's own notes to the library in the same way, as well as storing their notes and links. Documents already in the library keep their place; a damaged library file is never overwritten, and the import says so.

To keep a vault's notes in your library as it changes, with new notes found and searchable, add the vault as a library folder instead:

```powershell
tw library add "D:\Notes\Biology"
```

That adds every Markdown note in the vault, skips the `.obsidian` folder, and makes the notes searchable with `tw library search`. See [the library guide](library.md).

## Bring your Obsidian edits back

```powershell
tw vault import "D:\Notes\Biology"
```

This is the round trip: export from textweaver, edit in Obsidian, then import. After you change an exported note in Obsidian, importing the vault updates the same note on its document. For example, the lecture's notes were exported into the Biology vault, which also holds three notes of its own. Then one exported note's text was changed in Obsidian. The import said:

"Imported 6 notes from Biology: 3 documents added to the library, 1 note on a document updated, 5 links; 2 linked notes could not be found."

Importing again, with nothing changed, leaves out the updated note: "Imported 6 notes from Biology: 3 documents added to the library, 5 links; 2 linked notes could not be found."

What comes back from an exported note:

- Its text: everything after the front matter, except the `## Links` section and `%% comments %%`. Links in the text are reduced to their words, so `[[Note|the cell]]` comes back as "the cell".
- Its `title`, which becomes the text the note is about.
- Its tags: the front matter `tags` and any `#tags` you typed in the text.
- Its `cite`.
- Its links: the lines under `## Links`, plus any `[[links]]` you added to the text. Edit, add, or remove lines under `## Links` in the same form, such as `- CONTRADICTS:: [[Other note]] - because of the lab results`.

What does not come back:

- `document` and `pct` are ignored. `position` is used only if the note no longer exists on its document; then the note is added again at that position.
- The file name does not matter. Rename notes as you like.
- Deleting a note's file in Obsidian does not delete the note in textweaver. Import never deletes anything.

From a document note, only the `%% textweaver-highlight ... %%` comments are read. A highlight that is missing from its document is added back. If you change the `start`, `end`, or `color` in a comment, the highlight changes too; the sentence does not mention this, but the JSON output counts it as `highlights_updated`. Editing the quoted text changes nothing, and removing a highlight from the document note does not remove it from the document.

Keep these in order, and nothing is lost:

1. Import the vault, so your Obsidian edits reach textweaver.
2. Read and add notes in textweaver.
3. Export, which rewrites the exported files from what textweaver now has.

## Read vault notes in textweaver

```powershell
tw open "D:\Notes\Biology\Cell.md"
```

A vault note is a Markdown file, so textweaver opens and reads it like any other. The front matter is not read aloud as text; its `title` becomes the document's title.

A wikilink is a link in the reader, as in Obsidian. `[[Mitochondria]]` is read as "Mitochondria", and `[[Other|the cell]]` as "the cell", without the brackets. The next link and previous link commands stop on wikilinks as on ordinary Markdown links. **Alt+Shift+F** follows the link at the cursor: textweaver opens the note it names, looking for `Mitochondria.md` beside the note, then for a file of that name in the folders below it. `[[Note#Heading]]` opens the note at that heading. **Alt+Left** comes back.

The links an import stores with your textweaver notes are another matter. An export writes them out again, but textweaver does not show them yet: the notes list and `tw marks` show each note's text and tags, not its links. To see which links an import finds, use a dry run with `--json`.

## Turn vault notes into web pages

```powershell
tw convert "D:\Notes\Biology" --to html --flavor obsidian --out "D:\Notes\Biology-web"
```

This converts every note in the vault to an HTML page in `Biology-web`, keeping the vault's folders. The `.obsidian` folder is skipped. With `--flavor obsidian`, `[[Note]]` becomes a link to `Note.html`, `[[Note#Heading]]` to that heading, and `[[Note|text]]` shows your own link text. Callouts, `#tags`, `==highlights==`, and block ids work too.

An embedded note, `![[Note]]`, becomes a link by default. To put the note's text in its place instead, add `--embeds inline`:

```powershell
tw convert "D:\Notes\Biology" --to html --flavor obsidian --embeds inline --out "D:\Notes\Biology-web"
```

Two limits to know about. A link points to a page of the same name in the same folder, so a link to a note in another folder, or a link by title or alias, leads to a page that does not exist. And the web pages are made from the vault's files only; your textweaver notes and highlights are not part of them.

See [the converting guide](converting.md#markdown-flavors) for every option.

## If something goes wrong

- **"Name the documents to export with --document FILE".** Export needs at least one `--document`. Add the document whose notes you want.
- **"... has no notes or highlights."** The document has nothing to export, or its path is wrong. Check it with `tw marks` and the same path.
- **"... could not be opened, so highlights are exported without their text."** The document has moved, been deleted, or is in a format textweaver cannot read. The export still works, but quotes and percentages are missing. Put the document back where it was, or open it once in textweaver to check it reads.
- **"... links left out because the linked note is not in the vault."** A note links to a note on a document you did not export this time or before. Export that document too, in the same command or into the same vault, to see the link in Obsidian. The link is not lost in the meantime: importing the vault keeps a note's links to notes that are not in the vault, and only a link you delete in Obsidian, to a note that is in the vault, is removed.
- **"... linked notes could not be found."** A `[[link]]` names a note that is not in the vault, or is misspelled, or points to a document note. Run a dry run with `--json` and look at each note's links to find which.
- **"... is not a folder".** The vault path is wrong or the folder does not exist. Import needs an existing folder. Export creates one.
- **"Could not read ...".** A note file could not be opened, for example because another program has it locked. The reason follows the file name. The rest of the vault is still imported.
- **"Unknown relation ...".** The name given to `--link-relation` is not one of the ten kinds. Use a name from the list in the message.
- **A note appears as `Name 2`.** A note with that name was already in the folder, often a note of your own. textweaver never overwrites it, so it picks a new name. Rename either note in Obsidian if you like; the next export finds textweaver's note by its id.
- **Changes made in Obsidian disappeared.** The notes were exported again before the vault was imported. Export replaces the files it wrote. Always import first.
- **Imported notes do not appear in the library.** Check that the import was not a dry run, and that the sentence says "documents added to the library". A vault note already in the library keeps its place and is not counted again. To have new notes found as the vault grows, add the vault as a library folder with `tw library add`.
- **A dry run reports more than the real import did.** A dry run compares against an empty textweaver, so it counts notes and highlights you already have as new.
- **Links stored by an import point to the wrong place.** The vault was given as a relative path, such as `Biology`. Import again with the full path.
- **An option seems to do nothing.** Options for export are ignored by import, and the other way round. Check the list in [Options for tw vault](#options-for-tw-vault).

## See also

- [Notes, bookmarks, and highlights](notes.md): how to make the notes and highlights that export writes.
- [The library](library.md): adding a vault folder to the library and searching it.
- [Converting documents](converting.md): `tw convert` with the Obsidian flavor and every other option.
- [Documentation index](README.md)
