### Menus, the command palette, and interface announcements.

## Menus, the command palette, interface announcements, colors, and settings

## Menu titles; the top menus mark their access key with &.

menu-file = &File
menu-edit = &Edit
menu-view = &View
menu-reading = &Reading
menu-speech = &Speech
menu-tools = &Tools
menu-help = &Help
menu-recent = Recent documents
menu-export-as = E&xport as
menu-preview = Preview
menu-settings = Settings
menu-find = Find
menu-format = Format
menu-insert = Insert
menu-proofing = Proofing
menu-citations = Citations
menu-text-size = Text size
menu-reading-aids = Reading aids
menu-rsvp = RSVP
menu-say = Say
menu-move-by = Move by
menu-headings = Headings
menu-go-to = Go to
menu-cursor = Cursor and selection
menu-bookmarks = Bookmarks and notes
menu-highlights = Highlights
menu-tables = Tables
menu-speech-cursor = Speech Cursor

## Menus, the palette, and interface announcements.

menu-bar = Menus
menu-title = { $name } menu
menu-submenu = { $name }, submenu
menu-checked = { $name }, checked
menu-not-checked = { $name }, not checked
menu-value = { $name }: { $value }
menu-with-keys = { $item }, { $keys }
menu-recent-document = { $name }, { $pct } percent
menu-recent-none = No recent documents
menu-not-available = { $name } is not available in this version.
menu-no-access-key = No item with the key { $letter }.
menu-closed = Menus closed.
menu-context = Context menu
menu-context-closed = Context menu closed.
menu-press-a-key = Press a key to hear what it does.
menu-key-described = { $name }: { $help }. Keys: { $keys }. In the menus: { $path }.
menu-key-described-no-menu = { $name }: { $help }. Keys: { $keys }.
menu-keys = { $item }. Enter or Right opens a menu or runs a command, a letter moves to its item, Left or Backspace goes back, Escape closes.
edit-line-continues = line continues
announce-level-changed = Interface announcements { $level }.
announce-level-off = off
announce-level-minimal = minimal
announce-level-normal = normal
announce-level-full = full
palette-item = { $name }, { $keys }
palette-item-no-keys = { $name }
palette-item-recent = { $name }, { $keys }, recent
palette-item-recent-no-keys = { $name }, recent
palette-list-title = Commands matching { $query }
palette-list-title-all = Commands
palette-list-intro =
    { $title }, { $n ->
        [one] 1 command
       *[other] { $n } commands
    }. Enter runs one, F1 explains it.
color-name-theme = the theme's color
color-name-blue = blue
color-name-orange = orange
color-name-navy = dark blue
color-name-skyblue = sky blue
color-name-teal = teal
color-name-gold = gold
color-name-yellow = yellow
color-name-green = green
color-name-cyan = cyan
color-name-purple = purple
color-name-pink = pink
color-name-brown = brown
color-name-gray = gray
color-name-black = black
color-name-white = white
section-colors = Colors
colors-contrast-good = good
colors-contrast-fair = fair
colors-contrast-low = low
colors-item = { $label }: { $value }, contrast { $ratio } to 1, { $verdict }
colors-contrast = Contrast { $ratio } to 1, { $verdict }.
colors-contrast-warning = Under 3 to 1 it is hard to see; choose a lighter or darker color.
# A highlight palette entry in the Colors view. $name is the entry's name.
colors-palette-label = Highlight name { $name }
# $color and $shape in words; $text and $page are ratios; $verdict good, fair or low.
colors-palette-value = { $color }, { $shape }, contrast { $text } to 1 with the text and { $page } to 1 with the page, { $verdict }
colors-palette-system = { $color }, { $shape }; the system's high contrast colors are drawn, and the shape still marks it
colors-palette-unknown = { $color }, { $shape }; a color textweaver cannot measure
colors-intro = Colors, { $n } settings. Left and Right choose a named color, Enter types a name or a #rrggbb value, Delete puts the theme's back, F1 says the help.
settings-item-recent = { $item }, recently changed
settings-reset = { $label } back to its default, { $value }.
settings-row-help = { $label }: { $value }. Default: { $default }. { $help }
number-group-separator = ,
number-decimal-separator = .
settingsio-import-question-names =
    { $n ->
        [one] Import { $n } changed setting from { $name }: { $names }? y or n
       *[other] Import { $n } changed settings from { $name }: { $names }? y or n
    }
settingsio-and-more = { $names }, and { $n } more
