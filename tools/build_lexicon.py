#!/usr/bin/env python3
"""Build textweaver's define-word data file from Open English WordNet and CMUdict.

The data file (`third_party/lexicon/lexicon-en.twlex`) holds every WordNet
headword, sense, example, synonym, opposite, and "kind of", the exception
lists morphy uses, and CMUdict's pronunciations, in the format described in
`crates/textweaver-lexicon/src/data.rs`.

This script:

1. downloads the two sources, only from their official GitHub locations,
   into a cache folder (`target/lexicon-sources` by default), unless they
   are already there:
   - Open English WordNet 2025, the WNDB zip from the `2025-edition`
     release of github.com/globalwordnet/english-wordnet (CC BY 4.0);
   - `cmudict.dict` from github.com/cmusphinx/cmudict at a pinned commit
     (BSD-style licence), with its LICENSE file;
2. checks each file's SHA-256 against the sums below (and in
   `third_party/lexicon/README.md`), and stops if one differs;
3. unpacks the WNDB files and runs the builder,
   `cargo run --release -p textweaver-lexicon --example build_lexicon`,
   which parses them, builds the file, and times some lookups;
4. writes the licence notice from the head of WordNet's data files, which
   must go with every copy of the database, to
   `third_party/lexicon/WORDNET-LICENSE`, and CMUdict's licence to
   `third_party/lexicon/CMUDICT-LICENSE`;
5. prints the file's size and SHA-256.

The build is reproducible: the same sources give the same bytes. With
`--check`, the file is built into the cache folder and compared with the
one in `third_party/lexicon/` instead of replacing it.

Needs Python 3.8+ and Rust. Run from anywhere:

    python tools/build_lexicon.py
    python tools/build_lexicon.py --check
"""

from __future__ import annotations

import argparse
import hashlib
import os
import shutil
import subprocess
import sys
import urllib.request
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "third_party" / "lexicon" / "lexicon-en.twlex"

WORDNET_NAME = "Open English WordNet"
WORDNET_VERSION = "2025-edition"
WORDNET_URL = (
    "https://github.com/globalwordnet/english-wordnet/releases/download/"
    "2025-edition/english-wordnet-2025.zip"
)
WORDNET_FILE = "english-wordnet-2025.zip"
WORDNET_SHA256 = "73355e48f8117a24ca9ebc23ed75b35434e6cd21cc9dd3984e80aff5a5f63636"
WORDNET_LICENCE = "CC BY 4.0"

CMUDICT_NAME = "CMUdict"
CMUDICT_COMMIT = "74790861f652b15e4ac49015a90074ad62a27690"
CMUDICT_URL = (
    f"https://raw.githubusercontent.com/cmusphinx/cmudict/{CMUDICT_COMMIT}/cmudict.dict"
)
CMUDICT_FILE = "cmudict.dict"
CMUDICT_SHA256 = "81917843c7f44ce2b094ac63873c2c7a4cf802040792c455ba3ca406891c3d22"
CMUDICT_LICENSE_URL = (
    f"https://raw.githubusercontent.com/cmusphinx/cmudict/{CMUDICT_COMMIT}/LICENSE"
)
CMUDICT_LICENSE_SHA256 = "bd4ce8e44170a5f9f481310ca85c51de3c4f851a65e679b40e603b143bd3542a"
CMUDICT_LICENCE = "BSD-2-Clause-style (Carnegie Mellon University)"

# A neutral User-Agent: nothing about the person running the build leaves
# the machine.
USER_AGENT = "textweaver-research (+https://github.com/leavesofgrass/textweaver)"


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def fetch(url: str, dest: Path, expect: str) -> None:
    """Download `url` to `dest` unless a file with the right sum is there."""
    if dest.exists() and sha256(dest) == expect:
        print(f"have {dest.name}")
        return
    print(f"downloading {url}")
    tmp = dest.with_suffix(dest.suffix + ".part")
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(req, timeout=120) as r, tmp.open("wb") as f:
        shutil.copyfileobj(r, f)
    got = sha256(tmp)
    if got != expect:
        tmp.unlink()
        sys.exit(f"{dest.name}: SHA-256 is {got}, expected {expect}; not using it")
    tmp.replace(dest)


def write_notices(data_noun: Path, cmudict_license: Path) -> None:
    """Copy the WordNet notice (the numbered lines heading every data file)
    and CMUdict's LICENSE next to the data file."""
    lines = []
    with data_noun.open(encoding="utf-8", errors="replace") as f:
        for line in f:
            if not line.startswith("  "):
                break
            text = line.strip().split(" ", 1)
            lines.append(text[1].rstrip() if len(text) > 1 else "")
    (OUT.parent / "WORDNET-LICENSE").write_text("\n".join(lines).strip() + "\n", encoding="utf-8", newline="\n")
    text = cmudict_license.read_text(encoding="utf-8").replace("\r\n", "\n")
    (OUT.parent / "CMUDICT-LICENSE").write_text(text, encoding="utf-8", newline="\n")


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    ap.add_argument(
        "--cache",
        type=Path,
        default=ROOT / "target" / "lexicon-sources",
        help="where the downloads and unpacked files go",
    )
    ap.add_argument(
        "--check",
        action="store_true",
        help="build into the cache and compare with the committed file",
    )
    args = ap.parse_args()
    cache: Path = args.cache
    cache.mkdir(parents=True, exist_ok=True)

    fetch(WORDNET_URL, cache / WORDNET_FILE, WORDNET_SHA256)
    fetch(CMUDICT_URL, cache / CMUDICT_FILE, CMUDICT_SHA256)
    fetch(CMUDICT_LICENSE_URL, cache / "cmudict-LICENSE", CMUDICT_LICENSE_SHA256)

    wndb = cache / "wndb"
    if wndb.exists():
        shutil.rmtree(wndb)
    with zipfile.ZipFile(cache / WORDNET_FILE) as z:
        z.extractall(wndb)
    dirs = [p.parent for p in wndb.rglob("data.noun")]
    if len(dirs) != 1:
        sys.exit(f"expected one data.noun in the WNDB zip, found {len(dirs)}")

    if not args.check:
        write_notices(dirs[0] / "data.noun", cache / "cmudict-LICENSE")

    out = cache / OUT.name if args.check else OUT
    sources = [
        f"{WORDNET_NAME}|{WORDNET_VERSION}|{WORDNET_LICENCE}|{WORDNET_URL}|{WORDNET_SHA256}",
        f"{CMUDICT_NAME}|{CMUDICT_COMMIT}|{CMUDICT_LICENCE}|{CMUDICT_URL}|{CMUDICT_SHA256}",
    ]
    cmd = [
        os.environ.get("CARGO", "cargo"),
        "run",
        "--release",
        "--quiet",
        "-p",
        "textweaver-lexicon",
        "--example",
        "build_lexicon",
        "--",
        "--wndb",
        str(dirs[0]),
        "--cmudict",
        str(cache / CMUDICT_FILE),
        "--out",
        str(out),
    ]
    for s in sources:
        cmd += ["--source", s]
    subprocess.run(cmd, cwd=ROOT, check=True)

    size = out.stat().st_size
    digest = sha256(out)
    print(f"{out.relative_to(ROOT) if out.is_relative_to(ROOT) else out}: {size:,} bytes, SHA-256 {digest}")
    if args.check:
        if not OUT.exists():
            sys.exit(f"{OUT} is missing")
        if sha256(OUT) != digest:
            sys.exit(f"{OUT.relative_to(ROOT)} differs from a fresh build")
        print("the committed file matches a fresh build")


if __name__ == "__main__":
    main()
