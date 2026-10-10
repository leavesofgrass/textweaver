# textweaver

<img src="docs/assets/textweaver-logo.svg" alt="textweaver logo: the letters t and w woven on a loom" width="128" height="128">

textweaver reads documents out loud and highlights each word as it is spoken. It also lets you write, speaking the letters and words you type.

It is made first for people who use a screen reader, and for students who find print hard to read. It runs on Linux, macOS, and Windows, as an app with a graphical interface or in a terminal.

Status: beta. The current release is 0.1.0-beta.1. It is ready for wider testing, and some parts are still changing. See [known limits](docs/known-limits.md).

## Get textweaver

Every release on the [releases page](https://github.com/leavesofgrass/textweaver/releases) has one download for each platform, and each download holds everything: the app (`textweaver-gui`), the terminal program (`tw`, with `textweaver` as a second name), the speech engine helpers, and the complete documentation. Pick the file for your computer.

| Platform | Download |
| --- | --- |
| Linux, x86_64 | `linux-x86_64.AppImage` |
| Linux, 64-bit ARM | `linux-aarch64.AppImage` |
| macOS, Apple silicon and Intel | `macos-universal.zip` |
| Windows, x86_64 | `windows-x86_64.zip` |

Linux also has `.tar.gz` packages for systems where AppImages cannot run. Releases up to 0.1.0-alpha.9 had the app in separate downloads whose names end in `-gui`.

Each package is self-contained: unpack it, keep its files together, and run the program. Each platform has one first-run step, because the builds are not code-signed:

- **Linux:** make the AppImage executable with `chmod +x`, then run it; on its own it starts the app, and with `--tw` it runs `tw`. Add `--install` to link `textweaver-gui`, `tw`, and `textweaver` into `~/.local/bin` and add menu entries.
- **macOS:** the zip holds `textweaver.app` beside `tw`. Clear the quarantine flag once with `xattr -dr com.apple.quarantine` on the unpacked folder, or use "Open Anyway" under Privacy & Security.
- **Windows:** run `textweaver-gui.exe` for the app, or `tw.exe` in a terminal. SmartScreen warns the first time: choose "More info", then "Run anyway".

Or let a script do it. Clone the repository and run the one for your platform; each says what it will do, asks before changing your PATH, and takes `--gui` to install the app too:

```bash
bash scripts/install-linux.sh --release latest --gui
bash scripts/install-macos.sh --gui
powershell -ExecutionPolicy Bypass -File scripts\install-windows.ps1 -Gui
```

[Installing textweaver](docs/install.md) covers every package, checking downloads, updates, and building from source.

## First steps

These keys are the same in the app on every platform.

1. Ctrl+O opens a file: a Word document, a PDF, EPUB, Markdown, or plain text. textweaver says "Opened" and the title.
2. Space starts reading; the highlight follows each word. Space again pauses. Escape stops.
3. Alt+Down and Alt+Up move by sentence. H jumps to the next heading.
4. Plus and Minus change the voice speed.
5. Ctrl+E switches between reading and writing. Ctrl+S saves.
6. F1 opens help. F2 lists every command: type part of its name and press Enter.
7. Ctrl+Q quits.

textweaver remembers your place in each file and picks up where you left off.

## The terminal reader

The terminal reader is text only, with no mouse and no pictures. Many screen reader users prefer it. It is part of `tw`, in the same package as the app. Open a terminal in the unpacked folder and run:

```bash
tw QUICKSTART.md
```

It reads its own quick start out loud. Space starts and pauses; `?` lists every key. Plain `tw` opens the reader with no file, and `textweaver QUICKSTART.md` does the same as `tw QUICKSTART.md`. With a command instead of a file, `tw` works without the reader, for scripts: `tw convert`, `tw speak`, and the rest are in [the command line guide](docs/command-line.md). The reader and the app share settings, reading positions, and notes.

## Screen readers

textweaver can speak by itself or hand speech to your screen reader. Alt+Shift+A chooses who speaks. On Windows it works with NVDA and JAWS. On macOS the app works with VoiceOver; testing so far is basic and more is planned. On Linux the app has had basic testing with Orca, and if it gives you trouble, the terminal reader is a solid fallback. [Using textweaver with a screen reader](docs/screen-readers.md) explains the choices.

## Languages

Menus, messages, and speech come in English, Spanish, German, French, Portuguese, and Arabic. English and Spanish have been checked by native speakers; the others have not, so if a word sounds wrong, use Help, then Report a problem.

## Learn more

- [Quick start](docs/quickstart.md): your first 30 seconds, in full.
- [Start here, for students](docs/start-students.md) and [for staff](docs/start-staff.md).
- [Using the textweaver app](docs/gui.md) and [every key](docs/keyboard.md).
- [Privacy](docs/privacy.md) and the [accessibility statement](docs/accessibility.md).
- [What is new](docs/whats-new.md) and the [changelog](CHANGELOG.md).
- [Updates](docs/updates.md), [optional components](docs/components.md), and [the roadmap](docs/roadmap.md).
- For developers: [Building](docs/dev/building.md), [Contributing](CONTRIBUTING.md), and the [documentation index](docs/README.md).

## Where it came from

textweaver grew out of star and the owner's other work, and now has a life of its own.

## License

Copyright (C) 2026 Jon Pielaet

Licensed under the GNU General Public License, version 3 or later (GPL-3.0-or-later). See [LICENSE](LICENSE) and [NOTICE](NOTICE). Work by others that textweaver includes is listed in [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).
