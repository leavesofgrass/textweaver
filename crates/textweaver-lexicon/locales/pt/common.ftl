### Common words, parts of speech, define word, and names said inside other messages.

### As mensagens da interface do textweaver, em português.
###
### Fluent (https://projectfluent.org/), no subconjunto que o
### textweaver-lexicon lê. Os identificadores e as variáveis são os de
### locales/en; uma mensagem que falte aqui é dita em inglês.

-brand = textweaver

## Parts of speech.

pos-noun = substantivo
pos-verb = verbo
pos-adjective = adjetivo
pos-adverb = advérbio

## Define word: sources.

source-glossary = seu glossário
source-wordnet = Open English WordNet
source-cmudict = o CMU Pronouncing Dictionary

## Define word: the list of senses.

# $word is the word looked up, $n the number of senses, $source a source-* message.
define-title =
    { $n ->
        [0] Pronúncia de { $word }, de { $source }
        [one] Definições de { $word }, 1 sentido, de { $source }
       *[other] Definições de { $word }, { $n } sentidos, de { $source }
    }
# $lemma is the headword, $pos a pos-* message, $i the sense number, $n how many.
define-sense-head = { $lemma }, { $pos }, { $i } de { $n }
define-sense-head-nopos = { $lemma }, { $i } de { $n }
define-sense = { $head }: { $definition }.
define-example = Por exemplo: { $text }.
define-synonyms = Sinônimos: { $words }.
define-antonyms = Oposto: { $words }.
define-kind-of = Um tipo de: { $words }.
# $say is a respelling such as RUN-ing, with the stressed syllable in capitals.
define-pronounced = Pronunciado { $say }.
define-pronounced-or = Pronunciado { $say }, ou { $other }.

## Common words.

common-cancelled = Cancelado.
# Durations: $h hours, $m minutes, $s seconds.
duration-hours =
    { $h ->
        [one] 1 hora
       *[other] { $h } horas
    } e { $m ->
        [one] 1 minuto
       *[other] { $m } minutos
    }
duration-minutes =
    { $m ->
        [one] 1 minuto
       *[other] { $m } minutos
    } e { $s ->
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
define-intro = { $title }. Seta para cima e para baixo percorrem os sentidos, Enter copia um, Escape fecha.
define-nothing-here = Não há palavra no cursor.
define-not-found = Nenhuma definição encontrada para { $word }.
define-no-dictionary = O arquivo do dicionário não está instalado, então apenas seu glossário foi pesquisado. O guia de leitura explica como instalá-lo.
define-dictionary-damaged = Não foi possível ler o arquivo do dicionário: { $error } Só o seu glossário é pesquisado.
define-glossary-problem = Não foi possível ler seu glossário: { $error } Só o dicionário é pesquisado.
define-glossary-skipped =
    { $n ->
        [one] 1 linha do seu glossário não tem definição e foi ignorada.
       *[other] { $n } linhas do seu glossário não têm definição e foram ignoradas.
    }
define-copied = Copiado.

## Names of structure, units, and modes (said inside other messages).

kind-heading = cabeçalho
kind-paragraph = parágrafo
kind-list-item = item de lista
kind-list = lista
kind-table = tabela
kind-row = linha
kind-cell = célula
kind-link = link
kind-graphic = imagem
kind-code = código
kind-quote = citação
kind-page = página
kind-section = seção
kind-bold = negrito
kind-italic = itálico
kind-underline = sublinhado
kind-footnote = nota de rodapé
kind-strikethrough = tachado
kind-separator = separador
kind-math = matemática
unit-character = caractere
unit-word = palavra
unit-sentence = frase
unit-line = linha
unit-paragraph = parágrafo
unit-document = documento
# $what is a kind-* noun, $unit its key.
unit-with-level = { $what } nível { $level }
mode-browse = Navegar
mode-speech-cursor = Cursor de leitura
mode-edit = Editar
mode-find = Localizar
mode-command = Comando
mode-go-to = Ir para
mode-open = Abrir
mode-prompt = Prompt
common-on = ligado
common-off = desligado

## The interface language. $language is the language's name in itself
## (Español), $voice a voice's name.

language-voice-changed = A voz agora é { $voice }, para { $language }.
language-voice-kept = Não há voz para { $language } neste motor de fala, então { $voice } continua falando.
language-list-title = Idioma
language-list-intro =
    { $n ->
        [one] Idioma, 1 opção. Seta para cima e para baixo move, Enter escolhe, Escape mantém o idioma.
       *[other] Idioma, { $n } opções. Seta para cima e para baixo move, Enter escolhe, Escape mantém o idioma.
    }

## Characters and selections, as spoken.

# The name of a white-space character read on its own; $name is a fixed key.
text-char-name =
    { $name ->
        [space] espaço
        [new-line] nova linha
        [tab] tabulação
        [no-break-space] espaço inseparável
       *[white-space] espaço em branco
    }
# Said after a selection grows ($change is selected) or shrinks (unselected); $text is the text or a character's name.
text-selection-change =
    { $change ->
        [selected] { $text } selecionado
       *[unselected] { $text } não selecionado
    }

text-summary =
    { $change ->
        [selected] { $count } caracteres selecionados
        [unselected] { $count } caracteres desmarcados
        [copied] { $count } caracteres copiados
        [cut] { $count } caracteres recortados
       *[deleted] { $count } caracteres excluídos
    }
text-summary-range =
    { $change ->
        [selected] { $count } caracteres selecionados, de { $first } a { $last }
        [unselected] { $count } caracteres desmarcados, de { $first } a { $last }
        [copied] { $count } caracteres copiados, de { $first } a { $last }
        [cut] { $count } caracteres recortados, de { $first } a { $last }
       *[deleted] { $count } caracteres excluídos, de { $first } a { $last }
    }
voice-character-keys-on = Atalhos de tecla única ativados.
voice-character-keys-off = Atalhos de tecla única desativados.
