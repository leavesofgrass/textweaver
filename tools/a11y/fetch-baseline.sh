#!/usr/bin/env bash
# Fetch an artifact from main's last successful run of a workflow, as the
# baseline a new accessibility tree is compared with (ADR-0039).
#
#   bash tools/a11y/fetch-baseline.sh WORKFLOW_FILE ARTIFACT DEST_DIR
#
# Writes the artifact's files into DEST_DIR and the run's id into
# DEST_DIR/run-id.txt. With no such run (the first run, or artifacts that
# expired) it says so and exits 0: a missing baseline is not a failure.
# Needs the GitHub CLI with GH_TOKEN (the job's token, `actions: read`) and
# GITHUB_REPOSITORY, as on every GitHub runner. It looks at the five newest
# successful runs on main, skipping this run, and gives up after that.
set -euo pipefail

workflow="${1:?usage: fetch-baseline.sh WORKFLOW_FILE ARTIFACT DEST_DIR}"
artifact="${2:?usage: fetch-baseline.sh WORKFLOW_FILE ARTIFACT DEST_DIR}"
dest="${3:?usage: fetch-baseline.sh WORKFLOW_FILE ARTIFACT DEST_DIR}"
repo="${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is not set}"
this_run="${GITHUB_RUN_ID:-0}"

mkdir -p "$dest"
runs="$(gh run list --repo "$repo" --workflow "$workflow" --branch main --status success \
  --limit 5 --json databaseId --jq '.[].databaseId' 2>/dev/null || true)"
for run in $runs; do
  [[ "$run" == "$this_run" ]] && continue
  if gh run download "$run" --repo "$repo" --name "$artifact" --dir "$dest" >/dev/null 2>&1; then
    echo "$run" >"$dest/run-id.txt"
    echo "Baseline: $artifact from run $run on main."
    exit 0
  fi
done
echo "No baseline: no successful run on main has the artifact $artifact."
