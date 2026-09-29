### Die Meldungen der Oberfläche von textweaver, auf Deutsch.
###
### Fluent (https://projectfluent.org/), in der Teilmenge, die
### textweaver-lexicon liest. Kennungen und Variablen sind die aus en.ftl;
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

## Prompts.

prompt-define-word = Welches Wort definieren?
prompt-profile-name = Name für das neue Profil
prompt-profile-rename = Neuer Name für das Profil, Eingabetaste behält ihn
prompt-profiles-import = Profile aus Datei importieren
prompt-profiles-export = Profile in Datei exportieren, zum Beispiel textweaver-profiles.json

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
define-dictionary-damaged = Die Wörterbuchdatei konnte nicht gelesen werden: { $error }
define-glossary-problem = Ihr Glossar konnte nicht gelesen werden: { $error }
define-glossary-skipped =
    { $n ->
        [one] 1 Zeile Ihres Glossars hat keine Definition und wurde übersprungen.
       *[other] { $n } Zeilen Ihres Glossars haben keine Definition und wurden übersprungen.
    }
define-copied = Kopiert.

## Settings profiles.

profiles-title =
    { $n ->
        [0] Einstellungsprofile, noch keins gespeichert
        [one] Einstellungsprofile, 1 Profil
       *[other] Einstellungsprofile, { $n } Profile
    }
# $title is profiles-title.
profiles-intro = { $title }. Eingabetaste wechselt zu einem Profil, F2 benennt es um, Entf löscht es.
# $summary is profile-summary-* parts joined by commas.
profiles-item = { $name }: { $summary }
profiles-item-active = { $name }, in Gebrauch: { $summary }
profiles-save-new = Aktuelle Einstellungen als neues Profil speichern
profiles-update = Aktuelle Einstellungen in { $name } speichern
profiles-import = Profile aus einer Datei importieren
profiles-export = Alle Profile in eine Datei exportieren
profile-summary-voice = Stimme { $voice }
profile-summary-rate = Geschwindigkeit { $rate }
profile-summary-theme = Design { $theme }
# $mode is self voicing, hybrid, or screen reader.
profile-summary-access = Modus { $mode }
profile-summary-empty = nichts gespeichert
profile-switched = Zu { $name } gewechselt.
profile-switched-backend = Zu { $name } gewechselt. Die Sprachausgabe wird ab dem nächsten Start verwendet.
# $keys lists settings such as speech.pitch.
profile-dropped =
    { $n ->
        [one] 1 Einstellung darin wird von dieser Version nicht verwendet: { $keys }.
       *[other] { $n } Einstellungen darin werden von dieser Version nicht verwendet: { $keys }.
    }
profile-saved = Aktuelle Einstellungen als { $name } gespeichert.
profile-replaced = Aktuelle Einstellungen in { $name } gespeichert.
profile-renamed = { $old } in { $new } umbenannt.
profile-delete-question = Profil { $name } löschen? y oder n
profile-deleted = { $name } gelöscht.
profile-kept = Beibehalten.
profile-not-found = Es gibt kein Profil namens { $name }.
profile-needs-name = Ein Profil braucht einen Namen.
profile-exists = Ein Profil namens { $name } existiert bereits.
profiles-not-an-export = { $detail }
profiles-no-persistence = Profile werden in dieser Sitzung nicht gespeichert.
profiles-read-failed = Die Profildatei konnte nicht gelesen werden, daher wird sie als leer behandelt: { $error }
profiles-save-failed = Die Profile konnten nicht gespeichert werden: { $error }
profiles-none-to-export = Es gibt noch keine Profile zu exportieren.
profiles-exported =
    { $n ->
        [one] 1 Profil nach { $file } exportiert.
       *[other] { $n } Profile nach { $file } exportiert.
    }
profiles-export-failed = Die Profile konnten nicht exportiert werden: { $error }
profiles-imported =
    { $n ->
        [0] Es waren keine Profile in { $file }.
        [one] 1 Profil aus { $file } importiert: { $names }.
       *[other] { $n } Profile aus { $file } importiert: { $names }.
    }

## Reading statistics.

stats-title = Lesestatistik
stats-intro = Lesestatistik. Eingabetaste bei einem Dokument öffnet es.
stats-off = Die Lesestatistik ist aus. Der letzte Eintrag schaltet sie ein.
stats-empty = Noch keine Lesezeit erfasst. Die Zeit wird gezählt, während textweaver vorliest.
# $time is a duration-* message.
stats-total =
    { $time } insgesamt gelesen, in { $sessions ->
        [one] 1 Sitzung
       *[other] { $sessions } Sitzungen
    }, über { $docs ->
        [one] 1 Dokument
       *[other] { $docs } Dokumente
    }.
stats-current =
    Dieses Dokument: { $time } gelesen, weitester Punkt { $pct } Prozent, { $sessions ->
        [one] 1 Sitzung
       *[other] { $sessions } Sitzungen
    }.
stats-current-none = Dieses Dokument wurde noch nicht vorgelesen.
stats-most-read = Meistgelesen { $rank }: { $title }, { $time }, weitester Punkt { $pct } Prozent.
stats-toggle-on = Statistik ist an. Eingabetaste schaltet sie aus.
stats-toggle-off = Statistik ist aus. Eingabetaste schaltet sie ein.
stats-turned-on = Lesestatistik ist an.
stats-turned-off = Lesestatistik ist aus. Aufgezeichnetes bleibt erhalten; tw stats --clear entfernt es.

## Lists.

study-nothing-to-delete = Nichts zu löschen in dieser Liste.
study-nothing-to-rename = Nichts umzubenennen in dieser Liste.
## tw stats.

stats-clear-question =
    { $n ->
        [one] Lesestatistik von 1 Dokument entfernen? y oder n
       *[other] Lesestatistik von { $n } Dokumenten entfernen? y oder n
    }
stats-cleared = Lesestatistik entfernt.
stats-off-cli = Die Lesestatistik ist aus: stats.enabled ist in den Einstellungen false.

## Navigation. $dir is next or previous; $what is a kind-* or unit-*
## noun and $unit its key (heading, list-item, sentence), for languages
## whose words agree with the noun.

nav-blank = leer
# High verbosity: $label is a structure label ("Heading level 2").
nav-message-at-labelled = { $label }, Zeile { $line }, { $pct } Prozent: { $content }
nav-message-at = Zeile { $line }, { $pct } Prozent: { $content }
nav-message-labelled = { $label }: { $content }
# $message is the navigation message after wrapping around.
nav-wrapped = Von vorn begonnen. { $message }
nav-no-next =
    { $unit ->
        [heading] Keine nächste { $what }.
        [list] Keine nächste { $what }.
        [table] Keine nächste { $what }.
        [row] Keine nächste { $what }.
        [cell] Keine nächste { $what }.
        [graphic] Keine nächste { $what }.
        [page] Keine nächste { $what }.
        [footnote] Keine nächste { $what }.
        [math] Keine nächste { $what }.
        [line] Keine nächste { $what }.
        [list-item] Kein nächstes { $what }.
        [quote] Kein nächstes { $what }.
        [character] Kein nächstes { $what }.
        [word] Kein nächstes { $what }.
        [document] Kein nächstes { $what }.
       *[other] Kein nächster { $what }.
    }
nav-no-previous =
    { $unit ->
        [heading] Keine vorherige { $what }.
        [list] Keine vorherige { $what }.
        [table] Keine vorherige { $what }.
        [row] Keine vorherige { $what }.
        [cell] Keine vorherige { $what }.
        [graphic] Keine vorherige { $what }.
        [page] Keine vorherige { $what }.
        [footnote] Keine vorherige { $what }.
        [math] Keine vorherige { $what }.
        [line] Keine vorherige { $what }.
        [list-item] Kein vorheriges { $what }.
        [quote] Kein vorheriges { $what }.
        [character] Kein vorheriges { $what }.
        [word] Kein vorheriges { $what }.
        [document] Kein vorheriges { $what }.
       *[other] Kein vorheriger { $what }.
    }
nav-nothing-to-read = { $what }: nichts zu lesen.
nav-label-heading-level = Überschriftsebene { $level }
nav-label-list =
    { $n ->
        [0] Liste
        [one] Liste, 1 Eintrag
       *[other] Liste, { $n } Einträge
    }
nav-label-table =
    { $n ->
        [0] Tabelle
        [one] Tabelle, 1 Zeile
       *[other] Tabelle, { $n } Zeilen
    }
nav-label-list-item-level = Listenelement, Ebene { $level }
nav-no-heading-level =
    { $dir ->
        [next] Keine weitere Überschrift auf Ebene { $level }.
       *[previous] Keine vorherige Überschrift auf Ebene { $level }.
    }
# Said before a line's text on caret moves.
nav-line-heading = Überschriftsebene { $level }
nav-line-row = Zeile { $n }
nav-line-list-item-level = Listenelement, Ebene { $level }
# $language is a programming language name such as Python.
nav-line-code-language = Code, { $language }
nav-no-chapters = Dieses Dokument hat keine Kapitel.
nav-chapter = Kapitel
nav-no-chapter =
    { $dir ->
        [next] Kein weiteres Kapitel.
       *[previous] Kein vorheriges Kapitel.
    }
nav-back = Zurück
nav-forward = Vorwärts
nav-page = Seite
nav-percent = { $pct } Prozent
nav-no-earlier-history = Kein früherer Verlauf.
nav-no-forward-history = Kein vorwärtsgerichteter Verlauf.
# $label is nav-back, nav-forward, nav-page, or nav-percent.
nav-label-line = { $label }, Zeile { $line }
nav-top-of-document = Anfang des Dokuments
nav-end-of-document = Ende des Dokuments
# $edge is nav-top-of-document or nav-end-of-document; $content the line there.
nav-edge-message = { $edge }. { $content }
nav-top-of-document-stop = Anfang des Dokuments.
nav-end-of-document-stop = Ende des Dokuments.
nav-position = Zeile { $line } von { $lines }, { $pct } Prozent.
nav-position-word = Wort { $word } von { $words }.
nav-position-heading = Unter Überschrift { $heading }.
nav-position-mode = Modus { $mode }.

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

## Reading aloud.

playback-caps-no-words = Diese Stimme meldet keine Wörter, daher wird die Wort-Hervorhebung geschätzt.
playback-caps-words = Diese Stimme meldet jedes Wort, daher folgt die Hervorhebung genau.
playback-caps-no-pitch = Die Tonhöhe kann bei dieser Stimme nicht geändert werden.
playback-caps-no-volume = Die Lautstärke kann bei dieser Stimme nicht geändert werden.
# The reading state on the title line, one word.
state-reading = Lesen
state-paused = Pausiert
state-stopped = Gestoppt
state-ready = Bereit
playback-reading-at = Lesen mit { $rate } Wörtern pro Minute.
playback-paused = Pausiert.
playback-stopped-speech-cursor-off = Gestoppt. Lesecursor aus.
playback-stopped = Gestoppt.
playback-search-cleared = Suche gelöscht.
# $key is the key that turns edit mode off.
playback-still-editing = Noch in Bearbeitung. { $key } beendet sie.
playback-end-of-document-content = Ende des Dokuments
playback-no-unit-here = { $what }: nichts hier.
playback-no-selection = Keine Auswahl.
# $next is what happens now (a restart, or nothing).
playback-speech-died = Die Sprachausgabe hat aufgehört zu arbeiten ({ $reason }). { $next }
playback-done-reading = Lesen beendet.
playback-speech-restarted = Sprachausgabe neu gestartet: { $reason }. Weiter ab dem letzten Wort.
playback-speech-error = Fehler der Sprachausgabe: { $error }

## The title line and Say Status.

# The title line's position; % is shown, not said.
status-position = Zeile { $line } von { $lines }, { $pct }%
status-mode = Modus { $mode }
status-modified = geändert
status-self-voicing = Selbstsprechend
status-hybrid = Hybrid
status-screen-reader = Screenreader-Modus
status-rate-spoken = { $wpm } Wörter pro Minute
status-rate = { $wpm } wpm
status-no-document = Kein Dokument
# $parts are the title line's parts, joined with commas.
status-said = { $title }: { $parts }.
status-no-message = Noch keine Meldung.
# A list shown without its own introduction.
status-list-intro =
    { $n ->
        [one] { $title }, 1 Eintrag. Auf und Ab bewegen, Eingabetaste wählt, Escape schließt.
       *[other] { $title }, { $n } Einträge. Auf und Ab bewegen, Eingabetaste wählt, Escape schließt.
    }

## Questions and answers. Keep the letters y and n: they are the keys
## that answer.

common-press-y-or-n = Drücken Sie y oder n.
common-kept = Beibehalten.
confirm-quit = textweaver beenden? y oder n
confirm-delete-note = Diese Notiz oder Hervorhebung löschen? y oder n
notes-remove-highlight-question = Diese Hervorhebung entfernen? y oder n
notes-delete-note-question = Diese Notiz löschen? y oder n
list-nothing-to-mark = Nichts zu markieren in dieser Liste.
notes-editing = Notiz wird bearbeitet: { $text }
# $key opens a document.
app-no-document-open = Kein Dokument ist geöffnet. Drücken Sie { $key }, um eines zu öffnen.
app-window-only = Dieser Befehl funktioniert im textweaver-Fenster.
settings-save-failed = Einstellungen konnten nicht gespeichert werden: { $error }
edit-still-editing = Noch in Bearbeitung.
goto-not-a-target = Kein Sprungziel: { $text }. Geben Sie eine Zeilennummer ein, einen Prozentwert wie 50%, start oder end.

## Opening a document.

open-opened = { $title } geöffnet.
open-resumed = { $title } geöffnet. Fortgesetzt bei { $pct } Prozent.
open-resumed-synced = { $title } geöffnet. Fortgesetzt bei { $pct } Prozent, von einem anderen Gerät.
open-resumed-conflict = { $title } geöffnet. Fortgesetzt bei { $pct } Prozent. Ein anderes Gerät ist an einer anderen Stelle; die Stelle dieses Geräts wurde beibehalten.

## Prompts: the label is shown and said when the prompt opens.

prompt-find = Suchen
prompt-go-to = Gehe zu Zeile, Prozent, start oder end
prompt-open = Datei öffnen
prompt-command = Befehl
# $label is prompt-command.
prompt-command-palette-intro = { $label }. Geben Sie einen Teil eines Namens ein; Tabulator vervollständigt, Auf und Ab listen Treffer.
prompt-save-as = Speichern unter
prompt-table-size = Tabellengröße, Spalten mal Zeilen, zum Beispiel 3 mal 2
prompt-image-path = Bilddatei
prompt-replace-find = Ersetzen, wonach suchen
prompt-replace-with = Ersetzen durch
prompt-note = Notiz
prompt-edit-note = Notiz bearbeiten, Eingabetaste behält sie
prompt-rename-bookmark = Neuer Name für das Lesezeichen, Eingabetaste behält ihn
prompt-export-settings = Einstellungen in Datei exportieren, zum Beispiel textweaver-settings.json
prompt-import-settings = Einstellungen aus Datei importieren
prompt-citation-locator = Seite oder anderer Fundort, zum Beispiel 12 oder Kapitel 2; Eingabetaste für keinen
prompt-reference-identifier = DOI oder ISBN zum Hinzufügen
prompt-import-references = Literaturangaben aus Datei importieren
prompt-template-title = Titel des neuen Dokuments
prompt-setting-value = Neuer Wert, Eingabetaste behält ihn

## Keys named in messages and the help.

help-the-command-palette = die Befehlspalette
help-not-bound = nicht belegt
# Two keys, or a key and a list of keys: "p or Ctrl+P".
help-or = { $a } oder { $b }
# A command without keys: $name is its palette name, such as list highlights.
help-the-command = der Befehl { $name }
# One line of the keyboard shortcuts list: a category-* title, an action-*
# help, and its keys.
help-entry = { $category }: { $help }. { $keys }
# One command palette candidate: its id (not translated), help, and keys.
help-palette-item = { $id }: { $help }. { $keys }
help-unknown-command = Unbekannter Befehl: { $text }.
help-shortcuts-intro = Tastenkombinationen, { $n } Befehle. Auf und Ab bewegen, Eingabetaste führt aus, Escape schließt.
help-shortcuts-title = Tastenkombinationen
help-title = Hilfe
help-intro = Hilfe. Auf und Ab bewegen, Escape schließt.

## The help list. Each value is a key or keys from the keymap.

help-about = textweaver liest Dokumente vor. Die Tasten unten sind die aktuellen Belegungen.
help-open = Ein Dokument öffnen: { $open }. Bibliothek und zuletzt verwendete Dateien: { $library }.
help-play = Abspielen oder pausieren: { $key }.
help-read-from-cursor = Ab dem Cursor lesen: { $key }.
help-stop = Stopp: { $key }.
help-sentences = Nächster und vorheriger Satz: { $next } und { $previous }.
help-paragraphs = Nächster und vorheriger Absatz: { $next } und { $previous }.
help-headings = Nächste und vorherige Überschrift: { $next } und { $previous }. Überschrift auf einer Ebene: { $first } bis { $last }, mit Umschalt für die vorherige.
help-read-headings = Ab der nächsten und vorherigen Überschrift lesen: { $next } und { $previous }.
help-quick-keys = Schnelltasten, wie in NVDA und JAWS: Liste { $list }, Listenelement { $item }, Tabelle { $table }, Link { $link }, Blockzitat { $quote }, Trenner { $separator }, Grafik { $graphic }, Abschnitt { $section }. Umschalt mit der Taste geht zur vorherigen.
help-speech-cursor = Lesecursor, Zeile für Zeile: { $key }.
help-find = Suchen: { $key }.
help-bookmark = Lesezeichen hinzufügen: { $key }.
help-history = Zurück und vorwärts durch Ihre Sprünge: { $back } und { $forward }.
help-rate = Schneller und langsamer: { $faster } und { $slower }.
help-where = Wo bin ich: { $key }.
help-repeat = Die letzte Meldung noch einmal hören: { $repeat }. Die letzte Meldung und der Status: Modus, Geschwindigkeit, Engine und Position: { $status }.
help-notes = Notizen: hinzufügen { $add }, auflisten { $list }, nächste und vorherige { $next } und { $previous }, die am Cursor löschen { $delete }. In der Liste löscht Entf, F2 bearbeitet.
help-highlights = Die Auswahl oder den Satz hervorheben, oder eine Hervorhebung entfernen: { $highlight }. Hervorhebungen auflisten: { $list }.
help-bookmarks-list = Lesezeichenliste: Entf löscht ein Lesezeichen, F2 benennt es um.
help-edit = Das Dokument bearbeiten: { $edit }. Speichern: { $save }. Speichern unter: { $saveas }. Neues Dokument: { $new }.
help-editing = Beim Bearbeiten: rückgängig { $undo }, wiederholen { $redo }, fett { $bold }. Jeder Formatierungsbefehl steht in den Tastenkombinationen.
help-outline = Gliederung der Überschriften, Tippen filtert: { $outline }. Einem Link oder einer Fußnote folgen: { $follow }.
help-tables = Tabellen: { $nextrow } und { $previousrow } bewegen zeilenweise, { $nextcell } und { $previouscell } zellenweise.
help-citations = Zitate beim Bearbeiten: einfügen { $insert }, eine Literaturangabe per DOI oder ISBN hinzufügen { $reference }. Rechtschreibung: nächster und vorheriger Fehler { $next } und { $previous }, Vorschläge { $suggestions }.
help-export = Export als HTML, PDF, Word, EPUB oder Braille, Vorschau im Browser, und Start von einer Vorlage: Geben Sie export, preview oder template in die Befehlspalette ein.
help-verbosity = Wie viel gesagt wird: { $verbosity }. Wie viel Interpunktion: { $punctuation }.
help-voice = Eine Stimme wählen: { $voice }. Sprachausgabe neu starten, wenn sie aussetzt: { $restart }.
help-access = Mit einem Screenreader, wer spricht: { $key } wechselt zwischen Selbstsprechend, Hybrid und Screenreader-Modus.
help-character-keys = Einzeltasten-Kurzbefehle an oder aus, für Diktat: { $keys }. Einstellungen: { $settings }.
help-all-shortcuts = Alle Tastenkombinationen: { $key }.
help-palette = Jeden Befehl über seinen Namen ausführen: { $key }.
# Keep the letters y, n, and a: they are the keys that answer.
help-quit = Beenden, mit Speichern Ihrer Stelle: { $key }, dann y zum Bestätigen; n, a oder Escape bricht ab.

## Help categories.

category-reading = Lesen
category-navigation = Navigation
category-speech-cursor = Lesecursor
category-voice = Stimme
category-search = Suchen
category-bookmarks = Lesezeichen und Notizen
category-file = Datei
category-editing = Bearbeiten
category-view = Ansicht und Hilfe

## Key names as textweaver's own voice says them. Written names (Ctrl+S)
## are not translated.

keyname-control = Strg
keyname-command = Command
keyname-alt = Alt
keyname-shift = Umschalt
keyname-period = Punkt
keyname-comma = Komma
keyname-semicolon = Semikolon
keyname-colon = Doppelpunkt
keyname-apostrophe = Apostroph
keyname-quote = Anführungszeichen
keyname-grave-accent = Gravis
keyname-tilde = Tilde
keyname-exclamation-mark = Ausrufezeichen
keyname-question-mark = Fragezeichen
keyname-at-sign = At-Zeichen
keyname-number-sign = Rautezeichen
keyname-dollar-sign = Dollarzeichen
keyname-percent = Prozentzeichen
keyname-caret = Zirkumflex
keyname-ampersand = Kaufmanns-Und
keyname-asterisk = Sternchen
keyname-left-parenthesis = runde Klammer auf
keyname-right-parenthesis = runde Klammer zu
keyname-left-bracket = eckige Klammer auf
keyname-right-bracket = eckige Klammer zu
keyname-left-brace = geschweifte Klammer auf
keyname-right-brace = geschweifte Klammer zu
keyname-less-than = Kleiner-als-Zeichen
keyname-greater-than = Größer-als-Zeichen
keyname-plus = Plus
keyname-minus = Minus
keyname-equals = Gleichheitszeichen
keyname-underscore = Unterstrich
keyname-slash = Schrägstrich
keyname-backslash = Rückstrich
keyname-vertical-bar = senkrechter Strich
keyname-page-up = Bild auf
keyname-page-down = Bild ab
keyname-up-arrow = Pfeil nach oben
keyname-down-arrow = Pfeil nach unten
keyname-left-arrow = Pfeil nach links
keyname-right-arrow = Pfeil nach rechts
keyname-escape = Escape
keyname-space = Leertaste
keyname-enter = Eingabetaste
keyname-tab = Tabulator
keyname-backspace = Rücktaste
keyname-delete = Entf
keyname-insert = Einfg
keyname-home = Pos1
keyname-end = Ende

## Commands: the one-line help of each, in the keyboard shortcuts list and
## the command palette. Their ids (play_pause) stay as they are.

action-play-pause = Lesen ab dem aktuellen Wort abspielen oder pausieren
action-stop = Lesen stoppen
action-read-from-cursor = Fortlaufend ab dem Cursor lesen
action-read-document = Das ganze Dokument von Anfang an lesen
action-read-current-character = Das Zeichen am Cursor sagen
action-read-current-word = Das Wort am Cursor sagen
action-read-current-sentence = Den Satz am Cursor sagen, ohne die Stelle zu wechseln
action-read-current-line = Die Zeile am Cursor sagen
action-read-paragraph = Den Absatz am Cursor sagen, ohne die Stelle zu wechseln
action-read-selection = Den ausgewählten Text lesen
action-say-position = Die Position sagen: Zeile, Prozent, Wortnummer und Überschrift
action-say-status = Die letzte Meldung noch einmal sagen, dann den Status: Modus, Lesezustand, Position, Geschwindigkeit und Sprachausgabe; in einer Liste deren Einleitung
action-repeat-message = Die letzte Meldung noch einmal sagen
action-word-count = Sagen, wie viele Wörter im Dokument oder in der Auswahl stehen
action-link-address = Die Adresse des Links am Cursor sagen
action-replay-sentence = Ab dem Anfang des aktuellen Satzes erneut lesen
action-replay-paragraph = Ab dem Anfang des aktuellen Absatzes erneut lesen
action-rsvp-toggle = RSVP anzeigen oder ausblenden: ein Wort nach dem anderen, ab dem Cursor
action-rsvp-play-pause = RSVP starten oder pausieren
action-rsvp-faster = RSVP schneller
action-rsvp-slower = RSVP langsamer
action-rsvp-position-next = Das RSVP-Wort an die nächste Stelle auf dem Bildschirm bewegen
action-reading-level = Das Leseniveau des Dokuments oder der Auswahl sagen
action-define-word = Das Wort am Cursor oder die ausgewählten Wörter definieren: Bedeutungen, Beispiele, Synonyme und Aussprache
action-toggle-citations = Zitate beim fortlaufenden Lesen ein- oder ausschalten: aus überspringt sie, an sagt sie in Worten
action-explore-math = Die Mathematik am Cursor Term für Term erkunden: Pfeile bewegen, Ab geht in einen Teil hinein, Auf kommt heraus, Escape verlässt
action-listen-rendered = Dem Dokument zuhören, wie es dargestellt wird, ohne den Bearbeitungsmodus zu verlassen
action-next-sentence = Zum nächsten Satz bewegen
action-previous-sentence = Zum vorherigen Satz bewegen, oder zum Anfang des aktuellen, wenn mehr als drei Wörter gelesen sind
action-next-paragraph = Zum nächsten Absatz bewegen
action-previous-paragraph = Zum vorherigen Absatz bewegen
action-next-heading = Ab der nächsten Überschrift lesen
action-previous-heading = Ab der vorherigen Überschrift lesen
action-skip-next-heading = Zur nächsten Überschrift bewegen, ohne zu lesen
action-skip-previous-heading = Zur vorherigen Überschrift bewegen, ohne zu lesen
action-outline = Die Überschriften auflisten: Tippen filtert, Eingabetaste springt zu einer
action-next-heading-level-1 = Zur nächsten Überschrift auf Ebene 1 bewegen
action-next-heading-level-2 = Zur nächsten Überschrift auf Ebene 2 bewegen
action-next-heading-level-3 = Zur nächsten Überschrift auf Ebene 3 bewegen
action-next-heading-level-4 = Zur nächsten Überschrift auf Ebene 4 bewegen
action-next-heading-level-5 = Zur nächsten Überschrift auf Ebene 5 bewegen
action-next-heading-level-6 = Zur nächsten Überschrift auf Ebene 6 bewegen
action-previous-heading-level-1 = Zur vorherigen Überschrift auf Ebene 1 bewegen
action-previous-heading-level-2 = Zur vorherigen Überschrift auf Ebene 2 bewegen
action-previous-heading-level-3 = Zur vorherigen Überschrift auf Ebene 3 bewegen
action-previous-heading-level-4 = Zur vorherigen Überschrift auf Ebene 4 bewegen
action-previous-heading-level-5 = Zur vorherigen Überschrift auf Ebene 5 bewegen
action-previous-heading-level-6 = Zur vorherigen Überschrift auf Ebene 6 bewegen
action-next-table = Zur nächsten Tabelle bewegen
action-previous-table = Zur vorherigen Tabelle bewegen
action-next-list = Zur nächsten Liste bewegen
action-previous-list = Zur vorherigen Liste bewegen
action-next-list-item = Zum nächsten Listenelement bewegen
action-previous-list-item = Zum vorherigen Listenelement bewegen
action-next-link = Zum nächsten Link bewegen
action-previous-link = Zum vorherigen Link bewegen
action-next-block-quote = Zum nächsten Blockzitat bewegen
action-previous-block-quote = Zum vorherigen Blockzitat bewegen
action-next-separator = Zum nächsten Trenner bewegen (horizontale Linie)
action-previous-separator = Zum vorherigen Trenner bewegen (horizontale Linie)
action-next-graphic = Zur nächsten Grafik bewegen (Bild)
action-previous-graphic = Zur vorherigen Grafik bewegen (Bild)
action-follow-link = Dem Link am Cursor folgen, oder zwischen einer Fußnote und ihrer Notiz wechseln
action-table-next-row = In einer Tabelle eine Zeile nach unten in derselben Spalte bewegen
action-table-previous-row = In einer Tabelle eine Zeile nach oben in derselben Spalte bewegen
action-table-next-column = In einer Tabelle zur nächsten Zelle in der Zeile bewegen
action-table-previous-column = In einer Tabelle zur vorherigen Zelle in der Zeile bewegen
action-next-chapter = Zum nächsten Kapitel oder Abschnitt bewegen
action-previous-chapter = Zum vorherigen Kapitel oder Abschnitt bewegen
action-history-back = Zurückgehen, wohin Sie vor dem letzten Sprung waren
action-history-forward = Nach einem Zurückgehen wieder vorwärtsgehen
action-go-to = Zu einer Zeile, einem Prozentwert oder einer Position springen
action-document-start = Zum Anfang des Dokuments bewegen
action-document-end = Zum Ende des Dokuments bewegen
action-caret-next-word = Den Cursor zum nächsten Wort bewegen
action-caret-previous-word = Den Cursor zum vorherigen Wort bewegen
action-caret-next-line = Den Cursor zur nächsten Zeile bewegen
action-caret-previous-line = Den Cursor zur vorherigen Zeile bewegen
action-select-next-word = Die Auswahl bis zum nächsten Wort erweitern
action-select-previous-word = Die Auswahl bis zum vorherigen Wort erweitern
action-select-next-line = Die Auswahl bis zur nächsten Zeile erweitern
action-select-previous-line = Die Auswahl bis zur vorherigen Zeile erweitern
action-page-down = Einen Bildschirm nach unten bewegen
action-page-up = Einen Bildschirm nach oben bewegen
action-scroll-down = Eine Zeile nach unten scrollen, ohne den Cursor zu bewegen
action-scroll-up = Eine Zeile nach oben scrollen, ohne den Cursor zu bewegen
action-speech-cursor-toggle = Den Lesecursor-Modus (zeilenweise) betreten oder verlassen
action-speech-cursor-next-line = Lesecursor: die nächste Zeile lesen
action-speech-cursor-previous-line = Lesecursor: die vorherige Zeile lesen
action-speech-cursor-reread-line = Lesecursor: die aktuelle Zeile noch einmal lesen
action-speech-cursor-exit-and-read = Lesecursor: verlassen und ab dieser Zeile weiterlesen
action-rate-up = Schneller sprechen
action-rate-down = Langsamer sprechen
action-pitch-up = Tonhöhe anheben
action-pitch-down = Tonhöhe absenken
action-volume-up = Lauter
action-volume-down = Leiser
action-cycle-speed-preset = Durch die Geschwindigkeitsvorgaben wechseln (überfliegen, normal, lernen, langsam)
action-choose-voice = Eine Stimme wählen
action-restart-speech = Die Sprachausgabe mit den aktuellen Einstellungen neu starten (nachdem sie aufgehört hat zu arbeiten)
action-cycle-verbosity = Durch die Ausführlichkeit wechseln: niedrig, normal, hoch
action-cycle-punctuation = Durch die Interpunktionsstufe wechseln: keine, etwas, alle
action-find = Text im Dokument suchen
action-find-next = Den nächsten Treffer suchen
action-find-previous = Den vorherigen Treffer suchen
action-next-misspelling = Zum nächsten falsch geschriebenen Wort bewegen und es buchstabieren
action-previous-misspelling = Zum vorherigen falsch geschriebenen Wort bewegen und es buchstabieren
action-spelling-suggestions = Vorschläge für das falsch geschriebene Wort am Cursor auflisten, oder es zu Ihrer Wortliste hinzufügen
action-next-grammar-problem = Zum nächsten Grammatikproblem bewegen und es mit seiner Korrektur sagen
action-previous-grammar-problem = Zum vorherigen Grammatikproblem bewegen und es mit seiner Korrektur sagen
action-next-lint-problem = Im Bearbeitungsmodus zum nächsten Markdown-Lint-Problem bewegen und es sagen
action-previous-lint-problem = Im Bearbeitungsmodus zum vorherigen Markdown-Lint-Problem bewegen und es sagen
action-add-bookmark = Ein Lesezeichen am Cursor hinzufügen
action-list-bookmarks = Lesezeichen auflisten
action-next-bookmark = Zum nächsten Lesezeichen bewegen
action-previous-bookmark = Zum vorherigen Lesezeichen bewegen
action-add-note = Eine Notiz zur Auswahl oder zum Satz am Cursor hinzufügen
action-list-notes = Notizen auflisten
action-next-note = Zur nächsten Notiz bewegen
action-previous-note = Zur vorherigen Notiz bewegen
action-delete-note = Die Notiz oder Hervorhebung am Cursor löschen
action-highlight-selection = Die Auswahl oder den Satz am Cursor hervorheben
action-export-study-sheet = Die Notizen und Hervorhebungen als Markdown-Lernblatt exportieren, gruppiert nach Überschrift
action-open = Ein Dokument öffnen
action-open-path = Ein Dokument öffnen, indem Sie seinen Pfad eingeben
action-open-library = Die Bibliothek öffnen: Dokumente in Ihren Bibliotheksordnern und zuletzt verwendete Dateien
action-new-document = Ein neues Dokument im Bearbeitungsmodus beginnen
action-save = Speichern (Markdown und Text an Ort und Stelle; andere Formate als Markdown)
action-save-as = Unter neuem Namen speichern
action-export-settings = Einstellungen und Tastenüberschreibungen in eine JSON- oder TOML-Datei exportieren
action-import-settings = Einstellungen aus einer JSON- oder TOML-Datei importieren, nach einem Ja oder Nein
action-reading-statistics = Lesestatistik auflisten: Lesezeit, weitester Punkt, Sitzungen und die meistgelesenen Dokumente
action-new-from-template = Ein neues Dokument aus einer Vorlage beginnen, mit Titel, Autor, Datum und einer Überschrift für Literatur
action-export-html = Das Dokument als Webseite (HTML) daneben exportieren
action-export-pdf = Das Dokument als getaggtes PDF daneben exportieren
action-export-docx = Das Dokument als Word-Datei (DOCX) daneben exportieren
action-export-epub = Das Dokument als EPUB-Buch daneben exportieren
action-export-brf = Das Dokument als Braille (BRF) daneben exportieren
action-preview-in-browser = Das Dokument im Webbrowser als Vorschau anzeigen, mit Mathematik; jedes Speichern schreibt die Vorschau neu
action-toggle-preview-auto-reload = Das automatische Neuladen der Browser-Vorschau ein- oder ausschalten
action-toggle-preview-live = Die Live-Vorschau ein- oder ausschalten: bei automatischem Neuladen lädt die Vorschau auch neu, wenn das Tippen pausiert
action-quit = Beenden, mit Speichern der Leseposition
action-toggle-edit-mode = Zwischen Lesen und Bearbeiten wechseln
action-undo = Rückgängig
action-redo = Wiederholen
action-bold = Die Auswahl fett machen
action-italic = Die Auswahl kursiv machen
action-underline = Die Auswahl unterstreichen
action-strikethrough = Die Auswahl durchstreichen
action-inline-code = Die Auswahl als Code markieren
action-code-block = Die ausgewählten Zeilen zu einem Codeblock machen
action-insert-link = Die Auswahl zu einem Link machen
action-heading = Die aktuelle Zeile zu einer Überschrift machen
action-bullet-list = Die ausgewählten Zeilen zu einer Aufzählungsliste machen
action-numbered-list = Die ausgewählten Zeilen zu einer nummerierten Liste machen
action-block-quote = Die ausgewählten Zeilen zu einem Blockzitat machen
action-horizontal-rule = Eine horizontale Linie einfügen
action-insert-table = Eine Tabelle einfügen
action-add-table-row = Eine Zeile zur Tabelle am Cursor hinzufügen
action-insert-image = Ein Bild einfügen
action-replace = Suchen und ersetzen
action-copy = Die Auswahl oder den Satz am Cursor in die Zwischenablage kopieren
action-cut = Die Auswahl in die Zwischenablage ausschneiden
action-next-table-cell = In einer Tabelle zur nächsten Zelle bewegen und ihre Spalte sagen; sonst einen Tabulator eingeben
action-previous-table-cell = In einer Tabelle zur vorherigen Zelle bewegen und ihre Spalte sagen
action-cycle-typing-echo = Durch das Tipp-Echo wechseln: Zeichen und Wörter, Zeichen, Wörter, oder keins
action-select-all = Den ganzen Text auswählen
action-delete-word-before = Das Wort vor dem Cursor löschen
action-delete-word-after = Das Wort nach dem Cursor löschen
action-paste = Den zuletzt in textweaver kopierten oder ausgeschnittenen Text einfügen; das Einfügen des Terminals funktioniert auch
action-insert-citation = Ein Zitat einfügen: eine Literaturangabe wählen, dann eine Seite oder einen anderen Fundort angeben
action-add-reference = Eine Literaturangabe per DOI oder ISBN zu Ihrer Bibliothek hinzufügen
action-insert-bibliography = Das Literaturverzeichnis der zitierten Werke am Cursor einfügen
action-check-citations = Die Zitate prüfen: wie viele es gibt, und welche Schlüssel nicht in Ihrer Bibliothek sind
action-import-references = Literaturangaben aus einer BibTeX-, RIS- oder CSL-JSON-Datei in Ihre Bibliothek importieren
action-next-theme = Zum nächsten Farbdesign wechseln
action-toggle-line-numbers = Zeilennummern anzeigen oder ausblenden
action-toggle-character-keys = Einzeltasten-Kurzbefehle ein- oder ausschalten, damit Diktat und Tippen keine Befehle auslösen
action-cycle-access-mode = Durch den Zugänglichkeitsmodus wechseln: Selbstsprechend, Hybrid oder Screenreader
action-settings-profiles = Einstellungsprofile auflisten: zu einem wechseln, die aktuellen Einstellungen als eines speichern, umbenennen, löschen, importieren oder exportieren
action-bionic-toggle = Bionisches Lesen ein- oder ausschalten: der Anfang jedes Worts fett
action-ruler-cycle = Durch das Leselineal wechseln: aus, aktuelle Zeile, Lineal
action-syllables-toggle = Silben anzeigen oder ausblenden: Wörter mit einem Mittelpunkt getrennt
action-difficult-words-toggle = Schwierige Wörter ein- oder ausschalten: unterstrichen, und bei Wortbewegungen mit hoher Ausführlichkeit genannt
action-text-larger = Den Text des Dokuments vergrößern
action-text-smaller = Den Text des Dokuments verkleinern
action-text-size-reset = Den Text des Dokuments auf die Standardgröße zurücksetzen
action-choose-font = Die Schriftart des Dokumenttexts wählen
action-command-palette = Jeden Befehl über seinen Namen ausführen
action-settings = Die Einstellungen öffnen: jede Option mit ihrer Hilfe, gefiltert beim Tippen; Links und Rechts ändern einen Wert
action-keyboard-help = Tastenkombinationen auflisten
action-help = Die Hilfe öffnen

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

## Restarting speech.

restart-silent-now = { -brand } ist jetzt stumm; starten Sie es neu, um wieder Sprache zu hören.
# $keys names the Restart Speech key or keys.
restart-silent-use-key = { -brand } ist jetzt stumm. Starten Sie die Sprachausgabe neu mit { $keys }.
restart-restarting = Sprachausgabe wird neu gestartet.
restart-not-here = Die Sprachausgabe kann hier nicht neu gestartet werden.
restart-already = Die Sprachausgabe wird bereits neu gestartet.
# $error is the system's reason, in its own words.
restart-failed = Die Sprachausgabe konnte nicht neu gestartet werden: { $error }.
restart-start-failed = Die Sprachausgabe konnte nicht neu gestartet werden: der Start ist fehlgeschlagen.
restart-no-engine = Keine Sprachausgabe ist verfügbar; { -brand } bleibt stumm.
restart-done-silent = Sprachausgabe neu gestartet, aber keine Sprachausgabe ist verfügbar; { -brand } bleibt stumm.
restart-done = Sprachausgabe neu gestartet.

## Tab completion of file paths in prompts.

# $folder is the folder's full path.
pathc-no-folder = Es gibt keinen Ordner { $folder }.
# $prefix is what was typed after the last separator.
pathc-no-match = Keine Datei oder kein Ordner beginnt mit { $prefix }.
# The one name that matched; $kind is folder or file.
pathc-one =
    { $kind ->
        [folder] { $name }, Ordner
       *[file] { $name }, Datei
    }
# $n names matched; $names are the first few, joined with commas; $more is
# yes when more matched than are read out.
pathc-many =
    { $more ->
        [yes] { $n } Treffer: { $names }, und weitere.
       *[no] { $n } Treffer: { $names }.
    }

## Exporting and importing settings.

# $n settings differ; $name is the file's name.
settingsio-import-question =
    { $n ->
        [one] { $n } geänderte Einstellung aus { $name } importieren? y oder n
       *[other] { $n } geänderte Einstellungen aus { $name } importieren? y oder n
    }
settingsio-no-persistence = Einstellungen werden in dieser Sitzung nicht gespeichert, daher können sie nicht exportiert oder importiert werden.
# $path is the file written.
settingsio-exported = Einstellungen nach { $path } exportiert.
settingsio-export-failed = Einstellungen konnten nicht exportiert werden: { $error }
# $path is the file; $error the system's reason.
settingsio-read-failed = { $path } konnte nicht gelesen werden: { $error }.
settingsio-nothing-to-import = Nichts zu importieren: Ihre Einstellungen stimmen bereits mit dieser Datei überein.
settingsio-cancelled-unchanged = Abgebrochen. Nichts wurde geändert.
settingsio-import-failed = Einstellungen konnten nicht importiert werden: { $error }
# $summary lists what changed (from the settings store, in English).
settingsio-imported = Einstellungen importiert. { $summary }
settingsio-backend-next-start = Die neue Sprachausgabe wird ab dem nächsten Start verwendet.
settingsio-backed-up = Die alten Einstellungen wurden gesichert.

## Opening a document: failures and opening in the background.

# $name is the file's name.
opening-is-folder = { $name } ist ein Ordner, kein Dokument. Geben Sie den Namen einer Datei darin an.
# Said after "Could not open NAME:", so it starts in lower case.
opening-no-file-in = es gibt keine Datei namens { $name } in { $folder }. Prüfen Sie den Namen.
opening-no-file-here = es gibt keine Datei namens { $name } hier. Prüfen Sie den Namen.
opening-no-permission = Sie haben keine Berechtigung, sie zu lesen.
opening-damaged-rtf = es ist keine lesbare RTF-Datei; sie ist möglicherweise beschädigt.
opening-damaged-odt = es ist keine lesbare OpenDocument-Textdatei; sie ist möglicherweise beschädigt.
opening-damaged-latex = es ist keine lesbare LaTeX-Datei; sie ist möglicherweise beschädigt oder zu groß.
opening-damaged-email = es ist keine lesbare E-Mail-Nachricht; sie ist möglicherweise beschädigt oder zu groß.
opening-damaged-mhtml = es ist kein lesbares Webarchiv; es ist möglicherweise beschädigt oder zu groß.
# $reason is one of the opening-no-* messages, or the loader's own words.
opening-failed = { $name } konnte nicht geöffnet werden: { $reason }
opening-started = { $name } wird geöffnet. Escape bricht ab.
opening-stopped = Öffnen von { $name } gestoppt.
opening-still = { $name } wird noch geöffnet, { $secs } Sekunden.
# $step is the loader's report, such as "recognizing text on page 3 (3 of 40)."
opening-still-step = { $name } wird noch geöffnet: { $step }
opening-stopped-unexpectedly = { $name } konnte nicht geöffnet werden: das Laden ist unerwartet gestoppt.

## A build without the publish feature. "tw convert" is a command typed
## at the terminal: keep it as it is.

lean-citations-not-in-build = Zitate sind in dieser Version von { -brand } nicht enthalten. Sie wurde ohne die Veröffentlichungsfunktion gebaut.
lean-publish-not-in-build = Export und Vorschau sind in dieser Version von { -brand } nicht enthalten. Sie wurde ohne die Veröffentlichungsfunktion gebaut; tw convert wandelt weiterhin um.

## The voice manager's list.

# $n voices are shown; $language is a language name or voices-all-languages;
# $engine an engine's name or voices-all-engines.
voices-shown =
    { $n ->
        [one] { $n } Stimme: { $language }, { $engine }.
       *[other] { $n } Stimmen: { $language }, { $engine }.
    }
voices-all-languages = alle Sprachen
voices-all-engines = alle Engines
# The filter rows at the top of the list.
voices-language-row = Sprache: { $language }
voices-engine-row = Engine: { $engine }
voices-fetch-row = Die Piper-Stimmenliste aus dem Internet abrufen
# Parts of a voice's row, joined with commas. $size is in megabytes, such
# as "63 MB".
voices-download-size = Download { $size }
voices-licence-public-domain = gemeinfrei
voices-licence-attribution = frei mit Namensnennung
voices-licence-share-alike = frei mit Namensnennung, Weitergabe unter gleichen Bedingungen
voices-licence-non-commercial = nicht kommerziell
voices-licence-unknown = Lizenz wird vor dem Download angezeigt
voices-favourite = Favorit
voices-current = aktuell

## Language names in the voice manager's language filter.

voices-language-ar = Arabisch
voices-language-ca = Katalanisch
voices-language-cs = Tschechisch
voices-language-cy = Walisisch
voices-language-da = Dänisch
voices-language-de = Deutsch
voices-language-el = Griechisch
voices-language-en = Englisch
voices-language-es = Spanisch
voices-language-fa = Persisch
voices-language-fi = Finnisch
voices-language-fr = Französisch
voices-language-hi = Hindi
voices-language-hu = Ungarisch
voices-language-is = Isländisch
voices-language-it = Italienisch
voices-language-ja = Japanisch
voices-language-ka = Georgisch
voices-language-kk = Kasachisch
voices-language-ko = Koreanisch
voices-language-lb = Luxemburgisch
voices-language-lv = Lettisch
voices-language-nl = Niederländisch
voices-language-no = Norwegisch
voices-language-pl = Polnisch
voices-language-pt = Portugiesisch
voices-language-ro = Rumänisch
voices-language-ru = Russisch
voices-language-sk = Slowakisch
voices-language-sl = Slowenisch
voices-language-sr = Serbisch
voices-language-sv = Schwedisch
voices-language-sw = Suaheli
voices-language-tr = Türkisch
voices-language-uk = Ukrainisch
voices-language-vi = Vietnamesisch
voices-language-zh = Chinesisch

## Voices: the voice manager, rate, pitch, and volume.

# Spoken by a newly chosen voice as its sample.
voice-sample = Franz jagt im komplett verwahrlosten Taxi quer durch Bayern.
voice-list-title = Eine Stimme wählen
voice-still-loading = Die Stimmen werden noch geladen. Die Liste öffnet sich, sobald sie bereit sind.
voice-list-failed = Die Stimmen konnten nicht aufgelistet werden: { $error }.
# $shown is voices-shown ("12 voices: English, all engines."). Enter,
# Space, Delete and Escape are the list's own keys.
voice-manager-intro = Stimmenverwaltung. { $shown } Eingabetaste verwendet eine Stimme und spricht ein Beispiel, oder lädt eine herunter; Leertaste markiert einen Favoriten; Entf entfernt eine heruntergeladene Stimme; Escape schließt.
voice-more-ready = { $n } weitere Stimmen von anderen Engines sind bereit. Drücken Sie Escape und öffnen Sie die Stimmenverwaltung erneut, um sie zu sehen.
# $keys names the Choose Voice key.
voice-ready = Die Stimmen sind bereit. { $keys } listet sie auf.
voice-fetch-catalog-question = Die Liste der Piper-Stimmen, etwa 250 Kilobyte, von Hugging Face herunterladen? y oder n
voice-fetch-catalog-question-short = Die Liste der Piper-Stimmen herunterladen? y oder n
# $engine is the engine's name, such as "Piper neural voices".
voice-switching-engine = Stimme { $voice }, auf { $engine }. Engine wird gewechselt.
voice-download-in-progress = Ein Stimmen-Download läuft bereits.
voice-no-data-folder = Es gibt keinen Datenordner, um Piper-Stimmen aufzubewahren.
voice-not-in-list = Diese Stimme ist nicht mehr in der Piper-Stimmenliste.
voice-download-start-failed = Der Download konnte nicht gestartet werden.
voice-reading-licence = Die Lizenz von { $voice } wird gelesen.
voice-remove-question = Die Stimme { $voice } entfernen? y oder n
voice-only-piper-removable = Nur heruntergeladene Piper-Stimmen können entfernt werden.
# $plan describes the download: the voice, its size and licence.
voice-download-question = { $plan } y oder n
voice-in-use = { $voice } ist die verwendete Stimme. Wählen Sie zuerst eine andere Stimme.
voice-removed = { $voice } entfernt.
voice-remove-failed = { $voice } konnte nicht entfernt werden: { $error }.
voice-downloading-catalog = Die Piper-Stimmenliste wird heruntergeladen.
voice-downloading = { $voice } wird heruntergeladen.
voice-downloading-percent = { $voice } wird heruntergeladen, { $pct } Prozent.
voice-details-failed = Die Details der Stimme konnten nicht gelesen werden: { $error }.
voice-download-stopped = Der Download wurde gestoppt.
voice-catalog-fetched = Die Piper-Stimmenliste hat { $voices } Stimmen in { $languages } Sprachen. Stimme wählen listet sie auf.
voice-catalog-failed = Die Stimmenliste konnte nicht heruntergeladen werden: { $error }.
# $licence describes the voice's licence, in a sentence of its own.
voice-installed = { $voice } ist installiert. { $licence } Stimme wählen listet sie auf.
voice-download-failed = { $voice } konnte nicht heruntergeladen werden: { $error }.
voice-only-voice-favourite = Nur eine Stimme kann ein Favorit sein.
voice-favourite-added = { $voice } zu den Favoriten hinzugefügt.
voice-favourite-removed = { $voice } aus den Favoriten entfernt.
voice-chosen = Stimme { $voice }.
voice-chosen-rate = Stimme { $voice }, { $wpm } Wörter pro Minute.
voice-fastest-rate = Schnellste Geschwindigkeit.
voice-slowest-rate = Langsamste Geschwindigkeit.
voice-rate = { $wpm } Wörter pro Minute.
voice-highest-pitch = Höchste Tonhöhe.
voice-lowest-pitch = Tiefste Tonhöhe.
voice-pitch-normal = Normale Tonhöhe.
# $n is a number of semitones.
voice-pitch-plus = Tonhöhe plus { $n }.
voice-pitch-minus = Tonhöhe minus { $n }.
voice-full-volume = Volle Lautstärke.
voice-volume-off = Lautstärke aus.
voice-volume = Lautstärke { $pct } Prozent.
voice-no-speed-presets = Keine Geschwindigkeitsvorgaben.
# $name is the preset's name from the settings, such as "Study".
voice-speed-preset = { $name }, Tempo { $wpm }.
voice-line-numbers-on = Zeilennummern an.
voice-line-numbers-off = Zeilennummern aus.

## Export and preview from the reader. F5 is the browser's reload key,
## not textweaver's.

publish-no-document = Kein Dokument ist geöffnet.
# Said after "Could not export:", so it starts in lower case. $path is a
# folder or a file; $error the system's reason.
publish-cannot-write-to = kann nicht nach { $path } schreiben: { $error }
publish-cannot-write = kann { $path } nicht schreiben: { $error }
publish-start-failed = Der Export konnte nicht gestartet werden: { $error }
publish-export-error = Export nicht möglich: { $error }
# $format is the format's name, such as PDF, HTML, or Word.
publish-exporting = Export nach { $format }.
publish-writing-preview = Die Vorschau wird geschrieben.
publish-preview-error = Die Vorschau konnte nicht geschrieben werden: { $error }
publish-still-exporting =
    { $secs ->
        [one] Export nach { $format } läuft noch, { $secs } Sekunde.
       *[other] Export nach { $format } läuft noch, { $secs } Sekunden.
    }
publish-still-previewing =
    { $secs ->
        [one] Die Vorschau wird noch geschrieben, { $secs } Sekunde.
       *[other] Die Vorschau wird noch geschrieben, { $secs } Sekunden.
    }
# $again is yes when a preview is open already.
publish-auto-reload-on =
    { $again ->
        [yes] Automatisches Neuladen der Vorschau an: nach jedem Speichern lädt der Browser die Seite von selbst neu. Führen Sie Vorschau im Browser erneut aus, um sie zu nutzen.
       *[no] Automatisches Neuladen der Vorschau an: nach jedem Speichern lädt der Browser die Seite von selbst neu.
    }
publish-auto-reload-off = Automatisches Neuladen der Vorschau aus: drücken Sie F5 im Browser nach einem Speichern.
publish-live-on = Live-Vorschau an: die Vorschau lädt auch neu, wenn das Tippen pausiert.
# "toggle preview auto reload" is the command's name in the command palette.
publish-live-on-needs-reload = Live-Vorschau an. Sie funktioniert mit automatischem Neuladen, das aus ist; schalten Sie es ein mit Automatisches Neuladen der Vorschau umschalten.
publish-live-off = Live-Vorschau aus: die Vorschau lädt nur nach dem Speichern neu.
# $error is the converter's reason.
publish-export-failed = Export nach { $format } fehlgeschlagen: { $error }
publish-preview-failed = Vorschau fehlgeschlagen: { $error }
# The converter's warnings: how many, and the first one.
publish-warnings =
    { $n ->
        [one] 1 Warnung: { $first }
       *[other] { $n } Warnungen; die erste: { $first }
    }
# $file is the file's name, $folder its folder; $warned is empty or a
# space and publish-warnings.
publish-exported = Nach { $format } exportiert: { $file }. Öffnen? y oder n. In { $folder }.{ $warned }
publish-preview-written-served = Vorschau geschrieben. Sie wird im Browser geöffnet. Sie lädt nach jedem Speichern von selbst neu.{ $warned }
publish-preview-written = Vorschau geschrieben. Sie wird im Browser geöffnet. Speichern schreibt sie erneut; drücken Sie dann F5 im Browser.{ $warned }
publish-preview-updated = Vorschau aktualisiert.
publish-preview-updated-press-f5 = Vorschau aktualisiert. Drücken Sie F5 im Browser.
publish-server-failed = Der Neuladeserver der Vorschau konnte nicht gestartet werden ({ $error }); die Datei wird stattdessen geöffnet.
publish-render-failed = Der Text konnte nicht dargestellt werden: { $error }
publish-nothing-after-caret = Nichts zu lesen nach dem Cursor.
publish-listening = Der dargestellte Text wird angehört.

## The preview's reload server: shown in the browser.

preview-being-written = Die Vorschau wird gerade geschrieben. Laden Sie gleich neu.

## Notes and highlights.

notes-nothing-to-attach = Hier gibt es nichts, dem eine Notiz angehängt werden kann.
# $on is the start of the passage the note is on.
notes-added = Notiz hinzugefügt zu: { $on }
# $tags are the note's tags, joined with commas.
notes-added-with-tags = Notiz mit Tags { $tags } hinzugefügt zu: { $on }
# An item in the notes list. $anchor is the passage; $lost is yes when the
# passage was not found after the file changed.
notes-item =
    { $lost ->
        [yes] { $note }, Zeile { $line }. Zu: { $anchor } Nach der Dateiänderung nicht gefunden.
       *[no] { $note }, Zeile { $line }. Zu: { $anchor }
    }
notes-none = Keine Notizen.
notes-list-title = Notizen
notes-list-intro =
    { $n ->
        [one] Notizen, 1 Eintrag. Eingabetaste springt zu einer Notiz, Entf löscht sie, F2 bearbeitet sie.
       *[other] Notizen, { $n } Einträge. Eingabetaste springt zu einer Notiz, Entf löscht sie, F2 bearbeitet sie.
    }
# Said on jumping to a note: its text, then the passage it is on.
notes-note-content = { $note }. Zu: { $anchor }
# $i is the note's number, $n how many notes there are.
notes-note-label = Notiz { $i } von { $n }
notes-deleted = Notiz gelöscht: { $text }.
notes-none-here = Keine Notiz oder Hervorhebung hier.
notes-unchanged = Notiz unverändert.
notes-updated = Notiz aktualisiert.
notes-nothing-to-highlight = Hier gibt es nichts hervorzuheben.
notes-highlight-removed = Hervorhebung entfernt: { $text }
notes-highlighted-at = Hervorgehoben bei { $pct } Prozent: { $text }
notes-highlighted = Hervorgehoben: { $text }
# An item in the highlights list. $color is the highlight's color name;
# $lost is yes when the text was not found after the file changed.
notes-highlight-item =
    { $lost ->
        [yes] { $text }, Zeile { $line }, { $color }, nach der Dateiänderung nicht gefunden
       *[no] { $text }, Zeile { $line }, { $color }
    }
notes-no-highlights = Keine Hervorhebungen.
notes-highlights-title = Hervorhebungen
notes-highlights-intro =
    { $n ->
        [one] Hervorhebungen, 1 Eintrag. Eingabetaste springt zu einer, Entf entfernt sie.
       *[other] Hervorhebungen, { $n } Einträge. Eingabetaste springt zu einer, Entf entfernt sie.
    }
# The label said before a highlight's text on jumping to it.
notes-highlight-label = Hervorhebung
# Shown while reading reaches a note's passage.
notes-signal = Notiz: { $text }
# Said after moving onto a note's passage.
notes-has-note = Hat eine Notiz: { $text }

## Bookmarks: rename and delete.

notes-choose-bookmark-delete = Wählen Sie ein Lesezeichen und drücken Sie Entf.
notes-choose-bookmark-rename = Wählen Sie ein Lesezeichen und drücken Sie F2, um es umzubenennen.
notes-bookmark-deleted = Lesezeichen { $name } gelöscht.
notes-renaming-bookmark = Lesezeichen { $name } wird umbenannt.
notes-bookmark-unchanged = Lesezeichen unverändert.
# $name is the name asked for, $old the bookmark's name.
notes-bookmark-name-taken = Es gibt bereits ein Lesezeichen namens { $name }. Lesezeichen { $old } unverändert.
notes-bookmark-renamed = Lesezeichen { $old } in { $name } umbenannt.

## The study sheet.

notes-nothing-to-export = Keine Notizen oder Hervorhebungen zu exportieren.
# Keep the letters y and n: they are the keys that answer. $file is the
# sheet's file name, $folder the folder it was saved in.
notes-study-sheet-saved-notes =
    Lernblatt mit { $n ->
        [one] 1 Notiz
       *[other] { $n } Notizen
    } gespeichert als { $file }. Öffnen? y oder n. In { $folder }.
notes-study-sheet-saved-highlights =
    Lernblatt mit { $h ->
        [one] 1 Hervorhebung
       *[other] { $h } Hervorhebungen
    } gespeichert als { $file }. Öffnen? y oder n. In { $folder }.
notes-study-sheet-saved-both =
    Lernblatt mit { $n ->
        [one] 1 Notiz
       *[other] { $n } Notizen
    } und { $h ->
        [one] 1 Hervorhebung
       *[other] { $h } Hervorhebungen
    } gespeichert als { $file }. Öffnen? y oder n. In { $folder }.
notes-study-sheet-failed = Das Lernblatt konnte nicht geschrieben werden: { $error }
# The study sheet file's own text (Markdown; the # marks stay in the code).
notes-sheet-title = Lernblatt: { $title }
notes-sheet-exported = Exportiert aus { -brand } am { $date }.
notes-sheet-before-first-heading = Vor der ersten Überschrift
# After a note's text: its tags, joined with commas.
notes-sheet-tags = (Tags: { $tags })
# $color is the highlight's color name.
notes-sheet-highlighted = Hervorgehoben, { $color }.

## Find, bookmarks, and selection.

marks-cannot-search = Suche nicht möglich: { $error }.
# $pattern is the text searched for.
marks-no-matches = Keine Treffer für { $pattern }.
# The label of a match reached by Find, at high verbosity; $number is its place among $n matches.
marks-match-label = Treffer { $number } von { $n }
# Find wrapped past an end of the document; $dir is next (to the top) or previous (to the bottom); $message says the match.
marks-find-wrapped =
    { $dir ->
        [next] Am Anfang fortgesetzt. { $message }
       *[previous] Am Ende fortgesetzt. { $message }
    }
# $name is the bookmark's name, such as mark1.
marks-bookmark-already-here = Lesezeichen { $name } ist bereits hier.
marks-bookmark-set = Lesezeichen { $name } gesetzt bei { $pct } Prozent.
marks-no-bookmarks = Keine Lesezeichen.
marks-bookmarks-intro =
    { $n ->
        [one] Lesezeichen, { $n } Eintrag. Eingabetaste springt zu einem, Entf löscht es, F2 benennt es um.
       *[other] Lesezeichen, { $n } Einträge. Eingabetaste springt zu einem, Entf löscht es, F2 benennt es um.
    }
# One line of the bookmark list; $lost is yes when the bookmark's text was not found after the file changed; $text is the start of its line.
marks-bookmark-item =
    { $lost ->
        [yes] { $name } (nach der Dateiänderung nicht gefunden), Zeile { $line }, { $pct } Prozent: { $text }
       *[no] { $name }, Zeile { $line }, { $pct } Prozent: { $text }
    }
marks-bookmarks-title = Lesezeichen
# The label of a bookmark reached, at high verbosity.
marks-bookmark-label = Lesezeichen { $name }
marks-selection-cleared = Auswahl aufgehoben.
# $text is the selected text, shortened.
marks-selected = { $text } ausgewählt

## Authoring lists: the outline, the citation picker, spelling, grammar, and templates.

# An outline item: $text is the heading's text, $level its level.
lists-outline-item = { $text }, Ebene { $level }
# $n is the number of headings.
lists-outline-title =
    { $n ->
        [one] Gliederung, { $n } Überschrift
       *[other] Gliederung, { $n } Überschriften
    }
# $shown headings of $n match the filter $filter typed so far.
lists-outline-title-filtered = Gliederung, { $shown } von { $n } stimmen mit { $filter } überein
# $n is the number of references.
lists-citations-title =
    { $n ->
        [one] Zitat einfügen, { $n } Literaturangabe
       *[other] Zitat einfügen, { $n } Literaturangaben
    }
# $shown references of $n match the filter $filter typed so far.
lists-citations-title-filtered = Zitat einfügen, { $shown } von { $n } stimmen mit { $filter } überein
# $word is the misspelled word.
lists-spelling-title = Rechtschreibung von { $word }
lists-spelling-add = { $word } zu Ihrer Wortliste hinzufügen
lists-leave-as-is = So lassen, wie es ist
# $words are the words the grammar fixes are for.
lists-grammar-title = Grammatikkorrekturen für { $words }
# $n is the number of templates.
lists-templates-title = Neues Dokument aus einer Vorlage, { $n } Vorlagen
lists-no-filter = Diese Liste filtert nicht.
# The filter was emptied: $n items are shown.
lists-filter-cleared-headings =
    { $n ->
        [one] Filter gelöscht, { $n } Überschrift.
       *[other] Filter gelöscht, { $n } Überschriften.
    }
lists-filter-cleared-references =
    { $n ->
        [one] Filter gelöscht, { $n } Literaturangabe.
       *[other] Filter gelöscht, { $n } Literaturangaben.
    }
lists-filter-cleared-items =
    { $n ->
        [one] Filter gelöscht, { $n } Eintrag.
       *[other] Filter gelöscht, { $n } Einträge.
    }
# Nothing matches the filter $query.
lists-filter-none-headings = Keine Überschrift stimmt mit { $query } überein. Rücktaste entfernt Buchstaben.
lists-filter-none-references = Keine Literaturangabe stimmt mit { $query } überein. Rücktaste entfernt Buchstaben.
lists-filter-none-items = Kein Eintrag stimmt mit { $query } überein. Rücktaste entfernt Buchstaben.
# $n items match the filter.
lists-filter-matched-headings =
    { $n ->
        [one] { $n } Überschrift stimmt überein.
       *[other] { $n } Überschriften stimmen überein.
    }
lists-filter-matched-references =
    { $n ->
        [one] { $n } Literaturangabe stimmt überein.
       *[other] { $n } Literaturangaben stimmen überein.
    }
lists-filter-matched-items =
    { $n ->
        [one] { $n } Eintrag stimmt überein.
       *[other] { $n } Einträge stimmen überein.
    }
lists-no-headings = Dieses Dokument hat keine Überschriften.
# $n is the number of headings; the keys are the outline list's own.
lists-outline-intro =
    { $n ->
        [one] Gliederung, { $n } Überschrift. Tippen filtert, Eingabetaste springt zu einer Überschrift, Escape schließt.
       *[other] Gliederung, { $n } Überschriften. Tippen filtert, Eingabetaste springt zu einer Überschrift, Escape schließt.
    }
# $heading is the text of the heading the cursor is under.
lists-outline-here = Sie sind unter { $heading }.

## Lists and prompts shared by every frontend.

# The focused list item: $item is its text, $k its place, $n the number of items.
listmodel-item-position = { $k } von { $n }, { $item }
# $letter is the letter or digit typed.
listmodel-no-item-starts = Kein Eintrag beginnt mit { $letter }.
listmodel-top-of-list = Anfang der Liste.
listmodel-end-of-list = Ende der Liste.
listmodel-no-matching-commands = Keine passenden Befehle.
listmodel-no-earlier-entries = Keine früheren Einträge.
# Tab in the command palette: $n commands match (always more than one); $names lists the first few, joined by commas.
listmodel-command-matches = { $n } Treffer: { $names }.

## The library.

# $n is how many documents the scan has found.
library-still-scanning = Die Bibliothek wird noch durchsucht: { $n } bisher gefunden.
library-scan-failed = Die Bibliothek konnte nicht durchsucht werden: { $error }.
library-scanning = Die Bibliothek wird durchsucht.
library-scan-progress = Die Bibliothek wird durchsucht: { $n } bisher gefunden.
library-scan-stopped = Die Bibliotheksdurchsuchung wurde durch einen internen Fehler gestoppt.
# $command is the command line that adds a folder; $key names the Open command's key.
library-empty = Die Bibliothek ist leer. Fügen Sie einen Ordner hinzu mit { $command }, oder öffnen Sie eine Datei mit { $key }.
library-intro =
    { $n ->
        [one] Bibliothek, { $n } Dokument. Tippen filtert, Eingabetaste öffnet eines.
       *[other] Bibliothek, { $n } Dokumente. Tippen filtert, Eingabetaste öffnet eines.
    }
library-title = Bibliothek

## Following links and footnotes.

links-none-here = Kein Link oder keine Fußnote am Cursor.
# $text is the link's text.
links-no-address = Der Link { $text } hat keine Adresse.
# $kind is mail or web; $target is the link's address.
links-open-question =
    { $kind ->
        [mail] Mail-Link öffnen? y oder n. { $target }
       *[web] Web-Link öffnen? y oder n. { $target }
    }
# The label of a heading reached by a link, at high verbosity.
links-heading-label = Überschrift
# $anchor is the heading name the link gives.
links-no-heading = Keine Überschrift namens { $anchor } in diesem Dokument.
# $file is the file the link names.
links-file-not-found = Der Link führt zu { $file }, was nicht gefunden wurde.
# $file is the file's name; $key names the History Back command's keys.
links-followed = Dem Link zu { $file } gefolgt. Zurück: { $key }.
links-back-in = Zurück in { $file }.
# $label is the footnote's label, such as 1.
links-back-to-footnote-reference = Zurück zum Fußnotenverweis { $label }, Zeile { $line }.
# $text is the start of the note.
links-footnote = Fußnote { $label }: { $text }
links-footnote-unreferenced = Kein Verweis auf Fußnote { $label } im Text.
links-footnote-no-note = Fußnote { $label } hat keine Notiz.

## Citations: inserting, looking up, importing, checking, and the bibliography.

citations-on = Zitate an.
citations-off = Zitate aus.
# $key names the Add Reference command's keys.
citations-library-empty = Ihre Literaturbibliothek ist leer. Fügen Sie eine Literaturangabe per DOI oder ISBN hinzu mit { $key }, oder führen Sie Literaturangaben importieren aus der Befehlspalette aus.
# $n is how many references the picker lists.
citations-picker-intro =
    { $n ->
        [one] Zitat einfügen, { $n } Literaturangabe. Tippen filtert, Eingabetaste wählt, Escape bricht ab.
       *[other] Zitat einfügen, { $n } Literaturangaben. Tippen filtert, Eingabetaste wählt, Escape bricht ab.
    }
# $text is what was typed at the locator prompt.
citations-locator-unreadable = Der Fundort { $text } konnte nicht gelesen werden. Geben Sie eine Seite wie 12 ein, Seiten wie 3-5, oder Kapitel 2; Eingabetaste allein für keinen.
citations-insert-failed = Das Zitat konnte nicht eingefügt werden: { $error }
# $what is the identifier being looked up, as the citation library describes it.
citations-looking-up = { $what } wird nachgeschlagen.
citations-lookup-not-started = Das Nachschlagen konnte nicht gestartet werden: { $error }
# $input is the DOI or ISBN as typed.
citations-lookup-failed = { $input } konnte nicht nachgeschlagen werden: { $error }
citations-no-library-to-add-to = Es gibt keine Bibliothek zum Hinzufügen: { -brand } hält in dieser Sitzung keine Dateien.
citations-library-save-failed = Die Bibliothek konnte nicht gespeichert werden: { $error }
# $n is how many citations the document has.
citations-found-no-library =
    { $n ->
        [one] { $n } Zitat gefunden. { -brand } hält in dieser Sitzung keine Bibliothek.
       *[other] { $n } Zitate gefunden. { -brand } hält in dieser Sitzung keine Bibliothek.
    }
citations-check-failed = Die Zitate konnten nicht geprüft werden: { $error }
citations-no-library-to-import-into = Es gibt keine Bibliothek zum Importieren: { -brand } hält in dieser Sitzung keine Dateien.
# $file is the file's path.
citations-import-failed = { $file } konnte nicht importiert werden: { $error }
# $style is the style's name from the front matter, such as apa.
citations-style-unusable = Der Zitierstil { $style } kann nicht verwendet werden: { $error }
citations-format-failed = Die Zitate konnten nicht formatiert werden: { $error }
# $key names the Insert Citation command's keys.
citations-none-yet = Das Dokument hat noch keine Zitate. Fügen Sie eines ein mit { $key }.
citations-nothing-to-list = Keines der zitierten Werke ist in Ihrer Bibliothek, daher gibt es nichts aufzulisten.
# $n is how many entries went in; $style is the style's name, such as apa.
citations-bibliography-inserted =
    { $n ->
        [one] Literaturverzeichnis eingefügt, { $n } Eintrag, Stil { $style }.
       *[other] Literaturverzeichnis eingefügt, { $n } Einträge, Stil { $style }.
    }
# Follows citations-bibliography-inserted; $keys are citation keys joined with commas.
citations-not-in-library = Nicht in der Bibliothek: { $keys }.
citations-bibliography-insert-failed = Das Literaturverzeichnis konnte nicht eingefügt werden: { $error }

## Speech Cursor mode.

speechcursor-off = Lesecursor aus.
# $line is the line number.
speechcursor-on = Lesecursor an, Zeile { $line }. Auf und Ab lesen Zeilen, Eingabetaste liest weiter, Tabulator oder Escape verlässt.
# $text is the line read, as the status line shows it.
speechcursor-on-with-text = Lesecursor an, Zeile { $line }: { $text }. Auf und Ab lesen Zeilen, Eingabetaste liest weiter, Tabulator oder Escape verlässt.

## Scrolling the view without moving the cursor.

view-bottom-of-document = Ende des Dokuments.
# $line is the line now at the top of the view.
view-line-at-top = Zeile { $line } oben.

## Background work, and opening files and addresses.

# $what names the export, as its own message says it.
tasks-stopped = { $what } unerwartet gestoppt.
# $input is the DOI, ISBN, or other identifier being looked up.
tasks-lookup-stopped = Nachschlagen von { $input } unerwartet gestoppt.
# The reason in tasks-could-not-open when a session keeps no files.
tasks-launch-off = andere Programme öffnen ist aus in einer Sitzung, die keine Dateien hält
tasks-opening = Wird geöffnet.
# $target is a file or a web address; $error says why.
tasks-could-not-open = { $target } konnte nicht geöffnet werden: { $error }
tasks-not-opened = Nicht geöffnet.
# Keep the letters y and n: they are the keys that answer.
tasks-open-it-question = Öffnen? y oder n.

## Math exploration.

mathx-no-math = Keine Mathematik hier. Bewegen Sie sich zu einer Formel und versuchen Sie es erneut.
# $math is the whole expression as spoken; $parts is yes when it has parts to go into. The keys are exploration's own.
mathx-exploring =
    { $parts ->
        [yes] Mathematik wird erkundet: { $math }. Ab geht hinein, Pfeile bewegen, Escape verlässt.
       *[no] Mathematik wird erkundet: { $math }. Escape verlässt.
    }
mathx-left = Mathematik verlassen.
mathx-last-term = Letzter Term.
mathx-first-term = Erster Term.
mathx-no-parts = Keine Teile darin.
mathx-whole-expression = Ganzer Ausdruck.
mathx-nothing-here = Nichts hier.
# $speech is what was said for the step; $code is the math braille code's name (Nemeth or UEB); $braille is the part's braille in Unicode braille cells, for the Braille display.
mathx-step-braille = { $speech } { $code }: { $braille }

## Reading aids: RSVP, bionic reading, syllables, difficult words, the ruler, and the reading level.

aids-rsvp-off = RSVP aus.
aids-rsvp-leave-edit = Verlassen Sie den Bearbeitungsmodus, um RSVP zu nutzen.
# $status is RSVP's status line (word and sentence counts, rate, state).
aids-rsvp-on = RSVP an. { $status }
aids-rsvp-no-words = Keine Wörter zum Anzeigen.
aids-rsvp-fastest = Schnellste RSVP-Geschwindigkeit.
aids-rsvp-slowest = Langsamste RSVP-Geschwindigkeit.
# $wpm is the new rate in words per minute.
aids-rsvp-rate = RSVP { $wpm } Wörter pro Minute.
# Where the RSVP word is shown; $position is one of nine fixed keys.
aids-rsvp-position =
    { $position ->
        [top-left] RSVP oben links.
        [top-center] RSVP oben mittig.
        [top-right] RSVP oben rechts.
        [center-left] RSVP mittig links.
        [center] RSVP in der Mitte.
        [center-right] RSVP mittig rechts.
        [bottom-left] RSVP unten links.
        [bottom-right] RSVP unten rechts.
       *[bottom-center] RSVP unten mittig.
    }
aids-rsvp-playing = RSVP spielt ab.
aids-rsvp-paused = RSVP pausiert.
aids-rsvp-end-of-text = Ende des Texts.
aids-rsvp-start-of-text = Anfang des Texts.
aids-bionic-on = Bionisches Lesen an.
aids-bionic-off = Bionisches Lesen aus.
aids-syllables-shown = Silben angezeigt.
aids-syllables-hidden = Silben ausgeblendet.
aids-difficult-on = Schwierige Wörter unterstrichen.
aids-difficult-no-list = Schwierige Wörter an, aber die Wortliste fehlt in dieser Version.
aids-difficult-off = Schwierige Wörter nicht markiert.
# Added after a word at high verbosity, following a comma.
aids-difficult-word = schwieriges Wort
aids-ruler-off = Leselineal aus.
aids-ruler-current-line = Aktuelle Zeile markiert.
aids-ruler-on = Leselineal an.
# $summary is aids-level-summary; $scope says what was measured.
aids-reading-level =
    { $scope ->
        [selection] Auswahl: { $summary }
       *[document] Dokument: { $summary }
    }
aids-reading-level-too-short = Nicht genug Text, um das Leseniveau zu messen.
# $grade is the Flesch-Kincaid grade with one decimal, $band an aids-band-* message, $ease the reading ease (0 to 100), $words aids-level-words, $sentences aids-level-sentences.
aids-level-summary = Klasse { $grade }, { $band }. Leseleichtigkeit { $ease } von 100. { $words } in { $sentences }.
# $n is the count, $count the same number written with thousands separators.
aids-level-words =
    { $n ->
        [one] { $count } Wort
       *[other] { $count } Wörter
    }
aids-level-sentences =
    { $n ->
        [one] { $count } Satz
       *[other] { $count } Sätze
    }
aids-band-elementary = Grundschule
aids-band-middle-school = Mittelstufe
aids-band-high-school = Oberstufe
aids-band-college = Studienbeginn
aids-band-graduate = Hochschulabschluss

## Themes.

# $name is the theme name in the settings; $used the display name of the theme used instead.
themes-unknown = Es gibt kein Design namens { $name }; { $used } wird verwendet.
# $theme is the new theme's display name.
themes-next = Design { $theme }.

## Accessibility modes and the first-run question.

# Said when the accessibility mode changes; $mode is the new mode's id.
access-mode-changed =
    { $mode ->
        [self-voicing] Selbstsprechend-Modus. textweaver spricht alles.
        [screen-reader] Screenreader-Modus. textweaver ist stumm; Ihr Screenreader liest die Statuszeile.
       *[hybrid] Hybrid-Modus. textweaver liest Dokumente vor; Ihr Screenreader spricht Meldungen und Tippen.
    }
# Yes to the first-run question; $key names the keys that change the mode.
access-hybrid-chosen = Hybrid-Modus. textweaver liest Dokumente vor; Ihr Screenreader spricht Meldungen und Tippen. { $key } wechselt den Modus.
# No to the first-run question; $key names the keys that change the mode.
access-hybrid-declined = Bleibt im Selbstsprechend-Modus. { $key } wechselt den Modus.
# $reader is the screen reader found (NVDA, JAWS), or access-a-screen-reader.
access-hybrid-question = { $reader } läuft. Hybrid-Modus verwenden, bei dem textweaver Dokumente vorliest und Ihr Screenreader Meldungen und Tippen spricht? y oder n
access-a-screen-reader = Ein Screenreader

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

## The settings screen. $label is a setting-* label, $value its value as
## described below.

settings-not-set = nicht gesetzt
settings-none = keine
settings-empty = leer
# A number and its unit (a settings-unit-* message): "300 words per minute".
settings-number-unit = { $n } { $unit }
settings-entries =
    { $n ->
        [0] keine
        [one] 1 Eintrag
       *[other] { $n } Einträge
    }
settings-type-on-or-off = Geben Sie an oder aus ein.
settings-type-a-number = Geben Sie eine Zahl von { $min } bis { $max } ein.
settings-outside = { $n } liegt außerhalb von { $min } bis { $max }.
# $names are the choices, joined with commas.
settings-choose-one-of = Wählen Sie eines von: { $names }.
settings-edit-table = Bearbeiten Sie { $label } in settings.toml; sie enthält Namen und Werte.
# $path is a key such as speech.rate, not translated.
settings-no-such-setting = Es gibt keine Einstellung { $path }.
settings-cannot-be = { $label } kann das nicht sein: { $error }.
settings-changed = { $label }, { $value }.
settings-clamped = Außerhalb des Bereichs, daher wird der nächstliegende Wert verwendet.
settings-restart-speech = Starten Sie die Sprachausgabe neu, um es zu verwenden.
settings-next-start = Ab dem nächsten Start verwendet.
settings-intro = Einstellungen, { $n } Einstellungen. Tippen filtert. Links und Rechts ändern einen Wert, Eingabetaste ändert oder gibt einen ein, Entf setzt den Standard zurück, Escape schließt.
settings-item = { $label }: { $value }
settings-title = Einstellungen
settings-title-matching = Einstellungen, die mit { $filter } übereinstimmen
settings-closed = Einstellungen geschlossen.
settings-filter-cleared =
    { $n ->
        [one] Filter gelöscht, 1 Einstellung.
       *[other] Filter gelöscht, { $n } Einstellungen.
    }
settings-filter-none = Keine Einstellung stimmt mit { $query } überein. Rücktaste entfernt Buchstaben.
settings-filter-match =
    { $n ->
        [one] 1 Einstellung stimmt überein.
       *[other] { $n } Einstellungen stimmen überein.
    }
settings-largest = Größter Wert, { $value }.
settings-smallest = Kleinster Wert, { $value }.
settings-press-enter = { $label }: Eingabetaste drücken, um einen neuen Wert einzugeben.
settings-table-item = { $label }: { $value }. Bearbeiten Sie sie in settings.toml.
# $help is the setting's help (setting-*-help), which may be empty.
settings-editing = { $label }, jetzt { $value }. { $help }

## Settings: labels, help, and choices, as the settings screen shows and
## says them. Ids follow the key in settings.toml (speech.rate is
## setting-speech-rate).

setting-speech-backend = Sprachausgabe
setting-speech-backend-help = Die Sprachausgabe: automatisch wählt die beste verfügbare. Eine Änderung startet die Sprachausgabe neu.
choice-speech-backend-auto = automatisch
choice-speech-backend-eci = Eloquence
choice-speech-backend-sapi = SAPI-5-Stimmen
choice-speech-backend-espeak = eSpeak NG
choice-speech-backend-speechd = Speech Dispatcher
choice-speech-backend-nsspeech = Apple NSSpeech
choice-speech-backend-avspeech = Apple AVSpeech
choice-speech-backend-dectalk = DECtalk
choice-speech-backend-omnivox = Omnivox
choice-speech-backend-null = stumm
setting-speech-rate = Geschwindigkeit
setting-speech-rate-help = Wie schnell textweaver spricht.
setting-speech-volume = Lautstärke
setting-speech-volume-help = Wie laut textweaver spricht.
setting-speech-pitch = Tonhöhe
setting-speech-pitch-help = Höher oder tiefer als die eigene Tonhöhe der Stimme.
setting-speech-voice = Stimme
setting-speech-voice-help = Die Kennung der Stimme; nicht gesetzt wählt automatisch eine. Stimme wählen listet sie auf.
setting-speech-prefer-voice = Bevorzugte Stimme
setting-speech-prefer-voice-help = Wenn keine Stimme gesetzt ist, die erste Stimme, deren Name dies enthält, zum Beispiel eloquence.
setting-speech-favorite-voices = Bevorzugte Stimmen
setting-speech-favorite-voices-help = Stimmen, die in Stimme wählen zuerst aufgelistet werden, nach Kennung.
setting-speech-punctuation = Interpunktion
setting-speech-punctuation-help = Wie viel Interpunktion gesprochen wird.
choice-speech-punctuation-none = keine
choice-speech-punctuation-some = etwas
choice-speech-punctuation-all = alle
setting-speech-split-caps = Großbuchstaben trennen
setting-speech-split-caps-help = Mit Großbuchstaben zusammengesetzte Wörter, wie TextWeaver, als getrennte Wörter sagen.
setting-speech-caps = Großbuchstaben
setting-speech-caps-help = Wie ein Großbuchstabe markiert wird, wenn Zeichen gesprochen und getippt werden.
choice-speech-caps-none = nicht markiert
choice-speech-caps-tone = ein Ton
choice-speech-caps-pitch = eine höhere Tonhöhe
choice-speech-caps-say-cap = "groß" sagen
setting-speech-auto-play = Beim Öffnen lesen
setting-speech-auto-play-help = Mit Lesen beginnen, wenn ein Dokument geöffnet wird.
setting-speech-skip-code = Codeblöcke überspringen
setting-speech-skip-code-help = Codeblöcke nicht sprechen.
setting-speech-speed-presets = Geschwindigkeitsvorgaben
setting-speech-speed-presets-help = Benannte Geschwindigkeiten, durch die F8 wechselt.
setting-speech-voices-by-language = Stimmen nach Sprache
setting-speech-voices-by-language-help = Die Stimme für jede Oberflächensprache, nach Sprachkürzel, zum Beispiel es = die Kennung der Stimme. Eine nicht aufgeführte Sprache verwendet die erste Stimme der Engine dafür.
setting-speech-latency-offset-ms = Verzögerung der Hervorhebung
setting-speech-latency-offset-ms-help = Wie lange nach der Meldung eines Worts durch eine Engine sich die Hervorhebung bewegt, für Engines, die nach ihrer Audiozeit getaktet sind.
setting-speech-verbosity = Ausführlichkeit
setting-speech-verbosity-help = Wie viel textweaver darüber sagt, was es tut.
choice-speech-verbosity-low = niedrig
choice-speech-verbosity-normal = normal
choice-speech-verbosity-high = hoch
setting-speech-eci-dictionaries = Eloquence-Wörterbücher
setting-speech-eci-dictionaries-help = Die Community-Ausspracheverzeichnisse für Eloquence: an, aus, oder ein eigener Ordner.
choice-speech-eci-dictionaries-true = an
choice-speech-eci-dictionaries-false = aus
setting-speech-eci-library = Eloquence-Bibliothek
setting-speech-eci-library-help = Die zu ladende ECI-Bibliothek; nicht gesetzt sucht an den üblichen Orten.
setting-speech-eci-code-factory = Eloquence von Code Factory suchen
setting-speech-eci-code-factory-help = Auch nach Eloquence von Code Factory für Windows suchen. Deren Lizenz deckt möglicherweise keine anderen Programme ab.
setting-speech-sapi-onecore = OneCore-Stimmen
setting-speech-sapi-onecore-help = Auch die Windows-OneCore-Stimmen über SAPI 5 auflisten.
setting-speech-apple-backend = Apple-Sprachausgabe
setting-speech-apple-backend-help = Welche der Apple-Sprachausgaben unter macOS verwendet wird.
choice-speech-apple-backend-auto = automatisch
choice-speech-apple-backend-nsspeech = NSSpeechSynthesizer
choice-speech-apple-backend-avspeech = AVSpeechSynthesizer
setting-highlight-enabled = Gesprochenen Text hervorheben
setting-highlight-enabled-help = Das gelesene Wort oder den gelesenen Satz hervorheben.
setting-highlight-granularity = Hervorheben
setting-highlight-granularity-help = Was die Lese-Hervorhebung abdeckt.
choice-highlight-granularity-word = das Wort
choice-highlight-granularity-sentence = den Satz
choice-highlight-granularity-both = das Wort und den Satz
setting-highlight-lead-words = Hervorhebungsvorlauf
setting-highlight-lead-words-help = Die Hervorhebung so viele Wörter vor dem gehörten Wort zeichnen (1 ist das gehörte Wort).
setting-highlight-speed = Hervorhebungsgeschwindigkeit
setting-highlight-speed-help = Geschwindigkeit der zeitgesteuerten Hervorhebung für Engines, die keine Wörter melden.
setting-highlight-color = Wort-Hervorhebungsfarbe
setting-highlight-color-help = Ein Farbname oder #rrggbb über der Wort-Hervorhebung des Designs; Design behält die des Designs.
setting-highlight-sentence-color = Satz-Hervorhebungsfarbe
setting-highlight-sentence-color-help = Ein Farbname oder #rrggbb über der Satz-Hervorhebung des Designs; nicht gesetzt behält die des Designs.
setting-normalization-math = Mathematik sprechen
setting-normalization-math-help = Mathematische Notation in Worten sprechen.
setting-normalization-math-verbosity = Mathematik-Ausführlichkeit
setting-normalization-math-verbosity-help = Wie ausführlich gesprochene Mathematik ist: niedrig sagt a durch b, normal und hoch sagen mehr.
choice-normalization-math-verbosity-low = niedrig
choice-normalization-math-verbosity-normal = normal
choice-normalization-math-verbosity-high = hoch
setting-normalization-asciimath-delimiter = ASCIIMath-Trennzeichen
setting-normalization-asciimath-delimiter-help = Das Zeichen um ASCIIMath, meist ein Gravis; nicht gesetzt liest kein ASCIIMath.
setting-normalization-abbreviations = Abkürzungen ausschreiben
setting-normalization-abbreviations-help = Abkürzungen ausgeschrieben sagen, wie Doktor für Dr.
setting-normalization-abbrev-expansions = Ihre Abkürzungen
setting-normalization-abbrev-expansions-help = Eigene Abkürzungen und wofür sie stehen.
setting-normalization-numbers = Zahlen in Worten
setting-normalization-numbers-help = Zahlen, Daten, Uhrzeiten und Geldbeträge in Worten sagen.
setting-normalization-use-pronunciations = Aussprachen verwenden
setting-normalization-use-pronunciations-help = Ihre Ausspracheliste anwenden.
setting-normalization-pronunciations = Aussprachen
setting-normalization-pronunciations-help = Wörter und wie sie gesagt werden.
setting-normalization-table-mode = Tabellen
setting-normalization-table-mode-help = Wie Tabellen gelesen werden.
choice-normalization-table-mode-structured = mit Zeilen und Spalten
choice-normalization-table-mode-flat = als Text
choice-normalization-table-mode-skip = übersprungen
setting-normalization-footnote-mode = Fußnoten
setting-normalization-footnote-mode-help = Wo Fußnoten gelesen werden.
choice-normalization-footnote-mode-inline = wo sie markiert sind
choice-normalization-footnote-mode-deferred = am Ende
choice-normalization-footnote-mode-skip = übersprungen
setting-normalization-community-lexicon-enabled = Community-Lexikon
setting-normalization-community-lexicon-enabled-help = Die Community-Ausspracheverzeichnisse für andere Engines als Eloquence anwenden.
setting-normalization-community-lexicon-dir = Community-Lexikon-Ordner
setting-normalization-community-lexicon-dir-help = Der Ordner mit den Wörterbuchdateien; nicht gesetzt schaut neben textweaver.
setting-normalization-community-lexicon-language = Community-Lexikon-Sprache
setting-normalization-community-lexicon-language-help = Die Sprache der Wörterbücher.
choice-normalization-community-lexicon-language-enu = US-Englisch
choice-normalization-community-lexicon-language-deu = Deutsch
setting-reading-auto-resume = Fortsetzen, wo aufgehört wurde
setting-reading-auto-resume-help = Zur gespeicherten Position zurückkehren, wenn ein Dokument geöffnet wird.
setting-reading-nav-history-size = Verlauf zurück
setting-reading-nav-history-size-help = Wie viele Stellen Zurück speichert.
setting-reading-wrap-navigation = Navigation umbrechen
setting-reading-wrap-navigation-help = Über das Ende des Dokuments hinaus geht es vom Anfang weiter.
setting-reading-cursor-follows-speech = Cursor folgt der Sprachausgabe
setting-reading-cursor-follows-speech-help = Der Cursor bewegt sich mit dem gelesenen Wort.
setting-reading-sync-conflict-policy = Synchronisierte Positionen
setting-reading-sync-conflict-policy-help = Welche Position gewinnt, wenn ein anderes Gerät weiter oder später gelesen hat.
choice-reading-sync-conflict-policy-newest = die neueste
choice-reading-sync-conflict-policy-highest-progress = die weiteste
choice-reading-sync-conflict-policy-manual = fragen
setting-reading-citations = Zitate
setting-reading-citations-help = Zitate beim fortlaufenden Lesen: übersprungen, oder in Worten gesagt.
choice-reading-citations-off = übersprungen
choice-reading-citations-words = in Worten
setting-reading-ocr = Gescannte Seiten erkennen
setting-reading-ocr-help = Den Text gescannter PDFs und Bilder durch Erkennung lesen (OCR).
setting-reading-ocr-lang = Sprache des gescannten Texts
setting-reading-ocr-lang-help = Die Sprache des gescannten Texts, als Tesseract-Kürzel wie fra oder deu+eng; leer bedeutet die Sprache des Dokuments, sonst Englisch.
choice-reading-ocr-lang- = die des Dokuments
choice-reading-ocr-lang-eng = Englisch
choice-reading-ocr-lang-fra = Französisch
choice-reading-ocr-lang-deu = Deutsch
choice-reading-ocr-lang-spa = Spanisch
setting-reading-ocr-engine = OCR-Engine
setting-reading-ocr-engine-help = Welche Engine gescannte Seiten erkennt: ocrs für Englisch und Tesseract für andere Sprachen, oder eine davon immer.
choice-reading-ocr-engine-auto = automatisch
choice-reading-ocr-engine-ocrs = ocrs
choice-reading-ocr-engine-tesseract = Tesseract
choice-reading-ocr-engine-paddle = PaddleOCR (experimentell)
setting-reading-math-engine = Mathematik-Sprachausgabe
setting-reading-math-engine-help = Welche Engine Mathematik vorliest: die eigene von textweaver, oder MathCAT in ClearSpeak oder SimpleSpeak, in der Sprache des Dokuments. MathCAT braucht eine Version, die es enthält; sonst wird die eigene von textweaver verwendet.
choice-reading-math-engine-builtin = textweaver
choice-reading-math-engine-mathcat = MathCAT ClearSpeak
choice-reading-math-engine-mathcat-simplespeak = MathCAT SimpleSpeak
setting-braille-math-code = Mathematik-Braille
setting-braille-math-code-help = Der Braille-Code für Mathematik in BRF-Dateien und beim Erkunden einer Formel mit MathCAT: Nemeth oder UEB-Mathematik. Braucht eine Version, die MathCAT enthält; sonst wird Mathematik mit ihren gesprochenen Wörtern geschrieben.
choice-braille-math-code-nemeth = Nemeth
choice-braille-math-code-ueb = UEB
setting-reading-math-display = Mathematik auf dem Bildschirm
setting-reading-math-display-help = Wie Mathematik in der Leseansicht aussieht: als Quelltext, wie x^2, oder als Unicode, wie x mit hochgestellter 2. Sprache und Bearbeitungsmodus verwenden immer den Quelltext.
choice-reading-math-display-source = Quelltext
choice-reading-math-display-unicode = Unicode
setting-reading-revisions = Nachverfolgte Änderungen
setting-reading-revisions-help = Wie nachverfolgte Änderungen in Word-, OpenDocument- und RTF-Dateien gelesen werden: an Ort und Stelle angesagt bei hoher Ausführlichkeit (automatisch), immer, oder nie, dann wird der endgültige Text gelesen. Gilt beim Öffnen eines Dokuments.
choice-reading-revisions-auto = automatisch
choice-reading-revisions-marked = immer ansagen
choice-reading-revisions-final = nur endgültiger Text
setting-display-theme = Design
setting-display-theme-help = Das Farbdesign.
setting-display-follow-os-theme = Dem Systemdesign folgen
setting-display-follow-os-theme-help = Beim Start ein helles, dunkles oder kontrastreiches Design wie das System verwenden, sofern keins gewählt wurde.
setting-display-wrap-width = Umbruchbreite
setting-display-wrap-width-help = Zeilen bei so vielen Spalten umbrechen; 0 nutzt die ganze Breite.
setting-display-tab-width = Tabulatorbreite
setting-display-tab-width-help = Spalten, die ein Tabulator einnimmt.
setting-display-show-line-numbers = Zeilennummern
setting-display-show-line-numbers-help = Zeilennummern anzeigen.
setting-display-scroll-margin = Bildlaufrand
setting-display-scroll-margin-help = Zeilen, die über und unter dem Cursor sichtbar bleiben.
setting-editing-autosave-recovery = Wiederherstellungs-Schnappschüsse
setting-editing-autosave-recovery-help = Eine Kopie nicht gespeicherter Arbeit aufbewahren und sie nach einem Absturz anbieten.
setting-editing-autosave-interval-secs = Schnappschuss-Intervall
setting-editing-autosave-interval-secs-help = Sekunden zwischen Wiederherstellungs-Schnappschüssen, während es ungespeicherte Änderungen gibt.
setting-editing-echo-characters = Zeichen ansagen
setting-editing-echo-characters-help = Jedes getippte Zeichen sagen.
setting-editing-echo-words = Wörter ansagen
setting-editing-echo-words-help = Jedes getippte Wort sagen.
setting-editing-echo-deletions = Löschungen ansagen
setting-editing-echo-deletions-help = Sagen, was Rücktaste und Entf entfernen.
setting-editing-echo-lines-on-move = Zeilen ansagen
setting-editing-echo-lines-on-move-help = Die Zeile sagen, wenn sich der Cursor in eine andere Zeile bewegt.
setting-editing-undo-steps = Rückgängig-Schritte
setting-editing-undo-steps-help = Meiste Rückgängig-Schritte, die beim Bearbeiten aufbewahrt werden.
setting-editing-undo-memory-mb = Rückgängig-Speicher
setting-editing-undo-memory-mb-help = Meister Speicher, den der Rückgängig-Verlauf nutzen darf.
setting-library-recent-limit = Zuletzt verwendete Dateien
setting-library-recent-limit-help = Wie viele zuletzt verwendete Dateien gemerkt werden.
setting-library-folders = Bibliotheksordner
setting-library-folders-help = Ordner, deren Dokumente die Bibliothek auflistet und deren Positionen zwischen Computern synchronisiert werden.
setting-keyboard-character-keys = Einzeltasten-Kurzbefehle
setting-keyboard-character-keys-help = Durchsuch-Tasten wie h und Punkt. Aus, dann lösen Diktat und Tippen nie Befehle aus.
setting-keyboard-preset = Tasten
setting-keyboard-preset-help = Die Standardtasten: wie der Durchsuchmodus von NVDA und JAWS, oder die früheren Tasten von textweaver. Ab dem nächsten Start verwendet.
choice-keyboard-preset-default = Screenreader-Stil
choice-keyboard-preset-classic = klassisch
setting-keyboard-digit-row = Zifferreihe
setting-keyboard-digit-row-help = Wie das Terminal die Zifferntasten für Überschriftsebenen erkennt: automatisch, oder eine französische AZERTY-Tastatur.
choice-keyboard-digit-row-auto = automatisch
choice-keyboard-digit-row-azerty = AZERTY
setting-accessibility-mode = Zugänglichkeitsmodus
setting-accessibility-mode-help = Selbstsprechend spricht alles; Screenreader überlässt das Sprechen Ihrem Screenreader; Hybrid vertont nur das Lesen.
choice-accessibility-mode-self-voicing = Selbstsprechend
choice-accessibility-mode-screen-reader = Screenreader
choice-accessibility-mode-hybrid = Hybrid
setting-accessibility-say-all = Alles vorlesen mit einem Screenreader
setting-accessibility-say-all-help = Fortlaufendes Lesen im Screenreader-Modus: satzweise auf der Statuszeile, oder mit der Stimme von textweaver.
choice-accessibility-say-all-screen = auf der Statuszeile
choice-accessibility-say-all-voice = mit der Stimme von textweaver
setting-accessibility-quiet-screen = Ruhiger Bildschirm beim Lesen
setting-accessibility-quiet-screen-help = Den Bildschirm ruhig halten, während textweaver vorliest.
setting-accessibility-cursor = Cursor
setting-accessibility-cursor-help = Wo der Cursor des Terminals wartet: bei dem, woran Sie arbeiten, oder auf der Statuszeile.
choice-accessibility-cursor-follow = folgt dem Fokus
choice-accessibility-cursor-status = auf der Statuszeile
setting-export-subtitle-format = Untertitelformat
setting-export-subtitle-format-help = Das Format der ohne Dateinamen geschriebenen Untertitel.
choice-export-subtitle-format-srt = SubRip
choice-export-subtitle-format-vtt = WebVTT
setting-export-subtitle-word-level = Wort-Untertitel
setting-export-subtitle-word-level-help = Ein Untertitel pro Wort statt Untertitelzeilen.
setting-export-subtitles-with-audio = Untertitel mit Audio
setting-export-subtitles-with-audio-help = Immer Untertitel neben exportiertem Audio schreiben.
setting-reading-aids-rsvp-wpm = RSVP-Geschwindigkeit
setting-reading-aids-rsvp-wpm-help = Wörter pro Minute der schnellen sequenziellen visuellen Darstellung.
setting-reading-aids-rsvp-pacing = RSVP-Taktung
setting-reading-aids-rsvp-pacing-help = Was das RSVP-Wort weiterbewegt: sein eigener Timer, oder die Sprachausgabe.
choice-reading-aids-rsvp-pacing-timer = sein eigener Timer
choice-reading-aids-rsvp-pacing-external = Sprache
setting-reading-aids-rsvp-clause-pause = RSVP-Satzgliedpause
setting-reading-aids-rsvp-clause-pause-help = Extra Zeit nach einem Komma, Doppelpunkt, Gedankenstrich oder einer Klammer, in Prozent der Wortzeit.
setting-reading-aids-rsvp-sentence-pause = RSVP-Satzpause
setting-reading-aids-rsvp-sentence-pause-help = Extra Zeit am Satzende, in Prozent.
setting-reading-aids-rsvp-paragraph-pause = RSVP-Absatzpause
setting-reading-aids-rsvp-paragraph-pause-help = Extra Zeit am Absatzende, in Prozent.
setting-reading-aids-rsvp-long-word-len = RSVP langes Wort
setting-reading-aids-rsvp-long-word-len-help = Wörter, die länger als diese Buchstabenzahl sind, erhalten extra Zeit.
setting-reading-aids-rsvp-long-word-step = RSVP Schritt langes Wort
setting-reading-aids-rsvp-long-word-step-help = Extra Zeit pro Buchstabe über die Länge eines langen Worts hinaus, in Prozent.
setting-reading-aids-rsvp-long-word-max = RSVP langes Wort Höchstwert
setting-reading-aids-rsvp-long-word-max-help = Meiste extra Zeit, die ein langes Wort erhält, in Prozent.
setting-reading-aids-rsvp-show-previous = RSVP voriges Wort
setting-reading-aids-rsvp-show-previous-help = Auch das vorige Wort anzeigen.
setting-reading-aids-rsvp-show-next = RSVP nächstes Wort
setting-reading-aids-rsvp-show-next-help = Auch das nächste Wort anzeigen.
setting-reading-aids-rsvp-position = RSVP-Position
setting-reading-aids-rsvp-position-help = Wo das RSVP-Wort erscheint.
choice-reading-aids-rsvp-position-top-left = oben links
choice-reading-aids-rsvp-position-top-center = oben mittig
choice-reading-aids-rsvp-position-top-right = oben rechts
choice-reading-aids-rsvp-position-center-left = mittig links
choice-reading-aids-rsvp-position-center = mittig
choice-reading-aids-rsvp-position-center-right = mittig rechts
choice-reading-aids-rsvp-position-bottom-left = unten links
choice-reading-aids-rsvp-position-bottom-center = unten mittig
choice-reading-aids-rsvp-position-bottom-right = unten rechts
setting-reading-aids-rsvp-font-size-pt = RSVP-Größe
setting-reading-aids-rsvp-font-size-pt-help = Größe des RSVP-Worts in der grafischen Oberfläche.
setting-reading-aids-rsvp-lead-words = RSVP-Vorlauf
setting-reading-aids-rsvp-lead-words-help = Bei Sprachtaktung so viele Wörter vor dem gesprochenen Wort anzeigen.
setting-reading-aids-bionic = Bionisches Lesen
setting-reading-aids-bionic-help = Den Anfang jedes Worts fett zeichnen.
setting-reading-aids-bionic-options-ratio = Bionischer Anteil
setting-reading-aids-bionic-options-ratio-help = Wie viel von jedem Wort fett ist.
setting-reading-aids-bionic-options-min-word-len = Bionisch kürzestes Wort
setting-reading-aids-bionic-options-min-word-len-help = Kürzere Wörter als dies werden in Ruhe gelassen.
setting-reading-aids-bionic-options-skip-numbers = Bionisch überspringt Zahlen
setting-reading-aids-bionic-options-skip-numbers-help = Wörter mit Ziffern in Ruhe lassen.
setting-reading-aids-bionic-options-skip-urls = Bionisch überspringt Adressen
setting-reading-aids-bionic-options-skip-urls-help = Web- und E-Mail-Adressen in Ruhe lassen.
setting-reading-aids-bionic-options-skip-code = Bionisch überspringt Code
setting-reading-aids-bionic-options-skip-code-help = Code in Ruhe lassen.
setting-reading-aids-spacing-line-height = Zeilenhöhe
setting-reading-aids-spacing-line-height-help = Zeilenhöhe als Vielfaches der Schriftgröße; der WCAG-Wert ist 1,5.
setting-reading-aids-spacing-paragraph-spacing = Absatzabstand
setting-reading-aids-spacing-paragraph-spacing-help = Abstand nach jedem Absatz, als Vielfaches der Schriftgröße.
setting-reading-aids-spacing-letter-spacing = Buchstabenabstand
setting-reading-aids-spacing-letter-spacing-help = Extra Abstand zwischen Buchstaben, als Vielfaches der Schriftgröße.
setting-reading-aids-spacing-word-spacing = Wortabstand
setting-reading-aids-spacing-word-spacing-help = Extra Abstand zwischen Wörtern, als Vielfaches der Schriftgröße.
setting-reading-aids-font-family = Schrift
setting-reading-aids-font-family-help = Die Leseschrift der grafischen Oberfläche; jede installierte Familie kann eingegeben werden.
choice-reading-aids-font-family-system-ui = die Systemschrift
choice-reading-aids-font-family-sans = serifenlos
choice-reading-aids-font-family-serif = Serifenschrift
choice-reading-aids-font-family-monospace = Festbreitenschrift
choice-reading-aids-font-family-atkinson = Atkinson Hyperlegible
choice-reading-aids-font-family-opendyslexic = OpenDyslexic
choice-reading-aids-font-family-lexend = Lexend
setting-reading-aids-font-size-pt = Schriftgröße
setting-reading-aids-font-size-pt-help = Die Schriftgröße der grafischen Oberfläche.
setting-reading-aids-font-weight = Schriftstärke
setting-reading-aids-font-weight-help = 400 ist regulär, 700 fett.
setting-reading-aids-ruler-mode = Leselineal
setting-reading-aids-ruler-mode-help = Die aktuelle Zeile markieren, oder ein Band von Zeilen.
choice-reading-aids-ruler-mode-off = aus
choice-reading-aids-ruler-mode-current-line = aktuelle Zeile
choice-reading-aids-ruler-mode-ruler = Lineal
setting-reading-aids-ruler-scope = Lineal umfasst
setting-reading-aids-ruler-scope-help = Eine umgebrochene Zeile, oder die ganze Zeile.
choice-reading-aids-ruler-scope-row = eine umgebrochene Zeile
choice-reading-aids-ruler-scope-line = die ganze Zeile
setting-reading-aids-ruler-rows-above = Lineal-Zeilen darüber
setting-reading-aids-ruler-rows-above-help = Zeilen des Bands über der aktuellen.
setting-reading-aids-ruler-rows-below = Lineal-Zeilen darunter
setting-reading-aids-ruler-rows-below-help = Zeilen des Bands unter der aktuellen.
setting-reading-aids-ruler-mask-outside = Lineal-Abdunkelung
setting-reading-aids-ruler-mask-outside-help = Die Zeilen außerhalb des Bands abdunkeln.
setting-reading-aids-syllables = Silben
setting-reading-aids-syllables-help = Wörter in Silben mit einem Mittelpunkt getrennt zeichnen; die Sprachausgabe bleibt unverändert.
setting-reading-aids-difficult-words = Schwierige Wörter
setting-reading-aids-difficult-words-help = Seltene Wörter unterstreichen und sie bei Wortbewegungen mit hoher Ausführlichkeit nennen.
setting-reading-aids-syllable-options-separator = Silbentrennzeichen
setting-reading-aids-syllable-options-separator-help = Was zwischen Silben gezeichnet wird.
setting-reading-aids-syllable-options-left-min = Silben erste Trennung
setting-reading-aids-syllable-options-left-min-help = Wenigste Buchstaben vor der ersten Trennung.
setting-reading-aids-syllable-options-right-min = Silben letzte Trennung
setting-reading-aids-syllable-options-right-min-help = Wenigste Buchstaben nach der letzten Trennung.
setting-reading-aids-syllable-options-min-word-len = Silben kürzestes Wort
setting-reading-aids-syllable-options-min-word-len-help = Kürzere Wörter als dies werden nie getrennt.
setting-reading-aids-syllable-options-skip-urls = Silben überspringen Adressen
setting-reading-aids-syllable-options-skip-urls-help = Web- und E-Mail-Adressen in Ruhe lassen.
setting-reading-aids-syllable-options-skip-code = Silben überspringen Code
setting-reading-aids-syllable-options-skip-code-help = Code in Ruhe lassen.
setting-preview-auto-reload = Die Vorschau neu laden
setting-preview-auto-reload-help = Die Browser-Vorschau nach jedem Speichern neu laden, über einen kleinen Server nur auf diesem Computer.
setting-preview-live = Live-Vorschau
setting-preview-live-help = Bei aktivem Neuladen auch neu laden, wenn das Tippen pausiert.
setting-lexicon-glossary = Glossar
setting-lexicon-glossary-help = Ihr eigenes Glossar, vor dem Wörterbuch nachgeschlagen: Begriff: Definition-Zeilen, oder das JSON von Star. Nicht gesetzt verwendet glossary.txt im Einstellungsordner.
setting-lexicon-data-file = Wörterbuchdatei
setting-lexicon-data-file-help = Das Wort-Definieren-Wörterbuch, lexicon-en.twlex. Nicht gesetzt schaut neben dem Programm.
setting-stats-enabled = Lesestatistik
setting-stats-enabled-help = Die vorgelesene Zeit, den weitesten Punkt und die Sitzungen für jedes Dokument zählen.
setting-interface-language = Oberflächensprache
setting-interface-language-help = Die Sprache der eigenen Wörter von textweaver, sofort geändert. Die Stimme folgt, wenn die Engine eine dafür hat; sonst bleibt die Stimme.
choice-interface-language-en = English
choice-interface-language-es = Español
choice-interface-language-fr = Français
choice-interface-language-de = Deutsch
choice-interface-language-pt = Português
choice-interface-language-ar = العربية
choice-interface-language-en-xa = Test: akzentuiert
choice-interface-language-ar-xb = Test: rechts nach links
setting-interface-rtl = Rechts-nach-links-Anzeige
setting-interface-rtl-help = Ob der Terminal-Reader rechts-nach-links-Text für die Anzeige neu ordnet: automatisch überlässt es Terminals, die es selbst tun. Sprachausgabe und Screenreader erhalten den Text immer in Lesereihenfolge.
choice-interface-rtl-auto = automatisch
choice-interface-rtl-on = an
choice-interface-rtl-off = aus
setting-gui-announce = Ankündigungen
setting-gui-announce-help = Wie die Meldungen des Fensters den Screenreader erreichen, ab dem nächsten Start: eine Live-Region, oder UI-Automation-Benachrichtigungen (nur Windows).
choice-gui-announce-live = Live-Region
choice-gui-announce-uia = UI-Automation-Benachrichtigungen

## Units, said after a number.

settings-unit-words-per-minute = Wörter pro Minute
settings-unit-percent = Prozent
settings-unit-semitones = Halbtöne
settings-unit-milliseconds = Millisekunden
settings-unit-words = Wörter
settings-unit-times = Mal
settings-unit-places = Stellen
settings-unit-columns = Spalten
settings-unit-lines = Zeilen
settings-unit-seconds = Sekunden
settings-unit-steps = Schritte
settings-unit-megabytes = Megabyte
settings-unit-files = Dateien
settings-unit-letters = Buchstaben
settings-unit-points = Punkt
settings-unit-rows = Zeilen

## Settings sections.

section-speech = Sprache
section-highlight = Hervorheben
section-normalization = Text sprechen
section-reading = Lesen
section-display = Anzeige
section-editing = Bearbeiten
section-library = Bibliothek
section-keyboard = Tastatur
section-accessibility = Zugänglichkeit
section-export = Exportieren
section-braille = Braille
section-reading-aids = Lesehilfen
section-preview = Vorschau
section-lexicon = Wort definieren
section-stats = Lesestatistik
section-interface = Oberfläche
section-gui = Fenster

## Edit mode: entering, leaving, saving, and typing.

# $key makes a new document.
edit-no-document = Kein Dokument zum Bearbeiten. Drücken Sie { $key } für ein neues.
# $line is the line at the caret, as echoed.
edit-mode-on-brief = Bearbeitungsmodus an. { $line }
# $save and $finish are the keys that save and leave edit mode; $line is
# the line at the caret.
edit-mode-on = Bearbeitungsmodus an. Speichern: { $save }. Beenden: { $finish }. { $line }
# Keep the letters s, d and c: they are the keys that answer.
edit-unsaved-question = { $title } hat ungespeicherte Änderungen. Speichern, verwerfen oder abbrechen? Drücken Sie s, d oder c, oder Auf und Ab und Eingabetaste. Escape bricht ab.
edit-save-changes-title = Änderungen an { $title } speichern?
edit-choice-save = Speichern, dann fortfahren
edit-choice-discard = Die Änderungen verwerfen
edit-choice-cancel = Abbrechen, weiter bearbeiten
edit-save-failed = Konnte nicht speichern: { $error }. Noch in Bearbeitung.
# The Save As prompt; $path is the suggested file.
edit-save-as-label = Speichern unter, Eingabetaste für { $path }
# $name is a file name. Keep the letters y and n.
edit-file-exists-question = { $name } existiert bereits. Ersetzen? y oder n.
edit-mode-off = Bearbeitungsmodus aus.
edit-mode-off-discarded = Änderungen verworfen. Bearbeitungsmodus aus.
# The title of a new, unsaved document.
edit-untitled = Unbenannt
edit-new-document = Neues Dokument bereit zur Bearbeitung.
# $key turns on edit mode.
edit-nothing-to-save = Nichts zu speichern. Schalten Sie den Bearbeitungsmodus mit { $key } ein, um Änderungen vorzunehmen.
# $key turns on edit mode; $what is what the user tried to do.
edit-not-editing =
    { $what ->
        [type] Schalten Sie den Bearbeitungsmodus mit { $key } ein, um zu tippen.
        [change-text] Schalten Sie den Bearbeitungsmodus mit { $key } ein, um den Text zu ändern.
        [delete-text] Schalten Sie den Bearbeitungsmodus mit { $key } ein, um Text zu löschen.
        [undo] Schalten Sie den Bearbeitungsmodus mit { $key } ein, um rückgängig zu machen.
        [redo] Schalten Sie den Bearbeitungsmodus mit { $key } ein, um zu wiederholen.
        [replace-text] Schalten Sie den Bearbeitungsmodus mit { $key } ein, um Text zu ersetzen.
        [insert] Schalten Sie den Bearbeitungsmodus mit { $key } ein, um in den Text einzufügen.
        [cut-text] Schalten Sie den Bearbeitungsmodus mit { $key } ein, um Text auszuschneiden.
        [move-cells] Schalten Sie den Bearbeitungsmodus mit { $key } ein, um zwischen Tabellenzellen zu wechseln.
        [delete-words] Schalten Sie den Bearbeitungsmodus mit { $key } ein, um Wörter zu löschen.
        [paste] Schalten Sie den Bearbeitungsmodus mit { $key } ein, um einzufügen.
        [citation] Schalten Sie den Bearbeitungsmodus mit { $key } ein, um ein Zitat einzufügen.
        [bibliography] Schalten Sie den Bearbeitungsmodus mit { $key } ein, um ein Literaturverzeichnis einzufügen.
       *[format] Schalten Sie den Bearbeitungsmodus mit { $key } ein, um Text zu formatieren.
    }
# A paste: $n characters.
edit-pasted =
    { $n ->
        [one] 1 Zeichen eingefügt.
       *[other] { $n } Zeichen eingefügt.
    }
# $start is how the pasted text starts.
edit-pasted-start =
    { $n ->
        [one] 1 Zeichen eingefügt: { $start }
       *[other] { $n } Zeichen eingefügt: { $start }
    }
edit-insert-failed = Konnte nicht einfügen: { $error }
# $start and $end are character positions, $len the text's length.
edit-range-out-of-text = Zeichen { $start } bis { $end } können nicht geändert werden: der Text hat { $len }.
edit-change-failed = Der Text konnte nicht geändert werden: { $error }
edit-delete-failed = Konnte nicht löschen: { $error }
edit-list-ended = Liste beendet.
# Said when Enter continues a bulleted list.
edit-bullet = Aufzählungspunkt
edit-table-divider = Tabellenkopf-Trennlinie
edit-end-of-line-stop = Ende der Zeile.
edit-start-of-line-stop = Anfang der Zeile.
edit-end-of-line-content = Ende der Zeile

## Edit mode: formatting, undo, tables, images, and replace.

# $what names the formatting command; $level is a heading level, and
# $cols and $rows a table's size.
edit-format-done =
    { $what ->
        [bold] Fett.
        [italic] Kursiv.
        [underline] Unterstrichen.
        [strikethrough] Durchgestrichen.
        [code] Code.
        [code-block] Codeblock.
        [link] Link.
        [bulleted-list] Aufzählungsliste.
        [numbered-list] Nummerierte Liste.
        [block-quote] Blockzitat.
        [horizontal-rule] Horizontale Linie eingefügt.
        [table-row] Tabellenzeile hinzugefügt.
        [heading-level] Überschriftsebene { $level }.
        [table] Tabelle eingefügt, { $cols } Spalten mal { $rows } Zeilen.
       *[heading] Überschrift.
    }
# The command toggled its markup off.
edit-format-removed =
    { $what ->
        [bold] Fett entfernt.
        [italic] Kursiv entfernt.
        [underline] Unterstrichen entfernt.
        [strikethrough] Durchgestrichen entfernt.
        [code] Code entfernt.
        [code-block] Codeblock entfernt.
        [link] Link entfernt.
        [bulleted-list] Aufzählungsliste entfernt.
        [numbered-list] Nummerierte Liste entfernt.
        [block-quote] Blockzitat entfernt.
        [horizontal-rule] Eingefügte horizontale Linie entfernt.
        [table-row] Hinzugefügte Tabellenzeile entfernt.
        [heading-level] Überschriftsebene { $level } entfernt.
        [table] Eingefügte Tabelle, { $cols } Spalten mal { $rows } Zeilen, entfernt.
       *[heading] Überschrift entfernt.
    }
edit-format-unchanged =
    { $what ->
        [bold] Fett: nichts geändert.
        [italic] Kursiv: nichts geändert.
        [underline] Unterstrichen: nichts geändert.
        [strikethrough] Durchgestrichen: nichts geändert.
        [code] Code: nichts geändert.
        [code-block] Codeblock: nichts geändert.
        [link] Link: nichts geändert.
        [bulleted-list] Aufzählungsliste: nichts geändert.
        [numbered-list] Nummerierte Liste: nichts geändert.
        [block-quote] Blockzitat: nichts geändert.
        [horizontal-rule] Eingefügte horizontale Linie: nichts geändert.
        [table-row] Hinzugefügte Tabellenzeile: nichts geändert.
        [heading-level] Überschriftsebene { $level }: nichts geändert.
        [table] Eingefügte Tabelle, { $cols } Spalten mal { $rows } Zeilen: nichts geändert.
       *[heading] Überschrift: nichts geändert.
    }
# Added after a formatting message; $text is the start of the selection.
edit-format-selected = Ausgewählt: { $text }
edit-heading-level-now = Überschriftsebene { $level }.
# $line is the line at the caret after the undo or redo.
edit-undo-redo =
    { $what ->
        [undo] Rückgängig.
       *[redo] Wiederholen.
    }
edit-undo-redo-line =
    { $what ->
        [undo] Rückgängig. { $line }
       *[redo] Wiederholen. { $line }
    }
edit-nothing-to-undo = Nichts rückgängig zu machen.
edit-nothing-to-redo = Nichts zu wiederholen.
edit-not-a-table-size = Keine Tabellengröße: { $text }. Geben Sie Spalten und Zeilen ein, zum Beispiel 3 mal 2.
# $name is the image's file name.
edit-image-inserted = Bild { $name } eingefügt. Seine Beschreibung ist ausgewählt; tippen Sie, um sie zu ersetzen.
edit-image-failed = Das Bild konnte nicht eingefügt werden: { $error }.
# $query is the text to find.
edit-no-matches = Keine Treffer für { $query }.
# $n matches of $query were found; the replacement is asked next.
edit-replace-with =
    { $n ->
        [one] 1 Treffer für { $query }. Ersetzen durch?
       *[other] { $n } Treffer für { $query }. Ersetzen durch?
    }

## Edit mode: autosave and recovering unsaved work.

edit-recovery-write-failed = Die Wiederherstellungskopie konnte nicht geschrieben werden: { $error }. Speichern Sie bald; { -brand } versucht es weiter.
edit-recovery-writing-again = Die Wiederherstellungskopie wird erneut geschrieben.
# $title is the document; $when is how long ago its work was saved.
edit-recovery-offer = { -brand } wurde mit ungespeicherten Änderungen an { $title } geschlossen, gespeichert { $when }. Jetzt wiederherstellen? Auf und Ab wählen, Eingabetaste bestätigt.
edit-recovery-title = Ungespeicherte Arbeit in { $title } wiederherstellen?
edit-recovery-yes = Ja, { $title } wiederherstellen und weiter bearbeiten
edit-recovery-no = Nein, die ungespeicherten Änderungen verwerfen
edit-recovery-discarded = Die ungespeicherten Änderungen an { $title } verworfen.
edit-recovered = Ungespeicherte Arbeit in { $title } wiederhergestellt. Denken Sie ans Speichern.
edit-recovery-postponed = Wiederherstellung verschoben. Die ungespeicherte Arbeit wird beim nächsten Mal wieder angeboten.

## Find and replace, one match at a time.

# $title is replace-match-title. Keep the letters r, s and a: they are the
# keys that answer.
replace-match-question = { $title }. Drücken Sie r zum Ersetzen, s zum Überspringen, a für Alle ersetzen, Escape zum Stoppen.
# The replace list's title when no match is being asked about.
replace-title = Ersetzen
# $n is this match's number, $total the number of matches, $line the line
# number, and $context the text of that line.
replace-match-title = Treffer { $n } von { $total }, Zeile { $line }: { $context }
replace-item-this = Diesen ersetzen
replace-item-skip = Diesen überspringen
replace-item-rest = Alle restlichen ersetzen
# $state is common-on or common-off.
replace-item-match-case = Groß-/Kleinschreibung beachten: { $state }
replace-item-whole-words = Nur ganze Wörter: { $state }
replace-failed = Konnte nicht ersetzen: { $error }
# Said after switching match case; $state is common-on or common-off, and
# $n is the number of matches now.
replace-match-case-now =
    { $n ->
        [one] Groß-/Kleinschreibung beachten { $state }. 1 Treffer.
       *[other] Groß-/Kleinschreibung beachten { $state }. { $n } Treffer.
    }
replace-whole-words-now =
    { $n ->
        [one] Nur ganze Wörter { $state }. 1 Treffer.
       *[other] Nur ganze Wörter { $state }. { $n } Treffer.
    }
# $query is the text that was searched for.
replace-no-matches = Keine Treffer für { $query }.
replace-replaced =
    { $n ->
        [one] 1 Treffer ersetzt.
       *[other] { $n } Treffer ersetzt.
    }
replace-replaced-skipped = { $n } ersetzt, { $skipped } übersprungen.
replace-stopped = Gestoppt. { $n } ersetzt, { $skipped } übersprungen.

## Saving in the background.

writes-still-saving = Speichert noch. Bitte warten.
writes-not-written-in-time = Einige Änderungen konnten nicht rechtzeitig geschrieben werden: die Festplatte antwortet nicht.
# $error is the system's reason.
writes-save-failed = Konnte nicht speichern: { $error }. Noch in Bearbeitung.
# $name is the bookmark's name, $pct where it is.
writes-bookmark-set = Lesezeichen { $name } gesetzt bei { $pct } Prozent.
writes-bookmark-not-saved = Lesezeichen { $name } ist vorerst gesetzt, konnte aber nicht gespeichert werden: { $error }.
writes-recovery-copy-failed = Die Wiederherstellungskopie konnte nicht geschrieben werden: { $error }. Speichern Sie bald; { -brand } versucht es weiter.
writes-recovery-copy-resumed = Die Wiederherstellungskopie wird erneut geschrieben.
# $name is the saved file's name.
writes-saved = { $name } gespeichert. Noch in Bearbeitung.

## Files changed on disk. $name is a file name. Keep the letters y and
## n: they are the keys that answer.

disk-replace-question = { $name } existiert bereits. Ersetzen? y oder n.
# A prompt label, also said with a full stop after it.
disk-not-replaced = Nicht ersetzt. Geben Sie einen anderen Namen ein
# $key is the key for Save As.
disk-not-saved = Nicht gespeichert. Noch in Bearbeitung. Speichern unter, { $key }, behält beide Versionen.
disk-kept-open-version = Die geöffnete Version wurde beibehalten.
disk-overwrite-question = { $name } wurde auf der Festplatte geändert, seit Sie es geöffnet haben. Über diese Änderungen speichern? y oder n.
disk-reload-question = { $name } wurde auf der Festplatte geändert. Neu laden? y oder n.

## Marks found again after a file changed outside textweaver.

relocate-reading-position = Ihre Leseposition
relocate-bookmarks =
    { $n ->
        [one] 1 Lesezeichen
       *[other] { $n } Lesezeichen
    }
relocate-notes =
    { $n ->
        [one] 1 Notiz
       *[other] { $n } Notizen
    }
relocate-highlights =
    { $n ->
        [one] 1 Hervorhebung
       *[other] { $n } Hervorhebungen
    }
# Lists of relocate-* items: "a and b", and "a, b, and c", where $rest is
# every item but the last, joined by commas.
relocate-join-two = { $a } und { $b }
relocate-join-more = { $rest } und { $last }
# $items is a list of the items above; $n how many marks it counts in all.
relocate-moved =
    { $n ->
        [one] { $items } wurde zur Übereinstimmung verschoben
       *[other] { $items } wurden zur Übereinstimmung verschoben
    }
relocate-lost =
    { $n ->
        [one] { $items } konnte nicht gefunden werden und ist markiert
       *[other] { $items } konnten nicht gefunden werden und sind markiert
    }
# $clauses are relocate-moved and relocate-lost, joined by a comma.
relocate-changed = Die Datei hat sich geändert; { $clauses }.

## New documents from templates.

# The built-in templates' names.
templates-essay = Aufsatz
templates-report = Bericht
templates-notes = Notizen
# One of the user's own templates in the list; $name is its file name.
templates-yours = { $name }, Ihre Vorlage
# $folder is where the user's own templates go.
templates-intro =
    { $n ->
        [one] Neues Dokument aus einer Vorlage, 1 Vorlage. Eingabetaste wählt. Ihre eigenen Vorlagen kommen in { $folder }.
       *[other] Neues Dokument aus einer Vorlage, { $n } Vorlagen. Eingabetaste wählt. Ihre eigenen Vorlagen kommen in { $folder }.
    }
# The title given when none is typed.
templates-untitled = Unbenannt
# $template is the template's name, $title the document's, $date today's date (2026-09-26).
templates-created = Neues Dokument aus der Vorlage { $template }: { $title }. Datiert { $date }. Der Cursor steht dort, wo das Schreiben beginnt. Denken Sie ans Speichern.

## Markdown structure said in edit mode, before a line's text or as it
## is typed.

mdline-heading-level = Überschriftsebene { $level }
mdline-bullet = Aufzählungspunkt
# A numbered list item; $n is its number.
mdline-item = Eintrag { $n }
# $item is mdline-bullet or mdline-item.
mdline-task-done = { $item }, Aufgabe erledigt
mdline-task-not-done = { $item }, Aufgabe nicht erledigt
mdline-table-row = Tabellenzeile
mdline-quote = Zitat
mdline-code-fence = Code-Umrandung
mdline-task = Aufgabe
# Said as "1. " is typed at the start of a line; $n is the number as typed.
mdline-numbered-item = nummerierter Eintrag { $n }

## Moving through tables by row and cell. $dir is next (forward) or
## previous (backward).

tables-not-in-table = Nicht in einer Tabelle.
tables-edge-of-table =
    { $dir ->
        [next] Ende der Tabelle.
       *[previous] Anfang der Tabelle.
    }
tables-edge-of-row =
    { $dir ->
        [next] Ende der Zeile.
       *[previous] Anfang der Zeile.
    }
# $cell is the cell's text, after its column header and a colon when it has one.
tables-header-row = Kopfzeile, { $cell }
tables-row = Zeile { $row }, { $cell }
# High verbosity: $message is what the move said, then where it is.
tables-with-position = { $message }. Zeile { $row } von { $rows }, Spalte { $col } von { $cols }
# Say Position in a table.
tables-position = Tabelle, Zeile { $row } von { $rows }, Spalte { $col } von { $cols }.

## Authoring quick wins: word count, links, clipboard, table cells,
## deleting words, and cycling settings.

# Code block languages said with ordinary words; proper names such as
# Python are not translated.
authoring-language-jsx = JavaScript mit JSX
authoring-language-tsx = TypeScript mit JSX
authoring-language-shell = Shell
authoring-language-batch = Windows-Batch
authoring-language-c-header = C-Header
authoring-language-cpp = C plus plus
authoring-language-csharp = C Sharp
authoring-language-diff = diff
authoring-language-plain-text = einfacher Text
authoring-grammar-not-in-build = Grammatikprüfung ist in dieser Version nicht enthalten.
# $count is $n with thousands separators.
authoring-word-count-selection =
    { $n ->
        [one] 1 Wort in der Auswahl.
       *[other] { $count } Wörter in der Auswahl.
    }
authoring-word-count-document =
    { $n ->
        [one] 1 Wort im Dokument.
       *[other] { $count } Wörter im Dokument.
    }
# $text is the link's text.
authoring-link-no-address = Der Link { $text } hat keine Adresse.
authoring-link-address = Link-Adresse: { $url }
authoring-link-named-address = Link { $text }, Adresse: { $url }
authoring-no-link = Kein Link am Cursor.
authoring-typing-echo =
    { $echo ->
        [characters-and-words] Tipp-Echo: Zeichen und Wörter.
        [characters] Tipp-Echo: Zeichen.
        [words] Tipp-Echo: Wörter.
       *[none] Tipp-Echo: keins.
    }
authoring-nothing-to-copy = Nichts zum Kopieren ausgewählt.
# $text is the first words of what was copied.
authoring-copied = Kopiert: { $text }
authoring-copied-sentence = Den Satz kopiert: { $text }
authoring-nothing-to-cut = Nichts zum Ausschneiden ausgewählt.
authoring-cut = Ausgeschnitten: { $text }
authoring-not-in-table = Nicht in einer Tabelle.
# $dir is next (moving forward) or previous.
authoring-table-edge =
    { $dir ->
        [next] Ende der Tabelle.
       *[previous] Anfang der Tabelle.
    }
# A column with no header text.
authoring-table-column = Spalte { $n }
# Moving into a new row: $header is the column's header, $content the cell.
authoring-table-cell-row = Zeile { $row }. { $header }: { $content }
authoring-nothing-to-select = Nichts auszuwählen.
authoring-selected-all =
    { $n ->
        [one] Alles ausgewählt, 1 Wort.
       *[other] Alles ausgewählt, { $count } Wörter.
    }
authoring-space-deleted = Leerzeichen gelöscht.
# $text is the word deleted.
authoring-deleted = { $text } gelöscht.
# $key is the terminal's own paste key.
authoring-nothing-copied = In { -brand } noch nichts kopiert. Verwenden Sie das Einfügen Ihres Terminals, zum Beispiel { $key }.
authoring-verbosity =
    { $level ->
        [low] Ausführlichkeit: niedrig.
        [high] Ausführlichkeit: hoch.
       *[normal] Ausführlichkeit: normal.
    }
authoring-punctuation =
    { $level ->
        [none] Interpunktion: keine.
        [all] Interpunktion: alle.
       *[some] Interpunktion: etwas.
    }

## Markdown lint (edit mode). A problem is said after "Lint: ".

lint-heading-level = Überschriftsebene { $level } nach Ebene { $prev }; verwenden Sie Ebene { $use }.
# $reference is the link reference's name.
lint-link-reference = Der Linkverweis { $reference } hat keine Definition.
lint-bare-url = Nackte Webadresse; setzen Sie sie in spitze Klammern oder machen Sie einen benannten Link daraus.
# $marker and $used are bullet names: lint-marker-dash and the others.
lint-list-marker = Listenzeichen { $marker }; diese Liste verwendet { $used }.
lint-marker-dash = Gedankenstrich
lint-marker-star = Sternchen
lint-marker-plus = Pluszeichen
lint-marker-other = anderes
# $n is how many tabs and spaces there are.
lint-trailing-tabs-empty-line = Tabulatoren oder Leerzeichen auf einer leeren Zeile.
lint-trailing-tabs-line-end = Tabulatoren oder Leerzeichen am Zeilenende.
lint-trailing-spaces-empty-line =
    { $n ->
        [one] 1 Leerzeichen auf einer leeren Zeile.
       *[other] { $n } Leerzeichen auf einer leeren Zeile.
    }
lint-trailing-spaces-line-end =
    { $n ->
        [one] 1 Leerzeichen am Zeilenende.
       *[other] { $n } Leerzeichen am Zeilenende.
    }
# $key turns on edit mode.
lint-not-editing = Lint prüft das Markdown, das Sie schreiben. Schalten Sie zuerst den Bearbeitungsmodus mit { $key } ein.
lint-not-markdown = Lint prüft Markdown, und dieses Dokument ist kein Markdown.
lint-none = Keine Lint-Probleme.
# $count is $n with thousands separators.
lint-no-more =
    { $n ->
        [one] Kein weiteres Lint-Problem. 1 Lint-Problem insgesamt.
       *[other] Kein weiteres Lint-Problem. { $count } Lint-Probleme insgesamt.
    }
lint-no-earlier =
    { $n ->
        [one] Kein früheres Lint-Problem. 1 Lint-Problem insgesamt.
       *[other] Kein früheres Lint-Problem. { $count } Lint-Probleme insgesamt.
    }
# $message is one of the problems above.
lint-said = Lint: { $message }
# Added at high verbosity.
lint-line = Zeile { $line }.

## Grammar checking (Harper). $message is Harper's own message, in English.

# $words are the words the problem is about.
grammar-said = Grammatik: { $message } Die Wörter: { $words }.
# Said after grammar-said when the first fix removes the words.
grammar-fix-remove = Korrektur: entfernen.
grammar-fix = Korrektur: { $fix }.
# A fix in the fixes list that removes the words.
grammar-remove-the-words = Die Wörter entfernen
grammar-none = Keine Grammatikprobleme gefunden.
# $count is $n with thousands separators.
grammar-no-more =
    { $n ->
        [one] Kein weiteres Grammatikproblem. 1 Grammatikproblem insgesamt.
       *[other] Kein weiteres Grammatikproblem. { $count } Grammatikprobleme insgesamt.
    }
grammar-no-earlier =
    { $n ->
        [one] Kein früheres Grammatikproblem. 1 Grammatikproblem insgesamt.
       *[other] Kein früheres Grammatikproblem. { $count } Grammatikprobleme insgesamt.
    }
# $key opens the fixes list.
grammar-lists-fixes = { $key } listet Korrekturen auf.
# Added at high verbosity.
grammar-line = Zeile { $line }.
# $described is grammar-said (and its fix) without the last full stop.
grammar-no-fix = { $described } Keine Korrektur verfügbar.
grammar-fixes =
    { $n ->
        [one] { $words }: 1 Korrektur.
       *[other] { $words }: { $n } Korrekturen.
    }
grammar-fixes-edit =
    { $n ->
        [one] { $words }: 1 Korrektur. Eingabetaste nimmt die Änderung vor.
       *[other] { $words }: { $n } Korrekturen. Eingabetaste nimmt die Änderung vor.
    }
grammar-left-as-is = So belassen, wie es ist.
# $fix is the fix chosen; $key turns on edit mode.
grammar-fix-not-editing = { $fix }. Schalten Sie den Bearbeitungsmodus mit { $key } ein, um den Text zu ändern.
grammar-removed = Entfernt.
grammar-changed = Geändert zu { $fix }.
grammar-change-failed = Der Text konnte nicht geändert werden: { $error }

## Spell checking.

# Said for an apostrophe when a word is spelled out letter by letter.
spell-apostrophe = Apostroph
spell-not-available = Rechtschreibprüfung ist nicht verfügbar: diese Version hat keine Wortliste.
spell-none-found = Keine Rechtschreibfehler gefunden.
# $count is $n with thousands separators.
spell-no-more =
    { $n ->
        [one] Kein weiterer Rechtschreibfehler. 1 möglicher Rechtschreibfehler insgesamt.
       *[other] Kein weiterer Rechtschreibfehler. { $count } mögliche Rechtschreibfehler insgesamt.
    }
spell-no-earlier =
    { $n ->
        [one] Kein früherer Rechtschreibfehler. 1 möglicher Rechtschreibfehler insgesamt.
       *[other] Kein früherer Rechtschreibfehler. { $count } mögliche Rechtschreibfehler insgesamt.
    }
# Added at high verbosity.
spell-line = Zeile { $line }.
spell-no-misspelled-word = Kein falsch geschriebenes Wort am Cursor.
# $word is the misspelled word; $n how many suggestions follow.
spell-suggestions =
    { $n ->
        [0] { $word }: keine Vorschläge.
        [one] { $word }: 1 Vorschlag.
       *[other] { $word }: { $n } Vorschläge.
    }
spell-suggestions-edit =
    { $n ->
        [0] { $word }: keine Vorschläge. Eingabetaste ersetzt das Wort.
        [one] { $word }: 1 Vorschlag. Eingabetaste ersetzt das Wort.
       *[other] { $word }: { $n } Vorschläge. Eingabetaste ersetzt das Wort.
    }
# $word is the suggestion chosen; $key turns on edit mode.
spell-replace-not-editing = { $word }. Schalten Sie den Bearbeitungsmodus mit { $key } ein, um den Text zu ändern.
spell-replaced = Ersetzt durch { $word }.
spell-replace-failed = Konnte nicht ersetzen: { $error }
spell-left-as-is = So belassen, wie es ist.
spell-added-for-session = { $word } für diese Sitzung zu Ihrer Wortliste hinzugefügt.
spell-added = { $word } zu Ihrer Wortliste hinzugefügt.
spell-save-failed = Ihre Wortliste konnte nicht gespeichert werden: { $error }
# After a save; $count is $n with thousands separators.
spell-count =
    { $n ->
        [0] Keine Rechtschreibfehler.
        [one] 1 möglicher Rechtschreibfehler.
       *[other] { $count } mögliche Rechtschreibfehler.
    }

## The terminal reader's startup.

# $wanted is the speech backend asked for, $backend the one used instead.
tui-setup-backend-unavailable = Sprachausgabe { $wanted } ist nicht verfügbar; { $backend } wird verwendet.
# Shown inside tui-setup-speech-failed as its $error.
tui-setup-backend-not-built = Sprachausgabe { $backend } ist nicht mit eingebaut
tui-setup-speech-failed = Die Sprachausgabe konnte nicht gestartet werden ({ $error }); läuft stumm weiter.
tui-setup-cannot-save = Einstellungen oder Positionen können nicht gespeichert werden: { $error }.
tui-setup-keymap-ignored = Tastenzuordnungsdatei ignoriert: { $error }.
# The first-run welcome. Each value names the key for an action: $play
# reads and pauses, $stop stops, $heading moves to the next heading,
# $help opens the help, $quit quits.
tui-setup-welcome = Willkommen bei { -brand }. { $play } liest vor und pausiert, { $stop } stoppt, { $heading } bewegt zur nächsten Überschrift, { $help } öffnet die Hilfe, und { $quit } beendet.
# Said at startup without a document. $open, $new, and $help name the
# keys for Open, New Document, and Help.
tui-setup-no-document = Kein Dokument ist geöffnet. Drücken Sie { $open }, um eines zu öffnen, { $new } für ein neues, oder { $help } für Hilfe.

## The terminal reader's launch.

# $name is the file asked for on the command line; $error says why.
tui-could-not-open = { $name } konnte nicht geöffnet werden: { $error }

## Copying in the terminal reader.

tui-clip-system = Mit der Systemzwischenablage kopiert, weil dieses Terminal keinen kopierten Text annehmen kann.
# $error is why: tui-clip-not-available, tui-clip-not-built, or the
# system's own message.
tui-clip-failed = Konnte nicht mit der Systemzwischenablage kopieren: { $error }. Stattdessen an das Terminal gesendet.
tui-clip-not-available = die Systemzwischenablage ist nicht verfügbar
tui-clip-not-built = diese Version hat keine Systemzwischenablage

## The terminal reader's screen.

# The title line's start; $title is the document's title or
# tui-title-no-document.
tui-title = { -brand }: { $title }
tui-title-no-document = kein Dokument
# The screen without a document. $keys names the keys for the action.
tui-empty-no-document = Kein Dokument ist geöffnet.
tui-empty-open = Eines öffnen: { $keys }.
tui-empty-help = Hilfe: { $keys }.
tui-empty-quit = Beenden: { $keys }.
# The key hints while a yes-or-no question waits. y, n, and a are the
# answer keys the reader takes; $escape names the Escape key.
tui-hints-confirm = y ja  n oder a nein  { $escape } nein
# Key hint labels, each shown after its key on the bottom line.
tui-hint-play = abspielen
tui-hint-sentence = Satz
tui-hint-faster = schneller
tui-hint-slower = langsamer
tui-hint-close-rsvp = RSVP schließen
tui-hint-quit = beenden
tui-hint-save = speichern
tui-hint-finish = beenden
tui-hint-undo = rückgängig
tui-hint-bold = fett
tui-hint-heading = Überschrift
tui-hint-commands = Befehle
tui-hint-next-line = nächste Zeile
tui-hint-previous-line = vorherige Zeile
tui-hint-again = nochmal
tui-hint-read-on = weiterlesen
tui-hint-leave = verlassen
tui-hint-paragraph = Absatz
tui-hint-find = suchen
tui-hint-mark = markieren
tui-hint-lines = Zeilen
tui-hint-keys = Tasten
# The list overlay's border: $n is the focused item's number, $count
# the number of items.
tui-list-title = { $n } von { $count }, { $title }

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
goto-word-start = start
goto-word-end = end

language-voices-loading = Die Stimmenliste wird noch geladen, daher spricht die aktuelle Stimme weiter.

## The window (GUI)

gui-open-title = Ein Dokument öffnen
gui-open-documents = Dokumente, die textweaver liest
gui-open-all-files = Alle Dateien
gui-open-no-dialog = Die Dateiauswahl des Systems hat sich nicht geöffnet. Geben Sie stattdessen den Pfad des Dokuments ein.
gui-text-size = Textgröße { $size } Punkt.
gui-text-size-largest = Textgröße { $size } Punkt, die größte.
gui-text-size-smallest = Textgröße { $size } Punkt, die kleinste.
gui-font = Schriftart: { $family }.
gui-font-unchanged = Schriftart unverändert.
gui-font-list = Schriftart

## The Braille pass (Wave 5, W5x): pages in paged documents such as a PDF.
## $page and $n are page numbers, $label a printed page label such as iv,
## $pages the number of pages. Keep the page first: a 40-cell Braille
## display shows the start of the line.

status-page = Seite { $page } von { $pages }
status-page-labelled = Seite { $label }, { $n } von { $pages }
status-position-page = { $page }, { $pct }%
pages-position = Seite { $page } von { $pages }.
pages-position-labelled = Seite { $label }, { $n } von { $pages }.
pages-none = Dieses Dokument hat keine Seiten.
pages-no-such-page = Keine Seite { $page }. Die Seiten gehen von 1 bis { $pages }.
pages-label = Seite { $label }
pages-outline-item = Seite { $label }: { $text }
lists-pages-title =
    { $n ->
        [one] Seiten, { $n } Seite
       *[other] Seiten, { $n } Seiten
    }
lists-pages-title-filtered = Seiten, { $shown } von { $n } stimmen mit { $filter } überein
lists-pages-intro =
    { $n ->
        [one] Seiten, { $n } Seite. Tippen filtert, Eingabetaste springt zu einer Seite, Escape schließt.
       *[other] Seiten, { $n } Seiten. Tippen filtert, Eingabetaste springt zu einer Seite, Escape schließt.
    }
lists-pages-here = Sie sind auf { $heading }.
lists-filter-cleared-pages =
    { $n ->
        [one] Filter gelöscht, { $n } Seite.
       *[other] Filter gelöscht, { $n } Seiten.
    }
lists-filter-none-pages = Keine Seite stimmt mit { $query } überein. Rücktaste entfernt Buchstaben.
lists-filter-matched-pages =
    { $n ->
        [one] { $n } Seite stimmt überein.
       *[other] { $n } Seiten stimmen überein.
    }
prompt-go-to-pages = Gehe zu Seite, oder Zeile 12, Prozent, start oder end
goto-not-a-target-pages = Kein Sprungziel: { $text }. Geben Sie eine Seitenzahl ein, Zeile und eine Zahl, einen Prozentwert wie 50%, start oder end.
goto-word-page = Seite

## Wave 5 (W5y): der Bibliotheksfilter, das Wörterbuch und die Geschwindigkeiten.

# The library list filtered: $shown of $n documents match $filter.
library-title-filtered = Bibliothek, { $shown } von { $n } stimmen mit { $filter } überein
# The filter was emptied: $n documents are shown.
library-filter-cleared =
    { $n ->
        [one] Filter gelöscht, { $n } Dokument.
       *[other] Filter gelöscht, { $n } Dokumente.
    }
# No document matches the filter $query.
library-filter-none = Kein Dokument stimmt mit { $query } überein. Rücktaste entfernt Buchstaben.
# $n documents match the filter.
library-filter-matched =
    { $n ->
        [one] { $n } Dokument stimmt überein.
       *[other] { $n } Dokumente stimmen überein.
    }
# Said once when define word is used while the dictionary file is still opening.
define-still-loading = Das Wörterbuch wird noch geladen.
# Von W5y hinzugefügte Einstellungen.
setting-speech-dectalk-library = DECtalk-Bibliothek
setting-speech-dectalk-library-help = Die zu ladende DECtalk-Bibliothek; nicht gesetzt sucht an den üblichen Orten.
setting-speech-piper-voices = Ordner der Piper-Stimmen
setting-speech-piper-voices-help = Der Ordner der Piper-Stimmen; nicht gesetzt nutzt den Ordner piper im Datenordner von textweaver.
setting-speech-piper-voice = Piper-Stimme
setting-speech-piper-voice-help = Die Piper-Stimme zum Start, nach ID; nicht gesetzt nimmt die erste installierte.
setting-speech-piper-phonemizer = Piper-Phonemisierer
setting-speech-piper-phonemizer-help = Wie Piper Text in Laute umwandelt: die espeak-ng-Bibliothek, wenn installiert, diese Bibliothek oder der von textweaver.
choice-speech-piper-phonemizer-auto = automatisch
choice-speech-piper-phonemizer-library = espeak-ng-Bibliothek
choice-speech-piper-phonemizer-rust = der von textweaver
setting-speech-voice-params = Tempo und Tonhöhe je Stimme
setting-speech-voice-params-help = Tempo und Tonhöhe, mit denen jede Stimme zuletzt genutzt wurde; wird die Stimme wieder gewählt, kehren sie zurück.
setting-editing-author = Autor
setting-editing-author-help = Der Autor, der in neue Dokumente aus einer Vorlage geschrieben wird; leer lässt ihn frei.

## The window (GUI), Wave 5 (W5a4): drawn labels, hints, and questions.
## Keep the letters Y and N: they are the keys that answer.

gui-yes = Ja
gui-no = Nein
gui-question-hint = Y antwortet ja, N antwortet nein, Escape antwortet nein.
gui-button-open = Öffnen…
gui-button-font = Schriftart…
gui-button-edit = Bearbeiten
gui-button-finish-editing = Bearbeiten beenden
gui-button-settings = Einstellungen…
gui-button-commands = Befehle…
gui-button-play = Abspielen
gui-button-pause = Pause
gui-button-stop = Stopp
gui-button-previous-sentence = Vorheriger Satz
gui-button-next-sentence = Nächster Satz
gui-button-slower = Langsamer
gui-button-faster = Schneller
gui-button-close = Schließen
gui-toolbar-reading = Lesen
gui-document = Dokument
gui-list-hint = Eingabetaste wählt, Escape schließt.
gui-settings-sections = Bereiche
gui-settings-form = Einstellungen: { $section }
gui-settings-saved-hint = Änderungen wirken sofort und werden sofort gespeichert.
gui-settings-close-help = Die Einstellungen schließen. Jede Änderung ist bereits gespeichert.
gui-settings-closed = Einstellungen geschlossen.
gui-settings-table = { $label } ist eine Tabelle. Bearbeiten Sie sie in settings.toml.
gui-setting-new-value = Neuer Wert für { $label }
gui-setting-value-hint = Eingabetaste übernimmt, Escape geht zurück.
gui-prompt-path-hint = Geben Sie den Pfad eines Dokuments ein und drücken Sie die Eingabetaste. Tab vervollständigt ihn; Pfeil nach oben und unten holen frühere zurück.
gui-prompt-hint = Eingabetaste übernimmt, Escape bricht ab. Pfeil nach oben und unten holen frühere Antworten zurück.
gui-palette-filter = Tippen, um die Befehle zu filtern
gui-palette-list = Befehle
gui-palette-hint = Eingabetaste führt den ersten Treffer aus; Tab wechselt zur Liste.
gui-no-document = Kein Dokument ist geöffnet. Drücken Sie { $key }, um eines zu öffnen.
gui-open-failed = { $path } konnte nicht geöffnet werden: { $error }
gui-uia-unavailable = UI-Automation-Benachrichtigungen gibt es nur unter Windows; die Live-Region wird verwendet.
gui-rsvp = RSVP
gui-rsvp-playing = RSVP läuft, Wort { $n } von { $total }
gui-rsvp-paused = RSVP pausiert, Wort { $n } von { $total }
gui-rsvp-finished = RSVP beendet, Wort { $n } von { $total }
gui-settings-section-item =
    { $section }, { $n ->
        [one] 1 Einstellung
       *[other] { $n } Einstellungen
    }
gui-palette-count =
    { $n ->
        [0] Kein Befehl passt.
        [one] 1 Befehl.
       *[other] { $n } Befehle.
    }
gui-settings-form-help = Pfeil nach oben und unten wechseln zwischen Einstellungen. Pfeil nach links und rechts ändern eine. Die Eingabetaste gibt einen neuen Wert ein. Entf stellt den Standard wieder her. { $next } und { $previous } wechseln den Bereich.
gui-settings-press-enter = Drücken Sie die Eingabetaste, um einen neuen Wert für { $label } einzugeben.
gui-font-built-in = { $family } (eingebaut)
