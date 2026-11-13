### Authoring: lists, citations, find and replace, tables, quick wins, and dictation.

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

## Citations: inserting, looking up, importing, checking, and the bibliography.

citations-on = Zitate an.
citations-off = Zitate aus.
book-audio-on = Hörbuchaufnahme an. Die Aufnahme spielt, wo das Buch eine hat.
book-audio-off = Hörbuchaufnahme aus. Die Sprachausgabe liest das Buch.
book-audio-rate = Das Tempo gilt nicht für aufgenommenes Audio.
# $key names the Add Reference command's keys.
citations-library-empty = Ihre Literaturbibliothek ist leer. Fügen Sie eine Literaturangabe per DOI, ISBN oder PMID hinzu mit { $key }, oder führen Sie Literaturangaben importieren aus der Befehlspalette aus.
# $n is how many references the picker lists.
citations-picker-intro =
    { $n ->
        [one] Zitat einfügen, { $n } Literaturangabe. Tippen filtert, Eingabetaste wählt, Escape bricht ab.
       *[other] Zitat einfügen, { $n } Literaturangaben. Tippen filtert, Eingabetaste wählt, Escape bricht ab.
    }
# $text is what was typed at the locator prompt.
citations-locator-unreadable = Der Fundort { $text } konnte nicht gelesen werden. Geben Sie eine Seite wie 12 ein, Seiten wie 3-5, oder Kapitel 2; Eingabetaste allein für keinen.
citations-insert-failed = Das Zitat konnte nicht eingefügt werden: { $error } Der Text ist unverändert.
# $what is the identifier being looked up, as the citation library describes it.
citations-looking-up = { $what } wird nachgeschlagen.
citations-lookup-not-started = Das Nachschlagen konnte nicht gestartet werden: { $error } Warten Sie kurz und versuchen Sie es dann erneut.
# $input is the DOI or ISBN as typed.
citations-lookup-failed = { $input } konnte nicht nachgeschlagen werden: { $error } Prüfen Sie die DOI, ISBN oder PMID und die Verbindung.
citations-no-library-to-add-to = Es gibt keine Bibliothek zum Hinzufügen: { -brand } hält in dieser Sitzung keine Dateien.
citations-library-save-failed = Die Bibliothek konnte nicht gespeichert werden: { $error } Prüfen Sie, ob in seinen Ordner geschrieben werden kann.
# $n is how many citations the document has.
citations-found-no-library =
    { $n ->
        [one] { $n } Zitat gefunden. { -brand } hält in dieser Sitzung keine Bibliothek.
       *[other] { $n } Zitate gefunden. { -brand } hält in dieser Sitzung keine Bibliothek.
    }
citations-check-failed = Die Zitate konnten nicht geprüft werden: { $error } Prüfen Sie die Datei der Literaturbibliothek.
citations-no-library-to-import-into = Es gibt keine Bibliothek zum Importieren: { -brand } hält in dieser Sitzung keine Dateien.
# $file is the file's path.
citations-import-failed = { $file } konnte nicht importiert werden: { $error } Prüfen Sie, ob es eine .bib-, .ris- oder .json-Datei ist.
# $style is the style's name from the front matter, such as apa.
citations-style-unusable = Der Zitierstil { $style } kann nicht verwendet werden: { $error } Prüfen Sie den Stilnamen am Anfang des Dokuments.
citations-format-failed = Die Zitate konnten nicht formatiert werden: { $error } Prüfen Sie die Zitierschlüssel und die Bibliothek.
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
citations-bibliography-insert-failed = Das Literaturverzeichnis konnte nicht eingefügt werden: { $error } Der Text ist unverändert.

## Find and replace, one match at a time.

# $title is replace-match-title. Keep the letters r, s and a: they are the
# keys that answer.
replace-match-question = { $title }. Die Zeile: { $context }. Drücken Sie r zum Ersetzen, s zum Überspringen, a für Alle ersetzen, Escape zum Stoppen.
# The replace list's title when no match is being asked about.
replace-title = Ersetzen
# $n is this match's number, $total the number of matches, $line the line
# number, $found the matched text, and $result what it becomes (both
# shortened); the -removed form is for an empty replacement. $context in
# replace-match-question is the text of the match's line.
replace-match-title = Treffer { $n } von { $total }, Zeile { $line }: { $found } wird { $result }
replace-match-title-removed = Treffer { $n } von { $total }, Zeile { $line }: { $found } wird entfernt
replace-item-this = Diesen ersetzen
replace-item-skip = Diesen überspringen
replace-item-rest = Alle restlichen ersetzen
# $state is common-on or common-off.
replace-item-match-case = Groß-/Kleinschreibung beachten: { $state }
replace-item-whole-words = Nur ganze Wörter: { $state }
replace-failed = Konnte nicht ersetzen: { $error } Der Text ist unverändert.
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
replace-replaced =
    { $n ->
        [one] 1 Treffer ersetzt.
       *[other] { $n } Treffer ersetzt.
    }
replace-replaced-skipped = { $n } ersetzt, { $skipped } übersprungen.
replace-stopped = Gestoppt. { $n } ersetzt, { $skipped } übersprungen.
# Find and replace with regular expressions (B1-fr). $state is common-on
# or common-off; $n is the number of matches now.
replace-item-regex = Regulärer Ausdruck: { $state }
replace-item-across-lines = Über Zeilen hinweg: { $state }
replace-regex-now =
    { $n ->
        [one] Regulärer Ausdruck { $state }. 1 Treffer.
       *[other] Regulärer Ausdruck { $state }. { $n } Treffer.
    }
replace-across-lines-now =
    { $n ->
        [one] Über Zeilen hinweg { $state }. 1 Treffer.
       *[other] Über Zeilen hinweg { $state }. { $n } Treffer.
    }
# $problem is search-invalid-pattern: switching the option would make the
# pattern invalid.
replace-option-refused = { $problem } Die Option bleibt unverändert.
# Asked once before replacing all the rest; $n is how many. Keep y and n.
replace-all-question =
    { $n ->
        [one] Den letzten verbleibenden Treffer ersetzen? y oder n
       *[other] Alle { $n } verbleibenden Treffer ersetzen? y oder n
    }
replace-all-declined = Nichts ersetzt.
# The search options list, and the options as named in search-options-on.
search-options-title = Suchoptionen
search-option-match-case = Groß- und Kleinschreibung
search-option-whole-words = ganze Wörter
search-option-regex = regulärer Ausdruck
search-option-across-lines = über Zeilen hinweg
# Said as Find or Replace opens when an option is on; $list joins the
# options' names with commas.
search-options-on = Optionen an: { $list }.
# $at is the character where the pattern fails, counting from 1; $reason
# is the regular expression engine's own explanation (in English).
search-invalid-pattern = Ungültiges Muster bei Zeichen { $at }: { $reason }.
search-invalid-pattern-anywhere = Ungültiges Muster: { $reason }.

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

common-not-in-table = Nicht in einer Tabelle.
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
authoring-language-mermaid = Mermaid-Diagramm
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
# $parts lists what came in, from the paste-part messages.
paste-markdown = Als Markdown eingefügt: { $parts }
paste-part-heading =
    { $n ->
        [one] 1 Überschrift
       *[other] { $n } Überschriften
    }
paste-part-paragraph =
    { $n ->
        [one] 1 Absatz
       *[other] { $n } Absätze
    }
paste-part-list =
    { $n ->
        [one] 1 Liste
       *[other] { $n } Listen
    }
paste-part-table =
    { $n ->
        [one] 1 Tabelle
       *[other] { $n } Tabellen
    }
paste-part-code =
    { $n ->
        [one] 1 Codeblock
       *[other] { $n } Codeblöcke
    }
paste-part-quote =
    { $n ->
        [one] 1 Zitat
       *[other] { $n } Zitate
    }
paste-part-link =
    { $n ->
        [one] 1 Link
       *[other] { $n } Links
    }
paste-empty = Nichts einzufügen: die Zwischenablage ist leer.
paste-converting = Der formatierte Text wird zum Einfügen umgewandelt.
paste-failed = Einfügen nicht möglich: { $error } Versuchen Sie: Als reinen Text einfügen.
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

## Dictation in edit mode (ADR-0042). Keep the meaning first: a
## 40-cell Braille display shows the start of the line. $words are the
## dictated words, $key the dictate key, $dir a folder, $error and $text
## are passed on as they are.
dictation-status = Diktat: { $words }
dictation-listening = Diktat läuft. Sprechen Sie, dann { $key } zum Beenden.
dictation-finishing = Diktat wird abgeschlossen.
dictation-done = Diktat fertig.
dictation-busy = Das Diktat wird abgeschlossen. Versuchen Sie es gleich noch einmal.
dictation-needs-edit = Das Diktat schreibt im Bearbeitungsmodus. Bearbeitungsmodus einschalten und diktieren? y oder n
dictation-no-model = Das Diktat braucht das Whisper-Modell in { $dir }. Siehe Dictation in der Dokumentation.
dictation-failed = Diktat fehlgeschlagen: { $error } Siehe Dictation in der Dokumentation.
dictation-no-words = In diesem Satz wurden keine Wörter erkannt.
dictation-lost = Das Diktat wurde beendet, bevor seine letzten Wörter geschrieben waren.
dictation-not-typed = Diktierte Wörter nicht geschrieben, der Bearbeitungsmodus ist aus: { $text }
dictation-paused = Diktat angehalten.
dictation-resumed = Diktat fortgesetzt.
dictation-not-active = Kein Diktat läuft.
dictation-cannot-pause = Dieses Diktat kann nicht anhalten; beenden Sie es stattdessen.
section-dictation = Diktat
