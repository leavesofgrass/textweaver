### The help, key names, About, and the packaged guides.

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
help-intro = Ayuda. Escriba para buscar en toda la ayuda; Arriba y Abajo se mueven, Escape cierra.

## The help list. Each value is a key or keys from the keymap.

help-about = textweaver lee documentos en voz alta. Las teclas de abajo son los atajos actuales.
help-search = Busque en toda la ayuda escribiendo aquí: comandos, teclas, ajustes y las guías.
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
help-highlights = Resaltar la selección o la oración, o quitar un resaltado: { $highlight }. Con un nombre: { $first } a { $fifth }, o la lista: { $named }. Listar resaltados: { $list }.
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

## Help's ways to the docs, About's facts, and first-run choices asked
## again. $address is a web address; $path a folder; facts start with
## their name so each Braille line leads with it.
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

## B1-hp: las guías incluidas y Buscar en la ayuda.
docs-not-found = No se encontró la documentación junto a textweaver. ¿Abrir { $address } en el navegador? y o n
docs-read-only = Esta guía viene con textweaver y es de solo lectura. Para cambiarla, copie el archivo a otro lugar y abra la copia.
docs-guide-missing =
    { $n ->
        [one] Falta una guía: { $guides }. Las demás guías se abren.
       *[other] Faltan { $n } guías: { $guides }. Las demás guías se abren.
    }
helpsearch-title = Buscar en la ayuda
helpsearch-title-matching = Ayuda sobre { $filter }
helpsearch-intro = Buscar en la ayuda, { $n } temas. Escriba para buscar comandos, teclas, ajustes y las guías; Enter abre un tema, F1 lo explica.
helpsearch-match =
    { $n ->
        [one] 1 resultado.
       *[other] { $n } resultados.
    }
helpsearch-none = No hay ayuda sobre { $query }. Retroceso borra letras.
helpsearch-cleared =
    { $n ->
        [one] Filtro borrado, 1 tema.
       *[other] Filtro borrado, { $n } temas.
    }
helpsearch-command = { $row }, comando
helpsearch-setting = { $label }, ajuste en { $section }
helpsearch-guide = { $heading }, en { $guide }
helpsearch-at-heading = En { $heading }.
## End of B1-hp
