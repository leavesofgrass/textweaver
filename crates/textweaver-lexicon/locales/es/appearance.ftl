### Themes, accessibility modes, and the Lexend font.

## Themes.

message-error = Error: { $message }
# $name is the theme name in the settings; $used the display name of the theme used instead.
themes-unknown = No hay ningún tema llamado { $name }; se usa { $used }.
# $theme is the new theme's display name.
themes-next = Tema { $theme }.
themes-soft-dark-name = { $theme }, oscuro suave
themes-choice-aa = { $theme }, cumple AA
themes-choice-below-aa = { $theme }, por debajo de AA
themes-below-aa =
    { $count ->
        [one] Por debajo de AA: 1 comprobación no alcanza el mínimo.
       *[other] Por debajo de AA: { $count } comprobaciones no alcanzan el mínimo.
    }

## Accessibility modes and the first-run step for a screen reader.

# Said when the accessibility mode changes; $mode is the new mode's id.
access-mode-changed =
    { $mode ->
        [self-voicing] Modo de voz propia. textweaver lo dice todo.
        [screen-reader] Modo lector de pantalla. textweaver está en silencio; su lector de pantalla lee la línea de estado.
       *[hybrid] Modo híbrido. textweaver lee los documentos en voz alta; su lector de pantalla dice los mensajes y lo que escribe.
    }
# $reader is the screen reader found (NVDA, JAWS), or access-a-screen-reader.
access-a-screen-reader = Un lector de pantalla
# On the first run with a screen reader and no mode chosen: hybrid mode is
# used, not asked. $reader is the screen reader found (NVDA, JAWS), or
# access-a-screen-reader; $key names the keys that change the mode.
access-hybrid-inferred = Modo híbrido: { $reader } dice los mensajes y { -brand } lee los documentos. { $key } lo cambia.
# The window's two modes, when the mode changes; $key changes it again.
access-window-choice-reads-aloud = textweaver lee en voz alta
access-window-choice-screen-reader = mi lector de pantalla lee
access-window-mode-help = Quién lee en la ventana: la voz de textweaver o solo su lector de pantalla.
access-window-mode-changed =
    { $mode ->
        [screen-reader] Mi lector de pantalla lee: textweaver calla y envía el texto a su lector de pantalla. { $key } lo cambia.
        [speaks-messages] textweaver lee en voz alta y dice sus mensajes. { $key } lo cambia.
       *[reads-aloud] textweaver lee en voz alta; los mensajes van a su lector de pantalla. { $key } lo cambia.
    }

## Lexend downloaded on first choice.
font-download-question = ¿Descargar la fuente { $font }, { $kb } KB, { $licence }? y o n
font-downloading = Descargando { $font }.
font-downloaded = { $font } descargada y lista.
font-download-failed = { $font } no se descargó: { $error } Se usa otra fuente.
font-download-declined = No se descargó. Se usa otra fuente.
font-download-busy = { $font } aún se está descargando.
font-download-no-folder = No hay carpeta de datos para { $font }.
font-download-not-in-build = Esta versión no descarga fuentes.
gui-font-to-download = { $family } (descargar, { $kb } KB)
gui-font-downloaded = { $family } (descargada)
