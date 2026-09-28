#!/usr/bin/env python3
"""Normalize an accessibility-cli tree dump, and compare two of them.

The GUI workflow dumps the window's accessibility tree on Windows, macOS,
and Linux with accessibility-cli (ADR-0039). The raw JSON carries element
ids, bounds, and the process id, which change on every run; `normalize`
keeps what a screen reader reads (role, name, value, description, states,
actions) as one indented line per element, so two runs of the same code
give the same text. `compare` writes a Markdown summary of the difference
from the baseline (main's last run), meaning first on every line, in
words: "Pass", "Changed", "No baseline", "Fail".

    python3 tools/a11y/tree_report.py normalize raw.json tree.txt
    python3 tools/a11y/tree_report.py compare --system Linux \\
        --current tree.txt --baseline baseline/tree.txt --baseline-run 123 \\
        --summary summary.md

`compare` exits 0 whether or not the tree changed: the diff is for a
person to read. It exits 1 only when the current tree is missing or empty.
Standard library only, so it runs on every runner without installing
anything.
"""

import argparse
import difflib
import json
import sys

# Keys that name what a screen reader reads, in the order they are printed.
NAME_KEYS = ("title", "name", "label")
TEXT_KEYS = ("value", "description", "help", "role_description", "identifier", "url")
# Everything else (ids, bounds, the process id) is left out: it changes
# from run to run, or says where an element is rather than what it is.
MAX_TEXT = 80
MAX_DIFF_LINES = 60


def key_of(value):
    """A hashable form of an element id, whatever shape it has in JSON."""
    return json.dumps(value, sort_keys=True)


def is_node(d):
    return isinstance(d, dict) and "role" in d


def clip(text):
    text = " ".join(str(text).split())
    return text if len(text) <= MAX_TEXT else text[: MAX_TEXT - 3] + "..."


def describe(node):
    """One line for one element: role, then name, then the rest."""
    parts = [str(node.get("role", "?"))]
    for k in NAME_KEYS:
        if node.get(k):
            parts.append(repr(clip(node[k])))
            break
    for k in TEXT_KEYS:
        v = node.get(k)
        if v not in (None, "", [], {}):
            parts.append(f"{k}={clip(v)!r}")
    flags = []
    if node.get("enabled") is False:
        flags.append("disabled")
    if node.get("focused") is True:
        flags.append("focused")
    for k in ("states", "state"):
        if isinstance(node.get(k), list):
            flags += [str(s) for s in node[k]]
    if flags:
        parts.append("[" + ", ".join(sorted(set(flags))) + "]")
    actions = node.get("actions")
    if isinstance(actions, list) and actions:
        parts.append("actions: " + ", ".join(sorted(str(a) for a in actions)))
    return " ".join(parts)


def collect(obj, found):
    """Every element dict anywhere in the JSON."""
    if is_node(obj):
        found.append(obj)
    if isinstance(obj, dict):
        for v in obj.values():
            collect(v, found)
    elif isinstance(obj, list):
        for v in obj:
            collect(v, found)


def child_lists(node):
    """The lists in an element that hold its children (dicts or ids)."""
    for k in ("children", "child_ids", "childIds", "kids"):
        v = node.get(k)
        if isinstance(v, list):
            yield v


def normalize(data):
    """The tree as indented lines, and a note when it could not be nested."""
    nodes = []
    collect(data, nodes)
    if not nodes:
        return [], "no elements found in the JSON"
    by_id = {}
    for n in nodes:
        for k in ("id", "key", "element_id"):
            if k in n:
                by_id[key_of(n[k])] = n
                break

    def kids(n):
        out = []
        for lst in child_lists(n):
            for c in lst:
                if is_node(c):
                    out.append(c)
                else:
                    target = by_id.get(key_of(c))
                    if target is not None:
                        out.append(target)
        return out

    referenced = set()
    for n in nodes:
        for c in kids(n):
            referenced.add(id(c))
    roots = [n for n in nodes if id(n) not in referenced]
    lines, seen = [], set()

    def walk(n, depth):
        if id(n) in seen or depth > 60:
            return
        seen.add(id(n))
        lines.append("  " * depth + "- " + describe(n))
        for c in kids(n):
            walk(c, depth + 1)

    for r in roots:
        walk(r, 0)
    note = ""
    if len(roots) == len(nodes) and len(nodes) > 1:
        # Nothing nested: keep a stable order so the diff still works.
        lines = sorted(lines)
        note = "the elements came out flat (no children found); the list is sorted"
    return lines, note


def load_json(path):
    """The JSON in a file, skipping any banner lines printed before it."""
    with open(path, encoding="utf-8", errors="replace") as f:
        text = f.read()
    start = min((i for i in (text.find("{"), text.find("[")) if i >= 0), default=-1)
    if start < 0:
        raise ValueError("no JSON in the dump")
    return json.JSONDecoder().raw_decode(text[start:])[0]


def cmd_normalize(args):
    try:
        data = load_json(args.raw)
    except (OSError, ValueError) as e:
        print(f"Fail: could not read the tree dump {args.raw}: {e}")
        return 1
    lines, note = normalize(data)
    with open(args.out, "w", encoding="utf-8", newline="\n") as f:
        if note:
            f.write(f"# Note: {note}\n")
        f.write("\n".join(lines) + "\n")
    print(f"{len(lines)} elements written to {args.out}" + (f" ({note})" if note else ""))
    return 0 if lines else 1


def read_lines(path):
    try:
        with open(path, encoding="utf-8") as f:
            return [line.rstrip("\n") for line in f]
    except OSError:
        return None


def cmd_compare(args):
    out = [f"## Accessibility tree, {args.system}", ""]
    current = read_lines(args.current)
    status = 0
    if not current or not any(line.strip() for line in current):
        out.append(f"Fail: no tree was dumped on {args.system}. The raw dump and the GUI log are in the artifact.")
        status = 1
    else:
        count = sum(1 for line in current if line.lstrip().startswith("- "))
        base = read_lines(args.baseline) if args.baseline else None
        where = f"main's last run ({args.baseline_run})" if args.baseline_run else "main's last run"
        if base is None:
            out.append(f"No baseline: {count} elements dumped; no earlier tree from main to compare with.")
        elif base == current:
            out.append(f"Pass: {count} elements, the same as {where}.")
        else:
            diff = [d for d in difflib.unified_diff(base, current, lineterm="", n=0)
                    if not d.startswith(("---", "+++", "@@"))]
            added = [d[1:] for d in diff if d.startswith("+")]
            removed = [d[1:] for d in diff if d.startswith("-")]
            def lines_word(n):
                return f"{n} line" if n == 1 else f"{n} lines"

            out.append(f"Changed: {lines_word(len(added))} added and {lines_word(len(removed))} removed, against {where}. "
                       "Check that each change was meant.")
            out.append("")
            shown = 0
            for d in diff:
                if shown >= MAX_DIFF_LINES:
                    out.append(f"- And {len(diff) - shown} more lines: see the artifact.")
                    break
                word = "Added" if d.startswith("+") else "Removed"
                out.append(f"- {word}: `{d[1:].strip()}`")
                shown += 1
    out.append("")
    text = "\n".join(out) + "\n"
    print(text)
    if args.summary:
        with open(args.summary, "a", encoding="utf-8") as f:
            f.write(text)
    return status


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    n = sub.add_parser("normalize", help="raw accessibility-cli JSON to stable text")
    n.add_argument("raw")
    n.add_argument("out")
    c = sub.add_parser("compare", help="Markdown summary of the change from a baseline")
    c.add_argument("--system", required=True)
    c.add_argument("--current", required=True)
    c.add_argument("--baseline")
    c.add_argument("--baseline-run")
    c.add_argument("--summary")
    args = ap.parse_args()
    return cmd_normalize(args) if args.cmd == "normalize" else cmd_compare(args)


if __name__ == "__main__":
    sys.exit(main())
