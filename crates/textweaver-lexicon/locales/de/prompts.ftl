### Prompts, questions, lists, and file choosers.

## Prompts.

prompt-define-word = Welches Wort definieren?
prompt-profile-name = Name für das neue Profil
prompt-profile-rename = Neuer Name für das Profil, Eingabetaste behält ihn
prompt-profiles-import = Profile aus Datei importieren
prompt-profiles-export = Profile in Datei exportieren, zum Beispiel textweaver-profiles.json

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
app-window-only = Dieser Befehl funktioniert in der textweaver-App.
app-terminal-only = Dieser Befehl funktioniert im Terminal-Reader.
settings-save-failed = Einstellungen konnten nicht gespeichert werden: { $error } Ihre Änderungen gelten, bis Sie beenden.
settings-outside-kept = Außerhalb von textweaver geänderte Einstellungen wurden beibehalten.
edit-still-editing = Noch in Bearbeitung.
# The reading view inside edit mode (toggle_preview). $key is its key;
# $line the line the caret is back on.
edit-preview-on = Vorschau, schreibgeschützt: die Leseansicht an derselben Stelle. { $key } kehrt zum Bearbeiten zurück.
edit-preview-off = Zurück beim Bearbeiten. { $line }
edit-preview-read-only = Die Vorschau ist schreibgeschützt. { $key } kehrt zum Bearbeiten zurück.
edit-preview-not-editing = Die Vorschau gibt es im Bearbeitungsmodus. { $key } startet das Bearbeiten.
edit-preview-plain = Reiner Text hat keine Formatierung für eine Vorschau; er liest sich gleich.
edit-preview-failed = Die Vorschau konnte nicht angezeigt werden: { $error } Prüfen Sie das Markdown und versuchen Sie es erneut.
goto-not-a-target = Kein Sprungziel: { $text }. Geben Sie eine Zeilennummer ein, einen Prozentwert wie 50%, start oder end.

## Prompts: the label is shown and said when the prompt opens.

prompt-find = Suchen
prompt-go-to = Gehe zu Zeile, Prozent, start oder end
prompt-open = Datei öffnen
prompt-command = Befehl
# $label is prompt-command.
prompt-command-palette-intro = { $label }. Geben Sie einen Teil eines Namens ein; Tabulator vervollständigt, Auf und Ab listen Treffer.
prompt-save-as = Speichern unter
prompt-export-as = Exportieren unter
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
prompt-reference-identifier = DOI, ISBN oder PMID zum Hinzufügen
prompt-import-references = Literaturangaben aus Datei importieren
prompt-template-title = Titel des neuen Dokuments
prompt-setting-value = Neuer Wert, Eingabetaste behält ihn

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
goto-word-start = start
goto-word-end = end

language-voices-loading = Die Stimmenliste wird noch geladen, daher spricht die aktuelle Stimme weiter.

## Datei- und Ordnerauswahl.
prompt-browse-hint = { $label }. { $key } zum Durchsuchen.
prompt-browse-file = Datei wählen: { $label }
prompt-browse-folder = Ordner wählen: { $label }
prompt-browse-filled = { $name } gewählt. Eingabe bestätigt.
chooser-type-files = { $type }-Dateien
chooser-image-title = Bild einfügen
chooser-images = Bilder
chooser-references-title = Literaturangaben importieren
chooser-reference-files = Literaturdateien
chooser-anki-title = Anki-Karten importieren
chooser-anki-files = Anki-Pakete
chooser-profiles-import-title = Profile importieren
chooser-profiles-export-title = Profile exportieren
chooser-profile-files = Profildateien
chooser-live-source-files = Live-Quelldateien
gui-folder-no-dialog = Die Ordnerauswahl des Systems hat sich nicht geöffnet. Wählen Sie den Ordner in dieser Liste.
gui-prompt-browse-hint = { $key } öffnet den Dateibrowser.

## A secret field's Show choice: the show key, or the Show box in
## the textweaver app, shows a password, token or join code as typed.
prompt-show-hint = { $label }. { $key } zeigt es an.
prompt-secret-hidden = { $count ->
        [one] 1 Zeichen, verborgen
       *[other] { $count } Zeichen, verborgen
    }
prompt-show-join-code = Beitrittscode anzeigen
prompt-showing-join-code = Beitrittscode wird angezeigt
prompt-hiding-join-code = Beitrittscode wird verborgen
prompt-show-password = Passwort anzeigen
prompt-showing-password = Passwort wird angezeigt
prompt-hiding-password = Passwort wird verborgen
prompt-show-token = Token anzeigen
prompt-showing-token = Token wird angezeigt
prompt-hiding-token = Token wird verborgen
gui-prompt-show-hint = { $key } oder das Feld Anzeigen zeigt die Eingabe.
