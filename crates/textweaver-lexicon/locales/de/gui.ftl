### The textweaver app: drawn labels, hints, buttons, and dialogs.

## The window (GUI)

gui-open-title = Ein Dokument öffnen
gui-open-documents = Dokumente, die textweaver liest
gui-open-all-files = Alle Dateien
gui-open-no-dialog = Die Dateiauswahl des Systems hat sich nicht geöffnet. Geben Sie stattdessen den Pfad des Dokuments ein.
gui-text-size = Textgröße { $size } Punkt.
gui-text-size-largest = Textgröße { $size } Punkt, die größte.
gui-text-size-smallest = Textgröße { $size } Punkt, die kleinste.
gui-font = Schriftart: { $family }.
gui-font-list = Schriftart

## The window (GUI): drawn labels, hints, and questions.
## Keep the letters Y and N: they are the keys that answer.

gui-yes = Ja
gui-no = Nein
gui-answer-delete = Löschen
gui-answer-remove = Entfernen
gui-answer-replace = Ersetzen
gui-question-hint = Y antwortet ja, N antwortet nein, Escape antwortet nein.
gui-find-title = Suchen und ersetzen
gui-find-what = Suchen nach
gui-find-with = Ersetzen durch
gui-find-match-case = Groß-/Kleinschreibung
gui-find-whole-words = Ganze Wörter
gui-find-regex = Regulärer Ausdruck
gui-find-across-lines = Über Zeilen hinweg
gui-find-next = Weitersuchen
gui-find-next-help = Wählt den nächsten Treffer aus.
gui-find-replace = Ersetzen
gui-find-replace-help = Ersetzt den gezeigten Treffer und geht zum nächsten. Der erste Druck sucht einen Treffer.
gui-find-replace-all = Alle ersetzen
gui-find-replace-all-help = Sagt, wie viele Treffer es gibt, und fragt einmal. Ein Rückgängig nimmt alle zurück.
gui-find-hint = Enter in „Suchen nach“ sucht den nächsten Treffer, Enter in „Ersetzen durch“ ersetzt ihn. Pfeil nach oben holt frühere Texte zurück. Escape schließt.
gui-find-empty = Nichts zu suchen: Text in „Suchen nach“ eingeben.
gui-button-open = Öffnen…
gui-button-font = Schriftart…
gui-button-edit = Bearbeiten
gui-button-finish-editing = Bearbeitung fertig
gui-button-settings = Einstellungen…
gui-button-commands = Befehle…
gui-button-play = Abspielen
gui-button-pause = Pause
gui-button-stop = Stopp
gui-button-previous-sentence = Vorheriger Satz
gui-button-next-sentence = Nächster Satz
gui-button-slower = Langsamer
gui-button-faster = Schneller
gui-button-start-dictation = Diktat starten
gui-button-pause-dictation = Diktat anhalten
gui-button-resume-dictation = Diktat fortsetzen
gui-button-stop-dictation = Diktat beenden
gui-button-live-pause = Aktualisierung anhalten
gui-button-live-resume = Aktualisierung fortsetzen
gui-button-live-newest = Zum neuesten
gui-button-live-table = Tabellenansicht
gui-button-live-list = Listenansicht
gui-button-live-save = Protokoll speichern
gui-button-live-close = Quelle schließen
gui-button-live-show = Änderungen zeigen
gui-button-live-show-on = Änderungen zeigen: ein
gui-button-live-show-off = Änderungen zeigen: aus
gui-button-live-speak = Änderungen sprechen
gui-button-live-speak-on = Änderungen sprechen: ein
gui-button-live-speak-off = Änderungen sprechen: aus
gui-button-unavailable = nicht verfügbar
gui-hint-open = Ein Dokument zum Lesen wählen.
gui-hint-font = Die Schrift des Dokumenttexts wählen.
gui-hint-edit = Bearbeiten ein oder aus.
gui-hint-settings = Jede Option, mit ihrer Hilfe.
gui-hint-commands = Jeden Befehl per Namen ausführen.
gui-hint-play = Vorlesen ab dem Wort, oder Pause.
gui-hint-live-pause = Neue Einträge warten bis zum Fortsetzen.
gui-hint-live-view = Ihre Stelle im Protokoll bleibt.
gui-hint-live-save = Das Protokoll jetzt als JSON speichern.
gui-hint-live-close = Empfang beenden; das Protokoll bleibt.
gui-hint-live-field = Für das Feld in der Zeile des Cursors.
gui-button-close = Schließen
gui-toolbar-reading = Lesen
gui-document = Dokument
gui-document-titled = { $title }, Dokument
gui-list-hint = Eingabetaste wählt, Escape schließt.
gui-sidebar-contents = Inhalt
gui-sidebar-notes = Notizen
gui-sidebar-open =
    { $n ->
        [one] { $panel } offen, 1 Eintrag.
       *[other] { $panel } offen, { $n } Einträge.
    }
gui-sidebar-closed = { $panel } geschlossen.
gui-header-shown = Kopfzeile angezeigt.
gui-header-hidden = Kopfzeile ausgeblendet. Ihre Befehle behalten ihre Tasten.
gui-toolbar-shown = Symbolleiste angezeigt.
gui-toolbar-hidden = Symbolleiste ausgeblendet. Ihre Befehle behalten ihre Tasten.
gui-focus-mode-on = Fokusmodus an. Eine Mausbewegung zeigt die Leisten.
gui-focus-mode-off = Fokusmodus aus.
gui-full-screen-on = Vollbild an.
gui-full-screen-off = Vollbild aus.

## Schaltflächen anpassen (crate::buttons).

buttons-title = Schaltflächen anpassen
buttons-intro =
    { $n ->
        [one] Schaltflächen anpassen, 1 Schaltfläche. Enter auf einer Schaltfläche, um sie zu verschieben oder zu entfernen.
       *[other] Schaltflächen anpassen, { $n } Schaltflächen. Enter auf einer Schaltfläche, um sie zu verschieben oder zu entfernen.
    }
buttons-row =
    { $bar ->
        [header] { $name }, Kopfzeile { $pos } von { $count }
       *[toolbar] { $name }, Symbolleiste { $pos } von { $count }
    }
buttons-add-header = Schaltfläche zur Kopfzeile hinzufügen
buttons-add-toolbar = Schaltfläche zur Symbolleiste hinzufügen
buttons-reset = Beide Leisten auf die Standardwerte zurücksetzen
buttons-move-up = Nach oben
buttons-move-down = Nach unten
buttons-remove = Entfernen
buttons-add-intro =
    { $n ->
        [one] 1 Befehl zum Hinzufügen. Enter fügt ihn am Ende hinzu.
       *[other] { $n } Befehle zum Hinzufügen. Enter fügt einen am Ende hinzu.
    }
buttons-added =
    { $bar ->
        [header] { $name } hinzugefügt, Kopfzeile { $pos } von { $count }.
       *[toolbar] { $name } hinzugefügt, Symbolleiste { $pos } von { $count }.
    }
buttons-removed =
    { $bar ->
        [header] { $name } aus der Kopfzeile entfernt.
       *[toolbar] { $name } aus der Symbolleiste entfernt.
    }
buttons-moved = { $name } verschoben, { $pos } von { $count }.
buttons-first = { $name } ist schon die erste.
buttons-last = { $name } ist schon die letzte.
buttons-already =
    { $bar ->
        [header] { $name } ist schon in der Kopfzeile.
       *[toolbar] { $name } ist schon in der Symbolleiste.
    }
buttons-no-name = Dieser Befehl hat keinen Kurznamen und kann keine Schaltfläche sein.
buttons-no-button = Dort ist keine Schaltfläche.
buttons-reset-done = Schaltflächen auf die Standardwerte zurückgesetzt.
buttons-none-to-add = Keine weiteren Befehle zum Hinzufügen.
# The preview pane beside the editor (edit mode). $key is the key that moves between the window's parts (F6).
gui-preview = Vorschau
gui-preview-shown = Vorschau neben dem Editor eingeblendet. { $key } wechselt dorthin.
gui-preview-hidden = Vorschau ausgeblendet.
gui-preview-later = Vorschau an. Sie erscheint im Bearbeitungsmodus neben dem Editor.
gui-preview-empty = Noch nichts in der Vorschau.
gui-sidebar-no-headings = Keine Überschriften.
gui-sidebar-no-notes = Keine Notizen.
gui-sidebar-current = { $item }, aktuell
gui-sidebar-hint = Eingabetaste springt hin. { $leave } springt und kehrt zurück. Escape kehrt zurück.
gui-sidebar-hint-notes = Eingabetaste springt hin. Leertaste zeigt ihre Verknüpfungen. { $leave } springt und kehrt zurück. Escape kehrt zurück.
gui-sidebar-notes-keys = Leertaste zeigt die Verknüpfungen einer Notiz.
gui-settings-sections = Bereiche
gui-settings-form = Einstellungen: { $section }
gui-settings-saved-hint = Änderungen wirken sofort und werden sofort gespeichert.
gui-settings-close-help = Die Einstellungen schließen. Jede Änderung ist bereits gespeichert.
gui-settings-recent = Zuletzt geändert
gui-settings-matching = Passend zu { $filter }
gui-settings-table = { $label } ist eine Tabelle. Bearbeiten Sie sie in settings.toml.
gui-setting-new-value = Neuer Wert für { $label }
gui-setting-value-hint = Eingabetaste übernimmt, Escape geht zurück.
gui-prompt-path-hint = Geben Sie den Pfad eines Dokuments ein und drücken Sie die Eingabetaste. Tab vervollständigt ihn; Pfeil nach oben und unten holen frühere zurück.
gui-prompt-hint = Eingabetaste übernimmt, Escape bricht ab. Pfeil nach oben und unten holen frühere Antworten zurück.
gui-palette-filter = Tippen, um die Befehle zu filtern
gui-palette-list = Befehle
gui-palette-hint = Eingabetaste führt den ersten Treffer aus; Tab wechselt zur Liste; F1 erklärt einen Befehl.
gui-open-failed = { $name } konnte nicht geöffnet werden: { $error }
gui-uia-unavailable = UI-Automation-Benachrichtigungen gibt es nur unter Windows; die Live-Region wird verwendet.
gui-crashed = textweaver wurde nach einem internen Fehler beendet.
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
gui-settings-form-help = Pfeil nach oben und unten wechseln zwischen Einstellungen. Pfeil nach links und rechts ändern eine. Die Eingabetaste gibt einen neuen Wert ein. Entf stellt den Standard wieder her. { $next } und { $previous } wechseln den Bereich. Tippen filtert. F1 sagt die Hilfe.
gui-settings-press-enter = Drücken Sie die Eingabetaste, um einen neuen Wert für { $label } einzugeben.
gui-font-built-in = { $family } (eingebaut)

## The window's menus and dialogs. Settings files chosen with the
## system's file chooser, the Colors dialog, and the font list. $ratio is
## a contrast ratio such as 4.8; $verdict is good, fair, or low.
gui-settings-files = Einstellungsdateien
gui-settings-export-title = Einstellungen exportieren
gui-settings-import-title = Einstellungen importieren
gui-chooser-no-dialog = Die Dateiauswahl des Systems hat sich nicht geöffnet. Geben Sie stattdessen den Pfad der Datei ein.
gui-colors-value = { $value }, Kontrast { $ratio } zu 1, { $verdict }
gui-colors-help = Links und Rechts wählen eine benannte Farbe, Blau und Orange zuerst. Eingabe tippt einen Namen oder einen #rrggbb-Wert. Entf stellt die Farbe des Designs wieder her. Jede Markierung behält ihre Unterstreichung, Stärke oder ihr Symbol, welche Farbe sie auch hat.
gui-colors-value-system = { $value }; nicht gezeichnet, solange die Kontrastfarben des Systems an sind
gui-colors-help-system = Die Kontrastfarben des Systems sind an und werden statt dieser Farben gezeichnet; diese gelten wieder, wenn sie aus sind. Jeder Hervorhebungsname behält seine eigene Form.
gui-colors-reset-all = Alle Farben zurücksetzen
gui-colors-reset-all-help = Für jeden Teil die Farbe des Designs wiederherstellen.
gui-colors-reset-done = Alle Farben sind wieder die des Designs.
colors-reset-question = Jede Farbe auf die des Designs zurücksetzen? y oder n
gui-colors-closed = Farben geschlossen.
gui-font-list-intro =
    { $n ->
        [one] { $title }, 1 Schriftfamilie.
       *[other] { $title }, { $n } Schriftfamilien.
    }

## W9b-x: die Leseeinstellungen (Ansicht, Leseeinstellungen).
reading-form-intro = Leseeinstellungen, { $n } Einstellungen. Links und Rechts ändern einen Wert, Eingabe tippt einen, Entf stellt den Standard wieder her, F1 sagt die Hilfe.
reading-form-spacing-wcag-done = Abstände auf die WCAG-Werte gesetzt.
reading-form-spacing-generous-done = Abstände auf Großzügig gesetzt, weiter als WCAG.
spacing-letter-without-word = Wortabstand mit dem Zeichenabstand erhöhen.
gui-reading-form-help = Auf und Ab bewegen, Links und Rechts ändern einen Wert, Eingabe tippt einen, F1 sagt die Hilfe.
gui-reading-voices = Stimmen
gui-reading-voices-help = Die Stimmenverwaltung öffnen.
gui-reading-wcag = WCAG-Abstände
gui-reading-wcag-help = Die vier Abstände auf die WCAG-Werte setzen.
gui-reading-generous = Großzügige Abstände
gui-reading-generous-help = Die vier Abstände weiter als WCAG setzen.
gui-reading-closed = Leseeinstellungen geschlossen.
## End of W9b-x
