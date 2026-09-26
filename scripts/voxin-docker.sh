#!/usr/bin/env bash
# voxin-docker.sh: run textweaver's Eloquence tests, or tw speak, in the
# Docker development container with your Voxin installation mounted
# (compose.voxin.yaml). Wraps the command lines in docs/docker.md.
#
# Shell: bash (3.2 or later); works from Git Bash on Windows. See
# scripts/README.md, or run with --help.

set -eu
set -o pipefail

DRY_RUN=0
ACTION=""
OUTFILE=""
ARGS=()

usage() {
  cat <<'EOF'
Usage: scripts/voxin-docker.sh ACTION [options] [-- ARGUMENTS]

Runs textweaver with ETI-Eloquence for Linux (Voxin) inside the development
container. Voxin is licensed per user, so it is never copied into the
repository or the image: compose.voxin.yaml mounts your existing Voxin
volume read-only (emacspeak-docker_voxin, or the volume named in
VOXIN_VOLUME).

Actions:
  check        Check that the Voxin engine runs: voxin-say writes a short
               WAV file inside the container (nothing is played).
  test         Run the Eloquence backend's real-engine tests:
               TEXTWEAVER_ECI=1 cargo test -p textweaver-eci -- --ignored
  speak TEXT   Speak TEXT with tw speak --backend eci into a WAV file
               (default: target-audio/voxin.wav in the checkout). Nothing
               is played; listen to the file on the host.
  spike        Run tools/eci-spike/voxin_spike.py (index marks).
  shell        Open an interactive shell in the container with Voxin.

Options:
  --out FILE   For speak: the WAV file, relative to the checkout
               (default: target-audio/voxin.wav).
  --dry-run    Print the docker command instead of running it.
  --yes        Accepted for consistency; this script asks nothing.
  -h, --help   Show this help.

Arguments after -- go to cargo test (for test), for example:
  scripts/voxin-docker.sh test -- dictionaries

The build output goes to the textweaver-target volume under /target/voxin,
so it never mixes with other builds.
EOF
}

say() { printf '%s\n' "$*"; }
die() {
  printf 'Error: %s\n' "$*" >&2
  exit 1
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

TEXT=""
while [ "$#" -gt 0 ]; do
  case $1 in
    --out)
      [ "$#" -ge 2 ] || die "--out needs a file name."
      OUTFILE="$2"
      shift
      ;;
    --out=*) OUTFILE="${1#*=}" ;;
    --dry-run) DRY_RUN=1 ;;
    --yes | -y) ;;
    -h | --help)
      usage
      exit 0
      ;;
    --)
      shift
      ARGS=("$@")
      break
      ;;
    -*) die "Unknown option $1. Run with --help to see the options." ;;
    *)
      if [ -z "$ACTION" ]; then
        ACTION="$1"
      elif [ "$ACTION" = speak ]; then
        TEXT="${TEXT:+$TEXT }$1"
      else
        die "Unexpected argument $1. Run with --help to see the usage."
      fi
      ;;
  esac
  shift
done

[ -n "$ACTION" ] || {
  usage
  exit 2
}

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT" || die "Cannot enter $ROOT."
[ -f compose.voxin.yaml ] || die "compose.voxin.yaml is missing from $ROOT."

# Git Bash on Windows would rewrite /target and /work paths without this.
export MSYS_NO_PATHCONV=1

VOLUME="${VOXIN_VOLUME:-emacspeak-docker_voxin}"
if [ "$DRY_RUN" = 0 ]; then
  command -v docker > /dev/null 2>&1 || die "docker is not installed or not on PATH."
  if ! docker volume inspect "$VOLUME" > /dev/null 2>&1; then
    die "The Docker volume $VOLUME does not exist. Install Voxin with emacspeak-docker's install-outloud, or set VOXIN_VOLUME to the volume that holds /opt/oralux. See docs/docker.md."
  fi
fi

# The image sets CARGO_TERM_COLOR=always; plain output reads better, and
# NO_COLOR turns colour off entirely.
color=auto
[ -n "${NO_COLOR:-}" ] && color=never
compose=(docker compose -p textweaver -f compose.yaml -f compose.voxin.yaml run --rm
  -e "CARGO_TERM_COLOR=$color" -e CARGO_TERM_PROGRESS_WHEN=never)
env_target=(-e CARGO_TARGET_DIR=/target/voxin)

case $ACTION in
  check)
    say "Checking that Voxin runs: voxin-say writes /tmp/voxin-check.wav inside the container. Nothing is played."
    cmd=("${compose[@]}" -T dev bash -c 'voxin-say -w /tmp/voxin-check.wav "textweaver Voxin check" && ls -l /tmp/voxin-check.wav')
    ;;
  test)
    say "Running the Eloquence backend's real-engine tests with Voxin."
    cmd=("${compose[@]}" -T "${env_target[@]}" -e TEXTWEAVER_ECI=1 dev cargo test -p textweaver-eci -- --ignored ${ARGS[@]+"${ARGS[@]}"})
    ;;
  speak)
    [ -n "$TEXT" ] || TEXT="Hello from textweaver and Eloquence."
    [ -n "$OUTFILE" ] || OUTFILE="target-audio/voxin.wav"
    case $OUTFILE in
      /*) die "--out is relative to the checkout, such as target-audio/voxin.wav." ;;
    esac
    say "Speaking with Eloquence into $OUTFILE in the checkout. Nothing is played."
    # The inner script expands $1 and $2 in the container, not here.
    # shellcheck disable=SC2016
    cmd=("${compose[@]}" -T "${env_target[@]}" dev bash -c 'mkdir -p "$(dirname "/work/$1")" && cargo run -q -p textweaver-cli --features omnivox -- speak --backend eci --out "/work/$1" "$2"' _ "$OUTFILE" "$TEXT")
    ;;
  spike)
    say "Checking ECI index marks with tools/eci-spike/voxin_spike.py."
    cmd=("${compose[@]}" -T dev python3 tools/eci-spike/voxin_spike.py)
    ;;
  shell)
    say "Opening a shell in the container with Voxin. Type exit to leave."
    cmd=("${compose[@]}" "${env_target[@]}" dev)
    ;;
  *) die "Unknown action $ACTION. Use check, test, speak, spike, or shell." ;;
esac

if [ "$DRY_RUN" = 1 ]; then
  say "Would run: $(show_cmd "${cmd[@]}")"
  exit 0
fi
say "Running: $(show_cmd "${cmd[@]}")"
"${cmd[@]}"
