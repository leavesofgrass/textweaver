### The textweaver app: drawn labels, hints, buttons, and dialogs.

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
gui-button-edit = Editar
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
gui-button-start-dictation = Iniciar dictado
gui-button-pause-dictation = Pausar dictado
gui-button-resume-dictation = Reanudar dictado
gui-button-stop-dictation = Detener dictado
gui-button-live-pause = Pausar actualizaciones
gui-button-live-resume = Reanudar actualizaciones
gui-button-live-newest = Ir a la más reciente
gui-button-live-table = Vista de tabla
gui-button-live-list = Vista de lista
gui-button-live-save = Guardar registro
gui-button-live-close = Cerrar fuente
gui-button-live-show = Mostrar cambios
gui-button-live-show-on = Mostrar cambios: sí
gui-button-live-show-off = Mostrar cambios: no
gui-button-live-speak = Decir cambios
gui-button-live-speak-on = Decir cambios: sí
gui-button-live-speak-off = Decir cambios: no
gui-button-unavailable = no disponible
gui-hint-open = Elegir un documento para leer.
gui-hint-font = Elegir la fuente del texto.
gui-hint-edit = Cambiar entre leer y editar.
gui-hint-settings = Cada opción, con su ayuda.
gui-hint-commands = Ejecutar un comando por su nombre.
gui-hint-play = Leer desde la palabra actual, o pausar.
gui-hint-live-pause = Las entradas nuevas esperan en pausa.
gui-hint-live-view = Conserva su lugar en el registro.
gui-hint-live-save = Guardar el registro como JSON ahora.
gui-hint-live-close = Deja de recibir; el registro sigue.
gui-hint-live-field = Para el campo de la línea del cursor.
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
gui-focus-mode-on = Modo enfoque activado. Mover el ratón muestra las barras.
gui-focus-mode-off = Modo enfoque desactivado.
gui-full-screen-on = Pantalla completa activada.
gui-full-screen-off = Pantalla completa desactivada.

## Personalizar botones (crate::buttons).

buttons-title = Personalizar botones
buttons-intro =
    { $n ->
        [one] Personalizar botones, 1 botón. Enter en un botón para moverlo o quitarlo.
       *[other] Personalizar botones, { $n } botones. Enter en un botón para moverlo o quitarlo.
    }
buttons-row =
    { $bar ->
        [header] { $name }, cabecera { $pos } de { $count }
       *[toolbar] { $name }, barra de herramientas { $pos } de { $count }
    }
buttons-add-header = Añadir un botón a la cabecera
buttons-add-toolbar = Añadir un botón a la barra de herramientas
buttons-reset = Restablecer las dos barras
buttons-move-up = Subir
buttons-move-down = Bajar
buttons-remove = Quitar
buttons-add-intro =
    { $n ->
        [one] 1 comando para añadir. Enter lo añade al final.
       *[other] { $n } comandos para añadir. Enter añade uno al final.
    }
buttons-added =
    { $bar ->
        [header] { $name } añadido, cabecera { $pos } de { $count }.
       *[toolbar] { $name } añadido, barra de herramientas { $pos } de { $count }.
    }
buttons-removed =
    { $bar ->
        [header] { $name } quitado de la cabecera.
       *[toolbar] { $name } quitado de la barra de herramientas.
    }
buttons-moved = { $name } movido, { $pos } de { $count }.
buttons-first = { $name } ya es el primero.
buttons-last = { $name } ya es el último.
buttons-already =
    { $bar ->
        [header] { $name } ya está en la cabecera.
       *[toolbar] { $name } ya está en la barra de herramientas.
    }
buttons-no-name = Ese comando no tiene nombre corto, así que no puede ser un botón.
buttons-no-button = No hay ningún botón ahí.
buttons-reset-done = Botones restablecidos a los valores predeterminados.
buttons-none-to-add = No quedan comandos para añadir.
# The preview pane beside the editor (edit mode). $key is the key that moves between the window's parts (F6).
gui-preview = Vista previa
gui-preview-shown = Vista previa mostrada junto al editor. { $key } va a ella.
gui-preview-hidden = Vista previa oculta.
gui-preview-later = Vista previa activada. Aparece junto al editor en el modo de edición.
gui-preview-empty = Todavía no hay nada que previsualizar.
gui-sidebar-no-headings = No hay encabezados.
gui-sidebar-no-notes = No hay notas.
gui-sidebar-current = { $item }, actual
gui-sidebar-hint = Intro va allí. { $leave } va y vuelve. Escape vuelve.
gui-sidebar-hint-notes = Intro va allí. Espacio muestra sus enlaces. { $leave } va y vuelve. Escape vuelve.
gui-sidebar-notes-keys = Espacio muestra los enlaces de una nota.
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

## The window's menus and dialogs. Settings files chosen with the
## system's file chooser, the Colors dialog, and the font list. $ratio is
## a contrast ratio such as 4.8; $verdict is good, fair, or low.
gui-settings-files = Archivos de configuración
gui-settings-export-title = Exportar la configuración
gui-settings-import-title = Importar la configuración
gui-chooser-no-dialog = El selector de archivos del sistema no se abrió. Escriba la ruta del archivo.
gui-colors-value = { $value }, contraste { $ratio } a 1, { $verdict }
gui-colors-help = Izquierda y Derecha eligen un color con nombre, primero azul y naranja. Intro escribe un nombre o un valor #rrggbb. Suprimir recupera el color del tema. Cada marca conserva su subrayado, su grosor o su símbolo, sea cual sea su color.
gui-colors-value-system = { $value }; no se dibuja mientras los colores de alto contraste del sistema están activos
gui-colors-help-system = Los colores de alto contraste del sistema están activos y se dibujan en lugar de estos, que vuelven a aplicarse cuando se desactivan. Cada nombre de resaltado conserva su propia forma.
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

## W9b-x: las opciones de lectura (Ver, Opciones de lectura).
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
