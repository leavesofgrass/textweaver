### Menus, the command palette, and interface announcements.

## Menus, the command palette, interface announcements, colors, and settings

## Menu titles; the top menus mark their access key with &.

menu-file = &Fichier
menu-edit = É&dition
menu-view = &Affichage
menu-reading = &Lecture
menu-speech = &Parole
menu-tools = Ou&tils
menu-help = A&ide
menu-recent = Documents récents
menu-export-as = E&xporter en
menu-preview = Aperçu
menu-settings = Paramètres
menu-find = Rechercher
menu-format = Format
menu-insert = Insérer
menu-proofing = Relecture
menu-citations = Citations
menu-text-size = Taille du texte
menu-reading-aids = Aides à la lecture
menu-rsvp = RSVP
menu-say = Dire
menu-move-by = Se déplacer par
menu-headings = Titres
menu-go-to = Aller à
menu-cursor = Curseur et sélection
menu-bookmarks = Signets et notes
menu-highlights = Surlignages
menu-tables = Tableaux
menu-speech-cursor = Curseur vocal

## Menus, the palette, and interface announcements.

menu-bar = Menus
menu-title = Menu { $name }
menu-submenu = { $name }, sous-menu
menu-checked = { $name }, coché
menu-not-checked = { $name }, non coché
menu-value = { $name } : { $value }
menu-with-keys = { $item }, { $keys }
menu-recent-document = { $name }, { $pct } pour cent
menu-recent-none = Aucun document récent
menu-not-available = { $name } n'est pas disponible dans cette version.
menu-no-access-key = Aucun élément avec la touche { $letter }.
menu-closed = Menus fermés.
menu-context = Menu contextuel
menu-context-closed = Menu contextuel fermé.
menu-press-a-key = Appuyez sur une touche pour entendre ce qu'elle fait.
menu-key-described = { $name } : { $help }. Touches : { $keys }. Dans les menus : { $path }.
menu-key-described-no-menu = { $name } : { $help }. Touches : { $keys }.
menu-keys = { $item }. Entrée ou Droite ouvre un menu ou exécute une commande, une lettre va à son élément, Gauche ou Retour arrière revient, Échap ferme.
edit-line-continues = la ligne continue
announce-level-changed = Annonces de l'interface : { $level }.
announce-level-off = désactivées
announce-level-minimal = minimales
announce-level-normal = normales
announce-level-full = complètes
palette-item = { $name }, { $keys }
palette-item-no-keys = { $name }
palette-item-recent = { $name }, { $keys }, récent
palette-item-recent-no-keys = { $name }, récent
palette-list-title = Commandes correspondant à { $query }
palette-list-title-all = Commandes
palette-list-intro =
    { $title }, { $n ->
        [one] 1 commande
       *[other] { $n } commandes
    }. Entrée en exécute une, F1 l'explique.
color-name-theme = couleur du thème
color-name-blue = bleu
color-name-orange = orange
color-name-navy = bleu foncé
color-name-skyblue = bleu ciel
color-name-teal = bleu canard
color-name-gold = doré
color-name-yellow = jaune
color-name-green = vert
color-name-cyan = cyan
color-name-purple = violet
color-name-pink = rose
color-name-brown = marron
color-name-gray = gris
color-name-black = noir
color-name-white = blanc
section-colors = Couleurs
colors-contrast-good = bon
colors-contrast-fair = passable
colors-contrast-low = faible
colors-item = { $label } : { $value }, contraste { $ratio } pour 1, { $verdict }
colors-contrast = Contraste { $ratio } pour 1, { $verdict }.
colors-contrast-warning = Sous 3 pour 1, c'est difficile à voir ; choisissez une couleur plus claire ou plus foncée.
colors-palette-label = Nom de surlignage { $name }
colors-palette-value = { $color }, { $shape }, contraste { $text } pour 1 avec le texte et { $page } pour 1 avec la page, { $verdict }
colors-palette-system = { $color }, { $shape } ; les couleurs de contraste élevé du système sont dessinées, et la forme le marque toujours
colors-palette-unknown = { $color }, { $shape } ; une couleur que textweaver ne peut pas mesurer
colors-intro = Couleurs, { $n } paramètres. Gauche et Droite choisissent une couleur nommée, Entrée saisit un nom ou une valeur #rrggbb, Suppr remet celle du thème, F1 dit l'aide.
settings-item-recent = { $item }, modifié récemment
settings-reset = { $label } revient à sa valeur par défaut, { $value }.
settings-row-help = { $label } : { $value }. Par défaut : { $default }. { $help }
number-group-separator = { " " }
number-decimal-separator = ,
settingsio-import-question-names =
    { $n ->
        [one] Importer { $n } paramètre modifié depuis { $name } : { $names } ? y ou n
       *[other] Importer { $n } paramètres modifiés depuis { $name } : { $names } ? y ou n
    }
settingsio-and-more = { $names } et { $n } de plus
