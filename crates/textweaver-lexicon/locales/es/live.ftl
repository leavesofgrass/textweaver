### Live documents, live sources, and the broker.

## Live documents (beta 5): settings, and live sources' passwords. $source
## is a live source's name; $reason says why the credential store failed.
section-live = Documentos en vivo
prompt-live-password = Contraseña de { $source }, Intro cancela
prompt-live-password-any = Contraseña del origen en vivo, Intro cancela
live-source-unknown = No hay ningún origen en vivo llamado { $source }. Revise los orígenes en settings.toml.
live-password-no-store = Esta versión no tiene almacén de credenciales: { $source } no puede iniciar sesión con contraseña. Use un paquete de versión.
live-password-invalid = No es una contraseña: hasta 1024 caracteres, sin saltos de línea ni tabulaciones.
live-password-kept = Contraseña de { $source } guardada en el almacén de credenciales del sistema.
live-password-not-kept = Contraseña de { $source } no guardada: { $reason }. Se usa solo en esta sesión.
live-password-forgotten = Contraseña de { $source } olvidada.
live-password-none = No hay contraseña guardada para { $source }.
live-password-not-forgotten = Contraseña de { $source } no olvidada: { $reason }. Inténtelo de nuevo o quítela en el almacén de credenciales del sistema.

## Beta 5: live documents (a log fed by a live source, such as an MQTT
## broker). $name is the source's name, $topic an MQTT topic, $path and
## $folder file paths, $line a log entry as one line.
live-log-title = Registro en vivo
live-log-empty = Todavía no hay entradas.
live-entry-empty = Entrada vacía.
live-field-callsign = Indicativo
live-field-name = Nombre
live-field-location = Ubicación
live-field-status = Estado
live-field-traffic = Tráfico
live-field-notes = Notas
live-field-time = Hora
live-traffic = Tráfico
live-yes = sí
live-no = no
live-read-only = Este registro en vivo es de solo lectura; no se puede editar.
live-not-open = No hay ningún registro en vivo abierto.
live-view-list = Vista de lista.
live-view-table = Vista de tabla.
live-newest-none = El registro todavía no tiene entradas.
live-paused = Actualizaciones en pausa. Las entradas nuevas esperan hasta que reanude.
live-paused-status = actualizaciones en pausa
live-stopped = { $name } se detuvo. El registro sigue abierto, de solo lectura.
live-log-saved = Registro guardado como { $name } en { $folder }.
live-log-empty-not-saved = El registro todavía no tiene entradas, así que no se guardó nada.
live-log-nowhere = No hay ninguna carpeta donde guardar el registro en vivo.
live-log-saved-already = Este registro se abrió desde { $path }; ya está guardado.
live-connecting = Conectando con { $name }.
live-reconnecting = Volviendo a conectar con { $name }, intento { $attempt }.
live-connected = Conectado con { $name }, cifrado.
live-connected-tailscale = Conectado con { $name } en una dirección del rango de Tailscale. Si es Tailscale, el enlace está cifrado.
live-connected-plain = Conectado con { $name }. La conexión no está cifrada.
live-disconnected = Desconectado de { $name }: { $reason }
live-quiet = Todavía no hay mensajes en { $topic }. Si esto continúa, compruebe que el broker permita leer a esta cuenta.
live-unread =
    { $count ->
        [0] ninguna sin leer
       *[other] { $count } sin leer
    }
live-entries =
    { $count ->
        [one] 1 entrada
       *[other] { $count } entradas
    }
live-resumed =
    { $count ->
        [0] Actualizaciones reanudadas.
        [one] Actualizaciones reanudadas, 1 actualización aplicada.
       *[other] Actualizaciones reanudadas, { $count } actualizaciones aplicadas.
    }

## Beta 5: live sources' commands, the set-up steps, and new entries said.
## $name is a source's name, $topic an MQTT topic, $line an entry as one
## line, $reason a sentence in words.
menu-live = Documentos en vivo
menu-live-reading = Lectura
menu-live-broker = Su broker
menu-live-invite = Invitar a otros
menu-live-publish = Publicar
prompt-live-setup = Configurar una fuente en vivo
prompt-save-live-source-file = Guardar fuente como archivo
live-failed = No se puede conectar con { $name }: { $reason } No habrá más intentos; corríjalo y vuelva a abrir la fuente.
live-new-entries = { $line } { $count } entradas nuevas.
live-unread-newest = { $count }. La más reciente: { $line }
live-not-in-build = Las fuentes en vivo no están en esta versión de { -brand }.
live-speak-on = Decir las entradas nuevas: sí.
live-speak-off = Decir las entradas nuevas: no.
live-none-set-up = No hay fuentes en vivo configuradas. Configurar una fuente en vivo añade una.
live-open-title = Fuentes en vivo
live-source-item = { $name }, { $host }, { $topic }
live-forget-title = ¿Olvidar la contraseña de qué fuente en vivo?
live-no-passwords = Ninguna fuente en vivo inicia sesión con un nombre de usuario, así que no hay contraseña guardada.
live-not-receiving = Ninguna fuente en vivo está recibiendo.
live-cannot-open = No se puede abrir la fuente en vivo. { $problems }
live-not-opened = { $name } no se abrió: no hay contraseña.
live-setup-host-needed = Hace falta un nombre de host o una dirección, sin espacios.
live-setup-ca-file-needed = Este cifrado necesita un archivo de la autoridad de certificación.
live-setup-ca-file-missing = No hay ningún archivo en { $path }.
live-setup-port-bad = El puerto es un número de 1 a 65535.
live-setup-topic-needed = Hace falta un tema.
live-setup-saved = Fuente en vivo { $name } guardada.
live-setup-not-saved = Fuente en vivo { $name } no guardada: { $error } Compruebe que se puede escribir en la carpeta de ajustes y vuelva a configurarla.
live-setup-offer-title = ¿Abrir { $name } ahora?
live-setup-open-now = Abrirla ahora
live-setup-later = Ahora no
live-setup-not-now = No se abrió. Abrir fuente en vivo la abre más tarde.
live-form-intro-new = Configurar una fuente en vivo. Tab cambia de campo, Intro guarda, Escape cancela.
live-form-intro-edit = Editar { $name }. Tab cambia de campo, Intro guarda, Escape cancela.
live-form-intro-file = Fuente en vivo de { $file }. Revise cada campo: Tab cambia, Intro la guarda como una fuente nueva, Escape cancela.
live-form-name = Nombre
live-form-host = Host del broker
live-form-tls = Cifrado
live-form-ca-file = Archivo de CA
live-form-port = Puerto
live-form-topic = Tema
live-form-username = Nombre de usuario
live-form-view = Vista
live-form-choice-label = { $label }, { $n } de { $total }, Flecha arriba o abajo cambia
live-form-choice-field = { $label }: { $value }. Flecha arriba o abajo lo cambia.
live-form-tls-off = No
live-form-tls-system = Raíces incluidas
live-form-tls-ca-file = Archivo de CA
live-form-tls-off-about = No: sin cifrar, puerto 1883; válido a través de Tailscale.
live-form-tls-system-about = Raíces incluidas: TLS, comprobado con las autoridades de certificación que trae { -brand }, puerto 8883.
live-form-tls-ca-file-about = Archivo de CA: TLS, comprobado con un archivo de autoridad de certificación que usted indica, puerto 8883.
live-form-view-log = Registro
live-form-view-values = Valores
live-form-view-log-about = Registro: una entrada por mensaje.
live-form-view-values-about = Valores: una línea por campo, actualizada en su sitio.
live-form-name-needed = Hace falta un nombre.
live-form-not-a-choice = { $label } es uno de estos: { $choices }. Flecha arriba o abajo lo cambia.
live-form-saved-beside = Fuente en vivo { $name } guardada, junto a otra fuente con el mismo nombre.
live-form-unchanged = { $name } sin cambios.
live-form-cancelled = Cancelado. No se guardó ninguna fuente en vivo.
live-form-cancelled-edit = Cancelado. { $name } sin cambios.
live-edit-title = ¿Editar qué fuente en vivo?
live-save-file-title = ¿Guardar qué fuente en vivo como archivo?
live-save-file-done = { $name } guardada como { $path }, sin contraseña.
live-save-file-failed = { $name } no se guardó como archivo: { $error } Elija otra carpeta e inténtelo de nuevo.
live-file-not-read = { $file } no se abrió como fuente en vivo: { $reason }
live-file-too-large = ocupa más de { $max } bytes.
live-file-not-text = no es texto.
live-file-not-toml = no es TOML válido en la línea { $line }.
live-file-left-out = Se omitió del archivo: { $items }.
live-file-note-password = una contraseña, que { -brand } nunca lee de un archivo; se pide después de guardar
live-file-note-unknown = la clave desconocida { $key }
live-file-note-too-many = todo salvo los primeros { $max } de { $key }
live-values-title = Valores en vivo
live-values-empty = Todavía no hay valores.
live-value-none = sin valor
live-value-stale = sin valor reciente
live-values-more = { $count } más
live-values-count =
    { $count ->
        [one] 1 campo
       *[other] { $count } campos
    }
live-values-no-table = La vista de valores no tiene tabla: cada campo es una línea.
live-values-no-log = La vista de valores no lleva un registro que guardar.
live-values-full = La vista de valores admite como máximo { $max } campos; los demás se omiten.
live-values-resumed =
    { $count ->
        [0] Actualizaciones reanudadas. Ningún campo cambió.
        [one] Actualizaciones reanudadas, 1 campo cambió.
       *[other] Actualizaciones reanudadas, { $count } campos cambiaron.
    }
live-field-none = No hay ningún campo en esta línea.
live-field-show-on = Los cambios de { $field } se muestran.
live-field-show-off = Los cambios de { $field } ya no se muestran.
live-field-speak-on = Los cambios de { $field } se leen en voz alta.
live-field-speak-off = Los cambios de { $field } ya no se leen en voz alta.
live-state-connecting = Conectando
live-state-reconnecting = Reconectando
live-state-encrypted = Conectado, cifrado
live-state-tailscale = Conectado por Tailscale
live-state-plain = Conectado, sin cifrar
live-state-lost = Conexión perdida

## Beta 5, phase 5b: the broker in the app, sharing a document live,
## check-ins, pinned keys and join codes. $name is a document or live
## source; $rev a revision number; $topic an MQTT topic; $code and $pin
## are in groups of four; $reason is a sentence in words.
prompt-live-check-in = Registro
live-form-pin = Pin
live-form-join-code = Código de acceso
live-form-tls-pinned = Clave fijada
live-form-tls-pinned-about = Clave fijada: TLS, confía solo en la clave del broker cuyo pin escribe, como lo muestra el broker de { -brand }, puerto 8883.
live-form-tls-join = Código de acceso
live-form-tls-join-about = Código de acceso: TLS con la clave que indica el código, con el código para entrar, puerto 8883.
live-form-pin-needed = Hace falta un pin: el broker lo muestra tras "Certificate pin".
live-form-pin-bad = No es un pin: tiene 52 caracteres, como los muestra el broker, o 12 de un código de acceso, en grupos de cuatro.
live-form-join-needed = Hace falta un código de acceso: 16 caracteres, en cuatro grupos de cuatro.
live-form-join-bad = No es un código de acceso: tiene 16 letras y cifras, en cuatro grupos de cuatro.
live-file-note-join-code = un código de acceso, que { -brand } nunca lee de un archivo; pida el código en sí
live-pin-not-read = No se puede abrir { $name }: su pin no es válido. Edite la fuente y escriba el pin de nuevo.
live-document-shown = { $name }, revisión { $rev }, publicado en vivo.
live-document-updated = { $name } actualizado, revisión { $rev }.
live-document-revision = Revisión { $rev }
live-document-entries-left-out = Se omiten las entradas de registro de esta fuente: muestra un documento publicado.
live-document-one-view = Es un documento publicado: no tiene entradas, tabla ni registro.
live-broker-status =
    { $count ->
        [one] Broker activo, 1 cliente
       *[other] Broker activo, { $count } clientes
    }
live-share-status = Publicando, revisión { $rev }
live-broker-already = El broker ya está en marcha.
live-broker-no-folder = no hay carpeta de datos para su clave y sus cuentas.
live-broker-not-started = Broker no iniciado: { $reason }
live-broker-nobody = no hay cuentas ni código de acceso, así que nadie podría entrar. Añada una cuenta con tw broker account add o active el código de acceso.
live-broker-bad-address = { $address } no es una dirección en la que escuchar. Indique una dirección IP, como 127.0.0.1 o 100.101.1.2, con o sin puerto.
live-broker-started-local = Broker activo: { $addresses }, TLS, solo este equipo.
live-broker-started = Broker activo: { $addresses }, TLS.
live-broker-started-plain-local = Broker activo: { $addresses }, sin cifrar, solo este equipo.
live-broker-started-plain = Broker activo: { $addresses }, sin cifrar.
live-broker-join-code = Código de acceso: { $code }.
live-broker-pin = Pin del certificado: { $pin }.
live-broker-not-running = No hay ningún broker en marcha. Iniciar su broker inicia uno.
live-join-code-title = Código de acceso
live-pin-title = Pin del certificado
live-broker-no-join-code = Sin código de acceso: el broker funciona con cuentas. Los lectores fijan su clave.
live-broker-no-join-code-plain = Ni código de acceso ni pin: el broker no está cifrado.
live-code-group = Grupo { $n } de { $total }: { $group }
live-join-code-copied = Código de acceso copiado.
live-join-link-copied = Enlace de acceso copiado.
live-join-code-pasted = Código de acceso pegado.
live-join-link-filled = Rellenado con el enlace: host, puerto, código y tema.
live-join-link-filled-no-topic = Rellenado con el enlace: host, puerto y código. Escriba el tema.
live-join-link-bad = No es un enlace de acceso: { $reason }
live-form-intro-link = Unirse con un enlace. Pegue el enlace de acceso en cualquier campo; Intro se une, Escape cancela.
live-form-intro-link-clipboard = Unirse con el enlace del portapapeles. Intro se une, Escape cancela.
live-qr-title = Código QR de acceso
live-qr-warning = Cualquiera que vea este código puede unirse.
live-qr-image = Código QR para unirse a { $host }
live-qr-save = Guardar el código QR
live-qr-draw = Dibujar el código QR con caracteres
live-qr-drawing-title = Código QR de acceso con caracteres
live-qr-drawing-warning = Siguen caracteres de bloque para una cámara; un lector de pantalla los lee como ruido. Escape cierra.
live-qr-failed = Sin código QR: { $reason }
live-qr-save-type = Código QR no guardado: el nombre del archivo debe terminar en .png o .svg.
live-qr-save-failed = Código QR no guardado: { $error } Elija otra carpeta u otro nombre.
live-qr-saved = Código QR guardado: { $path }
live-qr-intro-clipboard = Unirse desde el código QR del portapapeles. Intro se une, Escape cancela.
live-qr-intro-file = Unirse desde el código QR de { $file }. Intro se une, Escape cancela.
live-qr-clipboard-no-link = No hay enlace de acceso en la imagen del portapapeles: { $reason } Elija un archivo de imagen.
live-qr-no-link = No hay enlace de acceso: { $reason }
live-qr-other-text = El código QR contiene otra cosa.
live-qr-not-found = No se encontró ningún código QR en la imagen.
live-qr-unreadable = Se encontró un código QR, pero no se pudo leer. Pruebe una imagen más nítida o más grande.
live-qr-too-large = El archivo de imagen es demasiado grande.
live-qr-too-long = El enlace de acceso no cabe en un código QR.
live-qr-image-error = No se pudo leer la imagen: { $error } Pruebe un archivo PNG o JPEG.
live-qr-reading = Leyendo la imagen QR.
prompt-save-join-qr = Guardar el código QR en un archivo (.png o .svg)
prompt-join-qr-image = Archivo de imagen con el código QR
chooser-qr-files = Imágenes PNG y SVG
live-field-menu-title = Campo del código de acceso
live-field-hide-join-code = Ocultar el código de acceso
live-field-empty = El campo del código de acceso está vacío.
gui-qr-link = Enlace de acceso
live-share-stopped-with-broker = Se dejó de publicar con el broker.
live-broker-stopped = Broker detenido. Todos los lectores están desconectados.
live-send-lost = el envío se detuvo antes de terminar.
live-broker-joined = Conectado: { $user }.
live-broker-left = Se fue: { $user }.
live-broker-refused = Acceso rechazado: { $user }, desde { $address }.
live-broker-event = Broker: { $event }
live-broker-join-off = Código de acceso desactivado: demasiados códigos incorrectos. Inicie el broker de nuevo para obtener un código nuevo.
live-send-too-large = el mensaje tiene { $len } bytes; el límite es { $max }.
live-send-no-password = No enviado: no hay contraseña guardada para { $name }. Ábrala una vez para escribirla.
live-checkin-sent = Publicado: { $entry }, en { $topic }.
live-share-sent = Publicado: { $name }, revisión { $rev }, en { $topic }.
live-checkin-not-sent = Registro no publicado: { $reason }
live-share-not-sent = { $name } no publicado: { $reason }
live-share-already = Ya se publica { $name } en { $topic }. Deje de publicar primero.
live-share-no-document = Abra un documento para publicarlo en vivo.
live-share-live-document = Un documento en vivo no se puede volver a publicar. Abra un archivo de documento para publicarlo.
live-share-through-title = ¿Publicar a través de qué fuente en vivo?
live-send-nowhere = No hay por dónde publicar. Inicie su broker o configure una fuente en vivo que pueda publicar.
live-share-none = No se publica ningún documento.
live-share-stopped = Se dejó de publicar { $name }. Los lectores conservan la revisión { $rev }.
live-checkin-through-title = ¿Publicar el registro a través de qué fuente en vivo?
live-checkin-to-broker = el broker de este equipo
live-checkin-intro = Registro para { $to }, en { $topic }. Tab cambia de campo, Intro publica, Escape cancela.
live-checkin-empty = Rellene al menos un campo.
live-checkin-cancelled = Cancelado. No se publicó ningún registro.
