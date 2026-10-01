# ADR-0049: Sync beyond the place

- Status: accepted
- Date: 2026-09-30 (Wednesday, September 30, 2026)
- Builds on: [ADR-0024](0024-app-core-for-the-gui.md) (one app core and its background writer), [ADR-0043](0043-menus-and-the-palette-from-one-model.md) (menus, the palette, and announcement levels), and [ADR-0030](0030-interface-translations.md) (messages in six languages)

## Context

A student reads on more than one computer: a laptop at home, a desktop in a lab, a computer borrowed for an exam. What they made while reading (notes, highlights, bookmarks, the place in each document, their word list and their settings) should follow them, without an account, a server, or a network connection inside textweaver.

Today only the reading place travels, and only inside a library folder. `textweaver_store::sync` keeps one sidecar per library folder, `<folder>/.textweaver/progress.json`, keyed by each document's path inside the folder and merged by Star's rules (`reading.sync_conflict_policy`: newest, highest progress, or manual). `LibrarySync` mirrors the position there when it is saved. The sidecar also carries a `_meta` entry with a note count and some statistics, but not the notes themselves. A document outside a library folder never syncs.

Extending the sidecar to everything else would carry its problems with it:

1. **"Ask" never asks.** The manual policy keeps this computer's place and only says that another one differs.
2. **Conflicts are logged, not reported.** When the sidecar is written, the conflicts `flush` returns go to the log (`crates/textweaver-app/src/writer.rs`, `Job::SyncFlush`) and the reader never hears them.
3. **Deletes come back.** Notes and highlights merge as a union by id, with no record of a deletion, so a note deleted on one computer returns from the other.
4. **Bookmarks have no id.** Two computers can each make a different `mark1`, and a merge by name would lose one.
5. **Note ids are short.** They are 8 hex digits (32 bits), checked for uniqueness only on the computer that made them.
6. **Keys depend on the full path.** `DocKey::for_path` hashes the resolved path, and so does `stats.json`, so one document has a different key on each computer. The vault's `document_id` and `source` come from the full path too, so a vault export is tied to the computer that wrote it.
7. **Two writers, one file.** Every computer rewrites the same `progress.json`. A sync service that sees two versions of one file keeps one and renames the other ("progress (conflicted copy).json"), and textweaver never reads the renamed one.

## Decision

### A folder the owner chooses

Sync goes through a folder the owner chooses (`[sync] folder`): a folder kept in step by Syncthing, OneDrive, or Dropbox, or a USB stick carried to a lab computer. textweaver reads and writes files there and nothing else: no account, no server, no network code. Moving the files between computers is the sync service's job, or the owner's.

Inside the chosen folder:

- `textweaver-sync/`
  - `format.json`
  - `devices/<device-id>/`, one folder per computer:
    - `device.json`
    - `docs/<sync-id>.json`, one file per document
    - `library.json`, `settings.json`, `profiles.json`, `keymap.json`
    - `words.json`, `glossary.json`, `voices.json`

In that layout:

- `format.json` holds the format's version. `device.json` holds the computer's name, chosen by the owner, and its install token (below).
- Every file name is a random id or a fixed name. No document title, path, or computer name is ever part of a file name.

### Each computer writes only its own files

A computer writes only under `devices/<its own id>/`. It reads every other computer's folder and never changes or deletes anything in it. Since no two computers write the same file, a sync service never has two versions of one file to choose between, and no conflicted copies appear (problem 7).

Each file is written whole and replaced atomically: written to a temporary file in the same folder (a name starting with `.` and ending in `.tmp`, which readers skip), then renamed over the old one. Nothing is ever appended. A computer writes its **full merged view**, everything it knows from every computer, not only its own edits, so its files are a complete copy, and merging the same files twice changes nothing.

textweaver never deletes in the sync folder. A computer that is retired leaves its folder behind; its data is already in every other computer's merged view, and the owner can remove the folder by hand.

### Format versions, damaged files, and a newer format

- `format.json` carries a whole-number version. A change that only adds fields keeps the version; readers ignore fields they do not know.
- A file that cannot be read (cut short by a sync still in progress, damaged, or not JSON) is skipped for that merge and reported once in the sync status. It is tried again on the next change.
- If `format.json` says a newer version than this textweaver knows, sync on this computer becomes **read only**: it writes nothing to the folder, merges what it can read, and the status line says so ("Sync: newer version, read only"). An older textweaver cannot damage what a newer one wrote.
- The local state folder is backed up once before the first merge, so turning sync on can be undone.

### Device ids and the install marker

Each computer has a random 128-bit device id, made when sync is first set up, and a random install token. Both are in `device.json` in the sync folder and in an install marker in the local state folder. The marker also holds a fingerprint of where the state folder is on this computer, kept locally and never written to the sync folder.

When someone copies a whole state folder to a new computer (a common way to move settings), the copy would claim the old computer's id, and the two would overwrite each other's files. textweaver catches this two ways: the marker's fingerprint does not match the folder it is in, or the sync folder shows this device id with a different install token. Either way the computer takes a fresh device id and keeps its data; nothing is lost, and the old computer's folder is untouched.

### A hybrid logical clock

Every change carries a hybrid logical clock stamp: the wall time in milliseconds, a counter, and the device id for ties. A computer's clock never goes backward: it is the larger of its own wall time and the newest stamp it has seen, plus one tick. So an edit made after reading another computer's files always sorts after them, even when the lab computer's clock is an hour slow, and two stamps are never equal.

A clock far ahead is the known risk: an edit stamped a year in the future keeps winning until real time passes it. The sync status names a computer whose stamps are more than a day ahead of this one's wall time, so the owner can fix its clock.

### Recognizing the same document

Each document gets a random 128-bit **sync id**, and records are keyed by it, never by path (problem 6). A local `sync-ids.json` in the state folder maps this computer's path keys to sync ids. When a document opens and has no sync id yet, one is found, in order, by:

1. the file's exact contents (SHA-256);
2. a hash of the text as textweaver reads it, so the same book converted or saved differently still matches;
3. the library folder's id plus the path inside it. Each library folder gets a small id file, so the same folder at `D:\Books` and at `/home/student/Books` is recognized as one;
4. a DOI or ISBN, which is only **suggested**, never matched alone, because two chapters of one book share an ISBN: "This may be Cells from the laptop, with 4 notes. Use them? Yes or no."

Failing all four, the document gets a new id. After an edit the id stays, the record publishes the new content hash, and `relocate.rs` moves notes, highlights, and bookmarks by their anchors and text stamps, as it does for any outside edit. Hashing runs on the background writer, never on the input thread.

### One record per document, and a merge rule per kind of data

Each `docs/<sync-id>.json` holds one document's content hashes, its title, every computer's place in it, its bookmarks, notes, and highlights with their deletion records, and each computer's reading statistics for it. The merge rules are these; each is commutative, associative, and idempotent, so the order files arrive in and how often they are read never change the result.

- **Place.** Each computer has its own place, and none overwrites another's. Which one to resume at follows `[sync] position_policy` (newest, furthest, or ask), which replaces `reading.sync_conflict_policy` and is migrated from it. "Ask" now asks, naming the computer: "Resume at the laptop's place, 42 percent? Yes or no." (problem 1). A place that arrives while reading never moves the cursor; it is offered through "Go to another computer's place".
- **Bookmarks.** Every bookmark gets a stable id (problem 4); existing ones are migrated. Newest wins by id. A bookmark whose name is already taken by a different bookmark from another computer is renamed with that computer's name ("mark1, lab").
- **Notes.** Note ids grow to 64 bits; old 32-bit ids are kept as they are (problem 5). For a note's text, **the newest edit wins, and the owner is told which note was replaced**: "Cells: a note was replaced by the laptop's newer edit." The replaced text is kept in a local backup of replaced notes in the state folder, so nothing is lost silently. The backup never syncs. There is no letter-by-letter merge of two texts.
- **Highlights.** Newest wins by id, as for bookmarks.
- **Tags and links on notes.** Sets: adding wins, and a removal is recorded so it is not undone by a computer that has not seen it.
- **Deletions.** Deleting a note, a highlight, or a bookmark leaves a deletion record with its stamp, instead of removing the item from the merge (problem 3). A deletion wins over the edits it is newer than. An edit newer than the deletion brings the item back, and the owner hears so.
- **Reading statistics.** Each computer adds only to its own counters; the totals are their sum. Statistics never conflict.
- **Settings, key overrides, glossary, pronunciations.** Newest wins, key by key.
- **Word list and favorite voices.** Sets, with removals recorded.
- **Library details** (title, author, DOI, ISBN, format, first added), by sync id: newest wins per field; "first added" keeps the earliest.
- **Profiles.** The definitions sync, newest wins per profile; which profile is active on a computer does not.

Conflicts found when merging are reported through the sync status and the announcements below, never only logged (problem 2).

### What syncs, and what never does

Each group has its own switch under `[sync]`: documents (places, bookmarks, notes, highlights, statistics), settings, profiles, key overrides, word lists (the word list, glossary, and pronunciations), favorite voices, and library details. Turning a group off stops that group only.

- **Settings** sync only if they are portable: rate, punctuation, verbosity, capitals, reading aids, highlight, theme, Braille and math codes, interface language, speed presets, and the announcement level. Every setting is marked portable or machine, and a test fails when one is neither.
- **Machine settings never sync:** the speech engine, the voice, the access mode, the NVDA or JAWS key preset, and every path. A favorite voice that is not installed on this computer is listed as "not on this computer".
- **Key overrides** are labeled with their system; a Mac override is kept on Windows but not applied there.
- **Never synced:** recent files (paths differ, and file names reveal what was read on a shared lab computer; a "Continue reading" list built from synced places replaces them), navigation history, recovery snapshots and unsaved text, caches, logs, the documents themselves, and the local backup of replaced notes.

### Privacy

- No user name, host name, account name, or full path is ever written to the sync folder. Computers are named by the owner ("laptop", "lab"); the default is "Computer 1", never the host name.
- Records carry short excerpts that do travel: the 40 characters of a bookmark's or a place's anchor, a note's text, and a highlight's text. The owner chooses where the folder lives knowing this.
- The sync folder is not encrypted (below).
- A test scans the sync folder after every sync scenario for user names, host names, and full paths.

### The old sidecar

The library sidecar stays as one more source to read, so places written by an older textweaver are still honored. New places are written to the sync folder, not the sidecar.

### Accessibility

- The status line starts with "Sync": "Sync: up to date", "Sync: folder missing, saving here", "Sync: newer version, read only".
- Messages put their meaning first, fit the 40 cells of a Braille line where they can, and never rely on color. They are in all six catalogs.
- Sync announcements go through ADR-0043's levels and are never said while reading; they wait for a pause.
- Commands live in Tools, Sync, and in the palette: Set up sync (the folder through the file browser, naming this computer, choosing groups), Sync status (the one sync command with a default key), Sync now, Go to another computer's place, and Stop syncing on this computer. The CLI has `tw sync setup`, `tw sync status`, and `tw sync now`, each with `--json`.

## Alternatives set aside

- **Git.** A repository carries history and merges, but it needs a git installation, credentials for any remote, and a merge that stops on a conflict and asks for text editing no reader should face. It also keeps every past version of every note, which works against deleting something private.
- **The Obsidian vault.** The vault export is for reading notes in Obsidian, one file per document in Markdown. It holds no places, bookmarks, statistics, or settings, and a two-way merge through hand-editable Markdown would lose data whenever a note was reformatted. The vault stays an export; its `document_id` can move to the sync id later.
- **Direct network sync** between computers. It would need discovery, a listening port, firewall prompts on every computer, pairing, and a secure channel, all inside a reader, and it fails the lab computer and the USB stick, where the two computers are never on at the same time.
- **Encryption at rest.** Encrypting the sync folder (with argon2 and chacha20poly1305) was proposed for cloud folders and USB sticks. The owner decided against it: the folder is one the owner chooses, and a passphrase adds a thing to lose and a prompt on every computer. There is no encryption and no sealed-format flag. The privacy rules above hold regardless.
- **"Keep both" for notes.** Keeping both texts when two computers edit one note offline, and asking at the next open, was the first design. The owner chose the newest edit instead, with a message and a local backup: a review list of conflicts is one more thing to go through by ear, and the backup keeps the older text safe.
- **Extending the sidecar.** One shared file per library folder, written by every computer, produces conflicted copies under sync services, covers only library folders, and is keyed by path.

## Consequences

- Notes, highlights, bookmarks, places, statistics, portable settings, profiles, key overrides, word lists, favorite voices, and library details travel between computers through any folder that syncs, including a USB stick. Documents anywhere on disk sync, not only those in library folders.
- Because each computer writes only its own files, sync services never make conflicted copies, and a damaged file on one computer cannot corrupt another computer's data.
- The folder grows with the number of computers: each keeps a full copy. For a student's library this is kilobytes to a few megabytes per computer. Merge cost grows with the library; merging is by document, and only changed files are read again.
- A note edited on two computers offline keeps only the newest text in the synced notes; the older one is in the local backup of the computer that had it, and the owner hears which note was replaced.
- Deletion records stay in the files, so a deleted note's id and stamp remain there, though not its text.
- The state format changes (bookmark ids, deletion records, 64-bit note ids, sync ids). That work is scheduled before the state format freezes at the final alpha.
- A computer with a clock far in the future can win edits it should not, until its clock is fixed; the sync status names it.
- The sync folder is readable by anyone who can read the folder; the owner chooses where it lives.
- Sync depends on the folder being kept in step by something else. textweaver cannot tell a sync service that is paused from one that is done.

## Update: Wave 7 (Wednesday, September 30, 2026)

- **Profiles** publish only their portable settings. The machine settings a profile holds (the voice, the engine, the access mode) stay on the computer that saved them, and are kept when another computer's version of the profile arrives.
- **Two ids for one document**, made before the computers ever synced, are folded together: identifying a document picks the smallest id among the records that share its content hash, text hash, or library key. The computer whose id lost merges its record into the winner's and marks the old record `folded_into`, a field added without raising the format; readers count a folded record as part of its winner. A DOI or an ISBN still never matches on its own.
- **The crash window.** What a merge sends the app to apply is saved in the data folder (`sync-pending.json`, never synced) before the merged view is published, for documents and for the other groups alike. After a crash, the next session sends those arrivals again instead of taking the app's older version for a new edit.

## See also

- [ADR-0024: App core for the GUI](0024-app-core-for-the-gui.md), for the background writer
- [ADR-0043: Menus and the palette from one model](0043-menus-and-the-palette-from-one-model.md), for announcement levels
- [ADR-0045: A file browser on the list model](0045-a-file-browser-on-the-list-model.md), for choosing the folder
- [Library guide](../library.md)
- [Notes guide](../notes.md)
