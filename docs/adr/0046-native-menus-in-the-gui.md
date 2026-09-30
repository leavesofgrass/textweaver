# ADR-0046: Native menus in the GUI

- Status: accepted. Waiting for session 4 (the menus with NVDA, JAWS, and the Braille display) and for the dialogs of browse files, batch conversion, audio export, and dictation, which join the menus as their modules register.
- Date: 2026-09-29
- Builds on: [ADR-0043](0043-menus-and-the-palette-from-one-model.md) (the menus and the palette from one model), [ADR-0027](0027-xilem-gui.md) (the Xilem GUI), [ADR-0033](0033-gui-session-2-and-edit-mode.md) (the GUI after session 2, the system's file chooser)

## Context

ADR-0043 put the menus in the app core: seven menus of commands, submenus, and separators, labelled from the catalog, with every key read from the live keymap and every toggle's state read live. The terminal shows them as a list on F10. The window had a banner of five buttons and the command palette, and no menu bar.

Masonry has no menus. A drawn menu bar would have to get every screen-reader behavior right by hand, while a Windows menu bar gives NVDA and JAWS what they expect with no work: UI Automation's MenuBar, Menu, and MenuItem, the access key from the `&` letter, the shortcut as `AcceleratorKey` from the text after a tab, Alt and F10 to enter, and "checked" on a check item. macOS has one menu bar per application, reached by VoiceOver's own key. Linux has no system menu bar that works without GTK.

Two things were unknown before this work, and were checked on its first day:

- **Whether a native menu attaches to Masonry's winit window, and how keys reach it.** muda's Windows menus draw accelerators only if the event loop calls `TranslateAcceleratorW` with the menu's table. winit's `EventLoopBuilderExtWindows::with_msg_hook` is reachable, because `masonry_winit::app::EventLoop::with_user_event()` returns winit's own builder.
- **Whether Alt and F10 enter the bar through winit.** winit hands `WM_SYSKEYUP` to `DefWindowProc` when a window has a menu (so Alt alone and F10 work), but it answers `WM_SYSCHAR` itself, so Alt with a menu's letter and Alt+Space (the system menu) never reach Windows' own menu handling.

## Decision

### One model, shown three ways

`crate::menus` turns `App::menu_bar` and `App::menu_view` into a tree of the menus as a system shows them. No list of commands exists in the GUI: every label, access key, key, and toggle state comes from the model, so the window, the terminal, and JSON-RPC cannot drift apart. The few commands only the terminal has (scrolling by lines, line numbers) are left out of the window's menus, by the same parity table the GUI already keeps.

- **Windows:** a Win32 menu bar (an `HMENU`) through `muda` 0.20, with its default features off (no GTK, no libxdo). Each item's text is its label with the access key marked, a tab, and its key: UI Automation reads the text after the tab as `AcceleratorKey`. Toggles are check items; a choice shows its value ("Reading ruler: current line").
- **macOS:** the application's menu bar (`NSMenu`, through `muda`), under a first menu named for the application as macOS expects. Each key becomes the item's key equivalent, as a logical key from the macOS column of the keymap (ADR-0043), only when it has a modifier or is a function key, so a bare letter or arrow never takes keys from typing or the caret.
- **Linux, and with `--list-menus`:** the app's list menu (F10) in the window's list dialog, with the terminal's keys. A submenu opens under its own name, so the screen reader says it.

`muda` is a target dependency for Windows and macOS only; Linux builds never compile it.

### Keys stay in the keymap

The menus show keys; they do not handle them. No accelerator table is installed and `TranslateAcceleratorW` is never called, so a key runs through the keymap alone, in every dialog and mode, exactly as before, and the menus cannot run a command a second time. On macOS a key equivalent does reach the menu first (that is how macOS works); the menu runs the same command through the app, as the key would.

Windows' own menu keys keep working through winit. F10 and Alt alone are Windows'. For Alt with a menu's letter and Alt+Space, the window posts `WM_SYSCOMMAND` with `SC_KEYMENU` and the character itself, as `DefWindowProc` would have, but only for the seven menus' letters and the space, and only when the keymap binds nothing to the chord; the menus' letters never collide with a GUI Alt chord (ADR-0043's check). The message hook stays unused: a hidden accelerator table beside the keymap is the kind of second list the model exists to prevent.

### Choosing an item

A chosen item arrives through muda's handler as an id (`a:open`, or `d:` and a recent document's path), posted to the event loop. An open in-window dialog closes as Escape would close it, then the command runs as `Command::RunCommand`, so it joins the recent commands the palette lists first. The window's own commands (settings, colors, text size, font) join them too, through `App::remember_command`, made public for this.

### Keeping the menus current

After a command, the window compares a fingerprint of what the menus depend on (the settings, the keymap, the mode, edit mode, RSVP, the open document, and which pending commands have their module) with the one they were built from. Building the tree reads the settings schema for each toggle (a few milliseconds in a release build, about 45 in a debug one); comparing the fingerprint takes microseconds. Only when it changed is the tree built, and only when the tree changed are the menus rebuilt and attached again. A test checks that the fingerprint changes whenever the tree does over a run of commands.

### Dialogs the menus reach

- **Colors** (View, and File, Settings): every color setting in one form, the reading aids' highlights first, each row saying its color and its contrast where it is drawn as a ratio and a word, with a sample beside it that is never the only cue. Enter types a name or `#rrggbb`, Delete puts the theme's back, and Reset all colors resets every part, said once. The settings dialog's own Colors section shows the same rows.
- **Export and import settings:** the system's save and open dialogs (the same `rfd` chooser as Open), for TOML and JSON, answering the app's prompts. The import still asks first and names the first changes; the typed prompt takes over when no chooser can open.
- **The font list** is now the app's list model: the window hands the app its families (`App::show_frontend_list`) and reads the choice back (`App::take_frontend_choice`), so the list moves, jumps by letter, and is introduced like every other list.

### The window's own messages have levels

Every message the window says goes through `App::announce_as` with an `Importance`, so `[accessibility] interface_announcements` governs the window as it governs the app. A test reads the GUI's sources and fails on an announcement without a level.

## Alternatives rejected

- **A drawn menu bar in Masonry:** weeks of work to match what Windows gives, unproven with NVDA and JAWS. Revisit when Masonry has menus (xilem issue 1343).
- **muda on Linux:** brings GTK into a winit process, and both want the main loop.
- **Accelerators through the message hook:** two places handling keys, and a key in a dialog's field would run a menu command behind it.
- **The `windows` crate directly:** possible, and the fallback had muda fought winit's window procedure. It did not, and muda also covers macOS and the event plumbing.
- **Rebuilding the menus after every command:** the tree costs milliseconds per command; the fingerprint keeps typing and reading free of it.

## Consequences

- The UI Automation report checks the menu bar's seven menus and their access keys, and every item's text and key as the window's menu holds them (read back with `GetMenuStringW`). Opening a menu needs the foreground, so `-Menus` (opening the first menu to read its items through UI Automation) is for a test machine.
- The report also found that pressing a button through UI Automation brings the off-screen `--background` window to the front. It happens with a build from before the menus, so it is older than this work; the report shows it as a warning until it is fixed.
- A debug build of Masonry writes a full trace log to the system's temporary folder on every start; the report now points the GUI's temporary folder at its own.
- Commands appear in the menus as their modules register their handlers. Browse files (ADR-0045) and dictation (ADR-0042) have: the browser is the app's list in the window's list dialog, with its own keys asked of the app (`App::browse_list_key_for`) and Say Status previewing the focused row, and dictation needed nothing of the window. Batch conversion and audio export join as they merge.

## See also

- [The textweaver window](../gui.md)
- [ADR-0043: Menus and the palette from one model](0043-menus-and-the-palette-from-one-model.md)
