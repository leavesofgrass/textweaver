# textweaver development and CI image.
#
# Linux build and test environment with every native library the workspace can
# link: espeak-ng (feature `espeak`), ALSA (rodio, wave 2 export/tones),
# speech-dispatcher (wave 2 backend), Python for the Star parity scripts, and
# pandoc for the optional loader. No clang: nothing outside the GUI (which is
# not built here) uses bindgen.
# It is also ready for Voxin (ETI-Eloquence for Linux), mounted from an
# existing licensed installation by compose.voxin.yaml.
#
# Build:   docker compose build dev
# Use:     docker compose run --rm dev cargo test --workspace
# See docs/dev/docker.md.

ARG RUST_VERSION=1.96
FROM rust:${RUST_VERSION}-trixie

ARG DEBIAN_FRONTEND=noninteractive
# i386 libc/libstdc++ run Voxin's 32-bit voxind engine when the Voxin volume
# is mounted. libasound2-plugins routes ALSA to PulseAudio.
RUN dpkg --add-architecture i386 \
 && apt-get update \
 && apt-get install -y --no-install-recommends \
      ca-certificates git pkg-config file \
      espeak-ng espeak-ng-data libespeak-ng-dev \
      libasound2-dev libasound2-plugins alsa-utils pulseaudio-utils \
      libc6:i386 libstdc++6:i386 \
      speech-dispatcher libspeechd-dev \
      python3 python3-venv \
      pandoc \
 && rm -rf /var/lib/apt/lists/*

RUN rustup component add rustfmt clippy

# The repository pins channel "1.96" (rust-toolchain.toml). Install it under
# that name, with its components, so containers don't download it on every
# run.
RUN rustup toolchain install 1.96 --profile minimal --component rustfmt,clippy \
 && rustup default 1.96

# Shared compile cache: every container reuses dependencies compiled by any
# other, through the textweaver-sccache volume (capped in compose.yaml).
RUN cargo install sccache --locked --no-default-features \
 && rm -rf /usr/local/cargo/registry /usr/local/cargo/git

# Audio leaves the container through PulseAudio when PULSE_SERVER points at a
# server (for example tcp:host.docker.internal:4713 with PulseAudio on
# Windows, as emacspeak-docker does). Tests never open an audio device.
RUN printf 'pcm.!default { type pulse }\nctl.!default { type pulse }\n' > /etc/asound.conf

# Voxin is licensed and never baked into the image. compose.voxin.yaml mounts
# an existing installation at /opt/oralux; these links, dangling until then,
# are where libvoxin looks for the engine and its configuration.
RUN ln -s /opt/oralux/voxin/rfs32/opt/IBM /opt/IBM \
 && mkdir -p /var/opt \
 && ln -s /opt/oralux/voxin/rfs32/var/opt/IBM /var/opt/IBM

# The workspace is bind-mounted at /work. Build output goes to a named volume so
# Linux artifacts never mix with the Windows host's target/ directory.
ENV CARGO_TARGET_DIR=/target \
    CARGO_TERM_COLOR=always \
    RUST_BACKTRACE=1 \
    RUSTC_WRAPPER=sccache \
    SCCACHE_DIR=/sccache \
    CARGO_INCREMENTAL=0
WORKDIR /work

CMD ["bash"]
