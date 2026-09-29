### As mensagens da interface do textweaver, em português.
###
### Fluent (https://projectfluent.org/), no subconjunto que o
### textweaver-lexicon lê. Os identificadores e as variáveis são os de
### en.ftl; uma mensagem que falte aqui é dita em inglês.

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

## Prompts.

prompt-define-word = Definir qual palavra?
prompt-profile-name = Nome do novo perfil
prompt-profile-rename = Novo nome do perfil, Enter mantém
prompt-profiles-import = Importar perfis de um arquivo
prompt-profiles-export = Exportar perfis para um arquivo, por exemplo textweaver-profiles.json

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
define-dictionary-damaged = Não foi possível ler o arquivo do dicionário: { $error }
define-glossary-problem = Não foi possível ler seu glossário: { $error }
define-glossary-skipped =
    { $n ->
        [one] 1 linha do seu glossário não tem definição e foi ignorada.
       *[other] { $n } linhas do seu glossário não têm definição e foram ignoradas.
    }
define-copied = Copiado.

## Settings profiles.

profiles-title =
    { $n ->
        [0] Perfis de configurações, nenhum salvo ainda
        [one] Perfis de configurações, 1 perfil
       *[other] Perfis de configurações, { $n } perfis
    }
# $title is profiles-title.
profiles-intro = { $title }. Enter muda para um perfil, F2 renomeia, Delete exclui.
# $summary is profile-summary-* parts joined by commas.
profiles-item = { $name }: { $summary }
profiles-item-active = { $name }, em uso: { $summary }
profiles-save-new = Salvar as configurações atuais como um novo perfil
profiles-update = Salvar as configurações atuais em { $name }
profiles-import = Importar perfis de um arquivo
profiles-export = Exportar todos os perfis para um arquivo
profile-summary-voice = voz { $voice }
profile-summary-rate = velocidade { $rate }
profile-summary-theme = tema { $theme }
# $mode is self voicing, hybrid, or screen reader.
profile-summary-access = modo { $mode }
profile-summary-empty = nada salvo
profile-switched = Mudou para { $name }.
profile-switched-backend = Mudou para { $name }. O motor de fala dele é usado a partir do próximo início.
# $keys lists settings such as speech.pitch.
profile-dropped =
    { $n ->
        [one] 1 configuração dele não é usada nesta versão: { $keys }.
       *[other] { $n } configurações dele não são usadas nesta versão: { $keys }.
    }
profile-saved = Salvou as configurações atuais como { $name }.
profile-replaced = Salvou as configurações atuais em { $name }.
profile-renamed = Renomeou { $old } para { $new }.
profile-delete-question = Excluir o perfil { $name }? y ou n
profile-deleted = Excluiu { $name }.
profile-kept = Mantido.
profile-not-found = Não há nenhum perfil chamado { $name }.
profile-needs-name = Um perfil precisa de um nome.
profile-exists = Já existe um perfil chamado { $name }.
profiles-not-an-export = { $detail }
profiles-no-persistence = Os perfis não são salvos nesta sessão.
profiles-read-failed = Não foi possível ler o arquivo de perfis, então ele é tratado como vazio: { $error }
profiles-save-failed = Não foi possível salvar os perfis: { $error }
profiles-none-to-export = Ainda não há perfis para exportar.
profiles-exported =
    { $n ->
        [one] Exportou 1 perfil para { $file }.
       *[other] Exportou { $n } perfis para { $file }.
    }
profiles-export-failed = Não foi possível exportar os perfis: { $error }
profiles-imported =
    { $n ->
        [0] Não havia perfis em { $file }.
        [one] Importou 1 perfil de { $file }: { $names }.
       *[other] Importou { $n } perfis de { $file }: { $names }.
    }

## Reading statistics.

stats-title = Estatísticas de leitura
stats-intro = Estatísticas de leitura. Enter em um documento o abre.
stats-off = As estatísticas de leitura estão desligadas. O último item as liga.
stats-empty = Nenhuma leitura registrada ainda. O tempo é contado enquanto o textweaver lê em voz alta.
# $time is a duration-* message.
stats-total =
    { $time } lido no total, em { $sessions ->
        [one] 1 sessão
       *[other] { $sessions } sessões
    }, em { $docs ->
        [one] 1 documento
       *[other] { $docs } documentos
    }.
stats-current =
    Este documento: { $time } lido, ponto mais distante { $pct } por cento, { $sessions ->
        [one] 1 sessão
       *[other] { $sessions } sessões
    }.
stats-current-none = Este documento ainda não foi lido em voz alta.
stats-most-read = Mais lido { $rank }: { $title }, { $time }, ponto mais distante { $pct } por cento.
stats-toggle-on = As estatísticas estão ligadas. Enter as desliga.
stats-toggle-off = As estatísticas estão desligadas. Enter as liga.
stats-turned-on = As estatísticas de leitura estão ligadas.
stats-turned-off = As estatísticas de leitura estão desligadas. O que foi registrado é mantido; tw stats --clear o remove.

## Lists.

study-nothing-to-delete = Nada para excluir nesta lista.
study-nothing-to-rename = Nada para renomear nesta lista.
## tw stats.

stats-clear-question =
    { $n ->
        [one] Remover as estatísticas de leitura de 1 documento? y ou n
       *[other] Remover as estatísticas de leitura de { $n } documentos? y ou n
    }
stats-cleared = Estatísticas de leitura removidas.
stats-off-cli = As estatísticas de leitura estão desligadas: stats.enabled é false nas configurações.

## Navigation. $dir is next or previous; $what is a kind-* or unit-*
## noun and $unit its key (heading, list-item, sentence), for languages
## whose words agree with the noun.

nav-blank = em branco
# High verbosity: $label is a structure label ("Heading level 2").
nav-message-at-labelled = { $label }, linha { $line }, { $pct } por cento: { $content }
nav-message-at = Linha { $line }, { $pct } por cento: { $content }
nav-message-labelled = { $label }: { $content }
# $message is the navigation message after wrapping around.
nav-wrapped = Voltou ao início. { $message }
nav-no-next =
    { $unit ->
        [list] Sem próxima { $what }.
        [table] Sem próxima { $what }.
        [row] Sem próxima { $what }.
        [cell] Sem próxima { $what }.
        [graphic] Sem próxima { $what }.
        [quote] Sem próxima { $what }.
        [page] Sem próxima { $what }.
        [section] Sem próxima { $what }.
        [footnote] Sem próxima { $what }.
        [math] Sem próxima { $what }.
        [word] Sem próxima { $what }.
        [sentence] Sem próxima { $what }.
        [line] Sem próxima { $what }.
       *[other] Sem próximo { $what }.
    }
nav-no-previous = Sem { $what } anterior.
nav-nothing-to-read = Nenhum { $what } para ler.
nav-label-heading-level = Nível de cabeçalho { $level }
nav-label-list =
    { $n ->
        [0] Lista
        [one] Lista, 1 item
       *[other] Lista, { $n } itens
    }
nav-label-table =
    { $n ->
        [0] Tabela
        [one] Tabela, 1 linha
       *[other] Tabela, { $n } linhas
    }
nav-label-list-item-level = Item de lista, nível { $level }
nav-no-heading-level =
    { $dir ->
        [next] Sem próximo cabeçalho no nível { $level }.
       *[previous] Sem cabeçalho anterior no nível { $level }.
    }
# Said before a line's text on caret moves.
nav-line-heading = cabeçalho nível { $level }
nav-line-row = linha { $n }
nav-line-list-item-level = item de lista, nível { $level }
# $language is a programming language name such as Python.
nav-line-code-language = código, { $language }
nav-no-chapters = Este documento não tem capítulos.
nav-chapter = Capítulo
nav-no-chapter =
    { $dir ->
        [next] Sem próximo capítulo.
       *[previous] Sem capítulo anterior.
    }
nav-back = Voltar
nav-forward = Avançar
nav-page = Página
nav-percent = { $pct } por cento
nav-no-earlier-history = Sem histórico anterior.
nav-no-forward-history = Sem histórico à frente.
# $label is nav-back, nav-forward, nav-page, or nav-percent.
nav-label-line = { $label }, linha { $line }
nav-top-of-document = Início do documento
nav-end-of-document = Fim do documento
# $edge is nav-top-of-document or nav-end-of-document; $content the line there.
nav-edge-message = { $edge }. { $content }
nav-top-of-document-stop = Início do documento.
nav-end-of-document-stop = Fim do documento.
nav-position = Linha { $line } de { $lines }, { $pct } por cento.
nav-position-word = Palavra { $word } de { $words }.
nav-position-heading = Sob o cabeçalho { $heading }.
nav-position-mode = Modo { $mode }.

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

## Reading aloud.

playback-caps-no-words = Esta voz não informa as palavras, então o realce de palavra é estimado.
playback-caps-words = Esta voz informa cada palavra, então o realce a acompanha exatamente.
playback-caps-no-pitch = O tom não pode ser alterado com esta voz.
playback-caps-no-volume = O volume não pode ser alterado com esta voz.
# The reading state on the title line, one word.
state-reading = Leitura
state-paused = Em pausa
state-stopped = Parado
state-ready = Pronto
playback-reading-at = Lendo a { $rate } palavras por minuto.
playback-paused = Em pausa.
playback-stopped-speech-cursor-off = Parado. Cursor de leitura desligado.
playback-stopped = Parado.
playback-search-cleared = Busca limpa.
# $key is the key that turns edit mode off.
playback-still-editing = Ainda editando. { $key } termina.
playback-end-of-document-content = fim do documento
playback-no-unit-here = Nenhum { $what } aqui.
playback-no-selection = Nenhuma seleção.
# $next is what happens now (a restart, or nothing).
playback-speech-died = A fala parou de funcionar ({ $reason }). { $next }
playback-done-reading = Leitura concluída.
playback-speech-restarted = Fala reiniciada: { $reason }. Continuando a leitura a partir da última palavra.
playback-speech-error = Erro de fala: { $error }

## The title line and Say Status.

# The title line's position; % is shown, not said.
status-position = linha { $line } de { $lines }, { $pct }%
status-mode = Modo { $mode }
status-modified = modificado
status-self-voicing = autofala
status-hybrid = híbrido
status-screen-reader = modo leitor de tela
status-rate-spoken = { $wpm } palavras por minuto
status-rate = { $wpm } ppm
status-no-document = Sem documento
# $parts are the title line's parts, joined with commas.
status-said = { $title }: { $parts }.
status-no-message = Nenhuma mensagem ainda.
# A list shown without its own introduction.
status-list-intro =
    { $n ->
        [one] { $title }, 1 item. Seta para cima e para baixo move, Enter escolhe, Escape fecha.
       *[other] { $title }, { $n } itens. Seta para cima e para baixo move, Enter escolhe, Escape fecha.
    }

## Questions and answers. Keep the letters y and n: they are the keys
## that answer.

common-press-y-or-n = Pressione y ou n.
common-kept = Mantido.
confirm-quit = Sair do textweaver? y ou n
confirm-delete-note = Excluir esta nota ou realce? y ou n
notes-remove-highlight-question = Remover este realce? y ou n
notes-delete-note-question = Excluir esta nota? y ou n
list-nothing-to-mark = Nada para marcar nesta lista.
notes-editing = Editando nota: { $text }
# $key opens a document.
app-no-document-open = Nenhum documento está aberto. Pressione { $key } para abrir um.
app-window-only = Este comando funciona na janela do textweaver.
settings-save-failed = Não foi possível salvar as configurações: { $error }
edit-still-editing = Ainda editando.
goto-not-a-target = Não é um destino válido: { $text }. Digite um número de linha, uma porcentagem como 50%, início ou fim.

## Opening a document.

open-opened = Abriu { $title }.
open-resumed = Abriu { $title }. Retomado em { $pct } por cento.
open-resumed-synced = Abriu { $title }. Retomado em { $pct } por cento, de outro dispositivo.
open-resumed-conflict = Abriu { $title }. Retomado em { $pct } por cento. Outro dispositivo está em um ponto diferente; manteve o deste dispositivo.

## Prompts: the label is shown and said when the prompt opens.

prompt-find = Localizar
prompt-go-to = Ir para linha, porcentagem, início ou fim
prompt-open = Abrir arquivo
prompt-command = Comando
# $label is prompt-command.
prompt-command-palette-intro = { $label }. Digite parte de um nome; Tab completa, Seta para cima e para baixo listam correspondências.
prompt-save-as = Salvar como
prompt-table-size = Tamanho da tabela, colunas por linhas, por exemplo 3 por 2
prompt-image-path = Arquivo de imagem
prompt-replace-find = Substituir, localizar o quê
prompt-replace-with = Substituir por
prompt-note = Nota
prompt-edit-note = Editar nota, Enter mantém
prompt-rename-bookmark = Novo nome do marcador, Enter mantém
prompt-export-settings = Exportar configurações para um arquivo, por exemplo textweaver-settings.json
prompt-import-settings = Importar configurações de um arquivo
prompt-citation-locator = Página ou outro localizador, por exemplo 12 ou capítulo 2; Enter para nenhum
prompt-reference-identifier = DOI ou ISBN para adicionar
prompt-import-references = Importar referências de um arquivo
prompt-template-title = Título do novo documento
prompt-setting-value = Novo valor, Enter mantém

## Keys named in messages and the help.

help-the-command-palette = a paleta de comandos
help-not-bound = não atribuída
# Two keys, or a key and a list of keys: "p or Ctrl+P".
help-or = { $a } ou { $b }
# A command without keys: $name is its palette name, such as list highlights.
help-the-command = o comando { $name }
# One line of the keyboard shortcuts list: a category-* title, an action-*
# help, and its keys.
help-entry = { $category }: { $help }. { $keys }
# One command palette candidate: its id (not translated), help, and keys.
help-palette-item = { $id }: { $help }. { $keys }
help-unknown-command = Comando desconhecido: { $text }.
help-shortcuts-intro = Atalhos de teclado, { $n } comandos. Seta para cima e para baixo move, Enter executa, Escape fecha.
help-shortcuts-title = Atalhos de teclado
help-title = Ajuda
help-intro = Ajuda. Seta para cima e para baixo move, Escape fecha.

## The help list. Each value is a key or keys from the keymap.

help-about = O textweaver lê documentos em voz alta. As teclas abaixo são as atribuições atuais.
help-open = Abrir um documento: { $open }. Biblioteca e arquivos recentes: { $library }.
help-play = Reproduzir ou pausar: { $key }.
help-read-from-cursor = Ler a partir do cursor: { $key }.
help-stop = Parar: { $key }.
help-sentences = Próxima e anterior frase: { $next } e { $previous }.
help-paragraphs = Próximo e anterior parágrafo: { $next } e { $previous }.
help-headings = Próximo e anterior cabeçalho: { $next } e { $previous }. Cabeçalho em um nível: { $first } a { $last }, com Shift para o anterior.
help-read-headings = Ler a partir do próximo e do anterior cabeçalho: { $next } e { $previous }.
help-quick-keys = Teclas rápidas, como no NVDA e no JAWS: lista { $list }, item de lista { $item }, tabela { $table }, link { $link }, citação { $quote }, separador { $separator }, imagem { $graphic }, seção { $section }. Shift com a tecla vai para o anterior.
help-speech-cursor = Cursor de leitura, linha por linha: { $key }.
help-find = Localizar: { $key }.
help-bookmark = Adicionar um marcador: { $key }.
help-history = Voltar e avançar pelos seus saltos: { $back } e { $forward }.
help-rate = Mais rápido e mais devagar: { $faster } e { $slower }.
help-where = Onde estou: { $key }.
help-repeat = Ouvir a última mensagem novamente: { $repeat }. A última mensagem e o status: modo, velocidade, motor e posição: { $status }.
help-notes = Notas: adicionar { $add }, listar { $list }, próxima e anterior { $next } e { $previous }, excluir a do cursor { $delete }. Na lista, Delete exclui e F2 edita.
help-highlights = Realçar a seleção ou a frase, ou remover um realce: { $highlight }. Listar realces: { $list }.
help-bookmarks-list = Lista de marcadores: Delete exclui um marcador, F2 o renomeia.
help-edit = Editar o documento: { $edit }. Salvar: { $save }. Salvar como: { $saveas }. Novo documento: { $new }.
help-editing = Durante a edição: desfazer { $undo }, refazer { $redo }, negrito { $bold }. Todo comando de formatação está nos atalhos de teclado.
help-outline = Estrutura dos cabeçalhos, digite para filtrar: { $outline }. Seguir um link ou nota de rodapé: { $follow }.
help-tables = Tabelas: { $nextrow } e { $previousrow } movem por linha, { $nextcell } e { $previouscell } por célula.
help-citations = Citações durante a edição: inserir { $insert }, adicionar uma referência por DOI ou ISBN { $reference }. Ortografia: próximo e anterior erro { $next } e { $previous }, sugestões { $suggestions }.
help-export = Exportar para HTML, PDF, Word, EPUB ou braille, pré-visualizar no navegador, e começar a partir de um modelo: digite export, preview ou template na paleta de comandos.
help-verbosity = Quanto é dito: { $verbosity }. Quanta pontuação: { $punctuation }.
help-voice = Escolher uma voz: { $voice }. Reiniciar a fala se ela parar: { $restart }.
help-access = Com um leitor de tela, quem fala: { $key } alterna entre autofala, híbrido e modo leitor de tela.
help-character-keys = Atalhos de tecla única ativados ou desativados, para ditado: { $keys }. Configurações: { $settings }.
help-all-shortcuts = Todos os atalhos de teclado: { $key }.
help-palette = Executar qualquer comando pelo nome: { $key }.
# Keep the letters y, n, and a: they are the keys that answer.
help-quit = Sair, salvando seu lugar: { $key }, depois y para confirmar; n, a ou Escape cancela.

## Help categories.

category-reading = Leitura
category-navigation = Navegação
category-speech-cursor = Cursor de leitura
category-voice = Voz
category-search = Busca
category-bookmarks = Marcadores e notas
category-file = Arquivo
category-editing = Edição
category-view = Exibição e ajuda

## Key names as textweaver's own voice says them. Written names (Ctrl+S)
## are not translated.

keyname-control = Control
keyname-command = Command
keyname-alt = Alt
keyname-shift = Shift
keyname-period = ponto
keyname-comma = vírgula
keyname-semicolon = ponto e vírgula
keyname-colon = dois pontos
keyname-apostrophe = apóstrofo
keyname-quote = aspas
keyname-grave-accent = acento grave
keyname-tilde = til
keyname-exclamation-mark = ponto de exclamação
keyname-question-mark = ponto de interrogação
keyname-at-sign = arroba
keyname-number-sign = cerquilha
keyname-dollar-sign = cifrão
keyname-percent = porcentagem
keyname-caret = circunflexo
keyname-ampersand = comercial e
keyname-asterisk = asterisco
keyname-left-parenthesis = abre parêntese
keyname-right-parenthesis = fecha parêntese
keyname-left-bracket = abre colchete
keyname-right-bracket = fecha colchete
keyname-left-brace = abre chave
keyname-right-brace = fecha chave
keyname-less-than = menor que
keyname-greater-than = maior que
keyname-plus = mais
keyname-minus = menos
keyname-equals = igual
keyname-underscore = sublinhado
keyname-slash = barra
keyname-backslash = barra invertida
keyname-vertical-bar = barra vertical
keyname-page-up = Page Up
keyname-page-down = Page Down
keyname-up-arrow = Seta para cima
keyname-down-arrow = Seta para baixo
keyname-left-arrow = Seta para a esquerda
keyname-right-arrow = Seta para a direita
keyname-escape = Escape
keyname-space = Espaço
keyname-enter = Enter
keyname-tab = Tab
keyname-backspace = Backspace
keyname-delete = Excluir
keyname-insert = Insert
keyname-home = Home
keyname-end = End

## Commands: the one-line help of each, in the keyboard shortcuts list and
## the command palette. Their ids (play_pause) stay as they are.

action-play-pause = Reproduzir ou pausar a leitura a partir da palavra atual
action-stop = Parar a leitura
action-read-from-cursor = Ler continuamente a partir do cursor
action-read-document = Ler o documento inteiro desde o início
action-read-current-character = Dizer o caractere no cursor
action-read-current-word = Dizer a palavra no cursor
action-read-current-sentence = Dizer a frase no cursor sem mover
action-read-current-line = Dizer a linha no cursor
action-read-paragraph = Dizer o parágrafo no cursor sem mover
action-read-selection = Ler o texto selecionado
action-say-position = Dizer a posição: linha, porcentagem, número da palavra e cabeçalho
action-say-status = Dizer a última mensagem novamente, depois o status: modo, estado da leitura, posição, velocidade e motor de fala; em uma lista, a introdução da lista
action-repeat-message = Dizer a última mensagem novamente
action-word-count = Dizer quantas palavras há no documento, ou na seleção
action-link-address = Dizer o endereço do link no cursor
action-replay-sentence = Ler novamente desde o início da frase atual
action-replay-paragraph = Ler novamente desde o início do parágrafo atual
action-rsvp-toggle = Mostrar ou ocultar o RSVP: uma palavra de cada vez, a partir do cursor
action-rsvp-play-pause = Iniciar ou pausar o RSVP
action-rsvp-faster = RSVP mais rápido
action-rsvp-slower = RSVP mais devagar
action-rsvp-position-next = Mover a palavra do RSVP para o próximo lugar na tela
action-reading-level = Dizer o nível de leitura do documento ou da seleção
action-define-word = Definir a palavra no cursor, ou as palavras selecionadas: sentidos, exemplos, sinônimos e pronúncia
action-toggle-citations = Ligar ou desligar as citações na leitura contínua: desligado as ignora, ligado as diz por extenso
action-explore-math = Explorar a matemática no cursor, termo por termo: as setas movem, para baixo entra em uma parte, para cima sai, Escape sai
action-listen-rendered = Ouvir o documento como ele será renderizado, sem sair do modo de edição
action-next-sentence = Mover para a próxima frase
action-previous-sentence = Mover para a frase anterior, ou para o início desta quando já se passaram mais de três palavras
action-next-paragraph = Mover para o próximo parágrafo
action-previous-paragraph = Mover para o parágrafo anterior
action-next-heading = Ler a partir do próximo cabeçalho
action-previous-heading = Ler a partir do cabeçalho anterior
action-skip-next-heading = Mover para o próximo cabeçalho sem ler
action-skip-previous-heading = Mover para o cabeçalho anterior sem ler
action-outline = Listar os cabeçalhos: digite para filtrar, Enter vai até um
action-next-heading-level-1 = Mover para o próximo cabeçalho no nível 1
action-next-heading-level-2 = Mover para o próximo cabeçalho no nível 2
action-next-heading-level-3 = Mover para o próximo cabeçalho no nível 3
action-next-heading-level-4 = Mover para o próximo cabeçalho no nível 4
action-next-heading-level-5 = Mover para o próximo cabeçalho no nível 5
action-next-heading-level-6 = Mover para o próximo cabeçalho no nível 6
action-previous-heading-level-1 = Mover para o cabeçalho anterior no nível 1
action-previous-heading-level-2 = Mover para o cabeçalho anterior no nível 2
action-previous-heading-level-3 = Mover para o cabeçalho anterior no nível 3
action-previous-heading-level-4 = Mover para o cabeçalho anterior no nível 4
action-previous-heading-level-5 = Mover para o cabeçalho anterior no nível 5
action-previous-heading-level-6 = Mover para o cabeçalho anterior no nível 6
action-next-table = Mover para a próxima tabela
action-previous-table = Mover para a tabela anterior
action-next-list = Mover para a próxima lista
action-previous-list = Mover para a lista anterior
action-next-list-item = Mover para o próximo item de lista
action-previous-list-item = Mover para o item de lista anterior
action-next-link = Mover para o próximo link
action-previous-link = Mover para o link anterior
action-next-block-quote = Mover para a próxima citação
action-previous-block-quote = Mover para a citação anterior
action-next-separator = Mover para o próximo separador (linha horizontal)
action-previous-separator = Mover para o separador anterior (linha horizontal)
action-next-graphic = Mover para a próxima imagem
action-previous-graphic = Mover para a imagem anterior
action-follow-link = Seguir o link no cursor, ou ir entre uma nota de rodapé e sua nota
action-table-next-row = Em uma tabela, mover uma linha para baixo na mesma coluna
action-table-previous-row = Em uma tabela, mover uma linha para cima na mesma coluna
action-table-next-column = Em uma tabela, mover para a próxima célula na linha
action-table-previous-column = Em uma tabela, mover para a célula anterior na linha
action-next-chapter = Mover para o próximo capítulo ou seção
action-previous-chapter = Mover para o capítulo ou seção anterior
action-history-back = Voltar para onde você estava antes do último salto
action-history-forward = Avançar novamente depois de voltar
action-go-to = Ir para uma linha, porcentagem ou posição
action-document-start = Mover para o início do documento
action-document-end = Mover para o fim do documento
action-caret-next-word = Mover o cursor para a próxima palavra
action-caret-previous-word = Mover o cursor para a palavra anterior
action-caret-next-line = Mover o cursor para a próxima linha
action-caret-previous-line = Mover o cursor para a linha anterior
action-select-next-word = Estender a seleção até a próxima palavra
action-select-previous-word = Estender a seleção até a palavra anterior
action-select-next-line = Estender a seleção até a próxima linha
action-select-previous-line = Estender a seleção até a linha anterior
action-page-down = Mover uma tela para baixo
action-page-up = Mover uma tela para cima
action-scroll-down = Rolar uma linha para baixo sem mover o cursor
action-scroll-up = Rolar uma linha para cima sem mover o cursor
action-speech-cursor-toggle = Entrar ou sair do modo Cursor de leitura (linha)
action-speech-cursor-next-line = Cursor de leitura: ler a próxima linha
action-speech-cursor-previous-line = Cursor de leitura: ler a linha anterior
action-speech-cursor-reread-line = Cursor de leitura: ler a linha atual novamente
action-speech-cursor-exit-and-read = Cursor de leitura: sair e continuar lendo a partir desta linha
action-rate-up = Falar mais rápido
action-rate-down = Falar mais devagar
action-pitch-up = Aumentar o tom
action-pitch-down = Diminuir o tom
action-volume-up = Mais alto
action-volume-down = Mais baixo
action-cycle-speed-preset = Alternar os presets de velocidade (rápida, normal, estudo, lenta)
action-choose-voice = Escolher uma voz
action-restart-speech = Reiniciar a fala com as configurações atuais (depois que o motor de fala parou de funcionar)
action-cycle-verbosity = Alternar quanto o textweaver diz: baixo, normal, alto
action-cycle-punctuation = Alternar quanta pontuação é falada: nenhuma, alguma, toda
action-find = Localizar texto no documento
action-find-next = Localizar a próxima ocorrência
action-find-previous = Localizar a ocorrência anterior
action-next-misspelling = Mover para a próxima palavra com erro ortográfico, e soletrá-la
action-previous-misspelling = Mover para a palavra com erro ortográfico anterior, e soletrá-la
action-spelling-suggestions = Listar sugestões para a palavra com erro ortográfico no cursor, ou adicioná-la à sua lista de palavras
action-next-grammar-problem = Mover para o próximo problema de gramática, e dizê-lo com sua correção
action-previous-grammar-problem = Mover para o problema de gramática anterior, e dizê-lo com sua correção
action-next-lint-problem = No modo de edição, mover para o próximo problema de Lint do Markdown, e dizê-lo
action-previous-lint-problem = No modo de edição, mover para o problema de Lint do Markdown anterior, e dizê-lo
action-add-bookmark = Adicionar um marcador no cursor
action-list-bookmarks = Listar marcadores
action-next-bookmark = Mover para o próximo marcador
action-previous-bookmark = Mover para o marcador anterior
action-add-note = Adicionar uma nota à seleção ou à frase no cursor
action-list-notes = Listar notas
action-next-note = Mover para a próxima nota
action-previous-note = Mover para a nota anterior
action-delete-note = Excluir a nota ou o realce no cursor
action-highlight-selection = Realçar a seleção, ou a frase no cursor
action-export-study-sheet = Exportar as notas e realces como uma folha de estudo em Markdown, agrupada por cabeçalho
action-open = Abrir um documento
action-open-path = Abrir um documento digitando o caminho
action-open-library = Abrir a biblioteca: documentos nas suas pastas de biblioteca e arquivos recentes
action-new-document = Iniciar um novo documento no modo de edição
action-save = Salvar (Markdown e texto no lugar; outros formatos como Markdown)
action-save-as = Salvar com um novo nome
action-export-settings = Exportar configurações e teclas personalizadas para um arquivo JSON ou TOML
action-import-settings = Importar configurações de um arquivo JSON ou TOML, após uma confirmação
action-reading-statistics = Listar estatísticas de leitura: tempo lido, ponto mais distante, sessões e os documentos mais lidos
action-new-from-template = Iniciar um novo documento a partir de um modelo, com título, autor, data e um cabeçalho de referências
action-export-html = Exportar o documento como uma página web (HTML) ao lado dele
action-export-pdf = Exportar o documento como um PDF com marcação (tagged) ao lado dele
action-export-docx = Exportar o documento como um arquivo do Word (DOCX) ao lado dele
action-export-epub = Exportar o documento como um livro EPUB ao lado dele
action-export-brf = Exportar o documento como braille (BRF) ao lado dele
action-preview-in-browser = Pré-visualizar o documento no navegador web, com matemática; cada salvamento reescreve a pré-visualização
action-toggle-preview-auto-reload = Ligar ou desligar a recarga automática da pré-visualização no navegador
action-toggle-preview-live = Ligar ou desligar a pré-visualização ao vivo: com a recarga automática, a pré-visualização também recarrega quando a digitação pausa
action-quit = Sair, salvando a posição de leitura
action-toggle-edit-mode = Alternar entre leitura e edição
action-undo = Desfazer
action-redo = Refazer
action-bold = Deixar a seleção em negrito
action-italic = Deixar a seleção em itálico
action-underline = Sublinhar a seleção
action-strikethrough = Tachar a seleção
action-inline-code = Marcar a seleção como código
action-code-block = Transformar as linhas selecionadas em um bloco de código
action-insert-link = Transformar a seleção em um link
action-heading = Transformar a linha atual em um cabeçalho
action-bullet-list = Transformar as linhas selecionadas em uma lista com marcadores
action-numbered-list = Transformar as linhas selecionadas em uma lista numerada
action-block-quote = Transformar as linhas selecionadas em uma citação
action-horizontal-rule = Inserir uma linha horizontal
action-insert-table = Inserir uma tabela
action-add-table-row = Adicionar uma linha à tabela no cursor
action-insert-image = Inserir uma imagem
action-replace = Localizar e substituir
action-copy = Copiar a seleção, ou a frase no cursor, para a área de transferência
action-cut = Recortar a seleção para a área de transferência
action-next-table-cell = Em uma tabela, mover para a próxima célula e dizer sua coluna; fora dela, digitar uma tabulação
action-previous-table-cell = Em uma tabela, mover para a célula anterior e dizer sua coluna
action-cycle-typing-echo = Alternar o eco de digitação: caracteres e palavras, caracteres, palavras, ou nenhum
action-select-all = Selecionar todo o texto
action-delete-word-before = Excluir a palavra antes do cursor
action-delete-word-after = Excluir a palavra depois do cursor
action-paste = Colar o último texto copiado ou recortado no textweaver; a colagem do terminal também funciona
action-insert-citation = Inserir uma citação: escolha uma referência, depois informe uma página ou outro localizador
action-add-reference = Adicionar uma referência à sua biblioteca por DOI ou ISBN
action-insert-bibliography = Inserir a bibliografia das obras citadas, no cursor
action-check-citations = Verificar as citações: quantas há, e quais chaves não estão na sua biblioteca
action-import-references = Importar referências de um arquivo BibTeX, RIS ou CSL-JSON para a sua biblioteca
action-next-theme = Mudar para o próximo tema de cores
action-toggle-line-numbers = Mostrar ou ocultar números de linha
action-toggle-character-keys = Ligar ou desligar os atalhos de tecla única, para que o ditado e a digitação nunca acionem comandos
action-cycle-access-mode = Alternar o modo de acessibilidade: autofala, híbrido ou leitor de tela
action-settings-profiles = Listar perfis de configurações: mudar para um, salvar as configurações atuais como um, renomear, excluir, importar ou exportar
action-bionic-toggle = Ligar ou desligar a leitura biônica: o início de cada palavra em negrito
action-ruler-cycle = Alternar a régua de leitura: desligada, linha atual, régua
action-syllables-toggle = Mostrar ou ocultar sílabas: palavras divididas com um ponto médio
action-difficult-words-toggle = Ligar ou desligar a marcação de palavras difíceis: sublinhadas, e nomeadas ao mover entre palavras com verbosidade alta
action-text-larger = Aumentar o texto do documento
action-text-smaller = Diminuir o texto do documento
action-text-size-reset = Voltar o texto do documento ao tamanho padrão
action-choose-font = Escolher a fonte do texto do documento
action-command-palette = Executar qualquer comando pelo nome
action-settings = Abrir as configurações: cada opção com sua ajuda, filtrada conforme você digita; Esquerda e Direita mudam um valor
action-keyboard-help = Listar atalhos de teclado
action-help = Abrir a ajuda

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

## Restarting speech.

restart-silent-now = O { -brand } está silencioso agora; reinicie-o para ouvir a fala de novo.
# $keys names the Restart Speech key or keys.
restart-silent-use-key = O { -brand } está silencioso agora. Reinicie a fala com { $keys }.
restart-restarting = Reiniciando a fala.
restart-not-here = A fala não pode ser reiniciada aqui.
restart-already = A fala já está reiniciando.
# $error is the system's reason, in its own words.
restart-failed = Não foi possível reiniciar a fala: { $error }.
restart-start-failed = Não foi possível reiniciar a fala: a inicialização falhou.
restart-no-engine = Nenhum motor de fala está disponível; o { -brand } permanece silencioso.
restart-done-silent = Fala reiniciada, mas nenhum motor de fala está disponível; o { -brand } permanece silencioso.
restart-done = Fala reiniciada.

## Tab completion of file paths in prompts.

# $folder is the folder's full path.
pathc-no-folder = Não há pasta { $folder }.
# $prefix is what was typed after the last separator.
pathc-no-match = Nenhum arquivo ou pasta começa com { $prefix }.
# The one name that matched; $kind is folder or file.
pathc-one =
    { $kind ->
        [folder] { $name }, pasta
       *[file] { $name }, arquivo
    }
# $n names matched; $names are the first few, joined with commas; $more is
# yes when more matched than are read out.
pathc-many =
    { $more ->
        [yes] { $n } correspondências: { $names }, e mais.
       *[no] { $n } correspondências: { $names }.
    }

## Exporting and importing settings.

# $n settings differ; $name is the file's name.
settingsio-import-question =
    { $n ->
        [one] Importar { $n } configuração alterada de { $name }? y ou n
       *[other] Importar { $n } configurações alteradas de { $name }? y ou n
    }
settingsio-no-persistence = As configurações não são salvas nesta sessão, então não podem ser exportadas ou importadas.
# $path is the file written.
settingsio-exported = Configurações exportadas para { $path }.
settingsio-export-failed = Não foi possível exportar as configurações: { $error }
# $path is the file; $error the system's reason.
settingsio-read-failed = Não foi possível ler { $path }: { $error }.
settingsio-nothing-to-import = Nada para importar: suas configurações já correspondem a esse arquivo.
settingsio-cancelled-unchanged = Cancelado. Nada foi alterado.
settingsio-import-failed = Não foi possível importar as configurações: { $error }
# $summary lists what changed (from the settings store, in English).
settingsio-imported = Configurações importadas. { $summary }
settingsio-backend-next-start = O novo motor de fala é usado a partir do próximo início.
settingsio-backed-up = As configurações antigas foram salvas como backup.

## Opening a document: failures and opening in the background.

# $name is the file's name.
opening-is-folder = { $name } é uma pasta, não um documento. Informe o nome de um arquivo dentro dela.
# Said after "Could not open NAME:", so it starts in lower case.
opening-no-file-in = não há nenhum arquivo chamado { $name } em { $folder }. Verifique o nome.
opening-no-file-here = não há nenhum arquivo chamado { $name } aqui. Verifique o nome.
opening-no-permission = você não tem permissão para lê-lo.
opening-damaged-rtf = não é um arquivo RTF legível; pode estar danificado.
opening-damaged-odt = não é um arquivo de texto OpenDocument legível; pode estar danificado.
opening-damaged-latex = não é um arquivo LaTeX legível; pode estar danificado ou ser grande demais.
opening-damaged-email = não é uma mensagem de e-mail legível; pode estar danificada ou ser grande demais.
opening-damaged-mhtml = não é uma página da web arquivada legível; pode estar danificada ou ser grande demais.
# $reason is one of the opening-no-* messages, or the loader's own words.
opening-failed = Não foi possível abrir { $name }: { $reason }
opening-started = Abrindo { $name }. Escape cancela.
opening-stopped = Parou de abrir { $name }.
opening-still = Ainda abrindo { $name }, { $secs } segundos.
# $step is the loader's report, such as "recognizing text on page 3 (3 of 40)."
opening-still-step = Ainda abrindo { $name }: { $step }
opening-stopped-unexpectedly = Não foi possível abrir { $name }: o carregamento parou inesperadamente.

## A build without the publish feature. "tw convert" is a command typed
## at the terminal: keep it as it is.

lean-citations-not-in-build = As citações não estão nesta versão do { -brand }. Ele foi compilado sem o recurso de publicação.
lean-publish-not-in-build = Exportar e pré-visualizar não estão nesta versão do { -brand }. Ele foi compilado sem o recurso de publicação; tw convert ainda converte.

## The voice manager's list.

# $n voices are shown; $language is a language name or voices-all-languages;
# $engine an engine's name or voices-all-engines.
voices-shown =
    { $n ->
        [one] { $n } voz: { $language }, { $engine }.
       *[other] { $n } vozes: { $language }, { $engine }.
    }
voices-all-languages = todos os idiomas
voices-all-engines = todos os motores
# The filter rows at the top of the list.
voices-language-row = Idioma: { $language }
voices-engine-row = Motor: { $engine }
voices-fetch-row = Baixar a lista de vozes do Piper da internet
# Parts of a voice's row, joined with commas. $size is in megabytes, such
# as "63 MB".
voices-download-size = download { $size }
voices-licence-public-domain = domínio público
voices-licence-attribution = livre com crédito
voices-licence-share-alike = livre com crédito, compartilhamento igual
voices-licence-non-commercial = não comercial
voices-licence-unknown = licença mostrada antes do download
voices-favourite = favorita
voices-current = atual

## Language names in the voice manager's language filter.

voices-language-ar = Árabe
voices-language-ca = Catalão
voices-language-cs = Tcheco
voices-language-cy = Galês
voices-language-da = Dinamarquês
voices-language-de = Alemão
voices-language-el = Grego
voices-language-en = Inglês
voices-language-es = Espanhol
voices-language-fa = Persa
voices-language-fi = Finlandês
voices-language-fr = Francês
voices-language-hi = Hindi
voices-language-hu = Húngaro
voices-language-is = Islandês
voices-language-it = Italiano
voices-language-ja = Japonês
voices-language-ka = Georgiano
voices-language-kk = Cazaque
voices-language-ko = Coreano
voices-language-lb = Luxemburguês
voices-language-lv = Letão
voices-language-nl = Holandês
voices-language-no = Norueguês
voices-language-pl = Polonês
voices-language-pt = Português
voices-language-ro = Romeno
voices-language-ru = Russo
voices-language-sk = Eslovaco
voices-language-sl = Esloveno
voices-language-sr = Sérvio
voices-language-sv = Sueco
voices-language-sw = Suaíli
voices-language-tr = Turco
voices-language-uk = Ucraniano
voices-language-vi = Vietnamita
voices-language-zh = Chinês

## Voices: the voice manager, rate, pitch, and volume.

# Spoken by a newly chosen voice as its sample.
voice-sample = Um pequeno jabuti xereta viu dez cegonhas felizes.
voice-list-title = Escolher uma voz
voice-still-loading = As vozes ainda estão carregando. A lista abre quando estiverem prontas.
voice-list-failed = Não foi possível listar as vozes: { $error }.
# $shown is voices-shown ("12 voices: English, all engines."). Enter,
# Space, Delete and Escape are the list's own keys.
voice-manager-intro = Gerenciador de vozes. { $shown } Enter usa uma voz e fala uma amostra, ou baixa uma; Espaço marca uma favorita; Delete remove uma voz baixada; Escape fecha.
voice-more-ready = { $n } mais vozes de outros motores estão prontas. Pressione Escape e abra o gerenciador de vozes de novo para vê-las.
# $keys names the Choose Voice key.
voice-ready = As vozes estão prontas. { $keys } as lista.
voice-fetch-catalog-question = Baixar a lista de vozes do Piper, cerca de 250 kilobytes, do Hugging Face? y ou n
voice-fetch-catalog-question-short = Baixar a lista de vozes do Piper? y ou n
# $engine is the engine's name, such as "Piper neural voices".
voice-switching-engine = Voz { $voice }, em { $engine }. Trocando de motor.
voice-download-in-progress = Já há um download de voz em andamento.
voice-no-data-folder = Não há pasta de dados para guardar as vozes do Piper.
voice-not-in-list = Essa voz não está mais na lista de vozes do Piper.
voice-download-start-failed = Não foi possível iniciar o download.
voice-reading-licence = Lendo a licença de { $voice }.
voice-remove-question = Remover a voz { $voice }? y ou n
voice-only-piper-removable = Somente vozes do Piper baixadas podem ser removidas.
# $plan describes the download: the voice, its size and licence.
voice-download-question = { $plan } y ou n
voice-in-use = { $voice } é a voz em uso. Escolha outra voz primeiro.
voice-removed = { $voice } removida.
voice-remove-failed = Não foi possível remover { $voice }: { $error }.
voice-downloading-catalog = Baixando a lista de vozes do Piper.
voice-downloading = Baixando { $voice }.
voice-downloading-percent = Baixando { $voice }, { $pct } por cento.
voice-details-failed = Não foi possível ler os detalhes da voz: { $error }.
voice-download-stopped = O download parou.
voice-catalog-fetched = A lista de vozes do Piper tem { $voices } vozes em { $languages } idiomas. Escolher Voz as lista.
voice-catalog-failed = Não foi possível baixar a lista de vozes: { $error }.
# $licence describes the voice's licence, in a sentence of its own.
voice-installed = { $voice } está instalada. { $licence } Escolher Voz a lista.
voice-download-failed = Não foi possível baixar { $voice }: { $error }.
voice-only-voice-favourite = Somente uma voz pode ser favorita.
voice-favourite-added = { $voice } adicionada às favoritas.
voice-favourite-removed = { $voice } removida das favoritas.
voice-chosen = Voz { $voice }.
voice-chosen-rate = Voz { $voice }, { $wpm } palavras por minuto.
voice-fastest-rate = Velocidade máxima.
voice-slowest-rate = Velocidade mínima.
voice-rate = { $wpm } palavras por minuto.
voice-highest-pitch = Tom mais alto.
voice-lowest-pitch = Tom mais baixo.
voice-pitch-normal = Tom normal.
# $n is a number of semitones.
voice-pitch-plus = Tom mais { $n }.
voice-pitch-minus = Tom menos { $n }.
voice-full-volume = Volume máximo.
voice-volume-off = Volume desligado.
voice-volume = Volume { $pct } por cento.
voice-no-speed-presets = Nenhum preset de velocidade.
# $name is the preset's name from the settings, such as "Study".
voice-speed-preset = { $name }, velocidade { $wpm }.
voice-line-numbers-on = Números de linha ligados.
voice-line-numbers-off = Números de linha desligados.

## Export and preview from the reader. F5 is the browser's reload key,
## not textweaver's.

publish-no-document = Nenhum documento está aberto.
# Said after "Could not export:", so it starts in lower case. $path is a
# folder or a file; $error the system's reason.
publish-cannot-write-to = não é possível escrever em { $path }: { $error }
publish-cannot-write = não é possível escrever { $path }: { $error }
publish-start-failed = Não foi possível iniciar a exportação: { $error }
publish-export-error = Não foi possível exportar: { $error }
# $format is the format's name, such as PDF, HTML, or Word.
publish-exporting = Exportando para { $format }.
publish-writing-preview = Escrevendo a pré-visualização.
publish-preview-error = Não foi possível escrever a pré-visualização: { $error }
publish-still-exporting =
    { $secs ->
        [one] Ainda exportando para { $format }, { $secs } segundo.
       *[other] Ainda exportando para { $format }, { $secs } segundos.
    }
publish-still-previewing =
    { $secs ->
        [one] Ainda escrevendo a pré-visualização, { $secs } segundo.
       *[other] Ainda escrevendo a pré-visualização, { $secs } segundos.
    }
# $again is yes when a preview is open already.
publish-auto-reload-on =
    { $again ->
        [yes] Recarga automática da pré-visualização ligada: depois de cada salvamento o navegador recarrega a página por conta própria. Execute pré-visualizar no navegador de novo para usá-la.
       *[no] Recarga automática da pré-visualização ligada: depois de cada salvamento o navegador recarrega a página por conta própria.
    }
publish-auto-reload-off = Recarga automática da pré-visualização desligada: pressione F5 no navegador depois de salvar.
publish-live-on = Pré-visualização ao vivo ligada: a pré-visualização também recarrega quando a digitação pausa.
# "toggle preview auto reload" is the command's name in the command palette.
publish-live-on-needs-reload = Pré-visualização ao vivo ligada. Ela funciona com a recarga automática, que está desligada; ligue-a com toggle preview auto reload.
publish-live-off = Pré-visualização ao vivo desligada: a pré-visualização recarrega só depois de salvar.
# $error is the converter's reason.
publish-export-failed = A exportação para { $format } falhou: { $error }
publish-preview-failed = A pré-visualização falhou: { $error }
# The converter's warnings: how many, and the first one.
publish-warnings =
    { $n ->
        [one] 1 aviso: { $first }
       *[other] { $n } avisos; o primeiro: { $first }
    }
# $file is the file's name, $folder its folder; $warned is empty or a
# space and publish-warnings.
publish-exported = Exportado para { $format }: { $file }. Abrir? y ou n. Em { $folder }.{ $warned }
publish-preview-written-served = Pré-visualização escrita. Abrindo-a no navegador. Ela recarrega por conta própria depois de cada salvamento.{ $warned }
publish-preview-written = Pré-visualização escrita. Abrindo-a no navegador. Salvar a escreve de novo; depois pressione F5 no navegador.{ $warned }
publish-preview-updated = Pré-visualização atualizada.
publish-preview-updated-press-f5 = Pré-visualização atualizada. Pressione F5 no navegador.
publish-server-failed = Não foi possível iniciar o servidor de recarga da pré-visualização ({ $error }); abrindo o arquivo em vez disso.
publish-render-failed = Não foi possível renderizar o texto: { $error }
publish-nothing-after-caret = Nada para ler depois do cursor.
publish-listening = Ouvindo o texto renderizado.

## The preview's reload server: shown in the browser.

preview-being-written = A pré-visualização está sendo escrita. Recarregue em um instante.

## Notes and highlights.

notes-nothing-to-attach = Nada aqui para anexar uma nota.
# $on is the start of the passage the note is on.
notes-added = Nota adicionada em: { $on }
# $tags are the note's tags, joined with commas.
notes-added-with-tags = Nota adicionada com as tags { $tags } em: { $on }
# An item in the notes list. $anchor is the passage; $lost is yes when the
# passage was not found after the file changed.
notes-item =
    { $lost ->
        [yes] { $note }, linha { $line }. Em: { $anchor } Não encontrada depois que o arquivo mudou.
       *[no] { $note }, linha { $line }. Em: { $anchor }
    }
notes-none = Nenhuma nota.
notes-list-title = Notas
notes-list-intro =
    { $n ->
        [one] Notas, 1 item. Enter vai até uma nota, Delete a exclui, F2 a edita.
       *[other] Notas, { $n } itens. Enter vai até uma nota, Delete a exclui, F2 a edita.
    }
# Said on jumping to a note: its text, then the passage it is on.
notes-note-content = { $note }. Em: { $anchor }
# $i is the note's number, $n how many notes there are.
notes-note-label = Nota { $i } de { $n }
notes-deleted = Nota excluída: { $text }.
notes-none-here = Nenhuma nota ou realce aqui.
notes-unchanged = Nota inalterada.
notes-updated = Nota atualizada.
notes-nothing-to-highlight = Nada aqui para realçar.
notes-highlight-removed = Realce removido: { $text }
notes-highlighted-at = Realçado em { $pct } por cento: { $text }
notes-highlighted = Realçado: { $text }
# An item in the highlights list. $color is the highlight's color name;
# $lost is yes when the text was not found after the file changed.
notes-highlight-item =
    { $lost ->
        [yes] { $text }, linha { $line }, { $color }, não encontrado depois que o arquivo mudou
       *[no] { $text }, linha { $line }, { $color }
    }
notes-no-highlights = Nenhum realce.
notes-highlights-title = Realces
notes-highlights-intro =
    { $n ->
        [one] Realces, 1 item. Enter vai até um, Delete o remove.
       *[other] Realces, { $n } itens. Enter vai até um, Delete o remove.
    }
# The label said before a highlight's text on jumping to it.
notes-highlight-label = Realçar
# Shown while reading reaches a note's passage.
notes-signal = Nota: { $text }
# Said after moving onto a note's passage.
notes-has-note = Tem uma nota: { $text }

## Bookmarks: rename and delete.

notes-choose-bookmark-delete = Escolha um marcador e pressione Delete.
notes-choose-bookmark-rename = Escolha um marcador e pressione F2 para renomeá-lo.
notes-bookmark-deleted = Marcador { $name } excluído.
notes-renaming-bookmark = Renomeando o marcador { $name }.
notes-bookmark-unchanged = Marcador inalterado.
# $name is the name asked for, $old the bookmark's name.
notes-bookmark-name-taken = Já existe um marcador chamado { $name }. Marcador { $old } inalterado.
notes-bookmark-renamed = Marcador { $old } renomeado para { $name }.

## The study sheet.

notes-nothing-to-export = Nenhuma nota ou realce para exportar.
# Keep the letters y and n: they are the keys that answer. $file is the
# sheet's file name, $folder the folder it was saved in.
notes-study-sheet-saved-notes =
    Folha de estudo com { $n ->
        [one] 1 nota
       *[other] { $n } notas
    } salva como { $file }. Abrir? y ou n. Em { $folder }.
notes-study-sheet-saved-highlights =
    Folha de estudo com { $h ->
        [one] 1 realce
       *[other] { $h } realces
    } salva como { $file }. Abrir? y ou n. Em { $folder }.
notes-study-sheet-saved-both =
    Folha de estudo com { $n ->
        [one] 1 nota
       *[other] { $n } notas
    } e { $h ->
        [one] 1 realce
       *[other] { $h } realces
    } salva como { $file }. Abrir? y ou n. Em { $folder }.
notes-study-sheet-failed = Não foi possível escrever a folha de estudo: { $error }
# The study sheet file's own text (Markdown; the # marks stay in the code).
notes-sheet-title = Folha de estudo: { $title }
notes-sheet-exported = Exportado do { -brand } em { $date }.
notes-sheet-before-first-heading = Antes do primeiro cabeçalho
# After a note's text: its tags, joined with commas.
notes-sheet-tags = (tags: { $tags })
# $color is the highlight's color name.
notes-sheet-highlighted = Realçado, { $color }.

## Find, bookmarks, and selection.

marks-cannot-search = Não foi possível buscar: { $error }.
# $pattern is the text searched for.
marks-no-matches = Nenhuma ocorrência de { $pattern }.
# The label of a match reached by Find, at high verbosity; $number is its place among $n matches.
marks-match-label = Ocorrência { $number } de { $n }
# Find wrapped past an end of the document; $dir is next (to the top) or previous (to the bottom); $message says the match.
marks-find-wrapped =
    { $dir ->
        [next] Voltou ao início. { $message }
       *[previous] Voltou ao fim. { $message }
    }
# $name is the bookmark's name, such as mark1.
marks-bookmark-already-here = O marcador { $name } já está aqui.
marks-bookmark-set = Marcador { $name } definido em { $pct } por cento.
marks-no-bookmarks = Nenhum marcador.
marks-bookmarks-intro =
    { $n ->
        [one] Marcadores, { $n } item. Enter vai até um, Delete o exclui, F2 o renomeia.
       *[other] Marcadores, { $n } itens. Enter vai até um, Delete o exclui, F2 o renomeia.
    }
# One line of the bookmark list; $lost is yes when the bookmark's text was not found after the file changed; $text is the start of its line.
marks-bookmark-item =
    { $lost ->
        [yes] { $name } (não encontrado depois que o arquivo mudou), linha { $line }, { $pct } por cento: { $text }
       *[no] { $name }, linha { $line }, { $pct } por cento: { $text }
    }
marks-bookmarks-title = Marcadores
# The label of a bookmark reached, at high verbosity.
marks-bookmark-label = Marcador { $name }
marks-selection-cleared = Seleção limpa.
# $text is the selected text, shortened.
marks-selected = Selecionado { $text }

## Authoring lists: the outline, the citation picker, spelling, grammar, and templates.

# An outline item: $text is the heading's text, $level its level.
lists-outline-item = { $text }, nível { $level }
# $n is the number of headings.
lists-outline-title =
    { $n ->
        [one] Estrutura, { $n } cabeçalho
       *[other] Estrutura, { $n } cabeçalhos
    }
# $shown headings of $n match the filter $filter typed so far.
lists-outline-title-filtered = Estrutura, { $shown } de { $n } correspondem a { $filter }
# $n is the number of references.
lists-citations-title =
    { $n ->
        [one] Inserir citação, { $n } referência
       *[other] Inserir citação, { $n } referências
    }
# $shown references of $n match the filter $filter typed so far.
lists-citations-title-filtered = Inserir citação, { $shown } de { $n } correspondem a { $filter }
# $word is the misspelled word.
lists-spelling-title = Ortografia de { $word }
lists-spelling-add = Adicionar { $word } à sua lista de palavras
lists-leave-as-is = Deixar como está
# $words are the words the grammar fixes are for.
lists-grammar-title = Correções de gramática para { $words }
# $n is the number of templates.
lists-templates-title = Novo documento a partir de um modelo, { $n } modelos
lists-no-filter = Esta lista não filtra.
# The filter was emptied: $n items are shown.
lists-filter-cleared-headings =
    { $n ->
        [one] Filtro limpo, { $n } cabeçalho.
       *[other] Filtro limpo, { $n } cabeçalhos.
    }
lists-filter-cleared-references =
    { $n ->
        [one] Filtro limpo, { $n } referência.
       *[other] Filtro limpo, { $n } referências.
    }
lists-filter-cleared-items =
    { $n ->
        [one] Filtro limpo, { $n } item.
       *[other] Filtro limpo, { $n } itens.
    }
# Nothing matches the filter $query.
lists-filter-none-headings = Nenhum cabeçalho corresponde a { $query }. Backspace remove letras.
lists-filter-none-references = Nenhuma referência corresponde a { $query }. Backspace remove letras.
lists-filter-none-items = Nenhum item corresponde a { $query }. Backspace remove letras.
# $n items match the filter.
lists-filter-matched-headings =
    { $n ->
        [one] { $n } cabeçalho corresponde.
       *[other] { $n } cabeçalhos correspondem.
    }
lists-filter-matched-references =
    { $n ->
        [one] { $n } referência corresponde.
       *[other] { $n } referências correspondem.
    }
lists-filter-matched-items =
    { $n ->
        [one] { $n } item corresponde.
       *[other] { $n } itens correspondem.
    }
lists-no-headings = Este documento não tem cabeçalhos.
# $n is the number of headings; the keys are the outline list's own.
lists-outline-intro =
    { $n ->
        [one] Estrutura, { $n } cabeçalho. Digite para filtrar, Enter vai até um cabeçalho, Escape fecha.
       *[other] Estrutura, { $n } cabeçalhos. Digite para filtrar, Enter vai até um cabeçalho, Escape fecha.
    }
# $heading is the text of the heading the cursor is under.
lists-outline-here = Você está sob { $heading }.

## Lists and prompts shared by every frontend.

# The focused list item: $item is its text, $k its place, $n the number of items.
listmodel-item-position = { $k } de { $n }, { $item }
# $letter is the letter or digit typed.
listmodel-no-item-starts = Nenhum item começa com { $letter }.
listmodel-top-of-list = Início da lista.
listmodel-end-of-list = Fim da lista.
listmodel-no-matching-commands = Nenhum comando correspondente.
listmodel-no-earlier-entries = Nenhuma entrada anterior.
# Tab in the command palette: $n commands match (always more than one); $names lists the first few, joined by commas.
listmodel-command-matches = { $n } correspondências: { $names }.

## The library.

# $n is how many documents the scan has found.
library-still-scanning = Ainda examinando a biblioteca: { $n } encontrados até agora.
library-scan-failed = Não foi possível examinar a biblioteca: { $error }.
library-scanning = Examinando a biblioteca.
library-scan-progress = Examinando a biblioteca: { $n } encontrados até agora.
library-scan-stopped = O exame da biblioteca parou com um erro interno.
# $command is the command line that adds a folder; $key names the Open command's key.
library-empty = A biblioteca está vazia. Adicione uma pasta com { $command }, ou abra um arquivo com { $key }.
library-intro =
    { $n ->
        [one] Biblioteca, { $n } documento. Digite para filtrar, Enter abre um.
       *[other] Biblioteca, { $n } documentos. Digite para filtrar, Enter abre um.
    }
library-title = Biblioteca

## Following links and footnotes.

links-none-here = Nenhum link ou nota de rodapé no cursor.
# $text is the link's text.
links-no-address = O link { $text } não tem endereço.
# $kind is mail or web; $target is the link's address.
links-open-question =
    { $kind ->
        [mail] Abrir o link de e-mail? y ou n. { $target }
       *[web] Abrir o link da web? y ou n. { $target }
    }
# The label of a heading reached by a link, at high verbosity.
links-heading-label = Cabeçalho
# $anchor is the heading name the link gives.
links-no-heading = Nenhum cabeçalho chamado { $anchor } neste documento.
# $file is the file the link names.
links-file-not-found = O link vai para { $file }, que não foi encontrado.
# $file is the file's name; $key names the History Back command's keys.
links-followed = Seguiu o link até { $file }. Voltar: { $key }.
links-back-in = De volta em { $file }.
# $label is the footnote's label, such as 1.
links-back-to-footnote-reference = De volta à referência da nota de rodapé { $label }, linha { $line }.
# $text is the start of the note.
links-footnote = Nota de rodapé { $label }: { $text }
links-footnote-unreferenced = Nenhuma referência à nota de rodapé { $label } no texto.
links-footnote-no-note = A nota de rodapé { $label } não tem nota.

## Citations: inserting, looking up, importing, checking, and the bibliography.

citations-on = Citações ligadas.
citations-off = Citações desligadas.
# $key names the Add Reference command's keys.
citations-library-empty = Sua biblioteca de referências está vazia. Adicione uma referência por DOI ou ISBN com { $key }, ou execute importar referências na paleta de comandos.
# $n is how many references the picker lists.
citations-picker-intro =
    { $n ->
        [one] Inserir citação, { $n } referência. Digite para filtrar, Enter escolhe, Escape cancela.
       *[other] Inserir citação, { $n } referências. Digite para filtrar, Enter escolhe, Escape cancela.
    }
# $text is what was typed at the locator prompt.
citations-locator-unreadable = Não foi possível ler o localizador { $text }. Digite uma página como 12, páginas como 3-5, ou capítulo 2; Enter sozinho para nenhum.
citations-insert-failed = Não foi possível inserir a citação: { $error }
# $what is the identifier being looked up, as the citation library describes it.
citations-looking-up = Procurando { $what }.
citations-lookup-not-started = Não foi possível iniciar a busca: { $error }
# $input is the DOI or ISBN as typed.
citations-lookup-failed = Não foi possível encontrar { $input }: { $error }
citations-no-library-to-add-to = Não há biblioteca para adicionar: o { -brand } não guarda arquivos nesta sessão.
citations-library-save-failed = Não foi possível salvar a biblioteca: { $error }
# $n is how many citations the document has.
citations-found-no-library =
    { $n ->
        [one] { $n } citação encontrada. O { -brand } não guarda biblioteca nesta sessão.
       *[other] { $n } citações encontradas. O { -brand } não guarda biblioteca nesta sessão.
    }
citations-check-failed = Não foi possível verificar as citações: { $error }
citations-no-library-to-import-into = Não há biblioteca para importar: o { -brand } não guarda arquivos nesta sessão.
# $file is the file's path.
citations-import-failed = Não foi possível importar { $file }: { $error }
# $style is the style's name from the front matter, such as apa.
citations-style-unusable = Não é possível usar o estilo de citação { $style }: { $error }
citations-format-failed = Não foi possível formatar as citações: { $error }
# $key names the Insert Citation command's keys.
citations-none-yet = O documento ainda não tem citações. Insira uma com { $key }.
citations-nothing-to-list = Nenhuma das obras citadas está na sua biblioteca, então não há nada para listar.
# $n is how many entries went in; $style is the style's name, such as apa.
citations-bibliography-inserted =
    { $n ->
        [one] Inseriu a bibliografia, { $n } entrada, estilo { $style }.
       *[other] Inseriu a bibliografia, { $n } entradas, estilo { $style }.
    }
# Follows citations-bibliography-inserted; $keys are citation keys joined with commas.
citations-not-in-library = Não estão na biblioteca: { $keys }.
citations-bibliography-insert-failed = Não foi possível inserir a bibliografia: { $error }

## Speech Cursor mode.

speechcursor-off = Cursor de leitura desligado.
# $line is the line number.
speechcursor-on = Cursor de leitura ligado, linha { $line }. Seta para cima e para baixo leem linhas, Enter continua lendo, Tab ou Escape saem.
# $text is the line read, as the status line shows it.
speechcursor-on-with-text = Cursor de leitura ligado, linha { $line }: { $text }. Seta para cima e para baixo leem linhas, Enter continua lendo, Tab ou Escape saem.

## Scrolling the view without moving the cursor.

view-bottom-of-document = Fim do documento.
# $line is the line now at the top of the view.
view-line-at-top = Linha { $line } no topo.

## Background work, and opening files and addresses.

# $what names the export, as its own message says it.
tasks-stopped = { $what } parou inesperadamente.
# $input is the DOI, ISBN, or other identifier being looked up.
tasks-lookup-stopped = A busca por { $input } parou inesperadamente.
# The reason in tasks-could-not-open when a session keeps no files.
tasks-launch-off = abrir outros programas está desligado em uma sessão que não guarda arquivos
tasks-opening = Abrindo.
# $target is a file or a web address; $error says why.
tasks-could-not-open = Não foi possível abrir { $target }: { $error }
tasks-not-opened = Não aberto.
# Keep the letters y and n: they are the keys that answer.
tasks-open-it-question = Abrir? y ou n.

## Math exploration.

mathx-no-math = Nenhuma matemática aqui. Mova até uma fórmula e tente de novo.
# $math is the whole expression as spoken; $parts is yes when it has parts to go into. The keys are exploration's own.
mathx-exploring =
    { $parts ->
        [yes] Explorando a matemática: { $math }. Para baixo entra, as setas movem, Escape sai.
       *[no] Explorando a matemática: { $math }. Escape sai.
    }
mathx-left = Saiu da matemática.
mathx-last-term = Último termo.
mathx-first-term = Primeiro termo.
mathx-no-parts = Nenhuma parte dentro.
mathx-whole-expression = Expressão inteira.
mathx-nothing-here = Nada aqui.

## Reading aids: RSVP, bionic reading, syllables, difficult words, the ruler, and the reading level.

aids-rsvp-off = RSVP desligado.
aids-rsvp-leave-edit = Saia do modo de edição para usar o RSVP.
# $status is RSVP's status line (word and sentence counts, rate, state).
aids-rsvp-on = RSVP ligado. { $status }
aids-rsvp-no-words = Nenhuma palavra para mostrar.
aids-rsvp-fastest = Velocidade máxima do RSVP.
aids-rsvp-slowest = Velocidade mínima do RSVP.
# $wpm is the new rate in words per minute.
aids-rsvp-rate = RSVP { $wpm } palavras por minuto.
# Where the RSVP word is shown; $position is one of nine fixed keys.
aids-rsvp-position =
    { $position ->
        [top-left] RSVP no canto superior esquerdo.
        [top-center] RSVP no topo central.
        [top-right] RSVP no canto superior direito.
        [center-left] RSVP no meio à esquerda.
        [center] RSVP no centro.
        [center-right] RSVP no meio à direita.
        [bottom-left] RSVP no canto inferior esquerdo.
        [bottom-right] RSVP no canto inferior direito.
       *[bottom-center] RSVP na base central.
    }
aids-rsvp-playing = RSVP em reprodução.
aids-rsvp-paused = RSVP em pausa.
aids-rsvp-end-of-text = Fim do texto.
aids-rsvp-start-of-text = Início do texto.
aids-bionic-on = Leitura biônica ligada.
aids-bionic-off = Leitura biônica desligada.
aids-syllables-shown = Sílabas mostradas.
aids-syllables-hidden = Sílabas ocultas.
aids-difficult-on = Palavras difíceis sublinhadas.
aids-difficult-no-list = Palavras difíceis ligado, mas a lista de palavras está ausente nesta versão.
aids-difficult-off = Palavras difíceis não marcadas.
# Added after a word at high verbosity, following a comma.
aids-difficult-word = palavra difícil
aids-ruler-off = Régua de leitura desligada.
aids-ruler-current-line = Linha atual marcada.
aids-ruler-on = Régua de leitura ligada.
# $summary is aids-level-summary; $scope says what was measured.
aids-reading-level =
    { $scope ->
        [selection] Seleção: { $summary }
       *[document] Documento: { $summary }
    }
aids-reading-level-too-short = Não há texto suficiente para medir o nível de leitura.
# $grade is the Flesch-Kincaid grade with one decimal, $band an aids-band-* message, $ease the reading ease (0 to 100), $words aids-level-words, $sentences aids-level-sentences.
aids-level-summary = Série { $grade }, { $band }. Facilidade de leitura { $ease } de 100. { $words } em { $sentences }.
# $n is the count, $count the same number written with thousands separators.
aids-level-words =
    { $n ->
        [one] { $count } palavra
       *[other] { $count } palavras
    }
aids-level-sentences =
    { $n ->
        [one] { $count } frase
       *[other] { $count } frases
    }
aids-band-elementary = fundamental
aids-band-middle-school = ensino médio inicial
aids-band-high-school = ensino médio
aids-band-college = graduação
aids-band-graduate = pós-graduação

## Themes.

# $name is the theme name in the settings; $used the display name of the theme used instead.
themes-unknown = Não há tema chamado { $name }; usando { $used }.
# $theme is the new theme's display name.
themes-next = Tema { $theme }.

## Accessibility modes and the first-run question.

# Said when the accessibility mode changes; $mode is the new mode's id.
access-mode-changed =
    { $mode ->
        [self-voicing] Modo autofala. O textweaver fala tudo.
        [screen-reader] Modo leitor de tela. O textweaver fica em silêncio; seu leitor de tela lê a linha de status.
       *[hybrid] Modo híbrido. O textweaver lê documentos em voz alta; seu leitor de tela fala mensagens e digitação.
    }
# Yes to the first-run question; $key names the keys that change the mode.
access-hybrid-chosen = Modo híbrido. O textweaver lê documentos em voz alta; seu leitor de tela fala mensagens e digitação. { $key } muda o modo.
# No to the first-run question; $key names the keys that change the mode.
access-hybrid-declined = Permanecendo no modo autofala. { $key } muda o modo.
# $reader is the screen reader found (NVDA, JAWS), or access-a-screen-reader.
access-hybrid-question = { $reader } está em execução. Usar o modo híbrido, em que o textweaver lê documentos em voz alta e seu leitor de tela fala mensagens e digitação? y ou n
access-a-screen-reader = Um leitor de tela

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

## The settings screen. $label is a setting-* label, $value its value as
## described below.

settings-not-set = não definido
settings-none = nenhum
settings-empty = vazio
# A number and its unit (a settings-unit-* message): "300 words per minute".
settings-number-unit = { $n } { $unit }
settings-entries =
    { $n ->
        [0] nenhuma
        [one] 1 entrada
       *[other] { $n } entradas
    }
settings-type-on-or-off = Digite ligado ou desligado.
settings-type-a-number = Digite um número de { $min } a { $max }.
settings-outside = { $n } está fora de { $min } a { $max }.
# $names are the choices, joined with commas.
settings-choose-one-of = Escolha uma destas: { $names }.
settings-edit-table = Edite { $label } em settings.toml; ele guarda nomes e valores.
# $path is a key such as speech.rate, not translated.
settings-no-such-setting = Não há configuração { $path }.
settings-cannot-be = { $label } não pode ser isso: { $error }.
settings-changed = { $label }, { $value }.
settings-clamped = Fora do intervalo, então o valor mais próximo é usado.
settings-restart-speech = Reinicie a fala para usá-la.
settings-next-start = Usado a partir do próximo início.
settings-intro = Configurações, { $n } configurações. Digite para filtrar. Esquerda e Direita mudam um valor, Enter altera ou digita um, Delete restaura o padrão, Escape fecha.
settings-item = { $label }: { $value }
settings-title = Configurações
settings-title-matching = Configurações que correspondem a { $filter }
settings-closed = Configurações fechadas.
settings-filter-cleared =
    { $n ->
        [one] Filtro limpo, 1 configuração.
       *[other] Filtro limpo, { $n } configurações.
    }
settings-filter-none = Nenhuma configuração corresponde a { $query }. Backspace remove letras.
settings-filter-match =
    { $n ->
        [one] 1 configuração corresponde.
       *[other] { $n } configurações correspondem.
    }
settings-largest = Maior valor, { $value }.
settings-smallest = Menor valor, { $value }.
settings-press-enter = { $label }: pressione Enter para digitar um novo valor.
settings-table-item = { $label }: { $value }. Edite-o em settings.toml.
# $help is the setting's help (setting-*-help), which may be empty.
settings-editing = { $label }, agora { $value }. { $help }

## Settings: labels, help, and choices, as the settings screen shows and
## says them. Ids follow the key in settings.toml (speech.rate is
## setting-speech-rate).

setting-speech-backend = Motor de fala
setting-speech-backend-help = O motor de fala: automático escolhe o melhor disponível. Uma alteração reinicia a fala.
choice-speech-backend-auto = automático
choice-speech-backend-eci = Eloquence
choice-speech-backend-sapi = vozes SAPI 5
choice-speech-backend-espeak = eSpeak NG
choice-speech-backend-speechd = Speech Dispatcher
choice-speech-backend-nsspeech = Apple NSSpeech
choice-speech-backend-avspeech = Apple AVSpeech
choice-speech-backend-dectalk = DECtalk
choice-speech-backend-omnivox = Omnivox
choice-speech-backend-null = silencioso
setting-speech-rate = Velocidade
setting-speech-rate-help = Com que rapidez o textweaver fala.
setting-speech-volume = Volume
setting-speech-volume-help = Com que intensidade o textweaver fala.
setting-speech-pitch = Tom
setting-speech-pitch-help = Mais alto ou mais baixo que o tom próprio da voz.
setting-speech-voice = Voz
setting-speech-voice-help = O identificador da voz; não definido escolhe uma automaticamente. Escolher Voz as lista.
setting-speech-prefer-voice = Voz preferida
setting-speech-prefer-voice-help = Quando nenhuma voz está definida, a primeira voz cujo nome contém isto, como eloquence.
setting-speech-favorite-voices = Vozes favoritas
setting-speech-favorite-voices-help = Vozes listadas primeiro em Escolher Voz, por identificador.
setting-speech-punctuation = Pontuação
setting-speech-punctuation-help = Quanta pontuação é falada.
choice-speech-punctuation-none = nenhuma
choice-speech-punctuation-some = alguma
choice-speech-punctuation-all = toda
setting-speech-split-caps = Separar maiúsculas
setting-speech-split-caps-help = Dizer palavras unidas por maiúsculas, como TextWeaver, como palavras separadas.
setting-speech-caps = Maiúsculas
setting-speech-caps-help = Como uma letra maiúscula é marcada quando caracteres são falados e digitados.
choice-speech-caps-none = não marcada
choice-speech-caps-tone = um tom
choice-speech-caps-pitch = um tom mais agudo
choice-speech-caps-say-cap = dizer maiúscula
setting-speech-auto-play = Ler ao abrir
setting-speech-auto-play-help = Começar a ler quando um documento for aberto.
setting-speech-skip-code = Ignorar blocos de código
setting-speech-skip-code-help = Não falar blocos de código.
setting-speech-speed-presets = Presets de velocidade
setting-speech-speed-presets-help = Velocidades nomeadas que F8 percorre.
setting-speech-voices-by-language = Vozes por idioma
setting-speech-voices-by-language-help = A voz de cada idioma da interface, pela marca de idioma, como es = o identificador da voz. Um idioma não listado usa a primeira voz do motor para ele.
setting-speech-latency-offset-ms = Atraso do realce
setting-speech-latency-offset-ms-help = Quanto tempo depois de um motor informar uma palavra o realce se move, para motores cronometrados pelo relógio do áudio.
setting-speech-verbosity = Verbosidade
setting-speech-verbosity-help = Quanto o textweaver diz sobre o que faz.
choice-speech-verbosity-low = baixa
choice-speech-verbosity-normal = normal
choice-speech-verbosity-high = alta
setting-speech-eci-dictionaries = Dicionários do Eloquence
setting-speech-eci-dictionaries-help = Os dicionários de pronúncia da comunidade para o Eloquence: ligado, desligado, ou uma pasta sua.
choice-speech-eci-dictionaries-true = ligado
choice-speech-eci-dictionaries-false = desligado
setting-speech-eci-library = Biblioteca do Eloquence
setting-speech-eci-library-help = A biblioteca ECI a carregar; não definido procura nos locais de costume.
setting-speech-eci-code-factory = Procurar o Eloquence da Code Factory
setting-speech-eci-code-factory-help = Também procurar o Eloquence da Code Factory para Windows. A licença dele pode não cobrir outros programas.
setting-speech-sapi-onecore = Vozes OneCore
setting-speech-sapi-onecore-help = Também listar as vozes OneCore do Windows através do SAPI 5.
setting-speech-apple-backend = Motor de fala da Apple
setting-speech-apple-backend-help = Qual dos motores de fala da Apple usar no macOS.
choice-speech-apple-backend-auto = automático
choice-speech-apple-backend-nsspeech = NSSpeechSynthesizer
choice-speech-apple-backend-avspeech = AVSpeechSynthesizer
setting-highlight-enabled = Realçar o texto falado
setting-highlight-enabled-help = Realçar a palavra ou frase sendo lida.
setting-highlight-granularity = Realce
setting-highlight-granularity-help = O que o realce de leitura cobre.
choice-highlight-granularity-word = a palavra
choice-highlight-granularity-sentence = a frase
choice-highlight-granularity-both = a palavra e a frase
setting-highlight-lead-words = Antecipação do realce
setting-highlight-lead-words-help = Desenhar o realce esta quantidade de palavras à frente da palavra ouvida (1 é a palavra ouvida).
setting-highlight-speed = Velocidade do realce
setting-highlight-speed-help = Velocidade do realce cronometrado para motores que não informam palavras.
setting-highlight-color = Cor do realce de palavra
setting-highlight-color-help = Um nome de cor ou #rrggbb sobre o realce de palavra do tema; tema mantém o do tema.
setting-highlight-sentence-color = Cor do realce de frase
setting-highlight-sentence-color-help = Um nome de cor ou #rrggbb sobre o realce de frase do tema; não definido mantém o do tema.
setting-normalization-math = Falar matemática
setting-normalization-math-help = Falar notação matemática em palavras.
setting-normalization-math-verbosity = Verbosidade da matemática
setting-normalization-math-verbosity-help = Quão explícita é a matemática falada: baixa diz a sobre b, normal e alta dizem mais.
choice-normalization-math-verbosity-low = baixa
choice-normalization-math-verbosity-normal = normal
choice-normalization-math-verbosity-high = alta
setting-normalization-asciimath-delimiter = Delimitador de ASCIIMath
setting-normalization-asciimath-delimiter-help = O caractere ao redor do ASCIIMath, geralmente um acento grave; não definido não lê ASCIIMath.
setting-normalization-abbreviations = Expandir abreviações
setting-normalization-abbreviations-help = Dizer abreviações por extenso, como Doutor para Dr.
setting-normalization-abbrev-expansions = Suas abreviações
setting-normalization-abbrev-expansions-help = Abreviações suas e o que elas significam.
setting-normalization-numbers = Números por extenso
setting-normalization-numbers-help = Dizer números, datas, horas e valores em dinheiro por extenso.
setting-normalization-use-pronunciations = Usar pronúncias
setting-normalization-use-pronunciations-help = Aplicar sua lista de pronúncias.
setting-normalization-pronunciations = Pronúncias
setting-normalization-pronunciations-help = Palavras e como dizê-las.
setting-normalization-table-mode = Tabelas
setting-normalization-table-mode-help = Como as tabelas são lidas.
choice-normalization-table-mode-structured = com linhas e colunas
choice-normalization-table-mode-flat = como texto
choice-normalization-table-mode-skip = ignoradas
setting-normalization-footnote-mode = Notas de rodapé
setting-normalization-footnote-mode-help = Onde as notas de rodapé são lidas.
choice-normalization-footnote-mode-inline = onde são marcadas
choice-normalization-footnote-mode-deferred = no fim
choice-normalization-footnote-mode-skip = ignoradas
setting-normalization-community-lexicon-enabled = Léxico da comunidade
setting-normalization-community-lexicon-enabled-help = Aplicar os dicionários de pronúncia da comunidade para motores diferentes do Eloquence.
setting-normalization-community-lexicon-dir = Pasta do léxico da comunidade
setting-normalization-community-lexicon-dir-help = A pasta que guarda os arquivos de dicionário; não definido procura ao lado do textweaver.
setting-normalization-community-lexicon-language = Idioma do léxico da comunidade
setting-normalization-community-lexicon-language-help = O idioma dos dicionários.
choice-normalization-community-lexicon-language-enu = Inglês dos EUA
choice-normalization-community-lexicon-language-deu = Alemão
setting-reading-auto-resume = Retomar de onde parou
setting-reading-auto-resume-help = Voltar à posição salva quando um documento é aberto.
setting-reading-nav-history-size = Histórico de retorno
setting-reading-nav-history-size-help = Quantos lugares Voltar lembra.
setting-reading-wrap-navigation = Navegação circular
setting-reading-wrap-navigation-help = Mover além do fim do documento continua a partir do início.
setting-reading-cursor-follows-speech = Cursor segue a fala
setting-reading-cursor-follows-speech-help = O cursor se move com a palavra sendo lida.
setting-reading-sync-conflict-policy = Posições sincronizadas
setting-reading-sync-conflict-policy-help = Qual posição prevalece quando outro dispositivo leu mais adiante ou mais tarde.
choice-reading-sync-conflict-policy-newest = a mais recente
choice-reading-sync-conflict-policy-highest-progress = a mais distante
choice-reading-sync-conflict-policy-manual = perguntar
setting-reading-citations = Citações
setting-reading-citations-help = Citações na leitura contínua: ignoradas, ou ditas por extenso.
choice-reading-citations-off = ignoradas
choice-reading-citations-words = por extenso
setting-reading-ocr = Reconhecer páginas digitalizadas
setting-reading-ocr-help = Ler o texto de PDFs e imagens digitalizados reconhecendo-o (OCR).
setting-reading-ocr-lang = Idioma do texto digitalizado
setting-reading-ocr-lang-help = O idioma do texto digitalizado, como os códigos do Tesseract fra ou deu+eng; vazio significa o idioma do próprio documento, senão inglês.
choice-reading-ocr-lang- = o do documento
choice-reading-ocr-lang-eng = Inglês
choice-reading-ocr-lang-fra = Francês
choice-reading-ocr-lang-deu = Alemão
choice-reading-ocr-lang-spa = Espanhol
setting-reading-ocr-engine = Motor de OCR
setting-reading-ocr-engine-help = Qual motor reconhece páginas digitalizadas: ocrs para inglês e Tesseract para outros idiomas, ou sempre um deles.
choice-reading-ocr-engine-auto = automático
choice-reading-ocr-engine-ocrs = ocrs
choice-reading-ocr-engine-tesseract = Tesseract
choice-reading-ocr-engine-paddle = PaddleOCR (experimental)
setting-reading-math-engine = Fala da matemática
setting-reading-math-engine-help = Qual motor lê matemática em voz alta: o próprio do textweaver, ou o MathCAT em ClearSpeak ou SimpleSpeak, no idioma do documento. O MathCAT precisa de uma versão que o inclua; senão o próprio do textweaver é usado.
choice-reading-math-engine-builtin = textweaver
choice-reading-math-engine-mathcat = MathCAT ClearSpeak
choice-reading-math-engine-mathcat-simplespeak = MathCAT SimpleSpeak
setting-reading-math-display = Matemática na tela
setting-reading-math-display-help = Como a matemática aparece na visão de leitura: como sua origem, como x^2, ou como Unicode, como x com um 2 sobrescrito. A fala e o modo de edição sempre usam a origem.
choice-reading-math-display-source = origem
choice-reading-math-display-unicode = Unicode
setting-reading-revisions = Alterações controladas
setting-reading-revisions-help = Como as alterações controladas em arquivos do Word, OpenDocument e RTF são lidas: ditas no lugar com verbosidade alta (automático), sempre, ou nunca, lendo o texto final. Vale ao abrir um documento.
choice-reading-revisions-auto = automático
choice-reading-revisions-marked = sempre dizê-las
choice-reading-revisions-final = só o texto final
setting-display-theme = Tema
setting-display-theme-help = O tema de cores.
setting-display-follow-os-theme = Seguir o tema do sistema
setting-display-follow-os-theme-help = Na inicialização, usar um tema claro, escuro ou de alto contraste como o do sistema, a menos que você tenha escolhido um.
setting-display-wrap-width = Largura da quebra de linha
setting-display-wrap-width-help = Quebrar linhas nesta quantidade de colunas; 0 usa a largura inteira.
setting-display-tab-width = Largura da tabulação
setting-display-tab-width-help = Colunas que uma tabulação ocupa.
setting-display-show-line-numbers = Números de linha
setting-display-show-line-numbers-help = Mostrar números de linha.
setting-display-scroll-margin = Margem de rolagem
setting-display-scroll-margin-help = Linhas mantidas visíveis acima e abaixo do cursor.
setting-editing-autosave-recovery = Instantâneos de recuperação
setting-editing-autosave-recovery-help = Manter uma cópia do trabalho não salvo e oferecê-la depois de uma queda.
setting-editing-autosave-interval-secs = Intervalo dos instantâneos
setting-editing-autosave-interval-secs-help = Segundos entre instantâneos de recuperação enquanto há alterações não salvas.
setting-editing-echo-characters = Ecoar caracteres
setting-editing-echo-characters-help = Dizer cada caractere digitado.
setting-editing-echo-words = Ecoar palavras
setting-editing-echo-words-help = Dizer cada palavra digitada.
setting-editing-echo-deletions = Ecoar exclusões
setting-editing-echo-deletions-help = Dizer o que Backspace e Delete removem.
setting-editing-echo-lines-on-move = Ecoar linhas
setting-editing-echo-lines-on-move-help = Dizer a linha quando o cursor se move para outra linha.
setting-editing-undo-steps = Passos de desfazer
setting-editing-undo-steps-help = Quantos passos de desfazer são mantidos durante a edição, no máximo.
setting-editing-undo-memory-mb = Memória de desfazer
setting-editing-undo-memory-mb-help = Quanta memória o histórico de desfazer pode usar, no máximo.
setting-library-recent-limit = Arquivos recentes
setting-library-recent-limit-help = Quantos arquivos recentes são lembrados.
setting-library-folders = Pastas da biblioteca
setting-library-folders-help = Pastas cujos documentos a biblioteca lista, e cujas posições sincronizam entre computadores.
setting-keyboard-character-keys = Atalhos de tecla única
setting-keyboard-character-keys-help = Teclas de navegação como h e ponto. Desligado, o ditado e a digitação nunca acionam comandos.
setting-keyboard-preset = Teclas
setting-keyboard-preset-help = As teclas padrão: como o modo de navegação do NVDA e do JAWS, ou as teclas anteriores do textweaver. Usado a partir do próximo início.
choice-keyboard-preset-default = estilo leitor de tela
choice-keyboard-preset-classic = clássico
setting-keyboard-digit-row = Linha de dígitos
setting-keyboard-digit-row-help = Como o terminal reconhece as teclas de dígito para níveis de cabeçalho: automático, ou um teclado AZERTY francês.
choice-keyboard-digit-row-auto = automático
choice-keyboard-digit-row-azerty = AZERTY
setting-accessibility-mode = Modo de acessibilidade
setting-accessibility-mode-help = Autofala fala tudo; leitor de tela deixa a fala para seu leitor de tela; híbrido vocaliza só a leitura.
choice-accessibility-mode-self-voicing = autofala
choice-accessibility-mode-screen-reader = leitor de tela
choice-accessibility-mode-hybrid = híbrido
setting-accessibility-say-all = Dizer tudo com um leitor de tela
setting-accessibility-say-all-help = Leitura contínua no modo leitor de tela: uma frase de cada vez na linha de status, ou com a voz do textweaver.
choice-accessibility-say-all-screen = na linha de status
choice-accessibility-say-all-voice = com a voz do textweaver
setting-accessibility-quiet-screen = Tela quieta durante a leitura
setting-accessibility-quiet-screen-help = Manter a tela parada enquanto o textweaver lê em voz alta.
setting-accessibility-cursor = Cursor
setting-accessibility-cursor-help = Onde o cursor do terminal espera: no que você está trabalhando, ou na linha de status.
choice-accessibility-cursor-follow = segue o foco
choice-accessibility-cursor-status = na linha de status
setting-export-subtitle-format = Formato de legenda
setting-export-subtitle-format-help = O formato das legendas escritas sem um nome de arquivo.
choice-export-subtitle-format-srt = SubRip
choice-export-subtitle-format-vtt = WebVTT
setting-export-subtitle-word-level = Legendas por palavra
setting-export-subtitle-word-level-help = Uma legenda por palavra em vez de linhas de legenda.
setting-export-subtitles-with-audio = Legendas com áudio
setting-export-subtitles-with-audio-help = Sempre escrever legendas junto com o áudio exportado.
setting-reading-aids-rsvp-wpm = Velocidade do RSVP
setting-reading-aids-rsvp-wpm-help = Palavras por minuto da apresentação visual serial rápida.
setting-reading-aids-rsvp-pacing = Ritmo do RSVP
setting-reading-aids-rsvp-pacing-help = O que move a palavra do RSVP adiante: seu próprio temporizador, ou a fala.
choice-reading-aids-rsvp-pacing-timer = seu próprio temporizador
choice-reading-aids-rsvp-pacing-external = a fala
setting-reading-aids-rsvp-clause-pause = Pausa de oração do RSVP
setting-reading-aids-rsvp-clause-pause-help = Tempo extra depois de uma vírgula, dois pontos, travessão ou colchete, em porcentagem do tempo de uma palavra.
setting-reading-aids-rsvp-sentence-pause = Pausa de frase do RSVP
setting-reading-aids-rsvp-sentence-pause-help = Tempo extra no fim de uma frase, em porcentagem.
setting-reading-aids-rsvp-paragraph-pause = Pausa de parágrafo do RSVP
setting-reading-aids-rsvp-paragraph-pause-help = Tempo extra no fim de um parágrafo, em porcentagem.
setting-reading-aids-rsvp-long-word-len = Palavra longa do RSVP
setting-reading-aids-rsvp-long-word-len-help = Palavras mais longas que esta quantidade de letras recebem tempo extra.
setting-reading-aids-rsvp-long-word-step = Incremento de palavra longa do RSVP
setting-reading-aids-rsvp-long-word-step-help = Tempo extra por letra além do comprimento de uma palavra longa, em porcentagem.
setting-reading-aids-rsvp-long-word-max = Máximo de palavra longa do RSVP
setting-reading-aids-rsvp-long-word-max-help = Tempo extra máximo que uma palavra longa recebe, em porcentagem.
setting-reading-aids-rsvp-show-previous = Palavra anterior do RSVP
setting-reading-aids-rsvp-show-previous-help = Mostrar também a palavra anterior.
setting-reading-aids-rsvp-show-next = Próxima palavra do RSVP
setting-reading-aids-rsvp-show-next-help = Mostrar também a próxima palavra.
setting-reading-aids-rsvp-position = Posição do RSVP
setting-reading-aids-rsvp-position-help = Onde a palavra do RSVP aparece.
choice-reading-aids-rsvp-position-top-left = superior esquerda
choice-reading-aids-rsvp-position-top-center = topo central
choice-reading-aids-rsvp-position-top-right = superior direita
choice-reading-aids-rsvp-position-center-left = meio à esquerda
choice-reading-aids-rsvp-position-center = meio
choice-reading-aids-rsvp-position-center-right = meio à direita
choice-reading-aids-rsvp-position-bottom-left = inferior esquerda
choice-reading-aids-rsvp-position-bottom-center = base central
choice-reading-aids-rsvp-position-bottom-right = inferior direita
setting-reading-aids-rsvp-font-size-pt = Tamanho do RSVP
setting-reading-aids-rsvp-font-size-pt-help = Tamanho da palavra do RSVP na interface gráfica.
setting-reading-aids-rsvp-lead-words = Antecipação do RSVP
setting-reading-aids-rsvp-lead-words-help = Com o ritmo pela fala, mostrar esta quantidade de palavras à frente da palavra falada.
setting-reading-aids-bionic = Leitura biônica
setting-reading-aids-bionic-help = Desenhar o início de cada palavra em negrito.
setting-reading-aids-bionic-options-ratio = Proporção biônica
setting-reading-aids-bionic-options-ratio-help = Quanto de cada palavra fica em negrito.
setting-reading-aids-bionic-options-min-word-len = Palavra mais curta biônica
setting-reading-aids-bionic-options-min-word-len-help = Palavras mais curtas que isto são deixadas de lado.
setting-reading-aids-bionic-options-skip-numbers = Biônica ignora números
setting-reading-aids-bionic-options-skip-numbers-help = Deixar de lado palavras com dígitos.
setting-reading-aids-bionic-options-skip-urls = Biônica ignora endereços
setting-reading-aids-bionic-options-skip-urls-help = Deixar de lado endereços da web e de e-mail.
setting-reading-aids-bionic-options-skip-code = Biônica ignora código
setting-reading-aids-bionic-options-skip-code-help = Deixar o código de lado.
setting-reading-aids-spacing-line-height = Altura da linha
setting-reading-aids-spacing-line-height-help = Altura da linha em múltiplos do tamanho da fonte; o valor do WCAG é 1,5.
setting-reading-aids-spacing-paragraph-spacing = Espaçamento de parágrafo
setting-reading-aids-spacing-paragraph-spacing-help = Espaço depois de cada parágrafo, em múltiplos do tamanho da fonte.
setting-reading-aids-spacing-letter-spacing = Espaçamento de letra
setting-reading-aids-spacing-letter-spacing-help = Espaço extra entre letras, em múltiplos do tamanho da fonte.
setting-reading-aids-spacing-word-spacing = Espaçamento de palavra
setting-reading-aids-spacing-word-spacing-help = Espaço extra entre palavras, em múltiplos do tamanho da fonte.
setting-reading-aids-font-family = Fonte
setting-reading-aids-font-family-help = A fonte de leitura da interface gráfica; qualquer família instalada pode ser digitada.
choice-reading-aids-font-family-system-ui = a fonte do sistema
choice-reading-aids-font-family-sans = sem serifa
choice-reading-aids-font-family-serif = com serifa
choice-reading-aids-font-family-monospace = monoespaçada
choice-reading-aids-font-family-atkinson = Atkinson Hyperlegible
choice-reading-aids-font-family-opendyslexic = OpenDyslexic
choice-reading-aids-font-family-lexend = Lexend
setting-reading-aids-font-size-pt = Tamanho da fonte
setting-reading-aids-font-size-pt-help = O tamanho da fonte da interface gráfica.
setting-reading-aids-font-weight = Peso da fonte
setting-reading-aids-font-weight-help = 400 é regular, 700 é negrito.
setting-reading-aids-ruler-mode = Régua de leitura
setting-reading-aids-ruler-mode-help = Marcar a linha atual, ou uma faixa de linhas.
choice-reading-aids-ruler-mode-off = desligada
choice-reading-aids-ruler-mode-current-line = linha atual
choice-reading-aids-ruler-mode-ruler = régua
setting-reading-aids-ruler-scope = A régua cobre
setting-reading-aids-ruler-scope-help = Uma linha visual quebrada, ou a linha inteira.
choice-reading-aids-ruler-scope-row = uma linha visual
choice-reading-aids-ruler-scope-line = a linha inteira
setting-reading-aids-ruler-rows-above = Linhas da régua acima
setting-reading-aids-ruler-rows-above-help = Linhas da faixa acima da atual.
setting-reading-aids-ruler-rows-below = Linhas da régua abaixo
setting-reading-aids-ruler-rows-below-help = Linhas da faixa abaixo da atual.
setting-reading-aids-ruler-mask-outside = Máscara da régua
setting-reading-aids-ruler-mask-outside-help = Escurecer as linhas fora da faixa.
setting-reading-aids-syllables = Sílabas
setting-reading-aids-syllables-help = Desenhar palavras divididas em sílabas com um ponto médio; a fala não muda.
setting-reading-aids-difficult-words = Palavras difíceis
setting-reading-aids-difficult-words-help = Sublinhar palavras raras, e nomeá-las ao mover entre palavras com verbosidade alta.
setting-reading-aids-syllable-options-separator = Separador de sílaba
setting-reading-aids-syllable-options-separator-help = O que é desenhado entre sílabas.
setting-reading-aids-syllable-options-left-min = Primeira quebra da sílaba
setting-reading-aids-syllable-options-left-min-help = Menos letras antes da primeira quebra.
setting-reading-aids-syllable-options-right-min = Última quebra da sílaba
setting-reading-aids-syllable-options-right-min-help = Menos letras depois da última quebra.
setting-reading-aids-syllable-options-min-word-len = Palavra mais curta para sílabas
setting-reading-aids-syllable-options-min-word-len-help = Palavras mais curtas que isto nunca são divididas.
setting-reading-aids-syllable-options-skip-urls = Sílabas ignoram endereços
setting-reading-aids-syllable-options-skip-urls-help = Deixar de lado endereços da web e de e-mail.
setting-reading-aids-syllable-options-skip-code = Sílabas ignoram código
setting-reading-aids-syllable-options-skip-code-help = Deixar o código de lado.
setting-preview-auto-reload = Recarregar a pré-visualização
setting-preview-auto-reload-help = Recarregar a pré-visualização do navegador depois de cada salvamento, através de um pequeno servidor apenas neste computador.
setting-preview-live = Pré-visualização ao vivo
setting-preview-live-help = Com a recarga ligada, recarregar também quando a digitação pausa.
setting-lexicon-glossary = Glossário
setting-lexicon-glossary-help = Seu próprio glossário, consultado antes do dicionário: linhas termo: definição, ou o JSON do Star. Não definido usa glossary.txt na pasta de configurações.
setting-lexicon-data-file = Arquivo de dicionário
setting-lexicon-data-file-help = O dicionário de definir palavra, lexicon-en.twlex. Não definido procura ao lado do programa.
setting-stats-enabled = Estatísticas de leitura
setting-stats-enabled-help = Contar o tempo lido em voz alta, o ponto mais distante e as sessões de cada documento.
setting-interface-language = Idioma da interface
setting-interface-language-help = O idioma das próprias palavras do textweaver, alterado imediatamente. A voz o acompanha quando o motor tem uma para ele; senão a voz permanece.
choice-interface-language-en = English
choice-interface-language-es = Español
choice-interface-language-fr = Français
choice-interface-language-de = Deutsch
choice-interface-language-pt = Português
choice-interface-language-ar = العربية
choice-interface-language-en-xa = teste: acentuado
choice-interface-language-ar-xb = teste: direita para a esquerda
setting-interface-rtl = Exibição direita para a esquerda
setting-interface-rtl-help = Se o leitor de terminal reordena o texto direita para a esquerda para exibição: automático deixa a cargo dos terminais que fazem isso sozinhos. A fala e o leitor de tela sempre recebem o texto na ordem de leitura.
choice-interface-rtl-auto = automático
choice-interface-rtl-on = ligado
choice-interface-rtl-off = desligado
setting-gui-announce = Anúncios
setting-gui-announce-help = Como as mensagens da janela chegam ao leitor de tela, a partir do próximo início: uma região dinâmica, ou notificações de UI Automation (só Windows).
choice-gui-announce-live = região dinâmica
choice-gui-announce-uia = notificações de UI Automation

## Units, said after a number.

settings-unit-words-per-minute = palavras por minuto
settings-unit-percent = por cento
settings-unit-semitones = semitons
settings-unit-milliseconds = milissegundos
settings-unit-words = palavras
settings-unit-times = vezes
settings-unit-places = casas
settings-unit-columns = colunas
settings-unit-lines = linhas
settings-unit-seconds = segundos
settings-unit-steps = passos
settings-unit-megabytes = megabytes
settings-unit-files = arquivos
settings-unit-letters = letras
settings-unit-points = pontos
settings-unit-rows = linhas

## Settings sections.

section-speech = Fala
section-highlight = Realce
section-normalization = Como o texto é falado
section-reading = Leitura
section-display = Exibição
section-editing = Edição
section-library = Biblioteca
section-keyboard = Teclado
section-accessibility = Acessibilidade
section-export = Exportar
section-reading-aids = Recursos de leitura
section-preview = Pré-visualização
section-lexicon = Definir palavra
section-stats = Estatísticas de leitura
section-interface = Interface
section-gui = Janela

## Edit mode: entering, leaving, saving, and typing.

# $key makes a new document.
edit-no-document = Nenhum documento para editar. Pressione { $key } para um novo.
# $line is the line at the caret, as echoed.
edit-mode-on-brief = Modo de edição ligado. { $line }
# $save and $finish are the keys that save and leave edit mode; $line is
# the line at the caret.
edit-mode-on = Modo de edição ligado. Salvar: { $save }. Terminar: { $finish }. { $line }
# Keep the letters s, d and c: they are the keys that answer.
edit-unsaved-question = { $title } tem alterações não salvas. Salvar, descartar ou cancelar? Pressione s, d ou c, ou Seta para cima e para baixo e Enter. Escape cancela.
edit-save-changes-title = Salvar as alterações em { $title }?
edit-choice-save = Salvar, e continuar
edit-choice-discard = Descartar as alterações
edit-choice-cancel = Cancelar, continuar editando
edit-save-failed = Não foi possível salvar: { $error }. Ainda editando.
# The Save As prompt; $path is the suggested file.
edit-save-as-label = Salvar como, Enter para { $path }
# $name is a file name. Keep the letters y and n.
edit-file-exists-question = { $name } já existe. Substituir? y ou n.
edit-mode-off = Modo de edição desligado.
edit-mode-off-discarded = Alterações descartadas. Modo de edição desligado.
# The title of a new, unsaved document.
edit-untitled = Sem título
edit-new-document = Novo documento pronto para editar.
# $key turns on edit mode.
edit-nothing-to-save = Nada para salvar. Ligue o modo de edição com { $key } para fazer alterações.
# $key turns on edit mode; $what is what the user tried to do.
edit-not-editing =
    { $what ->
        [type] Ligue o modo de edição com { $key } para digitar.
        [change-text] Ligue o modo de edição com { $key } para alterar o texto.
        [delete-text] Ligue o modo de edição com { $key } para excluir texto.
        [undo] Ligue o modo de edição com { $key } para desfazer.
        [redo] Ligue o modo de edição com { $key } para refazer.
        [replace-text] Ligue o modo de edição com { $key } para substituir texto.
        [insert] Ligue o modo de edição com { $key } para inserir no texto.
        [cut-text] Ligue o modo de edição com { $key } para recortar texto.
        [move-cells] Ligue o modo de edição com { $key } para mover entre células da tabela.
        [delete-words] Ligue o modo de edição com { $key } para excluir palavras.
        [paste] Ligue o modo de edição com { $key } para colar.
        [citation] Ligue o modo de edição com { $key } para inserir uma citação.
        [bibliography] Ligue o modo de edição com { $key } para inserir uma bibliografia.
       *[format] Ligue o modo de edição com { $key } para formatar texto.
    }
# A paste: $n characters.
edit-pasted =
    { $n ->
        [one] Colou 1 caractere.
       *[other] Colou { $n } caracteres.
    }
# $start is how the pasted text starts.
edit-pasted-start =
    { $n ->
        [one] Colou 1 caractere: { $start }
       *[other] Colou { $n } caracteres: { $start }
    }
edit-insert-failed = Não foi possível inserir: { $error }
# $start and $end are character positions, $len the text's length.
edit-range-out-of-text = Não é possível alterar os caracteres { $start } a { $end }: o texto tem { $len }.
edit-change-failed = Não foi possível alterar o texto: { $error }
edit-delete-failed = Não foi possível excluir: { $error }
edit-list-ended = Lista terminada.
# Said when Enter continues a bulleted list.
edit-bullet = marcador
edit-table-divider = divisor de cabeçalho de tabela
edit-end-of-line-stop = Fim da linha.
edit-start-of-line-stop = Início da linha.
edit-end-of-line-content = fim da linha

## Edit mode: formatting, undo, tables, images, and replace.

# $what names the formatting command; $level is a heading level, and
# $cols and $rows a table's size.
edit-format-done =
    { $what ->
        [bold] Negrito.
        [italic] Itálico.
        [underline] Sublinhado.
        [strikethrough] Tachado.
        [code] Código.
        [code-block] Bloco de código.
        [link] Link.
        [bulleted-list] Lista com marcadores.
        [numbered-list] Lista numerada.
        [block-quote] Citação.
        [horizontal-rule] Linha horizontal inserida.
        [table-row] Linha de tabela adicionada.
        [heading-level] Nível de cabeçalho { $level }.
        [table] Inseriu uma tabela, { $cols } colunas por { $rows } linhas.
       *[heading] Cabeçalho.
    }
# The command toggled its markup off.
edit-format-removed =
    { $what ->
        [bold] Negrito removido.
        [italic] Itálico removido.
        [underline] Sublinhado removido.
        [strikethrough] Tachado removido.
        [code] Código removido.
        [code-block] Bloco de código removido.
        [link] Link removido.
        [bulleted-list] Lista com marcadores removida.
        [numbered-list] Lista numerada removida.
        [block-quote] Citação removida.
        [horizontal-rule] Linha horizontal inserida removida.
        [table-row] Linha de tabela adicionada removida.
        [heading-level] Nível de cabeçalho { $level } removido.
        [table] Tabela inserida, { $cols } colunas por { $rows } linhas, removida.
       *[heading] Cabeçalho removido.
    }
edit-format-unchanged =
    { $what ->
        [bold] Negrito: nada mudou.
        [italic] Itálico: nada mudou.
        [underline] Sublinhado: nada mudou.
        [strikethrough] Tachado: nada mudou.
        [code] Código: nada mudou.
        [code-block] Bloco de código: nada mudou.
        [link] Link: nada mudou.
        [bulleted-list] Lista com marcadores: nada mudou.
        [numbered-list] Lista numerada: nada mudou.
        [block-quote] Citação: nada mudou.
        [horizontal-rule] Linha horizontal inserida: nada mudou.
        [table-row] Linha de tabela adicionada: nada mudou.
        [heading-level] Nível de cabeçalho { $level }: nada mudou.
        [table] Tabela inserida, { $cols } colunas por { $rows } linhas: nada mudou.
       *[heading] Cabeçalho: nada mudou.
    }
# Added after a formatting message; $text is the start of the selection.
edit-format-selected = Selecionado: { $text }
edit-heading-level-now = Nível de cabeçalho { $level }.
# $line is the line at the caret after the undo or redo.
edit-undo-redo =
    { $what ->
        [undo] Desfez.
       *[redo] Refez.
    }
edit-undo-redo-line =
    { $what ->
        [undo] Desfez. { $line }
       *[redo] Refez. { $line }
    }
edit-nothing-to-undo = Nada para desfazer.
edit-nothing-to-redo = Nada para refazer.
edit-not-a-table-size = Não é um tamanho de tabela: { $text }. Digite colunas e linhas, por exemplo 3 por 2.
# $name is the image's file name.
edit-image-inserted = Inseriu a imagem { $name }. A descrição dela está selecionada; digite para substituí-la.
edit-image-failed = Não foi possível inserir a imagem: { $error }.
# $query is the text to find.
edit-no-matches = Nenhuma ocorrência de { $query }.
# $n matches of $query were found; the replacement is asked next.
edit-replace-with =
    { $n ->
        [one] 1 ocorrência de { $query }. Substituir por?
       *[other] { $n } ocorrências de { $query }. Substituir por?
    }

## Edit mode: autosave and recovering unsaved work.

edit-recovery-write-failed = Não foi possível escrever a cópia de recuperação: { $error }. Salve em breve; o { -brand } continuará tentando.
edit-recovery-writing-again = A cópia de recuperação está sendo escrita de novo.
# $title is the document; $when is how long ago its work was saved.
edit-recovery-offer = O { -brand } fechou com alterações não salvas em { $title }, salvas { $when }. Recuperá-las agora? Seta para cima e para baixo escolhem, Enter confirma.
edit-recovery-title = Recuperar o trabalho não salvo em { $title }?
edit-recovery-yes = Sim, recuperar { $title } e continuar editando
edit-recovery-no = Não, descartar as alterações não salvas
edit-recovery-discarded = Descartou as alterações não salvas em { $title }.
edit-recovered = Recuperou o trabalho não salvo em { $title }. Lembre-se de salvar.
edit-recovery-postponed = Recuperação adiada. O trabalho não salvo será oferecido de novo da próxima vez.

## Find and replace, one match at a time.

# $title is replace-match-title. Keep the letters r, s and a: they are the
# keys that answer.
replace-match-question = { $title }. Pressione r para substituir, s para pular, a para substituir tudo, Escape para parar.
# The replace list's title when no match is being asked about.
replace-title = Substituir
# $n is this match's number, $total the number of matches, $line the line
# number, and $context the text of that line.
replace-match-title = Ocorrência { $n } de { $total }, linha { $line }: { $context }
replace-item-this = Substituir esta
replace-item-skip = Pular esta
replace-item-rest = Substituir todo o resto
# $state is common-on or common-off.
replace-item-match-case = Diferenciar maiúsculas: { $state }
replace-item-whole-words = Somente palavras inteiras: { $state }
replace-failed = Não foi possível substituir: { $error }
# Said after switching match case; $state is common-on or common-off, and
# $n is the number of matches now.
replace-match-case-now =
    { $n ->
        [one] Diferenciar maiúsculas { $state }. 1 ocorrência.
       *[other] Diferenciar maiúsculas { $state }. { $n } ocorrências.
    }
replace-whole-words-now =
    { $n ->
        [one] Somente palavras inteiras { $state }. 1 ocorrência.
       *[other] Somente palavras inteiras { $state }. { $n } ocorrências.
    }
# $query is the text that was searched for.
replace-no-matches = Nenhuma ocorrência de { $query }.
replace-replaced =
    { $n ->
        [one] Substituiu 1 ocorrência.
       *[other] Substituiu { $n } ocorrências.
    }
replace-replaced-skipped = Substituiu { $n }, pulou { $skipped }.
replace-stopped = Parou. Substituiu { $n }, pulou { $skipped }.

## Saving in the background.

writes-still-saving = Ainda salvando. Aguarde.
writes-not-written-in-time = Algumas alterações não puderam ser escritas a tempo: o disco não está respondendo.
# $error is the system's reason.
writes-save-failed = Não foi possível salvar: { $error }. Ainda editando.
# $name is the bookmark's name, $pct where it is.
writes-bookmark-set = Marcador { $name } definido em { $pct } por cento.
writes-bookmark-not-saved = O marcador { $name } está definido por enquanto, mas não pôde ser salvo: { $error }.
writes-recovery-copy-failed = Não foi possível escrever a cópia de recuperação: { $error }. Salve em breve; o { -brand } continuará tentando.
writes-recovery-copy-resumed = A cópia de recuperação está sendo escrita de novo.
# $name is the saved file's name.
writes-saved = Salvou { $name }. Ainda editando.

## Files changed on disk. $name is a file name. Keep the letters y and
## n: they are the keys that answer.

disk-replace-question = { $name } já existe. Substituir? y ou n.
# A prompt label, also said with a full stop after it.
disk-not-replaced = Não substituído. Digite outro nome
# $key is the key for Save As.
disk-not-saved = Não salvo. Ainda editando. Salvar Como, { $key }, mantém as duas versões.
disk-kept-open-version = Manteve a versão aberta.
disk-overwrite-question = { $name } mudou no disco desde que você o abriu. Salvar sobre essas alterações? y ou n.
disk-reload-question = { $name } mudou no disco. Recarregar? y ou n.

## Marks found again after a file changed outside textweaver.

relocate-reading-position = sua posição de leitura
relocate-bookmarks =
    { $n ->
        [one] 1 marcador
       *[other] { $n } marcadores
    }
relocate-notes =
    { $n ->
        [one] 1 nota
       *[other] { $n } notas
    }
relocate-highlights =
    { $n ->
        [one] 1 realce
       *[other] { $n } realces
    }
# Lists of relocate-* items: "a and b", and "a, b, and c", where $rest is
# every item but the last, joined by commas.
relocate-join-two = { $a } e { $b }
relocate-join-more = { $rest }, e { $last }
# $items is a list of the items above; $n how many marks it counts in all.
relocate-moved =
    { $n ->
        [one] { $items } foi movido para corresponder
       *[other] { $items } foram movidos para corresponder
    }
relocate-lost =
    { $n ->
        [one] { $items } não pôde ser encontrado e está marcado
       *[other] { $items } não puderam ser encontrados e estão marcados
    }
# $clauses are relocate-moved and relocate-lost, joined by a comma.
relocate-changed = O arquivo mudou; { $clauses }.

## New documents from templates.

# The built-in templates' names.
templates-essay = Redação
templates-report = Relatório
templates-notes = Notas
# One of the user's own templates in the list; $name is its file name.
templates-yours = { $name }, seu modelo
# $folder is where the user's own templates go.
templates-intro =
    { $n ->
        [one] Novo documento a partir de um modelo, 1 modelo. Enter escolhe. Seus próprios modelos vão em { $folder }.
       *[other] Novo documento a partir de um modelo, { $n } modelos. Enter escolhe. Seus próprios modelos vão em { $folder }.
    }
# The title given when none is typed.
templates-untitled = Sem título
# $template is the template's name, $title the document's, $date today's date (2026-09-26).
templates-created = Novo documento a partir do modelo { $template }: { $title }. Datado de { $date }. O cursor está onde a escrita começa. Lembre-se de salvar.

## Markdown structure said in edit mode, before a line's text or as it
## is typed.

mdline-heading-level = cabeçalho nível { $level }
mdline-bullet = marcador
# A numbered list item; $n is its number.
mdline-item = item { $n }
# $item is mdline-bullet or mdline-item.
mdline-task-done = { $item }, tarefa concluída
mdline-task-not-done = { $item }, tarefa não concluída
mdline-table-row = linha de tabela
mdline-quote = citação
mdline-code-fence = cerca de código
mdline-task = tarefa
# Said as "1. " is typed at the start of a line; $n is the number as typed.
mdline-numbered-item = item numerado { $n }

## Moving through tables by row and cell. $dir is next (forward) or
## previous (backward).

tables-not-in-table = Fora de uma tabela.
tables-edge-of-table =
    { $dir ->
        [next] Fim da tabela.
       *[previous] Início da tabela.
    }
tables-edge-of-row =
    { $dir ->
        [next] Fim da linha.
       *[previous] Início da linha.
    }
# $cell is the cell's text, after its column header and a colon when it has one.
tables-header-row = Linha de cabeçalho, { $cell }
tables-row = Linha { $row }, { $cell }
# High verbosity: $message is what the move said, then where it is.
tables-with-position = { $message }. Linha { $row } de { $rows }, coluna { $col } de { $cols }
# Say Position in a table.
tables-position = Tabela, linha { $row } de { $rows }, coluna { $col } de { $cols }.

## Authoring quick wins: word count, links, clipboard, table cells,
## deleting words, and cycling settings.

# Code block languages said with ordinary words; proper names such as
# Python are not translated.
authoring-language-jsx = JavaScript com JSX
authoring-language-tsx = TypeScript com JSX
authoring-language-shell = shell
authoring-language-batch = lote do Windows
authoring-language-c-header = cabeçalho C
authoring-language-cpp = C mais mais
authoring-language-csharp = C sharp
authoring-language-diff = diff
authoring-language-plain-text = texto simples
authoring-grammar-not-in-build = A verificação de gramática não está nesta versão.
# $count is $n with thousands separators.
authoring-word-count-selection =
    { $n ->
        [one] 1 palavra na seleção.
       *[other] { $count } palavras na seleção.
    }
authoring-word-count-document =
    { $n ->
        [one] 1 palavra no documento.
       *[other] { $count } palavras no documento.
    }
# $text is the link's text.
authoring-link-no-address = O link { $text } não tem endereço.
authoring-link-address = Endereço do link: { $url }
authoring-link-named-address = Link { $text }, endereço: { $url }
authoring-no-link = Nenhum link no cursor.
authoring-typing-echo =
    { $echo ->
        [characters-and-words] Eco de digitação: caracteres e palavras.
        [characters] Eco de digitação: caracteres.
        [words] Eco de digitação: palavras.
       *[none] Eco de digitação: nenhum.
    }
authoring-nothing-to-copy = Nada selecionado para copiar.
# $text is the first words of what was copied.
authoring-copied = Copiado: { $text }
authoring-copied-sentence = Copiou a frase: { $text }
authoring-nothing-to-cut = Nada selecionado para recortar.
authoring-cut = Recortado: { $text }
authoring-not-in-table = Fora de uma tabela.
# $dir is next (moving forward) or previous.
authoring-table-edge =
    { $dir ->
        [next] Fim da tabela.
       *[previous] Início da tabela.
    }
# A column with no header text.
authoring-table-column = coluna { $n }
# Moving into a new row: $header is the column's header, $content the cell.
authoring-table-cell-row = Linha { $row }. { $header }: { $content }
authoring-nothing-to-select = Nada para selecionar.
authoring-selected-all =
    { $n ->
        [one] Selecionou tudo, 1 palavra.
       *[other] Selecionou tudo, { $count } palavras.
    }
authoring-space-deleted = Espaço excluído.
# $text is the word deleted.
authoring-deleted = { $text } excluída.
# $key is the terminal's own paste key.
authoring-nothing-copied = Nada copiado no { -brand } ainda. Use a colagem do seu terminal, por exemplo { $key }.
authoring-verbosity =
    { $level ->
        [low] Verbosidade: baixa.
        [high] Verbosidade: alta.
       *[normal] Verbosidade: normal.
    }
authoring-punctuation =
    { $level ->
        [none] Pontuação: nenhuma.
        [all] Pontuação: toda.
       *[some] Pontuação: alguma.
    }

## Markdown lint (edit mode). A problem is said after "Lint: ".

lint-heading-level = nível de cabeçalho { $level } depois do nível { $prev }; use o nível { $use }.
# $reference is the link reference's name.
lint-link-reference = a referência de link { $reference } não tem definição.
lint-bare-url = endereço web solto; coloque-o entre colchetes angulares ou transforme-o em um link com nome.
# $marker and $used are bullet names: lint-marker-dash and the others.
lint-list-marker = marcador de lista { $marker }; esta lista usa { $used }.
lint-marker-dash = traço
lint-marker-star = asterisco
lint-marker-plus = mais
lint-marker-other = outro
# $n is how many tabs and spaces there are.
lint-trailing-tabs-empty-line = tabulações ou espaços em uma linha vazia.
lint-trailing-tabs-line-end = tabulações ou espaços no fim da linha.
lint-trailing-spaces-empty-line =
    { $n ->
        [one] 1 espaço em uma linha vazia.
       *[other] { $n } espaços em uma linha vazia.
    }
lint-trailing-spaces-line-end =
    { $n ->
        [one] 1 espaço no fim da linha.
       *[other] { $n } espaços no fim da linha.
    }
# $key turns on edit mode.
lint-not-editing = O Lint verifica o Markdown que você escreve. Ligue o modo de edição com { $key } primeiro.
lint-not-markdown = O Lint verifica Markdown, e este documento não é Markdown.
lint-none = Nenhum problema de Lint.
# $count is $n with thousands separators.
lint-no-more =
    { $n ->
        [one] Nenhum outro problema de Lint. 1 problema de Lint no total.
       *[other] Nenhum outro problema de Lint. { $count } problemas de Lint no total.
    }
lint-no-earlier =
    { $n ->
        [one] Nenhum problema de Lint anterior. 1 problema de Lint no total.
       *[other] Nenhum problema de Lint anterior. { $count } problemas de Lint no total.
    }
# $message is one of the problems above.
lint-said = Lint: { $message }
# Added at high verbosity.
lint-line = Linha { $line }.

## Grammar checking (Harper). $message is Harper's own message, in English.

# $words are the words the problem is about.
grammar-said = Gramática: { $message } As palavras: { $words }.
# Said after grammar-said when the first fix removes the words.
grammar-fix-remove = Correção: remover as palavras.
grammar-fix = Correção: { $fix }.
# A fix in the fixes list that removes the words.
grammar-remove-the-words = Remover as palavras
grammar-none = Nenhum problema de gramática encontrado.
# $count is $n with thousands separators.
grammar-no-more =
    { $n ->
        [one] Nenhum outro problema de gramática. 1 problema de gramática no total.
       *[other] Nenhum outro problema de gramática. { $count } problemas de gramática no total.
    }
grammar-no-earlier =
    { $n ->
        [one] Nenhum problema de gramática anterior. 1 problema de gramática no total.
       *[other] Nenhum problema de gramática anterior. { $count } problemas de gramática no total.
    }
# $key opens the fixes list.
grammar-lists-fixes = { $key } lista as correções.
# Added at high verbosity.
grammar-line = Linha { $line }.
# $described is grammar-said (and its fix) without the last full stop.
grammar-no-fix = { $described } Nenhuma correção a oferecer.
grammar-fixes =
    { $n ->
        [one] { $words }: 1 correção.
       *[other] { $words }: { $n } correções.
    }
grammar-fixes-edit =
    { $n ->
        [one] { $words }: 1 correção. Enter faz a alteração.
       *[other] { $words }: { $n } correções. Enter faz a alteração.
    }
grammar-left-as-is = Deixado como está.
# $fix is the fix chosen; $key turns on edit mode.
grammar-fix-not-editing = { $fix }. Ligue o modo de edição com { $key } para alterar o texto.
grammar-removed = Removido.
grammar-changed = Alterado para { $fix }.
grammar-change-failed = Não foi possível alterar o texto: { $error }

## Spell checking.

# Said for an apostrophe when a word is spelled out letter by letter.
spell-apostrophe = apóstrofo
spell-not-available = A verificação ortográfica não está disponível: esta versão não tem lista de palavras.
spell-none-found = Nenhum erro ortográfico encontrado.
# $count is $n with thousands separators.
spell-no-more =
    { $n ->
        [one] Nenhum outro erro ortográfico. 1 possível erro ortográfico no total.
       *[other] Nenhum outro erro ortográfico. { $count } possíveis erros ortográficos no total.
    }
spell-no-earlier =
    { $n ->
        [one] Nenhum erro ortográfico anterior. 1 possível erro ortográfico no total.
       *[other] Nenhum erro ortográfico anterior. { $count } possíveis erros ortográficos no total.
    }
# Added at high verbosity.
spell-line = Linha { $line }.
spell-no-misspelled-word = Nenhuma palavra com erro ortográfico no cursor.
# $word is the misspelled word; $n how many suggestions follow.
spell-suggestions =
    { $n ->
        [0] { $word }: sem sugestões.
        [one] { $word }: 1 sugestão.
       *[other] { $word }: { $n } sugestões.
    }
spell-suggestions-edit =
    { $n ->
        [0] { $word }: sem sugestões. Enter substitui a palavra.
        [one] { $word }: 1 sugestão. Enter substitui a palavra.
       *[other] { $word }: { $n } sugestões. Enter substitui a palavra.
    }
# $word is the suggestion chosen; $key turns on edit mode.
spell-replace-not-editing = { $word }. Ligue o modo de edição com { $key } para alterar o texto.
spell-replaced = Substituída por { $word }.
spell-replace-failed = Não foi possível substituir: { $error }
spell-left-as-is = Deixada como está.
spell-added-for-session = Adicionou { $word } à sua lista de palavras para esta sessão.
spell-added = Adicionou { $word } à sua lista de palavras.
spell-save-failed = Não foi possível salvar sua lista de palavras: { $error }
# After a save; $count is $n with thousands separators.
spell-count =
    { $n ->
        [0] Nenhum erro ortográfico.
        [one] 1 possível erro ortográfico.
       *[other] { $count } possíveis erros ortográficos.
    }

## The terminal reader's startup.

# $wanted is the speech backend asked for, $backend the one used instead.
tui-setup-backend-unavailable = O motor de fala { $wanted } não está disponível; usando { $backend }.
# Shown inside tui-setup-speech-failed as its $error.
tui-setup-backend-not-built = o motor { $backend } não foi compilado
tui-setup-speech-failed = A fala não pôde iniciar ({ $error }); executando em silêncio.
tui-setup-cannot-save = Não é possível salvar configurações ou posições: { $error }.
tui-setup-keymap-ignored = Arquivo de teclas ignorado: { $error }.
# The first-run welcome. Each value names the key for an action: $play
# reads and pauses, $stop stops, $heading moves to the next heading,
# $help opens the help, $quit quits.
tui-setup-welcome = Bem-vindo ao { -brand }. { $play } lê em voz alta e pausa, { $stop } para, { $heading } move para o próximo cabeçalho, { $help } abre a ajuda, e { $quit } sai.
# Said at startup without a document. $open, $new, and $help name the
# keys for Open, New Document, and Help.
tui-setup-no-document = Nenhum documento está aberto. Pressione { $open } para abrir um, { $new } para um novo, ou { $help } para ajuda.

## The terminal reader's launch.

# $name is the file asked for on the command line; $error says why.
tui-could-not-open = Não foi possível abrir { $name }: { $error }

## Copying in the terminal reader.

tui-clip-system = Copiado com a área de transferência do sistema, porque este terminal não aceita texto copiado.
# $error is why: tui-clip-not-available, tui-clip-not-built, or the
# system's own message.
tui-clip-failed = Não foi possível copiar com a área de transferência do sistema: { $error }. Enviado ao terminal em vez disso.
tui-clip-not-available = a área de transferência do sistema não está disponível
tui-clip-not-built = esta versão não tem área de transferência do sistema

## The terminal reader's screen.

# The title line's start; $title is the document's title or
# tui-title-no-document.
tui-title = { -brand }: { $title }
tui-title-no-document = sem documento
# The screen without a document. $keys names the keys for the action.
tui-empty-no-document = Nenhum documento está aberto.
tui-empty-open = Abrir um: { $keys }.
tui-empty-help = Ajuda: { $keys }.
tui-empty-quit = Sair: { $keys }.
# The key hints while a yes-or-no question waits. y, n, and a are the
# answer keys the reader takes; $escape names the Escape key.
tui-hints-confirm = y sim  n ou a não  { $escape } não
# Key hint labels, each shown after its key on the bottom line.
tui-hint-play = reproduzir
tui-hint-sentence = frase
tui-hint-faster = mais rápido
tui-hint-slower = mais devagar
tui-hint-close-rsvp = fechar RSVP
tui-hint-quit = sair
tui-hint-save = salvar
tui-hint-finish = terminar
tui-hint-undo = desfazer
tui-hint-bold = negrito
tui-hint-heading = cabeçalho
tui-hint-commands = comandos
tui-hint-next-line = próxima linha
tui-hint-previous-line = linha anterior
tui-hint-again = de novo
tui-hint-read-on = continuar lendo
tui-hint-leave = sair
tui-hint-paragraph = parágrafo
tui-hint-find = localizar
tui-hint-mark = marcar
tui-hint-lines = linhas
tui-hint-keys = teclas
# The list overlay's border: $n is the focused item's number, $count
# the number of items.
tui-list-title = { $n } de { $count }, { $title }

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
goto-word-start = início
goto-word-end = fim

language-voices-loading = A lista de vozes ainda está carregando, então a voz atual continua falando.

## The window (GUI)

gui-open-title = Abrir um documento
gui-open-documents = Documentos que o textweaver lê
gui-open-all-files = Todos os arquivos
gui-open-no-dialog = O seletor de arquivos do sistema não abriu. Digite o caminho do documento.
gui-text-size = Tamanho do texto { $size } pontos.
gui-text-size-largest = Tamanho do texto { $size } pontos, o maior.
gui-text-size-smallest = Tamanho do texto { $size } pontos, o menor.
gui-font = Fonte: { $family }.
gui-font-unchanged = Fonte sem alterações.
gui-font-list = Fonte

## The Braille pass (Wave 5, W5x): pages in paged documents such as a PDF.
## $page and $n are page numbers, $label a printed page label such as iv,
## $pages the number of pages. Keep the page first: a 40-cell Braille
## display shows the start of the line.

status-page = página { $page } de { $pages }
status-page-labelled = página { $label }, { $n } de { $pages }
status-position-page = { $page }, { $pct }%
pages-position = Página { $page } de { $pages }.
pages-position-labelled = Página { $label }, { $n } de { $pages }.
pages-none = Este documento não tem páginas.
pages-no-such-page = Não há página { $page }. As páginas vão de 1 a { $pages }.
pages-label = Página { $label }
pages-outline-item = Página { $label }: { $text }
lists-pages-title =
    { $n ->
        [one] Páginas, { $n } página
       *[other] Páginas, { $n } páginas
    }
lists-pages-title-filtered = Páginas, { $shown } de { $n } correspondem a { $filter }
lists-pages-intro =
    { $n ->
        [one] Páginas, { $n } página. Digite para filtrar, Enter vai até uma página, Escape fecha.
       *[other] Páginas, { $n } páginas. Digite para filtrar, Enter vai até uma página, Escape fecha.
    }
lists-pages-here = Você está em { $heading }.
lists-filter-cleared-pages =
    { $n ->
        [one] Filtro limpo, { $n } página.
       *[other] Filtro limpo, { $n } páginas.
    }
lists-filter-none-pages = Nenhuma página corresponde a { $query }. Backspace remove letras.
lists-filter-matched-pages =
    { $n ->
        [one] { $n } página corresponde.
       *[other] { $n } páginas correspondem.
    }
prompt-go-to-pages = Ir para página, ou linha 12, porcentagem, início ou fim
goto-not-a-target-pages = Não é um destino válido: { $text }. Digite um número de página, linha e um número, uma porcentagem como 50%, início ou fim.
goto-word-page = página

## Wave 5 (W5y): o filtro da biblioteca, o dicionário e as velocidades.

# The library list filtered: $shown of $n documents match $filter.
library-title-filtered = Biblioteca, { $shown } de { $n } correspondem a { $filter }
# The filter was emptied: $n documents are shown.
library-filter-cleared =
    { $n ->
        [one] Filtro limpo, { $n } documento.
       *[other] Filtro limpo, { $n } documentos.
    }
# No document matches the filter $query.
library-filter-none = Nenhum documento corresponde a { $query }. Backspace remove letras.
# $n documents match the filter.
library-filter-matched =
    { $n ->
        [one] { $n } documento corresponde.
       *[other] { $n } documentos correspondem.
    }
# Said once when define word is used while the dictionary file is still opening.
define-still-loading = O dicionário ainda está carregando.
# Configurações adicionadas por W5y.
setting-speech-dectalk-library = Biblioteca do DECtalk
setting-speech-dectalk-library-help = A biblioteca do DECtalk a carregar; não definido procura nos locais de costume.
setting-speech-piper-voices = Pasta de vozes do Piper
setting-speech-piper-voices-help = A pasta de vozes do Piper; não definido usa a pasta piper na pasta de dados do textweaver.
setting-speech-piper-voice = Voz do Piper
setting-speech-piper-voice-help = A voz do Piper para começar, pelo id; não definido usa a primeira instalada.
setting-speech-piper-phonemizer = Fonetizador do Piper
setting-speech-piper-phonemizer-help = Como o Piper transforma texto em sons: a biblioteca espeak-ng se instalada, essa biblioteca ou o do textweaver.
choice-speech-piper-phonemizer-auto = automático
choice-speech-piper-phonemizer-library = biblioteca espeak-ng
choice-speech-piper-phonemizer-rust = o do textweaver
setting-speech-voice-params = Velocidade e tom por voz
setting-speech-voice-params-help = A velocidade e o tom com que cada voz foi usada por último; escolher a voz de novo os traz de volta.
setting-editing-author = Autor
setting-editing-author-help = O autor escrito nos novos documentos feitos a partir de um modelo; vazio deixa em branco.
