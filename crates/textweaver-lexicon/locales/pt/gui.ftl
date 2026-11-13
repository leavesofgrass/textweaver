### The textweaver app: drawn labels, hints, buttons, and dialogs.

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
gui-button-edit = Editar
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
gui-button-start-dictation = Iniciar ditado
gui-button-pause-dictation = Pausar ditado
gui-button-resume-dictation = Retomar ditado
gui-button-stop-dictation = Parar ditado
gui-button-live-pause = Pausar atualizações
gui-button-live-resume = Retomar atualizações
gui-button-live-newest = Ir para a mais recente
gui-button-live-table = Vista de tabela
gui-button-live-list = Vista de lista
gui-button-live-save = Salvar registro
gui-button-live-close = Fechar fonte
gui-button-live-show = Mostrar mudanças
gui-button-live-show-on = Mostrar mudanças: ligado
gui-button-live-show-off = Mostrar mudanças: desligado
gui-button-live-speak = Falar mudanças
gui-button-live-speak-on = Falar mudanças: ligado
gui-button-live-speak-off = Falar mudanças: desligado
gui-button-unavailable = indisponível
gui-hint-open = Escolher um documento para ler.
gui-hint-font = Escolher a fonte do texto.
gui-hint-edit = Alternar entre ler e editar.
gui-hint-settings = Cada opção, com sua ajuda.
gui-hint-commands = Executar um comando pelo nome.
gui-hint-play = Ler da palavra atual, ou pausar.
gui-hint-live-pause = Novas entradas esperam na pausa.
gui-hint-live-view = Mantém o seu lugar no registro.
gui-hint-live-save = Salvar o registro como JSON agora.
gui-hint-live-close = Para de receber; o registro fica.
gui-hint-live-field = Para o campo da linha do cursor.
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
gui-focus-mode-on = Modo foco ativado. Mover o mouse mostra as barras.
gui-focus-mode-off = Modo foco desativado.
gui-full-screen-on = Tela cheia ativada.
gui-full-screen-off = Tela cheia desativada.

## Personalizar botões (crate::buttons).

buttons-title = Personalizar botões
buttons-intro =
    { $n ->
        [one] Personalizar botões, 1 botão. Enter num botão para movê-lo ou removê-lo.
       *[other] Personalizar botões, { $n } botões. Enter num botão para movê-lo ou removê-lo.
    }
buttons-row =
    { $bar ->
        [header] { $name }, cabeçalho { $pos } de { $count }
       *[toolbar] { $name }, barra de ferramentas { $pos } de { $count }
    }
buttons-add-header = Adicionar um botão ao cabeçalho
buttons-add-toolbar = Adicionar um botão à barra de ferramentas
buttons-reset = Restaurar o padrão das duas barras
buttons-move-up = Mover para cima
buttons-move-down = Mover para baixo
buttons-remove = Remover
buttons-add-intro =
    { $n ->
        [one] 1 comando para adicionar. Enter o adiciona no fim.
       *[other] { $n } comandos para adicionar. Enter adiciona um no fim.
    }
buttons-added =
    { $bar ->
        [header] { $name } adicionado, cabeçalho { $pos } de { $count }.
       *[toolbar] { $name } adicionado, barra de ferramentas { $pos } de { $count }.
    }
buttons-removed =
    { $bar ->
        [header] { $name } removido do cabeçalho.
       *[toolbar] { $name } removido da barra de ferramentas.
    }
buttons-moved = { $name } movido, { $pos } de { $count }.
buttons-first = { $name } já é o primeiro.
buttons-last = { $name } já é o último.
buttons-already =
    { $bar ->
        [header] { $name } já está no cabeçalho.
       *[toolbar] { $name } já está na barra de ferramentas.
    }
buttons-no-name = Esse comando não tem nome curto, então não pode ser um botão.
buttons-no-button = Não há botão aí.
buttons-reset-done = Botões restaurados para o padrão.
buttons-none-to-add = Não há mais comandos para adicionar.
# The preview pane beside the editor (edit mode). $key is the key that moves between the window's parts (F6).
gui-preview = Pré-visualização
gui-preview-shown = Pré-visualização mostrada ao lado do editor. { $key } vai até ela.
gui-preview-hidden = Pré-visualização oculta.
gui-preview-later = Pré-visualização ligada. Aparece ao lado do editor no modo de edição.
gui-preview-empty = Nada para pré-visualizar ainda.
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
gui-crashed = O textweaver parou após um erro interno.
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

## The window's menus and dialogs. Settings files chosen with the
## system's file chooser, the Colors dialog, and the font list. $ratio is
## a contrast ratio such as 4.8; $verdict is good, fair, or low.
gui-settings-files = Arquivos de configurações
gui-settings-export-title = Exportar configurações
gui-settings-import-title = Importar configurações
gui-chooser-no-dialog = O seletor de arquivos do sistema não abriu. Digite o caminho do arquivo.
gui-colors-value = { $value }, contraste { $ratio } para 1, { $verdict }
gui-colors-help = Esquerda e Direita escolhem uma cor com nome, azul e laranja primeiro. Enter digita um nome ou um valor #rrggbb. Delete volta à cor do tema. Cada marca mantém seu sublinhado, seu peso ou seu símbolo, qualquer que seja a cor.
gui-colors-value-system = { $value }; não desenhada enquanto as cores de alto contraste do sistema estão ativas
gui-colors-help-system = As cores de alto contraste do sistema estão ativas e são desenhadas no lugar destas, que voltam a valer quando forem desativadas. Cada nome de realce mantém sua própria forma.
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

## W9b-x: as configurações de leitura (Exibir, Configurações de leitura).
reading-form-intro = Configurações de leitura, { $n } configurações. Esquerda e Direita mudam um valor, Enter digita um, Delete volta ao padrão, F1 diz a ajuda.
reading-form-spacing-wcag-done = Espaçamento nos valores da WCAG.
reading-form-spacing-generous-done = Espaçamento amplo, maior que a WCAG.
spacing-letter-without-word = Aumente o espaço entre palavras com o das letras.
gui-reading-form-help = Cima e Baixo movem, Esquerda e Direita mudam um valor, Enter digita um, F1 diz a ajuda.
gui-reading-voices = Vozes
gui-reading-voices-help = Abrir o gerenciador de vozes.
gui-reading-wcag = Espaçamento WCAG
gui-reading-wcag-help = Pôr os quatro espaçamentos nos valores da WCAG.
gui-reading-generous = Espaçamento amplo
gui-reading-generous-help = Pôr os quatro espaçamentos maiores que a WCAG.
gui-reading-closed = Configurações de leitura fechadas.
## End of W9b-x
