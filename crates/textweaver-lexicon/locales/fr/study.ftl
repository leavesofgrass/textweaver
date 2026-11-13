### The study sheet, the self-test, and study cards.

## The study sheet.

notes-nothing-to-export = Aucune note ou surlignage à exporter.
# Keep the letters y and n: they are the keys that answer. $file is the
# sheet's file name, $folder the folder it was saved in.
notes-study-sheet-saved-notes =
    Fiche d'étude avec { $n ->
        [one] 1 note
       *[other] { $n } notes
    } enregistrée sous { $file }. L'ouvrir ? y ou n. Dans { $folder }.
notes-study-sheet-saved-highlights =
    Fiche d'étude avec { $h ->
        [one] 1 surlignage
       *[other] { $h } surlignages
    } enregistrée sous { $file }. L'ouvrir ? y ou n. Dans { $folder }.
notes-study-sheet-saved-both =
    Fiche d'étude avec { $n ->
        [one] 1 note
       *[other] { $n } notes
    } et { $h ->
        [one] 1 surlignage
       *[other] { $h } surlignages
    } enregistrée sous { $file }. L'ouvrir ? y ou n. Dans { $folder }.
notes-study-sheet-failed = Impossible d'écrire la fiche d'étude : { $error } Vérifiez que le dossier est accessible en écriture.
# The study sheet file's own text (Markdown; the # marks stay in the code).
notes-sheet-title = Fiche d'étude : { $title }
notes-sheet-exported = Exporté depuis { -brand } le { $date }.
notes-sheet-before-first-heading = Avant le premier titre
# After a note's text: its tags, joined with commas.
notes-sheet-tags = (étiquettes : { $tags })
# $name is the highlight's palette name.
notes-sheet-highlighted = Surligné, { $name }.
# On the study sheet grouped by name: the heading a highlight falls under.
notes-sheet-under = Sous : { $heading }
# The study sheet's section of notes, after the names.
notes-sheet-notes = Notes

## L’autotest : questions aux réponses cachées (crate::reveal).

reveal-self-test-title = Autotest : { $title }
reveal-self-test-intro =
    { $n ->
        [one] Autotest, 1 question. Entrée montre la réponse. Espace pour répondre à voix haute.
       *[other] Autotest, { $n } questions. Entrée montre chaque réponse. Espace pour répondre à voix haute.
    }
reveal-nothing-to-test = Aucune note ni surlignage à tester. Ajoutez d’abord une note ou un surlignage.
reveal-prompt-note = { $note } (dans { $section })
reveal-prompt-highlight = Qu’avez-vous surligné dans { $section } ?
reveal-row-shown = { $prompt } Réponse : { $answer }
reveal-answer = Réponse : { $answer }
reveal-listening = Répondez à voix haute maintenant. Espace pour arrêter.
reveal-you-said = Vous avez dit : { $words }. Entrée montre la réponse.
reveal-heard-nothing = Aucune réponse entendue. Espace pour réessayer.
reveal-no-dictation = Répondre à voix haute demande la dictée, absente de cette version.

## Cartes d’étude et la séance d’étude (crate::cards).

cards-blank = blanc
cards-recall-question = Que dit « { $heading } » ?
cards-no-document = Ouvrez un document pour créer ou étudier des cartes.
cards-nothing-to-make = Aucune note ni aucun surlignage pour créer des cartes. Ajoutez d’abord une note ou un surlignage.
cards-made =
    { $added ->
        [0] Aucune nouvelle carte. { $total } cartes en tout.
        [one] Cartes créées : 1 nouvelle, { $total } en tout.
       *[other] Cartes créées : { $added } nouvelles, { $total } en tout.
    }
cards-none-from-item = Aucune carte pour cet élément : il lui faut un texte sur un passage.
cards-made-one = Carte créée : { $question }
cards-updated-one = Carte mise à jour : { $question }
cards-none = Pas encore de cartes. Pour en créer : { $key }.
cards-study-title = Étudier les cartes : { $title }
cards-study-intro =
    { $n ->
        [one] Étudier les cartes, 1 carte. Entrée montre la réponse, 1 à 4 la notent. Espace pour répondre à voix haute.
       *[other] Étudier les cartes, { $n } cartes. Entrée montre chaque réponse, 1 à 4 la notent. Espace pour répondre à voix haute.
    }
cards-no-session = Aucune séance d’étude. Pour en commencer une : { $key }.
cards-card-gone = Cette carte a été supprimée.
cards-grade-again = À revoir
cards-grade-hard = Difficile
cards-grade-good = Bien
cards-grade-easy = Facile
cards-graded = { $grade }. Carte { $i } sur { $n }. Question : { $question }
# Said when a grade from the palette opens the session again on the next card.
cards-graded-reopen = { $grade }. Étudier les cartes, carte { $i } sur { $n }.
cards-session-done =
    { $n ->
        [one] { $grade }. Terminé : la carte est notée.
       *[other] { $grade }. Terminé : les { $n } cartes sont notées.
    }
cards-reversed = Inversée. Question : { $question }
cards-unreversed = Comme créée. Question : { $question }
cards-not-reversible = Seules les cartes question peuvent être inversées.
cards-kind-cloze = Texte à trou
cards-kind-question = Question
cards-kind-recall = Rappel
cards-not-graded = pas encore notée
cards-last-grade = dernière note : { $grade }
cards-item = { $kind } : { $question }, { $grade }
cards-list-title = Cartes
cards-list-intro =
    { $n ->
        [one] Cartes, 1 élément. Entrée va à sa source, Suppr la supprime.
       *[other] Cartes, { $n } éléments. Entrée va à la source d’une carte, Suppr la supprime.
    }
cards-source-label = Source de la carte
cards-remove-question = Supprimer cette carte et ses notes ? y ou n
cards-removed = Carte supprimée.
cards-save-failed = Impossible d’enregistrer les cartes : { $error } Vérifiez que le dossier des données est accessible en écriture.
cards-anki-exported =
    { $n ->
        [one] 1 carte exportée vers Anki : { $file }, dans { $folder }.
       *[other] { $n } cartes exportées vers Anki : { $file }, dans { $folder }.
    }
cards-anki-export-failed = Impossible d’exporter les cartes vers Anki : { $error } Vérifiez que le dossier du document est accessible en écriture.
cards-anki-import-failed = Impossible d’importer { $file } : { $error } Vérifiez que le fichier est un paquet Anki, .apkg.
cards-anki-imported =
    { $added ->
        [one] 1 carte importée.
       *[other] { $added } cartes importées.
    }
cards-anki-already =
    { $n ->
        [one] 1 carte était déjà là ; ses notes ont été fusionnées.
       *[other] { $n } cartes étaient déjà là ; leurs notes ont été fusionnées.
    }
cards-anki-skipped-cloze =
    { $n ->
        [one] 1 note à trous ignorée.
       *[other] { $n } notes à trous ignorées.
    }
cards-anki-skipped-media =
    { $n ->
        [one] 1 note faite seulement d’images ou de son ignorée.
       *[other] { $n } notes faites seulement d’images ou de son ignorées.
    }
cards-anki-skipped-other =
    { $n ->
        [one] 1 note sans texte ou à plus de deux cartes ignorée.
       *[other] { $n } notes sans texte ou à plus de deux cartes ignorées.
    }
cards-from-anki = Cette carte vient d’Anki ; elle n’a pas de place dans le document.
# Said first in the study session and the Cards list (B1-f2). $due is how
# many graded cards are due, $new how many were never graded.
cards-due-summary =
    { $due ->
        [one] À revoir aujourd’hui : 1 carte, { $new } nouvelles.
       *[other] À revoir aujourd’hui : { $due } cartes, { $new } nouvelles.
    }
# When a card is next due, after a grade. $days is whole days, at least 1.
cards-next-in =
    { $days ->
        [one] à revoir demain
       *[other] à revoir dans { $days } jours
    }
cards-due-now = à revoir aujourd’hui
# A grade or last grade, then when the card is next due: "Good, next in 3 days".
cards-grade-next = { $grade }, { $next }
# Study cards when no card is due and none is new: every card is asked.
cards-nothing-due =
    { $days ->
        [one] Rien à revoir aujourd’hui ; la prochaine carte est pour demain. Toutes les cartes sont étudiées en avance.
       *[other] Rien à revoir aujourd’hui ; la prochaine carte est dans { $days } jours. Toutes les cartes sont étudiées en avance.
    }
