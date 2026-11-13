### Edit mode: typing, formatting, saving, recovery, and templates.

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
edit-save-failed = Konnte nicht speichern: { $error } Noch in Bearbeitung. Versuchen Sie Speichern unter.
# The Save As prompt; $path is the suggested file.
edit-save-as-label = Speichern unter, Eingabetaste für { $path }
# $name is a file name. Keep the letters y and n.
edit-file-exists-question = { $name } existiert bereits. Ersetzen? y oder n
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
edit-insert-failed = Konnte nicht einfügen: { $error } Der Text ist unverändert.
# $start is the first words of the pasted text.
edit-pasted-lines =
    { $n ->
        [one] 1 Zeile eingefügt: { $start }
       *[other] { $n } Zeilen eingefügt: { $start }
    }
# $start and $end are character positions, $len the text's length.
edit-range-out-of-text = Zeichen { $start } bis { $end } können nicht geändert werden: der Text hat { $len }.
edit-change-failed = Der Text konnte nicht geändert werden: { $error } Der Text ist unverändert.
edit-delete-failed = Konnte nicht löschen: { $error } Der Text ist unverändert.
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
        [horizontal-rule] Horizontale Linie entfernt.
        [table-row] Tabellenzeile entfernt.
        [heading-level] Überschriftsebene { $level } entfernt.
        [table] Tabelle entfernt.
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
        [horizontal-rule] Horizontale Linie: nichts geändert.
        [table-row] Tabellenzeile: nichts geändert.
        [heading-level] Überschriftsebene { $level }: nichts geändert.
        [table] Tabelle: nichts geändert.
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
edit-image-failed = Das Bild konnte nicht eingefügt werden: { $error } Der Text ist unverändert.
# Edit mode's Obsidian aids: a wiki link, tasks, callouts.
edit-wikilink-title = Link zu einer Notiz
edit-wikilink-intro =
    { $n ->
        [one] Link zu einer Notiz, 1 Notiz. Die Eingabetaste wählt sie.
       *[other] Link zu einer Notiz, { $n } Notizen. Die Eingabetaste wählt eine.
    }
edit-wikilink-no-notes = In diesem Vault gibt es keine Notizen, auf die verlinkt werden kann.
edit-wikilink-anchor-title = Link zu einem Teil von { $note }
edit-wikilink-anchor-intro = Link zur ganzen Notiz { $note }, oder zu einer Überschrift oder einem Block darin. Die Eingabetaste wählt.
edit-wikilink-whole = Die ganze Notiz
edit-wikilink-heading = Überschrift { $level }: { $text }
edit-wikilink-block = Block { $id }: { $text }
edit-wikilink-inserted = Link zu { $target } eingefügt.
edit-callout-title = Callout-Typ
edit-callout-intro = Callout-Typen, { $n }. Die Eingabetaste fügt einen ein.
edit-callout-inserted = Callout vom Typ { $type } eingefügt. Geben Sie seinen Text ein.
edit-task-status =
    { $status ->
        [done] Abgehakt.
        [open] Nicht abgehakt.
        [in-progress] In Arbeit.
        [cancelled] Abgebrochen.
        [forwarded] Weitergeleitet.
        [important] Wichtig.
        [question] Frage.
       *[other] Markiert mit { $mark }.
    }
edit-task-new = Neue Aufgabe, nicht abgehakt.
edit-tasks-changed =
    { $n ->
        [one] Aufgabe in 1 Zeile geändert.
       *[other] Aufgaben in { $n } Zeilen geändert.
    }
# $query is the text to find.
common-no-matches = Keine Treffer für { $query }.
# $n matches of $query were found; the replacement is asked next.
edit-replace-with =
    { $n ->
        [one] 1 Treffer für { $query }. Ersetzen durch?
       *[other] { $n } Treffer für { $query }. Ersetzen durch?
    }

## Edit mode: autosave and recovering unsaved work.

common-recovery-write-failed = Die Wiederherstellungskopie konnte nicht geschrieben werden: { $error }. Speichern Sie bald; { -brand } versucht es weiter.
common-recovery-writing-again = Die Wiederherstellungskopie wird erneut geschrieben.
# $title is the document; $when is how long ago its work was saved.
edit-recovery-title = Ungespeicherte Arbeit in { $title } wiederherstellen?
edit-recovery-yes = Ja, { $title } wiederherstellen und weiter bearbeiten
edit-recovery-no = Nein, die ungespeicherten Änderungen verwerfen
edit-recovery-discarded = Die ungespeicherten Änderungen an { $title } verworfen.
edit-recovered = Ungespeicherte Arbeit in { $title } wiederhergestellt. Denken Sie ans Speichern.
edit-recovery-postponed = Wiederherstellung verschoben. Die ungespeicherte Arbeit wird beim nächsten Mal wieder angeboten.

## Saving in the background.

writes-still-saving = Speichert noch. Bitte warten.
writes-not-written-in-time = Einige Änderungen konnten nicht rechtzeitig geschrieben werden: die Festplatte antwortet nicht.
# $error is the system's reason.
writes-save-failed = Konnte nicht speichern: { $error }. Noch in Bearbeitung.
# $name is the bookmark's name, $pct where it is.
writes-bookmark-not-saved = Lesezeichen { $name } ist vorerst gesetzt, konnte aber nicht gespeichert werden: { $error } Prüfen Sie, ob in den Datenordner geschrieben werden kann.
# $name is the saved file's name.
writes-saved = { $name } gespeichert. Noch in Bearbeitung.

## Files changed on disk. $name is a file name. Keep the letters y and
## n: they are the keys that answer.

disk-replace-question = { $name } existiert bereits. Ersetzen? y oder n
# A prompt label, also said with a full stop after it.
disk-not-replaced = Nicht ersetzt. Geben Sie einen anderen Namen ein
# $key is the key for Save As.
disk-not-saved = Nicht gespeichert. Noch in Bearbeitung. Speichern unter, { $key }, behält beide Versionen.
disk-kept-open-version = Die geöffnete Version wurde beibehalten.
disk-overwrite-question = { $name } wurde auf der Festplatte geändert, seit Sie es geöffnet haben. Über diese Änderungen speichern? y oder n
disk-reload-question = { $name } wurde auf der Festplatte geändert. Neu laden? y oder n

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
