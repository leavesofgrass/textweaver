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
- **clippy**: `cargo clippy --workspace --exclude textweaver-gui --all-targets` with the features for your system, and `-D warnings`. Every warning is an error.
- **test**: `cargo test --workspace --exclude textweaver-gui` with the same features.
- **doc**: `cargo doc --workspace --exclude textweaver-gui --no-deps` with `RUSTDOCFLAGS="-D warnings"`. The usual failures are a redundant link target (write ``[`X`]``, not ``[`X`](crate::X)``), a link to a private item from public docs, and square brackets in prose (put `[mm:ss]` or `[@key]` in backticks).
- **keyboard**: `cargo xtask keyboard --check`. It fails when [docs/keyboard.md](../keyboard.md) is out of date. Regenerate it with `cargo xtask keyboard`; never edit it by hand.
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

Bulk conversion has its own benchmark:

```bash
cargo run --release -p textweaver-convert --example bench_convert
```

## See also

- [Building](building.md): setting up Rust and each system's libraries.
- [CONTRIBUTING.md](../../CONTRIBUTING.md#ci): the CI workflows.
- [Fuzzing](../../fuzz/README.md): the cargo-fuzz targets.
- [Documentation index](../README.md)
