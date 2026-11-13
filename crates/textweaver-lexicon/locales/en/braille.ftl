### Braille: BRF files read as print, and the Braille view.

## B1-r5: braille (BRF) files read as print. $page is a braille page
## number ("3", "p1"); $n is a number of lines; $reason is an error.

## Beta 4: the Braille view (braille dots at the bottom of the app)
## and the braille grade. $n is the part shown, $total how many.
braille-view-shown = Braille view shown.
braille-view-hidden = Braille view hidden.
braille-view-not-in-build = The Braille view is not in this version of { -brand }.
braille-grade-uncontracted = Uncontracted braille, grade 1.
braille-grade-contracted = Contracted braille, grade 2.
braille-grade-contracted-no-liblouis = Contracted braille, grade 2. liblouis is missing, so uncontracted braille is shown until it is installed.
settings-unit-cells =
    { $n ->
        [one] cell
       *[other] cells
    }
gui-braille-view = Braille view
gui-braille-view-description = Visual only: the current line as braille dots, for sighted readers learning or teaching braille. Your screen reader shows braille on your Braille display.
gui-braille-caption-uncontracted = Uncontracted braille, { $n } of { $total }
gui-braille-caption-contracted = Contracted braille, { $n } of { $total }
gui-braille-caption-fell-back = Uncontracted braille (contracted needs liblouis), { $n } of { $total }
brf-original-title = Original Braille, page { $page }
brf-original-intro =
    { $n ->
        [one] Original Braille, page { $page }, { $n } line. Escape closes.
       *[other] Original Braille, page { $page }, { $n } lines. Escape closes.
    }
brf-original-not-brf = Not a braille file. Show original Braille works on BRF files.
brf-original-unreadable = Cannot read the braille file: { $reason }
brf-no-liblouis = Braille shown as braille: liblouis is missing. To read it as print, install liblouis from liblouis.io or your system's packages, then open the file again.
brf-no-liblouis-short = Braille shown as braille: liblouis is missing. Open the file again once it is installed.
# Said on opening a DAISY 2.02 book with no text files.
daisy-headings-only = Headings only: this DAISY book has no text, just headings and audio, so its headings are read.
## End of B1-r5
