# ADR-0044: Obsidian, JSON, SVG and content MathML in the reader

- Status: accepted
- Date: 2026-09-29 (Tuesday, September 29, 2026)

## Context

Students and writers keep notes in Obsidian vaults, get data and assignments as JSON and Jupyter notebooks, meet charts as SVG drawings, and find formulas written in content MathML. Before this decision:

- The HTML renderer (`textweaver-render`, the `obsidian` flavor) already knew Obsidian's callouts, embeds, tags, and highlights, but only for output. The reading loader knew GitHub's five alerts, so a reader heard `[!tip]` as text in an Obsidian note, heard `![[note]]` as a file name, and heard `%%comments%%` aloud.
- JSON, JSON Lines, and SVG files opened as plain text, with every brace and quote read. Notebooks opened only through Pandoc, and only in `tw convert`.
- The HTML loader dropped every inline `<svg>`, even a chart with a title.
- MathML meant presentation MathML (ADR-0029, ADR-0035). Content MathML (`<apply><plus/>...`) read as a run of letters, and no loader claimed `.mml` files.
- The LaTeX loader (ADR-0035) did not expand macros with arguments or read `\newenvironment`, `.bib` files, or table spans.

There is no Pandoc fallback in the reader, so every gap here was a hard gap: a format the native loaders do not read is not read well at all.

## Decision

### Callouts: one set of rules, in the formats crate

The callout head rules (Obsidian's thirteen types and their aliases, custom titles, the `+` and `-` fold marks, and GitHub's five alerts) move from `textweaver-render`'s preprocessor into `textweaver_formats::callout`. The renderer calls them, so the reader and the HTML writer agree on what a callout is, its type, its title, and its fold state. This adds a dependency edge: `textweaver-render` depends on `textweaver-formats`, without its default features (no PDF, OCR, archive, or workbook loaders). The render crate is already part of the conversion stack the lean reader leaves out, so `cargo xtask deps --check` is unchanged. A lighter home, `textweaver-text`, was considered; the formats crate was chosen because the rules are about reading a source format, and the loaders are where format rules live.

The Markdown loader reads a callout as a block quote labeled with its type as written, and says the type first, in words: "Warning: Hot surface" on a line of its own before the body, or "Note:" before the body when the callout has no title. A foldable callout says its state once, in that label ("Tip, collapsed:"). The body is always read: folding is a visual convenience, not a reason to hide text from a listener.

### Obsidian notes

`textweaver_formats::obsidian` holds the rest:

- **Embeds** (`![[note]]`, `![[note#Heading]]`, `![[note#^id]]`) are read in place, between "Embedded from Note" and "End of embed", under a block quote labeled `embed`, so the start and end are each said once and the embed can be skipped as a unit. Notes are found by path, then by file name, only in the embedding note's own folder or below it: never through `..`, an absolute path, or a link, since the canonical path must stay inside the folder. Two levels deep at most; a cycle is refused and says so in words; each note is at most 4 megabytes, 16 in all, and 64 embeds per document. A note that cannot be read is a link, with the reason in words ("(embedded note not found)").
- **Picture embeds** (`![[diagram.png|300]]`) are graphics named by their file; the size is not read.
- **Tags** read as words: `#physics/waves` is "tag physics slash waves". Only a `#` after white space starts a tag, and a tag of digits alone (`#12`) is a number.
- **Highlights** (`==text==`) are under an `Underline` marker labeled `highlight`, opened only when a closing `==` follows in the same block. A dedicated `Highlight` marker kind in `textweaver-core` is the better model; it touches the core contract, so it is requested separately rather than made here.
- **Comments** (`%%...%%`) are not read, inline or across blocks.
- **Block ids** (`^id`) are not read. The document keeps them in its properties (`textweaver.block_ids`), and `obsidian::block_position` finds the block a `note#^id` link names; the app's link following needs a one-line call to use it.

### JSON, JSON Lines, and notebooks

`json.rs` has its own small JSON parser rather than `serde_json`: keys must keep the file's order for reading (`serde_json` sorts them unless its `preserve_order` feature changes the map type for the whole workspace), and a reader needs bounded work on hostile files (64 megabytes, nesting to 256, two million values).

A JSON file reads as a document: every top-level key is a heading, so `h` moves by key ("name: Ada Example", or "address, object, 3 keys" before its members). Deeper objects and arrays are headings one level down to level six ("item 1 of 2, object, 3 keys"), and plain values are list items. No punctuation is read. Invalid JSON is read as plain text, and the warning says where it broke, since a listener opening a broken file still wants its words. JSON Lines give a heading per line.

A notebook reads cell by cell: markdown cells through the Markdown loader (written into the notebook's builder, footnotes in place), code cells as code under a line naming the language ("Python code", with the language as the marker's label), text outputs as quotes after "Output", errors by name and message without the colored traceback, pictures as graphics ("Output picture, PNG"), and raw cells as text.

JSON and SVG files open by name, but a folder scan leaves them out: a folder of configuration files or pictures is not a folder of documents.

### SVG, as SVG-AAM exposes it

`svg.rs` reads a drawing as a screen reader meets one: the name (`aria-label`, else `<title>`) first, a level-1 heading for a file, then `<desc>`, then each titled part (a group, or a chart's bar with a title) as a list item in document order, then the `<text>` it shows, one line each. `role="img"` makes the drawing one graphic. Hidden parts (`defs`, `aria-hidden`, `display="none"`) are not read. A file with nothing to read says "Drawing with no description". Files are parsed with roxmltree, which is already a dependency (`usvg` builds a render tree and does not promise to keep titles and descriptions); a DOCTYPE is allowed, external entities are never read, and nesting and element counts are bounded.

An inline `<svg>` in a web page gets the same reading from the HTML parser's tree, converted to the same small tree first so both read alike, with one difference: an inline drawing with nothing to read stays silent, as a decorative picture does, because web pages are full of icon drawings.

### Content MathML

`mathml.rs` converts content MathML to the same LaTeX presentation MathML becomes, so speech (textweaver's own or MathCAT) and braille read both alike: arithmetic with parentheses where the structure needs them (a precedence for each operator), powers, roots, fractions, relations chained, logic, sets and intervals, functions, sums, products, integrals, limits, derivatives, selectors, matrices, and piecewise functions. An operator it does not know is read by name, as a function. An XSLT engine was rejected: a large dependency for a rare input. A MathML file opens as one display formula.

### LaTeX past ADR-0035

Macros with up to nine arguments (`#1` to `#9`, the first optional when it has a default) expand in text and in math, under the same token, step, and expansion budgets as before, so a self-doubling macro stops with the cut warning. `\def` with delimited parameters is still named in the warnings. `\newenvironment` runs its begin code with its arguments, and its end code. `\multicolumn` and `\multirow` cells say what they span after their text ("Wide (spans 2 columns)"). `\includegraphics[alt={...}]` is described by its alt text when its figure has no caption.

`\bibliography{refs}`, and `\addbibresource` with `\printbibliography`, read the `.bib` files from the document's folder (the same path checks as `\input`) through `textweaver-cite`, and list the cited entries under "References" in the style `\bibliographystyle` names. This is an optional dependency of `textweaver-formats` on `textweaver-cite`, the `bibliography` feature: off by default, so the lean reader (no `publish` feature) stays without the citation stack, as `cargo xtask deps` requires; `textweaver-convert` turns it on. Without it, the document's warnings say the bibliography is not read.

### Pandoc's input

Pandoc reads text only as UTF-8. Text sent to it in another encoding (Windows-1252, UTF-16, XML declaring Latin-1) is now converted to UTF-8 first, as the native loaders decode. Its output was already read as UTF-8 bytes, never with the Windows code page, which was the bug star shipped.

## Consequences

- The canonical text version is 7: cached documents are read again.
- New open-failure messages, in six languages: `opening-damaged-json`, `opening-damaged-notebook`, `opening-damaged-svg`, `opening-damaged-mathml`.
- New fuzz targets `json`, `svg` (with `.mml` and inline drawings), and `obsidian`, seeded from `fixtures/o`; the unclosed `<meta charset>` page that emptied whole documents in star is a seed of the `html` target.
- A highlight is an underline to the writers until the core has a `Highlight` marker kind.
- The fallback for any of these readers is plain text: every loader here is new code, and a document it cannot read opens as text with a warning (JSON) or fails with a plain message (notebooks, drawings, formulas).

## See also

- [ADR-0029: MathCAT speech](0029-mathcat-speech.md)
- [ADR-0035: Native LaTeX subset, and email and web archives](0035-latex-email-and-web-archives.md)
- [Converting documents](../converting.md)
