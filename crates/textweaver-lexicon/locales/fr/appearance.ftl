### Themes, accessibility modes, and the Lexend font.

## Thèmes.

message-error = Erreur : { $message }
# $name is the theme name in the settings; $used the display name of the theme used instead.
themes-unknown = Il n'y a aucun thème appelé { $name } ; { $used } est utilisé à la place.
# $theme is the new theme's display name.
themes-next = Thème { $theme }.
themes-soft-dark-name = { $theme }, sombre doux
themes-choice-aa = { $theme }, conforme AA
themes-choice-below-aa = { $theme }, sous AA
themes-below-aa =
    { $count ->
        [one] Sous AA : 1 vérification n’atteint pas le seuil.
       *[other] Sous AA : { $count } vérifications n’atteignent pas le seuil.
    }

## Accessibility modes and the first-run question.

# Said when the accessibility mode changes; $mode is the new mode's id.
access-mode-changed =
    { $mode ->
        [self-voicing] Mode autonome. textweaver dit tout.
        [screen-reader] Mode lecteur d'écran. textweaver est silencieux ; votre lecteur d'écran lit la ligne d'état.
       *[hybrid] Mode hybride. textweaver lit les documents à voix haute ; votre lecteur d'écran dit les messages et la frappe.
    }
# Yes to the first-run question; $key names the keys that change the mode.
access-hybrid-chosen = Mode hybride. textweaver lit les documents à voix haute ; votre lecteur d'écran dit les messages et la frappe. { $key } change le mode.
# No to the first-run question; $key names the keys that change the mode.
access-hybrid-declined = Maintien du mode autonome. { $key } change le mode.
# $reader is the screen reader found (NVDA, JAWS), or access-a-screen-reader.
access-hybrid-question = { $reader } est en cours d'exécution. Utiliser le mode hybride, où textweaver lit les documents à voix haute et votre lecteur d'écran dit les messages et la frappe ? y ou n
access-a-screen-reader = Un lecteur d'écran
# On the first run with a screen reader and no mode chosen: hybrid mode is
# used, not asked. $reader is the screen reader found (NVDA, JAWS), or
# access-a-screen-reader; $key names the keys that change the mode.
access-hybrid-inferred = { $reader } est en cours d'exécution : textweaver lit les documents à voix haute et laisse les messages à votre lecteur d'écran. { $key } change cela.
# The window's two modes, when the mode changes; $key changes it again.
access-window-choice-reads-aloud = textweaver lit à voix haute
access-window-choice-screen-reader = mon lecteur d'écran lit
access-window-mode-help = Qui lit dans la fenêtre : la voix de textweaver, ou votre lecteur d'écran seul.
access-window-mode-changed =
    { $mode ->
        [screen-reader] Mon lecteur d'écran lit : textweaver se tait et envoie le texte à votre lecteur d'écran. { $key } change cela.
        [speaks-messages] textweaver lit à voix haute et dit ses messages. { $key } change cela.
       *[reads-aloud] textweaver lit à voix haute ; les messages vont à votre lecteur d'écran. { $key } change cela.
    }

## Lexend downloaded on first choice.
font-download-question = Télécharger la police { $font }, { $kb } Ko, { $licence } ? y ou n
font-downloading = Téléchargement de { $font }.
font-downloaded = { $font } téléchargée et prête.
font-download-failed = { $font } non téléchargée : { $error } Autre police utilisée.
font-download-declined = Non téléchargée. Autre police utilisée.
font-download-busy = { $font } est encore en téléchargement.
font-download-no-folder = Aucun dossier de données pour { $font }.
font-download-not-in-build = Téléchargement de polices non inclus.
gui-font-to-download = { $family } (à télécharger, { $kb } Ko)
gui-font-downloaded = { $family } (téléchargée)
