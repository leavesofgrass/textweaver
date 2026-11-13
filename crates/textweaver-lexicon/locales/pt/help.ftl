### The help, key names, About, and the packaged guides.

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
help-intro = Ajuda. Digite para pesquisar em toda a ajuda; seta para cima e para baixo move, Escape fecha.

## The help list. Each value is a key or keys from the keymap.

help-about = O textweaver lê documentos em voz alta. As teclas abaixo são as atribuições atuais.
help-search = Pesquise em toda a ajuda digitando aqui: comandos, teclas, configurações e os guias.
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

## Help's ways to the docs, About's facts, and first-run choices asked
## again. $address is a web address; $path a folder; facts start with
## their name so each Braille line leads with it.
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

## B1-hp: os guias incluídos e Pesquisar na ajuda.
docs-not-found = Documentação não encontrada junto ao textweaver. Abrir { $address } no navegador? y ou n
docs-read-only = Este guia vem com o textweaver e é somente leitura. Para alterá-lo, copie o arquivo para outro lugar e abra a cópia.
docs-guide-missing =
    { $n ->
        [one] Guia ausente: { $guides }. Os outros guias abrem.
       *[other] { $n } guias ausentes: { $guides }. Os outros guias abrem.
    }
helpsearch-title = Pesquisar na ajuda
helpsearch-title-matching = Ajuda sobre { $filter }
helpsearch-intro = Pesquisar na ajuda, { $n } tópicos. Digite para pesquisar comandos, teclas, configurações e os guias; Enter abre um tópico, F1 o explica.
helpsearch-match =
    { $n ->
        [one] 1 resultado.
       *[other] { $n } resultados.
    }
helpsearch-none = Nenhuma ajuda sobre { $query }. Backspace apaga letras.
helpsearch-cleared =
    { $n ->
        [one] Filtro limpo, 1 tópico.
       *[other] Filtro limpo, { $n } tópicos.
    }
helpsearch-command = { $row }, comando
helpsearch-setting = { $label }, configuração em { $section }
helpsearch-guide = { $heading }, em { $guide }
helpsearch-at-heading = Em { $heading }.
## End of B1-hp
