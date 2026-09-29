#!/bin/sh
# Builds liblouis from its release tarball, pinned by version and SHA-256,
# into PREFIX (default /opt/liblouis), for the braille second-tool check.
# Needs a C compiler, make, m4 (for the tables), and curl. Used by .github/workflows/second-tool.yml
# on the runner and, for a local run, inside a Linux container (see
# docs/dev/testing.md); never on a development machine directly.
#
# To move to a new release, change both values below; the digest is the one
# GitHub publishes for the asset, checked here before anything is built.
set -eu

LIBLOUIS_VERSION=3.39.0
LIBLOUIS_SHA256=629fa8cb0dfd9ad457c5bf47a42f0953b673e62c8ad6b1d03ddc4e2bd20008f1
PREFIX="${1:-/opt/liblouis}"
WORK="${TMPDIR:-/tmp}/liblouis-build"

mkdir -p "$WORK"
cd "$WORK"
curl -sSfL -A "textweaver-research (+https://github.com/leavesofgrass/textweaver)" \
  -o liblouis.tar.gz \
  "https://github.com/liblouis/liblouis/releases/download/v$LIBLOUIS_VERSION/liblouis-$LIBLOUIS_VERSION.tar.gz"
echo "$LIBLOUIS_SHA256  liblouis.tar.gz" | sha256sum -c -
tar -xzf liblouis.tar.gz
cd "liblouis-$LIBLOUIS_VERSION"
# The build log stays quiet unless a step fails; then its end is shown.
step() {
  log="$1"
  shift
  if ! "$@" > "$log" 2>&1; then
    echo "Fail: $* (the end of $log follows)"
    tail -n 40 "$log"
    exit 1
  fi
}
step configure.log ./configure --prefix="$PREFIX" --disable-static --without-yaml
step make.log make -j"$(nproc)"
step install.log make install
"$PREFIX/bin/lou_translate" --version | head -n 1
