"""One-off parity export: run Star's own document pipeline over textweaver's
fixtures and dump the canonical text, word tokens, sentence starts, paragraph
starts and speak-time normalization as JSON.

Usage (from anywhere; Python 3.11, no third-party packages needed):

    python D:\\textweaver\\tools\\star_parity_export.py

Reads   D:\\textweaver\\fixtures\\sample.{txt,md,html}
Writes  D:\\textweaver\\fixtures\\star-parity\\<fixture-name>.json

Star is imported read-only from D:\\star (sys.path insert).  Nothing in D:\\star is
modified.  Star's user settings file is NOT read (SETTINGS_FILE is redirected to
a path that does not exist, so every setting is Star's built-in default) and the
document cache is disabled so Star never reads or writes its cache directory.

All offsets are Python ``str`` indices, i.e. Unicode code points, end-exclusive.
Parallel ``*_utf8`` arrays give the same offsets as UTF-8 byte offsets for Rust.
"""
from __future__ import annotations

import datetime as _dt
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

STAR_ROOT = Path(r"D:\star")
TW_ROOT = Path(__file__).resolve().parent.parent
FIXTURES = TW_ROOT / "fixtures"
OUT_DIR = FIXTURES / "star-parity"
FIXTURE_NAMES = ["sample.txt", "sample.md", "sample.html"]
TUI_WRAP = 78  # what the TUI uses on an 80-column terminal: wrap_width 0 -> (w - 2)

os.environ.setdefault("STAR_NO_AUTOINSTALL", "1")  # never let Star pip-install
sys.dont_write_bytecode = True  # never write __pycache__ into D:\star
sys.path.insert(0, str(STAR_ROOT))

import star  # noqa: E402
import star.settings as _star_settings  # noqa: E402

# Pure defaults: point Settings at a file that does not exist (Star treats
# FileNotFoundError as "first launch, defaults are correct").
_star_settings.SETTINGS_FILE = OUT_DIR / "__no_such_settings__.json"

from star.settings import Settings  # noqa: E402
from star.documents import load_document  # noqa: E402
from star.documents.model import _WORD_TOKEN_RE, _build_word_map  # noqa: E402
from star.documents.html import _HTML2MD  # noqa: E402
from star.documents.handlers import _document_from_markdown  # noqa: E402
from star._runtime import _SENTENCE_SPLIT_RE, _PANDOC_BIN  # noqa: E402
from star.ttstext import _preprocess_tts_text  # noqa: E402
from star.render import render_markdown  # noqa: E402
from star.tui.mixin_document import DocumentMixin  # noqa: E402
from star.tui.theming import _HEADING_ROLES, _TABLE_ROLES  # noqa: E402

_PARA_SPLIT_RE = re.compile(r"\n{2,}")


def _settings(prefer_pandoc: bool = True) -> Settings:
    s = Settings()
    # Deviation from a stock install, both side-effect avoidance only:
    s._data["document_cache"] = False  # never read/write Star's cache dir
    s._data["prefer_pandoc"] = prefer_pandoc  # True is Star's default
    return s


def _utf8_offsets(text: str, offsets):
    """Map code-point offsets to UTF-8 byte offsets."""
    out = []
    for o in offsets:
        out.append(len(text[:o].encode("utf-8")))
    return out


class _SentenceStub:
    """Minimal object so Star's own TUI _build_sentence_map can run unchanged."""

    def __init__(self, doc):
        self.doc = doc
        self._sentence_starts = [0]


def _tui_view(doc):
    """Render like the TUI (star/tui/mixin_document.py:_render_doc) and build the
    word map the way the TUI does, so line-based navigation data is exact."""
    rendered = render_markdown(doc.markdown, TUI_WRAP, tab_width=4, syntax=True)
    flat = ["".join(t for t, _ in line) for line in rendered]
    word_map = _build_word_map(doc.plain_text, flat)

    def line_to_word(line):  # star/tui/mixin_navigation.py:_line_to_word
        for i, wp in enumerate(word_map):
            if wp.disp_line >= line:
                return i
        return 0

    para_starts = []  # first line of each run of non-blank rendered lines
    for i, line in enumerate(rendered):
        if line and (i == 0 or not rendered[i - 1]):
            para_starts.append(i)
    heading_lines = [
        i for i, line in enumerate(rendered)
        if any(role in _HEADING_ROLES for _, role in line)
    ]
    table_lines = [
        i for i, line in enumerate(rendered)
        if any(role in _TABLE_ROLES for _, role in line)
    ]
    return {
        "wrap_width": TUI_WRAP,
        "lines": flat,
        "paragraph_start_lines": para_starts,
        "paragraph_start_words": [line_to_word(i) for i in para_starts],
        "heading_lines": heading_lines,
        "table_lines": table_lines,
        "word_map": [[wp.tts_offset, wp.tts_len, wp.disp_line, wp.disp_col] for wp in word_map],
    }, word_map


def build_record(doc, settings) -> dict:
    plain = doc.plain_text

    # Word tokenizer: star/documents/model.py:_WORD_TOKEN_RE (used by _build_word_map)
    tokens = [[m.start(), m.end(), m.group()] for m in _WORD_TOKEN_RE.finditer(plain)]

    # Sentence boundaries: star/_runtime.py:_SENTENCE_SPLIT_RE, char_starts as in
    # star/tui/mixin_document.py:_build_sentence_map (0 + every match end).
    sentence_starts = [0] + [m.end() for m in _SENTENCE_SPLIT_RE.finditer(plain)]

    # Word-index sentence map exactly as the TUI builds it (Star's own method).
    tui, word_map = _tui_view(doc)
    doc.word_map = word_map
    stub = _SentenceStub(doc)
    DocumentMixin._build_sentence_map(stub)
    sentence_start_words = list(stub._sentence_starts)
    sentence_start_word_chars = [
        word_map[w].tts_offset for w in sentence_start_words if 0 <= w < len(word_map)
    ]

    # Paragraph starts in the plain text (derived: Star separates paragraphs in
    # plain_text with "\n\n"; there is no dedicated plain-text paragraph splitter).
    paragraph_starts = [0] + [m.end() for m in _PARA_SPLIT_RE.finditer(plain)] if plain else []

    # Speak-time normalization per sentence (star/ttstext/pipeline.py), defaults.
    normalized = []
    bounds = sentence_starts + [len(plain)]
    for a, b in zip(bounds, bounds[1:]):
        sent = plain[a:b].strip()
        if not sent:
            continue
        normalized.append({
            "start": a,
            "input": sent,
            "normalized": _preprocess_tts_text(sent, settings),
        })

    return {
        "title": doc.title,
        "format": doc.format,
        "markdown": doc.markdown,
        "plain_text": plain,
        "word_tokens": tokens,
        "word_tokens_utf8": [
            [s, e] for s, e in zip(
                _utf8_offsets(plain, [t[0] for t in tokens]),
                _utf8_offsets(plain, [t[1] for t in tokens]),
            )
        ],
        "sentence_starts": sentence_starts,
        "sentence_starts_utf8": _utf8_offsets(plain, sentence_starts),
        "sentence_start_words": sentence_start_words,
        "sentence_start_word_chars": sentence_start_word_chars,
        "paragraph_starts": paragraph_starts,
        "paragraph_starts_utf8": _utf8_offsets(plain, paragraph_starts),
        "normalized": normalized,
        "normalized_full_text": _preprocess_tts_text(plain, settings),
        "tui_view": tui,
    }


class _HTML2MDVoidFixed(_HTML2MD):
    """DIAGNOSTIC ONLY - not Star behaviour.  Star's _HTML2MD puts the void
    elements meta/link/base in its _SKIP set and increments a skip counter on the
    start tag, but HTML void elements have no end tag, so the counter never
    returns to 0 and everything after an unclosed <meta ...> is dropped.  This
    subclass ignores those three tags so the converter's intended output is
    visible."""

    def handle_starttag(self, tag, attrs):
        if tag in ("meta", "link", "base"):
            return
        super().handle_starttag(tag, attrs)

    def handle_endtag(self, tag):
        if tag in ("meta", "link", "base"):
            return
        super().handle_endtag(tag)


def _pandoc_version() -> str:
    if not _PANDOC_BIN:
        return ""
    try:
        r = subprocess.run([_PANDOC_BIN, "--version"], capture_output=True, text=True, timeout=30)
        return r.stdout.splitlines()[0] if r.stdout else ""
    except Exception:
        return ""


def _star_commit() -> str:
    git = shutil.which("git")
    if not git:
        return ""
    try:
        r = subprocess.run([git, "-C", str(STAR_ROOT), "rev-parse", "HEAD"],
                           capture_output=True, text=True, timeout=30)
        return r.stdout.strip()
    except Exception:
        return ""


def main() -> int:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    meta = {
        "star_version": getattr(star, "__version__", ""),
        "star_commit": _star_commit(),
        "python": sys.version.split()[0],
        "pandoc_on_path": _pandoc_version(),
        "generated": _dt.date.today().isoformat(),
        "offset_unit": "unicode code points (Python str index), end-exclusive; *_utf8 = UTF-8 bytes",
        "settings": "Star DEFAULTS (star/settings.py) with document_cache=False",
    }
    for name in FIXTURE_NAMES:
        path = FIXTURES / name
        settings = _settings(prefer_pandoc=True)
        doc = load_document(str(path), settings)
        rec = {"source": f"fixtures/{name}", "meta": meta}
        rec.update(build_record(doc, settings))
        if name.endswith(".html"):
            rec["primary_route"] = (
                "pandoc (prefer_pandoc=True default and a pandoc binary was found)"
                if _PANDOC_BIN else "native _HTML2MD (no pandoc found)"
            )
            variants = {}
            s2 = _settings(prefer_pandoc=False)
            d2 = load_document(str(path), s2)
            variants["native_prefer_pandoc_false"] = {
                "note": "Star's native HTML loader (HTMLHandler -> _load_html -> _HTML2MD). "
                        "Empty because of the void-element skip bug (unclosed <meta>).",
                **build_record(d2, s2),
            }
            p = _HTML2MDVoidFixed()
            p.feed(path.read_text(encoding="utf-8", errors="replace"))
            p.close()
            d3 = _document_from_markdown(str(path), "html", p.result(), s2)
            variants["diagnostic_html2md_void_fix_NOT_STAR_BEHAVIOR"] = {
                "note": "NOT Star output: _HTML2MD with meta/link/base ignored, rest of "
                        "Star's pipeline unchanged. Shows the converter's intended shape.",
                **build_record(d3, s2),
            }
            rec["variants"] = variants
        out = OUT_DIR / (name + ".json")
        out.write_text(json.dumps(rec, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
        print(f"wrote {out}  ({len(rec['plain_text'])} chars, "
              f"{len(rec['word_tokens'])} words, {len(rec['sentence_starts'])} sentence starts)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
