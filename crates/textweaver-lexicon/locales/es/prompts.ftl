### Prompts, questions, lists, and file choosers.

## Prompts.

prompt-define-word = ¿Definir qué palabra?
prompt-profile-name = Nombre del perfil nuevo
prompt-profile-rename = Nuevo nombre del perfil, Intro lo conserva
prompt-profiles-import = Importar perfiles desde un archivo
prompt-profiles-export = Exportar perfiles a un archivo, por ejemplo textweaver-profiles.json

## Questions and answers. Keep the letters y and n: they are the keys
## that answer.

common-press-y-or-n = Pulse y o n.
common-kept = Se conservó.
confirm-quit = ¿Salir de textweaver? y o n
confirm-delete-note = ¿Eliminar esta nota o resaltado? y o n
notes-remove-highlight-question = ¿Quitar este resaltado? y o n
notes-delete-note-question = ¿Eliminar esta nota? y o n
list-nothing-to-mark = Nada que marcar en esta lista.
notes-editing = Editando la nota: { $text }
# $key opens a document.
app-no-document-open = No hay ningún documento abierto. Pulse { $key } para abrir uno.
app-window-only = Esta orden funciona en la aplicación textweaver.
app-terminal-only = Esta orden funciona en el lector del terminal.
settings-save-failed = No se pudo guardar la configuración: { $error } Sus cambios siguen en uso hasta que salga.
settings-outside-kept = Se conservó la configuración cambiada fuera de textweaver.
edit-still-editing = Aún editando.
# The reading view inside edit mode (toggle_preview). $key is its key;
# $line the line the caret is back on.
edit-preview-on = Vista previa, solo lectura: la vista de lectura en el mismo lugar. { $key } vuelve a la edición.
edit-preview-off = De vuelta a la edición. { $line }
edit-preview-read-only = La vista previa es de solo lectura. { $key } vuelve a la edición.
edit-preview-not-editing = La vista previa funciona en modo de edición. { $key } empieza a editar.
edit-preview-plain = El texto plano no tiene formato que previsualizar; se lee igual.
edit-preview-failed = No se pudo mostrar la vista previa: { $error } Revise el Markdown e inténtelo de nuevo.
goto-not-a-target = No es un destino válido: { $text }. Escriba un número de línea, un porcentaje como 50%, inicio o fin.

## Prompts: the label is shown and said when the prompt opens.

prompt-find = Buscar
prompt-go-to = Ir a línea, porcentaje, inicio o fin
prompt-open = Abrir archivo
prompt-command = Comando
# $label is prompt-command.
prompt-command-palette-intro = { $label }. Escriba parte de un nombre; Tab completa, Arriba y Abajo recorren las coincidencias.
prompt-save-as = Guardar como
prompt-export-as = Exportar como
prompt-table-size = Tamaño de la tabla, columnas por filas, por ejemplo 3 por 2
prompt-image-path = Archivo de imagen
prompt-replace-find = Reemplazar, buscar qué
prompt-replace-with = Reemplazar con
prompt-note = Nota
prompt-edit-note = Editar nota, Intro la conserva
prompt-rename-bookmark = Nuevo nombre del marcador, Intro lo conserva
prompt-export-settings = Exportar la configuración a un archivo, por ejemplo textweaver-settings.json
prompt-import-settings = Importar configuración desde un archivo
prompt-citation-locator = Página u otro localizador, por ejemplo 12 o capítulo 2; Intro para ninguno
prompt-reference-identifier = DOI o ISBN a agregar
prompt-import-references = Importar referencias desde un archivo
prompt-template-title = Título del documento nuevo
prompt-setting-value = Valor nuevo, Intro lo conserva

## Tab completion of file paths in prompts.

# $folder is the folder's full path.
pathc-no-folder = No hay ninguna carpeta { $folder }.
# $prefix is what was typed after the last separator.
pathc-no-match = Ningún archivo o carpeta empieza por { $prefix }.
# The one name that matched; $kind is folder or file.
pathc-one =
    { $kind ->
        [folder] { $name }, carpeta
       *[file] { $name }, archivo
    }
# $n names matched; $names are the first few, joined with commas; $more is
# yes when more matched than are read out.
pathc-many =
    { $more ->
        [yes] { $n } coincidencias: { $names }, y más.
       *[no] { $n } coincidencias: { $names }.
    }

## Lists and prompts shared by every frontend.

# The focused list item: $item is its text, $k its place, $n the number of items.
listmodel-item-position = { $k } de { $n }, { $item }
# $letter is the letter or digit typed.
listmodel-no-item-starts = Ningún elemento empieza por { $letter }.
listmodel-top-of-list = Principio de la lista.
listmodel-end-of-list = Final de la lista.
listmodel-no-matching-commands = No hay comandos coincidentes.
listmodel-no-earlier-entries = No hay entradas anteriores.
# Tab in the command palette: $n commands match (always more than one); $names lists the first few, joined by commas.
listmodel-command-matches = { $n } coincidencias: { $names }.
goto-word-start = inicio
goto-word-end = fin

language-voices-loading = La lista de voces todavía se está cargando, así que sigue hablando la voz actual.

## Selectores de archivos y carpetas.
prompt-browse-hint = { $label }. { $key } para explorar.
prompt-browse-file = Elija el archivo: { $label }
prompt-browse-folder = Elija la carpeta: { $label }
prompt-browse-filled = { $name } elegido. Intro confirma.
chooser-type-files = Archivos { $type }
chooser-image-title = Insertar una imagen
chooser-images = Imágenes
chooser-references-title = Importar referencias
chooser-reference-files = Archivos de referencias
chooser-anki-title = Importar tarjetas de Anki
chooser-anki-files = Paquetes de Anki
chooser-profiles-import-title = Importar perfiles
chooser-profiles-export-title = Exportar perfiles
chooser-profile-files = Archivos de perfiles
chooser-live-source-files = Archivos de fuentes en vivo
gui-folder-no-dialog = El selector de carpetas del sistema no se abrió. Elija la carpeta en esta lista.
gui-prompt-browse-hint = { $key } abre el explorador de archivos.

## A secret field's Show choice: the show key, or the Show box in
## the textweaver app, shows a password, token or join code as typed.
prompt-show-hint = { $label }. { $key } lo muestra.
prompt-secret-hidden = { $count ->
        [one] 1 carácter, oculto
       *[other] { $count } caracteres, ocultos
    }
prompt-show-join-code = Mostrar el código de acceso
prompt-showing-join-code = Código de acceso visible
prompt-hiding-join-code = Código de acceso oculto
prompt-show-password = Mostrar la contraseña
prompt-showing-password = Contraseña visible
prompt-hiding-password = Contraseña oculta
prompt-show-token = Mostrar el token
prompt-showing-token = Token visible
prompt-hiding-token = Token oculto
gui-prompt-show-hint = { $key } o la casilla Mostrar enseña lo escrito.
