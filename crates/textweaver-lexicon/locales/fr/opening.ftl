### Opening documents, the file browser, and batch conversion.

## Opening a document.

open-opened = { $title } ouvert.
open-resumed = { $title } ouvert. Reprise à { $pct } pour cent.
open-resumed-synced = { $title } ouvert. Reprise à { $pct } pour cent, depuis un autre appareil.

## Opening a document: failures and opening in the background.

# $name is the file's name.
opening-is-folder = { $name } est un dossier, pas un document. Donnez le nom d'un fichier qu'il contient.
# Said after "Could not open NAME:", so it starts in lower case.
opening-no-file-in = il n'y a aucun fichier nommé { $name } dans { $folder }. Vérifiez le nom.
opening-no-file-here = il n'y a aucun fichier nommé { $name } ici. Vérifiez le nom.
opening-no-permission = vous n'avez pas la permission de le lire.
opening-damaged-rtf = ce n'est pas un fichier RTF lisible, il est peut-être endommagé.
opening-damaged-odt = ce n'est pas un fichier texte OpenDocument lisible, il est peut-être endommagé.
opening-damaged-latex = ce n'est pas un fichier LaTeX lisible, il est peut-être endommagé ou trop volumineux.
opening-damaged-email = ce n'est pas un message électronique lisible, il est peut-être endommagé ou trop volumineux.
opening-damaged-mhtml = ce n'est pas une archive web lisible, elle est peut-être endommagée ou trop volumineuse.
opening-pdf-password = il est protégé par un mot de passe. Retirez le mot de passe dans un logiciel PDF, puis ouvrez-le de nouveau.
opening-old-office = c'est un ancien fichier Microsoft Office. Enregistrez-le dans un format plus récent, comme .docx, puis ouvrez celui-ci.
opening-rar = c'est une archive RAR, qui ne s'ouvre pas. Extrayez-la d'abord, ou utilisez ZIP ou 7z.
# $reason is one of the opening-no-* messages, or the loader's own words.
opening-failed = Impossible d'ouvrir { $name } : { $reason }
opening-started = Ouverture de { $name }. Échap annule.
opening-stopped = Ouverture de { $name } arrêtée.
opening-still = Ouverture de { $name } toujours en cours, { $secs } secondes.
# $step is the loader's report, such as "recognizing text on page 3 (3 of 40)."
opening-still-step = Ouverture de { $name } toujours en cours : { $step }
opening-stopped-unexpectedly = Impossible d'ouvrir { $name } : le chargement s'est arrêté de façon inattendue.

## A build without the publish feature. "tw convert" is a command typed
## at the terminal: keep it as it is.

lean-citations-not-in-build = Les citations ne sont pas incluses dans cette version de { -brand }.
lean-publish-not-in-build = L'export et l'aperçu ne sont pas inclus dans cette version de { -brand }. tw convert continue de convertir.

## Background work, and opening files and addresses.

# $what names the export, as its own message says it.
tasks-stopped = { $what } arrêté de façon inattendue.
# $input is the DOI, ISBN, or other identifier being looked up.
tasks-lookup-stopped = La recherche de { $input } s'est arrêtée de façon inattendue.
# The reason in tasks-could-not-open when a session keeps no files.
tasks-launch-off = l'ouverture d'autres programmes est désactivée dans une session qui ne conserve aucun fichier
tasks-opening = Ouverture.
# $target is a file or a web address; $error says why.
tasks-could-not-open = Impossible d'ouvrir { $target } : { $error }
open-refused-scheme = Non ouvert : lien { $scheme } bloqué.
open-refused-missing = Non ouvert : fichier introuvable.
open-refused-invalid = Non ouvert : adresse non valide.
open-refused-network = Non ouvert : le fichier est sur un autre ordinateur.
tasks-not-opened = Non ouvert.
# Keep the letters y and n: they are the keys that answer.
tasks-open-it-question = L'ouvrir ? y ou n

## Opening the new formats. Said after "Could not open NAME:", so
## each starts in lower case.
opening-damaged-json = ce n'est pas un fichier JSON lisible, il est peut-être trop volumineux.
opening-damaged-notebook = ce n'est pas un carnet Jupyter lisible, il est peut-être endommagé ou trop volumineux.
opening-damaged-canvas = ce n'est pas un canevas lisible, il est peut-être endommagé ou trop volumineux.
opening-damaged-svg = ce n'est pas un dessin SVG lisible, il est peut-être endommagé ou trop volumineux.
opening-damaged-mathml = ce n'est pas une formule MathML lisible, elle est peut-être endommagée ou trop volumineuse.

## The file browser. Every row and introduction starts with the name,
## then the kind, so the first cells of a 40-cell Braille line hold what
## matters. $name is a file or folder name; $n a number that chooses the
## plural and $count the same number written with its separators.
# A list item with its position after it, in the file browser.
listmodel-item-position-last = { $item }, { $k } sur { $n }
browse-places-title = Emplacements
browse-places-intro =
    { $n ->
        [one] Emplacements, 1 emplacement.
       *[other] Emplacements, { $n } emplacements.
    }
# $purpose says what the folder or file is chosen for; $intro follows.
browse-choosing = { $purpose }. { $intro }
browse-place-document = { $name }, le dossier du document
browse-place-start = { $name }, dossier de départ
browse-place-library = { $name }, dossier de la bibliothèque
browse-place-disk = { $name }, disque
browse-place-removable = { $name }, lecteur amovible
browse-place-network = { $name }, lecteur réseau
browse-place-cd = { $name }, lecteur de CD ou DVD
browse-place-root = { $name }, le dossier racine
browse-choose-here = Choisir ce dossier, { $name }
browse-row-folder = { $name }, dossier
browse-row-folder-items =
    { $name }, dossier, { $n ->
        [one] 1 élément
       *[other] { $count } éléments
    }
# $kind is a kind below ("Markdown"); $size a size below ("12 KB").
browse-row-file = { $name }, { $kind }, { $size }
browse-row-kind = { $name }, { $kind }
browse-row-hidden = { $row }, caché
# $kind is zip, tar, tar.gz, gzip, or 7z.
browse-kind-archive = archive { $kind }
browse-kind-file = fichier
browse-kind-markdown = Markdown
browse-kind-text = texte
browse-kind-html = page web
browse-kind-epub = livre EPUB
browse-kind-docx = document Word
browse-kind-rtf = document RTF
browse-kind-odt = texte OpenDocument
browse-kind-latex = LaTeX
browse-kind-eml = courriel
browse-kind-mhtml = archive web
browse-kind-pdf = PDF
browse-kind-image = image
browse-kind-daisy = livre DAISY
browse-kind-pptx = diapositives PowerPoint
browse-kind-sheet = feuille de calcul
browse-kind-json = JSON
browse-kind-notebook = carnet Jupyter
browse-kind-svg = dessin SVG
browse-kind-mathml = formule MathML
browse-kind-pandoc = document lu avec Pandoc
browse-size-bytes =
    { $n ->
        [one] 1 octet
       *[other] { $count } octets
    }
# $size is a number, with a decimal under 10 ("3.4").
browse-size-kb = { $size } Ko
browse-size-mb = { $size } Mo
browse-size-gb = { $size } Go
browse-intro =
    { $name }, { $n ->
        [one] 1 élément.
       *[other] { $count } éléments.
    }
browse-intro-empty = { $name } n'a rien à afficher.
# $filter is what was typed.
browse-intro-filtered =
    { $name }, { $n ->
        [one] 1 élément correspond à
       *[other] { $count } éléments correspondent à
    } { $filter }.
browse-intro-in-archive = { $intro } Dans { $archive }.
browse-intro-hidden =
    { $intro } { $n ->
        [one] 1 fichier caché.
       *[other] { $count } fichiers cachés.
    }
browse-intro-cut = { $intro } Seuls les { $max } premiers sont affichés.
# $preview, $choose, $sort, and $all are keys; $item the focused row.
browse-keys = { $intro } Entrée ouvre, Retour arrière remonte, la saisie filtre. { $preview } donne un aperçu, { $choose } choisit un dossier, { $sort } trie, { $all } montre tous les fichiers. { $item }
browse-sorted-name = Trié par nom.
browse-sorted-date = Trié par date, le plus récent d'abord.
browse-sorted-size = Trié par taille, le plus grand d'abord.
browse-showing-all = Tous les fichiers sont affichés.
browse-showing-readable = Seuls les fichiers lisibles sont affichés.
browse-closed = Explorateur de fichiers fermé.
browse-read-only = L'explorateur de fichiers ne fait qu'ouvrir et choisir des fichiers ; il ne les modifie jamais.
# $key is the Choose Folder key.
browse-choose-a-folder = Choisissez un dossier : Entrée en ouvre un, { $key } le choisit.
browse-choose-a-file = Choisissez un fichier : Entrée en choisit un.
browse-no-archive-folder = Un dossier dans une archive ne peut pas être choisi ; choisissez un dossier du disque.
browse-nothing-waiting = Aucune commande n'attend de dossier ; Entrée l'ouvre.
browse-cannot-read = { $name } n'est pas un type de fichier que textweaver sait lire.
browse-folder-unreadable = Impossible d'ouvrir { $name } : { $reason }
browse-archive-too-deep = { $name } est dans trop d'archives pour être ouvert.
browse-archive-too-large = { $name } est trop gros pour être listé sans risque.
browse-archive-unreadable = { $name } n'est pas une archive que textweaver sait lire ; elle est peut-être endommagée.
# A document's preview: its title, then its first sentence.
browse-preview-document = { $title }. { $sentence }
browse-preview-no-text = { $title }. Il n'a pas de texte.
browse-preview-failed = Impossible de donner un aperçu de { $name } : { $reason }
# $names are the first few names inside.
browse-preview-archive =
    { $name } : { $n ->
        [one] 1 fichier
       *[other] { $count } fichiers
    }, { $readable } lisibles. { $names }
browse-preview-archive-folder =
    { $name }, dossier de l'archive, { $n ->
        [one] 1 élément.
       *[other] { $count } éléments.
    }
browse-preview-folder = { $path } : { $names }
browse-preview-folder-empty = { $path } : rien à lire ici.
browse-preview-other = { $name }, { $size } ; textweaver ne sait pas lire ce type de fichier.
browse-preview-path = { $path }

## Batch conversion (File, Batch convert). Keep the meaning first.
batch-choose-source = Choisissez le dossier à convertir
batch-choose-output = Choisissez le dossier des fichiers convertis
batch-format-title = Convertir en
batch-format-intro = Convertir { $name } en : choisissez un format, { $n } choix.
batch-where-title = Où vont les fichiers
batch-where-intro = Où mettre les fichiers convertis ?
batch-where-converted = Dans un dossier converted, { $path }
batch-where-beside = À côté de chaque fichier
batch-where-choose = Dans un autre dossier, choisi ensuite
batch-nothing = Aucun document à convertir dans { $path }.
batch-confirm =
    Convertir { $n ->
        [one] 1 fichier
       *[other] { $n } fichiers
    } en { $format } dans { $path } ? y ou n
batch-confirm-beside =
    Convertir { $n ->
        [one] 1 fichier
       *[other] { $n } fichiers
    } en { $format } à côté de chaque fichier ? y ou n
batch-started =
    Conversion de { $n ->
        [one] 1 fichier
       *[other] { $n } fichiers
    } en { $format }. Échap arrête.
batch-progress = { $percent } pour cent convertis, { $done } fichiers sur { $total }.
batch-busy = Conversion déjà en cours, { $done } fichiers sur { $total }. Échap arrête.
batch-stop-question = Arrêter la conversion ? Les fichiers déjà faits sont gardés. y ou n
batch-stopping = Arrêt après les fichiers en cours d'écriture.
batch-still-converting = La conversion continue.
batch-done =
    { $converted ->
        [one] 1 fichier converti
       *[other] { $converted } fichiers convertis
    } en { $format } ; { $skipped } à jour ; { $failed } en échec.
batch-stopped =
    Arrêté. { $converted ->
        [one] 1 fichier converti
       *[other] { $converted } fichiers convertis
    } ; { $left } non convertis ; { $failed } en échec.
batch-report = Rapport enregistré dans { $path }.
batch-report-failed = Le rapport n'a pas pu être enregistré : { $error } Les fichiers convertis sont conservés.
batch-inaccessible =
    { $n ->
        [one] 1 fichier a
       *[other] { $n } fichiers ont
    } des éléments non accessibles ; voir le rapport.
batch-failures-title =
    { $n ->
        [one] 1 fichier en échec
       *[other] { $n } fichiers en échec
    }
batch-failure-item = { $name } : { $reason }
batch-start-failed = Impossible de lancer la conversion : { $error } Vérifiez le dossier et le format, puis réessayez.
batch-thread-stopped = La conversion par lots s'est arrêtée de façon inattendue.
