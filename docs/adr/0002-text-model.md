# ADR-0002: Text model

- Status: accepted
- Date: 2026-09-25
- Status update (Saturday, September 26, 2026): Implemented as decided. `Document`, markers, units, navigation, history, search, and narration are in `textweaver-text`; loaders for text, Markdown, HTML, EPUB, DOCX, and PDF in `textweaver-formats` produce this canonical shape. Positions are persisted as `CharPos` in the per-document state files, and `tw migrate-star` maps Star's offsets by word alignment as described.
- Status update (Saturday, September 26, 2026, Phases 1 and 2): core gained `Strikethrough`, `Rule`, and `Math` markers (Agent P1d). Positions and bookmarks carry the text they were on, and each state records the text's length and hash, so marks are found again after the file changes outside textweaver (Agents P1b and P2a, `crates/textweaver-app/src/relocate.rs`). In edit mode, the Markdown source has its own markers at source positions, and positions move between the source and the canonical text by paired block markers instead of the word aligner (Agent P2b, `crates/textweaver-app/src/structure.rs`).
- Status update (Monday, September 28, 2026, Agent W4b): ropes measured for Wave 5's rope decision; nothing changed. ropey 1.6.1 (ours), ropey 2.0.0-beta.1 (`metric_chars` on), and crop 0.4.3 ran three synthetic edit traces, built from what the reader's edit mode does, on the bench's 1 MB and 10 MB Markdown corpora. Typing: 20,000 keystrokes at a caret, a backspace every tenth, a jump every thousand, the caret's line looked up after each, and a snapshot (a clone kept, as the writer thread gets) every 500. Replace all: every " the " replaced from the end (4,560 edits on 1 MB, 44,314 on 10 MB). Paste: 2,000 pastes of 4 KB at random places. ropey 1 took char offsets; ropey 2 and crop took byte offsets, and ropey 2 also took char offsets converted on each edit, which is what our `CharPos` positions would cost. Release build, Windows, median of nine runs with other agents building (fastest and slowest in the probe's log); every rope ended with the same length. 10 MB: building the rope 10.0, 4.9, and 3.4 ms (ropey 1, ropey 2, crop); typing 10.4, 6.6 (9.7 with char conversion), and 3.7 ms; replace all 22.5, 18.0 (26.1), and 13.4 ms; paste 24.0, 16.8 (18.0), and 11.9 ms. 1 MB: typing 10.6, 6.0 (9.5), and 3.4 ms; replace all 2.2, 1.7 (2.2), and 1.3 ms; paste 26.9, 18.2 (20.3), and 14.3 ms. Peak heap was the same for all three within 20 percent; crop made up to 2.5 times the allocations while typing (644 against 246 on 10 MB). So crop is 2 to 3 times as fast as ropey 1 on edits, and ropey 2 about 1.3 to 1.8 times, but through char offsets ropey 2's edge mostly goes; crop has no char metric at all (we would keep our own char index) and counts only LF and CRLF as line breaks, while ropey 1 counts every Unicode line break, as the canonical text's line model does (U+2028 included). One edit costs well under a microsecond with all three, so the rope is not today's bottleneck: loading and the narration plan are.
- Status update (Monday, September 28, 2026, Agent W5r): the rope decision is made in [ADR-0034](0034-rope-after-measurement.md): stay on ropey 1.6, and look again when ropey 2.0 is final or a trace a user can feel points at the rope. Nothing in this ADR changes.

## Context

Star keeps several parallel representations of a document (Markdown, a TTS `plain_text`, a word map, Qt's own document) and maps between them by offsets and alignment. Navigation units, highlighting, and saved positions drift when those representations disagree. Paperback (Rust) shows a simpler shape: one text buffer plus markers with index tables.

## Decision

A `Document` is **canonical text in a `ropey::Rope` plus sorted `Marker`s**:

```rust
Document { meta: DocumentMeta, text: Rope, markers: Vec<Marker>, display: OnceCell<DisplayIndex> }
Marker   { kind: MarkerKind, range: CharRange, level: u8, label: Option<String>, reference: Option<String> }
```

- **Positions are `CharPos`**, a count of Unicode scalar values from the start of the canonical text. They are what gets persisted. Byte offsets come from the rope; UTF-16 offsets for GUI toolkits come from a lazily built `DisplayIndex`. Neither is ever persisted.
- **Canonical text shape.** Line endings are `\n`. Paragraphs are separated by one blank line. Headings and list items are bare lines (no `#`, no bullets). Tables are one row per line under a `Table` marker, with `TableRow` and `TableCell` markers. Images are their alt text under an `Image` marker. Code is its text under a `Code` marker. Plain-text files keep their lines.
- **Markers** carry structure: `Heading` (level 1–6), `Paragraph`, `ListItem` (level = depth), `List`, `Table`, `TableRow`, `TableCell`, `Link` (reference = target), `Image`, `Code`, `Quote`, `PageBreak`, `SectionBreak`, `Bold`, `Italic`, `Underline`, `Footnote`. `MarkerKind` lives in core because `Unit::Marker { kind, level }` names navigation targets.
- **Edits** go through `Document::apply(&Edit)`, which applies the edit to the rope, shifts every marker with `EditOutcome::map_range`, and drops the display index. Ranges do not grow at their edges; a fully deleted range collapses to an empty range at the edit point. Bookmarks, notes, history, and the cursor shift with the same `EditOutcome`.
- **Everything else is a pure function** of `&Document` and a position: `units::unit_at`, `navigate`, `go_to`, `History`, `search::find`, `narrate::plan`. This keeps them testable with no UI and no speech.
- **Large documents** are laid out through a window slice (Paperback's 500k-unit windows) from day one; positions stay document-absolute.

### Deliberate difference from Star's canonical text

Star's `plain_text` joins every single newline into a space, plain text included, and runs list items and table cells together into one paragraph (`fixtures/star-parity/*.json`). textweaver keeps line structure, because Speech Cursor line mode, line navigation, and "blank" on empty lines need it, and because list and table navigation need item and row boundaries.

Consequence: Star's saved character offsets do not map one-to-one onto textweaver positions. `tw migrate-star` (wave 2) maps each saved Star offset to Star's word index (first word at or after the offset, Star's own restore rule), then to the textweaver word with the same index and text in the aligned word sequences, falling back to the saved percentage.

## Consequences

- One source of truth for text; highlight ranges, search hits, and bookmarks are all `CharRange`s into it.
- Rope operations make edits and line lookups `O(log n)`; the Phase 0 unit functions materialize strings and are replaced by Agent A with rope-based segment iterators.
- Parity with Star is measured, not assumed: the parity report compares word and sentence boundaries against Star's export and documents every difference.

## See also

- [Reading and moving around](../reading.md): the units and navigation this model supports.
- [The library](../library.md): where positions are saved, and `tw migrate-star`.
- [ADR-0005: Narration and the OffsetMap](0005-narration-and-offset-map.md): how this text is spoken.
- [Architecture](../dev/architecture.md): the crate map, the threads, and the path from a file to a spoken, highlighted word.
- [Documentation index](../README.md)
