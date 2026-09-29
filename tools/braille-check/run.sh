#!/bin/sh
# The braille second-tool check: every fixture below, in UEB grade 1
# (textweaver's native translator) and grade 2 (through liblouis), written
# by braille-check and compared with liblouis by compare.py.
#
#   tools/braille-check/run.sh OUT_DIR
#
# Needs lou_translate on the PATH (install-liblouis.sh) and Rust. Run from
# the repository's root. Writes OUT_DIR/results.txt, one line per finding,
# each starting with Pass, Fail, Warning, Allowed, Measured, or Detail, and
# exits 1 when any fixture fails. GRADES chooses the grades (default "1 2").
set -eu

out="${1:?usage: run.sh OUT_DIR}"
grades="${GRADES:-1 2}"
mkdir -p "$out"
results="$out/results.txt"
: > "$results"

command -v lou_translate > /dev/null || {
  echo "Fail: lou_translate is not on the PATH; the check would verify nothing." | tee -a "$results"
  exit 1
}
lou_translate --version | head -n 1

# The root lock keeps every shared dependency at the workspace's version;
# cargo adds only braille-check's own entry.
cp Cargo.lock tools/braille-check/Cargo.lock
cargo build --release --manifest-path tools/braille-check/Cargo.toml
target_dir="${CARGO_TARGET_DIR:-tools/braille-check/target}"
tool="$target_dir/release/braille-check"

# Markdown fixtures without math: this build has no MathCAT, and math
# braille has its own tests (ADR-0036).
status=0
for src in fixtures/sample.md fixtures/t/reading.md fixtures/m/sample.md \
           fixtures/l/flavors/gfm.md fixtures/l/flavors/obsidian.md fixtures/l/flavors/pandoc.md \
           fixtures/g2/apa-paper.md; do
  name="$(echo "$src" | sed 's|^fixtures/||; s|/|-|g; s|\.md$||')"
  for grade in $grades; do
    if ! "$tool" "$grade" "$src" "$out/$name.g$grade.brf" "$out/$name.txt" > "$out/$name.g$grade.log" 2>&1; then
      echo "Fail: $name, grade $grade: braille-check failed: $(tail -n 1 "$out/$name.g$grade.log")" >> "$results"
      status=1
      continue
    fi
    python3 tools/braille-check/compare.py --grade "$grade" --fixture "$name" \
      --brf "$out/$name.g$grade.brf" --text "$out/$name.txt" \
      --writer-log "$out/$name.g$grade.log" --summary "$results" > /dev/null || status=1
  done
done

cat "$results"
if [ "$status" -eq 0 ]; then
  echo "Pass: every fixture reads back through liblouis within its limit."
else
  echo "Fail: at least one fixture did not; see the Fail lines above."
fi
exit "$status"
