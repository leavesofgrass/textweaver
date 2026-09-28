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
