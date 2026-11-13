### The textweaver app: drawn labels, hints, buttons, and dialogs.

## The window (GUI)

gui-open-title = Open a document
gui-open-documents = Documents textweaver reads
gui-open-all-files = All files
gui-open-no-dialog = The system's file chooser did not open. Type the path of the document instead.
gui-text-size = Text size { $size } points.
gui-text-size-largest = Text size { $size } points, the largest.
gui-text-size-smallest = Text size { $size } points, the smallest.
gui-font = Font: { $family }.
gui-font-list = Font

## The window (GUI): drawn labels, hints, and questions.
## Keep the letters Y and N: they are the keys that answer.

gui-yes = Yes
gui-no = No
gui-answer-delete = Delete
gui-answer-remove = Remove
gui-answer-replace = Replace
gui-question-hint = Y answers yes, N answers no, Escape answers no.
gui-find-title = Find and replace
gui-find-what = Find what
gui-find-with = Replace with
gui-find-match-case = Match case
gui-find-whole-words = Whole words
gui-find-regex = Regular expression
gui-find-across-lines = Across lines
gui-find-next = Find next
gui-find-next-help = Selects the next match.
gui-find-replace = Replace
gui-find-replace-help = Replaces the match shown and goes to the next. The first press finds a match.
gui-find-replace-all = Replace all
gui-find-replace-all-help = Says how many matches there are and asks once. One undo takes them all back.
gui-find-hint = Enter in Find what finds the next match; Enter in Replace with replaces it. Up brings back earlier text. Escape closes.
gui-find-empty = Nothing to find: type the text in Find what.
gui-button-open = Open…
gui-button-font = Font…
gui-button-edit = Edit
gui-button-finish-editing = Done editing
gui-button-settings = Settings…
gui-button-commands = Commands…
gui-button-play = Play
gui-button-pause = Pause
gui-button-stop = Stop
gui-button-previous-sentence = Previous sentence
gui-button-next-sentence = Next sentence
gui-button-slower = Slower
gui-button-faster = Faster
gui-button-start-dictation = Start dictation
gui-button-pause-dictation = Pause dictation
gui-button-resume-dictation = Resume dictation
gui-button-stop-dictation = Stop dictation
gui-button-live-pause = Pause updates
gui-button-live-resume = Resume updates
gui-button-live-newest = Jump to newest
gui-button-live-table = Table view
gui-button-live-list = List view
gui-button-live-save = Save log
gui-button-live-close = Close source
gui-button-live-show = Show changes
gui-button-live-show-on = Show changes: on
gui-button-live-show-off = Show changes: off
gui-button-live-speak = Speak changes
gui-button-live-speak-on = Speak changes: on
gui-button-live-speak-off = Speak changes: off
gui-button-unavailable = unavailable
# Button descriptions, read after the name: at most 40 cells, saying
# what the name does not. The command's full help stays in F1.
gui-hint-open = Choose a document to read.
gui-hint-font = Choose the font of the document text.
gui-hint-edit = Switch between reading and editing.
gui-hint-settings = Every option, with its help.
gui-hint-commands = Run any command by its name.
gui-hint-play = Read from the current word, or pause.
gui-hint-live-pause = New entries wait while paused.
gui-hint-live-view = Keeps your place in the log.
gui-hint-live-save = Save the log as JSON now.
gui-hint-live-close = Stop receiving; the log stays open.
gui-hint-live-field = For the field on the cursor's line.
gui-button-close = Close
gui-toolbar-reading = Reading
gui-document = Document
gui-document-titled = { $title }, document
gui-list-hint = Enter chooses, Escape closes.
gui-sidebar-contents = Contents
gui-sidebar-notes = Notes
gui-sidebar-open =
    { $n ->
        [one] { $panel } open, 1 item.
       *[other] { $panel } open, { $n } items.
    }
gui-sidebar-closed = { $panel } closed.
gui-header-shown = Header shown.
gui-header-hidden = Header hidden. Its commands keep their keys.
gui-toolbar-shown = Toolbar shown.
gui-toolbar-hidden = Toolbar hidden. Its commands keep their keys.
gui-focus-mode-on = Focus mode on. Moving the mouse shows the bars.
gui-focus-mode-off = Focus mode off.
gui-full-screen-on = Full screen on.
gui-full-screen-off = Full screen off.

## Customize buttons: the header's and the toolbar's buttons (crate::buttons).

buttons-title = Customize buttons
buttons-intro =
    { $n ->
        [one] Customize buttons, 1 button. Enter on a button to move or remove it.
       *[other] Customize buttons, { $n } buttons. Enter on a button to move or remove it.
    }
# A row: the button's name, its bar, and its place there.
buttons-row =
    { $bar ->
        [header] { $name }, header { $pos } of { $count }
       *[toolbar] { $name }, toolbar { $pos } of { $count }
    }
buttons-add-header = Add a button to the header
buttons-add-toolbar = Add a button to the toolbar
buttons-reset = Reset both bars to their defaults
buttons-move-up = Move up
buttons-move-down = Move down
buttons-remove = Remove
buttons-add-intro =
    { $n ->
        [one] 1 command to add. Enter adds it at the end.
       *[other] { $n } commands to add. Enter adds one at the end.
    }
buttons-added =
    { $bar ->
        [header] { $name } added, header { $pos } of { $count }.
       *[toolbar] { $name } added, toolbar { $pos } of { $count }.
    }
buttons-removed =
    { $bar ->
        [header] { $name } removed from the header.
       *[toolbar] { $name } removed from the toolbar.
    }
buttons-moved = { $name } moved, { $pos } of { $count }.
buttons-first = { $name } is already first.
buttons-last = { $name } is already last.
buttons-already =
    { $bar ->
        [header] { $name } is already on the header.
       *[toolbar] { $name } is already on the toolbar.
    }
buttons-no-name = That command has no short name, so it cannot be a button.
buttons-no-button = No button there.
buttons-reset-done = Buttons reset to their defaults.
buttons-none-to-add = No more commands to add.
# The preview pane beside the editor (edit mode). $key is the key that moves between the window's parts (F6).
gui-preview = Preview
gui-preview-shown = Preview shown beside the editor. { $key } moves to it.
gui-preview-hidden = Preview hidden.
gui-preview-later = Preview on. It shows beside the editor in edit mode.
gui-preview-empty = Nothing to preview yet.
gui-sidebar-no-headings = No headings.
gui-sidebar-no-notes = No notes.
gui-sidebar-current = { $item }, current
gui-sidebar-hint = Enter goes there. { $leave } goes and returns. Escape returns.
gui-sidebar-hint-notes = Enter goes there. Space shows its links. { $leave } goes and returns. Escape returns.
gui-sidebar-notes-keys = Space shows a note's links.
gui-settings-sections = Sections
gui-settings-form = { $section } settings
gui-settings-saved-hint = Changes take effect and are saved at once.
gui-settings-close-help = Close the settings. Every change is already saved.
gui-settings-recent = Recently changed
gui-settings-matching = Matching { $filter }
gui-settings-table = { $label } is a table. Edit it in settings.toml.
gui-setting-new-value = New value for { $label }
gui-setting-value-hint = Press Enter to accept, or Escape to go back.
gui-prompt-path-hint = Type the path of a document, then press Enter. Tab completes it; Up and Down recall earlier ones.
gui-prompt-hint = Press Enter to accept, or Escape to cancel. Up and Down recall earlier answers.
gui-palette-filter = Type to filter the commands
gui-palette-list = Commands
gui-palette-hint = Enter runs the first match; Tab moves to the list; F1 explains a command.
gui-open-failed = Could not open { $name }: { $error }
gui-uia-unavailable = UI Automation notifications exist only on Windows; using the live region.
gui-graphics-failed = Could not start the { -brand } app: its graphics failed. The terminal reader, tw, needs none.
gui-crashed = textweaver stopped after an internal error.
gui-crashed-saved = Place saved. Unsaved edits are offered for recovery next time.
gui-rsvp = RSVP
gui-rsvp-playing = RSVP playing, word { $n } of { $total }
gui-rsvp-paused = RSVP paused, word { $n } of { $total }
gui-rsvp-finished = RSVP finished, word { $n } of { $total }
gui-settings-section-item =
    { $section }, { $n ->
        [one] 1 setting
       *[other] { $n } settings
    }
gui-palette-count =
    { $n ->
        [0] No commands match.
        [one] 1 command.
       *[other] { $n } commands.
    }
gui-settings-form-help = Up and Down move between settings. Left and Right change one. Enter types a new value. Delete puts the default back. { $next } and { $previous } change the section. Type to filter. F1 says the help.
gui-settings-press-enter = Press Enter to type a new value for { $label }.
gui-font-built-in = { $family } (built in)

## The window's menus and dialogs. Settings files chosen with the
## system's file chooser, the Colors dialog, and the font list. $ratio is
## a contrast ratio such as 4.8; $verdict is good, fair, or low.
gui-settings-files = Settings files
gui-settings-export-title = Export settings
gui-settings-import-title = Import settings
gui-chooser-no-dialog = The system's file chooser did not open. Type the path of the file instead.
gui-colors-value = { $value }, contrast { $ratio } to 1, { $verdict }
gui-colors-help = Left and Right choose a named color, blue and orange first. Enter types a name or a #rrggbb value. Delete puts the theme's color back. Every mark keeps its underline, weight, or symbol, whatever its color.
gui-colors-value-system = { $value }; not drawn while the system's high contrast colors are on
gui-colors-help-system = The system's high contrast colors are on, so they are drawn instead of these colors, which apply again when it is off. Each highlight name keeps its own shape.
gui-colors-reset-all = Reset all colors
gui-colors-reset-all-help = Put the theme's own color back for every part.
gui-colors-reset-done = Every color is the theme's again.
# Asked before Reset all colors; a yes resets them.
colors-reset-question = Reset every color to the theme's? y or n
gui-colors-closed = Colors closed.
gui-font-list-intro =
    { $n ->
        [one] { $title }, 1 family.
       *[other] { $title }, { $n } families.
    }

## W9b-x: the reading settings (View, Reading settings): the settings a
## reader changes most, in one form, with the two spacing presets. $n is a
## number of settings.
reading-form-intro = Reading settings, { $n } settings. Left and Right change a value, Enter types one, Delete puts the default back, F1 says the help.
reading-form-spacing-wcag-done = Spacing set to the WCAG values.
reading-form-spacing-generous-done = Spacing set to Generous, wider than WCAG.
# Said after a spacing change leaves letters spaced wider than words.
spacing-letter-without-word = Raise word spacing with letter spacing.
gui-reading-form-help = Up and Down move, Left and Right change a value, Enter types one, F1 says the help.
gui-reading-voices = Voices
gui-reading-voices-help = Open the voice manager.
gui-reading-wcag = WCAG spacing
gui-reading-wcag-help = Set the four spacings to the WCAG values.
gui-reading-generous = Generous spacing
gui-reading-generous-help = Set the four spacings wider than WCAG.
gui-reading-closed = Reading settings closed.
## End of W9b-x
