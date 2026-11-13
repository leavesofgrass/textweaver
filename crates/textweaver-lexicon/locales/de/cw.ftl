### CW (Morse code): the speech engine, practice, and keying.

## Beta 4: CW (Morse code) as a speech engine.
cw-sending = CW: { $text }
cw-speed = Zeichentempo { $wpm }, effektiv { $effective } Wörter pro Minute.
cw-skipped =
    { $count ->
        [one] 1 Zeichen nicht als Morsecode gesendet.
       *[other] { $count } Zeichen nicht als Morsecode gesendet.
    }

## Beta 4: CW practice (Koch method, random drills, copy checking).
cw-practice-koch-start = Koch-Lektion { $lesson } von 40, { $count } Zeichen, neu: { $new }.
cw-practice-characters-start = Zufallszeichen, { $groups } Gruppen.
cw-practice-words-start = Zufallswörter, { $groups } Gruppen.
cw-practice-group = Gruppe { $n } von { $groups }.
cw-practice-again = Noch einmal, Gruppe { $n } von { $groups }.
cw-practice-perfect = { $correct } von { $total }, alles richtig.
cw-practice-score = { $correct } von { $total }. Fehler: { $missed }. Gesendet: { $sent }.
cw-practice-wrong = { $sent } als { $typed }
cw-practice-missing = { $sent } nicht mitgeschrieben
cw-practice-extra = { $typed } zu viel
cw-practice-round = Runde: { $correct } von { $total }, { $percent } Prozent.
cw-practice-added = Lektion { $lesson }: { $new } hinzugefügt.
cw-practice-not-yet = Ab { $threshold } Prozent kommt das nächste Zeichen hinzu.
cw-practice-all-learned = Alle Zeichen gelernt.
cw-practice-stopped = Übung beendet.
cw-practice-progress = Koch-Lektion { $lesson } von 40, neu: { $new }. { $recent }
cw-practice-recent = Letzte Runden: { $scores } Prozent.
cw-practice-no-rounds = Noch keine Runden.
cw-practice-unavailable = Die CW-Ausgabe ist nicht gestartet, daher kann die Übung nicht senden.
cw-char-period = Punkt
cw-char-comma = Komma
cw-char-equals = Gleichheitszeichen
cw-char-slash = Schrägstrich
cw-char-question = Fragezeichen

## Beta 4: CW keying, Morse code typed with a key or paddle. $key is a
## key name, $name a MIDI input's name.
cw-keying-on = CW-Morsegeben ein. Morsecode geben, um an der Cursorposition zu schreiben; { $key } schaltet es aus.
cw-keying-off = CW-Morsegeben aus.
cw-keying-ended = CW-Morsegeben aus, weil der Bearbeitungsmodus beendet wurde.
cw-keying-needs-edit = CW-Morsegeben schreibt im Bearbeitungsmodus. Zuerst den Bearbeitungsmodus einschalten, mit { $key }.
cw-keying-listening = CW-Morsegeben: hört auf { $name }.
cw-keying-midi-failed = CW-Morsegeben kann { $name } nicht öffnen: { $error } Prüfen Sie die Verbindung und schalten Sie das Morsegeben erneut ein.
cw-keying-no-midi = Kein MIDI-Tastenadapter gefunden; verbinden oder koppeln Sie ihn und schalten Sie das Morsegeben erneut ein. Tastatur-Adapter funktionieren in der { -brand }-App.
cw-keying-midi-not-in-build = MIDI ist in dieser Version von { -brand } nicht enthalten. Tastatur-Adapter funktionieren in der { -brand }-App.
cw-keying-no-sidetone = Kein Mithörton: { $error } Das Morsegeben geht ohne ihn weiter; nutzen Sie den Mithörton des Adapters oder Funkgeräts.
cw-keying-unknown = Kein Morsezeichen.
cw-keying-erased = Irrungszeichen: { $text } gelöscht.
cw-keying-nothing-erased = Irrungszeichen: nichts Gegebenes zu löschen.
cw-keying-rsvp-on = RSVP zeigt die gegebenen Wörter.
