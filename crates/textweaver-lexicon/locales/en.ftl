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

common-cancelled = Canceled.
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
define-dictionary-damaged = The dictionary file could not be read: { $error } Only your glossary is searched.
define-glossary-problem = Your glossary could not be read: { $error } Only the dictionary is searched.
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
profiles-read-failed = The profiles file could not be read, so it is treated as empty: { $error } Saving a profile replaces the file.
profiles-save-failed = Could not save the profiles: { $error } Check that the settings folder can be written to.
profiles-none-to-export = There are no profiles to export yet.
profiles-exported =
    { $n ->
        [one] Exported 1 profile to { $file }.
       *[other] Exported { $n } profiles to { $file }.
    }
profiles-export-failed = Could not export the profiles: { $error } Check that the folder can be written to.
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
stats-turned-off = Reading statistics are off. What was recorded is kept.

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
stats-clear-item = Remove the reading statistics
stats-clear-failed = Could not remove the reading statistics: { $error }. Check that the data folder can be written.
stats-off-cli = Reading statistics are off: stats.enabled is false in the settings.

## Continue reading and every computer's statistics (the sync wave, S6).

continue-title = Continue reading
# $n is the number of documents listed.
continue-intro =
    { $n ->
        [one] Continue reading: 1 document, newest first.
       *[other] Continue reading: { $n } documents, newest first.
    }
continue-empty = No places yet. Reading saves your place.
# One row, meaning first: the title, how far in, the computer, and how
# long ago (continue-ago-*).
continue-item = { $title }, { $pct } percent, { $device }, { $when }
continue-this-computer = this computer
continue-ago-now = just now
continue-ago-minutes =
    { $n ->
        [one] 1 minute ago
       *[other] { $n } minutes ago
    }
continue-ago-hours =
    { $n ->
        [one] 1 hour ago
       *[other] { $n } hours ago
    }
continue-ago-days =
    { $n ->
        [one] 1 day ago
       *[other] { $n } days ago
    }
name-continue-reading = Continue reading
action-continue-reading = Continue reading: the documents on this computer with a saved place, from any computer, newest first
name-add-library-folder = Add a folder to the library
action-add-library-folder = Add a folder to the library: choose it in the file browser
name-edit-document-details = Edit details
action-edit-document-details = Edit the document's details: title, author, DOI, and ISBN
prompt-document-details = Document details

## Edit a document's details by hand.

# $name is the document's title; said when the form opens.
details-intro = Details of { $name }. Tab moves, Enter saves, Escape cancels.
details-label-title = Title
details-label-author = Author
details-label-doi = DOI
details-label-isbn = ISBN
# A field's drawn label: its label, then $n of $total fields.
details-prompt-label = { $label }, { $n } of { $total }
# Said on moving to a field: its label and value (or nav-blank).
details-field = { $label }: { $value }
# $fields lists the fields saved, by their labels.
details-saved = Details saved: { $fields }.
details-unchanged = Details not changed.
details-cancelled = Canceled. Details not changed.
# $text is what was typed in the DOI or ISBN field.
details-not-a-doi = Not a DOI: { $text }. Fix it or clear it.
details-not-an-isbn = Not an ISBN: { $text }. Fix it or clear it.
details-no-file = This document has no file, so no details to edit.
details-save-failed = Could not save the details: { $error } Try again.
# The statistics list could not wait for the sync folder.
stats-others-slow = Other computers skipped: folder slow.
stats-untitled = Untitled document
# One computer's share of a document: $device, $time, $sessions.
stats-computer =
    { $sessions ->
        [one] { $device }: { $time }, 1 session
       *[other] { $device }: { $time }, { $sessions } sessions
    }
stats-by-computer-off = Each computer: hidden. Enter shows it.
stats-by-computer-on = Each computer: shown. Enter hides it.

## Navigation. $dir is next or previous; $what is a kind-* or unit-*
## noun and $unit its key (heading, list-item, sentence), for languages
## whose words agree with the noun.

nav-blank = blank
# Said for a picture that has no description (no alternative text).
nav-no-description = no description
# The document overview: the title first, then the counts. $time is overview-time.
overview-line = { $title }. Headings: { $headings }, tables: { $tables }, pictures: { $pictures }, footnotes: { $footnotes }. { $time }
# Reading time left from the cursor at the current rate.
overview-time =
    { $minutes ->
        [0] Less than a minute left.
        [one] About 1 minute left.
       *[other] About { $minutes } minutes left.
    }
# Said when the reading pass changes and as reading starts in a skim. $pass is a reading-pass-* name.
reading-pass-changed = Pass: { $pass }.
reading-pass-full = full text
reading-pass-first-sentences = first sentences
reading-pass-headings = headings
# High verbosity: $label is a structure label ("Heading level 2").
nav-message-at-labelled = { $label }, line { $line }, { $pct } percent: { $content }
nav-message-at = Line { $line }, { $pct } percent: { $content }
nav-message-labelled = { $label }: { $content }
# $message is the navigation message after wrapping around.
nav-wrapped = Wrapped. { $message }
# $what is a kind-* or unit-* noun, $unit its key.
nav-no-next = No next { $what }.
nav-no-previous = No previous { $what }.
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
playback-speech-error = Speech error: { $error } Try again, or use the Restart speech command.
playback-end-of-section = End of section. { $key } to go on.
playback-time-up =
    { $minutes ->
        [one] Time is up after 1 minute. { $key } to go on.
       *[other] Time is up after { $minutes } minutes. { $key } to go on.
    }
playback-repeat-slower = Repeating slower, at { $rate } words per minute.
playback-recall-prompt = Say what you remember from { $section }. { $key } to go on.

## The title line and Say Status.

# The title line's position.
status-position = line { $line } of { $lines }
status-mode = { $mode } mode
status-modified = modified
status-self-voicing = self-voicing
status-hybrid = hybrid
status-screen-reader = screen reader mode
status-rate-spoken = { $wpm } words per minute
status-rate = { $wpm } wpm
# The window's status bar, last: reading time left at the current rate.
status-time-left =
    { $minutes ->
        [0] under a minute left
        [one] 1 minute left
       *[other] { $minutes } minutes left
    }
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
app-window-only = This command works in the textweaver window.
app-terminal-only = This command works in the terminal reader.
settings-save-failed = Could not save settings: { $error } Your changes stay in use until you quit.
settings-outside-kept = Settings changed outside textweaver were kept.
edit-still-editing = Still editing.
goto-not-a-target = Not a go-to target: { $text }. Type a line number, a percentage such as 50%, start, or end.

## Opening a document.

open-opened = Opened { $title }.
open-resumed = Opened { $title }. Resumed at { $pct } percent.
open-resumed-synced = Opened { $title }. Resumed at { $pct } percent, from another device.

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
# One line of the keyboard shortcuts list: the command's name-* (first, so
# type-ahead finds commands), its keys (inside a 40-cell Braille line), an
# action-* help, and its category-* title.
help-entry = { $name }: { $keys }. { $help }. { $category }
help-unknown-command = Unknown command: { $text }.
help-shortcuts-intro = Keyboard shortcuts, { $n } commands. Type to filter. Page Down moves to the next group, F1 explains a command, Enter runs it, Escape closes.
help-shortcuts-title = Keyboard shortcuts
# The keyboard shortcuts list, filtered: $filter is what was typed.
help-shortcuts-title-matching = Keyboard shortcuts matching { $filter }
# $n commands of $total match the filter.
help-shortcuts-filter-match = { $n } of { $total } commands match.
help-shortcuts-filter-none = No commands match { $query }. Backspace removes letters.
help-shortcuts-filter-cleared = Filter cleared, { $n } commands.
# Moving into a group of the keyboard shortcuts list: its name, its
# size, then the row ($item, with its place in the list).
help-shortcuts-group-item =
    { $group }, { $n ->
        [one] 1 command
       *[other] { $n } commands
    }. { $item }
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
help-access-window = Who reads: { $key } switches between textweaver reads aloud and my screen reader reads.
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
action-repeat-sentence-slower = Say the sentence at the cursor again more slowly, then go back to the usual rate
action-rsvp-toggle = Show or hide RSVP: one word at a time, from the cursor
action-rsvp-play-pause = Start or pause RSVP
action-rsvp-faster = RSVP faster
action-rsvp-slower = RSVP slower
action-rsvp-position-next = Move the RSVP word to the next place on the screen
action-reading-level = Say the reading level of the document or the selection
action-document-overview = Say the document's title, how many headings, tables, pictures, and footnotes it has, and about how many minutes are left
action-reading-pass = Change what reading says: the full text, the first sentence of each paragraph with the headings, or the headings only
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
action-search-options = Choose how Find and Replace match: case, whole words, regular expression, across lines
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
action-self-test = Test yourself on the notes and highlights: Enter shows each answer
action-open = Open a document
action-open-path = Open a document by typing its path
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
action-text-larger = Make the document text larger
action-text-smaller = Make the document text smaller
action-text-size-reset = Return the document text to its standard size
action-choose-font = Choose the font of the document text
action-contents-panel = Show the Contents panel beside the document and go to it, or close it from inside it: Enter goes to a heading
action-notes-panel = Show the Notes panel beside the document and go to it, or close it from inside it: Enter goes to a note
action-toggle-header = Show or hide the header, the bar of Open, Font, Edit, Settings and Commands
action-toggle-toolbar = Show or hide the toolbar, the bar of Play, Stop and the reading buttons
action-next-region = Move to the next part of the window: the header, the panel, the document, or the toolbar
action-previous-region = Move to the previous part of the window
action-command-palette = Run any command by name
action-settings = Open the settings: every option with its help; Left and Right change a value
action-keyboard-help = List keyboard shortcuts
action-help = Open the help

## The interface language. $language is the language's name in itself
## (Español), $voice a voice's name.

language-voice-changed = The voice is now { $voice }, for { $language }.
language-voice-kept = No voice for { $language } in this speech engine, so { $voice } keeps speaking.
language-list-title = Language
language-list-intro =
    { $n ->
        [one] Language, 1 choice. Up and Down move, Enter chooses, Escape keeps the language.
       *[other] Language, { $n } choices. Up and Down move, Enter chooses, Escape keeps the language.
    }

## Restarting speech.

restart-silent-now = { -brand } is silent now; restart it to hear speech again.
# $keys names the Restart Speech key or keys.
restart-silent-use-key = { -brand } is silent now. Restart speech with { $keys }.
restart-restarting = Restarting speech.
restart-not-here = Speech cannot be restarted here.
restart-already = Speech is already restarting.
# $error is the system's reason, in its own words.
restart-failed = Could not restart speech: { $error } Wait a moment, then try again.
restart-start-failed = Could not restart speech: starting it failed.
restart-no-engine = No speech engine is available; { -brand } stays silent. Troubleshooting, No speech at all, has the checks.
restart-done-silent = Speech restarted, but no speech engine is available; { -brand } stays silent.
restart-done = Speech restarted.

## Tab completion of file paths in prompts.

# $folder is the folder's full path.
pathc-no-folder = There is no folder { $folder }.
# $prefix is what was typed after the last separator.
pathc-no-match = No file or folder starts with { $prefix }.
# The one name that matched; $kind is folder or file.
pathc-one =
    { $kind ->
        [folder] { $name }, folder
       *[file] { $name }, file
    }
# $n names matched; $names are the first few, joined with commas; $more is
# yes when more matched than are read out.
pathc-many =
    { $more ->
        [yes] { $n } matches: { $names }, and more.
       *[no] { $n } matches: { $names }.
    }

## Exporting and importing settings.

# $n settings differ; $name is the file's name.
settingsio-import-question =
    { $n ->
        [one] Import { $n } changed setting from { $name }? y or n
       *[other] Import { $n } changed settings from { $name }? y or n
    }
settingsio-no-persistence = Settings are not saved in this session, so they cannot be exported or imported.
# $path is the file written.
settingsio-exported = Settings exported to { $path }.
settingsio-export-failed = Could not export settings: { $error } Check that the folder can be written to.
# $path is the file; $error the system's reason.
settingsio-read-failed = Could not read { $path }: { $error } Check the file, then import again.
settingsio-nothing-to-import = Nothing to import: your settings already match that file.
settingsio-cancelled-unchanged = Canceled. Nothing was changed.
settingsio-import-failed = Could not import settings: { $error } Check that the settings folder can be written to.
# $summary lists what changed (from the settings store, in English).
settingsio-imported = Settings imported. { $summary }
settingsio-backend-next-start = The new speech engine is used from the next start.
settingsio-backed-up = The old settings were backed up.

## Opening a document: failures and opening in the background.

# $name is the file's name.
opening-is-folder = { $name } is a folder, not a document. Give the name of a file in it.
# Said after "Could not open NAME:", so it starts in lower case.
opening-no-file-in = there is no file named { $name } in { $folder }. Check the name.
opening-no-file-here = there is no file named { $name } here. Check the name.
opening-no-permission = you do not have permission to read it.
opening-damaged-rtf = it is not a readable RTF file; it may be damaged.
opening-damaged-odt = it is not a readable OpenDocument text file; it may be damaged.
opening-damaged-latex = it is not a readable LaTeX file; it may be damaged or too large.
opening-damaged-email = it is not a readable email message; it may be damaged or too large.
opening-damaged-mhtml = it is not a readable web archive; it may be damaged or too large.
opening-pdf-password = it is protected by a password. Remove the password in a PDF program, then open it again.
opening-old-office = it is an old Microsoft Office file. Save it in a newer format, such as .docx, then open that.
opening-rar = it is a RAR archive, which does not open. Extract it first, or use ZIP or 7z.
# $reason is one of the opening-no-* messages, or the loader's own words.
opening-failed = Could not open { $name }: { $reason }
opening-started = Opening { $name }. Escape cancels.
opening-stopped = Stopped opening { $name }.
opening-still = Still opening { $name }, { $secs } seconds.
# $step is the loader's report, such as "recognizing text on page 3 (3 of 40)."
opening-still-step = Still opening { $name }: { $step }
opening-stopped-unexpectedly = Could not open { $name }: loading stopped unexpectedly.

## A build without the publish feature. "tw convert" is a command typed
## at the terminal: keep it as it is.

lean-citations-not-in-build = Citations are not in this version of { -brand }.
lean-publish-not-in-build = Export and preview are not in this version of { -brand }. tw convert still converts.

## The voice manager's list.

# $n voices are shown; $language is a language name or voices-all-languages;
# $engine an engine's name or voices-all-engines.
voices-shown =
    { $n ->
        [one] { $n } voice: { $language }, { $engine }.
       *[other] { $n } voices: { $language }, { $engine }.
    }
voices-all-languages = all languages
voices-all-engines = all engines
# The filter rows at the top of the list.
voices-language-row = Language: { $language }
voices-engine-row = Engine: { $engine }
voices-fetch-row = Fetch the Piper voice list from the internet
# Parts of a voice's row, joined with commas. $size is in megabytes, such
# as "63 MB".
voices-download-size = download { $size }
voices-licence-public-domain = public domain
voices-licence-attribution = free with credit
voices-licence-share-alike = free with credit, share alike
voices-licence-non-commercial = non-commercial
voices-licence-unknown = license shown before download
voices-favourite = favorite
voices-current = current

## Language names in the voice manager's language filter.

voices-language-ar = Arabic
voices-language-ca = Catalan
voices-language-cs = Czech
voices-language-cy = Welsh
voices-language-da = Danish
voices-language-de = German
voices-language-el = Greek
voices-language-en = English
voices-language-es = Spanish
voices-language-fa = Persian
voices-language-fi = Finnish
voices-language-fr = French
voices-language-hi = Hindi
voices-language-hu = Hungarian
voices-language-is = Icelandic
voices-language-it = Italian
voices-language-ja = Japanese
voices-language-ka = Georgian
voices-language-kk = Kazakh
voices-language-ko = Korean
voices-language-lb = Luxembourgish
voices-language-lv = Latvian
voices-language-nl = Dutch
voices-language-no = Norwegian
voices-language-pl = Polish
voices-language-pt = Portuguese
voices-language-ro = Romanian
voices-language-ru = Russian
voices-language-sk = Slovak
voices-language-sl = Slovenian
voices-language-sr = Serbian
voices-language-sv = Swedish
voices-language-sw = Swahili
voices-language-tr = Turkish
voices-language-uk = Ukrainian
voices-language-vi = Vietnamese
voices-language-zh = Chinese

## Voices: the voice manager, rate, pitch, and volume.

# Spoken by a newly chosen voice as its sample.
voice-sample = The quick brown fox jumps over the lazy dog.
voice-list-title = Choose a voice
voice-still-loading = The voices are still loading. The list opens when they are ready.
voice-list-failed = Could not list the voices: { $error } Choose another engine in the Speech menu.
# $shown is voices-shown ("12 voices: English, all engines."). Enter,
# Space, Delete and Escape are the list's own keys.
voice-manager-intro = Voice manager. { $shown } Enter uses a voice and speaks a sample, or downloads one; { $preview } previews a voice; Space marks a favorite; Delete removes a downloaded voice; Escape closes.
voice-more-ready =
    { $n ->
        [one] One more voice from another engine is listed.
       *[other] { $n } more voices from other engines are listed.
    }
voice-preview = Preview: { $voice }.
voice-preview-sample = { $voice }. The quick brown fox jumps over the lazy dog.
voice-preview-starting = Preview: { $voice }, starting { $engine }.
voice-preview-not-installed = { $voice } is not downloaded yet. Enter downloads it, after a question.
voice-preview-unavailable = { $engine } cannot start here for a preview. Enter switches to it.
voice-preview-engine-failed = Could not start { $engine } to preview { $voice }.
voice-preview-failed = Could not preview { $voice }: { $error } Try another voice.
# $keys names the Choose Voice key.
voice-ready = The voices are ready. { $keys } lists them.
voice-fetch-catalog-question = Download the list of Piper voices, about 250 kilobytes, from Hugging Face? y or n
voice-fetch-catalog-question-short = Download the list of Piper voices? y or n
# $engine is the engine's name, such as "Piper neural voices".
voice-switching-engine = Voice { $voice }, on { $engine }. Switching engine.
voice-download-in-progress = A voice download is already in progress.
voice-no-data-folder = There is no data folder to keep Piper voices in.
voice-not-in-list = That voice is not in the Piper voice list any more.
voice-download-start-failed = Could not start the download.
voice-reading-licence = Reading the license of { $voice }.
voice-remove-question = Remove the voice { $voice }? y or n
voice-only-piper-removable = Only downloaded Piper voices can be removed.
# $plan describes the download: the voice, its size and license.
voice-download-question = { $plan } y or n
voice-in-use = { $voice } is the voice in use. Choose another voice first.
voice-removed = { $voice } removed.
voice-remove-failed = Could not remove { $voice }: { $error } Check that its folder can be written to.
voice-downloading-catalog = Downloading the Piper voice list.
voice-downloading = Downloading { $voice }.
voice-downloading-percent = Downloading { $voice }, { $pct } percent.
voice-details-failed = Could not read the voice's details: { $error } Check the connection, then try again.
voice-download-stopped = The download stopped.
voice-catalog-fetched = The Piper voice list has { $voices } voices in { $languages } languages. The Voices command lists them.
voice-catalog-failed = Could not download the voice list: { $error } Check the connection, then try again.
# $licence describes the voice's license, in a sentence of its own.
voice-installed = { $voice } is installed. { $licence } The Voices command lists it.
voice-download-failed = Could not download { $voice }: { $error } Choose the voice again to retry.
voice-only-voice-favourite = Only a voice can be a favorite.
voice-favourite-added = { $voice } added to favorites.
voice-favourite-removed = { $voice } removed from favorites.
voice-chosen = Voice { $voice }.
voice-chosen-rate = Voice { $voice }, { $wpm } words per minute.
voice-fastest-rate = Fastest rate.
voice-slowest-rate = Slowest rate.
voice-rate = { $wpm } words per minute.
voice-highest-pitch = Highest pitch.
voice-lowest-pitch = Lowest pitch.
voice-pitch-normal = Normal pitch.
# $n is a number of semitones.
voice-pitch-plus = Pitch plus { $n }.
voice-pitch-minus = Pitch minus { $n }.
voice-full-volume = Full volume.
voice-volume-off = Volume off.
voice-volume = Volume { $pct } percent.
voice-no-speed-presets = No speed presets.
# $name is the preset's name from the settings, such as "Study".
voice-speed-preset = { $name }, { $wpm } words per minute.
voice-line-numbers-on = Line numbers on.
voice-line-numbers-off = Line numbers off.

## Export and preview from the reader. F5 is the browser's reload key,
## not textweaver's.

common-no-document = No document is open.
# Said after "Could not export:", so it starts in lower case. $path is a
# folder or a file; $error the system's reason.
publish-cannot-write-to = cannot write to { $path }: { $error } Check that the folder can be written to.
publish-cannot-write = cannot write { $path }: { $error } Check that its folder can be written to.
publish-start-failed = Could not start the export: { $error } Wait a moment, then try again.
publish-export-error = Could not export: { $error } Fix that, then export again.
# $format is the format's name, such as PDF, HTML, or Word.
publish-exporting = Exporting to { $format }.
publish-theme-title = Theme for the HTML page
publish-theme-intro = Theme for the HTML page? { $first } first, { $n } choices. Escape cancels.
publish-writing-preview = Writing the preview.
publish-preview-error = Could not write the preview: { $error } Fix that, then preview again.
publish-still-exporting =
    { $secs ->
        [one] Still exporting to { $format }, { $secs } second.
       *[other] Still exporting to { $format }, { $secs } seconds.
    }
publish-still-previewing =
    { $secs ->
        [one] Still writing the preview, { $secs } second.
       *[other] Still writing the preview, { $secs } seconds.
    }
# $again is yes when a preview is open already.
publish-auto-reload-on =
    { $again ->
        [yes] Automatic preview reloading on: after each save the browser reloads the page by itself. Run preview in browser again to use it.
       *[no] Automatic preview reloading on: after each save the browser reloads the page by itself.
    }
publish-auto-reload-off = Automatic preview reloading off: press F5 in the browser after a save.
publish-live-on = Live preview on: the preview also reloads when typing pauses.
# "toggle preview auto reload" is the command's name in the command palette.
publish-live-on-needs-reload = Live preview on. It needs automatic reloading, which is off. Turn it on with Reload preview automatically.
publish-live-off = Live preview off: the preview reloads after saves only.
# $error is the converter's reason.
publish-export-failed = Could not export to { $format }: { $error } Try another format.
publish-preview-failed = Preview failed: { $error } Save to try again.
# The converter's warnings: how many, and the first one.
publish-warnings =
    { $n ->
        [one] 1 warning: { $first }
       *[other] { $n } warnings; the first: { $first }
    }
# $file is the file's name, $folder its folder; $warned is empty or a
# space and publish-warnings.
publish-exported = Exported { $file }. Open it? y or n. Format { $format }, in { $folder }.{ $warned }
publish-report = Report saved as { $file }.
publish-report-issues =
    { $n ->
        [one] 1 item
       *[other] { $n } items
    } not made accessible; see the report.
publish-report-failed = The report could not be saved: { $error } Check that the output folder can be written to.
publish-preview-written-served = Preview written. Opening it in the browser. It reloads by itself after each save.{ $warned }
publish-preview-written = Preview written. Opening it in the browser. Saving writes it again; then press F5 in the browser.{ $warned }
publish-preview-updated = Preview updated.
publish-preview-updated-press-f5 = Preview updated. Press F5 in the browser.
publish-server-failed = Could not start the preview's reload server ({ $error }); opening the file instead.
publish-render-failed = Could not render the text: { $error } Leave edit mode to read the text.
publish-nothing-after-caret = Nothing to read after the cursor.
publish-listening = Listening to the rendered text.

## The preview's reload server: shown in the browser.

preview-being-written = The preview is being written. Reload in a moment.

## Notes and highlights.

notes-nothing-to-attach = Nothing here to attach a note to.
# $on is the start of the passage the note is on.
notes-added = Note added on: { $on }
# $tags are the note's tags, joined with commas.
notes-added-with-tags = Note added with tags { $tags } on: { $on }
# An item in the notes list. $anchor is the passage; $lost is yes when the
# passage was not found after the file changed.
notes-item =
    { $lost ->
        [yes] { $note }, line { $line }. On: { $anchor } Not found after the file changed.
       *[no] { $note }, line { $line }. On: { $anchor }
    }
notes-none = No notes. To add one: { $key }.
notes-list-title = Notes
notes-list-intro =
    { $n ->
        [one] Notes, 1 item. Enter goes to a note, Delete deletes it, F2 edits it, Space opens its links.
       *[other] Notes, { $n } items. Enter goes to a note, Delete deletes it, F2 edits it, Space opens its links.
    }
# Said on jumping to a note: its text, then the passage it is on.
notes-note-content = { $note }. On: { $anchor }
# $i is the note's number, $n how many notes there are.
notes-note-label = Note { $i } of { $n }
notes-deleted = Note deleted: { $text }.
notes-none-here = No note or highlight here.
notes-unchanged = Note unchanged.
notes-updated = Note updated.
notes-nothing-to-highlight = Nothing here to highlight.
notes-highlight-removed = Highlight removed: { $text }
notes-highlighted-at = Highlighted at { $pct } percent: { $text }
notes-highlighted = Highlighted: { $text }
# An item in the highlights list. $color is the highlight's color name;
# $lost is yes when the text was not found after the file changed.
notes-highlight-item =
    { $lost ->
        [yes] { $text }, line { $line }, { $color }, not found after the file changed
       *[no] { $text }, line { $line }, { $color }
    }
notes-no-highlights = No highlights. To make one: { $key }.
notes-highlights-title = Highlights
notes-highlights-intro =
    { $n ->
        [one] Highlights, 1 item. Enter goes to one, Delete removes it.
       *[other] Highlights, { $n } items. Enter goes to one, Delete removes it.
    }
# The label said before a highlight's text on jumping to it.
notes-highlight-label = Highlight
# Shown while reading reaches a note's passage.
notes-signal = Note: { $text }
# Said after moving onto a note's passage.
notes-has-note = Has a note: { $text }

## Relations between notes (the knowledge graph as lists).

# The ten relation types, as they read aloud. Lower case: they start list
# rows such as "supports: Chapter 3 note".
relations-type-conflicts-with = conflicts with
relations-type-supports = supports
relations-type-is-example-of = is an example of
relations-type-cites = cites
relations-type-contradicts = contradicts
relations-type-defines = defines
relations-type-extends = extends
relations-type-see-also = see also
relations-type-precedes = precedes
relations-type-follows = follows
# After a note in the notes list: how many links it has to other notes,
# and how many notes link to it.
relations-count = Links: { $out } out, { $in } in.
relations-note-title = Links of: { $note }
# $title is a list title, $filter the type filter typed.
relations-title-filtered = { $title }, filter: { $filter }
relations-note-intro = Links of { $note }: { $out } out, { $in } in. Enter follows a link, F2 changes it, Delete removes it. Type to filter by type.
# A link of the note: its type, then the note it goes to.
relations-out-item = { $type }: { $target }
# A note in another document. $doc is that document's title.
relations-target-in = { $note }, in { $doc }
relations-target-missing = a note not found
relations-empty-note = Empty note
relations-incoming-row =
    { $n ->
        [0] What links here: nothing yet
        [one] What links here: 1 note
       *[other] What links here: { $n } notes
    }
relations-add-row = Add a link
relations-backlinks-title = What links here: { $note }
relations-backlinks-intro =
    { $n ->
        [one] What links to { $note }: 1 note. Enter goes to it. Type to filter by type.
       *[other] What links to { $note }: { $n } notes. Enter goes to one. Type to filter by type.
    }
# A note linking here: the link's type, then that note.
relations-backlink-item = { $type } this, from: { $note }
relations-none-in = Nothing links to this note yet.
relations-types-title = Link type for: { $note }
relations-types-intro = Choose the type of link, 10 types. Type to filter.
# $type is the link type chosen.
relations-targets-title = { $type }: which note?
relations-targets-intro =
    { $n ->
        [0] No other note here. Choose a note in another document.
        [one] Choose the note to link to: 1 note. Enter links it.
       *[other] Choose the note to link to: { $n } notes. Enter links it.
    }
relations-other-document-row = A note in another document
relations-documents-title = Documents with notes
relations-documents-intro = Documents with notes: { $n }. Enter lists a document's notes.
relations-document-item =
    { $n ->
        [one] { $title }, 1 note
       *[other] { $title }, { $n } notes
    }
relations-no-other-documents = No other document in the library has notes.
relations-linked = Linked: { $type } { $target }.
relations-changed = Link changed: { $type } { $target }.
relations-already = Already linked: { $type } { $target }.
relations-removed = Link removed: { $type } { $target }.
relations-remove-question = Remove this link? y or n
relations-nothing-to-remove = Only a link can be removed here.
relations-note-gone = Note not found: it was deleted, or its document is gone.
relations-document-missing = Document not found: { $file }.
relations-no-note-here = No note here. Links belong to notes; add one: { $key }.
relations-filter-cleared = Filter cleared, { $n } shown.
relations-filter-none = Nothing matches { $filter }.
relations-filter-matched = Filter { $filter }: { $n } shown.

## Bookmarks: rename and delete.

notes-choose-bookmark-delete = Choose a bookmark and press Delete.
notes-choose-bookmark-rename = Choose a bookmark and press F2 to rename it.
notes-bookmark-deleted = Bookmark { $name } deleted.
notes-renaming-bookmark = Renaming bookmark { $name }.
notes-bookmark-unchanged = Bookmark unchanged.
# $name is the name asked for, $old the bookmark's name.
notes-bookmark-name-taken = There is already a bookmark called { $name }. Bookmark { $old } unchanged.
notes-bookmark-renamed = Bookmark { $old } renamed to { $name }.

## The study sheet.

notes-nothing-to-export = No notes or highlights to export.
# Keep the letters y and n: they are the keys that answer. $file is the
# sheet's file name, $folder the folder it was saved in.
notes-study-sheet-saved-notes =
    Study sheet with { $n ->
        [one] 1 note
       *[other] { $n } notes
    } saved as { $file }. Open it? y or n. In { $folder }.
notes-study-sheet-saved-highlights =
    Study sheet with { $h ->
        [one] 1 highlight
       *[other] { $h } highlights
    } saved as { $file }. Open it? y or n. In { $folder }.
notes-study-sheet-saved-both =
    Study sheet with { $n ->
        [one] 1 note
       *[other] { $n } notes
    } and { $h ->
        [one] 1 highlight
       *[other] { $h } highlights
    } saved as { $file }. Open it? y or n. In { $folder }.
notes-study-sheet-failed = Could not write the study sheet: { $error } Check that the folder can be written to.
# The study sheet file's own text (Markdown; the # marks stay in the code).
notes-sheet-title = Study sheet: { $title }
notes-sheet-exported = Exported from { -brand } on { $date }.
notes-sheet-before-first-heading = Before the first heading
# After a note's text: its tags, joined with commas.
notes-sheet-tags = (tags: { $tags })
# $color is the highlight's color name.
notes-sheet-highlighted = Highlighted, { $color }.

## The self-test: prompts with hidden answers (crate::reveal).

reveal-self-test-title = Self-test: { $title }
reveal-self-test-intro =
    { $n ->
        [one] Self-test, 1 prompt. Enter shows the answer. Space to answer aloud.
       *[other] Self-test, { $n } prompts. Enter shows each answer. Space to answer aloud.
    }
reveal-nothing-to-test = No notes or highlights to test. Add a note or highlight first.
reveal-prompt-note = { $note } (in { $section })
reveal-prompt-highlight = What did you highlight in { $section }?
reveal-row-shown = { $prompt } Answer: { $answer }
reveal-answer = Answer: { $answer }
reveal-listening = Answer aloud now. Space to stop.
reveal-you-said = You said: { $words }. Enter shows the answer.
reveal-heard-nothing = No answer heard. Space to try again.
reveal-no-dictation = Answering aloud needs dictation, which is not in this version.

## Find, bookmarks, and selection.

marks-cannot-search = Cannot search: { $error } Check the pattern between the slashes.
# $pattern is the text searched for.
marks-no-matches = No matches for { $pattern }.
# The label of a match reached by Find, at high verbosity; $number is its place among $n matches.
marks-match-label = Match { $number } of { $n }
# Find wrapped past an end of the document; $dir is next (to the top) or previous (to the bottom); $message says the match.
marks-find-wrapped =
    { $dir ->
        [next] Wrapped to top. { $message }
       *[previous] Wrapped to bottom. { $message }
    }
# $name is the bookmark's name, such as mark1.
marks-bookmark-already-here = Bookmark { $name } is already here.
common-bookmark-set = Bookmark { $name } set at { $pct } percent.
marks-no-bookmarks = No bookmarks. To add one: { $key }.
marks-bookmarks-intro =
    { $n ->
        [one] Bookmarks, { $n } item. Enter goes to one, Delete deletes it, F2 renames it.
       *[other] Bookmarks, { $n } items. Enter goes to one, Delete deletes it, F2 renames it.
    }
# One line of the bookmark list; $lost is yes when the bookmark's text was not found after the file changed; $text is the start of its line.
marks-bookmark-item =
    { $lost ->
        [yes] { $name } (not found after the file changed), line { $line }, { $pct } percent: { $text }
       *[no] { $name }, line { $line }, { $pct } percent: { $text }
    }
marks-bookmarks-title = Bookmarks
# The label of a bookmark reached, at high verbosity.
marks-bookmark-label = Bookmark { $name }
marks-selection-cleared = Selection cleared.
# $text is the selected text, shortened.
marks-selected = Selected { $text }

## Authoring lists: the outline, the citation picker, spelling, grammar, and templates.

# An outline item: $text is the heading's text, $level its level.
lists-outline-item = { $text }, level { $level }
# $n is the number of headings.
lists-outline-title =
    { $n ->
        [one] Outline, { $n } heading
       *[other] Outline, { $n } headings
    }
# $shown headings of $n match the filter $filter typed so far.
lists-outline-title-filtered = Outline, { $shown } of { $n } match { $filter }
# $n is the number of references.
lists-citations-title =
    { $n ->
        [one] Insert citation, { $n } reference
       *[other] Insert citation, { $n } references
    }
# $shown references of $n match the filter $filter typed so far.
lists-citations-title-filtered = Insert citation, { $shown } of { $n } match { $filter }
# $word is the misspelled word.
lists-spelling-title = Spelling of { $word }
lists-spelling-add = Add { $word } to your word list
lists-leave-as-is = Leave it as it is
# $words are the words the grammar fixes are for.
lists-grammar-title = Grammar fixes for { $words }
# $n is the number of templates.
lists-templates-title = New document from a template, { $n } templates
lists-no-filter = This list does not filter.
# The filter was emptied: $n items are shown.
lists-filter-cleared-headings =
    { $n ->
        [one] Filter cleared, { $n } heading.
       *[other] Filter cleared, { $n } headings.
    }
lists-filter-cleared-references =
    { $n ->
        [one] Filter cleared, { $n } reference.
       *[other] Filter cleared, { $n } references.
    }
lists-filter-cleared-items =
    { $n ->
        [one] Filter cleared, { $n } item.
       *[other] Filter cleared, { $n } items.
    }
# Nothing matches the filter $query.
lists-filter-none-headings = No headings match { $query }. Backspace removes letters.
lists-filter-none-references = No references match { $query }. Backspace removes letters.
lists-filter-none-items = No items match { $query }. Backspace removes letters.
# $n items match the filter.
lists-filter-matched-headings =
    { $n ->
        [one] { $n } heading match.
       *[other] { $n } headings match.
    }
lists-filter-matched-references =
    { $n ->
        [one] { $n } reference match.
       *[other] { $n } references match.
    }
lists-filter-matched-items =
    { $n ->
        [one] { $n } item match.
       *[other] { $n } items match.
    }
lists-no-headings = This document has no headings.
# $n is the number of headings; the keys are the outline list's own.
lists-outline-intro =
    { $n ->
        [one] Outline, { $n } heading. Type to filter, Enter goes to a heading, Escape closes.
       *[other] Outline, { $n } headings. Type to filter, Enter goes to a heading, Escape closes.
    }
# $heading is the text of the heading the cursor is under.
lists-outline-here = You are under { $heading }.

## Lists and prompts shared by every frontend.

# The focused list item: $item is its text, $k its place, $n the number of items.
listmodel-item-position = { $k } of { $n }, { $item }
# $letter is the letter or digit typed.
listmodel-no-item-starts = No item starts with { $letter }.
listmodel-top-of-list = Top of list.
listmodel-end-of-list = End of list.
listmodel-no-matching-commands = No matching commands.
listmodel-no-earlier-entries = No earlier entries.
# Tab in the command palette: $n commands match (always more than one); $names lists the first few, joined by commas.
listmodel-command-matches = { $n } matches: { $names }.

## The library.

# $n is how many documents the scan has found.
library-still-scanning = Still scanning the library: { $n } found so far.
library-scan-failed = Could not scan the library: { $error } Check the library folders in Settings.
library-scanning = Scanning the library.
library-scan-progress = Scanning the library: { $n } found so far.
library-scan-stopped = The library scan stopped unexpectedly. Open the library again to retry.
# $command is the command line that adds a folder; $key names the Open command's key.
library-empty = The library is empty. Add a folder in Settings, under Library folders, or open a file with { $key }.
library-add-folder-choose = Choose the folder to add to the library
library-folder-added = Added { $name } to the library. Open the library to see its documents.
library-folder-already = { $name } is already in the library.
library-intro =
    { $n ->
        [one] Library, { $n } document. Type to filter, Enter opens one, F2 edits details.
       *[other] Library, { $n } documents. Type to filter, Enter opens one, F2 edits details.
    }
library-title = Library

## Following links and footnotes.

links-none-here = No link or footnote at the cursor.
# $text is the link's text.
common-link-no-address = The link { $text } has no address.
# $kind is mail or web; $target is the link's address.
links-open-question =
    { $kind ->
        [mail] Open mail link? y or n. { $target }
       *[web] Open web link? y or n. { $target }
    }
# The label of a heading reached by a link, at high verbosity.
links-heading-label = Heading
# $anchor is the heading name the link gives.
links-no-heading = No heading called { $anchor } in this document.
# $file is the file the link names.
links-file-not-found = The link goes to { $file }, which was not found.
# $file is the file's name; $key names the History Back command's keys.
links-followed = Followed the link to { $file }. Back: { $key }.
links-back-in = Back in { $file }.
# $label is the footnote's label, such as 1.
links-back-to-footnote-reference = Back to footnote reference { $label }, line { $line }.
# $text is the start of the note.
links-footnote = Footnote { $label }: { $text }
links-footnote-unreferenced = No reference to footnote { $label } in the text.
links-footnote-no-note = Footnote { $label } has no note.

## Citations: inserting, looking up, importing, checking, and the bibliography.

citations-on = Citations on.
citations-off = Citations off.
# $key names the Add Reference command's keys.
citations-library-empty = Your reference library is empty. Add a reference by DOI or ISBN with { $key }, or run import references from the command palette.
# $n is how many references the picker lists.
citations-picker-intro =
    { $n ->
        [one] Insert citation, { $n } reference. Type to filter, Enter chooses, Escape cancels.
       *[other] Insert citation, { $n } references. Type to filter, Enter chooses, Escape cancels.
    }
# $text is what was typed at the locator prompt.
citations-locator-unreadable = Could not read the locator { $text }. Type a page such as 12, pages such as 3-5, or chapter 2; Enter alone for none.
citations-insert-failed = Could not insert the citation: { $error } The text is unchanged.
# $what is the identifier being looked up, as the citation library describes it.
citations-looking-up = Looking up { $what }.
citations-lookup-not-started = Could not start the lookup: { $error } Wait a moment, then try again.
# $input is the DOI or ISBN as typed.
citations-lookup-failed = Could not look up { $input }: { $error } Check the DOI or ISBN and the connection.
citations-no-library-to-add-to = There is no library to add to: { -brand } keeps no files in this session.
citations-library-save-failed = Could not save the library: { $error } Check that its folder can be written to.
# $n is how many citations the document has.
citations-found-no-library =
    { $n ->
        [one] { $n } citation found. { -brand } keeps no library in this session.
       *[other] { $n } citations found. { -brand } keeps no library in this session.
    }
citations-check-failed = Could not check the citations: { $error } Check the reference library file.
citations-no-library-to-import-into = There is no library to import into: { -brand } keeps no files in this session.
# $file is the file's path.
citations-import-failed = Could not import { $file }: { $error } Check that it is a .bib, .ris, or .json file.
# $style is the style's name from the front matter, such as apa.
citations-style-unusable = Cannot use the citation style { $style }: { $error } Check the style name at the top of the document.
citations-format-failed = Could not format the citations: { $error } Check the citation keys and the library.
# $key names the Insert Citation command's keys.
citations-none-yet = The document has no citations yet. Insert one with { $key }.
citations-nothing-to-list = None of the cited works is in your library, so there is nothing to list.
# $n is how many entries went in; $style is the style's name, such as apa.
citations-bibliography-inserted =
    { $n ->
        [one] Inserted the bibliography, { $n } entry, { $style } style.
       *[other] Inserted the bibliography, { $n } entries, { $style } style.
    }
# Follows citations-bibliography-inserted; $keys are citation keys joined with commas.
citations-not-in-library = Not in the library: { $keys }.
citations-bibliography-insert-failed = Could not insert the bibliography: { $error } The text is unchanged.

## Speech Cursor mode.

speechcursor-off = Speech Cursor off.
# $line is the line number.
speechcursor-on = Speech Cursor on, line { $line }. Up and Down read lines, Enter reads on, Tab or Escape leaves.
# $text is the line read, as the status line shows it.
speechcursor-on-with-text = Speech Cursor on, line { $line }: { $text }. Up and Down read lines, Enter reads on, Tab or Escape leaves.

## Scrolling the view without moving the cursor.

view-bottom-of-document = Bottom of document.
# $line is the line now at the top of the view.
view-line-at-top = Line { $line } at top.

## Background work, and opening files and addresses.

# $what names the export, as its own message says it.
tasks-stopped = { $what } stopped unexpectedly.
# $input is the DOI, ISBN, or other identifier being looked up.
tasks-lookup-stopped = Looking up { $input } stopped unexpectedly.
# The reason in tasks-could-not-open when a session keeps no files.
tasks-launch-off = opening other programs is off in a session that keeps no files
tasks-opening = Opening.
# $target is a file or a web address; $error says why.
tasks-could-not-open = Could not open { $target }: { $error }
open-refused-scheme = Not opened: { $scheme } link blocked.
open-refused-missing = Not opened: the file was not found.
open-refused-invalid = Not opened: not a valid address.
tasks-not-opened = Not opened.
# Keep the letters y and n: they are the keys that answer.
tasks-open-it-question = Open it? y or n

## Math exploration.

mathx-no-math = No math here. Move to a formula, then try again.
# $math is the whole expression as spoken; $parts is yes when it has parts to go into. The keys are exploration's own.
mathx-exploring =
    { $parts ->
        [yes] Exploring math: { $math }. Down goes in, arrows move, Escape leaves.
       *[no] Exploring math: { $math }. Escape leaves.
    }
mathx-left = Left math.
mathx-last-term = Last term.
mathx-first-term = First term.
mathx-no-parts = No parts inside.
mathx-whole-expression = Whole expression.
mathx-nothing-here = Nothing here.
# $speech is what was said for the step; $code is the math braille code's name (Nemeth or UEB); $braille is the part's braille in Unicode braille cells, for the Braille display.
mathx-step-braille = { $speech } { $code }: { $braille }

## Reading aids: RSVP, bionic reading, syllables, difficult words, the ruler, and the reading level.

aids-rsvp-off = RSVP off.
aids-rsvp-leave-edit = Leave edit mode to use RSVP.
# $status is RSVP's status line (word and sentence counts, rate, state).
aids-rsvp-on = RSVP on. { $status }
aids-rsvp-no-words = No words to show.
aids-rsvp-fastest = Fastest RSVP rate.
aids-rsvp-slowest = Slowest RSVP rate.
# $wpm is the new rate in words per minute.
aids-rsvp-rate = RSVP { $wpm } words per minute.
# Where the RSVP word is shown; $position is one of nine fixed keys.
aids-rsvp-position =
    { $position ->
        [top-left] RSVP at the top left.
        [top-center] RSVP at the top center.
        [top-right] RSVP at the top right.
        [center-left] RSVP at the middle left.
        [center] RSVP at the center.
        [center-right] RSVP at the middle right.
        [bottom-left] RSVP at the bottom left.
        [bottom-right] RSVP at the bottom right.
       *[bottom-center] RSVP at the bottom center.
    }
aids-rsvp-playing = RSVP playing.
aids-rsvp-paused = RSVP paused.
aids-rsvp-end-of-text = End of text.
aids-rsvp-start-of-text = Start of text.
aids-bionic-on = Bionic reading on.
aids-bionic-off = Bionic reading off.
aids-syllables-shown = Syllables shown.
aids-syllables-hidden = Syllables hidden.
aids-difficult-on = Difficult words underlined.
aids-difficult-no-list = Difficult words on, but this version has no word list.
aids-difficult-off = Difficult words not marked.
# Added after a word at high verbosity, following a comma.
aids-difficult-word = difficult word
aids-ruler-off = Reading ruler off.
aids-ruler-current-line = Current line marked.
aids-ruler-on = Reading ruler on.
# $summary is aids-level-summary; $scope says what was measured.
aids-reading-level =
    { $scope ->
        [selection] Selection: { $summary }
       *[document] Document: { $summary }
    }
aids-reading-level-too-short = Not enough text to measure the reading level.
# $grade is the Flesch-Kincaid grade with one decimal, $band an aids-band-* message, $ease the reading ease (0 to 100), $words aids-level-words, $sentences aids-level-sentences.
aids-level-summary = Grade { $grade }, { $band }. Reading ease { $ease } out of 100. { $words } in { $sentences }.
# $n is the count, $count the same number written with thousands separators.
aids-level-words =
    { $n ->
        [one] { $count } word
       *[other] { $count } words
    }
aids-level-sentences =
    { $n ->
        [one] { $count } sentence
       *[other] { $count } sentences
    }
aids-band-elementary = elementary
aids-band-middle-school = middle school
aids-band-high-school = high school
aids-band-college = college
aids-band-graduate = graduate

## Themes.

# Put in front of every error message, so it reads as an error without color; $message is the error.
message-error = Error: { $message }
# $name is the theme name in the settings; $used the display name of the theme used instead.
themes-unknown = There is no theme called { $name }; using { $used }.
# $theme is the new theme's display name.
themes-next = Theme { $theme }.
themes-soft-dark-name = { $theme }, soft dark
themes-choice-aa = { $theme }, meets AA
themes-choice-below-aa = { $theme }, below AA
# Said after the theme name when the theme falls short of WCAG AA; $count is the number of contrast checks that fail.
themes-below-aa =
    { $count ->
        [one] Below AA: 1 check falls short.
       *[other] Below AA: { $count } checks fall short.
    }

## Accessibility modes and the first-run question.

# Said when the accessibility mode changes; $mode is the new mode's id.
access-mode-changed =
    { $mode ->
        [self-voicing] Self-voicing mode. textweaver speaks everything.
        [screen-reader] Screen reader mode. textweaver is silent; your screen reader reads the status line.
       *[hybrid] Hybrid mode. textweaver reads documents aloud; your screen reader speaks messages and typing.
    }
# Yes to the first-run question; $key names the keys that change the mode.
access-hybrid-chosen = Hybrid mode. textweaver reads documents aloud; your screen reader speaks messages and typing. { $key } changes the mode.
# No to the first-run question; $key names the keys that change the mode.
access-hybrid-declined = Staying in self-voicing mode. { $key } changes the mode.
# $reader is the screen reader found (NVDA, JAWS), or access-a-screen-reader.
access-hybrid-question = { $reader } is running. Use hybrid mode, where textweaver reads documents aloud and your screen reader speaks messages and typing? y or n
access-a-screen-reader = A screen reader
# On the first run with a screen reader and no mode chosen: hybrid mode is
# used, not asked. $reader is the screen reader found (NVDA, JAWS), or
# access-a-screen-reader; $key names the keys that change the mode.
access-hybrid-inferred = { $reader } is running: textweaver reads documents aloud and leaves messages to your screen reader. { $key } changes this.
# The window's two modes, when the mode changes; $key changes it again.
access-window-choice-reads-aloud = textweaver reads aloud
access-window-choice-screen-reader = my screen reader reads
access-window-mode-help = Who reads in the window: textweaver's voice, or your screen reader alone.
access-window-mode-changed =
    { $mode ->
        [screen-reader] My screen reader reads: textweaver stays silent and sends the text to your screen reader. { $key } changes this.
        [speaks-messages] textweaver reads aloud and speaks its messages. { $key } changes this.
       *[reads-aloud] textweaver reads aloud; messages go to your screen reader. { $key } changes this.
    }

## Characters and selections, as spoken.

# The name of a white-space character read on its own; $name is a fixed key.
text-char-name =
    { $name ->
        [space] space
        [new-line] new line
        [tab] tab
        [no-break-space] no-break space
       *[white-space] white space
    }
# Said after a selection grows ($change is selected) or shrinks (unselected); $text is the text or a character's name.
text-selection-change =
    { $change ->
        [selected] { $text } selected
       *[unselected] { $text } unselected
    }

## The settings screen. $label is a setting-* label, $value its value as
## described below.

settings-not-set = not set
settings-none = none
settings-empty = empty
# A number and its unit (a settings-unit-* message): "300 words per minute".
settings-number-unit = { $n } { $unit }
settings-entries =
    { $n ->
        [0] none
        [one] 1 entry
       *[other] { $n } entries
    }
settings-type-on-or-off = Type on or off.
settings-type-a-number = Type a number from { $min } to { $max }.
settings-outside = { $n } is outside { $min } to { $max }.
# $names are the choices, joined with commas.
settings-choose-one-of = Choose one of: { $names }.
settings-edit-table = Edit { $label } in settings.toml; it holds names and values.
# $path is a key such as speech.rate, not translated.
settings-no-such-setting = There is no setting { $path }.
settings-cannot-be = { $label } cannot be that: { $error } Choose another value.
settings-changed = { $label }, { $value }.
settings-clamped = Out of range, so the nearest value is used.
settings-restart-speech = Restart speech to use it.
settings-next-start = Used from the next start.
settings-intro = Settings, { $n } settings. Type to filter. Left and Right change a value, Enter changes or types one, Delete puts the default back, Escape closes.
settings-item = { $label }: { $value }
settings-title = Settings
settings-title-matching = Settings matching { $filter }
settings-closed = Settings closed.
settings-filter-cleared =
    { $n ->
        [one] Filter cleared, 1 setting.
       *[other] Filter cleared, { $n } settings.
    }
settings-filter-none = No settings match { $query }. Backspace removes letters.
settings-filter-match =
    { $n ->
        [one] 1 setting match.
       *[other] { $n } settings match.
    }
settings-largest = Largest value, { $value }.
settings-smallest = Smallest value, { $value }.
settings-press-enter = { $label }: press Enter to type a new value.
settings-table-item = { $label }: { $value }. Edit it in settings.toml.
# $help is the setting's help (setting-*-help), which may be empty.
settings-editing = { $label }, now { $value }. { $help }

## Settings: labels, help, and choices, as the settings screen shows and
## says them. Ids follow the key in settings.toml (speech.rate is
## setting-speech-rate).

setting-speech-backend = Speech engine
setting-speech-backend-help = The speech engine: auto picks the best one available. A change restarts speech.
choice-speech-backend-auto = automatic
choice-speech-backend-eci = Eloquence
choice-speech-backend-sapi = SAPI 5 voices
choice-speech-backend-espeak = eSpeak NG
choice-speech-backend-speechd = Speech Dispatcher
choice-speech-backend-nsspeech = Apple NSSpeech
choice-speech-backend-avspeech = Apple AVSpeech
choice-speech-backend-dectalk = DECtalk
choice-speech-backend-omnivox = Omnivox
choice-speech-backend-null = silent
setting-speech-rate = Rate
setting-speech-rate-help = How fast textweaver speaks.
setting-speech-volume = Volume
setting-speech-volume-help = How loud textweaver speaks.
setting-speech-pitch = Pitch
setting-speech-pitch-help = Higher or lower than the voice's own pitch.
setting-speech-voice = Voice
setting-speech-voice-help = The voice's id; not set picks one automatically. The Voices command lists them.
setting-speech-prefer-voice = Preferred voice
setting-speech-prefer-voice-help = When no voice is set, the first voice whose name contains this, such as eloquence.
setting-speech-favorite-voices = Favorite voices
setting-speech-favorite-voices-help = Voices listed first by the Voices command, by id, separated by commas.
setting-speech-punctuation = Punctuation
setting-speech-punctuation-help = How much punctuation is spoken.
choice-speech-punctuation-none = none
choice-speech-punctuation-some = some
choice-speech-punctuation-all = all
setting-speech-split-caps = Split capitals
setting-speech-split-caps-help = Say words joined with capitals, such as TextWeaver, as separate words.
setting-speech-caps = Capitals
setting-speech-caps-help = How a capital letter is marked when characters are spoken and typed.
choice-speech-caps-none = not marked
choice-speech-caps-tone = a tone
choice-speech-caps-pitch = a higher pitch
choice-speech-caps-say-cap = say cap
setting-speech-auto-play = Read on opening
setting-speech-auto-play-help = Start reading when a document opens.
setting-speech-skip-code = Skip code blocks
setting-speech-skip-code-help = Do not speak code blocks.
setting-speech-speed-presets = Speed presets
setting-speech-speed-presets-help = Named rates that F8 cycles through.
setting-speech-voices-by-language = Voices by language
setting-speech-voices-by-language-help = The voice for each interface language, by language tag, such as es = the voice's id. A language not listed uses the engine's first voice for it.
setting-speech-latency-offset-ms = Highlight delay
setting-speech-latency-offset-ms-help = How long after an engine reports a word the highlight moves, for engines timed by their audio clock.
setting-speech-pause-heading-ms = Pause after headings
setting-speech-pause-heading-ms-help = Silence after a heading, shorter at faster rates. 0 turns it off.
setting-speech-pause-paragraph-ms = Pause after paragraphs
setting-speech-pause-paragraph-ms-help = Silence after a paragraph, shorter at faster rates. 0 turns it off.
setting-speech-pause-list-item-ms = Pause after list items
setting-speech-pause-list-item-ms-help = Silence after a list item, shorter at faster rates. 0 turns it off.
setting-speech-markup-pauses = Pauses written as markup
setting-speech-markup-pauses-help = Read pause markup in a document, such as <break time="1s"/>, as a pause. Turn it off for documents that quote such markup.
setting-speech-output-device = Output device
setting-speech-output-device-help = The sound device speech plays on, by its id. Not set, or a device that is not connected, uses the system's default.
setting-speech-verbosity = Verbosity
setting-speech-verbosity-help = How much textweaver says about what it does.
choice-speech-verbosity-low = low
choice-speech-verbosity-normal = normal
choice-speech-verbosity-high = high
setting-speech-eci-dictionaries = Eloquence dictionaries
setting-speech-eci-dictionaries-help = The community pronunciation dictionaries for Eloquence: on, off, or a folder of your own.
choice-speech-eci-dictionaries-true = on
choice-speech-eci-dictionaries-false = off
setting-speech-eci-library = Eloquence library
setting-speech-eci-library-help = The ECI library to load; not set searches the usual places.
setting-speech-eci-code-factory = Search Code Factory's Eloquence
setting-speech-eci-code-factory-help = Also look for Code Factory's Eloquence for Windows. Its license may not cover other programs.
setting-speech-sapi-onecore = OneCore voices
setting-speech-sapi-onecore-help = Also list the Windows OneCore voices through SAPI 5.
setting-speech-apple-backend = Apple speech engine
setting-speech-apple-backend-help = Which of Apple's speech engines to use on macOS.
choice-speech-apple-backend-auto = automatic
choice-speech-apple-backend-nsspeech = NSSpeechSynthesizer
choice-speech-apple-backend-avspeech = AVSpeechSynthesizer
setting-highlight-enabled = Highlight spoken text
setting-highlight-enabled-help = Highlight the word or sentence being read.
setting-highlight-granularity = Highlight
setting-highlight-granularity-help = What the reading highlight covers.
choice-highlight-granularity-word = the word
choice-highlight-granularity-sentence = the sentence
choice-highlight-granularity-both = the word and the sentence
setting-highlight-lead-words = Highlight lead
setting-highlight-lead-words-help = Draw the highlight this many words ahead of the word heard (1 is the word heard).
setting-highlight-speed = Highlight speed
setting-highlight-speed-help = Speed of the timed highlight for engines that report no words, as a multiple. 1 is normal speed.
setting-highlight-color = Word highlight color
setting-highlight-color-help = The color behind the word being read. Choose a name, or type a hex code. Default: the theme's color.
setting-highlight-sentence-color = Sentence highlight color
setting-highlight-sentence-color-help = The color behind the sentence being read. Choose a name, or type a hex code. Default: the theme's color.
setting-normalization-math = Speak math
setting-normalization-math-help = Speak math notation in words.
setting-normalization-math-verbosity = Math verbosity
setting-normalization-math-verbosity-help = How explicit spoken math is: low says a over b, normal and high say more.
choice-normalization-math-verbosity-low = low
choice-normalization-math-verbosity-normal = normal
choice-normalization-math-verbosity-high = high
setting-normalization-asciimath-delimiter = ASCIIMath delimiter
setting-normalization-asciimath-delimiter-help = The character around ASCIIMath, usually a backtick; not set reads no ASCIIMath.
setting-normalization-abbreviations = Expand abbreviations
setting-normalization-abbreviations-help = Say abbreviations in full, such as Doctor for Dr.
setting-normalization-abbrev-expansions = Your abbreviations
setting-normalization-abbrev-expansions-help = Abbreviations of your own and what they stand for.
setting-normalization-numbers = Numbers in words
setting-normalization-numbers-help = Say numbers, dates, times, and money in words.
setting-normalization-use-pronunciations = Use pronunciations
setting-normalization-use-pronunciations-help = Apply your pronunciation list.
setting-normalization-pronunciations = Pronunciations
setting-normalization-pronunciations-help = Words and how to say them.
setting-normalization-table-mode = Tables
setting-normalization-table-mode-help = How tables are read.
choice-normalization-table-mode-structured = with rows and columns
choice-normalization-table-mode-flat = as text
choice-normalization-table-mode-skip = skipped
setting-normalization-footnote-mode = Footnotes
setting-normalization-footnote-mode-help = Where footnotes are read.
choice-normalization-footnote-mode-inline = where they are marked
choice-normalization-footnote-mode-deferred = at the end
choice-normalization-footnote-mode-skip = skipped
setting-normalization-community-lexicon-enabled = Community lexicon
setting-normalization-community-lexicon-enabled-help = Apply the community pronunciation dictionaries for engines other than Eloquence.
setting-normalization-community-lexicon-dir = Community lexicon folder
setting-normalization-community-lexicon-dir-help = The folder holding the dictionary files; not set looks beside textweaver.
setting-normalization-community-lexicon-language = Community lexicon language
setting-normalization-community-lexicon-language-help = The dictionaries' language.
choice-normalization-community-lexicon-language-enu = US English
choice-normalization-community-lexicon-language-deu = German
setting-normalization-medical-lexicon-enabled = Medical lexicon
setting-normalization-medical-lexicon-enabled-help = Read drug names, clinical terms and dosing abbreviations from a medical pronunciation list.
setting-normalization-medical-lexicon-overlay = Medical lexicon file
setting-normalization-medical-lexicon-overlay-help = Your own medical pronunciations, which win over the built-in ones. Not set reads medical-lexicon.toml in the settings folder.
setting-reading-auto-resume =Resume where you left off
setting-reading-auto-resume-help = Go back to the saved position when a document opens.
setting-reading-nav-history-size = Back history
setting-reading-nav-history-size-help = How many places Back remembers.
setting-reading-wrap-navigation = Wrap navigation
setting-reading-wrap-navigation-help = Moving past the end of the document goes on from the start.
setting-reading-cursor-follows-speech = Cursor follows speech
setting-reading-cursor-follows-speech-help = The cursor moves with the word being read.
setting-reading-citations = Citations
setting-reading-citations-help = Citations in continuous reading: skipped, or said in words.
choice-reading-citations-off = skipped
choice-reading-citations-words = in words
setting-reading-ocr = Recognize scanned pages
setting-reading-ocr-help = Read the text of scanned PDFs and pictures by recognizing it (OCR).
setting-reading-ocr-lang = Scanned text language
setting-reading-ocr-lang-help = The language of scanned text, as Tesseract codes such as fra or deu+eng. Empty means the document's own language, else English.
choice-reading-ocr-lang- = the document's
choice-reading-ocr-lang-eng = English
choice-reading-ocr-lang-fra = French
choice-reading-ocr-lang-deu = German
choice-reading-ocr-lang-spa = Spanish
setting-reading-ocr-engine = OCR engine
setting-reading-ocr-engine-help = Which engine recognizes scanned pages. ocrs for English and Tesseract for other languages, or one of them always.
choice-reading-ocr-engine-auto = automatic
choice-reading-ocr-engine-ocrs = ocrs
choice-reading-ocr-engine-tesseract = Tesseract
choice-reading-ocr-engine-paddle = PaddleOCR (experimental)
setting-reading-math-engine = Math speech
setting-reading-math-engine-help = Which engine reads math aloud. textweaver's own, or MathCAT in ClearSpeak or SimpleSpeak, in the document's language. MathCAT needs a version that includes it; otherwise textweaver's own is used.
choice-reading-math-engine-builtin = textweaver
choice-reading-math-engine-mathcat = MathCAT ClearSpeak
choice-reading-math-engine-mathcat-simplespeak = MathCAT SimpleSpeak
setting-braille-math-code = Math braille
setting-braille-math-code-help = The braille code for math in BRF files and while exploring a formula with MathCAT. Nemeth, or UEB mathematics. It needs a version that includes MathCAT; otherwise math is written as its spoken words.
choice-braille-math-code-nemeth = Nemeth
choice-braille-math-code-ueb = UEB
setting-braille-table-format = Braille tables
setting-braille-table-format-help = How BRF files lay out tables. Linear, one row per line with semicolons between entries; listed, each row a heading with each entry on its own line after its column heading; or stairstep, each entry two cells right of the one before, for tables of up to four columns.
choice-braille-table-format-linear = linear
choice-braille-table-format-listed = listed
choice-braille-table-format-stairstep = stairstep
setting-reading-math-display = Math on screen
setting-reading-math-display-help = How math looks in the reading view. As its source, such as x^2, or as Unicode, such as x with a superscript 2. Speech and edit mode always use the source.
choice-reading-math-display-source = source
choice-reading-math-display-unicode = Unicode
setting-reading-revisions = Tracked changes
setting-reading-revisions-help = How tracked changes in Word, OpenDocument, and RTF files are read. Said in place at high verbosity (automatic), always said, or never said, reading the final text. Applies when a document is opened.
choice-reading-revisions-auto = automatic
choice-reading-revisions-marked = always say them
choice-reading-revisions-final = final text only
setting-reading-stop-at = Stop at section end
setting-reading-stop-at-help = Where continuous reading stops by itself and says End of section. Never, at the next heading of any level, or at the next chapter: a section break, else a level 1 heading. Reading goes on from the heading with the read key.
choice-reading-stop-at-off = never
choice-reading-stop-at-heading = next heading
choice-reading-stop-at-chapter = next chapter
setting-reading-stop-after-minutes = Reading timer
setting-reading-stop-after-minutes-help = Continuous reading stops at a sentence end after this many minutes of reading. It says so. Pausing stops the clock; stopping starts it over. 0 turns the timer off.
setting-reading-recall-prompts = Recall prompts
setting-reading-recall-prompts-help = At a section end, reading asks you to say what you remember. Reading stops at the next heading for this when Stop at section end is never. Reading goes on with the read key.
setting-display-theme = Theme
setting-display-theme-help = The color theme.
setting-display-follow-os-theme = Follow the system theme
setting-display-follow-os-theme-help = At startup, use a light, dark, or high-contrast theme like the system, unless you picked one.
setting-display-wrap-width = Wrap width
setting-display-wrap-width-help = Wrap lines at this many columns; 0 uses the whole width.
setting-display-measure = Line length
setting-display-measure-help = How many characters a line holds in the window, from 25 to 90. 0 fills the window. The terminal uses the wrap width.
setting-display-tab-width = Tab width
setting-display-tab-width-help = Columns a tab takes.
setting-display-show-line-numbers = Line numbers
setting-display-show-line-numbers-help = Show line numbers.
setting-display-scroll-margin = Scroll margin
setting-display-scroll-margin-help = Lines kept in view above and below the cursor.
setting-display-hints = Key hints line
setting-display-hints-help = Whether the terminal reader shows key hints on its bottom line. Automatic shows them when self-voicing and hides them with a screen reader. F1 and the keyboard shortcuts list always name the keys.
choice-display-hints-auto = automatic
choice-display-hints-on = on
choice-display-hints-off = off
setting-editing-autosave-recovery = Recovery snapshots
setting-editing-autosave-recovery-help = Keep a copy of unsaved work and offer it after a crash.
setting-editing-autosave-interval-secs = Snapshot interval
setting-editing-autosave-interval-secs-help = Seconds between recovery snapshots while there are unsaved changes.
setting-editing-echo-characters = Echo characters
setting-editing-echo-characters-help = Say each character typed.
setting-editing-echo-words = Echo words
setting-editing-echo-words-help = Say each word typed.
setting-editing-echo-deletions = Echo deletions
setting-editing-echo-deletions-help = Say what Backspace and Delete remove.
setting-editing-echo-lines-on-move = Echo lines
setting-editing-echo-lines-on-move-help = Say the line when the cursor moves to another line.
setting-editing-undo-steps = Undo steps
setting-editing-undo-steps-help = Most undo steps kept while editing.
setting-editing-undo-memory-mb = Undo memory
setting-editing-undo-memory-mb-help = Most memory the undo history may use.
setting-library-recent-limit = Recent files
setting-library-recent-limit-help = How many recent files are remembered.
setting-library-folders = Library folders
setting-library-folders-help = Folders whose documents the library lists, and whose positions sync between computers. Separate folders with semicolons.
setting-keyboard-character-keys = Single-key shortcuts
setting-keyboard-character-keys-help = Browse keys such as h and period. Off, dictation and typing never trigger commands.
setting-keyboard-preset = Keys
setting-keyboard-preset-help = The default keys: like NVDA's and JAWS's browse mode, or textweaver's earlier keys. Used from the next start.
choice-keyboard-preset-default = screen reader style
choice-keyboard-preset-classic = classic
setting-keyboard-digit-row = Digit row
setting-keyboard-digit-row-help = How the terminal recognizes the digit keys for heading levels. Auto, or a French AZERTY keyboard.
choice-keyboard-digit-row-auto = automatic
choice-keyboard-digit-row-azerty = AZERTY
setting-accessibility-mode = Accessibility mode
setting-accessibility-mode-help = Self-voicing speaks everything; screen reader leaves speech to your screen reader; hybrid voices reading only.
choice-accessibility-mode-self-voicing = self-voicing
choice-accessibility-mode-screen-reader = screen reader
choice-accessibility-mode-hybrid = hybrid
setting-accessibility-say-all = Say all with a screen reader
setting-accessibility-say-all-help = Continuous reading in screen-reader mode. A sentence at a time on the status line, or textweaver's voice.
choice-accessibility-say-all-screen = on the status line
choice-accessibility-say-all-voice = with textweaver's voice
setting-accessibility-quiet-screen = Quiet screen while reading
setting-accessibility-quiet-screen-help = Keep the screen still while textweaver reads aloud. On by default in hybrid mode.
choice-accessibility-quiet-screen-auto = automatic
choice-accessibility-quiet-screen-true = on
choice-accessibility-quiet-screen-false = off
setting-accessibility-cursor = Cursor
setting-accessibility-cursor-help = Where the terminal's cursor waits. On what you are working on, or on the status line.
choice-accessibility-cursor-follow = follows focus
choice-accessibility-cursor-status = on the status line
setting-export-audio-format = Audio export format
setting-export-audio-format-help = The format Export audio lists first. tw export-audio also uses it for a file name with no extension.
choice-export-audio-format-flac = FLAC
choice-export-audio-format-mp3 = MP3
choice-export-audio-format-opus = Opus
choice-export-audio-format-ogg = Ogg Vorbis
choice-export-audio-format-wav = WAV
choice-export-audio-format-m4b = M4B audiobook
choice-export-audio-format-mp4 = MP4 video with captions
setting-export-subtitle-format = Subtitle format
setting-export-subtitle-format-help = The format of subtitles written without a file name.
choice-export-subtitle-format-srt = SubRip
choice-export-subtitle-format-vtt = WebVTT
setting-export-subtitle-word-level = Word subtitles
setting-export-subtitle-word-level-help = One subtitle per word instead of caption lines.
setting-export-subtitles-with-audio = Subtitles with audio
setting-export-subtitles-with-audio-help = Always write subtitles beside exported audio.
choice-export-subtitle-format-ass = ASS karaoke
setting-export-subtitle-karaoke = Subtitle karaoke
setting-export-subtitle-karaoke-help = How subtitle lines show the word being read. Off, underlined as it is spoken (WebVTT tags), or one cue per word in bold and underline.
choice-export-subtitle-karaoke-off = Off
choice-export-subtitle-karaoke-tags = Underline as spoken
choice-export-subtitle-karaoke-lines = One cue per word
setting-export-subtitle-chapters = Chapters file
setting-export-subtitle-chapters-help = Also write a WebVTT chapters file beside the subtitles or the audio.
# Names for chapters the document leaves untitled, in audio export.
export-chapter-untitled = Audiobook
export-chapter-numbered = Chapter { $number }
# The read-along page (tw export-audio essay.md --out essay.html).
readalong-skip = Skip to the text
readalong-controls = Audio
readalong-play = Play
readalong-pause = Pause
readalong-back = Back a sentence
readalong-forward = Forward a sentence
readalong-follow = Follow along
readalong-speed = Speed
readalong-contents = Contents
readalong-play-section = Play section: { $title }
setting-reading-aids-rsvp-wpm = RSVP rate
setting-reading-aids-rsvp-wpm-help = Words per minute of rapid serial visual presentation.
setting-reading-aids-rsvp-pacing = RSVP pacing
setting-reading-aids-rsvp-pacing-help = What moves the RSVP word on: its own timer, or speech.
choice-reading-aids-rsvp-pacing-timer = its own timer
choice-reading-aids-rsvp-pacing-external = speech
setting-reading-aids-rsvp-clause-pause = RSVP clause pause
setting-reading-aids-rsvp-clause-pause-help = Extra time after a comma, colon, dash, or bracket, in percent of a word's time.
setting-reading-aids-rsvp-sentence-pause = RSVP sentence pause
setting-reading-aids-rsvp-sentence-pause-help = Extra time at the end of a sentence, in percent.
setting-reading-aids-rsvp-paragraph-pause = RSVP paragraph pause
setting-reading-aids-rsvp-paragraph-pause-help = Extra time at the end of a paragraph, in percent.
setting-reading-aids-rsvp-long-word-len = RSVP long word
setting-reading-aids-rsvp-long-word-len-help = Words longer than this many letters get extra time.
setting-reading-aids-rsvp-long-word-step = RSVP long word step
setting-reading-aids-rsvp-long-word-step-help = Extra time per letter beyond a long word's length, in percent.
setting-reading-aids-rsvp-long-word-max = RSVP long word most
setting-reading-aids-rsvp-long-word-max-help = Most extra time a long word gets, in percent.
setting-reading-aids-rsvp-show-previous = RSVP previous word
setting-reading-aids-rsvp-show-previous-help = Show the previous word too.
setting-reading-aids-rsvp-show-next = RSVP next word
setting-reading-aids-rsvp-show-next-help = Show the next word too.
setting-reading-aids-rsvp-position = RSVP position
setting-reading-aids-rsvp-position-help = Where the RSVP word appears.
choice-reading-aids-rsvp-position-top-left = top left
choice-reading-aids-rsvp-position-top-center = top center
choice-reading-aids-rsvp-position-top-right = top right
choice-reading-aids-rsvp-position-center-left = middle left
choice-reading-aids-rsvp-position-center = center
choice-reading-aids-rsvp-position-center-right = middle right
choice-reading-aids-rsvp-position-bottom-left = bottom left
choice-reading-aids-rsvp-position-bottom-center = bottom center
choice-reading-aids-rsvp-position-bottom-right = bottom right
setting-reading-aids-rsvp-font-size-pt = RSVP size
setting-reading-aids-rsvp-font-size-pt-help = Size of the RSVP word in the window.
setting-reading-aids-rsvp-lead-words = RSVP lead
setting-reading-aids-rsvp-lead-words-help = With speech pacing, show this many words ahead of the word spoken.
setting-reading-aids-bionic = Bionic reading
setting-reading-aids-bionic-help = Draw the start of each word in bold.
setting-reading-aids-bionic-options-ratio = Bionic share
setting-reading-aids-bionic-options-ratio-help = How much of each word is bold.
setting-reading-aids-bionic-options-min-word-len = Bionic shortest word
setting-reading-aids-bionic-options-min-word-len-help = Words shorter than this are left alone.
setting-reading-aids-bionic-options-skip-numbers = Bionic skips numbers
setting-reading-aids-bionic-options-skip-numbers-help = Leave words with digits alone.
setting-reading-aids-bionic-options-skip-urls = Bionic skips addresses
setting-reading-aids-bionic-options-skip-urls-help = Leave web and email addresses alone.
setting-reading-aids-bionic-options-skip-code = Bionic skips code
setting-reading-aids-bionic-options-skip-code-help = Leave code alone.
setting-reading-aids-spacing-line-height = Line height
setting-reading-aids-spacing-line-height-help = Line height as a multiple of the font size. 1.5 is the WCAG value.
setting-reading-aids-spacing-paragraph-spacing = Paragraph spacing
setting-reading-aids-spacing-paragraph-spacing-help = Space after each paragraph, in multiples of the font size.
setting-reading-aids-spacing-letter-spacing = Letter spacing
setting-reading-aids-spacing-letter-spacing-help = Extra space between letters, in multiples of the font size.
setting-reading-aids-spacing-word-spacing = Word spacing
setting-reading-aids-spacing-word-spacing-help = Extra space between words, in multiples of the font size.
setting-reading-aids-font-family = Font
setting-reading-aids-font-family-help = The window's reading font. You can also type the name of any installed font.
choice-reading-aids-font-family-system-ui = the system font
choice-reading-aids-font-family-sans = sans serif
choice-reading-aids-font-family-serif = serif
choice-reading-aids-font-family-monospace = monospace
choice-reading-aids-font-family-atkinson = Atkinson Hyperlegible
choice-reading-aids-font-family-opendyslexic = OpenDyslexic
choice-reading-aids-font-family-lexend = Lexend
setting-reading-aids-font-size-pt = Font size
setting-reading-aids-font-size-pt-help = The window's font size.
setting-reading-aids-font-weight = Font weight
setting-reading-aids-font-weight-help = 400 is regular, 700 bold.
setting-reading-aids-ruler-mode = Reading ruler
setting-reading-aids-ruler-mode-help = Mark the current line, or a band of lines.
choice-reading-aids-ruler-mode-off = off
choice-reading-aids-ruler-mode-current-line = current line
choice-reading-aids-ruler-mode-ruler = ruler
setting-reading-aids-ruler-scope = Ruler covers
setting-reading-aids-ruler-scope-help = A wrapped row, or the whole line.
choice-reading-aids-ruler-scope-row = a row
choice-reading-aids-ruler-scope-line = the whole line
setting-reading-aids-ruler-rows-above = Ruler rows above
setting-reading-aids-ruler-rows-above-help = Rows of the band above the current one.
setting-reading-aids-ruler-rows-below = Ruler rows below
setting-reading-aids-ruler-rows-below-help = Rows of the band below the current one.
setting-reading-aids-ruler-mask-outside = Ruler mask
setting-reading-aids-ruler-mask-outside-help = Dim the rows outside the band.
setting-reading-aids-syllables = Syllables
setting-reading-aids-syllables-help = Draw words split into syllables with a middle dot; speech is unchanged.
setting-reading-aids-difficult-words = Difficult words
setting-reading-aids-difficult-words-help = Underline rare words, and name them on word moves at high verbosity.
setting-reading-aids-syllable-options-separator = Syllable separator
setting-reading-aids-syllable-options-separator-help = What is drawn between syllables.
setting-reading-aids-syllable-options-left-min = Syllable first break
setting-reading-aids-syllable-options-left-min-help = Fewest letters before the first break.
setting-reading-aids-syllable-options-right-min = Syllable last break
setting-reading-aids-syllable-options-right-min-help = Fewest letters after the last break.
setting-reading-aids-syllable-options-min-word-len = Syllable shortest word
setting-reading-aids-syllable-options-min-word-len-help = Words shorter than this are never split.
setting-reading-aids-syllable-options-skip-urls = Syllables skip addresses
setting-reading-aids-syllable-options-skip-urls-help = Leave web and email addresses alone.
setting-reading-aids-syllable-options-skip-code = Syllables skip code
setting-reading-aids-syllable-options-skip-code-help = Leave code alone.
setting-preview-auto-reload = Reload the preview
setting-preview-auto-reload-help = Reload the browser preview after each save, through a small server on this computer only.
setting-preview-live = Live preview
setting-preview-live-help = With reloading on, also reload when typing pauses.
setting-lexicon-glossary = Glossary
setting-lexicon-glossary-help = Your own glossary, looked up before the dictionary: term: definition lines, or star's JSON. Not set uses glossary.txt in the settings folder.
setting-lexicon-data-file = Dictionary file
setting-lexicon-data-file-help = The define-word dictionary, lexicon-en.twlex. Not set looks beside the program.
setting-stats-enabled = Reading statistics
setting-stats-enabled-help = Count the time read aloud, the furthest point, and sessions for each document.
setting-interface-language = Interface language
setting-interface-language-help = The language of textweaver's own words, changed at once. The voice follows it when the engine has one for it; otherwise the voice stays.
choice-interface-language-en = English
choice-interface-language-es = Español
choice-interface-language-fr = Français
choice-interface-language-de = Deutsch
choice-interface-language-pt = Português
choice-interface-language-ar = العربية
setting-interface-rtl = Right-to-left display
setting-interface-rtl-help = Whether the terminal reader reorders right-to-left text for display. Automatic leaves it to terminals that do it themselves. Speech and the screen reader always get the text in reading order.
choice-interface-rtl-auto = automatic
choice-interface-rtl-on = on
choice-interface-rtl-off = off
setting-gui-announce = Announcements
setting-gui-announce-help = How the window's messages reach the screen reader, from the next start. A live region, or UI Automation notifications (Windows only).
choice-gui-announce-live = live region
choice-gui-announce-uia = UI Automation notifications
setting-gui-header = Show the header
setting-gui-header-help = Show the bar of Open, Font, Edit, Settings and Commands above the document. Off, the commands keep their keys and menu items.
setting-gui-toolbar = Show the toolbar
setting-gui-toolbar-help = Show the bar of Play, Stop and the reading buttons. Off, the commands keep their keys and menu items.
setting-gui-auto-hide-menu = Hide the menu bar
setting-gui-auto-hide-menu-help = Windows: hide the window's menu bar until Alt or F10 shows it. It hides again when the menu closes. No effect on Linux, whose menus are the F10 list, or on macOS.
setting-gui-speak-messages = Speak textweaver's messages
setting-gui-speak-messages-help = When textweaver reads aloud, also say its messages, typing and cursor moves in its voice, for reading by ear without a screen reader.
setting-gui-sidebar = Panel beside the document
setting-gui-sidebar-help = The panel the window shows beside the document. None, the Contents (the headings), or the Notes. The panel keys change it, and the window remembers the last one.
choice-gui-sidebar-off = none
choice-gui-sidebar-contents = Contents
choice-gui-sidebar-notes = Notes

## Units, said after a number.

settings-unit-words-per-minute =
    { $n ->
        [one] word per minute
       *[other] words per minute
    }
settings-unit-percent = percent
settings-unit-semitones =
    { $n ->
        [one] semitone
       *[other] semitones
    }
settings-unit-milliseconds =
    { $n ->
        [one] millisecond
       *[other] milliseconds
    }
settings-unit-words =
    { $n ->
        [one] word
       *[other] words
    }
settings-unit-places =
    { $n ->
        [one] place
       *[other] places
    }
settings-unit-columns =
    { $n ->
        [one] column
       *[other] columns
    }
settings-unit-characters =
    { $n ->
        [one] character
       *[other] characters
    }
settings-unit-lines =
    { $n ->
        [one] line
       *[other] lines
    }
settings-unit-seconds =
    { $n ->
        [one] second
       *[other] seconds
    }
settings-unit-steps =
    { $n ->
        [one] step
       *[other] steps
    }
settings-unit-megabytes =
    { $n ->
        [one] megabyte
       *[other] megabytes
    }
settings-unit-files =
    { $n ->
        [one] file
       *[other] files
    }
settings-unit-letters =
    { $n ->
        [one] letter
       *[other] letters
    }
settings-unit-points =
    { $n ->
        [one] point
       *[other] points
    }
settings-unit-rows =
    { $n ->
        [one] row
       *[other] rows
    }
settings-unit-minutes =
    { $n ->
        [one] minute
       *[other] minutes
    }

## Settings sections.

section-speech = Speech
section-highlight = Highlight
section-normalization = Speaking text
section-reading = Reading
section-display = Display
section-editing = Editing
section-library = Library
section-keyboard = Keyboard
section-accessibility = Accessibility
section-export = Export
section-braille = Braille
section-reading-aids = Reading aids
section-preview = Preview
section-lexicon = Define word
section-stats = Reading statistics
section-interface = Interface
section-gui = Window

## Edit mode: entering, leaving, saving, and typing.

# $key makes a new document.
edit-no-document = No document to edit. Press { $key } for a new one.
# $line is the line at the caret, as echoed.
edit-mode-on-brief = Edit mode on. { $line }
# $save and $finish are the keys that save and leave edit mode; $line is
# the line at the caret.
edit-mode-on = Edit mode on. Save: { $save }. Finish: { $finish }. { $line }
# Keep the letters s, d and c: they are the keys that answer.
edit-unsaved-question = { $title } has unsaved changes. Save, discard, or cancel? Press s, d, or c, or Up and Down and Enter. Escape cancels.
edit-save-changes-title = Save changes to { $title }?
edit-choice-save = Save, then continue
edit-choice-discard = Discard the changes
edit-choice-cancel = Cancel, keep editing
edit-save-failed = Could not save: { $error } Still editing. Try Save as.
# The Save As prompt; $path is the suggested file.
edit-save-as-label = Save as, Enter for { $path }
# $name is a file name. Keep the letters y and n.
edit-file-exists-question = { $name } already exists. Replace it? y or n
edit-mode-off = Edit mode off.
edit-mode-off-discarded = Changes discarded. Edit mode off.
# The title of a new, unsaved document.
edit-untitled = Untitled
edit-new-document = New document ready for editing.
# $key turns on edit mode.
edit-nothing-to-save = Nothing to save. Turn on edit mode with { $key } to make changes.
# $key turns on edit mode; $what is what the user tried to do.
edit-not-editing =
    { $what ->
        [type] Turn on edit mode with { $key } to type.
        [change-text] Turn on edit mode with { $key } to change the text.
        [delete-text] Turn on edit mode with { $key } to delete text.
        [undo] Turn on edit mode with { $key } to undo.
        [redo] Turn on edit mode with { $key } to redo.
        [replace-text] Turn on edit mode with { $key } to replace text.
        [insert] Turn on edit mode with { $key } to insert into the text.
        [cut-text] Turn on edit mode with { $key } to cut text.
        [move-cells] Turn on edit mode with { $key } to move between table cells.
        [delete-words] Turn on edit mode with { $key } to delete words.
        [paste] Turn on edit mode with { $key } to paste.
        [citation] Turn on edit mode with { $key } to insert a citation.
        [bibliography] Turn on edit mode with { $key } to insert a bibliography.
       *[format] Turn on edit mode with { $key } to format text.
    }
# A paste: $n characters.
edit-pasted =
    { $n ->
        [one] Pasted 1 character.
       *[other] Pasted { $n } characters.
    }
# $start is how the pasted text starts.
edit-pasted-start =
    { $n ->
        [one] Pasted 1 character: { $start }
       *[other] Pasted { $n } characters: { $start }
    }
edit-insert-failed = Could not insert: { $error } The text is unchanged.
# $start and $end are character positions, $len the text's length.
edit-range-out-of-text = Cannot change characters { $start } to { $end }: the text has { $len }.
edit-change-failed = Could not change the text: { $error } The text is unchanged.
edit-delete-failed = Could not delete: { $error } The text is unchanged.
edit-list-ended = List ended.
# Said when Enter continues a bulleted list.
edit-bullet = bullet
edit-table-divider = table header divider
edit-end-of-line-stop = End of line.
edit-start-of-line-stop = Start of line.
edit-end-of-line-content = end of line

## Edit mode: formatting, undo, tables, images, and replace.

# $what names the formatting command; $level is a heading level, and
# $cols and $rows a table's size.
edit-format-done =
    { $what ->
        [bold] Bold.
        [italic] Italic.
        [underline] Underline.
        [strikethrough] Strikethrough.
        [code] Code.
        [code-block] Code block.
        [link] Link.
        [bulleted-list] Bulleted list.
        [numbered-list] Numbered list.
        [block-quote] Block quote.
        [horizontal-rule] Horizontal rule inserted.
        [table-row] Table row added.
        [heading-level] Heading level { $level }.
        [table] Inserted a table, { $cols } columns by { $rows } rows.
       *[heading] Heading.
    }
# The command toggled its markup off.
edit-format-removed =
    { $what ->
        [bold] Bold removed.
        [italic] Italic removed.
        [underline] Underline removed.
        [strikethrough] Strikethrough removed.
        [code] Code removed.
        [code-block] Code block removed.
        [link] Link removed.
        [bulleted-list] Bulleted list removed.
        [numbered-list] Numbered list removed.
        [block-quote] Block quote removed.
        [horizontal-rule] Horizontal rule removed.
        [table-row] Table row removed.
        [heading-level] Heading level { $level } removed.
        [table] Table removed.
       *[heading] Heading removed.
    }
edit-format-unchanged =
    { $what ->
        [bold] Bold: nothing changed.
        [italic] Italic: nothing changed.
        [underline] Underline: nothing changed.
        [strikethrough] Strikethrough: nothing changed.
        [code] Code: nothing changed.
        [code-block] Code block: nothing changed.
        [link] Link: nothing changed.
        [bulleted-list] Bulleted list: nothing changed.
        [numbered-list] Numbered list: nothing changed.
        [block-quote] Block quote: nothing changed.
        [horizontal-rule] Horizontal rule: nothing changed.
        [table-row] Table row: nothing changed.
        [heading-level] Heading level { $level }: nothing changed.
        [table] Table: nothing changed.
       *[heading] Heading: nothing changed.
    }
# Added after a formatting message; $text is the start of the selection.
edit-format-selected = Selected: { $text }
edit-heading-level-now = Heading level { $level }.
# $line is the line at the caret after the undo or redo.
edit-undo-redo =
    { $what ->
        [undo] Undo.
       *[redo] Redo.
    }
edit-undo-redo-line =
    { $what ->
        [undo] Undo. { $line }
       *[redo] Redo. { $line }
    }
edit-nothing-to-undo = Nothing to undo.
edit-nothing-to-redo = Nothing to redo.
edit-not-a-table-size = Not a table size: { $text }. Type columns and rows, for example 3 by 2.
# $name is the image's file name.
edit-image-inserted = Inserted image { $name }. Its description is selected; type to replace it.
edit-image-failed = Could not insert the image: { $error } The text is unchanged.
# $query is the text to find.
common-no-matches = No matches for { $query }.
# $n matches of $query were found; the replacement is asked next.
edit-replace-with =
    { $n ->
        [one] 1 match for { $query }. Replace with?
       *[other] { $n } matches for { $query }. Replace with?
    }

## Edit mode: autosave and recovering unsaved work.

common-recovery-write-failed = Could not write the recovery copy: { $error }. Save soon; { -brand } will keep trying.
common-recovery-writing-again = The recovery copy is being written again.
# $title is the document; $when is how long ago its work was saved.
edit-recovery-offer = { -brand } closed with unsaved changes to { $title }, saved { $when }. Recover them now? Up and Down choose, Enter confirms.
edit-recovery-title = Recover unsaved work in { $title }?
edit-recovery-yes = Yes, recover { $title } and keep editing
edit-recovery-no = No, discard the unsaved changes
edit-recovery-discarded = Discarded the unsaved changes to { $title }.
edit-recovered = Recovered unsaved work in { $title }. Remember to save.
edit-recovery-postponed = Recovery postponed. The unsaved work will be offered again next time.

## Find and replace, one match at a time.

# $title is replace-match-title. Keep the letters r, s and a: they are the
# keys that answer.
replace-match-question = { $title }. The line: { $context }. Press r to replace, s to skip, a to replace all, Escape to stop.
# The replace list's title when no match is being asked about.
replace-title = Replace
# $n is this match's number, $total the number of matches, $line the line
# number, $found the matched text, and $result what it becomes (both
# shortened); the -removed form is for an empty replacement. $context in
# replace-match-question is the text of the match's line.
replace-match-title = Match { $n } of { $total }, line { $line }: { $found } becomes { $result }
replace-match-title-removed = Match { $n } of { $total }, line { $line }: { $found } is removed
replace-item-this = Replace this one
replace-item-skip = Skip this one
replace-item-rest = Replace all the rest
# $state is common-on or common-off.
replace-item-match-case = Match case: { $state }
replace-item-whole-words = Whole words only: { $state }
replace-failed = Could not replace: { $error } The text is unchanged.
# Said after switching match case; $state is common-on or common-off, and
# $n is the number of matches now.
replace-match-case-now =
    { $n ->
        [one] Match case { $state }. 1 match.
       *[other] Match case { $state }. { $n } matches.
    }
replace-whole-words-now =
    { $n ->
        [one] Whole words only { $state }. 1 match.
       *[other] Whole words only { $state }. { $n } matches.
    }
# $query is the text that was searched for.
replace-replaced =
    { $n ->
        [one] Replaced 1 match.
       *[other] Replaced { $n } matches.
    }
replace-replaced-skipped = Replaced { $n }, skipped { $skipped }.
replace-stopped = Stopped. Replaced { $n }, skipped { $skipped }.
# Find and replace with regular expressions (B1-fr). $state is common-on
# or common-off; $n is the number of matches now.
replace-item-regex = Regular expression: { $state }
replace-item-across-lines = Across lines: { $state }
replace-regex-now =
    { $n ->
        [one] Regular expression { $state }. 1 match.
       *[other] Regular expression { $state }. { $n } matches.
    }
replace-across-lines-now =
    { $n ->
        [one] Across lines { $state }. 1 match.
       *[other] Across lines { $state }. { $n } matches.
    }
# $problem is search-invalid-pattern: switching the option would make the
# pattern invalid.
replace-option-refused = { $problem } The option is unchanged.
# Asked once before replacing all the rest; $n is how many. Keep y and n.
replace-all-question =
    { $n ->
        [one] Replace the 1 remaining match? y or n
       *[other] Replace all { $n } remaining matches? y or n
    }
replace-all-declined = Nothing replaced.
# The search options list, and the options as named in search-options-on.
search-options-title = Search options
search-option-match-case = match case
search-option-whole-words = whole words
search-option-regex = regular expression
search-option-across-lines = across lines
# Said as Find or Replace opens when an option is on; $list joins the
# options' names with commas.
search-options-on = Options on: { $list }.
# $at is the character where the pattern fails, counting from 1; $reason
# is the regular expression engine's own explanation (in English).
search-invalid-pattern = Invalid pattern at character { $at }: { $reason }.
search-invalid-pattern-anywhere = Invalid pattern: { $reason }.

## Saving in the background.

writes-still-saving = Still saving. Please wait.
writes-not-written-in-time = Some changes could not be written in time: the disk is not answering.
# $error is the system's reason.
writes-save-failed = Could not save: { $error }. Still editing.
# $name is the bookmark's name, $pct where it is.
writes-bookmark-not-saved = Bookmark { $name } is set for now, but could not be saved: { $error } Check that the data folder can be written to.
# $name is the saved file's name.
writes-saved = Saved { $name }. Still editing.

## Files changed on disk. $name is a file name. Keep the letters y and
## n: they are the keys that answer.

disk-replace-question = { $name } already exists. Replace it? y or n
# A prompt label, also said with a full stop after it.
disk-not-replaced = Not replaced. Type another name
# $key is the key for Save As.
disk-not-saved = Not saved. Still editing. Save As, { $key }, keeps both versions.
disk-kept-open-version = Kept the open version.
disk-overwrite-question = { $name } changed on disk since you opened it. Save over those changes? y or n
disk-reload-question = { $name } changed on disk. Reload it? y or n

## Marks found again after a file changed outside textweaver.

relocate-reading-position = your reading position
relocate-bookmarks =
    { $n ->
        [one] 1 bookmark
       *[other] { $n } bookmarks
    }
relocate-notes =
    { $n ->
        [one] 1 note
       *[other] { $n } notes
    }
relocate-highlights =
    { $n ->
        [one] 1 highlight
       *[other] { $n } highlights
    }
# Lists of relocate-* items: "a and b", and "a, b, and c", where $rest is
# every item but the last, joined by commas.
relocate-join-two = { $a } and { $b }
relocate-join-more = { $rest }, and { $last }
# $items is a list of the items above; $n how many marks it counts in all.
relocate-moved =
    { $n ->
        [one] { $items } was moved to match
       *[other] { $items } were moved to match
    }
relocate-lost =
    { $n ->
        [one] { $items } could not be found and is marked
       *[other] { $items } could not be found and are marked
    }
# $clauses are relocate-moved and relocate-lost, joined by a comma.
relocate-changed = The file changed; { $clauses }.

## New documents from templates.

# The built-in templates' names.
templates-essay = Essay
templates-report = Report
templates-notes = Notes
# One of the user's own templates in the list; $name is its file name.
templates-yours = { $name }, your template
# $folder is where the user's own templates go.
templates-intro =
    { $n ->
        [one] New document from a template, 1 template. Enter chooses. Your own templates go in { $folder }.
       *[other] New document from a template, { $n } templates. Enter chooses. Your own templates go in { $folder }.
    }
# The title given when none is typed.
templates-untitled = Untitled
# $template is the template's name, $title the document's, $date today's date (2026-09-26).
templates-created = New document from the { $template } template: { $title }. Dated { $date }. The cursor is where the writing starts. Remember to save.

## Markdown structure said in edit mode, before a line's text or as it
## is typed.

mdline-heading-level = heading level { $level }
mdline-bullet = bullet
# A numbered list item; $n is its number.
mdline-item = item { $n }
# $item is mdline-bullet or mdline-item.
mdline-task-done = { $item }, task done
mdline-task-not-done = { $item }, task not done
mdline-table-row = table row
mdline-quote = quote
mdline-code-fence = code fence
mdline-task = task
# Said as "1. " is typed at the start of a line; $n is the number as typed.
mdline-numbered-item = numbered item { $n }

## Moving through tables by row and cell. $dir is next (forward) or
## previous (backward).

common-not-in-table = Not in a table.
tables-edge-of-table =
    { $dir ->
        [next] End of table.
       *[previous] Start of table.
    }
tables-edge-of-row =
    { $dir ->
        [next] End of row.
       *[previous] Start of row.
    }
# $cell is the cell's text, after its column header and a colon when it has one.
tables-header-row = Header row, { $cell }
tables-row = Row { $row }, { $cell }
# High verbosity: $message is what the move said, then where it is.
tables-with-position = { $message }. Row { $row } of { $rows }, column { $col } of { $cols }
# Say Position in a table.
tables-position = Table, row { $row } of { $rows }, column { $col } of { $cols }.

## Authoring quick wins: word count, links, clipboard, table cells,
## deleting words, and cycling settings.

# Code block languages said with ordinary words; proper names such as
# Python are not translated.
authoring-language-jsx = JavaScript with JSX
authoring-language-tsx = TypeScript with JSX
authoring-language-shell = shell
authoring-language-batch = Windows batch
authoring-language-c-header = C header
authoring-language-cpp = C plus plus
authoring-language-csharp = C sharp
authoring-language-diff = diff
authoring-language-plain-text = plain text
authoring-grammar-not-in-build = Grammar checking is not in this version.
# $count is $n with thousands separators.
authoring-word-count-selection =
    { $n ->
        [one] 1 word in the selection.
       *[other] { $count } words in the selection.
    }
authoring-word-count-document =
    { $n ->
        [one] 1 word in the document.
       *[other] { $count } words in the document.
    }
# $text is the link's text.
authoring-link-address = Link address: { $url }
authoring-link-named-address = Link { $text }, address: { $url }
authoring-no-link = No link at the cursor.
authoring-typing-echo =
    { $echo ->
        [characters-and-words] Typing echo: characters and words.
        [characters] Typing echo: characters.
        [words] Typing echo: words.
       *[none] Typing echo: none.
    }
authoring-nothing-to-copy = Nothing selected to copy.
# $text is the first words of what was copied.
authoring-copied = Copied: { $text }
authoring-copied-sentence = Copied the sentence: { $text }
authoring-nothing-to-cut = Nothing selected to cut.
authoring-cut = Cut: { $text }
# $dir is next (moving forward) or previous.
authoring-table-edge =
    { $dir ->
        [next] End of table.
       *[previous] Start of table.
    }
# A column with no header text.
authoring-table-column = column { $n }
# Moving into a new row: $header is the column's header, $content the cell.
authoring-table-cell-row = Row { $row }. { $header }: { $content }
authoring-nothing-to-select = Nothing to select.
authoring-selected-all =
    { $n ->
        [one] Selected all, 1 word.
       *[other] Selected all, { $count } words.
    }
authoring-space-deleted = Space deleted.
# $text is the word deleted.
authoring-deleted = { $text } deleted.
# $key is the terminal's own paste key.
authoring-nothing-copied = Nothing copied in { -brand } yet. Use your terminal's paste, for example { $key }.
authoring-verbosity =
    { $level ->
        [low] Verbosity: low.
        [high] Verbosity: high.
       *[normal] Verbosity: normal.
    }
authoring-punctuation =
    { $level ->
        [none] Punctuation: none.
        [all] Punctuation: all.
       *[some] Punctuation: some.
    }

## Markdown lint (edit mode). A problem is said after "Lint: ".

lint-heading-level = heading level { $level } after level { $prev }; use level { $use }.
# $reference is the link reference's name.
lint-link-reference = link reference { $reference } has no definition.
lint-bare-url = bare web address; put it in angle brackets or make it a link with a name.
# $marker and $used are bullet names: lint-marker-dash and the others.
lint-list-marker = list marker { $marker }; this list uses { $used }.
lint-marker-dash = dash
lint-marker-star = star
lint-marker-plus = plus
lint-marker-other = other
# $n is how many tabs and spaces there are.
lint-trailing-tabs-empty-line = tabs or spaces on an empty line.
lint-trailing-tabs-line-end = tabs or spaces at the end of the line.
lint-trailing-spaces-empty-line =
    { $n ->
        [one] 1 space on an empty line.
       *[other] { $n } spaces on an empty line.
    }
lint-trailing-spaces-line-end =
    { $n ->
        [one] 1 space at the end of the line.
       *[other] { $n } spaces at the end of the line.
    }
# $key turns on edit mode.
lint-not-editing = Lint checks the Markdown you write. Turn on edit mode with { $key } first.
lint-not-markdown = Lint checks Markdown, and this document is not Markdown.
lint-none = No lint problems.
# $count is $n with thousands separators.
lint-no-more =
    { $n ->
        [one] No more lint problem. 1 lint problem in all.
       *[other] No more lint problem. { $count } lint problems in all.
    }
lint-no-earlier =
    { $n ->
        [one] No earlier lint problem. 1 lint problem in all.
       *[other] No earlier lint problem. { $count } lint problems in all.
    }
# $message is one of the problems above.
lint-said = Lint: { $message }
# Added at high verbosity.
common-line = Line { $line }.

## Grammar checking (Harper). $message is Harper's own message, in English.

# $words are the words the problem is about.
grammar-said = Grammar: { $message } The words: { $words }.
# Said after grammar-said when the first fix removes the words.
grammar-fix-remove = Fix: remove them.
grammar-fix = Fix: { $fix }.
# A fix in the fixes list that removes the words.
grammar-remove-the-words = Remove the words
grammar-none = No grammar problems found.
# $count is $n with thousands separators.
grammar-no-more =
    { $n ->
        [one] No more grammar problem. 1 grammar problem in all.
       *[other] No more grammar problem. { $count } grammar problems in all.
    }
grammar-no-earlier =
    { $n ->
        [one] No earlier grammar problem. 1 grammar problem in all.
       *[other] No earlier grammar problem. { $count } grammar problems in all.
    }
# $key opens the fixes list.
grammar-lists-fixes = { $key } lists fixes.
# Added at high verbosity.
# $described is grammar-said (and its fix) without the last full stop.
grammar-no-fix = { $described } No fix to offer.
grammar-fixes =
    { $n ->
        [one] { $words }: 1 fix.
       *[other] { $words }: { $n } fixes.
    }
grammar-fixes-edit =
    { $n ->
        [one] { $words }: 1 fix. Enter makes the change.
       *[other] { $words }: { $n } fixes. Enter makes the change.
    }
common-left-as-is = Left as it is.
# $fix is the fix chosen; $key turns on edit mode.
grammar-fix-not-editing = { $fix }. Turn on edit mode with { $key } to change the text.
grammar-removed = Removed.
grammar-changed = Changed to { $fix }.
grammar-change-failed = Could not change the text: { $error } Nothing was changed.

## Spell checking.

# Said for an apostrophe when a word is spelled out letter by letter.
spell-apostrophe = apostrophe
spell-not-available = Spell checking is not available: this version has no word list.
spell-none-found = No misspellings found.
# $count is $n with thousands separators.
spell-no-more =
    { $n ->
        [one] No more misspelling. 1 possible misspelling in all.
       *[other] No more misspelling. { $count } possible misspellings in all.
    }
spell-no-earlier =
    { $n ->
        [one] No earlier misspelling. 1 possible misspelling in all.
       *[other] No earlier misspelling. { $count } possible misspellings in all.
    }
# Added at high verbosity.
spell-no-misspelled-word = No misspelled word at the cursor.
# $word is the misspelled word; $n how many suggestions follow.
spell-suggestions =
    { $n ->
        [0] { $word }: no suggestions.
        [one] { $word }: 1 suggestion.
       *[other] { $word }: { $n } suggestions.
    }
spell-suggestions-edit =
    { $n ->
        [0] { $word }: no suggestions. Enter replaces the word.
        [one] { $word }: 1 suggestion. Enter replaces the word.
       *[other] { $word }: { $n } suggestions. Enter replaces the word.
    }
# $word is the suggestion chosen; $key turns on edit mode.
spell-replace-not-editing = { $word }. Turn on edit mode with { $key } to change the text.
spell-replaced = Replaced with { $word }.
spell-replace-failed = Could not replace: { $error } The word is unchanged.
spell-added-for-session = Added { $word } to your word list for this session.
spell-added = Added { $word } to your word list.
spell-save-failed = Could not save your word list: { $error } The word is known until you quit.
# After a save; $count is $n with thousands separators.
spell-count =
    { $n ->
        [0] No misspellings.
        [one] 1 possible misspelling.
       *[other] { $count } possible misspellings.
    }

## The terminal reader's startup.

# $wanted is the speech backend asked for, $backend the one used instead.
tui-setup-backend-unavailable = Speech engine { $wanted } is not available; using { $backend }.
tui-setup-speech-failed = Speech could not start ({ $error }); running silently.
speech-engine-fallback = { $failed } could not start; { $engine } is speaking instead.
speech-engine-fallback-silent = { $failed } could not start, and no other speech engine is available; { -brand } stays silent.
tui-setup-cannot-save = Cannot save settings or positions: { $error } Reading works. Changes are lost on exit.
tui-setup-keymap-ignored = Keymap file ignored: { $error } The default keys are used. Fix the file, then restart.
# The first-run welcome. Each value names the key for an action: $play
# reads and pauses, $stop stops, $heading moves to the next heading,
# $help opens the help, $quit quits.
tui-setup-welcome = Welcome to { -brand }. { $open } opens a document. { $play } starts and pauses reading, and { $stop } stops. { $palette } lists every command. { $help } opens the help.
# The window's first-run welcome on Windows, where the menu bar is
# hidden until Alt or $menu (F10) shows it: the keys of
# tui-setup-welcome, and the menus.
gui-setup-welcome-menus = Welcome to { -brand }. { $open } opens a document. { $play } plays and pauses, { $stop } stops. { $palette } lists every command, Alt or { $menu } the menus. { $help } opens the help.
# Said at startup without a document. $open, $new, and $help name the
# keys for Open, New Document, and Help.
tui-setup-no-document = No document is open. Press { $open } to open one, { $new } for a new one, or { $help } for help.

## The terminal reader's launch.

# $name is the file asked for on the command line; $error says why.
tui-could-not-open = Could not open { $name }: { $error }

## Copying in the terminal reader.

tui-clip-system = Copied with the system clipboard, because this terminal cannot take copied text.
# $error is why: tui-clip-not-available, tui-clip-not-built, or the
# system's own message.
tui-clip-failed = Could not copy with the system clipboard: { $error }. Sent to the terminal instead.
tui-clip-not-available = the system clipboard is not available
tui-clip-not-built = this version has no system clipboard

## The terminal reader's screen.

# The title line's start; $title is the document's title or
# tui-title-no-document.
tui-title = { -brand }: { $title }
tui-title-no-document = no document
# The screen without a document. $keys names the keys for the action.
tui-empty-open = Open one: { $keys }.
tui-empty-help = Help: { $keys }.
tui-empty-quit = Quit: { $keys }.
# The key hints while a yes-or-no question waits. y, n, and a are the
# answer keys the reader takes; $escape names the Escape key.
tui-hints-confirm = y yes  n or a no  { $escape } no
# Key hint labels, each shown after its key on the bottom line.
tui-hint-play = play
tui-hint-sentence = sentence
tui-hint-faster = faster
tui-hint-slower = slower
tui-hint-close-rsvp = close RSVP
tui-hint-quit = quit
tui-hint-save = save
tui-hint-finish = finish
tui-hint-undo = undo
tui-hint-bold = bold
tui-hint-heading = heading
tui-hint-commands = commands
tui-hint-next-line = next line
tui-hint-previous-line = previous line
tui-hint-again = again
tui-hint-read-on = read on
tui-hint-leave = leave
tui-hint-paragraph = paragraph
tui-hint-find = find
tui-hint-mark = mark
tui-hint-lines = lines
tui-hint-keys = keys
tui-hint-choose = choose
tui-hint-close = close
tui-hint-back = back
# The list overlay's border: $n is the focused item's number, $count
# the number of items.
tui-list-title = { $n } of { $count }, { $title }

## Long texts are summarized. $count is a number of characters, $first and
## $last the words at each end; $change says what happened to the text.

text-summary =
    { $change ->
        [selected] { $count } characters selected
        [unselected] { $count } characters unselected
        [copied] { $count } characters copied
        [cut] { $count } characters cut
       *[deleted] { $count } characters deleted
    }
text-summary-range =
    { $change ->
        [selected] { $count } characters selected, from { $first } to { $last }
        [unselected] { $count } characters unselected, from { $first } to { $last }
        [copied] { $count } characters copied, from { $first } to { $last }
        [cut] { $count } characters cut, from { $first } to { $last }
       *[deleted] { $count } characters deleted, from { $first } to { $last }
    }
voice-character-keys-on = Single-key shortcuts on.
voice-character-keys-off = Single-key shortcuts off.

## Words typed at the go-to prompt, besides a number: they must be the
## words prompt-go-to and goto-not-a-target tell the user to type.

goto-word-start = start
goto-word-end = end

language-voices-loading = The voice list is still loading, so the current voice keeps speaking.

## The window (GUI)

gui-open-title = Open a document
gui-open-documents = Documents textweaver reads
gui-open-all-files = All files
gui-open-no-dialog = The system's file chooser did not open. Type the path of the document instead.
gui-text-size = Text size { $size } points.
gui-text-size-largest = Text size { $size } points, the largest.
gui-text-size-smallest = Text size { $size } points, the smallest.
gui-font = Font: { $family }.
gui-font-list = Font

## The Braille pass: pages in paged documents such as a PDF.
## $page and $n are page numbers, $label a printed page label such as iv,
## $pages the number of pages. Keep the page first: a 40-cell Braille
## display shows the start of the line.

# On the title line, in place of status-position.
status-page = page { $page } of { $pages }
status-page-labelled = page { $label }, { $n } of { $pages }
# The title line's percentage, after the place (in edit and Speech Cursor
# modes after the mode, "modified" and the reading state); % is shown, not said.
status-percent = { $pct }%
# Said first by the position report.
pages-position = Page { $page } of { $pages }.
pages-position-labelled = Page { $label }, { $n } of { $pages }.
pages-none = This document has no pages.
pages-no-such-page = No page { $page }. Pages go from 1 to { $pages }.
pages-label = Page { $label }
# An outline item: $text is the page's first words.
pages-outline-item = Page { $label }: { $text }
lists-pages-title =
    { $n ->
        [one] Pages, { $n } page
       *[other] Pages, { $n } pages
    }
lists-pages-title-filtered = Pages, { $shown } of { $n } match { $filter }
lists-pages-intro =
    { $n ->
        [one] Pages, { $n } page. Type to filter, Enter goes to a page, Escape closes.
       *[other] Pages, { $n } pages. Type to filter, Enter goes to a page, Escape closes.
    }
# $heading is the outline item of the page the cursor is on.
lists-pages-here = You are on { $heading }.
lists-filter-cleared-pages =
    { $n ->
        [one] Filter cleared, { $n } page.
       *[other] Filter cleared, { $n } pages.
    }
lists-filter-none-pages = No pages match { $query }. Backspace removes letters.
lists-filter-matched-pages =
    { $n ->
        [one] { $n } page match.
       *[other] { $n } pages match.
    }
# The go-to prompt in a paged document: a bare number is a page there.
prompt-go-to-pages = Go to page, or line 12, percent, start, or end
goto-not-a-target-pages = Not a go-to target: { $text }. Type a page number, line and a number, a percentage such as 50%, start, or end.
# Typed before a page label at the go-to prompt; page and p always work.
goto-word-page = page

## The library's filter, the dictionary, and speed presets.

# The library list filtered: $shown of $n documents match $filter.
library-title-filtered = Library, { $shown } of { $n } match { $filter }
# The filter was emptied: $n documents are shown.
library-filter-cleared =
    { $n ->
        [one] Filter cleared, { $n } document.
       *[other] Filter cleared, { $n } documents.
    }
# No document matches the filter $query.
library-filter-none = No documents match { $query }. Backspace removes letters.
# $n documents match the filter.
library-filter-matched =
    { $n ->
        [one] { $n } document matches.
       *[other] { $n } documents match.
    }
# Said once when define word is used while the dictionary file is still opening.
define-still-loading = Dictionary still loading.
# Settings: the DECtalk and Piper sections, the rate and pitch per voice, and the template author.
setting-speech-dectalk-library = DECtalk library
setting-speech-dectalk-library-help = The DECtalk library to load; not set searches the usual places.
setting-speech-piper-voices = Piper voices folder
setting-speech-piper-voices-help = The folder of Piper voices; not set uses the piper folder in textweaver's data folder.
setting-speech-piper-voice = Piper voice
setting-speech-piper-voice-help = The Piper voice to start with, by id; not set takes the first installed.
setting-speech-piper-phonemizer = Piper phonemizer
setting-speech-piper-phonemizer-help = How Piper turns text into sounds. The espeak-ng library when installed, that library, or textweaver's own.
choice-speech-piper-phonemizer-auto = automatic
choice-speech-piper-phonemizer-library = espeak-ng library
choice-speech-piper-phonemizer-rust = textweaver's own
setting-speech-voice-params = Rate and pitch per voice
setting-speech-voice-params-help = The rate and pitch each voice was last used at. Choosing a voice again brings them back.
setting-editing-author = Author
setting-editing-author-help = The author written into new documents made from a template; empty leaves it blank.

## The window (GUI): drawn labels, hints, and questions.
## Keep the letters Y and N: they are the keys that answer.

gui-yes = Yes
gui-no = No
gui-answer-delete = Delete
gui-answer-remove = Remove
gui-answer-replace = Replace
gui-question-hint = Y answers yes, N answers no, Escape answers no.
gui-button-open = Open…
gui-button-font = Font…
gui-button-edit = Start editing
gui-button-finish-editing = Finish editing
gui-button-settings = Settings…
gui-button-commands = Commands…
gui-button-play = Play
gui-button-pause = Pause
gui-button-stop = Stop
gui-button-previous-sentence = Previous sentence
gui-button-next-sentence = Next sentence
gui-button-slower = Slower
gui-button-faster = Faster
# Button descriptions, read after the name: at most 40 cells, saying
# what the name does not. The command's full help stays in F1.
gui-hint-open = Choose a document to read.
gui-hint-font = Choose the font of the document text.
gui-hint-edit = Switch between reading and editing.
gui-hint-settings = Every option, with its help.
gui-hint-commands = Run any command by its name.
gui-hint-play = Read from the current word, or pause.
gui-button-close = Close
gui-toolbar-reading = Reading
gui-document = Document
gui-document-titled = { $title }, document
gui-list-hint = Enter chooses, Escape closes.
gui-sidebar-contents = Contents
gui-sidebar-notes = Notes
gui-sidebar-open =
    { $n ->
        [one] { $panel } open, 1 item.
       *[other] { $panel } open, { $n } items.
    }
gui-sidebar-closed = { $panel } closed.
gui-header-shown = Header shown.
gui-header-hidden = Header hidden. Its commands keep their keys.
gui-toolbar-shown = Toolbar shown.
gui-toolbar-hidden = Toolbar hidden. Its commands keep their keys.
gui-sidebar-no-headings = No headings.
gui-sidebar-no-notes = No notes.
gui-sidebar-current = { $item }, current
gui-sidebar-hint = Enter goes there. { $leave } goes and returns. Escape returns.
gui-settings-sections = Sections
gui-settings-form = { $section } settings
gui-settings-saved-hint = Changes take effect and are saved at once.
gui-settings-close-help = Close the settings. Every change is already saved.
gui-settings-recent = Recently changed
gui-settings-matching = Matching { $filter }
gui-settings-table = { $label } is a table. Edit it in settings.toml.
gui-setting-new-value = New value for { $label }
gui-setting-value-hint = Press Enter to accept, or Escape to go back.
gui-prompt-path-hint = Type the path of a document, then press Enter. Tab completes it; Up and Down recall earlier ones.
gui-prompt-hint = Press Enter to accept, or Escape to cancel. Up and Down recall earlier answers.
gui-palette-filter = Type to filter the commands
gui-palette-list = Commands
gui-palette-hint = Enter runs the first match; Tab moves to the list; F1 explains a command.
gui-open-failed = Could not open { $name }: { $error }
gui-uia-unavailable = UI Automation notifications exist only on Windows; using the live region.
gui-graphics-failed = The window could not start its graphics. The terminal reader, textweaver, needs none.
gui-crashed = textweaver stopped after an internal error.
gui-crashed-saved = Your place was saved, and unsaved edits will be offered for recovery next time.
gui-rsvp = RSVP
gui-rsvp-playing = RSVP playing, word { $n } of { $total }
gui-rsvp-paused = RSVP paused, word { $n } of { $total }
gui-rsvp-finished = RSVP finished, word { $n } of { $total }
gui-settings-section-item =
    { $section }, { $n ->
        [one] 1 setting
       *[other] { $n } settings
    }
gui-palette-count =
    { $n ->
        [0] No commands match.
        [one] 1 command.
       *[other] { $n } commands.
    }
gui-settings-form-help = Up and Down move between settings. Left and Right change one. Enter types a new value. Delete puts the default back. { $next } and { $previous } change the section. Type to filter. F1 says the help.
gui-settings-press-enter = Press Enter to type a new value for { $label }.
gui-font-built-in = { $family } (built in)
## Summaries and difficult-word definitions.

action-summarize = Summarize the selection, the chapter, or the document: its most central sentences in a list; Enter goes to one
# The summary list's title: $n sentences of the whole document.
summary-title =
    { $n ->
        [one] Summary, { $n } sentence
       *[other] Summary, { $n } sentences
    }
# The summary of the chapter at the cursor.
summary-title-chapter =
    { $n ->
        [one] Chapter summary, { $n } sentence
       *[other] Chapter summary, { $n } sentences
    }
# The summary of the selection.
summary-title-selection =
    { $n ->
        [one] Selection summary, { $n } sentence
       *[other] Selection summary, { $n } sentences
    }
# Said when the summary list opens; $title is one of the titles above.
summary-intro = { $title }. Enter goes to the sentence and says it.
# The same, when a long text was read in samples.
summary-intro-sampled = { $title }, from samples of this long text. Enter goes to the sentence and says it.
summary-none = Nothing to summarize: no sentence of four words or more.
# tw summarize, on standard error, when a long text was read in samples: $read of $total characters.
summary-sampled-cli = A long text: the summary comes from { $read } of its { $total } characters, read in samples.
# After a difficult word at high verbosity, with definitions on: its first definition.
aids-difficult-word-defined = difficult word: { $definition }
setting-summary-sentences = Summary sentences
setting-summary-sentences-help = How many sentences Summarize and tw summarize give, 1 to 50.
setting-reading-aids-difficult-definitions = Difficult word definitions
setting-reading-aids-difficult-definitions-help = With difficult words marked, at high verbosity also say a difficult word's first definition from the dictionary.
section-summary = Summaries
settings-unit-sentences =
    { $n ->
        [one] sentence
       *[other] sentences
    }

# The GUI. Said in textweaver's own voice when the window takes the
# focus; $title is the document's title.
gui-window-focused = { $title }, { -brand }.

## Opening the new formats. Said after "Could not open NAME:", so
## each starts in lower case.
opening-damaged-json = it is not a readable JSON file; it may be too large.
opening-damaged-notebook = it is not a readable Jupyter notebook; it may be damaged or too large.
opening-damaged-svg = it is not a readable SVG drawing; it may be damaged or too large.
opening-damaged-mathml = it is not a readable MathML formula; it may be damaged or too large.

## Menus, the command palette, interface announcements, colors, and settings

## Menu titles; the top menus mark their access key with &.

menu-file = &File
menu-edit = &Edit
menu-view = &View
menu-reading = &Reading
menu-speech = &Speech
menu-tools = &Tools
menu-help = &Help
menu-recent = Recent documents
menu-export-as = E&xport as
menu-preview = Preview
menu-settings = Settings
menu-find = Find
menu-format = Format
menu-insert = Insert
menu-proofing = Proofing
menu-citations = Citations
menu-text-size = Text size
menu-reading-aids = Reading aids
menu-rsvp = RSVP
menu-say = Say
menu-move-by = Move by
menu-headings = Headings
menu-go-to = Go to
menu-cursor = Cursor and selection
menu-bookmarks = Bookmarks and notes
menu-tables = Tables
menu-speech-cursor = Speech Cursor

## Command names, in the menus and the command palette.

name-play-pause = Play or pause
name-stop = Stop
name-read-from-cursor = Read from the cursor
name-read-document = Read the whole document
name-read-current-character = Say character
name-read-current-word = Say word
name-read-current-sentence = Say sentence
name-read-current-line = Say line
name-read-paragraph = Say paragraph
name-read-selection = Read selection
name-say-position = Say position
name-say-status = Say status
name-repeat-message = Repeat last message
name-word-count = Word count
name-link-address = Link address
name-replay-sentence = Replay sentence
name-replay-paragraph = Replay paragraph
name-repeat-sentence-slower = Repeat slower
name-rsvp-toggle = RSVP
name-rsvp-play-pause = Start or pause RSVP
name-rsvp-faster = RSVP faster
name-rsvp-slower = RSVP slower
name-rsvp-position-next = Move the RSVP word
name-reading-level = Reading level
name-document-overview = Document overview
name-reading-pass = Reading pass
name-define-word = Define word
name-summarize = Summarize
name-toggle-citations = Read citations
name-explore-math = Explore math
name-listen-rendered = Listen as rendered
name-next-sentence = Next sentence
name-previous-sentence = Previous sentence
name-next-paragraph = Next paragraph
name-previous-paragraph = Previous paragraph
name-next-heading = Read from next heading
name-previous-heading = Read from previous heading
name-skip-next-heading = Next heading
name-skip-previous-heading = Previous heading
name-outline = Outline
name-next-heading-level-1 = Next heading, level 1
name-next-heading-level-2 = Next heading, level 2
name-next-heading-level-3 = Next heading, level 3
name-next-heading-level-4 = Next heading, level 4
name-next-heading-level-5 = Next heading, level 5
name-next-heading-level-6 = Next heading, level 6
name-previous-heading-level-1 = Previous heading, level 1
name-previous-heading-level-2 = Previous heading, level 2
name-previous-heading-level-3 = Previous heading, level 3
name-previous-heading-level-4 = Previous heading, level 4
name-previous-heading-level-5 = Previous heading, level 5
name-previous-heading-level-6 = Previous heading, level 6
name-next-table = Next table
name-previous-table = Previous table
name-next-list = Next list
name-previous-list = Previous list
name-next-list-item = Next list item
name-previous-list-item = Previous list item
name-next-link = Next link
name-previous-link = Previous link
name-next-block-quote = Next block quote
name-previous-block-quote = Previous block quote
name-next-separator = Next separator
name-previous-separator = Previous separator
name-next-graphic = Next graphic
name-previous-graphic = Previous graphic
name-follow-link = Follow link
name-table-next-row = Next table row
name-table-previous-row = Previous table row
name-table-next-column = Next table column
name-table-previous-column = Previous table column
name-next-chapter = Next chapter
name-previous-chapter = Previous chapter
name-history-back = Back
name-history-forward = Forward
name-go-to = Go to
name-document-start = Start of document
name-document-end = End of document
name-caret-next-word = Next word
name-caret-previous-word = Previous word
name-caret-next-line = Next line
name-caret-previous-line = Previous line
name-select-next-word = Select next word
name-select-previous-word = Select previous word
name-select-next-line = Select next line
name-select-previous-line = Select previous line
name-page-down = Page down
name-page-up = Page up
name-scroll-down = Scroll down
name-scroll-up = Scroll up
name-speech-cursor-toggle = Speech Cursor
name-speech-cursor-next-line = Speech Cursor next line
name-speech-cursor-previous-line = Speech Cursor previous line
name-speech-cursor-reread-line = Speech Cursor reread line
name-speech-cursor-exit-and-read = Speech Cursor read on
name-rate-up = Faster
name-rate-down = Slower
name-pitch-up = Higher pitch
name-pitch-down = Lower pitch
name-volume-up = Louder
name-volume-down = Quieter
name-cycle-speed-preset = Speed preset
name-choose-voice = Voices
name-restart-speech = Restart speech
name-cycle-verbosity = Verbosity
name-cycle-punctuation = Punctuation
name-find = Find
name-find-next = Find next
name-find-previous = Find previous
name-search-options = Search options
name-next-misspelling = Next misspelling
name-previous-misspelling = Previous misspelling
name-spelling-suggestions = Spelling suggestions
name-next-grammar-problem = Next grammar problem
name-previous-grammar-problem = Previous grammar problem
name-next-lint-problem = Next lint problem
name-previous-lint-problem = Previous lint problem
name-add-bookmark = Add bookmark
name-list-bookmarks = Bookmarks
name-next-bookmark = Next bookmark
name-previous-bookmark = Previous bookmark
name-add-note = Add note
name-list-notes = Notes
name-next-note = Next note
name-previous-note = Previous note
name-delete-note = Delete note or highlight
name-highlight-selection = Highlight
name-export-study-sheet = Export study sheet
name-self-test = Self-test
name-open = Open
name-open-path = Open by path
name-open-library = Library
name-new-document = New document
name-save = Save
name-save-as = Save as
name-export-settings = Export settings
name-import-settings = Import settings
name-reading-statistics = Reading statistics
name-new-from-template = New from template
name-export-html = Export HTML
name-export-pdf = Export PDF
name-export-docx = Export Word
name-export-epub = Export EPUB
name-export-brf = Export braille
name-preview-in-browser = Preview in browser
name-toggle-preview-auto-reload = Reload preview automatically
name-toggle-preview-live = Live preview
name-browse-files = Browse files
name-batch-convert = Batch convert
name-export-audio = Export audio
name-quit = Quit
name-toggle-edit-mode = Edit mode
name-undo = Undo
name-redo = Redo
name-bold = Bold
name-italic = Italic
name-underline = Underline
name-strikethrough = Strikethrough
name-inline-code = Inline code
name-code-block = Code block
name-insert-link = Insert link
name-heading = Heading
name-bullet-list = Bulleted list
name-numbered-list = Numbered list
name-block-quote = Block quote
name-horizontal-rule = Horizontal rule
name-insert-table = Insert table
name-add-table-row = Add table row
name-insert-image = Insert image
name-replace = Replace
name-copy = Copy
name-cut = Cut
name-next-table-cell = Next table cell
name-previous-table-cell = Previous table cell
name-cycle-typing-echo = Typing echo
name-select-all = Select all
name-delete-word-before = Delete word before
name-delete-word-after = Delete word after
name-paste = Paste
name-insert-citation = Insert citation
name-add-reference = Add reference
name-insert-bibliography = Insert bibliography
name-check-citations = Check citations
name-import-references = Import references
name-dictate = Dictate
name-next-theme = Next theme
name-toggle-line-numbers = Line numbers
name-toggle-character-keys = Single-key shortcuts
name-cycle-access-mode = Accessibility mode
name-settings-profiles = Profiles
name-bionic-toggle = Bionic reading
name-ruler-cycle = Reading ruler
name-syllables-toggle = Syllables
name-difficult-words-toggle = Difficult words
name-text-larger = Larger text
name-text-smaller = Smaller text
name-text-size-reset = Standard text size
name-choose-font = Font
name-contents-panel = Contents panel
name-notes-panel = Notes panel
name-toggle-header = Header
name-toggle-toolbar = Toolbar
name-next-region = Next region
name-previous-region = Previous region
name-color-settings = Colors
name-cycle-interface-announcements = Interface announcements
name-menu = Menus
name-command-palette = Command palette
name-settings = Settings
name-keyboard-help = Keyboard shortcuts
name-what-does-this-key-do = What does this key do
name-about = About textweaver
name-help = Help

## Menus, the palette, and interface announcements.

menu-bar = Menus
menu-title = { $name } menu
menu-submenu = { $name }, submenu
menu-checked = { $name }, checked
menu-not-checked = { $name }, not checked
menu-value = { $name }: { $value }
menu-with-keys = { $item }, { $keys }
menu-recent-document = { $name }, { $pct } percent
menu-recent-none = No recent documents
menu-not-available = { $name } is not available in this version.
menu-no-access-key = No item with the key { $letter }.
menu-closed = Menus closed.
menu-press-a-key = Press a key to hear what it does.
menu-key-described = { $name }: { $help }. Keys: { $keys }. In the menus: { $path }.
menu-key-described-no-menu = { $name }: { $help }. Keys: { $keys }.
menu-keys = { $item }. Enter or Right opens a menu or runs a command, a letter moves to its item, Left or Backspace goes back, Escape closes.
edit-line-continues = line continues
announce-level-changed = Interface announcements { $level }.
announce-level-off = off
announce-level-minimal = minimal
announce-level-normal = normal
announce-level-full = full
setting-accessibility-interface-announcements = Interface announcements
setting-accessibility-interface-announcements-help = How much textweaver says about itself: dialogs, progress, hints, and routine confirmations. Errors and answers to what you asked are always said. Automatic is minimal with a screen reader, normal when self-voicing.
choice-accessibility-interface-announcements-auto = automatic
choice-accessibility-interface-announcements-off = off
choice-accessibility-interface-announcements-minimal = minimal
choice-accessibility-interface-announcements-normal = normal
choice-accessibility-interface-announcements-full = full
palette-item = { $name }, { $keys }
palette-item-no-keys = { $name }
palette-item-recent = { $name }, { $keys }, recent
palette-item-recent-no-keys = { $name }, recent
palette-list-title = Commands matching { $query }
palette-list-title-all = Commands
palette-list-intro =
    { $title }, { $n ->
        [one] 1 command
       *[other] { $n } commands
    }. Enter runs one, F1 explains it.
action-browse-files = Browse files and archives: Enter opens a folder, an archive, or a document; Backspace goes up
action-batch-convert = Convert a folder of documents to another format, in the background
action-export-audio = Export the document as spoken audio: MP3, FLAC, Opus, WAV, or an M4B audiobook
action-dictate = Start or stop dictation: spoken words are typed at the cursor in edit mode
action-color-settings = Open the color settings: the reading highlight, the ruler, marks, and each part of the screen, with their contrast
action-cycle-interface-announcements = Cycle how much textweaver announces about itself: off, minimal, normal, or full; errors and answers are always said
action-menu = Open the menus: File, Edit, View, Reading, Speech, Tools, and Help
action-what-does-this-key-do = Press a key to hear what it does and where it is in the menus, without running it
action-about = List the facts a problem report needs: version, build, components, speech engines, and folders
setting-colors-ruler = Reading ruler color
setting-colors-ruler-help = The band of the reading ruler and the marked current line. The ruler keeps its underline or bold. The terminal reader uses it; the window does not yet. Choose a name, or type a hex code. Default: the theme's color.
setting-colors-difficult-words = Difficult words color
setting-colors-difficult-words-help = The underline of difficult words; they stay underlined and are named at high verbosity. The terminal reader uses it; the window does not yet. Choose a name, or type a hex code. Default: the theme's color.
setting-colors-syllables = Syllable marks color
setting-colors-syllables-help = The middle dots between syllables. The terminal reader uses it; the window does not yet. Choose a name, or type a hex code. Default: the theme's color.
setting-colors-misspellings = Misspellings color
setting-colors-misspellings-help = The underline of misspelled words, which are also said. Not used yet: the window draws the theme's color. Choose a name, or type a hex code. Default: the theme's color.
setting-colors-lint = Lint marks color
setting-colors-lint-help = The underline of Markdown lint and grammar problems, which are also said. Not used yet: the window draws the theme's color. Choose a name, or type a hex code. Default: the theme's color.
setting-colors-find-match = Search match color
setting-colors-find-match-help = The band behind search matches; they stay underlined. Choose a name, or type a hex code. Default: the theme's color.
setting-colors-selection = Selection color
setting-colors-selection-help = The band behind selected text. Choose a name, or type a hex code. Default: the theme's color.
setting-colors-focus = Focus color
setting-colors-focus-help = The focus outline and the focused item of a list; they stay bold. Choose a name, or type a hex code. Default: the theme's color.
setting-colors-links = Link color
setting-colors-links-help = The color of links. Links stay underlined. Choose a name, or type a hex code. Default: the theme's color.
setting-colors-headings = Heading color
setting-colors-headings-help = The color of headings. Headings stay bold. Choose a name, or type a hex code. Default: the theme's color.
setting-colors-status-bar = Status bar color
setting-colors-status-bar-help = The band of the status and title bars. Choose a name, or type a hex code. Default: the theme's color.
setting-colors-notes = Note color
setting-colors-notes-help = The band behind text with a note; it stays italic and underlined. Choose a name, or type a hex code. Default: the theme's color.
setting-colors-bookmarks = Bookmark color
setting-colors-bookmarks-help = The band behind a bookmarked word; it stays bold and underlined. Choose a name, or type a hex code. Default: the theme's color.
color-name-theme = the theme's color
color-name-blue = blue
color-name-orange = orange
color-name-navy = dark blue
color-name-skyblue = sky blue
color-name-teal = teal
color-name-gold = gold
color-name-yellow = yellow
color-name-purple = purple
color-name-pink = pink
color-name-brown = brown
color-name-gray = gray
color-name-black = black
color-name-white = white
section-colors = Colors
colors-contrast-good = good
colors-contrast-fair = fair
colors-contrast-low = low
colors-item = { $label }: { $value }, contrast { $ratio } to 1, { $verdict }
colors-contrast = Contrast { $ratio } to 1, { $verdict }.
colors-contrast-warning = Under 3 to 1 it is hard to see; choose a lighter or darker color.
colors-intro = Colors, { $n } settings. Left and Right choose a named color, Enter types a name or a #rrggbb value, Delete puts the theme's back, F1 says the help.
settings-item-recent = { $item }, recently changed
settings-reset = { $label } back to its default, { $value }.
settings-row-help = { $label }: { $value }. Default: { $default }. { $help }
number-group-separator = ,
number-decimal-separator = .
settingsio-import-question-names =
    { $n ->
        [one] Import { $n } changed setting from { $name }: { $names }? y or n
       *[other] Import { $n } changed settings from { $name }: { $names }? y or n
    }
settingsio-and-more = { $names }, and { $n } more


## Dictation in edit mode (ADR-0042). Keep the meaning first: a
## 40-cell Braille display shows the start of the line. $words are the
## dictated words, $key the dictate key, $dir a folder, $error and $text
## are passed on as they are.
dictation-status = Dictating: { $words }
dictation-listening = Dictating. Speak, then press { $key } to stop.
dictation-finishing = Finishing dictation.
dictation-done = Dictation done.
dictation-busy = Dictation is finishing. Try again in a moment.
dictation-needs-edit = Dictation types in edit mode. Turn on edit mode and dictate? y or n
dictation-no-model = Dictation needs the Whisper model in { $dir }. See Dictation in the documentation.
dictation-failed = Dictation failed: { $error } See Dictation in the documentation.
dictation-no-words = No words recognized in that phrase.
dictation-lost = Dictation stopped before its last words were typed.
dictation-not-typed = Dictated words not typed, edit mode is off: { $text }
setting-dictation-speak-while-recording = Speak while dictating
setting-dictation-speak-while-recording-help = Say dictated words as they come. Off, they are shown on the status line and said at each pause, so the microphone does not hear the voice.
setting-dictation-model-dir = Dictation model folder
setting-dictation-model-dir-help = A folder holding a Whisper model for dictation. Not set uses the dictation model below, in the data folder.
section-dictation = Dictation


## The file browser. Every row and introduction starts with the name,
## then the kind, so the first cells of a 40-cell Braille line hold what
## matters. $name is a file or folder name; $n a number that chooses the
## plural and $count the same number written with its separators.
# A list item with its position after it, in the file browser.
listmodel-item-position-last = { $item }, { $k } of { $n }
browse-places-title = Places
browse-places-intro =
    { $n ->
        [one] Places, 1 place.
       *[other] Places, { $n } places.
    }
# $purpose says what the folder or file is chosen for; $intro follows.
browse-choosing = { $purpose }. { $intro }
browse-place-document = { $name }, the document's folder
browse-place-start = { $name }, start folder
browse-place-library = { $name }, library folder
browse-place-disk = { $name }, disk
browse-place-removable = { $name }, removable drive
browse-place-network = { $name }, network drive
browse-place-cd = { $name }, CD or DVD drive
browse-place-root = { $name }, the root folder
browse-choose-here = Choose this folder, { $name }
browse-row-folder = { $name }, folder
browse-row-folder-items =
    { $name }, folder, { $n ->
        [one] 1 item
       *[other] { $count } items
    }
# $kind is a kind below ("Markdown"); $size a size below ("12 KB").
browse-row-file = { $name }, { $kind }, { $size }
browse-row-kind = { $name }, { $kind }
browse-row-hidden = { $row }, hidden
# $kind is zip, tar, tar.gz, gzip, or 7z.
browse-kind-archive = { $kind } archive
browse-kind-file = file
browse-kind-markdown = Markdown
browse-kind-text = text
browse-kind-html = web page
browse-kind-epub = EPUB book
browse-kind-docx = Word document
browse-kind-rtf = RTF document
browse-kind-odt = OpenDocument text
browse-kind-latex = LaTeX
browse-kind-eml = email
browse-kind-mhtml = web archive
browse-kind-pdf = PDF
browse-kind-image = picture
browse-kind-daisy = DAISY book
browse-kind-pptx = PowerPoint slides
browse-kind-sheet = spreadsheet
browse-kind-json = JSON
browse-kind-notebook = Jupyter notebook
browse-kind-svg = SVG drawing
browse-kind-mathml = MathML formula
browse-kind-pandoc = document read through Pandoc
browse-size-bytes =
    { $n ->
        [one] 1 byte
       *[other] { $count } bytes
    }
# $size is a number, with a decimal under 10 ("3.4").
browse-size-kb = { $size } KB
browse-size-mb = { $size } MB
browse-size-gb = { $size } GB
browse-intro =
    { $name }, { $n ->
        [one] 1 item.
       *[other] { $count } items.
    }
browse-intro-empty = { $name } has nothing to list.
# $filter is what was typed.
browse-intro-filtered =
    { $name }, { $n ->
        [one] 1 item matches
       *[other] { $count } items match
    } { $filter }.
browse-intro-in-archive = { $intro } In { $archive }.
browse-intro-hidden =
    { $intro } { $n ->
        [one] 1 file hidden.
       *[other] { $count } files hidden.
    }
browse-intro-cut = { $intro } Only the first { $max } are listed.
# $preview, $choose, $sort, and $all are keys; $item the focused row.
browse-keys = { $intro } Enter opens, Backspace goes up, typing filters. { $preview } previews, { $choose } chooses a folder, { $sort } sorts, { $all } shows all files. { $item }
browse-sorted-name = Sorted by name.
browse-sorted-date = Sorted by date, newest first.
browse-sorted-size = Sorted by size, largest first.
browse-showing-all = Showing all files.
browse-showing-readable = Showing readable files only.
browse-closed = File browser closed.
browse-read-only = The file browser only opens and chooses files; it never changes them.
# $key is the Choose Folder key.
browse-choose-a-folder = Choose a folder: Enter opens one, { $key } chooses it.
browse-choose-a-file = Choose a file: Enter chooses one.
browse-no-archive-folder = A folder inside an archive cannot be chosen; choose a folder on disk.
browse-nothing-waiting = No command is waiting for a folder; Enter opens it.
browse-cannot-read = { $name } is not a kind of file textweaver can read.
browse-folder-unreadable = Could not open { $name }: { $reason }
browse-archive-too-deep = { $name } is inside too many archives to open.
browse-archive-too-large = { $name } is too large to list safely.
browse-archive-unreadable = { $name } is not an archive textweaver can read; it may be damaged.
# A document's preview: its title, then its first sentence.
browse-preview-document = { $title }. { $sentence }
browse-preview-no-text = { $title }. It has no text.
browse-preview-failed = Could not preview { $name }: { $reason }
# $names are the first few names inside.
browse-preview-archive =
    { $name }: { $n ->
        [one] 1 file
       *[other] { $count } files
    }, { $readable } readable. { $names }
browse-preview-archive-folder =
    { $name }, folder in the archive, { $n ->
        [one] 1 item.
       *[other] { $count } items.
    }
browse-preview-folder = { $path }: { $names }
browse-preview-folder-empty = { $path }: nothing to read here.
browse-preview-other = { $name }, { $size }; textweaver cannot read this kind of file.
browse-preview-path = { $path }


## Batch conversion (File, Batch convert). Keep the meaning first: a
## 40-cell Braille display shows the start of the line. $n, $done, $total,
## $converted, $skipped, $failed and $left are numbers of files; $format is
## a format's name (PDF, Markdown); $path a folder; $name a file or folder
## name; $reason and $error are passed on as they are.
batch-choose-source = Choose the folder to convert
batch-choose-output = Choose the folder for the converted files
batch-format-title = Convert to
batch-format-intro = Convert { $name } to: choose a format, { $n } choices.
batch-where-title = Where the files go
batch-where-intro = Where should the converted files go?
batch-where-converted = In a converted folder, { $path }
batch-where-beside = Beside each file
batch-where-choose = In another folder, chosen next
batch-nothing = No documents to convert in { $path }.
batch-confirm =
    Convert { $n ->
        [one] 1 file
       *[other] { $n } files
    } to { $format } into { $path }? y or n
batch-confirm-beside =
    Convert { $n ->
        [one] 1 file
       *[other] { $n } files
    } to { $format } beside each file? y or n
batch-started =
    Converting { $n ->
        [one] 1 file
       *[other] { $n } files
    } to { $format }. Escape stops.
batch-progress = { $percent } percent converted, { $done } of { $total } files.
batch-busy = Already converting, { $done } of { $total } files. Escape stops.
batch-stop-question = Stop converting? Files already done are kept. y or n
batch-stopping = Stopping after the files being written.
batch-still-converting = Still converting.
batch-done =
    Converted { $converted ->
        [one] 1 file
       *[other] { $converted } files
    } to { $format }; { $skipped } up to date; { $failed } failed.
batch-stopped =
    Stopped. Converted { $converted ->
        [one] 1 file
       *[other] { $converted } files
    }; { $left } not converted; { $failed } failed.
batch-report = Report saved in { $path }.
batch-report-failed = The report could not be saved: { $error } The converted files are kept.
batch-inaccessible =
    { $n ->
        [one] 1 file has
       *[other] { $n } files have
    } items not made accessible; see the report.
batch-failures-title =
    { $n ->
        [one] 1 failed file
       *[other] { $n } failed files
    }
batch-failure-item = { $name }: { $reason }
batch-start-failed = Could not start converting: { $error } Check the folder and the format, then try again.
batch-thread-stopped = Batch conversion stopped unexpectedly.


## Audio export (File, Export audio). Keep the meaning first: a
## 40-cell Braille display shows the start of the line. $name is a file
## name (essay.flac); $path a folder or a file's full path; $format a
## format's name (FLAC, MP3); $voice a voice's or engine's name; $wpm is
## words per minute; $length a length of time from the duration-*
## messages; $chapters and $n are numbers; $percent is a multiple of ten;
## $formats lists format names (M4B); $error is passed on as it is.
audio-format-title = Export audio as
audio-format-intro = Export { $name } as audio: choose a format, { $n } choices.
audio-no-ffmpeg =
    { $formats } { $n ->
        [one] needs
       *[other] need
    } ffmpeg, which was not found.
audio-format-flac = FLAC: lossless, about half the size of WAV
audio-format-wav = WAV: the largest, plays everywhere
audio-format-mp3 = MP3: small, plays everywhere
audio-format-opus = Opus: the smallest, made for speech
audio-format-ogg = Ogg Vorbis: small and open, plays in most players
audio-format-m4b = M4B audiobook, through ffmpeg
audio-format-mp4 = Video with captions: MP4, needs ffmpeg
audio-format-html = Read-along page: text and audio, one file
audio-where-title = Where the audio goes
audio-where-intro = Where should the audio go?
audio-where-beside = Beside the document, { $path }
audio-where-choose = In another folder, chosen next
audio-choose-folder = Choose the folder for the audio
audio-no-engine = No speech engine here can write audio files. Install eSpeak NG, or choose another engine in the Speech menu.
audio-confirm = Export { $name } with { $voice } at { $wpm } words per minute, into { $path }? y or n
audio-started = Exporting { $name } as { $format }. Escape stops.
audio-progress = Exporting audio, { $percent } percent.
audio-busy = Already exporting { $name }. Escape stops.
audio-stop-question = Stop the export? No file is kept. y or n
audio-stopping = Stopping the export.
audio-still-exporting = Still exporting audio.
audio-stopped = Audio export stopped; no file was written.
audio-done =
    Wrote { $name }: { $length }, { $chapters ->
        [one] 1 chapter
       *[other] { $chapters } chapters
    }.
audio-subtitles = Subtitles in { $name }.
audio-failed = Could not export audio: { $error } Try another format or voice.
audio-thread-stopped = Audio export stopped unexpectedly.


## The window's menus and dialogs. Settings files chosen with the
## system's file chooser, the Colors dialog, and the font list. $ratio is
## a contrast ratio such as 4.8; $verdict is good, fair, or low.
gui-settings-files = Settings files
gui-settings-export-title = Export settings
gui-settings-import-title = Import settings
gui-chooser-no-dialog = The system's file chooser did not open. Type the path of the file instead.
gui-colors-value = { $value }, contrast { $ratio } to 1, { $verdict }
gui-colors-help = Left and Right choose a named color, blue and orange first. Enter types a name or a #rrggbb value. Delete puts the theme's color back. Every mark keeps its underline, weight, or symbol, whatever its color.
gui-colors-reset-all = Reset all colors
gui-colors-reset-all-help = Put the theme's own color back for every part.
gui-colors-reset-done = Every color is the theme's again.
# Asked before Reset all colors; a yes resets them.
colors-reset-question = Reset every color to the theme's? y or n
gui-colors-closed = Colors closed.
gui-font-list-intro =
    { $n ->
        [one] { $title }, 1 family.
       *[other] { $title }, { $n } families.
    }


## PDF links. Said before the first line of the page a link inside
## a PDF goes to, when the page has no heading there (as links-heading-label
## is for a heading). $page is the page's printed number or label (12, iv).
links-page-label = Page { $page }


## Sync wave, S4: sync in the reader (ADR-0049). $device is a computer's
## name as the owner gave it ("laptop", "lab", "Computer 2"); $title a
## document's title; $pct a percentage; $n a count. Meaning first, and
## within the 40 cells of a Braille line where it can be.
sync-status-off = Sync: off
sync-status-not-set-up = Sync: not set up
sync-status-starting = Sync: starting
sync-status-folder-missing = Sync: folder missing, saving here
sync-status-read-only = Sync: newer format, read only
sync-status-failed = Sync: cannot use the folder
sync-status-cannot-write = Sync: cannot write, saving here
sync-status-clock-ahead = Sync: { $device }'s clock is ahead
sync-status-damaged =
    { $n ->
        [one] Sync: 1 damaged file skipped
       *[other] Sync: { $n } damaged files skipped
    }
sync-status-up-to-date = Sync: up to date
sync-status-this-computer = This computer: { $name }.
sync-status-no-others = No other computers yet.
sync-status-others = Other computers: { $names }.
sync-status-error = Problem: { $error } Check the sync folder.
sync-another-computer = another computer
sync-untitled = a document
sync-damaged = Sync: damaged file from { $device } skipped.
sync-newer-file = Sync: newer file from { $device } skipped.
sync-read-only = Sync: newer format, read only.
sync-clock-ahead = Sync: { $device }'s clock is { $hours } hours ahead.
sync-fresh-id = Sync: copied setup; new computer id.
sync-write-failed = Sync: cannot write. { $error } Saving on this computer for now.
sync-name-refused = Name not allowed. Try one like laptop.
sync-note-replaced =
    { $n ->
        [one] { $title }: a note was replaced by { $device }'s newer edit.
       *[other] { $title }: { $n } notes were replaced by { $device }'s newer edits.
    }
sync-restored-notes =
    { $n ->
        [one] { $title }: a deleted note came back, edited on { $device }.
       *[other] { $title }: { $n } deleted notes came back, edited on { $device }.
    }
sync-restored-bookmarks =
    { $n ->
        [one] { $title }: a deleted bookmark came back, edited on { $device }.
       *[other] { $title }: { $n } deleted bookmarks came back, edited on { $device }.
    }
sync-restored-highlights =
    { $n ->
        [one] { $title }: a deleted highlight came back, edited on { $device }.
       *[other] { $title }: { $n } deleted highlights came back, edited on { $device }.
    }
sync-arrived =
    { $n ->
        [one] { $title }: 1 change from { $device }.
       *[other] { $title }: { $n } changes from { $device }.
    }
sync-resumed = { $title }: resumed at { $pct } percent, from { $device }.
sync-place-arrived = { $device }'s place: { $pct } percent.
sync-place-question = { $device } at { $pct } percent. Go there? y or n
sync-suggestion-question =
    { $n ->
        [one] This may be { $title } from { $device }, with 1 note. Use the note from { $device }? y or n
       *[other] This may be { $title } from { $device }, with { $n } notes. Use the notes from { $device }? y or n
    }
sync-went-to-place = { $device }'s place, { $pct } percent.
sync-kept-place = Kept this place.
sync-suggestion-accepted = Using the notes from { $device }.
sync-suggestion-declined = Kept separate.
sync-sidecar-differed =
    { $n ->
        [one] Sync: 1 library place differed.
       *[other] Sync: { $n } library places differed.
    }
sync-sidecar-failed = Sync: cannot write a library place. { $error } Check that the library folder can be written to.
sync-no-state = Sync is off in this run: nothing is saved.
sync-choose-folder = Choose the sync folder
sync-group-places = Places
sync-group-notes = Notes
sync-group-highlights = Highlights
sync-group-bookmarks = Bookmarks
sync-group-statistics = Statistics
sync-group-item = { $name }: { $state }
sync-start = Start syncing
sync-groups-title = What syncs
sync-groups-intro = { $title }, as { $name }. Enter turns one on or off; Start syncing ends.
sync-started = Sync on, as { $name }. Sync status: { $key }.
sync-how-to-set-up = To set it up: Tools, Sync, Set up sync.
sync-now-started = Syncing.
sync-now-done =
    { $n ->
        [one] Sync: up to date, 1 document checked.
       *[other] Sync: up to date, { $n } documents checked.
    }
sync-now-changed =
    { $n ->
        [one] Sync: 1 document took changes.
       *[other] Sync: { $n } documents took changes.
    }
sync-no-places = No other computer has a place here.
sync-place-item = { $device }, { $pct } percent
sync-places-title =
    { $n ->
        [one] 1 other place
       *[other] { $n } other places
    }
sync-no-replaced = No replaced notes in this document.
sync-replaced-item = { $text }, replaced by { $device }
sync-replaced-item-deleted = { $text }, deleted by { $device }
sync-replaced-title =
    { $n ->
        [one] 1 replaced note
       *[other] { $n } replaced notes
    }
sync-replaced-intro = { $title }. Enter puts one back.
sync-note-restored = Note put back: { $text }
sync-already-off = Sync is already off here.
sync-stopped = Sync off here. The folder is left as it is.
prompt-sync-computer-name = Name this computer, Enter keeps it
menu-sync = Sync
name-sync-setup = Set up sync
name-sync-status = Sync status
name-sync-now = Sync now
name-sync-go-to-place = Go to another computer's place
name-sync-replaced-notes = Replaced notes
name-sync-stop = Stop syncing on this computer
action-sync-setup = Set up sync: choose the sync folder, name this computer, and choose what syncs
action-sync-status = Say how sync stands (up to date, the folder missing, or a problem) and name the other computers
action-sync-now = Sync now: send this computer's changes and take the other computers' for every document
action-sync-go-to-place = List the other computers' places in this document; Enter goes to one
action-sync-replaced-notes = List notes replaced by another computer's newer edit; Enter puts one back
action-sync-stop = Stop syncing on this computer; the sync folder is left as it is
section-sync = Sync
setting-sync-enabled = Sync
setting-sync-enabled-help = Sync notes, highlights, bookmarks, and places with your other computers through the sync folder. Tools, Sync, Set up sync turns it on.
setting-sync-folder = Sync folder
setting-sync-folder-help = The folder your computers share. One kept in step by Syncthing, a cloud folder, or a USB stick.
setting-sync-device-name = Computer name
setting-sync-device-name-help = This computer's name in sync messages, such as laptop or lab. Empty uses Computer 1, Computer 2, and so on.
setting-sync-places = Sync places
setting-sync-places-help = Share where you are in each document.
setting-sync-notes = Sync notes
setting-sync-notes-help = Share notes.
setting-sync-highlights = Sync highlights
setting-sync-highlights-help = Share highlights.
setting-sync-bookmarks = Sync bookmarks
setting-sync-bookmarks-help = Share bookmarks.
setting-sync-statistics = Sync statistics
setting-sync-statistics-help = Share each computer's reading time and sessions.
setting-sync-position-policy = Place to resume
setting-sync-position-policy-help = Which place a document opens at when another computer has one too. The newest, the furthest, or ask.
choice-sync-position-policy-newest = the newest
choice-sync-position-policy-furthest = the furthest
choice-sync-position-policy-ask = ask

## End of S4

## Sync wave, S5: settings and word lists (ADR-0049). $n is a count,
## $device a computer's name as the owner gave it, $computers a count of
## computers, $voice a voice's id. Meaning first, within 40 cells where it
## can be.
sync-settings-arrived =
    { $n ->
        [one] Settings: 1 change from { $device }.
       *[other] Settings: { $n } changes from { $device }.
    }
sync-settings-arrived-several = Settings: { $n } changes from { $computers } computers.
sync-kept-keys-mac =
    { $n ->
        [one] Mac key overrides: 1, kept, not used here.
       *[other] Mac key overrides: { $n }, kept, not used here.
    }
sync-kept-keys-pc =
    { $n ->
        [one] Windows and Linux key overrides: 1, kept, not used here.
       *[other] Windows and Linux key overrides: { $n }, kept, not used here.
    }
sync-group-settings = Settings
sync-group-profiles = Profiles
sync-group-key-overrides = Key overrides
sync-group-words = Word list
sync-group-glossary = Glossary and pronunciations
sync-group-favorite-voices = Favorite voices
voices-missing-row = { $voice }, favorite, not on this computer
voices-missing = { $voice } is not on this computer. Space takes it off your favorites.
gui-voices-list = Voices
gui-voices-use = Use voice
gui-voices-use-help = Use the focused voice and hear a sample, or download it after a question.
gui-voices-preview = Preview
gui-voices-preview-help = Hear a sample in the focused voice without choosing it.
gui-voices-favorite = Favorite
gui-voices-favorite-help = Make the focused voice a favorite, or stop it being one. Favorites come first.
gui-voices-remove = Remove
gui-voices-remove-help = Remove the focused downloaded Piper voice, after a question.
gui-voices-remove-unavailable = unavailable
gui-voices-language-help = Show only the next language's voices, then all languages again.
gui-voices-engine-help = Show only the next engine's voices, then all engines again.
gui-voices-fetch-help = Download the list of Piper voices, about 250 KB, after a question.
gui-voices-close-help = Close the voice manager.
gui-voices-hint = Enter uses the voice, Space marks a favorite, Escape closes.
setting-sync-settings = Sync settings
setting-sync-settings-help = Share the portable settings: rate, punctuation, theme, reading aids, and the like. The voice, the engine, the access mode, the key preset, and paths stay on each computer.
setting-sync-profiles = Sync profiles
setting-sync-profiles-help = Share your profiles; which one is in use stays on each computer.
setting-sync-key-overrides = Sync key overrides
setting-sync-key-overrides-help = Share keymap.toml. A Mac's keys are kept but not used on Windows or Linux, and the reverse.
setting-sync-words = Sync word list
setting-sync-words-help = Share your spelling word list.
setting-sync-glossary = Sync glossary
setting-sync-glossary-help = Share your glossary's entries and your pronunciations.
setting-sync-favorite-voices = Sync favorite voices
setting-sync-favorite-voices-help = Share your favorite voices. One this computer does not have is listed as not on this computer.

## End of S5

## Lexend downloaded on first choice. The question names the size and
## the license before y or n; the rest put the font's name first.
font-download-question = Download the { $font } font, { $kb } KB, { $licence }? y or n
font-downloading = Downloading { $font }.
font-downloaded = { $font } downloaded and ready.
font-download-failed = { $font } not downloaded: { $error } Another font is used.
font-download-declined = Not downloaded. Another font is used.
font-download-busy = { $font } is still downloading.
font-download-no-folder = No data folder to keep { $font } in.
font-download-not-in-build = Font downloads are not in this version.
gui-font-to-download = { $family } (download, { $kb } KB)
gui-font-downloaded = { $family } (downloaded)

## File and folder choosers. The browse key fills a prompt for a
## path from the file browser; the GUI names its choosers and filters.
prompt-browse-hint = { $label }. { $key } to browse.
prompt-browse-file = Choose the file: { $label }
prompt-browse-folder = Choose the folder: { $label }
prompt-browse-filled = { $name } chosen. Enter confirms.
chooser-type-files = { $type } files
chooser-image-title = Insert an image
chooser-images = Images
chooser-references-title = Import references
chooser-reference-files = Reference files
chooser-profiles-import-title = Import profiles
chooser-profiles-export-title = Export profiles
chooser-profile-files = Profile files
gui-folder-no-dialog = The system's folder chooser did not open. Choose the folder in this list.
gui-prompt-browse-hint = { $key } opens the file browser.

## Optional components: Manage optional components, the
## first-run list, and the questions features ask. Status messages fit 40
## Braille cells, meaning first. $title is a component's title, $size a
## size ("79.3 MB"), $license its license, $features what needs it.
name-manage-components = Manage optional components…
name-download-dictation-model = Download the dictation model
action-manage-components = Manage optional components: the models, fonts, and voices textweaver can download, with their size and license
action-download-dictation-model = Download the dictation model chosen in the settings, after saying its size and license
component-feature-dictation = dictation
component-feature-ocr = reading scanned pages
component-feature-reading-font = a reading font
component-feature-voice = a speech voice
component-state-installed = installed
component-state-not-installed = not installed
component-state-partial = partly installed
component-state-damaged = damaged
component-state-downloading = downloading
components-title = Optional components
components-intro =
    { $n ->
        [one] 1 optional component. Enter for actions.
       *[other] { $n } optional components. Enter for actions.
    }
components-item = { $title }: { $state }, { $size }, license { $license }, for { $features }
components-actions-intro = { $title }: { $state }.
components-action-download = Download, { $size }
components-action-verify = Verify the files
components-action-remove = Remove
components-action-install-zip = Install from a zip file…
components-action-install-folder = Install from a folder…
components-install-purpose = Install the component from here
component-question = Download { $title }, { $size }, license { $license }? y or n
component-remove-question = Remove { $title }? y or n
component-downloading = Downloading. Escape stops it.
component-installing = Installing from the file.
component-verifying = Checking the files.
component-progress = { $percent } percent downloaded.
component-ready = Ready: { $title }.
component-verified = Files check out: { $title }.
component-verify-failed =
    { $n ->
        [one] 1 file does not check out: { $files }.
       *[other] { $n } files do not check out: { $files }.
    }
component-removed = Removed: { $title }.
component-refused =
    { $n ->
        [one] 1 file left out: { $files }.
       *[other] { $n } files left out: { $files }.
    }
component-already = Already installed: { $title }.
component-not-there = Not installed: { $title }.
component-declined = Not downloaded.
component-not-in-build = Downloads are not in this version.
component-no-folder = No data folder to keep it in.
component-error-fetch = Not downloaded: the source failed.
component-error-size = Not installed: wrong file size.
component-error-hash = Not installed: a file did not match. Get it again, or check the source.
component-error-missing = Not installed: a file is missing.
component-error-cancelled = Download stopped; it resumes later.
component-error-busy = Already downloading one.
component-error-no-source = Not downloaded: no address for it.
component-error-name = Refused: a name is not plain.
component-error-manifest = A components list is unreadable.
component-error-io = Not installed: could not write.
components-chooser-title = Optional components
components-chooser-intro = Optional extras, none chosen. Space chooses one; Download the chosen ones gets them; Escape skips.
components-chooser-item = { $mark }: { $title }, for { $features }, { $size }, license { $license }
components-chosen = Chosen
components-not-chosen = Not chosen
components-chooser-download = Download the chosen ones
components-chooser-skip = Skip for now
components-chooser-skipped = Skipped; see Manage optional components.
components-chooser-none = Nothing chosen, nothing downloaded.
dictation-model-question = Dictation needs the Whisper model, { $size }, license { $license }. Download it now? y or n
dictation-model-declined = No model, so no dictation for now.
dictation-model-not-in-build = No model; this version cannot download.
dictation-model-file-missing = Dictation model lacks { $file }.
dictation-model-damaged = Dictation model damaged: { $file }.
dictation-model-no-folder = No model folder: { $dir }.

## Settings for optional components.
section-components = Optional components
setting-dictation-model = Dictation model
setting-dictation-model-help = The Whisper model dictation uses when no folder is set. Download the dictation model, in the Tools menu, gets it.
choice-dictation-model-whisper-base-en = base.en, the default
choice-dictation-model-whisper-small-en = small.en, larger and more accurate
setting-components-source = Components source
setting-components-source-help = Your own components, used first. A GitHub repository as owner/name, or a folder on this computer. Empty uses none. Never put a password here.
setting-components-mirror = Components mirror
setting-components-mirror-help = Where optional components come from first: an https address or a folder on this computer. Empty uses their public sources. Never put a password here.

## Help's ways to the docs, About's facts, and first-run choices asked
## again. $address is a web address; $path a folder; facts start with
## their name so each Braille line leads with it.
name-quick-start = Quick start
name-documentation = Documentation…
name-report-problem = Report a problem…
name-ask-first-run-again = Ask again about first-run choices
action-quick-start = Open the quick start guide in textweaver
action-documentation = Show the documentation's web address, and ask before opening it in a browser
action-report-problem = Show where to report a problem, and ask before opening it in a browser; nothing is sent
action-ask-first-run-again = Ask again about first-run choices: hybrid mode with a screen reader, and the optional components
about-title = About textweaver
about-intro = About textweaver, { $n } facts. Include them in a problem report.
about-version = Version: textweaver { $version }
about-build = Build: { $frontend }, { $profile }, { $os } { $arch }
about-frontend-window = window
about-frontend-terminal = terminal reader
about-profile-release = release
about-profile-debug = debug
about-license = License: { $license }
about-engine-in-use = Speech engine in use: { $engine }
about-engines = Speech engines found: { $engines }
about-engines-none = Speech engines found: none
about-components = Components: { $installed } of { $known } installed
about-folder-settings = Settings folder: { $path }
about-folder-data = Data folder: { $path }
about-folder-cache = Cache folder: { $path }
about-no-folders = Folders: none; this session keeps no files.
about-quick-start-online = Quick start not found beside textweaver. Open { $address } in your browser? y or n
about-docs-question = Documentation: { $address }. Open it in your browser? y or n
about-report-question = Report a problem: { $address }. Nothing is sent. Open it in your browser? y or n
about-first-run-again = First-run choices reset; asked at the next start.

## W9b-x: the reading settings (View, Reading settings): the settings a
## reader changes most, in one form, with the two spacing presets. $n is a
## number of settings.
name-reading-form = Reading settings
action-reading-form = Open the reading settings: rate, font, spacing, line length, theme, highlight, ruler, bionic reading, and syllables
reading-form-intro = Reading settings, { $n } settings. Left and Right change a value, Enter types one, Delete puts the default back, F1 says the help.
reading-form-spacing-wcag-done = Spacing set to the WCAG values.
reading-form-spacing-generous-done = Spacing set to Generous, wider than WCAG.
gui-reading-form-help = Up and Down move, Left and Right change a value, Enter types one, F1 says the help.
gui-reading-voices = Voices
gui-reading-voices-help = Open the voice manager.
gui-reading-wcag = WCAG spacing
gui-reading-wcag-help = Set the four spacings to the WCAG values.
gui-reading-generous = Generous spacing
gui-reading-generous-help = Set the four spacings wider than WCAG.
gui-reading-closed = Reading settings closed.
## End of W9b-x

## B1-t1: tracked changes and comments (the changes list). A row puts the
## kind first; $text is the changed text, $who is changes-by or
## changes-no-author, $when is changes-date or changes-no-date.
name-list-changes = Changes and comments
action-list-changes = List the tracked changes and comments: Enter goes to one, A accepts a change, R rejects it
name-accept-all-changes = Accept all changes
action-accept-all-changes = Accept every tracked change in the document
name-reject-all-changes = Reject all changes
action-reject-all-changes = Reject every tracked change in the document
name-add-comment = Add comment
action-add-comment = Add a comment to the selection or the sentence at the cursor
prompt-comment-reply = Reply
prompt-comment-text = Comment
changes-title = Changes and comments
# Said when the list opens; $title is changes-title, $n how many rows.
changes-intro =
    { $n ->
        [one] { $title }, { $n } item. Enter goes to it. A accepts a change, R rejects it, and Shift with either does every change by its author. On a comment, F2 replies, Space resolves, Delete deletes. N adds a comment.
       *[other] { $title }, { $n } items. Enter goes to one. A accepts a change, R rejects it, and Shift with either does every change by its author. On a comment, F2 replies, Space resolves, Delete deletes. N adds a comment.
    }
changes-none = No tracked changes or comments in this document.
changes-row = { $kind }: '{ $text }', { $who }, { $when }
changes-kind-inserted = Inserted
changes-kind-deleted = Deleted
changes-kind-moved-away = Moved away
changes-kind-moved-here = Moved here
changes-by = by { $author }
changes-no-author = author not recorded
changes-no-date = date not recorded
# A date from the document, said in full: "Tuesday, March 3, 2026".
changes-date = { $weekday }, { $month } { $day }, { $year }
changes-weekday-0 = Sunday
changes-weekday-1 = Monday
changes-weekday-2 = Tuesday
changes-weekday-3 = Wednesday
changes-weekday-4 = Thursday
changes-weekday-5 = Friday
changes-weekday-6 = Saturday
changes-month-1 = January
changes-month-2 = February
changes-month-3 = March
changes-month-4 = April
changes-month-5 = May
changes-month-6 = June
changes-month-7 = July
changes-month-8 = August
changes-month-9 = September
changes-month-10 = October
changes-month-11 = November
changes-month-12 = December
changes-comment = Comment: { $text }
changes-comment-by = Comment by { $author }: { $text }
changes-replies =
    { $n ->
        [one] { $n } reply
       *[other] { $n } replies
    }
changes-resolved = resolved
# $kind is one of the changes-kind-* words.
changes-accepted = Accepted. { $kind }: '{ $text }'.
changes-rejected = Rejected. { $kind }: '{ $text }'.
changes-accepted-all =
    { $n ->
        [one] Accepted { $n } change.
       *[other] Accepted { $n } changes.
    }
changes-rejected-all =
    { $n ->
        [one] Rejected { $n } change.
       *[other] Rejected { $n } changes.
    }
changes-accepted-author =
    { $n ->
        [one] Accepted { $n } change by { $author }.
       *[other] Accepted { $n } changes by { $author }.
    }
changes-rejected-author =
    { $n ->
        [one] Rejected { $n } change by { $author }.
       *[other] Rejected { $n } changes by { $author }.
    }
changes-none-left = No tracked changes to accept or reject.
changes-not-a-change = This row is a comment. F2 replies, Space resolves, Delete deletes.
changes-not-a-comment = This row is a change. A accepts it, R rejects it.
changes-edit-mode = Changes stay as they are in edit mode. Leave edit mode to accept or reject them.
changes-replied = Reply added.
changes-resolved-done = Comment resolved.
changes-reopened = Comment open again.
changes-comment-deleted = Comment and its replies deleted.
changes-comment-added = Comment added.
changes-delete-comment-question = Delete this comment and its replies? y or n
# tw changes --out: $path is the file written, $n how many changes.
changes-written-accepted =
    { $n ->
        [one] Wrote { $path } with { $n } change accepted.
       *[other] Wrote { $path } with { $n } changes accepted.
    }
changes-written-rejected =
    { $n ->
        [one] Wrote { $path } with { $n } change rejected.
       *[other] Wrote { $path } with { $n } changes rejected.
    }
## End of B1-t1
## B1-r5: braille (BRF) files read as print. $page is a braille page
## number ("3", "p1"); $n is a number of lines; $reason is an error.
setting-braille-brf-code = Braille code of BRF files
setting-braille-brf-code-help = The braille code BRF files are read in. UEB is for books made since 2016; EBAE, English Braille American Edition, is for older books. Reading a BRF file as print needs liblouis. Open the file again after a change.
choice-braille-brf-code-ueb = UEB
choice-braille-brf-code-ebae = EBAE
name-show-original-braille = Show original Braille
action-show-original-braille = Show the original braille of the page at the cursor, in a BRF file read as print
brf-original-title = Original Braille, page { $page }
brf-original-intro =
    { $n ->
        [one] Original Braille, page { $page }, { $n } line. Escape closes.
       *[other] Original Braille, page { $page }, { $n } lines. Escape closes.
    }
brf-original-not-brf = Not a braille file. Show original Braille works on BRF files.
brf-original-unreadable = Cannot read the braille file: { $reason }
brf-no-liblouis = Braille shown as braille: liblouis is missing. To read it as print, install liblouis from liblouis.io or your system's packages, then open the file again.
## End of B1-r5
