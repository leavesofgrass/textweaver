# User interface refinement pass: the list

Started Monday, September 28, 2026, at the owner's request, after the documentation sweep. This page gathers everything to improve in textweaver's interfaces (the terminal reader, `tw`, and the GUI) before the pass is planned. Nothing here is being built yet. The owner adds items first; the plan and agent briefs come after the owner says the list is complete.

Each item says what is wrong, where it was found, and which interface it affects. Items are not ranked yet.

## The owner's items

Given on Monday, September 28, 2026. The owner notes some go beyond interface polish; they are wanted all the same. The owner's wiki (`D:\star\wiki`) has development notes on Star and abax that the planning should read first.

1. **A real menu bar, in the GUI (and a menu in the terminal reader).** File, Edit, View, Reading, Speech, Tools, Help, as in every other application the owner uses. Every feature and option reachable from a menu, and each item showing its shortcut beside it, so the shortcuts are easy to learn. Today there is no menu: the GUI has a toolbar and the command palette, the terminal reader has the palette (F2) and F1 help.
2. **The command palette, refined.** Start typing a command and either Tab-complete it or see it in context: its category, what it does, and its shortcut. As in Star and the owner's other projects. Today both frontends have a palette (Tab completes, Up and Down list matches); the refinement is the context shown with each match.
3. **A proper file manager and file chooser, with archives.** Browse folders and archives (zip, tar, 7z) to open reading material or import other formats, as abax did by following the Worker file manager. Today the GUI uses the system's Open dialog (W4a3) with a typed-path fallback, and the terminal reader takes a typed path; archives open through the archive loader (W3d) but cannot be browsed.
4. **Export speech as WAV or MP3, and a better way than ffmpeg if there is one.** Today `tw export-audio` writes WAV itself and MP3 and M4B audiobooks through ffmpeg, which the user installs; Star also used ffmpeg. To research: encoding MP3 (and M4B's AAC, or Opus) without an outside program, ideally in pure Rust, with its licence and quality; and exporting from inside the reader and the GUI's menus, not only from `tw`.

## Found so far

### The GUI

- **Settings dialog focus:** it opened on a table setting, so NVDA read "4 entries", "none", and the next setting's help all at once. Fixed by W5a4 (it starts on a plain setting); to confirm in session 3.
- **Yes-or-no questions:** the GUI had no way to answer a question outside a list. Fixed by W5a4: a dialog with Yes and No, keys Y and N, Escape for no; to confirm in session 3.
- **The document on macOS:** the accessibility tree shows it as a group with its text as the value, not as a text area, which may change how VoiceOver reads it. Found by the tree dump in CI (W5t).
- **Edit mode, left from W4a3:** caret and selection speech in self-voicing mode, Tab typing a tab (Ctrl+Tab leaves the document), and misspelling marks. Fixed by W5a4; to confirm in session 3. The marks are drawn only: AccessKit's spelling-error flag reaches no platform yet.
- **The window's name on focus:** when the GUI's window takes the focus, NVDA says neither the window's nor the document's name (the NVDA check in CI, ADR-0039). A screen reader user should hear where they are.
- **Startup announcements:** "Opened" and "Reading at ..." can be lost when they come before a screen reader has asked for the window (ADR-0028; seen again by the Orca and NVDA checks in CI).
- **The GUI's own labels in six languages:** fixed by W5a4 (every drawn label from the catalog, changing live); the six languages still need a native speaker's review.
- **The GUI's rate keys:** moved to F11 and Shift+F11 when Ctrl+Plus and Ctrl+Minus became text size (W4a3). Worth checking that they are easy to find.

### The terminal reader and `tw`

- **Orca's key echo:** Orca speaks each key pressed ("left control", "space") by default; worth a note in the screen reader guide, not a textweaver change.
- **Filtered outline with a question waiting:** `y` and `n` answer an "Open it?" question instead of typing into the outline's filter (W5x's open issue).
- **Braille settings:** every NVDA and JAWS braille setting in the screen reader guide is marked "to verify" until the owner's Braille session B1.
- **Numbers:** thousands separators stay English ("3,412") in every interface language (W4d).

### The docs site

- The arrow-key highlight in search results is not announced; Tab reaches each result instead (the site agent's workaround).
- Markdown tables have header cells but no captions.
- Wide code blocks cannot take focus, so scrolling them by keyboard depends on the browser.

## How the pass will be planned

When the owner says the list is complete:
1. Each item gets its interface, a size, the files it touches, and whether it needs the owner's session to judge.
2. The items are grouped into agents that do not share files, and run in parallel, as in Wave 5.
3. Each agent's report ends with a short checklist for NVDA, JAWS, and the Braille display.

## See also

- [Wave 5, recalibrated](wave5-recalibrated.md)
- [ADR-0033: the GUI after session 2](../adr/0033-gui-session-2-and-edit-mode.md)
- [ADR-0039: automated screen reader checks](../adr/0039-automated-screen-reader-checks.md)
- [Tasks and agent briefs](../history/tasks.md)
