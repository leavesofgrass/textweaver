# Running the broker for a live log

This page is for the person who runs the MQTT broker that a textweaver reader connects to: for example, a friend who logs check-ins during a net and publishes them for others to follow. It is technical, and it assumes you are comfortable at a command line. The reader's side is in [Live documents](live-documents.md).

There are two ways to run the broker:

- **textweaver's own broker**, `tw broker`, described first. It is one command with no configuration file to write by hand: accounts are added with `tw broker account add`, TLS is on by default with a key the broker makes itself, and senders check that key by its pin. `tw publish` publishes to it, or to any MQTT broker.
- **Mosquitto**, the widely used broker, from [What you need](#what-you-need) on. Choose it if you already run one, or need MQTT 5, WebSockets, bridging, or retained messages that survive a restart.

A live source in the textweaver app only subscribes; it never publishes. Sending is a separate, small client, used by `tw publish`, and in the app by Publish a check-in and Publish this document live. The app can also run textweaver's own broker itself, with Start your broker ([Running it from the textweaver app](#running-it-from-the-textweaver-app)).

## textweaver's own broker

`tw broker` runs a small MQTT 3.1.1 broker in the foreground until you press Ctrl+C. It works on Windows, macOS and Linux, as `tw broker` or `textweaver --tw broker`. It is built for a net log or a shared document: a few loggers who publish, and readers who follow along.

What it does, and what it leaves out:

- QoS 0 and 1, retained messages, keepalive, clean sessions, and `+` and `#` in subscriptions. A QoS 2 message is accepted and passed on at QoS 1.
- Named accounts, each a reader or a logger. **A reader can never publish:** if it tries, the message is refused, nobody receives it, and the reader is disconnected. A logger may publish only to its own publish topics, with the same result otherwise. A subscription outside an account's read topics is refused, and the reader is told which topic.
- MQTT 5 clients are refused with the code for an unacceptable protocol version. There is no WebSocket listener and no bridge to other brokers. Retained messages are kept in memory only and are lost when the broker stops. Last will messages are not sent.
- Limits: 200 connections, 256 KiB per message, 1024 bytes per topic, 64 subscriptions per client, 1000 retained topics, and 1024 messages waiting for one reader (a reader that falls further behind is disconnected, and its client reconnects). Five failed logins within a minute from one address lock that address out for five minutes.

### Starting it

First add at least one account (the next section). Then start the broker:

```bash
tw broker
```

It prints what it does, one line each, meaning first:

```text
Starting: textweaver's broker, 2 accounts.
Listening: 127.0.0.1:8883, TLS, loopback only. Other computers cannot connect.
New key: a key and certificate were made in /home/example/.local/share/textweaver.
Certificate pin: ABCD-EFGH-...
Running: press Ctrl+C to stop.
Connected: netcontrol from 127.0.0.1.
```

By default it listens on this computer only. To let others in, name addresses with `--listen`, repeated for more than one: the computer's Tailscale address, a LAN address, or `0.0.0.0` for every network:

```bash
tw broker --listen 100.64.0.10
tw broker --listen 0.0.0.0 --port 8883
```

The port is 8883 with TLS unless `--port` says otherwise, or an address carries its own (`--listen [::1]:9883`). `--home DIR` uses another data folder.

When the computer's firewall asks whether textweaver may accept connections, allow it for the networks your readers use. On Windows, canceling that question blocks the program on every network without saying so; the rule can be changed later in Windows Defender Firewall.

### Running it from the textweaver app

The same broker runs inside the textweaver app: Tools, Live documents, Your broker, **Start your broker**. Its settings are the `[live.broker]` section ([Settings for a source](live-documents.md#settings-for-a-source)): the addresses to listen on (this computer only by default), the port, plain or TLS, whether to make a join code (on by default) and what it may read. The accounts are the same `broker.toml`. The app says how the broker runs and the join code, in groups of four; **Show the join code** says it again and shows it one group to a line; the status bar says "Broker on, 2 clients" while it runs; and **Stop your broker**, or quitting, stops it and frees its ports. **Publish a check-in** and **Publish this document live** publish into it directly, with no network and no login ([Your broker and publishing](live-documents.md#your-broker-and-publishing)).

### Accounts

Accounts are kept in `broker.toml` in textweaver's data folder (`tw settings path` shows the folder). Each has a name, a role, the topics it may read, and for a logger the topics it may publish to. Passwords are never kept, only a salted PBKDF2-HMAC-SHA256 hash with 600,000 rounds.

```bash
tw broker account add netcontrol --role logger --read 'netlog/#' --publish 'netlog/#'
tw broker account add ada --role reader --read 'netlog/#'
tw broker account list
tw broker account remove ada
```

`add` asks for the password twice, without showing or speaking what you type. When standard input is piped, it reads the password as one line instead, so a script can set it; a password is never given on the command line. `--replace` changes an existing account, for example to set a new password. Without `--read`, an account reads every topic (`#`). A logger needs at least one `--publish` topic. `list --json` prints the accounts as JSON, without the hashes.

The broker reads `broker.toml` when it starts, so restart it after changing accounts. An example file with placeholder accounts is [`docs/examples/live/broker.toml`](examples/live/broker.toml).

### The certificate pin

With TLS, the broker makes a key and a self-signed certificate on its first start, and keeps them in the data folder: `broker-key.pem` (readable by its owner only, on Linux and macOS) and `broker-cert.pem`. A sender does not need a certificate authority. It checks the broker's key by its **pin**: the SHA-256 hash of the key in 52 letters and digits, shown in groups of four, which the broker prints after "Certificate pin:". `tw broker pin` prints it without starting the broker.

The pin stays the same across restarts as long as the two files stay. If they are moved away, the broker makes a new pair, and everyone who checks the pin needs the new one.

`tw publish` takes the pin with `--pin`. A reader's live source takes it as the encryption **Pinned key** in the live source form, or `tls = "pinned"` with `pin = "..."` in `settings.toml` or a `.twlive` file you send; a pin is not a secret. Names and dates in the certificate are not checked, the key is, so the same pin works from any computer and through Tailscale.

### A join code for a quick session

For a one-off session, such as a class or a meeting, start the broker with a join code:

```bash
tw broker --listen 0.0.0.0 --join-code --join-topic 'docs/#'
```

It prints a code of 16 characters in four groups of four, such as `ABCD-EFGH-JKMN-PQRS`, to read aloud or show, and on its own line the join link to copy to readers, such as `Join link: twjoin://192.168.1.20:8883/ABCD-EFGH-JKMN-PQRS`. With `--listen 0.0.0.0`, the link names this computer's address on its main network. `--qr FILE` also saves the link as a QR code for a phone or another computer to scan: a PNG, or an SVG when the name ends in `.svg`, named "QR code for joining HOST" for screen readers. It prints "QR code saved: FILE", and textweaver keeps no other copy; anyone who sees the code can join. `tw join-qr IMAGE` reads a join link back out of a PNG or JPEG image, such as a photo of the code, and prints it as "Join link: ..."; with no join link in the image it prints one line saying why ("No join link: no QR code was found in the image.") and ends with exit status 1. The session gets a key of its own, held in memory only, so the code works only until the broker stops. The first 12 characters are the key's short pin and the last 4 are a secret. The login password is derived from the whole code, so someone who reaches the port and reads the certificate still cannot log in. Anyone with the code may read the `--join-topic` filters (every topic, by default) and may never publish. Accounts still work beside the code: a logger checks the session's key with the code's pin (`tw publish --check-in --join-code CODE --user netcontrol`).

A reader in the textweaver app joins with the encryption **Join code** in the live source form, typing the code or pasting the join link into any field; textweaver keeps the password the code gives in the credential store and the code nowhere ([The form](live-documents.md#the-form)). Typing a code forgives case, spaces and dashes, and reads the letter O as zero and I or L as one. A wrong character is named with its group: "U in group 3 is not used in join codes." A join code needs TLS, so `--join-code` cannot be combined with `--plain`.

### Plain connections and Tailscale

`--plain` turns TLS off, and the port becomes 1883. For each address the broker says what that means:

- Loopback: "Listening: 127.0.0.1:1883, plain, loopback only. Other computers cannot connect."
- Tailscale (100.64.0.0/10): "Listening: 100.64.0.10:1883, plain, on Tailscale, which encrypts the link."
- Anything else: "Warning: 192.168.1.20:1883 is plain and not on Tailscale. Anyone on that network can read the messages and the passwords."

Inside a tailnet, plain MQTT is protected by Tailscale's WireGuard tunnel, as [Reaching the broker over Tailscale](#reaching-the-broker-over-tailscale) explains.

### Publishing a check-in: tw publish --check-in

`tw publish --check-in` publishes one entry, a JSON object, to a topic, and waits for the broker to acknowledge it (QoS 1). The fields are sent in the order given:

```bash
tw publish --check-in --host 100.64.0.10 --pin ABCD-EFGH-... --user netcontrol --topic netlog/checkins --field callsign=N0CALL --field 'name=Ada Example' --field 'notes=Mobile, portable antenna'
```

With no `--field`, it reads one JSON object from standard input:

```bash
echo '{"callsign": "N0CALL", "name": "Ada Example"}' | tw publish --check-in --plain --user netcontrol --topic netlog/checkins
```

How to reach the broker, for a check-in and a document alike:

- `--host` and `--port`: the broker. By default 127.0.0.1, on the port that goes with the encryption.
- `--pin PIN`: TLS, trusting exactly the key with this pin. `--join-code CODE`: the same with a join code's short pin, and without `--user`, the code's own login. `--join-link LINK`: the host, the port and the join code in one, from a join link. With `--join-code` or `--join-link`, `tw publish FILE` prints the join link readers follow the document with ("Join link: ...") after the first revision, and `--qr FILE` saves it as a QR code as `tw broker --qr` does. `--ca-file FILE`: TLS checked against a private certificate authority or the broker's own certificate. `--plain`: no TLS. With none of these, TLS is checked against the bundled Mozilla root certificates, as for a public broker.
- `--user NAME`: the password is asked without echo, or read from the environment variable `TEXTWEAVER_MQTT_PASSWORD` when it is set, for scripts. It is never given on the command line.

The entry is not retained unless `--retain` is given. `tw publish --check-in` prints "Published: one entry to netlog/checkins on 100.64.0.10:8883." When the broker refuses, it says why, for example: "Not published: Refused: the broker closed the connection instead of taking the message. This account may not publish to netlog/checkins."

### Publishing a document: tw publish

`tw publish FILE` publishes a Markdown or text file as a live document, then again each time the file is saved, until Ctrl+C:

```bash
tw publish agenda.md --topic docs/agenda --plain --user netcontrol
```

Each revision is one retained message on the topic, so a reader who joins late receives the current text at once. `--title` sets the title readers see (the file's name, by default), `--once` publishes once and stops, and `--every` sets the seconds between looks at the file (1). Each revision prints one line, such as "Published: revision 2, 4096 bytes." A save that cannot be published is said in words, and the next save tries again. When publishing stops, readers keep the last revision.

`--field` and `--retain` go only with `--check-in`, and `--title`, `--once`, `--every` and `--qr` only with a document. The names used earlier in beta 5 still work, with the same options and output, for scripts written with them: `tw share FILE` is `tw publish FILE`, and `tw send` is `tw publish --check-in`. They are left out of `tw --help`; `tw publish --help` names them.

### The live document message

A shared document is one JSON object, retained on its topic and capped at 256 KiB like any message:

```json
{"type": "textweaver.document", "rev": 3, "title": "Agenda", "format": "markdown", "text": "# Agenda\n\n1. Check-ins\n2. Traffic"}
```

- `type` is always `textweaver.document`, which tells it apart from a log entry.
- `rev` counts revisions from 1. A reader can ignore a revision older than the one it shows.
- `format` is `markdown` (for `.md` and `.markdown` files) or `text`.
- `text` is the whole document, every time.

A net log is one kind of live document, and its entries stay plain JSON objects, as in [Example messages](#example-messages). A reader's textweaver shows a live document message in its document view: the document itself, replaced in place by each new revision, keeping the reader's position ([The document view](live-documents.md#the-document-view)). The `text` keeps its line breaks and may hold up to 250,000 characters, where any other value is one line of at most 2,000; it is still cleaned of control characters, terminal escape sequences and text-direction overrides.

### A net log, ready to copy

The folder [`docs/examples/live/`](examples/live/) holds a complete example with placeholder names: `broker.toml` with a logger and a reader account, `netlog.twlive`, a live source file for readers, and `publish-checkin.sh`, a sample `tw publish --check-in` line. The example accounts' passwords are written in the file's comments, so make your own accounts before a real net.

### Testing it with Mosquitto's clients

`mosquitto_sub` and `mosquitto_pub` work with textweaver's broker. On the broker's own computer, with TLS, give the broker's certificate as the CA file:

```bash
mosquitto_sub -h localhost -p 8883 --cafile broker-cert.pem -u ada -P READERPASSWORD -t 'netlog/#' -v
mosquitto_pub -h localhost -p 8883 --cafile broker-cert.pem -u netcontrol -P LOGGERPASSWORD -t netlog/checkins -q 1 -m '{"callsign": "N0CALL"}'
```

From another computer the certificate's names do not match, so test against a plain listener across Tailscale. The [checks before the net](#checks-before-the-net) apply to textweaver's broker too. A reader that publishes is disconnected, and the broker prints a line starting "Publish refused:".

The rest of this page is about Mosquitto.

## What you need

- A computer that stays on during the net, running Debian 13 (trixie) or similar.
- Mosquitto, the broker. Debian 13 packages version 2.0.21, which speaks MQTT 5.0, 3.1.1, and 3.1. textweaver uses MQTT 3.1.1, which Mosquitto accepts with no setting.
- A way for the reader to reach the computer: Tailscale or 44Net (below), or an ordinary public address.
- An account for the publisher, and a read-only account for each reader.

Install the broker and the test programs:

```bash
sudo apt install mosquitto mosquitto-clients
```

The package starts the broker at once. Its main file, `/etc/mosquitto/mosquitto.conf`, tells you to place your own settings in `/etc/mosquitto/conf.d/`, and it reads every `.conf` file there. Leave the main file alone and add a file of your own.

## How Mosquitto 2 behaves by default

- With no listener configured, the broker listens on port 1883 on the local computer only, and accepts anonymous clients from there only.
- As soon as your configuration defines a listener, anonymous clients are refused unless you say otherwise. In Mosquitto 2 you must choose your authentication explicitly before anyone can connect.
- A listener you define replaces the default one. If you also want to test from the same computer, add `listener 1883 127.0.0.1`.

This default is good: the broker is closed until you open it. The steps below open it for named accounts only.

## Accounts

Make a password file with an account for the publisher and one for readers. `mosquitto_passwd` prompts for each password, so it does not land in your shell history:

```bash
sudo mosquitto_passwd -c /etc/mosquitto/passwd logger
sudo mosquitto_passwd /etc/mosquitto/passwd reader
```

`-c` creates the file and overwrites an existing one, so use it only for the first account. Avoid the `-b` option, which puts the password on the command line. The default hash is argon2id.

Make the file readable by the broker and nobody else:

```bash
sudo chown mosquitto:mosquitto /etc/mosquitto/passwd
sudo chmod 640 /etc/mosquitto/passwd
```

## Read-only access

An access list file gives each account only what it needs. Create `/etc/mosquitto/acl`:

```text
user logger
topic readwrite netlog/#

user reader
topic read netlog/#
```

With an access list in place, an account may use only the topics listed for it. The reader can subscribe to anything under `netlog/` and cannot publish. Give the file the same owner and mode as the password file.

If you serve several readers, one shared `reader` account is simplest. Separate accounts let you stop one reader without changing the others.

## Reaching the broker over Tailscale

[Tailscale](https://tailscale.com/) connects your computers into a private network, a tailnet, over WireGuard. Every connection in it is encrypted end to end, whether direct or relayed, so plain MQTT on port 1883 is acceptable inside a tailnet. The passwords and the log are protected on the wire by Tailscale.

Add `/etc/mosquitto/conf.d/netlog.conf`, with the broker's own Tailscale address (the example is a placeholder):

```text
listener 1883 100.64.0.10
allow_anonymous false
password_file /etc/mosquitto/passwd
acl_file /etc/mosquitto/acl
```

Then restart the broker and check that it came up:

```bash
sudo systemctl restart mosquitto
sudo systemctl status mosquitto
```

Tailscale addresses are in 100.64.0.0 to 100.127.255.255, and a computer keeps its address across networks. If Mosquitto starts before Tailscale has brought its network up, it may fail to bind the address; restart it after Tailscale is up. This is an inference from how binding works, so test it after a reboot.

### Sharing the broker's computer with a friend

Each person in a tailnet is a user of it. A friend who is not in your tailnet can still reach one computer if you share it:

- You need to be an owner, admin, or IT admin of your tailnet to share, and the friend must be one in theirs to accept. Share by email, or with a link that you treat like a password.
- The friend sees only that one computer, not the rest of your tailnet.
- A shared computer accepts connections but cannot start them into the friend's tailnet, which suits a broker.
- The friend must use the computer's full name, in the form `machine.tailnet-name.ts.net`. The short name works only inside your own tailnet.
- Revoking the share ends the access.

To allow only the broker's port, use an access rule in your tailnet policy that accepts the shared users to that one port, for example the source `autogroup:shared` and the destination `100.64.0.10:1883`. Tailscale's rules are deny by default once an `acls` section exists. This rule is assembled from Tailscale's documentation and has not been tested as one rule, so test it. Avoid a source of `*`, which includes shared users.

textweaver recognizes a Tailscale address (100.64.0.0/10 and fd7a:115c:a1e0::/48) or a name ending in `.ts.net`, and says "Connected over Tailscale, which encrypts the link" instead of warning that the link is not encrypted.

## Reaching the broker over 44Net

44Net (AMPRNet) is the network of amateur radio addresses in 44.0.0.0/9 and 44.128.0.0/10. You get an address through the portal at portal.ampr.org, and 44Net Connect offers a WireGuard-based way to use one behind a home router. These are public addresses. The WireGuard tunnel of 44Net Connect protects only the stretch between your computer and the service's gateway; between your broker and a reader, packets can cross the open internet.

So prefer TLS on 44Net. A password and a log sent in the clear can be read on the way. Use the TLS setup below, on port 8883, and keep `allow_anonymous false` and the access list, because a public address can be reached by anyone.

## TLS on port 8883 with a private certificate authority

A private certificate authority (CA) lets you issue the broker's certificate without paying for one. The reader needs only the CA's public certificate, `ca.crt`.

Make the authority, and a certificate for the broker. Replace the names and the address with the ones a reader will type as the host. The certificate must name exactly that, or the reader refuses it.

```bash
openssl genrsa -out ca.key 4096
openssl req -x509 -new -key ca.key -sha256 -days 1825 -subj "/CN=Example Net CA" -out ca.crt

openssl genrsa -out server.key 2048
openssl req -new -key server.key -subj "/CN=broker.example.org" -out server.csr
echo "subjectAltName = DNS:broker.example.org, IP:203.0.113.10" > san.ext
openssl x509 -req -in server.csr -CA ca.crt -CAkey ca.key -CAcreateserial -days 825 -sha256 -extfile san.ext -out server.crt
```

Keep `ca.key` somewhere safe and off the broker; anyone who holds it can issue certificates that readers trust. Give readers `ca.crt` only. Copy `ca.crt`, `server.crt`, and `server.key` into `/etc/mosquitto/certs/`, owned by `mosquitto` with `server.key` readable by that account alone.

Then use this drop-in instead of the Tailscale one (the commands above were tried with OpenSSL 3.5, which Debian 13 also ships):

```text
listener 8883 203.0.113.10
allow_anonymous false
password_file /etc/mosquitto/passwd
acl_file /etc/mosquitto/acl
cafile /etc/mosquitto/certs/ca.crt
certfile /etc/mosquitto/certs/server.crt
keyfile /etc/mosquitto/certs/server.key
```

The three certificate lines belong to the listener above them, so keep them below it. Both `certfile` and `keyfile` are needed to turn TLS on. The default minimum is TLS 1.2, and 1.3 is allowed.

The reader sets `tls = "ca_file"` and `ca_file` to the path of `ca.crt`. A certificate from a public authority, such as Let's Encrypt, works with `tls = "system"`, which trusts the bundled Mozilla list of authorities.

## Testing with mosquitto_sub and mosquitto_pub

Test from another computer if you can, since that is what a reader does. In one terminal, subscribe as the reader and print each topic with its message:

```bash
mosquitto_sub -h broker.example.ts.net -p 1883 -u reader -P READERPASSWORD -t 'netlog/#' -v
```

For TLS, add `--cafile ca.crt` and use port 8883. In another terminal, publish as the logger:

```bash
mosquitto_pub -h broker.example.ts.net -p 1883 -u logger -P LOGGERPASSWORD -t netlog/checkins -q 1 -m '{"call": "N0CALL", "name": "Ada Example"}'
```

The subscriber should print the topic and the message. Then test that the access list holds. Publish as `reader`, and watch the first terminal: nothing should arrive. MQTT 3.1.1 gives a publisher no error for a denied message, so the check is that nobody receives it.

Useful options: `-r` retains a message, so a later subscriber gets it at once, and an empty retained message clears it. `-C 5` makes the subscriber exit after five messages, and `-W 30` after 30 seconds. `-P` puts a password on the command line, where other users of the computer can see it in the process list; for a real password, start the command with a space (Debian's default Bash then leaves it out of the history) or use a throwaway password for tests.

If the subscriber sees nothing, look in `/var/log/mosquitto/mosquitto.log`, where the packaged configuration writes the broker's log.

## Example messages

All names below are placeholders.

**One check-in** is one JSON object. Publish it to `netlog/checkins`. Each new one adds an entry at the end of the reader's log:

```json
{"id": 7, "call": "N0CALL", "name": "Ada Example", "location": "Portland, OR", "status": "mobile", "notes": "Portable antenna", "time": "2026-10-26T20:10:00Z"}
```

The reader sees: "N0CALL, Ada Example. Portland, OR. mobile. Portable antenna. Time: 2026-10-26T20:10:00Z."

The smallest useful check-in has a callsign, a name, and notes:

```json
{"callsign": "N0CALL", "name": "Ada Example", "notes": "Mobile, portable antenna"}
```

**The whole log as a list.** Publish with retain, so a reader who joins late receives it at once. It replaces the reader's log and keeps them on the same entry:

```json
[
  {"id": 1, "call": "N0CALL", "name": "Ada Example", "notes": "Net control", "time": "2026-10-26T20:00:00Z"},
  {"id": 2, "call": "N1TEST", "name": "Grace Sample", "notes": "Short time", "time": "2026-10-26T20:02:00Z"}
]
```

**A wrapper object** adds details about the net. A list under `checkins`, `entries`, `log`, `stations`, `items`, or `records` is the whole log, and the object's plain values become the heading line above the entries ("Example Net. freq: 147.040."):

```json
{"net": "Example Net", "freq": "147.040", "checkins": [{"call": "N0CALL", "name": "Ada Example"}]}
```

**A correction** is a message with the same id as an earlier entry. It replaces that entry in place:

```json
{"id": 2, "call": "N1TEST", "name": "Grace Sample", "notes": "Traffic, one message"}
```

A message with no id is matched by its callsign instead, so a second message from the same callsign is a correction too. A logger that records one station twice should send an id.

**Removing an entry.** textweaver has no delete message. To remove a check-in, publish the whole log again, as a list, without it. The list replaces what the reader has.

### A choice for the logger

A reader who joins in the middle of the net receives only retained messages. There are two sensible plans:

- **Retain the whole list.** After every change, publish the complete log as a retained list to one topic, for example `netlog/log`. Late readers get everything at once, and an earlier retained list is replaced by the next.
- **Send each check-in, not retained.** Readers who are connected from the start see every entry. A late reader sees only what arrives after joining.

The list plan is simpler for readers. Do not retain a single check-in on a topic where you also retain a list, since a late reader would receive the old retained check-in as an entry of its own.

## A live source file for your readers

Rather than read out a host, a port, and a topic for each reader to type, you can send them one small file. A live source file is TOML with the extension `.twlive`, holding the keys of one source. A reader opens it in textweaver (File, Open, or `tw netlog.twlive`), checks the form it fills, and presses Enter to add the source. The keys are the ones a reader's `settings.toml` uses, listed in [Live documents](live-documents.md#settings-for-a-source).

For the plain Tailscale setup above:

```toml
# A textweaver live source. Open this file in textweaver to add it.
name = "Practice net"
host = "broker.example.ts.net"
username = "reader"
topics = ["netlog/#"]
id_field = "serial"
```

For TLS with your own certificate authority, send `ca.crt` beside the file and name it without a folder; textweaver reads a relative `ca_file` from the folder the `.twlive` file is in:

```toml
name = "Practice net"
host = "broker.example.org"
tls = "ca_file"
ca_file = "ca.crt"
topics = ["netlog/#"]
username = "reader"
```

Never put a password in the file. textweaver refuses a `password` key, says so to the reader, and asks for the password after the source is saved, keeping it in the reader's system credential store. Send the password some other way, or let each reader have an account of their own.

You do not have to write the file by hand. Set the source up once in your own textweaver, then run **Save live source as file** (Tools, Live documents): it writes the source without its password, ready to send.

## Checks before the net

1. From another computer, subscribe as `reader` and publish as `logger`. The message arrives.
2. Publish as `reader`. Nothing arrives.
3. Subscribe with a wrong password. The login is refused.
4. Restart the broker and check that it comes up with the network, then check it again after a reboot.
5. With a retained list in place, open the source in textweaver and read the log.
6. If you send readers a live source file, open it yourself first in a textweaver with no sources, and check that the form it fills connects.

## See also

- [Live documents](live-documents.md): what the reader does with the messages.
- [Privacy](privacy.md).
