### Sync between computers.

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
menu-cw-practice = Prática de código Morse
section-sync = Sincronização

## End of S4

## Sync wave, S5 (see locales/en).
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

## End of S5
