# Fuzzing textweaver

This folder holds [cargo-fuzz](https://rust-fuzz.github.io/book/cargo-fuzz.html) targets for the document loaders, the settings and keyboard files, the saved state, the engine-host protocol, and the parsers a student's own writing goes through: math, citations, themes, the lexicon, Obsidian vaults, and JSON-RPC requests. It is its own Cargo workspace, so the main build, `cargo test`, and CI never build it.

Each target feeds random input to one part of textweaver. A loader may refuse the input with an error, but it must never panic, and a document it returns must have every marker inside its text, in order. The checks are in `src/lib.rs`.

## The targets

- `markdown`: the Markdown loader.
- `html`: the HTML loader, with its charset detection.
- `epub`: the EPUB loader. A zip archive is loaded as it is. Anything else becomes the book's one chapter, and, after a NUL byte, its navigation document.
- `docx`: the Word loader. A zip archive is loaded as it is. Anything else becomes `word/document.xml`, and, after a NUL byte, `word/numbering.xml`.
- `pdf`: the PDF loader.
- `rtf`: the RTF loader, our own iterative parser with its limits on nesting, `\bin`, and `\uc`, loaded with the default options and with tracked changes said in place.
- `odt`: the OpenDocument text loader. A zip archive is loaded as it is. Anything else becomes `content.xml`, and, after a NUL byte, `meta.xml`, in a minimal package, and is also loaded as flat `.fodt` XML. Every comment must lie inside the text.
- `docx_revisions`: Word's comments and tracked changes. A zip archive is loaded as it is. Anything else is split at NUL bytes into `word/document.xml`, `word/comments.xml`, and `word/commentsExtended.xml`. Every comment must lie inside the text, and replies must not loop.
- `daisy`: the DAISY loader. A zip archive is loaded as it is. Anything else becomes a book's DTBook file, and, after a NUL byte, its NCX; the bytes are also loaded as a DTBook file on their own.
- `pptx`: the PowerPoint loader. A zip archive is loaded as it is. Anything else becomes the one slide, and, after a NUL byte, its speaker notes.
- `sheet`: the spreadsheet loader. The first byte picks CSV, TSV, OpenDocument, or Excel; the rest is the file, or the sheet's XML in a minimal package.
- `archive`: the archive loader. The bytes are listed as a zip, tar, tar.gz, and 7z archive in turn.
- `image`: the PNG and JPEG decoders that OCR reads, with their size limits, and the picture loader with OCR off.
- `web`: a web page's response, read as the web loader reads it after fetching. The bytes up to the first NUL are the `Content-Type` value, and the rest is the body. No request is made and nothing is saved.
- `settings`: `settings.toml` read into the settings, written back, and read again, and the same text planned as a settings import (JSON or TOML).
- `keymap`: `keymap.toml`, the keyboard overrides, applied to every platform's and frontend's defaults; each line is also parsed as a key chord, which must print and parse back to itself.
- `state`: a document's saved state (position, history, bookmarks), which must write back and read again, and a folder's sidecar (`.textweaver/progress.json`), merged with itself and with nothing under every policy.
- `frame`: the engine-host frame decoder. The input is read as a stream of frames, as the app reads a host's output, and each frame is decoded as every engine's requests and replies. A message that decodes must encode to one that decodes the same.
- `latex_math`: the LaTeX math parser. Every node and diagnostic lies inside the source, children in order inside their parent; speech at every verbosity keeps a valid offset map; MathML and navigation do not panic. The text is also read as prose with `$...$` math in it, as speech reads a document.
- `asciimath`: the ASCIIMath parser, with the same checks as `latex_math`.
- `bibtex`: the BibTeX and BibLaTeX importer. The references it returns format in APA as entries and citations, and write in every format, without panicking.
- `ris`: the RIS importer, with the same checks as `bibtex`.
- `csl_json`: the CSL-JSON importer, with the same checks; written back as CSL-JSON, the references read again, as many as before.
- `theme`: the theme file reader. A file that reads writes back as TOML that reads again and writes the same text. Resolved, on its own and on a built-in theme, it goes through the contrast check, CSS, and terminal styles without panicking.
- `lexicon`: the lexicon data file reader. Input that starts with the file's magic bytes is a whole file, read as it is and with its checksum made right. Any other input is a list of patches (a four-byte little-endian offset, a length byte, the bytes) to a small valid lexicon the target builds. A damaged file is refused with an error; a file that opens is looked up, completed, and defined.
- `vault_import`: the Obsidian vault importer. The bytes are two notes, split at the first NUL. Each is parsed, and its front matter, written back, reads again the same. The two notes are also written to a folder of their own under the system's temporary folder and read as a vault in both import modes.
- `rpc`: the JSON-RPC server behind `tw serve`. Each line is one message to a server with a small document open in memory, silent speech, and nothing saved. Every reply is a JSON-RPC 2.0 object that encodes and decodes the same, and a request with an id gets exactly one response. Messages naming a method that can open, write, or save a file (`open`, `action`, `answer`, `choose`, `list_key`, `prompt_key`, `set_setting`, `shutdown`, `exit`) are skipped. This target links the app, whose audio output needs the ALSA headers on Linux (`libasound2-dev`), so it builds only with `--features rpc`.
- `latex`: the LaTeX loader, our own tokenizer and two-pass parser with its limits on size, tokens, steps, macro expansions, and nesting. Loaded with the default options, with code skipped and notes inline, and with skipped commands named in place. The input has no folder, so `\input` reads nothing.
- `eml`: email and web archives through mail-parser. The bytes are loaded as an email and as a web archive, with the limits on size, parts, nesting, and body text.

The loaders are built with the formats crate's `images` feature instead of `ocr`: the picture and scanned-page loaders are fuzzed with OCR off, so the in-process OCR engine, which took 45 minutes to compile under the sanitizer, is left out. The `rpc` target links the app, which still brings it.

CI runs every target for 10 minutes each night (`.github/workflows/nightly.yml`) and keeps any crash as an artifact named `fuzz-crash-TARGET`. Download it from the run's page and replay it as below.

## Run a target

cargo-fuzz needs a nightly Rust toolchain. Install it once:

```bash
rustup toolchain install nightly
cargo install cargo-fuzz
```

Then, from the repository root:

```bash
cargo +nightly fuzz run markdown
```

It runs until it finds a crash or you press Control C. To run for a set time, for example five minutes:

```bash
cargo +nightly fuzz run docx -- -max_total_time=300
```

The `rpc` target needs its feature, and on Linux the ALSA headers:

```bash
cargo +nightly fuzz run rpc --features rpc
```

To list the targets:

```bash
cargo +nightly fuzz list
```

Linux and macOS work best. On Windows, cargo-fuzz works with the MSVC toolchain, but the simplest way is the development container:

```bash
docker compose -p textweaver run --rm dev bash -c "rustup toolchain install nightly && cargo install cargo-fuzz && cargo +nightly fuzz run markdown -- -max_total_time=300"
```

On Windows, give the fuzzer a corpus folder inside the container, such as a copy of `fuzz/corpus/markdown` in `/tmp`, as the first argument after the target. libFuzzer reads and writes its corpus folder all the time, and through the shared Windows folder the `latex` and `eml` targets managed 2 to 4 runs a second instead of 35 to 83 (Monday, September 28, 2026). Copy new finds back only if you want to keep them.

## Seed the corpus

The fuzzer finds its way faster when it starts from real files. Copy the fixtures that suit each target into its corpus folder before the first run:

```bash
cargo xtask fuzz-seed
```

Or for one target:

```bash
cargo xtask fuzz-seed --target bibtex
```

It only creates folders and copies files into `fuzz/corpus/`. Which fixtures go to which target is listed in `xtask/src/fuzz_seed.rs`; the small seeds for the math and JSON-RPC targets are in `fixtures/cloud/`. The nightly workflow seeds every target this way.

The `corpus`, `artifacts`, and `target` folders are not committed.

## When it finds a crash

cargo-fuzz saves the input under `fuzz/artifacts/TARGET/` and prints the command that runs it again, such as:

```bash
cargo +nightly fuzz run markdown fuzz/artifacts/markdown/crash-1234abcd
```

Fix the bug, then add the input as a regular test in the crate it came from, so it stays fixed. The loaders' property tests (`crates/textweaver-formats/tests/positions.rs`) and hostile-input tests (`crates/textweaver-formats/tests/hostile.rs`, and `hostile_w3d.rs` for DAISY, PowerPoint, spreadsheets, archives, and pictures) are the place for loader cases.
