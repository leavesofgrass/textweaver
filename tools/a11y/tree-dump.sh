#!/usr/bin/env bash
# Dump the Xilem GUI's accessibility tree with accessibility-cli, on Linux
# (AT-SPI, under Xvfb with a private D-Bus session) or macOS (the AX API).
# Windows has tree-dump.ps1. ADR-0039 describes the check.
#
#   bash tools/a11y/tree-dump.sh OUT_DIR
#
# It opens fixtures/t/reading.md with the silent `paced` backend and
# --background (never activated, off screen), waits until the tree holds
# the document, and writes into OUT_DIR:
#   raw.json      accessibility-cli --json, as dumped
#   raw-tree.txt  accessibility-cli's own text tree, for a person to read
#   tree.txt      the normalized tree (tools/a11y/tree_report.py)
#   gui.log       the GUI's log
#
# Environment: EXE (default target/debug/textweaver-xilem), DOC, A11Y_CLI
# (default accessibility-cli on the PATH), WAIT_SECONDS (default 40: the
# longest it waits for the tree). Runs on CI runners; it plays no audio.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
out="${1:?usage: tree-dump.sh OUT_DIR}"
exe="${EXE:-${CARGO_TARGET_DIR:-$repo/target}/debug/textweaver-xilem}"
doc="${DOC:-$repo/fixtures/t/reading.md}"
cli="${A11Y_CLI:-accessibility-cli}"
wait_seconds="${WAIT_SECONDS:-40}"
python="$(command -v python3 || command -v python)"

case "$(uname -s)" in
  Linux) platform=linux ;;
  Darwin) platform=mac ;;
  *) echo "Fail: tree-dump.sh runs on Linux and macOS; on Windows use tree-dump.ps1."; exit 1 ;;
esac

dump() {
  mkdir -p "$out"
  local home gui found=0 deadline listener=""
  home="$(mktemp -d)"
  local background=(--background)
  if [[ $platform == linux ]]; then
    export NO_AT_BRIDGE=0
    export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-$(mktemp -d)}"
    /usr/libexec/at-spi-bus-launcher --launch-immediately >/dev/null 2>&1 &
    sleep 1
    # AccessKit's AT-SPI adapter shows the window only while assistive
    # technology is active. The AT-SPI check turns that on by listening for
    # events; accessibility-cli only reads, and the second run found no
    # application on the bus at all, not even through pyatspi. So say that
    # a screen reader is on, and keep a listener running during the dump.
    for prop in IsEnabled ScreenReaderEnabled; do
      dbus-send --session --print-reply --dest=org.a11y.Bus /org/a11y/bus \
        org.freedesktop.DBus.Properties.Set string:org.a11y.Status "string:$prop" \
        variant:boolean:true >/dev/null 2>&1 || true
    done
    timeout $((wait_seconds + 30)) "$python" - >/dev/null 2>&1 <<'EOF' &
import pyatspi
pyatspi.Registry.registerEventListener(lambda e: None, "object:state-changed:focused")
pyatspi.Registry.start()
EOF
    listener=$!
    sleep 1
    # Xvfb is a private display with no one at it, so the window opens as
    # it does in the AT-SPI check, without --background (the first run
    # found no application by the GUI's process id with it).
    background=()
  fi
  "$exe" "$doc" --backend paced "${background[@]}" --exit-after $((wait_seconds + 30)) \
    --home "$home" --log-file "$out/gui.log" &
  gui=$!
  deadline=$((SECONDS + wait_seconds))
  while ((SECONDS < deadline)); do
    sleep 2
    if ! kill -0 "$gui" 2>/dev/null; then
      echo "Fail: the GUI exited before its tree could be dumped."
      break
    fi
    # The tree counts once it holds the document's title.
    if "$cli" --platform "$platform" --pid "$gui" --json >"$out/raw.json" 2>"$out/cli-errors.txt" \
      && grep -q '"role"' "$out/raw.json" && grep -q 'Reading check' "$out/raw.json"; then
      found=1
      break
    fi
  done
  "$cli" --platform "$platform" --pid "$gui" >"$out/raw-tree.txt" 2>&1 || true
  if [[ $found == 0 ]]; then
    # What accessibility-cli can see while the GUI still runs, to compare
    # with the GUI's process id.
    echo "GUI process id: $gui" >"$out/windows.txt"
    "$cli" --platform "$platform" --list-windows >>"$out/windows.txt" 2>&1 || true
    if [[ $platform == linux ]]; then
      # And what pyatspi sees on the same bus, as the AT-SPI check reads it.
      "$python" - >>"$out/windows.txt" 2>&1 <<'EOF' || true
import pyatspi
desktop = pyatspi.Registry.getDesktop(0)
print("pyatspi sees:")
for i in range(desktop.childCount):
    app = desktop.getChildAtIndex(i)
    try:
        print(f"- {app.name!r} process {app.get_process_id()}")
    except Exception as e:
        print(f"- (unreadable: {e})")
EOF
    fi
  fi
  kill "$gui" 2>/dev/null || true
  wait "$gui" 2>/dev/null || true
  if [[ -n $listener ]]; then
    kill "$listener" 2>/dev/null || true
  fi
  if [[ $found == 0 ]]; then
    echo "Fail: no tree with the document in ${wait_seconds} seconds. accessibility-cli said:"
    cat "$out/cli-errors.txt" 2>/dev/null || true
    cat "$out/windows.txt" 2>/dev/null || true
    return 1
  fi
  "$python" "$here/tree_report.py" normalize "$out/raw.json" "$out/tree.txt"
}

if [[ $platform == linux && "${2:-}" != "--inner" ]]; then
  export EXE="$exe" DOC="$doc" A11Y_CLI="$cli" WAIT_SECONDS="$wait_seconds"
  xvfb-run -a -s "-screen 0 1280x900x24" dbus-run-session -- bash "$0" "$out" --inner
else
  dump
fi
