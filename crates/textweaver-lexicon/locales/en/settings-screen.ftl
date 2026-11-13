### The settings screen, profiles, units, sections, and settings files.

## Settings profiles.

profiles-title =
    { $n ->
        [0] Settings profiles, none saved yet
        [one] Settings profiles, 1 profile
       *[other] Settings profiles, { $n } profiles
    }
# $title is profiles-title.
profiles-intro = { $title }. Enter switches to a profile, F2 renames it, Delete deletes it.
# $summary is profile-summary-* parts joined by commas.
profiles-item = { $name }: { $summary }
profiles-item-active = { $name }, in use: { $summary }
profiles-save-new = Save the current settings as a new profile
profiles-update = Save the current settings into { $name }
profiles-import = Import profiles from a file
profiles-export = Export all profiles to a file
profile-summary-voice = voice { $voice }
profile-summary-rate = rate { $rate }
profile-summary-theme = theme { $theme }
# $mode is self voicing, hybrid, or screen reader.
profile-summary-access = { $mode } mode
profile-summary-empty = nothing saved
profile-switched = Switched to { $name }.
profile-switched-backend = Switched to { $name }. Its speech engine is used from the next start.
# $keys lists settings such as speech.pitch.
profile-dropped =
    { $n ->
        [one] 1 setting in it is not used by this version: { $keys }.
       *[other] { $n } settings in it are not used by this version: { $keys }.
    }
profile-saved = Saved the current settings as { $name }.
profile-replaced = Saved the current settings into { $name }.
profile-renamed = Renamed { $old } to { $new }.
profile-delete-question = Delete the profile { $name }? y or n
profile-deleted = Deleted { $name }.
profile-kept = Kept.
profile-not-found = There is no profile named { $name }.
profile-needs-name = A profile needs a name.
profile-exists = A profile named { $name } already exists.
profiles-not-an-export = { $detail }
profiles-no-persistence = Profiles are not saved in this session.
profiles-read-failed = The profiles file could not be read, so it is treated as empty: { $error } Saving a profile replaces the file.
profiles-save-failed = Could not save the profiles: { $error } Check that the settings folder can be written to.
profiles-none-to-export = There are no profiles to export yet.
profiles-exported =
    { $n ->
        [one] Exported 1 profile to { $file }.
       *[other] Exported { $n } profiles to { $file }.
    }
profiles-export-failed = Could not export the profiles: { $error } Check that the folder can be written to.
profiles-imported =
    { $n ->
        [0] There were no profiles in { $file }.
        [one] Imported 1 profile from { $file }: { $names }.
       *[other] Imported { $n } profiles from { $file }: { $names }.
    }

## Exporting and importing settings.

# $n settings differ; $name is the file's name.
settingsio-import-question =
    { $n ->
        [one] Import { $n } changed setting from { $name }? y or n
       *[other] Import { $n } changed settings from { $name }? y or n
    }
settingsio-no-persistence = Settings are not saved in this session, so they cannot be exported or imported.
# $path is the file written.
settingsio-exported = Settings exported to { $path }.
settingsio-export-failed = Could not export settings: { $error } Check that the folder can be written to.
# $path is the file; $error the system's reason.
settingsio-read-failed = Could not read { $path }: { $error } Check the file, then import again.
settingsio-nothing-to-import = Nothing to import: your settings already match that file.
settingsio-cancelled-unchanged = Canceled. Nothing was changed.
settingsio-import-failed = Could not import settings: { $error } Check that the settings folder can be written to.
# $summary lists what changed (from the settings store, in English).
settingsio-imported = Settings imported. { $summary }
settingsio-backend-next-start = The new speech engine is used from the next start.
settingsio-backed-up = The old settings were backed up.

## The settings screen. $label is a setting-* label, $value its value as
## described below.

settings-not-set = not set
settings-none = none
settings-empty = empty
# A number and its unit (a settings-unit-* message): "300 words per minute".
settings-number-unit = { $n } { $unit }
settings-entries =
    { $n ->
        [0] none
        [one] 1 entry
       *[other] { $n } entries
    }
settings-type-on-or-off = Type on or off.
settings-type-a-number = Type a number from { $min } to { $max }.
settings-outside = { $n } is outside { $min } to { $max }.
# $names are the choices, joined with commas.
settings-choose-one-of = Choose one of: { $names }.
settings-edit-table = Edit { $label } in settings.toml; it holds names and values.
# $path is a key such as speech.rate, not translated.
settings-no-such-setting = There is no setting { $path }.
settings-cannot-be = { $label } cannot be that: { $error } Choose another value.
settings-changed = { $label }, { $value }.
settings-clamped = Out of range, so the nearest value is used.
settings-restart-speech = Restart speech to use it.
settings-next-start = Used from the next start.
settings-intro = Settings, { $n } settings. Type to filter. Left and Right change a value, Enter changes or types one, Delete puts the default back, Escape closes.
settings-item = { $label }: { $value }
settings-title = Settings
settings-title-matching = Settings matching { $filter }
settings-closed = Settings closed.
settings-filter-cleared =
    { $n ->
        [one] Filter cleared, 1 setting.
       *[other] Filter cleared, { $n } settings.
    }
settings-filter-none = No settings match { $query }. Backspace removes letters.
settings-filter-match =
    { $n ->
        [one] 1 setting match.
       *[other] { $n } settings match.
    }
settings-largest = Largest value, { $value }.
settings-smallest = Smallest value, { $value }.
settings-press-enter = { $label }: press Enter to type a new value.
settings-table-item = { $label }: { $value }. Edit it in settings.toml.
# $help is the setting's help (setting-*-help), which may be empty.
settings-editing = { $label }, now { $value }. { $help }

## Units, said after a number.

settings-unit-words-per-minute =
    { $n ->
        [one] word per minute
       *[other] words per minute
    }
settings-unit-hertz =
    { $n ->
        [one] hertz
       *[other] hertz
    }
settings-unit-percent = percent
settings-unit-semitones =
    { $n ->
        [one] semitone
       *[other] semitones
    }
settings-unit-milliseconds =
    { $n ->
        [one] millisecond
       *[other] milliseconds
    }
settings-unit-words =
    { $n ->
        [one] word
       *[other] words
    }
settings-unit-places =
    { $n ->
        [one] place
       *[other] places
    }
settings-unit-columns =
    { $n ->
        [one] column
       *[other] columns
    }
settings-unit-groups =
    { $n ->
        [one] group
       *[other] groups
    }
settings-unit-characters =
    { $n ->
        [one] character
       *[other] characters
    }
settings-unit-lines =
    { $n ->
        [one] line
       *[other] lines
    }
settings-unit-seconds =
    { $n ->
        [one] second
       *[other] seconds
    }
settings-unit-steps =
    { $n ->
        [one] step
       *[other] steps
    }
settings-unit-megabytes =
    { $n ->
        [one] megabyte
       *[other] megabytes
    }
settings-unit-files =
    { $n ->
        [one] file
       *[other] files
    }
settings-unit-letters =
    { $n ->
        [one] letter
       *[other] letters
    }
settings-unit-points =
    { $n ->
        [one] point
       *[other] points
    }
settings-unit-rows =
    { $n ->
        [one] row
       *[other] rows
    }
settings-unit-minutes =
    { $n ->
        [one] minute
       *[other] minutes
    }

## Settings sections.

section-highlight = Highlight
section-reading = Reading
section-display = Display
section-editing = Editing
section-library = Library and study tools
section-keyboard = Keyboard and language
section-accessibility = Accessibility and announcements
section-export = Export
section-braille = Braille

## Settings by task (beta 4): sections, and when a setting applies.

section-voice = Voice and speech
section-pauses = Pauses and verbosity
section-speaking-text = Speaking text
section-documents = Documents
section-math = Math
section-morse = Morse code
section-morse-practice = Morse practice
section-spacing = Spacing, font and ruler
section-word-aids = Word aids
section-rsvp = Word by word (RSVP)
section-window = Toolbars and panels
# $n is the number of sections.
settings-intro-sections =
    { $n ->
        [one] Settings, 1 section. Enter opens a section. Type to search every setting. Escape closes.
       *[other] Settings, { $n } sections. Enter opens a section. Type to search every setting. Escape closes.
    }
# $section is a section-* title, $n its number of settings.
settings-section-row =
    { $n ->
        [one] { $section }, 1 setting
       *[other] { $section }, { $n } settings
    }
settings-all = All settings
settings-all-row =
    { $n ->
        [one] All settings, 1 setting
       *[other] All settings, { $n } settings
    }
settings-title-section = Settings: { $section }
settings-section-opened =
    { $n ->
        [one] { $section }, 1 setting. Left and Right change a value, Enter changes or types one, Delete puts the default back, Escape goes back to the sections.
       *[other] { $section }, { $n } settings. Left and Right change a value, Enter changes or types one, Delete puts the default back, Escape goes back to the sections.
    }
settings-all-opened =
    { $n ->
        [one] All settings, 1 setting, including those that do not apply now. Escape goes back to the sections.
       *[other] All settings, { $n } settings, including those that do not apply now. Escape goes back to the sections.
    }
settings-back = Settings sections.
settings-section-now =
    { $n ->
        [one] { $section } now lists 1 setting.
       *[other] { $section } now lists { $n } settings.
    }
settings-section-help =
    { $n ->
        [one] { $section }, 1 setting. Enter opens it.
       *[other] { $section }, { $n } settings. Enter opens it.
    }
# $item is a settings-item row, $why a settings-applies-* sentence.
settings-item-not-now = { $item }. { $why }
# $engine, $first and $second are speech engine names, such as Eloquence.
settings-applies-engine = Applies when { $engine } is the speech engine.
settings-applies-engines = Applies when { $first } or { $second } is the speech engine.
settings-applies-not-engine = Applies when { $engine } is not the speech engine.
# $os is Windows, macOS or Linux.
settings-applies-os = Applies on { $os } only.
