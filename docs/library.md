# The library

This guide covers textweaver's library: the folders of documents you read from, the list of recent files, searching the text of every document at once, and keeping your reading place in step between computers. It also covers where textweaver keeps its files, and importing your data from Star. It is for anyone with more than a few documents, and for anyone moving from Star.

## Open the library in the reader: Alt+L

Press **Alt+L** in the terminal reader. The GUI uses **Ctrl+Shift+B**. You hear "Library", the number of documents, then "Type to filter, Enter opens one."

The folders are read in the background, so a large library (up to 20,000 files) never holds up the keyboard. While it is read you may hear "Scanning the library.", and the status line counts the documents found every second; the list opens when the scan is done. Pressing **Alt+L** again meanwhile says how many have been found so far.

The list has every document in your library folders, then the files you opened recently that are not in those folders, newest first. Each item says:

- the title;
- "by", then the author, when textweaver knows it;
- how far you have read, as a percentage, when you have a saved place;
- "in", then the folder's name, for a document in a library folder, or "recent" for a recent file.

For example: "Cells, 25 percent, in Readings".

Use **Up** and **Down** to move, **Enter** to open, and **Escape** to close. With nothing to show you hear: "The library is empty. Add a folder with tw library --add, or open a file with Ctrl+O."

### Filter the list as you type

Type in the list to filter it. Each word you type must be in a document's title, path, author, DOI, or ISBN, or in its text. You hear how many documents match, such as "2 documents match.", and the list shows only those. **Backspace** removes a letter; with the filter empty you hear "Filter cleared", then the number of documents. When nothing matches you hear "No documents match", then what you typed.

A DOI or an ISBN matches however it is written: `10.1000/xyz`, `doi:10.1000/XYZ`, a `https://doi.org/` link, `978-0-306-40615-7`, or the same book's ten-digit ISBN.

The text of a document counts once `tw library --search` has read it (below); the list uses what that search keeps in the cache, and never reads documents itself.

## Search by author, DOI, and ISBN

textweaver learns a document's author, DOI, and ISBN in three ways:

- **When you open it.** The author comes from the document's own details: Markdown front matter (`author`, `doi`, `isbn`), a Word or EPUB file's author, or a web page's `<meta>` tags (`citation_doi`, `dc.identifier`). A DOI or an ISBN printed near the start of the text counts too, such as a paper's DOI on its first page or a book's ISBN on its copyright page. An ISBN counts only after the word "ISBN". These go on the bookshelf, below.
- **From its text,** once `tw library --search` has read it, for documents you have not opened yet.
- **From your reference library.** When `tw cite` has a record of the same work (the same DOI or ISBN, or the same title when it is at least twelve letters long), its authors, DOI, and ISBN fill in what the document lacks. Both your personal `references.json` and each library folder's `references.json` count.

Word and EPUB files keep their DOI or ISBN in an identifier field that textweaver does not read yet; a DOI or ISBN printed in the text is found.

## Library folders

A library folder is an ordinary folder of documents. textweaver lists every document it can open in the folder and in all its subfolders. It skips hidden folders, its own `.textweaver` folders, Star's `.star` folders, `.obsidian`, `.git`, `node_modules`, and the recycle bin. It reads at most 20,000 files per folder. Files are never changed.

### Add a folder: tw library --add

```bash
tw library --add C:\Users\me\Readings
```

You hear, for example, "Added folder Readings with 12 documents". Adding the same folder again says it is "already in the library". A path that is not a folder is refused.

### Remove a folder: tw library --remove

```bash
tw library --remove C:\Users\me\Readings
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

## Recent files

Every document you open goes to the front of the recent list, `recent.json` in the data folder. The list keeps `recent_limit` files. A file opened again moves to the front instead of appearing twice. Save As adds the new file too.

## The bookshelf

Every document you open is also recorded on the bookshelf, `library.json` in the data folder. Each entry holds the document's full path, its title, the kind of file, when you first opened it, when you last opened it, and its author, DOI, and ISBN when known. The bookshelf keeps up to 500 documents; past that, the ones opened longest ago are dropped. `tw migrate-star` fills it from Star's library.

## Search every document: tw library --search

```bash
tw library --search mitochondria
```

`--search` looks in two places:

- titles, paths, authors, DOIs, and ISBNs: the number of documents "matching" your words "by title, author, DOI, or ISBN", then each one. For example, `tw library --search 10.1000/xyz` finds the paper with that DOI;
- the text of every document in the library folders and every recent file that still exists: "Text matches in", the number of documents, then each document with its number of matches and a short passage. The documents with the most matches come first. At most 50 are listed.

The first search reads every document, which takes a while for a large library. textweaver keeps what it read in `fulltext.json` in the cache folder, and later searches read only the documents that changed. If a document cannot be read, the search says how many were skipped.

## Machine-readable output: --json

Every `tw library` form prints JSON instead with `--json`:

```bash
tw library --search mitochondria --json
```

You can combine options: `--add` and `--search` in one command adds the folder, then searches.

## Sync your place between computers

A library folder can live in Dropbox, OneDrive, Syncthing, iCloud, or any other synced folder. textweaver then keeps your reading place in step between the computers that use it.

### What is synced

Each library folder gets a small file, `.textweaver/progress.json`, inside the folder. For each document in the folder it holds:

- the document's path, relative to the library folder, so it matches on every computer;
- the reading position (`offset`), the percentage (`pct`), and when it was saved (`ts`, in UTC).

It also has a `_meta` part with the number of notes each document has.

The notes, highlights, and bookmarks themselves are not synced. They stay on the computer where you made them.

textweaver writes this file whenever it saves your place, and again when you open another document or quit.

### Which place wins

When you open a document, textweaver compares the place saved on this computer with the place in the folder's `progress.json`. `[reading] sync_conflict_policy` decides:

```toml
[reading]
sync_conflict_policy = "newest"
```

- `"newest"` (the default): the place saved most recently wins, from whichever computer.
- `"highest_progress"`: the place furthest into the document wins.
- `"manual"`: this computer's place is always kept.

What you hear when the document opens:

- "Opened", the title, "Resumed at 42 percent, from another device." when the place came from the folder;
- "Opened", the title, "Resumed at 42 percent. Another device is at a different place; kept this device's." with `"manual"` when the two differ.

When two computers write the same `progress.json` at once, the entries are merged document by document with the same policy.

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

The reader's `--home FOLDER` option, and `--home` on `tw open`, `tw settings`, `tw speak`, `tw voices`, `tw backends`, `tw export-audio`, and `tw serve`, do the same for one run. `tw library`, `tw marks`, and `tw migrate-star` have no `--home` option; they follow `TEXTWEAVER_HOME`.

## Import from Star: tw migrate-star

`tw migrate-star` copies what you had in Star into textweaver. It only reads Star's files; it never changes them.

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

- `--from DIR`: Star's configuration folder, the one that holds its `settings.json`. By default textweaver looks in `%APPDATA%\star` on Windows, `~/Library/Application Support/star` on macOS, and `~/.config/star` on Linux. If there is no `settings.json` there, it stops and says "No Star settings found in", the folder, then "Use --from with Star's configuration directory."
- `--dry-run`: report without writing anything.
- `--json`: print the report as JSON.

### What is imported

- Settings that have a textweaver equivalent, when you changed them from Star's defaults.
- Library folders that still exist.
- GUI key changes, as `keymap.toml` overrides. The new key is added; the single browse keys stay.
- For each document: the reading position, bookmarks, notes, and highlights.
- Recent files and the bookshelf.
- Each library folder's Star sync file, `.star/progress.json`, converted to `.textweaver/progress.json` and merged with any textweaver one.

### What is skipped

Each skipped item is listed with the reason. The usual reasons:

- a document that no longer exists, a web page, or an untitled document;
- a document textweaver cannot open;
- a setting with no textweaver equivalent (listed together);
- a key textweaver cannot read, or a Star shortcut for a command textweaver does not have;
- Star's saved note searches, because textweaver does not keep those yet. (Star's reading statistics are imported, into `stats.json`.)

Running it twice imports nothing new. Where textweaver already has something, textweaver's copy wins: a newer position, a bookmark name already used, a note already there.

### How positions are mapped

Star and textweaver lay out a document's text differently. For example, Star ran list items together, and textweaver keeps each on its own line. So Star's saved character positions do not point to the same places in textweaver.

textweaver maps each position by words:

1. It finds the word Star's position was on.
2. It lines up the words of Star's text with the words of textweaver's text, and finds the same word there.
3. If no words line up, it uses the percentage Star saved.

Star's own cached copy of the text is used when it is still current; the report says for how many documents. Otherwise textweaver's text stands in.

### The report

The report says where Star's files were, how many files were written, then a summary per kind, such as "Reading positions: 12 imported, 1 already present, 2 skipped", then every imported item and every skipped item with its reason.

## If something goes wrong

- **A document is missing from the library.** Check that its folder was added (`tw library`), that textweaver can open its kind of file, and that it is not in a hidden or skipped folder.
- **"Could not open" from the library.** The file was moved or deleted since it was listed.
- **The place from another computer is wrong.** Set `[reading] sync_conflict_policy = "manual"` to always keep this computer's place.
- **The library search is slow the first time.** It reads every document once; later searches are fast.
- **`tw migrate-star` found nothing.** Use `--from` with the folder that holds Star's `settings.json`.

## See also

- [Bookmarks, notes, and highlights](notes.md): what is stored for each document.
- [Settings](settings.md): where settings live, and how to export and import them.
- [Obsidian vaults](vault.md): importing a vault's documents into the library.
- [ADR-0002: Text model](adr/0002-text-model.md): how positions work, and how Star's are mapped.
- [Documentation index](README.md)
