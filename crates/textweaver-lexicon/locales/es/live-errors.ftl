### Live documents: why a live source is down, and why a join link does not read.

## Why a live source is down (beta 6). Each is said after "Disconnected
## from NAME:" or "Cannot connect to NAME:". $target is the broker as
## host:port, $host its host, $path a CA file, $error and $detail the
## system's or the TLS library's own words, $packet an MQTT packet name
## such as PUBREC, $topic a topic filter. $why is one of the live-why-*
## or live-topic-problem-* pieces below.
live-down-bad-login = No autorizado: el broker rechazó el nombre de usuario o la contraseña.
live-down-login-refused = No autorizado: el broker rechazó el inicio de sesión. Revise el nombre de usuario y la contraseña.
live-down-protocol-version = Rechazado: el broker no acepta MQTT 3.1.1.
live-down-client-id = Rechazado: el broker rechazó el id de cliente.
live-down-unavailable = Inalcanzable: el broker dice que no está disponible ahora.
live-down-unexpected-success = Error de protocolo: código de éxito inesperado.
live-down-tls-needed = Hace falta TLS: { $target } respondió con TLS. Active TLS para esta fuente.
live-down-cannot-connect = Inalcanzable: no se puede conectar con { $target }: { $why }.
live-down-connect-failed = Inalcanzable: no se puede conectar con { $target }.
live-down-handshake = Fallo de TLS: el protocolo de enlace con { $target } no terminó: { $why }.
live-down-lost-logging-in = Conexión perdida al iniciar sesión en { $target }: { $why }.
live-down-lost-logging-in-tls = Conexión perdida al iniciar sesión en { $target }: { $why }. Si este puerto espera TLS, active TLS para esta fuente.
live-down-lost = Conexión perdida con { $target }: { $why }.
live-down-session-ended = Conexión perdida con { $target }: el broker terminó la sesión.
live-down-stopped-answering = Conexión perdida con { $target }: el broker dejó de responder.
live-down-login-timeout = Inalcanzable: { $target } aceptó la conexión pero no respondió a tiempo al inicio de sesión.
live-down-no-address = Inalcanzable: el host { $host } no tiene dirección.
live-down-host-not-found = Inalcanzable: no se encuentra el host { $host }: { $why }.
live-down-thread = Falló: no se puede iniciar el hilo de conexión: { $error } Abra la fuente de nuevo.
live-down-topics-refused = No autorizado: el broker rechazó todos los temas que pidió esta fuente.
live-down-qos2 = Error de protocolo: el broker envió QoS 2, que no se había pedido.
live-down-unexpected-packet = Error de protocolo: paquete { $packet } inesperado del broker.
live-down-no-connack = Error de protocolo: el broker no respondió al inicio de sesión con CONNACK.
live-down-no-host = Error de ajustes: no hay ningún host definido.
live-down-port-zero = Error de ajustes: el puerto es 0.
live-down-no-topic = Error de ajustes: no hay ningún tema definido.
live-down-bad-topic = Error de ajustes: { $why }: { $topic }.
live-down-password-without-user = Error de ajustes: una contraseña necesita un nombre de usuario.
live-down-tls-on-plain = Error de ajustes: se pidió TLS en una conexión sin cifrar.
live-down-ca-unreadable = Error de ajustes: no se puede leer el archivo de CA { $path }: { $error } Revise el archivo de CA en los ajustes de la fuente.
live-down-ca-not-pem = Error de ajustes: el archivo de CA { $path } no es un PEM válido: { $error } Elija un archivo de certificado PEM.
live-down-ca-refused = Error de ajustes: se rechazó un certificado de { $path }: { $error } Revise el archivo de CA en los ajustes de la fuente.
live-down-ca-empty = Error de ajustes: el archivo de CA { $path } no contiene ningún certificado.
live-down-bad-tls-host = Error de ajustes: { $host } no es un nombre de host válido para TLS.
live-down-unknown-issuer = Fallo de TLS: el certificado del broker no es de confianza. Se desconoce su emisor; defina un archivo de CA si el broker usa un certificado privado.
live-down-wrong-host-name = Fallo de TLS: el certificado del broker es de otro nombre de host.
live-down-expired = Fallo de TLS: el certificado del broker ha caducado.
live-down-not-valid-yet = Fallo de TLS: el certificado del broker aún no es válido. Revise el reloj de este equipo.
live-down-revoked = Fallo de TLS: el certificado del broker fue revocado.
live-down-pin-mismatch = Fallo de TLS: la clave del broker no coincide con el pin. Revise el pin o el código de acceso; puede que no sea el broker que quería.
live-down-certificate-refused = Fallo de TLS: se rechazó el certificado del broker ({ $detail }).
live-down-handshake-alert = Fallo de TLS: el broker terminó el protocolo de enlace ({ $detail }).
live-down-no-certificate = Fallo de TLS: el broker no envió ningún certificado.
live-down-incompatible = Fallo de TLS: los ajustes de TLS del broker no son compatibles ({ $detail }).
live-down-not-tls = Fallo de TLS: el broker no respondió con TLS ({ $detail }). ¿Es un puerto sin cifrar?
live-down-tls = Fallo de TLS: { $error } Revise los ajustes de TLS de la fuente.

## Why a connection step failed, lower case, to follow "cannot connect to
## host:port:" and the like.
live-why-nothing-listening = nadie escucha en ese puerto
live-why-no-answer = sin respuesta a tiempo
live-why-reset = se restableció la conexión
live-why-closed = el broker cerró la conexión
live-why-no-route = no hay ruta hasta el host
live-why-lookup-failed = falló la búsqueda

## Why a topic filter cannot be followed, lower case, to follow a lead-in.
live-topic-problem-empty = un tema está vacío
live-topic-problem-too-long = un tema supera los 65535 bytes
live-topic-problem-nul = un tema contiene un carácter NUL
live-topic-problem-hash = # solo puede ser el último nivel completo de un tema
live-topic-problem-plus = + solo puede ser un nivel completo de un tema

## Why a join link does not read (beta 6), lower case, said after "Not a
## join link:" or "No join link:". $scheme is twjoin, $key what followed
## the question mark, $character a character as typed, $code a
## character's number, $group its group of four in the code, $count the
## characters a code has, $needed the 16 it needs. $why is one of the
## live-topic-problem-* pieces.
live-link-bad-too-long = el texto es demasiado largo.
live-link-bad-scheme = un enlace de acceso empieza con { $scheme }://.
live-link-bad-spaces = un enlace de acceso no lleva espacios ni saltos de línea.
live-link-bad-no-code = falta el código de acceso después del host y el puerto.
live-link-bad-no-host = falta el host.
live-link-bad-bracket = una dirección IPv6 necesita el corchete de cierre.
live-link-bad-not-ipv6 = la dirección entre corchetes no es una dirección IPv6.
live-link-bad-port = el puerto no es un número de 1 a 65535.
live-link-bad-host-too-long = el nombre del host es demasiado largo.
live-link-bad-host-character = { $character } no se usa en nombres de host. Revise el host.
live-link-bad-host-character-code = un carácter con código { $code } no se usa en nombres de host. Revise el host.
live-link-bad-key = { $key } no forma parte de un enlace de acceso. Solo topic puede ir tras el signo de interrogación.
live-link-bad-topic-twice = el tema aparece dos veces.
live-link-bad-topic-control = el tema contiene un carácter de control.
live-link-bad-topic = { $why }.
live-link-bad-percent = el tema tiene un signo de porcentaje que no va seguido de dos dígitos hexadecimales.
live-link-bad-utf8 = el tema no es texto UTF-8.
live-link-bad-code-empty = falta el código.
live-link-bad-code-length =
    { $count ->
        [one] el código tiene 1 carácter; necesita { $needed }, en cuatro grupos de cuatro.
       *[other] el código tiene { $count } caracteres; necesita { $needed }, en cuatro grupos de cuatro.
    }
live-link-bad-code-character = { $character } en el grupo { $group } no se usa en códigos de acceso. Revise ese carácter.
live-link-bad-code-character-code = un carácter con código { $code } en el grupo { $group } no se usa en códigos de acceso. Revise ese carácter.
