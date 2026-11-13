### Voices, the voice manager, and restarting speech.

## Restarting speech.

restart-silent-now = { -brand } est silencieux maintenant ; redémarrez-le pour entendre la synthèse vocale de nouveau.
# $keys names the Restart Speech key or keys.
restart-silent-use-key = { -brand } est silencieux maintenant. Redémarrez la synthèse vocale avec { $keys }.
restart-restarting = Redémarrage de la synthèse vocale.
restart-not-here = La synthèse vocale ne peut pas être redémarrée ici.
restart-already = La synthèse vocale est déjà en cours de redémarrage.
# $error is the system's reason, in its own words.
restart-failed = Impossible de redémarrer la synthèse vocale : { $error } Patientez un instant, puis réessayez.
restart-start-failed = Impossible de redémarrer la synthèse vocale : le démarrage a échoué.
restart-no-engine = Aucun moteur vocal disponible ; { -brand } reste silencieux. Le guide Troubleshooting, section No speech at all, donne les vérifications.
restart-done-silent = Synthèse vocale redémarrée, mais silencieuse : aucun moteur n'est disponible.
restart-done = Synthèse vocale redémarrée.

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
voice-list-failed = Impossible de lister les voix : { $error } Choisissez un autre moteur dans le menu Parole.
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
voice-preview-failed = Impossible d'écouter { $voice } : { $error } Essayez une autre voix.
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
voice-remove-failed = Impossible de supprimer { $voice } : { $error } Vérifiez que son dossier est accessible en écriture.
voice-downloading-catalog = Téléchargement de la liste des voix Piper.
voice-downloading = Téléchargement de { $voice }.
voice-downloading-percent = Téléchargement de { $voice }, { $pct } pour cent.
voice-details-failed = Impossible de lire les détails de la voix : { $error } Vérifiez la connexion, puis réessayez.
voice-download-stopped = Le téléchargement s'est arrêté.
voice-catalog-fetched = La liste des voix Piper contient { $voices } voix en { $languages } langues. La commande Voix les liste.
voice-catalog-failed = Impossible de télécharger la liste des voix : { $error } Vérifiez la connexion, puis réessayez.
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
