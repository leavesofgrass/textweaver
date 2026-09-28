# ADR-0034: The rope after measurement: stay on ropey 1.6

- Status: accepted (Wave 5, Agent W5r)
- Date: 2026-09-28
- Builds on: [ADR-0002](0002-text-model.md) (the text model) and its status update of Monday, September 28, 2026 (Agent W4b's rope measurements)

## Context

ADR-0002 put the canonical text in a `ropey::Rope`, with positions as `CharPos`, a count of Unicode scalar values. Wave 4 asked whether a newer rope would be worth the move, and Agent W4b measured three on the bench corpora without changing anything:

- ropey 1.6.1, the one we use, with char offsets;
- ropey 2.0.0-beta.1, with `metric_chars` on, fed byte offsets and also char offsets converted on each edit;
- crop 0.4.3, byte offsets only.

The traces were built from what the reader's edit mode does: typing (20,000 keystrokes with backspaces, jumps, a line lookup after each, and a snapshot clone every 500), replace all (every " the ", 44,314 edits on 10 MB), and paste (2,000 pastes of 4 KB). Release build on Windows, the median of nine runs.

What the numbers say, on 10 MB:

- **Typing:** ropey 1 10.4 ms, ropey 2 6.6 ms (9.7 ms through char offsets), crop 3.7 ms.
- **Replace all:** 22.5, 18.0 (26.1), and 13.4 ms.
- **Paste:** 24.0, 16.8 (18.0), and 11.9 ms.
- **Building the rope:** 10.0, 4.9, and 3.4 ms.
- **Peak heap:** the same within 20 percent. crop made up to 2.5 times the allocations while typing.

So crop is 2 to 3 times as fast as ropey 1 on edits, and ropey 2 about 1.3 to 1.8 times. But 20,000 keystrokes on 10 MB take 10.4 ms with ropey 1: one edit costs about half a microsecond with every rope measured. Nobody can hear or feel that difference.

The same wave measured where the time does go. On 10 MB, `tw info` takes 580 to 700 ms, almost all of it loading, and the whole-document narration plan took 631 ms after W4b's work (1,155 ms before). The rope is not the bottleneck; loading and the narration plan are.

On crates.io today, ropey 2 is still `2.0.0-beta.1` (August 2, 2025), with no final release, and crop's newest is 0.4.3 (April 25, 2025).

## Decision

**Stay on ropey 1.6.** Nothing in the text model changes: the rope, `CharPos`, the state file format, and the line model all stay as ADR-0002 describes. Wave 5 spends the rope's slot on loading and the narration plan instead (Agent W5r, deliverable 2).

### Why not ropey 2 now

- **It is still a beta.** A beta rope under every open document, every edit, and every saved position is a risk with no user-visible gain.
- **Its edge mostly goes through char offsets.** ropey 2 indexes by bytes. Our positions are chars, persisted in every state file and meant never to change meaning. Fed char offsets converted on each edit, as our `CharPos` would need, typing went from 6.6 to 9.7 ms and replace all from 18.0 to 26.1 ms, which is slower than ropey 1 on replace all.
- **The move is wide.** ropey is used in six crates today (core, text, editor, formats, writers, and app), in about 44 source files, and `textweaver-core`'s `Edit::apply_to_rope` takes a `ropey::Rope` directly. Every loader builds one.

### Why not crop

- **No char metric at all.** crop counts bytes and lines only. We would keep our own char index beside it, for every position, highlight, bookmark, and search hit, and keep it in step on every edit. That index is the part ropey already does for us, and it would eat most of crop's lead.
- **Its line model is not ours.** crop counts only LF and CRLF as line breaks. ropey 1 counts every Unicode line break, U+2028 and U+2029 included, which is what the canonical text's line model and line navigation use. Moving would change what "next line" means in some documents, or need our own line index too.
- **More allocations while typing** (up to 2.5 times), for a speed nobody can notice.

### When to look again

Revisit this decision when any of these happens:

- **ropey 2.0 is released final.** Then measure it again with the same traces, through char offsets, and weigh the move against its breadth.
- **A trace a user can feel is slow** points at the rope: an edit, a line lookup, or a snapshot taking a noticeable part of a keystroke's time.
- **The positions change meaning.** If textweaver ever moves `CharPos` to bytes or UTF-16, which this ADR does not propose, the byte-indexed ropes become a closer fit.

The measuring probe's traces are described in ADR-0002's status update, so the next measurement can repeat them exactly.

### The fallback

This decision keeps what works, so its fallback is the measured alternative: if a real trace shows the rope is slow, crop is the fastest measured, at the cost of our own char and line indexes. ropey 2 final, if it arrives first, is the smaller step, since it keeps a char metric.

## Consequences

- No change to the state file format, `CharPos`, the markers, or any crate's API. No migration.
- ropey 1.6.1 (October 2023) stays pinned through `ropey = "1.6"` in the root `Cargo.toml`. It is mature, and its API has not changed since.
- The rope is not where textweaver's time goes. Loading a document and planning its narration are, and W5r's bench numbers before and after each change to them are recorded in [Testing](../dev/testing.md#benchmarks).
- The owner has nothing to decide or hear: "stay" needs no action.

## See also

- [ADR-0002: Text model](0002-text-model.md): the rope, `CharPos`, and W4b's measurements.
- [ADR-0005: Narration and the OffsetMap](0005-narration-and-offset-map.md): the narration plan, now the hot spot.
- [Testing](../dev/testing.md#benchmarks): the bench harness and its measurements.
- [Architecture](../dev/architecture.md): where the text model sits.
- [ADR index](README.md)
