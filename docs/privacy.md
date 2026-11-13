# Privacy

textweaver sends nothing over the network unless you ask it to. It has no account, no analytics, and no crash reports. Your documents, notes, and settings stay on your computer.

## What stays on your computer

- Your documents, and every file textweaver writes.
- Notes, highlights, bookmarks, reading places, and reading statistics.
- Settings, profiles, and word lists.
- The log file, which is for you and for a bug report you choose to send.
- A live document's JSON log, which can hold other people's names and callsigns. It is never synced.
- Speech, OCR, dictation, and the dictionary. They run on your computer. Looking up a word's definition never goes to the internet.

## When textweaver uses the network

Only when you ask. Each item below happens after an action of yours.

- **Optional downloads.** The dictation model, the OCR models, the Lexend font, Piper voices, and the helper programs ffmpeg, liblouis, and Pandoc are not in the packages. textweaver asks first, names the size and the license, and downloads only when you say yes. Saying no is always fine. See [Optional components](components.md).
- **Update checks.** Only if you say yes to the one question about them, textweaver reads the list of releases from GitHub once a day, and Help, Check for updates and `tw update` do it when you ask. A newer release is downloaded only after another yes (`tw update` is itself the yes), and it is checked against the release's checksums. See [Updates](updates.md).
- **Adding a reference by DOI, ISBN, or PubMed ID.** The DOI is sent to doi.org, the ISBN to Open Library, or the PubMed ID to NCBI's E-utilities (eutils.ncbi.nlm.nih.gov). Nothing else is sent: no API key, tool name, or email address. See [Citations](citations.md#what-is-sent-over-the-network).
- **Opening a web page** by its address, with `tw open` or `tw text`. textweaver fetches the page you named.
- **Opening a link** in a document. textweaver asks first, and then your browser opens it.
- **A link to a file on another computer.** A link to a network share, such as `\\server\share\notes.md`, `//server/share/notes.md`, or `file://server/share/notes.md`, asks "Open a file on another computer? y or n. Your sign-in may be sent to it." and the path, before textweaver so much as looks for the file, because on Windows even checking that such a file exists connects to that computer and offers it your Windows sign-in. Only **y** goes on; anything else leaves the link alone. Opening a file with the system's default program refuses such a path outright.
- **Live sources.** When you open a live source, textweaver connects to the MQTT broker you named in your settings. The broker sees your computer's network address, your user name, the topics you subscribe to, and when you connect, and with no encryption anyone on the path can read the password and the messages. An address in the range Tailscale uses (100.64.0.0/10) is said to be one only as "If this is Tailscale, the link is encrypted", because phone and internet providers use the same range; textweaver cannot confirm Tailscale from the reader's side. The connection uses a neutral client id, `tw-` and random digits, new at each start, and sends no name, computer name, or account name. A live source only receives; it publishes nothing. See [Live documents](live-documents.md#privacy).
- **Sending and the broker, only when you run them.** `tw publish`, and in the app Publish a check-in and Publish this document live, publish exactly the entry or the document you give them, to the broker you name or to the broker the app runs, with the same neutral client id. `tw broker`, and Start your broker in the app, accept connections on the addresses you give them (this computer only, by default) and keep the accounts' password hashes, a key and a certificate in textweaver's data folder (the key and the accounts file are set to their owner only on Linux and macOS; on Windows they take the data folder's permissions, which under your profile are yours alone); they show each client's network address and user name while they run and write them nowhere else. See [Running the broker for a live log](live-broker.md#textweavers-own-broker).
- **Join codes, links and QR codes.** A join link holds the broker's address and a join code, so it is as private as the code. textweaver copies a link or a code to the clipboard only when you choose Copy, and says so without saying the code. It marks the copied code or link as private, so it stays out of clipboard history: on Windows it is left out of the clipboard history list (Windows+V) and is not uploaded to the cloud clipboard; on macOS it is marked concealed, which clipboard managers that follow the convention skip; on Linux it is marked for clipboard managers to skip. In a terminal that receives copied text through its own escape sequence (OSC 52, the usual way over SSH), the terminal decides what to keep, and textweaver cannot mark it. A QR code is drawn or saved only when you ask, to the file you name, and textweaver keeps no copy. Join from a QR image reads the clipboard's image, or the image file you name, only then, and keeps neither the image nor the link. A join code resists guessing, not eavesdropping on a connection that is not encrypted: its secret part is one of about a million values, the password it gives travels only inside TLS to the key its pin names, and the broker limits guesses (five wrong logins within a minute lock an address out for five minutes, connections are capped, and after 200 wrong codes the code turns off), as [the broker guide](live-broker.md#a-join-code-for-a-quick-session) explains. The code itself is kept nowhere: a reader keeps the code's pin in its settings, which is not a secret, and the password the code gives in the system credential store.
- **Install and update scripts.** They download a release from GitHub and check it. Each says what it will do first.

Every request textweaver makes over HTTP, whether the components downloader, a web page you open, a reference lookup or the update check, carries one neutral User-Agent, `textweaver-research (+https://github.com/leavesofgrass/textweaver)`, the same in every release and without a version. It carries nothing about you or your computer.

## The browser preview

The browser preview shows your document as a web page, and a document can hold raw HTML, including scripts. A script in a document someone else wrote could, on a page served from your computer, read the files beside the document and send them elsewhere. So by default the preview is cleaned: scripts, event handlers such as `onclick`, `javascript:` addresses, frames, and other unsafe markup are left out, while ordinary HTML such as `<b>` or `<details>` stays. When something is left out, textweaver says once for that document: "Raw HTML left out of the preview. Trusted folders are in Settings."

When the preview follows your saves or typing, the page comes from a small server on this computer only (127.0.0.1, with a random secret in the address). That server adds a second guard. The page may run only its own reload script, may load images, style sheets, and fonts only from that server, and may connect only to it, so a remote image cannot reveal that you opened the document. From the document's folder it serves images, style sheets, and fonts, and never another document, a text file, or a file or folder whose name starts with a dot.

**Trusted folders** (`trusted_folders` in `[preview]`, Trusted preview folders in Settings) are folders whose documents keep their raw HTML, scripts included, and are served without the second guard. Trust only folders of your own writing, such as your notes: a script in a trusted document runs with the same reach as the page. A folder trusts the folders inside it. The list is empty by default, it belongs to this computer, and it is never synced. `tw convert` cleans only with `--sanitize`, as before.

## Syncing between computers

textweaver has no sync server and no network code for sync. If you turn sync on, it writes files into a folder you chose. Something else you run, such as Syncthing or a USB stick, moves them. No name, path, or computer name is written to that folder. See [Syncing between computers](sync.md).

## A components mirror

If you set a components mirror, textweaver reads that mirror's list of components from it. Without a mirror, it reads nothing until you say yes.

## What textweaver does not do

- It does not collect telemetry.
- It does not store a password in a file. The two secrets it can keep, a GitHub token for a components source of your own and the password of a live source (which a join code gives), go only into your system's credential store, and only if you type them in or join with a code. The join code field is masked, and nothing typed in a secret field is kept in the prompt's history or written to the log.
- It does not send your documents anywhere.
- It does not let a document write to your terminal. `tw text`, `tw search` and `tw info` leave out control characters when their output goes to a terminal, so a document cannot set the terminal's title or write to the clipboard through an escape sequence ([The command line](command-line.md#control-characters-on-a-terminal)).
- It does not install anything outside its own data folder and its own program folder. An update replaces only the textweaver package you installed, and a downloaded installer starts only after you say yes.

## Reporting a problem

A bug report is a page on GitHub that you write and send yourself. Read the log before you attach it: it can hold file names.

## See also

- [Optional components](components.md)
- [Syncing between computers](sync.md)
- [Citations](citations.md)
- [Security policy](https://github.com/leavesofgrass/textweaver/blob/main/SECURITY.md)
