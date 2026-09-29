# Docker development container

textweaver is developed on Windows and tested on Linux in a Docker container. The container carries every native library the workspace can link, so Linux-only features (espeak-ng) build and test on a Windows machine.

## What is in the image

`docker/Dockerfile`, built as `textweaver-dev:latest`:

| Component | Version at Phase 0 | Why |
|---|---|---|
| Debian trixie + `rust:1.96` | Rust 1.96.1, rustfmt, clippy | matches `rust-toolchain.toml` |
| espeak-ng, `libespeak-ng-dev` | 1.52.0 | `textweaver-speech/espeak` |
| `libasound2-dev` | | `rodio` (audio playback for the engine hosts and tones) |
| speech-dispatcher, `libspeechd-dev` | | `textweaver-speech/speechd` |
| pkg-config | | finds espeak-ng. There is no clang: the espeak feature's declarations are hand-written, and only the GUI (not built in the container) uses bindgen |
| Python 3 | 3.13.5 | parity scripts, `tools/check_links.py`, `tools/gen_site_data.py` |
| pandoc | 3.1.11.1 | the Pandoc fallback in `tw convert` and `textweaver-formats/pandoc` |

The container has no sound device, so audio features are tested for building and for synthesis to files, not for playback. Listen on the host.

## Volumes, limits, and disk

`compose.yaml` mounts the working tree at `/work` and keeps these named volumes:

- **`tw-target-<agent>` at `/target`:** Linux build output (`CARGO_TARGET_DIR=/target`). Each agent gets its own, chosen with `TW_AGENT`; the default is `tw-target-orch`. It never mixes with the Windows `target/` directory.
- **`textweaver-sccache` at `/sccache`:** the shared compile cache, capped at 30 GB (`SCCACHE_CACHE_SIZE`). Dependencies compile once for every container. The image sets `RUSTC_WRAPPER=sccache`.
- **`textweaver-cargo-registry` and `textweaver-cargo-git`:** the crates.io cache, so rebuilding the container does not re-download dependencies.

Limits, set in September 2026 after one shared build volume grew to 536 GB and Docker's disk on D: to 626 GB:

- **Per container:** 6 GB of RAM, 4 CPUs and 4 cargo jobs by default. Change them with `TW_MEM`, `TW_CPUS` and `TW_JOBS`.
- **At most three containers building at once.** Docker's machine has about 31 GB in all. Before a build, at least 26 GB of host RAM must be free, so 20 GB always stays free on a 64 GB host.
- **Smaller builds:** `Cargo.toml`'s dev and test profiles keep line tables only for our crates, no debug info for dependencies, and incremental builds off.
- **Before heavy runs,** check the disk with `powershell -File tools/build-hygiene.ps1`. It reports D:'s free space, Docker's images, volumes, build cache and virtual disk, and each agent volume. Stop if D: is near its 200 GB floor.
- **After an agent's work is merged,** remove its build volume with `tools/build-hygiene.ps1 -RemoveVolume tw-target-<agent>`. That lists the volume and removes nothing; run it again with `-Apply` to remove exactly that volume. The script refuses any volume not named `tw-target-*`.
- **Docker's virtual disk** (`D:\DockerData\...\docker_data.vhdx`) never shrinks on its own. After a wave, compact it with Docker Desktop stopped. That needs one administrator prompt; see `D:\recovery\work\compact-docker-vhdx.ps1`.

## Everyday commands

Run these from the repository root.

Build (or rebuild) the image:

```bash
docker compose build dev
```

The full check, as CI runs it on Linux. `scripts/dev-check.sh` runs formatting, clippy, the tests, rustdoc, the keyboard reference check, the link check, and the site data check, with every feature:

```bash
docker compose run --rm -T dev bash scripts/dev-check.sh
```


An interactive shell in `/work`:

```bash
docker compose run --rm dev
```

Speak to a file with espeak-ng inside the container:

```bash
docker compose run --rm -T dev cargo run -p textweaver-cli --features espeak -- speak --backend espeak --out /work/target-audio/test.wav "Hello from textweaver"
```

## Parallel agents

Each agent works in its own git worktree, and runs its builds, tests and file operations **inside the container**, so shell commands never touch the host or files outside its worktree. From the worktree's root, use a fixed project name and the agent's own build volume:

```bash
TW_AGENT=w4b MSYS_NO_PATHCONV=1 docker compose -p textweaver run --rm -T dev cargo test -p textweaver-speech --all-features
```

In Git Bash, `MSYS_NO_PATHCONV=1` stops Git Bash rewriting container paths into Windows paths.

## Resetting

Removing volumes is a delete: check with the project maintainer first, and list what goes. Remove one worktree's build output with `tools/build-hygiene.ps1 -RemoveVolume tw-target-<agent>`, then `-Apply`. Never remove volumes you did not create, such as `emacspeak-docker_voxin`.

## Voxin (ETI-Eloquence for Linux)

Voxin is licensed per user, so it is never copied into this repository or the `textweaver-dev` image. `compose.voxin.yaml` mounts an existing installation read-only: by default the `emacspeak-docker_voxin` volume created by the emacspeak-docker project's `install-outloud` (Voxin 3.3, US English), or another volume named in `VOXIN_VOLUME`. The image already carries what Voxin needs: 32-bit libc for its `voxind` engine and the `/opt/IBM` and `/var/opt/IBM` links `libvoxin` looks for.

Check that the engine runs:

```bash
docker compose -f compose.yaml -f compose.voxin.yaml run --rm -T dev voxin-say -w /tmp/t.wav "hello"
```

Check ECI index marks (the mechanism behind word highlighting; ADR-0007):

```bash
docker compose -f compose.yaml -f compose.voxin.yaml run --rm -T dev python3 tools/eci-spike/voxin_spike.py
```

Run the Eloquence backend's real-engine tests on Linux:

```bash
docker compose -f compose.yaml -f compose.voxin.yaml run --rm -T -e TEXTWEAVER_ECI=1 dev cargo test -p textweaver-eci -- --ignored
```

The overlay sets `ECIINI`, `LD_LIBRARY_PATH`, and `TEXTWEAVER_ECI_LIBRARY` (`/opt/oralux/voxin/lib/libibmeci.so`, the 64-bit ECI library).

## Hearing the container

ALSA inside the image is routed to PulseAudio. Point `PULSE_SERVER` at a PulseAudio server on the host (the Voxin overlay defaults to `tcp:host.docker.internal:4713`, the address emacspeak-docker's `scripts/setup-audio.ps1` sets up on Windows) and anything textweaver plays in the container is heard on the host. Tests never open an audio device.

`scripts/voxin-docker.sh` wraps the Voxin commands above; see [scripts/README.md](../../scripts/README.md#voxin-dockersh).

## The Linux release image

`docker/appimage/` holds a second image, only for building the Linux release packages: Ubuntu 22.04 (an older glibc, so the packages run on older distributions), Rust, and the AppImage tools, checked against their published checksums. `cargo xtask appimage --docker` builds the image and the packages from any system with Docker, into `target/dist/`, and `bash docker/appimage/test-distros.sh target/dist` runs them on Debian, Fedora, and Arch. [Releasing](releasing.md#the-linux-packages) has the details.

## See also

- [CONTRIBUTING.md](../../CONTRIBUTING.md): setting up, the checks, and the agent workflow.
- [The Eloquence guide](../eloquence.md): Voxin for users.
- [ADR-0001: Workspace and dependencies](../adr/0001-workspace-and-dependencies.md): why Linux-only features are tested in the container.
- [ADR-0007: Eloquence through an ECI host](../adr/0007-eloquence-via-eci-host.md): the Voxin measurements.
- [Documentation index](../README.md)
