"""Fetch a release of the community IBMTTS pronunciation dictionaries
(github.com/eigencrow/IBMTTSDictionaries, CC0 1.0) into
third_party/ibmtts-dictionaries/, byte for byte.

    python tools/update_ibmtts_dictionaries.py            # latest release
    python tools/update_ibmtts_dictionaries.py v26.09     # a given tag

The .dic files are Windows-1252 text; they are written unchanged and marked
binary in .gitattributes so git never converts them. Update PINNED.txt's
contents by running this script, then review the diff and commit.
"""
import json
import pathlib
import sys
import urllib.request

REPO = "eigencrow/IBMTTSDictionaries"
DEST = pathlib.Path(__file__).resolve().parent.parent / "third_party" / "ibmtts-dictionaries"
FILES = ["ENUmain.dic", "ENURoot.dic", "ENUabbr.dic", "DEUmain.dic", "DEURoot.dic", "DEUabbr.dic", "LICENSE.md"]


def get(url: str) -> bytes:
    req = urllib.request.Request(url, headers={"User-Agent": "textweaver-dictionary-update"})
    with urllib.request.urlopen(req, timeout=60) as r:
        return r.read()


def main() -> None:
    if len(sys.argv) > 1:
        tag = sys.argv[1]
    else:
        tag = json.loads(get(f"https://api.github.com/repos/{REPO}/releases/latest"))["tag_name"]
    ref = json.loads(get(f"https://api.github.com/repos/{REPO}/commits/{tag}"))
    sha, date = ref["sha"], ref["commit"]["committer"]["date"]
    DEST.mkdir(parents=True, exist_ok=True)
    for name in FILES:
        data = get(f"https://raw.githubusercontent.com/{REPO}/{sha}/{name}")
        (DEST / name).write_bytes(data)
        print(f"{name}: {len(data)} bytes")
    (DEST / "PINNED.txt").write_text(f"repository: https://github.com/{REPO}\ntag: {tag}\ncommit: {sha}\ncommit date: {date}\n", encoding="utf-8", newline="\n")
    print(f"pinned {tag} ({sha})")


if __name__ == "__main__":
    main()
