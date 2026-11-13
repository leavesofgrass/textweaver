### The library, links and footnotes, and the dictionary.

## The library.

# $n is how many documents the scan has found.
library-still-scanning = Analyse de la bibliothèque toujours en cours : { $n } trouvés jusqu'ici.
library-scan-failed = Impossible d'analyser la bibliothèque : { $error } Vérifiez les dossiers de la bibliothèque dans les paramètres.
library-scanning = Analyse de la bibliothèque.
library-scan-progress = Analyse de la bibliothèque : { $n } trouvés jusqu'ici.
library-scan-stopped = L'analyse de la bibliothèque s'est arrêtée de façon inattendue. Ouvrez de nouveau la bibliothèque pour réessayer.
# $command is the command line that adds a folder; $key names the Open command's key.
library-empty = La bibliothèque est vide. Ajoutez un dossier dans les Paramètres, sous Dossiers de bibliothèque, ou ouvrez un fichier avec { $key }.
library-add-folder-choose = Choisissez le dossier à ajouter à la bibliothèque
library-folder-added = { $name } a été ajouté à la bibliothèque. Ouvrez la bibliothèque pour voir ses documents.
library-folder-already = { $name } est déjà dans la bibliothèque.
library-intro =
    { $n ->
        [one] Bibliothèque, { $n } document. Tapez pour filtrer, Entrée en ouvre un, F2 modifie les détails, { $queue } le met dans la file.
       *[other] Bibliothèque, { $n } documents. Tapez pour filtrer, Entrée en ouvre un, F2 modifie les détails, { $queue } le met dans la file.
    }
library-title = Bibliothèque

## Following links and footnotes.

links-none-here = Aucun lien ou note de bas de page au curseur.
# $text is the link's text.
common-link-no-address = Le lien { $text } n'a pas d'adresse.
# $kind is mail or web; $target is the link's address.
links-open-question =
    { $kind ->
        [mail] Ouvrir le lien de messagerie ? y ou n. { $target }
       *[web] Ouvrir le lien web ? y ou n. { $target }
    }
# A link to a file on another computer (a network share). Keep the
# letters y and n: they are the keys that answer. $target is the link.
links-network-question = Ouvrir un fichier sur un autre ordinateur ? y ou n. Votre identification peut lui être envoyée. { $target }
# The label of a heading reached by a link, at high verbosity.
links-heading-label = Titre
# $anchor is the heading name the link gives.
links-no-heading = Aucun titre appelé { $anchor } dans ce document.
# $file is the file the link names.
links-file-not-found = Le lien va vers { $file }, qui est introuvable.
links-block-label = Bloc
links-no-block = Aucun bloc { $id } dans ce document.
links-note-not-found = Lien vers une note qui n’existe pas : { $name }.
vault-same-name = { $count } notes s’appellent { $name } ; la plus proche a été ouverte.
vault-properties-count = Propriétés : { $n }.
vault-properties-title = Propriétés
vault-no-properties = Ce document n’a pas de propriétés.
vault-property-row = { $name } : { $value }
vault-property-checked = cochée
vault-property-not-checked = non cochée
vault-no-file = Ce document n’est pas enregistré dans un dossier, il n’a donc pas de coffre.
vault-partial = Seule une partie du coffre a été parcourue ; il est très grand.
vault-top-folder = dossier principal
vault-backlink-row =
    { $links ->
        [one] { $title }, 1 lien, dans { $folder }
       *[other] { $title }, { $links } liens, dans { $folder }
    }
vault-note-row = { $title }, dans { $folder }
vault-backlinks-title = Notes qui renvoient ici
vault-backlinks-intro =
    { $n ->
        [one] Notes qui renvoient ici, 1 note. Entrée l’ouvre.
       *[other] Notes qui renvoient ici, { $n } notes. Entrée en ouvre une.
    }
vault-no-backlinks = Aucune note ne renvoie ici.
vault-unresolved-title = Liens vers des notes inexistantes
vault-unresolved-intro =
    { $n ->
        [one] Liens vers des notes inexistantes, 1 lien.
       *[other] Liens vers des notes inexistantes, { $n } liens.
    }
vault-no-unresolved = Chaque lien de cette note mène à une note.
vault-note-tags-title = Étiquettes de cette note
vault-note-tags-intro =
    { $n ->
        [one] Étiquettes de cette note, 1 étiquette. Entrée liste les notes qui la portent.
       *[other] Étiquettes de cette note, { $n } étiquettes. Entrée liste les notes d’une étiquette.
    }
vault-no-note-tags = Cette note n’a pas d’étiquettes.
vault-tags-title = Étiquettes du coffre
vault-tags-intro =
    { $n ->
        [one] Étiquettes du coffre, 1 étiquette. Entrée liste ses notes.
       *[other] Étiquettes du coffre, { $n } étiquettes. Entrée liste les notes d’une étiquette.
    }
vault-no-tags = Aucune étiquette dans ce coffre.
vault-tag-row =
    { $n ->
        [one] { $tag }, 1 note
       *[other] { $tag }, { $n } notes
    }
vault-tag-notes-title = Notes portant une étiquette
vault-tag-notes-intro =
    { $n ->
        [one] Étiquette { $tag }, 1 note. Entrée l’ouvre.
       *[other] Étiquette { $tag }, { $n } notes. Entrée en ouvre une.
    }
vault-tag-no-notes = Aucune note ne porte l’étiquette { $tag }.
# $title is the document's title; $heading is the heading arrived at, $anchor
# the heading a link names; $key names the History Back command's keys;
# $text is the first words at the position arrived at.
links-opened = { $title } ouvert. Précédent : { $key }.
links-opened-at = { $title } ouvert, à { $heading }. Précédent : { $key }.
links-opened-no-heading = { $title } ouvert, au début : aucun titre nommé { $anchor }. Précédent : { $key }.
links-opened-no-block = { $title } ouvert, au début : aucun bloc { $id }. Précédent : { $key }.
links-back-in = Retour dans { $title } : { $text }
links-forward-in = Suivant dans { $title } : { $text }
# $label is the footnote's label, such as 1.
links-back-to-footnote-reference = Retour à l'appel de note { $label }, ligne { $line }.
# $text is the start of the note.
links-footnote = Note de bas de page { $label } : { $text }
links-footnote-unreferenced = Aucun appel vers la note de bas de page { $label } dans le texte.
links-footnote-no-note = La note de bas de page { $label } n'a pas de texte.

## Le filtre de la bibliothèque, le dictionnaire et les vitesses.

# The library list filtered: $shown of $n documents match $filter.
library-title-filtered = Bibliothèque, { $shown } sur { $n } correspondent à { $filter }
# The filter was emptied: $n documents are shown.
library-filter-cleared =
    { $n ->
        [one] Filtre effacé, { $n } document.
       *[other] Filtre effacé, { $n } documents.
    }
# No document matches the filter $query.
library-filter-none = Aucun document ne correspond à { $query }. Retour arrière retire des lettres.
# $n documents match the filter.
library-filter-matched =
    { $n ->
        [one] { $n } document correspond.
       *[other] { $n } documents correspondent.
    }
# Said once when define word is used while the dictionary file is still opening.
define-still-loading = Le dictionnaire est encore en cours de chargement.

## PDF links. Said before the first line of the page a link inside
## a PDF goes to, when the page has no heading there (as links-heading-label
## is for a heading). $page is the page's printed number or label (12, iv).
links-page-label = Page { $page }
