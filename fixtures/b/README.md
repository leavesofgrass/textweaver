# Fixtures for braille (BRF) formatting

Used by `crates/textweaver-writers/tests/it/brf_formats.rs`, which writes each
Markdown file as BRF and compares it with the reviewed snapshot beside it.
Set `TW_BLESS=1` to write the snapshots again, then read the difference
before committing.

- `table.md`: a table with a header row, a blank entry, and a long entry.
  Snapshots: `table.linear.brf`, `table.listed.brf` and
  `table.stairstep.brf`, one for each `[braille] table_format`. The listed
  and stairstep layouts follow BANA's Braille Formats: Principles of
  Print-to-Braille Transcription (2016), 11.16 and 11.18; the linear
  layout follows 11.17, and every table has a blank line before and after
  it (11.2.5d).
- `typeforms.md`: an italic title of four words (a passage), one bold and
  one italic word, a highlight (written as underline), closing punctuation
  after an italic word, and bold inside a word. Snapshot: `typeforms.brf`.
  The indicators follow The Rules of Unified English Braille (2013),
  section 9.
- `capitals.md`: a heading in capitals (one capitals passage indicator and
  one terminator), a passage inside a sentence, and two capitalized words
  that keep their word indicators. Snapshot: `capitals.brf`. The
  indicators follow the Rules' section 8.
- `passages.md`: three paragraphs in capitals, a heading in capitals, two
  paragraphs in italics, bold list items, and a quotation in italics.
  Snapshot: `passages.brf`. A passage that goes on over paragraphs is
  opened at each and terminated once (the Rules' 8.5.5 and 9.9.1, numbered
  the same in the third edition, 2024); each heading is capitalized by
  itself (8.5.6); the bullet between bold list items ends each item's
  passage, since the typeform changes there.

The snapshots are 40 cells by 25 lines, with braille page numbers.
