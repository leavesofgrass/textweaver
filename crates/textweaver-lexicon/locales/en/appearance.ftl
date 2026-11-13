### Themes, accessibility modes, and the Lexend font.

## Themes.

# Put in front of every error message, so it reads as an error without color; $message is the error.
message-error = Error: { $message }
# $name is the theme name in the settings; $used the display name of the theme used instead.
themes-unknown = There is no theme called { $name }; using { $used }.
# $theme is the new theme's display name.
themes-next = Theme { $theme }.
themes-soft-dark-name = { $theme }, soft dark
themes-choice-aa = { $theme }, meets AA
themes-choice-below-aa = { $theme }, below AA
# Said after the theme name when the theme falls short of WCAG AA; $count is the number of contrast checks that fail.
themes-below-aa =
    { $count ->
        [one] Below AA: 1 check falls short.
       *[other] Below AA: { $count } checks fall short.
    }

## Accessibility modes and the first-run step for a screen reader.

# Said when the accessibility mode changes; $mode is the new mode's id.
access-mode-changed =
    { $mode ->
        [self-voicing] Self-voicing mode. textweaver speaks everything.
        [screen-reader] Screen reader mode. textweaver is silent; your screen reader reads the status line.
       *[hybrid] Hybrid mode. textweaver reads documents aloud; your screen reader speaks messages and typing.
    }
# $reader is the screen reader found (NVDA, JAWS), or access-a-screen-reader.
access-a-screen-reader = A screen reader
# On the first run with a screen reader and no mode chosen: hybrid mode is
# used, not asked. $reader is the screen reader found (NVDA, JAWS), or
# access-a-screen-reader; $key names the keys that change the mode.
access-hybrid-inferred = Hybrid mode: { $reader } speaks messages, { -brand } reads documents. { $key } changes it.
# The window's two modes, when the mode changes; $key changes it again.
access-window-choice-reads-aloud = textweaver reads aloud
access-window-choice-screen-reader = my screen reader reads
access-window-mode-help = Who reads in the window: textweaver's voice, or your screen reader alone.
access-window-mode-changed =
    { $mode ->
        [screen-reader] My screen reader reads: textweaver stays silent and sends the text to your screen reader. { $key } changes this.
        [speaks-messages] textweaver reads aloud and speaks its messages. { $key } changes this.
       *[reads-aloud] textweaver reads aloud; messages go to your screen reader. { $key } changes this.
    }

## Lexend downloaded on first choice. The question names the size and
## the license before y or n; the rest put the font's name first.
font-download-question = Download the { $font } font, { $kb } KB, { $licence }? y or n
font-downloading = Downloading { $font }.
font-downloaded = { $font } downloaded and ready.
font-download-failed = { $font } not downloaded: { $error } Another font is used.
font-download-declined = Not downloaded. Another font is used.
font-download-busy = { $font } is still downloading.
font-download-no-folder = No data folder to keep { $font } in.
font-download-not-in-build = Font downloads are not in this version.
gui-font-to-download = { $family } (download, { $kb } KB)
gui-font-downloaded = { $family } (downloaded)
