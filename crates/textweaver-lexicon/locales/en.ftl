### textweaver's interface messages, in English.
###
### The format is Fluent (https://projectfluent.org/), in the subset that
### textweaver-lexicon reads: messages, terms, comments, variables, and
### one-line select variants. Every message the code asks for must be here;
### a test in textweaver-lexicon checks that.
###
### Write for the ear: every message is spoken as well as shown. Spell out
### symbols, keep sentences short, and end sentences with a full stop.

-brand = textweaver

## Parts of speech.

pos-noun = noun
pos-verb = verb
pos-adjective = adjective
pos-adverb = adverb

## Define word: sources.

source-glossary = your glossary
source-wordnet = Open English WordNet
source-cmudict = the CMU Pronouncing Dictionary

## Define word: the list of senses.

# $word is the word looked up, $n the number of senses, $source a source-* message.
define-title =
    { $n ->
        [0] Pronunciation of { $word }, from { $source }
        [one] Definitions of { $word }, 1 sense, from { $source }
       *[other] Definitions of { $word }, { $n } senses, from { $source }
    }
# $lemma is the headword, $pos a pos-* message, $i the sense number, $n how many.
define-sense-head = { $lemma }, { $pos }, { $i } of { $n }
define-sense-head-nopos = { $lemma }, { $i } of { $n }
define-sense = { $head }: { $definition }.
define-example = For example: { $text }.
define-synonyms = Synonyms: { $words }.
define-antonyms = Opposite: { $words }.
define-kind-of = A kind of: { $words }.
# $say is a respelling such as RUN-ing, with the stressed syllable in capitals.
define-pronounced = Pronounced { $say }.
define-pronounced-or = Pronounced { $say }, or { $other }.

## Prompts.

prompt-define-word = Define which word?
prompt-profile-name = Name for the new profile
prompt-profile-rename = New name for the profile, Enter keeps it
prompt-profiles-import = Import profiles from file
prompt-profiles-export = Export profiles to file, for example textweaver-profiles.json

## Common words.

common-cancelled = Cancelled.
# Durations: $h hours, $m minutes, $s seconds.
duration-hours =
    { $h ->
        [one] 1 hour
       *[other] { $h } hours
    } and { $m ->
        [one] 1 minute
       *[other] { $m } minutes
    }
duration-minutes =
    { $m ->
        [one] 1 minute
       *[other] { $m } minutes
    } and { $s ->
        [one] 1 second
       *[other] { $s } seconds
    }
duration-seconds =
    { $s ->
        [one] 1 second
       *[other] { $s } seconds
    }

## Define word in the reader.

# $title is define-title.
define-intro = { $title }. Up and Down move through the senses, Enter copies one, Escape closes.
define-nothing-here = There is no word at the cursor.
define-not-found = No definition found for { $word }.
define-no-dictionary = The dictionary file is not installed, so only your glossary was searched. The reading guide says how to install it.
define-dictionary-damaged = The dictionary file could not be read: { $error }
define-glossary-problem = Your glossary could not be read: { $error }
define-glossary-skipped =
    { $n ->
        [one] 1 line of your glossary has no definition and was skipped.
       *[other] { $n } lines of your glossary have no definition and were skipped.
    }
define-copied = Copied.

## Settings profiles.

profiles-title =
    { $n ->
        [0] Settings profiles, none saved yet
        [one] Settings profiles, 1 profile
       *[other] Settings profiles, { $n } profiles
    }
# $title is profiles-title.
profiles-intro = { $title }. Enter switches to a profile, F2 renames it, Delete deletes it.
# $summary is profile-summary-* parts joined by commas.
profiles-item = { $name }: { $summary }
profiles-item-active = { $name }, in use: { $summary }
profiles-save-new = Save the current settings as a new profile
profiles-update = Save the current settings into { $name }
profiles-import = Import profiles from a file
profiles-export = Export all profiles to a file
profile-summary-voice = voice { $voice }
profile-summary-rate = rate { $rate }
profile-summary-theme = theme { $theme }
# $mode is self voicing, hybrid, or screen reader.
profile-summary-access = { $mode } mode
profile-summary-empty = nothing saved
profile-switched = Switched to { $name }.
profile-switched-backend = Switched to { $name }. Its speech engine is used from the next start.
# $keys lists settings such as speech.pitch.
profile-dropped =
    { $n ->
        [one] 1 setting in it is not used by this version: { $keys }.
       *[other] { $n } settings in it are not used by this version: { $keys }.
    }
profile-saved = Saved the current settings as { $name }.
profile-replaced = Saved the current settings into { $name }.
profile-renamed = Renamed { $old } to { $new }.
profile-delete-question = Delete the profile { $name }? y or n
profile-deleted = Deleted { $name }.
profile-kept = Kept.
profile-not-found = There is no profile named { $name }.
profile-needs-name = A profile needs a name.
profile-exists = A profile named { $name } already exists.
profiles-not-an-export = { $detail }
profiles-no-persistence = Profiles are not saved in this session.
profiles-read-failed = The profiles file could not be read, so it is treated as empty: { $error }
profiles-save-failed = Could not save the profiles: { $error }
profiles-none-to-export = There are no profiles to export yet.
profiles-exported =
    { $n ->
        [one] Exported 1 profile to { $file }.
       *[other] Exported { $n } profiles to { $file }.
    }
profiles-export-failed = Could not export the profiles: { $error }
profiles-imported =
    { $n ->
        [0] There were no profiles in { $file }.
        [one] Imported 1 profile from { $file }: { $names }.
       *[other] Imported { $n } profiles from { $file }: { $names }.
    }

## Reading statistics.

stats-title = Reading statistics
stats-intro = Reading statistics. Enter on a document opens it.
stats-off = Reading statistics are off. The last item turns them on.
stats-empty = No reading recorded yet. Time is counted while textweaver reads aloud.
# $time is a duration-* message.
stats-total =
    { $time } read in all, in { $sessions ->
        [one] 1 session
       *[other] { $sessions } sessions
    }, over { $docs ->
        [one] 1 document
       *[other] { $docs } documents
    }.
stats-current =
    This document: { $time } read, furthest point { $pct } percent, { $sessions ->
        [one] 1 session
       *[other] { $sessions } sessions
    }.
stats-current-none = This document has not been read aloud yet.
stats-most-read = Most read { $rank }: { $title }, { $time }, furthest point { $pct } percent.
stats-toggle-on = Statistics are on. Enter turns them off.
stats-toggle-off = Statistics are off. Enter turns them on.
stats-turned-on = Reading statistics are on.
stats-turned-off = Reading statistics are off. What was recorded is kept; tw stats --clear removes it.

## Lists.

study-nothing-to-delete = Nothing to delete in this list.
study-nothing-to-rename = Nothing to rename in this list.
## tw stats.

stats-clear-question =
    { $n ->
        [one] Remove the reading statistics of 1 document? y or n
       *[other] Remove the reading statistics of { $n } documents? y or n
    }
stats-cleared = Reading statistics removed.
stats-off-cli = Reading statistics are off: stats.enabled is false in the settings.

## Navigation. $dir is next or previous; $what is a kind-* or unit-*
## noun and $unit its key (heading, list-item, sentence), for languages
## whose words agree with the noun.

nav-blank = blank
# High verbosity: $label is a structure label ("Heading level 2").
nav-message-at-labelled = { $label }, line { $line }, { $pct } percent: { $content }
nav-message-at = Line { $line }, { $pct } percent: { $content }
nav-message-labelled = { $label }: { $content }
# $message is the navigation message after wrapping around.
nav-wrapped = Wrapped. { $message }
nav-no-unit =
    { $dir ->
        [next] No next { $what }.
       *[previous] No previous { $what }.
    }
nav-nothing-to-read = No { $what } to read.
nav-label-heading-level = Heading level { $level }
nav-label-list =
    { $n ->
        [0] List
        [one] List, 1 item
       *[other] List, { $n } items
    }
nav-label-table =
    { $n ->
        [0] Table
        [one] Table, 1 row
       *[other] Table, { $n } rows
    }
nav-label-list-item-level = List item, level { $level }
nav-no-heading-level =
    { $dir ->
        [next] No next heading at level { $level }.
       *[previous] No previous heading at level { $level }.
    }
# Said before a line's text on caret moves.
nav-line-heading = heading level { $level }
nav-line-row = row { $n }
nav-line-list-item-level = list item, level { $level }
# $language is a programming language name such as Python.
nav-line-code-language = code, { $language }
nav-no-chapters = This document has no chapters.
nav-chapter = Chapter
nav-no-chapter =
    { $dir ->
        [next] No next chapter.
       *[previous] No previous chapter.
    }
nav-back = Back
nav-forward = Forward
nav-page = Page
nav-percent = { $pct } percent
nav-no-earlier-history = No earlier history.
nav-no-forward-history = No forward history.
# $label is nav-back, nav-forward, nav-page, or nav-percent.
nav-label-line = { $label }, line { $line }
nav-top-of-document = Top of document
nav-end-of-document = End of document
# $edge is nav-top-of-document or nav-end-of-document; $content the line there.
nav-edge-message = { $edge }. { $content }
nav-top-of-document-stop = Top of document.
nav-end-of-document-stop = End of document.
nav-position = Line { $line } of { $lines }, { $pct } percent.
nav-position-word = Word { $word } of { $words }.
nav-position-heading = Under heading { $heading }.
nav-position-mode = { $mode } mode.

## Names of structure, units, and modes (said inside other messages).

kind-heading = heading
kind-paragraph = paragraph
kind-list-item = list item
kind-list = list
kind-table = table
kind-row = row
kind-cell = cell
kind-link = link
kind-graphic = graphic
kind-code = code
kind-quote = block quote
kind-page = page
kind-section = section
kind-bold = bold
kind-italic = italic
kind-underline = underline
kind-footnote = footnote
kind-strikethrough = strikethrough
kind-separator = separator
kind-math = math
unit-character = character
unit-word = word
unit-sentence = sentence
unit-line = line
unit-paragraph = paragraph
unit-document = document
# $what is a kind-* noun, $unit its key.
unit-with-level = { $what } level { $level }
mode-browse = Browse
mode-speech-cursor = Speech Cursor
mode-edit = Edit
mode-find = Find
mode-command = Command
mode-go-to = Go to
mode-open = Open
mode-prompt = Prompt
common-on = on
common-off = off

## Reading aloud.

playback-caps-no-words = This voice does not report words, so the word highlight is estimated.
playback-caps-words = This voice reports each word, so the highlight follows it exactly.
playback-caps-no-pitch = Pitch cannot be changed with this voice.
playback-caps-no-volume = Volume cannot be changed with this voice.
# The reading state on the title line, one word.
state-reading = Reading
state-paused = Paused
state-stopped = Stopped
state-ready = Ready
playback-reading-at = Reading at { $rate } words per minute.
playback-paused = Paused.
playback-stopped-speech-cursor-off = Stopped. Speech Cursor off.
playback-stopped = Stopped.
playback-search-cleared = Search cleared.
# $key is the key that turns edit mode off.
playback-still-editing = Still editing. { $key } finishes.
playback-end-of-document-content = end of document
playback-no-unit-here = No { $what } here.
playback-no-selection = No selection.
# $next is what happens now (a restart, or nothing).
playback-speech-died = Speech stopped working ({ $reason }). { $next }
playback-done-reading = Done reading.
playback-speech-restarted = Speech restarted: { $reason }. Reading on from the last word.
playback-speech-error = Speech error: { $error }

## The title line and Say Status.

# The title line's position; % is shown, not said.
status-position = line { $line } of { $lines }, { $pct }%
status-mode = { $mode } mode
status-modified = modified
status-self-voicing = self-voicing
status-hybrid = hybrid
status-screen-reader = screen reader mode
status-rate-spoken = { $wpm } words per minute
status-rate = { $wpm } wpm
status-no-document = No document
# $parts are the title line's parts, joined with commas.
status-said = { $title }: { $parts }.
status-no-message = No message yet.
# A list shown without its own introduction.
status-list-intro =
    { $n ->
        [one] { $title }, 1 item. Up and Down move, Enter chooses, Escape closes.
       *[other] { $title }, { $n } items. Up and Down move, Enter chooses, Escape closes.
    }

## Questions and answers. Keep the letters y and n: they are the keys
## that answer.

common-press-y-or-n = Press y or n.
common-kept = Kept.
confirm-quit = Quit textweaver? y or n
confirm-delete-note = Delete this note or highlight? y or n
notes-remove-highlight-question = Remove this highlight? y or n
notes-delete-note-question = Delete this note? y or n
list-nothing-to-mark = Nothing to mark in this list.
notes-editing = Editing note: { $text }
# $key opens a document.
app-no-document-open = No document is open. Press { $key } to open one.
settings-save-failed = Could not save settings: { $error }
edit-still-editing = Still editing.
goto-not-a-target = Not a go-to target: { $text }. Type a line number, a percentage such as 50%, start, or end.

## Opening a document.

open-opened = Opened { $title }.
open-resumed = Opened { $title }. Resumed at { $pct } percent.
open-resumed-synced = Opened { $title }. Resumed at { $pct } percent, from another device.
open-resumed-conflict = Opened { $title }. Resumed at { $pct } percent. Another device is at a different place; kept this device's.

## Prompts: the label is shown and said when the prompt opens.

prompt-find = Find
prompt-go-to = Go to line, percent, start, or end
prompt-open = Open file
prompt-command = Command
# $label is prompt-command.
prompt-command-palette-intro = { $label }. Type part of a name; Tab completes, Up and Down list matches.
prompt-save-as = Save as
prompt-table-size = Table size, columns by rows, for example 3 by 2
prompt-image-path = Image file
prompt-replace-find = Replace, find what
prompt-replace-with = Replace with
prompt-note = Note
prompt-edit-note = Edit note, Enter keeps it
prompt-rename-bookmark = New bookmark name, Enter keeps it
prompt-export-settings = Export settings to file, for example textweaver-settings.json
prompt-import-settings = Import settings from file
prompt-citation-locator = Page or other locator, for example 12 or chapter 2; Enter for none
prompt-reference-identifier = DOI or ISBN to add
prompt-import-references = Import references from file
prompt-template-title = Title of the new document
prompt-setting-value = New value, Enter keeps it

## Keys named in messages and the help.

help-the-command-palette = the command palette
help-not-bound = not bound
# Two keys, or a key and a list of keys: "p or Ctrl+P".
help-or = { $a } or { $b }
# A command without keys: $name is its palette name, such as list highlights.
help-the-command = the command { $name }
# One line of the keyboard shortcuts list: a category-* title, an action-*
# help, and its keys.
help-entry = { $category }: { $help }. { $keys }
# One command palette candidate: its id (not translated), help, and keys.
help-palette-item = { $id }: { $help }. { $keys }
help-unknown-command = Unknown command: { $text }.
help-shortcuts-intro = Keyboard shortcuts, { $n } commands. Up and Down move, Enter runs, Escape closes.
help-shortcuts-title = Keyboard shortcuts
help-title = Help
help-intro = Help. Up and Down move, Escape closes.

## The help list. Each value is a key or keys from the keymap.

help-about = textweaver reads documents aloud. Keys below are the current bindings.
help-open = Open a document: { $open }. Library and recent files: { $library }.
help-play = Play or pause: { $key }.
help-read-from-cursor = Read from the cursor: { $key }.
help-stop = Stop: { $key }.
help-sentences = Next and previous sentence: { $next } and { $previous }.
help-paragraphs = Next and previous paragraph: { $next } and { $previous }.
help-headings = Next and previous heading: { $next } and { $previous }. Heading at a level: { $first } to { $last }, with Shift for the previous one.
help-read-headings = Read from the next and previous heading: { $next } and { $previous }.
help-quick-keys = Quick keys, as in NVDA and JAWS: list { $list }, list item { $item }, table { $table }, link { $link }, block quote { $quote }, separator { $separator }, graphic { $graphic }, section { $section }. Shift with the key goes to the previous one.
help-speech-cursor = Speech Cursor, line by line: { $key }.
help-find = Find: { $key }.
help-bookmark = Add a bookmark: { $key }.
help-history = Back and forward through your jumps: { $back } and { $forward }.
help-rate = Faster and slower: { $faster } and { $slower }.
help-where = Where am I: { $key }.
help-repeat = Hear the last message again: { $repeat }. The last message and the status: mode, rate, engine, and position: { $status }.
help-notes = Notes: add { $add }, list { $list }, next and previous { $next } and { $previous }, delete the one at the cursor { $delete }. In the list, Delete deletes and F2 edits.
help-highlights = Highlight the selection or sentence, or remove a highlight: { $highlight }. List highlights: { $list }.
help-bookmarks-list = Bookmarks list: Delete deletes a bookmark, F2 renames it.
help-edit = Edit the document: { $edit }. Save: { $save }. Save as: { $saveas }. New document: { $new }.
help-editing = While editing: undo { $undo }, redo { $redo }, bold { $bold }. Every formatting command is in the keyboard shortcuts.
help-outline = Outline of the headings, type to filter: { $outline }. Follow a link or footnote: { $follow }.
help-tables = Tables: { $nextrow } and { $previousrow } move by row, { $nextcell } and { $previouscell } by cell.
help-citations = Citations while editing: insert { $insert }, add a reference by DOI or ISBN { $reference }. Spelling: next and previous misspelling { $next } and { $previous }, suggestions { $suggestions }.
help-export = Export to HTML, PDF, Word, EPUB, or braille, preview in the browser, and start from a template: type export, preview, or template in the command palette.
help-verbosity = How much is said: { $verbosity }. How much punctuation: { $punctuation }.
help-voice = Choose a voice: { $voice }. Restart speech if it stops: { $restart }.
help-access = With a screen reader, who speaks: { $key } cycles self-voicing, hybrid, and screen reader mode.
help-character-keys = Single-key shortcuts on or off, for dictation: { $keys }. Settings: { $settings }.
help-all-shortcuts = All keyboard shortcuts: { $key }.
help-palette = Run any command by name: { $key }.
# Keep the letters y, n, and a: they are the keys that answer.
help-quit = Quit, saving your place: { $key }, then y to confirm; n, a, or Escape cancels.

## Help categories.

category-reading = Reading
category-navigation = Navigation
category-speech-cursor = Speech Cursor
category-voice = Voice
category-search = Search
category-bookmarks = Bookmarks and notes
category-file = File
category-editing = Editing
category-view = View and help

## Key names as textweaver's own voice says them. Written names (Ctrl+S)
## are not translated.

keyname-control = Control
keyname-command = Command
keyname-alt = Alt
keyname-shift = Shift
keyname-period = period
keyname-comma = comma
keyname-semicolon = semicolon
keyname-colon = colon
keyname-apostrophe = apostrophe
keyname-quote = quote
keyname-grave-accent = grave accent
keyname-tilde = tilde
keyname-exclamation-mark = exclamation mark
keyname-question-mark = question mark
keyname-at-sign = at sign
keyname-number-sign = number sign
keyname-dollar-sign = dollar sign
keyname-percent = percent
keyname-caret = caret
keyname-ampersand = ampersand
keyname-asterisk = asterisk
keyname-left-parenthesis = left parenthesis
keyname-right-parenthesis = right parenthesis
keyname-left-bracket = left bracket
keyname-right-bracket = right bracket
keyname-left-brace = left brace
keyname-right-brace = right brace
keyname-less-than = less than
keyname-greater-than = greater than
keyname-plus = plus
keyname-minus = minus
keyname-equals = equals
keyname-underscore = underscore
keyname-slash = slash
keyname-backslash = backslash
keyname-vertical-bar = vertical bar
keyname-page-up = Page Up
keyname-page-down = Page Down
keyname-up-arrow = Up Arrow
keyname-down-arrow = Down Arrow
keyname-left-arrow = Left Arrow
keyname-right-arrow = Right Arrow
keyname-escape = Escape
keyname-space = Space
keyname-enter = Enter
keyname-tab = Tab
keyname-backspace = Backspace
keyname-delete = Delete
keyname-insert = Insert
keyname-home = Home
keyname-end = End

## Commands: the one-line help of each, in the keyboard shortcuts list and
## the command palette. Their ids (play_pause) stay as they are.

action-play-pause = Play or pause reading from the current word
action-stop = Stop reading
action-read-from-cursor = Read continuously from the cursor
action-read-document = Read the whole document from the start
action-read-current-character = Say the character at the cursor
action-read-current-word = Say the word at the cursor
action-read-current-sentence = Say the sentence at the cursor without moving
action-read-current-line = Say the line at the cursor
action-read-paragraph = Say the paragraph at the cursor without moving
action-read-selection = Read the selected text
action-say-position = Say the position: line, percentage, word number, and heading
action-say-status = Say the last message again, then the status: mode, reading state, position, rate, and speech engine; in a list, the list's introduction
action-repeat-message = Say the last message again
action-word-count = Say how many words are in the document, or in the selection
action-link-address = Say the address of the link at the cursor
action-replay-sentence = Read again from the start of the current sentence
action-replay-paragraph = Read again from the start of the current paragraph
action-rsvp-toggle = Show or hide RSVP: one word at a time, from the cursor
action-rsvp-play-pause = Start or pause RSVP
action-rsvp-faster = RSVP faster
action-rsvp-slower = RSVP slower
action-rsvp-position-next = Move the RSVP word to the next place on the screen
action-reading-level = Say the reading level of the document or the selection
action-define-word = Define the word at the cursor, or the selected words: senses, examples, synonyms, and pronunciation
action-toggle-citations = Turn citations on or off in continuous reading: off skips them, on says them in words
action-explore-math = Explore the math at the cursor term by term: arrows move, Down goes into a part, Up comes out, Escape leaves
action-listen-rendered = Listen to the document as it will render, without leaving edit mode
action-next-sentence = Move to the next sentence
action-previous-sentence = Move to the previous sentence, or to the start of this one when more than three words in
action-next-paragraph = Move to the next paragraph
action-previous-paragraph = Move to the previous paragraph
action-next-heading = Read from the next heading
action-previous-heading = Read from the previous heading
action-skip-next-heading = Move to the next heading without reading
action-skip-previous-heading = Move to the previous heading without reading
action-outline = List the headings: type to filter, Enter jumps to one
action-next-heading-level-1 = Move to the next heading at level 1
action-next-heading-level-2 = Move to the next heading at level 2
action-next-heading-level-3 = Move to the next heading at level 3
action-next-heading-level-4 = Move to the next heading at level 4
action-next-heading-level-5 = Move to the next heading at level 5
action-next-heading-level-6 = Move to the next heading at level 6
action-previous-heading-level-1 = Move to the previous heading at level 1
action-previous-heading-level-2 = Move to the previous heading at level 2
action-previous-heading-level-3 = Move to the previous heading at level 3
action-previous-heading-level-4 = Move to the previous heading at level 4
action-previous-heading-level-5 = Move to the previous heading at level 5
action-previous-heading-level-6 = Move to the previous heading at level 6
action-next-table = Move to the next table
action-previous-table = Move to the previous table
action-next-list = Move to the next list
action-previous-list = Move to the previous list
action-next-list-item = Move to the next list item
action-previous-list-item = Move to the previous list item
action-next-link = Move to the next link
action-previous-link = Move to the previous link
action-next-block-quote = Move to the next block quote
action-previous-block-quote = Move to the previous block quote
action-next-separator = Move to the next separator (horizontal rule)
action-previous-separator = Move to the previous separator (horizontal rule)
action-next-graphic = Move to the next graphic (image)
action-previous-graphic = Move to the previous graphic (image)
action-follow-link = Follow the link at the cursor, or go between a footnote and its note
action-table-next-row = In a table, move down a row in the same column
action-table-previous-row = In a table, move up a row in the same column
action-table-next-column = In a table, move to the next cell in the row
action-table-previous-column = In a table, move to the previous cell in the row
action-next-chapter = Move to the next chapter or section
action-previous-chapter = Move to the previous chapter or section
action-history-back = Go back to where you were before the last jump
action-history-forward = Go forward again after going back
action-go-to = Go to a line, percentage, or position
action-document-start = Move to the start of the document
action-document-end = Move to the end of the document
action-caret-next-word = Move the cursor to the next word
action-caret-previous-word = Move the cursor to the previous word
action-caret-next-line = Move the cursor to the next line
action-caret-previous-line = Move the cursor to the previous line
action-select-next-word = Extend the selection to the next word
action-select-previous-word = Extend the selection to the previous word
action-select-next-line = Extend the selection to the next line
action-select-previous-line = Extend the selection to the previous line
action-page-down = Move down one screen
action-page-up = Move up one screen
action-scroll-down = Scroll down one line without moving the cursor
action-scroll-up = Scroll up one line without moving the cursor
action-speech-cursor-toggle = Enter or leave Speech Cursor (line) mode
action-speech-cursor-next-line = Speech Cursor: read the next line
action-speech-cursor-previous-line = Speech Cursor: read the previous line
action-speech-cursor-reread-line = Speech Cursor: read the current line again
action-speech-cursor-exit-and-read = Speech Cursor: leave and read on from this line
action-rate-up = Speak faster
action-rate-down = Speak slower
action-pitch-up = Raise the pitch
action-pitch-down = Lower the pitch
action-volume-up = Louder
action-volume-down = Quieter
action-cycle-speed-preset = Cycle the speed presets (skim, normal, study, slow)
action-choose-voice = Choose a voice
action-restart-speech = Restart speech with the current settings (after the speech engine stopped working)
action-cycle-verbosity = Cycle how much textweaver says: low, normal, high
action-cycle-punctuation = Cycle how much punctuation is spoken: none, some, all
action-find = Find text in the document
action-find-next = Find the next match
action-find-previous = Find the previous match
action-next-misspelling = Move to the next misspelled word, and spell it
action-previous-misspelling = Move to the previous misspelled word, and spell it
action-spelling-suggestions = List suggestions for the misspelled word at the cursor, or add it to your word list
action-next-grammar-problem = Move to the next grammar problem, and say it and its fix
action-previous-grammar-problem = Move to the previous grammar problem, and say it and its fix
action-next-lint-problem = In edit mode, move to the next Markdown lint problem, and say it
action-previous-lint-problem = In edit mode, move to the previous Markdown lint problem, and say it
action-add-bookmark = Add a bookmark at the cursor
action-list-bookmarks = List bookmarks
action-next-bookmark = Move to the next bookmark
action-previous-bookmark = Move to the previous bookmark
action-add-note = Add a note to the selection or the sentence at the cursor
action-list-notes = List notes
action-next-note = Move to the next note
action-previous-note = Move to the previous note
action-delete-note = Delete the note or highlight at the cursor
action-highlight-selection = Highlight the selection, or the sentence at the cursor
action-export-study-sheet = Export the notes and highlights as a Markdown study sheet, grouped by heading
action-open = Open a document
action-open-library = Open the library: documents in your library folders and recent files
action-new-document = Start a new document in edit mode
action-save = Save (Markdown and text in place; other formats as Markdown)
action-save-as = Save under a new name
action-export-settings = Export settings and key overrides to a JSON or TOML file
action-import-settings = Import settings from a JSON or TOML file, after a yes or no
action-reading-statistics = List reading statistics: time read, the furthest point, sessions, and the most read documents
action-new-from-template = Start a new document from a template, with a title, author, date, and References heading
action-export-html = Export the document as a web page (HTML) next to it
action-export-pdf = Export the document as a tagged PDF next to it
action-export-docx = Export the document as a Word file (DOCX) next to it
action-export-epub = Export the document as an EPUB book next to it
action-export-brf = Export the document as braille (BRF) next to it
action-preview-in-browser = Preview the document in the web browser, with math; each save rewrites the preview
action-toggle-preview-auto-reload = Turn automatic reloading of the browser preview on or off
action-toggle-preview-live = Turn live preview on or off: with automatic reloading, the preview also reloads when typing pauses
action-quit = Quit, saving the reading position
action-toggle-edit-mode = Switch between reading and editing
action-undo = Undo
action-redo = Redo
action-bold = Make the selection bold
action-italic = Make the selection italic
action-underline = Underline the selection
action-strikethrough = Strike through the selection
action-inline-code = Mark the selection as code
action-code-block = Make the selected lines a code block
action-insert-link = Make the selection a link
action-heading = Make the current line a heading
action-bullet-list = Make the selected lines a bulleted list
action-numbered-list = Make the selected lines a numbered list
action-block-quote = Make the selected lines a block quote
action-horizontal-rule = Insert a horizontal rule
action-insert-table = Insert a table
action-add-table-row = Add a row to the table at the cursor
action-insert-image = Insert an image
action-replace = Find and replace
action-copy = Copy the selection, or the sentence at the cursor, to the clipboard
action-cut = Cut the selection to the clipboard
action-next-table-cell = In a table, move to the next cell and say its column; elsewhere, type a tab
action-previous-table-cell = In a table, move to the previous cell and say its column
action-cycle-typing-echo = Cycle typing echo: characters and words, characters, words, or none
action-select-all = Select all the text
action-delete-word-before = Delete the word before the cursor
action-delete-word-after = Delete the word after the cursor
action-paste = Paste the text last copied or cut in textweaver; the terminal paste works too
action-insert-citation = Insert a citation: pick a reference, then give a page or other locator
action-add-reference = Add a reference to your library by DOI or ISBN
action-insert-bibliography = Insert the bibliography of the works cited, at the cursor
action-check-citations = Check the citations: how many there are, and which keys are not in your library
action-import-references = Import references from a BibTeX, RIS, or CSL-JSON file into your library
action-next-theme = Switch to the next color theme
action-toggle-line-numbers = Show or hide line numbers
action-toggle-character-keys = Turn single-key shortcuts on or off, so dictation and typing never trigger commands
action-cycle-access-mode = Cycle the accessibility mode: self-voicing, hybrid, or screen reader
action-settings-profiles = List settings profiles: switch to one, save the current settings as one, rename, delete, import, or export
action-bionic-toggle = Turn bionic reading on or off: the start of each word in bold
action-ruler-cycle = Cycle the reading ruler: off, current line, ruler
action-syllables-toggle = Show or hide syllables: words split with a middle dot
action-difficult-words-toggle = Mark difficult words on or off: underlined, and named on word moves at high verbosity
action-command-palette = Run any command by name
action-settings = Open the settings: every option with its help, filtered as you type; Left and Right change a value
action-keyboard-help = List keyboard shortcuts
action-help = Open the help
