# Settings reference

Every setting in `settings.toml`, section by section, in the order the settings screen lists them. Each item gives the key, its default, its label on the settings screen, what it does, and the values it takes. [Settings](settings.md) explains where the file lives, and how to export, import, and reset it.

This page is generated from the settings schema by `cargo xtask settings-doc`. Do not edit it by hand; CI checks that it is current.

## Speech: the `[speech]` section

- `speech.backend`: default automatic (`"auto"`). Speech engine. The speech engine: auto picks the best one available. A change restarts speech. Choices: `"auto"` (automatic), `"eci"` (Eloquence), `"sapi"` (SAPI 5 voices), `"espeak"` (eSpeak NG), `"speechd"` (Speech Dispatcher), `"nsspeech"` (Apple NSSpeech), `"avspeech"` (Apple AVSpeech), `"dectalk"` (DECtalk), `"omnivox"` (Omnivox), `"null"` (silent). Other values may be written too.
- `speech.rate`: default 265 words per minute. Rate. How fast textweaver speaks. From 50 to 900 words per minute, in steps of 20.
- `speech.volume`: default 100 percent. Volume. How loud textweaver speaks. From 0 to 100 percent, in steps of 10.
- `speech.pitch`: default 0 semitones. Pitch. Higher or lower than the voice's own pitch. From -12 to 12 semitones, in steps of 1.
- `speech.voice`: default not set. Voice. The voice's id; not set picks one automatically. Choose Voice lists them. Text; empty means not set.
- `speech.prefer_voice`: default `"eloquence"`. Preferred voice. When no voice is set, the first voice whose name contains this, such as eloquence. Text; empty means not set.
- `speech.favorite_voices`: default an empty list. Favourite voices. Voices listed first in Choose Voice, by id. A list of texts, such as `["a", "b"]`.
- `speech.punctuation`: default `"some"`. Punctuation. How much punctuation is spoken. Choices: `"none"`, `"some"`, `"all"`.
- `speech.split_caps`: default off (`false`). Split capitals. Say words joined with capitals, such as TextWeaver, as separate words. On or off: `true` or `false`.
- `speech.caps`: default a higher pitch (`"pitch"`). Capitals. How a capital letter is marked when characters are spoken and typed. Choices: `"none"` (not marked), `"tone"` (a tone), `"pitch"` (a higher pitch), `"say_cap"` (say cap).
- `speech.auto_play`: default off (`false`). Read on opening. Start reading when a document opens. On or off: `true` or `false`.
- `speech.skip_code`: default on (`true`). Skip code blocks. Do not speak code blocks. On or off: `true` or `false`.
- `speech.speed_presets`: default 4 entries. Speed presets. Named rates that F8 cycles through. A table of names and values, edited in the file.
- `speech.latency_offset_ms`: default 120 milliseconds. Highlight delay. How long after an engine reports a word the highlight moves, for engines timed by their audio clock. From 0 to 1000 milliseconds, in steps of 10.
- `speech.verbosity`: default `"normal"`. Verbosity. How much textweaver says about what it does. Choices: `"low"`, `"normal"`, `"high"`.
- `speech.eci.dictionaries`: default on (`true`). Eloquence dictionaries. The community pronunciation dictionaries for Eloquence: on, off, or a folder of your own. Choices: `true` (on), `false` (off). Other values may be written too.
- `speech.eci.library`: default not set. Eloquence library. The ECI library to load; not set searches the usual places. Text; empty means not set.
- `speech.eci.code_factory`: default off (`false`). Search Code Factory's Eloquence. Also look for Code Factory's Eloquence for Windows. Its licence may not cover other programs. On or off: `true` or `false`.
- `speech.sapi.onecore`: default on (`true`). OneCore voices. Also list the Windows OneCore voices through SAPI 5. On or off: `true` or `false`.
- `speech.apple.backend`: default automatic (`"auto"`). Apple speech engine. Which of Apple's speech engines to use on macOS. Choices: `"auto"` (automatic), `"nsspeech"` (NSSpeechSynthesizer), `"avspeech"` (AVSpeechSynthesizer).

## Highlight: the `[highlight]` section

- `highlight.enabled`: default on (`true`). Highlight spoken text. Highlight the word or sentence being read. On or off: `true` or `false`.
- `highlight.granularity`: default the word (`"word"`). Highlight. What the reading highlight covers. Choices: `"word"` (the word), `"sentence"` (the sentence), `"both"` (the word and the sentence).
- `highlight.lead_words`: default 1 words. Highlight lead. Draw the highlight this many words ahead of the word heard (1 is the word heard). From -5 to 5 words, in steps of 1.
- `highlight.speed`: default 1 times. Highlight speed. Speed of the timed highlight for engines that report no words. From 0.5 to 1.5 times, in steps of 0.1.
- `highlight.color`: default `"theme"`. Word highlight colour. A colour name or #rrggbb over the theme's word highlight; theme keeps the theme's. Text.
- `highlight.sentence_color`: default not set. Sentence highlight colour. A colour name or #rrggbb over the theme's sentence highlight; not set keeps the theme's. Text; empty means not set.

## Speaking text: the `[normalization]` section

- `normalization.math`: default on (`true`). Speak math. Speak math notation in words. On or off: `true` or `false`.
- `normalization.math_verbosity`: default `"normal"`. Math verbosity. How explicit spoken math is: low says a over b, normal and high say more. Choices: `"low"`, `"normal"`, `"high"`.
- `normalization.asciimath_delimiter`: default not set. ASCIIMath delimiter. The character around ASCIIMath, usually a backtick; not set reads no ASCIIMath. Text; empty means not set.
- `normalization.abbreviations`: default on (`true`). Expand abbreviations. Say abbreviations in full, such as Doctor for Dr. On or off: `true` or `false`.
- `normalization.abbrev_expansions`: default none. Your abbreviations. Abbreviations of your own and what they stand for. A table of names and values, edited in the file.
- `normalization.numbers`: default on (`true`). Numbers in words. Say numbers, dates, times, and money in words. On or off: `true` or `false`.
- `normalization.use_pronunciations`: default on (`true`). Use pronunciations. Apply your pronunciation list. On or off: `true` or `false`.
- `normalization.pronunciations`: default none. Pronunciations. Words and how to say them. A table of names and values, edited in the file.
- `normalization.table_mode`: default with rows and columns (`"structured"`). Tables. How tables are read. Choices: `"structured"` (with rows and columns), `"flat"` (as text), `"skip"` (skipped).
- `normalization.footnote_mode`: default where they are marked (`"inline"`). Footnotes. Where footnotes are read. Choices: `"inline"` (where they are marked), `"deferred"` (at the end), `"skip"` (skipped).
- `normalization.community_lexicon.enabled`: default off (`false`). Community lexicon. Apply the community pronunciation dictionaries for engines other than Eloquence. On or off: `true` or `false`.
- `normalization.community_lexicon.dir`: default not set. Community lexicon folder. The folder holding the dictionary files; not set looks beside textweaver. Text; empty means not set.
- `normalization.community_lexicon.language`: default US English (`"ENU"`). Community lexicon language. The dictionaries' language. Choices: `"ENU"` (US English), `"DEU"` (German). Other values may be written too.

## Reading: the `[reading]` section

- `reading.auto_resume`: default on (`true`). Resume where you left off. Go back to the saved position when a document opens. On or off: `true` or `false`.
- `reading.nav_history_size`: default 50 places. Back history. How many places Back remembers. From 1 to 1000 places, in steps of 10.
- `reading.wrap_navigation`: default off (`false`). Wrap navigation. Moving past the end of the document goes on from the start. On or off: `true` or `false`.
- `reading.cursor_follows_speech`: default on (`true`). Cursor follows speech. The cursor moves with the word being read. On or off: `true` or `false`.
- `reading.sync_conflict_policy`: default the newest (`"newest"`). Synced positions. Which position wins when another device read further or later. Choices: `"newest"` (the newest), `"highest_progress"` (the furthest), `"manual"` (ask).
- `reading.citations`: default skipped (`"off"`). Citations. Citations in continuous reading: skipped, or said in words. Choices: `"off"` (skipped), `"words"` (in words).
- `reading.ocr`: default on (`true`). Recognize scanned pages. Read the text of scanned PDFs and pictures by recognizing it (OCR). On or off: `true` or `false`.
- `reading.ocr_lang`: default the document's (`""`). Scanned text language. The language of scanned text, as Tesseract codes such as fra or deu+eng; empty means the document's own language, else English. Choices: `""` (the document's), `"eng"` (English), `"fra"` (French), `"deu"` (German), `"spa"` (Spanish). Other values may be written too.
- `reading.ocr_engine`: default automatic (`"auto"`). OCR engine. Which engine recognizes scanned pages: ocrs for English and Tesseract for other languages, or one of them always. Choices: `"auto"` (automatic), `"ocrs"`, `"tesseract"` (Tesseract), `"paddle"` (PaddleOCR (experimental)).

## Display: the `[display]` section

- `display.theme`: default `"galaxy"`. Theme. The colour theme. Choices: . Other values may be written too.
- `display.follow_os_theme`: default on (`true`). Follow the system theme. At startup, use a light, dark, or high-contrast theme like the system, unless you picked one. On or off: `true` or `false`.
- `display.wrap_width`: default 0 columns. Wrap width. Wrap lines at this many columns; 0 uses the whole width. From 0 to 400 columns, in steps of 10.
- `display.tab_width`: default 4 columns. Tab width. Columns a tab takes. From 1 to 16 columns, in steps of 1.
- `display.show_line_numbers`: default off (`false`). Line numbers. Show line numbers. On or off: `true` or `false`.
- `display.scroll_margin`: default 3 lines. Scroll margin. Lines kept in view above and below the cursor. From 0 to 20 lines, in steps of 1.

## Editing: the `[editing]` section

- `editing.autosave_recovery`: default on (`true`). Recovery snapshots. Keep a copy of unsaved work and offer it after a crash. On or off: `true` or `false`.
- `editing.autosave_interval_secs`: default 20 seconds. Snapshot interval. Seconds between recovery snapshots while there are unsaved changes. From 5 to 3600 seconds, in steps of 5.
- `editing.echo_characters`: default on (`true`). Echo characters. Say each character typed. On or off: `true` or `false`.
- `editing.echo_words`: default on (`true`). Echo words. Say each word typed. On or off: `true` or `false`.
- `editing.echo_deletions`: default on (`true`). Echo deletions. Say what Backspace and Delete remove. On or off: `true` or `false`.
- `editing.echo_lines_on_move`: default on (`true`). Echo lines. Say the line when the caret moves to another line. On or off: `true` or `false`.
- `editing.undo_steps`: default 1000 steps. Undo steps. Most undo steps kept while editing. From 1 to 100000 steps, in steps of 100.
- `editing.undo_memory_mb`: default 50 megabytes. Undo memory. Most memory the undo history may use. From 1 to 4096 megabytes, in steps of 16.

## Library: the `[library]` section

- `library.recent_limit`: default 20 files. Recent files. How many recent files are remembered. From 1 to 500 files, in steps of 5.
- `library.folders`: default an empty list. Library folders. Folders whose documents the library lists, and whose positions sync between computers. A list of texts, such as `["a", "b"]`.

## Keyboard: the `[keyboard]` section

- `keyboard.character_keys`: default on (`true`). Single-key shortcuts. Browse keys such as h and period. Off, dictation and typing never trigger commands. On or off: `true` or `false`.
- `keyboard.preset`: default screen reader style (`"default"`). Keys. The default keys: like NVDA's and JAWS's browse mode, or textweaver's earlier keys. Used from the next start. Choices: `"default"` (screen reader style), `"classic"`.
- `keyboard.digit_row`: default automatic (`"auto"`). Digit row. How the terminal recognises the digit keys for heading levels: auto, or a French AZERTY keyboard. Choices: `"auto"` (automatic), `"azerty"` (AZERTY).

## Accessibility: the `[accessibility]` section

- `accessibility.mode`: default `"self-voicing"`. Accessibility mode. Self-voicing speaks everything; screen reader leaves speech to your screen reader; hybrid voices reading only. Choices: `"self-voicing"`, `"screen-reader"` (screen reader), `"hybrid"`.
- `accessibility.say_all`: default on the status line (`"screen"`). Say all with a screen reader. Continuous reading in screen-reader mode: a sentence at a time on the status line, or textweaver's voice. Choices: `"screen"` (on the status line), `"voice"` (with textweaver's voice).
- `accessibility.quiet_screen`: default off (`false`). Quiet screen while reading. Keep the screen still while textweaver reads aloud. On or off: `true` or `false`.
- `accessibility.cursor`: default follows focus (`"follow"`). Cursor. Where the terminal's cursor waits: on what you are working on, or on the status line. Choices: `"follow"` (follows focus), `"status"` (on the status line).

## Export: the `[export]` section

- `export.subtitle_format`: default SubRip (`"srt"`). Subtitle format. The format of subtitles written without a file name. Choices: `"srt"` (SubRip), `"vtt"` (WebVTT).
- `export.subtitle_word_level`: default off (`false`). Word subtitles. One subtitle per word instead of caption lines. On or off: `true` or `false`.
- `export.subtitles_with_audio`: default off (`false`). Subtitles with audio. Always write subtitles beside exported audio. On or off: `true` or `false`.

## Reading aids: the `[reading_aids]` section

- `reading_aids.rsvp.wpm`: default 300 words per minute. RSVP rate. Words per minute of rapid serial visual presentation. From 60 to 1500 words per minute, in steps of 20.
- `reading_aids.rsvp.pacing`: default its own timer (`"timer"`). RSVP pacing. What moves the RSVP word on: its own timer, or speech. Choices: `"timer"` (its own timer), `"external"` (speech).
- `reading_aids.rsvp.clause_pause`: default 50 percent. RSVP clause pause. Extra time after a comma, colon, dash, or bracket, in percent of a word's time. From 0 to 500 percent, in steps of 10.
- `reading_aids.rsvp.sentence_pause`: default 100 percent. RSVP sentence pause. Extra time at the end of a sentence, in percent. From 0 to 500 percent, in steps of 10.
- `reading_aids.rsvp.paragraph_pause`: default 150 percent. RSVP paragraph pause. Extra time at the end of a paragraph, in percent. From 0 to 500 percent, in steps of 10.
- `reading_aids.rsvp.long_word_len`: default 8 letters. RSVP long word. Words longer than this many letters get extra time. From 1 to 40 letters, in steps of 1.
- `reading_aids.rsvp.long_word_step`: default 10 percent. RSVP long word step. Extra time per letter beyond a long word's length, in percent. From 0 to 100 percent, in steps of 5.
- `reading_aids.rsvp.long_word_max`: default 80 percent. RSVP long word most. Most extra time a long word gets, in percent. From 0 to 500 percent, in steps of 10.
- `reading_aids.rsvp.show_previous`: default on (`true`). RSVP previous word. Show the previous word too. On or off: `true` or `false`.
- `reading_aids.rsvp.show_next`: default on (`true`). RSVP next word. Show the next word too. On or off: `true` or `false`.
- `reading_aids.rsvp.position`: default top centre (`"top-center"`). RSVP position. Where the RSVP word appears. Choices: `"top-left"` (top left), `"top-center"` (top centre), `"top-right"` (top right), `"center-left"` (middle left), `"center"` (middle), `"center-right"` (middle right), `"bottom-left"` (bottom left), `"bottom-center"` (bottom centre), `"bottom-right"` (bottom right).
- `reading_aids.rsvp.font_size_pt`: default 48 points. RSVP size. Size of the RSVP word in the GUI. From 8 to 200 points, in steps of 2.
- `reading_aids.rsvp.lead_words`: default 0 words. RSVP lead. With speech pacing, show this many words ahead of the word spoken. From -5 to 5 words, in steps of 1.
- `reading_aids.bionic`: default off (`false`). Bionic reading. Draw the start of each word in bold. On or off: `true` or `false`.
- `reading_aids.bionic_options.ratio`: default 0.4. Bionic share. How much of each word is bold. From 0.1 to 0.9, in steps of 0.1.
- `reading_aids.bionic_options.min_word_len`: default 2 letters. Bionic shortest word. Words shorter than this are left alone. From 1 to 20 letters, in steps of 1.
- `reading_aids.bionic_options.skip_numbers`: default on (`true`). Bionic skips numbers. Leave words with digits alone. On or off: `true` or `false`.
- `reading_aids.bionic_options.skip_urls`: default on (`true`). Bionic skips addresses. Leave web and e-mail addresses alone. On or off: `true` or `false`.
- `reading_aids.bionic_options.skip_code`: default on (`true`). Bionic skips code. Leave code alone. On or off: `true` or `false`.
- `reading_aids.spacing.line_height`: default 1.5 times. Line height. Line height in multiples of the font size; WCAG's value is 1.5. From 1 to 3 times, in steps of 0.1.
- `reading_aids.spacing.paragraph_spacing`: default 1 times. Paragraph spacing. Space after each paragraph, in multiples of the font size. From 0 to 4 times, in steps of 0.25.
- `reading_aids.spacing.letter_spacing`: default 0 times. Letter spacing. Extra space between letters, in multiples of the font size. From 0 to 0.5 times, in steps of 0.02.
- `reading_aids.spacing.word_spacing`: default 0 times. Word spacing. Extra space between words, in multiples of the font size. From 0 to 1 times, in steps of 0.04.
- `reading_aids.font.family`: default sans serif (`"sans"`). Font. The GUI's reading font; any installed family may be typed. Choices: `"system-ui"` (the system font), `"sans"` (sans serif), `"serif"`, `"monospace"`, `"atkinson"` (Atkinson Hyperlegible), `"opendyslexic"` (OpenDyslexic), `"lexend"` (Lexend). Other values may be written too.
- `reading_aids.font.size_pt`: default 14 points. Font size. The GUI's font size. From 6 to 72 points, in steps of 1.
- `reading_aids.font.weight`: default 400. Font weight. 400 is regular, 700 bold. From 100 to 900, in steps of 100.
- `reading_aids.font.fetch_missing`: default on (`true`). Offer missing fonts. Offer to download a reading font that is not installed, after asking. On or off: `true` or `false`.
- `reading_aids.ruler.mode`: default `"off"`. Reading ruler. Mark the current line, or a band of lines. Choices: `"off"`, `"current_line"` (current line), `"ruler"`.
- `reading_aids.ruler.scope`: default the whole line (`"line"`). Ruler covers. A wrapped row, or the whole line. Choices: `"row"` (a row), `"line"` (the whole line).
- `reading_aids.ruler.rows_above`: default 1 rows. Ruler rows above. Rows of the band above the current one. From 0 to 10 rows, in steps of 1.
- `reading_aids.ruler.rows_below`: default 1 rows. Ruler rows below. Rows of the band below the current one. From 0 to 10 rows, in steps of 1.
- `reading_aids.ruler.mask_outside`: default off (`false`). Ruler mask. Dim the rows outside the band. On or off: `true` or `false`.
- `reading_aids.syllables`: default off (`false`). Syllables. Draw words split into syllables with a middle dot; speech is unchanged. On or off: `true` or `false`.
- `reading_aids.difficult_words`: default off (`false`). Difficult words. Underline rare words, and name them on word moves at high verbosity. On or off: `true` or `false`.
- `reading_aids.syllable_options.separator`: default `"·"`. Syllable separator. What is drawn between syllables. Text.
- `reading_aids.syllable_options.left_min`: default 2 letters. Syllable first break. Fewest letters before the first break. From 1 to 10 letters, in steps of 1.
- `reading_aids.syllable_options.right_min`: default 2 letters. Syllable last break. Fewest letters after the last break. From 1 to 10 letters, in steps of 1.
- `reading_aids.syllable_options.min_word_len`: default 4 letters. Syllable shortest word. Words shorter than this are never split. From 1 to 20 letters, in steps of 1.
- `reading_aids.syllable_options.skip_urls`: default on (`true`). Syllables skip addresses. Leave web and e-mail addresses alone. On or off: `true` or `false`.
- `reading_aids.syllable_options.skip_code`: default on (`true`). Syllables skip code. Leave code alone. On or off: `true` or `false`.

## Preview: the `[preview]` section

- `preview.auto_reload`: default off (`false`). Reload the preview. Reload the browser preview after each save, through a small server on this computer only. On or off: `true` or `false`.
- `preview.live`: default off (`false`). Live preview. With reloading on, also reload when typing pauses. On or off: `true` or `false`.

## Define word: the `[lexicon]` section

- `lexicon.glossary`: default not set. Glossary. Your own glossary, looked up before the dictionary: term: definition lines, or Star's JSON. Not set uses glossary.txt in the settings folder. Text; empty means not set.
- `lexicon.data_file`: default not set. Dictionary file. The define-word dictionary, lexicon-en.twlex. Not set looks beside the program. Text; empty means not set.

## Reading statistics: the `[stats]` section

- `stats.enabled`: default on (`true`). Reading statistics. Count the time read aloud, the furthest point, and sessions for each document. On or off: `true` or `false`.

## Interface: the `[interface]` section

- `interface.language`: default English (`"en"`). Interface language. The language of textweaver's own words, from the next start. Only English is complete; the others are for testing. Choices: `"en"` (English), `"en-XA"` (test: accented), `"ar-XB"` (test: right to left). Other values may be written too.

## Kept by textweaver

textweaver writes these itself, such as a question already asked. They are in the file, but not on the settings screen.

- `display.theme_explicit`: default off (`false`). Theme picked. Set when you pick a theme; it stops following the system. On or off: `true` or `false`.
- `accessibility.hybrid_offered`: default off (`false`). Hybrid mode offered. Set once textweaver has asked whether to use hybrid mode. On or off: `true` or `false`.

## See also

- [Settings](settings.md): where settings live, and export, import, and reset.
- [JSON-RPC](json-rpc.md): `settings_schema`, `get_setting`, and `set_setting`.
- [Documentation index](README.md)
