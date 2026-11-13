### The study sheet, the self-test, and study cards.

## The study sheet.

notes-nothing-to-export = Keine Notizen oder Hervorhebungen zu exportieren.
# Keep the letters y and n: they are the keys that answer. $file is the
# sheet's file name, $folder the folder it was saved in.
notes-study-sheet-saved-notes =
    Lernblatt mit { $n ->
        [one] 1 Notiz
       *[other] { $n } Notizen
    } gespeichert als { $file }. Öffnen? y oder n. In { $folder }.
notes-study-sheet-saved-highlights =
    Lernblatt mit { $h ->
        [one] 1 Hervorhebung
       *[other] { $h } Hervorhebungen
    } gespeichert als { $file }. Öffnen? y oder n. In { $folder }.
notes-study-sheet-saved-both =
    Lernblatt mit { $n ->
        [one] 1 Notiz
       *[other] { $n } Notizen
    } und { $h ->
        [one] 1 Hervorhebung
       *[other] { $h } Hervorhebungen
    } gespeichert als { $file }. Öffnen? y oder n. In { $folder }.
notes-study-sheet-failed = Das Lernblatt konnte nicht geschrieben werden: { $error } Prüfen Sie, ob in den Ordner geschrieben werden kann.
# The study sheet file's own text (Markdown; the # marks stay in the code).
notes-sheet-title = Lernblatt: { $title }
notes-sheet-exported = Exportiert aus { -brand } am { $date }.
notes-sheet-before-first-heading = Vor der ersten Überschrift
# After a note's text: its tags, joined with commas.
notes-sheet-tags = (Tags: { $tags })
# $name is the highlight's palette name.
notes-sheet-highlighted = Hervorgehoben, { $name }.
# On the study sheet grouped by name: the heading a highlight falls under.
notes-sheet-under = Unter: { $heading }
# The study sheet's section of notes, after the names.
notes-sheet-notes = Notizen

## Der Selbsttest: Fragen mit verdeckten Antworten (crate::reveal).

reveal-self-test-title = Selbsttest: { $title }
reveal-self-test-intro =
    { $n ->
        [one] Selbsttest, 1 Frage. Eingabe zeigt die Antwort. Leertaste, um laut zu antworten.
       *[other] Selbsttest, { $n } Fragen. Eingabe zeigt jede Antwort. Leertaste, um laut zu antworten.
    }
reveal-nothing-to-test = Keine Notizen oder Hervorhebungen zum Abfragen. Fügen Sie zuerst eine Notiz oder Hervorhebung hinzu.
reveal-prompt-note = { $note } (in { $section })
reveal-prompt-highlight = Was haben Sie in { $section } hervorgehoben?
reveal-row-shown = { $prompt } Antwort: { $answer }
reveal-answer = Antwort: { $answer }
reveal-listening = Antworten Sie jetzt laut. Leertaste zum Beenden.
reveal-you-said = Sie sagten: { $words }. Eingabe zeigt die Antwort.
reveal-heard-nothing = Keine Antwort gehört. Leertaste, um es erneut zu versuchen.
reveal-no-dictation = Laut antworten braucht das Diktat, das in diesem Build fehlt.

## Lernkarten und die Lernsitzung (crate::cards).

cards-blank = Lücke
cards-recall-question = Was steht unter „{ $heading }“?
cards-no-document = Öffnen Sie ein Dokument, um Karten zu erstellen oder zu lernen.
cards-nothing-to-make = Keine Notizen oder Hervorhebungen für Karten. Fügen Sie zuerst eine Notiz oder Hervorhebung hinzu.
cards-made =
    { $added ->
        [0] Keine neuen Karten. { $total } Karten insgesamt.
        [one] Karten erstellt: 1 neu, { $total } insgesamt.
       *[other] Karten erstellt: { $added } neu, { $total } insgesamt.
    }
cards-none-from-item = Keine Karte aus diesem Eintrag: Er braucht Text zu einer Passage.
cards-made-one = Karte erstellt: { $question }
cards-updated-one = Karte aktualisiert: { $question }
cards-none = Noch keine Karten. Zum Erstellen: { $key }.
cards-study-title = Karten lernen: { $title }
cards-study-intro =
    { $n ->
        [one] Karten lernen, 1 Karte. Eingabe zeigt die Antwort, 1 bis 4 bewerten sie. Leertaste, um laut zu antworten.
       *[other] Karten lernen, { $n } Karten. Eingabe zeigt jede Antwort, 1 bis 4 bewerten sie. Leertaste, um laut zu antworten.
    }
cards-no-session = Keine Lernsitzung. Zum Starten: { $key }.
cards-card-gone = Diese Karte wurde entfernt.
cards-grade-again = Nochmal
cards-grade-hard = Schwer
cards-grade-good = Gut
cards-grade-easy = Leicht
cards-graded = { $grade }. Karte { $i } von { $n }. Frage: { $question }
# Said when a grade from the palette opens the session again on the next card.
cards-graded-reopen = { $grade }. Karten lernen, Karte { $i } von { $n }.
cards-session-done =
    { $n ->
        [one] { $grade }. Fertig: Die Karte ist bewertet.
       *[other] { $grade }. Fertig: Alle { $n } Karten bewertet.
    }
cards-reversed = Umgekehrt. Frage: { $question }
cards-unreversed = Wie erstellt. Frage: { $question }
cards-not-reversible = Nur Fragekarten lassen sich umkehren.
cards-kind-cloze = Lückentext
cards-kind-question = Frage
cards-kind-recall = Erinnern
cards-not-graded = noch nicht bewertet
cards-last-grade = zuletzt bewertet: { $grade }
cards-item = { $kind }: { $question }, { $grade }
cards-list-title = Karten
cards-list-intro =
    { $n ->
        [one] Karten, 1 Eintrag. Eingabe springt zur Quelle, Entf entfernt sie.
       *[other] Karten, { $n } Einträge. Eingabe springt zur Quelle einer Karte, Entf entfernt sie.
    }
cards-source-label = Quelle der Karte
cards-remove-question = Diese Karte und ihre Bewertungen entfernen? y oder n
cards-removed = Karte entfernt.
cards-save-failed = Die Karten konnten nicht gespeichert werden: { $error } Prüfen Sie, ob in den Datenordner geschrieben werden kann.
cards-anki-exported =
    { $n ->
        [one] 1 Karte nach Anki exportiert: { $file }, in { $folder }.
       *[other] { $n } Karten nach Anki exportiert: { $file }, in { $folder }.
    }
cards-anki-export-failed = Die Karten konnten nicht nach Anki exportiert werden: { $error } Prüfen Sie, ob in den Ordner des Dokuments geschrieben werden kann.
cards-anki-import-failed = { $file } konnte nicht importiert werden: { $error } Prüfen Sie, ob die Datei ein Anki-Paket (.apkg) ist.
cards-anki-imported =
    { $added ->
        [one] 1 Karte importiert.
       *[other] { $added } Karten importiert.
    }
cards-anki-already =
    { $n ->
        [one] 1 Karte war schon da; ihre Bewertungen wurden zusammengeführt.
       *[other] { $n } Karten waren schon da; ihre Bewertungen wurden zusammengeführt.
    }
cards-anki-skipped-cloze =
    { $n ->
        [one] 1 Lückentext-Notiz übersprungen.
       *[other] { $n } Lückentext-Notizen übersprungen.
    }
cards-anki-skipped-media =
    { $n ->
        [one] 1 Notiz übersprungen, die nur aus Bildern oder Ton besteht.
       *[other] { $n } Notizen übersprungen, die nur aus Bildern oder Ton bestehen.
    }
cards-anki-skipped-other =
    { $n ->
        [one] 1 Notiz ohne Text oder mit mehr als zwei Karten übersprungen.
       *[other] { $n } Notizen ohne Text oder mit mehr als zwei Karten übersprungen.
    }
cards-from-anki = Diese Karte stammt aus Anki; sie hat keine Stelle im Dokument.
# Said first in the study session and the Cards list (B1-f2). $due is how
# many graded cards are due, $new how many were never graded.
cards-due-summary =
    { $due ->
        [one] Heute fällig: 1 Karte, { $new } neu.
       *[other] Heute fällig: { $due } Karten, { $new } neu.
    }
# When a card is next due, after a grade. $days is whole days, at least 1.
cards-next-in =
    { $days ->
        [one] wieder morgen
       *[other] wieder in { $days } Tagen
    }
cards-due-now = heute fällig
# A grade or last grade, then when the card is next due: "Good, next in 3 days".
cards-grade-next = { $grade }, { $next }
# Study cards when no card is due and none is new: every card is asked.
cards-nothing-due =
    { $days ->
        [one] Heute ist nichts fällig; die nächste Karte ist morgen fällig. Alle Karten werden vorab gelernt.
       *[other] Heute ist nichts fällig; die nächste Karte ist in { $days } Tagen fällig. Alle Karten werden vorab gelernt.
    }
