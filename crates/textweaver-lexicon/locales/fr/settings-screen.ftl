### The settings screen, profiles, units, sections, and settings files.

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
profiles-read-failed = Impossible de lire le fichier de profils ; il est donc traité comme vide : { $error } Enregistrer un profil remplace le fichier.
profiles-save-failed = Impossible d'enregistrer les profils : { $error } Vérifiez que le dossier des paramètres est accessible en écriture.
profiles-none-to-export = Il n'y a encore aucun profil à exporter.
profiles-exported =
    { $n ->
        [one] 1 profil exporté vers { $file }.
       *[other] { $n } profils exportés vers { $file }.
    }
profiles-export-failed = Impossible d'exporter les profils : { $error } Vérifiez que le dossier est accessible en écriture.
profiles-imported =
    { $n ->
        [0] Il n'y avait aucun profil dans { $file }.
        [one] 1 profil importé depuis { $file } : { $names }.
       *[other] { $n } profils importés depuis { $file } : { $names }.
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
settingsio-export-failed = Impossible d'exporter les paramètres : { $error } Vérifiez que le dossier est accessible en écriture.
# $path is the file; $error the system's reason.
settingsio-read-failed = Impossible de lire { $path } : { $error } Vérifiez le fichier, puis importez de nouveau.
settingsio-nothing-to-import = Rien à importer : vos paramètres correspondent déjà à ce fichier.
settingsio-cancelled-unchanged = Annulé. Rien n'a été modifié.
settingsio-import-failed = Impossible d'importer les paramètres : { $error } Vérifiez que le dossier des paramètres est accessible en écriture.
# $summary lists what changed (from the settings store, in English).
settingsio-imported = Paramètres importés. { $summary }
settingsio-backend-next-start = Le nouveau moteur vocal sera utilisé au prochain démarrage.
settingsio-backed-up = Les anciens paramètres ont été sauvegardés.

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
settings-cannot-be = { $label } ne peut pas être cela : { $error } Choisissez une autre valeur.
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

## Units, said after a number.

settings-unit-words-per-minute =
    { $n ->
        [one] mot par minute
       *[other] mots par minute
    }
settings-unit-hertz =
    { $n ->
        [one] hertz
       *[other] hertz
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
settings-unit-groups =
    { $n ->
        [one] groupe
       *[other] groupes
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
settings-unit-minutes =
    { $n ->
        [one] minute
       *[other] minutes
    }

## Settings sections.

section-highlight = Surligner
section-reading = Lecture
section-display = Affichage
section-editing = Édition
section-library = Bibliothèque et outils d'étude
section-keyboard = Clavier et langue
section-accessibility = Accessibilité et annonces
section-export = Exporter
section-braille = Braille

## Settings by task (beta 4): sections, and when a setting applies.

section-voice = Voix et parole
section-pauses = Pauses et verbosité
section-speaking-text = Lecture du texte
section-documents = Documents
section-math = Mathématiques
section-morse = Code Morse
section-morse-practice = Entraînement au Morse
section-spacing = Espacement, police et règle
section-word-aids = Aides pour les mots
section-rsvp = Mot à mot (RSVP)
section-window = Barres et panneaux
# $n is the number of sections.
settings-intro-sections =
    { $n ->
        [one] Paramètres, 1 section. Entrée ouvre une section. Tapez pour chercher dans tous les paramètres. Échap ferme.
       *[other] Paramètres, { $n } sections. Entrée ouvre une section. Tapez pour chercher dans tous les paramètres. Échap ferme.
    }
# $section is a section-* title, $n its number of settings.
settings-section-row =
    { $n ->
        [one] { $section }, 1 paramètre
       *[other] { $section }, { $n } paramètres
    }
settings-all = Tous les paramètres
settings-all-row =
    { $n ->
        [one] Tous les paramètres, 1 paramètre
       *[other] Tous les paramètres, { $n } paramètres
    }
settings-title-section = Paramètres : { $section }
settings-section-opened =
    { $n ->
        [one] { $section }, 1 paramètre. Gauche et Droite changent une valeur, Entrée en change ou en tape une, Suppr remet la valeur par défaut, Échap revient aux sections.
       *[other] { $section }, { $n } paramètres. Gauche et Droite changent une valeur, Entrée en change ou en tape une, Suppr remet la valeur par défaut, Échap revient aux sections.
    }
settings-all-opened =
    { $n ->
        [one] Tous les paramètres, 1 paramètre, y compris ceux qui ne s'appliquent pas maintenant. Échap revient aux sections.
       *[other] Tous les paramètres, { $n } paramètres, y compris ceux qui ne s'appliquent pas maintenant. Échap revient aux sections.
    }
settings-back = Sections des paramètres.
settings-section-now =
    { $n ->
        [one] { $section } affiche maintenant 1 paramètre.
       *[other] { $section } affiche maintenant { $n } paramètres.
    }
settings-section-help =
    { $n ->
        [one] { $section }, 1 paramètre. Entrée l’ouvre.
       *[other] { $section }, { $n } paramètres. Entrée l’ouvre.
    }
# $item is a settings-item row, $why a settings-applies-* sentence.
settings-item-not-now = { $item }. { $why }
# $engine, $first and $second are speech engine names, such as Eloquence.
settings-applies-engine = S'applique quand { $engine } est le moteur vocal.
settings-applies-engines = S'applique quand { $first } ou { $second } est le moteur vocal.
settings-applies-not-engine = S'applique quand { $engine } n'est pas le moteur vocal.
# $os is Windows, macOS or Linux.
settings-applies-os = S'applique seulement sous { $os }.
