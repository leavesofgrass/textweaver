### The study sheet, the self-test, and study cards.

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
# $name is the highlight's palette name.
notes-sheet-highlighted = Resaltado, { $name }.
# On the study sheet grouped by name: the heading a highlight falls under.
notes-sheet-under = Bajo: { $heading }
# The study sheet's section of notes, after the names.
notes-sheet-notes = Notas

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

## Tarjetas de estudio y la sesión de estudio (crate::cards).

cards-blank = espacio
cards-recall-question = ¿Qué dice «{ $heading }»?
cards-no-document = Abra un documento para crear o estudiar tarjetas.
cards-nothing-to-make = No hay notas ni resaltados para crear tarjetas. Añada primero una nota o un resaltado.
cards-made =
    { $added ->
        [0] Ninguna tarjeta nueva. { $total } tarjetas en total.
        [one] Tarjetas creadas: 1 nueva, { $total } en total.
       *[other] Tarjetas creadas: { $added } nuevas, { $total } en total.
    }
cards-none-from-item = Ninguna tarjeta de este elemento: necesita texto sobre un pasaje.
cards-made-one = Tarjeta creada: { $question }
cards-updated-one = Tarjeta actualizada: { $question }
cards-none = Aún no hay tarjetas. Para crearlas: { $key }.
cards-study-title = Estudiar tarjetas: { $title }
cards-study-intro =
    { $n ->
        [one] Estudiar tarjetas, 1 tarjeta. Intro muestra la respuesta, de 1 a 4 la califican. Espacio para responder en voz alta.
       *[other] Estudiar tarjetas, { $n } tarjetas. Intro muestra cada respuesta, de 1 a 4 la califican. Espacio para responder en voz alta.
    }
cards-no-session = No hay sesión de estudio. Para empezar una: { $key }.
cards-card-gone = Esa tarjeta se quitó.
cards-grade-again = Otra vez
cards-grade-hard = Difícil
cards-grade-good = Bien
cards-grade-easy = Fácil
cards-graded = { $grade }. Tarjeta { $i } de { $n }. Pregunta: { $question }
# Said when a grade from the palette opens the session again on the next card.
cards-graded-reopen = { $grade }. Estudiar tarjetas, tarjeta { $i } de { $n }.
cards-session-done =
    { $n ->
        [one] { $grade }. Terminado: la tarjeta está calificada.
       *[other] { $grade }. Terminado: las { $n } tarjetas están calificadas.
    }
cards-reversed = Invertida. Pregunta: { $question }
cards-unreversed = Como se creó. Pregunta: { $question }
cards-not-reversible = Solo las tarjetas de pregunta se pueden invertir.
cards-kind-cloze = Completar el espacio
cards-kind-question = Pregunta
cards-kind-recall = Recordar
cards-not-graded = sin calificar aún
cards-last-grade = última calificación: { $grade }
cards-item = { $kind }: { $question }, { $grade }
cards-list-title = Tarjetas
cards-list-intro =
    { $n ->
        [one] Tarjetas, 1 elemento. Intro va a su origen, Supr la quita.
       *[other] Tarjetas, { $n } elementos. Intro va al origen de una tarjeta, Supr la quita.
    }
cards-source-label = Origen de la tarjeta
cards-remove-question = ¿Quitar esta tarjeta y sus calificaciones? y o n
cards-removed = Tarjeta quitada.
cards-save-failed = No se pudieron guardar las tarjetas: { $error } Compruebe que se puede escribir en la carpeta de datos.
cards-anki-exported =
    { $n ->
        [one] Se exportó 1 tarjeta a Anki: { $file }, en { $folder }.
       *[other] Se exportaron { $n } tarjetas a Anki: { $file }, en { $folder }.
    }
cards-anki-export-failed = No se pudieron exportar las tarjetas a Anki: { $error } Compruebe que se puede escribir en la carpeta del documento.
cards-anki-import-failed = No se pudo importar { $file }: { $error } Compruebe que el archivo es un paquete de Anki, .apkg.
cards-anki-imported =
    { $added ->
        [one] Se importó 1 tarjeta.
       *[other] Se importaron { $added } tarjetas.
    }
cards-anki-already =
    { $n ->
        [one] 1 tarjeta ya estaba; se combinaron sus calificaciones.
       *[other] { $n } tarjetas ya estaban; se combinaron sus calificaciones.
    }
cards-anki-skipped-cloze =
    { $n ->
        [one] Se omitió 1 nota de huecos.
       *[other] Se omitieron { $n } notas de huecos.
    }
cards-anki-skipped-media =
    { $n ->
        [one] Se omitió 1 nota que solo tiene imágenes o sonido.
       *[other] Se omitieron { $n } notas que solo tienen imágenes o sonido.
    }
cards-anki-skipped-other =
    { $n ->
        [one] Se omitió 1 nota sin texto o con más de dos tarjetas.
       *[other] Se omitieron { $n } notas sin texto o con más de dos tarjetas.
    }
cards-from-anki = Esta tarjeta viene de Anki; no tiene un lugar en el documento.
# Said first in the study session and the Cards list (B1-f2). $due is how
# many graded cards are due, $new how many were never graded.
cards-due-summary =
    { $due ->
        [one] Para hoy: 1 tarjeta, { $new } nuevas.
       *[other] Para hoy: { $due } tarjetas, { $new } nuevas.
    }
# When a card is next due, after a grade. $days is whole days, at least 1.
cards-next-in =
    { $days ->
        [one] de nuevo mañana
       *[other] de nuevo en { $days } días
    }
cards-due-now = para hoy
# A grade or last grade, then when the card is next due: "Good, next in 3 days".
cards-grade-next = { $grade }, { $next }
# Study cards when no card is due and none is new: every card is asked.
cards-nothing-due =
    { $days ->
        [one] Nada para hoy; la próxima tarjeta toca mañana. Se estudian todas por adelantado.
       *[other] Nada para hoy; la próxima tarjeta toca en { $days } días. Se estudian todas por adelantado.
    }
