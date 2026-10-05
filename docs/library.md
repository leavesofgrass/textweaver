# The library

This guide covers textweaver's library: the folders of documents you read from, the list of recent files, "Continue reading", searching the text of every document at once, and the older way of keeping your reading place in step between computers through a library folder. It also covers where textweaver keeps its files, and importing your data from star. It is for anyone with more than a few documents, and for anyone moving from star. Syncing notes, highlights, bookmarks, places, and library details between your computers has [a guide of its own](sync.md).

## Open the library in the reader: Alt+L

Press **Alt+L** in the terminal reader. The textweaver app uses **Ctrl+Shift+B**. You hear "Library", the number of documents, then "Type to filter, Enter opens one, F2 edits details."

The folders are read in the background, so a large library (up to 20,000 files) never holds up the keyboard. While it is read you may hear "Scanning the library.", and the status line counts the documents found every second; the list opens when the scan is done. Pressing **Alt+L** again meanwhile says how many have been found so far.

The list has every document in your library folders, then the files you opened recently that are not in those folders, newest first. Each item says:

- the title;
- "by", then the author, when textweaver knows it;
- how far you have read, as a percentage, when you have a saved place;
- "in", then the folder's name, for a document in a library folder, or "recent" for a recent file.

For example: "Cells, 25 percent, in Readings".

Use **Up** and **Down** to move, **Enter** to open, **F2** to edit a document's details (below), and **Escape** to close. With nothing to show you hear: "The library is empty. Add a folder in Settings, under Library folders, or open a file with Ctrl+O."

### Filter the list as you type

Type in the list to filter it. Each word you type must be in a document's title, path, author, DOI, or ISBN, or in its text. You hear how many documents match, such as "2 documents match.", and the list shows only those. **Backspace** removes a letter; with the filter empty you hear "Filter cleared", then the number of documents. When nothing matches you hear "No documents match", then what you typed.

A DOI or an ISBN matches however it is written: `10.1000/xyz`, `doi:10.1000/XYZ`, a `https://doi.org/` link, `978-0-306-40615-7`, or the same book's ten-digit ISBN.

The text of a document counts once `tw library search` has read it (below); the list uses what that search keeps in the cache, and never reads documents itself.

## Search by author, DOI, and ISBN

textweaver learns a document's author, DOI, and ISBN in three ways:

- **When you open it.** The author comes from the document's own details: Markdown front matter (`author`, `doi`, `isbn`), a Word or EPUB file's author, or a web page's `<meta>` tags (`citation_doi`, `dc.identifier`). A DOI or an ISBN printed near the start of the text counts too, such as a paper's DOI on its first page or a book's ISBN on its copyright page. An ISBN counts only after the word "ISBN". These go on the bookshelf, below.
- **From its text,** once `tw library search` has read it, for documents you have not opened yet.
- **From your reference library.** When `tw cite` has a record of the same work (the same DOI or ISBN, or the same title when it is at least twelve letters long), its authors, DOI, and ISBN fill in what the document lacks. Both your personal `references.json` and each library folder's `references.json` count.
- **From your other computers,** with [sync](sync.md) on. When another computer opened a document, its title, author, DOI, and ISBN travel with it, so the filter and `tw library search` find it here by a DOI only that computer knew, even if you never opened it here. textweaver finds the document here when you opened it here before, when it is in a library folder that is itself synced between the computers (its `.textweaver/library-id.json` travels with it), or when `tw library search` has read its text. When two computers know different details, the newest wins, detail by detail.

Word and EPUB files keep their DOI or ISBN in an identifier field that textweaver does not read yet; a DOI or ISBN printed in the text is found.

## Edit a document's details

When a document's details are wrong or missing, such as a scan named "scan0042" with no author, you can type them yourself: its title, author, DOI, and ISBN. What you type wins over what the document says about itself, in the library list, its filter, and `tw library search`, and it stays when the document is opened again.

### In the reader

There are two ways to open the form:

- **For the open document:** choose **Edit details** in the File menu, under Continue reading, or type "edit details" in the command palette (F2). It has no key of its own; you can give it one in `keymap.toml`.
- **From the library list:** move to a document and press **F2**. When you save or cancel, the list comes back on the same document, with its row changed.

You hear "Details of", the document's title, then "Tab moves, Enter saves, Escape cancels." The form has four fields, one line each: Title, Author, DOI, and ISBN. Each field's label says where it is, such as "Author, 2 of 4", and starts with the field's current value.

- **Tab** moves to the next field and **Shift+Tab** to the previous one. You hear the field's label and value, such as "Author: Ada Example", or "Author: blank" for an empty field. What you typed in a field is kept when you move away from it. Tab from the last field goes back to the first.
- **Enter** saves every field you changed, from any field. You hear "Details saved:" and the fields, such as "Details saved: Title, Author." With nothing changed you hear "Details not changed."
- **Escape** closes the form without saving anything, and you hear "Cancelled. Details not changed."

A DOI or an ISBN can be typed any way it is usually written: `10.1000/xyz`, `doi:10.1000/XYZ`, a `https://doi.org/` link, or an ISBN with or without hyphens. One that is not a DOI or an ISBN is not saved: you hear, for example, "Not a DOI: 10.10/x. Fix it or clear it.", and the form opens again on that field.

**Clearing a field** removes your edit, so the document's own value shows again, at once in the library list. A field you never edited cannot hide the document's own value.

In the app the form is a dialog with one field at a time, labeled the same way; Tab and Shift+Tab move between the fields, Enter saves, and Escape cancels.

### From the command line: tw library edit

```bash
tw library edit "C:\Users\me\Readings\scan0042.pdf" --title "Cell Biology, Chapter 3" --author "Ada Example"
tw library edit scan0042.pdf --doi 10.1000/cells --json
tw library edit scan0042.pdf --author ""
```

Give any of `--title`, `--author`, `--doi`, and `--isbn`; the others keep their values. An empty value, such as `--author ""`, clears your edit. It prints "Details saved:" and the fields, or "Details not changed." A DOI or ISBN that is not one is refused with the same message as in the reader. With `--json` it prints the document's `path`, whether the bookshelf `changed`, the `fields` given, whether the edit was `published` to the sync folder, and every detail you have `edited`.

### On your other computers

With [sync](sync.md) on, what you type travels to your other computers with the document's other library details. Your edit wins over the details the document states, even when another computer opens the document later. When you edit the same detail on two computers, the newest edit wins. Clearing a detail travels too.

## Library folders

A library folder is an ordinary folder of documents. textweaver lists every document it can open in the folder and in all its subfolders. It skips hidden folders, its own `.textweaver` folders, star's `.star` folders, `.obsidian`, `.git`, `node_modules`, and the recycle bin. It reads at most 20,000 files per folder. Files are never changed.

### Add a folder

In the reader, in the app or the terminal, choose **Add a folder to the library** in the File menu, or type its name in the command palette. The file browser opens on your places; go to the folder and choose it with **Ctrl+Enter**, or the "Choose this folder" row. You hear, for example, "Added Readings to the library. Open the library to see its documents." Choosing a folder already there says it is "already in the library".

On the command line:

```bash
tw library add C:\Users\me\Readings
```

You hear, for example, "Added folder Readings with 12 documents". Adding the same folder again says it is "already in the library". A path that is not a folder is refused.

### Remove a folder: tw library remove

```bash
tw library remove C:\Users\me\Readings
```

It says "Removed", the folder, then "from the library; its files are unchanged".

### See the library: tw library

```bash
tw library
```

With no options, `tw library` lists the library folders, then every document, one per item, with its path on the next line.

### The settings

The folders are kept in `settings.toml`, in the `[library]` section. You can edit them there too:

```toml
[library]
folders = ["C:\\Users\\me\\Readings", "D:\\Course"]
recent_limit = 20
```

- `folders` (default: none): the library folders. In TOML, a backslash in a path is written twice.
- `recent_limit` (default `20`): how many recent files to remember.

## Continue reading

**Continue reading** lists the documents on this computer that have a reading place, newest first, whichever computer read them last. It is in the File menu under Library, and in the command palette (F2); it has no key of its own, and you can give it one in `keymap.toml`. You hear how many documents it lists, then each row, meaning first:

"Cells, 42 percent, laptop, 2 hours ago"

That is the title, how far into the document the place is, the computer the place is from, and how long ago it was saved. A place saved on this computer names this computer. Enter opens the document, which resumes by `[sync] position_policy` (see [Syncing between computers](sync.md#a-place-from-another-computer)).

Only documents found on this computer are listed: one you read only on another computer, and do not have here, is left out. The places of your other computers count only with sync on; without it, the list has this computer's own places. With nothing to list you hear "Nothing to continue: no places saved."

From the command line:

```bash
tw library continue
tw library continue --json
```

`--continue` prints the same rows, each with its path on the next line. With `--json`, each document has its `path`, `title`, `pct`, the `device` name, `this_computer`, and `when_ms` (when the place was saved, in milliseconds since 1970, UTC).

## Recent files

Every document you open goes to the front of the recent list, `recent.json` in the data folder. The list keeps `recent_limit` files. A file opened again moves to the front instead of appearing twice. Save As adds the new file too.

## The bookshelf

Every document you open is also recorded on the bookshelf, `library.json` in the data folder. Each entry holds the document's full path, its title, the kind of file, when you first opened it, when you last opened it, its author, DOI, and ISBN when known, and the details you typed yourself (under `edited`). The bookshelf keeps up to 500 documents; past that, the ones opened longest ago are dropped. `tw migrate-star` fills it from star's library.

## Search every document: tw library search

```bash
tw library search mitochondria
```

`--search` looks in two places:

- titles, paths, authors, DOIs, and ISBNs: the number of documents "matching" your words "by title, author, DOI, or ISBN", then each one. For example, `tw library search 10.1000/xyz` finds the paper with that DOI;
- the text of every document in the library folders and every recent file that still exists: "Text matches in", the number of documents, then each document with its number of matches and a short passage. The documents with the most matches come first. At most 50 are listed.

The first search reads every document, which takes a while for a large library. textweaver keeps what it read in `fulltext.json` in the cache folder, and later searches read only the documents that changed. If a document cannot be read, the search says how many were skipped.

## Machine-readable output: --json

Every `tw library` form prints JSON instead with `--json`:

```bash
tw library search mitochondria --json
```

You can combine options: `--add` and `--search` in one command adds the folder, then searches.

## The older place sync through a library folder

Before [sync](sync.md), a library folder kept in step by Dropbox, OneDrive, Syncthing, or iCloud was how textweaver carried your reading place between computers, and star did the same. That still works:

- **With sync off,** textweaver keeps the folder's progress file up to date, as older versions did.
- **With sync on,** your places go to the sync folder instead, and the progress file is only read. A place an older textweaver, or star through `tw migrate-star`, wrote there is still honored when a document opens.

### What the progress file holds

Each library folder gets a small file, `.textweaver/progress.json`, inside the folder. For each document in the folder it holds:

- the document's path, relative to the library folder, so it matches on every computer;
- the reading position (`offset`), the percentage (`pct`), and when it was saved (`ts`, in UTC).

It also has a `_meta` part with the number of notes each document has.

The notes, highlights, and bookmarks themselves do not travel through this file; [sync](sync.md) carries them.

With sync off, textweaver writes this file whenever it saves your place, and again when you open another document or quit.

### Which place wins

When you open a document, textweaver compares the place saved on this computer with the place in the folder's `progress.json`. `[sync] position_policy` decides, the same setting that decides between your computers' places with sync on:

```toml
[sync]
position_policy = "newest"
```

- `"newest"` (the default): the place saved most recently wins, from whichever computer.
- `"furthest"`: the place furthest into the document wins.
- `"ask"`: this computer's place is kept, and textweaver asks about the other one.

This setting used to be `[reading] sync_conflict_policy`; a value you set there moves to `[sync] position_policy` on its own, with `"highest_progress"` becoming `"furthest"` and `"manual"` becoming `"ask"`.

What you hear when the document opens:

- "Opened", the title, "Resumed at 42 percent, from another device." when the place came from the folder;
- with `"ask"`, when the two differ: "another computer at 42 percent. Go there? Y or N". Y goes there; N keeps this computer's place.

When two computers write the same `progress.json` at once, the entries are merged document by document with the same policy, and you hear it: "Sync: 2 library places differed."

`tw marks` shows both places for a document:

```bash
tw marks C:\Users\me\Readings\cells.md
```

## Where textweaver keeps its files

textweaver keeps its files in three folders.

- **The configuration folder** holds `settings.toml`, `keymap.toml`, and the `themes` folder.
- **The data folder** holds the `state` folder (one file per document with its position, history, bookmarks, notes, and highlights, and the log file `textweaver.log`), the `recovery` folder (unsaved work from edit mode), `recent.json`, and `library.json`. The `state` folder is called the state folder in these guides.
- **The cache folder** holds `fulltext.json`, the library search cache. You can delete it at any time; it is rebuilt.

Where they are:

- Windows: configuration `%APPDATA%\leavesofgrass\textweaver\config`, data `%APPDATA%\leavesofgrass\textweaver\data`, cache `%LOCALAPPDATA%\leavesofgrass\textweaver\cache`.
- macOS: configuration and data both `~/Library/Application Support/org.leavesofgrass.textweaver`, cache `~/Library/Caches/org.leavesofgrass.textweaver`.
- Linux: configuration `~/.config/textweaver`, data `~/.local/share/textweaver`, cache `~/.cache/textweaver`. If `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, or `XDG_CACHE_HOME` is set, textweaver uses it instead.

To see the configuration folder on your computer:

```bash
tw settings path
```

### Everything in one folder: TEXTWEAVER_HOME

Set the `TEXTWEAVER_HOME` environment variable to put everything under one folder, in `config`, `data`, and `cache` subfolders. Use it for a portable copy on a USB stick, or to try something without touching your own settings. An empty value is ignored.

On Windows, in PowerShell, for this window only:

```powershell
$env:TEXTWEAVER_HOME = "E:\textweaver-home"
```

On macOS and Linux:

```bash
export TEXTWEAVER_HOME=~/textweaver-home
```

The reader's `--home FOLDER` option, and `--home` on every `tw` command that reads or writes the data folder, do the same for one run. The [command line guide](command-line.md) lists them.

## Import from star: tw migrate-star

`tw migrate-star` copies what you had in star into textweaver. It only reads star's files; it never changes them.

### Try it first: --dry-run

```bash
tw migrate-star --dry-run
```

A dry run reads everything and prints the full report, but writes nothing. The report starts "Dry run: nothing was written."

### Import

```bash
tw migrate-star
```

### Options

- `--from DIR`: star's configuration folder, the one that holds its `settings.json`. By default textweaver looks in `%APPDATA%\star` on Windows, `~/Library/Application Support/star` on macOS, and `~/.config/star` on Linux. If there is no `settings.json` there, it stops and says "No star settings found in", the folder, then "Use --from with star's configuration directory."
- `--dry-run`: report without writing anything.
- `--json`: print the report as JSON.

### What is imported

- Settings that have a textweaver equivalent, when you changed them from star's defaults.
- Library folders that still exist.
- Window key changes, as `keymap.toml` overrides. The new key is added; the single browse keys stay.
- For each document: the reading position, bookmarks, notes, and highlights.
- Recent files and the bookshelf.
- Each library folder's star sync file, `.star/progress.json`, converted to `.textweaver/progress.json` and merged with any textweaver one.
- star's settings profiles, into `profiles.toml`, one report line each. A profile keeps the settings textweaver keeps in profiles and has an equivalent for: the voice, rate, and volume, the theme, and the highlight. The report line names each setting and its value, such as "Settings profiles: Study: speech.rate 200, display.theme nord", then what was left out, such as star's line height. Switch to one with **Ctrl+Shift+U** or **Alt+U** ([settings.md](settings.md#settings-profiles)). A textweaver profile of the same name is kept.

### What is skipped

Each skipped item is listed with the reason. The usual reasons:

- a document that no longer exists, a web page, or an untitled document;
- a document textweaver cannot open;
- a setting with no textweaver equivalent (listed together);
- a profile none of whose settings has a textweaver equivalent;
- a key textweaver cannot read, or a star shortcut for a command textweaver does not have;
- star's saved note searches, because textweaver does not keep those yet. (star's reading statistics are imported, into `stats.json`.)

Running it twice imports nothing new. Where textweaver already has something, textweaver's copy wins: a newer position, a bookmark name already used, a note already there.

### How positions are mapped

star and textweaver lay out a document's text differently. For example, star ran list items together, and textweaver keeps each on its own line. So star's saved character positions do not point to the same places in textweaver.

textweaver maps each position by words:

1. It finds the word star's position was on.
2. It lines up the words of star's text with the words of textweaver's text, and finds the same word there.
3. If no words line up, it uses the percentage star saved.

star's own cached copy of the text is used when it is still current; the report says for how many documents. Otherwise textweaver's text stands in.

### The report

The report says where star's files were, how many files were written, then a summary per kind, such as "Reading positions: 12 imported, 1 already present, 2 skipped", then every imported item and every skipped item with its reason.

## If something goes wrong

- **A document is missing from the library.** Check that its folder was added (`tw library`), that textweaver can open its kind of file, and that it is not in a hidden or skipped folder.
- **"Could not open" from the library.** The file was moved or deleted since it was listed.
- **The place from another computer is wrong.** Set `[sync] position_policy = "ask"`, and textweaver asks before it goes to another computer's place.
- **A document read on another computer is not in Continue reading.** It is listed only when the document is on this computer too: opened here before, in a library folder, or among your recent files. Sync must be on for other computers' places.
- **The library search is slow the first time.** It reads every document once; later searches are fast.
- **`tw migrate-star` found nothing.** Use `--from` with the folder that holds star's `settings.json`.

## See also

- [Syncing between computers](sync.md): notes, highlights, bookmarks, places, library details, and statistics on your other computers.
- [Bookmarks, notes, and highlights](notes.md): what is stored for each document.
- [Settings](settings.md): where settings live, and how to export and import them.
- [Obsidian vaults](vault.md): importing a vault's documents into the library.
- [Citations](citations.md): `tw cite`, the reference library that fills in a document's author, DOI, and ISBN when it has none of its own.
- [ADR-0002: Text model](adr/0002-text-model.md): how positions work, and how star's are mapped.
- [Documentation index](README.md)
