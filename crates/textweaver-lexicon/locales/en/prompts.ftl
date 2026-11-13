### Prompts, questions, lists, and file choosers.

## Prompts.

prompt-define-word = Define which word?
prompt-profile-name = Name for the new profile
prompt-profile-rename = New name for the profile, Enter keeps it
prompt-profiles-import = Import profiles from file
prompt-profiles-export = Export profiles to file, for example textweaver-profiles.json

## Questions and answers. Keep the letters y and n: they are the keys
## that answer.

common-press-y-or-n = Press y or n.
common-kept = Kept.
confirm-quit = Quit textweaver? y or n
confirm-delete-note = Delete this note or highlight? y or n
notes-remove-highlight-question = Remove this highlight? y or n
notes-delete-note-question = Delete this note? y or n
list-nothing-to-mark = Nothing to mark in this list.
notes-editing = Editing note: { $text }
# $key opens a document.
app-no-document-open = No document is open. Press { $key } to open one.
app-window-only = This command works in the textweaver app.
app-terminal-only = This command works in the terminal reader.
settings-save-failed = Could not save settings: { $error } Your changes stay in use until you quit.
settings-outside-kept = Settings changed outside textweaver were kept.
edit-still-editing = Still editing.
# The reading view inside edit mode (toggle_preview). $key is its key;
# $line the line the caret is back on.
edit-preview-on = Preview, read-only: the reading view at the same place. { $key } goes back to editing.
edit-preview-off = Back to editing. { $line }
edit-preview-read-only = Preview is read-only. { $key } goes back to editing.
edit-preview-not-editing = Preview works in edit mode. { $key } starts editing.
edit-preview-plain = Plain text has no formatting to preview; it reads the same.
edit-preview-failed = Could not show the preview: { $error } Check the Markdown, then try again.
goto-not-a-target = Not a go-to target: { $text }. Type a line number, a percentage such as 50%, start, or end.

## Prompts: the label is shown and said when the prompt opens.

prompt-find = Find
prompt-go-to = Go to line, percent, start, or end
prompt-open = Open file
prompt-command = Command
# $label is prompt-command.
prompt-command-palette-intro = { $label }. Type part of a name; Tab completes, Up and Down list matches.
prompt-save-as = Save as
prompt-export-as = Export as
prompt-table-size = Table size, columns by rows, for example 3 by 2
prompt-image-path = Image file
prompt-replace-find = Replace, find what
prompt-replace-with = Replace with
prompt-note = Note
prompt-edit-note = Edit note, Enter keeps it
prompt-rename-bookmark = New bookmark name, Enter keeps it
prompt-export-settings = Export settings to file, for example textweaver-settings.json
prompt-import-settings = Import settings from file
prompt-citation-locator = Page or other locator, for example 12 or chapter 2; Enter for none
prompt-reference-identifier = DOI or ISBN to add
prompt-import-references = Import references from file
prompt-template-title = Title of the new document
prompt-setting-value = New value, Enter keeps it

## Tab completion of file paths in prompts.

# $folder is the folder's full path.
pathc-no-folder = There is no folder { $folder }.
# $prefix is what was typed after the last separator.
pathc-no-match = No file or folder starts with { $prefix }.
# The one name that matched; $kind is folder or file.
pathc-one =
    { $kind ->
        [folder] { $name }, folder
       *[file] { $name }, file
    }
# $n names matched; $names are the first few, joined with commas; $more is
# yes when more matched than are read out.
pathc-many =
    { $more ->
        [yes] { $n } matches: { $names }, and more.
       *[no] { $n } matches: { $names }.
    }

## Lists and prompts shared by every frontend.

# The focused list item: $item is its text, $k its place, $n the number of items.
listmodel-item-position = { $k } of { $n }, { $item }
# $letter is the letter or digit typed.
listmodel-no-item-starts = No item starts with { $letter }.
listmodel-top-of-list = Top of list.
listmodel-end-of-list = End of list.
listmodel-no-matching-commands = No matching commands.
listmodel-no-earlier-entries = No earlier entries.
# Tab in the command palette: $n commands match (always more than one); $names lists the first few, joined by commas.
listmodel-command-matches = { $n } matches: { $names }.

## Words typed at the go-to prompt, besides a number: they must be the
## words prompt-go-to and goto-not-a-target tell the user to type.

goto-word-start = start
goto-word-end = end

language-voices-loading = The voice list is still loading, so the current voice keeps speaking.

## File and folder choosers. The browse key fills a prompt for a
## path from the file browser; the GUI names its choosers and filters.
prompt-browse-hint = { $label }. { $key } to browse.
prompt-browse-file = Choose the file: { $label }
prompt-browse-folder = Choose the folder: { $label }
prompt-browse-filled = { $name } chosen. Enter confirms.
chooser-type-files = { $type } files
chooser-image-title = Insert an image
chooser-images = Images
chooser-references-title = Import references
chooser-reference-files = Reference files
chooser-anki-title = Import Anki cards
chooser-anki-files = Anki packages
chooser-profiles-import-title = Import profiles
chooser-profiles-export-title = Export profiles
chooser-profile-files = Profile files
chooser-live-source-files = Live source files
gui-folder-no-dialog = The system's folder chooser did not open. Choose the folder in this list.
gui-prompt-browse-hint = { $key } opens the file browser.

## A secret field's Show choice: the show key, or the Show box in
## the textweaver app, shows a password, token or join code as typed.
prompt-show-hint = { $label }. { $key } shows it.
prompt-secret-hidden = { $count ->
        [one] 1 character, hidden
       *[other] { $count } characters, hidden
    }
prompt-show-join-code = Show the join code
prompt-showing-join-code = Showing the join code
prompt-hiding-join-code = Hiding the join code
prompt-show-password = Show the password
prompt-showing-password = Showing the password
prompt-hiding-password = Hiding the password
prompt-show-token = Show the token
prompt-showing-token = Showing the token
prompt-hiding-token = Hiding the token
gui-prompt-show-hint = { $key } or the Show box shows what is typed.
