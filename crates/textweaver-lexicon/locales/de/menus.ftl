### Menus, the command palette, and interface announcements.

## Menus, the command palette, interface announcements, colors, and settings

## Menu titles; the top menus mark their access key with &.

menu-file = &Datei
menu-edit = &Bearbeiten
menu-view = &Ansicht
menu-reading = &Lesen
menu-speech = &Sprache
menu-tools = E&xtras
menu-help = &Hilfe
menu-recent = Zuletzt geöffnet
menu-export-as = E&xportieren als
menu-preview = Vorschau
menu-settings = Einstellungen
menu-find = Suchen
menu-format = Format
menu-insert = Einfügen
menu-proofing = Korrektur
menu-citations = Zitate
menu-text-size = Textgröße
menu-reading-aids = Lesehilfen
menu-rsvp = RSVP
menu-say = Sagen
menu-move-by = Springen nach
menu-headings = Überschriften
menu-go-to = Gehe zu
menu-cursor = Cursor und Auswahl
menu-bookmarks = Lesezeichen und Notizen
menu-highlights = Hervorhebungen
menu-tables = Tabellen
menu-speech-cursor = Sprachcursor

## Menus, the palette, and interface announcements.

menu-bar = Menüs
menu-title = Menü { $name }
menu-submenu = { $name }, Untermenü
menu-checked = { $name }, aktiviert
menu-not-checked = { $name }, nicht aktiviert
menu-value = { $name }: { $value }
menu-with-keys = { $item }, { $keys }
menu-recent-document = { $name }, { $pct } Prozent
menu-recent-none = Keine zuletzt geöffneten Dokumente
menu-not-available = { $name } gibt es in dieser Version nicht.
menu-no-access-key = Kein Eintrag mit der Taste { $letter }.
menu-closed = Menüs geschlossen.
menu-context = Kontextmenü
menu-context-closed = Kontextmenü geschlossen.
menu-press-a-key = Drücken Sie eine Taste, um zu hören, was sie tut.
menu-key-described = { $name }: { $help }. Tasten: { $keys }. In den Menüs: { $path }.
menu-key-described-no-menu = { $name }: { $help }. Tasten: { $keys }.
menu-keys = { $item }. Eingabe oder Rechts öffnet ein Menü oder führt einen Befehl aus, ein Buchstabe springt zu seinem Eintrag, Links oder Rücktaste geht zurück, Escape schließt.
edit-line-continues = Zeile geht weiter
announce-level-changed = Oberflächenansagen: { $level }.
announce-level-off = aus
announce-level-minimal = minimal
announce-level-normal = normal
announce-level-full = vollständig
palette-item = { $name }, { $keys }
palette-item-no-keys = { $name }
palette-item-recent = { $name }, { $keys }, zuletzt
palette-item-recent-no-keys = { $name }, zuletzt
palette-list-title = Befehle zu { $query }
palette-list-title-all = Befehle
palette-list-intro =
    { $title }, { $n ->
        [one] 1 Befehl
       *[other] { $n } Befehle
    }. Eingabe führt einen aus, F1 erklärt ihn.
color-name-theme = Farbe des Designs
color-name-blue = Blau
color-name-orange = Orange
color-name-navy = Dunkelblau
color-name-skyblue = Himmelblau
color-name-teal = Petrol
color-name-gold = Gold
color-name-yellow = Gelb
color-name-green = Grün
color-name-cyan = Cyan
color-name-purple = Lila
color-name-pink = Rosa
color-name-brown = Braun
color-name-gray = Grau
color-name-black = Schwarz
color-name-white = Weiß
section-colors = Farben
colors-contrast-good = gut
colors-contrast-fair = ausreichend
colors-contrast-low = gering
colors-item = { $label }: { $value }, Kontrast { $ratio } zu 1, { $verdict }
colors-contrast = Kontrast { $ratio } zu 1, { $verdict }.
colors-contrast-warning = Unter 3 zu 1 ist das schwer zu sehen; wählen Sie eine hellere oder dunklere Farbe.
colors-palette-label = Hervorhebungsname { $name }
colors-palette-value = { $color }, { $shape }, Kontrast { $text } zu 1 zum Text und { $page } zu 1 zur Seite, { $verdict }
colors-palette-system = { $color }, { $shape }; die Kontrastfarben des Systems werden gezeichnet, die Form kennzeichnet sie weiterhin
colors-palette-unknown = { $color }, { $shape }; eine Farbe, die textweaver nicht messen kann
colors-intro = Farben, { $n } Einstellungen. Links und Rechts wählen eine benannte Farbe, Eingabe tippt einen Namen oder einen #rrggbb-Wert, Entf stellt die Farbe des Designs wieder her, F1 sagt die Hilfe.
settings-item-recent = { $item }, kürzlich geändert
settings-reset = { $label } wieder auf dem Standard, { $value }.
settings-row-help = { $label }: { $value }. Standard: { $default }. { $help }
number-group-separator = .
number-decimal-separator = ,
settingsio-import-question-names =
    { $n ->
        [one] { $n } geänderte Einstellung aus { $name } importieren: { $names }? y oder n
       *[other] { $n } geänderte Einstellungen aus { $name } importieren: { $names }? y oder n
    }
settingsio-and-more = { $names } und { $n } weitere
