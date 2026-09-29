#!/usr/bin/env bash
# The AT-SPI reading session (tools/a11y/atspi-session.py) under Xvfb, with
# a private D-Bus session, the AT-SPI bus, and Orca running beside it with a
# debug log, so the report lists what Orca said at each step (ADR-0039).
# It does not rely on AT-SPI's Collection interface (AccessKit pull request
# 758 is a draft).
#
#   bash tools/a11y/orca-session.sh OUT_DIR
#
# Needs xvfb, dbus, at-spi2-core, python3-pyatspi, python3-gi, xdotool,
# orca, and Mesa's software GPU libraries (the GUI workflow's list). The
# GUI is the hybrid-renderer build, which suits Mesa. Runs on CI runners;
# Orca's speech goes to a runner with no sound card. Set ORCA=0 to run the
# session without Orca.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
out="${1:?usage: orca-session.sh OUT_DIR}"
exe="${EXE:-${CARGO_TARGET_DIR:-$repo/target}/debug/textweaver-xilem}"
orca="${ORCA:-1}"

run() {
  mkdir -p "$out"
  export NO_AT_BRIDGE=0
  export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-$(mktemp -d)}"
  /usr/libexec/at-spi-bus-launcher --launch-immediately >/dev/null 2>&1 &
  sleep 1
  local orca_pid="" orca_args=() status=0
  if [[ $orca == 1 ]]; then
    # Orca reads its settings from here, so the runner's user is untouched.
    export XDG_CONFIG_HOME="$out/orca-config"
    mkdir -p "$XDG_CONFIG_HOME"
    orca --replace --debug-file="$out/orca-debug.log" >"$out/orca-stdout.txt" 2>&1 &
    orca_pid=$!
    # Up to 20 seconds for Orca to start writing its log.
    for _ in $(seq 1 20); do
      [[ -s "$out/orca-debug.log" ]] && break
      sleep 1
    done
    orca_args=(--orca-log "$out/orca-debug.log")
  fi
  timeout 150 python3 "$here/atspi-session.py" --exe "$exe" --out "$out/atspi-session.md" \
    "${orca_args[@]}" || status=$?
  if [[ -n $orca_pid ]]; then
    kill "$orca_pid" 2>/dev/null || true
    wait "$orca_pid" 2>/dev/null || true
  fi
  return $status
}

if [[ "${2:-}" == "--inner" ]]; then
  run
else
  export EXE="$exe" ORCA="$orca"
  xvfb-run -a -s "-screen 0 1280x900x24" dbus-run-session -- bash "$0" "$out" --inner
fi
