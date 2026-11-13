### Export, preview, and audio export.

## Export and preview from the reader. F5 is the browser's reload key,
## not textweaver's.

common-no-document = Kein Dokument ist geöffnet.
# Said after "Could not export:", so it starts in lower case. $path is a
# folder or a file; $error the system's reason.
publish-cannot-write-to = kann nicht nach { $path } schreiben: { $error } Prüfen Sie, ob in den Ordner geschrieben werden kann.
publish-cannot-write = kann { $path } nicht schreiben: { $error } Prüfen Sie, ob in seinen Ordner geschrieben werden kann.
publish-start-failed = Der Export konnte nicht gestartet werden: { $error } Warten Sie kurz und versuchen Sie es dann erneut.
publish-export-error = Export nicht möglich: { $error } Beheben Sie das und exportieren Sie dann erneut.
publish-export-as-label = Exportieren unter, Eingabetaste für { $path }
publish-export-over-source = Nicht exportiert: Das ist das Dokument selbst. Wählen Sie einen anderen Namen.
# $format is the format's name, such as PDF, HTML, or Word.
publish-exporting = Export nach { $format }.
publish-theme-title = Design für die HTML-Seite
publish-theme-intro = Design für die HTML-Seite? { $first } zuerst, { $n } zur Auswahl. Escape bricht ab.
publish-writing-preview = Die Vorschau wird geschrieben.
publish-preview-error = Die Vorschau konnte nicht geschrieben werden: { $error } Beheben Sie das und öffnen Sie die Vorschau erneut.
publish-still-exporting =
    { $secs ->
        [one] Export nach { $format } läuft noch, { $secs } Sekunde.
       *[other] Export nach { $format } läuft noch, { $secs } Sekunden.
    }
publish-still-previewing =
    { $secs ->
        [one] Die Vorschau wird noch geschrieben, { $secs } Sekunde.
       *[other] Die Vorschau wird noch geschrieben, { $secs } Sekunden.
    }
# "Browser preview follows" was chosen. $again is yes when a preview
# is open that reloads by itself only once it is opened again.
publish-follow-off = Browser-Vorschau folgt nichts: drücken Sie nach dem Speichern F5 im Browser.
publish-follow-save =
    { $again ->
        [yes] Browser-Vorschau folgt jedem Speichern: die Seite lädt von selbst neu. Öffnen Sie die Vorschau im Browser erneut, um es zu starten.
       *[no] Browser-Vorschau folgt jedem Speichern: die Seite lädt von selbst neu.
    }
publish-follow-typing =
    { $again ->
        [yes] Browser-Vorschau folgt Ihrem Tippen: die Seite lädt nach jedem Speichern und bei Tipppausen neu. Öffnen Sie die Vorschau im Browser erneut, um es zu starten.
       *[no] Browser-Vorschau folgt Ihrem Tippen: die Seite lädt nach jedem Speichern und bei Tipppausen neu.
    }
# The first preview of a session: what the browser will do. $follow is
# the "Browser preview follows" setting: off, save, or typing.
publish-preview-first =
    { $follow ->
        [save] Die Vorschau öffnet sich in Ihrem Browser und lädt nach jedem Speichern neu.
        [typing] Die Vorschau öffnet sich in Ihrem Browser und folgt Ihrem Tippen.
       *[off] Die Vorschau öffnet sich in Ihrem Browser. Drücken Sie dort nach jedem Speichern F5.
    }
# $error is the converter's reason.
publish-export-failed = Export nach { $format } fehlgeschlagen: { $error } Versuchen Sie ein anderes Format.
publish-preview-failed = Vorschau fehlgeschlagen: { $error } Speichern Sie, um es erneut zu versuchen.
# The converter's warnings: how many, and the first one.
publish-warnings =
    { $n ->
        [one] 1 Warnung: { $first }
       *[other] { $n } Warnungen; die erste: { $first }
    }
# $file is the file's name, $folder its folder; $warned is empty or a
# space and publish-warnings.
publish-exported = Nach { $format } exportiert: { $file }. Öffnen? y oder n. In { $folder }.{ $warned }
publish-report = Bericht gespeichert als { $file }.
publish-report-issues =
    { $n ->
        [one] 1 Element
       *[other] { $n } Elemente
    } nicht barrierefrei; siehe Bericht.
publish-report-failed = Der Bericht konnte nicht gespeichert werden: { $error } Prüfen Sie, ob in den Ausgabeordner geschrieben werden kann.
publish-preview-written-served = Vorschau geschrieben. Sie wird im Browser geöffnet. Sie lädt nach jedem Speichern von selbst neu.{ $warned }
publish-preview-written = Vorschau geschrieben. Sie wird im Browser geöffnet. Speichern schreibt sie erneut; drücken Sie dann F5 im Browser.{ $warned }
publish-preview-updated = Vorschau aktualisiert.
publish-preview-updated-press-f5 = Vorschau aktualisiert. Drücken Sie F5 im Browser.
# Said once for a document whose preview was cleaned of raw HTML
# (scripts, event handlers) because its folder is not trusted.
publish-preview-raw-html-left-out = Rohes HTML in der Vorschau weggelassen. Vertrauenswürdige Ordner stehen in den Einstellungen.
publish-server-failed = Der Neuladeserver der Vorschau konnte nicht gestartet werden ({ $error }); die Datei wird stattdessen geöffnet.
publish-render-failed = Der Text konnte nicht dargestellt werden: { $error } Verlassen Sie den Bearbeitungsmodus, um den Text zu lesen.
publish-nothing-after-caret = Nichts zu lesen nach dem Cursor.
publish-listening = Der dargestellte Text wird angehört.

## The preview's reload server: shown in the browser.

preview-being-written = Die Vorschau wird gerade geschrieben. Laden Sie gleich neu.

## Audio export (File, Export audio). Keep the meaning first: a
## 40-cell Braille display shows the start of the line. $name is a file
## name (essay.flac); $path a folder or a file's full path; $format a
## format's name (FLAC, MP3); $voice a voice's or engine's name; $wpm is
## words per minute; $length a length of time from the duration-*
## messages; $chapters and $n are numbers; $percent is a multiple of ten;
## $formats lists format names (M4B); $error is passed on as it is.
audio-format-title = Audio exportieren als
audio-format-intro = { $name } als Audio exportieren: Format wählen, { $n } Möglichkeiten.
audio-no-ffmpeg =
    { $formats } { $n ->
        [one] braucht
       *[other] brauchen
    } ffmpeg, das nicht gefunden wurde.
audio-format-flac = FLAC: verlustfrei, etwa halb so groß wie WAV
audio-format-wav = WAV: am größten, läuft überall
audio-format-mp3 = MP3: klein, läuft überall
audio-format-opus = Opus: am kleinsten, für Sprache gemacht
audio-format-ogg = Ogg Vorbis: klein und offen, läuft in den meisten Playern
audio-format-m4b = M4B-Hörbuch, über ffmpeg
audio-format-mp4 = Video mit Untertiteln: MP4, braucht ffmpeg
audio-format-html = Mitlese-Seite: Text und Audio, eine Datei
audio-where-title = Wohin das Audio kommt
audio-where-intro = Wohin soll das Audio?
audio-where-beside = Neben das Dokument, { $path }
audio-where-choose = In einen anderen Ordner, als Nächstes gewählt
audio-choose-folder = Ordner für das Audio wählen
audio-no-engine = Keine Sprachausgabe hier kann Audiodateien schreiben. Installieren Sie eSpeak NG, oder wählen Sie im Menü Sprache eine andere.
audio-confirm = { $name } mit { $voice } bei { $wpm } Wörtern pro Minute in { $path } exportieren? y oder n
audio-started = { $name } wird als { $format } exportiert. Escape hält an.
audio-progress = Audio wird exportiert, { $percent } Prozent.
audio-video-progress =
    { $minutes ->
        [one] Video wird kodiert: { $fed } von { $all } Bildern, noch etwa 1 Minute.
       *[other] Video wird kodiert: { $fed } von { $all } Bildern, noch etwa { $minutes } Minuten.
    }
audio-video-progress-soon = Video wird kodiert: { $fed } von { $all } Bildern, noch unter einer Minute.
audio-busy = { $name } wird bereits exportiert. Escape hält an.
audio-stop-question = Export anhalten? Keine Datei bleibt. y oder n
audio-stopping = Der Export wird angehalten.
audio-still-exporting = Audio wird weiter exportiert.
audio-stopped = Audioexport angehalten; keine Datei geschrieben.
audio-done =
    { $name } geschrieben: { $length }, { $chapters ->
        [one] 1 Kapitel
       *[other] { $chapters } Kapitel
    }.
audio-subtitles = Untertitel in { $name }.
audio-failed = Audio konnte nicht exportiert werden: { $error } Versuchen Sie ein anderes Format oder eine andere Stimme.
audio-thread-stopped = Der Audioexport hat unerwartet angehalten.
