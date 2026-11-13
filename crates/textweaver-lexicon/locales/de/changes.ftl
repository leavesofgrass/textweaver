### Tracked changes and comments in Word files.

## B1-t1: Nachverfolgte Änderungen und Kommentare (die Änderungsliste).
prompt-comment-reply = Antwort
prompt-comment-text = Kommentar
prompt-github-token = GitHub-Token für die Komponentenquelle, Eingabe überspringt
prompt-import-cards = Lernkarten aus Anki-Paket importieren
prompt-cw-copy = Schreiben Sie, was Sie gehört haben
changes-title = Änderungen und Kommentare
changes-intro =
    { $n ->
        [one] { $title }, { $n } Eintrag. Eingabe geht hin. A nimmt eine Änderung an, R lehnt sie ab, mit Umschalt jede Änderung derselben Person. Bei einem Kommentar antwortet F2, Leertaste erledigt, Entf löscht. N fügt einen Kommentar hinzu.
       *[other] { $title }, { $n } Einträge. Eingabe geht hin. A nimmt eine Änderung an, R lehnt sie ab, mit Umschalt jede Änderung derselben Person. Bei einem Kommentar antwortet F2, Leertaste erledigt, Entf löscht. N fügt einen Kommentar hinzu.
    }
changes-none = Keine nachverfolgten Änderungen oder Kommentare in diesem Dokument.
changes-row = { $kind }: „{ $text }“, { $who }, { $when }
changes-kind-inserted = Eingefügt
changes-kind-deleted = Gelöscht
changes-kind-moved-away = Wegverschoben
changes-kind-moved-here = Hierher verschoben
changes-by = von { $author }
changes-no-author = Autor nicht angegeben
changes-no-date = Datum nicht angegeben
changes-date = { $weekday }, { $day }. { $month } { $year }
changes-weekday-0 = Sonntag
changes-weekday-1 = Montag
changes-weekday-2 = Dienstag
changes-weekday-3 = Mittwoch
changes-weekday-4 = Donnerstag
changes-weekday-5 = Freitag
changes-weekday-6 = Samstag
changes-month-1 = Januar
changes-month-2 = Februar
changes-month-3 = März
changes-month-4 = April
changes-month-5 = Mai
changes-month-6 = Juni
changes-month-7 = Juli
changes-month-8 = August
changes-month-9 = September
changes-month-10 = Oktober
changes-month-11 = November
changes-month-12 = Dezember
changes-comment = Kommentar: { $text }
changes-comment-by = Kommentar von { $author }: { $text }
changes-replies =
    { $n ->
        [one] { $n } Antwort
       *[other] { $n } Antworten
    }
changes-resolved = erledigt
changes-accepted = Angenommen. { $kind }: „{ $text }“.
changes-rejected = Abgelehnt. { $kind }: „{ $text }“.
changes-accepted-all =
    { $n ->
        [one] { $n } Änderung angenommen.
       *[other] { $n } Änderungen angenommen.
    }
changes-rejected-all =
    { $n ->
        [one] { $n } Änderung abgelehnt.
       *[other] { $n } Änderungen abgelehnt.
    }
changes-accepted-author =
    { $n ->
        [one] { $n } Änderung von { $author } angenommen.
       *[other] { $n } Änderungen von { $author } angenommen.
    }
changes-rejected-author =
    { $n ->
        [one] { $n } Änderung von { $author } abgelehnt.
       *[other] { $n } Änderungen von { $author } abgelehnt.
    }
changes-none-left = Keine nachverfolgten Änderungen zum Annehmen oder Ablehnen.
changes-not-a-change = Diese Zeile ist ein Kommentar. F2 antwortet, Leertaste erledigt, Entf löscht.
changes-not-a-comment = Diese Zeile ist eine Änderung. A nimmt sie an, R lehnt sie ab.
changes-edit-mode = Im Bearbeitungsmodus bleiben Änderungen, wie sie sind. Verlassen Sie den Bearbeitungsmodus, um sie anzunehmen oder abzulehnen.
changes-replied = Antwort hinzugefügt.
changes-resolved-done = Kommentar erledigt.
changes-reopened = Kommentar wieder offen.
changes-comment-deleted = Kommentar und Antworten gelöscht.
changes-comment-added = Kommentar hinzugefügt.
changes-delete-comment-question = Diesen Kommentar und seine Antworten löschen? y oder n
changes-written-accepted =
    { $n ->
        [one] { $path } geschrieben, { $n } Änderung angenommen.
       *[other] { $path } geschrieben, { $n } Änderungen angenommen.
    }
changes-written-rejected =
    { $n ->
        [one] { $path } geschrieben, { $n } Änderung abgelehnt.
       *[other] { $path } geschrieben, { $n } Änderungen abgelehnt.
    }
## End of B1-t1

## B1-t2: die Überprüfung in die Word-Datei speichern.
changes-accept-all-question =
    { $n ->
        [one] Die 1 Änderung annehmen? y oder n
       *[other] Alle { $n } Änderungen annehmen? y oder n
    }
changes-reject-all-question =
    { $n ->
        [one] Die 1 Änderung ablehnen? y oder n
       *[other] Alle { $n } Änderungen ablehnen? y oder n
    }
changes-save-not-word = Keine Word-Datei. Änderungen werden nur in .docx-Dateien zurückgeschrieben; der Export schreibt den entschiedenen Text in andere Formate.
changes-save-nothing = Nichts zu speichern: keine Änderung wurde angenommen oder abgelehnt, und kein Kommentar wurde geändert.
changes-saved-backup = Änderungen in { $file } gespeichert. Das Original bleibt als { $backup } erhalten.
changes-saved = Änderungen in { $file } gespeichert.
changes-save-unplaced =
    { $n ->
        [one] 1 neuer Kommentar steht am Anfang: sein Text wurde nicht gefunden.
       *[other] { $n } neue Kommentare stehen am Anfang: ihr Text wurde nicht gefunden.
    }
changes-save-failed = Die Änderungen in { $file } konnten nicht gespeichert werden: { $error }. Die Datei ist unverändert; schließen Sie sie in Word, falls sie geöffnet ist, und versuchen Sie es erneut.
changes-save-not-in-build = Änderungen in der Word-Datei speichern ist in dieser Version von { -brand } nicht enthalten. tw changes --in-place kann es weiterhin.
changes-in-place-accepted =
    { $n ->
        [one] 1 Änderung in { $path } angenommen. Das Original bleibt als { $backup } erhalten.
       *[other] { $n } Änderungen in { $path } angenommen. Das Original bleibt als { $backup } erhalten.
    }
changes-in-place-rejected =
    { $n ->
        [one] 1 Änderung in { $path } abgelehnt. Das Original bleibt als { $backup } erhalten.
       *[other] { $n } Änderungen in { $path } abgelehnt. Das Original bleibt als { $backup } erhalten.
    }

## B1-t3: edits saved into a Word file as tracked changes.
changes-tracked-saved =
    { $n ->
        [one] 1 nachverfolgte Änderung in { $file } gespeichert.
       *[other] { $n } nachverfolgte Änderungen in { $file } gespeichert.
    }
changes-original-kept = Das Original bleibt als { $backup } erhalten.
changes-tracked-refused =
    { $n ->
        [one] Nicht gespeichert: 1 Änderung in { $file } überschreitet einen Absatz oder liegt in einem Link oder Feld und kann nicht nachverfolgt werden. Speichern unter behält Ihre Bearbeitungen als Markdown.
       *[other] Nicht gespeichert: { $n } Änderungen in { $file } überschreiten einen Absatz oder liegen in einem Link oder Feld und können nicht nachverfolgt werden. Speichern unter behält Ihre Bearbeitungen als Markdown.
    }
## End of B1-t2
