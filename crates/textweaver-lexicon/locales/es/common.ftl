### Common words, parts of speech, define word, and names said inside other messages.

### Los mensajes de la interfaz de textweaver, en español.
###
### Fluent (https://projectfluent.org/), en el subconjunto que lee
### textweaver-lexicon. Los identificadores y las variables son los de
### locales/en; un mensaje que falte aquí se dice en inglés.

-brand = textweaver

## Parts of speech.

pos-noun = sustantivo
pos-verb = verbo
pos-adjective = adjetivo
pos-adverb = adverbio

## Define word: sources.

source-glossary = su glosario
source-wordnet = Open English WordNet
source-cmudict = el diccionario de pronunciación CMU

## Define word: the list of senses.

# $word is the word looked up, $n the number of senses, $source a source-* message.
define-title =
    { $n ->
        [0] Pronunciación de { $word }, de { $source }
        [one] Definiciones de { $word }, 1 acepción, de { $source }
       *[other] Definiciones de { $word }, { $n } acepciones, de { $source }
    }
# $lemma is the headword, $pos a pos-* message, $i the sense number, $n how many.
define-sense-head = { $lemma }, { $pos }, { $i } de { $n }
define-sense-head-nopos = { $lemma }, { $i } de { $n }
define-sense = { $head }: { $definition }.
define-example = Por ejemplo: { $text }.
define-synonyms = Sinónimos: { $words }.
define-antonyms = Antónimos: { $words }.
define-kind-of = Un tipo de: { $words }.
# $say is a respelling such as RUN-ing, with the stressed syllable in capitals.
define-pronounced = Se pronuncia { $say }.
define-pronounced-or = Se pronuncia { $say }, o { $other }.

## Common words.

common-cancelled = Cancelado.
# Durations: $h hours, $m minutes, $s seconds.
duration-hours =
    { $h ->
        [one] 1 hora
       *[other] { $h } horas
    } y { $m ->
        [one] 1 minuto
       *[other] { $m } minutos
    }
duration-minutes =
    { $m ->
        [one] 1 minuto
       *[other] { $m } minutos
    } y { $s ->
        [one] 1 segundo
       *[other] { $s } segundos
    }
duration-seconds =
    { $s ->
        [one] 1 segundo
       *[other] { $s } segundos
    }

## Define word in the reader.

# $title is define-title.
define-intro = { $title }. Arriba y Abajo recorren las acepciones, Intro copia una, Escape cierra.
define-nothing-here = No hay ninguna palabra en el cursor.
define-not-found = No se encontró ninguna definición de { $word }.
define-no-dictionary = El archivo del diccionario no está instalado, así que solo se buscó en su glosario. La guía de lectura explica cómo instalarlo.
define-dictionary-damaged = No se pudo leer el archivo del diccionario: { $error } Solo se busca en su glosario.
define-glossary-problem = No se pudo leer su glosario: { $error } Solo se busca en el diccionario.
define-glossary-skipped =
    { $n ->
        [one] 1 línea de su glosario no tiene definición y se omitió.
       *[other] { $n } líneas de su glosario no tienen definición y se omitieron.
    }
define-copied = Copiado.

## Names of structure, units, and modes (said inside other messages).

kind-heading = encabezado
kind-paragraph = párrafo
kind-list-item = elemento de lista
kind-list = lista
kind-table = tabla
kind-row = fila
kind-cell = celda
kind-link = enlace
kind-graphic = gráfico
kind-code = código
kind-quote = cita en bloque
kind-page = página
kind-section = sección
kind-bold = negrita
kind-italic = cursiva
kind-underline = subrayado
kind-footnote = nota al pie
kind-strikethrough = tachado
kind-separator = separador
kind-math = matemáticas
unit-character = carácter
unit-word = palabra
unit-sentence = oración
unit-line = línea
unit-paragraph = párrafo
unit-document = documento
# $what is a kind-* noun, $unit its key.
unit-with-level = { $what } de nivel { $level }
mode-browse = Exploración
mode-speech-cursor = Cursor de lectura
mode-edit = Editar
mode-find = Buscar
mode-command = Comando
mode-go-to = Ir a
mode-open = Abrir
mode-prompt = Preguntar
common-on = activado
common-off = desactivado

## The interface language. $language is the language's name in itself
## (Español), $voice a voice's name.

language-voice-changed = La voz ahora es { $voice }, para { $language }.
language-voice-kept = No hay voz para { $language } en este motor de voz, así que { $voice } sigue hablando.
language-list-title = Idioma
language-list-intro =
    { $n ->
        [one] Idioma, 1 opción. Arriba y Abajo se mueven, Intro elige, Escape conserva el idioma.
       *[other] Idioma, { $n } opciones. Arriba y Abajo se mueven, Intro elige, Escape conserva el idioma.
    }

## Characters and selections, as spoken.

# The name of a white-space character read on its own; $name is a fixed key.
text-char-name =
    { $name ->
        [space] espacio
        [new-line] salto de línea
        [tab] tabulador
        [no-break-space] espacio de no separación
       *[white-space] espacio en blanco
    }
# Said after a selection grows ($change is selected) or shrinks (unselected); $text is the text or a character's name.
text-selection-change =
    { $change ->
        [selected] { $text } seleccionado
       *[unselected] { $text } deseleccionado
    }

text-summary =
    { $change ->
        [selected] { $count } caracteres seleccionados
        [unselected] { $count } caracteres deseleccionados
        [copied] { $count } caracteres copiados
        [cut] { $count } caracteres cortados
       *[deleted] { $count } caracteres eliminados
    }
text-summary-range =
    { $change ->
        [selected] { $count } caracteres seleccionados, desde { $first } hasta { $last }
        [unselected] { $count } caracteres deseleccionados, desde { $first } hasta { $last }
        [copied] { $count } caracteres copiados, desde { $first } hasta { $last }
        [cut] { $count } caracteres cortados, desde { $first } hasta { $last }
       *[deleted] { $count } caracteres eliminados, desde { $first } hasta { $last }
    }
voice-character-keys-on = Atajos de una sola tecla activados.
voice-character-keys-off = Atajos de una sola tecla desactivados.
