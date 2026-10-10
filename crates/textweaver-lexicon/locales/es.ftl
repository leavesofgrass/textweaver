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
define-dictionary-damaged = No se pudo leer el archivo del diccionario: { $error } Solo se busca en su glosario.
define-glossary-problem = No se pudo leer su glosario: { $error } Solo se busca en el diccionario.
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
stats-turned-off = Las estadísticas de lectura están desactivadas. Lo registrado se conserva.

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
stats-clear-item = Eliminar las estadísticas de lectura
stats-clear-failed = No se pudieron eliminar las estadísticas de lectura: { $error }. Compruebe que se puede escribir en la carpeta de datos.
stats-off-cli = Las estadísticas de lectura están desactivadas: stats.enabled es false en la configuración.

## Continue reading and every computer's statistics (the sync wave, S6).

continue-title = Seguir leyendo
# $n is the number of documents listed.
continue-intro =
    { $n ->
        [one] Seguir leyendo: 1 documento, el más reciente primero.
       *[other] Seguir leyendo: { $n } documentos, los más recientes primero.
    }
continue-empty = Sin posiciones aún. Leer las guarda.
# One row, meaning first: the title, how far in, the computer, and how
# long ago (continue-ago-*).
continue-item = { $title }, { $pct } por ciento, { $device }, { $when }
continue-this-computer = este equipo
continue-ago-now = ahora mismo
continue-ago-minutes =
    { $n ->
        [one] hace 1 minuto
       *[other] hace { $n } minutos
    }
continue-ago-hours =
    { $n ->
        [one] hace 1 hora
       *[other] hace { $n } horas
    }
continue-ago-days =
    { $n ->
        [one] hace 1 día
       *[other] hace { $n } días
    }
name-continue-reading = Seguir leyendo
action-continue-reading = Seguir leyendo: los documentos de este equipo con una posición guardada, de cualquier equipo, los más recientes primero
name-add-library-folder = Añadir una carpeta a la biblioteca
action-add-library-folder = Añadir una carpeta a la biblioteca: elíjala en el explorador de archivos
name-edit-document-details = Editar detalles
action-edit-document-details = Editar los detalles del documento: título, autor, DOI e ISBN
prompt-document-details = Detalles del documento

## Edit a document's details by hand.

# $name is the document's title; said when the form opens.
details-intro = Detalles de { $name }. Tab cambia de campo, Intro guarda, Escape cancela.
details-label-title = Título
details-label-author = Autor
details-label-doi = DOI
details-label-isbn = ISBN
# A field's drawn label: its label, then $n of $total fields.
details-prompt-label = { $label }, { $n } de { $total }
# Said on moving to a field: its label and value (or nav-blank).
details-field = { $label }: { $value }
# $fields lists the fields saved, by their labels.
details-saved = Detalles guardados: { $fields }.
details-unchanged = Detalles sin cambios.
details-cancelled = Cancelado. Detalles sin cambios.
# $text is what was typed in the DOI or ISBN field.
details-not-a-doi = No es un DOI: { $text }. Corríjalo o bórrelo.
details-not-an-isbn = No es un ISBN: { $text }. Corríjalo o bórrelo.
details-no-file = Este documento no tiene archivo, así que no tiene detalles que editar.
details-save-failed = No se pudieron guardar los detalles: { $error } Inténtelo de nuevo.
# The statistics list could not wait for the sync folder.
stats-others-slow = Otros equipos omitidos: carpeta lenta.
stats-untitled = Documento sin título
# One computer's share of a document: $device, $time, $sessions.
stats-computer =
    { $sessions ->
        [one] { $device }: { $time }, 1 sesión
       *[other] { $device }: { $time }, { $sessions } sesiones
    }
stats-by-computer-off = Cada equipo: oculto. Intro lo muestra.
stats-by-computer-on = Cada equipo: visible. Intro lo oculta.

## Navigation. $dir is next or previous; $what is a kind-* or unit-*
## noun and $unit its key (heading, list-item, sentence), for languages
## whose words agree with the noun.

nav-blank = en blanco
# Said for a picture that has no description (no alternative text).
nav-no-description = sin descripción
# The document overview: the title first, then the counts. $time is overview-time.
overview-line = { $title }. Encabezados: { $headings }, tablas: { $tables }, imágenes: { $pictures }, notas al pie: { $footnotes }. { $time }
# Reading time left from the cursor at the current rate.
overview-time =
    { $minutes ->
        [0] Queda menos de un minuto.
        [one] Queda alrededor de 1 minuto.
       *[other] Quedan alrededor de { $minutes } minutos.
    }
# Said when the reading pass changes and as reading starts in a skim. $pass is a reading-pass-* name.
reading-pass-changed = Pasada: { $pass }.
reading-pass-full = texto completo
reading-pass-first-sentences = primeras oraciones
reading-pass-headings = encabezados
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
playback-speech-error = Error de voz: { $error } Vuelva a intentarlo, o use la orden Reiniciar la voz.
playback-end-of-section = Fin de la sección. { $key } para seguir.
playback-time-up =
    { $minutes ->
        [one] Se acabó el tiempo tras 1 minuto. { $key } para seguir.
       *[other] Se acabó el tiempo tras { $minutes } minutos. { $key } para seguir.
    }
playback-repeat-slower = Repitiendo más despacio, a { $rate } palabras por minuto.
playback-recall-prompt = Diga lo que recuerda de { $section }. { $key } para seguir.

## The title line and Say Status.

# The title line's position.
status-position = línea { $line } de { $lines }
status-mode = Modo { $mode }
status-modified = modificado
status-self-voicing = voz propia
status-hybrid = híbrido
status-screen-reader = modo lector de pantalla
status-rate-spoken = { $wpm } palabras por minuto
status-rate = { $wpm } ppm
# The window's status bar, last: reading time left at the current rate.
status-time-left =
    { $minutes ->
        [0] menos de un minuto restante
        [one] 1 minuto restante
       *[other] { $minutes } minutos restantes
    }
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
app-window-only = Esta orden funciona en la ventana de textweaver.
app-terminal-only = Esta orden funciona en el lector del terminal.
settings-save-failed = No se pudo guardar la configuración: { $error } Sus cambios siguen en uso hasta que salga.
settings-outside-kept = Se conservó la configuración cambiada fuera de textweaver.
edit-still-editing = Aún editando.
goto-not-a-target = No es un destino válido: { $text }. Escriba un número de línea, un porcentaje como 50%, inicio o fin.

## Opening a document.

open-opened = Se abrió { $title }.
open-resumed = Se abrió { $title }. Se reanudó en el { $pct } por ciento.
open-resumed-synced = Se abrió { $title }. Se reanudó en el { $pct } por ciento, desde otro dispositivo.

## Prompts: the label is shown and said when the prompt opens.

prompt-find = Buscar
prompt-go-to = Ir a línea, porcentaje, inicio o fin
prompt-open = Abrir archivo
prompt-command = Comando
# $label is prompt-command.
prompt-command-palette-intro = { $label }. Escriba parte de un nombre; Tab completa, Arriba y Abajo recorren las coincidencias.
prompt-save-as = Guardar como
prompt-export-as = Exportar como
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
# One line of the keyboard shortcuts list: the command's name-* (first, so
# type-ahead finds commands), its keys (inside a 40-cell Braille line), an
# action-* help, and its category-* title.
help-entry = { $name }: { $keys }. { $help }. { $category }
help-unknown-command = Comando desconocido: { $text }.
help-shortcuts-intro = Atajos de teclado, { $n } comandos. Escriba para filtrar. Avance de página pasa al grupo siguiente, F1 explica un comando, Intro lo ejecuta, Escape cierra.
help-shortcuts-title = Atajos de teclado
# The keyboard shortcuts list, filtered: $filter is what was typed.
help-shortcuts-title-matching = Atajos de teclado que coinciden con { $filter }
# $n commands of $total match the filter.
help-shortcuts-filter-match = { $n } de { $total } comandos coinciden.
help-shortcuts-filter-none = Ningún comando coincide con { $query }. Retroceso quita letras.
help-shortcuts-filter-cleared = Filtro borrado, { $n } comandos.
# Moving into a group of the keyboard shortcuts list: its name, its
# size, then the row ($item, with its place in the list).
help-shortcuts-group-item =
    { $group }, { $n ->
        [one] 1 comando
       *[other] { $n } comandos
    }. { $item }
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
help-access-window = Quién lee: { $key } alterna entre textweaver lee en voz alta y mi lector de pantalla lee.
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
action-repeat-sentence-slower = Decir de nuevo la oración en el cursor más despacio y volver a la velocidad habitual
action-rsvp-toggle = Mostrar u ocultar RSVP: una palabra a la vez, desde el cursor
action-rsvp-play-pause = Iniciar o pausar RSVP
action-rsvp-faster = RSVP más rápido
action-rsvp-slower = RSVP más lento
action-rsvp-position-next = Mover la palabra de RSVP al siguiente lugar de la pantalla
action-reading-level = Decir el nivel de lectura del documento o de la selección
action-document-overview = Decir el título del documento, cuántos encabezados, tablas, imágenes y notas al pie tiene, y unos cuántos minutos quedan
action-reading-pass = Cambiar lo que dice la lectura: el texto completo, la primera oración de cada párrafo con los encabezados, o solo los encabezados
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
action-search-options = Elegir cómo buscan Buscar y Reemplazar: mayúsculas, palabras completas, expresión regular, a través de líneas
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
action-self-test = Ponerse a prueba con las notas y resaltados: Intro muestra cada respuesta
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
action-export-html = Exportar el documento como página web (HTML), eligiendo dónde guardarlo
action-export-pdf = Exportar el documento como PDF etiquetado, eligiendo dónde guardarlo
action-export-docx = Exportar el documento como archivo de Word (DOCX), eligiendo dónde guardarlo
action-export-epub = Exportar el documento como libro EPUB, eligiendo dónde guardarlo
action-export-brf = Exportar el documento como braille (BRF), eligiendo dónde guardarlo
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
action-paste = Pegar el portapapeles; el texto con formato de un navegador o un procesador de textos se convierte en Markdown
action-paste-plain-text = Pegar el portapapeles como texto sin formato, sin conservar nada de su formato
action-context-menu = Abrir el menú contextual: cortar, copiar, pegar y los comandos para donde está el cursor
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
action-text-larger = Agrandar el texto del documento
action-text-smaller = Reducir el texto del documento
action-text-size-reset = Devolver el texto del documento a su tamaño normal
action-choose-font = Elegir la fuente del texto del documento
action-contents-panel = Mostrar el panel Contenido junto al documento e ir a él, o cerrarlo desde dentro: Intro va a un encabezado
action-notes-panel = Mostrar el panel Notas junto al documento e ir a él, o cerrarlo desde dentro: Intro va a una nota
action-toggle-header = Mostrar u ocultar la cabecera, la barra de comandos sobre el documento
action-toggle-toolbar = Mostrar u ocultar la barra de herramientas, la barra de los botones de lectura
action-next-region = Ir a la siguiente parte de la ventana: la cabecera, el panel, el documento o la barra de herramientas
action-previous-region = Ir a la parte anterior de la ventana
action-command-palette = Ejecutar cualquier comando por su nombre
action-settings = Abrir la configuración: cada opción con su ayuda; Izquierda y Derecha cambian un valor
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
restart-failed = No se pudo reiniciar la voz: { $error } Espere un momento y vuelva a intentarlo.
restart-start-failed = No se pudo reiniciar la voz: no se pudo iniciar.
restart-no-engine = No hay ningún motor de voz disponible; { -brand } permanece en silencio. Consulte Troubleshooting, No speech at all, en la documentación.
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

## Opening a document: failures and opening in the background.

# $name is the file's name.
opening-is-folder = { $name } es una carpeta, no un documento. Indique el nombre de un archivo dentro de ella.
# Said after "Could not open NAME:", so it starts in lower case.
opening-no-file-in = no hay ningún archivo llamado { $name } en { $folder }. Compruebe el nombre.
opening-no-file-here = no hay ningún archivo llamado { $name } aquí. Compruebe el nombre.
opening-no-permission = no tiene permiso para leerlo.
opening-damaged-rtf = no es un archivo RTF legible; puede estar dañado.
opening-damaged-odt = no es un archivo de texto OpenDocument legible; puede estar dañado.
opening-damaged-latex = no es un archivo LaTeX legible; puede estar dañado o ser demasiado grande.
opening-damaged-email = no es un mensaje de correo legible; puede estar dañado o ser demasiado grande.
opening-damaged-mhtml = no es una página web archivada legible; puede estar dañada o ser demasiado grande.
opening-pdf-password = está protegido con contraseña. Quite la contraseña en un programa de PDF y vuelva a abrirlo.
opening-old-office = es un archivo antiguo de Microsoft Office. Guárdelo en un formato más nuevo, como .docx, y abra ese.
opening-rar = es un archivo RAR, que no se abre. Extráigalo primero, o use ZIP o 7z.
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

lean-citations-not-in-build = Las citas no están en esta versión de { -brand }.
lean-publish-not-in-build = Exportar y ver la vista previa no están en esta versión de { -brand }. tw convert sigue convirtiendo.

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
voice-list-failed = No se pudieron listar las voces: { $error } Elija otro motor en el menú Voz.
# $shown is voices-shown ("12 voices: English, all engines."). Enter,
# Space, Delete and Escape are the list's own keys.
voice-manager-intro = Gestor de voces. { $shown } Intro usa una voz y dice una muestra, o la descarga; { $preview } prueba una voz; Espacio marca una favorita; Suprimir elimina una voz descargada; Escape cierra.
voice-more-ready =
    { $n ->
        [one] Una voz más de otro motor está en la lista.
       *[other] { $n } voces más de otros motores están en la lista.
    }
voice-preview = Prueba: { $voice }.
voice-preview-sample = { $voice }. El veloz murciélago hindú comía feliz cardillo y kiwi.
voice-preview-starting = Prueba: { $voice }, iniciando { $engine }.
voice-preview-not-installed = { $voice } aún no está descargada. Intro la descarga, tras una pregunta.
voice-preview-unavailable = { $engine } no puede iniciarse aquí para una prueba. Intro cambia a ese motor.
voice-preview-engine-failed = No se pudo iniciar { $engine } para probar { $voice }.
voice-preview-failed = No se pudo probar { $voice }: { $error } Pruebe otra voz.
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
voice-remove-failed = No se pudo quitar { $voice }: { $error } Compruebe que se puede escribir en su carpeta.
voice-downloading-catalog = Descargando la lista de voces de Piper.
voice-downloading = Descargando { $voice }.
voice-downloading-percent = Descargando { $voice }, { $pct } por ciento.
voice-details-failed = No se pudieron leer los detalles de la voz: { $error } Compruebe la conexión y vuelva a intentarlo.
voice-download-stopped = La descarga se detuvo.
voice-catalog-fetched = La lista de voces de Piper tiene { $voices } voces en { $languages } idiomas. El comando Voces las lista.
voice-catalog-failed = No se pudo descargar la lista de voces: { $error } Compruebe la conexión y vuelva a intentarlo.
# $licence describes the voice's licence, in a sentence of its own.
voice-installed = { $voice } está instalada. { $licence } El comando Voces la lista.
voice-download-failed = No se pudo descargar { $voice }: { $error } Elija la voz de nuevo para reintentar.
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
voice-speed-preset = { $name }, velocidad { $wpm }.
voice-line-numbers-on = Números de línea activados.
voice-line-numbers-off = Números de línea desactivados.

## Export and preview from the reader. F5 is the browser's reload key,
## not textweaver's.

common-no-document = No hay ningún documento abierto.
# Said after "Could not export:", so it starts in lower case. $path is a
# folder or a file; $error the system's reason.
publish-cannot-write-to = no se puede escribir en { $path }: { $error } Compruebe que se puede escribir en la carpeta.
publish-cannot-write = no se puede escribir { $path }: { $error } Compruebe que se puede escribir en su carpeta.
publish-start-failed = No se pudo iniciar la exportación: { $error } Espere un momento y vuelva a intentarlo.
publish-export-error = No se pudo exportar: { $error } Corríjalo y vuelva a exportar.
publish-export-as-label = Exportar como, Intro para { $path }
publish-export-over-source = No exportado: es el propio documento. Elija otro nombre.
# $format is the format's name, such as PDF, HTML, or Word.
publish-exporting = Exportando a { $format }.
publish-theme-title = Tema para la página HTML
publish-theme-intro = ¿Tema para la página HTML? { $first } primero, { $n } opciones. Escape cancela.
publish-writing-preview = Escribiendo la vista previa.
publish-preview-error = No se pudo escribir la vista previa: { $error } Corríjalo y vuelva a abrir la vista previa.
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
publish-live-on-needs-reload = Vista previa en vivo activada. Funciona con la recarga automática, que está desactivada; actívela con Recargar la vista previa sola.
publish-live-off = Vista previa en vivo desactivada: la vista previa se recarga solo tras guardar.
# $error is the converter's reason.
publish-export-failed = Error al exportar a { $format }: { $error } Pruebe otro formato.
publish-preview-failed = Error en la vista previa: { $error } Guarde para volver a intentarlo.
# The converter's warnings: how many, and the first one.
publish-warnings =
    { $n ->
        [one] 1 advertencia: { $first }
       *[other] { $n } advertencias; la primera: { $first }
    }
# $file is the file's name, $folder its folder; $warned is empty or a
# space and publish-warnings.
publish-exported = Exportado a { $format }: { $file }. ¿Abrirlo? y o n. En { $folder }.{ $warned }
publish-report = Informe guardado como { $file }.
publish-report-issues =
    { $n ->
        [one] 1 elemento no accesible
       *[other] { $n } elementos no accesibles
    }; vea el informe.
publish-report-failed = No se pudo guardar el informe: { $error } Compruebe que se puede escribir en la carpeta de salida.
publish-preview-written-served = Vista previa escrita. Abriéndola en el navegador. Se recarga por sí sola después de cada guardado.{ $warned }
publish-preview-written = Vista previa escrita. Abriéndola en el navegador. Guardar la vuelve a escribir; luego pulse F5 en el navegador.{ $warned }
publish-preview-updated = Vista previa actualizada.
publish-preview-updated-press-f5 = Vista previa actualizada. Pulse F5 en el navegador.
publish-server-failed = No se pudo iniciar el servidor de recarga de la vista previa ({ $error }); se abre el archivo en su lugar.
publish-render-failed = No se pudo representar el texto: { $error } Salga del modo de edición para leer el texto.
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
notes-none = Sin notas. Para agregar una: { $key }.
notes-list-title = Notas
notes-list-intro =
    { $n ->
        [one] Notas, 1 elemento. Intro va a una nota, Suprimir la elimina, F2 la edita, Espacio abre sus enlaces.
       *[other] Notas, { $n } elementos. Intro va a una nota, Suprimir la elimina, F2 la edita, Espacio abre sus enlaces.
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
notes-no-highlights = Sin resaltados. Para crear uno: { $key }.
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

## Relations between notes (the knowledge graph as lists).

relations-type-conflicts-with = está en conflicto con
relations-type-supports = respalda
relations-type-is-example-of = es un ejemplo de
relations-type-cites = cita
relations-type-contradicts = contradice
relations-type-defines = define
relations-type-extends = amplía
relations-type-see-also = véase también
relations-type-precedes = precede a
relations-type-follows = sigue a
relations-count = Enlaces: { $out } salientes, { $in } entrantes.
relations-note-title = Enlaces de: { $note }
relations-title-filtered = { $title }, filtro: { $filter }
relations-note-intro = Enlaces de { $note }: { $out } salientes, { $in } entrantes. Intro sigue un enlace, F2 lo cambia, Suprimir lo quita. Escriba para filtrar por tipo.
relations-out-item = { $type }: { $target }
relations-target-in = { $note }, en { $doc }
relations-target-missing = una nota no encontrada
relations-empty-note = Nota vacía
relations-incoming-row =
    { $n ->
        [0] Qué enlaza aquí: nada aún
        [one] Qué enlaza aquí: 1 nota
       *[other] Qué enlaza aquí: { $n } notas
    }
relations-add-row = Añadir un enlace
relations-backlinks-title = Qué enlaza aquí: { $note }
relations-backlinks-intro =
    { $n ->
        [one] Qué enlaza a { $note }: 1 nota. Intro va a ella. Escriba para filtrar por tipo.
       *[other] Qué enlaza a { $note }: { $n } notas. Intro va a una. Escriba para filtrar por tipo.
    }
relations-backlink-item = { $type } esta, desde: { $note }
relations-none-in = Todavía nada enlaza a esta nota.
relations-types-title = Tipo de enlace para: { $note }
relations-types-intro = Elija el tipo de enlace, 10 tipos. Escriba para filtrar.
relations-targets-title = { $type }: ¿qué nota?
relations-targets-intro =
    { $n ->
        [0] No hay otra nota aquí. Elija una nota de otro documento.
        [one] Elija la nota de destino: 1 nota. Intro la enlaza.
       *[other] Elija la nota de destino: { $n } notas. Intro la enlaza.
    }
relations-other-document-row = Una nota de otro documento
relations-documents-title = Documentos con notas
relations-documents-intro = Documentos con notas: { $n }. Intro muestra las notas de un documento.
relations-document-item =
    { $n ->
        [one] { $title }, 1 nota
       *[other] { $title }, { $n } notas
    }
relations-no-other-documents = Ningún otro documento de la biblioteca tiene notas.
relations-linked = Enlazado: { $type } { $target }.
relations-changed = Enlace cambiado: { $type } { $target }.
relations-already = Ya enlazado: { $type } { $target }.
relations-removed = Enlace quitado: { $type } { $target }.
relations-remove-question = ¿Quitar este enlace? y o n
relations-nothing-to-remove = Aquí solo se puede quitar un enlace.
relations-note-gone = Nota no encontrada: se eliminó o su documento ya no está.
relations-document-missing = Documento no encontrado: { $file }.
relations-no-note-here = No hay ninguna nota aquí. Los enlaces pertenecen a notas; añada una: { $key }.
relations-filter-cleared = Filtro borrado, { $n } mostrados.
relations-filter-none = Nada coincide con { $filter }.
relations-filter-matched = Filtro { $filter }: { $n } mostrados.

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
    } guardada como { $file }. ¿Abrirla? y o n. En { $folder }.
notes-study-sheet-saved-highlights =
    Hoja de estudio con { $h ->
        [one] 1 resaltado
       *[other] { $h } resaltados
    } guardada como { $file }. ¿Abrirla? y o n. En { $folder }.
notes-study-sheet-saved-both =
    Hoja de estudio con { $n ->
        [one] 1 nota
       *[other] { $n } notas
    } y { $h ->
        [one] 1 resaltado
       *[other] { $h } resaltados
    } guardada como { $file }. ¿Abrirla? y o n. En { $folder }.
notes-study-sheet-failed = No se pudo escribir la hoja de estudio: { $error } Compruebe que se puede escribir en la carpeta.
# The study sheet file's own text (Markdown; the # marks stay in the code).
notes-sheet-title = Hoja de estudio: { $title }
notes-sheet-exported = Exportado desde { -brand } el { $date }.
notes-sheet-before-first-heading = Antes del primer encabezado
# After a note's text: its tags, joined with commas.
notes-sheet-tags = (etiquetas: { $tags })
# $color is the highlight's color name.
notes-sheet-highlighted = Resaltado, { $color }.

## La autoevaluación: preguntas con respuestas ocultas (crate::reveal).

reveal-self-test-title = Autoevaluación: { $title }
reveal-self-test-intro =
    { $n ->
        [one] Autoevaluación, 1 pregunta. Intro muestra la respuesta. Espacio para responder en voz alta.
       *[other] Autoevaluación, { $n } preguntas. Intro muestra cada respuesta. Espacio para responder en voz alta.
    }
reveal-nothing-to-test = No hay notas ni resaltados para evaluar. Añada antes una nota o un resaltado.
reveal-prompt-note = { $note } (en { $section })
reveal-prompt-highlight = ¿Qué resaltó en { $section }?
reveal-row-shown = { $prompt } Respuesta: { $answer }
reveal-answer = Respuesta: { $answer }
reveal-listening = Responda en voz alta ahora. Espacio para terminar.
reveal-you-said = Usted dijo: { $words }. Intro muestra la respuesta.
reveal-heard-nothing = No se oyó ninguna respuesta. Espacio para intentarlo de nuevo.
reveal-no-dictation = Responder en voz alta necesita el dictado, que no está en esta versión.

## Find, bookmarks, and selection.

marks-cannot-search = No se puede buscar: { $error } Compruebe el patrón entre las barras.
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
common-bookmark-set = Marcador { $name } puesto en el { $pct } por ciento.
marks-no-bookmarks = Sin marcadores. Para agregar uno: { $key }.
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
listmodel-item-position = { $k } de { $n }, { $item }
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
library-scan-failed = No se pudo explorar la biblioteca: { $error } Revise las carpetas de la biblioteca en la configuración.
library-scanning = Explorando la biblioteca.
library-scan-progress = Explorando la biblioteca: { $n } encontrados hasta ahora.
library-scan-stopped = La exploración de la biblioteca se detuvo inesperadamente. Abra la biblioteca de nuevo para reintentar.
# $command is the command line that adds a folder; $key names the Open command's key.
library-empty = La biblioteca está vacía. Agregue una carpeta en Configuración, en Carpetas de la biblioteca, o abra un archivo con { $key }.
library-add-folder-choose = Elija la carpeta que se añadirá a la biblioteca
library-folder-added = { $name } se añadió a la biblioteca. Abra la biblioteca para ver sus documentos.
library-folder-already = { $name } ya está en la biblioteca.
library-intro =
    { $n ->
        [one] Biblioteca, { $n } documento. Escriba para filtrar, Intro abre uno, F2 edita los detalles.
       *[other] Biblioteca, { $n } documentos. Escriba para filtrar, Intro abre uno, F2 edita los detalles.
    }
library-title = Biblioteca

## Following links and footnotes.

links-none-here = No hay ningún enlace o nota al pie en el cursor.
# $text is the link's text.
common-link-no-address = El enlace { $text } no tiene dirección.
# $kind is mail or web; $target is the link's address.
links-open-question =
    { $kind ->
        [mail] ¿Abrir el enlace de correo? y o n. { $target }
       *[web] ¿Abrir el enlace web? y o n. { $target }
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
citations-insert-failed = No se pudo insertar la cita: { $error } El texto no ha cambiado.
# $what is the identifier being looked up, as the citation library describes it.
citations-looking-up = Buscando { $what }.
citations-lookup-not-started = No se pudo iniciar la búsqueda: { $error } Espere un momento y vuelva a intentarlo.
# $input is the DOI or ISBN as typed.
citations-lookup-failed = No se pudo buscar { $input }: { $error } Compruebe el DOI o el ISBN y la conexión.
citations-no-library-to-add-to = No hay ninguna biblioteca a la que agregar: { -brand } no guarda archivos en esta sesión.
citations-library-save-failed = No se pudo guardar la biblioteca: { $error } Compruebe que se puede escribir en su carpeta.
# $n is how many citations the document has.
citations-found-no-library =
    { $n ->
        [one] Se encontró { $n } cita. { -brand } no guarda biblioteca en esta sesión.
       *[other] Se encontraron { $n } citas. { -brand } no guarda biblioteca en esta sesión.
    }
citations-check-failed = No se pudieron comprobar las citas: { $error } Compruebe el archivo de la biblioteca de referencias.
citations-no-library-to-import-into = No hay ninguna biblioteca a la que importar: { -brand } no guarda archivos en esta sesión.
# $file is the file's path.
citations-import-failed = No se pudo importar { $file }: { $error } Compruebe que sea un archivo .bib, .ris o .json.
# $style is the style's name from the front matter, such as apa.
citations-style-unusable = No se puede usar el estilo de cita { $style }: { $error } Compruebe el nombre del estilo al principio del documento.
citations-format-failed = No se pudieron formatear las citas: { $error } Compruebe las claves de cita y la biblioteca.
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
citations-bibliography-insert-failed = No se pudo insertar la bibliografía: { $error } El texto no ha cambiado.

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
open-refused-scheme = No abierto: enlace { $scheme } bloqueado.
open-refused-missing = No abierto: archivo no encontrado.
open-refused-invalid = No abierto: dirección no válida.
tasks-not-opened = No se abrió.
# Keep the letters y and n: they are the keys that answer.
tasks-open-it-question = ¿Abrirlo? y o n

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
# $speech is what was said for the step; $code is the math braille code's name (Nemeth or UEB); $braille is the part's braille in Unicode braille cells, for the Braille display.
mathx-step-braille = { $speech } { $code }: { $braille }

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
# On the first run with a screen reader and no mode chosen: hybrid mode is
# used, not asked. $reader is the screen reader found (NVDA, JAWS), or
# access-a-screen-reader; $key names the keys that change the mode.
access-hybrid-inferred = { $reader } está en marcha: textweaver lee los documentos en voz alta y deja los mensajes a su lector de pantalla. { $key } lo cambia.
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
setting-speech-voice-help = El id de la voz; sin definir elige una automáticamente. El comando Voces las lista.
setting-speech-prefer-voice = Voz preferida
setting-speech-prefer-voice-help = Cuando no hay voz definida, la primera voz cuyo nombre contenga esto, como eloquence.
setting-speech-favorite-voices = Voces favoritas
setting-speech-favorite-voices-help = Voces listadas primero por el comando Voces, por id, separadas por comas.
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
setting-speech-pause-heading-ms = Pausa tras encabezados
setting-speech-pause-heading-ms-help = Silencio tras un encabezado, más corto a mayor velocidad. 0 lo desactiva.
setting-speech-pause-paragraph-ms = Pausa tras párrafos
setting-speech-pause-paragraph-ms-help = Silencio tras un párrafo, más corto a mayor velocidad. 0 lo desactiva.
setting-speech-pause-list-item-ms = Pausa tras elementos de lista
setting-speech-pause-list-item-ms-help = Silencio tras un elemento de lista, más corto a mayor velocidad. 0 lo desactiva.
setting-speech-markup-pauses = Pausas escritas como marcado
setting-speech-markup-pauses-help = Lee el marcado de pausa de un documento, como <break time="1s"/>, como una pausa. Desactívalo en documentos que citan ese marcado.
setting-speech-output-device = Dispositivo de salida
setting-speech-output-device-help = El dispositivo de sonido en el que suena la voz, por su identificador. Sin definir usa el predeterminado del sistema, igual que un dispositivo que no está conectado.
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
setting-highlight-color-help = El color detrás de la palabra que se lee. Elija un nombre o escriba un código hexadecimal. Predeterminado: el color del tema.
setting-highlight-sentence-color = Color del resaltado de oración
setting-highlight-sentence-color-help = El color detrás de la oración que se lee. Elija un nombre o escriba un código hexadecimal. Predeterminado: el color del tema.
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
setting-normalization-medical-lexicon-enabled = Léxico médico
setting-normalization-medical-lexicon-enabled-help = Leer nombres de fármacos, términos clínicos y abreviaturas de dosis con una lista de pronunciación médica.
setting-normalization-medical-lexicon-overlay = Archivo del léxico médico
setting-normalization-medical-lexicon-overlay-help = Sus propias pronunciaciones médicas, que tienen prioridad sobre las integradas. Sin definir lee medical-lexicon.toml en la carpeta de configuración.
setting-reading-auto-resume = Reanudar donde lo dejó
setting-reading-auto-resume-help = Volver a la posición guardada al abrir un documento.
setting-reading-nav-history-size = Historial de Atrás
setting-reading-nav-history-size-help = Cuántos lugares recuerda Atrás.
setting-reading-wrap-navigation = Navegación cíclica
setting-reading-wrap-navigation-help = Moverse más allá del final del documento continúa desde el principio.
setting-reading-cursor-follows-speech = El cursor sigue a la voz
setting-reading-cursor-follows-speech-help = El cursor se mueve con la palabra que se está leyendo.
setting-reading-citations = Citas
setting-reading-citations-help = Citas en la lectura continua: omitidas, o dichas en palabras.
choice-reading-citations-off = omitidas
choice-reading-citations-words = en palabras
setting-reading-ocr = Reconocer páginas escaneadas
setting-reading-ocr-help = Leer el texto de PDF e imágenes escaneados reconociéndolo (OCR).
setting-reading-ocr-lang = Idioma del texto escaneado
setting-reading-ocr-lang-help = El idioma del texto escaneado, como códigos de Tesseract tales como fra o deu+eng. Vacío significa el idioma propio del documento, si no, inglés.
choice-reading-ocr-lang- = el del documento
choice-reading-ocr-lang-eng = inglés
choice-reading-ocr-lang-fra = francés
choice-reading-ocr-lang-deu = alemán
choice-reading-ocr-lang-spa = español
setting-reading-ocr-engine = Motor de OCR
setting-reading-ocr-engine-help = Qué motor reconoce las páginas escaneadas. ocrs para inglés y Tesseract para otros idiomas, o uno de ellos siempre.
choice-reading-ocr-engine-auto = automático
choice-reading-ocr-engine-ocrs = ocrs
choice-reading-ocr-engine-tesseract = Tesseract
choice-reading-ocr-engine-paddle = PaddleOCR (experimental)
setting-reading-math-engine = Voz de las matemáticas
setting-reading-math-engine-help = Qué motor lee las matemáticas en voz alta. El propio de textweaver, o MathCAT en ClearSpeak o SimpleSpeak, en el idioma del documento. MathCAT necesita una compilación que lo incluya; si no, se usa el propio de textweaver.
choice-reading-math-engine-builtin = textweaver
choice-reading-math-engine-mathcat = MathCAT ClearSpeak
choice-reading-math-engine-mathcat-simplespeak = MathCAT SimpleSpeak
setting-braille-math-code = Braille matemático
setting-braille-math-code-help = El código braille para las matemáticas en archivos BRF y al explorar una fórmula con MathCAT. Nemeth o las matemáticas de UEB. Necesita una compilación que incluya MathCAT; si no, las matemáticas se escriben con sus palabras habladas.
choice-braille-math-code-nemeth = Nemeth
choice-braille-math-code-ueb = UEB
setting-braille-table-format = Tablas en braille
setting-braille-table-format-help = Cómo disponen las tablas los archivos BRF. Lineal, una fila por línea con punto y coma entre las entradas; en lista, cada fila como un encabezado y cada entrada en su propia línea tras el encabezado de su columna; o escalonada, cada entrada dos celdas a la derecha de la anterior, para tablas de hasta cuatro columnas.
choice-braille-table-format-linear = lineal
choice-braille-table-format-listed = en lista
choice-braille-table-format-stairstep = escalonada
setting-reading-math-display = Matemáticas en pantalla
setting-reading-math-display-help = Cómo se ven las matemáticas en la vista de lectura. Como su fuente, tal como x^2, o como Unicode, tal como x con un 2 en superíndice. La voz y el modo de edición siempre usan la fuente.
choice-reading-math-display-source = fuente
choice-reading-math-display-unicode = Unicode
setting-reading-revisions = Control de cambios
setting-reading-revisions-help = Cómo se leen los cambios registrados en archivos de Word, OpenDocument y RTF. Se dicen en su lugar con verbosidad alta (automático), siempre, o nunca, leyendo el texto final. Se aplica al abrir un documento.
choice-reading-revisions-auto = automático
choice-reading-revisions-marked = decirlos siempre
choice-reading-revisions-final = solo el texto final
setting-reading-stop-at = Parar al final de la sección
setting-reading-stop-at-help = Dónde se detiene sola la lectura continua y dice Fin de la sección. Nunca, en el siguiente encabezado de cualquier nivel o en el siguiente capítulo: un salto de sección, o si no un encabezado de nivel 1. La lectura sigue desde el encabezado con la tecla de leer.
choice-reading-stop-at-off = nunca
choice-reading-stop-at-heading = siguiente encabezado
choice-reading-stop-at-chapter = siguiente capítulo
setting-reading-stop-after-minutes = Temporizador de lectura
setting-reading-stop-after-minutes-help = La lectura continua se detiene al final de la oración tras estos minutos de lectura, y lo dice. Pausar detiene el reloj; detener lo reinicia. 0 apaga el temporizador.
setting-reading-recall-prompts = Preguntas de recuerdo
setting-reading-recall-prompts-help = Al final de una sección, la lectura le pide que diga lo que recuerda. Si Detenerse al final de la sección es nunca, la lectura se detiene para ello en el siguiente encabezado. La lectura sigue con la tecla de leer.
setting-display-theme = Tema
setting-display-theme-help = El tema de color.
setting-display-follow-os-theme = Seguir el tema del sistema
setting-display-follow-os-theme-help = Al iniciar, usar un tema claro, oscuro o de alto contraste como el del sistema, a menos que haya elegido uno.
setting-display-wrap-width = Ancho de ajuste
setting-display-wrap-width-help = Ajustar las líneas a esta cantidad de columnas; 0 usa todo el ancho.
setting-display-measure = Longitud de línea
setting-display-measure-help = Cuántos caracteres caben en una línea de la ventana, de 25 a 90. 0 llena la ventana. La terminal usa el ancho de ajuste.
setting-display-tab-width = Ancho de tabulación
setting-display-tab-width-help = Columnas que ocupa un tabulador.
setting-display-show-line-numbers = Números de línea
setting-display-show-line-numbers-help = Mostrar los números de línea.
setting-display-scroll-margin = Margen de desplazamiento
setting-display-scroll-margin-help = Líneas que se mantienen visibles por encima y por debajo del cursor.
setting-display-hints = Línea de atajos
setting-display-hints-help = Si el lector del terminal muestra atajos de teclado en su última línea. Automático los muestra con voz propia y los oculta con un lector de pantalla. F1 y la lista de atajos de teclado siempre nombran las teclas.
choice-display-hints-auto = automático
choice-display-hints-on = activado
choice-display-hints-off = desactivado
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
setting-library-folders-help = Carpetas cuyos documentos lista la biblioteca, y cuyas posiciones se sincronizan entre computadoras. Separe las carpetas con punto y coma.
setting-keyboard-character-keys = Atajos de una sola tecla
setting-keyboard-character-keys-help = Teclas de exploración como h y punto. Desactivado, el dictado y la escritura nunca activan comandos.
setting-keyboard-preset = Teclas
setting-keyboard-preset-help = Las teclas predeterminadas: como el modo de exploración de NVDA y JAWS, o las teclas anteriores de textweaver. Se usa a partir del próximo inicio.
choice-keyboard-preset-default = estilo lector de pantalla
choice-keyboard-preset-classic = clásico
setting-keyboard-digit-row = Fila de dígitos
setting-keyboard-digit-row-help = Cómo reconoce el terminal las teclas de dígitos para los niveles de encabezado. Automático, o un teclado AZERTY francés.
choice-keyboard-digit-row-auto = automático
choice-keyboard-digit-row-azerty = AZERTY
setting-accessibility-mode = Modo de accesibilidad
setting-accessibility-mode-help = Voz propia lo dice todo; lector de pantalla deja la voz a su lector de pantalla; híbrido solo pone voz a la lectura.
choice-accessibility-mode-self-voicing = voz propia
choice-accessibility-mode-screen-reader = lector de pantalla
choice-accessibility-mode-hybrid = híbrido
setting-accessibility-say-all = Decir todo con un lector de pantalla
setting-accessibility-say-all-help = Lectura continua en modo lector de pantalla. Una oración a la vez en la línea de estado, o con la voz de textweaver.
choice-accessibility-say-all-screen = en la línea de estado
choice-accessibility-say-all-voice = con la voz de textweaver
setting-accessibility-quiet-screen = Pantalla quieta al leer
setting-accessibility-quiet-screen-help = Mantener la pantalla quieta mientras textweaver lee en voz alta. Activado de forma predeterminada en el modo híbrido.
choice-accessibility-quiet-screen-auto = automático
choice-accessibility-quiet-screen-true = activado
choice-accessibility-quiet-screen-false = desactivado
setting-accessibility-cursor = Cursor
setting-accessibility-cursor-help = Dónde espera el cursor del terminal. En lo que está trabajando, o en la línea de estado.
choice-accessibility-cursor-follow = sigue al foco
choice-accessibility-cursor-status = en la línea de estado
setting-export-audio-format = Formato de exportación de audio
setting-export-audio-format-help = El formato que Exportar audio ofrece primero. tw export-audio también lo usa para nombres de archivo sin extensión.
choice-export-audio-format-flac = FLAC
choice-export-audio-format-mp3 = MP3
choice-export-audio-format-opus = Opus
choice-export-audio-format-ogg = Ogg Vorbis
choice-export-audio-format-wav = WAV
choice-export-audio-format-m4b = Audiolibro M4B
choice-export-audio-format-mp4 = Vídeo MP4 con subtítulos
setting-export-subtitle-format = Formato de subtítulos
setting-export-subtitle-format-help = El formato de los subtítulos escritos sin nombre de archivo.
choice-export-subtitle-format-srt = SubRip
choice-export-subtitle-format-vtt = WebVTT
setting-export-subtitle-word-level = Subtítulos por palabra
setting-export-subtitle-word-level-help = Un subtítulo por palabra en vez de líneas de subtítulo.
setting-export-subtitles-with-audio = Subtítulos con audio
setting-export-subtitles-with-audio-help = Escribir siempre subtítulos junto al audio exportado.
choice-export-subtitle-format-ass = Karaoke ASS
setting-export-subtitle-karaoke = Karaoke de subtítulos
setting-export-subtitle-karaoke-help = Cómo muestran las líneas de subtítulos la palabra leída. Desactivado, subrayada al pronunciarse (etiquetas WebVTT) o un subtítulo por palabra en negrita y subrayado.
choice-export-subtitle-karaoke-off = Desactivado
choice-export-subtitle-karaoke-tags = Subrayar al pronunciar
choice-export-subtitle-karaoke-lines = Un subtítulo por palabra
setting-export-subtitle-chapters = Archivo de capítulos
setting-export-subtitle-chapters-help = Escribir también un archivo de capítulos WebVTT junto a los subtítulos o al audio.
# Names for chapters the document leaves untitled, in audio export.
export-chapter-untitled = Audiolibro
export-chapter-numbered = Capítulo { $number }
# The read-along page (tw export-audio essay.md --out essay.html).
readalong-skip = Saltar al texto
readalong-controls = Audio
readalong-play = Reproducir
readalong-pause = Pausa
readalong-back = Una frase atrás
readalong-forward = Una frase adelante
readalong-follow = Seguir la lectura
readalong-speed = Velocidad
readalong-contents = Contenido
readalong-play-section = Reproducir sección: { $title }
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
setting-lexicon-glossary-help = Su propio glosario, consultado antes que el diccionario: líneas término: definición, o el JSON de star. Sin definir usa glossary.txt en la carpeta de configuración.
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
setting-interface-rtl = Presentación de derecha a izquierda
setting-interface-rtl-help = Si el lector del terminal reordena el texto de derecha a izquierda para mostrarlo. Automático lo deja a los terminales que ya lo hacen por sí solos. La voz y el lector de pantalla siempre reciben el texto en el orden de lectura.
choice-interface-rtl-auto = automático
choice-interface-rtl-on = activado
choice-interface-rtl-off = desactivado
setting-gui-announce = Anuncios
setting-gui-announce-help = Cómo llegan los mensajes de la ventana al lector de pantalla, a partir del próximo inicio. Una región activa, o notificaciones de UI Automation (solo Windows).
choice-gui-announce-live = región activa
choice-gui-announce-uia = notificaciones de UI Automation
setting-gui-header = Mostrar la cabecera
setting-gui-header-help = Muestra la barra de comandos sobre el documento. Desactivada, los comandos conservan sus teclas y sus elementos de menú.
setting-gui-toolbar = Mostrar la barra de herramientas
setting-gui-toolbar-help = Muestra la barra de los botones de lectura. Desactivada, los comandos conservan sus teclas y sus elementos de menú.
setting-gui-auto-hide-menu = Ocultar la barra de menús
setting-gui-auto-hide-menu-help = Windows: oculta la barra de menús de la ventana hasta que Alt o F10 la muestra. Se oculta de nuevo al cerrar el menú. Sin efecto en Linux, cuyos menús son la lista de F10, ni en macOS.
setting-gui-speak-messages = Decir los mensajes de textweaver
setting-gui-speak-messages-help = Cuando textweaver lee en voz alta, decir también sus mensajes, lo que se escribe y los movimientos del cursor con su voz, para leer de oído sin lector de pantalla.
setting-gui-sidebar = Panel junto al documento
setting-gui-sidebar-help = El panel que la ventana muestra junto al documento. Ninguno, el Contenido (los encabezados) o las Notas. Las teclas de panel lo cambian, y la ventana recuerda el último.
choice-gui-sidebar-off = ninguno
choice-gui-sidebar-contents = Contenido
choice-gui-sidebar-notes = Notas

## Units, said after a number.

settings-unit-words-per-minute =
    { $n ->
        [one] palabra por minuto
       *[other] palabras por minuto
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
section-braille = Braille
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
edit-save-failed = No se pudo guardar: { $error } Aún editando. Pruebe Guardar como.
# The Save As prompt; $path is the suggested file.
edit-save-as-label = Guardar como, Intro para { $path }
# $name is a file name. Keep the letters y and n.
edit-file-exists-question = { $name } ya existe. ¿Reemplazarlo? y o n
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
edit-insert-failed = No se pudo insertar: { $error } El texto no ha cambiado.
# $start is the first words of the pasted text.
edit-pasted-lines =
    { $n ->
        [one] Se pegó 1 línea: { $start }
       *[other] Se pegaron { $n } líneas: { $start }
    }
# $start and $end are character positions, $len the text's length.
edit-range-out-of-text = No se pueden cambiar los caracteres { $start } a { $end }: el texto tiene { $len }.
edit-change-failed = No se pudo cambiar el texto: { $error } El texto no ha cambiado.
edit-delete-failed = No se pudo eliminar: { $error } El texto no ha cambiado.
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
        [horizontal-rule] Línea horizontal quitada.
        [table-row] Fila de tabla quitada.
        [heading-level] Encabezado de nivel { $level } quitado.
        [table] Tabla quitada.
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
        [horizontal-rule] Línea horizontal: sin cambios.
        [table-row] Fila de tabla: sin cambios.
        [heading-level] Encabezado de nivel { $level }: sin cambios.
        [table] Tabla: sin cambios.
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
edit-image-failed = No se pudo insertar la imagen: { $error } El texto no ha cambiado.
# $query is the text to find.
common-no-matches = Sin coincidencias para { $query }.
# $n matches of $query were found; the replacement is asked next.
edit-replace-with =
    { $n ->
        [one] 1 coincidencia de { $query }. ¿Reemplazar con?
       *[other] { $n } coincidencias de { $query }. ¿Reemplazar con?
    }

## Edit mode: autosave and recovering unsaved work.

common-recovery-write-failed = No se pudo escribir la copia de recuperación: { $error }. Guarde pronto; { -brand } lo seguirá intentando.
common-recovery-writing-again = La copia de recuperación se está escribiendo de nuevo.
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
replace-match-question = { $title }. La línea: { $context }. Pulse r para reemplazar, s para omitir, a para reemplazar todo, Escape para detener.
# The replace list's title when no match is being asked about.
replace-title = Reemplazar
# $n is this match's number, $total the number of matches, $line the line
# number, $found the matched text, and $result what it becomes (both
# shortened); the -removed form is for an empty replacement. $context in
# replace-match-question is the text of the match's line.
replace-match-title = Coincidencia { $n } de { $total }, línea { $line }: { $found } pasa a { $result }
replace-match-title-removed = Coincidencia { $n } de { $total }, línea { $line }: se elimina { $found }
replace-item-this = Reemplazar esta
replace-item-skip = Omitir esta
replace-item-rest = Reemplazar todas las demás
# $state is common-on or common-off.
replace-item-match-case = Distinguir mayúsculas: { $state }
replace-item-whole-words = Solo palabras completas: { $state }
replace-failed = No se pudo reemplazar: { $error } El texto no ha cambiado.
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
replace-replaced =
    { $n ->
        [one] Se reemplazó 1 coincidencia.
       *[other] Se reemplazaron { $n } coincidencias.
    }
replace-replaced-skipped = Se reemplazaron { $n }, se omitieron { $skipped }.
replace-stopped = Detenido. Se reemplazaron { $n }, se omitieron { $skipped }.
# Find and replace with regular expressions (B1-fr). $state is common-on
# or common-off; $n is the number of matches now.
replace-item-regex = Expresión regular: { $state }
replace-item-across-lines = A través de líneas: { $state }
replace-regex-now =
    { $n ->
        [one] Expresión regular { $state }. 1 coincidencia.
       *[other] Expresión regular { $state }. { $n } coincidencias.
    }
replace-across-lines-now =
    { $n ->
        [one] A través de líneas { $state }. 1 coincidencia.
       *[other] A través de líneas { $state }. { $n } coincidencias.
    }
# $problem is search-invalid-pattern: switching the option would make the
# pattern invalid.
replace-option-refused = { $problem } La opción no cambia.
# Asked once before replacing all the rest; $n is how many. Keep y and n.
replace-all-question =
    { $n ->
        [one] ¿Reemplazar la última coincidencia? y o n
       *[other] ¿Reemplazar las { $n } coincidencias restantes? y o n
    }
replace-all-declined = No se reemplazó nada.
# The search options list, and the options as named in search-options-on.
search-options-title = Opciones de búsqueda
search-option-match-case = coincidir mayúsculas
search-option-whole-words = palabras completas
search-option-regex = expresión regular
search-option-across-lines = a través de líneas
# Said as Find or Replace opens when an option is on; $list joins the
# options' names with commas.
search-options-on = Opciones activadas: { $list }.
# $at is the character where the pattern fails, counting from 1; $reason
# is the regular expression engine's own explanation (in English).
search-invalid-pattern = Patrón no válido en el carácter { $at }: { $reason }.
search-invalid-pattern-anywhere = Patrón no válido: { $reason }.

## Saving in the background.

writes-still-saving = Todavía guardando. Espere, por favor.
writes-not-written-in-time = Algunos cambios no se pudieron escribir a tiempo: el disco no responde.
# $error is the system's reason.
writes-save-failed = No se pudo guardar: { $error }. Aún editando.
# $name is the bookmark's name, $pct where it is.
writes-bookmark-not-saved = El marcador { $name } está puesto por ahora, pero no se pudo guardar: { $error } Compruebe que se puede escribir en la carpeta de datos.
# $name is the saved file's name.
writes-saved = Se guardó { $name }. Aún editando.

## Files changed on disk. $name is a file name. Keep the letters y and
## n: they are the keys that answer.

disk-replace-question = { $name } ya existe. ¿Reemplazarlo? y o n
# A prompt label, also said with a full stop after it.
disk-not-replaced = No se reemplazó. Escriba otro nombre
# $key is the key for Save As.
disk-not-saved = No se guardó. Aún editando. Guardar Como, { $key }, conserva ambas versiones.
disk-kept-open-version = Se conservó la versión abierta.
disk-overwrite-question = { $name } cambió en el disco desde que lo abrió. ¿Guardar sobre esos cambios? y o n
disk-reload-question = { $name } cambió en el disco. ¿Volver a cargarlo? y o n

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

common-not-in-table = No está en una tabla.
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
# $parts lists what came in, from the paste-part messages.
paste-markdown = Pegado como Markdown: { $parts }
paste-part-heading =
    { $n ->
        [one] 1 encabezado
       *[other] { $n } encabezados
    }
paste-part-paragraph =
    { $n ->
        [one] 1 párrafo
       *[other] { $n } párrafos
    }
paste-part-list =
    { $n ->
        [one] 1 lista
       *[other] { $n } listas
    }
paste-part-table =
    { $n ->
        [one] 1 tabla
       *[other] { $n } tablas
    }
paste-part-code =
    { $n ->
        [one] 1 bloque de código
       *[other] { $n } bloques de código
    }
paste-part-quote =
    { $n ->
        [one] 1 cita
       *[other] { $n } citas
    }
paste-part-link =
    { $n ->
        [one] 1 enlace
       *[other] { $n } enlaces
    }
paste-empty = Nada que pegar: el portapapeles está vacío.
paste-converting = Convirtiendo el texto con formato para pegarlo.
paste-failed = No se pudo pegar: { $error } Pruebe Pegar como texto sin formato.
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
common-line = Línea { $line }.

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
common-left-as-is = Se dejó tal cual.
# $fix is the fix chosen; $key turns on edit mode.
grammar-fix-not-editing = { $fix }. Active el modo de edición con { $key } para cambiar el texto.
grammar-removed = Quitado.
grammar-changed = Cambiado a { $fix }.
grammar-change-failed = No se pudo cambiar el texto: { $error } No se cambió nada.

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
spell-replace-failed = No se pudo reemplazar: { $error } La palabra no ha cambiado.
spell-added-for-session = Se agregó { $word } a su lista de palabras para esta sesión.
spell-added = Se agregó { $word } a su lista de palabras.
spell-save-failed = No se pudo guardar su lista de palabras: { $error } La palabra se reconoce hasta que salga.
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
tui-setup-speech-failed = No se pudo iniciar la voz ({ $error }); ejecutando en silencio.
speech-engine-fallback = { $failed } no pudo iniciarse; ahora habla { $engine }.
speech-engine-fallback-silent = { $failed } no pudo iniciarse y no hay otro motor de voz disponible; { -brand } queda en silencio.
tui-setup-cannot-save = No se puede guardar la configuración ni las posiciones: { $error } La lectura funciona. Los cambios se pierden al salir.
tui-setup-keymap-ignored = Se ignoró el archivo de teclas: { $error } Se usan las teclas predeterminadas. Corrija el archivo y reinicie.
# The first-run welcome. Each value names the key for an action: $play
# reads and pauses, $stop stops, $heading moves to the next heading,
# $help opens the help, $quit quits.
tui-setup-welcome = Bienvenido a { -brand }. { $open } abre un documento. { $play } inicia y pausa la lectura, y { $stop } la detiene. { $palette } lista todos los comandos. { $help } abre la ayuda.
gui-setup-welcome-menus = Bienvenido a { -brand }. { $open } abre un documento. { $play } lee y pausa, { $stop } detiene. { $palette } lista todos los comandos, Alt o { $menu } los menús. { $help } abre la ayuda.
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
tui-hint-choose = elegir
tui-hint-close = cerrar
tui-hint-back = atrás
# The list overlay's border: $n is the focused item's number, $count
# the number of items.
tui-list-title = { $n } de { $count }, { $title }

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
gui-text-size = Tamaño del texto { $size } puntos.
gui-text-size-largest = Tamaño del texto { $size } puntos, el mayor.
gui-text-size-smallest = Tamaño del texto { $size } puntos, el menor.
gui-font = Fuente: { $family }.
gui-font-list = Fuente

## The Braille pass: pages in paged documents such as a PDF.
## $page and $n are page numbers, $label a printed page label such as iv,
## $pages the number of pages. Keep the page first: a 40-cell Braille
## display shows the start of the line.

status-page = página { $page } de { $pages }
status-page-labelled = página { $label }, { $n } de { $pages }
status-percent = { $pct }%
pages-position = Página { $page } de { $pages }.
pages-position-labelled = Página { $label }, { $n } de { $pages }.
pages-none = Este documento no tiene páginas.
pages-no-such-page = No hay página { $page }. Las páginas van de 1 a { $pages }.
pages-label = Página { $label }
pages-outline-item = Página { $label }: { $text }
lists-pages-title =
    { $n ->
        [one] Páginas, { $n } página
       *[other] Páginas, { $n } páginas
    }
lists-pages-title-filtered = Páginas, { $shown } de { $n } coinciden con { $filter }
lists-pages-intro =
    { $n ->
        [one] Páginas, { $n } página. Escriba para filtrar, Intro va a una página, Escape cierra.
       *[other] Páginas, { $n } páginas. Escriba para filtrar, Intro va a una página, Escape cierra.
    }
lists-pages-here = Está en { $heading }.
lists-filter-cleared-pages =
    { $n ->
        [one] Filtro borrado, { $n } página.
       *[other] Filtro borrado, { $n } páginas.
    }
lists-filter-none-pages = Ninguna página coincide con { $query }. Retroceso quita letras.
lists-filter-matched-pages =
    { $n ->
        [one] { $n } página coincide.
       *[other] { $n } páginas coinciden.
    }
prompt-go-to-pages = Ir a página, o línea 12, porcentaje, inicio o fin
goto-not-a-target-pages = No es un destino válido: { $text }. Escriba un número de página, línea y un número, un porcentaje como 50%, inicio o fin.
goto-word-page = página

## El filtro de la biblioteca, el diccionario y las velocidades.

# The library list filtered: $shown of $n documents match $filter.
library-title-filtered = Biblioteca, { $shown } de { $n } coinciden con { $filter }
# The filter was emptied: $n documents are shown.
library-filter-cleared =
    { $n ->
        [one] Filtro borrado, { $n } documento.
       *[other] Filtro borrado, { $n } documentos.
    }
# No document matches the filter $query.
library-filter-none = Ningún documento coincide con { $query }. Retroceso quita letras.
# $n documents match the filter.
library-filter-matched =
    { $n ->
        [one] { $n } documento coincide.
       *[other] { $n } documentos coinciden.
    }
# Said once when define word is used while the dictionary file is still opening.
define-still-loading = El diccionario aún se está cargando.
# Ajustes.
setting-speech-dectalk-library = Biblioteca de DECtalk
setting-speech-dectalk-library-help = La biblioteca de DECtalk que se cargará; sin definir busca en los lugares habituales.
setting-speech-espeak-helper = Programa auxiliar de eSpeak NG
setting-speech-espeak-helper-help = Ejecutar eSpeak NG en su propio programa auxiliar, para que un fallo del motor no cierre textweaver. Automático usa el programa auxiliar en Windows cuando está instalado y, en los demás sistemas, ejecuta eSpeak NG dentro de textweaver.
choice-speech-espeak-helper-auto = automático
choice-speech-espeak-helper-always = siempre el programa auxiliar
choice-speech-espeak-helper-never = dentro de textweaver
setting-speech-piper-voices = Carpeta de voces de Piper
setting-speech-piper-voices-help = La carpeta de voces de Piper; sin definir usa la carpeta piper de la carpeta de datos de textweaver.
setting-speech-piper-voice = Voz de Piper
setting-speech-piper-voice-help = La voz de Piper con la que empezar, por su id; sin definir toma la primera instalada.
setting-speech-piper-phonemizer = Fonetizador de Piper
setting-speech-piper-phonemizer-help = Cómo convierte Piper el texto en sonidos. La biblioteca espeak-ng si está instalada, esa biblioteca o el de textweaver.
choice-speech-piper-phonemizer-auto = automático
choice-speech-piper-phonemizer-library = biblioteca espeak-ng
choice-speech-piper-phonemizer-rust = el de textweaver
setting-speech-voice-params = Velocidad y tono por voz
setting-speech-voice-params-help = La velocidad y el tono con que se usó cada voz por última vez. Al elegir de nuevo una voz, vuelven.
setting-editing-author = Autor
setting-editing-author-help = El autor que se escribe en los documentos nuevos hechos con una plantilla; vacío lo deja en blanco.

## The window (GUI): drawn labels, hints, and questions.
## Keep the letters Y and N: they are the keys that answer.

gui-yes = Sí
gui-no = No
gui-answer-delete = Eliminar
gui-answer-remove = Quitar
gui-answer-replace = Reemplazar
gui-question-hint = Y responde sí, N responde no, Escape responde no.
gui-find-title = Buscar y reemplazar
gui-find-what = Buscar
gui-find-with = Reemplazar con
gui-find-match-case = Coincidir mayúsculas
gui-find-whole-words = Palabras completas
gui-find-regex = Expresión regular
gui-find-across-lines = Entre líneas
gui-find-next = Buscar siguiente
gui-find-next-help = Selecciona la siguiente coincidencia.
gui-find-replace = Reemplazar
gui-find-replace-help = Reemplaza la coincidencia mostrada y pasa a la siguiente. La primera pulsación busca una coincidencia.
gui-find-replace-all = Reemplazar todo
gui-find-replace-all-help = Dice cuántas coincidencias hay y pregunta una vez. Un solo deshacer las revierte todas.
gui-find-hint = Intro en Buscar busca la siguiente coincidencia; Intro en Reemplazar con la reemplaza. Flecha arriba recupera textos anteriores. Escape cierra.
gui-find-empty = Nada que buscar: escriba el texto en Buscar.
gui-button-open = Abrir…
gui-button-font = Fuente…
gui-button-edit = Empezar a editar
gui-button-finish-editing = Terminar de editar
gui-button-settings = Configuración…
gui-button-commands = Comandos…
gui-button-play = Reproducir
gui-button-pause = Pausa
gui-button-stop = Detener
gui-button-previous-sentence = Frase anterior
gui-button-next-sentence = Frase siguiente
gui-button-slower = Más lento
gui-button-faster = Más rápido
gui-hint-open = Elegir un documento para leer.
gui-hint-font = Elegir la fuente del texto.
gui-hint-edit = Cambiar entre leer y editar.
gui-hint-settings = Cada opción, con su ayuda.
gui-hint-commands = Ejecutar un comando por su nombre.
gui-hint-play = Leer desde la palabra actual, o pausar.
gui-button-close = Cerrar
gui-toolbar-reading = Lectura
gui-document = Documento
gui-document-titled = { $title }, documento
gui-list-hint = Intro elige, Escape cierra.
gui-sidebar-contents = Contenido
gui-sidebar-notes = Notas
gui-sidebar-open =
    { $n ->
        [one] { $panel } abierto, 1 elemento.
       *[other] { $panel } abierto, { $n } elementos.
    }
gui-sidebar-closed = { $panel } cerrado.
gui-header-shown = Cabecera mostrada.
gui-header-hidden = Cabecera oculta. Sus comandos conservan sus teclas.
gui-toolbar-shown = Barra de herramientas mostrada.
gui-toolbar-hidden = Barra de herramientas oculta. Sus comandos conservan sus teclas.
gui-sidebar-no-headings = No hay encabezados.
gui-sidebar-no-notes = No hay notas.
gui-sidebar-current = { $item }, actual
gui-sidebar-hint = Intro va allí. { $leave } va y vuelve. Escape vuelve.
gui-settings-sections = Secciones
gui-settings-form = Configuración: { $section }
gui-settings-saved-hint = Los cambios se aplican y se guardan al momento.
gui-settings-close-help = Cerrar la configuración. Cada cambio ya está guardado.
gui-settings-recent = Cambiados hace poco
gui-settings-matching = Coinciden con { $filter }
gui-settings-table = { $label } es una tabla. Edítela en settings.toml.
gui-setting-new-value = Nuevo valor para { $label }
gui-setting-value-hint = Pulse Intro para aceptar, o Escape para volver.
gui-prompt-path-hint = Escriba la ruta de un documento y pulse Intro. Tab la completa; Arriba y Abajo recuperan las anteriores.
gui-prompt-hint = Pulse Intro para aceptar, o Escape para cancelar. Arriba y Abajo recuperan respuestas anteriores.
gui-palette-filter = Escriba para filtrar los comandos
gui-palette-list = Comandos
gui-palette-hint = Intro ejecuta la primera coincidencia; Tab pasa a la lista; F1 explica un comando.
gui-open-failed = No se pudo abrir { $name }: { $error }
gui-uia-unavailable = Las notificaciones de UI Automation solo existen en Windows; se usa la región activa.
gui-graphics-failed = La ventana no pudo iniciar sus gráficos. El lector de terminal, textweaver, no los necesita.
gui-crashed = textweaver se detuvo tras un error interno.
gui-crashed-saved = Se guardó su posición, y las ediciones sin guardar se ofrecerán para recuperar la próxima vez.
gui-rsvp = RSVP
gui-rsvp-playing = RSVP en marcha, palabra { $n } de { $total }
gui-rsvp-paused = RSVP en pausa, palabra { $n } de { $total }
gui-rsvp-finished = RSVP terminado, palabra { $n } de { $total }
gui-settings-section-item =
    { $section }, { $n ->
        [one] 1 ajuste
       *[other] { $n } ajustes
    }
gui-palette-count =
    { $n ->
        [0] Ningún comando coincide.
        [one] 1 comando.
       *[other] { $n } comandos.
    }
gui-settings-form-help = Arriba y Abajo pasan de un ajuste a otro. Izquierda y Derecha cambian uno. Intro escribe un valor nuevo. Suprimir restablece el valor predeterminado. { $next } y { $previous } cambian de sección. Escribe para filtrar. F1 dice la ayuda.
gui-settings-press-enter = Pulse Intro para escribir un valor nuevo para { $label }.
gui-font-built-in = { $family } (incluida)
## Summaries and difficult-word definitions.

action-summarize = Resumir la selección, el capítulo o el documento: sus oraciones más centrales en una lista; Intro va a una
# The summary list's title: $n sentences of the whole document.
summary-title =
    { $n ->
        [one] Resumen, { $n } oración
       *[other] Resumen, { $n } oraciones
    }
# The summary of the chapter at the cursor.
summary-title-chapter =
    { $n ->
        [one] Resumen del capítulo, { $n } oración
       *[other] Resumen del capítulo, { $n } oraciones
    }
# The summary of the selection.
summary-title-selection =
    { $n ->
        [one] Resumen de la selección, { $n } oración
       *[other] Resumen de la selección, { $n } oraciones
    }
# Said when the summary list opens; $title is one of the titles above.
summary-intro = { $title }. Intro va a la oración y la dice.
# The same, when a long text was read in samples.
summary-intro-sampled = { $title }, de muestras de este texto largo. Intro va a la oración y la dice.
summary-none = Nada que resumir: ninguna oración de cuatro palabras o más.
# tw summarize, on standard error, when a long text was read in samples: $read of $total characters.
summary-sampled-cli = Un texto largo: el resumen sale de { $read } de sus { $total } caracteres, leídos en muestras.
# After a difficult word at high verbosity, with definitions on: its first definition.
aids-difficult-word-defined = palabra difícil: { $definition }
setting-summary-sentences = Oraciones del resumen
setting-summary-sentences-help = Cuántas oraciones dan Resumir y tw summarize, de 1 a 50.
setting-reading-aids-difficult-definitions = Definiciones de palabras difíciles
setting-reading-aids-difficult-definitions-help = Con las palabras difíciles marcadas, con verbosidad alta decir también la primera definición del diccionario de una palabra difícil.
section-summary = Resúmenes
settings-unit-sentences =
    { $n ->
        [one] oración
       *[other] oraciones
    }

# The GUI. Said in textweaver's own voice when the window takes the
# focus; $title is the document's title.
gui-window-focused = { $title }, { -brand }.

## Opening the new formats. Said after "Could not open NAME:", so
## each starts in lower case.
opening-damaged-json = no es un archivo JSON legible; puede ser demasiado grande.
opening-damaged-notebook = no es un cuaderno de Jupyter legible; puede estar dañado o ser demasiado grande.
opening-damaged-svg = no es un dibujo SVG legible; puede estar dañado o ser demasiado grande.
opening-damaged-mathml = no es una fórmula MathML legible; puede estar dañada o ser demasiado grande.

## Menus, the command palette, interface announcements, colors, and settings

## Menu titles; the top menus mark their access key with &.

menu-file = &Archivo
menu-edit = &Edición
menu-view = &Ver
menu-reading = &Lectura
menu-speech = Vo&z
menu-tools = &Herramientas
menu-help = A&yuda
menu-recent = Documentos recientes
menu-export-as = E&xportar como
menu-preview = Vista previa
menu-settings = Configuración
menu-find = Buscar
menu-format = Formato
menu-insert = Insertar
menu-proofing = Revisión
menu-citations = Citas
menu-text-size = Tamaño del texto
menu-reading-aids = Ayudas de lectura
menu-rsvp = RSVP
menu-say = Decir
menu-move-by = Moverse por
menu-headings = Títulos
menu-go-to = Ir a
menu-cursor = Cursor y selección
menu-bookmarks = Marcadores y notas
menu-tables = Tablas
menu-speech-cursor = Cursor de voz

## Command names, in the menus and the command palette.

name-play-pause = Reproducir o pausar
name-stop = Detener
name-read-from-cursor = Leer desde el cursor
name-read-document = Leer todo el documento
name-read-current-character = Decir carácter
name-read-current-word = Decir palabra
name-read-current-sentence = Decir oración
name-read-current-line = Decir línea
name-read-paragraph = Decir párrafo
name-read-selection = Leer selección
name-say-position = Decir posición
name-say-status = Decir estado
name-repeat-message = Repetir último mensaje
name-word-count = Número de palabras
name-link-address = Dirección del enlace
name-replay-sentence = Repetir oración
name-replay-paragraph = Repetir párrafo
name-repeat-sentence-slower = Repetir más despacio
name-rsvp-toggle = RSVP
name-rsvp-play-pause = Iniciar o pausar RSVP
name-rsvp-faster = RSVP más rápido
name-rsvp-slower = RSVP más lento
name-rsvp-position-next = Mover la palabra RSVP
name-reading-level = Nivel de lectura
name-document-overview = Resumen del documento
name-reading-pass = Pasada de lectura
name-define-word = Definir palabra
name-summarize = Resumir
name-toggle-citations = Leer citas
name-explore-math = Explorar matemáticas
name-listen-rendered = Escuchar como se verá
name-next-sentence = Oración siguiente
name-previous-sentence = Oración anterior
name-next-paragraph = Párrafo siguiente
name-previous-paragraph = Párrafo anterior
name-next-heading = Leer desde el título siguiente
name-previous-heading = Leer desde el título anterior
name-skip-next-heading = Título siguiente
name-skip-previous-heading = Título anterior
name-outline = Esquema
name-next-heading-level-1 = Título siguiente, nivel 1
name-next-heading-level-2 = Título siguiente, nivel 2
name-next-heading-level-3 = Título siguiente, nivel 3
name-next-heading-level-4 = Título siguiente, nivel 4
name-next-heading-level-5 = Título siguiente, nivel 5
name-next-heading-level-6 = Título siguiente, nivel 6
name-previous-heading-level-1 = Título anterior, nivel 1
name-previous-heading-level-2 = Título anterior, nivel 2
name-previous-heading-level-3 = Título anterior, nivel 3
name-previous-heading-level-4 = Título anterior, nivel 4
name-previous-heading-level-5 = Título anterior, nivel 5
name-previous-heading-level-6 = Título anterior, nivel 6
name-next-table = Tabla siguiente
name-previous-table = Tabla anterior
name-next-list = Lista siguiente
name-previous-list = Lista anterior
name-next-list-item = Elemento de lista siguiente
name-previous-list-item = Elemento de lista anterior
name-next-link = Enlace siguiente
name-previous-link = Enlace anterior
name-next-block-quote = Cita siguiente
name-previous-block-quote = Cita anterior
name-next-separator = Separador siguiente
name-previous-separator = Separador anterior
name-next-graphic = Gráfico siguiente
name-previous-graphic = Gráfico anterior
name-follow-link = Seguir enlace
name-table-next-row = Fila siguiente de la tabla
name-table-previous-row = Fila anterior de la tabla
name-table-next-column = Columna siguiente de la tabla
name-table-previous-column = Columna anterior de la tabla
name-next-chapter = Capítulo siguiente
name-previous-chapter = Capítulo anterior
name-history-back = Atrás
name-history-forward = Adelante
name-go-to = Ir a
name-document-start = Inicio del documento
name-document-end = Final del documento
name-caret-next-word = Palabra siguiente
name-caret-previous-word = Palabra anterior
name-caret-next-line = Línea siguiente
name-caret-previous-line = Línea anterior
name-select-next-word = Seleccionar palabra siguiente
name-select-previous-word = Seleccionar palabra anterior
name-select-next-line = Seleccionar línea siguiente
name-select-previous-line = Seleccionar línea anterior
name-page-down = Página abajo
name-page-up = Página arriba
name-scroll-down = Desplazar abajo
name-scroll-up = Desplazar arriba
name-speech-cursor-toggle = Cursor de voz
name-speech-cursor-next-line = Cursor de voz, línea siguiente
name-speech-cursor-previous-line = Cursor de voz, línea anterior
name-speech-cursor-reread-line = Cursor de voz, releer línea
name-speech-cursor-exit-and-read = Cursor de voz, seguir leyendo
name-rate-up = Más rápido
name-rate-down = Más lento
name-pitch-up = Tono más agudo
name-pitch-down = Tono más grave
name-volume-up = Más fuerte
name-volume-down = Más bajo
name-cycle-speed-preset = Velocidad predefinida
name-choose-voice = Voces
name-restart-speech = Reiniciar la voz
name-cycle-verbosity = Detalle
name-cycle-punctuation = Puntuación
name-find = Buscar
name-find-next = Buscar siguiente
name-find-previous = Buscar anterior
name-search-options = Opciones de búsqueda
name-next-misspelling = Error ortográfico siguiente
name-previous-misspelling = Error ortográfico anterior
name-spelling-suggestions = Sugerencias ortográficas
name-next-grammar-problem = Problema gramatical siguiente
name-previous-grammar-problem = Problema gramatical anterior
name-next-lint-problem = Problema de formato siguiente
name-previous-lint-problem = Problema de formato anterior
name-add-bookmark = Añadir marcador
name-list-bookmarks = Marcadores
name-next-bookmark = Marcador siguiente
name-previous-bookmark = Marcador anterior
name-add-note = Añadir nota
name-list-notes = Notas
name-next-note = Nota siguiente
name-previous-note = Nota anterior
name-delete-note = Borrar nota o resaltado
name-highlight-selection = Resaltar
name-export-study-sheet = Exportar hoja de estudio
name-self-test = Autoevaluación
name-open = Abrir
name-open-path = Abrir por ruta
name-open-library = Biblioteca
name-new-document = Documento nuevo
name-save = Guardar
name-save-as = Guardar como
name-export-settings = Exportar configuración
name-import-settings = Importar configuración
name-reading-statistics = Estadísticas de lectura
name-new-from-template = Nuevo desde plantilla
name-export-html = Exportar HTML
name-export-pdf = Exportar PDF
name-export-docx = Exportar Word
name-export-epub = Exportar EPUB
name-export-brf = Exportar braille
name-preview-in-browser = Vista previa en el navegador
name-toggle-preview-auto-reload = Recargar la vista previa sola
name-toggle-preview-live = Vista previa en vivo
name-browse-files = Explorar archivos
name-batch-convert = Convertir por lotes
name-export-audio = Exportar audio
name-quit = Salir
name-toggle-edit-mode = Modo de edición
name-undo = Deshacer
name-redo = Rehacer
name-bold = Negrita
name-italic = Cursiva
name-underline = Subrayado
name-strikethrough = Tachado
name-inline-code = Código en línea
name-code-block = Bloque de código
name-insert-link = Insertar enlace
name-heading = Título
name-bullet-list = Lista con viñetas
name-numbered-list = Lista numerada
name-block-quote = Cita en bloque
name-horizontal-rule = Línea horizontal
name-insert-table = Insertar tabla
name-add-table-row = Añadir fila a la tabla
name-insert-image = Insertar imagen
name-replace = Reemplazar
name-copy = Copiar
name-cut = Cortar
name-next-table-cell = Celda siguiente
name-previous-table-cell = Celda anterior
name-cycle-typing-echo = Eco del teclado
name-select-all = Seleccionar todo
name-delete-word-before = Borrar palabra anterior
name-delete-word-after = Borrar palabra siguiente
name-paste = Pegar
name-paste-plain-text = Pegar como texto sin formato
name-context-menu = Menú contextual
name-insert-citation = Insertar cita
name-add-reference = Añadir referencia
name-insert-bibliography = Insertar bibliografía
name-check-citations = Revisar citas
name-import-references = Importar referencias
name-dictate = Dictar
name-next-theme = Tema siguiente
name-toggle-line-numbers = Números de línea
name-toggle-character-keys = Atajos de una tecla
name-cycle-access-mode = Modo de accesibilidad
name-settings-profiles = Perfiles
name-bionic-toggle = Lectura biónica
name-ruler-cycle = Regla de lectura
name-syllables-toggle = Sílabas
name-difficult-words-toggle = Palabras difíciles
name-text-larger = Texto más grande
name-text-smaller = Texto más pequeño
name-text-size-reset = Tamaño de texto normal
name-choose-font = Fuente
name-contents-panel = Panel Contenido
name-notes-panel = Panel Notas
name-toggle-header = Cabecera
name-toggle-toolbar = Barra de herramientas
name-next-region = Siguiente región
name-previous-region = Región anterior
name-color-settings = Colores
name-cycle-interface-announcements = Avisos de la interfaz
name-menu = Menús
name-command-palette = Paleta de comandos
name-settings = Configuración
name-keyboard-help = Atajos de teclado
name-what-does-this-key-do = Qué hace esta tecla
name-about = Acerca de textweaver
name-help = Ayuda

## Menus, the palette, and interface announcements.

menu-bar = Menús
menu-title = Menú { $name }
menu-submenu = { $name }, submenú
menu-checked = { $name }, activado
menu-not-checked = { $name }, desactivado
menu-value = { $name }: { $value }
menu-with-keys = { $item }, { $keys }
menu-recent-document = { $name }, { $pct } por ciento
menu-recent-none = No hay documentos recientes
menu-not-available = { $name } no está disponible en esta versión.
menu-no-access-key = Ningún elemento con la tecla { $letter }.
menu-closed = Menús cerrados.
menu-context = Menú contextual
menu-context-closed = Menú contextual cerrado.
menu-press-a-key = Pulse una tecla para oír lo que hace.
menu-key-described = { $name }: { $help }. Teclas: { $keys }. En los menús: { $path }.
menu-key-described-no-menu = { $name }: { $help }. Teclas: { $keys }.
menu-keys = { $item }. Intro o Derecha abre un menú o ejecuta un comando, una letra va a su elemento, Izquierda o Retroceso vuelve, Escape cierra.
edit-line-continues = la línea sigue
announce-level-changed = Avisos de la interfaz: { $level }.
announce-level-off = desactivados
announce-level-minimal = mínimos
announce-level-normal = normales
announce-level-full = completos
setting-accessibility-interface-announcements = Avisos de la interfaz
setting-accessibility-interface-announcements-help = Cuánto dice textweaver de sí mismo: diálogos, progreso, pistas y confirmaciones de rutina. Los errores y las respuestas a lo que pidió se dicen siempre. Automático es mínimo con un lector de pantalla y normal si textweaver habla solo.
choice-accessibility-interface-announcements-auto = automático
choice-accessibility-interface-announcements-off = desactivados
choice-accessibility-interface-announcements-minimal = mínimos
choice-accessibility-interface-announcements-normal = normales
choice-accessibility-interface-announcements-full = completos
palette-item = { $name }, { $keys }
palette-item-no-keys = { $name }
palette-item-recent = { $name }, { $keys }, reciente
palette-item-recent-no-keys = { $name }, reciente
palette-list-title = Comandos que coinciden con { $query }
palette-list-title-all = Comandos
palette-list-intro =
    { $title }, { $n ->
        [one] 1 comando
       *[other] { $n } comandos
    }. Intro ejecuta uno, F1 lo explica.
action-browse-files = Explorar archivos y archivos comprimidos: Intro abre una carpeta, un archivo comprimido o un documento; Retroceso sube un nivel
action-batch-convert = Convertir una carpeta de documentos a otro formato, en segundo plano
action-export-audio = Exportar el documento como audio hablado: MP3, FLAC, Opus, WAV o un audiolibro M4B
action-dictate = Iniciar o detener el dictado: las palabras habladas se escriben en el cursor en modo de edición
action-color-settings = Abrir las opciones de color: el resaltado de lectura, la regla, las marcas y cada parte de la pantalla, con su contraste
action-cycle-interface-announcements = Cambiar cuánto anuncia textweaver de sí mismo: desactivados, mínimos, normales o completos; los errores y las respuestas se dicen siempre
action-menu = Abrir los menús: Archivo, Edición, Ver, Lectura, Voz, Herramientas y Ayuda
action-what-does-this-key-do = Pulsar una tecla para oír lo que hace y dónde está en los menús, sin ejecutarla
action-about = Listar los datos que necesita un informe de problemas: versión, compilación, componentes, motores de voz y carpetas
setting-colors-ruler = Color de la regla de lectura
setting-colors-ruler-help = La banda de la regla de lectura y la línea actual marcada. La regla conserva su subrayado o su negrita. Elija un nombre o escriba un código hexadecimal. Predeterminado: el color del tema.
setting-colors-difficult-words = Color de las palabras difíciles
setting-colors-difficult-words-help = El subrayado de las palabras difíciles; siguen subrayadas y se nombran con detalle alto. Elija un nombre o escriba un código hexadecimal. Predeterminado: el color del tema.
setting-colors-syllables = Color de las marcas de sílaba
setting-colors-syllables-help = Los puntos medios entre sílabas. Elija un nombre o escriba un código hexadecimal. Predeterminado: el color del tema.
setting-colors-misspellings = Color de los errores ortográficos
setting-colors-misspellings-help = El subrayado de las palabras mal escritas, en la ventana; también se dicen. Elija un nombre o escriba un código hexadecimal. Predeterminado: el color del tema.
setting-colors-lint = Color de las marcas de formato
setting-colors-lint-help = El subrayado de los problemas de formato Markdown y de gramática, en la ventana; también se dicen. Elija un nombre o escriba un código hexadecimal. Predeterminado: el color del tema.
setting-colors-find-match = Color de las coincidencias
setting-colors-find-match-help = La banda detrás de las coincidencias; siguen subrayadas. Elija un nombre o escriba un código hexadecimal. Predeterminado: el color del tema.
setting-colors-selection = Color de la selección
setting-colors-selection-help = La banda detrás del texto seleccionado. Elija un nombre o escriba un código hexadecimal. Predeterminado: el color del tema.
setting-colors-focus = Color del foco
setting-colors-focus-help = El contorno del foco y el elemento con foco de una lista; siguen en negrita. Elija un nombre o escriba un código hexadecimal. Predeterminado: el color del tema.
setting-colors-links = Color de los enlaces
setting-colors-links-help = El color de los enlaces; siguen subrayados. Elija un nombre o escriba un código hexadecimal. Predeterminado: el color del tema.
setting-colors-headings = Color de los títulos
setting-colors-headings-help = El color de los títulos; siguen en negrita. Elija un nombre o escriba un código hexadecimal. Predeterminado: el color del tema.
setting-colors-status-bar = Color de la barra de estado
setting-colors-status-bar-help = La banda de las barras de estado y de título. Elija un nombre o escriba un código hexadecimal. Predeterminado: el color del tema.
setting-colors-notes = Color de las notas
setting-colors-notes-help = La banda detrás del texto con una nota; sigue en cursiva y subrayado. Elija un nombre o escriba un código hexadecimal. Predeterminado: el color del tema.
setting-colors-bookmarks = Color de los marcadores
setting-colors-bookmarks-help = La banda detrás de una palabra con marcador; sigue en negrita y subrayada. Elija un nombre o escriba un código hexadecimal. Predeterminado: el color del tema.
color-name-theme = color del tema
color-name-blue = azul
color-name-orange = naranja
color-name-navy = azul oscuro
color-name-skyblue = azul cielo
color-name-teal = verde azulado
color-name-gold = dorado
color-name-yellow = amarillo
color-name-purple = morado
color-name-pink = rosa
color-name-brown = marrón
color-name-gray = gris
color-name-black = negro
color-name-white = blanco
section-colors = Colores
colors-contrast-good = bueno
colors-contrast-fair = suficiente
colors-contrast-low = bajo
colors-item = { $label }: { $value }, contraste { $ratio } a 1, { $verdict }
colors-contrast = Contraste { $ratio } a 1, { $verdict }.
colors-contrast-warning = Por debajo de 3 a 1 se ve mal; elija un color más claro o más oscuro.
colors-intro = Colores, { $n } opciones. Izquierda y Derecha eligen un color con nombre, Intro escribe un nombre o un valor #rrggbb, Suprimir recupera el del tema, F1 dice la ayuda.
settings-item-recent = { $item }, cambiado hace poco
settings-reset = { $label } vuelve a su valor por defecto, { $value }.
settings-row-help = { $label }: { $value }. Por defecto: { $default }. { $help }
number-group-separator = .
number-decimal-separator = ,
settingsio-import-question-names =
    { $n ->
        [one] ¿Importar { $n } opción cambiada de { $name }: { $names }? y o n
       *[other] ¿Importar { $n } opciones cambiadas de { $name }: { $names }? y o n
    }
settingsio-and-more = { $names } y { $n } más


## Dictation in edit mode (ADR-0042). Keep the meaning first: a
## 40-cell Braille display shows the start of the line. $words are the
## dictated words, $key the dictate key, $dir a folder, $error and $text
## are passed on as they are.
dictation-status = Dictando: { $words }
dictation-listening = Dictando. Hable y pulse { $key } para parar.
dictation-finishing = Terminando el dictado.
dictation-done = Dictado terminado.
dictation-busy = El dictado está terminando. Inténtelo de nuevo en un momento.
dictation-needs-edit = El dictado escribe en modo de edición. ¿Activar el modo de edición y dictar? y o n
dictation-no-model = El dictado necesita el modelo Whisper en { $dir }. Consulte Dictation en la documentación.
dictation-failed = El dictado falló: { $error } Consulte Dictation en la documentación.
dictation-no-words = No se reconocieron palabras en esa frase.
dictation-lost = El dictado se detuvo antes de escribir sus últimas palabras.
dictation-not-typed = Palabras dictadas sin escribir, el modo de edición está desactivado: { $text }
setting-dictation-speak-while-recording = Hablar mientras se dicta
setting-dictation-speak-while-recording-help = Decir las palabras dictadas según llegan. Desactivado, se muestran en la línea de estado y se dicen en cada pausa, para que el micrófono no oiga la voz.
setting-dictation-model-dir = Carpeta del modelo de dictado
setting-dictation-model-dir-help = El modelo Whisper para el dictado. Sin valor usa whisper/rten/base.en en la carpeta de datos.
section-dictation = Dictado


## The file browser. Every row and introduction starts with the name,
## then the kind, so the first cells of a 40-cell Braille line hold what
## matters. $name is a file or folder name; $n a number that chooses the
## plural and $count the same number written with its separators.
# A list item with its position after it, in the file browser.
listmodel-item-position-last = { $item }, { $k } de { $n }
browse-places-title = Lugares
browse-places-intro =
    { $n ->
        [one] Lugares, 1 lugar.
       *[other] Lugares, { $n } lugares.
    }
# $purpose says what the folder or file is chosen for; $intro follows.
browse-choosing = { $purpose }. { $intro }
browse-place-document = { $name }, la carpeta del documento
browse-place-start = { $name }, carpeta de inicio
browse-place-library = { $name }, carpeta de la biblioteca
browse-place-disk = { $name }, disco
browse-place-removable = { $name }, unidad extraíble
browse-place-network = { $name }, unidad de red
browse-place-cd = { $name }, unidad de CD o DVD
browse-place-root = { $name }, la carpeta raíz
browse-choose-here = Elegir esta carpeta, { $name }
browse-row-folder = { $name }, carpeta
browse-row-folder-items =
    { $name }, carpeta, { $n ->
        [one] 1 elemento
       *[other] { $count } elementos
    }
# $kind is a kind below ("Markdown"); $size a size below ("12 KB").
browse-row-file = { $name }, { $kind }, { $size }
browse-row-kind = { $name }, { $kind }
browse-row-hidden = { $row }, oculto
# $kind is zip, tar, tar.gz, gzip, or 7z.
browse-kind-archive = archivo comprimido { $kind }
browse-kind-file = archivo
browse-kind-markdown = Markdown
browse-kind-text = texto
browse-kind-html = página web
browse-kind-epub = libro EPUB
browse-kind-docx = documento de Word
browse-kind-rtf = documento RTF
browse-kind-odt = texto OpenDocument
browse-kind-latex = LaTeX
browse-kind-eml = correo electrónico
browse-kind-mhtml = archivo web
browse-kind-pdf = PDF
browse-kind-image = imagen
browse-kind-daisy = libro DAISY
browse-kind-pptx = diapositivas de PowerPoint
browse-kind-sheet = hoja de cálculo
browse-kind-json = JSON
browse-kind-notebook = cuaderno de Jupyter
browse-kind-svg = dibujo SVG
browse-kind-mathml = fórmula MathML
browse-kind-pandoc = documento leído con Pandoc
browse-size-bytes =
    { $n ->
        [one] 1 byte
       *[other] { $count } bytes
    }
# $size is a number, with a decimal under 10 ("3.4").
browse-size-kb = { $size } KB
browse-size-mb = { $size } MB
browse-size-gb = { $size } GB
browse-intro =
    { $name }, { $n ->
        [one] 1 elemento.
       *[other] { $count } elementos.
    }
browse-intro-empty = { $name } no tiene nada que mostrar.
# $filter is what was typed.
browse-intro-filtered =
    { $name }, { $n ->
        [one] 1 elemento coincide con
       *[other] { $count } elementos coinciden con
    } { $filter }.
browse-intro-in-archive = { $intro } En { $archive }.
browse-intro-hidden =
    { $intro } { $n ->
        [one] 1 archivo oculto.
       *[other] { $count } archivos ocultos.
    }
browse-intro-cut = { $intro } Solo se muestran los primeros { $max }.
# $preview, $choose, $sort, and $all are keys; $item the focused row.
browse-keys = { $intro } Intro abre, Retroceso sube, escribir filtra. { $preview } muestra un avance, { $choose } elige una carpeta, { $sort } ordena, { $all } muestra todos los archivos. { $item }
browse-sorted-name = Ordenado por nombre.
browse-sorted-date = Ordenado por fecha, el más reciente primero.
browse-sorted-size = Ordenado por tamaño, el más grande primero.
browse-showing-all = Se muestran todos los archivos.
browse-showing-readable = Solo se muestran los archivos legibles.
browse-closed = Explorador de archivos cerrado.
browse-read-only = El explorador de archivos solo abre y elige archivos; nunca los cambia.
# $key is the Choose Folder key.
browse-choose-a-folder = Elija una carpeta: Intro abre una, { $key } la elige.
browse-choose-a-file = Elija un archivo: Intro elige uno.
browse-no-archive-folder = No se puede elegir una carpeta dentro de un archivo comprimido; elija una carpeta del disco.
browse-nothing-waiting = Ningún comando espera una carpeta; Intro la abre.
browse-cannot-read = { $name } no es un tipo de archivo que textweaver pueda leer.
browse-folder-unreadable = No se pudo abrir { $name }: { $reason }
browse-archive-too-deep = { $name } está dentro de demasiados archivos comprimidos para abrirlo.
browse-archive-too-large = { $name } es demasiado grande para listarlo con seguridad.
browse-archive-unreadable = { $name } no es un archivo comprimido que textweaver pueda leer; puede estar dañado.
# A document's preview: its title, then its first sentence.
browse-preview-document = { $title }. { $sentence }
browse-preview-no-text = { $title }. No tiene texto.
browse-preview-failed = No se pudo mostrar un avance de { $name }: { $reason }
# $names are the first few names inside.
browse-preview-archive =
    { $name }: { $n ->
        [one] 1 archivo
       *[other] { $count } archivos
    }, { $readable } legibles. { $names }
browse-preview-archive-folder =
    { $name }, carpeta del archivo comprimido, { $n ->
        [one] 1 elemento.
       *[other] { $count } elementos.
    }
browse-preview-folder = { $path }: { $names }
browse-preview-folder-empty = { $path }: nada que leer aquí.
browse-preview-other = { $name }, { $size }; textweaver no puede leer este tipo de archivo.
browse-preview-path = { $path }


## Batch conversion (File, Batch convert). Keep the meaning first.
batch-choose-source = Elija la carpeta que desea convertir
batch-choose-output = Elija la carpeta para los archivos convertidos
batch-format-title = Convertir a
batch-format-intro = Convertir { $name } a: elija un formato, { $n } opciones.
batch-where-title = Dónde van los archivos
batch-where-intro = ¿Dónde deben ir los archivos convertidos?
batch-where-converted = En una carpeta converted, { $path }
batch-where-beside = Junto a cada archivo
batch-where-choose = En otra carpeta, que elegirá a continuación
batch-nothing = No hay documentos que convertir en { $path }.
batch-confirm =
    ¿Convertir { $n ->
        [one] 1 archivo
       *[other] { $n } archivos
    } a { $format } en { $path }? y o n
batch-confirm-beside =
    ¿Convertir { $n ->
        [one] 1 archivo
       *[other] { $n } archivos
    } a { $format } junto a cada archivo? y o n
batch-started =
    Convirtiendo { $n ->
        [one] 1 archivo
       *[other] { $n } archivos
    } a { $format }. Escape detiene.
batch-progress = { $percent } por ciento convertido, { $done } de { $total } archivos.
batch-busy = Ya se está convirtiendo, { $done } de { $total } archivos. Escape detiene.
batch-stop-question = ¿Detener la conversión? Los archivos ya hechos se conservan. y o n
batch-stopping = Se detendrá tras los archivos que se están escribiendo.
batch-still-converting = La conversión continúa.
batch-done =
    { $converted ->
        [one] 1 archivo convertido
       *[other] { $converted } archivos convertidos
    } a { $format }; { $skipped } al día; { $failed } con error.
batch-stopped =
    Detenido. { $converted ->
        [one] 1 archivo convertido
       *[other] { $converted } archivos convertidos
    }; { $left } sin convertir; { $failed } con error.
batch-report = Informe guardado en { $path }.
batch-report-failed = No se pudo guardar el informe: { $error } Los archivos convertidos se conservan.
batch-inaccessible =
    { $n ->
        [one] 1 archivo tiene
       *[other] { $n } archivos tienen
    } elementos no accesibles; vea el informe.
batch-failures-title =
    { $n ->
        [one] 1 archivo con error
       *[other] { $n } archivos con error
    }
batch-failure-item = { $name }: { $reason }
batch-start-failed = No se pudo empezar a convertir: { $error } Compruebe la carpeta y el formato, y vuelva a intentarlo.
batch-thread-stopped = La conversión por lotes se detuvo de forma inesperada.


## Audio export (File, Export audio). Keep the meaning first: a
## 40-cell Braille display shows the start of the line. $name is a file
## name (essay.flac); $path a folder or a file's full path; $format a
## format's name (FLAC, MP3); $voice a voice's or engine's name; $wpm is
## words per minute; $length a length of time from the duration-*
## messages; $chapters and $n are numbers; $percent is a multiple of ten;
## $formats lists format names (M4B); $error is passed on as it is.
audio-format-title = Exportar audio como
audio-format-intro = Exportar { $name } como audio: elija un formato, { $n } opciones.
audio-no-ffmpeg =
    { $formats } { $n ->
        [one] necesita
       *[other] necesitan
    } ffmpeg, que no se encontró.
audio-format-flac = FLAC: sin pérdida, la mitad del tamaño de WAV
audio-format-wav = WAV: el más grande, se reproduce en todas partes
audio-format-mp3 = MP3: pequeño, se reproduce en todas partes
audio-format-opus = Opus: el más pequeño, pensado para la voz
audio-format-ogg = Ogg Vorbis: pequeño y abierto, se reproduce en la mayoría de los reproductores
audio-format-m4b = Audiolibro M4B, mediante ffmpeg
audio-format-mp4 = Vídeo con subtítulos: MP4, necesita ffmpeg
audio-format-html = Página de lectura: texto y audio, un archivo
audio-where-title = Dónde va el audio
audio-where-intro = ¿Dónde debe ir el audio?
audio-where-beside = Junto al documento, { $path }
audio-where-choose = En otra carpeta, elegida a continuación
audio-choose-folder = Elija la carpeta para el audio
audio-no-engine = Ningún motor de voz de aquí puede escribir archivos de audio. Instale eSpeak NG, o elija otro motor en el menú Voz.
audio-confirm = ¿Exportar { $name } con { $voice } a { $wpm } palabras por minuto, en { $path }? y o n
audio-started = Exportando { $name } como { $format }. Escape detiene.
audio-progress = Exportando audio, { $percent } por ciento.
audio-video-progress =
    { $minutes ->
        [one] Codificando video: { $fed } de { $all } fotogramas, falta 1 minuto.
       *[other] Codificando video: { $fed } de { $all } fotogramas, faltan unos { $minutes } minutos.
    }
audio-video-progress-soon = Codificando video: { $fed } de { $all } fotogramas, falta menos de un minuto.
audio-busy = Ya se está exportando { $name }. Escape detiene.
audio-stop-question = ¿Detener la exportación? No se guarda ningún archivo. y o n
audio-stopping = Deteniendo la exportación.
audio-still-exporting = Se sigue exportando el audio.
audio-stopped = Exportación de audio detenida; no se escribió ningún archivo.
audio-done =
    { $name } escrito: { $length }, { $chapters ->
        [one] 1 capítulo
       *[other] { $chapters } capítulos
    }.
audio-subtitles = Subtítulos en { $name }.
audio-failed = No se pudo exportar el audio: { $error } Pruebe otro formato u otra voz.
audio-thread-stopped = La exportación de audio se detuvo de forma inesperada.


## The window's menus and dialogs. Settings files chosen with the
## system's file chooser, the Colors dialog, and the font list. $ratio is
## a contrast ratio such as 4.8; $verdict is good, fair, or low.
gui-settings-files = Archivos de configuración
gui-settings-export-title = Exportar la configuración
gui-settings-import-title = Importar la configuración
gui-chooser-no-dialog = El selector de archivos del sistema no se abrió. Escriba la ruta del archivo.
gui-colors-value = { $value }, contraste { $ratio } a 1, { $verdict }
gui-colors-help = Izquierda y Derecha eligen un color con nombre, primero azul y naranja. Intro escribe un nombre o un valor #rrggbb. Suprimir recupera el color del tema. Cada marca conserva su subrayado, su grosor o su símbolo, sea cual sea su color.
gui-colors-reset-all = Restablecer todos los colores
gui-colors-reset-all-help = Recuperar el color propio del tema en cada parte.
gui-colors-reset-done = Todos los colores vuelven a ser los del tema.
colors-reset-question = ¿Restablecer todos los colores a los del tema? y o n
gui-colors-closed = Colores cerrados.
gui-font-list-intro =
    { $n ->
        [one] { $title }, 1 familia.
       *[other] { $title }, { $n } familias.
    }


## PDF links. Said before the first line of the page a link inside
## a PDF goes to, when the page has no heading there (as links-heading-label
## is for a heading). $page is the page's printed number or label (12, iv).
links-page-label = Página { $page }


## Sync wave, S4: sync in the reader (ADR-0049).
sync-status-off = Sincronización: desactivada
sync-status-not-set-up = Sincronización: sin configurar
sync-status-starting = Sincronización: iniciando
sync-status-folder-missing = Sincronización: falta la carpeta, se guarda aquí
sync-status-read-only = Sincronización: formato más nuevo, solo lectura
sync-status-failed = Sincronización: no se puede usar la carpeta
sync-status-cannot-write = Sincronización: no se puede escribir, se guarda aquí
sync-status-clock-ahead = Sincronización: el reloj de { $device } va adelantado
sync-status-damaged =
    { $n ->
        [one] Sincronización: 1 archivo dañado omitido
       *[other] Sincronización: { $n } archivos dañados omitidos
    }
sync-status-up-to-date = Sincronización: al día
sync-status-this-computer = Este equipo: { $name }.
sync-status-no-others = Todavía no hay otros equipos.
sync-status-others = Otros equipos: { $names }.
sync-status-error = Problema: { $error } Compruebe la carpeta de sincronización.
sync-another-computer = otro equipo
sync-untitled = un documento
sync-damaged = Sincronización: archivo dañado de { $device } omitido.
sync-newer-file = Sincronización: archivo más nuevo de { $device } omitido.
sync-read-only = Sincronización: formato más nuevo, solo lectura.
sync-clock-ahead = Sincronización: el reloj de { $device } va { $hours } horas adelantado.
sync-fresh-id = Sincronización: configuración copiada; nuevo identificador.
sync-write-failed = Sincronización: no se puede escribir. { $error } Por ahora se guarda en este equipo.
sync-name-refused = Nombre no permitido. Pruebe uno como portátil.
sync-note-replaced =
    { $n ->
        [one] { $title }: una nota fue reemplazada por la edición más reciente de { $device }.
       *[other] { $title }: { $n } notas fueron reemplazadas por las ediciones más recientes de { $device }.
    }
sync-restored-notes =
    { $n ->
        [one] { $title }: una nota borrada volvió, editada en { $device }.
       *[other] { $title }: { $n } notas borradas volvieron, editadas en { $device }.
    }
sync-restored-bookmarks =
    { $n ->
        [one] { $title }: un marcador borrado volvió, editado en { $device }.
       *[other] { $title }: { $n } marcadores borrados volvieron, editados en { $device }.
    }
sync-restored-highlights =
    { $n ->
        [one] { $title }: un resaltado borrado volvió, editado en { $device }.
       *[other] { $title }: { $n } resaltados borrados volvieron, editados en { $device }.
    }
sync-arrived =
    { $n ->
        [one] { $title }: 1 cambio de { $device }.
       *[other] { $title }: { $n } cambios de { $device }.
    }
sync-resumed = { $title }: reanudado en el { $pct } por ciento, desde { $device }.
sync-place-arrived = Lugar de { $device }: { $pct } por ciento.
sync-place-question = { $device } en el { $pct } por ciento. ¿Ir allí? y o n
sync-suggestion-question =
    { $n ->
        [one] Puede ser { $title } de { $device }, con 1 nota. ¿Usarla? y o n
       *[other] Puede ser { $title } de { $device }, con { $n } notas. ¿Usarlas? y o n
    }
sync-went-to-place = Lugar de { $device }, { $pct } por ciento.
sync-kept-place = Se mantiene este lugar.
sync-suggestion-accepted = Se usan las notas de { $device }.
sync-suggestion-declined = Se mantienen separados.
sync-sidecar-differed =
    { $n ->
        [one] Sincronización: 1 lugar de la biblioteca difería.
       *[other] Sincronización: { $n } lugares de la biblioteca diferían.
    }
sync-sidecar-failed = Sincronización: no se puede escribir un lugar de la biblioteca. { $error } Compruebe que se puede escribir en la carpeta de la biblioteca.
sync-no-state = La sincronización está desactivada en esta sesión: no se guarda nada.
sync-choose-folder = Elija la carpeta de sincronización
sync-group-places = Lugares
sync-group-notes = Notas
sync-group-highlights = Resaltados
sync-group-bookmarks = Marcadores
sync-group-statistics = Estadísticas
sync-group-item = { $name }: { $state }
sync-start = Empezar a sincronizar
sync-groups-title = Qué se sincroniza
sync-groups-intro = { $title }, como { $name }. Intro activa o desactiva; Empezar a sincronizar termina.
sync-started = Sincronización activada, como { $name }. Estado: { $key }.
sync-how-to-set-up = Para configurarla: Herramientas, Sincronización, Configurar la sincronización.
sync-now-started = Sincronizando.
sync-now-done =
    { $n ->
        [one] Sincronización: al día, 1 documento revisado.
       *[other] Sincronización: al día, { $n } documentos revisados.
    }
sync-now-changed =
    { $n ->
        [one] Sincronización: 1 documento recibió cambios.
       *[other] Sincronización: { $n } documentos recibieron cambios.
    }
sync-no-places = Ningún otro equipo tiene un lugar aquí.
sync-place-item = { $device }, { $pct } por ciento
sync-places-title =
    { $n ->
        [one] 1 lugar de otro equipo
       *[other] { $n } lugares de otros equipos
    }
sync-no-replaced = No hay notas reemplazadas en este documento.
sync-replaced-item = { $text }, reemplazada por { $device }
sync-replaced-item-deleted = { $text }, borrada por { $device }
sync-replaced-title =
    { $n ->
        [one] 1 nota reemplazada
       *[other] { $n } notas reemplazadas
    }
sync-replaced-intro = { $title }. Intro restaura una.
sync-note-restored = Nota restaurada: { $text }
sync-already-off = La sincronización ya está desactivada aquí.
sync-stopped = Sincronización desactivada aquí. La carpeta queda como está.
prompt-sync-computer-name = Nombre de este equipo, Intro lo mantiene
menu-sync = Sincronización
name-sync-setup = Configurar la sincronización
name-sync-status = Estado de la sincronización
name-sync-now = Sincronizar ahora
name-sync-go-to-place = Ir al lugar de otro equipo
name-sync-replaced-notes = Notas reemplazadas
name-sync-stop = Dejar de sincronizar en este equipo
action-sync-setup = Configurar la sincronización: elegir la carpeta, nombrar este equipo y elegir qué se sincroniza
action-sync-status = Decir cómo va la sincronización (al día, falta la carpeta o un problema) y nombrar los otros equipos
action-sync-now = Sincronizar ahora: enviar los cambios de este equipo y tomar los de los demás en cada documento
action-sync-go-to-place = Listar los lugares de los otros equipos en este documento; Intro va a uno
action-sync-replaced-notes = Listar las notas reemplazadas por una edición más reciente de otro equipo; Intro restaura una
action-sync-stop = Dejar de sincronizar en este equipo; la carpeta de sincronización queda como está
section-sync = Sincronización
setting-sync-enabled = Sincronización
setting-sync-enabled-help = Compartir notas, resaltados, marcadores y lugares con sus otros equipos mediante la carpeta de sincronización. Herramientas, Sincronización, Configurar la sincronización la activa.
setting-sync-folder = Carpeta de sincronización
setting-sync-folder-help = La carpeta que comparten sus equipos. Una que Syncthing mantiene al día, una carpeta en la nube o una memoria USB.
setting-sync-device-name = Nombre del equipo
setting-sync-device-name-help = El nombre de este equipo en los mensajes de sincronización, como portátil o laboratorio. Vacío usa Computer 1, Computer 2 y así sucesivamente.
setting-sync-places = Sincronizar lugares
setting-sync-places-help = Compartir dónde está en cada documento.
setting-sync-notes = Sincronizar notas
setting-sync-notes-help = Compartir notas.
setting-sync-highlights = Sincronizar resaltados
setting-sync-highlights-help = Compartir resaltados.
setting-sync-bookmarks = Sincronizar marcadores
setting-sync-bookmarks-help = Compartir marcadores.
setting-sync-statistics = Sincronizar estadísticas
setting-sync-statistics-help = Compartir el tiempo de lectura y las sesiones de cada equipo.
setting-sync-position-policy = Lugar para reanudar
setting-sync-position-policy-help = En qué lugar se abre un documento cuando otro equipo también tiene uno. El más reciente, el más avanzado o preguntar.
choice-sync-position-policy-newest = el más reciente
choice-sync-position-policy-furthest = el más avanzado
choice-sync-position-policy-ask = preguntar

## End of S4

## Sync wave, S5 (see en.ftl).
sync-settings-arrived =
    { $n ->
        [one] Ajustes: 1 cambio de { $device }.
       *[other] Ajustes: { $n } cambios de { $device }.
    }
sync-settings-arrived-several = Ajustes: { $n } cambios de { $computers } equipos.
sync-kept-keys-mac =
    { $n ->
        [one] Teclas de Mac: 1, guardada, sin usar aquí.
       *[other] Teclas de Mac: { $n }, guardadas, sin usar aquí.
    }
sync-kept-keys-pc =
    { $n ->
        [one] Teclas de Windows y Linux: 1, guardada, sin usar aquí.
       *[other] Teclas de Windows y Linux: { $n }, guardadas, sin usar aquí.
    }
sync-group-settings = Ajustes
sync-group-profiles = Perfiles
sync-group-key-overrides = Teclas propias
sync-group-words = Lista de palabras
sync-group-glossary = Glosario y pronunciaciones
sync-group-favorite-voices = Voces favoritas
voices-missing-row = { $voice }, favorita, no está en este equipo
voices-missing = { $voice } no está en este equipo. Espacio la quita de favoritas.
gui-voices-list = Voces
gui-voices-use = Usar voz
gui-voices-use-help = Usar la voz enfocada y oír una muestra, o descargarla tras una pregunta.
gui-voices-preview = Probar
gui-voices-preview-help = Oír una muestra con la voz enfocada sin elegirla.
gui-voices-favorite = Favorita
gui-voices-favorite-help = Marcar la voz enfocada como favorita, o quitarla. Las favoritas van primero.
gui-voices-remove = Eliminar
gui-voices-remove-help = Eliminar la voz Piper descargada enfocada, tras una pregunta.
gui-voices-remove-unavailable = no disponible
gui-voices-language-help = Mostrar solo las voces del idioma siguiente, y luego todos los idiomas.
gui-voices-engine-help = Mostrar solo las voces del motor siguiente, y luego todos los motores.
gui-voices-fetch-help = Descargar la lista de voces Piper, unos 250 KB, tras una pregunta.
gui-voices-close-help = Cerrar el gestor de voces.
gui-voices-hint = Intro usa la voz, Espacio marca una favorita, Escape cierra.
setting-sync-settings = Sincronizar ajustes
setting-sync-settings-help = Compartir los ajustes portátiles: velocidad, puntuación, tema, ayudas de lectura y otros. La voz, el motor, el modo de acceso, el juego de teclas y las rutas quedan en cada equipo.
setting-sync-profiles = Sincronizar perfiles
setting-sync-profiles-help = Compartir los perfiles; el que está en uso queda en cada equipo.
setting-sync-key-overrides = Sincronizar teclas propias
setting-sync-key-overrides-help = Compartir keymap.toml. Las teclas de un Mac se guardan pero no se usan en Windows ni Linux, y al revés.
setting-sync-words = Sincronizar lista de palabras
setting-sync-words-help = Compartir la lista de palabras de ortografía.
setting-sync-glossary = Sincronizar glosario
setting-sync-glossary-help = Compartir las entradas del glosario y las pronunciaciones.
setting-sync-favorite-voices = Sincronizar voces favoritas
setting-sync-favorite-voices-help = Compartir las voces favoritas. Una que este equipo no tiene aparece como no está en este equipo.

## End of S5

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

## Selectores de archivos y carpetas.
prompt-browse-hint = { $label }. { $key } para explorar.
prompt-browse-file = Elija el archivo: { $label }
prompt-browse-folder = Elija la carpeta: { $label }
prompt-browse-filled = { $name } elegido. Intro confirma.
chooser-type-files = Archivos { $type }
chooser-image-title = Insertar una imagen
chooser-images = Imágenes
chooser-references-title = Importar referencias
chooser-reference-files = Archivos de referencias
chooser-profiles-import-title = Importar perfiles
chooser-profiles-export-title = Exportar perfiles
chooser-profile-files = Archivos de perfiles
gui-folder-no-dialog = El selector de carpetas del sistema no se abrió. Elija la carpeta en esta lista.
gui-prompt-browse-hint = { $key } abre el explorador de archivos.

## Componentes opcionales.
name-manage-components = Administrar componentes opcionales…
name-download-dictation-model = Descargar el modelo de dictado
action-manage-components = Administrar componentes opcionales: los modelos, fuentes y voces que textweaver puede descargar, con su tamaño y licencia
action-download-dictation-model = Descargar el modelo de dictado elegido en la configuración, tras decir su tamaño y licencia
component-feature-dictation = el dictado
component-feature-ocr = leer páginas escaneadas
component-feature-reading-font = una fuente de lectura
component-feature-voice = una voz
component-state-installed = instalado
component-state-not-installed = no instalado
component-state-partial = instalado en parte
component-state-damaged = dañado
component-state-downloading = descargando
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
dictation-model-not-in-build = Sin modelo ni descargas en esta versión.
dictation-model-file-missing = Al modelo le falta { $file }.
dictation-model-damaged = Modelo de dictado dañado: { $file }.
dictation-model-no-folder = No existe la carpeta: { $dir }.

## Configuración de los componentes opcionales.
section-components = Componentes opcionales
setting-dictation-model = Modelo de dictado
setting-dictation-model-help = El modelo Whisper que usa el dictado cuando no hay carpeta. Descargar el modelo de dictado, en el menú Herramientas, lo obtiene.
choice-dictation-model-whisper-base-en = base.en, el predeterminado
choice-dictation-model-whisper-small-en = small.en, más grande y preciso
setting-components-source = Origen de componentes
setting-components-source-help = Sus propios componentes, usados primero. Un repositorio de GitHub como propietario/nombre, o una carpeta en este equipo. Vacío no usa ninguno. Nunca ponga aquí una contraseña.
setting-components-mirror = Espejo de componentes
setting-components-mirror-help = De dónde vienen primero los componentes opcionales: una dirección https o una carpeta en este equipo. Vacío usa sus orígenes públicos. Nunca ponga aquí una contraseña.

## Help's ways to the docs, About's facts, and first-run choices asked
## again. $address is a web address; $path a folder; facts start with
## their name so each Braille line leads with it.
name-quick-start = Inicio rápido
name-documentation = Documentación…
name-report-problem = Informar de un problema…
name-ask-first-run-again = Repetir preguntas del primer inicio
action-quick-start = Abrir la guía de inicio rápido en textweaver
action-documentation = Mostrar la dirección web de la documentación y preguntar antes de abrirla en un navegador
action-report-problem = Mostrar dónde informar de un problema y preguntar antes de abrirlo en un navegador; no se envía nada
action-ask-first-run-again = Volver a preguntar las opciones del primer inicio: el modo híbrido con un lector de pantalla y los componentes opcionales
about-title = Acerca de textweaver
about-intro = Acerca de textweaver, { $n } datos. Inclúyalos en un informe de problemas.
about-version = Versión: textweaver { $version }
about-build = Compilación: { $frontend }, { $profile }, { $os } { $arch }
about-frontend-window = ventana
about-frontend-terminal = lector de terminal
about-profile-release = publicación
about-profile-debug = depuración
about-license = Licencia: { $license }
about-engine-in-use = Motor de voz en uso: { $engine }
about-engines = Motores de voz encontrados: { $engines }
about-engines-none = Motores de voz encontrados: ninguno
about-components = Componentes: { $installed } de { $known } instalados
about-folder-settings = Carpeta de ajustes: { $path }
about-folder-data = Carpeta de datos: { $path }
about-folder-cache = Carpeta de caché: { $path }
about-no-folders = Carpetas: ninguna; esta sesión no guarda archivos.
about-quick-start-online = Inicio rápido no encontrado junto a textweaver. ¿Abrir { $address } en el navegador? y o n
about-docs-question = Documentación: { $address }. ¿Abrirla en el navegador? y o n
about-report-question = Informar de un problema: { $address }. No se envía nada. ¿Abrirlo en el navegador? y o n
about-first-run-again = Opciones del primer inicio restablecidas; se preguntan al próximo inicio.

## W9b-x: las opciones de lectura (Ver, Opciones de lectura).
name-reading-form = Opciones de lectura
action-reading-form = Abrir las opciones de lectura: velocidad, fuente, espaciado, longitud de línea, tema, resaltado, regla, lectura biónica y sílabas
reading-form-intro = Opciones de lectura, { $n } opciones. Izquierda y Derecha cambian un valor, Intro escribe uno, Suprimir recupera el predeterminado, F1 dice la ayuda.
reading-form-spacing-wcag-done = Espaciado con los valores de WCAG.
reading-form-spacing-generous-done = Espaciado amplio, más que WCAG.
spacing-letter-without-word = Suba el espacio entre palabras con el de letras.
gui-reading-form-help = Arriba y Abajo mueven, Izquierda y Derecha cambian un valor, Intro escribe uno, F1 dice la ayuda.
gui-reading-voices = Voces
gui-reading-voices-help = Abrir el gestor de voces.
gui-reading-wcag = Espaciado WCAG
gui-reading-wcag-help = Poner los cuatro espaciados en los valores de WCAG.
gui-reading-generous = Espaciado amplio
gui-reading-generous-help = Poner los cuatro espaciados más amplios que WCAG.
gui-reading-closed = Opciones de lectura cerradas.
## End of W9b-x

## B1-t1: cambios con seguimiento y comentarios (la lista de cambios).
name-list-changes = Cambios y comentarios
action-list-changes = Mostrar los cambios con seguimiento y los comentarios: Intro va a uno, A acepta un cambio, R lo rechaza
name-accept-all-changes = Aceptar todos los cambios
action-accept-all-changes = Aceptar todos los cambios con seguimiento del documento
name-reject-all-changes = Rechazar todos los cambios
action-reject-all-changes = Rechazar todos los cambios con seguimiento del documento
name-add-comment = Añadir comentario
action-add-comment = Añadir un comentario a la selección o a la oración del cursor
prompt-comment-reply = Respuesta
prompt-comment-text = Comentario
changes-title = Cambios y comentarios
changes-intro =
    { $n ->
        [one] { $title }, { $n } elemento. Intro va a él. A acepta un cambio, R lo rechaza, y con Mayús cualquiera de las dos hace todos los cambios de su autor. En un comentario, F2 responde, Espacio lo resuelve, Suprimir lo borra. N añade un comentario.
       *[other] { $title }, { $n } elementos. Intro va a uno. A acepta un cambio, R lo rechaza, y con Mayús cualquiera de las dos hace todos los cambios de su autor. En un comentario, F2 responde, Espacio lo resuelve, Suprimir lo borra. N añade un comentario.
    }
changes-none = No hay cambios con seguimiento ni comentarios en este documento.
changes-row = { $kind }: «{ $text }», { $who }, { $when }
changes-kind-inserted = Insertado
changes-kind-deleted = Eliminado
changes-kind-moved-away = Movido de aquí
changes-kind-moved-here = Movido aquí
changes-by = por { $author }
changes-no-author = autor no registrado
changes-no-date = fecha no registrada
changes-date = { $weekday }, { $day } de { $month } de { $year }
changes-weekday-0 = domingo
changes-weekday-1 = lunes
changes-weekday-2 = martes
changes-weekday-3 = miércoles
changes-weekday-4 = jueves
changes-weekday-5 = viernes
changes-weekday-6 = sábado
changes-month-1 = enero
changes-month-2 = febrero
changes-month-3 = marzo
changes-month-4 = abril
changes-month-5 = mayo
changes-month-6 = junio
changes-month-7 = julio
changes-month-8 = agosto
changes-month-9 = septiembre
changes-month-10 = octubre
changes-month-11 = noviembre
changes-month-12 = diciembre
changes-comment = Comentario: { $text }
changes-comment-by = Comentario de { $author }: { $text }
changes-replies =
    { $n ->
        [one] { $n } respuesta
       *[other] { $n } respuestas
    }
changes-resolved = resuelto
changes-accepted = Aceptado. { $kind }: «{ $text }».
changes-rejected = Rechazado. { $kind }: «{ $text }».
changes-accepted-all =
    { $n ->
        [one] { $n } cambio aceptado.
       *[other] { $n } cambios aceptados.
    }
changes-rejected-all =
    { $n ->
        [one] { $n } cambio rechazado.
       *[other] { $n } cambios rechazados.
    }
changes-accepted-author =
    { $n ->
        [one] { $n } cambio de { $author } aceptado.
       *[other] { $n } cambios de { $author } aceptados.
    }
changes-rejected-author =
    { $n ->
        [one] { $n } cambio de { $author } rechazado.
       *[other] { $n } cambios de { $author } rechazados.
    }
changes-none-left = No hay cambios con seguimiento que aceptar o rechazar.
changes-not-a-change = Esta fila es un comentario. F2 responde, Espacio lo resuelve, Suprimir lo borra.
changes-not-a-comment = Esta fila es un cambio. A lo acepta, R lo rechaza.
changes-edit-mode = En el modo de edición los cambios quedan como están. Salga del modo de edición para aceptarlos o rechazarlos.
changes-replied = Respuesta añadida.
changes-resolved-done = Comentario resuelto.
changes-reopened = Comentario abierto de nuevo.
changes-comment-deleted = Comentario y respuestas borrados.
changes-comment-added = Comentario añadido.
changes-delete-comment-question = ¿Borrar este comentario y sus respuestas? y o n
changes-written-accepted =
    { $n ->
        [one] { $path } escrito con { $n } cambio aceptado.
       *[other] { $path } escrito con { $n } cambios aceptados.
    }
changes-written-rejected =
    { $n ->
        [one] { $path } escrito con { $n } cambio rechazado.
       *[other] { $path } escrito con { $n } cambios rechazados.
    }
## End of B1-t1
## B1-r5: braille (BRF) files read as print. $page is a braille page
## number ("3", "p1"); $n is a number of lines; $reason is an error.
setting-braille-brf-code = Código braille de los archivos BRF
setting-braille-brf-code-help = El código braille en que se leen los archivos BRF. UEB es para libros hechos desde 2016; EBAE, English Braille American Edition, para libros más antiguos. Leer un archivo BRF como texto impreso necesita liblouis. Vuelva a abrir el archivo después de un cambio.
choice-braille-brf-code-ueb = UEB
choice-braille-brf-code-ebae = EBAE
name-show-original-braille = Mostrar el braille original
action-show-original-braille = Mostrar el braille original de la página del cursor, en un archivo BRF leído como texto impreso
brf-original-title = Braille original, página { $page }
brf-original-intro =
    { $n ->
        [one] Braille original, página { $page }, { $n } línea. Escape cierra.
       *[other] Braille original, página { $page }, { $n } líneas. Escape cierra.
    }
brf-original-not-brf = No es un archivo braille. Mostrar el braille original funciona con archivos BRF.
brf-original-unreadable = No se puede leer el archivo braille: { $reason }
brf-no-liblouis = Braille mostrado como braille: falta liblouis. Para leerlo como texto impreso, instale liblouis desde liblouis.io o los paquetes de su sistema y vuelva a abrir el archivo.
daisy-headings-only = Solo encabezados: este libro DAISY no tiene texto, solo encabezados y audio, así que se leen sus encabezados.
## End of B1-r5
