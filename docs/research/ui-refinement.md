# User interface refinement pass: the list

Started Monday, September 28, 2026, at the owner's request, after the documentation sweep. This page gathers everything to improve in textweaver's interfaces (the terminal reader, `tw`, and the GUI) before the pass is planned. Nothing here is being built yet. The owner adds items first; the plan and agent briefs come after the owner says the list is complete.

Each item says what is wrong, where it was found, and which interface it affects. Items are not ranked yet.

## The owner's items

To be added by the owner.

## Found so far

### The GUI

- **Settings dialog focus:** it opens on a table setting, so NVDA reads "4 entries", "none", and the next setting's help all at once. Focus should start on a plain first setting, with the section named. Found by the NVDA check in CI (W5t, ADR-0039); sent to W5a4.
- **Yes-or-no questions:** the GUI seems to have no way to answer a question outside a list (it never sends `Command::Confirm`). Found by W5x; sent to W5a4.
- **The document on macOS:** the accessibility tree shows it as a group with its text as the value, not as a text area, which may change how VoiceOver reads it. Found by the tree dump in CI (W5t).
- **Edit mode, left from W4a3:** caret and selection moves are not spoken in self-voicing mode; Tab moves focus instead of typing a tab or moving between table cells; misspellings are not marked on screen. W5a4 is working on these.
- **Startup announcements:** "Opened" and "Reading at ..." can be lost when they come before a screen reader has asked for the window (ADR-0028; seen again by the Orca and NVDA checks in CI).
- **The GUI's own labels in six languages:** messages are translated, but the settings dialog's drawn labels and some of the GUI's own strings are still English (W4d's open issue).
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
