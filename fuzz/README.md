# Fuzzing textweaver

This folder holds [cargo-fuzz](https://rust-fuzz.github.io/book/cargo-fuzz.html) targets for the document loaders and the settings files. It is its own Cargo workspace, so the main build, `cargo test`, and CI never build it.

Each target feeds random input to one part of textweaver. A loader may refuse the input with an error, but it must never panic, and a document it returns must have every marker inside its text, in order. The checks are in `src/lib.rs`.

## The targets

- `markdown`: the Markdown loader.
- `html`: the HTML loader, with its charset detection.
- `epub`: the EPUB loader. A zip archive is loaded as it is. Anything else becomes the book's one chapter, and, after a NUL byte, its navigation document.
- `docx`: the Word loader. A zip archive is loaded as it is. Anything else becomes `word/document.xml`, and, after a NUL byte, `word/numbering.xml`.
- `pdf`: the PDF loader.
- `settings`: `settings.toml` read into the settings, written back, and read again, and the same text planned as a settings import (JSON or TOML).

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
```

The `corpus`, `artifacts`, and `target` folders are not committed.

## When it finds a crash

cargo-fuzz saves the input under `fuzz/artifacts/TARGET/` and prints the command that runs it again, such as:

```bash
cargo +nightly fuzz run markdown fuzz/artifacts/markdown/crash-1234abcd
```

Fix the bug, then add the input as a regular test in the crate it came from, so it stays fixed. The loaders' property tests (`crates/textweaver-formats/tests/positions.rs`) and hostile-input tests (`crates/textweaver-formats/tests/hostile.rs`) are the place for loader cases.
