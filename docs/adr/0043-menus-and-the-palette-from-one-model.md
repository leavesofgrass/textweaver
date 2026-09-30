# ADR-0043: Menus and the palette from one model

- Status: accepted. Waiting for the terminal session (the F10 menus and the palette on the Braille display) and for the GUI's native menus (ADR-0046), which build on this model.
- Date: 2026-09-29
- Builds on: [ADR-0006](0006-keymap-and-actions.md) (the keymap and help from one table), [ADR-0024](0024-app-core-for-the-gui.md) (one app core for every frontend), [ADR-0020](0020-themes.md) (themes and contrast), and [ADR-0030](0030-interface-translations.md) (messages in six languages)

## Context

textweaver had no menus. Commands were reached by their keys, the command palette (a one-line prompt that completed ids), and the keyboard shortcuts list. A new user had no place to browse what the reader can do, and an experienced user had no place to find a command whose key they forgot, other than typing its id. The palette spoke `next_sentence: Move to the next sentence. Alt+.` and ranked matches by id prefix only.

Wave 6 adds menus to both frontends, gives the palette names and ranking, and adds four things the menus must show from the start: the owner's settings for colors, settings import and export, a way to turn textweaver's own announcements down, and keys on macOS that follow the Mac.

Star's lessons carry over: a shortcut shown in a menu or tooltip that is not bound (Star's toolbar advertised eight keys the GUI never bound), a checkmark restored from a stored flag for a mode that was not running, settings lost across a key rename, and one untyped "announcement" event a screen reader could not filter.

## Decision

### One model in the app core

`crate::menu` holds the menus: File, Edit, View, Reading, Speech, Tools, and Help, each an ordered list of commands, submenus, and separators, with no submenu over 21 items. Every label comes from the catalog in the interface's language: `menu-*` for menus, `name-*` for commands. The same `name-*` is the command's name in the palette. Every shortcut shown is read from the live keymap (`menu_chord`), so an override in `keymap.toml` shows at once and no menu can advertise an unbound key.

- **Access keys** are chosen per menu and per language: a catalog label may mark its letter with `&`; the rest take the first free letter starting a word, then any free letter. The seven top menus never take a letter a GUI `Alt` chord uses (Alt+O is the outline), in any language.
- **State is live.** A switch shows "checked" from the setting it flips, or for a mode that ends with the session (edit mode, RSVP, Speech Cursor) from the mode itself, never from a stored flag. A choice shows its value through the settings schema's own words ("Reading ruler: current line").
- **Commands provided by later modules** (browse files, batch conversion, audio export, dictation) have their ids, keys, and menu entries now. A module makes its command real with `App::register_handler`; until then the command is left out of the menus and the palette, and a key for it says the command is not in this version. A test checks that the commands missing from the menus are exactly these pending ones, so none can be forgotten or ship as a placeholder.
- **Tests** pin the menu bar's order, check that every command is in a menu, that no access key repeats in a menu, and that every label fits the first 40 cells of a Braille line, in six languages.

The terminal shows the model as a list on F10 (freed from previous chapter, which keeps Alt+PageUp): "Menus, 1 of 7, File", then items read name, state, keys. Enter or Right opens, a letter moves to its item as in a platform menu (it does not run it), Left or Backspace goes up, Escape closes. The GUI builds native menus from `App::menu_bar` (ADR-0046).

We rejected a drawn menu bar in the terminal (a screen reader sees characters and a moving cursor, not a menu) and separate menu definitions per frontend (they drift, as Star's tooltips did).

### The palette

Each candidate is one line, name first: "Export PDF, File: Export the document as a tagged PDF next to it." Matches rank exact name, name or id prefix, the query's letters starting the name's words in order (`ep` finds Export PDF, `exp pd` too), letters in order, then every word in the help; ties keep help order, and names match in the interface's language and English. With nothing typed, the last eight commands run from the palette or the menus come first, said as "recent". Ctrl+L shows the matches as a list. A scorer of our own was enough for about 200 commands; `nucleo-matcher` stays the answer if the palette grows to files.

### Interface announcements

Every message has an `Importance` (`textweaver_a11y::level`): error, question, answer, result, routine, dialog, progress, tip, hint, detail. `[accessibility] interface_announcements` (`auto`, `off`, `minimal`, `normal`, `full`; Ctrl+F9 cycles) lets a kind through or not. Errors, questions, and answers to what the user asked are never silenced. `auto` is minimal in screen-reader and hybrid modes, where the screen reader and the display already say what a list or window is, and normal when self-voicing.

The app's existing helpers keep their meaning (`tell` answers, `error`, `ask`, `note` for routine confirmations), and new ones name the rest. A test reads the app's sources and fails when anything outside the routing functions writes a message to the status line, the announcer, or the voice, so no message escapes the setting. A message held for a list or prompt that has closed is dropped (a dialog generation, checked when held messages are said and by `App::announce_for`), as NVDA drops speech for a closed dialog.

### Keys on macOS

The keymap gains a macOS column of logical keys, not a Ctrl-to-Cmd swap: Option moves by word and paragraph, Cmd with the arrows goes to the ends of the line and the document, Cmd+[ and Cmd+] go back and forward, Cmd runs commands, and an `Alt` chord with a letter becomes Cmd+Option (Option with a letter types a character on a Mac). A few commands have Mac keys written out where the translation would land on macOS's own keys (Cmd+Space, Cmd+H, Cmd+M, Cmd+`, Cmd+Shift+Q, F11) or where the Mac has its own (Cmd+T fonts, Cmd+D bookmark, Cmd+; next misspelling). No default uses VoiceOver's Ctrl+Option. The terminal keeps its keys on a Mac except where VoiceOver, Mission Control, or macOS's keyboard navigation take them. `text_motion` gives an edit field's caret keys per platform, so a GUI widget asks the keymap instead of swapping modifiers. Tests check every command has a Mac key in both frontends, that each follows the platform, and that none clashes with VoiceOver or macOS.

### Colors and settings

`[colors]` holds a color for each reading aid and part of the screen (the ruler, difficult words, syllable marks, misspellings, lint marks, search matches, the selection, the focus, links, headings, the status bar, notes, and bookmarks), beside the word and sentence highlights. Each is a named color, blue and orange first, or `#rrggbb`; red and green are not offered. Colors are laid over the theme's roles, every mark keeps the attribute that is not a color, and the settings screen says each color's contrast as a ratio and a word. A color under 3 to 1 is applied and warned about, never refused silently.

The settings screen lists the five settings changed last at the top, says the default Delete puts back, and says a row's help on F1. Import names its first changes before asking. Renamed keys keep their values on load and on import (`RENAMED_SETTINGS`, empty until a key is renamed, tested with a table of its own), because Star's loader dropped a key missing from its defaults before its migration saw it.

## Checks

- The menu and palette tests above, in six languages; the keymap's macOS, VoiceOver, Windows Terminal, and access-key tests.
- The source scan that keeps every announcement routed, and the tests that `off` still says errors and answers.
- A listening session on the terminal (session T6): F10, a menu's items, an access key, the palette's `ep`, and the recent commands.

## Consequences

- Four later modules register handlers in their own files; `menu.rs` does not change for them.
- The GUI's menus (ADR-0046) and color dialog use this model; the GUI honors the interface level through `App::announce_as`.
- Moving a command between menus is a one-line change with tests; adding a top menu is a deliberate change to a pinned test.
- About 250 catalog messages were added in six languages; native speakers should review the names.
- Mac keys are checked by tests but not yet by a person on a Mac; the first Mac listening session should try Option and Cmd with the arrows, and the Cmd+Option chords.

## See also

- [Keyboard reference](../keyboard.md): the GUI on macOS column and the terminal on macOS.
- [Reading and moving around](../reading.md): the menus, the palette, and interface announcements.
- [Settings](../settings.md): the settings screen and colors.
- [ADR index](README.md)
