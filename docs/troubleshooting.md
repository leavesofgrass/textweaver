# Troubleshooting

This guide helps when textweaver does not do what you expect: no speech, the wrong voice, a highlight out of step, a file that will not open, and other common problems. It also explains the log file, the checking tools, and how to report a bug. It is for anyone using textweaver, and for anyone helping them.

Each problem below is a heading that says what you notice. Under it are the steps to try, in order.

## Tools for finding out what is wrong

### The log file

The terminal reader and the window write warnings and errors to a log file, `textweaver.log`, in the state folder. The state folder is `state` inside the data folder; the [library guide](library.md) says where that is on each system. With `TEXTWEAVER_HOME` or `--home`, it is `data\state` under that folder.

The log is written by `textweaver`, `tw open`, and `tw serve`. Other `tw` commands print their errors in the terminal instead.

The log is small. When it reaches 1 MB it is renamed `textweaver.log.1`, and a new one starts. Three older logs are kept: `textweaver.log.1` is the newest of them, and `textweaver.log.3` the oldest.

To log more, start the reader with `--log` and a level:

```bash
textweaver --log debug essay.md
```

The levels, from least to most: `off`, `error`, `warn` (the default), `info`, `debug`, and `trace`. `--log` alone means `debug`. `--log off` writes no log.

You can also set the level with the `TEXTWEAVER_LOG` environment variable. `--log` wins when both are given. `tw serve` reads only the variable. An unknown level is announced when the reader starts: "Unknown log level", the name, then "Use off, error, warn, info, debug, or trace."

The log holds messages about textweaver, such as a position that could not be saved or a state file that was set aside. It does not hold your documents' text.

### tw backends and tw voices

To see which speech engines textweaver found, and which one it chooses:

```bash
tw backends
```

To list the voices of the chosen engine:

```bash
tw voices
```

To list the voices of one engine, name it:

```bash
tw voices --backend sapi
```

To hear a test sentence:

```bash
tw speak "Testing one two three."
```

These three commands read your `settings.toml`, as the reader does, so they use your engine and voice. To see what they do with the default settings, point them at an empty folder with `--home`. The [speech guide](speech.md) explains what they print.

### The doctor and speech-check scripts

The `scripts` folder has two checking scripts. The [scripts guide](../scripts/README.md) describes every option.

The doctor writes one plain-text report to paste into a bug report: your system, your terminal, the screen reader if one is running, where textweaver and the window are installed, `tw backends`, the optional components with one line such as "Components: 3 of 12 installed", text recognition, whether the engine hosts sit beside the programs, and the optional tools. It reads no file contents and shows only the names of `TEXTWEAVER_` variables that are set. On Windows:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\doctor.ps1 -Out doctor.txt
```

On macOS and Linux:

```bash
scripts/doctor.sh --out doctor.txt
```

The speech check reports on speech alone: `tw backends`, the first voices of each engine, `tw eloquence`, and on Linux the state of espeak-ng, Speech Dispatcher, and the sound server. Nothing is spoken unless you add `--speak` (`-Speak` in PowerShell). On Windows:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\speech-check.ps1
```

On macOS and Linux:

```bash
scripts/speech-check.sh
```

## Common problems

### No speech at all

1. Look at the title line. Its last part is the speech engine. If it says `silent`, textweaver was started with `--no-speech`, or no engine could start.
2. Look at the status line when textweaver starts. "Speech engine X is not available; using Y." means the engine you asked for was not found. "X could not start; Y is speaking instead." means an engine was found but failed, and textweaver moved on to the next one; the log says why. textweaver is silent only when every engine on the computer failed, and then says which ones did.
3. Run `tw backends`. It lists the engines that can run on your system; an engine marked "Not installed" was not found. If only `null` is available, textweaver found no engine. On Windows, check that the engine hosts are next to the programs (see [A voice or engine is missing](#a-voice-or-engine-is-missing)). On Linux, install espeak-ng or Speech Dispatcher, or use the install script, which does.
4. Check the volume: press **F7** a few times. "Full volume." means it is at the top. Check your system's volume too.
5. Run `tw speak "Hello"`. If that speaks but the reader does not, check `[speech] backend` in `settings.toml`, or start with `--backend` and an engine id.
6. Start with `--log debug`, try again, and read `textweaver.log`.

### The window does not open, or it is blank

1. Start it from a terminal, so any error shows there, and write a log: `textweaver-gui --log-file gui.log`. Started from a shortcut or File Explorer, a startup error is shown in a message box instead.
2. Try another graphics interface for one run: `textweaver-gui --graphics vulkan`. The others are `dx12` (Windows), `metal` (macOS), and `gl`. To keep the one that works, put `graphics = "vulkan"` under `[gui]` in `settings.toml`.
3. Attach `gui.log` to a bug report; see [Report a bug](#report-a-bug).

### Your screen reader does not read the window's messages

1. Open Settings (Ctrl+,), choose the section Window, and change Announcements. The choices are a live region (the default) and UI Automation notifications (Windows only). The change applies from the next start. To try the other one for a single run, start with `--announce uia` or `--announce live`.
2. Check the accessibility mode: **Alt+Shift+A** changes it. In screen reader mode textweaver itself is silent, and your screen reader reads its messages.
3. NVDA says a button's key only when its setting for reporting object shortcut keys is on (Object Presentation).

### The file chooser does not open

On Linux the window's Open dialog needs the XDG desktop portal. When it cannot open, textweaver says so and shows a one-line prompt for the path instead. **Ctrl+Shift+G** opens the same prompt at any time, and Tab completes the path.

### textweaver and your screen reader talk at once

textweaver speaks for itself by default. With a screen reader running, choose who speaks:

1. Press **Alt+Shift+A** to cycle the accessibility mode: self-voicing, hybrid, and screen reader. You hear the new mode, and it is saved.
2. **Hybrid** leaves messages, typing echo, and cursor moves to your screen reader, and textweaver reads documents aloud. **Screen reader** mode makes textweaver silent.
3. To try a mode for one run, start with `--mode hybrid` or `--mode screen-reader`. `--no-speech` is screen reader mode.

On its first run with a screen reader, textweaver offers hybrid mode once. [Using textweaver with a screen reader](screen-readers.md) explains the modes and the screen reader settings that help.

### The wrong voice speaks

1. Press **Alt+V** in the terminal reader, or **Ctrl+Shift+V** in the window, and choose the voice. The one in use says "current". The choice is saved.
2. Check `[speech] voice` in `settings.toml`. A voice name that matches nothing leaves the engine's default voice in use.
3. With no voice chosen, `[speech] prefer_voice` (default `"eloquence"`) picks one whose name contains it. Set it to part of the name you want, or to `""`.
4. Check the engine. Your voice may belong to another engine: `tw voices --backend sapi`, for example. Set `[speech] backend` to that engine.

### The highlight runs ahead of or behind the voice

1. Check which engine you use. With Eloquence and DECtalk the highlight is exact. With SAPI, eSpeak NG, and avspeech it follows the word times plus a delay for the sound card.
2. For those engines, change the delay in `settings.toml`. Raise it if the highlight is early, lower it if it is late. The default is 120 milliseconds:

   ```toml
   [speech]
   latency_offset_ms = 180
   ```

3. If textweaver said "This voice does not report words, so the word highlight is estimated.", the highlight is a guess based on the rate. Adjust it with `[highlight] speed`, from 0.5 (slower) to 1.5 (faster).
4. `[highlight] lead_words` moves the drawn highlight a whole word ahead or behind. 1, the default, is the word you hear.

The [speech guide](speech.md) explains how each engine is highlighted.

### Eloquence is not found

1. Run `tw eloquence`. It lists every Eloquence it found, which one it uses, and what to do if there is none.
2. textweaver does not include Eloquence. The [Eloquence guide](eloquence.md) explains how to get a licensed copy for Windows, macOS, and Linux.
3. With Code Factory's Eloquence for Windows, textweaver uses it only after you say you own it: set `TEXTWEAVER_ECI_CODE_FACTORY` to 1, or `[speech.eci] code_factory = true`.
4. If your Eloquence is somewhere unusual, name its library with `TEXTWEAVER_ECI_LIBRARY`, or `[speech.eci] library`.
5. Eloquence runs in a helper program, `textweaver-eci-host`. It must be in the same folder as `textweaver` and `tw`.

### A voice or engine is missing

Eloquence, SAPI voices, DECtalk, and on Windows eSpeak NG run in helper programs, called engine hosts. On Windows these are `textweaver-eci-host.exe`, `textweaver-sapi-host.exe`, `textweaver-dectalk-host.exe`, `textweaver-espeak-host.exe`, and their 32-bit versions ending in `-x86.exe`.

1. Check that the host files are in the same folder as `textweaver.exe` and `tw.exe`. If you copied only the two programs somewhere else, copy the hosts and the `ibmtts-dictionaries` folder too. The doctor script checks this.
2. A 32-bit SAPI voice (such as older VW or eSpeak SAPI voices) needs `textweaver-sapi-host-x86.exe`. Without it, 32-bit voices are not listed.
3. The whole `sapi` engine is available only when the 64-bit host, `textweaver-sapi-host.exe`, is found.
4. If the hosts are elsewhere, name them with `TEXTWEAVER_SAPI_HOST`, `TEXTWEAVER_SAPI_HOST_X86`, `TEXTWEAVER_ECI_HOST`, or `TEXTWEAVER_DECTALK_HOST`. The [speech guide](speech.md) lists them.
5. OneCore voices (Microsoft Mark, David, Zira) are listed only when `[speech.sapi] onecore` is `true`, the default.

### Windows says "Windows protected your PC", or macOS will not open the program

The programs are not code-signed.

- **Windows SmartScreen** may warn the first time you run a downloaded program. Choose "More info", then "Run anyway". A program started from a terminal usually shows no warning.
- **macOS Gatekeeper** blocks the first run of a download that Apple has not notarized. Remove the quarantine flag once, in the extracted folder:

  ```bash
  xattr -dr com.apple.quarantine .
  ```

The [install guide](install.md) has the details.

### On macOS, tw speak with avspeech says nothing

The `avspeech` engine needs the program's main thread to keep handing it work. The terminal reader does this, so reading with `avspeech` works there. `tw speak` does not, so with `--backend avspeech` it waits, then reports an error starting "AVSpeechSynthesizer produced no audio".

- For `tw speak`, use `--backend nsspeech`.
- Or write the speech to a file with `--out`.

This comes from the code; it has not been tested on a real Mac yet.

### A key does nothing

1. Your terminal may not send that key. Terminals cannot send `Ctrl+H`, `Ctrl+I`, `Ctrl+M`, `Ctrl+J`, `Ctrl+Shift` with a letter, or `Ctrl` with digits and most punctuation. Some terminal programs keep **F10**, **F11**, **Shift+Up**, and **Shift+Down** for themselves. The "Terminal notes" in the [keyboard reference](keyboard.md) list them.
2. Use the command's other key, if it has one. Chapters, for example, also move with **Alt+PageDown** and **Alt+PageUp**.
3. Run the command from the command palette: press **F2** and type its name.
4. Single-key shortcuts may be off. Press **F9**; you hear "Single-key shortcuts on."
5. Your screen reader may take the key first. See [Using textweaver with a screen reader](screen-readers.md).
6. Give the command a key your terminal can send, in `keymap.toml`. textweaver warns about keys a terminal cannot send.

### The colors are hard to read

1. Press **F5** to try the next color theme. The title line and status line change at once.
2. Pick a theme for one run with `--theme`, for example `--theme high-contrast`. `textweaver --help` lists the themes.
3. Turn color off: set the `NO_COLOR` environment variable to `1`. textweaver still marks the highlight, the selection, and the cursor with bold, underline, or reverse video, never with color alone.
4. If colors look wrong, your terminal may report the wrong color support. Set `TEXTWEAVER_COLOR` to `truecolor`, `256`, `16`, or `none`. It wins over `NO_COLOR`.

The [themes guide](themes.md) explains themes and how to make your own.

### A PDF reads in the wrong order, or says it has no text

1. A scanned PDF is a picture of the pages, and textweaver recognizes its text (OCR). When it reads as one sentence that begins "This PDF has no text layer", no recognition engine could run, and the sentence says why. Run `tw ocr status` to see which engines are ready, and `tw ocr download` for the English models (it asks first); other languages need Tesseract. See [Scanned pages](converting.md#scanned-pages-ocr).
2. textweaver rebuilds each page's reading order: columns left to right, and each column top to bottom, without running heads and page numbers. Unusual layouts can confuse it, such as three-column magazines, tables without aligned columns, or lists whose bullets are pictures. Tagged PDFs, such as those saved from Word with accessibility tags, read most reliably.
3. To look at the text textweaver got, print it:

   ```bash
   tw text paper.pdf
   ```

4. A password-protected PDF is refused. Remove the password in a PDF program first.

### "is not a text file"

textweaver refuses a file that is not text, such as a program, an audio file, or an old Word `.doc`. The message says what the file looks like. RTF, OpenDocument text, LaTeX, email, and saved web pages open directly, and PNG and JPEG pictures are read by recognizing their text.

1. Check that you opened the file you meant.
2. For an old Word `.doc`, Excel `.xls`, or PowerPoint `.ppt` file, you hear "it is an old Microsoft Office file". Open it in Word or LibreOffice, save it in a newer format such as `.docx`, then open that.
3. For a RAR archive, you hear "it is a RAR archive, which does not open". Extract it first, or pack it as ZIP or 7z, which open directly.
4. For a Textile, DocBook, or similar file, convert it to Markdown, then open the Markdown. This needs Pandoc. LaTeX, Org, and reStructuredText open directly.

   ```bash
   tw convert report.textile --to md
   ```

The [converting guide](converting.md) explains `tw convert`.

### "Could not open" a document that has the right file name ending

A file can have the right ending, `.odt`, `.rtf`, `.tex`, `.eml`, or `.mhtml`, and still not be a readable one, for example a `.eml` that is really an export log, or a `.tex` file saved with the wrong encoding. textweaver names the kind of file it expected and says it may be damaged, plainly and without an operating system error code:

- "Could not open notes.odt: it is not a readable OpenDocument text file; it may be damaged."
- "Could not open report.tex: it is not a readable LaTeX file; it may be damaged."
- "Could not open message.eml: it is not a readable email message; it may be damaged."
- "Could not open page.mhtml: it is not a readable web archive; it may be damaged."

1. Open the file in the program that made it (Word, your mail program, your browser's "Save page as") and save it again.
2. For LaTeX, check that the file is plain text saved as UTF-8, not a binary format such as a compiled `.pdf` renamed to `.tex`.
3. If the file opens correctly elsewhere, the file itself is fine and this is worth reporting; see [Report a bug](#report-a-bug).

### Bookmarks and notes for one document are gone, and there is a .bak file

When a document's state file cannot be read, for example after a sync conflict or a hand edit, textweaver does not overwrite it. It renames `<key>.json` to `<key>.corrupt-<time>.bak` in the state folder, where the time is in seconds since 1970, and the document starts with no saved place or marks. This is written to the log, but not announced.

1. Look in the state folder for the `.corrupt-` file. The log names it.
2. It is JSON. Open it in a text editor. Often one missing comma or bracket is the problem.
3. Close textweaver. Fix the file, and rename it back to `<key>.json`: the same name, with `.corrupt-`, the time, and `.bak` replaced by `.json`. If textweaver already wrote a new `<key>.json` for that document, move that one away first.

A corrupt `settings.toml` is handled the same way. textweaver uses the default settings and says so: "Settings file was unreadable and has been reset to defaults", the reason, and where the backup was saved (`settings.toml.corrupt-<date>-<time>.bak`). One invalid value costs only that value: textweaver says "Some settings were invalid and use their defaults:" and names each one.

### Your place moved after the file changed elsewhere

When a document changes in another program (Obsidian, `git pull`, another editor), textweaver finds your place, bookmarks, notes, and highlights again from the text they were on, and says once what moved: "The file changed;" then what was moved to match, and what could not be found.

1. A mark that could not be found is marked as not found and put at the same share of the way through the document. Its passage was rewritten or deleted.
2. Check the mark's list (**Shift+M** for bookmarks, **Shift+A** for notes) and move or delete it.

[Bookmarks, notes, and highlights](notes.md#when-the-file-changes-in-another-program) explains how marks are found again.

### Quitting says "Still saving"

Saving, positions, and notes are written in the background. If the disk is slow, a network drive for example, quitting waits for the writing to finish, at most ten seconds, and says "Still saving. Please wait." Wait for it; your work is being written.

### Sync folder missing

The status line says "Sync: folder missing, saving here", and Sync status (Shift+F5) says the same: the sync folder chosen in Set up sync is not there. Usually that is expected, for example a USB stick that is not plugged in, or a network folder while you are away from the network.

1. Nothing is lost. Your places, notes, highlights, and bookmarks are saved on this computer as usual.
2. Plug the stick back in, or reconnect. textweaver picks the folder up again on its own and sends what changed; you do not need to restart. Tools, Sync, Sync now sends everything at once.
3. If the folder was moved or renamed, choose it again with Tools, Sync, Set up sync, or set `[sync] folder` in `settings.toml`.
4. If the folder is there but the status stays the same, check that the drive letter or mount point is the same as when you set sync up.

### Sync: newer format, read only

The status line says "Sync: newer format, read only": a newer textweaver on another computer wrote the sync folder in a format this version does not know. This computer still reads what it can, and writes nothing to the folder, so it can never damage what the newer version wrote. Your changes here are saved on this computer, and are not sent.

1. Update textweaver on this computer to the version your other computers use. Sync then carries on by itself, and sends what you did here.
2. Until then, "Continue reading", library search, and the statistics still use what this computer can read from the folder.
3. The same message appears when the folder's `textweaver-sync/format.json` is damaged. If every computer runs the same version, look at that file: it should hold only the format number, such as `{"format": 1}`. Do not delete the folder; the other computers' data is in it.

A single damaged file is different: the status says "Sync: 1 damaged file skipped", usually a file a sync service had not finished copying. It is tried again at the next change. See [Syncing between computers](sync.md#problems).

### Importing settings fails

`tw settings import` checks everything before it writes anything. If anything is wrong, nothing changes.

1. Try a dry run first. It shows each change, or each problem:

   ```bash
   tw settings import my-settings.json --dry-run
   ```

2. Each problem names its place in the file, such as `settings.speech.rate: 5000 is outside 50 to 900 words per minute`. Fix those values and try again.
3. "This is a star settings file" means the file is star's `settings.json`. Use `tw migrate-star` instead; see the [library guide](library.md).
4. A file from a newer textweaver is refused: "This file was exported by a newer textweaver". Update textweaver, or export again with fewer settings (`--changed-only`).
5. "Your current settings file ... cannot be read" means your own `settings.toml` is damaged. Fix or move it away, then import.
6. Close the reader before importing from the command line. Otherwise it may save its own settings over the imported ones.

The [settings guide](settings.md) explains import and export.

### Speech Dispatcher does not work in a container or over SSH

The `speechd` engine talks to the Speech Dispatcher server through a socket.

1. textweaver looks for the socket where Speech Dispatcher's own programs do: `SPEECHD_ADDRESS` if set, else in `XDG_RUNTIME_DIR`, else, where there is none (a container, or a session without a login manager), in `~/.cache/speech-dispatcher/`.
2. If nothing answers, textweaver starts the server itself and tries for five seconds. Check that `speech-dispatcher` is installed and that `spd-say hello` works in the same shell.
3. A container has no sound device of its own. Route its sound to the host, as the [Docker guide](dev/docker.md) describes, or check speech without a device: `TEXTWEAVER_ESPEAK_OUTPUT=virtual` makes eSpeak NG keep real timing without playing anything.
4. Run `scripts/speech-check.sh`. On Linux it reports on Speech Dispatcher, the sound server, and the session.

### Speech stops in the middle, or you hear "Speech restarted"

textweaver restarts an engine that crashes, or that plays nothing for 12 seconds, and reads on from the last word: "Speech restarted:", the reason, then "Reading on from the last word." If it happens again at once, reading stops with a message ending "Check the audio device".

1. Check that the sound device is connected and not in use by another program.
2. Check the log for the reason.
3. Try another engine with `--backend`, to see whether the problem is the engine or the device.
4. If you chose a device in `[speech] output_device`, run `tw backends --devices`: its last line says when that device is not connected, and speech then plays on the default.

### Speech plays on the wrong device

Speech plays on the system's default sound device. When a headset or another device goes away, textweaver moves speech to the current default at once. To keep speech on one device, choose it by its id: `tw backends --devices` lists them, and the [speech guide](speech.md#choose-the-sound-device) has the steps. eSpeak NG, Speech Dispatcher, and the Apple voices always play on the default; choose the device in the system's own sound settings for them.

If you hear "Speech stopped working", textweaver restarts speech once by itself. If it stops working again, press **Shift+F8** (Restart speech), or restart textweaver.

## Report a bug

A good report lets someone else see the same problem. Include:

1. What you did, what you expected, and what happened instead. Quote what textweaver said, from the status line.
2. textweaver's version. In the window or the terminal reader, choose About from the command palette (in the window, also from the Help menu): it says the version. From a command line:

   ```bash
   tw --version
   ```

3. The doctor report: run the doctor script with `--out doctor.txt` (or `-Out doctor.txt` in PowerShell), as shown above, and attach the file.
4. The log: start the reader with `--log debug`, make the problem happen, quit, and attach `textweaver.log` from the state folder (`tw settings path` names it). For the window, start it with `--log-file gui.log` and attach that file.
5. If a document causes it, and you may share it, attach the document or a small part of it that shows the problem.

Report bugs at the project's issue tracker: [github.com/leavesofgrass/textweaver/issues](https://github.com/leavesofgrass/textweaver/issues).

## See also

- [Using textweaver with a screen reader](screen-readers.md): keys, double speech, and terminals.
- [Speech engines and voices](speech.md): engines, voices, and every speech setting.
- [Installing textweaver](install.md): packages, SmartScreen, and Gatekeeper.
- [Keyboard reference](keyboard.md): every key, and the terminal notes.
- [Scripts](../scripts/README.md): the doctor, the speech check, and the install scripts.
- [Documentation index](README.md)
