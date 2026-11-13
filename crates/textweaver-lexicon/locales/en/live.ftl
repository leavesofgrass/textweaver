### Live documents, live sources, and the broker.

## Live documents (beta 5): settings, and live sources' passwords. $source
## is a live source's name; $reason says why the credential store failed.
section-live = Live documents
prompt-live-password = Password for { $source }, Enter cancels
prompt-live-password-any = Password for the live source, Enter cancels
live-source-unknown = No live source named { $source }. Check the sources in settings.toml.
live-password-no-store = No credential store in this version: { $source } cannot sign in with a password. Use a release package.
live-password-invalid = Not a password: up to 1024 characters, with no line breaks or tabs.
live-password-kept = Password kept for { $source }, in the system credential store.
live-password-not-kept = Password not kept for { $source }: { $reason }. Used this session only.
live-password-forgotten = Password forgotten for { $source }.
live-password-none = No password kept for { $source }.
live-password-not-forgotten = Password not forgotten for { $source }: { $reason }. Try again, or remove it in the system credential store.

## Beta 5: live documents (a log fed by a live source, such as an MQTT
## broker). $name is the source's name, $topic an MQTT topic, $path and
## $folder file paths, $line a log entry as one line.
live-log-title = Live log
live-log-empty = No entries yet.
live-entry-empty = Empty entry.
live-field-callsign = Callsign
live-field-name = Name
live-field-location = Location
live-field-status = Status
live-field-traffic = Traffic
live-field-notes = Notes
live-field-time = Time
live-traffic = Traffic
live-yes = yes
live-no = no
live-read-only = This live log is read only; it cannot be edited.
live-not-open = No live log is open.
live-view-list = List view.
live-view-table = Table view.
live-newest-none = The log has no entries yet.
live-paused = Updates paused. New entries wait until you resume.
live-paused-status = updates paused
live-stopped = { $name } stopped. The log stays open, read only.
live-log-saved = Log saved as { $name } in { $folder }.
live-log-save-failed = Could not save the live log: { $error } Trying again in 10 seconds. Log: { $path }.
live-log-empty-not-saved = The log has no entries yet, so nothing was saved.
live-log-nowhere = There is no folder to save the live log in.
live-log-saved-already = This log was opened from { $path }; it is already saved.
live-binary = Left out, not text: { $len } bytes on { $topic }.
live-too-large = Left out, too large: { $len } bytes on { $topic }.
live-full = Log full at { $max } entries; newer ones are left out.
live-connecting = Connecting to { $name }.
live-reconnecting = Reconnecting to { $name }, attempt { $attempt }.
live-connected = Connected to { $name }, encrypted.
live-connected-tailscale = Connected to { $name } at a Tailscale-range address. If this is Tailscale, the link is encrypted.
live-connected-plain = Connected to { $name }. The connection is not encrypted.
live-disconnected = Disconnected from { $name }: { $reason }
live-quiet = No messages yet on { $topic }. If this lasts, check that the broker lets this account read it.
live-unread =
    { $count ->
        [0] none unread
       *[other] { $count } unread
    }
live-entries =
    { $count ->
        [one] 1 entry
       *[other] { $count } entries
    }
live-resumed =
    { $count ->
        [0] Updates resumed.
        [one] Updates resumed, 1 update applied.
       *[other] Updates resumed, { $count } updates applied.
    }
live-resumed-dropped = Updates resumed, { $count } applied; { $dropped } older ones left out while paused.

## Beta 5: live sources' commands, the set-up steps, and new entries said.
## $name is a source's name, $topic an MQTT topic, $line an entry as one
## line, $reason a sentence in words.
menu-live = Live documents
menu-live-reading = Reading
menu-live-broker = Your broker
menu-live-invite = Invite others
menu-live-publish = Publish
prompt-live-setup = Set up a live source
prompt-save-live-source-file = Save live source as file
live-failed = Cannot connect to { $name }: { $reason } No more tries; fix this, then open the source again.
live-topic-refused = Topic refused: { $topic }. This account may not read it.
live-skipped =
    { $count ->
        [one] Left out 1 message: it came too fast to show.
       *[other] Left out { $count } messages: they came too fast to show.
    }
live-new-entries = { $line } { $count } new entries.
live-unread-newest = { $count }. Newest: { $line }
live-not-in-build = Live sources are not in this version of { -brand }.
live-speak-on = Speak new entries on.
live-speak-off = Speak new entries off.
live-none-set-up = No live sources are set up. Set up a live source adds one.
live-open-title = Live sources
live-source-item = { $name }, { $host }, { $topic }
live-forget-title = Forget the password of which live source?
live-no-passwords = No live source signs in with a user name, so no password is kept.
live-not-receiving = No live source is receiving.
live-cannot-open = Cannot open the live source. { $problems }
live-not-opened = { $name } not opened: no password.
live-setup-host-needed = A host name or address is needed, with no spaces.
live-setup-ca-file-needed = A certificate authority file is needed for this encryption.
live-setup-ca-file-missing = No file at { $path }.
live-setup-port-bad = The port is a number from 1 to 65535.
live-setup-topic-needed = A topic is needed.
live-setup-saved = Live source { $name } saved.
live-setup-not-saved = Live source { $name } not saved: { $error } Check that the settings folder can be written to, then set it up again.
live-setup-offer-title = Open { $name } now?
live-setup-open-now = Open it now
live-setup-later = Not now
live-setup-not-now = Not opened. Open live source opens it later.
live-form-intro-new = Set up a live source. Tab moves between fields, Enter saves, Escape cancels.
live-form-intro-edit = Edit { $name }. Tab moves between fields, Enter saves, Escape cancels.
live-form-intro-file = Live source from { $file }. Check each field: Tab moves, Enter saves it as a new source, Escape cancels.
live-form-name = Name
live-form-host = Broker host
live-form-tls = Encryption
live-form-ca-file = CA file
live-form-port = Port
live-form-topic = Topic
live-form-username = User name
live-form-view = View
live-form-choice-label = { $label }, { $n } of { $total }, Up or Down Arrow changes
live-form-choice-field = { $label }: { $value }. Up or Down Arrow changes it.
live-form-tls-off = Off
live-form-tls-system = Bundled roots
live-form-tls-ca-file = CA file
live-form-tls-off-about = Off: not encrypted, port 1883; fine over Tailscale.
live-form-tls-system-about = Bundled roots: TLS, checked against the certificate authorities that come with { -brand }, port 8883.
live-form-tls-ca-file-about = CA file: TLS, checked against a certificate authority file you name, port 8883.
live-form-view-log = Log
live-form-view-values = Values
live-form-view-log-about = Log: one entry per message.
live-form-view-values-about = Values: one line per field, updated in place.
live-form-name-needed = A name is needed.
live-form-not-a-choice = { $label } is one of { $choices }. Up or Down Arrow changes it.
live-form-saved-beside = Live source { $name } saved, beside another source of the same name.
live-form-unchanged = { $name } not changed.
live-form-cancelled = Canceled. No live source saved.
live-form-cancelled-edit = Canceled. { $name } not changed.
live-edit-title = Edit which live source?
live-save-file-title = Save which live source as a file?
live-save-file-done = { $name } saved as { $path }, without a password.
live-save-file-failed = { $name } not saved as a file: { $error } Choose another folder and try again.
live-file-not-read = { $file } was not opened as a live source: { $reason }
live-file-too-large = it is larger than { $max } bytes.
live-file-not-text = it is not text.
live-file-not-toml = it is not valid TOML at line { $line }.
live-file-left-out = Left out of the file: { $items }.
live-file-note-password = a password, which { -brand } never reads from a file; it is asked for after saving
live-file-note-unknown = the unknown key { $key }
live-file-note-too-many = all but the first { $max } { $key }
live-values-title = Live values
live-values-empty = No values yet.
live-value-none = no value
live-value-stale = no recent value
live-values-more = { $count } more
live-values-count =
    { $count ->
        [one] 1 field
       *[other] { $count } fields
    }
live-values-no-table = The values view has no table: each field is one line.
live-values-no-log = The values view keeps no log to save.
live-values-full = The values view holds at most { $max } fields; further fields are left out.
live-values-resumed =
    { $count ->
        [0] Updates resumed. No field changed.
        [one] Updates resumed, 1 field changed.
       *[other] Updates resumed, { $count } fields changed.
    }
live-field-none = There is no field on this line.
live-field-show-on = Changes of { $field } are shown.
live-field-show-off = Changes of { $field } are no longer shown.
live-field-speak-on = Changes of { $field } are spoken.
live-field-speak-off = Changes of { $field } are no longer spoken.
live-state-connecting = Connecting
live-state-reconnecting = Reconnecting
live-state-encrypted = Connected, encrypted
live-state-tailscale = Connected over Tailscale
live-state-plain = Connected, not encrypted
live-state-lost = Connection lost

## Beta 5, phase 5b: the broker in the app, sharing a document live,
## check-ins, pinned keys and join codes. $name is a document or live
## source; $rev a revision number; $topic an MQTT topic; $code and $pin
## are in groups of four; $reason is a sentence in words.
prompt-live-check-in = Check-in
live-form-pin = Pin
live-form-join-code = Join code
live-form-tls-pinned = Pinned key
live-form-tls-pinned-about = Pinned key: TLS, trusting only the broker key whose pin you type, as { -brand }'s broker prints it, port 8883.
live-form-tls-join = Join code
live-form-tls-join-about = Join code: TLS with the key the code names, signing in with the code, port 8883.
live-form-pin-needed = A pin is needed: the broker prints it after "Certificate pin".
live-form-pin-bad = Not a pin: it has 52 characters as the broker prints them, or 12 from a join code, in groups of four.
live-form-join-needed = A join code is needed: 16 characters, in four groups of four.
live-form-join-bad = Not a join code: it has 16 letters and digits, in four groups of four.
live-file-note-join-code = a join code, which { -brand } never reads from a file; ask for the code itself
live-pin-not-read = Cannot open { $name }: its pin is not one. Edit the source and type the pin again.
live-document-shown = { $name }, revision { $rev }, published live.
live-document-updated = { $name } updated, revision { $rev }.
live-document-revision = Revision { $rev }
live-document-entries-left-out = Log entries on this source are left out: it shows a published document.
live-document-one-view = This is a published document: it has no entries, table or log.
live-broker-status =
    { $count ->
        [one] Broker on, 1 client
       *[other] Broker on, { $count } clients
    }
live-share-status = Publishing, revision { $rev }
live-broker-already = The broker is already running.
live-broker-no-folder = there is no data folder for its key and accounts.
live-broker-not-started = Broker not started: { $reason }
live-broker-nobody = no accounts and no join code, so nobody could sign in. Add an account with tw broker account add, or turn the join code on.
live-broker-bad-address = { $address } is not an address to listen on. Give an IP address, such as 127.0.0.1 or 100.101.1.2, with or without a port.
live-broker-started-local = Broker on: { $addresses }, TLS, this computer only.
live-broker-started = Broker on: { $addresses }, TLS.
live-broker-started-plain-local = Broker on: { $addresses }, not encrypted, this computer only.
live-broker-started-plain = Broker on: { $addresses }, not encrypted.
live-broker-plain-warning = Not encrypted: anyone on this network can read the messages and passwords. Use TLS, or this computer or Tailscale only.
live-broker-join-code = Join code: { $code }.
live-broker-pin = Certificate pin: { $pin }.
live-broker-not-running = No broker is running. Start your broker starts one.
live-join-code-title = Join code
live-pin-title = Certificate pin
live-broker-no-join-code = No join code: the broker runs with accounts. Readers pin its key.
live-broker-no-join-code-plain = No join code and no pin: the broker is not encrypted.
live-code-group = Group { $n } of { $total }: { $group }
live-join-code-copied = Join code copied.
live-join-link-copied = Join link copied.
live-join-code-pasted = Join code pasted.
live-join-link-filled = Filled from the join link: host, port, code and topic.
live-join-link-filled-no-topic = Filled from the join link: host, port and code. Type the topic.
live-join-link-bad = Not a join link: { $reason }
live-form-intro-link = Join from a link. Paste the join link into any field; Enter joins, Escape cancels.
live-form-intro-link-clipboard = Join from the link on the clipboard. Enter joins, Escape cancels.
live-qr-title = Join QR code
live-qr-warning = Anyone who sees this code can join.
live-qr-image = QR code for joining { $host }
live-qr-save = Save the QR code
live-qr-draw = Draw the QR code in characters
live-qr-drawing-title = Join QR code in characters
live-qr-drawing-warning = Block characters follow, for a camera; a screen reader reads them as noise. Escape closes.
live-qr-failed = No QR code: { $reason }
live-qr-save-type = QR code not saved: the file name must end in .png or .svg.
live-qr-save-failed = QR code not saved: { $error } Choose another folder or name.
live-qr-saved = QR code saved: { $path }
live-qr-intro-clipboard = Join from the QR code on the clipboard. Enter joins, Escape cancels.
live-qr-intro-file = Join from the QR code in { $file }. Enter joins, Escape cancels.
live-qr-clipboard-no-link = No join link in the clipboard image: { $reason } Choose an image file.
live-qr-no-link = No join link: { $reason }
live-qr-other-text = The QR code holds something else.
live-qr-not-found = No QR code was found in the image.
live-qr-unreadable = A QR code was found but could not be read. Try a sharper or larger image.
live-qr-too-large = The image file is too large.
live-qr-too-long = The join link does not fit in a QR code.
live-qr-image-error = The image could not be read: { $error } Try a PNG or JPEG file.
live-qr-reading = Reading the QR image.
prompt-save-join-qr = Save the QR code to file (.png or .svg)
prompt-join-qr-image = Image file with the QR code
chooser-qr-files = PNG and SVG images
live-field-menu-title = Join code field
live-field-hide-join-code = Hide the join code
live-field-empty = The join code field is empty.
gui-qr-link = Join link
live-share-stopped-with-broker = Publishing stopped with the broker.
live-broker-stopped = Broker stopped. Every reader is disconnected.
live-send-lost = the sending stopped before it finished.
live-broker-joined = Joined: { $user }.
live-broker-left = Left: { $user }.
live-broker-refused = Sign-in refused: { $user }, from { $address }.
live-broker-event = Broker: { $event }
live-broker-join-off = Join code off: too many wrong codes. Start the broker again for a new code.
live-send-too-large = the message is { $len } bytes; the limit is { $max }.
live-send-no-password = Not sent: no password is kept for { $name }. Open it once to type the password.
live-checkin-sent = Published: { $entry }, on { $topic }.
live-share-sent = Published: { $name }, revision { $rev }, on { $topic }.
live-checkin-not-sent = Check-in not published: { $reason }
live-share-not-sent = { $name } not published: { $reason }
live-share-already = Already publishing { $name } on { $topic }. Stop publishing first.
live-share-no-document = Open a document to publish it live.
live-share-live-document = A live document cannot be published again. Open a document file to publish it.
live-share-through-title = Publish through which live source?
live-send-nowhere = Nothing to publish through. Start your broker, or set up a live source that may publish.
live-share-none = No document is being published.
live-share-stopped = Stopped publishing { $name }. Readers keep revision { $rev }.
live-checkin-through-title = Publish a check-in through which live source?
live-checkin-to-broker = this computer's broker
live-checkin-intro = Check-in to { $to }, on { $topic }. Tab moves between fields, Enter publishes, Escape cancels.
live-checkin-empty = Fill in at least one field.
live-checkin-cancelled = Canceled. No check-in published.
