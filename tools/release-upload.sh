#!/usr/bin/env bash
# Upload release packages to the GitHub release for TAG, creating the
# release (a pre-release, notes from CHANGELOG.md) when it does not exist
# yet, then rebuild SHA256SUMS.txt from every package on the release.
# Used by .github/workflows/release.yml and by hand as the fallback when
# a package is built locally (docs/releasing.md).
#
#   tools/release-upload.sh [--no-sums] TAG [FILE...]
#
# --no-sums uploads without rebuilding SHA256SUMS.txt: the release
# workflow's package jobs use it, and its last job writes the checksums
# once, so parallel jobs never race on the file. With no FILE, the script
# only creates the release (if needed) and rebuilds the checksums.
set -euo pipefail

sums=1
if [ "${1:-}" = "--no-sums" ]; then
  sums=0
  shift
fi
tag="${1:?usage: tools/release-upload.sh [--no-sums] TAG [FILE...]}"
shift
version="${tag#v}"
root="$(cd "$(dirname "$0")/.." && pwd)"

notes="$(mktemp)"
work="$(mktemp -d)"
trap 'rm -rf "$notes" "$work"' EXIT

# The CHANGELOG section for this version, without its heading.
awk -v v="$version" '
  /^## / { if (found) exit; if (index($0, "[" v "]")) { found = 1; next } }
  found { print }
' "$root/CHANGELOG.md" > "$notes"
if [ ! -s "$notes" ]; then
  echo "Release $tag. See CHANGELOG.md." > "$notes"
fi

if ! gh release view "$tag" > /dev/null 2>&1; then
  if [[ "$version" == *-* || "$version" == 0.* ]]; then pre=--prerelease; else pre=; fi
  # Another job may create it at the same moment; fall through to upload.
  gh release create "$tag" --verify-tag $pre --title "textweaver $version" \
    --notes-file "$notes" || gh release view "$tag" > /dev/null
fi

if [ "$#" -gt 0 ]; then
  gh release upload "$tag" "$@" --clobber
fi

if [ "$sums" = 0 ]; then
  exit 0
fi

# Checksums of every package now on the release.
gh release download "$tag" --dir "$work" --pattern 'textweaver-*'
if command -v sha256sum > /dev/null; then sum=(sha256sum); else sum=(shasum -a 256); fi
(cd "$work" && "${sum[@]}" textweaver-* > SHA256SUMS.txt && cat SHA256SUMS.txt)
gh release upload "$tag" "$work/SHA256SUMS.txt" --clobber
