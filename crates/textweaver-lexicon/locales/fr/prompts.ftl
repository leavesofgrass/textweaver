### Prompts, questions, lists, and file choosers.

## Invites.

prompt-define-word = Définir quel mot ?
prompt-profile-name = Nom du nouveau profil
prompt-profile-rename = Nouveau nom du profil, Entrée le conserve
prompt-profiles-import = Importer des profils depuis un fichier
prompt-profiles-export = Exporter les profils vers un fichier, par exemple textweaver-profiles.json

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
app-window-only = Cette commande fonctionne dans l'application textweaver.
app-terminal-only = Cette commande fonctionne dans le lecteur en mode terminal.
settings-save-failed = Impossible d'enregistrer les paramètres : { $error } Vos changements restent actifs jusqu'à la fermeture.
settings-outside-kept = Les paramètres modifiés en dehors de textweaver ont été conservés.
edit-still-editing = Toujours en cours d'édition.
# The reading view inside edit mode (toggle_preview). $key is its key;
# $line the line the caret is back on.
edit-preview-on = Aperçu, en lecture seule : la vue de lecture au même endroit. { $key } revient à l'édition.
edit-preview-off = Retour à l'édition. { $line }
edit-preview-read-only = L'aperçu est en lecture seule. { $key } revient à l'édition.
edit-preview-not-editing = L'aperçu fonctionne en mode édition. { $key } commence l'édition.
edit-preview-plain = Le texte brut n'a pas de mise en forme à prévisualiser ; il se lit de la même façon.
edit-preview-failed = Impossible d'afficher l'aperçu : { $error } Vérifiez le Markdown, puis réessayez.
goto-not-a-target = Ce n'est pas une cible valide : { $text }. Tapez un numéro de ligne, un pourcentage tel que 50%, start, ou end.

## Prompts: the label is shown and said when the prompt opens.

prompt-find = Rechercher
prompt-go-to = Aller à une ligne, un pourcentage, start, ou end
prompt-open = Ouvrir un fichier
prompt-command = Commande
# $label is prompt-command.
prompt-command-palette-intro = { $label }. Tapez une partie d'un nom ; Tab complète, Haut et Bas parcourent les correspondances.
prompt-save-as = Enregistrer sous
prompt-export-as = Exporter sous
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
goto-word-start = start
goto-word-end = end

language-voices-loading = La liste des voix se charge encore, donc la voix actuelle continue de parler.

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
chooser-anki-title = Importer des cartes Anki
chooser-anki-files = Paquets Anki
chooser-profiles-import-title = Importer des profils
chooser-profiles-export-title = Exporter les profils
chooser-profile-files = Fichiers de profils
chooser-live-source-files = Fichiers de sources en direct
gui-folder-no-dialog = Le sélecteur de dossiers du système ne s'est pas ouvert. Choisissez le dossier dans cette liste.
gui-prompt-browse-hint = { $key } ouvre le navigateur de fichiers.

## A secret field's Show choice: the show key, or the Show box in
## the textweaver app, shows a password, token or join code as typed.
prompt-show-hint = { $label }. { $key } l’affiche.
prompt-secret-hidden = { $count ->
        [one] 1 caractère, masqué
       *[other] { $count } caractères, masqués
    }
prompt-show-join-code = Afficher le code d’accès
prompt-showing-join-code = Code d’accès affiché
prompt-hiding-join-code = Code d’accès masqué
prompt-show-password = Afficher le mot de passe
prompt-showing-password = Mot de passe affiché
prompt-hiding-password = Mot de passe masqué
prompt-show-token = Afficher le jeton
prompt-showing-token = Jeton affiché
prompt-hiding-token = Jeton masqué
gui-prompt-show-hint = { $key } ou la case Afficher montre la saisie.
