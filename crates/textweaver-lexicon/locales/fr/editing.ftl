### Edit mode: typing, formatting, saving, recovery, and templates.

## Edit mode: entering, leaving, saving, and typing.

# $key makes a new document.
edit-no-document = Aucun document à modifier. Appuyez sur { $key } pour en créer un.
# $line is the line at the caret, as echoed.
edit-mode-on-brief = Mode édition activé. { $line }
# $save and $finish are the keys that save and leave edit mode; $line is
# the line at the caret.
edit-mode-on = Mode édition activé. Enregistrer : { $save }. Terminer : { $finish }. { $line }
# Keep the letters s, d and c: they are the keys that answer.
edit-unsaved-question = { $title } contient des modifications non enregistrées. Enregistrer, abandonner, ou annuler ? Appuyez sur s, d, ou c, ou Haut et Bas et Entrée. Échap annule.
edit-save-changes-title = Enregistrer les modifications de { $title } ?
edit-choice-save = Enregistrer, puis continuer
edit-choice-discard = Abandonner les modifications
edit-choice-cancel = Annuler, continuer à modifier
edit-save-failed = Impossible d'enregistrer : { $error } Toujours en cours d'édition. Essayez Enregistrer sous.
# The Save As prompt; $path is the suggested file.
edit-save-as-label = Enregistrer sous, Entrée pour { $path }
# $name is a file name. Keep the letters y and n.
edit-file-exists-question = { $name } existe déjà. Le remplacer ? y ou n
edit-mode-off = Mode édition désactivé.
edit-mode-off-discarded = Modifications abandonnées. Mode édition désactivé.
# The title of a new, unsaved document.
edit-untitled = Sans titre
edit-new-document = Nouveau document prêt à être modifié.
# $key turns on edit mode.
edit-nothing-to-save = Rien à enregistrer. Activez le mode édition avec { $key } pour apporter des modifications.
# $key turns on edit mode; $what is what the user tried to do.
edit-not-editing =
    { $what ->
        [type] Activez le mode édition avec { $key } pour taper.
        [change-text] Activez le mode édition avec { $key } pour modifier le texte.
        [delete-text] Activez le mode édition avec { $key } pour supprimer du texte.
        [undo] Activez le mode édition avec { $key } pour annuler.
        [redo] Activez le mode édition avec { $key } pour rétablir.
        [replace-text] Activez le mode édition avec { $key } pour remplacer du texte.
        [insert] Activez le mode édition avec { $key } pour insérer dans le texte.
        [cut-text] Activez le mode édition avec { $key } pour couper du texte.
        [move-cells] Activez le mode édition avec { $key } pour vous déplacer entre les cellules du tableau.
        [delete-words] Activez le mode édition avec { $key } pour supprimer des mots.
        [paste] Activez le mode édition avec { $key } pour coller.
        [citation] Activez le mode édition avec { $key } pour insérer une citation.
        [bibliography] Activez le mode édition avec { $key } pour insérer une bibliographie.
       *[format] Activez le mode édition avec { $key } pour mettre le texte en forme.
    }
# A paste: $n characters.
edit-pasted =
    { $n ->
        [one] 1 caractère collé.
       *[other] { $n } caractères collés.
    }
# $start is how the pasted text starts.
edit-pasted-start =
    { $n ->
        [one] 1 caractère collé : { $start }
       *[other] { $n } caractères collés : { $start }
    }
edit-insert-failed = Impossible d'insérer : { $error } Le texte n'a pas changé.
# $start is the first words of the pasted text.
edit-pasted-lines =
    { $n ->
        [one] 1 ligne collée : { $start }
       *[other] { $n } lignes collées : { $start }
    }
# $start and $end are character positions, $len the text's length.
edit-range-out-of-text = Impossible de modifier les caractères { $start } à { $end } : le texte en a { $len }.
edit-change-failed = Impossible de modifier le texte : { $error } Le texte n'a pas changé.
edit-delete-failed = Impossible de supprimer : { $error } Le texte n'a pas changé.
edit-list-ended = Fin de la liste.
# Said when Enter continues a bulleted list.
edit-bullet = puce
edit-table-divider = séparateur d'en-tête de tableau
edit-end-of-line-stop = Fin de ligne.
edit-start-of-line-stop = Début de ligne.
edit-end-of-line-content = fin de ligne

## Edit mode: formatting, undo, tables, images, and replace.

# $what names the formatting command; $level is a heading level, and
# $cols and $rows a table's size.
edit-format-done =
    { $what ->
        [bold] Gras.
        [italic] Italique.
        [underline] Souligné.
        [strikethrough] Barré.
        [code] Code.
        [code-block] Bloc de code.
        [link] Lien.
        [bulleted-list] Liste à puces.
        [numbered-list] Liste numérotée.
        [block-quote] Citation.
        [horizontal-rule] Ligne horizontale insérée.
        [table-row] Ligne de tableau ajoutée.
        [heading-level] Titre de niveau { $level }.
        [table] Tableau inséré, { $cols } colonnes sur { $rows } lignes.
       *[heading] Titre.
    }
# The command toggled its markup off.
edit-format-removed =
    { $what ->
        [bold] Gras supprimé.
        [italic] Italique supprimé.
        [underline] Souligné supprimé.
        [strikethrough] Barré supprimé.
        [code] Code supprimé.
        [code-block] Bloc de code supprimé.
        [link] Lien supprimé.
        [bulleted-list] Liste à puces supprimée.
        [numbered-list] Liste numérotée supprimée.
        [block-quote] Citation supprimée.
        [horizontal-rule] Ligne horizontale supprimée.
        [table-row] Ligne de tableau supprimée.
        [heading-level] Titre de niveau { $level } supprimé.
        [table] Tableau supprimé.
       *[heading] Titre supprimé.
    }
edit-format-unchanged =
    { $what ->
        [bold] Gras : rien n'a changé.
        [italic] Italique : rien n'a changé.
        [underline] Souligné : rien n'a changé.
        [strikethrough] Barré : rien n'a changé.
        [code] Code : rien n'a changé.
        [code-block] Bloc de code : rien n'a changé.
        [link] Lien : rien n'a changé.
        [bulleted-list] Liste à puces : rien n'a changé.
        [numbered-list] Liste numérotée : rien n'a changé.
        [block-quote] Citation : rien n'a changé.
        [horizontal-rule] Ligne horizontale : rien n'a changé.
        [table-row] Ligne de tableau : rien n'a changé.
        [heading-level] Titre de niveau { $level } : rien n'a changé.
        [table] Tableau : rien n'a changé.
       *[heading] Titre : rien n'a changé.
    }
# Added after a formatting message; $text is the start of the selection.
edit-format-selected = Sélectionné : { $text }
edit-heading-level-now = Titre de niveau { $level }.
# $line is the line at the caret after the undo or redo.
edit-undo-redo =
    { $what ->
        [undo] Annuler.
       *[redo] Rétablir.
    }
edit-undo-redo-line =
    { $what ->
        [undo] Annuler. { $line }
       *[redo] Rétablir. { $line }
    }
edit-nothing-to-undo = Rien à annuler.
edit-nothing-to-redo = Rien à rétablir.
edit-not-a-table-size = Ce n'est pas une taille de tableau : { $text }. Tapez colonnes et lignes, par exemple 3 par 2.
# $name is the image's file name.
edit-image-inserted = Image { $name } insérée. Sa description est sélectionnée ; tapez pour la remplacer.
edit-image-failed = Impossible d'insérer l'image : { $error } Le texte n'a pas changé.
# Edit mode's Obsidian aids: a wiki link, tasks, callouts.
edit-wikilink-title = Lier à une note
edit-wikilink-intro =
    { $n ->
        [one] Lier à une note, 1 note. Entrée la choisit.
       *[other] Lier à une note, { $n } notes. Entrée en choisit une.
    }
edit-wikilink-no-notes = Ce coffre ne contient aucune note vers laquelle créer un lien.
edit-wikilink-anchor-title = Lier à une partie de { $note }
edit-wikilink-anchor-intro = Lier à toute la note { $note }, ou à un titre ou un bloc de celle-ci. Entrée choisit.
edit-wikilink-whole = La note entière
edit-wikilink-heading = Titre { $level } : { $text }
edit-wikilink-block = Bloc { $id } : { $text }
edit-wikilink-inserted = Lien vers { $target } inséré.
edit-callout-title = Type d'encadré
edit-callout-intro = Types d'encadré, { $n }. Entrée en insère un.
edit-callout-inserted = Encadré de type { $type } inséré. Tapez son texte.
edit-task-status =
    { $status ->
        [done] Cochée.
        [open] Non cochée.
        [in-progress] En cours.
        [cancelled] Annulée.
        [forwarded] Transférée.
        [important] Importante.
        [question] Question.
       *[other] Marquée { $mark }.
    }
edit-task-new = Nouvelle tâche, non cochée.
edit-tasks-changed =
    { $n ->
        [one] Tâche modifiée sur 1 ligne.
       *[other] Tâches modifiées sur { $n } lignes.
    }
# $query is the text to find.
common-no-matches = Aucune correspondance pour { $query }.
# $n matches of $query were found; the replacement is asked next.
edit-replace-with =
    { $n ->
        [one] 1 correspondance pour { $query }. Remplacer par ?
       *[other] { $n } correspondances pour { $query }. Remplacer par ?
    }

## Edit mode: autosave and recovering unsaved work.

common-recovery-write-failed = Impossible d'écrire la copie de récupération : { $error }. Enregistrez bientôt ; { -brand } continuera d'essayer.
common-recovery-writing-again = La copie de récupération est en cours de réécriture.
# $title is the document; $when is how long ago its work was saved.
edit-recovery-offer = Récupérer les modifications non enregistrées de { $title } ? Enregistré : { $when }. Haut et Bas choisissent, Entrée confirme.
edit-recovery-title = Récupérer le travail non enregistré de { $title } ?
edit-recovery-yes = Oui, récupérer { $title } et continuer à modifier
edit-recovery-no = Non, abandonner les modifications non enregistrées
edit-recovery-discarded = Modifications non enregistrées de { $title } abandonnées.
edit-recovered = Travail non enregistré de { $title } récupéré. N'oubliez pas d'enregistrer.
edit-recovery-postponed = Récupération reportée. Le travail non enregistré sera proposé de nouveau la prochaine fois.

## Saving in the background.

writes-still-saving = Enregistrement en cours. Veuillez patienter.
writes-not-written-in-time = Certaines modifications n'ont pas pu être écrites à temps : le disque ne répond pas.
# $error is the system's reason.
writes-save-failed = Impossible d'enregistrer : { $error }. Toujours en cours d'édition.
# $name is the bookmark's name, $pct where it is.
writes-bookmark-not-saved = Le signet { $name } est posé pour l'instant, mais n'a pas pu être enregistré : { $error } Vérifiez que le dossier de données est accessible en écriture.
# $name is the saved file's name.
writes-saved = { $name } enregistré. Toujours en cours d'édition.

## Files changed on disk. $name is a file name. Keep the letters y and
## n: they are the keys that answer.

disk-replace-question = { $name } existe déjà. Le remplacer ? y ou n
# A prompt label, also said with a full stop after it.
disk-not-replaced = Non remplacé. Tapez un autre nom
# $key is the key for Save As.
disk-not-saved = Non enregistré. Toujours en cours d'édition. Enregistrer sous, { $key }, conserve les deux versions.
disk-kept-open-version = Version ouverte conservée.
disk-overwrite-question = { $name } a changé sur le disque depuis son ouverture. Enregistrer par-dessus ces modifications ? y ou n
disk-reload-question = { $name } a changé sur le disque. Le recharger ? y ou n

## Marks found again after a file changed outside textweaver.

relocate-reading-position = votre position de lecture
relocate-bookmarks =
    { $n ->
        [one] 1 signet
       *[other] { $n } signets
    }
relocate-notes =
    { $n ->
        [one] 1 note
       *[other] { $n } notes
    }
relocate-highlights =
    { $n ->
        [one] 1 surlignage
       *[other] { $n } surlignages
    }
# Lists of relocate-* items: "a and b", and "a, b, and c", where $rest is
# every item but the last, joined by commas.
relocate-join-two = { $a } et { $b }
relocate-join-more = { $rest }, et { $last }
# $items is a list of the items above; $n how many marks it counts in all.
relocate-moved =
    { $n ->
        [one] { $items } a été déplacé pour correspondre
       *[other] { $items } ont été déplacés pour correspondre
    }
relocate-lost =
    { $n ->
        [one] { $items } n'a pas pu être retrouvé et est marqué
       *[other] { $items } n'ont pas pu être retrouvés et sont marqués
    }
# $clauses are relocate-moved and relocate-lost, joined by a comma.
relocate-changed = Le fichier a changé ; { $clauses }.

## New documents from templates.

# The built-in templates' names.
templates-essay = Essai
templates-report = Rapport
templates-notes = Notes
# One of the user's own templates in the list; $name is its file name.
templates-yours = { $name }, votre modèle
# $folder is where the user's own templates go.
templates-intro =
    { $n ->
        [one] Nouveau document à partir d'un modèle, 1 modèle. Entrée choisit. Vos propres modèles vont dans { $folder }.
       *[other] Nouveau document à partir d'un modèle, { $n } modèles. Entrée choisit. Vos propres modèles vont dans { $folder }.
    }
# The title given when none is typed.
templates-untitled = Sans titre
# $template is the template's name, $title the document's, $date today's date (2026-09-26).
templates-created = Nouveau document à partir du modèle { $template } : { $title }. Daté du { $date }. Le curseur est là où l'écriture commence. N'oubliez pas d'enregistrer.
