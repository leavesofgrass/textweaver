# Writing messages

How to write the words textweaver says and shows: status messages, questions, errors, labels, setting helps, and button descriptions. The rules come from the messages that already work best, read aloud and on a 40-cell Braille display.

Every message lives in the message catalog, `crates/textweaver-lexicon/locales/`, with the same id in all six languages (`en.ftl`, `ar.ftl`, `de.ftl`, `es.ftl`, `fr.ftl`, `pt.ftl`). English is the source. A setting's label and help also live in `INFO` in `crates/textweaver-app/src/settings_schema.rs`, and a command's help in `crates/textweaver-keymap/src/action.rs`; tests keep each pair equal, so change both, then run `cargo xtask regen` for the generated references.

The style tests are in `crates/textweaver-lexicon/src/i18n/style_tests.rs`. Where English still breaks a rule, the test lists the ids that do; those lists may only shrink. When you fix a message, take its id off the list. A new message that breaks a rule fails the test, which names the rule's number below.

## The rules

### 1. Meaning first

The first words say what happened or what this is. A fact that must survive a Braille line sits in cells 1 to 40.

- Good: "Opened essay.md.", "Bookmark 2 set at 40 percent.", "3 of 12".
- Not: a category or a count before the result ("Reading: Play or pause ..."), or a full path before the reason.
- Grammar comes with it: no verb stacked on another ("Table row added removed."). Tested: `english_catalog_has_no_stacked_verbs`.

### 2. Errors: what failed, why, what to do

The shape is "Could not VERB OBJECT: CAUSE NEXT STEP." The cause is often the system's words, `{ $error }`, which stay English in every language. Put no period right after `{ $error }`, and end with a sentence that says what to do or what still works.

- Good: "Could not save: { $error } Still editing. Try Save as."
- Good: "Could not scan the library: { $error } Check the library folders in Settings."
- Not: "Could not save the profiles: { $error }" (nothing to do next).
- Tested: `errors_get_a_next_step`. Its list `NO_NEXT_STEP_YET` names the errors still waiting for a next step.

### 3. Questions: the question first, naming what yes does, then "y or n"

- Good: "Quit textweaver? y or n", "Download the Lexend font, 206 KB, SIL Open Font License? y or n".
- Lower case "y or n", never "Y or N". No period after it, unless more text follows ("Exported essay.html. Open it? y or n. Format HTML, in Documents.").
- Name the object: "Use the notes from Ada's laptop? y or n", not "Use them?". When a stray key asks again, ask the whole question again.
- A question that changes or removes something asks before it acts (Reset all colors asks first).
- In the window, the dialog's name is the question without "y or n", because its Yes and No buttons show the keys. The status line and the terminal keep "y or n".
- Tested: `questions_end_with_y_or_n`.

### 4. Casing

Sentences start with a capital. Labels, names, and choices are in sentence case ("Reading ruler"), and a choice after a label stays lower case ("Rate: automatic"). Proper names keep their case: Speech Cursor, Piper, Eloquence. A piece written to follow a lead-in may start lower case ("Could not open x: it is not a readable file").

### 5. Punctuation

A sentence ends with a period. Labels, prompts, names, choices, and list items do not. In a status-like message, prefer two short sentences to one joined with a semicolon. No em dashes, quotation marks inside messages, arrows, or symbols a speech engine skips or spells out.

### 6. Numbers and units

Digits for numbers. Text that is spoken spells its units: "3 percent", "265 words per minute", "contrast 6.1 to 1". Short forms ("%", "wpm", "KB") belong only on a title line or status bar that saves cells, and their spoken twin uses words.

- Units agree with the number: "1 word", "2 words". A setting's unit is a plural select on `$n` (`settings-unit-*`).
- Colors: say "Choose a name, or type a hex code"; a screen reader reads "#rrggbb" as "number sign r r g g b b". Only the two messages that teach the typed form show it. Tested: `hex_color_codes_appear_only_where_typing_is_taught`.

### 7. One word for each thing

- **document**: what you read. **file**: what is on disk.
- **cursor**, never "caret", in messages. (The window guide says "caret" for the editing insertion point, which is NVDA's word.)
- **speech engine**, never "backend". **voice**: one named voice.
- **window**, never "GUI". **terminal reader**: the other program.
- **version**, never "build" ("Citations are not in this version of textweaver.").
- **theme**: the whole look. **colors**: single parts.
- **note**, **highlight**, **bookmark**: three things.
- A command is named by its name in the menus and the palette: "The Voices command lists them."
- US spelling: color, center, license, favorite. `cargo xtask docs --check` checks the English catalog.
- One sentence, one id: never put the same sentence under two ids; reuse the id that has it.
- Tested: `english_catalog_uses_one_word_for_each_thing` and `no_new_duplicate_sentences`.

### 8. Keys come from the keymap

Name a key with `{ $key }`, filled from the keymap, so a user's own keys and the classic preset are named right: the status line shows "Ctrl+S" and textweaver's voice says "Control S". Hard-code only keys a list or dialog owns: Enter, Escape, Delete, Up, and Down. Letters are lower case: "y or n". The keymap test catches hard-coded modifier chords; bare keys such as F2 are not caught yet, so check them by hand.

### 9. Positions

"k of n" first for an item ("3 of 12, Chapter 2"); "line 12 of 400, 3 percent" for text; "Page 12 of 30".

### 10. Counts and empties

Use real plural selects ("1 note", "2 notes"), never "(s)". Zero says "No notes." or "Nothing to read after the cursor."

### 11. The 40-cell rule

The key fact is in cells 1 to 40. A prompt label plus eight typed letters fits. A menu name fits. A question comes before what it is about. A status-like result fits whole, or its first 40 cells already carry the outcome. Count Braille cells, not characters: capitals and digits take extra cells in uncontracted braille, and `textweaver_tui::ui::braille_cells` counts them.

- The window's button descriptions (`gui-hint-*`) fit 40 cells and say what the name does not; Stop, Slower, Faster, and the sentence buttons have none, since their names say it. The full help stays in F1, Shift+F1, and the palette. Tested: `button_hints_are_short_in_every_language`, and in cells by `window_button_hints_fit_a_braille_line` in the terminal's tests.
- A setting's help starts with one sentence of at most fifteen words. The window shows that sentence under the form and gives it as the row's description; F1 says the whole help, and the settings reference prints it. Tested: `setting_helps_start_with_a_short_sentence`, whose list `LONG_FIRST_SENTENCE` may only shrink.
- A button's accessible name is its label without the key in parentheses; the key is its keyboard shortcut. A lone punctuation key is never shown as the key: Commands shows "(F2)", not "(:)".

### 12. What the tests check, and what they do not

Checked:

- Every id the code uses is in `en.ftl`, every id in `en.ftl` is used, and every translation has exactly English's ids, only English's variables, and plural keys its language uses (`crates/textweaver-lexicon/src/i18n/tests.rs`).
- The pseudo-locale run fails on any English left outside the catalog (`crates/textweaver-app/tests/it/pseudo_locale.rs`).
- Settings labels and helps equal `INFO`; command helps equal the keymap's.
- The style rules above, on the English catalog (`style_tests.rs`).
- 40 cells: about thirty terminal lines, every prompt and menu name in six languages, some sync and font messages, and the window's button descriptions.

Not checked yet, so a reviewer reads for them: meaning first, casing, and units in spoken text.

## Translations

Write English first. Then add the same id to the five other catalogs, in the same place. If you cannot translate, copy the English and say so in your pull request; a maintainer finds a translator. Translations go through a native-speaker review before a release.

## See also

- [CONTRIBUTING.md](../../CONTRIBUTING.md): the accessibility expectations every change meets.
- [Using textweaver with a screen reader](../screen-readers.md): how messages reach the screen reader and the Braille display.
- [ADR-0025: Lexicon and message catalog](../adr/0025-lexicon-and-message-catalog.md) and [ADR-0030: Interface translations](../adr/0030-interface-translations.md): how the catalog works and what stays English.
- [Testing](testing.md).
- [Documentation index](../README.md).
