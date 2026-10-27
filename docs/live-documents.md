# Live documents

A live document is a document that grows while you read it. textweaver connects to an MQTT broker, subscribes to one or more topics, and shows what arrives as a document: a log with one entry per message, or a list of values with one line per field. You read it like any other document, with the same keys, headings, tables, find, and bookmarks, on the screen, on a Braille display through your screen reader, and by speech if you ask for it.

MQTT is a small publish and subscribe protocol that sensors, home automation, and logging tools use. Any standard MQTT broker and any topic can be opened as a live document. A net log is the first use: a friend logs check-ins as JSON and publishes them, and textweaver shows the log as it grows. A weather station or a workshop sensor is another.

textweaver only receives. It never publishes a message, so a live document cannot change what the broker holds.

This guide is for the person who reads the live document. If you also run the broker, [Running the broker for a live log](live-broker.md) covers textweaver's own broker (`tw broker`, `tw send` and `tw share`), Mosquitto, Tailscale, and 44Net.

## In brief

1. Run **Set up a live source** (Tools, Live sources, or the command palette) and answer its questions: a name, the broker's host, encryption, port, topic, user name, and view.
2. Run **Open live source**, choose the source, and press Enter. If it needs a password, textweaver asks once and keeps it in your system's credential store.
3. Read the log. New entries arrive at the end and never move your place. Press `Alt+Shift+F6` to jump to the newest entry.
4. When you close the source, textweaver has already saved the entries as a JSON file.

## The log view

The log view suits a stream of records, such as check-ins. Each message adds an entry, or corrects an earlier one.

### What a message can be

textweaver accepts these shapes, and the sender can mix them:

- **One JSON object** is one entry. It is added at the end.
- **A JSON list of objects** is the whole log. It replaces what you have and keeps you on the same entry. A broker can hold such a list as a retained message, so a reader who joins late receives the whole log at once.
- **An object wrapping one list** is also a whole log. The list may sit under `checkins`, `entries`, `log`, `stations`, `items`, or `records`. Failing those, the object's only list of objects is used, if the object has no callsign or name of its own.
- **Text that is not JSON** becomes one entry holding that text.
- **A binary message, or one over 256 KiB,** is left out, and textweaver says so.

An entry whose id matches an earlier entry corrects that entry in place, so a fixed spelling or a changed note replaces the old line instead of adding a second one. The id is the field named by the source's `id_field` setting, else the first of `id`, `uuid`, `serial`, `serialno`, or `seq`. With no id, the callsign is used, ignoring case and spaces.

This has one consequence worth knowing. If the sender gives no id, a second check-in from the same callsign corrects the first instead of adding a new entry. A logger that records a station twice should send an id.

A message cannot delete an entry. To remove one, the sender publishes the whole log again without it: a whole log replaces the entries you have.

A log holds at most 20,000 entries. Each entry keeps at most 64 fields, and a value is shown to 500 characters. What is over is left out, and textweaver says so.

### Field names

textweaver recognizes common field names, ignoring case, so most logs need no setup. Within each group the first name found is the one shown.

- **Callsign:** `callsign`, `call`, `call_sign`, `cs`, `col_call`.
- **Name:** `name`, `first_name`, `firstname`, `preferredname`, `operator`, `op`.
- **Location:** `location`, `loc`, `qth`, `city`, `citycountry`, `state`, `county`, `grid`, `gridsquare`.
- **Status:** `status`, `state_of_checkin`, `checkin_type`, `role`.
- **Traffic:** `traffic`, `has_traffic`, `msgs`, `messages`.
- **Notes:** `notes`, `note`, `remarks`, `comment`, `comments`, `text`, `message`.
- **Time:** `time`, `timestamp`, `ts`, `utc`, `checked_in_at`, `time_on`, `col_time_on`. The time is shown as the sender wrote it.

Every other field is kept and shown after these, in the order the message sent it. Nothing is dropped silently. The recognized fields are shown in the order above, however the message ordered them.

### List and table

The log opens as a **list**, one entry per line, with the meaning first so that a line fits a 40-cell Braille display:

```text
N0CALL, Ada Example. Portland, OR. mobile. Portable antenna.
```

The callsign and name come first, joined by a comma. The other known fields follow as sentences. A true traffic field reads "Traffic", and a false one "Traffic: no". The time reads "Time: " and the value. Unknown fields read "key: value".

The same log can be a **table** with real column headers: Callsign, Name, Location, Status, Traffic, Notes, and Time when entries have them, then a column for each other field. Switching with `Alt+Shift+F12` keeps you on the same entry. In the textweaver app the switch is a button named for the view it switches to, "Table view" or "List view".

Both views are ordinary document structure, so a screen reader's table commands work in the table, and list commands work in the list. A log always opens as a list, and one live source is open at a time.

### The heading line

When the whole log arrives wrapped in an object, such as:

```json
{"net": "Practice net", "frequency": "147.040", "checkins": [{"call": "N0CALL", "name": "Ada Example"}]}
```

the object's plain values become a line above the entries, in both views: "Practice net. frequency: 147.040." A value named `net`, `name`, `title`, `event`, or `session` comes first, with no label. The heading line is kept in the JSON log and shown again when the log is reopened.

### Unread entries, and your place

New entries are added at the end of the document. Your cursor, selection, bookmarks, find matches, the spoken range, and the Braille display stay on the same text. A new entry never moves the screen reader.

- The unread count is the number of entries after the furthest entry you have reached. The title line and the status bar show it ("3 unread") in place of a position that would change with every entry.
- **Say unread entries** (`Alt+Shift+F5`) says the count and the newest entry.
- **Jump to newest entry** (`Alt+Shift+F6`) moves to the last entry and says it.
- **Pause or resume updates** (`Alt+Shift+F1`) holds new messages while you read a stretch undisturbed. Up to 1,000 messages wait, and a whole log replaces the ones that waited. On resuming, textweaver applies them as one change and says how many ("Updates resumed, 3 updates applied.").

A live document is read only. Edit mode is refused in words. textweaver saves no reading position for it, adds it to no library or recent list, and never syncs it. Save writes the JSON log (see [The JSON log](#the-json-log)).

## The values view

The values view suits a sensor or any source whose messages report the current state of a few things. It shows one line per field, meaning first, and updates each line in place:

```text
Temperature: 21.4 C
Door: closed
Battery: 3.9
```

A change replaces only its own line, so the cursor and the Braille display stay on the line you are reading.

- **Mapped fields come first,** by their `order` setting, else in the order listed, with their label, unit, decimals, and value words (see [Settings for a source](#settings-for-a-source)). The other fields follow in the order the messages sent them, labeled from their keys: `wind_kmh` reads "Wind kmh".
- **A field keeps its place** once shown. A field missing from a later message reads "no value" instead of vanishing.
- **Nested objects** become groups under headings. A list of plain values is one line per item, up to 20, then a line saying how many more. A list of objects is a group per item, titled by its name or id.
- **A message that is not an object** is one field, named after the last level of its topic. Numeric text counts as a number.
- **With more than one topic,** each topic's fields are grouped under a heading naming the topic.
- **Stale values are said in words.** A value not updated for a while reads "Temperature: 21.4 C, no recent value". The time is the source's `stale_after_seconds`, else `quiet_hint_seconds` in the `[live]` settings. Zero never marks a value stale.
- **Jump to newest** moves to the field that changed last. **Pause** keeps changes out of sight, and resuming shows them in one step and says how many fields changed. The title line shows the number of fields.
- **The values view keeps no JSON log,** because it holds current values, not a history.

### Show and speak, per field

Each field has two choices, saved with the source:

- **Show changes** (on by default) sends a change of the field to the screen reader as a message, so it reaches the Braille display.
- **Speak changes** (off by default) also has textweaver's own voice say it.

On the field's line, run **Show changes of this field** or **Speak changes of this field** from the command palette or the Tools menu. In the textweaver app they are buttons named for their state: "Show changes: on". On a line that holds no field, such as a heading, they are unavailable and say why.

A field's changes are announced at most once every `min_interval_seconds` (10 by default), and only its latest value is said. A busy sensor does not flood the display.

## Setting up a source

### The guided way

**Set up a live source** (Tools, Live sources, or the command palette) asks one question at a time:

1. A name, such as "Practice log". Pressing Enter with nothing typed cancels.
2. The broker's host name or address.
3. Encryption: off, TLS checked against the bundled certificate authorities, or TLS checked against a certificate authority file.
4. The certificate authority file, only if you chose that.
5. The port. Enter keeps the default: 1883 with encryption off, 8883 with it on.
6. The topic to read, such as `log/checkins`.
7. A user name. Enter means none.
8. The view: log or values.

textweaver then writes the source to `settings.toml` and offers to open it. The source's id comes from its name, in lowercase with hyphens between words ("Practice log" becomes `practice-log`), and a number is added if that id is taken. An id may hold only letters, digits, hyphens, and underscores.

### Settings for a source

You can also write sources by hand in `settings.toml` ([Settings](settings.md#where-settings-live) says where the file is). The `[live]` section holds the choices that apply to every source:

- `announce_new_entries`, default `true`: send each new entry, whole, to the screen reader and so to the Braille display.
- `speak_new_entries`, default `false`: have textweaver's own voice say each new entry.
- `min_interval_seconds`, default `10`, 0 to 3600: in the values view, the fewest seconds between two announcements of one field.
- `quiet_hint_seconds`, default `60`, 0 to 3600: after this many seconds without a message on a topic, textweaver says so, with a hint. Zero never hints.
- `log_folder`, default empty: where JSON logs are saved. Empty means the `logs` folder in textweaver's data folder. A relative folder is inside the data folder, not the folder textweaver was started from.

Each source is a table, `[live.sources.<id>]`:

- `name`: what textweaver shows and says. Empty means the id.
- `host`: the broker's host name or address. Required.
- `port`: 1883, or 8883 when `tls` is on. A port of 0, or one over 65535, is not valid.
- `tls`: `off`, `system`, or `ca_file`. The value `system` trusts the certificate authorities bundled with textweaver (Mozilla's list), not your operating system's store. With `ca_file`, textweaver trusts only the authority in that file. There is no way to turn off certificate checking.
- `ca_file`: the path of a PEM file, with `tls = "ca_file"`. Use a full path.
- `username`: the user name to sign in with. Empty means none.
- `topics`: a list of topics. MQTT's wildcards work: `+` for one level and `#` for the rest.
- `view`: `log` (the default) or `values`.
- `id_field`: in the log view, the field that identifies an entry, so a later message with the same value corrects it.
- `stale_after_seconds`: in the values view, when a value reads "no recent value". Zero never.

In the values view, each `[[live.sources.<id>.fields]]` entry maps one field:

- `path`: where the field is in the message: dotted (`sensor.temp`) or a JSON Pointer (`/sensor/temp`).
- `label`: its name, such as "Temperature". Empty means the field's own name.
- `unit`: said after the value, such as "C".
- `decimals`: how many decimals a number shows, 0 to 9. Unset means as sent.
- `values`: words for raw values, such as `{ "0" = "closed", "1" = "open" }`. A value with words shows the words and no unit.
- `show` and `speak`: `off` or `on_change`. `show` defaults to `on_change`, `speak` to `off`.
- `order`: its place among the fields, smallest first.

A setting that does not fit is dropped with a warning in words ("live.sources.net.port 70000 is outside 1 to 65535; using the default"), and the rest of the source is kept. When you open a source that cannot work, such as one with no host or no topic, textweaver says what is wrong in words.

Sources and the log folder belong to this computer, and never sync. The announcement choices in `[live]` do sync.

### Worked example: a net log

A friend publishes check-ins to `netlog/checkins` on a broker reached over Tailscale, and you sign in as `reader`:

```toml
[live.sources.netlog]
name = "Practice net"
host = "broker.example.ts.net"
topics = ["netlog/#"]
username = "reader"
```

Tailscale encrypts the link, so the plain port 1883 is fine. If the broker is reached across the open internet, such as over 44Net, use TLS with the friend's certificate authority file:

```toml
[live.sources.netlog]
name = "Practice net"
host = "broker.example.org"
tls = "ca_file"
ca_file = 'C:\Users\Ada\netlog-ca.crt'
topics = ["netlog/#"]
username = "reader"
```

The single quotes make this a literal TOML string, so the backslashes stay as written. The host must be the name or address that the broker's certificate names. Add `id_field = "serial"` if the logger numbers check-ins with a different field name.

### Worked example: a sensor

A workshop sensor publishes `{"temp": 21.43, "door": 0, "battery": 3.9}` to `shed/status`:

```toml
[live.sources.shed]
name = "Shed sensors"
host = "broker.example.ts.net"
topics = ["shed/#"]
view = "values"
stale_after_seconds = 300

[[live.sources.shed.fields]]
path = "temp"
label = "Temperature"
unit = "C"
decimals = 1
order = 1

[[live.sources.shed.fields]]
path = "door"
label = "Door"
values = { "0" = "closed", "1" = "open" }
speak = "on_change"
order = 2
```

This shows "Temperature: 21.4 C", then "Door: closed", then "Battery: 3.9", which has no mapping. The door's changes are also spoken in textweaver's own voice, if that voice is on.

### The password

A source's password is never written to `settings.toml`, a log, a message, or the status line. When a source has a user name and no password is kept, textweaver asks for it in a masked prompt that shows one star per character, and keeps it in your system's credential store under the account `live:<id>`, beside the GitHub token. After that, opening the source needs no password.

- **Forget live source password** (Tools, Live sources) removes it, so textweaver asks again the next time. Use it after the password changes.
- A version of textweaver built without the credential store cannot keep a password. It says so in words rather than keep one in a file.
- In the textweaver app, the masked field is best edited at the end: type, and press Backspace to delete the last characters. See [Known limits](known-limits.md#live-documents).

## Announcements: Braille and speech

Each new or corrected entry is sent whole, meaning first, as one polite message through the same route as every other message from textweaver. A screen reader shows that message on a Braille display, and speaks it unless its speech is off. This is the way to read a log by Braille while you listen to something else, such as a radio net.

- **textweaver's own voice is off by default** for new entries. `speak_new_entries` turns it on, and the command **Speak new entries** switches it on or off for this session. The voice applies only in the self-voicing and hybrid modes. With a screen reader in charge, textweaver never speaks.
- **`announce_new_entries`** turns the messages off altogether. The log still holds every entry.
- **They are never silenced by the announcement level** in Settings, since an entry is content you asked to receive.
- **They never interrupt reading.** While textweaver reads aloud, or reads to the status line for a screen reader, new entries wait and go out as one message when reading stops.
- **Bursts become one message.** Entries that arrive within two seconds of the last announcement are said together: the newest entry whole and first, then how many ("3 new entries").
- **A lost connection is the only assertive message.** Everything else is polite.

### Braille only

To read by Braille without hearing the entries, turn off your screen reader's speech and leave its Braille on. textweaver needs no separate setting, because it sends the entry through the screen reader and the screen reader decides what to do with it. Keep `speak_new_entries` off so that textweaver adds no voice of its own.

How long a message stays on the display is the screen reader's setting, not textweaver's. In NVDA it is the Braille message timeout (Settings, Braille, "Show messages" and "Message timeout"). In JAWS it is the flash message time (Settings Center, Braille). If entries vanish before you finish them, lengthen that time. The log itself always holds every entry, and **Say unread entries** brings back the newest one. These settings have not yet been tried with a live log on a Mantis Q40; see the checklist in [Using textweaver with a screen reader](screen-readers.md).

### The terminal reader

In the terminal reader, the status line carries each new or corrected entry, meaning first, cut at a word boundary to fit one 40-cell Braille line, with "..." where words were left out. The unread count follows when the whole still fits ("N0CALL, Ada.  3 unread"). The line changes once per entry, so moving through the log does not change it and the screen reader does not read it again. Any other message keeps the status line until the next entry.

## The JSON log

textweaver saves the log as a JSON file, so a crash or a closed window loses little.

- **When:** at most every 10 seconds while the log runs, and when it closes. **Save live log now** writes it at once and says where.
- **Where:** the `logs` folder of textweaver's data folder, or `log_folder`. `tw settings path` prints the data folder. The folder is made when the first entry arrives.
- **File name:** `live-log-` and the start time, `YYYY-MM-DD-HHMM`, in local time: `live-log-2026-10-26-2010.json`. A source's id comes first when it has one: `netlog-live-log-2026-10-26-2010.json`. If that name exists, textweaver adds `-2`, `-3`, and so on.
- **What it holds:** the entries as received, with the time each arrived and the time it was last corrected, and the wrapper's heading line. Keys keep the order the sender used.
- **Reopening:** open the file like any document, with Ctrl+O or `tw FILE`. textweaver recognizes it by its content and shows it in the same log view, read only. Any other JSON file opens as before.
- **Not saved:** the values view, which keeps no log.

## Keys and buttons

These keys work in the terminal reader and the textweaver app. On a Mac, use Option in place of Alt.

- `Alt+Shift+F5`: say the unread entries.
- `Alt+Shift+F6`: jump to the newest entry.
- `Alt+Shift+F12`: switch between list and table.
- `Alt+Shift+F1`: pause or resume updates.

Every other command is in Tools, Live sources and in the command palette (F2): Open live source, Set up a live source, Close live source, Speak new entries, Show changes of this field, Speak changes of this field, Save live log now, and Forget live source password. Close live source stops receiving; the log stays open, read only, and is saved. [The keyboard reference](keyboard.md) lists every key, and you can give any of these commands a key in `keymap.toml`.

In the textweaver app, a live document's buttons join the toolbar while it is open. [The textweaver app](gui.md#a-live-documents-buttons-and-status-bar) lists them and describes the status bar, which shows the unread count and the connection in words.

## Privacy

- **A broker sees your connection.** It sees your computer's network address, the user name you sign in with, the topics you subscribe to, and when you connect. If the link is not encrypted, anyone on the path between you can also read the user name, the password, and every message. Over Tailscale the link is encrypted by Tailscale. Over the open internet, use TLS.
- **The client id is neutral.** textweaver connects as `tw-` and 16 random hexadecimal digits, new at each start. It sends no name, computer name, or account name.
- **textweaver sends nothing else.** It publishes no message. It subscribes, answers the broker's acknowledgements, and keeps the connection alive.
- **Passwords stay in the credential store.** They are never written to settings, logs, or the status line.
- **Messages are data.** Sizes, nesting, and field counts are capped, and control characters, terminal escape sequences, and text-direction overrides are removed from every topic, key, and value before display.
- **Where the log goes.** The JSON log stays on your computer, in the folder above. It is never synced. A log can hold other people's names and callsigns, so treat the file with care.

See also [Privacy](privacy.md).

## Connection messages

textweaver says each change of state once, in words, and starts each message with its meaning. Reading is never interrupted for these.

- **"Connecting to NAME."** textweaver is trying. Nothing to do.
- **"Connected to NAME, encrypted."** TLS is in use.
- **"Connected to NAME over Tailscale, which encrypts the link."** The host is a Tailscale address or a `.ts.net` name. Plain MQTT is fine here.
- **"Connected to NAME. The connection is not encrypted."** Fine for a broker on your own computer or network. For a broker across the internet, set `tls`.
- **"Reconnecting to NAME, attempt N."** The link dropped and textweaver is trying again, waiting longer after each failure, up to a minute.
- **"Disconnected from NAME: ..."** The link was lost. This is the only assertive message. textweaver reconnects and subscribes again by itself.
- **"Cannot connect to NAME: ... No more tries; fix this, then open the source again."** The cause is something only you can fix, so textweaver stops. The reason comes first:
  - "Not authorized: ..." The user name or password is wrong, or the account may not connect. Run Forget live source password, open the source, and type the password again. Mosquitto answers a wrong password this way.
  - "TLS failure: the broker's certificate is not trusted." The broker uses a private certificate. Set `tls = "ca_file"` and name the authority's file.
  - "TLS failure: ... a different host name." The `host` setting is not the name in the certificate. Use the name the certificate carries.
  - "TLS failure: ... expired" or "not valid yet." Renew the certificate, or check this computer's clock.
  - "TLS needed: ... answered with TLS. Turn TLS on for this source." The port expects TLS.
  - "Settings error: ..." The CA file cannot be read, or the host name is not valid.
  - "Refused: the broker does not accept MQTT 3.1.1." textweaver speaks only that version.
- **"Unreachable: cannot connect to HOST:PORT: ..."** The broker could not be reached: "nothing is listening on that port", "no answer in time", or "no route to the host". textweaver keeps trying. Check the host, the port, that the broker runs, and that Tailscale or your network is up. Right after a first failed connection on a plain port, the message may add "If this port expects TLS, turn TLS on for this source."
- **"No messages yet on TOPIC. If this lasts, check that the broker lets this account read it."** A broker may accept a subscription its access list forbids, and then send nothing, so textweaver cannot tell a silent topic from a forbidden one. It says this once, after `quiet_hint_seconds`.
- **"The broker refused the topic TOPIC; it may not let this account read it."** The broker refused the subscription. Check the topic and the account's access.
- **"N messages were left out: they came faster than they could be shown."** The source sent a flood. The log holds what was shown.
- **"A message on TOPIC was left out: it is not text"** or **"too large."** Binary messages and those over 256 KiB are skipped.
- **"The log is full at 20000 entries; newer entries are left out."** Close the source and open it again for a new log.
- **"NAME not opened: no password."** You cancelled the password prompt.
- **"Live sources are not in this version of textweaver."** This build leaves out MQTT. A saved JSON log still opens.

## See also

- [Running the broker for a live log](live-broker.md): Mosquitto on Debian, Tailscale, 44Net, and example messages.
- [The textweaver app](gui.md#a-live-documents-buttons-and-status-bar): the buttons and the status bar.
- [Keyboard reference](keyboard.md): every key.
- [Settings reference](settings-reference.md): the `[live]` settings with their defaults.
- [Using textweaver with a screen reader](screen-readers.md): Braille display settings.
- [Known limits](known-limits.md#live-documents).
- [Privacy](privacy.md).
