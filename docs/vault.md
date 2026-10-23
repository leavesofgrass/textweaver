# Obsidian vaults

`tw vault` exchanges study notes between textweaver and an Obsidian vault. `tw vault export` writes the notes and highlights made while reading into the vault as ordinary Markdown notes, linked to one another so that they appear in Obsidian's graph. `tw vault import` reads a vault back: it applies edits made to those notes in Obsidian to their documents, and it converts the vault's own notes and their `[[links]]` into textweaver notes that can be listed and navigated. It suits readers who read in textweaver and organize or review in Obsidian, and those who keep a vault and want its links available inside textweaver.

## Before you start

- A vault is a folder of Markdown files; Obsidian keeps its own settings in a `.obsidian` folder inside it. `tw vault` operates on the folder, so Obsidian need not be running or even installed.
- Export and import belong to `tw`, the command-line tool. The reader, in the terminal and in the textweaver app, follows a vault's links and lists its properties, backlinks, unresolved links, and tags; see [Read vault notes in textweaver](#read-vault-notes-in-textweaver).
- Notes and highlights are created while reading; [the notes guide](notes.md) describes how. `tw marks`, given a document's file name, lists the notes and highlights that document holds.
- textweaver stores each document's notes and highlights in its own data folder, never inside the document. On Windows that is `%APPDATA%\leavesofgrass\textweaver\data\state`. On macOS it is `~/Library/Application Support/org.leavesofgrass.textweaver/state`, and on Linux `~/.local/share/textweaver/state`. If the `TEXTWEAVER_HOME` environment variable is set, it is `data\state` inside that folder.
- Import only reads the vault; it never changes, moves, or deletes a file in it.
- Export never overwrites a note written by hand. It replaces only files that textweaver wrote earlier.
- The vault should be given by its full path, such as `D:\Notes\Biology`, rather than a relative one such as `Biology`. Links found during an import are stored with the path exactly as typed, so a relative path works only when `tw` is later run from the same folder.
- The examples use a vault at `D:\Notes\Biology` and a document at `D:\Courses\lecture.md`; substitute your own paths. Paths containing spaces need quotation marks.

## Options for tw vault

```powershell
tw vault --help
```

The command takes the form `tw vault`, then an action, then the vault folder (or a note in it), then any options. The actions are `import` and `export`, which exchange notes with textweaver, and four that only read the vault: `backlinks`, `unresolved`, `tags`, and `properties` (see [Ask the vault](#ask-the-vault)). Some options belong to one action only, and an option given to another action is ignored without a warning.

- `--document FILE` (export only): a document whose notes and highlights to export. Export needs at least one. Repeat the option for several documents.
- `--folder FOLDER` (export only): a folder inside the vault for new notes, such as `textweaver`. Without it, new notes go in the vault's top folder.
- `--no-document-notes` (export only): omit the document note that each document otherwise receives. The document's highlights are then not exported either.
- `--mode graph` or `--mode library` (import only): what to import. `graph`, the default, imports notes and their links. `library` adds the vault's notes to the library as documents and does nothing else; see [Library mode](#library-mode).
- `--link-relation NAME` (import only): the kind of link a plain `[[link]]` becomes, such as `SUPPORTS`. Without it, plain links become `SEE_ALSO`.
- `--dry-run` (import only): read the vault and report what an import would do, storing nothing.
- `--tag NAME` (tags only): list the notes that carry the tag `NAME`, written with or without `#`, instead of every tag.
- `--json` (every action): print the full result as JSON, for scripts.

## Export notes and highlights to a vault

```powershell
tw vault export "D:\Notes\Biology" --document "D:\Courses\lecture.md"
```

This writes one Markdown note for each note on `lecture.md`, plus a document note for `lecture.md` itself that holds its highlights and links to its notes. The vault folder is created if it does not yet exist.

To export several documents at once, repeat `--document`:

```powershell
tw vault export "D:\Notes\Biology" --document "D:\Courses\lecture.md" --document "D:\Courses\lab.md"
```

On completion, textweaver announces a one-sentence summary, for example:

"Exported 2 notes and 1 highlight to Biology."

The name at the end is that of the vault folder. The summary may also include these reports:

- "lab has no notes or highlights." The document has nothing to export and is skipped. The same message appears if the file name is misspelled, so check the name when notes were expected.
- "Nothing to export." None of the documents had notes or highlights. Nothing is written.
- "lecture could not be opened, so highlights are exported without their text." The notes are still exported, but each highlight's quote states only the characters it covers, such as "Characters 30 to 55", and percentages are omitted.
- "...; 2 links left out because the linked note is not in the vault." A note links to a note on a document you did not export. See [If something goes wrong](#if-something-goes-wrong).

Without `--document`, export stops with "Name the documents to export with --document FILE"; there is no option yet to export every document at once.

### What a note file holds

The following is an actual exported note, with the long folder path in `source` shortened to `D:\Courses`.

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

The block between the two lines of three dashes is the front matter, which Obsidian displays as the note's properties. The keys appear in this order:

- `textweaver_id`: the note's id, by which textweaver finds the file again even after it is renamed or moved to another folder in the vault. It should not be changed.
- `title`: the text the note is attached to, either the selected words or the sentence at the cursor. If there is no such text, the note's first line is used.
- `source`: the full path of the document the note belongs to.
- `document`: a link to the document's document note. It is left out with `--no-document-notes`.
- `position`: where the note is in the document, counted in characters from the start.
- `pct`: the same place as a percentage, omitted when the document could not be opened.
- `tags`: the note's tags; the key is always written, as `[]` when there are none.
- `cite`: the note's citation, written only when it has one.
- `updated`: when the note last changed, in UTC.

Values that could confuse Obsidian, such as a title containing a colon, are put in quotation marks.

The note's own text follows the front matter. If the note links to other notes, a `## Links` section lists them, one per line, as a Dataview inline field: a dash, the kind of link in capitals, two colons, and a link to the other note, followed by a comment after " - " when the link has one. Obsidian's graph draws these links as lines between notes, and the Dataview plugin can query them.

The kinds of link are `CONFLICTS_WITH`, `SUPPORTS`, `IS_EXAMPLE_OF`, `CITES`, `CONTRADICTS`, `DEFINES`, `EXTENDS`, `SEE_ALSO`, `PRECEDES`, and `FOLLOWS`. A kind unknown to this version, written by a newer one, is exported in the same capitals and returns unchanged on import. Any other field name that import does not recognize, such as `related::`, produces a plain link, as in star.

### What the document note holds

Each exported document also receives a document note, named after the document's title. This is the actual note for the lecture, with the path shortened in the same way:

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
- Under `## Highlights`, each highlight has three parts. The first is a comment between `%%` marks holding the highlight's exact place and color, which Obsidian hides when it displays the note and textweaver reads back on import. The second is the highlighted text as a quote, ending with a block id such as `^hl-h1`. The third is a line giving its color and location.
- Quotes are the highlighted text with line breaks joined into spaces; a quote longer than 600 characters is truncated and ends with an ellipsis.
- The block id allows any note in Obsidian to link to a single highlight, for example `[[Lecture 3 Energy#^hl-h1]]`.
- Under `## Notes`, each of the document's notes is a link, with its place as a percentage and its first line.

### How files are named

- A note file is named after the note's `title`, the text the note is about. If that is empty, the note's first line is used, and failing that, its id.
- A document note is named after the document's title. If the document has no title, its file name is used, without the extension.
- These characters cannot appear in a name and are replaced by spaces: `<` `>` `:` `"` `/` `\` `|` `?` `*`, the square brackets `[` and `]`, `#`, `^`, and control characters such as tabs. Obsidian's links would break on the brackets, `#`, `^`, and `|`.
- Runs of spaces collapse to one space. Trailing dots and spaces are removed, as is a trailing `.md`.
- A name is at most 120 characters long.
- A name that would be empty becomes `note`. A name that Windows reserves for devices, such as `CON`, `NUL`, `COM1`, or `LPT1`, has " note" appended, as in `CON note`.
- If a name is already used in the folder, by a note written by hand or by another note in the same export, a number is appended: `The Cell 2`, `The Cell 3`, and so on. Upper and lower case count as the same name, as they do in Obsidian.

In the example, the note about "Ribosomes: protein builders" became `Ribosomes protein builders.md`, and "Lecture 3: Energy" became `Lecture 3 Energy.md`.

### Choose a folder for new notes

```powershell
tw vault export "D:\Notes\Biology" --folder textweaver --document "D:\Courses\lecture.md"
```

`--folder` places new notes in a folder inside the vault, here `D:\Notes\Biology\textweaver`, creating it if necessary. Notes exported earlier stay where they are, even if they lie elsewhere in the vault.

### Leave out the document notes

```powershell
tw vault export "D:\Notes\Biology" --no-document-notes --document "D:\Courses\lecture.md"
```

This writes only the note files. Highlights are held in the document notes, so with this option they are not exported, and the notes carry no `document` key.

### Export again

```powershell
tw vault export "D:\Notes\Biology" --document "D:\Courses\lecture.md"
```

Running the same export again updates the notes written earlier instead of adding copies. The summary then reads, for example:

"Exported 2 notes and 1 highlight to Biology, updating 3 existing files."

- Each note is found again by the id in its front matter, anywhere in the vault except the `.obsidian` and `.trash` folders, even if it was renamed or moved; its file keeps its new name and place.
- An exported file is replaced as a whole, so anything typed into it in Obsidian since the last import is lost. Import first and then export; see [Bring your Obsidian edits back](#bring-your-obsidian-edits-back).
- A note or highlight deleted in textweaver is not deleted from the vault; its file must be deleted in Obsidian.
- If a document moves to another folder, its next export creates a new document note, because the document note's id derives from the document's path.

## Import a vault

```powershell
tw vault import "D:\Notes\Biology"
```

This reads every note in the vault and stores the results in textweaver. On completion, textweaver announces a one-sentence summary, for example:

"Imported 3 notes from Biology: 3 documents added to the library, 4 links; 2 linked notes could not be found."

The summary may also contain these parts:

- "2 notes on documents updated": notes that textweaver exported were applied back to their documents.
- "1 highlight restored": a highlight from a document note was added back to its document.
- "1 note could not be read", followed by one line per file, "Could not read" with the file and the reason.

What import reads:

- Every file ending in `.md` in the vault and all its folders, in any letter case.
- The `.obsidian` and `.trash` folders are skipped; other folders whose names begin with a dot, such as `.git`, are read like any other.
- Pictures, PDFs, canvases, and other attachments are not read.
- Links to folders (symbolic links) are not followed.

Each note is one of three kinds, handled differently.

- A note textweaver exported for a note on a document (it has `textweaver_id` and `source` in its front matter) updates that note on that document. See [Bring your Obsidian edits back](#bring-your-obsidian-edits-back).
- A document note textweaver exported (it has `textweaver_doc` and `source`) brings back its highlights.
- Any other note is one of the vault's own and receives a textweaver note of its own, described next.

### Your own notes

Each of the vault's own notes receives one textweaver note, attached to the very start of that note's file. It can be seen by opening the vault note in textweaver and listing its notes, or with `tw marks`:

```powershell
tw marks "D:\Notes\Biology\Cell.md"
```

It holds:

- The note's title: the `title` in its front matter, or else the file name without `.md`.
- As its text, the note's first line of content with heading marks and link brackets removed, up to 200 characters; an empty note uses its title instead.
- Its tags: the front matter `tags` (or `tag`), then every `#tag` in the text, without the `#`, plus the tag `obsidian-note`, which marks the note as coming from a vault.
- Its links to other notes in the vault, each with its kind, as described below. The notes list and `tw marks` do not show links yet.

Importing the same vault again refreshes these notes and never adds a second one. Links are rebuilt from the files each time, so a link removed in Obsidian disappears from textweaver on the next import.

### How links are read

- `[[Note]]` links to the note whose file name, front matter `title`, or one of whose `aliases` is "Note". Case is ignored. If two notes share a name, the first one found, in path order, is used. Aliases may be a list (`aliases: [Cell biology, Cytology]`, or one alias per line under `aliases:`) or text with commas between them; an alias may contain spaces.
- `[[Note#Heading]]` and `[[Note#^block]]` link to the whole note; the heading or block part is not kept.
- `[[Note|shown text]]` links to "Note".
- A Dataview field such as `supports:: [[Note]]` gives a link of that kind. The field name may be written in any case, with spaces or hyphens, such as `is example of::` or `See-Also::`. If the field begins its line, the rest of the line after the link becomes a comment on the link.
- A field name that is not one of the ten kinds, such as `author:: [[Note]]`, counts as a plain link.
- A plain `[[link]]` becomes `SEE_ALSO`, or the kind you name with `--link-relation`.
- The same kind of link to the same note, written twice, is counted once.
- Embeds, `![[Note]]` and `![[picture.png]]`, do not become links.
- Links inside code, in backticks or code blocks, and inside `%% comments %%` are ignored, as Obsidian ignores them.
- Only notes inside the vault can be linked to. A link to a note that is absent is counted in "could not be found", as is a link to a document note that textweaver wrote.

### Choose the kind of plain links

```powershell
tw vault import "D:\Notes\Biology" --link-relation supports
```

Every plain `[[link]]` in the vault now becomes a `SUPPORTS` link instead of `SEE_ALSO`, while typed Dataview fields keep their own kind. The name may be in any case, with spaces or hyphens in place of underscores. An unknown name stops the import before anything is read, with a message listing the ten kinds. For `--link-relation related` the message is:

```text
Error: Unknown relation "related"; choose one of CONFLICTS_WITH, SUPPORTS, IS_EXAMPLE_OF, CITES, CONTRADICTS, DEFINES, EXTENDS, SEE_ALSO, PRECEDES, FOLLOWS
```

There is no setting for this yet, so the option must be given each time.

### Check first with a dry run

```powershell
tw vault import "D:\Notes\Biology" --dry-run
```

A dry run reads the whole vault and reports what it found without storing anything. The sentence begins with "Dry run, nothing stored." and continues as for a real import, for example:

"Dry run, nothing stored. Imported 3 notes from Biology: 3 documents added to the library, 4 links; 2 linked notes could not be found."

The counts compare the vault with an empty textweaver, not with the existing data, so a dry run may count notes and highlights that already exist. On the same vault, for example, a dry run reported "2 notes on documents updated, 1 highlight restored" where the real import reported "1 note on a document updated", because only one note had changed and the highlight was already present.

`--json` lists every note found:

```powershell
tw vault import "D:\Notes\Biology" --dry-run --json
```

For each note, the JSON gives its path, title, aliases, tags, id, summary (its first line), text, citation, time, kind (`plain`, `annotation`, or `document`), and the links found in it, each with its kind and target. At the end come the counts of links found, links not found, and files that could not be read.

### Library mode

```powershell
tw vault import "D:\Notes\Biology" --mode library
```

Library mode adds every vault note to textweaver's library as a document and does nothing else: no notes, links, or highlights are stored. The notes then appear on the bookshelf (`tw library`, Alt+L in the reader). Graph mode, the default, adds the vault's own notes to the library in the same way and also stores their notes and links. Documents already in the library keep their place, and a damaged library file is never overwritten; the import reports it.

To keep a vault's notes in the library as the vault changes, with new notes found and searchable, add the vault as a library folder instead:

```powershell
tw library add "D:\Notes\Biology"
```

This adds every Markdown note in the vault, skips the `.obsidian` folder, and makes the notes searchable with `tw library search`. See [the library guide](library.md).

## Bring your Obsidian edits back

```powershell
tw vault import "D:\Notes\Biology"
```

The round trip is: export from textweaver, edit in Obsidian, then import. After an exported note is changed in Obsidian, importing the vault updates the same note on its document. In one test, the lecture's notes were exported into the Biology vault, which also holds three notes of its own, and one exported note's text was then changed in Obsidian. The import reported:

"Imported 6 notes from Biology: 3 documents added to the library, 1 note on a document updated, 5 links; 2 linked notes could not be found."

Importing again, with nothing changed, omits the updated note: "Imported 6 notes from Biology: 3 documents added to the library, 5 links; 2 linked notes could not be found."

What comes back from an exported note:

- Its text: everything after the front matter, except the `## Links` section and `%% comments %%`. Links in the text are reduced to their words, so `[[Note|the cell]]` comes back as "the cell".
- Its `title`, which becomes the text the note is about.
- Its tags: the front matter `tags` and any `#tags` you typed in the text.
- Its `cite`.
- Its links: the lines under `## Links`, plus any `[[links]]` you added to the text. Edit, add, or remove lines under `## Links` in the same form, such as `- CONTRADICTS:: [[Other note]] - because of the lab results`.

What does not return:

- `document` and `pct` are ignored. `position` is used only if the note no longer exists on its document; then the note is added again at that position.
- The file name is irrelevant, so notes may be renamed freely.
- Deleting a note's file in Obsidian does not delete the note in textweaver; import never deletes anything.

From a document note, only the `%% textweaver-highlight ... %%` comments are read. A highlight missing from its document is added back. Changing the `start`, `end`, or `color` in a comment changes the highlight too; the summary sentence does not mention this, but the JSON output counts it as `highlights_updated`. Editing the quoted text changes nothing, and removing a highlight from the document note does not remove it from the document.

Nothing is lost if the steps are taken in this order:

1. Import the vault, so your Obsidian edits reach textweaver.
2. Read and add notes in textweaver.
3. Export, which rewrites the exported files from what textweaver now has.

## Read vault notes in textweaver

```powershell
tw open "D:\Notes\Biology\Cell.md"
```

A vault note is a Markdown file, so textweaver opens and reads it like any other. The front matter, which Obsidian calls the note's properties, is not read aloud as text, and its `title` becomes the document's title.

### The vault a note belongs to

textweaver treats as the vault the nearest folder at or above the note that holds a `.obsidian` folder, the folder in which Obsidian keeps its settings. Links and embeds are resolved anywhere inside that folder and never outside it. A note with no `.obsidian` folder above it is treated as before: links and embeds reach its own folder and the folders below it. Folders whose names begin with a dot, such as `.obsidian`, `.trash`, and `.git`, are never searched.

### Follow a link: Alt+Shift+F

A wikilink is a link in the reader, as in Obsidian. `[[Mitochondria]]` is read as "Mitochondria" and `[[Other|the cell]]` as "the cell", without the brackets, and the next link and previous link commands stop on wikilinks as on ordinary Markdown links. **Alt+Shift+F** follows the link at the cursor, and **Alt+Left** returns. textweaver finds the note in this order:

1. A path written in the link, from the current note's folder (`[[../Projects/Plan]]`) and then from the top of the vault (`[[Projects/Plan]]`). A path that would lead outside the vault is not followed.
2. A note of that name anywhere in the vault, with or without `.md`; `[[Projects/Plan]]` also matches any note whose path ends that way.
3. A note whose front matter `title` is the name, then one with the name among its `aliases`.

Case is ignored throughout. When several notes share a name, the nearest one opens, the one with the fewest folders between it and the current note, and textweaver says once, for example, "2 notes named Dup; the nearest opened." To reach a different one, write its folder in the link.

A link can also name part of a note:

- `[[Note#Heading]]` opens the note at that heading, matched by its text or its slug. `[[Note#Part#Detail]]`, a heading inside a heading, opens at "Detail".
- `[[Note#^key-point]]` opens the note at the block with that id. `[[#^key-point]]` goes to the block in the current note. A missing block is reported as "No block key-point in this document."

A link to a note that does not exist says "Link to a note that doesn't exist:" and its name.

### Properties: f

When a note with front matter opens, the opening message ends with the number of properties, as in "Opened The Cell. Properties: 4." Press **f** to list them, one row per property, name first: "tags: biology, exam". A list property is read as its items separated by commas, and a checkbox property as "checked" or "not checked". Properties nested inside another property are not listed. Lists may be written either way Obsidian writes them, in brackets (`tags: [biology, exam]`) or one item per line under the name; `tag`, `alias`, and `cssclass`, Obsidian's older names, are read as `tags`, `aliases`, and `cssclasses`. Dataview fields written in the text (`status:: draft`) are not properties here; `tw vault properties` lists them (see [Ask the vault](#ask-the-vault)).

### Notes that link here, links to notes that don't exist, and tags

These commands are in the command palette and in the Reading menu under Go to. They read the whole vault the first time one is used, and again only the notes that have changed since, so a note added or edited in Obsidian is included the next time. Each shows a list, which works like every other list in textweaver: the arrow keys move, Enter acts on the row, and Escape closes the list.

- **Notes that link here** lists every note that links to or embeds the open note, each row naming the note first, then how many links, then its folder: "Mitochondria, 1 link, in Projects". Links written as Markdown links (`[the cell](../Notes/Cell.md)`) count as well. Enter opens the note.
- **Links to notes that don't exist** lists this note's links that lead nowhere, each name once.
- **Tags in this note** lists the tags from the front matter and from the text, without `#`. Enter on a tag lists the notes in the vault that carry it.
- **Tags in the vault** lists every tag with the number of notes carrying it: "exam, 2 notes". Tags that differ only in case count as one tag. Enter on a tag lists its notes; a tag such as `biology` also lists the notes tagged `biology/organelles`. Enter on a note opens it.

Links inside code and inside `%% comments %%` are not counted, as in Obsidian. For a very large vault textweaver reads at most 20,000 notes, and a note larger than 4 megabytes only by its name; when a limit is reached, the list says "Only part of the vault was searched; it is very large."

The links that an import stores with textweaver notes are a separate matter. An export writes them out again, but textweaver does not yet display them: the notes list and `tw marks` show each note's text and tags, not its links. To see which links an import finds, use a dry run with `--json`.

## Ask the vault

Four actions of `tw vault` answer questions about a vault without storing or changing anything. Each accepts `--json`.

```powershell
tw vault backlinks "D:\Notes\Biology\Cell.md"
```

This lists the notes that link to `Cell.md`, as the reader's Notes that link here does, for example:

```text
1 note links to The Cell:
Mitochondria, in Projects, 2 links
```

```powershell
tw vault unresolved "D:\Notes\Biology"
```

This lists the links that lead to notes that don't exist, with the note each appears in, for the whole vault, or for one note when a note is given: "Ghost, from The Cell, in Notes".

```powershell
tw vault tags "D:\Notes\Biology"
tw vault tags "D:\Notes\Biology" --tag biology
```

The first lists every tag in the vault with the number of notes carrying it, one per line ("biology, 1 note"); the second lists the notes carrying `biology` or a tag nested under it.

```powershell
tw vault properties "D:\Notes\Biology\Cell.md"
```

This prints the note's properties, after the line "Properties: 3", and then any Dataview fields written in its text (`status:: draft`, or `- supports:: [[Mitochondria]]` in a list), after the line "Dataview fields: 2". Only the vault commands read Dataview fields; the reader does not.

A note's vault is found as in the reader: the nearest folder above it holding `.obsidian`, or the folder given.

## Turn vault notes into web pages

```powershell
tw convert "D:\Notes\Biology" --to html --flavor obsidian --out "D:\Notes\Biology-web"
```

This converts every note in the vault to an HTML page in `Biology-web`, preserving the vault's folder structure and skipping the `.obsidian` folder. With `--flavor obsidian`, `[[Note]]` becomes a link to the page of the note it names, found as the reader finds it: `[[Plan]]` in `Notes\Main.md` links to `../Projects/Plan.html` when the note is `Projects\Plan.md`. `[[Note#Heading]]` links to that heading, and `[[Note|text]]` shows the given link text. Callouts, `#tags`, `==highlights==`, and block ids are supported as well.

An embedded note, `![[Note]]`, becomes a link by default; `--embeds inline` places the note's text there instead:

```powershell
tw convert "D:\Notes\Biology" --to html --flavor obsidian --embeds inline --out "D:\Notes\Biology-web"
```

Three limits apply. A link by title or alias, or to a note that does not exist, points to a page of that name in the same folder, which does not exist. A link to a note outside the converted folder points to where its page would be, so convert the whole vault for every link to work. And the web pages are generated from the vault's files only; textweaver notes and highlights are not part of them.

[The converting guide](converting.md#markdown-flavors) lists every option.

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
