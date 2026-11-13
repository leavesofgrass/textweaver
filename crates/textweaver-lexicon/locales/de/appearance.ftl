### Themes, accessibility modes, and the Lexend font.

## Themes.

message-error = Fehler: { $message }
# $name is the theme name in the settings; $used the display name of the theme used instead.
themes-unknown = Es gibt kein Design namens { $name }; { $used } wird verwendet.
# $theme is the new theme's display name.
themes-next = Design { $theme }.
themes-soft-dark-name = { $theme }, sanft dunkel
themes-choice-aa = { $theme }, erfüllt AA
themes-choice-below-aa = { $theme }, unter AA
themes-below-aa =
    { $count ->
        [one] Unter AA: 1 Prüfung verfehlt den Wert.
       *[other] Unter AA: { $count } Prüfungen verfehlen den Wert.
    }

## Accessibility modes and the first-run step for a screen reader.

# Said when the accessibility mode changes; $mode is the new mode's id.
access-mode-changed =
    { $mode ->
        [self-voicing] Selbstsprechend-Modus. textweaver spricht alles.
        [screen-reader] Screenreader-Modus. textweaver ist stumm; Ihr Screenreader liest die Statuszeile.
       *[hybrid] Hybrid-Modus. textweaver liest Dokumente vor; Ihr Screenreader spricht Meldungen und Tippen.
    }
# $reader is the screen reader found (NVDA, JAWS), or access-a-screen-reader.
access-a-screen-reader = Ein Screenreader
# On the first run with a screen reader and no mode chosen: hybrid mode is
# used, not asked. $reader is the screen reader found (NVDA, JAWS), or
# access-a-screen-reader; $key names the keys that change the mode.
access-hybrid-inferred = Hybrid-Modus: { $reader } spricht Meldungen, { -brand } liest Dokumente vor. { $key } ändert das.
# The window's two modes, when the mode changes; $key changes it again.
access-window-choice-reads-aloud = textweaver liest vor
access-window-choice-screen-reader = mein Screenreader liest
access-window-mode-help = Wer im Fenster liest: die Stimme von textweaver oder nur Ihr Screenreader.
access-window-mode-changed =
    { $mode ->
        [screen-reader] Mein Screenreader liest: textweaver schweigt und gibt den Text an Ihren Screenreader. { $key } ändert das.
        [speaks-messages] textweaver liest vor und spricht seine Meldungen. { $key } ändert das.
       *[reads-aloud] textweaver liest vor; Meldungen gehen an Ihren Screenreader. { $key } ändert das.
    }

## Lexend downloaded on first choice.
font-download-question = Schrift { $font } herunterladen, { $kb } KB, { $licence }? y oder n
font-downloading = { $font } wird heruntergeladen.
font-downloaded = { $font } heruntergeladen und bereit.
font-download-failed = { $font } nicht heruntergeladen: { $error } Andere Schrift aktiv.
font-download-declined = Nicht geladen. Andere Schrift aktiv.
font-download-busy = { $font } wird noch heruntergeladen.
font-download-no-folder = Kein Datenordner für { $font }.
font-download-not-in-build = Keine Schrift-Downloads in diesem Build.
gui-font-to-download = { $family } (herunterladen, { $kb } KB)
gui-font-downloaded = { $family } (heruntergeladen)
