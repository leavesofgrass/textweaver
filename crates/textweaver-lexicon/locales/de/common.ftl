### Common words, parts of speech, define word, and names said inside other messages.

### Die Meldungen der Oberfläche von textweaver, auf Deutsch.
###
### Fluent (https://projectfluent.org/), in der Teilmenge, die
### textweaver-lexicon liest. Kennungen und Variablen sind die aus locales/en;
### eine hier fehlende Meldung wird auf Englisch gesagt.

-brand = textweaver

## Parts of speech.

pos-noun = Substantiv
pos-verb = Verb
pos-adjective = Adjektiv
pos-adverb = Adverb

## Define word: sources.

source-glossary = Ihr Glossar
source-wordnet = Open English WordNet
source-cmudict = das CMU-Ausspracheverzeichnis

## Define word: the list of senses.

# $word is the word looked up, $n the number of senses, $source a source-* message.
define-title =
    { $n ->
        [0] Aussprache von { $word }, aus { $source }
        [one] Definitionen von { $word }, 1 Bedeutung, aus { $source }
       *[other] Definitionen von { $word }, { $n } Bedeutungen, aus { $source }
    }
# $lemma is the headword, $pos a pos-* message, $i the sense number, $n how many.
define-sense-head = { $lemma }, { $pos }, { $i } von { $n }
define-sense-head-nopos = { $lemma }, { $i } von { $n }
define-sense = { $head }: { $definition }.
define-example = Zum Beispiel: { $text }.
define-synonyms = Synonyme: { $words }.
define-antonyms = Gegenteil: { $words }.
define-kind-of = Eine Art von: { $words }.
# $say is a respelling such as RUN-ing, with the stressed syllable in capitals.
define-pronounced = Ausgesprochen { $say }.
define-pronounced-or = Ausgesprochen { $say }, oder { $other }.

## Common words.

common-cancelled = Abgebrochen.
# Durations: $h hours, $m minutes, $s seconds.
duration-hours =
    { $h ->
        [one] 1 Stunde
       *[other] { $h } Stunden
    } und { $m ->
        [one] 1 Minute
       *[other] { $m } Minuten
    }
duration-minutes =
    { $m ->
        [one] 1 Minute
       *[other] { $m } Minuten
    } und { $s ->
        [one] 1 Sekunde
       *[other] { $s } Sekunden
    }
duration-seconds =
    { $s ->
        [one] 1 Sekunde
       *[other] { $s } Sekunden
    }

## Define word in the reader.

# $title is define-title.
define-intro = { $title }. Auf und Ab bewegen sich durch die Bedeutungen, Eingabetaste kopiert eine, Escape schließt.
define-nothing-here = Am Cursor steht kein Wort.
define-not-found = Keine Definition gefunden für { $word }.
define-no-dictionary = Die Wörterbuchdatei ist nicht installiert, daher wurde nur Ihr Glossar durchsucht. Die Lesehilfe beschreibt die Installation.
define-dictionary-damaged = Die Wörterbuchdatei konnte nicht gelesen werden: { $error } Nur Ihr Glossar wird durchsucht.
define-glossary-problem = Ihr Glossar konnte nicht gelesen werden: { $error } Nur das Wörterbuch wird durchsucht.
define-glossary-skipped =
    { $n ->
        [one] 1 Zeile Ihres Glossars hat keine Definition und wurde übersprungen.
       *[other] { $n } Zeilen Ihres Glossars haben keine Definition und wurden übersprungen.
    }
define-copied = Kopiert.

## Names of structure, units, and modes (said inside other messages).

kind-heading = Überschrift
kind-paragraph = Absatz
kind-list-item = Listenelement
kind-list = Liste
kind-table = Tabelle
kind-row = Zeile
kind-cell = Zelle
kind-link = Link
kind-graphic = Grafik
kind-code = Code
kind-quote = Blockzitat
kind-page = Seite
kind-section = Abschnitt
kind-bold = Fett
kind-italic = Kursiv
kind-underline = Unterstrichen
kind-footnote = Fußnote
kind-strikethrough = Durchgestrichen
kind-separator = Trenner
kind-math = Mathe
unit-character = Zeichen
unit-word = Wort
unit-sentence = Satz
unit-line = Zeile
unit-paragraph = Absatz
unit-document = Dokument
# $what is a kind-* noun, $unit its key.
unit-with-level = { $what } Ebene { $level }
mode-browse = Durchsuchen
mode-speech-cursor = Lesecursor
mode-edit = Bearbeiten
mode-find = Suchen
mode-command = Befehl
mode-go-to = Gehe zu
mode-open = Öffnen
mode-prompt = Eingabe
common-on = an
common-off = aus

## The interface language. $language is the language's name in itself
## (Español), $voice a voice's name.

language-voice-changed = Die Stimme ist jetzt { $voice }, für { $language }.
language-voice-kept = Keine Stimme für { $language } in dieser Sprachausgabe, daher spricht { $voice } weiter.
language-list-title = Sprache
language-list-intro =
    { $n ->
        [one] Sprache, 1 Auswahl. Auf und Ab bewegen, Eingabetaste wählt, Escape behält die Sprache.
       *[other] Sprache, { $n } Auswahlmöglichkeiten. Auf und Ab bewegen, Eingabetaste wählt, Escape behält die Sprache.
    }

## Characters and selections, as spoken.

# The name of a white-space character read on its own; $name is a fixed key.
text-char-name =
    { $name ->
        [space] Leerzeichen
        [new-line] Zeilenumbruch
        [tab] Tabulator
        [no-break-space] geschütztes Leerzeichen
       *[white-space] Leerraum
    }
# Said after a selection grows ($change is selected) or shrinks (unselected); $text is the text or a character's name.
text-selection-change =
    { $change ->
        [selected] { $text } ausgewählt
       *[unselected] { $text } abgewählt
    }

text-summary =
    { $change ->
        [selected] { $count } Zeichen ausgewählt
        [unselected] { $count } Zeichen abgewählt
        [copied] { $count } Zeichen kopiert
        [cut] { $count } Zeichen ausgeschnitten
       *[deleted] { $count } Zeichen gelöscht
    }
text-summary-range =
    { $change ->
        [selected] { $count } Zeichen ausgewählt, von { $first } bis { $last }
        [unselected] { $count } Zeichen abgewählt, von { $first } bis { $last }
        [copied] { $count } Zeichen kopiert, von { $first } bis { $last }
        [cut] { $count } Zeichen ausgeschnitten, von { $first } bis { $last }
       *[deleted] { $count } Zeichen gelöscht, von { $first } bis { $last }
    }
voice-character-keys-on = Einzeltasten-Kurzbefehle an.
voice-character-keys-off = Einzeltasten-Kurzbefehle aus.
