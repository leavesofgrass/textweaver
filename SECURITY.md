# Security policy

textweaver reads documents that come from anywhere: course sites, email, shared drives. A document must never be able to crash the reader in a way that loses work, run code, read or write files it should not, or reach the network. Please report anything that could let it.

## Reporting a vulnerability

Please do not open a public issue for a vulnerability. Report it privately through GitHub's private vulnerability reporting:

1. Go to the repository's **Security** tab.
2. Choose **Report a vulnerability**. The form is also at [the new advisory page](https://github.com/leavesofgrass/textweaver/security/advisories/new).
3. Describe the problem. Only you and the maintainers can see the report.

If the **Report a vulnerability** button is not there, open an ordinary issue that asks the maintainers for a private way to report a security problem. Leave out every detail of the problem itself.

Include what you can:

- the textweaver version (`tw --version`) and your operating system;
- the steps, and a file that shows the problem if there is one;
- what an attacker could do with it.

You will get an answer within a week. textweaver is maintained by one person, so a fix may take longer; you will be told the plan and credited in the release notes unless you ask not to be.

## Supported versions

textweaver is in beta. Only the newest release, and `main`, get security fixes.

## In scope

These are the parts that handle input textweaver does not control. Reports about any of them are welcome.

- **Document parsers.** PDF (lopdf and textweaver's own content-stream interpreter), EPUB and DOCX (ZIP archives holding XML), HTML (html5ever and scraper), XML (roxmltree), Markdown (pulldown-cmark and comrak), caption files (SubRip and WebVTT), and plain text in several encodings. The HTML cleaner behind the browser preview and `--sanitize` is ammonia. Hostile input includes deep nesting, huge or recursive objects, decompression bombs, path traversal in ZIP entry names, and malformed encodings. The Pandoc loader, when enabled, runs Pandoc with `--sandbox`.
- **The hot folder.** `tw convert --watch` converts files as they appear in a folder. Anyone who can write to that folder can feed it files, and the output must stay inside the output folder. The hot folder never follows a symbolic link or a junction, and every name it writes is plain: a name that comes from outside is cut to letters, digits, `-`, `_` and `.` before anything is written.
- **JSON-RPC.** `tw serve --stdio` accepts commands from the program that started it, on standard input. It opens no network port. A client can open files and change settings, so it must be trusted like the user.
- **The engine-host protocol.** Speech engines run in separate host processes (a copy of `tw` or the app started with `--engine-host <engine>`, or on Windows the 32-bit `textweaver-host-x86.exe`) that speak a framed binary protocol over pipes. textweaver must survive a host that crashes or sends malformed frames, and a host must exit when textweaver does. Engine libraries are the user's own (Eloquence, DECtalk), loaded from paths in the settings or the environment.
- **Live documents and the broker.** The MQTT client for live sources (`textweaver-mqtt`) reads packets from a broker the user named, and `tw broker` (`textweaver-broker`, also run inside the app) reads packets from any computer that can reach its port. Hostile input includes malformed or oversized packets, a flood of messages, topics and JSON built to exhaust memory or to inject control characters into the display, a reader that tries to publish, repeated failed logins, and a certificate or pin that does not match. Join links, QR code images (`textweaver-qr`) and live source files (`.twlive`) come from other people and are parsed as untrusted input; a live source file may never carry a password or a join code. Certificate checking cannot be turned off. On Linux and macOS the broker's key and accounts file are set to their owner only; on Windows they take the data folder's permissions. A plain listener on a Tailscale-range address (100.64.0.0/10) is called Tailscale only when the broker finds a Tailscale network interface, which it can do only on Linux; elsewhere it warns that the link may not be encrypted. Join codes and links are copied to the clipboard as private text, which clipboard history and managers that follow the convention skip.
- **What a join code resists.** A join code's last four characters are its secret, one of about a million values. The certificate reveals only the first twelve (the key's short pin), and the password derived from the code travels only inside TLS to that pinned key, so guesses cannot be tested offline; each one is a login. The broker limits online guessing: five failed logins within a minute lock an address (an IPv6 /64) out for five minutes, counting logins still being checked, so a burst gets no more answers than single tries; one address may hold 8 connections before login and 16 after, and the join code 50; and after 200 wrong join codes in one session the code turns off for everyone, which caps a guesser's chance at about 0.02 percent however many addresses they use. Stopping the broker ends a code. A way to get more guesses than this is in scope.
- **The browser preview.** A document may hold raw HTML and scripts, and the preview is a web page served from the user's computer. By default textweaver leaves out scripts, event handlers, `javascript:` addresses and frames, and the served page carries a content security policy (only textweaver's reload script, with a new nonce for each page, and only its own server for images, style sheets, fonts and connections). Only folders listed in `[preview] trusted_folders` keep raw HTML. The server listens on 127.0.0.1 behind a random token drawn from the system's random source, and serves only images, style sheets and fonts from the document's folder. A way for a document outside a trusted folder to run a script, read another file, or reach the network through the preview is in scope.
- **Links to other computers.** A link to a UNC or `file:` path with a host asks first, because on Windows even checking the file sends the computer the user's sign-in; opening such a path with the default program is refused. A way to reach another computer without the question is in scope.
- **Terminal output.** `tw text`, `tw search` and `tw info` leave out control characters other than tab and line feed when standard output is a terminal, so a document cannot set the terminal's title or write the clipboard through an escape sequence. A way around that is in scope.
- **Network lookups.** Only when the user asks: citation lookups by DOI, ISBN and PubMed ID (doi.org, Open Library, and NCBI's E-utilities, which gets only the PubMed ID) over HTTPS with rustls, web pages the user names, and the offer to download a missing reading font. Every request carries one neutral User-Agent, without a version or any name of the user. Responses are untrusted input to the citation and font parsers. Nothing is sent without a user action, and no telemetry is collected.
- **Settings, state, and themes.** TOML and JSON files in the user's folders, and settings files imported with `tw settings import`, which are validated before they are applied.
- **The installers.** `scripts/install-macos.sh`, `scripts/install-windows.ps1` and `scripts/install-linux.sh` (with `--release`) download release packages from GitHub over HTTPS and check them against `SHA256SUMS.txt`. Without `--release`, `scripts/install-linux.sh` installs build dependencies with the system package manager and builds from source; the macOS and Windows scripts can build from source too. Release packages also carry build provenance attestations (`gh attestation verify FILE --repo leavesofgrass/textweaver`).

## Out of scope

- Problems that need an attacker who already controls the user's account or the textweaver settings folder.
- Bugs in the speech engines themselves (Eloquence, SAPI voices, DECtalk, espeak-ng, speech-dispatcher), unless textweaver makes them reachable from a document. Please report those to their makers too.
- Crashes or hangs on a document, when they lose no work and cannot be turned into anything worse. These are welcome as ordinary [bug reports](https://github.com/leavesofgrass/textweaver/issues/new/choose).

## See also

- [Contributing](CONTRIBUTING.md): how to report other problems, and how to send a fix.
- [Code of conduct](CODE_OF_CONDUCT.md).
