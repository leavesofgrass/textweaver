# textweaver development and CI image.
#
# Linux build and test environment with every native library the workspace can
# link: espeak-ng (feature `espeak`), ALSA (rodio, wave 2 export/tones),
# speech-dispatcher (wave 2 backend), clang for bindgen-based -sys crates,
# Python for the Star parity scripts, and pandoc for the optional loader.
#
# Build:   docker compose build dev
# Use:     docker compose run --rm dev cargo test --workspace
# See docs/docker.md.

ARG RUST_VERSION=1.96
FROM rust:${RUST_VERSION}-trixie

ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      ca-certificates git pkg-config clang libclang-dev \
      espeak-ng espeak-ng-data libespeak-ng-dev \
      libasound2-dev \
      speech-dispatcher libspeechd-dev \
      python3 python3-venv \
      pandoc \
 && rm -rf /var/lib/apt/lists/*

RUN rustup component add rustfmt clippy

# The workspace is bind-mounted at /work. Build output goes to a named volume so
# Linux artifacts never mix with the Windows host's target/ directory.
ENV CARGO_TARGET_DIR=/target \
    CARGO_TERM_COLOR=always \
    RUST_BACKTRACE=1
WORKDIR /work

CMD ["bash"]
