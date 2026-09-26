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