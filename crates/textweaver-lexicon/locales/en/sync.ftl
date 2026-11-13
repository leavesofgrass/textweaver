### Sync between computers.

## Sync wave, S4: sync in the reader (ADR-0049). $device is a computer's
## name as the owner gave it ("laptop", "lab", "Computer 2"); $title a
## document's title; $pct a percentage; $n a count. Meaning first, and
## within the 40 cells of a Braille line where it can be.
sync-status-off = Sync: off
sync-status-not-set-up = Sync: not set up
sync-status-starting = Sync: starting
sync-status-folder-missing = Sync: folder missing, saving here
sync-status-read-only = Sync: newer format, read only
sync-status-failed = Sync: cannot use the folder
sync-status-cannot-write = Sync: cannot write, saving here
sync-status-clock-ahead = Sync: { $device }'s clock is ahead
sync-status-damaged =
    { $n ->
        [one] Sync: 1 damaged file skipped
       *[other] Sync: { $n } damaged files skipped
    }
sync-status-up-to-date = Sync: up to date
sync-status-this-computer = This computer: { $name }.
sync-status-no-others = No other computers yet.
sync-status-others = Other computers: { $names }.
sync-status-error = Problem: { $error } Check the sync folder.
sync-another-computer = another computer
sync-untitled = a document
sync-damaged = Sync: damaged file from { $device } skipped.
sync-newer-file = Sync: newer file from { $device } skipped.
sync-read-only = Sync: newer format, read only.
sync-clock-ahead = Sync: { $device }'s clock is { $hours } hours ahead.
sync-fresh-id = Sync: copied setup; new computer id.
sync-write-failed = Sync: cannot write. { $error } Saving on this computer for now.
sync-name-refused = Name not allowed. Try one like laptop.
sync-note-replaced =
    { $n ->
        [one] { $title }: a note was replaced by { $device }'s newer edit.
       *[other] { $title }: { $n } notes were replaced by { $device }'s newer edits.
    }
sync-restored-notes =
    { $n ->
        [one] { $title }: a deleted note came back, edited on { $device }.
       *[other] { $title }: { $n } deleted notes came back, edited on { $device }.
    }
sync-restored-bookmarks =
    { $n ->
        [one] { $title }: a deleted bookmark came back, edited on { $device }.
       *[other] { $title }: { $n } deleted bookmarks came back, edited on { $device }.
    }
sync-restored-highlights =
    { $n ->
        [one] { $title }: a deleted highlight came back, edited on { $device }.
       *[other] { $title }: { $n } deleted highlights came back, edited on { $device }.
    }
sync-arrived =
    { $n ->
        [one] { $title }: 1 change from { $device }.
       *[other] { $title }: { $n } changes from { $device }.
    }
sync-resumed = { $title }: resumed at { $pct } percent, from { $device }.
sync-place-arrived = { $device }'s place: { $pct } percent.
sync-place-question = { $device } at { $pct } percent. Go there? y or n
sync-suggestion-question =
    { $n ->
        [one] This may be { $title } from { $device }, with 1 note. Use the note from { $device }? y or n
       *[other] This may be { $title } from { $device }, with { $n } notes. Use the notes from { $device }? y or n
    }
sync-went-to-place = { $device }'s place, { $pct } percent.
sync-kept-place = Kept this place.
sync-suggestion-accepted = Using the notes from { $device }.
sync-suggestion-declined = Kept separate.
sync-sidecar-differed =
    { $n ->
        [one] Sync: 1 library place differed.
       *[other] Sync: { $n } library places differed.
    }
sync-sidecar-failed = Sync: cannot write a library place. { $error } Check that the library folder can be written to.
sync-no-state = Sync is off in this run: nothing is saved.
sync-choose-folder = Choose the sync folder
sync-group-places = Places
sync-group-notes = Notes
sync-group-highlights = Highlights
sync-group-bookmarks = Bookmarks
sync-group-statistics = Statistics
sync-group-item = { $name }: { $state }
sync-start = Start syncing
sync-groups-title = What syncs
sync-groups-intro = { $title }, as { $name }. Enter turns one on or off; Start syncing ends.
sync-started = Sync on, as { $name }. Sync status: { $key }.
sync-how-to-set-up = To set it up: Tools, Sync, Set up sync.
sync-now-started = Syncing.
sync-now-done =
    { $n ->
        [one] Sync: up to date, 1 document checked.
       *[other] Sync: up to date, { $n } documents checked.
    }
sync-now-changed =
    { $n ->
        [one] Sync: 1 document took changes.
       *[other] Sync: { $n } documents took changes.
    }
sync-no-places = No other computer has a place here.
sync-place-item = { $device }, { $pct } percent
sync-places-title =
    { $n ->
        [one] 1 other place
       *[other] { $n } other places
    }
sync-no-replaced = No replaced notes in this document.
sync-replaced-item = { $text }, replaced by { $device }
sync-replaced-item-deleted = { $text }, deleted by { $device }
sync-replaced-title =
    { $n ->
        [one] 1 replaced note
       *[other] { $n } replaced notes
    }
sync-replaced-intro = { $title }. Enter puts one back.
sync-note-restored = Note put back: { $text }
sync-already-off = Sync is already off here.
sync-stopped = Sync off here. The folder is left as it is.
prompt-sync-computer-name = Name this computer, Enter keeps it
menu-sync = Sync
menu-cards = Study cards
menu-cw-practice = Morse code practice
section-sync = Sync

## End of S4

## Sync wave, S5: settings and word lists (ADR-0049). $n is a count,
## $device a computer's name as the owner gave it, $computers a count of
## computers, $voice a voice's id. Meaning first, within 40 cells where it
## can be.
sync-settings-arrived =
    { $n ->
        [one] Settings: 1 change from { $device }.
       *[other] Settings: { $n } changes from { $device }.
    }
sync-settings-arrived-several = Settings: { $n } changes from { $computers } computers.
sync-kept-keys-mac =
    { $n ->
        [one] Mac key overrides: 1, kept, not used here.
       *[other] Mac key overrides: { $n }, kept, not used here.
    }
sync-kept-keys-pc =
    { $n ->
        [one] Windows and Linux key overrides: 1, kept, not used here.
       *[other] Windows and Linux key overrides: { $n }, kept, not used here.
    }
sync-group-settings = Settings
sync-group-profiles = Profiles
sync-group-key-overrides = Key overrides
sync-group-words = Word list
sync-group-glossary = Glossary and pronunciations
sync-group-favorite-voices = Favorite voices
voices-missing-row = { $voice }, favorite, not on this computer
voices-missing = { $voice } is not on this computer. Space takes it off your favorites.
gui-voices-list = Voices
gui-voices-use = Use voice
gui-voices-use-help = Use the focused voice and hear a sample, or download it after a question.
gui-voices-preview = Preview
gui-voices-preview-help = Hear a sample in the focused voice without choosing it.
gui-voices-favorite = Favorite
gui-voices-favorite-help = Make the focused voice a favorite, or stop it being one. Favorites come first.
gui-voices-remove = Remove
gui-voices-remove-help = Remove the focused downloaded Piper voice, after a question.
gui-voices-remove-unavailable = unavailable
gui-voices-language-help = Show only the next language's voices, then all languages again.
gui-voices-engine-help = Show only the next engine's voices, then all engines again.
gui-voices-fetch-help = Download the list of Piper voices, about 250 KB, after a question.
gui-voices-close-help = Close the voice manager.
gui-voices-hint = Enter uses the voice, Space marks a favorite, Escape closes.

## End of S5
