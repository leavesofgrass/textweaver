#!/usr/bin/env python3
"""Picks a SAPI voice from `tw voices --backend sapi --json` for the engine
checks, and prints its id.

Kinds:
- sapi64: a SAPI 5 voice in the 64-bit host (no "OneCore" or "32-bit" tag);
- onecore: a OneCore voice, loaded through SAPI 5 (tag "OneCore");
- sapi32: a voice in the 32-bit host (tag "32-bit").

Only Microsoft voices and eSpeak are used, as in the SAPI crate's real-voice
tests: never the VW voices, which belong to another product, and never
Eloquence. When the voice list cannot be read, it prints a line starting
"Fail:" and exits 1. When the runner simply has no voice of the kind, it
prints a line starting "Skipped:" with the voices it did find, and exits
3, so the caller can report that plainly without failing.

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
    try:
        with open(path, encoding="utf-8-sig") as f:
            voices = json.load(f)["voices"]
    except (OSError, ValueError, KeyError) as e:
        print(f"Fail: the voice list could not be read: {e}")
        return 1
    usable = [
        v for v in voices
        if v["name"].startswith(ALLOWED_PREFIXES) and kind_of(v) == kind
        and not tags(v) & {"eloquence", "openevv"}
    ]
    # English first: the fixture is English. For OneCore, David last:
    # Microsoft David (OneCore) shares its voice data with David Desktop
    # (the SAPI 5 pick) and writes the same audio byte for byte, so the
    # workflow's "two voices give different audio" check needs Mark or Zira.
    def order(v: dict) -> tuple[bool, bool]:
        english = any(lang.lower().startswith("en") for lang in v.get("languages", []))
        david = kind == "onecore" and "david" in v["name"].casefold()
        return (not english, david)

    usable.sort(key=order)
    if not usable:
        names = ", ".join(v["name"] for v in voices) or "none"
        print(f"Skipped: this runner has no {kind} voice from Microsoft or eSpeak. Voices: {names}.")
        return 3
    print(usable[0]["id"])
    return 0


if __name__ == "__main__":
    sys.exit(main())
