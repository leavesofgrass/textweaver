### Common words, parts of speech, define word, and names said inside other messages.

### textweaver's interface messages, in English.
###
### The format is Fluent (https://projectfluent.org/), in the subset that
### textweaver-lexicon reads: messages, terms, comments, variables, and
### one-line select variants. Every message the code asks for must be in
### one of this folder's files, each file one area of the interface; a test
### in textweaver-lexicon checks that, and that no id is defined twice.
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
