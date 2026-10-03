"""Read Ace by DAISY reports into the second-tool workflow's findings.

    python3 tools/ace/results.py REPORTS_DIR

REPORTS_DIR holds one folder per EPUB, named ace-FIXTURE, each with Ace's
report.json (`ace-cli --outdir`). Prints one line per finding, meaning
first, in the same form as the epubcheck and veraPDF findings:

    KIND: ace RULE, FIXTURE: MESSAGE

KIND is "Fail" for a critical or serious violation, "Warning" for a
moderate or minor one. The workflow's Results step reads these lines and
applies tools/second_tool_allowlist.txt. A folder with no report, or a
report with no assertions, is a Fail: Ace did not finish, which must never
pass as "no findings".

Standard library only; it runs on the runner's own Python.
"""

import glob
import json
import os
import sys

FAILING = {"critical", "serious"}


def failures(node):
    """Every failed assertion under node: (rule, impact, message)."""
    if isinstance(node, dict):
        result = node.get("earl:result")
        test = node.get("earl:test")
        if isinstance(result, dict) and isinstance(test, dict):
            if result.get("earl:outcome") == "fail":
                rule = test.get("dct:title") or "unknown-rule"
                impact = (test.get("earl:impact") or "unknown").lower()
                message = (
                    result.get("dct:description")
                    or test.get("dct:description")
                    or ""
                )
                yield rule, impact, " ".join(str(message).split())[:200]
        for value in node.values():
            yield from failures(value)
    elif isinstance(node, list):
        for value in node:
            yield from failures(value)


def report_lines(fixture, report):
    """The finding lines for one parsed report."""
    if not report.get("assertions"):
        return [f"Fail: ace report, {fixture}: the report has no assertions"]
    lines = []
    for rule, impact, message in failures(report):
        kind = "Fail" if impact in FAILING else "Warning"
        lines.append(f"{kind}: ace {rule}, {fixture}: {impact}: {message}")
    return lines


def main(argv):
    if len(argv) != 2:
        print("usage: python3 tools/ace/results.py REPORTS_DIR", file=sys.stderr)
        return 2
    out = []
    folders = sorted(p for p in glob.glob(os.path.join(argv[1], "ace-*")) if os.path.isdir(p))
    if not folders:
        out.append("Fail: ace report, *: no Ace report folder was written")
    for folder in folders:
        fixture = os.path.basename(folder)[len("ace-"):]
        path = os.path.join(folder, "report.json")
        try:
            with open(path, encoding="utf-8") as f:
                report = json.load(f)
        except (OSError, ValueError) as e:
            out.append(f"Fail: ace report, {fixture}: Ace did not finish ({e.__class__.__name__})")
            continue
        out.extend(report_lines(fixture, report))
    for line in out:
        print(line)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
