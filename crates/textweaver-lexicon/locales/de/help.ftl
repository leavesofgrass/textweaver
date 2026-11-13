### The help, key names, About, and the packaged guides.

## Keys named in messages and the help.

help-the-command-palette = die Befehlspalette
help-not-bound = nicht belegt
# Two keys, or a key and a list of keys: "p or Ctrl+P".
help-or = { $a } oder { $b }
# A command without keys: $name is its palette name, such as list highlights.
help-the-command = der Befehl { $name }
# One line of the keyboard shortcuts list: the command's name-* (first, so
# type-ahead finds commands), its keys (inside a 40-cell Braille line), an
# action-* help, and its category-* title.
help-entry = { $name }: { $keys }. { $help }. { $category }
help-unknown-command = Unbekannter Befehl: { $text }.
help-shortcuts-intro = Tastenkombinationen, { $n } Befehle. Tippen filtert. Bild ab springt zur nächsten Gruppe, F1 erklärt einen Befehl, Eingabetaste führt ihn aus, Escape schließt.
help-shortcuts-title = Tastenkombinationen
# The keyboard shortcuts list, filtered: $filter is what was typed.
help-shortcuts-title-matching = Tastenkombinationen zu { $filter }
# $n commands of $total match the filter.
help-shortcuts-filter-match = { $n } von { $total } Befehlen stimmen überein.
help-shortcuts-filter-none = Kein Befehl stimmt mit { $query } überein. Rücktaste entfernt Buchstaben.
help-shortcuts-filter-cleared = Filter gelöscht, { $n } Befehle.
# Moving into a group of the keyboard shortcuts list: its name, its
# size, then the row ($item, with its place in the list).
help-shortcuts-group-item =
    { $group }, { $n ->
        [one] 1 Befehl
       *[other] { $n } Befehle
    }. { $item }
help-title = Hilfe
help-intro = Hilfe. Tippen durchsucht die ganze Hilfe; Auf und Ab bewegen, Escape schließt.

## The help list. Each value is a key or keys from the keymap.

help-about = textweaver liest Dokumente vor. Die Tasten unten sind die aktuellen Belegungen.
help-search = Die ganze Hilfe durchsuchen, indem Sie hier tippen: Befehle, Tasten, Einstellungen und die Anleitungen.
help-open = Ein Dokument öffnen: { $open }. Bibliothek und zuletzt verwendete Dateien: { $library }.
help-play = Abspielen oder pausieren: { $key }.
help-read-from-cursor = Ab dem Cursor lesen: { $key }.
help-stop = Stopp: { $key }.
help-sentences = Nächster und vorheriger Satz: { $next } und { $previous }.
help-paragraphs = Nächster und vorheriger Absatz: { $next } und { $previous }.
help-headings = Nächste und vorherige Überschrift: { $next } und { $previous }. Überschrift auf einer Ebene: { $first } bis { $last }, mit Umschalt für die vorherige.
help-read-headings = Ab der nächsten und vorherigen Überschrift lesen: { $next } und { $previous }.
help-quick-keys = Schnelltasten, wie in NVDA und JAWS: Liste { $list }, Listenelement { $item }, Tabelle { $table }, Link { $link }, Blockzitat { $quote }, Trenner { $separator }, Grafik { $graphic }, Abschnitt { $section }. Umschalt mit der Taste geht zur vorherigen.
help-speech-cursor = Lesecursor, Zeile für Zeile: { $key }.
help-find = Suchen: { $key }.
help-bookmark = Lesezeichen hinzufügen: { $key }.
help-history = Zurück und vorwärts durch Ihre Sprünge: { $back } und { $forward }.
help-rate = Schneller und langsamer: { $faster } und { $slower }.
help-where = Wo bin ich: { $key }.
help-repeat = Die letzte Meldung noch einmal hören: { $repeat }. Die letzte Meldung und der Status: Modus, Geschwindigkeit, Engine und Position: { $status }.
help-notes = Notizen: hinzufügen { $add }, auflisten { $list }, nächste und vorherige { $next } und { $previous }, die am Cursor löschen { $delete }. In der Liste löscht Entf, F2 bearbeitet.
help-highlights = Die Auswahl oder den Satz hervorheben, oder eine Hervorhebung entfernen: { $highlight }. Mit einem Namen: { $first } bis { $fifth }, oder die Liste: { $named }. Hervorhebungen auflisten: { $list }.
help-bookmarks-list = Lesezeichenliste: Entf löscht ein Lesezeichen, F2 benennt es um.
help-edit = Das Dokument bearbeiten: { $edit }. Speichern: { $save }. Speichern unter: { $saveas }. Neues Dokument: { $new }.
help-editing = Beim Bearbeiten: rückgängig { $undo }, wiederholen { $redo }, fett { $bold }. Jeder Formatierungsbefehl steht in den Tastenkombinationen.
help-outline = Gliederung der Überschriften, Tippen filtert: { $outline }. Einem Link oder einer Fußnote folgen: { $follow }.
help-tables = Tabellen: { $nextrow } und { $previousrow } bewegen zeilenweise, { $nextcell } und { $previouscell } zellenweise.
help-citations = Zitate beim Bearbeiten: einfügen { $insert }, eine Literaturangabe per DOI, ISBN oder PMID hinzufügen { $reference }. Rechtschreibung: nächster und vorheriger Fehler { $next } und { $previous }, Vorschläge { $suggestions }.
help-export = Export als HTML, PDF, Word, EPUB oder Braille, Vorschau im Browser, und Start von einer Vorlage: Geben Sie export, preview oder template in die Befehlspalette ein.
help-verbosity = Wie viel gesagt wird: { $verbosity }. Wie viel Interpunktion: { $punctuation }.
help-voice = Eine Stimme wählen: { $voice }. Sprachausgabe neu starten, wenn sie aussetzt: { $restart }.
help-access = Mit einem Screenreader, wer spricht: { $key } wechselt zwischen Selbstsprechend, Hybrid und Screenreader-Modus.
help-access-window = Wer liest: { $key } wechselt zwischen textweaver liest vor und mein Screenreader liest.
help-character-keys = Einzeltasten-Kurzbefehle an oder aus, für Diktat: { $keys }. Einstellungen: { $settings }.
help-all-shortcuts = Alle Tastenkombinationen: { $key }.
help-palette = Jeden Befehl über seinen Namen ausführen: { $key }.
# Keep the letters y, n, and a: they are the keys that answer.
help-quit = Beenden, mit Speichern Ihrer Stelle: { $key }, dann y zum Bestätigen; n, a oder Escape bricht ab.

## Help categories.

category-reading = Lesen
category-navigation = Navigation
category-speech-cursor = Lesecursor
category-voice = Stimme
category-search = Suchen
category-bookmarks = Lesezeichen und Notizen
category-file = Datei
category-editing = Bearbeiten
category-view = Ansicht und Hilfe

## Key names as textweaver's own voice says them. Written names (Ctrl+S)
## are not translated.

keyname-control = Strg
keyname-command = Command
keyname-alt = Alt
keyname-shift = Umschalt
keyname-period = Punkt
keyname-comma = Komma
keyname-semicolon = Semikolon
keyname-colon = Doppelpunkt
keyname-apostrophe = Apostroph
keyname-quote = Anführungszeichen
keyname-grave-accent = Gravis
keyname-tilde = Tilde
keyname-exclamation-mark = Ausrufezeichen
keyname-question-mark = Fragezeichen
keyname-at-sign = At-Zeichen
keyname-number-sign = Rautezeichen
keyname-dollar-sign = Dollarzeichen
keyname-percent = Prozentzeichen
keyname-caret = Zirkumflex
keyname-ampersand = Kaufmanns-Und
keyname-asterisk = Sternchen
keyname-left-parenthesis = runde Klammer auf
keyname-right-parenthesis = runde Klammer zu
keyname-left-bracket = eckige Klammer auf
keyname-right-bracket = eckige Klammer zu
keyname-left-brace = geschweifte Klammer auf
keyname-right-brace = geschweifte Klammer zu
keyname-less-than = Kleiner-als-Zeichen
keyname-greater-than = Größer-als-Zeichen
keyname-plus = Plus
keyname-minus = Minus
keyname-equals = Gleichheitszeichen
keyname-underscore = Unterstrich
keyname-slash = Schrägstrich
keyname-backslash = Rückstrich
keyname-vertical-bar = senkrechter Strich
keyname-page-up = Bild auf
keyname-page-down = Bild ab
keyname-up-arrow = Pfeil nach oben
keyname-down-arrow = Pfeil nach unten
keyname-left-arrow = Pfeil nach links
keyname-right-arrow = Pfeil nach rechts
keyname-escape = Escape
keyname-space = Leertaste
keyname-enter = Eingabetaste
keyname-tab = Tabulator
keyname-backspace = Rücktaste
keyname-delete = Entf
keyname-insert = Einfg
keyname-home = Pos1
keyname-end = Ende

## Help's ways to the docs, About's facts, and first-run choices asked
## again. $address is a web address; $path a folder; facts start with
## their name so each Braille line leads with it.
about-title = Über textweaver
about-intro = Über textweaver, { $n } Angaben. Fügen Sie sie einer Fehlermeldung bei.
about-version = Version: textweaver { $version }
about-build = Build: { $frontend }, { $profile }, { $os } { $arch }
about-frontend-window = Fenster
about-frontend-terminal = Terminal-Leser
about-profile-release = Release
about-profile-debug = Debug
about-license = Lizenz: { $license }
about-engine-in-use = Sprachausgabe in Gebrauch: { $engine }
about-engines = Gefundene Sprachausgaben: { $engines }
about-engines-none = Gefundene Sprachausgaben: keine
about-components = Komponenten: { $installed } von { $known } installiert
about-folder-settings = Einstellungsordner: { $path }
about-folder-data = Datenordner: { $path }
about-folder-cache = Cache-Ordner: { $path }
about-no-folders = Ordner: keine; diese Sitzung speichert keine Dateien.
about-quick-start-online = Schnellstart nicht neben textweaver gefunden. { $address } im Browser öffnen? y oder n
about-docs-question = Dokumentation: { $address }. Im Browser öffnen? y oder n
about-report-question = Problem melden: { $address }. Nichts wird gesendet. Im Browser öffnen? y oder n
about-first-run-again = Fragen zum ersten Start zurückgesetzt; beim nächsten Start.

## B1-hp: die gelieferten Anleitungen und Hilfe durchsuchen.
docs-not-found = Dokumentation nicht neben textweaver gefunden. { $address } im Browser öffnen? y oder n
docs-read-only = Diese Anleitung gehört zu textweaver und ist schreibgeschützt. Um sie zu ändern, kopieren Sie die Datei an einen anderen Ort und öffnen die Kopie.
docs-guide-missing =
    { $n ->
        [one] Anleitung fehlt: { $guides }. Die anderen Anleitungen öffnen sich.
       *[other] { $n } Anleitungen fehlen: { $guides }. Die anderen Anleitungen öffnen sich.
    }
helpsearch-title = Hilfe durchsuchen
helpsearch-title-matching = Hilfe zu { $filter }
helpsearch-intro = Hilfe durchsuchen, { $n } Themen. Tippen durchsucht Befehle, Tasten, Einstellungen und die Anleitungen; Enter öffnet ein Thema, F1 erklärt es.
helpsearch-match =
    { $n ->
        [one] 1 Treffer.
       *[other] { $n } Treffer.
    }
helpsearch-none = Keine Hilfe zu { $query }. Die Rücktaste entfernt Buchstaben.
helpsearch-cleared =
    { $n ->
        [one] Filter gelöscht, 1 Thema.
       *[other] Filter gelöscht, { $n } Themen.
    }
helpsearch-command = { $row }, Befehl
helpsearch-setting = { $label }, Einstellung in { $section }
helpsearch-guide = { $heading }, in { $guide }
helpsearch-at-heading = Bei { $heading }.
## End of B1-hp
