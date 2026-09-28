# Building textweaver

How to set up a machine to build textweaver: Rust, Python, and what each system needs, the Docker container for the Linux-only features, and the GUI. [Testing](testing.md) covers the checks every change must pass.

## Everyone

1. Install Rust with [rustup](https://rustup.rs). You do not need to pick a version: `rust-toolchain.toml` pins Rust 1.96, and rustup installs it the first time you build. The minimum supported version is 1.92 (`rust-version` in `Cargo.toml`).
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

`--workspace` builds the default members, which leave out the GUI spike. Build the GUI only when you work on it (see below).

## Windows

- Install Visual Studio or the Build Tools with the "Desktop development with C++" workload, for the MSVC linker.
- Add the 32-bit target, which the 32-bit engine hosts need:

  ```powershell
  rustup target add i686-pc-windows-msvc
  ```

- On Windows, CI and `dev-check` use `--features textweaver-speech/omnivox` in place of `--all-features`. Test the Linux engines, espeak-ng and speech-dispatcher, in Docker.

## Linux

The build itself needs only pkg-config and the ALSA development files. espeak-ng is loaded when textweaver starts, with no headers at build time, and speech-dispatcher is spoken to over its socket in pure Rust. The engines themselves are needed to hear them and for their real-engine tests. On Debian and Ubuntu:

```bash
sudo apt install pkg-config libasound2-dev espeak-ng speech-dispatcher
```

`scripts/dev-check.sh` turns on every feature only when `pkg-config` finds espeak-ng, so for its full run also install `libespeak-ng-dev`. `scripts/install-linux.sh --deps-only` installs the build dependencies on Debian, Ubuntu, Fedora, Arch, openSUSE, and Alpine.

## macOS

Install the Xcode command line tools. The Apple speech backends need nothing else.

## Docker

The development container has every library the workspace can link, so Linux-only features build and test on any machine with Docker. [docs/dev/docker.md](docker.md) explains it.

```bash
docker compose build dev
```

## The GUI

The GUI is `textweaver-xilem`, all Rust ([ADR-0027](../adr/0027-xilem-gui.md)); it needs no C or C++ toolkit. The wxDragon spike that came before it ([ADR-0014](../adr/0014-gui-toolkit.md)) was removed in Wave 4.

A lean reader, without in-reader export, preview, and citations (the `publish` feature, on by default), builds with:

```bash
cargo build -p textweaver-tui --no-default-features
```

`cargo xtask deps --check` makes sure that build links none of the conversion and citation stack.

## Scripts

The `scripts/` folder has installers and helpers for every system. Each script has `--help` and `--dry-run`. It says what it will do before it does it, and asks before it uses sudo or changes your PATH. The full list is in [scripts/README.md](../../scripts/README.md).

- `install-linux.sh`: install a release (the AppImage, for x86_64 or aarch64), or build and install from source on Debian, Ubuntu, Fedora, Arch, openSUSE, or Alpine.
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
- `xtask/`: maintenance tasks, run as `cargo xtask bench`, `startup`, `soak`, `dist`, `gui-dist`, `appimage`, `release`, `listen`, `hosts`, `eci-host`, `sapi-host`, `keyboard`, `deps`, `docs`, `settings-doc`, `notices`, `fuzz-seed`, and `parity`.
- `scripts/`: installers, update, speech check, doctor, dev-check, and folder conversion.
- `tools/`: helper programs, among them the link checker (`check_links.py`), the site data generator (`gen_site_data.py`), and the engine spikes.
- `docs/`: user guides, contributor guides, the ADRs, and the interactive pages in `docs/site/`. Start at [the documentation index](../README.md).
- `fixtures/`: sample documents for tests, and Star's reference output for them.
- `third_party/`: pronunciation dictionaries, fonts, and word lists, each with its licence. See [third-party data](third-party-data.md).
- `docker/`, `compose.yaml`, `compose.voxin.yaml`: the Linux development container.

## See also

- [Testing](testing.md): the checks, the tests, and the benchmarks.
- [Docker development container](docker.md): Linux builds and Voxin on any machine.
- [Architecture](architecture.md): the crates and which way dependencies point.
- [Third-party data](third-party-data.md): the bundled dictionaries, fonts, and word lists, and their licences.
- [CONTRIBUTING.md](../../CONTRIBUTING.md): code rules, the agent workflow, commits, and docs.
- [Documentation index](../README.md)
