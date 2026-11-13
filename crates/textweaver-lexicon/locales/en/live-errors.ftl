### Live documents: why a live source is down, and why a join link does not read.

## Why a live source is down (beta 6). Each is said after "Disconnected
## from NAME:" or "Cannot connect to NAME:". $target is the broker as
## host:port, $host its host, $path a CA file, $error and $detail the
## system's or the TLS library's own words, $packet an MQTT packet name
## such as PUBREC, $topic a topic filter. $why is one of the live-why-*
## or live-topic-problem-* pieces below.
live-down-bad-login = Not authorized: the broker refused the user name or password.
live-down-login-refused = Not authorized: the broker refused the login. Check the user name and password.
live-down-protocol-version = Refused: the broker does not accept MQTT 3.1.1.
live-down-client-id = Refused: the broker rejected the client id.
live-down-unavailable = Unreachable: the broker says it is unavailable right now.
live-down-unexpected-success = Protocol error: unexpected success code.
live-down-tls-needed = TLS needed: { $target } answered with TLS. Turn TLS on for this source.
live-down-cannot-connect = Unreachable: cannot connect to { $target }: { $why }.
live-down-connect-failed = Unreachable: cannot connect to { $target }.
live-down-handshake = TLS failure: the handshake with { $target } did not finish: { $why }.
live-down-lost-logging-in = Connection lost while logging in to { $target }: { $why }.
live-down-lost-logging-in-tls = Connection lost while logging in to { $target }: { $why }. If this port expects TLS, turn TLS on for this source.
live-down-lost = Connection lost to { $target }: { $why }.
live-down-session-ended = Connection lost to { $target }: the broker ended the session.
live-down-stopped-answering = Connection lost to { $target }: the broker stopped answering.
live-down-login-timeout = Unreachable: { $target } accepted the connection but did not answer the login in time.
live-down-no-address = Unreachable: the host { $host } has no address.
live-down-host-not-found = Unreachable: cannot find the host { $host }: { $why }.
live-down-thread = Failed: cannot start the connection thread: { $error } Open the source again.
live-down-topics-refused = Not authorized: the broker refused every topic this source asked for.
live-down-qos2 = Protocol error: the broker sent QoS 2, which was not asked for.
live-down-unexpected-packet = Protocol error: unexpected { $packet } packet from the broker.
live-down-no-connack = Protocol error: the broker did not answer the login with CONNACK.
live-down-no-host = Settings error: no host is set.
live-down-port-zero = Settings error: the port is 0.
live-down-no-topic = Settings error: no topic is set.
live-down-bad-topic = Settings error: { $why }: { $topic }.
live-down-password-without-user = Settings error: a password needs a user name.
live-down-tls-on-plain = Settings error: TLS was asked for on a plain connection.
live-down-ca-unreadable = Settings error: cannot read the CA file { $path }: { $error } Check the CA file in the source's settings.
live-down-ca-not-pem = Settings error: the CA file { $path } is not valid PEM: { $error } Choose a PEM certificate file.
live-down-ca-refused = Settings error: a certificate in { $path } was refused: { $error } Check the CA file in the source's settings.
live-down-ca-empty = Settings error: the CA file { $path } holds no certificate.
live-down-bad-tls-host = Settings error: { $host } is not a valid host name for TLS.
live-down-unknown-issuer = TLS failure: the broker's certificate is not trusted. Its issuer is unknown; set a CA file if the broker uses a private certificate.
live-down-wrong-host-name = TLS failure: the broker's certificate is for a different host name.
live-down-expired = TLS failure: the broker's certificate has expired.
live-down-not-valid-yet = TLS failure: the broker's certificate is not valid yet. Check this computer's clock.
live-down-revoked = TLS failure: the broker's certificate was revoked.
live-down-pin-mismatch = TLS failure: the broker's key does not match the pin. Check the pin or join code; this may not be the broker you meant.
live-down-certificate-refused = TLS failure: the broker's certificate was refused ({ $detail }).
live-down-handshake-alert = TLS failure: the broker ended the handshake ({ $detail }).
live-down-no-certificate = TLS failure: the broker sent no certificate.
live-down-incompatible = TLS failure: the broker's TLS settings are not compatible ({ $detail }).
live-down-not-tls = TLS failure: the broker did not answer with TLS ({ $detail }). Is this a plain port?
live-down-tls = TLS failure: { $error } Check the source's TLS settings.

## Why a connection step failed, lower case, to follow "cannot connect to
## host:port:" and the like.
live-why-nothing-listening = nothing is listening on that port
live-why-no-answer = no answer in time
live-why-reset = the connection was reset
live-why-closed = the broker closed the connection
live-why-no-route = no route to the host
live-why-lookup-failed = the lookup failed

## Why a topic filter cannot be followed, lower case, to follow a lead-in.
live-topic-problem-empty = a topic is empty
live-topic-problem-too-long = a topic is longer than 65535 bytes
live-topic-problem-nul = a topic holds a NUL character
live-topic-problem-hash = # may only be a whole last level of a topic
live-topic-problem-plus = + may only be a whole level of a topic
