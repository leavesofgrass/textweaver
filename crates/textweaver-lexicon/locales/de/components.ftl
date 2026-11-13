### Optional components and updates.

## Optionale Komponenten.
component-feature-dictation = Diktieren
component-feature-ocr = gescannte Seiten lesen
component-feature-reading-font = eine Leseschrift
component-feature-voice = eine Stimme
component-feature-audio-export = M4B- und MP4-Export
component-feature-braille = Braille-Dateien als Schwarzschrift, Kurzschrift
component-feature-pandoc = weitere Dokumentformate
component-state-installed = installiert
component-state-not-installed = nicht installiert
component-state-partial = teilweise installiert
component-state-damaged = beschädigt
component-state-downloading = wird geladen
component-state-installer-ready = Installationsprogramm geladen
components-title = Optionale Komponenten
components-intro =
    { $n ->
        [one] 1 optionale Komponente. Eingabe für Aktionen.
       *[other] { $n } optionale Komponenten. Eingabe für Aktionen.
    }
components-item = { $title }: { $state }, { $size }, Lizenz { $license }, für { $features }
components-actions-intro = { $title }: { $state }.
components-action-download = Herunterladen, { $size }
components-action-verify = Dateien prüfen
components-action-remove = Entfernen
components-action-install-zip = Aus einer ZIP-Datei installieren…
components-action-install-folder = Aus einem Ordner installieren…
components-install-purpose = Die Komponente von hier installieren
component-question = { $title } herunterladen, { $size }, Lizenz { $license }? y oder n
component-remove-question = { $title } entfernen? y oder n
component-downloading = Wird geladen. Escape bricht ab.
component-installing = Installation aus der Datei.
component-verifying = Dateien werden geprüft.
component-progress = { $percent } Prozent geladen.
component-ready = Bereit: { $title }.
component-verified = Dateien in Ordnung: { $title }.
component-verify-failed =
    { $n ->
        [one] 1 Datei stimmt nicht: { $files }.
       *[other] { $n } Dateien stimmen nicht: { $files }.
    }
component-removed = Entfernt: { $title }.
component-refused =
    { $n ->
        [one] 1 Datei ausgelassen: { $files }.
       *[other] { $n } Dateien ausgelassen: { $files }.
    }
component-already = Schon installiert: { $title }.
component-not-there = Nicht installiert: { $title }.
component-declined = Nicht heruntergeladen.
component-not-in-build = Downloads fehlen in diesem Build.
component-no-folder = Kein Datenordner dafür.
component-error-fetch = Nicht geladen: Quelle fehlgeschlagen.
component-error-size = Nicht installiert: falsche Größe.
component-error-hash = Nicht installiert: eine Datei passte nicht. Neu laden oder die Quelle prüfen.
component-error-missing = Nicht installiert: Datei fehlt.
component-error-cancelled = Gestoppt; geht später weiter.
component-error-busy = Es lädt schon eine.
component-error-no-source = Nicht geladen: keine Adresse.
component-error-name = Abgelehnt: Name nicht einfach.
component-error-manifest = Eine Komponentenliste ist unlesbar.
component-error-io = Nicht installiert: Schreibfehler.
component-error-unpack = Nicht installiert: das Archiv ließ sich nicht entpacken.
component-launch-question = Installationsprogramm starten: { $title }, Version { $version }, Lizenz { $license }? Danach fragt das System. y oder n
component-version-unknown = nicht angegeben
component-launched = Installationsprogramm gestartet: { $title }. Folgen Sie den Fragen des Systems.
component-launch-failed = Installationsprogramm startete nicht: { $reason }
component-launch-declined = Nicht gestartet; das Installationsprogramm bleibt.
components-token-invalid = Kein Token. Bitte prüfen und erneut einfügen.
components-token-kept = Token im Anmeldeinformationsspeicher des Systems gespeichert.
components-token-not-kept = Token verwendet, nicht gespeichert: { $reason }.
components-token-forgotten = Token vergessen.
components-token-none = Es war kein Token gespeichert.
components-token-not-forgotten = Nicht vergessen: { $reason }.
components-token-gh-still = Die Anmeldung der GitHub CLI wird weiter verwendet; gh auth logout beendet sie.
components-chooser-title = Optionale Komponenten
components-chooser-intro = Optionale Extras, keine gewählt. Leertaste wählt; Gewählte herunterladen holt sie; Escape überspringt.
components-chooser-item = { $mark }: { $title }, für { $features }, { $size }, Lizenz { $license }
components-chosen = Gewählt
components-not-chosen = Nicht gewählt
components-chooser-download = Gewählte herunterladen
components-chooser-skip = Später
components-chooser-skipped = Übersprungen; siehe Komponenten.
components-chooser-none = Nichts gewählt, nichts geladen.
dictation-model-question = Diktieren braucht das Whisper-Modell, { $size }, Lizenz { $license }. Jetzt herunterladen? y oder n
dictation-model-declined = Kein Modell, vorerst kein Diktat.
helper-question = { $need } braucht { $title }, { $size }, Lizenz { $license }. Jetzt herunterladen? y oder n
helper-need-audio = M4B- und MP4-Export
helper-need-brf = Braille als Schwarzschrift lesen
helper-run-command = Um { $title } zu bekommen, ausführen: { $command }
dictation-model-not-in-build = Kein Modell, keine Downloads im Build.
dictation-model-file-missing = Dem Modell fehlt { $file }.
dictation-model-damaged = Diktiermodell beschädigt: { $file }.
dictation-model-no-folder = Kein Modellordner: { $dir }.

## Einstellungen der optionalen Komponenten.

## Updates (B1-u1).
components-chooser-updates = { $mark }: Automatisch nach Updates suchen, einmal täglich, auf GitHub
update-checking = Suche nach Updates.
update-none = Kein Update: { $version } ist die neueste.
update-found = Update verfügbar: textweaver { $version }, { $size }. Herunterladen? y oder n
update-available-elsewhere = Update verfügbar: textweaver { $version }. Diese Kopie so aktualisieren, wie sie installiert wurde.
update-declined = Nicht geladen. Eine neuere Version wird angeboten.
update-downloading = Update wird geladen, { $size }.
update-progress = Update zu { $percent } Prozent geladen.
update-installed = Update geprüft und installiert. textweaver neu starten, um es zu nutzen.
update-on-close = Update geprüft. Es wird beim Schließen von textweaver installiert.
update-busy = Suche nach Updates läuft schon.
update-not-in-build = Updates fehlen in diesem Build.
update-automatic-question = Automatisch nach Updates suchen, einmal täglich? y oder n
update-automatic-on = Updates: einmal täglich gesucht.
update-automatic-off = Updates: nicht gesucht. Hilfe, Nach Updates suchen sucht jederzeit.
update-error-check = Update-Suche fehlgeschlagen: { $reason }
update-error-no-package = Kein Updatepaket für diesen Computer in { $version }.
update-error-no-checksum = Update abgelehnt: keine Prüfsumme dafür. Nichts geändert.
update-error-mismatch = Update abgelehnt: Prüfsumme passt nicht. Nichts geändert.
update-error-cancelled = Update-Download gestoppt; er geht beim nächsten Mal weiter.
update-error-download = Update nicht geladen: { $reason }
update-error-install = Update nicht installiert: { $reason }. Nichts geändert.
update-error-not-package = Nicht aktualisiert: diese Kopie stammt nicht aus einem Release-Paket.
section-updates = Updates und Komponenten
