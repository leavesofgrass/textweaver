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

The GUI moves to Xilem in Wave 3 (Jon's choice, Saturday, September 26, 2026), in a new crate; the wxDragon spike stays as a fallback until then. `textweaver-gui` builds wxWidgets from source through wxDragon. The first build takes several minutes and needs CMake, Ninja, and libclang. On Windows, `crates/textweaver-gui/tools/build-windows.ps1` finds Visual Studio's own CMake and Ninja and sets up the build. [ADR-0014](../adr/0014-gui-toolkit.md) has the details.

A lean reader, without in-reader export, preview, and citations (the `publish` feature, on by default), builds with:

```bash
cargo build -p textweaver-tui --no-default-features
```

`cargo xtask deps --check` makes sure that build links none of the conversion and citation stack.

## See also

- [Testing](testing.md): the checks, the tests, and the benchmarks.
- [Docker development container](docker.md): Linux builds and Voxin on any machine.
- [Architecture](architecture.md): the crates and which way dependencies point.
- [CONTRIBUTING.md](../../CONTRIBUTING.md): code rules, the agent workflow, commits, and docs.
- [Documentation index](../README.md)
