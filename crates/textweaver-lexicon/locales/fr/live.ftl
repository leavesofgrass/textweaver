### Live documents, live sources, and the broker.

## Live documents (beta 5): settings, and live sources' passwords. $source
## is a live source's name; $reason says why the credential store failed.
section-live = Documents en direct
prompt-live-password = Mot de passe de { $source }, Entrée annule
prompt-live-password-any = Mot de passe de la source en direct, Entrée annule
live-source-unknown = Aucune source en direct nommée { $source }. Vérifiez les sources dans settings.toml.
live-password-no-store = Pas de magasin d'identifiants dans cette version : { $source } ne peut pas se connecter avec un mot de passe. Utilisez un paquet de version.
live-password-invalid = Pas un mot de passe : 1024 caractères au plus, sans saut de ligne ni tabulation.
live-password-kept = Mot de passe de { $source } gardé dans le magasin d'identifiants du système.
live-password-not-kept = Mot de passe de { $source } non gardé : { $reason }. Utilisé pour cette session seulement.
live-password-forgotten = Mot de passe de { $source } oublié.
live-password-none = Aucun mot de passe gardé pour { $source }.
live-password-not-forgotten = Mot de passe de { $source } non oublié : { $reason }. Réessayez, ou retirez-le dans le magasin d'identifiants du système.

## Beta 5: live documents (a log fed by a live source, such as an MQTT
## broker). $name is the source's name, $topic an MQTT topic, $path and
## $folder file paths, $line a log entry as one line.
live-log-title = Journal en direct
live-log-empty = Aucune entrée pour l'instant.
live-entry-empty = Entrée vide.
live-field-callsign = Indicatif
live-field-name = Nom
live-field-location = Lieu
live-field-status = Statut
live-field-traffic = Trafic
live-field-notes = Notes
live-field-time = Heure
live-traffic = Trafic
live-yes = oui
live-no = non
live-read-only = Ce journal en direct est en lecture seule ; il ne peut pas être modifié.
live-not-open = Aucun journal en direct n'est ouvert.
live-view-list = Vue en liste.
live-view-table = Vue en tableau.
live-newest-none = Le journal n'a encore aucune entrée.
live-paused = Mises à jour en pause. Les nouvelles entrées attendent que vous repreniez.
live-paused-status = mises à jour en pause
live-stopped = { $name } arrêté. Le journal reste ouvert, en lecture seule.
live-log-saved = Journal enregistré sous { $name } dans { $folder }.
live-log-save-failed = Impossible d'enregistrer le journal en direct { $path } : { $error } Vérifiez que le dossier des journaux est accessible en écriture ; textweaver réessaie dans 10 secondes.
live-log-empty-not-saved = Le journal n'a encore aucune entrée, rien n'a donc été enregistré.
live-log-nowhere = Il n'y a aucun dossier où enregistrer le journal en direct.
live-log-saved-already = Ce journal a été ouvert depuis { $path } ; il est déjà enregistré.
live-binary = Un message sur { $topic } a été omis : ce n'est pas du texte ({ $len } octets).
live-too-large = Un message sur { $topic } a été omis : il est trop volumineux ({ $len } octets).
live-full = Le journal est plein à { $max } entrées ; les entrées plus récentes sont omises.
live-connecting = Connexion à { $name }.
live-reconnecting = Nouvelle connexion à { $name }, tentative { $attempt }.
live-connected = Connecté à { $name }, chiffré.
live-connected-tailscale = Connecté à { $name } à une adresse de la plage Tailscale. S’il s’agit bien de Tailscale, la liaison est chiffrée.
live-connected-plain = Connecté à { $name }. La connexion n'est pas chiffrée.
live-disconnected = Déconnecté de { $name } : { $reason }
live-quiet = Aucun message pour l'instant sur { $topic }. Si cela dure, vérifiez que le broker autorise ce compte à le lire.
live-unread =
    { $count ->
        [0] aucune non lue
        [one] 1 non lue
       *[other] { $count } non lues
    }
live-entries =
    { $count ->
        [one] 1 entrée
       *[other] { $count } entrées
    }
live-resumed =
    { $count ->
        [0] Mises à jour reprises.
        [one] Mises à jour reprises, 1 mise à jour appliquée.
       *[other] Mises à jour reprises, { $count } mises à jour appliquées.
    }
live-resumed-dropped = Mises à jour reprises, { $count } mises à jour appliquées. { $dropped } mises à jour plus anciennes ont été omises pendant la pause.

## Beta 5: live sources' commands, the set-up steps, and new entries said.
## $name is a source's name, $topic an MQTT topic, $line an entry as one
## line, $reason a sentence in words.
menu-live = Documents en direct
menu-live-reading = Lecture
menu-live-broker = Votre broker
menu-live-invite = Inviter d’autres personnes
menu-live-publish = Publier
prompt-live-setup = Configurer une source en direct
prompt-save-live-source-file = Source en direct vers fichier
live-failed = Connexion impossible à { $name } : { $reason } Plus aucun essai ; corrigez cela, puis rouvrez la source.
live-topic-refused = Le broker a refusé le topic { $topic } ; il ne laisse peut-être pas ce compte le lire.
live-skipped =
    { $count ->
        [one] 1 message a été omis : ils arrivaient plus vite qu'ils ne pouvaient être affichés.
       *[other] { $count } messages ont été omis : ils arrivaient plus vite qu'ils ne pouvaient être affichés.
    }
live-new-entries = { $line } { $count } nouvelles entrées.
live-unread-newest = { $count }. La plus récente : { $line }
live-not-in-build = Les sources en direct ne sont pas dans cette version de { -brand }.
live-speak-on = Dire les nouvelles entrées : activé.
live-speak-off = Dire les nouvelles entrées : désactivé.
live-none-set-up = Aucune source en direct configurée. « Configurer une source en direct » en ajoute une.
live-open-title = Sources en direct
live-source-item = { $name }, { $host }, { $topic }
live-forget-title = Oublier le mot de passe de quelle source en direct ?
live-no-passwords = Aucune source en direct ne se connecte avec un nom d'utilisateur ; aucun mot de passe n'est gardé.
live-not-receiving = Aucune source en direct ne reçoit.
live-cannot-open = Impossible d'ouvrir la source en direct. { $problems }
live-not-opened = { $name } non ouverte : pas de mot de passe.
live-setup-host-needed = Il faut un nom d'hôte ou une adresse, sans espace.
live-setup-ca-file-needed = Ce chiffrement demande un fichier de l'autorité de certification.
live-setup-ca-file-missing = Aucun fichier à { $path }.
live-setup-port-bad = Le port est un nombre de 1 à 65535.
live-setup-topic-needed = Il faut un topic.
live-setup-saved = Source en direct { $name } enregistrée.
live-setup-not-saved = Source en direct { $name } non enregistrée : { $error } Vérifiez que le dossier des réglages est accessible en écriture, puis configurez-la de nouveau.
live-setup-offer-title = Ouvrir { $name } maintenant ?
live-setup-open-now = L'ouvrir maintenant
live-setup-later = Pas maintenant
live-setup-not-now = Non ouverte. « Ouvrir une source en direct » l'ouvre plus tard.
live-form-intro-new = Configurer une source en direct. Tab change de champ, Entrée enregistre, Échap annule.
live-form-intro-edit = Modifier { $name }. Tab change de champ, Entrée enregistre, Échap annule.
live-form-intro-file = Source en direct de { $file }. Vérifiez chaque champ : Tab change, Entrée l'enregistre comme nouvelle source, Échap annule.
live-form-name = Nom
live-form-host = Hôte du broker
live-form-tls = Chiffrement
live-form-ca-file = Fichier de l'AC
live-form-port = Port
live-form-topic = Topic
live-form-username = Nom d'utilisateur
live-form-view = Vue
live-form-choice-label = { $label }, { $n } sur { $total }, Flèche haut ou bas change
live-form-choice-field = { $label } : { $value }. Flèche haut ou bas le change.
live-form-tls-off = Désactivé
live-form-tls-system = Racines fournies
live-form-tls-ca-file = Fichier de l'AC
live-form-tls-off-about = Désactivé : non chiffré, port 1883 ; convient par Tailscale.
live-form-tls-system-about = Racines fournies : TLS, vérifié avec les autorités de certification fournies avec { -brand }, port 8883.
live-form-tls-ca-file-about = Fichier de l'AC : TLS, vérifié avec un fichier d'autorité de certification que vous indiquez, port 8883.
live-form-view-log = Journal
live-form-view-values = Valeurs
live-form-view-log-about = Journal : une entrée par message.
live-form-view-values-about = Valeurs : une ligne par champ, mise à jour sur place.
live-form-name-needed = Un nom est nécessaire.
live-form-not-a-choice = { $label } est l'un de ceux-ci : { $choices }. Flèche haut ou bas le change.
live-form-saved-beside = Source en direct { $name } enregistrée, à côté d'une autre source du même nom.
live-form-unchanged = { $name } inchangée.
live-form-cancelled = Annulé. Aucune source en direct enregistrée.
live-form-cancelled-edit = Annulé. { $name } inchangée.
live-edit-title = Modifier quelle source en direct ?
live-save-file-title = Enregistrer quelle source en direct comme fichier ?
live-save-file-done = { $name } enregistrée sous { $path }, sans mot de passe.
live-save-file-failed = { $name } non enregistrée comme fichier : { $error } Choisissez un autre dossier et réessayez.
live-file-not-read = { $file } n'a pas été ouvert comme source en direct : { $reason }
live-file-too-large = il dépasse { $max } octets.
live-file-not-text = ce n'est pas du texte.
live-file-not-toml = ce n'est pas du TOML valide à la ligne { $line }.
live-file-left-out = Omis du fichier : { $items }.
live-file-note-password = un mot de passe, que { -brand } ne lit jamais dans un fichier ; il est demandé après l'enregistrement
live-file-note-unknown = la clé inconnue { $key }
live-file-note-too-many = tout sauf les { $max } premiers de { $key }
live-values-title = Valeurs en direct
live-values-empty = Pas encore de valeurs.
live-value-none = aucune valeur
live-value-stale = aucune valeur récente
live-values-more = { $count } de plus
live-values-count =
    { $count ->
        [one] 1 champ
       *[other] { $count } champs
    }
live-values-no-table = La vue des valeurs n'a pas de tableau : chaque champ occupe une ligne.
live-values-no-log = La vue des valeurs ne tient pas de journal à enregistrer.
live-values-full = La vue des valeurs contient au plus { $max } champs ; les autres sont omis.
live-values-resumed =
    { $count ->
        [0] Mises à jour reprises. Aucun champ n'a changé.
        [one] Mises à jour reprises, 1 champ a changé.
       *[other] Mises à jour reprises, { $count } champs ont changé.
    }
live-field-none = Il n'y a pas de champ sur cette ligne.
live-field-show-on = Les changements de { $field } sont affichés.
live-field-show-off = Les changements de { $field } ne sont plus affichés.
live-field-speak-on = Les changements de { $field } sont lus à voix haute.
live-field-speak-off = Les changements de { $field } ne sont plus lus à voix haute.
live-state-connecting = Connexion
live-state-reconnecting = Reconnexion
live-state-encrypted = Connecté, chiffré
live-state-tailscale = Connecté par Tailscale
live-state-plain = Connecté, non chiffré
live-state-lost = Connexion perdue

## Beta 5, phase 5b: the broker in the app, sharing a document live,
## check-ins, pinned keys and join codes. $name is a document or live
## source; $rev a revision number; $topic an MQTT topic; $code and $pin
## are in groups of four; $reason is a sentence in words.
prompt-live-check-in = Pointage
live-form-pin = Empreinte
live-form-join-code = Code d’accès
live-form-tls-pinned = Clé épinglée
live-form-tls-pinned-about = Clé épinglée : TLS, ne fait confiance qu’à la clé du broker dont vous tapez l’empreinte, telle que l’affiche le broker de { -brand }, port 8883.
live-form-tls-join = Code d’accès
live-form-tls-join-about = Code d’accès : TLS avec la clé que le code désigne, connexion avec le code, port 8883.
live-form-pin-needed = Une empreinte est nécessaire : le broker l’affiche après « Certificate pin ».
live-form-pin-bad = Ce n’est pas une empreinte : elle a 52 caractères tels que le broker les affiche, ou 12 d’un code d’accès, par groupes de quatre.
live-form-join-needed = Un code d’accès est nécessaire : 16 caractères, en quatre groupes de quatre.
live-form-join-bad = Ce n’est pas un code d’accès : il a 16 lettres et chiffres, en quatre groupes de quatre.
live-file-note-join-code = un code d’accès, que { -brand } ne lit jamais dans un fichier ; demandez le code lui-même
live-pin-not-read = Impossible d’ouvrir { $name } : son empreinte n’en est pas une. Modifiez la source et retapez l’empreinte.
live-document-shown = { $name }, révision { $rev }, publié en direct.
live-document-updated = { $name } mis à jour, révision { $rev }.
live-document-revision = Révision { $rev }
live-document-entries-left-out = Les entrées de journal de cette source sont omises : elle affiche un document publié.
live-document-one-view = C’est un document publié : il n’a ni entrées, ni tableau, ni journal.
live-broker-status =
    { $count ->
        [one] Broker actif, 1 client
       *[other] Broker actif, { $count } clients
    }
live-share-status = Publication, révision { $rev }
live-broker-already = Le broker est déjà actif.
live-broker-no-folder = il n’y a pas de dossier de données pour sa clé et ses comptes.
live-broker-not-started = Broker non démarré : { $reason }
live-broker-nobody = aucun compte et aucun code d’accès, donc personne ne pourrait se connecter. Ajoutez un compte avec tw broker account add, ou activez le code d’accès.
live-broker-bad-address = { $address } n’est pas une adresse d’écoute. Donnez une adresse IP, comme 127.0.0.1 ou 100.101.1.2, avec ou sans port.
live-broker-started-local = Broker actif : { $addresses }, TLS, cet ordinateur seulement.
live-broker-started = Broker actif : { $addresses }, TLS.
live-broker-started-plain-local = Broker actif : { $addresses }, non chiffré, cet ordinateur seulement.
live-broker-started-plain = Broker actif : { $addresses }, non chiffré.
live-broker-plain-warning = Avertissement : le broker n’est pas chiffré, sur un réseau qui n’est pas seulement cet ordinateur ou Tailscale. Tout le monde y peut lire les messages et les mots de passe.
live-broker-join-code = Code d’accès : { $code }.
live-broker-pin = Empreinte du certificat : { $pin }.
live-broker-not-running = Aucun broker n’est actif. Démarrer votre broker en démarre un.
live-join-code-title = Code d’accès
live-pin-title = Empreinte du certificat
live-broker-no-join-code = Pas de code d’accès : le broker fonctionne avec des comptes. Les lecteurs épinglent sa clé.
live-broker-no-join-code-plain = Ni code d’accès ni empreinte : le broker n’est pas chiffré.
live-code-group = Groupe { $n } sur { $total } : { $group }
live-join-code-copied = Code d’accès copié.
live-join-link-copied = Lien d’accès copié.
live-join-code-pasted = Code d’accès collé.
live-join-link-filled = Rempli avec le lien : hôte, port, code et topic.
live-join-link-filled-no-topic = Rempli avec le lien : hôte, port et code. Tapez le topic.
live-join-link-bad = Pas un lien d’accès : { $reason }
live-form-intro-link = Rejoindre par un lien. Collez le lien d’accès dans n’importe quel champ ; Entrée rejoint, Échap annule.
live-form-intro-link-clipboard = Rejoindre par le lien du presse-papiers. Entrée rejoint, Échap annule.
live-qr-title = Code QR d’accès
live-qr-warning = Quiconque voit ce code peut rejoindre.
live-qr-image = Code QR pour rejoindre { $host }
live-qr-save = Enregistrer le code QR
live-qr-draw = Dessiner le code QR en caractères
live-qr-drawing-title = Code QR d’accès en caractères
live-qr-drawing-warning = Des caractères de bloc suivent, pour une caméra ; un lecteur d’écran les lit comme du bruit. Échap ferme.
live-qr-failed = Pas de code QR : { $reason }
live-qr-save-type = Code QR non enregistré : le nom du fichier doit finir par .png ou .svg.
live-qr-save-failed = Code QR non enregistré : { $error } Choisissez un autre dossier ou un autre nom.
live-qr-saved = Code QR enregistré : { $path }
live-qr-intro-clipboard = Rejoindre depuis le code QR du presse-papiers. Entrée rejoint, Échap annule.
live-qr-intro-file = Rejoindre depuis le code QR de { $file }. Entrée rejoint, Échap annule.
live-qr-clipboard-no-link = Pas de lien d’accès dans l’image du presse-papiers : { $reason } Choisissez un fichier image.
live-qr-no-link = Pas de lien d’accès : { $reason }
live-qr-other-text = Le code QR contient autre chose.
live-qr-not-found = Aucun code QR n’a été trouvé dans l’image.
live-qr-unreadable = Un code QR a été trouvé mais n’a pas pu être lu. Essayez une image plus nette ou plus grande.
live-qr-too-large = Le fichier image est trop volumineux.
live-qr-too-long = Le lien d’accès ne tient pas dans un code QR.
live-qr-image-error = L’image n’a pas pu être lue : { $error } Essayez un fichier PNG ou JPEG.
live-qr-reading = Lecture de l’image QR.
prompt-save-join-qr = Enregistrer le code QR dans un fichier (.png ou .svg)
prompt-join-qr-image = Fichier image avec le code QR
chooser-qr-files = Images PNG et SVG
live-field-menu-title = Champ du code d’accès
live-field-hide-join-code = Masquer le code d’accès
live-field-empty = Le champ du code d’accès est vide.
gui-qr-link = Lien d’accès
live-share-stopped-with-broker = La publication s’est arrêtée avec le broker.
live-broker-stopped = Broker arrêté. Tous les lecteurs sont déconnectés.
live-send-lost = l’envoi s’est arrêté avant la fin.
live-broker-joined = Connecté : { $user }.
live-broker-left = Parti : { $user }.
live-broker-refused = Connexion refusée : { $user }, depuis { $address }.
live-broker-event = Broker : { $event }
live-broker-join-off = Code d’accès désactivé : trop de codes erronés. Redémarrez le broker pour obtenir un nouveau code.
live-send-too-large = le message fait { $len } octets ; la limite est { $max }.
live-send-no-password = Non envoyé : aucun mot de passe n’est gardé pour { $name }. Ouvrez-la une fois pour le taper.
live-checkin-sent = Publié : { $entry }, sur { $topic }.
live-share-sent = Publié : { $name }, révision { $rev }, sur { $topic }.
live-checkin-not-sent = Pointage non publié : { $reason }
live-share-not-sent = { $name } non publié : { $reason }
live-share-already = { $name } est déjà publié sur { $topic }. Arrêtez d’abord la publication.
live-share-no-document = Ouvrez un document pour le publier en direct.
live-share-live-document = Un document en direct ne peut pas être republié. Ouvrez un fichier de document pour le publier.
live-share-through-title = Publier par quelle source en direct ?
live-send-nowhere = Rien par où publier. Démarrez votre broker, ou configurez une source en direct qui peut publier.
live-share-none = Aucun document n’est publié.
live-share-stopped = Publication de { $name } arrêtée. Les lecteurs gardent la révision { $rev }.
live-checkin-through-title = Publier le pointage par quelle source en direct ?
live-checkin-to-broker = le broker de cet ordinateur
live-checkin-intro = Pointage vers { $to }, sur { $topic }. Tab change de champ, Entrée publie, Échap annule.
live-checkin-empty = Remplissez au moins un champ.
live-checkin-cancelled = Annulé. Aucun pointage publié.
