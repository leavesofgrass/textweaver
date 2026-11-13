### Common words, parts of speech, define word, and names said inside other messages.

### Les messages de l'interface de textweaver, en français.
###
### Fluent (https://projectfluent.org/), dans le sous-ensemble que lit
### textweaver-lexicon. Les identifiants et les variables sont ceux de
### locales/en ; un message absent ici est dit en anglais.

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
define-glossary-problem = Impossible de lire votre glossaire : { $error } Seul le dictionnaire est consulté.
define-glossary-skipped =
    { $n ->
        [one] 1 ligne de votre glossaire n'a pas de définition et a été ignorée.
       *[other] { $n } lignes de votre glossaire n'ont pas de définition et ont été ignorées.
    }
define-copied = Copié.

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
