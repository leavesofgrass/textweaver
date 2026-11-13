### Live documents: why a live source is down, and why a join link does not read.

## Why a live source is down (beta 6). Each is said after "Disconnected
## from NAME:" or "Cannot connect to NAME:". $target is the broker as
## host:port, $host its host, $path a CA file, $error and $detail the
## system's or the TLS library's own words, $packet an MQTT packet name
## such as PUBREC, $topic a topic filter. $why is one of the live-why-*
## or live-topic-problem-* pieces below.
live-down-bad-login = Non autorisé : le broker a refusé le nom d’utilisateur ou le mot de passe.
live-down-login-refused = Non autorisé : le broker a refusé la connexion. Vérifiez le nom d’utilisateur et le mot de passe.
live-down-protocol-version = Refusé : le broker n’accepte pas MQTT 3.1.1.
live-down-client-id = Refusé : le broker a rejeté l’identifiant du client.
live-down-unavailable = Injoignable : le broker dit qu’il est indisponible pour le moment.
live-down-unexpected-success = Erreur de protocole : code de réussite inattendu.
live-down-tls-needed = TLS requis : { $target } a répondu en TLS. Activez TLS pour cette source.
live-down-cannot-connect = Injoignable : connexion impossible à { $target } : { $why }.
live-down-connect-failed = Injoignable : connexion impossible à { $target }.
live-down-handshake = Échec TLS : la négociation avec { $target } n’a pas abouti : { $why }.
live-down-lost-logging-in = Connexion perdue pendant l’ouverture de session sur { $target } : { $why }.
live-down-lost-logging-in-tls = Connexion perdue pendant l’ouverture de session sur { $target } : { $why }. Si ce port attend du TLS, activez TLS pour cette source.
live-down-lost = Connexion perdue avec { $target } : { $why }.
live-down-session-ended = Connexion perdue avec { $target } : le broker a mis fin à la session.
live-down-stopped-answering = Connexion perdue avec { $target } : le broker a cessé de répondre.
live-down-login-timeout = Injoignable : { $target } a accepté la connexion mais n’a pas répondu à temps à l’ouverture de session.
live-down-no-address = Injoignable : l’hôte { $host } n’a aucune adresse.
live-down-host-not-found = Injoignable : hôte { $host } introuvable : { $why }.
live-down-thread = Échec : impossible de démarrer le fil de connexion : { $error } Rouvrez la source.
live-down-topics-refused = Non autorisé : le broker a refusé tous les topics demandés par cette source.
live-down-qos2 = Erreur de protocole : le broker a envoyé du QoS 2, qui n’était pas demandé.
live-down-unexpected-packet = Erreur de protocole : paquet { $packet } inattendu de la part du broker.
live-down-no-connack = Erreur de protocole : le broker n’a pas répondu à l’ouverture de session par CONNACK.
live-down-no-host = Erreur de réglages : aucun hôte n’est défini.
live-down-port-zero = Erreur de réglages : le port est 0.
live-down-no-topic = Erreur de réglages : aucun topic n’est défini.
live-down-bad-topic = Erreur de réglages : { $why } : { $topic }.
live-down-password-without-user = Erreur de réglages : un mot de passe demande un nom d’utilisateur.
live-down-tls-on-plain = Erreur de réglages : TLS a été demandé sur une connexion non chiffrée.
live-down-ca-unreadable = Erreur de réglages : impossible de lire le fichier de l’AC { $path } : { $error } Vérifiez le fichier de l’AC dans les réglages de la source.
live-down-ca-not-pem = Erreur de réglages : le fichier de l’AC { $path } n’est pas du PEM valide : { $error } Choisissez un fichier de certificat PEM.
live-down-ca-refused = Erreur de réglages : un certificat de { $path } a été refusé : { $error } Vérifiez le fichier de l’AC dans les réglages de la source.
live-down-ca-empty = Erreur de réglages : le fichier de l’AC { $path } ne contient aucun certificat.
live-down-bad-tls-host = Erreur de réglages : { $host } n’est pas un nom d’hôte valide pour TLS.
live-down-unknown-issuer = Échec TLS : le certificat du broker n’est pas approuvé. Son émetteur est inconnu ; indiquez un fichier d’AC si le broker utilise un certificat privé.
live-down-wrong-host-name = Échec TLS : le certificat du broker est pour un autre nom d’hôte.
live-down-expired = Échec TLS : le certificat du broker a expiré.
live-down-not-valid-yet = Échec TLS : le certificat du broker n’est pas encore valide. Vérifiez l’horloge de cet ordinateur.
live-down-revoked = Échec TLS : le certificat du broker a été révoqué.
live-down-pin-mismatch = Échec TLS : la clé du broker ne correspond pas à l’empreinte. Vérifiez l’empreinte ou le code d’accès ; ce n’est peut-être pas le broker voulu.
live-down-certificate-refused = Échec TLS : le certificat du broker a été refusé ({ $detail }).
live-down-handshake-alert = Échec TLS : le broker a mis fin à la négociation ({ $detail }).
live-down-no-certificate = Échec TLS : le broker n’a envoyé aucun certificat.
live-down-incompatible = Échec TLS : les réglages TLS du broker ne sont pas compatibles ({ $detail }).
live-down-not-tls = Échec TLS : le broker n’a pas répondu en TLS ({ $detail }). Est-ce un port non chiffré ?
live-down-tls = Échec TLS : { $error } Vérifiez les réglages TLS de la source.

## Why a connection step failed, lower case, to follow "cannot connect to
## host:port:" and the like.
live-why-nothing-listening = rien n’écoute sur ce port
live-why-no-answer = pas de réponse à temps
live-why-reset = la connexion a été réinitialisée
live-why-closed = le broker a fermé la connexion
live-why-no-route = aucune route vers l’hôte
live-why-lookup-failed = la recherche a échoué

## Why a topic filter cannot be followed, lower case, to follow a lead-in.
live-topic-problem-empty = un topic est vide
live-topic-problem-too-long = un topic dépasse 65535 octets
live-topic-problem-nul = un topic contient un caractère NUL
live-topic-problem-hash = # ne peut être que le dernier niveau complet d’un topic
live-topic-problem-plus = + ne peut être qu’un niveau complet d’un topic

## Why a join link does not read (beta 6), lower case, said after "Not a
## join link:" or "No join link:". $scheme is twjoin, $key what followed
## the question mark, $character a character as typed, $code a
## character's number, $group its group of four in the code, $count the
## characters a code has, $needed the 16 it needs. $why is one of the
## live-topic-problem-* pieces.
live-link-bad-too-long = le texte est beaucoup trop long.
live-link-bad-scheme = un lien d’accès commence par { $scheme }://.
live-link-bad-spaces = un lien d’accès n’a ni espaces ni sauts de ligne.
live-link-bad-no-code = aucun code d’accès n’a été donné après l’hôte et le port.
live-link-bad-no-host = aucun hôte n’a été donné.
live-link-bad-bracket = une adresse IPv6 demande son crochet fermant.
live-link-bad-not-ipv6 = l’adresse entre crochets n’est pas une adresse IPv6.
live-link-bad-port = le port n’est pas un nombre de 1 à 65535.
live-link-bad-host-too-long = le nom d’hôte est trop long.
live-link-bad-host-character = { $character } n’est pas utilisé dans les noms d’hôte. Vérifiez l’hôte.
live-link-bad-host-character-code = un caractère de code { $code } n’est pas utilisé dans les noms d’hôte. Vérifiez l’hôte.
live-link-bad-key = { $key } ne fait pas partie d’un lien d’accès. Seul topic peut suivre le point d’interrogation.
live-link-bad-topic-twice = le topic est donné deux fois.
live-link-bad-topic-control = le topic contient un caractère de contrôle.
live-link-bad-topic = { $why }.
live-link-bad-percent = le topic contient un signe pour cent qui n’est pas suivi de deux chiffres hexadécimaux.
live-link-bad-utf8 = le topic n’est pas du texte UTF-8.
live-link-bad-code-empty = aucun code n’a été donné.
live-link-bad-code-length =
    { $count ->
        [one] le code a 1 caractère ; il en faut { $needed }, en quatre groupes de quatre.
       *[other] le code a { $count } caractères ; il en faut { $needed }, en quatre groupes de quatre.
    }
live-link-bad-code-character = { $character } du groupe { $group } n’est pas utilisé dans les codes d’accès. Vérifiez ce caractère.
live-link-bad-code-character-code = un caractère de code { $code } du groupe { $group } n’est pas utilisé dans les codes d’accès. Vérifiez ce caractère.
