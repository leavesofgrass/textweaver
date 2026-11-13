### Notes, highlights, bookmarks, relations, and find.

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
        [one] Notas, 1 elemento. Intro va a una nota, Suprimir la elimina, F2 la edita, Espacio abre sus enlaces. C crea una tarjeta.
       *[other] Notas, { $n } elementos. Intro va a una nota, Suprimir la elimina, F2 la edita, Espacio abre sus enlaces. C crea una tarjeta.
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
notes-highlighted-at = Resaltado, { $name }, en el { $pct } por ciento: { $text }
notes-highlighted = Resaltado, { $name }: { $text }
# An item in the highlights list. $name is the highlight's palette name;
# $lost is yes when the text was not found after the file changed.
notes-highlight-item =
    { $lost ->
        [yes] { $name }: { $text }, línea { $line }, no encontrado después de que el archivo cambiara
       *[no] { $name }: { $text }, línea { $line }
    }
notes-no-highlights = Sin resaltados. Para crear uno: { $key }.
notes-highlights-title = Resaltados
notes-highlights-intro =
    { $n ->
        [one] Resaltados, 1 elemento. Intro va a uno, Suprimir lo quita, F2 cambia su nombre, Espacio muestra solo su nombre. C crea una tarjeta.
       *[other] Resaltados, { $n } elementos. Intro va a uno, Suprimir lo quita, F2 cambia su nombre, Espacio muestra solo su nombre. C crea una tarjeta.
    }
# The label said before a highlight's text on jumping to it.
notes-highlight-label = Resaltado, { $name }
notes-highlight-changed = Resaltado cambiado a { $name }: { $text }
# $n is the palette entry's number, $key the keys of Highlight with a name.
notes-palette-no-entry = No hay nombre de resaltado { $n } en la paleta. Para elegir un nombre: { $key }.
notes-highlights-named-title = Resaltados: { $name }
notes-highlights-named-intro =
    { $n ->
        [one] Resaltados llamados { $name }, 1 elemento. Espacio muestra todos los resaltados.
       *[other] Resaltados llamados { $name }, { $n } elementos. Espacio muestra todos los resaltados.
    }
notes-palette-title = Nombres de resaltado
# A palette name: the name, how many highlights have it, its shape and its color.
notes-palette-row =
    { $count ->
        [one] { $name }, 1 resaltado, { $shape }, { $color }
       *[other] { $name }, { $count } resaltados, { $shape }, { $color }
    }
notes-palette-intro-highlight = ¿Resaltar con qué nombre? { $n } nombres. Intro elige uno.
notes-palette-intro-change = ¿Cambiar el resaltado a qué nombre? { $n } nombres. Intro elige uno.
notes-palette-intro-collect = ¿Reunir los resaltados de qué nombre? { $n } nombres. Intro los escribe como lista.
notes-collect-none = No hay resaltados llamados { $name }.
# The collected list's own title (Markdown).
notes-collect-title = Resaltados llamados { $name }: { $title }
notes-collect-line = (línea { $line })
# Keep the letters y and n: they are the keys that answer.
notes-collect-saved =
    { $n ->
        [one] 1 resaltado llamado { $name } guardado como { $file }. ¿Abrirlo? y o n. En { $folder }.
       *[other] { $n } resaltados llamados { $name } guardados como { $file }. ¿Abrirlo? y o n. En { $folder }.
    }
palette-shape-underline = subrayado
palette-shape-double-underline = subrayado doble
palette-shape-bold = negrita
palette-shape-dotted = subrayado punteado
palette-shape-brackets = corchetes
palette-shape-symbol = símbolo
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

## Exportar el grafo de conocimiento (B1-g2).

graph-export-title = Exportar el grafo de conocimiento como
graph-export-intro = Grafo de conocimiento, enlaces: { $links }. Elija un formato; la lista Markdown es el texto para leer.
graph-export-empty = No hay enlaces entre notas para exportar. Añada uno desde la lista de enlaces de una nota.
graph-format-md = Lista Markdown, el texto para leer
graph-format-json = JSON, para Gephi y Cytoscape
graph-format-dot = DOT, para Graphviz
graph-format-graphml = GraphML, para Gephi, Cytoscape y yEd
graph-format-mermaid = Diagrama Mermaid
graph-format-plantuml = Diagrama PlantUML
graph-format-csv = Lista de aristas CSV, para hojas de cálculo
graph-export-saved = Grafo de conocimiento guardado como { $file }. ¿Abrirlo? y o n. En { $folder }.
graph-export-failed = No se pudo escribir el grafo de conocimiento: { $error } Compruebe que se puede escribir en la carpeta.

## Bookmarks: rename and delete.

notes-choose-bookmark-delete = Elija un marcador y pulse Suprimir.
notes-choose-bookmark-rename = Elija un marcador y pulse F2 para renombrarlo.
notes-bookmark-deleted = Marcador { $name } eliminado.
notes-renaming-bookmark = Renombrando el marcador { $name }.
notes-bookmark-unchanged = Marcador sin cambios.
# $name is the name asked for, $old the bookmark's name.
notes-bookmark-name-taken = Ya hay un marcador llamado { $name }. El marcador { $old } no cambió.
notes-bookmark-renamed = Se renombró el marcador { $old } a { $name }.

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
