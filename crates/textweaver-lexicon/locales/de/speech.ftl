### Voices, the voice manager, and restarting speech.

## Restarting speech.

restart-silent-now = { -brand } ist jetzt stumm; starten Sie es neu, um wieder Sprache zu hören.
# $keys names the Restart Speech key or keys.
restart-silent-use-key = { -brand } ist jetzt stumm. Starten Sie die Sprachausgabe neu mit { $keys }.
restart-restarting = Sprachausgabe wird neu gestartet.
restart-not-here = Die Sprachausgabe kann hier nicht neu gestartet werden.
restart-already = Die Sprachausgabe wird bereits neu gestartet.
# $error is the system's reason, in its own words.
restart-failed = Die Sprachausgabe konnte nicht neu gestartet werden: { $error } Warten Sie kurz und versuchen Sie es dann erneut.
restart-start-failed = Die Sprachausgabe konnte nicht neu gestartet werden: der Start ist fehlgeschlagen.
restart-no-engine = Keine Sprachausgabe verfügbar; es bleibt still. „Troubleshooting“, „No speech at all“ nennt die Prüfungen.
restart-done-silent = Sprachausgabe neu gestartet, aber stumm: keine Sprachausgabe ist verfügbar.
restart-done = Sprachausgabe neu gestartet.

## The voice manager's list.

# $n voices are shown; $language is a language name or voices-all-languages;
# $engine an engine's name or voices-all-engines.
voices-shown =
    { $n ->
        [one] { $n } Stimme: { $language }, { $engine }.
       *[other] { $n } Stimmen: { $language }, { $engine }.
    }
voices-all-languages = alle Sprachen
voices-all-engines = alle Engines
# The filter rows at the top of the list.
voices-language-row = Sprache: { $language }
voices-engine-row = Engine: { $engine }
voices-fetch-row = Die Piper-Stimmenliste aus dem Internet abrufen
# Parts of a voice's row, joined with commas. $size is in megabytes, such
# as "63 MB".
voices-download-size = Download { $size }
voices-licence-public-domain = gemeinfrei
voices-licence-attribution = frei mit Namensnennung
voices-licence-share-alike = frei mit Namensnennung, Weitergabe unter gleichen Bedingungen
voices-licence-non-commercial = nicht kommerziell
voices-licence-unknown = Lizenz wird vor dem Download angezeigt
voices-favourite = Favorit
voices-current = aktuell

## Language names in the voice manager's language filter.

voices-language-ar = Arabisch
voices-language-ca = Katalanisch
voices-language-cs = Tschechisch
voices-language-cy = Walisisch
voices-language-da = Dänisch
voices-language-de = Deutsch
voices-language-el = Griechisch
voices-language-en = Englisch
voices-language-es = Spanisch
voices-language-fa = Persisch
voices-language-fi = Finnisch
voices-language-fr = Französisch
voices-language-hi = Hindi
voices-language-hu = Ungarisch
voices-language-is = Isländisch
voices-language-it = Italienisch
voices-language-ja = Japanisch
voices-language-ka = Georgisch
voices-language-kk = Kasachisch
voices-language-ko = Koreanisch
voices-language-lb = Luxemburgisch
voices-language-lv = Lettisch
voices-language-nl = Niederländisch
voices-language-no = Norwegisch
voices-language-pl = Polnisch
voices-language-pt = Portugiesisch
voices-language-ro = Rumänisch
voices-language-ru = Russisch
voices-language-sk = Slowakisch
voices-language-sl = Slowenisch
voices-language-sr = Serbisch
voices-language-sv = Schwedisch
voices-language-sw = Suaheli
voices-language-tr = Türkisch
voices-language-uk = Ukrainisch
voices-language-vi = Vietnamesisch
voices-language-zh = Chinesisch

## Voices: the voice manager, rate, pitch, and volume.

# Spoken by a newly chosen voice as its sample.
voice-sample = Franz jagt im komplett verwahrlosten Taxi quer durch Bayern.
voice-list-title = Eine Stimme wählen
voice-still-loading = Die Stimmen werden noch geladen. Die Liste öffnet sich, sobald sie bereit sind.
voice-list-failed = Die Stimmen konnten nicht aufgelistet werden: { $error } Wählen Sie im Menü Sprache eine andere Sprachausgabe.
# $shown is voices-shown ("12 voices: English, all engines."). Enter,
# Space, Delete and Escape are the list's own keys.
voice-manager-intro = Stimmenverwaltung. { $shown } Eingabetaste verwendet eine Stimme und spricht ein Beispiel, oder lädt eine herunter; { $preview } spielt eine Hörprobe; Leertaste markiert einen Favoriten; Entf entfernt eine heruntergeladene Stimme; Escape schließt.
voice-more-ready =
    { $n ->
        [one] Eine weitere Stimme einer anderen Engine steht jetzt in der Liste.
       *[other] { $n } weitere Stimmen von anderen Engines stehen jetzt in der Liste.
    }
voice-preview = Hörprobe: { $voice }.
voice-preview-sample = { $voice }. Franz jagt im komplett verwahrlosten Taxi quer durch Bayern.
voice-preview-starting = Hörprobe: { $voice }, { $engine } wird gestartet.
voice-preview-not-installed = { $voice } ist noch nicht heruntergeladen. Die Eingabetaste lädt sie herunter, nach einer Frage.
voice-preview-unavailable = { $engine } kann hier nicht für eine Hörprobe starten. Die Eingabetaste wechselt zu dieser Engine.
voice-preview-engine-failed = { $engine } konnte für eine Hörprobe von { $voice } nicht gestartet werden.
voice-preview-failed = Hörprobe von { $voice } nicht möglich: { $error } Versuchen Sie eine andere Stimme.
# $keys names the Choose Voice key.
voice-ready = Die Stimmen sind bereit. { $keys } listet sie auf.
voice-fetch-catalog-question = Die Liste der Piper-Stimmen, etwa 250 Kilobyte, von Hugging Face herunterladen? y oder n
voice-fetch-catalog-question-short = Die Liste der Piper-Stimmen herunterladen? y oder n
# $engine is the engine's name, such as "Piper neural voices".
voice-switching-engine = Stimme { $voice }, auf { $engine }. Engine wird gewechselt.
voice-download-in-progress = Ein Stimmen-Download läuft bereits.
voice-no-data-folder = Es gibt keinen Datenordner, um Piper-Stimmen aufzubewahren.
voice-not-in-list = Diese Stimme ist nicht mehr in der Piper-Stimmenliste.
voice-download-start-failed = Der Download konnte nicht gestartet werden.
voice-reading-licence = Die Lizenz von { $voice } wird gelesen.
voice-remove-question = Die Stimme { $voice } entfernen? y oder n
voice-only-piper-removable = Nur heruntergeladene Piper-Stimmen können entfernt werden.
# $plan describes the download: the voice, its size and licence.
voice-download-question = { $plan } y oder n
voice-in-use = { $voice } ist die verwendete Stimme. Wählen Sie zuerst eine andere Stimme.
voice-removed = { $voice } entfernt.
voice-remove-failed = { $voice } konnte nicht entfernt werden: { $error } Prüfen Sie, ob in seinen Ordner geschrieben werden kann.
voice-downloading-catalog = Die Piper-Stimmenliste wird heruntergeladen.
voice-downloading = { $voice } wird heruntergeladen.
voice-downloading-percent = { $voice } wird heruntergeladen, { $pct } Prozent.
voice-details-failed = Die Details der Stimme konnten nicht gelesen werden: { $error } Prüfen Sie die Verbindung und versuchen Sie es erneut.
voice-download-stopped = Der Download wurde gestoppt.
voice-catalog-fetched = Die Piper-Stimmenliste hat { $voices } Stimmen in { $languages } Sprachen. Der Befehl Stimmen listet sie auf.
voice-catalog-failed = Die Stimmenliste konnte nicht heruntergeladen werden: { $error } Prüfen Sie die Verbindung und versuchen Sie es erneut.
# $licence describes the voice's licence, in a sentence of its own.
voice-installed = { $voice } ist installiert. { $licence } Der Befehl Stimmen listet sie auf.
voice-download-failed = { $voice } konnte nicht heruntergeladen werden: { $error } Wählen Sie die Stimme erneut, um es noch einmal zu versuchen.
voice-only-voice-favourite = Nur eine Stimme kann ein Favorit sein.
voice-favourite-added = { $voice } zu den Favoriten hinzugefügt.
voice-favourite-removed = { $voice } aus den Favoriten entfernt.
voice-chosen = Stimme { $voice }.
voice-chosen-rate = Stimme { $voice }, { $wpm } Wörter pro Minute.
voice-fastest-rate = Schnellste Geschwindigkeit.
voice-slowest-rate = Langsamste Geschwindigkeit.
voice-rate = { $wpm } Wörter pro Minute.
voice-highest-pitch = Höchste Tonhöhe.
voice-lowest-pitch = Tiefste Tonhöhe.
voice-pitch-normal = Normale Tonhöhe.
# $n is a number of semitones.
voice-pitch-plus = Tonhöhe plus { $n }.
voice-pitch-minus = Tonhöhe minus { $n }.
voice-full-volume = Volle Lautstärke.
voice-volume-off = Lautstärke aus.
voice-volume = Lautstärke { $pct } Prozent.
voice-no-speed-presets = Keine Geschwindigkeitsvorgaben.
# $name is the preset's name from the settings, such as "Study".
voice-speed-preset = { $name }, Tempo { $wpm }.
voice-line-numbers-on = Zeilennummern an.
voice-line-numbers-off = Zeilennummern aus.
