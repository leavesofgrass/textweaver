### Braille: BRF files read as print, and the Braille view.

## B1-r5: braille (BRF) files read as print. $page is a braille page
## number ("3", "p1"); $n is a number of lines; $reason is an error.

## Beta 4: the Braille view (braille dots at the bottom of the app)
## and the braille grade. $n is the part shown, $total how many.
braille-view-shown = Vue braille affichée.
braille-view-hidden = Vue braille masquée.
braille-view-not-in-build = La vue braille n'est pas incluse dans cette version de { -brand }.
braille-grade-uncontracted = Braille intégral, grade 1.
braille-grade-contracted = Braille abrégé, grade 2.
braille-grade-contracted-no-liblouis = Braille abrégé, grade 2. liblouis est absent : le braille intégral est affiché jusqu'à son installation.
settings-unit-cells =
    { $n ->
        [one] cellule
       *[other] cellules
    }
gui-braille-view = Vue braille
gui-braille-view-description = Visuel seulement : la ligne courante en points braille, pour les personnes voyantes qui apprennent ou enseignent le braille. Votre lecteur d'écran affiche le braille sur votre afficheur braille.
gui-braille-caption-uncontracted = Braille intégral, { $n } sur { $total }
gui-braille-caption-contracted = Braille abrégé, { $n } sur { $total }
gui-braille-caption-fell-back = Braille intégral (l'abrégé demande liblouis), { $n } sur { $total }
brf-original-title = Braille d'origine, page { $page }
brf-original-intro =
    { $n ->
        [one] Braille d'origine, page { $page }, { $n } ligne. Échap ferme.
       *[other] Braille d'origine, page { $page }, { $n } lignes. Échap ferme.
    }
brf-original-not-brf = Pas un fichier braille. Afficher le braille d'origine fonctionne avec les fichiers BRF.
brf-original-unreadable = Impossible de lire le fichier braille : { $reason }
brf-no-liblouis = Braille affiché en braille : liblouis manque. Pour le lire en noir, installez liblouis depuis liblouis.io ou les paquets de votre système, puis rouvrez le fichier.
brf-no-liblouis-short = Braille affiché en braille : liblouis manque. Rouvrez le fichier une fois qu'il est installé.
daisy-headings-only = Titres seulement : ce livre DAISY n'a pas de texte, seulement des titres et de l'audio, donc ses titres sont lus.
## End of B1-r5
