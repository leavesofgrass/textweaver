### Edit mode: typing, formatting, saving, recovery, and templates.

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
# Edit mode's Obsidian aids: a wiki link, tasks, callouts.
edit-wikilink-title = Enlazar a una nota
edit-wikilink-intro =
    { $n ->
        [one] Enlazar a una nota, 1 nota. Intro la elige.
       *[other] Enlazar a una nota, { $n } notas. Intro elige una.
    }
edit-wikilink-no-notes = No hay notas en esta bóveda a las que enlazar.
edit-wikilink-anchor-title = Enlazar a una parte de { $note }
edit-wikilink-anchor-intro = Enlazar a toda la nota { $note }, o a un encabezado o bloque de ella. Intro elige.
edit-wikilink-whole = La nota completa
edit-wikilink-heading = Encabezado { $level }: { $text }
edit-wikilink-block = Bloque { $id }: { $text }
edit-wikilink-inserted = Se insertó un enlace a { $target }.
edit-callout-title = Tipo de aviso
edit-callout-intro = Tipos de aviso, { $n }. Intro inserta uno.
edit-callout-inserted = Se insertó un aviso de tipo { $type }. Escriba su texto.
edit-task-status =
    { $status ->
        [done] Marcada.
        [open] Sin marcar.
        [in-progress] En curso.
        [cancelled] Cancelada.
        [forwarded] Reenviada.
        [important] Importante.
        [question] Pregunta.
       *[other] Marcada con { $mark }.
    }
edit-task-new = Nueva tarea, sin marcar.
edit-tasks-changed =
    { $n ->
        [one] Tarea cambiada en 1 línea.
       *[other] Tareas cambiadas en { $n } líneas.
    }
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
