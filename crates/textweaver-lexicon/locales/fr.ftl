### Les messages de l'interface de textweaver, en français.
###
### Fluent (https://projectfluent.org/), dans le sous-ensemble que lit
### textweaver-lexicon. Les identifiants et les variables sont ceux de
### en.ftl ; un message absent ici est dit en anglais.

-brand = textweaver

## Catégories grammaticales.

pos-noun = nom
pos-verb = verbe
pos-adjective = adjectif
pos-adverb = adverbe

## Définir un mot : sources.

source-glossary = votre glossaire
source-wordnet = Open English WordNet
source-cmudict = le CMU Pronouncing Dictionary

## Définir un mot : la liste des sens.

# $word is the word looked up, $n the number of senses, $source a source-* message.
define-title =
    { $n ->
        [0] Prononciation de { $word }, depuis { $source }
        [one] Définitions de { $word }, 1 sens, depuis { $source }
       *[other] Définitions de { $word }, { $n } sens, depuis { $source }
    }
# $lemma is the headword, $pos a pos-* message, $i the sense number, $n how many.
define-sense-head = { $lemma }, { $pos }, { $i } sur { $n }
define-sense-head-nopos = { $lemma }, { $i } sur { $n }
define-sense = { $head } : { $definition }.
define-example = Par exemple : { $text }.
define-synonyms = Synonymes : { $words }.
define-antonyms = Contraire : { $words }.
define-kind-of = Une sorte de : { $words }.
# $say is a respelling such as RUN-ing, with the stressed syllable in capitals.
define-pronounced = Prononcé { $say }.
define-pronounced-or = Prononcé { $say }, ou { $other }.

## Invites.

prompt-define-word = Définir quel mot ?
prompt-profile-name = Nom du nouveau profil
prompt-profile-rename = Nouveau nom du profil, Entrée le conserve
prompt-profiles-import = Importer des profils depuis un fichier
prompt-profiles-export = Exporter les profils vers un fichier, par exemple textweaver-profiles.json

## Mots courants.

common-cancelled = Annulé.
# Durations: $h hours, $m minutes, $s seconds.
duration-hours =
    { $h ->
        [one] 1 heure
       *[other] { $h } heures
    } et { $m ->
        [one] 1 minute
       *[other] { $m } minutes
    }
duration-minutes =
    { $m ->
        [one] 1 minute
       *[other] { $m } minutes
    } et { $s ->
        [one] 1 seconde
       *[other] { $s } secondes
    }
duration-seconds =
    { $s ->
        [one] 1 seconde
       *[other] { $s } secondes
    }

## Définir un mot dans le lecteur.

# $title is define-title.
define-intro = { $title }. Haut et Bas déplacent entre les sens, Entrée en copie un, Échap ferme.
define-nothing-here = Aucun mot au curseur.
define-not-found = Aucune définition trouvée pour { $word }.
define-no-dictionary = Le fichier de dictionnaire n'est pas installé ; seul votre glossaire a donc été consulté. Le guide de lecture explique comment l'installer.
define-dictionary-damaged = Impossible de lire le fichier de dictionnaire : { $error } Seul votre glossaire est consulté.
define-glossary-problem = Impossible de lire votre glossaire : { $error }
define-glossary-skipped =
    { $n ->
        [one] 1 ligne de votre glossaire n'a pas de définition et a été ignorée.
       *[other] { $n } lignes de votre glossaire n'ont pas de définition et ont été ignorées.
    }
define-copied = Copié.

## Profils de paramètres.

profiles-title =
    { $n ->
        [0] Profils de paramètres, aucun enregistré pour l'instant
        [one] Profils de paramètres, 1 profil
       *[other] Profils de paramètres, { $n } profils
    }
# $title is profiles-title.
profiles-intro = { $title }. Entrée bascule vers un profil, F2 le renomme, Suppr le supprime.
# $summary is profile-summary-* parts joined by commas.
profiles-item = { $name } : { $summary }
profiles-item-active = { $name }, utilisé : { $summary }
profiles-save-new = Enregistrer les paramètres actuels comme nouveau profil
profiles-update = Enregistrer les paramètres actuels dans { $name }
profiles-import = Importer des profils depuis un fichier
profiles-export = Exporter tous les profils vers un fichier
profile-summary-voice = voix { $voice }
profile-summary-rate = débit { $rate }
profile-summary-theme = thème { $theme }
# $mode is self voicing, hybrid, or screen reader.
profile-summary-access = mode { $mode }
profile-summary-empty = rien d'enregistré
profile-switched = Basculé vers { $name }.
profile-switched-backend = Basculé vers { $name }. Son moteur vocal sera utilisé au prochain démarrage.
# $keys lists settings such as speech.pitch.
profile-dropped =
    { $n ->
        [one] 1 paramètre qu'il contient n'est pas utilisé par cette version : { $keys }.
       *[other] { $n } paramètres qu'il contient ne sont pas utilisés par cette version : { $keys }.
    }
profile-saved = Paramètres actuels enregistrés sous { $name }.
profile-replaced = Paramètres actuels enregistrés dans { $name }.
profile-renamed = { $old } renommé en { $new }.
profile-delete-question = Supprimer le profil { $name } ? y ou n
profile-deleted = { $name } supprimé.
profile-kept = Conservé.
profile-not-found = Il n'existe aucun profil nommé { $name }.
profile-needs-name = Un profil doit avoir un nom.
profile-exists = Un profil nommé { $name } existe déjà.
profiles-not-an-export = { $detail }
profiles-no-persistence = Les profils ne sont pas enregistrés dans cette session.
profiles-read-failed = Impossible de lire le fichier de profils ; il est donc traité comme vide : { $error }
profiles-save-failed = Impossible d'enregistrer les profils : { $error } Vérifiez que le dossier des paramètres est accessible en écriture.
profiles-none-to-export = Il n'y a encore aucun profil à exporter.
profiles-exported =
    { $n ->
        [one] 1 profil exporté vers { $file }.
       *[other] { $n } profils exportés vers { $file }.
    }
profiles-export-failed = Impossible d'exporter les profils : { $error }
profiles-imported =
    { $n ->
        [0] Il n'y avait aucun profil dans { $file }.
        [one] 1 profil importé depuis { $file } : { $names }.
       *[other] { $n } profils importés depuis { $file } : { $names }.
    }

## Statistiques de lecture.

stats-title = Statistiques de lecture
stats-intro = Statistiques de lecture. Entrée sur un document l'ouvre.
stats-off = Les statistiques de lecture sont désactivées. Le dernier élément les active.
stats-empty = Aucune lecture enregistrée pour l'instant. Le temps est compté pendant que textweaver lit à voix haute.
# $time is a duration-* message.
stats-total =
    { $time } lus au total, en { $sessions ->
        [one] 1 session
       *[other] { $sessions } sessions
    }, sur { $docs ->
        [one] 1 document
       *[other] { $docs } documents
    }.
stats-current =
    Ce document : { $time } lus, point le plus avancé { $pct } pour cent, { $sessions ->
        [one] 1 session
       *[other] { $sessions } sessions
    }.
stats-current-none = Ce document n'a pas encore été lu à voix haute.
stats-most-read = Le plus lu { $rank } : { $title }, { $time }, point le plus avancé { $pct } pour cent.
stats-toggle-on = Les statistiques sont activées. Entrée les désactive.
stats-toggle-off = Les statistiques sont désactivées. Entrée les active.
stats-turned-on = Les statistiques de lecture sont activées.
stats-turned-off = Les statistiques de lecture sont désactivées. Ce qui a été enregistré est conservé.

## Listes.

study-nothing-to-delete = Rien à supprimer dans cette liste.
study-nothing-to-rename = Rien à renommer dans cette liste.
## tw stats.

stats-clear-question =
    { $n ->
        [one] Supprimer les statistiques de lecture de 1 document ? y ou n
       *[other] Supprimer les statistiques de lecture de { $n } documents ? y ou n
    }
stats-cleared = Statistiques de lecture supprimées.
stats-clear-item = Supprimer les statistiques de lecture
stats-clear-failed = Impossible de supprimer les statistiques de lecture : { $error }. Vérifiez que le dossier de données est accessible en écriture.
stats-off-cli = Les statistiques de lecture sont désactivées : stats.enabled est à false dans les paramètres.

## Continue reading and every computer's statistics (the sync wave, S6).

continue-title = Reprendre la lecture
# $n is the number of documents listed.
continue-intro =
    { $n ->
        [one] Reprendre la lecture : 1 document, le plus récent d'abord.
       *[other] Reprendre la lecture : { $n } documents, les plus récents d'abord.
    }
continue-empty = Aucune position. Lire l'enregistre.
# One row, meaning first: the title, how far in, the computer, and how
# long ago (continue-ago-*).
continue-item = { $title }, { $pct } pour cent, { $device }, { $when }
continue-this-computer = cet ordinateur
continue-ago-now = à l'instant
continue-ago-minutes =
    { $n ->
        [one] il y a 1 minute
       *[other] il y a { $n } minutes
    }
continue-ago-hours =
    { $n ->
        [one] il y a 1 heure
       *[other] il y a { $n } heures
    }
continue-ago-days =
    { $n ->
        [one] il y a 1 jour
       *[other] il y a { $n } jours
    }
name-continue-reading = Reprendre la lecture
action-continue-reading = Reprendre la lecture : les documents de cet ordinateur avec une position enregistrée, sur n'importe quel ordinateur, les plus récents d'abord
name-add-library-folder = Ajouter un dossier à la bibliothèque
action-add-library-folder = Ajouter un dossier à la bibliothèque : le choisir dans le navigateur de fichiers
name-edit-document-details = Modifier les détails
action-edit-document-details = Modifier les détails du document : titre, auteur, DOI et ISBN
prompt-document-details = Détails du document

## Edit a document's details by hand.

# $name is the document's title; said when the form opens.
details-intro = Détails de { $name }. Tab change de champ, Entrée enregistre, Échap annule.
details-label-title = Titre
details-label-author = Auteur
details-label-doi = DOI
details-label-isbn = ISBN
# A field's drawn label: its label, then $n of $total fields.
details-prompt-label = { $label }, { $n } sur { $total }
# Said on moving to a field: its label and value (or nav-blank).
details-field = { $label } : { $value }
# $fields lists the fields saved, by their labels.
details-saved = Détails enregistrés : { $fields }.
details-unchanged = Détails inchangés.
details-cancelled = Annulé. Détails inchangés.
# $text is what was typed in the DOI or ISBN field.
details-not-a-doi = Pas un DOI : { $text }. Corrigez-le ou effacez-le.
details-not-an-isbn = Pas un ISBN : { $text }. Corrigez-le ou effacez-le.
details-no-file = Ce document n'a pas de fichier, donc pas de détails à modifier.
details-save-failed = Impossible d'enregistrer les détails : { $error } Réessayez.
# The statistics list could not wait for the sync folder.
stats-others-slow = Autres ordinateurs omis : dossier lent.
stats-untitled = Document sans titre
# One computer's share of a document: $device, $time, $sessions.
stats-computer =
    { $sessions ->
        [one] { $device } : { $time }, 1 session
       *[other] { $device } : { $time }, { $sessions } sessions
    }
stats-by-computer-off = Par ordinateur : masqué. Entrée affiche.
stats-by-computer-on = Par ordinateur : affiché. Entrée masque.

## Navigation. $dir is next or previous; $what is a kind-* or unit-*
## noun and $unit its key (heading, list-item, sentence), for languages
## whose words agree with the noun.

nav-blank = vide
# Said for a picture that has no description (no alternative text).
nav-no-description = aucune description
# The document overview: the title first, then the counts. $time is overview-time.
overview-line = { $title }. Titres : { $headings }, tableaux : { $tables }, images : { $pictures }, notes : { $footnotes }. { $time }
# Reading time left from the cursor at the current rate.
overview-time =
    { $minutes ->
        [0] Il reste moins d'une minute.
        [one] Il reste environ 1 minute.
       *[other] Il reste environ { $minutes } minutes.
    }
# Said when the reading pass changes and as reading starts in a skim. $pass is a reading-pass-* name.
reading-pass-changed = Passe : { $pass }.
reading-pass-full = texte entier
reading-pass-first-sentences = premières phrases
reading-pass-headings = titres
# High verbosity: $label is a structure label ("Heading level 2").
nav-message-at-labelled = { $label }, ligne { $line }, { $pct } pour cent : { $content }
nav-message-at = Ligne { $line }, { $pct } pour cent : { $content }
nav-message-labelled = { $label } : { $content }
# $message is the navigation message after wrapping around.
nav-wrapped = Bouclé. { $message }
nav-no-next =
    { $unit ->
        [list] Pas de { $what } suivante.
        [row] Pas de { $what } suivante.
        [cell] Pas de { $what } suivante.
        [graphic] Pas de { $what } suivante.
        [quote] Pas de { $what } suivante.
        [page] Pas de { $what } suivante.
        [section] Pas de { $what } suivante.
        [footnote] Pas de { $what } suivante.
        [math] Pas de { $what } suivante.
        [sentence] Pas de { $what } suivante.
        [line] Pas de { $what } suivante.
       *[other] Pas de { $what } suivant.
    }
nav-no-previous =
    { $unit ->
        [list] Pas de { $what } précédente.
        [row] Pas de { $what } précédente.
        [cell] Pas de { $what } précédente.
        [graphic] Pas de { $what } précédente.
        [quote] Pas de { $what } précédente.
        [page] Pas de { $what } précédente.
        [section] Pas de { $what } précédente.
        [footnote] Pas de { $what } précédente.
        [math] Pas de { $what } précédente.
        [sentence] Pas de { $what } précédente.
        [line] Pas de { $what } précédente.
       *[other] Pas de { $what } précédent.
    }
nav-nothing-to-read = Pas de { $what } à lire.
nav-label-heading-level = Titre de niveau { $level }
nav-label-list =
    { $n ->
        [0] Liste
        [one] Liste, 1 élément
       *[other] Liste, { $n } éléments
    }
nav-label-table =
    { $n ->
        [0] Tableau
        [one] Tableau, 1 ligne
       *[other] Tableau, { $n } lignes
    }
nav-label-list-item-level = Élément de liste, niveau { $level }
nav-no-heading-level =
    { $dir ->
        [next] Pas de titre suivant de niveau { $level }.
       *[previous] Pas de titre précédent de niveau { $level }.
    }
# Said before a line's text on caret moves.
nav-line-heading = titre de niveau { $level }
nav-line-row = ligne { $n }
nav-line-list-item-level = élément de liste, niveau { $level }
# $language is a programming language name such as Python.
nav-line-code-language = code, { $language }
nav-no-chapters = Ce document n'a aucun chapitre.
nav-chapter = Chapitre
nav-no-chapter =
    { $dir ->
        [next] Pas de chapitre suivant.
       *[previous] Pas de chapitre précédent.
    }
nav-back = Précédent
nav-forward = Suivant
nav-page = Page
nav-percent = { $pct } pour cent
nav-no-earlier-history = Aucun historique antérieur.
nav-no-forward-history = Aucun historique suivant.
# $label is nav-back, nav-forward, nav-page, or nav-percent.
nav-label-line = { $label }, ligne { $line }
nav-top-of-document = Haut du document
nav-end-of-document = Fin du document
# $edge is nav-top-of-document or nav-end-of-document; $content the line there.
nav-edge-message = { $edge }. { $content }
nav-top-of-document-stop = Haut du document.
nav-end-of-document-stop = Fin du document.
nav-position = Ligne { $line } sur { $lines }, { $pct } pour cent.
nav-position-word = Mot { $word } sur { $words }.
nav-position-heading = Sous le titre { $heading }.
nav-position-mode = Mode { $mode }.

## Names of structure, units, and modes (said inside other messages).

kind-heading = titre
kind-paragraph = paragraphe
kind-list-item = élément de liste
kind-list = liste
kind-table = tableau
kind-row = ligne
kind-cell = cellule
kind-link = lien
kind-graphic = image
kind-code = code
kind-quote = citation
kind-page = page
kind-section = section
kind-bold = gras
kind-italic = italique
kind-underline = souligné
kind-footnote = note de bas de page
kind-strikethrough = barré
kind-separator = séparateur
kind-math = formule mathématique
unit-character = caractère
unit-word = mot
unit-sentence = phrase
unit-line = ligne
unit-paragraph = paragraphe
unit-document = document
# $what is a kind-* noun, $unit its key.
unit-with-level = { $what } niveau { $level }
mode-browse = Parcours
mode-speech-cursor = Curseur de lecture
mode-edit = Édition
mode-find = Rechercher
mode-command = Commande
mode-go-to = Aller à
mode-open = Ouvrir
mode-prompt = Invite
common-on = activé
common-off = désactivé

## Reading aloud.

playback-caps-no-words = Cette voix ne signale pas les mots ; le surlignage du mot est donc estimé.
playback-caps-words = Cette voix signale chaque mot ; le surlignage le suit donc exactement.
playback-caps-no-pitch = La hauteur ne peut pas être changée avec cette voix.
playback-caps-no-volume = Le volume ne peut pas être changé avec cette voix.
# The reading state on the title line, one word.
state-reading = Lecture
state-paused = En pause
state-stopped = Arrêté
state-ready = Prêt
playback-reading-at = Lecture à { $rate } mots par minute.
playback-paused = En pause.
playback-stopped-speech-cursor-off = Arrêté. Curseur de lecture désactivé.
playback-stopped = Arrêté.
playback-search-cleared = Recherche effacée.
# $key is the key that turns edit mode off.
playback-still-editing = Toujours en cours d'édition. { $key } termine.
playback-end-of-document-content = fin du document
playback-no-unit-here = Pas de { $what } ici.
playback-no-selection = Aucune sélection.
# $next is what happens now (a restart, or nothing).
playback-speech-died = La synthèse vocale s'est arrêtée ({ $reason }). { $next }
playback-done-reading = Lecture terminée.
playback-speech-restarted = Synthèse vocale redémarrée : { $reason }. Reprise de la lecture au dernier mot.
playback-speech-error = Erreur de synthèse vocale : { $error }

## The title line and Say Status.

# The title line's position.
status-position = ligne { $line } sur { $lines }
status-mode = mode { $mode }
status-modified = modifié
status-self-voicing = autonome
status-hybrid = hybride
status-screen-reader = mode lecteur d'écran
status-rate-spoken = { $wpm } mots par minute
status-rate = { $wpm } mpm
# The window's status bar, last: reading time left at the current rate.
status-time-left =
    { $minutes ->
        [0] moins d'une minute restante
        [one] 1 minute restante
       *[other] { $minutes } minutes restantes
    }
status-no-document = Aucun document
# $parts are the title line's parts, joined with commas.
status-said = { $title } : { $parts }.
status-no-message = Aucun message pour l'instant.
# A list shown without its own introduction.
status-list-intro =
    { $n ->
        [one] { $title }, 1 élément. Haut et Bas déplacent, Entrée choisit, Échap ferme.
       *[other] { $title }, { $n } éléments. Haut et Bas déplacent, Entrée choisit, Échap ferme.
    }

## Questions and answers. Keep the letters y and n: they are the keys
## that answer.

common-press-y-or-n = Appuyez sur y ou n.
common-kept = Conservé.
confirm-quit = Quitter textweaver ? y ou n
confirm-delete-note = Supprimer cette note ou ce surlignage ? y ou n
notes-remove-highlight-question = Supprimer ce surlignage ? y ou n
notes-delete-note-question = Supprimer cette note ? y ou n
list-nothing-to-mark = Rien à marquer dans cette liste.
notes-editing = Modification de la note : { $text }
# $key opens a document.
app-no-document-open = Aucun document n'est ouvert. Appuyez sur { $key } pour en ouvrir un.
app-window-only = Cette commande fonctionne dans la fenêtre de textweaver.
app-terminal-only = Cette commande fonctionne dans le lecteur en mode terminal.
settings-save-failed = Impossible d'enregistrer les paramètres : { $error } Vos changements restent actifs jusqu'à la fermeture.
settings-outside-kept = Les paramètres modifiés en dehors de textweaver ont été conservés.
edit-still-editing = Toujours en cours d'édition.
goto-not-a-target = Ce n'est pas une cible valide : { $text }. Tapez un numéro de ligne, un pourcentage tel que 50%, start, ou end.

## Opening a document.

open-opened = { $title } ouvert.
open-resumed = { $title } ouvert. Reprise à { $pct } pour cent.
open-resumed-synced = { $title } ouvert. Reprise à { $pct } pour cent, depuis un autre appareil.

## Prompts: the label is shown and said when the prompt opens.

prompt-find = Rechercher
prompt-go-to = Aller à une ligne, un pourcentage, start, ou end
prompt-open = Ouvrir un fichier
prompt-command = Commande
# $label is prompt-command.
prompt-command-palette-intro = { $label }. Tapez une partie d'un nom ; Tab complète, Haut et Bas parcourent les correspondances.
prompt-save-as = Enregistrer sous
prompt-table-size = Taille du tableau, colonnes par lignes, par exemple 3 by 2
prompt-image-path = Fichier image
prompt-replace-find = Remplacer, rechercher quoi
prompt-replace-with = Remplacer par
prompt-note = Note
prompt-edit-note = Modifier la note, Entrée la conserve
prompt-rename-bookmark = Nouveau nom du signet, Entrée le conserve
prompt-export-settings = Exporter les paramètres vers un fichier, par exemple textweaver-settings.json
prompt-import-settings = Importer les paramètres depuis un fichier
prompt-citation-locator = Page ou autre repère, par exemple 12 ou chapitre 2 ; Entrée pour aucun
prompt-reference-identifier = DOI ou ISBN à ajouter
prompt-import-references = Importer des références depuis un fichier
prompt-template-title = Titre du nouveau document
prompt-setting-value = Nouvelle valeur, Entrée la conserve

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
help-shortcuts-intro = Raccourcis clavier, { $n } commandes. Haut et Bas déplacent, Entrée exécute, Échap ferme.
help-shortcuts-title = Raccourcis clavier
help-title = Aide
help-intro = Aide. Haut et Bas déplacent, Échap ferme.

## The help list. Each value is a key or keys from the keymap.

help-about = textweaver lit les documents à voix haute. Les touches ci-dessous sont les raccourcis actuels.
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
help-highlights = Surligner la sélection ou la phrase, ou supprimer un surlignage : { $highlight }. Lister les surlignages : { $list }.
help-bookmarks-list = Liste des signets : Suppr supprime un signet, F2 le renomme.
help-edit = Modifier le document : { $edit }. Enregistrer : { $save }. Enregistrer sous : { $saveas }. Nouveau document : { $new }.
help-editing = Pendant l'édition : annuler { $undo }, rétablir { $redo }, gras { $bold }. Toutes les commandes de mise en forme sont dans les raccourcis clavier.
help-outline = Plan des titres, tapez pour filtrer : { $outline }. Suivre un lien ou une note de bas de page : { $follow }.
help-tables = Tableaux : { $nextrow } et { $previousrow } déplacent par ligne, { $nextcell } et { $previouscell } par cellule.
help-citations = Citations pendant l'édition : insérer { $insert }, ajouter une référence par DOI ou ISBN { $reference }. Orthographe : faute suivante et précédente { $next } et { $previous }, suggestions { $suggestions }.
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

## Commands: the one-line help of each, in the keyboard shortcuts list and
## the command palette. Their ids (play_pause) stay as they are.

action-play-pause = Lire ou mettre en pause la lecture à partir du mot actuel
action-stop = Arrêter la lecture
action-read-from-cursor = Lire en continu à partir du curseur
action-read-document = Lire tout le document depuis le début
action-read-current-character = Dire le caractère au curseur
action-read-current-word = Dire le mot au curseur
action-read-current-sentence = Dire la phrase au curseur sans se déplacer
action-read-current-line = Dire la ligne au curseur
action-read-paragraph = Dire le paragraphe au curseur sans se déplacer
action-read-selection = Lire le texte sélectionné
action-say-position = Dire la position : ligne, pourcentage, numéro de mot, et titre
action-say-status = Répéter le dernier message, puis dire le statut : mode, état de lecture, position, débit, et moteur vocal ; dans une liste, l'introduction de la liste
action-repeat-message = Répéter le dernier message
action-word-count = Dire le nombre de mots dans le document, ou dans la sélection
action-link-address = Dire l'adresse du lien au curseur
action-replay-sentence = Relire depuis le début de la phrase actuelle
action-replay-paragraph = Relire depuis le début du paragraphe actuel
action-rsvp-toggle = Afficher ou masquer le RSVP : un mot à la fois, à partir du curseur
action-rsvp-play-pause = Démarrer ou mettre en pause le RSVP
action-rsvp-faster = RSVP plus vite
action-rsvp-slower = RSVP plus lent
action-rsvp-position-next = Déplacer le mot du RSVP vers la prochaine position à l'écran
action-reading-level = Dire le niveau de lecture du document ou de la sélection
action-document-overview = Dire le titre du document, combien il a de titres, de tableaux, d'images et de notes, et environ combien de minutes il reste
action-reading-pass = Changer ce que dit la lecture : le texte entier, la première phrase de chaque paragraphe avec les titres, ou seulement les titres
action-define-word = Définir le mot au curseur, ou les mots sélectionnés : sens, exemples, synonymes, et prononciation
action-toggle-citations = Activer ou désactiver les citations en lecture continue : désactivé les ignore, activé les dit en mots
action-explore-math = Explorer la formule mathématique au curseur terme par terme : les flèches déplacent, Bas entre dans une partie, Haut en sort, Échap quitte
action-listen-rendered = Écouter le document tel qu'il sera rendu, sans quitter le mode édition
action-next-sentence = Passer à la phrase suivante
action-previous-sentence = Passer à la phrase précédente, ou au début de celle-ci après plus de trois mots
action-next-paragraph = Passer au paragraphe suivant
action-previous-paragraph = Passer au paragraphe précédent
action-next-heading = Lire à partir du titre suivant
action-previous-heading = Lire à partir du titre précédent
action-skip-next-heading = Passer au titre suivant sans lire
action-skip-previous-heading = Passer au titre précédent sans lire
action-outline = Lister les titres : tapez pour filtrer, Entrée va à l'un d'eux
action-next-heading-level-1 = Passer au titre suivant de niveau 1
action-next-heading-level-2 = Passer au titre suivant de niveau 2
action-next-heading-level-3 = Passer au titre suivant de niveau 3
action-next-heading-level-4 = Passer au titre suivant de niveau 4
action-next-heading-level-5 = Passer au titre suivant de niveau 5
action-next-heading-level-6 = Passer au titre suivant de niveau 6
action-previous-heading-level-1 = Passer au titre précédent de niveau 1
action-previous-heading-level-2 = Passer au titre précédent de niveau 2
action-previous-heading-level-3 = Passer au titre précédent de niveau 3
action-previous-heading-level-4 = Passer au titre précédent de niveau 4
action-previous-heading-level-5 = Passer au titre précédent de niveau 5
action-previous-heading-level-6 = Passer au titre précédent de niveau 6
action-next-table = Passer au tableau suivant
action-previous-table = Passer au tableau précédent
action-next-list = Passer à la liste suivante
action-previous-list = Passer à la liste précédente
action-next-list-item = Passer à l'élément de liste suivant
action-previous-list-item = Passer à l'élément de liste précédent
action-next-link = Passer au lien suivant
action-previous-link = Passer au lien précédent
action-next-block-quote = Passer à la citation suivante
action-previous-block-quote = Passer à la citation précédente
action-next-separator = Passer au séparateur suivant (ligne horizontale)
action-previous-separator = Passer au séparateur précédent (ligne horizontale)
action-next-graphic = Passer à l'image suivante
action-previous-graphic = Passer à l'image précédente
action-follow-link = Suivre le lien au curseur, ou aller entre une note de bas de page et sa note
action-table-next-row = Dans un tableau, descendre d'une ligne dans la même colonne
action-table-previous-row = Dans un tableau, monter d'une ligne dans la même colonne
action-table-next-column = Dans un tableau, passer à la cellule suivante de la ligne
action-table-previous-column = Dans un tableau, passer à la cellule précédente de la ligne
action-next-chapter = Passer au chapitre ou à la section suivante
action-previous-chapter = Passer au chapitre ou à la section précédente
action-history-back = Retourner là où vous étiez avant le dernier déplacement
action-history-forward = Avancer de nouveau après être revenu en arrière
action-go-to = Aller à une ligne, un pourcentage, ou une position
action-document-start = Passer au début du document
action-document-end = Passer à la fin du document
action-caret-next-word = Déplacer le curseur au mot suivant
action-caret-previous-word = Déplacer le curseur au mot précédent
action-caret-next-line = Déplacer le curseur à la ligne suivante
action-caret-previous-line = Déplacer le curseur à la ligne précédente
action-select-next-word = Étendre la sélection au mot suivant
action-select-previous-word = Étendre la sélection au mot précédent
action-select-next-line = Étendre la sélection à la ligne suivante
action-select-previous-line = Étendre la sélection à la ligne précédente
action-page-down = Descendre d'un écran
action-page-up = Monter d'un écran
action-scroll-down = Défiler vers le bas d'une ligne sans déplacer le curseur
action-scroll-up = Défiler vers le haut d'une ligne sans déplacer le curseur
action-speech-cursor-toggle = Entrer ou quitter le mode Curseur de lecture (ligne)
action-speech-cursor-next-line = Curseur de lecture : lire la ligne suivante
action-speech-cursor-previous-line = Curseur de lecture : lire la ligne précédente
action-speech-cursor-reread-line = Curseur de lecture : relire la ligne actuelle
action-speech-cursor-exit-and-read = Curseur de lecture : quitter et lire à partir de cette ligne
action-rate-up = Parler plus vite
action-rate-down = Parler plus lentement
action-pitch-up = Augmenter la hauteur
action-pitch-down = Baisser la hauteur
action-volume-up = Plus fort
action-volume-down = Moins fort
action-cycle-speed-preset = Faire défiler les préréglages de vitesse (survol, normal, étude, lent)
action-choose-voice = Choisir une voix
action-restart-speech = Redémarrer la synthèse vocale avec les paramètres actuels (après l'arrêt du moteur vocal)
action-cycle-verbosity = Faire défiler la verbosité de textweaver : faible, normale, élevée
action-cycle-punctuation = Faire défiler la ponctuation dite : aucune, quelques signes, tous
action-find = Rechercher du texte dans le document
action-find-next = Rechercher la correspondance suivante
action-find-previous = Rechercher la correspondance précédente
action-next-misspelling = Passer au mot mal orthographié suivant, et l'épeler
action-previous-misspelling = Passer au mot mal orthographié précédent, et l'épeler
action-spelling-suggestions = Lister les suggestions pour le mot mal orthographié au curseur, ou l'ajouter à votre liste de mots
action-next-grammar-problem = Passer au problème de grammaire suivant, et le dire avec sa correction
action-previous-grammar-problem = Passer au problème de grammaire précédent, et le dire avec sa correction
action-next-lint-problem = En mode édition, passer au problème Markdown suivant, et le dire
action-previous-lint-problem = En mode édition, passer au problème Markdown précédent, et le dire
action-add-bookmark = Ajouter un signet au curseur
action-list-bookmarks = Lister les signets
action-next-bookmark = Passer au signet suivant
action-previous-bookmark = Passer au signet précédent
action-add-note = Ajouter une note à la sélection ou à la phrase au curseur
action-list-notes = Lister les notes
action-next-note = Passer à la note suivante
action-previous-note = Passer à la note précédente
action-delete-note = Supprimer la note ou le surlignage au curseur
action-highlight-selection = Surligner la sélection, ou la phrase au curseur
action-export-study-sheet = Exporter les notes et surlignages en fiche d'étude Markdown, groupés par titre
action-open = Ouvrir un document
action-open-path = Ouvrir un document en tapant son chemin
action-open-library = Ouvrir la bibliothèque : documents dans vos dossiers de bibliothèque et fichiers récents
action-new-document = Commencer un nouveau document en mode édition
action-save = Enregistrer (Markdown et texte sur place ; autres formats en Markdown)
action-save-as = Enregistrer sous un nouveau nom
action-export-settings = Exporter les paramètres et les touches personnalisées vers un fichier JSON ou TOML
action-import-settings = Importer les paramètres depuis un fichier JSON ou TOML, après une confirmation
action-reading-statistics = Lister les statistiques de lecture : temps lu, point le plus avancé, sessions, et les documents les plus lus
action-new-from-template = Commencer un nouveau document à partir d'un modèle, avec un titre, un auteur, une date, et une rubrique Références
action-export-html = Exporter le document en page web (HTML) à côté de lui
action-export-pdf = Exporter le document en PDF balisé à côté de lui
action-export-docx = Exporter le document en fichier Word (DOCX) à côté de lui
action-export-epub = Exporter le document en livre EPUB à côté de lui
action-export-brf = Exporter le document en braille (BRF) à côté de lui
action-preview-in-browser = Aperçu du document dans le navigateur, avec les mathématiques ; chaque enregistrement réécrit l'aperçu
action-toggle-preview-auto-reload = Activer ou désactiver le rechargement automatique de l'aperçu dans le navigateur
action-toggle-preview-live = Activer ou désactiver l'aperçu en direct : avec le rechargement automatique, l'aperçu se recharge aussi quand la frappe fait une pause
action-quit = Quitter, en enregistrant la position de lecture
action-toggle-edit-mode = Basculer entre lecture et édition
action-undo = Annuler
action-redo = Rétablir
action-bold = Mettre la sélection en gras
action-italic = Mettre la sélection en italique
action-underline = Souligner la sélection
action-strikethrough = Barrer la sélection
action-inline-code = Marquer la sélection comme code
action-code-block = Faire des lignes sélectionnées un bloc de code
action-insert-link = Faire de la sélection un lien
action-heading = Faire de la ligne actuelle un titre
action-bullet-list = Faire des lignes sélectionnées une liste à puces
action-numbered-list = Faire des lignes sélectionnées une liste numérotée
action-block-quote = Faire des lignes sélectionnées une citation
action-horizontal-rule = Insérer une ligne horizontale
action-insert-table = Insérer un tableau
action-add-table-row = Ajouter une ligne au tableau au curseur
action-insert-image = Insérer une image
action-replace = Rechercher et remplacer
action-copy = Copier la sélection, ou la phrase au curseur, dans le presse-papiers
action-cut = Couper la sélection dans le presse-papiers
action-next-table-cell = Dans un tableau, passer à la cellule suivante et dire sa colonne ; ailleurs, taper une tabulation
action-previous-table-cell = Dans un tableau, passer à la cellule précédente et dire sa colonne
action-cycle-typing-echo = Faire défiler l'écho de frappe : caractères et mots, caractères, mots, ou aucun
action-select-all = Sélectionner tout le texte
action-delete-word-before = Supprimer le mot avant le curseur
action-delete-word-after = Supprimer le mot après le curseur
action-paste = Coller le texte le plus récemment copié ou coupé dans textweaver ; le collage du terminal fonctionne aussi
action-insert-citation = Insérer une citation : choisir une référence, puis donner une page ou un autre repère
action-add-reference = Ajouter une référence à votre bibliothèque par DOI ou ISBN
action-insert-bibliography = Insérer la bibliographie des ouvrages cités, au curseur
action-check-citations = Vérifier les citations : leur nombre, et lesquelles n'ont pas de clé dans votre bibliothèque
action-import-references = Importer des références depuis un fichier BibTeX, RIS, ou CSL-JSON dans votre bibliothèque
action-next-theme = Passer au thème de couleur suivant
action-toggle-line-numbers = Afficher ou masquer les numéros de ligne
action-toggle-character-keys = Activer ou désactiver les raccourcis à une seule touche, afin que la dictée et la frappe ne déclenchent jamais de commandes
action-cycle-access-mode = Faire défiler le mode d'accessibilité : autonome, hybride, ou lecteur d'écran
action-settings-profiles = Lister les profils de paramètres : basculer vers l'un d'eux, enregistrer les paramètres actuels comme profil, renommer, supprimer, importer, ou exporter
action-bionic-toggle = Activer ou désactiver la lecture bionique : le début de chaque mot en gras
action-ruler-cycle = Faire défiler la règle de lecture : désactivée, ligne actuelle, règle
action-syllables-toggle = Afficher ou masquer les syllabes : mots séparés par un point médian
action-difficult-words-toggle = Activer ou désactiver le marquage des mots difficiles : soulignés, et nommés lors des déplacements de mots en verbosité élevée
action-text-larger = Agrandir le texte du document
action-text-smaller = Réduire le texte du document
action-text-size-reset = Remettre le texte du document à sa taille normale
action-choose-font = Choisir la police du texte du document
action-contents-panel = Afficher le panneau Sommaire à côté du document et y aller, ou le fermer depuis l'intérieur : Entrée va à un titre
action-notes-panel = Afficher le panneau Notes à côté du document et y aller, ou le fermer depuis l'intérieur : Entrée va à une note
action-toggle-header = Afficher ou masquer l'en-tête, la barre de commandes au-dessus du document
action-toggle-toolbar = Afficher ou masquer la barre d'outils, la barre des boutons de lecture
action-next-region = Aller à la partie suivante de la fenêtre : l'en-tête, le panneau, le document ou la barre d'outils
action-previous-region = Aller à la partie précédente de la fenêtre
action-command-palette = Exécuter une commande par son nom
action-settings = Ouvrir les paramètres : chaque option avec son aide ; Gauche et Droite changent une valeur
action-keyboard-help = Lister les raccourcis clavier
action-help = Ouvrir l'aide

## The interface language. $language is the language's name in itself
## (Español), $voice a voice's name.

language-voice-changed = La voix est maintenant { $voice }, pour { $language }.
language-voice-kept = Aucune voix pour { $language } dans ce moteur vocal ; { $voice } continue donc de parler.
language-list-title = Langue
language-list-intro =
    { $n ->
        [one] Langue, 1 choix. Haut et Bas déplacent, Entrée choisit, Échap conserve la langue.
       *[other] Langue, { $n } choix. Haut et Bas déplacent, Entrée choisit, Échap conserve la langue.
    }

## Restarting speech.

restart-silent-now = { -brand } est silencieux maintenant ; redémarrez-le pour entendre la synthèse vocale de nouveau.
# $keys names the Restart Speech key or keys.
restart-silent-use-key = { -brand } est silencieux maintenant. Redémarrez la synthèse vocale avec { $keys }.
restart-restarting = Redémarrage de la synthèse vocale.
restart-not-here = La synthèse vocale ne peut pas être redémarrée ici.
restart-already = La synthèse vocale est déjà en cours de redémarrage.
# $error is the system's reason, in its own words.
restart-failed = Impossible de redémarrer la synthèse vocale : { $error }
restart-start-failed = Impossible de redémarrer la synthèse vocale : le démarrage a échoué.
restart-no-engine = Aucun moteur vocal n'est disponible ; { -brand } reste silencieux. Voir Troubleshooting, No speech at all, dans la documentation.
restart-done-silent = Synthèse vocale redémarrée, mais aucun moteur vocal n'est disponible ; { -brand } reste silencieux.
restart-done = Synthèse vocale redémarrée.

## Tab completion of file paths in prompts.

# $folder is the folder's full path.
pathc-no-folder = Il n'y a pas de dossier { $folder }.
# $prefix is what was typed after the last separator.
pathc-no-match = Aucun fichier ou dossier ne commence par { $prefix }.
# The one name that matched; $kind is folder or file.
pathc-one =
    { $kind ->
        [folder] { $name }, dossier
       *[file] { $name }, fichier
    }
# $n names matched; $names are the first few, joined with commas; $more is
# yes when more matched than are read out.
pathc-many =
    { $more ->
        [yes] { $n } correspondances : { $names }, et d'autres.
       *[no] { $n } correspondances : { $names }.
    }

## Exporting and importing settings.

# $n settings differ; $name is the file's name.
settingsio-import-question =
    { $n ->
        [one] Importer { $n } paramètre modifié depuis { $name } ? y ou n
       *[other] Importer { $n } paramètres modifiés depuis { $name } ? y ou n
    }
settingsio-no-persistence = Les paramètres ne sont pas enregistrés dans cette session ; ils ne peuvent donc pas être exportés ou importés.
# $path is the file written.
settingsio-exported = Paramètres exportés vers { $path }.
settingsio-export-failed = Impossible d'exporter les paramètres : { $error }
# $path is the file; $error the system's reason.
settingsio-read-failed = Impossible de lire { $path } : { $error }
settingsio-nothing-to-import = Rien à importer : vos paramètres correspondent déjà à ce fichier.
settingsio-cancelled-unchanged = Annulé. Rien n'a été modifié.
settingsio-import-failed = Impossible d'importer les paramètres : { $error }
# $summary lists what changed (from the settings store, in English).
settingsio-imported = Paramètres importés. { $summary }
settingsio-backend-next-start = Le nouveau moteur vocal sera utilisé au prochain démarrage.
settingsio-backed-up = Les anciens paramètres ont été sauvegardés.

## Opening a document: failures and opening in the background.

# $name is the file's name.
opening-is-folder = { $name } est un dossier, pas un document. Donnez le nom d'un fichier qu'il contient.
# Said after "Could not open NAME:", so it starts in lower case.
opening-no-file-in = il n'y a aucun fichier nommé { $name } dans { $folder }. Vérifiez le nom.
opening-no-file-here = il n'y a aucun fichier nommé { $name } ici. Vérifiez le nom.
opening-no-permission = vous n'avez pas la permission de le lire.
opening-damaged-rtf = ce n'est pas un fichier RTF lisible, il est peut-être endommagé.
opening-damaged-odt = ce n'est pas un fichier texte OpenDocument lisible, il est peut-être endommagé.
opening-damaged-latex = ce n'est pas un fichier LaTeX lisible, il est peut-être endommagé ou trop volumineux.
opening-damaged-email = ce n'est pas un message électronique lisible, il est peut-être endommagé ou trop volumineux.
opening-damaged-mhtml = ce n'est pas une archive web lisible, elle est peut-être endommagée ou trop volumineuse.
# $reason is one of the opening-no-* messages, or the loader's own words.
opening-failed = Impossible d'ouvrir { $name } : { $reason }
opening-started = Ouverture de { $name }. Échap annule.
opening-stopped = Ouverture de { $name } arrêtée.
opening-still = Ouverture de { $name } toujours en cours, { $secs } secondes.
# $step is the loader's report, such as "recognizing text on page 3 (3 of 40)."
opening-still-step = Ouverture de { $name } toujours en cours : { $step }
opening-stopped-unexpectedly = Impossible d'ouvrir { $name } : le chargement s'est arrêté de façon inattendue.

## A build without the publish feature. "tw convert" is a command typed
## at the terminal: keep it as it is.

lean-citations-not-in-build = Les citations ne sont pas incluses dans cette version de { -brand }.
lean-publish-not-in-build = L'export et l'aperçu ne sont pas inclus dans cette version de { -brand }. tw convert continue de convertir.

## The voice manager's list.

# $n voices are shown; $language is a language name or voices-all-languages;
# $engine an engine's name or voices-all-engines.
voices-shown =
    { $n ->
        [one] { $n } voix : { $language }, { $engine }.
       *[other] { $n } voix : { $language }, { $engine }.
    }
voices-all-languages = toutes les langues
voices-all-engines = tous les moteurs
# The filter rows at the top of the list.
voices-language-row = Langue : { $language }
voices-engine-row = Moteur : { $engine }
voices-fetch-row = Télécharger la liste des voix Piper depuis Internet
# Parts of a voice's row, joined with commas. $size is in megabytes, such
# as "63 MB".
voices-download-size = téléchargement { $size }
voices-licence-public-domain = domaine public
voices-licence-attribution = libre avec attribution
voices-licence-share-alike = libre avec attribution, partage dans les mêmes conditions
voices-licence-non-commercial = non commercial
voices-licence-unknown = licence affichée avant le téléchargement
voices-favourite = favorite
voices-current = actuelle

## Language names in the voice manager's language filter.

voices-language-ar = arabe
voices-language-ca = catalan
voices-language-cs = tchèque
voices-language-cy = gallois
voices-language-da = danois
voices-language-de = allemand
voices-language-el = grec
voices-language-en = anglais
voices-language-es = espagnol
voices-language-fa = persan
voices-language-fi = finnois
voices-language-fr = français
voices-language-hi = hindi
voices-language-hu = hongrois
voices-language-is = islandais
voices-language-it = italien
voices-language-ja = japonais
voices-language-ka = géorgien
voices-language-kk = kazakh
voices-language-ko = coréen
voices-language-lb = luxembourgeois
voices-language-lv = letton
voices-language-nl = néerlandais
voices-language-no = norvégien
voices-language-pl = polonais
voices-language-pt = portugais
voices-language-ro = roumain
voices-language-ru = russe
voices-language-sk = slovaque
voices-language-sl = slovène
voices-language-sr = serbe
voices-language-sv = suédois
voices-language-sw = swahili
voices-language-tr = turc
voices-language-uk = ukrainien
voices-language-vi = vietnamien
voices-language-zh = chinois

## Voices: the voice manager, rate, pitch, and volume.

# Spoken by a newly chosen voice as its sample.
voice-sample = Portez ce vieux whisky au juge blond qui fume.
voice-list-title = Choisir une voix
voice-still-loading = Les voix sont encore en cours de chargement. La liste s'ouvre dès qu'elles sont prêtes.
voice-list-failed = Impossible de lister les voix : { $error }
# $shown is voices-shown ("12 voices: English, all engines."). Enter,
# Space, Delete and Escape are the list's own keys.
voice-manager-intro = Gestionnaire de voix. { $shown } Entrée utilise une voix et en dit un échantillon, ou en télécharge une ; { $preview } écoute une voix ; Espace marque une favorite ; Suppr supprime une voix téléchargée ; Échap ferme.
voice-more-ready =
    { $n ->
        [one] Une voix supplémentaire d'un autre moteur est dans la liste.
       *[other] { $n } voix supplémentaires d'autres moteurs sont dans la liste.
    }
voice-preview = Écoute : { $voice }.
voice-preview-sample = { $voice }. Portez ce vieux whisky au juge blond qui fume.
voice-preview-starting = Écoute : { $voice }, démarrage de { $engine }.
voice-preview-not-installed = { $voice } n'est pas encore téléchargée. Entrée la télécharge, après une question.
voice-preview-unavailable = { $engine } ne peut pas démarrer ici pour une écoute. Entrée passe à ce moteur.
voice-preview-engine-failed = Impossible de démarrer { $engine } pour écouter { $voice }.
voice-preview-failed = Impossible d'écouter { $voice } : { $error }
# $keys names the Choose Voice key.
voice-ready = Les voix sont prêtes. { $keys } les liste.
voice-fetch-catalog-question = Télécharger la liste des voix Piper, environ 250 kilo-octets, depuis Hugging Face ? y ou n
voice-fetch-catalog-question-short = Télécharger la liste des voix Piper ? y ou n
# $engine is the engine's name, such as "Piper neural voices".
voice-switching-engine = Voix { $voice }, sur { $engine }. Changement de moteur.
voice-download-in-progress = Un téléchargement de voix est déjà en cours.
voice-no-data-folder = Il n'y a aucun dossier de données pour conserver les voix Piper.
voice-not-in-list = Cette voix n'est plus dans la liste des voix Piper.
voice-download-start-failed = Impossible de démarrer le téléchargement.
voice-reading-licence = Lecture de la licence de { $voice }.
voice-remove-question = Supprimer la voix { $voice } ? y ou n
voice-only-piper-removable = Seules les voix Piper téléchargées peuvent être supprimées.
# $plan describes the download: the voice, its size and licence.
voice-download-question = { $plan } y ou n
voice-in-use = { $voice } est la voix utilisée. Choisissez d'abord une autre voix.
voice-removed = { $voice } supprimée.
voice-remove-failed = Impossible de supprimer { $voice } : { $error }
voice-downloading-catalog = Téléchargement de la liste des voix Piper.
voice-downloading = Téléchargement de { $voice }.
voice-downloading-percent = Téléchargement de { $voice }, { $pct } pour cent.
voice-details-failed = Impossible de lire les détails de la voix : { $error }
voice-download-stopped = Le téléchargement s'est arrêté.
voice-catalog-fetched = La liste des voix Piper contient { $voices } voix en { $languages } langues. La commande Voix les liste.
voice-catalog-failed = Impossible de télécharger la liste des voix : { $error }
# $licence describes the voice's licence, in a sentence of its own.
voice-installed = { $voice } est installée. { $licence } La commande Voix la liste.
voice-download-failed = Impossible de télécharger { $voice } : { $error } Choisissez de nouveau la voix pour réessayer.
voice-only-voice-favourite = Seule une voix peut être favorite.
voice-favourite-added = { $voice } ajoutée aux favorites.
voice-favourite-removed = { $voice } retirée des favorites.
voice-chosen = Voix { $voice }.
voice-chosen-rate = Voix { $voice }, { $wpm } mots par minute.
voice-fastest-rate = Débit le plus rapide.
voice-slowest-rate = Débit le plus lent.
voice-rate = { $wpm } mots par minute.
voice-highest-pitch = Hauteur la plus haute.
voice-lowest-pitch = Hauteur la plus basse.
voice-pitch-normal = Hauteur normale.
# $n is a number of semitones.
voice-pitch-plus = Hauteur plus { $n }.
voice-pitch-minus = Hauteur moins { $n }.
voice-full-volume = Volume maximal.
voice-volume-off = Volume coupé.
voice-volume = Volume { $pct } pour cent.
voice-no-speed-presets = Aucun préréglage de vitesse.
# $name is the preset's name from the settings, such as "Study".
voice-speed-preset = { $name }, débit { $wpm }.
voice-line-numbers-on = Numéros de ligne activés.
voice-line-numbers-off = Numéros de ligne désactivés.

## Export et aperçu depuis le lecteur. F5 est la touche de rechargement du
## navigateur, pas celle de textweaver.

common-no-document = Aucun document n'est ouvert.
# Said after "Could not export:", so it starts in lower case. $path is a
# folder or a file; $error the system's reason.
publish-cannot-write-to = impossible d'écrire dans { $path } : { $error }
publish-cannot-write = impossible d'écrire { $path } : { $error }
publish-start-failed = Impossible de démarrer l'export : { $error }
publish-export-error = Impossible d'exporter : { $error }
# $format is the format's name, such as PDF, HTML, or Word.
publish-exporting = Export vers { $format }.
publish-theme-title = Thème de la page HTML
publish-theme-intro = Thème de la page HTML ? { $first } en premier, { $n } choix. Échap annule.
publish-writing-preview = Écriture de l'aperçu.
publish-preview-error = Impossible d'écrire l'aperçu : { $error }
publish-still-exporting =
    { $secs ->
        [one] Export vers { $format } toujours en cours, { $secs } seconde.
       *[other] Export vers { $format } toujours en cours, { $secs } secondes.
    }
publish-still-previewing =
    { $secs ->
        [one] Écriture de l'aperçu toujours en cours, { $secs } seconde.
       *[other] Écriture de l'aperçu toujours en cours, { $secs } secondes.
    }
# $again is yes when a preview is open already.
publish-auto-reload-on =
    { $again ->
        [yes] Rechargement automatique de l'aperçu activé : après chaque enregistrement, le navigateur recharge la page tout seul. Relancez l'aperçu dans le navigateur pour l'utiliser.
       *[no] Rechargement automatique de l'aperçu activé : après chaque enregistrement, le navigateur recharge la page tout seul.
    }
publish-auto-reload-off = Rechargement automatique de l'aperçu désactivé : appuyez sur F5 dans le navigateur après un enregistrement.
publish-live-on = Aperçu en direct activé : l'aperçu se recharge aussi quand la frappe fait une pause.
# "toggle preview auto reload" is the command's name in the command palette.
publish-live-on-needs-reload = Aperçu en direct activé. Il fonctionne avec le rechargement automatique, qui est désactivé ; activez-le avec Recharger l'aperçu automatiquement.
publish-live-off = Aperçu en direct désactivé : l'aperçu ne recharge qu'après un enregistrement.
# $error is the converter's reason.
publish-export-failed = Échec de l'export vers { $format } : { $error }
publish-preview-failed = Échec de l'aperçu : { $error }
# The converter's warnings: how many, and the first one.
publish-warnings =
    { $n ->
        [one] 1 avertissement : { $first }
       *[other] { $n } avertissements ; le premier : { $first }
    }
# $file is the file's name, $folder its folder; $warned is empty or a
# space and publish-warnings.
publish-exported = Exporté vers { $format } : { $file }. L'ouvrir ? y ou n. Dans { $folder }.{ $warned }
publish-report = Rapport enregistré sous { $file }.
publish-report-issues =
    { $n ->
        [one] 1 élément non accessible
       *[other] { $n } éléments non accessibles
    } ; voir le rapport.
publish-report-failed = Le rapport n'a pas pu être enregistré : { $error } Vérifiez que le dossier de sortie est accessible en écriture.
publish-preview-written-served = Aperçu écrit. Ouverture dans le navigateur. Il se recharge tout seul après chaque enregistrement.{ $warned }
publish-preview-written = Aperçu écrit. Ouverture dans le navigateur. Un enregistrement le réécrit ; appuyez ensuite sur F5 dans le navigateur.{ $warned }
publish-preview-updated = Aperçu mis à jour.
publish-preview-updated-press-f5 = Aperçu mis à jour. Appuyez sur F5 dans le navigateur.
publish-server-failed = Impossible de démarrer le serveur de rechargement de l'aperçu ({ $error }) ; ouverture du fichier à la place.
publish-render-failed = Impossible de rendre le texte : { $error }
publish-nothing-after-caret = Rien à lire après le curseur.
publish-listening = Écoute du texte rendu.

## The preview's reload server: shown in the browser.

preview-being-written = L'aperçu est en cours d'écriture. Rechargez dans un instant.

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
        [one] Notes, 1 élément. Entrée va à une note, Suppr la supprime, F2 la modifie.
       *[other] Notes, { $n } éléments. Entrée va à une note, Suppr la supprime, F2 la modifie.
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
notes-highlighted-at = Surligné à { $pct } pour cent : { $text }
notes-highlighted = Surligné : { $text }
# An item in the highlights list. $color is the highlight's color name;
# $lost is yes when the text was not found after the file changed.
notes-highlight-item =
    { $lost ->
        [yes] { $text }, ligne { $line }, { $color }, introuvable après la modification du fichier
       *[no] { $text }, ligne { $line }, { $color }
    }
notes-no-highlights = Aucun surlignage. Pour en faire un : { $key }.
notes-highlights-title = Surlignages
notes-highlights-intro =
    { $n ->
        [one] Surlignages, 1 élément. Entrée va à l'un d'eux, Suppr le supprime.
       *[other] Surlignages, { $n } éléments. Entrée va à l'un d'eux, Suppr le supprime.
    }
# The label said before a highlight's text on jumping to it.
notes-highlight-label = Surligner
# Shown while reading reaches a note's passage.
notes-signal = Note : { $text }
# Said after moving onto a note's passage.
notes-has-note = A une note : { $text }

## Bookmarks: rename and delete.

notes-choose-bookmark-delete = Choisissez un signet et appuyez sur Suppr.
notes-choose-bookmark-rename = Choisissez un signet et appuyez sur F2 pour le renommer.
notes-bookmark-deleted = Signet { $name } supprimé.
notes-renaming-bookmark = Renommage du signet { $name }.
notes-bookmark-unchanged = Signet inchangé.
# $name is the name asked for, $old the bookmark's name.
notes-bookmark-name-taken = Il existe déjà un signet appelé { $name }. Signet { $old } inchangé.
notes-bookmark-renamed = Signet { $old } renommé en { $name }.

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
notes-study-sheet-failed = Impossible d'écrire la fiche d'étude : { $error }
# The study sheet file's own text (Markdown; the # marks stay in the code).
notes-sheet-title = Fiche d'étude : { $title }
notes-sheet-exported = Exporté depuis { -brand } le { $date }.
notes-sheet-before-first-heading = Avant le premier titre
# After a note's text: its tags, joined with commas.
notes-sheet-tags = (étiquettes : { $tags })
# $color is the highlight's color name.
notes-sheet-highlighted = Surligné, { $color }.

## Find, bookmarks, and selection.

marks-cannot-search = Impossible de rechercher : { $error }
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

## Lists and prompts shared by every frontend.

# The focused list item: $item is its text, $k its place, $n the number of items.
listmodel-item-position = { $k } sur { $n }, { $item }
# $letter is the letter or digit typed.
listmodel-no-item-starts = Aucun élément ne commence par { $letter }.
listmodel-top-of-list = Haut de la liste.
listmodel-end-of-list = Fin de la liste.
listmodel-no-matching-commands = Aucune commande correspondante.
listmodel-no-earlier-entries = Aucune entrée antérieure.
# Tab in the command palette: $n commands match (always more than one); $names lists the first few, joined by commas.
listmodel-command-matches = { $n } correspondances : { $names }.

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
        [one] Bibliothèque, { $n } document. Tapez pour filtrer, Entrée en ouvre un, F2 modifie les détails.
       *[other] Bibliothèque, { $n } documents. Tapez pour filtrer, Entrée en ouvre un, F2 modifie les détails.
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
# The label of a heading reached by a link, at high verbosity.
links-heading-label = Titre
# $anchor is the heading name the link gives.
links-no-heading = Aucun titre appelé { $anchor } dans ce document.
# $file is the file the link names.
links-file-not-found = Le lien va vers { $file }, qui est introuvable.
# $file is the file's name; $key names the History Back command's keys.
links-followed = Lien suivi vers { $file }. Précédent : { $key }.
links-back-in = Retour dans { $file }.
# $label is the footnote's label, such as 1.
links-back-to-footnote-reference = Retour à l'appel de note { $label }, ligne { $line }.
# $text is the start of the note.
links-footnote = Note de bas de page { $label } : { $text }
links-footnote-unreferenced = Aucun appel vers la note de bas de page { $label } dans le texte.
links-footnote-no-note = La note de bas de page { $label } n'a pas de texte.

## Citations: inserting, looking up, importing, checking, and the bibliography.

citations-on = Citations activées.
citations-off = Citations désactivées.
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
citations-insert-failed = Impossible d'insérer la citation : { $error }
# $what is the identifier being looked up, as the citation library describes it.
citations-looking-up = Recherche de { $what }.
citations-lookup-not-started = Impossible de démarrer la recherche : { $error }
# $input is the DOI or ISBN as typed.
citations-lookup-failed = Impossible de rechercher { $input } : { $error }
citations-no-library-to-add-to = Il n'y a aucune bibliothèque où ajouter : { -brand } ne conserve aucun fichier dans cette session.
citations-library-save-failed = Impossible d'enregistrer la bibliothèque : { $error }
# $n is how many citations the document has.
citations-found-no-library =
    { $n ->
        [one] { $n } citation trouvée. { -brand } ne conserve aucune bibliothèque dans cette session.
       *[other] { $n } citations trouvées. { -brand } ne conserve aucune bibliothèque dans cette session.
    }
citations-check-failed = Impossible de vérifier les citations : { $error }
citations-no-library-to-import-into = Il n'y a aucune bibliothèque où importer : { -brand } ne conserve aucun fichier dans cette session.
# $file is the file's path.
citations-import-failed = Impossible d'importer { $file } : { $error }
# $style is the style's name from the front matter, such as apa.
citations-style-unusable = Impossible d'utiliser le style de citation { $style } : { $error }
citations-format-failed = Impossible de mettre en forme les citations : { $error }
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
citations-bibliography-insert-failed = Impossible d'insérer la bibliographie : { $error }

## Speech Cursor mode.

speechcursor-off = Curseur de lecture désactivé.
# $line is the line number.
speechcursor-on = Curseur de lecture activé, ligne { $line }. Haut et Bas lisent les lignes, Entrée continue la lecture, Tab ou Échap quitte.
# $text is the line read, as the status line shows it.
speechcursor-on-with-text = Curseur de lecture activé, ligne { $line } : { $text }. Haut et Bas lisent les lignes, Entrée continue la lecture, Tab ou Échap quitte.

## Scrolling the view without moving the cursor.

view-bottom-of-document = Bas du document.
# $line is the line now at the top of the view.
view-line-at-top = Ligne { $line } en haut.

## Background work, and opening files and addresses.

# $what names the export, as its own message says it.
tasks-stopped = { $what } arrêté de façon inattendue.
# $input is the DOI, ISBN, or other identifier being looked up.
tasks-lookup-stopped = La recherche de { $input } s'est arrêtée de façon inattendue.
# The reason in tasks-could-not-open when a session keeps no files.
tasks-launch-off = l'ouverture d'autres programmes est désactivée dans une session qui ne conserve aucun fichier
tasks-opening = Ouverture.
# $target is a file or a web address; $error says why.
tasks-could-not-open = Impossible d'ouvrir { $target } : { $error }
open-refused-scheme = Non ouvert : lien { $scheme } bloqué.
open-refused-missing = Non ouvert : fichier introuvable.
open-refused-invalid = Non ouvert : adresse non valide.
tasks-not-opened = Non ouvert.
# Keep the letters y and n: they are the keys that answer.
tasks-open-it-question = L'ouvrir ? y ou n

## Math exploration.

mathx-no-math = Aucune formule mathématique ici. Placez-vous sur une formule, puis réessayez.
# $math is the whole expression as spoken; $parts is yes when it has parts to go into. The keys are exploration's own.
mathx-exploring =
    { $parts ->
        [yes] Exploration de la formule : { $math }. Bas entre dedans, les flèches déplacent, Échap quitte.
       *[no] Exploration de la formule : { $math }. Échap quitte.
    }
mathx-left = Formule quittée.
mathx-last-term = Dernier terme.
mathx-first-term = Premier terme.
mathx-no-parts = Aucune partie à l'intérieur.
mathx-whole-expression = Expression entière.
mathx-nothing-here = Rien ici.
# $speech is what was said for the step; $code is the math braille code's name (Nemeth or UEB); $braille is the part's braille in Unicode braille cells, for the Braille display.
mathx-step-braille = { $speech } { $code } : { $braille }

## Reading aids: RSVP, bionic reading, syllables, difficult words, the ruler, and the reading level.

aids-rsvp-off = RSVP désactivé.
aids-rsvp-leave-edit = Quittez le mode édition pour utiliser le RSVP.
# $status is RSVP's status line (word and sentence counts, rate, state).
aids-rsvp-on = RSVP activé. { $status }
aids-rsvp-no-words = Aucun mot à afficher.
aids-rsvp-fastest = Débit RSVP le plus rapide.
aids-rsvp-slowest = Débit RSVP le plus lent.
# $wpm is the new rate in words per minute.
aids-rsvp-rate = RSVP { $wpm } mots par minute.
# Where the RSVP word is shown; $position is one of nine fixed keys.
aids-rsvp-position =
    { $position ->
        [top-left] RSVP en haut à gauche.
        [top-center] RSVP en haut au centre.
        [top-right] RSVP en haut à droite.
        [center-left] RSVP au milieu à gauche.
        [center] RSVP au centre.
        [center-right] RSVP au milieu à droite.
        [bottom-left] RSVP en bas à gauche.
        [bottom-right] RSVP en bas à droite.
       *[bottom-center] RSVP en bas au centre.
    }
aids-rsvp-playing = RSVP en cours.
aids-rsvp-paused = RSVP en pause.
aids-rsvp-end-of-text = Fin du texte.
aids-rsvp-start-of-text = Début du texte.
aids-bionic-on = Lecture bionique activée.
aids-bionic-off = Lecture bionique désactivée.
aids-syllables-shown = Syllabes affichées.
aids-syllables-hidden = Syllabes masquées.
aids-difficult-on = Mots difficiles soulignés.
aids-difficult-no-list = Mots difficiles activés, mais la liste de mots est absente de cette version.
aids-difficult-off = Mots difficiles non marqués.
# Added after a word at high verbosity, following a comma.
aids-difficult-word = mot difficile
aids-ruler-off = Règle de lecture désactivée.
aids-ruler-current-line = Ligne actuelle marquée.
aids-ruler-on = Règle de lecture activée.
# $summary is aids-level-summary; $scope says what was measured.
aids-reading-level =
    { $scope ->
        [selection] Sélection : { $summary }
       *[document] Document : { $summary }
    }
aids-reading-level-too-short = Pas assez de texte pour mesurer le niveau de lecture.
# $grade is the Flesch-Kincaid grade with one decimal, $band an aids-band-* message, $ease the reading ease (0 to 100), $words aids-level-words, $sentences aids-level-sentences.
aids-level-summary = Niveau { $grade }, { $band }. Facilité de lecture { $ease } sur 100. { $words } en { $sentences }.
# $n is the count, $count the same number written with thousands separators.
aids-level-words =
    { $n ->
        [one] { $count } mot
       *[other] { $count } mots
    }
aids-level-sentences =
    { $n ->
        [one] { $count } phrase
       *[other] { $count } phrases
    }
aids-band-elementary = primaire
aids-band-middle-school = collège
aids-band-high-school = lycée
aids-band-college = premier cycle universitaire
aids-band-graduate = deuxième cycle universitaire

## Thèmes.

message-error = Erreur : { $message }
# $name is the theme name in the settings; $used the display name of the theme used instead.
themes-unknown = Il n'y a aucun thème appelé { $name } ; { $used } est utilisé à la place.
# $theme is the new theme's display name.
themes-next = Thème { $theme }.
themes-soft-dark-name = { $theme }, sombre doux
themes-choice-aa = { $theme }, conforme AA
themes-choice-below-aa = { $theme }, sous AA
themes-below-aa =
    { $count ->
        [one] Sous AA : 1 vérification n’atteint pas le seuil.
       *[other] Sous AA : { $count } vérifications n’atteignent pas le seuil.
    }

## Accessibility modes and the first-run question.

# Said when the accessibility mode changes; $mode is the new mode's id.
access-mode-changed =
    { $mode ->
        [self-voicing] Mode autonome. textweaver dit tout.
        [screen-reader] Mode lecteur d'écran. textweaver est silencieux ; votre lecteur d'écran lit la ligne d'état.
       *[hybrid] Mode hybride. textweaver lit les documents à voix haute ; votre lecteur d'écran dit les messages et la frappe.
    }
# Yes to the first-run question; $key names the keys that change the mode.
access-hybrid-chosen = Mode hybride. textweaver lit les documents à voix haute ; votre lecteur d'écran dit les messages et la frappe. { $key } change le mode.
# No to the first-run question; $key names the keys that change the mode.
access-hybrid-declined = Maintien du mode autonome. { $key } change le mode.
# $reader is the screen reader found (NVDA, JAWS), or access-a-screen-reader.
access-hybrid-question = { $reader } est en cours d'exécution. Utiliser le mode hybride, où textweaver lit les documents à voix haute et votre lecteur d'écran dit les messages et la frappe ? y ou n
access-a-screen-reader = Un lecteur d'écran
# On the first run with a screen reader and no mode chosen: hybrid mode is
# used, not asked. $reader is the screen reader found (NVDA, JAWS), or
# access-a-screen-reader; $key names the keys that change the mode.
access-hybrid-inferred = { $reader } est en cours d'exécution : textweaver lit les documents à voix haute et laisse les messages à votre lecteur d'écran. { $key } change cela.
# The window's two modes, when the mode changes; $key changes it again.
access-window-choice-reads-aloud = textweaver lit à voix haute
access-window-choice-screen-reader = mon lecteur d'écran lit
access-window-mode-help = Qui lit dans la fenêtre : la voix de textweaver, ou votre lecteur d'écran seul.
access-window-mode-changed =
    { $mode ->
        [screen-reader] Mon lecteur d'écran lit : textweaver se tait et envoie le texte à votre lecteur d'écran. { $key } change cela.
        [speaks-messages] textweaver lit à voix haute et dit ses messages. { $key } change cela.
       *[reads-aloud] textweaver lit à voix haute ; les messages vont à votre lecteur d'écran. { $key } change cela.
    }

## Characters and selections, as spoken.

# The name of a white-space character read on its own; $name is a fixed key.
text-char-name =
    { $name ->
        [space] espace
        [new-line] nouvelle ligne
        [tab] tabulation
        [no-break-space] espace insécable
       *[white-space] espace blanc
    }
# Said after a selection grows ($change is selected) or shrinks (unselected); $text is the text or a character's name.
text-selection-change =
    { $change ->
        [selected] { $text } sélectionné
       *[unselected] { $text } désélectionné
    }

## The settings screen. $label is a setting-* label, $value its value as
## described below.

settings-not-set = non défini
settings-none = aucun
settings-empty = vide
# A number and its unit (a settings-unit-* message): "300 words per minute".
settings-number-unit = { $n } { $unit }
settings-entries =
    { $n ->
        [0] aucune
        [one] 1 entrée
       *[other] { $n } entrées
    }
settings-type-on-or-off = Tapez on ou off.
settings-type-a-number = Tapez un nombre de { $min } à { $max }.
settings-outside = { $n } est en dehors de { $min } à { $max }.
# $names are the choices, joined with commas.
settings-choose-one-of = Choisissez parmi : { $names }.
settings-edit-table = Modifiez { $label } dans settings.toml ; il contient des noms et des valeurs.
# $path is a key such as speech.rate, not translated.
settings-no-such-setting = Il n'y a aucun paramètre { $path }.
settings-cannot-be = { $label } ne peut pas être cela : { $error }
settings-changed = { $label }, { $value }.
settings-clamped = Hors limites ; la valeur la plus proche est utilisée.
settings-restart-speech = Redémarrez la synthèse vocale pour l'utiliser.
settings-next-start = Utilisé à partir du prochain démarrage.
settings-intro = Paramètres, { $n } paramètres. Tapez pour filtrer. Gauche et Droite changent une valeur, Entrée modifie ou en tape une, Suppr remet la valeur par défaut, Échap ferme.
settings-item = { $label } : { $value }
settings-title = Paramètres
settings-title-matching = Paramètres correspondant à { $filter }
settings-closed = Paramètres fermés.
settings-filter-cleared =
    { $n ->
        [one] Filtre effacé, 1 paramètre.
       *[other] Filtre effacé, { $n } paramètres.
    }
settings-filter-none = Aucun paramètre ne correspond à { $query }. Retour arrière retire des lettres.
settings-filter-match =
    { $n ->
        [one] 1 paramètre correspond.
       *[other] { $n } paramètres correspondent.
    }
settings-largest = Valeur la plus grande, { $value }.
settings-smallest = Valeur la plus petite, { $value }.
settings-press-enter = { $label } : appuyez sur Entrée pour taper une nouvelle valeur.
settings-table-item = { $label } : { $value }. Modifiez-le dans settings.toml.
# $help is the setting's help (setting-*-help), which may be empty.
settings-editing = { $label }, maintenant { $value }. { $help }

## Settings: labels, help, and choices, as the settings screen shows and
## says them. Ids follow the key in settings.toml (speech.rate is
## setting-speech-rate).

setting-speech-backend = Moteur vocal
setting-speech-backend-help = Le moteur vocal : automatique choisit le meilleur disponible. Un changement redémarre la synthèse vocale.
choice-speech-backend-auto = automatique
choice-speech-backend-eci = Eloquence
choice-speech-backend-sapi = voix SAPI 5
choice-speech-backend-espeak = eSpeak NG
choice-speech-backend-speechd = Speech Dispatcher
choice-speech-backend-nsspeech = Apple NSSpeech
choice-speech-backend-avspeech = Apple AVSpeech
choice-speech-backend-dectalk = DECtalk
choice-speech-backend-omnivox = Omnivox
choice-speech-backend-null = silencieux
setting-speech-rate = Débit
setting-speech-rate-help = À quelle vitesse textweaver parle.
setting-speech-volume = Volume
setting-speech-volume-help = À quel volume textweaver parle.
setting-speech-pitch = Hauteur
setting-speech-pitch-help = Plus haute ou plus basse que la hauteur propre de la voix.
setting-speech-voice = Voix
setting-speech-voice-help = L'identifiant de la voix ; non défini en choisit une automatiquement. La commande Voix les liste.
setting-speech-prefer-voice = Voix préférée
setting-speech-prefer-voice-help = Quand aucune voix n'est définie, la première voix dont le nom contient ceci, par exemple eloquence.
setting-speech-favorite-voices = Voix favorites
setting-speech-favorite-voices-help = Voix listées en premier par la commande Voix, par identifiant, séparées par des virgules.
setting-speech-punctuation = Ponctuation
setting-speech-punctuation-help = Combien de ponctuation est dite.
choice-speech-punctuation-none = aucune
choice-speech-punctuation-some = quelques signes
choice-speech-punctuation-all = tous
setting-speech-split-caps = Séparer les majuscules
setting-speech-split-caps-help = Dire les mots joints par des majuscules, tel que TextWeaver, comme des mots séparés.
setting-speech-caps = Majuscules
setting-speech-caps-help = Comment une majuscule est signalée quand les caractères sont dits et tapés.
choice-speech-caps-none = non signalée
choice-speech-caps-tone = une tonalité
choice-speech-caps-pitch = une hauteur plus élevée
choice-speech-caps-say-cap = dire majuscule
setting-speech-auto-play = Lire à l'ouverture
setting-speech-auto-play-help = Commencer la lecture à l'ouverture d'un document.
setting-speech-skip-code = Ignorer les blocs de code
setting-speech-skip-code-help = Ne pas dire les blocs de code.
setting-speech-speed-presets = Préréglages de vitesse
setting-speech-speed-presets-help = Débits nommés que F8 fait défiler.
setting-speech-voices-by-language = Voix par langue
setting-speech-voices-by-language-help = La voix pour chaque langue de l'interface, par code de langue, par exemple es = l'identifiant de la voix. Une langue non listée utilise la première voix du moteur pour elle.
setting-speech-latency-offset-ms = Délai du surlignage
setting-speech-latency-offset-ms-help = Combien de temps après qu'un moteur signale un mot le surlignage se déplace, pour les moteurs synchronisés sur leur horloge audio.
setting-speech-pause-heading-ms = Pause après les titres
setting-speech-pause-heading-ms-help = Silence après un titre, plus court à débit rapide. 0 le désactive.
setting-speech-pause-paragraph-ms = Pause après les paragraphes
setting-speech-pause-paragraph-ms-help = Silence après un paragraphe, plus court à débit rapide. 0 le désactive.
setting-speech-pause-list-item-ms = Pause après les éléments de liste
setting-speech-pause-list-item-ms-help = Silence après un élément de liste, plus court à débit rapide. 0 le désactive.
setting-speech-markup-pauses = Pauses écrites en balisage
setting-speech-markup-pauses-help = Lit le balisage de pause d’un document, comme <break time="1s"/>, comme une pause. Désactivez-le pour les documents qui citent ce balisage.
setting-speech-output-device = Périphérique de sortie
setting-speech-output-device-help = Le périphérique audio sur lequel la voix est jouée, par son identifiant. Non défini utilise celui du système par défaut, de même qu'un périphérique non connecté.
setting-speech-verbosity = Verbosité
setting-speech-verbosity-help = Ce que textweaver dit de ce qu'il fait.
choice-speech-verbosity-low = faible
choice-speech-verbosity-normal = normale
choice-speech-verbosity-high = élevée
setting-speech-eci-dictionaries = Dictionnaires Eloquence
setting-speech-eci-dictionaries-help = Les dictionnaires de prononciation communautaires pour Eloquence : activés, désactivés, ou un dossier de votre choix.
choice-speech-eci-dictionaries-true = activés
choice-speech-eci-dictionaries-false = désactivés
setting-speech-eci-library = Bibliothèque Eloquence
setting-speech-eci-library-help = La bibliothèque ECI à charger ; non défini cherche aux emplacements habituels.
setting-speech-eci-code-factory = Rechercher l'Eloquence de Code Factory
setting-speech-eci-code-factory-help = Chercher aussi l'Eloquence pour Windows de Code Factory. Sa licence peut ne pas couvrir d'autres programmes.
setting-speech-sapi-onecore = Voix OneCore
setting-speech-sapi-onecore-help = Lister aussi les voix Windows OneCore via SAPI 5.
setting-speech-apple-backend = Moteur vocal Apple
setting-speech-apple-backend-help = Lequel des moteurs vocaux d'Apple utiliser sur macOS.
choice-speech-apple-backend-auto = automatique
choice-speech-apple-backend-nsspeech = NSSpeechSynthesizer
choice-speech-apple-backend-avspeech = AVSpeechSynthesizer
setting-highlight-enabled = Surligner le texte lu
setting-highlight-enabled-help = Surligner le mot ou la phrase en cours de lecture.
setting-highlight-granularity = Surligner
setting-highlight-granularity-help = Ce que couvre le surlignage de lecture.
choice-highlight-granularity-word = le mot
choice-highlight-granularity-sentence = la phrase
choice-highlight-granularity-both = le mot et la phrase
setting-highlight-lead-words = Avance du surlignage
setting-highlight-lead-words-help = Dessiner le surlignage ce nombre de mots avant le mot entendu (1 est le mot entendu).
setting-highlight-speed = Vitesse du surlignage
setting-highlight-speed-help = Vitesse du surlignage minuté pour les moteurs qui ne signalent aucun mot.
setting-highlight-color = Couleur du surlignage de mot
setting-highlight-color-help = La couleur derrière le mot lu. Choisissez un nom ou tapez un code hexadécimal. Par défaut : la couleur du thème.
setting-highlight-sentence-color = Couleur du surlignage de phrase
setting-highlight-sentence-color-help = La couleur derrière la phrase lue. Choisissez un nom ou tapez un code hexadécimal. Par défaut : la couleur du thème.
setting-normalization-math = Dire les mathématiques
setting-normalization-math-help = Dire la notation mathématique en mots.
setting-normalization-math-verbosity = Verbosité des mathématiques
setting-normalization-math-verbosity-help = À quel point les mathématiques dites sont explicites : faible dit a sur b, normale et élevée en disent plus.
choice-normalization-math-verbosity-low = faible
choice-normalization-math-verbosity-normal = normale
choice-normalization-math-verbosity-high = élevée
setting-normalization-asciimath-delimiter = Délimiteur ASCIIMath
setting-normalization-asciimath-delimiter-help = Le caractère autour d'ASCIIMath, généralement un accent grave ; non défini ne lit aucun ASCIIMath.
setting-normalization-abbreviations = Développer les abréviations
setting-normalization-abbreviations-help = Dire les abréviations en entier, par exemple Docteur pour Dr.
setting-normalization-abbrev-expansions = Vos abréviations
setting-normalization-abbrev-expansions-help = Vos propres abréviations et ce qu'elles signifient.
setting-normalization-numbers = Nombres en mots
setting-normalization-numbers-help = Dire les nombres, dates, heures, et sommes d'argent en mots.
setting-normalization-use-pronunciations = Utiliser les prononciations
setting-normalization-use-pronunciations-help = Appliquer votre liste de prononciations.
setting-normalization-pronunciations = Prononciations
setting-normalization-pronunciations-help = Les mots et comment les dire.
setting-normalization-table-mode = Tableaux
setting-normalization-table-mode-help = Comment les tableaux sont lus.
choice-normalization-table-mode-structured = avec lignes et colonnes
choice-normalization-table-mode-flat = comme du texte
choice-normalization-table-mode-skip = ignorés
setting-normalization-footnote-mode = Notes de bas de page
setting-normalization-footnote-mode-help = Où les notes de bas de page sont lues.
choice-normalization-footnote-mode-inline = là où elles sont marquées
choice-normalization-footnote-mode-deferred = à la fin
choice-normalization-footnote-mode-skip = ignorées
setting-normalization-community-lexicon-enabled = Lexique communautaire
setting-normalization-community-lexicon-enabled-help = Appliquer les dictionnaires de prononciation communautaires pour les moteurs autres qu'Eloquence.
setting-normalization-community-lexicon-dir = Dossier du lexique communautaire
setting-normalization-community-lexicon-dir-help = Le dossier contenant les fichiers de dictionnaire ; non défini cherche à côté de textweaver.
setting-normalization-community-lexicon-language = Langue du lexique communautaire
setting-normalization-community-lexicon-language-help = La langue des dictionnaires.
choice-normalization-community-lexicon-language-enu = anglais américain
choice-normalization-community-lexicon-language-deu = allemand
setting-normalization-medical-lexicon-enabled = Lexique médical
setting-normalization-medical-lexicon-enabled-help = Lire les noms de médicaments, les termes cliniques et les abréviations de posologie avec une liste de prononciation médicale.
setting-normalization-medical-lexicon-overlay = Fichier du lexique médical
setting-normalization-medical-lexicon-overlay-help = Vos propres prononciations médicales, prioritaires sur celles intégrées. Non défini lit medical-lexicon.toml dans le dossier des réglages.
setting-reading-auto-resume = Reprendre où vous en étiez
setting-reading-auto-resume-help = Retourner à la position enregistrée à l'ouverture d'un document.
setting-reading-nav-history-size = Historique de retour
setting-reading-nav-history-size-help = Combien d'endroits Précédent se souvient.
setting-reading-wrap-navigation = Navigation bouclée
setting-reading-wrap-navigation-help = Se déplacer au-delà de la fin du document continue depuis le début.
setting-reading-cursor-follows-speech = Le curseur suit la synthèse vocale
setting-reading-cursor-follows-speech-help = Le curseur se déplace avec le mot lu.
setting-reading-citations = Citations
setting-reading-citations-help = Citations en lecture continue : ignorées, ou dites en mots.
choice-reading-citations-off = ignorées
choice-reading-citations-words = en mots
setting-reading-ocr = Reconnaître les pages numérisées
setting-reading-ocr-help = Lire le texte des PDF et images numérisés en le reconnaissant (OCR).
setting-reading-ocr-lang = Langue du texte numérisé
setting-reading-ocr-lang-help = La langue du texte numérisé, en codes Tesseract tels que fra ou deu+eng. Vide signifie la langue du document, sinon l'anglais.
choice-reading-ocr-lang- = celle du document
choice-reading-ocr-lang-eng = anglais
choice-reading-ocr-lang-fra = français
choice-reading-ocr-lang-deu = allemand
choice-reading-ocr-lang-spa = espagnol
setting-reading-ocr-engine = Moteur OCR
setting-reading-ocr-engine-help = Quel moteur reconnaît les pages numérisées. ocrs pour l'anglais et Tesseract pour les autres langues, ou l'un d'eux toujours.
choice-reading-ocr-engine-auto = automatique
choice-reading-ocr-engine-ocrs = ocrs
choice-reading-ocr-engine-tesseract = Tesseract
choice-reading-ocr-engine-paddle = PaddleOCR (expérimental)
setting-reading-math-engine = Lecture des mathématiques
setting-reading-math-engine-help = Quel moteur lit les mathématiques à voix haute. Celui de textweaver, ou MathCAT en ClearSpeak ou SimpleSpeak, dans la langue du document. MathCAT nécessite une version qui l'inclut ; sinon celui de textweaver est utilisé.
choice-reading-math-engine-builtin = textweaver
choice-reading-math-engine-mathcat = MathCAT ClearSpeak
choice-reading-math-engine-mathcat-simplespeak = MathCAT SimpleSpeak
setting-braille-math-code = Braille mathématique
setting-braille-math-code-help = Le code braille des mathématiques dans les fichiers BRF et pendant l'exploration d'une formule avec MathCAT. Nemeth, ou les mathématiques UEB. Il faut une version qui inclut MathCAT ; sinon, les mathématiques sont écrites avec leurs mots parlés.
choice-braille-math-code-nemeth = Nemeth
choice-braille-math-code-ueb = UEB
setting-braille-table-format = Tableaux en braille
setting-braille-table-format-help = Comment les fichiers BRF disposent les tableaux. Linéaire, une ligne par rangée avec des points-virgules entre les entrées ; en liste, chaque rangée comme un titre et chaque entrée sur sa propre ligne après le titre de sa colonne ; ou en escalier, chaque entrée deux cellules à droite de la précédente, pour les tableaux de quatre colonnes au plus.
choice-braille-table-format-linear = linéaire
choice-braille-table-format-listed = en liste
choice-braille-table-format-stairstep = en escalier
setting-reading-math-display = Mathématiques à l'écran
setting-reading-math-display-help = À quoi ressemblent les mathématiques dans la vue de lecture. Comme leur source, tel que x^2, ou en Unicode, tel que x avec un 2 en exposant. La synthèse vocale et le mode édition utilisent toujours la source.
choice-reading-math-display-source = source
choice-reading-math-display-unicode = Unicode
setting-reading-revisions = Suivi des modifications
setting-reading-revisions-help = Comment les modifications suivies des fichiers Word, OpenDocument et RTF sont lues. Annoncées sur place en verbosité élevée (automatique), toujours, ou jamais, en lisant le texte final. S'applique à l'ouverture d'un document.
choice-reading-revisions-auto = automatique
choice-reading-revisions-marked = toujours les annoncer
choice-reading-revisions-final = texte final seulement
setting-display-theme = Thème
setting-display-theme-help = Le thème de couleur.
setting-display-follow-os-theme = Suivre le thème du système
setting-display-follow-os-theme-help = Au démarrage, utiliser un thème clair, sombre, ou à fort contraste comme le système, sauf si vous en avez choisi un.
setting-display-wrap-width = Largeur du retour à la ligne
setting-display-wrap-width-help = Retourner à la ligne à ce nombre de colonnes ; 0 utilise toute la largeur.
setting-display-measure = Longueur de ligne
setting-display-measure-help = Combien de caractères une ligne contient dans la fenêtre, de 25 à 90. 0 remplit la fenêtre. Le terminal utilise la largeur du retour à la ligne.
setting-display-tab-width = Largeur de tabulation
setting-display-tab-width-help = Colonnes qu'occupe une tabulation.
setting-display-show-line-numbers = Numéros de ligne
setting-display-show-line-numbers-help = Afficher les numéros de ligne.
setting-display-scroll-margin = Marge de défilement
setting-display-scroll-margin-help = Lignes conservées visibles au-dessus et en dessous du curseur.
setting-display-hints = Ligne des raccourcis
setting-display-hints-help = Si le lecteur en mode terminal affiche des raccourcis clavier sur sa dernière ligne. Automatique les affiche en mode autonome et les masque avec un lecteur d'écran. F1 et la liste des raccourcis clavier nomment toujours les touches.
choice-display-hints-auto = automatique
choice-display-hints-on = activé
choice-display-hints-off = désactivé
setting-editing-autosave-recovery = Instantanés de récupération
setting-editing-autosave-recovery-help = Conserver une copie du travail non enregistré et la proposer après un plantage.
setting-editing-autosave-interval-secs = Intervalle des instantanés
setting-editing-autosave-interval-secs-help = Secondes entre les instantanés de récupération pendant qu'il y a des modifications non enregistrées.
setting-editing-echo-characters = Écho des caractères
setting-editing-echo-characters-help = Dire chaque caractère tapé.
setting-editing-echo-words = Écho des mots
setting-editing-echo-words-help = Dire chaque mot tapé.
setting-editing-echo-deletions = Écho des suppressions
setting-editing-echo-deletions-help = Dire ce que Retour arrière et Suppr retirent.
setting-editing-echo-lines-on-move = Écho des lignes
setting-editing-echo-lines-on-move-help = Dire la ligne quand le curseur se déplace vers une autre ligne.
setting-editing-undo-steps = Étapes d'annulation
setting-editing-undo-steps-help = Nombre maximal d'étapes d'annulation conservées pendant l'édition.
setting-editing-undo-memory-mb = Mémoire d'annulation
setting-editing-undo-memory-mb-help = Mémoire maximale que peut utiliser l'historique d'annulation.
setting-library-recent-limit = Fichiers récents
setting-library-recent-limit-help = Combien de fichiers récents sont mémorisés.
setting-library-folders = Dossiers de bibliothèque
setting-library-folders-help = Dossiers dont la bibliothèque liste les documents, et dont les positions se synchronisent entre ordinateurs. Séparez les dossiers par des points-virgules.
setting-keyboard-character-keys = Raccourcis à une seule touche
setting-keyboard-character-keys-help = Touches de parcours telles que h et point. Désactivé, la dictée et la frappe ne déclenchent jamais de commandes.
setting-keyboard-preset = Touches
setting-keyboard-preset-help = Les touches par défaut : comme le mode navigation de NVDA et JAWS, ou les anciennes touches de textweaver. Utilisé à partir du prochain démarrage.
choice-keyboard-preset-default = style lecteur d'écran
choice-keyboard-preset-classic = classique
setting-keyboard-digit-row = Rangée de chiffres
setting-keyboard-digit-row-help = Comment le terminal reconnaît les touches de chiffres pour les niveaux de titre. Automatique, ou un clavier AZERTY français.
choice-keyboard-digit-row-auto = automatique
choice-keyboard-digit-row-azerty = AZERTY
setting-accessibility-mode = Mode d'accessibilité
setting-accessibility-mode-help = Autonome dit tout ; lecteur d'écran laisse la parole à votre lecteur d'écran ; hybride ne vocalise que la lecture.
choice-accessibility-mode-self-voicing = autonome
choice-accessibility-mode-screen-reader = lecteur d'écran
choice-accessibility-mode-hybrid = hybride
setting-accessibility-say-all = Tout dire avec un lecteur d'écran
setting-accessibility-say-all-help = Lecture continue en mode lecteur d'écran. Une phrase à la fois sur la ligne d'état, ou avec la voix de textweaver.
choice-accessibility-say-all-screen = sur la ligne d'état
choice-accessibility-say-all-voice = avec la voix de textweaver
setting-accessibility-quiet-screen = Écran immobile pendant la lecture
setting-accessibility-quiet-screen-help = Garder l'écran immobile pendant que textweaver lit à voix haute. Activé par défaut en mode hybride.
choice-accessibility-quiet-screen-auto = automatique
choice-accessibility-quiet-screen-true = activé
choice-accessibility-quiet-screen-false = désactivé
setting-accessibility-cursor = Curseur
setting-accessibility-cursor-help = Où le curseur du terminal attend. Sur ce sur quoi vous travaillez, ou sur la ligne d'état.
choice-accessibility-cursor-follow = suit le focus
choice-accessibility-cursor-status = sur la ligne d'état
setting-export-subtitle-format = Format des sous-titres
setting-export-subtitle-format-help = Le format des sous-titres écrits sans nom de fichier.
choice-export-subtitle-format-srt = SubRip
choice-export-subtitle-format-vtt = WebVTT
setting-export-subtitle-word-level = Sous-titres par mot
setting-export-subtitle-word-level-help = Un sous-titre par mot au lieu de lignes de légende.
setting-export-subtitles-with-audio = Sous-titres avec l'audio
setting-export-subtitles-with-audio-help = Toujours écrire les sous-titres à côté de l'audio exporté.
choice-export-subtitle-format-ass = Karaoké ASS
setting-export-subtitle-karaoke = Karaoké des sous-titres
setting-export-subtitle-karaoke-help = Comment les lignes de sous-titres montrent le mot lu. Désactivé, souligné une fois prononcé (balises WebVTT) ou un sous-titre par mot, en gras et souligné.
choice-export-subtitle-karaoke-off = Désactivé
choice-export-subtitle-karaoke-tags = Souligner une fois prononcé
choice-export-subtitle-karaoke-lines = Un sous-titre par mot
setting-export-subtitle-chapters = Fichier de chapitres
setting-export-subtitle-chapters-help = Écrire aussi un fichier de chapitres WebVTT à côté des sous-titres ou de l'audio.
# Names for chapters the document leaves untitled, in audio export.
export-chapter-untitled = Livre audio
export-chapter-numbered = Chapitre { $number }
# The read-along page (tw export-audio essay.md --out essay.html).
readalong-skip = Aller au texte
readalong-controls = Audio
readalong-play = Lire
readalong-pause = Pause
readalong-back = Phrase précédente
readalong-forward = Phrase suivante
readalong-follow = Suivre la lecture
readalong-speed = Vitesse
readalong-contents = Sommaire
readalong-play-section = Lire la section : { $title }
setting-reading-aids-rsvp-wpm = Débit RSVP
setting-reading-aids-rsvp-wpm-help = Mots par minute de la présentation visuelle en série rapide (RSVP).
setting-reading-aids-rsvp-pacing = Rythme du RSVP
setting-reading-aids-rsvp-pacing-help = Ce qui fait avancer le mot du RSVP : sa propre minuterie, ou la synthèse vocale.
choice-reading-aids-rsvp-pacing-timer = sa propre minuterie
choice-reading-aids-rsvp-pacing-external = la synthèse vocale
setting-reading-aids-rsvp-clause-pause = Pause de proposition du RSVP
setting-reading-aids-rsvp-clause-pause-help = Temps supplémentaire après une virgule, deux-points, tiret, ou parenthèse, en pourcentage du temps d'un mot.
setting-reading-aids-rsvp-sentence-pause = Pause de phrase du RSVP
setting-reading-aids-rsvp-sentence-pause-help = Temps supplémentaire à la fin d'une phrase, en pourcentage.
setting-reading-aids-rsvp-paragraph-pause = Pause de paragraphe du RSVP
setting-reading-aids-rsvp-paragraph-pause-help = Temps supplémentaire à la fin d'un paragraphe, en pourcentage.
setting-reading-aids-rsvp-long-word-len = Mot long du RSVP
setting-reading-aids-rsvp-long-word-len-help = Les mots plus longs que ce nombre de lettres reçoivent du temps supplémentaire.
setting-reading-aids-rsvp-long-word-step = Palier de mot long du RSVP
setting-reading-aids-rsvp-long-word-step-help = Temps supplémentaire par lettre au-delà de la longueur d'un mot long, en pourcentage.
setting-reading-aids-rsvp-long-word-max = Maximum de mot long du RSVP
setting-reading-aids-rsvp-long-word-max-help = Temps supplémentaire maximal qu'un mot long reçoit, en pourcentage.
setting-reading-aids-rsvp-show-previous = Mot précédent du RSVP
setting-reading-aids-rsvp-show-previous-help = Afficher aussi le mot précédent.
setting-reading-aids-rsvp-show-next = Mot suivant du RSVP
setting-reading-aids-rsvp-show-next-help = Afficher aussi le mot suivant.
setting-reading-aids-rsvp-position = Position du RSVP
setting-reading-aids-rsvp-position-help = Où le mot du RSVP apparaît.
choice-reading-aids-rsvp-position-top-left = en haut à gauche
choice-reading-aids-rsvp-position-top-center = en haut au centre
choice-reading-aids-rsvp-position-top-right = en haut à droite
choice-reading-aids-rsvp-position-center-left = au milieu à gauche
choice-reading-aids-rsvp-position-center = au milieu
choice-reading-aids-rsvp-position-center-right = au milieu à droite
choice-reading-aids-rsvp-position-bottom-left = en bas à gauche
choice-reading-aids-rsvp-position-bottom-center = en bas au centre
choice-reading-aids-rsvp-position-bottom-right = en bas à droite
setting-reading-aids-rsvp-font-size-pt = Taille du RSVP
setting-reading-aids-rsvp-font-size-pt-help = Taille du mot du RSVP dans l'interface graphique.
setting-reading-aids-rsvp-lead-words = Avance du RSVP
setting-reading-aids-rsvp-lead-words-help = Avec le rythme par synthèse vocale, afficher ce nombre de mots avant le mot dit.
setting-reading-aids-bionic = Lecture bionique
setting-reading-aids-bionic-help = Dessiner le début de chaque mot en gras.
setting-reading-aids-bionic-options-ratio = Part bionique
setting-reading-aids-bionic-options-ratio-help = Quelle part de chaque mot est en gras.
setting-reading-aids-bionic-options-min-word-len = Mot le plus court pour la lecture bionique
setting-reading-aids-bionic-options-min-word-len-help = Les mots plus courts que ceci sont laissés tels quels.
setting-reading-aids-bionic-options-skip-numbers = La lecture bionique ignore les nombres
setting-reading-aids-bionic-options-skip-numbers-help = Laisser les mots avec des chiffres tels quels.
setting-reading-aids-bionic-options-skip-urls = La lecture bionique ignore les adresses
setting-reading-aids-bionic-options-skip-urls-help = Laisser les adresses web et de messagerie telles quelles.
setting-reading-aids-bionic-options-skip-code = La lecture bionique ignore le code
setting-reading-aids-bionic-options-skip-code-help = Laisser le code tel quel.
setting-reading-aids-spacing-line-height = Hauteur de ligne
setting-reading-aids-spacing-line-height-help = Hauteur de ligne en multiples de la taille de police ; la valeur des WCAG est 1,5.
setting-reading-aids-spacing-paragraph-spacing = Espacement des paragraphes
setting-reading-aids-spacing-paragraph-spacing-help = Espace après chaque paragraphe, en multiples de la taille de police.
setting-reading-aids-spacing-letter-spacing = Espacement des lettres
setting-reading-aids-spacing-letter-spacing-help = Espace supplémentaire entre les lettres, en multiples de la taille de police.
setting-reading-aids-spacing-word-spacing = Espacement des mots
setting-reading-aids-spacing-word-spacing-help = Espace supplémentaire entre les mots, en multiples de la taille de police.
setting-reading-aids-font-family = Police
setting-reading-aids-font-family-help = La police de lecture de l'interface graphique ; toute famille installée peut être tapée.
choice-reading-aids-font-family-system-ui = la police du système
choice-reading-aids-font-family-sans = sans empattement
choice-reading-aids-font-family-serif = avec empattement
choice-reading-aids-font-family-monospace = à chasse fixe
choice-reading-aids-font-family-atkinson = Atkinson Hyperlegible
choice-reading-aids-font-family-opendyslexic = OpenDyslexic
choice-reading-aids-font-family-lexend = Lexend
setting-reading-aids-font-size-pt = Taille de police
setting-reading-aids-font-size-pt-help = La taille de police de l'interface graphique.
setting-reading-aids-font-weight = Graisse de police
setting-reading-aids-font-weight-help = 400 est normal, 700 gras.
setting-reading-aids-ruler-mode = Règle de lecture
setting-reading-aids-ruler-mode-help = Marquer la ligne actuelle, ou une bande de lignes.
choice-reading-aids-ruler-mode-off = désactivée
choice-reading-aids-ruler-mode-current-line = ligne actuelle
choice-reading-aids-ruler-mode-ruler = règle
setting-reading-aids-ruler-scope = La règle couvre
setting-reading-aids-ruler-scope-help = Une ligne d'affichage, ou toute la ligne.
choice-reading-aids-ruler-scope-row = une ligne d'affichage
choice-reading-aids-ruler-scope-line = toute la ligne
setting-reading-aids-ruler-rows-above = Lignes de la règle au-dessus
setting-reading-aids-ruler-rows-above-help = Lignes de la bande au-dessus de la ligne actuelle.
setting-reading-aids-ruler-rows-below = Lignes de la règle en dessous
setting-reading-aids-ruler-rows-below-help = Lignes de la bande en dessous de la ligne actuelle.
setting-reading-aids-ruler-mask-outside = Masque de la règle
setting-reading-aids-ruler-mask-outside-help = Estomper les lignes en dehors de la bande.
setting-reading-aids-syllables = Syllabes
setting-reading-aids-syllables-help = Dessiner les mots séparés en syllabes par un point médian ; la synthèse vocale est inchangée.
setting-reading-aids-difficult-words = Mots difficiles
setting-reading-aids-difficult-words-help = Souligner les mots rares, et les nommer lors des déplacements de mots en verbosité élevée.
setting-reading-aids-syllable-options-separator = Séparateur de syllabes
setting-reading-aids-syllable-options-separator-help = Ce qui est dessiné entre les syllabes.
setting-reading-aids-syllable-options-left-min = Première coupure de syllabe
setting-reading-aids-syllable-options-left-min-help = Nombre minimal de lettres avant la première coupure.
setting-reading-aids-syllable-options-right-min = Dernière coupure de syllabe
setting-reading-aids-syllable-options-right-min-help = Nombre minimal de lettres après la dernière coupure.
setting-reading-aids-syllable-options-min-word-len = Mot le plus court pour les syllabes
setting-reading-aids-syllable-options-min-word-len-help = Les mots plus courts que ceci ne sont jamais séparés.
setting-reading-aids-syllable-options-skip-urls = Les syllabes ignorent les adresses
setting-reading-aids-syllable-options-skip-urls-help = Laisser les adresses web et de messagerie telles quelles.
setting-reading-aids-syllable-options-skip-code = Les syllabes ignorent le code
setting-reading-aids-syllable-options-skip-code-help = Laisser le code tel quel.
setting-preview-auto-reload = Recharger l'aperçu
setting-preview-auto-reload-help = Recharger l'aperçu du navigateur après chaque enregistrement, via un petit serveur sur cet ordinateur seulement.
setting-preview-live = Aperçu en direct
setting-preview-live-help = Avec le rechargement activé, recharger aussi quand la frappe fait une pause.
setting-lexicon-glossary = Glossaire
setting-lexicon-glossary-help = Votre propre glossaire, consulté avant le dictionnaire : lignes terme : définition, ou le JSON de star. Non défini utilise glossary.txt dans le dossier des paramètres.
setting-lexicon-data-file = Fichier de dictionnaire
setting-lexicon-data-file-help = Le dictionnaire de définition de mots, lexicon-en.twlex. Non défini cherche à côté du programme.
setting-stats-enabled = Statistiques de lecture
setting-stats-enabled-help = Compter le temps lu à voix haute, le point le plus avancé, et les sessions pour chaque document.
setting-interface-language = Langue de l'interface
setting-interface-language-help = La langue des mots propres de textweaver, changée immédiatement. La voix la suit quand le moteur en a une pour elle ; sinon la voix ne change pas.
choice-interface-language-en = English
choice-interface-language-es = Español
choice-interface-language-fr = Français
choice-interface-language-de = Deutsch
choice-interface-language-pt = Português
choice-interface-language-ar = العربية
setting-interface-rtl = Affichage de droite à gauche
setting-interface-rtl-help = Si le lecteur en mode terminal réordonne le texte de droite à gauche pour l'affichage. Automatique laisse faire aux terminaux qui le font eux-mêmes. La synthèse vocale et le lecteur d'écran reçoivent toujours le texte dans l'ordre de lecture.
choice-interface-rtl-auto = automatique
choice-interface-rtl-on = activé
choice-interface-rtl-off = désactivé
setting-gui-announce = Annonces
setting-gui-announce-help = Comment les messages de la fenêtre atteignent le lecteur d'écran, à partir du prochain démarrage. Une région dynamique, ou les notifications UI Automation (Windows uniquement).
choice-gui-announce-live = région dynamique
choice-gui-announce-uia = notifications UI Automation
setting-gui-header = Afficher l'en-tête
setting-gui-header-help = Affiche la barre de commandes au-dessus du document. Désactivé, les commandes gardent leurs touches et leurs éléments de menu.
setting-gui-toolbar = Afficher la barre d'outils
setting-gui-toolbar-help = Affiche la barre des boutons de lecture. Désactivée, les commandes gardent leurs touches et leurs éléments de menu.
setting-gui-auto-hide-menu = Masquer la barre de menus
setting-gui-auto-hide-menu-help = Windows : masque la barre de menus de la fenêtre jusqu'à ce qu'Alt ou F10 l'affiche. Elle se masque de nouveau quand le menu se ferme. Sans effet sous Linux, dont les menus sont la liste F10, ni sous macOS.
setting-gui-speak-messages = Dire les messages de textweaver
setting-gui-speak-messages-help = Quand textweaver lit à voix haute, dire aussi ses messages, la frappe et les déplacements du curseur avec sa voix, pour lire à l'oreille sans lecteur d'écran.
setting-gui-sidebar = Panneau à côté du document
setting-gui-sidebar-help = Le panneau que la fenêtre affiche à côté du document. Aucun, le Sommaire (les titres) ou les Notes. Les touches de panneau le changent, et la fenêtre retient le dernier.
choice-gui-sidebar-off = aucun
choice-gui-sidebar-contents = Sommaire
choice-gui-sidebar-notes = Notes

## Units, said after a number.

settings-unit-words-per-minute =
    { $n ->
        [one] mot par minute
       *[other] mots par minute
    }
settings-unit-percent = pour cent
settings-unit-semitones =
    { $n ->
        [one] demi-ton
       *[other] demi-tons
    }
settings-unit-milliseconds =
    { $n ->
        [one] milliseconde
       *[other] millisecondes
    }
settings-unit-words =
    { $n ->
        [one] mot
       *[other] mots
    }
settings-unit-places =
    { $n ->
        [one] endroit
       *[other] endroits
    }
settings-unit-columns =
    { $n ->
        [one] colonne
       *[other] colonnes
    }
settings-unit-characters =
    { $n ->
        [one] caractère
       *[other] caractères
    }
settings-unit-lines =
    { $n ->
        [one] ligne
       *[other] lignes
    }
settings-unit-seconds =
    { $n ->
        [one] seconde
       *[other] secondes
    }
settings-unit-steps =
    { $n ->
        [one] étape
       *[other] étapes
    }
settings-unit-megabytes =
    { $n ->
        [one] mégaoctet
       *[other] mégaoctets
    }
settings-unit-files =
    { $n ->
        [one] fichier
       *[other] fichiers
    }
settings-unit-letters =
    { $n ->
        [one] lettre
       *[other] lettres
    }
settings-unit-points =
    { $n ->
        [one] point
       *[other] points
    }
settings-unit-rows =
    { $n ->
        [one] ligne
       *[other] lignes
    }

## Settings sections.

section-speech = Voix
section-highlight = Surligner
section-normalization = Lecture du texte
section-reading = Lecture
section-display = Affichage
section-editing = Édition
section-library = Bibliothèque
section-keyboard = Clavier
section-accessibility = Accessibilité
section-export = Exporter
section-braille = Braille
section-reading-aids = Aides à la lecture
section-preview = Aperçu
section-lexicon = Définir un mot
section-stats = Statistiques de lecture
section-interface = Interface
section-gui = Fenêtre

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
edit-insert-failed = Impossible d'insérer : { $error }
# $start and $end are character positions, $len the text's length.
edit-range-out-of-text = Impossible de modifier les caractères { $start } à { $end } : le texte en a { $len }.
edit-change-failed = Impossible de modifier le texte : { $error }
edit-delete-failed = Impossible de supprimer : { $error }
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
edit-not-a-table-size = Ce n'est pas une taille de tableau : { $text }. Tapez colonnes et lignes, par exemple 3 by 2.
# $name is the image's file name.
edit-image-inserted = Image { $name } insérée. Sa description est sélectionnée ; tapez pour la remplacer.
edit-image-failed = Impossible d'insérer l'image : { $error }
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
edit-recovery-offer = { -brand } s'est fermé avec des modifications non enregistrées de { $title }, enregistrées { $when }. Les récupérer maintenant ? Haut et Bas choisissent, Entrée confirme.
edit-recovery-title = Récupérer le travail non enregistré de { $title } ?
edit-recovery-yes = Oui, récupérer { $title } et continuer à modifier
edit-recovery-no = Non, abandonner les modifications non enregistrées
edit-recovery-discarded = Modifications non enregistrées de { $title } abandonnées.
edit-recovered = Travail non enregistré de { $title } récupéré. N'oubliez pas d'enregistrer.
edit-recovery-postponed = Récupération reportée. Le travail non enregistré sera proposé de nouveau la prochaine fois.

## Find and replace, one match at a time.

# $title is replace-match-title. Keep the letters r, s and a: they are the
# keys that answer.
replace-match-question = { $title }. Appuyez sur r pour remplacer, s pour ignorer, a pour tout remplacer, Échap pour arrêter.
# The replace list's title when no match is being asked about.
replace-title = Remplacer
# $n is this match's number, $total the number of matches, $line the line
# number, and $context the text of that line.
replace-match-title = Correspondance { $n } sur { $total }, ligne { $line } : { $context }
replace-item-this = Remplacer celle-ci
replace-item-skip = Ignorer celle-ci
replace-item-rest = Remplacer tout le reste
# $state is common-on or common-off.
replace-item-match-case = Respecter la casse : { $state }
replace-item-whole-words = Mots entiers seulement : { $state }
replace-failed = Impossible de remplacer : { $error }
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

## Saving in the background.

writes-still-saving = Enregistrement en cours. Veuillez patienter.
writes-not-written-in-time = Certaines modifications n'ont pas pu être écrites à temps : le disque ne répond pas.
# $error is the system's reason.
writes-save-failed = Impossible d'enregistrer : { $error }. Toujours en cours d'édition.
# $name is the bookmark's name, $pct where it is.
writes-bookmark-not-saved = Le signet { $name } est posé pour l'instant, mais n'a pas pu être enregistré : { $error }
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
grammar-change-failed = Impossible de modifier le texte : { $error }

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
spell-replace-failed = Impossible de remplacer : { $error }
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

## The terminal reader's startup.

# $wanted is the speech backend asked for, $backend the one used instead.
tui-setup-backend-unavailable = Le moteur vocal { $wanted } n'est pas disponible ; { $backend } est utilisé.
tui-setup-speech-failed = La synthèse vocale n'a pas pu démarrer ({ $error }) ; exécution silencieuse.
speech-engine-fallback = { $failed } n'a pas pu démarrer ; { $engine } parle à la place.
speech-engine-fallback-silent = { $failed } n'a pas pu démarrer et aucun autre moteur vocal n'est disponible ; { -brand } reste silencieux.
tui-setup-cannot-save = Impossible d'enregistrer les paramètres ou les positions : { $error }
tui-setup-keymap-ignored = Fichier de touches ignoré : { $error }
# The first-run welcome. Each value names the key for an action: $play
# reads and pauses, $stop stops, $heading moves to the next heading,
# $help opens the help, $quit quits.
tui-setup-welcome = Bienvenue dans { -brand }. { $open } ouvre un document. { $play } lance et met en pause la lecture, et { $stop } l'arrête. { $palette } liste toutes les commandes. { $help } ouvre l'aide.
# Said at startup without a document. $open, $new, and $help name the
# keys for Open, New Document, and Help.
tui-setup-no-document = Aucun document n'est ouvert. Appuyez sur { $open } pour en ouvrir un, { $new } pour un nouveau, ou { $help } pour de l'aide.

## The terminal reader's launch.

# $name is the file asked for on the command line; $error says why.
tui-could-not-open = Impossible d'ouvrir { $name } : { $error }

## Copying in the terminal reader.

tui-clip-system = Copié avec le presse-papiers système, parce que ce terminal ne peut pas prendre de texte copié.
# $error is why: tui-clip-not-available, tui-clip-not-built, or the
# system's own message.
tui-clip-failed = Impossible de copier avec le presse-papiers système : { $error }. Envoyé au terminal à la place.
tui-clip-not-available = le presse-papiers système n'est pas disponible
tui-clip-not-built = cette version n'a pas de presse-papiers système

## The terminal reader's screen.

# The title line's start; $title is the document's title or
# tui-title-no-document.
tui-title = { -brand } : { $title }
tui-title-no-document = aucun document
# The screen without a document. $keys names the keys for the action.
tui-empty-open = Ouvrir un document : { $keys }.
tui-empty-help = Aide : { $keys }.
tui-empty-quit = Quitter : { $keys }.
# The key hints while a yes-or-no question waits. y, n, and a are the
# answer keys the reader takes; $escape names the Escape key.
tui-hints-confirm = y oui  n ou a non  { $escape } non
# Key hint labels, each shown after its key on the bottom line.
tui-hint-play = lecture
tui-hint-sentence = phrase
tui-hint-faster = plus vite
tui-hint-slower = plus lent
tui-hint-close-rsvp = fermer RSVP
tui-hint-quit = quitter
tui-hint-save = enregistrer
tui-hint-finish = terminer
tui-hint-undo = annuler
tui-hint-bold = gras
tui-hint-heading = titre
tui-hint-commands = commandes
tui-hint-next-line = ligne suivante
tui-hint-previous-line = ligne précédente
tui-hint-again = encore
tui-hint-read-on = continuer
tui-hint-leave = quitter
tui-hint-paragraph = paragraphe
tui-hint-find = rechercher
tui-hint-mark = marquer
tui-hint-lines = lignes
tui-hint-keys = touches
tui-hint-choose = choisir
tui-hint-close = fermer
tui-hint-back = retour
# The list overlay's border: $n is the focused item's number, $count
# the number of items.
tui-list-title = { $n } sur { $count }, { $title }

text-summary =
    { $change ->
        [selected] { $count } caractères sélectionnés
        [unselected] { $count } caractères désélectionnés
        [copied] { $count } caractères copiés
        [cut] { $count } caractères coupés
       *[deleted] { $count } caractères supprimés
    }
text-summary-range =
    { $change ->
        [selected] { $count } caractères sélectionnés, de { $first } à { $last }
        [unselected] { $count } caractères désélectionnés, de { $first } à { $last }
        [copied] { $count } caractères copiés, de { $first } à { $last }
        [cut] { $count } caractères coupés, de { $first } à { $last }
       *[deleted] { $count } caractères supprimés, de { $first } à { $last }
    }
voice-character-keys-on = Raccourcis à une seule touche activés.
voice-character-keys-off = Raccourcis à une seule touche désactivés.
goto-word-start = start
goto-word-end = end

language-voices-loading = La liste des voix se charge encore, donc la voix actuelle continue de parler.

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

## The Braille pass: pages in paged documents such as a PDF.
## $page and $n are page numbers, $label a printed page label such as iv,
## $pages the number of pages. Keep the page first: a 40-cell Braille
## display shows the start of the line.

status-page = page { $page } sur { $pages }
status-page-labelled = page { $label }, { $n } sur { $pages }
status-percent = { $pct }%
pages-position = Page { $page } sur { $pages }.
pages-position-labelled = Page { $label }, { $n } sur { $pages }.
pages-none = Ce document n'a pas de pages.
pages-no-such-page = Pas de page { $page }. Les pages vont de 1 à { $pages }.
pages-label = Page { $label }
pages-outline-item = Page { $label } : { $text }
lists-pages-title =
    { $n ->
        [one] Pages, { $n } page
       *[other] Pages, { $n } pages
    }
lists-pages-title-filtered = Pages, { $shown } sur { $n } correspondent à { $filter }
lists-pages-intro =
    { $n ->
        [one] Pages, { $n } page. Tapez pour filtrer, Entrée va à une page, Échap ferme.
       *[other] Pages, { $n } pages. Tapez pour filtrer, Entrée va à une page, Échap ferme.
    }
lists-pages-here = Vous êtes sur { $heading }.
lists-filter-cleared-pages =
    { $n ->
        [one] Filtre effacé, { $n } page.
       *[other] Filtre effacé, { $n } pages.
    }
lists-filter-none-pages = Aucune page ne correspond à { $query }. Retour arrière retire des lettres.
lists-filter-matched-pages =
    { $n ->
        [one] { $n } page correspond.
       *[other] { $n } pages correspondent.
    }
prompt-go-to-pages = Aller à une page, ou ligne 12, un pourcentage, start, ou end
goto-not-a-target-pages = Ce n'est pas une cible valide : { $text }. Tapez un numéro de page, ligne et un numéro, un pourcentage tel que 50%, start, ou end.
goto-word-page = page

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
# Réglages.
setting-speech-dectalk-library = Bibliothèque DECtalk
setting-speech-dectalk-library-help = La bibliothèque DECtalk à charger ; non défini cherche aux emplacements habituels.
setting-speech-piper-voices = Dossier des voix Piper
setting-speech-piper-voices-help = Le dossier des voix Piper ; non défini utilise le dossier piper du dossier de données de textweaver.
setting-speech-piper-voice = Voix Piper
setting-speech-piper-voice-help = La voix Piper de départ, par son id ; non défini prend la première installée.
setting-speech-piper-phonemizer = Phonétiseur Piper
setting-speech-piper-phonemizer-help = Comment Piper change le texte en sons. La bibliothèque espeak-ng si elle est installée, cette bibliothèque, ou celui de textweaver.
choice-speech-piper-phonemizer-auto = automatique
choice-speech-piper-phonemizer-library = bibliothèque espeak-ng
choice-speech-piper-phonemizer-rust = celui de textweaver
setting-speech-voice-params = Débit et hauteur par voix
setting-speech-voice-params-help = Le débit et la hauteur de la dernière utilisation de chaque voix. Choisir à nouveau une voix les rétablit.
setting-editing-author = Auteur
setting-editing-author-help = L'auteur écrit dans les nouveaux documents créés à partir d'un modèle ; vide le laisse en blanc.

## The window (GUI): drawn labels, hints, and questions.
## Keep the letters Y and N: they are the keys that answer.

gui-yes = Oui
gui-no = Non
gui-answer-delete = Supprimer
gui-answer-remove = Retirer
gui-answer-replace = Remplacer
gui-question-hint = Y répond oui, N répond non, Échap répond non.
gui-button-open = Ouvrir…
gui-button-font = Police…
gui-button-edit = Commencer la modification
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
gui-hint-open = Choisir un document à lire.
gui-hint-font = Choisir la police du texte.
gui-hint-edit = Passer de la lecture à l'édition.
gui-hint-settings = Chaque option, avec son aide.
gui-hint-commands = Lancer une commande par son nom.
gui-hint-play = Lire depuis le mot actuel, ou pause.
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
gui-sidebar-no-headings = Aucun titre.
gui-sidebar-no-notes = Aucune note.
gui-sidebar-current = { $item }, actuel
gui-sidebar-hint = Entrée y va. { $leave } y va et revient. Échap revient.
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
gui-palette-hint = Entrée exécute la première correspondance ; Tab passe à la liste.
gui-open-failed = Impossible d'ouvrir { $name } : { $error }
gui-uia-unavailable = Les notifications UI Automation n'existent que sous Windows ; la région active est utilisée.
gui-graphics-failed = La fenêtre n'a pas pu démarrer son affichage graphique. Le lecteur en terminal, textweaver, n'en a pas besoin.
gui-crashed = textweaver s'est arrêté après une erreur interne.
gui-crashed-saved = Votre position a été enregistrée, et les modifications non enregistrées seront proposées à la récupération au prochain démarrage.
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
## Summaries and difficult-word definitions.

action-summarize = Résumer la sélection, le chapitre ou le document : ses phrases les plus centrales dans une liste ; Entrée va à l'une d'elles
# The summary list's title: $n sentences of the whole document.
summary-title =
    { $n ->
        [one] Résumé, { $n } phrase
       *[other] Résumé, { $n } phrases
    }
# The summary of the chapter at the cursor.
summary-title-chapter =
    { $n ->
        [one] Résumé du chapitre, { $n } phrase
       *[other] Résumé du chapitre, { $n } phrases
    }
# The summary of the selection.
summary-title-selection =
    { $n ->
        [one] Résumé de la sélection, { $n } phrase
       *[other] Résumé de la sélection, { $n } phrases
    }
# Said when the summary list opens; $title is one of the titles above.
summary-intro = { $title }. Entrée va à la phrase et la dit.
# The same, when a long text was read in samples.
summary-intro-sampled = { $title }, d'après des extraits de ce long texte. Entrée va à la phrase et la dit.
summary-none = Rien à résumer : aucune phrase de quatre mots ou plus.
# tw summarize, on standard error, when a long text was read in samples: $read of $total characters.
summary-sampled-cli = Un long texte : le résumé vient de { $read } de ses { $total } caractères, lus par extraits.
# After a difficult word at high verbosity, with definitions on: its first definition.
aids-difficult-word-defined = mot difficile : { $definition }
setting-summary-sentences = Phrases du résumé
setting-summary-sentences-help = Combien de phrases donnent Résumer et tw summarize, de 1 à 50.
setting-reading-aids-difficult-definitions = Définitions des mots difficiles
setting-reading-aids-difficult-definitions-help = Avec les mots difficiles marqués, en verbosité élevée, dire aussi la première définition du dictionnaire d'un mot difficile.
section-summary = Résumés
settings-unit-sentences =
    { $n ->
        [one] phrase
       *[other] phrases
    }

# The GUI. Said in textweaver's own voice when the window takes the
# focus; $title is the document's title.
gui-window-focused = { $title }, { -brand }.

## Opening the new formats. Said after "Could not open NAME:", so
## each starts in lower case.
opening-damaged-json = ce n'est pas un fichier JSON lisible, il est peut-être trop volumineux.
opening-damaged-notebook = ce n'est pas un carnet Jupyter lisible, il est peut-être endommagé ou trop volumineux.
opening-damaged-svg = ce n'est pas un dessin SVG lisible, il est peut-être endommagé ou trop volumineux.
opening-damaged-mathml = ce n'est pas une formule MathML lisible, elle est peut-être endommagée ou trop volumineuse.

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
menu-tables = Tableaux
menu-speech-cursor = Curseur vocal

## Command names, in the menus and the command palette.

name-play-pause = Lire ou mettre en pause
name-stop = Arrêter
name-read-from-cursor = Lire depuis le curseur
name-read-document = Lire tout le document
name-read-current-character = Dire le caractère
name-read-current-word = Dire le mot
name-read-current-sentence = Dire la phrase
name-read-current-line = Dire la ligne
name-read-paragraph = Dire le paragraphe
name-read-selection = Lire la sélection
name-say-position = Dire la position
name-say-status = Dire l'état
name-repeat-message = Répéter le dernier message
name-word-count = Nombre de mots
name-link-address = Adresse du lien
name-replay-sentence = Relire la phrase
name-replay-paragraph = Relire le paragraphe
name-rsvp-toggle = RSVP
name-rsvp-play-pause = Démarrer ou suspendre RSVP
name-rsvp-faster = RSVP plus rapide
name-rsvp-slower = RSVP plus lent
name-rsvp-position-next = Déplacer le mot RSVP
name-reading-level = Niveau de lecture
name-document-overview = Aperçu du document
name-reading-pass = Passe de lecture
name-define-word = Définir le mot
name-summarize = Résumer
name-toggle-citations = Lire les citations
name-explore-math = Explorer les maths
name-listen-rendered = Écouter le rendu
name-next-sentence = Phrase suivante
name-previous-sentence = Phrase précédente
name-next-paragraph = Paragraphe suivant
name-previous-paragraph = Paragraphe précédent
name-next-heading = Lire depuis le titre suivant
name-previous-heading = Lire depuis le titre précédent
name-skip-next-heading = Titre suivant
name-skip-previous-heading = Titre précédent
name-outline = Plan
name-next-heading-level-1 = Titre suivant, niveau 1
name-next-heading-level-2 = Titre suivant, niveau 2
name-next-heading-level-3 = Titre suivant, niveau 3
name-next-heading-level-4 = Titre suivant, niveau 4
name-next-heading-level-5 = Titre suivant, niveau 5
name-next-heading-level-6 = Titre suivant, niveau 6
name-previous-heading-level-1 = Titre précédent, niveau 1
name-previous-heading-level-2 = Titre précédent, niveau 2
name-previous-heading-level-3 = Titre précédent, niveau 3
name-previous-heading-level-4 = Titre précédent, niveau 4
name-previous-heading-level-5 = Titre précédent, niveau 5
name-previous-heading-level-6 = Titre précédent, niveau 6
name-next-table = Tableau suivant
name-previous-table = Tableau précédent
name-next-list = Liste suivante
name-previous-list = Liste précédente
name-next-list-item = Élément de liste suivant
name-previous-list-item = Élément de liste précédent
name-next-link = Lien suivant
name-previous-link = Lien précédent
name-next-block-quote = Citation suivante
name-previous-block-quote = Citation précédente
name-next-separator = Séparateur suivant
name-previous-separator = Séparateur précédent
name-next-graphic = Image suivante
name-previous-graphic = Image précédente
name-follow-link = Suivre le lien
name-table-next-row = Ligne suivante du tableau
name-table-previous-row = Ligne précédente du tableau
name-table-next-column = Colonne suivante du tableau
name-table-previous-column = Colonne précédente du tableau
name-next-chapter = Chapitre suivant
name-previous-chapter = Chapitre précédent
name-history-back = Précédent
name-history-forward = Suivant
name-go-to = Aller à
name-document-start = Début du document
name-document-end = Fin du document
name-caret-next-word = Mot suivant
name-caret-previous-word = Mot précédent
name-caret-next-line = Ligne suivante
name-caret-previous-line = Ligne précédente
name-select-next-word = Sélectionner le mot suivant
name-select-previous-word = Sélectionner le mot précédent
name-select-next-line = Sélectionner la ligne suivante
name-select-previous-line = Sélectionner la ligne précédente
name-page-down = Page suivante
name-page-up = Page précédente
name-scroll-down = Défiler vers le bas
name-scroll-up = Défiler vers le haut
name-speech-cursor-toggle = Curseur vocal
name-speech-cursor-next-line = Curseur vocal, ligne suivante
name-speech-cursor-previous-line = Curseur vocal, ligne précédente
name-speech-cursor-reread-line = Curseur vocal, relire la ligne
name-speech-cursor-exit-and-read = Curseur vocal, lire la suite
name-rate-up = Plus rapide
name-rate-down = Plus lent
name-pitch-up = Plus aigu
name-pitch-down = Plus grave
name-volume-up = Plus fort
name-volume-down = Moins fort
name-cycle-speed-preset = Vitesse prédéfinie
name-choose-voice = Voix
name-restart-speech = Redémarrer la parole
name-cycle-verbosity = Niveau de détail
name-cycle-punctuation = Ponctuation
name-find = Rechercher
name-find-next = Rechercher le suivant
name-find-previous = Rechercher le précédent
name-next-misspelling = Faute suivante
name-previous-misspelling = Faute précédente
name-spelling-suggestions = Suggestions d'orthographe
name-next-grammar-problem = Problème de grammaire suivant
name-previous-grammar-problem = Problème de grammaire précédent
name-next-lint-problem = Problème de format suivant
name-previous-lint-problem = Problème de format précédent
name-add-bookmark = Ajouter un signet
name-list-bookmarks = Signets
name-next-bookmark = Signet suivant
name-previous-bookmark = Signet précédent
name-add-note = Ajouter une note
name-list-notes = Notes
name-next-note = Note suivante
name-previous-note = Note précédente
name-delete-note = Supprimer la note ou le surlignage
name-highlight-selection = Surligner
name-export-study-sheet = Exporter la fiche d'étude
name-open = Ouvrir
name-open-path = Ouvrir par chemin
name-open-library = Bibliothèque
name-new-document = Nouveau document
name-save = Enregistrer
name-save-as = Enregistrer sous
name-export-settings = Exporter les paramètres
name-import-settings = Importer les paramètres
name-reading-statistics = Statistiques de lecture
name-new-from-template = Nouveau d'après un modèle
name-export-html = Exporter en HTML
name-export-pdf = Exporter en PDF
name-export-docx = Exporter en Word
name-export-epub = Exporter en EPUB
name-export-brf = Exporter en braille
name-preview-in-browser = Aperçu dans le navigateur
name-toggle-preview-auto-reload = Recharger l'aperçu automatiquement
name-toggle-preview-live = Aperçu en direct
name-browse-files = Parcourir les fichiers
name-batch-convert = Convertir par lots
name-export-audio = Exporter l'audio
name-quit = Quitter
name-toggle-edit-mode = Mode édition
name-undo = Annuler
name-redo = Rétablir
name-bold = Gras
name-italic = Italique
name-underline = Souligné
name-strikethrough = Barré
name-inline-code = Code en ligne
name-code-block = Bloc de code
name-insert-link = Insérer un lien
name-heading = Titre
name-bullet-list = Liste à puces
name-numbered-list = Liste numérotée
name-block-quote = Bloc de citation
name-horizontal-rule = Ligne horizontale
name-insert-table = Insérer un tableau
name-add-table-row = Ajouter une ligne au tableau
name-insert-image = Insérer une image
name-replace = Remplacer
name-copy = Copier
name-cut = Couper
name-next-table-cell = Cellule suivante
name-previous-table-cell = Cellule précédente
name-cycle-typing-echo = Écho de la frappe
name-select-all = Tout sélectionner
name-delete-word-before = Supprimer le mot avant
name-delete-word-after = Supprimer le mot après
name-paste = Coller
name-insert-citation = Insérer une citation
name-add-reference = Ajouter une référence
name-insert-bibliography = Insérer la bibliographie
name-check-citations = Vérifier les citations
name-import-references = Importer des références
name-dictate = Dicter
name-next-theme = Thème suivant
name-toggle-line-numbers = Numéros de ligne
name-toggle-character-keys = Raccourcis à une touche
name-cycle-access-mode = Mode d'accessibilité
name-settings-profiles = Profils
name-bionic-toggle = Lecture bionique
name-ruler-cycle = Règle de lecture
name-syllables-toggle = Syllabes
name-difficult-words-toggle = Mots difficiles
name-text-larger = Texte plus grand
name-text-smaller = Texte plus petit
name-text-size-reset = Taille de texte normale
name-choose-font = Police
name-contents-panel = Panneau Sommaire
name-notes-panel = Panneau Notes
name-toggle-header = En-tête
name-toggle-toolbar = Barre d'outils
name-next-region = Région suivante
name-previous-region = Région précédente
name-color-settings = Couleurs
name-cycle-interface-announcements = Annonces de l'interface
name-menu = Menus
name-command-palette = Palette de commandes
name-settings = Paramètres
name-keyboard-help = Raccourcis clavier
name-what-does-this-key-do = Que fait cette touche
name-about = À propos de textweaver
name-help = Aide

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
setting-accessibility-interface-announcements = Annonces de l'interface
setting-accessibility-interface-announcements-help = Ce que textweaver dit de lui-même : dialogues, progression, astuces et confirmations courantes. Les erreurs et les réponses à vos questions sont toujours dites. Automatique veut dire minimales avec un lecteur d'écran, normales sinon.
choice-accessibility-interface-announcements-auto = automatique
choice-accessibility-interface-announcements-off = désactivées
choice-accessibility-interface-announcements-minimal = minimales
choice-accessibility-interface-announcements-normal = normales
choice-accessibility-interface-announcements-full = complètes
palette-item = { $name }, { $category } : { $help }. { $keys }
palette-item-no-keys = { $name }, { $category } : { $help }.
palette-item-recent = { $name }, récent, { $category } : { $help }. { $keys }
palette-item-recent-no-keys = { $name }, récent, { $category } : { $help }.
palette-list-title = Commandes correspondant à { $query }
palette-list-title-all = Commandes
palette-list-intro =
    { $title }, { $n ->
        [one] 1 commande
       *[other] { $n } commandes
    }. Entrée en exécute une.
action-browse-files = Parcourir les fichiers et les archives : Entrée ouvre un dossier, une archive ou un document ; Retour arrière remonte
action-batch-convert = Convertir un dossier de documents dans un autre format, en arrière-plan
action-export-audio = Exporter le document en audio parlé : MP3, FLAC, Opus, WAV ou un livre audio M4B
action-dictate = Démarrer ou arrêter la dictée : les mots prononcés s'écrivent au curseur en mode édition
action-color-settings = Ouvrir les paramètres des couleurs : le surlignage de lecture, la règle, les marques et chaque partie de l'écran, avec leur contraste
action-cycle-interface-announcements = Changer ce que textweaver annonce de lui-même : désactivées, minimales, normales ou complètes ; les erreurs et les réponses sont toujours dites
action-menu = Ouvrir les menus : Fichier, Édition, Affichage, Lecture, Parole, Outils et Aide
action-what-does-this-key-do = Appuyer sur une touche pour entendre ce qu'elle fait et où elle se trouve dans les menus, sans l'exécuter
action-about = Lister les informations utiles à un signalement : version, compilation, composants, moteurs vocaux et dossiers
setting-colors-ruler = Couleur de la règle de lecture
setting-colors-ruler-help = La bande de la règle de lecture et de la ligne en cours marquée. La règle garde son soulignement ou son gras. Choisissez un nom ou tapez un code hexadécimal. Par défaut : la couleur du thème.
setting-colors-difficult-words = Couleur des mots difficiles
setting-colors-difficult-words-help = Le soulignement des mots difficiles ; ils restent soulignés et sont nommés au niveau de détail élevé. Choisissez un nom ou tapez un code hexadécimal. Par défaut : la couleur du thème.
setting-colors-syllables = Couleur des marques de syllabe
setting-colors-syllables-help = Les points médians entre les syllabes. Choisissez un nom ou tapez un code hexadécimal. Par défaut : la couleur du thème.
setting-colors-misspellings = Couleur des fautes d'orthographe
setting-colors-misspellings-help = Le soulignement des mots mal orthographiés, dans la fenêtre ; ils sont aussi dits. Choisissez un nom ou tapez un code hexadécimal. Par défaut : la couleur du thème.
setting-colors-lint = Couleur des marques de format
setting-colors-lint-help = Le soulignement des problèmes de format Markdown et de grammaire, dans la fenêtre ; ils sont aussi dits. Choisissez un nom ou tapez un code hexadécimal. Par défaut : la couleur du thème.
setting-colors-find-match = Couleur des résultats de recherche
setting-colors-find-match-help = La bande derrière les résultats ; ils restent soulignés. Choisissez un nom ou tapez un code hexadécimal. Par défaut : la couleur du thème.
setting-colors-selection = Couleur de la sélection
setting-colors-selection-help = La bande derrière le texte sélectionné. Choisissez un nom ou tapez un code hexadécimal. Par défaut : la couleur du thème.
setting-colors-focus = Couleur du focus
setting-colors-focus-help = Le contour du focus et l'élément sélectionné d'une liste ; ils restent en gras. Choisissez un nom ou tapez un code hexadécimal. Par défaut : la couleur du thème.
setting-colors-links = Couleur des liens
setting-colors-links-help = La couleur des liens ; ils restent soulignés. Choisissez un nom ou tapez un code hexadécimal. Par défaut : la couleur du thème.
setting-colors-headings = Couleur des titres
setting-colors-headings-help = La couleur des titres ; ils restent en gras. Choisissez un nom ou tapez un code hexadécimal. Par défaut : la couleur du thème.
setting-colors-status-bar = Couleur de la barre d'état
setting-colors-status-bar-help = La bande des barres d'état et de titre. Choisissez un nom ou tapez un code hexadécimal. Par défaut : la couleur du thème.
setting-colors-notes = Couleur des notes
setting-colors-notes-help = La bande derrière un texte annoté ; il reste en italique et souligné. Choisissez un nom ou tapez un code hexadécimal. Par défaut : la couleur du thème.
setting-colors-bookmarks = Couleur des signets
setting-colors-bookmarks-help = La bande derrière un mot avec un signet ; il reste en gras et souligné. Choisissez un nom ou tapez un code hexadécimal. Par défaut : la couleur du thème.
color-name-theme = couleur du thème
color-name-blue = bleu
color-name-orange = orange
color-name-navy = bleu foncé
color-name-skyblue = bleu ciel
color-name-teal = bleu canard
color-name-gold = doré
color-name-yellow = jaune
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
dictation-failed = La dictée a échoué : { $error }
dictation-no-words = Aucun mot reconnu dans cette phrase.
dictation-lost = La dictée s’est arrêtée avant que ses derniers mots soient écrits.
dictation-not-typed = Mots dictés non écrits, le mode édition est désactivé : { $text }
setting-dictation-speak-while-recording = Parler pendant la dictée
setting-dictation-speak-while-recording-help = Dire les mots dictés à mesure qu’ils arrivent. Désactivé, ils s’affichent dans la ligne d’état et sont dits à chaque pause, pour que le micro n’entende pas la voix.
setting-dictation-model-dir = Dossier du modèle de dictée
setting-dictation-model-dir-help = Le modèle Whisper de la dictée. Sans valeur, whisper/rten/base.en dans le dossier des données.
section-dictation = Dictée


## The file browser. Every row and introduction starts with the name,
## then the kind, so the first cells of a 40-cell Braille line hold what
## matters. $name is a file or folder name; $n a number that chooses the
## plural and $count the same number written with its separators.
# A list item with its position after it, in the file browser.
listmodel-item-position-last = { $item }, { $k } sur { $n }
browse-places-title = Emplacements
browse-places-intro =
    { $n ->
        [one] Emplacements, 1 emplacement.
       *[other] Emplacements, { $n } emplacements.
    }
# $purpose says what the folder or file is chosen for; $intro follows.
browse-choosing = { $purpose }. { $intro }
browse-place-document = { $name }, le dossier du document
browse-place-start = { $name }, dossier de départ
browse-place-library = { $name }, dossier de la bibliothèque
browse-place-disk = { $name }, disque
browse-place-removable = { $name }, lecteur amovible
browse-place-network = { $name }, lecteur réseau
browse-place-cd = { $name }, lecteur de CD ou DVD
browse-place-root = { $name }, le dossier racine
browse-choose-here = Choisir ce dossier, { $name }
browse-row-folder = { $name }, dossier
browse-row-folder-items =
    { $name }, dossier, { $n ->
        [one] 1 élément
       *[other] { $count } éléments
    }
# $kind is a kind below ("Markdown"); $size a size below ("12 KB").
browse-row-file = { $name }, { $kind }, { $size }
browse-row-kind = { $name }, { $kind }
browse-row-hidden = { $row }, caché
# $kind is zip, tar, tar.gz, gzip, or 7z.
browse-kind-archive = archive { $kind }
browse-kind-file = fichier
browse-kind-markdown = Markdown
browse-kind-text = texte
browse-kind-html = page web
browse-kind-epub = livre EPUB
browse-kind-docx = document Word
browse-kind-rtf = document RTF
browse-kind-odt = texte OpenDocument
browse-kind-latex = LaTeX
browse-kind-eml = courriel
browse-kind-mhtml = archive web
browse-kind-pdf = PDF
browse-kind-image = image
browse-kind-daisy = livre DAISY
browse-kind-pptx = diapositives PowerPoint
browse-kind-sheet = feuille de calcul
browse-kind-json = JSON
browse-kind-notebook = carnet Jupyter
browse-kind-svg = dessin SVG
browse-kind-mathml = formule MathML
browse-kind-pandoc = document lu avec Pandoc
browse-size-bytes =
    { $n ->
        [one] 1 octet
       *[other] { $count } octets
    }
# $size is a number, with a decimal under 10 ("3.4").
browse-size-kb = { $size } Ko
browse-size-mb = { $size } Mo
browse-size-gb = { $size } Go
browse-intro =
    { $name }, { $n ->
        [one] 1 élément.
       *[other] { $count } éléments.
    }
browse-intro-empty = { $name } n'a rien à afficher.
# $filter is what was typed.
browse-intro-filtered =
    { $name }, { $n ->
        [one] 1 élément correspond à
       *[other] { $count } éléments correspondent à
    } { $filter }.
browse-intro-in-archive = { $intro } Dans { $archive }.
browse-intro-hidden =
    { $intro } { $n ->
        [one] 1 fichier caché.
       *[other] { $count } fichiers cachés.
    }
browse-intro-cut = { $intro } Seuls les { $max } premiers sont affichés.
# $preview, $choose, $sort, and $all are keys; $item the focused row.
browse-keys = { $intro } Entrée ouvre, Retour arrière remonte, la saisie filtre. { $preview } donne un aperçu, { $choose } choisit un dossier, { $sort } trie, { $all } montre tous les fichiers. { $item }
browse-sorted-name = Trié par nom.
browse-sorted-date = Trié par date, le plus récent d'abord.
browse-sorted-size = Trié par taille, le plus grand d'abord.
browse-showing-all = Tous les fichiers sont affichés.
browse-showing-readable = Seuls les fichiers lisibles sont affichés.
browse-closed = Explorateur de fichiers fermé.
browse-read-only = L'explorateur de fichiers ne fait qu'ouvrir et choisir des fichiers ; il ne les modifie jamais.
# $key is the Choose Folder key.
browse-choose-a-folder = Choisissez un dossier : Entrée en ouvre un, { $key } le choisit.
browse-choose-a-file = Choisissez un fichier : Entrée en choisit un.
browse-no-archive-folder = Un dossier dans une archive ne peut pas être choisi ; choisissez un dossier du disque.
browse-nothing-waiting = Aucune commande n'attend de dossier ; Entrée l'ouvre.
browse-cannot-read = { $name } n'est pas un type de fichier que textweaver sait lire.
browse-folder-unreadable = Impossible d'ouvrir { $name } : { $reason }
browse-archive-too-deep = { $name } est dans trop d'archives pour être ouvert.
browse-archive-too-large = { $name } est trop gros pour être listé sans risque.
browse-archive-unreadable = { $name } n'est pas une archive que textweaver sait lire ; elle est peut-être endommagée.
# A document's preview: its title, then its first sentence.
browse-preview-document = { $title }. { $sentence }
browse-preview-no-text = { $title }. Il n'a pas de texte.
browse-preview-failed = Impossible de donner un aperçu de { $name } : { $reason }
# $names are the first few names inside.
browse-preview-archive =
    { $name } : { $n ->
        [one] 1 fichier
       *[other] { $count } fichiers
    }, { $readable } lisibles. { $names }
browse-preview-archive-folder =
    { $name }, dossier de l'archive, { $n ->
        [one] 1 élément.
       *[other] { $count } éléments.
    }
browse-preview-folder = { $path } : { $names }
browse-preview-folder-empty = { $path } : rien à lire ici.
browse-preview-other = { $name }, { $size } ; textweaver ne sait pas lire ce type de fichier.
browse-preview-path = { $path }


## Batch conversion (File, Batch convert). Keep the meaning first.
batch-choose-source = Choisissez le dossier à convertir
batch-choose-output = Choisissez le dossier des fichiers convertis
batch-format-title = Convertir en
batch-format-intro = Convertir { $name } en : choisissez un format, { $n } choix.
batch-where-title = Où vont les fichiers
batch-where-intro = Où mettre les fichiers convertis ?
batch-where-converted = Dans un dossier converted, { $path }
batch-where-beside = À côté de chaque fichier
batch-where-choose = Dans un autre dossier, choisi ensuite
batch-nothing = Aucun document à convertir dans { $path }.
batch-confirm =
    Convertir { $n ->
        [one] 1 fichier
       *[other] { $n } fichiers
    } en { $format } dans { $path } ? y ou n
batch-confirm-beside =
    Convertir { $n ->
        [one] 1 fichier
       *[other] { $n } fichiers
    } en { $format } à côté de chaque fichier ? y ou n
batch-started =
    Conversion de { $n ->
        [one] 1 fichier
       *[other] { $n } fichiers
    } en { $format }. Échap arrête.
batch-progress = { $percent } pour cent convertis, { $done } fichiers sur { $total }.
batch-busy = Conversion déjà en cours, { $done } fichiers sur { $total }. Échap arrête.
batch-stop-question = Arrêter la conversion ? Les fichiers déjà faits sont gardés. y ou n
batch-stopping = Arrêt après les fichiers en cours d'écriture.
batch-still-converting = La conversion continue.
batch-done =
    { $converted ->
        [one] 1 fichier converti
       *[other] { $converted } fichiers convertis
    } en { $format } ; { $skipped } à jour ; { $failed } en échec.
batch-stopped =
    Arrêté. { $converted ->
        [one] 1 fichier converti
       *[other] { $converted } fichiers convertis
    } ; { $left } non convertis ; { $failed } en échec.
batch-report = Rapport enregistré dans { $path }.
batch-report-failed = Le rapport n'a pas pu être enregistré : { $error }
batch-inaccessible =
    { $n ->
        [one] 1 fichier a
       *[other] { $n } fichiers ont
    } des éléments non accessibles ; voir le rapport.
batch-failures-title =
    { $n ->
        [one] 1 fichier en échec
       *[other] { $n } fichiers en échec
    }
batch-failure-item = { $name } : { $reason }
batch-start-failed = Impossible de lancer la conversion : { $error }
batch-thread-stopped = La conversion par lots s'est arrêtée de façon inattendue.


## Audio export (File, Export audio). Keep the meaning first: a
## 40-cell Braille display shows the start of the line. $name is a file
## name (essay.flac); $path a folder or a file's full path; $format a
## format's name (FLAC, MP3); $voice a voice's or engine's name; $wpm is
## words per minute; $length a length of time from the duration-*
## messages; $chapters and $n are numbers; $percent is a multiple of ten;
## $formats lists format names (M4B); $error is passed on as it is.
audio-format-title = Exporter l'audio en
audio-format-intro = Exporter { $name } en audio : choisissez un format, { $n } choix.
audio-no-ffmpeg =
    { $formats } { $n ->
        [one] demande
       *[other] demandent
    } ffmpeg, qui est introuvable.
audio-format-flac = FLAC : sans perte, environ moitié moins lourd que WAV
audio-format-wav = WAV : le plus lourd, se lit partout
audio-format-mp3 = MP3 : léger, se lit partout
audio-format-opus = Opus : le plus léger, conçu pour la voix
audio-format-ogg = Ogg Vorbis : léger et ouvert, se lit dans la plupart des lecteurs
audio-format-m4b = Livre audio M4B, par ffmpeg
audio-format-mp4 = Vidéo avec sous-titres : MP4, demande ffmpeg
audio-format-html = Page de lecture : texte et audio, un seul fichier
audio-where-title = Où va l'audio
audio-where-intro = Où enregistrer l'audio ?
audio-where-beside = À côté du document, { $path }
audio-where-choose = Dans un autre dossier, choisi ensuite
audio-choose-folder = Choisissez le dossier de l'audio
audio-no-engine = Aucun moteur vocal ici ne sait écrire de fichier audio. Installez eSpeak NG, ou choisissez un autre moteur dans le menu Parole.
audio-confirm = Exporter { $name } avec { $voice } à { $wpm } mots par minute, dans { $path } ? y ou n
audio-started = Export de { $name } en { $format }. Échap arrête.
audio-progress = Export audio, { $percent } pour cent.
audio-busy = { $name } est déjà en cours d'export. Échap arrête.
audio-stop-question = Arrêter l'export ? Aucun fichier n'est gardé. y ou n
audio-stopping = Arrêt de l'export.
audio-still-exporting = L'export audio continue.
audio-stopped = Export audio arrêté ; aucun fichier écrit.
audio-done =
    { $name } écrit : { $length }, { $chapters ->
        [one] 1 chapitre
       *[other] { $chapters } chapitres
    }.
audio-subtitles = Sous-titres dans { $name }.
audio-failed = Impossible d'exporter l'audio : { $error }
audio-thread-stopped = L'export audio s'est arrêté de façon inattendue.


## The window's menus and dialogs. Settings files chosen with the
## system's file chooser, the Colors dialog, and the font list. $ratio is
## a contrast ratio such as 4.8; $verdict is good, fair, or low.
gui-settings-files = Fichiers de paramètres
gui-settings-export-title = Exporter les paramètres
gui-settings-import-title = Importer les paramètres
gui-chooser-no-dialog = Le sélecteur de fichiers du système ne s'est pas ouvert. Tapez plutôt le chemin du fichier.
gui-colors-value = { $value }, contraste { $ratio } pour 1, { $verdict }
gui-colors-help = Gauche et Droite choisissent une couleur nommée, le bleu et l'orange d'abord. Entrée saisit un nom ou une valeur #rrggbb. Suppr remet la couleur du thème. Chaque marque garde son soulignement, sa graisse ou son symbole, quelle que soit sa couleur.
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


## PDF links. Said before the first line of the page a link inside
## a PDF goes to, when the page has no heading there (as links-heading-label
## is for a heading). $page is the page's printed number or label (12, iv).
links-page-label = Page { $page }


## Sync wave, S4: sync in the reader (ADR-0049).
sync-status-off = Synchro : désactivée
sync-status-not-set-up = Synchro : non configurée
sync-status-starting = Synchro : démarrage
sync-status-folder-missing = Synchro : dossier absent, enregistré ici
sync-status-read-only = Synchro : format plus récent, lecture seule
sync-status-failed = Synchro : dossier inutilisable
sync-status-cannot-write = Synchro : écriture impossible, enregistré ici
sync-status-clock-ahead = Synchro : l'horloge de { $device } avance
sync-status-damaged =
    { $n ->
        [one] Synchro : 1 fichier endommagé ignoré
       *[other] Synchro : { $n } fichiers endommagés ignorés
    }
sync-status-up-to-date = Synchro : à jour
sync-status-this-computer = Cet ordinateur : { $name }.
sync-status-no-others = Pas encore d'autre ordinateur.
sync-status-others = Autres ordinateurs : { $names }.
sync-status-error = Problème : { $error }
sync-another-computer = un autre ordinateur
sync-untitled = un document
sync-damaged = Synchro : fichier endommagé de { $device } ignoré.
sync-newer-file = Synchro : fichier plus récent de { $device } ignoré.
sync-read-only = Synchro : format plus récent, lecture seule.
sync-clock-ahead = Synchro : l'horloge de { $device } avance de { $hours } heures.
sync-fresh-id = Synchro : configuration copiée ; nouvel identifiant.
sync-write-failed = Synchro : écriture impossible. { $error }
sync-name-refused = Nom refusé. Essayez par exemple portable.
sync-note-replaced =
    { $n ->
        [one] { $title } : une note a été remplacée par la modification plus récente de { $device }.
       *[other] { $title } : { $n } notes ont été remplacées par les modifications plus récentes de { $device }.
    }
sync-restored-notes =
    { $n ->
        [one] { $title } : une note supprimée est revenue, modifiée sur { $device }.
       *[other] { $title } : { $n } notes supprimées sont revenues, modifiées sur { $device }.
    }
sync-restored-bookmarks =
    { $n ->
        [one] { $title } : un signet supprimé est revenu, modifié sur { $device }.
       *[other] { $title } : { $n } signets supprimés sont revenus, modifiés sur { $device }.
    }
sync-restored-highlights =
    { $n ->
        [one] { $title } : un surlignage supprimé est revenu, modifié sur { $device }.
       *[other] { $title } : { $n } surlignages supprimés sont revenus, modifiés sur { $device }.
    }
sync-arrived =
    { $n ->
        [one] { $title } : 1 modification de { $device }.
       *[other] { $title } : { $n } modifications de { $device }.
    }
sync-resumed = { $title } : reprise à { $pct } pour cent, depuis { $device }.
sync-place-arrived = Position de { $device } : { $pct } pour cent.
sync-place-question = { $device } à { $pct } pour cent. Y aller ? y ou n
sync-suggestion-question =
    { $n ->
        [one] C'est peut-être { $title } de { $device }, avec 1 note. L'utiliser ? y ou n
       *[other] C'est peut-être { $title } de { $device }, avec { $n } notes. Les utiliser ? y ou n
    }
sync-went-to-place = Position de { $device }, { $pct } pour cent.
sync-kept-place = Position actuelle conservée.
sync-suggestion-accepted = Notes de { $device } utilisées.
sync-suggestion-declined = Laissés séparés.
sync-sidecar-differed =
    { $n ->
        [one] Synchro : 1 position de bibliothèque différait.
       *[other] Synchro : { $n } positions de bibliothèque différaient.
    }
sync-sidecar-failed = Synchro : position de bibliothèque non écrite. { $error }
sync-no-state = Synchro désactivée pour cette session : rien n'est enregistré.
sync-choose-folder = Choisir le dossier de synchro
sync-group-places = Positions
sync-group-notes = Notes
sync-group-highlights = Surlignages
sync-group-bookmarks = Signets
sync-group-statistics = Statistiques
sync-group-item = { $name } : { $state }
sync-start = Démarrer la synchro
sync-groups-title = Ce qui se synchronise
sync-groups-intro = { $title }, comme { $name }. Entrée active ou désactive ; Démarrer la synchro termine.
sync-started = Synchro activée, comme { $name }. État de la synchro : { $key }.
sync-how-to-set-up = Pour la configurer : Outils, Synchro, Configurer la synchro.
sync-now-started = Synchronisation.
sync-now-done =
    { $n ->
        [one] Synchro : à jour, 1 document vérifié.
       *[other] Synchro : à jour, { $n } documents vérifiés.
    }
sync-now-changed =
    { $n ->
        [one] Synchro : 1 document a reçu des modifications.
       *[other] Synchro : { $n } documents ont reçu des modifications.
    }
sync-no-places = Aucun autre ordinateur n'a de position ici.
sync-place-item = { $device }, { $pct } pour cent
sync-places-title =
    { $n ->
        [one] 1 autre position
       *[other] { $n } autres positions
    }
sync-no-replaced = Aucune note remplacée dans ce document.
sync-replaced-item = { $text }, remplacée par { $device }
sync-replaced-item-deleted = { $text }, supprimée par { $device }
sync-replaced-title =
    { $n ->
        [one] 1 note remplacée
       *[other] { $n } notes remplacées
    }
sync-replaced-intro = { $title }. Entrée en rétablit une.
sync-note-restored = Note rétablie : { $text }
sync-already-off = La synchro est déjà désactivée ici.
sync-stopped = Synchro désactivée ici. Le dossier reste tel quel.
prompt-sync-computer-name = Nom de cet ordinateur, Entrée le garde
menu-sync = Synchro
name-sync-setup = Configurer la synchro
name-sync-status = État de la synchro
name-sync-now = Synchroniser maintenant
name-sync-go-to-place = Position d'un autre ordinateur
name-sync-replaced-notes = Notes remplacées
name-sync-stop = Arrêter la synchro sur cet ordinateur
action-sync-setup = Configurer la synchro : choisir le dossier, nommer cet ordinateur et choisir ce qui se synchronise
action-sync-status = Dire où en est la synchro (à jour, dossier absent ou un problème) et nommer les autres ordinateurs
action-sync-now = Synchroniser maintenant : envoyer les modifications de cet ordinateur et prendre celles des autres pour chaque document
action-sync-go-to-place = Lister les positions des autres ordinateurs dans ce document ; Entrée va à l'une d'elles
action-sync-replaced-notes = Lister les notes remplacées par la modification plus récente d'un autre ordinateur ; Entrée en rétablit une
action-sync-stop = Arrêter la synchro sur cet ordinateur ; le dossier de synchro reste tel quel
section-sync = Synchro
setting-sync-enabled = Synchro
setting-sync-enabled-help = Partager notes, surlignages, signets et positions avec vos autres ordinateurs par le dossier de synchro. Outils, Synchro, Configurer la synchro l'active.
setting-sync-folder = Dossier de synchro
setting-sync-folder-help = Le dossier que vos ordinateurs partagent. Un dossier tenu à jour par Syncthing, un dossier cloud ou une clé USB.
setting-sync-device-name = Nom de l'ordinateur
setting-sync-device-name-help = Le nom de cet ordinateur dans les messages de synchro, comme portable ou labo. Vide donne Computer 1, Computer 2, etc.
setting-sync-places = Synchroniser les positions
setting-sync-places-help = Partager où vous en êtes dans chaque document.
setting-sync-notes = Synchroniser les notes
setting-sync-notes-help = Partager les notes.
setting-sync-highlights = Synchroniser les surlignages
setting-sync-highlights-help = Partager les surlignages.
setting-sync-bookmarks = Synchroniser les signets
setting-sync-bookmarks-help = Partager les signets.
setting-sync-statistics = Synchroniser les statistiques
setting-sync-statistics-help = Partager le temps de lecture et les sessions de chaque ordinateur.
setting-sync-position-policy = Position de reprise
setting-sync-position-policy-help = La position où s'ouvre un document quand un autre ordinateur en a une aussi. La plus récente, la plus avancée, ou demander.
choice-sync-position-policy-newest = la plus récente
choice-sync-position-policy-furthest = la plus avancée
choice-sync-position-policy-ask = demander

## End of S4

## Sync wave, S5 (see en.ftl).
sync-settings-arrived =
    { $n ->
        [one] Réglages : 1 changement de { $device }.
       *[other] Réglages : { $n } changements de { $device }.
    }
sync-settings-arrived-several = Réglages : { $n } changements de { $computers } ordinateurs.
sync-kept-keys-mac =
    { $n ->
        [one] Touches Mac : 1, gardée, inutilisée ici.
       *[other] Touches Mac : { $n }, gardées, inutilisées ici.
    }
sync-kept-keys-pc =
    { $n ->
        [one] Touches Windows et Linux : 1, gardée, inutilisée ici.
       *[other] Touches Windows et Linux : { $n }, gardées, inutilisées ici.
    }
sync-group-settings = Réglages
sync-group-profiles = Profils
sync-group-key-overrides = Touches personnelles
sync-group-words = Liste de mots
sync-group-glossary = Glossaire et prononciations
sync-group-favorite-voices = Voix favorites
voices-missing-row = { $voice }, favorite, absente de cet ordinateur
voices-missing = { $voice } n'est pas sur cet ordinateur. Espace la retire des favorites.
gui-voices-list = Voix
gui-voices-use = Utiliser la voix
gui-voices-use-help = Utiliser la voix sélectionnée et en entendre un échantillon, ou la télécharger après une question.
gui-voices-preview = Écouter
gui-voices-preview-help = Entendre un échantillon de la voix sélectionnée sans la choisir.
gui-voices-favorite = Favorite
gui-voices-favorite-help = Marquer la voix sélectionnée comme favorite, ou l'enlever. Les favorites viennent en premier.
gui-voices-remove = Supprimer
gui-voices-remove-help = Supprimer la voix Piper téléchargée sélectionnée, après une question.
gui-voices-remove-unavailable = indisponible
gui-voices-language-help = Afficher seulement les voix de la langue suivante, puis toutes les langues.
gui-voices-engine-help = Afficher seulement les voix du moteur suivant, puis tous les moteurs.
gui-voices-fetch-help = Télécharger la liste des voix Piper, environ 250 Ko, après une question.
gui-voices-close-help = Fermer le gestionnaire de voix.
gui-voices-hint = Entrée utilise la voix, Espace marque une favorite, Échap ferme.
setting-sync-settings = Synchroniser les réglages
setting-sync-settings-help = Partager les réglages portables : débit, ponctuation, thème, aides à la lecture et autres. La voix, le moteur, le mode d'accès, le jeu de touches et les chemins restent sur chaque ordinateur.
setting-sync-profiles = Synchroniser les profils
setting-sync-profiles-help = Partager les profils ; celui en usage reste propre à chaque ordinateur.
setting-sync-key-overrides = Synchroniser les touches personnelles
setting-sync-key-overrides-help = Partager keymap.toml. Les touches d'un Mac sont gardées mais pas utilisées sous Windows ou Linux, et l'inverse.
setting-sync-words = Synchroniser la liste de mots
setting-sync-words-help = Partager la liste de mots de l'orthographe.
setting-sync-glossary = Synchroniser le glossaire
setting-sync-glossary-help = Partager les entrées du glossaire et les prononciations.
setting-sync-favorite-voices = Synchroniser les voix favorites
setting-sync-favorite-voices-help = Partager les voix favorites. Une voix absente de cet ordinateur est indiquée comme telle.

## End of S5

## Lexend downloaded on first choice.
font-download-question = Télécharger la police { $font }, { $kb } Ko, { $licence } ? y ou n
font-downloading = Téléchargement de { $font }.
font-downloaded = { $font } téléchargée et prête.
font-download-failed = { $font } non téléchargée : { $error }
font-download-declined = Non téléchargée. Autre police utilisée.
font-download-busy = { $font } est encore en téléchargement.
font-download-no-folder = Aucun dossier de données pour { $font }.
font-download-not-in-build = Téléchargement de polices non inclus.
gui-font-to-download = { $family } (à télécharger, { $kb } Ko)
gui-font-downloaded = { $family } (téléchargée)

## Sélecteurs de fichiers et de dossiers.
prompt-browse-hint = { $label }. { $key } pour parcourir.
prompt-browse-file = Choisissez le fichier : { $label }
prompt-browse-folder = Choisissez le dossier : { $label }
prompt-browse-filled = { $name } choisi. Entrée confirme.
chooser-type-files = Fichiers { $type }
chooser-image-title = Insérer une image
chooser-images = Images
chooser-references-title = Importer des références
chooser-reference-files = Fichiers de références
chooser-profiles-import-title = Importer des profils
chooser-profiles-export-title = Exporter les profils
chooser-profile-files = Fichiers de profils
gui-folder-no-dialog = Le sélecteur de dossiers du système ne s'est pas ouvert. Choisissez le dossier dans cette liste.
gui-prompt-browse-hint = { $key } ouvre le navigateur de fichiers.

## Composants facultatifs.
name-manage-components = Gérer les composants facultatifs…
name-download-dictation-model = Télécharger le modèle de dictée
action-manage-components = Gérer les composants facultatifs : les modèles, polices et voix que textweaver peut télécharger, avec leur taille et leur licence
action-download-dictation-model = Télécharger le modèle de dictée choisi dans les réglages, après avoir dit sa taille et sa licence
component-feature-dictation = la dictée
component-feature-ocr = lire les pages numérisées
component-feature-reading-font = une police de lecture
component-feature-voice = une voix
component-state-installed = installé
component-state-not-installed = non installé
component-state-partial = installé en partie
component-state-damaged = endommagé
component-state-downloading = en téléchargement
components-title = Composants facultatifs
components-intro =
    { $n ->
        [one] 1 composant facultatif. Entrée pour les actions.
       *[other] { $n } composants facultatifs. Entrée pour les actions.
    }
components-item = { $title } : { $state }, { $size }, licence { $license }, pour { $features }
components-actions-intro = { $title } : { $state }.
components-action-download = Télécharger, { $size }
components-action-verify = Vérifier les fichiers
components-action-remove = Supprimer
components-action-install-zip = Installer depuis un fichier zip…
components-action-install-folder = Installer depuis un dossier…
components-install-purpose = Installer le composant depuis ici
component-question = Télécharger { $title }, { $size }, licence { $license } ? y ou n
component-remove-question = Supprimer { $title } ? y ou n
component-downloading = Téléchargement. Échap l'arrête.
component-installing = Installation depuis le fichier.
component-verifying = Vérification des fichiers.
component-progress = { $percent } pour cent téléchargés.
component-ready = Prêt : { $title }.
component-verified = Fichiers corrects : { $title }.
component-verify-failed =
    { $n ->
        [one] 1 fichier incorrect : { $files }.
       *[other] { $n } fichiers incorrects : { $files }.
    }
component-removed = Supprimé : { $title }.
component-refused =
    { $n ->
        [one] 1 fichier écarté : { $files }.
       *[other] { $n } fichiers écartés : { $files }.
    }
component-already = Déjà installé : { $title }.
component-not-there = Non installé : { $title }.
component-declined = Rien n'a été téléchargé.
component-not-in-build = Pas de téléchargement ici.
component-no-folder = Aucun dossier de données pour lui.
component-error-fetch = Non téléchargé : la source a échoué.
component-error-size = Non installé : mauvaise taille.
component-error-hash = Non installé : un fichier diffère.
component-error-missing = Non installé : un fichier manque.
component-error-cancelled = Arrêté ; reprendra plus tard.
component-error-busy = Un téléchargement est en cours.
component-error-no-source = Non téléchargé : pas d'adresse.
component-error-name = Refusé : un nom n'est pas simple.
component-error-manifest = Liste du miroir illisible.
component-error-io = Non installé : écriture impossible.
components-chooser-title = Composants facultatifs
components-chooser-intro = Extras facultatifs, aucun choisi. Espace en choisit un ; Télécharger ceux choisis les obtient ; Échap passe.
components-chooser-item = { $mark } : { $title }, pour { $features }, { $size }, licence { $license }
components-chosen = Choisi
components-not-chosen = Non choisi
components-chooser-download = Télécharger ceux choisis
components-chooser-skip = Plus tard
components-chooser-skipped = Passé ; voir Gérer les composants.
components-chooser-none = Rien choisi, rien téléchargé.
dictation-model-question = La dictée a besoin du modèle Whisper, { $size }, licence { $license }. Le télécharger maintenant ? y ou n
dictation-model-declined = Pas de modèle : dictée indisponible.
dictation-model-not-in-build = Pas de modèle ni de téléchargement ici.
dictation-model-file-missing = Il manque au modèle { $file }.
dictation-model-damaged = Modèle de dictée endommagé : { $file }.
dictation-model-no-folder = Dossier absent : { $dir }.

## Réglages des composants facultatifs.
section-components = Composants facultatifs
setting-dictation-model = Modèle de dictée
setting-dictation-model-help = Le modèle Whisper qu'utilise la dictée quand aucun dossier n'est réglé. Télécharger le modèle de dictée, dans le menu Outils, l'obtient.
choice-dictation-model-whisper-base-en = base.en, par défaut
choice-dictation-model-whisper-small-en = small.en, plus grand et plus précis
setting-components-mirror = Miroir des composants
setting-components-mirror-help = D'où viennent d'abord les composants facultatifs : une adresse https ou un dossier sur cet ordinateur. Vide utilise leurs sources publiques. N'y mettez jamais de mot de passe.

## Help's ways to the docs, About's facts, and first-run choices asked
## again. $address is a web address; $path a folder; facts start with
## their name so each Braille line leads with it.
name-quick-start = Démarrage rapide
name-documentation = Documentation…
name-report-problem = Signaler un problème…
name-ask-first-run-again = Reposer les questions de démarrage
action-quick-start = Ouvrir le guide de démarrage rapide dans textweaver
action-documentation = Afficher l'adresse web de la documentation, et demander avant de l'ouvrir dans un navigateur
action-report-problem = Afficher où signaler un problème, et demander avant de l'ouvrir dans un navigateur ; rien n'est envoyé
action-ask-first-run-again = Reposer les questions du premier démarrage : le mode hybride avec un lecteur d'écran, et les composants facultatifs
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

## W9b-x : les paramètres de lecture (Affichage, Paramètres de lecture).
name-reading-form = Paramètres de lecture
action-reading-form = Ouvrir les paramètres de lecture : débit, police, espacements, longueur de ligne, thème, surlignage, règle, lecture bionique et syllabes
reading-form-intro = Paramètres de lecture, { $n } paramètres. Gauche et Droite changent une valeur, Entrée en saisit une, Suppr remet la valeur par défaut, F1 dit l'aide.
reading-form-spacing-wcag-done = Espacements réglés sur les valeurs WCAG.
reading-form-spacing-generous-done = Espacements réglés sur Large, plus que WCAG.
gui-reading-form-help = Haut et Bas déplacent, Gauche et Droite changent une valeur, Entrée en saisit une, F1 dit l'aide.
gui-reading-voices = Voix
gui-reading-voices-help = Ouvrir le gestionnaire de voix.
gui-reading-wcag = Espacement WCAG
gui-reading-wcag-help = Régler les quatre espacements sur les valeurs WCAG.
gui-reading-generous = Espacement large
gui-reading-generous-help = Régler les quatre espacements plus larges que WCAG.
gui-reading-closed = Paramètres de lecture fermés.
## End of W9b-x
