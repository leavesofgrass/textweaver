# textweaver quick start

New to textweaver? [Start here, for students](start-students.md) is the shortest path. In 30 seconds, you can hear a document read aloud and move around in it. To practice, open this guide itself: it is a Markdown file.

textweaver opens text, Markdown, HTML, EPUB, Word (DOCX), and PDF files.

## Windows: the window

1. Download the window package, `textweaver-VERSION-windows-x86_64-gui.zip`, from the [releases page](https://github.com/leavesofgrass/textweaver/releases). Right-click it, choose Extract All, and extract it to a folder of your own.
2. Open the folder and run `textweaver-gui.exe`. Windows warns once, because the program is not code-signed: choose "More info", then "Run anyway".
3. Press **Ctrl+O** to open a document and **Space** to hear it. **F1** opens the help, and **?** lists every key.

The window is described in [the window guide](gui.md). The rest of this page is for the terminal reader.

## Windows: the terminal reader

1. Download the Windows `.zip` from the [releases page](https://github.com/leavesofgrass/textweaver/releases). Right-click it, choose Extract All, and extract it to a folder such as `C:\textweaver`.
2. Open that folder. Click the address bar, type `cmd`, and press Enter. A command prompt opens in the folder.
3. Type this and press Enter:

   ```bash
   textweaver QUICKSTART.md
   ```

textweaver speaks with your Windows voices. It uses Eloquence if you have it; see [the Eloquence guide](eloquence.md) (`docs/eloquence.md` in the package).

Or install with a script, which also offers to add textweaver to your PATH and the Start menu. From a copy of the repository:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\install-windows.ps1
```

## macOS

1. Download the macOS `.tar.gz` from the [releases page](https://github.com/leavesofgrass/textweaver/releases) and double-click it to extract it.
2. Open Terminal and type `cd `, with a space after it. Drag the extracted folder into the Terminal window, then press Enter.
3. The first time only, allow the programs to run. This build is not notarized by Apple yet:

   ```bash
   xattr -dr com.apple.quarantine .
   ```

4. Open this guide:

   ```bash
   ./textweaver QUICKSTART.md
   ```

textweaver speaks with Apple's voices. It uses Reed if it is installed.

Or install with a script, from a copy of the repository. It downloads, checks, and installs the newest release in `~/.local/bin`:

```bash
bash scripts/install-macos.sh
```

## Linux

There is an AppImage: one file that runs on most distributions from 2022 on, for x86_64 or for 64-bit ARM (aarch64) computers. `uname -m` says which yours is.

1. Download the Linux `.AppImage` from the [releases page](https://github.com/leavesofgrass/textweaver/releases).
2. Open a terminal in the folder you saved it to, and make it executable:

   ```bash
   chmod +x textweaver-*-linux-x86_64.AppImage
   ```

3. Check that it runs; this says which version you have. With `--tw` first, the AppImage runs `tw`:

   ```bash
   ./textweaver-*-linux-x86_64.AppImage --tw --version
   ```

   On its own it runs the reader. Give it a document, for example this guide, saved from the repository as `quickstart.md`:

   ```bash
   ./textweaver-*-linux-x86_64.AppImage quickstart.md
   ```

4. To run `textweaver` and `tw` from any folder, and add a menu entry, run it once with `--install`. It asks first:

   ```bash
   ./textweaver-*-linux-x86_64.AppImage --install
   ```

Or let the script download the newest release, check it, and install it in `~/.local`:

```bash
bash scripts/install-linux.sh --release latest
```

If the AppImage says FUSE is missing, the script installs the plain tarball instead; [Installing textweaver](install.md#linux) has the details.

textweaver speaks with espeak-ng when the `espeak-ng` package is installed, or through speech-dispatcher. `tw backends` lists the engines it found.

## The GUI

textweaver also has a window, `textweaver-gui`: a native, screen-reader-accessible GUI that shares documents, keys, settings, and voices with the terminal reader. It is its own download, in packages whose names end in `-gui` (see [Installing textweaver](install.md#the-gui)). It is supported on Windows; the macOS and Linux packages are built and checked automatically, but no one has listened to them with a screen reader yet.

To build the window yourself, see [Building](dev/building.md#the-gui).

[The textweaver window (GUI)](gui.md) covers what is in the window, its keys, and its announcements.

## Your first 30 seconds

The five keys the first-run welcome names, in the same order, in both the terminal reader and the window:

- **Ctrl+O** opens a document.
- **Space** starts reading, and pauses. The highlight follows each word.
- **Escape** stops.
- **F2** lists every command by name: type part of one, then press Enter.
- **F1** opens the help.

### More keys

- **Alt+Down** and **Alt+Up** move to the next or previous sentence.
- **p** and **Shift+P** move by paragraph. **h** jumps to the next heading, and **1** to **6** to the next heading at that level, as in NVDA and JAWS.
- **+** and **-** make the voice faster or slower.
- **Tab** turns Speech Cursor mode on and off in the terminal reader. In the window, press **Alt+Shift+S** (or Reading, then Speech Cursor, in the menus); Tab moves between the document and the buttons there. In the mode, the Up and Down arrows read one line at a time.
- **Shift+W** says where you are: the line, the percentage, the word number, and the heading.
- **?** lists every key. **F1** opens the help.
- **Ctrl+Q** quits. textweaver asks "Quit textweaver? y or n". Press **y** to quit, or **n** to stay. With the classic keys (`[keyboard] preset = "classic"`), **q** quits too, after the same question.
- **'** says the last message again. **z** says it too, then the status: the mode, "Ready" or "Reading", the position, the rate, and the voice.

textweaver remembers your place. Open the same file again and it picks up where you left off.

## Writing

- **Ctrl+E** switches between reading and editing. Type as usual; textweaver echoes what you type.
- **Ctrl+S** saves.
- **Ctrl+Z** undoes.
- **Alt+O** lists the headings. Type to filter them, and press Enter to jump to one.

## Tips

- **Ctrl+O** opens another document.
- **F10** opens the menus: File, Edit, View, Reading, Speech, Tools, and Help. **F2** opens the command palette: type part of a command's name, then press Enter. Both offer the same commands, under the same names.
- **F9** turns single-key shortcuts off, so dictation or typing never triggers a command. Chords still work: **Alt+P** plays or pauses in the terminal reader, and **Ctrl+Shift+Space** in the window.
- If you use a screen reader, **Alt+Shift+A** chooses who speaks: textweaver alone (self-voicing), both (hybrid: textweaver reads documents aloud and your screen reader speaks the rest), or your screen reader alone. `textweaver --no-speech FILE` starts silent. [Using textweaver with a screen reader](screen-readers.md) explains the modes.
- If speech stops, **Shift+F8** restarts it.
- textweaver speaks and shows its own words in English, Spanish, French, German, Portuguese, or Arabic. The first run starts with the list of languages; later, choose "Interface language" in the settings screen (**Shift+F10** in the terminal reader, **Ctrl+,** in the window), or run `tw settings language es`.
- `tw speak "Hello"` checks your voice. `tw voices` lists your voices, and `tw backends` lists the speech engines textweaver found. In the terminal reader, **Alt+V** lists the voices, and in the window **Ctrl+Shift+V** does; Enter chooses one and speaks a sample.
- Something went wrong? Warnings and errors are written to `textweaver.log` in the state folder (next to your reading positions). Start with `textweaver --log debug FILE` to log more, or `--log off` to log nothing. [Troubleshooting](troubleshooting.md) covers the common problems.

## Next steps

- [Reading and moving around](reading.md): every way to read and move, Speech Cursor, find, and go to.
- [Writing and editing](editing.md): edit mode, typing echo, and Markdown commands.
- [Keyboard reference](keyboard.md): every key, in both frontends.
- [Speech engines and voices](speech.md): choosing an engine and a voice.
- [Settings](settings.md): where settings live, and how to export and import them.

## See also

- [Installing textweaver](install.md): the details of each package.
- [Documentation index](README.md)
