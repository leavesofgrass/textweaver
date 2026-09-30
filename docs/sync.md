# Syncing between computers

This guide covers syncing your notes, highlights, bookmarks, reading places, reading statistics, settings, profiles, key overrides, word list, glossary, pronunciations, and favorite voices between computers, through a folder you choose. There is no account, no server, and no network code inside textweaver: textweaver only reads and writes files in the folder, and something else, such as Syncthing or a USB stick you carry, moves them between computers.

Documents sync wherever they are on disk, not only inside a library folder: textweaver recognizes the same document on each computer by its contents, even under a different name or path.

## What syncs, and what never does

Each group has its own switch, so you can turn one off without turning off the rest. Turning a group off stops that group only: this computer neither sends nor takes its items.

- **Places**: where you are in each document, one place per computer.
- **Notes.**
- **Highlights.**
- **Bookmarks.**
- **Statistics**: each computer's reading time and sessions for each document; the totals are their sum.
- **Settings**: the portable ones, which are about you as a reader rather than the computer: rate, punctuation, verbosity, capitals, the reading aids, the highlight, the theme and colors, the Braille and math codes, the interface language, speed presets, and the announcement level. The newest change to each setting wins. The [settings reference](settings-reference.md) says for every setting whether it syncs.
- **Profiles**: each profile's settings. Which profile is in use stays on each computer.
- **Key overrides**: your `keymap.toml`, each override labeled with the system it was made on. Windows and Linux share their keys, so an override made on one is used on the other. A Mac's overrides are kept but not used on Windows or Linux, and the reverse, so a Mac key never lands on a Windows keyboard.
- **Word list**: the words you added to the spelling list. Adding a word wins; removing one is recorded, so it does not come back from a computer that had not heard.
- **Glossary and pronunciations**: your glossary's entries and your pronunciation list, the newest change winning word by word. A glossary written as text keeps its comments and order; a changed term is replaced where it was.
- **Favorite voices**: a voice you starred on one computer is a favorite on the others. One that is not installed on this computer stays on the list, and Choose voice shows it last as "not on this computer"; Enter on it only says so, and Space takes it off your favorites.

**Coming in this wave: not built yet.** Library details (title, author, DOI, ISBN) are planned as a group of their own.

Some things never sync, because they are tied to one machine or would expose what you read:

- **Machine settings**: the speech engine, the voice, the volume, the access mode, your NVDA or JAWS key preset, the keyboard layout, the wrap width, the author name written into new documents, every path (library folders, the glossary file, engine libraries, the sync folder), and the sync settings themselves.
- **Recent files.** The paths differ from computer to computer, and the file names themselves would reveal what was read on a shared lab computer.
- **Navigation history, recovery snapshots and unsaved text, caches, and logs.**
- **The documents themselves.** Only what you did with them syncs.
- **The local backup of replaced notes** (below). It stays on the computer that made it.

## Choosing a folder

Sync works through any ordinary folder: a folder kept in step by Syncthing between your own computers, a folder on a USB stick you carry to a lab computer, or a folder kept in step by OneDrive, Dropbox, iCloud, or anything else. textweaver only reads and writes files there, the same way it reads and writes any folder on disk.

Two things follow from this:

- If the folder is never in two places at once, for example a USB stick you move by hand, sync still works: each computer picks up what changed the next time it can read the stick. While the stick is out, everything is saved on this computer as usual, and the status says "Sync: folder missing, saving here".
- textweaver cannot tell a sync service that is paused from one that has finished. If the folder does not look like it is catching up, check the sync service itself, not textweaver.

textweaver looks at the other computers' files for the open document, and for the settings and word lists, every few seconds, and sends this computer's changes a moment after you make them, when you leave a document, and when you quit.

## Setting it up

Sync is set up, checked, and driven from **Tools, Sync** in the menus (F10 in the terminal) and from the command palette (F2):

- **Set up sync**: choose the folder in the file browser, name this computer, and choose which groups sync: places, notes, highlights, bookmarks, statistics, settings, profiles, key overrides, word list, glossary and pronunciations, and favorite voices. Enter on a group turns it on or off; "Start syncing", after the groups, finishes.
- **Sync status** (Shift+F5): says how sync stands, this computer's name, and the other computers' names, and how many key overrides from the other kind of system are kept but not used here ("Mac key overrides: 2, kept, not used here.").
- **Sync now**: sends this computer's changes and takes the other computers' for every document this computer knows, not only the open one.
- **Go to another computer's place**: lists the other computers' places in this document, such as "lab, 42 percent"; Enter goes there.
- **Replaced notes**: lists the notes in this document that another computer's newer edit replaced, and the ones another computer deleted; Enter puts one back.
- **Stop syncing on this computer**: turns sync off here. The sync folder is left as it is, and the other computers go on.

Sync status is the one sync command with a default key, Shift+F5, in both the terminal and the window, on every system. The others have no keys; you can give them keys in `keymap.toml` (see [the keyboard reference](keyboard.md)).

### Naming each computer

Every computer you sync gets a name you choose, such as "laptop" or "lab", instead of its real computer name. The name suggested is "Computer 1", "Computer 2", and so on, never the machine's own name, which textweaver refuses. A name is at most 40 characters, so it fits a Braille line. It is what you hear in every message about another computer's place, edits, or clock.

### The settings

Set up sync writes these under `[sync]` in `settings.toml`; you can also change them on the settings screen, in the Sync section:

- `enabled`: sync on this computer.
- `folder`: the sync folder.
- `device_name`: this computer's name; empty uses "Computer 1", "Computer 2", and so on.
- `places`, `notes`, `highlights`, `bookmarks`, `statistics`, `settings`, `profiles`, `key_overrides`, `words`, `glossary`, `favorite_voices`: one switch per group, all on by default. Turning one off stops that group only.
- `position_policy`: which place a document opens at when another computer has one too: `newest` (the default), `furthest`, or `ask`. It replaces `[reading] sync_conflict_policy`; a value you set there moves here on its own (`highest_progress` becomes `furthest`, `manual` becomes `ask`), and it also decides between places in a library folder's old progress file.

## What you hear

Sync messages go through your interface announcement level (Ctrl+F9; see [reading](reading.md)). A change that arrives is routine; a replaced note, a note that came back, and a place resumed from another computer are results; a folder that cannot be written is an error. Nothing about sync is said while textweaver reads aloud: the messages wait, and are said when reading pauses or stops.

### Settings from another computer

A setting, profile, key override, word, glossary entry, pronunciation, or favorite voice that changes on another computer is taken in quietly, never while textweaver reads aloud: it waits for the pause, so the voice never changes mid-sentence. Then you hear one short summary: "Settings: 3 changes from laptop." The changes take effect at once, and are saved. Taking them in never sends them back out as this computer's own change.

A setting you change on this computer while another computer changed the same one goes to the newer change.

### A place from another computer

When a document opens, `position_policy` decides:

- **Newest** or **furthest**: if another computer's place is newer (or further), the document opens there, and you hear which computer it came from: "Cells: resumed at 42 percent, from laptop."
- **Ask**: textweaver asks, naming the computer: "lab at 42 percent. Go there? Y or N". Y goes there; N keeps this computer's place.

Your place never moves on its own after that. A place that arrives while you read is only offered, once: "laptop's place: 42 percent." Go to another computer's place takes you there when you want.

### A note replaced by a newer edit

When the same note was edited on two computers while they were apart, the newest edit wins, and you are told which note it replaced: "Cells: a note was replaced by laptop's newer edit." There is no letter-by-letter merge of the two texts, and no list of conflicts to go through by ear.

The text that lost is not discarded: it goes into this computer's backup of replaced notes, which keeps the last 20 per document. Tools, Sync, Replaced notes lists them ("First words, replaced by laptop"), and Enter puts one back as a new edit, which then wins on the other computers too. The text it replaces is kept in turn, so putting one back can be undone the same way.

### Deleted notes, and edits that bring them back

Deleting a note, a highlight, or a bookmark deletes it on the other computers too, and it does not come back from a computer that had not heard of the deletion yet. An edit made on another computer after the deletion does bring it back, and you hear so: "Cells: a deleted note came back, edited on lab."

### A document that may be the same book

Two copies of a book with the same contents are recognized at once. A document that only shares a DOI or an ISBN with one on another computer, such as two chapters of one book, is never matched on its own; textweaver asks: "This may be Cells from laptop, with 4 notes. Use them? Y or N". Y shares that document's notes and places; N keeps the two apart and is remembered.

### Problems

The status line always starts with "Sync", so it reads cleanly on a Braille display:

- "Sync: up to date"
- "Sync: folder missing, saving here": the folder is not there, for example a USB stick that is out. Everything is saved on this computer and synced when the folder is back.
- "Sync: newer format, read only": the folder was written by a newer textweaver. This computer still reads what it can, and writes nothing, so it can never damage what the newer one wrote.
- "Sync: lab's clock is ahead": another computer's clock is more than a day ahead, which would let its edits win when they should not. Set that computer's clock.
- "Sync: 1 damaged file skipped": a file could not be read, perhaps cut short by a sync still in progress. It is said once a session, and tried again at the next change.
- "Sync: cannot write, saving here": writing to the folder failed; the reason is said once, as an error.

Before the first merge, textweaver copies this computer's reading state to a folder named `state-before-sync` beside it, once, so turning sync on can be undone.

When two library-folder progress files disagree as they are written (the older, library-only place sync), that is said too: "Sync: 2 library places differed."

## On the command line

The same actions are on the command line, each with `--json` for scripts, and `--home DIR` to use another set of files:

- `tw sync setup --folder DIR [--name NAME] [--groups places,notes,highlights,bookmarks,statistics,settings,profiles,key_overrides,words,glossary,favorite_voices]`
- `tw sync status`: the status line, this computer, and the others.
- `tw sync now`: merges every document this computer knows, and the settings and word lists, and says how many documents took changes and how many settings changed (`settings_changes` in `--json`).

## Privacy

No name, path, or computer name is ever written to the sync folder. Documents, computers, and records are identified by random ids, never by their titles as file names or the names you gave them on disk; computers are identified only by the name you chose for them ("laptop", "lab"). A document's title travels only when the document states one itself, never one made from its file name. Records do carry short pieces of text: the words around a bookmark or a place, a note's text, and a highlight's text.

The folder is not encrypted, so choose a folder you trust the way you would trust any other place that can hold your notes and highlights in plain text.

## See also

- [The library](library.md#sync-your-place-between-computers): the older reading-place sync inside a shared library folder, which is still read.
- [Bookmarks, notes, and highlights](notes.md): what a note, highlight, and bookmark hold.
- [ADR-0049: Sync beyond the place](adr/0049-sync-beyond-the-place.md): the full design, including the folder layout, the merge rules, and what was set aside.
- [Documentation index](README.md)
