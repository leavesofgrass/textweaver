### The terminal reader.

## The terminal reader's startup.

# $wanted is the speech backend asked for, $backend the one used instead.
tui-setup-backend-unavailable = O motor de fala { $wanted } não está disponível; usando { $backend }.
tui-setup-speech-failed = A fala não pôde iniciar ({ $error }); executando em silêncio.
tui-setup-cannot-save = Não é possível salvar configurações ou posições: { $error } A leitura funciona. As alterações se perdem ao sair.
tui-setup-keymap-ignored = Arquivo de teclas ignorado: { $error } As teclas padrão valem. Corrija o arquivo e reinicie.
# The first-run welcome. Each value names the key for an action: $play
# reads and pauses, $stop stops, $heading moves to the next heading,
# $help opens the help, $quit quits.
# Said at startup without a document. $open, $new, and $help name the
# keys for Open, New Document, and Help.
tui-setup-no-document = Nenhum documento está aberto. Pressione { $open } para abrir um, { $new } para um novo, ou { $help } para ajuda.

## The terminal reader's launch.

# $name is the file asked for on the command line; $error says why.
tui-could-not-open = Não foi possível abrir { $name }: { $error }

## Copying in the terminal reader.

tui-clip-system = Copiado com a área de transferência do sistema, porque este terminal não aceita texto copiado.
# $error is why: tui-clip-not-available, tui-clip-not-built, or the
# system's own message.
tui-clip-failed = Não foi possível copiar com a área de transferência do sistema: { $error }. Enviado ao terminal em vez disso.
tui-clip-not-available = a área de transferência do sistema não está disponível
tui-clip-not-built = esta versão não tem área de transferência do sistema

## The terminal reader's screen.

# The title line's start; $title is the document's title or
# tui-title-no-document.
tui-title = { -brand }: { $title }
tui-title-no-document = sem documento
# The screen without a document. $keys names the keys for the action.
tui-empty-open = Abrir um: { $keys }.
tui-empty-help = Ajuda: { $keys }.
tui-empty-quit = Sair: { $keys }.
# The key hints while a yes-or-no question waits. y, n, and a are the
# answer keys the reader takes; $escape names the Escape key.
tui-hints-confirm = y sim  n ou a não  { $escape } não
# Key hint labels, each shown after its key on the bottom line.
tui-hint-play = reproduzir
tui-hint-sentence = frase
tui-hint-faster = mais rápido
tui-hint-slower = mais devagar
tui-hint-close-rsvp = fechar RSVP
tui-hint-quit = sair
tui-hint-save = salvar
tui-hint-finish = terminar
tui-hint-undo = desfazer
tui-hint-bold = negrito
tui-hint-heading = cabeçalho
tui-hint-commands = comandos
tui-hint-next-line = próxima linha
tui-hint-previous-line = linha anterior
tui-hint-again = de novo
tui-hint-read-on = continuar lendo
tui-hint-leave = sair
tui-hint-find = localizar
tui-hint-mark = marcar
tui-hint-lines = linhas
tui-hint-keys = teclas
tui-hint-choose = escolher
tui-hint-close = fechar
tui-hint-back = voltar
# The list overlay's border: $n is the focused item's number, $count
# the number of items.
tui-list-title = { $n } de { $count }, { $title }
