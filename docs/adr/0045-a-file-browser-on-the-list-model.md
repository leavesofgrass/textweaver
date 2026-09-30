# ADR-0045: A file browser on the list model

- Status: accepted
- Date: 2026-09-29 (Tuesday, September 29, 2026)

## Context

Before this decision there were three ways to open a document: the system's Open dialog in the GUI (`rfd`), a typed path with Tab completion in both frontends, and the library. None of them could look inside an archive. `textweaver_formats::archive` could already list and read zip, tar, tar.gz, and 7z archives, address a member as `book.zip!chapter.pdf`, nest archives up to four deep, and refuse hostile ones; opening an archive showed a document of links. A student handed `course.zip` with twenty files in six folders had to open it as a page of links, follow one, and start again for the next.

Three commands planned for the same wave need a place to choose a folder or a file: batch conversion, audio export, and settings import. Each could have built its own prompt; that would give three ways of choosing, each with its own gaps.

Both frontends already share a list model (`ListModel`, Wave 3): one list, with its title, rows, focus, and the keys every list takes, announced the same way in the terminal, the GUI's list dialog, and JSON-RPC.

## Decision

### One list that enters and leaves places

The browser (`crates/textweaver-app/src/browse.rs`) is a single pane on the list model. It shows one place at a time: the places, a folder on disk, or a folder inside an archive. Enter (or Right) goes into a folder or an archive and opens a document; Backspace (or Left) goes up and lands on the row it left, so the way back is always where the user expects. Typing filters by name, as in the library.

A tree view was considered and rejected: a tree is harder to follow by ear than a list that is entered and left, since every level's expansion state has to be said and remembered, and both frontends already have the list. Two panes were rejected too: a second pane earns its place for copying and moving, and the browser does neither (below).

It opens on the places: the open document's folder (focused, since it is the most likely next step), the folder textweaver started in, the library's folders, and on Windows the drives. Drives are listed from `GetLogicalDrives` and named by `GetDriveTypeW` ("disk", "removable drive", "network drive"), neither of which touches the drive, so a disconnected network drive or an empty card reader cannot stall the list. Other systems list the root folder.

### Rows say the meaning first

Every row starts with the name, then the kind, then the size or count: "notes.md, Markdown, 12 KB", "Week 1, folder, 12 items", "course.zip, zip archive, 3.4 MB". The first cells of a 40-cell Braille line hold what the user needs to decide, and a screen reader's first words are the name. The position is said after the row ("..., 3 of 40"), not before it as in other lists; `ListModel::with_position_last` carries that per list, with its own message.

Kinds are words from the catalog, one per loader (`browse-kind-markdown` is "Markdown", `browse-kind-docx` "Word document"), with the extension in capitals for a loader without a name. Sizes are in bytes under a kilobyte and in KB, MB, and GB above, with one decimal under ten, in the language's number format. A folder's count is what its row would list with the same settings, so the number heard matches what is shown on entering. Only the first 500 folders of a folder are counted, so a huge folder still lists quickly.

### Archives: members addressed, nothing extracted

An archive is entered by listing it (`archive::list_path`) and building its folders from the members' names. A member opens by its member path, `course.zip!week1/notes.md`, the form the loaders already read and the key under which its reading position, bookmarks, and notes are kept. The same addressing was Star's (`archive.zip!inner/path`), and it keeps annotations valid across sessions without ever extracting a file. An archive inside an archive is entered the same way (`course.zip!week1/extra.zip`), up to the archive module's four levels; `archive::nesting` refuses a deeper one before reading it.

Nothing is written to disk. Members are read into memory within the archive module's limits (a member's size, the bytes decompressed while searching, a 7z dictionary's size, the number of entries). Names are already normalized by the archive module, so a member called `../../secret.txt` lists as `secret.txt` inside the archive and cannot climb out of it; there is no extraction for a zip-slip or tar-slip to exploit. Junk that is never a document is left out (`__MACOSX/`, `.DS_Store`, `Thumbs.db`, `desktop.ini`: `archive::is_junk`). A damaged archive, one that expands to too much, and one nested too deeply are each refused with a sentence that says which, and the list stays where it was.

### Hostile names

A file name can hold control characters (an escape sequence that clears or rewrites a terminal) or direction marks (`photo\u{202E}gpj.exe` shown as `photoexe.jpg`). The browser shows every name with control, format, and direction characters replaced by a question mark. The real name is kept for opening.

### Readable files first

Only folders, archives, and files a loader reads are listed by default; the introduction says how many were left out ("1 file hidden"). The Show All key lists every file, hidden ones included. Sorting is by name (numbers in order, so "Week 2" comes before "Week 10"), by date (newest first), or by size (largest first), folders always first. The order and Show All last for the session and are not settings: a setting nobody asked for is one more thing to explain.

### The preview is read in the background

The Say Status key, which in other lists repeats the list's introduction, says a preview of the focused row in the browser: a document's title and first sentence, an archive's count of files and its first names, a folder's full path and first names. The document is loaded on a helper thread with the loaders' own limits and text recognition off, so a scanned PDF cannot keep a thread busy for minutes; a newer preview cancels the older one's loading. The result is said only if the list it was asked in is still shown (the dialog generation of ADR-0043), so a preview never arrives for a closed browser. The frontends send the new list key `ListKey::Details` for Say Status; lists other than the browser treat it as the introduction, so nothing else changes.

### Choosing, for other commands

`App::choose_folder(purpose, then)` and `App::choose_file(purpose, extensions, then)` open the same browser for another command and call `then` with the path chosen. The purpose is said with the places ("Choose the folder to convert. Places, 5 places."). A folder is chosen with the Choose Folder key, Ctrl+Enter, on a folder row or anywhere in the folder shown. Many terminals send the same thing for Enter and Ctrl+Enter, so each folder also starts with a "Choose this folder" row that Enter takes. A folder inside an archive cannot be chosen: every command that asks for a folder writes to it or reads from it on disk. While choosing a file, archives are listed as files and not entered, and only files with the caller's extensions are listed. Escape closes the chooser without calling `then`.

The browser's three keys (Choose Folder, Sort, Show All) are not in the keymap: they mean something only while the browser is shown, and the keymap has no layer for one list. They follow the keymap's command modifier instead (`command_modifier`, Open's): Ctrl+Enter, Ctrl+R, and Ctrl+A, or Cmd+Enter, Cmd+R, and Cmd+Shift+Period in the Mac window, the Mac's own "show hidden files". The frontends ask the app for them (`App::browse_list_key_for`), so the key and the name said for it come from one table, and tests press them from that table.

### It opens and chooses; it never changes a file

The browser has no copy, move, rename, or delete. textweaver is a reader, and a delete from a reader is exactly the kind of action that should need a file manager's deliberate steps and confirmations. Delete and F2 in the browser say that it only opens and chooses.

### Opening many documents in a row

The browser makes opening one document after another common, so two costs left the input thread when a document opens. The text's stamp (an FNV-1a hash of the whole text, 15 to 30 ms on 10 MB in a release build) is computed on the loading thread for documents opened in the background. The wait of up to two seconds for the writer thread is gone: the writer keeps the newest state it has queued for each document until that job is done, and opening reads it from there, so a position saved a moment before is found even on a slow disk. The library's scan, which reads the recent list the writer updates, waits for the writer on its own thread. Measured with the disk stalled for 1.5 seconds in a debug build on 10 MB of text, opening took 354 ms (89 ms of it the stamp, which a foreground open still computes); the wait it replaced was 1,151 ms.

## Consequences

- Archives are browsable in the terminal and, through its list dialog, the GUI. The GUI's File menu and its preview key come with its native menus (ADR-0046); until then its Say Status in a list repeats the introduction, and its command palette opens the browser.
- Batch conversion and audio export (W6k, W6v) choose their folders through `choose_folder`; settings import can use `choose_file`. Each calls it from its registered handler and returns its effects.
- Folder listings are read on the input thread, bounded by the entry limit and the 500 folders counted. A folder on a slow network share can pause the list; moving the listing to a helper thread, as the preview does, is the next step if that is heard.
- A member path is only as stable as the archive: if `course.zip` is replaced by a different archive, notes kept under `course.zip!week1/notes.md` are found again by the relocation of positions, as for any changed file.
- JSON-RPC's `list_key` takes `details`, `choose_here`, `sort`, and `show_all`.

## See also

- [ADR-0043: Menus and the palette from one model](0043-menus-and-the-palette-from-one-model.md): `register_handler`, interface announcements, and the dialog generation.
- [ADR-0026: OCR, and formats for students](0026-ocr-and-student-formats.md), for archives and their limits.
- [Reading and moving around](../reading.md#from-the-file-browser-file-browse-files), the user's guide.
