### CW (Morse code): the speech engine, practice, and keying.

## Beta 4: CW (Morse code) as a speech engine.
cw-sending = CW: { $text }
cw-speed = Velocidade dos caracteres { $wpm }, efetiva { $effective } palavras por minuto.
cw-skipped =
    { $count ->
        [one] 1 caractere não enviado em código Morse.
       *[other] { $count } caracteres não enviados em código Morse.
    }

## Beta 4: CW practice (Koch method, random drills, copy checking).
cw-practice-koch-start = Lição Koch { $lesson } de 40, { $count } caracteres, o novo: { $new }.
cw-practice-characters-start = Caracteres aleatórios, { $groups } grupos.
cw-practice-words-start = Palavras aleatórias, { $groups } grupos.
cw-practice-group = Grupo { $n } de { $groups }.
cw-practice-again = De novo, grupo { $n } de { $groups }.
cw-practice-perfect = { $correct } de { $total }, tudo certo.
cw-practice-score = { $correct } de { $total }. Erros: { $missed }. Enviado: { $sent }.
cw-practice-wrong = { $sent } como { $typed }
cw-practice-missing = { $sent } não copiado
cw-practice-extra = { $typed } a mais
cw-practice-round = Rodada: { $correct } de { $total }, { $percent } por cento.
cw-practice-added = Lição { $lesson }: { $new } acrescentado.
cw-practice-not-yet = Com { $threshold } por cento entra o próximo caractere.
cw-practice-all-learned = Todos os caracteres aprendidos.
cw-practice-stopped = Prática interrompida.
cw-practice-progress = Lição Koch { $lesson } de 40, o novo: { $new }. { $recent }
cw-practice-recent = Últimas rodadas: { $scores } por cento.
cw-practice-no-rounds = Nenhuma rodada ainda.
cw-practice-unavailable = O motor CW não iniciou, então a prática não pode enviar.
cw-char-period = ponto
cw-char-comma = vírgula
cw-char-equals = igual
cw-char-slash = barra
cw-char-question = ponto de interrogação

## Beta 4: CW keying, Morse code typed with a key or paddle. $key is a
## key name, $name a MIDI input's name.
cw-keying-on = Manipulação CW ligada. Manipule Morse para o escrever no cursor; { $key } desliga-a.
cw-keying-off = Manipulação CW desligada.
cw-keying-ended = Manipulação CW desligada, porque o modo de edição terminou.
cw-keying-needs-edit = A manipulação CW escreve no modo de edição. Ligue primeiro o modo de edição, com { $key }.
cw-keying-listening = Manipulação CW: a ouvir { $name }.
cw-keying-midi-failed = A manipulação CW não consegue abrir { $name }: { $error } Verifique se está ligado e volte a ligar a manipulação.
cw-keying-no-midi = Nenhum adaptador MIDI de manipulador encontrado; ligue-o ou emparelhe-o e volte a ligar a manipulação. Os adaptadores de teclado funcionam na aplicação { -brand }.
cw-keying-midi-not-in-build = O MIDI não está nesta versão de { -brand }. Os adaptadores de teclado funcionam na aplicação { -brand }.
cw-keying-no-sidetone = Sem tom local: { $error } A manipulação continua sem ele; use o tom local do adaptador ou do rádio.
cw-keying-unknown = Não é um caractere Morse.
cw-keying-erased = Sinal de erro: { $text } apagado.
cw-keying-nothing-erased = Sinal de erro: nada manipulado para apagar.
cw-keying-rsvp-on = O RSVP mostra as palavras que manipula.
