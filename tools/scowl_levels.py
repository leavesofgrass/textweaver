#!/usr/bin/env python3
"""Derive textweaver's built-in difficult-word list from SCOWL.

SCOWL (Spell Checker Oriented Word Lists, https://github.com/en-wl/wordlist)
gives every English word a *size*: the smallest dictionary it belongs in.
Smaller sizes hold more common words (35 small, 50 medium, 60 the default
spell-checker size, 70 large, 80 "a valid word in current usage"). textweaver
marks a word difficult when it first appears above a chosen size
(`textweaver-aids`, `difficult.rs`).

This script:

1. downloads the SCOWL release archive (tag `rel-2026.02.25`) from GitHub and
   checks its SHA-256, or uses `--source DIR` (an unpacked archive);
2. builds SCOWL's SQLite database with SCOWL's own `combine.py create-db`
   (what SCOWL's Makefile runs; about four minutes), unless it exists;
3. selects, from SCOWL's documented `scowl_v0` view, every word of size 80 or
   less in any of SCOWL's English spellings (American `A`, British `B` and
   `Z`, Canadian `C`, Australian `D`, and region-neutral `_`), any region, a
   variant level of 6 (`V`, acceptable) or less, and not an abbreviation;
4. keeps words made only of letters, with apostrophes only inside
   (`don't`); drops possessives (`'s`, which textweaver strips before a
   lookup); lowercases; adds an accent-free copy of accented words (`naïve`
   and `naive`); and gives each word the smallest size any of its forms has;
5. writes one section per size, `@SIZE` then one word per line in code point
   order (the same as UTF-8 byte order), LF line ends, as `scowl-levels.txt`
   inside a deflate-compressed zip with a fixed timestamp, so the output is
   reproducible: `third_party/scowl/scowl-levels.zip`.

It also copies SCOWL's `Copyright` file, which must accompany the list.

Needs Python 3.8+ with the standard library (sqlite3 3.33 or newer). Run from
anywhere:

    python tools/scowl_levels.py
    python tools/scowl_levels.py --source path/to/wordlist-rel-2026.02.25
"""

from __future__ import annotations

import argparse
import hashlib
import io
import os
import re
import shutil
import sqlite3
import subprocess
import sys
import tarfile
import unicodedata
import urllib.request
import zipfile
from pathlib import Path

TAG = "rel-2026.02.25"
ARCHIVE_URL = f"https://github.com/en-wl/wordlist/archive/refs/tags/{TAG}.tar.gz"
ARCHIVE_SHA256 = "74e7cc3e9e03e609c1c74bb7e8862fcd988cdd64768dcbee4611581b7e633852"
MAX_SIZE = 80
MAX_VARIANT_LEVEL = 6
SPELLINGS = ("A", "B", "Z", "C", "D", "_")
REGIONS = ("", "US", "GB", "CA", "AU")

REPO = Path(__file__).resolve().parent.parent
OUT_DIR = REPO / "third_party" / "scowl"
CACHE = REPO / "target-local" / "scowl"

WORD = re.compile(r"^[^\W\d_]+(?:'[^\W\d_]+)*$")


def fetch_source() -> Path:
    CACHE.mkdir(parents=True, exist_ok=True)
    archive = CACHE / f"wordlist-{TAG}.tar.gz"
    if not archive.exists():
        print(f"downloading {ARCHIVE_URL}")
        req = urllib.request.Request(ARCHIVE_URL, headers={"User-Agent": "textweaver"})
        with urllib.request.urlopen(req, timeout=300) as r:
            archive.write_bytes(r.read())
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    if digest != ARCHIVE_SHA256:
        sys.exit(f"SHA-256 mismatch for {archive}: {digest}")
    src = CACHE / f"wordlist-{TAG}"
    if not src.exists():
        with tarfile.open(archive) as t:
            t.extractall(CACHE)
    return src


def build_db(src: Path) -> Path:
    db = src / "scowl.db"
    if db.exists():
        return db
    print("building scowl.db with SCOWL's combine.py (a few minutes)")
    env = dict(os.environ, PYTHONUTF8="1")
    subprocess.run([sys.executable, "combine.py", "create-db", "scowl.db"], cwd=src, env=env, check=True)
    return db


def deaccent(word: str) -> str:
    return "".join(c for c in unicodedata.normalize("NFD", word) if unicodedata.category(c) != "Mn")


def derive(db: Path) -> dict[str, int]:
    con = sqlite3.connect(db)
    marks = lambda xs: ",".join("?" * len(xs))  # noqa: E731
    rows = con.execute(
        f"""select word, min(size) from scowl_v0
            where size <= ? and variant_level <= ?
              and spelling in ({marks(SPELLINGS)}) and region in ({marks(REGIONS)})
              and base_pos != 'abbr'
            group by word""",
        (MAX_SIZE, MAX_VARIANT_LEVEL, *SPELLINGS, *REGIONS),
    ).fetchall()
    best: dict[str, int] = {}
    for word, size in rows:
        if not WORD.match(word):
            continue
        w = word.lower()
        if w.endswith("'s"):
            continue
        for form in {w, deaccent(w)}:
            if form not in best or size < best[form]:
                best[form] = size
    return best


def write(best: dict[str, int]) -> None:
    lines = []
    for size in sorted(set(best.values())):
        lines.append(f"@{size}")
        lines.extend(sorted(w for w, s in best.items() if s == size))
    text = ("\n".join(lines) + "\n").encode("utf-8")
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w") as z:
        info = zipfile.ZipInfo("scowl-levels.txt", date_time=(1980, 1, 1, 0, 0, 0))
        info.compress_type = zipfile.ZIP_DEFLATED
        info.external_attr = 0o644 << 16
        info.create_system = 3
        z.writestr(info, text, compress_type=zipfile.ZIP_DEFLATED, compresslevel=9)
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    out = OUT_DIR / "scowl-levels.zip"
    out.write_bytes(buf.getvalue())
    counts = {s: sum(1 for v in best.values() if v == s) for s in sorted(set(best.values()))}
    print(f"words: {len(best)}; by size: {counts}")
    print(f"scowl-levels.txt: {len(text)} bytes, SHA-256 {hashlib.sha256(text).hexdigest()}")
    print(f"{out.relative_to(REPO)}: {out.stat().st_size} bytes, SHA-256 {hashlib.sha256(out.read_bytes()).hexdigest()}")


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--source", type=Path, help=f"an unpacked wordlist-{TAG} tree (default: download)")
    args = ap.parse_args()
    src = args.source.resolve() if args.source else fetch_source()
    db = build_db(src)
    write(derive(db))
    shutil.copyfile(src / "Copyright", OUT_DIR / "Copyright")


if __name__ == "__main__":
    main()
