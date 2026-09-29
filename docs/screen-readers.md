# Using textweaver with a screen reader

This guide is for people who use a screen reader, such as JAWS, NVDA, VoiceOver, or Orca, and want to use textweaver's terminal reader alongside it. It explains the three accessibility modes, how to stop hearing things twice, what your screen reader can read, the settings to try in NVDA and JAWS, which terminals to use, and which keys may clash. It also covers braille displays, with a checklist for a 40-cell display, and the GUI, and ends with a checklist for trying each mode.

textweaver's keys are the same in every mode. See [Reading and moving around](reading.md) and the [keyboard reference](keyboard.md).

What this guide says about textweaver comes from its code. Screen reader and terminal settings are marked **not yet verified** until they have been tried by ear. Where something is untested, please tell us how it goes (see [Troubleshooting](troubleshooting.md), "Report a bug").

## Three modes

textweaver can speak with its own voice, write on its status line, or both. A screen reader that reads new text in a terminal speaks the status line too. The accessibility mode decides which of the two each kind of output uses, so you hear it once.

### Self-voicing

textweaver speaks everything with its own voice: reading, messages such as "Paused." or "Heading level 2: Methods", typing echo, and the word or line a caret key reaches. Each message is also written on the status line.

This is the default, and it is what textweaver always did. Choose it when you use textweaver without a screen reader, or when you quiet your screen reader in the terminal (see [Avoid hearing things twice](#avoid-hearing-things-twice)).

### Hybrid

textweaver reads documents aloud: continuous reading, and reading a character, word, sentence, line, paragraph, or selection. That reading includes the narration only textweaver can do: math in words, table rows with their headers, citations, and structure such as "heading level 2" and "list with 3 items".

Your screen reader does the rest from the status line and the cursor: messages, typing echo, and caret moves. textweaver does not speak those, and it does not copy what it reads aloud onto the status line.

Hybrid is recommended when a screen reader is running. You keep textweaver's voice and rate for long reading, and your screen reader's voice for everything else.

### Screen reader

textweaver never speaks on its own. Everything goes to the status line, written for your screen reader:

- Messages, moves, and questions, as in the other modes.
- The word, line, or character a caret key or a "say" key reaches.
- Text you ask to hear, such as the sentence (`.`) or the line (**Alt+Shift+L**), narrated as textweaver would say it: math in words, and tables with their headers.

Continuous reading still works. By default, **Space** moves through the text a sentence at a time. Each sentence goes to the status line for your screen reader, the cursor moves with it, and the next one follows after the time textweaver's rate allows. Press **+** or **-** to match the pace to your screen reader. **Space** pauses and resumes, and **Escape** stops. To have textweaver's own voice read instead, set `say_all = "voice"`; that is the only thing it then says.

You can also move through the text yourself, by sentence (**Alt+Down**, or **Alt+.**), line (**Down**), or word (**Right**), and your screen reader reads each place from the status line.

### Choosing a mode

- **Alt+Shift+A** cycles self-voicing, hybrid, and screen reader, and saves your choice. The new mode is announced the way the old one spoke, so you hear it.
- `--mode hybrid`, `--mode screen-reader`, or `--mode self-voicing` on the command line sets the mode for one run without saving it. `tw open --mode hybrid essay.md` works too.
- `--no-speech` starts textweaver with no voice at all, in screen-reader mode.
- In `settings.toml`:

```toml
[accessibility]
mode = "hybrid"
```

The title line shows `hybrid` or `screen reader mode` when one of those is in effect.

### The first run with a screen reader

The very first run starts with the list of interface languages instead (see [Language](#language)); the question below comes on the next start.

When textweaver starts, the mode has never been chosen, and a screen reader is running, it asks once: "NVDA is running. Use hybrid mode, where textweaver reads documents aloud and your screen reader speaks messages and typing? y or n". The question stays on the status line until you answer. **y** switches to hybrid and saves it. **n** or **Escape** keeps self-voicing. Either way it is not asked again; set `hybrid_offered = false` under `[accessibility]` to be asked again.

How textweaver notices a screen reader:

- Windows: the system's screen reader flag, which NVDA, JAWS, and Narrator set, and the running programs `nvda.exe`, `jfw.exe` (JAWS), and `narrator.exe`, which also give its name.
- macOS: VoiceOver's setting (`defaults read com.apple.universalaccess voiceOverOnOffKey`).
- Linux: GNOME's screen reader setting, the accessibility bus's "screen reader enabled" flag, or a running `orca`.

If textweaver gets it wrong, set `TEXTWEAVER_SCREEN_READER=0` to say there is none, or `TEXTWEAVER_SCREEN_READER=NVDA` (any name) to say there is one.

## Avoid hearing things twice

- **Use hybrid mode** (Alt+Shift+A). textweaver reads; your screen reader speaks the rest.
- **Use screen-reader mode** for one voice for everything, or with a braille display.
- **Stay in self-voicing and quiet your screen reader** while you are in textweaver. NVDA's speech modes (**NVDA+S**) include "on demand" in recent versions, which speaks only when you ask, for example to read the current line. JAWS has a way to turn speech off (**Insert+Space**, then **S**) and, in recent versions, a Speech on Demand setting. These belong to your screen reader; check its own documentation. **Not yet verified.**

While textweaver reads aloud, the terminal's cursor moves to each word. If your screen reader speaks as the cursor moves, it talks over the reading. Two settings help:

- `quiet_screen = true` keeps the screen still while textweaver reads continuously: the title line's position stops updating, and the text being read is not copied to the status line.
- `cursor = "status"` parks the cursor on the status line instead of on the spoken word.

```toml
[accessibility]
quiet_screen = true
cursor = "status"
```

With `cursor = "status"`, the cursor waits at the start of the status line, as it did in Star, so your screen reader's "read current line" (**NVDA+Up**, JAWS **Insert+Up**) repeats the last message. A prompt, such as Find, still puts the cursor at its caret, where you type.

## What your screen reader can read

textweaver is built so that everything it would say is also on the screen, in screen-reader mode, and for the parts left to your screen reader in hybrid mode.

- **The status line** is the line above the bottom line. Messages go there: moves, modes, settings, errors, and questions such as "Quit textweaver? y or n". Screen readers that read new text in a terminal read it as it changes. While a yes-or-no question waits, it stays on the status line in front of any other message.
- **Math** on the status line is written in words for a screen reader, as textweaver would say it: `$x^2$` becomes "x squared".
- **Repeated messages.** A terminal screen reader speaks the status line only when it changes. When the same message comes twice in a row, such as "No next heading." after pressing **h** twice at the end, textweaver blanks the status line for 150 milliseconds first, so the message changes and is read again.
- **The cursor.** With `cursor = "follow"`, the default, textweaver parks the terminal's cursor where your attention is: on the chosen item of a list, on the caret of a prompt, on the Speech Cursor line, on the word being read, or else on the reading cursor. Screen readers, braille displays, and magnifiers that follow the cursor follow it there.
- **Lists.** The help, the keyboard shortcuts, bookmarks, notes, the library, and the voices appear in a box over the document. When a list opens, the status line says what it is and how to use it, followed by the first item. As you press **Up** and **Down**, the status line shows the chosen item.
- **The prompt line.** The bottom line shows the prompt and what you type, with the cursor at the caret. Find, Go to, Open file, the command palette, and the note prompt all use it. In hybrid and screen-reader modes textweaver does not echo typing; your screen reader does.
- **Moves.** After a sentence, paragraph, heading, table, link, find, go to, or bookmark jump, the status line shows where you arrived, with a preview of the text.
- **Caret keys.** **Right** and **Left** put the word on the status line; **Down** and **Up** put the whole line there. **c** puts the character's name there.
- **Where am I.** **Shift+W** puts the line, the percentage, and the heading on the status line.

How much textweaver says is set by `[speech] verbosity`. See [Reading and moving around](reading.md).

## Keys like a screen reader's browse mode

textweaver's default keys are the quick navigation keys of NVDA's and JAWS's browse mode, so they are already familiar:

- **h** and **Shift+H**: next and previous heading.
- **1** to **6**: next heading at that level; **Shift** with the digit: the previous one. textweaver matches the digit key itself, on any keyboard layout (on Windows it reads the key from the console; elsewhere it knows the US, UK, German, Spanish, Nordic, and Italian layouts, and French with `digit_row = "azerty"`).
- **l** and **Shift+L**: list. **i** and **Shift+I**: list item.
- **t** and **Shift+T**: table.
- **k** and **Shift+K**: link.
- **q** and **Shift+Q**: block quote.
- **s** and **Shift+S**: separator (a horizontal rule).
- **g** and **Shift+G**: graphic (an image).
- **d** and **Shift+D**: section or chapter, the nearest thing to a landmark.
- **Backspace**: go back after a jump.
- **Alt+Down** and **Alt+Up**: next and previous sentence, as in JAWS. **Ctrl+Down** and **Ctrl+Up**: next and previous paragraph.

These follow NVDA's letters; where JAWS differs (its `q`, `d`, `r`, and link keys), the [keyboard reference](keyboard.md#quick-navigation-as-in-nvda-and-jaws) says how. Quitting is **Ctrl+Q**, and it still asks first.

The earlier keys are the classic preset. The `screen-reader` preset is now the default, and naming it still works:

```toml
[keyboard]
preset = "classic"    # the earlier keys; "default" (or "screen-reader") for these
```

Your `keymap.toml` overrides apply on top. The [keyboard reference](keyboard.md#what-changed) lists every change and [the classic preset](keyboard.md#the-classic-preset).

## Screen reader settings

These are the settings to look at in NVDA and JAWS for each mode. Names and keys differ between versions. **Every item here is not yet verified by ear.**

### NVDA

- **Report dynamic content changes** (**NVDA+5**, in Object Presentation settings). On for hybrid and screen-reader modes: that is how NVDA reads the status line as it changes. Off, or speech on demand, for self-voicing.
- **Speak typed characters** (**NVDA+2**) and **Speak typed words** (**NVDA+3**), in Keyboard settings. On for hybrid and screen-reader modes, where textweaver leaves typing echo to NVDA. Off for self-voicing, where textweaver echoes.
- **Speech mode** (**NVDA+S**): talk, beeps, off, and, in recent versions, on demand. "On demand" suits self-voicing: NVDA stays quiet until you ask it to read.
- **Windows Terminal**: in Advanced settings, NVDA can read new text in Windows Terminal by "diffing" or by UIA notifications. Try both if new text is missed or read late.
- **Windows Console support** (Advanced settings): UIA or legacy, for the classic console.
- **Read current line**: **NVDA+Up** (laptop layout **NVDA+L**). With `cursor = "status"` it repeats the last message.

### JAWS

- **Screen echo** (**Insert+S** cycles none, highlighted, and all). "All" for hybrid and screen-reader modes, so JAWS reads the status line as it changes. "None" for self-voicing.
- **Typing echo** (**Insert+2** cycles characters, words, both, and none). On for hybrid and screen-reader modes; none for self-voicing.
- **Speech on and off**: **Insert+Space**, then **S**. Recent versions also have Speech on Demand in Settings Center, which suits self-voicing.
- **Read current line**: **Insert+Up**. With `cursor = "status"` it repeats the last message.
- JAWS's PC cursor follows the terminal's cursor; with `cursor = "follow"` it moves with the spoken word while textweaver reads.

### VoiceOver (macOS)

- textweaver runs in Terminal.app. VoiceOver reads new text in Terminal as it appears. Untested on a real Mac.
- textweaver's **Alt** chords, such as **Alt+P** and **Alt+Shift+A**, need the Option key to act as Alt (Meta). In Terminal, open Settings, Profiles, Keyboard, and turn on "Use Option as Meta key". Untested.
- VoiceOver's own keys use **Control+Option**; textweaver's terminal keys use no **Ctrl+Alt** chords, so there is no clash by default.

### Orca (Linux)

- textweaver runs in GNOME Terminal and other terminals. Orca reads new text in terminals as it appears. Untested.
- Orca's typing echo is in Orca Preferences, Echo: on for hybrid and screen-reader modes, off for self-voicing.

## Terminals

textweaver runs in any terminal that sends key presses in the usual way.

### Windows Terminal or the classic console

textweaver runs in Windows Terminal and in the classic console (conhost, the window `cmd` opens by default).

- Windows Terminal exposes its text through UI Automation, which recent NVDA and JAWS versions use. The classic console is older and well known to screen readers. Try both; which works better with JAWS and NVDA for each mode has not been recorded yet. **Not yet verified.**
- Paste with **Ctrl+V** in Windows Terminal, or with right-click in either.

### Windows Terminal keys that clash

Windows Terminal keeps some keys for itself, so textweaver never sees them. These come from Windows Terminal's defaults and the settings file a new installation writes (checked against Windows Terminal 1.24):

- **Alt+Left** and **Alt+Right** move between panes. In textweaver they are history back and forward. Use **Backspace** and **\\** instead (**Shift+H** and **Shift+L** with the classic preset), or unbind them in Windows Terminal. With only one pane open, Windows Terminal may pass them on; **not yet verified**.
- **Alt+Up** and **Alt+Down** move between panes. In textweaver they move by sentence (JAWS's keys). **Alt+.** and **Alt+,** do the same and never clash; or unbind them in Windows Terminal. (With the classic preset they step through notes; **e** and **Shift+E**, or **F12** and **Shift+F12**, do that too.)
- **Alt+Shift+Up** and **Alt+Shift+Down** resize panes. In textweaver they make RSVP faster and slower; **Alt+Shift+PageUp** and **Alt+Shift+PageDown** do the same and reach textweaver.
- **F11** and **Alt+Enter** switch full screen. In textweaver **F11** is the next chapter. Use **Alt+PageDown** and **Alt+PageUp** for chapters.
- **Ctrl+C** copies when text is selected in Windows Terminal; otherwise textweaver gets it and copies. **Ctrl+V** pastes, which textweaver takes as pasted text.
- **Alt+Shift+D**, **Alt+Shift+minus**, and **Alt+Shift+plus** split the window into panes. So the terminal adds a reference by DOI or ISBN with **Alt+B** (the GUI keeps **Alt+Shift+D**), and "add reference" is in the command palette (**F2**). Pitch is **Alt+=** and **Alt+-**: pressing Shift by mistake splits the window.
- **Ctrl+Alt+Left** moves to the previous pane in Windows Terminal 1.24 (`Terminal.MoveFocusPrevious` in its defaults). In textweaver it is the previous cell in a table row; with one pane open Windows Terminal may pass it on, **not yet verified**, or run `table previous column` from the palette or give it another key in `keymap.toml`. The other **Ctrl+Alt** arrows are not bound by Windows Terminal, but some graphics drivers rotate the screen with them, and a screen reader may keep them for its own table commands.
- textweaver's newer chords were checked against the same list and do not clash: **Alt+Shift+Q** (citations), **Alt+Shift+X** (explore math), **Alt+Shift+Z** (syllables), **Alt+Shift+J** (difficult words), **Alt+B** (add a reference), **F12** and **Shift+F12** (notes), **Ctrl+Down** and **Ctrl+Up** (paragraphs), and **Alt+Shift+PageUp** and **Alt+Shift+PageDown** (RSVP).
- **Alt+Space** opens the window menu. textweaver does not use it.
- **Ctrl+Shift** chords (new tab, close pane, find, scroll) and **Ctrl+Alt** with digits (switch tabs) do not clash: terminals cannot send textweaver Ctrl+Shift chords, and textweaver's terminal keys use no Ctrl+Alt digits.

To unbind a key in Windows Terminal, open its settings (**Ctrl+comma**), choose Actions, and remove the key from the action; or add this to the `actions` list in its `settings.json`, one line per key:

```json
{ "command": "unbound", "keys": "alt+left" }
```

Not yet verified which of the two works in your version.

### macOS and Linux

See [VoiceOver](#voiceover-macos) and [Orca](#orca-linux) above.

## Keys that may clash

- **The screen reader key.** NVDA, JAWS, and Orca use **Insert**, or **Caps Lock** if you choose, as their own modifier key. textweaver uses neither, so your screen reader's commands pass through to it as usual.
- **Single-letter keys.** textweaver's reading keys are single letters and punctuation, such as `.`, `h`, and `t`, like a screen reader's browse mode in a web page. Screen readers do not use their own browse mode in a terminal, so these keys reach textweaver. If a letter does nothing, check that your screen reader is not in a special mode, such as a virtual or review mode.
- **Dictation and speech recognition.** Dictation software types letters, and a letter is a command in textweaver. Press **F9** to turn single-key shortcuts off. You hear "Single-key shortcuts off." Chords with **Ctrl** or **Alt**, the arrow keys, the function keys, and the command palette (**F2**) keep working. Press **F9** again to turn them back on.
- **VoiceOver** uses **Control+Option** as its modifier. textweaver's terminal keys use no **Ctrl+Alt** chords, so there is no clash by default.
- **Windows Terminal**: see [the keys it keeps](#windows-terminal-keys-that-clash).

Any key can be changed in `keymap.toml`. The [keyboard reference](keyboard.md) explains how, and lists what terminals cannot send.

## Language

textweaver's own words (messages, lists, help, and settings) are in English, Spanish, French, German, Portuguese, or Arabic. Your documents are read in their own language whatever you choose.

- **The first run** shows a list of languages, each in its own name ("Español", "Français"), your system's language first. Up and Down move, Enter chooses, Escape keeps English.
- **Later**, open the settings screen (Shift+F10), type `language`, and press Enter or Right on "Interface language". The change is immediate: you hear it confirmed in the new language, then the title line. From the command line: `tw settings language es`.
- **The voice follows the language** when the speech engine has a voice for it: a Spanish voice for Spanish. **When the engine has no voice for the language, the current voice keeps speaking, and textweaver says so.** It never goes silent. For example, if your Eloquence has English only, choosing Spanish keeps Eloquence and says that no Spanish voice was found. `[speech.voices_by_language]` in the [settings](settings.md) chooses the voice per language.
- **Keys keep their written names** (Ctrl+S) on the status line and on your Braille display; textweaver's own voice says them in your language.
- **Answers to yes-or-no questions stay y and n** in every language, since they are the keys that answer.
- **Right-to-left languages.** Arabic messages are sent to your screen reader in reading order. On screen, Windows Terminal and the classic console do not support right-to-left text, so sighted helpers may see Arabic reversed there; your screen reader and Braille display are not affected. On Linux and macOS, `[interface] rtl` reorders it for the screen where the terminal does not.
- **To check a translation's coverage**, `en-XA` is a test language: every message comes out accented and in brackets, so anything plain was missed.

## Braille displays

textweaver draws no braille of its own, in the terminal or in the GUI. A braille display shows what your screen reader shows: the line at the terminal's cursor, and new text as it appears. This section is written for a 40-cell display, the HumanWare Mantis Q40, with NVDA or JAWS on Windows.

### Set up textweaver for the display

- **Use screen-reader mode**: `--mode screen-reader`, or `mode = "screen-reader"` under `[accessibility]`. Hybrid mode lays lines out the same way.
- **Park the cursor on the status line**: `cursor = "status"` under `[accessibility]`. The display then rests on the latest message, and your screen reader's read-current-line key repeats it. At a prompt the cursor moves to what you type, and comes back when the prompt closes.

```toml
[accessibility]
mode = "screen-reader"
cursor = "status"
```

### What a 40-cell line shows

Every status line, title line, list line, and prompt line puts its key fact in the first 40 cells. A test walks a document in screen-reader mode with `cursor = "status"` and checks each line against 40 cells (`crates/textweaver-tui/tests/braille.rs`).

- **Messages** start with what happened: "Opened essay.", "Page 12, line 400: ..." A question comes before what it is about: "Exported essay.html. Open it? y or n." and "Open web link? y or n." then the address.
- **The title line** starts with where you are, then the reading state: "Line 12 of 400, 3%, Reading". In a PDF it names the page: "Page 12 of 30, 40%". The mode, rate, and engine follow, then the document's name. In screen-reader and hybrid modes the line starts at the first cell, with no padding.
- **Lists** say the place first: "3 of 12, Chapter two, level 2". The list's first line is its place and title: "3 of 12, Outline, 12 headings". In screen-reader and hybrid modes a list covers the whole window with no border, so no border or document text comes before an item.
- **Prompts** show a short label, then what you type: "Find: chapter", "Go to page: 12". A long label is cut to its first part on that line ("Export settings to file"); the whole label is said, and shown on the status line, when the prompt opens.
- **Pages in a PDF.** Say Position (**Shift+W**) starts with the page: "Page 12 of 30. Line 400 of 2000, 20 percent." Go To (**Ctrl+G**) takes a page: in a PDF a plain number is a page, and `line 12` is a line. `p 12` and `page iv` work in any document with pages, and the printed page label wins, so `p 1` is the page printed "1" even after pages i to x. A PDF with no headings lists its pages in the outline (**Alt+O**).
- Keys keep their written names (Ctrl+S) on the display; symbols such as arrows and box drawing are not used as the only signal anywhere.
- A line that runs past 40 cells still reads correctly when you pan; only the start is guaranteed.

### NVDA braille settings to try

In NVDA's Settings, Braille category. **Every item here is not yet verified on a Mantis Q40.**

- **Tether braille**: "automatically" or "to focus", so the display follows the terminal's cursor, which `cursor = "status"` keeps on the status line. "To review" leaves the display where you moved it. To verify.
- **Show messages** and **Message timeout**: NVDA's own messages (such as "focus mode") cover the line for a few seconds. "Use timeout" with a short timeout, or "Disabled", keeps textweaver's status line in view. To verify.
- **Word wrap**: on, so a line longer than 40 cells breaks between words when you pan. To verify.
- **Focus context presentation**: "fill display for context changes" is NVDA's default; try it if the start of a line is not what you expect. To verify.
- **Interrupt speech while scrolling**: on, if you read by braille and do not want speech to go on while you pan. To verify.

### JAWS braille settings to try

In Settings Center (**Insert+6**), Braille group. **Every item here is not yet verified on a Mantis Q40.**

- **Braille mode**: Line mode shows the line at the cursor, which is what textweaver's layout is designed for. Structured mode adds control type words. To verify.
- **Braille follows active cursor**: on, so the display follows the terminal's cursor (the PC cursor), which `cursor = "status"` keeps on the status line. To verify.
- **Flash messages**: JAWS shows its own messages for a while (**Flash message time**). A shorter time, or messages off, keeps textweaver's status line in view. To verify.
- **Word wrap**: on, so panning breaks between words. To verify.
- **Status cells**: off on a 40-cell display, so all 40 cells show the line. To verify.

### Checklist for the Mantis Q40

1. Open a document in screen-reader mode with `cursor = "status"`. Are "Opened" and the document's title in the first 40 cells?
2. Press **Alt+O**. Does "3 of 12" come before each heading's text as you move?
3. Open Find (**Ctrl+F**). Does the display show the label, then what you type?
4. Press **Shift+W**, then go to page 12 of a PDF (**Ctrl+G**, `12`, Enter). Do the position and the page come first?
5. Try the NVDA and JAWS settings above. Which ones work, and which do you keep?

### What has been tried

- **Tried in testing:** a 40-cell Mantis Q40 through NVDA and JAWS on Windows, reading the status line, messages, and moving by unit in the terminal reader, before this layout; and, in the GUI, the document control, the settings dialog, and edit mode, across two sessions. The layout above waits for the checklist.
- **Not tried:** other cell widths, other display models, Orca's braille on Linux, and VoiceOver's on macOS.
- **The BRF writer** (`tw convert --to brf`, see [Converting documents](converting.md)) is a separate feature: a grade 1, or grade 2 with the `liblouis` feature, braille file you save and read on a notetaker or emboss, not the live display output above. A BRF from a document with math writes the math in Nemeth or UEB mathematics; see [Math in braille files](math.md#math-in-braille-files).
- If your combination behaves differently from this, add it to the checklist below and let the project know what you found.

## The GUI

textweaver also has a window, written entirely in Rust with AccessKit for screen readers. It shares documents, keys, settings, notes, and voices with the terminal reader, and it has been checked with NVDA, JAWS, and a braille display on Windows. [The GUI guide](gui.md) covers it in full: starting it, the file chooser, edit mode, how messages reach your screen reader (a live region, or UI Automation notifications), and the spoken word's highlight. [ADR-0027](adr/0027-xilem-gui.md), [ADR-0028](adr/0028-xilem-gui-after-the-session.md), and [ADR-0033](adr/0033-gui-session-2-and-edit-mode.md) record what was checked.

## Checklist: try each mode with JAWS and NVDA

Run through this once with NVDA and once with JAWS, in Windows Terminal and, if you like, again in the classic console. Use a throwaway state folder so your own settings are untouched: add `--home %TEMP%\tw-check` to each command (in PowerShell, `--home $env:TEMP\tw-check`). Two sample files have what is needed: `fixtures/sample.md` has a table, and `fixtures/o/math-sample.md` has math.

### First run

1. Delete the folder `%TEMP%\tw-check` if it exists. With your screen reader running, start `textweaver --home %TEMP%\tw-check fixtures/sample.md`.
2. Expected: the question "NVDA is running. Use hybrid mode ...? y or n" (JAWS: "JAWS is running."). Your screen reader reads it from the status line, and textweaver speaks it too.
3. Press **y**. Expected: "Hybrid mode. ..." is read once, by your screen reader. Quit with **Ctrl+Q**, then **y**, and start again: no question this time.

### Hybrid (`--mode hybrid`)

Screen reader settings: dynamic content (NVDA) or screen echo "all" (JAWS) on; typing echo on.

1. Press **Alt+.** (next sentence). Expected: your screen reader reads the sentence preview once; textweaver is silent.
2. Press **.** (say the sentence). Expected: textweaver reads the sentence in its voice; your screen reader says nothing more.
3. Press **Space**. Expected: textweaver reads on; the status line does not change while it reads. Press **Space** again: "Paused" from your screen reader.
4. Press **Right**. Expected: your screen reader says the word; textweaver is silent.
5. Press **Ctrl+F** and type `ada`. Expected: your screen reader echoes the typing; textweaver does not. Press **Escape**.
6. Press **t** to reach the table, then **.**. Expected: textweaver narrates the row with its headers.
7. Open `fixtures/o/math-sample.md` (**Ctrl+O**) and press **.** on a formula. Expected: textweaver says the math in words. Then press **Alt+Shift+X**, **Down**, and **Right**. Expected: each part of the formula is said and highlighted; **Escape** leaves.

### Screen reader (`--mode screen-reader`, or `--no-speech`)

1. Press **Alt+.**, then **.**. Expected: your screen reader reads the preview, then the sentence; textweaver never speaks.
2. In `fixtures/o/math-sample.md`, press **.** on a formula. Expected: your screen reader reads the math in words, not dollar signs and carets.
3. Press **Space**. Expected: your screen reader reads the document a sentence at a time, and the cursor moves along. Press **+** or **-** to change the pace, **Space** to pause and resume, **Escape** to stop.
4. With `say_all = "voice"` under `[accessibility]`, press **Space** again. Expected: textweaver's voice reads instead.

### Self-voicing (`--mode self-voicing`)

1. Turn your screen reader to speech on demand, or turn dynamic content (NVDA) or screen echo (JAWS) off.
2. Press **Alt+.** and **Space**. Expected: textweaver speaks everything, once.

### Quiet screen and the status-line cursor

1. Add `quiet_screen = true` and `cursor = "status"` under `[accessibility]` in `%TEMP%\tw-check`'s `settings.toml`, and start in self-voicing mode with your screen reader's dynamic content or screen echo on.
2. Press **Space**. Expected: the title line's "line ... of ..." stays still while textweaver reads, and your screen reader does not chatter.
3. Press **Escape**, then **Alt+.**, then your screen reader's "read current line" (**NVDA+Up**, **Insert+Up**). Expected: it repeats the message on the status line.

### Keys

1. With the default keys: **h** and **Shift+H** move by heading, **l** by list, **k** by link, **q** by block quote, **Backspace** goes back, and **1** to **6** with and without Shift move by heading level, also on a non-US keyboard layout. Add `preset = "classic"` under `[keyboard]` and start again: **.** moves by sentence and **q** asks to quit.
2. In Windows Terminal, press **Alt+Left** after a jump. Expected: textweaver goes back, or Windows Terminal takes the key. Note which, and whether it changes with more than one pane.
3. Press **Alt+Shift+A** three times. Expected: hybrid, screen reader, and self-voicing, each announced once.

Please note which steps did not behave as expected, with the screen reader, its version, and the terminal.

## If something goes wrong

- **Everything is said twice.** Use hybrid mode (**Alt+Shift+A**), or see [Avoid hearing things twice](#avoid-hearing-things-twice).
- **Your screen reader does not read the status line.** Turn on its reading of new text in terminals: NVDA's "report dynamic content changes" (**NVDA+5**), or JAWS's screen echo (**Insert+S**).
- **Nothing is said at all.** In screen-reader mode textweaver is silent by design; check that your screen reader reads new text. The title line shows the mode.
- **The question about hybrid mode never came, or came wrongly.** See [The first run with a screen reader](#the-first-run-with-a-screen-reader) and `TEXTWEAVER_SCREEN_READER`.
- **A key does nothing.** See [Windows Terminal keys that clash](#windows-terminal-keys-that-clash) and [Troubleshooting](troubleshooting.md), "A key does nothing".
- **No speech from textweaver.** Check the title line: `silent` means `--no-speech` or no engine. See [Troubleshooting](troubleshooting.md).

## See also

- [Reading and moving around](reading.md): every reading key, and the command line options.
- [Keyboard reference](keyboard.md): every key, what changed, the classic preset, and what terminals cannot send.
- [Settings](settings.md): the `[accessibility]` settings and `[keyboard] preset`.
- [Converting documents](converting.md): the `brf` braille output, and the other formats `tw convert` writes.
- [The GUI guide](gui.md): the window in full, including edit mode and the file chooser.
- [Troubleshooting](troubleshooting.md): common problems and how to report a bug.
- [ADR-0006: Keymap, actions, and announcements](adr/0006-keymap-and-actions.md): how announcements reach the status line.
- [ADR-0014: GUI toolkit](adr/0014-gui-toolkit.md): the GUI preview and what was checked.
- [Documentation index](README.md)
