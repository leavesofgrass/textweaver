### The library, links and footnotes, and the dictionary.

## The library.

# $n is how many documents the scan has found.
library-still-scanning = Todavía explorando la biblioteca: { $n } encontrados hasta ahora.
library-scan-failed = No se pudo explorar la biblioteca: { $error } Revise las carpetas de la biblioteca en la configuración.
library-scanning = Explorando la biblioteca.
library-scan-progress = Explorando la biblioteca: { $n } encontrados hasta ahora.
library-scan-stopped = La exploración de la biblioteca se detuvo inesperadamente. Abra la biblioteca de nuevo para reintentar.
# $command is the command line that adds a folder; $key names the Open command's key.
library-empty = La biblioteca está vacía. Agregue una carpeta en Configuración, en Carpetas de la biblioteca, o abra un archivo con { $key }.
library-add-folder-choose = Elija la carpeta que se añadirá a la biblioteca
library-folder-added = { $name } se añadió a la biblioteca. Abra la biblioteca para ver sus documentos.
library-folder-already = { $name } ya está en la biblioteca.
library-intro =
    { $n ->
        [one] Biblioteca, { $n } documento. Escriba para filtrar, Intro abre uno, F2 edita los detalles, { $queue } lo pone en la cola.
       *[other] Biblioteca, { $n } documentos. Escriba para filtrar, Intro abre uno, F2 edita los detalles, { $queue } lo pone en la cola.
    }
library-title = Biblioteca

## Following links and footnotes.

links-none-here = No hay ningún enlace o nota al pie en el cursor.
# $text is the link's text.
common-link-no-address = El enlace { $text } no tiene dirección.
# $kind is mail or web; $target is the link's address.
links-open-question =
    { $kind ->
        [mail] ¿Abrir el enlace de correo? y o n. { $target }
       *[web] ¿Abrir el enlace web? y o n. { $target }
    }
# A link to a file on another computer (a network share). Keep the
# letters y and n: they are the keys that answer. $target is the link.
links-network-question = ¿Abrir un archivo de otro equipo? y o n. Sus datos de inicio de sesión pueden enviarse a ese equipo. { $target }
# The label of a heading reached by a link, at high verbosity.
links-heading-label = Encabezado
# $anchor is the heading name the link gives.
links-no-heading = No hay ningún encabezado llamado { $anchor } en este documento.
# $file is the file the link names.
links-file-not-found = El enlace va a { $file }, que no se encontró.
links-block-label = Bloque
links-no-block = No hay ningún bloque { $id } en este documento.
links-note-not-found = Enlace a una nota que no existe: { $name }.
vault-same-name = { $count } notas se llaman { $name }; se abrió la más cercana.
vault-properties-count = Propiedades: { $n }.
vault-properties-title = Propiedades
vault-no-properties = Este documento no tiene propiedades.
vault-property-row = { $name }: { $value }
vault-property-checked = marcada
vault-property-not-checked = sin marcar
vault-no-file = Este documento no está guardado en una carpeta, así que no tiene vault.
vault-partial = Solo se buscó en parte del vault; es muy grande.
vault-top-folder = carpeta principal
vault-backlink-row =
    { $links ->
        [one] { $title }, 1 enlace, en { $folder }
       *[other] { $title }, { $links } enlaces, en { $folder }
    }
vault-note-row = { $title }, en { $folder }
vault-backlinks-title = Notas que enlazan aquí
vault-backlinks-intro =
    { $n ->
        [one] Notas que enlazan aquí, 1 nota. Intro la abre.
       *[other] Notas que enlazan aquí, { $n } notas. Intro abre una.
    }
vault-no-backlinks = Ninguna nota enlaza aquí.
vault-unresolved-title = Enlaces a notas que no existen
vault-unresolved-intro =
    { $n ->
        [one] Enlaces a notas que no existen, 1 enlace.
       *[other] Enlaces a notas que no existen, { $n } enlaces.
    }
vault-no-unresolved = Todos los enlaces de esta nota llevan a una nota.
vault-note-tags-title = Etiquetas de esta nota
vault-note-tags-intro =
    { $n ->
        [one] Etiquetas de esta nota, 1 etiqueta. Intro lista las notas que la llevan.
       *[other] Etiquetas de esta nota, { $n } etiquetas. Intro lista las notas con una.
    }
vault-no-note-tags = Esta nota no tiene etiquetas.
vault-tags-title = Etiquetas del vault
vault-tags-intro =
    { $n ->
        [one] Etiquetas del vault, 1 etiqueta. Intro lista sus notas.
       *[other] Etiquetas del vault, { $n } etiquetas. Intro lista las notas de una etiqueta.
    }
vault-no-tags = No hay etiquetas en este vault.
vault-tag-row =
    { $n ->
        [one] { $tag }, 1 nota
       *[other] { $tag }, { $n } notas
    }
vault-tag-notes-title = Notas con una etiqueta
vault-tag-notes-intro =
    { $n ->
        [one] Etiqueta { $tag }, 1 nota. Intro la abre.
       *[other] Etiqueta { $tag }, { $n } notas. Intro abre una.
    }
vault-tag-no-notes = Ninguna nota lleva la etiqueta { $tag }.
# $title is the document's title; $heading is the heading arrived at, $anchor
# the heading a link names; $key names the History Back command's keys;
# $text is the first words at the position arrived at.
links-opened = Se abrió { $title }. Atrás: { $key }.
links-opened-at = Se abrió { $title }, en { $heading }. Atrás: { $key }.
links-opened-no-heading = Se abrió { $title }, al principio: no hay ningún encabezado llamado { $anchor }. Atrás: { $key }.
links-opened-no-block = Se abrió { $title }, al principio: no hay ningún bloque { $id }. Atrás: { $key }.
links-back-in = Atrás en { $title }: { $text }
links-forward-in = Adelante en { $title }: { $text }
# $label is the footnote's label, such as 1.
links-back-to-footnote-reference = Volver a la referencia de la nota al pie { $label }, línea { $line }.
# $text is the start of the note.
links-footnote = Nota al pie { $label }: { $text }
links-footnote-unreferenced = No hay ninguna referencia a la nota al pie { $label } en el texto.
links-footnote-no-note = La nota al pie { $label } no tiene nota.

## El filtro de la biblioteca, el diccionario y las velocidades.

# The library list filtered: $shown of $n documents match $filter.
library-title-filtered = Biblioteca, { $shown } de { $n } coinciden con { $filter }
# The filter was emptied: $n documents are shown.
library-filter-cleared =
    { $n ->
        [one] Filtro borrado, { $n } documento.
       *[other] Filtro borrado, { $n } documentos.
    }
# No document matches the filter $query.
library-filter-none = Ningún documento coincide con { $query }. Retroceso quita letras.
# $n documents match the filter.
library-filter-matched =
    { $n ->
        [one] { $n } documento coincide.
       *[other] { $n } documentos coinciden.
    }
# Said once when define word is used while the dictionary file is still opening.
define-still-loading = El diccionario aún se está cargando.

## PDF links. Said before the first line of the page a link inside
## a PDF goes to, when the page has no heading there (as links-heading-label
## is for a heading). $page is the page's printed number or label (12, iv).
links-page-label = Página { $page }
