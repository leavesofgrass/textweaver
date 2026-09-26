#!/usr/bin/env python3
"""Find broken relative links and anchors in textweaver's Markdown and HTML docs.

Usage:

    python tools/check_links.py            # check every doc, print problems
    python tools/check_links.py FILE...    # check only these files

What it checks:

- Every relative link in a Markdown file (``[text](target)``, ``[text]: target``
  reference definitions, and ``<a href>`` or ``<img src>`` in inline HTML) and
  in an HTML file under ``docs/site/`` (``href`` and ``src``).
- The target file or folder exists, relative to the file that links to it.
- An anchor (``file.md#heading`` or ``#heading``) names a heading in the
  target Markdown file (GitHub-style slug), or an ``id``/``name`` attribute in
  the target HTML file or in HTML inside the Markdown file.

Links with a scheme (``https:``, ``mailto:``, ``file:``) are not checked:
this tool never goes on the network. Links inside fenced code blocks and
inline code are ignored, since they are examples, not links.

The files checked are every ``*.md`` in the repository (except under
``target*``, ``third_party``, ``fixtures`` (test inputs), ``.git``, and
``node_modules``) and every
``*.html`` under ``docs/``. The exit status is 1 when any link is broken, so
``scripts/dev-check`` and CI can run it. Standard library only.
"""

from __future__ import annotations

import html
import html.parser
import os
import re
import sys
import unicodedata
from pathlib import Path
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parent.parent

SKIP_DIRS = {".git", "third_party", "node_modules", ".claude", "fixtures"}
SCHEME = re.compile(r"^[a-zA-Z][a-zA-Z0-9+.-]*:")
FENCE = re.compile(r"^\s{0,3}(`{3,}|~{3,})")
HEADING = re.compile(r"^\s{0,3}(#{1,6})\s+(.*?)\s*#*\s*$")
# [text](target "title") — target may be wrapped in <...>. Images too.
INLINE_LINK = re.compile(r"!?\[(?:[^\[\]]|\[[^\[\]]*\])*\]\(\s*(<[^>]*>|[^)\s]+)(?:\s+(?:\"[^\"]*\"|'[^']*'|\([^)]*\)))?\s*\)")
REF_DEF = re.compile(r"^\s{0,3}\[(?!\^)[^\]]+\]:\s*(<[^>]*>|\S+)")
HTML_ATTR = re.compile(r"""<(?:a|img|link|script)\b[^>]*?\b(?:href|src)\s*=\s*("[^"]*"|'[^']*')""", re.I)
HTML_ID = re.compile(r"""\b(?:id|name)\s*=\s*("[^"]*"|'[^']*')""", re.I)
INLINE_CODE = re.compile(r"(`+)(?:(?!\1).)+?\1")


def skipped(path: Path) -> bool:
    rel = path.relative_to(ROOT).parts
    return any(p in SKIP_DIRS or p.startswith("target") for p in rel[:-1])


def all_docs() -> list[Path]:
    files: list[Path] = []
    for dirpath, dirnames, filenames in os.walk(ROOT):
        dirnames[:] = [
            d for d in dirnames if d not in SKIP_DIRS and not d.startswith("target")
        ]
        for name in filenames:
            p = Path(dirpath) / name
            if name.endswith(".md"):
                files.append(p)
            elif name.endswith(".html") and "docs" in p.relative_to(ROOT).parts[:1]:
                files.append(p)
    return sorted(files)


def slugify(text: str) -> str:
    """GitHub's heading anchor: lowercase, drop punctuation, spaces to hyphens."""
    # Drop inline markup that GitHub drops: links keep their text, code keeps its text.
    text = re.sub(r"!?\[([^\]]*)\]\([^)]*\)", r"\1", text)
    text = re.sub(r"<[^>]+>", "", text)
    text = html.unescape(text)
    text = text.strip().lower()
    out = []
    for ch in text:
        cat = unicodedata.category(ch)
        if ch in "-_" or ch.isalnum():
            out.append(ch)
        elif ch == " ":
            out.append("-")
        elif cat.startswith("M"):
            out.append(ch)
        # everything else (punctuation, symbols) is dropped
    return "".join(out)


class _Ids(html.parser.HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.ids: set[str] = set()
        self.links: list[tuple[int, str]] = []

    def handle_starttag(self, tag, attrs):  # noqa: D401
        for k, v in attrs:
            if v is None:
                continue
            if k in ("id", "name"):
                self.ids.add(v)
            if k in ("href", "src") and tag in ("a", "img", "link", "script", "area", "source"):
                self.links.append((self.getpos()[0], v))


_anchor_cache: dict[Path, set[str]] = {}


def markdown_lines(path: Path) -> list[tuple[int, str]]:
    """Lines outside fenced code blocks, with inline code removed."""
    out: list[tuple[int, str]] = []
    fence: str | None = None
    for n, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        m = FENCE.match(line)
        if fence is None and m:
            fence = m.group(1)[0] * len(m.group(1))
            continue
        if fence is not None:
            if m and m.group(1).startswith(fence):
                fence = None
            continue
        out.append((n, INLINE_CODE.sub("", line)))
    return out


def anchors(path: Path) -> set[str]:
    if path in _anchor_cache:
        return _anchor_cache[path]
    found: set[str] = set()
    if path.suffix == ".md":
        counts: dict[str, int] = {}
        for _, line in markdown_lines(path):
            m = HEADING.match(line)
            if m:
                base = slugify(m.group(2))
                n = counts.get(base, 0)
                counts[base] = n + 1
                found.add(base if n == 0 else f"{base}-{n}")
            for idm in HTML_ID.finditer(line):
                found.add(idm.group(1)[1:-1])
    elif path.suffix in (".html", ".htm"):
        p = _Ids()
        p.feed(path.read_text(encoding="utf-8"))
        found = p.ids
    _anchor_cache[path] = found
    return found


def links_in(path: Path) -> list[tuple[int, str]]:
    if path.suffix == ".md":
        found: list[tuple[int, str]] = []
        for n, line in markdown_lines(path):
            for m in INLINE_LINK.finditer(line):
                found.append((n, m.group(1)))
            m = REF_DEF.match(line)
            if m:
                found.append((n, m.group(1)))
            for m in HTML_ATTR.finditer(line):
                found.append((n, m.group(1)[1:-1]))
        return found
    p = _Ids()
    p.feed(path.read_text(encoding="utf-8"))
    return p.links


def check_file(path: Path) -> list[str]:
    problems: list[str] = []
    rel = path.relative_to(ROOT).as_posix()
    for line, raw in links_in(path):
        target = raw.strip()
        if target.startswith("<") and target.endswith(">"):
            target = target[1:-1]
        if not target or SCHEME.match(target) or target.startswith("//"):
            continue
        file_part, _, anchor = target.partition("#")
        file_part = unquote(file_part.split("?", 1)[0])
        anchor = unquote(anchor)
        if file_part:
            dest = (path.parent / file_part).resolve()
            try:
                dest.relative_to(ROOT)
            except ValueError:
                problems.append(f"{rel}:{line}: {raw} points outside the repository.")
                continue
            if not dest.exists():
                problems.append(f"{rel}:{line}: {raw}: {file_part} does not exist.")
                continue
        else:
            dest = path
        if anchor and dest.is_file() and dest.suffix in (".md", ".html", ".htm"):
            if anchor not in anchors(dest):
                where = dest.relative_to(ROOT).as_posix()
                problems.append(f"{rel}:{line}: {raw}: no heading or id #{anchor} in {where}.")
    return problems


def main(argv: list[str]) -> int:
    files = [Path(a).resolve() for a in argv] if argv else all_docs()
    problems: list[str] = []
    for f in files:
        if not f.exists():
            problems.append(f"{f}: file not found.")
            continue
        problems.extend(check_file(f))
    for p in problems:
        print(p)
    count = len(files)
    noun = "file" if count == 1 else "files"
    if problems:
        print(f"{len(problems)} broken link{'s' if len(problems) != 1 else ''} in {count} {noun}.")
        return 1
    print(f"All links resolve in {count} {noun}.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
