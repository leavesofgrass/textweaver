# textweaver quick start

In 30 seconds, you can hear a document read aloud and move around in it. To practise, open this guide itself: it is a Markdown file.

## Windows

1. Download the Windows `.zip` from the [releases page](https://github.com/leavesofgrass/textweaver/releases). Right-click it, choose Extract All, and extract it to a folder such as `C:\textweaver`.
2. Open that folder. Click the address bar, type `cmd`, and press Enter. A command prompt opens in the folder.
3. Type this and press Enter:

   ```bash
   textweaver QUICKSTART.md
   ```

textweaver speaks with your Windows voices. It uses Eloquence if you have it; see `docs/eloquence.md`.

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

Install with the script in a copy of the repository (`git clone https://github.com/leavesofgrass/textweaver`). It works on Debian, Ubuntu, Fedora, Arch, openSUSE, and Alpine: it installs what the build needs, builds textweaver, and installs it in `~/.local`. It asks before it uses sudo or changes your PATH.

```bash
bash scripts/install-linux.sh
```

Then run:

```bash
textweaver ~/.local/share/doc/textweaver/QUICKSTART.md
```

## Your first 30 seconds

Once the document is open:

- **Space** starts reading, and pauses. The highlight follows each word.
- **Escape** stops.
- **.** (period) and **,** (comma) move to the next or previous sentence.
- **p** and **Shift+P** move by paragraph. **h** jumps to the next heading.
- **+** and **-** make the voice faster or slower.
- **%** says where you are.
- **?** lists every key. **F1** opens the help.
- **q** quits. textweaver asks "Quit textweaver? y or n". Press **y** to quit, or **n** to stay.

textweaver remembers your place. Open the same file again and it picks up where you left off.

## Writing

- **Ctrl+E** switches between reading and editing. Type as usual; textweaver echoes what you type.
- **Ctrl+S** saves.
- **Ctrl+Z** undoes.

## Tips

- **Ctrl+O** opens another document.
- **F2** opens the command palette: type part of a command's name, then press Enter.
- **F9** turns single-key shortcuts off, so dictation or typing never triggers a command. Chords such as **Alt+P** (play or pause) still work.
- If you use a screen reader and want it to do all the talking, start with `textweaver --no-speech FILE`.
- `tw speak "Hello"` checks your voice. `tw voices` lists your voices, and `tw backends` lists the speech engines textweaver found.
