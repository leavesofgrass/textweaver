### Los mensajes de la interfaz de textweaver, en español.
###
### Fluent (https://projectfluent.org/), en el subconjunto que lee
### textweaver-lexicon. Los identificadores y las variables son los de
### en.ftl; un mensaje que falte aquí se dice en inglés.

-brand = textweaver

## Parts of speech.

pos-noun = sustantivo
pos-verb = verbo
pos-adjective = adjetivo
pos-adverb = adverbio

## Define word: sources.

source-glossary = su glosario
source-wordnet = Open English WordNet
source-cmudict = el diccionario de pronunciación CMU

## Define word: the list of senses.

# $word is the word looked up, $n the number of senses, $source a source-* message.
define-title =
    { $n ->
        [0] Pronunciación de { $word }, de { $source }
        [one] Definiciones de { $word }, 1 acepción, de { $source }
       *[other] Definiciones de { $word }, { $n } acepciones, de { $source }
    }
# $lemma is the headword, $pos a pos-* message, $i the sense number, $n how many.
define-sense-head = { $lemma }, { $pos }, { $i } de { $n }
define-sense-head-nopos = { $lemma }, { $i } de { $n }
define-sense = { $head }: { $definition }.
define-example = Por ejemplo: { $text }.
define-synonyms = Sinónimos: { $words }.
define-antonyms = Antónimos: { $words }.
define-kind-of = Un tipo de: { $words }.
# $say is a respelling such as RUN-ing, with the stressed syllable in capitals.
define-pronounced = Se pronuncia { $say }.
define-pronounced-or = Se pronuncia { $say }, o { $other }.

## Prompts.

prompt-define-word = ¿Definir qué palabra?
prompt-profile-name = Nombre del perfil nuevo
prompt-profile-rename = Nuevo nombre del perfil, Intro lo conserva
prompt-profiles-import = Importar perfiles desde un archivo
prompt-profiles-export = Exportar perfiles a un archivo, por ejemplo textweaver-profiles.json

## Common words.

common-cancelled = Cancelado.
# Durations: $h hours, $m minutes, $s seconds.
duration-hours =
    { $h ->
        [one] 1 hora
       *[other] { $h } horas
    } y { $m ->
        [one] 1 minuto
       *[other] { $m } minutos
    }
duration-minutes =
    { $m ->
        [one] 1 minuto
       *[other] { $m } minutos
    } y { $s ->
        [one] 1 segundo
       *[other] { $s } segundos
    }
duration-seconds =
    { $s ->
        [one] 1 segundo
       *[other] { $s } segundos
    }

## Define word in the reader.

# $title is define-title.
define-intro = { $title }. Arriba y Abajo recorren las acepciones, Intro copia una, Escape cierra.
define-nothing-here = No hay ninguna palabra en el cursor.
define-not-found = No se encontró ninguna definición de { $word }.
define-no-dictionary = El archivo del diccionario no está instalado, así que solo se buscó en su glosario. La guía de lectura explica cómo instalarlo.
define-dictionary-damaged = No se pudo leer el archivo del diccionario: { $error }
define-glossary-problem = No se pudo leer su glosario: { $error }
define-glossary-skipped =
    { $n ->
        [one] 1 línea de su glosario no tiene definición y se omitió.
       *[other] { $n } líneas de su glosario no tienen definición y se omitieron.
    }
define-copied = Copiado.

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
profiles-read-failed = No se pudo leer el archivo de perfiles, así que se trata como vacío: { $error }
profiles-save-failed = No se pudieron guardar los perfiles: { $error }
profiles-none-to-export = Todavía no hay perfiles para exportar.
profiles-exported =
    { $n ->
        [one] Se exportó 1 perfil a { $file }.
       *[other] Se exportaron { $n } perfiles a { $file }.
    }
profiles-export-failed = No se pudieron exportar los perfiles: { $error }
profiles-imported =
    { $n ->
        [0] No había perfiles en { $file }.
        [one] Se importó 1 perfil de { $file }: { $names }.
       *[other] Se importaron { $n } perfiles de { $file }: { $names }.
    }

## Reading statistics.

stats-title = Estadísticas de lectura
stats-intro = Estadísticas de lectura. Intro en un documento lo abre.
stats-off = Las estadísticas de lectura están desactivadas. El último elemento las activa.
stats-empty = Todavía no se ha registrado ninguna lectura. El tiempo se cuenta mientras textweaver lee en voz alta.
# $time is a duration-* message.
stats-total =
    { $time } leído en total, en { $sessions ->
        [one] 1 sesión
       *[other] { $sessions } sesiones
    }, en { $docs ->
        [one] 1 documento
       *[other] { $docs } documentos
    }.
stats-current =
    Este documento: { $time } leído, punto más lejano { $pct } por ciento, { $sessions ->
        [one] 1 sesión
       *[other] { $sessions } sesiones
    }.
stats-current-none = Este documento todavía no se ha leído en voz alta.
stats-most-read = Más leído { $rank }: { $title }, { $time }, punto más lejano { $pct } por ciento.
stats-toggle-on = Las estadísticas están activadas. Intro las desactiva.
stats-toggle-off = Las estadísticas están desactivadas. Intro las activa.
stats-turned-on = Las estadísticas de lectura están activadas.
stats-turned-off = Las estadísticas de lectura están desactivadas. Lo registrado se conserva; tw stats --clear lo elimina.

## Lists.

study-nothing-to-delete = Nada que eliminar en esta lista.
study-nothing-to-rename = Nada que renombrar en esta lista.
## tw stats.

stats-clear-question =
    { $n ->
        [one] ¿Eliminar las estadísticas de lectura de 1 documento? y o n
       *[other] ¿Eliminar las estadísticas de lectura de { $n } documentos? y o n
    }
stats-cleared = Estadísticas de lectura eliminadas.
stats-off-cli = Las estadísticas de lectura están desactivadas: stats.enabled es false en la configuración.

## Navigation. $dir is next or previous; $what is a kind-* or unit-*
## noun and $unit its key (heading, list-item, sentence), for languages
## whose words agree with the noun.

nav-blank = en blanco
# High verbosity: $label is a structure label ("Heading level 2").
nav-message-at-labelled = { $label }, línea { $line }, { $pct } por ciento: { $content }
nav-message-at = Línea { $line }, { $pct } por ciento: { $content }
nav-message-labelled = { $label }: { $content }
# $message is the navigation message after wrapping around.
nav-wrapped = Ha dado la vuelta. { $message }
nav-no-next = No hay { $what } siguiente.
nav-no-previous = No hay { $what } anterior.
nav-nothing-to-read = No hay { $what } que leer.
nav-label-heading-level = Encabezado de nivel { $level }
nav-label-list =
    { $n ->
        [0] Lista
        [one] Lista, 1 elemento
       *[other] Lista, { $n } elementos
    }
nav-label-table =
    { $n ->
        [0] Tabla
        [one] Tabla, 1 fila
       *[other] Tabla, { $n } filas
    }
nav-label-list-item-level = Elemento de lista, nivel { $level }
nav-no-heading-level =
    { $dir ->
        [next] No hay encabezado siguiente de nivel { $level }.
       *[previous] No hay encabezado anterior de nivel { $level }.
    }
# Said before a line's text on caret moves.
nav-line-heading = encabezado de nivel { $level }
nav-line-row = fila { $n }
nav-line-list-item-level = elemento de lista, nivel { $level }
# $language is a programming language name such as Python.
nav-line-code-language = código, { $language }
nav-no-chapters = Este documento no tiene capítulos.
nav-chapter = Capítulo
nav-no-chapter =
    { $dir ->
        [next] No hay capítulo siguiente.
       *[previous] No hay capítulo anterior.
    }
nav-back = Atrás
nav-forward = Adelante
nav-page = Página
nav-percent = { $pct } por ciento
nav-no-earlier-history = No hay historial anterior.
nav-no-forward-history = No hay historial hacia adelante.
# $label is nav-back, nav-forward, nav-page, or nav-percent.
nav-label-line = { $label }, línea { $line }
nav-top-of-document = Principio del documento
nav-end-of-document = Final del documento
# $edge is nav-top-of-document or nav-end-of-document; $content the line there.
nav-edge-message = { $edge }. { $content }
nav-top-of-document-stop = Principio del documento.
nav-end-of-document-stop = Final del documento.
nav-position = Línea { $line } de { $lines }, { $pct } por ciento.
nav-position-word = Palabra { $word } de { $words }.
nav-position-heading = Bajo el encabezado { $heading }.
nav-position-mode = Modo { $mode }.

## Names of structure, units, and modes (said inside other messages).

kind-heading = encabezado
kind-paragraph = párrafo
kind-list-item = elemento de lista
kind-list = lista
kind-table = tabla
kind-row = fila
kind-cell = celda
kind-link = enlace
kind-graphic = gráfico
kind-code = código
kind-quote = cita en bloque
kind-page = página
kind-section = sección
kind-bold = negrita
kind-italic = cursiva
kind-underline = subrayado
kind-footnote = nota al pie
kind-strikethrough = tachado
kind-separator = separador
kind-math = matemáticas
unit-character = carácter
unit-word = palabra
unit-sentence = oración
unit-line = línea
unit-paragraph = párrafo
unit-document = documento
# $what is a kind-* noun, $unit its key.
unit-with-level = { $what } de nivel { $level }
mode-browse = Exploración
mode-speech-cursor = Cursor de lectura
mode-edit = Editar
mode-find = Buscar
mode-command = Comando
mode-go-to = Ir a
mode-open = Abrir
mode-prompt = Preguntar
common-on = activado
common-off = desactivado

## Reading aloud.

playback-caps-no-words = Esta voz no informa palabras, así que el resaltado de palabras es aproximado.
playback-caps-words = Esta voz informa cada palabra, así que el resaltado la sigue con exactitud.
playback-caps-no-pitch = El tono no se puede cambiar con esta voz.
playback-caps-no-volume = El volumen no se puede cambiar con esta voz.
# The reading state on the title line, one word.
state-reading = Lectura
state-paused = En pausa
state-stopped = Detenido
state-ready = Listo
playback-reading-at = Leyendo a { $rate } palabras por minuto.
playback-paused = En pausa.
playback-stopped-speech-cursor-off = Detenido. Cursor de lectura desactivado.
playback-stopped = Detenido.
playback-search-cleared = Búsqueda borrada.
# $key is the key that turns edit mode off.
playback-still-editing = Aún editando. { $key } termina.
playback-end-of-document-content = final del documento
playback-no-unit-here = No hay { $what } aquí.
playback-no-selection = No hay selección.
# $next is what happens now (a restart, or nothing).
playback-speech-died = La voz dejó de funcionar ({ $reason }). { $next }
playback-done-reading = Lectura terminada.
playback-speech-restarted = Voz reiniciada: { $reason }. Continúa la lectura desde la última palabra.
playback-speech-error = Error de voz: { $error }

## The title line and Say Status.

# The title line's position; % is shown, not said.
status-position = línea { $line } de { $lines }, { $pct }%
status-mode = Modo { $mode }
status-modified = modificado
status-self-voicing = voz propia
status-hybrid = híbrido
status-screen-reader = modo lector de pantalla
status-rate-spoken = { $wpm } palabras por minuto
status-rate = { $wpm } ppm
status-no-document = Sin documento
# $parts are the title line's parts, joined with commas.
status-said = { $title }: { $parts }.
status-no-message = Todavía no hay ningún mensaje.
# A list shown without its own introduction.
status-list-intro =
    { $n ->
        [one] { $title }, 1 elemento. Arriba y Abajo se mueven, Intro elige, Escape cierra.
       *[other] { $title }, { $n } elementos. Arriba y Abajo se mueven, Intro elige, Escape cierra.
    }

## Questions and answers. Keep the letters y and n: they are the keys
## that answer.

common-press-y-or-n = Pulse y o n.
common-kept = Se conservó.
confirm-quit = ¿Salir de textweaver? y o n
confirm-delete-note = ¿Eliminar esta nota o resaltado? y o n
notes-remove-highlight-question = ¿Quitar este resaltado? y o n
notes-delete-note-question = ¿Eliminar esta nota? y o n
list-nothing-to-mark = Nada que marcar en esta lista.
notes-editing = Editando la nota: { $text }
# $key opens a document.
app-no-document-open = No hay ningún documento abierto. Pulse { $key } para abrir uno.
settings-save-failed = No se pudo guardar la configuración: { $error }
edit-still-editing = Aún editando.
goto-not-a-target = No es un destino válido: { $text }. Escriba un número de línea, un porcentaje como 50%, inicio o fin.

## Opening a document.

open-opened = Se abrió { $title }.
open-resumed = Se abrió { $title }. Se reanudó en el { $pct } por ciento.
open-resumed-synced = Se abrió { $title }. Se reanudó en el { $pct } por ciento, desde otro dispositivo.
open-resumed-conflict = Se abrió { $title }. Se reanudó en el { $pct } por ciento. Otro dispositivo está en un punto diferente; se conservó el de este dispositivo.

## Prompts: the label is shown and said when the prompt opens.

prompt-find = Buscar
prompt-go-to = Ir a línea, porcentaje, inicio o fin
prompt-open = Abrir archivo
prompt-command = Comando
# $label is prompt-command.
prompt-command-palette-intro = { $label }. Escriba parte de un nombre; Tab completa, Arriba y Abajo recorren las coincidencias.
prompt-save-as = Guardar como
prompt-table-size = Tamaño de la tabla, columnas por filas, por ejemplo 3 por 2
prompt-image-path = Archivo de imagen
prompt-replace-find = Reemplazar, buscar qué
prompt-replace-with = Reemplazar con
prompt-note = Nota
prompt-edit-note = Editar nota, Intro la conserva
prompt-rename-bookmark = Nuevo nombre del marcador, Intro lo conserva
prompt-export-settings = Exportar la configuración a un archivo, por ejemplo textweaver-settings.json
prompt-import-settings = Importar configuración desde un archivo
prompt-citation-locator = Página u otro localizador, por ejemplo 12 o capítulo 2; Intro para ninguno
prompt-reference-identifier = DOI o ISBN a agregar
prompt-import-references = Importar referencias desde un archivo
prompt-template-title = Título del documento nuevo
prompt-setting-value = Valor nuevo, Intro lo conserva

## Keys named in messages and the help.

help-the-command-palette = la paleta de comandos
help-not-bound = sin asignar
# Two keys, or a key and a list of keys: "p or Ctrl+P".
help-or = { $a } o { $b }
# A command without keys: $name is its palette name, such as list highlights.
help-the-command = el comando { $name }
# One line of the keyboard shortcuts list: a category-* title, an action-*
# help, and its keys.
help-entry = { $category }: { $help }. { $keys }
# One command palette candidate: its id (not translated), help, and keys.
help-palette-item = { $id }: { $help }. { $keys }
help-unknown-command = Comando desconocido: { $text }.
help-shortcuts-intro = Atajos de teclado, { $n } comandos. Arriba y Abajo se mueven, Intro ejecuta, Escape cierra.
help-shortcuts-title = Atajos de teclado
help-title = Ayuda
help-intro = Ayuda. Arriba y Abajo se mueven, Escape cierra.

## The help list. Each value is a key or keys from the keymap.

help-about = textweaver lee documentos en voz alta. Las teclas de abajo son los atajos actuales.
help-open = Abrir un documento: { $open }. Biblioteca y archivos recientes: { $library }.
help-play = Reproducir o pausar: { $key }.
help-read-from-cursor = Leer desde el cursor: { $key }.
help-stop = Detener: { $key }.
help-sentences = Oración siguiente y anterior: { $next } y { $previous }.
help-paragraphs = Párrafo siguiente y anterior: { $next } y { $previous }.
help-headings = Encabezado siguiente y anterior: { $next } y { $previous }. Encabezado de un nivel: de { $first } a { $last }, con Mayús para el anterior.
help-read-headings = Leer desde el encabezado siguiente y anterior: { $next } y { $previous }.
help-quick-keys = Teclas rápidas, como en NVDA y JAWS: lista { $list }, elemento de lista { $item }, tabla { $table }, enlace { $link }, cita en bloque { $quote }, separador { $separator }, gráfico { $graphic }, sección { $section }. Mayús con la tecla va al anterior.
help-speech-cursor = Cursor de lectura, línea por línea: { $key }.
help-find = Buscar: { $key }.
help-bookmark = Agregar un marcador: { $key }.
help-history = Atrás y adelante por sus saltos: { $back } y { $forward }.
help-rate = Más rápido y más lento: { $faster } y { $slower }.
help-where = Dónde estoy: { $key }.
help-repeat = Repetir el último mensaje: { $repeat }. El último mensaje y el estado: modo, velocidad, motor y posición: { $status }.
help-notes = Notas: agregar { $add }, listar { $list }, siguiente y anterior { $next } y { $previous }, eliminar la del cursor { $delete }. En la lista, Suprimir elimina y F2 edita.
help-highlights = Resaltar la selección o la oración, o quitar un resaltado: { $highlight }. Listar resaltados: { $list }.
help-bookmarks-list = Lista de marcadores: Suprimir elimina un marcador, F2 lo renombra.
help-edit = Editar el documento: { $edit }. Guardar: { $save }. Guardar como: { $saveas }. Documento nuevo: { $new }.
help-editing = Mientras edita: deshacer { $undo }, rehacer { $redo }, negrita { $bold }. Todos los comandos de formato están en los atajos de teclado.
help-outline = Esquema de los encabezados, escriba para filtrar: { $outline }. Seguir un enlace o nota al pie: { $follow }.
help-tables = Tablas: { $nextrow } y { $previousrow } se mueven por fila, { $nextcell } y { $previouscell } por celda.
help-citations = Citas mientras edita: insertar { $insert }, agregar una referencia por DOI o ISBN { $reference }. Ortografía: falta siguiente y anterior { $next } y { $previous }, sugerencias { $suggestions }.
help-export = Exportar a HTML, PDF, Word, EPUB o braille, ver la vista previa en el navegador y empezar desde una plantilla: escriba export, preview o template en la paleta de comandos.
help-verbosity = Cuánto se dice: { $verbosity }. Cuánta puntuación: { $punctuation }.
help-voice = Elegir una voz: { $voice }. Reiniciar la voz si deja de funcionar: { $restart }.
help-access = Con un lector de pantalla, quién habla: { $key } recorre voz propia, híbrido y modo lector de pantalla.
help-character-keys = Atajos de una sola tecla activados o desactivados, para dictado: { $keys }. Configuración: { $settings }.
help-all-shortcuts = Todos los atajos de teclado: { $key }.
help-palette = Ejecutar cualquier comando por su nombre: { $key }.
# Keep the letters y, n, and a: they are the keys that answer.
help-quit = Salir, guardando su posición: { $key }, luego y para confirmar; n, a o Escape cancela.

## Help categories.

category-reading = Lectura
category-navigation = Navegación
category-speech-cursor = Cursor de lectura
category-voice = Voz
category-search = Buscar
category-bookmarks = Marcadores y notas
category-file = Archivo
category-editing = Edición
category-view = Vista y ayuda

## Key names as textweaver's own voice says them. Written names (Ctrl+S)
## are not translated.

keyname-control = Control
keyname-command = Comando
keyname-alt = Alt
keyname-shift = Mayús
keyname-period = punto
keyname-comma = coma
keyname-semicolon = punto y coma
keyname-colon = dos puntos
keyname-apostrophe = apóstrofo
keyname-quote = comilla
keyname-grave-accent = acento grave
keyname-tilde = virgulilla
keyname-exclamation-mark = signo de exclamación
keyname-question-mark = signo de interrogación
keyname-at-sign = arroba
keyname-number-sign = almohadilla
keyname-dollar-sign = signo de dólar
keyname-percent = por ciento
keyname-caret = circunflejo
keyname-ampersand = ampersand
keyname-asterisk = asterisco
keyname-left-parenthesis = paréntesis de apertura
keyname-right-parenthesis = paréntesis de cierre
keyname-left-bracket = corchete de apertura
keyname-right-bracket = corchete de cierre
keyname-left-brace = llave de apertura
keyname-right-brace = llave de cierre
keyname-less-than = menor que
keyname-greater-than = mayor que
keyname-plus = más
keyname-minus = menos
keyname-equals = igual
keyname-underscore = guion bajo
keyname-slash = barra
keyname-backslash = barra invertida
keyname-vertical-bar = barra vertical
keyname-page-up = Re Pág
keyname-page-down = Av Pág
keyname-up-arrow = Flecha arriba
keyname-down-arrow = Flecha abajo
keyname-left-arrow = Flecha izquierda
keyname-right-arrow = Flecha derecha
keyname-escape = Escape
keyname-space = Espacio
keyname-enter = Intro
keyname-tab = Tab
keyname-backspace = Retroceso
keyname-delete = Suprimir
keyname-insert = Insertar
keyname-home = Inicio
keyname-end = Fin

## Commands: the one-line help of each, in the keyboard shortcuts list and
## the command palette. Their ids (play_pause) stay as they are.

action-play-pause = Reproducir o pausar la lectura desde la palabra actual
action-stop = Detener la lectura
action-read-from-cursor = Leer de forma continua desde el cursor
action-read-document = Leer todo el documento desde el principio
action-read-current-character = Decir el carácter en el cursor
action-read-current-word = Decir la palabra en el cursor
action-read-current-sentence = Decir la oración en el cursor sin moverse
action-read-current-line = Decir la línea en el cursor
action-read-paragraph = Decir el párrafo en el cursor sin moverse
action-read-selection = Leer el texto seleccionado
action-say-position = Decir la posición: línea, porcentaje, número de palabra y encabezado
action-say-status = Repetir el último mensaje y luego el estado: modo, estado de lectura, posición, velocidad y motor de voz; en una lista, la introducción de la lista
action-repeat-message = Repetir el último mensaje
action-word-count = Decir cuántas palabras tiene el documento, o la selección
action-link-address = Decir la dirección del enlace en el cursor
action-replay-sentence = Volver a leer desde el principio de la oración actual
action-replay-paragraph = Volver a leer desde el principio del párrafo actual
action-rsvp-toggle = Mostrar u ocultar RSVP: una palabra a la vez, desde el cursor
action-rsvp-play-pause = Iniciar o pausar RSVP
action-rsvp-faster = RSVP más rápido
action-rsvp-slower = RSVP más lento
action-rsvp-position-next = Mover la palabra de RSVP al siguiente lugar de la pantalla
action-reading-level = Decir el nivel de lectura del documento o de la selección
action-define-word = Definir la palabra en el cursor, o las palabras seleccionadas: acepciones, ejemplos, sinónimos y pronunciación
action-toggle-citations = Activar o desactivar las citas en la lectura continua: desactivado las omite, activado las dice en palabras
action-explore-math = Explorar las matemáticas en el cursor, término por término: las flechas mueven, Abajo entra en una parte, Arriba sale, Escape termina
action-listen-rendered = Escuchar el documento tal como se representará, sin salir del modo de edición
action-next-sentence = Ir a la oración siguiente
action-previous-sentence = Ir a la oración anterior, o al principio de esta si ha avanzado más de tres palabras
action-next-paragraph = Ir al párrafo siguiente
action-previous-paragraph = Ir al párrafo anterior
action-next-heading = Leer desde el encabezado siguiente
action-previous-heading = Leer desde el encabezado anterior
action-skip-next-heading = Ir al encabezado siguiente sin leer
action-skip-previous-heading = Ir al encabezado anterior sin leer
action-outline = Listar los encabezados: escriba para filtrar, Intro va a uno
action-next-heading-level-1 = Ir al encabezado siguiente de nivel 1
action-next-heading-level-2 = Ir al encabezado siguiente de nivel 2
action-next-heading-level-3 = Ir al encabezado siguiente de nivel 3
action-next-heading-level-4 = Ir al encabezado siguiente de nivel 4
action-next-heading-level-5 = Ir al encabezado siguiente de nivel 5
action-next-heading-level-6 = Ir al encabezado siguiente de nivel 6
action-previous-heading-level-1 = Ir al encabezado anterior de nivel 1
action-previous-heading-level-2 = Ir al encabezado anterior de nivel 2
action-previous-heading-level-3 = Ir al encabezado anterior de nivel 3
action-previous-heading-level-4 = Ir al encabezado anterior de nivel 4
action-previous-heading-level-5 = Ir al encabezado anterior de nivel 5
action-previous-heading-level-6 = Ir al encabezado anterior de nivel 6
action-next-table = Ir a la tabla siguiente
action-previous-table = Ir a la tabla anterior
action-next-list = Ir a la lista siguiente
action-previous-list = Ir a la lista anterior
action-next-list-item = Ir al elemento de lista siguiente
action-previous-list-item = Ir al elemento de lista anterior
action-next-link = Ir al enlace siguiente
action-previous-link = Ir al enlace anterior
action-next-block-quote = Ir a la cita en bloque siguiente
action-previous-block-quote = Ir a la cita en bloque anterior
action-next-separator = Ir al separador siguiente (línea horizontal)
action-previous-separator = Ir al separador anterior (línea horizontal)
action-next-graphic = Ir al gráfico siguiente (imagen)
action-previous-graphic = Ir al gráfico anterior (imagen)
action-follow-link = Seguir el enlace en el cursor, o ir entre una nota al pie y su nota
action-table-next-row = En una tabla, bajar una fila en la misma columna
action-table-previous-row = En una tabla, subir una fila en la misma columna
action-table-next-column = En una tabla, ir a la celda siguiente de la fila
action-table-previous-column = En una tabla, ir a la celda anterior de la fila
action-next-chapter = Ir al capítulo o sección siguiente
action-previous-chapter = Ir al capítulo o sección anterior
action-history-back = Volver a donde estaba antes del último salto
action-history-forward = Avanzar de nuevo tras volver atrás
action-go-to = Ir a una línea, porcentaje o posición
action-document-start = Ir al principio del documento
action-document-end = Ir al final del documento
action-caret-next-word = Mover el cursor a la palabra siguiente
action-caret-previous-word = Mover el cursor a la palabra anterior
action-caret-next-line = Mover el cursor a la línea siguiente
action-caret-previous-line = Mover el cursor a la línea anterior
action-select-next-word = Extender la selección a la palabra siguiente
action-select-previous-word = Extender la selección a la palabra anterior
action-select-next-line = Extender la selección a la línea siguiente
action-select-previous-line = Extender la selección a la línea anterior
action-page-down = Bajar una pantalla
action-page-up = Subir una pantalla
action-scroll-down = Desplazar hacia abajo una línea sin mover el cursor
action-scroll-up = Desplazar hacia arriba una línea sin mover el cursor
action-speech-cursor-toggle = Entrar o salir del modo Cursor de lectura (línea)
action-speech-cursor-next-line = Cursor de lectura: leer la línea siguiente
action-speech-cursor-previous-line = Cursor de lectura: leer la línea anterior
action-speech-cursor-reread-line = Cursor de lectura: volver a leer la línea actual
action-speech-cursor-exit-and-read = Cursor de lectura: salir y seguir leyendo desde esta línea
action-rate-up = Hablar más rápido
action-rate-down = Hablar más lento
action-pitch-up = Subir el tono
action-pitch-down = Bajar el tono
action-volume-up = Más alto
action-volume-down = Más bajo
action-cycle-speed-preset = Recorrer las velocidades preestablecidas (rápida, normal, estudio, lenta)
action-choose-voice = Elegir una voz
action-restart-speech = Reiniciar la voz con la configuración actual (después de que el motor de voz dejara de funcionar)
action-cycle-verbosity = Recorrer cuánto dice textweaver: bajo, normal, alto
action-cycle-punctuation = Recorrer cuánta puntuación se dice: ninguna, algo, toda
action-find = Buscar texto en el documento
action-find-next = Buscar la coincidencia siguiente
action-find-previous = Buscar la coincidencia anterior
action-next-misspelling = Ir a la palabra mal escrita siguiente, y deletrearla
action-previous-misspelling = Ir a la palabra mal escrita anterior, y deletrearla
action-spelling-suggestions = Listar sugerencias para la palabra mal escrita en el cursor, o agregarla a su lista de palabras
action-next-grammar-problem = Ir al problema de gramática siguiente, y decirlo junto con su corrección
action-previous-grammar-problem = Ir al problema de gramática anterior, y decirlo junto con su corrección
action-next-lint-problem = En modo de edición, ir al problema de estilo Markdown siguiente, y decirlo
action-previous-lint-problem = En modo de edición, ir al problema de estilo Markdown anterior, y decirlo
action-add-bookmark = Agregar un marcador en el cursor
action-list-bookmarks = Listar marcadores
action-next-bookmark = Ir al marcador siguiente
action-previous-bookmark = Ir al marcador anterior
action-add-note = Agregar una nota a la selección o a la oración en el cursor
action-list-notes = Listar notas
action-next-note = Ir a la nota siguiente
action-previous-note = Ir a la nota anterior
action-delete-note = Eliminar la nota o resaltado en el cursor
action-highlight-selection = Resaltar la selección, o la oración en el cursor
action-export-study-sheet = Exportar las notas y resaltados como una hoja de estudio en Markdown, agrupados por encabezado
action-open = Abrir un documento
action-open-path = Abrir un documento escribiendo su ruta
action-open-library = Abrir la biblioteca: documentos de sus carpetas de biblioteca y archivos recientes
action-new-document = Empezar un documento nuevo en modo de edición
action-save = Guardar (Markdown y texto en su lugar; otros formatos como Markdown)
action-save-as = Guardar con otro nombre
action-export-settings = Exportar la configuración y las teclas personalizadas a un archivo JSON o TOML
action-import-settings = Importar la configuración desde un archivo JSON o TOML, tras un sí o no
action-reading-statistics = Listar las estadísticas de lectura: tiempo leído, punto más lejano, sesiones y los documentos más leídos
action-new-from-template = Empezar un documento nuevo a partir de una plantilla, con título, autor, fecha y un encabezado de Referencias
action-export-html = Exportar el documento como página web (HTML) junto a él
action-export-pdf = Exportar el documento como PDF etiquetado junto a él
action-export-docx = Exportar el documento como archivo de Word (DOCX) junto a él
action-export-epub = Exportar el documento como libro EPUB junto a él
action-export-brf = Exportar el documento como braille (BRF) junto a él
action-preview-in-browser = Ver la vista previa del documento en el navegador web, con matemáticas; cada guardado la reescribe
action-toggle-preview-auto-reload = Activar o desactivar la recarga automática de la vista previa en el navegador
action-toggle-preview-live = Activar o desactivar la vista previa en vivo: con la recarga automática, la vista previa también se recarga al hacer una pausa al escribir
action-quit = Salir, guardando la posición de lectura
action-toggle-edit-mode = Cambiar entre lectura y edición
action-undo = Deshacer
action-redo = Rehacer
action-bold = Poner en negrita la selección
action-italic = Poner en cursiva la selección
action-underline = Subrayar la selección
action-strikethrough = Tachar la selección
action-inline-code = Marcar la selección como código
action-code-block = Convertir las líneas seleccionadas en un bloque de código
action-insert-link = Convertir la selección en un enlace
action-heading = Convertir la línea actual en un encabezado
action-bullet-list = Convertir las líneas seleccionadas en una lista con viñetas
action-numbered-list = Convertir las líneas seleccionadas en una lista numerada
action-block-quote = Convertir las líneas seleccionadas en una cita en bloque
action-horizontal-rule = Insertar una línea horizontal
action-insert-table = Insertar una tabla
action-add-table-row = Agregar una fila a la tabla en el cursor
action-insert-image = Insertar una imagen
action-replace = Buscar y reemplazar
action-copy = Copiar la selección, o la oración en el cursor, al portapapeles
action-cut = Cortar la selección al portapapeles
action-next-table-cell = En una tabla, ir a la celda siguiente y decir su columna; fuera de una tabla, escribe un tabulador
action-previous-table-cell = En una tabla, ir a la celda anterior y decir su columna
action-cycle-typing-echo = Recorrer el eco de escritura: caracteres y palabras, caracteres, palabras, o ninguno
action-select-all = Seleccionar todo el texto
action-delete-word-before = Eliminar la palabra anterior al cursor
action-delete-word-after = Eliminar la palabra posterior al cursor
action-paste = Pegar el texto copiado o cortado por última vez en textweaver; el pegado del terminal también funciona
action-insert-citation = Insertar una cita: elegir una referencia y luego indicar una página u otro localizador
action-add-reference = Agregar una referencia a su biblioteca por DOI o ISBN
action-insert-bibliography = Insertar la bibliografía de las obras citadas, en el cursor
action-check-citations = Comprobar las citas: cuántas hay, y qué claves no están en su biblioteca
action-import-references = Importar referencias desde un archivo BibTeX, RIS o CSL-JSON a su biblioteca
action-next-theme = Cambiar al tema de color siguiente
action-toggle-line-numbers = Mostrar u ocultar los números de línea
action-toggle-character-keys = Activar o desactivar los atajos de una sola tecla, para que el dictado y la escritura nunca activen comandos
action-cycle-access-mode = Recorrer el modo de accesibilidad: voz propia, híbrido o lector de pantalla
action-settings-profiles = Listar los perfiles de configuración: cambiar a uno, guardar la configuración actual como uno, renombrar, eliminar, importar o exportar
action-bionic-toggle = Activar o desactivar la lectura biónica: el principio de cada palabra en negrita
action-ruler-cycle = Recorrer la regla de lectura: desactivada, línea actual, regla
action-syllables-toggle = Mostrar u ocultar las sílabas: palabras separadas con un punto central
action-difficult-words-toggle = Activar o desactivar el marcado de palabras difíciles: subrayadas, y nombradas al moverse entre palabras con verbosidad alta
action-command-palette = Ejecutar cualquier comando por su nombre
action-settings = Abrir la configuración: cada opción con su ayuda, filtrada mientras escribe; Izquierda y Derecha cambian un valor
action-keyboard-help = Listar los atajos de teclado
action-help = Abrir la ayuda

## The interface language. $language is the language's name in itself
## (Español), $voice a voice's name.

language-voice-changed = La voz ahora es { $voice }, para { $language }.
language-voice-kept = No hay voz para { $language } en este motor de voz, así que { $voice } sigue hablando.
language-list-title = Idioma
language-list-intro =
    { $n ->
        [one] Idioma, 1 opción. Arriba y Abajo se mueven, Intro elige, Escape conserva el idioma.
       *[other] Idioma, { $n } opciones. Arriba y Abajo se mueven, Intro elige, Escape conserva el idioma.
    }

## Restarting speech.

restart-silent-now = { -brand } está en silencio ahora; reinicie la voz para volver a oírla.
# $keys names the Restart Speech key or keys.
restart-silent-use-key = { -brand } está en silencio ahora. Reinicie la voz con { $keys }.
restart-restarting = Reiniciando la voz.
restart-not-here = La voz no se puede reiniciar aquí.
restart-already = La voz ya se está reiniciando.
# $error is the system's reason, in its own words.
restart-failed = No se pudo reiniciar la voz: { $error }.
restart-start-failed = No se pudo reiniciar la voz: no se pudo iniciar.
restart-no-engine = No hay ningún motor de voz disponible; { -brand } permanece en silencio.
restart-done-silent = Se reinició la voz, pero no hay ningún motor de voz disponible; { -brand } permanece en silencio.
restart-done = Voz reiniciada.

## Tab completion of file paths in prompts.

# $folder is the folder's full path.
pathc-no-folder = No hay ninguna carpeta { $folder }.
# $prefix is what was typed after the last separator.
pathc-no-match = Ningún archivo o carpeta empieza por { $prefix }.
# The one name that matched; $kind is folder or file.
pathc-one =
    { $kind ->
        [folder] { $name }, carpeta
       *[file] { $name }, archivo
    }
# $n names matched; $names are the first few, joined with commas; $more is
# yes when more matched than are read out.
pathc-many =
    { $more ->
        [yes] { $n } coincidencias: { $names }, y más.
       *[no] { $n } coincidencias: { $names }.
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
settingsio-export-failed = No se pudo exportar la configuración: { $error }
# $path is the file; $error the system's reason.
settingsio-read-failed = No se pudo leer { $path }: { $error }.
settingsio-nothing-to-import = Nada que importar: su configuración ya coincide con ese archivo.
settingsio-cancelled-unchanged = Cancelado. No se cambió nada.
settingsio-import-failed = No se pudo importar la configuración: { $error }
# $summary lists what changed (from the settings store, in English).
settingsio-imported = Configuración importada. { $summary }
settingsio-backend-next-start = El nuevo motor de voz se usará a partir del próximo inicio.
settingsio-backed-up = Se hizo una copia de seguridad de la configuración anterior.

## Opening a document: failures and opening in the background.

# $name is the file's name.
opening-is-folder = { $name } es una carpeta, no un documento. Indique el nombre de un archivo dentro de ella.
# Said after "Could not open NAME:", so it starts in lower case.
opening-no-file-in = no hay ningún archivo llamado { $name } en { $folder }. Compruebe el nombre.
opening-no-file-here = no hay ningún archivo llamado { $name } aquí. Compruebe el nombre.
opening-no-permission = no tiene permiso para leerlo.
opening-damaged-rtf = no es un archivo RTF legible; puede estar dañado.
opening-damaged-odt = no es un archivo de texto OpenDocument legible; puede estar dañado.
# $reason is one of the opening-no-* messages, or the loader's own words.
opening-failed = No se pudo abrir { $name }: { $reason }
opening-started = Abriendo { $name }. Escape cancela.
opening-stopped = Se detuvo la apertura de { $name }.
opening-still = Todavía abriendo { $name }, { $secs } segundos.
# $step is the loader's report, such as "recognizing text on page 3 (3 of 40)."
opening-still-step = Todavía abriendo { $name }: { $step }
opening-stopped-unexpectedly = No se pudo abrir { $name }: la carga se detuvo de forma inesperada.

## A build without the publish feature. "tw convert" is a command typed
## at the terminal: keep it as it is.

lean-citations-not-in-build = Las citas no están en esta compilación de { -brand }. Se compiló sin la función de publicación.
lean-publish-not-in-build = Exportar y ver la vista previa no están en esta compilación de { -brand }. Se compiló sin la función de publicación; tw convert sigue convirtiendo.

## The voice manager's list.

# $n voices are shown; $language is a language name or voices-all-languages;
# $engine an engine's name or voices-all-engines.
voices-shown =
    { $n ->
        [one] { $n } voz: { $language }, { $engine }.
       *[other] { $n } voces: { $language }, { $engine }.
    }
voices-all-languages = todos los idiomas
voices-all-engines = todos los motores
# The filter rows at the top of the list.
voices-language-row = Idioma: { $language }
voices-engine-row = Motor: { $engine }
voices-fetch-row = Descargar la lista de voces de Piper desde internet
# Parts of a voice's row, joined with commas. $size is in megabytes, such
# as "63 MB".
voices-download-size = descarga { $size }
voices-licence-public-domain = dominio público
voices-licence-attribution = gratis con atribución
voices-licence-share-alike = gratis con atribución, compartir igual
voices-licence-non-commercial = no comercial
voices-licence-unknown = licencia mostrada antes de la descarga
voices-favourite = favorita
voices-current = actual

## Language names in the voice manager's language filter.

voices-language-ar = árabe
voices-language-ca = catalán
voices-language-cs = checo
voices-language-cy = galés
voices-language-da = danés
voices-language-de = alemán
voices-language-el = griego
voices-language-en = inglés
voices-language-es = español
voices-language-fa = persa
voices-language-fi = finés
voices-language-fr = francés
voices-language-hi = hindi
voices-language-hu = húngaro
voices-language-is = islandés
voices-language-it = italiano
voices-language-ja = japonés
voices-language-ka = georgiano
voices-language-kk = kazajo
voices-language-ko = coreano
voices-language-lb = luxemburgués
voices-language-lv = letón
voices-language-nl = neerlandés
voices-language-no = noruego
voices-language-pl = polaco
voices-language-pt = portugués
voices-language-ro = rumano
voices-language-ru = ruso
voices-language-sk = eslovaco
voices-language-sl = esloveno
voices-language-sr = serbio
voices-language-sv = sueco
voices-language-sw = suajili
voices-language-tr = turco
voices-language-uk = ucraniano
voices-language-vi = vietnamita
voices-language-zh = chino

## Voices: the voice manager, rate, pitch, and volume.

# Spoken by a newly chosen voice as its sample.
voice-sample = El veloz murciélago hindú comía feliz cardillo y kiwi.
voice-list-title = Elegir una voz
voice-still-loading = Las voces todavía se están cargando. La lista se abre cuando estén listas.
voice-list-failed = No se pudieron listar las voces: { $error }.
# $shown is voices-shown ("12 voices: English, all engines."). Enter,
# Space, Delete and Escape are the list's own keys.
voice-manager-intro = Gestor de voces. { $shown } Intro usa una voz y dice una muestra, o la descarga; Espacio marca una favorita; Suprimir elimina una voz descargada; Escape cierra.
voice-more-ready = { $n } voces más de otros motores están listas. Pulse Escape y vuelva a abrir el gestor de voces para verlas.
# $keys names the Choose Voice key.
voice-ready = Las voces están listas. { $keys } las lista.
voice-fetch-catalog-question = ¿Descargar la lista de voces de Piper, unos 250 kilobytes, desde Hugging Face? y o n
voice-fetch-catalog-question-short = ¿Descargar la lista de voces de Piper? y o n
# $engine is the engine's name, such as "Piper neural voices".
voice-switching-engine = Voz { $voice }, en { $engine }. Cambiando de motor.
voice-download-in-progress = Ya hay una descarga de voz en curso.
voice-no-data-folder = No hay ninguna carpeta de datos donde guardar las voces de Piper.
voice-not-in-list = Esa voz ya no está en la lista de voces de Piper.
voice-download-start-failed = No se pudo iniciar la descarga.
voice-reading-licence = Leyendo la licencia de { $voice }.
voice-remove-question = ¿Quitar la voz { $voice }? y o n
voice-only-piper-removable = Solo se pueden quitar las voces de Piper descargadas.
# $plan describes the download: the voice, its size and licence.
voice-download-question = { $plan } y o n
voice-in-use = { $voice } es la voz en uso. Elija otra voz primero.
voice-removed = { $voice } quitada.
voice-remove-failed = No se pudo quitar { $voice }: { $error }.
voice-downloading-catalog = Descargando la lista de voces de Piper.
voice-downloading = Descargando { $voice }.
voice-downloading-percent = Descargando { $voice }, { $pct } por ciento.
voice-details-failed = No se pudieron leer los detalles de la voz: { $error }.
voice-download-stopped = La descarga se detuvo.
voice-catalog-fetched = La lista de voces de Piper tiene { $voices } voces en { $languages } idiomas. Elegir Voz las lista.
voice-catalog-failed = No se pudo descargar la lista de voces: { $error }.
# $licence describes the voice's licence, in a sentence of its own.
voice-installed = { $voice } está instalada. { $licence } Elegir Voz la lista.
voice-download-failed = No se pudo descargar { $voice }: { $error }.
voice-only-voice-favourite = Solo una voz puede ser favorita.
voice-favourite-added = { $voice } agregada a favoritas.
voice-favourite-removed = { $voice } quitada de favoritas.
voice-chosen = Voz { $voice }.
voice-chosen-rate = Voz { $voice }, { $wpm } palabras por minuto.
voice-fastest-rate = Velocidad máxima.
voice-slowest-rate = Velocidad mínima.
voice-rate = { $wpm } palabras por minuto.
voice-highest-pitch = Tono más alto.
voice-lowest-pitch = Tono más bajo.
voice-pitch-normal = Tono normal.
# $n is a number of semitones.
voice-pitch-plus = Tono más { $n }.
voice-pitch-minus = Tono menos { $n }.
voice-full-volume = Volumen máximo.
voice-volume-off = Volumen desactivado.
voice-volume = Volumen { $pct } por ciento.
voice-no-speed-presets = No hay velocidades preestablecidas.
# $name is the preset's name from the settings, such as "Study".
voice-speed-preset = Velocidad { $name }, { $wpm } palabras por minuto.
voice-line-numbers-on = Números de línea activados.
voice-line-numbers-off = Números de línea desactivados.

## Export and preview from the reader. F5 is the browser's reload key,
## not textweaver's.

publish-no-document = No hay ningún documento abierto.
# Said after "Could not export:", so it starts in lower case. $path is a
# folder or a file; $error the system's reason.
publish-cannot-write-to = no se puede escribir en { $path }: { $error }
publish-cannot-write = no se puede escribir { $path }: { $error }
publish-start-failed = No se pudo iniciar la exportación: { $error }
publish-export-error = No se pudo exportar: { $error }
# $format is the format's name, such as PDF, HTML, or Word.
publish-exporting = Exportando a { $format }.
publish-writing-preview = Escribiendo la vista previa.
publish-preview-error = No se pudo escribir la vista previa: { $error }
publish-still-exporting =
    { $secs ->
        [one] Todavía exportando a { $format }, { $secs } segundo.
       *[other] Todavía exportando a { $format }, { $secs } segundos.
    }
publish-still-previewing =
    { $secs ->
        [one] Todavía escribiendo la vista previa, { $secs } segundo.
       *[other] Todavía escribiendo la vista previa, { $secs } segundos.
    }
# $again is yes when a preview is open already.
publish-auto-reload-on =
    { $again ->
        [yes] Recarga automática de la vista previa activada: después de cada guardado, el navegador recarga la página por sí solo. Ejecute ver vista previa en el navegador de nuevo para usarla.
       *[no] Recarga automática de la vista previa activada: después de cada guardado, el navegador recarga la página por sí solo.
    }
publish-auto-reload-off = Recarga automática de la vista previa desactivada: pulse F5 en el navegador después de guardar.
publish-live-on = Vista previa en vivo activada: la vista previa también se recarga al hacer una pausa al escribir.
# "toggle preview auto reload" is the command's name in the command palette.
publish-live-on-needs-reload = Vista previa en vivo activada. Funciona con la recarga automática, que está desactivada; actívela con toggle preview auto reload.
publish-live-off = Vista previa en vivo desactivada: la vista previa se recarga solo tras guardar.
# $error is the converter's reason.
publish-export-failed = Error al exportar a { $format }: { $error }
publish-preview-failed = Error en la vista previa: { $error }
# The converter's warnings: how many, and the first one.
publish-warnings =
    { $n ->
        [one] 1 advertencia: { $first }
       *[other] { $n } advertencias; la primera: { $first }
    }
# $file is the file's name, $folder its folder; $warned is empty or a
# space and publish-warnings.
publish-exported = Exportado a { $format }: { $file } en { $folder }.{ $warned } ¿Abrirlo? y o n.
publish-preview-written-served = Vista previa escrita. Abriéndola en el navegador. Se recarga por sí sola después de cada guardado.{ $warned }
publish-preview-written = Vista previa escrita. Abriéndola en el navegador. Guardar la vuelve a escribir; luego pulse F5 en el navegador.{ $warned }
publish-preview-updated = Vista previa actualizada.
publish-preview-updated-press-f5 = Vista previa actualizada. Pulse F5 en el navegador.
publish-server-failed = No se pudo iniciar el servidor de recarga de la vista previa ({ $error }); se abre el archivo en su lugar.
publish-render-failed = No se pudo representar el texto: { $error }
publish-nothing-after-caret = Nada que leer después del cursor.
publish-listening = Escuchando el texto representado.

## The preview's reload server: shown in the browser.

preview-being-written = La vista previa se está escribiendo. Vuelva a cargar en un momento.

## Notes and highlights.

notes-nothing-to-attach = No hay nada aquí para adjuntar una nota.
# $on is the start of the passage the note is on.
notes-added = Nota agregada en: { $on }
# $tags are the note's tags, joined with commas.
notes-added-with-tags = Nota agregada con las etiquetas { $tags } en: { $on }
# An item in the notes list. $anchor is the passage; $lost is yes when the
# passage was not found after the file changed.
notes-item =
    { $lost ->
        [yes] { $note }, línea { $line }. En: { $anchor } No se encontró después de que el archivo cambiara.
       *[no] { $note }, línea { $line }. En: { $anchor }
    }
notes-none = Sin notas.
notes-list-title = Notas
notes-list-intro =
    { $n ->
        [one] Notas, 1 elemento. Intro va a una nota, Suprimir la elimina, F2 la edita.
       *[other] Notas, { $n } elementos. Intro va a una nota, Suprimir la elimina, F2 la edita.
    }
# Said on jumping to a note: its text, then the passage it is on.
notes-note-content = { $note }. En: { $anchor }
# $i is the note's number, $n how many notes there are.
notes-note-label = Nota { $i } de { $n }
notes-deleted = Nota eliminada: { $text }.
notes-none-here = No hay ninguna nota o resaltado aquí.
notes-unchanged = Nota sin cambios.
notes-updated = Nota actualizada.
notes-nothing-to-highlight = No hay nada aquí para resaltar.
notes-highlight-removed = Resaltado quitado: { $text }
notes-highlighted-at = Resaltado en el { $pct } por ciento: { $text }
notes-highlighted = Resaltado: { $text }
# An item in the highlights list. $color is the highlight's color name;
# $lost is yes when the text was not found after the file changed.
notes-highlight-item =
    { $lost ->
        [yes] { $text }, línea { $line }, { $color }, no encontrado después de que el archivo cambiara
       *[no] { $text }, línea { $line }, { $color }
    }
notes-no-highlights = Sin resaltados.
notes-highlights-title = Resaltados
notes-highlights-intro =
    { $n ->
        [one] Resaltados, 1 elemento. Intro va a uno, Suprimir lo quita.
       *[other] Resaltados, { $n } elementos. Intro va a uno, Suprimir lo quita.
    }
# The label said before a highlight's text on jumping to it.
notes-highlight-label = Resaltado
# Shown while reading reaches a note's passage.
notes-signal = Nota: { $text }
# Said after moving onto a note's passage.
notes-has-note = Tiene una nota: { $text }

## Bookmarks: rename and delete.

notes-choose-bookmark-delete = Elija un marcador y pulse Suprimir.
notes-choose-bookmark-rename = Elija un marcador y pulse F2 para renombrarlo.
notes-bookmark-deleted = Marcador { $name } eliminado.
notes-renaming-bookmark = Renombrando el marcador { $name }.
notes-bookmark-unchanged = Marcador sin cambios.
# $name is the name asked for, $old the bookmark's name.
notes-bookmark-name-taken = Ya hay un marcador llamado { $name }. El marcador { $old } no cambió.
notes-bookmark-renamed = Se renombró el marcador { $old } a { $name }.

## The study sheet.

notes-nothing-to-export = No hay notas ni resaltados que exportar.
# Keep the letters y and n: they are the keys that answer. $file is the
# sheet's file name, $folder the folder it was saved in.
notes-study-sheet-saved-notes =
    Hoja de estudio con { $n ->
        [one] 1 nota
       *[other] { $n } notas
    } guardada como { $file } en { $folder }. ¿Abrirla? y o n.
notes-study-sheet-saved-highlights =
    Hoja de estudio con { $h ->
        [one] 1 resaltado
       *[other] { $h } resaltados
    } guardada como { $file } en { $folder }. ¿Abrirla? y o n.
notes-study-sheet-saved-both =
    Hoja de estudio con { $n ->
        [one] 1 nota
       *[other] { $n } notas
    } y { $h ->
        [one] 1 resaltado
       *[other] { $h } resaltados
    } guardada como { $file } en { $folder }. ¿Abrirla? y o n.
notes-study-sheet-failed = No se pudo escribir la hoja de estudio: { $error }
# The study sheet file's own text (Markdown; the # marks stay in the code).
notes-sheet-title = Hoja de estudio: { $title }
notes-sheet-exported = Exportado desde { -brand } el { $date }.
notes-sheet-before-first-heading = Antes del primer encabezado
# After a note's text: its tags, joined with commas.
notes-sheet-tags = (etiquetas: { $tags })
# $color is the highlight's color name.
notes-sheet-highlighted = Resaltado, { $color }.

## Find, bookmarks, and selection.

marks-cannot-search = No se puede buscar: { $error }.
# $pattern is the text searched for.
marks-no-matches = Sin coincidencias para { $pattern }.
# The label of a match reached by Find, at high verbosity; $number is its place among $n matches.
marks-match-label = Coincidencia { $number } de { $n }
# Find wrapped past an end of the document; $dir is next (to the top) or previous (to the bottom); $message says the match.
marks-find-wrapped =
    { $dir ->
        [next] Se volvió al principio. { $message }
       *[previous] Se volvió al final. { $message }
    }
# $name is the bookmark's name, such as mark1.
marks-bookmark-already-here = El marcador { $name } ya está aquí.
marks-bookmark-set = Marcador { $name } puesto en el { $pct } por ciento.
marks-no-bookmarks = Sin marcadores.
marks-bookmarks-intro =
    { $n ->
        [one] Marcadores, { $n } elemento. Intro va a uno, Suprimir lo elimina, F2 lo renombra.
       *[other] Marcadores, { $n } elementos. Intro va a uno, Suprimir lo elimina, F2 lo renombra.
    }
# One line of the bookmark list; $lost is yes when the bookmark's text was not found after the file changed; $text is the start of its line.
marks-bookmark-item =
    { $lost ->
        [yes] { $name } (no encontrado después de que el archivo cambiara), línea { $line }, { $pct } por ciento: { $text }
       *[no] { $name }, línea { $line }, { $pct } por ciento: { $text }
    }
marks-bookmarks-title = Marcadores
# The label of a bookmark reached, at high verbosity.
marks-bookmark-label = Marcador { $name }
marks-selection-cleared = Selección borrada.
# $text is the selected text, shortened.
marks-selected = Seleccionado { $text }

## Authoring lists: the outline, the citation picker, spelling, grammar, and templates.

# An outline item: $text is the heading's text, $level its level.
lists-outline-item = { $text }, nivel { $level }
# $n is the number of headings.
lists-outline-title =
    { $n ->
        [one] Esquema, { $n } encabezado
       *[other] Esquema, { $n } encabezados
    }
# $shown headings of $n match the filter $filter typed so far.
lists-outline-title-filtered = Esquema, { $shown } de { $n } coinciden con { $filter }
# $n is the number of references.
lists-citations-title =
    { $n ->
        [one] Insertar cita, { $n } referencia
       *[other] Insertar cita, { $n } referencias
    }
# $shown references of $n match the filter $filter typed so far.
lists-citations-title-filtered = Insertar cita, { $shown } de { $n } coinciden con { $filter }
# $word is the misspelled word.
lists-spelling-title = Ortografía de { $word }
lists-spelling-add = Agregar { $word } a su lista de palabras
lists-leave-as-is = Dejarlo como está
# $words are the words the grammar fixes are for.
lists-grammar-title = Correcciones de gramática para { $words }
# $n is the number of templates.
lists-templates-title = Documento nuevo a partir de una plantilla, { $n } plantillas
lists-no-filter = Esta lista no se puede filtrar.
# The filter was emptied: $n items are shown.
lists-filter-cleared-headings =
    { $n ->
        [one] Filtro borrado, { $n } encabezado.
       *[other] Filtro borrado, { $n } encabezados.
    }
lists-filter-cleared-references =
    { $n ->
        [one] Filtro borrado, { $n } referencia.
       *[other] Filtro borrado, { $n } referencias.
    }
lists-filter-cleared-items =
    { $n ->
        [one] Filtro borrado, { $n } elemento.
       *[other] Filtro borrado, { $n } elementos.
    }
# Nothing matches the filter $query.
lists-filter-none-headings = Ningún encabezado coincide con { $query }. Retroceso quita letras.
lists-filter-none-references = Ninguna referencia coincide con { $query }. Retroceso quita letras.
lists-filter-none-items = Ningún elemento coincide con { $query }. Retroceso quita letras.
# $n items match the filter.
lists-filter-matched-headings =
    { $n ->
        [one] { $n } encabezado coincide.
       *[other] { $n } encabezados coinciden.
    }
lists-filter-matched-references =
    { $n ->
        [one] { $n } referencia coincide.
       *[other] { $n } referencias coinciden.
    }
lists-filter-matched-items =
    { $n ->
        [one] { $n } elemento coincide.
       *[other] { $n } elementos coinciden.
    }
lists-no-headings = Este documento no tiene encabezados.
# $n is the number of headings; the keys are the outline list's own.
lists-outline-intro =
    { $n ->
        [one] Esquema, { $n } encabezado. Escriba para filtrar, Intro va a un encabezado, Escape cierra.
       *[other] Esquema, { $n } encabezados. Escriba para filtrar, Intro va a un encabezado, Escape cierra.
    }
# $heading is the text of the heading the cursor is under.
lists-outline-here = Está bajo { $heading }.

## Lists and prompts shared by every frontend.

# The focused list item: $item is its text, $k its place, $n the number of items.
listmodel-item-position = { $item }, { $k } de { $n }
# $letter is the letter or digit typed.
listmodel-no-item-starts = Ningún elemento empieza por { $letter }.
listmodel-top-of-list = Principio de la lista.
listmodel-end-of-list = Final de la lista.
listmodel-no-matching-commands = No hay comandos coincidentes.
listmodel-no-earlier-entries = No hay entradas anteriores.
# Tab in the command palette: $n commands match (always more than one); $names lists the first few, joined by commas.
listmodel-command-matches = { $n } coincidencias: { $names }.

## The library.

# $n is how many documents the scan has found.
library-still-scanning = Todavía explorando la biblioteca: { $n } encontrados hasta ahora.
library-scan-failed = No se pudo explorar la biblioteca: { $error }.
library-scanning = Explorando la biblioteca.
library-scan-progress = Explorando la biblioteca: { $n } encontrados hasta ahora.
library-scan-stopped = La exploración de la biblioteca se detuvo por un error interno.
# $command is the command line that adds a folder; $key names the Open command's key.
library-empty = La biblioteca está vacía. Agregue una carpeta con { $command }, o abra un archivo con { $key }.
library-intro =
    { $n ->
        [one] Biblioteca, { $n } documento. Intro abre uno.
       *[other] Biblioteca, { $n } documentos. Intro abre uno.
    }
library-title = Biblioteca

## Following links and footnotes.

links-none-here = No hay ningún enlace o nota al pie en el cursor.
# $text is the link's text.
links-no-address = El enlace { $text } no tiene dirección.
# $kind is mail or web; $target is the link's address.
links-open-question =
    { $kind ->
        [mail] Enlace de correo: { $target }. ¿Abrirlo? y o n.
       *[web] Enlace web: { $target }. ¿Abrirlo? y o n.
    }
# The label of a heading reached by a link, at high verbosity.
links-heading-label = Encabezado
# $anchor is the heading name the link gives.
links-no-heading = No hay ningún encabezado llamado { $anchor } en este documento.
# $file is the file the link names.
links-file-not-found = El enlace va a { $file }, que no se encontró.
# $file is the file's name; $key names the History Back command's keys.
links-followed = Se siguió el enlace a { $file }. Atrás: { $key }.
links-back-in = Atrás en { $file }.
# $label is the footnote's label, such as 1.
links-back-to-footnote-reference = Volver a la referencia de la nota al pie { $label }, línea { $line }.
# $text is the start of the note.
links-footnote = Nota al pie { $label }: { $text }
links-footnote-unreferenced = No hay ninguna referencia a la nota al pie { $label } en el texto.
links-footnote-no-note = La nota al pie { $label } no tiene nota.

## Citations: inserting, looking up, importing, checking, and the bibliography.

citations-on = Citas activadas.
citations-off = Citas desactivadas.
# $key names the Add Reference command's keys.
citations-library-empty = Su biblioteca de referencias está vacía. Agregue una referencia por DOI o ISBN con { $key }, o ejecute importar referencias desde la paleta de comandos.
# $n is how many references the picker lists.
citations-picker-intro =
    { $n ->
        [one] Insertar cita, { $n } referencia. Escriba para filtrar, Intro elige, Escape cancela.
       *[other] Insertar cita, { $n } referencias. Escriba para filtrar, Intro elige, Escape cancela.
    }
# $text is what was typed at the locator prompt.
citations-locator-unreadable = No se pudo leer el localizador { $text }. Escriba una página como 12, páginas como 3-5, o capítulo 2; Intro sola para ninguno.
citations-insert-failed = No se pudo insertar la cita: { $error }
# $what is the identifier being looked up, as the citation library describes it.
citations-looking-up = Buscando { $what }.
citations-lookup-not-started = No se pudo iniciar la búsqueda: { $error }
# $input is the DOI or ISBN as typed.
citations-lookup-failed = No se pudo buscar { $input }: { $error }
citations-no-library-to-add-to = No hay ninguna biblioteca a la que agregar: { -brand } no guarda archivos en esta sesión.
citations-library-save-failed = No se pudo guardar la biblioteca: { $error }
# $n is how many citations the document has.
citations-found-no-library =
    { $n ->
        [one] Se encontró { $n } cita. { -brand } no guarda biblioteca en esta sesión.
       *[other] Se encontraron { $n } citas. { -brand } no guarda biblioteca en esta sesión.
    }
citations-check-failed = No se pudieron comprobar las citas: { $error }
citations-no-library-to-import-into = No hay ninguna biblioteca a la que importar: { -brand } no guarda archivos en esta sesión.
# $file is the file's path.
citations-import-failed = No se pudo importar { $file }: { $error }
# $style is the style's name from the front matter, such as apa.
citations-style-unusable = No se puede usar el estilo de cita { $style }: { $error }
citations-format-failed = No se pudieron formatear las citas: { $error }
# $key names the Insert Citation command's keys.
citations-none-yet = El documento todavía no tiene citas. Inserte una con { $key }.
citations-nothing-to-list = Ninguna de las obras citadas está en su biblioteca, así que no hay nada que listar.
# $n is how many entries went in; $style is the style's name, such as apa.
citations-bibliography-inserted =
    { $n ->
        [one] Se insertó la bibliografía, { $n } entrada, estilo { $style }.
       *[other] Se insertó la bibliografía, { $n } entradas, estilo { $style }.
    }
# Follows citations-bibliography-inserted; $keys are citation keys joined with commas.
citations-not-in-library = No están en la biblioteca: { $keys }.
citations-bibliography-insert-failed = No se pudo insertar la bibliografía: { $error }

## Speech Cursor mode.

speechcursor-off = Cursor de lectura desactivado.
# $line is the line number.
speechcursor-on = Cursor de lectura activado, línea { $line }. Arriba y Abajo leen líneas, Intro sigue leyendo, Tab o Escape salen.
# $text is the line read, as the status line shows it.
speechcursor-on-with-text = Cursor de lectura activado, línea { $line }: { $text }. Arriba y Abajo leen líneas, Intro sigue leyendo, Tab o Escape salen.

## Scrolling the view without moving the cursor.

view-bottom-of-document = Final del documento.
# $line is the line now at the top of the view.
view-line-at-top = Línea { $line } arriba.

## Background work, and opening files and addresses.

# $what names the export, as its own message says it.
tasks-stopped = { $what } se detuvo de forma inesperada.
# $input is the DOI, ISBN, or other identifier being looked up.
tasks-lookup-stopped = La búsqueda de { $input } se detuvo de forma inesperada.
# The reason in tasks-could-not-open when a session keeps no files.
tasks-launch-off = abrir otros programas está desactivado en una sesión que no guarda archivos
tasks-opening = Abriendo.
# $target is a file or a web address; $error says why.
tasks-could-not-open = No se pudo abrir { $target }: { $error }
tasks-not-opened = No se abrió.
# Keep the letters y and n: they are the keys that answer.
tasks-open-it-question = ¿Abrirlo? y o n.

## Math exploration.

mathx-no-math = No hay matemáticas aquí. Vaya a una fórmula y vuelva a intentarlo.
# $math is the whole expression as spoken; $parts is yes when it has parts to go into. The keys are exploration's own.
mathx-exploring =
    { $parts ->
        [yes] Explorando matemáticas: { $math }. Abajo entra, las flechas mueven, Escape sale.
       *[no] Explorando matemáticas: { $math }. Escape sale.
    }
mathx-left = Salió de las matemáticas.
mathx-last-term = Último término.
mathx-first-term = Primer término.
mathx-no-parts = No hay partes dentro.
mathx-whole-expression = Expresión completa.
mathx-nothing-here = No hay nada aquí.

## Reading aids: RSVP, bionic reading, syllables, difficult words, the ruler, and the reading level.

aids-rsvp-off = RSVP desactivado.
aids-rsvp-leave-edit = Salga del modo de edición para usar RSVP.
# $status is RSVP's status line (word and sentence counts, rate, state).
aids-rsvp-on = RSVP activado. { $status }
aids-rsvp-no-words = No hay palabras que mostrar.
aids-rsvp-fastest = Velocidad de RSVP máxima.
aids-rsvp-slowest = Velocidad de RSVP mínima.
# $wpm is the new rate in words per minute.
aids-rsvp-rate = RSVP { $wpm } palabras por minuto.
# Where the RSVP word is shown; $position is one of nine fixed keys.
aids-rsvp-position =
    { $position ->
        [top-left] RSVP arriba a la izquierda.
        [top-center] RSVP arriba al centro.
        [top-right] RSVP arriba a la derecha.
        [center-left] RSVP en el medio a la izquierda.
        [center] RSVP en el centro.
        [center-right] RSVP en el medio a la derecha.
        [bottom-left] RSVP abajo a la izquierda.
        [bottom-right] RSVP abajo a la derecha.
       *[bottom-center] RSVP abajo al centro.
    }
aids-rsvp-playing = RSVP reproduciéndose.
aids-rsvp-paused = RSVP en pausa.
aids-rsvp-end-of-text = Final del texto.
aids-rsvp-start-of-text = Principio del texto.
aids-bionic-on = Lectura biónica activada.
aids-bionic-off = Lectura biónica desactivada.
aids-syllables-shown = Sílabas mostradas.
aids-syllables-hidden = Sílabas ocultas.
aids-difficult-on = Palabras difíciles subrayadas.
aids-difficult-no-list = Palabras difíciles activadas, pero falta la lista de palabras en esta compilación.
aids-difficult-off = Palabras difíciles sin marcar.
# Added after a word at high verbosity, following a comma.
aids-difficult-word = palabra difícil
aids-ruler-off = Regla de lectura desactivada.
aids-ruler-current-line = Línea actual marcada.
aids-ruler-on = Regla de lectura activada.
# $summary is aids-level-summary; $scope says what was measured.
aids-reading-level =
    { $scope ->
        [selection] Selección: { $summary }
       *[document] Documento: { $summary }
    }
aids-reading-level-too-short = No hay suficiente texto para medir el nivel de lectura.
# $grade is the Flesch-Kincaid grade with one decimal, $band an aids-band-* message, $ease the reading ease (0 to 100), $words aids-level-words, $sentences aids-level-sentences.
aids-level-summary = Grado { $grade }, { $band }. Facilidad de lectura { $ease } sobre 100. { $words } en { $sentences }.
# $n is the count, $count the same number written with thousands separators.
aids-level-words =
    { $n ->
        [one] { $count } palabra
       *[other] { $count } palabras
    }
aids-level-sentences =
    { $n ->
        [one] { $count } oración
       *[other] { $count } oraciones
    }
aids-band-elementary = primaria
aids-band-middle-school = secundaria
aids-band-high-school = bachillerato
aids-band-college = universitario
aids-band-graduate = posgrado

## Themes.

# $name is the theme name in the settings; $used the display name of the theme used instead.
themes-unknown = No hay ningún tema llamado { $name }; se usa { $used }.
# $theme is the new theme's display name.
themes-next = Tema { $theme }.

## Accessibility modes and the first-run question.

# Said when the accessibility mode changes; $mode is the new mode's id.
access-mode-changed =
    { $mode ->
        [self-voicing] Modo de voz propia. textweaver lo dice todo.
        [screen-reader] Modo lector de pantalla. textweaver está en silencio; su lector de pantalla lee la línea de estado.
       *[hybrid] Modo híbrido. textweaver lee los documentos en voz alta; su lector de pantalla dice los mensajes y lo que escribe.
    }
# Yes to the first-run question; $key names the keys that change the mode.
access-hybrid-chosen = Modo híbrido. textweaver lee los documentos en voz alta; su lector de pantalla dice los mensajes y lo que escribe. { $key } cambia el modo.
# No to the first-run question; $key names the keys that change the mode.
access-hybrid-declined = Se mantiene el modo de voz propia. { $key } cambia el modo.
# $reader is the screen reader found (NVDA, JAWS), or access-a-screen-reader.
access-hybrid-question = { $reader } se está ejecutando. ¿Usar el modo híbrido, donde textweaver lee los documentos en voz alta y su lector de pantalla dice los mensajes y lo que escribe? y o n
access-a-screen-reader = Un lector de pantalla

## Characters and selections, as spoken.

# The name of a white-space character read on its own; $name is a fixed key.
text-char-name =
    { $name ->
        [space] espacio
        [new-line] salto de línea
        [tab] tabulador
        [no-break-space] espacio de no separación
       *[white-space] espacio en blanco
    }
# Said after a selection grows ($change is selected) or shrinks (unselected); $text is the text or a character's name.
text-selection-change =
    { $change ->
        [selected] { $text } seleccionado
       *[unselected] { $text } deseleccionado
    }

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
settings-cannot-be = { $label } no puede ser eso: { $error }.
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

## Settings: labels, help, and choices, as the settings screen shows and
## says them. Ids follow the key in settings.toml (speech.rate is
## setting-speech-rate).

setting-speech-backend = Motor de voz
setting-speech-backend-help = El motor de voz: automático elige el mejor disponible. Un cambio reinicia la voz.
choice-speech-backend-auto = automático
choice-speech-backend-eci = Eloquence
choice-speech-backend-sapi = voces SAPI 5
choice-speech-backend-espeak = eSpeak NG
choice-speech-backend-speechd = Speech Dispatcher
choice-speech-backend-nsspeech = Apple NSSpeech
choice-speech-backend-avspeech = Apple AVSpeech
choice-speech-backend-dectalk = DECtalk
choice-speech-backend-omnivox = Omnivox
choice-speech-backend-null = silencioso
setting-speech-rate = Velocidad
setting-speech-rate-help = Con qué rapidez habla textweaver.
setting-speech-volume = Volumen
setting-speech-volume-help = Con qué fuerza habla textweaver.
setting-speech-pitch = Tono
setting-speech-pitch-help = Más alto o más bajo que el tono propio de la voz.
setting-speech-voice = Voz
setting-speech-voice-help = El id de la voz; sin definir elige una automáticamente. Elegir Voz las lista.
setting-speech-prefer-voice = Voz preferida
setting-speech-prefer-voice-help = Cuando no hay voz definida, la primera voz cuyo nombre contenga esto, como eloquence.
setting-speech-favorite-voices = Voces favoritas
setting-speech-favorite-voices-help = Voces listadas primero en Elegir Voz, por id.
setting-speech-punctuation = Puntuación
setting-speech-punctuation-help = Cuánta puntuación se dice.
choice-speech-punctuation-none = ninguna
choice-speech-punctuation-some = algo
choice-speech-punctuation-all = toda
setting-speech-split-caps = Separar mayúsculas
setting-speech-split-caps-help = Decir por separado palabras unidas con mayúsculas, como TextWeaver.
setting-speech-caps = Mayúsculas
setting-speech-caps-help = Cómo se marca una mayúscula al hablar y escribir caracteres.
choice-speech-caps-none = sin marcar
choice-speech-caps-tone = un tono
choice-speech-caps-pitch = un tono más alto
choice-speech-caps-say-cap = decir «mayúscula»
setting-speech-auto-play = Leer al abrir
setting-speech-auto-play-help = Empezar a leer al abrir un documento.
setting-speech-skip-code = Omitir bloques de código
setting-speech-skip-code-help = No leer los bloques de código.
setting-speech-speed-presets = Velocidades preestablecidas
setting-speech-speed-presets-help = Velocidades con nombre que F8 recorre.
setting-speech-voices-by-language = Voces por idioma
setting-speech-voices-by-language-help = La voz de cada idioma de la interfaz, por etiqueta de idioma, como es = el id de la voz. Un idioma no listado usa la primera voz del motor para él.
setting-speech-latency-offset-ms = Retraso del resaltado
setting-speech-latency-offset-ms-help = Cuánto tiempo después de que un motor informe una palabra se mueve el resaltado, para motores medidos por su reloj de audio.
setting-speech-verbosity = Verbosidad
setting-speech-verbosity-help = Cuánto dice textweaver sobre lo que hace.
choice-speech-verbosity-low = baja
choice-speech-verbosity-normal = normal
choice-speech-verbosity-high = alta
setting-speech-eci-dictionaries = Diccionarios de Eloquence
setting-speech-eci-dictionaries-help = Los diccionarios de pronunciación de la comunidad para Eloquence: activados, desactivados, o una carpeta propia.
choice-speech-eci-dictionaries-true = activados
choice-speech-eci-dictionaries-false = desactivados
setting-speech-eci-library = Biblioteca de Eloquence
setting-speech-eci-library-help = La biblioteca ECI que se cargará; sin definir busca en los lugares habituales.
setting-speech-eci-code-factory = Buscar Eloquence de Code Factory
setting-speech-eci-code-factory-help = Buscar también Eloquence de Code Factory para Windows. Es posible que su licencia no cubra otros programas.
setting-speech-sapi-onecore = Voces OneCore
setting-speech-sapi-onecore-help = Listar también las voces OneCore de Windows a través de SAPI 5.
setting-speech-apple-backend = Motor de voz de Apple
setting-speech-apple-backend-help = Qué motor de voz de Apple usar en macOS.
choice-speech-apple-backend-auto = automático
choice-speech-apple-backend-nsspeech = NSSpeechSynthesizer
choice-speech-apple-backend-avspeech = AVSpeechSynthesizer
setting-highlight-enabled = Resaltar el texto leído
setting-highlight-enabled-help = Resaltar la palabra o la oración que se está leyendo.
setting-highlight-granularity = Resaltado
setting-highlight-granularity-help = Qué cubre el resaltado de lectura.
choice-highlight-granularity-word = la palabra
choice-highlight-granularity-sentence = la oración
choice-highlight-granularity-both = la palabra y la oración
setting-highlight-lead-words = Adelanto del resaltado
setting-highlight-lead-words-help = Dibujar el resaltado esta cantidad de palabras por delante de la palabra oída (1 es la palabra oída).
setting-highlight-speed = Velocidad del resaltado
setting-highlight-speed-help = Velocidad del resaltado cronometrado para motores que no informan palabras.
setting-highlight-color = Color del resaltado de palabra
setting-highlight-color-help = Un nombre de color o #rrggbb sobre el resaltado de palabra del tema; tema conserva el del tema.
setting-highlight-sentence-color = Color del resaltado de oración
setting-highlight-sentence-color-help = Un nombre de color o #rrggbb sobre el resaltado de oración del tema; sin definir conserva el del tema.
setting-normalization-math = Hablar matemáticas
setting-normalization-math-help = Decir la notación matemática con palabras.
setting-normalization-math-verbosity = Verbosidad de las matemáticas
setting-normalization-math-verbosity-help = Cuán explícitas son las matemáticas habladas: baja dice a entre b, normal y alta dicen más.
choice-normalization-math-verbosity-low = baja
choice-normalization-math-verbosity-normal = normal
choice-normalization-math-verbosity-high = alta
setting-normalization-asciimath-delimiter = Delimitador de ASCIIMath
setting-normalization-asciimath-delimiter-help = El carácter alrededor de ASCIIMath, normalmente un acento grave; sin definir no lee ningún ASCIIMath.
setting-normalization-abbreviations = Expandir abreviaturas
setting-normalization-abbreviations-help = Decir las abreviaturas completas, como Doctor en vez de Dr.
setting-normalization-abbrev-expansions = Sus abreviaturas
setting-normalization-abbrev-expansions-help = Abreviaturas propias y lo que significan.
setting-normalization-numbers = Números en palabras
setting-normalization-numbers-help = Decir números, fechas, horas y cantidades de dinero con palabras.
setting-normalization-use-pronunciations = Usar pronunciaciones
setting-normalization-use-pronunciations-help = Aplicar su lista de pronunciaciones.
setting-normalization-pronunciations = Pronunciaciones
setting-normalization-pronunciations-help = Palabras y cómo decirlas.
setting-normalization-table-mode = Tablas
setting-normalization-table-mode-help = Cómo se leen las tablas.
choice-normalization-table-mode-structured = con filas y columnas
choice-normalization-table-mode-flat = como texto
choice-normalization-table-mode-skip = omitidas
setting-normalization-footnote-mode = Notas al pie
setting-normalization-footnote-mode-help = Dónde se leen las notas al pie.
choice-normalization-footnote-mode-inline = donde están marcadas
choice-normalization-footnote-mode-deferred = al final
choice-normalization-footnote-mode-skip = omitidas
setting-normalization-community-lexicon-enabled = Léxico comunitario
setting-normalization-community-lexicon-enabled-help = Aplicar los diccionarios de pronunciación de la comunidad para motores que no sean Eloquence.
setting-normalization-community-lexicon-dir = Carpeta del léxico comunitario
setting-normalization-community-lexicon-dir-help = La carpeta que contiene los archivos del diccionario; sin definir busca junto a textweaver.
setting-normalization-community-lexicon-language = Idioma del léxico comunitario
setting-normalization-community-lexicon-language-help = El idioma de los diccionarios.
choice-normalization-community-lexicon-language-enu = inglés de EE. UU.
choice-normalization-community-lexicon-language-deu = alemán
setting-reading-auto-resume = Reanudar donde lo dejó
setting-reading-auto-resume-help = Volver a la posición guardada al abrir un documento.
setting-reading-nav-history-size = Historial de Atrás
setting-reading-nav-history-size-help = Cuántos lugares recuerda Atrás.
setting-reading-wrap-navigation = Navegación cíclica
setting-reading-wrap-navigation-help = Moverse más allá del final del documento continúa desde el principio.
setting-reading-cursor-follows-speech = El cursor sigue a la voz
setting-reading-cursor-follows-speech-help = El cursor se mueve con la palabra que se está leyendo.
setting-reading-sync-conflict-policy = Posiciones sincronizadas
setting-reading-sync-conflict-policy-help = Qué posición prevalece cuando otro dispositivo leyó más lejos o más tarde.
choice-reading-sync-conflict-policy-newest = la más reciente
choice-reading-sync-conflict-policy-highest-progress = la más avanzada
choice-reading-sync-conflict-policy-manual = preguntar
setting-reading-citations = Citas
setting-reading-citations-help = Citas en la lectura continua: omitidas, o dichas en palabras.
choice-reading-citations-off = omitidas
choice-reading-citations-words = en palabras
setting-reading-ocr = Reconocer páginas escaneadas
setting-reading-ocr-help = Leer el texto de PDF e imágenes escaneados reconociéndolo (OCR).
setting-reading-ocr-lang = Idioma del texto escaneado
setting-reading-ocr-lang-help = El idioma del texto escaneado, como códigos de Tesseract tales como fra o deu+eng; vacío significa el idioma propio del documento, si no, inglés.
choice-reading-ocr-lang- = el del documento
choice-reading-ocr-lang-eng = inglés
choice-reading-ocr-lang-fra = francés
choice-reading-ocr-lang-deu = alemán
choice-reading-ocr-lang-spa = español
setting-reading-ocr-engine = Motor de OCR
setting-reading-ocr-engine-help = Qué motor reconoce las páginas escaneadas: ocrs para inglés y Tesseract para otros idiomas, o uno de ellos siempre.
choice-reading-ocr-engine-auto = automático
choice-reading-ocr-engine-ocrs = ocrs
choice-reading-ocr-engine-tesseract = Tesseract
choice-reading-ocr-engine-paddle = PaddleOCR (experimental)
setting-reading-math-engine = Voz de las matemáticas
setting-reading-math-engine-help = Qué motor lee las matemáticas en voz alta: el propio de textweaver, o MathCAT en ClearSpeak o SimpleSpeak, en el idioma del documento. MathCAT necesita una compilación que lo incluya; si no, se usa el propio de textweaver.
choice-reading-math-engine-builtin = textweaver
choice-reading-math-engine-mathcat = MathCAT ClearSpeak
choice-reading-math-engine-mathcat-simplespeak = MathCAT SimpleSpeak
setting-reading-math-display = Matemáticas en pantalla
setting-reading-math-display-help = Cómo se ven las matemáticas en la vista de lectura: como su fuente, tal como x^2, o como Unicode, tal como x con un 2 en superíndice. La voz y el modo de edición siempre usan la fuente.
choice-reading-math-display-source = fuente
choice-reading-math-display-unicode = Unicode
setting-reading-revisions = Control de cambios
setting-reading-revisions-help = Cómo se leen los cambios registrados en archivos de Word, OpenDocument y RTF: se dicen en su lugar con verbosidad alta (automático), siempre, o nunca, leyendo el texto final. Se aplica al abrir un documento.
choice-reading-revisions-auto = automático
choice-reading-revisions-marked = decirlos siempre
choice-reading-revisions-final = solo el texto final
setting-display-theme = Tema
setting-display-theme-help = El tema de color.
setting-display-follow-os-theme = Seguir el tema del sistema
setting-display-follow-os-theme-help = Al iniciar, usar un tema claro, oscuro o de alto contraste como el del sistema, a menos que haya elegido uno.
setting-display-wrap-width = Ancho de ajuste
setting-display-wrap-width-help = Ajustar las líneas a esta cantidad de columnas; 0 usa todo el ancho.
setting-display-tab-width = Ancho de tabulación
setting-display-tab-width-help = Columnas que ocupa un tabulador.
setting-display-show-line-numbers = Números de línea
setting-display-show-line-numbers-help = Mostrar los números de línea.
setting-display-scroll-margin = Margen de desplazamiento
setting-display-scroll-margin-help = Líneas que se mantienen visibles por encima y por debajo del cursor.
setting-editing-autosave-recovery = Instantáneas de recuperación
setting-editing-autosave-recovery-help = Guardar una copia del trabajo sin guardar y ofrecerla después de un fallo.
setting-editing-autosave-interval-secs = Intervalo de instantáneas
setting-editing-autosave-interval-secs-help = Segundos entre instantáneas de recuperación mientras hay cambios sin guardar.
setting-editing-echo-characters = Eco de caracteres
setting-editing-echo-characters-help = Decir cada carácter escrito.
setting-editing-echo-words = Eco de palabras
setting-editing-echo-words-help = Decir cada palabra escrita.
setting-editing-echo-deletions = Eco de eliminaciones
setting-editing-echo-deletions-help = Decir lo que quitan Retroceso y Suprimir.
setting-editing-echo-lines-on-move = Eco de líneas
setting-editing-echo-lines-on-move-help = Decir la línea cuando el cursor se mueve a otra línea.
setting-editing-undo-steps = Pasos de deshacer
setting-editing-undo-steps-help = Máximo de pasos de deshacer conservados al editar.
setting-editing-undo-memory-mb = Memoria de deshacer
setting-editing-undo-memory-mb-help = Máxima memoria que puede usar el historial de deshacer.
setting-library-recent-limit = Archivos recientes
setting-library-recent-limit-help = Cuántos archivos recientes se recuerdan.
setting-library-folders = Carpetas de la biblioteca
setting-library-folders-help = Carpetas cuyos documentos lista la biblioteca, y cuyas posiciones se sincronizan entre computadoras.
setting-keyboard-character-keys = Atajos de una sola tecla
setting-keyboard-character-keys-help = Teclas de exploración como h y punto. Desactivado, el dictado y la escritura nunca activan comandos.
setting-keyboard-preset = Teclas
setting-keyboard-preset-help = Las teclas predeterminadas: como el modo de exploración de NVDA y JAWS, o las teclas anteriores de textweaver. Se usa a partir del próximo inicio.
choice-keyboard-preset-default = estilo lector de pantalla
choice-keyboard-preset-classic = clásico
setting-keyboard-digit-row = Fila de dígitos
setting-keyboard-digit-row-help = Cómo reconoce el terminal las teclas de dígitos para los niveles de encabezado: automático, o un teclado AZERTY francés.
choice-keyboard-digit-row-auto = automático
choice-keyboard-digit-row-azerty = AZERTY
setting-accessibility-mode = Modo de accesibilidad
setting-accessibility-mode-help = Voz propia lo dice todo; lector de pantalla deja la voz a su lector de pantalla; híbrido solo pone voz a la lectura.
choice-accessibility-mode-self-voicing = voz propia
choice-accessibility-mode-screen-reader = lector de pantalla
choice-accessibility-mode-hybrid = híbrido
setting-accessibility-say-all = Decir todo con un lector de pantalla
setting-accessibility-say-all-help = Lectura continua en modo lector de pantalla: una oración a la vez en la línea de estado, o con la voz de textweaver.
choice-accessibility-say-all-screen = en la línea de estado
choice-accessibility-say-all-voice = con la voz de textweaver
setting-accessibility-quiet-screen = Pantalla quieta al leer
setting-accessibility-quiet-screen-help = Mantener la pantalla quieta mientras textweaver lee en voz alta.
setting-accessibility-cursor = Cursor
setting-accessibility-cursor-help = Dónde espera el cursor del terminal: en lo que está trabajando, o en la línea de estado.
choice-accessibility-cursor-follow = sigue al foco
choice-accessibility-cursor-status = en la línea de estado
setting-export-subtitle-format = Formato de subtítulos
setting-export-subtitle-format-help = El formato de los subtítulos escritos sin nombre de archivo.
choice-export-subtitle-format-srt = SubRip
choice-export-subtitle-format-vtt = WebVTT
setting-export-subtitle-word-level = Subtítulos por palabra
setting-export-subtitle-word-level-help = Un subtítulo por palabra en vez de líneas de subtítulo.
setting-export-subtitles-with-audio = Subtítulos con audio
setting-export-subtitles-with-audio-help = Escribir siempre subtítulos junto al audio exportado.
setting-reading-aids-rsvp-wpm = Velocidad de RSVP
setting-reading-aids-rsvp-wpm-help = Palabras por minuto de la presentación visual serial rápida.
setting-reading-aids-rsvp-pacing = Ritmo de RSVP
setting-reading-aids-rsvp-pacing-help = Qué hace avanzar la palabra de RSVP: su propio temporizador, o la voz.
choice-reading-aids-rsvp-pacing-timer = su propio temporizador
choice-reading-aids-rsvp-pacing-external = la voz
setting-reading-aids-rsvp-clause-pause = Pausa de cláusula de RSVP
setting-reading-aids-rsvp-clause-pause-help = Tiempo extra después de una coma, dos puntos, guion o paréntesis, en porcentaje del tiempo de una palabra.
setting-reading-aids-rsvp-sentence-pause = Pausa de oración de RSVP
setting-reading-aids-rsvp-sentence-pause-help = Tiempo extra al final de una oración, en porcentaje.
setting-reading-aids-rsvp-paragraph-pause = Pausa de párrafo de RSVP
setting-reading-aids-rsvp-paragraph-pause-help = Tiempo extra al final de un párrafo, en porcentaje.
setting-reading-aids-rsvp-long-word-len = Palabra larga de RSVP
setting-reading-aids-rsvp-long-word-len-help = Las palabras más largas que esto reciben tiempo extra.
setting-reading-aids-rsvp-long-word-step = Paso de palabra larga de RSVP
setting-reading-aids-rsvp-long-word-step-help = Tiempo extra por letra más allá de la longitud de una palabra larga, en porcentaje.
setting-reading-aids-rsvp-long-word-max = Máximo de palabra larga de RSVP
setting-reading-aids-rsvp-long-word-max-help = Máximo tiempo extra que recibe una palabra larga, en porcentaje.
setting-reading-aids-rsvp-show-previous = Palabra anterior de RSVP
setting-reading-aids-rsvp-show-previous-help = Mostrar también la palabra anterior.
setting-reading-aids-rsvp-show-next = Palabra siguiente de RSVP
setting-reading-aids-rsvp-show-next-help = Mostrar también la palabra siguiente.
setting-reading-aids-rsvp-position = Posición de RSVP
setting-reading-aids-rsvp-position-help = Dónde aparece la palabra de RSVP.
choice-reading-aids-rsvp-position-top-left = arriba a la izquierda
choice-reading-aids-rsvp-position-top-center = arriba al centro
choice-reading-aids-rsvp-position-top-right = arriba a la derecha
choice-reading-aids-rsvp-position-center-left = en el medio a la izquierda
choice-reading-aids-rsvp-position-center = en el medio
choice-reading-aids-rsvp-position-center-right = en el medio a la derecha
choice-reading-aids-rsvp-position-bottom-left = abajo a la izquierda
choice-reading-aids-rsvp-position-bottom-center = abajo al centro
choice-reading-aids-rsvp-position-bottom-right = abajo a la derecha
setting-reading-aids-rsvp-font-size-pt = Tamaño de RSVP
setting-reading-aids-rsvp-font-size-pt-help = Tamaño de la palabra de RSVP en la interfaz gráfica.
setting-reading-aids-rsvp-lead-words = Adelanto de RSVP
setting-reading-aids-rsvp-lead-words-help = Con el ritmo por voz, mostrar esta cantidad de palabras por delante de la palabra hablada.
setting-reading-aids-bionic = Lectura biónica
setting-reading-aids-bionic-help = Dibujar el principio de cada palabra en negrita.
setting-reading-aids-bionic-options-ratio = Proporción biónica
setting-reading-aids-bionic-options-ratio-help = Cuánto de cada palabra está en negrita.
setting-reading-aids-bionic-options-min-word-len = Palabra más corta para biónica
setting-reading-aids-bionic-options-min-word-len-help = Las palabras más cortas que esto se dejan tal cual.
setting-reading-aids-bionic-options-skip-numbers = Biónica omite números
setting-reading-aids-bionic-options-skip-numbers-help = Dejar tal cual las palabras con dígitos.
setting-reading-aids-bionic-options-skip-urls = Biónica omite direcciones
setting-reading-aids-bionic-options-skip-urls-help = Dejar tal cual las direcciones web y de correo.
setting-reading-aids-bionic-options-skip-code = Biónica omite código
setting-reading-aids-bionic-options-skip-code-help = Dejar tal cual el código.
setting-reading-aids-spacing-line-height = Altura de línea
setting-reading-aids-spacing-line-height-help = Altura de línea en múltiplos del tamaño de fuente; el valor de WCAG es 1,5.
setting-reading-aids-spacing-paragraph-spacing = Espaciado de párrafo
setting-reading-aids-spacing-paragraph-spacing-help = Espacio después de cada párrafo, en múltiplos del tamaño de fuente.
setting-reading-aids-spacing-letter-spacing = Espaciado de letras
setting-reading-aids-spacing-letter-spacing-help = Espacio extra entre letras, en múltiplos del tamaño de fuente.
setting-reading-aids-spacing-word-spacing = Espaciado de palabras
setting-reading-aids-spacing-word-spacing-help = Espacio extra entre palabras, en múltiplos del tamaño de fuente.
setting-reading-aids-font-family = Fuente
setting-reading-aids-font-family-help = La fuente de lectura de la interfaz gráfica; se puede escribir cualquier familia instalada.
choice-reading-aids-font-family-system-ui = la fuente del sistema
choice-reading-aids-font-family-sans = sans serif
choice-reading-aids-font-family-serif = serif
choice-reading-aids-font-family-monospace = monoespaciada
choice-reading-aids-font-family-atkinson = Atkinson Hyperlegible
choice-reading-aids-font-family-opendyslexic = OpenDyslexic
choice-reading-aids-font-family-lexend = Lexend
setting-reading-aids-font-size-pt = Tamaño de fuente
setting-reading-aids-font-size-pt-help = El tamaño de fuente de la interfaz gráfica.
setting-reading-aids-font-weight = Grosor de fuente
setting-reading-aids-font-weight-help = 400 es normal, 700 negrita.
setting-reading-aids-font-fetch-missing = Ofrecer fuentes que faltan
setting-reading-aids-font-fetch-missing-help = Ofrecer descargar una fuente de lectura que no está instalada, tras preguntar.
setting-reading-aids-ruler-mode = Regla de lectura
setting-reading-aids-ruler-mode-help = Marcar la línea actual, o una banda de líneas.
choice-reading-aids-ruler-mode-off = desactivada
choice-reading-aids-ruler-mode-current-line = línea actual
choice-reading-aids-ruler-mode-ruler = regla
setting-reading-aids-ruler-scope = La regla cubre
setting-reading-aids-ruler-scope-help = Una fila ajustada, o la línea completa.
choice-reading-aids-ruler-scope-row = una fila
choice-reading-aids-ruler-scope-line = la línea completa
setting-reading-aids-ruler-rows-above = Filas de la regla arriba
setting-reading-aids-ruler-rows-above-help = Filas de la banda por encima de la actual.
setting-reading-aids-ruler-rows-below = Filas de la regla abajo
setting-reading-aids-ruler-rows-below-help = Filas de la banda por debajo de la actual.
setting-reading-aids-ruler-mask-outside = Máscara de la regla
setting-reading-aids-ruler-mask-outside-help = Atenuar las filas fuera de la banda.
setting-reading-aids-syllables = Sílabas
setting-reading-aids-syllables-help = Dibujar las palabras separadas en sílabas con un punto central; la voz no cambia.
setting-reading-aids-difficult-words = Palabras difíciles
setting-reading-aids-difficult-words-help = Subrayar las palabras poco frecuentes, y nombrarlas al moverse entre palabras con verbosidad alta.
setting-reading-aids-syllable-options-separator = Separador de sílabas
setting-reading-aids-syllable-options-separator-help = Qué se dibuja entre sílabas.
setting-reading-aids-syllable-options-left-min = Primer corte de sílaba
setting-reading-aids-syllable-options-left-min-help = Mínimo de letras antes del primer corte.
setting-reading-aids-syllable-options-right-min = Último corte de sílaba
setting-reading-aids-syllable-options-right-min-help = Mínimo de letras después del último corte.
setting-reading-aids-syllable-options-min-word-len = Palabra más corta para sílabas
setting-reading-aids-syllable-options-min-word-len-help = Las palabras más cortas que esto nunca se dividen.
setting-reading-aids-syllable-options-skip-urls = Sílabas omiten direcciones
setting-reading-aids-syllable-options-skip-urls-help = Dejar tal cual las direcciones web y de correo.
setting-reading-aids-syllable-options-skip-code = Sílabas omiten código
setting-reading-aids-syllable-options-skip-code-help = Dejar tal cual el código.
setting-preview-auto-reload = Recargar la vista previa
setting-preview-auto-reload-help = Recargar la vista previa del navegador después de cada guardado, mediante un pequeño servidor en esta computadora únicamente.
setting-preview-live = Vista previa en vivo
setting-preview-live-help = Con la recarga activada, recargar también al hacer una pausa al escribir.
setting-lexicon-glossary = Glosario
setting-lexicon-glossary-help = Su propio glosario, consultado antes que el diccionario: líneas término: definición, o el JSON de Star. Sin definir usa glossary.txt en la carpeta de configuración.
setting-lexicon-data-file = Archivo del diccionario
setting-lexicon-data-file-help = El diccionario para definir palabras, lexicon-en.twlex. Sin definir busca junto al programa.
setting-stats-enabled = Estadísticas de lectura
setting-stats-enabled-help = Contar el tiempo leído en voz alta, el punto más lejano y las sesiones de cada documento.
setting-interface-language = Idioma de la interfaz
setting-interface-language-help = El idioma de las palabras propias de textweaver, cambiado al instante. La voz lo sigue cuando el motor tiene una para ese idioma; si no, la voz se mantiene.
choice-interface-language-en = English
choice-interface-language-es = Español
choice-interface-language-fr = Français
choice-interface-language-de = Deutsch
choice-interface-language-pt = Português
choice-interface-language-ar = العربية
choice-interface-language-en-xa = prueba: acentuado
choice-interface-language-ar-xb = prueba: de derecha a izquierda
setting-interface-rtl = Presentación de derecha a izquierda
setting-interface-rtl-help = Si el lector del terminal reordena el texto de derecha a izquierda para mostrarlo: automático lo deja a los terminales que ya lo hacen por sí solos. La voz y el lector de pantalla siempre reciben el texto en el orden de lectura.
choice-interface-rtl-auto = automático
choice-interface-rtl-on = activado
choice-interface-rtl-off = desactivado
setting-gui-announce = Anuncios
setting-gui-announce-help = Cómo llegan los mensajes de la ventana al lector de pantalla, a partir del próximo inicio: una región activa, o notificaciones de UI Automation (solo Windows).
choice-gui-announce-live = región activa
choice-gui-announce-uia = notificaciones de UI Automation

## Units, said after a number.

settings-unit-words-per-minute = palabras por minuto
settings-unit-percent = por ciento
settings-unit-semitones = semitonos
settings-unit-milliseconds = milisegundos
settings-unit-words = palabras
settings-unit-times = veces
settings-unit-places = lugares
settings-unit-columns = columnas
settings-unit-lines = líneas
settings-unit-seconds = segundos
settings-unit-steps = pasos
settings-unit-megabytes = megabytes
settings-unit-files = archivos
settings-unit-letters = letras
settings-unit-points = puntos
settings-unit-rows = filas

## Settings sections.

section-speech = Voz
section-highlight = Resaltado
section-normalization = Cómo se habla el texto
section-reading = Lectura
section-display = Pantalla
section-editing = Edición
section-library = Biblioteca
section-keyboard = Teclado
section-accessibility = Accesibilidad
section-export = Exportar
section-reading-aids = Ayudas de lectura
section-preview = Vista previa
section-lexicon = Definir palabra
section-stats = Estadísticas de lectura
section-interface = Interfaz
section-gui = Ventana

## Edit mode: entering, leaving, saving, and typing.

# $key makes a new document.
edit-no-document = No hay ningún documento para editar. Pulse { $key } para uno nuevo.
# $line is the line at the caret, as echoed.
edit-mode-on-brief = Modo de edición activado. { $line }
# $save and $finish are the keys that save and leave edit mode; $line is
# the line at the caret.
edit-mode-on = Modo de edición activado. Guardar: { $save }. Terminar: { $finish }. { $line }
# Keep the letters s, d and c: they are the keys that answer.
edit-unsaved-question = { $title } tiene cambios sin guardar. ¿Guardar, descartar o cancelar? Pulse s, d o c, o Arriba y Abajo e Intro. Escape cancela.
edit-save-changes-title = ¿Guardar los cambios en { $title }?
edit-choice-save = Guardar y continuar
edit-choice-discard = Descartar los cambios
edit-choice-cancel = Cancelar, seguir editando
edit-save-failed = No se pudo guardar: { $error }. Aún editando.
# The Save As prompt; $path is the suggested file.
edit-save-as-label = Guardar como, Intro para { $path }
# $name is a file name. Keep the letters y and n.
edit-file-exists-question = { $name } ya existe. ¿Reemplazarlo? y o n.
edit-mode-off = Modo de edición desactivado.
edit-mode-off-discarded = Cambios descartados. Modo de edición desactivado.
# The title of a new, unsaved document.
edit-untitled = Sin título
edit-new-document = Documento nuevo listo para editar.
# $key turns on edit mode.
edit-nothing-to-save = Nada que guardar. Active el modo de edición con { $key } para hacer cambios.
# $key turns on edit mode; $what is what the user tried to do.
edit-not-editing =
    { $what ->
        [type] Active el modo de edición con { $key } para escribir.
        [change-text] Active el modo de edición con { $key } para cambiar el texto.
        [delete-text] Active el modo de edición con { $key } para eliminar texto.
        [undo] Active el modo de edición con { $key } para deshacer.
        [redo] Active el modo de edición con { $key } para rehacer.
        [replace-text] Active el modo de edición con { $key } para reemplazar texto.
        [insert] Active el modo de edición con { $key } para insertar en el texto.
        [cut-text] Active el modo de edición con { $key } para cortar texto.
        [move-cells] Active el modo de edición con { $key } para moverse entre celdas de la tabla.
        [delete-words] Active el modo de edición con { $key } para eliminar palabras.
        [paste] Active el modo de edición con { $key } para pegar.
        [citation] Active el modo de edición con { $key } para insertar una cita.
        [bibliography] Active el modo de edición con { $key } para insertar una bibliografía.
       *[format] Active el modo de edición con { $key } para dar formato al texto.
    }
# A paste: $n characters.
edit-pasted =
    { $n ->
        [one] Se pegó 1 carácter.
       *[other] Se pegaron { $n } caracteres.
    }
# $start is how the pasted text starts.
edit-pasted-start =
    { $n ->
        [one] Se pegó 1 carácter: { $start }
       *[other] Se pegaron { $n } caracteres: { $start }
    }
edit-insert-failed = No se pudo insertar: { $error }
# $start and $end are character positions, $len the text's length.
edit-range-out-of-text = No se pueden cambiar los caracteres { $start } a { $end }: el texto tiene { $len }.
edit-change-failed = No se pudo cambiar el texto: { $error }
edit-delete-failed = No se pudo eliminar: { $error }
edit-list-ended = Lista terminada.
# Said when Enter continues a bulleted list.
edit-bullet = viñeta
edit-table-divider = separador de encabezado de tabla
edit-end-of-line-stop = Final de línea.
edit-start-of-line-stop = Principio de línea.
edit-end-of-line-content = final de línea

## Edit mode: formatting, undo, tables, images, and replace.

# $what names the formatting command; $level is a heading level, and
# $cols and $rows a table's size.
edit-format-done =
    { $what ->
        [bold] Negrita.
        [italic] Cursiva.
        [underline] Subrayado.
        [strikethrough] Tachado.
        [code] Código.
        [code-block] Bloque de código.
        [link] Enlace.
        [bulleted-list] Lista con viñetas.
        [numbered-list] Lista numerada.
        [block-quote] Cita en bloque.
        [horizontal-rule] Línea horizontal insertada.
        [table-row] Fila de tabla agregada.
        [heading-level] Encabezado de nivel { $level }.
        [table] Se insertó una tabla, { $cols } columnas por { $rows } filas.
       *[heading] Encabezado.
    }
# The command toggled its markup off.
edit-format-removed =
    { $what ->
        [bold] Negrita quitada.
        [italic] Cursiva quitada.
        [underline] Subrayado quitado.
        [strikethrough] Tachado quitado.
        [code] Código quitado.
        [code-block] Bloque de código quitado.
        [link] Enlace quitado.
        [bulleted-list] Lista con viñetas quitada.
        [numbered-list] Lista numerada quitada.
        [block-quote] Cita en bloque quitada.
        [horizontal-rule] Línea horizontal insertada quitada.
        [table-row] Fila de tabla agregada quitada.
        [heading-level] Encabezado de nivel { $level } quitado.
        [table] Tabla insertada, { $cols } columnas por { $rows } filas, quitada.
       *[heading] Encabezado quitado.
    }
edit-format-unchanged =
    { $what ->
        [bold] Negrita: sin cambios.
        [italic] Cursiva: sin cambios.
        [underline] Subrayado: sin cambios.
        [strikethrough] Tachado: sin cambios.
        [code] Código: sin cambios.
        [code-block] Bloque de código: sin cambios.
        [link] Enlace: sin cambios.
        [bulleted-list] Lista con viñetas: sin cambios.
        [numbered-list] Lista numerada: sin cambios.
        [block-quote] Cita en bloque: sin cambios.
        [horizontal-rule] Línea horizontal insertada: sin cambios.
        [table-row] Fila de tabla agregada: sin cambios.
        [heading-level] Encabezado de nivel { $level }: sin cambios.
        [table] Tabla insertada, { $cols } columnas por { $rows } filas: sin cambios.
       *[heading] Encabezado: sin cambios.
    }
# Added after a formatting message; $text is the start of the selection.
edit-format-selected = Seleccionado: { $text }
edit-heading-level-now = Encabezado de nivel { $level }.
# $line is the line at the caret after the undo or redo.
edit-undo-redo =
    { $what ->
        [undo] Deshacer.
       *[redo] Rehacer.
    }
edit-undo-redo-line =
    { $what ->
        [undo] Deshacer. { $line }
       *[redo] Rehacer. { $line }
    }
edit-nothing-to-undo = Nada que deshacer.
edit-nothing-to-redo = Nada que rehacer.
edit-not-a-table-size = No es un tamaño de tabla: { $text }. Escriba columnas y filas, por ejemplo 3 por 2.
# $name is the image's file name.
edit-image-inserted = Se insertó la imagen { $name }. Su descripción está seleccionada; escriba para reemplazarla.
edit-image-failed = No se pudo insertar la imagen: { $error }.
# $query is the text to find.
edit-no-matches = Sin coincidencias para { $query }.
# $n matches of $query were found; the replacement is asked next.
edit-replace-with =
    { $n ->
        [one] 1 coincidencia de { $query }. ¿Reemplazar con?
       *[other] { $n } coincidencias de { $query }. ¿Reemplazar con?
    }

## Edit mode: autosave and recovering unsaved work.

edit-recovery-write-failed = No se pudo escribir la copia de recuperación: { $error }. Guarde pronto; { -brand } lo seguirá intentando.
edit-recovery-writing-again = La copia de recuperación se está escribiendo de nuevo.
# $title is the document; $when is how long ago its work was saved.
edit-recovery-offer = { -brand } se cerró con cambios sin guardar en { $title }, guardados { $when }. ¿Recuperarlos ahora? Arriba y Abajo eligen, Intro confirma.
edit-recovery-title = ¿Recuperar el trabajo sin guardar en { $title }?
edit-recovery-yes = Sí, recuperar { $title } y seguir editando
edit-recovery-no = No, descartar los cambios sin guardar
edit-recovery-discarded = Se descartaron los cambios sin guardar en { $title }.
edit-recovered = Se recuperó el trabajo sin guardar en { $title }. Recuerde guardar.
edit-recovery-postponed = Recuperación aplazada. El trabajo sin guardar se ofrecerá de nuevo la próxima vez.

## Find and replace, one match at a time.

# $title is replace-match-title. Keep the letters r, s and a: they are the
# keys that answer.
replace-match-question = { $title }. Pulse r para reemplazar, s para omitir, a para reemplazar todo, Escape para detener.
# The replace list's title when no match is being asked about.
replace-title = Reemplazar
# $n is this match's number, $total the number of matches, $line the line
# number, and $context the text of that line.
replace-match-title = Coincidencia { $n } de { $total }, línea { $line }: { $context }
replace-item-this = Reemplazar esta
replace-item-skip = Omitir esta
replace-item-rest = Reemplazar todas las demás
# $state is common-on or common-off.
replace-item-match-case = Distinguir mayúsculas: { $state }
replace-item-whole-words = Solo palabras completas: { $state }
replace-failed = No se pudo reemplazar: { $error }
# Said after switching match case; $state is common-on or common-off, and
# $n is the number of matches now.
replace-match-case-now =
    { $n ->
        [one] Distinguir mayúsculas { $state }. 1 coincidencia.
       *[other] Distinguir mayúsculas { $state }. { $n } coincidencias.
    }
replace-whole-words-now =
    { $n ->
        [one] Solo palabras completas { $state }. 1 coincidencia.
       *[other] Solo palabras completas { $state }. { $n } coincidencias.
    }
# $query is the text that was searched for.
replace-no-matches = Sin coincidencias para { $query }.
replace-replaced =
    { $n ->
        [one] Se reemplazó 1 coincidencia.
       *[other] Se reemplazaron { $n } coincidencias.
    }
replace-replaced-skipped = Se reemplazaron { $n }, se omitieron { $skipped }.
replace-stopped = Detenido. Se reemplazaron { $n }, se omitieron { $skipped }.

## Saving in the background.

writes-still-saving = Todavía guardando. Espere, por favor.
writes-not-written-in-time = Algunos cambios no se pudieron escribir a tiempo: el disco no responde.
# $error is the system's reason.
writes-save-failed = No se pudo guardar: { $error }. Aún editando.
# $name is the bookmark's name, $pct where it is.
writes-bookmark-set = Marcador { $name } puesto en el { $pct } por ciento.
writes-bookmark-not-saved = El marcador { $name } está puesto por ahora, pero no se pudo guardar: { $error }.
writes-recovery-copy-failed = No se pudo escribir la copia de recuperación: { $error }. Guarde pronto; { -brand } lo seguirá intentando.
writes-recovery-copy-resumed = La copia de recuperación se está escribiendo de nuevo.
# $name is the saved file's name.
writes-saved = Se guardó { $name }. Aún editando.

## Files changed on disk. $name is a file name. Keep the letters y and
## n: they are the keys that answer.

disk-replace-question = { $name } ya existe. ¿Reemplazarlo? y o n.
# A prompt label, also said with a full stop after it.
disk-not-replaced = No se reemplazó. Escriba otro nombre
# $key is the key for Save As.
disk-not-saved = No se guardó. Aún editando. Guardar Como, { $key }, conserva ambas versiones.
disk-kept-open-version = Se conservó la versión abierta.
disk-overwrite-question = { $name } cambió en el disco desde que lo abrió. ¿Guardar sobre esos cambios? y o n.
disk-reload-question = { $name } cambió en el disco. ¿Volver a cargarlo? y o n.

## Marks found again after a file changed outside textweaver.

relocate-reading-position = su posición de lectura
relocate-bookmarks =
    { $n ->
        [one] 1 marcador
       *[other] { $n } marcadores
    }
relocate-notes =
    { $n ->
        [one] 1 nota
       *[other] { $n } notas
    }
relocate-highlights =
    { $n ->
        [one] 1 resaltado
       *[other] { $n } resaltados
    }
# Lists of relocate-* items: "a and b", and "a, b, and c", where $rest is
# every item but the last, joined by commas.
relocate-join-two = { $a } y { $b }
relocate-join-more = { $rest } y { $last }
# $items is a list of the items above; $n how many marks it counts in all.
relocate-moved =
    { $n ->
        [one] { $items } se movió para coincidir
       *[other] { $items } se movieron para coincidir
    }
relocate-lost =
    { $n ->
        [one] { $items } no se pudo encontrar y está marcado
       *[other] { $items } no se pudieron encontrar y están marcados
    }
# $clauses are relocate-moved and relocate-lost, joined by a comma.
relocate-changed = El archivo cambió; { $clauses }.

## New documents from templates.

# The built-in templates' names.
templates-essay = Ensayo
templates-report = Informe
templates-notes = Notas
# One of the user's own templates in the list; $name is its file name.
templates-yours = { $name }, su plantilla
# $folder is where the user's own templates go.
templates-intro =
    { $n ->
        [one] Documento nuevo a partir de una plantilla, 1 plantilla. Intro elige. Sus propias plantillas van en { $folder }.
       *[other] Documento nuevo a partir de una plantilla, { $n } plantillas. Intro elige. Sus propias plantillas van en { $folder }.
    }
# The title given when none is typed.
templates-untitled = Sin título
# $template is the template's name, $title the document's, $date today's date (2026-09-26).
templates-created = Documento nuevo a partir de la plantilla { $template }: { $title }. Fechado el { $date }. El cursor está donde empieza el texto. Recuerde guardar.

## Markdown structure said in edit mode, before a line's text or as it
## is typed.

mdline-heading-level = encabezado de nivel { $level }
mdline-bullet = viñeta
# A numbered list item; $n is its number.
mdline-item = elemento { $n }
# $item is mdline-bullet or mdline-item.
mdline-task-done = { $item }, tarea hecha
mdline-task-not-done = { $item }, tarea sin hacer
mdline-table-row = fila de tabla
mdline-quote = cita
mdline-code-fence = valla de código
mdline-task = tarea
# Said as "1. " is typed at the start of a line; $n is the number as typed.
mdline-numbered-item = elemento numerado { $n }

## Moving through tables by row and cell. $dir is next (forward) or
## previous (backward).

tables-not-in-table = No está en una tabla.
tables-edge-of-table =
    { $dir ->
        [next] Final de la tabla.
       *[previous] Principio de la tabla.
    }
tables-edge-of-row =
    { $dir ->
        [next] Final de la fila.
       *[previous] Principio de la fila.
    }
# $cell is the cell's text, after its column header and a colon when it has one.
tables-header-row = Fila de encabezado, { $cell }
tables-row = Fila { $row }, { $cell }
# High verbosity: $message is what the move said, then where it is.
tables-with-position = { $message }. Fila { $row } de { $rows }, columna { $col } de { $cols }
# Say Position in a table.
tables-position = Tabla, fila { $row } de { $rows }, columna { $col } de { $cols }.

## Authoring quick wins: word count, links, clipboard, table cells,
## deleting words, and cycling settings.

# Code block languages said with ordinary words; proper names such as
# Python are not translated.
authoring-language-jsx = JavaScript con JSX
authoring-language-tsx = TypeScript con JSX
authoring-language-shell = shell
authoring-language-batch = batch de Windows
authoring-language-c-header = cabecera de C
authoring-language-cpp = C más más
authoring-language-csharp = C almohadilla
authoring-language-diff = diff
authoring-language-plain-text = texto sin formato
authoring-grammar-not-in-build = La comprobación de gramática no está en esta compilación.
# $count is $n with thousands separators.
authoring-word-count-selection =
    { $n ->
        [one] 1 palabra en la selección.
       *[other] { $count } palabras en la selección.
    }
authoring-word-count-document =
    { $n ->
        [one] 1 palabra en el documento.
       *[other] { $count } palabras en el documento.
    }
# $text is the link's text.
authoring-link-no-address = El enlace { $text } no tiene dirección.
authoring-link-address = Dirección del enlace: { $url }
authoring-link-named-address = Enlace { $text }, dirección: { $url }
authoring-no-link = No hay ningún enlace en el cursor.
authoring-typing-echo =
    { $echo ->
        [characters-and-words] Eco de escritura: caracteres y palabras.
        [characters] Eco de escritura: caracteres.
        [words] Eco de escritura: palabras.
       *[none] Eco de escritura: ninguno.
    }
authoring-nothing-to-copy = Nada seleccionado para copiar.
# $text is the first words of what was copied.
authoring-copied = Copiado: { $text }
authoring-copied-sentence = Se copió la oración: { $text }
authoring-nothing-to-cut = Nada seleccionado para cortar.
authoring-cut = Cortado: { $text }
authoring-not-in-table = No está en una tabla.
# $dir is next (moving forward) or previous.
authoring-table-edge =
    { $dir ->
        [next] Final de la tabla.
       *[previous] Principio de la tabla.
    }
# A column with no header text.
authoring-table-column = columna { $n }
# Moving into a new row: $header is the column's header, $content the cell.
authoring-table-cell-row = Fila { $row }. { $header }: { $content }
authoring-nothing-to-select = Nada que seleccionar.
authoring-selected-all =
    { $n ->
        [one] Se seleccionó todo, 1 palabra.
       *[other] Se seleccionó todo, { $count } palabras.
    }
authoring-space-deleted = Espacio eliminado.
# $text is the word deleted.
authoring-deleted = { $text } eliminada.
# $key is the terminal's own paste key.
authoring-nothing-copied = Todavía no se ha copiado nada en { -brand }. Use el pegado de su terminal, por ejemplo { $key }.
authoring-verbosity =
    { $level ->
        [low] Verbosidad: baja.
        [high] Verbosidad: alta.
       *[normal] Verbosidad: normal.
    }
authoring-punctuation =
    { $level ->
        [none] Puntuación: ninguna.
        [all] Puntuación: toda.
       *[some] Puntuación: algo.
    }

## Markdown lint (edit mode). A problem is said after "Lint: ".

lint-heading-level = encabezado de nivel { $level } después de nivel { $prev }; use el nivel { $use }.
# $reference is the link reference's name.
lint-link-reference = la referencia de enlace { $reference } no tiene definición.
lint-bare-url = dirección web suelta; póngala entre corchetes angulares o conviértala en un enlace con nombre.
# $marker and $used are bullet names: lint-marker-dash and the others.
lint-list-marker = marcador de lista { $marker }; esta lista usa { $used }.
lint-marker-dash = guion
lint-marker-star = asterisco
lint-marker-plus = signo más
lint-marker-other = otro
# $n is how many tabs and spaces there are.
lint-trailing-tabs-empty-line = tabuladores o espacios en una línea vacía.
lint-trailing-tabs-line-end = tabuladores o espacios al final de la línea.
lint-trailing-spaces-empty-line =
    { $n ->
        [one] 1 espacio en una línea vacía.
       *[other] { $n } espacios en una línea vacía.
    }
lint-trailing-spaces-line-end =
    { $n ->
        [one] 1 espacio al final de la línea.
       *[other] { $n } espacios al final de la línea.
    }
# $key turns on edit mode.
lint-not-editing = Lint comprueba el Markdown que escribe. Active primero el modo de edición con { $key }.
lint-not-markdown = Lint comprueba Markdown, y este documento no es Markdown.
lint-none = Sin problemas de lint.
# $count is $n with thousands separators.
lint-no-more =
    { $n ->
        [one] No hay más problemas de lint. 1 problema de lint en total.
       *[other] No hay más problemas de lint. { $count } problemas de lint en total.
    }
lint-no-earlier =
    { $n ->
        [one] No hay problemas de lint anteriores. 1 problema de lint en total.
       *[other] No hay problemas de lint anteriores. { $count } problemas de lint en total.
    }
# $message is one of the problems above.
lint-said = Lint: { $message }
# Added at high verbosity.
lint-line = Línea { $line }.

## Grammar checking (Harper). $message is Harper's own message, in English.

# $words are the words the problem is about.
grammar-said = Gramática: { $message } Las palabras: { $words }.
# Said after grammar-said when the first fix removes the words.
grammar-fix-remove = Corrección: quitarlas.
grammar-fix = Corrección: { $fix }.
# A fix in the fixes list that removes the words.
grammar-remove-the-words = Quitar las palabras
grammar-none = No se encontraron problemas de gramática.
# $count is $n with thousands separators.
grammar-no-more =
    { $n ->
        [one] No hay más problemas de gramática. 1 problema de gramática en total.
       *[other] No hay más problemas de gramática. { $count } problemas de gramática en total.
    }
grammar-no-earlier =
    { $n ->
        [one] No hay problemas de gramática anteriores. 1 problema de gramática en total.
       *[other] No hay problemas de gramática anteriores. { $count } problemas de gramática en total.
    }
# $key opens the fixes list.
grammar-lists-fixes = { $key } lista las correcciones.
# Added at high verbosity.
grammar-line = Línea { $line }.
# $described is grammar-said (and its fix) without the last full stop.
grammar-no-fix = { $described } No hay ninguna corrección que ofrecer.
grammar-fixes =
    { $n ->
        [one] { $words }: 1 corrección.
       *[other] { $words }: { $n } correcciones.
    }
grammar-fixes-edit =
    { $n ->
        [one] { $words }: 1 corrección. Intro hace el cambio.
       *[other] { $words }: { $n } correcciones. Intro hace el cambio.
    }
grammar-left-as-is = Se dejó tal cual.
# $fix is the fix chosen; $key turns on edit mode.
grammar-fix-not-editing = { $fix }. Active el modo de edición con { $key } para cambiar el texto.
grammar-removed = Quitado.
grammar-changed = Cambiado a { $fix }.
grammar-change-failed = No se pudo cambiar el texto: { $error }

## Spell checking.

# Said for an apostrophe when a word is spelled out letter by letter.
spell-apostrophe = apóstrofo
spell-not-available = La comprobación ortográfica no está disponible: esta compilación no tiene lista de palabras.
spell-none-found = No se encontraron errores ortográficos.
# $count is $n with thousands separators.
spell-no-more =
    { $n ->
        [one] No hay más errores. 1 posible error ortográfico en total.
       *[other] No hay más errores. { $count } posibles errores ortográficos en total.
    }
spell-no-earlier =
    { $n ->
        [one] No hay errores anteriores. 1 posible error ortográfico en total.
       *[other] No hay errores anteriores. { $count } posibles errores ortográficos en total.
    }
# Added at high verbosity.
spell-line = Línea { $line }.
spell-no-misspelled-word = No hay ninguna palabra mal escrita en el cursor.
# $word is the misspelled word; $n how many suggestions follow.
spell-suggestions =
    { $n ->
        [0] { $word }: sin sugerencias.
        [one] { $word }: 1 sugerencia.
       *[other] { $word }: { $n } sugerencias.
    }
spell-suggestions-edit =
    { $n ->
        [0] { $word }: sin sugerencias. Intro reemplaza la palabra.
        [one] { $word }: 1 sugerencia. Intro reemplaza la palabra.
       *[other] { $word }: { $n } sugerencias. Intro reemplaza la palabra.
    }
# $word is the suggestion chosen; $key turns on edit mode.
spell-replace-not-editing = { $word }. Active el modo de edición con { $key } para cambiar el texto.
spell-replaced = Reemplazado con { $word }.
spell-replace-failed = No se pudo reemplazar: { $error }
spell-left-as-is = Se dejó tal cual.
spell-added-for-session = Se agregó { $word } a su lista de palabras para esta sesión.
spell-added = Se agregó { $word } a su lista de palabras.
spell-save-failed = No se pudo guardar su lista de palabras: { $error }
# After a save; $count is $n with thousands separators.
spell-count =
    { $n ->
        [0] Sin errores ortográficos.
        [one] 1 posible error ortográfico.
       *[other] { $count } posibles errores ortográficos.
    }

## The terminal reader's startup.

# $wanted is the speech backend asked for, $backend the one used instead.
tui-setup-backend-unavailable = El motor de voz { $wanted } no está disponible; se usa { $backend }.
# Shown inside tui-setup-speech-failed as its $error.
tui-setup-backend-not-built = el motor { $backend } no está compilado
tui-setup-speech-failed = No se pudo iniciar la voz ({ $error }); ejecutando en silencio.
tui-setup-cannot-save = No se puede guardar la configuración ni las posiciones: { $error }.
tui-setup-keymap-ignored = Se ignoró el archivo de teclas: { $error }.
# The first-run welcome. Each value names the key for an action: $play
# reads and pauses, $stop stops, $heading moves to the next heading,
# $help opens the help, $quit quits.
tui-setup-welcome = Bienvenido a { -brand }. { $play } lee en voz alta y pausa, { $stop } detiene, { $heading } va al encabezado siguiente, { $help } abre la ayuda, y { $quit } sale.
# Said at startup without a document. $open, $new, and $help name the
# keys for Open, New Document, and Help.
tui-setup-no-document = No hay ningún documento abierto. Pulse { $open } para abrir uno, { $new } para uno nuevo, o { $help } para ayuda.

## The terminal reader's launch.

# $name is the file asked for on the command line; $error says why.
tui-could-not-open = No se pudo abrir { $name }: { $error }

## Copying in the terminal reader.

tui-clip-system = Se copió con el portapapeles del sistema, porque este terminal no puede recibir texto copiado.
# $error is why: tui-clip-not-available, tui-clip-not-built, or the
# system's own message.
tui-clip-failed = No se pudo copiar con el portapapeles del sistema: { $error }. Se envió al terminal en su lugar.
tui-clip-not-available = el portapapeles del sistema no está disponible
tui-clip-not-built = esta compilación no tiene portapapeles del sistema

## The terminal reader's screen.

# The title line's start; $title is the document's title or
# tui-title-no-document.
tui-title = { -brand }: { $title }
tui-title-no-document = sin documento
# The screen without a document. $keys names the keys for the action.
tui-empty-no-document = No hay ningún documento abierto.
tui-empty-open = Abrir uno: { $keys }.
tui-empty-help = Ayuda: { $keys }.
tui-empty-quit = Salir: { $keys }.
# The key hints while a yes-or-no question waits. y, n, and a are the
# answer keys the reader takes; $escape names the Escape key.
tui-hints-confirm = y sí  n o a no  { $escape } no
# Key hint labels, each shown after its key on the bottom line.
tui-hint-play = reproducir
tui-hint-sentence = oración
tui-hint-faster = más rápido
tui-hint-slower = más lento
tui-hint-close-rsvp = cerrar RSVP
tui-hint-quit = salir
tui-hint-save = guardar
tui-hint-finish = terminar
tui-hint-undo = deshacer
tui-hint-bold = negrita
tui-hint-heading = encabezado
tui-hint-commands = comandos
tui-hint-next-line = línea siguiente
tui-hint-previous-line = línea anterior
tui-hint-again = otra vez
tui-hint-read-on = seguir leyendo
tui-hint-leave = salir
tui-hint-paragraph = párrafo
tui-hint-find = buscar
tui-hint-mark = marcar
tui-hint-lines = líneas
tui-hint-keys = teclas
# The list overlay's border: $n is the focused item's number, $count
# the number of items.
tui-list-title = { $title } ({ $n } de { $count })

text-summary =
    { $change ->
        [selected] { $count } caracteres seleccionados
        [unselected] { $count } caracteres deseleccionados
        [copied] { $count } caracteres copiados
        [cut] { $count } caracteres cortados
       *[deleted] { $count } caracteres eliminados
    }
text-summary-range =
    { $change ->
        [selected] { $count } caracteres seleccionados, desde { $first } hasta { $last }
        [unselected] { $count } caracteres deseleccionados, desde { $first } hasta { $last }
        [copied] { $count } caracteres copiados, desde { $first } hasta { $last }
        [cut] { $count } caracteres cortados, desde { $first } hasta { $last }
       *[deleted] { $count } caracteres eliminados, desde { $first } hasta { $last }
    }
voice-character-keys-on = Atajos de una sola tecla activados.
voice-character-keys-off = Atajos de una sola tecla desactivados.
goto-word-start = inicio
goto-word-end = fin

language-voices-loading = La lista de voces todavía se está cargando, así que sigue hablando la voz actual.

## The window (GUI)

gui-open-title = Abrir un documento
gui-open-documents = Documentos que textweaver lee
gui-open-all-files = Todos los archivos
gui-open-no-dialog = El selector de archivos del sistema no se abrió. Escriba la ruta del documento.
