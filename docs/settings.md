# Settings: export, share, and import

textweaver can copy all of your settings and key changes into one JSON file. You can keep that file as a backup, move it to another computer, or share it with someone. Importing it checks everything first and keeps a backup of what it replaces.

## Where settings live

textweaver keeps two files in its settings folder:

- `settings.toml` holds your settings. Only values you changed are stored, so the file stays short.
- `keymap.toml` holds your key changes (key overrides).

To find the folder, type:

```bash
tw settings path
```

It prints the folder and both files, and says whether each file exists yet. The usual places are:

- Windows: `%APPDATA%\leavesofgrass\textweaver\config`
- macOS: `~/Library/Application Support/org.leavesofgrass.textweaver`
- Linux: `~/.config/textweaver`

If the `TEXTWEAVER_HOME` environment variable is set, everything lives under that folder instead. Every `tw settings` command also takes `--home FOLDER` for the same purpose.

## Export

Save every setting and key override to a file:

```bash
tw settings export my-settings.json
```

Without a file name, the export is printed instead. Other ways to export:

- `--changed-only` saves only the values that differ from the defaults. This is the easiest file to read and share.
- `--format toml` writes TOML instead of JSON. A file name ending in `.toml` does the same.

A full export lists every setting. A setting that is not set, such as a voice you never chose, appears as `null`.

## Import

Load settings from a file:

```bash
tw settings import my-settings.json
```

To see what would change first, without writing anything, add `--dry-run`. It prints one line per setting, for example:

```text
Dry run: nothing was written. Importing my-settings.json would make these changes. 2 settings change.
speech.rate changes from 265 to 300.
Keys for next_sentence change from the default keys to Alt+N, b:..
```

How import works:

- **Only what the file names changes.** A file that sets only `speech.rate` leaves every other setting alone. To make your settings exactly the file's instead, add `--replace`.
- **`null` means "use the default".** For example, `"voice": null` goes back to choosing a voice automatically.
- **A list or a map is one setting.** Importing `speed_presets` or `pronunciations` replaces the whole list, not single entries.
- **Everything is checked before anything is written.** A value of the wrong kind, or one out of range, stops the import. The message names each problem by its place in the file, such as `settings.speech.rate: 5000 is outside 50 to 900 words per minute`.
- **Unknown settings are kept.** A setting this version does not know, perhaps from a newer textweaver, is reported and saved as it is.
- **Your old files are backed up.** Before replacing `settings.toml` or `keymap.toml`, import copies it to a file such as `settings.toml.bak-20260925-140307` (the time is UTC). Import writes each file in one step, so an interrupted import never leaves half a file.
- **Other files work too.** You can import a TOML export, or a `settings.toml` copied from another computer.
- **Star settings need `tw migrate-star`.** A Star `settings.json` is recognized, and import tells you to use `tw migrate-star` instead.

Close textweaver before you import from the command line. Otherwise the running reader may save its own settings over the imported ones when you next change something.

## Reset

Return settings to their defaults:

```bash
tw settings reset
```

It says how many settings will change and asks you to type `y` first. `--yes` skips the question. To reset only one section, add `--section` and the section name. The sections are speech, speech.eci, speech.sapi, speech.apple, highlight, normalization, reading, display, editing, library, keyboard, and keymap (your key overrides). A reset is backed up like an import.

## Example file

This is what `tw settings export --changed-only` might print:

```json
{
  "exported": "2026-09-25T14:03:07Z",
  "keymap": {
    "next_sentence": [
      "Alt+N",
      "b:."
    ]
  },
  "settings": {
    "display": {
      "theme": "nord"
    },
    "highlight": {
      "granularity": "sentence"
    },
    "speech": {
      "rate": 300,
      "voice": "David"
    }
  },
  "textweaver_settings": 1
}
```

What each part means:

- `textweaver_settings` is the file format number, now 1. A file from a newer format is refused, so nothing is misread.
- `exported` is when the file was made, in UTC. Import ignores it.
- `settings` has one section per group, named as in `settings.toml`.
- `keymap` maps a command to its keys. An empty list, `[]`, removes all of a command's keys. `null` restores the default keys.

Keys are sorted and indented by two spaces, so two exports can be compared line by line.
