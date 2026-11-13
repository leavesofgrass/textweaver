### Live documents: why a live source is down, and why a join link does not read.

## Why a live source is down (beta 6). Each is said after "Disconnected
## from NAME:" or "Cannot connect to NAME:". $target is the broker as
## host:port, $host its host, $path a CA file, $error and $detail the
## system's or the TLS library's own words, $packet an MQTT packet name
## such as PUBREC, $topic a topic filter. $why is one of the live-why-*
## or live-topic-problem-* pieces below.
live-down-bad-login = Nicht berechtigt: der Broker hat Benutzername oder Passwort abgelehnt.
live-down-login-refused = Nicht berechtigt: der Broker hat die Anmeldung abgelehnt. Prüfen Sie Benutzername und Passwort.
live-down-protocol-version = Abgelehnt: der Broker akzeptiert MQTT 3.1.1 nicht.
live-down-client-id = Abgelehnt: der Broker hat die Client-ID zurückgewiesen.
live-down-unavailable = Nicht erreichbar: der Broker meldet, dass er gerade nicht verfügbar ist.
live-down-unexpected-success = Protokollfehler: unerwarteter Erfolgscode.
live-down-tls-needed = TLS nötig: { $target } hat mit TLS geantwortet. Schalten Sie TLS für diese Quelle ein.
live-down-cannot-connect = Nicht erreichbar: Verbindung zu { $target } nicht möglich: { $why }.
live-down-connect-failed = Nicht erreichbar: Verbindung zu { $target } nicht möglich.
live-down-handshake = TLS-Fehler: der Handshake mit { $target } wurde nicht abgeschlossen: { $why }.
live-down-lost-logging-in = Verbindung beim Anmelden bei { $target } verloren: { $why }.
live-down-lost-logging-in-tls = Verbindung beim Anmelden bei { $target } verloren: { $why }. Erwartet dieser Port TLS, schalten Sie TLS für diese Quelle ein.
live-down-lost = Verbindung zu { $target } verloren: { $why }.
live-down-session-ended = Verbindung zu { $target } verloren: der Broker hat die Sitzung beendet.
live-down-stopped-answering = Verbindung zu { $target } verloren: der Broker antwortet nicht mehr.
live-down-login-timeout = Nicht erreichbar: { $target } hat die Verbindung angenommen, aber die Anmeldung nicht rechtzeitig beantwortet.
live-down-no-address = Nicht erreichbar: der Host { $host } hat keine Adresse.
live-down-host-not-found = Nicht erreichbar: der Host { $host } wurde nicht gefunden: { $why }.
live-down-thread = Fehlgeschlagen: der Verbindungs-Thread konnte nicht starten: { $error } Öffnen Sie die Quelle erneut.
live-down-topics-refused = Nicht berechtigt: der Broker hat jedes Topic abgelehnt, das diese Quelle angefragt hat.
live-down-qos2 = Protokollfehler: der Broker hat QoS 2 gesendet, das nicht angefordert war.
live-down-unexpected-packet = Protokollfehler: unerwartetes Paket { $packet } vom Broker.
live-down-no-connack = Protokollfehler: der Broker hat die Anmeldung nicht mit CONNACK beantwortet.
live-down-no-host = Einstellungsfehler: kein Host gesetzt.
live-down-port-zero = Einstellungsfehler: der Port ist 0.
live-down-no-topic = Einstellungsfehler: kein Topic gesetzt.
live-down-bad-topic = Einstellungsfehler: { $why }: { $topic }.
live-down-password-without-user = Einstellungsfehler: ein Passwort braucht einen Benutzernamen.
live-down-tls-on-plain = Einstellungsfehler: TLS wurde für eine unverschlüsselte Verbindung verlangt.
live-down-ca-unreadable = Einstellungsfehler: die CA-Datei { $path } lässt sich nicht lesen: { $error } Prüfen Sie die CA-Datei in den Einstellungen der Quelle.
live-down-ca-not-pem = Einstellungsfehler: die CA-Datei { $path } ist kein gültiges PEM: { $error } Wählen Sie eine PEM-Zertifikatsdatei.
live-down-ca-refused = Einstellungsfehler: ein Zertifikat in { $path } wurde abgelehnt: { $error } Prüfen Sie die CA-Datei in den Einstellungen der Quelle.
live-down-ca-empty = Einstellungsfehler: die CA-Datei { $path } enthält kein Zertifikat.
live-down-bad-tls-host = Einstellungsfehler: { $host } ist kein gültiger Hostname für TLS.
live-down-unknown-issuer = TLS-Fehler: dem Zertifikat des Brokers wird nicht vertraut. Der Aussteller ist unbekannt; setzen Sie eine CA-Datei, wenn der Broker ein privates Zertifikat nutzt.
live-down-wrong-host-name = TLS-Fehler: das Zertifikat des Brokers gilt für einen anderen Hostnamen.
live-down-expired = TLS-Fehler: das Zertifikat des Brokers ist abgelaufen.
live-down-not-valid-yet = TLS-Fehler: das Zertifikat des Brokers ist noch nicht gültig. Prüfen Sie die Uhr dieses Computers.
live-down-revoked = TLS-Fehler: das Zertifikat des Brokers wurde widerrufen.
live-down-pin-mismatch = TLS-Fehler: der Schlüssel des Brokers passt nicht zum Pin. Prüfen Sie den Pin oder Beitrittscode; es ist vielleicht nicht der gemeinte Broker.
live-down-certificate-refused = TLS-Fehler: das Zertifikat des Brokers wurde abgelehnt ({ $detail }).
live-down-handshake-alert = TLS-Fehler: der Broker hat den Handshake beendet ({ $detail }).
live-down-no-certificate = TLS-Fehler: der Broker hat kein Zertifikat gesendet.
live-down-incompatible = TLS-Fehler: die TLS-Einstellungen des Brokers sind nicht kompatibel ({ $detail }).
live-down-not-tls = TLS-Fehler: der Broker hat nicht mit TLS geantwortet ({ $detail }). Ist das ein unverschlüsselter Port?
live-down-tls = TLS-Fehler: { $error } Prüfen Sie die TLS-Einstellungen der Quelle.

## Why a connection step failed, lower case, to follow "cannot connect to
## host:port:" and the like.
live-why-nothing-listening = auf diesem Port lauscht nichts
live-why-no-answer = keine Antwort innerhalb der Zeit
live-why-reset = die Verbindung wurde zurückgesetzt
live-why-closed = der Broker hat die Verbindung geschlossen
live-why-no-route = keine Route zum Host
live-why-lookup-failed = die Namensauflösung ist fehlgeschlagen

## Why a topic filter cannot be followed, lower case, to follow a lead-in.
live-topic-problem-empty = ein Topic ist leer
live-topic-problem-too-long = ein Topic ist länger als 65535 Bytes
live-topic-problem-nul = ein Topic enthält ein NUL-Zeichen
live-topic-problem-hash = # darf nur die ganze letzte Ebene eines Topics sein
live-topic-problem-plus = + darf nur eine ganze Ebene eines Topics sein

## Why a join link does not read (beta 6), lower case, said after "Not a
## join link:" or "No join link:". $scheme is twjoin, $key what followed
## the question mark, $character a character as typed, $code a
## character's number, $group its group of four in the code, $count the
## characters a code has, $needed the 16 it needs. $why is one of the
## live-topic-problem-* pieces.
live-link-bad-too-long = der Text ist viel zu lang.
live-link-bad-scheme = ein Beitrittslink beginnt mit { $scheme }://.
live-link-bad-spaces = ein Beitrittslink hat keine Leerzeichen oder Zeilenumbrüche.
live-link-bad-no-code = nach Host und Port wurde kein Beitrittscode angegeben.
live-link-bad-no-host = es wurde kein Host angegeben.
live-link-bad-bracket = eine IPv6-Adresse braucht ihre schließende eckige Klammer.
live-link-bad-not-ipv6 = die Adresse in eckigen Klammern ist keine IPv6-Adresse.
live-link-bad-port = der Port ist keine Zahl von 1 bis 65535.
live-link-bad-host-too-long = der Hostname ist zu lang.
live-link-bad-host-character = { $character } kommt in Hostnamen nicht vor. Prüfen Sie den Host.
live-link-bad-host-character-code = ein Zeichen mit dem Code { $code } kommt in Hostnamen nicht vor. Prüfen Sie den Host.
live-link-bad-key = { $key } gehört nicht zu einem Beitrittslink. Nach dem Fragezeichen darf nur topic folgen.
live-link-bad-topic-twice = das Topic wurde doppelt angegeben.
live-link-bad-topic-control = das Topic enthält ein Steuerzeichen.
live-link-bad-topic = { $why }.
live-link-bad-percent = das Topic hat ein Prozentzeichen, auf das keine zwei Hexziffern folgen.
live-link-bad-utf8 = das Topic ist kein UTF-8-Text.
live-link-bad-code-empty = es wurde kein Code angegeben.
live-link-bad-code-length =
    { $count ->
        [one] der Code hat 1 Zeichen; er braucht { $needed }, in vier Gruppen zu vier.
       *[other] der Code hat { $count } Zeichen; er braucht { $needed }, in vier Gruppen zu vier.
    }
live-link-bad-code-character = { $character } in Gruppe { $group } kommt in Beitrittscodes nicht vor. Prüfen Sie dieses Zeichen.
live-link-bad-code-character-code = ein Zeichen mit dem Code { $code } in Gruppe { $group } kommt in Beitrittscodes nicht vor. Prüfen Sie dieses Zeichen.
