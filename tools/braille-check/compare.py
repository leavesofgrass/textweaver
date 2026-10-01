#!/usr/bin/env python3
"""Checks a BRF file from braille-check against liblouis, the second tool.

For each fixture, braille-check wrote the BRF and the document's text. This
script:

- reads the BRF back to print with liblouis (`lou_translate --backward`,
  display table en-us-brf.dis and the UEB table for the grade) and compares
  its words with the document's words, in order. Words the writer adds on
  purpose (an ordered list's numbers, "checked" and "not checked" for task
  items) are allowed and left out of the count;
- does the same round trip with liblouis alone (its forward translation of
  the text, read back), as the baseline: back-translation is ambiguous,
  above all in grade 2, where a lone "m" reads back as "more". The fixture
  fails when textweaver's file reads back more than 2 points worse than
  the baseline, or below 90 percent;
- measures how many braille words match liblouis's own forward translation
  of the same text, and shows where they differ. For grade 1 that compares
  textweaver's native translator with liblouis; for grade 2 textweaver
  calls liblouis, so it checks the layout around it. This is reported, not
  failed.

Every line it prints starts with a word: Pass, Fail, Warning, Allowed,
Measured, or Detail. Nothing here reads aloud or needs a display. Standard
library only.

Usage:
    compare.py --grade 1 --brf F.brf --text F.txt --fixture NAME
               [--writer-log F.log] [--lou lou_translate] [--summary FILE]
"""

from __future__ import annotations

import argparse
import difflib
import re
import subprocess
import sys
import unicodedata

TABLES = {1: "en-ueb-g1.ctb", 2: "en-ueb-g2.ctb"}
# How far below liblouis's own round trip a file may read back, and the
# floor under any baseline.
MARGIN = 0.02
FLOOR = 0.90
# Words the writer adds that are not in the document's text.
ADDED = ({"checked"}, {"not", "checked"})
# A print page indicator: a line of dashes, then the print page number.
PRINT_PAGE = re.compile(r"^-{3,}\S*$")


def brf_lines(data: str) -> list[str]:
    """The BRF's text lines, with divided words joined again.

    Pages break on form feeds and lines on CR LF. Print page indicator
    lines are layout, not text, and are left out. A line ending in dot 5
    (`"`, the line continuation indicator) continues on the next line
    without a space.
    """
    lines: list[str] = []
    joining = False
    for page in data.split("\f"):
        for raw in page.replace("\r\n", "\n").split("\n"):
            line = raw.rstrip()
            if not line.strip() or PRINT_PAGE.match(line.strip()):
                continue
            text = line.strip()
            continues = text.endswith('"') and len(text) > 1
            if continues:
                text = text[:-1]
            if joining and lines:
                lines[-1] += text
            else:
                lines.append(text)
            joining = continues
    return lines


def louis(lou: str, direction: str, table: str, lines: list[str]) -> list[str]:
    """Runs lou_translate over `lines`, one output line per input line.

    lou_translate reads backslash escapes in its input, so each backslash
    is doubled. In BRF a backslash is the cell for "ou" (dots 1256); sent
    as it is, "AL\\D1" (aloud) was an invalid escape and its whole line
    read back as nothing, and "COL\\R$" (coloured) read "\\r" as a
    carriage return.
    """
    if not lines:
        return []
    done = subprocess.run(
        [lou, direction, f"en-us-brf.dis,{table}"],
        input="\n".join(line.replace("\\", "\\\\") for line in lines) + "\n",
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=False,
    )
    if done.returncode != 0:
        raise RuntimeError(f"lou_translate {direction} failed: {done.stderr.strip()[:300]}")
    return done.stdout.splitlines()


def words(text: str) -> list[str]:
    """Lowercase words of letters and digits, accents removed."""
    plain = unicodedata.normalize("NFKD", text)
    plain = "".join(c for c in plain if not unicodedata.combining(c))
    return re.findall(r"[0-9a-z]+", plain.casefold())


def ratio(a: list[str], b: list[str]) -> float:
    return difflib.SequenceMatcher(None, a, b, autojunk=False).ratio()


def drop_added(print_words: list[str], back: list[str]) -> tuple[list[str], list[str]]:
    """`back` without the words the writer adds on purpose, and those words.

    An insertion (print has nothing there) that is all numbers, or is
    "checked" or "not checked", is the writer's own: list numbers and task
    states.
    """
    kept, added = [], []
    matcher = difflib.SequenceMatcher(None, print_words, back, autojunk=False)
    for tag, _i1, _i2, j1, j2 in matcher.get_opcodes():
        span = back[j1:j2]
        if tag == "insert" and (all(w.isdigit() for w in span) or set(span) in ADDED):
            added.append(" ".join(span))
        else:
            kept.extend(span)
    return kept, added


def differences(a: list[str], b: list[str], what: str, limit: int = 8) -> list[str]:
    """Up to `limit` places where the word lists differ, in words."""
    out = []
    matcher = difflib.SequenceMatcher(None, a, b, autojunk=False)
    for tag, i1, i2, j1, j2 in matcher.get_opcodes():
        if tag == "equal":
            continue
        before = " ".join(a[max(0, i1 - 3):i1]) or "the start"
        ours = " ".join(a[i1:i2]) or "nothing"
        theirs = " ".join(b[j1:j2]) or "nothing"
        out.append(f'Detail: {what}, after "{before}": "{ours}" against "{theirs}".')
        if len(out) == limit:
            break
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--grade", type=int, choices=(1, 2), required=True)
    ap.add_argument("--brf", required=True)
    ap.add_argument("--text", required=True)
    ap.add_argument("--fixture", required=True)
    ap.add_argument("--writer-log", help="braille-check's output, for its warnings")
    ap.add_argument("--lou", default="lou_translate")
    ap.add_argument("--summary", help="append the lines to this file too")
    args = ap.parse_args()

    grade, table = args.grade, TABLES[args.grade]
    name = f"{args.fixture}, grade {grade}"
    lines: list[str] = []
    failed = False

    if args.writer_log:
        with open(args.writer_log, encoding="utf-8", errors="replace") as f:
            for w in f:
                w = w.strip()
                if not w.startswith("Warning:"):
                    continue
                # Grade 2 that fell back to grade 1 means liblouis was not
                # used: the check would verify nothing.
                if grade == 2 and "uncontracted" in w:
                    lines.append(f"Fail: {name}: the writer did not use liblouis. {w}")
                    failed = True
                else:
                    lines.append(f"Warning: {name}: {w[len('Warning:'):].strip()}")

    with open(args.brf, encoding="ascii", errors="replace") as f:
        brf = brf_lines(f.read())
    with open(args.text, encoding="utf-8") as f:
        text = f.read()
    print_words = words(text)

    try:
        back = louis(args.lou, "--backward", table, [line.lower() for line in brf])
        # Tabs (table cells) become spaces, as the writer's linear tables
        # separate cells.
        paragraphs = [" ".join(p.split()) for p in text.splitlines() if p.strip()]
        forward = louis(args.lou, "--forward", table, paragraphs)
        own_back = louis(args.lou, "--backward", table, [line.lower() for line in forward])
    except (OSError, RuntimeError) as e:
        lines.append(f"Fail: {name}: {e}")
        return report(lines, args.summary, True)

    back_words, added = drop_added(print_words, words(" ".join(back)))
    for a in added:
        lines.append(f'Allowed: {name}: "{a}" is the writer\'s own (a list number or a task state).')
    agreement = ratio(print_words, back_words)
    baseline = ratio(print_words, words(" ".join(own_back)))
    limit = max(FLOOR, baseline - MARGIN)
    said = (
        f"liblouis reads back {agreement * 100:.1f} percent of the {len(print_words)} words "
        f"in order; its own translation reads back {baseline * 100:.1f} percent"
    )
    if not print_words:
        lines.append(f"Fail: {name}: the document has no words to compare.")
        failed = True
    elif agreement >= limit:
        lines.append(f"Pass: {name}: {said}.")
    else:
        lines.append(f"Fail: {name}: {said}; the limit is {limit * 100:.1f} percent.")
        failed = True
    lines.extend(differences(print_words, back_words, "print against read back"))

    ours = " ".join(brf).upper().split()
    theirs = " ".join(forward).upper().split()
    lines.append(
        f"Measured: {name}: {ratio(ours, theirs) * 100:.1f} percent of braille words match "
        f"liblouis's own translation ({len(ours)} words written, {len(theirs)} from liblouis)."
    )
    lines.extend(differences(ours, theirs, "textweaver's braille against liblouis's"))
    return report(lines, args.summary, failed)


def report(lines: list[str], summary: str | None, failed: bool) -> int:
    for line in lines:
        print(line)
    if summary:
        with open(summary, "a", encoding="utf-8") as s:
            for line in lines:
                s.write(line + "\n")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
