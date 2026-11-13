### The terminal reader.

## The terminal reader's startup.

# $wanted is the speech backend asked for, $backend the one used instead.
tui-setup-backend-unavailable = El motor de voz { $wanted } no está disponible; se usa { $backend }.
tui-setup-speech-failed = No se pudo iniciar la voz ({ $error }); ejecutando en silencio.
speech-engine-fallback = Hablando con { $engine }: { $failed } no pudo iniciarse.
speech-engine-fallback-silent = Sin voz: { $failed } no pudo iniciarse y no hay otro motor disponible.
tui-setup-cannot-save = No se puede guardar la configuración ni las posiciones: { $error } La lectura funciona. Los cambios se pierden al salir.
tui-setup-keymap-ignored = Se ignoró el archivo de teclas: { $error } Se usan las teclas predeterminadas. Corrija el archivo y reinicie.
# The first-run welcome. Each value names the key for an action: $play
# reads and pauses, $stop stops, $heading moves to the next heading,
# $help opens the help, $quit quits.
tui-setup-welcome = Le damos la bienvenida a { -brand }. { $open } abre un documento, { $help } la ayuda. { $play } reproduce y pausa, { $stop } detiene, { $palette } lista todas las órdenes.
# The window's first-run welcome on Windows, where the menu bar is
# hidden until Alt or $menu (F10) shows it: the keys of
# tui-setup-welcome, and the menus.
gui-setup-welcome-menus = Le damos la bienvenida a { -brand }. { $open } abre un documento, { $help } la ayuda. { $play } reproduce y pausa, { $stop } detiene, { $palette } lista todas las órdenes. Alt o { $menu } muestra los menús.
# Said at startup without a document. $open, $new, and $help name the
# keys for Open, New Document, and Help.
tui-setup-no-document = No hay ningún documento abierto. Pulse { $open } para abrir uno, { $new } para uno nuevo, o { $help } para ayuda.

## The terminal reader's launch.

# $name is the file asked for on the command line; $error says why.
tui-could-not-open = No se pudo abrir { $name }: { $error }

## Copying in the terminal reader.

tui-clip-system = Se copió con el portapapeles del sistema, porque este terminal no puede recibir texto copiado.
# $error is why: tui-clip-not-available, tui-clip-not-built, or the
# system's own message.
tui-clip-failed = No se pudo copiar con el portapapeles del sistema: { $error }. Se envió al terminal en su lugar.
tui-clip-not-available = el portapapeles del sistema no está disponible
tui-clip-not-built = esta compilación no tiene portapapeles del sistema

## The terminal reader's screen.

# The title line's start; $title is the document's title or
# tui-title-no-document.
tui-title = { -brand }: { $title }
tui-title-no-document = sin documento
# The screen without a document. $keys names the keys for the action.
tui-empty-open = Abrir uno: { $keys }.
tui-empty-help = Ayuda: { $keys }.
tui-empty-quit = Salir: { $keys }.
# The key hints while a yes-or-no question waits. y, n, and a are the
# answer keys the reader takes; $escape names the Escape key.
tui-hints-confirm = y sí  n o a no  { $escape } no
# Key hint labels, each shown after its key on the bottom line.
tui-hint-play = reproducir
tui-hint-sentence = oración
tui-hint-faster = más rápido
tui-hint-slower = más lento
tui-hint-close-rsvp = cerrar RSVP
tui-hint-quit = salir
tui-hint-save = guardar
tui-hint-finish = terminar
tui-hint-undo = deshacer
tui-hint-bold = negrita
tui-hint-heading = encabezado
tui-hint-commands = comandos
tui-hint-next-line = línea siguiente
tui-hint-previous-line = línea anterior
tui-hint-again = otra vez
tui-hint-read-on = seguir leyendo
tui-hint-leave = salir
tui-hint-find = buscar
tui-hint-mark = marcar
tui-hint-lines = líneas
tui-hint-keys = teclas
tui-hint-choose = elegir
tui-hint-close = cerrar
tui-hint-back = atrás
# The list overlay's border: $n is the focused item's number, $count
# the number of items.
tui-list-title = { $n } de { $count }, { $title }
