### Live documents, live sources, and the broker.

## Live documents (beta 5): settings, and live sources' passwords. $source
## is a live source's name; $reason says why the credential store failed.
section-live = Documentos ao vivo
prompt-live-password = Senha de { $source }, Enter cancela
prompt-live-password-any = Senha da origem ao vivo, Enter cancela
live-source-unknown = Nenhuma origem ao vivo chamada { $source }. Verifique as origens em settings.toml.
live-password-no-store = Sem armazenamento de credenciais nesta versão: { $source } não pode entrar com senha. Use um pacote de versão.
live-password-invalid = Não é uma senha: até 1024 caracteres, sem quebras de linha nem tabulações.
live-password-kept = Senha de { $source } guardada no armazenamento de credenciais do sistema.
live-password-not-kept = Senha de { $source } não guardada: { $reason }. Usada só nesta sessão.
live-password-forgotten = Senha de { $source } esquecida.
live-password-none = Nenhuma senha guardada para { $source }.
live-password-not-forgotten = Senha de { $source } não esquecida: { $reason }. Tente de novo, ou remova-a no armazenamento de credenciais do sistema.

## Beta 5: live documents (a log fed by a live source, such as an MQTT
## broker). $name is the source's name, $topic an MQTT topic, $path and
## $folder file paths, $line a log entry as one line.
live-log-title = Registro ao vivo
live-log-empty = Ainda não há entradas.
live-entry-empty = Entrada vazia.
live-field-callsign = Indicativo
live-field-name = Nome
live-field-location = Local
live-field-status = Estado
live-field-traffic = Tráfego
live-field-notes = Notas
live-field-time = Hora
live-traffic = Tráfego
live-yes = sim
live-no = não
live-read-only = Este registro ao vivo é somente leitura; não pode ser editado.
live-not-open = Nenhum registro ao vivo está aberto.
live-view-list = Exibição em lista.
live-view-table = Exibição em tabela.
live-newest-none = O registro ainda não tem entradas.
live-paused = Atualizações pausadas. As novas entradas aguardam até você retomar.
live-paused-status = atualizações pausadas
live-stopped = { $name } parou. O registro continua aberto, somente leitura.
live-log-saved = Registro salvo como { $name } em { $folder }.
live-log-save-failed = Não foi possível salvar o registro ao vivo: { $error } Nova tentativa em 10 segundos. Registro: { $path }.
live-log-empty-not-saved = O registro ainda não tem entradas, então nada foi salvo.
live-log-nowhere = Não há pasta para salvar o registro ao vivo.
live-log-saved-already = Este registro foi aberto de { $path }; já está salvo.
live-binary = Ignorado, não é texto: { $len } bytes em { $topic }.
live-too-large = Ignorado, grande demais: { $len } bytes em { $topic }.
live-full = Registro cheio com { $max } entradas; as mais novas são ignoradas.
live-connecting = Conectando a { $name }.
live-reconnecting = Reconectando a { $name }, tentativa { $attempt }.
live-connected = Conectado a { $name }, criptografado.
live-connected-tailscale = Conectado a { $name } num endereço da faixa do Tailscale. Se for o Tailscale, o enlace é criptografado.
live-connected-plain = Conectado a { $name }. A conexão não é criptografada.
live-disconnected = Desconectado de { $name }: { $reason }
live-quiet = Ainda não há mensagens em { $topic }. Se isso continuar, verifique se o broker permite que esta conta leia.
live-unread =
    { $count ->
        [0] nenhuma não lida
        [one] 1 não lida
       *[other] { $count } não lidas
    }
live-entries =
    { $count ->
        [one] 1 entrada
       *[other] { $count } entradas
    }
live-resumed =
    { $count ->
        [0] Atualizações retomadas.
        [one] Atualizações retomadas, 1 atualização aplicada.
       *[other] Atualizações retomadas, { $count } atualizações aplicadas.
    }
live-resumed-dropped = Atualizações retomadas, { $count } aplicadas; { $dropped } mais antigas ignoradas durante a pausa.

## Beta 5: live sources' commands, the set-up steps, and new entries said.
## $name is a source's name, $topic an MQTT topic, $line an entry as one
## line, $reason a sentence in words.
menu-live = Documentos ao vivo
menu-live-reading = Leitura
menu-live-broker = Seu broker
menu-live-invite = Convidar outras pessoas
menu-live-publish = Publicar
prompt-live-setup = Configurar uma fonte ao vivo
prompt-save-live-source-file = Salvar fonte como arquivo
live-failed = Não é possível conectar a { $name }: { $reason } Sem novas tentativas; corrija isso e abra a fonte de novo.
live-topic-refused = Tópico recusado: { $topic }. Esta conta não pode lê-lo.
live-skipped =
    { $count ->
        [one] Ignorou 1 mensagem: chegou rápido demais para mostrar.
       *[other] Ignorou { $count } mensagens: chegaram rápido demais para mostrar.
    }
live-new-entries = { $line } { $count } novas entradas.
live-unread-newest = { $count }. A mais recente: { $line }
live-not-in-build = As fontes ao vivo não estão nesta versão do { -brand }.
live-speak-on = Falar as novas entradas: ligado.
live-speak-off = Falar as novas entradas: desligado.
live-none-set-up = Nenhuma fonte ao vivo configurada. Configurar uma fonte ao vivo adiciona uma.
live-open-title = Fontes ao vivo
live-source-item = { $name }, { $host }, { $topic }
live-forget-title = Esquecer a senha de qual fonte ao vivo?
live-no-passwords = Nenhuma fonte ao vivo entra com nome de usuário, então nenhuma senha está guardada.
live-not-receiving = Nenhuma fonte ao vivo está recebendo.
live-cannot-open = Não é possível abrir a fonte ao vivo. { $problems }
live-not-opened = { $name } não foi aberta: sem senha.
live-setup-host-needed = É preciso um nome de host ou endereço, sem espaços.
live-setup-ca-file-needed = Esta criptografia precisa de um arquivo da autoridade certificadora.
live-setup-ca-file-missing = Nenhum arquivo em { $path }.
live-setup-port-bad = A porta é um número de 1 a 65535.
live-setup-topic-needed = É preciso um tópico.
live-setup-saved = Fonte ao vivo { $name } salva.
live-setup-not-saved = Fonte ao vivo { $name } não salva: { $error } Verifique se a pasta de configurações pode ser gravada e configure-a de novo.
live-setup-offer-title = Abrir { $name } agora?
live-setup-open-now = Abrir agora
live-setup-later = Agora não
live-setup-not-now = Não foi aberta. Abrir fonte ao vivo a abre mais tarde.
live-form-intro-new = Configurar uma fonte ao vivo. Tab muda de campo, Enter salva, Escape cancela.
live-form-intro-edit = Editar { $name }. Tab muda de campo, Enter salva, Escape cancela.
live-form-intro-file = Fonte ao vivo de { $file }. Confira cada campo: Tab muda, Enter a salva como fonte nova, Escape cancela.
live-form-name = Nome
live-form-host = Host do broker
live-form-tls = Criptografia
live-form-ca-file = Arquivo da AC
live-form-port = Porta
live-form-topic = Tópico
live-form-username = Nome de usuário
live-form-view = Visualização
live-form-choice-label = { $label }, { $n } de { $total }, Seta para cima ou para baixo muda
live-form-choice-field = { $label }: { $value }. Seta para cima ou para baixo muda.
live-form-tls-off = Desligada
live-form-tls-system = Raízes incluídas
live-form-tls-ca-file = Arquivo da AC
live-form-tls-off-about = Desligada: sem criptografia, porta 1883; serve pelo Tailscale.
live-form-tls-system-about = Raízes incluídas: TLS, verificado com as autoridades certificadoras que vêm com o { -brand }, porta 8883.
live-form-tls-ca-file-about = Arquivo da AC: TLS, verificado com um arquivo de autoridade certificadora que você indica, porta 8883.
live-form-view-log = Registro
live-form-view-values = Valores
live-form-view-log-about = Registro: uma entrada por mensagem.
live-form-view-values-about = Valores: uma linha por campo, atualizada no lugar.
live-form-name-needed = É preciso um nome.
live-form-not-a-choice = { $label } é um destes: { $choices }. Seta para cima ou para baixo muda.
live-form-saved-beside = Fonte ao vivo { $name } salva, ao lado de outra fonte com o mesmo nome.
live-form-unchanged = { $name } sem alterações.
live-form-cancelled = Cancelado. Nenhuma fonte ao vivo salva.
live-form-cancelled-edit = Cancelado. { $name } sem alterações.
live-edit-title = Editar qual fonte ao vivo?
live-save-file-title = Salvar qual fonte ao vivo como arquivo?
live-save-file-done = { $name } salva como { $path }, sem senha.
live-save-file-failed = { $name } não foi salva como arquivo: { $error } Escolha outra pasta e tente de novo.
live-file-not-read = { $file } não foi aberto como fonte ao vivo: { $reason }
live-file-too-large = ele tem mais de { $max } bytes.
live-file-not-text = não é texto.
live-file-not-toml = não é TOML válido na linha { $line }.
live-file-left-out = Deixado de fora do arquivo: { $items }.
live-file-note-password = uma senha, que o { -brand } nunca lê de um arquivo; ela é pedida depois de salvar
live-file-note-unknown = a chave desconhecida { $key }
live-file-note-too-many = tudo menos os primeiros { $max } de { $key }
live-values-title = Valores ao vivo
live-values-empty = Ainda não há valores.
live-value-none = sem valor
live-value-stale = sem valor recente
live-values-more = mais { $count }
live-values-count =
    { $count ->
        [one] 1 campo
       *[other] { $count } campos
    }
live-values-no-table = A vista de valores não tem tabela: cada campo é uma linha.
live-values-no-log = A vista de valores não mantém um registro para salvar.
live-values-full = A vista de valores comporta no máximo { $max } campos; os demais são deixados de fora.
live-values-resumed =
    { $count ->
        [0] Atualizações retomadas. Nenhum campo mudou.
        [one] Atualizações retomadas, 1 campo mudou.
       *[other] Atualizações retomadas, { $count } campos mudaram.
    }
live-field-none = Não há campo nesta linha.
live-field-show-on = As mudanças de { $field } são mostradas.
live-field-show-off = As mudanças de { $field } não são mais mostradas.
live-field-speak-on = As mudanças de { $field } são faladas.
live-field-speak-off = As mudanças de { $field } não são mais faladas.
live-state-connecting = Conectando
live-state-reconnecting = Reconectando
live-state-encrypted = Conectado, criptografado
live-state-tailscale = Conectado, faixa do Tailscale
live-state-plain = Conectado, sem criptografia
live-state-lost = Conexão perdida

## Beta 5, phase 5b: the broker in the app, sharing a document live,
## check-ins, pinned keys and join codes. $name is a document or live
## source; $rev a revision number; $topic an MQTT topic; $code and $pin
## are in groups of four; $reason is a sentence in words.
prompt-live-check-in = Registro
live-form-pin = Pin
live-form-join-code = Código de acesso
live-form-tls-pinned = Chave fixada
live-form-tls-pinned-about = Chave fixada: TLS, confia só na chave do broker cujo pin você digita, como o broker do { -brand } o mostra, porta 8883.
live-form-tls-join = Código de acesso
live-form-tls-join-about = Código de acesso: TLS com a chave que o código indica, entrando com o código, porta 8883.
live-form-pin-needed = É preciso um pin: o broker o mostra depois de "Certificate pin".
live-form-pin-bad = Não é um pin: ele tem 52 caracteres, como o broker os mostra, ou 12 de um código de acesso, em grupos de quatro.
live-form-join-needed = É preciso um código de acesso: 16 caracteres, em quatro grupos de quatro.
live-form-join-bad = Não é um código de acesso: ele tem 16 letras e algarismos, em quatro grupos de quatro.
live-file-note-join-code = um código de acesso, que o { -brand } nunca lê de um arquivo; peça o próprio código
live-pin-not-read = Não é possível abrir { $name }: o pin dela não é válido. Edite a fonte e digite o pin de novo.
live-document-shown = { $name }, revisão { $rev }, publicado ao vivo.
live-document-updated = { $name } atualizado, revisão { $rev }.
live-document-revision = Revisão { $rev }
live-document-entries-left-out = As entradas de registro desta fonte são deixadas de fora: ela mostra um documento publicado.
live-document-one-view = É um documento publicado: não tem entradas, tabela nem registro.
live-broker-status =
    { $count ->
        [one] Broker ligado, 1 cliente
       *[other] Broker ligado, { $count } clientes
    }
live-share-status = Publicando, revisão { $rev }
live-broker-already = O broker já está ligado.
live-broker-no-folder = não há pasta de dados para a chave e as contas.
live-broker-not-started = Broker não iniciado: { $reason }
live-broker-nobody = não há contas nem código de acesso, então ninguém poderia entrar. Adicione uma conta com tw broker account add, ou ligue o código de acesso.
live-broker-bad-address = { $address } não é um endereço para escutar. Dê um endereço IP, como 127.0.0.1 ou 100.101.1.2, com ou sem porta.
live-broker-started-local = Broker ligado: { $addresses }, TLS, só este computador.
live-broker-started = Broker ligado: { $addresses }, TLS.
live-broker-started-plain-local = Broker ligado: { $addresses }, sem criptografia, só este computador.
live-broker-started-plain = Broker ligado: { $addresses }, sem criptografia.
live-broker-plain-warning = Sem criptografia: qualquer pessoa nesta rede pode ler as mensagens e as senhas. Use TLS, ou só este computador ou o Tailscale.
live-broker-join-code = Código de acesso: { $code }.
live-broker-pin = Pin do certificado: { $pin }.
live-broker-not-running = Nenhum broker está ligado. Iniciar seu broker inicia um.
live-join-code-title = Código de acesso
live-pin-title = Pin do certificado
live-broker-no-join-code = Sem código de acesso: o broker funciona com contas. Os leitores fixam a chave dele.
live-broker-no-join-code-plain = Sem código de acesso nem pin: o broker não tem criptografia.
live-code-group = Grupo { $n } de { $total }: { $group }
live-join-code-copied = Código de acesso copiado.
live-join-link-copied = Link de acesso copiado.
live-join-code-pasted = Código de acesso colado.
live-join-link-filled = Preenchido com o link: host, porta, código e tópico.
live-join-link-filled-no-topic = Preenchido com o link: host, porta e código. Digite o tópico.
live-join-link-bad = Não é um link de acesso: { $reason }
live-form-intro-link = Entrar por um link. Cole o link de acesso em qualquer campo; Enter entra, Escape cancela.
live-form-intro-link-clipboard = Entrar pelo link da área de transferência. Enter entra, Escape cancela.
live-qr-title = Código QR de acesso
live-qr-warning = Qualquer pessoa que veja este código pode entrar.
live-qr-image = Código QR para entrar em { $host }
live-qr-save = Guardar o código QR
live-qr-draw = Desenhar o código QR com caracteres
live-qr-drawing-title = Código QR de acesso com caracteres
live-qr-drawing-warning = Seguem caracteres de bloco para uma câmera; um leitor de tela lê-os como ruído. Escape fecha.
live-qr-failed = Sem código QR: { $reason }
live-qr-save-type = Código QR não guardado: o nome do arquivo deve terminar em .png ou .svg.
live-qr-save-failed = Código QR não guardado: { $error } Escolha outra pasta ou outro nome.
live-qr-saved = Código QR guardado: { $path }
live-qr-intro-clipboard = Entrar a partir do código QR da área de transferência. Enter entra, Escape cancela.
live-qr-intro-file = Entrar a partir do código QR em { $file }. Enter entra, Escape cancela.
live-qr-clipboard-no-link = Nenhum link de acesso na imagem da área de transferência: { $reason } Escolha um arquivo de imagem.
live-qr-no-link = Nenhum link de acesso: { $reason }
live-qr-other-text = O código QR contém outra coisa.
live-qr-not-found = Nenhum código QR foi encontrado na imagem.
live-qr-unreadable = Foi encontrado um código QR, mas não foi possível lê-lo. Tente uma imagem mais nítida ou maior.
live-qr-too-large = O arquivo de imagem é grande demais.
live-qr-too-long = O link de acesso não cabe num código QR.
live-qr-image-error = Não foi possível ler a imagem: { $error } Tente um arquivo PNG ou JPEG.
live-qr-reading = Lendo a imagem QR.
prompt-save-join-qr = Guardar o código QR em arquivo (.png ou .svg)
prompt-join-qr-image = Arquivo de imagem com o código QR
chooser-qr-files = Imagens PNG e SVG
live-field-menu-title = Campo do código de acesso
live-field-hide-join-code = Ocultar o código de acesso
live-field-empty = O campo do código de acesso está vazio.
gui-qr-link = Link de acesso
live-share-stopped-with-broker = A publicação parou com o broker.
live-broker-stopped = Broker parado. Todos os leitores foram desconectados.
live-send-lost = o envio parou antes de terminar.
live-broker-joined = Conectado: { $user }.
live-broker-left = Saiu: { $user }.
live-broker-refused = Entrada recusada: { $user }, de { $address }.
live-broker-event = Broker: { $event }
live-broker-join-off = Código de acesso desativado: códigos errados demais. Inicie o broker de novo para obter um código novo.
live-send-too-large = a mensagem tem { $len } bytes; o limite é { $max }.
live-send-no-password = Não enviado: não há senha guardada para { $name }. Abra-a uma vez para digitar a senha.
live-checkin-sent = Publicado: { $entry }, em { $topic }.
live-share-sent = Publicado: { $name }, revisão { $rev }, em { $topic }.
live-checkin-not-sent = Registro não publicado: { $reason }
live-share-not-sent = { $name } não publicado: { $reason }
live-share-already = { $name } já está sendo publicado em { $topic }. Pare de publicar primeiro.
live-share-no-document = Abra um documento para publicá-lo ao vivo.
live-share-live-document = Um documento ao vivo não pode ser publicado de novo. Abra um arquivo de documento para publicá-lo.
live-share-through-title = Publicar por qual fonte ao vivo?
live-send-nowhere = Não há por onde publicar. Inicie seu broker ou configure uma fonte ao vivo que possa publicar.
live-share-none = Nenhum documento está sendo publicado.
live-share-stopped = Parou de publicar { $name }. Os leitores ficam com a revisão { $rev }.
live-checkin-through-title = Publicar o registro por qual fonte ao vivo?
live-checkin-to-broker = o broker deste computador
live-checkin-intro = Registro para { $to }, em { $topic }. Tab muda de campo, Enter publica, Esc cancela.
live-checkin-empty = Preencha pelo menos um campo.
live-checkin-cancelled = Cancelado. Nenhum registro publicado.
