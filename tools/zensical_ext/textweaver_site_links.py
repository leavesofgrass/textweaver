"""A Markdown extension for the textweaver documentation site (Zensical).

The guides in docs/ link to files outside docs/ (CONTRIBUTING.md,
CHANGELOG.md, scripts/README.md, fuzz/README.md, source folders) and to
folders inside docs/ that have no index page (dev/, history/). Those links
work on GitHub, but a site built from docs/ alone has nothing to serve at
them. This extension rewrites each such link to the same file or folder on
GitHub, so no link on the published site is broken, and the Markdown sources
stay unchanged.

It also sends links aimed at docs/site/README.md to site/about.html, because
docs/site/index.html (the interactive overview) must keep the site/ address.
tools/build_site.py moves the rendered README there after the build.

It runs before Zensical's own link rewriting, which then treats the GitHub
addresses as absolute and leaves them alone. Standard library and
Python-Markdown only.

Use it from zensical.toml:

    [project.markdown_extensions.textweaver_site_links]
    repo_blob = "https://github.com/leavesofgrass/textweaver/blob/main/"
    repo_tree = "https://github.com/leavesofgrass/textweaver/tree/main/"

tools/build_site.py puts this folder on PYTHONPATH.
"""

from __future__ import annotations

import posixpath
from pathlib import Path
from urllib.parse import urlparse

from markdown import Extension
from markdown.treeprocessors import Treeprocessor

INDEX_NAMES = ("index.md", "README.md")


def _context(md):
    """Return (page path relative to docs/, docs dir, repo root), or None."""
    try:
        from zensical.extensions.context import ContextPreprocessor
    except ImportError:  # not running under Zensical
        return None
    ctx = ContextPreprocessor.from_markdown(md)
    if ctx is None:
        return None
    config = ctx.config
    root = Path(config.get("root_dir") or ".").resolve()
    docs = Path(config.get("docs_dir") or "docs")
    docs = docs if docs.is_absolute() else (root / docs)
    docs = docs.resolve()
    page = Path(ctx.page.path)
    if page.is_absolute():
        try:
            page = page.resolve().relative_to(docs)
        except ValueError:
            return None
    return page.as_posix(), docs, root


class RepoLinksTreeprocessor(Treeprocessor):
    def __init__(self, md, blob: str, tree: str):
        super().__init__(md)
        self.blob = blob
        self.tree = tree

    def rewrite(self, value: str, page: str, docs: Path, root: Path) -> str | None:
        try:
            url = urlparse(value)
        except ValueError:
            return None
        if url.scheme or url.netloc or not url.path or url.path.startswith("/"):
            return None
        page_dir = posixpath.dirname(page)
        target_rel = posixpath.normpath(posixpath.join(page_dir, url.path))
        target = (docs / target_rel).resolve()
        frag = f"#{url.fragment}" if url.fragment else ""

        inside = target == docs or docs in target.parents
        if not inside:
            try:
                repo_rel = target.relative_to(root).as_posix()
            except ValueError:
                return None  # outside the repository: leave it to the checks
            base = self.tree if target.is_dir() else self.blob
            return base + repo_rel + frag

        rel = target.relative_to(docs).as_posix()
        if rel == "site/README.md":
            new = posixpath.relpath("site/about.html", page_dir or ".")
            return new + frag
        if target.is_dir() and not any((target / n).exists() for n in INDEX_NAMES):
            return self.tree + "docs/" + rel + frag
        return None

    def run(self, root_el):
        ctx = _context(self.md)
        if ctx is None:
            return
        page, docs, root = ctx
        for el in root_el.iter():
            for key in ("href", "src"):
                value = el.get(key)
                if not value:
                    continue
                new = self.rewrite(value, page, docs, root)
                if new is not None:
                    el.set(key, new)


class RepoLinksExtension(Extension):
    def __init__(self, **kwargs):
        self.config = {
            "repo_blob": ["", "Base URL for files on the repository host."],
            "repo_tree": ["", "Base URL for folders on the repository host."],
        }
        super().__init__(**kwargs)

    def extendMarkdown(self, md):
        md.registerExtension(self)
        proc = RepoLinksTreeprocessor(
            md, self.getConfig("repo_blob"), self.getConfig("repo_tree")
        )
        # Zensical's own link rewriting ("zrelpath") runs at priority 0;
        # higher priorities run first.
        md.treeprocessors.register(proc, "textweaver_site_links", 1)


def makeExtension(**kwargs):
    return RepoLinksExtension(**kwargs)
