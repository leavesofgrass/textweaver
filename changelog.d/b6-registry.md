### For contributors

- A command a module registers with `App::register_handler` now runs its handler before the app's own command table, so a feature adds its commands in its own file and never edits `app.rs`. A command that a lean build leaves out is registered as not in this build (`App::register_command` with `Availability::NotInThisBuild`), which replaces the separate list of commands that need the MQTT client.
- A settings test now checks every number setting against the store: a value one step past either end of the range the settings screen shows comes back clamped to that same bound. The schema takes the CW, pause, Braille display and preview limits from the store's constants instead of repeating them.
- Removed declarations that did nothing: the unused workspace dependency `live-region`, the empty features `paperback` (formats) and `tts-crate` (speech), and two unused announcement helpers.
- The `mod` lines and the `pub use` block of `textweaver-app`'s `lib.rs` are sorted by name, and a test keeps them sorted, so new modules no longer all land at the end of the list.
