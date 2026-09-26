# ADR-0002: Text model

- Status: accepted
- Date: 2026-09-25
- Status update (Saturday, September 26, 2026): Implemented as decided. `Document`, markers, units, navigation, history, search, and narration are in `textweaver-text`; loaders for text, Markdown, HTML, EPUB, DOCX, and PDF in `textweaver-formats` produce this canonical shape. Positions are persisted as `CharPos` in the per-document state files, and `tw migrate-star` maps Star's offsets by word alignment as described.

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
- [Architecture](../architecture.md): the crate map, the threads, and the path from a file to a spoken, highlighted word.
- [Documentation index](../README.md)
