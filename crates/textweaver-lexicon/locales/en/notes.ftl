### Notes, highlights, bookmarks, relations, and find.

## Notes and highlights.

notes-nothing-to-attach = Nothing here to attach a note to.
# $on is the start of the passage the note is on.
notes-added = Note added on: { $on }
# $tags are the note's tags, joined with commas.
notes-added-with-tags = Note added with tags { $tags } on: { $on }
# An item in the notes list. $anchor is the passage; $lost is yes when the
# passage was not found after the file changed.
notes-item =
    { $lost ->
        [yes] { $note }, line { $line }. On: { $anchor } Not found after the file changed.
       *[no] { $note }, line { $line }. On: { $anchor }
    }
notes-none = No notes. To add one: { $key }.
notes-list-title = Notes
notes-list-intro =
    { $n ->
        [one] Notes, 1 item. Enter goes to a note, Delete deletes it, F2 edits it, Space opens its links. C makes a card.
       *[other] Notes, { $n } items. Enter goes to a note, Delete deletes it, F2 edits it, Space opens its links. C makes a card.
    }
# Said on jumping to a note: its text, then the passage it is on.
notes-note-content = { $note }. On: { $anchor }
# $i is the note's number, $n how many notes there are.
notes-note-label = Note { $i } of { $n }
notes-deleted = Note deleted: { $text }.
notes-none-here = No note or highlight here.
notes-unchanged = Note unchanged.
notes-updated = Note updated.
notes-nothing-to-highlight = Nothing here to highlight.
notes-highlight-removed = Highlight removed: { $text }
notes-highlighted-at = Highlighted, { $name }, at { $pct } percent: { $text }
notes-highlighted = Highlighted, { $name }: { $text }
# An item in the highlights list. $name is the highlight's palette name;
# $lost is yes when the text was not found after the file changed.
notes-highlight-item =
    { $lost ->
        [yes] { $name }: { $text }, line { $line }, not found after the file changed
       *[no] { $name }: { $text }, line { $line }
    }
notes-no-highlights = No highlights. To make one: { $key }.
notes-highlights-title = Highlights
notes-highlights-intro =
    { $n ->
        [one] Highlights, 1 item. Enter goes to one, Delete removes it, F2 changes its name, Space shows only its name. C makes a card.
       *[other] Highlights, { $n } items. Enter goes to one, Delete removes it, F2 changes its name, Space shows only its name. C makes a card.
    }
# The label said before a highlight's text on jumping to it.
notes-highlight-label = Highlight, { $name }
notes-highlight-changed = Highlight changed to { $name }: { $text }
# $n is the palette entry's number, $key the keys of Highlight with a name.
notes-palette-no-entry = No highlight name { $n } in the palette. To choose a name: { $key }.
notes-highlights-named-title = Highlights: { $name }
notes-highlights-named-intro =
    { $n ->
        [one] Highlights named { $name }, 1 item. Space shows every highlight.
       *[other] Highlights named { $name }, { $n } items. Space shows every highlight.
    }
notes-palette-title = Highlight names
# A palette name: the name, how many highlights have it, its shape and its color.
notes-palette-row =
    { $count ->
        [one] { $name }, 1 highlight, { $shape }, { $color }
       *[other] { $name }, { $count } highlights, { $shape }, { $color }
    }
notes-palette-intro-highlight = Highlight with which name? { $n } names. Enter chooses one.
notes-palette-intro-change = Change the highlight to which name? { $n } names. Enter chooses one.
notes-palette-intro-collect = Collect the highlights of which name? { $n } names. Enter writes them as a list.
notes-collect-none = No highlights named { $name }.
# The collected list's own title (Markdown).
notes-collect-title = Highlights named { $name }: { $title }
notes-collect-line = (line { $line })
# Keep the letters y and n: they are the keys that answer.
notes-collect-saved =
    { $n ->
        [one] 1 highlight named { $name } saved as { $file }. Open it? y or n. In { $folder }.
       *[other] { $n } highlights named { $name } saved as { $file }. Open it? y or n. In { $folder }.
    }
palette-shape-underline = underline
palette-shape-double-underline = double underline
palette-shape-bold = bold
palette-shape-dotted = dotted underline
palette-shape-brackets = brackets
palette-shape-symbol = symbol
# Shown while reading reaches a note's passage.
notes-signal = Note: { $text }
# Said after moving onto a note's passage.
notes-has-note = Has a note: { $text }

## Relations between notes (the knowledge graph as lists).

# The ten relation types, as they read aloud. Lower case: they start list
# rows such as "supports: Chapter 3 note".
relations-type-conflicts-with = conflicts with
relations-type-supports = supports
relations-type-is-example-of = is an example of
relations-type-cites = cites
relations-type-contradicts = contradicts
relations-type-defines = defines
relations-type-extends = extends
relations-type-see-also = see also
relations-type-precedes = precedes
relations-type-follows = follows
# After a note in the notes list: how many links it has to other notes,
# and how many notes link to it.
relations-count = Links: { $out } out, { $in } in.
relations-note-title = Links of: { $note }
# $title is a list title, $filter the type filter typed.
relations-title-filtered = { $title }, filter: { $filter }
relations-note-intro = Links of { $note }: { $out } out, { $in } in. Enter follows a link, F2 changes it, Delete removes it. Type to filter by type.
# A link of the note: its type, then the note it goes to.
relations-out-item = { $type }: { $target }
# A note in another document. $doc is that document's title.
relations-target-in = { $note }, in { $doc }
relations-target-missing = a note not found
relations-empty-note = Empty note
relations-incoming-row =
    { $n ->
        [0] What links here: nothing yet
        [one] What links here: 1 note
       *[other] What links here: { $n } notes
    }
relations-add-row = Add a link
relations-backlinks-title = What links here: { $note }
relations-backlinks-intro =
    { $n ->
        [one] What links to { $note }: 1 note. Enter goes to it. Type to filter by type.
       *[other] What links to { $note }: { $n } notes. Enter goes to one. Type to filter by type.
    }
# A note linking here: the link's type, then that note.
relations-backlink-item = { $type } this, from: { $note }
relations-none-in = Nothing links to this note yet.
relations-types-title = Link type for: { $note }
relations-types-intro = Choose the type of link, 10 types. Type to filter.
# $type is the link type chosen.
relations-targets-title = { $type }: which note?
relations-targets-intro =
    { $n ->
        [0] No other note here. Choose a note in another document.
        [one] Choose the note to link to: 1 note. Enter links it.
       *[other] Choose the note to link to: { $n } notes. Enter links it.
    }
relations-other-document-row = A note in another document
relations-documents-title = Documents with notes
relations-documents-intro = Documents with notes: { $n }. Enter lists a document's notes.
relations-document-item =
    { $n ->
        [one] { $title }, 1 note
       *[other] { $title }, { $n } notes
    }
relations-no-other-documents = No other document in the library has notes.
relations-linked = Linked: { $type } { $target }.
relations-changed = Link changed: { $type } { $target }.
relations-already = Already linked: { $type } { $target }.
relations-removed = Link removed: { $type } { $target }.
relations-remove-question = Remove this link? y or n
relations-nothing-to-remove = Only a link can be removed here.
relations-note-gone = Note not found: it was deleted, or its document is gone.
relations-document-missing = Document not found: { $file }.
relations-no-note-here = No note here. Links belong to notes; add one: { $key }.
relations-filter-cleared = Filter cleared, { $n } shown.
relations-filter-none = Nothing matches { $filter }.
relations-filter-matched = Filter { $filter }: { $n } shown.

## Export the knowledge graph (B1-g2).

graph-export-title = Export the knowledge graph as
# $links is the number of links between notes in the library.
graph-export-intro = Knowledge graph, links: { $links }. Choose a format; the Markdown list is the text to read.
graph-export-empty = No links between notes to export. Add one from a note's links list.
graph-format-md = Markdown list, the text to read
graph-format-json = JSON, for Gephi and Cytoscape
graph-format-dot = DOT, for Graphviz
graph-format-graphml = GraphML, for Gephi, Cytoscape and yEd
graph-format-mermaid = Mermaid diagram
graph-format-plantuml = PlantUML diagram
graph-format-csv = CSV edge list, for spreadsheets
# Keep the letters y and n: they are the keys that answer. $file is the
# file name, $folder the folder it was saved in.
graph-export-saved = Knowledge graph saved as { $file }. Open it? y or n. In { $folder }.
graph-export-failed = Could not write the knowledge graph: { $error } Check that the folder can be written to.

## Bookmarks: rename and delete.

notes-choose-bookmark-delete = Choose a bookmark and press Delete.
notes-choose-bookmark-rename = Choose a bookmark and press F2 to rename it.
notes-bookmark-deleted = Bookmark { $name } deleted.
notes-renaming-bookmark = Renaming bookmark { $name }.
notes-bookmark-unchanged = Bookmark unchanged.
# $name is the name asked for, $old the bookmark's name.
notes-bookmark-name-taken = There is already a bookmark called { $name }. Bookmark { $old } unchanged.
notes-bookmark-renamed = Bookmark { $old } renamed to { $name }.

## Find, bookmarks, and selection.

marks-cannot-search = Cannot search: { $error } Check the pattern between the slashes.
# $pattern is the text searched for.
marks-no-matches = No matches for { $pattern }.
# The label of a match reached by Find, at high verbosity; $number is its place among $n matches.
marks-match-label = Match { $number } of { $n }
# Find wrapped past an end of the document; $dir is next (to the top) or previous (to the bottom); $message says the match.
marks-find-wrapped =
    { $dir ->
        [next] Wrapped to top. { $message }
       *[previous] Wrapped to bottom. { $message }
    }
# $name is the bookmark's name, such as mark1.
marks-bookmark-already-here = Bookmark { $name } is already here.
common-bookmark-set = Bookmark { $name } set at { $pct } percent.
marks-no-bookmarks = No bookmarks. To add one: { $key }.
marks-bookmarks-intro =
    { $n ->
        [one] Bookmarks, { $n } item. Enter goes to one, Delete deletes it, F2 renames it.
       *[other] Bookmarks, { $n } items. Enter goes to one, Delete deletes it, F2 renames it.
    }
# One line of the bookmark list; $lost is yes when the bookmark's text was not found after the file changed; $text is the start of its line.
marks-bookmark-item =
    { $lost ->
        [yes] { $name } (not found after the file changed), line { $line }, { $pct } percent: { $text }
       *[no] { $name }, line { $line }, { $pct } percent: { $text }
    }
marks-bookmarks-title = Bookmarks
# The label of a bookmark reached, at high verbosity.
marks-bookmark-label = Bookmark { $name }
marks-selection-cleared = Selection cleared.
# $text is the selected text, shortened.
marks-selected = Selected { $text }
