### Menus, the command palette, and interface announcements.

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
menu-highlights = Resaltados
menu-tables = Tablas
menu-speech-cursor = Cursor de voz

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
color-name-theme = color del tema
color-name-blue = azul
color-name-orange = naranja
color-name-navy = azul oscuro
color-name-skyblue = azul cielo
color-name-teal = verde azulado
color-name-gold = dorado
color-name-yellow = amarillo
color-name-green = verde
color-name-cyan = cian
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
colors-palette-label = Nombre de resaltado { $name }
colors-palette-value = { $color }, { $shape }, contraste { $text } a 1 con el texto y { $page } a 1 con la página, { $verdict }
colors-palette-system = { $color }, { $shape }; se dibujan los colores de alto contraste del sistema, y la forma sigue marcándolo
colors-palette-unknown = { $color }, { $shape }; un color que textweaver no puede medir
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
