### Authoring: lists, citations, find and replace, tables, quick wins, and dictation.

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

## Citations: inserting, looking up, importing, checking, and the bibliography.

citations-on = Citas activadas.
citations-off = Citas desactivadas.
book-audio-on = Audio del libro activado. La grabación suena donde el libro la tiene.
book-audio-off = Audio del libro desactivado. La voz lee el libro.
book-audio-rate = Los cambios de velocidad no se aplican al audio grabado.
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
authoring-language-mermaid = diagrama Mermaid
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
dictation-paused = Dictado en pausa.
dictation-resumed = Dictado reanudado.
dictation-not-active = No se está dictando.
dictation-cannot-pause = Este dictado no puede pausarse; deténgalo en su lugar.
section-dictation = Dictado
