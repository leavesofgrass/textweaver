#!/usr/bin/env python3
"""Offline static accessibility check for the pages in docs/site/, and for
the built documentation site.

It reads each page's HTML as written (before any script runs) and reports:

- a missing lang on <html>, or a missing or empty <title>;
- not exactly one <h1>, and headings that skip a level;
- a first focusable element that is not the "Skip to main content" link to
  #main, or a missing <main id="main" tabindex="-1">;
- missing landmarks: header, nav with an aria-label, main, footer;
- inputs, selects, and textareas without a label;
- buttons without text or a label;
- svg elements that are neither hidden nor labelled with role="img",
  <title>, and <desc>;
- img elements without alt;
- duplicate ids, and aria-labelledby, aria-describedby, aria-controls,
  and label for= references that point at no id;
- tables without a caption or header cells;
- not exactly one polite live region;
- "outline: none" anywhere in the page;
- relative links to files that do not exist, in the HTML and in the page
  data (the JSON in <script id="site-data">).

Content that a page's script builds at run time is not seen here; check it
in a browser. Standard library only.

With `--built DIR` it checks the documentation site that
tools/build_site.py builds (target/docs-site/) instead. Every built page is
checked for:

- a lang on <html>, a title, exactly one <h1>, and no skipped heading level;
- a skip link as the first thing Tab reaches, pointing at an id on the page;
- landmarks: header, a labelled nav, main, footer;
- controls and buttons with a name (text, aria-label, or title), images with
  alt, and no duplicate ids;
- tables with a caption and header cells;
- scripts, styles, fonts, and images loaded from this site only, so reading
  the site makes no request to another host;
- relative links and #anchors that lead to a built page and an id on it.

The interactive pages in the built site/ folder also get the full docs/site/
checks above.

Usage:

    python tools/check_site_a11y.py
    python tools/check_site_a11y.py --built target/docs-site

Exit status 0 when every page passes, 1 when a problem was found.
Links to files that do not exist are problems too; `--allow-missing FILE`
(repeatable) lists files, relative to docs/, that are expected to appear
later and are reported as notes instead.
"""

from __future__ import annotations

import argparse
import re
import sys
from html.parser import HTMLParser
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SITE = ROOT / "docs" / "site"

FOCUSABLE = {"a", "button", "input", "select", "textarea", "summary"}
LABELLED_CONTROLS = {"input", "select", "textarea"}
VOID = {"area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr"}


class Page(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.stack: list[tuple[str, dict]] = []
        self.ids: dict[str, int] = {}
        self.refs: list[tuple[str, str, str]] = []  # (attr, id, tag)
        self.headings: list[int] = []
        self.first_focusable: tuple[str, dict] | None = None
        self.first_focusable_text = ""
        self.capture_first = False
        self.html_lang = None
        self.title = ""
        self.in_title = False
        self.landmarks: dict[str, list[dict]] = {"header": [], "nav": [], "main": [], "footer": []}
        self.controls: list[dict] = []
        self.label_for: set[str] = set()
        self.label_depth = 0
        self.buttons: list[dict] = []
        self.svgs: list[dict] = []
        self.imgs: list[dict] = []
        self.tables: list[dict] = []
        self.live_regions = 0
        self.links: list[str] = []
        self.in_script = False

    # --- helpers
    def _current(self, tag: str):
        for i in range(len(self.stack) - 1, -1, -1):
            if self.stack[i][0] == tag:
                return self.stack[i][1]
        return None

    def handle_starttag(self, tag, attrs):
        a = {k: (v if v is not None else "") for k, v in attrs}
        if tag == "script":
            self.in_script = True
        if tag == "html":
            self.html_lang = a.get("lang")
        if tag == "title":
            self.in_title = True
        if "id" in a:
            self.ids[a["id"]] = self.ids.get(a["id"], 0) + 1
        for attr in ("aria-labelledby", "aria-describedby", "aria-controls"):
            if attr in a:
                for ref in a[attr].split():
                    self.refs.append((attr, ref, tag))
        if re.fullmatch(r"h[1-6]", tag):
            self.headings.append(int(tag[1]))
        if tag in self.landmarks:
            self.landmarks[tag].append(a)
        if a.get("role") == "status" or a.get("aria-live") == "polite":
            self.live_regions += 1
        focusable = (
            (tag == "a" and "href" in a)
            or (tag in FOCUSABLE - {"a"} and a.get("type") != "hidden" and "disabled" not in a)
            or (a.get("tabindex") not in (None, "-1") and a.get("tabindex", "").lstrip("-").isdigit()
                and int(a["tabindex"]) >= 0)
        )
        if focusable and self.first_focusable is None:
            self.first_focusable = (tag, a)
            self.capture_first = True
        if tag == "label":
            self.label_depth += 1
            if "for" in a:
                self.label_for.add(a["for"])
                self.refs.append(("label for", a["for"], tag))
        if tag in LABELLED_CONTROLS and a.get("type") != "hidden":
            self.controls.append({"tag": tag, "attrs": a, "wrapped": self.label_depth > 0})
        if tag == "button":
            self.buttons.append({"attrs": a, "text": ""})
        if tag == "svg":
            self.svgs.append({"attrs": a, "title": False, "desc": False})
        if tag in ("title", "desc") and self._current("svg") is not None:
            self.svgs[-1][tag] = True
        if tag == "img":
            self.imgs.append(a)
        if tag == "table":
            self.tables.append({"attrs": a, "caption": False, "th": False})
        if tag == "caption" and self.tables:
            self.tables[-1]["caption"] = True
        if tag == "th" and self.tables:
            self.tables[-1]["th"] = True
        if tag == "a" and "href" in a:
            self.links.append(a["href"])
        if tag not in VOID:
            self.stack.append((tag, a))

    def handle_startendtag(self, tag, attrs):
        self.handle_starttag(tag, attrs)
        if tag not in VOID and self.stack and self.stack[-1][0] == tag:
            self.stack.pop()

    def handle_endtag(self, tag):
        if tag == "script":
            self.in_script = False
        if tag == "title":
            self.in_title = False
        if tag == "label":
            self.label_depth = max(0, self.label_depth - 1)
        if self.first_focusable is not None and self.first_focusable[0] == tag:
            self.capture_first = False
        for i in range(len(self.stack) - 1, -1, -1):
            if self.stack[i][0] == tag:
                del self.stack[i:]
                break

    def handle_data(self, data):
        if self.in_script:
            return
        if self.in_title and self._current("svg") is None:
            self.title += data
        if self.capture_first:
            self.first_focusable_text += data
        if self._current("button") is not None and self.buttons:
            self.buttons[-1]["text"] += data


def check_page(path: Path, allow_missing: set[str]) -> tuple[list[str], list[str]]:
    html = path.read_text(encoding="utf-8")
    p = Page()
    p.feed(html)
    problems: list[str] = []
    notes: list[str] = []

    if not p.html_lang:
        problems.append("<html> has no lang attribute.")
    if not p.title.strip():
        problems.append("The page has no <title>, or it is empty.")
    if p.headings.count(1) != 1:
        problems.append(f"The page has {p.headings.count(1)} h1 headings; it needs exactly one.")
    prev = 0
    for level in p.headings:
        if prev and level > prev + 1:
            problems.append(f"A heading skips from h{prev} to h{level}.")
        prev = level
    if p.headings and p.headings[0] != 1:
        problems.append(f"The first heading is h{p.headings[0]}, not h1.")

    ff = p.first_focusable
    if ff is None or ff[0] != "a" or ff[1].get("href") != "#main" or \
            p.first_focusable_text.strip() != "Skip to main content":
        problems.append('The first focusable element is not the "Skip to main content" link to #main.')
    mains = p.landmarks["main"]
    if len(mains) != 1 or mains[0].get("id") != "main" or mains[0].get("tabindex") != "-1":
        problems.append('The page needs exactly one <main id="main" tabindex="-1">.')
    for lm in ("header", "footer"):
        if not p.landmarks[lm]:
            problems.append(f"The page has no <{lm}>.")
    if not p.landmarks["nav"]:
        problems.append("The page has no <nav>.")
    for nav in p.landmarks["nav"]:
        if not (nav.get("aria-label") or nav.get("aria-labelledby")):
            problems.append("A <nav> has no aria-label.")

    for c in p.controls:
        a = c["attrs"]
        if not (c["wrapped"] or a.get("id") in p.label_for or a.get("aria-label") or a.get("aria-labelledby")):
            problems.append(f"A <{c['tag']}> (id {a.get('id', 'none')}) has no label.")
    for b in p.buttons:
        a = b["attrs"]
        if not (b["text"].strip() or a.get("aria-label") or a.get("aria-labelledby")):
            problems.append(f"A button (id {a.get('id', 'none')}) has no text or label.")
    for s in p.svgs:
        a = s["attrs"]
        if a.get("aria-hidden") == "true":
            continue
        if a.get("role") != "img" or not s["title"] or not s["desc"] or \
                not a.get("aria-labelledby") or not a.get("aria-describedby"):
            problems.append("An <svg> is not hidden and lacks role=img, <title>, <desc>, or its aria references.")
    for img in p.imgs:
        if "alt" not in img:
            problems.append(f"An <img> ({img.get('src', '?')}) has no alt.")
    for ident, n in p.ids.items():
        if n > 1:
            problems.append(f"The id {ident!r} is used {n} times.")
    for attr, ref, tag in p.refs:
        if ref not in p.ids:
            problems.append(f"{attr}={ref!r} on <{tag}> points at no element.")
    for t in p.tables:
        if not t["caption"]:
            problems.append("A <table> has no <caption>.")
        if not t["th"]:
            problems.append("A <table> has no header cells.")
    if p.live_regions != 1:
        problems.append(f"The page has {p.live_regions} polite live regions; it needs exactly one.")
    if re.search(r"outline\s*:\s*(none|0)\b", html):
        problems.append('The page uses "outline: none".')

    # Links in the page data (built into the page by its script at run time).
    data = re.search(r'<script type="application/json" id="site-data">(.*?)</script>', html, re.S)
    data_links = []
    if data:
        try:
            data_links = [h for h in re.findall(r'"href":\s*"([^"]+)"', data.group(1))]
        except re.error:
            data_links = []
    for href in sorted(set(p.links) | set(data_links)):
        if re.match(r"^[a-z][a-z0-9+.-]*:", href, re.I) or href.startswith("#"):
            continue
        target = href.split("#", 1)[0]
        if not target:
            continue
        resolved = (path.parent / target).resolve()
        if not resolved.exists():
            try:
                rel = resolved.relative_to((ROOT / "docs").resolve()).as_posix()
            except ValueError:
                rel = resolved.as_posix()
            if rel in allow_missing:
                notes.append(f"{href} does not exist yet (expected later).")
            else:
                problems.append(f"The link {href} points at a file that does not exist.")
    return problems, notes


# --- The built documentation site ---------------------------------------

# Elements the theme hides with CSS (display: none) before any script runs,
# so they never take focus: the check boxes behind its menus.
HIDDEN_TOGGLE = re.compile(r"\bmd-toggle\b")


class BuiltPage(Page):
    """The same parser, plus what the built-site checks need."""

    def __init__(self) -> None:
        super().__init__()
        self.resources: list[tuple[str, str]] = []  # (tag, url)
        self.named_controls: list[tuple[str, dict]] = []
        self.anchor_texts: list[str] = []

    def handle_starttag(self, tag, attrs):
        a = {k: (v if v is not None else "") for k, v in attrs}
        if tag == "input" and HIDDEN_TOGGLE.search(a.get("class", "")):
            # Not focusable and not a control a reader meets.
            if "id" in a:
                self.ids[a["id"]] = self.ids.get(a["id"], 0) + 1
            return
        if tag == "script" and a.get("src"):
            self.resources.append((tag, a["src"]))
        if tag == "link" and "stylesheet" in a.get("rel", "") and a.get("href"):
            self.resources.append((tag, a["href"]))
        if tag == "link" and "preconnect" in a.get("rel", ""):
            self.resources.append((tag, a.get("href", "")))
        if tag == "img" and a.get("src"):
            self.resources.append((tag, a["src"]))
        super().handle_starttag(tag, attrs)


def _is_external(url: str) -> bool:
    return bool(re.match(r"^(?:[a-z][a-z0-9+.-]*:)?//", url, re.I))


def _base_path() -> str:
    """The site's path on its host, from site_url in zensical.toml ("/textweaver/")."""
    try:
        import tomllib
        with open(ROOT / "zensical.toml", "rb") as f:
            url = tomllib.load(f)["project"].get("site_url", "")
    except (OSError, KeyError, ImportError, ValueError):
        return "/"
    m = re.match(r"^[a-z]+://[^/]+(/.*)?$", url, re.I)
    path = (m.group(1) if m else None) or "/"
    return path if path.endswith("/") else path + "/"


BASE_PATH = _base_path()


def _built_target(page: Path, href: str, root: Path) -> Path | None:
    """The file a link leads to in the built site, or None."""
    path_part = href.split("#", 1)[0].split("?", 1)[0]
    if not path_part:
        return page
    if path_part.startswith("/"):
        # Absolute on the host: only the site's own path is ours.
        if not path_part.startswith(BASE_PATH):
            return None
        target = (root / path_part[len(BASE_PATH):]).resolve()
    else:
        target = (page.parent / path_part).resolve()
    if target.is_dir():
        target = target / "index.html"
    return target if target.is_file() else None


_ids_cache: dict[Path, set[str]] = {}


def _ids_of(path: Path) -> set[str]:
    if path not in _ids_cache:
        p = BuiltPage()
        try:
            p.feed(path.read_text(encoding="utf-8"))
        except (OSError, UnicodeDecodeError):
            pass
        _ids_cache[path] = set(p.ids)
    return _ids_cache[path]


def check_built_page(path: Path, root: Path) -> list[str]:
    html = path.read_text(encoding="utf-8")
    p = BuiltPage()
    p.feed(html)
    problems: list[str] = []

    if not p.html_lang:
        problems.append("<html> has no lang attribute.")
    if not p.title.strip():
        problems.append("The page has no <title>, or it is empty.")
    if p.headings.count(1) != 1:
        problems.append(f"The page has {p.headings.count(1)} h1 headings; it needs exactly one.")
    prev = 0
    for level in p.headings:
        if prev and level > prev + 1:
            problems.append(f"A heading skips from h{prev} to h{level}.")
        prev = level

    ff = p.first_focusable
    if ff is None or ff[0] != "a" or not ff[1].get("href", "").startswith("#"):
        problems.append("The first focusable element is not a skip link.")
    else:
        target = ff[1]["href"][1:]
        if target and target not in p.ids:
            problems.append(f"The skip link points at #{target}, which is not on the page.")
    if len(p.landmarks["main"]) != 1:
        problems.append(f"The page has {len(p.landmarks['main'])} <main> elements; it needs one.")
    for lm in ("header", "nav", "footer"):
        if not p.landmarks[lm]:
            problems.append(f"The page has no <{lm}>.")
    if p.landmarks["nav"] and not any(n.get("aria-label") or n.get("aria-labelledby")
                                      for n in p.landmarks["nav"]):
        problems.append("No <nav> has an aria-label.")

    for c in p.controls:
        a = c["attrs"]
        if not (c["wrapped"] or a.get("id") in p.label_for or a.get("aria-label")
                or a.get("aria-labelledby") or a.get("title")):
            problems.append(f"A <{c['tag']}> (id {a.get('id', 'none')}) has no label.")
    for b in p.buttons:
        a = b["attrs"]
        if not (b["text"].strip() or a.get("aria-label") or a.get("aria-labelledby") or a.get("title")):
            problems.append(f"A button (class {a.get('class', 'none')}) has no text or label.")
    for img in p.imgs:
        if "alt" not in img:
            problems.append(f"An <img> ({img.get('src', '?')}) has no alt.")
    for ident, n in p.ids.items():
        if n > 1:
            problems.append(f"The id {ident!r} is used {n} times.")
    # tools/zensical_ext/textweaver_site_tables.py captions every table.
    for t in p.tables:
        if not t["caption"]:
            problems.append("A <table> has no <caption>.")
        if not t["th"]:
            problems.append("A <table> has no header cells.")

    for tag, url in p.resources:
        if _is_external(url):
            problems.append(f"A <{tag}> loads {url} from another host.")

    for href in sorted(set(p.links)):
        if not href or _is_external(href) or re.match(r"^[a-z][a-z0-9+.-]*:", href, re.I):
            continue
        target = _built_target(path, href, root)
        if target is None:
            problems.append(f"The link {href} leads to no page in the built site.")
            continue
        if "#" in href:
            frag = href.split("#", 1)[1]
            if frag and target.suffix == ".html" and frag not in _ids_of(target):
                problems.append(f"The link {href} leads to an anchor that is not on that page.")
    return problems


def check_built_site(root: Path) -> int:
    root = root.resolve()
    pages = sorted(root.rglob("*.html"))
    if not pages:
        print(f"No pages found in {root}.", file=sys.stderr)
        return 1
    interactive = {page.name for page in SITE.glob("*.html")}
    failed = 0
    passed = 0
    for page in pages:
        rel = page.relative_to(root).as_posix()
        if page.parent.name == "site" and page.parent.parent == root and page.name in interactive:
            problems, notes = check_page(page, set())
        else:
            problems, notes = check_built_page(page, root), []
        if problems:
            failed += 1
            print(f"{rel}: {len(problems)} problems.")
            for msg in problems:
                print(f"  - {msg}")
        else:
            passed += 1
        for msg in notes:
            print(f"  note ({rel}): {msg}")
    print(f"Built site: {passed} pages pass, {failed} have problems.")
    return 1 if failed else 0


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description="Check the docs/site/ pages for accessibility basics.")
    parser.add_argument("--allow-missing", action="append", default=[], metavar="FILE",
                        help="a file under docs/ that may be missing for now (repeatable)")
    parser.add_argument("--built", metavar="DIR",
                        help="check the built documentation site in DIR instead")
    args = parser.parse_args(argv)
    if args.built:
        return check_built_site(Path(args.built))
    allow = set(args.allow_missing)
    pages = sorted(SITE.glob("*.html"))
    if not pages:
        print("No pages found in docs/site/.", file=sys.stderr)
        return 1
    failed = 0
    for page in pages:
        problems, notes = check_page(page, allow)
        if problems:
            failed += 1
            print(f"docs/site/{page.name}: {len(problems)} problems.")
            for msg in problems:
                print(f"  - {msg}")
        else:
            print(f"docs/site/{page.name} passes.")
        for msg in notes:
            print(f"  note: {msg}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
