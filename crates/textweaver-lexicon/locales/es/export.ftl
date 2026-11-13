### Export, preview, and audio export.

## Export and preview from the reader. F5 is the browser's reload key,
## not textweaver's.

common-no-document = No hay ningún documento abierto.
# Said after "Could not export:", so it starts in lower case. $path is a
# folder or a file; $error the system's reason.
publish-cannot-write-to = no se puede escribir en { $path }: { $error } Compruebe que se puede escribir en la carpeta.
publish-cannot-write = no se puede escribir { $path }: { $error } Compruebe que se puede escribir en su carpeta.
publish-start-failed = No se pudo iniciar la exportación: { $error } Espere un momento y vuelva a intentarlo.
publish-export-error = No se pudo exportar: { $error } Corríjalo y vuelva a exportar.
publish-export-as-label = Exportar como, Intro para { $path }
publish-export-over-source = No exportado: es el propio documento. Elija otro nombre.
# $format is the format's name, such as PDF, HTML, or Word.
publish-exporting = Exportando a { $format }.
publish-theme-title = Tema para la página HTML
publish-theme-intro = ¿Tema para la página HTML? { $first } primero, { $n } opciones. Escape cancela.
publish-writing-preview = Escribiendo la vista previa.
publish-preview-error = No se pudo escribir la vista previa: { $error } Corríjalo y vuelva a abrir la vista previa.
publish-still-exporting =
    { $secs ->
        [one] Todavía exportando a { $format }, { $secs } segundo.
       *[other] Todavía exportando a { $format }, { $secs } segundos.
    }
publish-still-previewing =
    { $secs ->
        [one] Todavía escribiendo la vista previa, { $secs } segundo.
       *[other] Todavía escribiendo la vista previa, { $secs } segundos.
    }
# "Browser preview follows" was chosen. $again is yes when a preview
# is open that reloads by itself only once it is opened again.
publish-follow-off = La vista previa del navegador no sigue nada: pulse F5 en el navegador tras guardar.
publish-follow-save =
    { $again ->
        [yes] La vista previa del navegador sigue cada guardado: la página se recarga sola. Vuelva a abrir la vista previa en el navegador para empezar.
       *[no] La vista previa del navegador sigue cada guardado: la página se recarga sola.
    }
publish-follow-typing =
    { $again ->
        [yes] La vista previa del navegador sigue lo que escribe: la página se recarga tras cada guardado y al hacer una pausa al escribir. Vuelva a abrir la vista previa en el navegador para empezar.
       *[no] La vista previa del navegador sigue lo que escribe: la página se recarga tras cada guardado y al hacer una pausa al escribir.
    }
# The first preview of a session: what the browser will do. $follow is
# the "Browser preview follows" setting: off, save, or typing.
publish-preview-first =
    { $follow ->
        [save] La vista previa se abre en su navegador y se recarga tras cada guardado.
        [typing] La vista previa se abre en su navegador y sigue lo que escribe.
       *[off] La vista previa se abre en su navegador. Pulse F5 allí tras cada guardado.
    }
# $error is the converter's reason.
publish-export-failed = Error al exportar a { $format }: { $error } Pruebe otro formato.
publish-preview-failed = Error en la vista previa: { $error } Guarde para volver a intentarlo.
# The converter's warnings: how many, and the first one.
publish-warnings =
    { $n ->
        [one] 1 advertencia: { $first }
       *[other] { $n } advertencias; la primera: { $first }
    }
# $file is the file's name, $folder its folder; $warned is empty or a
# space and publish-warnings.
publish-exported = Exportado a { $format }: { $file }. ¿Abrirlo? y o n. En { $folder }.{ $warned }
publish-report = Informe guardado como { $file }.
publish-report-issues =
    { $n ->
        [one] 1 elemento no accesible
       *[other] { $n } elementos no accesibles
    }; vea el informe.
publish-report-failed = No se pudo guardar el informe: { $error } Compruebe que se puede escribir en la carpeta de salida.
publish-preview-written-served = Vista previa escrita. Abriéndola en el navegador. Se recarga por sí sola después de cada guardado.{ $warned }
publish-preview-written = Vista previa escrita. Abriéndola en el navegador. Guardar la vuelve a escribir; luego pulse F5 en el navegador.{ $warned }
publish-preview-updated = Vista previa actualizada.
publish-preview-updated-press-f5 = Vista previa actualizada. Pulse F5 en el navegador.
# Said once for a document whose preview was cleaned of raw HTML
# (scripts, event handlers) because its folder is not trusted.
publish-preview-raw-html-left-out = HTML sin procesar omitido en la vista previa. Las carpetas de confianza están en Configuración.
publish-server-failed = No se pudo iniciar el servidor de recarga de la vista previa ({ $error }); se abre el archivo en su lugar.
publish-render-failed = No se pudo representar el texto: { $error } Salga del modo de edición para leer el texto.
publish-nothing-after-caret = Nada que leer después del cursor.
publish-listening = Escuchando el texto representado.

## The preview's reload server: shown in the browser.

preview-being-written = La vista previa se está escribiendo. Vuelva a cargar en un momento.

## Audio export (File, Export audio). Keep the meaning first: a
## 40-cell Braille display shows the start of the line. $name is a file
## name (essay.flac); $path a folder or a file's full path; $format a
## format's name (FLAC, MP3); $voice a voice's or engine's name; $wpm is
## words per minute; $length a length of time from the duration-*
## messages; $chapters and $n are numbers; $percent is a multiple of ten;
## $formats lists format names (M4B); $error is passed on as it is.
audio-format-title = Exportar audio como
audio-format-intro = Exportar { $name } como audio: elija un formato, { $n } opciones.
audio-no-ffmpeg =
    { $formats } { $n ->
        [one] necesita
       *[other] necesitan
    } ffmpeg, que no se encontró.
audio-format-flac = FLAC: sin pérdida, la mitad del tamaño de WAV
audio-format-wav = WAV: el más grande, se reproduce en todas partes
audio-format-mp3 = MP3: pequeño, se reproduce en todas partes
audio-format-opus = Opus: el más pequeño, pensado para la voz
audio-format-ogg = Ogg Vorbis: pequeño y abierto, se reproduce en la mayoría de los reproductores
audio-format-m4b = Audiolibro M4B, mediante ffmpeg
audio-format-mp4 = Vídeo con subtítulos: MP4, necesita ffmpeg
audio-format-html = Página de lectura: texto y audio, un archivo
audio-where-title = Dónde va el audio
audio-where-intro = ¿Dónde debe ir el audio?
audio-where-beside = Junto al documento, { $path }
audio-where-choose = En otra carpeta, elegida a continuación
audio-choose-folder = Elija la carpeta para el audio
audio-no-engine = Ningún motor de voz de aquí puede escribir archivos de audio. Instale eSpeak NG, o elija otro motor en el menú Voz.
audio-confirm = ¿Exportar { $name } con { $voice } a { $wpm } palabras por minuto, en { $path }? y o n
audio-started = Exportando { $name } como { $format }. Escape detiene.
audio-progress = Exportando audio, { $percent } por ciento.
audio-video-progress =
    { $minutes ->
        [one] Codificando video: { $fed } de { $all } fotogramas, falta 1 minuto.
       *[other] Codificando video: { $fed } de { $all } fotogramas, faltan unos { $minutes } minutos.
    }
audio-video-progress-soon = Codificando video: { $fed } de { $all } fotogramas, falta menos de un minuto.
audio-busy = Ya se está exportando { $name }. Escape detiene.
audio-stop-question = ¿Detener la exportación? No se guarda ningún archivo. y o n
audio-stopping = Deteniendo la exportación.
audio-still-exporting = Se sigue exportando el audio.
audio-stopped = Exportación de audio detenida; no se escribió ningún archivo.
audio-done =
    { $name } escrito: { $length }, { $chapters ->
        [one] 1 capítulo
       *[other] { $chapters } capítulos
    }.
audio-subtitles = Subtítulos en { $name }.
audio-failed = No se pudo exportar el audio: { $error } Pruebe otro formato u otra voz.
audio-thread-stopped = La exportación de audio se detuvo de forma inesperada.
