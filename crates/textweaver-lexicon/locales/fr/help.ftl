### The help, key names, About, and the packaged guides.

## Keys named in messages and the help.

help-the-command-palette = la palette de commandes
help-not-bound = non affectée
# Two keys, or a key and a list of keys: "p or Ctrl+P".
help-or = { $a } ou { $b }
# A command without keys: $name is its palette name, such as list highlights.
help-the-command = la commande { $name }
# One line of the keyboard shortcuts list: the command's name-* (first, so
# type-ahead finds commands), its keys (inside a 40-cell Braille line), an
# action-* help, and its category-* title.
help-entry = { $name } : { $keys }. { $help }. { $category }
help-unknown-command = Commande inconnue : { $text }.
help-shortcuts-intro = Raccourcis clavier, { $n } commandes. Tapez pour filtrer. Page suivante passe au groupe suivant, F1 explique une commande, Entrée l'exécute, Échap ferme.
help-shortcuts-title = Raccourcis clavier
# The keyboard shortcuts list, filtered: $filter is what was typed.
help-shortcuts-title-matching = Raccourcis clavier correspondant à { $filter }
# $n commands of $total match the filter.
help-shortcuts-filter-match = { $n } sur { $total } commandes correspondent.
help-shortcuts-filter-none = Aucune commande ne correspond à { $query }. Retour arrière retire des lettres.
help-shortcuts-filter-cleared = Filtre effacé, { $n } commandes.
# Moving into a group of the keyboard shortcuts list: its name, its
# size, then the row ($item, with its place in the list).
help-shortcuts-group-item =
    { $group }, { $n ->
        [one] 1 commande
       *[other] { $n } commandes
    }. { $item }
help-title = Aide
help-intro = Aide. Tapez pour chercher dans toute l'aide ; Haut et Bas déplacent, Échap ferme.

## The help list. Each value is a key or keys from the keymap.

help-about = textweaver lit les documents à voix haute. Les touches ci-dessous sont les raccourcis actuels.
help-search = Cherchez dans toute l'aide en tapant ici : commandes, touches, réglages et les guides.
help-open = Ouvrir un document : { $open }. Bibliothèque et fichiers récents : { $library }.
help-play = Lecture ou pause : { $key }.
help-read-from-cursor = Lire à partir du curseur : { $key }.
help-stop = Arrêter : { $key }.
help-sentences = Phrase suivante et précédente : { $next } et { $previous }.
help-paragraphs = Paragraphe suivant et précédent : { $next } et { $previous }.
help-headings = Titre suivant et précédent : { $next } et { $previous }. Titre à un niveau donné : { $first } à { $last }, avec Maj pour le précédent.
help-read-headings = Lire à partir du titre suivant et précédent : { $next } et { $previous }.
help-quick-keys = Touches rapides, comme dans NVDA et JAWS : liste { $list }, élément de liste { $item }, tableau { $table }, lien { $link }, citation { $quote }, séparateur { $separator }, image { $graphic }, section { $section }. Maj avec la touche va au précédent.
help-speech-cursor = Curseur de lecture, ligne par ligne : { $key }.
help-find = Rechercher : { $key }.
help-bookmark = Ajouter un signet : { $key }.
help-history = Précédent et suivant dans vos déplacements : { $back } et { $forward }.
help-rate = Plus vite et plus lentement : { $faster } et { $slower }.
help-where = Où suis-je : { $key }.
help-repeat = Répéter le dernier message : { $repeat }. Le dernier message et le statut : mode, débit, moteur vocal, et position : { $status }.
help-notes = Notes : ajouter { $add }, lister { $list }, suivante et précédente { $next } et { $previous }, supprimer celle au curseur { $delete }. Dans la liste, Suppr supprime et F2 modifie.
help-highlights = Surligner la sélection ou la phrase, ou supprimer un surlignage : { $highlight }. Avec un nom : { $first } à { $fifth }, ou la liste : { $named }. Lister les surlignages : { $list }.
help-bookmarks-list = Liste des signets : Suppr supprime un signet, F2 le renomme.
help-edit = Modifier le document : { $edit }. Enregistrer : { $save }. Enregistrer sous : { $saveas }. Nouveau document : { $new }.
help-editing = Pendant l'édition : annuler { $undo }, rétablir { $redo }, gras { $bold }. Toutes les commandes de mise en forme sont dans les raccourcis clavier.
help-outline = Plan des titres, tapez pour filtrer : { $outline }. Suivre un lien ou une note de bas de page : { $follow }.
help-tables = Tableaux : { $nextrow } et { $previousrow } déplacent par ligne, { $nextcell } et { $previouscell } par cellule.
help-citations = Citations pendant l'édition : insérer { $insert }, ajouter une référence par DOI, ISBN ou PMID { $reference }. Orthographe : faute suivante et précédente { $next } et { $previous }, suggestions { $suggestions }.
help-export = Exporter en HTML, PDF, Word, EPUB, ou braille, aperçu dans le navigateur, et créer à partir d'un modèle : tapez export, preview, ou template dans la palette de commandes.
help-verbosity = Ce qui est dit : { $verbosity }. Ponctuation dite : { $punctuation }.
help-voice = Choisir une voix : { $voice }. Redémarrer la synthèse vocale si elle s'arrête : { $restart }.
help-access = Avec un lecteur d'écran, qui parle : { $key } fait défiler autonome, hybride, et mode lecteur d'écran.
help-access-window = Qui lit : { $key } bascule entre textweaver lit à voix haute et mon lecteur d'écran lit.
help-character-keys = Raccourcis à une seule touche activés ou non, pour la dictée : { $keys }. Paramètres : { $settings }.
help-all-shortcuts = Tous les raccourcis clavier : { $key }.
help-palette = Exécuter une commande par son nom : { $key }.
# Keep the letters y, n, and a: they are the keys that answer.
help-quit = Quitter, en enregistrant votre position : { $key }, puis y pour confirmer ; n, a, ou Échap annule.

## Catégories de l'aide.

category-reading = Lecture
category-navigation = Navigation
category-speech-cursor = Curseur de lecture
category-voice = Voix
category-search = Recherche
category-bookmarks = Signets et notes
category-file = Fichier
category-editing = Édition
category-view = Affichage et aide

## Key names as textweaver's own voice says them. Written names (Ctrl+S)
## are not translated.

keyname-control = Contrôle
keyname-command = Commande
keyname-alt = Alt
keyname-shift = Majuscule
keyname-period = point
keyname-comma = virgule
keyname-semicolon = point-virgule
keyname-colon = deux-points
keyname-apostrophe = apostrophe
keyname-quote = guillemet
keyname-grave-accent = accent grave
keyname-tilde = tilde
keyname-exclamation-mark = point d'exclamation
keyname-question-mark = point d'interrogation
keyname-at-sign = arobase
keyname-number-sign = dièse
keyname-dollar-sign = dollar
keyname-percent = pourcent
keyname-caret = accent circonflexe
keyname-ampersand = esperluette
keyname-asterisk = astérisque
keyname-left-parenthesis = parenthèse ouvrante
keyname-right-parenthesis = parenthèse fermante
keyname-left-bracket = crochet ouvrant
keyname-right-bracket = crochet fermant
keyname-left-brace = accolade ouvrante
keyname-right-brace = accolade fermante
keyname-less-than = inférieur à
keyname-greater-than = supérieur à
keyname-plus = plus
keyname-minus = moins
keyname-equals = égal
keyname-underscore = tiret bas
keyname-slash = barre oblique
keyname-backslash = barre oblique inversée
keyname-vertical-bar = barre verticale
keyname-page-up = Page précédente
keyname-page-down = Page suivante
keyname-up-arrow = Flèche haut
keyname-down-arrow = Flèche bas
keyname-left-arrow = Flèche gauche
keyname-right-arrow = Flèche droite
keyname-escape = Échap
keyname-space = Espace
keyname-enter = Entrée
keyname-tab = Tabulation
keyname-backspace = Retour arrière
keyname-delete = Supprimer
keyname-insert = Insertion
keyname-home = Début
keyname-end = Fin

## Help's ways to the docs, About's facts, and first-run choices asked
## again. $address is a web address; $path a folder; facts start with
## their name so each Braille line leads with it.
about-title = À propos de textweaver
about-intro = À propos de textweaver, { $n } informations. Joignez-les à un signalement.
about-version = Version : textweaver { $version }
about-build = Compilation : { $frontend }, { $profile }, { $os } { $arch }
about-frontend-window = fenêtre
about-frontend-terminal = lecteur en terminal
about-profile-release = version publiée
about-profile-debug = débogage
about-license = Licence : { $license }
about-engine-in-use = Moteur vocal utilisé : { $engine }
about-engines = Moteurs vocaux trouvés : { $engines }
about-engines-none = Moteurs vocaux trouvés : aucun
about-components = Composants : { $installed } sur { $known } installés
about-folder-settings = Dossier des réglages : { $path }
about-folder-data = Dossier des données : { $path }
about-folder-cache = Dossier du cache : { $path }
about-no-folders = Dossiers : aucun ; cette session ne garde aucun fichier.
about-quick-start-online = Démarrage rapide introuvable à côté de textweaver. Ouvrir { $address } dans le navigateur ? y ou n
about-docs-question = Documentation : { $address }. L'ouvrir dans le navigateur ? y ou n
about-report-question = Signaler un problème : { $address }. Rien n'est envoyé. L'ouvrir dans le navigateur ? y ou n
about-first-run-again = Questions du premier démarrage réinitialisées ; posées au prochain démarrage.

## B1-hp : les guides fournis et Chercher dans l'aide.
docs-not-found = Documentation introuvable à côté de textweaver. Ouvrir { $address } dans le navigateur ? y ou n
docs-read-only = Ce guide est fourni avec textweaver et est en lecture seule. Pour le modifier, copiez le fichier ailleurs et ouvrez la copie.
docs-guide-missing =
    { $n ->
        [one] Guide manquant : { $guides }. Les autres guides s'ouvrent.
       *[other] { $n } guides manquants : { $guides }. Les autres guides s'ouvrent.
    }
helpsearch-title = Chercher dans l'aide
helpsearch-title-matching = Aide sur { $filter }
helpsearch-intro = Chercher dans l'aide, { $n } sujets. Tapez pour chercher commandes, touches, réglages et guides ; Entrée ouvre un sujet, F1 l'explique.
helpsearch-match =
    { $n ->
        [one] 1 résultat.
       *[other] { $n } résultats.
    }
helpsearch-none = Aucune aide sur { $query }. Retour arrière efface des lettres.
helpsearch-cleared =
    { $n ->
        [one] Filtre effacé, 1 sujet.
       *[other] Filtre effacé, { $n } sujets.
    }
helpsearch-command = { $row }, commande
helpsearch-setting = { $label }, réglage dans { $section }
helpsearch-guide = { $heading }, dans { $guide }
helpsearch-at-heading = Sur { $heading }.
## End of B1-hp
