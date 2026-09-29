"""A Markdown extension for the textweaver documentation site (Zensical):
every table gets a caption.

Markdown tables have header cells but no caption, so a screen reader that
lists or jumps between tables (NVDA's T key, JAWS's T and Ctrl+Insert+T)
can only say "table with 4 rows". This extension gives each table a
<caption> made from the heading it sits under: "Moving by word", or
"Moving by word, table 2" when a heading has more than one table. The
caption is visually hidden (the heading is already on screen just above),
so the page looks the same.

A table that already has a caption (written in HTML) keeps it. Standard
library and Python-Markdown only.

Use it from zensical.toml:

    [project.markdown_extensions]
    textweaver_site_tables = {}

tools/build_site.py puts this folder on PYTHONPATH, and
tools/check_site_a11y.py --built checks that every table in the built site
has a caption and header cells.
"""

from __future__ import annotations

import re
import xml.etree.ElementTree as etree

from markdown import Extension
from markdown.treeprocessors import Treeprocessor

HEADINGS = {"h1", "h2", "h3", "h4", "h5", "h6"}
# Placeholders Python-Markdown leaves for stashed HTML.
STASH = re.compile("\x02[^\x03]*\x03")
# The class the site's stylesheet hides visually (docs/assets/textweaver-site.css).
HIDDEN = "tw-visually-hidden"


def heading_text(el: etree.Element) -> str:
    """The heading's words, without a permalink the toc extension added."""
    parts: list[str] = []

    def walk(node: etree.Element) -> None:
        if "headerlink" in (node.get("class") or "").split():
            return
        if node.text:
            parts.append(node.text)
        for child in node:
            walk(child)
            if child.tail:
                parts.append(child.tail)

    walk(el)
    return " ".join(STASH.sub("", "".join(parts)).split())


def caption_for(heading: str, number: int) -> str:
    """The caption for the number-th table under a heading (1-based)."""
    base = heading or "Table"
    return base if number == 1 else f"{base}, table {number}"


class TableCaptionsTreeprocessor(Treeprocessor):
    def run(self, root: etree.Element) -> None:
        # Document order: a heading, then the tables under it.
        heading = ""
        counts: dict[str, int] = {}
        # A snapshot, since captions are inserted while walking.
        for el in list(root.iter()):
            if el.tag in HEADINGS:
                heading = heading_text(el)
                continue
            if el.tag != "table":
                continue
            counts[heading] = counts.get(heading, 0) + 1
            if el.find("caption") is not None:
                continue
            caption = etree.Element("caption", {"class": HIDDEN})
            caption.text = caption_for(heading, counts[heading])
            el.insert(0, caption)


class TableCaptionsExtension(Extension):
    def extendMarkdown(self, md):
        md.registerExtension(self)
        # After inline processing (20), so headings hold their final text.
        md.treeprocessors.register(TableCaptionsTreeprocessor(md), "textweaver_site_tables", 2)


def makeExtension(**kwargs):
    return TableCaptionsExtension(**kwargs)
