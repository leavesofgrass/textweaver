#!/usr/bin/env python3
"""Dump what Orca would see of the Xilem GUI through AT-SPI.

Finds the textweaver-xilem window on the accessibility bus, prints its tree
(roles, names, states), checks the document's Text interface (character
count, text, caret offset, attributes at the caret), records the caret,
selection, and announcement events for a few seconds while the GUI reads
with the silent `paced` backend, and exits non-zero if a check fails.

Run it with tools/atspi-check.sh (Xvfb, a D-Bus session, and the AT-SPI
bus), or by hand on a desktop with accessibility on:

    python3 crates/textweaver-xilem/tools/atspi-dump.py --pid PID --seconds 6
"""

import argparse
import sys
import time

import pyatspi  # python3-pyatspi

EVENTS = []
PID = None


def ours(acc):
    try:
        return acc.get_process_id() == PID
    except Exception:
        return False


def on_event(event):
    if not ours(event.source):
        return
    detail = ""
    if event.type.startswith("object:announcement"):
        detail = repr(event.any_data)
    elif event.type.startswith("object:text-caret-moved"):
        detail = f"offset {event.detail1}"
    EVENTS.append(f"{time.monotonic() - START:7.2f} s  {event.type:40} {detail}")


def states(acc):
    return sorted(pyatspi.stateToString(s) for s in acc.getState().getStates())


def walk(acc, depth, out, docs):
    if depth > 14 or len(out) > 300:
        return
    role = acc.getRoleName()
    if role == "text" and acc.parent and acc.parent.getRoleName() == "document text":
        return
    name = acc.name or ""
    st = ",".join(s for s in states(acc) if s in ("focusable", "focused", "editable", "read-only", "selectable", "showing"))
    out.append(f"{'  ' * depth}- {role} {name!r} [{st}]")
    if role in ("document text", "document frame", "document web"):
        docs.append(acc)
        return
    for i in range(acc.childCount):
        try:
            walk(acc.getChildAtIndex(i), depth + 1, out, docs)
        except Exception as e:  # a node went away mid-walk
            out.append(f"{'  ' * (depth + 1)}- (gone: {e})")


def find_app():
    desktop = pyatspi.Registry.getDesktop(0)
    for i in range(desktop.childCount):
        app = desktop.getChildAtIndex(i)
        if app is None:
            continue
        try:
            if PID and app.get_process_id() != PID:
                continue
        except Exception:
            continue
        for j in range(app.childCount):
            w = app.getChildAtIndex(j)
            if w is not None and "textweaver" in (w.name or ""):
                return app, w
    return None, None


def main():
    global PID, START
    ap = argparse.ArgumentParser()
    ap.add_argument("--pid", type=int, required=True)
    ap.add_argument("--seconds", type=float, default=6.0)
    args = ap.parse_args()
    PID = args.pid
    START = time.monotonic()
    failures = []

    for kind in ("object:announcement", "object:text-caret-moved", "object:text-selection-changed"):
        pyatspi.Registry.registerEventListener(on_event, kind)

    app, window = None, None
    for _ in range(100):
        app, window = find_app()
        if window is not None:
            break
        time.sleep(0.2)
    if window is None:
        print("FAIL: no textweaver window on the AT-SPI bus (is accessibility on?)")
        return 1
    print(f"# textweaver-xilem AT-SPI report\n\n- Application: {app.name!r} (toolkit {app.get_toolkit_name()!r})")
    print(f"- Window: {window.getRoleName()} {window.name!r}\n")

    # Let the GUI read while events arrive, sampling the caret.
    from gi.repository import GLib

    samples = []

    def sample():
        try:
            docs = []
            walk(window, 0, [], docs)
            if docs:
                t = docs[0].queryText()
                off = t.caretOffset
                attrs = t.getAttributeRun(off, False)[0]
                word = t.getTextAtOffset(off, pyatspi.TEXT_BOUNDARY_WORD_START)[0]
                samples.append(f"caret {off:5}  word {word!r:16} attributes {attrs}")
        except Exception as e:
            samples.append(f"(sample failed: {e})")
        return True

    def stop():
        pyatspi.Registry.stop()
        return False

    GLib.timeout_add(500, sample)
    GLib.timeout_add(int(args.seconds * 1000), stop)
    pyatspi.Registry.start()

    out, docs = [], []
    walk(window, 0, out, docs)
    print("## Tree (document text runs left out)\n\n```")
    print("\n".join(out))
    print("```\n")
    if not docs:
        failures.append("no document with the Text interface")
    else:
        d = docs[0]
        t = d.queryText()
        text = t.getText(0, -1)
        print("## Document\n")
        print(f"- Role {d.getRoleName()!r}, name {d.name!r}, states {states(d)}")
        print(f"- Characters: {t.characterCount}")
        print(f"- First 120: {text[:120]!r}")
        print(f"- Caret offset: {t.caretOffset}\n")
        if t.characterCount == 0:
            failures.append("the document's Text is empty")
        print("Caret, word, and attributes while reading, every 500 ms:\n\n```")
        print("\n".join(samples) or "(none)")
        print("```\n")
        if len(set(samples)) < 2:
            failures.append("the caret did not follow the reading")
        if not any("background-color" in s or "bg-color" in s for s in samples):
            failures.append("no background colour attribute on the spoken word")
    print("## Events\n\n```")
    print("\n".join(EVENTS) or "(none)")
    print("```\n")
    if not any("announcement" in e for e in EVENTS):
        failures.append("no announcement events")
    if not any("caret" in e for e in EVENTS):
        failures.append("no caret-moved events")
    print("## Result\n")
    if failures:
        print("FAIL:")
        for f in failures:
            print(f"- {f}")
        return 1
    print("PASS: every check passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
