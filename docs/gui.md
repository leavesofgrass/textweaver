# The textweaver window (GUI)

textweaver has two readers: the terminal reader, `textweaver`, and a window. They share everything that matters: the documents, the keys, the settings, the notes, and the voices. This page covers what is different about the window.

The window is written entirely in Rust (Xilem's Masonry widgets, Vello drawing, Parley text, and AccessKit for screen readers). It is new in this alpha. It reads, and since this alpha it edits too; see [Editing](#editing).

Its crate and source binary are named `textweaver-xilem`; a downloaded [GUI package](install.md#the-gui) installs it as `textweaver-gui`. This guide uses `textweaver-xilem` for the command, since that is what `cargo build` produces; if you installed a release package, run `textweaver-gui` instead wherever this guide says `textweaver-xilem`.

## Starting it

```sh
textweaver-xilem path/to/document.md
```

From a release package: `textweaver-gui path/to/document.md`.

With no document, it opens empty and says which key opens one (Ctrl+O).

Useful options:

- `--read`: start reading once the document is open.
- `--theme NAME`: use this theme instead of the saved one, for this run.
- `--voice ID` and `--backend ID`: the voice or speech engine for this run (see `tw voices` and `tw backends`).
- `--no-speech`: run silently.
- `--announce live` or `--announce uia`: how messages reach your screen reader, for this run (see [Announcements](#announcements)).
- `--select-spoken`: while reading, select the spoken word instead of only moving the caret to it (see [The spoken word](#the-spoken-word)).
- `--home FOLDER`: keep settings and reading positions in this folder, as `TEXTWEAVER_HOME` does.
- `--graphics API`: draw with one graphics API only: `vulkan`, `dx12` (Windows), `metal` (macOS), or `gl`; `auto`, the default, lets the graphics library use every one it finds. On the development machine `vulkan` used about 26 MB less memory, but this depends on your graphics driver. To keep a choice, put `graphics = "vulkan"` in the `[gui]` section of `settings.toml`.
- `--log` or `--log-file PATH`: write what the window announces and does, for a bug report.

`textweaver-xilem --help` lists every option.

On Windows the window opens with no console window beside it. Started from a terminal, `--help`, `--version`, and errors still appear in that terminal. PowerShell does not wait for a windowed program, so its output may come after the next prompt; `textweaver-xilem --help | Out-Host` waits for it. Started from a shortcut or File Explorer, a startup error is shown in a message box, and `--log-file PATH` keeps it in a file too.

## What is in the window

From top to bottom:

1. **The header,** a banner with the document's title and five buttons: Open, Font, Edit (or Finish editing), Settings, and Commands.
2. **The document,** one control your screen reader reads as a document. Arrow keys, Home, End, Page Up, and Page Down move the caret, with Shift to select and Ctrl for words, paragraphs, and the document's ends. Ctrl+C copies the selection. Every other key goes to textweaver's keymap, so the browse keys of NVDA and JAWS work here too: `h` for the next heading, `t` for the next table, `k` for the next link, and so on.
3. **The RSVP strip,** only while RSVP is on (Alt+Shift+R). It shows one word at a time under the document, so it never covers the text or the caret.
4. **The toolbar,** named "Reading": Play or Pause, Stop, Previous sentence, Next sentence, Slower, and Faster.
5. **The status bar:** the last message, then what the terminal's title line shows: the reading state, "line 3 of 40, 7%", the accessibility mode, the rate, and the speech engine.

Every button has a key, shown on screen with its name, for example "Open… (Ctrl+O)". Your screen reader reads it as the button's shortcut key: NVDA and JAWS say it after the name when their setting for reporting shortcut keys is on (in NVDA, Object Presentation, "Report object shortcut keys"). The name itself is only the label, "Open", so it stays short. The key comes from the keymap, so a key you change in `keymap.toml` shows here too, and F1 and the command palette list every key. While single-key shortcuts are on, a button shows its single key ("Play (Space)"); press F9 to turn them off, and the buttons show their chords instead ("Play (Ctrl+Shift+Space)").

Tab and Shift+Tab move between the document and the buttons. Dialogs (settings, lists, the command palette) open inside the window and take the focus; Escape closes them and puts you back in the document.

## Opening a document

Open (Ctrl+O) shows your system's own file chooser: on Windows the standard Open dialog, which NVDA and JAWS know. It lists the documents textweaver reads; choose "All files" in the file type list to see everything. It starts in the folder of the document you have open. When you choose a file, the dialog closes, textweaver says "Opened" and the title, and the focus is back in the document. Escape cancels.

To type a path instead, press Ctrl+Shift+G (Open Path): a one-line prompt where Tab completes the path and Up and Down recall earlier ones. If the system's file chooser cannot open (on Linux it needs the XDG desktop portal), textweaver says so and shows this prompt instead.

## Text size and font

- **Ctrl+Plus** (Ctrl+=, or the plus key on the number pad): larger text.
- **Ctrl+Minus**: smaller text.
- **Ctrl+0**: back to the standard size, 14 points.
- **Ctrl+D**, or the Font button: the font list. The fonts that come with textweaver are first, marked "built in": Atkinson Hyperlegible Next, Atkinson Hyperlegible Mono, and OpenDyslexic. Then your installed fonts. Enter uses the font at once.

Each change is said, for example "Text size 18 points." or "Font: OpenDyslexic.", and saved in `[reading_aids.font]` (see [Settings](settings.md)). The size steps one point at a time around the usual sizes and more quickly above 16 points, from 8 up to 72 points. The Settings dialog changes the same settings, under "Reading aids".

## Keys

The window uses the same keymap as the terminal reader, with a few chords the terminal cannot send. The [keyboard reference](keyboard.md) lists every key, with a column for the GUI. The ones you will use most:

- **Space** (browse) or **Ctrl+Shift+Space**: play or pause.
- **Escape**: stop.
- **Alt+Down** and **Alt+Up**: next and previous sentence.
- **Ctrl+O**: open a document with the system's file chooser. **Ctrl+Shift+G**: type its path instead.
- **Ctrl+Plus**, **Ctrl+Minus**, **Ctrl+0**: text size. **Ctrl+D**: the font list.
- **F11** and **Shift+F11**: faster and slower (or **+** and **-** in browse). In the window, Ctrl+= and Ctrl+- size the text instead of the rate.
- **Ctrl+Shift+V**: the voice manager (see [Voices](#voices)).
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

## Editing

Ctrl+E, or the Edit button, turns edit mode on, as in the terminal reader: you edit the document's source (a Markdown file's Markdown; for other formats, the Markdown made from them, which Save stores as a new `.md` file). Ctrl+E again finishes, asking to save if there are changes.

In edit mode the document is a multi-line edit, so NVDA and JAWS switch to focus mode by themselves.

- **Typing** goes in at the caret, and over the selection if there is one. Enter starts a new line (and continues a list). Backspace and Delete delete. Input methods and dictation work too.
- **Your screen reader echoes** what you type, and reads the caret and the selection as they move. In the self-voicing mode, textweaver says them itself, as the terminal does: typing as the typing echo setting says (Shift+F9 cycles it), the character, word, or line the caret moves to, and what a Shift key added to the selection or took from it.
- **Undo** is Ctrl+Z, **redo** Ctrl+Y or Ctrl+Shift+Z, and each says what it undid. The editing keys are the terminal's: Ctrl+B bold, Ctrl+I italic, Ctrl+K a link, Ctrl+Alt+1 a heading, and the rest in the [keyboard reference](keyboard.md). Ctrl+S saves.
- **Tab** types a tab, or in a table moves to the next cell (Shift+Tab to the previous one), as in the terminal. **Ctrl+Tab** moves the focus out of the document, to the buttons.
- **Spell check:** Alt+M moves to the next misspelled word and selects it, so your screen reader says it and textweaver spells it. Type to replace it, or press Alt+J for suggestions. Alt+Shift+M goes back. Misspelled words are also marked on screen with a dotted underline, shortly after you stop typing (in documents up to a million characters).
- **Citations while writing:** Alt+C opens the citation picker. Type part of an author or title to filter, Enter inserts it, and textweaver asks for a page or other locator. Alt+Shift+D adds a reference by DOI or ISBN.
- **Export and preview:** the command palette (F2) has Export as a web page, PDF, Word, EPUB, and braille (BRF), each written next to the document, and Preview in the browser, which reloads when you save.
- Ctrl+Tab and Ctrl+Shift+Tab move between the document and the buttons, so you are never trapped in the edit; Ctrl+E is always the way out of edit mode.

## The spoken word

While textweaver reads, the spoken word has its own background color, and the caret sits at its start, so your screen reader and Braille display follow the reading. This is the default, chosen after the first screen reader session. `--select-spoken` selects the word instead, for anyone who prefers it.

The document window: a very long document is shown a few hundred pages at a time, around where you are. When reading reaches the edge, the window moves on by itself. The text that stays keeps its place, so your screen reader does not lose it.

## Questions

When textweaver asks a yes-or-no question (a voice to download, after its size and licence; a voice to remove; a file changed on disk), the window shows it as a small dialog: the question is the dialog's name, so your screen reader says it, and the focus is on **Yes**. Press **Y** or **N**, as in the terminal, or Tab to **No** and press Enter. Escape answers no. Any other key asks the question again.

## Voices

**Ctrl+Shift+V** opens the voice manager, the same list as the terminal's. The first two rows filter it: Enter on "Language" or "Engine" cycles through the choices. Enter on a voice uses it and speaks a sample; on a voice you can download, textweaver reads its licence and size, then asks before downloading. Space marks a favourite; Delete removes a downloaded voice, after a yes.

## Language

The window's own labels (the buttons, the settings dialog, the hints) follow the interface language, `[interface] language` in `settings.toml`, as textweaver's messages do. Change it in Settings, under "Interface", and the window relabels itself at once.

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
- **Syllables** (Alt+Shift+Z): long words drawn split into syllables with a middle dot, "read·a·bil·i·ty", as in the terminal. The dot is only drawn: the words keep their letters, so your screen reader and Braille display read "readability", and the caret and the spoken word stay where they were. The separator and when words are split are in `[reading_aids.syllable_options]`.
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
- [ADR-0027: Xilem GUI](adr/0027-xilem-gui.md), [ADR-0028: the Xilem GUI after the first listening session](adr/0028-xilem-gui-after-the-session.md), and [ADR-0033: the GUI after further accessibility testing, and edit mode](adr/0033-gui-session-2-and-edit-mode.md)
