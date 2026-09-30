# Syncing between computers

This guide covers syncing your notes, highlights, bookmarks, reading places, and portable settings between computers, through a folder you choose. There is no account, no server, and no network code inside textweaver: textweaver only reads and writes files in the folder, and something else, such as Syncthing or a USB stick you carry, moves them between computers.

**Coming in this wave: not in 0.1.0-alpha.6.** The parts of textweaver that turn sync on, choose its folder, and show its status are still being built. This guide describes the whole design as planned, so you know what is coming; each section below that is not built yet is marked the same way. What already works today is the reading-place sync inside a shared library folder, covered in [the library guide](library.md#sync-your-place-between-computers).

## What syncs, and what never does

**Coming in this wave: not in 0.1.0-alpha.6.**

Sync is grouped, and each group has its own switch, so you can turn one off without turning off the rest:

- **Documents**: your reading place in each document, bookmarks, notes, highlights, and reading statistics.
- **Settings**, but only the ones that make sense to carry to another computer: rate, punctuation, verbosity, capitals, reading aids, highlight, theme, Braille and math codes, interface language, speed presets, and the announcement level.
- **Profiles**: the profiles themselves sync; which one is active on a given computer does not.
- **Key overrides**: kept, and labeled with the system they were made on.
- **Word lists**: your word list, glossary, and pronunciations.
- **Favorite voices.**
- **Library details**: a document's title, author, DOI, ISBN, format, and when you first added it.

Some things never sync, because they are tied to one machine or would expose what you read:

- **Machine settings**: the speech engine, the voice, the access mode, your NVDA or JAWS key preset, and every path. A favorite voice that is not installed on this computer is listed as "not on this computer" rather than silently dropped.
- **Recent files.** The paths differ from computer to computer, and the file names themselves would reveal what was read on a shared lab computer. A "Continue reading" list, built from the places that do sync, takes their place.
- **Navigation history, recovery snapshots and unsaved text, caches, and logs.**
- **The documents themselves.** Only what you did with them syncs.
- **The local backup of replaced notes** (below). It stays on the computer that made it.

## Choosing a folder

**Coming in this wave: not in 0.1.0-alpha.6.**

Sync works through any ordinary folder: point it at a folder kept in step by Syncthing between your own computers, a folder on a USB stick you carry to a lab computer, or a folder kept in step by OneDrive, Dropbox, iCloud, or anything else. textweaver does not care which; it only reads and writes files there, the same way it reads and writes any folder on disk.

Two things follow from this:

- If the folder is never in two places at once, for example a USB stick you move by hand, sync still works: you carry the folder, and each computer picks up what changed the next time it reads the stick.
- textweaver cannot tell a sync service that is paused from one that has finished. If the folder does not look like it is catching up, check the sync service itself, not textweaver.

## Naming each computer

**Coming in this wave: not in 0.1.0-alpha.6.**

Every computer you sync gets a name you choose, such as "laptop" or "lab", instead of its real computer name. The default is "Computer 1", "Computer 2", and so on, never the machine's own name. This name is what you hear in every message about another computer's place, edits, or clock, and it is the only thing that identifies a computer in the sync folder.

## What you hear

**Coming in this wave: not in 0.1.0-alpha.6.**

### A place arriving from another computer

Reading your place never moves while you read: a place that arrives from another computer is never applied out from under you. Instead, it is offered, by name: "Resume at the laptop's place, 42 percent? Yes or no." Whether textweaver offers the newest place, the furthest one, or always asks depends on the `position_policy` setting, which replaces today's `reading.sync_conflict_policy`.

### A note replaced by a newer edit

When the same note was edited on two computers while they were apart, the newest edit wins, and you are told which note it replaced: "Cells: a note was replaced by the laptop's newer edit." There is no letter-by-letter merge of the two texts, and no list of conflicts to review by ear; you hear it once, at the moment the edit arrives.

### Where replaced notes are kept

The text that lost is not discarded. It goes into a local backup of replaced notes, kept in your state folder on the computer that had it. This backup never syncs and never leaves that computer. If you need the older text back, you can look through it there; nothing is lost silently.

### A damaged file, or a newer format

Every computer writes only its own files, written whole and swapped in atomically, so a sync service never has two versions of one file to choose between. Even so, a file can arrive cut short by a sync still in progress, or otherwise unreadable. textweaver skips a file it cannot read for that merge and reports it once in the sync status; it tries again the next time something changes.

If the sync folder says a newer format than this copy of textweaver understands, sync on this computer becomes read only: it keeps merging what it can read but writes nothing back, so an older textweaver can never damage what a newer one wrote. The status line says so plainly: "Sync: newer version, read only."

## Privacy

**Coming in this wave: not in 0.1.0-alpha.6.**

No name, path, or computer name is ever written to the sync folder. Documents, computers, and records are identified by random ids, never by their titles or the names you gave them on disk; computers are identified only by the name you chose for them ("laptop", "lab"), never by their real machine name. Because of this, the folder is safe to put somewhere someone else can see, such as a shared cloud account or a lab's USB stick, as far as names and paths go. It is not encrypted, so choose a folder you trust the way you would trust any other place that can hold your notes and highlights in plain text.

## Setting it up

**Coming in this wave: not in 0.1.0-alpha.6.** The commands below describe what the sync wave plans to add; use the menus and the command palette once they exist, not a key you have to guess.

Sync is set up, checked, and driven from the menus and the command palette, under Tools, Sync:

- **Set up sync**: choose the folder (through the file browser), name this computer, and choose which groups to sync.
- **Sync status**: says what is in step, what is not, and what could not be read.
- **Sync now.**
- **Go to another computer's place.**
- **Stop syncing on this computer.**

The same actions are planned for the command line, each with a `--json` form for scripts: `tw sync setup`, `tw sync status`, and `tw sync now`.

## See also

- [The library](library.md#sync-your-place-between-computers): today's reading-place sync inside a shared library folder, and `[reading] sync_conflict_policy`.
- [Bookmarks, notes, and highlights](notes.md): what a note, highlight, and bookmark hold.
- [ADR-0049: Sync beyond the place](adr/0049-sync-beyond-the-place.md): the full design, including the folder layout, the merge rules, and what was set aside.
- [Documentation index](README.md)
