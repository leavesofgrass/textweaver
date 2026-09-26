# The Linux release build environment: the AppImage and the plain tarball.
#
# Ubuntu 22.04 has glibc 2.35, so what is built here runs on 2022-era and
# newer distributions (Debian 12 and 13, Fedora, Arch, openSUSE, Ubuntu
# 22.04 and later). It has only what the release needs: a C toolchain, the
# ALSA headers (audio output), Rust from rustup (checked against its
# published SHA-256), and the AppImage tools from docker/appimage/fetch-tools.sh.
# espeak-ng is not needed to build: it is loaded at run time.
#
# Build and use it through the xtask, from the repository root, on any OS
# with Docker:
#
#   cargo xtask appimage --docker
#
# or by hand:
#
#   docker build -t textweaver-appimage docker/appimage
#   docker run --rm -v "$PWD:/work" -e CARGO_TARGET_DIR=/target/appimage \
#     -v textweaver_textweaver-target:/target textweaver-appimage cargo xtask appimage
#
# See docs/releasing.md.

FROM ubuntu:22.04

ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      ca-certificates curl git file \
      build-essential pkg-config libasound2-dev \
      desktop-file-utils zsync \
 && rm -rf /var/lib/apt/lists/*

# Rust, as the official rust images install it.
ARG RUSTUP_VERSION=1.29.1
ARG RUST_VERSION=1.96
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:/opt/appimage:$PATH
RUN set -eu; \
    arch="$(uname -m)"; \
    case "$arch" in \
      x86_64) sha=dda7234360b7f578ca8b0ddcb80145646fa61a67c1720a5abc7051b35c9fcb71 ;; \
      aarch64) sha=15f6e4ce9f583b929c996c91562bad6d4454f3281de858b02cdfdef615fac433 ;; \
      *) echo "unsupported architecture $arch"; exit 1 ;; \
    esac; \
    curl --proto '=https' --tlsv1.2 -fsSL -o /tmp/rustup-init \
      "https://static.rust-lang.org/rustup/archive/${RUSTUP_VERSION}/${arch}-unknown-linux-gnu/rustup-init"; \
    echo "$sha  /tmp/rustup-init" | sha256sum -c -; \
    chmod +x /tmp/rustup-init; \
    /tmp/rustup-init -y -q --no-modify-path --profile minimal --default-toolchain "$RUST_VERSION"; \
    rm /tmp/rustup-init; \
    chmod -R a+w "$RUSTUP_HOME" "$CARGO_HOME"; \
    rustc --version

# appimagetool and the type 2 runtime, pinned and checked.
COPY fetch-tools.sh /usr/local/bin/fetch-appimage-tools
RUN sh /usr/local/bin/fetch-appimage-tools /opt/appimage

# appimagetool is itself an AppImage; containers have no FUSE, so it
# extracts itself and runs. The AppImage runtime honours this variable.
ENV APPIMAGE_EXTRACT_AND_RUN=1 \
    APPIMAGETOOL=/opt/appimage/appimagetool \
    CARGO_TERM_COLOR=always

WORKDIR /work
CMD ["bash"]
