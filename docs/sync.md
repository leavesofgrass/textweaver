# Syncing between computers

This guide covers syncing your notes, highlights, study cards, bookmarks, reading places, reading statistics, settings, profiles, key overrides, word list, glossary, pronunciations, and favorite voices between computers, through a folder you choose. There is no account, no server, and no network code inside textweaver: textweaver only reads and writes files in the folder, and something else, such as Syncthing or a USB stick you carry, moves them between computers.

Documents sync wherever they are on disk, not only inside a library folder: textweaver recognizes the same document on each computer by its contents, even under a different name or path.

## What syncs, and what never does

Each group has its own switch, so you can turn one off without turning off the rest. Turning a group off stops that group only: this computer neither sends nor takes its items.

- **Places**: where you are in each document, one place per computer.
- **Notes**, and with them your [study cards](notes.md#study-with-cards) and every grade you gave them.
- **Highlights.**
- **Bookmarks.**
- **Statistics**: each computer's reading time and sessions for each document; the totals are their sum, in the statistics list and `tw stats` (below).
- **Settings**: the portable ones, which are about you as a reader rather than the computer: rate, punctuation, verbosity, capitals, the reading aids, the highlight, the theme and colors, the Braille and math codes, the interface language, speed presets, and the announcement level. The newest change to each setting wins. The [settings reference](settings-reference.md) says for every setting whether it syncs.
- **Profiles**: each profile's portable settings. The voice, the speech engine, the access mode, and the other machine settings a profile holds stay on the computer that saved them: a profile that arrives keeps this computer's own voice, engine, and access mode for it, and a profile new to this computer uses the ones already set here. Which profile is in use stays on each computer.
- **Key overrides**: your `keymap.toml`, each override labeled with the system it was made on. Windows and Linux share their keys, so an override made on one is used on the other. A Mac's overrides are kept but not used on Windows or Linux, and the reverse, so a Mac key never lands on a Windows keyboard.
- **Word list**: the words you added to the spelling list. Adding a word wins; removing one is recorded, so it does not come back from a computer that had not heard.
- **Glossary and pronunciations**: your glossary's entries and your pronunciation list, the newest change winning word by word. A glossary written as text keeps its comments and order; a changed term is replaced where it was.
- **Favorite voices**: a voice you starred on one computer is a favorite on the others. One that is not installed on this computer stays on the list, and Choose voice shows it last as "not on this computer"; Enter on it only says so, and Space takes it off your favorites.

Library details travel with each document, as part of how it is recognized: its title, author, DOI, ISBN, and kind of file, and when it was first added to a library. They have no switch of their own; a title or an author is sent only when the document states it (below).

Some things never sync, because they are tied to one machine or would expose what you read:

- **Machine settings**: the speech engine, the voice, the volume, the access mode, your NVDA or JAWS key preset, the keyboard layout, the wrap width, the author name written into new documents, every path (library folders, the glossary file, engine libraries, the sync folder), and the sync settings themselves.
- **Recent files.** The paths differ from computer to computer, and the file names themselves would reveal what was read on a shared lab computer. **Continue reading** (below) takes their place: it is built from the synced places, and lists only documents found on this computer.
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

### Study cards and their grades

Study cards travel with the Notes group. A card's question, answer, and direction follow the newest edit, as a note does, but its grades are never replaced: when the same card was graded on two computers while they were apart, the next sync keeps every grade from both, in the order they were given, and each computer works out the same schedule from them. Nothing is spoken when cards arrive; the next study session and the Cards list simply include them. Taking a card out of the Cards list takes it out on the other computers too; making cards again from the same notes afterwards brings it back, without the old grades.

The cards were added to the sync record without changing its format, so a sync folder written by an earlier version is read as before, and an earlier version reading a folder written by this one ignores the cards.

### Deleted notes, and edits that bring them back

Deleting a note, a highlight, or a bookmark deletes it on the other computers too, and it does not come back from a computer that had not heard of the deletion yet. An edit made on another computer after the deletion does bring it back, and you hear so: "Cells: a deleted note came back, edited on lab."

### A document that may be the same book

Two copies of a book with the same contents are recognized at once. If two computers each opened the book before they ever synced, each gave it an id of its own; once their files meet, the next time the book opens the two are joined into one, with both computers' notes, highlights, bookmarks, and places. Nothing is asked, and nothing is lost. A document that only shares a DOI or an ISBN with one on another computer, such as two chapters of one book, is never matched on its own; textweaver asks: "This may be Cells from laptop, with 4 notes. Use them? Y or N". Y shares that document's notes and places; N keeps the two apart and is remembered.

### Problems

The status line always starts with "Sync", so it reads cleanly on a Braille display:

- "Sync: up to date"
- "Sync: folder missing, saving here": the folder is not there, for example a USB stick that is out. Everything is saved on this computer and synced when the folder is back.
- "Sync: newer format, read only": the folder was written by a newer textweaver. This computer still reads what it can, and writes nothing, so it can never damage what the newer one wrote.
- "Sync: lab's clock is ahead": another computer's clock is more than a day ahead, which would let its edits win when they should not. Set that computer's clock.
- "Sync: 1 damaged file skipped": a file could not be read, perhaps cut short by a sync still in progress. It is said once a session, and tried again at the next change.
- "Sync: cannot write, saving here": writing to the folder failed; the reason is said once, as an error.

If textweaver stops after taking in another computer's changes and before saving them here, for example in a power cut, nothing is lost: the changes it was about to save are kept on this computer, in `sync-pending.json` beside the reading state, and taken in again at the next start, and this computer's older version never wins over them. That file never syncs.

Before the first merge, textweaver copies this computer's reading state to a folder named `state-before-sync` beside it, once, so turning sync on can be undone.

When two library-folder progress files disagree as they are written (the older, library-only place sync), that is said too: "Sync: 2 library places differed."

## Continue reading

**Continue reading**, in the File menu under Library and in the command palette, lists the documents on this computer with a reading place from any computer, newest first, one row each, meaning first: "Cells, 42 percent, laptop, 2 hours ago". Enter opens the document, which resumes by `position_policy`. Only documents found on this computer are listed: opened here before, in a library folder, or among the recent files. A document read only on the laptop, and not on this computer at all, is left out. With the places group off, only this computer's own places are used. [The library guide](library.md#continue-reading) has the details, and `tw library continue` prints the same list.

## Library details and search

With sync on, the library's filter and `tw library search` also know what your other computers learned about a document: its title, author, DOI, and ISBN. So a paper opened on the laptop is found on the lab computer by its DOI, even before it is opened there. The document is found here when it was opened here before, when its library folder is itself synced between the computers (its `.textweaver/library-id.json` travels with it), or when `tw library search` has read its text. When two computers know different details, the newest wins, detail by detail; the date a document was first added keeps the earliest.

Details you type yourself ([Edit a document's details](library.md#edit-a-documents-details), F2 in the library list or `tw library edit`) travel the same way, and they win over what the document states, even when another computer opens the document later and sends its own title. When you edit the same detail on two computers, the newest edit wins; clearing an edit travels too, and the document's own value shows again.

## Reading statistics from every computer

With the statistics group on, the statistics list (File, Reading statistics) and `tw stats` add every computer's reading of a document together: the time read aloud and the sessions are summed, and the furthest point is the furthest any computer reached. Each computer only ever adds to its own counts, so nothing is counted twice and nothing conflicts.

To see each computer's share, choose "Each computer: hidden. Enter shows it." near the end of the statistics list; under each document read on more than one computer you then hear a line per computer, such as "laptop: 20 minutes and 5 seconds, 2 sessions". On the command line, `tw stats --by-computer` prints the same lines. A document read only on another computer is listed too, by the title that computer knew.

If the sync folder is slow to read, for example on a network folder, the list does not wait: it shows this computer's reading, and says "Other computers skipped: folder slow."

## On the command line

The same actions are on the command line, each with `--json` for scripts, and `--home DIR` to use another set of files:

- `tw sync setup --folder DIR [--name NAME] [--groups places,notes,highlights,bookmarks,statistics,settings,profiles,key_overrides,words,glossary,favorite_voices]`
- `tw sync status`: the status line, this computer, and the others.
- `tw sync now`: merges every document this computer knows, and the settings and word lists, and says how many documents took changes and how many settings changed (`settings_changes` in `--json`).
- `tw sync stop`: stops syncing on this computer, as "Stop syncing on this computer" in the reader does. The sync folder is left as it is.

Three other commands read the sync folder too, and change nothing in it:

- `tw library continue`: Continue reading, newest first.
- `tw library search WORDS`: finds documents by what your other computers know about them too.
- `tw stats --by-computer`: every computer's reading, summed, with a line per computer.

## Privacy

No name, path, or computer name is ever written to the sync folder. Documents, computers, and records are identified by random ids, never by their titles as file names or the names you gave them on disk; computers are identified only by the name you chose for them ("laptop", "lab"). A document's title travels only when the document states one itself, never one made from its file name, and an author only when the document names one that is not this computer's user or computer name (a Word file's author is often the account's name). Records do carry short pieces of text: the words around a bookmark or a place, a note's text, a highlight's text, and a document's title, author, DOI, and ISBN.

The folder is not encrypted, so choose a folder you trust the way you would trust any other place that can hold your notes and highlights in plain text.

## See also

- [The library](library.md#the-older-place-sync-through-a-library-folder): the older reading-place sync inside a shared library folder, which is still read.
- [The library: Continue reading](library.md#continue-reading).
- [Troubleshooting](troubleshooting.md#sync-folder-missing): the sync folder missing, and a folder in a newer format.
- [Bookmarks, notes, and highlights](notes.md): what a note, highlight, and bookmark hold.
- [ADR-0049: Sync beyond the place](adr/0049-sync-beyond-the-place.md): the full design, including the folder layout, the merge rules, and what was set aside.
- [Documentation index](README.md)
