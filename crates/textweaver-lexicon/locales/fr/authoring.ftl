### Authoring: lists, citations, find and replace, tables, quick wins, and dictation.

## Authoring lists: the outline, the citation picker, spelling, grammar, and templates.

# An outline item: $text is the heading's text, $level its level.
lists-outline-item = { $text }, niveau { $level }
# $n is the number of headings.
lists-outline-title =
    { $n ->
        [one] Plan, { $n } titre
       *[other] Plan, { $n } titres
    }
# $shown headings of $n match the filter $filter typed so far.
lists-outline-title-filtered = Plan, { $shown } sur { $n } correspondent à { $filter }
# $n is the number of references.
lists-citations-title =
    { $n ->
        [one] Insérer une citation, { $n } référence
       *[other] Insérer une citation, { $n } références
    }
# $shown references of $n match the filter $filter typed so far.
lists-citations-title-filtered = Insérer une citation, { $shown } sur { $n } correspondent à { $filter }
# $word is the misspelled word.
lists-spelling-title = Orthographe de { $word }
lists-spelling-add = Ajouter { $word } à votre liste de mots
lists-leave-as-is = Le laisser tel quel
# $words are the words the grammar fixes are for.
lists-grammar-title = Corrections de grammaire pour { $words }
# $n is the number of templates.
lists-templates-title = Nouveau document à partir d'un modèle, { $n } modèles
lists-no-filter = Cette liste ne filtre pas.
# The filter was emptied: $n items are shown.
lists-filter-cleared-headings =
    { $n ->
        [one] Filtre effacé, { $n } titre.
       *[other] Filtre effacé, { $n } titres.
    }
lists-filter-cleared-references =
    { $n ->
        [one] Filtre effacé, { $n } référence.
       *[other] Filtre effacé, { $n } références.
    }
lists-filter-cleared-items =
    { $n ->
        [one] Filtre effacé, { $n } élément.
       *[other] Filtre effacé, { $n } éléments.
    }
# Nothing matches the filter $query.
lists-filter-none-headings = Aucun titre ne correspond à { $query }. Retour arrière retire des lettres.
lists-filter-none-references = Aucune référence ne correspond à { $query }. Retour arrière retire des lettres.
lists-filter-none-items = Aucun élément ne correspond à { $query }. Retour arrière retire des lettres.
# $n items match the filter.
lists-filter-matched-headings =
    { $n ->
        [one] { $n } titre correspond.
       *[other] { $n } titres correspondent.
    }
lists-filter-matched-references =
    { $n ->
        [one] { $n } référence correspond.
       *[other] { $n } références correspondent.
    }
lists-filter-matched-items =
    { $n ->
        [one] { $n } élément correspond.
       *[other] { $n } éléments correspondent.
    }
lists-no-headings = Ce document n'a aucun titre.
# $n is the number of headings; the keys are the outline list's own.
lists-outline-intro =
    { $n ->
        [one] Plan, { $n } titre. Tapez pour filtrer, Entrée va à un titre, Échap ferme.
       *[other] Plan, { $n } titres. Tapez pour filtrer, Entrée va à un titre, Échap ferme.
    }
# $heading is the text of the heading the cursor is under.
lists-outline-here = Vous êtes sous { $heading }.

## Citations: inserting, looking up, importing, checking, and the bibliography.

citations-on = Citations activées.
citations-off = Citations désactivées.
book-audio-on = Audio du livre activé. L’enregistrement est lu là où le livre en a un.
book-audio-off = Audio du livre désactivé. La synthèse vocale lit le livre.
book-audio-rate = Les changements de débit ne s’appliquent pas à l’audio enregistré.
# $key names the Add Reference command's keys.
citations-library-empty = Votre bibliothèque de références est vide. Ajoutez une référence par DOI ou ISBN avec { $key }, ou exécutez import references depuis la palette de commandes.
# $n is how many references the picker lists.
citations-picker-intro =
    { $n ->
        [one] Insérer une citation, { $n } référence. Tapez pour filtrer, Entrée choisit, Échap annule.
       *[other] Insérer une citation, { $n } références. Tapez pour filtrer, Entrée choisit, Échap annule.
    }
# $text is what was typed at the locator prompt.
citations-locator-unreadable = Impossible de lire le repère { $text }. Tapez une page telle que 12, des pages telles que 3-5, ou chapitre 2 ; Entrée seule pour aucun.
citations-insert-failed = Impossible d'insérer la citation : { $error } Le texte n'a pas changé.
# $what is the identifier being looked up, as the citation library describes it.
citations-looking-up = Recherche de { $what }.
citations-lookup-not-started = Impossible de démarrer la recherche : { $error } Patientez un instant, puis réessayez.
# $input is the DOI or ISBN as typed.
citations-lookup-failed = Impossible de rechercher { $input } : { $error } Vérifiez le DOI ou l'ISBN et la connexion.
citations-no-library-to-add-to = Il n'y a aucune bibliothèque où ajouter : { -brand } ne conserve aucun fichier dans cette session.
citations-library-save-failed = Impossible d'enregistrer la bibliothèque : { $error } Vérifiez que son dossier est accessible en écriture.
# $n is how many citations the document has.
citations-found-no-library =
    { $n ->
        [one] { $n } citation trouvée. { -brand } ne conserve aucune bibliothèque dans cette session.
       *[other] { $n } citations trouvées. { -brand } ne conserve aucune bibliothèque dans cette session.
    }
citations-check-failed = Impossible de vérifier les citations : { $error } Vérifiez le fichier de la bibliothèque de références.
citations-no-library-to-import-into = Il n'y a aucune bibliothèque où importer : { -brand } ne conserve aucun fichier dans cette session.
# $file is the file's path.
citations-import-failed = Impossible d'importer { $file } : { $error } Vérifiez qu'il s'agit d'un fichier .bib, .ris ou .json.
# $style is the style's name from the front matter, such as apa.
citations-style-unusable = Impossible d'utiliser le style de citation { $style } : { $error } Vérifiez le nom du style en tête du document.
citations-format-failed = Impossible de mettre en forme les citations : { $error } Vérifiez les clés de citation et la bibliothèque.
# $key names the Insert Citation command's keys.
citations-none-yet = Le document n'a encore aucune citation. Insérez-en une avec { $key }.
citations-nothing-to-list = Aucun des ouvrages cités n'est dans votre bibliothèque ; il n'y a donc rien à lister.
# $n is how many entries went in; $style is the style's name, such as apa.
citations-bibliography-inserted =
    { $n ->
        [one] Bibliographie insérée, { $n } entrée, style { $style }.
       *[other] Bibliographie insérée, { $n } entrées, style { $style }.
    }
# Follows citations-bibliography-inserted; $keys are citation keys joined with commas.
citations-not-in-library = Absentes de la bibliothèque : { $keys }.
citations-bibliography-insert-failed = Impossible d'insérer la bibliographie : { $error } Le texte n'a pas changé.

## Find and replace, one match at a time.

# $title is replace-match-title. Keep the letters r, s and a: they are the
# keys that answer.
replace-match-question = { $title }. La ligne : { $context }. Appuyez sur r pour remplacer, s pour ignorer, a pour tout remplacer, Échap pour arrêter.
# The replace list's title when no match is being asked about.
replace-title = Remplacer
# $n is this match's number, $total the number of matches, $line the line
# number, $found the matched text, and $result what it becomes (both
# shortened); the -removed form is for an empty replacement. $context in
# replace-match-question is the text of the match's line.
replace-match-title = Correspondance { $n } sur { $total }, ligne { $line } : { $found } devient { $result }
replace-match-title-removed = Correspondance { $n } sur { $total }, ligne { $line } : { $found } est supprimé
replace-item-this = Remplacer celle-ci
replace-item-skip = Ignorer celle-ci
replace-item-rest = Remplacer tout le reste
# $state is common-on or common-off.
replace-item-match-case = Respecter la casse : { $state }
replace-item-whole-words = Mots entiers seulement : { $state }
replace-failed = Impossible de remplacer : { $error } Le texte n'a pas changé.
# Said after switching match case; $state is common-on or common-off, and
# $n is the number of matches now.
replace-match-case-now =
    { $n ->
        [one] Respecter la casse { $state }. 1 correspondance.
       *[other] Respecter la casse { $state }. { $n } correspondances.
    }
replace-whole-words-now =
    { $n ->
        [one] Mots entiers seulement { $state }. 1 correspondance.
       *[other] Mots entiers seulement { $state }. { $n } correspondances.
    }
# $query is the text that was searched for.
replace-replaced =
    { $n ->
        [one] 1 correspondance remplacée.
       *[other] { $n } correspondances remplacées.
    }
replace-replaced-skipped = { $n } remplacées, { $skipped } ignorées.
replace-stopped = Arrêté. { $n } remplacées, { $skipped } ignorées.
# Find and replace with regular expressions (B1-fr). $state is common-on
# or common-off; $n is the number of matches now.
replace-item-regex = Expression régulière : { $state }
replace-item-across-lines = Sur plusieurs lignes : { $state }
replace-regex-now =
    { $n ->
        [one] Expression régulière { $state }. 1 correspondance.
       *[other] Expression régulière { $state }. { $n } correspondances.
    }
replace-across-lines-now =
    { $n ->
        [one] Sur plusieurs lignes { $state }. 1 correspondance.
       *[other] Sur plusieurs lignes { $state }. { $n } correspondances.
    }
# $problem is search-invalid-pattern: switching the option would make the
# pattern invalid.
replace-option-refused = { $problem } L'option ne change pas.
# Asked once before replacing all the rest; $n is how many. Keep y and n.
replace-all-question =
    { $n ->
        [one] Remplacer la dernière correspondance ? y ou n
       *[other] Remplacer les { $n } correspondances restantes ? y ou n
    }
replace-all-declined = Rien n'a été remplacé.
# The search options list, and the options as named in search-options-on.
search-options-title = Options de recherche
search-option-match-case = respecter la casse
search-option-whole-words = mots entiers
search-option-regex = expression régulière
search-option-across-lines = sur plusieurs lignes
# Said as Find or Replace opens when an option is on; $list joins the
# options' names with commas.
search-options-on = Options activées : { $list }.
# $at is the character where the pattern fails, counting from 1; $reason
# is the regular expression engine's own explanation (in English).
search-invalid-pattern = Motif non valide au caractère { $at } : { $reason }.
search-invalid-pattern-anywhere = Motif non valide : { $reason }.

## Markdown structure said in edit mode, before a line's text or as it
## is typed.

mdline-heading-level = titre de niveau { $level }
mdline-bullet = puce
# A numbered list item; $n is its number.
mdline-item = élément { $n }
# $item is mdline-bullet or mdline-item.
mdline-task-done = { $item }, tâche faite
mdline-task-not-done = { $item }, tâche non faite
mdline-table-row = ligne de tableau
mdline-quote = citation
mdline-code-fence = clôture de code
mdline-task = tâche
# Said as "1. " is typed at the start of a line; $n is the number as typed.
mdline-numbered-item = élément numéroté { $n }

## Moving through tables by row and cell. $dir is next (forward) or
## previous (backward).

common-not-in-table = Pas dans un tableau.
tables-edge-of-table =
    { $dir ->
        [next] Fin du tableau.
       *[previous] Début du tableau.
    }
tables-edge-of-row =
    { $dir ->
        [next] Fin de ligne.
       *[previous] Début de ligne.
    }
# $cell is the cell's text, after its column header and a colon when it has one.
tables-header-row = Ligne d'en-tête, { $cell }
tables-row = Ligne { $row }, { $cell }
# High verbosity: $message is what the move said, then where it is.
tables-with-position = { $message }. Ligne { $row } sur { $rows }, colonne { $col } sur { $cols }
# Say Position in a table.
tables-position = Tableau, ligne { $row } sur { $rows }, colonne { $col } sur { $cols }.

## Authoring quick wins: word count, links, clipboard, table cells,
## deleting words, and cycling settings.

# Code block languages said with ordinary words; proper names such as
# Python are not translated.
authoring-language-jsx = JavaScript avec JSX
authoring-language-tsx = TypeScript avec JSX
authoring-language-shell = shell
authoring-language-batch = batch Windows
authoring-language-c-header = en-tête C
authoring-language-cpp = C plus plus
authoring-language-csharp = C sharp
authoring-language-diff = diff
authoring-language-plain-text = texte brut
authoring-language-mermaid = diagramme Mermaid
authoring-grammar-not-in-build = La vérification grammaticale n'est pas incluse dans cette version.
# $count is $n with thousands separators.
authoring-word-count-selection =
    { $n ->
        [one] 1 mot dans la sélection.
       *[other] { $count } mots dans la sélection.
    }
authoring-word-count-document =
    { $n ->
        [one] 1 mot dans le document.
       *[other] { $count } mots dans le document.
    }
# $text is the link's text.
authoring-link-address = Adresse du lien : { $url }
authoring-link-named-address = Lien { $text }, adresse : { $url }
authoring-no-link = Aucun lien au curseur.
authoring-typing-echo =
    { $echo ->
        [characters-and-words] Écho de frappe : caractères et mots.
        [characters] Écho de frappe : caractères.
        [words] Écho de frappe : mots.
       *[none] Écho de frappe : aucun.
    }
authoring-nothing-to-copy = Rien de sélectionné à copier.
# $text is the first words of what was copied.
authoring-copied = Copié : { $text }
authoring-copied-sentence = Phrase copiée : { $text }
authoring-nothing-to-cut = Rien de sélectionné à couper.
authoring-cut = Coupé : { $text }
# $dir is next (moving forward) or previous.
authoring-table-edge =
    { $dir ->
        [next] Fin du tableau.
       *[previous] Début du tableau.
    }
# A column with no header text.
authoring-table-column = colonne { $n }
# Moving into a new row: $header is the column's header, $content the cell.
authoring-table-cell-row = Ligne { $row }. { $header } : { $content }
authoring-nothing-to-select = Rien à sélectionner.
authoring-selected-all =
    { $n ->
        [one] Tout sélectionné, 1 mot.
       *[other] Tout sélectionné, { $count } mots.
    }
authoring-space-deleted = Espace supprimée.
# $text is the word deleted.
authoring-deleted = { $text } supprimé.
# $key is the terminal's own paste key.
authoring-nothing-copied = Rien de copié dans { -brand } pour l'instant. Utilisez le collage de votre terminal, par exemple { $key }.
# $parts lists what came in, from the paste-part messages.
paste-markdown = Collé en Markdown : { $parts }
paste-part-heading =
    { $n ->
        [one] 1 titre
       *[other] { $n } titres
    }
paste-part-paragraph =
    { $n ->
        [one] 1 paragraphe
       *[other] { $n } paragraphes
    }
paste-part-list =
    { $n ->
        [one] 1 liste
       *[other] { $n } listes
    }
paste-part-table =
    { $n ->
        [one] 1 tableau
       *[other] { $n } tableaux
    }
paste-part-code =
    { $n ->
        [one] 1 bloc de code
       *[other] { $n } blocs de code
    }
paste-part-quote =
    { $n ->
        [one] 1 citation
       *[other] { $n } citations
    }
paste-part-link =
    { $n ->
        [one] 1 lien
       *[other] { $n } liens
    }
paste-empty = Rien à coller : le presse-papiers est vide.
paste-converting = Conversion du texte mis en forme à coller.
paste-failed = Impossible de coller : { $error } Essayez Coller en texte brut.
authoring-verbosity =
    { $level ->
        [low] Verbosité : faible.
        [high] Verbosité : élevée.
       *[normal] Verbosité : normale.
    }
authoring-punctuation =
    { $level ->
        [none] Ponctuation : aucune.
        [all] Ponctuation : tous.
       *[some] Ponctuation : quelques signes.
    }

## Dictation in edit mode (ADR-0042). Keep the meaning first: a
## 40-cell Braille display shows the start of the line. $words are the
## dictated words, $key the dictate key, $dir a folder, $error and $text
## are passed on as they are.
dictation-status = Dictée : { $words }
dictation-listening = Dictée en cours. Parlez, puis appuyez sur { $key } pour arrêter.
dictation-finishing = Fin de la dictée.
dictation-done = Dictée terminée.
dictation-busy = La dictée se termine. Réessayez dans un instant.
dictation-needs-edit = La dictée écrit en mode édition. Activer le mode édition et dicter ? y ou n
dictation-no-model = La dictée a besoin du modèle Whisper dans { $dir }. Voir Dictation dans la documentation.
dictation-failed = La dictée a échoué : { $error } Voir Dictation dans la documentation.
dictation-no-words = Aucun mot reconnu dans cette phrase.
dictation-lost = La dictée s’est arrêtée avant que ses derniers mots soient écrits.
dictation-not-typed = Mots dictés non écrits, le mode édition est désactivé : { $text }
dictation-paused = Dictée suspendue.
dictation-resumed = Dictée reprise.
dictation-not-active = Aucune dictée en cours.
dictation-cannot-pause = Cette dictée ne peut pas être suspendue ; arrêtez-la plutôt.
section-dictation = Dictée
