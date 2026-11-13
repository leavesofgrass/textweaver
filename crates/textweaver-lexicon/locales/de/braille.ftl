### Braille: BRF files read as print, and the Braille view.

## B1-r5: braille (BRF) files read as print. $page is a braille page
## number ("3", "p1"); $n is a number of lines; $reason is an error.

## Beta 4: the Braille view (braille dots at the bottom of the app)
## and the braille grade. $n is the part shown, $total how many.
braille-view-shown = Braille-Ansicht eingeblendet.
braille-view-hidden = Braille-Ansicht ausgeblendet.
braille-view-not-in-build = Die Braille-Ansicht ist in dieser Version von { -brand } nicht enthalten.
braille-grade-uncontracted = Braille-Vollschrift, Grad 1.
braille-grade-contracted = Braille-Kurzschrift, Grad 2.
braille-grade-contracted-no-liblouis = Braille-Kurzschrift, Grad 2. liblouis fehlt, daher wird Vollschrift gezeigt, bis es installiert ist.
settings-unit-cells =
    { $n ->
        [one] Zelle
       *[other] Zellen
    }
gui-braille-view = Braille-Ansicht
gui-braille-view-description = Nur sichtbar: die aktuelle Zeile als Braillepunkte, für sehende Menschen, die Braille lernen oder lehren. Ihr Bildschirmleser zeigt Braille auf Ihrer Braillezeile.
gui-braille-caption-uncontracted = Vollschrift, { $n } von { $total }
gui-braille-caption-contracted = Kurzschrift, { $n } von { $total }
gui-braille-caption-fell-back = Vollschrift (Kurzschrift braucht liblouis), { $n } von { $total }
brf-original-title = Original-Braille, Seite { $page }
brf-original-intro =
    { $n ->
        [one] Original-Braille, Seite { $page }, { $n } Zeile. Escape schließt.
       *[other] Original-Braille, Seite { $page }, { $n } Zeilen. Escape schließt.
    }
brf-original-not-brf = Keine Braille-Datei. Original-Braille anzeigen funktioniert mit BRF-Dateien.
brf-original-unreadable = Die Braille-Datei kann nicht gelesen werden: { $reason }
brf-no-liblouis = Braille als Braille angezeigt: liblouis fehlt. Um sie als Schwarzschrift zu lesen, installieren Sie liblouis von liblouis.io oder aus den Paketen Ihres Systems und öffnen Sie die Datei erneut.
brf-no-liblouis-short = Braille als Braille angezeigt: liblouis fehlt. Öffnen Sie die Datei erneut, sobald es installiert ist.
daisy-headings-only = Nur Überschriften: Dieses DAISY-Buch hat keinen Text, nur Überschriften und Audio, daher werden die Überschriften gelesen.
## End of B1-r5
