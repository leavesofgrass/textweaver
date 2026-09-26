#!/usr/bin/env bash
# install-linux.sh: build textweaver from source and install it, on any Linux
# distribution (apt, dnf, pacman, zypper, or apk; other systems get the list
# of packages to install by hand).
#
# Shell: bash (3.2 or later). Safe to run twice. See scripts/README.md, or
# run with --help.

set -eu
set -o pipefail

REPO_URL="${TEXTWEAVER_REPO_URL:-https://github.com/leavesofgrass/textweaver.git}"
MARKER="# added by the textweaver installer"

DRY_RUN=0
ASSUME_YES=0
PREFIX="$HOME/.local"
MODE="source"
RELEASE_TAG=""
SOURCE_DIR=""
PM=""
INSTALL_DEPS=1
ESPEAK=1
OPTIONAL="ask"
UNINSTALL=0

usage() {
  cat <<'EOF'
Usage: scripts/install-linux.sh [options]

Builds textweaver from source and installs it for you, on any Linux
distribution. It installs the build dependencies with your package manager,
installs Rust with rustup if cargo is missing, builds textweaver and tw in
release mode with the Linux speech engines (espeak-ng, speech-dispatcher, and
Omnivox), builds the engine hosts, and installs everything under a prefix.

Every step is announced before it runs. The script asks before it uses sudo,
installs Rust, or changes your PATH. Running it again updates the install.

Options:
  --prefix DIR           Install under DIR (default: ~/.local). Programs go
                         in DIR/bin, the engine hosts and dictionaries in
                         DIR/lib/textweaver, and the guides in
                         DIR/share/doc/textweaver.
  --from-source          Build from source (the default).
  --release TAG          Install a published Linux package. None are
                         published yet, so this only says so.
  --source DIR           Use the textweaver checkout in DIR. Without it, the
                         checkout this script sits in is used, or the
                         repository is cloned to ~/.local/src/textweaver.
  --package-manager PM   Use PM (apt, dnf, yum, pacman, zypper, or apk)
                         instead of detecting it. "none" skips packages.
  --no-deps              Do not install system packages.
  --no-espeak            Build without the in-process espeak-ng engine.
                         speech-dispatcher still speaks with espeak-ng. The
                         script also falls back to this when the espeak-ng
                         build fails.
  --no-optional          Do not offer the optional tools (ffmpeg, pandoc).
  --optional             Install the optional tools without asking.
  --uninstall            Remove textweaver from the prefix. Your settings,
                         reading positions, and system packages are kept.
  --dry-run              Print each command instead of running it.
  --yes                  Answer yes to every question.
  -h, --help             Show this help.

Examples:
  scripts/install-linux.sh
  scripts/install-linux.sh --dry-run
  scripts/install-linux.sh --prefix /usr/local
  scripts/install-linux.sh --uninstall
EOF
}

# ---------------------------------------------------------------- helpers --

say() { printf '%s\n' "$*"; }
warn() { printf 'Warning: %s\n' "$*" >&2; }
die() {
  printf 'Error: %s\n' "$*" >&2
  exit 1
}

# Prints a command so it can be copied and run.
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

# Runs a command, or prints it in a dry run.
run() {
  if [ "$DRY_RUN" = 1 ]; then
    say "Would run: $(show_cmd "$@")"
    return 0
  fi
  say "Running: $(show_cmd "$@")"
  "$@"
}

# Like run, but pipes the output through cat, so tools that draw progress
# bars on a terminal print plain lines instead.
run_plain() {
  if [ "$DRY_RUN" = 1 ]; then
    say "Would run: $(show_cmd "$@")"
    return 0
  fi
  say "Running: $(show_cmd "$@")"
  "$@" 2>&1 | cat
}

# Asks a yes-or-no question; --yes answers yes, and no terminal answers no.
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

# $HOME shown as ~, so reports do not include the user name.
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
    --from-source) MODE="source" ;;
    --release)
      [ "$#" -ge 2 ] || die "--release needs a tag, such as v0.1.0-alpha.3."
      MODE="release"
      RELEASE_TAG="$2"
      shift
      ;;
    --release=*)
      MODE="release"
      RELEASE_TAG="${1#*=}"
      ;;
    --source)
      [ "$#" -ge 2 ] || die "--source needs a folder."
      SOURCE_DIR="$2"
      shift
      ;;
    --source=*) SOURCE_DIR="${1#*=}" ;;
    --package-manager)
      [ "$#" -ge 2 ] || die "--package-manager needs a name."
      PM="$2"
      shift
      ;;
    --package-manager=*) PM="${1#*=}" ;;
    --no-deps) INSTALL_DEPS=0 ;;
    --no-espeak) ESPEAK=0 ;;
    --no-optional) OPTIONAL="no" ;;
    --optional) OPTIONAL="yes" ;;
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
LIBDIR="$PREFIX/lib/textweaver"
DOCDIR="$PREFIX/share/doc/textweaver"
MANDIR="$PREFIX/share/man/man1"
APPDIR="$PREFIX/share/applications"
DATADIR="$PREFIX/share/textweaver"
MANIFEST="$DATADIR/install-manifest.txt"

# The build honours NO_COLOR and never draws a progress bar.
export CARGO_TERM_PROGRESS_WHEN=never
if [ -n "${NO_COLOR:-}" ]; then
  export CARGO_TERM_COLOR=never
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# --------------------------------------------------------------- sudo ----

# SUDO holds the command that gives administrator rights ("" when running as
# root). The question is asked once.
SUDO_DECIDED=0
SUDO=""
need_root() {
  if [ "$SUDO_DECIDED" = 1 ]; then
    return 0
  fi
  if [ "$(id -u)" = 0 ]; then
    SUDO=""
  elif have sudo; then
    ask "$1 needs administrator rights, so the script will run it with sudo. sudo may ask for your password. Use sudo?" \
      || return 1
    SUDO="sudo"
  elif have doas; then
    ask "$1 needs administrator rights, so the script will run it with doas. Use doas?" || return 1
    SUDO="doas"
  else
    warn "$1 needs administrator rights, and neither sudo nor doas is installed. Run this script as root, or do that step yourself."
    return 1
  fi
  SUDO_DECIDED=1
}

# Runs a command as root (through $SUDO).
as_root() {
  if [ -n "$SUDO" ]; then
    run_plain "$SUDO" "$@"
  else
    run_plain "$@"
  fi
}

# ------------------------------------------------------- package manager --

OS_NAME="Linux"
OS_ID=""
OS_LIKE=""
detect_os() {
  if [ -r /etc/os-release ]; then
    local info
    # shellcheck disable=SC1091
    info="$(. /etc/os-release && printf '%s\n%s\n%s\n' "${PRETTY_NAME:-${NAME:-Linux}}" "${ID:-}" "${ID_LIKE:-}")"
    OS_NAME="$(printf '%s\n' "$info" | sed -n 1p)"
    OS_ID="$(printf '%s\n' "$info" | sed -n 2p)"
    OS_LIKE="$(printf '%s\n' "$info" | sed -n 3p)"
  fi
}

# Picks the package manager from /etc/os-release, then from what is on PATH.
detect_pm() {
  if [ -n "$PM" ]; then
    return 0
  fi
  local id
  for id in $OS_ID $OS_LIKE; do
    case $id in
      debian | ubuntu | linuxmint | pop | raspbian | kali | elementary | zorin | neon | devuan)
        have apt-get && PM=apt && return 0
        ;;
      fedora | rhel | centos | rocky | almalinux | ol | amzn | nobara)
        if have dnf; then
          PM=dnf
          return 0
        fi
        have yum && PM=yum && return 0
        ;;
      arch | manjaro | endeavouros | garuda | artix | cachyos)
        have pacman && PM=pacman && return 0
        ;;
      opensuse* | suse | sles | sled)
        have zypper && PM=zypper && return 0
        ;;
      alpine | postmarketos)
        have apk && PM=apk && return 0
        ;;
    esac
  done
  local cand
  for cand in apt-get dnf yum pacman zypper apk; do
    if have "$cand"; then
      case $cand in
        apt-get) PM=apt ;;
        *) PM=$cand ;;
      esac
      return 0
    fi
  done
  PM=unknown
}

# Build dependencies for each package manager: a C toolchain, clang and
# libclang (bindgen), pkg-config, ALSA headers (audio output), espeak-ng
# with its headers, speech-dispatcher with its headers, and git and curl.
packages_for() {
  case $1 in
    apt)
      echo "build-essential clang libclang-dev pkg-config libasound2-dev espeak-ng libespeak-ng-dev speech-dispatcher speech-dispatcher-espeak-ng libspeechd-dev git curl ca-certificates"
      ;;
    dnf | yum)
      echo "gcc gcc-c++ make clang clang-devel pkgconf-pkg-config alsa-lib-devel espeak-ng espeak-ng-devel speech-dispatcher speech-dispatcher-espeak-ng speech-dispatcher-devel git curl ca-certificates"
      ;;
    pacman)
      echo "base-devel clang pkgconf alsa-lib espeak-ng speech-dispatcher git curl ca-certificates"
      ;;
    zypper)
      echo "gcc gcc-c++ make clang clang-devel pkgconf-pkg-config alsa-devel espeak-ng espeak-ng-devel speech-dispatcher speech-dispatcher-module-espeak libspeechd-devel git curl ca-certificates"
      ;;
    apk)
      echo "build-base clang clang-dev clang-libclang pkgconf alsa-lib-dev espeak-ng espeak-ng-dev speech-dispatcher speech-dispatcher-dev git curl ca-certificates"
      ;;
    *)
      echo ""
      ;;
  esac
}

# Optional tools: ffmpeg (audio export formats) and pandoc (more document
# formats).
optional_for() {
  case $1 in
    apt | zypper | apk) echo "ffmpeg pandoc" ;;
    dnf | yum) echo "ffmpeg-free pandoc" ;;
    pacman) echo "ffmpeg pandoc-cli" ;;
    *) echo "" ;;
  esac
}

pm_label() {
  case $1 in
    apt) echo "apt (Debian and Ubuntu family)" ;;
    dnf) echo "dnf (Fedora and Red Hat family)" ;;
    yum) echo "yum (older Red Hat family)" ;;
    pacman) echo "pacman (Arch family)" ;;
    zypper) echo "zypper (openSUSE family)" ;;
    apk) echo "apk (Alpine)" ;;
    none) echo "none (skipping system packages)" ;;
    *) echo "unknown" ;;
  esac
}

APT_UPDATED=0
pm_install() {
  # shellcheck disable=SC2206
  local pkgs=($*)
  case $PM in
    apt)
      if [ "$APT_UPDATED" = 0 ]; then
        as_root env DEBIAN_FRONTEND=noninteractive apt-get update -q
        APT_UPDATED=1
      fi
      as_root env DEBIAN_FRONTEND=noninteractive apt-get install -y -q --no-install-recommends \
        -o Dpkg::Progress-Fancy=0 "${pkgs[@]}"
      ;;
    dnf) as_root dnf install -y "${pkgs[@]}" ;;
    yum) as_root yum install -y "${pkgs[@]}" ;;
    pacman) as_root pacman -Syu --needed --noconfirm --noprogressbar "${pkgs[@]}" ;;
    zypper) as_root zypper --non-interactive install --no-recommends "${pkgs[@]}" ;;
    apk) as_root apk add --no-progress "${pkgs[@]}" ;;
    *) return 1 ;;
  esac
}

print_generic_packages() {
  say "Install these with your package manager, then run this script again with --no-deps:"
  say "- a C compiler and make (gcc or clang)"
  say "- clang and libclang (the development files)"
  say "- pkg-config"
  say "- the ALSA development files (often alsa-lib-devel or libasound2-dev)"
  say "- espeak-ng and its development files"
  say "- speech-dispatcher and its development files, and its espeak-ng module"
  say "- git and curl"
  say "Optional: ffmpeg and pandoc."
}

DEPS_INSTALLED=0
install_dependencies() {
  section "System packages"
  detect_os
  detect_pm
  say "This system is $OS_NAME."
  say "Package manager: $(pm_label "$PM")."
  if [ "$INSTALL_DEPS" = 0 ] || [ "$PM" = none ]; then
    say "Skipping system packages, as asked."
    return 0
  fi
  if [ "$PM" = unknown ]; then
    say "No supported package manager was found."
    print_generic_packages
    return 0
  fi
  local pkgs
  pkgs="$(packages_for "$PM")"
  say "Build dependencies for $PM: $pkgs"
  if [ "$PM" = pacman ]; then
    say "On Arch, pacman installs with -Syu, which also upgrades the rest of the system, as Arch recommends."
  fi
  if [ "$PM" = dnf ] || [ "$PM" = yum ]; then
    case " $OS_ID $OS_LIKE " in
      *" fedora "*) ;;
      *) say "On Red Hat, Rocky, and Alma Linux, espeak-ng and speech-dispatcher come from EPEL. Enable EPEL first if the install cannot find them." ;;
    esac
  fi
  if ask "Install the build dependencies now?"; then
    if need_root "Installing packages"; then
      pm_install "$pkgs"
      DEPS_INSTALLED=1
    else
      say "Skipping system packages. If the build fails, install the packages listed above."
    fi
  else
    say "Skipping system packages. If the build fails, install the packages listed above."
  fi

  # Optional tools.
  local opt
  opt="$(optional_for "$PM")"
  if [ "$OPTIONAL" != no ] && [ -n "$opt" ]; then
    say ""
    say "Optional tools: $opt. ffmpeg lets tw export-audio write MP3, Ogg, and other formats. pandoc lets textweaver open ODT, RTF, reStructuredText, Org, and LaTeX files."
    if [ "$PM" = dnf ] || [ "$PM" = yum ]; then
      say "Fedora's ffmpeg-free covers the common formats. For the full ffmpeg, enable RPM Fusion."
    fi
    local want=1
    if [ "$OPTIONAL" = ask ]; then
      ask "Install the optional tools?" || want=0
    fi
    if [ "$want" = 1 ]; then
      if need_root "Installing packages"; then
        pm_install "$opt" || warn "The optional tools did not install. textweaver works without them."
      fi
    else
      say "Skipping the optional tools."
    fi
  fi
  say ""
  say "Dictation uses whisper.cpp (the whisper-cli program) when it is installed. Most distributions do not package it yet. Build it from https://github.com/ggml-org/whisper.cpp, or on Arch install whisper.cpp from the AUR."
  if have whisper-cli; then
    say "whisper-cli is already on your PATH."
  fi
}

# --------------------------------------------------------------- source --

is_checkout() {
  [ -f "$1/Cargo.toml" ] && [ -d "$1/crates/textweaver-cli" ]
}

SRC=""
find_source() {
  section "Source code"
  if [ -n "$SOURCE_DIR" ]; then
    SRC="$SOURCE_DIR"
  elif is_checkout "$SCRIPT_DIR/.."; then
    SRC="$(cd "$SCRIPT_DIR/.." && pwd)"
  else
    SRC="$HOME/.local/src/textweaver"
  fi
  if is_checkout "$SRC"; then
    say "Using the textweaver source in $SRC."
    return 0
  fi
  if [ -e "$SRC" ] && [ -n "$(ls -A "$SRC" 2> /dev/null)" ]; then
    die "$SRC exists but is not a textweaver checkout. Choose another folder with --source."
  fi
  say "No textweaver source was found, so the script can clone it from $REPO_URL into $SRC."
  ask "Clone the repository now?" || die "The source is needed to build. Clone it yourself, then use --source."
  have git || [ "$DRY_RUN" = 1 ] || die "git is not installed. Install git, then run this script again."
  run mkdir -p "$(dirname "$SRC")"
  run git clone --depth 1 "$REPO_URL" "$SRC"
}

# ----------------------------------------------------------------- rust --

version_at_least() {
  # version_at_least 1.89.0 1.96.1: is the second at least the first?
  local want="$1" got="$2"
  [ "$(printf '%s\n%s\n' "$want" "$got" | sort -t. -k1,1n -k2,2n -k3,3n | head -n 1)" = "$want" ]
}

ensure_rust() {
  section "Rust"
  if ! have cargo && [ -x "$HOME/.cargo/bin/cargo" ]; then
    say "Found Rust in ~/.cargo/bin, which is not on your PATH. Using it for this build."
    PATH="$HOME/.cargo/bin:$PATH"
    export PATH
  fi
  if have rustup; then
    say "rustup is installed. It installs the Rust version this project pins in rust-toolchain.toml."
  elif have cargo; then
    local v
    v="$(cargo --version 2> /dev/null | awk '{print $2}' | sed 's/-.*//')"
    say "cargo $v is installed without rustup. textweaver needs Rust 1.89 or later, and is tested with the version in rust-toolchain.toml."
    if version_at_least 1.89.0 "$v"; then
      say "That version is new enough, so the script uses it."
      return 0
    fi
    say "That version is too old."
    install_rustup || die "Rust 1.89 or later is needed. Install rustup from https://rustup.rs, then run this script again."
  else
    say "Rust is not installed. The script can install rustup, the Rust installer, from https://sh.rustup.rs into ~/.cargo and ~/.rustup, for you only. It does not change your PATH."
    install_rustup || die "Rust is needed to build textweaver. Install rustup from https://rustup.rs, then run this script again."
  fi
  if have rustup || [ "$DRY_RUN" = 1 ]; then
    say "Installing the pinned Rust toolchain, if it is not installed yet."
    if [ "$DRY_RUN" = 1 ]; then
      say "Would run in $SRC: rustup toolchain install"
    else
      (cd "$SRC" && run_plain rustup toolchain install) || (cd "$SRC" && run_plain rustup show active-toolchain)
    fi
  fi
}

install_rustup() {
  ask "Install rustup now?" || return 1
  have curl || [ "$DRY_RUN" = 1 ] || die "curl is needed to download rustup. Install curl, then run this script again."
  if [ "$DRY_RUN" = 1 ]; then
    say "Would run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y -q --no-modify-path --profile minimal --default-toolchain none"
  else
    say "Running: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y -q --no-modify-path --profile minimal --default-toolchain none"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y -q --no-modify-path --profile minimal --default-toolchain none 2>&1 | cat
  fi
  PATH="$HOME/.cargo/bin:$PATH"
  export PATH
  say "rustup is installed in ~/.cargo/bin. That folder is not on your PATH; scripts/update.sh finds it anyway."
}

# ---------------------------------------------------------------- build --

FEATURES=""
ENGINES=""
check_build_tools() {
  local missing=""
  have cc || have gcc || have clang || missing="$missing a C compiler,"
  have pkg-config || have pkgconf || missing="$missing pkg-config,"
  if have pkg-config && ! pkg-config --exists alsa; then
    missing="$missing the ALSA development files,"
  fi
  if [ -n "$missing" ]; then
    missing="${missing%,}"
    die "These are missing:$missing. Install the build dependencies (run this script without --no-deps), then try again."
  fi
}

choose_features() {
  local engines="omnivox speechd"
  if [ "$ESPEAK" = 0 ]; then
    say "Leaving out the in-process espeak-ng engine, as asked. textweaver can still speak through speech-dispatcher."
  elif [ "$DRY_RUN" = 1 ] || [ "$DEPS_INSTALLED" = 1 ] \
    || { have pkg-config && pkg-config --exists espeak-ng; } \
    || [ -f /usr/include/espeak-ng/speak_lib.h ]; then
    engines="espeak $engines"
  else
    say "The espeak-ng development files were not found, so textweaver is built without the in-process espeak-ng engine. It can still speak through speech-dispatcher."
  fi
  ENGINES="$engines"
  set_features
  say "Speech engines built in: $engines."
}

# FEATURES for the engines in ENGINES, for both programs.
set_features() {
  local f pkg
  FEATURES=""
  for pkg in textweaver-cli textweaver-tui; do
    for f in $ENGINES; do
      FEATURES="$FEATURES${FEATURES:+,}$pkg/$f"
    done
  done
}

TARGET_DIR=""
build() {
  section "Build"
  TARGET_DIR="${CARGO_TARGET_DIR:-$SRC/target}"
  if [ "$DRY_RUN" = 0 ]; then
    check_build_tools
  fi
  choose_features
  say "Building textweaver and tw in release mode. The first build takes several minutes."
  if [ "$DRY_RUN" = 1 ]; then
    say "Would run in $SRC: cargo build --release --locked -p textweaver-tui -p textweaver-cli --bin textweaver --bin tw --features $FEATURES"
    say "Would run in $SRC: cargo xtask hosts"
    return 0
  fi
  if ! (cd "$SRC" && run cargo build --release --locked -p textweaver-tui -p textweaver-cli \
    --bin textweaver --bin tw --features "$FEATURES"); then
    case " $ENGINES " in
      *" espeak "*) ;;
      *) die "The build failed. The error is above. scripts/doctor.sh collects what a bug report needs." ;;
    esac
    say ""
    say "The build failed. On new distributions the usual cause is the in-process espeak-ng engine: its generated bindings can disagree with the system headers. The error is above."
    say "Building again without it. textweaver can still speak with espeak-ng through speech-dispatcher. Use --no-espeak to skip the first attempt next time."
    ENGINES="$(printf '%s' "$ENGINES" | sed 's/espeak //')"
    set_features
    (cd "$SRC" && run cargo build --release --locked -p textweaver-tui -p textweaver-cli \
      --bin textweaver --bin tw --features "$FEATURES") \
      || die "The build failed again. The error is above. scripts/doctor.sh collects what a bug report needs."
    say "Built without espeak-ng. Speech engines built in: $ENGINES."
  fi
  say "Building the engine hosts: the helper programs that run Eloquence (Voxin) and DECtalk in their own processes."
  (cd "$SRC" && run cargo xtask hosts)
}

# -------------------------------------------------------------- install --

# Writes standard input to a file, or says so in a dry run.
write_file() {
  if [ "$DRY_RUN" = 1 ]; then
    cat > /dev/null
    say "Would write $1"
    return 0
  fi
  cat > "$1"
  say "Wrote $1"
}

# Turns --help text into a small manual page.
man_page() {
  local name="$1" summary="$2" text="$3"
  printf '.TH %s 1 "" "textweaver" "User Commands"\n' "$(printf '%s' "$name" | tr '[:lower:]' '[:upper:]')"
  printf '.SH NAME\n%s \\- %s\n' "$name" "$summary"
  printf '.SH DESCRIPTION\n.nf\n'
  printf '%s\n' "$text" | sed -e 's/\\/\\e/g' -e 's/^\([.'"'"']\)/\\\&\1/'
  printf '.fi\n.SH SEE ALSO\nThe guides in %s, and https://github.com/leavesofgrass/textweaver\n' "$DOCDIR"
}

# Lists tw's subcommands from its --help output.
tw_subcommands() {
  "$1" --help | awk '/^Commands:/ { on = 1; next } on && NF == 0 { exit } on { print $1 }' | grep -v '^help$'
}

INSTALL_SUDO=""
install_files() {
  section "Install"
  say "Installing into $PREFIX:"
  say "- the programs textweaver and tw, linked from $BINDIR;"
  say "- the programs, engine hosts, and pronunciation dictionaries in $LIBDIR;"
  say "- the guides and a help file in $DOCDIR, and manual pages in $MANDIR;"
  say "- a menu entry in $APPDIR."

  local parent="$PREFIX"
  while [ ! -e "$parent" ]; do parent="$(dirname "$parent")"; done
  if [ ! -w "$parent" ] || { [ -e "$BINDIR" ] && [ ! -w "$BINDIR" ]; }; then
    if need_root "Writing to $PREFIX"; then
      INSTALL_SUDO="$SUDO"
    else
      die "Cannot write to $PREFIX. Choose another folder with --prefix."
    fi
  fi

  local rel="$TARGET_DIR/release"
  if [ "$DRY_RUN" = 0 ]; then
    [ -x "$rel/tw" ] || die "The build did not produce $rel/tw."
  fi

  ir mkdir -p "$BINDIR" "$LIBDIR" "$DOCDIR" "$MANDIR" "$APPDIR" "$DATADIR/scripts"
  local f
  for f in textweaver tw textweaver-eci-host textweaver-dectalk-host; do
    if [ "$DRY_RUN" = 1 ] || [ -f "$rel/$f" ]; then
      ir install -m 0755 "$rel/$f" "$LIBDIR/$f"
    else
      warn "$rel/$f was not built, so it is not installed."
    fi
  done
  ir rm -rf "$LIBDIR/ibmtts-dictionaries"
  ir cp -R "$SRC/third_party/ibmtts-dictionaries" "$LIBDIR/ibmtts-dictionaries"
  for f in textweaver tw; do
    ir ln -sfn "../lib/textweaver/$f" "$BINDIR/$f"
  done

  # Guides.
  ir install -m 0644 "$SRC/docs/quickstart.md" "$DOCDIR/QUICKSTART.md"
  ir install -m 0644 "$SRC/README.md" "$DOCDIR/README.md"
  ir install -m 0644 "$SRC/LICENSE" "$DOCDIR/LICENSE"
  ir install -m 0644 "$SRC/CHANGELOG.md" "$DOCDIR/CHANGELOG.md"
  ir install -m 0644 "$SRC/docs/install.md" "$DOCDIR/INSTALL.md"
  ir mkdir -p "$DOCDIR/docs"
  for f in eloquence keyboard docker themes settings converting reading-aids dectalk; do
    if [ -f "$SRC/docs/$f.md" ]; then
      ir install -m 0644 "$SRC/docs/$f.md" "$DOCDIR/docs/$f.md"
    fi
  done

  # Helper scripts, so doctor, speech-check, and update work after install.
  for f in install-linux.sh update.sh doctor.sh speech-check.sh convert-folder.sh README.md; do
    if [ -f "$SCRIPT_DIR/$f" ]; then
      ir install -m 0755 "$SCRIPT_DIR/$f" "$DATADIR/scripts/$f"
    fi
  done

  install_help
  install_desktop
  write_manifest
}

# Runs an install step, through sudo when the prefix needs it.
ir() {
  if [ -n "$INSTALL_SUDO" ]; then
    run "$INSTALL_SUDO" "$@"
  else
    run "$@"
  fi
}

# Writes to a file in the prefix, through sudo when needed.
iw() {
  if [ -n "$INSTALL_SUDO" ] && [ "$DRY_RUN" = 0 ]; then
    "$INSTALL_SUDO" tee "$1" > /dev/null
    say "Wrote $1"
  else
    write_file "$1"
  fi
}

install_help() {
  local tw="$LIBDIR/tw" tv="$LIBDIR/textweaver"
  if [ "$DRY_RUN" = 1 ]; then
    say "Would write $DOCDIR/HELP.txt, from the --help of textweaver, tw, and each tw command."
    say "Would write $MANDIR/textweaver.1 and $MANDIR/tw.1."
    return 0
  fi
  {
    say "textweaver help"
    say "==============="
    say ""
    say "This file collects the --help text of every command. Read it with any"
    say "text viewer, or run the commands with --help yourself."
    say ""
    say "textweaver --help"
    say "-----------------"
    "$tv" --help
    say ""
    say "tw --help"
    say "---------"
    "$tw" --help
    local sub
    for sub in $(tw_subcommands "$tw"); do
      say ""
      say "tw $sub --help"
      say "-------------"
      "$tw" "$sub" --help
    done
  } | iw "$DOCDIR/HELP.txt"
  man_page textweaver "read documents aloud in the terminal" "$("$tv" --help)" | iw "$MANDIR/textweaver.1"
  man_page tw "read, convert, and speak documents from the command line" "$("$tw" --help)" | iw "$MANDIR/tw.1"
}

install_desktop() {
  local template="$SCRIPT_DIR/linux/textweaver.desktop"
  if [ ! -f "$template" ]; then
    warn "The menu entry template $template is missing, so no menu entry is installed."
    return 0
  fi
  local exe="$BINDIR/textweaver"
  sed -e "s|^Exec=textweaver|Exec=\"$exe\"|" -e "s|^TryExec=textweaver|TryExec=$exe|" "$template" \
    | iw "$APPDIR/textweaver.desktop"
  if have update-desktop-database && [ -z "$INSTALL_SUDO" ]; then
    run update-desktop-database "$APPDIR" || true
  fi
}

write_manifest() {
  local version="unknown"
  if [ -f "$SRC/Cargo.toml" ]; then
    version="$(awk -F'"' '/^\[workspace.package\]/ { on = 1 } on && /^version/ { print $2; exit }' "$SRC/Cargo.toml")"
  fi
  {
    say "# textweaver install manifest, written by scripts/install-linux.sh."
    say "kind=source"
    say "source=$SRC"
    say "prefix=$PREFIX"
    say "version=$version"
    say "features=$FEATURES"
  } | iw "$MANIFEST"
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
    zsh) printf '%s' "$HOME/.zshrc" ;;
    bash) printf '%s' "$HOME/.bashrc" ;;
    fish) printf '%s' "$HOME/.config/fish/conf.d/textweaver.fish" ;;
    *) printf '%s' "$HOME/.profile" ;;
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
    say "$(tilde "$rc") already adds $BINDIR to your PATH. Open a new terminal to use it."
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
      say "Added. Open a new terminal, or run: $line"
    fi
  else
    say "PATH unchanged. Run textweaver as $BINDIR/textweaver, or add $BINDIR to your PATH yourself."
  fi
}

remove_path_line() {
  local rc
  for rc in "$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.profile" "$HOME/.bash_profile"; do
    if [ -f "$rc" ] && grep -qF "$MARKER" "$rc"; then
      if ask "$(tilde "$rc") has the PATH line the installer added. Remove it?"; then
        if [ "$DRY_RUN" = 1 ]; then
          say "Would remove the textweaver line from $rc"
        else
          local tmp
          tmp="$(mktemp)"
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

# ------------------------------------------------------------ uninstall --

uninstall() {
  say "This removes textweaver from $PREFIX: the programs, engine hosts, dictionaries, guides, manual pages, and menu entry."
  say "It keeps your settings, reading positions, and notes, and it does not remove system packages or Rust."
  if [ -x "$LIBDIR/tw" ] && [ "$DRY_RUN" = 0 ]; then
    say "Your settings stay here:"
    "$LIBDIR/tw" settings path 2> /dev/null || true
  fi
  if [ ! -e "$LIBDIR" ] && [ ! -e "$DOCDIR" ] && [ ! -e "$DATADIR" ] && [ ! -L "$BINDIR/tw" ] && [ "$DRY_RUN" = 0 ]; then
    say "textweaver is not installed in $PREFIX, so there is nothing to remove."
    remove_path_line
    return 0
  fi
  ask "Remove textweaver from $PREFIX?" || {
    say "Nothing removed."
    exit 0
  }
  local parent="$PREFIX"
  while [ ! -e "$parent" ]; do parent="$(dirname "$parent")"; done
  if [ ! -w "$parent" ] || { [ -e "$LIBDIR" ] && [ ! -w "$LIBDIR" ]; }; then
    need_root "Removing files from $PREFIX" || die "Cannot write to $PREFIX."
    INSTALL_SUDO="$SUDO"
  fi
  local f
  for f in textweaver tw; do
    if [ "$DRY_RUN" = 1 ] && [ ! -L "$BINDIR/$f" ]; then
      ir rm -f "$BINDIR/$f"
    elif [ -L "$BINDIR/$f" ]; then
      case "$(readlink "$BINDIR/$f")" in
        *lib/textweaver/*) ir rm -f "$BINDIR/$f" ;;
        *) say "Leaving $BINDIR/$f: it does not point into this install." ;;
      esac
    fi
  done
  for f in "$LIBDIR" "$DOCDIR" "$DATADIR"; do
    if [ -e "$f" ] || [ "$DRY_RUN" = 1 ]; then ir rm -rf "$f"; fi
  done
  for f in "$MANDIR/textweaver.1" "$MANDIR/tw.1" "$APPDIR/textweaver.desktop"; do
    if [ -e "$f" ] || [ "$DRY_RUN" = 1 ]; then ir rm -f "$f"; fi
  done
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

if [ "$MODE" = release ]; then
  say "Linux packages are not published yet, so there is no release $RELEASE_TAG to install."
  say "Build from source instead: run this script without --release."
  exit 1
fi

say "This script builds textweaver from source and installs it under $PREFIX."
say "It has six steps: system packages, the source code, Rust, the build, the install, and your PATH."
say "It asks before it uses sudo, installs Rust, or changes your PATH."

install_dependencies
find_source
ensure_rust
build
install_files
offer_path

section "Done"
if [ "$DRY_RUN" = 1 ]; then
  say "Dry run finished. Nothing was changed."
  exit 0
fi
say "textweaver is installed in $PREFIX."
say "Check your speech engines: $BINDIR/tw backends"
say "Read the quick start aloud: $BINDIR/textweaver $DOCDIR/QUICKSTART.md"
say "Help for every command is in $DOCDIR/HELP.txt."
say "To update later, run $DATADIR/scripts/update.sh. To remove textweaver, run this script with --uninstall."
