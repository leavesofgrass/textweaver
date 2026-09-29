# ADR-0024: App core for the GUI

- Status: accepted
- Date: 2026-09-26 (Saturday, September 26, 2026)

## Context

The Xilem GUI (ADR-0027) and JSON-RPC need pieces the terminal reader kept to itself or did not need. The wxDragon spike loaded a whole document into a native control (9.3 s for 10 million characters) and polled speech every 30 ms. The list's focused item, first-letter jumps, the "3 of 12" announcements, and the prompt's text and history lived in `textweaver-tui`. There was no way to describe the settings to a dialog, no edit command a native text control could send, and a few slow jobs still ran on the input thread: opening a large file, writing `settings.toml`, the misspelling count after a save (0.6 s on 10 MB in a release build), and the speech engine's first start.

The spirit of this work is pure Rust first and bold choices with a recorded fallback. These are the choices.

## Decision

1. **The UTF-16 index is the rope.** `DisplayIndex` (ADR-0002) was a `Vec<u32>` of every char's UTF-16 offset: 40 MB and 1.1 s (debug build) for 10 million characters. ropey already keeps UTF-16 and byte counts in its tree, so `DisplayIndex` now holds a rope clone and answers in `O(log n)`: 29 µs to "build", no copy. It also maps UTF-8 bytes, which Parley and AccessKit use. *Fallback:* the old `Vec` index, if a ropey release drops its UTF-16 metrics.
2. **A document window** (`textweaver_app::window::DocWindow`). About 500,000 UTF-16 units around the focus, starting and ending on paragraph boundaries (else line starts, else a plain cut). It slides forward while reading, reporting what left the front and what joined the end, and recentres on jumps. It holds only a char range, so it is `Copy`; offsets in UTF-16, UTF-8, or chars are computed through the index when asked. `Session::revision` changes whenever the text changes, so a window made for older text reloads. On 10 million characters in a debug build: 80 µs to make a window, 0.8 ms to slice its text, 10 µs per word to follow the reading. *Fallback:* the frontend lays out the whole document (slow, but correct).
3. **Lists and prompts belong to the app** (`textweaver_app::list_model`). The app keeps the list shown (`ListModel`) and the prompt open (`PromptModel`), adopting them from the effects it returns, and handles `Command::ListKey` and `Command::PromptKey`. The terminal reader only maps keys and draws; the GUI's dialogs and JSON-RPC (`list_state`, `list_key`, `prompt_state`, `prompt_key`) use the same code, so every frontend says the same things. A GUI list that moves its own focus reports it quietly (`Command::ListFocus`), and a GUI text field sends its whole text (`PromptKey::SetText`). *Fallback:* a frontend may still keep its own state and send `Choose` and `Answer`, as before.
4. **A waker instead of polling.** `SpeechService::set_waker` takes a callback the speech thread calls after it sends statuses; `App::set_waker` passes it on, and the writer thread and every background job ring it too. A GUI posts an event to its event loop; the JSON-RPC server now sleeps up to 250 ms instead of polling every 20 ms. `App::tick_interval` says how long a frontend may sleep when nothing rings (the app's own timers: RSVP, autosave, the position save). *Fallback:* polling, which still works.
5. **`Command::ReplaceRange`** for edits made in a native text control: a range of the source and its replacement. One character typed at the caret joins the typing undo step; anything else is one step. Nothing is spoken, because the control and the screen reader already echo it.
6. **A settings schema generated from the store.** `SettingsSchema::generate` walks the store's own settings as JSON (`settings_to_json`), so every key, type, and default comes from the store, and joins each with a label, a help sentence, and its range, step, unit, or choices from one table (`INFO`). Tests fail when the store gains a key with no entry, loses one that has an entry, or a default falls outside its range, and every choice is checked to deserialize. `App::set_setting` checks a value against the store's types, clamps it with the store's `validate`, puts it into effect, and saves it. The schema drives a settings screen in the terminal reader (Settings: `Shift+F10`, or `Ctrl+,` in the GUI), the GUI's settings dialog, and JSON-RPC (`settings_schema`, `get_setting`, `set_setting`). *Rejected:* a derive macro on the store's types (it would put UI text in the store and a proc-macro in its build) and parsing the store's doc comments at build time (fragile). *Fallback:* labels made from the key names, which the schema already uses for a key with no entry.
7. **Nothing slow on the input thread.**
   - Files of 512 KiB or more open on a helper thread, with "Opening report.pdf. Escape cancels.", "Still opening report.pdf, 3 seconds.", and Escape to stop waiting. The loaders cannot be interrupted, so a cancelled load finishes on its own and is dropped. 10 MB of Markdown: the key returns in 0.3 ms instead of 2.7 s (debug build).
   - `settings.toml` is written by the writer thread; queued saves collapse into the newest. The key returns in 0.1 ms instead of waiting for the write (22 ms here).
   - The misspelling count after a save runs on a helper thread and is said when ready.
   - `App::start_speech_in_background` starts the first speech engine on a helper thread, as restarts already were; the terminal reader uses it. Messages said meanwhile are shown, the latest is spoken once the engine is ready, and a reading started meanwhile goes on.

## Consequences

- The Xilem GUI builds its document view on `DocWindow` and `Units::Utf8`, its dialogs on `ListModel` and `PromptModel`, its settings dialog on `SettingsSchema`, its event loop on the waker, and edit mode on `ReplaceRange`.
- The app now announces a list's focused item after the list's introduction, for every frontend; before, only the terminal reader did.
- A new setting in the store needs a line in `INFO` (label, help, range or choices), or the tests say which key is missing.
- `App::save_settings` and `App::update_settings` return before the file is written; callers that need the file call `App::wait_for_writes`.
- `DisplayIndex` keeps its API; the GUI spike's position mapping is unchanged.
