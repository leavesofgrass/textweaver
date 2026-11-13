### The settings screen, profiles, units, sections, and settings files.

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
profiles-read-failed = Die Profildatei konnte nicht gelesen werden, daher wird sie als leer behandelt: { $error } Beim Speichern eines Profils wird die Datei ersetzt.
profiles-save-failed = Die Profile konnten nicht gespeichert werden: { $error } Prüfen Sie, ob in den Einstellungsordner geschrieben werden kann.
profiles-none-to-export = Es gibt noch keine Profile zu exportieren.
profiles-exported =
    { $n ->
        [one] 1 Profil nach { $file } exportiert.
       *[other] { $n } Profile nach { $file } exportiert.
    }
profiles-export-failed = Die Profile konnten nicht exportiert werden: { $error } Prüfen Sie, ob in den Ordner geschrieben werden kann.
profiles-imported =
    { $n ->
        [0] Es waren keine Profile in { $file }.
        [one] 1 Profil aus { $file } importiert: { $names }.
       *[other] { $n } Profile aus { $file } importiert: { $names }.
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
settingsio-export-failed = Einstellungen konnten nicht exportiert werden: { $error } Prüfen Sie, ob in den Ordner geschrieben werden kann.
# $path is the file; $error the system's reason.
settingsio-read-failed = { $path } konnte nicht gelesen werden: { $error } Prüfen Sie die Datei und importieren Sie erneut.
settingsio-nothing-to-import = Nichts zu importieren: Ihre Einstellungen stimmen bereits mit dieser Datei überein.
settingsio-cancelled-unchanged = Abgebrochen. Nichts wurde geändert.
settingsio-import-failed = Einstellungen konnten nicht importiert werden: { $error } Prüfen Sie, ob in den Einstellungsordner geschrieben werden kann.
# $summary lists what changed (from the settings store, in English).
settingsio-imported = Einstellungen importiert. { $summary }
settingsio-backend-next-start = Die neue Sprachausgabe wird ab dem nächsten Start verwendet.
settingsio-backed-up = Die alten Einstellungen wurden gesichert.

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
settings-cannot-be = { $label } kann das nicht sein: { $error } Wählen Sie einen anderen Wert.
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

## Units, said after a number.

settings-unit-words-per-minute =
    { $n ->
        [one] Wort pro Minute
       *[other] Wörter pro Minute
    }
settings-unit-hertz =
    { $n ->
        [one] Hertz
       *[other] Hertz
    }
settings-unit-percent = Prozent
settings-unit-semitones =
    { $n ->
        [one] Halbton
       *[other] Halbtöne
    }
settings-unit-milliseconds =
    { $n ->
        [one] Millisekunde
       *[other] Millisekunden
    }
settings-unit-words =
    { $n ->
        [one] Wort
       *[other] Wörter
    }
settings-unit-places =
    { $n ->
        [one] Stelle
       *[other] Stellen
    }
settings-unit-columns =
    { $n ->
        [one] Spalte
       *[other] Spalten
    }
settings-unit-groups =
    { $n ->
        [one] Gruppe
       *[other] Gruppen
    }
settings-unit-characters =
    { $n ->
        [one] Zeichen
       *[other] Zeichen
    }
settings-unit-lines =
    { $n ->
        [one] Zeile
       *[other] Zeilen
    }
settings-unit-seconds =
    { $n ->
        [one] Sekunde
       *[other] Sekunden
    }
settings-unit-steps =
    { $n ->
        [one] Schritt
       *[other] Schritte
    }
settings-unit-megabytes = Megabyte
settings-unit-files =
    { $n ->
        [one] Datei
       *[other] Dateien
    }
settings-unit-letters = Buchstaben
settings-unit-points = Punkt
settings-unit-rows =
    { $n ->
        [one] Zeile
       *[other] Zeilen
    }
settings-unit-minutes =
    { $n ->
        [one] Minute
       *[other] Minuten
    }

## Settings sections.

section-highlight = Hervorheben
section-reading = Lesen
section-display = Anzeige
section-editing = Bearbeiten
section-library = Bibliothek und Lernwerkzeuge
section-keyboard = Tastatur und Sprache
section-accessibility = Zugänglichkeit und Ansagen
section-export = Exportieren
section-braille = Braille

## Settings by task (beta 4): sections, and when a setting applies.

section-voice = Stimme und Sprache
section-pauses = Pausen und Ausführlichkeit
section-speaking-text = Text sprechen
section-documents = Dokumente
section-math = Mathematik
section-morse = Morsecode
section-morse-practice = Morse-Übung
section-spacing = Abstände, Schrift und Lineal
section-word-aids = Worthilfen
section-rsvp = Wort für Wort (RSVP)
section-window = Symbolleisten und Bereiche
# $n is the number of sections.
settings-intro-sections =
    { $n ->
        [one] Einstellungen, 1 Bereich. Eingabetaste öffnet einen Bereich. Tippen durchsucht alle Einstellungen. Escape schließt.
       *[other] Einstellungen, { $n } Bereiche. Eingabetaste öffnet einen Bereich. Tippen durchsucht alle Einstellungen. Escape schließt.
    }
# $section is a section-* title, $n its number of settings.
settings-section-row =
    { $n ->
        [one] { $section }, 1 Einstellung
       *[other] { $section }, { $n } Einstellungen
    }
settings-all = Alle Einstellungen
settings-all-row =
    { $n ->
        [one] Alle Einstellungen, 1 Einstellung
       *[other] Alle Einstellungen, { $n } Einstellungen
    }
settings-title-section = Einstellungen: { $section }
settings-section-opened =
    { $n ->
        [one] { $section }, 1 Einstellung. Links und Rechts ändern einen Wert, Eingabetaste ändert oder gibt einen ein, Entf setzt den Standard zurück, Escape geht zurück zu den Bereichen.
       *[other] { $section }, { $n } Einstellungen. Links und Rechts ändern einen Wert, Eingabetaste ändert oder gibt einen ein, Entf setzt den Standard zurück, Escape geht zurück zu den Bereichen.
    }
settings-all-opened =
    { $n ->
        [one] Alle Einstellungen, 1 Einstellung, auch die, die gerade nicht gelten. Escape geht zurück zu den Bereichen.
       *[other] Alle Einstellungen, { $n } Einstellungen, auch die, die gerade nicht gelten. Escape geht zurück zu den Bereichen.
    }
settings-back = Bereiche der Einstellungen.
settings-section-now =
    { $n ->
        [one] { $section } zeigt jetzt 1 Einstellung.
       *[other] { $section } zeigt jetzt { $n } Einstellungen.
    }
settings-section-help =
    { $n ->
        [one] { $section }, 1 Einstellung. Eingabetaste öffnet ihn.
       *[other] { $section }, { $n } Einstellungen. Eingabetaste öffnet ihn.
    }
# $item is a settings-item row, $why a settings-applies-* sentence.
settings-item-not-now = { $item }. { $why }
# $engine, $first and $second are speech engine names, such as Eloquence.
settings-applies-engine = Gilt, wenn { $engine } die Sprachausgabe ist.
settings-applies-engines = Gilt, wenn { $first } oder { $second } die Sprachausgabe ist.
settings-applies-not-engine = Gilt, wenn { $engine } nicht die Sprachausgabe ist.
# $os is Windows, macOS or Linux.
settings-applies-os = Gilt nur unter { $os }.
