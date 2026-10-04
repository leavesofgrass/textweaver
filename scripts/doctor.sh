#!/usr/bin/env bash
# doctor.sh: one plain-text report about this system and textweaver, to
# paste into a bug report. Linux and macOS.
#
# Shell: bash (3.2 or later). Read-only. Nothing personal: no file contents
# and no environment dump; the home folder is shown as ~. See
# scripts/README.md, or run with --help.

set -u
set -o pipefail

DRY_RUN=0
OUT=""
TW="${TW:-}"

usage() {
  cat <<'EOF'
Usage: scripts/doctor.sh [--out FILE] [--tw PATH] [--dry-run]

Prints one plain-text report for bug reports: the operating system and its
version, the terminal (TERM, COLORTERM, size), the locale, the screen reader
if one is running, the Rust toolchain if present, where textweaver is
installed and where it keeps its settings, its speech engines, whether the
engine hosts and the pronunciation dictionaries sit beside the programs, and
the optional tools it can use.

It reads nothing personal: no file contents, and no environment dump (only
the names of TEXTWEAVER_ variables that are set). Your home folder is shown
as ~.

Options:
  --out FILE   Also write the report to FILE.
  --tw PATH    The tw program to check (default: tw on your PATH, then a
               build in this checkout).
  --dry-run    Print each command instead of running it.
  --yes        Accepted for consistency; this script asks nothing.
  -h, --help   Show this help.
EOF
}

say() { printf '%s\n' "$*"; }
die() {
  printf 'Error: %s\n' "$*" >&2
  exit 1
}
have() { command -v "$1" > /dev/null 2>&1; }

while [ "$#" -gt 0 ]; do
  case $1 in
    --out)
      [ "$#" -ge 2 ] || die "--out needs a file name."
      OUT="$2"
      shift
      ;;
    --out=*) OUT="${1#*=}" ;;
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

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
OS="$(uname -s)"

hide_home() {
  if [ -z "${HOME:-}" ] || [ "$HOME" = / ]; then
    cat
    return
  fi
  sed "s|$(printf '%s' "$HOME" | sed 's/[][\.*^$|]/\\&/g')|~|g"
}

tilde() { printf '%s\n' "$1" | hide_home; }

# The first line a command prints, or "not installed".
first_line() {
  if ! have "$1"; then
    say "not installed"
    return 0
  fi
  if [ "$DRY_RUN" = 1 ]; then
    say "(would run: $*)"
    return 0
  fi
  "$@" 2>&1 | head -n 1 | hide_home
}

# Runs a command and indents its output.
indent() {
  if [ "$DRY_RUN" = 1 ]; then
    say "  (would run: $*)"
    return 0
  fi
  "$@" 2>&1 | hide_home | sed 's/^/  /'
}

# Follows symbolic links (readlink -f is missing on older macOS).
real_path() {
  local p="$1" l
  while [ -L "$p" ]; do
    l="$(readlink "$p")"
    case $l in
      /*) p="$l" ;;
      *) p="$(dirname "$p")/$l" ;;
    esac
  done
  printf '%s/%s' "$(cd "$(dirname "$p")" && pwd -P)" "$(basename "$p")"
}

yes_no() {
  if "$@"; then say yes; else say no; fi
}

running() {
  if have pgrep; then
    pgrep -x "$1" > /dev/null 2>&1
  else
    # shellcheck disable=SC2009 # pgrep is missing on this system.
    ps -A -o comm= 2> /dev/null | grep -qx "$1"
  fi
}

find_tw() {
  if [ -n "$TW" ]; then
    return 0
  fi
  if have tw; then
    TW="$(command -v tw)"
    return 0
  fi
  local d
  for d in "$SCRIPT_DIR/../target/release" "$SCRIPT_DIR/../target/debug" "$SCRIPT_DIR/../../../bin"; do
    if [ -x "$d/tw" ]; then
      TW="$(cd "$d" && pwd)/tw"
      return 0
    fi
  done
  return 1
}

report() {
  say "textweaver doctor report"
  say "Date: $(LC_ALL=C date '+%A, %B %d, %Y')"
  if [ "$DRY_RUN" = 1 ]; then
    say "Dry run: commands are printed instead of run."
  fi

  say ""
  say "== System =="
  if [ "$OS" = Darwin ]; then
    say "OS: macOS $(sw_vers -productVersion 2> /dev/null || echo unknown) (build $(sw_vers -buildVersion 2> /dev/null || echo unknown))"
  elif [ -r /etc/os-release ]; then
    # shellcheck disable=SC1091
    say "OS: $(. /etc/os-release && printf '%s' "${PRETTY_NAME:-${NAME:-Linux}}")"
  else
    say "OS: $OS"
  fi
  say "Kernel: $(uname -sr)"
  say "Architecture: $(uname -m)"
  if [ -r /proc/version ] && grep -qi microsoft /proc/version; then
    say "Running under WSL (Windows Subsystem for Linux)."
  fi
  if [ -f /.dockerenv ] || [ -f /run/.containerenv ]; then
    say "Running in a container."
  fi

  say ""
  say "== Terminal =="
  say "TERM: ${TERM:-not set}"
  say "COLORTERM: ${COLORTERM:-not set}"
  say "TERM_PROGRAM: ${TERM_PROGRAM:-not set}${TERM_PROGRAM_VERSION:+ $TERM_PROGRAM_VERSION}"
  say "NO_COLOR: $([ -n "${NO_COLOR:-}" ] && echo set || echo not set)"
  say "Output is a terminal: $(yes_no test -t 1)"
  if [ -t 1 ] && have tput; then
    say "Size: $(tput cols 2> /dev/null || echo '?') columns by $(tput lines 2> /dev/null || echo '?') lines"
  fi
  say "Inside tmux: $([ -n "${TMUX:-}" ] && echo yes || echo no). Inside screen: $([ -n "${STY:-}" ] && echo yes || echo no)."
  say "SSH session: $([ -n "${SSH_CONNECTION:-}" ] && echo yes || echo no)"
  say "Graphical session: $({ [ -n "${DISPLAY:-}" ] || [ -n "${WAYLAND_DISPLAY:-}" ]; } && echo yes || echo no)"
  say "Shell: ${SHELL##*/}"

  say ""
  say "== Locale =="
  say "LANG: ${LANG:-not set}"
  say "LC_ALL: ${LC_ALL:-not set}"
  say "LC_CTYPE: ${LC_CTYPE:-not set}"
  if have locale; then
    say "Character set: $(locale charmap 2> /dev/null || echo unknown)"
  fi

  say ""
  say "== Screen reader =="
  if [ "$OS" = Darwin ]; then
    say "VoiceOver running: $(yes_no running VoiceOver)"
  else
    say "Orca running: $(yes_no running orca)"
    say "Fenrir running: $(yes_no running fenrir)"
    say "Speakup loaded: $(yes_no test -d /sys/module/speakup)"
    say "BRLTTY running: $(yes_no running brltty)"
  fi

  say ""
  say "== Rust =="
  local v
  for v in rustc cargo; do
    case "$(first_line "$v" --version)" in
      error:*) say "$v: installed by rustup, with no default toolchain (a checkout picks its own from rust-toolchain.toml)" ;;
      *) say "$v: $(first_line "$v" --version)" ;;
    esac
  done
  if have rustup; then
    if [ "$DRY_RUN" = 1 ]; then
      say "rustup toolchains: (would run: rustup toolchain list)"
    else
      say "rustup toolchains: $(rustup toolchain list 2> /dev/null | tr '\n' ' ')"
    fi
  else
    say "rustup: not installed"
  fi

  say ""
  say "== textweaver =="
  if find_tw; then
    local real dir
    real="$(real_path "$TW")"
    dir="$(dirname "$real")"
    say "tw: $(tilde "$TW")"
    if [ "$real" != "$TW" ]; then
      say "tw links to: $(tilde "$real")"
    fi
    say "tw version: $(first_line "$TW" --version)"
    if [ -x "$dir/textweaver" ]; then
      say "textweaver version: $(first_line "$dir/textweaver" --version)"
    else
      say "textweaver: not found beside tw"
    fi
    if [ -x "$dir/textweaver-gui" ]; then
      say "Window version: $(first_line "$dir/textweaver-gui" --version)"
    else
      say "Window (textweaver-gui): not installed beside tw"
    fi
    local manifest
    for manifest in "$dir/../../share/textweaver/install-manifest.txt" "$dir/../share/textweaver/install-manifest.txt"; do
      if [ -f "$manifest" ]; then
        say "Installed by: $(grep -E '^kind=' "$manifest" | cut -d= -f2) install, version $(grep -E '^version=' "$manifest" | cut -d= -f2)"
        break
      fi
    done

    say ""
    say "Settings (tw settings path):"
    indent "$TW" settings path

    say ""
    say "Speech engines (tw backends):"
    indent "$TW" backends

    say ""
    say "Optional components (tw components list):"
    indent "$TW" components list

    say ""
    say "Text recognition (tw ocr status):"
    indent "$TW" ocr status

    say ""
    say "Engine hosts beside the programs, in $(tilde "$dir"):"
    local hosts h
    if [ "$OS" = Darwin ]; then
      hosts=""
      say "  None are needed on macOS: Apple's voices, including Eloquence, run in process."
    else
      hosts="textweaver-eci-host textweaver-dectalk-host"
    fi
    for h in $hosts; do
      if [ -x "$dir/$h" ]; then say "  $h: present"; else say "  $h: missing"; fi
    done
    local dict="$dir/ibmtts-dictionaries"
    if [ -d "$dict" ]; then
      say "Pronunciation dictionaries: present, $(find "$dict" -type f | wc -l | tr -d ' ') files."
    elif [ "$OS" = Darwin ]; then
      say "Pronunciation dictionaries: not used on macOS."
    else
      say "Pronunciation dictionaries: missing (ibmtts-dictionaries beside the programs)."
    fi
  else
    say "tw was not found on your PATH or in this checkout."
  fi

  say ""
  say "== Speech and audio =="
  if [ "$OS" = Darwin ]; then
    say "say: $(have say && echo present || echo missing)"
  else
    say "espeak-ng: $(first_line espeak-ng --version)"
    say "speech-dispatcher: $(first_line speech-dispatcher --version)"
    say "spd-say: $(have spd-say && echo present || echo missing)"
    if have pactl && [ "$DRY_RUN" = 0 ]; then
      local server
      server="$(pactl info 2> /dev/null | sed -n 's/^Server Name: //p' | head -n 1)"
      say "Sound server: ${server:-none reachable (pactl could not connect)}"
    else
      say "Sound server: pactl not available"
    fi
    say "ALSA cards: $([ -r /proc/asound/cards ] && grep -c '^ *[0-9]' /proc/asound/cards || echo none visible)"
    say "XDG_RUNTIME_DIR: $([ -n "${XDG_RUNTIME_DIR:-}" ] && echo set || echo not set)"
    say "PULSE_SERVER: $([ -n "${PULSE_SERVER:-}" ] && echo set || echo not set)"
  fi

  say ""
  say "== Optional tools =="
  say "ffmpeg: $(first_line ffmpeg -version)"
  say "pandoc: $(first_line pandoc --version)"
  say "whisper-cli: $(have whisper-cli && echo present || echo not installed)"
  say "git: $(first_line git --version)"

  say ""
  say "== textweaver environment variables that are set (names only) =="
  local names
  names="$(env | sed -n 's/^\(TEXTWEAVER_[A-Za-z0-9_]*\)=.*/\1/p' | sort)"
  if [ -n "$names" ]; then
    printf '%s\n' "$names" | sed 's/^/  /'
  else
    say "  none"
  fi

  say ""
  say "End of report."
}

if [ -n "$OUT" ]; then
  if [ "$DRY_RUN" = 1 ]; then
    report
    say "Would write this report to $OUT"
  else
    report | tee "$OUT"
    say ""
    say "The report is also in $OUT."
  fi
else
  report
fi
