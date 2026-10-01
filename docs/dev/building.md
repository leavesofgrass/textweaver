# Building textweaver

How to set up a machine to build textweaver: Rust, Python, and what each system needs, the Docker container for the Linux-only features, and the GUI. [Testing](testing.md) covers the checks every change must pass.

## Everyone

1. Install Rust with [rustup](https://rustup.rs). You do not need to pick a version: `rust-toolchain.toml` pins Rust 1.96, and rustup installs it the first time you build. The minimum supported version is 1.94 (`rust-version` in `Cargo.toml`); the GUI needs 1.96.
2. Install Python 3. The link checker and the site data generator need it; they use only the standard library. On Windows, install it from python.org with the `py` launcher, and run the tools with `py -3`, for example `py -3 tools/check_links.py`: `python` there may be the Microsoft Store stub, which opens the Store instead of running Python.
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

`--workspace` builds the default members, which leave out the GUI. Build the GUI only when you work on it (see below).

### cmake, for Opus audio export

Opus export uses libopus, which is built from source with [cmake](https://cmake.org) during the build. Install cmake 3.16 or later and make sure `cmake` is on `PATH`, or set the `CMAKE` environment variable to the full path of `cmake.exe` (on Windows, a portable cmake works). The CI runners and the development container already have it.

To build without cmake, turn off the `opus` feature. Opus export then goes through ffmpeg, as M4B does:

```bash
cargo build -p textweaver-cli --no-default-features
```

For the GUI, list its other default features: `cargo build -p textweaver-xilem --no-default-features --features screenshot,renderer-vello,publish,lint,dictation,audio-export`. A build of the whole workspace, such as `cargo build --workspace`, always includes Opus, because `tw` turns it on.

## Your first contribution

1. **Clone and build**, as above.
2. **Run it:**

   ```bash
   cargo run -p textweaver-tui --bin textweaver -- fixtures/t/reading.md
   ```

   Or use the command-line tool, `tw`, to speak a file to a WAV without opening the reader:

   ```bash
   cargo run -p textweaver-cli --bin tw -- speak --backend null --file fixtures/t/reading.md --out /tmp/out.wav
   ```

   The `null` backend needs no speech engine installed; it is how CI, and this quick check, hear nothing but still exercise the whole pipeline.
3. **Test the crate you are about to change,** before changing anything, so you know the tests pass on a clean tree:

   ```bash
   cargo test -p textweaver-text
   ```

4. **Read [Architecture](architecture.md)** for the crate map and the path a document takes from disk to a spoken, highlighted word, then read the ADR it links for the crate you are working in. The [interactive architecture page](../site/architecture.html) shows the same map, browsable by keyboard.
5. **Make a change and run the checks** in [Testing](testing.md) before you send it: `scripts/dev-check.sh` (or `scripts\dev-check.ps1` on Windows) runs formatting, clippy, the tests, and the doc and link checks in one command.

**Where to start.** A few crates are good places to get oriented, because they are small, have few dependencies, and are exercised directly by their own tests:

- `textweaver-core` (`crates/textweaver-core/`): the shared position and unit types every other crate builds on. No workspace dependencies, so it is a self-contained read.
- `textweaver-theme` (`crates/textweaver-theme/`): the built-in color themes and contrast checks. A good first pull request is a new theme or a contrast fix.
- `textweaver-summary` (`crates/textweaver-summary/`): a small, self-contained algorithm (LexRank extractive summaries) with no model to download, a clear ADR ([ADR-0037](../adr/0037-extractive-summaries.md)), and a short test suite to learn the crate's shape from.

From there, [Architecture](architecture.md#the-crates) groups every crate by the part of the system it serves, so you can find the one closest to what you want to change. [CONTRIBUTING.md](../../CONTRIBUTING.md) has the code rules, commit style, and how to send a change.

## Windows

- Install Visual Studio or the Build Tools with the "Desktop development with C++" workload, for the MSVC linker, and cmake for the Opus encoder (`winget install Kitware.CMake`, or set `CMAKE` to a portable cmake).
- Add the 32-bit target, which the 32-bit engine hosts need:

  ```powershell
  rustup target add i686-pc-windows-msvc
  ```

- On Windows, CI and `dev-check` use `--features textweaver-speech/omnivox` in place of `--all-features`. Test the Linux engines, espeak-ng and speech-dispatcher, in Docker.

## Linux

The build itself needs only pkg-config, cmake, and the ALSA development files. espeak-ng is loaded when textweaver starts, with no headers at build time, and speech-dispatcher is spoken to over its socket in pure Rust. The engines themselves are needed to hear them and for their real-engine tests. On Debian and Ubuntu:

```bash
sudo apt install pkg-config cmake libasound2-dev espeak-ng speech-dispatcher
```

`scripts/dev-check.sh` turns on every feature only when `pkg-config` finds espeak-ng, so for its full run also install `libespeak-ng-dev`. `scripts/install-linux.sh --deps-only` installs the build dependencies on Debian, Ubuntu, Fedora, Arch, openSUSE, and Alpine.

## macOS

Install the Xcode command line tools, and cmake for the Opus encoder (`brew install cmake`). The Apple speech backends need nothing else.

## Docker

The development container has every library the workspace can link, so Linux-only features build and test on any machine with Docker. [docs/dev/docker.md](docker.md) explains it.

```bash
docker compose build dev
```

## The GUI

The GUI is `textweaver-xilem`, all Rust ([ADR-0027](../adr/0027-xilem-gui.md)); it needs no C or C++ toolkit. The wxDragon spike that came before it ([ADR-0014](../adr/0014-gui-toolkit.md)) has been removed.

A lean reader, without in-reader export, preview, and citations (the `publish` feature, on by default), builds with:

```bash
cargo build -p textweaver-tui --no-default-features
```

`cargo xtask deps --check` makes sure that build links none of the conversion and citation stack.

## Scripts

The `scripts/` folder has installers and helpers for every system. Each script has `--help` and `--dry-run`. It says what it will do before it does it, and asks before it uses sudo or changes your PATH. The full list is in [scripts/README.md](../../scripts/README.md).

- `install-linux.sh`: install a release (the AppImage or tarball, for x86_64 or aarch64), or build and install from source on Debian, Ubuntu, Fedora, Arch, openSUSE, or Alpine.
- `install-macos.sh`: install the newest macOS release, or build from source.
- `install-windows.ps1`: install the newest Windows release, or build from source.
- `update.sh` and `update.ps1`: update an installed textweaver.
- `speech-check.sh` and `speech-check.ps1`: report the speech engines, voices, and audio.
- `doctor.sh` and `doctor.ps1`: a system report to paste into a bug report.
- `dev-check.sh` and `dev-check.ps1`: run CI's checks locally.
- `convert-folder.sh` and `convert-folder.ps1`: convert a folder of Markdown to HTML, EPUB, PDF, or another format.
- `voxin-docker.sh`: run the Eloquence tests or `tw speak` with Voxin in the Docker container.

## Repository layout

- `crates/`: the Rust crates, one per job. [Architecture](architecture.md) describes each one, how they depend on each other, and how a document becomes speech.
- `xtask/`: maintenance tasks, run as `cargo xtask bench`, `startup`, `soak`, `dist`, `gui-dist`, `appimage`, `release`, `listen`, `hosts`, `eci-host`, `sapi-host`, `regen`, `keyboard`, `deps`, `docs`, `settings-doc`, `notices`, `fuzz-seed`, and `parity`.
- `scripts/`: installers, update, speech check, doctor, dev-check, and folder conversion.
- `tools/`: helper programs, among them the link checker (`check_links.py`), the site data generator (`gen_site_data.py`), and the engine spikes.
- `docs/`: user guides, contributor guides, the ADRs, and the interactive pages in `docs/site/`. Start at [the documentation index](../README.md).
- `fixtures/`: sample documents for tests, and Star's reference output for them.
- `third_party/`: pronunciation dictionaries, fonts, and word lists, each with its licence. See [third-party data](third-party-data.md).
- `docker/`, `compose.yaml`, `compose.voxin.yaml`: the Linux development container.

## Documentation site

`docs/` builds into the site at <https://leavesofgrass.github.io/textweaver/>, with [Zensical](https://zensical.dev), pinned at 0.0.66. Install it into a virtual environment, never into the system Python, and keep the environment and pip's cache on `D:`:

```powershell
py -3 -m venv D:\textweaver\.claude\tmp\zensical-venv
$env:PIP_CACHE_DIR = "D:\textweaver\.claude\tmp\pip-cache"
D:\textweaver\.claude\tmp\zensical-venv\Scripts\pip install --require-hashes -r tools\site-requirements.txt
```

On Linux or macOS:

```bash
python3 -m venv .venv-site
.venv-site/bin/pip install --require-hashes -r tools/site-requirements.txt
```

Build with `tools/build_site.py`, which finds Zensical through the `ZENSICAL` environment variable when it is not on `PATH`:

```powershell
$env:ZENSICAL = "D:\textweaver\.claude\tmp\zensical-venv\Scripts\zensical.exe"
py -3 tools/build_site.py
```

```bash
python3 tools/build_site.py
```

The script runs Zensical against `zensical.toml`, places the interactive pages from `docs/site/` beside the built guides, and checks every built page with `tools/check_site_a11y.py`. It must report 0 pages with problems.

**Every new doc must be added to `zensical.toml`'s navigation**, which mirrors `docs/README.md`'s groupings; the build stops if a Markdown file under `docs/` is missing from it. `.github/workflows/pages.yml` runs the same build on push to `main` when `docs/` or `zensical.toml` changes, and deploys it with `actions/deploy-pages`.

## See also

- [Testing](testing.md): the checks, the tests, and the benchmarks.
- [Docker development container](docker.md): Linux builds and Voxin on any machine.
- [Architecture](architecture.md): the crates and which way dependencies point.
- [Third-party data](third-party-data.md): the bundled dictionaries, fonts, and word lists, and their licences.
- [CONTRIBUTING.md](../../CONTRIBUTING.md): how to contribute: setup, the checks, accessibility, commits, and docs.
- [Documentation index](../README.md)
