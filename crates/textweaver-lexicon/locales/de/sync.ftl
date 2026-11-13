### Sync between computers.

## Sync wave, S4: sync in the reader (ADR-0049).
sync-status-off = Sync: aus
sync-status-not-set-up = Sync: nicht eingerichtet
sync-status-starting = Sync: startet
sync-status-folder-missing = Sync: Ordner fehlt, speichert hier
sync-status-read-only = Sync: neueres Format, nur lesen
sync-status-failed = Sync: Ordner nicht nutzbar
sync-status-cannot-write = Sync: Schreiben geht nicht, speichert hier
sync-status-clock-ahead = Sync: Uhr von { $device } geht vor
sync-status-damaged =
    { $n ->
        [one] Sync: 1 beschädigte Datei übersprungen
       *[other] Sync: { $n } beschädigte Dateien übersprungen
    }
sync-status-up-to-date = Sync: aktuell
sync-status-this-computer = Dieser Computer: { $name }.
sync-status-no-others = Noch keine anderen Computer.
sync-status-others = Andere Computer: { $names }.
sync-status-error = Problem: { $error } Prüfen Sie den Sync-Ordner.
sync-another-computer = ein anderer Computer
sync-untitled = ein Dokument
sync-damaged = Sync: beschädigte Datei von { $device } übersprungen.
sync-newer-file = Sync: neuere Datei von { $device } übersprungen.
sync-read-only = Sync: neueres Format, nur lesen.
sync-clock-ahead = Sync: Uhr von { $device } geht { $hours } Stunden vor.
sync-fresh-id = Sync: kopierte Einrichtung; neue Computer-ID.
sync-write-failed = Sync: Schreiben fehlgeschlagen. { $error } Vorerst wird auf diesem Computer gespeichert.
sync-name-refused = Name nicht erlaubt. Zum Beispiel: Laptop.
sync-note-replaced =
    { $n ->
        [one] { $title }: eine Notiz wurde durch die neuere Änderung von { $device } ersetzt.
       *[other] { $title }: { $n } Notizen wurden durch neuere Änderungen von { $device } ersetzt.
    }
sync-restored-notes =
    { $n ->
        [one] { $title }: eine gelöschte Notiz ist zurück, bearbeitet auf { $device }.
       *[other] { $title }: { $n } gelöschte Notizen sind zurück, bearbeitet auf { $device }.
    }
sync-restored-bookmarks =
    { $n ->
        [one] { $title }: ein gelöschtes Lesezeichen ist zurück, bearbeitet auf { $device }.
       *[other] { $title }: { $n } gelöschte Lesezeichen sind zurück, bearbeitet auf { $device }.
    }
sync-restored-highlights =
    { $n ->
        [one] { $title }: eine gelöschte Markierung ist zurück, bearbeitet auf { $device }.
       *[other] { $title }: { $n } gelöschte Markierungen sind zurück, bearbeitet auf { $device }.
    }
sync-arrived =
    { $n ->
        [one] { $title }: 1 Änderung von { $device }.
       *[other] { $title }: { $n } Änderungen von { $device }.
    }
sync-resumed = { $title }: fortgesetzt bei { $pct } Prozent, von { $device }.
sync-place-arrived = Stelle von { $device }: { $pct } Prozent.
sync-place-question = { $device } bei { $pct } Prozent. Dorthin? y oder n
sync-suggestion-question =
    { $n ->
        [one] Das ist vielleicht { $title } von { $device }, mit 1 Notiz. Übernehmen? y oder n
       *[other] Das ist vielleicht { $title } von { $device }, mit { $n } Notizen. Übernehmen? y oder n
    }
sync-went-to-place = Stelle von { $device }, { $pct } Prozent.
sync-kept-place = Diese Stelle behalten.
sync-suggestion-accepted = Notizen von { $device } werden verwendet.
sync-suggestion-declined = Getrennt gelassen.
sync-sidecar-differed =
    { $n ->
        [one] Sync: 1 Bibliotheksstelle wich ab.
       *[other] Sync: { $n } Bibliotheksstellen wichen ab.
    }
sync-sidecar-failed = Sync: Bibliotheksstelle nicht schreibbar. { $error } Prüfen Sie, ob in den Bibliotheksordner geschrieben werden kann.
sync-no-state = Sync ist in diesem Lauf aus: nichts wird gespeichert.
sync-choose-folder = Sync-Ordner wählen
sync-group-places = Stellen
sync-group-notes = Notizen
sync-group-highlights = Markierungen
sync-group-bookmarks = Lesezeichen
sync-group-statistics = Statistik
sync-group-item = { $name }: { $state }
sync-start = Sync starten
sync-groups-title = Was synchronisiert wird
sync-groups-intro = { $title }, als { $name }. Enter schaltet ein oder aus; Sync starten beendet.
sync-started = Sync an, als { $name }. Sync-Status: { $key }.
sync-how-to-set-up = Einrichten: Werkzeuge, Sync, Sync einrichten.
sync-now-started = Synchronisiert.
sync-now-done =
    { $n ->
        [one] Sync: aktuell, 1 Dokument geprüft.
       *[other] Sync: aktuell, { $n } Dokumente geprüft.
    }
sync-now-changed =
    { $n ->
        [one] Sync: 1 Dokument hat Änderungen übernommen.
       *[other] Sync: { $n } Dokumente haben Änderungen übernommen.
    }
sync-no-places = Kein anderer Computer hat hier eine Stelle.
sync-place-item = { $device }, { $pct } Prozent
sync-places-title =
    { $n ->
        [one] 1 andere Stelle
       *[other] { $n } andere Stellen
    }
sync-no-replaced = Keine ersetzten Notizen in diesem Dokument.
sync-replaced-item = { $text }, ersetzt von { $device }
sync-replaced-item-deleted = { $text }, gelöscht von { $device }
sync-replaced-title =
    { $n ->
        [one] 1 ersetzte Notiz
       *[other] { $n } ersetzte Notizen
    }
sync-replaced-intro = { $title }. Enter stellt eine wieder her.
sync-note-restored = Notiz wiederhergestellt: { $text }
sync-already-off = Sync ist hier schon aus.
sync-stopped = Sync hier aus. Der Ordner bleibt, wie er ist.
prompt-sync-computer-name = Name dieses Computers, Enter behält ihn
menu-sync = Sync
menu-cards = Lernkarten
menu-cw-practice = Morsecode üben
section-sync = Sync

## End of S4

## Sync wave, S5 (see locales/en).
sync-settings-arrived =
    { $n ->
        [one] Einstellungen: 1 Änderung von { $device }.
       *[other] Einstellungen: { $n } Änderungen von { $device }.
    }
sync-settings-arrived-several = Einstellungen: { $n } Änderungen von { $computers } Computern.
sync-kept-keys-mac =
    { $n ->
        [one] Mac-Tasten: 1, behalten, hier nicht benutzt.
       *[other] Mac-Tasten: { $n }, behalten, hier nicht benutzt.
    }
sync-kept-keys-pc =
    { $n ->
        [one] Windows- und Linux-Tasten: 1, behalten, hier nicht benutzt.
       *[other] Windows- und Linux-Tasten: { $n }, behalten, hier nicht benutzt.
    }
sync-group-settings = Einstellungen
sync-group-profiles = Profile
sync-group-key-overrides = Eigene Tasten
sync-group-words = Wortliste
sync-group-glossary = Glossar und Aussprachen
sync-group-favorite-voices = Lieblingsstimmen
voices-missing-row = { $voice }, Favorit, nicht auf diesem Computer
voices-missing = { $voice } ist nicht auf diesem Computer. Leertaste entfernt sie aus den Favoriten.
gui-voices-list = Stimmen
gui-voices-use = Stimme verwenden
gui-voices-use-help = Die ausgewählte Stimme verwenden und ein Beispiel hören, oder sie nach einer Frage herunterladen.
gui-voices-preview = Hörprobe
gui-voices-preview-help = Ein Beispiel in der ausgewählten Stimme hören, ohne sie zu wählen.
gui-voices-favorite = Favorit
gui-voices-favorite-help = Die ausgewählte Stimme als Favoriten markieren oder die Markierung entfernen. Favoriten stehen zuerst.
gui-voices-remove = Entfernen
gui-voices-remove-help = Die ausgewählte heruntergeladene Piper-Stimme nach einer Frage entfernen.
gui-voices-remove-unavailable = nicht verfügbar
gui-voices-language-help = Nur die Stimmen der nächsten Sprache zeigen, danach wieder alle Sprachen.
gui-voices-engine-help = Nur die Stimmen der nächsten Engine zeigen, danach wieder alle Engines.
gui-voices-fetch-help = Die Liste der Piper-Stimmen, etwa 250 KB, nach einer Frage herunterladen.
gui-voices-close-help = Die Stimmenverwaltung schließen.
gui-voices-hint = Eingabetaste verwendet die Stimme, Leertaste markiert einen Favoriten, Escape schließt.

## End of S5
