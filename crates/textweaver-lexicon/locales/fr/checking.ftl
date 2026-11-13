### Markdown lint, grammar, and spelling.

## Markdown lint (edit mode). A problem is said after "Lint: ".

lint-heading-level = titre de niveau { $level } après le niveau { $prev } ; utilisez le niveau { $use }.
# $reference is the link reference's name.
lint-link-reference = la référence de lien { $reference } n'a pas de définition.
lint-bare-url = adresse web nue ; mettez-la entre chevrons ou faites-en un lien avec un nom.
# $marker and $used are bullet names: lint-marker-dash and the others.
lint-list-marker = puce { $marker } ; cette liste utilise { $used }.
lint-marker-dash = tiret
lint-marker-star = étoile
lint-marker-plus = plus
lint-marker-other = autre
# $n is how many tabs and spaces there are.
lint-trailing-tabs-empty-line = tabulations ou espaces sur une ligne vide.
lint-trailing-tabs-line-end = tabulations ou espaces en fin de ligne.
lint-trailing-spaces-empty-line =
    { $n ->
        [one] 1 espace sur une ligne vide.
       *[other] { $n } espaces sur une ligne vide.
    }
lint-trailing-spaces-line-end =
    { $n ->
        [one] 1 espace en fin de ligne.
       *[other] { $n } espaces en fin de ligne.
    }
# $key turns on edit mode.
lint-not-editing = Lint vérifie le Markdown que vous écrivez. Activez d'abord le mode édition avec { $key }.
lint-not-markdown = Lint vérifie le Markdown, et ce document n'est pas du Markdown.
lint-none = Aucun problème Lint.
# $count is $n with thousands separators.
lint-no-more =
    { $n ->
        [one] Plus aucun problème Lint. 1 problème Lint au total.
       *[other] Plus aucun problème Lint. { $count } problèmes Lint au total.
    }
lint-no-earlier =
    { $n ->
        [one] Aucun problème Lint antérieur. 1 problème Lint au total.
       *[other] Aucun problème Lint antérieur. { $count } problèmes Lint au total.
    }
# $message is one of the problems above.
lint-said = Lint : { $message }
# Added at high verbosity.
common-line = Ligne { $line }.

## Grammar checking (Harper). $message is Harper's own message, in English.

# $words are the words the problem is about.
grammar-said = Grammaire : { $message } Les mots : { $words }.
# Said after grammar-said when the first fix removes the words.
grammar-fix-remove = Correction : les supprimer.
grammar-fix = Correction : { $fix }.
# A fix in the fixes list that removes the words.
grammar-remove-the-words = Supprimer les mots
grammar-none = Aucun problème de grammaire trouvé.
# $count is $n with thousands separators.
grammar-no-more =
    { $n ->
        [one] Plus aucun problème de grammaire. 1 problème de grammaire au total.
       *[other] Plus aucun problème de grammaire. { $count } problèmes de grammaire au total.
    }
grammar-no-earlier =
    { $n ->
        [one] Aucun problème de grammaire antérieur. 1 problème de grammaire au total.
       *[other] Aucun problème de grammaire antérieur. { $count } problèmes de grammaire au total.
    }
# $key opens the fixes list.
grammar-lists-fixes = { $key } liste les corrections.
# Added at high verbosity.
# $described is grammar-said (and its fix) without the last full stop.
grammar-no-fix = { $described } Aucune correction à proposer.
grammar-fixes =
    { $n ->
        [one] { $words } : 1 correction.
       *[other] { $words } : { $n } corrections.
    }
grammar-fixes-edit =
    { $n ->
        [one] { $words } : 1 correction. Entrée apporte la modification.
       *[other] { $words } : { $n } corrections. Entrée apporte la modification.
    }
common-left-as-is = Laissé tel quel.
# $fix is the fix chosen; $key turns on edit mode.
grammar-fix-not-editing = { $fix }. Activez le mode édition avec { $key } pour modifier le texte.
grammar-removed = Supprimé.
grammar-changed = Changé en { $fix }.
grammar-change-failed = Impossible de modifier le texte : { $error } Rien n'a été modifié.

## Spell checking.

# Said for an apostrophe when a word is spelled out letter by letter.
spell-apostrophe = apostrophe
spell-not-available = La vérification orthographique n'est pas disponible : cette version n'a pas de liste de mots.
spell-none-found = Aucune faute d'orthographe trouvée.
# $count is $n with thousands separators.
spell-no-more =
    { $n ->
        [one] Plus aucune faute. 1 faute possible au total.
       *[other] Plus aucune faute. { $count } fautes possibles au total.
    }
spell-no-earlier =
    { $n ->
        [one] Aucune faute antérieure. 1 faute possible au total.
       *[other] Aucune faute antérieure. { $count } fautes possibles au total.
    }
# Added at high verbosity.
spell-no-misspelled-word = Aucun mot mal orthographié au curseur.
# $word is the misspelled word; $n how many suggestions follow.
spell-suggestions =
    { $n ->
        [0] { $word } : aucune suggestion.
        [one] { $word } : 1 suggestion.
       *[other] { $word } : { $n } suggestions.
    }
spell-suggestions-edit =
    { $n ->
        [0] { $word } : aucune suggestion. Entrée remplace le mot.
        [one] { $word } : 1 suggestion. Entrée remplace le mot.
       *[other] { $word } : { $n } suggestions. Entrée remplace le mot.
    }
# $word is the suggestion chosen; $key turns on edit mode.
spell-replace-not-editing = { $word }. Activez le mode édition avec { $key } pour modifier le texte.
spell-replaced = Remplacé par { $word }.
spell-replace-failed = Impossible de remplacer : { $error } Le mot n'a pas changé.
spell-added-for-session = { $word } ajouté à votre liste de mots pour cette session.
spell-added = { $word } ajouté à votre liste de mots.
spell-save-failed = Impossible d'enregistrer votre liste de mots : { $error } Le mot est reconnu jusqu'à la fermeture.
# After a save; $count is $n with thousands separators.
spell-count =
    { $n ->
        [0] Aucune faute d'orthographe.
        [one] 1 faute possible.
       *[other] { $count } fautes possibles.
    }
