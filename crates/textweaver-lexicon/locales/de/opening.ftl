### Opening documents, the file browser, and batch conversion.

## Opening a document.

open-opened = { $title } geöffnet.
open-resumed = { $title } geöffnet. Fortgesetzt bei { $pct } Prozent.
open-resumed-synced = { $title } geöffnet. Fortgesetzt bei { $pct } Prozent, von einem anderen Gerät.

## Opening a document: failures and opening in the background.

# $name is the file's name.
opening-is-folder = { $name } ist ein Ordner, kein Dokument. Geben Sie den Namen einer Datei darin an.
# Said after "Could not open NAME:", so it starts in lower case.
opening-no-file-in = es gibt keine Datei namens { $name } in { $folder }. Prüfen Sie den Namen.
opening-no-file-here = es gibt keine Datei namens { $name } hier. Prüfen Sie den Namen.
opening-no-permission = Sie haben keine Berechtigung, sie zu lesen.
opening-damaged-rtf = es ist keine lesbare RTF-Datei; sie ist möglicherweise beschädigt.
opening-damaged-odt = es ist keine lesbare OpenDocument-Textdatei; sie ist möglicherweise beschädigt.
opening-damaged-latex = es ist keine lesbare LaTeX-Datei; sie ist möglicherweise beschädigt oder zu groß.
opening-damaged-email = es ist keine lesbare E-Mail-Nachricht; sie ist möglicherweise beschädigt oder zu groß.
opening-damaged-mhtml = es ist kein lesbares Webarchiv; es ist möglicherweise beschädigt oder zu groß.
opening-pdf-password = es ist durch ein Passwort geschützt; entfernen Sie das Passwort zuerst in einem PDF-Programm.
opening-old-office = es hat ein altes Office-Format; speichern Sie es als .docx und öffnen Sie dann das.
opening-rar = es ist ein RAR-Archiv, das sich nicht öffnen lässt. Entpacken Sie es zuerst, oder verwenden Sie ZIP oder 7z.
# $reason is one of the opening-no-* messages, or the loader's own words.
opening-failed = { $name } konnte nicht geöffnet werden: { $reason }
opening-started = { $name } wird geöffnet. Escape bricht ab.
opening-stopped = Öffnen von { $name } gestoppt.
opening-still = { $name } wird noch geöffnet, { $secs } Sekunden.
# $step is the loader's report, such as "recognizing text on page 3 (3 of 40)."
opening-still-step = { $name } wird noch geöffnet: { $step }
opening-stopped-unexpectedly = { $name } konnte nicht geöffnet werden: das Laden ist unerwartet gestoppt.

## A build without the publish feature. "tw convert" is a command typed
## at the terminal: keep it as it is.

lean-citations-not-in-build = Zitate sind in dieser Version von { -brand } nicht enthalten.
lean-publish-not-in-build = Export und Vorschau sind in dieser Version von { -brand } nicht enthalten. tw convert wandelt weiterhin um.

## Background work, and opening files and addresses.

# $what names the export, as its own message says it.
tasks-stopped = { $what } unerwartet gestoppt.
# $input is the DOI, ISBN, or other identifier being looked up.
tasks-lookup-stopped = Nachschlagen von { $input } unerwartet gestoppt.
# The reason in tasks-could-not-open when a session keeps no files.
tasks-launch-off = andere Programme öffnen ist aus in einer Sitzung, die keine Dateien hält
tasks-opening = Wird geöffnet.
# $target is a file or a web address; $error says why.
tasks-could-not-open = { $target } konnte nicht geöffnet werden: { $error }
open-refused-scheme = Nicht geöffnet: { $scheme } gesperrt.
open-refused-missing = Nicht geöffnet: Datei nicht gefunden.
open-refused-invalid = Nicht geöffnet: keine gültige Adresse.
open-refused-network = Nicht geöffnet: Die Datei liegt auf einem anderen Computer.
tasks-not-opened = Nicht geöffnet.
# Keep the letters y and n: they are the keys that answer.
tasks-open-it-question = Öffnen? y oder n

## Opening the new formats. Said after "Could not open NAME:", so
## each starts in lower case.
opening-damaged-json = es ist keine lesbare JSON-Datei; sie ist möglicherweise zu groß.
opening-damaged-notebook = es ist kein lesbares Jupyter-Notebook; es ist möglicherweise beschädigt oder zu groß.
opening-damaged-canvas = es ist keine lesbare Canvas-Datei; sie ist möglicherweise beschädigt oder zu groß.
opening-damaged-svg = es ist keine lesbare SVG-Zeichnung; sie ist möglicherweise beschädigt oder zu groß.
opening-damaged-mathml = es ist keine lesbare MathML-Formel; sie ist möglicherweise beschädigt oder zu groß.

## The file browser. Every row and introduction starts with the name,
## then the kind, so the first cells of a 40-cell Braille line hold what
## matters. $name is a file or folder name; $n a number that chooses the
## plural and $count the same number written with its separators.
# A list item with its position after it, in the file browser.
listmodel-item-position-last = { $item }, { $k } von { $n }
browse-places-title = Orte
browse-places-intro =
    { $n ->
        [one] Orte, 1 Ort.
       *[other] Orte, { $n } Orte.
    }
# $purpose says what the folder or file is chosen for; $intro follows.
browse-choosing = { $purpose }. { $intro }
browse-place-document = { $name }, der Ordner des Dokuments
browse-place-start = { $name }, Startordner
browse-place-library = { $name }, Bibliotheksordner
browse-place-disk = { $name }, Laufwerk
browse-place-removable = { $name }, Wechseldatenträger
browse-place-network = { $name }, Netzlaufwerk
browse-place-cd = { $name }, CD- oder DVD-Laufwerk
browse-place-root = { $name }, der Stammordner
browse-choose-here = Diesen Ordner wählen, { $name }
browse-row-folder = { $name }, Ordner
browse-row-folder-items =
    { $name }, Ordner, { $n ->
        [one] 1 Eintrag
       *[other] { $count } Einträge
    }
# $kind is a kind below ("Markdown"); $size a size below ("12 KB").
browse-row-file = { $name }, { $kind }, { $size }
browse-row-kind = { $name }, { $kind }
browse-row-hidden = { $row }, versteckt
# $kind is zip, tar, tar.gz, gzip, or 7z.
browse-kind-archive = { $kind }-Archiv
browse-kind-file = Datei
browse-kind-markdown = Markdown
browse-kind-text = Text
browse-kind-html = Webseite
browse-kind-epub = EPUB-Buch
browse-kind-docx = Word-Dokument
browse-kind-rtf = RTF-Dokument
browse-kind-odt = OpenDocument-Text
browse-kind-latex = LaTeX
browse-kind-eml = E-Mail
browse-kind-mhtml = Webarchiv
browse-kind-pdf = PDF
browse-kind-image = Bild
browse-kind-daisy = DAISY-Buch
browse-kind-pptx = PowerPoint-Folien
browse-kind-sheet = Tabelle
browse-kind-json = JSON
browse-kind-notebook = Jupyter-Notizbuch
browse-kind-svg = SVG-Zeichnung
browse-kind-mathml = MathML-Formel
browse-kind-pandoc = mit Pandoc gelesenes Dokument
browse-size-bytes =
    { $n ->
        [one] 1 Byte
       *[other] { $count } Byte
    }
# $size is a number, with a decimal under 10 ("3.4").
browse-size-kb = { $size } KB
browse-size-mb = { $size } MB
browse-size-gb = { $size } GB
browse-intro =
    { $name }, { $n ->
        [one] 1 Eintrag.
       *[other] { $count } Einträge.
    }
browse-intro-empty = { $name } enthält nichts zum Anzeigen.
# $filter is what was typed.
browse-intro-filtered =
    { $name }, { $n ->
        [one] 1 Eintrag passt zu
       *[other] { $count } Einträge passen zu
    } { $filter }.
browse-intro-in-archive = { $intro } In { $archive }.
browse-intro-hidden =
    { $intro } { $n ->
        [one] 1 Datei ausgeblendet.
       *[other] { $count } Dateien ausgeblendet.
    }
browse-intro-cut = { $intro } Nur die ersten { $max } werden angezeigt.
# $preview, $choose, $sort, and $all are keys; $item the focused row.
browse-keys = { $intro } Eingabe öffnet, Rücktaste geht nach oben, Tippen filtert. { $preview } zeigt eine Vorschau, { $choose } wählt einen Ordner, { $sort } sortiert, { $all } zeigt alle Dateien. { $item }
browse-sorted-name = Nach Name sortiert.
browse-sorted-date = Nach Datum sortiert, neueste zuerst.
browse-sorted-size = Nach Größe sortiert, größte zuerst.
browse-showing-all = Alle Dateien werden angezeigt.
browse-showing-readable = Nur lesbare Dateien werden angezeigt.
browse-closed = Dateibrowser geschlossen.
browse-read-only = Der Dateibrowser öffnet und wählt nur Dateien; er ändert sie nie.
# $key is the Choose Folder key.
browse-choose-a-folder = Wählen Sie einen Ordner: Eingabe öffnet einen, { $key } wählt ihn.
browse-choose-a-file = Wählen Sie eine Datei: Eingabe wählt eine.
browse-no-archive-folder = Ein Ordner in einem Archiv kann nicht gewählt werden; wählen Sie einen Ordner auf dem Laufwerk.
browse-nothing-waiting = Kein Befehl wartet auf einen Ordner; Eingabe öffnet ihn.
browse-cannot-read = { $name } ist keine Art von Datei, die textweaver lesen kann.
browse-folder-unreadable = { $name } konnte nicht geöffnet werden: { $reason }
browse-archive-too-deep = { $name } steckt in zu vielen Archiven, um es zu öffnen.
browse-archive-too-large = { $name } ist zu groß, um es sicher aufzulisten.
browse-archive-unreadable = { $name } ist kein Archiv, das textweaver lesen kann; es ist vielleicht beschädigt.
# A document's preview: its title, then its first sentence.
browse-preview-document = { $title }. { $sentence }
browse-preview-no-text = { $title }. Es enthält keinen Text.
browse-preview-failed = Keine Vorschau für { $name }: { $reason }
# $names are the first few names inside.
browse-preview-archive =
    { $name }: { $n ->
        [one] 1 Datei
       *[other] { $count } Dateien
    }, { $readable } lesbar. { $names }
browse-preview-archive-folder =
    { $name }, Ordner im Archiv, { $n ->
        [one] 1 Eintrag.
       *[other] { $count } Einträge.
    }
browse-preview-folder = { $path }: { $names }
browse-preview-folder-empty = { $path }: hier gibt es nichts zu lesen.
browse-preview-other = { $name }, { $size }; textweaver kann diese Art von Datei nicht lesen.
browse-preview-path = { $path }

## Batch conversion (File, Batch convert). Keep the meaning first.
batch-choose-source = Wählen Sie den Ordner zum Umwandeln
batch-choose-output = Wählen Sie den Ordner für die umgewandelten Dateien
batch-format-title = Umwandeln in
batch-format-intro = { $name } umwandeln in: Wählen Sie ein Format, { $n } zur Auswahl.
batch-where-title = Wohin die Dateien kommen
batch-where-intro = Wohin sollen die umgewandelten Dateien?
batch-where-converted = In einen Ordner converted, { $path }
batch-where-beside = Neben jede Datei
batch-where-choose = In einen anderen Ordner, als Nächstes gewählt
batch-nothing = In { $path } gibt es keine Dokumente zum Umwandeln.
batch-confirm =
    { $n ->
        [one] 1 Datei
       *[other] { $n } Dateien
    } in { $format } umwandeln, nach { $path }? y oder n
batch-confirm-beside =
    { $n ->
        [one] 1 Datei
       *[other] { $n } Dateien
    } in { $format } umwandeln, neben jede Datei? y oder n
batch-started =
    { $n ->
        [one] 1 Datei wird
       *[other] { $n } Dateien werden
    } in { $format } umgewandelt. Escape hält an.
batch-progress = { $percent } Prozent umgewandelt, { $done } von { $total } Dateien.
batch-busy = Umwandlung läuft schon, { $done } von { $total } Dateien. Escape hält an.
batch-stop-question = Umwandlung anhalten? Fertige Dateien bleiben erhalten. y oder n
batch-stopping = Hält nach den Dateien an, die gerade geschrieben werden.
batch-still-converting = Die Umwandlung läuft weiter.
batch-done =
    { $converted ->
        [one] 1 Datei
       *[other] { $converted } Dateien
    } in { $format } umgewandelt; { $skipped } aktuell; { $failed } fehlgeschlagen.
batch-stopped =
    Angehalten. { $converted ->
        [one] 1 Datei
       *[other] { $converted } Dateien
    } umgewandelt; { $left } nicht umgewandelt; { $failed } fehlgeschlagen.
batch-report = Bericht gespeichert in { $path }.
batch-report-failed = Der Bericht konnte nicht gespeichert werden: { $error } Die umgewandelten Dateien bleiben erhalten.
batch-inaccessible =
    { $n ->
        [one] 1 Datei hat
       *[other] { $n } Dateien haben
    } Inhalte, die nicht barrierefrei wurden; siehe Bericht.
batch-failures-title =
    { $n ->
        [one] 1 fehlgeschlagene Datei
       *[other] { $n } fehlgeschlagene Dateien
    }
batch-failure-item = { $name }: { $reason }
batch-start-failed = Umwandlung konnte nicht beginnen: { $error } Prüfen Sie Ordner und Format und versuchen Sie es erneut.
batch-thread-stopped = Die Stapelumwandlung hat unerwartet angehalten.
