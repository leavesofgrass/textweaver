### Prompts, questions, lists, and file choosers.

## Prompts.

prompt-define-word = Definir qual palavra?
prompt-profile-name = Nome do novo perfil
prompt-profile-rename = Novo nome do perfil, Enter mantém
prompt-profiles-import = Importar perfis de um arquivo
prompt-profiles-export = Exportar perfis para um arquivo, por exemplo textweaver-profiles.json

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
app-window-only = Este comando funciona no aplicativo textweaver.
app-terminal-only = Este comando funciona no leitor de terminal.
settings-save-failed = Não foi possível salvar as configurações: { $error } Suas alterações continuam em uso até você sair.
settings-outside-kept = As configurações alteradas fora do textweaver foram mantidas.
edit-still-editing = Ainda editando.
# The reading view inside edit mode (toggle_preview). $key is its key;
# $line the line the caret is back on.
edit-preview-on = Pré-visualização, somente leitura: a visualização de leitura no mesmo lugar. { $key } volta à edição.
edit-preview-off = De volta à edição. { $line }
edit-preview-read-only = A pré-visualização é somente leitura. { $key } volta à edição.
edit-preview-not-editing = A pré-visualização funciona no modo de edição. { $key } começa a editar.
edit-preview-plain = Texto simples não tem formatação para pré-visualizar; ele é lido igual.
edit-preview-failed = Não foi possível mostrar a pré-visualização: { $error } Verifique o Markdown e tente de novo.
goto-not-a-target = Não é um destino válido: { $text }. Digite um número de linha, uma porcentagem como 50%, início ou fim.

## Prompts: the label is shown and said when the prompt opens.

prompt-find = Localizar
prompt-go-to = Ir para linha, porcentagem, início ou fim
prompt-open = Abrir arquivo
prompt-command = Comando
# $label is prompt-command.
prompt-command-palette-intro = { $label }. Digite parte de um nome; Tab completa, Seta para cima e para baixo listam correspondências.
prompt-save-as = Salvar como
prompt-export-as = Exportar como
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
prompt-reference-identifier = DOI, ISBN ou PMID para adicionar
prompt-import-references = Importar referências de um arquivo
prompt-template-title = Título do novo documento
prompt-setting-value = Novo valor, Enter mantém

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
goto-word-start = início
goto-word-end = fim

language-voices-loading = A lista de vozes ainda está carregando, então a voz atual continua falando.

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
chooser-anki-title = Importar cartões do Anki
chooser-anki-files = Pacotes do Anki
chooser-profiles-import-title = Importar perfis
chooser-profiles-export-title = Exportar perfis
chooser-profile-files = Arquivos de perfis
chooser-live-source-files = Arquivos de fontes ao vivo
gui-folder-no-dialog = O seletor de pastas do sistema não abriu. Escolha a pasta nesta lista.
gui-prompt-browse-hint = { $key } abre o navegador de arquivos.

## A secret field's Show choice: the show key, or the Show box in
## the textweaver app, shows a password, token or join code as typed.
prompt-show-hint = { $label }. { $key } mostra.
prompt-secret-hidden = { $count ->
        [one] 1 caractere, oculto
       *[other] { $count } caracteres, ocultos
    }
prompt-show-join-code = Mostrar o código de acesso
prompt-showing-join-code = Código de acesso visível
prompt-hiding-join-code = Código de acesso oculto
prompt-show-password = Mostrar a senha
prompt-showing-password = Senha visível
prompt-hiding-password = Senha oculta
prompt-show-token = Mostrar o token
prompt-showing-token = Token visível
prompt-hiding-token = Token oculto
gui-prompt-show-hint = { $key } ou a caixa Mostrar exibe o que foi digitado.
