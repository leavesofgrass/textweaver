# Settings

This guide says where textweaver keeps its settings, lists every setting with its default, and explains how to export, share, import, and reset them.

textweaver can copy all of your settings and key changes into one JSON file. You can keep that file as a backup, move it to another computer, or share it with someone. Importing it checks everything first and keeps a backup of what it replaces.

You can also edit `settings.toml` yourself in any text editor. Close textweaver first, because it saves its settings when they change. Key changes go in `keymap.toml`; the [keyboard reference](keyboard.md#changing-keys) explains how to write them.

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

If the `TEXTWEAVER_HOME` environment variable is set, everything lives under that folder instead, in `config`, `data`, and `cache` subfolders. That is useful for a portable install on a USB stick. Every `tw settings` command also takes `--home FOLDER` for the same purpose.

Your reading positions, bookmarks, notes, recent files, and library live in the data folder, not with the settings. [The library](library.md) lists every folder.

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

It says how many settings will change and asks you to type `y` first. `--yes` skips the question. To reset only one section, add `--section` and the section name. The sections are `speech`, `speech.eci`, `speech.sapi`, `speech.apple`, `highlight`, `normalization`, `normalization.community_lexicon`, `reading`, `display`, `editing`, `library`, `keyboard`, `accessibility`, `export`, `reading_aids`, `reading_aids.rsvp`, `reading_aids.bionic_options`, `reading_aids.spacing`, `reading_aids.font`, `reading_aids.ruler`, `reading_aids.syllable_options`, and `keymap` (your key overrides). Sections you added yourself, such as `speech.dectalk`, can be reset by name too. A reset is backed up like an import.

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

## Every setting

This section lists every setting in `settings.toml`, section by section. Each entry gives the name, the default, the values it takes, and what it does. You only need to write the settings you change. To see all of them with their current values, export them:

```bash
tw settings export --format toml
```

A setting marked "not used yet" is stored and exported, but no part of textweaver reads it at present.

### [speech]

The voice and how it speaks. [Speech engines and voices](speech.md) explains these in more detail.

- `backend`, default `"auto"`: the speech engine's id, such as `"eci"`, `"sapi"`, `"espeak"`, or `"dectalk"`. `"auto"` picks the best available engine. Run `tw backends` to see the ids.
- `rate`, default `265`: words per minute, from 50 to 900.
- `pitch`, default `0`: semitones above or below the voice's own pitch, from -12 to 12.
- `volume`, default `100`: a percentage, from 0 to 100.
- `voice`, not set by default: the voice to use. Unset, textweaver chooses one.
- `prefer_voice`, default `"eloquence"`: when `voice` is not set, a voice whose name contains this text is preferred. An empty string means no preference.
- `favorite_voices`, default empty: your favourite voices, by id or name. Choose voice (Alt+V) lists them first, and Space in that list adds or removes one.
- `punctuation`, default `"some"`: how much punctuation is spoken. `"none"` speaks none, `"some"` speaks punctuation that carries meaning in prose (such as `@`, `#`, and `/`), and `"all"` speaks every punctuation character. Alt+Shift+N cycles it while textweaver runs, and saves it.
- `split_caps`, default `false`: speak the parts of words written in mixed capitals separately, such as "Java Script" for "JavaScript".
- `caps`, default `"pitch"`: how a capital letter is shown when a single character is spoken or echoed. `"none"`, `"tone"` (a short tone first), `"pitch"` (a higher pitch), or `"say_cap"` (the word "cap" first).
- `auto_play`, default `false`: start reading as soon as a document opens.
- `skip_code`, default `true`: do not read code blocks aloud.
- `latency_offset_ms`, default `120`: for engines whose word events carry audio times, how many milliseconds to wait before moving the highlight, so it matches what you hear.
- `verbosity`, default `"normal"`: how much textweaver says about state changes and structure. `"low"`, `"normal"`, or `"high"`. Alt+Shift+V cycles it while textweaver runs, and saves it. [Reading and moving around](reading.md) has examples.

### [speech.speed_presets]

Named rates for F8, which cycles through them. The defaults are `skim = 350`, `normal = 265`, `study = 200`, and `slow = 150`. Importing this table replaces all of it.

### [speech.eci]

ETI-Eloquence. See [the Eloquence guide](eloquence.md).

- `dictionaries`, default `true`: load the community pronunciation dictionaries. `false` loads none. A folder path loads the dictionaries in that folder.
- `library`, not set by default: the Eloquence engine library to load. Unset, textweaver searches the usual places.
- `code_factory`, default `false`: also look for Code Factory's Eloquence for Windows. Turn it on only if you own it.

The environment variables `TEXTWEAVER_ECI_LIBRARY`, `TEXTWEAVER_ECI_DICTIONARIES`, and `TEXTWEAVER_ECI_CODE_FACTORY` win over these settings.

### [speech.sapi]

Windows SAPI5 voices.

- `onecore`, default `true`: also list the OneCore voices, such as Microsoft Mark, David, and Zira.

### [speech.apple]

Apple's voices on macOS.

- `backend`, default `"auto"`: `"nsspeech"` (the quickest to respond), `"avspeech"` (the most exact word timing), or `"auto"`.

### [speech.dectalk]

DECtalk. This table is not written by an export unless you set it. See [the DECtalk guide](dectalk.md).

- `library`, not set by default: the DECtalk library to load. `TEXTWEAVER_DECTALK_LIBRARY` wins over it.

### [highlight]

The highlight that follows the reading.

- `enabled`, default `true`: show the reading highlight at all.
- `granularity`, default `"word"`: `"word"`, `"sentence"`, or `"both"` (the sentence, and the word inside it).
- `lead_words`, default `1`: move the drawn highlight ahead (a positive number) or behind (a negative number) by this many words, from -5 to 5. The default of 1 is the word being heard.
- `speed`, default `1.0`: for engines without word events, a multiplier on the estimated speed of the highlight, from 0.5 to 1.5.
- `color`, default `"theme"`: the colour of the band behind the word being read, laid over the theme's own. A name (`cyan`, `yellow`, `green`, `pink`, `orange`, `light blue`, and the common web colour names) or `#rrggbb`. `"theme"` keeps the theme's colour. The text in the band is the theme's text or page colour, whichever reads better, and the highlight keeps its bold or underline, so it never depends on colour alone. When the band leaves the text below 4.5 to 1 contrast (7 to 1 in high-contrast themes), textweaver says so at startup and when you change theme.
- `sentence_color`, not set by default: the same for the band behind the sentence being read.

### [normalization]

How text is turned into words before it is spoken. Engines that do this themselves, such as Eloquence, skip the parts they already do.

- `math`, default `true`: read LaTeX and ASCIIMath as spoken math. See [Math](math.md).
- `math_verbosity`, default `"normal"`: `"low"` ("a over b"), `"normal"`, or `"high"` (adds end markers such as "end fraction").
- `asciimath_delimiter`, not set by default: the character around ASCIIMath, usually a backtick. Unset, no ASCIIMath is read, because in Markdown a backtick marks code.
- `abbreviations`, default `true`: expand abbreviations such as "Dr." to "Doctor".
- `abbrev_expansions`, default empty: your own abbreviations, written as `"abbrev." = "expansion"`.
- `numbers`, default `true`: read numbers, dates, times, and money as words.
- `use_pronunciations`, default `true`: apply your pronunciation list.
- `pronunciations`, default empty: your pronunciation list, written as `term = "spoken form"`.
- `table_mode`, default `"structured"`: how tables are read. `"structured"` gives the row and column, `"flat"` reads only the cell text, and `"skip"` leaves tables out.
- `footnote_mode`, default `"inline"`: where footnotes are read. `"inline"` reads each at its reference, `"deferred"` at the end of the section, and `"skip"` leaves them out.

### [normalization.community_lexicon]

The community IBMTTS pronunciation dictionaries, used as a pronunciation list for engines other than Eloquence. Eloquence loads them itself.

- `enabled`, default `false`: use them.
- `dir`, not set by default: the folder with the `.dic` files. Unset, textweaver looks beside the program and in `TEXTWEAVER_ECI_DICTIONARIES`.
- `language`, default `"ENU"`: `"ENU"` for US English or `"DEU"` for German.

### [reading]

Reading and moving around. See [Reading and moving around](reading.md).

- `auto_resume`, default `true`: go back to your saved place when a document opens.
- `nav_history_size`, default `50`: how many places back and forward history keeps.
- `wrap_navigation`, default `false`: moving past the end of the document starts again at the beginning.
- `cursor_follows_speech`, default `true`: the cursor moves with the spoken word.
- `sync_conflict_policy`, default `"newest"`: when a library folder's sidecar and your own record of a position disagree, which one wins. `"newest"` (the newest time), `"highest_progress"` (the furthest position), or `"manual"` (keep both and ask). See [The library](library.md).
- `citations`, default `"off"`: what continuous reading does with a citation such as `[@doe2020, p. 12]`. `"off"` skips it (an in-text citation keeps its authors); `"words"` says it in words from your library, "Doe and Roe, 2020, page 12". Alt+Shift+Q switches it. Word moves say citations in words either way. See [Citations while reading](reading.md#citations-while-reading).
- `ocr`, default `true`: recognize the text of scanned pages and pictures (OCR). See [Scanned pages](converting.md#scanned-pages-ocr).
- `ocr_lang`, default `""`: the language of scanned text, as Tesseract codes (`"fra"`, `"deu+eng"`) or language tags (`"fr"`). Empty means the document's own language, else English. English is read by ocrs; other languages need Tesseract.
- `ocr_engine`, default `"auto"`: `"ocrs"`, `"tesseract"`, or `"paddle"` (experimental) to use one engine only.

### [display]

- `theme`, default `"galaxy"`: the colour theme. See [Themes](themes.md).
- `follow_os_theme`, default `true`: match the system's light, dark, or high-contrast setting at startup, unless you chose a theme.
- `theme_explicit`, default `false`: set by textweaver when you choose a theme; it stops following the system.
- `wrap_width`, default `0`: wrap lines at this many columns. 0 means the width of the terminal.
- `tab_width`, default `4`: columns per tab.
- `show_line_numbers`, default `false`: show line numbers. F6 turns them on and off.
- `scroll_margin`, default `3`: lines kept visible above and below the cursor.

### [editing]

Edit mode. See [Writing and editing](editing.md).

- `autosave_recovery`, default `true`: keep recovery snapshots while you edit, and offer them after a crash.
- `autosave_interval_secs`, default `20`: seconds between snapshots while there are unsaved changes.
- `echo_characters`, default `true`: speak each character you type.
- `echo_words`, default `true`: speak each word when you finish it.
- `echo_deletions`, default `true`: speak what you delete.
- `echo_lines_on_move`, default `true`: speak the line when the cursor moves to another line.
- `undo_steps`, default `1000`: the most undo steps kept while editing. The oldest are forgotten first. The smallest allowed value is 1.
- `undo_memory_mb`, default `50`: the most memory, in megabytes, the undo steps may use. The oldest are forgotten first; the newest step is always kept. The smallest allowed value is 1.
- `author`, not set by default: the author a new document from a template gets (`author = "Jo Writer"`). See [Start from a template](editing.md#start-from-a-template).

Your spelling word list is not a setting: it is `words.txt` in the data folder, one word per line. See [Spelling](editing.md#spelling).

### [library]

See [The library](library.md).

- `folders`, default empty: your library folders.
- `recent_limit`, default `20`: how many recent files are remembered.

### [keyboard]

- `character_keys`, default `true`: single-key shortcuts, such as `h` for the next heading. Set it to `false`, or press F9, so dictation or typing never triggers a command. See [the keyboard reference](keyboard.md#single-key-shortcuts).
- `preset`, default `"default"`: the set of keys to start from. `"default"` is the quick navigation of NVDA's and JAWS's browse mode (`h`, `1` to `6`, `l`, `i`, `t`, `k`, `q`, `s`, `g`, `d`); `"classic"` is textweaver's earlier keys. `"screen-reader"`, the old name of the default, still works. `keymap.toml` applies on top. See [the keyboard reference](keyboard.md#what-changed).
- `digit_row`, default `"auto"`: how the terminal knows the digit keys `1` to `6` (heading levels) when it gets only the typed character. `"auto"` knows the shifted digits of the US, UK, German, Spanish, Nordic, and Italian layouts; `"azerty"` is for French keyboards, where the digits need Shift. On Windows textweaver reads the digit key itself, and this setting does not matter.

### [accessibility]

How textweaver shares the work with a screen reader. See [Using textweaver with a screen reader](screen-readers.md).

- `mode`, default `"self-voicing"`: `"self-voicing"` (textweaver speaks everything), `"hybrid"` (textweaver reads documents aloud; your screen reader speaks messages, typing, and caret moves from the status line), or `"screen-reader"` (textweaver is silent). Alt+Shift+A cycles and saves it; `--mode` sets it for one run; `--no-speech` is screen-reader mode.
- `say_all`, default `"screen"`: continuous reading in screen-reader mode. `"screen"` moves a sentence at a time and puts each sentence on the status line at textweaver's rate; `"voice"` reads with textweaver's voice.
- `quiet_screen`, default `false`: while textweaver reads aloud, the title line's position stays still and the text being read is not copied to the status line.
- `cursor`, default `"follow"`: where the terminal's cursor waits. `"follow"` puts it on the spoken word, the caret, or the chosen item; `"status"` puts it on the status line, so your screen reader's "read current line" repeats the last message.
- `hybrid_offered`, default `false`: set after textweaver has asked, on its first run with a screen reader, whether to use hybrid mode. Set it back to `false` to be asked again.

### [export]

Audio export. See [Audio export](audio-export.md).

- `subtitle_format`, default `"srt"`: `"srt"` or `"vtt"`, used when subtitles are written without a file name.
- `subtitle_word_level`, default `false`: one subtitle cue per word instead of caption lines.
- `subtitles_with_audio`, default `false`: always write subtitles beside the exported audio.

### [reading_aids]

See [Reading aids](reading-aids.md).

- `bionic`, default `false`: bionic reading, the start of each word in bold. Alt+Shift+B turns it on and off.
- `syllables`, default `false`: show long words split into syllables, `read·a·bil·i·ty`. Alt+Shift+Z turns it on and off. The terminal reader draws it; the GUI does not yet.
- `difficult_words`, default `false`: underline rare words (SCOWL sizes above 50), and name them on word moves at high verbosity. Alt+Shift+J turns it on and off. The terminal reader draws it; the GUI does not yet.

### [reading_aids.bionic_options]

- `ratio`, default `0.4`: the part of each word in bold, from 0.1 to 0.9.
- `min_word_len`, default `2`: shorter words are left alone.
- `skip_numbers`, `skip_urls`, and `skip_code`, each default `true`: leave numbers, web addresses, and code alone.

### [reading_aids.rsvp]

RSVP shows one word at a time.

- `wpm`, default `300`: words per minute, from 50 to 1,500.
- `pacing`, default `"timer"`: `"timer"` uses the words-per-minute rate. `"external"` follows speech: it shows the word being spoken.
- `lead_words`, default `0`: when following speech, show a word this many words ahead.
- `clause_pause`, `sentence_pause`, and `paragraph_pause`, defaults `50`, `100`, and `150`: extra time, in percent of a word's time, after a comma, a sentence, and a paragraph.
- `long_word_len`, default `8`, `long_word_step`, default `10`, and `long_word_max`, default `80`: a word longer than `long_word_len` letters gets `long_word_step` percent more time per extra letter, up to `long_word_max` percent.
- `position`, default `"top-center"`: where the word box sits. `"top-left"`, `"top-center"`, `"top-right"`, `"center-left"`, `"center"`, `"center-right"`, `"bottom-left"`, `"bottom-center"`, or `"bottom-right"`.
- `show_previous` and `show_next`, default `true`: show the word before and the word after.
- `font_size_pt`, default `48`: the word's size in the GUI. The terminal uses its own font.

### [reading_aids.spacing]

Text spacing, in multiples of the font size. In the terminal, textweaver adds blank lines and spaces instead.

- `line_height`, default `1.5`.
- `paragraph_spacing`, default `1.0`.
- `letter_spacing`, default `0.0`.
- `word_spacing`, default `0.0`.

### [reading_aids.font]

The font in the GUI. The terminal always uses its own font.

- `family`, default `"sans"`: `"system-ui"`, `"sans"`, `"serif"`, `"monospace"`, a reading font (`"opendyslexic"`, `"atkinson"`, `"lexend"`), or the name of any installed font.
- `size_pt`, default `14.0`: the size in points, from 6 to 144.
- `weight`, default `400`: from 100 to 900. 700 is bold.
- `fetch_missing`, default `true`: offer to download a reading font that is missing, asking first. Not used yet: no part of textweaver offers the download.

### [reading_aids.ruler]

The reading ruler. Alt+Shift+U cycles it.

- `mode`, default `"off"`: `"off"`, `"current_line"`, or `"ruler"` (the line and a band around it).
- `rows_above` and `rows_below`, default `1`: the size of the band.
- `scope`, default `"line"`: `"line"` marks every row of a wrapped line; `"row"` marks one screen row.
- `mask_outside`, default `false`: dim everything outside the band.

### [reading_aids.syllable_options]

For the syllable display (`syllables` above).

- `separator`, default `"·"`: the character placed between syllables.
- `min_word_len`, default `4`, `left_min`, default `2`, and `right_min`, default `2`: the shortest word to split, and the fewest letters kept before the first break and after the last.
- `skip_urls` and `skip_code`, default `true`.

### [preview]

The browser preview of the document you are editing (`preview in browser` in the palette). See [the editing guide](editing.md#preview-in-the-browser).

- `auto_reload`, default `false`: reload the page by itself after each save, through a small server on this computer only (127.0.0.1, with a secret in the address), landing on the heading nearest the caret. Off, textweaver says "Preview updated. Press F5 in the browser." A reload moves your screen reader's place in the page, which is why it is off. The palette's `toggle preview auto reload` switches it.
- `live`, default `false`: with `auto_reload`, also reload when typing pauses for a second. The palette's `toggle preview live` switches it.

## See also

- [Speech engines and voices](speech.md): the speech settings in use.
- [Reading and moving around](reading.md): the reading and highlight settings.
- [Keyboard reference](keyboard.md): changing keys in `keymap.toml`.
- [Themes](themes.md): the `theme` setting and your own themes.
- [Reading aids](reading-aids.md): the `[reading_aids]` settings.
- [Troubleshooting](troubleshooting.md): when settings do not load.
- [Documentation index](README.md)
