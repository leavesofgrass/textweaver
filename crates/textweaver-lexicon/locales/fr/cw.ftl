### CW (Morse code): the speech engine, practice, and keying.

## Beta 4: CW (Morse code) as a speech engine.
cw-sending = CW : { $text }
cw-speed = Vitesse des caractères { $wpm }, effective { $effective } mots par minute.
cw-skipped =
    { $count ->
        [one] 1 caractère non envoyé en morse.
       *[other] { $count } caractères non envoyés en morse.
    }

## Beta 4: CW practice (Koch method, random drills, copy checking).
cw-practice-koch-start = Leçon Koch { $lesson } sur 40, { $count } caractères, nouveau : { $new }.
cw-practice-characters-start = Caractères au hasard, { $groups } groupes.
cw-practice-words-start = Mots au hasard, { $groups } groupes.
cw-practice-group = Groupe { $n } sur { $groups }.
cw-practice-again = Encore, groupe { $n } sur { $groups }.
cw-practice-perfect = { $correct } sur { $total }, tout juste.
cw-practice-score = { $correct } sur { $total }. Erreurs : { $missed }. Envoyé : { $sent }.
cw-practice-wrong = { $sent } pris pour { $typed }
cw-practice-missing = { $sent } non copié
cw-practice-extra = { $typed } en trop
cw-practice-round = Série : { $correct } sur { $total }, { $percent } pour cent.
cw-practice-added = Leçon { $lesson } : { $new } ajouté.
cw-practice-not-yet = { $threshold } pour cent ajoute le caractère suivant.
cw-practice-all-learned = Tous les caractères sont appris.
cw-practice-stopped = Entraînement arrêté.
cw-practice-progress = Leçon Koch { $lesson } sur 40, nouveau : { $new }. { $recent }
cw-practice-recent = Dernières séries : { $scores } pour cent.
cw-practice-no-rounds = Aucune série pour l’instant.
cw-practice-unavailable = Le moteur CW n’a pas démarré, l’entraînement ne peut donc pas envoyer.
cw-char-period = point
cw-char-comma = virgule
cw-char-equals = égal
cw-char-slash = barre oblique
cw-char-question = point d’interrogation

## Beta 4: CW keying, Morse code typed with a key or paddle. $key is a
## key name, $name a MIDI input's name.
cw-keying-on = Manipulation CW activée. Manipulez du morse pour l'écrire au curseur ; { $key } la désactive.
cw-keying-off = Manipulation CW désactivée.
cw-keying-ended = Manipulation CW désactivée, car le mode édition a pris fin.
cw-keying-needs-edit = La manipulation CW écrit en mode édition. Activez d'abord le mode édition, avec { $key }.
cw-keying-listening = Manipulation CW : écoute de { $name }.
cw-keying-midi-failed = La manipulation CW ne peut pas ouvrir { $name } : { $error } Vérifiez qu'il est connecté, puis réactivez la manipulation.
cw-keying-no-midi = Aucun adaptateur MIDI de manipulateur trouvé ; connectez-le ou appairez-le, puis réactivez la manipulation. Les adaptateurs clavier fonctionnent dans l'application { -brand }.
cw-keying-midi-not-in-build = Le MIDI n'est pas dans cette version de { -brand }. Les adaptateurs clavier fonctionnent dans l'application { -brand }.
cw-keying-no-sidetone = Pas de tonalité locale : { $error } La manipulation continue sans elle ; utilisez la tonalité locale de l'adaptateur ou de la radio.
cw-keying-unknown = Pas un caractère morse.
cw-keying-erased = Signal d'erreur : { $text } effacé.
cw-keying-nothing-erased = Signal d'erreur : rien de manipulé à effacer.
cw-keying-rsvp-on = Le RSVP montre les mots que vous manipulez.
