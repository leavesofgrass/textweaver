### Reading: navigation, reading aloud, status, statistics, the queue, and reading aids.

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
stats-turned-off = Lesestatistik ist aus. Aufgezeichnetes bleibt erhalten.

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
stats-clear-item = Lesestatistik entfernen
stats-clear-failed = Die Lesestatistik konnte nicht entfernt werden: { $error }. Prüfen Sie, ob in den Datenordner geschrieben werden kann.
stats-off-cli = Die Lesestatistik ist aus: stats.enabled ist in den Einstellungen false.

## Continue reading and every computer's statistics (the sync wave, S6).

continue-title = Weiterlesen
# $n is the number of documents listed.
continue-intro =
    { $n ->
        [one] Weiterlesen: 1 Dokument, neuestes zuerst.
       *[other] Weiterlesen: { $n } Dokumente, neueste zuerst.
    }
continue-empty = Noch keine Stellen. Lesen speichert sie.
# One row, meaning first: the title, how far in, the computer, and how
# long ago (continue-ago-*).
continue-item = { $title }, { $pct } Prozent, { $device }, { $when }
continue-this-computer = dieser Computer
continue-ago-now = gerade eben
continue-ago-minutes =
    { $n ->
        [one] vor 1 Minute
       *[other] vor { $n } Minuten
    }
continue-ago-hours =
    { $n ->
        [one] vor 1 Stunde
       *[other] vor { $n } Stunden
    }
continue-ago-days =
    { $n ->
        [one] vor 1 Tag
       *[other] vor { $n } Tagen
    }

## Die Leseliste und die Wiederholungsliste (Beta 3).

queue-title = Leseliste
queue-intro =
    { $n ->
        [one] Leseliste, 1 Dokument. Eingabetaste öffnet es, Entf nimmt es heraus.
       *[other] Leseliste, { $n } Dokumente. Eingabetaste öffnet eines, Entf nimmt es heraus.
    }
queue-empty = Die Leseliste ist leer. In der Bibliothek fügt { $key } ein Dokument hinzu.
queue-item-missing = { $title }, nicht gefunden
queue-item-elsewhere = { $title }, nicht auf diesem Computer
queue-untitled = Ein Dokument von einem anderen Computer
queue-elsewhere-chosen = { $title } ist nicht auf diesem Computer. Es bleibt in der Leseliste und wird gefunden, sobald Sie es hier öffnen.
queue-added =
    { $n ->
        [one] Zur Leseliste hinzugefügt: { $title }. 1 Dokument in der Liste.
       *[other] Zur Leseliste hinzugefügt: { $title }. { $n } Dokumente in der Liste.
    }
queue-removed = Aus der Leseliste genommen: { $title }.
queue-no-file = Nur ein als Datei gespeichertes Dokument kann in die Leseliste.
queue-read-failed = Die Bibliothek konnte für die Leseliste nicht gelesen werden: { $error }. Prüfen Sie library.json im Datenordner; tw settings path nennt ihn.
queue-next-ask = Als Nächstes: { $title }. { $key } zum Lesen.
queue-next-reading = Als Nächstes: { $title }.
review-title = Wiederholungsliste
review-intro =
    { $n ->
        [one] Wiederholungsliste, 1 Dokument, am längsten nicht gelesen zuerst. Eingabetaste öffnet es.
       *[other] Wiederholungsliste, { $n } Dokumente, am längsten nicht gelesen zuerst. Eingabetaste öffnet eines.
    }
review-empty = Noch nichts zu wiederholen. Vorgelesene Dokumente erscheinen hier.
review-stats-off = Die Wiederholungsliste braucht die Lesestatistik, die aus ist. Schalten Sie Lesestatistik in den Einstellungen ein.
review-item = { $title }, { $days }, { $pct } Prozent, { $notes }
review-days =
    { $n ->
        [0] heute
        [one] 1 Tag
       *[other] { $n } Tage
    }
review-notes =
    { $n ->
        [0] keine Notizen
        [one] 1 Notiz
       *[other] { $n } Notizen
    }
prompt-document-details = Dokumentdetails

## Edit a document's details by hand.

# $name is the document's title; said when the form opens.
details-intro = Details von { $name }. Tab wechselt, Eingabetaste speichert, Escape bricht ab.
details-label-title = Titel
details-label-author = Autor
details-label-doi = DOI
details-label-isbn = ISBN
# A field's drawn label: its label, then $n of $total fields.
details-prompt-label = { $label }, { $n } von { $total }
# Said on moving to a field: its label and value (or nav-blank).
details-field = { $label }: { $value }
# $fields lists the fields saved, by their labels.
details-saved = Details gespeichert: { $fields }.
details-unchanged = Details nicht geändert.
details-cancelled = Abgebrochen. Details nicht geändert.
# $text is what was typed in the DOI or ISBN field.
details-not-a-doi = Keine DOI: { $text }. Korrigieren oder leeren.
details-not-an-isbn = Keine ISBN: { $text }. Korrigieren oder leeren.
details-no-file = Dieses Dokument hat keine Datei, also keine Details zum Bearbeiten.
details-save-failed = Die Details konnten nicht gespeichert werden: { $error } Versuchen Sie es erneut.
# The statistics list could not wait for the sync folder.
stats-others-slow = Andere Computer fehlen: Ordner langsam.
stats-untitled = Dokument ohne Titel
# One computer's share of a document: $device, $time, $sessions.
stats-computer =
    { $sessions ->
        [one] { $device }: { $time }, 1 Sitzung
       *[other] { $device }: { $time }, { $sessions } Sitzungen
    }
stats-by-computer-off = Je Computer: aus. Eingabetaste zeigt.
stats-by-computer-on = Je Computer: an. Eingabetaste verbirgt.

## Navigation. $dir is next or previous; $what is a kind-* or unit-*
## noun and $unit its key (heading, list-item, sentence), for languages
## whose words agree with the noun.

nav-blank = leer
# Said for a picture that has no description (no alternative text).
nav-no-description = keine Beschreibung
# The document overview: the title first, then the counts. $time is overview-time.
overview-line = { $title }. Überschriften: { $headings }, Tabellen: { $tables }, Bilder: { $pictures }, Fußnoten: { $footnotes }. { $time }
# Reading time left from the cursor at the current rate.
overview-time =
    { $minutes ->
        [0] Weniger als eine Minute übrig.
        [one] Etwa 1 Minute übrig.
       *[other] Etwa { $minutes } Minuten übrig.
    }
# Said when the reading pass changes and as reading starts in a skim. $pass is a reading-pass-* name.
reading-pass-changed = Durchgang: { $pass }.
reading-pass-full = ganzer Text
reading-pass-first-sentences = erste Sätze
reading-pass-headings = Überschriften
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
playback-caps-no-words = Wortmarkierung geschätzt: diese Stimme meldet keine Wörter.
playback-caps-words = Wortmarkierung genau: diese Stimme meldet jedes Wort.

## Reading aloud.

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
playback-end-of-document-content = Ende des Dokuments
playback-no-unit-here = { $what }: nichts hier.
playback-no-selection = Keine Auswahl.
# $next is what happens now (a restart, or nothing).
playback-speech-died = Die Sprachausgabe hat aufgehört zu arbeiten ({ $reason }). { $next }
playback-done-reading = Lesen beendet.
playback-speech-restarted = Sprachausgabe neu gestartet: { $reason }. Weiter ab dem letzten Wort.
playback-speech-error = Fehler der Sprachausgabe: { $error } Versuchen Sie es erneut, oder nutzen Sie den Befehl Sprache neu starten.
playback-end-of-section = Ende des Abschnitts. { $key } liest weiter.
playback-time-up =
    { $minutes ->
        [one] Zeit ist um nach 1 Minute. { $key } liest weiter.
       *[other] Zeit ist um nach { $minutes } Minuten. { $key } liest weiter.
    }
playback-repeat-slower = Langsamer wiederholen, mit { $rate } Wörtern pro Minute.
playback-recall-prompt = Sagen Sie, was Sie von { $section } behalten haben. { $key } liest weiter.

## The title line and Say Status.

# The title line's position.
status-position = Zeile { $line } von { $lines }
status-mode = Modus { $mode }
status-modified = geändert
status-self-voicing = Selbstsprechend
status-hybrid = Hybrid
status-screen-reader = Screenreader-Modus
status-rate-spoken = { $wpm } Wörter pro Minute
status-rate = { $wpm } wpm
status-cw-rate = CW { $wpm } wpm
status-cw-rate-spoken = CW mit { $wpm } Wörtern pro Minute
status-cw-rate-farnsworth = CW { $wpm } wpm, effektiv { $effective }
status-cw-rate-farnsworth-spoken = CW mit { $wpm } Wörtern pro Minute, effektiv { $effective }
# The window's status bar, last: reading time left at the current rate.
status-time-left =
    { $minutes ->
        [0] unter einer Minute übrig
        [one] 1 Minute übrig
       *[other] { $minutes } Minuten übrig
    }
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

## The Braille pass: pages in paged documents such as a PDF.
## $page and $n are page numbers, $label a printed page label such as iv,
## $pages the number of pages. Keep the page first: a 40-cell Braille
## display shows the start of the line.

status-page = Seite { $page } von { $pages }
status-page-labelled = Seite { $label }, { $n } von { $pages }
status-percent = { $pct }%
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

## Summaries and difficult-word definitions.
# The summary list's title: $n sentences of the whole document.
summary-title =
    { $n ->
        [one] Zusammenfassung, { $n } Satz
       *[other] Zusammenfassung, { $n } Sätze
    }
# The summary of the chapter at the cursor.
summary-title-chapter =
    { $n ->
        [one] Kapitelzusammenfassung, { $n } Satz
       *[other] Kapitelzusammenfassung, { $n } Sätze
    }
# The summary of the selection.
summary-title-selection =
    { $n ->
        [one] Auswahlzusammenfassung, { $n } Satz
       *[other] Auswahlzusammenfassung, { $n } Sätze
    }
# Said when the summary list opens; $title is one of the titles above.
summary-intro = { $title }. Eingabe geht zum Satz und sagt ihn.
# The same, when a long text was read in samples.
summary-intro-sampled = { $title }, aus Stichproben dieses langen Textes. Eingabe geht zum Satz und sagt ihn.
summary-none = Nichts zusammenzufassen: kein Satz mit vier oder mehr Wörtern.
# tw summarize, on standard error, when a long text was read in samples: $read of $total characters.
summary-sampled-cli = Ein langer Text: Die Zusammenfassung stammt aus { $read } seiner { $total } Zeichen, in Stichproben gelesen.
# After a difficult word at high verbosity, with definitions on: its first definition.
aids-difficult-word-defined = schwieriges Wort: { $definition }
settings-unit-sentences =
    { $n ->
        [one] Satz
       *[other] Sätze
    }

# The GUI. Said in textweaver's own voice when the window takes the
# focus; $title is the document's title.
gui-window-focused = { $title }, { -brand }.
gui-updated-to = Aktualisiert auf { $version }. Hilfe, Dokumentation beschreibt die Neuerungen.
