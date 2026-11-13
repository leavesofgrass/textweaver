### The study sheet, the self-test, and study cards.

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
cards-anki-exported =
    { $n ->
        [one] 1 cartão exportado para o Anki: { $file }, em { $folder }.
       *[other] { $n } cartões exportados para o Anki: { $file }, em { $folder }.
    }
cards-anki-export-failed = Não foi possível exportar os cartões para o Anki: { $error } Verifique se a pasta do documento pode ser gravada.
cards-anki-import-failed = Não foi possível importar { $file }: { $error } Verifique se o arquivo é um pacote do Anki, .apkg.
cards-anki-imported =
    { $added ->
        [one] 1 cartão importado.
       *[other] { $added } cartões importados.
    }
cards-anki-already =
    { $n ->
        [one] 1 cartão já estava aqui; suas avaliações foram mescladas.
       *[other] { $n } cartões já estavam aqui; suas avaliações foram mescladas.
    }
cards-anki-skipped-cloze =
    { $n ->
        [one] 1 nota de lacunas ignorada.
       *[other] { $n } notas de lacunas ignoradas.
    }
cards-anki-skipped-media =
    { $n ->
        [one] 1 nota só com imagens ou som ignorada.
       *[other] { $n } notas só com imagens ou som ignoradas.
    }
cards-anki-skipped-other =
    { $n ->
        [one] 1 nota sem texto ou com mais de dois cartões ignorada.
       *[other] { $n } notas sem texto ou com mais de dois cartões ignoradas.
    }
cards-from-anki = Este cartão veio do Anki; não tem lugar no documento.
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
