# ADR-0031: Native RTF, ODT, and Word revisions

- Status: accepted
- Date: 2026-09-28

## Context

Students get course handouts as RTF and OpenDocument text (ODT) as often as Word files, and instructors return work with comments and tracked changes. Until now:

- RTF and ODT opened only through Pandoc, and only in `tw convert`. The reader, which never runs a subprocess to open a file, refused them, so a student had to convert first.
- The Word reader read the final text (insertions kept, deletions dropped), but comments were lost and there was no way to hear what changed.
- Word, OpenDocument, EPUB, and PowerPoint files are zip packages. The package reader capped one member at 256 MB, but not the number of members, overlapping members, compression ratios, or the total unpacked.

Research done for this work (kept outside the repository) found no crate worth adopting. rtf-parser 0.4.3 has no code pages, no tables, and a recursive lexer. The DOCX comment crates each bring a quick-xml stack beside our roxmltree reader.

## Decision

### RTF: our own iterative parser

`crates/textweaver-formats/src/rtf.rs`. One pass over the bytes with an explicit group stack; no recursion anywhere, so no input can overflow the stack.

- **Limits.** Groups nest at most 256 levels (`MAX_NESTING`, as for the XML walkers); deeper groups are read as plain text and the document carries the nesting warning. `\bin` data is skipped by moving the position, never read into memory, and never past the end, whatever length it claims. `\uc` is capped at 8. Control words are cut at 32 letters and parameters at 10 digits. Text never grows past the input's size.
- **Encoding.** `\ansicpg`, `\mac`, and each font's `\fcharset` or `\cpg` choose the code page for `\'hh` bytes and raw 8-bit text, decoded through encoding_rs. Bytes are held until the next control word, so double-byte pages (Shift_JIS, GBK, Big5, EUC-KR) decode correctly. `\u` characters, their `\uc` fallbacks, and surrogate pairs are handled. DOS code pages that encoding_rs lacks (437, 850) read as Windows-1252.
- **Destinations.** Unknown `\*` destinations are skipped, as are headers, footers, the color table, list tables, and picture data. The font table, style sheet, `\info` title and author, and `\revtbl` are read.
- **Structure.** Headings from style sheet names (`heading N`, `Title`) or `\outlinelevel`; lists from `\ls` and `\ilvl`, labeled from the `\listtext` or `\pntext` the writer rendered (so the label is what the author saw, without reimplementing Word's list numbering); tables from `\trowd`, `\cell`, `\row`, with `\trhdr` header rows; footnotes; `HYPERLINK` fields as links; bold, italic, underline; hidden text left out; picture alt text from `wzDescription`.

The parser collects paragraphs and rows first and builds the canonical text after, so paragraph properties that come before the text (`\pard\s1`) and after it (`\par`) meet in one place.

### ODT: roxmltree and zip, ODF 1.4

`crates/textweaver-formats/src/odt.rs`, for `.odt`, `.ott`, and flat `.fodt`. It follows the DOCX reader's walker: headings, nested lists with labels computed from the list style (formats `1 a A i I`, prefix, suffix, start value, display levels), tables with header rows, styles with inheritance, code blocks, links, frames' alt text, footnotes, and metadata. Repeated rows and cells with text are capped at 32 repeats; repeated empty ones are page padding and read as nothing.

### Comments

Comments are not text, so they do not go into the canonical text or its markers (and `textweaver-core`, which owns `MarkerKind`, is unchanged). A loader records them as `DocumentComment`s (id, range, author, date, text, replies, resolved) in the document's properties under `textweaver.comments`, as JSON, so they survive the document cache. Ranges come from a new builder facility, anchors: a range opened and closed like a marker but kept apart from the markers.

- **Word:** `w:commentRangeStart` and `w:commentRangeEnd`, or the `w:commentReference` point when there is no range; replies and resolved state from `w15:commentEx` (`paraIdParent`, `done`). A reply to a reply goes to the thread's first comment. Comments never anchored in the text are left out.
- **ODT:** `office:annotation` ranged to its `office:annotation-end`; LibreOffice's `loext:resolved` and `loext:parent-name`.

The reader turns each comment into a note of the store's existing `Note` type (no store change): id `comment-N`, the text "Comment by Ada Example: ... Reply by ...: ... Resolved.", tagged `comment` and `resolved`. A note with that id is never replaced, so an edited or kept comment note stays as the reader left it. Because they are ordinary notes, reading signals them with the note earcon and "Note: ..." on the status line, and they appear in the notes list and the study sheet.

### Tracked changes

`LoadOptions::revisions` (`RevisionMode`):

- `Final`, the default: the text as it reads with every change accepted, as before.
- `Marked`: each change said in place, "(inserted by Ada Example: new words)" and "(deleted by Ada Example: old words)", "moved here" and "moved away" for Word moves. The changed words are under an `Underline` marker for insertions and a `Strikethrough` marker for deletions, as word processors show them, with the phrase as the marker's label and the date as its reference.

Every loader counts changes under `textweaver.revisions` either way.

The reader's setting is `[reading] revisions = "auto" | "marked" | "final"`. `auto`, the default, is `Marked` at high verbosity and `Final` otherwise, so a student who wants every detail hears the changes, and one who wants the text does not. The choice is made when the document is loaded; a verbosity change applies the next time the document opens. The setting is read from the reading table's extra keys until the store type, export fixture, and schema entries land (a follow-up schema change; `[gui] announce` took the same path).

A `Revision` marker kind in core would let the reader skip or announce changes at speech time instead of load time. That is a core change, left for later.

### Package limits

For every zip package (DOCX, ODT, EPUB, PPTX, DAISY in a zip, ODS):

- at most 50,000 members;
- members may not overlap (compared from the central directory, before any member is read);
- a member over 1 MiB that claims more than 1,000 times its compressed size is refused before it is decompressed (deflate cannot pass about 1,030, bzip2 can);
- at most 1 GiB unpacked from one package in all, on top of the 256 MB per member;
- XML parts stop at 16 million nodes (roxmltree's `nodes_limit`); roxmltree expands no external entities.

### Fuzzing

Three cargo-fuzz targets: `rtf` (the RTF parser, final and marked), `odt` (a package built from the input, and the input as flat XML), and `docx_revisions` (document, comments, and extended comments from one input), each checking that markers and comment ranges stay inside the text. Their seeds are the fixtures in `fixtures/c2/`. The fuzz workspace was reserved for other work in progress when these targets were written, so they will be added once that work merges.

## Consequences

- RTF and ODT open in the reader and in `tw` with nothing installed; Pandoc is only for the long tail (reStructuredText, Org, LaTeX, and more). The Pandoc loader still lists `odt` and `rtf`, at its lower priority, so it is never chosen for them.
- `CANONICAL_VERSION` is 5, so cached documents are loaded again once.
- A document with comments adds notes to its saved state the first time it opens. Deleting such a note brings it back on the next open; a "dismissed comments" list would need a store field, so it is left for later.
- Comments in RTF (`\annotation`) and ODT change tracking of formatting are not read yet.
- Hostile packages that were read before (more than 50,000 members, extreme ratios) are now refused with a sentence that reads well aloud.

## Alternatives considered

- **Pandoc in the reader:** rejected again. The reader never runs a subprocess to open a file, and Pandoc is a large separate install.
- **rtf-parser 0.4.3:** too shallow (no code pages, no tables, recursive).
- **Comments as markers:** would need a new `MarkerKind` in core, and comments would be read as text by every consumer that walks markers. Properties keep them out of the text, and the reader decides what to do with them.
- **Tracked changes decided at speech time:** better for switching on the fly, but needs a core marker kind; load time works with today's types.
