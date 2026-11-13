### CW (Morse code): the speech engine, practice, and keying.

## Beta 4: CW (Morse code) as a speech engine.
cw-sending = CW: { $text }
cw-speed = Velocidad de caracteres { $wpm }, efectiva { $effective } palabras por minuto.
cw-skipped =
    { $count ->
        [one] 1 carácter no enviado en código Morse.
       *[other] { $count } caracteres no enviados en código Morse.
    }

## Beta 4: CW practice (Koch method, random drills, copy checking).
cw-practice-koch-start = Lección Koch { $lesson } de 40, { $count } caracteres, el nuevo: { $new }.
cw-practice-characters-start = Caracteres al azar, { $groups } grupos.
cw-practice-words-start = Palabras al azar, { $groups } grupos.
cw-practice-group = Grupo { $n } de { $groups }.
cw-practice-again = Otra vez, grupo { $n } de { $groups }.
cw-practice-perfect = { $correct } de { $total }, todo correcto.
cw-practice-score = { $correct } de { $total }. Fallos: { $missed }. Enviado: { $sent }.
cw-practice-wrong = { $sent } como { $typed }
cw-practice-missing = { $sent } sin copiar
cw-practice-extra = { $typed } de más
cw-practice-round = Ronda: { $correct } de { $total }, { $percent } por ciento.
cw-practice-added = Lección { $lesson }: se añade { $new }.
cw-practice-not-yet = Con { $threshold } por ciento se añade el siguiente carácter.
cw-practice-all-learned = Todos los caracteres aprendidos.
cw-practice-stopped = Práctica detenida.
cw-practice-progress = Lección Koch { $lesson } de 40, el nuevo: { $new }. { $recent }
cw-practice-recent = Últimas rondas: { $scores } por ciento.
cw-practice-no-rounds = Todavía no hay rondas.
cw-practice-unavailable = El motor CW no se inició, así que la práctica no puede enviar.
cw-char-period = punto
cw-char-comma = coma
cw-char-equals = igual
cw-char-slash = barra
cw-char-question = signo de interrogación

## Beta 4: CW keying, Morse code typed with a key or paddle. $key is a
## key name, $name a MIDI input's name.
cw-keying-on = Manipulación CW activada. Manipule Morse para escribirlo en el cursor; { $key } la desactiva.
cw-keying-off = Manipulación CW desactivada.
cw-keying-ended = Manipulación CW desactivada, porque terminó el modo de edición.
cw-keying-needs-edit = La manipulación CW escribe en modo de edición. Active primero el modo de edición, con { $key }.
cw-keying-listening = Manipulación CW: escuchando { $name }.
cw-keying-midi-failed = La manipulación CW no puede abrir { $name }: { $error } Compruebe que está conectado y vuelva a activar la manipulación.
cw-keying-no-midi = No se encontró ningún adaptador MIDI de manipulador; conéctelo o emparéjelo y vuelva a activar la manipulación. Los adaptadores de teclado funcionan en la aplicación { -brand }.
cw-keying-midi-not-in-build = MIDI no está en esta versión de { -brand }. Los adaptadores de teclado funcionan en la aplicación { -brand }.
cw-keying-no-sidetone = Sin tono local: { $error } La manipulación sigue sin él; use el tono local del adaptador o de la radio.
cw-keying-unknown = No es un carácter Morse.
cw-keying-erased = Señal de error: { $text } borrado.
cw-keying-nothing-erased = Señal de error: nada manipulado que borrar.
cw-keying-rsvp-on = RSVP muestra las palabras que manipula.
