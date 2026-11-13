### The terminal reader.

## The terminal reader's startup.

# $wanted is the speech backend asked for, $backend the one used instead.
tui-setup-backend-unavailable = Sprachausgabe { $wanted } ist nicht verfügbar; { $backend } wird verwendet.
tui-setup-speech-failed = Die Sprachausgabe konnte nicht gestartet werden ({ $error }); läuft stumm weiter.
speech-engine-fallback = Es spricht { $engine }: { $failed } konnte nicht starten.
speech-engine-fallback-silent = Keine Sprache: { $failed } konnte nicht starten, und keine andere Sprachausgabe ist verfügbar.
tui-setup-cannot-save = Einstellungen oder Positionen können nicht gespeichert werden: { $error } Lesen funktioniert. Änderungen gehen beim Beenden verloren.
tui-setup-keymap-ignored = Tastenzuordnungsdatei ignoriert: { $error } Die Standardtasten gelten. Korrigieren Sie die Datei und starten Sie neu.
# The first-run welcome. Each value names the key for an action: $play
# reads and pauses, $stop stops, $heading moves to the next heading,
# $help opens the help, $quit quits.
tui-setup-welcome = Willkommen bei { -brand }. { $open } öffnet ein Dokument, { $help } die Hilfe. { $play } startet und pausiert, { $stop } stoppt, { $palette } listet alle Befehle.
# The window's first-run welcome on Windows, where the menu bar is
# hidden until Alt or $menu (F10) shows it: the keys of
# tui-setup-welcome, and the menus.
gui-setup-welcome-menus = Willkommen bei { -brand }. { $open } öffnet ein Dokument, { $help } die Hilfe. { $play } startet und pausiert, { $stop } stoppt, { $palette } listet alle Befehle. Alt oder { $menu } zeigt die Menüs.
# Said at startup without a document. $open, $new, and $help name the
# keys for Open, New Document, and Help.
tui-setup-no-document = Kein Dokument ist geöffnet. Drücken Sie { $open }, um eines zu öffnen, { $new } für ein neues, oder { $help } für Hilfe.

## The terminal reader's launch.

# $name is the file asked for on the command line; $error says why.
tui-could-not-open = { $name } konnte nicht geöffnet werden: { $error }

## Copying in the terminal reader.

tui-clip-system = Mit der Systemzwischenablage kopiert, weil dieses Terminal keinen kopierten Text annehmen kann.
# $error is why: tui-clip-not-available, tui-clip-not-built, or the
# system's own message.
tui-clip-failed = Konnte nicht mit der Systemzwischenablage kopieren: { $error }. Stattdessen an das Terminal gesendet.
tui-clip-not-available = die Systemzwischenablage ist nicht verfügbar
tui-clip-not-built = diese Version hat keine Systemzwischenablage

## The terminal reader's screen.

# The title line's start; $title is the document's title or
# tui-title-no-document.
tui-title = { -brand }: { $title }
tui-title-no-document = kein Dokument
# The screen without a document. $keys names the keys for the action.
tui-empty-open = Eines öffnen: { $keys }.
tui-empty-help = Hilfe: { $keys }.
tui-empty-quit = Beenden: { $keys }.
# The key hints while a yes-or-no question waits. y, n, and a are the
# answer keys the reader takes; $escape names the Escape key.
tui-hints-confirm = y ja  n oder a nein  { $escape } nein
# Key hint labels, each shown after its key on the bottom line.
tui-hint-play = abspielen
tui-hint-sentence = Satz
tui-hint-faster = schneller
tui-hint-slower = langsamer
tui-hint-close-rsvp = RSVP schließen
tui-hint-quit = beenden
tui-hint-save = speichern
tui-hint-finish = beenden
tui-hint-undo = rückgängig
tui-hint-bold = fett
tui-hint-heading = Überschrift
tui-hint-commands = Befehle
tui-hint-next-line = nächste Zeile
tui-hint-previous-line = vorherige Zeile
tui-hint-again = nochmal
tui-hint-read-on = weiterlesen
tui-hint-leave = verlassen
tui-hint-find = suchen
tui-hint-mark = markieren
tui-hint-lines = Zeilen
tui-hint-keys = Tasten
tui-hint-choose = auswählen
tui-hint-close = schließen
tui-hint-back = zurück
# The list overlay's border: $n is the focused item's number, $count
# the number of items.
tui-list-title = { $n } von { $count }, { $title }
