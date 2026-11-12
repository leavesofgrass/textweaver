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
FAST=0
BASE=main
ONLY=""

usage() {
  cat <<'EOF'
Usage: scripts/dev-check.sh [--fast [--base REF]] [--docker] [--only STEP,...] [--fail-fast] [--dry-run]

Runs every check CI runs, so you can see CI's answer before you push. Every
cargo call has --locked, and RUSTFLAGS is -D warnings for every step, as CI
sets it:

  fmt        cargo fmt --all --check
  clippy     cargo clippy --workspace --all-targets
             FEATURES -- -D warnings
  lean       cargo build -p textweaver-tui --no-default-features (the lean
             terminal reader), in its own target subfolder, lean
  test       cargo test --workspace --no-fail-fast FEATURES, between the
             sentinel's record and check (cargo xtask sentinel;
             TEXTWEAVER_HOME is an empty folder in the target folder during
             the tests)
  sentinel   the sentinel's check: no test wrote to the real data folders
  doc        cargo doc --workspace --no-deps
             FEATURES, with RUSTDOCFLAGS="-D warnings"
  pseudo     cargo test -p textweaver-app --test it -- pseudo_locale:: (every
             message comes from the catalog: bracketed in en-XA, direction
             marks closed in ar-XB)
  generated  cargo xtask regen --check (every generated file is current:
             the notices, the settings reference, docs/keyboard.md, the
             docs/site data, and the docs indexes and crate counts; one
             line each; cargo xtask regen rebuilds them). --only keyboard
             or --only site selects this step.
  deps       cargo xtask deps --check (the dependency direction between the
             workspace crates)
  parity     cargo xtask parity (word and sentence parity with star)
  deny       cargo deny check (licenses, advisories, duplicates, sources),
             skipped when cargo-deny is not installed
  links      python3 tools/check_links.py (relative links and anchors in the
             docs resolve)
  site-a11y  python3 tools/check_site_a11y.py (static accessibility checks
             of the docs/site pages)
  scripts    shellcheck on scripts/*.sh, when shellcheck is installed

FEATURES is --all-features on Linux when the espeak-ng development files are
installed, as in CI and the Docker image. Without them it is the features
that need no system library (omnivox and speechd). On macOS it is empty, as
in CI. On Windows (Git Bash) it is the omnivox feature, which matches CI's
Windows leg only: a green run there does not cover CI's Linux --all-features
leg (espeak, speechd). Use --docker for that.

--fast is the quicker check for one branch (docs/dev/testing.md, "Check
tiers"): cargo xtask affected --base main names the crates changed since the
merge base and every crate that depends on them, and fmt, clippy, the tests
(cargo nextest when installed, else cargo test) and the doc tests run on
those only. The lean step runs when textweaver-app, textweaver-tui or
textweaver-components is in the set, and the generated and deps steps when
the change touches the docs, the locale catalogs, the settings schema, the
keymap or a Cargo.toml. It is not the full check: run that once per batch.

Every step runs, and a summary at the end says which passed. The exit
status is 1 when any failed.

Options:
  --fast         Only the crates changed since the merge base of --base, and
                 the crates that depend on them. Not the full check.
  --base REF     The branch --fast compares with (default main).
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
    --fast) FAST=1 ;;
    --base)
      [ "$#" -ge 2 ] || die "--base needs a branch."
      BASE="$2"
      shift
      ;;
    --base=*) BASE="${1#*=}" ;;
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
# As in CI, which sets it for every step: a warning that only rustc reports,
# in a test or doc build, fails here too.
export RUSTFLAGS="-D warnings"
if [ -n "${NO_COLOR:-}" ]; then
  export CARGO_TERM_COLOR=never RUSTUP_TERM_COLOR=never
fi

if [ "$DOCKER" = 1 ]; then
  # Git Bash on Windows would rewrite /work paths without this.
  export MSYS_NO_PATHCONV=1
  args=()
  [ -n "$ONLY" ] && args+=(--only "$ONLY")
  [ "$FAIL_FAST" = 1 ] && args+=(--fail-fast)
  [ "$FAST" = 1 ] && args+=(--fast --base "$BASE")
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
  # The keyboard and site checks are part of the generated step now.
  if [ "$1" = generated ]; then
    case ",$ONLY," in
      *",keyboard,"* | *",site,"*) return 0 ;;
    esac
  fi
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

# The target folder: the lean build and the sentinel's files go inside it.
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
LEAN_TARGET="$TARGET_DIR/lean"
SENTINEL_FILE="$TARGET_DIR/dev-check-sentinel.txt"
SENTINEL_HOME="$TARGET_DIR/dev-check-sentinel-home"

skip() {
  wanted "$1" || return 0
  say ""
  say "== $1: skipped =="
  say "$2"
  SKIPPED="$SKIPPED $1"
}

# tests_with_sentinel DESCRIPTION COMMAND...: the tests between the
# sentinel's record and check.
tests_with_sentinel() {
  local what="$1"
  shift
  wanted test || return 0
  local recorded=1
  say ""
  if [ "$DRY_RUN" = 1 ]; then
    say "Would run: TEXTWEAVER_HOME=$(show_cmd "$SENTINEL_HOME") cargo --locked xtask sentinel record $(show_cmd "$SENTINEL_FILE")"
  else
    say "== sentinel: listing the real data folders before the tests =="
    mkdir -p "$SENTINEL_HOME"
    if ! TEXTWEAVER_HOME="$SENTINEL_HOME" cargo --locked xtask sentinel record "$SENTINEL_FILE"; then
      say "The sentinel could not list the folders, so its check is left out."
      recorded=0
    fi
  fi
  step test "$what" env TEXTWEAVER_ESPEAK_OUTPUT=virtual TEXTWEAVER_HOME="$SENTINEL_HOME" "$@"
  if [ "$recorded" = 1 ]; then
    step sentinel "no test touched the real data folders" cargo --locked xtask sentinel check "$SENTINEL_FILE"
  else
    FAILED="$FAILED sentinel"
  fi
}

LEAN=(env CARGO_TARGET_DIR="$LEAN_TARGET" cargo build --locked -p textweaver-tui --no-default-features)

if [ "$FAST" = 1 ]; then
  say ""
  say "== affected: the crates changed since the merge base of $BASE, and their dependents =="
  if ! SET="$(cargo --locked xtask affected --base "$BASE")"; then
    say "affected FAILED: cargo xtask affected could not list the crates."
    FAILED="$FAILED affected"
    summary
    exit 1
  fi
  GENERATED="$(cargo --locked xtask affected --base "$BASE" --generated)"
  # The same features as the full check, where Cargo takes them for the set.
  FEATURE_OPT=()
  case ${FEATURES[0]:-} in
    --features) FEATURE_OPT=(--feature "${FEATURES[1]}") ;;
  esac
  PKG_LINE="$(cargo --locked xtask affected --base "$BASE" --args ${FEATURE_OPT[@]+"${FEATURE_OPT[@]}"})"
  if [ "${FEATURES[0]:-}" = --all-features ] && [ -n "$PKG_LINE" ]; then
    PKG_LINE="$PKG_LINE --all-features"
  fi
  read -r -a PKG_ARGS <<< "$PKG_LINE"
  COUNT=0
  NAMES=""
  FMT_ARGS=()
  WORDS=" "
  for c in $SET; do
    COUNT=$((COUNT + 1))
    WORDS="$WORDS$c "
    NAMES="${NAMES:+$NAMES, }$c"
    FMT_ARGS+=(-p "$c")
  done
  say "$COUNT crates: ${NAMES:-none}."
  if [ "$COUNT" -eq 0 ]; then
    for n in fmt clippy test doctest; do
      skip "$n" "No workspace crate changed since the merge base of $BASE."
    done
  else
    step fmt "formatting of the changed crates" cargo fmt --check "${FMT_ARGS[@]}"
    step clippy "lints of the changed crates, warnings are errors" cargo clippy --locked "${PKG_ARGS[@]}" --all-targets -- -D warnings
    if [ "$DRY_RUN" = 1 ] || cargo nextest --version > /dev/null 2>&1; then
      tests_with_sentinel "tests of the changed crates (cargo nextest)" cargo nextest run --locked --no-fail-fast "${PKG_ARGS[@]}"
    else
      say "cargo-nextest is not installed, so the tests run with cargo test."
      tests_with_sentinel "tests of the changed crates (cargo test)" cargo test --locked --no-fail-fast "${PKG_ARGS[@]}"
    fi
    # nextest does not run doc tests; cargo test --doc does.
    step doctest "doc tests of the changed crates" env TEXTWEAVER_ESPEAK_OUTPUT=virtual cargo test --locked --doc --no-fail-fast "${PKG_ARGS[@]}"
  fi
  case $WORDS in
    *" textweaver-app "* | *" textweaver-tui "* | *" textweaver-components "*)
      step lean "the lean terminal reader builds, warnings are errors" "${LEAN[@]}"
      ;;
    *) skip lean "Not needed: textweaver-app, textweaver-tui and textweaver-components did not change." ;;
  esac
  if [ "$GENERATED" = yes ]; then
    step generated "every generated file is current" cargo --locked xtask regen --check
    step deps "the dependency direction between the workspace crates" cargo --locked xtask deps --check
  else
    skip generated "Not needed: no docs, locale catalog, settings schema, keymap or Cargo.toml changed."
    skip deps "Not needed: no Cargo.toml changed."
  fi
  say ""
  say "The fast check leaves out doc, pseudo, parity, deny, links, site-a11y and scripts. Run the full check once per batch."
  if [ "$DRY_RUN" = 1 ]; then
    say ""
    say "Dry run finished. Only cargo xtask affected was run."
    exit 0
  fi
  summary
  [ -z "$FAILED" ]
  exit
fi

step fmt "formatting" cargo fmt --all --check
step clippy "lints, warnings are errors" cargo clippy --locked --workspace --all-targets ${FEATURES[@]+"${FEATURES[@]}"} -- -D warnings
# The lean reader in its own target folder, so the main one is not rebuilt.
step lean "the lean terminal reader builds, warnings are errors" "${LEAN[@]}"
# Tests never play audio: eSpeak NG writes to a virtual output, as in CI.
tests_with_sentinel "tests" cargo test --locked --workspace --no-fail-fast ${FEATURES[@]+"${FEATURES[@]}"}
step doc "API documentation, warnings are errors" env RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps ${FEATURES[@]+"${FEATURES[@]}"}
step pseudo "the interface in the pseudo-locales en-XA and ar-XB" env TEXTWEAVER_ESPEAK_OUTPUT=virtual cargo test --locked -p textweaver-app --test it -- pseudo_locale::
step generated "every generated file is current" cargo --locked xtask regen --check
step deps "the dependency direction between the workspace crates" cargo --locked xtask deps --check
step parity "word and sentence parity with star" cargo --locked xtask parity
if wanted deny; then
  if [ "$DRY_RUN" = 1 ] || cargo deny --version > /dev/null 2>&1; then
    step deny "licenses, advisories, duplicates and sources (deny.toml)" cargo deny --locked check
  else
    skip deny "cargo-deny is not installed, so the deny check cannot run. CI runs it; tools.toml names the version."
  fi
fi
PYTHON=""
if have python3; then
  PYTHON=python3
elif have python; then
  PYTHON=python
fi
for py_step in links site-a11y; do
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
    site-a11y) step site-a11y "static accessibility checks of docs/site" "$PYTHON" tools/check_site_a11y.py ;;
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
