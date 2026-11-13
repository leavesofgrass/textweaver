### The library, links and footnotes, and the dictionary.

## The library.

# $n is how many documents the scan has found.
library-still-scanning = Still scanning the library: { $n } found so far.
library-scan-failed = Could not scan the library: { $error } Check the library folders in Settings.
library-scanning = Scanning the library.
library-scan-progress = Scanning the library: { $n } found so far.
library-scan-stopped = The library scan stopped unexpectedly. Open the library again to retry.
# $command is the command line that adds a folder; $key names the Open command's key.
library-empty = The library is empty. Add a folder in Settings, under Library folders, or open a file with { $key }.
library-add-folder-choose = Choose the folder to add to the library
library-folder-added = Added { $name } to the library. Open the library to see its documents.
library-folder-already = { $name } is already in the library.
library-intro =
    { $n ->
        [one] Library, { $n } document. Type to filter, Enter opens one, F2 edits details, { $queue } queues it.
       *[other] Library, { $n } documents. Type to filter, Enter opens one, F2 edits details, { $queue } queues it.
    }
library-title = Library

## Following links and footnotes.

links-none-here = No link or footnote at the cursor.
# $text is the link's text.
common-link-no-address = The link { $text } has no address.
# $kind is mail or web; $target is the link's address.
links-open-question =
    { $kind ->
        [mail] Open mail link? y or n. { $target }
       *[web] Open web link? y or n. { $target }
    }
# A link to a file on another computer (a network share). Keep the
# letters y and n: they are the keys that answer. $target is the link.
links-network-question = Open a file on another computer? y or n. Your sign-in may be sent to it. { $target }
# The label of a heading reached by a link, at high verbosity.
links-heading-label = Heading
# $anchor is the heading name the link gives.
links-no-heading = No heading called { $anchor } in this document.
# $file is the file the link names.
links-file-not-found = The link goes to { $file }, which was not found.
links-block-label = Block
links-no-block = No block { $id } in this document.
links-note-not-found = Link to a note that doesn't exist: { $name }.
vault-same-name = { $count } notes named { $name }; the nearest opened.
vault-properties-count = Properties: { $n }.
vault-properties-title = Properties
vault-no-properties = This document has no properties.
vault-property-row = { $name }: { $value }
vault-property-checked = checked
vault-property-not-checked = not checked
vault-no-file = This document is not saved in a folder, so it has no vault.
vault-partial = Only part of the vault was searched; it is very large.
vault-top-folder = top folder
vault-backlink-row =
    { $links ->
        [one] { $title }, 1 link, in { $folder }
       *[other] { $title }, { $links } links, in { $folder }
    }
vault-note-row = { $title }, in { $folder }
vault-backlinks-title = Notes that link here
vault-backlinks-intro =
    { $n ->
        [one] Notes that link here, 1 note. Enter opens it.
       *[other] Notes that link here, { $n } notes. Enter opens one.
    }
vault-no-backlinks = No notes link here.
vault-unresolved-title = Links to notes that don't exist
vault-unresolved-intro =
    { $n ->
        [one] Links to notes that don't exist, 1 link.
       *[other] Links to notes that don't exist, { $n } links.
    }
vault-no-unresolved = Every link in this note leads to a note.
vault-note-tags-title = Tags in this note
vault-note-tags-intro =
    { $n ->
        [one] Tags in this note, 1 tag. Enter lists the notes with it.
       *[other] Tags in this note, { $n } tags. Enter lists the notes with one.
    }
vault-no-note-tags = This note has no tags.
vault-tags-title = Tags in the vault
vault-tags-intro =
    { $n ->
        [one] Tags in the vault, 1 tag. Enter lists its notes.
       *[other] Tags in the vault, { $n } tags. Enter lists a tag's notes.
    }
vault-no-tags = No tags in this vault.
vault-tag-row =
    { $n ->
        [one] { $tag }, 1 note
       *[other] { $tag }, { $n } notes
    }
vault-tag-notes-title = Notes with a tag
vault-tag-notes-intro =
    { $n ->
        [one] Tag { $tag }, 1 note. Enter opens it.
       *[other] Tag { $tag }, { $n } notes. Enter opens one.
    }
vault-tag-no-notes = No notes carry the tag { $tag }.
# $title is the document's title; $heading is the heading arrived at, $anchor
# the heading a link names; $key names the History Back command's keys;
# $text is the first words at the position arrived at.
links-opened = Opened { $title }. Back: { $key }.
links-opened-at = Opened { $title }, at { $heading }. Back: { $key }.
links-opened-no-heading = Opened { $title }, at the start: no heading called { $anchor }. Back: { $key }.
links-opened-no-block = Opened { $title }, at the start: no block { $id }. Back: { $key }.
links-back-in = Back in { $title }: { $text }
links-forward-in = Forward in { $title }: { $text }
# $label is the footnote's label, such as 1.
links-back-to-footnote-reference = Back to footnote reference { $label }, line { $line }.
# $text is the start of the note.
links-footnote = Footnote { $label }: { $text }
links-footnote-unreferenced = No reference to footnote { $label } in the text.
links-footnote-no-note = Footnote { $label } has no note.

## The library's filter, the dictionary, and speed presets.

# The library list filtered: $shown of $n documents match $filter.
library-title-filtered = Library, { $shown } of { $n } match { $filter }
# The filter was emptied: $n documents are shown.
library-filter-cleared =
    { $n ->
        [one] Filter cleared, { $n } document.
       *[other] Filter cleared, { $n } documents.
    }
# No document matches the filter $query.
library-filter-none = No documents match { $query }. Backspace removes letters.
# $n documents match the filter.
library-filter-matched =
    { $n ->
        [one] { $n } document matches.
       *[other] { $n } documents match.
    }
# Said once when define word is used while the dictionary file is still opening.
define-still-loading = Dictionary still loading.

## PDF links. Said before the first line of the page a link inside
## a PDF goes to, when the page has no heading there (as links-heading-label
## is for a heading). $page is the page's printed number or label (12, iv).
links-page-label = Page { $page }
