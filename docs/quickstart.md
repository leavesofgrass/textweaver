# textweaver quick start

textweaver reads documents aloud and highlights each word as it is spoken. This page covers the first few minutes: starting the program, reading a document, moving around, and finding help. The full guides are listed at the end.

textweaver comes in two forms that share the same documents, keys, settings, notes, and voices. The terminal reader runs in a terminal window. The textweaver app is the graphical version, with menus, buttons, and dialogs. Either works with a screen reader.

## Start textweaver

1. Download the package for your system from the [releases page](https://github.com/leavesofgrass/textweaver/releases). [Installing textweaver](install.md) has every package.
2. Windows, the app: unzip `textweaver-VERSION-windows-x86_64.zip`, and run `textweaver-gui.exe`. Windows warns once, because the program is not code-signed: choose "More info", then "Run anyway".
3. Mac and Linux, the app: unpack the package and open `textweaver.app` on a Mac, or run the AppImage on Linux. See [Installing textweaver](install.md#the-app).
4. The terminal reader is in the same package: open a terminal in its folder, and type `tw QUICKSTART.md` (`textweaver QUICKSTART.md` works too).
5. The first time, the app asks up to three questions: the interface language, who speaks (if a screen reader is running), and which optional components to download. You can skip each one. The terminal reader asks the same.

## Hear a document

1. Press **Ctrl+O**, choose a file, and press Enter. You hear "Opened" and the document's title.
2. Press **Space** to start reading. Press it again to pause.
3. Press **Escape** to stop.

textweaver remembers your place in each document and returns to it the next time you open the file.

## Move around

- **Alt+Down** and **Alt+Up**: next and previous sentence.
- **h**: next heading. **1** to **6**: next heading at that level.
- **Plus** and **Minus**: faster and slower speech.
- **Ctrl+F**, then **F3**: search, then go to the next match.

## Help and keys

- **?** lists every key.
- **F1** opens the help, where typing starts Search help over the commands, keys, settings, and the guides.
- **F2** opens the command palette, a list of every command: type part of a name, then press Enter.
- **Ctrl+Q** quits. Press **y** when asked.

## Who speaks

If you use a screen reader, decide whether textweaver or the screen reader reads the document aloud. **Alt+Shift+A** switches between the two modes: "textweaver reads aloud" and "my screen reader reads". [Using textweaver with a screen reader](screen-readers.md) explains the modes.

## See also

- In the app and the terminal reader, Help, then Quick start, opens this page. Help, then Documentation, opens the full guides that come with textweaver, and works offline. Help, then Online documentation, gives their web address.
- In the package, the guides are in the `docs` folder.
- [Start here, for students](start-students.md)
- [Reading and moving around](reading.md)
- [Every key](keyboard.md)
- [Troubleshooting](troubleshooting.md)
- [Documentation index](README.md)
