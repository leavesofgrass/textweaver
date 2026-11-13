### Optional components and updates.

## Componentes opcionales.
component-feature-dictation = el dictado
component-feature-ocr = leer páginas escaneadas
component-feature-reading-font = una fuente de lectura
component-feature-voice = una voz
component-feature-audio-export = exportar M4B y MP4
component-feature-braille = archivos braille como tinta, braille contraído
component-feature-pandoc = más formatos de documento
component-state-installed = instalado
component-state-not-installed = no instalado
component-state-partial = instalado en parte
component-state-damaged = dañado
component-state-downloading = descargando
component-state-installer-ready = instalador descargado
components-title = Componentes opcionales
components-intro =
    { $n ->
        [one] 1 componente opcional. Intro para acciones.
       *[other] { $n } componentes opcionales. Intro para acciones.
    }
components-item = { $title }: { $state }, { $size }, licencia { $license }, para { $features }
components-actions-intro = { $title }: { $state }.
components-action-download = Descargar, { $size }
components-action-verify = Verificar los archivos
components-action-remove = Quitar
components-action-install-zip = Instalar desde un archivo zip…
components-action-install-folder = Instalar desde una carpeta…
components-install-purpose = Instalar el componente desde aquí
component-question = ¿Descargar { $title }, { $size }, licencia { $license }? y o n
component-remove-question = ¿Quitar { $title }? y o n
component-downloading = Descargando. Escape lo detiene.
component-installing = Instalando desde el archivo.
component-verifying = Comprobando los archivos.
component-progress = { $percent } por ciento descargado.
component-ready = Listo: { $title }.
component-verified = Archivos correctos: { $title }.
component-verify-failed =
    { $n ->
        [one] 1 archivo no es correcto: { $files }.
       *[other] { $n } archivos no son correctos: { $files }.
    }
component-removed = Quitado: { $title }.
component-refused =
    { $n ->
        [one] 1 archivo omitido: { $files }.
       *[other] { $n } archivos omitidos: { $files }.
    }
component-already = Ya instalado: { $title }.
component-not-there = No instalado: { $title }.
component-declined = No se descargó.
component-not-in-build = Sin descargas en esta versión.
component-no-folder = No hay carpeta de datos para él.
component-error-fetch = No descargado: falló el origen.
component-error-size = No instalado: tamaño incorrecto.
component-error-hash = No instalado: un archivo no coincidió. Descárguelo de nuevo o revise el origen.
component-error-missing = No instalado: falta un archivo.
component-error-cancelled = Descarga detenida; seguirá luego.
component-error-busy = Ya se está descargando uno.
component-error-no-source = No descargado: no tiene dirección.
component-error-name = Rechazado: un nombre no es simple.
component-error-manifest = Una lista de componentes no se puede leer.
component-error-io = No instalado: no se pudo escribir.
component-error-unpack = No instalado: el archivo comprimido no se pudo extraer.
component-launch-question = ¿Iniciar instalador: { $title }, versión { $version }, licencia { $license }? Después pregunta el sistema. y o n
component-version-unknown = no indicada
component-launched = Instalador iniciado: { $title }. Siga las indicaciones del sistema.
component-launch-failed = El instalador no se inició: { $reason }
component-launch-declined = No se inició; el instalador se conserva.
components-token-invalid = No es un token. Revíselo y péguelo de nuevo.
components-token-kept = Token guardado en el almacén de credenciales del sistema.
components-token-not-kept = Token usado, no guardado: { $reason }.
components-token-forgotten = Token olvidado.
components-token-none = No había ningún token guardado.
components-token-not-forgotten = No olvidado: { $reason }.
components-token-gh-still = Se sigue usando la sesión de GitHub CLI; gh auth logout la cierra.
components-chooser-title = Componentes opcionales
components-chooser-intro = Extras opcionales, ninguno elegido. Espacio elige uno; Descargar los elegidos los obtiene; Escape omite.
components-chooser-item = { $mark }: { $title }, para { $features }, { $size }, licencia { $license }
components-chosen = Elegido
components-not-chosen = No elegido
components-chooser-download = Descargar los elegidos
components-chooser-skip = Omitir por ahora
components-chooser-skipped = Omitido; vea Administrar componentes.
components-chooser-none = Nada elegido, nada descargado.
dictation-model-question = El dictado necesita el modelo Whisper, { $size }, licencia { $license }. ¿Descargarlo ahora? y o n
dictation-model-declined = Sin modelo, no hay dictado por ahora.
helper-question = { $need } necesita { $title }, { $size }, licencia { $license }. ¿Descargarlo ahora? y o n
helper-need-audio = Exportar M4B y MP4
helper-need-brf = Leer braille como tinta
helper-run-command = Para obtener { $title }, ejecute: { $command }
dictation-model-not-in-build = Sin modelo ni descargas en esta versión.
dictation-model-file-missing = Al modelo le falta { $file }.
dictation-model-damaged = Modelo de dictado dañado: { $file }.
dictation-model-no-folder = No existe la carpeta: { $dir }.

## Configuración de los componentes opcionales.

## Actualizaciones (B1-u1).
components-chooser-updates = { $mark }: Buscar actualizaciones automáticamente, una vez al día, en GitHub
update-checking = Buscando actualizaciones.
update-none = Sin actualización: { $version } es la más nueva.
update-found = Actualización disponible: textweaver { $version }, { $size }. ¿Descargarla? y o n
update-available-elsewhere = Actualización disponible: textweaver { $version }. Actualice esta copia como se instaló.
update-declined = No descargada. Se ofrecerá una versión más nueva.
update-downloading = Descargando la actualización, { $size }.
update-progress = Actualización: { $percent } por ciento descargado.
update-installed = Actualización verificada e instalada. Reinicie textweaver para usarla.
update-on-close = Actualización verificada. Se instala al cerrar textweaver.
update-busy = Ya se buscan actualizaciones.
update-not-in-build = Sin actualizaciones en esta versión.
update-automatic-question = ¿Buscar actualizaciones automáticamente, una vez al día? y o n
update-automatic-on = Actualizaciones: se buscan una vez al día.
update-automatic-off = Actualizaciones: no se buscan. Ayuda, Buscar actualizaciones busca en cualquier momento.
update-error-check = Falló la búsqueda de actualizaciones: { $reason }
update-error-no-package = Sin paquete de actualización para este equipo en { $version }.
update-error-no-checksum = Actualización rechazada: no tiene suma de comprobación. Nada cambió.
update-error-mismatch = Actualización rechazada: la suma no coincide. Nada cambió.
update-error-cancelled = Descarga detenida; sigue la próxima vez.
update-error-download = Actualización no descargada: { $reason }
update-error-install = Actualización no instalada: { $reason }. Nada cambió.
update-error-not-package = No actualizado: esta copia no viene de un paquete publicado.
section-updates = Actualizaciones y componentes
