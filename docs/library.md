# The library

This guide covers textweaver's library: the folders of documents you read from, the list of recent files, "Continue reading", the reading queue, the review list, full-text search across every document, and the older method of keeping reading places synchronized between computers through a library folder. It also covers where textweaver stores its files and how to import your data from star. It is intended for readers with a sizable collection of documents and for those migrating from star. Synchronizing notes, highlights, bookmarks, places, and library details between computers has [a guide of its own](sync.md).

## Open the library in the reader: Alt+L

Press **Alt+L** in the terminal reader (**Ctrl+Shift+B** in the app). You hear "Library", the number of documents, then "Type to filter, Enter opens one, F2 edits details, Ctrl+Enter queues it."

The folders are read in the background, so even a large library (up to 20,000 files per folder) never blocks the keyboard. While the scan runs you may hear "Scanning the library.", and the status line updates the count of documents found every second; the list opens when the scan finishes. Pressing **Alt+L** again in the meantime reports how many have been found so far.

The list contains every document in your library folders, followed by recently opened files that lie outside those folders, newest first. Each item states:

- the title;
- "by", then the author, when textweaver knows it;
- how far you have read, as a percentage, when you have a saved place;
- "in", then the folder's name, for a document in a library folder, or "recent" for a recent file.

For example: "Cells, 25 percent, in Readings".

**Up** and **Down** move through the list, **Enter** opens a document, **F2** edits its details (below), **Ctrl+Enter** adds it to the reading queue or removes it (below; **Cmd+Enter** in the Mac app), and **Escape** closes the list. When there is nothing to show you hear: "The library is empty. Add a folder in Settings, under Library and study tools (Library folders), or open a file with Ctrl+O."

### Filter the list as you type

Typing in the list filters it. Each word you type must occur in a document's title, path, author, DOI, or ISBN, or in its text. You hear the number of matches, such as "2 documents match.", and the list narrows to those documents. **Backspace** removes a letter; when the filter is empty you hear "Filter cleared", then the number of documents. When nothing matches you hear "No documents match", followed by what you typed.

A DOI or ISBN matches however it is written: `10.1000/xyz`, `doi:10.1000/XYZ`, a `https://doi.org/` link, `978-0-306-40615-7`, or the same book's ten-digit ISBN.

A document's text takes part in the filter only after `tw library search` has read it (below); the list uses what that search stored in the cache and never reads documents itself.

## Search by author, DOI, and ISBN

textweaver learns a document's author, DOI, and ISBN from four sources:

- **When you open it.** The author comes from the document's own metadata: Markdown front matter (`author`, `doi`, `isbn`), the author of a Word or EPUB file, or a web page's `<meta>` tags (`citation_doi`, `dc.identifier`). A DOI or ISBN printed near the start of the text also counts, such as a paper's DOI on its first page or a book's ISBN on its copyright page; an ISBN counts only when it follows the word "ISBN". These details go on the bookshelf, described below.
- **From its text,** once `tw library search` has read it, for documents you have not yet opened.
- **From your reference library.** When `tw cite` holds a record of the same work (the same DOI or ISBN, or the same title when it is at least twelve letters long), its authors, DOI, and ISBN fill in whatever the document lacks. Your personal `references.json` and each library folder's `references.json` both count.
- **From your other computers,** with [sync](sync.md) on. When another computer has opened a document, its title, author, DOI, and ISBN travel with it, so the filter and `tw library search` can find the document here by a DOI that only that computer knew, even if you have never opened it here. textweaver matches the document here when you have opened it here before, when it lies in a library folder that is itself synced between the computers (its `.textweaver/library-id.json` travels with it), or when `tw library search` has read its text. When two computers hold different details, the newest wins, detail by detail.

Word and EPUB files store a DOI or ISBN in an identifier field that textweaver does not yet read; a DOI or ISBN printed in the text is found.

## Edit a document's details

When a document's details are wrong or missing, as with a scan named "scan0042" that has no author, you can supply its title, author, DOI, and ISBN yourself. What you enter takes precedence over what the document says about itself in the library list, its filter, and `tw library search`, and it persists when the document is opened again.

### In the reader

The form opens in either of two ways:

- **For the open document:** choose **Edit details** in the File menu, under Continue reading, or type "edit details" in the command palette (F2). It has no key of its own; you can assign one in `keymap.toml`.
- **From the library list:** move to a document and press **F2**. When you save or cancel, the list returns to the same document with its row updated.

You hear "Details of", the document's title, then "Tab moves, Enter saves, Escape cancels." The form has four single-line fields: Title, Author, DOI, and ISBN. Each field's label gives its position, such as "Author, 2 of 4", followed by its current value.

- **Tab** moves to the next field and **Shift+Tab** to the previous one, wrapping from the last field to the first. You hear the field's label and value, such as "Author: Ada Example", or "Author: blank" for an empty field. Text you have typed in a field is kept when you leave it.
- **Enter**, from any field, saves every field you changed. You hear "Details saved:" and the names of the fields, such as "Details saved: Title, Author." If nothing changed you hear "Details not changed."
- **Escape** closes the form without saving, and you hear "Cancelled. Details not changed."

A DOI or ISBN may be entered in any of its usual forms: `10.1000/xyz`, `doi:10.1000/XYZ`, a `https://doi.org/` link, or an ISBN with or without hyphens. A value that is not a valid DOI or ISBN is not saved; you hear, for example, "Not a DOI: 10.10/x. Fix it or clear it.", and the form reopens on that field.

**Clearing a field** removes your edit, so the document's own value shows again, immediately in the library list. A field you never edited cannot hide the document's own value.

In the app the form is a dialog showing one field at a time, labeled in the same way; Tab and Shift+Tab move between fields, Enter saves, and Escape cancels.

### From the command line: tw library edit

```bash
tw library edit "C:\Users\me\Readings\scan0042.pdf" --title "Cell Biology, Chapter 3" --author "Ada Example"
tw library edit scan0042.pdf --doi 10.1000/cells --json
tw library edit scan0042.pdf --author ""
```

Supply any of `--title`, `--author`, `--doi`, and `--isbn`; the others keep their values. An empty value, such as `--author ""`, clears your edit. The command prints "Details saved:" and the fields, or "Details not changed." An invalid DOI or ISBN is refused with the same message as in the reader. With `--json` it prints the document's `path`, whether the bookshelf `changed`, the `fields` given, whether the edit was `published` to the sync folder, and every detail you have `edited`.

### On your other computers

With [sync](sync.md) on, your edits travel to your other computers together with the document's other library details. Your edit takes precedence over the details the document states, even when another computer opens the document later. When the same detail is edited on two computers, the newest edit wins, and clearing a detail travels as well.

## Library folders

A library folder is an ordinary folder of documents. textweaver lists every document it can open in the folder and its subfolders, skipping hidden folders, its own `.textweaver` folders, star's `.star` folders, `.obsidian`, `.git`, `node_modules`, and the recycle bin. It reads at most 20,000 files per folder and never modifies them.

### Add a folder

In the reader, in either the app or the terminal, choose **Add a folder to the library** in the File menu, or type its name in the command palette. The file browser opens on your places; navigate to the folder and choose it with **Ctrl+Enter** or the "Choose this folder" row. You hear, for example, "Added Readings to the library. Open the library to see its documents." Choosing a folder that is already present reports that it is "already in the library".

On the command line:

```bash
tw library add C:\Users\me\Readings
```

You hear, for example, "Added folder Readings with 12 documents". Adding the same folder again reports that it is "already in the library", and a path that is not a folder is refused.

### Remove a folder: tw library remove

```bash
tw library remove C:\Users\me\Readings
```

It says "Removed", the folder, then "from the library; its files are unchanged".

### See the library: tw library

```bash
tw library
```

With no options, `tw library` lists the library folders and then every document, one item each, with its path on the following line.

### The settings

The folders are stored in the `[library]` section of `settings.toml`, where they can also be edited directly:

```toml
[library]
folders = ["C:\\Users\\me\\Readings", "D:\\Course"]
recent_limit = 20
```

- `folders` (default: none): the library folders. In TOML, each backslash in a path is written twice.
- `recent_limit` (default `20`): how many recent files to remember.

## Continue reading

**Continue reading** lists the documents on this computer that have a saved reading place, newest first, regardless of which computer read them last. It is in the File menu under Library and in the command palette (F2); it has no key of its own, but you can assign one in `keymap.toml`. You hear the number of documents listed, then each row, meaning first:

"Cells, 42 percent, laptop, 2 hours ago"

A row gives the title, how far into the document the place lies, the computer that saved the place (this computer, when it saved it), and how long ago it was saved. **Enter** opens the document, which resumes according to `[sync] position_policy` (see [Syncing between computers](sync.md#a-place-from-another-computer)).

Only documents present on this computer are listed; one you read solely on another computer and do not have here is omitted. The places saved by your other computers count only with sync on; without it, the list shows this computer's own places. When there is nothing to list you hear "Nothing to continue: no places saved."

From the command line:

```bash
tw library continue
tw library continue --json
```

`--continue` prints the same rows, each followed by its path on the next line. With `--json`, each document has its `path`, `title`, `pct`, the `device` name, `this_computer`, and `when_ms` (when the place was saved, in milliseconds since 1970, UTC).

## The reading queue

The reading queue is an ordered list of documents to read next, in the order you add them: this week's readings, for example, or the chapters assigned before an exam. It is kept on the bookshelf (`library.json`, below), so it is there the next time you start textweaver. With sync on, the queue also syncs with the places group, so the same queue is there on your other computers ([the reading queue in syncing](sync.md#the-reading-queue)).

**To add a document:**

- **From the library list:** move to the document and press **Ctrl+Enter**. You hear "Added to the reading queue", the title, and the number of documents in the queue. The list stays open so that you can add several. **Ctrl+Enter** on a queued document removes it: "Removed from the reading queue", then the title.
- **For the open document:** choose **Add to reading queue** in the File menu, or type "reading queue" in the command palette (F2). The same command removes it again.

Only a document saved as a file can join the queue; a web page or an unsaved document is refused, and you are told so.

**To see the queue,** choose **Reading queue** in the File menu or in the command palette. You hear "Reading queue", the number of documents, then "Enter opens one, Delete takes it out." The rows are the titles, in order. A document whose file cannot be found, such as one on a drive that is not connected, is announced as "not found"; it keeps its place and is passed over until it returns. A row such as "Week 4, not on this computer" is a document that another computer queued and that is not found here; it keeps its place too, **Enter** on it only says so, and **Delete** removes the document from the queue on every computer. **Enter** opens a document, and reading begins only when you press the read key. **Delete** removes a document from the queue without touching the file.

**A document leaves the queue automatically** when continuous reading reaches its end, and when it is removed from the bookshelf.

### What happens at the end of a document

By default nothing happens: reading stops at the end of a document, and the next document in the queue waits for you. The setting **After a document ends** (`[reading] after_end` in `settings.toml`, in the Reading section of Settings) changes that:

- **stop** (`"stop"`, the default): reading stops and you hear "Done reading."
- **ask first** (`"ask"`): reading stops and names the next document: "Done reading. Next: Week 4. Enter to read." The key named is your read key, the one that reads from the cursor, such as Enter or Ctrl+Space. Pressed at the end of the document, it opens the next document and reads it. If you move elsewhere first, the read key reads the document you are in, as usual.
- **read the next** (`"next"`): the next document opens at once and is read from its saved place: "Done reading. Next: Week 4."

textweaver never starts a document you did not ask for: with **stop** and **ask first** the next document opens only on your key press, and **read the next** is itself the request. The next document is the first in the queue that is not the one you just finished and whose file is present.

### From the command line

```bash
tw library queue
tw library queue add week4.pdf
tw library queue remove week4.pdf
tw library queue next
```

`tw library queue` (or `tw library queue list`) prints the queue in order, each document followed by its path on the next line. `add` appends a document and `remove` takes one out; the file is never touched. `next` prints the path of the next document to read, so that a script can open it. With `--json`, each command prints the queue as `path`, `title`, and `found` (`path` is absent for a document that another computer queued and that is not found here); `next` prints `next`, which is null when there is nothing to read.

## The review list

**The review list** complements Continue reading: it lists the documents you have read aloud before, the one read longest ago first. It is in the File menu under Continue reading and in the command palette (F2). You hear the number of documents listed, then each row, meaning first:

"Pharmacology 2, 21 days, 80 percent, 3 notes"

A row gives the title, the number of whole days since you last read it ("today" for under a day), the furthest point you reached, and the number of notes. **Enter** opens a document. At most 50 documents are listed, and only those whose files are on this computer.

The list comes from your reading statistics (`stats.json`, see [Reading statistics](reading.md)), so a document appears once you have read it aloud. With statistics off (`[stats] enabled = false`), the list stays empty and says so.

The review list is a convenience for returning to older readings. It is not a study method: the research on learning gives little support to rereading, and nothing here schedules reviews. For practice with better evidence, see the self-test and the study cards in [Study with textweaver](notes.md#study-with-textweaver).

From the command line, `tw library review` prints the same rows, each followed by its path on the next line; `--json` gives each document's `path`, `title`, `days`, `last_read` (Unix seconds), `percent`, and `notes`.

## Recent files

Every document you open goes to the front of the recent list, `recent.json` in the data folder, which holds `recent_limit` files. A file opened again moves to the front rather than appearing twice, and Save As adds the new file as well.

## The bookshelf

Every document you open is also recorded on the bookshelf, `library.json` in the data folder. Each entry holds the document's full path, title, and file kind; when you first and last opened it; its author, DOI, and ISBN when known; and any details you entered yourself (under `edited`). The reading queue is kept in the same file, under `queue`, as a list of paths, and is omitted while empty. The bookshelf holds up to 500 documents; beyond that, those opened longest ago are dropped. `tw migrate-star` fills it from star's library.

## Search every document: tw library search

```bash
tw library search mitochondria
```

The search covers two things:

- titles, paths, authors, DOIs, and ISBNs: it reports the number of documents "matching" your words "by title, author, DOI, or ISBN", then lists each one. For example, `tw library search 10.1000/xyz` finds the paper with that DOI;
- the text of every document in the library folders and every recent file that still exists: it reports "Text matches in", the number of documents, then each document with its match count and a short passage, those with the most matches first. At most 50 are listed.

The first search reads every document, which takes some time for a large library. textweaver stores what it read in `fulltext.json` in the cache folder, and later searches reread only the documents that changed. If a document cannot be read, the search reports how many were skipped.

## Machine-readable output: --json

Every `tw library` form prints JSON instead with `--json`:

```bash
tw library search mitochondria --json
```

Options can be combined: `--add` and `--search` in one command add the folder and then search.

## The older place sync through a library folder

Before [sync](sync.md) existed, textweaver carried your reading place between computers through a library folder kept in step by Dropbox, OneDrive, Syncthing, or iCloud, as star did. That method still works:

- **With sync off,** textweaver keeps the folder's progress file up to date, as older versions did.
- **With sync on,** your places go to the sync folder instead, and the progress file is only read. A place an older textweaver, or star through `tw migrate-star`, wrote there is still honored when a document opens.

### What the progress file holds

Each library folder contains a small file, `.textweaver/progress.json`. For each document in the folder it records:

- the document's path, relative to the library folder, so it matches on every computer;
- the reading position (`offset`), the percentage (`pct`), and when it was saved (`ts`, in UTC).

It also has a `_meta` section recording the number of notes each document has. The notes, highlights, and bookmarks themselves do not travel through this file; [sync](sync.md) carries them.

With sync off, textweaver writes the file whenever it saves your place, and again when you open another document or quit.

### Which place wins

When you open a document, textweaver compares the place saved on this computer with the place in the folder's `progress.json`. The `[sync] position_policy` setting decides, the same setting that arbitrates between your computers' places with sync on:

```toml
[sync]
position_policy = "newest"
```

- `"newest"` (the default): the most recently saved place wins, from whichever computer.
- `"furthest"`: the place furthest into the document wins.
- `"ask"`: this computer's place is kept, and textweaver asks about the other.

This setting was formerly `[reading] sync_conflict_policy`; a value set there migrates to `[sync] position_policy` automatically, with `"highest_progress"` becoming `"furthest"` and `"manual"` becoming `"ask"`.

When the document opens you hear:

- "Opened", the title, "Resumed at 42 percent, from another device." when the place came from the folder;
- with `"ask"`, when the two places differ: "another computer at 42 percent. Go there? Y or N". **Y** goes there; **N** keeps this computer's place.

When two computers write the same `progress.json` at once, the entries are merged document by document under the same policy, and you hear "Sync: 2 library places differed."

`tw marks` shows both places for a document:

```bash
tw marks C:\Users\me\Readings\cells.md
```

## Where textweaver keeps its files

textweaver keeps its files in three folders.

- **The configuration folder** holds `settings.toml`, `keymap.toml`, and the `themes` folder.
- **The data folder** holds the `state` folder (one file per document with its position, history, bookmarks, notes, and highlights, plus the log file `textweaver.log`), the `recovery` folder (unsaved work from edit mode), `recent.json`, and `library.json`. These guides call the `state` folder the state folder.
- **The cache folder** holds `fulltext.json`, the library search cache. It can be deleted at any time and is rebuilt on demand.

Where they are:

- Windows: configuration `%APPDATA%\leavesofgrass\textweaver\config`, data `%APPDATA%\leavesofgrass\textweaver\data`, cache `%LOCALAPPDATA%\leavesofgrass\textweaver\cache`.
- macOS: configuration and data both `~/Library/Application Support/org.leavesofgrass.textweaver`, cache `~/Library/Caches/org.leavesofgrass.textweaver`.
- Linux: configuration `~/.config/textweaver`, data `~/.local/share/textweaver`, cache `~/.cache/textweaver`. If `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, or `XDG_CACHE_HOME` is set, textweaver uses it instead.

To see the configuration folder on your computer:

```bash
tw settings path
```

### Everything in one folder: TEXTWEAVER_HOME

Setting the `TEXTWEAVER_HOME` environment variable places everything under one folder, in `config`, `data`, and `cache` subfolders. This suits a portable copy on a USB stick, or an experiment that leaves your own settings untouched. An empty value is ignored.

On Windows, in PowerShell, for the current session only:

```powershell
$env:TEXTWEAVER_HOME = "E:\textweaver-home"
```

On macOS and Linux:

```bash
export TEXTWEAVER_HOME=~/textweaver-home
```

The reader's `--home FOLDER` option, and `--home` on every `tw` command that reads or writes the data folder, have the same effect for a single run. The [command line guide](command-line.md) lists them.

## Import from star: tw migrate-star

`tw migrate-star` copies your star data into textweaver. It only reads star's files and never changes them.

### Try it first: --dry-run

```bash
tw migrate-star --dry-run
```

A dry run reads everything and prints the full report without writing anything. The report starts "Dry run: nothing was written."

### Import

```bash
tw migrate-star
```

### Options

- `--from DIR`: star's configuration folder, the one containing its `settings.json`. By default textweaver looks in `%APPDATA%\star` on Windows, `~/Library/Application Support/star` on macOS, and `~/.config/star` on Linux. If there is no `settings.json` there, it stops and says "No star settings found in", the folder, then "Use --from with star's configuration directory."
- `--dry-run`: report without writing anything.
- `--json`: print the report as JSON.

### What is imported

- Settings that have a textweaver equivalent, where you had changed them from star's defaults.
- Library folders that still exist.
- Key changes made in star's window, as `keymap.toml` overrides. The new key is added; the single browse keys remain.
- For each document: the reading position, bookmarks, notes, and highlights.
- Recent files and the bookshelf.
- Each library folder's star sync file, `.star/progress.json`, converted to `.textweaver/progress.json` and merged with any textweaver one.
- star's settings profiles, into `profiles.toml`, one report line each. A profile retains the settings that textweaver stores in profiles and for which it has an equivalent: the voice, rate, and volume, the theme, and the highlight. The report line names each setting and its value, such as "Settings profiles: Study: speech.rate 200, display.theme nord", then what was left out, such as star's line height. Switch to a profile with **Ctrl+Shift+U** or **Alt+U** ([settings.md](settings.md#settings-profiles)). A textweaver profile of the same name is kept.

### What is skipped

Each skipped item is listed with its reason. The usual reasons are:

- a document that no longer exists, a web page, or an untitled document;
- a document textweaver cannot open;
- a setting with no textweaver equivalent (listed together);
- a profile none of whose settings has a textweaver equivalent;
- a key textweaver cannot read, or a star shortcut for a command textweaver does not have;
- star's saved note searches, because textweaver does not store those yet. (star's reading statistics are imported, into `stats.json`.)

Running the import twice adds nothing new. Where textweaver already holds an item, its copy wins: a newer position, a bookmark name already in use, a note already present.

### How positions are mapped

star and textweaver lay out a document's text differently; star, for example, ran list items together, whereas textweaver keeps each on its own line. star's saved character positions therefore do not correspond to the same places in textweaver, and textweaver maps each position by words:

1. It identifies the word on which star's position fell.
2. It aligns the words of star's text with those of textweaver's text and locates the same word there.
3. If no words align, it falls back on the percentage star saved.

star's own cached copy of the text is used when it is still current, and the report says for how many documents; otherwise textweaver's text stands in.

### The report

The report states where star's files were found and how many files were written, then gives a summary per kind, such as "Reading positions: 12 imported, 1 already present, 2 skipped", then every imported item and every skipped item with its reason.

## If something goes wrong

- **A document is missing from the library.** Check that its folder was added (`tw library`), that textweaver can open its kind of file, and that it is not in a hidden or skipped folder.
- **"Could not open" from the library.** The file was moved or deleted after it was listed.
- **The place from another computer is wrong.** Set `[sync] position_policy = "ask"`, and textweaver asks before adopting another computer's place.
- **A document read on another computer is not in Continue reading.** It is listed only when the document is also on this computer: opened here before, in a library folder, or among your recent files. Sync must be on for other computers' places.
- **The library search is slow the first time.** The first search reads every document once; later searches are fast.
- **`tw migrate-star` found nothing.** Use `--from` with the folder that holds star's `settings.json`.

## See also

- [Syncing between computers](sync.md): notes, highlights, bookmarks, places, library details, and statistics on your other computers.
- [Bookmarks, notes, and highlights](notes.md): what is stored for each document.
- [Settings](settings.md): where settings live, and how to export and import them.
- [Obsidian vaults](vault.md): importing a vault's documents into the library.
- [Citations](citations.md): `tw cite`, the reference library that fills in a document's author, DOI, and ISBN when it has none of its own.
- [ADR-0002: Text model](adr/0002-text-model.md): how positions work, and how star's are mapped.
- [Documentation index](README.md)
