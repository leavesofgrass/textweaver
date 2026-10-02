# Settings reference

Every setting in `settings.toml`, section by section, in the order the settings screen lists them. Each item gives the key, its default, its label on the settings screen, what it does, and the values it takes, then whether it syncs between computers ([Syncing between computers](sync.md)). [Settings](settings.md) explains where the file lives, and how to export, import, and reset it.

This page is generated from the settings schema by `cargo xtask settings-doc`. Do not edit it by hand; CI checks that it is current.

## Speech: the `[speech]` section

- `speech.backend`: default automatic (`"auto"`). Speech engine. The speech engine: auto picks the best one available. A change restarts speech. Choices: `"auto"` (automatic), `"eci"` (Eloquence), `"sapi"` (SAPI 5 voices), `"espeak"` (eSpeak NG), `"speechd"` (Speech Dispatcher), `"nsspeech"` (Apple NSSpeech), `"avspeech"` (Apple AVSpeech), `"dectalk"` (DECtalk), `"omnivox"` (Omnivox), `"null"` (silent). Other values may be written too. Stays on this computer.
- `speech.rate`: default 265 words per minute. Rate. How fast textweaver speaks. From 50 to 900 words per minute, in steps of 20. Syncs between computers.
- `speech.volume`: default 100 percent. Volume. How loud textweaver speaks. From 0 to 100 percent, in steps of 10. Stays on this computer.
- `speech.pitch`: default 0 semitones. Pitch. Higher or lower than the voice's own pitch. From -12 to 12 semitones, in steps of 1. Syncs between computers.
- `speech.voice`: default not set. Voice. The voice's id; not set picks one automatically. Choose Voice lists them. Text; empty means not set. Stays on this computer.
- `speech.prefer_voice`: default `"eloquence"`. Preferred voice. When no voice is set, the first voice whose name contains this, such as eloquence. Text; empty means not set. Stays on this computer.
- `speech.favorite_voices`: default an empty list. Favorite voices. Voices listed first in Choose Voice, by id. A list of texts, such as `["a", "b"]`. Syncs between computers, with favorite voices.
- `speech.punctuation`: default `"some"`. Punctuation. How much punctuation is spoken. Choices: `"none"`, `"some"`, `"all"`. Syncs between computers.
- `speech.split_caps`: default off (`false`). Split capitals. Say words joined with capitals, such as TextWeaver, as separate words. On or off: `true` or `false`. Syncs between computers.
- `speech.caps`: default a higher pitch (`"pitch"`). Capitals. How a capital letter is marked when characters are spoken and typed. Choices: `"none"` (not marked), `"tone"` (a tone), `"pitch"` (a higher pitch), `"say_cap"` (say cap). Syncs between computers.
- `speech.auto_play`: default off (`false`). Read on opening. Start reading when a document opens. On or off: `true` or `false`. Syncs between computers.
- `speech.skip_code`: default on (`true`). Skip code blocks. Do not speak code blocks. On or off: `true` or `false`. Syncs between computers.
- `speech.speed_presets`: default 4 entries. Speed presets. Named rates that F8 cycles through. A table of names and values, edited in the file. Syncs between computers.
- `speech.voices_by_language`: default none. Voices by language. The voice for each interface language, by language tag, such as es = the voice's id. A language not listed uses the engine's first voice for it. A table of names and values, edited in the file. Stays on this computer.
- `speech.latency_offset_ms`: default 120 milliseconds. Highlight delay. How long after an engine reports a word the highlight moves, for engines timed by their audio clock. From 0 to 1000 milliseconds, in steps of 10. Stays on this computer.
- `speech.output_device`: default not set. Output device. The sound device speech plays on, by its id; tw backends --devices lists them. Not set uses the system's default, and so does a device that is not connected. Text; empty means not set. Stays on this computer.
- `speech.verbosity`: default `"normal"`. Verbosity. How much textweaver says about what it does. Choices: `"low"`, `"normal"`, `"high"`. Syncs between computers.
- `speech.eci.dictionaries`: default on (`true`). Eloquence dictionaries. The community pronunciation dictionaries for Eloquence: on, off, or a folder of your own. Choices: `true` (on), `false` (off). Other values may be written too. Stays on this computer.
- `speech.eci.library`: default not set. Eloquence library. The ECI library to load; not set searches the usual places. Text; empty means not set. Stays on this computer.
- `speech.eci.code_factory`: default off (`false`). Search Code Factory's Eloquence. Also look for Code Factory's Eloquence for Windows. Its license may not cover other programs. On or off: `true` or `false`. Stays on this computer.
- `speech.sapi.onecore`: default on (`true`). OneCore voices. Also list the Windows OneCore voices through SAPI 5. On or off: `true` or `false`. Stays on this computer.
- `speech.apple.backend`: default automatic (`"auto"`). Apple speech engine. Which of Apple's speech engines to use on macOS. Choices: `"auto"` (automatic), `"nsspeech"` (NSSpeechSynthesizer), `"avspeech"` (AVSpeechSynthesizer). Stays on this computer.
- `speech.dectalk.library`: default not set. DECtalk library. The DECtalk library to load; not set searches the usual places. Text; empty means not set. Stays on this computer.
- `speech.piper.voices`: default not set. Piper voices folder. The folder of Piper voices; not set uses the piper folder in textweaver's data folder. Text; empty means not set. Stays on this computer.
- `speech.piper.voice`: default not set. Piper voice. The Piper voice to start with, by id; not set takes the first installed. Text; empty means not set. Stays on this computer.
- `speech.piper.phonemizer`: default automatic (`"auto"`). Piper phonemizer. How Piper turns text into sounds: the espeak-ng library when installed, that library, or textweaver's own. Choices: `"auto"` (automatic), `"library"` (espeak-ng library), `"rust"` (textweaver's own). Stays on this computer.
- `speech.voice_params`: default none. Rate and pitch per voice. The rate and pitch each voice was last used at; choosing a voice again brings them back. A table of names and values, edited in the file. Stays on this computer.

## Highlight: the `[highlight]` section

- `highlight.enabled`: default on (`true`). Highlight spoken text. Highlight the word or sentence being read. On or off: `true` or `false`. Syncs between computers.
- `highlight.granularity`: default the word (`"word"`). Highlight. What the reading highlight covers. Choices: `"word"` (the word), `"sentence"` (the sentence), `"both"` (the word and the sentence). Syncs between computers.
- `highlight.lead_words`: default 1 words. Highlight lead. Draw the highlight this many words ahead of the word heard (1 is the word heard). From -5 to 5 words, in steps of 1. Syncs between computers.
- `highlight.speed`: default 1 times. Highlight speed. Speed of the timed highlight for engines that report no words. From 0.5 to 1.5 times, in steps of 0.1. Syncs between computers.
- `highlight.color`: default the theme's color (`"theme"`). Word highlight color. A color name or #rrggbb over the theme's word highlight; theme keeps the theme's. Choices: `"theme"` (the theme's color), `"blue"`, `"orange"`, `"navy"` (dark blue), `"skyblue"` (sky blue), `"teal"`, `"gold"`, `"yellow"`, `"purple"`, `"pink"`, `"brown"`, `"gray"`, `"black"`, `"white"`. Other values may be written too. Syncs between computers.
- `highlight.sentence_color`: default not set. Sentence highlight color. A color name or #rrggbb over the theme's sentence highlight; not set keeps the theme's. Choices: `"theme"` (the theme's color), `"blue"`, `"orange"`, `"navy"` (dark blue), `"skyblue"` (sky blue), `"teal"`, `"gold"`, `"yellow"`, `"purple"`, `"pink"`, `"brown"`, `"gray"`, `"black"`, `"white"`. Other values may be written too. Syncs between computers.

## Speaking text: the `[normalization]` section

- `normalization.math`: default on (`true`). Speak math. Speak math notation in words. On or off: `true` or `false`. Syncs between computers.
- `normalization.math_verbosity`: default `"normal"`. Math verbosity. How explicit spoken math is: low says a over b, normal and high say more. Choices: `"low"`, `"normal"`, `"high"`. Syncs between computers.
- `normalization.asciimath_delimiter`: default not set. ASCIIMath delimiter. The character around ASCIIMath, usually a backtick; not set reads no ASCIIMath. Text; empty means not set. Syncs between computers.
- `normalization.abbreviations`: default on (`true`). Expand abbreviations. Say abbreviations in full, such as Doctor for Dr. On or off: `true` or `false`. Syncs between computers.
- `normalization.abbrev_expansions`: default none. Your abbreviations. Abbreviations of your own and what they stand for. A table of names and values, edited in the file. Syncs between computers.
- `normalization.numbers`: default on (`true`). Numbers in words. Say numbers, dates, times, and money in words. On or off: `true` or `false`. Syncs between computers.
- `normalization.use_pronunciations`: default on (`true`). Use pronunciations. Apply your pronunciation list. On or off: `true` or `false`. Syncs between computers.
- `normalization.pronunciations`: default none. Pronunciations. Words and how to say them. A table of names and values, edited in the file. Syncs between computers, with the glossary.
- `normalization.table_mode`: default with rows and columns (`"structured"`). Tables. How tables are read. Choices: `"structured"` (with rows and columns), `"flat"` (as text), `"skip"` (skipped). Syncs between computers.
- `normalization.footnote_mode`: default where they are marked (`"inline"`). Footnotes. Where footnotes are read. Choices: `"inline"` (where they are marked), `"deferred"` (at the end), `"skip"` (skipped). Syncs between computers.
- `normalization.community_lexicon.enabled`: default off (`false`). Community lexicon. Apply the community pronunciation dictionaries for engines other than Eloquence. On or off: `true` or `false`. Syncs between computers.
- `normalization.community_lexicon.dir`: default not set. Community lexicon folder. The folder holding the dictionary files; not set looks beside textweaver. Text; empty means not set. Stays on this computer.
- `normalization.community_lexicon.language`: default US English (`"ENU"`). Community lexicon language. The dictionaries' language. Choices: `"ENU"` (US English), `"DEU"` (German). Other values may be written too. Syncs between computers.

## Reading: the `[reading]` section

- `reading.auto_resume`: default on (`true`). Resume where you left off. Go back to the saved position when a document opens. On or off: `true` or `false`. Syncs between computers.
- `reading.nav_history_size`: default 50 places. Back history. How many places Back remembers. From 1 to 1000 places, in steps of 10. Syncs between computers.
- `reading.wrap_navigation`: default off (`false`). Wrap navigation. Moving past the end of the document goes on from the start. On or off: `true` or `false`. Syncs between computers.
- `reading.cursor_follows_speech`: default on (`true`). Cursor follows speech. The cursor moves with the word being read. On or off: `true` or `false`. Syncs between computers.
- `reading.citations`: default skipped (`"off"`). Citations. Citations in continuous reading: skipped, or said in words. Choices: `"off"` (skipped), `"words"` (in words). Syncs between computers.
- `reading.ocr`: default on (`true`). Recognize scanned pages. Read the text of scanned PDFs and pictures by recognizing it (OCR). On or off: `true` or `false`. Syncs between computers.
- `reading.ocr_lang`: default the document's (`""`). Scanned text language. The language of scanned text, as Tesseract codes such as fra or deu+eng; empty means the document's own language, else English. Choices: `""` (the document's), `"eng"` (English), `"fra"` (French), `"deu"` (German), `"spa"` (Spanish). Other values may be written too. Syncs between computers.
- `reading.ocr_engine`: default automatic (`"auto"`). OCR engine. Which engine recognizes scanned pages: ocrs for English and Tesseract for other languages, or one of them always. Choices: `"auto"` (automatic), `"ocrs"`, `"tesseract"` (Tesseract), `"paddle"` (PaddleOCR (experimental)). Stays on this computer.
- `reading.math_engine`: default textweaver (`"builtin"`). Math speech. Which engine reads math aloud: textweaver's own, or MathCAT in ClearSpeak or SimpleSpeak, in the document's language. MathCAT needs a build that includes it; otherwise textweaver's own is used. Choices: `"builtin"` (textweaver), `"mathcat"` (MathCAT ClearSpeak), `"mathcat_simplespeak"` (MathCAT SimpleSpeak). Syncs between computers.
- `reading.math_display`: default `"source"`. Math on screen. How math looks in the reading view: as its source, such as x^2, or as Unicode, such as x with a superscript 2. Speech and edit mode always use the source. Choices: `"source"`, `"unicode"` (Unicode). Syncs between computers.
- `reading.revisions`: default automatic (`"auto"`). Tracked changes. How tracked changes in Word, OpenDocument, and RTF files are read: said in place at high verbosity (automatic), always said, or never said, reading the final text. Applies when a document is opened. Choices: `"auto"` (automatic), `"marked"` (always say them), `"final"` (final text only). Syncs between computers.

## Display: the `[display]` section

- `display.theme`: default `"galaxy"`. Theme. The color theme. Choices: . Other values may be written too. Syncs between computers.
- `display.follow_os_theme`: default on (`true`). Follow the system theme. At startup, use a light, dark, or high-contrast theme like the system, unless you picked one. On or off: `true` or `false`. Syncs between computers.
- `display.wrap_width`: default 0 columns. Wrap width. Wrap lines at this many columns; 0 uses the whole width. From 0 to 400 columns, in steps of 10. Stays on this computer.
- `display.tab_width`: default 4 columns. Tab width. Columns a tab takes. From 1 to 16 columns, in steps of 1. Syncs between computers.
- `display.show_line_numbers`: default off (`false`). Line numbers. Show line numbers. On or off: `true` or `false`. Syncs between computers.
- `display.scroll_margin`: default 3 lines. Scroll margin. Lines kept in view above and below the cursor. From 0 to 20 lines, in steps of 1. Syncs between computers.

## Editing: the `[editing]` section

- `editing.autosave_recovery`: default on (`true`). Recovery snapshots. Keep a copy of unsaved work and offer it after a crash. On or off: `true` or `false`. Syncs between computers.
- `editing.autosave_interval_secs`: default 20 seconds. Snapshot interval. Seconds between recovery snapshots while there are unsaved changes. From 5 to 3600 seconds, in steps of 5. Syncs between computers.
- `editing.echo_characters`: default on (`true`). Echo characters. Say each character typed. On or off: `true` or `false`. Syncs between computers.
- `editing.echo_words`: default on (`true`). Echo words. Say each word typed. On or off: `true` or `false`. Syncs between computers.
- `editing.echo_deletions`: default on (`true`). Echo deletions. Say what Backspace and Delete remove. On or off: `true` or `false`. Syncs between computers.
- `editing.echo_lines_on_move`: default on (`true`). Echo lines. Say the line when the caret moves to another line. On or off: `true` or `false`. Syncs between computers.
- `editing.undo_steps`: default 1000 steps. Undo steps. Most undo steps kept while editing. From 1 to 100000 steps, in steps of 100. Stays on this computer.
- `editing.undo_memory_mb`: default 50 megabytes. Undo memory. Most memory the undo history may use. From 1 to 4096 megabytes, in steps of 16. Stays on this computer.
- `editing.author`: default empty (`""`). Author. The author written into new documents made from a template; empty leaves it blank. Text. Stays on this computer.

## Library: the `[library]` section

- `library.recent_limit`: default 20 files. Recent files. How many recent files are remembered. From 1 to 500 files, in steps of 5. Syncs between computers.
- `library.folders`: default an empty list. Library folders. Folders whose documents the library lists, and whose positions sync between computers. A list of texts, such as `["a", "b"]`. Stays on this computer.

## Keyboard: the `[keyboard]` section

- `keyboard.character_keys`: default on (`true`). Single-key shortcuts. Browse keys such as h and period. Off, dictation and typing never trigger commands. On or off: `true` or `false`. Syncs between computers.
- `keyboard.preset`: default screen reader style (`"default"`). Keys. The default keys: like NVDA's and JAWS's browse mode, or textweaver's earlier keys. Used from the next start. Choices: `"default"` (screen reader style), `"classic"`. Stays on this computer.
- `keyboard.digit_row`: default automatic (`"auto"`). Digit row. How the terminal recognizes the digit keys for heading levels: auto, or a French AZERTY keyboard. Choices: `"auto"` (automatic), `"azerty"` (AZERTY). Stays on this computer.

## Accessibility: the `[accessibility]` section

- `accessibility.mode`: default `"self-voicing"`. Accessibility mode. Self-voicing speaks everything; screen reader leaves speech to your screen reader; hybrid voices reading only. Choices: `"self-voicing"`, `"screen-reader"` (screen reader), `"hybrid"`. Stays on this computer.
- `accessibility.say_all`: default on the status line (`"screen"`). Say all with a screen reader. Continuous reading in screen-reader mode: a sentence at a time on the status line, or textweaver's voice. Choices: `"screen"` (on the status line), `"voice"` (with textweaver's voice). Stays on this computer.
- `accessibility.quiet_screen`: default off (`false`). Quiet screen while reading. Keep the screen still while textweaver reads aloud. On or off: `true` or `false`. Stays on this computer.
- `accessibility.cursor`: default follows focus (`"follow"`). Cursor. Where the terminal's cursor waits: on what you are working on, or on the status line. Choices: `"follow"` (follows focus), `"status"` (on the status line). Stays on this computer.
- `accessibility.interface_announcements`: default automatic (`"auto"`). Interface announcements. How much textweaver says about itself: dialogs, progress, hints, and routine confirmations. Errors and answers to what you asked are always said. Automatic is minimal with a screen reader, normal when self-voicing. Choices: `"auto"` (automatic), `"off"`, `"minimal"`, `"normal"`, `"full"`. Syncs between computers.

## Export: the `[export]` section

- `export.subtitle_format`: default SubRip (`"srt"`). Subtitle format. The format of subtitles written without a file name. Choices: `"srt"` (SubRip), `"vtt"` (WebVTT). Syncs between computers.
- `export.subtitle_word_level`: default off (`false`). Word subtitles. One subtitle per word instead of caption lines. On or off: `true` or `false`. Syncs between computers.
- `export.subtitles_with_audio`: default off (`false`). Subtitles with audio. Always write subtitles beside exported audio. On or off: `true` or `false`. Syncs between computers.

## Braille: the `[braille]` section

- `braille.math_code`: default Nemeth (`"nemeth"`). Math braille. The braille code for math in BRF files and while exploring a formula with MathCAT: Nemeth, or UEB mathematics. It needs a build that includes MathCAT; otherwise math is written as its spoken words. Choices: `"nemeth"` (Nemeth), `"ueb"` (UEB). Syncs between computers.
- `braille.table_format`: default `"linear"`. Braille tables. How BRF files lay out tables: linear, one row per line with semicolons between entries; listed, each row a heading with each entry on its own line after its column heading; or stairstep, each entry two cells right of the one before, for tables of up to four columns. Choices: `"linear"`, `"listed"`, `"stairstep"`. Syncs between computers.

## Reading aids: the `[reading_aids]` section

- `reading_aids.rsvp.wpm`: default 300 words per minute. RSVP rate. Words per minute of rapid serial visual presentation. From 60 to 1500 words per minute, in steps of 20. Syncs between computers.
- `reading_aids.rsvp.pacing`: default its own timer (`"timer"`). RSVP pacing. What moves the RSVP word on: its own timer, or speech. Choices: `"timer"` (its own timer), `"external"` (speech). Syncs between computers.
- `reading_aids.rsvp.clause_pause`: default 50 percent. RSVP clause pause. Extra time after a comma, colon, dash, or bracket, in percent of a word's time. From 0 to 500 percent, in steps of 10. Syncs between computers.
- `reading_aids.rsvp.sentence_pause`: default 100 percent. RSVP sentence pause. Extra time at the end of a sentence, in percent. From 0 to 500 percent, in steps of 10. Syncs between computers.
- `reading_aids.rsvp.paragraph_pause`: default 150 percent. RSVP paragraph pause. Extra time at the end of a paragraph, in percent. From 0 to 500 percent, in steps of 10. Syncs between computers.
- `reading_aids.rsvp.long_word_len`: default 8 letters. RSVP long word. Words longer than this many letters get extra time. From 1 to 40 letters, in steps of 1. Syncs between computers.
- `reading_aids.rsvp.long_word_step`: default 10 percent. RSVP long word step. Extra time per letter beyond a long word's length, in percent. From 0 to 100 percent, in steps of 5. Syncs between computers.
- `reading_aids.rsvp.long_word_max`: default 80 percent. RSVP long word most. Most extra time a long word gets, in percent. From 0 to 500 percent, in steps of 10. Syncs between computers.
- `reading_aids.rsvp.show_previous`: default on (`true`). RSVP previous word. Show the previous word too. On or off: `true` or `false`. Syncs between computers.
- `reading_aids.rsvp.show_next`: default on (`true`). RSVP next word. Show the next word too. On or off: `true` or `false`. Syncs between computers.
- `reading_aids.rsvp.position`: default top center (`"top-center"`). RSVP position. Where the RSVP word appears. Choices: `"top-left"` (top left), `"top-center"` (top center), `"top-right"` (top right), `"center-left"` (middle left), `"center"` (middle), `"center-right"` (middle right), `"bottom-left"` (bottom left), `"bottom-center"` (bottom center), `"bottom-right"` (bottom right). Syncs between computers.
- `reading_aids.rsvp.font_size_pt`: default 48 points. RSVP size. Size of the RSVP word in the GUI. From 8 to 200 points, in steps of 2. Syncs between computers.
- `reading_aids.rsvp.lead_words`: default 0 words. RSVP lead. With speech pacing, show this many words ahead of the word spoken. From -5 to 5 words, in steps of 1. Syncs between computers.
- `reading_aids.bionic`: default off (`false`). Bionic reading. Draw the start of each word in bold. On or off: `true` or `false`. Syncs between computers.
- `reading_aids.bionic_options.ratio`: default 0.4. Bionic share. How much of each word is bold. From 0.1 to 0.9, in steps of 0.1. Syncs between computers.
- `reading_aids.bionic_options.min_word_len`: default 2 letters. Bionic shortest word. Words shorter than this are left alone. From 1 to 20 letters, in steps of 1. Syncs between computers.
- `reading_aids.bionic_options.skip_numbers`: default on (`true`). Bionic skips numbers. Leave words with digits alone. On or off: `true` or `false`. Syncs between computers.
- `reading_aids.bionic_options.skip_urls`: default on (`true`). Bionic skips addresses. Leave web and e-mail addresses alone. On or off: `true` or `false`. Syncs between computers.
- `reading_aids.bionic_options.skip_code`: default on (`true`). Bionic skips code. Leave code alone. On or off: `true` or `false`. Syncs between computers.
- `reading_aids.spacing.line_height`: default 1.5 times. Line height. Line height in multiples of the font size; WCAG's value is 1.5. From 1 to 3 times, in steps of 0.1. Syncs between computers.
- `reading_aids.spacing.paragraph_spacing`: default 1 times. Paragraph spacing. Space after each paragraph, in multiples of the font size. From 0 to 4 times, in steps of 0.25. Syncs between computers.
- `reading_aids.spacing.letter_spacing`: default 0 times. Letter spacing. Extra space between letters, in multiples of the font size. From 0 to 0.5 times, in steps of 0.02. Syncs between computers.
- `reading_aids.spacing.word_spacing`: default 0 times. Word spacing. Extra space between words, in multiples of the font size. From 0 to 1 times, in steps of 0.04. Syncs between computers.
- `reading_aids.font.family`: default sans serif (`"sans"`). Font. The GUI's reading font; any installed family may be typed. Choices: `"system-ui"` (the system font), `"sans"` (sans serif), `"serif"`, `"monospace"`, `"atkinson"` (Atkinson Hyperlegible), `"opendyslexic"` (OpenDyslexic), `"lexend"` (Lexend). Other values may be written too. Syncs between computers.
- `reading_aids.font.size_pt`: default 14 points. Font size. The GUI's font size. From 6 to 72 points, in steps of 1. Syncs between computers.
- `reading_aids.font.weight`: default 400. Font weight. 400 is regular, 700 bold. From 100 to 900, in steps of 100. Syncs between computers.
- `reading_aids.ruler.mode`: default `"off"`. Reading ruler. Mark the current line, or a band of lines. Choices: `"off"`, `"current_line"` (current line), `"ruler"`. Syncs between computers.
- `reading_aids.ruler.scope`: default the whole line (`"line"`). Ruler covers. A wrapped row, or the whole line. Choices: `"row"` (a row), `"line"` (the whole line). Syncs between computers.
- `reading_aids.ruler.rows_above`: default 1 rows. Ruler rows above. Rows of the band above the current one. From 0 to 10 rows, in steps of 1. Syncs between computers.
- `reading_aids.ruler.rows_below`: default 1 rows. Ruler rows below. Rows of the band below the current one. From 0 to 10 rows, in steps of 1. Syncs between computers.
- `reading_aids.ruler.mask_outside`: default off (`false`). Ruler mask. Dim the rows outside the band. On or off: `true` or `false`. Syncs between computers.
- `reading_aids.syllables`: default off (`false`). Syllables. Draw words split into syllables with a middle dot; speech is unchanged. On or off: `true` or `false`. Syncs between computers.
- `reading_aids.difficult_words`: default off (`false`). Difficult words. Underline rare words, and name them on word moves at high verbosity. On or off: `true` or `false`. Syncs between computers.
- `reading_aids.syllable_options.separator`: default `"·"`. Syllable separator. What is drawn between syllables. Text. Syncs between computers.
- `reading_aids.syllable_options.left_min`: default 2 letters. Syllable first break. Fewest letters before the first break. From 1 to 10 letters, in steps of 1. Syncs between computers.
- `reading_aids.syllable_options.right_min`: default 2 letters. Syllable last break. Fewest letters after the last break. From 1 to 10 letters, in steps of 1. Syncs between computers.
- `reading_aids.syllable_options.min_word_len`: default 4 letters. Syllable shortest word. Words shorter than this are never split. From 1 to 20 letters, in steps of 1. Syncs between computers.
- `reading_aids.syllable_options.skip_urls`: default on (`true`). Syllables skip addresses. Leave web and e-mail addresses alone. On or off: `true` or `false`. Syncs between computers.
- `reading_aids.syllable_options.skip_code`: default on (`true`). Syllables skip code. Leave code alone. On or off: `true` or `false`. Syncs between computers.
- `reading_aids.difficult_definitions`: default off (`false`). Difficult word definitions. With difficult words marked, at high verbosity also say a difficult word's first definition from the dictionary. On or off: `true` or `false`. Syncs between computers.

## Preview: the `[preview]` section

- `preview.auto_reload`: default off (`false`). Reload the preview. Reload the browser preview after each save, through a small server on this computer only. On or off: `true` or `false`. Syncs between computers.
- `preview.live`: default off (`false`). Live preview. With reloading on, also reload when typing pauses. On or off: `true` or `false`. Syncs between computers.

## Define word: the `[lexicon]` section

- `lexicon.glossary`: default not set. Glossary. Your own glossary, looked up before the dictionary: term: definition lines, or Star's JSON. Not set uses glossary.txt in the settings folder. Text; empty means not set. Stays on this computer.
- `lexicon.data_file`: default not set. Dictionary file. The define-word dictionary, lexicon-en.twlex. Not set looks beside the program. Text; empty means not set. Stays on this computer.

## Reading statistics: the `[stats]` section

- `stats.enabled`: default on (`true`). Reading statistics. Count the time read aloud, the furthest point, and sessions for each document. On or off: `true` or `false`. Syncs between computers.

## Summaries: the `[summary]` section

- `summary.sentences`: default 5 sentences. Summary sentences. How many sentences Summarize and tw summarize give, 1 to 50. From 1 to 50 sentences, in steps of 1. Syncs between computers.

## Dictation: the `[dictation]` section

- `dictation.speak_while_recording`: default off (`false`). Speak while dictating. Say dictated words as they come. Off, they are shown on the status line and said at each pause, so the microphone does not hear the voice. On or off: `true` or `false`. Syncs between computers.
- `dictation.model_dir`: default not set. Dictation model folder. A folder holding a Whisper model for dictation. Not set uses the dictation model below, in the data folder. Text; empty means not set. Stays on this computer.
- `dictation.model`: default base.en, the default (`"whisper-base.en"`). Dictation model. The Whisper model dictation uses when no folder is set. Download the dictation model, in the Tools menu, gets it. Choices: `"whisper-base.en"` (base.en, the default), `"whisper-small.en"` (small.en, larger and more accurate). Other values may be written too. Stays on this computer.

## Interface: the `[interface]` section

- `interface.language`: default English (`"en"`). Interface language. The language of textweaver's own words, changed at once. The voice follows it when the engine has one for it; otherwise the voice stays. Choices: `"en"` (English), `"es"` (Español), `"fr"` (Français), `"de"` (Deutsch), `"pt"` (Português), `"ar"` (العربية), `"en-XA"` (test: accented), `"ar-XB"` (test: right to left). Other values may be written too. Syncs between computers.
- `interface.rtl`: default automatic (`"auto"`). Right-to-left display. Whether the terminal reader reorders right-to-left text for display: automatic leaves it to terminals that do it themselves. Speech and the screen reader always get the text in reading order. Choices: `"auto"` (automatic), `"on"`, `"off"`. Stays on this computer.

## Window: the `[gui]` section

- `gui.announce`: default live region (`"live"`). Announcements. How the window's messages reach the screen reader, from the next start: a live region, or UI Automation notifications (Windows only). Choices: `"live"` (live region), `"uia"` (UI Automation notifications). Stays on this computer.
- `gui.auto_hide_menu`: default off (`false`). Hide the menu bar. Windows: hide the window's menu bar until Alt or F10 shows it; it hides again when the menu closes. No effect on Linux, whose menus are the F10 list, or on macOS. On or off: `true` or `false`. Stays on this computer.

## Colors: the `[colors]` section

- `colors.ruler`: default the theme's color (`"theme"`). Reading ruler color. The band of the reading ruler and the marked current line; the ruler keeps its underline or bold. A color name or #rrggbb; the theme's color by default. Choices: `"theme"` (the theme's color), `"blue"`, `"orange"`, `"navy"` (dark blue), `"skyblue"` (sky blue), `"teal"`, `"gold"`, `"yellow"`, `"purple"`, `"pink"`, `"brown"`, `"gray"`, `"black"`, `"white"`. Other values may be written too. Syncs between computers.
- `colors.difficult_words`: default the theme's color (`"theme"`). Difficult words color. The underline of difficult words; they stay underlined and are named at high verbosity. A color name or #rrggbb; the theme's color by default. Choices: `"theme"` (the theme's color), `"blue"`, `"orange"`, `"navy"` (dark blue), `"skyblue"` (sky blue), `"teal"`, `"gold"`, `"yellow"`, `"purple"`, `"pink"`, `"brown"`, `"gray"`, `"black"`, `"white"`. Other values may be written too. Syncs between computers.
- `colors.syllables`: default the theme's color (`"theme"`). Syllable marks color. The middle dots between syllables. A color name or #rrggbb; the theme's color by default. Choices: `"theme"` (the theme's color), `"blue"`, `"orange"`, `"navy"` (dark blue), `"skyblue"` (sky blue), `"teal"`, `"gold"`, `"yellow"`, `"purple"`, `"pink"`, `"brown"`, `"gray"`, `"black"`, `"white"`. Other values may be written too. Syncs between computers.
- `colors.misspellings`: default the theme's color (`"theme"`). Misspellings color. The underline of misspelled words, in the window; they are also said. A color name or #rrggbb; the theme's color by default. Choices: `"theme"` (the theme's color), `"blue"`, `"orange"`, `"navy"` (dark blue), `"skyblue"` (sky blue), `"teal"`, `"gold"`, `"yellow"`, `"purple"`, `"pink"`, `"brown"`, `"gray"`, `"black"`, `"white"`. Other values may be written too. Syncs between computers.
- `colors.lint`: default the theme's color (`"theme"`). Lint marks color. The underline of Markdown lint and grammar problems, in the window; they are also said. A color name or #rrggbb; the theme's color by default. Choices: `"theme"` (the theme's color), `"blue"`, `"orange"`, `"navy"` (dark blue), `"skyblue"` (sky blue), `"teal"`, `"gold"`, `"yellow"`, `"purple"`, `"pink"`, `"brown"`, `"gray"`, `"black"`, `"white"`. Other values may be written too. Syncs between computers.
- `colors.find_match`: default the theme's color (`"theme"`). Search match color. The band behind search matches; they stay underlined. A color name or #rrggbb; the theme's color by default. Choices: `"theme"` (the theme's color), `"blue"`, `"orange"`, `"navy"` (dark blue), `"skyblue"` (sky blue), `"teal"`, `"gold"`, `"yellow"`, `"purple"`, `"pink"`, `"brown"`, `"gray"`, `"black"`, `"white"`. Other values may be written too. Syncs between computers.
- `colors.selection`: default the theme's color (`"theme"`). Selection color. The band behind selected text. A color name or #rrggbb; the theme's color by default. Choices: `"theme"` (the theme's color), `"blue"`, `"orange"`, `"navy"` (dark blue), `"skyblue"` (sky blue), `"teal"`, `"gold"`, `"yellow"`, `"purple"`, `"pink"`, `"brown"`, `"gray"`, `"black"`, `"white"`. Other values may be written too. Syncs between computers.
- `colors.focus`: default the theme's color (`"theme"`). Focus color. The focus outline and the focused item of a list; they stay bold. A color name or #rrggbb; the theme's color by default. Choices: `"theme"` (the theme's color), `"blue"`, `"orange"`, `"navy"` (dark blue), `"skyblue"` (sky blue), `"teal"`, `"gold"`, `"yellow"`, `"purple"`, `"pink"`, `"brown"`, `"gray"`, `"black"`, `"white"`. Other values may be written too. Syncs between computers.
- `colors.links`: default the theme's color (`"theme"`). Link color. The color of links; they stay underlined. A color name or #rrggbb; the theme's color by default. Choices: `"theme"` (the theme's color), `"blue"`, `"orange"`, `"navy"` (dark blue), `"skyblue"` (sky blue), `"teal"`, `"gold"`, `"yellow"`, `"purple"`, `"pink"`, `"brown"`, `"gray"`, `"black"`, `"white"`. Other values may be written too. Syncs between computers.
- `colors.headings`: default the theme's color (`"theme"`). Heading color. The color of headings; they stay bold. A color name or #rrggbb; the theme's color by default. Choices: `"theme"` (the theme's color), `"blue"`, `"orange"`, `"navy"` (dark blue), `"skyblue"` (sky blue), `"teal"`, `"gold"`, `"yellow"`, `"purple"`, `"pink"`, `"brown"`, `"gray"`, `"black"`, `"white"`. Other values may be written too. Syncs between computers.
- `colors.status_bar`: default the theme's color (`"theme"`). Status bar color. The band of the status and title bars. A color name or #rrggbb; the theme's color by default. Choices: `"theme"` (the theme's color), `"blue"`, `"orange"`, `"navy"` (dark blue), `"skyblue"` (sky blue), `"teal"`, `"gold"`, `"yellow"`, `"purple"`, `"pink"`, `"brown"`, `"gray"`, `"black"`, `"white"`. Other values may be written too. Syncs between computers.
- `colors.notes`: default the theme's color (`"theme"`). Note color. The band behind text with a note; it stays italic and underlined. A color name or #rrggbb; the theme's color by default. Choices: `"theme"` (the theme's color), `"blue"`, `"orange"`, `"navy"` (dark blue), `"skyblue"` (sky blue), `"teal"`, `"gold"`, `"yellow"`, `"purple"`, `"pink"`, `"brown"`, `"gray"`, `"black"`, `"white"`. Other values may be written too. Syncs between computers.
- `colors.bookmarks`: default the theme's color (`"theme"`). Bookmark color. The band behind a bookmarked word; it stays bold and underlined. A color name or #rrggbb; the theme's color by default. Choices: `"theme"` (the theme's color), `"blue"`, `"orange"`, `"navy"` (dark blue), `"skyblue"` (sky blue), `"teal"`, `"gold"`, `"yellow"`, `"purple"`, `"pink"`, `"brown"`, `"gray"`, `"black"`, `"white"`. Other values may be written too. Syncs between computers.

## Sync: the `[sync]` section

- `sync.enabled`: default off (`false`). Sync. Sync notes, highlights, bookmarks, and places with your other computers through the sync folder. Tools, Sync, Set up sync turns it on. On or off: `true` or `false`. Stays on this computer.
- `sync.folder`: default not set. Sync folder. The folder your computers share: one kept in step by Syncthing, a cloud folder, or a USB stick. Text; empty means not set. Stays on this computer.
- `sync.device_name`: default empty (`""`). Computer name. This computer's name in sync messages, such as laptop or lab; empty uses Computer 1, Computer 2, and so on. Text. Stays on this computer.
- `sync.places`: default on (`true`). Sync places. Share where you are in each document. On or off: `true` or `false`. Stays on this computer.
- `sync.notes`: default on (`true`). Sync notes. Share notes. On or off: `true` or `false`. Stays on this computer.
- `sync.highlights`: default on (`true`). Sync highlights. Share highlights. On or off: `true` or `false`. Stays on this computer.
- `sync.bookmarks`: default on (`true`). Sync bookmarks. Share bookmarks. On or off: `true` or `false`. Stays on this computer.
- `sync.statistics`: default on (`true`). Sync statistics. Share each computer's reading time and sessions. On or off: `true` or `false`. Stays on this computer.
- `sync.settings`: default on (`true`). Sync settings. Share the portable settings: rate, punctuation, theme, reading aids, and the like. The voice, the engine, the access mode, the key preset, and paths stay on each computer. On or off: `true` or `false`. Stays on this computer.
- `sync.profiles`: default on (`true`). Sync profiles. Share your profiles; which one is in use stays on each computer. On or off: `true` or `false`. Stays on this computer.
- `sync.key_overrides`: default on (`true`). Sync key overrides. Share keymap.toml. A Mac's keys are kept but not used on Windows or Linux, and the reverse. On or off: `true` or `false`. Stays on this computer.
- `sync.words`: default on (`true`). Sync word list. Share your spelling word list. On or off: `true` or `false`. Stays on this computer.
- `sync.glossary`: default on (`true`). Sync glossary. Share your glossary's entries and your pronunciations. On or off: `true` or `false`. Stays on this computer.
- `sync.favorite_voices`: default on (`true`). Sync favorite voices. Share your favorite voices; one this computer does not have is listed as not on this computer. On or off: `true` or `false`. Stays on this computer.
- `sync.position_policy`: default the newest (`"newest"`). Place to resume. Which place a document opens at when another computer has one too: the newest, the furthest, or ask. Choices: `"newest"` (the newest), `"furthest"` (the furthest), `"ask"`. Stays on this computer.

## Optional components: the `[components]` section

- `components.mirror`: default empty (`""`). Components mirror. Where optional components come from first: an https address or a folder on this computer. Empty uses their public sources. Never put a password here. Text. Stays on this computer.

## Kept by textweaver

textweaver writes these itself, such as a question already asked. They are in the file, but not on the settings screen.

- `display.theme_explicit`: default off (`false`). Theme picked. Set when you pick a theme; it stops following the system. On or off: `true` or `false`. Syncs between computers.
- `accessibility.hybrid_offered`: default off (`false`). Hybrid mode offered. Set once textweaver has asked whether to use hybrid mode. On or off: `true` or `false`. Stays on this computer.
- `interface.recent_settings`: default an empty list. Recently changed settings. The settings changed last on the settings screen, listed at its top. Text; empty means not set. Stays on this computer.
- `components.chooser_shown`: default off (`false`). Components list shown. The first-run list of optional components was shown. On or off: `true` or `false`. Stays on this computer.

## See also

- [Settings](settings.md): where settings live, and export, import, and reset.
- [JSON-RPC](json-rpc.md): `settings_schema`, `get_setting`, and `set_setting`.
- [Documentation index](README.md)
