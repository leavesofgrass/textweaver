#!/usr/bin/env bash
# dev-check.sh: run the checks CI runs, locally, on Linux or macOS (or in
# the Docker development container with --docker).
#
# Shell: bash (3.2 or later). See scripts/README.md, or run with --help.

set -u
set -o pipefail

DRY_RUN=0
DOCKER=0
FAIL_FAST=0
ONLY=""

usage() {
  cat <<'EOF'
Usage: scripts/dev-check.sh [--docker] [--only STEP,...] [--fail-fast] [--dry-run]

Runs every check CI runs, so you can see CI's answer before you push:

  fmt        cargo fmt --all --check
  clippy     cargo clippy --workspace --exclude textweaver-gui --all-targets
             FEATURES -- -D warnings
  test       cargo test --workspace --exclude textweaver-gui FEATURES
  doc        cargo doc --workspace --exclude textweaver-gui --no-deps
             FEATURES, with RUSTDOCFLAGS="-D warnings"
  keyboard   cargo xtask keyboard --check (docs/keyboard.md is current)
  links      python3 tools/check_links.py (relative links and anchors in the
             docs resolve)
  site       python3 tools/gen_site_data.py --check (the data in the
             docs/site pages is current)
  scripts    shellcheck on scripts/*.sh, when shellcheck is installed

FEATURES is --all-features on Linux when the espeak-ng development files are
installed, as in CI and the Docker image. Without them it is the features
that need no system library (omnivox and speechd). On macOS it is empty, as
in CI.

Every step runs, and a summary at the end says which passed. The exit
status is 1 when any failed.

Options:
  --docker       Run the checks inside the development container
                 (docker compose run dev), which has every library.
  --only LIST    Run only these steps, separated by commas, such as
                 --only fmt,clippy.
  --fail-fast    Stop at the first failing step.
  --dry-run      Print each command instead of running it.
  --yes          Accepted for consistency; this script asks nothing.
  -h, --help     Show this help.
EOF
}

say() { printf '%s\n' "$*"; }
die() {
  printf 'Error: %s\n' "$*" >&2
  exit 1
}
have() { command -v "$1" > /dev/null 2>&1; }

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

while [ "$#" -gt 0 ]; do
  case $1 in
    --docker) DOCKER=1 ;;
    --only)
      [ "$#" -ge 2 ] || die "--only needs a list of steps."
      ONLY="$2"
      shift
      ;;
    --only=*) ONLY="${1#*=}" ;;
    --fail-fast) FAIL_FAST=1 ;;
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

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT" || die "Cannot enter $ROOT."
[ -f Cargo.toml ] || die "$ROOT is not the textweaver checkout."

export CARGO_TERM_PROGRESS_WHEN=never RUSTUP_TERM_PROGRESS_WHEN=never
if [ -n "${NO_COLOR:-}" ]; then
  export CARGO_TERM_COLOR=never RUSTUP_TERM_COLOR=never
fi

if [ "$DOCKER" = 1 ]; then
  # Git Bash on Windows would rewrite /work paths without this.
  export MSYS_NO_PATHCONV=1
  args=()
  [ -n "$ONLY" ] && args+=(--only "$ONLY")
  [ "$FAIL_FAST" = 1 ] && args+=(--fail-fast)
  # The image sets CARGO_TERM_COLOR=always; ask for plain output instead.
  color=auto
  [ -n "${NO_COLOR:-}" ] && color=never
  cmd=(docker compose -p textweaver run --rm -T -e CARGO_TARGET_DIR=/target/dev-check -e NO_COLOR -e "CARGO_TERM_COLOR=$color" dev bash scripts/dev-check.sh)
  say "Running the checks in the development container (docker compose run dev)."
  if [ "$DRY_RUN" = 1 ]; then
    say "Would run: $(show_cmd "${cmd[@]}" ${args[@]+"${args[@]}"})"
    exit 0
  fi
  say "Running: $(show_cmd "${cmd[@]}" ${args[@]+"${args[@]}"})"
  exec "${cmd[@]}" ${args[@]+"${args[@]}"}
fi

OS="$(uname -s)"
FEATURES=()
case $OS in
  MINGW* | MSYS* | CYGWIN*)
    say "This is Windows. scripts/dev-check.ps1 also builds the 32-bit engine hosts; this runs the shared checks with the omnivox feature."
    FEATURES=(--features textweaver-speech/omnivox)
    ;;
esac
if [ "$OS" = Linux ]; then
  if have pkg-config && pkg-config --exists espeak-ng; then
    FEATURES=(--all-features)
  else
    say "The espeak-ng development files are missing, so the espeak feature is left out. Use --docker for the full CI set."
    FEATURES=(--features "textweaver-speech/omnivox,textweaver-speech/speechd")
  fi
fi

wanted() {
  [ -z "$ONLY" ] && return 0
  case ",$ONLY," in
    *",$1,"*) return 0 ;;
  esac
  return 1
}

PASSED=""
FAILED=""
SKIPPED=""

# step NAME DESCRIPTION COMMAND...
step() {
  local name="$1" what="$2"
  shift 2
  wanted "$name" || return 0
  say ""
  say "== $name: $what =="
  if [ "$DRY_RUN" = 1 ]; then
    say "Would run: $(show_cmd "$@")"
    return 0
  fi
  say "Running: $(show_cmd "$@")"
  local start end
  start=$(date +%s)
  if "$@"; then
    end=$(date +%s)
    say "$name passed, in $((end - start)) seconds."
    PASSED="$PASSED $name"
  else
    end=$(date +%s)
    say "$name FAILED, after $((end - start)) seconds."
    FAILED="$FAILED $name"
    if [ "$FAIL_FAST" = 1 ]; then
      summary
      exit 1
    fi
  fi
}

summary() {
  say ""
  say "== Summary =="
  say "Passed:${PASSED:- none}."
  say "Failed:${FAILED:- none}."
  if [ -n "$SKIPPED" ]; then
    say "Skipped:$SKIPPED."
  fi
}

say "Running the CI checks in $ROOT."
if [ "${#FEATURES[@]}" -gt 0 ]; then
  say "Features: ${FEATURES[*]}."
else
  say "Features: none (as CI does on this system)."
fi

step fmt "formatting" cargo fmt --all --check
step clippy "lints, warnings are errors" cargo clippy --workspace --exclude textweaver-gui --all-targets ${FEATURES[@]+"${FEATURES[@]}"} -- -D warnings
step test "tests" cargo test --workspace --exclude textweaver-gui ${FEATURES[@]+"${FEATURES[@]}"}
step doc "API documentation, warnings are errors" env RUSTDOCFLAGS="-D warnings" cargo doc --workspace --exclude textweaver-gui --no-deps ${FEATURES[@]+"${FEATURES[@]}"}
step keyboard "docs/keyboard.md is current" cargo xtask keyboard --check
PYTHON=""
if have python3; then
  PYTHON=python3
elif have python; then
  PYTHON=python
fi
for py_step in links site; do
  wanted "$py_step" || continue
  if [ -z "$PYTHON" ]; then
    say ""
    say "== $py_step: skipped =="
    say "Python 3 is not installed, so the $py_step check cannot run."
    SKIPPED="$SKIPPED $py_step"
    continue
  fi
  case $py_step in
    links) step links "links and anchors in the docs resolve" "$PYTHON" tools/check_links.py ;;
    site) step site "the docs/site data is current" "$PYTHON" tools/gen_site_data.py --check ;;
  esac
done
if wanted scripts; then
  if have shellcheck; then
    step scripts "shellcheck on the scripts" shellcheck scripts/*.sh
  else
    say ""
    say "== scripts: skipped =="
    say "shellcheck is not installed. Install it, or run it in Docker: docker run --rm -v \"\$PWD:/w\" -w /w koalaman/shellcheck:stable scripts/*.sh"
    SKIPPED="$SKIPPED scripts"
  fi
fi

if [ "$DRY_RUN" = 1 ]; then
  say ""
  say "Dry run finished. Nothing was run."
  exit 0
fi
summary
[ -z "$FAILED" ]
