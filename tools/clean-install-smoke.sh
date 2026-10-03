#!/usr/bin/env bash
# The Linux packages, installed in a clean container with no network:
#
#   bash tools/clean-install-smoke.sh DIST_DIR [IMAGE]
#
# DIST_DIR holds textweaver-*-linux-ARCH.tar.gz and .AppImage (target/dist
# after `cargo xtask appimage`); ARCH is this machine's unless the ARCH
# variable names another. IMAGE defaults to debian:stable-slim.
#
# A first run must work with nothing installed and nothing fetched: no
# components, no voices, no models, no network. The image gets only the
# ALSA library a desktop always has (that is the one step with a network,
# when the image is built). Then, in a container started with
# --network none, as an unprivileged user with an empty home, the tarball
# is unpacked into /opt/textweaver and tools/package-smoke.sh runs its tw
# (--version, text, info, components list, backends, convert to EPUB),
# and the AppImage (extracted and run, since a container has no FUSE)
# runs tw components list and tw info. Each line starts with Pass or Fail.
set -euo pipefail

dist="${1:?usage: tools/clean-install-smoke.sh DIST_DIR [IMAGE]}"
image="${2:-debian:stable-slim}"
arch="${ARCH:-$(uname -m)}"
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/.." && pwd)"
dist="$(cd "$dist" && pwd)"

tarball=""
appimage=""
for f in "$dist"/textweaver-*-linux-"$arch".tar.gz; do
  [ -e "$f" ] && tarball="$(basename "$f")"
done
for f in "$dist"/textweaver-*-linux-"$arch".AppImage; do
  [ -e "$f" ] && appimage="$(basename "$f")"
done
[ -n "$tarball" ] || {
  echo "Fail: no textweaver-*-linux-$arch.tar.gz in $dist"
  exit 1
}

# The clean image: the base image plus the ALSA library, nothing else.
tag="textweaver-clean-smoke:local"
docker build -q -t "$tag" - > /dev/null << EOF
FROM $image
RUN apt-get update -qq \\
 && (apt-get install -y -qq --no-install-recommends libasound2t64 \\
     || apt-get install -y -qq --no-install-recommends libasound2) \\
 && rm -rf /var/lib/apt/lists/*
EOF

echo "Clean container: $image, $arch, no network, as user 65534"
status=0
docker run --rm --network none --user 65534:65534 -e HOME=/tmp/home \
  -e TARBALL="$tarball" -e APPIMAGE="$appimage" \
  -v "$dist:/pkg:ro" -v "$root/fixtures:/fixtures:ro" -v "$root/tools:/tools:ro" \
  "$tag" bash -euc '
    mkdir -p "$HOME" /tmp/opt/textweaver
    # No route out: a network request would fail rather than reach anywhere.
    if getent hosts github.com > /dev/null 2>&1; then
      echo "Fail: the container can resolve names; it should have no network"
      exit 1
    fi
    tar -xzf "/pkg/$TARBALL" -C /tmp/opt/textweaver --strip-components=1
    echo "Pass: the tarball unpacked into /tmp/opt/textweaver"
    rc=0
    bash /tools/package-smoke.sh /tmp/opt/textweaver/tw /fixtures/sample.md || rc=1
    if [ -n "$APPIMAGE" ]; then
      cp "/pkg/$APPIMAGE" /tmp/textweaver.AppImage
      chmod +x /tmp/textweaver.AppImage
      cd /tmp
      export TEXTWEAVER_HOME=/tmp/appimage-home
      for args in "components list" "info /fixtures/sample.md"; do
        # One word per argument: split on purpose.
        # shellcheck disable=SC2086
        if out="$(./textweaver.AppImage --appimage-extract-and-run --tw $args 2>&1)" && [ -n "$out" ]; then
          echo "Pass: the AppImage runs tw $args"
        else
          echo "Fail: the AppImage could not run tw $args"
          printf "%s\n" "$out" | sed -n "1,10s/^/  /p"
          rc=1
        fi
      done
    fi
    exit "$rc"
  ' || status=$?

if [ "$status" -eq 0 ]; then
  echo "Pass: the clean-container install smoke test, $arch"
else
  echo "Fail: the clean-container install smoke test, $arch"
fi
exit "$status"
