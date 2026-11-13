# Good first issues

Small, well-scoped tasks for a first contribution. Each one says what to change, where to look, and how to check it. They were checked against the code on Friday, November 13, 2026; if one has been done since, the code will show it.

Before you start:

- Read [CONTRIBUTING.md](../../CONTRIBUTING.md), and set up a build with [Building](building.md).
- Open an issue for the task you pick, or comment on one that exists, and say you are working on it. That way two people do not do the same work, and a maintainer can answer questions before you write code.
- Every task here must work with a screen reader and a Braille display. Where a task adds something the user hears, the message goes in the message catalog, in English, in the area file under `crates/textweaver-lexicon/locales/en/`. Translate it into the other five languages if you can; if you cannot, add its id to `crates/textweaver-lexicon/locales/untranslated.txt` and say so in your pull request. The other languages then show the English text until the translation pass before a release ([Messages](messages.md)).
- Where a task adds a command, it lives in its own module and registers a handler with `App::register_handler`, so `app.rs` is not edited. Add the command to `crates/textweaver-keymap/src/action.rs` (id, category, default key, help), give it a place in `crates/textweaver-app/src/menu.rs`, and add its `name-` and `action-` messages to the catalog; `every_action_is_handled` fails if the handler is missing. [Architecture](architecture.md) describes the registry.

Sizes: small is a few hours; medium is about a day.

## Tooling and docs

### A fuzz target for plain text

Size: small.

Almost every document loader has a fuzz target, caption files included. The plain-text loader has none (nor has the braille file loader, which needs liblouis). The plain-text loader decides a file's encoding (a byte order mark, then UTF-8, then Windows-1252), which is exactly the kind of code fuzzing is good at.

- Where: copy a short target such as `fuzz/fuzz_targets/rtf.rs` to `fuzz/fuzz_targets/text.rs`, loading the bytes with the hint `txt` through `textweaver_fuzz::load_checked`. Add a `[[bin]]` entry in `fuzz/Cargo.toml`, the target to the list in `fuzz/README.md`, `text` to the matrix in `.github/workflows/nightly.yml`, and seeds (`fixtures/sample.txt`) in `xtask/src/fuzz_seed.rs`.
- How to check: `cargo +nightly fuzz run text -- -max_total_time=60` runs without a crash. Fuzzing needs a nightly toolchain and cargo-fuzz; `fuzz/README.md` shows how to run it in Docker instead.

## Reading documents

Obsidian callouts of every type, SVG drawings inline in HTML, and JSON files as an outline were all built in Wave 6 ([ADR-0044](../adr/0044-obsidian-json-svg-and-content-mathml.md)), and caption files read as transcripts in beta 6 ([Captions as transcripts](../reading.md#captions-as-transcripts)); see [CHANGELOG.md](../../CHANGELOG.md).

## The command palette and keys

The category in each palette match, recent commands in an empty palette, and "what does this key do" (Shift+F1) were all built in Wave 6 ([ADR-0043](../adr/0043-menus-and-the-palette-from-one-model.md)); see [CHANGELOG.md](../../CHANGELOG.md) for what to build on next.

## See also

- [CONTRIBUTING.md](../../CONTRIBUTING.md): setting up, the checks, and how to propose a change.
- [Building](building.md) and [Testing](testing.md).
- [Architecture](architecture.md): where each crate fits.
- [star features not yet planned](../star-gaps.md): larger gaps, for when you want more.
- [Documentation index](../README.md).
