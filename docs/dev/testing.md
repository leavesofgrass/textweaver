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

### Measurements, Wave 5 (Monday, September 28, 2026)

Taken by Agent W5r on the same machine while three other agents were building, so single runs moved by a factor of two or more. After [ADR-0034](../adr/0034-rope-after-measurement.md) kept the rope, W4b's two hot spots were measured part by part: loading a Markdown file, and planning the narration of a whole document.

**How they were taken.**

- Times: a standalone probe outside the workspace, built twice (main, then the branch) and run alternately on the bench corpora, reporting the fastest of nine runs. The fastest run is the one least disturbed by the other builds; medians moved too much to compare.
- Allocations and peak heap: `cargo xtask bench --no-startup` on main, then on the branch with `--baseline` and the default `--max-ratio 2`. The gate passed: no memory number grew more than 1.26 times, and none of the reading paths' counts rose.

**Where the time went, 10 MB of Markdown, before any change.**

- Loading, 420 ms: reading the file 5 ms; pulldown-cmark's parse 71 to 93 ms, done twice (once only to collect footnote definitions); the rest of the conversion, building the canonical text and markers, about 170 ms; the rope 12 ms; sorting the markers under 40 ms.
- The narration plan, 770 ms: finding the sentences 415 ms, of which walking the paragraphs was 100 ms and ICU4X's sentence segmentation 115 ms; the rest is per sentence (marker lookups, the spoken text and its map).
- Rope calls: a `line_range` cost about 1 microsecond (a char read for the line count, two `line_to_char`, one or two char reads), and segmenting a paragraph made four per line.

**What changed, and the numbers before and after.**

- Markdown loads in one parse: deferred footnotes, the default, are gathered from the pass that builds the text, and the builder passes words as slices instead of copying them a char at a time. Conversion of 10 MB 450 to 240 ms; loading 420 to 275 ms; the 50,000-item list 286 to 195 ms; allocations while loading 376,819 to 61,022 on 10 MB, 218,035 to 22,884 on the list, 37,246 to 5,917 on 1 MB.
- Line lookups: the document remembers whether its text ends with a line break, so counting lines reads nothing from the rope, and a paragraph's line breaks take one `line_to_char` per line. Walking every paragraph of 10 MB 100 to 35 ms; every sentence 415 to 295 ms.
- A list's items are counted near the list (`MarkerIndex::iter_within`), not by walking every item of that depth in the document at every list.
- Together, the whole-document plan: 10 MB 770 to 610 ms; the 50,000-item list 480 to 355 ms; 1 MB 61 to 57 ms. The plan's allocations did not change (844,757 on 10 MB).

**What is left.** ICU4X's segmentation (115 ms on 10 MB) is the floor for finding sentences. The rest of the plan's time is per sentence: three `enclosing` lookups, a `String` for each literal piece, and the offset map. A table of line starts would make every line lookup a binary search, but it would have to be kept up to date on every edit, which costs typing more than it saves reading; it was not done.

Bulk conversion has its own benchmark:

```bash
cargo run --release -p textweaver-convert --example bench_convert
```

## Automated screen-reader checks

The GUI's real test is the owner's listening sessions with NVDA, JAWS, and the Braille display. Between sessions, CI runs these checks ([ADR-0039](../adr/0039-automated-screen-reader-checks.md)). They never replace a session. Every line of their reports starts with a word: Pass, Fail, Warning, Changed, No baseline, or Heard.

**Never run a screen-reader session on your own machine.** The scripts drive NVDA or VoiceOver and refuse to run outside a CI runner, so no one's own screen reader is taken over.

### The accessibility tree, on every GUI change

The GUI workflow (`gui-xilem.yml`) dumps the window's accessibility tree on Windows, macOS, and Linux with accessibility-cli, built from a pinned commit. It opens `fixtures/t/reading.md` in the background with the silent `paced` backend. The run summary compares the tree with main's last successful run:

- **Pass:** the same elements as main.
- **Changed:** each added and removed line, in words. Check that every change was meant, such as a new button or a renamed setting.
- **No baseline:** no earlier tree on main to compare with (the first run, or artifacts that expired).
- **Fail:** no tree was dumped. The raw dump and the GUI's log are in the artifact.

The artifacts are `a11y-tree-windows`, `a11y-tree-macos`, and `a11y-tree-linux`. Each holds `tree.txt` (the normalized tree), `raw.json`, `raw-tree.txt` (accessibility-cli's own text view), and `gui.log`. To compare two trees yourself:

```bash
python3 tools/a11y/tree_report.py compare --system Linux --current new/tree.txt --baseline old/tree.txt
```

Running `tree-dump.sh` or `tree-dump.ps1` on your own machine is safe: it opens the GUI in the background, reads nothing aloud, and drives no screen reader. It needs accessibility-cli on the `PATH`, or its path in `A11Y_CLI` (`-Cli` in PowerShell).

### The screen-reader sessions, by hand

Run "Screen-reader checks" (`a11y-tests.yml`) from the Actions tab, and choose a session: `nvda`, `orca`, `voiceover`, or `all`. It also runs when its own files change. Each session opens `fixtures/t/reading.md`, reads about three sentences, pauses, moves to the next heading, opens a dialog, and closes it. What was said is checked against `fixtures/t/expected-phrases.json`.

- **nvda** (Windows): Guidepup starts NVDA and records its spoken phrases. The summary's first line answers "can Guidepup drive NVDA against the textweaver window?" with "Answer: yes" or "Answer: no".
- **orca** (Ubuntu, under Xvfb): the session is checked through AT-SPI events: the caret, the announcements, the focus. Orca runs beside it, and the report lists what it said at each step.
- **voiceover** (macOS 14): the same with VoiceOver, if Guidepup can enable it on the runner. If not, the setup log says why.

The sessions are report-only for now: a failure shows in the summary but does not fail the workflow. The phrases NVDA spoke are in the `a11y-nvda` artifact (`phrases.json` and `report.md`): the owner compares them with what they heard in their own session.

Guidepup and its setup tool are locked, with integrity hashes, in `tools/a11y/package-lock.json`. To move to a new version, change `tools/a11y/package.json` and regenerate the lock file with `npm install --package-lock-only --ignore-scripts` in `tools/a11y`, in a container or on a machine with Node. Nothing else in the project needs Node.

## See also

- [Building](building.md): setting up Rust and each system's libraries.
- [CONTRIBUTING.md](../../CONTRIBUTING.md#ci): the CI workflows.
- [Fuzzing](../../fuzz/README.md): the cargo-fuzz targets.
- [Documentation index](../README.md)
