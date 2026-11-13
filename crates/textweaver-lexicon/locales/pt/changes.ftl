### Tracked changes and comments in Word files.

## B1-t1: alterações controladas e comentários (a lista de alterações).
prompt-comment-reply = Resposta
prompt-comment-text = Comentário
prompt-github-token = Token do GitHub da origem de componentes, Enter pula
prompt-import-cards = Importar cartões de estudo de um pacote do Anki
prompt-cw-copy = Digite o que você ouviu
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

## B1-t2: guardar a revisão no arquivo do Word.
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
