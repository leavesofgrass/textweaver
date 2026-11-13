### Braille: BRF files read as print, and the Braille view.

## B1-r5: braille (BRF) files read as print. $page is a braille page
## number ("3", "p1"); $n is a number of lines; $reason is an error.

## Beta 4: the Braille view (braille dots at the bottom of the app)
## and the braille grade. $n is the part shown, $total how many.
braille-view-shown = Visualização braille exibida.
braille-view-hidden = Visualização braille oculta.
braille-view-not-in-build = A visualização braille não está nesta versão do { -brand }.
braille-grade-uncontracted = Braille integral, grau 1.
braille-grade-contracted = Braille abreviado, grau 2.
braille-grade-contracted-no-liblouis = Braille abreviado, grau 2. O liblouis está faltando, então o braille integral é mostrado até que seja instalado.
settings-unit-cells =
    { $n ->
        [one] cela
       *[other] celas
    }
gui-braille-view = Visualização braille
gui-braille-view-description = Apenas visual: a linha atual em pontos braille, para pessoas que enxergam e aprendem ou ensinam braille. O seu leitor de tela mostra o braille na sua linha braille.
gui-braille-caption-uncontracted = Braille integral, { $n } de { $total }
gui-braille-caption-contracted = Braille abreviado, { $n } de { $total }
gui-braille-caption-fell-back = Braille integral (o abreviado precisa do liblouis), { $n } de { $total }
brf-original-title = Braille original, página { $page }
brf-original-intro =
    { $n ->
        [one] Braille original, página { $page }, { $n } linha. Escape fecha.
       *[other] Braille original, página { $page }, { $n } linhas. Escape fecha.
    }
brf-original-not-brf = Não é um arquivo braille. Mostrar o braille original funciona com arquivos BRF.
brf-original-unreadable = Não é possível ler o arquivo braille: { $reason }
brf-no-liblouis = Braille mostrado como braille: falta o liblouis. Para ler como texto impresso, instale o liblouis de liblouis.io ou dos pacotes do seu sistema e abra o arquivo de novo.
brf-no-liblouis-short = Braille mostrado como braille: falta o liblouis. Abra o arquivo de novo quando estiver instalado.
daisy-headings-only = Só títulos: este livro DAISY não tem texto, apenas títulos e áudio, então seus títulos são lidos.
## End of B1-r5
