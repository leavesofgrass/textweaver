### Export, preview, and audio export.

## Export et aperçu depuis le lecteur. F5 est la touche de rechargement du
## navigateur, pas celle de textweaver.

common-no-document = Aucun document n'est ouvert.
# Said after "Could not export:", so it starts in lower case. $path is a
# folder or a file; $error the system's reason.
publish-cannot-write-to = impossible d'écrire dans { $path } : { $error } Vérifiez que le dossier est accessible en écriture.
publish-cannot-write = impossible d'écrire { $path } : { $error } Vérifiez que son dossier est accessible en écriture.
publish-start-failed = Impossible de démarrer l'export : { $error } Patientez un instant, puis réessayez.
publish-export-error = Impossible d'exporter : { $error } Corrigez cela, puis exportez de nouveau.
publish-export-as-label = Exporter sous, Entrée pour { $path }
publish-export-over-source = Non exporté : c'est le document lui-même. Choisissez un autre nom.
# $format is the format's name, such as PDF, HTML, or Word.
publish-exporting = Export vers { $format }.
publish-theme-title = Thème de la page HTML
publish-theme-intro = Thème de la page HTML ? { $first } en premier, { $n } choix. Échap annule.
publish-writing-preview = Écriture de l'aperçu.
publish-preview-error = Impossible d'écrire l'aperçu : { $error } Corrigez cela, puis relancez l'aperçu.
publish-still-exporting =
    { $secs ->
        [one] Export vers { $format } toujours en cours, { $secs } seconde.
       *[other] Export vers { $format } toujours en cours, { $secs } secondes.
    }
publish-still-previewing =
    { $secs ->
        [one] Écriture de l'aperçu toujours en cours, { $secs } seconde.
       *[other] Écriture de l'aperçu toujours en cours, { $secs } secondes.
    }
# "Browser preview follows" was chosen. $again is yes when a preview
# is open that reloads by itself only once it is opened again.
publish-follow-off = L'aperçu du navigateur ne suit rien : appuyez sur F5 dans le navigateur après un enregistrement.
publish-follow-save =
    { $again ->
        [yes] L'aperçu du navigateur suit chaque enregistrement : la page se recharge seule. Ouvrez de nouveau l'aperçu dans le navigateur pour commencer.
       *[no] L'aperçu du navigateur suit chaque enregistrement : la page se recharge seule.
    }
publish-follow-typing =
    { $again ->
        [yes] L'aperçu du navigateur suit votre frappe : la page se recharge après chaque enregistrement et quand la frappe fait une pause. Ouvrez de nouveau l'aperçu dans le navigateur pour commencer.
       *[no] L'aperçu du navigateur suit votre frappe : la page se recharge après chaque enregistrement et quand la frappe fait une pause.
    }
# The first preview of a session: what the browser will do. $follow is
# the "Browser preview follows" setting: off, save, or typing.
publish-preview-first =
    { $follow ->
        [save] L'aperçu s'ouvre dans votre navigateur et se recharge après chaque enregistrement.
        [typing] L'aperçu s'ouvre dans votre navigateur et suit votre frappe.
       *[off] L'aperçu s'ouvre dans votre navigateur. Appuyez sur F5 dans celui-ci après chaque enregistrement.
    }
# $error is the converter's reason.
publish-export-failed = Échec de l'export vers { $format } : { $error } Essayez un autre format.
publish-preview-failed = Échec de l'aperçu : { $error } Enregistrez pour réessayer.
# The converter's warnings: how many, and the first one.
publish-warnings =
    { $n ->
        [one] 1 avertissement : { $first }
       *[other] { $n } avertissements ; le premier : { $first }
    }
# $file is the file's name, $folder its folder; $warned is empty or a
# space and publish-warnings.
publish-exported = Exporté vers { $format } : { $file }. L'ouvrir ? y ou n. Dans { $folder }.{ $warned }
publish-report = Rapport enregistré sous { $file }.
publish-report-issues =
    { $n ->
        [one] 1 élément non accessible
       *[other] { $n } éléments non accessibles
    } ; voir le rapport.
publish-report-failed = Le rapport n'a pas pu être enregistré : { $error } Vérifiez que le dossier de sortie est accessible en écriture.
publish-preview-written-served = Aperçu écrit. Ouverture dans le navigateur. Il se recharge tout seul après chaque enregistrement.{ $warned }
publish-preview-written = Aperçu écrit. Ouverture dans le navigateur. Un enregistrement le réécrit ; appuyez ensuite sur F5 dans le navigateur.{ $warned }
publish-preview-updated = Aperçu mis à jour.
publish-preview-updated-press-f5 = Aperçu mis à jour. Appuyez sur F5 dans le navigateur.
# Said once for a document whose preview was cleaned of raw HTML
# (scripts, event handlers) because its folder is not trusted.
publish-preview-raw-html-left-out = HTML brut omis dans l’aperçu. Les dossiers de confiance sont dans les Paramètres.
publish-server-failed = Impossible de démarrer le serveur de rechargement de l'aperçu ({ $error }) ; ouverture du fichier à la place.
publish-render-failed = Impossible de rendre le texte : { $error } Quittez le mode édition pour lire le texte.
publish-nothing-after-caret = Rien à lire après le curseur.
publish-listening = Écoute du texte rendu.

## The preview's reload server: shown in the browser.

preview-being-written = L'aperçu est en cours d'écriture. Rechargez dans un instant.

## Audio export (File, Export audio). Keep the meaning first: a
## 40-cell Braille display shows the start of the line. $name is a file
## name (essay.flac); $path a folder or a file's full path; $format a
## format's name (FLAC, MP3); $voice a voice's or engine's name; $wpm is
## words per minute; $length a length of time from the duration-*
## messages; $chapters and $n are numbers; $percent is a multiple of ten;
## $formats lists format names (M4B); $error is passed on as it is.
audio-format-title = Exporter l'audio en
audio-format-intro = Exporter { $name } en audio : choisissez un format, { $n } choix.
audio-no-ffmpeg =
    { $formats } { $n ->
        [one] demande
       *[other] demandent
    } ffmpeg, qui est introuvable.
audio-format-flac = FLAC : sans perte, environ moitié moins lourd que WAV
audio-format-wav = WAV : le plus lourd, se lit partout
audio-format-mp3 = MP3 : léger, se lit partout
audio-format-opus = Opus : le plus léger, conçu pour la voix
audio-format-ogg = Ogg Vorbis : léger et ouvert, se lit dans la plupart des lecteurs
audio-format-m4b = Livre audio M4B, par ffmpeg
audio-format-mp4 = Vidéo avec sous-titres : MP4, demande ffmpeg
audio-format-html = Page de lecture : texte et audio, un seul fichier
audio-where-title = Où va l'audio
audio-where-intro = Où enregistrer l'audio ?
audio-where-beside = À côté du document, { $path }
audio-where-choose = Dans un autre dossier, choisi ensuite
audio-choose-folder = Choisissez le dossier de l'audio
audio-no-engine = Aucun moteur vocal ici ne sait écrire de fichier audio. Installez eSpeak NG, ou choisissez un autre moteur dans le menu Parole.
audio-confirm = Exporter { $name } avec { $voice } à { $wpm } mots par minute, dans { $path } ? y ou n
audio-started = Export de { $name } en { $format }. Échap arrête.
audio-progress = Export audio, { $percent } pour cent.
audio-video-progress =
    { $minutes ->
        [one] Encodage vidéo : { $fed } images sur { $all }, environ 1 minute restante.
       *[other] Encodage vidéo : { $fed } images sur { $all }, environ { $minutes } minutes restantes.
    }
audio-video-progress-soon = Encodage vidéo : { $fed } images sur { $all }, moins d'une minute restante.
audio-busy = { $name } est déjà en cours d'export. Échap arrête.
audio-stop-question = Arrêter l'export ? Aucun fichier n'est gardé. y ou n
audio-stopping = Arrêt de l'export.
audio-still-exporting = L'export audio continue.
audio-stopped = Export audio arrêté ; aucun fichier écrit.
audio-done =
    { $name } écrit : { $length }, { $chapters ->
        [one] 1 chapitre
       *[other] { $chapters } chapitres
    }.
audio-subtitles = Sous-titres dans { $name }.
audio-failed = Impossible d'exporter l'audio : { $error } Essayez un autre format ou une autre voix.
audio-thread-stopped = L'export audio s'est arrêté de façon inattendue.
