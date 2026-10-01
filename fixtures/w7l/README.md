# Fixtures for downloading Lexend

The two Lexend font files textweaver downloads on first choice, unchanged from the Lexend project (googlefonts/lexend, commit `cd26b9c2538d758138c20c3d2f10362ed613854b`, `fonts/lexend/ttf/`). Their sizes and SHA-256 values are the pinned ones in `crates/textweaver-fonts/src/downloaded.rs`. License: SIL Open Font License 1.1, in `third_party/fonts/lexend/OFL.txt`; see `third_party/fonts/lexend/README.md`.

Tests never download. A fake fetcher in the fonts, app, writers, and GUI tests hands out these files for the pinned URLs, so the real checks (size and hash), the data folder layout, and the font itself (parsed, embedded in a PDF and an EPUB, registered in the window) are tested without a network.

- `Lexend-Regular.ttf`: 100,264 bytes.
- `Lexend-Bold.ttf`: 105,564 bytes.
