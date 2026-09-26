# Contributing to textweaver

Thank you for helping. textweaver is built first for screen-reader users and students with print disabilities, so accessibility is the product, not a feature. This guide covers setting up, the checks every change must pass, how the parallel agents work, and how to write commits and docs.

## Set up

### Everyone

1. Install Rust with [rustup](https://rustup.rs). You do not need to pick a version: `rust-toolchain.toml` pins Rust 1.96, and rustup installs it the first time you build. The minimum supported version is 1.92 (`rust-version` in `Cargo.toml`).
2. Install Python 3. The link checker and the site data generator need it; they use only the standard library.
3. Get the code:

   ```bash
   git clone https://github.com/leavesofgrass/textweaver
   ```

4. Build and test:

   ```bash
   cargo build --workspace
   ```

   ```bash
   cargo test --workspace
   ```

`--workspace` builds the default members, which leave out the GUI spike. Build the GUI only when you work on it (see below).

### Windows

- Install Visual Studio or the Build Tools with the "Desktop development with C++" workload, for the MSVC linker.
- Add the 32-bit target, which the 32-bit engine hosts need:

  ```powershell
  rustup target add i686-pc-windows-msvc
  ```

- The `espeak` feature needs libespeak-ng, which Windows does not have. Use `--features textweaver-speech/omnivox` in place of `--all-features`, and test Linux-only features in Docker.

### Linux

Install the development files for espeak-ng, ALSA, and speech-dispatcher, and pkg-config. On Debian and Ubuntu:

```bash
sudo apt install libespeak-ng-dev espeak-ng-data libasound2-dev libspeechd-dev pkg-config
```

`scripts/install-linux.sh --deps-only` installs the build dependencies on Debian, Ubuntu, Fedora, Arch, openSUSE, and Alpine.

### macOS

Install the Xcode command line tools. The Apple speech backends need nothing else.

### Docker

The development container has every library the workspace can link, so Linux-only features build and test on any machine with Docker. [docs/docker.md](docs/docker.md) explains it.

```bash
docker compose build dev
```

### The GUI spike

`textweaver-gui` builds wxWidgets from source through wxDragon. The first build takes several minutes and needs CMake, Ninja, and libclang. On Windows, `crates/textweaver-gui/tools/build-windows.ps1` finds Visual Studio's own CMake and Ninja and sets up the build. [ADR-0014](docs/adr/0014-gui-toolkit.md) has the details.

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
- **keyboard**: `cargo xtask keyboard --check`. It fails when [docs/keyboard.md](docs/keyboard.md) is out of date. Regenerate it with `cargo xtask keyboard`; never edit it by hand.
- **links**: `python3 tools/check_links.py`. Every relative link and anchor in the Markdown docs and in `docs/site` must resolve.
- **site**: `python3 tools/gen_site_data.py --check`. The data embedded in the `docs/site` pages must match `cargo metadata`, the keymap, and the theme files. Regenerate it with `python3 tools/gen_site_data.py`.
- **site-a11y**: `python3 tools/check_site_a11y.py`. Static accessibility checks of the `docs/site` pages: language, title, one level-1 heading and no skipped levels, the skip link, landmarks, a label for every control, text alternatives, and references that resolve.
- **hosts32** (Windows only): the 32-bit engine hosts build.
- **scripts**: shellcheck on the shell scripts, or PSScriptAnalyzer on the PowerShell scripts, when installed.

Useful options: `--only fmt,clippy` runs some steps, `--fail-fast` stops at the first failure, and `--dry-run` prints the commands.

The features: Linux CI uses `--all-features`, which includes `espeak`, `speechd`, and `omnivox`. Windows and macOS use `--features textweaver-speech/omnivox`.

## Tests

- Every crate has unit tests. Segmentation, offset maps, history, marker shifting, and editing also have property tests (`proptest`); loaders and renderers have snapshot tests (`insta`).
- Speech is tested with the `recording` backend and a fake clock, so timing tests never depend on the machine's speed. Engine hosts are tested against fake hosts that speak the real protocol.
- Tests never play audio aloud. Write audio to a temporary file, or use a silent output.
- Tests against real engines are ignored unless you ask for them with an environment variable: `TEXTWEAVER_ECI=1` (Eloquence, with licensed Voxin in the container), `TEXTWEAVER_SAPI=1` (Microsoft voices and eSpeak only), `TEXTWEAVER_APPLE=1` (macOS voices), `TEXTWEAVER_DECTALK=1` (a licensed DECtalk), `TEXTWEAVER_SPEECHD=1` (speech-dispatcher), `TEXTWEAVER_WHISPER_REAL=1` (an installed Whisper), and `TEXTWEAVER_WORD=1` (Microsoft Word opens a DOCX). Run them with `-- --ignored`.
- Never load Code Factory's Eloquence or OpenEVV in tests, and never commit audio made by an engine. Local samples go in the git-ignored `target-local/`.

## Benchmarks

Performance is a requirement ([ADR-0001](docs/adr/0001-workspace-and-dependencies.md)). Measure before and after you optimize a hot path.

```bash
cargo xtask bench
```

It times opening, first speech, navigation while reading, search, and entering edit mode on generated corpora of 1 MB and 10 MB, a 50,000-item list, and a 1 MB single line, and reports peak memory and the number of allocations. It also times the start of `tw --version`, `tw text`, `tw info`, and `tw backends` (`cargo xtask startup` runs only those). `--quick` skips the 10 MB corpus, `--only NAME` runs one measurement, `--file PATH` adds your own document, and `--json PATH` writes the numbers. [The audit](docs/audit-2026-09.md#benchmark-harness) describes the harness.

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

## Code

- Rustdoc on every public item. `missing_docs` is a warning, and CI denies warnings.
- No `unwrap()` or `expect()` on user input or I/O in library code. Libraries use `thiserror`; binaries (`tui`, `cli`, `xtask`) may use `anyhow`.
- No `todo!()`, `unimplemented!()`, or `dbg!()` left behind.
- `unsafe` is denied across the workspace. FFI modules opt out with `#[allow(unsafe_code)]` and a `// SAFETY:` comment on every block.
- Every third-party crate is declared once, in `[workspace.dependencies]` in the root `Cargo.toml`, and used with `name.workspace = true`. Adding one is a decision for the orchestrator; ask for it in your report.
- No async runtime in the speech path. Speech engines have thread affinity ([ADR-0003](docs/adr/0003-speech-threading-and-event-timing.md)).
- Accessibility rules:
  - Every user-visible state change is announced through the app's announcer, filtered by verbosity.
  - Every string the user hears must read well aloud: no symbols a speech engine skips or spells out, no visual-only formatting.
  - Nothing is shown by colour alone.
  - Keys come from the keymap, never hard-coded; a new action gets a default key, a help string, and a category in `crates/textweaver-keymap/src/action.rs`, and then `cargo xtask keyboard`.
- Fix Star's bugs rather than port them. If you keep a Star quirk on purpose, say so.

[docs/architecture.md](docs/architecture.md) explains how the crates fit together and which way dependencies may point.

## The agent and worktree workflow

textweaver is built by an orchestrator and parallel agents, each in its own git worktree. [docs/tasks.md](docs/tasks.md) holds the briefs, the ownership of every path, the acceptance criteria, and each agent's status. The rules:

- **Read first**: the shared preamble in `docs/tasks.md`, your brief, [the plan](docs/plan.md), and the ADRs your brief names.
- **Branch**: work on your own branch, named `wave2/<letter>-<topic>` in Wave 2, in your own worktree, from `main`.
- **Ownership**: edit only the paths your brief lists. Never edit `crates/textweaver-core`, the root `Cargo.toml`, `rust-toolchain.toml`, `.github/`, `docker/`, `compose.yaml`, or another agent's paths unless your brief says so.
- **Contract changes**: if a public type in core or in another agent's crate must change, work around it and write the exact change you need under "Contract change requests" in your report. The orchestrator decides at integration.
- **Separate build directories**: parallel agents must not share a build lock. Give cargo your own target directory, and in Docker use a fixed project name with a private target directory:

  ```bash
  docker compose -p textweaver run --rm -T -e CARGO_TARGET_DIR=/target/agent-y dev cargo test -p textweaver-speech --all-features
  ```

  From Git Bash on Windows, set `MSYS_NO_PATHCONV=1` first, or Git Bash rewrites `/target/...` into a Windows path.
- **Git**: commit in small steps. Do not push, merge, rebase onto `main`, or tag; the orchestrator integrates. Never use a bare `git stash` in a worktree, because the stash is shared with every other worktree.
- **Report**: the format is in `docs/tasks.md`: a summary, the files changed, the test results natively and in the container, contract change requests, open issues, and what the next wave should do first. Add one status line under your own heading in `docs/tasks.md`; leave the rest of its history alone.

## Commits

- One logical change per commit.
- The subject line starts with the area, in lower case, then a colon and a short summary in plain words: `speech: find speech-dispatcher without XDG_RUNTIME_DIR`, `docs: changelog and quick start`, `app: every action is wired`.
- The body says why, and anything a reviewer needs to know: a measurement, a Star bug fixed, a test added.
- Commits made by an AI agent end with the attribution line the session gives, for example `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Never write a date or a weekday from memory. Get today's date from the machine before it goes into a commit, a doc, or a report:

  ```bash
  python -c "import datetime as d; t=d.date.today(); print(t, t.strftime('%A'))"
  ```

## Documentation

- Every feature a user can reach has a guide, listed in [docs/README.md](docs/README.md).
- Write for listeners: plain language, short sentences, one idea per sentence, headings and lists. No tables in user guides; reference pages such as `docs/keyboard.md` may have them. Put every command in its own fenced block.
- Every doc ends with a "See also" section linking related docs and the [documentation index](docs/README.md). Guides link to the ADRs that decided them, and ADRs link back to the guides.
- Keep links relative. `tools/check_links.py` must pass.
- ADRs keep their decisions. When later work changes one, add a dated "Status update" line under its date instead of rewriting it.
- Generated files are never edited by hand: `docs/keyboard.md` (`cargo xtask keyboard`), `docs/parity-report.md` (`cargo xtask parity`), and the data in `docs/site/*.html` (`python3 tools/gen_site_data.py`).
- Add a line to `CHANGELOG.md` under "Unreleased" for anything a user would notice.

## Releases

[docs/releasing.md](docs/releasing.md) describes how a release is made and what the packages hold.

## CI

The workflows in `.github/workflows/`:

- `ci.yml`: formatting; the docs job (links and site data); and clippy, tests, and rustdoc on Ubuntu (all features), macOS, and Windows (Omnivox), with the Apple voice tests on macOS and the 32-bit hosts on Windows.
- `gui.yml`: the GUI spike on Windows and macOS, with a UI Automation report.
- `scripts.yml`: lints and dry runs of the scripts in `scripts/`.
- `apple.yml`: extra macOS voice measurements.
- `ci.yml` also has the real-engine jobs, marked "Real engine" in their names: espeak-ng on Linux and Microsoft's SAPI5 voices on Windows, silent (WAV files and a silent output).
- `bench.yml`: the benchmark gate on pull requests and main (see Benchmarks).
- `nightly.yml`: every night, the fuzz targets for 10 minutes each (`fuzz/README.md`), Miri on core, text, and the engine-host protocol, AddressSanitizer on the FFI crates, the tests in release mode, an MSRV check with Rust 1.92, the Docker image and its tests, and the soak test; on Mondays, `cargo hack --each-feature` on the speech, formats, and writers crates. Nightly Rust is used only for fuzzing, Miri, and the sanitizer.
- `release.yml`: the release job, started by pushing a tag. It builds the Windows, macOS, and Linux packages (the AppImage in `docker/appimage`).

## See also

- [Documentation index](docs/README.md): every guide, grouped by audience.
- [Architecture](docs/architecture.md): the crates, the threads, and the path from a file to a spoken word.
- [Docker development container](docs/docker.md): Linux builds and Voxin on any machine.
- [Tasks and ownership](docs/tasks.md): the agents' briefs and status.
- [scripts/README.md](scripts/README.md): dev-check and the other scripts.
