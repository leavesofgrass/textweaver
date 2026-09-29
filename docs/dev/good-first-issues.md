# Good first issues

Small, well-scoped tasks for a first contribution. Each one says what to change, where to look, and how to check it. They were checked against the code on Tuesday, September 29, 2026; if one has been done since, the code will show it.

Before you start:

- Read [CONTRIBUTING.md](../../CONTRIBUTING.md), and set up a build with [Building](building.md).
- Open an issue for the task you pick, or comment on one that exists, and say you are working on it. That way two people do not do the same work, and a maintainer can answer questions before you write code.
- Every task here must work with a screen reader and a Braille display. Where a task adds something the user hears, the message goes in the message catalog, `crates/textweaver-lexicon/locales/`, in all six languages. If you cannot translate, put the English text in each language and say so in your pull request; a maintainer will find a translator.

Sizes: small is a few hours; medium is about a day.

## Tooling and docs

### Run the docs checks in dev-check

Size: small. No Rust needed.

CI runs `cargo xtask docs --check` and `cargo xtask settings-doc --check`, but the local check script does not, so a contributor can pass dev-check and still fail CI. [Testing](testing.md#the-checks) marks both "not yet in dev-check".

- Where: `scripts/dev-check.sh` and `scripts/dev-check.ps1`. Copy how the `keyboard` step is written in each. Then update the step lists in [Testing](testing.md#the-checks) and `scripts/README.md`.
- How to check: `scripts/dev-check.sh --only docs,settings-doc` (and the PowerShell script on Windows) prints both steps and passes. Break a "See also" heading in a guide on purpose, and the `docs` step fails.

### US English in the English interface

Size: small. No Rust knowledge needed beyond editing strings.

The project writes US English, but some English messages and setting labels use British spellings, such as "licence" and "colour".

- Where: the message values in `crates/textweaver-lexicon/locales/en.ftl`, and the English labels and help in `crates/textweaver-app/src/settings_schema.rs`.
- Change only the text people read or hear. Keep every message id (the part before `=`), every setting key, and every Rust identifier as it is, so no one's settings break. Leave the other languages' files alone.
- Then regenerate the settings reference with `cargo xtask settings-doc`.
- How to check: `git grep -n -i -E "colour|licence|favourite|grey|catalogue" -- crates/textweaver-lexicon/locales/en.ftl` finds these words only in message ids (before the `=`), never in the text after it, and `cargo test -p textweaver-app -p textweaver-lexicon` passes.

### A fuzz target for plain text

Size: small.

Every document loader has a fuzz target except the plain-text loader. That loader decides a file's encoding (a byte order mark, then UTF-8, then Windows-1252), which is exactly the kind of code fuzzing is good at.

- Where: copy a short target such as `fuzz/fuzz_targets/rtf.rs` to `fuzz/fuzz_targets/text.rs`, loading the bytes with the hint `txt` through `textweaver_fuzz::load_checked`. Add a `[[bin]]` entry in `fuzz/Cargo.toml`, the target to the list in `fuzz/README.md`, `text` to the matrix in `.github/workflows/nightly.yml`, and seeds (`fixtures/sample.txt`) in `xtask/src/fuzz_seed.rs`.
- How to check: `cargo +nightly fuzz run text -- -max_total_time=60` runs without a crash. Fuzzing needs a nightly toolchain and cargo-fuzz; `fuzz/README.md` shows how to run it in Docker instead.

## Reading documents

### Obsidian callouts of every type

Size: small.

The Markdown loader reads GitHub's five alert types (`> [!NOTE]`, `[!TIP]`, `[!IMPORTANT]`, `[!WARNING]`, `[!CAUTION]`) as a block quote that starts with its label, such as "Note:". Obsidian has more types, such as `[!info]`, `[!example]`, `[!question]`, and `[!quote]`, and a title on the same line (`> [!tip] Remember`). Those are not recognized, so a listener hears the brackets.

- Where: `crates/textweaver-formats/src/markdown.rs`, `alert_label` and the block-quote handling around it. There is a test for alerts near the end of the file; add cases beside it.
- Say the type in words, then the title, then the text: "Info: Remember. ..." An unknown type reads as its own name, capitalized.
- How to check: `cargo test -p textweaver-formats markdown`. Then open a note with callouts in the terminal reader (`cargo run -p textweaver-tui -- note.md`) and listen: no brackets or exclamation marks are spoken.

### Pictures drawn inline in HTML

Size: small.

The HTML loader skips every `<svg>` element, so an inline drawing vanishes, even when it has a `<title>` or an `aria-label` that says what it shows. Images (`<img>`) are already read by their alternative text.

- Where: `crates/textweaver-formats/src/html.rs`. `svg` is in the `SKIP` list; the `image` function shows how a picture with a description is read. Read an `<svg>`'s `aria-label`, or else its first `<title>` child, the same way, and keep skipping one that has neither.
- How to check: add a test beside the existing image test in `html.rs`, and run `cargo test -p textweaver-formats html`. A page with `<svg aria-label="A bar chart">` reads "A bar chart" where the drawing was.

### JSON files as an outline

Size: medium.

A `.json` file opens today as plain text, so a listener hears every brace and quotation mark. A first version could read each top-level key as a heading, with its value below it, so heading navigation (`h`) moves from key to key.

- Where: a new loader in `crates/textweaver-formats/src/`, registered in `Registry::with_builtins` in `lib.rs`. `text.rs` is the smallest loader to learn from. `serde_json` is already in `[workspace.dependencies]`, so using it needs no new dependency.
- Read values plainly ("name: Ada Example"), and say the size of a nested object or array in words ("address, object, 3 keys") rather than printing it.
- New loaders get a fuzz target and a test with hostile input (very deep nesting, a huge file); see `crates/textweaver-formats/tests/hostile.rs`.
- How to check: `cargo test -p textweaver-formats`, then open a JSON file in the terminal reader and move with `h`.

### Subtitle files as transcripts

Size: medium.

textweaver writes SRT and WebVTT subtitles with its audio export, but cannot read them: a `.srt` or `.vtt` file opens as plain text, with every cue number and timestamp read aloud. A lecture's captions are often the only transcript a student has.

- Where: a new loader in `crates/textweaver-formats/src/`, as for JSON above. `crates/textweaver-export/src/cues.rs` writes both formats and shows their shape.
- Read the cue text as paragraphs. Leave out cue numbers, and keep the start time out of the spoken text (a heading every few minutes, such as "5 minutes", is one way to make time navigable).
- How to check: `cargo test -p textweaver-formats`, with a small fixture in `fixtures/` and a test that the text has no timestamps in it.

## The command palette and keys

### The category in each palette match

Size: small.

The command palette (F2) reads each match as "id: help. keys". It never says which category a command belongs to, so similar commands are hard to tell apart by ear.

- Where: the message `help-palette-item` in each file in `crates/textweaver-lexicon/locales/`, and `palette_candidates` in `crates/textweaver-app/src/help.rs`, which fills it in. `category_title` in the same file gives a category's name in the current language, so no new translations of category names are needed.
- Keep the command's name first, because a Braille display shows the start of the line: "open, File: open a document. Ctrl+O".
- How to check: `cargo test -p textweaver-app`, including `--test pseudo_locale`. Then press F2 in the terminal reader, type a few letters, and listen to the matches.

### Recent commands in an empty palette

Size: medium.

When the palette opens with nothing typed, the commands you used last should come first, marked "recent" in words.

- Where: `palette_candidates` and `palette_matches_in` in `crates/textweaver-app/src/help.rs`. Keep the last eight commands run from the palette. Talk in the issue about whether they are kept only while the program runs or saved with the rest of the reading state (`crates/textweaver-store`).
- How to check: a test in `help.rs` that runs two commands through the palette and finds them first, and a listen in the terminal reader.

### Say what a key does

Size: medium. It touches the keymap, the app, and the message catalog.

A screen reader's "input help" mode says what a key would do without doing it. textweaver has a list of every key (`?`) but no way to press one key and hear its command.

- Where: a new action in `crates/textweaver-keymap/src/action.rs` (with a default key and help text), handled in `crates/textweaver-app`; the keymap already knows each key's command. While the mode is on, each key press says the command's name and help, and Escape leaves the mode.
- Run `cargo xtask keyboard` to regenerate [the keyboard reference](../keyboard.md) after adding the action.
- How to check: `cargo test -p textweaver-keymap -p textweaver-app`, and in the terminal reader, turn the mode on, press Space, and hear its help, "Play or pause reading from the current word", instead of reading starting.

## Converting documents

### Save the failures of a batch conversion

Size: small.

`tw convert` on a folder prints each failure to the terminal as it happens, then a summary sentence. In a large folder, the failures scroll away before a screen reader user can review them.

- Where: `run` in `crates/textweaver-cli/src/cmd/convert.rs`. `Summary` in `crates/textweaver-convert/src/lib.rs` already has `failures()`. When any file failed, write each failure on its own line, file name first, then the reason, to `conversion-report.txt` in the output folder, and add a sentence that says where the report is.
- How to check: a test in `crates/textweaver-cli` that converts a folder holding one broken file, then reads the report. Write the report only when something failed, so a clean run leaves no extra file.

## See also

- [CONTRIBUTING.md](../../CONTRIBUTING.md): setting up, the checks, and how to propose a change.
- [Building](building.md) and [Testing](testing.md).
- [Architecture](architecture.md): where each crate fits.
- [Star features not yet planned](../star-gaps.md): larger gaps, for when you want more.
- [Documentation index](../README.md).
