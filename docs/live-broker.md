# Running the broker for a live log

This page is for the person who runs the MQTT broker that a textweaver reader connects to: for example, a friend who logs check-ins during a net and publishes them for others to follow. It is technical, and it assumes you are comfortable at a Linux command line. The reader's side is in [Live documents](live-documents.md).

textweaver is a receiver. It subscribes and never publishes, so the broker needs a publisher of its own (your logging tool, or `mosquitto_pub` for testing) and a read-only account for each reader.

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

You do not have to write the file by hand. Set the source up once in your own textweaver, then run **Save live source as file** (Tools, Live sources): it writes the source without its password, ready to send.

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
