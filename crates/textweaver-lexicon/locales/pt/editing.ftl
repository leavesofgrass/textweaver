### Edit mode: typing, formatting, saving, recovery, and templates.

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
# Edit mode's Obsidian aids: a wiki link, tasks, callouts.
edit-wikilink-title = Link para uma nota
edit-wikilink-intro =
    { $n ->
        [one] Link para uma nota, 1 nota. Enter a escolhe.
       *[other] Link para uma nota, { $n } notas. Enter escolhe uma.
    }
edit-wikilink-no-notes = Não há notas neste cofre para as quais criar um link.
edit-wikilink-anchor-title = Link para uma parte de { $note }
edit-wikilink-anchor-intro = Link para a nota { $note } inteira, ou para um título ou bloco dela. Enter escolhe.
edit-wikilink-whole = A nota inteira
edit-wikilink-heading = Título { $level }: { $text }
edit-wikilink-block = Bloco { $id }: { $text }
edit-wikilink-inserted = Link para { $target } inserido.
edit-callout-title = Tipo de destaque
edit-callout-intro = Tipos de destaque, { $n }. Enter insere um.
edit-callout-inserted = Destaque do tipo { $type } inserido. Digite o seu texto.
edit-task-status =
    { $status ->
        [done] Marcada.
        [open] Não marcada.
        [in-progress] Em andamento.
        [cancelled] Cancelada.
        [forwarded] Encaminhada.
        [important] Importante.
        [question] Pergunta.
       *[other] Marcada com { $mark }.
    }
edit-task-new = Nova tarefa, não marcada.
edit-tasks-changed =
    { $n ->
        [one] Tarefa alterada em 1 linha.
       *[other] Tarefas alteradas em { $n } linhas.
    }
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
edit-recovery-title = Recuperar o trabalho não salvo em { $title }?
edit-recovery-yes = Sim, recuperar { $title } e continuar editando
edit-recovery-no = Não, descartar as alterações não salvas
edit-recovery-discarded = Descartou as alterações não salvas em { $title }.
edit-recovered = Recuperou o trabalho não salvo em { $title }. Lembre-se de salvar.
edit-recovery-postponed = Recuperação adiada. O trabalho não salvo será oferecido de novo da próxima vez.

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
