#!/usr/bin/env python3
"""Checks a WAV file that `tw export-audio --json` wrote with a real engine.

Nothing is played: the file is only read. Only these fail (a "Fail" line
and exit status 1), since each means the engine did not export at all:

- the report or the WAV file cannot be read;
- the export used another backend than the one asked for (a fallback);
- the WAV file holds no audio.

These are checked and reported as "Pass" or "Warning", never failed, until
runs on main give each engine a baseline:

- the length matches the timeline's length (within 2 percent or 100 ms);
- it is not silent, and its speed is plausible (60 to 600 words a minute);
- word events: at least 90 percent of the text's words have a time, and
  the times rise and stay inside the audio. A backend that reports no word
  times in exported files at all gets one Warning saying so.

It also measures, and reports as "Measured" lines without failing, where
the word times fall against the audio's own silences: at each sentence
start after the first, the distance from the first word's time to the end
of the nearest silence; and how many word starts fall deep inside a long
silence, where no word can start. That is where drift between the engine's
word events and its audio would show (ADR-0023's check for Piper; on macOS
it shows whether AVSpeech's callback interleaving still holds).

Standard library only. Usage:
    check_export.py --engine LABEL --backend ID --wav F.wav --report F.json
                    [--summary FILE]
"""

from __future__ import annotations

import argparse
import array
import json
import math
import re
import statistics
import sys
import wave

WINDOW_MS = 10
SILENCE_MIN_MS = 80
DEEP_MS = 50


def read_wav(path: str) -> tuple[int, list[float]]:
    """Sample rate and mono samples scaled to -1..1 (PCM 8, 16, or 32-bit)."""
    with wave.open(path, "rb") as w:
        rate, channels, width = w.getframerate(), w.getnchannels(), w.getsampwidth()
        raw = w.readframes(w.getnframes())
    if width == 2:
        data, scale = array.array("h", raw), 32768.0
    elif width == 4:
        data, scale = array.array("i", raw), 2147483648.0
    elif width == 1:
        data, scale = array.array("B", raw), 128.0
        data = array.array("h", (v - 128 for v in data))
    else:
        raise ValueError(f"{width * 8}-bit samples are not read here")
    if sys.byteorder == "big" and width > 1:
        data.byteswap()
    mono = [sum(data[i:i + channels]) / channels / scale for i in range(0, len(data), channels)]
    return rate, mono


def window_rms(samples: list[float], rate: int) -> list[float]:
    n = max(1, rate * WINDOW_MS // 1000)
    return [
        math.sqrt(sum(s * s for s in samples[i:i + n]) / len(samples[i:i + n]))
        for i in range(0, len(samples), n)
    ]


def silences(rms: list[float]) -> list[tuple[int, int]]:
    """Runs of quiet windows at least SILENCE_MIN_MS long, as (start, end) ms.

    Quiet is 40 dB below the loud end of the file (its 95th percentile
    window), so the threshold follows the engine's own level.
    """
    if not rms:
        return []
    loud = sorted(rms)[int(len(rms) * 0.95)]
    quiet = loud / 100.0
    runs, start = [], None
    for i, v in enumerate(rms + [loud]):
        if v <= quiet and start is None:
            start = i
        elif v > quiet and start is not None:
            if (i - start) * WINDOW_MS >= SILENCE_MIN_MS:
                runs.append((start * WINDOW_MS, i * WINDOW_MS))
            start = None
    return runs


def text_words(text: str) -> int:
    return len(re.findall(r"\w+", text))


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--engine", required=True, help="a label for the report lines")
    ap.add_argument("--backend", required=True, help="the backend id the export must have used")
    ap.add_argument("--wav", required=True)
    ap.add_argument("--report", required=True)
    ap.add_argument("--summary")
    args = ap.parse_args()
    name = args.engine
    lines: list[str] = []
    failed = False

    def must(ok: bool, passed: str, failure: str) -> None:
        """A check whose failure means the engine did not export."""
        nonlocal failed
        lines.append(f"Pass: {name}: {passed}" if ok else f"Fail: {name}: {failure}")
        failed |= not ok

    def should(ok: bool, passed: str, warning: str) -> None:
        """A check reported without failing, until it has a baseline."""
        lines.append(f"Pass: {name}: {passed}" if ok else f"Warning: {name}: {warning}")

    try:
        with open(args.report, encoding="utf-8") as f:
            report = json.load(f)
        timeline = report["export"]["timeline"]
        rate, samples = read_wav(args.wav)
    except (OSError, ValueError, KeyError, wave.Error, json.JSONDecodeError) as e:
        lines.append(f"Fail: {name}: the WAV file or the report could not be read: {e}")
        return finish(lines, args.summary, True)

    selection = report.get("backend", {})
    backend = selection.get("backend", {}).get("id", "unknown")
    # A fallback to another engine would check the wrong one.
    must(
        backend == args.backend and not selection.get("fell_back", True),
        f"the export used the {backend} backend, as asked.",
        f"the export used {backend}, not {args.backend}: the engine asked for is missing.",
    )
    lines.append(f"Measured: {name}: {rate} samples a second.")
    wav_ms = len(samples) * 1000 // rate if rate else 0
    must(wav_ms > 0, f"the WAV file holds {wav_ms} ms of audio.", "the WAV file holds no audio.")
    if wav_ms == 0:
        return finish(lines, args.summary, True)

    expected_ms = int(timeline["duration_ms"])
    slack = max(100, expected_ms // 50)
    should(
        abs(wav_ms - expected_ms) <= slack,
        f"its length matches the timeline ({wav_ms} ms against {expected_ms} ms).",
        f"its length is {wav_ms} ms but the timeline says {expected_ms} ms.",
    )

    peak_rms = math.sqrt(sum(s * s for s in samples) / len(samples))
    should(
        peak_rms > 0.001,
        "it is not silent.",
        f"it is silent (overall level {peak_rms:.6f} of full scale).",
    )

    sentences = timeline["sentences"]
    n_text = sum(text_words(s["text"]) for s in sentences)
    wpm = n_text / (wav_ms / 60000)
    should(
        60 <= wpm <= 600,
        f"{n_text} words in {wav_ms / 1000:.1f} s, {wpm:.0f} words per minute.",
        f"{n_text} words in {wav_ms / 1000:.1f} s is {wpm:.0f} words per minute, outside 60 to 600.",
    )

    timed = [w for s in sentences for w in s["words"]]
    starts = [int(w["start_ms"]) for w in timed]
    if not timed:
        # Said once, plainly: some backends write files without word times
        # (the export's default, ADR-0011), which subtitles then lack.
        lines.append(
            f"Warning: {name}: the exported file has no word times at all, "
            f"so subtitles and word highlighting from it have none. "
            f"The audio itself exported."
        )
    else:
        share = len(timed) / n_text if n_text else 0.0
        should(
            share >= 0.9,
            f"word events for {len(timed)} of {n_text} words.",
            f"word events for only {len(timed)} of {n_text} words.",
        )
        rising = all(a <= b for a, b in zip(starts, starts[1:]))
        inside = all(0 <= t <= expected_ms + 50 for t in starts)
        should(
            rising and inside,
            "the word times rise and stay inside the audio.",
            "the word times do not rise, or fall outside the audio.",
        )

    # Measurements against the audio's silences: reported, never failed.
    quiet = silences(window_rms(samples, rate))
    lines.append(f"Measured: {name}: {len(quiet)} silences of {SILENCE_MIN_MS} ms or more.")
    offsets = []
    for s in sentences[1:]:
        if not s["words"] or not quiet:
            continue
        first = int(s["words"][0]["start_ms"])
        nearest = min((end for _, end in quiet), key=lambda end: abs(end - first))
        offsets.append(first - nearest)
    if offsets:
        lines.append(
            f"Measured: {name}: at {len(offsets)} sentence starts, the first word's time is "
            f"{statistics.median(offsets):+.0f} ms from the end of the nearest silence (median); "
            f"the largest difference is {max(offsets, key=abs):+d} ms. Plus means after."
        )
    if starts:
        deep = [
            t for t in starts
            if any(a + DEEP_MS < t < b - DEEP_MS for a, b in quiet)
        ]
        lines.append(
            f"Measured: {name}: {len(deep)} of {len(starts)} word starts fall more than "
            f"{DEEP_MS} ms inside a silence."
        )
    return finish(lines, args.summary, failed)


def finish(lines: list[str], summary: str | None, failed: bool) -> int:
    for line in lines:
        print(line)
    if summary:
        with open(summary, "a", encoding="utf-8") as s:
            for line in lines:
                s.write(line + "\n")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
