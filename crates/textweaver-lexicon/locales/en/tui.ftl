### The terminal reader.

## The terminal reader's startup.

# $wanted is the speech backend asked for, $backend the one used instead.
tui-setup-backend-unavailable = Speech engine { $wanted } is not available; using { $backend }.
tui-setup-speech-failed = Speech could not start ({ $error }); running silently.
speech-engine-fallback = { $failed } could not start; { $engine } is speaking instead.
speech-engine-fallback-silent = { $failed } could not start, and no other speech engine is available; { -brand } stays silent.
tui-setup-cannot-save = Cannot save settings or positions: { $error } Reading works. Changes are lost on exit.
tui-setup-keymap-ignored = Keymap file ignored: { $error } The default keys are used. Fix the file, then restart.
# The first-run welcome. Each value names the key for an action: $play
# reads and pauses, $stop stops, $heading moves to the next heading,
# $help opens the help, $quit quits.
tui-setup-welcome = Welcome to { -brand }. { $open } opens a document. { $play } starts and pauses reading, and { $stop } stops. { $palette } lists every command. { $help } opens the help.
# The window's first-run welcome on Windows, where the menu bar is
# hidden until Alt or $menu (F10) shows it: the keys of
# tui-setup-welcome, and the menus.
gui-setup-welcome-menus = Welcome to { -brand }. { $open } opens a document. { $play } plays and pauses, { $stop } stops. { $palette } lists every command, Alt or { $menu } the menus. { $help } opens the help.
# Said at startup without a document. $open, $new, and $help name the
# keys for Open, New Document, and Help.
tui-setup-no-document = No document is open. Press { $open } to open one, { $new } for a new one, or { $help } for help.

## The terminal reader's launch.

# $name is the file asked for on the command line; $error says why.
tui-could-not-open = Could not open { $name }: { $error }

## Copying in the terminal reader.

tui-clip-system = Copied with the system clipboard, because this terminal cannot take copied text.
# $error is why: tui-clip-not-available, tui-clip-not-built, or the
# system's own message.
tui-clip-failed = Could not copy with the system clipboard: { $error }. Sent to the terminal instead.
tui-clip-not-available = the system clipboard is not available
tui-clip-not-built = this version has no system clipboard

## The terminal reader's screen.

# The title line's start; $title is the document's title or
# tui-title-no-document.
tui-title = { -brand }: { $title }
tui-title-no-document = no document
# The screen without a document. $keys names the keys for the action.
tui-empty-open = Open one: { $keys }.
tui-empty-help = Help: { $keys }.
tui-empty-quit = Quit: { $keys }.
# The key hints while a yes-or-no question waits. y, n, and a are the
# answer keys the reader takes; $escape names the Escape key.
tui-hints-confirm = y yes  n or a no  { $escape } no
# Key hint labels, each shown after its key on the bottom line.
tui-hint-play = play
tui-hint-sentence = sentence
tui-hint-faster = faster
tui-hint-slower = slower
tui-hint-close-rsvp = close RSVP
tui-hint-quit = quit
tui-hint-save = save
tui-hint-finish = finish
tui-hint-undo = undo
tui-hint-bold = bold
tui-hint-heading = heading
tui-hint-commands = commands
tui-hint-next-line = next line
tui-hint-previous-line = previous line
tui-hint-again = again
tui-hint-read-on = read on
tui-hint-leave = leave
tui-hint-find = find
tui-hint-mark = mark
tui-hint-lines = lines
tui-hint-keys = keys
tui-hint-choose = choose
tui-hint-close = close
tui-hint-back = back
# The list overlay's border: $n is the focused item's number, $count
# the number of items.
tui-list-title = { $n } of { $count }, { $title }
