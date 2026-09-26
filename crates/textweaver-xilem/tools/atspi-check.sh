#!/usr/bin/env bash
# The AT-SPI check for the Xilem GUI on Linux, with no display and no
# screen reader: Xvfb, a private D-Bus session, the AT-SPI bus, and Mesa's
# software Vulkan/GL for Vello. It launches textweaver-xilem reading
# fixtures/sample.md with the silent `paced` backend and runs
# atspi-dump.py against it.
#
# Needs: xvfb, dbus, at-spi2-core, python3-pyatspi, python3-gi,
# mesa-vulkan-drivers (or libgl1-mesa-dri and libegl1), libxkbcommon-x11-0.
# In the dev container (the image is not changed; this installs into the
# throwaway container):
#
#   docker compose -p textweaver run --rm -T -e CARGO_TARGET_DIR=/target/w3b dev \
#     bash crates/textweaver-xilem/tools/atspi-check.sh --install
#
# Accessibility is turned on by the AT-SPI bus being present; AccessKit
# 0.22 and later find it without the old gsettings workaround (the check
# says whether toolkit-accessibility was set: it is not).
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../.." && pwd)"
target="${CARGO_TARGET_DIR:-$repo/target}"
exe="${EXE:-$target/debug/textweaver-xilem}"
doc="${DOC:-$repo/fixtures/sample.md}"
seconds="${SECONDS_TO_READ:-6}"

if [[ "${1:-}" == "--install" ]]; then
  export DEBIAN_FRONTEND=noninteractive
  apt-get update -qq
  apt-get install -y -qq --no-install-recommends \
    xvfb xauth dbus dbus-x11 at-spi2-core python3-pyatspi python3-gi gir1.2-atspi-2.0 \
    mesa-vulkan-drivers libvulkan1 libgl1-mesa-dri libegl1 \
    libxkbcommon-x11-0 libxcursor1 libxrandr2 libxi6 libx11-xcb1 >/dev/null
fi

if [[ ! -x "$exe" ]]; then
  (cd "$repo" && cargo build -p textweaver-xilem)
fi

run() {
  export NO_AT_BRIDGE=0
  # Start the AT-SPI bus for this session.
  /usr/libexec/at-spi-bus-launcher --launch-immediately >/dev/null 2>&1 &
  sleep 1
  echo "- toolkit-accessibility gsetting: $(gsettings get org.gnome.desktop.interface toolkit-accessibility 2>/dev/null || echo 'not set (no gsettings)')"
  home="$(mktemp -d)"
  "$exe" "$doc" --backend paced --read --exit-after $((seconds + 6)) --home "$home" \
    --log-file "$home/gui.log" &
  gui=$!
  status=0
  python3 "$here/atspi-dump.py" --pid "$gui" --seconds "$seconds" || status=$?
  wait "$gui" || true
  echo
  echo "## GUI log"
  echo
  echo '```'
  cat "$home/gui.log" || true
  echo '```'
  return $status
}

export -f run
export exe doc seconds here
xvfb-run -a -s "-screen 0 1280x900x24" dbus-run-session -- bash -c run
