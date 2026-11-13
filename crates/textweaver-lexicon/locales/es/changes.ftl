### Tracked changes and comments in Word files.

## B1-t1: cambios con seguimiento y comentarios (la lista de cambios).
prompt-comment-reply = Respuesta
prompt-comment-text = Comentario
prompt-github-token = Token de GitHub del origen de componentes, Intro lo omite
prompt-import-cards = Importar tarjetas de estudio de un paquete de Anki
prompt-cw-copy = Escriba lo que oyó
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

## B1-t2: guardar la revisión en el archivo de Word.
changes-accept-all-question =
    { $n ->
        [one] ¿Aceptar el cambio? y o n
       *[other] ¿Aceptar los { $n } cambios? y o n
    }
changes-reject-all-question =
    { $n ->
        [one] ¿Rechazar el cambio? y o n
       *[other] ¿Rechazar los { $n } cambios? y o n
    }
changes-save-not-word = No es un archivo de Word. Los cambios solo se guardan de vuelta en archivos .docx; la exportación escribe el texto decidido en otros formatos.
changes-save-nothing = Nada que guardar: no se aceptó ni rechazó ningún cambio y ningún comentario cambió.
changes-saved-backup = Cambios guardados en { $file }. El original se conserva como { $backup }.
changes-saved = Cambios guardados en { $file }.
changes-save-unplaced =
    { $n ->
        [one] 1 comentario nuevo está al principio: no se encontró su texto.
       *[other] { $n } comentarios nuevos están al principio: no se encontró su texto.
    }
changes-save-failed = No se pudieron guardar los cambios en { $file }: { $error }. El archivo sigue igual; ciérrelo en Word si está abierto y vuelva a intentarlo.
changes-save-not-in-build = Guardar los cambios en el archivo de Word no está en esta versión de { -brand }. tw changes --in-place sigue haciéndolo.
changes-in-place-accepted =
    { $n ->
        [one] 1 cambio aceptado en { $path }. El original se conserva como { $backup }.
       *[other] { $n } cambios aceptados en { $path }. El original se conserva como { $backup }.
    }
changes-in-place-rejected =
    { $n ->
        [one] 1 cambio rechazado en { $path }. El original se conserva como { $backup }.
       *[other] { $n } cambios rechazados en { $path }. El original se conserva como { $backup }.
    }

## B1-t3: edits saved into a Word file as tracked changes.
changes-tracked-saved =
    { $n ->
        [one] 1 cambio controlado guardado en { $file }.
       *[other] { $n } cambios controlados guardados en { $file }.
    }
changes-original-kept = El original se conserva como { $backup }.
changes-tracked-refused =
    { $n ->
        [one] No se guardó: 1 cambio en { $file } cruza un párrafo o está en un enlace o campo, y no se puede controlar. Guardar como conserva sus ediciones en Markdown.
       *[other] No se guardó: { $n } cambios en { $file } cruzan un párrafo o están en un enlace o campo, y no se pueden controlar. Guardar como conserva sus ediciones en Markdown.
    }
## End of B1-t2
