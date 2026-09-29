#!/usr/bin/env bash
# One Windows engine check for .github/workflows/engines.yml: picks a voice
# of KIND (sapi64, onecore, or sapi32) from the voices tw listed, reads the
# fixture into a WAV file with it, and checks the file with
# check_export.py. Lines go to $RUNNER_TEMP/out/summary.txt. When the
# runner has no voice of KIND, it says "Skipped:" with the reason and
# exits 0; it fails only when the export cannot be made.
#
#   tools/engines/windows-export.sh KIND LABEL
#
# Run from the repository's root in Git Bash on a Windows runner, after
# the workflow built tw and both SAPI hosts and wrote voices.json.
set -euo pipefail

kind="${1:?usage: windows-export.sh KIND LABEL}"
label="${2:?usage: windows-export.sh KIND LABEL}"
out="${RUNNER_TEMP:?}/out"
summary="$out/summary.txt"
doc="${DOC:-fixtures/t/reading.md}"

root="$(pwd -W)"
export TEXTWEAVER_SAPI_HOST="$root/target/debug/textweaver-sapi-host.exe"
export TEXTWEAVER_SAPI_HOST_X86="$root/target/i686-pc-windows-msvc/debug/textweaver-sapi-host.exe"

status=0
voice="$(python tools/engines/pick_voice.py "$kind" "$out/voices.json")" || status=$?
if [ "$status" -eq 3 ]; then
  # The runner image lacks this kind of voice: said plainly, not a failure.
  echo "$voice" | sed "s/^Skipped: /Skipped: $label: /" | tee -a "$summary"
  exit 0
elif [ "$status" -ne 0 ]; then
  echo "$voice" | sed "s/^Fail: /Fail: $label: /" | tee -a "$summary"
  exit 1
fi
echo "Measured: $label: voice $voice." | tee -a "$summary"

if ! target/debug/tw export-audio "$doc" --backend sapi --voice "$voice" \
       --out "$out/$kind.wav" --json --quiet --home "$RUNNER_TEMP/tw-home" > "$out/$kind.json"; then
  echo "Fail: $label: tw export-audio failed; see the log." | tee -a "$summary"
  exit 1
fi
python tools/engines/check_export.py --engine "$label" --backend sapi \
  --wav "$out/$kind.wav" --report "$out/$kind.json" --summary "$summary"
# For the comparison of voices after the last step.
sha256sum "$out/$kind.wav" | cut -d ' ' -f 1 > "$out/$kind.sha256"
