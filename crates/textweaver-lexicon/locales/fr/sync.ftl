### Sync between computers.

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
sync-status-error = Problème : { $error } Vérifiez le dossier de synchro.
sync-another-computer = un autre ordinateur
sync-untitled = un document
sync-damaged = Synchro : fichier endommagé de { $device } ignoré.
sync-newer-file = Synchro : fichier plus récent de { $device } ignoré.
sync-read-only = Synchro : format plus récent, lecture seule.
sync-clock-ahead = Synchro : l'horloge de { $device } avance de { $hours } heures.
sync-fresh-id = Synchro : configuration copiée ; nouvel identifiant.
sync-write-failed = Synchro : écriture impossible. { $error } Enregistrement sur cet ordinateur pour l'instant.
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
        [one] Utiliser ici la note de { $device } ? y ou n. Il peut s'agir de { $title }, avec 1 note.
       *[other] Utiliser ici les notes de { $device } ? y ou n. Il peut s'agir de { $title }, avec { $n } notes.
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
sync-sidecar-failed = Synchro : position de bibliothèque non écrite. { $error } Vérifiez que le dossier de la bibliothèque est accessible en écriture.
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
menu-cards = Cartes d’étude
menu-cw-practice = Entraînement au morse
section-sync = Synchro

## End of S4

## Sync wave, S5 (see locales/en).
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

## End of S5
