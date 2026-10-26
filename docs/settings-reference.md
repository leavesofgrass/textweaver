# Settings reference

         Every setting in `settings.toml`, in the sections and the order the settings screens list them.          Each item gives the key, its default, its label on the settings screen, what it does,          and the values it takes, then when it applies, if not always, and whether it syncs between computers          ([Syncing between computers](sync.md)). [Settings](settings.md) explains where the file lives,          and how to export, import, and reset it.

         A key names its table in the file and its name in that table: `speech.rate` is `rate` under `[speech]`,          and `speech.cw.wpm` is `wpm` under `[speech.cw]`. The sections are for finding a setting;          the file keeps its own tables.

         This page is generated from the settings schema by `cargo xtask settings-doc`.          Do not edit it by hand; CI checks that it is current.

## Where each table's keys are

- `[accessibility]`: Accessibility and announcements.
- `[authoring]`: Editing.
- `[braille]`: Braille.
- `[colors]`: Colors.
- `[components]`: Updates and components.
- `[dictation]`: Dictation.
- `[display]`: Display.
- `[editing]`: Editing.
- `[export]`: Export.
- `[gui]`: Accessibility and announcements, Toolbars and panels.
- `[highlight]`: Highlight.
- `[interface]`: Display, Keyboard and language.
- `[keyboard]`: Keyboard and language.
- `[lexicon]`: Library and study tools.
- `[library]`: Library and study tools.
- `[live]`: Live documents.
- `[normalization]`: Speaking text, Math.
- `[preview]`: Editing.
- `[reading]`: Reading, Documents, Math.
- `[reading_aids]`: Spacing, font and ruler, Word aids, Word by word (RSVP).
- `[speech]`: Voice and speech, Pauses and verbosity, Speaking text, Reading, Highlight, Morse code, Morse practice.
- `[stats]`: Library and study tools.
- `[summary]`: Library and study tools.
- `[sync]`: Sync.
- `[updates]`: Updates and components.

## Voice and speech

Only the speech engine's own settings are listed here: the voice, rate and pitch for a speaking voice,                  the options of the engine in use, and, while CW is the speech engine,                  the sound of Morse code (`speech.cw.wpm`, `speech.cw.effective_wpm`, `speech.cw.tone_hz` and `speech.cw.volume`),                  moved here from Morse code.

- `speech.backend`: default automatic (`"auto"`). Speech engine. The speech engine: auto picks the best one available. A change restarts speech. Choices: `"auto"` (automatic), `"eci"` (Eloquence), `"sapi"` (SAPI 5 voices), `"espeak"` (eSpeak NG), `"speechd"` (Speech Dispatcher), `"nsspeech"` (Apple NSSpeech), `"avspeech"` (Apple AVSpeech), `"dectalk"` (DECtalk), `"omnivox"` (Omnivox), `"piper"` (Piper), `"cw"` (CW (Morse code)), `"null"` (silent). Other values may be written too. Stays on this computer.
- `speech.voice`: default not set. Voice. The voice's id; not set picks one automatically. The Voices command lists them. Text; empty means not set. Applies when CW (Morse code) is not the speech engine. Stays on this computer.
- `speech.prefer_voice`: default `"eloquence"`. Preferred voice. When no voice is set, the first voice whose name contains this, such as eloquence. Text; empty means not set. Applies when CW (Morse code) is not the speech engine. Stays on this computer.
- `speech.favorite_voices`: default an empty list. Favorite voices. Voices listed first by the Voices command, by id, separated by commas. A list of texts, such as `["a", "b"]`. Applies when CW (Morse code) is not the speech engine. Syncs between computers, with favorite voices.
- `speech.rate`: default 265 words per minute. Rate. How fast textweaver speaks. From 50 to 900 words per minute, in steps of 20. Applies when CW (Morse code) is not the speech engine. Syncs between computers.
- `speech.pitch`: default 0 semitones. Pitch. Higher or lower than the voice's own pitch. From -12 to 12 semitones, in steps of 1. Applies when CW (Morse code) is not the speech engine. Syncs between computers.
- `speech.volume`: default 100 percent. Volume. How loud textweaver speaks. From 0 to 100 percent, in steps of 10. Stays on this computer.
- `speech.cw.prosigns`: default on (`true`). CW prosigns. Send structure as prosigns: BT after headings and paragraphs, AR at the end. Prosigns written in angle brackets, such as <BT>, are sent too. On or off: `true` or `false`. Applies when CW (Morse code) is the speech engine. Syncs between computers.
- `speech.cw.status_line`: default on (`true`). CW characters on the status line. Show each character on the status line as it is sent, for a Braille display. Nothing extra is said. On or off: `true` or `false`. Applies when CW (Morse code) is the speech engine. Syncs between computers.
- `speech.eci.dictionaries`: default on (`true`). Eloquence dictionaries. The community pronunciation dictionaries for Eloquence: on, off, or a folder of your own. Choices: `true` (on), `false` (off). Other values may be written too. Applies when Eloquence is the speech engine. Stays on this computer.
- `speech.eci.library`: default not set. Eloquence library. The ECI library to load; not set searches the usual places. Text; empty means not set. Applies when Eloquence is the speech engine. Stays on this computer.
- `speech.eci.code_factory`: default off (`false`). Search Code Factory's Eloquence. Also look for Code Factory's Eloquence for Windows. Its license may not cover other programs. On or off: `true` or `false`. Applies on Windows only. Applies when Eloquence is the speech engine. Stays on this computer.
- `speech.sapi.onecore`: default on (`true`). OneCore voices. Also list the Windows OneCore voices through SAPI 5. On or off: `true` or `false`. Applies on Windows only. Applies when SAPI 5 voices is the speech engine. Stays on this computer.
- `speech.apple.backend`: default automatic (`"auto"`). Apple speech engine. Which of Apple's speech engines to use on macOS. Choices: `"auto"` (automatic), `"nsspeech"` (NSSpeechSynthesizer), `"avspeech"` (AVSpeechSynthesizer). Applies on macOS only. Applies when Apple NSSpeech or Apple AVSpeech is the speech engine. Stays on this computer.
- `speech.dectalk.library`: default not set. DECtalk library. The DECtalk library to load; not set searches the usual places. Text; empty means not set. Applies when DECtalk is the speech engine. Stays on this computer.
- `speech.espeak.helper`: default automatic (`"auto"`). eSpeak NG helper program. Run eSpeak NG in its own helper program, so an engine crash cannot close textweaver. Automatic uses the helper on Windows when it is installed, and runs eSpeak NG inside textweaver elsewhere. Choices: `"auto"` (automatic), `"always"` (always the helper), `"never"` (inside textweaver). Applies when eSpeak NG is the speech engine. Stays on this computer.
- `speech.piper.voice`: default not set. Piper voice. The Piper voice to start with, by id; not set takes the first installed. Text; empty means not set. Applies when Piper is the speech engine. Stays on this computer.
- `speech.piper.voices`: default not set. Piper voices folder. The folder of Piper voices; not set uses the piper folder in textweaver's data folder. Text; empty means not set. Applies when Piper is the speech engine. Stays on this computer.
- `speech.piper.phonemizer`: default automatic (`"auto"`). Piper phonemizer. How Piper turns text into sounds. The espeak-ng library when installed, that library, or textweaver's own. Choices: `"auto"` (automatic), `"library"` (espeak-ng library), `"rust"` (textweaver's own). Applies when Piper is the speech engine. Stays on this computer.
- `speech.speed_presets`: default 4 entries. Speed presets. Named rates that F8 cycles through. A table of names and values, edited in the file. Applies when CW (Morse code) is not the speech engine. Syncs between computers.
- `speech.voices_by_language`: default none. Voices by language. The voice for each interface language, by language tag, such as es = the voice's id. A language not listed uses the engine's first voice for it. A table of names and values, edited in the file. Applies when CW (Morse code) is not the speech engine. Stays on this computer.
- `speech.voice_params`: default none. Rate and pitch per voice. The rate and pitch each voice was last used at. Choosing a voice again brings them back. A table of names and values, edited in the file. Applies when CW (Morse code) is not the speech engine. Stays on this computer.
- `speech.output_device`: default not set. Output device. The sound device speech plays on, by its id. Not set, or a device that is not connected, uses the system's default. Text; empty means not set. Stays on this computer.

## Pauses and verbosity

- `speech.verbosity`: default `"normal"`. Verbosity. How much textweaver says about what it does. Choices: `"low"`, `"normal"`, `"high"`. Syncs between computers.
- `speech.punctuation`: default `"some"`. Punctuation. How much punctuation is spoken. Choices: `"none"`, `"some"`, `"all"`. Syncs between computers.
- `speech.caps`: default a higher pitch (`"pitch"`). Capitals. How a capital letter is marked when characters are spoken and typed. Choices: `"none"` (not marked), `"tone"` (a tone), `"pitch"` (a higher pitch), `"say_cap"` (say cap). Syncs between computers.
- `speech.split_caps`: default off (`false`). Split capitals. Say words joined with capitals, such as TextWeaver, as separate words. On or off: `true` or `false`. Syncs between computers.
- `speech.pause_heading_ms`: default 400 milliseconds. Pause after headings. Silence after a heading, shorter at faster rates. 0 turns it off. From 0 to 3000 milliseconds, in steps of 50. Syncs between computers.
- `speech.pause_paragraph_ms`: default 300 milliseconds. Pause after paragraphs. Silence after a paragraph, shorter at faster rates. 0 turns it off. From 0 to 3000 milliseconds, in steps of 50. Syncs between computers.
- `speech.pause_list_item_ms`: default 150 milliseconds. Pause after list items. Silence after a list item, shorter at faster rates. 0 turns it off. From 0 to 3000 milliseconds, in steps of 50. Syncs between computers.
- `speech.markup_pauses`: default on (`true`). Pauses written as markup. Read pause markup in a document, such as <break time="1s"/>, as a pause. Turn it off for documents that quote such markup. On or off: `true` or `false`. Syncs between computers.

## Speaking text

- `normalization.numbers`: default on (`true`). Numbers in words. Say numbers, dates, times, and money in words. On or off: `true` or `false`. Syncs between computers.
- `speech.skip_code`: default off (`false`). Skip code blocks. Do not speak code blocks. On or off: `true` or `false`. Syncs between computers.
- `normalization.table_mode`: default with rows and columns (`"structured"`). Tables. How tables are read. Choices: `"structured"` (with rows and columns), `"flat"` (as text), `"skip"` (skipped). Syncs between computers.
- `normalization.footnote_mode`: default where they are marked (`"inline"`). Footnotes. Where footnotes are read. Choices: `"inline"` (where they are marked), `"deferred"` (at the end), `"skip"` (skipped). Syncs between computers.
- `normalization.abbreviations`: default on (`true`). Expand abbreviations. Say abbreviations in full, such as Doctor for Dr. On or off: `true` or `false`. Syncs between computers.
- `normalization.abbrev_expansions`: default none. Your abbreviations. Abbreviations of your own and what they stand for. A table of names and values, edited in the file. Syncs between computers.
- `normalization.use_pronunciations`: default on (`true`). Use pronunciations. Apply your pronunciation list. On or off: `true` or `false`. Syncs between computers.
- `normalization.pronunciations`: default none. Pronunciations. Words and how to say them. A table of names and values, edited in the file. Syncs between computers, with the glossary.
- `normalization.community_lexicon.enabled`: default off (`false`). Community lexicon. Apply the community pronunciation dictionaries for engines other than Eloquence. On or off: `true` or `false`. Syncs between computers.
- `normalization.community_lexicon.language`: default US English (`"ENU"`). Community lexicon language. The dictionaries' language. Choices: `"ENU"` (US English), `"DEU"` (German). Other values may be written too. Syncs between computers.
- `normalization.community_lexicon.dir`: default not set. Community lexicon folder. The folder holding the dictionary files; not set looks beside textweaver. Text; empty means not set. Stays on this computer.
- `normalization.medical_lexicon.enabled`: default off (`false`). Medical lexicon. Read drug names, clinical terms and dosing abbreviations from a medical pronunciation list. On or off: `true` or `false`. Syncs between computers.
- `normalization.medical_lexicon.overlay`: default not set. Medical lexicon file. Your own medical pronunciations, which win over the built-in ones. Not set reads medical-lexicon.toml in the settings folder. Text; empty means not set. Stays on this computer.

## Accessibility and announcements

- `accessibility.mode`: default `"self-voicing"`. Accessibility mode. Self-voicing speaks everything; screen reader leaves speech to your screen reader; hybrid voices reading only. Choices: `"self-voicing"`, `"screen-reader"` (screen reader), `"hybrid"`. Stays on this computer.
- `accessibility.interface_announcements`: default automatic (`"auto"`). Interface announcements. How much textweaver says about itself: dialogs, progress, hints, and routine confirmations. Errors and answers to what you asked are always said. Automatic is minimal with a screen reader, normal when self-voicing. Choices: `"auto"` (automatic), `"off"`, `"minimal"`, `"normal"`, `"full"`. Syncs between computers.
- `accessibility.say_all`: default on the status line (`"screen"`). Say all with a screen reader. Continuous reading in screen-reader mode. A sentence at a time on the status line, or textweaver's voice. Choices: `"screen"` (on the status line), `"voice"` (with textweaver's voice). Stays on this computer.
- `accessibility.quiet_screen`: default automatic (`"auto"`). Quiet screen while reading. Keep the screen still while textweaver reads aloud. On by default in hybrid mode. Choices: `"auto"` (automatic), `true` (on), `false` (off). Stays on this computer.
- `accessibility.cursor`: default follows focus (`"follow"`). Cursor. Where the terminal's cursor waits. On what you are working on, or on the status line. Choices: `"follow"` (follows focus), `"status"` (on the status line). Stays on this computer.
- `gui.speak_messages`: default off (`false`). Speak textweaver's messages. When textweaver reads aloud, also say its messages, typing and cursor moves in its voice, for reading by ear without a screen reader. On or off: `true` or `false`. Stays on this computer.
- `gui.announce`: default live region (`"live"`). Announcements. How the window's messages reach the screen reader, from the next start. A live region, or UI Automation notifications (Windows only). Choices: `"live"` (live region), `"uia"` (UI Automation notifications). Stays on this computer.

## Reading

- `speech.auto_play`: default off (`false`). Read on opening. Start reading when a document opens. On or off: `true` or `false`. Syncs between computers.
- `reading.auto_resume`: default on (`true`). Resume where you left off. Go back to the saved position when a document opens. On or off: `true` or `false`. Syncs between computers.
- `reading.cursor_follows_speech`: default on (`true`). Cursor follows speech. The cursor moves with the word being read. On or off: `true` or `false`. Syncs between computers.
- `reading.wrap_navigation`: default off (`false`). Wrap navigation. Moving past the end of the document goes on from the start. On or off: `true` or `false`. Syncs between computers.
- `reading.nav_history_size`: default 50 places. Back history. How many places Back remembers. From 1 to 1000 places, in steps of 10. Syncs between computers.
- `reading.stop_at`: default never (`"off"`). Stop at section end. Where continuous reading stops by itself and says End of section. Never, at the next heading of any level, or at the next chapter: a section break, else a level 1 heading. Reading goes on from the heading with the read key. Choices: `"off"` (never), `"heading"` (next heading), `"chapter"` (next chapter). Syncs between computers.
- `reading.stop_after_minutes`: default 0 minutes. Reading timer. Continuous reading stops at a sentence end after this many minutes of reading. It says so. Pausing stops the clock; stopping starts it over. 0 turns the timer off. From 0 to 240 minutes, in steps of 5. Syncs between computers.
- `reading.after_end`: default `"stop"`. After a document ends. What follows the end of a document when the reading queue has another. Nothing, the next document named with the read key to read it, or the next document read at once. A document never starts unless you ask. Choices: `"stop"`, `"ask"` (ask first), `"next"` (read the next). Syncs between computers.
- `reading.recall_prompts`: default off (`false`). Recall prompts. At a section end, reading asks you to say what you remember. Reading stops at the next heading for this when Stop at section end is never. Reading goes on with the read key. On or off: `true` or `false`. Syncs between computers.

## Documents

- `reading.citations`: default skipped (`"off"`). Citations. Citations in continuous reading: skipped, or said in words. Choices: `"off"` (skipped), `"words"` (in words). Syncs between computers.
- `reading.revisions`: default automatic (`"auto"`). Tracked changes. How tracked changes in Word, OpenDocument, and RTF files are read. Said in place at high verbosity (automatic), always said, or never said, reading the final text. Applies when a document is opened. Choices: `"auto"` (automatic), `"marked"` (always say them), `"final"` (final text only). Syncs between computers.
- `reading.read_comments`: default off (`false`). Read Obsidian comments. Read the comments in Obsidian notes, written between %% marks. Off leaves them out, as Obsidian's reading view does. On or off: `true` or `false`. Syncs between computers.
- `reading.book_audio`: default recorded narration (`"auto"`). Book audio. What reads a DAISY talking book that has a recording. Its recorded narration, with the highlight following it and speech reading any text without audio, or speech only. Rate changes do not apply to the recording. Choices: `"auto"` (recorded narration), `"speech"` (speech only). Syncs between computers.
- `reading.ocr`: default on (`true`). Recognize scanned pages. Read the text of scanned PDFs and pictures by recognizing it (OCR). On or off: `true` or `false`. Syncs between computers.
- `reading.ocr_lang`: default the document's (`""`). Scanned text language. The language of scanned text, as Tesseract codes such as fra or deu+eng. Empty means the document's own language, else English. Choices: `""` (the document's), `"eng"` (English), `"fra"` (French), `"deu"` (German), `"spa"` (Spanish). Other values may be written too. Syncs between computers.
- `reading.ocr_engine`: default automatic (`"auto"`). OCR engine. Which engine recognizes scanned pages. ocrs for English and Tesseract for other languages, or one of them always. Choices: `"auto"` (automatic), `"ocrs"`, `"tesseract"` (Tesseract), `"paddle"` (PaddleOCR (experimental)). Stays on this computer.

## Highlight

- `highlight.enabled`: default on (`true`). Highlight spoken text. Highlight the word or sentence being read. On or off: `true` or `false`. Syncs between computers.
- `highlight.granularity`: default the word and the sentence (`"both"`). Highlight. What the reading highlight covers. Choices: `"word"` (the word), `"sentence"` (the sentence), `"both"` (the word and the sentence). Syncs between computers.
- `highlight.lead_words`: default 1 word. Highlight lead. Draw the highlight this many words ahead of the word heard (1 is the word heard). From -5 to 5 words, in steps of 1. Syncs between computers.
- `highlight.speed`: default 1. Highlight speed. Speed of the timed highlight for engines that report no words, as a multiple. 1 is normal speed. From 0.5 to 1.5, in steps of 0.1. Syncs between computers.
- `speech.latency_offset_ms`: default 120 milliseconds. Highlight delay. How long after an engine reports a word the highlight moves, for engines timed by their audio clock. From 0 to 1000 milliseconds, in steps of 10. Stays on this computer.
- `highlight.color`: default the theme's color (`"theme"`). Word highlight color. The color behind the word being read. Choose a name, or type a hex code. Default: the theme's color. Choices: `"theme"` (the theme's color), `"blue"`, `"orange"`, `"navy"` (dark blue), `"skyblue"` (sky blue), `"teal"`, `"gold"`, `"yellow"`, `"purple"`, `"pink"`, `"brown"`, `"gray"`, `"black"`, `"white"`. Other values may be written too. Syncs between computers.
- `highlight.sentence_color`: default not set. Sentence highlight color. The color behind the sentence being read. Choose a name, or type a hex code. Default: the theme's color. Choices: `"theme"` (the theme's color), `"blue"`, `"orange"`, `"navy"` (dark blue), `"skyblue"` (sky blue), `"teal"`, `"gold"`, `"yellow"`, `"purple"`, `"pink"`, `"brown"`, `"gray"`, `"black"`, `"white"`. Other values may be written too. Syncs between computers.
- `highlight.palette`: default 5 entries (`[a table of 3 entries, a table of 3 entries, a table of 3 entries, a table of 3 entries, a table of 3 entries]`). Highlight names. Up to eight names for your highlights, each with a color and a shape. The first five have keys of their own. A table of names and values, edited in the file. Syncs between computers.

## Math

- `normalization.math`: default on (`true`). Speak math. Speak math notation in words. On or off: `true` or `false`. Syncs between computers.
- `normalization.math_verbosity`: default `"normal"`. Math verbosity. How explicit spoken math is: low says a over b, normal and high say more. Choices: `"low"`, `"normal"`, `"high"`. Syncs between computers.
- `reading.math_engine`: default textweaver (`"builtin"`). Math speech. Which engine reads math aloud. textweaver's own, or MathCAT in ClearSpeak or SimpleSpeak, in the document's language. MathCAT needs a version that includes it; otherwise textweaver's own is used. Choices: `"builtin"` (textweaver), `"mathcat"` (MathCAT ClearSpeak), `"mathcat_simplespeak"` (MathCAT SimpleSpeak). Syncs between computers.
- `reading.math_display`: default `"source"`. Math on screen. How math looks in the reading view. As its source, such as x^2, or as Unicode, such as x with a superscript 2. Speech and edit mode always use the source. Choices: `"source"`, `"unicode"` (Unicode). Syncs between computers.
- `normalization.asciimath_delimiter`: default not set. ASCIIMath delimiter. The character around ASCIIMath, usually a backtick; not set reads no ASCIIMath. Text; empty means not set. Syncs between computers.

## Braille

- `braille.grade`: default uncontracted (grade 1) (`"uncontracted"`). Braille grade. Uncontracted (grade 1) or contracted (grade 2) braille, in BRF files and the Braille view. Contracted braille needs liblouis; without it, uncontracted braille is used. Choices: `"uncontracted"` (uncontracted (grade 1)), `"contracted"` (contracted (grade 2)). Syncs between computers.
- `braille.display_cells`: default 40 cells. Braille display cells. How many cells your Braille display has. The Braille view shows this many cells at a time. From 8 to 120 cells, in steps of 1. Stays on this computer.
- `braille.brf_code`: default UEB (`"ueb"`). Braille code of BRF files. The braille code BRF files are read in. UEB is for books made since 2016; EBAE, English Braille American Edition, is for older books. Reading a BRF file as print needs liblouis. Open the file again after a change. Choices: `"ueb"` (UEB), `"ebae"` (EBAE). Syncs between computers.
- `braille.table_format`: default `"linear"`. Braille tables. How BRF files lay out tables. Linear, one row per line with semicolons between entries; listed, each row a heading with each entry on its own line after its column heading; or stairstep, each entry two cells right of the one before, for tables of up to four columns. Choices: `"linear"`, `"listed"`, `"stairstep"`. Syncs between computers.
- `braille.math_code`: default Nemeth (`"nemeth"`). Math braille. The braille code for math in BRF files and while exploring a formula with MathCAT. Nemeth, or UEB mathematics. It needs a version that includes MathCAT; otherwise math is written as its spoken words. Choices: `"nemeth"` (Nemeth), `"ueb"` (UEB). Syncs between computers.
- `braille.viewer`: default off (`false`). Braille view. Show the current line or sentence as braille dots at the bottom of the app. Visual only: screen readers are not given its contents. On or off: `true` or `false`. Syncs between computers.
- `braille.viewer_print_line`: default off (`false`). Braille view print line. Under each braille cell, the print letter or contraction it stands for. On or off: `true` or `false`. Syncs between computers.
- `braille.viewer_cell_outlines`: default off (`false`). Braille view cell outlines. Draw an outline around each braille cell, so blank cells and spacing show. On or off: `true` or `false`. Syncs between computers.

## Morse code

- `speech.cw.wpm`: default 20 words per minute. CW character speed. How fast each Morse character is sent, in words per minute. While CW is the engine, the rate keys change it by one, and the effective speed with it. From 5 to 50 words per minute, in steps of 1. Syncs between computers.
- `speech.cw.effective_wpm`: default 10 words per minute. CW effective speed. The overall speed, set by stretching the gaps between characters and words (Farnsworth spacing). At or above the character speed, the spacing is standard. From 5 to 50 words per minute, in steps of 1. Syncs between computers.
- `speech.cw.tone_hz`: default 600 hertz. CW tone. The pitch of sent Morse code. The pitch keys move it by semitones. The sidetone heard while keying has its own pitch, CW sidetone pitch. From 200 to 1500 hertz, in steps of 10. Syncs between computers.
- `speech.cw.volume`: default 50 percent. CW tone level. How loud the Morse tone is, before the speech volume. From 0 to 100 percent, in steps of 5. Syncs between computers.
- `speech.cw.keyer`: default iambic B (`"iambic_b"`). CW keyer. How a paddle's contacts become dits and dahs when keying Morse code. Iambic B adds the opposite element when a squeeze is released during an element; iambic A does not; both remember a contact pressed during an element. Straight keys the tone from either contact. Choices: `"iambic_b"` (iambic B), `"iambic_a"` (iambic A), `"straight"` (straight key). Syncs between computers.
- `speech.cw.swap_paddles`: default off (`false`). Swap CW paddles. Swap dit and dah, for a paddle used with the other hand. MIDI and keyboard adapters both follow it. On or off: `true` or `false`. Syncs between computers.
- `speech.cw.sidetone`: default on (`true`). CW sidetone. Play a tone while the key is down when keying Morse code. Its pitch is CW sidetone pitch. On or off: `true` or `false`. Syncs between computers.
- `speech.cw.sidetone_hz`: default 600 hertz. CW sidetone pitch. The pitch of the tone heard while keying Morse code. It is apart from the CW tone of sent Morse, and a change is heard at once, even while keying. From 200 to 1500 hertz, in steps of 10. Syncs between computers.
- `speech.cw.dit_note`: default 20. CW dit MIDI note. The MIDI note of the dit contact. 20 for the N6ARA TinyMIDI and the CTR2 MIDI. From 0 to 127, in steps of 1. Stays on this computer.
- `speech.cw.dah_note`: default 21. CW dah MIDI note. The MIDI note of the dah contact. 21 for the N6ARA TinyMIDI and the CTR2 MIDI. From 0 to 127, in steps of 1. Stays on this computer.
- `speech.cw.straight_note`: default 30. CW straight key MIDI note. The MIDI note of a straight key. 30 for the CTR2 MIDI in extended paddle mode. From 0 to 127, in steps of 1. Stays on this computer.
- `speech.cw.midi_channel`: default 0. CW MIDI channel. The MIDI channel keying listens to, 1 to 16, or 0 for any. The CTR2 MIDI always uses channel 1. From 0 to 16, in steps of 1. Stays on this computer.

## Morse practice

- `speech.cw.practice_threshold`: default 90 percent. CW practice pass mark. The score in a Koch practice round, in percent, that adds the next character. The Koch method uses 90. From 50 to 100 percent, in steps of 5. Syncs between computers.
- `speech.cw.practice_group`: default 5 characters. CW practice group size. How many characters each practice group has. From 2 to 10 characters, in steps of 1. Syncs between computers.
- `speech.cw.practice_groups`: default 10 groups. CW practice round length. How many groups a practice round has. After the last, the round's score is said and kept. From 1 to 50 groups, in steps of 1. Syncs between computers.
- `speech.cw.practice_delay_ms`: default 1000 milliseconds. CW practice start delay. The quiet before each group is sent. It comes after the time textweaver's own spoken messages are expected to take. From 0 to 5000 milliseconds, in steps of 250. Syncs between computers.

## Display

- `display.theme`: default `"galaxy"`. Theme. The color theme. Any value may be written. Syncs between computers.
- `display.follow_os_theme`: default on (`true`). Follow the system theme. At startup, use a light, dark, or high-contrast theme like the system, unless you picked one. On or off: `true` or `false`. Syncs between computers.
- `display.show_line_numbers`: default off (`false`). Line numbers. Show line numbers. On or off: `true` or `false`. Syncs between computers.
- `display.measure`: default 66 characters. Line length. How many characters a line holds in the window, from 25 to 90. 0 fills the window. The terminal uses the wrap width. From 0 to 90 characters, in steps of 1. Syncs between computers.
- `display.wrap_width`: default 0 columns. Wrap width. Wrap lines at this many columns; 0 uses the whole width. From 0 to 400 columns, in steps of 10. Stays on this computer.
- `display.tab_width`: default 4 columns. Tab width. Columns a tab takes. From 1 to 16 columns, in steps of 1. Syncs between computers.
- `display.scroll_margin`: default 3 lines. Scroll margin. Lines kept in view above and below the cursor. From 0 to 20 lines, in steps of 1. Syncs between computers.
- `display.hints`: default automatic (`"auto"`). Key hints line. Whether the terminal reader shows key hints on its bottom line. Automatic shows them when self-voicing and hides them with a screen reader. F1 and the keyboard shortcuts list always name the keys. Choices: `"auto"` (automatic), `"on"`, `"off"`. Stays on this computer.
- `interface.rtl`: default automatic (`"auto"`). Right-to-left display. Whether the terminal reader reorders right-to-left text for display. Automatic leaves it to terminals that do it themselves. Speech and the screen reader always get the text in reading order. Choices: `"auto"` (automatic), `"on"`, `"off"`. Stays on this computer.

## Colors

- `colors.ruler`: default the theme's color (`"theme"`). Reading ruler color. The band of the reading ruler and the marked current line. The ruler keeps its underline or bold. Both the terminal reader and the textweaver app use it. Choose a name, or type a hex code. Default: the theme's color. Choices: `"theme"` (the theme's color), `"blue"`, `"orange"`, `"navy"` (dark blue), `"skyblue"` (sky blue), `"teal"`, `"gold"`, `"yellow"`, `"purple"`, `"pink"`, `"brown"`, `"gray"`, `"black"`, `"white"`. Other values may be written too. Syncs between computers.
- `colors.difficult_words`: default the theme's color (`"theme"`). Difficult words color. The underline of difficult words; they stay underlined and are named at high verbosity. Both the terminal reader and the textweaver app use it. Choose a name, or type a hex code. Default: the theme's color. Choices: `"theme"` (the theme's color), `"blue"`, `"orange"`, `"navy"` (dark blue), `"skyblue"` (sky blue), `"teal"`, `"gold"`, `"yellow"`, `"purple"`, `"pink"`, `"brown"`, `"gray"`, `"black"`, `"white"`. Other values may be written too. Syncs between computers.
- `colors.syllables`: default the theme's color (`"theme"`). Syllable marks color. The middle dots between syllables. Both the terminal reader and the textweaver app use it. Choose a name, or type a hex code. Default: the theme's color. Choices: `"theme"` (the theme's color), `"blue"`, `"orange"`, `"navy"` (dark blue), `"skyblue"` (sky blue), `"teal"`, `"gold"`, `"yellow"`, `"purple"`, `"pink"`, `"brown"`, `"gray"`, `"black"`, `"white"`. Other values may be written too. Syncs between computers.
- `colors.misspellings`: default the theme's color (`"theme"`). Misspellings color. The underline of misspelled words, which are also said. Not used yet: the window draws the theme's color. Choose a name, or type a hex code. Default: the theme's color. Choices: `"theme"` (the theme's color), `"blue"`, `"orange"`, `"navy"` (dark blue), `"skyblue"` (sky blue), `"teal"`, `"gold"`, `"yellow"`, `"purple"`, `"pink"`, `"brown"`, `"gray"`, `"black"`, `"white"`. Other values may be written too. Syncs between computers.
- `colors.lint`: default the theme's color (`"theme"`). Lint marks color. The underline of Markdown lint and grammar problems, which are also said. Not used yet: the window draws the theme's color. Choose a name, or type a hex code. Default: the theme's color. Choices: `"theme"` (the theme's color), `"blue"`, `"orange"`, `"navy"` (dark blue), `"skyblue"` (sky blue), `"teal"`, `"gold"`, `"yellow"`, `"purple"`, `"pink"`, `"brown"`, `"gray"`, `"black"`, `"white"`. Other values may be written too. Syncs between computers.
- `colors.find_match`: default the theme's color (`"theme"`). Search match color. The band behind search matches; they stay underlined. Choose a name, or type a hex code. Default: the theme's color. Choices: `"theme"` (the theme's color), `"blue"`, `"orange"`, `"navy"` (dark blue), `"skyblue"` (sky blue), `"teal"`, `"gold"`, `"yellow"`, `"purple"`, `"pink"`, `"brown"`, `"gray"`, `"black"`, `"white"`. Other values may be written too. Syncs between computers.
- `colors.selection`: default the theme's color (`"theme"`). Selection color. The band behind selected text. Choose a name, or type a hex code. Default: the theme's color. Choices: `"theme"` (the theme's color), `"blue"`, `"orange"`, `"navy"` (dark blue), `"skyblue"` (sky blue), `"teal"`, `"gold"`, `"yellow"`, `"purple"`, `"pink"`, `"brown"`, `"gray"`, `"black"`, `"white"`. Other values may be written too. Syncs between computers.
- `colors.focus`: default the theme's color (`"theme"`). Focus color. The focus outline and the focused item of a list; they stay bold. Choose a name, or type a hex code. Default: the theme's color. Choices: `"theme"` (the theme's color), `"blue"`, `"orange"`, `"navy"` (dark blue), `"skyblue"` (sky blue), `"teal"`, `"gold"`, `"yellow"`, `"purple"`, `"pink"`, `"brown"`, `"gray"`, `"black"`, `"white"`. Other values may be written too. Syncs between computers.
- `colors.links`: default the theme's color (`"theme"`). Link color. The color of links. Links stay underlined. Choose a name, or type a hex code. Default: the theme's color. Choices: `"theme"` (the theme's color), `"blue"`, `"orange"`, `"navy"` (dark blue), `"skyblue"` (sky blue), `"teal"`, `"gold"`, `"yellow"`, `"purple"`, `"pink"`, `"brown"`, `"gray"`, `"black"`, `"white"`. Other values may be written too. Syncs between computers.
- `colors.headings`: default the theme's color (`"theme"`). Heading color. The color of headings. Headings stay bold. Choose a name, or type a hex code. Default: the theme's color. Choices: `"theme"` (the theme's color), `"blue"`, `"orange"`, `"navy"` (dark blue), `"skyblue"` (sky blue), `"teal"`, `"gold"`, `"yellow"`, `"purple"`, `"pink"`, `"brown"`, `"gray"`, `"black"`, `"white"`. Other values may be written too. Syncs between computers.
- `colors.status_bar`: default the theme's color (`"theme"`). Status bar color. The band of the status and title bars. Choose a name, or type a hex code. Default: the theme's color. Choices: `"theme"` (the theme's color), `"blue"`, `"orange"`, `"navy"` (dark blue), `"skyblue"` (sky blue), `"teal"`, `"gold"`, `"yellow"`, `"purple"`, `"pink"`, `"brown"`, `"gray"`, `"black"`, `"white"`. Other values may be written too. Syncs between computers.
- `colors.notes`: default the theme's color (`"theme"`). Note color. The band behind text with a note; it stays italic and underlined. Choose a name, or type a hex code. Default: the theme's color. Choices: `"theme"` (the theme's color), `"blue"`, `"orange"`, `"navy"` (dark blue), `"skyblue"` (sky blue), `"teal"`, `"gold"`, `"yellow"`, `"purple"`, `"pink"`, `"brown"`, `"gray"`, `"black"`, `"white"`. Other values may be written too. Syncs between computers.
- `colors.bookmarks`: default the theme's color (`"theme"`). Bookmark color. The band behind a bookmarked word; it stays bold and underlined. Choose a name, or type a hex code. Default: the theme's color. Choices: `"theme"` (the theme's color), `"blue"`, `"orange"`, `"navy"` (dark blue), `"skyblue"` (sky blue), `"teal"`, `"gold"`, `"yellow"`, `"purple"`, `"pink"`, `"brown"`, `"gray"`, `"black"`, `"white"`. Other values may be written too. Syncs between computers.

## Spacing, font and ruler

- `reading_aids.spacing.line_height`: default 1.5. Line height. Line height as a multiple of the font size. 1.5 is the WCAG value. From 1 to 3, in steps of 0.1. Syncs between computers.
- `reading_aids.spacing.paragraph_spacing`: default 1. Paragraph spacing. Space after each paragraph, in multiples of the font size. From 0 to 4, in steps of 0.25. Syncs between computers.
- `reading_aids.spacing.letter_spacing`: default 0. Letter spacing. Extra space between letters, in multiples of the font size. From 0 to 0.5, in steps of 0.02. Syncs between computers.
- `reading_aids.spacing.word_spacing`: default 0. Word spacing. Extra space between words, in multiples of the font size. From 0 to 1, in steps of 0.04. Syncs between computers.
- `reading_aids.font.family`: default sans serif (`"sans"`). Font. The window's reading font. You can also type the name of any installed font. Choices: `"system-ui"` (the system font), `"sans"` (sans serif), `"serif"`, `"monospace"`, `"atkinson"` (Atkinson Hyperlegible), `"opendyslexic"` (OpenDyslexic), `"lexend"` (Lexend). Other values may be written too. Syncs between computers.
- `reading_aids.font.size_pt`: default 14 points. Font size. The window's font size. From 6 to 72 points, in steps of 1. Syncs between computers.
- `reading_aids.font.weight`: default 400. Font weight. 400 is regular, 700 bold. From 100 to 900, in steps of 100. Syncs between computers.
- `reading_aids.ruler.mode`: default `"off"`. Reading ruler. Mark the current line, or a band of lines. Choices: `"off"`, `"current_line"` (current line), `"ruler"`. Syncs between computers.
- `reading_aids.ruler.scope`: default the whole line (`"line"`). Ruler covers. A wrapped row, or the whole line. Choices: `"row"` (a row), `"line"` (the whole line). Syncs between computers.
- `reading_aids.ruler.rows_above`: default 1 row. Ruler rows above. Rows of the band above the current one. From 0 to 10 rows, in steps of 1. Syncs between computers.
- `reading_aids.ruler.rows_below`: default 1 row. Ruler rows below. Rows of the band below the current one. From 0 to 10 rows, in steps of 1. Syncs between computers.
- `reading_aids.ruler.mask_outside`: default off (`false`). Ruler mask. Dim the rows outside the band. On or off: `true` or `false`. Syncs between computers.

## Word aids

- `reading_aids.bionic`: default off (`false`). Bionic reading. Draw the start of each word in bold. On or off: `true` or `false`. Syncs between computers.
- `reading_aids.bionic_options.ratio`: default 0.4. Bionic share. How much of each word is bold. From 0.1 to 0.9, in steps of 0.1. Syncs between computers.
- `reading_aids.bionic_options.min_word_len`: default 2 letters. Bionic shortest word. Words shorter than this are left alone. From 1 to 20 letters, in steps of 1. Syncs between computers.
- `reading_aids.bionic_options.skip_numbers`: default on (`true`). Bionic skips numbers. Leave words with digits alone. On or off: `true` or `false`. Syncs between computers.
- `reading_aids.bionic_options.skip_urls`: default on (`true`). Bionic skips addresses. Leave web and email addresses alone. On or off: `true` or `false`. Syncs between computers.
- `reading_aids.bionic_options.skip_code`: default on (`true`). Bionic skips code. Leave code alone. On or off: `true` or `false`. Syncs between computers.
- `reading_aids.syllables`: default off (`false`). Syllables. Draw words split into syllables with a middle dot; speech is unchanged. On or off: `true` or `false`. Syncs between computers.
- `reading_aids.syllable_options.separator`: default `"·"`. Syllable separator. What is drawn between syllables. Text. Syncs between computers.
- `reading_aids.syllable_options.left_min`: default 2 letters. Syllable first break. Fewest letters before the first break. From 1 to 10 letters, in steps of 1. Syncs between computers.
- `reading_aids.syllable_options.right_min`: default 2 letters. Syllable last break. Fewest letters after the last break. From 1 to 10 letters, in steps of 1. Syncs between computers.
- `reading_aids.syllable_options.min_word_len`: default 4 letters. Syllable shortest word. Words shorter than this are never split. From 1 to 20 letters, in steps of 1. Syncs between computers.
- `reading_aids.syllable_options.skip_urls`: default on (`true`). Syllables skip addresses. Leave web and email addresses alone. On or off: `true` or `false`. Syncs between computers.
- `reading_aids.syllable_options.skip_code`: default on (`true`). Syllables skip code. Leave code alone. On or off: `true` or `false`. Syncs between computers.
- `reading_aids.difficult_words`: default off (`false`). Difficult words. Underline rare words, and name them on word moves at high verbosity. On or off: `true` or `false`. Syncs between computers.
- `reading_aids.difficult_definitions`: default off (`false`). Difficult word definitions. With difficult words marked, at high verbosity also say a difficult word's first definition from the dictionary. On or off: `true` or `false`. Syncs between computers.

## Word by word (RSVP)

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
- `reading_aids.rsvp.position`: default top center (`"top-center"`). RSVP position. Where the RSVP word appears. Choices: `"top-left"` (top left), `"top-center"` (top center), `"top-right"` (top right), `"center-left"` (middle left), `"center"`, `"center-right"` (middle right), `"bottom-left"` (bottom left), `"bottom-center"` (bottom center), `"bottom-right"` (bottom right). Syncs between computers.
- `reading_aids.rsvp.font_size_pt`: default 48 points. RSVP size. Size of the RSVP word in the window. From 8 to 200 points, in steps of 2. Syncs between computers.
- `reading_aids.rsvp.lead_words`: default 0 words. RSVP lead. With speech pacing, show this many words ahead of the word spoken. From -5 to 5 words, in steps of 1. Syncs between computers.

## Editing

- `editing.echo_characters`: default on (`true`). Echo characters. Say each character typed. On or off: `true` or `false`. Syncs between computers.
- `editing.echo_words`: default on (`true`). Echo words. Say each word typed. On or off: `true` or `false`. Syncs between computers.
- `editing.echo_deletions`: default on (`true`). Echo deletions. Say what Backspace and Delete remove. On or off: `true` or `false`. Syncs between computers.
- `editing.echo_lines_on_move`: default on (`true`). Echo lines. Say the line when the cursor moves to another line. On or off: `true` or `false`. Syncs between computers.
- `editing.undo_steps`: default 1000 steps. Undo steps. Most undo steps kept while editing. From 1 to 100000 steps, in steps of 100. Stays on this computer.
- `editing.undo_memory_mb`: default 50 megabytes. Undo memory. Most memory the undo history may use. From 1 to 4096 megabytes, in steps of 16. Stays on this computer.
- `editing.autosave_recovery`: default on (`true`). Recovery snapshots. Keep a copy of unsaved work and offer it after a crash. On or off: `true` or `false`. Syncs between computers.
- `editing.autosave_interval_secs`: default 20 seconds. Snapshot interval. Seconds between recovery snapshots while there are unsaved changes. From 5 to 3600 seconds, in steps of 5. Syncs between computers.
- `authoring.author`: default empty (`""`). Author. The name textweaver writes on comments, replies, and documents from a template. Empty means textweaver on comments and no author in templates. Never taken from the computer. Text. Stays on this computer.
- `authoring.track_changes`: default off (`false`). Track changes in Word files. Save edits to a Word file as tracked changes a reviewer can accept. Off saves them as Markdown under a new name. On or off: `true` or `false`. Syncs between computers.
- `preview.pane`: default off (`false`). Preview pane. In edit mode, show the document as it reads beside the editor. F6 moves to it; the cursor stays in the editor. On or off: `true` or `false`. Syncs between computers.
- `preview.follow`: default nothing (`"off"`). Browser preview follows. When the browser preview reloads itself: never, after each save, or also when typing pauses. With never, press F5 in the browser after a save. It reloads through a small server on this computer only. Choices: `"off"` (nothing), `"save"` (each save), `"typing"` (your typing). Syncs between computers.
- `preview.pane_delay_ms`: default 300 milliseconds. Preview pause. How long typing must pause before a preview that follows your typing is rewritten. From 100 to 3000 milliseconds, in steps of 50. Syncs between computers.

## Library and study tools

- `library.folders`: default an empty list. Library folders. Folders whose documents the library lists, and whose positions sync between computers. Separate folders with semicolons. A list of texts, such as `["a", "b"]`. Stays on this computer.
- `library.recent_limit`: default 20 files. Recent files. How many recent files are remembered. From 1 to 500 files, in steps of 5. Syncs between computers.
- `stats.enabled`: default on (`true`). Reading statistics. Count the time read aloud, the furthest point, and sessions for each document. On or off: `true` or `false`. Syncs between computers.
- `summary.sentences`: default 5 sentences. Summary sentences. How many sentences Summarize and tw summarize give, 1 to 50. From 1 to 50 sentences, in steps of 1. Syncs between computers.
- `lexicon.glossary`: default not set. Glossary. Your own glossary, looked up before the dictionary: term: definition lines, or star's JSON. Not set uses glossary.txt in the settings folder. Text; empty means not set. Stays on this computer.
- `lexicon.data_file`: default not set. Dictionary file. The define-word dictionary, lexicon-en.twlex. Not set looks beside the program. Text; empty means not set. Stays on this computer.

## Dictation

- `dictation.model`: default base.en, the default (`"whisper-base.en"`). Dictation model. The Whisper model dictation uses when no folder is set. Download the dictation model, in the Tools menu, gets it. Choices: `"whisper-base.en"` (base.en, the default), `"whisper-small.en"` (small.en, larger and more accurate). Other values may be written too. Stays on this computer.
- `dictation.model_dir`: default not set. Dictation model folder. A folder holding a Whisper model for dictation. Not set uses the dictation model below, in the data folder. Text; empty means not set. Stays on this computer.
- `dictation.speak_while_recording`: default off (`false`). Speak while dictating. Say dictated words as they come. Off, they are shown on the status line and said at each pause, so the microphone does not hear the voice. On or off: `true` or `false`. Syncs between computers.

## Keyboard and language

- `interface.language`: default English (`"en"`). Interface language. The language of textweaver's own words, changed at once. The voice follows it when the engine has one for it; otherwise the voice stays. Choices: `"en"` (English), `"es"` (Español), `"fr"` (Français), `"de"` (Deutsch), `"pt"` (Português), `"ar"` (العربية). Other values may be written too. Syncs between computers.
- `keyboard.preset`: default screen reader style (`"default"`). Keys. The default keys: like NVDA's and JAWS's browse mode, or textweaver's earlier keys. Used from the next start. Choices: `"default"` (screen reader style), `"classic"`. Stays on this computer.
- `keyboard.character_keys`: default on (`true`). Single-key shortcuts. Browse keys such as h and period. Off, dictation and typing never trigger commands. On or off: `true` or `false`. Syncs between computers.
- `keyboard.digit_row`: default automatic (`"auto"`). Digit row. How the terminal recognizes the digit keys for heading levels. Auto, or a French AZERTY keyboard. Choices: `"auto"` (automatic), `"azerty"` (AZERTY). Stays on this computer.

## Toolbars and panels

- `gui.header`: default on (`true`). Show the header. Show the bar of Open, Font, Settings and Commands above the document. Off, the commands keep their keys and menu items. On or off: `true` or `false`. Stays on this computer.
- `gui.header_buttons`: default open, choose_font, settings, command_palette (`["open", "choose_font", "settings", "command_palette"]`). Header buttons. The header's buttons, in order, as command ids separated by commas. Customize buttons in the View menu changes them one at a time. Ids this version does not know are kept but not shown. A list of texts, such as `["a", "b"]`. Stays on this computer.
- `gui.toolbar`: default on (`true`). Show the toolbar. Show the bar of Play, Stop, the reading buttons and Edit. Off, the commands keep their keys and menu items. On or off: `true` or `false`. Stays on this computer.
- `gui.toolbar_buttons`: default play_pause, stop, previous_paragraph, next_paragraph, toggle_edit_mode, rate_down, rate_up (`["play_pause", "stop", "previous_paragraph", "next_paragraph", "toggle_edit_mode", "rate_down", "rate_up"]`). Toolbar buttons. The toolbar's buttons, in order, as command ids separated by commas. The terminal reader's key hints follow them. Customize buttons in the View menu changes them one at a time. A list of texts, such as `["a", "b"]`. Stays on this computer.
- `gui.auto_hide_menu`: default on (`true`). Hide the menu bar. Windows: hide the window's menu bar until Alt or F10 shows it. It hides again when the menu closes. No effect on Linux, whose menus are the F10 list, or on macOS. On or off: `true` or `false`. Stays on this computer.
- `gui.focus_mode`: default off (`false`). Focus mode. Hide the bars and the panel when the mouse rests a few seconds. The text then has the whole window. Moving the mouse shows them; keys do not. Screen readers still find every control. On or off: `true` or `false`. Stays on this computer.
- `gui.sidebar`: default none (`"off"`). Panel beside the document. The panel the textweaver app shows beside the document. None, the Contents (the headings), or the Notes. The panel keys change it, and the window remembers the last one. Choices: `"off"` (none), `"contents"` (Contents), `"notes"` (Notes). Stays on this computer.

## Export

- `export.audio_format`: default FLAC (`"flac"`). Audio export format. The format Export audio lists first. tw export-audio also uses it for a file name with no extension. Choices: `"flac"` (FLAC), `"mp3"` (MP3), `"opus"` (Opus), `"ogg"` (Ogg Vorbis), `"wav"` (WAV), `"m4b"` (M4B audiobook), `"mp4"` (MP4 video with captions). Syncs between computers.
- `export.subtitle_format`: default SubRip (`"srt"`). Subtitle format. The format of subtitles written without a file name. Choices: `"srt"` (SubRip), `"vtt"` (WebVTT), `"ass"` (ASS karaoke). Syncs between computers.
- `export.subtitle_karaoke`: default Off (`"off"`). Subtitle karaoke. How subtitle lines show the word being read. Off, underlined as it is spoken (WebVTT tags), or one cue per word in bold and underline. Choices: `"off"` (Off), `"tags"` (Underline as spoken), `"lines"` (One cue per word). Syncs between computers.
- `export.subtitle_chapters`: default off (`false`). Chapters file. Also write a WebVTT chapters file beside the subtitles or the audio. On or off: `true` or `false`. Syncs between computers.
- `export.subtitle_word_level`: default off (`false`). Word subtitles. One subtitle per word instead of caption lines. On or off: `true` or `false`. Syncs between computers.
- `export.subtitles_with_audio`: default off (`false`). Subtitles with audio. Always write subtitles beside exported audio. On or off: `true` or `false`. Syncs between computers.

## Live documents

- `live.sources`: default none. Live sources. MQTT connections opened as live documents. Edit in settings.toml; passwords stay in the credential store. A table of names and values, edited in the file. Stays on this computer.
- `live.announce_new_entries`: default on (`true`). Announce new live entries. Send each new entry, whole, to the screen reader and Braille display. On or off: `true` or `false`. Syncs between computers.
- `live.speak_new_entries`: default off (`false`). Speak new live entries. textweaver's own voice reads each new entry. Off by default. On or off: `true` or `false`. Syncs between computers.
- `live.min_interval_seconds`: default 10 seconds. Live value interval. In the values view, the fewest seconds between announcements of one field. From 0 to 3600 seconds, in steps of 5. Syncs between computers.
- `live.quiet_hint_seconds`: default 60 seconds. Quiet source hint. Seconds without messages before a hint in words. 0 never hints. From 0 to 3600 seconds, in steps of 10. Syncs between computers.
- `live.log_folder`: default empty (`""`). Live log folder. Where closed live logs are saved as JSON. Empty uses logs in the data folder. Text; empty means not set. Stays on this computer.

## Sync

- `sync.enabled`: default off (`false`). Sync. Sync notes, highlights, bookmarks, and places with your other computers through the sync folder. Tools, Sync, Set up sync turns it on. On or off: `true` or `false`. Stays on this computer.
- `sync.folder`: default not set. Sync folder. The folder your computers share. One kept in step by Syncthing, a cloud folder, or a USB stick. Text; empty means not set. Stays on this computer.
- `sync.device_name`: default empty (`""`). Computer name. This computer's name in sync messages, such as laptop or lab. Empty uses Computer 1, Computer 2, and so on. Text. Stays on this computer.
- `sync.position_policy`: default the newest (`"newest"`). Place to resume. Which place a document opens at when another computer has one too. The newest, the furthest, or ask. Choices: `"newest"` (the newest), `"furthest"` (the furthest), `"ask"`. Stays on this computer.
- `sync.places`: default on (`true`). Sync places. Share where you are in each document, and the reading queue. On or off: `true` or `false`. Stays on this computer.
- `sync.notes`: default on (`true`). Sync notes. Share notes. On or off: `true` or `false`. Stays on this computer.
- `sync.highlights`: default on (`true`). Sync highlights. Share highlights. On or off: `true` or `false`. Stays on this computer.
- `sync.bookmarks`: default on (`true`). Sync bookmarks. Share bookmarks. On or off: `true` or `false`. Stays on this computer.
- `sync.statistics`: default on (`true`). Sync statistics. Share each computer's reading time and sessions. On or off: `true` or `false`. Stays on this computer.
- `sync.settings`: default on (`true`). Sync settings. Share the portable settings: rate, punctuation, theme, reading aids, and the like. The voice, the engine, the access mode, the key preset, and paths stay on each computer. On or off: `true` or `false`. Stays on this computer.
- `sync.profiles`: default on (`true`). Sync profiles. Share your profiles; which one is in use stays on each computer. On or off: `true` or `false`. Stays on this computer.
- `sync.key_overrides`: default on (`true`). Sync key overrides. Share keymap.toml. A Mac's keys are kept but not used on Windows or Linux, and the reverse. On or off: `true` or `false`. Stays on this computer.
- `sync.words`: default on (`true`). Sync word list. Share your spelling word list. On or off: `true` or `false`. Stays on this computer.
- `sync.glossary`: default on (`true`). Sync glossary. Share your glossary's entries and your pronunciations. On or off: `true` or `false`. Stays on this computer.
- `sync.favorite_voices`: default on (`true`). Sync favorite voices. Share your favorite voices. One this computer does not have is listed as not on this computer. On or off: `true` or `false`. Stays on this computer.

## Updates and components

- `updates.check`: default off (`false`). Check for updates. Look for a newer textweaver once a day at start. It reads the public list of releases on GitHub and asks before downloading. Nothing about you is sent. Help, Check for updates checks any time. On or off: `true` or `false`. Stays on this computer.
- `components.source`: default empty (`""`). Components source. Your own components, used first. A GitHub repository as owner/name, or a folder on this computer. Empty uses none. Never put a password here. A private repository signs in with the GitHub CLI, or asks once for a token kept in the system credential store. Text. Stays on this computer.
- `components.mirror`: default empty (`""`). Components mirror. Where optional components come from first: an https address or a folder on this computer. Empty uses their public sources. Never put a password here. Text. Stays on this computer.

## Kept by textweaver

             textweaver writes these itself, such as a question already asked.              They are in the file, but not on the settings screen.

- `display.theme_explicit`: default off (`false`). Theme picked. Set when you pick a theme; it stops following the system. On or off: `true` or `false`. Syncs between computers.
- `accessibility.hybrid_offered`: default off (`false`). Hybrid mode offered. Set once textweaver has asked whether to use hybrid mode. On or off: `true` or `false`. Stays on this computer.
- `interface.recent_settings`: default an empty list. Recently changed settings. The settings changed last on the settings screen, listed at its top. Text; empty means not set. Stays on this computer.
- `gui.window`: default not set. Window place and size. Where the window was and how large, kept on this computer and never synced. Text; empty means not set. Stays on this computer.
- `gui.last_version`: default not set. Version last run. The version of the textweaver app last run on this computer, so it says Updated to once after an update. Never synced. Text; empty means not set. Stays on this computer.
- `components.chooser_shown`: default off (`false`). Components list shown. The first-run list of optional components was shown. On or off: `true` or `false`. Stays on this computer.
- `updates.asked`: default off (`false`). Updates question asked. The first-run question about updates was asked. On or off: `true` or `false`. Stays on this computer.
- `updates.last_check`: default 0. Last update check. When the last automatic check for updates ran, in seconds since 1970. Text; empty means not set. Stays on this computer.
- `updates.declined`: default empty (`""`). Declined update. The release you said no to; only a newer one is offered. Text; empty means not set. Stays on this computer.

## See also

         - [Settings](settings.md): where settings live, and export, import, and reset.
         - [JSON-RPC](json-rpc.md): `settings_schema`, `get_setting`, and `set_setting`.
         - [Documentation index](README.md)
