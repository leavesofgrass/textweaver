### The help, key names, About, and the packaged guides.

## Keys named in messages and the help.

help-the-command-palette = the command palette
help-not-bound = not bound
# Two keys, or a key and a list of keys: "p or Ctrl+P".
help-or = { $a } or { $b }
# A command without keys: $name is its palette name, such as list highlights.
help-the-command = the command { $name }
# One line of the keyboard shortcuts list: the command's name-* (first, so
# type-ahead finds commands), its keys (inside a 40-cell Braille line), an
# action-* help, and its category-* title.
help-entry = { $name }: { $keys }. { $help }. { $category }
help-unknown-command = Unknown command: { $text }.
help-shortcuts-intro = Keyboard shortcuts, { $n } commands. Type to filter. Page Down moves to the next group, F1 explains a command, Enter runs it, Escape closes.
help-shortcuts-title = Keyboard shortcuts
# The keyboard shortcuts list, filtered: $filter is what was typed.
help-shortcuts-title-matching = Keyboard shortcuts matching { $filter }
# $n commands of $total match the filter.
help-shortcuts-filter-match = { $n } of { $total } commands match.
help-shortcuts-filter-none = No commands match { $query }. Backspace removes letters.
help-shortcuts-filter-cleared = Filter cleared, { $n } commands.
# Moving into a group of the keyboard shortcuts list: its name, its
# size, then the row ($item, with its place in the list).
help-shortcuts-group-item =
    { $group }, { $n ->
        [one] 1 command
       *[other] { $n } commands
    }. { $item }
help-title = Help
help-intro = Help. Type to search all of help; Up and Down move, Escape closes.

## The help list. Each value is a key or keys from the keymap.

help-about = textweaver reads documents aloud. Keys below are the current bindings.
help-search = Search all of help by typing here: commands, keys, settings, and the guides.
help-open = Open a document: { $open }. Library and recent files: { $library }.
help-play = Play or pause: { $key }.
help-read-from-cursor = Read from the cursor: { $key }.
help-stop = Stop: { $key }.
help-sentences = Next and previous sentence: { $next } and { $previous }.
help-paragraphs = Next and previous paragraph: { $next } and { $previous }.
help-headings = Next and previous heading: { $next } and { $previous }. Heading at a level: { $first } to { $last }, with Shift for the previous one.
help-read-headings = Read from the next and previous heading: { $next } and { $previous }.
help-quick-keys = Quick keys, as in NVDA and JAWS: list { $list }, list item { $item }, table { $table }, link { $link }, block quote { $quote }, separator { $separator }, graphic { $graphic }, section { $section }. Shift with the key goes to the previous one.
help-speech-cursor = Speech Cursor, line by line: { $key }.
help-find = Find: { $key }.
help-bookmark = Add a bookmark: { $key }.
help-history = Back and forward through your jumps: { $back } and { $forward }.
help-rate = Faster and slower: { $faster } and { $slower }.
help-where = Where am I: { $key }.
help-repeat = Hear the last message again: { $repeat }. The last message and the status: mode, rate, engine, and position: { $status }.
help-notes = Notes: add { $add }, list { $list }, next and previous { $next } and { $previous }, delete the one at the cursor { $delete }. In the list, Delete deletes and F2 edits.
help-highlights = Highlight the selection or sentence, or remove a highlight: { $highlight }. With a name: { $first } to { $fifth }, or the list: { $named }. List highlights: { $list }.
help-bookmarks-list = Bookmarks list: Delete deletes a bookmark, F2 renames it.
help-edit = Edit the document: { $edit }. Save: { $save }. Save as: { $saveas }. New document: { $new }.
help-editing = While editing: undo { $undo }, redo { $redo }, bold { $bold }. Every formatting command is in the keyboard shortcuts.
help-outline = Outline of the headings, type to filter: { $outline }. Follow a link or footnote: { $follow }.
help-tables = Tables: { $nextrow } and { $previousrow } move by row, { $nextcell } and { $previouscell } by cell.
help-citations = Citations while editing: insert { $insert }, add a reference by DOI, ISBN or PMID { $reference }. Spelling: next and previous misspelling { $next } and { $previous }, suggestions { $suggestions }.
help-export = Export to HTML, PDF, Word, EPUB, or braille, preview in the browser, and start from a template: type export, preview, or template in the command palette.
help-verbosity = How much is said: { $verbosity }. How much punctuation: { $punctuation }.
help-voice = Choose a voice: { $voice }. Restart speech if it stops: { $restart }.
help-access = With a screen reader, who speaks: { $key } cycles self-voicing, hybrid, and screen reader mode.
help-access-window = Who reads: { $key } switches between textweaver reads aloud and my screen reader reads.
help-character-keys = Single-key shortcuts on or off, for dictation: { $keys }. Settings: { $settings }.
help-all-shortcuts = All keyboard shortcuts: { $key }.
help-palette = Run any command by name: { $key }.
# Keep the letters y, n, and a: they are the keys that answer.
help-quit = Quit, saving your place: { $key }, then y to confirm; n, a, or Escape cancels.

## Help categories.

category-reading = Reading
category-navigation = Navigation
category-speech-cursor = Speech Cursor
category-voice = Voice
category-search = Search
category-bookmarks = Bookmarks and notes
category-file = File
category-editing = Editing
category-view = View and help

## Key names as textweaver's own voice says them. Written names (Ctrl+S)
## are not translated.

keyname-control = Control
keyname-command = Command
keyname-alt = Alt
keyname-shift = Shift
keyname-period = period
keyname-comma = comma
keyname-semicolon = semicolon
keyname-colon = colon
keyname-apostrophe = apostrophe
keyname-quote = quote
keyname-grave-accent = grave accent
keyname-tilde = tilde
keyname-exclamation-mark = exclamation mark
keyname-question-mark = question mark
keyname-at-sign = at sign
keyname-number-sign = number sign
keyname-dollar-sign = dollar sign
keyname-percent = percent
keyname-caret = caret
keyname-ampersand = ampersand
keyname-asterisk = asterisk
keyname-left-parenthesis = left parenthesis
keyname-right-parenthesis = right parenthesis
keyname-left-bracket = left bracket
keyname-right-bracket = right bracket
keyname-left-brace = left brace
keyname-right-brace = right brace
keyname-less-than = less than
keyname-greater-than = greater than
keyname-plus = plus
keyname-minus = minus
keyname-equals = equals
keyname-underscore = underscore
keyname-slash = slash
keyname-backslash = backslash
keyname-vertical-bar = vertical bar
keyname-page-up = Page Up
keyname-page-down = Page Down
keyname-up-arrow = Up Arrow
keyname-down-arrow = Down Arrow
keyname-left-arrow = Left Arrow
keyname-right-arrow = Right Arrow
keyname-escape = Escape
keyname-space = Space
keyname-enter = Enter
keyname-tab = Tab
keyname-backspace = Backspace
keyname-delete = Delete
keyname-insert = Insert
keyname-home = Home
keyname-end = End

## Help's ways to the docs, About's facts, and first-run choices asked
## again. $address is a web address; $path a folder; facts start with
## their name so each Braille line leads with it.
about-title = About textweaver
about-intro = About textweaver, { $n } facts. Include them in a problem report.
about-version = Version: textweaver { $version }
about-build = Build: { $frontend }, { $profile }, { $os } { $arch }
about-frontend-window = window
about-frontend-terminal = terminal reader
about-profile-release = release
about-profile-debug = debug
about-license = License: { $license }
about-engine-in-use = Speech engine in use: { $engine }
about-engines = Speech engines found: { $engines }
about-engines-none = Speech engines found: none
about-components = Components: { $installed } of { $known } installed
about-folder-settings = Settings folder: { $path }
about-folder-data = Data folder: { $path }
about-folder-cache = Cache folder: { $path }
about-no-folders = Folders: none; this session keeps no files.
about-quick-start-online = Quick start not found beside textweaver. Open { $address } in your browser? y or n
about-docs-question = Documentation: { $address }. Open it in your browser? y or n
about-report-question = Report a problem: { $address }. Nothing is sent. Open it in your browser? y or n
about-first-run-again = First-run choices reset; asked at the next start.

## B1-hp: the packaged guides and Search help. $n counts topics or
## guides; $guides names guide files; $row is a command's row ("Find
## next, F3"). Each row starts with the topic, so a Braille line does.
docs-not-found = Documentation not found beside textweaver. Open { $address } in your browser? y or n
docs-read-only = This guide comes with textweaver and is read only. To change it, copy the file elsewhere and open the copy.
docs-guide-missing =
    { $n ->
        [one] Guide missing: { $guides }. The other guides open.
       *[other] { $n } guides missing: { $guides }. The other guides open.
    }
helpsearch-title = Search help
helpsearch-title-matching = Help matching { $filter }
helpsearch-intro = Search help, { $n } topics. Type to search commands, keys, settings, and the guides; Enter opens a topic, F1 explains it.
helpsearch-match =
    { $n ->
        [one] 1 match.
       *[other] { $n } matches.
    }
helpsearch-none = No help matches { $query }. Backspace removes letters.
helpsearch-cleared =
    { $n ->
        [one] Filter cleared, 1 topic.
       *[other] Filter cleared, { $n } topics.
    }
helpsearch-command = { $row }, command
helpsearch-setting = { $label }, setting in { $section }
helpsearch-guide = { $heading }, in { $guide }
helpsearch-at-heading = At { $heading }.
## End of B1-hp
