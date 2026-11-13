### Notes, highlights, bookmarks, relations, and find.

## Notes and highlights.

notes-nothing-to-attach = Rien ici à quoi attacher une note.
# $on is the start of the passage the note is on.
notes-added = Note ajoutée sur : { $on }
# $tags are the note's tags, joined with commas.
notes-added-with-tags = Note ajoutée avec les étiquettes { $tags } sur : { $on }
# An item in the notes list. $anchor is the passage; $lost is yes when the
# passage was not found after the file changed.
notes-item =
    { $lost ->
        [yes] { $note }, ligne { $line }. Sur : { $anchor } Introuvable après la modification du fichier.
       *[no] { $note }, ligne { $line }. Sur : { $anchor }
    }
notes-none = Aucune note. Pour en ajouter une : { $key }.
notes-list-title = Notes
notes-list-intro =
    { $n ->
        [one] Notes, 1 élément. Entrée va à une note, Suppr la supprime, F2 la modifie, Espace ouvre ses liens. C crée une carte.
       *[other] Notes, { $n } éléments. Entrée va à une note, Suppr la supprime, F2 la modifie, Espace ouvre ses liens. C crée une carte.
    }
# Said on jumping to a note: its text, then the passage it is on.
notes-note-content = { $note }. Sur : { $anchor }
# $i is the note's number, $n how many notes there are.
notes-note-label = Note { $i } sur { $n }
notes-deleted = Note supprimée : { $text }.
notes-none-here = Aucune note ou surlignage ici.
notes-unchanged = Note inchangée.
notes-updated = Note mise à jour.
notes-nothing-to-highlight = Rien ici à surligner.
notes-highlight-removed = Surlignage supprimé : { $text }
notes-highlighted-at = Surligné, { $name }, à { $pct } pour cent : { $text }
notes-highlighted = Surligné, { $name } : { $text }
# An item in the highlights list. $name is the highlight's palette name;
# $lost is yes when the text was not found after the file changed.
notes-highlight-item =
    { $lost ->
        [yes] { $name } : { $text }, ligne { $line }, introuvable après la modification du fichier
       *[no] { $name } : { $text }, ligne { $line }
    }
notes-no-highlights = Aucun surlignage. Pour en faire un : { $key }.
notes-highlights-title = Surlignages
notes-highlights-intro =
    { $n ->
        [one] Surlignages, 1 élément. Entrée va à l'un d'eux, Suppr le supprime, F2 change son nom, Espace n'affiche que son nom. C crée une carte.
       *[other] Surlignages, { $n } éléments. Entrée va à l'un d'eux, Suppr le supprime, F2 change son nom, Espace n'affiche que son nom. C crée une carte.
    }
# The label said before a highlight's text on jumping to it.
notes-highlight-label = Surlignage, { $name }
notes-highlight-changed = Surlignage changé en { $name } : { $text }
# $n is the palette entry's number, $key the keys of Highlight with a name.
notes-palette-no-entry = Pas de nom de surlignage { $n } dans la palette. Pour choisir un nom : { $key }.
notes-highlights-named-title = Surlignages : { $name }
notes-highlights-named-intro =
    { $n ->
        [one] Surlignages nommés { $name }, 1 élément. Espace affiche tous les surlignages.
       *[other] Surlignages nommés { $name }, { $n } éléments. Espace affiche tous les surlignages.
    }
notes-palette-title = Noms de surlignage
# A palette name: the name, how many highlights have it, its shape and its color.
notes-palette-row =
    { $count ->
        [one] { $name }, 1 surlignage, { $shape }, { $color }
       *[other] { $name }, { $count } surlignages, { $shape }, { $color }
    }
notes-palette-intro-highlight = Surligner avec quel nom ? { $n } noms. Entrée en choisit un.
notes-palette-intro-change = Changer le surlignage en quel nom ? { $n } noms. Entrée en choisit un.
notes-palette-intro-collect = Rassembler les surlignages de quel nom ? { $n } noms. Entrée les écrit en liste.
notes-collect-none = Aucun surlignage nommé { $name }.
# The collected list's own title (Markdown).
notes-collect-title = Surlignages nommés { $name } : { $title }
notes-collect-line = (ligne { $line })
# Keep the letters y and n: they are the keys that answer.
notes-collect-saved =
    { $n ->
        [one] 1 surlignage nommé { $name } enregistré sous { $file }. L'ouvrir ? y ou n. Dans { $folder }.
       *[other] { $n } surlignages nommés { $name } enregistrés sous { $file }. L'ouvrir ? y ou n. Dans { $folder }.
    }
palette-shape-underline = soulignement
palette-shape-double-underline = double soulignement
palette-shape-bold = gras
palette-shape-dotted = soulignement pointillé
palette-shape-brackets = crochets
palette-shape-symbol = symbole
# Shown while reading reaches a note's passage.
notes-signal = Note : { $text }
# Said after moving onto a note's passage.
notes-has-note = A une note : { $text }

## Relations between notes (the knowledge graph as lists).

relations-type-conflicts-with = est en conflit avec
relations-type-supports = appuie
relations-type-is-example-of = est un exemple de
relations-type-cites = cite
relations-type-contradicts = contredit
relations-type-defines = définit
relations-type-extends = prolonge
relations-type-see-also = voir aussi
relations-type-precedes = précède
relations-type-follows = suit
relations-count = Liens : { $out } sortants, { $in } entrants.
relations-note-title = Liens de : { $note }
relations-title-filtered = { $title }, filtre : { $filter }
relations-note-intro = Liens de { $note } : { $out } sortants, { $in } entrants. Entrée suit un lien, F2 le modifie, Suppr le supprime. Tapez pour filtrer par type.
relations-out-item = { $type } : { $target }
relations-target-in = { $note }, dans { $doc }
relations-target-missing = une note introuvable
relations-empty-note = Note vide
relations-incoming-row =
    { $n ->
        [0] Ce qui pointe ici : rien pour l'instant
        [one] Ce qui pointe ici : 1 note
       *[other] Ce qui pointe ici : { $n } notes
    }
relations-add-row = Ajouter un lien
relations-backlinks-title = Ce qui pointe ici : { $note }
relations-backlinks-intro =
    { $n ->
        [one] Ce qui pointe vers { $note } : 1 note. Entrée y va. Tapez pour filtrer par type.
       *[other] Ce qui pointe vers { $note } : { $n } notes. Entrée va à l'une d'elles. Tapez pour filtrer par type.
    }
relations-backlink-item = { $type } celle-ci, depuis : { $note }
relations-none-in = Rien ne pointe encore vers cette note.
relations-types-title = Type de lien pour : { $note }
relations-types-intro = Choisissez le type de lien, 10 types. Tapez pour filtrer.
relations-targets-title = { $type } : quelle note ?
relations-targets-intro =
    { $n ->
        [0] Aucune autre note ici. Choisissez une note d'un autre document.
        [one] Choisissez la note cible : 1 note. Entrée crée le lien.
       *[other] Choisissez la note cible : { $n } notes. Entrée crée le lien.
    }
relations-other-document-row = Une note d'un autre document
relations-documents-title = Documents avec des notes
relations-documents-intro = Documents avec des notes : { $n }. Entrée liste les notes d'un document.
relations-document-item =
    { $n ->
        [one] { $title }, 1 note
       *[other] { $title }, { $n } notes
    }
relations-no-other-documents = Aucun autre document de la bibliothèque n'a de notes.
relations-linked = Lien créé : { $type } { $target }.
relations-changed = Lien modifié : { $type } { $target }.
relations-already = Déjà lié : { $type } { $target }.
relations-removed = Lien supprimé : { $type } { $target }.
relations-remove-question = Supprimer ce lien ? y ou n
relations-nothing-to-remove = Seul un lien peut être supprimé ici.
relations-note-gone = Note introuvable : elle a été supprimée, ou son document a disparu.
relations-document-missing = Document introuvable : { $file }.
relations-no-note-here = Pas de note ici. Les liens appartiennent aux notes ; en ajouter une : { $key }.
relations-filter-cleared = Filtre effacé, { $n } affichés.
relations-filter-none = Rien ne correspond à { $filter }.
relations-filter-matched = Filtre { $filter } : { $n } affichés.

## Exporter le graphe de connaissances (B1-g2).

graph-export-title = Exporter le graphe de connaissances en
graph-export-intro = Graphe de connaissances, liens : { $links }. Choisissez un format ; la liste Markdown est le texte à lire.
graph-export-empty = Aucun lien entre notes à exporter. Ajoutez-en un depuis la liste des liens d'une note.
graph-format-md = Liste Markdown, le texte à lire
graph-format-json = JSON, pour Gephi et Cytoscape
graph-format-dot = DOT, pour Graphviz
graph-format-graphml = GraphML, pour Gephi, Cytoscape et yEd
graph-format-mermaid = Diagramme Mermaid
graph-format-plantuml = Diagramme PlantUML
graph-format-csv = Liste d'arêtes CSV, pour les tableurs
graph-export-saved = Graphe de connaissances enregistré sous { $file }. L'ouvrir ? y ou n. Dans { $folder }.
graph-export-failed = Impossible d'écrire le graphe de connaissances : { $error } Vérifiez que le dossier est accessible en écriture.

## Bookmarks: rename and delete.

notes-choose-bookmark-delete = Choisissez un signet et appuyez sur Suppr.
notes-choose-bookmark-rename = Choisissez un signet et appuyez sur F2 pour le renommer.
notes-bookmark-deleted = Signet { $name } supprimé.
notes-renaming-bookmark = Renommage du signet { $name }.
notes-bookmark-unchanged = Signet inchangé.
# $name is the name asked for, $old the bookmark's name.
notes-bookmark-name-taken = Il existe déjà un signet appelé { $name }. Signet { $old } inchangé.
notes-bookmark-renamed = Signet { $old } renommé en { $name }.

## Find, bookmarks, and selection.

marks-cannot-search = Impossible de rechercher : { $error } Vérifiez le motif entre les barres obliques.
# $pattern is the text searched for.
marks-no-matches = Aucune correspondance pour { $pattern }.
# The label of a match reached by Find, at high verbosity; $number is its place among $n matches.
marks-match-label = Correspondance { $number } sur { $n }
# Find wrapped past an end of the document; $dir is next (to the top) or previous (to the bottom); $message says the match.
marks-find-wrapped =
    { $dir ->
        [next] Retour au début. { $message }
       *[previous] Retour à la fin. { $message }
    }
# $name is the bookmark's name, such as mark1.
marks-bookmark-already-here = Le signet { $name } est déjà ici.
common-bookmark-set = Signet { $name } posé à { $pct } pour cent.
marks-no-bookmarks = Aucun signet. Pour en ajouter un : { $key }.
marks-bookmarks-intro =
    { $n ->
        [one] Signets, { $n } élément. Entrée va à l'un d'eux, Suppr le supprime, F2 le renomme.
       *[other] Signets, { $n } éléments. Entrée va à l'un d'eux, Suppr le supprime, F2 le renomme.
    }
# One line of the bookmark list; $lost is yes when the bookmark's text was not found after the file changed; $text is the start of its line.
marks-bookmark-item =
    { $lost ->
        [yes] { $name } (introuvable après la modification du fichier), ligne { $line }, { $pct } pour cent : { $text }
       *[no] { $name }, ligne { $line }, { $pct } pour cent : { $text }
    }
marks-bookmarks-title = Signets
# The label of a bookmark reached, at high verbosity.
marks-bookmark-label = Signet { $name }
marks-selection-cleared = Sélection effacée.
# $text is the selected text, shortened.
marks-selected = Sélectionné { $text }
