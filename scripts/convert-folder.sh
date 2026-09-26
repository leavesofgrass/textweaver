#!/usr/bin/env bash
# convert-folder.sh: convert a folder of Markdown (or other documents) to
# HTML, EPUB, PDF, DOCX, braille, or text with tw convert, with sensible
# defaults.
#
# Shell: bash (3.2 or later). See scripts/README.md, or run with --help.

set -eu
set -o pipefail

DRY_RUN=0
FORMAT="html"
OUT=""
WATCH=0
FORCE=0
FOLDER=""
TW="${TW:-}"
EXTRA=()

usage() {
  cat <<'EOF'
Usage: scripts/convert-folder.sh FOLDER [--to FORMAT] [--out DIR] [--watch]
                                 [--force] [--dry-run] [-- TW-CONVERT-OPTIONS]

Converts every document in FOLDER, and its subfolders, with tw convert. The
output goes to a folder beside it named after the format, so notes becomes
notes-html, and the subfolders are mirrored there. Files that are already
up to date are skipped, so running it again only converts what changed.

Options:
  --to FORMAT   html (the default), epub, pdf, docx, brf (braille), txt,
                or md. If your tw does not have a writer for a format
                yet, it says so and converts nothing.
  --out DIR     Write the output here instead of FOLDER-FORMAT.
  --watch       Keep watching FOLDER and convert new files as they arrive.
                Press Control+C to stop.
  --force       Convert every file, even when its output is newer.
  --tw PATH     The tw program to use (default: tw on your PATH).
  --dry-run     Print the tw convert command instead of running it.
  --yes         Accepted for consistency; this script asks nothing.
  -h, --help    Show this help.

Anything after -- goes to tw convert unchanged; run tw convert --help for
those options (for example --flavor obsidian, --smart, or --template print).

Examples:
  scripts/convert-folder.sh notes
  scripts/convert-folder.sh notes --to epub
  scripts/convert-folder.sh notes --to pdf --out ~/Documents/notes-pdf
  scripts/convert-folder.sh notes -- --flavor obsidian --smart
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
    --to)
      [ "$#" -ge 2 ] || die "--to needs a format."
      FORMAT="$2"
      shift
      ;;
    --to=*) FORMAT="${1#*=}" ;;
    --out)
      [ "$#" -ge 2 ] || die "--out needs a folder."
      OUT="$2"
      shift
      ;;
    --out=*) OUT="${1#*=}" ;;
    --watch) WATCH=1 ;;
    --force) FORCE=1 ;;
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
    --)
      shift
      EXTRA=("$@")
      break
      ;;
    -*) die "Unknown option $1. Run with --help to see the options." ;;
    *)
      [ -z "$FOLDER" ] || die "Give one folder. To convert several, run the script once for each."
      FOLDER="$1"
      ;;
  esac
  shift
done

[ -n "$FOLDER" ] || die "Name the folder to convert. Run with --help for examples."
[ -d "$FOLDER" ] || die "$FOLDER is not a folder."
case $FORMAT in
  html | epub | pdf | docx | brf | txt | md) ;;
  markdown) FORMAT=md ;;
  text) FORMAT=txt ;;
  *) die "Unknown format $FORMAT. Use html, epub, pdf, docx, brf, txt, or md." ;;
esac

FOLDER="${FOLDER%/}"
if [ -z "$OUT" ]; then
  OUT="$FOLDER-$FORMAT"
fi

if [ -z "$TW" ]; then
  if have tw; then
    TW=tw
  else
    root="$(cd "$(dirname "$0")/.." && pwd)"
    for d in "$root/target/release" "$root/target/debug"; do
      if [ -x "$d/tw" ]; then
        TW="$d/tw"
        break
      fi
    done
  fi
fi
[ -n "$TW" ] || [ "$DRY_RUN" = 1 ] || die "tw was not found. Install textweaver, or pass --tw PATH."
[ -n "$TW" ] || TW=tw

cmd=("$TW" convert "$FOLDER" --to "$FORMAT" --out "$OUT")
[ "$WATCH" = 1 ] && cmd+=(--watch)
[ "$FORCE" = 1 ] && cmd+=(--force)
cmd+=(${EXTRA[@]+"${EXTRA[@]}"})

if [ "$WATCH" = 1 ]; then
  say "Watching $FOLDER. New and changed documents are converted to $FORMAT in $OUT. Press Control+C to stop."
else
  say "Converting the documents in $FOLDER to $FORMAT, into $OUT."
fi
if [ "$DRY_RUN" = 1 ]; then
  say "Would run: $(show_cmd "${cmd[@]}")"
  exit 0
fi
say "Running: $(show_cmd "${cmd[@]}")"
"${cmd[@]}"
