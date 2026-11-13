### The textweaver app: drawn labels, hints, buttons, and dialogs.

## The window (GUI)

gui-open-title = Ouvrir un document
gui-open-documents = Documents que textweaver lit
gui-open-all-files = Tous les fichiers
gui-open-no-dialog = Le sélecteur de fichiers du système ne s'est pas ouvert. Tapez plutôt le chemin du document.
gui-text-size = Taille du texte { $size } points.
gui-text-size-largest = Taille du texte { $size } points, la plus grande.
gui-text-size-smallest = Taille du texte { $size } points, la plus petite.
gui-font = Police : { $family }.
gui-font-list = Police

## The window (GUI): drawn labels, hints, and questions.
## Keep the letters Y and N: they are the keys that answer.

gui-yes = Oui
gui-no = Non
gui-answer-delete = Supprimer
gui-answer-remove = Retirer
gui-answer-replace = Remplacer
gui-question-hint = Y répond oui, N répond non, Échap répond non.
gui-find-title = Rechercher et remplacer
gui-find-what = Rechercher
gui-find-with = Remplacer par
gui-find-match-case = Respecter la casse
gui-find-whole-words = Mots entiers
gui-find-regex = Expression régulière
gui-find-across-lines = Sur plusieurs lignes
gui-find-next = Suivant
gui-find-next-help = Sélectionne la correspondance suivante.
gui-find-replace = Remplacer
gui-find-replace-help = Remplace la correspondance affichée et passe à la suivante. Le premier appui trouve une correspondance.
gui-find-replace-all = Tout remplacer
gui-find-replace-all-help = Indique le nombre de correspondances et demande une fois. Une seule annulation les rétablit toutes.
gui-find-hint = Entrée dans Rechercher trouve la correspondance suivante ; Entrée dans Remplacer par la remplace. Flèche haut rappelle les textes précédents. Échap ferme.
gui-find-empty = Rien à rechercher : tapez le texte dans Rechercher.
gui-button-open = Ouvrir…
gui-button-font = Police…
gui-button-edit = Modifier
gui-button-finish-editing = Terminer la modification
gui-button-settings = Paramètres…
gui-button-commands = Commandes…
gui-button-play = Lire
gui-button-pause = Pause
gui-button-stop = Arrêter
gui-button-previous-sentence = Phrase précédente
gui-button-next-sentence = Phrase suivante
gui-button-slower = Plus lent
gui-button-faster = Plus rapide
gui-button-start-dictation = Démarrer la dictée
gui-button-pause-dictation = Suspendre la dictée
gui-button-resume-dictation = Reprendre la dictée
gui-button-stop-dictation = Arrêter la dictée
gui-button-live-pause = Suspendre les mises à jour
gui-button-live-resume = Reprendre les mises à jour
gui-button-live-newest = Aller à la plus récente
gui-button-live-table = Vue en tableau
gui-button-live-list = Vue en liste
gui-button-live-save = Enregistrer le journal
gui-button-live-close = Fermer la source
gui-button-live-show = Afficher les changements
gui-button-live-show-on = Afficher les changements : activé
gui-button-live-show-off = Afficher les changements : désactivé
gui-button-live-speak = Dire les changements
gui-button-live-speak-on = Dire les changements : activé
gui-button-live-speak-off = Dire les changements : désactivé
gui-button-unavailable = indisponible
gui-hint-open = Choisir un document à lire.
gui-hint-font = Choisir la police du texte.
gui-hint-edit = Passer de la lecture à l'édition.
gui-hint-settings = Chaque option, avec son aide.
gui-hint-commands = Lancer une commande par son nom.
gui-hint-play = Lire depuis le mot actuel, ou pause.
gui-hint-live-pause = Les entrées attendent la reprise.
gui-hint-live-view = Garde votre place dans le journal.
gui-hint-live-save = Enregistrer le journal en JSON.
gui-hint-live-close = Arrête la réception ; le journal reste.
gui-hint-live-field = Pour le champ de la ligne du curseur.
gui-button-close = Fermer
gui-toolbar-reading = Lecture
gui-document = Document
gui-document-titled = { $title }, document
gui-list-hint = Entrée choisit, Échap ferme.
gui-sidebar-contents = Sommaire
gui-sidebar-notes = Notes
gui-sidebar-open =
    { $n ->
        [one] { $panel } ouvert, 1 élément.
       *[other] { $panel } ouvert, { $n } éléments.
    }
gui-sidebar-closed = { $panel } fermé.
gui-header-shown = En-tête affiché.
gui-header-hidden = En-tête masqué. Ses commandes gardent leurs touches.
gui-toolbar-shown = Barre d'outils affichée.
gui-toolbar-hidden = Barre d'outils masquée. Ses commandes gardent leurs touches.
gui-focus-mode-on = Mode concentration activé. Bouger la souris affiche les barres.
gui-focus-mode-off = Mode concentration désactivé.
gui-full-screen-on = Plein écran activé.
gui-full-screen-off = Plein écran désactivé.

## Personnaliser les boutons (crate::buttons).

buttons-title = Personnaliser les boutons
buttons-intro =
    { $n ->
        [one] Personnaliser les boutons, 1 bouton. Entrée sur un bouton pour le déplacer ou le retirer.
       *[other] Personnaliser les boutons, { $n } boutons. Entrée sur un bouton pour le déplacer ou le retirer.
    }
buttons-row =
    { $bar ->
        [header] { $name }, en-tête { $pos } sur { $count }
       *[toolbar] { $name }, barre d'outils { $pos } sur { $count }
    }
buttons-add-header = Ajouter un bouton à l'en-tête
buttons-add-toolbar = Ajouter un bouton à la barre d'outils
buttons-reset = Rétablir les deux barres par défaut
buttons-move-up = Monter
buttons-move-down = Descendre
buttons-remove = Retirer
buttons-add-intro =
    { $n ->
        [one] 1 commande à ajouter. Entrée l'ajoute à la fin.
       *[other] { $n } commandes à ajouter. Entrée en ajoute une à la fin.
    }
buttons-added =
    { $bar ->
        [header] { $name } ajouté, en-tête { $pos } sur { $count }.
       *[toolbar] { $name } ajouté, barre d'outils { $pos } sur { $count }.
    }
buttons-removed =
    { $bar ->
        [header] { $name } retiré de l'en-tête.
       *[toolbar] { $name } retiré de la barre d'outils.
    }
buttons-moved = { $name } déplacé, { $pos } sur { $count }.
buttons-first = { $name } est déjà le premier.
buttons-last = { $name } est déjà le dernier.
buttons-already =
    { $bar ->
        [header] { $name } est déjà dans l'en-tête.
       *[toolbar] { $name } est déjà dans la barre d'outils.
    }
buttons-no-name = Cette commande n'a pas de nom court, elle ne peut donc pas être un bouton.
buttons-no-button = Aucun bouton à cet endroit.
buttons-reset-done = Boutons rétablis par défaut.
buttons-none-to-add = Plus aucune commande à ajouter.
# The preview pane beside the editor (edit mode). $key is the key that moves between the window's parts (F6).
gui-preview = Aperçu
gui-preview-shown = Aperçu affiché à côté de l'éditeur. { $key } y va.
gui-preview-hidden = Aperçu masqué.
gui-preview-later = Aperçu activé. Il s'affiche à côté de l'éditeur en mode édition.
gui-preview-empty = Rien à prévisualiser pour l'instant.
gui-sidebar-no-headings = Aucun titre.
gui-sidebar-no-notes = Aucune note.
gui-sidebar-current = { $item }, actuel
gui-sidebar-hint = Entrée y va. { $leave } y va et revient. Échap revient.
gui-sidebar-hint-notes = Entrée y va. Espace montre ses liens. { $leave } y va et revient. Échap revient.
gui-sidebar-notes-keys = Espace montre les liens d'une note.
gui-settings-sections = Sections
gui-settings-form = Paramètres : { $section }
gui-settings-saved-hint = Les modifications s'appliquent et sont enregistrées aussitôt.
gui-settings-close-help = Fermer les paramètres. Chaque modification est déjà enregistrée.
gui-settings-recent = Modifiés récemment
gui-settings-matching = Correspondant à { $filter }
gui-settings-table = { $label } est un tableau. Modifiez-le dans settings.toml.
gui-setting-new-value = Nouvelle valeur pour { $label }
gui-setting-value-hint = Appuyez sur Entrée pour valider, ou sur Échap pour revenir.
gui-prompt-path-hint = Tapez le chemin d'un document, puis appuyez sur Entrée. Tab le complète ; Haut et Bas rappellent les précédents.
gui-prompt-hint = Appuyez sur Entrée pour valider, ou sur Échap pour annuler. Haut et Bas rappellent les réponses précédentes.
gui-palette-filter = Tapez pour filtrer les commandes
gui-palette-list = Commandes
gui-palette-hint = Entrée exécute la première correspondance ; Tab passe à la liste ; F1 explique une commande.
gui-open-failed = Impossible d'ouvrir { $name } : { $error }
gui-uia-unavailable = Les notifications UI Automation n'existent que sous Windows ; la région active est utilisée.
gui-graphics-failed = Impossible de démarrer l'application { -brand } : ses graphismes ont échoué. Le lecteur en terminal, tw, n'en a pas besoin.
gui-crashed = textweaver s'est arrêté après une erreur interne.
gui-crashed-saved = Position enregistrée. Les modifications non enregistrées seront proposées à la récupération la prochaine fois.
gui-rsvp = RSVP
gui-rsvp-playing = RSVP en cours, mot { $n } sur { $total }
gui-rsvp-paused = RSVP en pause, mot { $n } sur { $total }
gui-rsvp-finished = RSVP terminé, mot { $n } sur { $total }
gui-settings-section-item =
    { $section }, { $n ->
        [one] 1 paramètre
       *[other] { $n } paramètres
    }
gui-palette-count =
    { $n ->
        [0] Aucune commande ne correspond.
        [one] 1 commande.
       *[other] { $n } commandes.
    }
gui-settings-form-help = Haut et Bas passent d'un paramètre à l'autre. Gauche et Droite en changent un. Entrée tape une nouvelle valeur. Suppr remet la valeur par défaut. { $next } et { $previous } changent de section. Tapez pour filtrer. F1 dit l'aide.
gui-settings-press-enter = Appuyez sur Entrée pour taper une nouvelle valeur pour { $label }.
gui-font-built-in = { $family } (intégrée)

## The window's menus and dialogs. Settings files chosen with the
## system's file chooser, the Colors dialog, and the font list. $ratio is
## a contrast ratio such as 4.8; $verdict is good, fair, or low.
gui-settings-files = Fichiers de paramètres
gui-settings-export-title = Exporter les paramètres
gui-settings-import-title = Importer les paramètres
gui-chooser-no-dialog = Le sélecteur de fichiers du système ne s'est pas ouvert. Tapez plutôt le chemin du fichier.
gui-colors-value = { $value }, contraste { $ratio } pour 1, { $verdict }
gui-colors-help = Gauche et Droite choisissent une couleur nommée, le bleu et l'orange d'abord. Entrée saisit un nom ou une valeur #rrggbb. Suppr remet la couleur du thème. Chaque marque garde son soulignement, sa graisse ou son symbole, quelle que soit sa couleur.
gui-colors-value-system = { $value } ; non dessinée tant que les couleurs de contraste élevé du système sont actives
gui-colors-help-system = Les couleurs de contraste élevé du système sont actives et sont dessinées à la place de celles-ci, qui s'appliquent de nouveau quand elles sont désactivées. Chaque nom de surlignage garde sa propre forme.
gui-colors-reset-all = Rétablir toutes les couleurs
gui-colors-reset-all-help = Remettre la couleur du thème pour chaque partie.
gui-colors-reset-done = Toutes les couleurs sont de nouveau celles du thème.
colors-reset-question = Rétablir toutes les couleurs du thème ? y ou n
gui-colors-closed = Couleurs fermées.
gui-font-list-intro =
    { $n ->
        [one] { $title }, 1 famille.
       *[other] { $title }, { $n } familles.
    }

## W9b-x : les paramètres de lecture (Affichage, Paramètres de lecture).
reading-form-intro = Paramètres de lecture, { $n } paramètres. Gauche et Droite changent une valeur, Entrée en saisit une, Suppr remet la valeur par défaut, F1 dit l'aide.
reading-form-spacing-wcag-done = Espacements réglés sur les valeurs WCAG.
reading-form-spacing-generous-done = Espacements réglés sur Large, plus que WCAG.
spacing-letter-without-word = Augmentez l'espace des mots avec celui des lettres.
gui-reading-form-help = Haut et Bas déplacent, Gauche et Droite changent une valeur, Entrée en saisit une, F1 dit l'aide.
gui-reading-voices = Voix
gui-reading-voices-help = Ouvrir le gestionnaire de voix.
gui-reading-wcag = Espacement WCAG
gui-reading-wcag-help = Régler les quatre espacements sur les valeurs WCAG.
gui-reading-generous = Espacement large
gui-reading-generous-help = Régler les quatre espacements plus larges que WCAG.
gui-reading-closed = Paramètres de lecture fermés.
## End of W9b-x
