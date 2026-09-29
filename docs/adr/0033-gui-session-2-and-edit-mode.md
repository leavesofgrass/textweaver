# ADR-0033: The GUI after further accessibility testing, and edit mode

- Status: accepted
- Date: 2026-09-28
- Builds on: [ADR-0027](0027-xilem-gui.md) (the Xilem GUI) and [ADR-0028](0028-xilem-gui-after-the-session.md) (the first session)

## Context

A second accessibility test session with the Xilem GUI, on Monday, September 28, 2026, passed: the release build works, and Eloquence reads with the words highlighted in step. It found four things to fix before edit mode:

1. A console window opened with the GUI and stayed behind it, so two windows were active.
2. Open was only a line for a file path. It needs the system's own file chooser, with the typed path kept as a fallback.
3. The font and the size of the text on screen need to be changeable from the keyboard.
4. Every control needs a keyboard shortcut, shown and spoken with its name, such as "Open, Control O".

Then edit mode, kept for the GUI (decided September 27).

## Decisions

### No console window

`textweaver-xilem.exe` is a GUI-subsystem program (`#![windows_subsystem = "windows"]`) in every build, so debug runs behave the same as release runs. A GUI program is not connected to the terminal it is started from, so `console::attach` calls `AttachConsole(ATTACH_PARENT_PROCESS)` before anything is printed, when standard output and error are not already redirected. `--help`, `--version`, and argument errors reach that terminal. Once the window runs, the console is let go (`FreeConsole`, and the standard handles cleared), unless `--log` writes there, so Control C in that terminal cannot close the window. An error at the end attaches again to print.

With no terminal at all (a shortcut, File Explorer), a startup error is shown in a message box, which screen readers read like any dialog, except in `--background` runs, which never show one. Errors and panics also go to the `--log-file` log.

PowerShell does not wait for a GUI-subsystem program, so its output can come after the next prompt; `| Out-Host` waits. The UI Automation report, which starts the GUI with `CREATE_NO_WINDOW` and `--log-file`, is unchanged and passes.

### The system's file chooser

Open shows the system's own dialog through the `rfd` crate (0.17.2, MIT): the common item dialog (`IFileOpenDialog`) on Windows, the open panel on macOS, and on Linux the XDG desktop portal over D-Bus, loaded at run time. It is built with `default-features = false, features = ["xdg-portal"]`: no GTK, and not the `wayland` feature, which links `libwayland-client` at build time only to name the parent window on Wayland (X11 parents still work). `cargo deny` passes; the new crates add duplicate-version warnings only (`block2`, `objc2`, `objc2-app-kit`, `objc2-foundation` on macOS, `bitflags`, and a third `windows-sys`).

- The filters are "Documents textweaver reads" (the format registry's extensions) and "All files". The dialog starts in the open document's folder.
- The dialog runs on its own thread, modal to the window, so the window's event loop never runs inside it. Its answer comes back as an action.
- The answer goes through the app's Open prompt (set the text, then Enter), so the file joins the prompt's history and the app says "Opened" with the title, as it does for a typed path. Cancel is the prompt's Escape. The focus returns to the document.
- The typed path stays. **Open Path** (`open_path`, Ctrl+Shift+G, after the "Go to folder" key of file dialogs) opens the one-line prompt, with Tab completion and history. A dialog that closes with no file in under 250 ms was never shown (no portal on Linux), so textweaver says so and shows the typed prompt.

Open Path, and the four commands below, are **window-only** (`ActionId::is_window_only`): they have GUI keys and no terminal keys, and a keymap test checks both. In the terminal Open is already a typed path, and its palette says the font and size commands work in the window.

A native dialog cannot be driven in tests. The code around it (filters, the start folder, what an answer means) has unit tests; the dialog itself is on the manual test checklist.

### Text size and font

The existing `[reading_aids.font]` settings (family, size in points, weight) already cover this, in the store, the export fixture, the schema, and the GUI, so nothing new was added to them.

- **Ctrl+Plus** (`Ctrl+=`; the number pad's plus and Shift with the equals key arrive as `Ctrl++`, and the GUI's key reader treats them as `Ctrl+=`), **Ctrl+Minus**, and **Ctrl+0** step the size through 8, 9, ... 16, 18, 20, ... 28, 32, 36, 40, 48, 56, 64, 72 points, or back to the standard 14. A size between two steps goes to the next one either way.
- **Ctrl+D** (the font dialog key of word processors), or the Font button, opens the font list, the bundled families first ("built in": Atkinson Hyperlegible Next and Mono, OpenDyslexic), then the installed ones. The family applies at once and keeps the size and weight. The earlier second list, of sizes, is gone: the size has its keys.
- Each change is saved and said, assertively for the size keys so a held key says only the latest size: "Text size 18 points.", "Text size 72 points, the largest.", "Font: OpenDyslexic." The messages are in all six catalogs.
- The window now takes the font from the settings on every refresh, so a change made in the Settings dialog reaches the document too.

`Ctrl+=` and `Ctrl+-` were Star's rate keys in the GUI. Text size is what screen reader users expect on them, so the GUI's rate moved to **F11** and **Shift+F11**, beside volume on F7. The browse keys `+`, `=`, and `-` still change the rate, and the terminal is unchanged. The keymap test of Star's GUI chords records the change.

### A shortcut on every control

Every button has its key from the keymap (`named_key_in`, then `written_text`), never a fixed string:

- **In the node's keyboard shortcut property:** "Ctrl+O", which is UI Automation's AcceleratorKey and AT-SPI's equivalent. NVDA and JAWS say it after the name when their "report shortcut keys" setting is on, so each listener chooses whether to hear it.
- **The name is the label only:** "Open", "Faster", without the label's ellipsis.
- **On screen, written:** "Open… (Ctrl+O)". F1 help and the command palette list every key too.
- It is the app's main key: the single key while single-key shortcuts are on ("Space" for Play), a chord while they are off ("Ctrl+Shift+Space"). F9 updates every button at once.
- The Font button is now the `choose_font` command, so it has a key like the others. The settings dialog's Close button has Escape, the dialog's own key.

**Changed after the first version (Monday, September 28, 2026).** The first version put the key in the name, spoken ("Open, Control O"), because not every screen reader reads the accelerator key property, and left the property unset so NVDA would not say the key twice. The key in the name proved wordy. The property leaves it to the screen reader's setting, so the name is the label again.

**AccessKit did not pass the property on.** AccessKit has the keyboard shortcut property, but its Windows adapter (0.35.1) never gave it to UI Automation, and its AT-SPI and macOS adapters do not either. So `third_party/accesskit_windows` is a vendored copy of the Windows adapter that answers UI Automation's AcceleratorKey with it (two lines in its property table; see its `TEXTWEAVER.md` and `textweaver.patch`), patched in through `[patch.crates-io]` in the root `Cargo.toml`. It is meant to go upstream; on Linux and macOS the key is on screen but not yet in the accessibility tree.

The new keys pass the keymap's conflict, reachability, layout, and WCAG 2.1.4 tests (all are chords). The UI Automation report fails if a button has no AcceleratorKey, or has its key in its name; it passes, with "Open" having AcceleratorKey "Ctrl+O".

## Edit mode

Edit mode was planned for `DocumentView`, based on Parley's `examples/editor`. It is built on the view the reading mode already has instead, because that view already holds what an editor needs and Parley's editor would duplicate it: the text as paragraphs with their layouts, the caret and selection as the node's text selection, caret keys that need layout (lines, Home, End, pages), the selection drawn by the view itself (Parley has no background style), and text runs with stable ids. The vendored Parley 0.8.0 is unchanged. What edit mode adds:

- **The app is the editor.** Ctrl+E (or the new Edit button, which becomes "Finish editing") enters the app's edit mode, as in the terminal: the session's document becomes the source text, and every edit goes through `textweaver-editor`, so undo and redo, formatting, autosave, and Save / Discard / Cancel are the terminal's.
- **The view becomes a multi-line edit.** Its role is `MultilineTextInput`, not read-only, with the `ReplaceSelectedText` and `SetValue` actions besides `SetTextSelection`; NVDA and JAWS switch to focus mode by themselves. Outside edit mode it is the read-only Document it was.
- **Keys.** A printable key (and Space), Enter (a new line), and an input method's text are typed; Backspace and Delete delete. Keys with Ctrl, Alt, or Command go on to the keymap, so the editing commands work as in the terminal, except AltGr (Ctrl with Alt) typing a symbol. Tab still moves the focus, so the edit never traps the keyboard.
- **How an edit reaches the app** (`DocAction`):
  - text typed at a collapsed caret is `Command::Insert`, and Backspace or Delete there is `Command::DeleteBack` or `DeleteForward`, so the typing echo follows the access mode as in the terminal: textweaver's voice echoes in the self-voicing mode, and the screen reader echoes in the other two, with nothing said twice;
  - typing or deleting over the view's selection, and a screen reader's or dictation's `ReplaceSelectedText` or `SetValue`, is `Command::ReplaceRange` with the view's range, which is quiet (the screen reader already said it) and does not depend on the app's own selection.
- **The caret and selection.** The view moves the caret and selection itself and tells the app the caret (`SetCursor`, quietly), which also moves the editor's caret; a selection the app holds (from a command) is dropped when the view moves the caret. A selection the app makes, such as the next misspelling, reaches the view as its selection, so the screen reader reads it and typing replaces it.
- **Keeping the screen reader's place.** An edit changes the document's revision. In edit mode the view takes the new text as it takes a window slide: paragraphs that start at the same place with the same text keep their layouts and their run nodes, so only the edited paragraph and the ones after it are sent again. Entering or leaving edit mode replaces every run.
- **What waited for edit mode now works in the window**, through the app: spell check (Alt+M selects the next misspelling, Alt+J suggests), citations while writing (Alt+C opens the picker, a list dialog filtered as you type), and export and the browser preview (in the command palette). The GUI now builds the app with its `publish` feature (the crate's own `publish` feature, on by default). Measurement showed that feature at about 18.6 MB in the terminal reader's release build (28,704,256 bytes without it, 47,332,864 with); the GUI's own size was not measured here.

Tested in the harness (`tests/edit_mode.rs`): the role and read-only state in and out of edit mode; typing, Enter, Backspace, and Space through the app, the caret following, and undo; typing and deleting over a selection; `ReplaceSelectedText` ignored while reading and applied while editing; a misspelling selected and corrected by typing; and the citation picker. What the harness cannot show (what NVDA and JAWS say while typing, and focus mode) is on the manual test checklist.

**Not done yet:**

- In the self-voicing mode, caret moves and selections made in the view are not spoken by textweaver, as in reading; typing and deleting are.
- Each edit rebuilds the window's paragraphs from the document (about 120,000 characters at most), which is quick for notes and papers; a very long single paragraph is the slow case, not measured.
- Tab does not type a tab or move between table cells in the window (it moves the focus); `next_table_cell` stays on its key in the terminal.
- Misspelled words are not marked on screen (neither are they in the terminal); Alt+M finds them.

## Status update: what was left, done (Monday, September 28, 2026)

Three of the four "Not done yet" items above are done; edit mode is otherwise unchanged by the Parley upgrade (ADR-0027's status update), since it never used Parley's editor.

- **Caret and selection speech in the self-voicing mode.** A caret key in the view now carries what the terminal's caret keys say (`DocAction::CaretMoved`'s `echo`): the character at the caret (Left, Right, Home, End), the word (Ctrl+Left, Ctrl+Right), the line as drawn (Up, Down, the page keys, Ctrl+Up and Ctrl+Down, Ctrl+Home and Ctrl+End), the end of a line or of the document, and with Shift what the selection gained or lost ("cd selected"). The driver hands it to `App::echo`, which speaks only when the typing echo goes to textweaver's voice (the self-voicing mode), and not while reading; a screen reader reads the caret itself. The same keys speak in reading mode too, as the terminal's do. Pointer clicks and a screen reader's own moves say nothing.
- **Tab.** In edit mode Tab and Shift+Tab run the app's `next_table_cell` and `previous_table_cell`, as the terminal's edit layer does: the next or previous cell in a table, else a tab typed (Shift+Tab outside a table says so). Ctrl+Tab and Ctrl+Shift+Tab now move the focus everywhere (as they leave a multi-line edit on Windows), so the edit still never traps the keyboard. Outside edit mode Tab moves the focus as before.
- **Misspelled words are marked** with a dotted underline (a shape unlike a link's line or a difficult word's thick one, in the focus color, never color alone), in edit mode, once typing has paused for half a second, in documents up to a million characters (the check reads the whole document on the input thread: 0.6 s for 10 million). The app gained one small hook for it, `App::misspelled_ranges`. The marks are drawn only; AccessKit's `is_spelling_error` is not set, because no platform adapter passes it on yet (UI Automation would need the annotation attribute in the vendored adapter).
- **Still open:** each edit rebuilds the window's paragraphs (a very long single paragraph is the slow case, not measured).

**Questions in the window (found on the way).** The app's yes-or-no questions (a voice download after its size and licence are said, a removal, a file changed on disk, a settings import) had no answer in the window: it never sent `Command::Confirm`, so y and n went to the keymap. A question is now an in-window dialog named by the question, with Yes (focused) and No buttons whose keys are Y and N; y and n typed anywhere in it answer, Escape is no, and any other character asks again, as in the terminal.

## Status update: edit mode at parity (Tuesday, September 29, 2026)

- **A key costs one paragraph.** "Each edit rebuilds the window's paragraphs" was measured before it was changed: a release build, the 1 MB corpus, the caret mid-document, 40 keys, the app's edit, the window's refresh, and Masonry's layout and accessibility passes (`examples/edit_timing.rs`): 24.4 ms a key (median; Backspace 27.7 ms), with 1,236 AccessKit nodes sent per key, the whole window. Two causes: the view took every root pass for a full rebuild (ADR-0027's status update), and an edit slid the model, matching paragraphs by position, so every paragraph after the caret lost its layout and nodes. `DocumentView::edit_model` now keeps the paragraphs before and after the change, with their layouts, visual lines, and run nodes moved by the change's length, and rebuilds only the edited ones. After: 2.2 ms a key (Backspace 2.1 ms), 13 nodes sent, one of them a text run; the window's own model (1.5 ms) is now most of it. A test checks that the runs around an edit keep their ids, that only the edited paragraph's runs are sent, and that the text a screen reader reads stays the document's through new lines, joined paragraphs, undo, and redo.
- **Copy, cut, and paste.** Copy and Cut are the keymap's, as in the terminal: the app's selection follows the view's, and what the app copies goes to the system clipboard. The platform's paste key, which Masonry reads for the window and delivers as clipboard text, types it at the caret (the view ignored it before).
- **Markdown lint** works in the window (the `lint` feature, as in the terminal). Grammar stays behind the `grammar` feature in both.

## Consequences

- One small `unsafe` module more in the GUI crate (`console`), with three console calls and one message box; Windows only.
- `rfd` joins the dependencies, in a "W4a3" block in the root `Cargo.toml`.
- A vendored `accesskit_windows` with one change, until AccessKit gives the keyboard shortcut to UI Automation itself.
- Five window-only commands, and the GUI's rate chords moved to F11 and Shift+F11.
- The GUI links the app's `publish` stack (citations, export, preview), for writing.
- `DocAction` is no longer `Copy`: it carries typed text.

## See also

- [The textweaver window](../gui.md)
- [ADR-0027: Xilem GUI](0027-xilem-gui.md)
- [ADR-0028: The Xilem GUI after the first accessibility session](0028-xilem-gui-after-the-session.md)
