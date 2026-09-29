# Fixtures for rendering and conversion

- `commonmark-spec.json`: the 652 examples of the CommonMark specification,
  version 0.31.2, as `{example, markdown, html}` objects. Extracted from the
  spec tests pulldown-cmark 0.13.4 generates from the specification
  (`tests/suite/spec.rs`). The CommonMark specification is by John MacFarlane
  and is licensed under the Creative Commons Attribution-ShareAlike 4.0
  license (CC BY-SA 4.0), <https://spec.commonmark.org/0.31.2/>.
- `flavors/`: small Markdown documents exercising each flavor (GFM,
  Obsidian, Pandoc) and math; their rendered HTML is snapshot-tested in
  `crates/textweaver-render/tests/flavors.rs`.
- `vault/`: a tiny Obsidian vault for embed resolution tests.
