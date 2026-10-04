# textweaver

<img src="docs/assets/textweaver-logo.svg" alt="textweaver logo: the letters t and w woven on a loom" width="128" height="128">

textweaver reads documents out loud. As it reads, it highlights each word as it is spoken. You can also use it to write, and it speaks the letters and words you type.

It is made first for people who use a screen reader, and for students who find print hard to read. It runs on Windows, Mac, and Linux.

Status: alpha. It is ready for testing, but not yet for every day. See [known limits](docs/known-limits.md).

## Get textweaver on Windows

You will download one file, unpack it, and run the program.

1. Go to the [releases page](https://github.com/leavesofgrass/textweaver/releases). Choose the latest release.
2. Find the file whose name ends in `windows-x86_64-gui.zip`. Download it.
3. Open your Downloads folder. Right-click the file.
4. Choose "Extract All". Then choose a folder of your own, and finish.
5. Open that folder. Run `textweaver-gui.exe`.
6. Windows may warn you the first time, because the program is not code-signed. Choose "More info". Then choose "Run anyway".

The window opens. You are ready for the first steps below.

## Your first steps

1. Press Ctrl+O. A box opens to choose a file.
2. Choose a document, such as a Word file, a PDF, or a text file. Press Enter. textweaver says "Opened" and the title.
3. Press Space. textweaver starts reading out loud. The highlight follows each word.
4. Press Space again to pause. Press Space once more to go on.
5. Press Escape to stop.
6. Press F1 for help.
7. Press F2 to see the list of every command. Type a few letters of a command's name. Press Enter to run it.
8. Press Ctrl+Q to quit. textweaver asks "Quit textweaver? y or n". Press y.

textweaver remembers your place. Open the same file later, and it picks up where you left off.

## More keys to try

- Alt+Down: next sentence.
- Alt+Up: previous sentence.
- h: next heading.
- Plus: faster voice. Minus: slower voice.
- Ctrl+E: switch between reading and writing.
- Ctrl+S: save what you wrote.

## If you use a screen reader

textweaver can speak by itself, or work with NVDA or JAWS. Press Alt+Shift+A to choose who speaks. [Using textweaver with a screen reader](docs/screen-readers.md) explains the choices.

## Mac and Linux

For both, go to the [releases page](https://github.com/leavesofgrass/textweaver/releases) and choose the latest release.

On a Mac:

1. Download the file ending in `macos-universal-gui.zip`. Double-click it to unpack it.
2. Open the unpacked folder. Run `textweaver-gui`.
3. Your Mac may block it the first time. [Installing textweaver](docs/install.md#the-gui) tells you how to allow it.

On Linux:

1. Download the file ending in `linux-x86_64-gui.AppImage`. For an ARM computer, choose the one with `aarch64` instead.
2. Make it runnable. In a terminal, type `chmod +x` and the file name. Press Enter.
3. Run the file.

The Mac and Linux windows have had basic testing with VoiceOver and Orca, but not every release is tested by a person yet. If something does not work, the terminal reader, below, is a good choice there.

## The terminal reader

The terminal reader runs in a text window. It has no mouse and no pictures. Many people who use a screen reader prefer it.

1. On the releases page, download the file for your computer. Its name has no `-gui` in it. It ends in `windows-x86_64.zip`, `macos-universal.tar.gz`, or `linux-x86_64.AppImage`.
2. Unpack it, and open a terminal in that folder.
3. Type `textweaver QUICKSTART.md` and press Enter.

textweaver reads its own quick start out loud. Press Space to start and pause. Press ? to list every key.

## Languages

textweaver speaks and shows its menus and messages in English, Spanish, German, French, Portuguese, and Arabic. Native speakers have checked the English and the Spanish. The German, French, Portuguese, and Arabic have not been checked yet, so if a word sounds wrong, please tell us. Use Help, then Report a problem.

## Learn more

- [Quick start](docs/quickstart.md): your first 30 seconds, in full.
- [Start here, for students](docs/start-students.md).
- [The textweaver window](docs/gui.md).
- [Every key](docs/keyboard.md).
- [Installing textweaver](docs/install.md): every package, and scripts that install for you.
- [Privacy](docs/privacy.md) and the [accessibility statement](docs/accessibility.md).
- [What is new](docs/whats-new.md) and the [changelog](CHANGELOG.md).
- Building from source, and everything for developers: [Building](docs/dev/building.md), [Contributing](CONTRIBUTING.md), and the [documentation index](docs/README.md).

## Where it came from

textweaver grew out of star and the owner's other work, and now has a life of its own.

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
