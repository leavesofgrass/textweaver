# Contributing to textweaver

Thank you for thinking about helping. This guide is for anyone who wants to fix a bug, add a feature, improve the docs, or report what does not work for them.

## What textweaver is for

textweaver is an accessible document reader and writer for students with print disabilities. It is built by and for screen reader users. It reads documents aloud with a highlight that follows the spoken word, lets you move through a document by word, sentence, heading, table, and link, and helps you write Markdown while telling you what you type.

Accessibility is the product, not a feature. A change that works with a mouse and a screen, but not with a screen reader, a Braille display, or the keyboard alone, is not finished.

textweaver is a Rust reimplementation of [Star](https://github.com/leavesofgrass/star), an earlier reader written in Python. It is in alpha: it works, and it is changing quickly.

## Ways to help

You do not need to write Rust to help.

- **Try it and report what gets in your way.** Reports from people who use screen readers, Braille displays, magnifiers, or the keyboard alone are the most valuable thing you can give. The [issue forms](https://github.com/leavesofgrass/textweaver/issues/new/choose) ask for what helps most.
- **Improve the docs.** If a guide confused you, it will confuse someone else. The guides are plain Markdown in [docs/](docs/README.md).
- **Pick a good first issue.** [Good first issues](docs/dev/good-first-issues.md) lists small, well-scoped tasks, each with where to look and how to check it.
- **Fix a bug or add a feature.** For anything larger than a small fix, open an issue first, so we can agree on the approach before you spend your time.

## Where to ask

Open an [issue](https://github.com/leavesofgrass/textweaver/issues) on GitHub. Questions are welcome there; you do not need to have found a bug. If you are working on something, say so in its issue, so nobody else starts the same work.

Security problems are different: please report them privately, as [SECURITY.md](SECURITY.md) explains.

Everyone who takes part is asked to follow the [code of conduct](CODE_OF_CONDUCT.md).

## Getting set up

[Building](docs/dev/building.md) has the full setup for Windows, Linux, and macOS. The short version:

1. Install Rust with [rustup](https://rustup.rs). You do not need to choose a version: `rust-toolchain.toml` pins it, and rustup installs it the first time you build.
2. Install Python 3. A few checks use it, with the standard library only. On Windows, install it from python.org and run the tools with `py -3`, because `python` there may be the Microsoft Store stub.
3. On Windows, install Visual Studio or the Build Tools with the "Desktop development with C++" workload. On Linux, install pkg-config and the ALSA development files (`libasound2-dev` on Debian and Ubuntu). On macOS, install the Xcode command line tools. Install cmake 3.16 or later on every system; the Opus encoder builds from source ([Building](docs/dev/building.md#cmake-for-opus-audio-export)).
4. Get the code, build it, and run the tests:

   ```bash
   git clone https://github.com/leavesofgrass/textweaver
   ```

   ```bash
   cargo build --workspace
   ```

   ```bash
   cargo test --workspace
   ```

5. Try the terminal reader and the window on a document. Space starts and pauses reading, and `?` lists every key:

   ```bash
   cargo run -p textweaver-tui -- fixtures/sample.md
   ```

   and the window:

   ```bash
   cargo run -p textweaver-xilem -- fixtures/sample.md
   ```

The Docker development container has every library the workspace can use, including the Linux speech engines, so you can test Linux features from any system. [Docker development container](docs/dev/docker.md) explains it.

[Architecture](docs/dev/architecture.md) explains how the crates fit together, and is the best place to start reading the code.

## The checks

CI runs a set of checks on every change. Run them yourself before you open a pull request; one script runs them all:

```bash
scripts/dev-check.sh
```

On Windows:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\dev-check.ps1
```

It runs formatting, clippy with warnings as errors, the tests, rustdoc, the check that every generated file is current (`cargo xtask regen --check`), the link checker, and the site accessibility checks, and ends with a summary that says, in words, which steps passed and which failed. `--only fmt,clippy` runs some of the steps, and `--docker` runs everything in the Linux container. [Testing](docs/dev/testing.md) describes each check and the few that CI runs but the script does not.

If a check fails and you cannot see why, open the pull request anyway and say so. We would rather help than have you stuck.

## Accessibility expectations

Every change must work for people who do not look at the screen.

- **Screen readers.** Every state change a user should know about is announced through the app's announcer, not only shown. Every string the user hears must read well aloud: no symbols a speech engine skips or spells out, and no meaning carried by layout alone.
- **Braille displays.** Many users read one line of about 40 cells at a time. Put the meaning first on each line and message, prefer words to symbols (emoji, arrows, check marks, and box drawing often come through as noise), and keep messages short.
- **Color never carries meaning alone.** Pair every color with text first, such as "Pass" and "Fail" in words, then symbols or patterns if they help. This covers themes, status lines, diffs, charts, and screenshots.
- **The keyboard.** Everything works from the keyboard. Keys come from the keymap, never hard-coded: a new action gets a default key, a help string, and a category in `crates/textweaver-keymap/src/action.rs`, and then `cargo xtask keyboard` regenerates [docs/keyboard.md](docs/keyboard.md).
- **Say how you checked.** In your pull request, say what you checked with a screen reader, which one, and which speech engine. If you could not check something by ear, say that too; a maintainer will.

## Code

- Rustdoc on every public item. `missing_docs` is a warning, and CI denies warnings.
- No `unwrap()` or `expect()` on user input or I/O in library code. Libraries use `thiserror`; binaries (`tui`, `cli`, `xtask`) may use `anyhow`.
- No `todo!()`, `unimplemented!()`, or `dbg!()` left behind.
- `unsafe` is denied across the workspace. FFI modules opt out with `#[allow(unsafe_code)]` and a `// SAFETY:` comment on every block.
- Every third-party crate is declared once, in `[workspace.dependencies]` in the root `Cargo.toml`, and used with `name.workspace = true`. A new dependency is a decision for the maintainers; propose it in the issue or pull request, with its license.
- No async runtime in the speech path. Speech engines have thread affinity ([ADR-0003](docs/adr/0003-speech-threading-and-event-timing.md)).
- Every state change is announced through the app's announcer, filtered by verbosity, and routed by the accessibility mode with `textweaver_a11y::route`, so a screen reader never hears it twice.
- Every file the app writes while it runs goes through the writer thread (`crates/textweaver-app/src/writer.rs`), never from the input thread.
- Tests never play audio aloud. Write audio to a temporary file, or use a silent output.
- Integration tests go in one test program per crate, `tests/it/main.rs`. A new integration test is a module there (`tests/it/<name>.rs`, with `mod <name>;` in `main.rs`), never a new file directly in `tests/`: every file there is a program of its own, which links the crate again and slows every test build. [Testing](docs/dev/testing.md#where-tests-go) has the details.
- Fix Star's bugs rather than port them. If you keep a Star quirk on purpose, say so.
- US English in code comments, docs, and messages ("color", "behavior").

The [architecture decision records](docs/adr/README.md) (ADRs) explain why the code is built as it is. If a change goes against one, say so in the pull request.

## Proposing a change

1. For anything beyond a small fix, open an issue first and describe what you want to change.
2. Fork the repository and make a branch for your change.
3. Make the change, with a test that fails without it.
4. If you changed a key, a setting, a dependency, or a crate, run `cargo xtask regen` to rebuild the generated files, and commit what changed. Then run the checks.
5. Add a line to `CHANGELOG.md` under "Unreleased" for anything a user would notice.
6. Open a pull request. The template asks what changed, why, and how you checked it.

A maintainer will review it. Reviews may ask for changes; that is normal, and not a judgment of you.

## Commits

- One logical change per commit.
- The subject line starts with the area, in lower case, then a colon and a short summary in plain words: `speech: find speech-dispatcher without XDG_RUNTIME_DIR`, `docs: changelog and quick start`, `app: every action is wired`.
- The body says why, and anything a reviewer needs to know: a measurement, a Star bug fixed, a test added.
- Check any date you write against your computer's clock, and compute the weekday rather than recalling it. On Windows:

  ```powershell
  py -3 -c "import datetime as d; t=d.date.today(); print(t, t.strftime('%A'))"
  ```

  On Linux and macOS:

  ```bash
  python3 -c "import datetime as d; t=d.date.today(); print(t, t.strftime('%A'))"
  ```

## Documentation

- Every feature a user can reach has a guide, listed in [docs/README.md](docs/README.md).
- Write for listeners: plain language, short sentences, one idea per sentence, headings and lists. No tables in user guides; reference pages such as `docs/keyboard.md` may have them. Put every command in its own fenced block.
- Every doc ends with a "See also" section linking related docs and the [documentation index](docs/README.md). Guides link to the ADRs that decided them, and ADRs link back to the guides.
- Keep links relative. `tools/check_links.py` must pass.
- ADRs keep their decisions. When later work changes one, add a dated "Status update" line under its date instead of rewriting it.
- Generated files are never edited by hand: `THIRD-PARTY-NOTICES.md`, `docs/keyboard.md`, `docs/settings-reference.md`, and the data in `docs/site/*.html`. `cargo xtask regen` rebuilds them all, in the right order; `cargo xtask regen --check` says which are out of date.

## Contributing with AI assistance

Some of textweaver is written with AI coding assistants, and you may use one too. The same rules apply to that work as to any other, and you are responsible for what you submit.

- Read and understand every change before you send it. Run the checks yourself.
- Check accessibility claims by ear where you can; an assistant cannot hear the result.
- Keep personal information out of prompts that leave your machine, and out of commits: no email addresses, user names, or machine names.
- End each commit made with an assistant with the attribution line your tool gives, for example `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- If you run several assistants at once, give each its own git worktree and its own build directory, so they do not share a build lock:

  ```bash
  CARGO_TARGET_DIR=target/agent-a cargo test -p textweaver-speech
  ```

  In Docker, use a fixed project name with a private target directory. From Git Bash on Windows, set `MSYS_NO_PATHCONV=1` first, or Git Bash rewrites `/target/...` into a Windows path:

  ```bash
  docker compose -p textweaver run --rm -T -e CARGO_TARGET_DIR=/target/agent-a dev cargo test -p textweaver-speech --all-features
  ```

- Every worktree shares one git stash. Use a temporary commit to set work aside instead of a bare `git stash`.

If you use an AI coding assistant, keep its settings and rules files out of your commits; this repository doesn't track them.

## Releases

[Releasing](docs/dev/releasing.md) describes how a release is made and what the packages hold. Releases are made by the maintainers.

## CI

The workflows in `.github/workflows/`:

- `ci.yml`: formatting; the docs job (links and site data); and clippy, tests, and rustdoc on Ubuntu (all features), macOS, and Windows (Omnivox), with the Apple voice tests on macOS and the 32-bit hosts on Windows. It also has the real-engine jobs, marked "Real engine" in their names: espeak-ng on Linux and Microsoft's SAPI5 voices on Windows, silent (WAV files and a silent output).
- `gui-xilem.yml`: the Xilem GUI on Windows, macOS, and Linux: build, clippy, tests, and the accessibility checks (the UI Automation report on Windows, an AT-SPI check on Linux, a silent smoke run on macOS, and on all three the accessibility tree compared with main's; see [ADR-0039](docs/adr/0039-automated-screen-reader-checks.md)).
- `a11y-tests.yml`: screen-reader sessions on CI runners, started by hand: NVDA through Guidepup on Windows, an AT-SPI session with Orca under Xvfb, and VoiceOver on macOS (see [Testing](docs/dev/testing.md#automated-screen-reader-checks)).
- `scripts.yml`: lints and dry runs of the scripts in `scripts/`.
- `apple.yml`: extra macOS voice measurements.
- `bench.yml`: the benchmark gate on pull requests and main (see [benchmarks](docs/dev/testing.md#benchmarks)).
- `second-tool.yml`: checks the writers' EPUB and PDF output with epubcheck and veraPDF.
- `pages.yml`: builds the documentation site with Zensical and deploys it to GitHub Pages (see [Documentation site](docs/dev/building.md#documentation-site)).
- `nightly.yml`: every night, the fuzz targets for 10 minutes each (`fuzz/README.md`), Miri on core, text, and the engine-host protocol, AddressSanitizer on the FFI crates, the tests in release mode, an MSRV check, the Docker image and its tests, and the soak test; on Mondays, `cargo hack --each-feature` on the speech, formats, and writers crates. Nightly Rust is used only for fuzzing, Miri, and the sanitizer.
- `release.yml`: the release job, started by pushing a tag. It builds the Windows, macOS, and Linux packages (the AppImage in `docker/appimage`), the terminal reader's and the GUI's.

## License

textweaver is free software under the GNU General Public License, version 3 or later ([LICENSE](LICENSE)). By contributing, you agree that your contribution is licensed the same way.

## See also

- [Documentation index](docs/README.md): every guide, grouped by audience.
- [Good first issues](docs/dev/good-first-issues.md): small tasks to start with.
- [Building](docs/dev/building.md) and [testing](docs/dev/testing.md).
- [Architecture](docs/dev/architecture.md): the crates, the threads, and the path from a file to a spoken word.
- [Docker development container](docs/dev/docker.md): Linux builds and Voxin on any machine.
- [Third-party data](docs/dev/third-party-data.md): the bundled dictionaries, fonts, and word lists, and their licenses.
- [scripts/README.md](scripts/README.md): dev-check and the other scripts.
- [Code of conduct](CODE_OF_CONDUCT.md) and [security policy](SECURITY.md).
