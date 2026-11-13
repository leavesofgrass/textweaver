### Authoring: lists, citations, find and replace, tables, quick wins, and dictation.

## Authoring lists: the outline, the citation picker, spelling, grammar, and templates.

# An outline item: $text is the heading's text, $level its level.
lists-outline-item = { $text }, level { $level }
# $n is the number of headings.
lists-outline-title =
    { $n ->
        [one] Outline, { $n } heading
       *[other] Outline, { $n } headings
    }
# $shown headings of $n match the filter $filter typed so far.
lists-outline-title-filtered = Outline, { $shown } of { $n } match { $filter }
# $n is the number of references.
lists-citations-title =
    { $n ->
        [one] Insert citation, { $n } reference
       *[other] Insert citation, { $n } references
    }
# $shown references of $n match the filter $filter typed so far.
lists-citations-title-filtered = Insert citation, { $shown } of { $n } match { $filter }
# $word is the misspelled word.
lists-spelling-title = Spelling of { $word }
lists-spelling-add = Add { $word } to your word list
lists-leave-as-is = Leave it as it is
# $words are the words the grammar fixes are for.
lists-grammar-title = Grammar fixes for { $words }
# $n is the number of templates.
lists-templates-title = New document from a template, { $n } templates
lists-no-filter = This list does not filter.
# The filter was emptied: $n items are shown.
lists-filter-cleared-headings =
    { $n ->
        [one] Filter cleared, { $n } heading.
       *[other] Filter cleared, { $n } headings.
    }
lists-filter-cleared-references =
    { $n ->
        [one] Filter cleared, { $n } reference.
       *[other] Filter cleared, { $n } references.
    }
lists-filter-cleared-items =
    { $n ->
        [one] Filter cleared, { $n } item.
       *[other] Filter cleared, { $n } items.
    }
# Nothing matches the filter $query.
lists-filter-none-headings = No headings match { $query }. Backspace removes letters.
lists-filter-none-references = No references match { $query }. Backspace removes letters.
lists-filter-none-items = No items match { $query }. Backspace removes letters.
# $n items match the filter.
lists-filter-matched-headings =
    { $n ->
        [one] { $n } heading match.
       *[other] { $n } headings match.
    }
lists-filter-matched-references =
    { $n ->
        [one] { $n } reference match.
       *[other] { $n } references match.
    }
lists-filter-matched-items =
    { $n ->
        [one] { $n } item match.
       *[other] { $n } items match.
    }
lists-no-headings = This document has no headings.
# $n is the number of headings; the keys are the outline list's own.
lists-outline-intro =
    { $n ->
        [one] Outline, { $n } heading. Type to filter, Enter goes to a heading, Escape closes.
       *[other] Outline, { $n } headings. Type to filter, Enter goes to a heading, Escape closes.
    }
# $heading is the text of the heading the cursor is under.
lists-outline-here = You are under { $heading }.

## Citations: inserting, looking up, importing, checking, and the bibliography.

citations-on = Citations on.
citations-off = Citations off.
book-audio-on = Book audio on. The recording plays where the book has one.
book-audio-off = Book audio off. Speech reads the book.
book-audio-rate = Rate changes do not apply to recorded audio.
# $key names the Add Reference command's keys.
citations-library-empty = Your reference library is empty. Add a reference by DOI or ISBN with { $key }, or run import references from the command palette.
# $n is how many references the picker lists.
citations-picker-intro =
    { $n ->
        [one] Insert citation, { $n } reference. Type to filter, Enter chooses, Escape cancels.
       *[other] Insert citation, { $n } references. Type to filter, Enter chooses, Escape cancels.
    }
# $text is what was typed at the locator prompt.
citations-locator-unreadable = Could not read the locator { $text }. Type a page such as 12, pages such as 3-5, or chapter 2; Enter alone for none.
citations-insert-failed = Could not insert the citation: { $error } The text is unchanged.
# $what is the identifier being looked up, as the citation library describes it.
citations-looking-up = Looking up { $what }.
citations-lookup-not-started = Could not start the lookup: { $error } Wait a moment, then try again.
# $input is the DOI or ISBN as typed.
citations-lookup-failed = Could not look up { $input }: { $error } Check the DOI or ISBN and the connection.
citations-no-library-to-add-to = There is no library to add to: { -brand } keeps no files in this session.
citations-library-save-failed = Could not save the library: { $error } Check that its folder can be written to.
# $n is how many citations the document has.
citations-found-no-library =
    { $n ->
        [one] { $n } citation found. { -brand } keeps no library in this session.
       *[other] { $n } citations found. { -brand } keeps no library in this session.
    }
citations-check-failed = Could not check the citations: { $error } Check the reference library file.
citations-no-library-to-import-into = There is no library to import into: { -brand } keeps no files in this session.
# $file is the file's path.
citations-import-failed = Could not import { $file }: { $error } Check that it is a .bib, .ris, or .json file.
# $style is the style's name from the front matter, such as apa.
citations-style-unusable = Cannot use the citation style { $style }: { $error } Check the style name at the top of the document.
citations-format-failed = Could not format the citations: { $error } Check the citation keys and the library.
# $key names the Insert Citation command's keys.
citations-none-yet = The document has no citations yet. Insert one with { $key }.
citations-nothing-to-list = None of the cited works is in your library, so there is nothing to list.
# $n is how many entries went in; $style is the style's name, such as apa.
citations-bibliography-inserted =
    { $n ->
        [one] Inserted the bibliography, { $n } entry, { $style } style.
       *[other] Inserted the bibliography, { $n } entries, { $style } style.
    }
# Follows citations-bibliography-inserted; $keys are citation keys joined with commas.
citations-not-in-library = Not in the library: { $keys }.
citations-bibliography-insert-failed = Could not insert the bibliography: { $error } The text is unchanged.

## Find and replace, one match at a time.

# $title is replace-match-title. Keep the letters r, s and a: they are the
# keys that answer.
replace-match-question = { $title }. The line: { $context }. Press r to replace, s to skip, a to replace all, Escape to stop.
# The replace list's title when no match is being asked about.
replace-title = Replace
# $n is this match's number, $total the number of matches, $line the line
# number, $found the matched text, and $result what it becomes (both
# shortened); the -removed form is for an empty replacement. $context in
# replace-match-question is the text of the match's line.
replace-match-title = Match { $n } of { $total }, line { $line }: { $found } becomes { $result }
replace-match-title-removed = Match { $n } of { $total }, line { $line }: { $found } is removed
replace-item-this = Replace this one
replace-item-skip = Skip this one
replace-item-rest = Replace all the rest
# $state is common-on or common-off.
replace-item-match-case = Match case: { $state }
replace-item-whole-words = Whole words only: { $state }
replace-failed = Could not replace: { $error } The text is unchanged.
# Said after switching match case; $state is common-on or common-off, and
# $n is the number of matches now.
replace-match-case-now =
    { $n ->
        [one] Match case { $state }. 1 match.
       *[other] Match case { $state }. { $n } matches.
    }
replace-whole-words-now =
    { $n ->
        [one] Whole words only { $state }. 1 match.
       *[other] Whole words only { $state }. { $n } matches.
    }
# $query is the text that was searched for.
replace-replaced =
    { $n ->
        [one] Replaced 1 match.
       *[other] Replaced { $n } matches.
    }
replace-replaced-skipped = Replaced { $n }, skipped { $skipped }.
replace-stopped = Stopped. Replaced { $n }, skipped { $skipped }.
# Find and replace with regular expressions (B1-fr). $state is common-on
# or common-off; $n is the number of matches now.
replace-item-regex = Regular expression: { $state }
replace-item-across-lines = Across lines: { $state }
replace-regex-now =
    { $n ->
        [one] Regular expression { $state }. 1 match.
       *[other] Regular expression { $state }. { $n } matches.
    }
replace-across-lines-now =
    { $n ->
        [one] Across lines { $state }. 1 match.
       *[other] Across lines { $state }. { $n } matches.
    }
# $problem is search-invalid-pattern: switching the option would make the
# pattern invalid.
replace-option-refused = { $problem } The option is unchanged.
# Asked once before replacing all the rest; $n is how many. Keep y and n.
replace-all-question =
    { $n ->
        [one] Replace the 1 remaining match? y or n
       *[other] Replace all { $n } remaining matches? y or n
    }
replace-all-declined = Nothing replaced.
# The search options list, and the options as named in search-options-on.
search-options-title = Search options
search-option-match-case = match case
search-option-whole-words = whole words
search-option-regex = regular expression
search-option-across-lines = across lines
# Said as Find or Replace opens when an option is on; $list joins the
# options' names with commas.
search-options-on = Options on: { $list }.
# $at is the character where the pattern fails, counting from 1; $reason
# is the regular expression engine's own explanation (in English).
search-invalid-pattern = Invalid pattern at character { $at }: { $reason }.
search-invalid-pattern-anywhere = Invalid pattern: { $reason }.

## Markdown structure said in edit mode, before a line's text or as it
## is typed.

mdline-heading-level = heading level { $level }
mdline-bullet = bullet
# A numbered list item; $n is its number.
mdline-item = item { $n }
# $item is mdline-bullet or mdline-item.
mdline-task-done = { $item }, task done
mdline-task-not-done = { $item }, task not done
mdline-table-row = table row
mdline-quote = quote
mdline-code-fence = code fence
mdline-task = task
# Said as "1. " is typed at the start of a line; $n is the number as typed.
mdline-numbered-item = numbered item { $n }

## Moving through tables by row and cell. $dir is next (forward) or
## previous (backward).

common-not-in-table = Not in a table.
tables-edge-of-table =
    { $dir ->
        [next] End of table.
       *[previous] Start of table.
    }
tables-edge-of-row =
    { $dir ->
        [next] End of row.
       *[previous] Start of row.
    }
# $cell is the cell's text, after its column header and a colon when it has one.
tables-header-row = Header row, { $cell }
tables-row = Row { $row }, { $cell }
# High verbosity: $message is what the move said, then where it is.
tables-with-position = { $message }. Row { $row } of { $rows }, column { $col } of { $cols }
# Say Position in a table.
tables-position = Table, row { $row } of { $rows }, column { $col } of { $cols }.

## Authoring quick wins: word count, links, clipboard, table cells,
## deleting words, and cycling settings.

# Code block languages said with ordinary words; proper names such as
# Python are not translated.
authoring-language-jsx = JavaScript with JSX
authoring-language-tsx = TypeScript with JSX
authoring-language-shell = shell
authoring-language-batch = Windows batch
authoring-language-c-header = C header
authoring-language-cpp = C plus plus
authoring-language-csharp = C sharp
authoring-language-diff = diff
authoring-language-plain-text = plain text
authoring-language-mermaid = Mermaid diagram
authoring-grammar-not-in-build = Grammar checking is not in this version.
# $count is $n with thousands separators.
authoring-word-count-selection =
    { $n ->
        [one] 1 word in the selection.
       *[other] { $count } words in the selection.
    }
authoring-word-count-document =
    { $n ->
        [one] 1 word in the document.
       *[other] { $count } words in the document.
    }
# $text is the link's text.
authoring-link-address = Link address: { $url }
authoring-link-named-address = Link { $text }, address: { $url }
authoring-no-link = No link at the cursor.
authoring-typing-echo =
    { $echo ->
        [characters-and-words] Typing echo: characters and words.
        [characters] Typing echo: characters.
        [words] Typing echo: words.
       *[none] Typing echo: none.
    }
authoring-nothing-to-copy = Nothing selected to copy.
# $text is the first words of what was copied.
authoring-copied = Copied: { $text }
authoring-copied-sentence = Copied the sentence: { $text }
authoring-nothing-to-cut = Nothing selected to cut.
authoring-cut = Cut: { $text }
# $dir is next (moving forward) or previous.
authoring-table-edge =
    { $dir ->
        [next] End of table.
       *[previous] Start of table.
    }
# A column with no header text.
authoring-table-column = column { $n }
# Moving into a new row: $header is the column's header, $content the cell.
authoring-table-cell-row = Row { $row }. { $header }: { $content }
authoring-nothing-to-select = Nothing to select.
authoring-selected-all =
    { $n ->
        [one] Selected all, 1 word.
       *[other] Selected all, { $count } words.
    }
authoring-space-deleted = Space deleted.
# $text is the word deleted.
authoring-deleted = { $text } deleted.
# $key is the terminal's own paste key.
authoring-nothing-copied = Nothing copied in { -brand } yet. Use your terminal's paste, for example { $key }.
# $parts lists what came in, from the paste-part messages.
paste-markdown = Pasted as Markdown: { $parts }
paste-part-heading =
    { $n ->
        [one] 1 heading
       *[other] { $n } headings
    }
paste-part-paragraph =
    { $n ->
        [one] 1 paragraph
       *[other] { $n } paragraphs
    }
paste-part-list =
    { $n ->
        [one] 1 list
       *[other] { $n } lists
    }
paste-part-table =
    { $n ->
        [one] 1 table
       *[other] { $n } tables
    }
paste-part-code =
    { $n ->
        [one] 1 code block
       *[other] { $n } code blocks
    }
paste-part-quote =
    { $n ->
        [one] 1 quote
       *[other] { $n } quotes
    }
paste-part-link =
    { $n ->
        [one] 1 link
       *[other] { $n } links
    }
paste-empty = Nothing to paste: the clipboard is empty.
paste-converting = Converting the formatted text to paste.
paste-failed = Could not paste: { $error } Try Paste as plain text.
authoring-verbosity =
    { $level ->
        [low] Verbosity: low.
        [high] Verbosity: high.
       *[normal] Verbosity: normal.
    }
authoring-punctuation =
    { $level ->
        [none] Punctuation: none.
        [all] Punctuation: all.
       *[some] Punctuation: some.
    }

## Dictation in edit mode (ADR-0042). Keep the meaning first: a
## 40-cell Braille display shows the start of the line. $words are the
## dictated words, $key the dictate key, $dir a folder, $error and $text
## are passed on as they are.
dictation-status = Dictating: { $words }
dictation-listening = Dictating. Speak, then press { $key } to stop.
dictation-finishing = Finishing dictation.
dictation-done = Dictation done.
dictation-busy = Dictation is finishing. Try again in a moment.
dictation-needs-edit = Dictation types in edit mode. Turn on edit mode and dictate? y or n
dictation-no-model = Dictation needs the Whisper model in { $dir }. See Dictation in the documentation.
dictation-failed = Dictation failed: { $error } See Dictation in the documentation.
dictation-no-words = No words recognized in that phrase.
dictation-lost = Dictation stopped before its last words were typed.
dictation-not-typed = Dictated words not typed, edit mode is off: { $text }
dictation-paused = Dictation paused.
dictation-resumed = Dictation resumed.
dictation-not-active = Not dictating.
dictation-cannot-pause = This dictation cannot pause; stop it instead.
section-dictation = Dictation
