### The study sheet, the self-test, and study cards.

## The study sheet.

notes-nothing-to-export = No notes or highlights to export.
# Keep the letters y and n: they are the keys that answer. $file is the
# sheet's file name, $folder the folder it was saved in.
notes-study-sheet-saved-notes =
    Study sheet with { $n ->
        [one] 1 note
       *[other] { $n } notes
    } saved as { $file }. Open it? y or n. In { $folder }.
notes-study-sheet-saved-highlights =
    Study sheet with { $h ->
        [one] 1 highlight
       *[other] { $h } highlights
    } saved as { $file }. Open it? y or n. In { $folder }.
notes-study-sheet-saved-both =
    Study sheet with { $n ->
        [one] 1 note
       *[other] { $n } notes
    } and { $h ->
        [one] 1 highlight
       *[other] { $h } highlights
    } saved as { $file }. Open it? y or n. In { $folder }.
notes-study-sheet-failed = Could not write the study sheet: { $error } Check that the folder can be written to.
# The study sheet file's own text (Markdown; the # marks stay in the code).
notes-sheet-title = Study sheet: { $title }
notes-sheet-exported = Exported from { -brand } on { $date }.
notes-sheet-before-first-heading = Before the first heading
# After a note's text: its tags, joined with commas.
notes-sheet-tags = (tags: { $tags })
# $name is the highlight's palette name.
notes-sheet-highlighted = Highlighted, { $name }.
# On the study sheet grouped by name: the heading a highlight falls under.
notes-sheet-under = Under: { $heading }
# The study sheet's section of notes, after the names.
notes-sheet-notes = Notes

## The self-test: prompts with hidden answers (crate::reveal).

reveal-self-test-title = Self-test: { $title }
reveal-self-test-intro =
    { $n ->
        [one] Self-test, 1 prompt. Enter shows the answer. Space to answer aloud.
       *[other] Self-test, { $n } prompts. Enter shows each answer. Space to answer aloud.
    }
reveal-nothing-to-test = No notes or highlights to test. Add a note or highlight first.
reveal-prompt-note = { $note } (in { $section })
reveal-prompt-highlight = What did you highlight in { $section }?
reveal-row-shown = { $prompt } Answer: { $answer }
reveal-answer = Answer: { $answer }
reveal-listening = Answer aloud now. Space to stop.
reveal-you-said = You said: { $words }. Enter shows the answer.
reveal-heard-nothing = No answer heard. Space to try again.
reveal-no-dictation = Answering aloud needs dictation, which is not in this version.

## Study cards and the study session (crate::cards).

# The word said in place of the blanked words of a cloze card.
cards-blank = blank
# A recall card's question. $heading is the heading's text.
cards-recall-question = What does “{ $heading }” say?
cards-no-document = Open a document to make or study cards.
cards-nothing-to-make = No notes or highlights to make cards from. Add a note or highlight first.
# $added is how many cards are new, $total how many the document has.
cards-made =
    { $added ->
        [0] No new cards. { $total } cards in all.
        [one] Cards made: 1 new, { $total } in all.
       *[other] Cards made: { $added } new, { $total } in all.
    }
cards-none-from-item = No card from this item: it needs text on a passage.
cards-made-one = Card made: { $question }
cards-updated-one = Card updated: { $question }
cards-none = No cards yet. To make them: { $key }.
cards-study-title = Study cards: { $title }
cards-study-intro =
    { $n ->
        [one] Study cards, 1 card. Enter shows the answer, 1 to 4 grade it. Space to answer aloud.
       *[other] Study cards, { $n } cards. Enter shows each answer, 1 to 4 grade it. Space to answer aloud.
    }
cards-no-session = No study session. To start one: { $key }.
cards-card-gone = That card was removed.
cards-grade-again = Again
cards-grade-hard = Hard
cards-grade-good = Good
cards-grade-easy = Easy
# Said after grading: the grade, then the next card.
cards-graded = { $grade }. Card { $i } of { $n }. Question: { $question }
# Said when a grade from the palette opens the session again on the next card.
cards-graded-reopen = { $grade }. Study cards, card { $i } of { $n }.
cards-session-done =
    { $n ->
        [one] { $grade }. Done: the card is graded.
       *[other] { $grade }. Done: all { $n } cards graded.
    }
cards-reversed = Reversed. Question: { $question }
cards-unreversed = As made. Question: { $question }
cards-not-reversible = Only question cards can be reversed.
cards-kind-cloze = Fill in the blank
cards-kind-question = Question
cards-kind-recall = Recall
cards-not-graded = not graded yet
cards-last-grade = last graded { $grade }
# An item in the Cards list.
cards-item = { $kind }: { $question }, { $grade }
cards-list-title = Cards
cards-list-intro =
    { $n ->
        [one] Cards, 1 item. Enter goes to its source, Delete removes it.
       *[other] Cards, { $n } items. Enter goes to a card's source, Delete removes it.
    }
cards-source-label = Card source
cards-remove-question = Remove this card and its grades? y or n
cards-removed = Card removed.
cards-save-failed = Could not save the cards: { $error } Check that the data folder can be written to.
# Anki packages (B1-f3). $file is the package's name, $folder where it is.
cards-anki-exported =
    { $n ->
        [one] Exported 1 card to Anki: { $file }, in { $folder }.
       *[other] Exported { $n } cards to Anki: { $file }, in { $folder }.
    }
cards-anki-export-failed = Could not export the cards to Anki: { $error } Check that the document's folder can be written to.
cards-anki-import-failed = Could not import { $file }: { $error } Check that the file is an Anki package, .apkg.
cards-anki-imported =
    { $added ->
        [one] Imported 1 card.
       *[other] Imported { $added } cards.
    }
cards-anki-already =
    { $n ->
        [one] 1 card was here already; its grades were merged.
       *[other] { $n } cards were here already; their grades were merged.
    }
cards-anki-skipped-cloze =
    { $n ->
        [one] Skipped 1 cloze note.
       *[other] Skipped { $n } cloze notes.
    }
cards-anki-skipped-media =
    { $n ->
        [one] Skipped 1 note that is only pictures or sound.
       *[other] Skipped { $n } notes that are only pictures or sound.
    }
cards-anki-skipped-other =
    { $n ->
        [one] Skipped 1 note with no text or with more than two cards.
       *[other] Skipped { $n } notes with no text or with more than two cards.
    }
cards-from-anki = This card came from Anki; it has no place in the document.
# Said first in the study session and the Cards list (B1-f2). $due is how
# many graded cards are due, $new how many were never graded.
cards-due-summary =
    { $due ->
        [one] Due today: 1 card, { $new } new.
       *[other] Due today: { $due } cards, { $new } new.
    }
# When a card is next due, after a grade. $days is whole days, at least 1.
cards-next-in =
    { $days ->
        [one] next tomorrow
       *[other] next in { $days } days
    }
cards-due-now = due today
# A grade or last grade, then when the card is next due: "Good, next in 3 days".
cards-grade-next = { $grade }, { $next }
# Study cards when no card is due and none is new: every card is asked.
cards-nothing-due =
    { $days ->
        [one] Nothing due today; the next card is due tomorrow. Studying every card ahead.
       *[other] Nothing due today; the next card is due in { $days } days. Studying every card ahead.
    }
