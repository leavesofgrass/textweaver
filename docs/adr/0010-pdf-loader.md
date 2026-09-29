# ADR-0010: PDF loader

- Status: accepted
- Date: 2026-09-25
- Status update (Saturday, September 26, 2026): Implemented as decided and on by default. The `PageBreak` markers exist, but the reader has no page navigation yet: go to does not take a page number. `pdf-extract` and `pdfium-render` remain in the workspace table although no crate uses them; removing them is still open. OCR is still out of scope.
- Status update (Saturday, September 26, 2026, Phase 1): `pdf-extract` and `pdfium-render` are gone from the workspace table, and hostile page labels and list counters are clamped. OCR of pages with no text layer is planned for later: the pure-Rust `ocrs` engine in process first, with a Tesseract subprocess as a fallback.
- Status update (Saturday, September 26, 2026): Pages with no text layer are now recognized by OCR (see [ADR-0026](0026-ocr-and-student-formats.md)): ocrs in process for English, Tesseract for other languages, and the words laid out by this loader's layout engine.

## Context

Most course material is read as PDF, and the goal is to read PDFs as Markdown with text-to-speech. So PDF support must be on by default in `tw` and `textweaver` on Windows, macOS, and Linux, and the loader must recover Markdown-quality structure (headings, paragraphs, lists, tables, reading order in columns, no running headers or page numbers, page navigation), not just a stream of text. Star's column-aware reconstruction (`star/documents/pdf.py`, on pdfminer.six's layout boxes; the Star parity reference, kept outside the repository) is the quality bar.

The workspace table offered three crates: `lopdf` (a PDF object model and content-stream parser), `pdf-extract` (text extraction on lopdf), and `pdfium-render` (bindings to Google's PDFium, loaded as a shared library at run time).

## Measurements

Fixtures (all generated here, none copyrighted): `fixtures/a/single.pdf` (one column; title, headings by size, weight, and number; a hyphenated line end; bulleted and numbered lists; a table; bookmarks), `columns.pdf` (two columns with a full-width caption band between two column sections), `running.pdf` (three pages with a running header, page-number footers, and a paragraph continuing across a page), all from `make_pdfs.py` with the standard 14 fonts; and `browser.pdf`, `browser.html` printed by Microsoft Edge (embedded subset TrueType fonts with ToUnicode maps, a tagged PDF, justified two-column text, bullets drawn as shapes, a list continuing into the second column, a figure with alternative text, a table). A 300-page PDF from `make_pdfs.py big` (1.26 MB) measured speed. Times are release builds on the development machine (Windows 11), fastest of several runs; the machine was shared with other builds, so a second, loaded run is given too.

| | textweaver on lopdf (chosen) | pdf-extract 0.12.1 | pdfium-render 0.9 |
|---|---|---|---|
| Native dependency | none (pure Rust) | none (pure Rust) | PDFium shared library (`pdfium.dll`, `libpdfium.so`, `libpdfium.dylib`), not on this machine, not in any OS; must be downloaded and shipped per platform |
| Reading order, `columns.pdf` | left column, right column, band, left, right (correct) | content-stream order (correct only because the fixture draws in reading order) | not measured |
| Reading order, `browser.pdf` | correct: columns, band dividers, list continued across columns | content order; `P DF`, `F igure`, `R eading` split by kerning; justified gaps doubled (`some  of  the`) | not measured |
| Structure | headings (tags, size, weight, numbering, outline), paragraphs joined and de-hyphenated, bulleted and numbered lists (text markers or tagged `LI`), tables, code, image alt text, `PageBreak` and `SectionBreak` markers | plain text only; no font identity (so no weight), no images, no tags | glyph boxes and fonts; structure would still be ours to build |
| Running heads and page numbers | removed (Star's rule; `Artifact` content skipped in tagged PDFs) | kept | not measured |
| Robustness | no panics by construction (no `unwrap` on file data); a page that fails to parse is skipped; operation and recursion budgets | about 100 `unwrap`/`expect`/`panic!` on file data (for example `Tj` with a non-string operand, missing `MediaBox`); image XObjects are fed to the content parser; form XObjects ignore the current CTM and `/Matrix`; `'` and `"` operators ignored | depends on PDFium (robust, C++) |
| 300 pages, unloaded machine | 0.20–0.31 s | 0.67 s | not measured |
| 300 pages, loaded machine | 0.14–0.28 s | 2.3–2.9 s | not measured |
| Small fixtures (fastest of five, from memory) | 0.6–1.8 ms; 8.9 ms for `browser.pdf` (embedded fonts) | 1.4–9 ms | not measured |
| Peak memory, `tw info` on 300 pages | about 20 MB working set (whole process) | not measured | not measured |
| Dependencies | lopdf 0.45 (already in the table) | pins lopdf 0.42 (a second lopdf), `euclid`, `adobe-cmap-parser`, `cff-parser`, `type1-encoding-parser`, `postscript` | `pdfium-render`, plus the library |

pdfium-render could not be measured: no PDFium library exists on this machine, and downloading one needs approval first. Its text and glyph boxes are the best of the three, but it cannot be a default that "just works": the library must be found or shipped for every platform.

## Decision

**The default PDF loader is textweaver's own layout engine on `lopdf` 0.45**, behind the `pdf` feature of `textweaver-formats`, **on by default** (so `tw text file.pdf` and `textweaver file.pdf` work everywhere with no setup). `pdf-extract` and `pdfium-render` are not used; they can be removed from the workspace table. A PDFium loader can return later as an optional second loader (lower priority, off by default) if a file shows the need; nothing in the design prevents it.

The loader (`crates/textweaver-formats/src/pdf/`):

1. **Glyphs** (`interp.rs`): interprets page content streams: graphics and text state, `cm`/`q`/`Q`, `Tf`/`Tc`/`Tw`/`Tz`/`TL`/`Ts`, `Td`/`TD`/`Tm`/`T*`, `Tj`/`TJ`/`'`/`"`, form XObjects with their matrix and resources (depth-limited), image XObjects (placement), marked content (`Artifact` skipped, `/ActualText` replaces the glyphs it covers, `/Alt` and `/MCID` recorded), `/Rotate` and the crop box. Budgets: 5 million operators per page, form depth 12.
2. **Fonts** (`fonts.rs`): Unicode from `/ToUnicode` first, then `/Encoding` (base encodings and `/Differences` via lopdf's glyph-name table), then StandardEncoding; widths from `/Widths`, `/W`/`/DW` for composite fonts, or the standard 14 font metrics (`metrics.rs`, from Adobe's Core14 AFM files) when a font omits them; bold, italic, and monospace from the font name, `/FontWeight`, `/Flags`, and `/ItalicAngle`.
3. **Layout** (`layout.rs`): glyphs into lines (split at gaps wider than an em); **columns** from a count-based vertical projection: a gutter is an interior strip at least 1.5% of the page wide that lines narrower than 55% of the page (almost) never cross, confirmed by three full lines of text on each side; **tables** found within a column as two or more aligned rows of short cells (rejected when the cells read like prose or when the rows around them are columns of text); justified lines rejoined; lines stacked into blocks by spacing, overlap, size, and weight; Star's running-head removal (margin text recurring on half the pages, bare page numbers); Star's reading order (bands divided by spanning blocks, columns left to right, each top to bottom).
4. **Structure** (`structure.rs`): paragraphs (wrapped lines joined; a line-end hyphen before a lowercase letter removed; a soft hyphen always; a paragraph broken by a column or page end continued), headings (tagged `H1`–`H6`; short lines 15% larger than the body text, levels by size; a short single bold line when the body is not bold; numbered headings, deeper numbers one level down; outline entries), lists (bullet glyphs including Word's Private Use Area bullets, `1.`/`a)`/`(iv)` markers, tagged `LI` items whose bullets are drawn as shapes; nesting by indent from the column's text margin), monospaced blocks as code, tables, images with alternate text (marked content or the structure tree's `Figure` elements), wholly italic or bold paragraphs kept as emphasis.
5. **Markers**: `PageBreak` per page (label = the printed page label from `/PageLabels`, such as `iv` or `A-3`, else the page number; the range is that page's text, so a page can start mid-paragraph and "go to page" lands on the first word printed on it), `SectionBreak` per outline entry (label = title, level = depth), and the usual heading, paragraph, list, table, code, and image markers. Metadata from `/Info` (title, author, subject) and the catalog's `/Lang`; a `pages` property.

A password-protected PDF (not openable with the empty password) and an unparseable file fail with a clear message; a PDF with no text layer (a scan) loads as one sentence saying so (OCR is out of scope).

### Refinements over Star, deliberately

- Gutters narrower than Star's 4% of the page (a browser's 0.3-inch gutter is 3.5%), and blocks crossing a gutter are band dividers even when narrower than 55% of the page: on the browser fixture Star's rule saw one column because the title crossed the gutter.
- Columns are found before tables, so a table in one column never takes lines from the other column.
- Star italicized caption lines by pattern; here emphasis comes from the fonts.

## Consequences

- PDFs load by default with no native library, in a few milliseconds for articles and a fraction of a second for a 300-page book.
- `tw text file.pdf --format markdown` prints Markdown from the recovered structure (tests check that it reads back with the same headings, list items, table rows, and paragraphs).
- Layout heuristics can misjudge unusual pages (three-column magazines with irregular gutters, tables without aligned columns, lists whose bullets are images and whose PDF is untagged). Tagged PDFs are the most reliable, since headings and list items come from the tags.
- Not yet done: vertical and right-to-left scripts, OCR, form fields and annotations (link targets are not recovered), and CJK predefined CMaps other than Identity. lopdf's default features (`chrono`, `jiff`, `time`, `rayon`) are not needed by the loader; `default-features = false` can be set on `lopdf` in the workspace table.

## See also

- [Converting documents](../converting.md#formats-textweaver-reads): the formats textweaver reads.
- [Reading and moving around](../reading.md): reading a PDF.
- [Documentation index](../README.md)
