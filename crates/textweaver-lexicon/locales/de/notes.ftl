### Notes, highlights, bookmarks, relations, and find.

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
notes-none = Keine Notizen. Eine hinzufügen: { $key }.
notes-list-title = Notizen
notes-list-intro =
    { $n ->
        [one] Notizen, 1 Eintrag. Eingabetaste springt zu einer Notiz, Entf löscht sie, F2 bearbeitet sie, Leertaste öffnet ihre Verknüpfungen. C erstellt eine Karte.
       *[other] Notizen, { $n } Einträge. Eingabetaste springt zu einer Notiz, Entf löscht sie, F2 bearbeitet sie, Leertaste öffnet ihre Verknüpfungen. C erstellt eine Karte.
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
notes-highlighted-at = Hervorgehoben, { $name }, bei { $pct } Prozent: { $text }
notes-highlighted = Hervorgehoben, { $name }: { $text }
# An item in the highlights list. $name is the highlight's palette name;
# $lost is yes when the text was not found after the file changed.
notes-highlight-item =
    { $lost ->
        [yes] { $name }: { $text }, Zeile { $line }, nach der Dateiänderung nicht gefunden
       *[no] { $name }: { $text }, Zeile { $line }
    }
notes-no-highlights = Keine Hervorhebungen. Eine erstellen: { $key }.
notes-highlights-title = Hervorhebungen
notes-highlights-intro =
    { $n ->
        [one] Hervorhebungen, 1 Eintrag. Eingabetaste springt zu einer, Entf entfernt sie, F2 ändert ihren Namen, Leertaste zeigt nur diesen Namen. C erstellt eine Karte.
       *[other] Hervorhebungen, { $n } Einträge. Eingabetaste springt zu einer, Entf entfernt sie, F2 ändert ihren Namen, Leertaste zeigt nur diesen Namen. C erstellt eine Karte.
    }
# The label said before a highlight's text on jumping to it.
notes-highlight-label = Hervorhebung, { $name }
notes-highlight-changed = Hervorhebung geändert zu { $name }: { $text }
# $n is the palette entry's number, $key the keys of Highlight with a name.
notes-palette-no-entry = Kein Hervorhebungsname { $n } in der Palette. Einen Namen wählen: { $key }.
notes-highlights-named-title = Hervorhebungen: { $name }
notes-highlights-named-intro =
    { $n ->
        [one] Hervorhebungen namens { $name }, 1 Eintrag. Leertaste zeigt alle Hervorhebungen.
       *[other] Hervorhebungen namens { $name }, { $n } Einträge. Leertaste zeigt alle Hervorhebungen.
    }
notes-palette-title = Hervorhebungsnamen
# A palette name: the name, how many highlights have it, its shape and its color.
notes-palette-row =
    { $count ->
        [one] { $name }, 1 Hervorhebung, { $shape }, { $color }
       *[other] { $name }, { $count } Hervorhebungen, { $shape }, { $color }
    }
notes-palette-intro-highlight = Mit welchem Namen hervorheben? { $n } Namen. Eingabetaste wählt einen.
notes-palette-intro-change = Die Hervorhebung zu welchem Namen ändern? { $n } Namen. Eingabetaste wählt einen.
notes-palette-intro-collect = Die Hervorhebungen welches Namens sammeln? { $n } Namen. Eingabetaste schreibt sie als Liste.
notes-collect-none = Keine Hervorhebungen namens { $name }.
# The collected list's own title (Markdown).
notes-collect-title = Hervorhebungen namens { $name }: { $title }
notes-collect-line = (Zeile { $line })
# Keep the letters y and n: they are the keys that answer.
notes-collect-saved =
    { $n ->
        [one] 1 Hervorhebung namens { $name } gespeichert als { $file }. Öffnen? y oder n. In { $folder }.
       *[other] { $n } Hervorhebungen namens { $name } gespeichert als { $file }. Öffnen? y oder n. In { $folder }.
    }
palette-shape-underline = Unterstreichung
palette-shape-double-underline = doppelte Unterstreichung
palette-shape-bold = fett
palette-shape-dotted = gepunktete Unterstreichung
palette-shape-brackets = Klammern
palette-shape-symbol = Symbol
# Shown while reading reaches a note's passage.
notes-signal = Notiz: { $text }
# Said after moving onto a note's passage.
notes-has-note = Hat eine Notiz: { $text }

## Relations between notes (the knowledge graph as lists).

relations-type-conflicts-with = steht im Konflikt mit
relations-type-supports = stützt
relations-type-is-example-of = ist ein Beispiel für
relations-type-cites = zitiert
relations-type-contradicts = widerspricht
relations-type-defines = definiert
relations-type-extends = erweitert
relations-type-see-also = siehe auch
relations-type-precedes = geht voraus
relations-type-follows = folgt auf
relations-count = Verknüpfungen: { $out } ausgehend, { $in } eingehend.
relations-note-title = Verknüpfungen von: { $note }
relations-title-filtered = { $title }, Filter: { $filter }
relations-note-intro = Verknüpfungen von { $note }: { $out } ausgehend, { $in } eingehend. Eingabetaste folgt einer Verknüpfung, F2 ändert sie, Entf entfernt sie. Tippen filtert nach Art.
relations-out-item = { $type }: { $target }
relations-target-in = { $note }, in { $doc }
relations-target-missing = eine nicht gefundene Notiz
relations-empty-note = Leere Notiz
relations-incoming-row =
    { $n ->
        [0] Was hierher verweist: noch nichts
        [one] Was hierher verweist: 1 Notiz
       *[other] Was hierher verweist: { $n } Notizen
    }
relations-add-row = Verknüpfung hinzufügen
relations-backlinks-title = Was hierher verweist: { $note }
relations-backlinks-intro =
    { $n ->
        [one] Was auf { $note } verweist: 1 Notiz. Eingabetaste springt dorthin. Tippen filtert nach Art.
       *[other] Was auf { $note } verweist: { $n } Notizen. Eingabetaste springt zu einer. Tippen filtert nach Art.
    }
relations-backlink-item = { $type } diese, von: { $note }
relations-none-in = Noch verweist nichts auf diese Notiz.
relations-types-title = Art der Verknüpfung für: { $note }
relations-types-intro = Wählen Sie die Art der Verknüpfung, 10 Arten. Tippen filtert.
relations-targets-title = { $type }: welche Notiz?
relations-targets-intro =
    { $n ->
        [0] Keine andere Notiz hier. Wählen Sie eine Notiz in einem anderen Dokument.
        [one] Wählen Sie die Zielnotiz: 1 Notiz. Eingabetaste verknüpft sie.
       *[other] Wählen Sie die Zielnotiz: { $n } Notizen. Eingabetaste verknüpft sie.
    }
relations-other-document-row = Eine Notiz in einem anderen Dokument
relations-documents-title = Dokumente mit Notizen
relations-documents-intro = Dokumente mit Notizen: { $n }. Eingabetaste listet die Notizen eines Dokuments.
relations-document-item =
    { $n ->
        [one] { $title }, 1 Notiz
       *[other] { $title }, { $n } Notizen
    }
relations-no-other-documents = Kein anderes Dokument der Bibliothek hat Notizen.
relations-linked = Verknüpft: { $type } { $target }.
relations-changed = Verknüpfung geändert: { $type } { $target }.
relations-already = Schon verknüpft: { $type } { $target }.
relations-removed = Verknüpfung entfernt: { $type } { $target }.
relations-remove-question = Diese Verknüpfung entfernen? y oder n
relations-nothing-to-remove = Hier lässt sich nur eine Verknüpfung entfernen.
relations-note-gone = Notiz nicht gefunden: Sie wurde gelöscht, oder ihr Dokument fehlt.
relations-document-missing = Dokument nicht gefunden: { $file }.
relations-no-note-here = Hier ist keine Notiz. Verknüpfungen gehören zu Notizen; eine hinzufügen: { $key }.
relations-filter-cleared = Filter gelöscht, { $n } angezeigt.
relations-filter-none = Nichts passt zu { $filter }.
relations-filter-matched = Filter { $filter }: { $n } angezeigt.

## Den Wissensgraphen exportieren (B1-g2).

graph-export-title = Wissensgraph exportieren als
graph-export-intro = Wissensgraph, Verknüpfungen: { $links }. Wählen Sie ein Format; die Markdown-Liste ist der Text zum Lesen.
graph-export-empty = Keine Verknüpfungen zwischen Notizen zum Exportieren. Fügen Sie eine in der Verknüpfungsliste einer Notiz hinzu.
graph-format-md = Markdown-Liste, der Text zum Lesen
graph-format-json = JSON, für Gephi und Cytoscape
graph-format-dot = DOT, für Graphviz
graph-format-graphml = GraphML, für Gephi, Cytoscape und yEd
graph-format-mermaid = Mermaid-Diagramm
graph-format-plantuml = PlantUML-Diagramm
graph-format-csv = CSV-Kantenliste, für Tabellenkalkulationen
graph-export-saved = Wissensgraph gespeichert als { $file }. Öffnen? y oder n. In { $folder }.
graph-export-failed = Der Wissensgraph konnte nicht geschrieben werden: { $error } Prüfen Sie, ob in den Ordner geschrieben werden kann.

## Bookmarks: rename and delete.

notes-choose-bookmark-delete = Wählen Sie ein Lesezeichen und drücken Sie Entf.
notes-choose-bookmark-rename = Wählen Sie ein Lesezeichen und drücken Sie F2, um es umzubenennen.
notes-bookmark-deleted = Lesezeichen { $name } gelöscht.
notes-renaming-bookmark = Lesezeichen { $name } wird umbenannt.
notes-bookmark-unchanged = Lesezeichen unverändert.
# $name is the name asked for, $old the bookmark's name.
notes-bookmark-name-taken = Es gibt bereits ein Lesezeichen namens { $name }. Lesezeichen { $old } unverändert.
notes-bookmark-renamed = Lesezeichen { $old } in { $name } umbenannt.

## Find, bookmarks, and selection.

marks-cannot-search = Suche nicht möglich: { $error } Prüfen Sie das Muster zwischen den Schrägstrichen.
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
common-bookmark-set = Lesezeichen { $name } gesetzt bei { $pct } Prozent.
marks-no-bookmarks = Keine Lesezeichen. Eines hinzufügen: { $key }.
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
