### Sync between computers.

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
menu-cards = Tarjetas de estudio
menu-cw-practice = Práctica de código Morse
section-sync = Sincronización

## End of S4

## Sync wave, S5 (see locales/en).
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

## End of S5
