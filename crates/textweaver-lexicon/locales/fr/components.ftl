### Optional components and updates.

## Composants facultatifs.
component-feature-dictation = la dictée
component-feature-ocr = lire les pages numérisées
component-feature-reading-font = une police de lecture
component-feature-voice = une voix
component-feature-audio-export = export M4B et MP4
component-feature-braille = fichiers braille en noir, braille abrégé
component-feature-pandoc = plus de formats de document
component-state-installed = installé
component-state-not-installed = non installé
component-state-partial = installé en partie
component-state-damaged = endommagé
component-state-downloading = en téléchargement
component-state-installer-ready = programme d'installation téléchargé
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
component-error-hash = Non installé : un fichier ne correspondait pas. Téléchargez-le à nouveau ou vérifiez la source.
component-error-missing = Non installé : un fichier manque.
component-error-cancelled = Arrêté ; reprendra plus tard.
component-error-busy = Un téléchargement est en cours.
component-error-no-source = Non téléchargé : pas d'adresse.
component-error-name = Refusé : un nom n'est pas simple.
component-error-manifest = Une liste de composants est illisible.
component-error-io = Non installé : écriture impossible.
component-error-unpack = Non installé : l'archive ne s'est pas décompressée.
component-launch-question = Lancer l'installation : { $title }, version { $version }, licence { $license } ? Le système demande ensuite. y ou n
component-version-unknown = non indiquée
component-launched = Installation lancée : { $title }. Suivez les demandes du système.
component-launch-failed = L'installation n'a pas démarré : { $reason }
component-launch-declined = Non lancé ; le programme d'installation est gardé.
components-token-invalid = Ce n'est pas un jeton. Vérifiez-le et collez-le de nouveau.
components-token-kept = Jeton gardé dans le coffre d'identifiants du système.
components-token-not-kept = Jeton utilisé, non gardé : { $reason }.
components-token-forgotten = Jeton oublié.
components-token-none = Aucun jeton n'était gardé.
components-token-not-forgotten = Non oublié : { $reason }.
components-token-gh-still = La connexion de GitHub CLI reste utilisée ; gh auth logout y met fin.
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
helper-question = { $need } a besoin de { $title }, { $size }, licence { $license }. Le télécharger maintenant ? y ou n
helper-need-audio = L'export M4B et MP4
helper-need-brf = La lecture du braille en noir
helper-run-command = Pour obtenir { $title }, exécutez : { $command }
dictation-model-not-in-build = Pas de modèle ni de téléchargement ici.
dictation-model-file-missing = Il manque au modèle { $file }.
dictation-model-damaged = Modèle de dictée endommagé : { $file }.
dictation-model-no-folder = Dossier absent : { $dir }.

## Réglages des composants facultatifs.

## Mises à jour (B1-u1).
components-chooser-updates = { $mark } : Rechercher les mises à jour automatiquement, une fois par jour, sur GitHub
update-checking = Recherche des mises à jour.
update-none = Pas de mise à jour : { $version } est la plus récente.
update-found = Mise à jour disponible : textweaver { $version }, { $size }. La télécharger ? y ou n
update-available-elsewhere = Mise à jour disponible : textweaver { $version }. Mettez cette copie à jour comme elle a été installée.
update-declined = Non téléchargée. Une version plus récente sera proposée.
update-downloading = Téléchargement de la mise à jour, { $size }.
update-progress = Mise à jour : { $percent } pour cent téléchargés.
update-installed = Mise à jour vérifiée et installée. Redémarrez textweaver pour l'utiliser.
update-on-close = Mise à jour vérifiée. Elle s'installe à la fermeture de textweaver.
update-busy = Recherche des mises à jour déjà en cours.
update-not-in-build = Pas de mise à jour dans cette version.
update-automatic-question = Rechercher les mises à jour automatiquement, une fois par jour ? y ou n
update-automatic-on = Mises à jour : recherchées une fois par jour.
update-automatic-off = Mises à jour : non recherchées. Aide, Rechercher les mises à jour le fait à tout moment.
update-error-check = Recherche de mise à jour échouée : { $reason }
update-error-no-package = Pas de paquet de mise à jour pour cet ordinateur dans { $version }.
update-error-no-checksum = Mise à jour refusée : aucune somme de contrôle. Rien n'a changé.
update-error-mismatch = Mise à jour refusée : somme de contrôle différente. Rien n'a changé.
update-error-cancelled = Téléchargement arrêté ; il reprendra la prochaine fois.
update-error-download = Mise à jour non téléchargée : { $reason }
update-error-install = Mise à jour non installée : { $reason }. Rien n'a changé.
update-error-not-package = Non mis à jour : cette copie ne vient pas d'un paquet publié.
section-updates = Mises à jour et composants
