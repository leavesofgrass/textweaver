### The library, links and footnotes, and the dictionary.

## The library.

# $n is how many documents the scan has found.
library-still-scanning = Die Bibliothek wird noch durchsucht: { $n } bisher gefunden.
library-scan-failed = Die Bibliothek konnte nicht durchsucht werden: { $error } Prüfen Sie die Bibliotheksordner in den Einstellungen.
library-scanning = Die Bibliothek wird durchsucht.
library-scan-progress = Die Bibliothek wird durchsucht: { $n } bisher gefunden.
library-scan-stopped = Die Bibliotheksdurchsuchung wurde unerwartet gestoppt. Öffnen Sie die Bibliothek erneut, um es noch einmal zu versuchen.
# $command is the command line that adds a folder; $key names the Open command's key.
library-empty = Die Bibliothek ist leer. Fügen Sie in den Einstellungen unter Bibliotheksordner einen Ordner hinzu, oder öffnen Sie eine Datei mit { $key }.
library-add-folder-choose = Wählen Sie den Ordner, der zur Bibliothek hinzugefügt wird
library-folder-added = { $name } wurde zur Bibliothek hinzugefügt. Öffnen Sie die Bibliothek, um seine Dokumente zu sehen.
library-folder-already = { $name } ist bereits in der Bibliothek.
library-intro =
    { $n ->
        [one] Bibliothek, { $n } Dokument. Tippen filtert, Eingabetaste öffnet eines, F2 bearbeitet Details, { $queue } setzt es auf die Leseliste.
       *[other] Bibliothek, { $n } Dokumente. Tippen filtert, Eingabetaste öffnet eines, F2 bearbeitet Details, { $queue } setzt es auf die Leseliste.
    }
library-title = Bibliothek

## Following links and footnotes.

links-none-here = Kein Link oder keine Fußnote am Cursor.
# $text is the link's text.
common-link-no-address = Der Link { $text } hat keine Adresse.
# $kind is mail or web; $target is the link's address.
links-open-question =
    { $kind ->
        [mail] Mail-Link öffnen? y oder n. { $target }
       *[web] Web-Link öffnen? y oder n. { $target }
    }
# A link to a file on another computer (a network share). Keep the
# letters y and n: they are the keys that answer. $target is the link.
links-network-question = Datei auf einem anderen Computer öffnen? y oder n. Ihre Anmeldung kann dorthin gesendet werden. { $target }
# The label of a heading reached by a link, at high verbosity.
links-heading-label = Überschrift
# $anchor is the heading name the link gives.
links-no-heading = Keine Überschrift namens { $anchor } in diesem Dokument.
# $file is the file the link names.
links-file-not-found = Der Link führt zu { $file }, was nicht gefunden wurde.
links-block-label = Block
links-no-block = Kein Block { $id } in diesem Dokument.
links-note-not-found = Link auf eine Notiz, die es nicht gibt: { $name }.
vault-same-name = { $count } Notizen heißen { $name }; die nächstgelegene wurde geöffnet.
vault-properties-count = Eigenschaften: { $n }.
vault-properties-title = Eigenschaften
vault-no-properties = Dieses Dokument hat keine Eigenschaften.
vault-property-row = { $name }: { $value }
vault-property-checked = aktiviert
vault-property-not-checked = nicht aktiviert
vault-no-file = Dieses Dokument ist in keinem Ordner gespeichert und hat daher keinen Vault.
vault-partial = Nur ein Teil des Vaults wurde durchsucht; er ist sehr groß.
vault-top-folder = oberster Ordner
vault-backlink-row =
    { $links ->
        [one] { $title }, 1 Link, in { $folder }
       *[other] { $title }, { $links } Links, in { $folder }
    }
vault-note-row = { $title }, in { $folder }
vault-backlinks-title = Notizen, die hierher verlinken
vault-backlinks-intro =
    { $n ->
        [one] Notizen, die hierher verlinken, 1 Notiz. Die Eingabetaste öffnet sie.
       *[other] Notizen, die hierher verlinken, { $n } Notizen. Die Eingabetaste öffnet eine.
    }
vault-no-backlinks = Keine Notiz verlinkt hierher.
vault-unresolved-title = Links auf Notizen, die es nicht gibt
vault-unresolved-intro =
    { $n ->
        [one] Links auf Notizen, die es nicht gibt, 1 Link.
       *[other] Links auf Notizen, die es nicht gibt, { $n } Links.
    }
vault-no-unresolved = Jeder Link in dieser Notiz führt zu einer Notiz.
vault-note-tags-title = Tags in dieser Notiz
vault-note-tags-intro =
    { $n ->
        [one] Tags in dieser Notiz, 1 Tag. Die Eingabetaste listet die Notizen damit.
       *[other] Tags in dieser Notiz, { $n } Tags. Die Eingabetaste listet die Notizen mit einem.
    }
vault-no-note-tags = Diese Notiz hat keine Tags.
vault-tags-title = Tags im Vault
vault-tags-intro =
    { $n ->
        [one] Tags im Vault, 1 Tag. Die Eingabetaste listet seine Notizen.
       *[other] Tags im Vault, { $n } Tags. Die Eingabetaste listet die Notizen eines Tags.
    }
vault-no-tags = Keine Tags in diesem Vault.
vault-tag-row =
    { $n ->
        [one] { $tag }, 1 Notiz
       *[other] { $tag }, { $n } Notizen
    }
vault-tag-notes-title = Notizen mit einem Tag
vault-tag-notes-intro =
    { $n ->
        [one] Tag { $tag }, 1 Notiz. Die Eingabetaste öffnet sie.
       *[other] Tag { $tag }, { $n } Notizen. Die Eingabetaste öffnet eine.
    }
vault-tag-no-notes = Keine Notiz trägt den Tag { $tag }.
# $title is the document's title; $heading is the heading arrived at, $anchor
# the heading a link names; $key names the History Back command's keys;
# $text is the first words at the position arrived at.
links-opened = { $title } geöffnet. Zurück: { $key }.
links-opened-at = { $title } geöffnet, bei { $heading }. Zurück: { $key }.
links-opened-no-heading = { $title } geöffnet, am Anfang: keine Überschrift namens { $anchor }. Zurück: { $key }.
links-opened-no-block = { $title } geöffnet, am Anfang: kein Block { $id }. Zurück: { $key }.
links-back-in = Zurück in { $title }: { $text }
links-forward-in = Vorwärts in { $title }: { $text }
# $label is the footnote's label, such as 1.
links-back-to-footnote-reference = Zurück zum Fußnotenverweis { $label }, Zeile { $line }.
# $text is the start of the note.
links-footnote = Fußnote { $label }: { $text }
links-footnote-unreferenced = Kein Verweis auf Fußnote { $label } im Text.
links-footnote-no-note = Fußnote { $label } hat keine Notiz.

## Der Bibliotheksfilter, das Wörterbuch und die Geschwindigkeiten.

# The library list filtered: $shown of $n documents match $filter.
library-title-filtered = Bibliothek, { $shown } von { $n } stimmen mit { $filter } überein
# The filter was emptied: $n documents are shown.
library-filter-cleared =
    { $n ->
        [one] Filter gelöscht, { $n } Dokument.
       *[other] Filter gelöscht, { $n } Dokumente.
    }
# No document matches the filter $query.
library-filter-none = Kein Dokument stimmt mit { $query } überein. Rücktaste entfernt Buchstaben.
# $n documents match the filter.
library-filter-matched =
    { $n ->
        [one] { $n } Dokument stimmt überein.
       *[other] { $n } Dokumente stimmen überein.
    }
# Said once when define word is used while the dictionary file is still opening.
define-still-loading = Das Wörterbuch wird noch geladen.

## PDF links. Said before the first line of the page a link inside
## a PDF goes to, when the page has no heading there (as links-heading-label
## is for a heading). $page is the page's printed number or label (12, iv).
links-page-label = Seite { $page }
