### CW (Morse code): the speech engine, practice, and keying.

## Beta 4: CW (Morse code) as a speech engine.
cw-sending = CW: { $text }
cw-speed = Character speed { $wpm }, effective { $effective } words per minute.
cw-skipped =
    { $count ->
        [one] 1 character not sent in Morse code.
       *[other] { $count } characters not sent in Morse code.
    }

## Beta 4: CW practice (Koch method, random drills, copy checking).
cw-practice-koch-start = Koch lesson { $lesson } of 40, { $count } characters, newest { $new }.
cw-practice-characters-start = Random characters, { $groups } groups.
cw-practice-words-start = Random words, { $groups } groups.
cw-practice-group = Group { $n } of { $groups }.
cw-practice-again = Again, group { $n } of { $groups }.
cw-practice-perfect = { $correct } of { $total }, all correct.
cw-practice-score = { $correct } of { $total }. Missed: { $missed }. Sent: { $sent }.
cw-practice-wrong = { $sent } as { $typed }
cw-practice-missing = { $sent } not copied
cw-practice-extra = { $typed } extra
cw-practice-round = Round: { $correct } of { $total }, { $percent } percent.
cw-practice-added = Lesson { $lesson }: added { $new }.
cw-practice-not-yet = { $threshold } percent adds the next character.
cw-practice-all-learned = Every character learned.
cw-practice-stopped = Practice stopped.
cw-practice-progress = Koch lesson { $lesson } of 40, newest { $new }. { $recent }
cw-practice-recent = Latest rounds: { $scores } percent.
cw-practice-no-rounds = No rounds yet.
cw-practice-unavailable = The CW engine did not start, so practice cannot send.
cw-char-period = period
cw-char-comma = comma
cw-char-equals = equals
cw-char-slash = slash
cw-char-question = question mark

## Beta 4: CW keying, Morse code typed with a key or paddle. $key is a
## key name, $name a MIDI input's name.
cw-keying-on = CW keying on. Key Morse code to type it at the cursor; { $key } turns keying off.
cw-keying-off = CW keying off.
cw-keying-ended = CW keying off, because edit mode ended.
cw-keying-needs-edit = CW keying types in edit mode. Turn on edit mode first, with { $key }.
cw-keying-listening = CW keying: listening to { $name }.
cw-keying-midi-failed = CW keying cannot open { $name }: { $error } Check that it is connected, then turn keying on again.
cw-keying-no-midi = No MIDI key adapter was found; connect or pair it, then turn keying on again. Keyboard adapters work in the { -brand } app.
cw-keying-midi-not-in-build = MIDI is not in this version of { -brand }. Keyboard adapters work in the { -brand } app.
cw-keying-no-sidetone = No sidetone: { $error } Keying goes on without it; use your adapter's or radio's sidetone.
cw-keying-unknown = Not a Morse character.
cw-keying-erased = Error sign: { $text } erased.
cw-keying-nothing-erased = Error sign: nothing keyed to erase.
cw-keying-rsvp-on = RSVP shows the words you key.
