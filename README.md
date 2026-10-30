# textweaver

<img src="docs/assets/textweaver-logo.svg" alt="textweaver logo: the letters t and w woven on a loom" width="128" height="128">

textweaver is a document reader and writer that speaks. It reads documents aloud while a highlight follows each spoken word, and when you write it echoes the letters and words you type.

It is designed first for people who use a screen reader or a Braille display, and for students with print disabilities. It runs on Linux, macOS, and Windows, in two forms that share documents, settings, and keys: a graphical app and a terminal reader.

Status: beta. The current release is 0.1.0-beta.5. It is suitable for wider testing, and some parts are still changing. [Known limits](docs/known-limits.md) lists what does not work yet.

## Get textweaver

Each release on the [releases page](https://github.com/leavesofgrass/textweaver/releases) offers one download per platform, and each download is complete: it contains textweaver, one program that opens the app by default and runs the terminal reader and the commands as `tw`, with the full documentation. Choose the file that matches your system.

| Platform | Download |
| --- | --- |
| Linux, x86_64 | `linux-x86_64.AppImage` |
| Linux, 64-bit ARM | `linux-aarch64.AppImage` |
| macOS, Apple silicon and Intel | `macos-universal.zip` |
| Windows, x86_64 | `windows-x86_64.zip` |

Linux also has `.tar.gz` packages for systems where AppImages cannot run. Releases up to 0.1.0-alpha.9 distributed the app as separate downloads whose names end in `-gui`.

Each package is self-contained: unpack it, keep its files together, and run the program. Because the builds are not code-signed, each platform needs one first-run step:

- **Linux:** make the AppImage executable with `chmod +x`, then run it. On its own it starts the app; with `--tw` it runs the terminal program. The `--install` option links `textweaver` and `tw` into `~/.local/bin` and adds menu entries for the app and the terminal reader.
- **macOS:** the zip contains `textweaver.app`, with `textweaver` and `tw` beside it as links to the program inside. Remove the quarantine attribute once with `xattr -dr com.apple.quarantine` on the unpacked folder, or choose "Open Anyway" under Privacy & Security.
- **Windows:** run `textweaver.exe` for the app, or `tw.exe` in a terminal. SmartScreen warns on first launch; choose "More info", then "Run anyway".

Alternatively, an install script does this for you. Clone the repository and run the script for your platform. Each script states what it will do and asks before changing your PATH:

```bash
bash scripts/install-linux.sh --release latest
bash scripts/install-macos.sh
powershell -ExecutionPolicy Bypass -File scripts\install-windows.ps1
```

[Installing textweaver](docs/install.md) covers every package, verifying downloads, updates, and building from source.

## First steps

These keys are the same in the app on every platform.

1. Ctrl+O opens a file: a Word document, PDF, EPUB, Markdown, or plain text. textweaver announces "Opened" and the title.
2. Space starts reading, and the highlight follows each spoken word. Space again pauses; Escape stops.
3. Alt+Down and Alt+Up move by sentence. H moves to the next heading.
4. Plus and Minus change the speaking rate.
5. Ctrl+E switches between reading and writing. Ctrl+S saves.
6. F1 opens help. F2 opens the command palette, a list of every command: type part of a name and press Enter.
7. Enter on a link opens a linked document in textweaver, at the heading the link names. Alt+Left goes back to where you were, and Alt+Right goes forward again.
8. Ctrl+Q quits.

textweaver records your position in each file and resumes there the next time you open it.

## What it does

- **Reads aloud with an exact highlight.** The highlight follows each spoken word, with the engines you choose: ETI-Eloquence, Windows and Apple voices, eSpeak NG, Piper neural voices, DECtalk, and more ([Speech engines and voices](docs/speech.md)).
- **Reads and writes your documents.** Word, PDF, EPUB, Markdown, HTML, and many other formats open directly, and edit mode adds typing echo, spell check, citations, and dictation ([Reading](docs/reading.md), [Writing and editing](docs/editing.md), [Converting documents](docs/converting.md)).
- **Follows links between documents.** Enter on a link to another local document opens it in textweaver, with Back and Forward as in a web browser ([Follow a link or a footnote](docs/reading.md#follow-a-link-or-a-footnote-enter-or-altshiftf)).
- **Shows braille.** The Braille view in the app draws the line you are reading as braille dots, in uncontracted or contracted braille, for learning and teaching braille and for checking spacing. It is visual only: your screen reader and Braille display are not sent it ([The Braille view](docs/reading-aids.md#the-braille-view)).
- **Works with Obsidian.** Notes read with their highlights, comments, task statuses, footnotes, and embeds; links resolve across the whole vault; properties, backlinks, unresolved links, tags, and canvases are lists you can open; and edit mode can insert wiki links, toggle tasks, and insert callouts ([Obsidian vaults](docs/vault.md), [Writing and editing](docs/editing.md#obsidian-notes-wiki-links-tasks-and-callouts)).
- **Speaks, keys, and teaches Morse code.** CW is a speech engine that sends a document in Morse code with the highlight following each word; Morse code from a key or paddle can be typed into a document; and Morse code practice teaches it with the Koch method ([CW: reading in Morse code](docs/cw.md)).
- **Helps you study and listen.** Reading aids (RSVP, bionic reading, a reading ruler, spacing, fonts), study cards that go to Anki and back, a reading queue and library, audio export to audiobooks with chapters, and sync through a folder you choose ([Reading aids](docs/reading-aids.md), [Notes](docs/notes.md), [The library](docs/library.md), [Audio export](docs/audio-export.md)).
- **Reads live documents (an advanced feature).** Under Tools, Live documents, textweaver can follow an MQTT broker as a document that grows, such as a net log or a sensor's current values, with each new entry sent to a Braille display and left unspoken unless you turn speech on. It can also run a small broker of its own and publish a document or a check-in to it ([Live documents](docs/live-documents.md), [Running the broker for a live log](docs/live-broker.md)).

The Braille view, link following, and the Obsidian and CW features above arrived in beta 4, and live documents in beta 5; [What is new](docs/whats-new.md) lists them.

## The terminal reader

The terminal reader is text only: it needs no mouse and draws no graphics, and many screen reader users prefer it. It is part of the same program as the app, and `tw` runs it. In a terminal, in the unpacked folder, run:

```bash
tw QUICKSTART.md
```

The reader reads its own quick start aloud. Space starts and pauses; `?` lists every key. Plain `tw` opens the reader with no document, and `textweaver --tw QUICKSTART.md` (or `--cli`) is equivalent to `tw QUICKSTART.md`; `textweaver QUICKSTART.md` opens the file in the app instead. Given a command instead of a file, `tw` runs without the reader, which suits scripts: `tw convert`, `tw speak`, and the others are described in [the command line guide](docs/command-line.md). The reader and the app share settings, reading positions, and notes.

## Screen readers

textweaver can speak with its own voice or leave speech to your screen reader; Alt+Shift+A chooses between them. On Windows it works with NVDA and JAWS. On macOS the app works with VoiceOver, though testing so far has been basic and more is planned. On Linux the app has had basic testing with Orca; if it gives you trouble there, the terminal reader is a dependable alternative. [Using textweaver with a screen reader](docs/screen-readers.md) explains the options.

## Languages

Menus, messages, and speech are available in English, Spanish, German, French, Portuguese, and Arabic. Native speakers have reviewed the English and Spanish; the others have not been reviewed. If a phrase sounds wrong, choose Help, then Report a problem.

## Learn more

- [Quick start](docs/quickstart.md): installation, first reading, and navigation.
- [Start here, for students](docs/start-students.md) and [for staff](docs/start-staff.md).
- [Using the textweaver app](docs/gui.md) and [every key](docs/keyboard.md).
- [Privacy](docs/privacy.md) and the [accessibility statement](docs/accessibility.md).
- [What is new](docs/whats-new.md) and the [changelog](CHANGELOG.md).
- [Updates](docs/updates.md), [optional components](docs/components.md), and [the roadmap](docs/roadmap.md).
- For developers: [Building](docs/dev/building.md), [Contributing](CONTRIBUTING.md), and the [documentation index](docs/README.md).

## Where it came from

textweaver grew out of star, an earlier Python reader, and the owner's other work. It is now an independent project.

## License

Copyright (C) 2026 Jon Pielaet

Licensed under the GNU General Public License, version 3 or later (GPL-3.0-or-later). See [LICENSE](LICENSE) and [NOTICE](NOTICE). Work by others that textweaver includes is listed in [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).
