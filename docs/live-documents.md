# Live documents

A live document is a document that grows while you read it. textweaver connects to an MQTT broker, subscribes to one or more topics, and shows what arrives as a document: a log with one entry per message, or a list of values with one line per field. You read it like any other document, with the same keys, headings, tables, find, and bookmarks, on the screen, on a Braille display through your screen reader, and by speech if you ask for it.

MQTT is a small publish and subscribe protocol that sensors, home automation, and logging tools use. Any standard MQTT broker and any topic can be opened as a live document. A net log is the first use: a friend logs check-ins as JSON and publishes them, and textweaver shows the log as it grows. A weather station or a workshop sensor is another.

A live source only receives. It never publishes a message, so a live document cannot change what the broker holds. Sending is separate: **Send a check-in** publishes one entry, **Share this document live** publishes the document you have open, and **Start a broker** runs textweaver's own broker inside the app ([Sending: check-ins and sharing](#sending-check-ins-and-sharing)).

This guide is for the person who reads the live document. If you also run the broker, [Running the broker for a live log](live-broker.md) covers textweaver's own broker (`tw broker`, `tw send` and `tw share`), Mosquitto, Tailscale, and 44Net.

## In brief

1. Run **Set up a live source** (Tools, Live sources, or the command palette) and fill in its form: a name, the broker's host, encryption, port, topic, user name, and view. If the broker's owner sent you a live source file (`.twlive`), open it instead, check the form it fills, and press Enter.
2. Run **Open live source**, choose the source, and press Enter. If it needs a password, textweaver asks once and keeps it in your system's credential store.
3. Read the log. New entries arrive at the end and never move your place. Press `Alt+Shift+F6` to jump to the newest entry. A source that carries a shared document shows the document itself instead, and each new revision replaces it in place ([The document view](#the-document-view)).
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

## The document view

A shared document is a whole document published as one message, by `tw share` or by **Share this document live** in another copy of textweaver ([The live document message](live-broker.md#the-live-document-message)). When such a message arrives, the source shows the document itself, as Markdown or as plain text, in place of a log. You read it as any document, with headings, lists, links and find.

- **A new revision replaces it in place.** textweaver changes only the lines that differ, so your cursor, selection, bookmarks and the screen reader's place stay on the same text. In the textweaver app the lines that did not change keep their place in the accessibility tree, so the screen reader does not read them again.
- **The revision is said once, briefly and politely:** "Agenda updated, revision 3." The text itself is never read out. The first revision says "Agenda, revision 1, shared live." The title line shows "Revision 3" where a log shows its unread count.
- **Line breaks are kept.** Every other value in a message is one line of at most 2,000 characters; a shared document keeps its lines and may hold up to 250,000 characters. It is cleaned all the same: control characters, terminal escape sequences, and text-direction overrides are removed. A longer document ends with a line saying "(cut)".
- **An older revision is left out,** and the same revision again changes nothing. Since the message is retained, a reader who opens the source late receives the current revision at once.
- **Pause updates** holds new revisions; resuming shows the newest one.
- **One document per source.** Log entries that arrive on a source showing a document are left out, and textweaver says so once. The view switch, Jump to newest and Save live log answer that a shared document has no entries, table or log.

## Setting up a source

### The form

**Set up a live source** (Tools, Live sources, or the command palette) opens one form. It works like the form for a document's details: each field is one labeled line, and moving to a field says its label, its place, and its value ("Broker host, 2 of 7", then "Broker host: blank"). The fields, in order:

1. **Name**, such as "Practice log".
2. **Broker host**: the broker's host name or address, with no spaces.
3. **Encryption**, a choice: **Off** (not encrypted, port 1883, fine over Tailscale), **Bundled roots** (TLS checked against the certificate authorities that come with textweaver, port 8883), **CA file** (TLS checked against a certificate authority file you name, port 8883), **Pinned key** (TLS trusting only the broker key whose pin you type, port 8883), or **Join code** (TLS with the key a join code names, signing in with the code, port 8883).
4. **CA file**: the certificate authority's PEM file. This field is there only when the encryption is CA file.
5. **Pin**: the broker's certificate pin, as textweaver's broker prints it after "Certificate pin:", 52 letters and digits in groups of four. This field is there only with a pinned key.
6. **Join code**: the 16 characters the broker's owner read out or showed you. This field is there only when joining by code. It is masked like a password: one star per character, nothing you type is spoken back, and moving to it says how many characters it holds rather than the code. `F3` shows the code as typed, to check it, and `F3` again hides it ([The password](#the-password)).
7. **Port**. It follows the encryption, 1883 or 8883, until you change it; a port you typed stays when the encryption changes.
8. **Topic**, such as `log/checkins`. Separate several topics with commas. MQTT's wildcards work: `+` for one level and `#` for the rest.
9. **User name**. Leave it blank if the broker needs none. A join code signs in by itself, so this field is not there when joining by code.
10. **View**, a choice: **Log** (one entry per message) or **Values** (one line per field, updated in place).

A pin and a join code are typed as they are read aloud: in any case, with or without spaces, dashes, commas or line breaks, and the letter O reads as zero and I or L as one. A pin or code with a wrong length or a character it cannot hold is said in words, and the form opens again on that field.

**Pasting a join link or code.** The broker's owner can copy a join link (Copy the join link, or the "Join link:" line that `tw broker --join-code` prints) and send it to you. A join link is one line, such as `twjoin://192.168.1.20:8883/ABCD-EFGH-JKMN-PQRS?topic=textweaver/document`: the broker's address and port, the join code, and the topic of a shared document. Paste it into any field of the form, and textweaver fills the broker host, the port, the encryption (Join code), the join code and the topic, and the name with the host when the name is empty. It says what it filled ("Filled from the join link: host, port, code and topic."), and the field you pasted into keeps what it held unless the link fills it. When the link names no topic, the form moves to the topic field for you to type one. A form filled from a link joins on `Enter`: it saves the source and opens it at once, with no "Open it now?" question. Pasting into the join code field takes the code out of whatever came with it: the code split across lines, a "Join code:" label, quotes, the "Show the join code" list as copied, or a whole link; textweaver says "Join code pasted." A link that is not one is said in words, such as "Not a join link: the port is not a number from 1 to 65535.", and nothing from it is kept.

**Open a join link** (Tools, Live sources, or the command palette) opens the form filled from the join link on the clipboard. With no link there, it opens the form empty, ready for one to be pasted. A join link holds the join code, secret part included, so treat it as you would the code: textweaver never says it, keeps it in a prompt's history, saves it, or shows it in the status bar, and keeps only what a typed code keeps.

**Joining by code** keeps nothing secret in the settings. textweaver saves the source as a pinned key, with the first 12 characters of the code as its pin and `join` as its user name, and keeps the password the whole code gives in your system's credential store, without asking for one. The code itself is kept nowhere. Editing such a source shows the join code field empty: leave it empty to keep the kept password, or type a new code. A join code reads only; textweaver's broker refuses anything it tries to publish.

The keys:

- `Tab` and `Shift+Tab` move to the next and previous field, keeping what you typed. They wrap from the last field to the first.
- `Up Arrow` and `Down Arrow` change a choice in place, Encryption or View, and say the new choice with what it means. You can also type a choice's name. On the other fields, they bring back earlier answers, as in any prompt.
- `Enter` checks every field and saves. If a field is wrong, such as a host with a space in it, a missing topic, a CA file that does not exist, or a port outside 1 to 65535, textweaver says what is wrong in words and opens the form again on that field. Every other value stays as you left it.
- `F3`, in the join code field, shows the code as typed or hides it again, and says which.
- `Escape` cancels the form and says so. Nothing is saved.

textweaver writes the source to `settings.toml`. If it has a user name, textweaver then asks for the password in a masked prompt ([The password](#the-password)). Last, it offers to open the source now or later. The source's id comes from its name, in lowercase with hyphens between words ("Practice log" becomes `practice-log`), and a number is added if that id is taken. An id may hold only letters, digits, hyphens, and underscores. If another source already has the same name, textweaver says the new one was saved beside it.

**Edit a live source** (Tools, Live sources) opens the same form, filled in with the open source, or with one you choose from a list. In the Open live source list, `F2` edits the source on the focused row, as `F2` edits a document's details in the library; `Escape` then brings the list back on the same row. Editing keeps everything the form does not show, such as the field mapping and `id_field`, and keeps the source's id, so its kept password still applies unless you change the user name.

### A live source file

A live source file is a small TOML file, with the extension `.twlive`, that holds one source. The broker's owner can send you one, so you do not type the host and topic yourself ([Running the broker for a live log](live-broker.md#a-live-source-file-for-your-readers)). Open it as you open any document: File, Open, the command line (`tw practice.twlive`), or `tw open`. textweaver does not show it as a document. It opens the live source form, filled in from the file, for you to check. `Enter` saves it as a new source, with an id made from its name; it never replaces a source you already have. Then textweaver asks for the password, if the source has a user name.

The file holds the keys of one source, the ones described under [Settings for a source](#settings-for-a-source), at the top level and with no id:

```toml
# A textweaver live source. Open this file in textweaver to add it.
name = "Practice log"
host = "logger.example.ts.net"
username = "reader"
topics = ["log/checkins"]
view = "log"
id_field = "serial"

[[fields]]
path = "call"
label = "Callsign"

[[fields]]
path = "name"
label = "Name"
```

A file comes from someone else, so textweaver reads it with care, and tells you what it left out before the form's first field:

- A file larger than 64 KiB, or one that is not text or not valid TOML, is refused in words, with the line where reading stopped.
- A key textweaver does not know is named and left out.
- A `password` key is never read. textweaver says so: a password is only ever kept in your system's credential store, and textweaver asks for it after saving. A `join_code` key is refused the same way, since a code's last four characters are a secret. A broker's `pin` is kept: it is not a secret, and it lets the file name a broker by its key.
- A value that does not fit its key is left out in words, as in `settings.toml`.
- Control characters, terminal escape sequences, and characters that change the text's direction are removed from every value. Very long values and very long lists are cut short.
- A relative `ca_file`, such as `ca_file = "club-ca.pem"`, is the file of that name in the same folder as the `.twlive` file. Send the two together.

**Save live source as file** (Tools, Live sources) writes the open source, or one you choose, to a `.twlive` file, to share or keep. It is offered as `<id>.twlive`; with no extension typed, `.twlive` is added. The file never holds a password, and leaves out keys only this computer knows about. A CA file in the same folder is written by its name alone.

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
- `tls`: `off`, `system`, `ca_file`, or `pinned`. The value `system` trusts the certificate authorities bundled with textweaver (Mozilla's list), not your operating system's store. With `ca_file`, textweaver trusts only the authority in that file. With `pinned`, it trusts exactly the key whose pin is `pin`, with no certificate authority; the names and dates in the certificate are not checked, the key is. There is no way to turn off certificate checking.
- `ca_file`: the path of a PEM file, with `tls = "ca_file"`. Use a full path in `settings.toml`; in a live source file, a relative path is read from the file's own folder.
- `pin`: with `tls = "pinned"`, the broker's certificate pin: 52 characters as textweaver's broker prints it, or the 12 of a join code, in groups of four. A source with a pin that does not read as one is not opened, and textweaver says so.
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

The `[live.broker]` section holds the broker the app starts and where the app sends ([Sending: check-ins and sharing](#sending-check-ins-and-sharing)):

- `listen`, default `["127.0.0.1"]`: the addresses the broker listens on, each with or without a port, such as `100.101.1.2` or `[::1]:8883`. The default is this computer only.
- `port`, default `0`: the port for addresses given without one. Zero means 8883 with TLS and 1883 without.
- `plain`, default `false`: no TLS. Only on this computer or over Tailscale.
- `join_code`, default `true`: a join code each time the broker starts. The code comes with a key of its own, held in memory, so readers who pinned the broker's usual key need it off.
- `join_topics`, default `["#"]`: the topic filters a join code may read.
- `share_topic`, default `"textweaver/document"`: the topic Share this document live publishes on.
- `checkin_topic`, default `"netlog/checkins"`: the topic Send a check-in publishes on when the live source has no plain topic.

Sources, the log folder, and `[live.broker]` belong to this computer, and never sync. The announcement choices in `[live]` do sync.

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
- The masked field edits like any text field: you can move inside it, type or delete in the middle, and use an asterisk in the password. In the textweaver app it is a password field, so a screen reader says "star" or "protected" rather than echo each character.
- **Show** reveals what you typed, to check it with speech or on a Braille display. In the terminal reader press `F3`, which the prompt names as it opens ("F3 shows it."); press it again to hide the text. In the textweaver app the **Show the password** check box follows the field (`Tab` from the field reaches it), and `F3` works there too. textweaver says "Showing the password" or "Hiding the password", and the text and the caret stay where they were. While shown, the field is an ordinary text field: the screen reader reads it and typed characters are spoken. Show is off each time the prompt opens and is never saved, and nothing typed is kept in the prompt's history or written to the app's log, shown or not. The same choice is in the join code field of the live source form and in the GitHub token prompt.

## Sending: check-ins and sharing

These commands are in Tools, Live sources and in the command palette. They publish; a live source never does.

### Start a broker

**Start a broker** runs textweaver's own broker inside the app, the same broker as `tw broker` ([textweaver's own broker](live-broker.md#textweavers-own-broker)), as `[live.broker]` says. With the defaults it listens on this computer only, with TLS on port 8883, and makes a join code. It says how it runs, meaning first: "Broker on: 127.0.0.1:8883, TLS, this computer only." Then it says the join code in its groups of four: "Join code: ABCD, EFGH, JKMN, PQRS." The accounts are those in `broker.toml`, which `tw broker account add` keeps.

- **Show the join code** says the code again and shows it in a list: the whole code first, then one group to a line ("Group 1 of 4: ABCD"), so it can be read out one group at a time from a Braille display, then two buttons, **Copy the join code** and **Copy the join link**, as the list's last rows: `Enter` on one copies. With no join code, the list holds the certificate pin, which readers pin instead.
- **Copy the join code** puts the code on the clipboard in its groups of four, and **Copy the join link** puts the join link there: one line with the broker's address, its port, the join code, and, while a document is shared into the broker, its topic. Each says so in words ("Join link copied.") and never says the code or the link. Both are in Tools, Live sources, in the command palette, and in the context menu (`Shift+F10` or the Applications key) while the broker has a join code. The link names the first address the broker listens on that is not this computer's loopback address; for every address (`0.0.0.0`), the computer's own address on its main network. Copy the link after Share this document live to include the topic.
- **The status bar** says "Broker on, 2 clients" while it runs, and "Sharing, revision 3" while a document is shared. A reader joining or leaving is said briefly ("Joined: join."), and a refused sign-in or a refused publish is said in words.
- **Stop the broker** ends every connection and frees every port before it answers. Quitting textweaver stops it too.
- A broker without TLS on a network other than this computer or Tailscale is warned about in words, since anyone on that network could read the messages and passwords.

### Send a check-in

**Send a check-in** opens a form with one field per line, like the live source form: `Tab` and `Shift+Tab` move between fields, `Enter` sends, `Escape` cancels. The fields are the open source's field mapping, in its order and with its labels; a source with none, or no source, gets callsign, name and notes. A net log is one example of a live document: the entry is a plain JSON object whose keys are the fields' paths, in the form's order, and empty fields are left out. An entry with every field empty is said, and the form opens again.

The entry goes into the broker the app runs, when one is running. Otherwise it goes to the open live source's broker, signed in as that source, or through a source chosen from a list. The topic is the source's first plain topic (one without `+` or `#`), else `checkin_topic`. textweaver says "Sent: N0CALL, on netlog/checkins." once the broker acknowledges it. A reader's account, or a join code, cannot publish: the broker refuses, and textweaver says "Check-in not sent:" with the reason.

### Share this document live

**Share this document live** publishes the document you have open as a live document message, retained on `share_topic`, so readers following that topic see it at once, in the document view. Each time you save it in edit mode, the new text goes out as the next revision. A Markdown or text file is shared as its file reads; any other document as its text. **Stop sharing** stops; readers keep the last revision. The document goes into the broker the app runs, or, with none running, through a live source you choose, signed in as that source.

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

Every other command is in Tools, Live sources and in the command palette (F2): Open live source, Set up a live source, Open a join link, Edit a live source, Close live source, Speak new entries, Show changes of this field, Speak changes of this field, Save live log now, Save live source as file, Forget live source password, Start a broker, Stop the broker, Show the join code, Copy the join code, Copy the join link, Share this document live, Stop sharing, and Send a check-in. In the live source form, `Tab` and `Shift+Tab` move between fields and `Up Arrow` and `Down Arrow` change a choice; in the Open live source list, `F2` edits the focused source. Close live source stops receiving; the log stays open, read only, and is saved. [The keyboard reference](keyboard.md) lists every key, and you can give any of these commands a key in `keymap.toml`.

In the textweaver app, a live document's buttons join the toolbar while it is open. [The textweaver app](gui.md#a-live-documents-buttons-and-status-bar) lists them and describes the status bar, which shows the unread count and the connection in words.

## Privacy

- **A broker sees your connection.** It sees your computer's network address, the user name you sign in with, the topics you subscribe to, and when you connect. If the link is not encrypted, anyone on the path between you can also read the user name, the password, and every message. Over Tailscale the link is encrypted by Tailscale. Over the open internet, use TLS.
- **The client id is neutral.** textweaver connects as `tw-` and 16 random hexadecimal digits, new at each start. It sends no name, computer name, or account name.
- **A live source sends nothing else.** It publishes no message. It subscribes, answers the broker's acknowledgements, and keeps the connection alive. Only Send a check-in and Share this document live publish, and only what you send.
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
