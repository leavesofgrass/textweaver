### Braille: BRF files read as print, and the Braille view.

## B1-r5: braille (BRF) files read as print. $page is a braille page
## number ("3", "p1"); $n is a number of lines; $reason is an error.

## Beta 4: the Braille view (braille dots at the bottom of the app)
## and the braille grade. $n is the part shown, $total how many.
braille-view-shown = Vista braille mostrada.
braille-view-hidden = Vista braille oculta.
braille-view-not-in-build = La vista braille no está en esta versión de { -brand }.
braille-grade-uncontracted = Braille sin contraer, grado 1.
braille-grade-contracted = Braille contraído, grado 2.
braille-grade-contracted-no-liblouis = Braille contraído, grado 2. Falta liblouis, así que se muestra braille sin contraer hasta que se instale.
settings-unit-cells =
    { $n ->
        [one] celda
       *[other] celdas
    }
gui-braille-view = Vista braille
gui-braille-view-description = Solo visual: la línea actual en puntos braille, para personas videntes que aprenden o enseñan braille. Su lector de pantalla muestra el braille en su línea braille.
gui-braille-caption-uncontracted = Braille sin contraer, { $n } de { $total }
gui-braille-caption-contracted = Braille contraído, { $n } de { $total }
gui-braille-caption-fell-back = Braille sin contraer (el contraído necesita liblouis), { $n } de { $total }
brf-original-title = Braille original, página { $page }
brf-original-intro =
    { $n ->
        [one] Braille original, página { $page }, { $n } línea. Escape cierra.
       *[other] Braille original, página { $page }, { $n } líneas. Escape cierra.
    }
brf-original-not-brf = No es un archivo braille. Mostrar el braille original funciona con archivos BRF.
brf-original-unreadable = No se puede leer el archivo braille: { $reason }
brf-no-liblouis = Braille mostrado como braille: falta liblouis. Para leerlo como texto impreso, instale liblouis desde liblouis.io o los paquetes de su sistema y vuelva a abrir el archivo.
brf-no-liblouis-short = Braille mostrado como braille: falta liblouis. Abra el archivo de nuevo cuando esté instalado.
daisy-headings-only = Solo encabezados: este libro DAISY no tiene texto, solo encabezados y audio, así que se leen sus encabezados.
## End of B1-r5
