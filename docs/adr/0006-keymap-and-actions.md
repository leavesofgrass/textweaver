# ADR-0006: Keymap, actions, and announcements

- Status: accepted
- Date: 2026-09-25

## Context

Star binds GUI shortcuts in menu code and stores remaps keyed by the default shortcut string, not by action. Its TUI hard-codes single keys, loses every Alt chord (on Unix, Esc followed by the key stops speech and discards the key; on Windows, PDCurses' `ALT_*` codes are unhandled), and cannot be reconfigured. Toolbar tooltips and cheat sheets disagree with the real bindings. Paperback keys everything by an action enum with TOML overrides.

## Decision

**Actions.** `ActionId` enumerates every user command. Each has a stable snake_case id (used in `keymap.toml` and the command palette), a `Category`, a one-line help string, and default chords per frontend. Help screens and `docs/keyboard.md` are generated from this table (`cargo xtask keyboard`), so documentation cannot drift from the bindings.

**Chords.** `KeyChord { key, mods }` parses and formats strings like `Ctrl+Shift+P`, `Alt+.`, `Ctrl++`, `F3`, `T`. Chords are normalized so terminal and GUI input compare equal: a character key never carries Shift (shifted letters are uppercase; other shifted characters are the character produced), `Char(' ')` is `Space`, `BackTab` is `Shift+Tab`. On the macOS GUI, default `Ctrl` chords become `Cmd`.

**Layers.** Every binding belongs to a layer:

| Mode | Lookup order |
|---|---|
| Browse (reading) | Browse, then Global |
| Speech Cursor | SpeechCursor, then Browse, then Global |
| Edit | Edit, then Global; unbound text keys type text |
| Find, Command | Global only; the prompt consumes text |

Browse-layer single keys (Star's TUI keys: `.` `,` `;` `p` `P` `[` `]` `r` `h` `{` `}` `<` `>` `t` `T` `n` `N` `H` `L` `j` `k`, plus new ones) are shared by both frontends, like a screen reader's browse mode. Global chords follow Star's GUI map (`Alt+.`, `Ctrl+P`, `Ctrl+H`, `Ctrl+T`, `Alt+Left`, ...). Terminal chords avoid what terminals cannot distinguish: `Ctrl+H` (Backspace), `Ctrl+I` (Tab), `Ctrl+M` (Enter). Crossterm reports Alt chords properly, which fixes Star's Esc/Alt bug by construction.

**Overrides.** `keymap.toml` maps action ids to chord lists (`next_sentence = ["Alt+.", "."]`; an empty list unbinds). An override replaces all of that action's bindings. Unknown ids and unparsable chords produce warnings, not failures.

**Conflicts.** `Keymap::conflicts()` reports a chord that reaches two actions in the same mode: bound twice in one layer, or bound in Global and in a mode layer. Speech Cursor bindings deliberately shadow Browse ones. The default maps must have no conflicts on either frontend and every platform (a test), and every action must have at least one default binding (a test).

**Announcements.** Every state change is announced through the `Announcer` trait (`a11y`): `SpeechAnnouncer` (self-voicing, a callback into the speech service), `StatusLineAnnouncer` (the TUI status line, which terminal screen readers read), `LogAnnouncer` (tests), and in wave 3 a live-region announcer for the GUI. `Verbosity { Low, Normal, High }` filters what is said. Star announced some changes only visually (rate changes, edit mode); textweaver announces all of them.

## Consequences

- One table drives bindings, help, the command palette, and documentation.
- Users can rebind anything in either frontend, including the TUI.
- Adding an action means adding one row to `action.rs`; the conflict and coverage tests keep the defaults sound.
