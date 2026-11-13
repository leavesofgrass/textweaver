### The terminal reader.

## The terminal reader's startup.

# $wanted is the speech backend asked for, $backend the one used instead.
tui-setup-backend-unavailable = Le moteur vocal { $wanted } n'est pas disponible ; { $backend } est utilisé.
tui-setup-speech-failed = La synthèse vocale n'a pas pu démarrer ({ $error }) ; exécution silencieuse.
speech-engine-fallback = { $failed } n'a pas pu démarrer ; { $engine } parle à la place.
speech-engine-fallback-silent = { $failed } n'a pas pu démarrer et aucun autre moteur vocal n'est disponible ; { -brand } reste silencieux.
tui-setup-cannot-save = Impossible d'enregistrer les paramètres ou les positions : { $error } La lecture fonctionne. Les changements sont perdus à la fermeture.
tui-setup-keymap-ignored = Fichier de touches ignoré : { $error } Les touches par défaut s'appliquent. Corrigez le fichier, puis redémarrez.
# The first-run welcome. Each value names the key for an action: $play
# reads and pauses, $stop stops, $heading moves to the next heading,
# $help opens the help, $quit quits.
tui-setup-welcome = Bienvenue dans { -brand }. { $open } ouvre un document. { $play } lance et met en pause la lecture, et { $stop } l'arrête. { $palette } liste toutes les commandes. { $help } ouvre l'aide.
gui-setup-welcome-menus = Bienvenue dans { -brand }. { $open } ouvre un document. { $play } lit et met en pause, { $stop } arrête. { $palette } liste toutes les commandes, Alt ou { $menu } les menus. { $help } ouvre l'aide.
# Said at startup without a document. $open, $new, and $help name the
# keys for Open, New Document, and Help.
tui-setup-no-document = Aucun document n'est ouvert. Appuyez sur { $open } pour en ouvrir un, { $new } pour un nouveau, ou { $help } pour de l'aide.

## The terminal reader's launch.

# $name is the file asked for on the command line; $error says why.
tui-could-not-open = Impossible d'ouvrir { $name } : { $error }

## Copying in the terminal reader.

tui-clip-system = Copié avec le presse-papiers système, parce que ce terminal ne peut pas prendre de texte copié.
# $error is why: tui-clip-not-available, tui-clip-not-built, or the
# system's own message.
tui-clip-failed = Impossible de copier avec le presse-papiers système : { $error }. Envoyé au terminal à la place.
tui-clip-not-available = le presse-papiers système n'est pas disponible
tui-clip-not-built = cette version n'a pas de presse-papiers système

## The terminal reader's screen.

# The title line's start; $title is the document's title or
# tui-title-no-document.
tui-title = { -brand } : { $title }
tui-title-no-document = aucun document
# The screen without a document. $keys names the keys for the action.
tui-empty-open = Ouvrir un document : { $keys }.
tui-empty-help = Aide : { $keys }.
tui-empty-quit = Quitter : { $keys }.
# The key hints while a yes-or-no question waits. y, n, and a are the
# answer keys the reader takes; $escape names the Escape key.
tui-hints-confirm = y oui  n ou a non  { $escape } non
# Key hint labels, each shown after its key on the bottom line.
tui-hint-play = lecture
tui-hint-sentence = phrase
tui-hint-faster = plus vite
tui-hint-slower = plus lent
tui-hint-close-rsvp = fermer RSVP
tui-hint-quit = quitter
tui-hint-save = enregistrer
tui-hint-finish = terminer
tui-hint-undo = annuler
tui-hint-bold = gras
tui-hint-heading = titre
tui-hint-commands = commandes
tui-hint-next-line = ligne suivante
tui-hint-previous-line = ligne précédente
tui-hint-again = encore
tui-hint-read-on = continuer
tui-hint-leave = quitter
tui-hint-find = rechercher
tui-hint-mark = marquer
tui-hint-lines = lignes
tui-hint-keys = touches
tui-hint-choose = choisir
tui-hint-close = fermer
tui-hint-back = retour
# The list overlay's border: $n is the focused item's number, $count
# the number of items.
tui-list-title = { $n } sur { $count }, { $title }
