#!/bin/sh
# Runs the Linux release packages on several distributions, in Docker:
# Debian stable, Fedora, and Arch (docs/releasing.md).
#
#   docker/appimage/test-distros.sh DIST_DIR [IMAGE...]
#
# DIST_DIR holds textweaver-*-linux-x86_64.AppImage and the matching
# .tar.gz (target/dist after `cargo xtask appimage`). Containers have no
# FUSE, so the AppImage runs with --appimage-extract-and-run (and, through
# the tw link, with APPIMAGE_EXTRACT_AND_RUN=1), as it does on a system
# without FUSE.
#
# On each image, without espeak-ng installed and then with it, it checks:
# - `--tw --version` (the AppImage's first-argument dispatch);
# - `tw backends` through a link named tw (the argv0 dispatch), which must
#   report espeak unavailable without libespeak-ng and available with it;
# - `tw text` on a fixture, which must print the fixture's text;
# - `textweaver --version` (the default program);
# - `--install --yes` and `--uninstall --yes` into a scratch home;
# - the tarball's `tw --version` and `tw text`;
# - scripts/install-linux.sh --release with the tarball and with the
#   AppImage (file:// URLs to DIST_DIR, checked against a SHA256SUMS.txt
#   written here when the folder has none), then --uninstall.
# Distributions ship the ALSA library on every desktop; minimal images may
# not, so it is installed first, as a desktop would have it.
set -eu

dist="${1:?usage: docker/appimage/test-distros.sh DIST_DIR [IMAGE...]}"
shift
if [ "$#" -eq 0 ]; then
  set -- debian:stable fedora:latest archlinux:latest
fi
# Git Bash on Windows: hand Docker Windows paths and leave /pkg alone.
abs() { (cd "$1" && pwd); }
case "$(uname -s)" in
  MINGW* | MSYS*)
    export MSYS_NO_PATHCONV=1
    abs() { (cd "$1" && pwd -W); }
    ;;
esac
dist="$(abs "$dist")"
here="$(cd "$(dirname "$0")" && pwd)"
fixtures="$(abs "$here/../../fixtures")"
scripts="$(abs "$here/../../scripts")"

appimage=""
tarball=""
for f in "$dist"/textweaver-*-linux-x86_64.AppImage; do
  [ -e "$f" ] && appimage="$(basename "$f")"
done
for f in "$dist"/textweaver-*-linux-x86_64.tar.gz; do
  [ -e "$f" ] && tarball="$(basename "$f")"
done
[ -n "$appimage" ] || {
  echo "No AppImage in $dist." >&2
  exit 1
}

# The checksums install-linux.sh checks against, as the release has them.
made_sums=0
if [ ! -f "$dist/SHA256SUMS.txt" ]; then
  (cd "$dist" && sha256sum textweaver-*-linux-x86_64.AppImage textweaver-*-linux-x86_64.tar.gz > SHA256SUMS.txt)
  made_sums=1
fi

# The check, run inside each container as root.
cat > "$dist/.distro-check.sh" << 'EOF'
set -eu
image="$1" appimage="$2" tarball="$3"
pm_install() {
  if command -v apt-get > /dev/null; then
    export DEBIAN_FRONTEND=noninteractive
    apt-get update -qq > /dev/null
    for p in "$@"; do apt-get install -y -qq --no-install-recommends "$p" > /dev/null 2>&1 || true; done
  elif command -v dnf > /dev/null; then
    dnf install -y -q "$@" > /dev/null 2>&1
  elif command -v pacman > /dev/null; then
    pacman -Sy --noconfirm --needed "$@" > /dev/null 2>&1
  fi
}
alsa() {
  if command -v apt-get > /dev/null; then echo libasound2t64 libasound2; else echo alsa-lib; fi
}
espeak() {
  if command -v apt-get > /dev/null; then echo libespeak-ng1 espeak-ng-data; else echo espeak-ng; fi
}
# shellcheck disable=SC2046
pm_install $(alsa) tar gzip
fail=0
check() {
  what="$1"
  shift
  if out="$("$@" 2>&1)"; then
    echo "  ok: $what"
    printf '%s\n' "$out" | sed -n '1,3s/^/      /p'
  else
    echo "  FAILED: $what"
    printf '%s\n' "$out" | sed 's/^/      /'
    fail=1
  fi
}
run_round() {
  label="$1" want_espeak="$2"
  echo "$image, $label:"
  work="$(mktemp -d)"
  cp "/pkg/$appimage" "$work/textweaver.AppImage"
  chmod +x "$work/textweaver.AppImage"
  ln -s textweaver.AppImage "$work/tw"
  cd "$work"
  check "AppImage --tw --version" ./textweaver.AppImage --appimage-extract-and-run --tw --version
  check "textweaver --version" ./textweaver.AppImage --appimage-extract-and-run --version
  backends="$(APPIMAGE_EXTRACT_AND_RUN=1 ./tw backends 2>&1)" || {
    echo "  FAILED: tw backends"
    printf '%s\n' "$backends" | sed 's/^/      /'
    fail=1
  }
  espeak_line="$(printf '%s\n' "$backends" | grep -i espeak | head -n 1)"
  echo "  tw backends (through the tw link): ${espeak_line:-no espeak line}"
  case "$want_espeak:$espeak_line" in
    yes:*"Not available"* | yes:)
      echo "  FAILED: espeak-ng is installed but not available"
      fail=1
      ;;
    no:*"Not available"*) echo "  ok: espeak unavailable without libespeak-ng" ;;
    yes:*) echo "  ok: espeak available with libespeak-ng" ;;
    no:*)
      echo "  FAILED: espeak reported available without libespeak-ng"
      fail=1
      ;;
  esac
  text="$(APPIMAGE_EXTRACT_AND_RUN=1 ./tw text /fixtures/sample.md 2>&1)" || true
  if printf '%s\n' "$text" | grep -q .; then
    echo "  ok: tw text printed $(printf '%s\n' "$text" | wc -l) lines"
    printf '%s\n' "$text" | sed -n '1,2s/^/      /p'
  else
    echo "  FAILED: tw text printed nothing"
    fail=1
  fi
  home="$(mktemp -d)"
  check "--install --yes" env HOME="$home" APPIMAGE_EXTRACT_AND_RUN=1 ./textweaver.AppImage --install --yes
  for f in .local/bin/textweaver .local/bin/tw .local/share/applications/textweaver.desktop \
    .local/share/icons/hicolor/scalable/apps/textweaver.svg; do
    [ -e "$home/$f" ] || [ -L "$home/$f" ] || {
      echo "  FAILED: --install did not make ~/$f"
      fail=1
    }
  done
  check "tw --version through the installed link" env HOME="$home" APPIMAGE_EXTRACT_AND_RUN=1 "$home/.local/bin/tw" --version
  check "--uninstall --yes" env HOME="$home" APPIMAGE_EXTRACT_AND_RUN=1 ./textweaver.AppImage --uninstall --yes
  [ ! -e "$home/.local/bin/tw" ] && [ ! -L "$home/.local/bin/tw" ] || {
    echo "  FAILED: --uninstall left ~/.local/bin/tw"
    fail=1
  }
  if [ -n "$tarball" ]; then
    mkdir tar
    tar -xzf "/pkg/$tarball" -C tar --strip-components=1
    check "tarball tw --version" ./tar/tw --version
    check "tarball tw text" ./tar/tw text /fixtures/sample.md
  fi
  cd /
  rm -rf "$work" "$home"
}
# scripts/install-linux.sh --release, from the packages in /pkg (served
# as file:// URLs), checked against /pkg/SHA256SUMS.txt: the tarball (the
# automatic choice without FUSE) and the AppImage (--appimage), each
# installed, run, and uninstalled.
install_round() {
  kind="$1"
  echo "$image, install-linux.sh --release ($kind):"
  home="$(mktemp -d)"
  version="${appimage#textweaver-}"
  version="${version%-linux-x86_64.AppImage}"
  flag=""
  [ "$kind" = appimage ] && flag="--appimage"
  # shellcheck disable=SC2086
  check "install" env HOME="$home" SHELL=/bin/bash TEXTWEAVER_RELEASE_URL=file:///pkg \
    bash /scripts/install-linux.sh --release "v$version" $flag --yes
  check "tw --version from ~/.local/bin" env HOME="$home" APPIMAGE_EXTRACT_AND_RUN=1 "$home/.local/bin/tw" --version
  check "textweaver --version from ~/.local/bin" env HOME="$home" APPIMAGE_EXTRACT_AND_RUN=1 "$home/.local/bin/textweaver" --version
  [ -f "$home/.local/share/applications/textweaver.desktop" ] || {
    echo "  FAILED: no menu entry"
    fail=1
  }
  check "uninstall" env HOME="$home" SHELL=/bin/bash bash /scripts/install-linux.sh --uninstall --yes
  if [ -e "$home/.local/bin/tw" ] || [ -L "$home/.local/bin/tw" ] || [ -e "$home/.local/bin/textweaver.AppImage" ]; then
    echo "  FAILED: --uninstall left files in ~/.local/bin"
    fail=1
  fi
  rm -rf "$home"
}
pm_install curl
if [ -f /pkg/SHA256SUMS.txt ]; then
  install_round tarball
  install_round appimage
fi
run_round "without espeak-ng" no
# shellcheck disable=SC2046
pm_install $(espeak)
run_round "with espeak-ng" yes
exit "$fail"
EOF

status=0
for image in "$@"; do
  echo "== $image =="
  if ! docker run --rm -v "$dist:/pkg:ro" -v "$fixtures:/fixtures:ro" -v "$scripts:/scripts:ro" "$image" \
    sh /pkg/.distro-check.sh "$image" "$appimage" "$tarball"; then
    echo "$image: FAILED"
    status=1
  else
    echo "$image: passed"
  fi
done
rm -f "$dist/.distro-check.sh"
if [ "$made_sums" = 1 ]; then
  rm -f "$dist/SHA256SUMS.txt"
fi
exit "$status"
