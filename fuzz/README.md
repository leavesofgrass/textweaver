# Fuzzing textweaver

This folder holds [cargo-fuzz](https://rust-fuzz.github.io/book/cargo-fuzz.html) targets for the document loaders, the settings and keyboard files, the saved state, and the engine-host protocol. It is its own Cargo workspace, so the main build, `cargo test`, and CI never build it.

Each target feeds random input to one part of textweaver. A loader may refuse the input with an error, but it must never panic, and a document it returns must have every marker inside its text, in order. The checks are in `src/lib.rs`.

## The targets

- `markdown`: the Markdown loader.
- `html`: the HTML loader, with its charset detection.
- `epub`: the EPUB loader. A zip archive is loaded as it is. Anything else becomes the book's one chapter, and, after a NUL byte, its navigation document.
- `docx`: the Word loader. A zip archive is loaded as it is. Anything else becomes `word/document.xml`, and, after a NUL byte, `word/numbering.xml`.
- `pdf`: the PDF loader.
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

To list the targets:

```bash
cargo +nightly fuzz list
```

Linux and macOS work best. On Windows, cargo-fuzz works with the MSVC toolchain, but the simplest way is the development container:

```bash
docker compose -p textweaver run --rm dev bash -c "rustup toolchain install nightly && cargo install cargo-fuzz && cargo +nightly fuzz run markdown -- -max_total_time=300"
```

## Seed the corpus

The fuzzer finds its way faster when it starts from real files. Copy some fixtures into the target's corpus folder before the first run:

```bash
mkdir -p fuzz/corpus/markdown fuzz/corpus/html fuzz/corpus/pdf fuzz/corpus/docx fuzz/corpus/epub
cp fixtures/sample.md fuzz/corpus/markdown/
cp fixtures/sample.html fuzz/corpus/html/
cp fixtures/a/*.pdf fuzz/corpus/pdf/
cp fixtures/a/*.docx fuzz/corpus/docx/
cp fixtures/a/*.epub fuzz/corpus/epub/
mkdir -p fuzz/corpus/archive fuzz/corpus/image
cp fixtures/w3d/course.7z fuzz/corpus/archive/
cp fixtures/w3d/*.png fuzz/corpus/image/
```

The `corpus`, `artifacts`, and `target` folders are not committed.

## When it finds a crash

cargo-fuzz saves the input under `fuzz/artifacts/TARGET/` and prints the command that runs it again, such as:

```bash
cargo +nightly fuzz run markdown fuzz/artifacts/markdown/crash-1234abcd
```

Fix the bug, then add the input as a regular test in the crate it came from, so it stays fixed. The loaders' property tests (`crates/textweaver-formats/tests/positions.rs`) and hostile-input tests (`crates/textweaver-formats/tests/hostile.rs`, and `hostile_w3d.rs` for DAISY, PowerPoint, spreadsheets, archives, and pictures) are the place for loader cases.
