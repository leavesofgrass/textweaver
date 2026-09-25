# Docker development container

textweaver is developed on Windows and tested on Linux in a Docker container. The container carries every native library the workspace can link, so Linux-only features (espeak-ng) build and test on a Windows machine.

## What is in the image

`docker/Dockerfile`, built as `textweaver-dev:latest`:

| Component | Version at Phase 0 | Why |
|---|---|---|
| Debian trixie + `rust:1.96` | Rust 1.96.1, rustfmt, clippy | matches `rust-toolchain.toml` |
| espeak-ng, `libespeak-ng-dev` | 1.52.0 | `textweaver-speech/espeak` |
| `libasound2-dev` | | `rodio` (wave 2 export, tones) |
| speech-dispatcher, `libspeechd-dev` | | `textweaver-speech/speechd` (wave 2) |
| clang, `libclang-dev`, pkg-config | | bindgen for `-sys` crates |
| Python 3 | 3.13.5 | parity scripts |
| pandoc | 3.1.11.1 | `textweaver-formats/pandoc` (wave 2) |

The container has no sound device, so audio features are tested for building and for synthesis to files, not for playback. Listen on the host.

## Volumes

`compose.yaml` mounts the working tree at `/work` and keeps three named volumes:

- `textweaver-target` at `/target`: Linux build output (`CARGO_TARGET_DIR=/target`), so it never mixes with the Windows `target/` directory;
- `textweaver-cargo-registry` and `textweaver-cargo-git`: the crates.io cache, so rebuilding the container does not re-download dependencies.

## Everyday commands

Run these from the repository root.

Build (or rebuild) the image:

```bash
docker compose build dev
```

The full check, as CI runs it on Linux:

```bash
docker compose run --rm -T dev bash -c "cargo fmt --all --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace --all-features && cargo doc --workspace --no-deps"
```

An interactive shell in `/work`:

```bash
docker compose run --rm dev
```

Speak to a file with espeak-ng inside the container (once Agent B's backend lands):

```bash
docker compose run --rm -T dev cargo run -p textweaver-cli --features espeak -- speak --backend espeak --out /work/target-audio/test.wav "Hello from textweaver"
```

## Parallel agents

Each agent works in its own git worktree. To share the image and the registry cache but not the build lock, run from the worktree's root with a fixed project name and a private target directory:

```bash
docker compose -p textweaver run --rm -T -e CARGO_TARGET_DIR=/target/agent-b dev cargo test -p textweaver-speech --all-features
```

## Resetting

Remove the build output and caches (the next build downloads and compiles everything again):

```bash
docker volume rm textweaver_textweaver-target textweaver_textweaver-cargo-registry textweaver_textweaver-cargo-git
```
