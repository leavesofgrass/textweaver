### Opening documents, the file browser, and batch conversion.

## Opening a document.

open-opened = Se abrió { $title }.
open-resumed = Se abrió { $title }. Se reanudó en el { $pct } por ciento.
open-resumed-synced = Se abrió { $title }. Se reanudó en el { $pct } por ciento, desde otro dispositivo.

## Opening a document: failures and opening in the background.

# $name is the file's name.
opening-is-folder = { $name } es una carpeta, no un documento. Indique el nombre de un archivo dentro de ella.
# Said after "Could not open NAME:", so it starts in lower case.
opening-no-file-in = no hay ningún archivo llamado { $name } en { $folder }. Compruebe el nombre.
opening-no-file-here = no hay ningún archivo llamado { $name } aquí. Compruebe el nombre.
opening-no-permission = no tiene permiso para leerlo.
opening-damaged-rtf = no es un archivo RTF legible; puede estar dañado.
opening-damaged-odt = no es un archivo de texto OpenDocument legible; puede estar dañado.
opening-damaged-latex = no es un archivo LaTeX legible; puede estar dañado o ser demasiado grande.
opening-damaged-email = no es un mensaje de correo legible; puede estar dañado o ser demasiado grande.
opening-damaged-mhtml = no es una página web archivada legible; puede estar dañada o ser demasiado grande.
opening-pdf-password = está protegido con contraseña; quite la contraseña antes con un programa de PDF.
opening-old-office = es un formato antiguo de Office; guárdelo como .docx y ábralo.
opening-rar = es un archivo RAR, que no se abre. Extráigalo primero, o use ZIP o 7z.
# $reason is one of the opening-no-* messages, or the loader's own words.
opening-failed = No se pudo abrir { $name }: { $reason }
opening-started = Abriendo { $name }. Escape cancela.
opening-stopped = Se detuvo la apertura de { $name }.
opening-still = Todavía abriendo { $name }, { $secs } segundos.
# $step is the loader's report, such as "recognizing text on page 3 (3 of 40)."
opening-still-step = Todavía abriendo { $name }: { $step }
opening-stopped-unexpectedly = No se pudo abrir { $name }: la carga se detuvo de forma inesperada.

## A build without the publish feature. "tw convert" is a command typed
## at the terminal: keep it as it is.

lean-citations-not-in-build = Las citas no están en esta versión de { -brand }.
lean-publish-not-in-build = Exportar y ver la vista previa no están en esta versión de { -brand }. tw convert sigue convirtiendo.

## Background work, and opening files and addresses.

# $what names the export, as its own message says it.
tasks-stopped = { $what } se detuvo de forma inesperada.
# $input is the DOI, ISBN, or other identifier being looked up.
tasks-lookup-stopped = La búsqueda de { $input } se detuvo de forma inesperada.
# The reason in tasks-could-not-open when a session keeps no files.
tasks-launch-off = abrir otros programas está desactivado en una sesión que no guarda archivos
tasks-opening = Abriendo.
# $target is a file or a web address; $error says why.
tasks-could-not-open = No se pudo abrir { $target }: { $error }
open-refused-scheme = No abierto: enlace { $scheme } bloqueado.
open-refused-missing = No abierto: archivo no encontrado.
open-refused-invalid = No abierto: dirección no válida.
open-refused-network = No abierto: el archivo está en otro equipo.
tasks-not-opened = No se abrió.
# Keep the letters y and n: they are the keys that answer.
tasks-open-it-question = ¿Abrirlo? y o n

## Opening the new formats. Said after "Could not open NAME:", so
## each starts in lower case.
opening-damaged-json = no es un archivo JSON legible; puede ser demasiado grande.
opening-damaged-notebook = no es un cuaderno de Jupyter legible; puede estar dañado o ser demasiado grande.
opening-damaged-canvas = no es un lienzo legible; puede estar dañado o ser demasiado grande.
opening-damaged-svg = no es un dibujo SVG legible; puede estar dañado o ser demasiado grande.
opening-damaged-mathml = no es una fórmula MathML legible; puede estar dañada o ser demasiado grande.

## The file browser. Every row and introduction starts with the name,
## then the kind, so the first cells of a 40-cell Braille line hold what
## matters. $name is a file or folder name; $n a number that chooses the
## plural and $count the same number written with its separators.
# A list item with its position after it, in the file browser.
listmodel-item-position-last = { $item }, { $k } de { $n }
browse-places-title = Lugares
browse-places-intro =
    { $n ->
        [one] Lugares, 1 lugar.
       *[other] Lugares, { $n } lugares.
    }
# $purpose says what the folder or file is chosen for; $intro follows.
browse-choosing = { $purpose }. { $intro }
browse-place-document = { $name }, la carpeta del documento
browse-place-start = { $name }, carpeta de inicio
browse-place-library = { $name }, carpeta de la biblioteca
browse-place-disk = { $name }, disco
browse-place-removable = { $name }, unidad extraíble
browse-place-network = { $name }, unidad de red
browse-place-cd = { $name }, unidad de CD o DVD
browse-place-root = { $name }, la carpeta raíz
browse-choose-here = Elegir esta carpeta, { $name }
browse-row-folder = { $name }, carpeta
browse-row-folder-items =
    { $name }, carpeta, { $n ->
        [one] 1 elemento
       *[other] { $count } elementos
    }
# $kind is a kind below ("Markdown"); $size a size below ("12 KB").
browse-row-file = { $name }, { $kind }, { $size }
browse-row-kind = { $name }, { $kind }
browse-row-hidden = { $row }, oculto
# $kind is zip, tar, tar.gz, gzip, or 7z.
browse-kind-archive = archivo comprimido { $kind }
browse-kind-file = archivo
browse-kind-markdown = Markdown
browse-kind-text = texto
browse-kind-html = página web
browse-kind-epub = libro EPUB
browse-kind-docx = documento de Word
browse-kind-rtf = documento RTF
browse-kind-odt = texto OpenDocument
browse-kind-latex = LaTeX
browse-kind-eml = correo electrónico
browse-kind-mhtml = archivo web
browse-kind-pdf = PDF
browse-kind-image = imagen
browse-kind-daisy = libro DAISY
browse-kind-pptx = diapositivas de PowerPoint
browse-kind-sheet = hoja de cálculo
browse-kind-json = JSON
browse-kind-notebook = cuaderno de Jupyter
browse-kind-svg = dibujo SVG
browse-kind-mathml = fórmula MathML
browse-kind-pandoc = documento leído con Pandoc
browse-size-bytes =
    { $n ->
        [one] 1 byte
       *[other] { $count } bytes
    }
# $size is a number, with a decimal under 10 ("3.4").
browse-size-kb = { $size } KB
browse-size-mb = { $size } MB
browse-size-gb = { $size } GB
browse-intro =
    { $name }, { $n ->
        [one] 1 elemento.
       *[other] { $count } elementos.
    }
browse-intro-empty = { $name } no tiene nada que mostrar.
# $filter is what was typed.
browse-intro-filtered =
    { $name }, { $n ->
        [one] 1 elemento coincide con
       *[other] { $count } elementos coinciden con
    } { $filter }.
browse-intro-in-archive = { $intro } En { $archive }.
browse-intro-hidden =
    { $intro } { $n ->
        [one] 1 archivo oculto.
       *[other] { $count } archivos ocultos.
    }
browse-intro-cut = { $intro } Solo se muestran los primeros { $max }.
# $preview, $choose, $sort, and $all are keys; $item the focused row.
browse-keys = { $intro } Intro abre, Retroceso sube, escribir filtra. { $preview } muestra un avance, { $choose } elige una carpeta, { $sort } ordena, { $all } muestra todos los archivos. { $item }
browse-sorted-name = Ordenado por nombre.
browse-sorted-date = Ordenado por fecha, el más reciente primero.
browse-sorted-size = Ordenado por tamaño, el más grande primero.
browse-showing-all = Se muestran todos los archivos.
browse-showing-readable = Solo se muestran los archivos legibles.
browse-closed = Explorador de archivos cerrado.
browse-read-only = El explorador de archivos solo abre y elige archivos; nunca los cambia.
# $key is the Choose Folder key.
browse-choose-a-folder = Elija una carpeta: Intro abre una, { $key } la elige.
browse-choose-a-file = Elija un archivo: Intro elige uno.
browse-no-archive-folder = No se puede elegir una carpeta dentro de un archivo comprimido; elija una carpeta del disco.
browse-nothing-waiting = Ningún comando espera una carpeta; Intro la abre.
browse-cannot-read = { $name } no es un tipo de archivo que textweaver pueda leer.
browse-folder-unreadable = No se pudo abrir { $name }: { $reason }
browse-archive-too-deep = { $name } está dentro de demasiados archivos comprimidos para abrirlo.
browse-archive-too-large = { $name } es demasiado grande para listarlo con seguridad.
browse-archive-unreadable = { $name } no es un archivo comprimido que textweaver pueda leer; puede estar dañado.
# A document's preview: its title, then its first sentence.
browse-preview-document = { $title }. { $sentence }
browse-preview-no-text = { $title }. No tiene texto.
browse-preview-failed = No se pudo mostrar un avance de { $name }: { $reason }
# $names are the first few names inside.
browse-preview-archive =
    { $name }: { $n ->
        [one] 1 archivo
       *[other] { $count } archivos
    }, { $readable } legibles. { $names }
browse-preview-archive-folder =
    { $name }, carpeta del archivo comprimido, { $n ->
        [one] 1 elemento.
       *[other] { $count } elementos.
    }
browse-preview-folder = { $path }: { $names }
browse-preview-folder-empty = { $path }: nada que leer aquí.
browse-preview-other = { $name }, { $size }; textweaver no puede leer este tipo de archivo.
browse-preview-path = { $path }

## Batch conversion (File, Batch convert). Keep the meaning first.
batch-choose-source = Elija la carpeta que desea convertir
batch-choose-output = Elija la carpeta para los archivos convertidos
batch-format-title = Convertir a
batch-format-intro = Convertir { $name } a: elija un formato, { $n } opciones.
batch-where-title = Dónde van los archivos
batch-where-intro = ¿Dónde deben ir los archivos convertidos?
batch-where-converted = En una carpeta converted, { $path }
batch-where-beside = Junto a cada archivo
batch-where-choose = En otra carpeta, que elegirá a continuación
batch-nothing = No hay documentos que convertir en { $path }.
batch-confirm =
    ¿Convertir { $n ->
        [one] 1 archivo
       *[other] { $n } archivos
    } a { $format } en { $path }? y o n
batch-confirm-beside =
    ¿Convertir { $n ->
        [one] 1 archivo
       *[other] { $n } archivos
    } a { $format } junto a cada archivo? y o n
batch-started =
    Convirtiendo { $n ->
        [one] 1 archivo
       *[other] { $n } archivos
    } a { $format }. Escape detiene.
batch-progress = { $percent } por ciento convertido, { $done } de { $total } archivos.
batch-busy = Ya se está convirtiendo, { $done } de { $total } archivos. Escape detiene.
batch-stop-question = ¿Detener la conversión? Los archivos ya hechos se conservan. y o n
batch-stopping = Se detendrá tras los archivos que se están escribiendo.
batch-still-converting = La conversión continúa.
batch-done =
    { $converted ->
        [one] 1 archivo convertido
       *[other] { $converted } archivos convertidos
    } a { $format }; { $skipped } al día; { $failed } con error.
batch-stopped =
    Detenido. { $converted ->
        [one] 1 archivo convertido
       *[other] { $converted } archivos convertidos
    }; { $left } sin convertir; { $failed } con error.
batch-report = Informe guardado en { $path }.
batch-report-failed = No se pudo guardar el informe: { $error } Los archivos convertidos se conservan.
batch-inaccessible =
    { $n ->
        [one] 1 archivo tiene
       *[other] { $n } archivos tienen
    } elementos no accesibles; vea el informe.
batch-failures-title =
    { $n ->
        [one] 1 archivo con error
       *[other] { $n } archivos con error
    }
batch-failure-item = { $name }: { $reason }
batch-start-failed = No se pudo empezar a convertir: { $error } Compruebe la carpeta y el formato, y vuelva a intentarlo.
batch-thread-stopped = La conversión por lotes se detuvo de forma inesperada.
