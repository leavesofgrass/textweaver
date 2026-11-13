### Live documents, live sources, and the broker.

## Live documents (beta 5): settings, and live sources' passwords. $source
## is a live source's name; $reason says why the credential store failed.
section-live = Live-Dokumente
prompt-live-password = Passwort für { $source }, Eingabe bricht ab
prompt-live-password-any = Passwort für die Live-Quelle, Eingabe bricht ab
live-source-unknown = Keine Live-Quelle namens { $source }. Prüfen Sie die Quellen in settings.toml.
live-password-no-store = Kein Anmeldeinformationsspeicher in dieser Version: { $source } kann sich nicht mit Passwort anmelden. Nutzen Sie ein Release-Paket.
live-password-invalid = Kein Passwort: höchstens 1024 Zeichen, ohne Zeilenumbrüche oder Tabulatoren.
live-password-kept = Passwort für { $source } im Anmeldeinformationsspeicher des Systems gespeichert.
live-password-not-kept = Passwort für { $source } nicht gespeichert: { $reason }. Nur in dieser Sitzung verwendet.
live-password-forgotten = Passwort für { $source } vergessen.
live-password-none = Kein Passwort für { $source } gespeichert.
live-password-not-forgotten = Passwort für { $source } nicht vergessen: { $reason }. Versuchen Sie es erneut oder entfernen Sie es im Anmeldeinformationsspeicher des Systems.

## Beta 5: live documents (a log fed by a live source, such as an MQTT
## broker). $name is the source's name, $topic an MQTT topic, $path and
## $folder file paths, $line a log entry as one line.
live-log-title = Live-Protokoll
live-log-empty = Noch keine Einträge.
live-entry-empty = Leerer Eintrag.
live-field-callsign = Rufzeichen
live-field-name = Name
live-field-location = Ort
live-field-status = Status
live-field-traffic = Verkehr
live-field-notes = Notizen
live-field-time = Zeit
live-traffic = Verkehr
live-yes = ja
live-no = nein
live-read-only = Dieses Live-Protokoll ist schreibgeschützt; es kann nicht bearbeitet werden.
live-not-open = Kein Live-Protokoll ist geöffnet.
live-view-list = Listenansicht.
live-view-table = Tabellenansicht.
live-newest-none = Das Protokoll hat noch keine Einträge.
live-paused = Aktualisierungen angehalten. Neue Einträge warten, bis Sie fortsetzen.
live-paused-status = Aktualisierungen angehalten
live-stopped = { $name } beendet. Das Protokoll bleibt schreibgeschützt geöffnet.
live-log-saved = Protokoll gespeichert als { $name } in { $folder }.
live-log-empty-not-saved = Das Protokoll hat noch keine Einträge, daher wurde nichts gespeichert.
live-log-nowhere = Es gibt keinen Ordner, in dem das Live-Protokoll gespeichert werden kann.
live-log-saved-already = Dieses Protokoll wurde aus { $path } geöffnet; es ist bereits gespeichert.
live-connecting = Verbindung zu { $name } wird hergestellt.
live-reconnecting = Neue Verbindung zu { $name }, Versuch { $attempt }.
live-connected = Verbunden mit { $name }, verschlüsselt.
live-connected-tailscale = Verbunden mit { $name } über eine Adresse im Tailscale-Bereich. Wenn das Tailscale ist, ist die Verbindung verschlüsselt.
live-connected-plain = Verbunden mit { $name }. Die Verbindung ist nicht verschlüsselt.
live-disconnected = Verbindung zu { $name } getrennt: { $reason }
live-quiet = Noch keine Nachrichten auf { $topic }. Wenn das anhält, prüfen Sie, ob der Broker diesem Konto das Lesen erlaubt.
live-unread =
    { $count ->
        [0] keine ungelesen
       *[other] { $count } ungelesen
    }
live-entries =
    { $count ->
        [one] 1 Eintrag
       *[other] { $count } Einträge
    }
live-resumed =
    { $count ->
        [0] Aktualisierungen fortgesetzt.
        [one] Aktualisierungen fortgesetzt, 1 Aktualisierung übernommen.
       *[other] Aktualisierungen fortgesetzt, { $count } Aktualisierungen übernommen.
    }

## Beta 5: live sources' commands, the set-up steps, and new entries said.
## $name is a source's name, $topic an MQTT topic, $line an entry as one
## line, $reason a sentence in words.
menu-live = Live-Dokumente
menu-live-reading = Lesen
menu-live-broker = Ihr Broker
menu-live-invite = Andere einladen
menu-live-publish = Veröffentlichen
prompt-live-setup = Live-Quelle einrichten
prompt-save-live-source-file = Quelle als Datei speichern
live-failed = Keine Verbindung zu { $name }: { $reason } Keine weiteren Versuche; beheben Sie das und öffnen Sie die Quelle erneut.
live-new-entries = { $line } { $count } neue Einträge.
live-unread-newest = { $count }. Neuester: { $line }
live-not-in-build = Live-Quellen gibt es in dieser Version von { -brand } nicht.
live-speak-on = Neue Einträge sprechen: ein.
live-speak-off = Neue Einträge sprechen: aus.
live-none-set-up = Keine Live-Quellen eingerichtet. „Live-Quelle einrichten“ fügt eine hinzu.
live-open-title = Live-Quellen
live-source-item = { $name }, { $host }, { $topic }
live-forget-title = Das Passwort welcher Live-Quelle vergessen?
live-no-passwords = Keine Live-Quelle meldet sich mit Benutzernamen an, also ist kein Passwort gespeichert.
live-not-receiving = Keine Live-Quelle empfängt.
live-cannot-open = Die Live-Quelle kann nicht geöffnet werden. { $problems }
live-not-opened = { $name } nicht geöffnet: kein Passwort.
live-setup-host-needed = Ein Hostname oder eine Adresse wird gebraucht, ohne Leerzeichen.
live-setup-ca-file-needed = Für diese Verschlüsselung wird eine Datei der Zertifizierungsstelle gebraucht.
live-setup-ca-file-missing = Keine Datei unter { $path }.
live-setup-port-bad = Der Port ist eine Zahl von 1 bis 65535.
live-setup-topic-needed = Ein Topic wird gebraucht.
live-setup-saved = Live-Quelle { $name } gespeichert.
live-setup-not-saved = Live-Quelle { $name } nicht gespeichert: { $error } Prüfen Sie, ob der Einstellungsordner beschreibbar ist, und richten Sie sie erneut ein.
live-setup-offer-title = { $name } jetzt öffnen?
live-setup-open-now = Jetzt öffnen
live-setup-later = Nicht jetzt
live-setup-not-now = Nicht geöffnet. „Live-Quelle öffnen“ öffnet sie später.
live-form-intro-new = Live-Quelle einrichten. Tab wechselt das Feld, Eingabe speichert, Escape bricht ab.
live-form-intro-edit = { $name } bearbeiten. Tab wechselt das Feld, Eingabe speichert, Escape bricht ab.
live-form-intro-file = Live-Quelle aus { $file }. Prüfen Sie jedes Feld: Tab wechselt, Eingabe speichert sie als neue Quelle, Escape bricht ab.
live-form-name = Name
live-form-host = Host des Brokers
live-form-tls = Verschlüsselung
live-form-ca-file = CA-Datei
live-form-port = Port
live-form-topic = Topic
live-form-username = Benutzername
live-form-view = Ansicht
live-form-choice-label = { $label }, { $n } von { $total }, Pfeil nach oben oder unten ändert
live-form-choice-field = { $label }: { $value }. Pfeil nach oben oder unten ändert es.
live-form-tls-off = Aus
live-form-tls-system = Mitgelieferte Stammzertifikate
live-form-tls-ca-file = CA-Datei
live-form-tls-off-about = Aus: nicht verschlüsselt, Port 1883; über Tailscale in Ordnung.
live-form-tls-system-about = Mitgelieferte Stammzertifikate: TLS, geprüft mit den Zertifizierungsstellen, die { -brand } mitbringt, Port 8883.
live-form-tls-ca-file-about = CA-Datei: TLS, geprüft mit einer Datei der Zertifizierungsstelle, die Sie angeben, Port 8883.
live-form-view-log = Protokoll
live-form-view-values = Werte
live-form-view-log-about = Protokoll: ein Eintrag pro Nachricht.
live-form-view-values-about = Werte: eine Zeile pro Feld, an Ort und Stelle aktualisiert.
live-form-name-needed = Ein Name wird benötigt.
live-form-not-a-choice = { $label } ist eines von: { $choices }. Pfeil nach oben oder unten ändert es.
live-form-saved-beside = Live-Quelle { $name } gespeichert, neben einer anderen Quelle mit demselben Namen.
live-form-unchanged = { $name } nicht geändert.
live-form-cancelled = Abgebrochen. Keine Live-Quelle gespeichert.
live-form-cancelled-edit = Abgebrochen. { $name } nicht geändert.
live-edit-title = Welche Live-Quelle bearbeiten?
live-save-file-title = Welche Live-Quelle als Datei speichern?
live-save-file-done = { $name } als { $path } gespeichert, ohne Passwort.
live-save-file-failed = { $name } nicht als Datei gespeichert: { $error } Wählen Sie einen anderen Ordner und versuchen Sie es erneut.
live-file-not-read = { $file } wurde nicht als Live-Quelle geöffnet: { $reason }
live-file-too-large = sie ist größer als { $max } Byte.
live-file-not-text = sie ist kein Text.
live-file-not-toml = sie ist in Zeile { $line } kein gültiges TOML.
live-file-left-out = In der Datei ausgelassen: { $items }.
live-file-note-password = ein Passwort, das { -brand } nie aus einer Datei liest; es wird nach dem Speichern abgefragt
live-file-note-unknown = der unbekannte Schlüssel { $key }
live-file-note-too-many = alle außer den ersten { $max } { $key }
live-values-title = Live-Werte
live-values-empty = Noch keine Werte.
live-value-none = kein Wert
live-value-stale = kein aktueller Wert
live-values-more = { $count } weitere
live-values-count =
    { $count ->
        [one] 1 Feld
       *[other] { $count } Felder
    }
live-values-no-table = Die Werteansicht hat keine Tabelle: Jedes Feld ist eine Zeile.
live-values-no-log = Die Werteansicht führt kein Protokoll, das gespeichert werden könnte.
live-values-full = Die Werteansicht fasst höchstens { $max } Felder; weitere Felder werden ausgelassen.
live-values-resumed =
    { $count ->
        [0] Aktualisierungen fortgesetzt. Kein Feld hat sich geändert.
        [one] Aktualisierungen fortgesetzt, 1 Feld geändert.
       *[other] Aktualisierungen fortgesetzt, { $count } Felder geändert.
    }
live-field-none = In dieser Zeile ist kein Feld.
live-field-show-on = Änderungen von { $field } werden angezeigt.
live-field-show-off = Änderungen von { $field } werden nicht mehr angezeigt.
live-field-speak-on = Änderungen von { $field } werden gesprochen.
live-field-speak-off = Änderungen von { $field } werden nicht mehr gesprochen.
live-state-connecting = Verbindung wird hergestellt
live-state-reconnecting = Erneute Verbindung
live-state-encrypted = Verbunden, verschlüsselt
live-state-tailscale = Verbunden über Tailscale
live-state-plain = Verbunden, nicht verschlüsselt
live-state-lost = Verbindung verloren

## Beta 5, phase 5b: the broker in the app, sharing a document live,
## check-ins, pinned keys and join codes. $name is a document or live
## source; $rev a revision number; $topic an MQTT topic; $code and $pin
## are in groups of four; $reason is a sentence in words.
prompt-live-check-in = Anmeldung
live-form-pin = Pin
live-form-join-code = Beitrittscode
live-form-tls-pinned = Festgelegter Schlüssel
live-form-tls-pinned-about = Festgelegter Schlüssel: TLS, vertraut nur dem Broker-Schlüssel mit dem eingegebenen Pin, wie ihn der Broker von { -brand } ausgibt, Port 8883.
live-form-tls-join = Beitrittscode
live-form-tls-join-about = Beitrittscode: TLS mit dem Schlüssel, den der Code nennt, Anmeldung mit dem Code, Port 8883.
live-form-pin-needed = Ein Pin wird gebraucht: Der Broker gibt ihn nach „Certificate pin“ aus.
live-form-pin-bad = Kein Pin: Er hat 52 Zeichen, wie der Broker sie ausgibt, oder 12 aus einem Beitrittscode, in Vierergruppen.
live-form-join-needed = Ein Beitrittscode wird gebraucht: 16 Zeichen, in vier Vierergruppen.
live-form-join-bad = Kein Beitrittscode: Er hat 16 Buchstaben und Ziffern, in vier Vierergruppen.
live-file-note-join-code = einen Beitrittscode, den { -brand } nie aus einer Datei liest; fragen Sie nach dem Code selbst
live-pin-not-read = { $name } kann nicht geöffnet werden: Sein Pin ist keiner. Bearbeiten Sie die Quelle und geben Sie den Pin neu ein.
live-document-shown = { $name }, Fassung { $rev }, live veröffentlicht.
live-document-updated = { $name } aktualisiert, Fassung { $rev }.
live-document-revision = Fassung { $rev }
live-document-entries-left-out = Protokolleinträge dieser Quelle werden ausgelassen: Sie zeigt ein veröffentlichtes Dokument.
live-document-one-view = Dies ist ein veröffentlichtes Dokument: Es hat keine Einträge, keine Tabelle und kein Protokoll.
live-broker-status =
    { $count ->
        [one] Broker an, 1 Client
       *[other] Broker an, { $count } Clients
    }
live-share-status = Veröffentlicht, Fassung { $rev }
live-broker-already = Der Broker läuft schon.
live-broker-no-folder = es gibt keinen Datenordner für seinen Schlüssel und die Konten.
live-broker-not-started = Broker nicht gestartet: { $reason }
live-broker-nobody = keine Konten und kein Beitrittscode, also könnte sich niemand anmelden. Fügen Sie mit tw broker account add ein Konto hinzu oder schalten Sie den Beitrittscode ein.
live-broker-bad-address = { $address } ist keine Adresse zum Lauschen. Geben Sie eine IP-Adresse an, etwa 127.0.0.1 oder 100.101.1.2, mit oder ohne Port.
live-broker-started-local = Broker an: { $addresses }, TLS, nur dieser Computer.
live-broker-started = Broker an: { $addresses }, TLS.
live-broker-started-plain-local = Broker an: { $addresses }, unverschlüsselt, nur dieser Computer.
live-broker-started-plain = Broker an: { $addresses }, unverschlüsselt.
live-broker-join-code = Beitrittscode: { $code }.
live-broker-pin = Zertifikat-Pin: { $pin }.
live-broker-not-running = Kein Broker läuft. „Ihren Broker starten“ startet einen.
live-join-code-title = Beitrittscode
live-pin-title = Zertifikat-Pin
live-broker-no-join-code = Kein Beitrittscode: Der Broker läuft mit Konten. Leser legen seinen Schlüssel fest.
live-broker-no-join-code-plain = Kein Beitrittscode und kein Pin: Der Broker ist unverschlüsselt.
live-code-group = Gruppe { $n } von { $total }: { $group }
live-join-code-copied = Beitrittscode kopiert.
live-join-link-copied = Beitrittslink kopiert.
live-join-code-pasted = Beitrittscode eingefügt.
live-join-link-filled = Aus dem Beitrittslink ausgefüllt: Host, Port, Code und Topic.
live-join-link-filled-no-topic = Aus dem Beitrittslink ausgefüllt: Host, Port und Code. Geben Sie das Topic ein.
live-join-link-bad = Kein Beitrittslink: { $reason }
live-form-intro-link = Per Link beitreten. Fügen Sie den Beitrittslink in ein beliebiges Feld ein; Eingabe tritt bei, Escape bricht ab.
live-form-intro-link-clipboard = Per Link in der Zwischenablage beitreten. Eingabe tritt bei, Escape bricht ab.
live-qr-title = QR-Code zum Beitreten
live-qr-warning = Wer diesen Code sieht, kann beitreten.
live-qr-image = QR-Code zum Beitreten bei { $host }
live-qr-save = QR-Code speichern
live-qr-draw = QR-Code in Zeichen zeichnen
live-qr-drawing-title = QR-Code zum Beitreten in Zeichen
live-qr-drawing-warning = Es folgen Blockzeichen für eine Kamera; ein Screenreader liest sie als Rauschen. Escape schließt.
live-qr-failed = Kein QR-Code: { $reason }
live-qr-save-type = QR-Code nicht gespeichert: Der Dateiname muss auf .png oder .svg enden.
live-qr-save-failed = QR-Code nicht gespeichert: { $error } Wählen Sie einen anderen Ordner oder Namen.
live-qr-saved = QR-Code gespeichert: { $path }
live-qr-intro-clipboard = Per QR-Code in der Zwischenablage beitreten. Eingabe tritt bei, Escape bricht ab.
live-qr-intro-file = Per QR-Code in { $file } beitreten. Eingabe tritt bei, Escape bricht ab.
live-qr-clipboard-no-link = Kein Beitrittslink im Bild der Zwischenablage: { $reason } Wählen Sie eine Bilddatei.
live-qr-no-link = Kein Beitrittslink: { $reason }
live-qr-other-text = Der QR-Code enthält etwas anderes.
live-qr-not-found = Im Bild wurde kein QR-Code gefunden.
live-qr-unreadable = Ein QR-Code wurde gefunden, ist aber nicht lesbar. Versuchen Sie ein schärferes oder größeres Bild.
live-qr-too-large = Die Bilddatei ist zu groß.
live-qr-too-long = Der Beitrittslink passt nicht in einen QR-Code.
live-qr-image-error = Das Bild ist nicht lesbar: { $error } Versuchen Sie eine PNG- oder JPEG-Datei.
live-qr-reading = Das QR-Bild wird gelesen.
prompt-save-join-qr = QR-Code in Datei speichern (.png oder .svg)
prompt-join-qr-image = Bilddatei mit dem QR-Code
chooser-qr-files = PNG- und SVG-Bilder
live-field-menu-title = Feld Beitrittscode
live-field-hide-join-code = Beitrittscode verbergen
live-field-empty = Das Feld Beitrittscode ist leer.
gui-qr-link = Beitrittslink
live-share-stopped-with-broker = Das Veröffentlichen endete mit dem Broker.
live-broker-stopped = Broker beendet. Alle Leser sind getrennt.
live-send-lost = das Senden brach ab, bevor es fertig war.
live-broker-joined = Verbunden: { $user }.
live-broker-left = Gegangen: { $user }.
live-broker-refused = Anmeldung abgelehnt: { $user }, von { $address }.
live-broker-event = Broker: { $event }
live-broker-join-off = Beitrittscode aus: zu viele falsche Codes. Starten Sie den Broker neu, um einen neuen Code zu erhalten.
live-send-too-large = die Nachricht hat { $len } Bytes; die Grenze ist { $max }.
live-send-no-password = Nicht gesendet: Für { $name } ist kein Passwort gespeichert. Öffnen Sie die Quelle einmal, um es einzugeben.
live-checkin-sent = Veröffentlicht: { $entry }, auf { $topic }.
live-share-sent = Veröffentlicht: { $name }, Fassung { $rev }, auf { $topic }.
live-checkin-not-sent = Anmeldung nicht veröffentlicht: { $reason }
live-share-not-sent = { $name } nicht veröffentlicht: { $reason }
live-share-already = { $name } wird schon auf { $topic } veröffentlicht. Beenden Sie zuerst das Veröffentlichen.
live-share-no-document = Öffnen Sie ein Dokument, um es live zu veröffentlichen.
live-share-live-document = Ein Live-Dokument kann nicht erneut veröffentlicht werden. Öffnen Sie eine Dokumentdatei, um sie zu veröffentlichen.
live-share-through-title = Über welche Live-Quelle veröffentlichen?
live-send-nowhere = Nichts zum Veröffentlichen. Starten Sie Ihren Broker oder richten Sie eine Live-Quelle ein, die veröffentlichen darf.
live-share-none = Kein Dokument wird veröffentlicht.
live-share-stopped = Veröffentlichen von { $name } beendet. Leser behalten Fassung { $rev }.
live-checkin-through-title = Anmeldung über welche Live-Quelle veröffentlichen?
live-checkin-to-broker = den Broker dieses Computers
live-checkin-intro = Anmeldung an { $to }, auf { $topic }. Tab wechselt das Feld, Enter veröffentlicht, Escape bricht ab.
live-checkin-empty = Füllen Sie mindestens ein Feld aus.
live-checkin-cancelled = Abgebrochen. Keine Anmeldung veröffentlicht.
