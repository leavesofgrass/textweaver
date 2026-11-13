### The settings screen, profiles, units, sections, and settings files.

## Settings profiles.

profiles-title =
    { $n ->
        [0] Perfiles de configuración, ninguno guardado todavía
        [one] Perfiles de configuración, 1 perfil
       *[other] Perfiles de configuración, { $n } perfiles
    }
# $title is profiles-title.
profiles-intro = { $title }. Intro cambia a un perfil, F2 lo renombra, Suprimir lo elimina.
# $summary is profile-summary-* parts joined by commas.
profiles-item = { $name }: { $summary }
profiles-item-active = { $name }, en uso: { $summary }
profiles-save-new = Guardar la configuración actual como un perfil nuevo
profiles-update = Guardar la configuración actual en { $name }
profiles-import = Importar perfiles desde un archivo
profiles-export = Exportar todos los perfiles a un archivo
profile-summary-voice = voz { $voice }
profile-summary-rate = velocidad { $rate }
profile-summary-theme = tema { $theme }
# $mode is self voicing, hybrid, or screen reader.
profile-summary-access = modo { $mode }
profile-summary-empty = nada guardado
profile-switched = Se cambió a { $name }.
profile-switched-backend = Se cambió a { $name }. Su motor de voz se usará a partir del próximo inicio.
# $keys lists settings such as speech.pitch.
profile-dropped =
    { $n ->
        [one] 1 opción de este perfil no se usa en esta versión: { $keys }.
       *[other] { $n } opciones de este perfil no se usan en esta versión: { $keys }.
    }
profile-saved = Se guardó la configuración actual como { $name }.
profile-replaced = Se guardó la configuración actual en { $name }.
profile-renamed = Se renombró { $old } a { $new }.
profile-delete-question = ¿Eliminar el perfil { $name }? y o n
profile-deleted = Se eliminó { $name }.
profile-kept = Se conservó.
profile-not-found = No hay ningún perfil llamado { $name }.
profile-needs-name = Un perfil necesita un nombre.
profile-exists = Ya existe un perfil llamado { $name }.
profiles-not-an-export = { $detail }
profiles-no-persistence = Los perfiles no se guardan en esta sesión.
profiles-read-failed = No se pudo leer el archivo de perfiles, así que se trata como vacío: { $error } Guardar un perfil reemplaza el archivo.
profiles-save-failed = No se pudieron guardar los perfiles: { $error } Compruebe que se puede escribir en la carpeta de configuración.
profiles-none-to-export = Todavía no hay perfiles para exportar.
profiles-exported =
    { $n ->
        [one] Se exportó 1 perfil a { $file }.
       *[other] Se exportaron { $n } perfiles a { $file }.
    }
profiles-export-failed = No se pudieron exportar los perfiles: { $error } Compruebe que se puede escribir en la carpeta.
profiles-imported =
    { $n ->
        [0] No había perfiles en { $file }.
        [one] Se importó 1 perfil de { $file }: { $names }.
       *[other] Se importaron { $n } perfiles de { $file }: { $names }.
    }

## Exporting and importing settings.

# $n settings differ; $name is the file's name.
settingsio-import-question =
    { $n ->
        [one] ¿Importar { $n } opción cambiada de { $name }? y o n
       *[other] ¿Importar { $n } opciones cambiadas de { $name }? y o n
    }
settingsio-no-persistence = La configuración no se guarda en esta sesión, así que no se puede exportar ni importar.
# $path is the file written.
settingsio-exported = Configuración exportada a { $path }.
settingsio-export-failed = No se pudo exportar la configuración: { $error } Compruebe que se puede escribir en la carpeta.
# $path is the file; $error the system's reason.
settingsio-read-failed = No se pudo leer { $path }: { $error } Compruebe el archivo y vuelva a importar.
settingsio-nothing-to-import = Nada que importar: su configuración ya coincide con ese archivo.
settingsio-cancelled-unchanged = Cancelado. No se cambió nada.
settingsio-import-failed = No se pudo importar la configuración: { $error } Compruebe que se puede escribir en la carpeta de configuración.
# $summary lists what changed (from the settings store, in English).
settingsio-imported = Configuración importada. { $summary }
settingsio-backend-next-start = El nuevo motor de voz se usará a partir del próximo inicio.
settingsio-backed-up = Se hizo una copia de seguridad de la configuración anterior.

## The settings screen. $label is a setting-* label, $value its value as
## described below.

settings-not-set = sin definir
settings-none = ninguno
settings-empty = vacío
# A number and its unit (a settings-unit-* message): "300 words per minute".
settings-number-unit = { $n } { $unit }
settings-entries =
    { $n ->
        [0] ninguna
        [one] 1 entrada
       *[other] { $n } entradas
    }
settings-type-on-or-off = Escriba activado o desactivado.
settings-type-a-number = Escriba un número de { $min } a { $max }.
settings-outside = { $n } está fuera de { $min } a { $max }.
# $names are the choices, joined with commas.
settings-choose-one-of = Elija una de estas opciones: { $names }.
settings-edit-table = Edite { $label } en settings.toml; contiene nombres y valores.
# $path is a key such as speech.rate, not translated.
settings-no-such-setting = No existe la opción { $path }.
settings-cannot-be = { $label } no puede ser eso: { $error } Elija otro valor.
settings-changed = { $label }, { $value }.
settings-clamped = Fuera de rango, así que se usa el valor más cercano.
settings-restart-speech = Reinicie la voz para usarlo.
settings-next-start = Se usa a partir del próximo inicio.
settings-intro = Configuración, { $n } opciones. Escriba para filtrar. Izquierda y Derecha cambian un valor, Intro cambia o escribe uno, Suprimir restablece el valor predeterminado, Escape cierra.
settings-item = { $label }: { $value }
settings-title = Configuración
settings-title-matching = Configuración que coincide con { $filter }
settings-closed = Configuración cerrada.
settings-filter-cleared =
    { $n ->
        [one] Filtro borrado, 1 opción.
       *[other] Filtro borrado, { $n } opciones.
    }
settings-filter-none = Ninguna opción coincide con { $query }. Retroceso quita letras.
settings-filter-match =
    { $n ->
        [one] 1 opción coincide.
       *[other] { $n } opciones coinciden.
    }
settings-largest = Valor máximo, { $value }.
settings-smallest = Valor mínimo, { $value }.
settings-press-enter = { $label }: pulse Intro para escribir un valor nuevo.
settings-table-item = { $label }: { $value }. Edítelo en settings.toml.
# $help is the setting's help (setting-*-help), which may be empty.
settings-editing = { $label }, ahora { $value }. { $help }

## Units, said after a number.

settings-unit-words-per-minute =
    { $n ->
        [one] palabra por minuto
       *[other] palabras por minuto
    }
settings-unit-hertz =
    { $n ->
        [one] hercio
       *[other] hercios
    }
settings-unit-percent = por ciento
settings-unit-semitones =
    { $n ->
        [one] semitono
       *[other] semitonos
    }
settings-unit-milliseconds =
    { $n ->
        [one] milisegundo
       *[other] milisegundos
    }
settings-unit-words =
    { $n ->
        [one] palabra
       *[other] palabras
    }
settings-unit-places =
    { $n ->
        [one] lugar
       *[other] lugares
    }
settings-unit-columns =
    { $n ->
        [one] columna
       *[other] columnas
    }
settings-unit-groups =
    { $n ->
        [one] grupo
       *[other] grupos
    }
settings-unit-characters =
    { $n ->
        [one] carácter
       *[other] caracteres
    }
settings-unit-lines =
    { $n ->
        [one] línea
       *[other] líneas
    }
settings-unit-seconds =
    { $n ->
        [one] segundo
       *[other] segundos
    }
settings-unit-steps =
    { $n ->
        [one] paso
       *[other] pasos
    }
settings-unit-megabytes =
    { $n ->
        [one] megabyte
       *[other] megabytes
    }
settings-unit-files =
    { $n ->
        [one] archivo
       *[other] archivos
    }
settings-unit-letters =
    { $n ->
        [one] letra
       *[other] letras
    }
settings-unit-points =
    { $n ->
        [one] punto
       *[other] puntos
    }
settings-unit-rows =
    { $n ->
        [one] fila
       *[other] filas
    }
settings-unit-minutes =
    { $n ->
        [one] minuto
       *[other] minutos
    }

## Settings sections.

section-highlight = Resaltado
section-reading = Lectura
section-display = Pantalla
section-editing = Edición
section-library = Biblioteca y herramientas de estudio
section-keyboard = Teclado e idioma
section-accessibility = Accesibilidad y avisos
section-export = Exportar
section-braille = Braille

## Settings by task (beta 4): sections, and when a setting applies.

section-voice = Voz y habla
section-pauses = Pausas y verbosidad
section-speaking-text = Leer el texto
section-documents = Documentos
section-math = Matemáticas
section-morse = Código Morse
section-morse-practice = Práctica de Morse
section-spacing = Espaciado, fuente y regla
section-word-aids = Ayudas para palabras
section-rsvp = Palabra a palabra (RSVP)
section-window = Barras y paneles
# $n is the number of sections.
settings-intro-sections =
    { $n ->
        [one] Ajustes, 1 sección. Intro abre una sección. Escribe para buscar en todos los ajustes. Escape cierra.
       *[other] Ajustes, { $n } secciones. Intro abre una sección. Escribe para buscar en todos los ajustes. Escape cierra.
    }
# $section is a section-* title, $n its number of settings.
settings-section-row =
    { $n ->
        [one] { $section }, 1 ajuste
       *[other] { $section }, { $n } ajustes
    }
settings-all = Todos los ajustes
settings-all-row =
    { $n ->
        [one] Todos los ajustes, 1 ajuste
       *[other] Todos los ajustes, { $n } ajustes
    }
settings-title-section = Ajustes: { $section }
settings-section-opened =
    { $n ->
        [one] { $section }, 1 ajuste. Izquierda y Derecha cambian un valor, Intro cambia o escribe uno, Suprimir restaura el predeterminado, Escape vuelve a las secciones.
       *[other] { $section }, { $n } ajustes. Izquierda y Derecha cambian un valor, Intro cambia o escribe uno, Suprimir restaura el predeterminado, Escape vuelve a las secciones.
    }
settings-all-opened =
    { $n ->
        [one] Todos los ajustes, 1 ajuste, también los que no se aplican ahora. Escape vuelve a las secciones.
       *[other] Todos los ajustes, { $n } ajustes, también los que no se aplican ahora. Escape vuelve a las secciones.
    }
settings-back = Secciones de ajustes.
settings-section-now =
    { $n ->
        [one] { $section } ahora muestra 1 ajuste.
       *[other] { $section } ahora muestra { $n } ajustes.
    }
settings-section-help =
    { $n ->
        [one] { $section }, 1 ajuste. Intro la abre.
       *[other] { $section }, { $n } ajustes. Intro la abre.
    }
# $item is a settings-item row, $why a settings-applies-* sentence.
settings-item-not-now = { $item }. { $why }
# $engine, $first and $second are speech engine names, such as Eloquence.
settings-applies-engine = Se aplica cuando { $engine } es el motor de voz.
settings-applies-engines = Se aplica cuando { $first } o { $second } es el motor de voz.
settings-applies-not-engine = Se aplica cuando { $engine } no es el motor de voz.
# $os is Windows, macOS or Linux.
settings-applies-os = Solo se aplica en { $os }.
