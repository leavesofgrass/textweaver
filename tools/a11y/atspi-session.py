#!/usr/bin/env python3
"""A scripted reading session in the Xilem GUI, checked through AT-SPI.

What atspi-dump.py (crates/textweaver-xilem/tools) checks once, this
checks through a session, the way Orca would follow it (ADR-0039): it
starts textweaver-xilem on fixtures/t/reading.md with the silent `paced`
backend, sends keys with xdotool, and records the AT-SPI events at each
step:

- open: the window and the document appear, the "Opened" announcement;
- read (Ctrl+Shift+Space): the caret follows the reading;
- pause (Ctrl+Shift+Space): the "Paused" announcement, once;
- heading (h): the caret moves to the line "Second heading";
- list (Alt+O, the outline): a list with the headings takes the focus;
- close (Escape): the list goes and the document is back.

It does not use AT-SPI's Collection interface: AccessKit does not offer it
yet (AccessKit pull request 758 is a draft), so the tree is walked by hand.

With --orca-log FILE, Orca is running in the same session with
`--debug-file FILE`; the SPEECH OUTPUT lines written during each step are
listed as what Orca said. They are reported, not checked: the events are
the check.

Run it with tools/a11y/orca-session.sh (Xvfb, D-Bus, the AT-SPI bus,
xdotool, and Orca). It exits 1 when a must check fails.
"""

import argparse
import importlib.util
import os
import re
import subprocess
import sys
import tempfile
import time

import pyatspi  # python3-pyatspi
from gi.repository import GLib

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", ".."))

# Reuse the tree walk of the GUI crate's AT-SPI check rather than copy it.
_spec = importlib.util.spec_from_file_location(
    "atspi_dump", os.path.join(REPO, "crates", "textweaver-xilem", "tools", "atspi-dump.py"))
dump = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(dump)

EVENT_KINDS = (
    "object:announcement",
    "object:text-caret-moved",
    "object:text-selection-changed",
    "object:state-changed:focused",
    "object:children-changed",
    "window:activate",
)

START = time.monotonic()
EVENTS = []          # (step, seconds, type, detail)
CURRENT = ["launch"]


def on_event(event):
    if not dump.ours(event.source):
        return
    if event.type.startswith("object:state-changed:focused") and not event.detail1:
        return
    detail = ""
    if event.type.startswith("object:announcement"):
        detail = str(event.any_data)
    elif event.type.startswith("object:text-caret-moved"):
        detail = f"offset {event.detail1}"
    else:
        try:
            detail = f"{event.source.getRoleName()} {event.source.name!r}"
        except Exception:
            detail = ""
    EVENTS.append((CURRENT[0], time.monotonic() - START, event.type, detail))


def xdotool(*args):
    """Runs xdotool with a 15-second limit; a timeout gives empty output."""
    try:
        return subprocess.run(["xdotool", *args], capture_output=True, text=True, timeout=15)
    except subprocess.TimeoutExpired:
        return subprocess.CompletedProcess(args, 1, "", "timed out")


def find_document(window):
    docs = []
    dump.walk(window, 0, [], docs)
    return docs[0] if docs else None


def find_roles(acc, roles, depth=0, found=None):
    """Nodes with one of these roles, walking the tree by hand (no Collection)."""
    found = [] if found is None else found
    if depth > 16 or len(found) > 50:
        return found
    try:
        role = acc.getRoleName()
        if role in roles:
            found.append(acc)
        if role in ("document text", "document frame"):
            return found
        for i in range(acc.childCount):
            child = acc.getChildAtIndex(i)
            if child is not None:
                find_roles(child, roles, depth + 1, found)
    except Exception:
        pass
    return found


def line_at_caret(doc):
    """The caret offset and the text of its line.

    AccessKit's AT-SPI adapter answers GetStringAtOffset but not the older
    GetTextAtOffset (the first run failed with "Unknown method
    'GetTextAtOffset'"), and pyatspi's wrapper for the newer call varies
    between versions, so the line is cut from the whole text instead.
    """
    t = doc.queryText()
    off = t.caretOffset
    text = t.getText(0, -1)
    start = text.rfind("\n", 0, off) + 1
    end = text.find("\n", off)
    line = text[start:end if end >= 0 else len(text)].strip()
    # If the document has no line breaks between paragraphs, the text from
    # the caret on still says where it is.
    return off, line if len(line) < 200 else text[off:off + 60].strip()


def orca_tail(path, start):
    """What Orca said since byte offset `start` of its debug log."""
    if not path or not os.path.exists(path):
        return [], start
    with open(path, "rb") as f:
        f.seek(start)
        data = f.read().decode("utf-8", "replace")
        end = f.tell()
    said = re.findall(r"SPEECH OUTPUT: '(.*?)'", data)
    return [s for s in said if s.strip()], end


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--exe", required=True)
    ap.add_argument("--doc", default=os.path.join(REPO, "fixtures", "t", "reading.md"))
    ap.add_argument("--out", default="atspi-session.md")
    ap.add_argument("--orca-log")
    args = ap.parse_args()

    for kind in EVENT_KINDS:
        pyatspi.Registry.registerEventListener(on_event, kind)

    home = tempfile.mkdtemp(prefix="tw-a11y-")
    log = os.path.join(os.path.dirname(os.path.abspath(args.out)), "gui.log")
    gui = subprocess.Popen([args.exe, args.doc, "--backend", "paced", "--home", home,
                            "--log-file", log, "--exit-after", "120"])
    dump.PID = gui.pid

    results = []   # (step, word, text)
    heard = {}     # step -> Orca's phrases
    state = {"window": None, "doc": None, "orca_offset": 0}
    if args.orca_log and os.path.exists(args.orca_log):
        state["orca_offset"] = os.path.getsize(args.orca_log)

    def ok(step, cond, text, must=True):
        results.append((step, "Pass" if cond else ("Fail" if must else "Warning"), text))

    def events(step, kind):
        return [e for e in EVENTS if e[0] == step and e[2].startswith(kind)]

    def end_step(step):
        said, state["orca_offset"] = orca_tail(args.orca_log, state["orca_offset"])
        heard[step] = said

    # Each step: (name, keys to send, seconds to wait, check to run after).
    def check_open():
        app, window = dump.find_app()
        state["window"] = window
        ok("open", window is not None, "the textweaver window is on the AT-SPI bus")
        if window is None:
            return
        ok("open", "Reading check" in (window.name or "") or "textweaver" in (window.name or ""),
           f"the window is named after the document ({window.name!r})")
        state["doc"] = find_document(window)
        ok("open", state["doc"] is not None, "the document has the Text interface")
        if state["doc"] is not None:
            count = state["doc"].queryText().characterCount
            ok("open", count > 0, f"the document holds its text ({count} characters)")
        ann = [e[3] for e in events("open", "object:announcement")] + \
              [e[3] for e in events("launch", "object:announcement")]
        ok("open", any("Opened" in a for a in ann),
           "the Opened announcement arrived (ADR-0028: it can come before a client listens)", must=False)

    def check_read():
        moves = events("read", "object:text-caret-moved")
        ok("read", len(moves) >= 3, f"the caret followed the reading ({len(moves)} caret moves)")

    def check_pause():
        paused = [e for e in events("pause", "object:announcement") if "Paused" in e[3]]
        ok("pause", len(paused) >= 1, "the Paused announcement arrived")
        ok("pause", len(paused) <= 1, f"Paused was announced once ({len(paused)} times)", must=False)

    def check_heading():
        if state["doc"] is None:
            ok("heading", False, "no document to check the caret in")
            return
        off, line = line_at_caret(state["doc"])
        ok("heading", "Second heading" in line, f"the caret is on the next heading (offset {off}, line {line!r})")

    def check_list():
        focus = events("list", "object:state-changed:focused")
        lists = find_roles(state["window"], ("list", "list box", "dialog")) if state["window"] else []
        ok("list", bool(lists), f"the outline opened as a dialog or list ({len(lists)} found)")
        # The outline's items are the headings' text with their level
        # ("Reading check, level 1"), as the first run showed.
        items = [e for e in focus if re.search(r"Reading check|Second heading|Third heading|level \d", e[3])]
        ok("list", bool(focus), f"the focus moved into the outline ({len(focus)} focus events)")
        ok("list", bool(items), "a focused item names a heading and its level", must=False)

    def check_close():
        lists = find_roles(state["window"], ("dialog",)) if state["window"] else []
        ok("close", not lists, "Escape closed the outline")

    steps = [
        ("open", [], 4, check_open),
        ("read", ["ctrl+shift+space"], 6, check_read),
        ("pause", ["ctrl+shift+space"], 2.5, check_pause),
        ("heading", ["h"], 2.5, check_heading),
        ("list", ["alt+o"], 3, check_list),
        ("close", ["Escape"], 2.5, check_close),
    ]
    keys_sent = {}
    notes = []

    # Wait for the window (up to 30 seconds), then give it the keyboard.
    window = None
    for _ in range(60):
        _, window = dump.find_app()
        if window is not None:
            break
        time.sleep(0.5)
    if window is None:
        notes.append("Stopped: no textweaver window on the AT-SPI bus in 30 seconds. On the bus: "
                     + "; ".join(dump.bus_listing()))
    else:
        found = xdotool("search", "--sync", "--pid", str(gui.pid))
        wids = found.stdout.split()
        if wids:
            xdotool("windowfocus", "--sync", wids[-1])
            notes.append(f"X window {wids[-1]} given the keyboard focus with xdotool.")
        else:
            notes.append("Warning: xdotool found no X window for the GUI; keys go to whatever has the focus.")

    queue = list(steps) if window is not None else []

    def next_step():
        if not queue:
            pyatspi.Registry.stop()
            return False
        name, keys, wait, check = queue.pop(0)
        CURRENT[0] = name
        for key in keys:
            xdotool("key", "--clearmodifiers", key)
            time.sleep(0.3)
        keys_sent[name] = keys

        def finish():
            try:
                check()
            except Exception as e:  # a node went away mid-check
                results.append((name, "Fail", f"the check raised {e!r}"))
            end_step(name)
            GLib.idle_add(next_step)
            return False

        GLib.timeout_add(int(wait * 1000), finish)
        return False

    # A hard limit on the whole session, whatever happens in it.
    GLib.timeout_add(90_000, lambda: (notes.append("Stopped: the session passed 90 seconds."),
                                      pyatspi.Registry.stop(), False)[-1])
    GLib.idle_add(next_step)
    if queue:
        pyatspi.Registry.start()

    gui.terminate()
    try:
        gui.wait(timeout=10)
    except subprocess.TimeoutExpired:
        gui.kill()

    failed = sum(1 for r in results if r[1] == "Fail") + (0 if window is not None else 1)
    out = ["## AT-SPI reading session (Linux, Xvfb)", ""]
    for n in notes:
        out.append(f"- {n}")
    if notes:
        out.append("")
    for name, _, _, _ in steps:
        if name not in keys_sent and name != "open":
            continue
        out.append(f"### Step: {name}")
        out.append("")
        out.append(f"- Keys: {', '.join(keys_sent.get(name, [])) or 'none'}")
        for step, word, text in results:
            if step == name:
                out.append(f"- {word}: {text}")
        for e in EVENTS:
            if e[0] == name and e[2].startswith("object:announcement"):
                out.append(f"- Announced: {e[3]!r}")
        if args.orca_log:
            said = heard.get(name, [])
            out.append(f"- Orca said: {len(said)} phrases")
            for s in said:
                out.append(f"- Orca: {s!r}")
        out.append("")
    out.append("### Every event")
    out.append("")
    out.append("```")
    out.extend(f"{step:8} {t:7.2f} s  {kind:36} {detail}" for step, t, kind, detail in EVENTS)
    out.append("```")
    out.append("")
    out.append(f"Fail: {failed} must checks failed." if failed else "Pass: every must check passed.")
    text = "\n".join(out) + "\n"
    with open(args.out, "w", encoding="utf-8") as f:
        f.write(text)
    print(text)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
