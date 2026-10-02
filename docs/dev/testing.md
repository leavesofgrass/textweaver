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
- **pseudo**: `cargo test -p textweaver-app --test it -- pseudo_locale::`. The interface in the pseudo-locales en-XA and ar-XB: it fails on any message that does not come from the translation catalog ([ADR-0030](../adr/0030-interface-translations.md)). CI runs it in the docs job.
- **generated**: `cargo xtask regen --check`. Every generated file is current. It runs each check below, even after one fails, and reports each on one line, meaning first, such as "notices: pass" or "site data: FAIL, out of date; run cargo xtask regen". A check whose tool is missing (cargo-about, or Python) is reported as skipped. `--only keyboard` or `--only site` selects this step. The checks, in order:
  - **notices**: `THIRD-PARTY-NOTICES.md` matches `cargo about` and the license files in `third_party/` (as `cargo xtask notices --check`). It needs `cargo-about`.
  - **settings reference**: [the settings reference](../settings-reference.md) matches the settings schema (as `cargo xtask settings-doc --check`). It builds the app crate's tests and runs their `settings_reference` module.
  - **keyboard**: [docs/keyboard.md](../keyboard.md) matches the keymap (as `cargo xtask keyboard --check`).
  - **site data**: the data embedded in the `docs/site` pages matches `cargo metadata`, the keymap, and the theme files (as `tools/gen_site_data.py --check`, run with `py -3` on Windows and `python3` elsewhere).
  - **docs**: the ADR index and the Decisions list in [docs/README.md](../README.md#decisions) list every ADR once, in number order; the crate counts match `crates/`; every guide ends with a "See also" section and is linked from the index (as `cargo xtask docs --check`).

  Never edit a generated file by hand. After changing a key, a setting, a dependency, a crate, or the site's sources, run `cargo xtask regen` to rebuild them all, in the right order, and commit what changed. It rewrites the crate counts too, but an ADR list or a See also section it only reports, for you to fix by hand.

The Python checks are written `python3` below. On Windows, run them with `py -3` instead (`py -3 tools/check_links.py`): `python` and `python3` there may be the Microsoft Store stub.

- **links**: `python3 tools/check_links.py`. Every relative link and anchor in the Markdown docs and in `docs/site` must resolve.
- **site-a11y**: `python3 tools/check_site_a11y.py`. Static accessibility checks of the `docs/site` pages: language, title, one level-1 heading and no skipped levels, the skip link, landmarks, a label for every control, text alternatives, and references that resolve.
- **hosts32** (Windows only): the 32-bit engine hosts build.
- **scripts**: shellcheck on the shell scripts, or PSScriptAnalyzer on the PowerShell scripts, when installed.

Useful options: `--only fmt,clippy` runs some steps, `--fail-fast` stops at the first failure, and `--dry-run` prints the commands.

CI also runs two checks that `dev-check` does not. Run them yourself when you change dependencies:

- **deps**: `cargo xtask deps --check`. The dependency direction between the workspace crates ([docs/dev/architecture.md](architecture.md#dependency-direction)), with cargo features resolved: the reader built with `--no-default-features` must not reach the conversion and citation stack.
- **deny**: `cargo deny check`. Licenses, advisories, duplicate versions, and sources, from `deny.toml`.

The writers' output is also checked by other tools, in `.github/workflows/second-tool.yml`. It runs weekly and on any change to the writers. The Markdown fixtures are converted to EPUB and PDF; each EPUB is checked with epubcheck 5.4.0, and each PDF with veraPDF 1.30.2 against the PDF/UA-1 profile. Both tools come from Maven Central, pinned and checked by SHA-256. Each line of the job summary starts with Pass, Fail, Allowed, or Warning, in words. A known warning that is accepted goes in `tools/second_tool_allowlist.txt`, with its reason.

**Running epubcheck and veraPDF locally.** You do not need Java installed: run both tools inside a throwaway Java container with Docker, the same way CI runs them, but without CI's Maven setup. First build `tw` and convert a fixture:

```bash
cargo build --release -p textweaver-cli --bin tw
mkdir -p target/second-tool
target/release/tw convert fixtures/sample.md --to epub --out target/second-tool --no-pandoc
target/release/tw convert fixtures/sample.md --to pdf --out target/second-tool --no-pandoc
```

Then, with Docker, fetch and run epubcheck 5.4.0 (checked against the SHA-256 in `.github/workflows/second-tool.yml`) against the EPUB:

```bash
docker run --rm -v "$PWD/target/second-tool:/work" -w /work eclipse-temurin:21-jre@sha256:d7051a45dd955e4d5d1db4d3f4269fe13d1c6dff8cc6b7ef89fc8577b96c1982 sh -c '
  set -eu
  curl -sSfL -A "textweaver-research (+https://github.com/leavesofgrass/textweaver)" \
    -o epubcheck.jar https://repo1.maven.org/maven2/org/w3c/epubcheck/5.4.0/epubcheck-5.4.0.jar
  echo "261cd3ba8f841b4a64ebb6ea9c8aa30ed2801e8764a1000123c66bf38bfe4066  epubcheck.jar" | sha256sum -c -
  java -jar epubcheck.jar sample.epub
'
```

And veraPDF 1.30.2 against the PDF, with the same profile CI uses (`-f ua1`):

```bash
docker run --rm -v "$PWD/target/second-tool:/work" -w /work eclipse-temurin:21-jre@sha256:d7051a45dd955e4d5d1db4d3f4269fe13d1c6dff8cc6b7ef89fc8577b96c1982 sh -c '
  set -eu
  apt-get update -qq && apt-get install -y -qq unzip zip
  curl -sSfL -A "textweaver-research (+https://github.com/leavesofgrass/textweaver)" \
    -o verapdf.zip https://repo1.maven.org/maven2/org/verapdf/apps/installer/1.30.2/installer-1.30.2-installer.zip
  echo "dfe2bab9f6a5cd5b603093c3ede98aa285348dbaa4d2e5e4b4c1428973f6dab0  verapdf.zip" | sha256sum -c -
  unzip -q verapdf.zip -d verapdf
  # Installs non-interactively; see .github/workflows/second-tool.yml for the full auto-install.xml.
  java -jar verapdf/verapdf-greenfield-1.30.2/verapdf-izpack-installer-1.30.2.jar -options-system
  ./verapdf/app/verapdf -f ua1 --format text sample.pdf
'
```

For the exact, always-current versions, checksums, and the full non-interactive install answers, read `.github/workflows/second-tool.yml`, which this is a local equivalent of. `target/second-tool/` is build output; nothing there is tracked.

The features: Linux CI uses `--all-features`, which includes `espeak`, `speechd`, and `omnivox`. Windows and macOS use `--features textweaver-speech/omnivox`.

## Tests

- Every crate has unit tests. Segmentation, offset maps, history, marker shifting, and editing also have property tests (`proptest`); loaders and renderers have snapshot tests (`insta`).
- Speech is tested with the `recording` backend and a fake clock, so timing tests never depend on the machine's speed. Engine hosts are tested against fake hosts that speak the real protocol.
- Tests never play audio aloud. Write audio to a temporary file, or use a silent output.
- Tests against real engines are ignored unless you ask for them with an environment variable: `TEXTWEAVER_ECI=1` (Eloquence, with licensed Voxin in the container), `TEXTWEAVER_SAPI=1` (Microsoft voices and eSpeak only), `TEXTWEAVER_APPLE=1` (macOS voices), `TEXTWEAVER_DECTALK=1` (a licensed DECtalk), `TEXTWEAVER_SPEECHD=1` (speech-dispatcher), `TEXTWEAVER_WHISPER_REAL=1` (an installed Whisper), and `TEXTWEAVER_WORD=1` (Microsoft Word opens a DOCX). Run them with `-- --ignored`.
- Never load Code Factory's Eloquence or OpenEVV in tests, and never commit audio made by an engine. Local samples go in the git-ignored `target-local/`.

### Where tests go

Unit tests live beside the code, in a `#[cfg(test)] mod tests`. Integration tests, the ones that use a crate only through its public interface, go in one test program per crate:

- `tests/it/main.rs` is the program. It holds only `mod` lines, one per module.
- Each module is a file beside it: `tests/it/sync.rs` is `mod sync;`. A module that needs a feature gets `#[cfg(feature = "...")]` on its `mod` line, or `#![cfg(...)]` at the top of its file.
- Helpers shared by several modules are a module of their own, such as `tests/it/common.rs`, used as `use crate::common;`.
- Snapshots (`insta`) are in `tests/it/snapshots/`, named `it__<module>__<name>.snap`. Property-test regressions are beside their module, `tests/it/<module>.proptest-regressions`.

**A new integration test is a module in `tests/it/`, never a new file directly in `tests/`.** Cargo makes every file in `tests/` a program of its own, and each one links the crate and all its dependencies again; on Windows the linking dominated the test build. The one exception is a program with `harness = false` in its `Cargo.toml` (the Apple voices, `textweaver-apple/tests/voices.rs`, and the engine host's `host_process.rs`), which has its own `main`.

To run one module, name the program and filter by the module's path:

```bash
cargo test -p textweaver-app --test it -- sync::
cargo test -p textweaver-app --test it -- sync::a_note_made_on_one_computer_appears_on_the_other --exact
```

A test that must be alone in its process (it changes something process-wide, such as an environment variable or a global flag) runs itself as a child process: `std::env::current_exe()` with its full name, module included, and `--exact` (see `textweaver-convert/tests/it/pandoc_env.rs`). The nightly's nextest runs every test in its own process anyway, but `cargo test` runs a program's tests on threads of one process.

## Benchmarks

Performance is a requirement ([ADR-0001](../adr/0001-workspace-and-dependencies.md)). Measure before and after you optimize a hot path.

```bash
cargo xtask bench
```

It times opening, first speech, navigation while reading, search, and entering edit mode on generated corpora of 1 MB and 10 MB, a 50,000-item list, and a 1 MB single line, and reports peak memory and the number of allocations. It also times the start of `tw --version`, `tw text`, `tw info`, and `tw backends` (`cargo xtask startup` runs only those). `--quick` skips the 10 MB corpus, `--only NAME` runs one measurement, `--file PATH` adds your own document, and `--json PATH` writes the numbers. The September 2026 audit (kept outside the repository) describes the harness, and `xtask/src/bench.rs` lists every measurement.

Per document it also measures, each as one line of the report and one JSON key:

- **Segmentation:** every sentence and every word through `Units`, on their own (`sentences_ms`, `words_ms`, with peak heap and allocations). The plan's number hides them.
- **Normalization:** the first 2,000 utterances of the plan through the default pipeline (`normalize_ms`, and `normalize_pipeline_ms` to build it).
- **An edit on the loaded document:** one insert in the middle with the markers kept (`apply_ms`), then the blank-line table rebuilt (`blank_lines_ms`).
- **Stop to speak:** Stop, then Read from cursor, until the backend is handed the first utterance, 20 times (`stop_to_speak`): the reader's share of a restart.
- **Stop to first audio:** the same on the recording backend playing in real time, until its first audio starts, as the speech service stamps it (`stop_to_first_audio`; `stop_to_first_audio_service` is the speech thread's share). The speech service records when each reading first sounds (`SpeechService::first_audio`).

**Stop to first audio with a real engine.** `--engine piper` or `--engine sapi` (Windows) adds the same measurement with that engine, playing to a silent output that takes samples in real time, so nothing is heard; the numbers go in the `first-audio` entry. Piper needs `TEXTWEAVER_PIPER_VOICES` naming a folder of installed voices, and is timed whenever that is set; nothing is ever downloaded, and with no voices the engine is skipped with a line that says why. The target, from the [next waves plan](research/next-waves-plan.md#principles-for-the-next-waves), is under 150 ms on a 2-core laptop with Piper.

**Pathological inputs.** One generated file per loader (plain text, Markdown, HTML, LaTeX, RTF, email, MHTML, CSV, JSON, notebooks, flat ODT, SVG, MathML, DOCX, EPUB, and PDF), each holding a 512 KB token with nothing to break on, lists nested 200 deep (or the format's nearest thing: RTF groups, JSON arrays, SVG groups, MathML rows), a 2,000-row table, and a 1 MB line. Each must load and plan within a ceiling, 10 seconds by default (`--ceiling-s`), or the run fails; so does a loader that panics. A loader that refuses a file with an error passes, and says so. The fuzz targets catch crashes; this catches slowness, such as Star's 34 seconds to wrap one 5 MB token. `--no-pathological` skips them; `--only pathological` runs only them. The generators are in `xtask/src/pathological.rs`. The first run found one: in MathML, a long token inside 200 nested rows takes time that grows faster than its length (8 KB 0.1 s, 32 KB 0.7 s, 512 KB 189 s). Until the loader is fixed, the MathML input keeps its long token beside the nested rows; put it back inside with the fix.

### The gate: a two-way ratchet

CI runs `cargo xtask bench --quick --baseline xtask/bench-baseline.json` on every pull request and every push to main (`bench.yml`). The baseline is a committed file with one entry per platform; each entry is a whole report, with the date, the commit, and why it was written. The ratchet (`xtask/src/ratchet.rs`):

- **Memory** (peak heap and allocation counts) fails when it grows more than 25 percent over the baseline, and also when it falls more than 25 percent below it. A gain must be written into the baseline, or the floor goes stale and a later regression hides under it.
- **Times** fail when they grow more than 50 percent over the baseline, but only against an entry measured on the runner type the gate runs on (its `gate_times` is true), only above 5 ms, and only by more than 2 ms. Times that improve are reported, never failed: a quiet runner is not a change in the code. Disk writes, process starts, document identity, and the pathological inputs (which have their ceiling) are reported only.
- Numbers too small to matter are not gated: peak heap under 1 MB, fewer than 5,000 allocations, or growth smaller than those.

The limits are in the baseline file's `policy`, so changing them is a commit that says why. Each line of the report puts the document first and the change in words: "md-1mb.md: load_allocs grew 31 percent, from 67862 to 88900; the limit is 25 percent."

**A failed run never becomes the baseline.** Only an explicit update writes it:

```bash
cargo xtask bench --quick --update-baseline --reason "the plan reuses its buffers"
```

It writes this platform's entry and keeps the others. It refuses without a reason, and refuses off `main` when the platform already has an entry; a platform's first entry may be written from any branch. Commit the file with a message that says why the numbers moved. The CI runner's own entry comes from running "Bench" by hand on main with "update the baseline" ticked and a reason: it adds `--gate-times` and uploads the new file as the `bench-baseline` artifact, to commit. Until that entry exists, the Linux entry in the file comes from the development container, with `gate_times` false: memory is gated, times are reported.

To compare two local runs, as before, keep a report and pass it back; a plain report as `--baseline` is the old one-way check, which fails when memory grew more than `--max-ratio` times (2 by default):

```bash
cargo xtask bench --quick --json before.json
cargo xtask bench --quick --baseline before.json --max-ratio 2
```

### GUI frame times

```bash
cargo xtask frames --json frames.json
cargo xtask frames --baseline xtask/frames-baseline.json
```

It builds the Xilem GUI in release mode with the `alloc-count` feature (a counting allocator, never in a package) and runs `textweaver-xilem --measure-frames 200` eight times: on the 1 MB corpus and on the 1 MB one-line corpus, plain and with every reading aid on (bionic reading, difficult words, syllables, the ruler with its band, text spacing, and RSVP), at 100 and 200 percent. Each run reads aloud on the silent paced backend at 900 words per minute, with no window on screen, and measures 200 moves of the spoken word the way the window makes them: the driver's refresh through `gui::Refresher` (the same `refresh_host` the window runs, which sends the spoken word and the sentence band), then Masonry's layout, paint, and accessibility passes over the whole widget tree. Per run it reports, in words, the median, 95th percentile, and worst move, the median refresh and passes, and the allocations and accessibility nodes per move; it also says how many moves had the sentence band.

The counts do not move with the machine's load, so they are what the gate checks: `--baseline xtask/frames-baseline.json` runs the same two-way ratchet as the bench on `moves_allocs` and `moves_nodes` (25 percent either way), and `--update-baseline --reason TEXT` writes this platform's entry. Times are reported; on a busy machine they move by 30 percent or more. `--max-worst MS` fails any run whose worst move is over a ceiling; ADR-0027's 30 ms is the hard one. `--quick` runs the 1 MB corpus at 100 percent only. Not included: rasterizing on the GPU, the UI thread's wait for it, and the platform's accessibility adapter; those are measured by hand in a real window (below). The nightly job runs it and puts the lines in its summary.

In a real window, `--log` (or `--log-file PATH`) now writes:

- the startup phases, each as "startup: PHASE at N ms" from the start of `run`: settings and app, the color-scheme probe (when it runs), the widget tree, the event loop, the window shown at its first tick, and the document open;
- every 200 highlight moves, one line with the driver's refresh time per move: median, 95th percentile, and worst. This replaces a pushed zero that nothing read.

The speech service writes, at debug level (`--log debug` in the terminal reader), one line every 200 words that the audio clock scheduled: how late they fired (median and 95th percentile) and how far apart the speech thread's timer steps were. On Windows the 10 ms waits round up to the system timer tick, so a word can light up a tick late; this line measures it before anything changes.

### The nightly profile

The nightly job `profile` records `perf` profiles on Linux of `tw info` on the 10 MB corpus and of the narration plan of the same corpus (`cargo xtask bench-run --profile-plan --only md-10mb`, which only loads and plans), built in release mode with line tables and symbols kept. The `profile` artifact holds a flame graph of each (made by flamegraph 0.6.14, pinned) and, because a flame graph is a picture, the same profile as a plain-text list of the hottest functions; the first 25 lines of each list are in the job summary.

`cargo xtask soak --minutes N` reads the 10 MB corpus with random navigation, pauses, rate changes, and edits, then from the top to the end, while a second reader's engine host is killed at random. It checks that the highlight only moves forward, reading finishes, memory stays level, and no engine host is left running. The nightly job runs it for 10 minutes.

### Benchmark history: September 28, 2026

Taken on Windows (x86_64, 64 GB) while other builds were running on the same machine, so times move by 30 percent or more from run to run; each is a median, repeated, and the spread is given where it matters. Allocation counts do not move once the harness waits for other threads (below).

**How they were taken.**

- Binaries: `cargo build --release -p textweaver-tui --bin textweaver`, then `cargo build --release -p textweaver-cli --bin tw`, each on its own (built together, cargo would give the reader `tw`'s features). `--no-default-features` on the first gives the reader without `publish`.
- Start-up: each command once to warm up, then 15 timed runs from PowerShell: `tw --version`, `textweaver --help`, and `tw info` on the 10 MB corpus. A program that does nothing takes 16 to 28 ms to start the same way.
- The reading paths: `cargo xtask bench --no-startup --json FILE`, on main and on the branch, with the same harness.
- Segmentation, find, and ropes: a standalone probe outside the workspace, on the bench corpora (numbers here; the ropes are in [ADR-0002](../adr/0002-text-model.md)).

**Binary size (release profile, bytes).**

- Before (main at c737694): `textweaver.exe` 46,827,520; `tw.exe` 52,799,488.
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

### Benchmark history: a later pass, same day

Taken on the same machine while three other builds were running, so single runs moved by a factor of two or more. After [ADR-0034](../adr/0034-rope-after-measurement.md) kept the rope, the two hot spots found above were measured part by part: loading a Markdown file, and planning the narration of a whole document.

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

### Speech latency: Friday, October 2, 2026

Wave 8b set the sound output's buffer (`crates/textweaver-enginehost/src/audio.rs`, `DEFAULT_OUTPUT_BUFFER_MS`). Before, the playback client left the size to rodio, whose `from_device` asks for about 50 ms rounded to a power of two in frames: 2,048 frames, 42.7 ms at 48 kHz. Every engine whose audio textweaver plays (SAPI 5, Eloquence, DECtalk, Piper) goes through that output.

**How they were taken.** On Windows (WASAPI, shared mode, the default output at 48 kHz), with other agents' builds running, by two ignored tests that open the real device but play only silence (zero samples at zero gain):

```bash
TEXTWEAVER_AUDIO_PROBE=1 cargo test -p textweaver-enginehost --features playback --test it -- latency --ignored --nocapture
TEXTWEAVER_SAPI=1 TEXTWEAVER_AUDIO_PROBE=1 cargo test -p textweaver-sapi --test it -- real_voices::stop_and_restart --ignored --nocapture
```

The first opens the device three times per buffer size and, each time, restarts 11 times: the feed cleared and new audio pushed at once, as from an engine that answers instantly. It reports the median time to the first callback that takes the new audio (when the playback client reports `Started`), the lead (how far the samples taken run ahead of the sound, which is the audio still in the device's buffer), and the drift of the lead over two seconds, which falls for good when a device runs dry. The time to the first sound is the first callback plus the lead; a stop is heard after the lead.

**The output alone, median of the restarts.**

- The library's size (before): to the first sound 42 to 45 ms (first callback 5 to 7 ms, lead 37 ms).
- 40 ms: 40 to 41 ms (lead 35 ms).
- 30 ms, the new default: 30 to 31 ms (first callback 5 ms, lead 25 ms).
- 20 ms: 22 to 23 ms (lead 17 ms).
- The drift stayed within 1 ms at every size, also while a workspace build loaded every core, so no size ran dry. With a busy machine a stream can start late, which shows as a low lead in one open (it never moves the drift); the highest of the three opens is reported.

30 ms was chosen over 20 ms: it saves 12 to 14 ms on every first word and every stop, and keeps a lead of at least 18 ms over WASAPI's 10 ms period, where 20 ms keeps 11 ms. Windows does not report underruns, so the margin is the only guard there. ALSA and JACK do report them; an output that reports two opens again between readings with twice the buffer, up to 100 ms (`UNDERRUNS_TO_GROW`, `MAX_OUTPUT_BUFFER_MS`).

**With SAPI 5 (Zira Desktop, volume 0),** from `stop` and a new `speak` to `Started`, median of 9, in the quietest of three runs: 18.3 ms with the library's size and 18.1 ms with 30 ms (fastest 17 ms; the busier runs gave medians of 25 to 47 ms, with single restarts up to 139 ms either way). Adding the lead, a SAPI restart is heard after about 56 ms before and 43 ms after. The engine's own time to its first audio is most of it and moves from run to run; the buffer changes what follows `Started` (the lead above), not `Started` itself. W8b-i's probe measures the same span through the speech service.

**Also in this change, checked without a device.**

- A push that arrives while the output plays silence is taken at the next sample, not after the rest of a 64-sample batch (up to 3 ms at 22,050 Hz), and a stop drops the samples the output had taken but not yet played (`FeedReader`, tests in `crates/textweaver-enginehost/tests/it/feed.rs`).
- A minute of speech through a fake device with the 30 ms and the 20 ms buffer, fed by a fake engine at a real-time factor of 0.1 to 0.5 in chunks of 2,048 to 4,096 samples: zero gaps, and every sample once, in order. Time is simulated, so the test never waits.

### Benchmark history: Friday, October 2, 2026, the first instrumented numbers

The "before" numbers for the alpha.8 performance work, taken with the new measurements on the same Windows machine (x86_64, 12 threads, 64 GB) while three or four other agents were building, so times moved by 30 percent or more between runs; allocation and node counts do not. `cargo xtask bench --quick` (twice), `cargo xtask bench --only md-1mb --engine sapi` with `TEXTWEAVER_PIPER_VOICES` set, and `cargo xtask frames`. The same quick run in the development container wrote the Linux baseline entry.

**Segmentation, normalization, and an edit, 1 MB of Markdown.** Every sentence through `Units` 21 to 49 ms and 21,532 allocations; every word 33 to 64 ms and 37,199 allocations; the first 2,000 utterances normalized 24 to 34 ms and 133,914 allocations (67 per utterance), after 3 to 6 ms to build the pipeline; one insert in the middle 0.4 to 0.9 ms and 6 allocations, then the blank-line table rebuilt in 1.7 to 2.3 ms. On the 50,000-item list (5.7 MB): sentences 179 to 272 ms, words 268 to 468 ms and 416,434 allocations, an insert 2 ms, the blank-line rebuild 12 to 16 ms.

**Stop to first audio.** The reader's share (Stop, then Read from cursor, until the backend is handed the first utterance) is under 2 ms at the median on every corpus. On the recording backend playing in real time, key to first audio is 0.2 to 4 ms, of which the speech thread's own share is 0.1 to 0.3 ms; the rest is the backend's poll. With real engines on a silent output, on the 1 MB corpus: **SAPI 5 39 ms** at the median (58 ms worst), nearly all of it the engine host; **Piper 830 to 980 ms** at the median (1.0 to 1.6 s worst), on the medium Joe voice, nearly all of it the first chunk's synthesis. The target is under 150 ms on a 2-core laptop with Piper; Piper is five to six times over it on this machine, before any work on it.

**Pathological inputs.** All sixteen loaders load and plan each file in 0.1 to 0.8 seconds, well under the 10-second ceiling, except one: the MathML loader took **189 seconds** with the 512 KB token inside 200 nested rows (8 KB took 0.1 s and 32 KB 0.7 s, so it grows faster than the token). The input now keeps the token beside the rows until the loader is fixed.

**GUI frame time, 200 moves each** (median, 95th percentile, worst; allocations and nodes per move):

- 1 MB Markdown, plain: 1.1 to 1.3 ms, 3.8 to 7.5 ms, up to 70 ms; 612 to 630 allocations and 6.5 to 6.8 nodes. At 200 percent about the same.
- 1 MB Markdown, every aid on: 3.2 to 4.6 ms, 8.7 to 15.8 ms, up to 78 ms; about 2,100 allocations and 8.9 nodes.
- 1 MB one-line text, plain: **70 to 74 ms**, 122 to 179 ms, up to 550 ms; 50,200 allocations and **1,442 nodes** per move: the whole one-paragraph window's runs are rebuilt and resent on every word (the performance report's G5).
- 1 MB one-line text, every aid on: **412 to 504 ms**, 628 to 1,524 ms, up to 2.1 s; 216,000 to 242,000 allocations and 1,549 to 1,731 nodes.

Every move had the sentence band. The one-line corpus is over ADR-0027's 30 ms ceiling by two to seventeen times; the Markdown corpus is well under it. The first probe run on the one-line corpus also found a panic: a layout line start inside a two-byte character (fixed in `caret::char_of`).

**Startup** (`tw`, unchanged code): `tw --version` 34 ms, `tw text` 75 ms, `tw info` on 1 MB 131 ms, `tw backends` 44 ms, at the median.

### Benchmark history: October 2, 2026, the narration plan and loading

Items 1 to 6 of the ranked plan in the [performance audit](research/performance-audit.md), and two from the [next waves plan](research/next-waves-plan.md). Taken on Windows (x86_64, 64 GB) while three other agents were building, so times moved by up to a factor of two between runs of the same code; allocation counts and peak heap did not move at all.

**How they were taken.**

- The reading paths: `cargo xtask bench --only md-10mb --no-startup --json FILE`, three runs after each change, median reported.
- Edits, the SSML markup, and the interface catalogs, which the bench does not measure: small release probes outside the committed code. Each built once before and once after the change, and the two run alternately where times were close.

**The narration plan, 10 MB of Markdown, whole document (111,125 utterances).**

| Change | Allocations | Peak heap | Time (median of three) |
| --- | --- | --- | --- |
| Before | 836,038 | 69.7 MB | 624 ms |
| Literal text pushed from the rope's chunks, no `String` per piece | 713,256 | 70.0 MB | 556 ms |
| One buffer per utterance's text and map; pieces, announcement, and split reused; no per-sentence `Vec` | 538,984 | 58.8 MB | 509 ms |

What is left is about two allocations per utterance (its text and its map, which the speech service keeps) and the sentence segmentation (item 9 of the audit).

**Loading, 10 MB of Markdown.** The canonical text is sized from the source's length, words are counted by length when they are ASCII, and code is not copied twice: allocations 67,950 to 66,122; load 334 to 248 ms in the bench (an interleaved probe of five rounds gave 365 to 360 ms, within the noise); peak heap 72.1 to 73.1 MB, since the full-size buffer exists from the start.

**Edits, 10 MB of Markdown (159,155 markers), median of 60 edits per run, three runs.**

- `Document::apply`: 2.77 to 1.14 ms. Markers past the edit move by an addition, markers before it are left alone, and only the few reaching into it are mapped and re-sorted. What is left is walking the markers' 11 MB.
- The blank-line table after an edit: rebuilt on next use, 19.3 ms, before; updated inside `apply`, under 0.05 ms, after. An edit plus the table went from 22.4 to 1.17 ms.

**Opening to the first Read.** Between opening and the first Read being handed to the speech service, 25 to 35 ms in every bench run, before and after (`open_and_read_dispatch_ms` less `open_ms`). Planning the first window is not the cost: 1.9 ms for 407 utterances, 0.07 ms for two sentences, so planning a smaller first window was not done. The cost is the lazy tables: the blank-line table (23 to 48 ms under load) and the marker tables (3 to 4 ms). Opening in the background now builds both on the loading thread. The bench opens synchronously, so its number does not show this.

**Speech and start-up.**

- SSML and DECtalk markup compiled five regular expressions per sentence: 324 to 534 microseconds a sentence before, 4 to 7 after, on the speech thread of SSML engines.
- The SCOWL list was already unpacked on first use (Wave 6). The interface catalogs were not: the first built-in translation asked for parsed all five. Starting in Spanish: 41 to 6 ms for the catalog; English is unchanged at 6 ms.
- `cargo xtask startup` after the pass, in English (no change expected there): `tw --version` 37.9 ms, `tw text` 58.3 ms, `tw info` 91.9 ms on the 1 MB corpus, `tw backends` 38.4 ms (medians of five).

Bulk conversion has its own benchmark:

```bash
cargo run --release -p textweaver-convert --example bench_convert
```

## Braille, real engines, and timing

These checks run on CI runners. Nothing in them plays audio or drives a screen reader.

### Braille against liblouis

The braille job in `second-tool.yml` checks BRF files with [liblouis](https://liblouis.io/), the translator most braille software uses. It builds liblouis from its release tarball, pinned by version and SHA-256 in `tools/braille-check/install-liblouis.sh`, and then:

- runs the BRF writer's tests with `lou_translate` installed, so the grade 2 test takes its liblouis branch;
- writes each Markdown fixture without math as BRF in UEB grade 1 (textweaver's own translator) and grade 2 (through liblouis), with `tools/braille-check`, a small program outside the workspace that turns on the writers' `liblouis` feature;
- reads each file back to print with liblouis and compares its words with the document's, in order (`tools/braille-check/compare.py`).

Words the writer adds on purpose, an ordered list's numbers and "checked" or "not checked" for task items, are reported as Allowed. Reading braille back is ambiguous, above all in grade 2 (a lone "m" reads back as "more"), so the limit is set by liblouis itself: the same text through liblouis alone, forward and back. A fixture fails when its file reads back more than 2 points worse than that, or below 90 percent. The job also reports, without failing, how many braille words match liblouis's own translation, and where they differ; for grade 1 that compares the two translators. `results.txt` in the `second-tool-braille` artifact has every line, the differences as Detail lines.

`lou_translate` reads backslash escapes (`\n`, `\x41`) in its input, so everything sent to it, by the check and by the BRF writer, has each backslash doubled. In BRF a backslash is the "ou" cell, common in grade 2 ("AL\D1" is "aloud"); sent as it was, a line with an invalid escape read back as nothing.

To run it yourself, use the development container (liblouis builds in about a minute; the program takes a few more):

```bash
docker compose run --rm dev sh -c '
  sh tools/braille-check/install-liblouis.sh /tmp/liblouis &&
  PATH=/tmp/liblouis/bin:$PATH LD_LIBRARY_PATH=/tmp/liblouis/lib CARGO_TARGET_DIR=/tmp/bc \
    sh tools/braille-check/run.sh target/braille'
```

The results are in `target/braille/results.txt`. Keep `CARGO_TARGET_DIR` inside the container: on Windows, a build folder on the shared drive is slow enough to stall a build script.

### Real engines

`engines.yml` reads `fixtures/t/reading.md` into a WAV file with `tw export-audio --json`, through each engine a hosted runner has:

- **espeak-ng** on Linux;
- on Windows, a **SAPI 5** voice in the 64-bit host, a **OneCore** voice through SAPI 5, and a voice in the **32-bit host**, which it also starts on its own to list its voices;
- **AVSpeech** on macOS 14 and macOS 15.

`tools/engines/check_export.py` checks each file. A step fails only when the engine could not export at all: the file or its report is missing, it holds no audio, or the export fell back to another engine. The rest measures rather than proves, so it is reported as Pass or Warning and never fails, until runs give each engine a baseline: the file's length against the timeline, its level, the speed, and word times for at least 90 percent of the words, rising and inside the audio. A file with no word times at all gets one Warning saying so; the Apple backends write their files that way today. A voice the runner image lacks (it has no 32-bit voice) is reported as "Skipped", with the voices it has, and passes. Only Microsoft voices and eSpeak are used on Windows (`tools/engines/pick_voice.py`).

It also measures, without failing, where the word times fall against the audio's own silences: at each sentence start, how far the first word's time is from the end of the nearest silence, and how many word starts fall deep inside one. That is where drift between an engine's word events and its audio shows first; for AVSpeech, whose word times come from interleaved callbacks, a new macOS is where to look.

The workflow runs on changes to the speech crates, weekly, and by hand. Started by hand with "sanitizer", it adds a one-time pass of Valgrind's memcheck over `tw` while espeak-ng reads the fixture: the audio path that crosses into a C library. It is a check to record in ADR-0039, not a standing job.

### Tests that wait: order, not the clock

A test that asserts how long something took fails on a loaded machine, however generous the limit. Check the order of events instead: that `speak` returned before the engine host was ready, that a cancelled utterance got Cancelled and never Started. When a test must wait, poll until the condition holds, with a long limit that only catches a hang, and say so in the test.

A test of a timeout moves the clock itself. The engine host's start deadline, stall timer, and the reopen wait of a stalled sound output read a `textweaver_enginehost::Clock`: `Clock::manual()` stands still until the test calls `advance`, so the test checks just before the deadline and just after it, with no real waiting (`HostStart::begin_with_clock`, `HostProcess::set_clock`, `Playback::set_clock`). For the app's background writer, the test-only `Job::Hold` stalls the disk until the test releases it, so a test can check that something finished while the disk was still stalled.

Silent outputs that run faster than real time make a whole utterance last a fraction of a second, and a starved output thread then plays it in one burst. A test that must act in the middle of an utterance (a pause) runs at real time, or tries again when the machine held it off too long.

To trust a fix to a flaky test, run it 40 times under load: fewer runs cannot tell a 10 percent flake from a fix. The fake-host suites were checked by running 8 copies of each at once, 5 times.

## Automated screen-reader checks

The GUI's real test is a human listening session with NVDA, JAWS, and a Braille display. Between sessions, CI runs these checks ([ADR-0039](../adr/0039-automated-screen-reader-checks.md)). They never replace a session. Every line of their reports starts with a word: Pass, Fail, Warning, Changed, No baseline, or Heard.

**Never run a screen-reader session on your own machine.** The scripts drive NVDA or VoiceOver and refuse to run outside a CI runner, so no one's own screen reader is taken over.

### The accessibility tree, on every GUI change

The GUI workflow (`gui-xilem.yml`) dumps the window's accessibility tree on Windows, macOS, and Linux with accessibility-cli, built from a pinned commit. It opens `fixtures/t/reading.md` in the background with the silent `paced` backend. The run summary compares the tree with main's last successful run:

- **Pass:** the same elements as main.
- **Changed:** each added and removed line, in words. Check that every change was meant, such as a new button or a renamed setting.
- **No baseline:** no earlier tree on main to compare with (the first run, or artifacts that expired).
- **Fail:** no tree was dumped. The raw dump and the GUI's log are in the artifact. This one fails the job: the dump is a standing check ([ADR-0039](../adr/0039-automated-screen-reader-checks.md)). "Changed" never does.

The artifacts are `a11y-tree-windows`, `a11y-tree-macos`, and `a11y-tree-linux`. Each holds `tree.txt` (the normalized tree), `raw.json`, `raw-tree.txt` (accessibility-cli's own text view), and `gui.log`. To compare two trees yourself:

```bash
python3 tools/a11y/tree_report.py compare --system Linux --current new/tree.txt --baseline old/tree.txt
```

Running `tree-dump.sh` or `tree-dump.ps1` on your own machine is safe: it opens the GUI in the background, reads nothing aloud, and drives no screen reader. It needs accessibility-cli on the `PATH`, or its path in `A11Y_CLI` (`-Cli` in PowerShell).

### The screen-reader sessions, by hand

Run "Screen-reader checks" (`a11y-tests.yml`) from the Actions tab, and choose a session: `nvda`, `orca`, `voiceover`, or `all`. It also runs when its own files change. Each session opens `fixtures/t/reading.md`, reads about three sentences, pauses, moves to the next heading, opens a dialog, and closes it. What was said is checked against `fixtures/t/expected-phrases.json`.

- **nvda** (Windows): Guidepup starts NVDA and records its spoken phrases. The summary's first line answers "can Guidepup drive NVDA against the textweaver window?" with "Answer: yes" or "Answer: no".
- **orca** (Ubuntu, under Xvfb): the session is checked through AT-SPI events: the caret, the announcements, the focus. Orca runs beside it, and the report lists what it said at each step.
- **voiceover** (macOS 14 and 15): the same with VoiceOver, if Guidepup can start it on the runner. If not, the setup log says why: the system version, System Integrity Protection, and whether AppleScript may drive VoiceOver.

The Orca session is standing: a failed must check fails its job. The NVDA and VoiceOver sessions are report-only: a failure shows in the summary but does not fail the job, whose result comes from building the GUI and installing the tools. ADR-0039 records why each check is standing or not. The phrases NVDA spoke are in the `a11y-nvda` artifact (`phrases.json` and `report.md`), for comparison with what a listener heard in their own session.

Guidepup and its setup tool are locked, with integrity hashes, in `tools/a11y/package-lock.json`. To move to a new version, change `tools/a11y/package.json` and regenerate the lock file with `npm install --package-lock-only --ignore-scripts` in `tools/a11y`, in a container or on a machine with Node. Nothing else in the project needs Node.

## See also

- [Building](building.md): setting up Rust and each system's libraries.
- [CONTRIBUTING.md](../../CONTRIBUTING.md#ci): the CI workflows.
- [Fuzzing](../../fuzz/README.md): the cargo-fuzz targets.
- [Documentation index](../README.md)
