# ADR-0015: JSON-RPC server (`tw serve --stdio`)

- Status: accepted
- Date: 2026-09-25
- Status update (Saturday, September 26, 2026): Implemented as described. The serve loop pumps the macOS main run loop for `avspeech`, as the terminal reader does.

## Context

Students write in editors (VS Code, Neovim, Emacs, Obsidian) and read in textweaver. An editor plugin, a web page, or another frontend should be able to open a document, move through it by textweaver's units, have it read aloud with textweaver's voices and highlighting, and follow along, without reimplementing any of it. Star had no such interface; its GUI and TUI each carried their own copy of the reading logic.

The app core (`textweaver-app`) is already frontend-independent: frontends send `Command`s, act on `Effect`s, and poll speech status. A protocol only has to carry those across a process boundary.

## Decision

**Transport.** `tw serve --stdio` speaks **JSON-RPC 2.0** on stdin and stdout. Messages are newline-delimited JSON by default; a client may instead frame messages with LSP-style `Content-Length` headers, and the server answers in the framing of the last message it received. Nothing else is written to stdout (startup messages become `announcement` notifications); logs go to stderr. No sockets: a client that wants a daemon starts `tw serve` as a child process.

**Core and loop.** `textweaver_app::rpc::Server` is transport-free: `handle(message) -> [messages]` and `poll() -> [notifications]`, so tests drive it directly. `rpc::serve(server, reader, writer)` reads messages on a thread and runs the app on the calling thread, polling speech every 20 ms while waiting (ADR-0003: nothing blocks the app thread; engines that need the main thread get it, because `serve` runs there). Requests are answered in order. A closed stdin ends the server, saving the position.

**Speech.** The server voices speech itself exactly like the terminal reader (same settings, backend selection, and self-voicing; `--no-speech` for silence). Clients never receive audio. What a screen reader user would hear is also sent as `announcement` notifications, so a client can show or voice it in its own way.

**Methods.** Version 1 (`PROTOCOL_VERSION`):

| Method | Params | Result |
|---|---|---|
| `initialize` | none | `{server, version, protocol, methods, notifications}` |
| `open` | `{path}` | `{document: {title, path, format, length, lines, editing, dirty}, position}` |
| `status` | none | `{mode, playback, document, position, rate, backend, status, pending}` |
| `position` | none | `{char, line, column, percent, word: {start, end, text}}` (1-based line and column; char offsets are Unicode scalar values, ADR-0002) |
| `navigate` | `{action}`, a navigation, Speech Cursor, bookmark, or search action id (`next_sentence`, `skip_next_heading`, `next_bookmark`, ...); or `{goto}`: `"12"`, `"line 12"`, `"50%"`, `"start"`, `"end"`, `"char 120"` | `position` |
| `read` | `{what}`: `cursor` (default; continuous from the cursor), `document` (continuous from the start), `paragraph` (continuous from the current paragraph's start), or one unit in place: `sentence`, `line`, `word`, `character`, `selection`; optional `{from}` char offset to move to first | `{playback}` |
| `pause`, `resume`, `stop` | none | `{playback}` |
| `search` | `{pattern, regex?}` | `{matches: [{start, end, line}], current}`; the cursor moves to the first match at or after it, wrapping |
| `text` | `{start?, end?}` | `{text, start, end}` (canonical text) |
| `action` | `{id}`: any keymap action id, or a notes command name (`add_note`, `list_notes`, `toggle_highlight`, ...); optional `{confirm}` for an action that asks first (`quit`, `delete_note`): `true` answers yes, `false` no | `{status, effects, pending}`; without `confirm` such an action only asks, and `pending` is `{action, question}` (for example `"Quit textweaver? y or n"`) until answered by another `action` call with `confirm`, or by `cancel`; otherwise `pending` is null |
| `answer` | `{text}` | answers the open prompt; `{status, effects}` |
| `choose` | `{index}` | picks item `index` of the shown list; `{status, effects}` |
| `cancel` | none | closes the prompt or list, or answers no to a pending question; `{status, effects}` |
| `shutdown` | none | `null`; saves the position and settings; later requests fail with `-32003` |
| `exit` | none (normally a notification) | the server stops |

`effects` lists what the command asked the frontend to show, in the notification shapes below (`{type: "prompt" | "list" | "quit", params}`), so prompts and lists work over the protocol exactly as in the TUI: a client shows the prompt, then calls `answer`.

**Notifications** (server to client, no `id`):

| Method | Params | When |
|---|---|---|
| `position` | `{start, end, line}` | the spoken word moved (every word, applied one status at a time) |
| `playback` | `{state}`: `reading`, `paused`, `stopped` | reading started, paused, resumed, or ended |
| `announcement` | `{text, priority}`: `polite` or `assertive` | every app announcement, filtered by verbosity |
| `prompt` | `{label, purpose}` | a prompt opened (purpose in snake_case: `find`, `go_to`, `save_as`, `note_text`, ...) |
| `list` | `{title, items}` | a list opened (bookmarks, notes, help, Save / Discard / Cancel) |
| `quit` | `{}` | the app quit |

**Errors.** JSON-RPC's codes (`-32700` parse error, `-32600` invalid request, `-32601` method not found, `-32602` invalid params), and the server's own: `-32001` no document open, `-32002` the document could not be opened, `-32003` shut down.

**Versioning.** `initialize` reports `protocol`. Adding methods, notifications, params, or result fields keeps version 1; clients ignore fields they do not know. Renaming or removing anything, or changing a meaning, bumps the version.

## Consequences

- Any editor can drive textweaver with a few lines of plugin code; the reading logic, voices, pacing, and announcements stay in one place.
- The TUI, the GUI (wave 3), and RPC clients share `Command`/`Effect`; a new command is reachable over RPC through `action` at once.
- Char offsets are the one position unit; clients that address UTF-16 (VS Code, LSP) convert, as the GUI does with `DisplayIndex`.
- Edit mode is reachable (`action` with `toggle_edit_mode`, `save`, ...), but typing over RPC is not in version 1: editors edit their own buffers. A later version may add `insert`.
- On macOS, the serve loop must pump the main run loop for AVSpeechSynthesizer (ADR-0008), as the TUI loop does after the Apple integration.

## See also

- [JSON-RPC](../json-rpc.md): the user guide to `tw serve --stdio`, with a worked session.
- [Keyboard reference](../keyboard.md): the action ids the `action` and `navigate` methods take.
- [Architecture](../architecture.md): the crate map, the threads, and the path from a file to a spoken, highlighted word.
- [Documentation index](../README.md)
