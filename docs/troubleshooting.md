# Troubleshooting

This guide addresses cases in which textweaver does not behave as expected: no speech, the wrong voice, a highlight out of step, a file that will not open, and other common problems. It also describes the log file, the diagnostic tools, and how to report a bug. It is meant for anyone using textweaver and for anyone helping them.

Each problem is a heading that names the symptom, followed by the steps to try, in order.

## Tools for finding out what is wrong

### The log file

The terminal reader and the textweaver app write warnings and errors to a log file, `textweaver.log`, in the state folder. The state folder is `state` inside the data folder; the [library guide](library.md) gives its location on each system. With `TEXTWEAVER_HOME` or `--home`, it is `data\state` under that folder.

The log is written by the reader (`tw` or `tw FILE`), `tw open`, and `tw serve`. Other `tw` commands print their errors in the terminal instead.

The log is kept small. When it reaches 1 MB it is renamed `textweaver.log.1` and a new log begins; three older logs are retained, with `textweaver.log.1` the newest and `textweaver.log.3` the oldest.

To log more, start the reader with `--log` and a level:

```bash
tw --log debug essay.md
```

The levels, from least to most: `off`, `error`, `warn` (the default), `info`, `debug`, and `trace`. `--log` alone means `debug`. `--log off` writes no log.

The level can also be set with the `TEXTWEAVER_LOG` environment variable; `--log` takes precedence when both are given, and `tw serve` reads only the variable. An unknown level is announced when the reader starts: "Unknown log level", the name, then "Use off, error, warn, info, debug, or trace."

The log records messages about textweaver itself, such as a position that could not be saved or a state file that was set aside. It never contains the text of your documents.

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

These three commands read your `settings.toml`, as the reader does, and therefore use your engine and voice. To see their behavior with the default settings, point them at an empty folder with `--home`. The [speech guide](speech.md) explains their output.

### The doctor and speech-check scripts

The `scripts` folder has two checking scripts. The [scripts guide](../scripts/README.md) describes every option.

The doctor writes a single plain-text report suitable for a bug report. It covers your system, your terminal, the screen reader if one is running, where textweaver and the app are installed, `tw backends`, the optional components with one line such as "Components: 3 of 12 installed", text recognition, whether the 32-bit engine host and the dictionaries sit beside the programs, and the optional tools. It reads no file contents and lists only the names of `TEXTWEAVER_` variables that are set. On Windows:

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

1. Look at the title line, whose last part names the speech engine. If it says `silent`, textweaver was started with `--no-speech`, or no engine could start.
2. Read the status line when textweaver starts. "Speech engine X is not available; using Y." means the engine you asked for was not found. "X could not start; Y is speaking instead." means an engine was found but failed and textweaver moved to the next one; the log gives the reason. textweaver is silent only when every engine on the computer has failed, and it then says which ones.
3. Run `tw backends`, which lists the engines that can run on your system; an engine marked "Not installed" was not found, and if only `null` is available, textweaver found no engine. On Windows, check that `textweaver-host-x86.exe` is next to the programs (see [A voice or engine is missing](#a-voice-or-engine-is-missing)). On Linux, install espeak-ng or Speech Dispatcher, or use the install script, which does.
4. Check the volume: press **F7** a few times, and "Full volume." confirms it is at the maximum. Check the system volume as well.
5. Run `tw speak "Hello"`. If that speaks but the reader does not, check `[speech] backend` in `settings.toml`, or start with `--backend` and an engine id.
6. Start with `--log debug`, try again, and read `textweaver.log`.

### The app does not open, or it is blank

1. Start it from a terminal, so that any error appears there, and write a log: `textweaver --log-file gui.log`. Started from a shortcut or File Explorer, a startup error is shown in a message box instead.
2. Try another graphics interface for one run: `textweaver --graphics vulkan`. The alternatives are `dx12` (Windows), `metal` (macOS), and `gl`. To keep the one that works, set `graphics = "vulkan"` under `[gui]` in `settings.toml`.
3. Attach `gui.log` to a bug report; see [Report a bug](#report-a-bug).

### Your screen reader does not read the app's messages

1. Open Settings (Ctrl+,), choose the section Window (it holds the app's settings), and change Announcements. The choices are a live region (the default) and UI Automation notifications (Windows only), and the change applies from the next start. To try the other for a single run, start with `--announce uia` or `--announce live`.
2. Check the accessibility mode, which **Alt+Shift+A** changes. In screen reader mode textweaver itself is silent and your screen reader reads its messages.
3. NVDA announces a button's shortcut key only when its setting for reporting object shortcut keys (Object Presentation) is on.

### The file chooser does not open

On Linux, the app's Open dialog requires the XDG desktop portal. When the dialog cannot open, textweaver says so and shows a one-line prompt for the path instead. **Ctrl+Shift+G** opens the same prompt at any time, and Tab completes the path.

### textweaver and your screen reader talk at once

By default textweaver speaks for itself. When a screen reader is also running, choose which of them speaks:

1. Press **Alt+Shift+A** to cycle the accessibility mode through self-voicing, hybrid, and screen reader. The new mode is announced and saved.
2. In **hybrid** mode your screen reader handles messages, typing echo, and cursor moves, while textweaver reads documents aloud. **Screen reader** mode silences textweaver entirely.
3. To try a mode for one run, start with `--mode hybrid` or `--mode screen-reader`; `--no-speech` is equivalent to screen reader mode.

On its first run with a screen reader, textweaver offers hybrid mode once. [Using textweaver with a screen reader](screen-readers.md) explains the modes and the screen reader settings that help.

### The wrong voice speaks

1. Press **Alt+V** in the terminal reader, or **Ctrl+Alt+V** in the app (**Cmd+Option+V** on macOS), and choose the voice. The voice in use is marked "current", and your choice is saved.
2. Check `[speech] voice` in `settings.toml`. A voice name that matches nothing leaves the engine's default voice in use.
3. When no voice is chosen, `[speech] prefer_voice` (default `"eloquence"`) selects a voice whose name contains that text. Set it to part of the name you want, or to `""`.
4. Check the engine, since your voice may belong to another one (for example, `tw voices --backend sapi`), and set `[speech] backend` to that engine.

### The highlight runs ahead of or behind the voice

1. Check which engine you use. With Eloquence and DECtalk the highlight is exact; with SAPI, eSpeak NG inside textweaver, and avspeech it follows the reported word times plus a delay for the sound card.
2. For those engines, adjust the delay in `settings.toml`: raise it if the highlight is early and lower it if the highlight is late. The default is 120 milliseconds:

   ```toml
   [speech]
   latency_offset_ms = 180
   ```

3. If textweaver said "This voice does not report words, so the word highlight is estimated.", the highlight is an estimate based on the rate. Adjust it with `[highlight] speed`, from 0.5 (slower) to 1.5 (faster).
4. `[highlight] lead_words` shifts the drawn highlight by whole words. The default, 1, is the word you hear.

The [speech guide](speech.md) explains how each engine is highlighted.

### Eloquence is not found

1. Run `tw eloquence`, which lists every Eloquence it found, the one it uses, and what to do if there is none.
2. textweaver does not include Eloquence. The [Eloquence guide](eloquence.md) explains how to obtain a licensed copy for Windows, macOS, and Linux.
3. textweaver uses Code Factory's Eloquence for Windows only after you declare that you own it: set `TEXTWEAVER_ECI_CODE_FACTORY` to 1, or `[speech.eci] code_factory = true`.
4. If your Eloquence is installed in an unusual location, name its library with `TEXTWEAVER_ECI_LIBRARY`, or `[speech.eci] library`.
5. Eloquence runs in an engine host: a copy of `textweaver` started in its hidden `--engine-host` mode for a 64-bit library, and `textweaver-host-x86.exe` for a 32-bit one. The 32-bit host must be in the same folder as `textweaver` and `tw`.

### A voice or engine is missing

Eloquence, SAPI voices, DECtalk, and (on Windows) eSpeak NG run in separate processes called engine hosts. A 64-bit engine runs in a copy of `textweaver` started in its hidden `--engine-host` mode; a 32-bit engine runs in the single 32-bit host, `textweaver-host-x86.exe`.

1. Check that `textweaver-host-x86.exe` and the `ibmtts-dictionaries` folder are in the same folder as `textweaver.exe` and `tw.exe`. If you copied only the programs somewhere else, copy those too. The doctor script checks this.
2. A 32-bit SAPI voice (such as older VW or eSpeak SAPI voices) requires `textweaver-host-x86.exe`; without it, 32-bit voices are not listed.
3. If a program other than textweaver's own should run the hosts (an older separate host, for example), name it with `TEXTWEAVER_SAPI_HOST`, `TEXTWEAVER_SAPI_HOST_X86`, `TEXTWEAVER_ECI_HOST`, or `TEXTWEAVER_DECTALK_HOST`. The [speech guide](speech.md) lists them.
4. OneCore voices (Microsoft Mark, David, Zira) are listed only when `[speech.sapi] onecore` is `true`, the default.

### Windows says "Windows protected your PC", or macOS will not open the program

The programs are not code-signed.

- **Windows SmartScreen** may warn the first time you run a downloaded program. Choose "More info", then "Run anyway". A program started from a terminal usually shows no warning.
- **macOS Gatekeeper** blocks the first run of a download that Apple has not notarized. Remove the quarantine flag once, in the extracted folder:

  ```bash
  xattr -dr com.apple.quarantine .
  ```

The [install guide](install.md) has the details.

### An antivirus program flags textweaver

Some antivirus programs flag a textweaver program, such as `tw.exe` or `textweaver-host-x86.exe`, with a heuristic detection: a name that starts with `Gen:Heur`, `Heur`, or `HEUR`, or says "suspicious" or "generic". A heuristic detection means only that the program looks unusual to the scanner, not that it matched known malware. Small unsigned programs that start other copies of themselves and communicate with them over pipes, as textweaver's engine hosts do, are a common cause. Since 0.1.0-beta.3 the package ships fewer such programs: the separate host for each engine is gone, and the one host left is the 32-bit host.

1. Confirm that your copy is genuine by comparing the package's checksum with `SHA256SUMS.txt` from the release page (see [Checking a download](install.md#checking-a-download)). If they match, the files are those the release published.
2. Report the false positive to your antivirus vendor. Most vendors provide a web form for this (search the vendor's site for "false positive" or "submit a sample"). Give the file name, the detection name, the textweaver version, and the place you downloaded it.
3. Until the detection is cleared, ask whoever administers the computer to allow the textweaver folder in the antivirus program; on a work computer that is your IT department. Do not turn the antivirus off.

If the flagged file is `textweaver-host-x86.exe`, textweaver still works without it, except for 32-bit engines and voices. If it is `textweaver.exe`, textweaver does not start at all, and `tw.exe` cannot start it; the engines that run in a host stop too, because the host is a copy of that file. If it is `tw.exe`, only the terminal reader and the commands are blocked; the app still starts.

### On macOS, tw speak with avspeech says nothing

The `avspeech` engine depends on the program's main thread to keep feeding it work. The terminal reader does this, so reading with `avspeech` works there; `tw speak` does not, so with `--backend avspeech` it waits and then reports an error beginning "AVSpeechSynthesizer produced no audio".

- For `tw speak`, use `--backend nsspeech`.
- Or write the speech to a file with `--out`.

This conclusion comes from reading the code; it has not yet been tested on a real Mac.

### A key does nothing

1. Your terminal may not send that key. Terminals cannot send `Ctrl+H`, `Ctrl+I`, `Ctrl+M`, `Ctrl+J`, `Ctrl+Shift` with a letter, or `Ctrl` with digits and most punctuation, and some terminal programs reserve **F10**, **F11**, **Shift+Up**, and **Shift+Down** for themselves. The "Terminal notes" in the [keyboard reference](keyboard.md) list them.
2. Use the command's other key, if it has one. Chapters, for example, also move with **Alt+PageDown** and **Alt+PageUp**.
3. Run the command from the command palette: press **F2** and type its name.
4. Single-key shortcuts may be off. Press **F9**; you hear "Single-key shortcuts on."
5. Your screen reader may take the key first. See [Using textweaver with a screen reader](screen-readers.md).
6. Assign the command a key your terminal can send, in `keymap.toml`; textweaver warns about keys a terminal cannot send.

### The colors are hard to read

1. Press **F5** to switch to the next color theme; the title line and status line change immediately.
2. Pick a theme for one run with `--theme`, for example `--theme high-contrast`. `tw --help` lists the themes.
3. Turn color off by setting the `NO_COLOR` environment variable to `1`. textweaver still marks the highlight, the selection, and the cursor with bold, underline, or reverse video, never with color alone.
4. If colors look wrong, your terminal may be reporting its color support incorrectly. Set `TEXTWEAVER_COLOR` to `truecolor`, `256`, `16`, or `none`; it takes precedence over `NO_COLOR`.

The [themes guide](themes.md) explains themes and how to make your own.

### A PDF reads in the wrong order, or says it has no text

1. A scanned PDF is an image of its pages, and textweaver recovers the text by optical character recognition (OCR). If the document reads as a single sentence beginning "This PDF has no text layer", no recognition engine could run, and the sentence gives the reason. Run `tw ocr status` to see which engines are ready, and `tw ocr download` to fetch the English models (it asks first); other languages need Tesseract. See [Scanned pages](converting.md#scanned-pages-ocr).
2. textweaver reconstructs each page's reading order: columns from left to right, each column from top to bottom, omitting running heads and page numbers. Unusual layouts can defeat this, such as three-column magazines, tables without aligned columns, or lists whose bullets are pictures. Tagged PDFs, such as those saved from Word with accessibility tags, read most reliably.
3. To look at the text textweaver got, print it:

   ```bash
   tw text paper.pdf
   ```

4. A password-protected PDF is refused; remove the password in a PDF program first.

### "is not a text file"

textweaver refuses a file that is not text, such as a program, an audio file, or an old Word `.doc`, and the message says what the file appears to be. RTF, OpenDocument text, LaTeX, email, and saved web pages open directly, and PNG and JPEG pictures are read by recognizing their text.

1. Check that you opened the file you meant.
2. For an old Word `.doc`, Excel `.xls`, or PowerPoint `.ppt` file, you hear "it is an old Microsoft Office file". Open it in Word or LibreOffice, save it in a newer format such as `.docx`, and open that.
3. For a RAR archive, you hear "it is a RAR archive, which does not open". Extract it first, or repack it as ZIP or 7z, which open directly.
4. For a Textile, DocBook, or similar file, convert it to Markdown with Pandoc and open the result. LaTeX, Org, and reStructuredText open directly.

   ```bash
   tw convert report.textile --to md
   ```

The [converting guide](converting.md) explains `tw convert`.

### "Could not open" a document that has the right file name ending

A file can carry the right extension (`.odt`, `.rtf`, `.tex`, `.eml`, or `.mhtml`) and still be unreadable, for example a `.eml` that is really an export log, or a `.tex` file saved with the wrong encoding. textweaver names the kind of file it expected and says that the file may be damaged, plainly and without an operating system error code:

- "Could not open notes.odt: it is not a readable OpenDocument text file; it may be damaged."
- "Could not open report.tex: it is not a readable LaTeX file; it may be damaged."
- "Could not open message.eml: it is not a readable email message; it may be damaged."
- "Could not open page.mhtml: it is not a readable web archive; it may be damaged."

1. Open the file in the program that made it (Word, your mail program, your browser's "Save page as") and save it again.
2. For LaTeX, check that the file is plain text saved as UTF-8, not a binary format such as a compiled `.pdf` renamed to `.tex`.
3. If the file opens correctly elsewhere, the file itself is sound and the case is worth reporting; see [Report a bug](#report-a-bug).

### Bookmarks and notes for one document are gone, and there is a .bak file

When a document's state file cannot be read, for example after a sync conflict or a hand edit, textweaver does not overwrite it. It renames `<key>.json` to `<key>.corrupt-<time>.bak` in the state folder, where the time is in seconds since 1970, and the document starts with no saved place or marks. The event is recorded in the log but not announced.

1. Look in the state folder for the `.corrupt-` file. The log names it.
2. The file is JSON; open it in a text editor. Often a single missing comma or bracket is the cause.
3. Close textweaver, repair the file, and rename it back to `<key>.json`: the same name, with `.corrupt-`, the time, and `.bak` replaced by `.json`. If textweaver has already written a new `<key>.json` for that document, move that one away first.

A corrupt `settings.toml` is handled the same way: textweaver uses the default settings and says so: "Settings file was unreadable and has been reset to defaults", the reason, and where the backup was saved (`settings.toml.corrupt-<date>-<time>.bak`). One invalid value affects only that value; textweaver says "Some settings were invalid and use their defaults:" and names each one.

### Your place moved after the file changed elsewhere

When a document changes in another program (Obsidian, `git pull`, another editor), textweaver finds your place, bookmarks, notes, and highlights again from the text they were on, and says once what moved: "The file changed;" then what was moved to match, and what could not be found.

1. A mark that could not be found is flagged as not found and placed at the same proportional position in the document; its passage was rewritten or deleted.
2. Check the mark's list (**Shift+M** for bookmarks, **Shift+A** for notes) and move or delete it.

[Bookmarks, notes, and highlights](notes.md#when-the-file-changes-in-another-program) explains how marks are found again.

### Quitting says "Still saving"

Saving, positions, and notes are written in the background. If the disk is slow, a network drive for example, quitting waits up to ten seconds for the writing to finish and says "Still saving. Please wait." Let it finish; your work is being written.

### Sync folder missing

The status line says "Sync: folder missing, saving here", and Sync status (Shift+F5) reports the same: the sync folder chosen in Set up sync is not present. This is often expected, for example when a USB stick is unplugged or you are away from the network that hosts the folder.

1. Nothing is lost. Your places, notes, highlights, and bookmarks are saved on this computer as usual.
2. Plug the stick back in, or reconnect. textweaver rediscovers the folder by itself and sends what changed, with no restart needed. Tools, Sync, Sync now sends everything at once.
3. If the folder was moved or renamed, choose it again with Tools, Sync, Set up sync, or set `[sync] folder` in `settings.toml`.
4. If the folder is there but the status stays the same, check that the drive letter or mount point is the same as when you set sync up.

### Sync: newer format, read only

The status line says "Sync: newer format, read only": a newer textweaver on another computer wrote the sync folder in a format that this version does not know. This computer still reads what it can and writes nothing to the folder, so it cannot damage what the newer version wrote. Your changes here are saved on this computer but not sent.

1. Update textweaver on this computer to the version your other computers use. Sync then resumes by itself and sends what you did here.
2. Until then, "Continue reading", library search, and the statistics still use what this computer can read from the folder.
3. The same message appears when the folder's `textweaver-sync/format.json` is damaged. If every computer runs the same version, inspect that file: it should hold only the format number, such as `{"format": 1}`. Do not delete the folder, because the other computers' data is in it.

A single damaged file is a different case: the status says "Sync: 1 damaged file skipped", usually because a sync service had not finished copying the file. It is retried at the next change. See [Syncing between computers](sync.md#problems).

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

### An update does not install

1. "Update refused: checksum does not match. Nothing changed." means the download was damaged or altered in transit, and nothing was installed. Check for updates again to download it afresh.
2. "Not updated: this copy is not from a release package." means this copy was built from source or installed by a system package manager; update it the way you installed it.
3. On Windows an update installs when textweaver closes. If nothing seems to happen, close every textweaver window and wait a moment. Neither `tw update` nor the app touches your settings and notes.
4. If textweaver cannot reach GitHub, the daily check says nothing and tries again the next day. Help, Check for updates gives the reason, and `tw update --check` in a terminal gives the same answer in words.

[Updates](updates.md) explains each step.

### A component will not download

1. Run `tw components list` to see what is installed and what each component is for.
2. A download that stops resumes from where it stopped the next time. "Not installed: a file did not match" means the file was damaged or is not the pinned one; fetch it again, or check your own components source.
3. Behind a firewall, or offline, install the component from a zip file or a folder instead (`tw components install ID PATH`), or set a mirror. [Optional components](components.md) has the steps.
4. Where a helper program has no build for your computer (liblouis on Linux and macOS, ffmpeg on macOS), the message names the command that installs it from your system's packages.

### Speech Dispatcher does not work in a container or over SSH

The `speechd` engine communicates with the Speech Dispatcher server through a socket.

1. textweaver looks for the socket in the places Speech Dispatcher's own programs do: `SPEECHD_ADDRESS` if set, else in `XDG_RUNTIME_DIR`, else, where there is none (a container, or a session without a login manager), in `~/.cache/speech-dispatcher/`.
2. If nothing answers, textweaver starts the server itself and tries for five seconds. Check that `speech-dispatcher` is installed and that `spd-say hello` works in the same shell.
3. A container has no sound device of its own. Route its sound to the host, as the [Docker guide](dev/docker.md) describes, or test speech without a device: `TEXTWEAVER_ESPEAK_OUTPUT=virtual` makes eSpeak NG keep real timing without playing anything.
4. Run `scripts/speech-check.sh`. On Linux it reports on Speech Dispatcher, the sound server, and the session.

### Speech stops in the middle, or you hear "Speech restarted"

textweaver restarts an engine that crashes, or that plays nothing for 12 seconds, and reads on from the last word, announcing "Speech restarted:", the reason, and "Reading on from the last word." If the failure recurs at once, reading stops with a message ending "Check the audio device".

1. Check that the sound device is connected and not in use by another program.
2. Read the log for the reason.
3. Try another engine with `--backend` to determine whether the engine or the device is at fault.
4. If you chose a device in `[speech] output_device`, run `tw backends --devices`; its last line says when that device is not connected, and speech then plays on the default.

### Speech plays on the wrong device

Speech plays on the system's default sound device. When a headset or other device disappears, textweaver moves speech to the current default immediately. To keep speech on one device, select it by its id: `tw backends --devices` lists them, and the [speech guide](speech.md#choose-the-sound-device) has the steps. eSpeak NG, Speech Dispatcher, and the Apple voices always play on the default; choose the device in the system's own sound settings for them.

If you hear "Speech stopped working", textweaver restarts speech once on its own. If it fails again, press **Shift+F8** (Restart speech), or restart textweaver.

## Report a bug

A good report allows someone else to reproduce the problem. Include:

1. What you did, what you expected, and what happened instead. Quote what textweaver said, from the status line.
2. textweaver's version. In the app or the terminal reader, choose About from the command palette (in the app, also from the Help menu): it says the version. From a command line:

   ```bash
   tw --version
   ```

3. The doctor report: run the doctor script with `--out doctor.txt` (or `-Out doctor.txt` in PowerShell), as shown above, and attach the file.
4. The log: start the reader with `--log debug`, reproduce the problem, quit, and attach `textweaver.log` from the state folder (`tw settings path` names it). For the app, start it with `--log-file gui.log` and attach that file.
5. If a document triggers the problem and you may share it, attach the document or a small excerpt that shows the problem.

Report bugs at the project's issue tracker: [github.com/leavesofgrass/textweaver/issues](https://github.com/leavesofgrass/textweaver/issues).

## See also

- [Using textweaver with a screen reader](screen-readers.md): keys, double speech, and terminals.
- [Speech engines and voices](speech.md): engines, voices, and every speech setting.
- [Installing textweaver](install.md): packages, SmartScreen, and Gatekeeper.
- [Updates](updates.md) and [Optional components](components.md): when an update or a download fails.
- [Keyboard reference](keyboard.md): every key, and the terminal notes.
- [Scripts](../scripts/README.md): the doctor, the speech check, and the install scripts.
- [Documentation index](README.md)
