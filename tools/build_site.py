#!/usr/bin/env python3
"""Build the textweaver documentation site with Zensical, then check it.

Steps:

1. Run `zensical build --clean --strict` with zensical.toml, with
   tools/zensical_ext on PYTHONPATH so the textweaver_site_links extension
   loads. The site goes to target/docs-site/.
2. Place the interactive pages. docs/site/README.md renders to
   site/index.html, which is the interactive overview's own address, so the
   rendered README moves to site/about.html and the overview is copied back
   over site/index.html.
3. In the built copies of the interactive pages only (never in docs/site/),
   point their links to Markdown guides at the built guide pages, and their
   links outside docs/ at GitHub.
4. Check every built page with tools/check_site_a11y.py's site checks.

Usage:

    py -3 tools/build_site.py            (Windows)
    python3 tools/build_site.py          (Linux, macOS)
    python3 tools/build_site.py --no-check

Zensical must be installed first, at the version pinned in
tools/site-requirements.txt. Set ZENSICAL to the zensical program if it is
not on PATH. Exit status 0 when the build and the checks pass.

To look at the result, serve the folder and open http://127.0.0.1:8000/:

    python3 -m http.server 8000 --bind 127.0.0.1 --directory target/docs-site

The checks read the HTML as built, before any script runs. The theme's
scripts, and docs/assets/textweaver-site.js, which fixes names, states, and
announcements the theme leaves out, need a browser. Repeat these by hand
(and with NVDA or JAWS) whenever the Zensical pin changes:

- Tab from the top: the skip link comes first, then the site name, the
  color theme switch, Search, and the repository link.
- Search: Enter on the Search button moves focus to a box named "Search the
  documentation"; typing announces the result count; Tab reaches the
  filter button and then each result as a link; Escape closes the panel and
  returns focus to the Search button; while closed, nothing in it can be
  reached with Tab.
- The color theme switch: Enter moves through system, light, and dark, and
  each choice says its name.
- The navigation: sections that open and close are buttons with an expanded
  or collapsed state; the current page is marked as the current page; at a
  narrow width (or 200 percent zoom), the Navigation button opens the menu.
- Contrast: every text at 4.5:1 or better in the light and dark schemes,
  also while focused.
"""

from __future__ import annotations

import argparse
import os
import posixpath
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DOCS = ROOT / "docs"
OUT = ROOT / "target" / "docs-site"
EXT = ROOT / "tools" / "zensical_ext"
REPO_BLOB = "https://github.com/leavesofgrass/textweaver/blob/main/"
REPO_TREE = "https://github.com/leavesofgrass/textweaver/tree/main/"

HREF = re.compile(r'(?P<pre>(?:href=|"href":\s*)")(?P<url>[^"]*)(?P<post>")')


def zensical_program() -> list[str]:
    env = os.environ.get("ZENSICAL")
    if env:
        return [env]
    found = shutil.which("zensical")
    if found:
        return [found]
    # The same Python that runs this script may have it installed.
    return [sys.executable, "-m", "zensical"]


def run_zensical() -> int:
    env = dict(os.environ)
    env["PYTHONPATH"] = os.pathsep.join(filter(None, [str(EXT), env.get("PYTHONPATH")]))
    env.setdefault("NO_COLOR", "1")
    cmd = zensical_program() + ["build", "--clean", "--strict", "-f", str(ROOT / "zensical.toml")]
    print("Running: " + " ".join(cmd[-5:]))
    return subprocess.run(cmd, cwd=ROOT, env=env).returncode


def built_url(target_rel: str) -> str:
    """The built address, relative to the site root, of a file under docs/."""
    if target_rel == "README.md":
        return ""
    if target_rel == "site/README.md":
        return "site/about.html"
    if target_rel.endswith("/README.md") or target_rel.endswith("/index.md"):
        return target_rel.rsplit("/", 1)[0] + "/"
    if target_rel.endswith(".md"):
        return target_rel[:-3] + "/"
    return target_rel


def rewrite_interactive_page(path: Path) -> int:
    """Point links in a built interactive page at built pages. Returns the count."""
    text = path.read_text(encoding="utf-8")
    page_dir = "site"
    changed = 0

    def fix(m: re.Match) -> str:
        nonlocal changed
        url = m.group("url")
        if not url or url.startswith("#") or re.match(r"^[a-z][a-z0-9+.-]*:", url, re.I):
            return m.group(0)
        path_part, _, frag = url.partition("#")
        frag = "#" + frag if frag else ""
        if not path_part:
            return m.group(0)
        target = posixpath.normpath(posixpath.join(page_dir, path_part))
        if target.startswith("../"):
            repo_rel = target[3:]
            base = REPO_TREE if (ROOT / repo_rel).is_dir() else REPO_BLOB
            new = base + repo_rel + frag
        else:
            built = built_url(target)
            if built == target:  # a file that is served as it is
                return m.group(0)
            new = posixpath.relpath(built or ".", page_dir)
            if built.endswith("/") or built == "":
                new = new.rstrip("/") + "/"
            new += frag
        if new == url:
            return m.group(0)
        changed += 1
        return m.group("pre") + new + m.group("post")

    text = HREF.sub(fix, text)
    path.write_text(text, encoding="utf-8")
    return changed


def place_interactive_pages() -> None:
    site_out = OUT / "site"
    rendered_readme = site_out / "index.html"
    if not rendered_readme.exists():
        raise SystemExit("The build has no site/index.html; did docs/site/ move?")
    shutil.move(rendered_readme, site_out / "about.html")
    for page in sorted((DOCS / "site").glob("*.html")):
        dest = site_out / page.name
        shutil.copyfile(page, dest)
        n = rewrite_interactive_page(dest)
        print(f"Placed site/{page.name}, {n} links pointed at the built site.")


def fix_not_found_page() -> None:
    """The theme's 404 page has a skip link to #__skip but no such id."""
    page = OUT / "404.html"
    if not page.exists():
        return
    text = page.read_text(encoding="utf-8")
    m = re.search(r'<a href="#([^"]+)" class="md-skip"', text)
    if m and f'id="{m.group(1)}"' not in text:
        text, n = re.subn(r"<h1>", f'<h1 id="{m.group(1)}">', text, count=1)
        if n:
            page.write_text(text, encoding="utf-8")
            print("Gave the 404 page's heading the skip link's target.")


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description="Build and check the documentation site.")
    parser.add_argument("--no-check", action="store_true", help="skip the accessibility check")
    args = parser.parse_args(argv)
    # Keep this script's lines in order with Zensical's own output.
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(line_buffering=True)

    code = run_zensical()
    if code != 0:
        print(f"Zensical failed with status {code}.", file=sys.stderr)
        return code
    place_interactive_pages()
    fix_not_found_page()
    if args.no_check:
        print(f"Built the site in {OUT.relative_to(ROOT).as_posix()}/. Checks skipped.")
        return 0
    check = subprocess.run([sys.executable, str(ROOT / "tools" / "check_site_a11y.py"),
                            "--built", str(OUT)], cwd=ROOT)
    if check.returncode == 0:
        print(f"Built and checked the site in {OUT.relative_to(ROOT).as_posix()}/.")
    return check.returncode


if __name__ == "__main__":
    sys.exit(main())
