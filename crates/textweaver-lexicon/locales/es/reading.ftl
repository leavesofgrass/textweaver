### Reading: navigation, reading aloud, status, statistics, the queue, and reading aids.

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

## La cola de lectura y la lista de repaso (beta 3).

queue-title = Cola de lectura
queue-intro =
    { $n ->
        [one] Cola de lectura, 1 documento. Intro lo abre, Suprimir lo quita.
       *[other] Cola de lectura, { $n } documentos. Intro abre uno, Suprimir lo quita.
    }
queue-empty = La cola de lectura está vacía. En la biblioteca, { $key } añade un documento.
queue-item-missing = { $title }, no encontrado
queue-item-elsewhere = { $title }, no está en este equipo
queue-untitled = Un documento de otro equipo
queue-elsewhere-chosen = { $title } no está en este equipo. Sigue en la cola de lectura, y se encuentra en cuanto lo abra aquí.
queue-added =
    { $n ->
        [one] Añadido a la cola de lectura: { $title }. 1 documento en la cola.
       *[other] Añadido a la cola de lectura: { $title }. { $n } documentos en la cola.
    }
queue-removed = Quitado de la cola de lectura: { $title }.
queue-no-file = Solo un documento guardado como archivo puede entrar en la cola de lectura.
queue-read-failed = No se pudo leer la biblioteca para la cola de lectura: { $error }. Revise library.json en la carpeta de datos; tw settings path la indica.
queue-next-ask = Siguiente: { $title }. { $key } para leer.
queue-next-reading = Siguiente: { $title }.
review-title = Lista de repaso
review-intro =
    { $n ->
        [one] Lista de repaso, 1 documento, el leído hace más tiempo primero. Intro lo abre.
       *[other] Lista de repaso, { $n } documentos, el leído hace más tiempo primero. Intro abre uno.
    }
review-empty = Aún no hay nada que repasar. Aquí aparecen los documentos leídos en voz alta.
review-stats-off = La lista de repaso necesita las estadísticas de lectura, que están desactivadas. Active Estadísticas de lectura en la configuración.
review-item = { $title }, { $days }, { $pct } por ciento, { $notes }
review-days =
    { $n ->
        [0] hoy
        [one] 1 día
       *[other] { $n } días
    }
review-notes =
    { $n ->
        [0] sin notas
        [one] 1 nota
       *[other] { $n } notas
    }
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
playback-caps-no-words = Resaltado de palabras estimado: esta voz no informa de las palabras.
playback-caps-words = Resaltado de palabras exacto: esta voz informa de cada palabra.

## Reading aloud.

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
status-cw-rate = CW { $wpm } ppm
status-cw-rate-spoken = CW a { $wpm } palabras por minuto
status-cw-rate-farnsworth = CW { $wpm } ppm, efectiva { $effective }
status-cw-rate-farnsworth-spoken = CW a { $wpm } palabras por minuto, efectiva { $effective }
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

## Summaries and difficult-word definitions.
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
settings-unit-sentences =
    { $n ->
        [one] oración
       *[other] oraciones
    }

# The GUI. Said in textweaver's own voice when the window takes the
# focus; $title is the document's title.
gui-window-focused = { $title }, { -brand }.
gui-updated-to = Actualizado a { $version }. Ayuda, Documentación describe las novedades.
