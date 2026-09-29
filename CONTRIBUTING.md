# Contributing to textweaver

Thank you for helping. textweaver is built first for screen-reader users and students with print disabilities, so accessibility is the product, not a feature. This guide covers the code rules, how the parallel agents work, and how to write commits and docs. [Building](docs/dev/building.md) and [testing](docs/dev/testing.md) have their own guides.

## Building and testing

- [Building](docs/dev/building.md): Rust, Python, what Windows, Linux, and macOS need, the Docker container, the GUI, and the lean reader.
- [Testing](docs/dev/testing.md): the checks every change must pass (one script, `scripts/dev-check`, runs them), the tests, and the benchmarks.

## Code

- Rustdoc on every public item. `missing_docs` is a warning, and CI denies warnings.
- No `unwrap()` or `expect()` on user input or I/O in library code. Libraries use `thiserror`; binaries (`tui`, `cli`, `xtask`) may use `anyhow`.
- No `todo!()`, `unimplemented!()`, or `dbg!()` left behind.
- `unsafe` is denied across the workspace. FFI modules opt out with `#[allow(unsafe_code)]` and a `// SAFETY:` comment on every block.
- Every third-party crate is declared once, in `[workspace.dependencies]` in the root `Cargo.toml`, and used with `name.workspace = true`. Adding one is a decision for the orchestrator; ask for it in your report.
- No async runtime in the speech path. Speech engines have thread affinity ([ADR-0003](docs/adr/0003-speech-threading-and-event-timing.md)).
- Accessibility rules:
  - Every user-visible state change is announced through the app's announcer, filtered by verbosity, and routed by the accessibility mode with `textweaver_a11y::route`, so a screen reader never hears it twice.
  - Every file the app writes while it runs goes through the writer thread (`crates/textweaver-app/src/writer.rs`), never from the input thread.
  - Every string the user hears must read well aloud: no symbols a speech engine skips or spells out, no visual-only formatting.
  - Nothing is shown by colour alone.
  - Keys come from the keymap, never hard-coded; a new action gets a default key, a help string, and a category in `crates/textweaver-keymap/src/action.rs`, and then `cargo xtask keyboard`.
- Fix Star's bugs rather than port them. If you keep a Star quirk on purpose, say so.

[docs/dev/architecture.md](docs/dev/architecture.md) explains how the crates fit together and which way dependencies may point.

## The agent and worktree workflow

textweaver is built by an orchestrator and parallel agents, each in its own git worktree. Each agent gets a brief from the maintainer with the paths it owns, the acceptance criteria, and what to report. The rules:

- **Read first**: your brief, [CLAUDE.md](CLAUDE.md), and the ADRs your brief names.
- **Branch**: work on your own branch, in your own worktree, from `main`. The brief names it: `phase2/<letter>-<topic>` in Phase 2, and `wave3/<letter>-<name>` in Wave 3.
- **Ownership**: edit only the paths your brief lists. Never edit `crates/textweaver-core`, the root `Cargo.toml`, `rust-toolchain.toml`, `.github/`, `docker/`, `compose.yaml`, or another agent's paths unless your brief says so.
- **Contract changes**: if a public type in core or in another agent's crate must change, work around it and write the exact change you need under "Contract change requests" in your report. The orchestrator decides at integration.
- **Separate build directories**: parallel agents must not share a build lock. Give cargo your own target directory, and in Docker use a fixed project name with a private target directory:

  ```bash
  docker compose -p textweaver run --rm -T -e CARGO_TARGET_DIR=/target/agent-y dev cargo test -p textweaver-speech --all-features
  ```

  From Git Bash on Windows, set `MSYS_NO_PATHCONV=1` first, or Git Bash rewrites `/target/...` into a Windows path.
- **Git**: commit in small steps. Do not push, merge, rebase onto `main`, or tag; the orchestrator integrates. Never use a bare `git stash` in a worktree, because the stash is shared with every other worktree.
- **Report**: a summary, the files changed, the test results natively and in the container, contract change requests, open issues, and what the next wave should do first.

## Commits

- One logical change per commit.
- The subject line starts with the area, in lower case, then a colon and a short summary in plain words: `speech: find speech-dispatcher without XDG_RUNTIME_DIR`, `docs: changelog and quick start`, `app: every action is wired`.
- The body says why, and anything a reviewer needs to know: a measurement, a Star bug fixed, a test added.
- Commits made by an AI agent end with the attribution line the session gives, for example `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Never write a date or a weekday from memory. Get today's date from the machine before it goes into a commit, a doc, or a report. On Windows use `py -3` (the Python launcher; `python` may be the Microsoft Store stub):

  ```powershell
  py -3 -c "import datetime as d; t=d.date.today(); print(t, t.strftime('%A'))"
  ```

  On Linux and macOS use `python3`:

  ```bash
  python3 -c "import datetime as d; t=d.date.today(); print(t, t.strftime('%A'))"
  ```

## Documentation

- Every feature a user can reach has a guide, listed in [docs/README.md](docs/README.md).
- Design records and plans are kept by the maintainer outside the repository. The ADRs in [docs/adr/](docs/adr/README.md) explain why the code is built as it is.
- Write for listeners: plain language, short sentences, one idea per sentence, headings and lists. No tables in user guides; reference pages such as `docs/keyboard.md` may have them. Put every command in its own fenced block.
- Every doc ends with a "See also" section linking related docs and the [documentation index](docs/README.md). Guides link to the ADRs that decided them, and ADRs link back to the guides.
- Keep links relative. `tools/check_links.py` must pass.
- ADRs keep their decisions. When later work changes one, add a dated "Status update" line under its date instead of rewriting it.
- Generated files are never edited by hand: `docs/keyboard.md` (`cargo xtask keyboard`) and the data in `docs/site/*.html` (`python3 tools/gen_site_data.py`, or `py -3 tools/gen_site_data.py` on Windows). `cargo xtask parity` writes its Star parity report to `target/parity-report.md`, or to the path given with `--out`; it is build output and is not tracked.
- Add a line to `CHANGELOG.md` under "Unreleased" for anything a user would notice.

## Releases

[docs/dev/releasing.md](docs/dev/releasing.md) describes how a release is made and what the packages hold.

## CI

The workflows in `.github/workflows/`:

- `ci.yml`: formatting; the docs job (links and site data); and clippy, tests, and rustdoc on Ubuntu (all features), macOS, and Windows (Omnivox), with the Apple voice tests on macOS and the 32-bit hosts on Windows.
- `gui-xilem.yml`: the Xilem GUI on Windows, macOS, and Linux: build, clippy, tests, and the accessibility checks (the UI Automation report on Windows, an AT-SPI check on Linux, a silent smoke run on macOS, and on all three the accessibility tree compared with main's; see [ADR-0039](docs/adr/0039-automated-screen-reader-checks.md)).
- `a11y-tests.yml`: screen-reader sessions on CI runners, by hand: NVDA through Guidepup on Windows, an AT-SPI session with Orca under Xvfb, and VoiceOver on macOS (see [Testing](docs/dev/testing.md#automated-screen-reader-checks)).
- `scripts.yml`: lints and dry runs of the scripts in `scripts/`.
- `apple.yml`: extra macOS voice measurements.
- `ci.yml` also has the real-engine jobs, marked "Real engine" in their names: espeak-ng on Linux and Microsoft's SAPI5 voices on Windows, silent (WAV files and a silent output).
- `bench.yml`: the benchmark gate on pull requests and main (see [benchmarks](docs/dev/testing.md#benchmarks)).
- `second-tool.yml`: checks the writers' EPUB and PDF output with epubcheck and veraPDF.
- `pages.yml`: builds the documentation site with Zensical and deploys it to GitHub Pages (see [Documentation site](docs/dev/building.md#documentation-site)).
- `nightly.yml`: every night, the fuzz targets for 10 minutes each (`fuzz/README.md`), Miri on core, text, and the engine-host protocol, AddressSanitizer on the FFI crates, the tests in release mode, an MSRV check with Rust 1.94, the Docker image and its tests, and the soak test; on Mondays, `cargo hack --each-feature` on the speech, formats, and writers crates. Nightly Rust is used only for fuzzing, Miri, and the sanitizer.
- `release.yml`: the release job, started by pushing a tag. It builds the Windows, macOS, and Linux packages (the AppImage in `docker/appimage`), the terminal reader's and the GUI's.

## See also

- [Documentation index](docs/README.md): every guide, grouped by audience.
- [Building](docs/dev/building.md) and [testing](docs/dev/testing.md).
- [Architecture](docs/dev/architecture.md): the crates, the threads, and the path from a file to a spoken word.
- [Docker development container](docs/dev/docker.md): Linux builds and Voxin on any machine.
- [Third-party data](docs/dev/third-party-data.md): the bundled dictionaries, fonts, and word lists, and their licences.
- [scripts/README.md](scripts/README.md): dev-check and the other scripts.
