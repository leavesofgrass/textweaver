### Reading: navigation, reading aloud, status, statistics, the queue, and reading aids.

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

## A fila de leitura e a lista de revisão (beta 3).

queue-title = Fila de leitura
queue-intro =
    { $n ->
        [one] Fila de leitura, 1 documento. Enter abre-o, Delete tira-o da fila.
       *[other] Fila de leitura, { $n } documentos. Enter abre um, Delete tira-o da fila.
    }
queue-empty = A fila de leitura está vazia. Na biblioteca, { $key } adiciona um documento.
queue-item-missing = { $title }, não encontrado
queue-item-elsewhere = { $title }, não está neste computador
queue-untitled = Um documento de outro computador
queue-elsewhere-chosen = { $title } não está neste computador. Continua na fila de leitura, e será encontrado assim que você o abrir aqui.
queue-added =
    { $n ->
        [one] Adicionado à fila de leitura: { $title }. 1 documento na fila.
       *[other] Adicionado à fila de leitura: { $title }. { $n } documentos na fila.
    }
queue-removed = Retirado da fila de leitura: { $title }.
queue-no-file = Só um documento guardado como arquivo pode entrar na fila de leitura.
queue-read-failed = Não foi possível ler a biblioteca para a fila de leitura: { $error }. Verifique library.json na pasta de dados; tw settings path indica-a.
queue-next-ask = A seguir: { $title }. { $key } para ler.
queue-next-reading = A seguir: { $title }.
review-title = Lista de revisão
review-intro =
    { $n ->
        [one] Lista de revisão, 1 documento, o lido há mais tempo primeiro. Enter abre-o.
       *[other] Lista de revisão, { $n } documentos, o lido há mais tempo primeiro. Enter abre um.
    }
review-empty = Nada para rever ainda. Os documentos lidos em voz alta aparecem aqui.
review-stats-off = A lista de revisão precisa das estatísticas de leitura, que estão desligadas. Ligue Estatísticas de leitura nas configurações.
review-item = { $title }, { $days }, { $pct } por cento, { $notes }
review-days =
    { $n ->
        [0] hoje
        [one] 1 dia
       *[other] { $n } dias
    }
review-notes =
    { $n ->
        [0] sem notas
        [one] 1 nota
       *[other] { $n } notas
    }
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

## Reading aloud.

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
status-cw-rate = CW { $wpm } ppm
status-cw-rate-spoken = CW a { $wpm } palavras por minuto
status-cw-rate-farnsworth = CW { $wpm } ppm, efetiva { $effective }
status-cw-rate-farnsworth-spoken = CW a { $wpm } palavras por minuto, efetiva { $effective }
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

## Summaries and difficult-word definitions.
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
settings-unit-sentences =
    { $n ->
        [one] frase
       *[other] frases
    }

# The GUI. Said in textweaver's own voice when the window takes the
# focus; $title is the document's title.
gui-window-focused = { $title }, { -brand }.
gui-updated-to = Atualizado para { $version }. Ajuda, Documentação descreve as novidades.
