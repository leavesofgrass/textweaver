### Tracked changes and comments in Word files.

## B1-t1 : modifications suivies et commentaires (la liste des modifications).
prompt-comment-reply = Réponse
prompt-comment-text = Commentaire
prompt-github-token = Jeton GitHub de la source de composants, Entrée pour passer
prompt-import-cards = Importer des cartes d’étude d’un paquet Anki
prompt-cw-copy = Tapez ce que vous avez entendu
changes-title = Modifications et commentaires
changes-intro =
    { $n ->
        [one] { $title }, { $n } élément. Entrée y va. A accepte une modification, R la refuse, et avec Maj chacune traite toutes les modifications de son auteur. Sur un commentaire, F2 répond, Espace le résout, Suppr le supprime. N ajoute un commentaire.
       *[other] { $title }, { $n } éléments. Entrée va à l'un d'eux. A accepte une modification, R la refuse, et avec Maj chacune traite toutes les modifications de son auteur. Sur un commentaire, F2 répond, Espace le résout, Suppr le supprime. N ajoute un commentaire.
    }
changes-none = Aucune modification suivie ni aucun commentaire dans ce document.
changes-row = { $kind } : « { $text } », { $who }, { $when }
changes-kind-inserted = Inséré
changes-kind-deleted = Supprimé
changes-kind-moved-away = Déplacé d'ici
changes-kind-moved-here = Déplacé ici
changes-by = par { $author }
changes-no-author = auteur non indiqué
changes-no-date = date non indiquée
changes-date = { $weekday } { $day } { $month } { $year }
changes-weekday-0 = dimanche
changes-weekday-1 = lundi
changes-weekday-2 = mardi
changes-weekday-3 = mercredi
changes-weekday-4 = jeudi
changes-weekday-5 = vendredi
changes-weekday-6 = samedi
changes-month-1 = janvier
changes-month-2 = février
changes-month-3 = mars
changes-month-4 = avril
changes-month-5 = mai
changes-month-6 = juin
changes-month-7 = juillet
changes-month-8 = août
changes-month-9 = septembre
changes-month-10 = octobre
changes-month-11 = novembre
changes-month-12 = décembre
changes-comment = Commentaire : { $text }
changes-comment-by = Commentaire de { $author } : { $text }
changes-replies =
    { $n ->
        [one] { $n } réponse
       *[other] { $n } réponses
    }
changes-resolved = résolu
changes-accepted = Accepté. { $kind } : « { $text } ».
changes-rejected = Refusé. { $kind } : « { $text } ».
changes-accepted-all =
    { $n ->
        [one] { $n } modification acceptée.
       *[other] { $n } modifications acceptées.
    }
changes-rejected-all =
    { $n ->
        [one] { $n } modification refusée.
       *[other] { $n } modifications refusées.
    }
changes-accepted-author =
    { $n ->
        [one] { $n } modification de { $author } acceptée.
       *[other] { $n } modifications de { $author } acceptées.
    }
changes-rejected-author =
    { $n ->
        [one] { $n } modification de { $author } refusée.
       *[other] { $n } modifications de { $author } refusées.
    }
changes-none-left = Aucune modification suivie à accepter ou refuser.
changes-not-a-change = Cette ligne est un commentaire. F2 répond, Espace le résout, Suppr le supprime.
changes-not-a-comment = Cette ligne est une modification. A l'accepte, R la refuse.
changes-edit-mode = En mode édition, les modifications restent telles quelles. Quittez le mode édition pour les accepter ou les refuser.
changes-replied = Réponse ajoutée.
changes-resolved-done = Commentaire résolu.
changes-reopened = Commentaire rouvert.
changes-comment-deleted = Commentaire et réponses supprimés.
changes-comment-added = Commentaire ajouté.
changes-delete-comment-question = Supprimer ce commentaire et ses réponses ? y ou n
changes-written-accepted =
    { $n ->
        [one] { $path } écrit avec { $n } modification acceptée.
       *[other] { $path } écrit avec { $n } modifications acceptées.
    }
changes-written-rejected =
    { $n ->
        [one] { $path } écrit avec { $n } modification refusée.
       *[other] { $path } écrit avec { $n } modifications refusées.
    }
## End of B1-t1

## B1-t2 : enregistrer la révision dans le fichier Word.
changes-accept-all-question =
    { $n ->
        [one] Accepter la modification ? y ou n
       *[other] Accepter les { $n } modifications ? y ou n
    }
changes-reject-all-question =
    { $n ->
        [one] Refuser la modification ? y ou n
       *[other] Refuser les { $n } modifications ? y ou n
    }
changes-save-not-word = Ce n'est pas un fichier Word. Les modifications ne sont réenregistrées que dans les fichiers .docx ; l'export écrit le texte décidé dans d'autres formats.
changes-save-nothing = Rien à enregistrer : aucune modification acceptée ou refusée, et aucun commentaire changé.
changes-saved-backup = Modifications enregistrées dans { $file }. L'original est gardé sous le nom { $backup }.
changes-saved = Modifications enregistrées dans { $file }.
changes-save-unplaced =
    { $n ->
        [one] 1 nouveau commentaire est au début : son texte est introuvable.
       *[other] { $n } nouveaux commentaires sont au début : leur texte est introuvable.
    }
changes-save-failed = Impossible d'enregistrer les modifications dans { $file } : { $error }. Le fichier est inchangé ; fermez-le dans Word s'il est ouvert, puis réessayez.
changes-save-not-in-build = L'enregistrement des modifications dans le fichier Word n'est pas inclus dans cette version de { -brand }. tw changes --in-place le fait toujours.
changes-in-place-accepted =
    { $n ->
        [one] 1 modification acceptée dans { $path }. L'original est gardé sous le nom { $backup }.
       *[other] { $n } modifications acceptées dans { $path }. L'original est gardé sous le nom { $backup }.
    }
changes-in-place-rejected =
    { $n ->
        [one] 1 modification refusée dans { $path }. L'original est gardé sous le nom { $backup }.
       *[other] { $n } modifications refusées dans { $path }. L'original est gardé sous le nom { $backup }.
    }

## B1-t3: edits saved into a Word file as tracked changes.
changes-tracked-saved =
    { $n ->
        [one] 1 modification suivie enregistrée dans { $file }.
       *[other] { $n } modifications suivies enregistrées dans { $file }.
    }
changes-original-kept = L'original est gardé sous le nom { $backup }.
changes-tracked-refused =
    { $n ->
        [one] Non enregistré : 1 modification dans { $file } traverse un paragraphe ou se trouve dans un lien ou un champ, et ne peut pas être suivie. Enregistrer sous garde vos modifications en Markdown.
       *[other] Non enregistré : { $n } modifications dans { $file } traversent un paragraphe ou se trouvent dans un lien ou un champ, et ne peuvent pas être suivies. Enregistrer sous garde vos modifications en Markdown.
    }
## End of B1-t2
