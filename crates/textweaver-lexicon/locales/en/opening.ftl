### Opening documents, the file browser, and batch conversion.

## Opening a document.

open-opened = Opened { $title }.
open-resumed = Opened { $title }. Resumed at { $pct } percent.
open-resumed-synced = Opened { $title }. Resumed at { $pct } percent, from another device.

## Opening a document: failures and opening in the background.

# $name is the file's name.
opening-is-folder = { $name } is a folder, not a document. Give the name of a file in it.
# Said after "Could not open NAME:", so it starts in lower case.
opening-no-file-in = there is no file named { $name } in { $folder }. Check the name.
opening-no-file-here = there is no file named { $name } here. Check the name.
opening-no-permission = you do not have permission to read it.
opening-damaged-rtf = it is not a readable RTF file; it may be damaged.
opening-damaged-odt = it is not a readable OpenDocument text file; it may be damaged.
opening-damaged-latex = it is not a readable LaTeX file; it may be damaged or too large.
opening-damaged-email = it is not a readable email message; it may be damaged or too large.
opening-damaged-mhtml = it is not a readable web archive; it may be damaged or too large.
opening-pdf-password = it is protected by a password. Remove the password in a PDF program, then open it again.
opening-old-office = it is an old Microsoft Office file. Save it in a newer format, such as .docx, then open that.
opening-rar = it is a RAR archive, which does not open. Extract it first, or use ZIP or 7z.
# $reason is one of the opening-no-* messages, or the loader's own words.
opening-failed = Could not open { $name }: { $reason }
opening-started = Opening { $name }. Escape cancels.
opening-stopped = Stopped opening { $name }.
opening-still = Still opening { $name }, { $secs } seconds.
# $step is the loader's report, such as "recognizing text on page 3 (3 of 40)."
opening-still-step = Still opening { $name }: { $step }
opening-stopped-unexpectedly = Could not open { $name }: loading stopped unexpectedly.

## A build without the publish feature. "tw convert" is a command typed
## at the terminal: keep it as it is.

lean-citations-not-in-build = Citations are not in this version of { -brand }.
lean-publish-not-in-build = Export and preview are not in this version of { -brand }. tw convert still converts.

## Background work, and opening files and addresses.

# $what names the export, as its own message says it.
tasks-stopped = { $what } stopped unexpectedly.
# $input is the DOI, ISBN, or other identifier being looked up.
tasks-lookup-stopped = Looking up { $input } stopped unexpectedly.
# The reason in tasks-could-not-open when a session keeps no files.
tasks-launch-off = opening other programs is off in a session that keeps no files
tasks-opening = Opening.
# $target is a file or a web address; $error says why.
tasks-could-not-open = Could not open { $target }: { $error }
open-refused-scheme = Not opened: { $scheme } link blocked.
open-refused-missing = Not opened: the file was not found.
open-refused-invalid = Not opened: not a valid address.
open-refused-network = Not opened: the file is on another computer.
tasks-not-opened = Not opened.
# Keep the letters y and n: they are the keys that answer.
tasks-open-it-question = Open it? y or n

## Opening the new formats. Said after "Could not open NAME:", so
## each starts in lower case.
opening-damaged-json = it is not a readable JSON file; it may be too large.
opening-damaged-notebook = it is not a readable Jupyter notebook; it may be damaged or too large.
opening-damaged-canvas = it is not a readable canvas; it may be damaged or too large.
opening-damaged-svg = it is not a readable SVG drawing; it may be damaged or too large.
opening-damaged-mathml = it is not a readable MathML formula; it may be damaged or too large.

## The file browser. Every row and introduction starts with the name,
## then the kind, so the first cells of a 40-cell Braille line hold what
## matters. $name is a file or folder name; $n a number that chooses the
## plural and $count the same number written with its separators.
# A list item with its position after it, in the file browser.
listmodel-item-position-last = { $item }, { $k } of { $n }
browse-places-title = Places
browse-places-intro =
    { $n ->
        [one] Places, 1 place.
       *[other] Places, { $n } places.
    }
# $purpose says what the folder or file is chosen for; $intro follows.
browse-choosing = { $purpose }. { $intro }
browse-place-document = { $name }, the document's folder
browse-place-start = { $name }, start folder
browse-place-library = { $name }, library folder
browse-place-disk = { $name }, disk
browse-place-removable = { $name }, removable drive
browse-place-network = { $name }, network drive
browse-place-cd = { $name }, CD or DVD drive
browse-place-root = { $name }, the root folder
browse-choose-here = Choose this folder, { $name }
browse-row-folder = { $name }, folder
browse-row-folder-items =
    { $name }, folder, { $n ->
        [one] 1 item
       *[other] { $count } items
    }
# $kind is a kind below ("Markdown"); $size a size below ("12 KB").
browse-row-file = { $name }, { $kind }, { $size }
browse-row-kind = { $name }, { $kind }
browse-row-hidden = { $row }, hidden
# $kind is zip, tar, tar.gz, gzip, or 7z.
browse-kind-archive = { $kind } archive
browse-kind-file = file
browse-kind-markdown = Markdown
browse-kind-text = text
browse-kind-html = web page
browse-kind-epub = EPUB book
browse-kind-docx = Word document
browse-kind-rtf = RTF document
browse-kind-odt = OpenDocument text
browse-kind-latex = LaTeX
browse-kind-eml = email
browse-kind-mhtml = web archive
browse-kind-pdf = PDF
browse-kind-image = picture
browse-kind-daisy = DAISY book
browse-kind-pptx = PowerPoint slides
browse-kind-sheet = spreadsheet
browse-kind-json = JSON
browse-kind-notebook = Jupyter notebook
browse-kind-svg = SVG drawing
browse-kind-mathml = MathML formula
browse-kind-pandoc = document read through Pandoc
browse-size-bytes =
    { $n ->
        [one] 1 byte
       *[other] { $count } bytes
    }
# $size is a number, with a decimal under 10 ("3.4").
browse-size-kb = { $size } KB
browse-size-mb = { $size } MB
browse-size-gb = { $size } GB
browse-intro =
    { $name }, { $n ->
        [one] 1 item.
       *[other] { $count } items.
    }
browse-intro-empty = { $name } has nothing to list.
# $filter is what was typed.
browse-intro-filtered =
    { $name }, { $n ->
        [one] 1 item matches
       *[other] { $count } items match
    } { $filter }.
browse-intro-in-archive = { $intro } In { $archive }.
browse-intro-hidden =
    { $intro } { $n ->
        [one] 1 file hidden.
       *[other] { $count } files hidden.
    }
browse-intro-cut = { $intro } Only the first { $max } are listed.
# $preview, $choose, $sort, and $all are keys; $item the focused row.
browse-keys = { $intro } Enter opens, Backspace goes up, typing filters. { $preview } previews, { $choose } chooses a folder, { $sort } sorts, { $all } shows all files. { $item }
browse-sorted-name = Sorted by name.
browse-sorted-date = Sorted by date, newest first.
browse-sorted-size = Sorted by size, largest first.
browse-showing-all = Showing all files.
browse-showing-readable = Showing readable files only.
browse-closed = File browser closed.
browse-read-only = The file browser only opens and chooses files; it never changes them.
# $key is the Choose Folder key.
browse-choose-a-folder = Choose a folder: Enter opens one, { $key } chooses it.
browse-choose-a-file = Choose a file: Enter chooses one.
browse-no-archive-folder = A folder inside an archive cannot be chosen; choose a folder on disk.
browse-nothing-waiting = No command is waiting for a folder; Enter opens it.
browse-cannot-read = { $name } is not a kind of file textweaver can read.
browse-folder-unreadable = Could not open { $name }: { $reason }
browse-archive-too-deep = { $name } is inside too many archives to open.
browse-archive-too-large = { $name } is too large to list safely.
browse-archive-unreadable = { $name } is not an archive textweaver can read; it may be damaged.
# A document's preview: its title, then its first sentence.
browse-preview-document = { $title }. { $sentence }
browse-preview-no-text = { $title }. It has no text.
browse-preview-failed = Could not preview { $name }: { $reason }
# $names are the first few names inside.
browse-preview-archive =
    { $name }: { $n ->
        [one] 1 file
       *[other] { $count } files
    }, { $readable } readable. { $names }
browse-preview-archive-folder =
    { $name }, folder in the archive, { $n ->
        [one] 1 item.
       *[other] { $count } items.
    }
browse-preview-folder = { $path }: { $names }
browse-preview-folder-empty = { $path }: nothing to read here.
browse-preview-other = { $name }, { $size }; textweaver cannot read this kind of file.
browse-preview-path = { $path }

## Batch conversion (File, Batch convert). Keep the meaning first: a
## 40-cell Braille display shows the start of the line. $n, $done, $total,
## $converted, $skipped, $failed and $left are numbers of files; $format is
## a format's name (PDF, Markdown); $path a folder; $name a file or folder
## name; $reason and $error are passed on as they are.
batch-choose-source = Choose the folder to convert
batch-choose-output = Choose the folder for the converted files
batch-format-title = Convert to
batch-format-intro = Convert { $name } to: choose a format, { $n } choices.
batch-where-title = Where the files go
batch-where-intro = Where should the converted files go?
batch-where-converted = In a converted folder, { $path }
batch-where-beside = Beside each file
batch-where-choose = In another folder, chosen next
batch-nothing = No documents to convert in { $path }.
batch-confirm =
    Convert { $n ->
        [one] 1 file
       *[other] { $n } files
    } to { $format } into { $path }? y or n
batch-confirm-beside =
    Convert { $n ->
        [one] 1 file
       *[other] { $n } files
    } to { $format } beside each file? y or n
batch-started =
    Converting { $n ->
        [one] 1 file
       *[other] { $n } files
    } to { $format }. Escape stops.
batch-progress = { $percent } percent converted, { $done } of { $total } files.
batch-busy = Already converting, { $done } of { $total } files. Escape stops.
batch-stop-question = Stop converting? Files already done are kept. y or n
batch-stopping = Stopping after the files being written.
batch-still-converting = Still converting.
batch-done =
    Converted { $converted ->
        [one] 1 file
       *[other] { $converted } files
    } to { $format }; { $skipped } up to date; { $failed } failed.
batch-stopped =
    Stopped. Converted { $converted ->
        [one] 1 file
       *[other] { $converted } files
    }; { $left } not converted; { $failed } failed.
batch-report = Report saved in { $path }.
batch-report-failed = The report could not be saved: { $error } The converted files are kept.
batch-inaccessible =
    { $n ->
        [one] 1 file has
       *[other] { $n } files have
    } items not made accessible; see the report.
batch-failures-title =
    { $n ->
        [one] 1 failed file
       *[other] { $n } failed files
    }
batch-failure-item = { $name }: { $reason }
batch-start-failed = Could not start converting: { $error } Check the folder and the format, then try again.
batch-thread-stopped = Batch conversion stopped unexpectedly.
