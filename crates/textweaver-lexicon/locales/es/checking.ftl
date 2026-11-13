### Markdown lint, grammar, and spelling.

## Markdown lint (edit mode). A problem is said after "Lint: ".

lint-heading-level = encabezado de nivel { $level } después de nivel { $prev }; use el nivel { $use }.
# $reference is the link reference's name.
lint-link-reference = la referencia de enlace { $reference } no tiene definición.
lint-bare-url = dirección web suelta; póngala entre corchetes angulares o conviértala en un enlace con nombre.
# $marker and $used are bullet names: lint-marker-dash and the others.
lint-list-marker = marcador de lista { $marker }; esta lista usa { $used }.
lint-marker-dash = guion
lint-marker-star = asterisco
lint-marker-plus = signo más
lint-marker-other = otro
# $n is how many tabs and spaces there are.
lint-trailing-tabs-empty-line = tabuladores o espacios en una línea vacía.
lint-trailing-tabs-line-end = tabuladores o espacios al final de la línea.
lint-trailing-spaces-empty-line =
    { $n ->
        [one] 1 espacio en una línea vacía.
       *[other] { $n } espacios en una línea vacía.
    }
lint-trailing-spaces-line-end =
    { $n ->
        [one] 1 espacio al final de la línea.
       *[other] { $n } espacios al final de la línea.
    }
# $key turns on edit mode.
lint-not-editing = Lint comprueba el Markdown que escribe. Active primero el modo de edición con { $key }.
lint-not-markdown = Lint comprueba Markdown, y este documento no es Markdown.
lint-none = Sin problemas de lint.
# $count is $n with thousands separators.
lint-no-more =
    { $n ->
        [one] No hay más problemas de lint. 1 problema de lint en total.
       *[other] No hay más problemas de lint. { $count } problemas de lint en total.
    }
lint-no-earlier =
    { $n ->
        [one] No hay problemas de lint anteriores. 1 problema de lint en total.
       *[other] No hay problemas de lint anteriores. { $count } problemas de lint en total.
    }
# $message is one of the problems above.
lint-said = Lint: { $message }
# Added at high verbosity.
common-line = Línea { $line }.

## Grammar checking (Harper). $message is Harper's own message, in English.

# $words are the words the problem is about.
grammar-said = Gramática: { $message } Las palabras: { $words }.
# Said after grammar-said when the first fix removes the words.
grammar-fix-remove = Corrección: quitarlas.
grammar-fix = Corrección: { $fix }.
# A fix in the fixes list that removes the words.
grammar-remove-the-words = Quitar las palabras
grammar-none = No se encontraron problemas de gramática.
# $count is $n with thousands separators.
grammar-no-more =
    { $n ->
        [one] No hay más problemas de gramática. 1 problema de gramática en total.
       *[other] No hay más problemas de gramática. { $count } problemas de gramática en total.
    }
grammar-no-earlier =
    { $n ->
        [one] No hay problemas de gramática anteriores. 1 problema de gramática en total.
       *[other] No hay problemas de gramática anteriores. { $count } problemas de gramática en total.
    }
# $key opens the fixes list.
grammar-lists-fixes = { $key } lista las correcciones.
# Added at high verbosity.
# $described is grammar-said (and its fix) without the last full stop.
grammar-no-fix = { $described } No hay ninguna corrección que ofrecer.
grammar-fixes =
    { $n ->
        [one] { $words }: 1 corrección.
       *[other] { $words }: { $n } correcciones.
    }
grammar-fixes-edit =
    { $n ->
        [one] { $words }: 1 corrección. Intro hace el cambio.
       *[other] { $words }: { $n } correcciones. Intro hace el cambio.
    }
common-left-as-is = Se dejó tal cual.
# $fix is the fix chosen; $key turns on edit mode.
grammar-fix-not-editing = { $fix }. Active el modo de edición con { $key } para cambiar el texto.
grammar-removed = Quitado.
grammar-changed = Cambiado a { $fix }.
grammar-change-failed = No se pudo cambiar el texto: { $error } No se cambió nada.

## Spell checking.

# Said for an apostrophe when a word is spelled out letter by letter.
spell-apostrophe = apóstrofo
spell-not-available = La comprobación ortográfica no está disponible: esta compilación no tiene lista de palabras.
spell-none-found = No se encontraron errores ortográficos.
# $count is $n with thousands separators.
spell-no-more =
    { $n ->
        [one] No hay más errores. 1 posible error ortográfico en total.
       *[other] No hay más errores. { $count } posibles errores ortográficos en total.
    }
spell-no-earlier =
    { $n ->
        [one] No hay errores anteriores. 1 posible error ortográfico en total.
       *[other] No hay errores anteriores. { $count } posibles errores ortográficos en total.
    }
# Added at high verbosity.
spell-no-misspelled-word = No hay ninguna palabra mal escrita en el cursor.
# $word is the misspelled word; $n how many suggestions follow.
spell-suggestions =
    { $n ->
        [0] { $word }: sin sugerencias.
        [one] { $word }: 1 sugerencia.
       *[other] { $word }: { $n } sugerencias.
    }
spell-suggestions-edit =
    { $n ->
        [0] { $word }: sin sugerencias. Intro reemplaza la palabra.
        [one] { $word }: 1 sugerencia. Intro reemplaza la palabra.
       *[other] { $word }: { $n } sugerencias. Intro reemplaza la palabra.
    }
# $word is the suggestion chosen; $key turns on edit mode.
spell-replace-not-editing = { $word }. Active el modo de edición con { $key } para cambiar el texto.
spell-replaced = Reemplazado con { $word }.
spell-replace-failed = No se pudo reemplazar: { $error } La palabra no ha cambiado.
spell-added-for-session = Se agregó { $word } a su lista de palabras para esta sesión.
spell-added = Se agregó { $word } a su lista de palabras.
spell-save-failed = No se pudo guardar su lista de palabras: { $error } La palabra se reconoce hasta que salga.
# After a save; $count is $n with thousands separators.
spell-count =
    { $n ->
        [0] Sin errores ortográficos.
        [one] 1 posible error ortográfico.
       *[other] { $count } posibles errores ortográficos.
    }
