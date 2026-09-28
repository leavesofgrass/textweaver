# ADR-0033: The GUI after the second session, and edit mode

- Status: accepted (Wave 4, Agent W4a3, sub-wave 4c)
- Date: 2026-09-28
- Builds on: [ADR-0027](0027-xilem-gui.md) (the Xilem GUI) and [ADR-0028](0028-xilem-gui-after-the-session.md) (the first session)

## Context

The owner's second session with the Xilem GUI, on Monday, September 28, 2026, passed: the release build works, and Eloquence reads with the words highlighted in step. It found four things to fix before edit mode:

1. A console window opened with the GUI and stayed behind it, so two windows were active.
2. Open was only a line for a file path. It needs the system's own file chooser, with the typed path kept as a fallback.
3. The owner wants to change the font and the size of the text on screen from the keyboard.
4. Every control needs a keyboard shortcut, shown and spoken with its name, such as "Open, Control O".

Then edit mode, which Wave 4 kept for the GUI (the owner's decision of September 27).

## Decisions

### No console window

`textweaver-xilem.exe` is a GUI-subsystem program (`#![windows_subsystem = "windows"]`) in every build, so debug runs behave as the owner's do. A GUI program is not connected to the terminal it is started from, so `console::attach` calls `AttachConsole(ATTACH_PARENT_PROCESS)` before anything is printed, when standard output and error are not already redirected. `--help`, `--version`, and argument errors reach that terminal. Once the window runs, the console is let go (`FreeConsole`, and the standard handles cleared), unless `--log` writes there, so Control C in that terminal cannot close the window. An error at the end attaches again to print.

With no terminal at all (a shortcut, File Explorer), a startup error is shown in a message box, which screen readers read like any dialog, except in `--background` runs, which never show one. Errors and panics also go to the `--log-file` log.

PowerShell does not wait for a GUI-subsystem program, so its output can come after the next prompt; `| Out-Host` waits. The UI Automation report, which starts the GUI with `CREATE_NO_WINDOW` and `--log-file`, is unchanged and passes.

### The system's file chooser

Open shows the system's own dialog through the `rfd` crate (0.17.2, MIT): the common item dialog (`IFileOpenDialog`) on Windows, the open panel on macOS, and on Linux the XDG desktop portal over D-Bus, loaded at run time. It is built with `default-features = false, features = ["xdg-portal"]`: no GTK, and not the `wayland` feature, which links `libwayland-client` at build time only to name the parent window on Wayland (X11 parents still work). `cargo deny` passes; the new crates add duplicate-version warnings only (`block2`, `objc2`, `objc2-app-kit`, `objc2-foundation` on macOS, `bitflags`, and a third `windows-sys`).

- The filters are "Documents textweaver reads" (the format registry's extensions) and "All files". The dialog starts in the open document's folder.
- The dialog runs on its own thread, modal to the window, so the window's event loop never runs inside it. Its answer comes back as an action.
- The answer goes through the app's Open prompt (set the text, then Enter), so the file joins the prompt's history and the app says "Opened" with the title, as it does for a typed path. Cancel is the prompt's Escape. The focus returns to the document.
- The typed path stays. **Open Path** (`open_path`, Ctrl+Shift+G, after the "Go to folder" key of file dialogs) opens the one-line prompt, with Tab completion and history. A dialog that closes with no file in under 250 ms was never shown (no portal on Linux), so textweaver says so and shows the typed prompt.

Open Path, and the four commands below, are **window-only** (`ActionId::is_window_only`): they have GUI keys and no terminal keys, and a keymap test checks both. In the terminal Open is already a typed path, and its palette says the font and size commands work in the window.

A native dialog cannot be driven in tests. The code around it (filters, the start folder, what an answer means) has unit tests; the dialog itself is on the owner's checklist.

### Text size and font

The existing `[reading_aids.font]` settings (family, size in points, weight) already cover this, in the store, the export fixture, the schema, and the GUI, so nothing new was added to them.

- **Ctrl+Plus** (`Ctrl+=`; the number pad's plus and Shift with the equals key arrive as `Ctrl++`, and the GUI's key reader treats them as `Ctrl+=`), **Ctrl+Minus**, and **Ctrl+0** step the size through 8, 9, ... 16, 18, 20, ... 28, 32, 36, 40, 48, 56, 64, 72 points, or back to the standard 14. A size between two steps goes to the next one either way.
- **Ctrl+D** (the font dialog key of word processors), or the Font button, opens the font list, the bundled families first ("built in": Atkinson Hyperlegible Next and Mono, OpenDyslexic), then the installed ones. The family applies at once and keeps the size and weight. The earlier second list, of sizes, is gone: the size has its keys.
- Each change is saved and said, assertively for the size keys so a held key says only the latest size: "Text size 18 points.", "Text size 72 points, the largest.", "Font: OpenDyslexic." The messages are in all six catalogs.
- The window now takes the font from the settings on every refresh, so a change made in the Settings dialog reaches the document too.

`Ctrl+=` and `Ctrl+-` were Star's rate keys in the GUI. Text size is what screen reader users expect on them, as the owner asked, so the GUI's rate moved to **F11** and **Shift+F11**, beside volume on F7. The browse keys `+`, `=`, and `-` still change the rate, and the terminal is unchanged. The keymap test of Star's GUI chords records the change.

### A shortcut on every control

Every button names its key from the keymap (`named_key_in`, then `written_text` and `spoken_text`), never a fixed string:

- **In its name, spoken:** "Open, Control O", "Faster, plus". The name leaves out the label's ellipsis. Screen readers do not all read UI Automation's accelerator key property, so the key is in the name, and the property is no longer set, so NVDA does not say it twice.
- **On screen, written:** "Open… (Ctrl+O)".
- It is the app's main key: the single key while single-key shortcuts are on ("Play, Space"), a chord while they are off ("Play, Control Shift Space"). F9 renames the buttons at once.
- The Font button is now the `choose_font` command, so it has a key like the others. The settings dialog's Close button names Escape, the dialog's own key.

The new keys pass the keymap's conflict, reachability, layout, and WCAG 2.1.4 tests (all are chords). The UI Automation report fails if a button's name does not include its key.

## Edit mode

To be written with the edit mode work in this branch.

## Consequences

- One small `unsafe` module more in the GUI crate (`console`), with three console calls and one message box; Windows only.
- `rfd` joins the dependencies, in a "W4a3" block in the root `Cargo.toml`.
- Five window-only commands, and the GUI's rate chords moved to F11 and Shift+F11.

## See also

- [The textweaver window](../gui.md)
- [ADR-0027: Xilem GUI](0027-xilem-gui.md)
- [ADR-0028: The Xilem GUI after the owner's session](0028-xilem-gui-after-the-session.md)
