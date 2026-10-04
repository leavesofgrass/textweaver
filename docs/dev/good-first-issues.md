# Good first issues

Small, well-scoped tasks for a first contribution. Each one says what to change, where to look, and how to check it. They were checked against the code on Wednesday, September 30, 2026; if one has been done since, the code will show it.

Before you start:

- Read [CONTRIBUTING.md](../../CONTRIBUTING.md), and set up a build with [Building](building.md).
- Open an issue for the task you pick, or comment on one that exists, and say you are working on it. That way two people do not do the same work, and a maintainer can answer questions before you write code.
- Every task here must work with a screen reader and a Braille display. Where a task adds something the user hears, the message goes in the message catalog, `crates/textweaver-lexicon/locales/`, in all six languages. If you cannot translate, put the English text in each language and say so in your pull request; a maintainer will find a translator.

Sizes: small is a few hours; medium is about a day.

## Tooling and docs

### A fuzz target for plain text

Size: small.

Every document loader has a fuzz target except the plain-text loader. That loader decides a file's encoding (a byte order mark, then UTF-8, then Windows-1252), which is exactly the kind of code fuzzing is good at.

- Where: copy a short target such as `fuzz/fuzz_targets/rtf.rs` to `fuzz/fuzz_targets/text.rs`, loading the bytes with the hint `txt` through `textweaver_fuzz::load_checked`. Add a `[[bin]]` entry in `fuzz/Cargo.toml`, the target to the list in `fuzz/README.md`, `text` to the matrix in `.github/workflows/nightly.yml`, and seeds (`fixtures/sample.txt`) in `xtask/src/fuzz_seed.rs`.
- How to check: `cargo +nightly fuzz run text -- -max_total_time=60` runs without a crash. Fuzzing needs a nightly toolchain and cargo-fuzz; `fuzz/README.md` shows how to run it in Docker instead.

## Reading documents

Obsidian callouts of every type, SVG drawings inline in HTML, and JSON files as an outline were all built in Wave 6 ([ADR-0044](../adr/0044-obsidian-json-svg-and-content-mathml.md)); see [CHANGELOG.md](../../CHANGELOG.md).

### Subtitle files as transcripts

Size: medium.

textweaver writes SRT and WebVTT subtitles with its audio export, but cannot read them: a `.srt` or `.vtt` file opens as plain text, with every cue number and timestamp read aloud. A lecture's captions are often the only transcript a student has.

- Where: a new loader in `crates/textweaver-formats/src/`, registered in `Registry::with_builtins` in `lib.rs`; `crates/textweaver-formats/src/json.rs` is a recent loader to learn the shape from, and `crates/textweaver-export/src/cues.rs` writes both subtitle formats and shows their shape.
- Read the cue text as paragraphs. Leave out cue numbers, and keep the start time out of the spoken text (a heading every few minutes, such as "5 minutes", is one way to make time navigable).
- How to check: `cargo test -p textweaver-formats`, with a small fixture in `fixtures/` and a test that the text has no timestamps in it.

## The command palette and keys

The category in each palette match, recent commands in an empty palette, and "what does this key do" (Shift+F1) were all built in Wave 6 ([ADR-0043](../adr/0043-menus-and-the-palette-from-one-model.md)); see [CHANGELOG.md](../../CHANGELOG.md) for what to build on next.

## See also

- [CONTRIBUTING.md](../../CONTRIBUTING.md): setting up, the checks, and how to propose a change.
- [Building](building.md) and [Testing](testing.md).
- [Architecture](architecture.md): where each crate fits.
- [star features not yet planned](../star-gaps.md): larger gaps, for when you want more.
- [Documentation index](../README.md).
