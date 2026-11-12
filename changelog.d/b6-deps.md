### Security

- pulldown-cmark 0.13.5 to 0.13.6, for RUSTSEC-2026-0433: a Markdown file with deeply nested image wikilinks (`![[`) took time in the depth times the body, so it could hang a load. A hostile-input test loads 10,000 levels over a 1 MB body within 5 seconds.
- calamine 0.36.1 to 0.38.0, for RUSTSEC-2026-0436 (a panic and an unbounded allocation in its compound-file reader). Spreadsheets load as before. zip 8 stays in the tree through harper-core and MathCAT, neither of which calls the affected extraction code.
- AccessKit's October 25, 2026 patch releases: accesskit 0.25.2, accesskit_consumer 0.39.2, accesskit_windows 0.35.2 (vendored again, with textweaver's shortcut-key change applied unchanged), accesskit_unix 0.24.1, accesskit_macos 0.27.2 and accesskit_winit 0.34.2. They keep the content of password fields from screen readers. tokio moves to 1.54.1, which accesskit_unix 0.24.1 requires.
- async-executor 1.14.0, which was yanked, to 1.14.1 (Linux builds, through the keyring's D-Bus library).
