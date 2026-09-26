# Using textweaver with a screen reader

This guide is for people who use a screen reader, such as JAWS, NVDA, VoiceOver, or Orca, and want to use textweaver's terminal reader alongside it. It explains the two ways to work, how to avoid hearing everything twice, what your screen reader can read, which terminals to use, and which keys may clash. It also covers braille displays and the GUI preview.

textweaver's keys are the same whichever way you work. See [Reading and moving around](reading.md) and the [keyboard reference](keyboard.md).

What this guide says about textweaver comes from its code. Which combinations of screen reader, terminal, and system have been tried by ear is stated for each; where it says untested, please tell us how it goes (see [Troubleshooting](troubleshooting.md), "Report a bug").

## Two ways to work

### Self-voicing: textweaver speaks

This is the default. textweaver reads documents aloud with its own voice, highlights each word, and speaks every announcement: "Paused.", "Heading level 2: Methods", "No next heading.", and so on.

Choose this when you want textweaver's voice, rate, and word highlighting for long reading, such as Eloquence at 400 words per minute, while your screen reader stays at its own settings for everything else.

### --no-speech: your screen reader speaks

```bash
textweaver --no-speech essay.md
```

`tw open --no-speech essay.md` does the same. With `--no-speech`, textweaver says nothing at all. It neither reads aloud nor speaks announcements. Your screen reader does all the talking, from what textweaver puts on the screen. The title line shows `silent` as the speech engine.

Choose this when you want one voice for everything, or when you use a braille display.

## Avoid hearing things twice

In self-voicing mode textweaver speaks each announcement, and also writes it on the status line. A screen reader that reads new text in a terminal will read the status line too, so you hear it twice. There are two ways out.

- **Let textweaver talk, and quiet your screen reader in the terminal.** Most screen readers have a command to stop speaking for a while. In NVDA, **NVDA+S** changes the speech mode. In JAWS, **Insert+Space**, then **S**, turns speech off and on. NVDA can also stop reading new text in terminals: **NVDA+5** turns "report dynamic content changes" off and on. Turn your screen reader back on when you leave textweaver. Check your screen reader's own documentation; these commands belong to it, not to textweaver.
- **Let your screen reader talk, and start textweaver with `--no-speech`.** You then hear each announcement once, in your screen reader's voice.

While textweaver reads aloud, the terminal's cursor moves to each word as it is spoken. If your screen reader speaks as the cursor moves, it will talk over the reading. Quiet it while textweaver reads, or use `--no-speech`.

## What your screen reader can read with --no-speech

textweaver is built so that everything it would say is also on the screen.

- **The status line.** It is the line above the bottom line. Every announcement goes there: moves, modes, settings, errors, and questions such as "Quit textweaver? y or n". Screen readers that read new text in a terminal read it as it changes. While a yes-or-no question waits, it stays on the status line in front of any other message.
- **Repeated messages.** A terminal screen reader speaks the status line only when it changes. When the same message comes twice in a row, such as "No next heading." after pressing **h** twice at the end, textweaver blanks the status line for 150 milliseconds first, so the message changes and is read again.
- **The cursor.** textweaver parks the terminal's cursor where your attention is: on the chosen item of a list, on the caret of a prompt, on the Speech Cursor line, on the word being read, or else on the reading cursor. Screen readers, braille displays, and magnifiers that follow the cursor follow it there.
- **Lists.** The help, the keyboard shortcuts, bookmarks, notes, the library, and the voices appear in a box over the document. When a list opens, the status line says what it is and how to use it, followed by the first item. As you press **Up** and **Down**, the status line shows the chosen item, and the cursor sits on it.
- **The prompt line.** The bottom line shows the prompt and what you type, with the cursor at the caret. Find, Go to, Open file, the command palette, and the note prompt all use it. Typing is not echoed by textweaver with `--no-speech`; your screen reader echoes it as usual.

What does not reach the status line:

- **Reading aloud.** **Space**, **Enter**, **s**, **w**, and the other keys that read text make no sound, and the text is not copied to the status line. Read the document with your screen reader's own review commands instead.
- **Speech Cursor lines.** In Speech Cursor mode, **Up** and **Down** move the cursor to the next line, but the line's text is not put on the status line. Use your screen reader's line review instead. Speech Cursor mode is of little use with `--no-speech`.

What does reach it:

- **Moves.** After a sentence, paragraph, heading, table, link, find, go to, or bookmark jump, the status line shows where you arrived, with a preview of the text.
- **Caret keys.** **Right** and **Left** put the word on the status line; **Down** and **Up** put the whole line there. **c** puts the character's name there.
- **Where am I.** **%** puts the line, the percentage, and the heading on the status line.

How much textweaver says is set by `[speech] verbosity`. See [Reading and moving around](reading.md).

## Terminals

textweaver runs in any terminal that sends key presses in the usual way.

### Windows

textweaver runs in Windows Terminal and in the classic console (conhost, the window `cmd` opens by default).

- Windows Terminal keeps some keys for itself by default, such as **F11** (full screen) and **Alt+Enter**. Use **Alt+PageDown** and **Alt+PageUp** for chapters instead of **F11** and **F10**.
- Paste with **Ctrl+V** in Windows Terminal, or with right-click in either.

Which of the two works better with JAWS and NVDA has not been recorded yet.

### macOS

textweaver runs in Terminal.app with VoiceOver. This has not been tested on a real Mac yet.

Terminal.app normally uses the Option key to type special characters. textweaver's **Alt** chords, such as **Alt+P** and **Alt+V**, need Option to act as Alt. Turning on "Use Option as Meta key" in Terminal's profile settings is expected to do this; this too is untested.

### Linux

textweaver runs in GNOME Terminal and other terminals. Its use with Orca has not been tested yet.

## Keys that may clash

- **The screen reader key.** NVDA, JAWS, and Orca use **Insert**, or **Caps Lock** if you choose, as their own modifier key. textweaver uses neither, so your screen reader's commands pass through to it as usual.
- **Single-letter keys.** textweaver's reading keys are single letters and punctuation, such as `.`, `h`, and `t`, like a screen reader's browse mode in a web page. Screen readers do not use their own browse mode in a terminal, so these keys reach textweaver. If a letter does nothing, check that your screen reader is not in a special mode, such as a virtual or review mode.
- **Dictation and speech recognition.** Dictation software types letters, and a letter is a command in textweaver. Press **F9** to turn single-key shortcuts off. You hear "Single-key shortcuts off." Chords with **Ctrl** or **Alt**, the arrow keys, the function keys, and the command palette (**F2**) keep working. Press **F9** again to turn them back on.
- **VoiceOver** uses **Control+Option** as its modifier. textweaver's terminal keys use no **Ctrl+Alt** chords, so there is no clash by default.

Any key can be changed in `keymap.toml`. The [keyboard reference](keyboard.md) explains how, and lists what terminals cannot send.

## Braille displays

textweaver draws no braille of its own. A braille display shows what your screen reader shows: the line at the terminal's cursor, and new text as it appears. With `--no-speech`, the status line and the cursor position described above are what reach the display. Which screen reader and display combinations work well has not been tested yet.

## The GUI preview

textweaver also has an early native window, the GUI. It is a preview, not yet part of the release packages, and a plain `cargo build` does not build it.

- It uses native controls: the document is in a read-only text box whose caret follows the spoken word, so your screen reader reads it with its usual keys.
- Self-voicing is off by default in the GUI: your screen reader speaks the announcements. Start it with `--self-voicing` to have textweaver speak them too, for use without a screen reader. Reading aloud works either way; `--no-speech` turns that off too.
- Announcements are sent to your screen reader as UI Automation notifications on Windows, as accessibility announcements on macOS, and as ATK notifications on Linux.

On Windows, the notifications and the text box were checked through UI Automation; NVDA speaks such notifications, and JAWS has not been tried. VoiceOver has not been tried on a real Mac, and the Linux version has not been built. [ADR-0014](adr/0014-gui-toolkit.md) records what was checked.

## If something goes wrong

- **Everything is said twice.** See [Avoid hearing things twice](#avoid-hearing-things-twice).
- **Your screen reader does not read the status line.** Turn on its reading of new text in terminals. In NVDA that is "report dynamic content changes", **NVDA+5**.
- **A key does nothing.** See [Troubleshooting](troubleshooting.md), "A key does nothing".
- **No speech from textweaver.** Check the title line: `silent` means `--no-speech` or no engine. See [Troubleshooting](troubleshooting.md).

## See also

- [Reading and moving around](reading.md): every reading key, and the command line options.
- [Keyboard reference](keyboard.md): every key, and what terminals cannot send.
- [Troubleshooting](troubleshooting.md): common problems and how to report a bug.
- [ADR-0006: Keymap, actions, and announcements](adr/0006-keymap-and-actions.md): how announcements reach the status line.
- [ADR-0014: GUI toolkit](adr/0014-gui-toolkit.md): the GUI preview and what was checked.
- [Documentation index](README.md)
