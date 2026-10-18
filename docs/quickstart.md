# textweaver quick start

textweaver reads documents aloud and highlights each word as it is spoken. This page covers the first few minutes: starting the program, reading a document, moving through it, and finding help. The full guides are listed at the end.

textweaver is one program with two forms that share the same documents, keys, settings, notes, and voices. The textweaver app is the graphical version, with menus, buttons, and dialogs; `textweaver` starts it. The terminal reader runs inside a terminal; `tw` starts it. Both work with a screen reader.

## Start textweaver

1. Download the package for your system from the [releases page](https://github.com/leavesofgrass/textweaver/releases). [Installing textweaver](install.md) describes every package.
2. Windows, the app: unzip `textweaver-VERSION-windows-x86_64.zip` and run `textweaver.exe`. Because the program is not code-signed, Windows warns on first launch; choose "More info", then "Run anyway".
3. macOS and Linux, the app: unpack the package and open `textweaver.app` on a Mac, or run the AppImage on Linux. See [Installing textweaver](install.md#the-app).
4. The terminal reader is the same program. Open a terminal in the package folder and type `tw QUICKSTART.md` (in PowerShell, `.\tw QUICKSTART.md` until the folder is on your PATH). On Linux, run the AppImage with `--tw` first: `./textweaver-*-linux-x86_64.AppImage --tw QUICKSTART.md`. `textweaver --tw QUICKSTART.md` is equivalent to `tw`; `textweaver QUICKSTART.md` opens the file in the app instead.
5. On first launch the app asks up to four questions: the interface language; who speaks, if a screen reader is running; which optional components to download; and whether textweaver may check for updates once a day. Each can be skipped, and nothing is downloaded without your consent. The terminal reader asks the same questions.

## Hear a document

1. Press **Ctrl+O**, choose a file, and press Enter. textweaver announces "Opened" and the document's title.
2. Press **Space** to start reading. Press it again to pause.
3. Press **Escape** to stop.

textweaver records your place in each document and returns to it the next time you open the file.

## Move around

- **Alt+Down** and **Alt+Up**: next and previous sentence.
- **h**: next heading. **1** to **6**: next heading at that level.
- **Plus** and **Minus**: faster and slower speech.
- **Ctrl+F**, then **F3**: search, then move to the next match.

## Help and keys

- **?** lists every key.
- **F1** opens the help. Typing there searches the commands, keys, settings, and guides.
- **F2** opens the command palette, a list of every command. Type part of a name, then press Enter.
- **Ctrl+Q** quits. Press **y** when asked to confirm.

## Who speaks

If you use a screen reader, decide whether textweaver or the screen reader reads documents aloud. **Alt+Shift+A** switches between "textweaver reads aloud" and "my screen reader reads". Avoiding duplicate speech is the main reason to choose deliberately; [Using textweaver with a screen reader](screen-readers.md) describes the modes.

## See also

- In both forms, Help, then Quick start, opens this page. Help, then Documentation, opens the full guides that ship with textweaver and works offline. Help, then Online documentation, gives their web address.
- In the package, the guides are in the `docs` folder.
- [Start here, for students](start-students.md)
- [Reading and moving around](reading.md)
- [What is new in this release](whats-new.md)
- [Every key](keyboard.md)
- [Troubleshooting](troubleshooting.md)
- [Documentation index](README.md)
