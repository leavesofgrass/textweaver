# Settings

This guide describes where textweaver stores its settings, lists every setting with its default, and explains how to export, share, import, and reset them.

textweaver can export all settings and key changes to a single JSON file, which serves as a backup, as a means of moving settings to another computer, or as something to share. Importing a file validates everything before writing and keeps a backup of whatever it replaces.

Settings can also be edited directly in `settings.toml` with any text editor. textweaver should be closed first, because it saves its settings whenever they change. Key changes belong in `keymap.toml`; the [keyboard reference](keyboard.md#changing-keys) explains the syntax.

## The settings screen

Settings can be changed without leaving textweaver: press `Shift+F10` in the terminal reader (`Ctrl+comma` in the graphical version), or type `settings` in the command palette (`F2`).

Settings are grouped by task, in short sections, in the same order in the terminal reader and the graphical version:

1. Voice and speech: the speech engine, then only that engine's settings.
2. Pauses and verbosity.
3. Speaking text: numbers, code, tables, footnotes, abbreviations, and pronunciations.
4. Accessibility and announcements.
5. Reading.
6. Documents: citations, tracked changes, talking books, and scanned pages.
7. Highlight.
8. Math.
9. Braille.
10. Morse code: its sound, and keying.
11. Morse practice.
12. Display.
13. Colors.
14. Spacing, font and ruler.
15. Word aids: bionic reading, syllables, and difficult words.
16. Word by word (RSVP).
17. Editing, with the preview.
18. Library and study tools.
19. Dictation.
20. Keyboard and language.
21. Toolbars and panels (the graphical version's bars).
22. Export.
23. Sync.
24. Updates and components.

A section lists only the settings that apply now. Voice and speech follows the speech engine: with a speaking voice it lists the voice, rate, and pitch, and the options of that engine only (Eloquence's dictionaries and library only while Eloquence is the engine, for example); with CW, it lists the Morse code speeds, tone, and prosigns instead of a voice. Settings for another system are not listed: OneCore voices appear only on Windows, and the Apple speech engine only on macOS. Nothing is lost: All settings, the last item, lists every setting, and a search finds every setting. A setting that does not apply now says why after its value, such as "Eloquence library: not set. Applies when Eloquence is the speech engine." It can still be changed, and the change says the same.

In the terminal reader:

- The screen opens on the list of sections, each with its number of settings, such as "Morse practice, 4 settings". The five settings changed most recently come first, each announced as "recently changed", so a setting under adjustment is one key away. All settings is the last item.
- Enter or Right opens a section, and Escape goes back to the list of sections, on the section you left.
- Typing searches every setting, from the list of sections or from inside a section: "rate" keeps the settings with "rate" in their name, section, or help, and Backspace removes a letter.
- Up and Down move from setting to setting.
- Left and Right change the value: a smaller or larger number, the previous or next choice, or off and on.
- Enter toggles a switch, takes the next choice, or asks for a typed value, starting from the current one. Enter with the value unchanged or empty keeps it, as does Escape; for a setting that may be left unset, such as the voice, an empty answer unsets it.
- Delete restores the default and announces it: "Rate back to its default, 265 words per minute."
- F1 announces the setting's value, its default, and its help; on a section, how many settings it has.
- When a change alters what a section lists, such as choosing CW as the speech engine, the new count is said: "Voice and speech now lists 9 settings."
- Escape on the list of sections closes the screen.

The Settings dialog of the graphical version is described in [the guide to the textweaver app](gui.md#settings).

View, Colors (or `color settings` in the command palette) shows only the colors: the word and sentence highlights, the reading ruler, difficult words, syllable marks, misspellings, lint marks, search matches, the selection, the focus, links, headings, the status bar, notes, and bookmarks. Left and Right step through named colors, beginning with blue and orange, which people who confuse red and green can tell apart; Enter accepts a typed name or `#rrggbb` value; Delete restores the theme's color. Each row announces its contrast, such as "Links: orange, contrast 6.2 to 1, good". A color below 3 to 1 is still applied, with a warning that it is hard to see. Every mark keeps its underline, bold, or spoken word whatever its color, so no color carries meaning alone.

Every change is announced, such as "Rate, 285 words per minute.", takes effect immediately, and is saved. A number outside its range is set to the nearest value that fits, and textweaver says so. Choosing a speech engine switches to it at once by restarting speech with the new engine. A few other settings (such as the keys preset and an engine's library or dictionaries) apply from the next start, or after Restart Speech (`Shift+F8`), and textweaver says which. Tables of names and values, such as pronunciations, are edited in `settings.toml`.

The sections only change how settings are presented: `settings.toml`, `tw settings`, and JSON-RPC keep the same keys and tables as before.

## Where settings live

textweaver keeps two files in its settings folder:

- `settings.toml` holds the settings. Only values that differ from the defaults are stored, so the file stays short.
- `keymap.toml` holds key changes (key overrides).

The folder is found with:

```bash
tw settings path
```

The command prints the settings folder and both files, stating whether each exists yet, followed by the data, state, and recovery folders and the log file. `--json` prints the same paths as JSON. The usual locations are:

- Windows: `%APPDATA%\leavesofgrass\textweaver\config`
- macOS: `~/Library/Application Support/org.leavesofgrass.textweaver`
- Linux: `~/.config/textweaver`

If the `TEXTWEAVER_HOME` environment variable is set, everything lives under that folder instead, in `config`, `data`, and `cache` subfolders, which suits a portable installation on a USB stick. Every `tw settings` command also accepts `--home FOLDER` for the same purpose.

Reading positions, bookmarks, notes, recent files, and the library live in the data folder, apart from the settings. [The library](library.md) lists every folder.

## Export

Save every setting and key override to a file:

```bash
tw settings export my-settings.json
```

Without a file name, the export is printed instead. Two options modify it:

- `--changed-only` saves only the values that differ from the defaults, which gives the file that is easiest to read and share.
- `--to toml` writes TOML instead of JSON; a file name ending in `.toml` has the same effect.

A full export lists every setting, and a setting that is not set, such as a voice never chosen, appears as `null`.

## Import

Load settings from a file:

```bash
tw settings import my-settings.json
```

`--dry-run` shows what would change without writing anything, printing one line per setting, for example:

```text
Dry run: nothing was written. Importing my-settings.json would make these changes. 2 settings change.
speech.rate changes from 265 to 300.
Keys for next_sentence change from the default keys to Alt+N, b:..
```

How import works:

- **Only what the file names changes.** A file that sets only `speech.rate` leaves every other setting alone. `--replace` makes the settings exactly those of the file instead.
- **`null` means "use the default".** For example, `"voice": null` returns to automatic voice selection.
- **A list or a map is one setting.** Importing `speed_presets` or `pronunciations` replaces the whole list, not individual entries.
- **Everything is validated before anything is written.** A value of the wrong kind or out of range stops the import, and the message names each problem by its place in the file, such as `settings.speech.rate: 5000 is outside 50 to 900 words per minute`.
- **Unknown settings are kept.** A setting this version does not recognize, perhaps from a newer textweaver, is reported and saved unchanged.
- **Existing files are backed up.** Before replacing `settings.toml` or `keymap.toml`, import copies it to a file such as `settings.toml.bak-20260925-140307` (the time is UTC). Each file is written in one step, so an interrupted import never leaves a partial file.
- **Other files are accepted.** A TOML export, or a `settings.toml` copied from another computer, can be imported.
- **star settings need `tw migrate-star`.** A star `settings.json` is recognized, and import directs you to `tw migrate-star` instead.

textweaver should be closed before importing from the command line; otherwise the running reader may save its own settings over the imported ones at its next change.

## Reset

Return settings to their defaults:

```bash
tw settings reset
```

The command reports how many settings will change and asks for `y` to confirm; `--yes` skips the question. `--section` followed by a section name resets only that section. The sections are `speech`, `speech.eci`, `speech.sapi`, `speech.apple`, `speech.dectalk`, `speech.espeak`, `speech.piper`, `speech.cw`, `highlight`, `normalization`, `normalization.community_lexicon`, `normalization.medical_lexicon`, `reading`, `display`, `editing`, `library`, `keyboard`, `accessibility`, `export`, `braille`, `reading_aids`, `reading_aids.rsvp`, `reading_aids.bionic_options`, `reading_aids.spacing`, `reading_aids.font`, `reading_aids.ruler`, `reading_aids.syllable_options`, `preview`, `lexicon`, `stats`, `summary`, `dictation`, `interface`, `gui`, `colors`, and `keymap` (your key overrides). Sections not in this list, added by a newer textweaver or a build with extra features, can be reset by name too. A reset is backed up in the same way as an import.

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

- `textweaver_settings` is the file format number, currently 1. A file in a newer format is refused, so nothing is misread.
- `exported` is the time the file was made, in UTC; import ignores it.
- `settings` has one section per group, named as in `settings.toml`.
- `keymap` maps a command to its keys. An empty list, `[]`, removes all of a command's keys, and `null` restores the default keys.

Keys are sorted and indented by two spaces, so two exports can be compared line by line.

## Every setting

This section lists every setting in `settings.toml`, in the sections the settings screens use. Each entry gives the key, the default, the values it accepts, and its effect. A key names its table in the file and its name there: `speech.rate` is written as `rate` under `[speech]`, and `speech.cw.wpm` as `wpm` under `[speech.cw]`. The [settings reference](settings-reference.md) begins with a list of which sections hold each table's keys. Only settings that differ from their defaults need to be written. All settings with their current values can be listed by exporting them:

```bash
tw settings export --to toml
```

A setting marked "not used yet" is stored and exported, but nothing in textweaver currently reads it.

### Voice and speech

The speech engine, and the settings of the engine in use. On the settings screens this section lists only what applies to the chosen engine: the voice, rate, and pitch for a speaking voice; the engine's own options, such as Eloquence's dictionaries, only while that engine is chosen; and, while CW is the engine, the sound of Morse code (from [Morse code](#morse-code)) instead of the voice. SAPI 5's OneCore voices appear only on Windows, and the Apple engine choice only on macOS. [Speech engines and voices](speech.md), [the Eloquence guide](eloquence.md), [the DECtalk guide](dectalk.md), and [CW: reading in Morse code](cw.md) explain the engines.

- `speech.backend`, default `"auto"`: the speech engine's id, such as `"eci"`, `"sapi"`, `"espeak"`, or `"dectalk"`. `"auto"` picks the best available engine. Run `tw backends` to see the ids.
- `speech.voice`, not set by default: the voice to use. Unset, textweaver chooses one.
- `speech.prefer_voice`, default `"eloquence"`: when `speech.voice` is not set, a voice whose name contains this text is preferred. An empty string means no preference.
- `speech.favorite_voices`, default empty: your favorite voices, by id or name. Choose voice (Alt+V) lists them first, and Space in that list adds or removes one.
- `speech.rate`, default `265`: words per minute, from 50 to 900.
- `speech.pitch`, default `0`: semitones above or below the voice's own pitch, from -12 to 12.
- `speech.volume`, default `100`: a percentage, from 0 to 100.
- `speech.cw.prosigns`, default `true`: send structure as prosigns (BT after headings and paragraphs, AR at the end of the document), and send prosigns written in angle brackets, such as `<BT>`.
- `speech.cw.status_line`, default `true`: show the characters being sent on the status line, so a screen reader can show them on a Braille display. Nothing extra is announced.
- `speech.eci.dictionaries`, default `true`: load the community pronunciation dictionaries. `false` loads none. A folder path loads the dictionaries in that folder.
- `speech.eci.library`, not set by default: the Eloquence engine library to load. Unset, textweaver searches the usual places.
- `speech.eci.code_factory`, default `false`: also look for Code Factory's Eloquence for Windows. Turn it on only if you own it.
- `speech.sapi.onecore`, default `true`: also list the OneCore voices, such as Microsoft Mark, David, and Zira.
- `speech.apple.backend`, default `"auto"`: `"nsspeech"` (the quickest to respond), `"avspeech"` (the most exact word timing), or `"auto"`.
- `speech.dectalk.library`, not set by default: the DECtalk library to load. `TEXTWEAVER_DECTALK_LIBRARY` wins over it.
- `speech.espeak.helper`, default `"auto"`: where eSpeak NG runs. `"auto"` uses its helper program on Windows when the helper and an installed eSpeak NG are found, and runs it inside textweaver elsewhere; `"always"` uses only the helper program, and `"never"` only runs it inside textweaver.
- `speech.piper.voice`, not set by default: the Piper voice to start with, by id, such as `"en_US-amy-medium"`. Unset, the first installed voice.
- `speech.piper.voices`, not set by default: the folder of Piper voices. Unset, textweaver uses the `piper/voices` folder in its data folder. `TEXTWEAVER_PIPER_VOICES` wins over it.
- `speech.piper.phonemizer`, default `"auto"`: how Piper turns text into sounds. `"library"` uses the espeak-ng library, `"rust"` textweaver's own, and `"auto"` the library when it is installed.
- `speech.speed_presets`: Named rates for F8, which cycles through them. The defaults are `skim = 350`, `normal = 265`, `study = 200`, and `slow = 150`. Importing this table replaces all of it.
- `speech.voices_by_language`: The voice to use for each interface language, by language tag, such as `es = "eci:es"`. When `[interface] language` changes to a language listed here, its voice is used. A language not listed keeps the current voice if it speaks the language, else takes the engine's voice for it (one whose name contains `prefer_voice` first, so Eloquence when you have it). Empty by default. `tw voices` lists the voice ids.
- `speech.voice_params`: The rate and pitch each voice was last used at, so choosing a voice again brings them back. textweaver writes this table whenever the voice changes, so it needs no manual editing. Each entry is keyed by engine and voice, such as `"sapi:David" = { rate = 310, pitch = -1 }`.
- `speech.output_device`, not set by default: the sound device speech plays on, by the id `tw backends --devices` prints. Unset, or when that device is not connected, speech plays on the system's default. The [speech guide](speech.md#choose-the-sound-device) has the steps.

The environment variables `TEXTWEAVER_ECI_LIBRARY`, `TEXTWEAVER_ECI_DICTIONARIES`, and `TEXTWEAVER_ECI_CODE_FACTORY` win over the Eloquence settings. A change to an engine's own settings (`[speech.eci]`, `[speech.dectalk]`, `[speech.espeak]`, or `[speech.piper]`) takes effect when speech restarts.

### Pauses and verbosity

How much textweaver says, how it marks punctuation and capitals, and the silences between parts of a document.

- `speech.verbosity`, default `"normal"`: how much textweaver says about state changes and structure. `"low"`, `"normal"`, or `"high"`. Alt+Shift+V cycles it while textweaver runs, and saves it. [Reading and moving around](reading.md) has examples.
- `speech.punctuation`, default `"some"`: how much punctuation is spoken. `"none"` speaks none, `"some"` speaks punctuation that carries meaning in prose (such as `@`, `#`, and `/`), and `"all"` speaks every punctuation character. Alt+Shift+N cycles it while textweaver runs, and saves it.
- `speech.caps`, default `"pitch"`: how a capital letter is shown when a single character is spoken or echoed. `"none"`, `"tone"` (a short tone first), `"pitch"` (a higher pitch), or `"say_cap"` (the word "cap" first).
- `speech.split_caps`, default `false`: speak the parts of words written in mixed capitals separately, such as "Java Script" for "JavaScript".
- `speech.pause_heading_ms`, default `400`; `speech.pause_paragraph_ms`, default `300`; `speech.pause_list_item_ms`, default `150`: how many milliseconds of silence follow a heading, a paragraph, and a list item while reading, from 0 to 3000. 0 turns that pause off. Faster rates than 265 words per minute shorten them in proportion. They travel with synced settings. The [speech guide](speech.md#pauses-at-headings-paragraphs-and-list-items) explains how they work with each engine.
- `speech.markup_pauses`, default `true`: read pause markup written in a document, such as `<break time="1s"/>`, as a pause. Turn it off for documents that quote SSML. The [speech guide](speech.md#pauses-written-in-the-text) explains it.

### Speaking text

How text is turned into words before it is spoken: numbers, code, tables, footnotes, abbreviations, and pronunciation lists. Engines that do part of this themselves, such as Eloquence, skip the parts they already do.

- `normalization.numbers`, default `true`: read numbers, dates, times, and money as words.
- `speech.skip_code`, default `false`: when on, code blocks are not read aloud (each is said as "code block skipped"). Off by default, so README files and getting-started guides read in full.
- `normalization.table_mode`, default `"structured"`: how tables are read. `"structured"` gives the row and column, `"flat"` reads only the cell text, and `"skip"` leaves tables out.
- `normalization.footnote_mode`, default `"inline"`: where footnotes are read. `"inline"` reads each at its reference, `"deferred"` at the end of the section, and `"skip"` leaves them out.
- `normalization.abbreviations`, default `true`: expand abbreviations such as "Dr." to "Doctor".
- `normalization.abbrev_expansions`, default empty: your own abbreviations, written as `"abbrev." = "expansion"`.
- `normalization.use_pronunciations`, default `true`: apply your pronunciation list.
- `normalization.pronunciations`, default empty: your pronunciation list, written as `term = "spoken form"`.
- `normalization.community_lexicon.enabled`, default `false`: use the community IBMTTS pronunciation dictionaries as a pronunciation list for engines other than Eloquence, which loads them itself.
- `normalization.community_lexicon.language`, default `"ENU"`: `"ENU"` for US English or `"DEU"` for German.
- `normalization.community_lexicon.dir`, not set by default: the folder with the `.dic` files. Unset, textweaver looks beside the program and in `TEXTWEAVER_ECI_DICTIONARIES`.
- `normalization.medical_lexicon.enabled`, default `false`: use the built-in medical pronunciation list: drug names, clinical terms, eponyms, and safe dosing abbreviations such as PRN and q6h. See [Clinical and scientific text](speech.md#clinical-and-scientific-text).
- `normalization.medical_lexicon.overlay`, not set by default: your own file of medical pronunciations, written as `term = "spoken form"`, which win over the built-in ones. Unset, textweaver reads `medical-lexicon.toml` in the settings folder, if there is one.

### Accessibility and announcements

How textweaver shares the work with a screen reader, and how much it says about itself. See [Using textweaver with a screen reader](screen-readers.md).

- `accessibility.mode`, default `"self-voicing"`: `"self-voicing"` (textweaver speaks everything), `"hybrid"` (textweaver reads documents aloud; your screen reader speaks messages, typing, and cursor moves from the status line), or `"screen-reader"` (textweaver is silent). Alt+Shift+A cycles and saves it; `--mode` sets it for one run; `--no-speech` is screen-reader mode.
- `accessibility.interface_announcements`, default `"auto"`: how much textweaver says about itself: `"off"`, `"minimal"`, `"normal"`, or `"full"`; `"auto"` is minimal in screen-reader and hybrid modes and normal when self-voicing. Errors, questions, and answers to what you asked are always said. Ctrl+F9 cycles it. See [Interface announcements](reading.md#interface-announcements-ctrlf9).
- `accessibility.say_all`, default `"screen"`: continuous reading in screen-reader mode. `"screen"` moves a sentence at a time and puts each sentence on the status line at textweaver's rate; `"voice"` reads with textweaver's voice.
- `accessibility.quiet_screen`, on by default in hybrid mode and off in the other modes: while textweaver reads aloud, the title line's position stays still and the text being read is not copied to the status line. Set `true` or `false` to choose for every mode; leave it out (or set `"auto"`) to follow the mode.
- `accessibility.cursor`, default `"follow"`: where the terminal's cursor waits. `"follow"` puts it on the spoken word, the editing cursor, or the chosen item; `"status"` puts it on the status line, so your screen reader's "read current line" repeats the last message.
- `gui.speak_messages`, default `false`: when textweaver reads aloud, also say its messages, typing, and cursor moves in its own voice, for reading by ear without a screen reader (Settings, "Speak textweaver's messages").
- `gui.announce`, default `"live"`: how the graphical version's messages reach the screen reader, from the next start. `"live"` uses a live region, which NVDA and JAWS both speak. `"uia"` raises UI Automation notifications instead (Windows only; elsewhere the live region is used). `--announce live` or `--announce uia` on the command line wins over the setting for one run.

### Reading

Starting, moving, and stopping while reading. See [Reading and moving around](reading.md).

- `speech.auto_play`, default `false`: start reading as soon as a document opens.
- `reading.auto_resume`, default `true`: go back to your saved place when a document opens.
- `reading.cursor_follows_speech`, default `true`: the cursor moves with the spoken word.
- `reading.wrap_navigation`, default `false`: moving past the end of the document starts again at the beginning.
- `reading.nav_history_size`, default `50`: how many places back and forward history keeps.
- `reading.stop_at`, default `"off"`: where continuous reading stops by itself and says "End of section." `"off"` never stops, `"heading"` stops at the next heading of any level, and `"chapter"` at the next chapter (a section break, else a level 1 heading). The read key goes on from there.
- `reading.stop_after_minutes`, default `0`: stop at the end of a sentence after this many minutes of reading, from 0 to 240 in steps of 5. Pausing stops the clock. 0 turns the timer off.
- `reading.after_end`, default `"stop"`: what follows the end of a document when the reading queue has another. `"stop"` does nothing more; `"ask"` names the next document, and the read key opens and reads it; `"next"` opens and reads it at once. A document is never started unless you ask ([The reading queue](library.md#the-reading-queue)).
- `reading.recall_prompts`, default `false`: at a section end, ask you to say what you remember before going on. It stops at the next heading for this when `stop_at` is `"off"`. See [Study with textweaver](notes.md#study-with-textweaver).

### Documents

How the content of a document is read: citations, tracked changes, Obsidian comments, the recorded narration of talking books, and scanned pages.

- `reading.citations`, default `"off"`: what continuous reading does with a citation such as `[@doe2020, p. 12]`. `"off"` skips it (an in-text citation keeps its authors); `"words"` says it in words from your library, "Doe and Roe, 2020, page 12". Alt+Shift+Q switches it. Word moves say citations in words either way. See [Citations while reading](reading.md#citations-while-reading).
- `reading.revisions`, default `"auto"`: how tracked changes in Word, OpenDocument, and RTF files are read. `"auto"` says each change in place ("deleted by Ada Example: three") at high verbosity and reads the final text otherwise; `"marked"` always says them; `"final"` never does. It applies when a document is opened. See [Converting documents](converting.md).
- `reading.read_comments`, default `false`: read the comments in Obsidian notes, the text between `%%` marks, without the marks. Off leaves them out, as Obsidian's reading view does. Converted files leave comments out whatever this says; `tw convert --keep-comments` keeps them ([Obsidian notes](converting.md#obsidian-notes)). Syncs between computers.
- `reading.book_audio`, default `"auto"`: what reads a DAISY talking book that has a recording. `"auto"` plays its recorded narration and uses speech for text without audio; `"speech"` reads everything with speech. Rate changes do not apply to a recording ([Talking books](reading.md#talking-books-the-recorded-narration)).
- `reading.ocr`, default `true`: recognize the text of scanned pages and pictures (OCR). See [Scanned pages](converting.md#scanned-pages-ocr).
- `reading.ocr_lang`, default `""`: the language of scanned text, as Tesseract codes (`"fra"`, `"deu+eng"`) or language tags (`"fr"`). Empty means the document's own language, else English. English is read by ocrs; other languages need Tesseract.
- `reading.ocr_engine`, default `"auto"`: `"ocrs"`, `"tesseract"`, or `"paddle"` (experimental) to use one engine only.

### Highlight

The highlight that follows the reading, and its timing.

- `highlight.enabled`, default `true`: show the reading highlight at all.
- `highlight.granularity`, default `"word"`: `"word"`, `"sentence"`, or `"both"` (the sentence, and the word inside it).
- `highlight.lead_words`, default `1`: move the drawn highlight ahead (a positive number) or behind (a negative number) by this many words, from -5 to 5. The default of 1 is the word being heard.
- `highlight.speed`, default `1.0`: for engines without word events, a multiplier on the estimated speed of the highlight, from 0.5 to 1.5.
- `speech.latency_offset_ms`, default `120`: for engines whose word events carry audio times, how many milliseconds to wait before moving the highlight, so it matches what you hear.
- `highlight.color`, default `"theme"`: the color of the band behind the word being read, laid over the theme's own. A name (`cyan`, `yellow`, `green`, `pink`, `orange`, `light blue`, and the common web color names) or `#rrggbb`. `"theme"` keeps the theme's color. The text in the band is the theme's text or page color, whichever reads better, and the highlight keeps its bold or underline, so it never depends on color alone. When the band leaves the text below 4.5 to 1 contrast (7 to 1 in high-contrast themes), textweaver says so at startup and when you change theme.
- `highlight.sentence_color`, not set by default: the same for the band behind the sentence being read.
- `highlight.palette`, default five names (important, define, question, example, review): your highlight names, up to eight, each with a color and a shape (`name`, `color`, `shape`). The first five have the keys Alt+1 to Alt+5. It is a table you edit in the file. See [Highlights](notes.md#highlights).

### Math

Spoken math and how math looks on screen. See [Math](math.md). The braille code for math is under [Braille](#braille).

- `normalization.math`, default `true`: read LaTeX and ASCIIMath as spoken math. See [Math](math.md).
- `normalization.math_verbosity`, default `"normal"`: `"low"` ("a over b"), `"normal"`, or `"high"` (adds end markers such as "end fraction").
- `reading.math_engine`, default `"builtin"`: which engine reads math aloud. `"builtin"` is textweaver's own; `"mathcat"` is MathCAT in ClearSpeak and `"mathcat_simplespeak"` MathCAT in SimpleSpeak, in the document's language. MathCAT needs a build with the `mathcat` feature; without it, textweaver's own is used. See [Hear math with MathCAT](math.md#hear-math-with-mathcat).
- `reading.math_display`, default `"source"`: how math looks in the reading view. `"source"` shows it as written (`$x^2$`); `"unicode"` draws each formula as one line of Unicode (`x²`, `√2`, `1⁄2`), as star did. Speech, edit mode, and exploring a formula always use the source. See [See math as Unicode](math.md#see-math-as-unicode).
- `normalization.asciimath_delimiter`, not set by default: the character around ASCIIMath, usually a backtick. Unset, no ASCIIMath is read, because in Markdown a backtick marks code.

### Braille

Braille output and input: BRF files written and read, math braille on the display, and the textweaver app's Braille view. See [Math in braille files](math.md#math-in-braille-files), [Converting documents](converting.md), and [The Braille view](reading-aids.md#the-braille-view).

- `braille.grade`, default `"uncontracted"`: uncontracted (grade 1) or contracted (grade 2, `"contracted"`) Unified English Braille, for BRF files exported from the reader and for the Braille view. Contracted braille goes through liblouis with the BRF writer's table, `en-ueb-g2.ctb`; without liblouis, uncontracted braille is used and textweaver says so. Alt+Shift+F3 switches it.
- `braille.display_cells`, default `40`: how many cells your Braille display has, from 8 to 120. The Braille view shows this many cells at a time. Stays on this computer, since each computer may have its own display.
- `braille.brf_code`, default `"ueb"`: the braille code BRF files are read in when you open one and textweaver translates it back to print through liblouis. `"ueb"` is Unified English Braille, the code of books produced since 2016; `"ebae"` is English Braille American Edition, for older American books. Open the file again after a change. See [Braille files](reading.md#braille-files-brf).
- `braille.table_format`, default `"linear"`: how BRF files lay out tables (BANA's Braille Formats, 2016, section 11). `"linear"` is one row per line, entries separated by semicolons; `"listed"` gives each row a cell-5 heading, then each entry on its own line after its column heading; `"stairstep"` sets each row's entries two cells further right than the one before, for tables of up to four columns (a wider table is listed instead, with a warning). `tw convert --table-format` sets it for one conversion.
- `braille.math_code`, default `"nemeth"`: the braille code math is written in, in BRF files and while exploring a formula. `"nemeth"` is the Nemeth Code inside UEB text, with the Nemeth switch indicators; `"ueb"` is Unified English Braille's own mathematics. Needs a build with MathCAT; without it, math is written as its spoken words in uncontracted braille.
- `braille.viewer`, default `false`: the Braille view, the current line or spoken sentence drawn as braille dots at the bottom of the textweaver app (in the terminal reader, a row of Unicode braille in self-voicing mode). Visual only. Alt+Shift+F2 turns it on and off.
- `braille.viewer_print_line`, default `false`: under each braille cell in the Braille view, the print letter or contraction it stands for.
- `braille.viewer_cell_outlines`, default `false`: an outline around each cell in the Braille view, so blank cells and spacing show.

### Morse code

The sound of Morse code and keying with a key or paddle. See [CW: reading in Morse code](cw.md) and [CW keying](cw.md#key-morse-code-into-a-document). The first four settings, the speeds, the tone, and its level, are used by keying and practice whatever the speech engine; while CW is the speech engine, the settings screens list them at the top of [Voice and speech](#voice-and-speech) instead. The keyer runs at the character speed `speech.cw.wpm`.

- `speech.cw.wpm`, default `20`: the character speed, 5 to 50 words per minute. While CW is the engine, the rate keys change it by one, and `effective_wpm` with it in proportion.
- `speech.cw.effective_wpm`, default `10`: the effective (Farnsworth) speed, 5 to 50 words per minute. The gaps between characters and words stretch so the text goes at this speed; at or above `wpm`, the spacing is standard.
- `speech.cw.tone_hz`, default `600`: the tone, 200 to 1500 hertz. The pitch keys move it by semitones.
- `speech.cw.volume`, default `50`: the tone's level, 0 to 100 percent of full scale, before the speech volume.
- `speech.cw.keyer`, default `"iambic_b"`: how a paddle's contacts become dits and dahs. `"iambic_b"` is iambic mode B, `"iambic_a"` iambic mode A, both with dot and dash memory; `"straight"` keys the tone from either contact, as a straight key. The keyer runs at the character speed `wpm`.
- `speech.cw.swap_paddles`, default `false`: swap dit and dah, from MIDI and from keyboard adapters, for a paddle used with the other hand.
- `speech.cw.sidetone`, default `true`: play the CW tone, at `tone_hz` and `volume`, while the key is down.
- `speech.cw.dit_note`, default `20`; `speech.cw.dah_note`, default `21`; `speech.cw.straight_note`, default `30`: the MIDI notes of the dit and dah contacts and of a straight key, 0 to 127. The defaults are the N6ARA TinyMIDI's and the CTR2 MIDI's.
- `speech.cw.midi_channel`, default `0`: the MIDI channel keying listens to, 1 to 16, or 0 for any.

A change to `[speech.cw]` applies from the next sentence; a reading in progress goes on from the last word with it. While keying, a new speed, keyer, note, channel, or paddle swap applies at once; a new tone or level applies to the sidetone the next time keying is turned on. The note numbers and the channel belong to the adapter on each computer, so they are not synced; the other keying settings are.

### Morse practice

Koch method practice rounds. See [Practice and training](cw.md#practice-and-training).

- `speech.cw.practice_threshold`, default `90`: in Morse code practice, the score of a Koch round, in percent (50 to 100), that adds the next character.
- `speech.cw.practice_group`, default `5`: the characters in each practice group, 2 to 10.
- `speech.cw.practice_groups`, default `10`: the groups in a practice round, 1 to 50.
- `speech.cw.practice_delay_ms`, default `1000`: the quiet before each practice group is sent, 0 to 5000 milliseconds, after the time textweaver's spoken message is expected to take. See [Practice and training](cw.md#practice-and-training).

### Display

The theme and how the text is laid out. Wrap width, tab width, scroll margin, the key hints line, and right-to-left display are the terminal reader's; the line length is the textweaver app's.

- `display.theme`, default `"galaxy"`: the color theme. See [Themes](themes.md).
- `display.follow_os_theme`, default `true`: match the system's light, dark, or high-contrast setting at startup, unless you chose a theme.
- `display.show_line_numbers`, default `false`: show line numbers, in the terminal reader's gutter and in the app's left margin. F6 turns them on and off in the terminal; in the app, View, Line numbers ([Scrolling and line numbers](gui.md#scrolling-and-line-numbers)).
- `display.measure`, default `66`: the line length in the graphical version, in characters, from 25 to 90. The column is this many average characters of the reading font at its size and spacing, so it grows with the text; 0 fills the width of the app's window. The terminal uses `wrap_width` instead.
- `display.wrap_width`, default `0`: wrap lines at this many columns. 0 means the width of the terminal.
- `display.tab_width`, default `4`: columns per tab.
- `display.scroll_margin`, default `3`: lines kept visible above and below the cursor.
- `display.hints`, default `"auto"`: the terminal reader's key hint line at the bottom of the screen. While reading, it begins with the toolbar's buttons from `gui.toolbar_buttons`, each by its short name, then the terminal's own keys. `"auto"` shows it when textweaver speaks for itself and hides it in hybrid and screen-reader modes, where the Braille display reads the screen; `"on"` always shows it; `"off"` never does. A prompt still uses that line. Stays on this computer. See [Braille-first layout](reading.md#braille-first-layout).
- `interface.rtl`, default `"auto"`: whether the terminal reader reorders right-to-left text (Arabic, Hebrew) for display. `"auto"` reorders only where it helps: not in terminals that do it themselves (GNOME Terminal and other VTE terminals, Konsole, mlterm, macOS Terminal), not on Windows (Windows Terminal and the console do not support right-to-left text), and not in hybrid or screen reader mode, since a screen reader reads the terminal's cells and would get reordered text backwards. `"on"` always reorders, `"off"` never. The document, speech, and your screen reader always get the text in reading order.

### Colors

The color of each reading aid and part of the screen, over the theme's own (View, Colors, or File, Settings, Colors; see [The Colors dialog](gui.md#the-colors-dialog)). Each is a color name (`"blue"`, `"orange"`, `"navy"`, `"skyblue"`, `"teal"`, `"gold"`, `"yellow"`, `"purple"`, `"pink"`, `"brown"`, `"gray"`, `"black"`, `"white"`), a `"#rrggbb"` value, or `"theme"` (the default), which keeps the theme's own color. The word and sentence highlights are under [Highlight](#highlight). Every colored mark keeps the cue that is not a color, such as an underline, bold, or a spoken word, whatever color is chosen.

- `colors.ruler`: the reading ruler and the current-line band.
- `colors.difficult_words`: the underline of difficult words.
- `colors.syllables`: the marks between syllables.
- `colors.misspellings`: the underline of misspelled words, in the graphical version.
- `colors.lint`: the underline of Markdown lint and grammar problems, in the graphical version.
- `colors.find_match`: the band behind search matches.
- `colors.selection`: the band behind selected text.
- `colors.focus`: the focus outline and the focused item of a list.
- `colors.links`: link text.
- `colors.headings`: heading text.
- `colors.status_bar`: the status and title bars.
- `colors.notes`: the band behind text with a note.
- `colors.bookmarks`: the band behind a bookmarked word.

### Spacing, font and ruler

Reading aids for the page. See [Reading aids](reading-aids.md). Text spacing is in multiples of the font size; in the terminal, textweaver adds blank lines and spaces instead. The font applies to the graphical version; the terminal always uses its own font. Alt+Shift+U cycles the reading ruler.

- `reading_aids.spacing.line_height`, default `1.5`.
- `reading_aids.spacing.paragraph_spacing`, default `1.0`.
- `reading_aids.spacing.letter_spacing`, default `0.0`.
- `reading_aids.spacing.word_spacing`, default `0.0`.
- `reading_aids.font.family`, default `"sans"`: `"system-ui"`, `"sans"`, `"serif"`, `"monospace"`, a reading font (`"opendyslexic"`, `"atkinson"`, `"lexend"`), or the name of any installed font.
- `reading_aids.font.size_pt`, default `14.0`: the size in points, from 6 to 144 in this file; the Settings dialog and the graphical version's size keys go up to 72.
- `reading_aids.font.weight`, default `400`: from 100 to 900. 700 is bold.
- `reading_aids.ruler.mode`, default `"off"`: `"off"`, `"current_line"`, or `"ruler"` (the line and a band around it).
- `reading_aids.ruler.scope`, default `"line"`: `"line"` marks every row of a wrapped line; `"row"` marks one screen row.
- `reading_aids.ruler.rows_above` and `reading_aids.ruler.rows_below`, default `1`: the size of the band.
- `reading_aids.ruler.mask_outside`, default `false`: dim everything outside the band.

`fetch_missing` was removed in 0.1.0-alpha.5: textweaver downloads a reading font (Lexend) only when you choose it and agree, so the setting did nothing. A `fetch_missing` line in an older `settings.toml` is ignored and dropped at the next save.

### Word aids

Reading aids for words: bionic reading, syllables, and difficult words. See [Reading aids](reading-aids.md).

- `reading_aids.bionic`, default `false`: bionic reading, the start of each word in bold. Alt+Shift+B turns it on and off.
- `reading_aids.bionic_options.ratio`, default `0.4`: the part of each word in bold, from 0.1 to 0.9.
- `reading_aids.bionic_options.min_word_len`, default `2`: shorter words are left alone.
- `reading_aids.bionic_options.skip_numbers`, `reading_aids.bionic_options.skip_urls`, and `reading_aids.bionic_options.skip_code`, each default `true`: leave numbers, web addresses, and code alone.
- `reading_aids.syllables`, default `false`: show long words split into syllables, `read·a·bil·i·ty`. Alt+Shift+Z turns it on and off. Both the terminal reader and the graphical version draw it.
- `reading_aids.syllable_options.separator`, default `"·"`: the character placed between syllables.
- `reading_aids.syllable_options.min_word_len`, default `4`, `reading_aids.syllable_options.left_min`, default `2`, and `reading_aids.syllable_options.right_min`, default `2`: the shortest word to split, and the fewest letters kept before the first break and after the last.
- `reading_aids.syllable_options.skip_urls` and `reading_aids.syllable_options.skip_code`, default `true`.
- `reading_aids.difficult_words`, default `false`: underline rare words (SCOWL sizes above 50), and name them on word moves at high verbosity. Alt+Shift+J turns it on and off. Both the terminal reader and the graphical version draw it.
- `reading_aids.difficult_definitions`, default `false`: with difficult words marked, at high verbosity also say a difficult word's first definition from the dictionary.

### Word by word (RSVP)

RSVP shows one word at a time. See [Reading aids](reading-aids.md).

- `reading_aids.rsvp.wpm`, default `300`: words per minute, from 50 to 1,500 in this file; the Settings dialog starts at 60.
- `reading_aids.rsvp.pacing`, default `"timer"`: `"timer"` uses the words-per-minute rate. `"external"` follows speech: it shows the word being spoken.
- `reading_aids.rsvp.clause_pause`, `reading_aids.rsvp.sentence_pause`, and `reading_aids.rsvp.paragraph_pause`, defaults `50`, `100`, and `150`: extra time, in percent of a word's time, after a comma, a sentence, and a paragraph.
- `reading_aids.rsvp.long_word_len`, default `8`, `reading_aids.rsvp.long_word_step`, default `10`, and `reading_aids.rsvp.long_word_max`, default `80`: a word longer than `long_word_len` letters gets `long_word_step` percent more time per extra letter, up to `long_word_max` percent.
- `reading_aids.rsvp.show_previous` and `reading_aids.rsvp.show_next`, default `true`: show the word before and the word after.
- `reading_aids.rsvp.position`, default `"top-center"`: where the word box sits. `"top-left"`, `"top-center"`, `"top-right"`, `"center-left"`, `"center"`, `"center-right"`, `"bottom-left"`, `"bottom-center"`, or `"bottom-right"`.
- `reading_aids.rsvp.font_size_pt`, default `48`: the word's size in the graphical version. The terminal uses its own font.
- `reading_aids.rsvp.lead_words`, default `0`: when following speech, show a word this many words ahead.

### Editing

Edit mode, the name written on what you add, and the preview of the document you are editing. See [Writing and editing](editing.md) and [Export and preview](editing.md#export-and-preview).

- `editing.echo_characters`, default `true`: speak each character you type.
- `editing.echo_words`, default `true`: speak each word when you finish it.
- `editing.echo_deletions`, default `true`: speak what you delete.
- `editing.echo_lines_on_move`, default `true`: speak the line when the cursor moves to another line.
- `editing.undo_steps`, default `1000`: the most undo steps kept while editing. The oldest are forgotten first. The smallest allowed value is 1.
- `editing.undo_memory_mb`, default `50`: the most memory, in megabytes, the undo steps may use. The oldest are forgotten first; the newest step is always kept. The smallest allowed value is 1.
- `editing.autosave_recovery`, default `true`: keep recovery snapshots while you edit, and offer them after a crash.
- `editing.autosave_interval_secs`, default `20`: seconds between snapshots while there are unsaved changes.
- `authoring.author`, default `""`: the name on comments, replies, and changes saved to a Word file (see [Tracked changes and comments](reading.md#tracked-changes-and-comments-ctrlshiftj-or-alta)), and the author of a new document from a template (`author = "Jo Writer"`; see [Start from a template](editing.md#start-from-a-template)). Empty signs comments "textweaver" and leaves a template's author blank. textweaver never takes a name from your computer or your account, and the name never syncs. A settings file with the older `[editing] author` keeps its value: it is read as `[authoring] author`.
- `authoring.track_changes`, default `false`: save edit mode's changes to a Word file (`.docx`) into the file itself as tracked changes, signed with `author` and the time, for a reviewer to accept or reject in Word. Off, a Word file's edits are saved as Markdown under a new name. See [Your own edits as tracked changes](editing.md#your-own-edits-as-tracked-changes).
- `preview.pane`, default `false`: in the app, show the document as it reads beside the editor. Alt+F5 turns it on and off; F6 moves to it.
- `preview.follow`, default `"off"`: when the browser preview reloads itself, through a small server on this computer only (127.0.0.1, with a secret in the address), landing on the heading nearest the cursor. `"off"` never: textweaver says "Preview updated. Press F5 in the browser." `"save"` reloads after each save, and `"typing"` also when typing pauses. A reload moves your screen reader's place in the page, which is why it is off. In the app and the palette it is called "Browser preview follows" (`browser preview follows` moves to the next value). The older `auto_reload` and `live` are still read as `"save"` and `"typing"`.
- `preview.pane_delay_ms`, default `300`: how long typing must pause before a preview that follows your typing is rewritten, from 100 to 3000 in steps of 50.

Your spelling word list is not a setting: it is `words.txt` in the data folder, one word per line. See [Spelling](editing.md#spelling).

### Library and study tools

The library, reading statistics, summaries, and Define word. See [The library](library.md), [Reading statistics](reading.md#reading-statistics-ctrlshifty-or-alty), [Summaries](reading.md#summaries), and [Define a word](reading.md#define-a-word-ctrlshiftd-or-alte).

- `library.folders`, default empty: your library folders.
- `library.recent_limit`, default `20`: how many recent files are remembered.
- `stats.enabled`, default `true`: count the time textweaver reads each document aloud, the furthest point, and the sessions, in `stats.json` in the data folder. Off, nothing more is recorded; what was recorded stays until `tw stats clear`.
- `summary.sentences`, default `5`: how many sentences Summarize and `tw summarize` give, from 1 to 50.
- `lexicon.glossary`, no default: your own glossary, looked up before the dictionary. Unset, textweaver uses `glossary.txt`, else `glossary.json`, in the settings folder when there is one. A text glossary has one `term: definition` line per sense (`term = definition` and `term - definition` work too); a JSON glossary is star's custom dictionary format.
- `lexicon.data_file`, no default: the dictionary file, `lexicon-en.twlex`. Unset, textweaver looks in the `lexicon` folder beside the program, then in its data folder.

### Dictation

Dictating text in edit mode, in the terminal reader and the graphical version. See [Dictation](dictation.md).

- `dictation.model`, default `"whisper-base.en"`: the Whisper model dictation uses, and the one Download the dictation model and the dictation question offer: `"whisper-base.en"` (79.3 MB) or `"whisper-small.en"` (251 MB, slower and more accurate). It is kept in `whisper/rten/base.en` or `whisper/rten/small.en` in the data folder.
- `dictation.model_dir`, not set by default: a folder holding an in-process Whisper model (its encoder, decoder, and `tokenizer.json`), used as it is. Unset, textweaver uses the model `model` chooses, in its data folder.
- `dictation.speak_while_recording`, default `false`: speak the dictated words with textweaver's own voice while the microphone is open. Off, they are shown on the status line as they come and spoken once, at each pause, so the microphone does not hear textweaver's voice.

### Keyboard and language

The language of textweaver's own words, and the keys. See [the keyboard reference](keyboard.md).

- `interface.language`, default `"en"`: the language of textweaver's own words: messages, lists, help, and the settings screen, not your documents. Built in: `"en"` English, `"es"` Spanish, `"fr"` French, `"de"` German, `"pt"` Portuguese (Brazilian), and `"ar"` Arabic. A tag with a region, such as `"es-MX"` or `"pt-BR"`, uses its language. A change on the settings screen takes effect at once: the change is said in the new language, then the title line. The voice follows the language when the speech engine has a voice for it; when it has none, the current voice keeps speaking and textweaver says so. `en-XA` shows every message accented and in `⟦ ⟧` brackets, and `ar-XB` shows them right to left: both are for testing. A `<language>.ftl` file in the `locales` folder of the settings folder adds a language, or goes over a built-in one message by message; messages it lacks come from the built-in translation, then English. `tw settings language` lists the languages, and `tw settings language es` sets one. The first run starts with the list of languages, your system's language first. Native speakers have checked English and Spanish; German, French, Portuguese, and Arabic have not been checked yet, and feedback on any translation is welcome (see [Known limits](known-limits.md)).
- `keyboard.preset`, default `"default"`: the set of keys to start from. `"default"` is the quick navigation of NVDA's and JAWS's browse mode (`h`, `1` to `6`, `l`, `i`, `t`, `k`, `q`, `s`, `g`, `d`); `"classic"` is textweaver's earlier keys. `"screen-reader"`, the old name of the default, still works. `keymap.toml` applies on top. See [the keyboard reference](keyboard.md#what-changed).
- `keyboard.character_keys`, default `true`: single-key shortcuts, such as `h` for the next heading. Set it to `false`, or press F9, so dictation or typing never triggers a command. See [the keyboard reference](keyboard.md#single-key-shortcuts).
- `keyboard.digit_row`, default `"auto"`: how the terminal knows the digit keys `1` to `6` (heading levels) when it gets only the typed character. `"auto"` knows the shifted digits of the US, UK, German, Spanish, Nordic, and Italian layouts; `"azerty"` is for French keyboards, where the digits need Shift. On Windows textweaver reads the digit key itself, and this setting does not matter.

### Toolbars and panels

The textweaver app's bars, buttons, and panel. The terminal reader uses only the toolbar's buttons, for its key hints line.

- `gui.header` and `gui.toolbar`, default `true`: show the header bar (Open, Font, Settings, Commands) and the reading toolbar (with Edit). Off, their commands keep their keys and menu items. View, Header and Toolbar switch them.
- `gui.header_buttons`, default `["open", "choose_font", "settings", "command_palette"]`: the header's buttons, in order, as command ids. Any command with a short name can be a button; an id this version does not recognize is kept but not shown, and a command already on a bar is not shown a second time. View, Customize buttons changes the list one button at a time and says each change. It stays on this computer. See [Customizing the buttons](gui.md#customizing-the-buttons).
- `gui.toolbar_buttons`, default `["play_pause", "stop", "previous_paragraph", "next_paragraph", "toggle_edit_mode", "rate_down", "rate_up"]`: the toolbar's buttons, in order, under the same rules. The defaults step by paragraph rather than by sentence, and include Edit (Done editing in edit mode). The terminal reader's key hint line follows this list too, so both programs offer the same commands. It stays on this computer.
- `gui.auto_hide_menu`, default `true`: on Windows, hide the menu bar until Alt, F10, or Alt with a menu's letter shows it; it hides again when the menu closes, and nothing is announced then. It applies at once and stays on this computer. It has no effect on Linux (the menus are the F10 list) or macOS. See [Menus](gui.md#menus).
- `gui.focus_mode`, default `false`: hide the header, the toolbar, the status bar and the panel once the mouse has rested for three seconds, so the text has the whole window; moving the mouse shows them, and keys never do. Visual only: the controls stay in the accessibility tree and the Tab order, and nothing is announced. Full screen (Alt+Enter) does the same while it lasts. It stays on this computer. See [Focus mode and full screen](gui.md#focus-mode-and-full-screen).
- `gui.sidebar`, default `"off"`: the panel beside the document: `"off"`, `"contents"` (the headings), or `"notes"`. Ctrl+1 and Ctrl+2 change it, and the app remembers the last one.

### Export

Audio export and subtitles. See [Audio export](audio-export.md).

- `export.audio_format`, default `"flac"`: the format Export audio lists first, and the one `tw export-audio` writes when the `--out` name has no extension. `"flac"`, `"mp3"`, `"opus"`, `"ogg"`, `"wav"`, `"m4b"` (an audiobook), or `"mp4"` (a video with captions). M4B and MP4 need ffmpeg; without it they are not listed, and the list keeps its usual order.
- `export.subtitle_format`, default `"srt"`: `"srt"` or `"vtt"`, used when subtitles are written without a file name.
- `export.subtitle_karaoke`, default `"off"`: how subtitle lines show the word being read. `"off"`, `"tags"` (underlined as it is spoken, in WebVTT) or `"lines"` (one cue per word, in bold and underline).
- `export.subtitle_chapters`, default `false`: also write a WebVTT chapters file beside the subtitles or the audio.
- `export.subtitle_word_level`, default `false`: one subtitle cue per word instead of caption lines.
- `export.subtitles_with_audio`, default `false`: always write subtitles beside the exported audio.

### Sync

Syncing with your other computers through a folder you choose. Tools, Sync, Set up sync writes these for you; see [Syncing between computers](sync.md). Every switch is on by default, and turning one off stops that group only: this computer neither sends nor takes its items.

- `sync.enabled`, default `false`: sync on this computer.
- `sync.folder`, not set by default: the sync folder, such as one kept in step by Syncthing or on a USB stick.
- `sync.device_name`, default `""`: this computer's name in sync messages, such as `"laptop"` or `"lab"`. Empty uses "Computer 1", "Computer 2", and so on, never the computer's own name.
- `sync.position_policy`, default `"newest"`: which place a document opens at when another computer has one too: `"newest"`, `"furthest"`, or `"ask"`. It replaces `[reading] sync_conflict_policy` (`"highest_progress"` reads as `"furthest"`, `"manual"` as `"ask"`), and also decides between places in a library folder's old progress file.
- `sync.places`, `sync.notes`, `sync.highlights`, `sync.bookmarks`, `sync.statistics`: the document groups.
- `sync.settings`: the portable settings (below).
- `sync.profiles`: your profiles' definitions. Which profile is in use stays on each computer.
- `sync.key_overrides`: your `keymap.toml`. Each override is labeled with the system it was made on. Windows and Linux share their keys; a Mac's overrides are kept but not used on Windows or Linux, and the reverse. Sync status says how many are kept but not used.
- `sync.words`: your spelling word list, `words.txt`.
- `sync.glossary`: your glossary's entries and your pronunciations (`[normalization] pronunciations`).
- `sync.favorite_voices`: your favorite voices. One that is not installed on this computer is kept, and Choose voice lists it as "not on this computer".

### Updates and components

Update checks and optional components (the models, fonts, and voices textweaver can download). See [Updates](updates.md) and [Optional components](components.md). These stay on this computer.

- `updates.check`, default `false`: look for a newer textweaver once a day at start. It reads the public list of releases on GitHub and asks before downloading. Nothing about you is sent. Help, Check for updates checks at any time.
- `components.source`, default `""`: your own components, used first. A GitHub repository written `owner/name`, or a folder on this computer. Never put a password here: a private repository signs in with the GitHub CLI, or asks once for a token kept in the system credential store. See [Your own components source](components.md#your-own-components-source).
- `components.mirror`, default `""`: where components come from first, before their public sources: an `https` address or a folder on this computer, holding each component's files under its id. Empty uses the public sources. The `TEXTWEAVER_COMPONENTS_MIRROR` environment variable wins over it. Never put a password or token here.

### Kept by textweaver

textweaver writes these itself. They are in the file, but not on the settings screens.

- `display.theme_explicit`, default `false`: set by textweaver when you choose a theme; it stops following the system.
- `accessibility.hybrid_offered`, default `false`: set after textweaver has asked, on its first run with a screen reader, whether to use hybrid mode. Set it back to `false` to be asked again.
- `interface.recent_settings`, default an empty list: the settings changed last on the settings screen, listed at its top. It stays on this computer.
- `gui.window` and `gui.last_version`: where the window was and how large, and the version that last ran here, so "Updated to" is said once after an update. textweaver keeps both itself; they never sync.
- `updates.asked`, `updates.last_check`, and `updates.declined`: markers textweaver keeps itself. `asked` is set once the first-run question has been asked, `last_check` is when the daily check last ran (seconds since 1970), and `declined` is the release you said no to; only a newer one is offered.
- `components.chooser_shown`, default `false`: set once the first-run list of optional components has been shown, so it is shown once.


Every setting is either **portable**, which syncs, or **machine**, which never does. Portable settings are about you as a reader: the rate, punctuation, verbosity, capitals, the reading aids, the highlight, the theme and colors, the Braille and math codes, the interface language, speed presets, and the announcement level. Machine settings belong to one computer: the speech engine and voice, the volume, the sound device, the access mode and what goes with your screen reader, the NVDA or JAWS key preset, the keyboard layout, the wrap width, undo memory, every path (library folders, the glossary file, engine libraries, the sync folder), the author name written on comments and new documents, and the sync settings themselves. The [settings reference](settings-reference.md) says for each setting whether it syncs.

## Settings profiles

A profile is a named set of the settings that are usually changed together: the speech engine, voice, rate, pitch, and volume; the theme; the font and text spacing; bionic reading and the ruler; the highlight; the access mode; and the audio export format. One profile might serve for study and another for skimming, or each person who shares a computer might have their own.

- In the reader, **Ctrl+Shift+U** in the graphical version or **Alt+U** in the terminal lists your profiles. Enter switches to one; its settings take effect at once and are saved. F2 renames a profile, and Delete deletes one after a yes or no. The last items save the current settings as a new profile, save them into the profile in use, and import and export profiles.
- From the command line:

  ```sh
  tw settings profile list
  tw settings profile save "Study"
  tw settings profile switch "Study"
  tw settings profile rename "Study" "Exam"
  tw settings profile export profiles.json
  tw settings profile import profiles.json
  tw settings profile delete "Exam"
  ```

Profiles are stored in `profiles.toml`, beside `settings.toml`, so a settings export does not include them. A profile export works in any version of textweaver: settings that version does not recognize are left out, and it says which. With sync on, profiles travel to the other computers (`[sync] profiles`), while the profile in use stays specific to each.

### Starter recipes

These are four starting points with exact values. None is a treatment for any condition, and no single configuration suits every reader; apply a recipe, then adjust whatever does not help. To use one, first save the current settings as a profile (`tw settings profile save "My settings"`). Put the recipe in a file such as `larger-print.toml` and check it with `tw settings import larger-print.toml --dry-run`, which lists each change and refuses any value out of range. Then import it without `--dry-run` and save the result as a profile of its own.

**Larger print.** Bigger letters and more room between lines and words:

```toml
[reading_aids.font]
size_pt = 20

[reading_aids.spacing]
line_height = 2.0
word_spacing = 0.3
```

**Slow and clear.** A slower voice, the whole sentence highlighted, and the Generous spacing values, with the ruler on the current line:

```toml
[speech]
rate = 150

[highlight]
granularity = "sentence"

[reading_aids.font]
size_pt = 18

[reading_aids.spacing]
line_height = 2.0
paragraph_spacing = 2.5
letter_spacing = 0.15
word_spacing = 0.3

[reading_aids.ruler]
mode = "ruler"
```

**Focus.** The ruler with everything outside its band dimmed, at a moderate rate:

```toml
[speech]
rate = 200

[reading_aids.ruler]
mode = "ruler"
mask_outside = true
```

**Short lines.** Fewer words to a line: about 45 characters in the graphical version, and 60 columns in the terminal reader. Some readers read better at three or four words a line and some do not, so try a few lengths between 25 and 66.

```toml
[display]
measure = 45
wrap_width = 60
```

### Developer profile (example)

An example profile, `docs/examples/developer-profile.toml`, ships with the source and every package, beside the guides. It is used only if imported and switched to, and it sets:

- the Eloquence engine (`speech.backend = "eci"`) with the Reed voice (`speech.voice = "Reed"`);
- 400 words per minute (`speech.rate = 400`);
- the Lexend font at 16 points (`reading_aids.font`, `family = "lexend"`, `size_pt = 16.0`); Lexend is downloaded the first time, after asking;
- the Galaxy theme (`display.theme = "galaxy"`);
- Opus for audio export (`export.audio_format = "opus"`).

To try it:

```sh
tw settings profile save "My settings"
tw settings profile import docs/examples/developer-profile.toml
tw settings profile switch "Developer"
```

Saving the current settings as a profile first means `tw settings profile switch "My settings"` restores them. Importing only adds the profile; nothing changes until a switch. In the reader, the profile list (Ctrl+Shift+U in the graphical version, Alt+U in the terminal) imports and switches in the same way.

Eloquence must be installed for the Reed voice; see [the Eloquence guide](eloquence.md). Without it, textweaver says so and uses another engine.

## See also

- [Speech engines and voices](speech.md): the speech settings in use.
- [Reading and moving around](reading.md): the reading and highlight settings.
- [Keyboard reference](keyboard.md): changing keys in `keymap.toml`.
- [Themes](themes.md): the `theme` setting and your own themes.
- [Reading aids](reading-aids.md): the `[reading_aids]` settings.
- [Troubleshooting](troubleshooting.md): when settings do not load.
- [Documentation index](README.md)
