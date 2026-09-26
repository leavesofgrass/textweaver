#!/usr/bin/env bash
# update.sh: update an installed textweaver on Linux or macOS. A source
# install is pulled with git and rebuilt; a macOS release install runs the
# installer again for the newest release.
#
# Shell: bash (3.2 or later). Safe to run twice. See scripts/README.md, or
# run with --help.

set -eu
set -o pipefail

DRY_RUN=0
ASSUME_YES=0
PREFIX="$HOME/.local"
DEPS=0

usage() {
  cat <<'EOF'
Usage: scripts/update.sh [--prefix DIR] [--deps] [--dry-run] [--yes]

Updates textweaver where the install scripts put it. It reads the install
manifest (PREFIX/share/textweaver/install-manifest.txt) to see how
textweaver was installed:

- From source (Linux, or macOS with --from-source): pulls the newest code
  with git pull --ff-only, then runs the installer again, which rebuilds
  and reinstalls. System packages are skipped unless you add --deps.
- From a release (macOS): runs install-macos.sh again, which downloads,
  checks, and installs the newest release.

Your settings and reading positions are not touched.

Options:
  --prefix DIR   The prefix textweaver was installed under (default: ~/.local).
  --deps         Also install or update the system packages (source installs).
  --dry-run      Print each command instead of running it.
  --yes          Answer yes to every question.
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

run() {
  if [ "$DRY_RUN" = 1 ]; then
    say "Would run: $(show_cmd "$@")"
    return 0
  fi
  say "Running: $(show_cmd "$@")"
  "$@"
}

ask() {
  if [ "$ASSUME_YES" = 1 ]; then
    say "$1 Answering yes, because of --yes."
    return 0
  fi
  if [ ! -t 0 ]; then
    say "$1 Answering no, because there is no terminal to ask on. Use --yes to answer yes."
    return 1
  fi
  local reply=""
  printf '%s Type y for yes or n for no, then press Enter: ' "$1"
  read -r reply || reply=""
  case $reply in
    [Yy] | [Yy][Ee][Ss]) return 0 ;;
    *) return 1 ;;
  esac
}

while [ "$#" -gt 0 ]; do
  case $1 in
    --prefix)
      [ "$#" -ge 2 ] || die "--prefix needs a folder."
      PREFIX="$2"
      shift
      ;;
    --prefix=*) PREFIX="${1#*=}" ;;
    --deps) DEPS=1 ;;
    --dry-run) DRY_RUN=1 ;;
    --yes | -y) ASSUME_YES=1 ;;
    -h | --help)
      usage
      exit 0
      ;;
    *) die "Unknown option $1. Run with --help to see the options." ;;
  esac
  shift
done
PREFIX="${PREFIX%/}"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
MANIFEST="$PREFIX/share/textweaver/install-manifest.txt"
OS="$(uname -s)"

is_checkout() {
  [ -f "$1/Cargo.toml" ] && [ -d "$1/crates/textweaver-cli" ]
}

manifest_value() {
  sed -n "s/^$1=//p" "$MANIFEST" | head -n 1
}

# Common flags for the installers. In a dry run the installer runs too,
# with --dry-run, so it prints its own steps.
pass=(--prefix "$PREFIX")
[ "$DRY_RUN" = 1 ] && pass+=(--dry-run)
[ "$ASSUME_YES" = 1 ] && pass+=(--yes)

if [ "$DRY_RUN" = 1 ]; then
  say "Dry run: nothing is changed. Each command is printed instead of run."
fi

KIND=""
SRC=""
if [ -f "$MANIFEST" ]; then
  KIND="$(manifest_value kind)"
  SRC="$(manifest_value source)"
  say "Found a $KIND install of textweaver $(manifest_value version) under $PREFIX."
elif is_checkout "$SCRIPT_DIR/.."; then
  SRC="$(cd "$SCRIPT_DIR/.." && pwd)"
  KIND="source"
  say "No install was found under $PREFIX, so this updates the checkout in $SRC and installs from it."
  ask "Continue?" || exit 0
else
  die "No textweaver install was found under $PREFIX. Use --prefix, or install with scripts/install-linux.sh or scripts/install-macos.sh."
fi

# Finds an installer: beside this script, then in the source checkout.
find_installer() {
  local name="$1" d
  for d in "$SCRIPT_DIR" "$PREFIX/share/textweaver/scripts" "${SRC:-/nonexistent}/scripts"; do
    if [ -f "$d/$name" ]; then
      printf '%s' "$d/$name"
      return 0
    fi
  done
  return 1
}

case $KIND in
  source)
    is_checkout "$SRC" || die "The source folder $SRC is missing or is not a textweaver checkout. Run the installer again."
    if [ -d "$SRC/.git" ] || [ -f "$SRC/.git" ]; then
      say "Pulling the newest code into $SRC."
      # Piped, so git prints plain lines instead of a progress meter.
      if ! run git -C "$SRC" pull --ff-only 2>&1 | cat; then
        die "git pull could not fast-forward $SRC. It may have local changes; look with: git -C $SRC status"
      fi
    else
      say "$SRC is not a git checkout, so it is rebuilt as it is."
    fi
    if [ "$OS" = Darwin ]; then
      installer="$SRC/scripts/install-macos.sh"
      extra=(--from-source --source "$SRC")
    else
      installer="$SRC/scripts/install-linux.sh"
      extra=(--source "$SRC")
      [ "$DEPS" = 1 ] || extra+=(--no-deps)
    fi
    [ -f "$installer" ] || die "$installer is missing."
    say "Rebuilding and reinstalling with $installer."
    say "Running: $(show_cmd bash "$installer" "${pass[@]}" "${extra[@]}")"
    bash "$installer" "${pass[@]}" "${extra[@]}"
    ;;
  release)
    if [ "$OS" = Darwin ]; then
      installer="$(find_installer install-macos.sh)" \
        || die "install-macos.sh was not found. Download it from https://github.com/leavesofgrass/textweaver/tree/main/scripts and run it."
      extra=()
    else
      installer="$(find_installer install-linux.sh)" \
        || die "install-linux.sh was not found. Download it from https://github.com/leavesofgrass/textweaver/tree/main/scripts and run it."
      # The same kind of package as before: the AppImage or the tarball.
      extra=(--release latest)
      case "$(manifest_value package)" in
        tarball) extra+=(--tarball) ;;
        appimage) extra+=(--appimage) ;;
      esac
    fi
    say "Installing the newest release with $installer."
    say "Running: $(show_cmd bash "$installer" "${pass[@]}" ${extra[@]+"${extra[@]}"})"
    bash "$installer" "${pass[@]}" ${extra[@]+"${extra[@]}"}
    ;;
  *)
    die "The manifest $MANIFEST names an unknown kind of install: $KIND."
    ;;
esac
