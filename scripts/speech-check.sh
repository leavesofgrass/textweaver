#!/usr/bin/env bash
# speech-check.sh: a plain report on textweaver's speech engines and the
# system speech and audio around them, on Linux and macOS.
#
# Shell: bash (3.2 or later, so it runs on macOS's bash). Read-only unless
# --speak is given. See scripts/README.md, or run with --help.

set -u
set -o pipefail

DRY_RUN=0
SPEAK=0
VOICES=5
TW="${TW:-}"
SKIP=""

usage() {
  cat <<'EOF'
Usage: scripts/speech-check.sh [--speak] [--skip ID,...] [--voices N] [--tw PATH] [--dry-run]

Prints a plain report on speech: textweaver's version, its speech engines
(tw backends), the first few voices of each available engine, and whether
Eloquence was found (tw eloquence).

On Linux it also checks espeak-ng, speech-dispatcher, the PipeWire or
PulseAudio and ALSA audio outputs, and the session details that decide
whether sound reaches your speakers. On macOS it lists the Eloquence voices
that come with the system (say -v '?').

Nothing is spoken unless you add --speak.

Options:
  --speak       Speak one short test sentence with textweaver's default
                engine (off by default).
  --skip ID,... Do not list the voices of these engines, such as eci.
                Listing voices starts the engine.
  --voices N    Show the first N voices of each engine (default: 5).
  --tw PATH     The tw program to check (default: tw on your PATH, then a
                build in this checkout).
  --dry-run     Print each command instead of running it.
  --yes         Accepted for consistency; this script asks nothing.
  -h, --help    Show this help.
EOF
}

say() { printf '%s\n' "$*"; }
die() {
  printf 'Error: %s\n' "$*" >&2
  exit 1
}
have() { command -v "$1" > /dev/null 2>&1; }

# $HOME shown as ~, so the report does not include the user name.
tilde() {
  case $1 in
    "$HOME"/*) printf '%s/%s' '~' "${1#"$HOME"/}" ;;
    *) printf '%s' "$1" ;;
  esac
}

# Replaces the home folder with ~ in command output.
hide_home() {
  if [ -z "${HOME:-}" ] || [ "$HOME" = / ]; then
    cat
    return
  fi
  sed "s|$(printf '%s' "$HOME" | sed 's/[][\.*^$|]/\\&/g')|~|g"
}

section() {
  say ""
  say "== $* =="
}

show_cmd() {
  local out="" a
  for a in "$@"; do
    case $a in
      "" | *[!A-Za-z0-9_./:=@%+,-]*) a="'$(printf '%s' "$a" | sed "s/'/'\\\\''/g")'" ;;
    esac
    out="$out $a"
  done
  printf '%s' "${out# }"
}

# Runs a read-only probe and indents its output. Returns its exit status.
probe() {
  if [ "$DRY_RUN" = 1 ]; then
    say "Would run: $(show_cmd "$@")"
    return 0
  fi
  say "\$ $(show_cmd "$@")"
  local status=0
  "$@" 2>&1 | hide_home | sed 's/^/  /' || status=$?
  return "$status"
}

# Like probe, but limits the output to N lines and says how many there were.
probe_head() {
  local n="$1"
  shift
  if [ "$DRY_RUN" = 1 ]; then
    say "Would run: $(show_cmd "$@"), and show the first $n lines"
    return 0
  fi
  say "\$ $(show_cmd "$@")"
  local out total
  out="$("$@" 2>&1)" || {
    printf '%s\n' "$out" | hide_home | sed 's/^/  /'
    return 1
  }
  total="$(printf '%s\n' "$out" | grep -c .)"
  printf '%s\n' "$out" | head -n "$n" | hide_home | sed 's/^/  /'
  if [ "$total" -gt "$n" ]; then
    say "  ... and $((total - n)) more lines."
  fi
}

with_timeout() {
  if have timeout; then
    timeout 15 "$@"
  else
    "$@"
  fi
}

while [ "$#" -gt 0 ]; do
  case $1 in
    --speak) SPEAK=1 ;;
    --skip)
      [ "$#" -ge 2 ] || die "--skip needs an engine id."
      SKIP="$SKIP,$2"
      shift
      ;;
    --voices)
      [ "$#" -ge 2 ] || die "--voices needs a number."
      VOICES="$2"
      shift
      ;;
    --tw)
      [ "$#" -ge 2 ] || die "--tw needs a path."
      TW="$2"
      shift
      ;;
    --dry-run) DRY_RUN=1 ;;
    --yes | -y) ;;
    -h | --help)
      usage
      exit 0
      ;;
    *) die "Unknown option $1. Run with --help to see the options." ;;
  esac
  shift
done
case $VOICES in
  '' | *[!0-9]*) die "--voices needs a number." ;;
esac

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
OS="$(uname -s)"

# Finds tw: --tw, PATH, then this checkout's release or debug build.
find_tw() {
  if [ -n "$TW" ]; then
    return 0
  fi
  if have tw; then
    TW="$(command -v tw)"
    return 0
  fi
  local d
  for d in "$SCRIPT_DIR/../target/release" "$SCRIPT_DIR/../target/debug" "${CARGO_TARGET_DIR:-/nonexistent}/release"; do
    if [ -x "$d/tw" ]; then
      TW="$(cd "$d" && pwd)/tw"
      return 0
    fi
  done
  return 1
}

say "textweaver speech check"
say "This report is read-only. It speaks only with --speak."
if [ "$DRY_RUN" = 1 ]; then
  say "Dry run: each command is printed instead of run."
fi

section "textweaver"
if find_tw; then
  say "tw: $(tilde "$TW")"
  probe "$TW" --version

  section "Speech engines (tw backends)"
  probe "$TW" backends

  section "Voices"
  if [ "$DRY_RUN" = 1 ]; then
    say "Would run: $TW voices --backend ID, for each available engine except null, and show the first $VOICES voices"
  else
    available="$("$TW" backends 2> /dev/null | sed -n 's/^\([A-Za-z0-9_-]*\): .*\. Available\..*/\1/p')"
    shown=0
    for id in $available; do
      [ "$id" = null ] && continue
      case ",$SKIP," in
        *",$id,"*)
          say "Engine $id: skipped, as asked."
          continue
          ;;
      esac
      shown=1
      say "Engine $id:"
      probe_head "$VOICES" "$TW" voices --backend "$id" || say "  Could not list the voices of $id."
    done
    if [ "$shown" = 0 ]; then
      say "No speech engine is available apart from null (silence). The sections below may say why."
    fi
  fi

  section "Eloquence (tw eloquence)"
  probe "$TW" eloquence
else
  say "tw was not found on your PATH or in this checkout. Install textweaver (scripts/install-linux.sh or scripts/install-macos.sh), or pass --tw PATH."
fi

linux_checks() {
  section "espeak-ng"
  if have espeak-ng || [ "$DRY_RUN" = 1 ]; then
    probe espeak-ng --version
    probe_head "$VOICES" espeak-ng --voices=en
  else
    say "espeak-ng is not installed. textweaver's espeak engine needs it (package espeak-ng)."
  fi

  section "speech-dispatcher"
  if ! have spd-say && have speech-dispatcher && [ "$DRY_RUN" = 0 ]; then
    probe_head 1 speech-dispatcher --version || true
    say "speech-dispatcher is installed, but its spd-say tool is not, so it cannot be checked further. On Fedora, install speech-dispatcher-utils."
  elif have spd-say || [ "$DRY_RUN" = 1 ]; then
    probe_head 1 speech-dispatcher --version || true
    if [ -n "${XDG_RUNTIME_DIR:-}" ] && [ -S "$XDG_RUNTIME_DIR/speech-dispatcher/speechd.sock" ]; then
      say "The speech-dispatcher socket exists, so a speech-dispatcher is running for this session."
    else
      say "No speech-dispatcher socket was found in XDG_RUNTIME_DIR. spd-say starts one on demand."
    fi
    say "Output modules (the synthesizers speech-dispatcher can use):"
    probe with_timeout spd-say -O || say "  spd-say could not reach speech-dispatcher."
    say "Voices of the default module:"
    probe_head "$VOICES" with_timeout spd-say -L || say "  spd-say could not list voices."
  else
    say "speech-dispatcher is not installed (packages speech-dispatcher, and speech-dispatcher-espeak-ng on Debian and Fedora). textweaver's speechd engine needs it."
  fi

  section "Audio output"
  if have pactl || [ "$DRY_RUN" = 1 ]; then
    if [ "$DRY_RUN" = 1 ]; then
      say "Would run: pactl info, and show the server name, default sink, and sample spec"
    elif pactl info > /dev/null 2>&1; then
      pactl info 2> /dev/null | grep -E '^(Server Name|Server Version|Default Sink|Default Sample Specification):' | sed 's/^/  /'
      case "$(pactl info 2> /dev/null | grep '^Server Name:')" in
        *PipeWire*) say "Sound goes through PipeWire, with its PulseAudio layer." ;;
        *) say "Sound goes through PulseAudio." ;;
      esac
    else
      say "pactl is installed but cannot reach a sound server. Sound from textweaver may not play."
    fi
  else
    say "pactl is not installed, so PipeWire or PulseAudio could not be checked."
  fi
  if have pw-cli && [ "$DRY_RUN" = 0 ]; then
    say "PipeWire tools are installed."
  fi
  if [ -r /proc/asound/cards ] || [ "$DRY_RUN" = 1 ]; then
    say "ALSA sound cards:"
    probe cat /proc/asound/cards
  else
    say "No ALSA sound cards are visible (no /proc/asound/cards). In a container or over SSH that is normal."
  fi
  if [ -n "${PULSE_SERVER:-}" ]; then
    say "PULSE_SERVER is set, so sound goes to that PulseAudio server."
  fi

  section "Session"
  if [ -n "${XDG_RUNTIME_DIR:-}" ]; then
    say "XDG_RUNTIME_DIR is set. speech-dispatcher and the sound server use it to find this login session."
  else
    say "XDG_RUNTIME_DIR is not set. speech-dispatcher and PipeWire or PulseAudio may not work. That is common with su, sudo, cron, or some SSH setups; log in normally, or use systemd's machinectl shell."
  fi
  if [ -n "${WAYLAND_DISPLAY:-}" ] || [ -n "${DISPLAY:-}" ]; then
    say "A graphical session is present. textweaver does not need one: it runs in any terminal, including a text console."
  else
    say "No graphical session (DISPLAY and WAYLAND_DISPLAY are not set). That is fine: textweaver runs in any terminal. Sound still needs a running sound server or ALSA."
  fi
  if [ -n "${SSH_CONNECTION:-}" ]; then
    say "This is an SSH session. Speech plays on the remote machine's speakers, not on yours."
  fi
  say "TERM is ${TERM:-not set}."
}

mac_checks() {
  section "Apple voices"
  if [ "$DRY_RUN" = 1 ]; then
    say "Would run: say -v '?', and show the Eloquence voices (Eddy, Flo, Grandma, Grandpa, Reed, Rocko, Sandy, Shelley)"
    return 0
  fi
  if ! have say; then
    say "The say command is missing, which is unusual on macOS."
    return 0
  fi
  local all eloq
  all="$(say -v '?' 2> /dev/null)"
  say "Apple voices installed: $(printf '%s\n' "$all" | grep -c .)."
  eloq="$(printf '%s\n' "$all" | grep -E '^(Eddy|Flo|Grandma|Grandpa|Reed|Rocko|Sandy|Shelley) ' || true)"
  if [ -n "$eloq" ]; then
    say "Eloquence voices (textweaver uses Reed by default when it is installed):"
    printf '%s\n' "$eloq" | head -n 40 | sed 's/^/  /'
  else
    say "No Eloquence voices were found. Add them in System Settings, Accessibility, Spoken Content, System Voice, Manage Voices: look for Reed, Eddy, and the others."
  fi
}

case $OS in
  Linux) linux_checks ;;
  Darwin) mac_checks ;;
  *) say "" && say "No system checks for $OS." ;;
esac

if [ "$SPEAK" = 1 ]; then
  section "Speaking a test sentence"
  if [ -n "$TW" ]; then
    probe "$TW" speak "This is the textweaver speech check. If you can hear this, speech works."
  else
    say "tw was not found, so nothing can be spoken."
  fi
fi

section "End of report"
say "Copy this report into a bug report if speech does not work. It holds no personal data: your home folder is shown as ~."
