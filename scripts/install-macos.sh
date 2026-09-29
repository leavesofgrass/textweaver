#!/usr/bin/env bash
# install-macos.sh: install textweaver on a Mac, from the latest GitHub
# release (the default) or from source.
#
# Shell: bash, and it runs on the bash 3.2 that macOS ships. Safe to run
# twice. See scripts/README.md, or run with --help.

set -eu
set -o pipefail

REPO="${TEXTWEAVER_REPO:-leavesofgrass/textweaver}"
MARKER="# added by the textweaver installer"

DRY_RUN=0
ASSUME_YES=0
PREFIX="$HOME/.local"
MODE="release"
TAG=""
SOURCE_DIR=""
BREW="ask"
UNINSTALL=0
# The GUI (textweaver.app), from the release's -gui package: "ask" keeps
# what an earlier install chose (the manifest's gui= line), else leaves it
# out.
GUI="ask"
# Where textweaver.app goes: the user's own Applications folder.
APPSDIR="${TEXTWEAVER_APPS_DIR:-$HOME/Applications}"

usage() {
  cat <<'EOF'
Usage: scripts/install-macos.sh [options]

Installs textweaver on a Mac. By default it downloads the newest release
from GitHub (the universal package, for Apple silicon and Intel), checks it
against SHA256SUMS.txt, removes the quarantine flag, and installs textweaver
and tw into ~/.local/bin. With --from-source it builds textweaver instead.

Every step is announced before it runs. The script asks before it changes
your PATH or installs anything outside the prefix. Running it again updates
the install.

Options:
  --prefix DIR      Install under DIR (default: ~/.local). Programs go in
                    DIR/bin and the guides in DIR/share/doc/textweaver.
  --release TAG     Install this release, such as v0.1.0-alpha.3, instead of
                    the newest one.
  --from-source     Build from source: needs the Xcode command line tools,
                    and installs Rust with rustup if cargo is missing (it
                    asks first).
  --source DIR      With --from-source, the textweaver checkout to build.
                    Without it, the checkout this script sits in is used, or
                    the repository is cloned to ~/.local/src/textweaver.
  --gui             Also install the GUI, textweaver.app, from the release's
                    GUI package into ~/Applications. Running the script
                    again keeps the GUI installed. Releases only.
  --no-gui          Do not install the GUI, and remove it if an earlier
                    run installed it.
  --brew            Install the optional Homebrew packages (ffmpeg, pandoc)
                    without asking, when Homebrew is installed.
  --no-brew         Do not offer the Homebrew packages.
  --uninstall       Remove textweaver from the prefix. Your settings and
                    reading positions are kept.
  --dry-run         Print each command instead of running it.
  --yes             Answer yes to every question.
  -h, --help        Show this help.

Examples:
  scripts/install-macos.sh
  scripts/install-macos.sh --release v0.1.0-alpha.3
  scripts/install-macos.sh --gui
  scripts/install-macos.sh --from-source
  scripts/install-macos.sh --uninstall
EOF
}

# ---------------------------------------------------------------- helpers --

say() { printf '%s\n' "$*"; }
warn() { printf 'Warning: %s\n' "$*" >&2; }
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

run() {
  if [ "$DRY_RUN" = 1 ]; then
    say "Would run: $(show_cmd "$@")"
    return 0
  fi
  say "Running: $(show_cmd "$@")"
  "$@"
}

run_plain() {
  if [ "$DRY_RUN" = 1 ]; then
    say "Would run: $(show_cmd "$@")"
    return 0
  fi
  say "Running: $(show_cmd "$@")"
  "$@" 2>&1 | cat
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

have() { command -v "$1" > /dev/null 2>&1; }

tilde() {
  case $1 in
    "$HOME"/*) printf '%s/%s' '~' "${1#"$HOME"/}" ;;
    *) printf '%s' "$1" ;;
  esac
}

section() {
  say ""
  say "== $* =="
}

# ------------------------------------------------------------------ args --

while [ "$#" -gt 0 ]; do
  case $1 in
    --prefix)
      [ "$#" -ge 2 ] || die "--prefix needs a folder."
      PREFIX="$2"
      shift
      ;;
    --prefix=*) PREFIX="${1#*=}" ;;
    --release)
      [ "$#" -ge 2 ] || die "--release needs a tag, such as v0.1.0-alpha.3."
      MODE="release"
      TAG="$2"
      shift
      ;;
    --release=*)
      MODE="release"
      TAG="${1#*=}"
      ;;
    --from-source) MODE="source" ;;
    --source)
      [ "$#" -ge 2 ] || die "--source needs a folder."
      SOURCE_DIR="$2"
      shift
      ;;
    --source=*) SOURCE_DIR="${1#*=}" ;;
    --gui) GUI=1 ;;
    --no-gui) GUI=0 ;;
    --brew) BREW="yes" ;;
    --no-brew) BREW="no" ;;
    --uninstall) UNINSTALL=1 ;;
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

case $PREFIX in
  /*) ;;
  *) PREFIX="$(pwd)/$PREFIX" ;;
esac
PREFIX="${PREFIX%/}"
BINDIR="$PREFIX/bin"
DOCDIR="$PREFIX/share/doc/textweaver"
DATADIR="$PREFIX/share/textweaver"
MANIFEST="$DATADIR/install-manifest.txt"

export CARGO_TERM_PROGRESS_WHEN=never RUSTUP_TERM_PROGRESS_WHEN=never
if [ -n "${NO_COLOR:-}" ]; then
  export CARGO_TERM_COLOR=never RUSTUP_TERM_COLOR=never
fi

SCRIPT_DIR=""
case $0 in
  */*) SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" ;;
esac

if [ "$(uname -s)" != Darwin ] && [ "$DRY_RUN" = 0 ]; then
  die "This script is for macOS. On Linux, use scripts/install-linux.sh. A dry run (--dry-run) works anywhere."
fi

WORK=""
cleanup() {
  if [ -n "$WORK" ] && [ -d "$WORK" ]; then
    rm -rf "$WORK"
  fi
}
trap cleanup EXIT

# -------------------------------------------------------------- release --

STAGE=""
VERSION=""
download_release() {
  section "Download"
  if [ -z "$TAG" ]; then
    local api="https://api.github.com/repos/$REPO/releases?per_page=1"
    if [ "$DRY_RUN" = 1 ]; then
      say "Would look up the newest release at $api"
      TAG="vVERSION"
    else
      say "Looking up the newest release at $api"
      TAG="$(curl -fsSL "$api" | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n 1)" \
        || die "Could not reach GitHub. Check your connection, or name a release with --release."
      [ -n "$TAG" ] || die "No release was found on https://github.com/$REPO/releases."
      say "The newest release is $TAG."
    fi
  fi
  VERSION="${TAG#v}"
  local name="textweaver-$VERSION-macos-universal"
  local base="https://github.com/$REPO/releases/download/$TAG"
  if [ "$DRY_RUN" = 1 ]; then
    WORK="/tmp/textweaver-install"
  else
    WORK="$(mktemp -d -t textweaver-install)"
  fi
  say "Downloading $name.tar.gz and SHA256SUMS.txt from release $TAG."
  run curl -fsSL -o "$WORK/$name.tar.gz" "$base/$name.tar.gz"
  run curl -fsSL -o "$WORK/SHA256SUMS.txt" "$base/SHA256SUMS.txt"

  say "Checking the download against SHA256SUMS.txt."
  if [ "$DRY_RUN" = 1 ]; then
    say "Would run: shasum -a 256 $WORK/$name.tar.gz, and compare it with the line for $name.tar.gz in SHA256SUMS.txt"
  else
    local want got
    want="$(awk -v f="$name.tar.gz" '$2 == f || $2 == "*" f { print $1 }' "$WORK/SHA256SUMS.txt" | head -n 1)"
    [ -n "$want" ] || die "SHA256SUMS.txt has no line for $name.tar.gz, so the download cannot be checked. Nothing was installed."
    got="$(shasum -a 256 "$WORK/$name.tar.gz" | awk '{ print $1 }')"
    if [ "$want" != "$got" ]; then
      die "The checksum does not match (expected $want, got $got). The download may be damaged. Nothing was installed."
    fi
    say "The checksum matches: $got."
  fi

  say "Extracting the package."
  run tar -xzf "$WORK/$name.tar.gz" -C "$WORK"
  STAGE="$WORK/$name"
  say "Removing the quarantine flag, so Gatekeeper lets the programs run. The package is signed ad hoc, not notarized."
  if [ "$DRY_RUN" = 1 ]; then
    say "Would run: xattr -dr com.apple.quarantine $STAGE"
  else
    xattr -dr com.apple.quarantine "$STAGE" 2> /dev/null || true
  fi
}

# ------------------------------------------------------------------ GUI --

# Decides whether this run installs the GUI: --gui or --no-gui, else what
# the manifest of an earlier install says.
decide_gui() {
  if [ "$GUI" = ask ]; then
    GUI=0
    if [ -f "$MANIFEST" ] && grep -q '^gui=' "$MANIFEST"; then
      GUI=1
      say "The GUI was installed before, so it is updated too. Use --no-gui to remove it."
    fi
  fi
}

# True when $1 is a textweaver.app this script installed (its bundle
# identifier is textweaver's), so removing it cannot remove anything else.
is_our_app() {
  [ -f "$1/Contents/Info.plist" ] && grep -q 'org.textweaver.gui' "$1/Contents/Info.plist"
}

remove_gui() {
  local app="$APPSDIR/textweaver.app"
  if [ -e "$app" ]; then
    if is_our_app "$app"; then
      run rm -rf "$app"
    else
      say "Leaving $app: it is not the textweaver GUI this script installs."
    fi
  fi
}

GUI_INSTALLED=""
# Downloads, checks, and installs textweaver.app. SHA256SUMS.txt is
# already in $WORK. The universal package is preferred; releases before it
# had an Apple silicon one only.
install_gui() {
  section "GUI"
  local base="https://github.com/$REPO/releases/download/$TAG"
  local name="" candidate
  local arch
  arch="$(uname -m)"
  if [ "$DRY_RUN" = 1 ]; then
    name="textweaver-$VERSION-macos-universal-gui"
  else
    for candidate in universal aarch64; do
      if [ "$candidate" = aarch64 ] && [ "$arch" != arm64 ]; then
        continue
      fi
      if awk -v f="textweaver-$VERSION-macos-$candidate-gui.zip" '$2 == f || $2 == "*" f { found = 1 } END { exit !found }' "$WORK/SHA256SUMS.txt"; then
        name="textweaver-$VERSION-macos-$candidate-gui"
        break
      fi
    done
  fi
  if [ -z "$name" ]; then
    warn "Release $TAG has no GUI package for this Mac ($arch), so the GUI is not installed."
    return 0
  fi
  say "Downloading $name.zip, the GUI, from release $TAG."
  run curl -fsSL -o "$WORK/$name.zip" "$base/$name.zip"
  if [ "$DRY_RUN" = 1 ]; then
    say "Would run: shasum -a 256 $WORK/$name.zip, and compare it with the line for $name.zip in SHA256SUMS.txt"
  else
    local want got
    want="$(awk -v f="$name.zip" '$2 == f || $2 == "*" f { print $1 }' "$WORK/SHA256SUMS.txt" | head -n 1)"
    got="$(shasum -a 256 "$WORK/$name.zip" | awk '{ print $1 }')"
    if [ "$want" != "$got" ]; then
      die "The GUI's checksum does not match (expected $want, got $got). The download may be damaged. The reader is installed; the GUI is not."
    fi
    say "The GUI's checksum matches: $got."
  fi
  local app="$APPSDIR/textweaver.app"
  if [ -e "$app" ] && ! is_our_app "$app" && [ "$DRY_RUN" = 0 ]; then
    die "$app exists and is not the textweaver GUI. Move it away, then run this script again."
  fi
  say "Installing textweaver.app into $APPSDIR."
  run ditto -x -k "$WORK/$name.zip" "$WORK/gui"
  if [ "$DRY_RUN" = 1 ]; then
    say "Would run: xattr -dr com.apple.quarantine $WORK/gui/$name/textweaver.app"
  else
    xattr -dr com.apple.quarantine "$WORK/gui/$name/textweaver.app" 2> /dev/null || true
  fi
  run mkdir -p "$APPSDIR"
  if [ -e "$app" ] || [ "$DRY_RUN" = 1 ]; then
    run rm -rf "$app"
  fi
  run ditto "$WORK/gui/$name/textweaver.app" "$app"
  GUI_INSTALLED="app"
}

# --------------------------------------------------------------- source --

is_checkout() {
  [ -f "$1/Cargo.toml" ] && [ -d "$1/crates/textweaver-cli" ]
}

ensure_xcode_tools() {
  if [ "$DRY_RUN" = 1 ]; then
    say "Would check for the Xcode command line tools with: xcode-select -p"
    return 0
  fi
  if xcode-select -p > /dev/null 2>&1; then
    say "The Xcode command line tools are installed."
    return 0
  fi
  say "The Xcode command line tools are not installed. They provide the C compiler and linker that Rust needs."
  if ask "Start their installer now? macOS opens a window to confirm."; then
    run xcode-select --install || true
  fi
  die "Finish installing the Xcode command line tools, then run this script again."
}

version_at_least() {
  local want="$1" got="$2"
  [ "$(printf '%s\n%s\n' "$want" "$got" | sort -t. -k1,1n -k2,2n -k3,3n | head -n 1)" = "$want" ]
}

install_rustup() {
  say "The script can install rustup, the Rust installer, from https://sh.rustup.rs into ~/.cargo and ~/.rustup, for you only. It does not change your PATH."
  ask "Install rustup now?" || return 1
  if [ "$DRY_RUN" = 1 ]; then
    say "Would run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y -q --no-modify-path --profile minimal --default-toolchain none"
  else
    say "Running: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y -q --no-modify-path --profile minimal --default-toolchain none"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y -q --no-modify-path --profile minimal --default-toolchain none 2>&1 | cat
  fi
  PATH="$HOME/.cargo/bin:$PATH"
  export PATH
}

SRC=""
build_from_source() {
  section "Build from source"
  ensure_xcode_tools
  if [ -n "$SOURCE_DIR" ]; then
    SRC="$SOURCE_DIR"
  elif [ -n "$SCRIPT_DIR" ] && is_checkout "$SCRIPT_DIR/.."; then
    SRC="$(cd "$SCRIPT_DIR/.." && pwd)"
  else
    SRC="$HOME/.local/src/textweaver"
  fi
  if is_checkout "$SRC"; then
    say "Using the textweaver source in $SRC."
  else
    say "No textweaver source was found, so the script can clone it into $SRC."
    ask "Clone https://github.com/$REPO now?" || die "The source is needed. Clone it yourself, then use --source."
    run mkdir -p "$(dirname "$SRC")"
    run_plain git clone --depth 1 "https://github.com/$REPO.git" "$SRC"
  fi

  if ! have cargo && [ -x "$HOME/.cargo/bin/cargo" ]; then
    PATH="$HOME/.cargo/bin:$PATH"
    export PATH
  fi
  if have rustup; then
    say "rustup is installed. It installs the Rust version pinned in rust-toolchain.toml."
  elif have cargo; then
    local v
    v="$(cargo --version | awk '{ print $2 }' | sed 's/-.*//')"
    if version_at_least 1.89.0 "$v"; then
      say "cargo $v is installed without rustup, and it is new enough."
    else
      say "cargo $v is too old: textweaver needs Rust 1.89 or later."
      install_rustup || die "Install rustup from https://rustup.rs, then run this script again."
    fi
  else
    say "Rust is not installed."
    install_rustup || die "Rust is needed to build textweaver. Install rustup from https://rustup.rs, then run this script again."
  fi
  if [ "$DRY_RUN" = 1 ]; then
    say "Would run in $SRC: rustup toolchain install"
    say "Would run in $SRC: cargo xtask dist"
  else
    if have rustup; then
      (cd "$SRC" && run_plain rustup toolchain install) || true
    fi
    say "Building textweaver and tw in release mode with cargo xtask dist. The first build takes several minutes."
    (cd "$SRC" && run cargo xtask dist)
  fi
  VERSION="$(awk -F'"' '/^\[workspace.package\]/ { on = 1 } on && /^version/ { print $2; exit }' "$SRC/Cargo.toml" 2> /dev/null || true)"
  [ -n "$VERSION" ] || VERSION="VERSION"
  local arch
  arch="$(uname -m)"
  [ "$arch" = arm64 ] && arch=aarch64
  STAGE="${CARGO_TARGET_DIR:-$SRC/target}/dist/textweaver-$VERSION-macos-$arch"
  TAG="source"
}

# -------------------------------------------------------------- install --

write_file() {
  if [ "$DRY_RUN" = 1 ]; then
    cat > /dev/null
    say "Would write $1"
    return 0
  fi
  cat > "$1"
  say "Wrote $1"
}

install_stage() {
  section "Install"
  say "Installing textweaver and tw into $BINDIR, and the guides into $DOCDIR."
  if [ "$DRY_RUN" = 0 ]; then
    [ -x "$STAGE/tw" ] || die "$STAGE/tw is missing, so there is nothing to install."
  fi
  run mkdir -p "$BINDIR" "$DOCDIR" "$DATADIR/scripts"
  local f
  for f in textweaver tw; do
    run install -m 0755 "$STAGE/$f" "$BINDIR/$f"
  done
  for f in QUICKSTART.md README.md LICENSE CHANGELOG.md INSTALL.md; do
    if [ "$DRY_RUN" = 1 ] || [ -f "$STAGE/$f" ]; then
      run install -m 0644 "$STAGE/$f" "$DOCDIR/$f"
    fi
  done
  if [ "$DRY_RUN" = 1 ] || [ -d "$STAGE/docs" ]; then
    run rm -rf "$DOCDIR/docs"
    run cp -R "$STAGE/docs" "$DOCDIR/docs"
  fi
  if [ -n "$SCRIPT_DIR" ]; then
    for f in install-macos.sh update.sh doctor.sh speech-check.sh convert-folder.sh README.md; do
      if [ -f "$SCRIPT_DIR/$f" ]; then
        case $f in
          *.md) run install -m 0644 "$SCRIPT_DIR/$f" "$DATADIR/scripts/$f" ;;
          *) run install -m 0755 "$SCRIPT_DIR/$f" "$DATADIR/scripts/$f" ;;
        esac
      fi
    done
  fi
  {
    say "# textweaver install manifest, written by scripts/install-macos.sh."
    if [ "$MODE" = source ]; then
      say "kind=source"
      say "source=$SRC"
    else
      say "kind=release"
      say "tag=$TAG"
    fi
    say "prefix=$PREFIX"
    say "version=$VERSION"
    if [ -n "$GUI_INSTALLED" ]; then
      say "gui=$GUI_INSTALLED"
    fi
  } | write_file "$MANIFEST"
}

offer_brew() {
  [ "$BREW" = no ] && return 0
  section "Optional tools"
  say "ffmpeg lets tw export-audio write MP3, Ogg, and other formats. pandoc lets textweaver open ODT, RTF, reStructuredText, Org, and LaTeX files."
  if ! have brew; then
    say "Homebrew is not installed, so the script does not install them. If you want them, install Homebrew from https://brew.sh, then run: brew install ffmpeg pandoc"
    return 0
  fi
  local missing="" p
  for p in ffmpeg pandoc; do
    have "$p" || missing="$missing $p"
  done
  if [ -z "$missing" ]; then
    say "ffmpeg and pandoc are already installed."
    return 0
  fi
  if [ "$BREW" = yes ] || ask "Install$missing with Homebrew?"; then
    # shellcheck disable=SC2086
    run_plain env HOMEBREW_NO_COLOR=1 HOMEBREW_NO_EMOJI=1 brew install $missing \
      || warn "Homebrew did not install$missing. textweaver works without them."
  else
    say "Skipping the optional tools."
  fi
}

# ----------------------------------------------------------------- PATH --

path_has() {
  case ":$PATH:" in
    *":$1:"*) return 0 ;;
  esac
  return 1
}

shell_rc() {
  case ${SHELL##*/} in
    bash) printf '%s' "$HOME/.bash_profile" ;;
    fish) printf '%s' "$HOME/.config/fish/conf.d/textweaver.fish" ;;
    *) printf '%s' "$HOME/.zshrc" ;;
  esac
}

offer_path() {
  section "PATH"
  if path_has "$BINDIR"; then
    say "$BINDIR is already on your PATH, so you can run textweaver and tw from any folder."
    return 0
  fi
  local rc line shown_dir
  rc="$(shell_rc)"
  shown_dir="$BINDIR"
  case $BINDIR in
    "$HOME"/*) shown_dir="\$HOME/${BINDIR#"$HOME"/}" ;;
  esac
  case $rc in
    *.fish) line="set -gx PATH \"$shown_dir\" \$PATH $MARKER" ;;
    *) line="export PATH=\"$shown_dir:\$PATH\" $MARKER" ;;
  esac
  if [ -f "$rc" ] && grep -qF "$MARKER" "$rc"; then
    say "$(tilde "$rc") already adds $BINDIR to your PATH. Open a new Terminal window to use it."
    return 0
  fi
  say "$BINDIR is not on your PATH. The script can add this line to $(tilde "$rc"):"
  say "  $line"
  if ask "Add $BINDIR to your PATH?"; then
    if [ "$DRY_RUN" = 1 ]; then
      say "Would add that line to $rc"
    else
      mkdir -p "$(dirname "$rc")"
      printf '\n%s\n' "$line" >> "$rc"
      say "Added. Open a new Terminal window, or run: $line"
    fi
  else
    say "PATH unchanged. Run textweaver as $BINDIR/textweaver."
  fi
}

remove_path_line() {
  local rc tmp
  for rc in "$HOME/.zshrc" "$HOME/.bash_profile" "$HOME/.bashrc" "$HOME/.profile"; do
    if [ -f "$rc" ] && grep -qF "$MARKER" "$rc"; then
      if ask "$(tilde "$rc") has the PATH line the installer added. Remove it?"; then
        if [ "$DRY_RUN" = 1 ]; then
          say "Would remove the textweaver line from $rc"
        else
          tmp="$(mktemp -t textweaver-rc)"
          grep -vF "$MARKER" "$rc" > "$tmp" || true
          cat "$tmp" > "$rc"
          rm -f "$tmp"
          say "Removed the textweaver line from $(tilde "$rc")."
        fi
      fi
    fi
  done
  rc="$HOME/.config/fish/conf.d/textweaver.fish"
  if [ -f "$rc" ] && ask "Remove $(tilde "$rc"), which adds textweaver to fish's PATH?"; then
    run rm -f "$rc"
  fi
}

uninstall() {
  say "This removes textweaver from $PREFIX: the programs, the guides, and the helper scripts, and textweaver.app from $APPSDIR if this script installed it."
  say "It keeps your settings and reading positions, and it does not remove Rust or Homebrew packages."
  if [ ! -e "$BINDIR/tw" ] && [ ! -e "$DOCDIR" ] && [ ! -e "$DATADIR" ] && [ "$DRY_RUN" = 0 ]; then
    say "textweaver is not installed in $PREFIX, so there is nothing to remove."
    remove_path_line
    return 0
  fi
  ask "Remove textweaver from $PREFIX?" || {
    say "Nothing removed."
    exit 0
  }
  local f
  for f in "$BINDIR/textweaver" "$BINDIR/tw"; do
    if [ -e "$f" ] || [ "$DRY_RUN" = 1 ]; then run rm -f "$f"; fi
  done
  for f in "$DOCDIR" "$DATADIR"; do
    if [ -e "$f" ] || [ "$DRY_RUN" = 1 ]; then run rm -rf "$f"; fi
  done
  remove_gui
  remove_path_line
  say ""
  say "textweaver is removed from $PREFIX."
}

# ----------------------------------------------------------------- main --

if [ "$DRY_RUN" = 1 ]; then
  say "Dry run: nothing is changed. Each command is printed instead of run."
fi

if [ "$UNINSTALL" = 1 ]; then
  uninstall
  exit 0
fi

if [ "$MODE" = source ] && [ "$GUI" = 1 ]; then
  die "The GUI is installed from release packages: run without --from-source to use --gui. Building it from source is described in docs/gui.md."
fi

if [ "$MODE" = source ]; then
  say "This script builds textweaver from source and installs it under $PREFIX."
  build_from_source
else
  say "This script downloads the newest textweaver release for macOS, checks it, and installs it under $PREFIX."
  have curl || [ "$DRY_RUN" = 1 ] || die "curl is needed and is missing."
  download_release
  decide_gui
  if [ "$GUI" = 1 ]; then
    install_gui
  else
    remove_gui
  fi
fi
install_stage
offer_brew
offer_path

section "Done"
if [ "$DRY_RUN" = 1 ]; then
  say "Dry run finished. Nothing was changed."
  exit 0
fi
say "textweaver $VERSION is installed in $PREFIX."
say "Check your voices: $BINDIR/tw voices"
say "Read the quick start aloud: $BINDIR/textweaver $DOCDIR/QUICKSTART.md"
if [ -n "$GUI_INSTALLED" ]; then
  say "The GUI is $APPSDIR/textweaver.app. Open it from Finder or Spotlight."
fi
say "To remove textweaver, run this script with --uninstall."
