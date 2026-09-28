# Testing textweaver

The checks every change must pass, how the tests are written, and the benchmarks. [Building](building.md) covers setting up a machine first.

## The checks

CI runs these on every push. Run them yourself before you send a change. One script runs them all:

```bash
scripts/dev-check.sh
```

On Windows:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\dev-check.ps1
```

To run the full Linux set, with every feature, inside the container:

```bash
scripts/dev-check.sh --docker
```

The steps, in order:

- **fmt**: `cargo fmt --all --check`. Formatting follows `rustfmt.toml`.
- **clippy**: `cargo clippy --workspace --all-targets` with the features for your system, and `-D warnings`. Every warning is an error.
- **test**: `cargo test --workspace` with the same features.
- **doc**: `cargo doc --workspace --no-deps` with `RUSTDOCFLAGS="-D warnings"`. The usual failures are a redundant link target (write ``[`X`]``, not ``[`X`](crate::X)``), a link to a private item from public docs, and square brackets in prose (put `[mm:ss]` or `[@key]` in backticks).
- **keyboard**: `cargo xtask keyboard --check`. It fails when [docs/keyboard.md](../keyboard.md) is out of date. Regenerate it with `cargo xtask keyboard`; never edit it by hand.
- **pseudo**: `cargo test -p textweaver-app --test pseudo_locale`. The interface in the pseudo-locales en-XA and ar-XB: it fails on any message that does not come from the translation catalog ([ADR-0030](../adr/0030-interface-translations.md)). CI runs it in the docs job.
The Python checks are written `python3` below. On Windows, run them with `py -3` instead (`py -3 tools/check_links.py`): `python` and `python3` there may be the Microsoft Store stub.

- **links**: `python3 tools/check_links.py`. Every relative link and anchor in the Markdown docs and in `docs/site` must resolve.
- **site**: `python3 tools/gen_site_data.py --check`. The data embedded in the `docs/site` pages must match `cargo metadata`, the keymap, and the theme files. Regenerate it with `python3 tools/gen_site_data.py`.
- **site-a11y**: `python3 tools/check_site_a11y.py`. Static accessibility checks of the `docs/site` pages: language, title, one level-1 heading and no skipped levels, the skip link, landmarks, a label for every control, text alternatives, and references that resolve.
- **docs** (in CI, not yet in dev-check): `cargo xtask docs --check`. The ADR index and the Decisions list in [docs/README.md](../README.md#decisions) list every ADR once, in number order; the crate counts match `crates/`; every guide ends with a "See also" section and is linked from the index.
- **settings-doc** (in CI, not yet in dev-check): `cargo xtask settings-doc --check`. [The settings reference](../settings-reference.md) matches the settings schema. After adding or changing a setting, regenerate it with `cargo xtask settings-doc`; never edit it by hand.
- **hosts32** (Windows only): the 32-bit engine hosts build.
- **scripts**: shellcheck on the shell scripts, or PSScriptAnalyzer on the PowerShell scripts, when installed.

Useful options: `--only fmt,clippy` runs some steps, `--fail-fast` stops at the first failure, and `--dry-run` prints the commands.

CI also runs three checks that `dev-check` does not. Run them yourself when you change dependencies:

- **deps**: `cargo xtask deps --check`. The dependency direction between the workspace crates ([docs/dev/architecture.md](architecture.md#dependency-direction)), with cargo features resolved: the reader built with `--no-default-features` must not reach the conversion and citation stack.
- **notices**: `cargo xtask notices --check`. `THIRD-PARTY-NOTICES.md` is current. It needs `cargo-about`.
- **deny**: `cargo deny check`. Licences, advisories, duplicate versions, and sources, from `deny.toml`.

The writers' output is also checked by other tools, in `.github/workflows/second-tool.yml`. It runs weekly and on any change to the writers. The Markdown fixtures are converted to EPUB and PDF; each EPUB is checked with epubcheck 5.4.0, and each PDF with veraPDF 1.30.2 against the PDF/UA-1 profile. Both tools come from Maven Central, pinned and checked by SHA-256. Each line of the job summary starts with Pass, Fail, Allowed, or Warning, in words. A known warning that is accepted goes in `tools/second_tool_allowlist.txt`, with its reason.

The features: Linux CI uses `--all-features`, which includes `espeak`, `speechd`, and `omnivox`. Windows and macOS use `--features textweaver-speech/omnivox`.

## Tests

- Every crate has unit tests. Segmentation, offset maps, history, marker shifting, and editing also have property tests (`proptest`); loaders and renderers have snapshot tests (`insta`).
- Speech is tested with the `recording` backend and a fake clock, so timing tests never depend on the machine's speed. Engine hosts are tested against fake hosts that speak the real protocol.
- Tests never play audio aloud. Write audio to a temporary file, or use a silent output.
- Tests against real engines are ignored unless you ask for them with an environment variable: `TEXTWEAVER_ECI=1` (Eloquence, with licensed Voxin in the container), `TEXTWEAVER_SAPI=1` (Microsoft voices and eSpeak only), `TEXTWEAVER_APPLE=1` (macOS voices), `TEXTWEAVER_DECTALK=1` (a licensed DECtalk), `TEXTWEAVER_SPEECHD=1` (speech-dispatcher), `TEXTWEAVER_WHISPER_REAL=1` (an installed Whisper), and `TEXTWEAVER_WORD=1` (Microsoft Word opens a DOCX). Run them with `-- --ignored`.
- Never load Code Factory's Eloquence or OpenEVV in tests, and never commit audio made by an engine. Local samples go in the git-ignored `target-local/`.

## Benchmarks

Performance is a requirement ([ADR-0001](../adr/0001-workspace-and-dependencies.md)). Measure before and after you optimize a hot path.

```bash
cargo xtask bench
```

It times opening, first speech, navigation while reading, search, and entering edit mode on generated corpora of 1 MB and 10 MB, a 50,000-item list, and a 1 MB single line, and reports peak memory and the number of allocations. It also times the start of `tw --version`, `tw text`, `tw info`, and `tw backends` (`cargo xtask startup` runs only those). `--quick` skips the 10 MB corpus, `--only NAME` runs one measurement, `--file PATH` adds your own document, and `--json PATH` writes the numbers. [The audit](../history/audit-2026-09.md#benchmark-harness) describes the harness.

To compare with an earlier run, keep its JSON and pass it back:

```bash
cargo xtask bench --quick --json before.json
cargo xtask bench --quick --baseline before.json --max-ratio 2
```

The comparison fails when a peak heap or an allocation count grew more than `--max-ratio` times (2 by default). Those numbers barely change from run to run; times do, so times are only reported. CI runs this on every pull request against main's numbers (`bench.yml`).

`cargo xtask soak --minutes N` reads the 10 MB corpus with random navigation, pauses, rate changes, and edits, then from the top to the end, while a second reader's engine host is killed at random. It checks that the highlight only moves forward, reading finishes, memory stays level, and no engine host is left running. The nightly job runs it for 10 minutes.

### Measurements, Wave 4 (Monday, September 28, 2026)

Taken by Agent W4b on Windows (x86_64, 64 GB) while other agents were building, so times move by 30 percent or more from run to run; each is a median, repeated, and the spread is given where it matters. Allocation counts do not move once the harness waits for other threads (below).

**How they were taken.**

- Binaries: `cargo build --release -p textweaver-tui --bin textweaver`, then `cargo build --release -p textweaver-cli --bin tw`, each on its own (built together, cargo would give the reader `tw`'s features). `--no-default-features` on the first gives the reader without `publish`.
- Start-up: each command once to warm up, then 15 timed runs from PowerShell: `tw --version`, `textweaver --help`, and `tw info` on the 10 MB corpus. A program that does nothing takes 16 to 28 ms to start the same way.
- The reading paths: `cargo xtask bench --no-startup --json FILE`, on main and on the branch, with the same harness.
- Segmentation, find, and ropes: a standalone probe outside the workspace, on the bench corpora (numbers here; the ropes are in [ADR-0002](../adr/0002-text-model.md)).

**Binary size (release profile, bytes).**

- Before Wave 4b (main at c737694): `textweaver.exe` 46,827,520; `tw.exe` 52,799,488.
- After: `textweaver.exe` 47,332,864 (505,344 more); `tw.exe` 53,294,592 (495,104 more); `textweaver.exe` without `publish` 28,704,256, so export, preview, and citations are 18.6 MB of the reader.
- What the zip features cost, in a small program that reads and writes one member: deflate alone 549,376; with bzip2 644,096; with LZMA 575,488; with XZ 716,800 (lzma-rust2 0.16, a second copy beside sevenz-rust2's 0.21); with PPMd 590,848; all five 867,840. ICU4X's word and sentence data and code add about 58 KB.
- The bundled fonts (1.4 MB) are in the reader only with `publish`; the SCOWL list (692 KB) is always in it.

**Start-up.** `tw --version` and `textweaver --help` start in 23 to 35 ms, 7 to 10 ms above a program that does nothing, before and after. `tw info` on 10 MB takes 580 to 700 ms, mostly loading, unchanged. Before its first announcement the reader reads its settings and keys, builds the app (the speech engine already starts on a helper thread), asks the system for its color scheme, and opens the document on the command line. The color-scheme question was two `reg query` processes on every launch (35 to 90 ms); it is now asked only when it can change the theme, on a helper thread while the app is built. `--log debug` writes how long the settings and the whole build took.

**Segmentation and find, 10 MB of Markdown, per whole document.**

- Words: unicode-segmentation 265 to 430 ms, ICU4X 128 to 193 ms; sentences: 300 to 366 ms against 96 to 126 ms. Same boundaries on the corpus. textweaver now uses ICU4X (see `crates/textweaver-text/src/units.rs` for the lines that keep unicode-segmentation).
- Literal find of "the": regex 1.5 ms, memchr's `memmem` 1.3 ms. regex already searches literals with memchr, and the reader's find is case-insensitive by default, where `memmem` does not apply; not adopted.
- Many terms at once (19 words, whole words, any case): a regex alternation 656 ms, aho-corasick 43 ms. textweaver has no many-term highlighting yet; use aho-corasick when it does (the speech crate's abbreviation expansion is a candidate).

**The reading paths, `cargo xtask bench`, main then branch.**

- Narration plan of the whole document: 10 MB 1,155 to 631 ms, 1,141,829 to 844,757 allocations; 1 MB 102 to 66 ms; the one-line 1 MB file 95 to 53 ms; the 50,000-item list 738 to 577 ms.
- Open to first speech, 10 MB: 470 to 353 ms (loading moved as much between runs; the plan of the first window is the part that changed).
- Next sentence, idle, on the one-line file and the list: half the allocations (6,909 to 3,449; 7,266 to 3,660).
- Nothing grew past the gate except autosave on 10 MB, whose count moves between 11,163 and 75,386 in runs of the same code (the writer thread).

The harness now waits until no thread has allocated for 300 ms before each document, after the first speech, before search, and before leaving edit mode. Before that, background work from opening a document landed in whichever step it overlapped: one step gave 1,267 and 38,773 allocations in two runs of the same code.

Bulk conversion has its own benchmark:

```bash
cargo run --release -p textweaver-convert --example bench_convert
```

## See also

- [Building](building.md): setting up Rust and each system's libraries.
- [CONTRIBUTING.md](../../CONTRIBUTING.md#ci): the CI workflows.
- [Fuzzing](../../fuzz/README.md): the cargo-fuzz targets.
- [Documentation index](../README.md)
