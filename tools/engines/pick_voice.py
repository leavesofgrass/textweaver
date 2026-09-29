#!/usr/bin/env python3
"""Picks a SAPI voice from `tw voices --backend sapi --json` for the engine
checks, and prints its id.

Kinds:
- sapi64: a SAPI 5 voice in the 64-bit host (no "OneCore" or "32-bit" tag);
- onecore: a OneCore voice, loaded through SAPI 5 (tag "OneCore");
- sapi32: a voice in the 32-bit host (tag "32-bit").

Only Microsoft voices and eSpeak are used, as in the SAPI crate's real-voice
tests: never the VW voices, which belong to another product, and never
Eloquence. When no voice of the kind is installed, it prints a line
starting "Fail:" and exits 1, since a check without its voice would verify
nothing.

Usage: pick_voice.py KIND VOICES.json
"""

from __future__ import annotations

import json
import sys

ALLOWED_PREFIXES = ("Microsoft ", "eSpeak")


def tags(v: dict) -> set[str]:
    return {t.casefold() for t in v.get("tags", [])}


def kind_of(v: dict) -> str:
    t = tags(v)
    if "32-bit" in t:
        return "sapi32"
    if "onecore" in t:
        return "onecore"
    return "sapi64"


def main() -> int:
    if len(sys.argv) != 3 or sys.argv[1] not in ("sapi64", "onecore", "sapi32"):
        print("usage: pick_voice.py sapi64|onecore|sapi32 VOICES.json", file=sys.stderr)
        return 2
    kind, path = sys.argv[1], sys.argv[2]
    with open(path, encoding="utf-8-sig") as f:
        voices = json.load(f)["voices"]
    usable = [
        v for v in voices
        if v["name"].startswith(ALLOWED_PREFIXES) and kind_of(v) == kind
        and not tags(v) & {"eloquence", "openevv"}
    ]
    # English first: the fixture is English.
    usable.sort(key=lambda v: not any(lang.lower().startswith("en") for lang in v.get("languages", [])))
    if not usable:
        names = ", ".join(v["name"] for v in voices) or "none"
        print(f"Fail: no {kind} voice from Microsoft or eSpeak is installed. Voices: {names}.")
        return 1
    print(usable[0]["id"])
    return 0


if __name__ == "__main__":
    sys.exit(main())
