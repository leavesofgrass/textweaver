### Reading: navigation, reading aloud, status, statistics, the queue, and reading aids.

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

## La file de lecture et la liste de révision (bêta 3).

queue-title = File de lecture
queue-intro =
    { $n ->
        [one] File de lecture, 1 document. Entrée l’ouvre, Suppr le retire.
       *[other] File de lecture, { $n } documents. Entrée en ouvre un, Suppr le retire.
    }
queue-empty = La file de lecture est vide. Dans la bibliothèque, { $key } ajoute un document.
queue-item-missing = { $title }, introuvable
queue-item-elsewhere = { $title }, pas sur cet ordinateur
queue-untitled = Un document d'un autre ordinateur
queue-elsewhere-chosen = { $title } n'est pas sur cet ordinateur. Il reste dans la file de lecture, et sera trouvé dès que vous l'ouvrirez ici.
queue-added =
    { $n ->
        [one] Ajouté à la file de lecture : { $title }. 1 document dans la file.
       *[other] Ajouté à la file de lecture : { $title }. { $n } documents dans la file.
    }
queue-removed = Retiré de la file de lecture : { $title }.
queue-no-file = Seul un document enregistré dans un fichier peut entrer dans la file de lecture.
queue-read-failed = Impossible de lire la bibliothèque pour la file de lecture : { $error }. Vérifiez library.json dans le dossier de données ; tw settings path l’indique.
queue-next-ask = Ensuite : { $title }. { $key } pour lire.
queue-next-reading = Ensuite : { $title }.
review-title = Liste de révision
review-intro =
    { $n ->
        [one] Liste de révision, 1 document, le plus anciennement lu d’abord. Entrée l’ouvre.
       *[other] Liste de révision, { $n } documents, le plus anciennement lu d’abord. Entrée en ouvre un.
    }
review-empty = Rien à revoir pour l’instant. Les documents lus à voix haute apparaissent ici.
review-stats-off = La liste de révision a besoin des statistiques de lecture, qui sont désactivées. Activez Statistiques de lecture dans les réglages.
review-item = { $title }, { $days }, { $pct } pour cent, { $notes }
review-days =
    { $n ->
        [0] aujourd’hui
        [one] 1 jour
       *[other] { $n } jours
    }
review-notes =
    { $n ->
        [0] aucune note
        [one] 1 note
       *[other] { $n } notes
    }
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
playback-end-of-document-content = fin du document
playback-no-unit-here = Pas de { $what } ici.
playback-no-selection = Aucune sélection.
# $next is what happens now (a restart, or nothing).
playback-speech-died = La synthèse vocale s'est arrêtée ({ $reason }). { $next }
playback-done-reading = Lecture terminée.
playback-speech-restarted = Synthèse vocale redémarrée : { $reason }. Reprise de la lecture au dernier mot.
playback-speech-error = Erreur de synthèse vocale : { $error } Réessayez, ou utilisez la commande Redémarrer la parole.
playback-end-of-section = Fin de la section. { $key } pour continuer.
playback-time-up =
    { $minutes ->
        [one] Temps écoulé après 1 minute. { $key } pour continuer.
       *[other] Temps écoulé après { $minutes } minutes. { $key } pour continuer.
    }
playback-repeat-slower = Répétition plus lente, à { $rate } mots par minute.
playback-recall-prompt = Dites ce dont vous vous souvenez de { $section }. { $key } pour continuer.

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
status-cw-rate = CW { $wpm } mpm
status-cw-rate-spoken = CW à { $wpm } mots par minute
status-cw-rate-farnsworth = CW { $wpm } mpm, effective { $effective }
status-cw-rate-farnsworth-spoken = CW à { $wpm } mots par minute, effective { $effective }
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

## Summaries and difficult-word definitions.
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
settings-unit-sentences =
    { $n ->
        [one] phrase
       *[other] phrases
    }

# The GUI. Said in textweaver's own voice when the window takes the
# focus; $title is the document's title.
gui-window-focused = { $title }, { -brand }.
gui-updated-to = Mis à jour vers { $version }. Aide, Documentation décrit les nouveautés.
