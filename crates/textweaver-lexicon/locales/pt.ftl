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
define-dictionary-damaged = Não foi possível ler o arquivo do dicionário: { $error } Só o seu glossário é pesquisado.
define-glossary-problem = Não foi possível ler seu glossário: { $error } Só o dicionário é pesquisado.
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
profiles-read-failed = Não foi possível ler o arquivo de perfis, então ele é tratado como vazio: { $error } Salvar um perfil substitui o arquivo.
profiles-save-failed = Não foi possível salvar os perfis: { $error } Verifique se a pasta de configurações pode ser gravada.
profiles-none-to-export = Ainda não há perfis para exportar.
profiles-exported =
    { $n ->
        [one] Exportou 1 perfil para { $file }.
       *[other] Exportou { $n } perfis para { $file }.
    }
profiles-export-failed = Não foi possível exportar os perfis: { $error } Verifique se a pasta pode ser gravada.
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
stats-turned-off = As estatísticas de leitura estão desligadas. O que foi registrado é mantido.

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
stats-clear-item = Remover as estatísticas de leitura
stats-clear-failed = Não foi possível remover as estatísticas de leitura: { $error }. Verifique se a pasta de dados pode ser gravada.
stats-off-cli = As estatísticas de leitura estão desligadas: stats.enabled é false nas configurações.

## Continue reading and every computer's statistics (the sync wave, S6).

continue-title = Continuar a ler
# $n is the number of documents listed.
continue-intro =
    { $n ->
        [one] Continuar a ler: 1 documento, o mais recente primeiro.
       *[other] Continuar a ler: { $n } documentos, os mais recentes primeiro.
    }
continue-empty = Sem posições ainda. Ler as salva.
# One row, meaning first: the title, how far in, the computer, and how
# long ago (continue-ago-*).
continue-item = { $title }, { $pct } por cento, { $device }, { $when }
continue-this-computer = este computador
continue-ago-now = agora mesmo
continue-ago-minutes =
    { $n ->
        [one] há 1 minuto
       *[other] há { $n } minutos
    }
continue-ago-hours =
    { $n ->
        [one] há 1 hora
       *[other] há { $n } horas
    }
continue-ago-days =
    { $n ->
        [one] há 1 dia
       *[other] há { $n } dias
    }
name-continue-reading = Continuar a ler
action-continue-reading = Continuar a ler: os documentos deste computador com uma posição guardada, de qualquer computador, os mais recentes primeiro
name-add-library-folder = Adicionar uma pasta à biblioteca
action-add-library-folder = Adicionar uma pasta à biblioteca: escolha-a no navegador de arquivos
name-edit-document-details = Editar detalhes
action-edit-document-details = Editar os detalhes do documento: título, autor, DOI e ISBN
prompt-document-details = Detalhes do documento

## Edit a document's details by hand.

# $name is the document's title; said when the form opens.
details-intro = Detalhes de { $name }. Tab muda de campo, Enter guarda, Escape cancela.
details-label-title = Título
details-label-author = Autor
details-label-doi = DOI
details-label-isbn = ISBN
# A field's drawn label: its label, then $n of $total fields.
details-prompt-label = { $label }, { $n } de { $total }
# Said on moving to a field: its label and value (or nav-blank).
details-field = { $label }: { $value }
# $fields lists the fields saved, by their labels.
details-saved = Detalhes guardados: { $fields }.
details-unchanged = Detalhes sem alterações.
details-cancelled = Cancelado. Detalhes sem alterações.
# $text is what was typed in the DOI or ISBN field.
details-not-a-doi = Não é um DOI: { $text }. Corrija-o ou apague-o.
details-not-an-isbn = Não é um ISBN: { $text }. Corrija-o ou apague-o.
details-no-file = Este documento não tem ficheiro, por isso não tem detalhes para editar.
details-save-failed = Não foi possível guardar os detalhes: { $error } Tente de novo.
# The statistics list could not wait for the sync folder.
stats-others-slow = Sem outros computadores: pasta lenta.
stats-untitled = Documento sem título
# One computer's share of a document: $device, $time, $sessions.
stats-computer =
    { $sessions ->
        [one] { $device }: { $time }, 1 sessão
       *[other] { $device }: { $time }, { $sessions } sessões
    }
stats-by-computer-off = Por computador: oculto. Enter mostra.
stats-by-computer-on = Por computador: visível. Enter oculta.

## Navigation. $dir is next or previous; $what is a kind-* or unit-*
## noun and $unit its key (heading, list-item, sentence), for languages
## whose words agree with the noun.

nav-blank = em branco
# Said for a picture that has no description (no alternative text).
nav-no-description = sem descrição
# The document overview: the title first, then the counts. $time is overview-time.
overview-line = { $title }. Títulos: { $headings }, tabelas: { $tables }, imagens: { $pictures }, notas de rodapé: { $footnotes }. { $time }
# Reading time left from the cursor at the current rate.
overview-time =
    { $minutes ->
        [0] Falta menos de um minuto.
        [one] Falta cerca de 1 minuto.
       *[other] Faltam cerca de { $minutes } minutos.
    }
# Said when the reading pass changes and as reading starts in a skim. $pass is a reading-pass-* name.
reading-pass-changed = Passagem: { $pass }.
reading-pass-full = texto inteiro
reading-pass-first-sentences = primeiras frases
reading-pass-headings = títulos
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
playback-speech-error = Erro de fala: { $error } Tente de novo, ou use o comando Reiniciar a fala.
playback-end-of-section = Fim da seção. { $key } para continuar.
playback-time-up =
    { $minutes ->
        [one] O tempo acabou após 1 minuto. { $key } para continuar.
       *[other] O tempo acabou após { $minutes } minutos. { $key } para continuar.
    }
playback-repeat-slower = Repetindo mais devagar, a { $rate } palavras por minuto.
playback-recall-prompt = Diga o que lembra de { $section }. { $key } para continuar.

## The title line and Say Status.

# The title line's position.
status-position = linha { $line } de { $lines }
status-mode = Modo { $mode }
status-modified = modificado
status-self-voicing = autofala
status-hybrid = híbrido
status-screen-reader = modo leitor de tela
status-rate-spoken = { $wpm } palavras por minuto
status-rate = { $wpm } ppm
# The window's status bar, last: reading time left at the current rate.
status-time-left =
    { $minutes ->
        [0] menos de um minuto restante
        [one] 1 minuto restante
       *[other] { $minutes } minutos restantes
    }
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
app-terminal-only = Este comando funciona no leitor de terminal.
settings-save-failed = Não foi possível salvar as configurações: { $error } Suas alterações continuam em uso até você sair.
settings-outside-kept = As configurações alteradas fora do textweaver foram mantidas.
edit-still-editing = Ainda editando.
goto-not-a-target = Não é um destino válido: { $text }. Digite um número de linha, uma porcentagem como 50%, início ou fim.

## Opening a document.

open-opened = Abriu { $title }.
open-resumed = Abriu { $title }. Retomado em { $pct } por cento.
open-resumed-synced = Abriu { $title }. Retomado em { $pct } por cento, de outro dispositivo.

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
# One line of the keyboard shortcuts list: the command's name-* (first, so
# type-ahead finds commands), its keys (inside a 40-cell Braille line), an
# action-* help, and its category-* title.
help-entry = { $name }: { $keys }. { $help }. { $category }
help-unknown-command = Comando desconhecido: { $text }.
help-shortcuts-intro = Atalhos de teclado, { $n } comandos. Digite para filtrar. Page Down vai para o próximo grupo, F1 explica um comando, Enter o executa, Escape fecha.
help-shortcuts-title = Atalhos de teclado
# The keyboard shortcuts list, filtered: $filter is what was typed.
help-shortcuts-title-matching = Atalhos de teclado que correspondem a { $filter }
# $n commands of $total match the filter.
help-shortcuts-filter-match = { $n } de { $total } comandos correspondem.
help-shortcuts-filter-none = Nenhum comando corresponde a { $query }. Backspace remove letras.
help-shortcuts-filter-cleared = Filtro limpo, { $n } comandos.
# Moving into a group of the keyboard shortcuts list: its name, its
# size, then the row ($item, with its place in the list).
help-shortcuts-group-item =
    { $group }, { $n ->
        [one] 1 comando
       *[other] { $n } comandos
    }. { $item }
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
help-highlights = Realçar a seleção ou a frase, ou remover um realce: { $highlight }. Com um nome: { $first } a { $fifth }, ou a lista: { $named }. Listar realces: { $list }.
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
help-access-window = Quem lê: { $key } alterna entre textweaver lê em voz alta e meu leitor de tela lê.
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
action-repeat-sentence-slower = Dizer de novo a frase no cursor mais devagar e voltar à velocidade habitual
action-rsvp-toggle = Mostrar ou ocultar o RSVP: uma palavra de cada vez, a partir do cursor
action-rsvp-play-pause = Iniciar ou pausar o RSVP
action-rsvp-faster = RSVP mais rápido
action-rsvp-slower = RSVP mais devagar
action-rsvp-position-next = Mover a palavra do RSVP para o próximo lugar na tela
action-reading-level = Dizer o nível de leitura do documento ou da seleção
action-document-overview = Dizer o título do documento, quantos títulos, tabelas, imagens e notas de rodapé tem, e cerca de quantos minutos faltam
action-reading-pass = Mudar o que a leitura diz: o texto inteiro, a primeira frase de cada parágrafo com os títulos, ou só os títulos
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
action-search-options = Escolher como Localizar e Substituir comparam: maiúsculas, palavras inteiras, expressão regular, entre linhas
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
action-highlight-as = Realçar a seleção ou a frase com um nome escolhido da paleta de realce
action-highlight-name-1 = Realçar com o primeiro nome da paleta, ou remover esse realce
action-highlight-name-2 = Realçar com o segundo nome da paleta, ou remover esse realce
action-highlight-name-3 = Realçar com o terceiro nome da paleta, ou remover esse realce
action-highlight-name-4 = Realçar com o quarto nome da paleta, ou remover esse realce
action-highlight-name-5 = Realçar com o quinto nome da paleta, ou remover esse realce
action-collect-highlights = Escrever os realces de um nome como uma lista em Markdown
action-export-study-sheet = Exportar as notas e realces como uma folha de estudo em Markdown, agrupada por cabeçalho
action-export-study-sheet-by-name = Exportar a folha de estudo com os realces agrupados por nome
action-self-test = Testar-se nas notas e realces: Enter mostra cada resposta
action-make-cards = Criar cartões de estudo a partir das notas, dos realces e de seus títulos
action-study-cards = Estudar os cartões: Enter mostra a resposta, 1 a 4 a avaliam
action-list-cards = Listar os cartões de estudo: Enter vai até a origem de um cartão, Delete o remove
action-grade-again = Avaliar o cartão em estudo: De novo, não lembrado
action-grade-hard = Avaliar o cartão em estudo: Difícil, lembrado com esforço
action-grade-good = Avaliar o cartão em estudo: Bom, lembrado
action-grade-easy = Avaliar o cartão em estudo: Fácil, lembrado na hora
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
action-export-knowledge-graph = Exportar o grafo de conhecimento: cada ligação entre notas, como lista Markdown, JSON, DOT, GraphML, Mermaid, PlantUML ou CSV
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
action-paste = Colar a área de transferência; o texto formatado de um navegador ou processador de texto vira Markdown
action-paste-plain-text = Colar a área de transferência como texto simples, sem nada da formatação
action-context-menu = Abrir o menu de contexto: recortar, copiar, colar e os comandos para onde o cursor está
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
action-contents-panel = Mostrar o painel Sumário ao lado do documento e ir até ele, ou fechá-lo de dentro dele: Enter vai a um título
action-notes-panel = Mostrar o painel Notas ao lado do documento e ir até ele, ou fechá-lo de dentro dele: Enter vai a uma nota
action-toggle-header = Mostrar ou ocultar o cabeçalho, a barra de comandos acima do documento
action-toggle-toolbar = Mostrar ou ocultar a barra de ferramentas, a barra dos botões de leitura
action-next-region = Ir para a próxima parte da janela: o cabeçalho, o painel, o documento ou a barra de ferramentas
action-previous-region = Ir para a parte anterior da janela
action-command-palette = Executar qualquer comando pelo nome
action-settings = Abrir as configurações: cada opção com sua ajuda; Esquerda e Direita mudam um valor
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
restart-failed = Não foi possível reiniciar a fala: { $error } Aguarde um momento e tente de novo.
restart-start-failed = Não foi possível reiniciar a fala: a inicialização falhou.
restart-no-engine = Nenhum motor de fala está disponível; o { -brand } permanece silencioso. Veja Troubleshooting, No speech at all, na documentação.
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
settingsio-export-failed = Não foi possível exportar as configurações: { $error } Verifique se a pasta pode ser gravada.
# $path is the file; $error the system's reason.
settingsio-read-failed = Não foi possível ler { $path }: { $error } Verifique o arquivo e importe de novo.
settingsio-nothing-to-import = Nada para importar: suas configurações já correspondem a esse arquivo.
settingsio-cancelled-unchanged = Cancelado. Nada foi alterado.
settingsio-import-failed = Não foi possível importar as configurações: { $error } Verifique se a pasta de configurações pode ser gravada.
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
opening-pdf-password = está protegido por senha. Remova a senha em um programa de PDF e abra-o de novo.
opening-old-office = é um arquivo antigo do Microsoft Office. Salve-o em um formato mais novo, como .docx, e abra esse.
opening-rar = é um arquivo RAR, que não abre. Extraia-o primeiro, ou use ZIP ou 7z.
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

lean-citations-not-in-build = As citações não estão nesta versão do { -brand }.
lean-publish-not-in-build = Exportar e pré-visualizar não estão nesta versão do { -brand }. tw convert ainda converte.

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
voice-list-failed = Não foi possível listar as vozes: { $error } Escolha outro motor no menu Fala.
# $shown is voices-shown ("12 voices: English, all engines."). Enter,
# Space, Delete and Escape are the list's own keys.
voice-manager-intro = Gerenciador de vozes. { $shown } Enter usa uma voz e fala uma amostra, ou baixa uma; { $preview } ouve uma prévia; Espaço marca uma favorita; Delete remove uma voz baixada; Escape fecha.
voice-more-ready =
    { $n ->
        [one] Mais uma voz de outro motor está na lista.
       *[other] Mais { $n } vozes de outros motores estão na lista.
    }
voice-preview = Prévia: { $voice }.
voice-preview-sample = { $voice }. Um pequeno jabuti xereta viu dez cegonhas felizes.
voice-preview-starting = Prévia: { $voice }, iniciando { $engine }.
voice-preview-not-installed = { $voice } ainda não foi baixada. Enter a baixa, depois de uma pergunta.
voice-preview-unavailable = { $engine } não pode iniciar aqui para uma prévia. Enter muda para esse motor.
voice-preview-engine-failed = Não foi possível iniciar { $engine } para a prévia de { $voice }.
voice-preview-failed = Não foi possível ouvir a prévia de { $voice }: { $error } Tente outra voz.
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
voice-remove-failed = Não foi possível remover { $voice }: { $error } Verifique se a pasta dele pode ser gravada.
voice-downloading-catalog = Baixando a lista de vozes do Piper.
voice-downloading = Baixando { $voice }.
voice-downloading-percent = Baixando { $voice }, { $pct } por cento.
voice-details-failed = Não foi possível ler os detalhes da voz: { $error } Verifique a conexão e tente de novo.
voice-download-stopped = O download parou.
voice-catalog-fetched = A lista de vozes do Piper tem { $voices } vozes em { $languages } idiomas. O comando Vozes as lista.
voice-catalog-failed = Não foi possível baixar a lista de vozes: { $error } Verifique a conexão e tente de novo.
# $licence describes the voice's licence, in a sentence of its own.
voice-installed = { $voice } está instalada. { $licence } O comando Vozes a lista.
voice-download-failed = Não foi possível baixar { $voice }: { $error } Escolha a voz de novo para tentar outra vez.
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

common-no-document = Nenhum documento está aberto.
# Said after "Could not export:", so it starts in lower case. $path is a
# folder or a file; $error the system's reason.
publish-cannot-write-to = não é possível escrever em { $path }: { $error } Verifique se a pasta pode ser gravada.
publish-cannot-write = não é possível escrever { $path }: { $error } Verifique se a pasta dele pode ser gravada.
publish-start-failed = Não foi possível iniciar a exportação: { $error } Aguarde um momento e tente de novo.
publish-export-error = Não foi possível exportar: { $error } Corrija isso e exporte de novo.
# $format is the format's name, such as PDF, HTML, or Word.
publish-exporting = Exportando para { $format }.
publish-theme-title = Tema para a página HTML
publish-theme-intro = Tema para a página HTML? { $first } primeiro, { $n } opções. Escape cancela.
publish-writing-preview = Escrevendo a pré-visualização.
publish-preview-error = Não foi possível escrever a pré-visualização: { $error } Corrija isso e abra a prévia de novo.
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
publish-live-on-needs-reload = Pré-visualização ao vivo ligada. Ela funciona com a recarga automática, que está desligada; ligue-a com Recarregar a prévia sozinha.
publish-live-off = Pré-visualização ao vivo desligada: a pré-visualização recarrega só depois de salvar.
# $error is the converter's reason.
publish-export-failed = A exportação para { $format } falhou: { $error } Tente outro formato.
publish-preview-failed = A pré-visualização falhou: { $error } Salve para tentar de novo.
# The converter's warnings: how many, and the first one.
publish-warnings =
    { $n ->
        [one] 1 aviso: { $first }
       *[other] { $n } avisos; o primeiro: { $first }
    }
# $file is the file's name, $folder its folder; $warned is empty or a
# space and publish-warnings.
publish-exported = Exportado para { $format }: { $file }. Abrir? y ou n. Em { $folder }.{ $warned }
publish-report = Relatório salvo como { $file }.
publish-report-issues =
    { $n ->
        [one] 1 item não acessível
       *[other] { $n } itens não acessíveis
    }; veja o relatório.
publish-report-failed = Não foi possível salvar o relatório: { $error } Verifique se a pasta de saída permite gravação.
publish-preview-written-served = Pré-visualização escrita. Abrindo-a no navegador. Ela recarrega por conta própria depois de cada salvamento.{ $warned }
publish-preview-written = Pré-visualização escrita. Abrindo-a no navegador. Salvar a escreve de novo; depois pressione F5 no navegador.{ $warned }
publish-preview-updated = Pré-visualização atualizada.
publish-preview-updated-press-f5 = Pré-visualização atualizada. Pressione F5 no navegador.
publish-server-failed = Não foi possível iniciar o servidor de recarga da pré-visualização ({ $error }); abrindo o arquivo em vez disso.
publish-render-failed = Não foi possível renderizar o texto: { $error } Saia do modo de edição para ler o texto.
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
notes-none = Nenhuma nota. Para adicionar uma: { $key }.
notes-list-title = Notas
notes-list-intro =
    { $n ->
        [one] Notas, 1 item. Enter vai até uma nota, Delete a exclui, F2 a edita, Espaço abre suas ligações. C cria um cartão.
       *[other] Notas, { $n } itens. Enter vai até uma nota, Delete a exclui, F2 a edita, Espaço abre suas ligações. C cria um cartão.
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
notes-highlighted-at = Realçado, { $name }, em { $pct } por cento: { $text }
notes-highlighted = Realçado, { $name }: { $text }
# An item in the highlights list. $name is the highlight's palette name;
# $lost is yes when the text was not found after the file changed.
notes-highlight-item =
    { $lost ->
        [yes] { $name }: { $text }, linha { $line }, não encontrado depois que o arquivo mudou
       *[no] { $name }: { $text }, linha { $line }
    }
notes-no-highlights = Nenhum realce. Para criar um: { $key }.
notes-highlights-title = Realces
notes-highlights-intro =
    { $n ->
        [one] Realces, 1 item. Enter vai até um, Delete o remove, F2 muda seu nome, Espaço mostra só seu nome. C cria um cartão.
       *[other] Realces, { $n } itens. Enter vai até um, Delete o remove, F2 muda seu nome, Espaço mostra só seu nome. C cria um cartão.
    }
# The label said before a highlight's text on jumping to it.
notes-highlight-label = Realce, { $name }
notes-highlight-changed = Realce mudado para { $name }: { $text }
# $n is the palette entry's number, $key the keys of Highlight with a name.
notes-palette-no-entry = Não há nome de realce { $n } na paleta. Para escolher um nome: { $key }.
notes-highlights-named-title = Realces: { $name }
notes-highlights-named-intro =
    { $n ->
        [one] Realces chamados { $name }, 1 item. Espaço mostra todos os realces.
       *[other] Realces chamados { $name }, { $n } itens. Espaço mostra todos os realces.
    }
notes-palette-title = Nomes de realce
# A palette name: the name, how many highlights have it, its shape and its color.
notes-palette-row =
    { $count ->
        [one] { $name }, 1 realce, { $shape }, { $color }
       *[other] { $name }, { $count } realces, { $shape }, { $color }
    }
notes-palette-intro-highlight = Realçar com qual nome? { $n } nomes. Enter escolhe um.
notes-palette-intro-change = Mudar o realce para qual nome? { $n } nomes. Enter escolhe um.
notes-palette-intro-collect = Reunir os realces de qual nome? { $n } nomes. Enter os escreve como lista.
notes-collect-none = Nenhum realce chamado { $name }.
# The collected list's own title (Markdown).
notes-collect-title = Realces chamados { $name }: { $title }
notes-collect-line = (linha { $line })
# Keep the letters y and n: they are the keys that answer.
notes-collect-saved =
    { $n ->
        [one] 1 realce chamado { $name } salvo como { $file }. Abrir? y ou n. Em { $folder }.
       *[other] { $n } realces chamados { $name } salvos como { $file }. Abrir? y ou n. Em { $folder }.
    }
palette-shape-underline = sublinhado
palette-shape-double-underline = sublinhado duplo
palette-shape-bold = negrito
palette-shape-dotted = sublinhado pontilhado
palette-shape-brackets = colchetes
palette-shape-symbol = símbolo
# Shown while reading reaches a note's passage.
notes-signal = Nota: { $text }
# Said after moving onto a note's passage.
notes-has-note = Tem uma nota: { $text }

## Relations between notes (the knowledge graph as lists).

relations-type-conflicts-with = está em conflito com
relations-type-supports = apoia
relations-type-is-example-of = é um exemplo de
relations-type-cites = cita
relations-type-contradicts = contradiz
relations-type-defines = define
relations-type-extends = amplia
relations-type-see-also = veja também
relations-type-precedes = precede
relations-type-follows = segue
relations-count = Ligações: { $out } de saída, { $in } de entrada.
relations-note-title = Ligações de: { $note }
relations-title-filtered = { $title }, filtro: { $filter }
relations-note-intro = Ligações de { $note }: { $out } de saída, { $in } de entrada. Enter segue uma ligação, F2 a altera, Delete a remove. Digite para filtrar por tipo.
relations-out-item = { $type }: { $target }
relations-target-in = { $note }, em { $doc }
relations-target-missing = uma nota não encontrada
relations-empty-note = Nota vazia
relations-incoming-row =
    { $n ->
        [0] O que liga para cá: nada ainda
        [one] O que liga para cá: 1 nota
       *[other] O que liga para cá: { $n } notas
    }
relations-add-row = Adicionar uma ligação
relations-backlinks-title = O que liga para cá: { $note }
relations-backlinks-intro =
    { $n ->
        [one] O que liga para { $note }: 1 nota. Enter vai até ela. Digite para filtrar por tipo.
       *[other] O que liga para { $note }: { $n } notas. Enter vai até uma. Digite para filtrar por tipo.
    }
relations-backlink-item = { $type } esta, de: { $note }
relations-none-in = Nada liga para esta nota ainda.
relations-types-title = Tipo de ligação para: { $note }
relations-types-intro = Escolha o tipo de ligação, 10 tipos. Digite para filtrar.
relations-targets-title = { $type }: qual nota?
relations-targets-intro =
    { $n ->
        [0] Nenhuma outra nota aqui. Escolha uma nota de outro documento.
        [one] Escolha a nota de destino: 1 nota. Enter cria a ligação.
       *[other] Escolha a nota de destino: { $n } notas. Enter cria a ligação.
    }
relations-other-document-row = Uma nota de outro documento
relations-documents-title = Documentos com notas
relations-documents-intro = Documentos com notas: { $n }. Enter lista as notas de um documento.
relations-document-item =
    { $n ->
        [one] { $title }, 1 nota
       *[other] { $title }, { $n } notas
    }
relations-no-other-documents = Nenhum outro documento da biblioteca tem notas.
relations-linked = Ligado: { $type } { $target }.
relations-changed = Ligação alterada: { $type } { $target }.
relations-already = Já ligado: { $type } { $target }.
relations-removed = Ligação removida: { $type } { $target }.
relations-remove-question = Remover esta ligação? y ou n
relations-nothing-to-remove = Aqui só é possível remover uma ligação.
relations-note-gone = Nota não encontrada: foi excluída, ou o documento dela sumiu.
relations-document-missing = Documento não encontrado: { $file }.
relations-no-note-here = Nenhuma nota aqui. Ligações pertencem a notas; adicione uma: { $key }.
relations-filter-cleared = Filtro limpo, { $n } mostrados.
relations-filter-none = Nada corresponde a { $filter }.
relations-filter-matched = Filtro { $filter }: { $n } mostrados.

## Exportar o grafo de conhecimento (B1-g2).

graph-export-title = Exportar o grafo de conhecimento como
graph-export-intro = Grafo de conhecimento, ligações: { $links }. Escolha um formato; a lista Markdown é o texto para ler.
graph-export-empty = Não há ligações entre notas para exportar. Adicione uma na lista de ligações de uma nota.
graph-format-md = Lista Markdown, o texto para ler
graph-format-json = JSON, para Gephi e Cytoscape
graph-format-dot = DOT, para Graphviz
graph-format-graphml = GraphML, para Gephi, Cytoscape e yEd
graph-format-mermaid = Diagrama Mermaid
graph-format-plantuml = Diagrama PlantUML
graph-format-csv = Lista de arestas CSV, para planilhas
graph-export-saved = Grafo de conhecimento salvo como { $file }. Abrir? y ou n. Em { $folder }.
graph-export-failed = Não foi possível escrever o grafo de conhecimento: { $error } Verifique se a pasta pode ser gravada.

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
notes-study-sheet-failed = Não foi possível escrever a folha de estudo: { $error } Verifique se a pasta pode ser gravada.
# The study sheet file's own text (Markdown; the # marks stay in the code).
notes-sheet-title = Folha de estudo: { $title }
notes-sheet-exported = Exportado do { -brand } em { $date }.
notes-sheet-before-first-heading = Antes do primeiro cabeçalho
# After a note's text: its tags, joined with commas.
notes-sheet-tags = (tags: { $tags })
# $name is the highlight's palette name.
notes-sheet-highlighted = Realçado, { $name }.
# On the study sheet grouped by name: the heading a highlight falls under.
notes-sheet-under = Sob: { $heading }
# The study sheet's section of notes, after the names.
notes-sheet-notes = Notas

## O autoteste: perguntas com respostas ocultas (crate::reveal).

reveal-self-test-title = Autoteste: { $title }
reveal-self-test-intro =
    { $n ->
        [one] Autoteste, 1 pergunta. Enter mostra a resposta. Espaço para responder em voz alta.
       *[other] Autoteste, { $n } perguntas. Enter mostra cada resposta. Espaço para responder em voz alta.
    }
reveal-nothing-to-test = Nenhuma nota ou realce para testar. Adicione antes uma nota ou um realce.
reveal-prompt-note = { $note } (em { $section })
reveal-prompt-highlight = O que você realçou em { $section }?
reveal-row-shown = { $prompt } Resposta: { $answer }
reveal-answer = Resposta: { $answer }
reveal-listening = Responda em voz alta agora. Espaço para parar.
reveal-you-said = Você disse: { $words }. Enter mostra a resposta.
reveal-heard-nothing = Nenhuma resposta ouvida. Espaço para tentar de novo.
reveal-no-dictation = Responder em voz alta precisa do ditado, que não está nesta versão.

## Cartões de estudo e a sessão de estudo (crate::cards).

cards-blank = lacuna
cards-recall-question = O que diz “{ $heading }”?
cards-no-document = Abra um documento para criar ou estudar cartões.
cards-nothing-to-make = Nenhuma nota ou realce para criar cartões. Adicione primeiro uma nota ou um realce.
cards-made =
    { $added ->
        [0] Nenhum cartão novo. { $total } cartões ao todo.
        [one] Cartões criados: 1 novo, { $total } ao todo.
       *[other] Cartões criados: { $added } novos, { $total } ao todo.
    }
cards-none-from-item = Nenhum cartão deste item: ele precisa de texto sobre um trecho.
cards-made-one = Cartão criado: { $question }
cards-updated-one = Cartão atualizado: { $question }
cards-none = Ainda não há cartões. Para criá-los: { $key }.
cards-study-title = Estudar cartões: { $title }
cards-study-intro =
    { $n ->
        [one] Estudar cartões, 1 cartão. Enter mostra a resposta, 1 a 4 a avaliam. Espaço para responder em voz alta.
       *[other] Estudar cartões, { $n } cartões. Enter mostra cada resposta, 1 a 4 a avaliam. Espaço para responder em voz alta.
    }
cards-no-session = Nenhuma sessão de estudo. Para começar uma: { $key }.
cards-card-gone = Esse cartão foi removido.
cards-grade-again = De novo
cards-grade-hard = Difícil
cards-grade-good = Bom
cards-grade-easy = Fácil
cards-graded = { $grade }. Cartão { $i } de { $n }. Pergunta: { $question }
# Said when a grade from the palette opens the session again on the next card.
cards-graded-reopen = { $grade }. Estudar cartões, cartão { $i } de { $n }.
cards-session-done =
    { $n ->
        [one] { $grade }. Pronto: o cartão foi avaliado.
       *[other] { $grade }. Pronto: os { $n } cartões foram avaliados.
    }
cards-reversed = Invertido. Pergunta: { $question }
cards-unreversed = Como criado. Pergunta: { $question }
cards-not-reversible = Só cartões de pergunta podem ser invertidos.
cards-kind-cloze = Preencher a lacuna
cards-kind-question = Pergunta
cards-kind-recall = Lembrar
cards-not-graded = ainda não avaliado
cards-last-grade = última avaliação: { $grade }
cards-item = { $kind }: { $question }, { $grade }
cards-list-title = Cartões
cards-list-intro =
    { $n ->
        [one] Cartões, 1 item. Enter vai até a origem, Delete o remove.
       *[other] Cartões, { $n } itens. Enter vai até a origem de um cartão, Delete o remove.
    }
cards-source-label = Origem do cartão
cards-remove-question = Remover este cartão e suas avaliações? y ou n
cards-removed = Cartão removido.
cards-save-failed = Não foi possível salvar os cartões: { $error } Verifique se a pasta de dados pode ser gravada.
# Said first in the study session and the Cards list (B1-f2). $due is how
# many graded cards are due, $new how many were never graded.
cards-due-summary =
    { $due ->
        [one] Para hoje: 1 cartão, { $new } novos.
       *[other] Para hoje: { $due } cartões, { $new } novos.
    }
# When a card is next due, after a grade. $days is whole days, at least 1.
cards-next-in =
    { $days ->
        [one] de novo amanhã
       *[other] de novo em { $days } dias
    }
cards-due-now = para hoje
# A grade or last grade, then when the card is next due: "Good, next in 3 days".
cards-grade-next = { $grade }, { $next }
# Study cards when no card is due and none is new: every card is asked.
cards-nothing-due =
    { $days ->
        [one] Nada para hoje; o próximo cartão é para amanhã. Estudando todos os cartões antecipadamente.
       *[other] Nada para hoje; o próximo cartão é daqui a { $days } dias. Estudando todos os cartões antecipadamente.
    }

## Find, bookmarks, and selection.

marks-cannot-search = Não foi possível buscar: { $error } Verifique o padrão entre as barras.
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
common-bookmark-set = Marcador { $name } definido em { $pct } por cento.
marks-no-bookmarks = Nenhum marcador. Para adicionar um: { $key }.
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
library-scan-failed = Não foi possível examinar a biblioteca: { $error } Verifique as pastas da biblioteca nas configurações.
library-scanning = Examinando a biblioteca.
library-scan-progress = Examinando a biblioteca: { $n } encontrados até agora.
library-scan-stopped = O exame da biblioteca parou inesperadamente. Abra a biblioteca de novo para tentar outra vez.
# $command is the command line that adds a folder; $key names the Open command's key.
library-empty = A biblioteca está vazia. Adicione uma pasta nas Configurações, em Pastas da biblioteca, ou abra um arquivo com { $key }.
library-add-folder-choose = Escolha a pasta a adicionar à biblioteca
library-folder-added = { $name } foi adicionada à biblioteca. Abra a biblioteca para ver os seus documentos.
library-folder-already = { $name } já está na biblioteca.
library-intro =
    { $n ->
        [one] Biblioteca, { $n } documento. Digite para filtrar, Enter abre um, F2 edita os detalhes.
       *[other] Biblioteca, { $n } documentos. Digite para filtrar, Enter abre um, F2 edita os detalhes.
    }
library-title = Biblioteca

## Following links and footnotes.

links-none-here = Nenhum link ou nota de rodapé no cursor.
# $text is the link's text.
common-link-no-address = O link { $text } não tem endereço.
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
citations-insert-failed = Não foi possível inserir a citação: { $error } O texto não foi alterado.
# $what is the identifier being looked up, as the citation library describes it.
citations-looking-up = Procurando { $what }.
citations-lookup-not-started = Não foi possível iniciar a busca: { $error } Aguarde um momento e tente de novo.
# $input is the DOI or ISBN as typed.
citations-lookup-failed = Não foi possível encontrar { $input }: { $error } Verifique o DOI ou o ISBN e a conexão.
citations-no-library-to-add-to = Não há biblioteca para adicionar: o { -brand } não guarda arquivos nesta sessão.
citations-library-save-failed = Não foi possível salvar a biblioteca: { $error } Verifique se a pasta dele pode ser gravada.
# $n is how many citations the document has.
citations-found-no-library =
    { $n ->
        [one] { $n } citação encontrada. O { -brand } não guarda biblioteca nesta sessão.
       *[other] { $n } citações encontradas. O { -brand } não guarda biblioteca nesta sessão.
    }
citations-check-failed = Não foi possível verificar as citações: { $error } Verifique o arquivo da biblioteca de referências.
citations-no-library-to-import-into = Não há biblioteca para importar: o { -brand } não guarda arquivos nesta sessão.
# $file is the file's path.
citations-import-failed = Não foi possível importar { $file }: { $error } Verifique se é um arquivo .bib, .ris ou .json.
# $style is the style's name from the front matter, such as apa.
citations-style-unusable = Não é possível usar o estilo de citação { $style }: { $error } Verifique o nome do estilo no início do documento.
citations-format-failed = Não foi possível formatar as citações: { $error } Verifique as chaves de citação e a biblioteca.
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
citations-bibliography-insert-failed = Não foi possível inserir a bibliografia: { $error } O texto não foi alterado.

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
open-refused-scheme = Não aberto: link { $scheme } bloqueado.
open-refused-missing = Não aberto: arquivo não encontrado.
open-refused-invalid = Não aberto: endereço inválido.
tasks-not-opened = Não aberto.
# Keep the letters y and n: they are the keys that answer.
tasks-open-it-question = Abrir? y ou n

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
# $speech is what was said for the step; $code is the math braille code's name (Nemeth or UEB); $braille is the part's braille in Unicode braille cells, for the Braille display.
mathx-step-braille = { $speech } { $code }: { $braille }

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

message-error = Erro: { $message }
# $name is the theme name in the settings; $used the display name of the theme used instead.
themes-unknown = Não há tema chamado { $name }; usando { $used }.
# $theme is the new theme's display name.
themes-next = Tema { $theme }.
themes-soft-dark-name = { $theme }, escuro suave
themes-choice-aa = { $theme }, atende AA
themes-choice-below-aa = { $theme }, abaixo de AA
themes-below-aa =
    { $count ->
        [one] Abaixo de AA: 1 verificação não atinge o mínimo.
       *[other] Abaixo de AA: { $count } verificações não atingem o mínimo.
    }

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
# On the first run with a screen reader and no mode chosen: hybrid mode is
# used, not asked. $reader is the screen reader found (NVDA, JAWS), or
# access-a-screen-reader; $key names the keys that change the mode.
access-hybrid-inferred = { $reader } está em execução: textweaver lê os documentos em voz alta e deixa as mensagens para seu leitor de tela. { $key } muda isso.
# The window's two modes, when the mode changes; $key changes it again.
access-window-choice-reads-aloud = textweaver lê em voz alta
access-window-choice-screen-reader = meu leitor de tela lê
access-window-mode-help = Quem lê na janela: a voz do textweaver, ou só o seu leitor de tela.
access-window-mode-changed =
    { $mode ->
        [screen-reader] Meu leitor de tela lê: textweaver fica em silêncio e envia o texto ao seu leitor de tela. { $key } muda isso.
        [speaks-messages] textweaver lê em voz alta e fala suas mensagens. { $key } muda isso.
       *[reads-aloud] textweaver lê em voz alta; as mensagens vão para seu leitor de tela. { $key } muda isso.
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
settings-cannot-be = { $label } não pode ser isso: { $error } Escolha outro valor.
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
setting-speech-voice-help = O identificador da voz; não definido escolhe uma automaticamente. O comando Vozes as lista.
setting-speech-prefer-voice = Voz preferida
setting-speech-prefer-voice-help = Quando nenhuma voz está definida, a primeira voz cujo nome contém isto, como eloquence.
setting-speech-favorite-voices = Vozes favoritas
setting-speech-favorite-voices-help = Vozes listadas primeiro pelo comando Vozes, por identificador, separadas por vírgulas.
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
setting-speech-pause-heading-ms = Pausa após cabeçalhos
setting-speech-pause-heading-ms-help = Silêncio após um cabeçalho, mais curto em velocidades maiores. 0 o desativa.
setting-speech-pause-paragraph-ms = Pausa após parágrafos
setting-speech-pause-paragraph-ms-help = Silêncio após um parágrafo, mais curto em velocidades maiores. 0 o desativa.
setting-speech-pause-list-item-ms = Pausa após itens de lista
setting-speech-pause-list-item-ms-help = Silêncio após um item de lista, mais curto em velocidades maiores. 0 o desativa.
setting-speech-markup-pauses = Pausas escritas como marcação
setting-speech-markup-pauses-help = Lê a marcação de pausa de um documento, como <break time="1s"/>, como uma pausa. Desative para documentos que citam essa marcação.
setting-speech-output-device = Dispositivo de saída
setting-speech-output-device-help = O dispositivo de som em que a fala é reproduzida, pelo seu identificador. Sem definição usa o padrão do sistema, assim como um dispositivo que não está conectado.
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
setting-highlight-color-help = A cor atrás da palavra sendo lida. Escolha um nome ou digite um código hexadecimal. Padrão: a cor do tema.
setting-highlight-sentence-color = Cor do realce de frase
setting-highlight-sentence-color-help = A cor atrás da frase sendo lida. Escolha um nome ou digite um código hexadecimal. Padrão: a cor do tema.
setting-highlight-palette = Nomes de realce
setting-highlight-palette-help = Até oito nomes para seus realces, cada um com uma cor e uma forma. Os cinco primeiros têm teclas próprias.
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
setting-normalization-medical-lexicon-enabled = Léxico médico
setting-normalization-medical-lexicon-enabled-help = Ler nomes de medicamentos, termos clínicos e abreviaturas de dose com uma lista de pronúncia médica.
setting-normalization-medical-lexicon-overlay = Arquivo do léxico médico
setting-normalization-medical-lexicon-overlay-help = Suas próprias pronúncias médicas, que têm prioridade sobre as incluídas. Sem definir lê medical-lexicon.toml na pasta de configurações.
setting-reading-auto-resume = Retomar de onde parou
setting-reading-auto-resume-help = Voltar à posição salva quando um documento é aberto.
setting-reading-nav-history-size = Histórico de retorno
setting-reading-nav-history-size-help = Quantos lugares Voltar lembra.
setting-reading-wrap-navigation = Navegação circular
setting-reading-wrap-navigation-help = Mover além do fim do documento continua a partir do início.
setting-reading-cursor-follows-speech = Cursor segue a fala
setting-reading-cursor-follows-speech-help = O cursor se move com a palavra sendo lida.
setting-reading-citations = Citações
setting-reading-citations-help = Citações na leitura contínua: ignoradas, ou ditas por extenso.
choice-reading-citations-off = ignoradas
choice-reading-citations-words = por extenso
setting-reading-ocr = Reconhecer páginas digitalizadas
setting-reading-ocr-help = Ler o texto de PDFs e imagens digitalizados reconhecendo-o (OCR).
setting-reading-ocr-lang = Idioma do texto digitalizado
setting-reading-ocr-lang-help = O idioma do texto digitalizado, como os códigos do Tesseract fra ou deu+eng. Vazio significa o idioma do próprio documento, senão inglês.
choice-reading-ocr-lang- = o do documento
choice-reading-ocr-lang-eng = Inglês
choice-reading-ocr-lang-fra = Francês
choice-reading-ocr-lang-deu = Alemão
choice-reading-ocr-lang-spa = Espanhol
setting-reading-ocr-engine = Motor de OCR
setting-reading-ocr-engine-help = Qual motor reconhece páginas digitalizadas. ocrs para inglês e Tesseract para outros idiomas, ou sempre um deles.
choice-reading-ocr-engine-auto = automático
choice-reading-ocr-engine-ocrs = ocrs
choice-reading-ocr-engine-tesseract = Tesseract
choice-reading-ocr-engine-paddle = PaddleOCR (experimental)
setting-reading-math-engine = Fala da matemática
setting-reading-math-engine-help = Qual motor lê matemática em voz alta. O próprio do textweaver, ou o MathCAT em ClearSpeak ou SimpleSpeak, no idioma do documento. O MathCAT precisa de uma versão que o inclua; senão o próprio do textweaver é usado.
choice-reading-math-engine-builtin = textweaver
choice-reading-math-engine-mathcat = MathCAT ClearSpeak
choice-reading-math-engine-mathcat-simplespeak = MathCAT SimpleSpeak
setting-braille-math-code = Braille matemático
setting-braille-math-code-help = O código braille para matemática em arquivos BRF e ao explorar uma fórmula com o MathCAT. Nemeth ou a matemática do UEB. Precisa de uma versão que inclua o MathCAT; senão, a matemática é escrita com as suas palavras faladas.
choice-braille-math-code-nemeth = Nemeth
choice-braille-math-code-ueb = UEB
setting-braille-table-format = Tabelas em braille
setting-braille-table-format-help = Como os arquivos BRF dispõem as tabelas. Linear, uma linha por fileira com ponto e vírgula entre as entradas; em lista, cada fileira como um título e cada entrada na sua própria linha depois do título da sua coluna; ou em escada, cada entrada duas celas à direita da anterior, para tabelas de até quatro colunas.
choice-braille-table-format-linear = linear
choice-braille-table-format-listed = em lista
choice-braille-table-format-stairstep = em escada
setting-reading-math-display = Matemática na tela
setting-reading-math-display-help = Como a matemática aparece na visão de leitura. Como sua origem, como x^2, ou como Unicode, como x com um 2 sobrescrito. A fala e o modo de edição sempre usam a origem.
choice-reading-math-display-source = origem
choice-reading-math-display-unicode = Unicode
setting-reading-revisions = Alterações controladas
setting-reading-revisions-help = Como as alterações controladas em arquivos do Word, OpenDocument e RTF são lidas. Ditas no lugar com verbosidade alta (automático), sempre, ou nunca, lendo o texto final. Vale ao abrir um documento.
choice-reading-revisions-auto = automático
choice-reading-revisions-marked = sempre dizê-las
choice-reading-revisions-final = só o texto final
setting-reading-stop-at = Parar no fim da seção
setting-reading-stop-at-help = Onde a leitura contínua para sozinha e diz Fim da seção. Nunca, no próximo título de qualquer nível, ou no próximo capítulo: uma quebra de seção, senão um título de nível 1. A leitura continua a partir do título com a tecla de ler.
choice-reading-stop-at-off = nunca
choice-reading-stop-at-heading = próximo título
choice-reading-stop-at-chapter = próximo capítulo
setting-reading-stop-after-minutes = Temporizador de leitura
setting-reading-stop-after-minutes-help = A leitura contínua para no fim da frase após estes minutos de leitura, e avisa. Pausar para o relógio; parar recomeça a contagem. 0 desliga o temporizador.
setting-reading-recall-prompts = Perguntas de recordação
setting-reading-recall-prompts-help = No fim de uma seção, a leitura pede que você diga o que lembra. Se Parar no fim da seção for nunca, a leitura para no próximo cabeçalho para isso. A leitura continua com a tecla de ler.
setting-display-theme = Tema
setting-display-theme-help = O tema de cores.
setting-display-follow-os-theme = Seguir o tema do sistema
setting-display-follow-os-theme-help = Na inicialização, usar um tema claro, escuro ou de alto contraste como o do sistema, a menos que você tenha escolhido um.
setting-display-wrap-width = Largura da quebra de linha
setting-display-wrap-width-help = Quebrar linhas nesta quantidade de colunas; 0 usa a largura inteira.
setting-display-measure = Comprimento da linha
setting-display-measure-help = Quantos caracteres cabem em uma linha da janela, de 25 a 90. 0 preenche a janela. O terminal usa a largura da quebra de linha.
setting-display-tab-width = Largura da tabulação
setting-display-tab-width-help = Colunas que uma tabulação ocupa.
setting-display-show-line-numbers = Números de linha
setting-display-show-line-numbers-help = Mostrar números de linha.
setting-display-scroll-margin = Margem de rolagem
setting-display-scroll-margin-help = Linhas mantidas visíveis acima e abaixo do cursor.
setting-display-hints = Linha de dicas de teclas
setting-display-hints-help = Se o leitor de terminal mostra dicas de teclas na última linha. Automático as mostra com autofala e as oculta com um leitor de tela. F1 e a lista de atalhos de teclado sempre nomeiam as teclas.
choice-display-hints-auto = automático
choice-display-hints-on = ligado
choice-display-hints-off = desligado
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
setting-library-folders-help = Pastas cujos documentos a biblioteca lista, e cujas posições sincronizam entre computadores. Separe as pastas com ponto e vírgula.
setting-keyboard-character-keys = Atalhos de tecla única
setting-keyboard-character-keys-help = Teclas de navegação como h e ponto. Desligado, o ditado e a digitação nunca acionam comandos.
setting-keyboard-preset = Teclas
setting-keyboard-preset-help = As teclas padrão: como o modo de navegação do NVDA e do JAWS, ou as teclas anteriores do textweaver. Usado a partir do próximo início.
choice-keyboard-preset-default = estilo leitor de tela
choice-keyboard-preset-classic = clássico
setting-keyboard-digit-row = Linha de dígitos
setting-keyboard-digit-row-help = Como o terminal reconhece as teclas de dígito para níveis de cabeçalho. Automático, ou um teclado AZERTY francês.
choice-keyboard-digit-row-auto = automático
choice-keyboard-digit-row-azerty = AZERTY
setting-accessibility-mode = Modo de acessibilidade
setting-accessibility-mode-help = Autofala fala tudo; leitor de tela deixa a fala para seu leitor de tela; híbrido vocaliza só a leitura.
choice-accessibility-mode-self-voicing = autofala
choice-accessibility-mode-screen-reader = leitor de tela
choice-accessibility-mode-hybrid = híbrido
setting-accessibility-say-all = Dizer tudo com um leitor de tela
setting-accessibility-say-all-help = Leitura contínua no modo leitor de tela. Uma frase de cada vez na linha de status, ou com a voz do textweaver.
choice-accessibility-say-all-screen = na linha de status
choice-accessibility-say-all-voice = com a voz do textweaver
setting-accessibility-quiet-screen = Tela quieta durante a leitura
setting-accessibility-quiet-screen-help = Manter a tela parada enquanto o textweaver lê em voz alta. Ligado por padrão no modo híbrido.
choice-accessibility-quiet-screen-auto = automático
choice-accessibility-quiet-screen-true = ligado
choice-accessibility-quiet-screen-false = desligado
setting-accessibility-cursor = Cursor
setting-accessibility-cursor-help = Onde o cursor do terminal espera. No que você está trabalhando, ou na linha de status.
choice-accessibility-cursor-follow = segue o foco
choice-accessibility-cursor-status = na linha de status
setting-export-audio-format = Formato de exportação de áudio
setting-export-audio-format-help = O formato que Exportar áudio oferece primeiro. tw export-audio também o usa para nomes de arquivo sem extensão.
choice-export-audio-format-flac = FLAC
choice-export-audio-format-mp3 = MP3
choice-export-audio-format-opus = Opus
choice-export-audio-format-ogg = Ogg Vorbis
choice-export-audio-format-wav = WAV
choice-export-audio-format-m4b = Audiolivro M4B
choice-export-audio-format-mp4 = Vídeo MP4 com legendas
setting-export-subtitle-format = Formato de legenda
setting-export-subtitle-format-help = O formato das legendas escritas sem um nome de arquivo.
choice-export-subtitle-format-srt = SubRip
choice-export-subtitle-format-vtt = WebVTT
setting-export-subtitle-word-level = Legendas por palavra
setting-export-subtitle-word-level-help = Uma legenda por palavra em vez de linhas de legenda.
setting-export-subtitles-with-audio = Legendas com áudio
setting-export-subtitles-with-audio-help = Sempre escrever legendas junto com o áudio exportado.
choice-export-subtitle-format-ass = Karaokê ASS
setting-export-subtitle-karaoke = Karaokê nas legendas
setting-export-subtitle-karaoke-help = Como as linhas de legenda mostram a palavra lida. Desligado, sublinhada ao ser falada (marcas WebVTT) ou uma legenda por palavra, em negrito e sublinhado.
choice-export-subtitle-karaoke-off = Desligado
choice-export-subtitle-karaoke-tags = Sublinhar ao falar
choice-export-subtitle-karaoke-lines = Uma legenda por palavra
setting-export-subtitle-chapters = Arquivo de capítulos
setting-export-subtitle-chapters-help = Também escrever um arquivo de capítulos WebVTT junto às legendas ou ao áudio.
# Names for chapters the document leaves untitled, in audio export.
export-chapter-untitled = Audiolivro
export-chapter-numbered = Capítulo { $number }
# The read-along page (tw export-audio essay.md --out essay.html).
readalong-skip = Ir para o texto
readalong-controls = Áudio
readalong-play = Reproduzir
readalong-pause = Pausar
readalong-back = Uma frase atrás
readalong-forward = Uma frase à frente
readalong-follow = Acompanhar a leitura
readalong-speed = Velocidade
readalong-contents = Conteúdo
readalong-play-section = Reproduzir seção: { $title }
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
setting-lexicon-glossary-help = Seu próprio glossário, consultado antes do dicionário: linhas termo: definição, ou o JSON do star. Não definido usa glossary.txt na pasta de configurações.
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
setting-interface-rtl = Exibição direita para a esquerda
setting-interface-rtl-help = Se o leitor de terminal reordena o texto direita para a esquerda para exibição. Automático deixa a cargo dos terminais que fazem isso sozinhos. A fala e o leitor de tela sempre recebem o texto na ordem de leitura.
choice-interface-rtl-auto = automático
choice-interface-rtl-on = ligado
choice-interface-rtl-off = desligado
setting-gui-announce = Anúncios
setting-gui-announce-help = Como as mensagens da janela chegam ao leitor de tela, a partir do próximo início. Uma região dinâmica, ou notificações de UI Automation (só Windows).
choice-gui-announce-live = região dinâmica
choice-gui-announce-uia = notificações de UI Automation
setting-gui-header = Mostrar o cabeçalho
setting-gui-header-help = Mostra a barra de comandos acima do documento. Desativado, os comandos mantêm suas teclas e itens de menu.
setting-gui-toolbar = Mostrar a barra de ferramentas
setting-gui-toolbar-help = Mostra a barra dos botões de leitura. Desativada, os comandos mantêm suas teclas e itens de menu.
setting-gui-auto-hide-menu = Ocultar a barra de menus
setting-gui-auto-hide-menu-help = Windows: oculta a barra de menus da janela até que Alt ou F10 a mostre. Ela se oculta de novo quando o menu fecha. Sem efeito no Linux, cujos menus são a lista do F10, nem no macOS.
setting-gui-speak-messages = Falar as mensagens do textweaver
setting-gui-speak-messages-help = Quando o textweaver lê em voz alta, falar também as mensagens, a digitação e os movimentos do cursor com a voz dele, para ler de ouvido sem leitor de tela.
setting-gui-sidebar = Painel ao lado do documento
setting-gui-sidebar-help = O painel que a janela mostra ao lado do documento. Nenhum, o Sumário (os títulos) ou as Notas. As teclas de painel o mudam, e a janela lembra o último.
choice-gui-sidebar-off = nenhum
choice-gui-sidebar-contents = Sumário
choice-gui-sidebar-notes = Notas

## Units, said after a number.

settings-unit-words-per-minute =
    { $n ->
        [one] palavra por minuto
       *[other] palavras por minuto
    }
settings-unit-percent = por cento
settings-unit-semitones =
    { $n ->
        [one] semitom
       *[other] semitons
    }
settings-unit-milliseconds =
    { $n ->
        [one] milissegundo
       *[other] milissegundos
    }
settings-unit-words =
    { $n ->
        [one] palavra
       *[other] palavras
    }
settings-unit-places =
    { $n ->
        [one] casa
       *[other] casas
    }
settings-unit-columns =
    { $n ->
        [one] coluna
       *[other] colunas
    }
settings-unit-characters =
    { $n ->
        [one] caractere
       *[other] caracteres
    }
settings-unit-lines =
    { $n ->
        [one] linha
       *[other] linhas
    }
settings-unit-seconds =
    { $n ->
        [one] segundo
       *[other] segundos
    }
settings-unit-steps =
    { $n ->
        [one] passo
       *[other] passos
    }
settings-unit-megabytes =
    { $n ->
        [one] megabyte
       *[other] megabytes
    }
settings-unit-files =
    { $n ->
        [one] arquivo
       *[other] arquivos
    }
settings-unit-letters =
    { $n ->
        [one] letra
       *[other] letras
    }
settings-unit-points =
    { $n ->
        [one] ponto
       *[other] pontos
    }
settings-unit-rows =
    { $n ->
        [one] linha
       *[other] linhas
    }
settings-unit-minutes =
    { $n ->
        [one] minuto
       *[other] minutos
    }

## Settings sections.

section-speech = Fala
section-highlight = Realce
section-normalization = Como o texto é falado
section-reading = Leitura
section-display = Exibição
section-editing = Edição
section-authoring = Autoria
section-library = Biblioteca
section-keyboard = Teclado
section-accessibility = Acessibilidade
section-export = Exportar
section-braille = Braille
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
edit-save-failed = Não foi possível salvar: { $error } Ainda editando. Tente Salvar como.
# The Save As prompt; $path is the suggested file.
edit-save-as-label = Salvar como, Enter para { $path }
# $name is a file name. Keep the letters y and n.
edit-file-exists-question = { $name } já existe. Substituir? y ou n
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
edit-insert-failed = Não foi possível inserir: { $error } O texto não foi alterado.
# $start is the first words of the pasted text.
edit-pasted-lines =
    { $n ->
        [one] 1 linha colada: { $start }
       *[other] { $n } linhas coladas: { $start }
    }
# $start and $end are character positions, $len the text's length.
edit-range-out-of-text = Não é possível alterar os caracteres { $start } a { $end }: o texto tem { $len }.
edit-change-failed = Não foi possível alterar o texto: { $error } O texto não foi alterado.
edit-delete-failed = Não foi possível excluir: { $error } O texto não foi alterado.
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
        [horizontal-rule] Linha horizontal removida.
        [table-row] Linha de tabela removida.
        [heading-level] Nível de cabeçalho { $level } removido.
        [table] Tabela removida.
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
        [horizontal-rule] Linha horizontal: nada mudou.
        [table-row] Linha de tabela: nada mudou.
        [heading-level] Nível de cabeçalho { $level }: nada mudou.
        [table] Tabela: nada mudou.
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
edit-image-failed = Não foi possível inserir a imagem: { $error } O texto não foi alterado.
# $query is the text to find.
common-no-matches = Nenhuma ocorrência de { $query }.
# $n matches of $query were found; the replacement is asked next.
edit-replace-with =
    { $n ->
        [one] 1 ocorrência de { $query }. Substituir por?
       *[other] { $n } ocorrências de { $query }. Substituir por?
    }

## Edit mode: autosave and recovering unsaved work.

common-recovery-write-failed = Não foi possível escrever a cópia de recuperação: { $error }. Salve em breve; o { -brand } continuará tentando.
common-recovery-writing-again = A cópia de recuperação está sendo escrita de novo.
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
replace-match-question = { $title }. A linha: { $context }. Pressione r para substituir, s para pular, a para substituir tudo, Escape para parar.
# The replace list's title when no match is being asked about.
replace-title = Substituir
# $n is this match's number, $total the number of matches, $line the line
# number, $found the matched text, and $result what it becomes (both
# shortened); the -removed form is for an empty replacement. $context in
# replace-match-question is the text of the match's line.
replace-match-title = Ocorrência { $n } de { $total }, linha { $line }: { $found } vira { $result }
replace-match-title-removed = Ocorrência { $n } de { $total }, linha { $line }: { $found } é removido
replace-item-this = Substituir esta
replace-item-skip = Pular esta
replace-item-rest = Substituir todo o resto
# $state is common-on or common-off.
replace-item-match-case = Diferenciar maiúsculas: { $state }
replace-item-whole-words = Somente palavras inteiras: { $state }
replace-failed = Não foi possível substituir: { $error } O texto não foi alterado.
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
replace-replaced =
    { $n ->
        [one] Substituiu 1 ocorrência.
       *[other] Substituiu { $n } ocorrências.
    }
replace-replaced-skipped = Substituiu { $n }, pulou { $skipped }.
replace-stopped = Parou. Substituiu { $n }, pulou { $skipped }.
# Find and replace with regular expressions (B1-fr). $state is common-on
# or common-off; $n is the number of matches now.
replace-item-regex = Expressão regular: { $state }
replace-item-across-lines = Entre linhas: { $state }
replace-regex-now =
    { $n ->
        [one] Expressão regular { $state }. 1 ocorrência.
       *[other] Expressão regular { $state }. { $n } ocorrências.
    }
replace-across-lines-now =
    { $n ->
        [one] Entre linhas { $state }. 1 ocorrência.
       *[other] Entre linhas { $state }. { $n } ocorrências.
    }
# $problem is search-invalid-pattern: switching the option would make the
# pattern invalid.
replace-option-refused = { $problem } A opção não mudou.
# Asked once before replacing all the rest; $n is how many. Keep y and n.
replace-all-question =
    { $n ->
        [one] Substituir a última ocorrência? y ou n
       *[other] Substituir todas as { $n } ocorrências restantes? y ou n
    }
replace-all-declined = Nada foi substituído.
# The search options list, and the options as named in search-options-on.
search-options-title = Opções de busca
search-option-match-case = diferenciar maiúsculas
search-option-whole-words = palavras inteiras
search-option-regex = expressão regular
search-option-across-lines = entre linhas
# Said as Find or Replace opens when an option is on; $list joins the
# options' names with commas.
search-options-on = Opções ativadas: { $list }.
# $at is the character where the pattern fails, counting from 1; $reason
# is the regular expression engine's own explanation (in English).
search-invalid-pattern = Padrão inválido no caractere { $at }: { $reason }.
search-invalid-pattern-anywhere = Padrão inválido: { $reason }.

## Saving in the background.

writes-still-saving = Ainda salvando. Aguarde.
writes-not-written-in-time = Algumas alterações não puderam ser escritas a tempo: o disco não está respondendo.
# $error is the system's reason.
writes-save-failed = Não foi possível salvar: { $error }. Ainda editando.
# $name is the bookmark's name, $pct where it is.
writes-bookmark-not-saved = O marcador { $name } está definido por enquanto, mas não pôde ser salvo: { $error } Verifique se a pasta de dados pode ser gravada.
# $name is the saved file's name.
writes-saved = Salvou { $name }. Ainda editando.

## Files changed on disk. $name is a file name. Keep the letters y and
## n: they are the keys that answer.

disk-replace-question = { $name } já existe. Substituir? y ou n
# A prompt label, also said with a full stop after it.
disk-not-replaced = Não substituído. Digite outro nome
# $key is the key for Save As.
disk-not-saved = Não salvo. Ainda editando. Salvar Como, { $key }, mantém as duas versões.
disk-kept-open-version = Manteve a versão aberta.
disk-overwrite-question = { $name } mudou no disco desde que você o abriu. Salvar sobre essas alterações? y ou n
disk-reload-question = { $name } mudou no disco. Recarregar? y ou n

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

common-not-in-table = Fora de uma tabela.
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
# $parts lists what came in, from the paste-part messages.
paste-markdown = Colado como Markdown: { $parts }
paste-part-heading =
    { $n ->
        [one] 1 título
       *[other] { $n } títulos
    }
paste-part-paragraph =
    { $n ->
        [one] 1 parágrafo
       *[other] { $n } parágrafos
    }
paste-part-list =
    { $n ->
        [one] 1 lista
       *[other] { $n } listas
    }
paste-part-table =
    { $n ->
        [one] 1 tabela
       *[other] { $n } tabelas
    }
paste-part-code =
    { $n ->
        [one] 1 bloco de código
       *[other] { $n } blocos de código
    }
paste-part-quote =
    { $n ->
        [one] 1 citação
       *[other] { $n } citações
    }
paste-part-link =
    { $n ->
        [one] 1 link
       *[other] { $n } links
    }
paste-empty = Nada para colar: a área de transferência está vazia.
paste-converting = Convertendo o texto formatado para colar.
paste-failed = Não foi possível colar: { $error } Tente Colar como texto simples.
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
common-line = Linha { $line }.

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
common-left-as-is = Deixado como está.
# $fix is the fix chosen; $key turns on edit mode.
grammar-fix-not-editing = { $fix }. Ligue o modo de edição com { $key } para alterar o texto.
grammar-removed = Removido.
grammar-changed = Alterado para { $fix }.
grammar-change-failed = Não foi possível alterar o texto: { $error } Nada foi alterado.

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
spell-replace-failed = Não foi possível substituir: { $error } A palavra não foi alterada.
spell-added-for-session = Adicionou { $word } à sua lista de palavras para esta sessão.
spell-added = Adicionou { $word } à sua lista de palavras.
spell-save-failed = Não foi possível salvar sua lista de palavras: { $error } A palavra é reconhecida até você sair.
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
tui-setup-speech-failed = A fala não pôde iniciar ({ $error }); executando em silêncio.
speech-engine-fallback = { $failed } não pôde iniciar; { $engine } está falando no lugar.
speech-engine-fallback-silent = { $failed } não pôde iniciar e nenhum outro motor de fala está disponível; { -brand } fica em silêncio.
tui-setup-cannot-save = Não é possível salvar configurações ou posições: { $error } A leitura funciona. As alterações se perdem ao sair.
tui-setup-keymap-ignored = Arquivo de teclas ignorado: { $error } As teclas padrão valem. Corrija o arquivo e reinicie.
# The first-run welcome. Each value names the key for an action: $play
# reads and pauses, $stop stops, $heading moves to the next heading,
# $help opens the help, $quit quits.
tui-setup-welcome = Bem-vindo ao { -brand }. { $open } abre um documento. { $play } inicia e pausa a leitura, e { $stop } a interrompe. { $palette } lista todos os comandos. { $help } abre a ajuda.
gui-setup-welcome-menus = Bem-vindo ao { -brand }. { $open } abre um documento. { $play } lê e pausa, { $stop } interrompe. { $palette } lista todos os comandos, Alt ou { $menu } os menus. { $help } abre a ajuda.
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
tui-hint-choose = escolher
tui-hint-close = fechar
tui-hint-back = voltar
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
gui-font-list = Fonte

## The Braille pass: pages in paged documents such as a PDF.
## $page and $n are page numbers, $label a printed page label such as iv,
## $pages the number of pages. Keep the page first: a 40-cell Braille
## display shows the start of the line.

status-page = página { $page } de { $pages }
status-page-labelled = página { $label }, { $n } de { $pages }
status-percent = { $pct }%
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

## O filtro da biblioteca, o dicionário e as velocidades.

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
# Configurações.
setting-speech-dectalk-library = Biblioteca do DECtalk
setting-speech-dectalk-library-help = A biblioteca do DECtalk a carregar; não definido procura nos locais de costume.
setting-speech-espeak-helper = Programa auxiliar do eSpeak NG
setting-speech-espeak-helper-help = Executar o eSpeak NG no seu próprio programa auxiliar, para que uma falha do motor não feche o textweaver. Automático usa o programa auxiliar no Windows quando está instalado e, nos outros sistemas, executa o eSpeak NG dentro do textweaver.
choice-speech-espeak-helper-auto = automático
choice-speech-espeak-helper-always = sempre o programa auxiliar
choice-speech-espeak-helper-never = dentro do textweaver
setting-speech-piper-voices = Pasta de vozes do Piper
setting-speech-piper-voices-help = A pasta de vozes do Piper; não definido usa a pasta piper na pasta de dados do textweaver.
setting-speech-piper-voice = Voz do Piper
setting-speech-piper-voice-help = A voz do Piper para começar, pelo id; não definido usa a primeira instalada.
setting-speech-piper-phonemizer = Fonetizador do Piper
setting-speech-piper-phonemizer-help = Como o Piper transforma texto em sons. A biblioteca espeak-ng se instalada, essa biblioteca ou o do textweaver.
choice-speech-piper-phonemizer-auto = automático
choice-speech-piper-phonemizer-library = biblioteca espeak-ng
choice-speech-piper-phonemizer-rust = o do textweaver
setting-speech-voice-params = Velocidade e tom por voz
setting-speech-voice-params-help = A velocidade e o tom com que cada voz foi usada por último. Escolher a voz de novo os traz de volta.
setting-authoring-author = Autor
setting-authoring-author-help = O nome que o { -brand } escreve em comentários, respostas e documentos feitos a partir de um modelo. Vazio significa { -brand } nos comentários e nenhum autor nos modelos. Nunca é tirado do computador.
setting-authoring-track-changes = Controlar alterações em arquivos Word
setting-authoring-track-changes-help = Guardar as edições de um arquivo do Word como alterações controladas que um revisor pode aceitar. Desligado as guarda como Markdown com outro nome.

## The window (GUI): drawn labels, hints, and questions.
## Keep the letters Y and N: they are the keys that answer.

gui-yes = Sim
gui-no = Não
gui-answer-delete = Excluir
gui-answer-remove = Remover
gui-answer-replace = Substituir
gui-question-hint = Y responde sim, N responde não, Escape responde não.
gui-find-title = Localizar e substituir
gui-find-what = Localizar
gui-find-with = Substituir por
gui-find-match-case = Diferenciar maiúsculas
gui-find-whole-words = Palavras inteiras
gui-find-regex = Expressão regular
gui-find-across-lines = Entre linhas
gui-find-next = Localizar próxima
gui-find-next-help = Seleciona a próxima ocorrência.
gui-find-replace = Substituir
gui-find-replace-help = Substitui a ocorrência mostrada e vai para a próxima. O primeiro toque localiza uma ocorrência.
gui-find-replace-all = Substituir tudo
gui-find-replace-all-help = Diz quantas ocorrências há e pergunta uma vez. Um único desfazer reverte todas.
gui-find-hint = Enter em Localizar acha a próxima ocorrência; Enter em Substituir por a substitui. Seta para cima traz textos anteriores. Escape fecha.
gui-find-empty = Nada a localizar: digite o texto em Localizar.
gui-button-open = Abrir…
gui-button-font = Fonte…
gui-button-edit = Começar a editar
gui-button-finish-editing = Concluir edição
gui-button-settings = Configurações…
gui-button-commands = Comandos…
gui-button-play = Reproduzir
gui-button-pause = Pausar
gui-button-stop = Parar
gui-button-previous-sentence = Frase anterior
gui-button-next-sentence = Próxima frase
gui-button-slower = Mais devagar
gui-button-faster = Mais rápido
gui-hint-open = Escolher um documento para ler.
gui-hint-font = Escolher a fonte do texto.
gui-hint-edit = Alternar entre ler e editar.
gui-hint-settings = Cada opção, com sua ajuda.
gui-hint-commands = Executar um comando pelo nome.
gui-hint-play = Ler da palavra atual, ou pausar.
gui-button-close = Fechar
gui-toolbar-reading = Leitura
gui-document = Documento
gui-document-titled = { $title }, documento
gui-list-hint = Enter escolhe, Escape fecha.
gui-sidebar-contents = Sumário
gui-sidebar-notes = Notas
gui-sidebar-open =
    { $n ->
        [one] { $panel } aberto, 1 item.
       *[other] { $panel } aberto, { $n } itens.
    }
gui-sidebar-closed = { $panel } fechado.
gui-header-shown = Cabeçalho mostrado.
gui-header-hidden = Cabeçalho oculto. Seus comandos mantêm suas teclas.
gui-toolbar-shown = Barra de ferramentas mostrada.
gui-toolbar-hidden = Barra de ferramentas oculta. Seus comandos mantêm suas teclas.
gui-sidebar-no-headings = Nenhum título.
gui-sidebar-no-notes = Nenhuma nota.
gui-sidebar-current = { $item }, atual
gui-sidebar-hint = Enter vai até lá. { $leave } vai e volta. Escape volta.
gui-sidebar-hint-notes = Enter vai até lá. Espaço mostra suas ligações. { $leave } vai e volta. Escape volta.
gui-sidebar-notes-keys = Espaço mostra as ligações de uma nota.
gui-settings-sections = Seções
gui-settings-form = Configurações: { $section }
gui-settings-saved-hint = As alterações entram em vigor e são salvas na hora.
gui-settings-close-help = Fechar as configurações. Todas as alterações já estão salvas.
gui-settings-recent = Alterados recentemente
gui-settings-matching = Correspondentes a { $filter }
gui-settings-table = { $label } é uma tabela. Edite-a em settings.toml.
gui-setting-new-value = Novo valor para { $label }
gui-setting-value-hint = Pressione Enter para aceitar, ou Escape para voltar.
gui-prompt-path-hint = Digite o caminho de um documento e pressione Enter. Tab o completa; Seta para cima e para baixo recuperam os anteriores.
gui-prompt-hint = Pressione Enter para aceitar, ou Escape para cancelar. Seta para cima e para baixo recuperam respostas anteriores.
gui-palette-filter = Digite para filtrar os comandos
gui-palette-list = Comandos
gui-palette-hint = Enter executa a primeira correspondência; Tab vai para a lista; F1 explica um comando.
gui-open-failed = Não foi possível abrir { $name }: { $error }
gui-uia-unavailable = As notificações do UI Automation só existem no Windows; usando a região dinâmica.
gui-graphics-failed = A janela não conseguiu iniciar os gráficos. O leitor de terminal, textweaver, não precisa deles.
gui-crashed = O textweaver parou após um erro interno.
gui-crashed-saved = Sua posição foi salva, e as edições não salvas serão oferecidas para recuperação na próxima vez.
gui-rsvp = RSVP
gui-rsvp-playing = RSVP em andamento, palavra { $n } de { $total }
gui-rsvp-paused = RSVP em pausa, palavra { $n } de { $total }
gui-rsvp-finished = RSVP concluído, palavra { $n } de { $total }
gui-settings-section-item =
    { $section }, { $n ->
        [one] 1 configuração
       *[other] { $n } configurações
    }
gui-palette-count =
    { $n ->
        [0] Nenhum comando corresponde.
        [one] 1 comando.
       *[other] { $n } comandos.
    }
gui-settings-form-help = Seta para cima e para baixo passam de uma configuração a outra. Seta para a esquerda e para a direita alteram uma. Enter digita um novo valor. Delete restaura o padrão. { $next } e { $previous } mudam de seção. Digite para filtrar. F1 diz a ajuda.
gui-settings-press-enter = Pressione Enter para digitar um novo valor para { $label }.
gui-font-built-in = { $family } (incluída)
## Summaries and difficult-word definitions.

action-summarize = Resumir a seleção, o capítulo ou o documento: as frases mais centrais numa lista; Enter vai para uma
# The summary list's title: $n sentences of the whole document.
summary-title =
    { $n ->
        [one] Resumo, { $n } frase
       *[other] Resumo, { $n } frases
    }
# The summary of the chapter at the cursor.
summary-title-chapter =
    { $n ->
        [one] Resumo do capítulo, { $n } frase
       *[other] Resumo do capítulo, { $n } frases
    }
# The summary of the selection.
summary-title-selection =
    { $n ->
        [one] Resumo da seleção, { $n } frase
       *[other] Resumo da seleção, { $n } frases
    }
# Said when the summary list opens; $title is one of the titles above.
summary-intro = { $title }. Enter vai para a frase e a diz.
# The same, when a long text was read in samples.
summary-intro-sampled = { $title }, a partir de amostras deste texto longo. Enter vai para a frase e a diz.
summary-none = Nada para resumir: nenhuma frase de quatro palavras ou mais.
# tw summarize, on standard error, when a long text was read in samples: $read of $total characters.
summary-sampled-cli = Um texto longo: o resumo vem de { $read } dos seus { $total } caracteres, lidos em amostras.
# After a difficult word at high verbosity, with definitions on: its first definition.
aids-difficult-word-defined = palavra difícil: { $definition }
setting-summary-sentences = Frases do resumo
setting-summary-sentences-help = Quantas frases Resumir e tw summarize dão, de 1 a 50.
setting-reading-aids-difficult-definitions = Definições de palavras difíceis
setting-reading-aids-difficult-definitions-help = Com as palavras difíceis marcadas, com verbosidade alta dizer também a primeira definição do dicionário de uma palavra difícil.
section-summary = Resumos
settings-unit-sentences =
    { $n ->
        [one] frase
       *[other] frases
    }

# The GUI. Said in textweaver's own voice when the window takes the
# focus; $title is the document's title.
gui-window-focused = { $title }, { -brand }.

## Opening the new formats. Said after "Could not open NAME:", so
## each starts in lower case.
opening-damaged-json = não é um arquivo JSON legível; pode ser grande demais.
opening-damaged-notebook = não é um notebook Jupyter legível; pode estar danificado ou ser grande demais.
opening-damaged-svg = não é um desenho SVG legível; pode estar danificado ou ser grande demais.
opening-damaged-mathml = não é uma fórmula MathML legível; pode estar danificada ou ser grande demais.

## Menus, the command palette, interface announcements, colors, and settings

## Menu titles; the top menus mark their access key with &.

menu-file = &Arquivo
menu-edit = &Editar
menu-view = E&xibir
menu-reading = &Leitura
menu-speech = &Fala
menu-tools = Ferramen&tas
menu-help = Aj&uda
menu-recent = Documentos recentes
menu-export-as = E&xportar como
menu-preview = Visualização
menu-settings = Configurações
menu-find = Localizar
menu-format = Formatar
menu-insert = Inserir
menu-proofing = Revisão
menu-citations = Citações
menu-text-size = Tamanho do texto
menu-reading-aids = Recursos de leitura
menu-rsvp = RSVP
menu-say = Dizer
menu-move-by = Mover por
menu-headings = Títulos
menu-go-to = Ir para
menu-cursor = Cursor e seleção
menu-bookmarks = Marcadores e notas
menu-highlights = Realces
menu-tables = Tabelas
menu-speech-cursor = Cursor de fala

## Command names, in the menus and the command palette.

name-play-pause = Reproduzir ou pausar
name-stop = Parar
name-read-from-cursor = Ler a partir do cursor
name-read-document = Ler o documento inteiro
name-read-current-character = Dizer caractere
name-read-current-word = Dizer palavra
name-read-current-sentence = Dizer frase
name-read-current-line = Dizer linha
name-read-paragraph = Dizer parágrafo
name-read-selection = Ler seleção
name-say-position = Dizer posição
name-say-status = Dizer estado
name-repeat-message = Repetir última mensagem
name-word-count = Contagem de palavras
name-link-address = Endereço do link
name-replay-sentence = Repetir frase
name-replay-paragraph = Repetir parágrafo
name-repeat-sentence-slower = Repetir mais devagar
name-rsvp-toggle = RSVP
name-rsvp-play-pause = Iniciar ou pausar RSVP
name-rsvp-faster = RSVP mais rápido
name-rsvp-slower = RSVP mais lento
name-rsvp-position-next = Mover a palavra RSVP
name-reading-level = Nível de leitura
name-document-overview = Visão geral do documento
name-reading-pass = Passagem de leitura
name-define-word = Definir palavra
name-summarize = Resumir
name-toggle-citations = Ler citações
name-explore-math = Explorar matemática
name-listen-rendered = Ouvir como renderizado
name-next-sentence = Próxima frase
name-previous-sentence = Frase anterior
name-next-paragraph = Próximo parágrafo
name-previous-paragraph = Parágrafo anterior
name-next-heading = Ler do próximo título
name-previous-heading = Ler do título anterior
name-skip-next-heading = Próximo título
name-skip-previous-heading = Título anterior
name-outline = Estrutura
name-next-heading-level-1 = Próximo título, nível 1
name-next-heading-level-2 = Próximo título, nível 2
name-next-heading-level-3 = Próximo título, nível 3
name-next-heading-level-4 = Próximo título, nível 4
name-next-heading-level-5 = Próximo título, nível 5
name-next-heading-level-6 = Próximo título, nível 6
name-previous-heading-level-1 = Título anterior, nível 1
name-previous-heading-level-2 = Título anterior, nível 2
name-previous-heading-level-3 = Título anterior, nível 3
name-previous-heading-level-4 = Título anterior, nível 4
name-previous-heading-level-5 = Título anterior, nível 5
name-previous-heading-level-6 = Título anterior, nível 6
name-next-table = Próxima tabela
name-previous-table = Tabela anterior
name-next-list = Próxima lista
name-previous-list = Lista anterior
name-next-list-item = Próximo item de lista
name-previous-list-item = Item de lista anterior
name-next-link = Próximo link
name-previous-link = Link anterior
name-next-block-quote = Próxima citação
name-previous-block-quote = Citação anterior
name-next-separator = Próximo separador
name-previous-separator = Separador anterior
name-next-graphic = Próxima imagem
name-previous-graphic = Imagem anterior
name-follow-link = Seguir link
name-table-next-row = Próxima linha da tabela
name-table-previous-row = Linha anterior da tabela
name-table-next-column = Próxima coluna da tabela
name-table-previous-column = Coluna anterior da tabela
name-next-chapter = Próximo capítulo
name-previous-chapter = Capítulo anterior
name-history-back = Voltar
name-history-forward = Avançar
name-go-to = Ir para
name-document-start = Início do documento
name-document-end = Fim do documento
name-caret-next-word = Próxima palavra
name-caret-previous-word = Palavra anterior
name-caret-next-line = Próxima linha
name-caret-previous-line = Linha anterior
name-select-next-word = Selecionar próxima palavra
name-select-previous-word = Selecionar palavra anterior
name-select-next-line = Selecionar próxima linha
name-select-previous-line = Selecionar linha anterior
name-page-down = Página abaixo
name-page-up = Página acima
name-scroll-down = Rolar para baixo
name-scroll-up = Rolar para cima
name-speech-cursor-toggle = Cursor de fala
name-speech-cursor-next-line = Cursor de fala, próxima linha
name-speech-cursor-previous-line = Cursor de fala, linha anterior
name-speech-cursor-reread-line = Cursor de fala, reler linha
name-speech-cursor-exit-and-read = Cursor de fala, continuar lendo
name-rate-up = Mais rápido
name-rate-down = Mais lento
name-pitch-up = Tom mais agudo
name-pitch-down = Tom mais grave
name-volume-up = Mais alto
name-volume-down = Mais baixo
name-cycle-speed-preset = Velocidade predefinida
name-choose-voice = Vozes
name-restart-speech = Reiniciar a fala
name-cycle-verbosity = Detalhamento
name-cycle-punctuation = Pontuação
name-find = Localizar
name-find-next = Localizar próxima
name-find-previous = Localizar anterior
name-search-options = Opções de busca
name-next-misspelling = Próximo erro de ortografia
name-previous-misspelling = Erro de ortografia anterior
name-spelling-suggestions = Sugestões de ortografia
name-next-grammar-problem = Próximo problema gramatical
name-previous-grammar-problem = Problema gramatical anterior
name-next-lint-problem = Próximo problema de formato
name-previous-lint-problem = Problema de formato anterior
name-add-bookmark = Adicionar marcador
name-list-bookmarks = Marcadores
name-next-bookmark = Próximo marcador
name-previous-bookmark = Marcador anterior
name-add-note = Adicionar nota
name-list-notes = Notas
name-next-note = Próxima nota
name-previous-note = Nota anterior
name-delete-note = Excluir nota ou destaque
name-highlight-selection = Destacar
name-export-study-sheet = Exportar folha de estudo
name-highlight-as = Realçar com um nome
name-highlight-name-1 = Realçar com o nome 1
name-highlight-name-2 = Realçar com o nome 2
name-highlight-name-3 = Realçar com o nome 3
name-highlight-name-4 = Realçar com o nome 4
name-highlight-name-5 = Realçar com o nome 5
name-collect-highlights = Reunir realces
name-export-study-sheet-by-name = Exportar folha de estudo por nome
name-self-test = Autoteste
name-make-cards = Criar cartões
name-study-cards = Estudar cartões
name-list-cards = Cartões
name-grade-again = Avaliar de novo
name-grade-hard = Avaliar difícil
name-grade-good = Avaliar bom
name-grade-easy = Avaliar fácil
name-open = Abrir
name-open-path = Abrir pelo caminho
name-open-library = Biblioteca
name-new-document = Novo documento
name-save = Salvar
name-save-as = Salvar como
name-export-settings = Exportar configurações
name-import-settings = Importar configurações
name-reading-statistics = Estatísticas de leitura
name-new-from-template = Novo a partir de modelo
name-export-html = Exportar HTML
name-export-pdf = Exportar PDF
name-export-docx = Exportar Word
name-export-epub = Exportar EPUB
name-export-brf = Exportar braille
name-export-knowledge-graph = Exportar grafo de conhecimento
name-preview-in-browser = Visualizar no navegador
name-toggle-preview-auto-reload = Recarregar a prévia sozinha
name-toggle-preview-live = Prévia ao vivo
name-browse-files = Navegar pelos arquivos
name-batch-convert = Converter em lote
name-export-audio = Exportar áudio
name-quit = Sair
name-toggle-edit-mode = Modo de edição
name-undo = Desfazer
name-redo = Refazer
name-bold = Negrito
name-italic = Itálico
name-underline = Sublinhado
name-strikethrough = Tachado
name-inline-code = Código em linha
name-code-block = Bloco de código
name-insert-link = Inserir link
name-heading = Título
name-bullet-list = Lista com marcadores
name-numbered-list = Lista numerada
name-block-quote = Citação em bloco
name-horizontal-rule = Linha horizontal
name-insert-table = Inserir tabela
name-add-table-row = Adicionar linha à tabela
name-insert-image = Inserir imagem
name-replace = Substituir
name-copy = Copiar
name-cut = Recortar
name-next-table-cell = Próxima célula
name-previous-table-cell = Célula anterior
name-cycle-typing-echo = Eco da digitação
name-select-all = Selecionar tudo
name-delete-word-before = Excluir palavra anterior
name-delete-word-after = Excluir próxima palavra
name-paste = Colar
name-paste-plain-text = Colar como texto simples
name-context-menu = Menu de contexto
name-insert-citation = Inserir citação
name-add-reference = Adicionar referência
name-insert-bibliography = Inserir bibliografia
name-check-citations = Verificar citações
name-import-references = Importar referências
name-dictate = Ditar
name-next-theme = Próximo tema
name-toggle-line-numbers = Números de linha
name-toggle-character-keys = Atalhos de uma tecla
name-cycle-access-mode = Modo de acessibilidade
name-settings-profiles = Perfis
name-bionic-toggle = Leitura biônica
name-ruler-cycle = Régua de leitura
name-syllables-toggle = Sílabas
name-difficult-words-toggle = Palavras difíceis
name-text-larger = Texto maior
name-text-smaller = Texto menor
name-text-size-reset = Tamanho de texto padrão
name-choose-font = Fonte
name-contents-panel = Painel Sumário
name-notes-panel = Painel Notas
name-toggle-header = Cabeçalho
name-toggle-toolbar = Barra de ferramentas
name-next-region = Próxima região
name-previous-region = Região anterior
name-color-settings = Cores
name-cycle-interface-announcements = Avisos da interface
name-menu = Menus
name-command-palette = Paleta de comandos
name-settings = Configurações
name-keyboard-help = Atalhos de teclado
name-what-does-this-key-do = O que esta tecla faz
name-about = Sobre o textweaver
name-help = Ajuda

## Menus, the palette, and interface announcements.

menu-bar = Menus
menu-title = Menu { $name }
menu-submenu = { $name }, submenu
menu-checked = { $name }, marcado
menu-not-checked = { $name }, desmarcado
menu-value = { $name }: { $value }
menu-with-keys = { $item }, { $keys }
menu-recent-document = { $name }, { $pct } por cento
menu-recent-none = Nenhum documento recente
menu-not-available = { $name } não está disponível nesta versão.
menu-no-access-key = Nenhum item com a tecla { $letter }.
menu-closed = Menus fechados.
menu-context = Menu de contexto
menu-context-closed = Menu de contexto fechado.
menu-press-a-key = Pressione uma tecla para ouvir o que ela faz.
menu-key-described = { $name }: { $help }. Teclas: { $keys }. Nos menus: { $path }.
menu-key-described-no-menu = { $name }: { $help }. Teclas: { $keys }.
menu-keys = { $item }. Enter ou Direita abre um menu ou executa um comando, uma letra vai ao seu item, Esquerda ou Backspace volta, Esc fecha.
edit-line-continues = a linha continua
announce-level-changed = Avisos da interface: { $level }.
announce-level-off = desativados
announce-level-minimal = mínimos
announce-level-normal = normais
announce-level-full = completos
setting-accessibility-interface-announcements = Avisos da interface
setting-accessibility-interface-announcements-help = Quanto o textweaver diz sobre si mesmo: diálogos, progresso, dicas e confirmações de rotina. Erros e respostas ao que você pediu são sempre ditos. Automático é mínimo com um leitor de tela e normal quando o textweaver fala sozinho.
choice-accessibility-interface-announcements-auto = automático
choice-accessibility-interface-announcements-off = desativados
choice-accessibility-interface-announcements-minimal = mínimos
choice-accessibility-interface-announcements-normal = normais
choice-accessibility-interface-announcements-full = completos
palette-item = { $name }, { $keys }
palette-item-no-keys = { $name }
palette-item-recent = { $name }, { $keys }, recente
palette-item-recent-no-keys = { $name }, recente
palette-list-title = Comandos que correspondem a { $query }
palette-list-title-all = Comandos
palette-list-intro =
    { $title }, { $n ->
        [one] 1 comando
       *[other] { $n } comandos
    }. Enter executa um, F1 o explica.
action-browse-files = Navegar por arquivos e pacotes: Enter abre uma pasta, um pacote ou um documento; Backspace sobe um nível
action-batch-convert = Converter uma pasta de documentos para outro formato, em segundo plano
action-export-audio = Exportar o documento como áudio falado: MP3, FLAC, Opus, WAV ou um audiolivro M4B
action-dictate = Iniciar ou parar o ditado: as palavras faladas são digitadas no cursor no modo de edição
action-color-settings = Abrir as configurações de cor: o destaque de leitura, a régua, as marcas e cada parte da tela, com o contraste
action-cycle-interface-announcements = Alternar quanto o textweaver anuncia sobre si mesmo: desativados, mínimos, normais ou completos; erros e respostas são sempre ditos
action-menu = Abrir os menus: Arquivo, Editar, Exibir, Leitura, Fala, Ferramentas e Ajuda
action-what-does-this-key-do = Pressionar uma tecla para ouvir o que ela faz e onde fica nos menus, sem executá-la
action-about = Listar os dados de que um relato de problema precisa: versão, compilação, componentes, motores de voz e pastas
setting-colors-ruler = Cor da régua de leitura
setting-colors-ruler-help = A faixa da régua de leitura e da linha atual marcada. A régua mantém o sublinhado ou o negrito. Escolha um nome ou digite um código hexadecimal. Padrão: a cor do tema.
setting-colors-difficult-words = Cor das palavras difíceis
setting-colors-difficult-words-help = O sublinhado das palavras difíceis; elas continuam sublinhadas e são citadas com detalhamento alto. Escolha um nome ou digite um código hexadecimal. Padrão: a cor do tema.
setting-colors-syllables = Cor das marcas de sílaba
setting-colors-syllables-help = Os pontos médios entre as sílabas. Escolha um nome ou digite um código hexadecimal. Padrão: a cor do tema.
setting-colors-misspellings = Cor dos erros de ortografia
setting-colors-misspellings-help = O sublinhado das palavras com erro, na janela; elas também são ditas. Escolha um nome ou digite um código hexadecimal. Padrão: a cor do tema.
setting-colors-lint = Cor das marcas de formato
setting-colors-lint-help = O sublinhado dos problemas de formato Markdown e de gramática, na janela; eles também são ditos. Escolha um nome ou digite um código hexadecimal. Padrão: a cor do tema.
setting-colors-find-match = Cor dos resultados da busca
setting-colors-find-match-help = A faixa atrás dos resultados; eles continuam sublinhados. Escolha um nome ou digite um código hexadecimal. Padrão: a cor do tema.
setting-colors-selection = Cor da seleção
setting-colors-selection-help = A faixa atrás do texto selecionado. Escolha um nome ou digite um código hexadecimal. Padrão: a cor do tema.
setting-colors-focus = Cor do foco
setting-colors-focus-help = O contorno do foco e o item focado de uma lista; eles continuam em negrito. Escolha um nome ou digite um código hexadecimal. Padrão: a cor do tema.
setting-colors-links = Cor dos links
setting-colors-links-help = A cor dos links; eles continuam sublinhados. Escolha um nome ou digite um código hexadecimal. Padrão: a cor do tema.
setting-colors-headings = Cor dos títulos
setting-colors-headings-help = A cor dos títulos; eles continuam em negrito. Escolha um nome ou digite um código hexadecimal. Padrão: a cor do tema.
setting-colors-status-bar = Cor da barra de status
setting-colors-status-bar-help = A faixa das barras de status e de título. Escolha um nome ou digite um código hexadecimal. Padrão: a cor do tema.
setting-colors-notes = Cor das notas
setting-colors-notes-help = A faixa atrás do texto com nota; ele continua em itálico e sublinhado. Escolha um nome ou digite um código hexadecimal. Padrão: a cor do tema.
setting-colors-bookmarks = Cor dos marcadores
setting-colors-bookmarks-help = A faixa atrás de uma palavra com marcador; ela continua em negrito e sublinhada. Escolha um nome ou digite um código hexadecimal. Padrão: a cor do tema.
color-name-theme = cor do tema
color-name-blue = azul
color-name-orange = laranja
color-name-navy = azul-escuro
color-name-skyblue = azul-celeste
color-name-teal = azul-petróleo
color-name-gold = dourado
color-name-yellow = amarelo
color-name-green = verde
color-name-cyan = ciano
color-name-purple = roxo
color-name-pink = rosa
color-name-brown = marrom
color-name-gray = cinza
color-name-black = preto
color-name-white = branco
section-colors = Cores
colors-contrast-good = bom
colors-contrast-fair = razoável
colors-contrast-low = baixo
colors-item = { $label }: { $value }, contraste { $ratio } para 1, { $verdict }
colors-contrast = Contraste { $ratio } para 1, { $verdict }.
colors-contrast-warning = Abaixo de 3 para 1 fica difícil de ver; escolha uma cor mais clara ou mais escura.
colors-intro = Cores, { $n } configurações. Esquerda e Direita escolhem uma cor com nome, Enter digita um nome ou um valor #rrggbb, Delete volta à cor do tema, F1 diz a ajuda.
settings-item-recent = { $item }, alterado recentemente
settings-reset = { $label } voltou ao padrão, { $value }.
settings-row-help = { $label }: { $value }. Padrão: { $default }. { $help }
number-group-separator = .
number-decimal-separator = ,
settingsio-import-question-names =
    { $n ->
        [one] Importar { $n } configuração alterada de { $name }: { $names }? y ou n
       *[other] Importar { $n } configurações alteradas de { $name }: { $names }? y ou n
    }
settingsio-and-more = { $names } e mais { $n }


## Dictation in edit mode (ADR-0042). Keep the meaning first: a
## 40-cell Braille display shows the start of the line. $words are the
## dictated words, $key the dictate key, $dir a folder, $error and $text
## are passed on as they are.
dictation-status = Ditando: { $words }
dictation-listening = Ditando. Fale e pressione { $key } para parar.
dictation-finishing = Terminando o ditado.
dictation-done = Ditado concluído.
dictation-busy = O ditado está terminando. Tente de novo em um momento.
dictation-needs-edit = O ditado escreve no modo de edição. Ativar o modo de edição e ditar? y ou n
dictation-no-model = O ditado precisa do modelo Whisper em { $dir }. Veja Dictation na documentação.
dictation-failed = O ditado falhou: { $error } Veja Dictation na documentação.
dictation-no-words = Nenhuma palavra reconhecida nessa frase.
dictation-lost = O ditado parou antes de escrever as últimas palavras.
dictation-not-typed = Palavras ditadas não escritas, o modo de edição está desligado: { $text }
setting-dictation-speak-while-recording = Falar durante o ditado
setting-dictation-speak-while-recording-help = Dizer as palavras ditadas à medida que chegam. Desligado, elas aparecem na linha de status e são ditas a cada pausa, para que o microfone não ouça a voz.
setting-dictation-model-dir = Pasta do modelo de ditado
setting-dictation-model-dir-help = O modelo Whisper para o ditado. Sem valor usa whisper/rten/base.en na pasta de dados.
section-dictation = Ditado


## The file browser. Every row and introduction starts with the name,
## then the kind, so the first cells of a 40-cell Braille line hold what
## matters. $name is a file or folder name; $n a number that chooses the
## plural and $count the same number written with its separators.
# A list item with its position after it, in the file browser.
listmodel-item-position-last = { $item }, { $k } de { $n }
browse-places-title = Locais
browse-places-intro =
    { $n ->
        [one] Locais, 1 local.
       *[other] Locais, { $n } locais.
    }
# $purpose says what the folder or file is chosen for; $intro follows.
browse-choosing = { $purpose }. { $intro }
browse-place-document = { $name }, a pasta do documento
browse-place-start = { $name }, pasta inicial
browse-place-library = { $name }, pasta da biblioteca
browse-place-disk = { $name }, disco
browse-place-removable = { $name }, unidade removível
browse-place-network = { $name }, unidade de rede
browse-place-cd = { $name }, unidade de CD ou DVD
browse-place-root = { $name }, a pasta raiz
browse-choose-here = Escolher esta pasta, { $name }
browse-row-folder = { $name }, pasta
browse-row-folder-items =
    { $name }, pasta, { $n ->
        [one] 1 item
       *[other] { $count } itens
    }
# $kind is a kind below ("Markdown"); $size a size below ("12 KB").
browse-row-file = { $name }, { $kind }, { $size }
browse-row-kind = { $name }, { $kind }
browse-row-hidden = { $row }, oculto
# $kind is zip, tar, tar.gz, gzip, or 7z.
browse-kind-archive = arquivo compactado { $kind }
browse-kind-file = arquivo
browse-kind-markdown = Markdown
browse-kind-text = texto
browse-kind-html = página web
browse-kind-epub = livro EPUB
browse-kind-docx = documento do Word
browse-kind-rtf = documento RTF
browse-kind-odt = texto OpenDocument
browse-kind-latex = LaTeX
browse-kind-eml = e-mail
browse-kind-mhtml = arquivo da web
browse-kind-pdf = PDF
browse-kind-image = imagem
browse-kind-daisy = livro DAISY
browse-kind-pptx = slides do PowerPoint
browse-kind-sheet = planilha
browse-kind-json = JSON
browse-kind-notebook = caderno do Jupyter
browse-kind-svg = desenho SVG
browse-kind-mathml = fórmula MathML
browse-kind-pandoc = documento lido com o Pandoc
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
       *[other] { $count } itens.
    }
browse-intro-empty = { $name } não tem nada a mostrar.
# $filter is what was typed.
browse-intro-filtered =
    { $name }, { $n ->
        [one] 1 item corresponde a
       *[other] { $count } itens correspondem a
    } { $filter }.
browse-intro-in-archive = { $intro } Em { $archive }.
browse-intro-hidden =
    { $intro } { $n ->
        [one] 1 arquivo oculto.
       *[other] { $count } arquivos ocultos.
    }
browse-intro-cut = { $intro } Só os primeiros { $max } são mostrados.
# $preview, $choose, $sort, and $all are keys; $item the focused row.
browse-keys = { $intro } Enter abre, Backspace sobe, digitar filtra. { $preview } mostra uma prévia, { $choose } escolhe uma pasta, { $sort } ordena, { $all } mostra todos os arquivos. { $item }
browse-sorted-name = Ordenado por nome.
browse-sorted-date = Ordenado por data, o mais recente primeiro.
browse-sorted-size = Ordenado por tamanho, o maior primeiro.
browse-showing-all = Mostrando todos os arquivos.
browse-showing-readable = Mostrando só os arquivos legíveis.
browse-closed = Navegador de arquivos fechado.
browse-read-only = O navegador de arquivos só abre e escolhe arquivos; nunca os altera.
# $key is the Choose Folder key.
browse-choose-a-folder = Escolha uma pasta: Enter abre uma, { $key } a escolhe.
browse-choose-a-file = Escolha um arquivo: Enter escolhe um.
browse-no-archive-folder = Não é possível escolher uma pasta dentro de um arquivo compactado; escolha uma pasta do disco.
browse-nothing-waiting = Nenhum comando espera uma pasta; Enter a abre.
browse-cannot-read = { $name } não é um tipo de arquivo que o textweaver consegue ler.
browse-folder-unreadable = Não foi possível abrir { $name }: { $reason }
browse-archive-too-deep = { $name } está dentro de arquivos compactados demais para ser aberto.
browse-archive-too-large = { $name } é grande demais para ser listado com segurança.
browse-archive-unreadable = { $name } não é um arquivo compactado que o textweaver consegue ler; pode estar danificado.
# A document's preview: its title, then its first sentence.
browse-preview-document = { $title }. { $sentence }
browse-preview-no-text = { $title }. Não tem texto.
browse-preview-failed = Não foi possível mostrar uma prévia de { $name }: { $reason }
# $names are the first few names inside.
browse-preview-archive =
    { $name }: { $n ->
        [one] 1 arquivo
       *[other] { $count } arquivos
    }, { $readable } legíveis. { $names }
browse-preview-archive-folder =
    { $name }, pasta do arquivo compactado, { $n ->
        [one] 1 item.
       *[other] { $count } itens.
    }
browse-preview-folder = { $path }: { $names }
browse-preview-folder-empty = { $path }: nada para ler aqui.
browse-preview-other = { $name }, { $size }; o textweaver não consegue ler este tipo de arquivo.
browse-preview-path = { $path }


## Batch conversion (File, Batch convert). Keep the meaning first.
batch-choose-source = Escolha a pasta a converter
batch-choose-output = Escolha a pasta para os arquivos convertidos
batch-format-title = Converter para
batch-format-intro = Converter { $name } para: escolha um formato, { $n } opções.
batch-where-title = Para onde vão os arquivos
batch-where-intro = Para onde devem ir os arquivos convertidos?
batch-where-converted = Numa pasta converted, { $path }
batch-where-beside = Ao lado de cada arquivo
batch-where-choose = Em outra pasta, escolhida a seguir
batch-nothing = Não há documentos a converter em { $path }.
batch-confirm =
    Converter { $n ->
        [one] 1 arquivo
       *[other] { $n } arquivos
    } para { $format } em { $path }? y ou n
batch-confirm-beside =
    Converter { $n ->
        [one] 1 arquivo
       *[other] { $n } arquivos
    } para { $format } ao lado de cada arquivo? y ou n
batch-started =
    Convertendo { $n ->
        [one] 1 arquivo
       *[other] { $n } arquivos
    } para { $format }. Escape interrompe.
batch-progress = { $percent } por cento convertido, { $done } de { $total } arquivos.
batch-busy = Já convertendo, { $done } de { $total } arquivos. Escape interrompe.
batch-stop-question = Interromper a conversão? Os arquivos já feitos são mantidos. y ou n
batch-stopping = Interrompe depois dos arquivos sendo gravados.
batch-still-converting = A conversão continua.
batch-done =
    { $converted ->
        [one] 1 arquivo convertido
       *[other] { $converted } arquivos convertidos
    } para { $format }; { $skipped } em dia; { $failed } com falha.
batch-stopped =
    Interrompido. { $converted ->
        [one] 1 arquivo convertido
       *[other] { $converted } arquivos convertidos
    }; { $left } não convertidos; { $failed } com falha.
batch-report = Relatório salvo em { $path }.
batch-report-failed = Não foi possível salvar o relatório: { $error } Os arquivos convertidos são mantidos.
batch-inaccessible =
    { $n ->
        [one] 1 arquivo tem
       *[other] { $n } arquivos têm
    } itens não acessíveis; veja o relatório.
batch-failures-title =
    { $n ->
        [one] 1 arquivo com falha
       *[other] { $n } arquivos com falha
    }
batch-failure-item = { $name }: { $reason }
batch-start-failed = Não foi possível começar a converter: { $error } Verifique a pasta e o formato e tente de novo.
batch-thread-stopped = A conversão em lote parou inesperadamente.


## Audio export (File, Export audio). Keep the meaning first: a
## 40-cell Braille display shows the start of the line. $name is a file
## name (essay.flac); $path a folder or a file's full path; $format a
## format's name (FLAC, MP3); $voice a voice's or engine's name; $wpm is
## words per minute; $length a length of time from the duration-*
## messages; $chapters and $n are numbers; $percent is a multiple of ten;
## $formats lists format names (M4B); $error is passed on as it is.
audio-format-title = Exportar áudio como
audio-format-intro = Exportar { $name } como áudio: escolha um formato, { $n } opções.
audio-no-ffmpeg =
    { $formats } { $n ->
        [one] precisa
       *[other] precisam
    } do ffmpeg, que não foi encontrado.
audio-format-flac = FLAC: sem perdas, cerca de metade do tamanho do WAV
audio-format-wav = WAV: o maior, toca em qualquer lugar
audio-format-mp3 = MP3: pequeno, toca em qualquer lugar
audio-format-opus = Opus: o menor, feito para voz
audio-format-ogg = Ogg Vorbis: pequeno e aberto, toca na maioria dos players
audio-format-m4b = Audiolivro M4B, pelo ffmpeg
audio-format-mp4 = Vídeo com legendas: MP4, precisa do ffmpeg
audio-format-html = Página de leitura: texto e áudio, um arquivo
audio-where-title = Onde fica o áudio
audio-where-intro = Onde o áudio deve ficar?
audio-where-beside = Ao lado do documento, { $path }
audio-where-choose = Em outra pasta, escolhida a seguir
audio-choose-folder = Escolha a pasta para o áudio
audio-no-engine = Nenhum motor de voz aqui grava arquivos de áudio. Instale o eSpeak NG, ou escolha outro motor no menu Fala.
audio-confirm = Exportar { $name } com { $voice } a { $wpm } palavras por minuto, em { $path }? y ou n
audio-started = Exportando { $name } como { $format }. Escape para.
audio-progress = Exportando áudio, { $percent } por cento.
audio-busy = Já exportando { $name }. Escape para.
audio-stop-question = Parar a exportação? Nenhum arquivo é mantido. y ou n
audio-stopping = Parando a exportação.
audio-still-exporting = Ainda exportando o áudio.
audio-stopped = Exportação de áudio parada; nenhum arquivo foi gravado.
audio-done =
    { $name } gravado: { $length }, { $chapters ->
        [one] 1 capítulo
       *[other] { $chapters } capítulos
    }.
audio-subtitles = Legendas em { $name }.
audio-failed = Não foi possível exportar o áudio: { $error } Tente outro formato ou outra voz.
audio-thread-stopped = A exportação de áudio parou inesperadamente.


## The window's menus and dialogs. Settings files chosen with the
## system's file chooser, the Colors dialog, and the font list. $ratio is
## a contrast ratio such as 4.8; $verdict is good, fair, or low.
gui-settings-files = Arquivos de configurações
gui-settings-export-title = Exportar configurações
gui-settings-import-title = Importar configurações
gui-chooser-no-dialog = O seletor de arquivos do sistema não abriu. Digite o caminho do arquivo.
gui-colors-value = { $value }, contraste { $ratio } para 1, { $verdict }
gui-colors-help = Esquerda e Direita escolhem uma cor com nome, azul e laranja primeiro. Enter digita um nome ou um valor #rrggbb. Delete volta à cor do tema. Cada marca mantém seu sublinhado, seu peso ou seu símbolo, qualquer que seja a cor.
gui-colors-reset-all = Redefinir todas as cores
gui-colors-reset-all-help = Voltar à cor do próprio tema em cada parte.
gui-colors-reset-done = Todas as cores voltaram a ser as do tema.
colors-reset-question = Redefinir todas as cores para as do tema? y ou n
gui-colors-closed = Cores fechadas.
gui-font-list-intro =
    { $n ->
        [one] { $title }, 1 família.
       *[other] { $title }, { $n } famílias.
    }


## PDF links. Said before the first line of the page a link inside
## a PDF goes to, when the page has no heading there (as links-heading-label
## is for a heading). $page is the page's printed number or label (12, iv).
links-page-label = Página { $page }


## Sync wave, S4: sync in the reader (ADR-0049).
sync-status-off = Sincronização: desligada
sync-status-not-set-up = Sincronização: não configurada
sync-status-starting = Sincronização: iniciando
sync-status-folder-missing = Sincronização: pasta ausente, salvando aqui
sync-status-read-only = Sincronização: formato mais novo, só leitura
sync-status-failed = Sincronização: não é possível usar a pasta
sync-status-cannot-write = Sincronização: não é possível gravar, salvando aqui
sync-status-clock-ahead = Sincronização: o relógio de { $device } está adiantado
sync-status-damaged =
    { $n ->
        [one] Sincronização: 1 arquivo danificado ignorado
       *[other] Sincronização: { $n } arquivos danificados ignorados
    }
sync-status-up-to-date = Sincronização: em dia
sync-status-this-computer = Este computador: { $name }.
sync-status-no-others = Ainda não há outros computadores.
sync-status-others = Outros computadores: { $names }.
sync-status-error = Problema: { $error } Verifique a pasta de sincronização.
sync-another-computer = outro computador
sync-untitled = um documento
sync-damaged = Sincronização: arquivo danificado de { $device } ignorado.
sync-newer-file = Sincronização: arquivo mais novo de { $device } ignorado.
sync-read-only = Sincronização: formato mais novo, só leitura.
sync-clock-ahead = Sincronização: o relógio de { $device } está { $hours } horas adiantado.
sync-fresh-id = Sincronização: configuração copiada; novo identificador.
sync-write-failed = Sincronização: não é possível gravar. { $error } Por enquanto, salvando neste computador.
sync-name-refused = Nome não permitido. Tente um como notebook.
sync-note-replaced =
    { $n ->
        [one] { $title }: uma nota foi substituída pela edição mais recente de { $device }.
       *[other] { $title }: { $n } notas foram substituídas pelas edições mais recentes de { $device }.
    }
sync-restored-notes =
    { $n ->
        [one] { $title }: uma nota apagada voltou, editada em { $device }.
       *[other] { $title }: { $n } notas apagadas voltaram, editadas em { $device }.
    }
sync-restored-bookmarks =
    { $n ->
        [one] { $title }: um marcador apagado voltou, editado em { $device }.
       *[other] { $title }: { $n } marcadores apagados voltaram, editados em { $device }.
    }
sync-restored-highlights =
    { $n ->
        [one] { $title }: um destaque apagado voltou, editado em { $device }.
       *[other] { $title }: { $n } destaques apagados voltaram, editados em { $device }.
    }
sync-arrived =
    { $n ->
        [one] { $title }: 1 alteração de { $device }.
       *[other] { $title }: { $n } alterações de { $device }.
    }
sync-resumed = { $title }: retomado em { $pct } por cento, de { $device }.
sync-place-arrived = Posição de { $device }: { $pct } por cento.
sync-place-question = { $device } em { $pct } por cento. Ir para lá? y ou n
sync-suggestion-question =
    { $n ->
        [one] Talvez seja { $title } de { $device }, com 1 nota. Usá-la? y ou n
       *[other] Talvez seja { $title } de { $device }, com { $n } notas. Usá-las? y ou n
    }
sync-went-to-place = Posição de { $device }, { $pct } por cento.
sync-kept-place = Esta posição foi mantida.
sync-suggestion-accepted = Usando as notas de { $device }.
sync-suggestion-declined = Mantidos separados.
sync-sidecar-differed =
    { $n ->
        [one] Sincronização: 1 posição da biblioteca era diferente.
       *[other] Sincronização: { $n } posições da biblioteca eram diferentes.
    }
sync-sidecar-failed = Sincronização: não é possível gravar uma posição da biblioteca. { $error } Verifique se a pasta da biblioteca pode ser gravada.
sync-no-state = A sincronização está desligada nesta sessão: nada é salvo.
sync-choose-folder = Escolha a pasta de sincronização
sync-group-places = Posições
sync-group-notes = Notas
sync-group-highlights = Destaques
sync-group-bookmarks = Marcadores
sync-group-statistics = Estatísticas
sync-group-item = { $name }: { $state }
sync-start = Começar a sincronizar
sync-groups-title = O que sincroniza
sync-groups-intro = { $title }, como { $name }. Enter liga ou desliga; Começar a sincronizar termina.
sync-started = Sincronização ligada, como { $name }. Estado: { $key }.
sync-how-to-set-up = Para configurar: Ferramentas, Sincronização, Configurar a sincronização.
sync-now-started = Sincronizando.
sync-now-done =
    { $n ->
        [one] Sincronização: em dia, 1 documento verificado.
       *[other] Sincronização: em dia, { $n } documentos verificados.
    }
sync-now-changed =
    { $n ->
        [one] Sincronização: 1 documento recebeu alterações.
       *[other] Sincronização: { $n } documentos receberam alterações.
    }
sync-no-places = Nenhum outro computador tem uma posição aqui.
sync-place-item = { $device }, { $pct } por cento
sync-places-title =
    { $n ->
        [one] 1 outra posição
       *[other] { $n } outras posições
    }
sync-no-replaced = Nenhuma nota substituída neste documento.
sync-replaced-item = { $text }, substituída por { $device }
sync-replaced-item-deleted = { $text }, apagada por { $device }
sync-replaced-title =
    { $n ->
        [one] 1 nota substituída
       *[other] { $n } notas substituídas
    }
sync-replaced-intro = { $title }. Enter restaura uma.
sync-note-restored = Nota restaurada: { $text }
sync-already-off = A sincronização já está desligada aqui.
sync-stopped = Sincronização desligada aqui. A pasta fica como está.
prompt-sync-computer-name = Nome deste computador, Enter o mantém
menu-sync = Sincronização
menu-cards = Cartões de estudo
name-sync-setup = Configurar a sincronização
name-sync-status = Estado da sincronização
name-sync-now = Sincronizar agora
name-sync-go-to-place = Ir para a posição de outro computador
name-sync-replaced-notes = Notas substituídas
name-sync-stop = Parar de sincronizar neste computador
action-sync-setup = Configurar a sincronização: escolher a pasta, dar nome a este computador e escolher o que sincroniza
action-sync-status = Dizer como está a sincronização (em dia, pasta ausente ou um problema) e nomear os outros computadores
action-sync-now = Sincronizar agora: enviar as alterações deste computador e receber as dos outros em cada documento
action-sync-go-to-place = Listar as posições dos outros computadores neste documento; Enter vai para uma
action-sync-replaced-notes = Listar as notas substituídas pela edição mais recente de outro computador; Enter restaura uma
action-sync-stop = Parar de sincronizar neste computador; a pasta de sincronização fica como está
section-sync = Sincronização
setting-sync-enabled = Sincronização
setting-sync-enabled-help = Compartilhar notas, destaques, marcadores e posições com seus outros computadores pela pasta de sincronização. Ferramentas, Sincronização, Configurar a sincronização a liga.
setting-sync-folder = Pasta de sincronização
setting-sync-folder-help = A pasta que seus computadores compartilham. Uma mantida em dia pelo Syncthing, uma pasta na nuvem ou um pen drive.
setting-sync-device-name = Nome do computador
setting-sync-device-name-help = O nome deste computador nas mensagens de sincronização, como notebook ou laboratório. Vazio usa Computer 1, Computer 2 e assim por diante.
setting-sync-places = Sincronizar posições
setting-sync-places-help = Compartilhar onde você está em cada documento.
setting-sync-notes = Sincronizar notas
setting-sync-notes-help = Compartilhar notas.
setting-sync-highlights = Sincronizar destaques
setting-sync-highlights-help = Compartilhar destaques.
setting-sync-bookmarks = Sincronizar marcadores
setting-sync-bookmarks-help = Compartilhar marcadores.
setting-sync-statistics = Sincronizar estatísticas
setting-sync-statistics-help = Compartilhar o tempo de leitura e as sessões de cada computador.
setting-sync-position-policy = Posição para retomar
setting-sync-position-policy-help = Em que posição um documento abre quando outro computador também tem uma. A mais recente, a mais adiantada ou perguntar.
choice-sync-position-policy-newest = a mais recente
choice-sync-position-policy-furthest = a mais adiantada
choice-sync-position-policy-ask = perguntar

## End of S4

## Sync wave, S5 (see en.ftl).
sync-settings-arrived =
    { $n ->
        [one] Configurações: 1 mudança de { $device }.
       *[other] Configurações: { $n } mudanças de { $device }.
    }
sync-settings-arrived-several = Configurações: { $n } mudanças de { $computers } computadores.
sync-kept-keys-mac =
    { $n ->
        [one] Teclas do Mac: 1, guardada, sem uso aqui.
       *[other] Teclas do Mac: { $n }, guardadas, sem uso aqui.
    }
sync-kept-keys-pc =
    { $n ->
        [one] Teclas do Windows e Linux: 1, guardada, sem uso aqui.
       *[other] Teclas do Windows e Linux: { $n }, guardadas, sem uso aqui.
    }
sync-group-settings = Configurações
sync-group-profiles = Perfis
sync-group-key-overrides = Teclas próprias
sync-group-words = Lista de palavras
sync-group-glossary = Glossário e pronúncias
sync-group-favorite-voices = Vozes favoritas
voices-missing-row = { $voice }, favorita, não está neste computador
voices-missing = { $voice } não está neste computador. Espaço a tira das favoritas.
gui-voices-list = Vozes
gui-voices-use = Usar voz
gui-voices-use-help = Usar a voz em foco e ouvir uma amostra, ou baixá-la depois de uma pergunta.
gui-voices-preview = Prévia
gui-voices-preview-help = Ouvir uma amostra da voz em foco sem escolhê-la.
gui-voices-favorite = Favorita
gui-voices-favorite-help = Marcar a voz em foco como favorita, ou desmarcar. As favoritas vêm primeiro.
gui-voices-remove = Remover
gui-voices-remove-help = Remover a voz Piper baixada em foco, depois de uma pergunta.
gui-voices-remove-unavailable = indisponível
gui-voices-language-help = Mostrar só as vozes do próximo idioma, depois todos os idiomas de novo.
gui-voices-engine-help = Mostrar só as vozes do próximo motor, depois todos os motores de novo.
gui-voices-fetch-help = Baixar a lista de vozes Piper, cerca de 250 KB, depois de uma pergunta.
gui-voices-close-help = Fechar o gerenciador de vozes.
gui-voices-hint = Enter usa a voz, Espaço marca uma favorita, Escape fecha.
setting-sync-settings = Sincronizar configurações
setting-sync-settings-help = Compartilhar as configurações portáteis: velocidade, pontuação, tema, ajudas de leitura e outras. A voz, o motor, o modo de acesso, o conjunto de teclas e os caminhos ficam em cada computador.
setting-sync-profiles = Sincronizar perfis
setting-sync-profiles-help = Compartilhar os perfis; o que está em uso fica em cada computador.
setting-sync-key-overrides = Sincronizar teclas próprias
setting-sync-key-overrides-help = Compartilhar keymap.toml. As teclas de um Mac são guardadas, mas não usadas no Windows nem no Linux, e o contrário.
setting-sync-words = Sincronizar lista de palavras
setting-sync-words-help = Compartilhar a lista de palavras da ortografia.
setting-sync-glossary = Sincronizar glossário
setting-sync-glossary-help = Compartilhar as entradas do glossário e as pronúncias.
setting-sync-favorite-voices = Sincronizar vozes favoritas
setting-sync-favorite-voices-help = Compartilhar as vozes favoritas. Uma que este computador não tem aparece como não está neste computador.

## End of S5

## Lexend downloaded on first choice.
font-download-question = Baixar a fonte { $font }, { $kb } KB, { $licence }? y ou n
font-downloading = Baixando { $font }.
font-downloaded = { $font } baixada e pronta.
font-download-failed = { $font } não baixada: { $error } Outra fonte é usada.
font-download-declined = Não baixada. Outra fonte é usada.
font-download-busy = { $font } ainda está baixando.
font-download-no-folder = Sem pasta de dados para { $font }.
font-download-not-in-build = Esta versão não baixa fontes.
gui-font-to-download = { $family } (baixar, { $kb } KB)
gui-font-downloaded = { $family } (baixada)

## Seletores de arquivos e pastas.
prompt-browse-hint = { $label }. { $key } para procurar.
prompt-browse-file = Escolha o arquivo: { $label }
prompt-browse-folder = Escolha a pasta: { $label }
prompt-browse-filled = { $name } escolhido. Enter confirma.
chooser-type-files = Arquivos { $type }
chooser-image-title = Inserir uma imagem
chooser-images = Imagens
chooser-references-title = Importar referências
chooser-reference-files = Arquivos de referências
chooser-profiles-import-title = Importar perfis
chooser-profiles-export-title = Exportar perfis
chooser-profile-files = Arquivos de perfis
gui-folder-no-dialog = O seletor de pastas do sistema não abriu. Escolha a pasta nesta lista.
gui-prompt-browse-hint = { $key } abre o navegador de arquivos.

## Componentes opcionais.
name-manage-components = Gerenciar componentes opcionais…
name-download-dictation-model = Baixar o modelo de ditado
action-manage-components = Gerenciar componentes opcionais: os modelos, fontes e vozes que o textweaver pode baixar, com tamanho e licença
name-forget-github-token = Esquecer o token do GitHub
action-forget-github-token = Esquecer o token do GitHub guardado para a origem de componentes; ele é pedido de novo quando necessário
action-download-dictation-model = Baixar o modelo de ditado escolhido nas configurações, depois de dizer o tamanho e a licença
component-feature-dictation = o ditado
component-feature-ocr = ler páginas digitalizadas
component-feature-reading-font = uma fonte de leitura
component-feature-voice = uma voz
component-state-installed = instalado
component-state-not-installed = não instalado
component-state-partial = instalado em parte
component-state-damaged = danificado
component-state-downloading = baixando
components-title = Componentes opcionais
components-intro =
    { $n ->
        [one] 1 componente opcional. Enter para ações.
       *[other] { $n } componentes opcionais. Enter para ações.
    }
components-item = { $title }: { $state }, { $size }, licença { $license }, para { $features }
components-actions-intro = { $title }: { $state }.
components-action-download = Baixar, { $size }
components-action-verify = Verificar os arquivos
components-action-remove = Remover
components-action-install-zip = Instalar de um arquivo zip…
components-action-install-folder = Instalar de uma pasta…
components-install-purpose = Instalar o componente daqui
component-question = Baixar { $title }, { $size }, licença { $license }? y ou n
component-remove-question = Remover { $title }? y ou n
component-downloading = Baixando. Escape interrompe.
component-installing = Instalando do arquivo.
component-verifying = Verificando os arquivos.
component-progress = { $percent } por cento baixado.
component-ready = Pronto: { $title }.
component-verified = Arquivos corretos: { $title }.
component-verify-failed =
    { $n ->
        [one] 1 arquivo não confere: { $files }.
       *[other] { $n } arquivos não conferem: { $files }.
    }
component-removed = Removido: { $title }.
component-refused =
    { $n ->
        [one] 1 arquivo deixado de fora: { $files }.
       *[other] { $n } arquivos deixados de fora: { $files }.
    }
component-already = Já instalado: { $title }.
component-not-there = Não instalado: { $title }.
component-declined = Nada foi baixado.
component-not-in-build = Sem downloads nesta versão.
component-no-folder = Sem pasta de dados para ele.
component-error-fetch = Não baixado: a origem falhou.
component-error-size = Não instalado: tamanho errado.
component-error-hash = Não instalado: um arquivo não correspondeu. Baixe de novo ou verifique a origem.
component-error-missing = Não instalado: falta um arquivo.
component-error-cancelled = Parado; continua mais tarde.
component-error-busy = Já está baixando um.
component-error-no-source = Não baixado: sem endereço.
component-error-name = Recusado: um nome não é simples.
component-error-manifest = Uma lista de componentes está ilegível.
component-error-io = Não instalado: falha ao gravar.
components-token-invalid = Não é um token. Confira e cole de novo.
components-token-kept = Token guardado no armazenamento de credenciais do sistema.
components-token-not-kept = Token usado, não guardado: { $reason }.
components-token-forgotten = Token esquecido.
components-token-none = Nenhum token estava guardado.
components-token-not-forgotten = Não esquecido: { $reason }.
components-token-gh-still = O login do GitHub CLI continua em uso; gh auth logout o encerra.
components-chooser-title = Componentes opcionais
components-chooser-intro = Extras opcionais, nenhum escolhido. Espaço escolhe; Baixar os escolhidos os obtém; Escape pula.
components-chooser-item = { $mark }: { $title }, para { $features }, { $size }, licença { $license }
components-chosen = Escolhido
components-not-chosen = Não escolhido
components-chooser-download = Baixar os escolhidos
components-chooser-skip = Agora não
components-chooser-skipped = Pulado; veja Gerenciar componentes.
components-chooser-none = Nada escolhido, nada baixado.
dictation-model-question = O ditado precisa do modelo Whisper, { $size }, licença { $license }. Baixar agora? y ou n
dictation-model-declined = Sem modelo, sem ditado por enquanto.
dictation-model-not-in-build = Sem modelo e sem downloads aqui.
dictation-model-file-missing = Falta ao modelo { $file }.
dictation-model-damaged = Modelo de ditado danificado: { $file }.
dictation-model-no-folder = Pasta não existe: { $dir }.

## Configurações dos componentes opcionais.
section-components = Componentes opcionais
setting-dictation-model = Modelo de ditado
setting-dictation-model-help = O modelo Whisper que o ditado usa quando nenhuma pasta é definida. Baixar o modelo de ditado, no menu Ferramentas, o obtém.
choice-dictation-model-whisper-base-en = base.en, o padrão
choice-dictation-model-whisper-small-en = small.en, maior e mais preciso
setting-components-source = Origem dos componentes
setting-components-source-help = Seus próprios componentes, usados primeiro. Um repositório do GitHub como dono/nome, ou uma pasta neste computador. Vazio não usa nenhuma. Nunca coloque uma senha aqui. Um repositório privado entra com o GitHub CLI, ou pede uma vez um token guardado no armazenamento de credenciais do sistema.
setting-components-mirror = Espelho de componentes
setting-components-mirror-help = De onde vêm primeiro os componentes opcionais: um endereço https ou uma pasta neste computador. Vazio usa as origens públicas. Nunca coloque uma senha aqui.

## Atualizações (B1-u1).
name-check-for-updates = Procurar atualizações
action-check-for-updates = Procurar atualizações: ver se há um textweaver mais novo no GitHub e perguntar antes de baixá-lo
components-chooser-updates = { $mark }: Procurar atualizações automaticamente, uma vez por dia, no GitHub
update-checking = Procurando atualizações.
update-none = Sem atualização: { $version } é a mais nova.
update-found = Atualização disponível: textweaver { $version }, { $size }. Baixar? y ou n
update-available-elsewhere = Atualização disponível: textweaver { $version }. Atualize esta cópia do jeito que foi instalada.
update-declined = Não baixada. Uma versão mais nova será oferecida.
update-downloading = Baixando a atualização, { $size }.
update-progress = Atualização: { $percent } por cento baixado.
update-installed = Atualização verificada e instalada. Reinicie o textweaver para usá-la.
update-on-close = Atualização verificada. Ela é instalada ao fechar o textweaver.
update-busy = Já procurando atualizações.
update-not-in-build = Sem atualizações nesta versão.
update-automatic-question = Procurar atualizações automaticamente, uma vez por dia? y ou n
update-automatic-on = Atualizações: procuradas uma vez por dia.
update-automatic-off = Atualizações: não procuradas. Ajuda, Procurar atualizações procura a qualquer hora.
update-error-check = Falha ao procurar atualizações: { $reason }
update-error-no-package = Sem pacote de atualização para este computador em { $version }.
update-error-no-checksum = Atualização recusada: sem soma de verificação. Nada mudou.
update-error-mismatch = Atualização recusada: a soma não confere. Nada mudou.
update-error-cancelled = Download parado; continua da próxima vez.
update-error-download = Atualização não baixada: { $reason }
update-error-install = Atualização não instalada: { $reason }. Nada mudou.
update-error-not-package = Não atualizado: esta cópia não vem de um pacote publicado.
section-updates = Atualizações
setting-updates-check = Procurar atualizações
setting-updates-check-help = Uma vez por dia ao iniciar, ler a lista pública de versões do textweaver no GitHub e oferecer uma mais nova, perguntando antes de baixá-la. Nada sobre você é enviado. Ajuda, Procurar atualizações procura a qualquer hora.

## Help's ways to the docs, About's facts, and first-run choices asked
## again. $address is a web address; $path a folder; facts start with
## their name so each Braille line leads with it.
name-quick-start = Início rápido
name-documentation = Documentação…
name-report-problem = Relatar um problema…
name-ask-first-run-again = Repetir perguntas da primeira execução
action-quick-start = Abrir o guia de início rápido no textweaver
action-documentation = Mostrar o endereço web da documentação e perguntar antes de abri-lo em um navegador
action-report-problem = Mostrar onde relatar um problema e perguntar antes de abri-lo em um navegador; nada é enviado
action-ask-first-run-again = Perguntar de novo as escolhas da primeira execução: o modo híbrido com leitor de tela e os componentes opcionais
about-title = Sobre o textweaver
about-intro = Sobre o textweaver, { $n } dados. Inclua-os em um relato de problema.
about-version = Versão: textweaver { $version }
about-build = Compilação: { $frontend }, { $profile }, { $os } { $arch }
about-frontend-window = janela
about-frontend-terminal = leitor de terminal
about-profile-release = versão final
about-profile-debug = depuração
about-license = Licença: { $license }
about-engine-in-use = Motor de voz em uso: { $engine }
about-engines = Motores de voz encontrados: { $engines }
about-engines-none = Motores de voz encontrados: nenhum
about-components = Componentes: { $installed } de { $known } instalados
about-folder-settings = Pasta das configurações: { $path }
about-folder-data = Pasta dos dados: { $path }
about-folder-cache = Pasta do cache: { $path }
about-no-folders = Pastas: nenhuma; esta sessão não guarda arquivos.
about-quick-start-online = Início rápido não encontrado junto ao textweaver. Abrir { $address } no navegador? y ou n
about-docs-question = Documentação: { $address }. Abrir no navegador? y ou n
about-report-question = Relatar um problema: { $address }. Nada é enviado. Abrir no navegador? y ou n
about-first-run-again = Escolhas da primeira execução redefinidas; perguntadas no próximo início.

## W9b-x: as configurações de leitura (Exibir, Configurações de leitura).
name-reading-form = Configurações de leitura
action-reading-form = Abrir as configurações de leitura: velocidade, fonte, espaçamento, comprimento da linha, tema, destaque, régua, leitura biônica e sílabas
reading-form-intro = Configurações de leitura, { $n } configurações. Esquerda e Direita mudam um valor, Enter digita um, Delete volta ao padrão, F1 diz a ajuda.
reading-form-spacing-wcag-done = Espaçamento nos valores da WCAG.
reading-form-spacing-generous-done = Espaçamento amplo, maior que a WCAG.
gui-reading-form-help = Cima e Baixo movem, Esquerda e Direita mudam um valor, Enter digita um, F1 diz a ajuda.
gui-reading-voices = Vozes
gui-reading-voices-help = Abrir o gerenciador de vozes.
gui-reading-wcag = Espaçamento WCAG
gui-reading-wcag-help = Pôr os quatro espaçamentos nos valores da WCAG.
gui-reading-generous = Espaçamento amplo
gui-reading-generous-help = Pôr os quatro espaçamentos maiores que a WCAG.
gui-reading-closed = Configurações de leitura fechadas.
## End of W9b-x

## B1-t1: alterações controladas e comentários (a lista de alterações).
name-list-changes = Alterações e comentários
action-list-changes = Listar as alterações controladas e os comentários: Enter vai até um, A aceita uma alteração, R a rejeita
name-accept-all-changes = Aceitar todas as alterações
action-accept-all-changes = Aceitar todas as alterações controladas do documento
name-reject-all-changes = Rejeitar todas as alterações
action-reject-all-changes = Rejeitar todas as alterações controladas do documento
name-add-comment = Adicionar comentário
action-add-comment = Adicionar um comentário à seleção ou à frase do cursor
prompt-comment-reply = Resposta
prompt-comment-text = Comentário
prompt-github-token = Token do GitHub da origem de componentes, Enter pula
changes-title = Alterações e comentários
changes-intro =
    { $n ->
        [one] { $title }, { $n } item. Enter vai até ele. A aceita uma alteração, R a rejeita, e com Shift qualquer uma faz todas as alterações do mesmo autor. Num comentário, F2 responde, Espaço resolve, Delete apaga. N adiciona um comentário.
       *[other] { $title }, { $n } itens. Enter vai até um. A aceita uma alteração, R a rejeita, e com Shift qualquer uma faz todas as alterações do mesmo autor. Num comentário, F2 responde, Espaço resolve, Delete apaga. N adiciona um comentário.
    }
changes-none = Não há alterações controladas nem comentários neste documento.
changes-row = { $kind }: “{ $text }”, { $who }, { $when }
changes-kind-inserted = Inserido
changes-kind-deleted = Excluído
changes-kind-moved-away = Movido daqui
changes-kind-moved-here = Movido para cá
changes-by = por { $author }
changes-no-author = autor não registrado
changes-no-date = data não registrada
changes-date = { $weekday }, { $day } de { $month } de { $year }
changes-weekday-0 = domingo
changes-weekday-1 = segunda-feira
changes-weekday-2 = terça-feira
changes-weekday-3 = quarta-feira
changes-weekday-4 = quinta-feira
changes-weekday-5 = sexta-feira
changes-weekday-6 = sábado
changes-month-1 = janeiro
changes-month-2 = fevereiro
changes-month-3 = março
changes-month-4 = abril
changes-month-5 = maio
changes-month-6 = junho
changes-month-7 = julho
changes-month-8 = agosto
changes-month-9 = setembro
changes-month-10 = outubro
changes-month-11 = novembro
changes-month-12 = dezembro
changes-comment = Comentário: { $text }
changes-comment-by = Comentário de { $author }: { $text }
changes-replies =
    { $n ->
        [one] { $n } resposta
       *[other] { $n } respostas
    }
changes-resolved = resolvido
changes-accepted = Aceito. { $kind }: “{ $text }”.
changes-rejected = Rejeitado. { $kind }: “{ $text }”.
changes-accepted-all =
    { $n ->
        [one] { $n } alteração aceita.
       *[other] { $n } alterações aceitas.
    }
changes-rejected-all =
    { $n ->
        [one] { $n } alteração rejeitada.
       *[other] { $n } alterações rejeitadas.
    }
changes-accepted-author =
    { $n ->
        [one] { $n } alteração de { $author } aceita.
       *[other] { $n } alterações de { $author } aceitas.
    }
changes-rejected-author =
    { $n ->
        [one] { $n } alteração de { $author } rejeitada.
       *[other] { $n } alterações de { $author } rejeitadas.
    }
changes-none-left = Não há alterações controladas para aceitar ou rejeitar.
changes-not-a-change = Esta linha é um comentário. F2 responde, Espaço resolve, Delete apaga.
changes-not-a-comment = Esta linha é uma alteração. A a aceita, R a rejeita.
changes-edit-mode = No modo de edição as alterações ficam como estão. Saia do modo de edição para aceitá-las ou rejeitá-las.
changes-replied = Resposta adicionada.
changes-resolved-done = Comentário resolvido.
changes-reopened = Comentário aberto de novo.
changes-comment-deleted = Comentário e respostas apagados.
changes-comment-added = Comentário adicionado.
changes-delete-comment-question = Apagar este comentário e as respostas? y ou n
changes-written-accepted =
    { $n ->
        [one] { $path } gravado com { $n } alteração aceita.
       *[other] { $path } gravado com { $n } alterações aceitas.
    }
changes-written-rejected =
    { $n ->
        [one] { $path } gravado com { $n } alteração rejeitada.
       *[other] { $path } gravado com { $n } alterações rejeitadas.
    }
## End of B1-t1
## B1-r5: braille (BRF) files read as print. $page is a braille page
## number ("3", "p1"); $n is a number of lines; $reason is an error.
setting-braille-brf-code = Código braille dos arquivos BRF
setting-braille-brf-code-help = O código braille em que os arquivos BRF são lidos. UEB serve para livros feitos desde 2016; EBAE, English Braille American Edition, para livros mais antigos. Ler um arquivo BRF como texto impresso precisa do liblouis. Abra o arquivo de novo depois de uma mudança.
choice-braille-brf-code-ueb = UEB
choice-braille-brf-code-ebae = EBAE
name-show-original-braille = Mostrar o braille original
action-show-original-braille = Mostrar o braille original da página no cursor, num arquivo BRF lido como texto impresso
brf-original-title = Braille original, página { $page }
brf-original-intro =
    { $n ->
        [one] Braille original, página { $page }, { $n } linha. Escape fecha.
       *[other] Braille original, página { $page }, { $n } linhas. Escape fecha.
    }
brf-original-not-brf = Não é um arquivo braille. Mostrar o braille original funciona com arquivos BRF.
brf-original-unreadable = Não é possível ler o arquivo braille: { $reason }
brf-no-liblouis = Braille mostrado como braille: falta o liblouis. Para ler como texto impresso, instale o liblouis de liblouis.io ou dos pacotes do seu sistema e abra o arquivo de novo.
## End of B1-r5

## B1-t2: guardar a revisão no arquivo do Word.
name-save-changes-to-word = Guardar alterações no arquivo Word
action-save-changes-to-word = Guardar as alterações aceitas e rejeitadas e os comentários no arquivo do Word, mantendo antes uma cópia do original
changes-accept-all-question =
    { $n ->
        [one] Aceitar a alteração? y ou n
       *[other] Aceitar as { $n } alterações? y ou n
    }
changes-reject-all-question =
    { $n ->
        [one] Rejeitar a alteração? y ou n
       *[other] Rejeitar as { $n } alterações? y ou n
    }
changes-save-not-word = Não é um arquivo do Word. As alterações só voltam para arquivos .docx; a exportação escreve o texto decidido em outros formatos.
changes-save-nothing = Nada a guardar: nenhuma alteração foi aceita ou rejeitada, e nenhum comentário mudou.
changes-saved-backup = Alterações guardadas em { $file }. O original fica como { $backup }.
changes-saved = Alterações guardadas em { $file }.
changes-save-unplaced =
    { $n ->
        [one] 1 comentário novo está no início: o texto dele não foi encontrado.
       *[other] { $n } comentários novos estão no início: o texto deles não foi encontrado.
    }
changes-save-failed = Não foi possível guardar as alterações em { $file }: { $error }. O arquivo continua como estava; feche-o no Word se estiver aberto e tente de novo.
changes-save-not-in-build = Guardar as alterações no arquivo do Word não está nesta versão do { -brand }. tw changes --in-place ainda faz isso.
changes-in-place-accepted =
    { $n ->
        [one] 1 alteração aceita em { $path }. O original fica como { $backup }.
       *[other] { $n } alterações aceitas em { $path }. O original fica como { $backup }.
    }
changes-in-place-rejected =
    { $n ->
        [one] 1 alteração rejeitada em { $path }. O original fica como { $backup }.
       *[other] { $n } alterações rejeitadas em { $path }. O original fica como { $backup }.
    }
## B1-t3: edits saved into a Word file as tracked changes.
changes-tracked-saved =
    { $n ->
        [one] 1 alteração controlada guardada em { $file }.
       *[other] { $n } alterações controladas guardadas em { $file }.
    }
changes-original-kept = O original fica como { $backup }.
changes-tracked-refused =
    { $n ->
        [one] Não guardado: 1 alteração em { $file } atravessa um parágrafo ou está num link ou campo, e não pode ser controlada. Guardar como mantém as suas edições em Markdown.
       *[other] Não guardado: { $n } alterações em { $file } atravessam um parágrafo ou estão num link ou campo, e não podem ser controladas. Guardar como mantém as suas edições em Markdown.
    }
## End of B1-t2
