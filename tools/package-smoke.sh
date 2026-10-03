#!/usr/bin/env bash
# The packaged tw, run as a user would on first use, before a release is
# uploaded (release.yml) and in a clean container with no network
# (tools/clean-install-smoke.sh):
#
#   bash tools/package-smoke.sh PATH/TO/tw [FIXTURE]
#
# FIXTURE defaults to fixtures/sample.md. Every command runs on a fresh,
# empty TEXTWEAVER_HOME, so nothing touches the runner's own profile and
# nothing is found from an earlier run. It checks, one line each, meaning
# first ("Pass:" or "Fail:"):
# - tw --version prints a version;
# - tw text opens the fixture and prints its text;
# - tw info prints the fixture's facts (its word count);
# - tw components list runs with no components installed;
# - tw backends lists the speech engines;
# - tw convert writes the fixture as EPUB, a ZIP file that is not empty.
# Exit status 1 when any check failed. More than --version: a package can
# start and still be missing the data it needs to open a file.
set -euo pipefail

tw="${1:?usage: tools/package-smoke.sh PATH/TO/tw [FIXTURE]}"
fixture="${2:-fixtures/sample.md}"
[ -f "$fixture" ] || {
  echo "Fail: the fixture $fixture is missing"
  exit 1
}

TEXTWEAVER_HOME="$(mktemp -d)"
export TEXTWEAVER_HOME
out="$(mktemp -d)"
fail=0

# check WHAT PATTERN COMMAND...: the command succeeds and its output
# matches PATTERN (an extended regular expression; "." for any output).
check() {
  local what="$1" pattern="$2"
  shift 2
  local text status=0
  text="$("$@" 2>&1)" || status=$?
  if [ "$status" -ne 0 ]; then
    echo "Fail: $what, exit status $status"
    printf '%s\n' "$text" | sed -n '1,10s/^/  /p'
    fail=1
  elif ! printf '%s\n' "$text" | grep -Eq -- "$pattern"; then
    echo "Fail: $what, the output did not match $pattern"
    printf '%s\n' "$text" | sed -n '1,10s/^/  /p'
    fail=1
  else
    echo "Pass: $what"
    printf '%s\n' "$text" | sed -n '1,2s/^/  /p'
  fi
}

check "tw --version" '[0-9]+\.[0-9]+\.[0-9]+' "$tw" --version
check "tw text opens the fixture" '[[:alpha:]]' "$tw" text "$fixture"
check "tw info on the fixture" '[Ww]ords?' "$tw" info "$fixture"
check "tw components list" '.' "$tw" components list
check "tw backends" '.' "$tw" backends
check "tw convert to EPUB" '.' "$tw" convert "$fixture" --to epub --out "$out" --no-pandoc
epub="$out/$(basename "${fixture%.*}").epub"
if [ -s "$epub" ] && [ "$(head -c 2 "$epub")" = "PK" ]; then
  echo "Pass: the EPUB was written, $(wc -c < "$epub" | tr -d ' ') bytes"
else
  echo "Fail: no EPUB at $epub"
  for f in "$out"/*; do
    [ -e "$f" ] && echo "  found: $(basename "$f")"
  done
  fail=1
fi

if [ "$fail" -ne 0 ]; then
  echo "Fail: the package smoke test"
else
  echo "Pass: the package smoke test"
fi
exit "$fail"
