### Optional components and updates.

## Componentes opcionais.
component-feature-dictation = o ditado
component-feature-ocr = ler páginas digitalizadas
component-feature-reading-font = uma fonte de leitura
component-feature-voice = uma voz
component-feature-audio-export = exportar M4B e MP4
component-feature-braille = arquivos braille como tinta, braille contraído
component-feature-pandoc = mais formatos de documento
component-state-installed = instalado
component-state-not-installed = não instalado
component-state-partial = instalado em parte
component-state-damaged = danificado
component-state-downloading = baixando
component-state-installer-ready = instalador baixado
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
component-error-unpack = Não instalado: o arquivo compactado não foi extraído.
component-launch-question = Iniciar instalador: { $title }, versão { $version }, licença { $license }? Depois o sistema pergunta. y ou n
component-version-unknown = não informada
component-launched = Instalador iniciado: { $title }. Siga as perguntas do sistema.
component-launch-failed = O instalador não iniciou: { $reason }
component-launch-declined = Não iniciado; o instalador foi mantido.
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
helper-question = { $need } precisa de { $title }, { $size }, licença { $license }. Baixar agora? y ou n
helper-need-audio = Exportar M4B e MP4
helper-need-brf = Ler braille como tinta
helper-run-command = Para obter { $title }, execute: { $command }
dictation-model-not-in-build = Sem modelo e sem downloads aqui.
dictation-model-file-missing = Falta ao modelo { $file }.
dictation-model-damaged = Modelo de ditado danificado: { $file }.
dictation-model-no-folder = Pasta não existe: { $dir }.

## Configurações dos componentes opcionais.

## Atualizações (B1-u1).
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
section-updates = Atualizações e componentes
