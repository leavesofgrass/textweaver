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

- **Document parsers.** PDF (lopdf and textweaver's own content-stream interpreter), EPUB and DOCX (ZIP archives holding XML), HTML (html5ever and scraper), XML (roxmltree), Markdown (pulldown-cmark and comrak), and plain text in several encodings. Hostile input includes deep nesting, huge or recursive objects, decompression bombs, path traversal in ZIP entry names, and malformed encodings. The Pandoc loader, when enabled, runs Pandoc with `--sandbox`.
- **The hot folder.** `tw convert --watch` converts files as they appear in a folder. Anyone who can write to that folder can feed it files, and the output must stay inside the output folder.
- **JSON-RPC.** `tw serve --stdio` accepts commands from the program that started it, on standard input. It opens no network port. A client can open files and change settings, so it must be trusted like the user.
- **The engine-host protocol.** Speech engines run in separate host processes (`textweaver-eci-host`, `textweaver-sapi-host`, `textweaver-dectalk-host`) that speak a framed binary protocol over pipes. textweaver must survive a host that crashes or sends malformed frames, and a host must exit when textweaver does. Engine libraries are the user's own (Eloquence, DECtalk), loaded from paths in the settings or the environment.
- **Network lookups.** Only when the user asks: citation lookups by DOI and ISBN (doi.org, Open Library) over HTTPS with rustls, and the offer to download a missing reading font. Responses are untrusted input to the citation and font parsers. Nothing is sent without a user action, and no telemetry is collected.
- **Settings, state, and themes.** TOML and JSON files in the user's folders, and settings files imported with `tw settings import`, which are validated before they are applied.
- **The installers.** `scripts/install-macos.sh` and `scripts/install-windows.ps1` download release packages from GitHub over HTTPS and check them against `SHA256SUMS.txt`, or build from source. `scripts/install-linux.sh` installs build dependencies with the system package manager and builds from source. Release packages also carry build provenance attestations (`gh attestation verify FILE --repo leavesofgrass/textweaver`).

## Out of scope

- Problems that need an attacker who already controls the user's account or the textweaver settings folder.
- Bugs in the speech engines themselves (Eloquence, SAPI voices, DECtalk, espeak-ng, speech-dispatcher), unless textweaver makes them reachable from a document. Please report those to their makers too.
- Crashes or hangs on a document, when they lose no work and cannot be turned into anything worse. These are welcome as ordinary [bug reports](https://github.com/leavesofgrass/textweaver/issues/new/choose).

## See also

- [Contributing](CONTRIBUTING.md): how to report other problems, and how to send a fix.
- [Code of conduct](CODE_OF_CONDUCT.md).
