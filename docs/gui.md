# The textweaver window (GUI)

textweaver has two readers: the terminal reader, `textweaver`, and a window, `textweaver-xilem`. They share everything that matters: the documents, the keys, the settings, the notes, and the voices. This page covers what is different about the window.

The window is written entirely in Rust (Xilem's Masonry widgets, Vello drawing, Parley text, and AccessKit for screen readers). It is new in this alpha, and edit mode comes in a later version. For writing, use the terminal reader for now.

## Starting it

```sh
textweaver-xilem path/to/document.md
```

With no document, it opens empty and says which key opens one (Ctrl+O).

Useful options:

- `--read`: start reading once the document is open.
- `--theme NAME`: use this theme instead of the saved one, for this run.
- `--voice ID` and `--backend ID`: the voice or speech engine for this run (see `tw voices` and `tw backends`).
- `--no-speech`: run silently.
- `--announce live` or `--announce uia`: how messages reach your screen reader, for this run (see [Announcements](#announcements)).
- `--select-spoken`: while reading, select the spoken word instead of only moving the caret to it (see [The spoken word](#the-spoken-word)).
- `--home FOLDER`: keep settings and reading positions in this folder, as `TEXTWEAVER_HOME` does.
- `--log` or `--log-file PATH`: write what the window announces and does, for a bug report.

`textweaver-xilem --help` lists every option.

## What is in the window

From top to bottom:

1. **The header,** a banner with the document's title and four buttons: Open, Fonts, Settings, and Commands. Each button says its key.
2. **The document,** one control your screen reader reads as a document. Arrow keys, Home, End, Page Up, and Page Down move the caret, with Shift to select and Ctrl for words, paragraphs, and the document's ends. Ctrl+C copies the selection. Every other key goes to textweaver's keymap, so the browse keys of NVDA and JAWS work here too: `h` for the next heading, `t` for the next table, `k` for the next link, and so on.
3. **The RSVP strip,** only while RSVP is on (Alt+Shift+R). It shows one word at a time under the document, so it never covers the text or the caret.
4. **The toolbar,** named "Reading": Play or Pause, Stop, Previous sentence, Next sentence, Slower, and Faster.
5. **The status bar:** the last message, then what the terminal's title line shows: the reading state, "line 3 of 40, 7%", the accessibility mode, the rate, and the speech engine.

Tab and Shift+Tab move between the document and the buttons. Dialogs (Open, settings, lists, the command palette) open inside the window and take the focus; Escape closes them and puts you back in the document.

## Keys

The window uses the same keymap as the terminal reader, with a few chords the terminal cannot send. The [keyboard reference](keyboard.md) lists every key, with a column for the GUI. The ones you will use most:

- **Space** (browse) or **Ctrl+Shift+Space**: play or pause.
- **Escape**: stop.
- **Alt+Down** and **Alt+Up**: next and previous sentence.
- **Ctrl+O**: open a document.
- **Ctrl+,**: settings.
- **F2**: the command palette, every command by name.
- **F1**: help. In a list, F1 repeats the list's introduction.
- **Alt+End**: say the last message and the status. In a list, it repeats the list's introduction too.
- **Alt+'**: say the last message again.
- **Alt+O**: the outline. Type to filter the headings, Enter jumps to one.
- **Ctrl+Shift+N**: the notes list.
- **Ctrl+T** and **Ctrl+Shift+T**: next and previous table. **Ctrl+Alt+arrows** move by cell in a table.
- **k** and **Shift+K** (browse): next and previous link. **Alt+Shift+F** follows a link.
- **Alt+Shift+A**: the accessibility mode: self-voicing, hybrid, or screen reader.
- **F5**: the next color theme.
- **F9**: single-key shortcuts off or on.

## The spoken word

While textweaver reads, the spoken word has its own background color, and the caret sits at its start, so your screen reader and Braille display follow the reading. This is the default, chosen after the first screen reader session. `--select-spoken` selects the word instead, for anyone who prefers it.

The document window: a very long document is shown a few hundred pages at a time, around where you are. When reading reaches the edge, the window moves on by itself. The text that stays keeps its place, so your screen reader does not lose it.

## Announcements

textweaver's messages ("Paused.", "Reading at 300 words per minute.") reach your screen reader in one of two ways:

- **A live region** (the default). NVDA and JAWS both speak it.
- **UI Automation notifications** (Windows only). Choose it in Settings, under "Window", or with `--announce uia` for one run.

The setting is `announce` in the `[gui]` section of `settings.toml`; see [Settings](settings.md#gui).

## Reading aids

The window draws the same [reading aids](reading-aids.md) as the terminal, with the same keys:

- **Text spacing:** line height, paragraph spacing, and letter and word spacing, in `[reading_aids.spacing]`. The window uses the exact values; the terminal rounds them to whole rows and spaces.
- **The reading ruler** (Alt+Shift+U): off, the current line, or the ruler. The reading line gets a band with a bar at its start, the lines around it a paler band, and with `mask_outside` the rest is dimmed. It follows the caret, and the spoken word while reading.
- **Bionic reading** (Alt+Shift+B): the start of each word in bold.
- **Difficult words** (Alt+Shift+J): underlined with a thick line, never marked by color alone.
- **RSVP** (Alt+Shift+R, then Alt+Shift+P to play): one word at a time in its own strip under the document. The word before and after sit to its left and right. The marked letter is bold and underlined as well as colored. RSVP's nine places move the word left, center, or right in the strip.

All of these change only how text looks. Your screen reader reads the same text either way. The RSVP word is hidden from screen readers, so it is never spoken by itself. Beside it is a quiet status ("RSVP paused, word 120 of 900") that you can find with your screen reader's review or object navigation.

## Settings

Settings (Ctrl+,) opens a dialog: the sections on the left, the chosen section's settings on the right. Every change takes effect and is saved at once. It is built from the same list as the terminal's settings screen, so every setting is in both; [Settings](settings.md) describes each one.

## For testers

- `--background` starts the window without taking the focus, off screen, with no taskbar button, for automated checks.
- `--backend paced` reads silently, timing words like a real engine.
- `crates/textweaver-xilem/tools/uia-report.ps1` reports what UI Automation sees (Windows); `-WindowEdge` reads past the document window's edge. `tools/atspi-check.sh` does the same with AT-SPI on Linux.
- `--review-screenshots FOLDER` draws the review screenshots (three themes, 100% and 200%, the dialogs, and the reading aids) without a window.

## See also

- [Keyboard reference](keyboard.md)
- [Reading aids](reading-aids.md)
- [Using textweaver with a screen reader](screen-readers.md)
- [ADR-0027: Xilem GUI](adr/0027-xilem-gui.md) and [ADR-0028: the GUI after the first screen reader session](adr/0028-xilem-gui-after-the-session.md)
