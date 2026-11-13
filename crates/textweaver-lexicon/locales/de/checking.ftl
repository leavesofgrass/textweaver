### Markdown lint, grammar, and spelling.

## Markdown lint (edit mode). A problem is said after "Lint: ".

lint-heading-level = Überschriftsebene { $level } nach Ebene { $prev }; verwenden Sie Ebene { $use }.
# $reference is the link reference's name.
lint-link-reference = Der Linkverweis { $reference } hat keine Definition.
lint-bare-url = Nackte Webadresse; setzen Sie sie in spitze Klammern oder machen Sie einen benannten Link daraus.
# $marker and $used are bullet names: lint-marker-dash and the others.
lint-list-marker = Listenzeichen { $marker }; diese Liste verwendet { $used }.
lint-marker-dash = Gedankenstrich
lint-marker-star = Sternchen
lint-marker-plus = Pluszeichen
lint-marker-other = anderes
# $n is how many tabs and spaces there are.
lint-trailing-tabs-empty-line = Tabulatoren oder Leerzeichen auf einer leeren Zeile.
lint-trailing-tabs-line-end = Tabulatoren oder Leerzeichen am Zeilenende.
lint-trailing-spaces-empty-line =
    { $n ->
        [one] 1 Leerzeichen auf einer leeren Zeile.
       *[other] { $n } Leerzeichen auf einer leeren Zeile.
    }
lint-trailing-spaces-line-end =
    { $n ->
        [one] 1 Leerzeichen am Zeilenende.
       *[other] { $n } Leerzeichen am Zeilenende.
    }
# $key turns on edit mode.
lint-not-editing = Lint prüft das Markdown, das Sie schreiben. Schalten Sie zuerst den Bearbeitungsmodus mit { $key } ein.
lint-not-markdown = Lint prüft Markdown, und dieses Dokument ist kein Markdown.
lint-none = Keine Lint-Probleme.
# $count is $n with thousands separators.
lint-no-more =
    { $n ->
        [one] Kein weiteres Lint-Problem. 1 Lint-Problem insgesamt.
       *[other] Kein weiteres Lint-Problem. { $count } Lint-Probleme insgesamt.
    }
lint-no-earlier =
    { $n ->
        [one] Kein früheres Lint-Problem. 1 Lint-Problem insgesamt.
       *[other] Kein früheres Lint-Problem. { $count } Lint-Probleme insgesamt.
    }
# $message is one of the problems above.
lint-said = Lint: { $message }
# Added at high verbosity.
common-line = Zeile { $line }.

## Grammar checking (Harper). $message is Harper's own message, in English.

# $words are the words the problem is about.
grammar-said = Grammatik: { $message } Die Wörter: { $words }.
# Said after grammar-said when the first fix removes the words.
grammar-fix-remove = Korrektur: entfernen.
grammar-fix = Korrektur: { $fix }.
# A fix in the fixes list that removes the words.
grammar-remove-the-words = Die Wörter entfernen
grammar-none = Keine Grammatikprobleme gefunden.
# $count is $n with thousands separators.
grammar-no-more =
    { $n ->
        [one] Kein weiteres Grammatikproblem. 1 Grammatikproblem insgesamt.
       *[other] Kein weiteres Grammatikproblem. { $count } Grammatikprobleme insgesamt.
    }
grammar-no-earlier =
    { $n ->
        [one] Kein früheres Grammatikproblem. 1 Grammatikproblem insgesamt.
       *[other] Kein früheres Grammatikproblem. { $count } Grammatikprobleme insgesamt.
    }
# $key opens the fixes list.
grammar-lists-fixes = { $key } listet Korrekturen auf.
# Added at high verbosity.
# $described is grammar-said (and its fix) without the last full stop.
grammar-no-fix = { $described } Keine Korrektur verfügbar.
grammar-fixes =
    { $n ->
        [one] { $words }: 1 Korrektur.
       *[other] { $words }: { $n } Korrekturen.
    }
grammar-fixes-edit =
    { $n ->
        [one] { $words }: 1 Korrektur. Eingabetaste nimmt die Änderung vor.
       *[other] { $words }: { $n } Korrekturen. Eingabetaste nimmt die Änderung vor.
    }
common-left-as-is = So belassen, wie es ist.
# $fix is the fix chosen; $key turns on edit mode.
grammar-fix-not-editing = { $fix }. Schalten Sie den Bearbeitungsmodus mit { $key } ein, um den Text zu ändern.
grammar-removed = Entfernt.
grammar-changed = Geändert zu { $fix }.
grammar-change-failed = Der Text konnte nicht geändert werden: { $error } Nichts wurde geändert.

## Spell checking.

# Said for an apostrophe when a word is spelled out letter by letter.
spell-apostrophe = Apostroph
spell-not-available = Rechtschreibprüfung ist nicht verfügbar: diese Version hat keine Wortliste.
spell-none-found = Keine Rechtschreibfehler gefunden.
# $count is $n with thousands separators.
spell-no-more =
    { $n ->
        [one] Kein weiterer Rechtschreibfehler. 1 möglicher Rechtschreibfehler insgesamt.
       *[other] Kein weiterer Rechtschreibfehler. { $count } mögliche Rechtschreibfehler insgesamt.
    }
spell-no-earlier =
    { $n ->
        [one] Kein früherer Rechtschreibfehler. 1 möglicher Rechtschreibfehler insgesamt.
       *[other] Kein früherer Rechtschreibfehler. { $count } mögliche Rechtschreibfehler insgesamt.
    }
# Added at high verbosity.
spell-no-misspelled-word = Kein falsch geschriebenes Wort am Cursor.
# $word is the misspelled word; $n how many suggestions follow.
spell-suggestions =
    { $n ->
        [0] { $word }: keine Vorschläge.
        [one] { $word }: 1 Vorschlag.
       *[other] { $word }: { $n } Vorschläge.
    }
spell-suggestions-edit =
    { $n ->
        [0] { $word }: keine Vorschläge. Eingabetaste ersetzt das Wort.
        [one] { $word }: 1 Vorschlag. Eingabetaste ersetzt das Wort.
       *[other] { $word }: { $n } Vorschläge. Eingabetaste ersetzt das Wort.
    }
# $word is the suggestion chosen; $key turns on edit mode.
spell-replace-not-editing = { $word }. Schalten Sie den Bearbeitungsmodus mit { $key } ein, um den Text zu ändern.
spell-replaced = Ersetzt durch { $word }.
spell-replace-failed = Konnte nicht ersetzen: { $error } Das Wort ist unverändert.
spell-added-for-session = { $word } für diese Sitzung zu Ihrer Wortliste hinzugefügt.
spell-added = { $word } zu Ihrer Wortliste hinzugefügt.
spell-save-failed = Ihre Wortliste konnte nicht gespeichert werden: { $error } Das Wort gilt bis zum Beenden als bekannt.
# After a save; $count is $n with thousands separators.
spell-count =
    { $n ->
        [0] Keine Rechtschreibfehler.
        [one] 1 möglicher Rechtschreibfehler.
       *[other] { $count } mögliche Rechtschreibfehler.
    }
